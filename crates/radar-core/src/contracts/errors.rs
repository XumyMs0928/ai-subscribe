//! Stable application error contract.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

pub const CONTRACT_VERSION: u32 = 1;
static CORRELATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    ValidationEffectId,
    ValidationIdempotencyKey,
    ValidationRfc3339Utc,
    ValidationEffectStatus,
    ValidationSecretLease,
    ConflictEffectAlreadyReported,
    ConflictSecretLeaseConsumed,
    InternalUnexpected,
}

impl ErrorCode {
    pub const ALL: [Self; 8] = [
        Self::ValidationEffectId,
        Self::ValidationIdempotencyKey,
        Self::ValidationRfc3339Utc,
        Self::ValidationEffectStatus,
        Self::ValidationSecretLease,
        Self::ConflictEffectAlreadyReported,
        Self::ConflictSecretLeaseConsumed,
        Self::InternalUnexpected,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValidationEffectId => "validation.effect_id",
            Self::ValidationIdempotencyKey => "validation.idempotency_key",
            Self::ValidationRfc3339Utc => "validation.rfc3339_utc",
            Self::ValidationEffectStatus => "validation.effect_status",
            Self::ValidationSecretLease => "validation.secret_lease",
            Self::ConflictEffectAlreadyReported => "conflict.effect_already_reported",
            Self::ConflictSecretLeaseConsumed => "conflict.secret_lease_consumed",
            Self::InternalUnexpected => "internal.unexpected",
        }
    }

    const fn category(self) -> ErrorCategory {
        match self {
            Self::ValidationEffectId
            | Self::ValidationIdempotencyKey
            | Self::ValidationRfc3339Utc
            | Self::ValidationEffectStatus
            | Self::ValidationSecretLease => ErrorCategory::Validation,
            Self::ConflictEffectAlreadyReported | Self::ConflictSecretLeaseConsumed => {
                ErrorCategory::Conflict
            }
            Self::InternalUnexpected => ErrorCategory::Internal,
        }
    }

    const fn retryability(self) -> Retryability {
        match self.category() {
            ErrorCategory::Conflict | ErrorCategory::Internal => Retryability::Manual,
            _ => Retryability::Never,
        }
    }

    const fn message_key(self) -> &'static str {
        match self.category() {
            ErrorCategory::Validation => "error.validation",
            ErrorCategory::Conflict => "error.conflict",
            _ => "error.internal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCategory {
    Validation,
    NotFound,
    Conflict,
    Permission,
    Network,
    RateLimited,
    SourceFormat,
    Provider,
    Storage,
    Migration,
    Cancelled,
    Internal,
}

impl ErrorCategory {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Permission => "permission",
            Self::Network => "network",
            Self::RateLimited => "rate_limited",
            Self::SourceFormat => "source_format",
            Self::Provider => "provider",
            Self::Storage => "storage",
            Self::Migration => "migration",
            Self::Cancelled => "cancelled",
            Self::Internal => "internal",
        }
    }

    pub const ALL: [Self; 12] = [
        Self::Validation,
        Self::NotFound,
        Self::Conflict,
        Self::Permission,
        Self::Network,
        Self::RateLimited,
        Self::SourceFormat,
        Self::Provider,
        Self::Storage,
        Self::Migration,
        Self::Cancelled,
        Self::Internal,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retryability {
    Never,
    Manual,
    Automatic,
    After,
}

impl Retryability {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Manual => "manual",
            Self::Automatic => "automatic",
            Self::After => "after",
        }
    }

    pub const ALL: [Self; 4] = [Self::Never, Self::Manual, Self::Automatic, Self::After];
}

#[derive(Clone, PartialEq, Eq)]
pub struct AppError {
    contract_version: u32,
    code: ErrorCode,
    category: ErrorCategory,
    message_key: &'static str,
    retryability: Retryability,
    source_id: Option<Box<str>>,
    task_id: Option<Box<str>>,
    details_allowlisted: Box<str>,
    correlation_id: Box<str>,
}

impl fmt::Debug for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppError")
            .field("contract_version", &self.contract_version)
            .field("code", &self.code.as_str())
            .field("category", &self.category)
            .field("message_key", &self.message_key)
            .field("retryability", &self.retryability)
            .field("source_id", &self.source_id)
            .field("task_id", &self.task_id)
            .field("details_allowlisted", &self.details_allowlisted)
            .field("correlation_id", &self.correlation_id)
            .finish()
    }
}

impl AppError {
    #[must_use]
    pub fn from_code(code: ErrorCode, correlation_id: impl Into<String>) -> Self {
        let category = code.category();
        let retryability = code.retryability();
        let message_key = code.message_key();
        Self {
            contract_version: CONTRACT_VERSION,
            code,
            category,
            message_key,
            retryability,
            source_id: None,
            task_id: None,
            details_allowlisted: Box::default(),
            correlation_id: correlation_id.into().into_boxed_str(),
        }
    }

    #[must_use]
    pub fn internal_generated(boundary: &'static str) -> Self {
        let sequence = CORRELATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Self::from_code(
            ErrorCode::InternalUnexpected,
            format!("{boundary}-{sequence:016x}"),
        )
    }

    #[must_use]
    pub const fn contract_version(&self) -> u32 {
        self.contract_version
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code.as_str()
    }

    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        self.category
    }

    #[must_use]
    pub const fn message_key(&self) -> &'static str {
        self.message_key
    }

    #[must_use]
    pub const fn retryability(&self) -> Retryability {
        self.retryability
    }

    #[must_use]
    pub fn source_id(&self) -> Option<&str> {
        self.source_id.as_deref()
    }

    #[must_use]
    pub fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }

    #[must_use]
    pub fn details_allowlisted(&self) -> &str {
        &self.details_allowlisted
    }

    #[must_use]
    pub fn correlation_id(&self) -> &str {
        &self.correlation_id
    }
}
