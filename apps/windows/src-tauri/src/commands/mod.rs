use radar_core::application::demo::{DemoCatalog, DemoEvidenceDetail, DemoPage, DemoStore};
use radar_core::application::setup::{SaveSetupStepInputV1, SetupProgressV1};
use radar_core::application::sources::{ProbedSource, probe_source_for_save};
use radar_core::application::sync::fetch_sync_source;
use radar_core::contracts::dto::configuration_validation::{
    ConfigurationValidationResultV1, ConfigurationViewV1, SaveConfigurationInputV1,
    ValidateConfigurationInputV1,
};
use radar_core::contracts::dto::intel_detail::{
    IntelEvidenceDetailV1, OpenIntelOriginalInputV1, OpenOriginalReceiptV1,
    QueryIntelEvidenceDetailInputV1,
};
use radar_core::contracts::dto::intel_feed::{IntelFeedPageV1, QueryIntelFeedInputV1};
use radar_core::contracts::dto::source::{SaveSourceInputV1, SourcePageV1, SourceViewV1};
use radar_core::contracts::dto::sync::{
    GetSyncResultInputV1, StartSyncInputV1, SyncHealthSummaryV1, SyncResultPageV1, TaskRefV1,
    TaskSnapshotV1,
};
use radar_core::contracts::errors::{AppError, ErrorCode};
use radar_core::domain::sources::{FetchIncrementalResult, IncrementalFetchRequest};
use radar_ffi::api;
use radar_ffi::error::install_redacting_panic_hook;
use radar_ffi::mapping::{AppErrorWire, HealthStatusWire};
use serde::Serialize;
use std::future::{Future, poll_fn};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_CONCURRENT_SOURCE_PROBES: usize = 4;

struct SourceProbePermit(Arc<AtomicUsize>);

impl Drop for SourceProbePermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Serialize)]
pub struct HealthResponseV1 {
    contract_version: u32,
    status: String,
    checked_at: Option<String>,
}

impl From<HealthStatusWire> for HealthResponseV1 {
    fn from(value: HealthStatusWire) -> Self {
        Self {
            contract_version: value.contract_version,
            status: value.status,
            checked_at: value.checked_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CommandErrorV1 {
    contract_version: u32,
    code: &'static str,
    category: &'static str,
    message_key: &'static str,
    retryability: &'static str,
    source_id: Option<String>,
    task_id: Option<String>,
    details_allowlisted: String,
    correlation_id: String,
}

impl From<AppError> for CommandErrorV1 {
    fn from(value: AppError) -> Self {
        let wire = AppErrorWire::from(&value);
        Self {
            contract_version: wire.contract_version,
            code: wire.code,
            category: wire.category,
            message_key: wire.message_key,
            retryability: wire.retryability,
            source_id: wire.source_id,
            task_id: wire.task_id,
            details_allowlisted: wire.details_allowlisted,
            correlation_id: wire.correlation_id,
        }
    }
}

#[tauri::command]
/// Returns the shared-core v1 health contract through the only release IPC command.
///
/// # Errors
/// Returns a stable, redacted v1 command error if the core adapter fails.
pub fn health_v1() -> Result<HealthResponseV1, Box<CommandErrorV1>> {
    api::health_v1()
        .map(HealthResponseV1::from)
        .map_err(|error| Box::new(CommandErrorV1::from(error)))
}

#[derive(Clone)]
pub struct DemoState {
    database_path: Arc<Result<PathBuf, AppError>>,
    store: Arc<Mutex<Option<DemoStore>>>,
    active_source_probes: Arc<AtomicUsize>,
}

impl DemoState {
    pub fn new(database_path: Result<PathBuf, AppError>) -> Self {
        Self {
            database_path: Arc::new(database_path),
            store: Arc::new(Mutex::new(None)),
            active_source_probes: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[cfg(test)]
    fn from_store(store: DemoStore) -> Self {
        Self {
            database_path: Arc::new(Err(AppError::internal_generated("test-store-only"))),
            store: Arc::new(Mutex::new(Some(store))),
            active_source_probes: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn acquire_source_probe(&self) -> Result<SourceProbePermit, Box<CommandErrorV1>> {
        self.active_source_probes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_CONCURRENT_SOURCE_PROBES).then_some(active + 1)
            })
            .map_err(|_| {
                Box::new(CommandErrorV1::from(
                    AppError::from_code(ErrorCode::RateLimitedSource, "source-probe-capacity")
                        .with_retry_after_ms(1_000),
                ))
            })?;
        Ok(SourceProbePermit(Arc::clone(&self.active_source_probes)))
    }

    fn with_store<T>(
        &self,
        action: impl FnOnce(&mut DemoStore) -> Result<T, AppError>,
    ) -> Result<T, Box<CommandErrorV1>> {
        install_redacting_panic_hook();
        let mut guard = self.store.lock().map_err(|_| {
            Box::new(CommandErrorV1::from(AppError::internal_generated(
                "demo-state-lock",
            )))
        })?;
        catch_unwind(AssertUnwindSafe(|| {
            if guard.is_none() {
                let path = self.database_path.as_ref().as_ref().map_err(Clone::clone)?;
                *guard = Some(DemoStore::open(path)?);
            }
            let store = guard
                .as_mut()
                .ok_or_else(|| AppError::internal_generated("demo-state-not-initialized"))?;
            action(store)
        }))
        .map_err(|_| {
            Box::new(CommandErrorV1::from(AppError::internal_generated(
                "demo-command-panic",
            )))
        })?
        .map_err(|error| Box::new(CommandErrorV1::from(error)))
    }

    async fn with_store_blocking<T>(
        &self,
        action: impl FnOnce(&mut DemoStore) -> Result<T, AppError> + Send + 'static,
    ) -> Result<T, Box<CommandErrorV1>>
    where
        T: Send + 'static,
    {
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move || state.with_store(action))
            .await
            .map_err(|_| {
                Box::new(CommandErrorV1::from(AppError::internal_generated(
                    "demo-command-join",
                )))
            })?
    }
}

/// Loads the deterministic demo catalog through the shared core.
///
/// # Errors
///
/// Returns the stable command error wire contract when storage initialization or querying fails.
#[allow(clippy::needless_pass_by_value)] // Tauri owns deserialized command arguments and state handles.
#[tauri::command]
pub async fn demo_bootstrap_v1(
    state: tauri::State<'_, DemoState>,
) -> Result<DemoCatalog, Box<CommandErrorV1>> {
    state.with_store_blocking(DemoStore::bootstrap).await
}

/// Searches the demo catalog with an optional track filter.
///
/// # Errors
///
/// Returns the stable command error wire contract when validation or querying fails.
#[allow(clippy::needless_pass_by_value)] // Tauri owns deserialized command arguments and state handles.
#[tauri::command]
pub async fn demo_search_v1(
    query: String,
    track: Option<String>,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoCatalog, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.search(&query, track.as_deref()))
        .await
}

/// Lists one deterministic page of demo intelligence.
///
/// # Errors
/// Returns the stable command error contract for invalid pagination or query failure.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn demo_list_v1(
    cursor: Option<String>,
    limit: u32,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoPage, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.list_page(None, cursor.as_deref(), limit))
        .await
}

/// Filters and lists one deterministic page by track.
///
/// # Errors
/// Returns the stable command error contract for invalid pagination or query failure.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn demo_filter_v1(
    track: String,
    cursor: Option<String>,
    limit: u32,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoPage, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.list_page(Some(&track), cursor.as_deref(), limit))
        .await
}

/// Returns one demo detail by its opaque identifier.
///
/// # Errors
///
/// Returns the stable command error wire contract when the identifier is absent or querying fails.
#[allow(clippy::needless_pass_by_value)] // Tauri owns deserialized command arguments and state handles.
#[tauri::command]
pub async fn demo_detail_v1(
    id: String,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoEvidenceDetail, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.detail(&id))
        .await
}

/// Returns device-local progressive setup state and core-owned defaults.
///
/// # Errors
/// Returns the stable command error contract when local storage cannot be read.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn setup_progress_v1(
    state: tauri::State<'_, DemoState>,
) -> Result<SetupProgressV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(|store| store.get_setup_progress())
        .await
}

/// Atomically saves one progressive setup intent.
///
/// # Errors
/// Returns a stable validation, conflict, storage, or migration command error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn save_setup_step_v1(
    input: SaveSetupStepInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<SetupProgressV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.save_setup_step(&input))
        .await
}

/// Returns the current immutable device-local attention configuration.
///
/// # Errors
/// Returns the stable command error contract when local storage cannot be read.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn configuration_v1(
    state: tauri::State<'_, DemoState>,
) -> Result<ConfigurationViewV1, Box<CommandErrorV1>> {
    configuration_from_state(&state).await
}

async fn configuration_from_state(
    state: &DemoState,
) -> Result<ConfigurationViewV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(|store| store.get_configuration())
        .await
}

/// Validates a configuration without persisting it.
///
/// # Errors
/// Returns a stable validation or storage command error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn validate_configuration_v1(
    input: ValidateConfigurationInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<ConfigurationValidationResultV1, Box<CommandErrorV1>> {
    validate_configuration_from_state(&state, input).await
}

async fn validate_configuration_from_state(
    state: &DemoState,
    input: ValidateConfigurationInputV1,
) -> Result<ConfigurationValidationResultV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.validate_attention_configuration(&input))
        .await
}

/// Atomically appends and selects a validated configuration version.
///
/// # Errors
/// Returns a stable validation, conflict, or storage command error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn save_configuration_v1(
    input: SaveConfigurationInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<ConfigurationViewV1, Box<CommandErrorV1>> {
    save_configuration_from_state(&state, input).await
}

async fn save_configuration_from_state(
    state: &DemoState,
    input: SaveConfigurationInputV1,
) -> Result<ConfigurationViewV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.save_attention_configuration(&input))
        .await
}

/// Probes and atomically saves one public HTTPS RSS/Atom source.
///
/// # Errors
/// Returns a stable validation, network, format, conflict, or storage error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn save_source_v1(
    input: SaveSourceInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<SourceViewV1, Box<CommandErrorV1>> {
    let replay_input = input.clone();
    if let Some(response) = state
        .with_store_blocking(move |store| store.replay_saved_source(&replay_input))
        .await?
    {
        return Ok(response);
    }
    let current_revision = state
        .with_store_blocking(|store| store.get_configuration().map(|view| view.revision))
        .await?;
    if current_revision != input.expected_configuration_revision {
        return Err(Box::new(CommandErrorV1::from(AppError::from_code(
            ErrorCode::ConflictConfigurationRevision,
            "source-preflight-revision",
        ))));
    }
    let _permit = state.acquire_source_probe()?;
    let probe_input = input.clone();
    let probed =
        tauri::async_runtime::spawn(async move { probe_source_for_save(&probe_input).await })
            .await
            .map_err(|_| {
                Box::new(CommandErrorV1::from(AppError::internal_generated(
                    "source-probe-panic",
                )))
            })?
            .map_err(|error| Box::new(CommandErrorV1::from(error)))?;
    save_source_from_state(&state, input, probed).await
}

async fn save_source_from_state(
    state: &DemoState,
    input: SaveSourceInputV1,
    probed: ProbedSource,
) -> Result<SourceViewV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.save_probed_source(&input, &probed))
        .await
}

/// Queries one source page from the device-local source projection.
///
/// # Errors
/// Returns a stable validation or storage error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn query_sources_v1(
    cursor: Option<String>,
    limit: u32,
    state: tauri::State<'_, DemoState>,
) -> Result<SourcePageV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.query_sources(cursor.as_deref(), limit))
        .await
}

/// Persists one RSS-only foreground synchronization intent and returns immediately.
///
/// The detached future is owned by the current Tauri runtime and is not a background-service
/// promise. Each source fetch is panic-contained, and no store lock is held across network I/O.
///
/// # Errors
/// Returns a stable validation, conflict, storage, or migration error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn start_sync_v1(
    input: StartSyncInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<TaskRefV1, Box<CommandErrorV1>> {
    let task = start_sync_from_state(&state, input).await?;
    let worker_state = state.inner().clone();
    let worker_task = task.clone();
    tauri::async_runtime::spawn(async move {
        let task_id = worker_task.task_id.clone();
        if execute_sync_task(worker_state.clone(), worker_task)
            .await
            .is_err()
        {
            let _ = worker_state
                .with_store_blocking(move |store| store.fail_active_sync_task(&task_id))
                .await;
        }
    });
    Ok(task)
}

async fn start_sync_from_state(
    state: &DemoState,
    input: StartSyncInputV1,
) -> Result<TaskRefV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.start_sync(&input))
        .await
}

async fn execute_sync_task(state: DemoState, task: TaskRefV1) -> Result<(), Box<CommandErrorV1>> {
    let task_id = task.task_id;
    let claim_task_id = task_id.clone();
    let plan = state
        .with_store_blocking(move |store| store.claim_sync_task(&claim_task_id, task.revision))
        .await?;
    let budget = Duration::from_millis(u64::from(plan.foreground_budget_ms));
    let work = execute_sync_plan(
        state.clone(),
        task_id.clone(),
        plan.task.revision,
        plan.sources,
    );
    let timer = async move {
        let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(budget)).await;
    };
    let outcome = race_with_budget(work, timer).await;
    if let Some(result) = outcome {
        result
    } else {
        state
            .with_store_blocking(move |store| store.cancel_sync_task_for_budget(&task_id))
            .await?;
        Ok(())
    }
}

async fn race_with_budget<Work, Timer>(work: Work, timer: Timer) -> Option<Work::Output>
where
    Work: Future,
    Timer: Future<Output = ()>,
{
    let mut work = Box::pin(work);
    let mut timer = Box::pin(timer);
    poll_fn(|context| {
        if let Poll::Ready(result) = work.as_mut().poll(context) {
            return Poll::Ready(Some(result));
        }
        if timer.as_mut().poll(context).is_ready() {
            return Poll::Ready(None);
        }
        Poll::Pending
    })
    .await
}

async fn execute_sync_plan(
    state: DemoState,
    task_id: String,
    expected_revision: u64,
    sources: Vec<IncrementalFetchRequest>,
) -> Result<(), Box<CommandErrorV1>> {
    let panic_task_id = task_id.clone();
    execute_sync_plan_with_fetch(state, task_id, expected_revision, sources, move |request| {
        let task_id = panic_task_id.clone();
        async move {
            let fetch_request = request.clone();
            tauri::async_runtime::spawn(async move { fetch_sync_source(&fetch_request).await })
                .await
                .unwrap_or_else(|_| {
                    Err(AppError::internal_generated("sync-source-worker-panic")
                        .with_task_id(&task_id)
                        .with_source_id(&request.source_id))
                })
        }
    })
    .await
}

async fn execute_sync_plan_with_fetch<F, Fut>(
    state: DemoState,
    task_id: String,
    mut expected_revision: u64,
    sources: Vec<IncrementalFetchRequest>,
    mut fetch: F,
) -> Result<(), Box<CommandErrorV1>>
where
    F: FnMut(IncrementalFetchRequest) -> Fut,
    Fut: Future<Output = Result<FetchIncrementalResult, AppError>>,
{
    for request in sources {
        let outcome = fetch(request.clone()).await;
        let committed =
            commit_sync_outcome(&state, task_id.clone(), expected_revision, request, outcome)
                .await?;
        expected_revision = committed.revision;
    }
    Ok(())
}

async fn commit_sync_outcome(
    state: &DemoState,
    task_id: String,
    expected_revision: u64,
    request: IncrementalFetchRequest,
    outcome: Result<FetchIncrementalResult, AppError>,
) -> Result<TaskRefV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| match outcome {
            Ok(result) => {
                store.commit_sync_source_success(&task_id, expected_revision, &request, &result)
            }
            Err(error) if error.code() == ErrorCode::InternalUnexpected.as_str() => {
                store.commit_sync_source_internal_failure(&task_id, expected_revision, &request)
            }
            Err(error) => store.commit_sync_source_failure(
                &task_id,
                expected_revision,
                &request,
                &error,
                observed_at_ms()?,
            ),
        })
        .await
}

fn observed_at_ms() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::internal_generated("sync-observed-time"))
        .and_then(|duration| {
            u64::try_from(duration.as_millis())
                .map_err(|_| AppError::internal_generated("sync-observed-time-range"))
        })
}

/// Returns one complete persistent RSS synchronization task projection.
///
/// # Errors
/// Returns a stable validation or storage error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn task_v1(
    task_id: String,
    state: tauri::State<'_, DemoState>,
) -> Result<TaskSnapshotV1, Box<CommandErrorV1>> {
    task_from_state(&state, task_id).await
}

async fn task_from_state(
    state: &DemoState,
    task_id: String,
) -> Result<TaskSnapshotV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.task_snapshot(&task_id))
        .await
}

/// Returns the current RSS-only synchronization and delivery-readiness projection.
///
/// # Errors
/// Returns a stable storage or migration error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn sync_health_v1(
    state: tauri::State<'_, DemoState>,
) -> Result<SyncHealthSummaryV1, Box<CommandErrorV1>> {
    sync_health_from_state(&state).await
}

async fn sync_health_from_state(
    state: &DemoState,
) -> Result<SyncHealthSummaryV1, Box<CommandErrorV1>> {
    state.with_store_blocking(|store| store.sync_health()).await
}

/// Returns one committed RSS/Atom synchronization result page without starting network work.
///
/// # Errors
/// Returns a stable validation, storage, or migration error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn get_sync_result_v1(
    input: GetSyncResultInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<SyncResultPageV1, Box<CommandErrorV1>> {
    sync_result_from_state(&state, input).await
}

async fn sync_result_from_state(
    state: &DemoState,
    input: GetSyncResultInputV1,
) -> Result<SyncResultPageV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.get_sync_result(&input))
        .await
}

/// Returns one read-only page from the current real RSS intelligence projection.
///
/// # Errors
/// Returns a stable validation, storage, or migration error.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn query_intel_feed_v1(
    input: QueryIntelFeedInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<IntelFeedPageV1, Box<CommandErrorV1>> {
    intel_feed_from_state(&state, input).await
}

/// Returns one bounded real RSS evidence detail by stable item identity.
///
/// # Errors
/// Returns a stable validation or storage error without leaking database details.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn query_intel_evidence_detail_v1(
    input: QueryIntelEvidenceDetailInputV1,
    state: tauri::State<'_, DemoState>,
) -> Result<IntelEvidenceDetailV1, Box<CommandErrorV1>> {
    query_intel_evidence_detail_from_state(&state, input).await
}

async fn query_intel_evidence_detail_from_state(
    state: &DemoState,
    input: QueryIntelEvidenceDetailInputV1,
) -> Result<IntelEvidenceDetailV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.query_intel_evidence_detail(&input))
        .await
}

/// Resolves one stable provenance intent and opens it with the system browser.
///
/// # Errors
/// Returns a stable validation/storage/platform error; neither response nor error contains a URL.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn open_intel_original_v1(
    input: OpenIntelOriginalInputV1,
    state: tauri::State<'_, DemoState>,
    app: tauri::AppHandle,
) -> Result<OpenOriginalReceiptV1, Box<CommandErrorV1>> {
    open_intel_original_from_state(&state, input, |link| {
        crate::platform::windows::external_links::open_validated_link(&app, link)
    })
    .await
}

async fn open_intel_original_from_state(
    state: &DemoState,
    input: OpenIntelOriginalInputV1,
    opener: impl FnOnce(
        &radar_core::application::intel_detail::ValidatedOriginalLink,
    ) -> Result<(), AppError>,
) -> Result<OpenOriginalReceiptV1, Box<CommandErrorV1>> {
    let resolve_input = input.clone();
    let link = state
        .with_store_blocking(move |store| store.resolve_intel_original(&resolve_input))
        .await?;
    catch_unwind(AssertUnwindSafe(|| opener(&link)))
        .map_err(|_| {
            Box::new(CommandErrorV1::from(AppError::internal_generated(
                "external-link-panic",
            )))
        })?
        .map_err(|_| {
            Box::new(CommandErrorV1::from(AppError::internal_generated(
                "external-link-open",
            )))
        })?;
    Ok(OpenOriginalReceiptV1 {
        contract_version: 1,
        intel_item_id: link.intel_item_id().to_owned(),
        provenance_id: link.provenance_id().to_owned(),
        status: "requested".to_owned(),
    })
}

async fn intel_feed_from_state(
    state: &DemoState,
    input: QueryIntelFeedInputV1,
) -> Result<IntelFeedPageV1, Box<CommandErrorV1>> {
    state
        .with_store_blocking(move |store| store.query_intel_feed(&input))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use radar_core::contracts::dto::configuration_validation::AttentionConfigurationV1;
    use radar_core::contracts::dto::intel_feed::{
        IntelFeedFiltersV1, IntelFeedSortV1, IntelFeedStreamV1, IntelFeedTimeWindowV1,
        QueryIntelFeedInputV1,
    };
    use radar_core::contracts::dto::source::SaveSourceInputV1;
    use radar_core::contracts::dto::sync::{GetSyncResultInputV1, StartSyncInputV1, SyncTargetV1};
    use radar_core::domain::sources::RawSourceCandidate;
    use serde::Deserialize;

    #[test]
    fn demo_state_contains_panics_without_poisoning_the_store() {
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));

        let error = state
            .with_store::<()>(|_| panic!("private demo panic"))
            .expect_err("panic must become a stable command error");
        assert_eq!(error.code, "internal.unexpected");
        assert!(!format!("{error:?}").contains("private demo panic"));

        let catalog = state
            .with_store(DemoStore::bootstrap)
            .expect("the store must remain usable after a contained panic");
        assert_eq!(catalog.dataset_id, "demo-v1");
    }

    #[test]
    fn intel_feed_helper_returns_empty_page_redacts_errors_and_recovers() {
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));
        let input = QueryIntelFeedInputV1 {
            contract_version: 1,
            stream: IntelFeedStreamV1::HighValue,
            filters: IntelFeedFiltersV1 {
                track_ids: Vec::new(),
                source_ids: Vec::new(),
                time_window: IntelFeedTimeWindowV1::AllTime,
                importance: Vec::new(),
            },
            sort: IntelFeedSortV1::ScoreDesc,
            cursor: None,
            limit: 30,
        };

        tauri::async_runtime::block_on(async {
            let page = intel_feed_from_state(&state, input.clone())
                .await
                .expect("empty feed page");
            assert!(page.items.is_empty());
            assert_eq!(page.stream, IntelFeedStreamV1::HighValue);

            let mut invalid = input.clone();
            invalid.cursor = Some("private-invalid-feed-cursor".to_owned());
            let error = intel_feed_from_state(&state, invalid)
                .await
                .expect_err("invalid cursor must fail closed");
            assert_eq!(error.code, "validation.source");
            assert!(!format!("{error:?}").contains("private-invalid-feed-cursor"));

            let recovered = intel_feed_from_state(&state, input)
                .await
                .expect("store remains usable");
            assert!(recovered.items.is_empty());
        });
    }

    #[derive(Deserialize)]
    struct ConfigurationGolden {
        input: AttentionConfigurationV1,
        expected: ConfigurationValidationResultV1,
    }

    #[test]
    fn shared_configuration_fixture_crosses_the_production_command_helpers() {
        let golden: ConfigurationGolden = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../contracts/fixtures/golden/configuration_validation_v1.json"
        )))
        .expect("shared configuration fixture");
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));
        let validate_input = ValidateConfigurationInputV1 {
            contract_version: 1,
            configuration: golden.input.clone(),
        };
        let wire = serde_json::to_value(&validate_input).expect("serialize command input");
        let validate_input: ValidateConfigurationInputV1 =
            serde_json::from_value(wire).expect("deserialize command input");

        tauri::async_runtime::block_on(async {
            let initial = configuration_from_state(&state)
                .await
                .expect("configuration command helper");
            let validation = validate_configuration_from_state(&state, validate_input)
                .await
                .expect("validation command helper");
            assert_eq!(validation, golden.expected);

            let saved = save_configuration_from_state(
                &state,
                SaveConfigurationInputV1 {
                    contract_version: 1,
                    configuration: golden.input,
                    expected_revision: initial.revision,
                    expected_normalized_config_hash: validation.normalized_config_hash,
                    idempotency_key: "tauri-shared-fixture-save".to_owned(),
                    validation_receipt: validation.validation_receipt,
                },
            )
            .await
            .expect("save command helper");
            assert_eq!(saved.revision, initial.revision + 1);
        });
    }

    #[test]
    fn source_contract_crosses_the_production_command_helper_and_store_recovers() {
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));
        tauri::async_runtime::block_on(async {
            let page = saved_source_page(&state).await;
            let task = queued_sync_task(&state, &page.items[0].source_id).await;
            assert_eq!(task.state.as_str(), "queued");

            let snapshot = task_from_state(&state, task.task_id.clone())
                .await
                .expect("task helper");
            assert_eq!(snapshot.task_id, task.task_id);
            assert_eq!(snapshot.sources.len(), 1);
            assert_eq!(snapshot.sources[0].source_id, page.items[0].source_id);

            let health = sync_health_from_state(&state)
                .await
                .expect("sync health helper");
            assert_eq!(health.pending_task_count, 1);
            assert_eq!(health.readiness.required_source_kinds, ["rss_atom"]);
            assert_eq!(
                health.latest_task.as_ref().map(|latest| &latest.task_id),
                Some(&task.task_id)
            );

            let task_id = task.task_id.clone();
            let expected_revision = task.revision;
            let claim_task_id = task_id.clone();
            let plan = state
                .with_store_blocking(move |store| {
                    store.claim_sync_task(&claim_task_id, expected_revision)
                })
                .await
                .expect("claim sync helper");
            let request = plan
                .sources
                .into_iter()
                .next()
                .expect("one claimed RSS source");
            let failed = commit_sync_outcome(
                &state,
                task_id.clone(),
                plan.task.revision,
                request,
                Err(AppError::internal_generated("test-worker-join")),
            )
            .await
            .expect("internal worker failure helper");
            assert_eq!(failed.state.as_str(), "failed");

            let failed_snapshot = task_from_state(&state, task_id)
                .await
                .expect("failed task snapshot");
            assert_eq!(failed_snapshot.sources[0].state.as_str(), "failed");
            assert_eq!(
                failed_snapshot.sources[0].error_code.as_deref(),
                Some("internal.unexpected")
            );
            let source_after_internal_failure = state
                .with_store_blocking(|store| store.query_sources(None, 100))
                .await
                .expect("source after internal failure");
            assert_eq!(source_after_internal_failure.items, page.items);
            assert_eq!(
                sync_health_from_state(&state)
                    .await
                    .expect("health after internal failure")
                    .pending_task_count,
                0
            );
        });
    }

    #[test]
    fn sync_result_command_helper_succeeds_redacts_errors_and_recovers() {
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));
        tauri::async_runtime::block_on(async {
            let page = saved_source_page(&state).await;
            let task = queued_sync_task(&state, &page.items[0].source_id).await;
            let task_id = task.task_id.clone();
            let plan = state
                .with_store_blocking(move |store| store.claim_sync_task(&task_id, task.revision))
                .await
                .expect("claim result-producing task");
            let request = plan.sources.into_iter().next().expect("one source");
            let completed = commit_sync_outcome(
                &state,
                plan.task.task_id.clone(),
                plan.task.revision,
                request,
                Ok(FetchIncrementalResult {
                    candidates: vec![RawSourceCandidate {
                        stable_external_id: "tauri-result-item".to_owned(),
                        title: Some("Tauri result item".to_owned()),
                        original_url: Some(["https:", "//example.com/items/tauri-result"].concat()),
                        author: None,
                        summary: None,
                        published_at: Some("2026-08-18T08:00:00Z".to_owned()),
                        updated_at: None,
                        content_hash: "a".repeat(64),
                        warnings: Vec::new(),
                    }],
                    etag: Some("tauri-etag".to_owned()),
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                }),
            )
            .await
            .expect("commit result");
            let completed_snapshot = task_from_state(&state, completed.task_id)
                .await
                .expect("completed snapshot");
            let sync_run_id = completed_snapshot.result_ref.expect("v7 result reference");
            let input = GetSyncResultInputV1 {
                contract_version: 1,
                sync_run_id,
                cursor: None,
                limit: 25,
            };
            let result = sync_result_from_state(&state, input.clone())
                .await
                .expect("result command helper");
            assert_eq!(result.items.len(), 1);
            assert_eq!(result.summary.task_id, plan.task.task_id);

            let private_canary = "private-result-cursor-canary";
            let error = sync_result_from_state(
                &state,
                GetSyncResultInputV1 {
                    cursor: Some(private_canary.to_owned()),
                    ..input.clone()
                },
            )
            .await
            .expect_err("invalid cursor must fail closed");
            assert_eq!(error.code, "validation.source");
            assert!(!format!("{error:?}").contains(private_canary));

            let panic_error = state
                .with_store::<()>(|_| panic!("{private_canary}"))
                .expect_err("panic must be contained");
            assert_eq!(panic_error.code, "internal.unexpected");
            assert!(!format!("{panic_error:?}").contains(private_canary));
            assert_eq!(
                sync_result_from_state(&state, input)
                    .await
                    .expect("result helper remains usable")
                    .items
                    .len(),
                1
            );
        });
    }

    #[test]
    fn sync_fetch_seam_runs_without_holding_the_store_mutex() {
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));
        tauri::async_runtime::block_on(async {
            let page = saved_source_page(&state).await;
            let task = queued_sync_task(&state, &page.items[0].source_id).await;
            let task_id = task.task_id.clone();
            let plan = state
                .with_store_blocking(move |store| store.claim_sync_task(&task_id, task.revision))
                .await
                .expect("claim");
            let probe_state = state.clone();
            execute_sync_plan_with_fetch(
                state.clone(),
                plan.task.task_id.clone(),
                plan.task.revision,
                plan.sources,
                move |_| {
                    let probe_state = probe_state.clone();
                    async move {
                        probe_state
                            .with_store(|_| Ok(()))
                            .expect("store mutex is free during fetch await");
                        Ok(FetchIncrementalResult {
                            candidates: Vec::new(),
                            etag: None,
                            last_modified: None,
                            adapter_cursor: None,
                            not_modified: true,
                        })
                    }
                },
            )
            .await
            .expect("execute injected fetch");
            let snapshot = task_from_state(&state, plan.task.task_id)
                .await
                .expect("snapshot");
            assert_eq!(snapshot.state.as_str(), "succeeded");
        });
    }

    #[test]
    fn foreground_budget_cancellation_is_terminal_and_releases_pending_count() {
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));
        tauri::async_runtime::block_on(async {
            let page = saved_source_page(&state).await;
            let task = queued_sync_task(&state, &page.items[0].source_id).await;
            let task_id = task.task_id.clone();
            let expected_revision = task.revision;
            state
                .with_store_blocking(move |store| {
                    store.claim_sync_task(&task_id, expected_revision)
                })
                .await
                .expect("claim");
            let task_id = task.task_id.clone();
            let cancelled = state
                .with_store_blocking(move |store| store.cancel_sync_task_for_budget(&task_id))
                .await
                .expect("cancel for budget");
            assert_eq!(cancelled.state.as_str(), "cancelled");
            assert_eq!(
                sync_health_from_state(&state)
                    .await
                    .expect("health")
                    .pending_task_count,
                0
            );
        });
    }

    #[test]
    fn real_detail_and_original_helpers_keep_the_effect_narrow_and_redacted() {
        let state = DemoState::from_store(DemoStore::open_in_memory().expect("memory store"));
        tauri::async_runtime::block_on(async {
            let (intel_item_id, provenance_id) = seed_real_detail(&state).await;
            let detail = query_intel_evidence_detail_from_state(
                &state,
                QueryIntelEvidenceDetailInputV1 {
                    contract_version: 1,
                    intel_item_id: intel_item_id.clone(),
                },
            )
            .await
            .expect("detail command helper");
            assert_eq!(detail.facts.intel_item_id, intel_item_id);
            assert_eq!(detail.ai_status.as_str(), "unavailable");

            let captured = Arc::new(Mutex::new(Vec::new()));
            let capture = Arc::clone(&captured);
            let receipt = open_intel_original_from_state(
                &state,
                OpenIntelOriginalInputV1 {
                    contract_version: 1,
                    intel_item_id: intel_item_id.clone(),
                    provenance_id: provenance_id.clone(),
                },
                move |link| {
                    capture
                        .lock()
                        .expect("capture lock")
                        .push(link.url().to_owned());
                    Ok(())
                },
            )
            .await
            .expect("open helper");
            assert_eq!(receipt.intel_item_id, intel_item_id);
            assert_eq!(receipt.provenance_id, provenance_id);
            assert_eq!(receipt.status, "requested");
            assert_eq!(captured.lock().expect("capture").len(), 1);

            let private_canary = "private-platform-error-canary";
            let error = open_intel_original_from_state(
                &state,
                OpenIntelOriginalInputV1 {
                    contract_version: 1,
                    intel_item_id,
                    provenance_id,
                },
                |_| Err(AppError::internal_generated(private_canary)),
            )
            .await
            .expect_err("platform failure");
            assert_eq!(error.code, "internal.unexpected");
            assert!(!format!("{error:?}").contains(private_canary));

            let panic_error = open_intel_original_from_state(
                &state,
                OpenIntelOriginalInputV1 {
                    contract_version: 1,
                    intel_item_id: receipt.intel_item_id.clone(),
                    provenance_id: receipt.provenance_id.clone(),
                },
                |_| panic!("private-opener-panic-canary"),
            )
            .await
            .expect_err("platform panic");
            assert_eq!(panic_error.code, "internal.unexpected");
            assert!(!format!("{panic_error:?}").contains("private-opener-panic-canary"));
            let recovered = query_intel_evidence_detail_from_state(
                &state,
                QueryIntelEvidenceDetailInputV1 {
                    contract_version: 1,
                    intel_item_id: receipt.intel_item_id,
                },
            )
            .await
            .expect("store recovers after opener panic");
            assert_eq!(recovered.rule_status.as_str(), "current");
        });
    }

    #[test]
    fn budget_race_is_deterministic_without_wall_clock_waits() {
        tauri::async_runtime::block_on(async {
            assert_eq!(
                race_with_budget(std::future::ready(7_u8), std::future::pending()).await,
                Some(7)
            );
            assert_eq!(
                race_with_budget(std::future::pending::<()>(), std::future::ready(())).await,
                None
            );
        });
    }

    async fn saved_source_page(state: &DemoState) -> SourcePageV1 {
        let revision = configuration_from_state(state)
            .await
            .expect("configuration")
            .revision;
        let private_url = ["https:", "//example.com/feed.xml?private=not-returned"].concat();
        let saved = save_source_from_state(
            state,
            SaveSourceInputV1 {
                contract_version: 1,
                source_kind: "rss_atom".to_owned(),
                url: private_url.clone(),
                expected_configuration_revision: revision,
                idempotency_key: "tauri-source-save".to_owned(),
            },
            ProbedSource::from_adapter_result(
                &private_url,
                radar_core::domain::sources::FetchIncrementalResult {
                    candidates: Vec::new(),
                    etag: None,
                    last_modified: None,
                    adapter_cursor: Some("fixture-probe".to_owned()),
                    not_modified: false,
                },
            )
            .expect("fixture probe"),
        )
        .await
        .expect("source helper");
        assert_eq!(
            saved.display_url,
            ["https:", "//example.com/feed.xml"].concat()
        );
        let page = state
            .with_store_blocking(|store| store.query_sources(None, 100))
            .await
            .expect("query helper");
        assert_eq!(page.items, vec![saved]);
        page
    }

    async fn seed_real_detail(state: &DemoState) -> (String, String) {
        let page = saved_source_page(state).await;
        let task = queued_sync_task(state, &page.items[0].source_id).await;
        let task_id = task.task_id.clone();
        let plan = state
            .with_store_blocking(move |store| store.claim_sync_task(&task_id, task.revision))
            .await
            .expect("claim detail task");
        let request = plan.sources.into_iter().next().expect("detail source");
        let completed = commit_sync_outcome(
            state,
            plan.task.task_id,
            plan.task.revision,
            request,
            Ok(FetchIncrementalResult {
                candidates: vec![RawSourceCandidate {
                    stable_external_id: "tauri-detail-item".to_owned(),
                    title: Some("Tauri evidence detail item".to_owned()),
                    original_url: Some(["https:", "//example.com/items/tauri-detail"].concat()),
                    author: None,
                    summary: Some("Locally persisted evidence summary".to_owned()),
                    published_at: Some("2026-08-20T08:00:00Z".to_owned()),
                    updated_at: None,
                    content_hash: "d".repeat(64),
                    warnings: Vec::new(),
                }],
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: false,
            }),
        )
        .await
        .expect("commit detail item");
        let snapshot = task_from_state(state, completed.task_id)
            .await
            .expect("detail snapshot");
        let result = sync_result_from_state(
            state,
            GetSyncResultInputV1 {
                contract_version: 1,
                sync_run_id: snapshot.result_ref.expect("detail result"),
                cursor: None,
                limit: 25,
            },
        )
        .await
        .expect("detail result page");
        let intel_item_id = result.items[0]
            .intel_item_id
            .clone()
            .expect("normalized detail item id");
        let detail = query_intel_evidence_detail_from_state(
            state,
            QueryIntelEvidenceDetailInputV1 {
                contract_version: 1,
                intel_item_id: intel_item_id.clone(),
            },
        )
        .await
        .expect("seeded detail");
        (intel_item_id, detail.provenance[0].provenance_id.clone())
    }

    async fn queued_sync_task(state: &DemoState, source_id: &str) -> TaskRefV1 {
        start_sync_from_state(
            state,
            StartSyncInputV1 {
                contract_version: 1,
                target: SyncTargetV1::SourceId {
                    source_id: source_id.to_owned(),
                },
                idempotency_key: "tauri-rss-sync-intent".to_owned(),
                foreground_budget_ms: 30_000,
            },
        )
        .await
        .expect("start sync helper")
    }
}
