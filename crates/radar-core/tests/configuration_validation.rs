use radar_core::application::demo::DemoStore;
use radar_core::contracts::dto::configuration_validation::{
    AttentionConfigurationV1, BlockingCodeV1, ConfigurationCandidateContext,
    ConfigurationCandidateV1, NarrowingRiskCodeV1, SaveConfigurationInputV1,
    ValidateConfigurationInputV1,
};
use radar_core::domain::rules::configuration_validation::{
    MAX_RECEIPTS, RECEIPT_TTL_MS, ReceiptRegistry, configuration_hash, configuration_identity,
    deterministic_entropy, normalize, validate_configuration,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

static TEST_DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
type RangeMutation = (&'static str, fn(&mut AttentionConfigurationV1));

struct ScopedDatabase {
    path: PathBuf,
}

impl ScopedDatabase {
    fn new(label: &str) -> Self {
        let sequence = TEST_DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::current_dir()
            .expect("project directory")
            .join("target")
            .join("story-2-1-core-review")
            .join(format!("{label}-{}-{sequence}.sqlite3", std::process::id()));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("test database directory");
        }
        Self { path }
    }
}

impl Drop for ScopedDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let candidate = PathBuf::from(format!("{}{suffix}", self.path.display()));
            let _ = std::fs::remove_file(candidate);
        }
    }
}

const VALID: &str =
    include_str!("../../../contracts/fixtures/configuration-validation/valid/basic-v1.json");
const BLOCKING_CASES: &str =
    include_str!("../../../contracts/fixtures/configuration-validation/blocking/cases-v1.json");
const NARROWING_CASES: &str =
    include_str!("../../../contracts/fixtures/configuration-validation/narrowing/cases-v1.json");
const VALIDATION_GOLDEN: &str =
    include_str!("../../../contracts/fixtures/golden/configuration_validation_v1.json");

#[test]
fn valid_fixture_has_stable_hash_and_no_confirmation() {
    let configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    let mut receipts = ReceiptRegistry::for_tests(1_700_000_000_000, deterministic_entropy(7));
    let result = validate_configuration(
        &configuration,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("validation");

    assert!(result.blocking_errors.is_empty());
    assert!(result.narrowing_risks.is_empty());
    assert!(result.validation_receipt.is_none());
    assert_eq!(result.normalized_config_hash.len(), 64);
    assert!(
        result
            .normalized_config_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
}

#[test]
fn production_validator_matches_the_shared_golden_wire_result() {
    let golden: serde_json::Value = serde_json::from_str(VALIDATION_GOLDEN).expect("golden");
    let configuration: AttentionConfigurationV1 =
        serde_json::from_value(golden["input"].clone()).expect("golden input");
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(11));
    let actual = validate_configuration(
        &configuration,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("validation");

    assert_eq!(
        serde_json::to_value(actual).expect("wire result"),
        golden["expected"]
    );
}

#[test]
fn shared_fixture_suites_cover_every_stable_blocking_and_narrowing_code() {
    let blocking: serde_json::Value =
        serde_json::from_str(BLOCKING_CASES).expect("blocking fixture");
    let narrowing: serde_json::Value =
        serde_json::from_str(NARROWING_CASES).expect("narrowing fixture");
    let blocking_codes = blocking["cases"]
        .as_array()
        .expect("blocking cases")
        .iter()
        .map(|case| case["expected_code"].as_str().expect("blocking code"))
        .collect::<std::collections::BTreeSet<_>>();
    let narrowing_codes = narrowing["cases"]
        .as_array()
        .expect("narrowing cases")
        .iter()
        .map(|case| case["expected_code"].as_str().expect("risk code"))
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(
        blocking_codes,
        std::collections::BTreeSet::from([
            "expression_unparseable",
            "invalid_source_or_unsupported_protocol",
            "lower_bound_above_upper_bound",
            "value_out_of_range",
        ])
    );
    assert_eq!(
        narrowing_codes,
        std::collections::BTreeSet::from([
            "all_high_trust_candidates_filtered",
            "all_sources_disabled",
        ])
    );
}

#[test]
fn every_blocking_fixture_executes_through_the_production_validator() {
    let blocking: serde_json::Value =
        serde_json::from_str(BLOCKING_CASES).expect("blocking fixture");
    for case in blocking["cases"].as_array().expect("blocking cases") {
        let mut configuration: AttentionConfigurationV1 =
            serde_json::from_str(VALID).expect("base");
        match case["field_path"].as_str().expect("field") {
            "include_expression" => {
                configuration.include_expression = case["value"].as_str().expect("text").into();
            }
            "refresh_interval_minutes" => {
                configuration.refresh_interval_minutes =
                    u32::try_from(case["value"].as_u64().expect("number")).expect("u32");
            }
            "minimum_trust" => {
                configuration.minimum_trust =
                    u8::try_from(case["value"].as_u64().expect("number")).expect("u8");
            }
            "source_preferences[0].identifier" => {
                configuration.source_preferences[0].identifier =
                    case["value"].as_str().expect("source").into();
            }
            field => panic!("unhandled fixture field {field}"),
        }
        let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(9));
        let actual = validate_configuration(
            &configuration,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("validation");
        let codes: Vec<String> = actual
            .blocking_errors
            .iter()
            .map(|error| {
                serde_json::to_value(error.code)
                    .expect("code")
                    .as_str()
                    .expect("string")
                    .to_owned()
            })
            .collect();
        assert!(
            codes
                .iter()
                .any(|code| code == case["expected_code"].as_str().expect("expected"))
        );
    }
}

#[test]
fn every_narrowing_fixture_executes_through_the_production_validator() {
    let narrowing: serde_json::Value =
        serde_json::from_str(NARROWING_CASES).expect("narrowing fixture");
    for case in narrowing["cases"].as_array().expect("narrowing cases") {
        let mut configuration: AttentionConfigurationV1 =
            serde_json::from_str(VALID).expect("base");
        match case["mutation"].as_str().expect("mutation") {
            "disable_all_sources" => configuration
                .source_preferences
                .iter_mut()
                .for_each(|source| source.enabled = false),
            "exclude_rust" => configuration.exclude_expression = "Rust".into(),
            mutation => panic!("unhandled fixture mutation {mutation}"),
        }
        let real_candidates = case["real_candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .map(|candidate| ConfigurationCandidateV1 {
                source_kind: candidate["source_kind"].as_str().expect("kind").into(),
                searchable_text: candidate["searchable_text"].as_str().expect("text").into(),
            })
            .collect();
        let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(10));
        let actual = validate_configuration(
            &configuration,
            &ConfigurationCandidateContext { real_candidates },
            &mut receipts,
        )
        .expect("validation");
        let codes: Vec<String> = actual
            .narrowing_risks
            .iter()
            .map(|risk| {
                serde_json::to_value(risk.code)
                    .expect("code")
                    .as_str()
                    .expect("string")
                    .to_owned()
            })
            .collect();
        assert!(
            codes
                .iter()
                .any(|code| code == case["expected_code"].as_str().expect("expected"))
        );
    }
}

#[test]
fn candidate_risk_uses_the_same_and_or_precedence_as_validation() {
    let mut configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    configuration.include_expression = "Rust AND release OR Python".into();
    configuration.exclude_expression.clear();
    let candidates = ConfigurationCandidateContext {
        real_candidates: vec![ConfigurationCandidateV1 {
            source_kind: "rss".into(),
            searchable_text: "Rust ownership guide".into(),
        }],
    };
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(12));
    let result =
        validate_configuration(&configuration, &candidates, &mut receipts).expect("validation");

    assert!(result.blocking_errors.is_empty());
    assert_eq!(result.narrowing_risks.len(), 1);
    assert_eq!(
        result.narrowing_risks[0].code,
        NarrowingRiskCodeV1::AllHighTrustCandidatesFiltered
    );

    configuration.include_expression = "Rust AND ownership OR Python".into();
    let accepted =
        validate_configuration(&configuration, &candidates, &mut receipts).expect("validation");
    assert!(accepted.narrowing_risks.is_empty());
}

#[test]
fn rss_aliases_share_narrowing_risk_and_canonical_duplicate_identity() {
    let mut configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    configuration.include_expression = "definitely_missing".to_owned();
    configuration.exclude_expression.clear();
    let candidates = ConfigurationCandidateContext {
        real_candidates: vec![ConfigurationCandidateV1 {
            source_kind: "rss_atom".to_owned(),
            searchable_text: "Rust release notes".to_owned(),
        }],
    };
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(13));
    let result = validate_configuration(&configuration, &candidates, &mut receipts)
        .expect("alias validation");
    assert_eq!(
        result.narrowing_risks[0].code,
        NarrowingRiskCodeV1::AllHighTrustCandidatesFiltered
    );

    let mut duplicate = configuration;
    let mut equivalent = duplicate.source_preferences[0].clone();
    equivalent.source_kind = "rss_atom".to_owned();
    equivalent.identifier = "https://example.com:443/feed.xml#duplicate".to_owned();
    duplicate.source_preferences.push(equivalent);
    let duplicate_result = validate_configuration(
        &duplicate,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("duplicate validation");
    assert!(duplicate_result.blocking_errors.iter().any(|error| {
        error.field_path == "source_preferences[1].identifier"
            && error.code == BlockingCodeV1::ValueOutOfRange
    }));
}

#[test]
fn parser_and_range_errors_use_the_four_stable_categories() {
    let mut configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    configuration.include_expression = "Rust AND (".into();
    configuration.refresh_interval_minutes = 14;
    configuration.minimum_trust = 90;
    configuration.maximum_trust = 10;
    configuration.source_preferences[0].identifier = "file:///private/feed".into();
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(1));
    let result = validate_configuration(
        &configuration,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("validation");
    let codes: Vec<_> = result
        .blocking_errors
        .iter()
        .map(|error| error.code)
        .collect();

    assert!(codes.contains(&BlockingCodeV1::ExpressionUnparseable));
    assert!(codes.contains(&BlockingCodeV1::ValueOutOfRange));
    assert!(codes.contains(&BlockingCodeV1::LowerBoundAboveUpperBound));
    assert!(codes.contains(&BlockingCodeV1::InvalidSourceOrUnsupportedProtocol));
    assert!(result.narrowing_risks.is_empty());
    assert!(result.validation_receipt.is_none());
}

#[test]
fn source_urls_time_windows_and_track_paths_are_validated_semantically() {
    let base: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    for invalid in ["https://", "https://?query", "https://[invalid"] {
        let mut configuration = base.clone();
        configuration.source_preferences[0].identifier = invalid.into();
        let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(2));
        let result = validate_configuration(
            &configuration,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("validation");
        assert!(result.blocking_errors.iter().any(|error| {
            error.code == BlockingCodeV1::InvalidSourceOrUnsupportedProtocol
                && error.field_path == "source_preferences[0].identifier"
        }));
    }

    let mut uppercase_scheme = base.clone();
    uppercase_scheme.source_preferences[0].identifier = "HTTPS://example.invalid/feed".into();
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(3));
    assert!(
        validate_configuration(
            &uppercase_scheme,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("uppercase scheme")
        .blocking_errors
        .is_empty()
    );

    let mut equal_instants = base.clone();
    equal_instants.active_from = Some("2026-01-01T00:00:00.1Z".into());
    equal_instants.active_until = Some("2026-01-01T00:00:00.10Z".into());
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(4));
    assert!(
        validate_configuration(
            &equal_instants,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("equivalent fractional instant")
        .blocking_errors
        .is_empty()
    );

    let mut invalid_id = base;
    invalid_id.tracks[0].id = "bad id".into();
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(5));
    let result = validate_configuration(
        &invalid_id,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("track validation");
    assert!(result.blocking_errors.iter().any(|error| {
        error.field_path == "tracks[0].id" && error.code == BlockingCodeV1::ValueOutOfRange
    }));
}

#[test]
fn declared_name_expression_schedule_and_range_boundaries_have_direct_evidence() {
    let base: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    let validate = |configuration: &AttentionConfigurationV1| {
        let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(6));
        validate_configuration(
            configuration,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("validation")
        .blocking_errors
    };

    let mut empty_name = base.clone();
    empty_name.tracks[0].name.clear();
    assert!(
        validate(&empty_name)
            .iter()
            .any(|error| error.field_path == "tracks[0].name")
    );

    let mut duplicate = base.clone();
    duplicate.tracks.push(duplicate.tracks[0].clone());
    assert!(
        validate(&duplicate).iter().any(|error| {
            matches!(error.field_path.as_str(), "tracks[1].id" | "tracks[1].name")
        })
    );

    let mut expression_limit = base.clone();
    expression_limit.include_expression = "x".repeat(513);
    assert!(
        validate(&expression_limit)
            .iter()
            .any(|error| error.code == BlockingCodeV1::ExpressionUnparseable)
    );

    let mut schedule = base.clone();
    schedule.quiet_hours.enabled = true;
    schedule.quiet_hours.end = schedule.quiet_hours.start.clone();
    schedule.notification_frequency.enabled = true;
    schedule.notification_frequency.max_per_24h = None;
    assert!(
        validate(&schedule)
            .iter()
            .any(|error| error.field_path == "schedule")
    );

    let mut inverted = base.clone();
    inverted.active_from = Some("2026-01-02T00:00:00Z".into());
    inverted.active_until = Some("2026-01-01T00:00:00Z".into());
    assert!(validate(&inverted).iter().any(|error| {
        error.field_path == "active_from" && error.code == BlockingCodeV1::LowerBoundAboveUpperBound
    }));

    for refresh_interval_minutes in [15, 10_080] {
        let mut boundary = base.clone();
        boundary.refresh_interval_minutes = refresh_interval_minutes;
        assert!(validate(&boundary).is_empty());
    }
}

#[test]
fn blocking_errors_report_precise_focusable_source_refresh_and_threshold_paths() {
    let base: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    let validate = |configuration: &AttentionConfigurationV1| {
        let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(7));
        validate_configuration(
            configuration,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("validation")
        .blocking_errors
    };

    let mut invalid_source = base.clone();
    invalid_source.source_preferences[0].source_kind = "unsupported".into();
    invalid_source.source_preferences[0].identifier.clear();
    invalid_source.source_preferences[0].trust = 101;
    let source_errors = validate(&invalid_source);
    for (field_path, code) in [
        (
            "source_preferences[0].source_kind",
            BlockingCodeV1::InvalidSourceOrUnsupportedProtocol,
        ),
        (
            "source_preferences[0].identifier",
            BlockingCodeV1::ValueOutOfRange,
        ),
        (
            "source_preferences[0].trust",
            BlockingCodeV1::ValueOutOfRange,
        ),
    ] {
        assert!(
            source_errors
                .iter()
                .any(|error| { error.field_path == field_path && error.code == code })
        );
    }
    assert!(
        source_errors
            .iter()
            .all(|error| error.field_path != "source_preferences[0]")
    );

    let mut duplicate_source = base.clone();
    duplicate_source
        .source_preferences
        .push(duplicate_source.source_preferences[0].clone());
    assert!(validate(&duplicate_source).iter().any(|error| {
        error.field_path == "source_preferences[1].identifier"
            && error.code == BlockingCodeV1::ValueOutOfRange
    }));

    let range_cases: [RangeMutation; 4] = [
        (
            "refresh_interval_minutes",
            |configuration: &mut AttentionConfigurationV1| {
                configuration.refresh_interval_minutes = 14;
            },
        ),
        (
            "minimum_trust",
            |configuration: &mut AttentionConfigurationV1| {
                configuration.minimum_trust = 101;
            },
        ),
        (
            "maximum_trust",
            |configuration: &mut AttentionConfigurationV1| {
                configuration.maximum_trust = 101;
            },
        ),
        (
            "alert_threshold",
            |configuration: &mut AttentionConfigurationV1| {
                configuration.alert_threshold = 101;
            },
        ),
    ];
    for (field_path, mutate) in range_cases {
        let mut configuration = base.clone();
        mutate(&mut configuration);
        let errors = validate(&configuration);
        assert!(errors.iter().any(|error| {
            error.field_path == field_path && error.code == BlockingCodeV1::ValueOutOfRange
        }));
        assert!(
            errors
                .iter()
                .all(|error| error.field_path != "refresh_or_threshold")
        );
    }
}

#[test]
fn risk_validation_fails_closed_when_receipt_entropy_is_unavailable() {
    let mut configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    configuration.source_preferences[0].enabled = false;
    let mut receipts = ReceiptRegistry::for_tests(0, Vec::new());
    assert!(
        validate_configuration(
            &configuration,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .is_err()
    );
}

#[test]
fn narrowing_risk_issues_single_use_receipt_without_inventing_empty_candidate_risk() {
    let mut configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    configuration.source_preferences[0].enabled = false;
    let mut receipts = ReceiptRegistry::for_tests(10_000, deterministic_entropy(3));
    let result = validate_configuration(
        &configuration,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("validation");

    assert_eq!(result.narrowing_risks.len(), 1);
    assert_eq!(
        result.narrowing_risks[0].code,
        NarrowingRiskCodeV1::AllSourcesDisabled
    );
    assert!(result.validation_receipt.is_some());
    assert!(
        !result
            .narrowing_risks
            .iter()
            .any(|risk| { risk.code == NarrowingRiskCodeV1::AllHighTrustCandidatesFiltered })
    );
}

#[test]
fn receipt_rejects_forgery_expiration_replay_and_changed_identity() {
    let mut configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    configuration.source_preferences[0].enabled = false;
    let hash = configuration_hash(&normalize(&configuration));
    let identity = configuration_identity(&normalize(&configuration));
    let risks = [NarrowingRiskCodeV1::AllSourcesDisabled];
    let mut receipts = ReceiptRegistry::for_tests(1_000, deterministic_entropy(20));
    let receipt = validate_configuration(
        &configuration,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("validation")
    .validation_receipt
    .expect("receipt");

    let mut forged = receipt.clone();
    forged.token.replace_range(..1, "Z");
    assert!(!receipts.is_valid(&forged, &hash, &identity, &risks));
    let mut wrong_hash = receipt.clone();
    wrong_hash.normalized_config_hash = "0".repeat(64);
    assert!(!receipts.is_valid(&wrong_hash, &hash, &identity, &risks));
    let mut wrong_validator = receipt.clone();
    wrong_validator.validator_version = "attention-configuration-v2".into();
    assert!(!receipts.is_valid(&wrong_validator, &hash, &identity, &risks));
    let mut changed_identity = identity.clone();
    changed_identity.push(b' ');
    assert!(!receipts.is_valid(&receipt, &hash, &changed_identity, &risks));
    assert!(!receipts.is_valid(
        &receipt,
        &hash,
        &identity,
        &[
            NarrowingRiskCodeV1::AllSourcesDisabled,
            NarrowingRiskCodeV1::AllHighTrustCandidatesFiltered,
        ],
    ));
    assert!(receipts.consume(&receipt, &hash, &identity, &risks));
    assert!(!receipts.consume(&receipt, &hash, &identity, &risks));

    let replacement = validate_configuration(
        &configuration,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("validation")
    .validation_receipt
    .expect("replacement");
    receipts.set_test_time(1_000 + RECEIPT_TTL_MS);
    assert!(!receipts.is_valid(&replacement, &hash, &identity, &risks));
}

#[test]
fn receipt_registry_evicts_the_oldest_available_entry_at_its_fixed_capacity() {
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(40));
    let mut issued = Vec::new();
    for index in 0..=MAX_RECEIPTS {
        let mut configuration: AttentionConfigurationV1 =
            serde_json::from_str(VALID).expect("fixture");
        configuration.tracks[0].name = format!("Rust {index}");
        configuration.source_preferences[0].enabled = false;
        let result = validate_configuration(
            &configuration,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("validation");
        issued.push((
            result.validation_receipt.expect("receipt"),
            result.normalized_config_hash,
            configuration_identity(&normalize(&configuration)),
        ));
    }

    assert!(!receipts.is_valid(
        &issued[0].0,
        &issued[0].1,
        &issued[0].2,
        &[NarrowingRiskCodeV1::AllSourcesDisabled],
    ));
    assert!(receipts.is_valid(
        &issued[MAX_RECEIPTS].0,
        &issued[MAX_RECEIPTS].1,
        &issued[MAX_RECEIPTS].2,
        &[NarrowingRiskCodeV1::AllSourcesDisabled],
    ));
}

#[test]
fn consumed_receipts_do_not_evict_older_available_receipts() {
    let mut receipts = ReceiptRegistry::for_tests(0, deterministic_entropy(90));
    let mut issued = Vec::new();
    for index in 0..MAX_RECEIPTS {
        let mut configuration: AttentionConfigurationV1 =
            serde_json::from_str(VALID).expect("fixture");
        configuration.tracks[0].name = format!("Capacity {index}");
        configuration.source_preferences[0].enabled = false;
        let identity = configuration_identity(&normalize(&configuration));
        let result = validate_configuration(
            &configuration,
            &ConfigurationCandidateContext::default(),
            &mut receipts,
        )
        .expect("validation");
        issued.push((
            result.validation_receipt.expect("receipt"),
            result.normalized_config_hash,
            identity,
        ));
    }
    let risks = [NarrowingRiskCodeV1::AllSourcesDisabled];
    let last = issued.last().expect("last");
    assert!(receipts.consume(&last.0, &last.1, &last.2, &risks));

    let mut replacement: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    replacement.tracks[0].name = "Capacity replacement".into();
    replacement.source_preferences[0].enabled = false;
    validate_configuration(
        &replacement,
        &ConfigurationCandidateContext::default(),
        &mut receipts,
    )
    .expect("replacement");

    assert!(receipts.is_valid(&issued[0].0, &issued[0].1, &issued[0].2, &risks));
}

#[test]
fn versioned_save_replays_committed_response_before_revision_or_receipt_checks() {
    let mut store = DemoStore::open_in_memory().expect("store");
    let initial = store.get_configuration().expect("initial configuration");
    let configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    let input = SaveConfigurationInputV1 {
        contract_version: 1,
        expected_normalized_config_hash: configuration_hash(&normalize(&configuration)),
        configuration,
        expected_revision: initial.revision,
        idempotency_key: "config-save-0001".into(),
        validation_receipt: None,
    };

    let saved = store.save_attention_configuration(&input).expect("save");
    let replayed = store.save_attention_configuration(&input).expect("replay");

    assert_eq!(saved, replayed);
    assert_eq!(saved.revision, initial.revision + 1);
    assert_eq!(store.get_configuration().expect("current"), saved);
}

#[test]
fn narrowing_save_requires_matching_single_use_receipt_but_same_intent_replays() {
    let mut store = DemoStore::open_in_memory().expect("store");
    let initial = store.get_configuration().expect("initial configuration");
    let mut configuration: AttentionConfigurationV1 = serde_json::from_str(VALID).expect("fixture");
    configuration.source_preferences[0].enabled = false;
    let validation = store
        .validate_attention_configuration(&ValidateConfigurationInputV1 {
            contract_version: 1,
            configuration: configuration.clone(),
        })
        .expect("validation");
    let receipt = validation.validation_receipt.expect("risk receipt");
    let input = SaveConfigurationInputV1 {
        contract_version: 1,
        expected_normalized_config_hash: validation.normalized_config_hash,
        configuration,
        expected_revision: initial.revision,
        idempotency_key: "config-risk-save-0001".into(),
        validation_receipt: Some(receipt),
    };

    let saved = store
        .save_attention_configuration(&input)
        .expect("confirmed save");
    assert_eq!(
        store
            .save_attention_configuration(&input)
            .expect("response-lost replay"),
        saved
    );
    let mut different_intent = input;
    different_intent.idempotency_key = "config-risk-save-0002".into();
    different_intent.expected_revision = saved.revision;
    let error = store
        .save_attention_configuration(&different_intent)
        .expect_err("consumed receipt cannot authorize a new intent");
    assert_eq!(error.code(), "validation.stale_validation_receipt");
}

#[test]
fn concurrent_writers_report_one_stable_revision_conflict() {
    let scoped = ScopedDatabase::new("concurrent-save");
    let initial_store = DemoStore::open(&scoped.path).expect("initialize store");
    let initial = initial_store.get_configuration().expect("initial");
    drop(initial_store);
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    let stores = [
        DemoStore::open(&scoped.path).expect("open writer zero"),
        DemoStore::open(&scoped.path).expect("open writer one"),
    ];
    for (index, mut store) in stores.into_iter().enumerate() {
        let barrier = Arc::clone(&barrier);
        let mut input = SaveConfigurationInputV1 {
            contract_version: 1,
            configuration: initial.configuration.clone(),
            expected_revision: initial.revision,
            expected_normalized_config_hash: initial.normalized_config_hash.clone(),
            idempotency_key: format!("concurrent-save-{index}"),
            validation_receipt: None,
        };
        input.configuration.tracks[0].name = format!("Concurrent winner {index}");
        input.expected_normalized_config_hash =
            configuration_hash(&normalize(&input.configuration));
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            store
                .save_attention_configuration(&input)
                .map_or_else(|error| error.code().to_owned(), |_| "ok".to_owned())
        }));
    }
    barrier.wait();
    let mut outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("writer"))
        .collect::<Vec<_>>();
    outcomes.sort();
    assert_eq!(outcomes, ["conflict.configuration_revision", "ok"]);
}
