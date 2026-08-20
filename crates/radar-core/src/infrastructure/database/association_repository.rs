//! Transaction-scoped persistence for deterministic association groups.

use std::collections::HashSet;

use rusqlite::{Connection, params};

use crate::domain::intel::{
    AssociationEvidenceBasis, AssociationRelationType, association_for_normalized_url,
    association_id_from_basis_hash,
};

pub(crate) fn reconcile_url_membership(
    connection: &Connection,
    item_id: i64,
    normalized_url: &str,
    first_observed_at: &str,
    last_observed_at: &str,
) -> rusqlite::Result<()> {
    let evidence =
        association_for_normalized_url(normalized_url).ok_or(rusqlite::Error::InvalidQuery)?;
    let mut affected_association_ids = connection
        .prepare(
            "SELECT association_id FROM intel_association_members WHERE item_id=?1 ORDER BY association_id",
        )?
        .query_map([item_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    affected_association_ids.push(evidence.association_id.as_str().to_owned());
    affected_association_ids.sort_unstable();
    affected_association_ids.dedup();
    connection.execute(
        "DELETE FROM intel_association_members WHERE item_id=?1 AND association_id<>?2",
        params![item_id, evidence.association_id.as_str()],
    )?;
    connection.execute(
        "INSERT INTO intel_associations
         (association_id,relation_type,evidence_basis,basis_version,basis_hash,first_observed_at,last_observed_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(association_id) DO UPDATE SET
           first_observed_at=MIN(first_observed_at,excluded.first_observed_at),
           last_observed_at=MAX(last_observed_at,excluded.last_observed_at)",
        params![
            evidence.association_id.as_str(),
            evidence.relation.as_str(),
            evidence.basis.as_str(),
            evidence.basis_version,
            evidence.basis_hash,
            first_observed_at,
            last_observed_at,
        ],
    )?;
    connection.execute(
        "INSERT INTO intel_association_members
         (association_id,item_id,first_observed_at,last_observed_at)
         VALUES(?1,?2,?3,?4)
         ON CONFLICT(association_id,item_id) DO UPDATE SET
           first_observed_at=MIN(first_observed_at,excluded.first_observed_at),
           last_observed_at=MAX(last_observed_at,excluded.last_observed_at)",
        params![
            evidence.association_id.as_str(),
            item_id,
            first_observed_at,
            last_observed_at,
        ],
    )?;
    refresh_envelopes(connection, &affected_association_ids)?;
    Ok(())
}

pub(crate) fn backfill_associations(connection: &Connection) -> rusqlite::Result<()> {
    let rows = connection
        .prepare(
            "SELECT p.item_id,p.original_url,p.first_discovered_at,p.last_updated_at
             FROM item_provenance p JOIN intel_items i ON i.id=p.item_id
             WHERE i.data_origin='real' AND i.source_kind='rss_atom'
               AND p.author_availability<>'unknown_legacy'
               AND p.availability_status<>'unknown_legacy'
             ORDER BY p.item_id",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (item_id, url, first, last) in rows {
        reconcile_url_membership(connection, item_id, &url, &first, &last)?;
    }
    Ok(())
}

pub(crate) fn verify_associations(connection: &Connection) -> rusqlite::Result<()> {
    verify_groups(connection)?;
    verify_envelopes(connection)?;
    let member_item_ids = verify_members(connection)?;
    verify_eligible_memberships(connection, &member_item_ids)
}

fn verify_groups(connection: &Connection) -> rusqlite::Result<()> {
    let rows = connection
        .prepare(
            "SELECT association_id,relation_type,evidence_basis,basis_version,basis_hash,
                    first_observed_at,last_observed_at
             FROM intel_associations ORDER BY association_id",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, u16>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, relation, basis, version, hash, first, last) in rows {
        let expected = association_id_from_basis_hash(
            AssociationRelationType::SameEvent,
            AssociationEvidenceBasis::NormalizedOriginalUrl,
            version,
            &hash,
        )
        .ok_or(rusqlite::Error::InvalidQuery)?;
        if relation != "same_event"
            || basis != "normalized_original_url"
            || id != expected.as_str()
            || first > last
            || crate::contracts::effects::normalize_rfc3339_utc(&first).as_deref()
                != Some(first.as_str())
            || crate::contracts::effects::normalize_rfc3339_utc(&last).as_deref()
                != Some(last.as_str())
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    Ok(())
}

fn verify_envelopes(connection: &Connection) -> rusqlite::Result<()> {
    let invalid_envelopes: u32 = connection.query_row(
        "SELECT COUNT(*) FROM intel_associations a
         WHERE NOT EXISTS (SELECT 1 FROM intel_association_members m WHERE m.association_id=a.association_id)
            OR a.first_observed_at<>(SELECT MIN(m.first_observed_at) FROM intel_association_members m WHERE m.association_id=a.association_id)
            OR a.last_observed_at<>(SELECT MAX(m.last_observed_at) FROM intel_association_members m WHERE m.association_id=a.association_id)",
        [],
        |row| row.get(0),
    )?;
    if invalid_envelopes != 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

struct AssociationMemberRow {
    association_id: String,
    item_id: i64,
    member_first: String,
    member_last: String,
    original_url: String,
    provenance_first: String,
    provenance_last: String,
    data_origin: String,
    source_kind: String,
    author_availability: String,
    availability_status: String,
}

fn verify_members(connection: &Connection) -> rusqlite::Result<HashSet<i64>> {
    let members = connection
        .prepare(
            "SELECT m.association_id,m.item_id,m.first_observed_at,m.last_observed_at,
                    p.original_url,p.first_discovered_at,p.last_updated_at,
                    i.data_origin,i.source_kind,p.author_availability,p.availability_status
             FROM intel_association_members m
             JOIN intel_items i ON i.id=m.item_id
             JOIN item_provenance p ON p.item_id=m.item_id
             ORDER BY m.item_id",
        )?
        .query_map([], |row| {
            Ok(AssociationMemberRow {
                association_id: row.get(0)?,
                item_id: row.get(1)?,
                member_first: row.get(2)?,
                member_last: row.get(3)?,
                original_url: row.get(4)?,
                provenance_first: row.get(5)?,
                provenance_last: row.get(6)?,
                data_origin: row.get(7)?,
                source_kind: row.get(8)?,
                author_availability: row.get(9)?,
                availability_status: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut member_item_ids = HashSet::with_capacity(members.len());
    for member in members {
        let evidence = association_for_normalized_url(&member.original_url)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        if member.data_origin != "real"
            || member.source_kind != "rss_atom"
            || member.author_availability == "unknown_legacy"
            || member.availability_status == "unknown_legacy"
            || member.association_id != evidence.association_id.as_str()
            || member.member_first != member.provenance_first
            || member.member_last < member.provenance_last
            || crate::contracts::effects::normalize_rfc3339_utc(&member.member_first).as_deref()
                != Some(member.member_first.as_str())
            || crate::contracts::effects::normalize_rfc3339_utc(&member.member_last).as_deref()
                != Some(member.member_last.as_str())
            || !member_item_ids.insert(member.item_id)
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    Ok(member_item_ids)
}

fn verify_eligible_memberships(
    connection: &Connection,
    member_item_ids: &HashSet<i64>,
) -> rusqlite::Result<()> {
    let eligible_items = connection
        .prepare(
            "SELECT p.item_id,p.original_url
             FROM item_provenance p JOIN intel_items i ON i.id=p.item_id
             WHERE i.data_origin='real' AND i.source_kind='rss_atom'
               AND p.author_availability<>'unknown_legacy'
               AND p.availability_status<>'unknown_legacy'
             ORDER BY p.item_id",
        )?
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (item_id, original_url) in eligible_items {
        if association_for_normalized_url(&original_url).is_none()
            || !member_item_ids.contains(&item_id)
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    Ok(())
}

fn refresh_envelopes(connection: &Connection, association_ids: &[String]) -> rusqlite::Result<()> {
    for association_id in association_ids {
        connection.execute(
            "UPDATE intel_associations SET
               first_observed_at=(SELECT MIN(m.first_observed_at) FROM intel_association_members m WHERE m.association_id=?1),
               last_observed_at=(SELECT MAX(m.last_observed_at) FROM intel_association_members m WHERE m.association_id=?1)
             WHERE association_id=?1
               AND EXISTS (SELECT 1 FROM intel_association_members m WHERE m.association_id=?1)",
            [association_id],
        )?;
        connection.execute(
            "DELETE FROM intel_associations WHERE association_id=?1
             AND NOT EXISTS (SELECT 1 FROM intel_association_members m WHERE m.association_id=?1)",
            [association_id],
        )?;
    }
    Ok(())
}
