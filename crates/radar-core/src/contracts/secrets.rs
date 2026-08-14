//! Short-lived secret lease contract.

use crate::contracts::errors::{AppError, ErrorCode};
use zeroize::Zeroize;

#[must_use]
pub const fn contract_version() -> u32 {
    1
}

pub struct SecretLeaseInput {
    contract_version: u32,
    secret_ref: String,
    secret: Option<SecretBytes>,
}

#[cfg(test)]
type ZeroizationObserver = Box<dyn FnOnce(&[u8])>;

#[cfg(test)]
struct SecretBytes {
    bytes: Vec<u8>,
    zeroization_observer: Option<ZeroizationObserver>,
}

#[cfg(not(test))]
struct SecretBytes(Vec<u8>);

impl Drop for SecretBytes {
    fn drop(&mut self) {
        // `Zeroize` uses volatile writes plus compiler fences, so release
        // optimization cannot remove the erasure. Zeroize the visible slice
        // first so the test-only observer can audit it, then the `Vec` itself
        // to cover the allocation's entire capacity and reset its length.
        self.bytes_mut().zeroize();
        #[cfg(test)]
        if let Some(observer) = self.zeroization_observer.take() {
            observer(&self.bytes);
        }
        self.vec_mut().zeroize();
    }
}

impl SecretBytes {
    #[cfg(not(test))]
    fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    #[cfg(test)]
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            zeroization_observer: None,
        }
    }

    #[cfg(test)]
    fn new_audited(bytes: Vec<u8>, observer: ZeroizationObserver) -> Self {
        Self {
            bytes,
            zeroization_observer: Some(observer),
        }
    }

    #[cfg(not(test))]
    fn bytes(&self) -> &[u8] {
        &self.0
    }

    #[cfg(test)]
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[cfg(not(test))]
    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }

    #[cfg(test)]
    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    #[cfg(not(test))]
    fn vec_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    #[cfg(test)]
    fn vec_mut(&mut self) -> &mut Vec<u8> {
        &mut self.bytes
    }
}

impl SecretLeaseInput {
    /// Creates a single-use in-memory secret lease.
    ///
    /// # Errors
    ///
    /// Returns validation when the reference or secret bytes are empty, or the
    /// reference exceeds the v1 contract limit.
    pub fn new(secret_ref: impl Into<String>, secret: Vec<u8>) -> Result<Self, AppError> {
        let secret_ref = secret_ref.into();
        let secret = SecretBytes::new(secret);
        if secret_ref.is_empty()
            || secret_ref.len() > 128
            || !secret_ref.bytes().all(is_safe_reference_byte)
            || secret.bytes().is_empty()
        {
            return Err(AppError::from_code(
                ErrorCode::ValidationSecretLease,
                "contract-secret-validation",
            ));
        }
        Ok(Self {
            contract_version: contract_version(),
            secret_ref,
            secret: Some(secret),
        })
    }

    #[must_use]
    pub const fn contract_version(&self) -> u32 {
        self.contract_version
    }

    #[must_use]
    pub fn secret_ref(&self) -> &str {
        &self.secret_ref
    }

    /// Exposes the secret to exactly one synchronous operation.
    ///
    /// # Errors
    ///
    /// Returns conflict when the lease has already been consumed.
    pub fn with_secret(
        &mut self,
        operation: impl FnOnce(&[u8]) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        let secret = self.secret.take().ok_or_else(|| {
            AppError::from_code(
                ErrorCode::ConflictSecretLeaseConsumed,
                "contract-secret-consumed",
            )
        })?;
        operation(secret.bytes())
    }

    #[cfg(test)]
    fn new_audited(
        secret_ref: impl Into<String>,
        secret: Vec<u8>,
        observer: ZeroizationObserver,
    ) -> Result<Self, AppError> {
        let secret_ref = secret_ref.into();
        let secret = SecretBytes::new_audited(secret, observer);
        if secret_ref.is_empty()
            || secret_ref.len() > 128
            || !secret_ref.bytes().all(is_safe_reference_byte)
            || secret.bytes().is_empty()
        {
            return Err(AppError::from_code(
                ErrorCode::ValidationSecretLease,
                "contract-secret-validation",
            ));
        }
        Ok(Self {
            contract_version: contract_version(),
            secret_ref,
            secret: Some(secret),
        })
    }
}

const fn is_safe_reference_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};

    use super::SecretLeaseInput;
    use crate::contracts::errors::{AppError, ErrorCode};

    const NOT_OBSERVED: u8 = 0;
    const OBSERVED_ZEROED: u8 = 1;
    const OBSERVED_PLAINTEXT: u8 = 2;
    const CANARY: &[u8] = b"ac2-10-memory-zeroization-canary";

    fn audited_lease(secret_ref: &str) -> (Result<SecretLeaseInput, AppError>, Arc<AtomicU8>) {
        let observed = Arc::new(AtomicU8::new(NOT_OBSERVED));
        let observer_state = Arc::clone(&observed);
        let lease = SecretLeaseInput::new_audited(
            secret_ref,
            CANARY.to_vec(),
            Box::new(move |bytes| {
                let state = if bytes.iter().all(|byte| *byte == 0) {
                    OBSERVED_ZEROED
                } else {
                    OBSERVED_PLAINTEXT
                };
                observer_state.store(state, Ordering::SeqCst);
            }),
        );
        (lease, observed)
    }

    fn assert_zeroed(observed: &AtomicU8, path: &str) {
        assert_eq!(
            observed.load(Ordering::SeqCst),
            OBSERVED_ZEROED,
            "{path} must expose only zeroed bytes to the test-only drop observer"
        );
    }

    #[test]
    fn secret_storage_is_zeroed_after_success_error_and_panic() {
        let (lease, success_observed) = audited_lease("secret:audit-success");
        let mut lease = lease.expect("valid success lease");
        lease
            .with_secret(|bytes| {
                assert_eq!(bytes, CANARY);
                Ok(())
            })
            .expect("operation succeeds");
        assert_zeroed(&success_observed, "success");

        let (lease, error_observed) = audited_lease("secret:audit-error");
        let mut lease = lease.expect("valid error lease");
        lease
            .with_secret(|_| {
                Err(AppError::from_code(
                    ErrorCode::ValidationSecretLease,
                    "audit-operation-error",
                ))
            })
            .expect_err("operation fails");
        assert_zeroed(&error_observed, "operation error");

        let (lease, panic_observed) = audited_lease("secret:audit-panic");
        let mut lease = lease.expect("valid panic lease");
        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _ = lease.with_secret(|_| -> Result<(), AppError> {
                panic!("panic payload is intentionally not inspected by the observer");
            });
        }));
        assert!(panic.is_err(), "test must execute the unwinding path");
        assert_zeroed(&panic_observed, "panic");
    }

    #[test]
    fn secret_storage_is_zeroed_when_constructor_validation_fails() {
        let (result, observed) = audited_lease("invalid/reference");
        assert!(result.is_err(), "invalid reference must be rejected");
        assert_zeroed(&observed, "constructor validation error");
    }
}
