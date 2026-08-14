use std::panic::{AssertUnwindSafe, catch_unwind};

use radar_core::contracts::errors::{AppError, ErrorCategory, ErrorCode};
use radar_core::contracts::secrets::SecretLeaseInput;

const CANARY: &[u8] = b"generated-secret-negative-canary";

fn valid_secret() -> Vec<u8> {
    CANARY.to_vec()
}

fn assert_validation_error(result: Result<SecretLeaseInput, AppError>) {
    let Err(error) = result else {
        panic!("invalid lease input must be rejected");
    };
    assert_eq!(error.code(), "validation.secret_lease");
    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.message_key(), "error.validation");
    assert!(error.details_allowlisted().is_empty());
    assert!(!format!("{error:?}").contains("generated-secret-negative-canary"));
}

/// [P0] Empty references, invalid characters, over-limit references, and empty bytes are rejected without observable plaintext.
#[test]
fn secret_lease_rejects_every_constructor_negative_path() {
    let invalid_cases = [
        ("empty reference", String::new(), valid_secret()),
        (
            "invalid reference character",
            "secret/unsafe".to_owned(),
            valid_secret(),
        ),
        (
            "non-ascii reference",
            "secret:\u{79d8}\u{5bc6}".to_owned(),
            valid_secret(),
        ),
        ("reference over 128 bytes", "s".repeat(129), valid_secret()),
        ("empty secret", "secret:empty".to_owned(), Vec::new()),
    ];

    for (case, secret_ref, secret) in invalid_cases {
        assert_validation_error(SecretLeaseInput::new(secret_ref, secret));
        assert!(!case.is_empty(), "case label documents the failed boundary");
    }
}

/// [P0] A callback error consumes the lease and never reflects secret bytes into `AppError` observables.
#[test]
fn secret_lease_operation_error_is_single_use_and_redacted() {
    let mut lease =
        SecretLeaseInput::new("secret:operation-error", valid_secret()).expect("valid lease");

    let error = lease
        .with_secret(|bytes| {
            assert_eq!(bytes, CANARY);
            Err(AppError::from_code(
                ErrorCode::ValidationSecretLease,
                "test-operation-failed",
            ))
        })
        .expect_err("operation error must be returned");

    assert_eq!(error.code(), "validation.secret_lease");
    assert_eq!(error.correlation_id(), "test-operation-failed");
    let observable = format!(
        "{error:?}|{}|{}|{}",
        error.message_key(),
        error.details_allowlisted(),
        error.correlation_id()
    );
    assert!(!observable.contains("generated-secret-negative-canary"));

    let consumed = lease
        .with_secret(|_| Ok(()))
        .expect_err("a failed operation still consumes the lease");
    assert_eq!(consumed.code(), "conflict.secret_lease_consumed");
}

/// [P0] Unwinding through a callback drops the taken secret and leaves the lease permanently consumed.
#[test]
fn secret_lease_callback_panic_cannot_make_the_lease_reusable() {
    let mut lease = SecretLeaseInput::new("secret:panic", valid_secret()).expect("valid lease");

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = lease.with_secret(|bytes| -> Result<(), AppError> {
            assert_eq!(bytes, CANARY);
            panic!("generated-secret-negative-canary");
        });
    }));
    assert!(
        panic.is_err(),
        "the callback panic remains catchable by its caller"
    );

    let consumed = lease
        .with_secret(|_| Ok(()))
        .expect_err("unwinding must not restore the lease");
    assert_eq!(consumed.code(), "conflict.secret_lease_consumed");
    assert!(!format!("{consumed:?}").contains("generated-secret-negative-canary"));
}
