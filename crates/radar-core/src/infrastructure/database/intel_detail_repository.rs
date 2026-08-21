//! Bounded, read-only `SQLite` projection for real RSS evidence details.

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::contracts::dto::intel_detail::{
    AssociationEvidenceStatusV1, AssociationEvidenceV1, INTEL_DETAIL_MAX_PROVENANCE,
    IntelProvenanceV1, ProvenanceRoleV1, SourceFactsV1,
};

pub(crate) struct DetailFactsRow {
    pub item_id: i64,
    pub facts: SourceFactsV1,
    pub primary: IntelProvenanceV1,
}

pub(crate) struct RuleRow {
    pub rule_version: Option<String>,
    pub configuration_revision: Option<u64>,
    pub configuration_hash: Option<String>,
    pub fact_revision: Option<u64>,
    pub evaluated_at_ms: Option<u64>,
    pub score: Option<u8>,
    pub importance: String,
    pub disposition: Option<String>,
    pub matched_tracks_json: Option<String>,
    pub factors_json: Option<String>,
    pub filter_reasons_json: Option<String>,
    pub ai_status: Option<String>,
}

pub(crate) struct AssociationRows {
    pub evidence: AssociationEvidenceV1,
    pub provenance: Vec<IntelProvenanceV1>,
}

pub(crate) fn query_facts(
    connection: &Connection,
    intel_item_id: &str,
) -> rusqlite::Result<Option<DetailFactsRow>> {
    connection
        .query_row(
            "SELECT i.id,i.intel_item_id,i.revision,i.content_hash,c.content_state,i.publisher,
                    i.title,c.source_summary,i.published_at,i.collected_at,
                    p.provenance_id,p.source_id,p.source_kind,p.publisher,p.author,
                    p.author_availability,p.original_title,p.original_url,p.published_at,
                    p.collected_at,p.first_discovered_at,p.last_updated_at,p.availability_status
             FROM intel_items i
             JOIN intel_contents c ON c.item_id=i.id AND c.content_hash=i.content_hash
             JOIN item_provenance p ON p.item_id=i.id AND p.source_id=i.source_id
               AND p.stable_external_id=i.stable_external_id AND p.source_kind=i.source_kind
               AND p.publisher=i.publisher AND p.original_title=i.title
               AND p.original_url=i.original_url AND p.published_at IS i.published_at
               AND p.collected_at=i.collected_at AND p.content_hash=i.content_hash
             WHERE i.intel_item_id=?1 AND i.data_origin='real' AND i.source_kind='rss_atom'
               AND i.source_id IS NOT NULL AND c.content_state='metadata_only'
               AND p.author_availability IN ('available','unavailable')
               AND p.availability_status IN ('available','unavailable')",
            [intel_item_id],
            |row| {
                let url: String = row.get(17)?;
                let availability: String = row.get(22)?;
                Ok(DetailFactsRow {
                    item_id: row.get(0)?,
                    facts: SourceFactsV1 {
                        intel_item_id: row.get(1)?,
                        fact_revision: u64_at(row, 2)?,
                        content_hash: row.get(3)?,
                        content_state: row.get(4)?,
                        publisher: row.get(5)?,
                        title: row.get(6)?,
                        source_summary: row.get(7)?,
                        published_at: row.get(8)?,
                        collected_at: row.get(9)?,
                    },
                    primary: IntelProvenanceV1 {
                        provenance_id: row.get(10)?,
                        intel_item_id: row.get(1)?,
                        role: ProvenanceRoleV1::Primary,
                        source_id: row.get(11)?,
                        source_kind: row.get(12)?,
                        publisher: row.get(13)?,
                        author: row.get(14)?,
                        author_availability: row.get(15)?,
                        original_title: row.get(16)?,
                        display_url: url,
                        published_at: row.get(18)?,
                        collected_at: row.get(19)?,
                        first_discovered_at: row.get(20)?,
                        last_updated_at: row.get(21)?,
                        availability_status: availability.clone(),
                        can_open_original: false,
                    },
                })
            },
        )
        .optional()
}

pub(crate) fn query_rule(
    connection: &Connection,
    item_id: i64,
) -> rusqlite::Result<Option<RuleRow>> {
    connection
        .query_row(
            "SELECT rule_version,CAST(configuration_revision AS TEXT),configuration_hash,
                    CAST(fact_revision AS TEXT),CAST(evaluated_at_ms AS TEXT),CAST(score AS TEXT),
                    importance,stream_disposition,matched_tracks_json,
                    factor_results_json,filter_reasons_json,ai_status
             FROM rule_evaluations WHERE item_id=?1",
            [item_id],
            |row| {
                Ok(RuleRow {
                    rule_version: row.get(0)?,
                    configuration_revision: parse_optional_integer(row.get(1)?),
                    configuration_hash: row.get(2)?,
                    fact_revision: parse_optional_integer(row.get(3)?),
                    evaluated_at_ms: parse_optional_integer(row.get(4)?),
                    score: parse_optional_integer(row.get(5)?),
                    importance: row.get(6)?,
                    disposition: row.get(7)?,
                    matched_tracks_json: row.get(8)?,
                    factors_json: row.get(9)?,
                    filter_reasons_json: row.get(10)?,
                    ai_status: row.get(11)?,
                })
            },
        )
        .optional()
}

pub(crate) fn query_association(
    connection: &Connection,
    item_id: i64,
) -> rusqlite::Result<Option<AssociationRows>> {
    let association = connection
        .query_row(
            "SELECT a.association_id,a.relation_type,a.evidence_basis,a.basis_version,
                    (SELECT COUNT(*) FROM intel_association_members m2 WHERE m2.association_id=a.association_id)
             FROM intel_association_members m JOIN intel_associations a ON a.association_id=m.association_id
             WHERE m.item_id=?1",
            [item_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, u32>(3)?, usize_at(row, 4)?)),
        )
        .optional()?;
    let Some((association_id, relation_type, evidence_basis, basis_version, expected_count)) =
        association
    else {
        return Ok(None);
    };
    let mut statement = connection.prepare(
        "SELECT i.id,i.intel_item_id,p.provenance_id,p.source_id,p.source_kind,p.publisher,p.author,
                p.author_availability,p.original_title,p.original_url,p.published_at,p.collected_at,
                p.first_discovered_at,p.last_updated_at,p.availability_status
         FROM intel_association_members m
         JOIN intel_items i ON i.id=m.item_id AND i.data_origin='real' AND i.source_kind='rss_atom'
         JOIN intel_contents c ON c.item_id=i.id AND c.content_hash=i.content_hash
         JOIN item_provenance p ON p.item_id=i.id AND p.source_id=i.source_id
           AND p.stable_external_id=i.stable_external_id AND p.source_kind=i.source_kind
           AND p.publisher=i.publisher AND p.original_title=i.title
           AND p.original_url=i.original_url AND p.published_at IS i.published_at
           AND p.collected_at=i.collected_at AND p.content_hash=i.content_hash
           AND p.author_availability IN ('available','unavailable')
           AND p.availability_status IN ('available','unavailable')
         WHERE m.association_id=?1
         ORDER BY CASE WHEN i.id=?2 THEN 0 ELSE 1 END,i.intel_item_id
         LIMIT ?3",
    )?;
    let rows = statement.query_map(
        params![
            association_id,
            item_id,
            i64::try_from(INTEL_DETAIL_MAX_PROVENANCE + 1).expect("bounded provenance")
        ],
        |row| {
            let member_item_id: i64 = row.get(0)?;
            let url: String = row.get(9)?;
            let availability: String = row.get(14)?;
            Ok(IntelProvenanceV1 {
                provenance_id: row.get(2)?,
                intel_item_id: row.get(1)?,
                role: if member_item_id == item_id {
                    ProvenanceRoleV1::Primary
                } else {
                    ProvenanceRoleV1::Associated
                },
                source_id: row.get(3)?,
                source_kind: row.get(4)?,
                publisher: row.get(5)?,
                author: row.get(6)?,
                author_availability: row.get(7)?,
                original_title: row.get(8)?,
                display_url: url,
                published_at: row.get(10)?,
                collected_at: row.get(11)?,
                first_discovered_at: row.get(12)?,
                last_updated_at: row.get(13)?,
                availability_status: availability.clone(),
                can_open_original: false,
            })
        },
    )?;
    let mut provenance = Vec::new();
    let mut decode_failed = false;
    for row in rows {
        match row {
            Ok(value) => provenance.push(value),
            Err(_) => decode_failed = true,
        }
    }
    let complete = !decode_failed
        && (2..=INTEL_DETAIL_MAX_PROVENANCE).contains(&expected_count)
        && provenance.len() == expected_count;
    provenance.truncate(INTEL_DETAIL_MAX_PROVENANCE);
    Ok(Some(AssociationRows {
        evidence: AssociationEvidenceV1 {
            status: if complete {
                AssociationEvidenceStatusV1::Complete
            } else {
                AssociationEvidenceStatusV1::Incomplete
            },
            issue_code: (!complete).then(|| "association.incomplete".to_owned()),
            relation_type: Some(relation_type),
            evidence_basis: Some(evidence_basis),
            basis_version: Some(basis_version),
        },
        provenance,
    }))
}

pub(crate) fn resolve_original_url(
    connection: &Connection,
    intel_item_id: &str,
    provenance_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT p.original_url
             FROM intel_items root
             JOIN item_provenance p ON p.provenance_id=?2
             JOIN intel_items target ON target.id=p.item_id
               AND target.data_origin='real' AND target.source_kind='rss_atom'
               AND p.source_id=target.source_id AND p.stable_external_id=target.stable_external_id
               AND p.source_kind=target.source_kind AND p.publisher=target.publisher
               AND p.original_title=target.title AND p.original_url=target.original_url
               AND p.published_at IS target.published_at AND p.collected_at=target.collected_at
               AND p.content_hash=target.content_hash
             JOIN intel_contents c ON c.item_id=target.id AND c.content_hash=target.content_hash
             WHERE root.intel_item_id=?1 AND root.data_origin='real' AND root.source_kind='rss_atom'
               AND p.availability_status='available'
               AND (target.id=root.id OR EXISTS(
                    SELECT 1 FROM intel_association_members root_member
                    JOIN intel_association_members target_member
                      ON target_member.association_id=root_member.association_id
                    WHERE root_member.item_id=root.id AND target_member.item_id=target.id))",
            params![intel_item_id, provenance_id],
            |row| row.get(0),
        )
        .optional()
}

fn u64_at(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Integer, Box::new(error))
    })
}

fn parse_optional_integer<T: std::str::FromStr>(value: Option<String>) -> Option<T> {
    value?.parse().ok()
}

fn usize_at(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let value = row.get::<_, i64>(index)?;
    usize::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Integer, Box::new(error))
    })
}
