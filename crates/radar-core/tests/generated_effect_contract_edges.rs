use radar_core::contracts::effects::{EffectLedger, EffectStatus, PlatformEffect, ReportResult};
use radar_core::contracts::errors::{AppError, ErrorCategory, ErrorCode, Retryability};

const VALID_TIME: &str = "2026-08-13T00:00:00Z";

fn contract_probe(effect_id: &str, idempotency_key: &str) -> PlatformEffect {
    PlatformEffect::new_contract_probe(effect_id, idempotency_key, VALID_TIME)
        .expect("test factory must produce a valid contract probe")
}

/// [P0] Every v1 terminal status applies once and returns the stable idempotent result on repetition.
#[test]
fn effect_ledger_covers_the_complete_terminal_status_matrix() {
    let terminals = [
        ("delivered", EffectStatus::Delivered),
        ("denied", EffectStatus::Denied),
        ("expired", EffectStatus::Expired),
        ("failed", EffectStatus::Failed),
    ];

    for (label, terminal) in terminals {
        let effect_id = format!("effect:{label}");
        let key = format!("probe:{label}");
        let mut ledger = EffectLedger::default();
        ledger
            .register(contract_probe(&effect_id, &key))
            .expect("effect registers");

        assert_eq!(
            ledger.report(&effect_id, &key, terminal),
            Ok(ReportResult::Applied),
            "{label} must apply once"
        );
        assert_eq!(ledger.status(&key), Some(terminal));
        assert_eq!(
            ledger.report(&effect_id, &key, terminal),
            Ok(ReportResult::AlreadyApplied),
            "{label} repetition must be idempotent"
        );
    }
}

/// [P0] Re-registering the same identity is a no-op and cannot reset an already-terminal effect.
#[test]
fn effect_ledger_same_identity_registration_preserves_terminal_state() {
    let mut ledger = EffectLedger::default();
    ledger
        .register(contract_probe("effect:same", "probe:same"))
        .expect("first registration succeeds");
    assert_eq!(
        ledger.report("effect:same", "probe:same", EffectStatus::Denied),
        Ok(ReportResult::Applied)
    );

    ledger
        .register(contract_probe("effect:same", "probe:same"))
        .expect("same identity registration is idempotent");

    assert_eq!(ledger.status("probe:same"), Some(EffectStatus::Denied));
    assert_eq!(
        ledger.report("effect:same", "probe:same", EffectStatus::Denied),
        Ok(ReportResult::AlreadyApplied)
    );
}

/// [P0] Reporting an unknown idempotency key returns its stable validation mapping without mutating registered effects.
#[test]
fn effect_ledger_unknown_key_is_validation_and_state_safe() {
    let mut ledger = EffectLedger::default();
    ledger
        .register(contract_probe("effect:known", "probe:known"))
        .expect("known effect registers");

    let error = ledger
        .report("effect:unknown", "probe:missing", EffectStatus::Expired)
        .expect_err("unknown key must fail");

    assert_eq!(error.code(), "validation.idempotency_key");
    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(ledger.status("probe:known"), Some(EffectStatus::Pending));
    assert_eq!(ledger.status("probe:missing"), None);
}

/// [P0] An effect-ID mismatch on a known key conflicts and leaves the authoritative effect pending.
#[test]
fn effect_ledger_identity_mismatch_is_conflict_and_state_safe() {
    let mut ledger = EffectLedger::default();
    ledger
        .register(contract_probe("effect:authority", "probe:authority"))
        .expect("authoritative effect registers");

    let error = ledger
        .report("effect:impostor", "probe:authority", EffectStatus::Failed)
        .expect_err("identity mismatch must fail");

    assert_eq!(error.code(), "conflict.effect_already_reported");
    assert_eq!(error.category(), ErrorCategory::Conflict);
    assert_eq!(
        ledger.status("probe:authority"),
        Some(EffectStatus::Pending)
    );
}

/// [P1] Opaque effect and idempotency IDs accept exact limits and reject empty, oversized, unsafe, and non-ASCII values with field-specific codes.
#[test]
fn platform_effect_id_boundaries_have_field_specific_errors() {
    let exact_limit = "a".repeat(128);
    let safe_punctuation = "Az09-_:.";
    for value in [&exact_limit, safe_punctuation] {
        PlatformEffect::new_contract_probe(value, "key:valid", VALID_TIME)
            .expect("valid effect ID boundary");
        PlatformEffect::new_contract_probe("effect:valid", value, VALID_TIME)
            .expect("valid idempotency-key boundary");
    }

    for value in ["", "contains space", "contains/slash", "\u{79d8}\u{5bc6}"] {
        let effect_error = PlatformEffect::new_contract_probe(value, "key:valid", VALID_TIME)
            .expect_err("unsafe effect ID must fail");
        assert_eq!(effect_error.code(), "validation.effect_id");

        let key_error = PlatformEffect::new_contract_probe("effect:valid", value, VALID_TIME)
            .expect_err("unsafe idempotency key must fail");
        assert_eq!(key_error.code(), "validation.idempotency_key");
    }

    let oversized = "z".repeat(129);
    assert_eq!(
        PlatformEffect::new_contract_probe(&oversized, "key:valid", VALID_TIME)
            .expect_err("oversized effect ID must fail")
            .code(),
        "validation.effect_id"
    );
    assert_eq!(
        PlatformEffect::new_contract_probe("effect:valid", oversized, VALID_TIME)
            .expect_err("oversized idempotency key must fail")
            .code(),
        "validation.idempotency_key"
    );
}

/// [P1] RFC3339 UTC validation accepts every supported UTC spelling and emits one canonical uppercase-Z representation.
#[test]
fn platform_effect_rfc3339_utc_boundaries_are_complete_and_canonical() {
    let max_fraction = format!("2000-02-29T23:59:59.{}Z", "7".repeat(43));
    let valid = [
        ("0000-01-01T00:00:00Z", "0000-01-01T00:00:00Z"),
        ("2000-02-29T23:59:59Z", "2000-02-29T23:59:59Z"),
        ("2024-02-29t00:00:00.0z", "2024-02-29T00:00:00.0Z"),
        ("2026-04-30T23:59:59+00:00", "2026-04-30T23:59:59Z"),
        ("2016-12-31T23:59:60Z", "2016-12-31T23:59:60Z"),
        ("2015-06-30t23:59:60z", "2015-06-30T23:59:60Z"),
        (max_fraction.as_str(), max_fraction.as_str()),
    ];
    for (timestamp, canonical) in valid {
        let effect = PlatformEffect::new_contract_probe("effect:time", "key:time", timestamp)
            .expect("valid RFC3339 UTC timestamp boundary");
        assert_eq!(effect.expires_at(), canonical, "{timestamp}");
    }

    let over_limit_fraction = format!("2000-02-29T23:59:59.{}Z", "7".repeat(44));
    for timestamp in [
        "1900-02-29T00:00:00Z",
        "2026-02-29T00:00:00Z",
        "2026-04-31T00:00:00Z",
        "2026-01-01T24:00:00Z",
        "2026-01-01T23:60:00Z",
        "2026-01-01T23:59:60Z",
        "2026-06-30T22:59:60Z",
        "2024-06-30T23:59:60Z",
        "2026-06-29T23:59:60Z",
        "2026-01-01T00:00:00+01:00",
        "2026-01-01T00:00:00-00:00",
        "2026-01-01T00:00:00.Z",
        over_limit_fraction.as_str(),
    ] {
        let error = PlatformEffect::new_contract_probe("effect:time", "key:time", timestamp)
            .expect_err("invalid UTC boundary must fail");
        assert_eq!(error.code(), "validation.rfc3339_utc", "{timestamp}");
    }
}

/// [P1] Every `ErrorCode` maps to its exact wire code, category, retryability, and message key.
#[test]
fn every_error_code_has_the_complete_stable_mapping() {
    let cases = [
        (
            ErrorCode::ValidationEffectId,
            "validation.effect_id",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ValidationIdempotencyKey,
            "validation.idempotency_key",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ValidationRfc3339Utc,
            "validation.rfc3339_utc",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ValidationEffectStatus,
            "validation.effect_status",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ValidationSecretLease,
            "validation.secret_lease",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ValidationSetupInput,
            "validation.setup_input",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ConflictEffectAlreadyReported,
            "conflict.effect_already_reported",
            ErrorCategory::Conflict,
            Retryability::Manual,
            "error.conflict",
        ),
        (
            ErrorCode::ConflictSecretLeaseConsumed,
            "conflict.secret_lease_consumed",
            ErrorCategory::Conflict,
            Retryability::Manual,
            "error.conflict",
        ),
        (
            ErrorCode::ConflictSetupRevision,
            "conflict.setup_revision",
            ErrorCategory::Conflict,
            Retryability::Manual,
            "error.conflict",
        ),
        (
            ErrorCode::StorageSetup,
            "storage.setup",
            ErrorCategory::Storage,
            Retryability::Never,
            "error.internal",
        ),
        (
            ErrorCode::MigrationSetup,
            "migration.setup",
            ErrorCategory::Migration,
            Retryability::Never,
            "error.internal",
        ),
        (
            ErrorCode::InternalUnexpected,
            "internal.unexpected",
            ErrorCategory::Internal,
            Retryability::Manual,
            "error.internal",
        ),
    ];

    assert_eq!(cases.len() + 11, ErrorCode::ALL.len());
    for (code, expected_code, category, retryability, message_key) in cases {
        assert_error_mapping(code, expected_code, category, retryability, message_key);
    }
}

#[test]
fn source_error_codes_have_complete_stable_mappings() {
    for (code, expected_code, category, retryability, message_key) in [
        (
            ErrorCode::ValidationSource,
            "validation.source",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ConflictSourceRevision,
            "conflict.source_revision",
            ErrorCategory::Conflict,
            Retryability::Manual,
            "error.conflict",
        ),
        (
            ErrorCode::NetworkSource,
            "network.source",
            ErrorCategory::Network,
            Retryability::Automatic,
            "error.network.source",
        ),
        (
            ErrorCode::RateLimitedSource,
            "rate_limited.source",
            ErrorCategory::RateLimited,
            Retryability::After,
            "error.rate_limited.source",
        ),
        (
            ErrorCode::SourceFormatRssAtom,
            "source_format.rss_atom",
            ErrorCategory::SourceFormat,
            Retryability::Never,
            "error.source_format.rss_atom",
        ),
        (
            ErrorCode::StorageSource,
            "storage.source",
            ErrorCategory::Storage,
            Retryability::Never,
            "error.internal",
        ),
        (
            ErrorCode::MigrationSource,
            "migration.source",
            ErrorCategory::Migration,
            Retryability::Never,
            "error.internal",
        ),
    ] {
        assert_error_mapping(code, expected_code, category, retryability, message_key);
    }
}

#[test]
fn configuration_error_codes_have_complete_stable_mappings() {
    for (code, expected_code, category, retryability, message_key) in [
        (
            ErrorCode::ValidationConfiguration,
            "validation.configuration",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ValidationStaleReceipt,
            "validation.stale_validation_receipt",
            ErrorCategory::Validation,
            Retryability::Never,
            "error.validation",
        ),
        (
            ErrorCode::ConflictConfigurationRevision,
            "conflict.configuration_revision",
            ErrorCategory::Conflict,
            Retryability::Manual,
            "error.conflict",
        ),
        (
            ErrorCode::StorageConfiguration,
            "storage.configuration",
            ErrorCategory::Storage,
            Retryability::Never,
            "error.internal",
        ),
    ] {
        assert_error_mapping(code, expected_code, category, retryability, message_key);
    }
}

fn assert_error_mapping(
    code: ErrorCode,
    expected_code: &str,
    category: ErrorCategory,
    retryability: Retryability,
    message_key: &str,
) {
    let error = AppError::from_code(code, "mapping-test");
    assert_eq!(error.contract_version(), 1);
    assert_eq!(error.code(), expected_code);
    assert_eq!(error.category(), category);
    assert_eq!(error.retryability(), retryability);
    assert_eq!(error.message_key(), message_key);
    assert_eq!(error.correlation_id(), "mapping-test");
}
