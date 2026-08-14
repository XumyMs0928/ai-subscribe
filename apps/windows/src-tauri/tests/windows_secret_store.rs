#![cfg(target_os = "windows")]

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use ai_subscribe_windows::platform::windows::secrets::WindowsSecretStore;
use radar_core::contracts::errors::{AppError, ErrorCode};

const CHILD_ENV: &str = "AI_SUBSCRIBE_SECRET_CHILD";
const TARGET_ENV: &str = "AI_SUBSCRIBE_SECRET_TARGET";
const TEST_NAMESPACE: &str = "test:story-1-2:";
static ROUND: AtomicU64 = AtomicU64::new(1);

struct CredentialCleanup(String);

impl Drop for CredentialCleanup {
    fn drop(&mut self) {
        let _ = WindowsSecretStore::delete(&self.0);
    }
}

struct SensitiveBytes(Vec<u8>);

impl SensitiveBytes {
    fn derived_from(target: &str) -> Self {
        let mut bytes = b"runtime-credential-".to_vec();
        bytes.extend_from_slice(target.as_bytes());
        bytes.extend_from_slice(b"-bytes");
        Self(bytes)
    }

    fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SensitiveBytes {
    fn drop(&mut self) {
        self.0.fill(0);
        std::hint::black_box(&mut self.0);
    }
}

fn runtime_target() -> String {
    let process = std::process::id();
    let round = ROUND.fetch_add(1, Ordering::Relaxed);
    format!("{TEST_NAMESPACE}{process}:{round}")
}

fn collect_files(directory: &Path, output: &mut Vec<PathBuf>) {
    if !directory.exists() {
        return;
    }
    for entry in fs::read_dir(directory).expect("read observable surface") {
        let path = entry.expect("read observable entry").path();
        if path.is_dir() {
            collect_files(&path, output);
        } else {
            output.push(path);
        }
    }
}

#[test]
fn credential_manager_child_exercises_lease_paths() {
    if std::env::var(CHILD_ENV).as_deref() != Ok("1") {
        return;
    }
    let target = std::env::var(TARGET_ENV).expect("child target");
    assert!(
        target.starts_with(TEST_NAMESPACE),
        "child may only touch the Story 1.2 test namespace"
    );
    let canary = SensitiveBytes::derived_from(&target);
    let _cleanup = CredentialCleanup(target.clone());
    WindowsSecretStore::set(&target, canary.as_slice()).expect("write test credential");

    let mut success = WindowsSecretStore::lease(&target).expect("success lease");
    success
        .with_secret(|bytes| {
            assert_eq!(bytes, canary.as_slice());
            Ok(())
        })
        .expect("consume success lease");
    assert_eq!(
        success
            .with_secret(|_| Ok(()))
            .expect_err("repeat is rejected")
            .code(),
        "conflict.secret_lease_consumed"
    );

    let mut operation_error = WindowsSecretStore::lease(&target).expect("error lease");
    assert_eq!(
        operation_error
            .with_secret(|_| {
                Err(AppError::from_code(
                    ErrorCode::ValidationSecretLease,
                    "windows-secret-operation",
                ))
            })
            .expect_err("operation error is preserved")
            .code(),
        "validation.secret_lease"
    );

    let mut panic_lease = WindowsSecretStore::lease(&target).expect("panic lease");
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = panic_lease.with_secret(|_| -> Result<(), AppError> {
            panic!("controlled panic without payload echo");
        });
    }));
    assert!(panic.is_err());

    WindowsSecretStore::delete(&target).expect("delete test credential");
}

#[test]
fn credential_manager_outputs_and_namespace_contain_no_runtime_canary() {
    let target = runtime_target();
    assert!(
        !WindowsSecretStore::test_target_exists(&target).expect("pre-test target probe"),
        "the unique per-round test target must start empty"
    );
    let canary = SensitiveBytes::derived_from(&target);
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("credential_manager_child_exercises_lease_paths")
        .arg("--exact")
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env(TARGET_ENV, &target)
        .output()
        .expect("run credential child");

    let mut observed = output.stdout;
    observed.extend_from_slice(&output.stderr);
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut observable_files = Vec::new();
    collect_files(&manifest.join("../dist"), &mut observable_files);
    collect_files(
        &manifest.join("../../../contracts/fixtures/golden"),
        &mut observable_files,
    );
    collect_files(
        &manifest.join("../../../contracts/snapshots"),
        &mut observable_files,
    );
    for path in observable_files {
        observed.extend_from_slice(&fs::read(path).expect("read observable file"));
    }
    let redacted_error = serde_json::json!({
        "code": "validation.secret_lease",
        "details_allowlisted": "",
        "source_id": null,
        "task_id": null
    });
    observed.extend_from_slice(redacted_error.to_string().as_bytes());

    assert!(
        !observed
            .windows(canary.as_slice().len())
            .any(|window| window == canary.as_slice()),
        "runtime canary must not reach captured output, IPC-shaped errors, fixtures, snapshots, or bundle"
    );
    assert!(
        output.status.success(),
        "credential child failed: {}",
        String::from_utf8_lossy(&observed)
    );
    assert!(
        !WindowsSecretStore::test_target_exists(&target).expect("post-test target probe"),
        "the unique per-round test namespace must contain no residual credential"
    );
}
