//! Read-only, origin-isolated demo catalog.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::contracts::effects::normalize_rfc3339_utc;
use crate::contracts::errors::AppError;

const DEMO_FIXTURE: &str = include_str!("../../../../contracts/fixtures/demo/manifest-v1.json");
const MIN_SQLITE_VERSION: (u32, u32, u32) = (3, 53, 4);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DemoFixture {
    contract_version: u32,
    dataset_id: String,
    items: Vec<DemoItem>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataOrigin {
    Demo,
    Real,
}

impl DataOrigin {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Demo => "demo",
            Self::Real => "real",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DemoItem {
    pub id: String,
    pub data_origin: DataOrigin,
    pub publisher: String,
    pub title: String,
    pub track: String,
    pub summary: String,
    pub original_url: String,
    pub published_at: String,
    pub collected_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DemoCatalog {
    pub contract_version: u32,
    pub dataset_id: String,
    pub items: Vec<DemoItem>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DemoPage {
    pub contract_version: u32,
    pub dataset_id: String,
    pub items: Vec<DemoItem>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoSideEffect {
    Network,
    AiTask,
    Notification,
    ValidationMetric,
    RealDedupe,
}

pub trait DemoEffectPorts {
    fn network(&mut self);
    fn ai_task(&mut self);
    fn notification(&mut self);
    fn validation_metric(&mut self);
    fn real_dedupe(&mut self);
}

/// Applies the centralized origin policy before any external or real-data side effect.
pub fn dispatch_origin_side_effects(origin: DataOrigin, ports: &mut impl DemoEffectPorts) {
    if origin == DataOrigin::Demo {
        return;
    }
    ports.network();
    ports.ai_task();
    ports.notification();
    ports.validation_metric();
    ports.real_dedupe();
}

impl DemoSideEffect {
    pub const ALL: [Self; 5] = [
        Self::Network,
        Self::AiTask,
        Self::Notification,
        Self::ValidationMetric,
        Self::RealDedupe,
    ];

    #[must_use]
    pub const fn allowed_for_demo(self) -> bool {
        false
    }
}

pub struct DemoStore {
    connection: Connection,
}

impl DemoStore {
    /// Opens the device-local core database.
    ///
    /// # Errors
    /// Returns a redacted application error when `SQLite` cannot be opened or migrated.
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| demo_error("demo-storage-directory"))?;
        }
        let connection = Connection::open(path).map_err(|_| demo_error("demo-storage-open"))?;
        Self::from_connection(connection)
    }

    /// Opens an isolated in-memory database for deterministic tests.
    ///
    /// # Errors
    /// Returns a redacted application error when `SQLite` initialization fails.
    pub fn open_in_memory() -> Result<Self, AppError> {
        let connection =
            Connection::open_in_memory().map_err(|_| demo_error("demo-memory-open"))?;
        Self::from_connection(connection)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, AppError> {
        let version: String = connection
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .map_err(|_| demo_error("demo-sqlite-version"))?;
        if parse_version(&version).is_none_or(|value| value < MIN_SQLITE_VERSION) {
            return Err(demo_error("demo-sqlite-unsupported"));
        }
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA busy_timeout=2500; PRAGMA journal_mode=WAL;",
            )
            .map_err(|_| demo_error("demo-database-config"))?;
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("demo-migration-version"))?;
        if schema_version > 1 {
            return Err(demo_error("demo-migration-future"));
        }
        if schema_version == 0 {
            let transaction = connection
                .transaction()
                .map_err(|_| demo_error("demo-migration-transaction"))?;
            transaction
                .execute_batch(
                    "CREATE TABLE app_metadata (
                   key TEXT PRIMARY KEY NOT NULL,
                   value TEXT NOT NULL
                 );
                 CREATE TABLE intel_items (
                   id INTEGER PRIMARY KEY,
                   external_id TEXT NOT NULL,
                   data_origin TEXT NOT NULL CHECK(data_origin IN ('demo', 'real')),
                   publisher TEXT NOT NULL,
                   title TEXT NOT NULL,
                   track TEXT NOT NULL,
                   summary TEXT NOT NULL,
                   original_url TEXT NOT NULL,
                   published_at TEXT NOT NULL,
                   collected_at TEXT NOT NULL,
                   UNIQUE(data_origin, external_id)
                 );
                 CREATE VIRTUAL TABLE intel_items_fts USING fts5(
                   title, publisher, track, summary, content='intel_items', content_rowid='id', tokenize='trigram'
                 );
                 PRAGMA user_version=1;",
                )
                .map_err(|_| demo_error("demo-migration"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("demo-migration-commit"))?;
        }
        verify_database_contract(&connection)?;
        Ok(Self { connection })
    }

    /// Idempotently seeds and returns the shared demo dataset.
    ///
    /// # Errors
    /// Returns a stable redacted error for invalid embedded data or transaction failure.
    pub fn bootstrap(&mut self) -> Result<DemoCatalog, AppError> {
        let fixture = parse_fixture()?;
        let current: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key='demo_dataset_id'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| demo_error("demo-metadata-read"))?;
        if current.as_deref() != Some(&fixture.dataset_id) || !self.fixture_is_current(&fixture)? {
            let transaction = self
                .connection
                .transaction()
                .map_err(|_| demo_error("demo-seed-transaction"))?;
            transaction
                .execute(
                    "DELETE FROM intel_items_fts WHERE rowid IN
                     (SELECT id FROM intel_items WHERE data_origin='demo')",
                    [],
                )
                .and_then(|_| {
                    transaction.execute("DELETE FROM intel_items WHERE data_origin='demo'", [])
                })
                .map_err(|_| demo_error("demo-seed-clear"))?;
            for item in &fixture.items {
                transaction
                    .execute(
                        "INSERT INTO intel_items
                         (external_id,data_origin,publisher,title,track,summary,original_url,published_at,collected_at)
                         VALUES (?1,'demo',?2,?3,?4,?5,?6,?7,?8)",
                        params![item.id, item.publisher, item.title, item.track, item.summary, item.original_url, item.published_at, item.collected_at],
                    )
                    .map_err(|_| demo_error("demo-seed-item"))?;
                let row_id = transaction.last_insert_rowid();
                transaction
                    .execute(
                        "INSERT INTO intel_items_fts(rowid,title,publisher,track,summary) VALUES (?1,?2,?3,?4,?5)",
                        params![row_id, item.title, item.publisher, item.track, item.summary],
                    )
                    .map_err(|_| demo_error("demo-seed-search"))?;
            }
            transaction
                .execute(
                    "INSERT INTO app_metadata(key,value) VALUES ('demo_dataset_id',?1)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    [&fixture.dataset_id],
                )
                .map_err(|_| demo_error("demo-metadata-write"))?;
            transaction
                .execute(
                    "INSERT INTO app_metadata(key,value) VALUES ('demo_fixture_hash',?1)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    [fixture_fingerprint(DEMO_FIXTURE)],
                )
                .map_err(|_| demo_error("demo-hash-write"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("demo-seed-commit"))?;
        }
        self.catalog(&fixture.dataset_id, "", None)
    }

    /// Searches the embedded demo catalog without network access.
    ///
    /// # Errors
    /// Returns a redacted application error when the local query fails.
    pub fn search(&self, query: &str, track: Option<&str>) -> Result<DemoCatalog, AppError> {
        let dataset_id = self.dataset_id()?;
        self.catalog(&dataset_id, query.trim(), normalize_track(track))
    }

    /// Returns a deterministic page of the demo catalog.
    ///
    /// # Errors
    /// Returns a stable validation error for an invalid cursor/limit or a redacted query error.
    pub fn list_page(
        &self,
        track: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<DemoPage, AppError> {
        if !(1..=100).contains(&limit) {
            return Err(demo_error("demo-page-limit"));
        }
        let offset = match cursor {
            None => 0,
            Some(value) => value
                .strip_prefix("offset:")
                .and_then(|number| number.parse::<usize>().ok())
                .ok_or_else(|| demo_error("demo-page-cursor"))?,
        };
        let catalog = self.catalog(&self.dataset_id()?, "", normalize_track(track))?;
        if offset > catalog.items.len() {
            return Err(demo_error("demo-page-cursor"));
        }
        let end = offset
            .saturating_add(limit as usize)
            .min(catalog.items.len());
        let next_cursor = (end < catalog.items.len()).then(|| format!("offset:{end}"));
        Ok(DemoPage {
            contract_version: catalog.contract_version,
            dataset_id: catalog.dataset_id,
            items: catalog.items[offset..end].to_vec(),
            next_cursor,
        })
    }

    /// Returns one demo detail by its opaque demo ID.
    ///
    /// # Errors
    /// Returns a redacted application error when absent or when the local query fails.
    pub fn detail(&self, id: &str) -> Result<DemoItem, AppError> {
        self.connection
            .query_row(
                "SELECT external_id,data_origin,publisher,title,track,summary,original_url,published_at,collected_at
                 FROM intel_items WHERE data_origin='demo' AND external_id=?1",
                [id],
                map_item,
            )
            .optional()
            .map_err(|_| demo_error("demo-detail-query"))?
            .ok_or_else(|| demo_error("demo-detail-not-found"))
    }

    #[must_use]
    pub fn identity_key(&self, origin: DataOrigin, external_id: &str) -> String {
        format!("{}:{external_id}", origin.as_str())
    }

    fn dataset_id(&self) -> Result<String, AppError> {
        self.connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key='demo_dataset_id'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| demo_error("demo-not-bootstrapped"))
    }

    fn fixture_is_current(&self, fixture: &DemoFixture) -> Result<bool, AppError> {
        let stored_hash: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key='demo_fixture_hash'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| demo_error("demo-hash-read"))?;
        if stored_hash.as_deref() != Some(&fixture_fingerprint(DEMO_FIXTURE)) {
            return Ok(false);
        }
        let mut expected_ids = fixture
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        expected_ids.sort_unstable();
        let mut statement = self
            .connection
            .prepare(
                "SELECT external_id FROM intel_items WHERE data_origin='demo' ORDER BY external_id",
            )
            .map_err(|_| demo_error("demo-integrity-prepare"))?;
        let actual_ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| demo_error("demo-integrity-query"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| demo_error("demo-integrity-row"))?;
        if actual_ids != expected_ids {
            return Ok(false);
        }
        let indexed_count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM intel_items_fts f JOIN intel_items i ON i.id=f.rowid
                 WHERE i.data_origin='demo'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| demo_error("demo-integrity-search"))?;
        Ok(usize::try_from(indexed_count).ok() == Some(fixture.items.len()))
    }

    fn catalog(
        &self,
        dataset_id: &str,
        query: &str,
        track: Option<&str>,
    ) -> Result<DemoCatalog, AppError> {
        let mut items = Vec::new();
        if query.is_empty() {
            let mut statement = self.connection.prepare(
                "SELECT external_id,data_origin,publisher,title,track,summary,original_url,published_at,collected_at
                 FROM intel_items WHERE data_origin='demo' AND (?1 IS NULL OR track=?1)
                 ORDER BY published_at DESC, external_id ASC",
            ).map_err(|_| demo_error("demo-list-prepare"))?;
            let rows = statement
                .query_map([track], map_item)
                .map_err(|_| demo_error("demo-list-query"))?;
            for row in rows {
                items.push(row.map_err(|_| demo_error("demo-list-row"))?);
            }
        } else if query.chars().count() < 3 {
            let pattern = format!("%{}%", escape_like(query));
            let mut statement = self.connection.prepare(
                "SELECT external_id,data_origin,publisher,title,track,summary,original_url,published_at,collected_at
                 FROM intel_items WHERE data_origin='demo'
                 AND (title LIKE ?1 ESCAPE '\\' OR publisher LIKE ?1 ESCAPE '\\'
                      OR track LIKE ?1 ESCAPE '\\' OR summary LIKE ?1 ESCAPE '\\')
                 AND (?2 IS NULL OR track=?2)
                 ORDER BY published_at DESC, external_id ASC",
            ).map_err(|_| demo_error("demo-short-search-prepare"))?;
            let rows = statement
                .query_map(params![pattern, track], map_item)
                .map_err(|_| demo_error("demo-short-search-query"))?;
            for row in rows {
                items.push(row.map_err(|_| demo_error("demo-short-search-row"))?);
            }
        } else {
            let literal_query = format!("\"{}\"", query.replace('"', "\"\""));
            let mut statement = self.connection.prepare(
                "SELECT i.external_id,i.data_origin,i.publisher,i.title,i.track,i.summary,i.original_url,i.published_at,i.collected_at
                 FROM intel_items i JOIN intel_items_fts f ON f.rowid=i.id
                 WHERE i.data_origin='demo' AND intel_items_fts MATCH ?1 AND (?2 IS NULL OR i.track=?2)
                 ORDER BY i.published_at DESC, i.external_id ASC",
            ).map_err(|_| demo_error("demo-search-prepare"))?;
            let rows = statement
                .query_map(params![literal_query, track], map_item)
                .map_err(|_| demo_error("demo-search-query"))?;
            for row in rows {
                items.push(row.map_err(|_| demo_error("demo-search-row"))?);
            }
        }
        Ok(DemoCatalog {
            contract_version: 1,
            dataset_id: dataset_id.to_owned(),
            items,
        })
    }
}

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<DemoItem> {
    let data_origin: String = row.get(1)?;
    if data_origin != DataOrigin::Demo.as_str() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(DemoItem {
        id: row.get(0)?,
        data_origin: DataOrigin::Demo,
        publisher: row.get(2)?,
        title: row.get(3)?,
        track: row.get(4)?,
        summary: row.get(5)?,
        original_url: row.get(6)?,
        published_at: row.get(7)?,
        collected_at: row.get(8)?,
    })
}

fn parse_fixture() -> Result<DemoFixture, AppError> {
    validate_fixture_json(DEMO_FIXTURE)
}

fn validate_fixture_json(value: &str) -> Result<DemoFixture, AppError> {
    let fixture: DemoFixture =
        serde_json::from_str(value).map_err(|_| demo_error("demo-fixture-json"))?;
    if fixture.contract_version != 1 || fixture.dataset_id != "demo-v1" || fixture.items.len() != 3
    {
        return Err(demo_error("demo-fixture-contract"));
    }
    let mut ids = std::collections::BTreeSet::new();
    for item in &fixture.items {
        if item.data_origin != DataOrigin::Demo
            || !item.id.starts_with("demo:")
            || item.id.len() <= "demo:".len()
            || item.id.len() > 128
            || !item.id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
            || !ids.insert(&item.id)
        {
            return Err(demo_error("demo-fixture-origin"));
        }
        let text_fields = [&item.publisher, &item.title, &item.track, &item.summary];
        let url_rest = item.original_url.strip_prefix("https://");
        let valid_url = url_rest.is_some_and(|rest| {
            !rest.is_empty() && !rest.starts_with('/') && !rest.chars().any(char::is_whitespace)
        });
        let valid_published = normalize_rfc3339_utc(&item.published_at)
            .is_some_and(|canonical| canonical == item.published_at);
        let valid_collected = normalize_rfc3339_utc(&item.collected_at)
            .is_some_and(|canonical| canonical == item.collected_at);
        if text_fields
            .iter()
            .any(|field| field.is_empty() || field.trim() != field.as_str())
            || !valid_url
            || !valid_published
            || !valid_collected
            || item.published_at > item.collected_at
        {
            return Err(demo_error("demo-fixture-field"));
        }
    }
    Ok(fixture)
}

/// Validates a candidate demo fixture with the exact production rules.
///
/// # Errors
/// Returns a stable error when the JSON structure or any semantic field is invalid.
pub fn validate_demo_fixture(value: &str) -> Result<(), AppError> {
    validate_fixture_json(value).map(|_| ())
}

#[must_use]
pub fn demo_fixture_fingerprint() -> String {
    fixture_fingerprint(DEMO_FIXTURE)
}

fn fixture_fingerprint(value: &str) -> String {
    let hash = value
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("{hash:016x}")
}

fn normalize_track(track: Option<&str>) -> Option<&str> {
    track.map(str::trim).filter(|value| !value.is_empty())
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn verify_database_contract(connection: &Connection) -> Result<(), AppError> {
    let foreign_keys: u32 = connection
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .map_err(|_| demo_error("demo-foreign-keys-read"))?;
    let busy_timeout: u32 = connection
        .pragma_query_value(None, "busy_timeout", |row| row.get(0))
        .map_err(|_| demo_error("demo-busy-timeout-read"))?;
    let journal_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|_| demo_error("demo-journal-mode-read"))?;
    if foreign_keys != 1
        || busy_timeout < 2_500
        || !matches!(journal_mode.as_str(), "wal" | "memory")
    {
        return Err(demo_error("demo-database-config-invalid"));
    }
    let required_objects: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE name IN ('app_metadata','intel_items','intel_items_fts')
             AND type IN ('table','view')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| demo_error("demo-schema-read"))?;
    if required_objects != 3 {
        return Err(demo_error("demo-schema-invalid"));
    }
    connection
        .execute(
            "INSERT INTO intel_items_fts(intel_items_fts, rank) VALUES('integrity-check', 1)",
            [],
        )
        .map_err(|_| demo_error("demo-fts-integrity"))?;
    Ok(())
}

fn parse_version(value: &str) -> Option<(u32, u32, u32)> {
    let mut parts = value.split('.').map(str::parse::<u32>);
    Some((
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ))
}

fn demo_error(boundary: &'static str) -> AppError {
    AppError::internal_generated(boundary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reseeding_demo_rows_preserves_real_rows_and_search_projection() {
        let mut store = DemoStore::open_in_memory().expect("demo store");
        store.bootstrap().expect("initial demo seed");
        store
            .connection
            .execute(
                "INSERT INTO intel_items
                 (external_id,data_origin,publisher,title,track,summary,original_url,published_at,collected_at)
                 VALUES ('shared-id','real','Real Publisher','Real Title','real','real summary','https://example.com/real','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                [],
            )
            .expect("insert synthetic real row");
        let real_row_id = store.connection.last_insert_rowid();
        store
            .connection
            .execute(
                "INSERT INTO intel_items_fts(rowid,title,publisher,track,summary)
                 VALUES (?1,'Real Title','Real Publisher','real','real summary')",
                [real_row_id],
            )
            .expect("insert synthetic real search row");
        store
            .connection
            .execute(
                "UPDATE app_metadata SET value='outdated' WHERE key='demo_dataset_id'",
                [],
            )
            .expect("force a versioned reseed");

        store.bootstrap().expect("versioned demo reseed");

        let real_count: u32 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM intel_items WHERE data_origin='real'",
                [],
                |row| row.get(0),
            )
            .expect("real row remains");
        let real_search_count: u32 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM intel_items_fts WHERE intel_items_fts MATCH 'Real'",
                [],
                |row| row.get(0),
            )
            .expect("real search row remains");
        assert_eq!(real_count, 1);
        assert_eq!(real_search_count, 1);
    }

    #[test]
    fn strict_fixture_validation_rejects_unknown_and_invalid_fields() {
        for invalid in [
            DEMO_FIXTURE.replacen("\"dataset_id\"", "\"unknown\":1,\"dataset_id\"", 1),
            DEMO_FIXTURE.replacen("https://openai.com/", "https:///", 1),
            DEMO_FIXTURE.replacen("2026-07-01T08:00:00Z", "Z", 1),
            DEMO_FIXTURE.replacen("demo:openai-agents-sdk-001", "demo:", 1),
        ] {
            assert!(validate_demo_fixture(&invalid).is_err());
        }
    }
}
