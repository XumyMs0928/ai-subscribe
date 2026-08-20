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
    ValidationSetupInput,
    ValidationConfiguration,
    ValidationStaleReceipt,
    ValidationSource,
    ConflictEffectAlreadyReported,
    ConflictSecretLeaseConsumed,
    ConflictSetupRevision,
    ConflictConfigurationRevision,
    ConflictSourceRevision,
    NetworkSource,
    RateLimitedSource,
    SourceFormatRssAtom,
    StorageSetup,
    StorageConfiguration,
    StorageSource,
    MigrationSetup,
    MigrationSource,
    InternalUnexpected,
}

impl ErrorCode {
    pub const ALL: [Self; 23] = [
        Self::ValidationEffectId,
        Self::ValidationIdempotencyKey,
        Self::ValidationRfc3339Utc,
        Self::ValidationEffectStatus,
        Self::ValidationSecretLease,
        Self::ValidationSetupInput,
        Self::ValidationConfiguration,
        Self::ValidationStaleReceipt,
        Self::ValidationSource,
        Self::ConflictEffectAlreadyReported,
        Self::ConflictSecretLeaseConsumed,
        Self::ConflictSetupRevision,
        Self::ConflictConfigurationRevision,
        Self::ConflictSourceRevision,
        Self::NetworkSource,
        Self::RateLimitedSource,
        Self::SourceFormatRssAtom,
        Self::StorageSetup,
        Self::StorageConfiguration,
        Self::StorageSource,
        Self::MigrationSetup,
        Self::MigrationSource,
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
            Self::ValidationSetupInput => "validation.setup_input",
            Self::ValidationConfiguration => "validation.configuration",
            Self::ValidationStaleReceipt => "validation.stale_validation_receipt",
            Self::ValidationSource => "validation.source",
            Self::ConflictEffectAlreadyReported => "conflict.effect_already_reported",
            Self::ConflictSecretLeaseConsumed => "conflict.secret_lease_consumed",
            Self::ConflictSetupRevision => "conflict.setup_revision",
            Self::ConflictConfigurationRevision => "conflict.configuration_revision",
            Self::ConflictSourceRevision => "conflict.source_revision",
            Self::NetworkSource => "network.source",
            Self::RateLimitedSource => "rate_limited.source",
            Self::SourceFormatRssAtom => "source_format.rss_atom",
            Self::StorageSetup => "storage.setup",
            Self::StorageConfiguration => "storage.configuration",
            Self::StorageSource => "storage.source",
            Self::MigrationSetup => "migration.setup",
            Self::MigrationSource => "migration.source",
            Self::InternalUnexpected => "internal.unexpected",
        }
    }

    const fn category(self) -> ErrorCategory {
        match self {
            Self::ValidationEffectId
            | Self::ValidationIdempotencyKey
            | Self::ValidationRfc3339Utc
            | Self::ValidationEffectStatus
            | Self::ValidationSecretLease
            | Self::ValidationSetupInput
            | Self::ValidationConfiguration
            | Self::ValidationStaleReceipt
            | Self::ValidationSource => ErrorCategory::Validation,
            Self::ConflictEffectAlreadyReported
            | Self::ConflictSecretLeaseConsumed
            | Self::ConflictSetupRevision
            | Self::ConflictConfigurationRevision
            | Self::ConflictSourceRevision => ErrorCategory::Conflict,
            Self::NetworkSource => ErrorCategory::Network,
            Self::RateLimitedSource => ErrorCategory::RateLimited,
            Self::SourceFormatRssAtom => ErrorCategory::SourceFormat,
            Self::StorageSetup | Self::StorageConfiguration | Self::StorageSource => {
                ErrorCategory::Storage
            }
            Self::MigrationSetup | Self::MigrationSource => ErrorCategory::Migration,
            Self::InternalUnexpected => ErrorCategory::Internal,
        }
    }

    const fn retryability(self) -> Retryability {
        match self.category() {
            ErrorCategory::Conflict | ErrorCategory::Internal => Retryability::Manual,
            ErrorCategory::Network => Retryability::Automatic,
            ErrorCategory::RateLimited => Retryability::After,
            _ => Retryability::Never,
        }
    }

    const fn message_key(self) -> &'static str {
        match self.category() {
            ErrorCategory::Validation => "error.validation",
            ErrorCategory::Conflict => "error.conflict",
            ErrorCategory::Network => "error.network.source",
            ErrorCategory::RateLimited => "error.rate_limited.source",
            ErrorCategory::SourceFormat => "error.source_format.rss_atom",
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

    /// Attaches an already validated opaque source identity without exposing endpoint data.
    #[must_use]
    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        let value = source_id.into();
        if !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
        {
            self.source_id = Some(value.into_boxed_str());
        }
        self
    }

    /// Attaches an already validated opaque task identity without exposing request data.
    #[must_use]
    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        let value = task_id.into();
        if !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
        {
            self.task_id = Some(value.into_boxed_str());
        }
        self
    }

    /// Attaches a numeric, non-sensitive server retry delay for source scheduling.
    #[must_use]
    pub fn with_retry_after_ms(mut self, retry_after_ms: u64) -> Self {
        self.details_allowlisted = format!("retry_after_ms={retry_after_ms}").into_boxed_str();
        self
    }

    #[must_use]
    pub fn retry_after_ms(&self) -> Option<u64> {
        self.details_allowlisted
            .strip_prefix("retry_after_ms=")?
            .parse()
            .ok()
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
