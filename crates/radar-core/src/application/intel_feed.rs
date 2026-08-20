//! Read-only, versioned real RSS intelligence feed query.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use super::configuration::read_configuration;
use super::demo::DemoStore;
use crate::contracts::dto::intel_feed::{
    INTEL_FEED_MAX_CURSOR_BYTES, INTEL_FEED_MAX_PAGE_SIZE, INTEL_FEED_MAX_SOURCE_FILTERS,
    INTEL_FEED_MAX_TRACK_FILTERS, IntelFeedFiltersV1, IntelFeedPageV1, IntelFeedSortV1,
    IntelFeedTimeWindowV1, QueryIntelFeedInputV1,
};
use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::rules::intelligence_value::INTELLIGENCE_VALUE_RULE_VERSION;
#[cfg(test)]
use crate::infrastructure::database::intel_feed_repository::explain_query_plan;
use crate::infrastructure::database::intel_feed_repository::{FeedQuery, query, resolve_boundary};

const FEED_CURSOR_PREFIX: &str = "feed-v1:";

impl DemoStore {
    /// Returns one stable keyset page from the current real RSS rule projection.
    ///
    /// # Errors
    /// Returns a stable validation error for malformed input/cursor and a storage error for
    /// unreadable or contradictory current projections.
    pub fn query_intel_feed(
        &self,
        input: &QueryIntelFeedInputV1,
    ) -> Result<IntelFeedPageV1, AppError> {
        self.query_intel_feed_at(input, now_ms()?)
    }

    fn query_intel_feed_at(
        &self,
        input: &QueryIntelFeedInputV1,
        requested_as_of_ms: u64,
    ) -> Result<IntelFeedPageV1, AppError> {
        let filters = normalize_filters(input)?;
        let configuration = read_configuration(&self.connection)?;
        let filter_hash = filter_hash(input.stream.as_str(), &filters, input.sort.as_str())?;
        let cursor = input
            .cursor
            .as_deref()
            .map(|value| decode_cursor(&self.feed_cursor_secret, value))
            .transpose()?;
        let as_of_ms = cursor
            .as_ref()
            .map_or(requested_as_of_ms, |value| value.as_of_ms);
        if as_of_ms == 0 {
            return Err(feed_error(ErrorCode::ValidationSource, "feed-as-of"));
        }
        let cutoff = cutoff_rfc3339(filters.time_window, as_of_ms);
        let as_of_rfc3339 = super::sources::unix_ms_to_rfc3339(as_of_ms);
        let after = if let Some(cursor) = &cursor {
            if cursor.stream != input.stream.as_str()
                || cursor.filter_hash != filter_hash
                || cursor.rule_version != INTELLIGENCE_VALUE_RULE_VERSION
                || cursor.configuration_revision != configuration.revision
                || cursor.configuration_hash != configuration.normalized_config_hash
            {
                return Err(feed_error(
                    ErrorCode::ValidationSource,
                    "feed-cursor-identity",
                ));
            }
            let boundary_query = FeedQuery {
                stream: input.stream,
                filters: &filters,
                rule_version: INTELLIGENCE_VALUE_RULE_VERSION,
                configuration_revision: configuration.revision,
                configuration_hash: &configuration.normalized_config_hash,
                cutoff: cutoff.as_deref(),
                as_of_rfc3339: &as_of_rfc3339,
                as_of_ms,
                after: None,
                limit: input.limit,
            };
            Some(
                resolve_boundary(
                    &self.connection,
                    &boundary_query,
                    &cursor.intel_item_id,
                    cursor.score,
                )
                .map_err(|_| feed_error(ErrorCode::StorageSource, "feed-cursor-boundary-read"))?
                .ok_or_else(|| feed_error(ErrorCode::ValidationSource, "feed-cursor-boundary"))?,
            )
        } else {
            None
        };
        let rows = query(
            &self.connection,
            &FeedQuery {
                stream: input.stream,
                filters: &filters,
                rule_version: INTELLIGENCE_VALUE_RULE_VERSION,
                configuration_revision: configuration.revision,
                configuration_hash: &configuration.normalized_config_hash,
                cutoff: cutoff.as_deref(),
                as_of_rfc3339: &as_of_rfc3339,
                as_of_ms,
                after: after.as_ref(),
                limit: input.limit,
            },
        )
        .map_err(|_| feed_error(ErrorCode::StorageSource, "feed-query"))?;
        let next_cursor = rows
            .next_boundary
            .as_ref()
            .map(|boundary| {
                encode_cursor(
                    &self.feed_cursor_secret,
                    &FeedCursor {
                        stream: input.stream.as_str().to_owned(),
                        filter_hash: filter_hash.clone(),
                        rule_version: INTELLIGENCE_VALUE_RULE_VERSION.to_owned(),
                        configuration_revision: configuration.revision,
                        configuration_hash: configuration.normalized_config_hash.clone(),
                        as_of_ms,
                        score: boundary.score,
                        intel_item_id: boundary.intel_item_id.clone(),
                    },
                )
            })
            .transpose()?;
        Ok(IntelFeedPageV1 {
            contract_version: 1,
            stream: input.stream,
            filters,
            sort: input.sort,
            rule_version: INTELLIGENCE_VALUE_RULE_VERSION.to_owned(),
            configuration_revision: configuration.revision,
            configuration_hash: configuration.normalized_config_hash,
            as_of_ms,
            items: rows.items,
            next_cursor,
        })
    }
}

fn normalize_filters(input: &QueryIntelFeedInputV1) -> Result<IntelFeedFiltersV1, AppError> {
    if input.contract_version != 1
        || input.sort != IntelFeedSortV1::ScoreDesc
        || !(1..=INTEL_FEED_MAX_PAGE_SIZE).contains(&input.limit)
        || input.filters.track_ids.len() > INTEL_FEED_MAX_TRACK_FILTERS
        || input.filters.source_ids.len() > INTEL_FEED_MAX_SOURCE_FILTERS
        || input.filters.importance.len() > 3
        || input
            .cursor
            .as_ref()
            .is_some_and(|value| value.len() > INTEL_FEED_MAX_CURSOR_BYTES)
    {
        return Err(feed_error(ErrorCode::ValidationSource, "feed-input"));
    }
    let mut filters = input.filters.clone();
    normalize_tokens(&mut filters.track_ids, is_safe_id)?;
    normalize_tokens(&mut filters.source_ids, |value| {
        value.len() == 31 && value.starts_with("source:") && is_safe_id(value)
    })?;
    normalize_tokens(&mut filters.importance, |value| {
        matches!(value, "low" | "medium" | "high")
    })?;
    Ok(filters)
}

fn normalize_tokens(
    values: &mut Vec<String>,
    validate: impl Fn(&str) -> bool,
) -> Result<(), AppError> {
    if values.iter().any(|value| !validate(value)) {
        return Err(feed_error(ErrorCode::ValidationSource, "feed-filter"));
    }
    values.sort();
    values.dedup();
    Ok(())
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn cutoff_rfc3339(window: IntelFeedTimeWindowV1, as_of_ms: u64) -> Option<String> {
    let age = match window {
        IntelFeedTimeWindowV1::AllTime => return None,
        IntelFeedTimeWindowV1::Last24h => 86_400_000,
        IntelFeedTimeWindowV1::Last7d => 604_800_000,
        IntelFeedTimeWindowV1::Last30d => 2_592_000_000,
    };
    Some(super::sources::unix_ms_to_rfc3339(
        as_of_ms.saturating_sub(age),
    ))
}

fn filter_hash(stream: &str, filters: &IntelFeedFiltersV1, sort: &str) -> Result<String, AppError> {
    serde_json::to_vec(&(stream, filters, sort))
        .map(|value| digest_hex(&value))
        .map_err(|_| feed_error(ErrorCode::ValidationSource, "feed-filter-json"))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FeedCursor {
    stream: String,
    filter_hash: String,
    rule_version: String,
    configuration_revision: u64,
    configuration_hash: String,
    as_of_ms: u64,
    score: u8,
    intel_item_id: String,
}

fn encode_cursor(secret: &[u8; 32], cursor: &FeedCursor) -> Result<String, AppError> {
    let json = serde_json::to_vec(cursor)
        .map_err(|_| feed_error(ErrorCode::StorageSource, "feed-cursor-json"))?;
    let signature = hex(&hmac_sha256(secret, &json));
    let encoded = format!("{FEED_CURSOR_PREFIX}{}:{signature}", hex(&json));
    if encoded.len() > INTEL_FEED_MAX_CURSOR_BYTES {
        return Err(feed_error(ErrorCode::StorageSource, "feed-cursor-size"));
    }
    Ok(encoded)
}

fn decode_cursor(secret: &[u8; 32], value: &str) -> Result<FeedCursor, AppError> {
    let body = value
        .strip_prefix(FEED_CURSOR_PREFIX)
        .ok_or_else(|| feed_error(ErrorCode::ValidationSource, "feed-cursor"))?;
    let (encoded, checksum) = body
        .rsplit_once(':')
        .ok_or_else(|| feed_error(ErrorCode::ValidationSource, "feed-cursor"))?;
    let json =
        unhex(encoded).ok_or_else(|| feed_error(ErrorCode::ValidationSource, "feed-cursor"))?;
    let expected = hex(&hmac_sha256(secret, &json));
    if value.len() > INTEL_FEED_MAX_CURSOR_BYTES
        || checksum.len() != 64
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || !constant_time_eq(checksum.as_bytes(), expected.as_bytes())
    {
        return Err(feed_error(ErrorCode::ValidationSource, "feed-cursor"));
    }
    let cursor: FeedCursor = serde_json::from_slice(&json)
        .map_err(|_| feed_error(ErrorCode::ValidationSource, "feed-cursor"))?;
    if cursor.as_of_ms == 0
        || cursor.score > 100
        || cursor.intel_item_id.len() != 70
        || !cursor.intel_item_id.starts_with("intel:")
        || !is_lower_hex_64(&cursor.intel_item_id[6..])
        || cursor.configuration_revision == 0
        || !is_lower_hex_64(&cursor.filter_hash)
        || !is_lower_hex_64(&cursor.configuration_hash)
    {
        return Err(feed_error(ErrorCode::ValidationSource, "feed-cursor"));
    }
    Ok(cursor)
}

fn hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn unhex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

fn digest_hex(value: &[u8]) -> String {
    hex(&Sha256::digest(value))
}

fn hmac_sha256(secret: &[u8; 32], value: &[u8]) -> [u8; 32] {
    let mut inner_pad = [0x36_u8; 64];
    let mut outer_pad = [0x5c_u8; 64];
    for (index, byte) in secret.iter().enumerate() {
        inner_pad[index] ^= byte;
        outer_pad[index] ^= byte;
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(value);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner);
    outer.finalize().into()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn is_lower_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn now_ms() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| feed_error(ErrorCode::StorageSource, "feed-time"))
        .and_then(|duration| {
            u64::try_from(duration.as_millis())
                .map_err(|_| feed_error(ErrorCode::StorageSource, "feed-time-range"))
        })
}

fn feed_error(code: ErrorCode, boundary: &'static str) -> AppError {
    AppError::from_code(code, boundary)
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File, OpenOptions};
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    use rusqlite::params;
    use serde_json::json;

    use super::*;

    const DATASET_SIZE: usize = 50_000;
    const SAMPLE_COUNT: usize = 30;
    const FIXED_AS_OF_MS: u64 = 1_777_000_000_000;
    const SOURCE_ID: &str = "source:999999999999999999999999";

    #[allow(clippy::too_many_lines)] // The deterministic row manifest intentionally mirrors every seeded projection column.
    fn seed_fixed_feed(store: &mut DemoStore, dataset_size: usize) -> String {
        let configuration_hash = read_configuration(&store.connection)
            .expect("configuration")
            .normalized_config_hash;
        let transaction = store.connection.transaction().expect("seed transaction");
        transaction
            .execute(
                "INSERT INTO sources
                 (source_id,configuration_version,source_kind,canonical_url,enabled,revision,status,
                  consecutive_failures,retryability,created_at_ms,updated_at_ms)
                 VALUES(?1,1,'rss_atom','https://feed-performance.example/feed.xml',1,1,'ready',
                        0,'manual',1,1)",
                [SOURCE_ID],
            )
            .expect("performance source");
        let mut dataset_hasher = Sha256::new();
        for index in 0..dataset_size {
            let external = format!("perf-{index:05}");
            let intel_id = format!("intel:{index:064x}");
            let content_hash = format!("{index:064x}");
            transaction
                .execute(
                    "INSERT INTO intel_items
                     (intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,
                      content_hash,revision,publisher,title,original_url,published_at,collected_at)
                     VALUES(?1,?2,'real',?3,'rss_atom',?2,?4,1,'Performance Publisher',?5,?6,
                             '2026-04-23T14:00:00Z','2026-04-23T14:01:00Z')",
                    params![
                        intel_id,
                        external,
                        SOURCE_ID,
                        content_hash,
                        format!("AI agent release {index}"),
                        format!("https://feed-performance.example/items/{index}"),
                    ],
                )
                .expect("performance fact");
            let item_id = transaction.last_insert_rowid();
            transaction
                .execute(
                    "INSERT INTO intel_contents
                     (item_id,what_happened,facts_json,source_summary,content_hash,content_state)
                     VALUES(?1,NULL,NULL,'Bounded local performance excerpt',?2,'metadata_only')",
                    params![item_id, content_hash],
                )
                .expect("performance content");
            transaction
                .execute(
                    "INSERT INTO item_provenance
                     (provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,
                      author_availability,original_title,original_url,published_at,collected_at,
                      first_discovered_at,last_updated_at,availability_status,warnings_json,
                      content_hash,deterministic_association_basis)
                     VALUES(?1,?2,?3,?4,'rss_atom','Performance Publisher',NULL,'unavailable',?5,?6,
                             '2026-04-23T14:00:00Z','2026-04-23T14:01:00Z','2026-04-23T14:01:00Z',
                             '2026-04-23T14:01:00Z','available','[]',?7,
                            'source_kind+canonical_external_id')",
                    params![
                        format!("prov:perf:{index:05}"),
                        item_id,
                        SOURCE_ID,
                        external,
                        format!("AI agent release {index}"),
                        format!("https://feed-performance.example/items/{index}"),
                        content_hash,
                    ],
                )
                .expect("performance provenance");
            let high = index % 2 == 0;
            transaction
                .execute(
                    "INSERT INTO rule_evaluations
                     (item_id,why_it_matters,possible_impact,importance,reasons_json,rule_version,
                      configuration_revision,configuration_hash,fact_revision,evaluated_at_ms,score,
                      stream_disposition,matched_tracks_json,factor_results_json,filter_reasons_json,
                      ai_status)
                     VALUES(?1,'fixture','fixture',?2,'[]',?3,?4,?5,1,?6,?7,?8,?9,'[]','[]','unavailable')",
                    params![
                        item_id,
                        if high { "high" } else { "low" },
                        INTELLIGENCE_VALUE_RULE_VERSION,
                        1_i64,
                        configuration_hash,
                        i64::try_from(FIXED_AS_OF_MS).expect("fixed time range"),
                        if high { 90 } else { 40 },
                        if high { "high_value" } else { "ordinary_candidate" },
                        if high { "[\"ai_agents\"]" } else { "[]" },
                    ],
                )
                .expect("performance rule");
            dataset_hasher.update(
                serde_json::to_vec(&(
                    &intel_id,
                    &external,
                    SOURCE_ID,
                    &content_hash,
                    "Performance Publisher",
                    format!("AI agent release {index}"),
                    format!("https://feed-performance.example/items/{index}"),
                    "2026-04-23T14:00:00Z",
                    "2026-04-23T14:01:00Z",
                    "Bounded local performance excerpt",
                    high,
                ))
                .expect("canonical dataset row"),
            );
        }
        transaction.commit().expect("seed commit");
        hex(&dataset_hasher.finalize())
    }

    fn default_feed_input(limit: u32) -> QueryIntelFeedInputV1 {
        QueryIntelFeedInputV1 {
            contract_version: 1,
            stream: crate::contracts::dto::intel_feed::IntelFeedStreamV1::HighValue,
            filters: IntelFeedFiltersV1 {
                track_ids: Vec::new(),
                source_ids: Vec::new(),
                time_window: IntelFeedTimeWindowV1::AllTime,
                importance: Vec::new(),
            },
            sort: IntelFeedSortV1::ScoreDesc,
            cursor: None,
            limit,
        }
    }

    #[test]
    fn feed_requires_current_fact_and_content_projection_without_writes() {
        let mut store = DemoStore::open_in_memory().expect("v10 store");
        seed_fixed_feed(&mut store, 3);
        let changes_before = store.connection.total_changes();
        let page = store
            .query_intel_feed_at(&default_feed_input(30), FIXED_AS_OF_MS)
            .expect("current projection");
        assert_eq!(page.items.len(), 2);
        assert_eq!(store.connection.total_changes(), changes_before);

        store
            .connection
            .execute(
                "UPDATE intel_items SET revision=2 WHERE intel_item_id=?1",
                [format!("intel:{:064x}", 0)],
            )
            .expect("advance fact without evaluation");
        let current = store
            .query_intel_feed_at(&default_feed_input(30), FIXED_AS_OF_MS)
            .expect("stale rule excluded");
        assert_eq!(current.items.len(), 1);

        store
            .connection
            .execute(
                "UPDATE intel_contents SET content_hash=?1 WHERE item_id=(SELECT id FROM intel_items WHERE intel_item_id=?2)",
                params!["f".repeat(64), format!("intel:{:064x}", 2)],
            )
            .expect("break content identity");
        assert!(
            store
                .query_intel_feed_at(&default_feed_input(30), FIXED_AS_OF_MS)
                .expect("inconsistent content excluded")
                .items
                .is_empty()
        );
    }

    #[test]
    fn feed_snapshot_excludes_future_time_and_late_evaluation() {
        let mut store = DemoStore::open_in_memory().expect("v10 store");
        seed_fixed_feed(&mut store, 5);
        let mut input = default_feed_input(1);
        let first = store
            .query_intel_feed_at(&input, FIXED_AS_OF_MS)
            .expect("first page");
        input.cursor = first.next_cursor;
        store
            .connection
            .execute(
                "UPDATE rule_evaluations SET evaluated_at_ms=?1 WHERE item_id=(SELECT id FROM intel_items WHERE intel_item_id=?2)",
                params![
                    i64::try_from(FIXED_AS_OF_MS + 1).expect("time"),
                    format!("intel:{:064x}", 2)
                ],
            )
            .expect("late evaluation");
        let next = store
            .query_intel_feed_at(&input, FIXED_AS_OF_MS)
            .expect("snapshot continuation");
        assert!(
            next.items
                .iter()
                .all(|item| item.intel_item_id != format!("intel:{:064x}", 2))
        );

        store
            .connection
            .execute_batch(
                "UPDATE intel_items SET published_at='2099-01-01T00:00:00Z',collected_at='2099-01-01T00:00:01Z' WHERE intel_item_id='intel:0000000000000000000000000000000000000000000000000000000000000004';
                 UPDATE item_provenance SET published_at='2099-01-01T00:00:00Z',collected_at='2099-01-01T00:00:01Z' WHERE item_id=(SELECT id FROM intel_items WHERE intel_item_id='intel:0000000000000000000000000000000000000000000000000000000000000004');",
            )
            .expect("future fact");
        assert!(
            store
                .query_intel_feed_at(&default_feed_input(30), FIXED_AS_OF_MS)
                .expect("future excluded")
                .items
                .iter()
                .all(|item| !item
                    .published_at
                    .as_deref()
                    .is_some_and(|time| time.starts_with("2099-")))
        );
    }

    #[test]
    fn recomputing_a_public_digest_cannot_forge_a_feed_cursor() {
        let mut store = DemoStore::open_in_memory().expect("v10 store");
        seed_fixed_feed(&mut store, 5);
        let first = store
            .query_intel_feed_at(&default_feed_input(1), FIXED_AS_OF_MS)
            .expect("first page");
        let cursor = first.next_cursor.expect("continuation");
        let body = cursor
            .strip_prefix(FEED_CURSOR_PREFIX)
            .and_then(|value| value.rsplit_once(':'))
            .expect("cursor body");
        let mut json = unhex(body.0).expect("cursor json");
        let marker = format!("\"as_of_ms\":{FIXED_AS_OF_MS}");
        let replacement = format!("\"as_of_ms\":{}", FIXED_AS_OF_MS - 1);
        let decoded = String::from_utf8(json).expect("cursor utf8");
        json = decoded.replacen(&marker, &replacement, 1).into_bytes();
        let forged = format!("{FEED_CURSOR_PREFIX}{}:{}", hex(&json), digest_hex(&json));
        let mut input = default_feed_input(1);
        input.cursor = Some(forged);
        assert_eq!(
            store
                .query_intel_feed_at(&input, FIXED_AS_OF_MS)
                .expect_err("public digest is not a valid HMAC")
                .code(),
            "validation.source"
        );
    }

    #[test]
    fn public_item_identity_orders_ties_and_limit_100_pages_without_duplicates() {
        let mut store = DemoStore::open_in_memory().expect("v10 store");
        seed_fixed_feed(&mut store, 201);
        store
            .connection
            .execute(
                "UPDATE intel_items SET intel_item_id=?1 WHERE intel_item_id=?2",
                params![
                    format!("intel:{}", "f".repeat(64)),
                    format!("intel:{:064x}", 0)
                ],
            )
            .expect("separate public order from row order");
        let mut input = default_feed_input(100);
        let first = store
            .query_intel_feed_at(&input, FIXED_AS_OF_MS)
            .expect("first 100");
        assert_eq!(first.items.len(), 100);
        assert!(first.items.windows(2).all(|pair| {
            pair[0].score > pair[1].score
                || (pair[0].score == pair[1].score && pair[0].intel_item_id < pair[1].intel_item_id)
        }));
        input.cursor = first.next_cursor;
        let second = store
            .query_intel_feed_at(&input, FIXED_AS_OF_MS)
            .expect("remaining page");
        assert_eq!(second.items.len(), 1);
        assert!(second.next_cursor.is_none());
        let first_ids = first
            .items
            .iter()
            .map(|item| item.intel_item_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        assert!(
            second
                .items
                .iter()
                .all(|item| !first_ids.contains(item.intel_item_id.as_str()))
        );
    }

    fn percentile_95(samples: &[u128]) -> u128 {
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        sorted[(sorted.len() * 95).div_ceil(100) - 1]
    }

    static PERFORMANCE_EVIDENCE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn reserve_performance_evidence(directory: &Path) -> (PathBuf, PathBuf, File) {
        loop {
            let invocation = PERFORMANCE_EVIDENCE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let run_name = format!("intel-feed-performance-{}-{invocation}", std::process::id());
            let evidence_path = directory.join(format!("{run_name}.json"));
            let temporary_path = directory.join(format!("{run_name}.json.tmp"));
            if evidence_path.exists() {
                continue;
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
            {
                Ok(file) => return (evidence_path, temporary_path, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("reserve performance evidence: {error}"),
            }
        }
    }

    #[test]
    fn performance_evidence_reservations_are_invocation_scoped_and_atomic() {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/story-4-5");
        fs::create_dir_all(&directory).expect("evidence directory");
        let (first_evidence, first_temporary, first_file) =
            reserve_performance_evidence(&directory);
        let (second_evidence, second_temporary, second_file) =
            reserve_performance_evidence(&directory);
        let process_id = std::process::id().to_string();
        assert!(
            first_evidence
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(|name| name.contains(&process_id))
        );
        assert_ne!(first_evidence, second_evidence);
        assert_ne!(first_temporary, second_temporary);
        drop((first_file, second_file));
        fs::remove_file(first_temporary).expect("first reservation cleanup");
        fs::remove_file(second_temporary).expect("second reservation cleanup");
    }

    #[test]
    #[ignore = "AC7 performance gate: run explicitly in an isolated process"]
    fn fixed_50000_item_feed_queries_have_30_sample_p95_below_200ms() {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/story-4-5");
        fs::create_dir_all(&directory).expect("evidence directory");
        let (evidence_path, temporary_path, mut temporary_file) =
            reserve_performance_evidence(&directory);
        let mut store = DemoStore::open_in_memory().expect("v10 store");
        let dataset_sha256 = seed_fixed_feed(&mut store, DATASET_SIZE);
        let default = default_feed_input(30);
        let mut combined = default.clone();
        combined.filters.track_ids = vec!["ai_agents".to_owned()];
        combined.filters.source_ids = vec![SOURCE_ID.to_owned()];
        combined.filters.time_window = IntelFeedTimeWindowV1::Last7d;
        combined.filters.importance = vec!["high".to_owned()];
        let measure = |input: &QueryIntelFeedInputV1| {
            (0..SAMPLE_COUNT)
                .map(|_| {
                    let started = Instant::now();
                    let page = store
                        .query_intel_feed_at(input, FIXED_AS_OF_MS)
                        .expect("performance page");
                    assert_eq!(page.items.len(), 30);
                    started.elapsed().as_micros()
                })
                .collect::<Vec<_>>()
        };
        let default_samples = measure(&default);
        let combined_samples = measure(&combined);
        let default_p95 = percentile_95(&default_samples);
        let combined_p95 = percentile_95(&combined_samples);
        assert!(default_p95 < 200_000, "default P95={default_p95}us");
        assert!(combined_p95 < 200_000, "combined P95={combined_p95}us");

        let configuration = read_configuration(&store.connection).expect("configuration");
        let explain = |input: &QueryIntelFeedInputV1| {
            let filters = normalize_filters(input).expect("normalized performance filters");
            let cutoff = cutoff_rfc3339(filters.time_window, FIXED_AS_OF_MS);
            let as_of_rfc3339 = crate::application::sources::unix_ms_to_rfc3339(FIXED_AS_OF_MS);
            explain_query_plan(
                &store.connection,
                &FeedQuery {
                    stream: input.stream,
                    filters: &filters,
                    rule_version: INTELLIGENCE_VALUE_RULE_VERSION,
                    configuration_revision: configuration.revision,
                    configuration_hash: &configuration.normalized_config_hash,
                    cutoff: cutoff.as_deref(),
                    as_of_rfc3339: &as_of_rfc3339,
                    as_of_ms: FIXED_AS_OF_MS,
                    after: None,
                    limit: input.limit,
                },
            )
            .expect("production query plan")
        };
        let default_plan = explain(&default);
        let combined_plan = explain(&combined);
        let candidate_sha256 = std::env::current_exe()
            .ok()
            .and_then(|path| fs::read(path).ok())
            .map(|bytes| digest_hex(&bytes));
        let evidence = json!({
            "story": "4.5",
            "dataset_size": DATASET_SIZE,
            "dataset_sha256": dataset_sha256,
            "sample_count_per_scenario": SAMPLE_COUNT,
            "threshold_ms": 200,
            "default_samples_us": default_samples,
            "default_p95_us": default_p95,
            "combined_samples_us": combined_samples,
            "combined_p95_us": combined_p95,
            "sqlite_version": rusqlite::version(),
            "platform": { "os": std::env::consts::OS, "arch": std::env::consts::ARCH },
            "hardware": std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "not_reported".to_owned()),
            "build_profile": option_env!("PROFILE").unwrap_or("project-local test profile"),
            "candidate_sha256": candidate_sha256,
            "source_sha256": digest_hex(concat!(
                include_str!("intel_feed.rs"),
                include_str!("../infrastructure/database/intel_feed_repository.rs"),
                include_str!("../contracts/dto/intel_feed.rs"),
            ).as_bytes()),
            "query_plan": { "default": default_plan, "combined": combined_plan },
        });
        serde_json::to_writer_pretty(&mut temporary_file, &evidence).expect("performance evidence");
        temporary_file
            .write_all(b"\n")
            .expect("performance evidence newline");
        temporary_file
            .sync_all()
            .expect("durable performance evidence");
        drop(temporary_file);
        fs::rename(&temporary_path, &evidence_path).expect("publish performance evidence");
        eprintln!(
            "Story 4.5 performance evidence: {}",
            evidence_path.display()
        );
    }
}
