use radar_core::application::demo::{DemoCatalog, DemoItem, DemoPage, DemoStore};
use radar_core::contracts::errors::AppError;
use radar_ffi::api;
use radar_ffi::error::install_redacting_panic_hook;
use radar_ffi::mapping::{AppErrorWire, HealthStatusWire};
use serde::Serialize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Mutex;

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

pub struct DemoState {
    database_path: Result<PathBuf, AppError>,
    store: Mutex<Option<DemoStore>>,
}

impl DemoState {
    pub fn new(database_path: Result<PathBuf, AppError>) -> Self {
        Self {
            database_path,
            store: Mutex::new(None),
        }
    }

    #[cfg(test)]
    fn from_store(store: DemoStore) -> Self {
        Self {
            database_path: Err(AppError::internal_generated("test-store-only")),
            store: Mutex::new(Some(store)),
        }
    }

    fn with_store<T>(
        &self,
        action: impl FnOnce(&mut DemoStore) -> Result<T, AppError>,
    ) -> Result<T, Box<CommandErrorV1>> {
        let mut guard = self.store.lock().map_err(|_| {
            Box::new(CommandErrorV1::from(AppError::internal_generated(
                "demo-state-lock",
            )))
        })?;
        if guard.is_none() {
            let path = self
                .database_path
                .as_ref()
                .map_err(|error| Box::new(CommandErrorV1::from(error.clone())))?;
            *guard =
                Some(DemoStore::open(path).map_err(|error| Box::new(CommandErrorV1::from(error)))?);
        }
        let store = guard.as_mut().expect("store initialized above");
        install_redacting_panic_hook();
        catch_unwind(AssertUnwindSafe(|| action(store)))
            .map_err(|_| {
                Box::new(CommandErrorV1::from(AppError::internal_generated(
                    "demo-command-panic",
                )))
            })?
            .map_err(|error| Box::new(CommandErrorV1::from(error)))
    }
}

/// Loads the deterministic demo catalog through the shared core.
///
/// # Errors
///
/// Returns the stable command error wire contract when storage initialization or querying fails.
#[allow(clippy::needless_pass_by_value)] // Tauri owns deserialized command arguments and state handles.
#[tauri::command]
pub fn demo_bootstrap_v1(
    state: tauri::State<'_, DemoState>,
) -> Result<DemoCatalog, Box<CommandErrorV1>> {
    state.with_store(DemoStore::bootstrap)
}

/// Searches the demo catalog with an optional track filter.
///
/// # Errors
///
/// Returns the stable command error wire contract when validation or querying fails.
#[allow(clippy::needless_pass_by_value)] // Tauri owns deserialized command arguments and state handles.
#[tauri::command]
pub fn demo_search_v1(
    query: String,
    track: Option<String>,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoCatalog, Box<CommandErrorV1>> {
    state.with_store(|store| store.search(&query, track.as_deref()))
}

/// Lists one deterministic page of demo intelligence.
///
/// # Errors
/// Returns the stable command error contract for invalid pagination or query failure.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn demo_list_v1(
    cursor: Option<String>,
    limit: u32,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoPage, Box<CommandErrorV1>> {
    state.with_store(|store| store.list_page(None, cursor.as_deref(), limit))
}

/// Filters and lists one deterministic page by track.
///
/// # Errors
/// Returns the stable command error contract for invalid pagination or query failure.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn demo_filter_v1(
    track: String,
    cursor: Option<String>,
    limit: u32,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoPage, Box<CommandErrorV1>> {
    state.with_store(|store| store.list_page(Some(&track), cursor.as_deref(), limit))
}

/// Returns one demo detail by its opaque identifier.
///
/// # Errors
///
/// Returns the stable command error wire contract when the identifier is absent or querying fails.
#[allow(clippy::needless_pass_by_value)] // Tauri owns deserialized command arguments and state handles.
#[tauri::command]
pub fn demo_detail_v1(
    id: String,
    state: tauri::State<'_, DemoState>,
) -> Result<DemoItem, Box<CommandErrorV1>> {
    state.with_store(|store| store.detail(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
