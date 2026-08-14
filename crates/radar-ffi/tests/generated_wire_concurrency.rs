use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};
use std::thread;

use radar_core::contracts::errors::AppError;
use radar_ffi::error::{map_unknown, run_guarded};
use radar_ffi::mapping::{AppErrorWire, HealthStatusWire};

/// [P0] Wire JSON escapes quotes, backslashes, controls, and newlines while preserving Unicode and explicit optional values.
#[test]
fn health_wire_json_escapes_all_special_characters() {
    let wire = HealthStatusWire {
        contract_version: 1,
        status: "\"quoted\"\\line\n\r\t\u{0008}\u{2603}".to_owned(),
        checked_at: Some("2026-08-13T00:00:00Z\n\"zone\"".to_owned()),
    };

    assert_eq!(
        wire.to_json(),
        r#"{"contract_version":1,"status":"\"quoted\"\\line\n\r\t\u0008☃","checked_at":"2026-08-13T00:00:00Z\n\"zone\""}"#
    );

    let without_time = HealthStatusWire {
        checked_at: None,
        ..wire
    };
    assert!(without_time.to_json().ends_with("\"checked_at\":null}"));
}

/// [P0] Concurrent panic and unknown-error paths allocate one unique, stable correlation ID per failure.
#[test]
fn concurrent_ffi_failures_never_reuse_correlation_ids() {
    const WORKERS: usize = 64;
    let barrier = Arc::new(Barrier::new(WORKERS));
    let handles: Vec<_> = (0..WORKERS)
        .map(|worker| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let error = if worker % 2 == 0 {
                    map_unknown::<()>(format!("private-unknown-{worker}"))
                        .expect_err("unknown error must map")
                } else {
                    run_guarded::<(), _>(|| -> Result<(), AppError> {
                        panic!("private-panic-{worker}");
                    })
                    .expect_err("panic must map")
                };
                let wire = AppErrorWire::from(&error);
                assert_eq!(wire.code, "internal.unexpected");
                assert_eq!(wire.category, "internal");
                assert_eq!(wire.message_key, "error.internal");
                assert_eq!(wire.retryability, "manual");
                assert!(wire.details_allowlisted.is_empty());
                assert_eq!(wire.correlation_id, error.correlation_id());
                wire.correlation_id
            })
        })
        .collect();

    let correlation_ids: BTreeSet<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("worker must not escape the FFI guard"))
        .collect();

    assert_eq!(correlation_ids.len(), WORKERS);
    assert!(correlation_ids.iter().all(|correlation_id| {
        correlation_id.starts_with("ffi-unknown-contained-")
            || correlation_id.starts_with("ffi-panic-contained-")
    }));
}
