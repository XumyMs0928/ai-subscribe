//! Approved FFI-facing use cases.

use radar_core::application::health_check;
use radar_core::contracts::errors::AppError;

use crate::error::run_guarded;
use crate::mapping::HealthStatusWire;

/// Executes the versioned health use case through the panic-contained adapter.
///
/// # Errors
///
/// Returns the stable internal error contract if the adapter panics.
pub fn health_v1() -> Result<HealthStatusWire, AppError> {
    run_guarded(|| Ok(health_check().into()))
}
