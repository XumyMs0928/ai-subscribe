//! Read-only `SQLite` projection for the real RSS intelligence feed.

use rusqlite::{Connection, OptionalExtension, params};

use crate::contracts::dto::intel_feed::{
    IntelFeedFiltersV1, IntelFeedItemV1, IntelFeedStreamV1, LIST_EXCERPT_MAX_CHARS,
};

pub(crate) struct FeedBoundary {
    pub score: u8,
    pub intel_item_id: String,
}

pub(crate) struct FeedRows {
    pub items: Vec<IntelFeedItemV1>,
    pub next_boundary: Option<FeedBoundary>,
}

pub(crate) struct FeedQuery<'a> {
    pub stream: IntelFeedStreamV1,
    pub filters: &'a IntelFeedFiltersV1,
    pub rule_version: &'a str,
    pub configuration_revision: u64,
    pub configuration_hash: &'a str,
    pub cutoff: Option<&'a str>,
    pub as_of_rfc3339: &'a str,
    pub as_of_ms: u64,
    pub after: Option<&'a FeedBoundary>,
    pub limit: u32,
}

const FEED_QUERY_SQL: &str = "SELECT i.intel_item_id,i.source_id,i.source_kind,i.publisher,i.title,
            c.source_summary,i.published_at,i.collected_at,r.importance,r.score,
            r.matched_tracks_json,r.ai_status
     FROM rule_evaluations r
     JOIN intel_items i ON i.id=r.item_id AND r.fact_revision=i.revision
     JOIN intel_contents c ON c.item_id=i.id AND c.content_hash=i.content_hash
     JOIN item_provenance p ON p.item_id=i.id AND p.source_id=i.source_id
       AND p.stable_external_id=i.stable_external_id AND p.source_kind=i.source_kind
       AND p.publisher=i.publisher AND p.original_title=i.title
       AND p.original_url=i.original_url AND p.published_at IS i.published_at
       AND p.collected_at=i.collected_at AND p.content_hash=i.content_hash
       AND p.availability_status='available'
     WHERE i.data_origin='real' AND i.source_kind='rss_atom' AND i.source_id IS NOT NULL
       AND r.stream_disposition=?1 AND r.rule_version=?2
       AND r.configuration_revision=?3 AND r.configuration_hash=?4
       AND (json_array_length(?5)=0 OR EXISTS(
             SELECT 1 FROM json_each(r.matched_tracks_json) matched
             JOIN json_each(?5) selected ON selected.value=matched.value))
       AND (json_array_length(?6)=0 OR EXISTS(
             SELECT 1 FROM json_each(?6) selected WHERE selected.value=i.source_id))
       AND (json_array_length(?7)=0 OR EXISTS(
             SELECT 1 FROM json_each(?7) selected WHERE selected.value=r.importance))
       AND (?8 IS NULL OR COALESCE(i.published_at,i.collected_at)>=?8)
       AND COALESCE(i.published_at,i.collected_at)<=?9
       AND r.evaluated_at_ms<=?10
       AND (?11<0 OR r.score<?11 OR (r.score=?11 AND i.intel_item_id>?12))
     ORDER BY r.score DESC,i.intel_item_id ASC LIMIT ?13";

pub(crate) fn query(connection: &Connection, input: &FeedQuery<'_>) -> rusqlite::Result<FeedRows> {
    let tracks = json(&input.filters.track_ids)?;
    let sources = json(&input.filters.source_ids)?;
    let importance = json(&input.filters.importance)?;
    let after_score = input
        .after
        .map_or(-1_i64, |boundary| i64::from(boundary.score));
    let after_id = input
        .after
        .map_or("", |boundary| boundary.intel_item_id.as_str());
    let revision = i64::try_from(input.configuration_revision)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let as_of_ms = i64::try_from(input.as_of_ms)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut statement = connection.prepare(FEED_QUERY_SQL)?;
    let rows = statement.query_map(
        params![
            input.stream.as_str(),
            input.rule_version,
            revision,
            input.configuration_hash,
            tracks,
            sources,
            importance,
            input.cutoff,
            input.as_of_rfc3339,
            as_of_ms,
            after_score,
            after_id,
            i64::from(input.limit) + 1,
        ],
        |row| {
            let source_summary: Option<String> = row.get(5)?;
            let (source_excerpt, excerpt_truncated) = truncate_excerpt(source_summary.as_deref());
            let matched_json: String = row.get(10)?;
            let matched_track_ids = serde_json::from_str(&matched_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    matched_json.len(),
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(IntelFeedItemV1 {
                contract_version: 1,
                intel_item_id: row.get(0)?,
                source_id: row.get(1)?,
                source_kind: row.get(2)?,
                publisher: row.get(3)?,
                title: row.get(4)?,
                source_excerpt,
                excerpt_truncated,
                published_at: row.get(6)?,
                collected_at: row.get(7)?,
                importance: row.get(8)?,
                score: row.get(9)?,
                matched_track_ids,
                stream_disposition: input.stream,
                ai_status: row.get(11)?,
            })
        },
    )?;
    let mut rows = rows.collect::<Result<Vec<_>, _>>()?;
    let has_more = rows.len() > input.limit as usize;
    rows.truncate(input.limit as usize);
    let next_boundary = has_more.then(|| {
        let item = rows.last().expect("continuation page is non-empty");
        FeedBoundary {
            score: item.score,
            intel_item_id: item.intel_item_id.clone(),
        }
    });
    Ok(FeedRows {
        items: rows,
        next_boundary,
    })
}

pub(crate) fn resolve_boundary(
    connection: &Connection,
    input: &FeedQuery<'_>,
    intel_item_id: &str,
    score: u8,
) -> rusqlite::Result<Option<FeedBoundary>> {
    let tracks = json(&input.filters.track_ids)?;
    let sources = json(&input.filters.source_ids)?;
    let importance = json(&input.filters.importance)?;
    let revision = i64::try_from(input.configuration_revision)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    connection
        .query_row(
            "SELECT i.intel_item_id
             FROM rule_evaluations r
             JOIN intel_items i ON i.id=r.item_id AND r.fact_revision=i.revision
             JOIN intel_contents c ON c.item_id=i.id AND c.content_hash=i.content_hash
             JOIN item_provenance p ON p.item_id=i.id AND p.source_id=i.source_id
               AND p.stable_external_id=i.stable_external_id AND p.source_kind=i.source_kind
               AND p.publisher=i.publisher AND p.original_title=i.title
               AND p.original_url=i.original_url AND p.published_at IS i.published_at
               AND p.collected_at=i.collected_at AND p.content_hash=i.content_hash
               AND p.availability_status='available'
             WHERE i.intel_item_id=?1 AND r.score=?2
               AND i.data_origin='real' AND i.source_kind='rss_atom' AND i.source_id IS NOT NULL
               AND r.stream_disposition=?3 AND r.rule_version=?4
               AND r.configuration_revision=?5 AND r.configuration_hash=?6
               AND (json_array_length(?7)=0 OR EXISTS(
                     SELECT 1 FROM json_each(r.matched_tracks_json) matched
                     JOIN json_each(?7) selected ON selected.value=matched.value))
               AND (json_array_length(?8)=0 OR EXISTS(
                     SELECT 1 FROM json_each(?8) selected WHERE selected.value=i.source_id))
               AND (json_array_length(?9)=0 OR EXISTS(
                     SELECT 1 FROM json_each(?9) selected WHERE selected.value=r.importance))
               AND (?10 IS NULL OR COALESCE(i.published_at,i.collected_at)>=?10)
               AND COALESCE(i.published_at,i.collected_at)<=?11
               AND r.evaluated_at_ms<=?12",
            params![
                intel_item_id,
                score,
                input.stream.as_str(),
                input.rule_version,
                revision,
                input.configuration_hash,
                tracks,
                sources,
                importance,
                input.cutoff,
                input.as_of_rfc3339,
                i64::try_from(input.as_of_ms).map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                })?,
            ],
            |row| {
                Ok(FeedBoundary {
                    score,
                    intel_item_id: row.get(0)?,
                })
            },
        )
        .optional()
}

#[cfg(test)]
pub(crate) fn explain_query_plan(
    connection: &Connection,
    input: &FeedQuery<'_>,
) -> rusqlite::Result<Vec<String>> {
    let tracks = json(&input.filters.track_ids)?;
    let sources = json(&input.filters.source_ids)?;
    let importance = json(&input.filters.importance)?;
    let after_score = input
        .after
        .map_or(-1_i64, |boundary| i64::from(boundary.score));
    let after_id = input
        .after
        .map_or("", |boundary| boundary.intel_item_id.as_str());
    let revision = i64::try_from(input.configuration_revision)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let as_of_ms = i64::try_from(input.as_of_ms)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut statement = connection.prepare(&format!("EXPLAIN QUERY PLAN {FEED_QUERY_SQL}"))?;
    statement
        .query_map(
            params![
                input.stream.as_str(),
                input.rule_version,
                revision,
                input.configuration_hash,
                tracks,
                sources,
                importance,
                input.cutoff,
                input.as_of_rfc3339,
                as_of_ms,
                after_score,
                after_id,
                i64::from(input.limit) + 1,
            ],
            |row| row.get::<_, String>(3),
        )?
        .collect()
}

fn json<T: serde::Serialize>(value: &T) -> rusqlite::Result<String> {
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn truncate_excerpt(value: Option<&str>) -> (Option<String>, bool) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return (None, false);
    };
    let mut chars = value.chars();
    let excerpt = chars
        .by_ref()
        .take(LIST_EXCERPT_MAX_CHARS)
        .collect::<String>();
    let truncated = chars.next().is_some();
    (Some(excerpt), truncated)
}
