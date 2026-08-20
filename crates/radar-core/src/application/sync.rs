//! Persistent RSS/Atom foreground synchronization orchestration.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use url::Url;

use super::demo::{
    Connection, DemoStore, OptionalExtension, SqlError, SqlResult, TransactionBehavior, sql_params,
};
use super::sources::{
    apply_incremental_result_in_connection, fetch_incremental, record_source_failure_in_connection,
};
use crate::contracts::dto::sync::{
    DeliveryReadinessStatusV1, GetSyncResultInputV1, SourceDeliveryReadinessV1,
    SourceReadinessStatusV1, SourceReadinessV1, SourceSyncStatusV1, StartSyncInputV1,
    SyncHealthSummaryV1, SyncResultCountsV1, SyncResultDispositionV1, SyncResultItemV1,
    SyncResultPageV1, SyncResultSummaryV1, SyncRunIdV1, SyncRunOutcomeV1, SyncSourceResultV1,
    SyncTargetV1, TaskRefV1, TaskSnapshotV1, TaskStateV1,
};
use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::intel::{NormalizationIssue, NormalizedIntelCandidate, normalize_rss_candidate};
use crate::domain::sources::{
    CandidateDisposition, FetchIncrementalResult, IncrementalFetchRequest, RawSourceCandidate,
};

const MAX_SYNC_SOURCES: usize = 64;
const MAX_FOREGROUND_BUDGET_MS: u32 = 30_000;
const MAX_TERMINAL_TASK_HISTORY: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncExecutionPlan {
    pub task: TaskRefV1,
    pub foreground_budget_ms: u32,
    pub sources: Vec<IncrementalFetchRequest>,
}

/// Executes only the existing Story 2.2 network/parser seam. Callers must not hold a store lock.
///
/// # Errors
/// Returns the existing stable source-scoped network, rate-limit, or format error.
pub async fn fetch_sync_source(
    request: &IncrementalFetchRequest,
) -> Result<FetchIncrementalResult, AppError> {
    fetch_incremental(request).await
}

impl DemoStore {
    /// Persists one bounded RSS-only foreground synchronization intent.
    ///
    /// # Errors
    /// Returns stable validation, conflict, or storage errors.
    pub fn start_sync(&mut self, input: &StartSyncInputV1) -> Result<TaskRefV1, AppError> {
        validate_start_input(input)?;
        let fingerprint = fingerprint(input)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-start-transaction"))?;

        if let Some((existing_fingerprint, task_id)) = transaction
            .query_row(
                "SELECT request_fingerprint,task_id FROM jobs WHERE idempotency_key=?1",
                [&input.idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-idempotency-read"))?
        {
            if existing_fingerprint != fingerprint {
                return Err(sync_error(
                    ErrorCode::ConflictSourceRevision,
                    "sync-idempotency-conflict",
                )
                .with_task_id(task_id));
            }
            return read_task_ref(&transaction, &task_id);
        }

        let source_ids = select_target_sources(&transaction, &input.target)?;
        if source_ids.is_empty() || source_ids.len() > MAX_SYNC_SOURCES {
            return Err(sync_error(
                ErrorCode::ValidationSource,
                "sync-target-empty-or-unbounded",
            ));
        }
        let now = now_ms()?;
        expire_elapsed_retry_wait_tasks(&transaction, &source_ids, now)?;
        if let Some(active_task_id) = find_active_task(&transaction, &source_ids)? {
            return Err(sync_error(
                ErrorCode::ConflictSourceRevision,
                "sync-source-already-active",
            )
            .with_task_id(active_task_id));
        }

        let task_id = format!("task:{}", &digest_hex(fingerprint.as_bytes())[..24]);
        let sync_run_id = format!("run:{}", &digest_hex(task_id.as_bytes())[..24]);
        let (target_kind, target_source_id) = encode_target(&input.target);
        transaction
            .execute(
                "INSERT INTO jobs(task_id,kind,target_kind,target_source_id,state,revision,idempotency_key,request_fingerprint,foreground_budget_ms,created_at_ms,updated_at_ms)
                 VALUES(?1,'rss_atom_sync',?2,?3,'queued',1,?4,?5,?6,?7,?7)",
                sql_params![
                    task_id,
                    target_kind,
                    target_source_id,
                    input.idempotency_key,
                    fingerprint,
                    i64::from(input.foreground_budget_ms),
                    sql_i64(now)?,
                ],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-job-insert"))?;
        transaction
            .execute(
                "INSERT INTO sync_runs(sync_run_id,task_id,scope,started_at_ms) VALUES(?1,?2,?3,?4)",
                sql_params![sync_run_id, task_id, target_kind, sql_i64(now)?],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-run-insert"))?;
        for source_id in source_ids {
            let (source_revision, canonical_url): (i64, String) = transaction
                .query_row(
                    "SELECT revision,canonical_url FROM sources WHERE source_id=?1 AND source_kind='rss_atom' AND enabled=1",
                    [&source_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-source-freeze"))?;
            transaction
                .execute(
                    "INSERT INTO job_source_states(task_id,source_id,source_revision,state,updated_at_ms)
                     VALUES(?1,?2,?3,'queued',?4)",
                    sql_params![task_id, source_id, source_revision, sql_i64(now)?],
                )
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-source-state-insert"))?;
            transaction
                .execute(
                    "INSERT INTO sync_source_results(sync_run_id,source_id,source_revision,source_kind,publisher,status)
                     VALUES(?1,?2,?3,'rss_atom',?4,'queued')",
                    sql_params![
                        sync_run_id,
                        source_id,
                        source_revision,
                        publisher_from_url(&canonical_url)?,
                    ],
                )
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-source-result-insert"))?;
        }
        prune_terminal_sync_history(&transaction)?;
        transaction
            .commit()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-start-commit"))?;
        self.task_ref(&task_id)
    }

    /// Returns the stable lightweight reference used to start or poll a task.
    ///
    /// # Errors
    /// Returns a stable storage error for an absent or corrupt task.
    pub fn task_ref(&self, task_id: &str) -> Result<TaskRefV1, AppError> {
        validate_task_id(task_id)?;
        read_task_ref(&self.connection, task_id)
    }

    /// Returns a complete task/source status projection without endpoint or content data.
    ///
    /// # Errors
    /// Returns a stable storage error for an absent or corrupt task.
    pub fn task_snapshot(&self, task_id: &str) -> Result<TaskSnapshotV1, AppError> {
        validate_task_id(task_id)?;
        read_task_snapshot(&self.connection, task_id)
    }

    /// Atomically claims a queued task and freezes the existing Story 2.2 fetch requests.
    /// Network I/O starts only after this method returns and the caller releases its store lock.
    ///
    /// # Errors
    /// Returns a conflict for stale task revision or stable storage errors.
    #[allow(clippy::too_many_lines)] // Claim, retry-deadline filtering, and frozen fetch projection are one atomic audit boundary.
    pub fn claim_sync_task(
        &mut self,
        task_id: &str,
        expected_revision: u64,
    ) -> Result<SyncExecutionPlan, AppError> {
        validate_task_id(task_id)?;
        let now = now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-claim-transaction"))?;
        let changed = transaction
            .execute(
                "UPDATE jobs SET state='running',revision=revision+1,started_at_ms=?1,updated_at_ms=?1
                 WHERE task_id=?2 AND revision=?3 AND state IN ('queued','retry_wait')",
                sql_params![sql_i64(now)?, task_id, sql_i64(expected_revision)?],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-claim-job"))?;
        if changed != 1 {
            return Err(
                sync_error(ErrorCode::ConflictSourceRevision, "sync-claim-revision")
                    .with_task_id(task_id),
            );
        }
        transaction
            .execute(
                "UPDATE job_source_states SET state='running',updated_at_ms=?1
                 WHERE task_id=?2 AND state IN ('queued','retry_wait')",
                sql_params![sql_i64(now)?, task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-claim-sources"))?;
        transaction
            .execute(
                "UPDATE sync_source_results SET status='running'
                 WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?1) AND status IN ('queued','retry_wait')",
                [task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-claim-results"))?;

        let budget: u32 = transaction
            .query_row(
                "SELECT foreground_budget_ms FROM jobs WHERE task_id=?1",
                [task_id],
                |row| row.get(0),
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-claim-budget"))?;
        let source_entries = task_source_entries(&transaction, task_id)?;
        let mut requests = Vec::with_capacity(source_entries.len());
        for (source_id, frozen_revision) in source_entries {
            let row = transaction
                .query_row(
                    "SELECT canonical_url,revision,etag,last_modified,adapter_cursor,next_allowed_at_ms,enabled
                     FROM sources WHERE source_id=?1 AND source_kind='rss_atom'",
                    [&source_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<i64>>(5)?,
                            row.get::<_, i64>(6)?,
                        ))
                    },
                )
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-claim-source"))?;
            if sql_u64_app(row.1)? != frozen_revision {
                transition_source_row(
                    &transaction,
                    task_id,
                    &source_id,
                    "failed",
                    row.1,
                    None,
                    Some(ErrorCode::ConflictSourceRevision.as_str()),
                    None,
                    now,
                )?;
                continue;
            }
            if row.6 != 1 {
                transition_source_row(
                    &transaction,
                    task_id,
                    &source_id,
                    "failed",
                    row.1,
                    None,
                    Some("validation.source"),
                    None,
                    now,
                )?;
                continue;
            }
            if row
                .5
                .is_some_and(|deadline| deadline > i64::try_from(now).unwrap_or(i64::MAX))
            {
                transition_source_row(
                    &transaction,
                    task_id,
                    &source_id,
                    "retry_wait",
                    row.1,
                    None,
                    Some("rate_limited.source"),
                    row.5,
                    now,
                )?;
                continue;
            }
            requests.push(IncrementalFetchRequest {
                source_id,
                canonical_url: row.0,
                expected_revision: sql_u64_app(row.1)?,
                etag: row.2,
                last_modified: row.3,
                adapter_cursor: row.4,
            });
        }
        recompute_job(&transaction, task_id, expected_revision + 1, now, false)?;
        transaction
            .commit()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-claim-commit"))?;
        Ok(SyncExecutionPlan {
            task: self.task_ref(task_id)?,
            foreground_budget_ms: budget,
            sources: requests,
        })
    }

    /// Commits the existing Story 2.2 checkpoint first, then advances the task projection by CAS.
    ///
    /// # Errors
    /// Returns the existing source error or a stale task conflict.
    pub fn commit_sync_source_success(
        &mut self,
        task_id: &str,
        expected_task_revision: u64,
        request: &IncrementalFetchRequest,
        result: &FetchIncrementalResult,
    ) -> Result<TaskRefV1, AppError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-success-transaction"))?;
        require_task_revision(
            &transaction,
            task_id,
            expected_task_revision,
            &request.source_id,
        )?;
        let publisher = publisher_from_url(&request.canonical_url)?;
        let collected_at = unix_ms_to_rfc3339(now_ms()?);
        let (eligible_result, normalized, issues) =
            normalize_result_candidates(result, &request.source_id, &publisher, &collected_at);
        if result.not_modified && !result.candidates.is_empty() {
            return Err(sync_error(
                ErrorCode::ValidationSource,
                "source-result-304-candidates",
            ));
        }
        if normalized.is_empty() && !issues.is_empty() {
            let observed_at_ms = now_ms()?;
            let source = record_source_failure_in_connection(
                &transaction,
                &request.source_id,
                request.expected_revision,
                ErrorCode::SourceFormatRssAtom,
                None,
                observed_at_ms,
            )
            .map_err(|error| error.with_source_id(&request.source_id))?;
            persist_normalization_failure_projection(
                &transaction,
                task_id,
                &request.source_id,
                source.revision,
                &issues,
            )?;
            commit_job_source_state_in_connection(
                &transaction,
                task_id,
                expected_task_revision,
                &request.source_id,
                source.revision,
                "failed",
                source.last_success_at.as_deref(),
                Some(ErrorCode::SourceFormatRssAtom.as_str()),
                None,
            )?;
            transaction.commit().map_err(|_| {
                sync_error(
                    ErrorCode::StorageSource,
                    "sync-normalization-failure-commit",
                )
            })?;
            return self.task_ref(task_id);
        }
        let applied = apply_incremental_result_in_connection(
            &transaction,
            &request.source_id,
            request.expected_revision,
            &eligible_result,
        )
        .map_err(|error| error.with_source_id(&request.source_id))?;
        let counts = persist_success_result_projection(
            &transaction,
            task_id,
            request,
            &normalized,
            &applied.candidates,
            &issues,
        )?;
        let has_consumable_candidate = counts[0] + counts[1] + counts[2] > 0;
        let source_state = if counts[3] > 0 && !has_consumable_candidate {
            "failed"
        } else {
            "succeeded"
        };
        commit_job_source_state_in_connection(
            &transaction,
            task_id,
            expected_task_revision,
            &request.source_id,
            applied.source.revision,
            source_state,
            applied.source.last_success_at.as_deref(),
            (source_state == "failed").then_some("source_format.rss_atom"),
            None,
        )?;
        transaction
            .commit()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-success-commit"))?;
        self.task_ref(task_id)
    }

    /// Commits the existing Story 2.2 retry/failure state first, then advances the task projection.
    ///
    /// # Errors
    /// Returns the existing source error or a stale task conflict.
    pub fn commit_sync_source_failure(
        &mut self,
        task_id: &str,
        expected_task_revision: u64,
        request: &IncrementalFetchRequest,
        error: &AppError,
        observed_at_ms: u64,
    ) -> Result<TaskRefV1, AppError> {
        let code = match error.code() {
            "rate_limited.source" => ErrorCode::RateLimitedSource,
            "network.source" => ErrorCode::NetworkSource,
            "source_format.rss_atom" => ErrorCode::SourceFormatRssAtom,
            _ => {
                return Err(sync_error(
                    ErrorCode::ValidationSource,
                    "sync-source-failure-code",
                ));
            }
        };
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-failure-transaction"))?;
        require_task_revision(
            &transaction,
            task_id,
            expected_task_revision,
            &request.source_id,
        )?;
        let source = record_source_failure_in_connection(
            &transaction,
            &request.source_id,
            request.expected_revision,
            code,
            error.retry_after_ms(),
            observed_at_ms,
        )
        .map_err(|failure| failure.with_source_id(&request.source_id))?;
        let state = if source.next_allowed_at.is_some() {
            "retry_wait"
        } else {
            "failed"
        };
        persist_failure_result_projection(
            &transaction,
            task_id,
            &request.source_id,
            state,
            error.code(),
        )?;
        commit_job_source_state_in_connection(
            &transaction,
            task_id,
            expected_task_revision,
            &request.source_id,
            source.revision,
            state,
            source.last_success_at.as_deref(),
            Some(error.code()),
            source.next_allowed_at.as_deref(),
        )?;
        transaction
            .commit()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-failure-commit"))?;
        self.task_ref(task_id)
    }

    /// Records a worker/runtime failure only on the task projection.
    ///
    /// This deliberately does not call the Story 2.2 source failure seam: a panic or join error
    /// is not evidence that the source endpoint failed, and must not change its checkpoint,
    /// retry deadline, or source health.
    ///
    /// # Errors
    /// Returns a stale task/source transition conflict or a stable storage error.
    pub fn commit_sync_source_internal_failure(
        &mut self,
        task_id: &str,
        expected_task_revision: u64,
        request: &IncrementalFetchRequest,
    ) -> Result<TaskRefV1, AppError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-internal-transaction"))?;
        require_task_revision(
            &transaction,
            task_id,
            expected_task_revision,
            &request.source_id,
        )?;
        persist_failure_result_projection(
            &transaction,
            task_id,
            &request.source_id,
            "failed",
            ErrorCode::InternalUnexpected.as_str(),
        )?;
        commit_job_source_state_in_connection(
            &transaction,
            task_id,
            expected_task_revision,
            &request.source_id,
            request.expected_revision,
            "failed",
            None,
            Some(ErrorCode::InternalUnexpected.as_str()),
            None,
        )?;
        transaction
            .commit()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-internal-commit"))?;
        self.task_ref(task_id)
    }

    /// Fails every still-active source in a task after its detached worker exits unexpectedly.
    ///
    /// # Errors
    /// Returns a stable storage error. A task already in a terminal state is returned unchanged.
    pub fn fail_active_sync_task(&mut self, task_id: &str) -> Result<TaskRefV1, AppError> {
        validate_task_id(task_id)?;
        let now = now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-worker-fail-transaction"))?;
        transaction
            .execute(
                "UPDATE job_source_states SET state='failed',error_code='internal.unexpected',next_allowed_at_ms=NULL,updated_at_ms=?1
                 WHERE task_id=?2 AND state IN ('queued','running','retry_wait')",
                sql_params![sql_i64(now)?, task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-worker-fail-sources"))?;
        let changed = transaction
            .execute(
                "UPDATE jobs SET state='failed',revision=revision+1,started_at_ms=COALESCE(started_at_ms,?1),finished_at_ms=?1,updated_at_ms=?1,error_summary='internal.unexpected'
                 WHERE task_id=?2 AND state IN ('queued','running','retry_wait')",
                sql_params![sql_i64(now)?, task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-worker-fail-job"))?;
        if changed == 0 {
            transaction
                .commit()
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-worker-fail-commit"))?;
            return self.task_ref(task_id);
        }
        transaction
            .execute(
                "UPDATE sync_source_results SET status='failed',failed_count=CASE WHEN failed_count=0 THEN 1 ELSE failed_count END,error_code='internal.unexpected'
                 WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?1) AND status IN ('queued','running','retry_wait')",
                [task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-worker-fail-results"))?;
        recompute_sync_run(&transaction, task_id, true, now)?;
        transaction
            .commit()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-worker-fail-commit"))?;
        self.task_ref(task_id)
    }

    /// Cancels remaining task work when the fixed foreground execution budget is exhausted.
    ///
    /// # Errors
    /// Returns a conflict for a stale task revision or a stable storage error.
    pub fn cancel_sync_task_for_budget(&mut self, task_id: &str) -> Result<TaskRefV1, AppError> {
        validate_task_id(task_id)?;
        let now = now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-budget-transaction"))?;
        transaction
            .execute(
                "UPDATE job_source_states SET state='cancelled',error_code='internal.unexpected',next_allowed_at_ms=NULL,updated_at_ms=?1
                 WHERE task_id=?2 AND state IN ('queued','running','retry_wait')",
                sql_params![sql_i64(now)?, task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-budget-sources"))?;
        let changed = transaction
            .execute(
                "UPDATE jobs SET state='cancelled',revision=revision+1,finished_at_ms=?1,updated_at_ms=?1,error_summary='internal.unexpected'
                 WHERE task_id=?2 AND state IN ('queued','running','retry_wait')",
                sql_params![sql_i64(now)?, task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-budget-job"))?;
        if changed != 1 {
            return Err(
                sync_error(ErrorCode::ConflictSourceRevision, "sync-budget-revision")
                    .with_task_id(task_id),
            );
        }
        transaction
            .execute(
                "UPDATE sync_source_results SET status='cancelled',failed_count=CASE WHEN failed_count=0 THEN 1 ELSE failed_count END,error_code='internal.unexpected'
                 WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?1) AND status IN ('queued','running','retry_wait')",
                [task_id],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-budget-results"))?;
        recompute_sync_run(&transaction, task_id, true, now)?;
        transaction
            .commit()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-budget-commit"))?;
        self.task_ref(task_id)
    }

    /// Computes the RSS-only task and delivery-readiness projection.
    ///
    /// # Errors
    /// Returns a stable storage error for corrupt persisted state.
    pub fn sync_health(&self) -> Result<SyncHealthSummaryV1, AppError> {
        let latest_task_id = self
            .connection
            .query_row(
                "SELECT task_id FROM jobs
                 ORDER BY CASE WHEN state IN ('queued','running','retry_wait') THEN 0 ELSE 1 END,
                          updated_at_ms DESC,task_id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-health-latest"))?;
        let latest_task = latest_task_id
            .as_deref()
            .map(|task_id| read_task_snapshot(&self.connection, task_id))
            .transpose()?;
        let pending_task_count: u32 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE state IN ('queued','running','retry_wait')",
                [],
                |row| row.get(0),
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-health-pending"))?;
        let readiness = read_readiness(&self.connection)?;
        let last_success_at = readiness
            .sources
            .iter()
            .filter_map(|source| source.last_success_at.as_deref())
            .max()
            .map(str::to_owned);
        let source_results = read_health_source_results(&self.connection, latest_task.as_ref())?;
        Ok(SyncHealthSummaryV1 {
            contract_version: 1,
            source_results,
            latest_task,
            pending_task_count,
            freshness: last_success_at.as_ref().map(|_| "available".to_owned()),
            last_success_at,
            readiness,
        })
    }

    /// Reads one committed synchronization result without network or AI work.
    ///
    /// # Errors
    /// Returns stable validation/storage errors for an invalid identity, cursor, limit, or
    /// incomplete/corrupt result projection.
    pub fn get_sync_result(
        &self,
        input: &GetSyncResultInputV1,
    ) -> Result<SyncResultPageV1, AppError> {
        validate_result_input(input)?;
        read_sync_result_page(&self.connection, input)
    }
}

fn require_task_revision(
    connection: &Connection,
    task_id: &str,
    expected_task_revision: u64,
    source_id: &str,
) -> Result<(), AppError> {
    let current_revision: i64 = connection
        .query_row(
            "SELECT revision FROM jobs WHERE task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-source-task-read"))?;
    if sql_u64_app(current_revision)? == expected_task_revision {
        return Ok(());
    }
    let mut error = sync_error(
        ErrorCode::ConflictSourceRevision,
        "sync-source-task-revision",
    )
    .with_task_id(task_id);
    if !source_id.is_empty() {
        error = error.with_source_id(source_id);
    }
    Err(error)
}

#[allow(clippy::too_many_arguments)]
fn commit_job_source_state_in_connection(
    connection: &Connection,
    task_id: &str,
    expected_task_revision: u64,
    source_id: &str,
    source_revision: u64,
    state: &str,
    last_success_at: Option<&str>,
    error_code: Option<&str>,
    next_allowed_at: Option<&str>,
) -> Result<(), AppError> {
    let now = now_ms()?;
    let last_success_at_ms = last_success_at.map(parse_rfc3339_ms).transpose()?;
    let next_allowed_at_ms = next_allowed_at.map(parse_rfc3339_ms).transpose()?;
    transition_source_row(
        connection,
        task_id,
        source_id,
        state,
        sql_i64(source_revision)?,
        last_success_at_ms.map(sql_i64).transpose()?,
        error_code,
        next_allowed_at_ms.map(sql_i64).transpose()?,
        now,
    )?;
    recompute_job(connection, task_id, expected_task_revision, now, true)
}

pub(crate) fn recover_interrupted_sync_jobs(connection: &Connection) -> Result<(), AppError> {
    let now = now_ms()?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-recovery-transaction"))?;
    let mut statement = transaction
        .prepare("SELECT task_id FROM jobs WHERE state IN ('queued','running') ORDER BY task_id")
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-recovery-task-prepare"))?;
    let affected_task_ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-recovery-task-read"))?;
    drop(statement);
    transaction
        .execute(
            "UPDATE job_source_states SET state='failed',error_code='internal.unexpected',next_allowed_at_ms=NULL,updated_at_ms=?1
             WHERE state IN ('queued','running')",
            [sql_i64(now)?],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-recovery-sources"))?;
    transaction
        .execute(
            "UPDATE jobs SET state='failed',revision=revision+1,started_at_ms=COALESCE(started_at_ms,?1),finished_at_ms=?1,updated_at_ms=?1,error_summary='internal.unexpected'
             WHERE state IN ('queued','running')",
            [sql_i64(now)?],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-recovery-jobs"))?;
    transaction
        .execute(
            "UPDATE sync_source_results SET status='failed',failed_count=CASE WHEN failed_count=0 THEN 1 ELSE failed_count END,error_code='internal.unexpected'
             WHERE status IN ('queued','running')",
            [],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-recovery-results"))?;
    for task_id in affected_task_ids {
        recompute_sync_run(&transaction, &task_id, true, now)?;
    }
    transaction
        .commit()
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-recovery-commit"))
}

fn validate_start_input(input: &StartSyncInputV1) -> Result<(), AppError> {
    if input.contract_version != 1
        || input.foreground_budget_ms != MAX_FOREGROUND_BUDGET_MS
        || !is_safe_id(&input.idempotency_key)
        || matches!(&input.target, SyncTargetV1::SourceId { source_id } if !is_safe_source_id(source_id))
    {
        return Err(sync_error(ErrorCode::ValidationSource, "sync-start-input"));
    }
    Ok(())
}

fn validate_task_id(task_id: &str) -> Result<(), AppError> {
    if task_id.len() != 29
        || !task_id.starts_with("task:")
        || !task_id[5..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(sync_error(ErrorCode::ValidationSource, "sync-task-id"));
    }
    Ok(())
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn is_safe_source_id(value: &str) -> bool {
    value.starts_with("source:") && is_safe_id(value)
}

fn fingerprint(input: &StartSyncInputV1) -> Result<String, AppError> {
    serde_json::to_vec(input)
        .map(|bytes| digest_hex(&bytes))
        .map_err(|_| sync_error(ErrorCode::ValidationSource, "sync-fingerprint"))
}

fn encode_target(target: &SyncTargetV1) -> (&'static str, Option<&str>) {
    match target {
        SyncTargetV1::AllEnabledRssAtom => ("all_enabled_rss_atom", None),
        SyncTargetV1::SourceId { source_id } => ("source_id", Some(source_id)),
    }
}

fn decode_target(kind: &str, source_id: Option<String>) -> SqlResult<SyncTargetV1> {
    match (kind, source_id) {
        ("all_enabled_rss_atom", None) => Ok(SyncTargetV1::AllEnabledRssAtom),
        ("source_id", Some(source_id)) if is_safe_source_id(&source_id) => {
            Ok(SyncTargetV1::SourceId { source_id })
        }
        _ => Err(SqlError::InvalidQuery),
    }
}

fn select_target_sources(
    connection: &Connection,
    target: &SyncTargetV1,
) -> Result<Vec<String>, AppError> {
    let (sql, parameter) = match target {
        SyncTargetV1::AllEnabledRssAtom => (
            "SELECT source_id FROM sources WHERE source_kind='rss_atom' AND enabled=1 ORDER BY source_id LIMIT 65",
            None,
        ),
        SyncTargetV1::SourceId { source_id } => (
            "SELECT source_id FROM sources WHERE source_kind='rss_atom' AND enabled=1 AND source_id=?1",
            Some(source_id.as_str()),
        ),
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-target-prepare"))?;
    let rows = if let Some(parameter) = parameter {
        statement
            .query_map([parameter], |row| row.get::<_, String>(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
    } else {
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
    }
    .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-target-read"))?;
    Ok(rows)
}

fn find_active_task(
    connection: &Connection,
    source_ids: &[String],
) -> Result<Option<String>, AppError> {
    for source_id in source_ids {
        if let Some(task_id) = connection
            .query_row(
                "SELECT s.task_id FROM job_source_states s JOIN jobs j ON j.task_id=s.task_id
                 WHERE s.source_id=?1 AND s.state IN ('queued','running','retry_wait')
                   AND j.state IN ('queued','running','retry_wait') LIMIT 1",
                [source_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-active-read"))?
        {
            return Ok(Some(task_id));
        }
    }
    Ok(None)
}

fn expire_elapsed_retry_wait_tasks(
    connection: &Connection,
    source_ids: &[String],
    now: u64,
) -> Result<(), AppError> {
    let mut affected_tasks = HashSet::new();
    for source_id in source_ids {
        let mut statement = connection
            .prepare(
                "SELECT s.task_id FROM job_source_states s JOIN jobs j ON j.task_id=s.task_id
                 WHERE s.source_id=?1 AND s.state='retry_wait' AND s.next_allowed_at_ms<=?2
                   AND j.state='retry_wait'",
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-retry-expire-prepare"))?;
        let task_ids = statement
            .query_map(sql_params![source_id, sql_i64(now)?], |row| {
                row.get::<_, String>(0)
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-retry-expire-read"))?;
        for task_id in task_ids {
            let changed = connection
                .execute(
                    "UPDATE job_source_states SET state='failed',error_code='rate_limited.source',next_allowed_at_ms=NULL,updated_at_ms=?1
                     WHERE task_id=?2 AND source_id=?3 AND state='retry_wait' AND next_allowed_at_ms<=?1",
                    sql_params![sql_i64(now)?, task_id, source_id],
                )
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-retry-expire-source"))?;
            if changed == 1 {
                connection
                    .execute(
                        "UPDATE sync_source_results SET status='failed',error_code='rate_limited.source'
                         WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?1) AND source_id=?2",
                        sql_params![task_id, source_id],
                    )
                    .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-retry-expire-result"))?;
                affected_tasks.insert(task_id);
            }
        }
    }
    for task_id in affected_tasks {
        let revision: i64 = connection
            .query_row(
                "SELECT revision FROM jobs WHERE task_id=?1",
                [&task_id],
                |row| row.get(0),
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-retry-expire-job"))?;
        recompute_job(connection, &task_id, sql_u64_app(revision)?, now, true)?;
    }
    Ok(())
}

fn prune_terminal_sync_history(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "DELETE FROM jobs
             WHERE state IN ('succeeded','partially_succeeded','failed','cancelled')
               AND task_id NOT IN (
                 SELECT task_id FROM jobs
                 WHERE state IN ('succeeded','partially_succeeded','failed','cancelled')
                 ORDER BY updated_at_ms DESC,task_id DESC LIMIT ?1
               )",
            [i64::try_from(MAX_TERMINAL_TASK_HISTORY)
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-history-limit"))?],
        )
        .map(|_| ())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-history-prune"))
}

fn read_task_ref(connection: &Connection, task_id: &str) -> Result<TaskRefV1, AppError> {
    connection
        .query_row(
            "SELECT state,revision FROM jobs WHERE task_id=?1 AND kind='rss_atom_sync'",
            [task_id],
            |row| {
                Ok(TaskRefV1 {
                    contract_version: 1,
                    task_id: task_id.to_owned(),
                    state: parse_task_state(&row.get::<_, String>(0)?)?,
                    revision: sql_u64(row.get(1)?)?,
                })
            },
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-task-ref").with_task_id(task_id))
}

fn read_task_snapshot(connection: &Connection, task_id: &str) -> Result<TaskSnapshotV1, AppError> {
    let mut snapshot = connection
        .query_row(
            "SELECT j.target_kind,j.target_source_id,j.state,j.revision,j.created_at_ms,j.started_at_ms,j.finished_at_ms,j.updated_at_ms,j.error_summary,r.sync_run_id
             FROM jobs j LEFT JOIN sync_runs r ON r.task_id=j.task_id WHERE j.task_id=?1 AND j.kind='rss_atom_sync'",
            [task_id],
            |row| {
                Ok(TaskSnapshotV1 {
                    contract_version: 1,
                    task_id: task_id.to_owned(),
                    target: decode_target(&row.get::<_, String>(0)?, row.get(1)?)?,
                    state: parse_task_state(&row.get::<_, String>(2)?)?,
                    revision: sql_u64(row.get(3)?)?,
                    created_at: timestamp(row.get(4)?)?,
                    started_at: optional_timestamp(row.get(5)?)?,
                    finished_at: optional_timestamp(row.get(6)?)?,
                    updated_at: timestamp(row.get(7)?)?,
                    error_summary: row.get(8)?,
                    result_ref: row
                        .get::<_, Option<String>>(9)?
                        .map(|value| SyncRunIdV1::parse(value).ok_or(SqlError::InvalidQuery))
                        .transpose()?,
                    sources: Vec::new(),
                })
            },
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-task-snapshot").with_task_id(task_id))?;
    snapshot.sources = read_task_sources(connection, task_id)?;
    Ok(snapshot)
}

fn read_task_sources(
    connection: &Connection,
    task_id: &str,
) -> Result<Vec<SourceSyncStatusV1>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT source_id,source_revision,state,last_success_at_ms,error_code,next_allowed_at_ms,updated_at_ms
             FROM job_source_states WHERE task_id=?1 ORDER BY source_id",
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-task-sources-prepare"))?;
    statement
        .query_map([task_id], |row| {
            Ok(SourceSyncStatusV1 {
                contract_version: 1,
                source_id: row.get(0)?,
                source_revision: sql_u64(row.get(1)?)?,
                state: parse_task_state(&row.get::<_, String>(2)?)?,
                last_success_at: optional_timestamp(row.get(3)?)?,
                error_code: row.get(4)?,
                next_allowed_at: optional_timestamp(row.get(5)?)?,
                updated_at: timestamp(row.get(6)?)?,
            })
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-task-sources-read"))
}

fn read_health_source_results(
    connection: &Connection,
    latest_task: Option<&TaskSnapshotV1>,
) -> Result<Vec<SourceSyncStatusV1>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT s.task_id FROM job_source_states s JOIN jobs j ON j.task_id=s.task_id
             WHERE s.state IN ('queued','running','retry_wait')
               AND j.state IN ('queued','running','retry_wait')
             ORDER BY s.updated_at_ms DESC,s.task_id DESC",
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-health-sources-prepare"))?;
    let active_task_ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-health-sources-read"))?;
    let mut seen_sources = HashSet::new();
    let mut results = Vec::new();
    for task_id in active_task_ids {
        for source in read_task_sources(connection, &task_id)? {
            if seen_sources.insert(source.source_id.clone()) {
                results.push(source);
            }
        }
    }
    if let Some(task) = latest_task {
        for source in &task.sources {
            if seen_sources.insert(source.source_id.clone()) {
                results.push(source.clone());
            }
        }
    }
    results.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    Ok(results)
}

fn task_source_entries(
    connection: &Connection,
    task_id: &str,
) -> Result<Vec<(String, u64)>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT source_id,source_revision FROM job_source_states WHERE task_id=?1 ORDER BY source_id",
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-source-ids-prepare"))?;
    statement
        .query_map([task_id], |row| {
            Ok((row.get::<_, String>(0)?, sql_u64(row.get(1)?)?))
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-source-ids-read"))
}

#[allow(clippy::too_many_arguments)]
fn transition_source_row(
    connection: &Connection,
    task_id: &str,
    source_id: &str,
    state: &str,
    source_revision: i64,
    last_success_at_ms: Option<i64>,
    error_code: Option<&str>,
    next_allowed_at_ms: Option<i64>,
    now: u64,
) -> Result<(), AppError> {
    let changed = connection
        .execute(
            "UPDATE job_source_states SET source_revision=?1,state=?2,last_success_at_ms=?3,error_code=?4,next_allowed_at_ms=?5,updated_at_ms=?6
             WHERE task_id=?7 AND source_id=?8 AND state IN ('queued','running','retry_wait')",
            sql_params![
                source_revision,
                state,
                last_success_at_ms,
                error_code,
                next_allowed_at_ms,
                sql_i64(now)?,
                task_id,
                source_id,
            ],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-source-state-write"))?;
    if changed != 1 {
        return Err(sync_error(
            ErrorCode::ConflictSourceRevision,
            "sync-source-state-transition",
        )
        .with_task_id(task_id)
        .with_source_id(source_id));
    }
    connection
        .execute(
            "UPDATE sync_source_results SET source_revision=?1,status=?2,error_code=?3,
                    failed_count=CASE WHEN ?2 IN ('failed','cancelled') AND failed_count=0 THEN 1 ELSE failed_count END
             WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?4) AND source_id=?5",
            sql_params![source_revision, state, error_code, task_id, source_id],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-source-transition"))?;
    Ok(())
}

fn validate_sync_run_id(sync_run_id: &str) -> Result<(), AppError> {
    if sync_run_id.len() != 28
        || !sync_run_id.starts_with("run:")
        || !sync_run_id[4..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(sync_error(
            ErrorCode::ValidationSource,
            "sync-result-run-id",
        ));
    }
    Ok(())
}

fn validate_result_input(input: &GetSyncResultInputV1) -> Result<(), AppError> {
    if input.contract_version != 1 || !(1..=100).contains(&input.limit) {
        return Err(sync_error(ErrorCode::ValidationSource, "sync-result-input"));
    }
    validate_sync_run_id(input.sync_run_id.as_str())?;
    if let Some(cursor) = &input.cursor {
        let (cursor_run, _, _, _) = decode_result_cursor(cursor)?;
        if cursor_run != input.sync_run_id {
            return Err(sync_error(
                ErrorCode::ValidationSource,
                "sync-result-cursor-run",
            ));
        }
    }
    Ok(())
}

fn publisher_from_url(value: &str) -> Result<String, AppError> {
    Url::parse(value)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .and_then(|url| url.host_str().map(str::to_owned))
        .filter(|publisher| !publisher.is_empty())
        .ok_or_else(|| sync_error(ErrorCode::ValidationSource, "sync-result-publisher"))
}

fn valid_public_result_url(value: &str) -> Option<&str> {
    Url::parse(value)
        .ok()
        .filter(|url| {
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        })
        .map(|_| value)
}

fn read_sync_result_page(
    connection: &Connection,
    input: &GetSyncResultInputV1,
) -> Result<SyncResultPageV1, AppError> {
    let summary = read_sync_result_summary(connection, &input.sync_run_id)?;
    let cursor = input
        .cursor
        .as_deref()
        .map(decode_result_cursor)
        .transpose()?;
    let mut statement = connection
        .prepare(
            "SELECT result_item_id,source_id,intel_item_id,source_kind,publisher,original_title,published_at,collected_at,original_url,disposition
             FROM sync_result_items
             WHERE sync_run_id=?1 AND (?2 IS NULL OR (source_id,collected_at,result_item_id)>(?2,?3,?4))
             ORDER BY source_id,collected_at,result_item_id LIMIT ?5",
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-page-prepare"))?;
    let (cursor_source, cursor_collected, cursor_item) = cursor
        .map_or((None, None, None), |(_, source, collected, item)| {
            (Some(source), Some(collected), Some(item))
        });
    if let (Some(source), Some(collected), Some(item)) =
        (&cursor_source, &cursor_collected, &cursor_item)
    {
        let boundary_exists = connection
            .query_row(
                "SELECT 1 FROM sync_result_items
                 WHERE sync_run_id=?1 AND source_id=?2 AND collected_at=?3 AND result_item_id=?4",
                sql_params![input.sync_run_id.as_str(), source, collected, item],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-cursor-read"))?
            .is_some();
        if !boundary_exists {
            return Err(sync_error(
                ErrorCode::ValidationSource,
                "sync-result-cursor-boundary",
            ));
        }
    }
    let limit = i64::from(input.limit) + 1;
    let mut items = statement
        .query_map(
            sql_params![
                input.sync_run_id.as_str(),
                cursor_source,
                cursor_collected,
                cursor_item,
                limit,
            ],
            |row| {
                Ok(SyncResultItemV1 {
                    contract_version: 1,
                    result_item_id: row.get(0)?,
                    sync_run_id: input.sync_run_id.clone(),
                    source_id: row.get(1)?,
                    intel_item_id: Some(row.get(2)?),
                    source_kind: row.get(3)?,
                    publisher: row.get(4)?,
                    original_title: row.get(5)?,
                    published_at: row.get(6)?,
                    collected_at: row.get(7)?,
                    original_url: row.get(8)?,
                    disposition: parse_result_disposition(&row.get::<_, String>(9)?)?,
                })
            },
        )
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-page-read"))?;
    for item in &items {
        validate_result_item(item)?;
    }
    let next_cursor = if items.len() > input.limit as usize {
        items.pop();
        items.last().map(encode_result_cursor)
    } else {
        None
    };
    Ok(SyncResultPageV1 {
        contract_version: 1,
        summary,
        items,
        next_cursor,
    })
}

fn read_sync_result_summary(
    connection: &Connection,
    sync_run_id: &SyncRunIdV1,
) -> Result<SyncResultSummaryV1, AppError> {
    let mut summary = connection
        .query_row(
            "SELECT task_id,outcome,started_at_ms,finished_at_ms,inserted_count,updated_count,skipped_count,failed_count
             FROM sync_runs WHERE sync_run_id=?1 AND outcome IS NOT NULL AND finished_at_ms IS NOT NULL",
            [sync_run_id.as_str()],
            |row| {
                Ok(SyncResultSummaryV1 {
                    contract_version: 1,
                    sync_run_id: sync_run_id.clone(),
                    task_id: row.get(0)?,
                    outcome: parse_sync_run_outcome(&row.get::<_, String>(1)?)?,
                    started_at: timestamp(row.get(2)?)?,
                    finished_at: timestamp(row.get(3)?)?,
                    counts: SyncResultCountsV1 {
                        inserted: sql_u64(row.get(4)?)?.try_into().map_err(|_| SqlError::InvalidQuery)?,
                        updated: sql_u64(row.get(5)?)?.try_into().map_err(|_| SqlError::InvalidQuery)?,
                        skipped: sql_u64(row.get(6)?)?.try_into().map_err(|_| SqlError::InvalidQuery)?,
                        failed: sql_u64(row.get(7)?)?.try_into().map_err(|_| SqlError::InvalidQuery)?,
                    },
                    sources: Vec::new(),
                })
            },
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-summary-read"))?;
    let mut statement = connection
        .prepare(
            "SELECT source_id,source_revision,source_kind,publisher,status,inserted_count,updated_count,skipped_count,failed_count,error_code
             FROM sync_source_results WHERE sync_run_id=?1 ORDER BY source_id",
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-sources-prepare"))?;
    summary.sources = statement
        .query_map([sync_run_id.as_str()], |row| {
            Ok(SyncSourceResultV1 {
                contract_version: 1,
                source_id: row.get(0)?,
                source_revision: sql_u64(row.get(1)?)?,
                source_kind: row.get(2)?,
                publisher: row.get(3)?,
                status: row.get(4)?,
                counts: SyncResultCountsV1 {
                    inserted: sql_u64(row.get(5)?)?
                        .try_into()
                        .map_err(|_| SqlError::InvalidQuery)?,
                    updated: sql_u64(row.get(6)?)?
                        .try_into()
                        .map_err(|_| SqlError::InvalidQuery)?,
                    skipped: sql_u64(row.get(7)?)?
                        .try_into()
                        .map_err(|_| SqlError::InvalidQuery)?,
                    failed: sql_u64(row.get(8)?)?
                        .try_into()
                        .map_err(|_| SqlError::InvalidQuery)?,
                },
                error_code: row.get(9)?,
            })
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-sources-read"))?;
    validate_result_summary_consistency(connection, &summary)?;
    Ok(summary)
}

fn validate_result_item(item: &SyncResultItemV1) -> Result<(), AppError> {
    let published_at_valid = item.published_at.as_deref().is_none_or(|value| {
        crate::contracts::effects::normalize_rfc3339_utc(value).as_deref() == Some(value)
    });
    if item.source_kind != "rss_atom"
        || item.publisher.trim().is_empty()
        || item.original_title.trim().is_empty()
        || item
            .intel_item_id
            .as_deref()
            .and_then(crate::domain::intel::IntelItemId::parse)
            .is_none()
        || !published_at_valid
        || crate::contracts::effects::normalize_rfc3339_utc(&item.collected_at).as_deref()
            != Some(item.collected_at.as_str())
        || valid_public_result_url(&item.original_url).is_none()
    {
        return Err(sync_error(
            ErrorCode::StorageSource,
            "sync-result-item-contract",
        ));
    }
    Ok(())
}

fn validate_result_summary_consistency(
    connection: &Connection,
    summary: &SyncResultSummaryV1,
) -> Result<(), AppError> {
    let source_totals = summary.sources.iter().try_fold(
        [0_u64; 4],
        |mut totals, source| -> Result<[u64; 4], AppError> {
            if source.source_kind != "rss_atom"
                || source.publisher.trim().is_empty()
                || !matches!(
                    source.status.as_str(),
                    "succeeded" | "failed" | "cancelled" | "retry_wait"
                )
            {
                return Err(sync_error(
                    ErrorCode::StorageSource,
                    "sync-result-source-contract",
                ));
            }
            for (total, value) in totals.iter_mut().zip([
                source.counts.inserted,
                source.counts.updated,
                source.counts.skipped,
                source.counts.failed,
            ]) {
                *total = total
                    .checked_add(u64::from(value))
                    .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-result-count"))?;
            }
            Ok(totals)
        },
    )?;
    let summary_totals = [
        u64::from(summary.counts.inserted),
        u64::from(summary.counts.updated),
        u64::from(summary.counts.skipped),
        u64::from(summary.counts.failed),
    ];
    let item_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sync_result_items WHERE sync_run_id=?1",
            [summary.sync_run_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-item-count"))?;
    let succeeded_sources = summary
        .sources
        .iter()
        .filter(|source| source.status == "succeeded")
        .count();
    let expected_outcome = if summary.counts.failed > 0 && succeeded_sources > 0 {
        SyncRunOutcomeV1::PartiallySucceeded
    } else if summary.counts.failed > 0 {
        SyncRunOutcomeV1::Failed
    } else if u64::from(summary.counts.inserted) + u64::from(summary.counts.updated) == 0 {
        SyncRunOutcomeV1::SucceededZeroResults
    } else {
        SyncRunOutcomeV1::SucceededWithResults
    };
    let expected_item_count = i64::try_from(source_totals[0] + source_totals[1])
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-item-count-range"))?;
    if summary.sources.is_empty()
        || source_totals != summary_totals
        || item_count != expected_item_count
        || summary.outcome != expected_outcome
    {
        return Err(sync_error(
            ErrorCode::StorageSource,
            "sync-result-summary-contract",
        ));
    }
    Ok(())
}

fn parse_sync_run_outcome(value: &str) -> SqlResult<SyncRunOutcomeV1> {
    match value {
        "succeeded_with_results" => Ok(SyncRunOutcomeV1::SucceededWithResults),
        "succeeded_zero_results" => Ok(SyncRunOutcomeV1::SucceededZeroResults),
        "partially_succeeded" => Ok(SyncRunOutcomeV1::PartiallySucceeded),
        "failed" => Ok(SyncRunOutcomeV1::Failed),
        _ => Err(SqlError::InvalidQuery),
    }
}

fn parse_result_disposition(value: &str) -> SqlResult<SyncResultDispositionV1> {
    match value {
        "inserted" => Ok(SyncResultDispositionV1::Inserted),
        "updated" => Ok(SyncResultDispositionV1::Updated),
        _ => Err(SqlError::InvalidQuery),
    }
}

fn encode_result_cursor(item: &SyncResultItemV1) -> String {
    let raw = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        item.sync_run_id, item.source_id, item.collected_at, item.result_item_id
    );
    let mut encoded = String::with_capacity(raw.len() * 2 + 7);
    encoded.push_str("cursor:");
    for byte in raw.bytes() {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

fn decode_result_cursor(value: &str) -> Result<(SyncRunIdV1, String, String, String), AppError> {
    let encoded = value
        .strip_prefix("cursor:")
        .filter(|encoded| !encoded.is_empty() && encoded.len() <= 1024 && encoded.len() % 2 == 0)
        .ok_or_else(|| sync_error(ErrorCode::ValidationSource, "sync-result-cursor"))?;
    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.as_bytes().chunks_exact(2) {
        let pair = std::str::from_utf8(pair)
            .map_err(|_| sync_error(ErrorCode::ValidationSource, "sync-result-cursor"))?;
        bytes.push(
            u8::from_str_radix(pair, 16)
                .map_err(|_| sync_error(ErrorCode::ValidationSource, "sync-result-cursor"))?,
        );
    }
    let decoded = String::from_utf8(bytes)
        .map_err(|_| sync_error(ErrorCode::ValidationSource, "sync-result-cursor"))?;
    let mut parts = decoded.split('\u{1f}');
    let run = SyncRunIdV1::parse(parts.next().unwrap_or_default().to_owned());
    let source = parts.next().unwrap_or_default().to_owned();
    let collected = parts.next().unwrap_or_default().to_owned();
    let item = parts.next().unwrap_or_default().to_owned();
    if parts.next().is_some()
        || run.is_none()
        || !is_safe_source_id(&source)
        || crate::contracts::effects::normalize_rfc3339_utc(&collected).is_none()
        || item.len() != 31
        || !item.starts_with("result:")
        || !item[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(sync_error(
            ErrorCode::ValidationSource,
            "sync-result-cursor",
        ));
    }
    Ok((run.expect("validated run id"), source, collected, item))
}

fn recompute_job(
    connection: &Connection,
    task_id: &str,
    expected_revision: u64,
    now: u64,
    increment_revision: bool,
) -> Result<(), AppError> {
    let (succeeded, failed, retry_wait, active): (u32, u32, u32, u32) = connection
        .query_row(
            "SELECT
               SUM(state='succeeded'),SUM(state IN ('failed','cancelled')),
               SUM(state='retry_wait'),SUM(state IN ('queued','running'))
             FROM job_source_states WHERE task_id=?1",
            [task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-job-aggregate"))?;
    let (state, terminal, error_summary) = if active > 0 {
        ("running", false, None)
    } else if succeeded > 0 && (failed > 0 || retry_wait > 0) {
        ("partially_succeeded", true, Some("source.partial_failure"))
    } else if retry_wait > 0 {
        ("retry_wait", false, Some("rate_limited.source"))
    } else if failed > 0 {
        ("failed", true, Some("source.sync_failed"))
    } else {
        ("succeeded", true, None)
    };
    let revision_increment = i64::from(increment_revision);
    let changed = connection
        .execute(
            "UPDATE jobs SET state=?1,revision=revision+?2,finished_at_ms=?3,updated_at_ms=?4,error_summary=?5
             WHERE task_id=?6 AND revision=?7 AND state IN ('running','retry_wait')",
            sql_params![
                state,
                revision_increment,
                terminal.then(|| sql_i64(now)).transpose()?,
                sql_i64(now)?,
                error_summary,
                task_id,
                sql_i64(expected_revision)?,
            ],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-job-recompute"))?;
    if changed != 1 {
        return Err(sync_error(
            ErrorCode::ConflictSourceRevision,
            "sync-job-recompute-revision",
        )
        .with_task_id(task_id));
    }
    recompute_sync_run(connection, task_id, terminal, now)?;
    Ok(())
}

fn persist_success_result_projection(
    connection: &Connection,
    task_id: &str,
    request: &IncrementalFetchRequest,
    normalized: &[NormalizedIntelCandidate],
    applied: &[crate::domain::sources::CandidateApplyResult],
    issues: &[NormalizationIssue],
) -> Result<[u32; 4], AppError> {
    let sync_run_id: String = connection
        .query_row(
            "SELECT sync_run_id FROM sync_runs WHERE task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-run-read"))?;
    let mut counts = [0_u32, 0, 0, u32::try_from(issues.len()).unwrap_or(u32::MAX)];
    persist_normalization_issues(connection, &sync_run_id, &request.source_id, issues)?;
    for applied_candidate in applied {
        let Some(candidate) = normalized
            .iter()
            .find(|candidate| candidate.stable_external_id == applied_candidate.stable_external_id)
        else {
            return Err(sync_error(
                ErrorCode::SourceFormatRssAtom,
                "sync-result-candidate-missing",
            ));
        };
        let disposition =
            persist_normalized_candidate(connection, &sync_run_id, &request.source_id, candidate)?;
        match disposition {
            CandidateDisposition::New => {
                counts[0] = counts[0].saturating_add(1);
            }
            CandidateDisposition::Changed => {
                counts[1] = counts[1].saturating_add(1);
            }
            CandidateDisposition::Unchanged => {
                counts[2] = counts[2].saturating_add(1);
            }
        }
    }
    update_success_source_result(connection, &sync_run_id, request, counts)?;
    Ok(counts)
}

fn persist_normalization_issues(
    connection: &Connection,
    sync_run_id: &str,
    source_id: &str,
    issues: &[NormalizationIssue],
) -> Result<(), AppError> {
    for issue in issues {
        let failure_id = format!(
            "failure:{}",
            &digest_hex(format!("{sync_run_id}\0{source_id}\0{}", issue.candidate_ref).as_bytes())
                [..24]
        );
        connection
            .execute(
                "INSERT INTO sync_result_item_failures(failure_id,sync_run_id,source_id,candidate_ref,field_name,reason_code,observed_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7)",
                sql_params![
                    failure_id,
                    sync_run_id,
                    source_id,
                    issue.candidate_ref,
                    issue.field,
                    issue.code.as_str(),
                    unix_ms_to_rfc3339(now_ms()?),
                ],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-failure-insert"))?;
    }
    Ok(())
}

fn persist_normalization_failure_projection(
    connection: &Connection,
    task_id: &str,
    source_id: &str,
    source_revision: u64,
    issues: &[NormalizationIssue],
) -> Result<(), AppError> {
    let sync_run_id: String = connection
        .query_row(
            "SELECT sync_run_id FROM sync_runs WHERE task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-run-read"))?;
    persist_normalization_issues(connection, &sync_run_id, source_id, issues)?;
    connection
        .execute(
            "UPDATE sync_source_results SET source_revision=?1,status='failed',inserted_count=0,updated_count=0,skipped_count=0,failed_count=?2,error_code='source_format.rss_atom'
             WHERE sync_run_id=?3 AND source_id=?4",
            sql_params![
                sql_i64(source_revision)?,
                i64::try_from(issues.len()).unwrap_or(i64::MAX),
                sync_run_id,
                source_id,
            ],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-source-normalization"))?;
    Ok(())
}

fn persist_normalized_candidate(
    connection: &Connection,
    sync_run_id: &str,
    source_id: &str,
    candidate: &NormalizedIntelCandidate,
) -> Result<CandidateDisposition, AppError> {
    let (first_seen_at_ms, last_seen_at_ms): (i64, i64) = connection
        .query_row(
            "SELECT first_seen_at_ms,last_seen_at_ms FROM source_entry_checkpoints WHERE source_id=?1 AND stable_external_id=?2",
            sql_params![source_id, candidate.stable_external_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-intel-checkpoint-read"))?;
    let first_discovered_at = timestamp(first_seen_at_ms)
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-intel-first-time"))?;
    let last_updated_at = timestamp(last_seen_at_ms)
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-intel-last-time"))?;
    let written = crate::infrastructure::database::intel_repository::upsert_normalized_intel(
        connection,
        candidate,
        &first_discovered_at,
        &last_updated_at,
    )
    .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-intel-upsert"))?;
    crate::infrastructure::database::association_repository::reconcile_url_membership(
        connection,
        written.item_id,
        &candidate.original_url,
        &first_discovered_at,
        &last_updated_at,
    )
    .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-association-upsert"))?;
    let configuration = super::configuration::read_configuration(connection)?;
    crate::infrastructure::database::rule_evaluation_repository::evaluate_item(
        connection,
        written.item_id,
        &configuration,
        u64::try_from(last_seen_at_ms)
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-rule-time"))?,
    )
    .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-rule-evaluation"))?;
    let disposition = match written.disposition {
        CandidateDisposition::New => Some("inserted"),
        CandidateDisposition::Changed => Some("updated"),
        CandidateDisposition::Unchanged => None,
    };
    if let Some(disposition) = disposition {
        let result_item_id = format!(
            "result:{}",
            &digest_hex(
                format!("{sync_run_id}:{source_id}:{}", candidate.stable_external_id).as_bytes(),
            )[..24]
        );
        connection
            .execute(
                "INSERT INTO sync_result_items(result_item_id,sync_run_id,source_id,stable_external_id,intel_item_id,source_kind,publisher,original_title,published_at,collected_at,original_url,disposition)
                 VALUES(?1,?2,?3,?4,?5,'rss_atom',?6,?7,?8,?9,?10,?11)
                 ON CONFLICT(sync_run_id,source_id,stable_external_id) DO NOTHING",
                sql_params![
                    result_item_id,
                    sync_run_id,
                    source_id,
                    candidate.stable_external_id,
                    candidate.intel_item_id.as_str(),
                    candidate.publisher,
                    candidate.original_title,
                    candidate.published_at,
                    candidate.collected_at,
                    candidate.original_url,
                    disposition,
                ],
            )
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-item-insert"))?;
    }
    Ok(written.disposition)
}

fn normalize_result_candidates(
    result: &FetchIncrementalResult,
    source_id: &str,
    publisher: &str,
    collected_at: &str,
) -> (
    FetchIncrementalResult,
    Vec<NormalizedIntelCandidate>,
    Vec<NormalizationIssue>,
) {
    let mut candidates = Vec::with_capacity(result.candidates.len());
    let mut normalized = Vec::with_capacity(result.candidates.len());
    let mut issues = Vec::new();
    for candidate in &result.candidates {
        match normalize_rss_candidate(source_id, publisher, collected_at, candidate) {
            Ok(value) => {
                candidates.push(RawSourceCandidate {
                    stable_external_id: value.stable_external_id.clone(),
                    title: Some(value.original_title.clone()),
                    original_url: Some(value.original_url.clone()),
                    author: value.author.clone(),
                    summary: value.source_summary.clone(),
                    published_at: value.published_at.clone(),
                    updated_at: candidate.updated_at.clone(),
                    content_hash: value.content_hash.clone(),
                    warnings: candidate.warnings.clone(),
                });
                normalized.push(value);
            }
            Err(issue) => issues.push(issue),
        }
    }
    (
        FetchIncrementalResult {
            candidates,
            etag: result.etag.clone(),
            last_modified: result.last_modified.clone(),
            adapter_cursor: result.adapter_cursor.clone(),
            not_modified: result.not_modified,
        },
        normalized,
        issues,
    )
}

fn update_success_source_result(
    connection: &Connection,
    sync_run_id: &str,
    request: &IncrementalFetchRequest,
    counts: [u32; 4],
) -> Result<(), AppError> {
    let result_status = if counts[3] > 0 && counts[0] + counts[1] + counts[2] == 0 {
        "failed"
    } else {
        "succeeded"
    };
    connection
        .execute(
            "UPDATE sync_source_results SET source_revision=?1,status=?2,inserted_count=?3,updated_count=?4,skipped_count=?5,failed_count=?6,error_code=?7
             WHERE sync_run_id=?8 AND source_id=?9",
            sql_params![
                sql_i64(request.expected_revision.saturating_add(1))?,
                result_status,
                i64::from(counts[0]),
                i64::from(counts[1]),
                i64::from(counts[2]),
                i64::from(counts[3]),
                (counts[3] > 0).then_some("source_format.rss_atom"),
                sync_run_id,
                request.source_id,
            ],
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-source-success"))?;
    Ok(())
}

fn persist_failure_result_projection(
    connection: &Connection,
    task_id: &str,
    source_id: &str,
    status: &str,
    error_code: &str,
) -> Result<(), AppError> {
    connection
        .execute(
            "UPDATE sync_source_results SET status=?1,failed_count=1,error_code=?2
             WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?3) AND source_id=?4",
            sql_params![status, error_code, task_id, source_id],
        )
        .map(|_| ())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-source-failure"))
}

fn recompute_sync_run(
    connection: &Connection,
    task_id: &str,
    terminal: bool,
    now: u64,
) -> Result<(), AppError> {
    let counts: (i64, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT COALESCE(SUM(inserted_count),0),COALESCE(SUM(updated_count),0),COALESCE(SUM(skipped_count),0),COALESCE(SUM(failed_count),0),COALESCE(SUM(status='succeeded'),0)
             FROM sync_source_results WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?1)",
            [task_id],
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
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-count-aggregate"))?;
    let outcome = terminal.then_some(if counts.3 > 0 && counts.4 > 0 {
        "partially_succeeded"
    } else if counts.3 > 0 {
        "failed"
    } else if counts.0 + counts.1 == 0 {
        "succeeded_zero_results"
    } else {
        "succeeded_with_results"
    });
    connection
        .execute(
            "UPDATE sync_runs SET outcome=?1,finished_at_ms=?2,inserted_count=?3,updated_count=?4,skipped_count=?5,failed_count=?6 WHERE task_id=?7",
            sql_params![
                outcome,
                terminal.then(|| sql_i64(now)).transpose()?,
                counts.0,
                counts.1,
                counts.2,
                counts.3,
                task_id,
            ],
        )
        .map(|_| ())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-result-run-update"))
}

fn read_readiness(connection: &Connection) -> Result<SourceDeliveryReadinessV1, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT source_id,enabled,status,retryability,last_success_at_ms,next_allowed_at_ms
             FROM sources WHERE source_kind='rss_atom' ORDER BY source_id",
        )
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-readiness-prepare"))?;
    let persisted = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? == 1,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
            ))
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-readiness-read"))?;
    let mut sources = Vec::with_capacity(persisted.len().max(1));
    let mut enabled_count = 0_u32;
    let mut active = false;
    let mut all_available = true;
    for (source_id, enabled, status, retryability, last_success, next_allowed) in persisted {
        enabled_count += u32::from(enabled);
        let active_state = connection
            .query_row(
                "SELECT s.state FROM job_source_states s JOIN jobs j ON j.task_id=s.task_id
                 WHERE s.source_id=?1 AND s.state IN ('queued','running','retry_wait')
                   AND j.state IN ('queued','running','retry_wait')
                 ORDER BY s.updated_at_ms DESC LIMIT 1",
                [&source_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-readiness-active"))?;
        let readiness = if !enabled {
            SourceReadinessStatusV1::Disabled
        } else if matches!(active_state.as_deref(), Some("queued" | "running")) {
            active = true;
            SourceReadinessStatusV1::Syncing
        } else if active_state.as_deref() == Some("retry_wait") {
            active = true;
            SourceReadinessStatusV1::RetryWait
        } else if status == "retry_wait" && retryability == "after" {
            SourceReadinessStatusV1::RateLimited
        } else if status == "retry_wait" {
            SourceReadinessStatusV1::RetryWait
        } else if status == "error" {
            SourceReadinessStatusV1::Failed
        } else if last_success.is_some() {
            SourceReadinessStatusV1::Available
        } else {
            SourceReadinessStatusV1::Failed
        };
        if enabled && readiness != SourceReadinessStatusV1::Available {
            all_available = false;
        }
        sources.push(SourceReadinessV1 {
            contract_version: 1,
            source_id: Some(source_id),
            source_kind: "rss_atom".to_owned(),
            status: readiness,
            last_success_at: last_success
                .map(timestamp)
                .transpose()
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-readiness-time"))?,
            next_allowed_at: next_allowed
                .map(timestamp)
                .transpose()
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-readiness-time"))?,
        });
    }
    let status = if enabled_count == 0 {
        if sources.is_empty() {
            sources.push(SourceReadinessV1 {
                contract_version: 1,
                source_id: None,
                source_kind: "rss_atom".to_owned(),
                status: SourceReadinessStatusV1::NotConfigured,
                last_success_at: None,
                next_allowed_at: None,
            });
        }
        DeliveryReadinessStatusV1::NotConfigured
    } else if active {
        DeliveryReadinessStatusV1::Syncing
    } else if all_available {
        DeliveryReadinessStatusV1::Ready
    } else {
        DeliveryReadinessStatusV1::Blocked
    };
    Ok(SourceDeliveryReadinessV1 {
        contract_version: 1,
        required_source_kinds: vec!["rss_atom".to_owned()],
        status,
        sources,
    })
}

fn parse_task_state(value: &str) -> SqlResult<TaskStateV1> {
    match value {
        "queued" => Ok(TaskStateV1::Queued),
        "running" => Ok(TaskStateV1::Running),
        "retry_wait" => Ok(TaskStateV1::RetryWait),
        "succeeded" => Ok(TaskStateV1::Succeeded),
        "partially_succeeded" => Ok(TaskStateV1::PartiallySucceeded),
        "failed" => Ok(TaskStateV1::Failed),
        "cancelled" => Ok(TaskStateV1::Cancelled),
        _ => Err(SqlError::InvalidQuery),
    }
}

fn timestamp(value: i64) -> SqlResult<String> {
    sql_u64(value).map(unix_ms_to_rfc3339)
}

fn optional_timestamp(value: Option<i64>) -> SqlResult<Option<String>> {
    value.map(timestamp).transpose()
}

fn parse_rfc3339_ms(value: &str) -> Result<u64, AppError> {
    let canonical = crate::contracts::effects::normalize_rfc3339_utc(value)
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let date_time = canonical
        .strip_suffix('Z')
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let (date, time) = date_time
        .split_once('T')
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let mut date_parts = date.split('-').map(str::parse::<i64>);
    let year = date_parts
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let month = date_parts
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let day = date_parts
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let (clock, fraction) = time.split_once('.').unwrap_or((time, "0"));
    let mut clock_parts = clock.split(':').map(str::parse::<u64>);
    let hour = clock_parts
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let minute = clock_parts
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let second = clock_parts
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let milliseconds = fraction.chars().take(3).collect::<String>();
    let milliseconds = format!("{milliseconds:0<3}")
        .parse::<u64>()
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-time-contract"))?;
    let days = days_from_civil(year, month, day);
    u64::try_from(days)
        .ok()
        .and_then(|days| days.checked_mul(86_400_000))
        .and_then(|base| {
            base.checked_add(hour * 3_600_000 + minute * 60_000 + second * 1_000 + milliseconds)
        })
        .ok_or_else(|| sync_error(ErrorCode::StorageSource, "sync-time-range"))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
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
        .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-time"))
        .and_then(|duration| {
            u64::try_from(duration.as_millis())
                .map_err(|_| sync_error(ErrorCode::StorageSource, "sync-time-range"))
        })
}

fn sql_i64(value: u64) -> Result<i64, AppError> {
    i64::try_from(value).map_err(|_| sync_error(ErrorCode::StorageSource, "sync-number-range"))
}

fn sql_u64(value: i64) -> SqlResult<u64> {
    u64::try_from(value).map_err(|error| SqlError::ToSqlConversionFailure(Box::new(error)))
}

fn sql_u64_app(value: i64) -> Result<u64, AppError> {
    u64::try_from(value).map_err(|_| sync_error(ErrorCode::StorageSource, "sync-number-range"))
}

pub(crate) fn unix_ms_to_rfc3339(milliseconds: u64) -> String {
    let seconds = milliseconds / 1_000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        day_seconds / 3_600,
        day_seconds % 3_600 / 60,
        day_seconds % 60,
        milliseconds % 1_000
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

fn sync_error(code: ErrorCode, boundary: &'static str) -> AppError {
    AppError::from_code(code, boundary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::sources::ProbedSource;
    use crate::contracts::dto::source::SaveSourceInputV1;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Barrier};

    static ASSOCIATION_DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct ScopedAssociationDatabase(PathBuf);

    impl ScopedAssociationDatabase {
        fn new() -> Self {
            let sequence = ASSOCIATION_DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::current_dir()
                .expect("project directory")
                .join("target")
                .join("story-4-2-association")
                .join(format!(
                    "concurrent-{}-{sequence}.sqlite3",
                    std::process::id()
                ));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("association test directory");
            }
            Self(path)
        }
    }

    impl Drop for ScopedAssociationDatabase {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
            }
        }
    }

    fn save_test_source(store: &mut DemoStore, url: &str, key: &str) -> String {
        let revision = store.get_configuration().expect("configuration").revision;
        store
            .save_probed_source(
                &SaveSourceInputV1 {
                    contract_version: 1,
                    source_kind: "rss_atom".to_owned(),
                    url: url.to_owned(),
                    expected_configuration_revision: revision,
                    idempotency_key: key.to_owned(),
                },
                &ProbedSource::from_adapter_result(
                    url,
                    FetchIncrementalResult {
                        candidates: Vec::new(),
                        etag: None,
                        last_modified: None,
                        adapter_cursor: None,
                        not_modified: false,
                    },
                )
                .expect("probe"),
            )
            .expect("save source")
            .source_id
    }

    fn test_candidate(id: &str, title: &str, url: &str, version: &str) -> RawSourceCandidate {
        let mut content_hash = String::with_capacity(64);
        for byte in Sha256::digest(version.as_bytes()) {
            let _ = write!(&mut content_hash, "{byte:02x}");
        }
        RawSourceCandidate {
            stable_external_id: id.to_owned(),
            title: Some(title.to_owned()),
            original_url: Some(url.to_owned()),
            author: None,
            summary: None,
            published_at: Some("2026-08-19T08:00:00Z".to_owned()),
            updated_at: None,
            content_hash,
            warnings: Vec::new(),
        }
    }

    fn start_and_claim(store: &mut DemoStore, source_id: &str, key: &str) -> SyncExecutionPlan {
        let task = store
            .start_sync(&StartSyncInputV1 {
                contract_version: 1,
                target: SyncTargetV1::SourceId {
                    source_id: source_id.to_owned(),
                },
                idempotency_key: key.to_owned(),
                foreground_budget_ms: MAX_FOREGROUND_BUDGET_MS,
            })
            .expect("start sync");
        store
            .claim_sync_task(&task.task_id, task.revision)
            .expect("claim sync")
    }

    fn assert_updated_rule_fact(store: &DemoStore, source_id: &str) {
        let real_count: u32 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM intel_items WHERE data_origin='real'",
                [],
                |row| row.get(0),
            )
            .expect("real fact count");
        let fact: (i64, String, Option<String>, Option<String>) = store
            .connection
            .query_row(
                "SELECT revision,title,track,importance FROM intel_items WHERE source_id=?1 AND stable_external_id='shared-entry'",
                [source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("updated fact");
        let provenance: (String, Option<String>, String, String, String) = store
            .connection
            .query_row(
                "SELECT provenance_id,author,author_availability,first_discovered_at,last_updated_at FROM item_provenance WHERE source_id=?1 AND stable_external_id='shared-entry'",
                [source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .expect("updated provenance");
        let derived_count: u32 = store.connection.query_row(
            "SELECT (SELECT COUNT(*) FROM rule_evaluations r JOIN intel_items i ON i.id=r.item_id WHERE i.data_origin='real') +
                    (SELECT COUNT(*) FROM analysis_results a JOIN intel_items i ON i.id=a.item_id WHERE i.data_origin='real')",
            [], |row| row.get(0),
        ).expect("derived projection count");
        let rule: (String, i64, i64, String, String) = store.connection.query_row(
            "SELECT r.rule_version,r.fact_revision,r.score,r.stream_disposition,r.ai_status
             FROM rule_evaluations r JOIN intel_items i ON i.id=r.item_id WHERE i.data_origin='real'",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).expect("current rule projection");
        assert_eq!(real_count, 1);
        assert_eq!((fact.0, fact.1.as_str()), (2, "Updated title"));
        assert_eq!((fact.2, fact.3), (None, None));
        assert_eq!((provenance.1, provenance.2.as_str()), (None, "unavailable"));
        assert!(provenance.0.starts_with("prov:intel:"));
        assert_eq!(provenance.0.len(), 75);
        assert!(provenance.3 <= provenance.4);
        assert_eq!(derived_count, 1);
        assert_eq!(
            (rule.0.as_str(), rule.1, rule.3.as_str(), rule.4.as_str()),
            (
                crate::domain::rules::intelligence_value::INTELLIGENCE_VALUE_RULE_VERSION,
                2,
                "ordinary_candidate",
                "unavailable"
            )
        );
        assert!((0..=100).contains(&rule.2));
    }

    #[test]
    fn final_fact_update_preserves_provenance_and_refreshes_the_rule_projection() {
        let mut store = DemoStore::open_in_memory().expect("v8");
        let source_id = save_test_source(
            &mut store,
            "https://publisher.example/feed.xml",
            "source-final-fact",
        );
        for (key, title, version) in [
            ("sync-final-v1", "First title", "version-one"),
            ("sync-final-v2", "Updated title", "version-two"),
        ] {
            let plan = start_and_claim(&mut store, &source_id, key);
            store
                .commit_sync_source_success(
                    &plan.task.task_id,
                    plan.task.revision,
                    &plan.sources[0],
                    &FetchIncrementalResult {
                        candidates: vec![test_candidate(
                            "shared-entry",
                            title,
                            "https://publisher.example/item",
                            version,
                        )],
                        etag: None,
                        last_modified: None,
                        adapter_cursor: None,
                        not_modified: false,
                    },
                )
                .expect("commit fact");
        }

        assert_updated_rule_fact(&store, &source_id);
    }

    #[test]
    fn rule_projection_failure_rolls_back_the_entire_source_transaction() {
        let mut store = DemoStore::open_in_memory().expect("v10");
        let source_id = save_test_source(
            &mut store,
            "https://rule-rollback.example/feed.xml",
            "source-rule-rollback",
        );
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_rule_projection
                 BEFORE INSERT ON rule_evaluations
                 WHEN NEW.rule_version IS NOT NULL
                 BEGIN SELECT RAISE(ABORT,'rule projection fault'); END;",
            )
            .expect("fault injection");
        let plan = start_and_claim(&mut store, &source_id, "sync-rule-rollback");
        let error = store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        "rollback-entry",
                        "Foundation model release",
                        "https://rule-rollback.example/item",
                        "version-one",
                    )],
                    etag: Some("must-roll-back".to_owned()),
                    last_modified: None,
                    adapter_cursor: Some("must-roll-back".to_owned()),
                    not_modified: false,
                },
            )
            .expect_err("rule fault must reject source commit");
        assert_eq!(error.code(), ErrorCode::StorageSource.as_str());
        let counts: (i64, i64, i64, Option<String>) = store
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM intel_items WHERE data_origin='real'),
                   (SELECT COUNT(*) FROM source_entry_checkpoints WHERE source_id=?1),
                   (SELECT COUNT(*) FROM sync_result_items WHERE source_id=?1),
                   (SELECT etag FROM sources WHERE source_id=?1)",
                [&source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("rollback state");
        assert_eq!(counts, (0, 0, 0, None));
    }

    #[test]
    fn configuration_save_atomically_refreshes_rules_without_rewriting_facts() {
        let mut store = DemoStore::open_in_memory().expect("v10");
        let source_id = save_test_source(
            &mut store,
            "https://rule-config.example/feed.xml",
            "source-rule-config",
        );
        let plan = start_and_claim(&mut store, &source_id, "sync-rule-config");
        store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        "config-entry",
                        "Foundation model release improves reasoning",
                        "https://rule-config.example/item",
                        "version-one",
                    )],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("initial projection");
        let fact_before: (String, String, i64, String) = store
            .connection
            .query_row(
                "SELECT i.title,i.content_hash,i.revision,p.original_url
                 FROM intel_items i JOIN item_provenance p ON p.item_id=i.id
                 WHERE i.source_id=?1",
                [&source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("fact before");
        let current = store.get_configuration().expect("configuration");
        let mut configuration = current.configuration.clone();
        configuration.alert_threshold = 0;
        let hash = crate::domain::rules::configuration_validation::configuration_hash(
            &crate::domain::rules::configuration_validation::normalize(&configuration),
        );
        let saved = store
            .save_attention_configuration(
                &crate::contracts::dto::configuration_validation::SaveConfigurationInputV1 {
                    contract_version: 1,
                    configuration,
                    expected_revision: current.revision,
                    expected_normalized_config_hash: hash,
                    idempotency_key: "rule-config-save".to_owned(),
                    validation_receipt: None,
                },
            )
            .expect("save and re-evaluate");
        let rule: (i64, String, i64) = store
            .connection
            .query_row(
                "SELECT configuration_revision,stream_disposition,COUNT(*)
                 FROM rule_evaluations r JOIN intel_items i ON i.id=r.item_id
                 WHERE i.source_id=?1",
                [&source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("refreshed rule");
        let fact_after: (String, String, i64, String) = store
            .connection
            .query_row(
                "SELECT i.title,i.content_hash,i.revision,p.original_url
                 FROM intel_items i JOIN item_provenance p ON p.item_id=i.id
                 WHERE i.source_id=?1",
                [&source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("fact after");
        assert_eq!(
            rule,
            (
                i64::try_from(saved.revision).expect("revision"),
                "high_value".to_owned(),
                1
            )
        );
        assert_eq!(fact_after, fact_before);
    }

    #[test]
    fn configuration_rule_refresh_failure_rolls_back_the_new_configuration_version() {
        let mut store = DemoStore::open_in_memory().expect("v10");
        let source_id = save_test_source(
            &mut store,
            "https://rule-config-rollback.example/feed.xml",
            "source-rule-config-rollback",
        );
        let plan = start_and_claim(&mut store, &source_id, "sync-rule-config-rollback");
        store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        "config-rollback-entry",
                        "Foundation model release",
                        "https://rule-config-rollback.example/item",
                        "version-one",
                    )],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("initial projection");
        let current = store.get_configuration().expect("configuration");
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_rule_refresh
                 BEFORE UPDATE ON rule_evaluations
                 WHEN NEW.rule_version IS NOT NULL
                 BEGIN SELECT RAISE(ABORT,'rule refresh fault'); END;",
            )
            .expect("fault injection");
        let mut configuration = current.configuration.clone();
        configuration.alert_threshold = 0;
        let hash = crate::domain::rules::configuration_validation::configuration_hash(
            &crate::domain::rules::configuration_validation::normalize(&configuration),
        );
        let error = store
            .save_attention_configuration(
                &crate::contracts::dto::configuration_validation::SaveConfigurationInputV1 {
                    contract_version: 1,
                    configuration,
                    expected_revision: current.revision,
                    expected_normalized_config_hash: hash,
                    idempotency_key: "rule-config-rollback-save".to_owned(),
                    validation_receipt: None,
                },
            )
            .expect_err("rule refresh must roll back configuration");
        assert_eq!(error.code(), ErrorCode::StorageConfiguration.as_str());
        assert_eq!(
            store
                .get_configuration()
                .expect("rolled back current")
                .revision,
            current.revision
        );
        let versions: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM configuration_versions", [], |row| {
                row.get(0)
            })
            .expect("configuration versions");
        assert_eq!(
            versions,
            i64::try_from(current.revision).expect("version count")
        );
    }

    #[test]
    fn one_source_rule_failure_does_not_roll_back_another_source() {
        let mut store = DemoStore::open_in_memory().expect("v10");
        let failing_source = save_test_source(
            &mut store,
            "https://rule-isolation-a.example/feed.xml",
            "source-rule-isolation-a",
        );
        let successful_source = save_test_source(
            &mut store,
            "https://rule-isolation-b.example/feed.xml",
            "source-rule-isolation-b",
        );
        store
            .connection
            .execute_batch(&format!(
                "CREATE TRIGGER reject_one_source_rule
                 BEFORE INSERT ON rule_evaluations
                 WHEN NEW.rule_version IS NOT NULL AND EXISTS(
                   SELECT 1 FROM intel_items WHERE id=NEW.item_id AND source_id='{failing_source}'
                 ) BEGIN SELECT RAISE(ABORT,'source rule fault'); END;"
            ))
            .expect("source-scoped fault");
        for (source_id, key, should_succeed) in [
            (&failing_source, "sync-rule-isolation-a", false),
            (&successful_source, "sync-rule-isolation-b", true),
        ] {
            let plan = start_and_claim(&mut store, source_id, key);
            let result = store.commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        key,
                        "Foundation model release",
                        &format!("https://{key}.example/item"),
                        "version-one",
                    )],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            );
            assert_eq!(result.is_ok(), should_succeed);
        }
        let persisted: Vec<String> = store
            .connection
            .prepare(
                "SELECT i.source_id FROM intel_items i JOIN rule_evaluations r ON r.item_id=i.id
                 WHERE i.data_origin='real' ORDER BY i.source_id",
            )
            .expect("isolation query")
            .query_map([], |row| row.get(0))
            .expect("isolation rows")
            .collect::<Result<_, _>>()
            .expect("isolation results");
        assert_eq!(persisted, [successful_source]);
    }

    #[test]
    fn unchanged_replay_in_the_same_rule_bucket_performs_zero_rule_writes() {
        let mut store = DemoStore::open_in_memory().expect("v10");
        let source_id = save_test_source(
            &mut store,
            "https://rule-replay.example/feed.xml",
            "source-rule-replay",
        );
        let candidate = test_candidate(
            "rule-replay-entry",
            "Foundation model release",
            "https://rule-replay.example/item",
            "stable-version",
        );
        let first = start_and_claim(&mut store, &source_id, "sync-rule-replay-first");
        store
            .commit_sync_source_success(
                &first.task.task_id,
                first.task.revision,
                &first.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![candidate.clone()],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("initial rule");
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_redundant_rule_update
                 BEFORE UPDATE ON rule_evaluations
                 WHEN NEW.rule_version IS NOT NULL
                 BEGIN SELECT RAISE(ABORT,'redundant rule update'); END;",
            )
            .expect("write detector");
        let second = start_and_claim(&mut store, &source_id, "sync-rule-replay-second");
        store
            .commit_sync_source_success(
                &second.task.task_id,
                second.task.revision,
                &second.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![candidate],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("unchanged rule skips write");
        let count: i64 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM rule_evaluations r JOIN intel_items i ON i.id=r.item_id
                 WHERE i.source_id=?1",
                [&source_id],
                |row| row.get(0),
            )
            .expect("one current rule");
        assert_eq!(count, 1);
    }

    #[test]
    fn all_invalid_candidates_fail_without_advancing_success_checkpoint() {
        let mut store = DemoStore::open_in_memory().expect("v8");
        let source_id = save_test_source(
            &mut store,
            "https://invalid-items.example/feed.xml",
            "source-all-invalid",
        );
        let plan = start_and_claim(&mut store, &source_id, "sync-all-invalid");
        let mut invalid = test_candidate(
            "invalid-entry",
            "   ",
            "https://invalid-items.example/item",
            "invalid-version",
        );
        invalid.title = Some("   ".to_owned());
        let task = store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![invalid],
                    etag: Some("must-not-advance".to_owned()),
                    last_modified: None,
                    adapter_cursor: Some("must-not-advance".to_owned()),
                    not_modified: false,
                },
            )
            .expect("record-level failure is a terminal task result");

        let source: (String, Option<i64>, Option<String>, Option<String>) = store
            .connection
            .query_row(
                "SELECT status,last_success_at_ms,etag,adapter_cursor FROM sources WHERE source_id=?1",
                [&source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("source failure state");
        let projection: (String, i64, String) = store
            .connection
            .query_row(
                "SELECT status,failed_count,error_code FROM sync_source_results WHERE sync_run_id=(SELECT sync_run_id FROM sync_runs WHERE task_id=?1)",
                [&plan.task.task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("failed source projection");
        let checkpoints: u32 = store
            .connection
            .query_row("SELECT COUNT(*) FROM source_entry_checkpoints", [], |row| {
                row.get(0)
            })
            .expect("checkpoint count");
        let failures: u32 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM sync_result_item_failures",
                [],
                |row| row.get(0),
            )
            .expect("failure count");

        assert_eq!(task.state, TaskStateV1::Failed);
        assert_eq!(source, ("error".to_owned(), None, None, None));
        assert_eq!(
            projection,
            ("failed".to_owned(), 1, "source_format.rss_atom".to_owned())
        );
        assert_eq!((checkpoints, failures), (0, 1));
    }

    #[test]
    fn changed_fact_keeps_last_updated_monotonic_when_clock_moves_back() {
        let mut store = DemoStore::open_in_memory().expect("v8");
        let source_id = save_test_source(
            &mut store,
            "https://monotonic.example/feed.xml",
            "source-monotonic",
        );
        let first = start_and_claim(&mut store, &source_id, "sync-monotonic-one");
        store
            .commit_sync_source_success(
                &first.task.task_id,
                first.task.revision,
                &first.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        "monotonic-entry",
                        "First",
                        "https://monotonic.example/item",
                        "v1",
                    )],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("first commit");
        let future_ms = 4_000_000_000_000_i64;
        store
            .connection
            .execute(
                "UPDATE source_entry_checkpoints SET last_seen_at_ms=?1 WHERE source_id=?2",
                sql_params![future_ms, source_id],
            )
            .expect("simulate prior future observation");
        let second = start_and_claim(&mut store, &source_id, "sync-monotonic-two");
        store
            .commit_sync_source_success(
                &second.task.task_id,
                second.task.revision,
                &second.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        "monotonic-entry",
                        "Changed",
                        "https://monotonic.example/item",
                        "v2",
                    )],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("changed commit");
        let (last_seen, last_updated): (i64, String) = store
            .connection
            .query_row(
                "SELECT c.last_seen_at_ms,p.last_updated_at FROM source_entry_checkpoints c JOIN item_provenance p ON p.source_id=c.source_id AND p.stable_external_id=c.stable_external_id WHERE c.source_id=?1",
                [&source_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("monotonic lifecycle");

        assert_eq!(last_seen, future_ms);
        assert_eq!(
            last_updated,
            unix_ms_to_rfc3339(u64::try_from(future_ms).unwrap())
        );
    }

    #[test]
    fn normalized_fact_write_failures_roll_back_the_whole_source() {
        for (sequence, table) in [
            "intel_items",
            "item_provenance",
            "intel_associations",
            "sync_result_items",
        ]
        .into_iter()
        .enumerate()
        {
            let mut store = DemoStore::open_in_memory().expect("v8");
            let source_id = save_test_source(
                &mut store,
                "https://atomic.example/feed.xml",
                &format!("source-atomic-{sequence}"),
            );
            let plan = start_and_claim(&mut store, &source_id, &format!("sync-atomic-{sequence}"));
            store
                .connection
                .execute_batch(&format!(
                    "CREATE TRIGGER fail_story_4_1 BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'story-4-1-failure'); END;"
                ))
                .expect("install failure trigger");
            let result = FetchIncrementalResult {
                candidates: vec![test_candidate(
                    "atomic-entry",
                    "Atomic entry",
                    "https://atomic.example/item",
                    "atomic-version",
                )],
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: false,
            };
            store
                .commit_sync_source_success(
                    &plan.task.task_id,
                    plan.task.revision,
                    &plan.sources[0],
                    &result,
                )
                .expect_err("injected write must roll back");
            for query in [
                "SELECT COUNT(*) FROM source_entry_checkpoints",
                "SELECT COUNT(*) FROM intel_items WHERE data_origin='real'",
                "SELECT COUNT(*) FROM sync_result_items",
                "SELECT COUNT(*) FROM sync_result_item_failures",
            ] {
                let count: u32 = store
                    .connection
                    .query_row(query, [], |row| row.get(0))
                    .expect("rolled back count");
                assert_eq!(count, 0, "{table}: {query}");
            }
            store
                .connection
                .execute("DROP TRIGGER fail_story_4_1", [])
                .expect("remove failure trigger");
            store
                .commit_sync_source_success(
                    &plan.task.task_id,
                    plan.task.revision,
                    &plan.sources[0],
                    &result,
                )
                .expect("same transition succeeds after repair");
        }
    }

    #[test]
    fn terminal_sync_history_is_bounded_without_touching_active_jobs() {
        let mut store = DemoStore::open_in_memory().expect("v6");
        let transaction = store
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("transaction");
        for index in 0..(MAX_TERMINAL_TASK_HISTORY + 5) {
            transaction
                .execute(
                    "INSERT INTO jobs(task_id,kind,target_kind,target_source_id,state,revision,idempotency_key,request_fingerprint,foreground_budget_ms,created_at_ms,started_at_ms,finished_at_ms,updated_at_ms)
                     VALUES(?1,'rss_atom_sync','all_enabled_rss_atom',NULL,'succeeded',1,?2,?3,30000,?4,?4,?4,?4)",
                    sql_params![
                        format!("task:{index:024x}"),
                        format!("history-{index}"),
                        format!("fingerprint-{index}"),
                        i64::try_from(index + 1).expect("timestamp"),
                    ],
                )
                .expect("insert terminal task");
            transaction
                .execute(
                    "INSERT INTO sync_runs(sync_run_id,task_id,scope,outcome,started_at_ms,finished_at_ms)
                     VALUES(?1,?2,'all_enabled_rss_atom','succeeded_zero_results',?3,?3)",
                    sql_params![
                        format!("run:{index:024x}"),
                        format!("task:{index:024x}"),
                        i64::try_from(index + 1).expect("timestamp"),
                    ],
                )
                .expect("insert terminal run");
        }
        transaction
            .execute(
                "INSERT INTO jobs(task_id,kind,target_kind,target_source_id,state,revision,idempotency_key,request_fingerprint,foreground_budget_ms,created_at_ms,updated_at_ms)
                 VALUES('task:ffffffffffffffffffffffff','rss_atom_sync','all_enabled_rss_atom',NULL,'queued',1,'active-history','active-fingerprint',30000,999,999)",
                [],
            )
            .expect("insert active task");
        prune_terminal_sync_history(&transaction).expect("prune");
        let terminal_count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE state='succeeded'",
                [],
                |row| row.get(0),
            )
            .expect("terminal count");
        let active_count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE state='queued'",
                [],
                |row| row.get(0),
            )
            .expect("active count");
        let run_count: i64 = transaction
            .query_row("SELECT COUNT(*) FROM sync_runs", [], |row| row.get(0))
            .expect("run count");
        assert_eq!(
            terminal_count,
            i64::try_from(MAX_TERMINAL_TASK_HISTORY).expect("history limit")
        );
        assert_eq!(active_count, 1);
        assert_eq!(run_count, terminal_count);
    }

    #[test]
    fn terminal_history_pruning_preserves_fact_provenance_and_checkpoint() {
        let mut store = DemoStore::open_in_memory().expect("v8");
        let source_id = save_test_source(
            &mut store,
            "https://retention.example/feed.xml",
            "source-retention",
        );
        let plan = start_and_claim(&mut store, &source_id, "sync-retention");
        let retained_task_id = plan.task.task_id.clone();
        store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        "retained-entry",
                        "Retained entry",
                        "https://retention.example/item",
                        "retained-version",
                    )],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("commit retained fact");
        store
            .connection
            .execute(
                "UPDATE jobs SET created_at_ms=1,updated_at_ms=1 WHERE task_id=?1",
                [&retained_task_id],
            )
            .expect("make completed run oldest");
        let transaction = store
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("history transaction");
        for index in 0..MAX_TERMINAL_TASK_HISTORY {
            let task_id = format!("task:{:024x}", index + 10);
            transaction
                .execute(
                    "INSERT INTO jobs(task_id,kind,target_kind,target_source_id,state,revision,idempotency_key,request_fingerprint,foreground_budget_ms,created_at_ms,started_at_ms,finished_at_ms,updated_at_ms)
                     VALUES(?1,'rss_atom_sync','all_enabled_rss_atom',NULL,'succeeded',1,?2,?3,30000,?4,?4,?4,?4)",
                    sql_params![&task_id, format!("retention-{index}"), format!("retention-fingerprint-{index}"), i64::try_from(index + 2).expect("timestamp")],
                )
                .expect("insert terminal task");
            transaction
                .execute(
                    "INSERT INTO sync_runs(sync_run_id,task_id,scope,outcome,started_at_ms,finished_at_ms)
                     VALUES(?1,?2,'all_enabled_rss_atom','succeeded_zero_results',?3,?3)",
                    sql_params![format!("run:{:024x}", index + 10), task_id, i64::try_from(index + 2).expect("timestamp")],
                )
                .expect("insert terminal run");
        }
        prune_terminal_sync_history(&transaction).expect("prune oldest run");
        let removed: u32 = transaction
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE task_id=?1",
                [&retained_task_id],
                |row| row.get(0),
            )
            .expect("old task removed");
        let retained: (u32, u32, u32, u32, u32) = transaction
            .query_row(
                "SELECT (SELECT COUNT(*) FROM intel_items WHERE data_origin='real'),
                        (SELECT COUNT(*) FROM item_provenance WHERE source_id=?1),
                        (SELECT COUNT(*) FROM source_entry_checkpoints WHERE source_id=?1),
                        (SELECT COUNT(*) FROM intel_associations),
                        (SELECT COUNT(*) FROM intel_association_members)",
                [&source_id],
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
            .expect("retained facts, provenance, checkpoint, and association");
        assert_eq!(removed, 0);
        assert_eq!(retained, (1, 1, 1, 1, 1));
    }

    #[test]
    fn contradictory_result_aggregates_fail_closed() {
        let mut store = DemoStore::open_in_memory().expect("v7");
        let source_id = save_test_source(
            &mut store,
            "https://example.com/result-corruption.xml",
            "source-result-corruption",
        );
        let plan = start_and_claim(&mut store, &source_id, "sync-result-corruption");
        store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: Vec::new(),
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: true,
                },
            )
            .expect("commit");
        let run_id = store
            .task_snapshot(&plan.task.task_id)
            .expect("snapshot")
            .result_ref
            .expect("v7 run");
        store
            .connection
            .execute(
                "UPDATE sync_runs SET inserted_count=1 WHERE sync_run_id=?1",
                [run_id.as_str()],
            )
            .expect("inject contradiction");
        let error = store
            .get_sync_result(&GetSyncResultInputV1 {
                contract_version: 1,
                sync_run_id: run_id,
                cursor: None,
                limit: 100,
            })
            .expect_err("contradiction rejected");
        assert_eq!(error.code(), "storage.source");
    }

    #[test]
    fn atomic_sync_commit_rolls_back_source_when_job_projection_write_fails() {
        let mut store = DemoStore::open_in_memory().expect("v6");
        let source_id = save_test_source(
            &mut store,
            "https://example.com/atomic.xml",
            "source-sync-atomic",
        );
        let plan = start_and_claim(&mut store, &source_id, "sync-atomic");
        let before = store
            .source_fetch_state(&plan.sources[0].source_id)
            .expect("before");
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_job_projection BEFORE UPDATE ON job_source_states
                 BEGIN SELECT RAISE(FAIL, 'projection rejected'); END;",
            )
            .expect("install trigger");

        store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: Vec::new(),
                    etag: Some("must-rollback".to_owned()),
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: true,
                },
            )
            .expect_err("projection failure rolls back source");
        let after = store
            .source_fetch_state(&plan.sources[0].source_id)
            .expect("after");
        assert_eq!(after, before);
        let result_projection: (String, i64, i64, i64, i64) = store
            .connection
            .query_row(
                "SELECT status,inserted_count,updated_count,skipped_count,failed_count
                 FROM sync_source_results WHERE source_id=?1",
                [&plan.sources[0].source_id],
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
            .expect("result projection");
        assert_eq!(result_projection, ("running".to_owned(), 0, 0, 0, 0));
        let item_count: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM sync_result_items", [], |row| {
                row.get(0)
            })
            .expect("result item count");
        assert_eq!(item_count, 0);
    }

    #[test]
    fn shared_url_groups_independent_facts_and_url_change_moves_membership() {
        let mut store = DemoStore::open_in_memory().expect("v9");
        let source_a = save_test_source(
            &mut store,
            "https://alpha.example/feed.xml",
            "source-association-alpha",
        );
        let source_b = save_test_source(
            &mut store,
            "https://beta.example/feed.xml",
            "source-association-beta",
        );
        let source_c = save_test_source(
            &mut store,
            "https://gamma.example/feed.xml",
            "source-association-gamma",
        );
        for (source_id, key, entry) in [
            (&source_a, "sync-association-alpha", "alpha-entry"),
            (&source_b, "sync-association-beta", "beta-entry"),
            (&source_c, "sync-association-gamma", "gamma-entry"),
        ] {
            let plan = start_and_claim(&mut store, source_id, key);
            store
                .commit_sync_source_success(
                    &plan.task.task_id,
                    plan.task.revision,
                    &plan.sources[0],
                    &FetchIncrementalResult {
                        candidates: vec![test_candidate(
                            entry,
                            "Independent title",
                            "https://shared.example/event",
                            entry,
                        )],
                        etag: None,
                        last_modified: None,
                        adapter_cursor: None,
                        not_modified: false,
                    },
                )
                .expect("association commit");
        }
        let grouped: (i64, i64, i64) = store
            .connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM intel_associations),
                        (SELECT COUNT(*) FROM intel_association_members),
                        (SELECT COUNT(*) FROM intel_items WHERE data_origin='real')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("grouped facts");
        assert_eq!(grouped, (1, 3, 3));

        let plan = start_and_claim(&mut store, &source_a, "sync-association-alpha-move");
        store
            .commit_sync_source_success(
                &plan.task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![test_candidate(
                        "alpha-entry",
                        "Changed URL",
                        "https://other.example/event",
                        "alpha-entry-v2",
                    )],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("membership move");
        let member_counts: Vec<i64> = store
            .connection
            .prepare(
                "SELECT COUNT(*) FROM intel_association_members GROUP BY association_id ORDER BY COUNT(*)",
            )
            .expect("membership query")
            .query_map([], |row| row.get(0))
            .expect("membership rows")
            .collect::<Result<_, _>>()
            .expect("membership counts");
        assert_eq!(member_counts, vec![1, 2]);
    }

    #[test]
    fn matching_metadata_and_hash_never_create_a_multi_member_association() {
        // [P0] Similar metadata is not evidence: only canonical original_url may associate facts.
        let mut store = DemoStore::open_in_memory().expect("v9");
        let sources = [
            save_test_source(
                &mut store,
                "https://publisher.example/alpha.xml",
                "source-near-match-alpha",
            ),
            save_test_source(
                &mut store,
                "https://publisher.example/beta.xml",
                "source-near-match-beta",
            ),
            save_test_source(
                &mut store,
                "https://publisher.example/gamma.xml",
                "source-near-match-gamma",
            ),
        ];
        for (index, source_id) in sources.iter().enumerate() {
            let plan = start_and_claim(&mut store, source_id, &format!("sync-near-match-{index}"));
            let mut candidate = test_candidate(
                &format!("near-match-entry-{index}"),
                "Same event-like title",
                &format!("https://events.example/story-{index}"),
                "same-content-hash",
            );
            candidate.author = Some("Same author".to_owned());
            candidate.summary = Some("Same event-like summary".to_owned());
            store
                .commit_sync_source_success(
                    &plan.task.task_id,
                    plan.task.revision,
                    &plan.sources[0],
                    &FetchIncrementalResult {
                        candidates: vec![candidate],
                        etag: None,
                        last_modified: None,
                        adapter_cursor: None,
                        not_modified: false,
                    },
                )
                .expect("near-match commit");
        }

        let (
            fact_count,
            provenance_count,
            source_count,
            publisher_count,
            title_count,
            content_hash_count,
            summary_count,
        ): (i64, i64, i64, i64, i64, i64, i64) = store.connection.query_row(
                "SELECT (SELECT COUNT(*) FROM intel_items WHERE data_origin='real'),
                        (SELECT COUNT(*) FROM item_provenance),
                        (SELECT COUNT(DISTINCT source_id) FROM intel_items WHERE data_origin='real'),
                        (SELECT COUNT(DISTINCT publisher) FROM intel_items WHERE data_origin='real'),
                        (SELECT COUNT(DISTINCT title) FROM intel_items WHERE data_origin='real'),
                        (SELECT COUNT(DISTINCT content_hash) FROM intel_items WHERE data_origin='real'),
                        (SELECT COUNT(DISTINCT source_summary) FROM intel_contents)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
        )
        .expect("independent fact evidence");
        let multi_member_associations: i64 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM (
                    SELECT association_id FROM intel_association_members
                    GROUP BY association_id HAVING COUNT(*) > 1
                 )",
                [],
                |row| row.get(0),
            )
            .expect("multi-member association count");

        assert_eq!(fact_count, 3);
        assert_eq!(provenance_count, 3);
        assert_eq!(source_count, 3);
        assert_eq!(publisher_count, 1);
        assert_eq!(title_count, 1);
        assert_eq!(content_hash_count, 1);
        assert_eq!(summary_count, 1);
        assert_eq!(multi_member_associations, 0);
    }

    #[test]
    fn unchanged_replay_monotonically_refreshes_association_observation() {
        let mut store = DemoStore::open_in_memory().expect("v9");
        let source_id = save_test_source(
            &mut store,
            "https://observation.example/feed.xml",
            "source-association-observation",
        );
        let first = start_and_claim(&mut store, &source_id, "sync-association-observation-one");
        let candidate = test_candidate(
            "observation-entry",
            "Observation",
            "https://observation.example/event",
            "same-version",
        );
        store
            .commit_sync_source_success(
                &first.task.task_id,
                first.task.revision,
                &first.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![candidate.clone()],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("first observation");
        let future_ms = 4_000_000_000_000_i64;
        store
            .connection
            .execute(
                "UPDATE source_entry_checkpoints SET last_seen_at_ms=?1 WHERE source_id=?2",
                sql_params![future_ms, source_id],
            )
            .expect("advance observation clock");

        let replay = start_and_claim(&mut store, &source_id, "sync-association-observation-two");
        store
            .commit_sync_source_success(
                &replay.task.task_id,
                replay.task.revision,
                &replay.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![candidate],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("unchanged observation");
        let observation: (i64, String, String) = store
            .connection
            .query_row(
                "SELECT COUNT(*),MIN(m.last_observed_at),MAX(a.last_observed_at)
                 FROM intel_association_members m JOIN intel_associations a USING(association_id)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("association observation");
        let expected = unix_ms_to_rfc3339(u64::try_from(future_ms).unwrap());
        assert_eq!(observation, (1, expected.clone(), expected));
    }

    #[test]
    fn concurrent_opposite_source_commits_converge_on_one_association() {
        let scoped = ScopedAssociationDatabase::new();
        let mut first_store = DemoStore::open(&scoped.0).expect("file-backed v9");
        let source_a = save_test_source(
            &mut first_store,
            "https://concurrent-a.example/feed.xml",
            "source-concurrent-a",
        );
        let source_b = save_test_source(
            &mut first_store,
            "https://concurrent-b.example/feed.xml",
            "source-concurrent-b",
        );
        let mut second_store = DemoStore::open(&scoped.0).expect("second connection");
        let plan_a = start_and_claim(&mut first_store, &source_a, "sync-concurrent-a");
        let plan_b = start_and_claim(&mut second_store, &source_b, "sync-concurrent-b");
        let worker_cases = [
            (first_store, plan_a, "entry-a", "A"),
            (second_store, plan_b, "entry-b", "B"),
        ];
        let barrier = Arc::new(Barrier::new(worker_cases.len() + 1));

        let handles = worker_cases
            .into_iter()
            .map(|(mut store, plan, entry, title)| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    store.commit_sync_source_success(
                        &plan.task.task_id,
                        plan.task.revision,
                        &plan.sources[0],
                        &FetchIncrementalResult {
                            candidates: vec![test_candidate(
                                entry,
                                title,
                                "https://concurrent.example/event",
                                entry,
                            )],
                            etag: None,
                            last_modified: None,
                            adapter_cursor: None,
                            not_modified: false,
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        for handle in handles {
            handle.join().expect("worker join").expect("source commit");
        }

        let reopened = DemoStore::open(&scoped.0).expect("reopen concurrent result");
        let counts: (i64, i64, i64) = reopened
            .connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM intel_associations),
                        (SELECT COUNT(*) FROM intel_association_members),
                        (SELECT COUNT(DISTINCT item_id) FROM intel_association_members)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("converged association");
        assert_eq!(counts, (1, 2, 2));
    }
}
