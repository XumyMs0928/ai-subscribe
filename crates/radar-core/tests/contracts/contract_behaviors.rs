use radar_core::contracts::effects::{EffectLedger, EffectStatus, PlatformEffect, ReportResult};
use radar_core::contracts::errors::{ErrorCategory, Retryability};
use radar_core::contracts::secrets::SecretLeaseInput;
use radar_core::contracts::{effects, errors, manifest, secrets};

#[test]
fn contract_modules_expose_the_v1_behaviors() {
    let _ = (&effects::CONTRACT_VERSION, &errors::CONTRACT_VERSION);
    assert!(manifest::contract_manifest_json().contains("\"contract_version\":1"));
    assert_eq!(secrets::contract_version(), 1);
}

#[test]
fn all_stable_error_enums_have_expected_names() {
    let categories: Vec<_> = ErrorCategory::ALL
        .into_iter()
        .map(ErrorCategory::as_str)
        .collect();
    assert_eq!(categories.len(), 12);
    assert!(categories.contains(&"validation"));
    assert!(categories.contains(&"rate_limited"));
    assert!(categories.contains(&"internal"));

    let retries: Vec<_> = Retryability::ALL
        .into_iter()
        .map(Retryability::as_str)
        .collect();
    assert_eq!(retries, ["never", "manual", "automatic", "after"]);
}

#[test]
fn platform_effect_reports_are_idempotent_and_conflict_safe() {
    let effect = PlatformEffect::new_contract_probe(
        "effect:opaque-1",
        "probe:opaque-1",
        "2026-08-13T00:00:00Z",
    )
    .expect("valid contract probe");
    assert_eq!(effect.status(), EffectStatus::Pending);
    let mut ledger = EffectLedger::default();
    ledger.register(effect).expect("effect registers");

    assert_eq!(
        ledger.report("effect:opaque-1", "probe:opaque-1", EffectStatus::Delivered),
        Ok(ReportResult::Applied)
    );
    assert_eq!(
        ledger.report("effect:opaque-1", "probe:opaque-1", EffectStatus::Delivered),
        Ok(ReportResult::AlreadyApplied)
    );
    let conflict = ledger
        .report("effect:opaque-1", "probe:opaque-1", EffectStatus::Failed)
        .expect_err("different terminal result must conflict");
    assert_eq!(conflict.category(), ErrorCategory::Conflict);
    assert_eq!(
        ledger.status("probe:opaque-1"),
        Some(EffectStatus::Delivered)
    );

    let duplicate_identity = PlatformEffect::new_contract_probe(
        "effect:different",
        "probe:opaque-1",
        "2026-08-13T00:00:00Z",
    )
    .expect("valid duplicate-key effect");
    assert_eq!(
        ledger
            .register(duplicate_identity)
            .expect_err("one key cannot identify two effects")
            .code(),
        "conflict.effect_already_reported"
    );
}

#[test]
fn invalid_contract_fields_map_to_validation_errors() {
    for (id, time) in [
        ("", "2026-08-13T00:00:00Z"),
        ("contains space", "2026-08-13T00:00:00Z"),
        ("valid-id", "2026-08-13 00:00:00"),
        ("valid-id", "2026-13-40T25:61:61Z"),
        (&"x".repeat(129), "2026-08-13T00:00:00Z"),
    ] {
        let error = PlatformEffect::new_contract_probe(id, "valid-key", time)
            .expect_err("invalid input must fail");
        assert_eq!(error.category(), ErrorCategory::Validation);
    }

    let unknown = EffectStatus::parse("future_status").expect_err("unknown enum must fail");
    assert_eq!(unknown.code(), "validation.effect_status");

    let effect = PlatformEffect::new_contract_probe(
        "effect:pending",
        "probe:pending",
        "2026-08-13T00:00:00Z",
    )
    .expect("valid effect");
    let mut ledger = EffectLedger::default();
    ledger.register(effect).expect("effect registers");
    let invalid_transition = ledger
        .report("effect:pending", "probe:pending", EffectStatus::Pending)
        .expect_err("pending is not a valid report");
    assert_eq!(invalid_transition.code(), "validation.effect_status");

    let oversized_fraction = format!("2026-08-13T00:00:00.{}Z", "1".repeat(64));
    assert!(
        PlatformEffect::new_contract_probe("valid-id", "valid-key", oversized_fraction).is_err()
    );
}

#[test]
fn secret_lease_is_single_use_and_never_rendered() {
    const CANARY: &[u8] = b"story-1-1-secret-canary";
    let mut lease = SecretLeaseInput::new("secret:test", CANARY.to_vec()).expect("valid lease");
    assert_eq!(lease.contract_version(), 1);
    assert_eq!(lease.secret_ref(), "secret:test");
    let mut length = 0;
    lease
        .with_secret(|bytes| {
            length = bytes.len();
            Ok(())
        })
        .expect("first use succeeds");
    assert_eq!(length, CANARY.len());

    let error = lease
        .with_secret(|_| Ok(()))
        .expect_err("second use must conflict");
    assert_eq!(error.category(), ErrorCategory::Conflict);
    let observable = format!(
        "{}{}{}{}",
        manifest::contract_manifest_json(),
        manifest::error_codes_json(),
        error.details_allowlisted(),
        error.message_key()
    );
    assert!(!observable.contains("story-1-1-secret-canary"));

    let health_fixture =
        include_str!("../../../../contracts/fixtures/golden/health_success_v1.json");
    assert!(health_fixture.contains("\"checked_at\":null"));

    let Err(invalid) = SecretLeaseInput::new("contains space", CANARY.to_vec()) else {
        panic!("invalid reference must be rejected");
    };
    assert_eq!(invalid.code(), "validation.secret_lease");
}
