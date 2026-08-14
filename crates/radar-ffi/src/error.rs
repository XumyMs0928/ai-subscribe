//! Panic and internal-error mapping.

use std::panic::{AssertUnwindSafe, UnwindSafe, catch_unwind};
use std::sync::Once;

use radar_core::contracts::errors::AppError;

static REDACTING_HOOK: Once = Once::new();

/// Installs the process-wide FFI panic hook once. The hook deliberately omits
/// panic payloads so secrets and private provider text cannot reach stderr.
pub fn install_redacting_panic_hook() {
    REDACTING_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            eprintln!("radar-ffi: contained panic at {:?}", info.location());
        }));
    });
}

/// Runs a pure Rust adapter call while containing panics at the future FFI boundary.
///
/// # Errors
///
/// Preserves an existing stable `AppError`, or returns `internal.unexpected` when
/// the operation panics.
pub fn run_guarded<T, F>(operation: F) -> Result<T, AppError>
where
    F: FnOnce() -> Result<T, AppError> + UnwindSafe,
{
    install_redacting_panic_hook();
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => Err(AppError::internal_generated("ffi-panic-contained")),
    }
}

/// Maps an unknown private implementation error without exposing its debug text.
///
/// # Errors
///
/// Always returns the stable `internal.unexpected` error contract.
pub fn map_unknown<T>(_private_error: impl std::fmt::Debug) -> Result<T, AppError> {
    Err(AppError::internal_generated("ffi-unknown-contained"))
}
