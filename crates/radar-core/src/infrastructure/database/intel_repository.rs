//! SQL plumbing for normalized intelligence facts and provenance.

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::intel::NormalizedIntelCandidate;
use crate::domain::sources::CandidateDisposition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IntelWriteOutcome {
    pub item_id: i64,
    pub disposition: CandidateDisposition,
}

pub(crate) fn upsert_normalized_intel(
    connection: &Connection,
    candidate: &NormalizedIntelCandidate,
    first_discovered_at: &str,
    last_updated_at: &str,
) -> rusqlite::Result<IntelWriteOutcome> {
    let existing = connection
        .query_row(
            "SELECT id,content_hash FROM intel_items WHERE source_id=?1 AND stable_external_id=?2",
            params![candidate.source_id, candidate.stable_external_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let (item_id, disposition) = match existing {
        None => {
            connection.execute(
                "INSERT INTO intel_items
                 (intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at)
                 VALUES(?1,?2,'real',?3,?4,?5,?6,1,?7,?8,NULL,NULL,?9,NULL,NULL,?10,?11)",
                params![
                    candidate.intel_item_id.as_str(),
                    candidate.canonical_external_id,
                    candidate.source_id,
                    candidate.source_kind,
                    candidate.stable_external_id,
                    candidate.content_hash,
                    candidate.publisher,
                    candidate.original_title,
                    candidate.original_url,
                    candidate.published_at,
                    candidate.collected_at,
                ],
            )?;
            (connection.last_insert_rowid(), CandidateDisposition::New)
        }
        Some((item_id, hash)) if hash == candidate.content_hash => {
            (item_id, CandidateDisposition::Unchanged)
        }
        Some((item_id, _)) => {
            connection.execute(
                "UPDATE intel_items SET content_hash=?1,revision=revision+1,publisher=?2,title=?3,original_url=?4,published_at=?5,collected_at=?6
                 WHERE id=?7",
                params![
                    candidate.content_hash,
                    candidate.publisher,
                    candidate.original_title,
                    candidate.original_url,
                    candidate.published_at,
                    candidate.collected_at,
                    item_id,
                ],
            )?;
            (item_id, CandidateDisposition::Changed)
        }
    };
    if disposition != CandidateDisposition::Unchanged {
        let warnings_json = serde_json::to_string(&candidate.warnings)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        connection.execute(
            "INSERT INTO intel_contents(item_id,what_happened,facts_json,source_summary,content_hash,content_state)
             VALUES(?1,NULL,NULL,?2,?3,'metadata_only')
             ON CONFLICT(item_id) DO UPDATE SET source_summary=excluded.source_summary,content_hash=excluded.content_hash,content_state='metadata_only'",
            params![item_id, candidate.source_summary, candidate.content_hash],
        )?;
        let provenance_id = format!("prov:{}", candidate.intel_item_id.as_str());
        connection.execute(
            "INSERT INTO item_provenance
             (provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,author_availability,original_title,original_url,published_at,collected_at,first_discovered_at,last_updated_at,availability_status,warnings_json,content_hash,deterministic_association_basis)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,'available',?15,?16,'source_kind+canonical_external_id')
             ON CONFLICT(item_id) DO UPDATE SET publisher=excluded.publisher,author=excluded.author,author_availability=excluded.author_availability,original_title=excluded.original_title,original_url=excluded.original_url,published_at=excluded.published_at,collected_at=excluded.collected_at,last_updated_at=excluded.last_updated_at,availability_status='available',warnings_json=excluded.warnings_json,content_hash=excluded.content_hash",
            params![
                provenance_id,
                item_id,
                candidate.source_id,
                candidate.stable_external_id,
                candidate.source_kind,
                candidate.publisher,
                candidate.author,
                candidate.author_availability.as_str(),
                candidate.original_title,
                candidate.original_url,
                candidate.published_at,
                candidate.collected_at,
                first_discovered_at,
                last_updated_at,
                warnings_json,
                candidate.content_hash,
            ],
        )?;
    }
    Ok(IntelWriteOutcome {
        item_id,
        disposition,
    })
}
