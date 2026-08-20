pub mod commands;
pub mod platform;

#[cfg(feature = "benchmark-instrumentation")]
fn benchmark_database_path() -> Option<std::path::PathBuf> {
    let requested = std::env::var_os("AI_SUBSCRIBE_BENCHMARK_DATA_DIR")?;
    let requested = std::path::PathBuf::from(requested).canonicalize().ok()?;
    let executable = std::env::current_exe().ok()?;
    let project_root = executable.parent()?.parent()?.parent()?.parent()?;
    let allowed_root = project_root
        .join("target/story-1-6-benchmark")
        .canonicalize()
        .ok()?;
    requested
        .starts_with(allowed_root)
        .then(|| requested.join("ai-subscribe.sqlite3"))
}

/// Starts the native desktop event loop.
///
/// # Panics
/// Panics when Tauri cannot initialize or run the application event loop.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let database_path = app
                .path()
                .app_data_dir()
                .map_err(|_| {
                    radar_core::contracts::errors::AppError::internal_generated("demo-app-data")
                })
                .map(|directory| directory.join("ai-subscribe.sqlite3"));
            #[cfg(feature = "benchmark-instrumentation")]
            let database_path = benchmark_database_path().map(Ok).unwrap_or(database_path);
            app.manage(commands::DemoState::new(database_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health_v1,
            commands::demo_bootstrap_v1,
            commands::demo_search_v1,
            commands::demo_list_v1,
            commands::demo_filter_v1,
            commands::demo_detail_v1,
            commands::setup_progress_v1,
            commands::save_setup_step_v1,
            commands::configuration_v1,
            commands::validate_configuration_v1,
            commands::save_configuration_v1,
            commands::save_source_v1,
            commands::query_sources_v1,
            commands::start_sync_v1,
            commands::task_v1,
            commands::sync_health_v1,
            commands::get_sync_result_v1,
            commands::query_intel_feed_v1
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed to run");
}
