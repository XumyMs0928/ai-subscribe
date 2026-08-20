//! Stable data-transfer objects.

pub mod configuration_validation;
pub mod intel_feed;
pub mod source;
pub mod sync;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthStatus {
    pub contract_version: u32,
    pub status: String,
    pub checked_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpaqueId(Box<str>);

impl OpaqueId {
    pub(crate) fn parse(
        value: String,
        error_code: super::errors::ErrorCode,
    ) -> Result<Self, super::errors::AppError> {
        if value.is_empty() || value.len() > 128 || !value.bytes().all(is_safe_id_byte) {
            return Err(super::errors::AppError::from_code(
                error_code,
                "contract-id-validation",
            ));
        }
        Ok(Self(value.into_boxed_str()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const fn is_safe_id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
}
