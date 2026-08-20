//! Source configuration and incremental checkpoint use cases.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use super::configuration::append_configuration_version;
use super::demo::DemoStore;
use crate::contracts::dto::configuration_validation::SourcePreferenceV1;
use crate::contracts::dto::source::{
    SaveSourceInputV1, SourcePageV1, SourceRetryabilityV1, SourceStatusV1, SourceViewV1,
};
use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::sources::{
    CandidateApplyResult, CandidateDisposition, FetchIncrementalResult, IncrementalFetchRequest,
    SourceFetchState,
};
use crate::infrastructure::http::source_http_policy::{
    canonicalize_public_https_url, probe_rss_atom_source,
};

const MAX_PAGE_SIZE: u32 = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncrementalApplyOutcome {
    pub source: SourceViewV1,
    pub candidates: Vec<CandidateApplyResult>,
}

#[derive(Clone, Debug)]
pub struct ProbedSource {
    canonical_url: String,
    #[allow(dead_code)]
    initial_result: FetchIncrementalResult,
}

impl ProbedSource {
    /// Binds an adapter result to the exact canonical endpoint that produced it.
    ///
    /// # Errors
    /// Returns a validation error for an invalid endpoint or contradictory adapter result.
    pub fn from_adapter_result(
        endpoint: &str,
        result: FetchIncrementalResult,
    ) -> Result<Self, AppError> {
        let canonical_url = canonicalize_public_https_url(endpoint)?.to_string();
        if result.not_modified && !result.candidates.is_empty() {
            return Err(source_error(
                ErrorCode::ValidationSource,
                "source-probe-304-candidates",
            ));
        }
        Ok(Self {
            canonical_url,
            initial_result: result,
        })
    }
}

/// Performs the complete side-effect-free input validation followed by the bounded production
/// source probe. The platform layer must call this application seam rather than infrastructure.
///
/// # Errors
/// Returns a stable validation, network, or source-format error.
pub async fn probe_source_for_save(input: &SaveSourceInputV1) -> Result<ProbedSource, AppError> {
    validate_input(input)?;
    canonicalize_public_https_url(&input.url)?;
    let result = probe_rss_atom_source(&input.url, None, None).await?;
    ProbedSource::from_adapter_result(&input.url, result)
}

/// Runs the production HTTP policy for one prepared incremental request without holding `SQLite`.
///
/// # Errors
/// Returns a redacted source-scoped validation, network, rate-limit, or format error.
pub async fn fetch_incremental(
    request: &IncrementalFetchRequest,
) -> Result<FetchIncrementalResult, AppError> {
    probe_rss_atom_source(
        &request.canonical_url,
        request.etag.as_deref(),
        request.last_modified.as_deref(),
    )
    .await
    .map_err(|error| error.with_source_id(&request.source_id))
}

impl DemoStore {
    /// Reads the exact endpoint, revision, validators, and cursor needed before network I/O.
    ///
    /// # Errors
    /// Returns a stable storage error when the source is absent or corrupt.
    pub fn prepare_incremental_fetch(
        &self,
        source_id: &str,
    ) -> Result<IncrementalFetchRequest, AppError> {
        self.connection
            .query_row(
                "SELECT source_id,canonical_url,revision,etag,last_modified,adapter_cursor FROM sources WHERE source_id=?1 AND enabled=1",
                [source_id],
                |row| {
                    Ok(IncrementalFetchRequest {
                        source_id: row.get(0)?,
                        canonical_url: row.get(1)?,
                        expected_revision: sql_u64(row.get(2)?)?,
                        etag: row.get(3)?,
                        last_modified: row.get(4)?,
                        adapter_cursor: row.get(5)?,
                    })
                },
            )
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-fetch-prepare"))
    }

    /// Commits a prepared fetch only if its source identity and revision are still current.
    ///
    /// # Errors
    /// Returns a conflict for stale state or a stable validation/storage error.
    pub fn commit_incremental_fetch(
        &mut self,
        request: &IncrementalFetchRequest,
        result: &FetchIncrementalResult,
    ) -> Result<IncrementalApplyOutcome, AppError> {
        let current = self.prepare_incremental_fetch(&request.source_id)?;
        if current.canonical_url != request.canonical_url
            || current.expected_revision != request.expected_revision
        {
            return Err(
                source_error(ErrorCode::ConflictSourceRevision, "source-fetch-stale")
                    .with_source_id(&request.source_id),
            );
        }
        self.apply_incremental_result(&request.source_id, request.expected_revision, result)
    }

    /// Persists a fetch failure using the retry context carried by the production error.
    ///
    /// # Errors
    /// Returns a conflict for stale state or rejects unsupported error categories.
    pub fn commit_incremental_failure(
        &mut self,
        request: &IncrementalFetchRequest,
        error: &AppError,
        observed_at_ms: u64,
    ) -> Result<SourceViewV1, AppError> {
        let code = match error.code() {
            "rate_limited.source" => ErrorCode::RateLimitedSource,
            "network.source" => ErrorCode::NetworkSource,
            "source_format.rss_atom" => ErrorCode::SourceFormatRssAtom,
            _ => {
                return Err(source_error(
                    ErrorCode::ValidationSource,
                    "source-fetch-failure-code",
                ));
            }
        };
        self.record_source_failure(
            &request.source_id,
            request.expected_revision,
            code,
            error.retry_after_ms(),
            observed_at_ms,
        )
    }

    /// Returns the opaque conditional-fetch state owned by the core adapter.
    ///
    /// # Errors
    /// Returns a stable storage error when the source is absent or persisted state is corrupt.
    pub fn source_fetch_state(&self, source_id: &str) -> Result<SourceFetchState, AppError> {
        self.connection
            .query_row(
                "SELECT revision,etag,last_modified,adapter_cursor FROM sources WHERE source_id=?1",
                [source_id],
                |row| {
                    Ok(SourceFetchState {
                        revision: sql_u64(row.get(0)?)?,
                        etag: row.get(1)?,
                        last_modified: row.get(2)?,
                        adapter_cursor: row.get(3)?,
                    })
                },
            )
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-fetch-state"))
    }

    /// Validates a save intent and returns an already committed idempotent response when present.
    /// This permits response-loss retries to complete without another network request.
    ///
    /// # Errors
    /// Returns a stable validation, idempotency-conflict, or storage error.
    pub fn replay_saved_source(
        &self,
        input: &SaveSourceInputV1,
    ) -> Result<Option<SourceViewV1>, AppError> {
        validate_input(input)?;
        canonicalize_public_https_url(&input.url)?;
        let request_fingerprint = source_request_fingerprint(input)?;
        let existing: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT request_fingerprint,response_json FROM source_idempotency WHERE idempotency_key=?1",
                [&input.idempotency_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-idempotency-read"))?;
        let Some((fingerprint, json)) = existing else {
            return Ok(None);
        };
        if fingerprint != request_fingerprint {
            return Err(source_error(
                ErrorCode::ConflictSourceRevision,
                "source-idempotency-conflict",
            ));
        }
        serde_json::from_str(&json)
            .map(Some)
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-idempotency-json"))
    }

    /// Commits a source that has already passed the production network probe.
    /// Network work must happen before entering this short transaction.
    ///
    /// # Errors
    /// Returns stable validation, conflict, migration, or storage errors without endpoint details.
    pub fn save_probed_source(
        &mut self,
        input: &SaveSourceInputV1,
        probed: &ProbedSource,
    ) -> Result<SourceViewV1, AppError> {
        validate_input(input)?;
        let canonical = canonicalize_public_https_url(&input.url)?;
        if canonical.as_str() != probed.canonical_url {
            return Err(source_error(
                ErrorCode::ValidationSource,
                "source-probe-endpoint-mismatch",
            ));
        }
        let request_fingerprint = source_request_fingerprint(input)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-transaction"))?;
        let existing: Option<(String, String)> = transaction
            .query_row(
                "SELECT request_fingerprint,response_json FROM source_idempotency WHERE idempotency_key=?1",
                [&input.idempotency_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-idempotency-read"))?;
        if let Some((fingerprint, json)) = existing {
            if fingerprint != request_fingerprint {
                return Err(source_error(
                    ErrorCode::ConflictSourceRevision,
                    "source-idempotency-conflict",
                ));
            }
            return serde_json::from_str(&json)
                .map_err(|_| source_error(ErrorCode::StorageSource, "source-idempotency-json"));
        }

        let current = super::configuration::read_configuration(&transaction)?;
        if current.revision != input.expected_configuration_revision {
            return Err(source_error(
                ErrorCode::ConflictConfigurationRevision,
                "source-stale-configuration",
            ));
        }
        let mut configuration = current.configuration;
        let canonical_string = canonical.to_string();
        if let Some(preference) = configuration
            .source_preferences
            .iter_mut()
            .find(|preference| {
                preference.source_kind == "rss" && preference.identifier == canonical_string
            })
        {
            preference.enabled = true;
        } else {
            configuration.source_preferences.push(SourcePreferenceV1 {
                source_kind: "rss".to_owned(),
                identifier: canonical_string.clone(),
                enabled: true,
                trust: 80,
            });
        }
        let configuration_view = append_configuration_version(
            &transaction,
            &configuration,
            input.expected_configuration_revision,
        )?;
        let source_id = format!("source:{}", &digest_hex(canonical_string.as_bytes())[..24]);
        let now = now_ms()?;
        let now_sql = sql_i64(now)?;
        let configuration_sql = sql_i64(configuration_view.revision)?;
        transaction
            .execute(
                "INSERT INTO sources(source_id,configuration_version,source_kind,canonical_url,enabled,revision,status,consecutive_failures,retryability,created_at_ms,updated_at_ms)
                 VALUES(?1,?2,'rss_atom',?3,1,1,'ready',0,'never',?4,?4)
                 ON CONFLICT(canonical_url) DO UPDATE SET configuration_version=excluded.configuration_version,enabled=1,revision=sources.revision+1,status='ready',consecutive_failures=0,retryability='never',last_attempt_at_ms=NULL,next_allowed_at_ms=NULL,error_code=NULL,updated_at_ms=excluded.updated_at_ms",
                params![source_id, configuration_sql, canonical_string, now_sql],
            )
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-upsert"))?;
        let response = read_source_by_id(&transaction, &source_id)?;
        let response_json = serde_json::to_string(&response)
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-response-json"))?;
        transaction
            .execute(
                "INSERT INTO source_idempotency(idempotency_key,request_fingerprint,response_json) VALUES(?1,?2,?3)",
                params![input.idempotency_key, request_fingerprint, response_json],
            )
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-idempotency-write"))?;
        transaction
            .commit()
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-commit"))?;
        Ok(response)
    }

    /// Returns a stable keyset page ordered by opaque source identity.
    ///
    /// # Errors
    /// Returns a validation error for invalid pagination and a storage error for corrupt state.
    pub fn query_sources(
        &self,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<SourcePageV1, AppError> {
        if !(1..=MAX_PAGE_SIZE).contains(&limit) {
            return Err(source_error(
                ErrorCode::ValidationSource,
                "source-page-limit",
            ));
        }
        let after = cursor.map(decode_cursor).transpose()?.unwrap_or_default();
        let mut statement = self
            .connection
            .prepare(
                "SELECT source_id FROM sources WHERE source_id>?1 ORDER BY source_id ASC LIMIT ?2",
            )
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-page-prepare"))?;
        let ids = statement
            .query_map(params![after, i64::from(limit) + 1], |row| {
                row.get::<_, String>(0)
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-page-read"))?;
        let has_more = ids.len() > limit as usize;
        let page_ids = &ids[..ids.len().min(limit as usize)];
        let items = page_ids
            .iter()
            .map(|id| read_source_by_id(&self.connection, id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SourcePageV1 {
            contract_version: 1,
            items,
            next_cursor: if has_more {
                page_ids.last().map(|id| encode_cursor(id))
            } else {
                None
            },
        })
    }

    /// Atomically advances incremental metadata and content hashes after a successful fetch.
    ///
    /// # Errors
    /// Returns a conflict for stale source revision or a stable storage error.
    pub fn apply_incremental_result(
        &mut self,
        source_id: &str,
        expected_revision: u64,
        result: &FetchIncrementalResult,
    ) -> Result<IncrementalApplyOutcome, AppError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-result-transaction"))?;
        let outcome = apply_incremental_result_in_connection(
            &transaction,
            source_id,
            expected_revision,
            result,
        );
        if outcome.is_ok() {
            transaction
                .commit()
                .map_err(|_| source_error(ErrorCode::StorageSource, "source-result-commit"))?;
        }
        outcome.map_err(|error: AppError| error.with_source_id(source_id))
    }

    /// Records one source-scoped failure and its monotonic retry schedule.
    ///
    /// # Errors
    /// Returns a validation error for unsupported failure codes, a conflict for stale state,
    /// or a stable storage error.
    pub fn record_source_failure(
        &mut self,
        source_id: &str,
        expected_revision: u64,
        code: ErrorCode,
        retry_after_ms: Option<u64>,
        observed_at_ms: u64,
    ) -> Result<SourceViewV1, AppError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-failure-transaction"))?;
        let outcome = record_source_failure_in_connection(
            &transaction,
            source_id,
            expected_revision,
            code,
            retry_after_ms,
            observed_at_ms,
        );
        if outcome.is_ok() {
            transaction
                .commit()
                .map_err(|_| source_error(ErrorCode::StorageSource, "source-failure-commit"))?;
        }
        outcome.map_err(|error: AppError| error.with_source_id(source_id))
    }
}

pub(crate) fn apply_incremental_result_in_connection(
    connection: &Connection,
    source_id: &str,
    expected_revision: u64,
    result: &FetchIncrementalResult,
) -> Result<IncrementalApplyOutcome, AppError> {
    let current = read_source_by_id(connection, source_id)?;
    if current.revision != expected_revision {
        return Err(source_error(
            ErrorCode::ConflictSourceRevision,
            "source-result-revision",
        ));
    }
    if result.not_modified {
        if !result.candidates.is_empty() {
            return Err(source_error(
                ErrorCode::ValidationSource,
                "source-result-304-candidates",
            ));
        }
        let now_sql = sql_i64(now_ms()?)?;
        let changed = connection
            .execute(
                "UPDATE sources SET revision=revision+1,etag=COALESCE(?1,etag),last_modified=COALESCE(?2,last_modified),last_attempt_at_ms=?3,last_success_at_ms=?3,status='ready',consecutive_failures=0,retryability='never',next_allowed_at_ms=NULL,error_code=NULL,updated_at_ms=?3 WHERE source_id=?4 AND revision=?5",
                params![result.etag, result.last_modified, now_sql, source_id, sql_i64(expected_revision)?],
            )
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-result-304-update"))?;
        if changed != 1 {
            return Err(source_error(
                ErrorCode::ConflictSourceRevision,
                "source-result-304-race",
            ));
        }
        return Ok(IncrementalApplyOutcome {
            source: read_source_by_id(connection, source_id)?,
            candidates: Vec::new(),
        });
    }

    let now_sql = sql_i64(now_ms()?)?;
    let mut identities = HashSet::with_capacity(result.candidates.len());
    let mut dispositions = Vec::with_capacity(result.candidates.len());
    for candidate in &result.candidates {
        if !identities.insert(candidate.stable_external_id.as_str()) {
            return Err(source_error(
                ErrorCode::SourceFormatRssAtom,
                "source-result-duplicate-identity",
            ));
        }
        let previous_hash = connection
            .query_row(
                "SELECT content_hash FROM source_entry_checkpoints WHERE source_id=?1 AND stable_external_id=?2",
                params![source_id, candidate.stable_external_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| source_error(ErrorCode::StorageSource, "source-checkpoint-read"))?;
        dispositions.push(CandidateApplyResult {
            stable_external_id: candidate.stable_external_id.clone(),
            disposition: match previous_hash {
                None => CandidateDisposition::New,
                Some(hash) if hash == candidate.content_hash => CandidateDisposition::Unchanged,
                Some(_) => CandidateDisposition::Changed,
            },
        });
        connection.execute(
            "INSERT INTO source_entry_checkpoints(source_id,stable_external_id,content_hash,first_seen_at_ms,last_seen_at_ms)
             VALUES(?1,?2,?3,?4,?4)
             ON CONFLICT(source_id,stable_external_id) DO UPDATE SET content_hash=excluded.content_hash,last_seen_at_ms=MAX(source_entry_checkpoints.last_seen_at_ms,excluded.last_seen_at_ms)",
            params![source_id, candidate.stable_external_id, candidate.content_hash, now_sql],
        ).map_err(|_| source_error(ErrorCode::StorageSource, "source-checkpoint"))?;
    }
    let changed = connection.execute(
        "UPDATE sources SET revision=revision+1,etag=?1,last_modified=?2,adapter_cursor=?3,last_attempt_at_ms=?4,last_success_at_ms=?4,status='ready',consecutive_failures=0,retryability='never',next_allowed_at_ms=NULL,error_code=NULL,updated_at_ms=?4 WHERE source_id=?5 AND revision=?6",
        params![result.etag, result.last_modified, result.adapter_cursor, now_sql, source_id, sql_i64(expected_revision)?],
    ).map_err(|_| source_error(ErrorCode::StorageSource, "source-result-update"))?;
    if changed != 1 {
        return Err(source_error(
            ErrorCode::ConflictSourceRevision,
            "source-result-race",
        ));
    }
    Ok(IncrementalApplyOutcome {
        source: read_source_by_id(connection, source_id)?,
        candidates: dispositions,
    })
}

pub(crate) fn record_source_failure_in_connection(
    connection: &Connection,
    source_id: &str,
    expected_revision: u64,
    code: ErrorCode,
    retry_after_ms: Option<u64>,
    observed_at_ms: u64,
) -> Result<SourceViewV1, AppError> {
    let (status, retryability) = match code {
        ErrorCode::RateLimitedSource => ("retry_wait", "after"),
        ErrorCode::NetworkSource => ("retry_wait", "automatic"),
        ErrorCode::SourceFormatRssAtom => ("error", "never"),
        _ => {
            return Err(source_error(
                ErrorCode::ValidationSource,
                "source-failure-code",
            ));
        }
    };
    let current_failures: u32 = connection
        .query_row(
            "SELECT consecutive_failures FROM sources WHERE source_id=?1 AND revision=?2",
            params![source_id, sql_i64(expected_revision)?],
            |row| row.get(0),
        )
        .map_err(|_| source_error(ErrorCode::ConflictSourceRevision, "source-failure-revision"))?;
    let next_failures = current_failures.saturating_add(1);
    let next_allowed = if retryability == "never" {
        None
    } else {
        let delay = crate::infrastructure::http::source_http_policy::retry_delay_ms(
            next_failures,
            retry_after_ms,
        );
        Some(observed_at_ms.saturating_add(delay))
    };
    let changed = connection.execute(
        "UPDATE sources SET revision=revision+1,last_attempt_at_ms=?1,status=?2,consecutive_failures=?3,retryability=?4,next_allowed_at_ms=?5,error_code=?6,updated_at_ms=?1 WHERE source_id=?7 AND revision=?8",
        params![sql_i64(observed_at_ms)?, status, next_failures, retryability, next_allowed.map(sql_i64).transpose()?, code.as_str(), source_id, sql_i64(expected_revision)?],
    ).map_err(|_| source_error(ErrorCode::StorageSource, "source-failure-write"))?;
    if changed != 1 {
        return Err(source_error(
            ErrorCode::ConflictSourceRevision,
            "source-failure-race",
        ));
    }
    read_source_by_id(connection, source_id)
}

fn read_source_by_id(
    connection: &rusqlite::Connection,
    source_id: &str,
) -> Result<SourceViewV1, AppError> {
    connection.query_row(
        "SELECT source_id,source_kind,canonical_url,enabled,revision,created_at_ms,updated_at_ms,last_success_at_ms,status,retryability,next_allowed_at_ms FROM sources WHERE source_id=?1",
        [source_id],
        |row| {
            let canonical: String = row.get(2)?;
            let display_url = safe_display_url(&canonical).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let status: String = row.get(8)?;
            let retryability: String = row.get(9)?;
            Ok(SourceViewV1 {
                contract_version: 1,
                source_id: row.get(0)?,
                source_kind: row.get(1)?,
                display_url,
                enabled: row.get::<_, i64>(3)? == 1,
                revision: sql_u64(row.get(4)?)?,
                created_at: unix_ms_to_rfc3339(sql_u64(row.get(5)?)?),
                updated_at: unix_ms_to_rfc3339(sql_u64(row.get(6)?)?),
                last_success_at: row.get::<_, Option<i64>>(7)?.map(sql_u64).transpose()?.map(unix_ms_to_rfc3339),
                freshness: None,
                status: parse_status(&status)?,
                retryability: parse_retryability(&retryability)?,
                next_allowed_at: row.get::<_, Option<i64>>(10)?.map(sql_u64).transpose()?.map(unix_ms_to_rfc3339),
            })
        },
    ).map_err(|_| source_error(ErrorCode::StorageSource, "source-read"))
}

fn validate_input(input: &SaveSourceInputV1) -> Result<(), AppError> {
    if input.contract_version != 1
        || input.source_kind != "rss_atom"
        || input.idempotency_key.is_empty()
        || input.idempotency_key.len() > 128
        || !input
            .idempotency_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
    {
        return Err(source_error(ErrorCode::ValidationSource, "source-input"));
    }
    Ok(())
}

fn source_request_fingerprint(input: &SaveSourceInputV1) -> Result<String, AppError> {
    serde_json::to_vec(input)
        .map(|value| digest_hex(&value))
        .map_err(|_| source_error(ErrorCode::ValidationSource, "source-input-json"))
}

fn safe_display_url(value: &str) -> Result<String, AppError> {
    let mut url = canonicalize_public_https_url(value)?;
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

fn parse_status(value: &str) -> rusqlite::Result<SourceStatusV1> {
    match value {
        "ready" => Ok(SourceStatusV1::Ready),
        "error" => Ok(SourceStatusV1::Error),
        "retry_wait" => Ok(SourceStatusV1::RetryWait),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_retryability(value: &str) -> rusqlite::Result<SourceRetryabilityV1> {
    match value {
        "never" => Ok(SourceRetryabilityV1::Never),
        "manual" => Ok(SourceRetryabilityV1::Manual),
        "automatic" => Ok(SourceRetryabilityV1::Automatic),
        "after" => Ok(SourceRetryabilityV1::After),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn encode_cursor(id: &str) -> String {
    format!("source-v1:{id}:{}", &digest_hex(id.as_bytes())[..16])
}

fn decode_cursor(cursor: &str) -> Result<String, AppError> {
    let Some(body) = cursor.strip_prefix("source-v1:") else {
        return Err(source_error(ErrorCode::ValidationSource, "source-cursor"));
    };
    let Some((id, checksum)) = body.rsplit_once(':') else {
        return Err(source_error(ErrorCode::ValidationSource, "source-cursor"));
    };
    if cursor.len() > 180
        || checksum.len() != 16
        || checksum != &digest_hex(id.as_bytes())[..16]
        || !id.starts_with("source:")
    {
        return Err(source_error(ErrorCode::ValidationSource, "source-cursor"));
    }
    Ok(id.to_owned())
}

fn digest_hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn now_ms() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| source_error(ErrorCode::StorageSource, "source-time"))
        .and_then(|duration| {
            u64::try_from(duration.as_millis())
                .map_err(|_| source_error(ErrorCode::StorageSource, "source-time-range"))
        })
}

fn sql_i64(value: u64) -> Result<i64, AppError> {
    i64::try_from(value).map_err(|_| source_error(ErrorCode::StorageSource, "source-number-range"))
}
fn sql_u64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

pub(crate) fn unix_ms_to_rfc3339(milliseconds: u64) -> String {
    let seconds = milliseconds / 1_000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        day_seconds / 3600,
        day_seconds % 3600 / 60,
        day_seconds % 60,
        milliseconds % 1000
    )
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn source_error(code: ErrorCode, boundary: &'static str) -> AppError {
    AppError::from_code(code, boundary)
}
