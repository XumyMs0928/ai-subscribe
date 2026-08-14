use std::process::Command;

use radar_core::contracts::errors::AppError;
use radar_core::contracts::secrets::SecretLeaseInput;
use radar_ffi::error::run_guarded;

const CHILD_ENV: &str = "RADAR_AC2_10_CAPTURE_CHILD";
const CANARY: &str = "ac2-10-stderr-test-output-canary";

#[test]
fn contained_panic_child() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }

    let mut lease = SecretLeaseInput::new("secret:captured-output", CANARY.as_bytes().to_vec())
        .expect("runtime canary lease is valid");
    let error = run_guarded::<(), _>(move || {
        lease.with_secret(|bytes| -> Result<(), AppError> {
            assert_eq!(bytes, CANARY.as_bytes());
            panic!("{CANARY}");
        })
    })
    .expect_err("panic must be contained");
    assert_eq!(error.code(), "internal.unexpected");
}

#[test]
fn captured_stderr_and_test_output_do_not_contain_secret_canary() {
    let current_test_binary = std::env::current_exe().expect("current test binary is available");
    let output = Command::new(current_test_binary)
        .args(["--exact", "contained_panic_child", "--nocapture"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("child test process starts");

    let mut captured = output.stdout.clone();
    captured.extend_from_slice(&output.stderr);
    assert!(
        !captured
            .windows(CANARY.len())
            .any(|window| window == CANARY.as_bytes()),
        "captured stdout/stderr must contain zero plaintext canary hits"
    );
    assert!(
        output.status.success(),
        "child test failed; captured output was redacted before reporting"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("radar-ffi: contained panic at"),
        "captured stderr must prove the redacting panic hook executed"
    );
}
