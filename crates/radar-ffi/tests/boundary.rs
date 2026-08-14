use radar_ffi::api::health_v1;
use radar_ffi::error::{map_unknown, run_guarded};
use radar_ffi::mapping::AppErrorWire;

#[test]
fn approved_health_adapter_maps_explicit_null_json() {
    let wire = health_v1().expect("health adapter succeeds");
    assert_eq!(
        wire.to_json(),
        "{\"contract_version\":1,\"status\":\"ok\",\"checked_at\":null}"
    );
}

#[test]
fn panic_is_mapped_without_leaking_its_message() {
    let error = run_guarded::<(), _>(|| -> Result<(), _> { panic!("private internal detail") })
        .expect_err("panic must become AppError");

    assert_eq!(error.code(), "internal.unexpected");
    assert_eq!(error.category().as_str(), "internal");
    assert!(
        !error
            .details_allowlisted()
            .contains("private internal detail")
    );
}

#[test]
fn unknown_error_is_mapped_without_leaking_debug_text() {
    let error = map_unknown::<()>("private provider response")
        .expect_err("unknown error must become AppError");
    let rendered = format!("{error:?}");

    assert_eq!(error.code(), "internal.unexpected");
    assert!(!rendered.contains("private provider response"));

    let second = map_unknown::<()>("another private response")
        .expect_err("second unknown error must become AppError");
    assert_ne!(error.correlation_id(), second.correlation_id());

    let wire = AppErrorWire::from(&error);
    assert_eq!(wire.code, "internal.unexpected");
    assert_eq!(wire.category, "internal");
    assert!(wire.source_id.is_none());
    assert!(wire.task_id.is_none());
}
