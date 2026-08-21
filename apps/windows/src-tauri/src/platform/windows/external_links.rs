//! Narrow Windows system-browser adapter for core-validated original links.

use radar_core::application::intel_detail::ValidatedOriginalLink;
use radar_core::contracts::errors::AppError;
use tauri_plugin_opener::OpenerExt;

/// Opens one already validated URL in the system default browser.
///
/// # Errors
/// Returns a stable redacted error without including the URL or platform diagnostic.
pub(crate) fn open_validated_link(
    app: &tauri::AppHandle,
    link: &ValidatedOriginalLink,
) -> Result<(), AppError> {
    app.opener()
        .open_url(link.url(), None::<&str>)
        .map_err(|_| AppError::internal_generated("external-link-open"))
}
