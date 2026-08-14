//! Windows Credential Manager adapter. No secret is exposed to the `WebView`.

use std::collections::HashMap;

use keyring_core::Entry;
use keyring_core::api::CredentialStoreApi;
use radar_core::contracts::errors::AppError;
use radar_core::contracts::secrets::SecretLeaseInput;

const SERVICE: &str = "com.aisubscribe.desktop";

pub struct WindowsSecretStore;

impl WindowsSecretStore {
    /// Stores secret bytes in the current user's Windows Credential Manager.
    ///
    /// # Errors
    /// Returns a redacted stable error if the platform store rejects the operation.
    pub fn set(secret_ref: &str, secret: &[u8]) -> Result<(), AppError> {
        let validation = SecretLeaseInput::new(secret_ref, secret.to_vec())?;
        drop(validation);
        entry(secret_ref)?
            .set_secret(secret)
            .map_err(map_store_error)
    }

    /// Retrieves the secret directly into a single-use core lease.
    ///
    /// # Errors
    /// Returns only redacted stable errors for store or lease validation failures.
    pub fn lease(secret_ref: &str) -> Result<SecretLeaseInput, AppError> {
        let secret = entry(secret_ref)?.get_secret().map_err(map_store_error)?;
        SecretLeaseInput::new(secret_ref, secret)
    }

    /// Deletes the named credential from the current user's store.
    ///
    /// # Errors
    /// Returns a redacted stable error if deletion fails.
    pub fn delete(secret_ref: &str) -> Result<(), AppError> {
        entry(secret_ref)?
            .delete_credential()
            .map_err(map_store_error)
    }

    /// Reports whether an exact Story 1.2 test target exists without reading its secret.
    ///
    /// # Errors
    /// Returns a redacted stable error if the namespace is invalid or search fails.
    pub fn test_target_exists(target: &str) -> Result<bool, AppError> {
        if !target.starts_with("test:story-1-2:")
            || !target
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-'))
        {
            return Err(AppError::internal_generated("windows-secret-namespace"));
        }
        match entry(target)?.get_credential() {
            Ok(_) => Ok(true),
            Err(keyring_core::Error::NoEntry) => Ok(false),
            Err(error) => Err(map_store_error(error)),
        }
    }
}

fn entry(secret_ref: &str) -> Result<Entry, AppError> {
    let store = windows_native_keyring_store::Store::new().map_err(map_store_error)?;
    // Story 1.2 only needs a short-lived, testable credential lease. Session
    // persistence keeps the credential inside Windows Credential Manager while
    // deliberately avoiding machine-wide or durable user-vault state. Story 5.1
    // owns the later persistent credential-management policy.
    let modifiers = HashMap::from([("persistence", "Session")]);
    store
        .build(SERVICE, secret_ref, Some(&modifiers))
        .map_err(map_store_error)
}

fn map_store_error(_error: keyring_core::Error) -> AppError {
    AppError::internal_generated("windows-secret-store")
}
