//! Read-only, origin-isolated demo catalog.

use std::fmt::Write as _;
use std::path::Path;

pub(crate) use rusqlite::params as sql_params;
use rusqlite::params;
pub(crate) use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::contracts::effects::normalize_rfc3339_utc;
use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::intel::normalize_rss_candidate;
use crate::domain::sources::RawSourceCandidate;

pub(crate) type SqlError = rusqlite::Error;
pub(crate) type SqlResult<T> = rusqlite::Result<T>;

const DEMO_FIXTURE: &str = include_str!("../../../../contracts/fixtures/demo/manifest-v1.json");
const MIN_SQLITE_VERSION: (u32, u32, u32) = (3, 53, 4);
const MAX_CURSOR_LENGTH: usize = 1_024;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DemoFixture {
    contract_version: u32,
    dataset_id: String,
    items: Vec<DemoFixtureItem>,
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
    pub importance: Importance,
    pub ai_status: AiStatus,
    pub published_at: String,
    pub collected_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    Low,
    Medium,
    High,
}

impl Importance {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl From<crate::domain::rules::intelligence_value::ImportanceV1> for Importance {
    fn from(value: crate::domain::rules::intelligence_value::ImportanceV1) -> Self {
        match value {
            crate::domain::rules::intelligence_value::ImportanceV1::Low => Self::Low,
            crate::domain::rules::intelligence_value::ImportanceV1::Medium => Self::Medium,
            crate::domain::rules::intelligence_value::ImportanceV1::High => Self::High,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiStatus {
    Generated,
    Waiting,
    Failed,
    Unavailable,
}

impl AiStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Waiting => "waiting",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityStatus {
    Available,
    Unavailable,
}

impl AvailabilityStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DemoProvenance {
    pub source_kind: String,
    pub publisher: String,
    pub author: Option<String>,
    pub original_title: String,
    pub original_url: String,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub first_discovered_at: String,
    pub last_updated_at: String,
    pub availability_status: AvailabilityStatus,
    pub deterministic_association_basis: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DemoFixtureItem {
    id: String,
    data_origin: DataOrigin,
    publisher: String,
    title: String,
    track: String,
    summary: String,
    published_at: String,
    collected_at: String,
    what_happened: String,
    why_it_matters: String,
    possible_impact: String,
    importance: Importance,
    facts: Vec<String>,
    rule_reasons: Vec<String>,
    ai_content: String,
    ai_confidence_percent: u8,
    ai_status: AiStatus,
    provenance: DemoProvenance,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DemoEvidenceDetail {
    pub contract_version: u32,
    pub dataset_id: String,
    pub id: String,
    pub data_origin: DataOrigin,
    pub publisher: String,
    pub title: String,
    pub track: String,
    pub summary: String,
    pub original_url: String,
    pub published_at: String,
    pub collected_at: String,
    pub what_happened: String,
    pub why_it_matters: String,
    pub possible_impact: String,
    pub importance: Importance,
    pub facts: Vec<String>,
    pub rule_reasons: Vec<String>,
    pub ai_content: String,
    pub ai_confidence_percent: u8,
    pub ai_status: AiStatus,
    pub provenance: DemoProvenance,
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
    pub(crate) connection: Connection,
    pub(crate) configuration_receipts:
        crate::domain::rules::configuration_validation::ReceiptRegistry,
    pub(crate) feed_cursor_secret: [u8; 32],
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

    #[allow(clippy::too_many_lines)] // The transactional schema definition and its sole legacy migration stay auditable together.
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
        if schema_version > 10 {
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
                   importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                   ai_status TEXT NOT NULL CHECK(ai_status IN ('generated','waiting','failed','unavailable')),
                   published_at TEXT NOT NULL,
                   collected_at TEXT NOT NULL,
                   UNIQUE(data_origin, external_id)
                 );
                 CREATE TABLE intel_contents (
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                   what_happened TEXT NOT NULL,
                   facts_json TEXT NOT NULL
                 );
                 CREATE TABLE item_provenance (
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                   source_kind TEXT NOT NULL,
                   publisher TEXT NOT NULL,
                   author TEXT,
                   original_title TEXT NOT NULL,
                   original_url TEXT NOT NULL,
                   published_at TEXT,
                   collected_at TEXT NOT NULL,
                   first_discovered_at TEXT NOT NULL,
                   last_updated_at TEXT NOT NULL,
                   availability_status TEXT NOT NULL CHECK(availability_status IN ('available','unavailable')),
                   deterministic_association_basis TEXT
                 );
                 CREATE TABLE rule_evaluations (
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                   why_it_matters TEXT NOT NULL,
                   possible_impact TEXT NOT NULL,
                   importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                   reasons_json TEXT NOT NULL
                 );
                 CREATE TABLE analysis_results (
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                   content TEXT NOT NULL,
                   confidence_percent INTEGER NOT NULL CHECK(confidence_percent BETWEEN 0 AND 100),
                   status TEXT NOT NULL CHECK(status IN ('generated','waiting','failed','unavailable'))
                 );
                 CREATE VIRTUAL TABLE intel_items_fts USING fts5(
                   title, publisher, track, summary, content='intel_items', content_rowid='id', tokenize='trigram'
                 );
                 PRAGMA user_version=2;",
                )
                .map_err(|_| demo_error("demo-migration"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("demo-migration-commit"))?;
        }
        if schema_version == 1 {
            let transaction = connection
                .transaction()
                .map_err(|_| demo_error("demo-migration-v2-transaction"))?;
            transaction
                .execute_batch(
                    "DROP TABLE intel_items_fts;
                     ALTER TABLE intel_items RENAME TO intel_items_v1;
                     CREATE TABLE intel_items (
                       id INTEGER PRIMARY KEY,
                       external_id TEXT NOT NULL,
                       data_origin TEXT NOT NULL CHECK(data_origin IN ('demo', 'real')),
                       publisher TEXT NOT NULL,
                       title TEXT NOT NULL,
                       track TEXT NOT NULL,
                       summary TEXT NOT NULL,
                       original_url TEXT NOT NULL,
                       importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                       ai_status TEXT NOT NULL CHECK(ai_status IN ('generated','waiting','failed','unavailable')),
                       published_at TEXT NOT NULL,
                       collected_at TEXT NOT NULL,
                       UNIQUE(data_origin, external_id)
                     );
                     INSERT INTO intel_items
                       (id,external_id,data_origin,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at)
                       SELECT id,external_id,data_origin,publisher,title,track,summary,original_url,'medium','unavailable',published_at,collected_at
                       FROM intel_items_v1;
                     CREATE VIRTUAL TABLE intel_items_fts USING fts5(
                       title, publisher, track, summary, content='intel_items', content_rowid='id', tokenize='trigram'
                     );
                     INSERT INTO intel_items_fts(rowid,title,publisher,track,summary)
                       SELECT id,title,publisher,track,summary FROM intel_items;
                     CREATE TABLE intel_contents (
                       item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                       what_happened TEXT NOT NULL, facts_json TEXT NOT NULL
                     );
                     CREATE TABLE item_provenance (
                       item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                       source_kind TEXT NOT NULL, publisher TEXT NOT NULL, author TEXT,
                       original_title TEXT NOT NULL, original_url TEXT NOT NULL, published_at TEXT,
                       collected_at TEXT NOT NULL, first_discovered_at TEXT NOT NULL,
                       last_updated_at TEXT NOT NULL,
                       availability_status TEXT NOT NULL CHECK(availability_status IN ('available','unavailable')),
                       deterministic_association_basis TEXT
                     );
                     CREATE TABLE rule_evaluations (
                       item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                       why_it_matters TEXT NOT NULL, possible_impact TEXT NOT NULL,
                       importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                       reasons_json TEXT NOT NULL
                     );
                     CREATE TABLE analysis_results (
                       item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                       content TEXT NOT NULL,
                       confidence_percent INTEGER NOT NULL CHECK(confidence_percent BETWEEN 0 AND 100),
                       status TEXT NOT NULL CHECK(status IN ('generated','waiting','failed','unavailable'))
                     );
                     INSERT INTO intel_contents(item_id,what_happened,facts_json)
                       SELECT id,summary,'[\"Legacy v1 summary\"]' FROM intel_items_v1;
                     INSERT INTO item_provenance
                       (item_id,source_kind,publisher,author,original_title,original_url,published_at,
                        collected_at,first_discovered_at,last_updated_at,availability_status,
                        deterministic_association_basis)
                       SELECT id,'legacy_v1',publisher,NULL,title,original_url,published_at,
                              collected_at,published_at,collected_at,'available','legacy-v1-migration'
                       FROM intel_items_v1;
                     INSERT INTO rule_evaluations
                       (item_id,why_it_matters,possible_impact,importance,reasons_json)
                       SELECT id,summary,summary,'medium','[\"Legacy v1 migration\"]' FROM intel_items_v1;
                     INSERT INTO analysis_results(item_id,content,confidence_percent,status)
                       SELECT id,'AI analysis unavailable after v1 migration',0,'unavailable'
                       FROM intel_items_v1;
                     DROP TABLE intel_items_v1;
                     PRAGMA user_version=2;",
                )
                .map_err(|_| demo_error("demo-migration-v2"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("demo-migration-v2-commit"))?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("setup-migration-version"))?;
        if schema_version == 2 {
            let transaction = connection
                .transaction()
                .map_err(|_| super::setup::setup_error(ErrorCode::MigrationSetup, "transaction"))?;
            transaction
                .execute_batch(
                    "CREATE TABLE settings_metadata (
                       key TEXT PRIMARY KEY NOT NULL,
                       value TEXT NOT NULL
                     );
                     CREATE TABLE setup_progress (
                       step_id TEXT PRIMARY KEY NOT NULL CHECK(step_id IN ('tracks','source_examples','refresh_cadence','ai_data_disclosure')),
                       status TEXT NOT NULL CHECK(status IN ('not_started','in_progress','skipped','partially_completed','completed')),
                       revision INTEGER NOT NULL CHECK(revision >= 1),
                       saved_fields_version INTEGER,
                       updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms >= 1)
                     );
                     CREATE TABLE setup_idempotency (
                       idempotency_key TEXT PRIMARY KEY NOT NULL,
                       request_fingerprint TEXT NOT NULL,
                       response_revision INTEGER NOT NULL CHECK(response_revision >= 1)
                     );
                     PRAGMA user_version=3;",
                )
                .map_err(|_| super::setup::setup_error(ErrorCode::MigrationSetup, "schema-v3"))?;
            transaction
                .commit()
                .map_err(|_| super::setup::setup_error(ErrorCode::MigrationSetup, "commit-v3"))?;
        }
        let has_original_url = connection
            .prepare("PRAGMA table_info(intel_items)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| demo_error("demo-schema-columns"))?
            .iter()
            .any(|column| column == "original_url");
        if !has_original_url {
            connection
                .execute(
                    "ALTER TABLE intel_items ADD COLUMN original_url TEXT NOT NULL DEFAULT ''",
                    [],
                )
                .map_err(|_| demo_error("demo-v2-original-url"))?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("configuration-migration-version"))?;
        if schema_version == 3 {
            let setup_has_response_json =
                table_has_column(&connection, "setup_idempotency", "response_json")?;
            let transaction = connection
                .transaction()
                .map_err(|_| demo_error("configuration-migration-transaction"))?;
            transaction
                .execute_batch(
                    "CREATE TABLE configuration_versions (
                       version INTEGER PRIMARY KEY CHECK(version>=1),
                       validator_version TEXT NOT NULL,
                       normalized_config_hash TEXT NOT NULL,
                       configuration_json TEXT NOT NULL,
                       created_at_ms INTEGER NOT NULL
                     );
                     CREATE TABLE configuration_current (
                       singleton_id INTEGER PRIMARY KEY CHECK(singleton_id=1),
                       version INTEGER NOT NULL UNIQUE REFERENCES configuration_versions(version)
                     );
                     CREATE TABLE configuration_idempotency (
                       idempotency_key TEXT PRIMARY KEY,
                       request_fingerprint TEXT NOT NULL,
                       response_version INTEGER NOT NULL REFERENCES configuration_versions(version),
                       response_json TEXT NOT NULL
                     );",
                )
                .map_err(|_| demo_error("configuration-migration-v4"))?;
            if !setup_has_response_json {
                transaction
                    .execute(
                        "ALTER TABLE setup_idempotency ADD COLUMN response_json TEXT NOT NULL DEFAULT ''",
                        [],
                    )
                    .map_err(|_| demo_error("configuration-migration-v4-setup-response"))?;
            }
            super::configuration::create_initial_configuration(&transaction)?;
            transaction
                .pragma_update(None, "user_version", 4_u32)
                .map_err(|_| demo_error("configuration-migration-v4-version"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("configuration-migration-v4-commit"))?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("source-migration-version"))?;
        if schema_version == 4 {
            let transaction = connection
                .transaction()
                .map_err(|_| demo_error("source-migration-transaction"))?;
            transaction
                .execute_batch(
                    "CREATE TABLE sources (
                       source_id TEXT PRIMARY KEY NOT NULL,
                       configuration_version INTEGER NOT NULL REFERENCES configuration_versions(version),
                       source_kind TEXT NOT NULL CHECK(source_kind='rss_atom'),
                       canonical_url TEXT NOT NULL UNIQUE,
                       enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
                       revision INTEGER NOT NULL CHECK(revision>=1),
                       etag TEXT,
                       last_modified TEXT,
                       adapter_cursor TEXT,
                       last_attempt_at_ms INTEGER,
                       last_success_at_ms INTEGER,
                       status TEXT NOT NULL CHECK(status IN ('ready','error','retry_wait')),
                       consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK(consecutive_failures>=0),
                       retryability TEXT NOT NULL CHECK(retryability IN ('never','manual','automatic','after')),
                       next_allowed_at_ms INTEGER,
                       error_code TEXT,
                       created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=1),
                       updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=1)
                     );
                     CREATE INDEX sources_configuration_version_idx ON sources(configuration_version);
                     CREATE TABLE source_entry_checkpoints (
                       source_id TEXT NOT NULL REFERENCES sources(source_id) ON DELETE CASCADE,
                       stable_external_id TEXT NOT NULL,
                       content_hash TEXT NOT NULL,
                       first_seen_at_ms INTEGER NOT NULL CHECK(first_seen_at_ms>=1),
                       last_seen_at_ms INTEGER NOT NULL CHECK(last_seen_at_ms>=1),
                       PRIMARY KEY(source_id, stable_external_id)
                     );
                     CREATE TABLE source_idempotency (
                       idempotency_key TEXT PRIMARY KEY NOT NULL,
                       request_fingerprint TEXT NOT NULL,
                       response_json TEXT NOT NULL
                     );
                     PRAGMA user_version=5;",
                )
                .map_err(|_| demo_error("source-migration-v5"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("source-migration-v5-commit"))?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("sync-migration-version"))?;
        if schema_version == 5 {
            let transaction = connection
                .transaction()
                .map_err(|_| demo_error("sync-migration-transaction"))?;
            transaction
                .execute_batch(
                    "CREATE TABLE jobs (
                       task_id TEXT PRIMARY KEY NOT NULL,
                       kind TEXT NOT NULL CHECK(kind='rss_atom_sync'),
                       target_kind TEXT NOT NULL CHECK(target_kind IN ('all_enabled_rss_atom','source_id')),
                       target_source_id TEXT REFERENCES sources(source_id),
                       state TEXT NOT NULL CHECK(state IN ('queued','running','retry_wait','succeeded','partially_succeeded','failed','cancelled')),
                       revision INTEGER NOT NULL CHECK(revision>=1),
                       idempotency_key TEXT NOT NULL UNIQUE,
                       request_fingerprint TEXT NOT NULL,
                       foreground_budget_ms INTEGER NOT NULL CHECK(foreground_budget_ms=30000),
                       created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=1),
                       started_at_ms INTEGER,
                       finished_at_ms INTEGER,
                       updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=1),
                       error_summary TEXT,
                       CHECK((target_kind='all_enabled_rss_atom' AND target_source_id IS NULL) OR
                             (target_kind='source_id' AND target_source_id IS NOT NULL)),
                       CHECK(error_summary IS NULL OR length(error_summary)<=128)
                     );
                     CREATE TABLE job_source_states (
                       task_id TEXT NOT NULL REFERENCES jobs(task_id) ON DELETE CASCADE,
                       source_id TEXT NOT NULL REFERENCES sources(source_id),
                       source_revision INTEGER NOT NULL CHECK(source_revision>=1),
                       state TEXT NOT NULL CHECK(state IN ('queued','running','retry_wait','succeeded','failed','cancelled')),
                       last_success_at_ms INTEGER,
                       error_code TEXT,
                       next_allowed_at_ms INTEGER,
                       updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=1),
                       PRIMARY KEY(task_id,source_id)
                     );
                     CREATE UNIQUE INDEX job_source_active_unique
                       ON job_source_states(source_id)
                       WHERE state IN ('queued','running');
                     CREATE INDEX jobs_updated_idx ON jobs(updated_at_ms DESC,task_id);
                     PRAGMA user_version=6;",
                )
                .map_err(|_| demo_error("sync-migration-v6"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("sync-migration-v6-commit"))?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("sync-result-migration-version"))?;
        if schema_version == 6 {
            let transaction = connection
                .transaction()
                .map_err(|_| demo_error("sync-result-migration-transaction"))?;
            transaction
                .execute_batch(
                    "CREATE TABLE sync_runs (
                       sync_run_id TEXT PRIMARY KEY NOT NULL,
                       task_id TEXT NOT NULL UNIQUE REFERENCES jobs(task_id) ON DELETE CASCADE,
                       scope TEXT NOT NULL CHECK(scope IN ('all_enabled_rss_atom','source_id')),
                       outcome TEXT CHECK(outcome IN ('succeeded_with_results','succeeded_zero_results','partially_succeeded','failed')),
                       started_at_ms INTEGER NOT NULL CHECK(started_at_ms>=1),
                       finished_at_ms INTEGER,
                       inserted_count INTEGER NOT NULL DEFAULT 0 CHECK(inserted_count>=0),
                       updated_count INTEGER NOT NULL DEFAULT 0 CHECK(updated_count>=0),
                       skipped_count INTEGER NOT NULL DEFAULT 0 CHECK(skipped_count>=0),
                       failed_count INTEGER NOT NULL DEFAULT 0 CHECK(failed_count>=0)
                     );
                     CREATE TABLE sync_source_results (
                       sync_run_id TEXT NOT NULL REFERENCES sync_runs(sync_run_id) ON DELETE CASCADE,
                       source_id TEXT NOT NULL REFERENCES sources(source_id),
                       source_revision INTEGER NOT NULL CHECK(source_revision>=1),
                       source_kind TEXT NOT NULL CHECK(source_kind='rss_atom'),
                       publisher TEXT NOT NULL CHECK(length(publisher)>=1),
                       status TEXT NOT NULL CHECK(status IN ('queued','running','retry_wait','succeeded','failed','cancelled')),
                       inserted_count INTEGER NOT NULL DEFAULT 0 CHECK(inserted_count>=0),
                       updated_count INTEGER NOT NULL DEFAULT 0 CHECK(updated_count>=0),
                       skipped_count INTEGER NOT NULL DEFAULT 0 CHECK(skipped_count>=0),
                       failed_count INTEGER NOT NULL DEFAULT 0 CHECK(failed_count>=0),
                       error_code TEXT,
                       PRIMARY KEY(sync_run_id,source_id)
                     );
                     CREATE TABLE sync_result_items (
                       result_item_id TEXT PRIMARY KEY NOT NULL,
                       sync_run_id TEXT NOT NULL REFERENCES sync_runs(sync_run_id) ON DELETE CASCADE,
                       source_id TEXT NOT NULL,
                       stable_external_id TEXT NOT NULL,
                       source_kind TEXT NOT NULL CHECK(source_kind='rss_atom'),
                       publisher TEXT NOT NULL CHECK(length(publisher)>=1),
                       original_title TEXT NOT NULL CHECK(length(original_title)>=1),
                       published_at TEXT,
                       collected_at TEXT NOT NULL,
                       original_url TEXT NOT NULL,
                       disposition TEXT NOT NULL CHECK(disposition IN ('inserted','updated')),
                       FOREIGN KEY(source_id,stable_external_id) REFERENCES source_entry_checkpoints(source_id,stable_external_id),
                       CONSTRAINT uq_sync_result_items_run_source_external UNIQUE(sync_run_id,source_id,stable_external_id)
                     );
                     CREATE INDEX sync_result_items_page_idx
                       ON sync_result_items(sync_run_id,source_id,collected_at,result_item_id);
                     PRAGMA user_version=7;",
                )
                .map_err(|_| demo_error("sync-result-migration-v7"))?;
            transaction
                .commit()
                .map_err(|_| demo_error("sync-result-migration-v7-commit"))?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("intel-migration-version"))?;
        if schema_version == 7 {
            migrate_to_v8(&mut connection)?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("association-migration-version"))?;
        if schema_version == 8 {
            migrate_to_v9(&mut connection)?;
        }
        let schema_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| demo_error("rule-migration-version"))?;
        if schema_version == 9 {
            migrate_to_v10(&mut connection)?;
        }
        super::sync::recover_interrupted_sync_jobs(&connection)?;
        let mut feed_cursor_secret = [0_u8; 32];
        getrandom::fill(&mut feed_cursor_secret).map_err(|_| demo_error("feed-cursor-secret"))?;
        let store = Self {
            connection,
            configuration_receipts:
                crate::domain::rules::configuration_validation::ReceiptRegistry::default(),
            feed_cursor_secret,
        };
        verify_database_contract(&store.connection)?;
        Ok(store)
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
                let intel_item_id = crate::domain::intel::derive_intel_item_id("demo", &item.id);
                transaction
                    .execute(
                        "INSERT INTO intel_items
                         (intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at)
                         VALUES (?1,?2,'demo',NULL,NULL,NULL,NULL,1,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                        params![intel_item_id.as_str(), item.id, item.publisher, item.title, item.track, item.summary, item.provenance.original_url, item.importance.as_str(), item.ai_status.as_str(), item.published_at, item.collected_at],
                    )
                    .map_err(|_| demo_error("demo-seed-item"))?;
                let row_id = transaction.last_insert_rowid();
                transaction
                    .execute(
                        "INSERT INTO intel_items_fts(rowid,title,publisher,track,summary) VALUES (?1,?2,?3,?4,?5)",
                        params![row_id, item.title, item.publisher, item.track, item.summary],
                    )
                    .map_err(|_| demo_error("demo-seed-search"))?;
                let facts_json = json_text(&item.facts)?;
                let rule_reasons_json = json_text(&item.rule_reasons)?;
                transaction
                    .execute(
                        "INSERT INTO intel_contents(item_id,what_happened,facts_json,source_summary,content_hash,content_state) VALUES (?1,?2,?3,NULL,NULL,'demo')",
                        params![row_id, item.what_happened, facts_json],
                    )
                    .map_err(|_| demo_error("demo-seed-content"))?;
                transaction
                    .execute(
                        "INSERT INTO rule_evaluations(item_id,why_it_matters,possible_impact,importance,reasons_json)
                         VALUES (?1,?2,?3,?4,?5)",
                        params![row_id, item.why_it_matters, item.possible_impact, item.importance.as_str(), rule_reasons_json],
                    )
                    .map_err(|_| demo_error("demo-seed-evidence"))?;
                transaction
                    .execute(
                        "INSERT INTO analysis_results(item_id,content,confidence_percent,status) VALUES (?1,?2,?3,?4)",
                        params![row_id, item.ai_content, item.ai_confidence_percent, item.ai_status.as_str()],
                    )
                    .map_err(|_| demo_error("demo-seed-analysis"))?;
                let provenance = &item.provenance;
                transaction
                    .execute(
                        "INSERT INTO item_provenance
                         (provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,author_availability,original_title,original_url,published_at,collected_at,first_discovered_at,last_updated_at,availability_status,warnings_json,content_hash,deterministic_association_basis)
                         VALUES (?1,?2,NULL,NULL,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'[]',NULL,?14)",
                        params![format!("prov:demo:{row_id:024x}"), row_id, provenance.source_kind, provenance.publisher, provenance.author, if provenance.author.is_some() { "available" } else { "unavailable" }, provenance.original_title, provenance.original_url, provenance.published_at, provenance.collected_at, provenance.first_discovered_at, provenance.last_updated_at, provenance.availability_status.as_str(), provenance.deterministic_association_basis],
                    )
                    .map_err(|_| demo_error("demo-seed-provenance"))?;
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
        let dataset_id = self.dataset_id()?;
        let track = normalize_track(track);
        let after = cursor
            .map(|value| decode_cursor(value, &dataset_id, track))
            .transpose()?;
        if let Some((published_at, id)) = &after {
            let cursor_exists: bool = self
                .connection
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1 FROM intel_items
                       WHERE data_origin='demo' AND external_id=?1 AND published_at=?2
                         AND (?3 IS NULL OR track=?3)
                     )",
                    params![id, published_at, track],
                    |row| row.get(0),
                )
                .map_err(|_| demo_error("demo-page-cursor-query"))?;
            if !cursor_exists {
                return Err(demo_error("demo-page-cursor"));
            }
        }
        let after_published_at = after.as_ref().map(|value| value.0.as_str());
        let after_id = after.as_ref().map(|value| value.1.as_str());
        let mut statement = self
            .connection
            .prepare(
                "SELECT external_id,data_origin,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at
                 FROM intel_items
                 WHERE data_origin='demo' AND (?1 IS NULL OR track=?1)
                 AND (?2 IS NULL OR published_at < ?2 OR (published_at=?2 AND external_id>?3))
                 ORDER BY published_at DESC, external_id ASC LIMIT ?4",
            )
            .map_err(|_| demo_error("demo-page-prepare"))?;
        let rows = statement
            .query_map(
                params![track, after_published_at, after_id, i64::from(limit) + 1],
                map_item,
            )
            .map_err(|_| demo_error("demo-page-query"))?;
        let mut items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| demo_error("demo-page-row"))?;
        let has_more = items.len() > limit as usize;
        if has_more {
            items.truncate(limit as usize);
        }
        let next_cursor = has_more
            .then(|| {
                items
                    .last()
                    .map(|item| encode_cursor(&dataset_id, track, &item.published_at, &item.id))
            })
            .flatten();
        Ok(DemoPage {
            contract_version: 1,
            dataset_id,
            items,
            next_cursor,
        })
    }

    /// Returns one demo detail by its opaque demo ID.
    ///
    /// # Errors
    /// Returns a redacted application error when absent or when the local query fails.
    pub fn detail(&self, id: &str) -> Result<DemoEvidenceDetail, AppError> {
        if id.len() > 128 || !id.starts_with("demo:") {
            return Err(demo_error("demo-detail-id"));
        }
        let dataset_id = self.dataset_id()?;
        self.connection
            .query_row(
                "SELECT i.external_id,i.data_origin,i.publisher,i.title,i.track,i.summary,i.original_url,
                        i.published_at,i.collected_at,c.what_happened,c.facts_json,
                        r.why_it_matters,r.possible_impact,r.importance,r.reasons_json,
                        a.content,a.confidence_percent,a.status,
                        p.source_kind,p.publisher,p.author,p.original_title,p.original_url,p.published_at,
                        p.collected_at,p.first_discovered_at,p.last_updated_at,p.availability_status,
                        p.deterministic_association_basis
                 FROM intel_items i
                 JOIN intel_contents c ON c.item_id=i.id
                 JOIN rule_evaluations r ON r.item_id=i.id
                 JOIN analysis_results a ON a.item_id=i.id
                 JOIN item_provenance p ON p.item_id=i.id
                 WHERE i.data_origin='demo' AND i.external_id=?1",
                [id],
                |row| map_detail(row, &dataset_id),
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
        let evidence_count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM intel_items i
                 JOIN intel_contents c ON c.item_id=i.id
                 JOIN item_provenance p ON p.item_id=i.id
                 JOIN rule_evaluations r ON r.item_id=i.id
                 JOIN analysis_results a ON a.item_id=i.id
                 WHERE i.data_origin='demo'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| demo_error("demo-integrity-evidence"))?;
        Ok(
            usize::try_from(indexed_count).ok() == Some(fixture.items.len())
                && usize::try_from(evidence_count).ok() == Some(fixture.items.len()),
        )
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
                "SELECT external_id,data_origin,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at
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
                "SELECT external_id,data_origin,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at
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
                "SELECT i.external_id,i.data_origin,i.publisher,i.title,i.track,i.summary,i.original_url,i.importance,i.ai_status,i.published_at,i.collected_at
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
        importance: parse_importance(&row.get::<_, String>(7)?)?,
        ai_status: parse_ai_status(&row.get::<_, String>(8)?)?,
        published_at: row.get(9)?,
        collected_at: row.get(10)?,
    })
}

fn map_detail(row: &rusqlite::Row<'_>, dataset_id: &str) -> rusqlite::Result<DemoEvidenceDetail> {
    let data_origin: String = row.get(1)?;
    if data_origin != DataOrigin::Demo.as_str() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(DemoEvidenceDetail {
        contract_version: 1,
        dataset_id: dataset_id.to_owned(),
        id: row.get(0)?,
        data_origin: DataOrigin::Demo,
        publisher: row.get(2)?,
        title: row.get(3)?,
        track: row.get(4)?,
        summary: row.get(5)?,
        original_url: row.get(6)?,
        published_at: row.get(7)?,
        collected_at: row.get(8)?,
        what_happened: row.get(9)?,
        facts: parse_json_list(&row.get::<_, String>(10)?)?,
        why_it_matters: row.get(11)?,
        possible_impact: row.get(12)?,
        importance: parse_importance(&row.get::<_, String>(13)?)?,
        rule_reasons: parse_json_list(&row.get::<_, String>(14)?)?,
        ai_content: row.get(15)?,
        ai_confidence_percent: row.get(16)?,
        ai_status: parse_ai_status(&row.get::<_, String>(17)?)?,
        provenance: DemoProvenance {
            source_kind: row.get(18)?,
            publisher: row.get(19)?,
            author: row.get(20)?,
            original_title: row.get(21)?,
            original_url: row.get(22)?,
            published_at: row.get(23)?,
            collected_at: row.get(24)?,
            first_discovered_at: row.get(25)?,
            last_updated_at: row.get(26)?,
            availability_status: parse_availability(&row.get::<_, String>(27)?)?,
            deterministic_association_basis: row.get(28)?,
        },
    })
}

fn parse_importance(value: &str) -> rusqlite::Result<Importance> {
    match value {
        "low" => Ok(Importance::Low),
        "medium" => Ok(Importance::Medium),
        "high" => Ok(Importance::High),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_ai_status(value: &str) -> rusqlite::Result<AiStatus> {
    match value {
        "generated" => Ok(AiStatus::Generated),
        "waiting" => Ok(AiStatus::Waiting),
        "failed" => Ok(AiStatus::Failed),
        "unavailable" => Ok(AiStatus::Unavailable),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_availability(value: &str) -> rusqlite::Result<AvailabilityStatus> {
    match value {
        "available" => Ok(AvailabilityStatus::Available),
        "unavailable" => Ok(AvailabilityStatus::Unavailable),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_json_list(value: &str) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(value).map_err(|_| rusqlite::Error::InvalidQuery)
}

fn json_text(value: &[String]) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|_| demo_error("demo-fixture-json-encode"))
}

fn encode_cursor(dataset_id: &str, track: Option<&str>, published_at: &str, id: &str) -> String {
    let track = track.map_or_else(|| "-".to_owned(), encode_hex);
    let payload = format!(
        "v1:{}:{track}:{}:{}",
        encode_hex(dataset_id),
        encode_hex(published_at),
        encode_hex(id)
    );
    format!("{payload}:{}", fixture_fingerprint(&payload))
}

fn decode_cursor(
    value: &str,
    expected_dataset_id: &str,
    expected_track: Option<&str>,
) -> Result<(String, String), AppError> {
    if value.len() > MAX_CURSOR_LENGTH {
        return Err(demo_error("demo-page-cursor"));
    }
    let mut parts = value.split(':');
    if parts.next() != Some("v1") {
        return Err(demo_error("demo-page-cursor"));
    }
    let dataset_id = parts
        .next()
        .and_then(decode_hex)
        .ok_or_else(|| demo_error("demo-page-cursor"))?;
    let track_part = parts.next().ok_or_else(|| demo_error("demo-page-cursor"))?;
    let track = if track_part == "-" {
        None
    } else {
        Some(decode_hex(track_part).ok_or_else(|| demo_error("demo-page-cursor"))?)
    };
    let published_at = parts
        .next()
        .and_then(decode_hex)
        .ok_or_else(|| demo_error("demo-page-cursor"))?;
    let id = parts
        .next()
        .and_then(decode_hex)
        .ok_or_else(|| demo_error("demo-page-cursor"))?;
    let checksum = parts.next().ok_or_else(|| demo_error("demo-page-cursor"))?;
    let payload = value
        .rsplit_once(':')
        .map(|value| value.0)
        .ok_or_else(|| demo_error("demo-page-cursor"))?;
    if parts.next().is_some()
        || checksum != fixture_fingerprint(payload)
        || dataset_id != expected_dataset_id
        || track.as_deref() != expected_track
        || normalize_rfc3339_utc(&published_at).as_deref() != Some(published_at.as_str())
        || !id.starts_with("demo:")
        || id.len() > 128
    {
        return Err(demo_error("demo-page-cursor"));
    }
    Ok((published_at, id))
}

fn encode_hex(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

fn decode_hex(value: &str) -> Option<String> {
    if value.is_empty() || !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
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
        let provenance = &item.provenance;
        let text_fields = [
            &item.publisher,
            &item.title,
            &item.track,
            &item.summary,
            &item.what_happened,
            &item.why_it_matters,
            &item.possible_impact,
            &item.ai_content,
            &provenance.source_kind,
            &provenance.publisher,
            &provenance.original_title,
        ];
        let url_rest = provenance.original_url.strip_prefix("https://");
        let valid_url = url_rest.is_some_and(|rest| {
            !rest.is_empty() && !rest.starts_with('/') && !rest.chars().any(char::is_whitespace)
        });
        let valid_published = normalize_rfc3339_utc(&item.published_at)
            .is_some_and(|canonical| canonical == item.published_at);
        let valid_collected = normalize_rfc3339_utc(&item.collected_at)
            .is_some_and(|canonical| canonical == item.collected_at);
        let provenance_times = [
            provenance.published_at.as_deref(),
            Some(provenance.collected_at.as_str()),
            Some(provenance.first_discovered_at.as_str()),
            Some(provenance.last_updated_at.as_str()),
        ];
        if text_fields
            .iter()
            .any(|field| field.is_empty() || field.trim() != field.as_str())
            || item.facts.is_empty()
            || item.rule_reasons.is_empty()
            || item
                .facts
                .iter()
                .chain(&item.rule_reasons)
                .any(|value| value.is_empty() || value.trim() != value)
            || item.ai_confidence_percent > 100
            || !valid_url
            || !valid_published
            || !valid_collected
            || item.published_at > item.collected_at
            || provenance_times
                .into_iter()
                .flatten()
                .any(|value| normalize_rfc3339_utc(value).as_deref() != Some(value))
            || provenance.collected_at != item.collected_at
            || provenance.publisher != item.publisher
            || provenance
                .author
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.trim() != value)
            || provenance
                .deterministic_association_basis
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.trim() != value)
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

#[derive(Debug)]
struct LegacyIntelRow {
    id: i64,
    external_id: String,
    data_origin: String,
    publisher: String,
    title: String,
    track: String,
    summary: String,
    original_url: String,
    importance: String,
    ai_status: String,
    published_at: String,
    collected_at: String,
}

#[derive(Debug)]
struct LegacyResultIdentity {
    source_id: String,
    stable_external_id: String,
    publisher: String,
    original_title: String,
    published_at: Option<String>,
    collected_at: String,
    original_url: String,
    content_hash: String,
    first_seen_at_ms: i64,
    last_seen_at_ms: i64,
}

fn validate_legacy_result_identity(result: &LegacyResultIdentity) -> Result<(), AppError> {
    if result.first_seen_at_ms < 0 || result.last_seen_at_ms < result.first_seen_at_ms {
        return Err(demo_error("intel-migration-result-lifecycle"));
    }
    let candidate = RawSourceCandidate {
        stable_external_id: result.stable_external_id.clone(),
        title: Some(result.original_title.clone()),
        original_url: Some(result.original_url.clone()),
        author: None,
        summary: None,
        published_at: result.published_at.clone(),
        updated_at: None,
        content_hash: result.content_hash.clone(),
        warnings: Vec::new(),
    };
    let normalized = normalize_rss_candidate(
        &result.source_id,
        &result.publisher,
        &result.collected_at,
        &candidate,
    )
    .map_err(|_| demo_error("intel-migration-result-contract"))?;
    if normalized.stable_external_id != result.stable_external_id
        || normalized.publisher != result.publisher
        || normalized.original_title != result.original_title
        || normalized.original_url != result.original_url
        || normalized.published_at != result.published_at
        || normalized.collected_at != result.collected_at
        || normalized.content_hash != result.content_hash
    {
        return Err(demo_error("intel-migration-result-noncanonical"));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn migrate_to_v8(connection: &mut Connection) -> Result<(), AppError> {
    if table_has_column(connection, "intel_items", "intel_item_id")? {
        return migrate_v8_result_extension(connection);
    }
    let legacy_items = connection
        .prepare(
            "SELECT id,external_id,data_origin,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at FROM intel_items ORDER BY id",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok(LegacyIntelRow {
                        id: row.get(0)?,
                        external_id: row.get(1)?,
                        data_origin: row.get(2)?,
                        publisher: row.get(3)?,
                        title: row.get(4)?,
                        track: row.get(5)?,
                        summary: row.get(6)?,
                        original_url: row.get(7)?,
                        importance: row.get(8)?,
                        ai_status: row.get(9)?,
                        published_at: row.get(10)?,
                        collected_at: row.get(11)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| demo_error("intel-migration-read-items"))?;
    let legacy_results = connection
        .prepare(
            "SELECT r.source_id,r.stable_external_id,r.publisher,r.original_title,r.published_at,r.collected_at,r.original_url,c.content_hash,c.first_seen_at_ms,c.last_seen_at_ms
             FROM sync_result_items r
             JOIN source_entry_checkpoints c ON c.source_id=r.source_id AND c.stable_external_id=r.stable_external_id
             WHERE r.result_item_id=(
               SELECT newest.result_item_id FROM sync_result_items newest
               WHERE newest.source_id=r.source_id AND newest.stable_external_id=r.stable_external_id
               ORDER BY newest.collected_at DESC,newest.result_item_id DESC LIMIT 1)
             ORDER BY r.source_id,r.stable_external_id",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok(LegacyResultIdentity {
                        source_id: row.get(0)?,
                        stable_external_id: row.get(1)?,
                        publisher: row.get(2)?,
                        original_title: row.get(3)?,
                        published_at: row.get(4)?,
                        collected_at: row.get(5)?,
                        original_url: row.get(6)?,
                        content_hash: row.get(7)?,
                        first_seen_at_ms: row.get(8)?,
                        last_seen_at_ms: row.get(9)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| demo_error("intel-migration-read-results"))?;
    for result in &legacy_results {
        validate_legacy_result_identity(result)?;
    }

    connection
        .execute_batch("PRAGMA foreign_keys=OFF;")
        .map_err(|_| demo_error("intel-migration-disable-fk"))?;
    let migration = (|| -> Result<(), AppError> {
        let transaction = connection
            .transaction()
            .map_err(|_| demo_error("intel-migration-transaction"))?;
        transaction
            .execute_batch(
                "CREATE TABLE intel_items_fts_v7_backup AS
                   SELECT rowid,title,publisher,track,summary FROM intel_items_fts;
                 DROP TABLE intel_items_fts;
                 CREATE TABLE intel_items_v8 (
                   id INTEGER PRIMARY KEY,
                   intel_item_id TEXT NOT NULL UNIQUE,
                   external_id TEXT NOT NULL,
                   data_origin TEXT NOT NULL CHECK(data_origin IN ('demo','real')),
                   source_id TEXT REFERENCES sources(source_id),
                   source_kind TEXT CHECK(source_kind IS NULL OR source_kind IN ('rss_atom','legacy')),
                   stable_external_id TEXT,
                   content_hash TEXT,
                   revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>=1),
                   publisher TEXT NOT NULL CHECK(length(publisher) BETWEEN 1 AND 253),
                   title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 1024),
                   track TEXT,
                   summary TEXT,
                   original_url TEXT NOT NULL,
                   importance TEXT CHECK(importance IS NULL OR importance IN ('low','medium','high')),
                   ai_status TEXT CHECK(ai_status IS NULL OR ai_status IN ('generated','waiting','failed','unavailable')),
                   published_at TEXT,
                   collected_at TEXT NOT NULL,
                   UNIQUE(data_origin,external_id),
                   UNIQUE(source_id,stable_external_id),
                   CHECK(data_origin='demo' OR (source_kind IS NOT NULL AND content_hash IS NOT NULL))
                 );
                 CREATE TABLE intel_contents_v8 (
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items_v8(id) ON DELETE CASCADE,
                   what_happened TEXT,
                   facts_json TEXT,
                   source_summary TEXT,
                   content_hash TEXT,
                   content_state TEXT NOT NULL CHECK(content_state IN ('demo','metadata_only'))
                 );
                 CREATE TABLE item_provenance_v8 (
                   provenance_id TEXT PRIMARY KEY NOT NULL,
                   item_id INTEGER NOT NULL UNIQUE REFERENCES intel_items_v8(id) ON DELETE CASCADE,
                   source_id TEXT REFERENCES sources(source_id),
                   stable_external_id TEXT,
                   source_kind TEXT NOT NULL,
                   publisher TEXT NOT NULL,
                   author TEXT,
                   author_availability TEXT NOT NULL CHECK(author_availability IN ('available','unavailable','unknown_legacy')),
                   original_title TEXT NOT NULL,
                   original_url TEXT NOT NULL,
                   published_at TEXT,
                   collected_at TEXT NOT NULL,
                   first_discovered_at TEXT NOT NULL,
                   last_updated_at TEXT NOT NULL,
                   availability_status TEXT NOT NULL CHECK(availability_status IN ('available','unavailable','unknown_legacy')),
                   warnings_json TEXT NOT NULL,
                   content_hash TEXT,
                   deterministic_association_basis TEXT,
                   UNIQUE(item_id,source_id,stable_external_id)
                 );
                 CREATE TABLE rule_evaluations_v8 (
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items_v8(id) ON DELETE CASCADE,
                   why_it_matters TEXT NOT NULL,possible_impact TEXT NOT NULL,
                   importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                   reasons_json TEXT NOT NULL
                 );
                 CREATE TABLE analysis_results_v8 (
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items_v8(id) ON DELETE CASCADE,
                   content TEXT NOT NULL,
                   confidence_percent INTEGER NOT NULL CHECK(confidence_percent BETWEEN 0 AND 100),
                   status TEXT NOT NULL CHECK(status IN ('generated','waiting','failed','unavailable'))
                 );
                 INSERT INTO intel_contents_v8(item_id,what_happened,facts_json,source_summary,content_hash,content_state)
                   SELECT c.item_id,c.what_happened,c.facts_json,NULL,NULL,
                          CASE WHEN i.data_origin='demo' THEN 'demo' ELSE 'metadata_only' END
                   FROM intel_contents c JOIN intel_items i ON i.id=c.item_id;
                 INSERT INTO item_provenance_v8
                   (provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,author_availability,original_title,original_url,published_at,collected_at,first_discovered_at,last_updated_at,availability_status,warnings_json,content_hash,deterministic_association_basis)
                   SELECT printf('prov:legacy:%024x',p.item_id),p.item_id,NULL,NULL,p.source_kind,p.publisher,p.author,
                          CASE WHEN i.data_origin='demo' THEN CASE WHEN p.author IS NULL THEN 'unavailable' ELSE 'available' END ELSE 'unknown_legacy' END,
                          p.original_title,p.original_url,p.published_at,p.collected_at,p.first_discovered_at,p.last_updated_at,
                          CASE WHEN i.data_origin='demo' THEN p.availability_status ELSE 'unknown_legacy' END,
                          '[]',NULL,p.deterministic_association_basis
                   FROM item_provenance p JOIN intel_items i ON i.id=p.item_id;
                 INSERT INTO rule_evaluations_v8 SELECT * FROM rule_evaluations;
                 INSERT INTO analysis_results_v8 SELECT * FROM analysis_results;",
            )
            .map_err(|_| demo_error("intel-migration-create"))?;

        for item in &legacy_items {
            let source_kind = (item.data_origin == "real").then_some("legacy");
            let intel_id = crate::domain::intel::derive_intel_item_id(
                source_kind.unwrap_or("demo"),
                &item.external_id,
            );
            transaction
                .execute(
                    "INSERT INTO intel_items_v8
                     (id,intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at)
                     VALUES(?1,?2,?3,?4,NULL,?5,NULL,?6,1,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                    params![
                        item.id,
                        intel_id.as_str(),
                        item.external_id,
                        item.data_origin,
                        source_kind,
                        (item.data_origin == "real").then_some("0".repeat(64)),
                        item.publisher,
                        item.title,
                        item.track,
                        item.summary,
                        item.original_url,
                        item.importance,
                        item.ai_status,
                        item.published_at,
                        item.collected_at,
                    ],
                )
                .map_err(|_| demo_error("intel-migration-copy-item"))?;
        }

        for result in &legacy_results {
            let canonical = crate::domain::intel::canonical_external_id(
                &result.source_id,
                &result.stable_external_id,
            );
            let intel_id = crate::domain::intel::derive_intel_item_id("rss_atom", &canonical);
            transaction
                .execute(
                    "INSERT INTO intel_items_v8
                     (intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at)
                     VALUES(?1,?2,'real',?3,'rss_atom',?4,?5,1,?6,?7,NULL,NULL,?8,NULL,NULL,?9,?10)
                     ON CONFLICT(source_id,stable_external_id) DO NOTHING",
                    params![
                        intel_id.as_str(),canonical,result.source_id,result.stable_external_id,
                        result.content_hash,result.publisher,result.original_title,result.original_url,
                        result.published_at,result.collected_at,
                    ],
                )
                .map_err(|_| demo_error("intel-migration-backfill-item"))?;
            let item_id: i64 = transaction
                .query_row(
                    "SELECT id FROM intel_items_v8 WHERE source_id=?1 AND stable_external_id=?2",
                    params![result.source_id, result.stable_external_id],
                    |row| row.get(0),
                )
                .map_err(|_| demo_error("intel-migration-backfill-id"))?;
            let first = super::sync::unix_ms_to_rfc3339(
                u64::try_from(result.first_seen_at_ms)
                    .map_err(|_| demo_error("intel-migration-first-time"))?,
            );
            let last = super::sync::unix_ms_to_rfc3339(
                u64::try_from(result.last_seen_at_ms)
                    .map_err(|_| demo_error("intel-migration-last-time"))?,
            );
            transaction
                .execute(
                    "INSERT OR IGNORE INTO intel_contents_v8(item_id,what_happened,facts_json,source_summary,content_hash,content_state)
                     VALUES(?1,NULL,NULL,NULL,?2,'metadata_only')",
                    params![item_id, result.content_hash],
                )
                .and_then(|_| {
                    transaction.execute(
                        "INSERT OR IGNORE INTO item_provenance_v8
                         (provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,author_availability,original_title,original_url,published_at,collected_at,first_discovered_at,last_updated_at,availability_status,warnings_json,content_hash,deterministic_association_basis)
                         VALUES(?1,?2,?3,?4,'rss_atom',?5,NULL,'unknown_legacy',?6,?7,?8,?9,?10,?11,'unknown_legacy','[]',?12,'source_kind+canonical_external_id')",
                        params![
                            format!("prov:{}", intel_id.as_str()),item_id,result.source_id,
                            result.stable_external_id,result.publisher,result.original_title,
                            result.original_url,result.published_at,result.collected_at,first,last,result.content_hash,
                        ],
                    )
                })
                .map_err(|_| demo_error("intel-migration-backfill-provenance"))?;
        }

        transaction
            .execute_batch(
                "CREATE TABLE sync_result_items_v8 (
                   result_item_id TEXT PRIMARY KEY NOT NULL,
                   sync_run_id TEXT NOT NULL REFERENCES sync_runs(sync_run_id) ON DELETE CASCADE,
                   source_id TEXT NOT NULL,
                   stable_external_id TEXT NOT NULL,
                   intel_item_id TEXT NOT NULL REFERENCES intel_items_v8(intel_item_id) ON DELETE RESTRICT,
                   source_kind TEXT NOT NULL CHECK(source_kind='rss_atom'),
                   publisher TEXT NOT NULL CHECK(length(publisher)>=1),
                   original_title TEXT NOT NULL CHECK(length(original_title)>=1),
                   published_at TEXT,collected_at TEXT NOT NULL,original_url TEXT NOT NULL,
                   disposition TEXT NOT NULL CHECK(disposition IN ('inserted','updated')),
                   FOREIGN KEY(source_id,stable_external_id) REFERENCES source_entry_checkpoints(source_id,stable_external_id),
                   CONSTRAINT uq_sync_result_items_run_source_external UNIQUE(sync_run_id,source_id,stable_external_id),
                   CONSTRAINT uq_sync_result_items_sync_run_id_intel_item_id UNIQUE(sync_run_id,intel_item_id)
                 );
                 INSERT INTO sync_result_items_v8
                   (result_item_id,sync_run_id,source_id,stable_external_id,intel_item_id,source_kind,publisher,original_title,published_at,collected_at,original_url,disposition)
                   SELECT r.result_item_id,r.sync_run_id,r.source_id,r.stable_external_id,i.intel_item_id,r.source_kind,r.publisher,r.original_title,r.published_at,r.collected_at,r.original_url,r.disposition
                   FROM sync_result_items r JOIN intel_items_v8 i ON i.source_id=r.source_id AND i.stable_external_id=r.stable_external_id;
                 CREATE TABLE sync_result_item_failures (
                   failure_id TEXT PRIMARY KEY NOT NULL,
                   sync_run_id TEXT NOT NULL REFERENCES sync_runs(sync_run_id) ON DELETE CASCADE,
                   source_id TEXT NOT NULL REFERENCES sources(source_id),
                   candidate_ref TEXT NOT NULL,field_name TEXT NOT NULL,reason_code TEXT NOT NULL,
                   observed_at TEXT NOT NULL,
                   UNIQUE(sync_run_id,source_id,candidate_ref)
                 );
                 DROP TABLE sync_result_items;
                 DROP TABLE intel_contents;
                 DROP TABLE item_provenance;
                 DROP TABLE rule_evaluations;
                 DROP TABLE analysis_results;
                 DROP TABLE intel_items;
                 ALTER TABLE intel_items_v8 RENAME TO intel_items;
                 ALTER TABLE intel_contents_v8 RENAME TO intel_contents;
                 ALTER TABLE item_provenance_v8 RENAME TO item_provenance;
                 ALTER TABLE rule_evaluations_v8 RENAME TO rule_evaluations;
                 ALTER TABLE analysis_results_v8 RENAME TO analysis_results;
                 ALTER TABLE sync_result_items_v8 RENAME TO sync_result_items;
                 CREATE INDEX sync_result_items_page_idx ON sync_result_items(sync_run_id,source_id,collected_at,result_item_id);
                 CREATE VIRTUAL TABLE intel_items_fts USING fts5(
                   title,publisher,track,summary,content='intel_items',content_rowid='id',tokenize='trigram');
                 INSERT INTO intel_items_fts(rowid,title,publisher,track,summary)
                   SELECT b.rowid,b.title,b.publisher,b.track,b.summary FROM intel_items_fts_v7_backup b JOIN intel_items i ON i.id=b.rowid;
                 DROP TABLE intel_items_fts_v7_backup;
                 PRAGMA user_version=8;",
            )
            .map_err(|_| demo_error("intel-migration-replace"))?;
        transaction
            .commit()
            .map_err(|_| demo_error("intel-migration-commit"))?;
        Ok(())
    })();
    connection
        .execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|_| demo_error("intel-migration-enable-fk"))?;
    migration
}

fn migrate_v8_result_extension(connection: &mut Connection) -> Result<(), AppError> {
    connection
        .execute_batch("PRAGMA foreign_keys=OFF;")
        .map_err(|_| demo_error("intel-extension-disable-fk"))?;
    let migration = (|| -> Result<(), AppError> {
        let transaction = connection
            .transaction()
            .map_err(|_| demo_error("intel-extension-transaction"))?;
        if !table_has_column(&transaction, "sync_result_items", "intel_item_id")? {
            transaction
                .execute_batch(
                    "ALTER TABLE sync_result_items RENAME TO sync_result_items_v7;
                     CREATE TABLE sync_result_items (
                       result_item_id TEXT PRIMARY KEY NOT NULL,
                       sync_run_id TEXT NOT NULL REFERENCES sync_runs(sync_run_id) ON DELETE CASCADE,
                       source_id TEXT NOT NULL,stable_external_id TEXT NOT NULL,
                       intel_item_id TEXT NOT NULL REFERENCES intel_items(intel_item_id) ON DELETE RESTRICT,
                       source_kind TEXT NOT NULL CHECK(source_kind='rss_atom'),
                       publisher TEXT NOT NULL CHECK(length(publisher)>=1),
                       original_title TEXT NOT NULL CHECK(length(original_title)>=1),
                       published_at TEXT,collected_at TEXT NOT NULL,original_url TEXT NOT NULL,
                       disposition TEXT NOT NULL CHECK(disposition IN ('inserted','updated')),
                       FOREIGN KEY(source_id,stable_external_id) REFERENCES source_entry_checkpoints(source_id,stable_external_id),
                       CONSTRAINT uq_sync_result_items_run_source_external UNIQUE(sync_run_id,source_id,stable_external_id),
                       CONSTRAINT uq_sync_result_items_sync_run_id_intel_item_id UNIQUE(sync_run_id,intel_item_id)
                     );
                     INSERT INTO sync_result_items
                       (result_item_id,sync_run_id,source_id,stable_external_id,intel_item_id,source_kind,publisher,original_title,published_at,collected_at,original_url,disposition)
                       SELECT r.result_item_id,r.sync_run_id,r.source_id,r.stable_external_id,i.intel_item_id,r.source_kind,r.publisher,r.original_title,r.published_at,r.collected_at,r.original_url,r.disposition
                       FROM sync_result_items_v7 r JOIN intel_items i ON i.source_id=r.source_id AND i.stable_external_id=r.stable_external_id;
                     DROP TABLE sync_result_items_v7;
                     CREATE INDEX sync_result_items_page_idx ON sync_result_items(sync_run_id,source_id,collected_at,result_item_id);",
                )
                .map_err(|_| demo_error("intel-extension-result"))?;
        }
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS sync_result_item_failures (
                   failure_id TEXT PRIMARY KEY NOT NULL,
                   sync_run_id TEXT NOT NULL REFERENCES sync_runs(sync_run_id) ON DELETE CASCADE,
                   source_id TEXT NOT NULL REFERENCES sources(source_id),
                   candidate_ref TEXT NOT NULL,field_name TEXT NOT NULL,reason_code TEXT NOT NULL,
                   observed_at TEXT NOT NULL,UNIQUE(sync_run_id,source_id,candidate_ref)
                 );
                 PRAGMA user_version=8;",
            )
            .map_err(|_| demo_error("intel-extension-schema"))?;
        transaction
            .commit()
            .map_err(|_| demo_error("intel-extension-commit"))?;
        Ok(())
    })();
    connection
        .execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|_| demo_error("intel-extension-enable-fk"))?;
    migration
}

fn migrate_to_v9(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(|_| demo_error("association-migration-transaction"))?;
    transaction
        .execute_batch(
            "CREATE TABLE intel_associations (
               association_id TEXT PRIMARY KEY NOT NULL,
               relation_type TEXT NOT NULL CHECK(relation_type='same_event'),
               evidence_basis TEXT NOT NULL CHECK(evidence_basis='normalized_original_url'),
               basis_version INTEGER NOT NULL CHECK(basis_version=1),
               basis_hash TEXT NOT NULL CHECK(length(basis_hash)=64),
               first_observed_at TEXT NOT NULL,
               last_observed_at TEXT NOT NULL,
               CONSTRAINT uq_intel_association_evidence UNIQUE(relation_type,evidence_basis,basis_version,basis_hash),
               CHECK(first_observed_at<=last_observed_at)
             );
             CREATE TABLE intel_association_members (
               association_id TEXT NOT NULL REFERENCES intel_associations(association_id) ON DELETE CASCADE,
               item_id INTEGER NOT NULL REFERENCES intel_items(id) ON DELETE RESTRICT,
               first_observed_at TEXT NOT NULL,
               last_observed_at TEXT NOT NULL,
               PRIMARY KEY(association_id,item_id),
               CONSTRAINT uq_intel_association_member_item UNIQUE(item_id),
               CHECK(first_observed_at<=last_observed_at)
             );
             CREATE INDEX intel_association_members_item_idx ON intel_association_members(item_id);",
        )
        .map_err(|_| demo_error("association-migration-schema"))?;
    crate::infrastructure::database::association_repository::backfill_associations(&transaction)
        .map_err(|_| demo_error("association-migration-backfill"))?;
    transaction
        .pragma_update(None, "user_version", 9_u32)
        .map_err(|_| demo_error("association-migration-version-write"))?;
    transaction
        .commit()
        .map_err(|_| demo_error("association-migration-commit"))?;
    Ok(())
}

fn migrate_to_v10(connection: &mut Connection) -> Result<(), AppError> {
    const EXTENSION_COLUMNS: &[&str] = &[
        "rule_version",
        "configuration_revision",
        "configuration_hash",
        "fact_revision",
        "evaluated_at_ms",
        "score",
        "stream_disposition",
        "matched_tracks_json",
        "factor_results_json",
        "filter_reasons_json",
        "ai_status",
    ];
    let mut extension_count = 0;
    for column in EXTENSION_COLUMNS {
        extension_count += usize::from(table_has_column(connection, "rule_evaluations", column)?);
    }
    if extension_count != 0 && extension_count != EXTENSION_COLUMNS.len() {
        return Err(demo_error("rule-migration-partial-schema"));
    }
    let already_extended = extension_count == EXTENSION_COLUMNS.len();
    if !already_extended {
        let invalid_legacy_rules: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM rule_evaluations r JOIN intel_items i ON i.id=r.item_id
                 WHERE trim(r.why_it_matters)='' OR trim(r.possible_impact)=''
                    OR json_valid(r.reasons_json)=0",
                [],
                |row| row.get(0),
            )
            .map_err(|_| demo_error("rule-migration-legacy-validation"))?;
        if invalid_legacy_rules != 0 {
            return Err(demo_error("rule-migration-legacy-invalid"));
        }
    }
    if already_extended {
        let index_exists: bool = connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='index' AND name='rule_evaluations_stream_idx'",
                [],
                |_| Ok(true),
            )
            .optional()
            .map_err(|_| demo_error("rule-migration-index-read"))?
            .unwrap_or(false);
        if !index_exists {
            return Err(demo_error("rule-migration-partial-schema"));
        }
    }
    let transaction = connection
        .transaction()
        .map_err(|_| demo_error("rule-migration-transaction"))?;
    if !already_extended {
        transaction
            .execute_batch(
            "ALTER TABLE rule_evaluations ADD COLUMN rule_version TEXT;
             ALTER TABLE rule_evaluations ADD COLUMN configuration_revision INTEGER CHECK(configuration_revision>=1);
             ALTER TABLE rule_evaluations ADD COLUMN configuration_hash TEXT CHECK(configuration_hash IS NULL OR length(configuration_hash)=64);
             ALTER TABLE rule_evaluations ADD COLUMN fact_revision INTEGER CHECK(fact_revision>=1);
             ALTER TABLE rule_evaluations ADD COLUMN evaluated_at_ms INTEGER CHECK(evaluated_at_ms>=1);
             ALTER TABLE rule_evaluations ADD COLUMN score INTEGER CHECK(score BETWEEN 0 AND 100);
             ALTER TABLE rule_evaluations ADD COLUMN stream_disposition TEXT CHECK(stream_disposition IN ('high_value','ordinary_candidate'));
             ALTER TABLE rule_evaluations ADD COLUMN matched_tracks_json TEXT;
             ALTER TABLE rule_evaluations ADD COLUMN factor_results_json TEXT;
             ALTER TABLE rule_evaluations ADD COLUMN filter_reasons_json TEXT;
             ALTER TABLE rule_evaluations ADD COLUMN ai_status TEXT CHECK(ai_status IN ('unavailable'));
             CREATE INDEX rule_evaluations_stream_idx
               ON rule_evaluations(stream_disposition,score DESC,item_id)
               WHERE rule_version IS NOT NULL;",
            )
            .map_err(|_| demo_error("rule-migration-schema"))?;
    }
    let current = super::configuration::read_configuration(&transaction)?;
    crate::infrastructure::database::rule_evaluation_repository::reevaluate_all(
        &transaction,
        &current,
        super::configuration::now_ms()?,
    )
    .map_err(|_| demo_error("rule-migration-backfill"))?;
    transaction
        .pragma_update(None, "user_version", 10_u32)
        .map_err(|_| demo_error("rule-migration-version-write"))?;
    verify_database_contract(&transaction)?;
    transaction
        .commit()
        .map_err(|_| demo_error("rule-migration-commit"))?;
    Ok(())
}

fn table_has_column(
    connection: &Connection,
    table: &str,
    expected_column: &str,
) -> Result<bool, AppError> {
    let columns = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| demo_error("schema-column-read"))?;
    Ok(columns.iter().any(|column| column == expected_column))
}

#[allow(clippy::too_many_lines)] // One ordered startup audit keeps every required object, column, FK, and current-row invariant visible.
fn verify_database_contract(connection: &Connection) -> Result<(), AppError> {
    let schema_version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| demo_error("configuration-schema-version"))?;
    let foreign_keys: u32 = connection
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .map_err(|_| demo_error("demo-foreign-keys-read"))?;
    let busy_timeout: u32 = connection
        .pragma_query_value(None, "busy_timeout", |row| row.get(0))
        .map_err(|_| demo_error("demo-busy-timeout-read"))?;
    let journal_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|_| demo_error("demo-journal-mode-read"))?;
    if schema_version != 10
        || foreign_keys != 1
        || busy_timeout < 2_500
        || !matches!(journal_mode.as_str(), "wal" | "memory")
    {
        return Err(demo_error("demo-database-config-invalid"));
    }
    let required_objects: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE name IN ('app_metadata','intel_items','intel_items_fts','intel_contents',
                            'item_provenance','rule_evaluations','analysis_results',
                            'settings_metadata','setup_progress','setup_idempotency',
                            'configuration_versions','configuration_current','configuration_idempotency',
                            'sources','source_entry_checkpoints','source_idempotency',
                            'jobs','job_source_states','sync_runs','sync_source_results','sync_result_items',
                            'sync_result_item_failures','intel_associations','intel_association_members')
             AND type IN ('table','view')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| demo_error("demo-schema-read"))?;
    if required_objects != 24 {
        return Err(demo_error("demo-schema-invalid"));
    }
    for (table, required_columns) in [
        (
            "intel_items",
            &[
                "id",
                "intel_item_id",
                "external_id",
                "data_origin",
                "source_id",
                "source_kind",
                "stable_external_id",
                "content_hash",
                "revision",
                "publisher",
                "title",
                "track",
                "summary",
                "original_url",
                "importance",
                "ai_status",
                "published_at",
                "collected_at",
            ][..],
        ),
        (
            "intel_contents",
            &[
                "item_id",
                "what_happened",
                "facts_json",
                "source_summary",
                "content_hash",
                "content_state",
            ][..],
        ),
        (
            "item_provenance",
            &[
                "provenance_id",
                "item_id",
                "source_id",
                "stable_external_id",
                "source_kind",
                "publisher",
                "author",
                "author_availability",
                "original_title",
                "original_url",
                "published_at",
                "collected_at",
                "first_discovered_at",
                "last_updated_at",
                "availability_status",
                "warnings_json",
                "content_hash",
                "deterministic_association_basis",
            ][..],
        ),
        (
            "rule_evaluations",
            &[
                "item_id",
                "why_it_matters",
                "possible_impact",
                "importance",
                "reasons_json",
                "rule_version",
                "configuration_revision",
                "configuration_hash",
                "fact_revision",
                "evaluated_at_ms",
                "score",
                "stream_disposition",
                "matched_tracks_json",
                "factor_results_json",
                "filter_reasons_json",
                "ai_status",
            ][..],
        ),
        ("settings_metadata", &["key", "value"][..]),
        (
            "setup_progress",
            &[
                "step_id",
                "status",
                "revision",
                "saved_fields_version",
                "updated_at_ms",
            ][..],
        ),
        (
            "setup_idempotency",
            &[
                "idempotency_key",
                "request_fingerprint",
                "response_revision",
                "response_json",
            ][..],
        ),
        (
            "configuration_versions",
            &[
                "version",
                "validator_version",
                "normalized_config_hash",
                "configuration_json",
                "created_at_ms",
            ][..],
        ),
        ("configuration_current", &["singleton_id", "version"][..]),
        (
            "configuration_idempotency",
            &[
                "idempotency_key",
                "request_fingerprint",
                "response_version",
                "response_json",
            ][..],
        ),
        (
            "sources",
            &[
                "source_id",
                "configuration_version",
                "source_kind",
                "canonical_url",
                "enabled",
                "revision",
                "etag",
                "last_modified",
                "adapter_cursor",
                "last_attempt_at_ms",
                "last_success_at_ms",
                "status",
                "consecutive_failures",
                "retryability",
                "next_allowed_at_ms",
                "error_code",
                "created_at_ms",
                "updated_at_ms",
            ][..],
        ),
        (
            "source_entry_checkpoints",
            &[
                "source_id",
                "stable_external_id",
                "content_hash",
                "first_seen_at_ms",
                "last_seen_at_ms",
            ][..],
        ),
        (
            "source_idempotency",
            &["idempotency_key", "request_fingerprint", "response_json"][..],
        ),
        (
            "jobs",
            &[
                "task_id",
                "kind",
                "target_kind",
                "target_source_id",
                "state",
                "revision",
                "idempotency_key",
                "request_fingerprint",
                "foreground_budget_ms",
                "created_at_ms",
                "started_at_ms",
                "finished_at_ms",
                "updated_at_ms",
                "error_summary",
            ][..],
        ),
        (
            "job_source_states",
            &[
                "task_id",
                "source_id",
                "source_revision",
                "state",
                "last_success_at_ms",
                "error_code",
                "next_allowed_at_ms",
                "updated_at_ms",
            ][..],
        ),
        (
            "sync_runs",
            &[
                "sync_run_id",
                "task_id",
                "scope",
                "outcome",
                "started_at_ms",
                "finished_at_ms",
                "inserted_count",
                "updated_count",
                "skipped_count",
                "failed_count",
            ][..],
        ),
        (
            "sync_source_results",
            &[
                "sync_run_id",
                "source_id",
                "source_revision",
                "source_kind",
                "publisher",
                "status",
                "inserted_count",
                "updated_count",
                "skipped_count",
                "failed_count",
                "error_code",
            ][..],
        ),
        (
            "sync_result_items",
            &[
                "result_item_id",
                "sync_run_id",
                "source_id",
                "stable_external_id",
                "intel_item_id",
                "source_kind",
                "publisher",
                "original_title",
                "published_at",
                "collected_at",
                "original_url",
                "disposition",
            ][..],
        ),
        (
            "sync_result_item_failures",
            &[
                "failure_id",
                "sync_run_id",
                "source_id",
                "candidate_ref",
                "field_name",
                "reason_code",
                "observed_at",
            ][..],
        ),
        (
            "intel_associations",
            &[
                "association_id",
                "relation_type",
                "evidence_basis",
                "basis_version",
                "basis_hash",
                "first_observed_at",
                "last_observed_at",
            ][..],
        ),
        (
            "intel_association_members",
            &[
                "association_id",
                "item_id",
                "first_observed_at",
                "last_observed_at",
            ][..],
        ),
    ] {
        let columns = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| demo_error("setup-schema-columns"))?;
        if !required_columns
            .iter()
            .all(|required| columns.iter().any(|column| column == required))
        {
            return Err(demo_error("setup-schema-invalid"));
        }
    }
    for (table, required_fragments) in [
        (
            "intel_items",
            &[
                "intel_item_id text not null unique",
                "unique(source_id,stable_external_id)",
                "check(data_origin='demo' or (source_kind is not null and content_hash is not null))",
            ][..],
        ),
        (
            "intel_contents",
            &["content_state text not null check(content_state in ('demo','metadata_only'))"][..],
        ),
        (
            "item_provenance",
            &[
                "provenance_id text primary key not null",
                "author_availability text not null check(author_availability in ('available','unavailable','unknown_legacy'))",
                "availability_status text not null check(availability_status in ('available','unavailable','unknown_legacy'))",
            ][..],
        ),
        (
            "rule_evaluations",
            &[
                "item_id integer primary key references intel_items(id) on delete cascade",
                "importance text not null check(importance in ('low','medium','high'))",
                "configuration_revision integer check(configuration_revision>=1)",
                "configuration_hash text check(configuration_hash is null or length(configuration_hash)=64)",
                "fact_revision integer check(fact_revision>=1)",
                "evaluated_at_ms integer check(evaluated_at_ms>=1)",
                "score integer check(score between 0 and 100)",
                "stream_disposition text check(stream_disposition in ('high_value','ordinary_candidate'))",
                "ai_status text check(ai_status in ('unavailable'))",
            ][..],
        ),
        (
            "configuration_versions",
            &[
                "version integer primary key check(version>=1)",
                "created_at_ms integer not null",
            ][..],
        ),
        (
            "configuration_current",
            &[
                "singleton_id integer primary key check(singleton_id=1)",
                "version integer not null unique references configuration_versions(version)",
            ][..],
        ),
        (
            "configuration_idempotency",
            &[
                "idempotency_key text primary key",
                "response_version integer not null references configuration_versions(version)",
            ][..],
        ),
        (
            "sources",
            &[
                "source_id text primary key not null",
                "source_kind text not null check(source_kind='rss_atom')",
                "canonical_url text not null unique",
                "enabled integer not null check(enabled in (0,1))",
                "status text not null check(status in ('ready','error','retry_wait'))",
                "retryability text not null check(retryability in ('never','manual','automatic','after'))",
            ][..],
        ),
        (
            "source_entry_checkpoints",
            &[
                "source_id text not null references sources(source_id) on delete cascade",
                "primary key(source_id, stable_external_id)",
            ][..],
        ),
        (
            "source_idempotency",
            &[
                "idempotency_key text primary key not null",
                "response_json text not null",
            ][..],
        ),
        (
            "jobs",
            &[
                "task_id text primary key not null",
                "kind text not null check(kind='rss_atom_sync')",
                "target_kind text not null check(target_kind in ('all_enabled_rss_atom','source_id'))",
                "state text not null check(state in ('queued','running','retry_wait','succeeded','partially_succeeded','failed','cancelled'))",
                "idempotency_key text not null unique",
                "foreground_budget_ms integer not null check(foreground_budget_ms=30000)",
            ][..],
        ),
        (
            "job_source_states",
            &[
                "task_id text not null references jobs(task_id) on delete cascade",
                "source_id text not null references sources(source_id)",
                "primary key(task_id,source_id)",
            ][..],
        ),
        (
            "sync_runs",
            &[
                "sync_run_id text primary key not null",
                "task_id text not null unique references jobs(task_id) on delete cascade",
                "outcome text check(outcome in ('succeeded_with_results','succeeded_zero_results','partially_succeeded','failed'))",
            ][..],
        ),
        (
            "sync_source_results",
            &[
                "sync_run_id text not null references sync_runs(sync_run_id) on delete cascade",
                "source_kind text not null check(source_kind='rss_atom')",
                "primary key(sync_run_id,source_id)",
            ][..],
        ),
        (
            "sync_result_items",
            &[
                "result_item_id text primary key not null",
                "intel_item_id text not null references intel_items(intel_item_id) on delete restrict",
                "foreign key(source_id,stable_external_id) references source_entry_checkpoints(source_id,stable_external_id)",
                "constraint uq_sync_result_items_run_source_external unique(sync_run_id,source_id,stable_external_id)",
                "constraint uq_sync_result_items_sync_run_id_intel_item_id unique(sync_run_id,intel_item_id)",
            ][..],
        ),
        (
            "sync_result_item_failures",
            &[
                "sync_run_id text not null references sync_runs(sync_run_id) on delete cascade",
                "unique(sync_run_id,source_id,candidate_ref)",
            ][..],
        ),
        (
            "intel_associations",
            &[
                "relation_type text not null check(relation_type='same_event')",
                "evidence_basis text not null check(evidence_basis='normalized_original_url')",
                "basis_version integer not null check(basis_version=1)",
                "basis_hash text not null check(length(basis_hash)=64)",
                "constraint uq_intel_association_evidence unique(relation_type,evidence_basis,basis_version,basis_hash)",
                "check(first_observed_at<=last_observed_at)",
            ][..],
        ),
        (
            "intel_association_members",
            &[
                "association_id text not null references intel_associations(association_id) on delete cascade",
                "item_id integer not null references intel_items(id) on delete restrict",
                "primary key(association_id,item_id)",
                "constraint uq_intel_association_member_item unique(item_id)",
                "check(first_observed_at<=last_observed_at)",
            ][..],
        ),
    ] {
        let sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .map_err(|_| demo_error("configuration-schema-sql-read"))?;
        let normalized_sql = sql
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace('"', "")
            .to_ascii_lowercase();
        if !required_fragments
            .iter()
            .all(|fragment| normalized_sql.contains(fragment))
        {
            return Err(demo_error("configuration-schema-constraints"));
        }
    }
    let active_index_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name='job_source_active_unique'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| demo_error("sync-schema-index-read"))?;
    let normalized_index = active_index_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if !normalized_index.contains("unique index job_source_active_unique")
        || !normalized_index.contains("where state in ('queued','running')")
    {
        return Err(demo_error("sync-schema-index-invalid"));
    }
    let association_member_index_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name='intel_association_members_item_idx'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| demo_error("association-schema-index-read"))?;
    let normalized_association_member_index = association_member_index_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if !normalized_association_member_index
        .contains("index intel_association_members_item_idx on intel_association_members(item_id)")
    {
        return Err(demo_error("association-schema-index-invalid"));
    }
    let rule_index_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name='rule_evaluations_stream_idx'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| demo_error("rule-schema-index-read"))?;
    let normalized_rule_index = rule_index_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if !normalized_rule_index.contains(
        "index rule_evaluations_stream_idx on rule_evaluations(stream_disposition,score desc,item_id)",
    ) || !normalized_rule_index.contains("where rule_version is not null")
    {
        return Err(demo_error("rule-schema-index-invalid"));
    }
    let foreign_key_violations: u32 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(|_| demo_error("configuration-foreign-key-check"))?;
    if foreign_key_violations != 0 {
        return Err(demo_error("configuration-foreign-key-invalid"));
    }
    crate::infrastructure::database::association_repository::verify_associations(connection)
        .map_err(|_| demo_error("association-contract-invalid"))?;
    let current_count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM configuration_current c
             JOIN configuration_versions v ON v.version=c.version
             WHERE c.singleton_id=1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| demo_error("configuration-current-read"))?;
    if current_count != 1 {
        return Err(demo_error("configuration-current-invalid"));
    }
    let current = super::configuration::read_configuration(connection)?;
    let normalized =
        crate::domain::rules::configuration_validation::normalize(&current.configuration);
    let expected_hash =
        crate::domain::rules::configuration_validation::configuration_hash(&normalized);
    if current.validator_version
        != crate::domain::rules::configuration_validation::VALIDATOR_VERSION
        || current.normalized_config_hash != expected_hash
        || current.configuration != normalized
    {
        return Err(demo_error("configuration-current-contract"));
    }
    if !crate::infrastructure::database::rule_evaluation_repository::verify_current(
        connection, &current,
    )
    .map_err(|_| demo_error("rule-current-read"))?
    {
        return Err(demo_error("rule-current-invalid"));
    }
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
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct ScopedDatabase {
        path: PathBuf,
    }

    impl ScopedDatabase {
        fn new(label: &str) -> Self {
            let sequence = TEST_DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::current_dir()
                .expect("project directory")
                .join("target")
                .join("story-2-1-core-review")
                .join(format!("{label}-{}-{sequence}.sqlite3", std::process::id()));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("test database directory");
            }
            Self { path }
        }
    }

    impl Drop for ScopedDatabase {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm"] {
                let candidate = PathBuf::from(format!("{}{suffix}", self.path.display()));
                let _ = std::fs::remove_file(candidate);
            }
        }
    }

    fn drop_v9_association_tables(connection: &Connection) {
        connection
            .execute_batch(
                "DROP TABLE intel_association_members;
                 DROP TABLE intel_associations;",
            )
            .expect("simulate pre-v9 boundary");
    }

    fn insert_association_fact(
        connection: &Connection,
        source_id: &str,
        stable_external_id: &str,
        original_url: &str,
        observed_at: &str,
    ) -> i64 {
        let canonical = crate::domain::intel::canonical_external_id(source_id, stable_external_id);
        let intel_id = crate::domain::intel::derive_intel_item_id("rss_atom", &canonical);
        connection
            .execute(
                "INSERT INTO sources
                 (source_id,configuration_version,source_kind,canonical_url,enabled,revision,status,consecutive_failures,retryability,created_at_ms,updated_at_ms)
                 VALUES(?1,1,'rss_atom','https://association-source.example/feed.xml',1,1,'ready',0,'manual',1,1)",
                [source_id],
            )
            .expect("association source");
        connection
            .execute(
                "INSERT INTO intel_items
                 (intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,original_url,published_at,collected_at)
                 VALUES(?1,?2,'real',?3,'rss_atom',?4,?5,1,'Association Publisher','Association Title',?6,NULL,?7)",
                params![
                    intel_id.as_str(),
                    canonical,
                    source_id,
                    stable_external_id,
                    "a".repeat(64),
                    original_url,
                    observed_at,
                ],
            )
            .expect("association fact");
        let item_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO item_provenance
                 (provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,author_availability,original_title,original_url,published_at,collected_at,first_discovered_at,last_updated_at,availability_status,warnings_json,content_hash,deterministic_association_basis)
                 VALUES(?1,?2,?3,?4,'rss_atom','Association Publisher',NULL,'unavailable','Association Title',?5,NULL,?6,?6,?6,'available','[]',?7,'source_kind+canonical_external_id')",
                params![
                    format!("prov:{}", intel_id.as_str()),
                    item_id,
                    source_id,
                    stable_external_id,
                    original_url,
                    observed_at,
                    "a".repeat(64),
                ],
            )
            .expect("association provenance");
        crate::infrastructure::database::association_repository::reconcile_url_membership(
            connection,
            item_id,
            original_url,
            observed_at,
            observed_at,
        )
        .expect("association membership");
        if table_has_column(connection, "rule_evaluations", "rule_version")
            .expect("rule schema check")
        {
            let current = crate::application::configuration::read_configuration(connection)
                .expect("rule configuration");
            crate::infrastructure::database::rule_evaluation_repository::evaluate_item(
                connection,
                item_id,
                &current,
                current.updated_at_ms,
            )
            .expect("rule projection");
        }
        item_id
    }

    fn replace_v8_fact_tables_with_v7_result(connection: &Connection) {
        connection
            .execute_batch(
                "PRAGMA foreign_keys=OFF;
                 DROP TABLE intel_association_members;
                 DROP TABLE intel_associations;
                 DROP TABLE sync_result_item_failures;
                 DROP TABLE sync_result_items;
                 DROP TABLE intel_items_fts;
                 DROP TABLE intel_contents;
                 DROP TABLE item_provenance;
                 DROP TABLE rule_evaluations;
                 DROP TABLE analysis_results;
                 DROP TABLE intel_items;
                 CREATE TABLE intel_items (
                   id INTEGER PRIMARY KEY,external_id TEXT NOT NULL,
                   data_origin TEXT NOT NULL CHECK(data_origin IN ('demo','real')),
                   publisher TEXT NOT NULL,title TEXT NOT NULL,track TEXT NOT NULL,summary TEXT NOT NULL,
                   original_url TEXT NOT NULL,importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                   ai_status TEXT NOT NULL CHECK(ai_status IN ('generated','waiting','failed','unavailable')),
                   published_at TEXT NOT NULL,collected_at TEXT NOT NULL,UNIQUE(data_origin,external_id)
                 );
                 CREATE TABLE intel_contents(item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,what_happened TEXT NOT NULL,facts_json TEXT NOT NULL);
                 CREATE TABLE item_provenance(item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,source_kind TEXT NOT NULL,publisher TEXT NOT NULL,author TEXT,original_title TEXT NOT NULL,original_url TEXT NOT NULL,published_at TEXT,collected_at TEXT NOT NULL,first_discovered_at TEXT NOT NULL,last_updated_at TEXT NOT NULL,availability_status TEXT NOT NULL CHECK(availability_status IN ('available','unavailable')),deterministic_association_basis TEXT);
                 CREATE TABLE rule_evaluations(item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,why_it_matters TEXT NOT NULL,possible_impact TEXT NOT NULL,importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),reasons_json TEXT NOT NULL);
                 CREATE TABLE analysis_results(item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,content TEXT NOT NULL,confidence_percent INTEGER NOT NULL CHECK(confidence_percent BETWEEN 0 AND 100),status TEXT NOT NULL CHECK(status IN ('generated','waiting','failed','unavailable')));
                 CREATE VIRTUAL TABLE intel_items_fts USING fts5(title,publisher,track,summary,content='intel_items',content_rowid='id',tokenize='trigram');
                 CREATE TABLE sync_result_items (
                   result_item_id TEXT PRIMARY KEY NOT NULL,
                   sync_run_id TEXT NOT NULL REFERENCES sync_runs(sync_run_id) ON DELETE CASCADE,
                   source_id TEXT NOT NULL,stable_external_id TEXT NOT NULL,
                   source_kind TEXT NOT NULL CHECK(source_kind='rss_atom'),publisher TEXT NOT NULL,
                   original_title TEXT NOT NULL,published_at TEXT,collected_at TEXT NOT NULL,original_url TEXT NOT NULL,
                   disposition TEXT NOT NULL CHECK(disposition IN ('inserted','updated')),
                   FOREIGN KEY(source_id,stable_external_id) REFERENCES source_entry_checkpoints(source_id,stable_external_id),
                   CONSTRAINT uq_sync_result_items_run_source_external UNIQUE(sync_run_id,source_id,stable_external_id)
                 );
                 CREATE INDEX sync_result_items_page_idx ON sync_result_items(sync_run_id,source_id,collected_at,result_item_id);
                 INSERT INTO sources(source_id,configuration_version,source_kind,canonical_url,enabled,revision,status,consecutive_failures,retryability,created_at_ms,updated_at_ms)
                   VALUES('source:aaaaaaaaaaaaaaaaaaaaaaaa',1,'rss_atom','https://legacy.example/feed.xml',1,2,'ready',0,'never',1000,2000);
                 INSERT INTO source_entry_checkpoints(source_id,stable_external_id,content_hash,first_seen_at_ms,last_seen_at_ms)
                   VALUES('source:aaaaaaaaaaaaaaaaaaaaaaaa','entry-1','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',1000,2000);
                 INSERT INTO jobs(task_id,kind,target_kind,target_source_id,state,revision,idempotency_key,request_fingerprint,foreground_budget_ms,created_at_ms,started_at_ms,finished_at_ms,updated_at_ms)
                   VALUES('task:aaaaaaaaaaaaaaaaaaaaaaaa','rss_atom_sync','source_id','source:aaaaaaaaaaaaaaaaaaaaaaaa','succeeded',3,'legacy-v7-task','legacy-v7-fingerprint',30000,1000,1000,2000,2000);
                 INSERT INTO sync_runs(sync_run_id,task_id,scope,outcome,started_at_ms,finished_at_ms,inserted_count,updated_count,skipped_count,failed_count)
                   VALUES('run:aaaaaaaaaaaaaaaaaaaaaaaa','task:aaaaaaaaaaaaaaaaaaaaaaaa','source_id','succeeded_with_results',1000,2000,1,0,0,0);
                 INSERT INTO sync_source_results(sync_run_id,source_id,source_revision,source_kind,publisher,status,inserted_count,updated_count,skipped_count,failed_count,error_code)
                   VALUES('run:aaaaaaaaaaaaaaaaaaaaaaaa','source:aaaaaaaaaaaaaaaaaaaaaaaa',2,'rss_atom','legacy.example','succeeded',1,0,0,0,NULL);
                 INSERT INTO sync_result_items(result_item_id,sync_run_id,source_id,stable_external_id,source_kind,publisher,original_title,published_at,collected_at,original_url,disposition)
                   VALUES('result:aaaaaaaaaaaaaaaaaaaaaaaa','run:aaaaaaaaaaaaaaaaaaaaaaaa','source:aaaaaaaaaaaaaaaaaaaaaaaa','entry-1','rss_atom','legacy.example','Legacy title',NULL,'2026-08-19T02:00:00Z','https://legacy.example/item','inserted');
                 INSERT INTO jobs(task_id,kind,target_kind,target_source_id,state,revision,idempotency_key,request_fingerprint,foreground_budget_ms,created_at_ms,started_at_ms,finished_at_ms,updated_at_ms)
                   VALUES('task:bbbbbbbbbbbbbbbbbbbbbbbb','rss_atom_sync','source_id','source:aaaaaaaaaaaaaaaaaaaaaaaa','succeeded',3,'legacy-v7-task-b','legacy-v7-fingerprint-b',30000,1000,1000,2000,2000);
                 INSERT INTO sync_runs(sync_run_id,task_id,scope,outcome,started_at_ms,finished_at_ms,inserted_count,updated_count,skipped_count,failed_count)
                   VALUES('run:bbbbbbbbbbbbbbbbbbbbbbbb','task:bbbbbbbbbbbbbbbbbbbbbbbb','source_id','succeeded_with_results',1000,2000,0,1,0,0);
                 INSERT INTO sync_source_results(sync_run_id,source_id,source_revision,source_kind,publisher,status,inserted_count,updated_count,skipped_count,failed_count,error_code)
                   VALUES('run:bbbbbbbbbbbbbbbbbbbbbbbb','source:aaaaaaaaaaaaaaaaaaaaaaaa',2,'rss_atom','legacy.example','succeeded',0,1,0,0,NULL);
                 INSERT INTO sync_result_items(result_item_id,sync_run_id,source_id,stable_external_id,source_kind,publisher,original_title,published_at,collected_at,original_url,disposition)
                   VALUES('result:bbbbbbbbbbbbbbbbbbbbbbbb','run:bbbbbbbbbbbbbbbbbbbbbbbb','source:aaaaaaaaaaaaaaaaaaaaaaaa','entry-1','rss_atom','legacy.example','Tie-break title',NULL,'2026-08-19T02:00:00Z','https://legacy.example/item','updated');
                 PRAGMA user_version=7;
                 PRAGMA foreign_keys=ON;",
            )
            .expect("exact v7 fact/result boundary");
    }

    #[test]
    fn reseeding_demo_rows_preserves_real_rows_and_search_projection() {
        let mut store = DemoStore::open_in_memory().expect("demo store");
        store.bootstrap().expect("initial demo seed");
        store
            .connection
            .execute(
                "INSERT INTO intel_items
                 (intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,track,summary,original_url,importance,ai_status,published_at,collected_at)
                 VALUES (?1,'shared-id','real',NULL,'legacy',NULL,?2,1,'Real Publisher','Real Title','real','real summary','https://example.com/real','medium','unavailable','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                params![
                    crate::domain::intel::derive_intel_item_id("legacy", "shared-id").as_str(),
                    "0".repeat(64),
                ],
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
            DEMO_FIXTURE.replacen(
                "\"ai_confidence_percent\": 88",
                "\"ai_confidence_percent\": 101",
                1,
            ),
            DEMO_FIXTURE.replacen("\"source_kind\"", "\"unknown\":1,\"source_kind\"", 1),
        ] {
            assert!(validate_demo_fixture(&invalid).is_err());
        }
    }

    #[test]
    fn v1_database_migrates_transactionally_to_versioned_evidence_and_setup() {
        let connection = Connection::open_in_memory().expect("legacy memory database");
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA busy_timeout=2500; PRAGMA journal_mode=WAL;
                 CREATE TABLE app_metadata (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
                 CREATE TABLE intel_items (
                   id INTEGER PRIMARY KEY, external_id TEXT NOT NULL, data_origin TEXT NOT NULL,
                   publisher TEXT NOT NULL, title TEXT NOT NULL, track TEXT NOT NULL,
                   summary TEXT NOT NULL, original_url TEXT NOT NULL,
                   published_at TEXT NOT NULL, collected_at TEXT NOT NULL,
                   UNIQUE(data_origin, external_id)
                 );
                 CREATE VIRTUAL TABLE intel_items_fts USING fts5(
                   title, publisher, track, summary, content='intel_items', content_rowid='id', tokenize='trigram'
                 );
                 INSERT INTO intel_items
                   (id,external_id,data_origin,publisher,title,track,summary,original_url,published_at,collected_at)
                 VALUES
                   (41,'demo:legacy-demo','demo','Legacy Demo','Legacy demo','legacy','legacy demo summary','https://example.com/demo','2026-01-01T00:00:00Z','2026-01-02T00:00:00Z'),
                   (42,'legacy-real','real','Legacy Real','Legacy real','legacy','legacy real summary','https://example.com/real','2026-02-01T00:00:00Z','2026-02-02T00:00:00Z');
                 INSERT INTO intel_items_fts(rowid,title,publisher,track,summary)
                   SELECT id,title,publisher,track,summary FROM intel_items;
                 PRAGMA user_version=1;",
            )
            .expect("legacy schema");

        let mut store = DemoStore::from_connection(connection).expect("migrate v1 to v2");
        let migrated_real: (String, String, String, String) = store
            .connection
            .query_row(
                "SELECT i.original_url,p.original_url,p.availability_status,a.status
                 FROM intel_items i
                 JOIN item_provenance p ON p.item_id=i.id
                 JOIN analysis_results a ON a.item_id=i.id
                 WHERE i.data_origin='real' AND i.external_id='legacy-real'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("legacy real evidence preserved");
        assert_eq!(
            migrated_real,
            (
                "https://example.com/real".to_owned(),
                "https://example.com/real".to_owned(),
                "unknown_legacy".to_owned(),
                "unavailable".to_owned(),
            )
        );
        store.bootstrap().expect("seed evidence after migration");
        let detail = store
            .detail("demo:openai-agents-sdk-001")
            .expect("versioned evidence detail");
        assert_eq!(detail.contract_version, 1);
        assert_eq!(detail.provenance.publisher, "OpenAI");
        let schema_version: u32 = store
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version");
        assert_eq!(schema_version, 10);
        assert_eq!(store.get_setup_progress().expect("fresh setup").revision, 0);
    }

    #[test]
    fn v2_database_upgrades_setup_without_losing_demo_rows() {
        let mut original = DemoStore::open_in_memory().expect("fresh store");
        let before = original.bootstrap().expect("seed demo");
        drop_v9_association_tables(&original.connection);
        original
            .connection
            .execute_batch(
                "DROP TABLE sync_result_items;
                 DROP TABLE sync_source_results;
                 DROP TABLE sync_runs;
                 DROP TABLE job_source_states;
                 DROP TABLE jobs;
                 DROP TABLE source_entry_checkpoints;
                 DROP TABLE source_idempotency;
                 DROP TABLE sources;
                 DROP TABLE configuration_idempotency;
                 DROP TABLE configuration_current;
                 DROP TABLE configuration_versions;
                 DROP TABLE setup_idempotency;
                 DROP TABLE setup_progress;
                 DROP TABLE settings_metadata;
                 PRAGMA user_version=2;",
            )
            .expect("simulate exact v2 boundary");

        let mut migrated = DemoStore::from_connection(original.connection).expect("migrate v2");
        assert_eq!(migrated.bootstrap().expect("preserved demo"), before);
        assert_eq!(
            migrated
                .get_setup_progress()
                .expect("setup available")
                .next_step_id,
            Some(super::super::setup::SetupStepId::Tracks)
        );
    }

    #[test]
    fn v3_setup_metadata_migrates_to_one_authoritative_configuration() {
        let original = DemoStore::open_in_memory().expect("fresh store");
        drop_v9_association_tables(&original.connection);
        original
            .connection
            .execute_batch(
                "DROP TABLE sync_result_items;
                 DROP TABLE sync_source_results;
                 DROP TABLE sync_runs;
                 DROP TABLE job_source_states;
                 DROP TABLE jobs;
                 DROP TABLE source_entry_checkpoints;
                 DROP TABLE source_idempotency;
                 DROP TABLE sources;
                 DROP TABLE configuration_idempotency;
                 DROP TABLE configuration_current;
                 DROP TABLE configuration_versions;
                 INSERT INTO settings_metadata(key,value) VALUES
                   ('track_ids','[\"ai_agents\",\"local_models\"]'),
                   ('source_example_ids','[\"github_releases\"]'),
                   ('refresh_cadence','hourly'),
                   ('ai_data_disclosure_acknowledged','true');
                 PRAGMA user_version=3;",
            )
            .expect("simulate exact v3 boundary");

        let migrated = DemoStore::from_connection(original.connection).expect("migrate v3");
        let configuration = migrated.get_configuration().expect("configuration");
        let setup = migrated.get_setup_progress().expect("setup projection");

        assert_eq!(configuration.revision, 1);
        assert_eq!(
            configuration
                .configuration
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            ["ai_agents", "local_models"]
        );
        assert!(configuration.configuration.refresh_enabled);
        assert_eq!(configuration.configuration.refresh_interval_minutes, 60);
        assert_eq!(setup.configuration_revision, 1);
        assert_eq!(setup.saved_config.track_ids, ["ai_agents", "local_models"]);
        assert_eq!(
            setup.saved_config.refresh_cadence.as_deref(),
            Some("hourly")
        );
        assert_eq!(setup.saved_config.source_example_ids, ["github_releases"]);
        assert_eq!(configuration.configuration.source_preferences.len(), 1);
        assert_eq!(
            configuration.configuration.source_preferences[0].identifier,
            "https://example.invalid/feed.xml"
        );
    }

    #[test]
    fn v4_database_migrates_sources_without_losing_existing_state() {
        let mut original = DemoStore::open_in_memory().expect("fresh store");
        let before = original.bootstrap().expect("seed demo");
        drop_v9_association_tables(&original.connection);
        original
            .connection
            .execute_batch(
                "DROP TABLE sync_result_items;
             DROP TABLE sync_source_results;
             DROP TABLE sync_runs;
             DROP TABLE job_source_states;
             DROP TABLE jobs;
             DROP TABLE source_entry_checkpoints;
             DROP TABLE source_idempotency;
             DROP TABLE sources;
             PRAGMA user_version=4;",
            )
            .expect("simulate exact v4 boundary");

        let mut migrated = DemoStore::from_connection(original.connection).expect("migrate v4");
        assert_eq!(migrated.bootstrap().expect("demo preserved"), before);
        assert_eq!(
            migrated
                .get_configuration()
                .expect("configuration preserved")
                .revision,
            1
        );
        assert!(
            migrated
                .query_sources(None, 100)
                .expect("source projection")
                .items
                .is_empty()
        );
        let version: u32 = migrated
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, 10);
    }

    #[test]
    fn v5_database_migrates_persistent_sync_jobs_and_result_fact_tables() {
        let original = DemoStore::open_in_memory().expect("fresh v8");
        drop_v9_association_tables(&original.connection);
        original
            .connection
            .execute_batch(
                "DROP TABLE sync_result_items;
                 DROP TABLE sync_source_results;
                 DROP TABLE sync_runs;
                 DROP TABLE job_source_states;
                 DROP TABLE jobs;
                 PRAGMA user_version=5;",
            )
            .expect("simulate exact v5 boundary");

        let migrated = DemoStore::from_connection(original.connection).expect("migrate v5");
        let version: u32 = migrated
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, 10);
        for required in ["sync_runs", "sync_source_results", "sync_result_items"] {
            let count: u32 = migrated
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [required],
                    |row| row.get(0),
                )
                .expect("required table lookup");
            assert_eq!(count, 1);
        }
    }

    #[test]
    fn v6_legacy_tasks_migrate_without_fabricating_result_references() {
        let original = DemoStore::open_in_memory().expect("fresh v8");
        drop_v9_association_tables(&original.connection);
        original
            .connection
            .execute(
                "INSERT INTO jobs(task_id,kind,target_kind,target_source_id,state,revision,idempotency_key,request_fingerprint,foreground_budget_ms,created_at_ms,started_at_ms,finished_at_ms,updated_at_ms)
                 VALUES('task:111111111111111111111111','rss_atom_sync','all_enabled_rss_atom',NULL,'succeeded',1,'legacy-task','legacy-fingerprint',30000,1,1,2,2)",
                [],
            )
            .expect("legacy task");
        original
            .connection
            .execute_batch(
                "DROP TABLE sync_result_items;
                 DROP TABLE sync_source_results;
                 DROP TABLE sync_runs;
                 PRAGMA user_version=6;",
            )
            .expect("simulate exact v6 boundary");

        let migrated = DemoStore::from_connection(original.connection).expect("migrate v6");
        let snapshot = migrated
            .task_snapshot("task:111111111111111111111111")
            .expect("legacy snapshot");
        assert!(snapshot.result_ref.is_none());
        let run_count: u32 = migrated
            .connection
            .query_row("SELECT COUNT(*) FROM sync_runs", [], |row| row.get(0))
            .expect("run count");
        assert_eq!(run_count, 0);
    }

    #[test]
    fn v7_results_backfill_one_stable_fact_with_deterministic_tie_break() {
        let original = DemoStore::open_in_memory().expect("fresh v8");
        replace_v8_fact_tables_with_v7_result(&original.connection);

        let migrated = DemoStore::from_connection(original.connection).expect("migrate v7");
        let version: u32 = migrated
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("v8 version");
        let (intel_item_id, revision, title): (String, i64, String) = migrated
            .connection
            .query_row(
                "SELECT intel_item_id,revision,title FROM intel_items WHERE data_origin='real'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("backfilled fact");
        let linked_ids: Vec<String> = migrated
            .connection
            .prepare("SELECT intel_item_id FROM sync_result_items ORDER BY result_item_id")
            .expect("link query")
            .query_map([], |row| row.get(0))
            .expect("link rows")
            .collect::<Result<_, _>>()
            .expect("linked IDs");
        let provenance: (String, String, String) = migrated
            .connection
            .query_row(
                "SELECT author_availability,availability_status,content_hash FROM item_provenance WHERE source_id='source:aaaaaaaaaaaaaaaaaaaaaaaa'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("legacy provenance");
        let foreign_key_violations: u32 = migrated
            .connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .expect("foreign key check");

        assert_eq!(version, 10);
        assert_eq!(linked_ids, vec![intel_item_id.clone(), intel_item_id]);
        assert_eq!(revision, 1);
        assert_eq!(title, "Tie-break title");
        assert_eq!(
            provenance,
            (
                "unknown_legacy".to_owned(),
                "unknown_legacy".to_owned(),
                "a".repeat(64),
            )
        );
        assert_eq!(foreign_key_violations, 0);
    }

    #[test]
    fn v8_real_facts_backfill_one_deterministic_association_group() {
        let original = DemoStore::open_in_memory().expect("fresh v9");
        for (index, source_id) in [
            "source:111111111111111111111111",
            "source:222222222222222222222222",
        ]
        .into_iter()
        .enumerate()
        {
            let stable_external_id = format!("entry-{index}");
            let canonical =
                crate::domain::intel::canonical_external_id(source_id, &stable_external_id);
            let intel_id = crate::domain::intel::derive_intel_item_id("rss_atom", &canonical);
            original
                .connection
                .execute(
                    "INSERT INTO sources
                     (source_id,configuration_version,source_kind,canonical_url,enabled,revision,status,consecutive_failures,retryability,created_at_ms,updated_at_ms)
                     VALUES(?1,1,'rss_atom',?2,1,1,'ready',0,'manual',1,1)",
                    params![source_id, format!("https://source-{index}.example/feed.xml")],
                )
                .expect("legacy source");
            original
                .connection
                .execute(
                    "INSERT INTO intel_items
                     (intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,original_url,published_at,collected_at)
                     VALUES(?1,?2,'real',?3,'rss_atom',?4,?5,1,?6,?7,'https://shared.example/event',NULL,?8)",
                    params![
                        intel_id.as_str(),canonical,source_id,stable_external_id,"a".repeat(64),
                        format!("Publisher {index}"),format!("Title {index}"),
                        format!("2026-08-19T08:00:0{index}Z"),
                    ],
                )
                .expect("legacy fact");
            let item_id = original.connection.last_insert_rowid();
            original
                .connection
                .execute(
                    "INSERT INTO item_provenance
                     (provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,author_availability,original_title,original_url,published_at,collected_at,first_discovered_at,last_updated_at,availability_status,warnings_json,content_hash,deterministic_association_basis)
                     VALUES(?1,?2,?3,?4,'rss_atom',?5,NULL,'unavailable',?6,'https://shared.example/event',NULL,?7,?7,?7,'available','[]',?8,'source_kind+canonical_external_id')",
                    params![
                        format!("prov:{}", intel_id.as_str()),item_id,source_id,stable_external_id,
                        format!("Publisher {index}"),format!("Title {index}"),
                        format!("2026-08-19T08:00:0{index}Z"),"a".repeat(64),
                    ],
                )
                .expect("legacy provenance");
        }
        drop_v9_association_tables(&original.connection);
        original
            .connection
            .pragma_update(None, "user_version", 8_u32)
            .expect("v8 boundary");

        let migrated = DemoStore::from_connection(original.connection).expect("migrate v8");
        let counts: (i64, i64, String, String) = migrated
            .connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM intel_associations),
                        (SELECT COUNT(*) FROM intel_association_members),
                        MIN(first_observed_at),MAX(last_observed_at)
                 FROM intel_associations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("association backfill");
        assert_eq!(counts.0, 1);
        assert_eq!(counts.1, 2);
        assert_eq!(counts.2, "2026-08-19T08:00:00Z");
        assert_eq!(counts.3, "2026-08-19T08:00:01Z");
    }

    #[test]
    fn v9_real_facts_backfill_current_v10_rule_projection_and_preserve_demo_rows() {
        let mut original = DemoStore::open_in_memory().expect("fresh v10");
        original.bootstrap().expect("demo seed");
        let item_id = insert_association_fact(
            &original.connection,
            "source:333333333333333333333333",
            "rule-entry",
            "https://rules.example/release",
            "2026-08-19T08:00:00Z",
        );
        original
            .connection
            .execute_batch(
                "DROP INDEX rule_evaluations_stream_idx;
                 ALTER TABLE rule_evaluations RENAME TO rule_evaluations_v10;
                 CREATE TABLE rule_evaluations(
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                   why_it_matters TEXT NOT NULL,possible_impact TEXT NOT NULL,
                   importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                   reasons_json TEXT NOT NULL
                 );
                 INSERT INTO rule_evaluations(item_id,why_it_matters,possible_impact,importance,reasons_json)
                   SELECT item_id,why_it_matters,possible_impact,importance,reasons_json
                   FROM rule_evaluations_v10 WHERE rule_version IS NULL;
                 DROP TABLE rule_evaluations_v10;
                 PRAGMA user_version=9;",
            )
            .expect("simulate v9 rule boundary");

        let migrated = DemoStore::from_connection(original.connection).expect("migrate v9");
        let version: u32 = migrated
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("v10 version");
        let projection: (String, i64, String, String, String) = migrated
            .connection
            .query_row(
                "SELECT rule_version,fact_revision,stream_disposition,ai_status,factor_results_json
                 FROM rule_evaluations WHERE item_id=?1",
                [item_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("backfilled rule projection");
        let demo_count: u32 = migrated
            .connection
            .query_row(
                "SELECT COUNT(*) FROM rule_evaluations r JOIN intel_items i ON i.id=r.item_id
                 WHERE i.data_origin='demo' AND r.rule_version IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("legacy demo rules");
        assert_eq!(version, 10);
        assert_eq!(
            projection.0,
            crate::domain::rules::intelligence_value::INTELLIGENCE_VALUE_RULE_VERSION
        );
        assert_eq!(projection.1, 1);
        assert_eq!(projection.2, "ordinary_candidate");
        assert_eq!(projection.3, "unavailable");
        assert!(serde_json::from_str::<serde_json::Value>(&projection.4).is_ok());
        assert_eq!(demo_count, 3);
    }

    #[test]
    fn partial_v10_extension_fails_without_advancing_v9() {
        let scoped = ScopedDatabase::new("rule-partial-schema");
        let mut store = DemoStore::open(&scoped.path).expect("fresh v10");
        store.bootstrap().expect("demo seed");
        store
            .connection
            .execute_batch(
                "DROP INDEX rule_evaluations_stream_idx;
                 ALTER TABLE rule_evaluations RENAME TO rule_evaluations_v10;
                 CREATE TABLE rule_evaluations(
                   item_id INTEGER PRIMARY KEY REFERENCES intel_items(id) ON DELETE CASCADE,
                   why_it_matters TEXT NOT NULL,possible_impact TEXT NOT NULL,
                   importance TEXT NOT NULL CHECK(importance IN ('low','medium','high')),
                   reasons_json TEXT NOT NULL
                 );
                 INSERT INTO rule_evaluations(item_id,why_it_matters,possible_impact,importance,reasons_json)
                   SELECT item_id,why_it_matters,possible_impact,importance,reasons_json
                   FROM rule_evaluations_v10 WHERE rule_version IS NULL;
                 DROP TABLE rule_evaluations_v10;
                 ALTER TABLE rule_evaluations ADD COLUMN rule_version TEXT;
                 PRAGMA user_version=9;",
            )
            .expect("partial extension");
        drop(store);

        assert!(DemoStore::open(&scoped.path).is_err());
        let connection = Connection::open(&scoped.path).expect("inspect rollback");
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, 9);
    }

    #[test]
    fn v10_reopen_rejects_corrupt_current_rule_json() {
        let scoped = ScopedDatabase::new("rule-corrupt");
        let store = DemoStore::open(&scoped.path).expect("fresh v10");
        let item_id = insert_association_fact(
            &store.connection,
            "source:444444444444444444444444",
            "rule-entry",
            "https://rules.example/item",
            "2026-08-19T08:00:00Z",
        );
        let current = crate::application::configuration::read_configuration(&store.connection)
            .expect("configuration");
        crate::infrastructure::database::rule_evaluation_repository::evaluate_item(
            &store.connection,
            item_id,
            &current,
            current.updated_at_ms,
        )
        .expect("rule projection");
        drop(store);
        let connection = Connection::open(&scoped.path).expect("mutate v10");
        connection
            .execute(
                "UPDATE rule_evaluations SET factor_results_json='not-json' WHERE item_id=?1",
                [item_id],
            )
            .expect("corrupt rule JSON");
        drop(connection);
        assert!(DemoStore::open(&scoped.path).is_err());
    }

    #[test]
    fn v10_reopen_rejects_forged_rule_semantics_and_missing_provenance() {
        for (sequence, mutation) in [
            (
                0_u8,
                "UPDATE rule_evaluations SET filter_reasons_json='[{\"code\":\"forged\",\"actual\":null,\"threshold\":null}]' WHERE rule_version IS NOT NULL",
            ),
            (
                1_u8,
                "DELETE FROM item_provenance WHERE source_kind='rss_atom'",
            ),
        ] {
            let scoped = ScopedDatabase::new(&format!("rule-semantic-{sequence}"));
            let store = DemoStore::open(&scoped.path).expect("fresh v10");
            insert_association_fact(
                &store.connection,
                &format!("source:{:024x}", u64::from(sequence) + 10),
                "rule-entry",
                "https://rules.example/item",
                "2026-08-19T08:00:00Z",
            );
            drop(store);
            let connection = Connection::open(&scoped.path).expect("mutate v10");
            connection.execute_batch(mutation).expect("corrupt v10");
            drop(connection);
            assert!(DemoStore::open(&scoped.path).is_err());
        }
    }

    #[test]
    fn v9_file_reopen_rejects_membership_drift_and_missing_index() {
        for (sequence, mutation) in [
            (
                0_u8,
                "UPDATE item_provenance SET original_url='https://other.example/event'",
            ),
            (1, "DELETE FROM intel_association_members"),
            (2, "DROP INDEX intel_association_members_item_idx"),
        ] {
            let scoped = ScopedDatabase::new(&format!("association-corrupt-{sequence}"));
            let store = DemoStore::open(&scoped.path).expect("fresh file-backed v9");
            insert_association_fact(
                &store.connection,
                &format!("source:{sequence:024x}"),
                "entry",
                "https://shared.example/event",
                "2026-08-19T08:00:00Z",
            );
            drop(store);
            DemoStore::open(&scoped.path).expect("valid v9 reopens");
            let connection = Connection::open(&scoped.path).expect("mutate v9");
            connection.execute_batch(mutation).expect("corrupt v9");
            drop(connection);
            assert!(
                DemoStore::open(&scoped.path).is_err(),
                "startup must reject: {mutation}"
            );
        }
    }

    #[test]
    fn v9_migration_contract_failure_rolls_back_to_v8() {
        let scoped = ScopedDatabase::new("association-v8-rollback");
        let store = DemoStore::open(&scoped.path).expect("fresh file-backed v9");
        insert_association_fact(
            &store.connection,
            "source:999999999999999999999999",
            "entry",
            "https://shared.example/event",
            "2026-08-19T08:00:00Z",
        );
        drop_v9_association_tables(&store.connection);
        store
            .connection
            .execute(
                "UPDATE item_provenance SET last_updated_at='2026-08-19T08:00:00+00:00'",
                [],
            )
            .expect("make v8 lifecycle noncanonical");
        store
            .connection
            .pragma_update(None, "user_version", 8_u32)
            .expect("v8 boundary");
        drop(store);

        assert!(DemoStore::open(&scoped.path).is_err());
        let connection = Connection::open(&scoped.path).expect("inspect rolled back v8");
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("rolled back version");
        let association_tables: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('intel_associations','intel_association_members')",
                [],
                |row| row.get(0),
            )
            .expect("rolled back association tables");
        assert_eq!((version, association_tables), (8, 0));
    }

    #[test]
    fn v7_result_backfill_rejects_noncanonical_or_unsafe_history() {
        for mutation in [
            "UPDATE sync_result_items SET original_url='https://203.0.113.8/private'",
            "UPDATE sync_result_items SET collected_at='2026-08-19T02:00:00+00:00'",
            "UPDATE source_entry_checkpoints SET content_hash='NOT-A-CANONICAL-HASH'",
        ] {
            let original = DemoStore::open_in_memory().expect("fresh v8");
            replace_v8_fact_tables_with_v7_result(&original.connection);
            original
                .connection
                .execute(mutation, [])
                .expect("corrupt v7 history");
            assert!(
                DemoStore::from_connection(original.connection).is_err(),
                "migration must reject: {mutation}"
            );
        }
    }

    #[test]
    fn v6_contract_rejects_missing_active_source_uniqueness_gate() {
        let store = DemoStore::open_in_memory().expect("fresh v6");
        store
            .connection
            .execute("DROP INDEX job_source_active_unique", [])
            .expect("drop required index");
        assert!(DemoStore::from_connection(store.connection).is_err());
    }

    #[test]
    fn version_four_startup_rejects_corrupt_current_hash_and_missing_singleton() {
        for mutation in [
            "UPDATE configuration_versions SET normalized_config_hash='bad' WHERE version=1",
            "UPDATE configuration_versions SET configuration_json='{}' WHERE version=1",
            "DELETE FROM configuration_current WHERE singleton_id=1",
        ] {
            let store = DemoStore::open_in_memory().expect("fresh v4");
            store
                .connection
                .execute(mutation, [])
                .expect("mutate schema");
            let Err(error) = DemoStore::from_connection(store.connection) else {
                panic!("corrupt v4 must fail during startup");
            };
            assert!(
                matches!(
                    error.code(),
                    "internal.unexpected" | "storage.configuration"
                ),
                "{}",
                error.code()
            );
        }
    }

    #[test]
    fn invalid_v3_metadata_rolls_back_the_entire_v4_migration() {
        let scoped = ScopedDatabase::new("invalid-v3");
        let store = DemoStore::open(&scoped.path).expect("fresh store");
        store
            .connection
            .execute_batch(
                "DROP TABLE sync_result_items;
                 DROP TABLE sync_source_results;
                 DROP TABLE sync_runs;
                 DROP TABLE job_source_states;
                 DROP TABLE jobs;
                 DROP TABLE source_entry_checkpoints;
                 DROP TABLE source_idempotency;
                 DROP TABLE sources;
                 DROP TABLE configuration_idempotency;
                 DROP TABLE configuration_current;
                 DROP TABLE configuration_versions;
                 INSERT INTO settings_metadata(key,value) VALUES('refresh_cadence','unknown')
                   ON CONFLICT(key) DO UPDATE SET value=excluded.value;
                 PRAGMA user_version=3;",
            )
            .expect("simulate invalid v3 boundary");
        drop(store);

        assert!(DemoStore::open(&scoped.path).is_err());
        let connection = Connection::open(&scoped.path).expect("reopen failed migration");
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version");
        let configuration_tables: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'configuration_%'",
                [],
                |row| row.get(0),
            )
            .expect("configuration table count");
        assert_eq!(version, 3);
        assert_eq!(configuration_tables, 0);
    }

    #[test]
    fn version_four_contract_rejects_constraint_compatible_lookalike_tables() {
        let store = DemoStore::open_in_memory().expect("fresh v4");
        store
            .connection
            .execute_batch(
                "ALTER TABLE configuration_current RENAME TO configuration_current_valid;
                 CREATE TABLE configuration_current (
                   singleton_id INTEGER PRIMARY KEY,
                   version INTEGER NOT NULL
                 );
                 INSERT INTO configuration_current(singleton_id,version)
                   SELECT singleton_id,version FROM configuration_current_valid;
                 DROP TABLE configuration_current_valid;",
            )
            .expect("replace table with constraint-free lookalike");

        let error = verify_database_contract(&store.connection)
            .expect_err("missing singleton/unique/FK constraints must fail");
        assert_eq!(error.code(), "internal.unexpected");
    }

    #[test]
    fn version_four_contract_rejects_missing_setup_objects() {
        for table in ["settings_metadata", "setup_progress", "setup_idempotency"] {
            let store = DemoStore::open_in_memory().expect("fresh v3 store");
            store
                .connection
                .execute_batch(&format!("DROP TABLE {table}"))
                .expect("remove required setup object");
            let error = verify_database_contract(&store.connection)
                .expect_err("incomplete v3 schema must fail");
            assert_eq!(error.code(), "internal.unexpected");
        }
    }

    #[test]
    fn keyset_cursor_is_scoped_bounded_and_covers_each_item_once() {
        let mut store = DemoStore::open_in_memory().expect("demo store");
        store.bootstrap().expect("demo seed");

        let mut cursor = None;
        let mut observed = Vec::new();
        loop {
            let page = store
                .list_page(None, cursor.as_deref(), 1)
                .expect("valid keyset page");
            observed.extend(page.items.into_iter().map(|item| item.id));
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(observed.len(), 3);
        assert_eq!(
            observed
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            3
        );

        let first_page = store.list_page(None, None, 1).expect("first page");
        let first_cursor = first_page.next_cursor.as_deref().expect("cursor");
        assert!(
            store
                .list_page(Some("开发工具"), Some(first_cursor), 1)
                .is_err()
        );
        assert!(
            store
                .list_page(None, Some(&"x".repeat(MAX_CURSOR_LENGTH + 1)), 1)
                .is_err()
        );

        let forged_boundary = encode_cursor(
            "demo-v1",
            None,
            "2026-01-01T00:00:00Z",
            "demo:not-in-dataset",
        );
        assert!(store.list_page(None, Some(&forged_boundary), 1).is_err());
    }

    #[test]
    fn summary_projection_excludes_detail_fields() {
        let mut store = DemoStore::open_in_memory().expect("demo store");
        store.bootstrap().expect("demo seed");
        let first_page = store.list_page(None, None, 1).expect("first page");
        let serialized = serde_json::to_value(&first_page.items[0]).expect("summary JSON");
        for detail_only in [
            "what_happened",
            "facts",
            "rule_reasons",
            "ai_content",
            "provenance",
        ] {
            assert!(serialized.get(detail_only).is_none());
        }
    }
}
