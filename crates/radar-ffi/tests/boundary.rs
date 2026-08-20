use radar_ffi::api::health_v1;
use radar_ffi::error::{map_unknown, run_guarded};
use radar_ffi::mapping::{AppErrorWire, ConfigurationValidationResultWireV1, SourceViewWireV1};

const CONFIGURATION_VALIDATION_GOLDEN: &str =
    include_str!("../../../contracts/fixtures/golden/configuration_validation_v1.json");
const SOURCE_VIEW_GOLDEN: &str =
    include_str!("../../../contracts/fixtures/golden/source_view_v1.json");

#[test]
fn approved_health_adapter_maps_explicit_null_json() {
    let wire = health_v1().expect("health adapter succeeds");
    assert_eq!(
        wire.to_json(),
        "{\"contract_version\":1,\"status\":\"ok\",\"checked_at\":null}"
    );
}

#[test]
fn panic_is_mapped_without_leaking_its_message() {
    let error = run_guarded::<(), _>(|| -> Result<(), _> { panic!("private internal detail") })
        .expect_err("panic must become AppError");

    assert_eq!(error.code(), "internal.unexpected");
    assert_eq!(error.category().as_str(), "internal");
    assert!(
        !error
            .details_allowlisted()
            .contains("private internal detail")
    );
}

#[test]
fn unknown_error_is_mapped_without_leaking_debug_text() {
    let error = map_unknown::<()>("private provider response")
        .expect_err("unknown error must become AppError");
    let rendered = format!("{error:?}");

    assert_eq!(error.code(), "internal.unexpected");
    assert!(!rendered.contains("private provider response"));

    let second = map_unknown::<()>("another private response")
        .expect_err("second unknown error must become AppError");
    assert_ne!(error.correlation_id(), second.correlation_id());

    let wire = AppErrorWire::from(&error);
    assert_eq!(wire.code, "internal.unexpected");
    assert_eq!(wire.category, "internal");
    assert!(wire.source_id.is_none());
    assert!(wire.task_id.is_none());
}

#[test]
fn configuration_validation_wire_alias_consumes_the_shared_golden_shape() {
    let fixture: serde_json::Value =
        serde_json::from_str(CONFIGURATION_VALIDATION_GOLDEN).expect("golden fixture");
    let wire: ConfigurationValidationResultWireV1 =
        serde_json::from_value(fixture["expected"].clone()).expect("wire shape");

    assert_eq!(wire.contract_version, 1);
    assert_eq!(wire.validator_version, "attention-configuration-v1");
    assert!(wire.blocking_errors.is_empty());
    assert!(wire.narrowing_risks.is_empty());
    assert!(wire.validation_receipt.is_none());
}

#[test]
fn configuration_validation_wire_preserves_blocking_and_receipt_enums() {
    let blocking: ConfigurationValidationResultWireV1 = serde_json::from_value(serde_json::json!({
        "contract_version": 1,
        "blocking_errors": [{
            "field_path": "include_expression",
            "code": "expression_unparseable",
            "message_key": "configuration.fix.expression_unparseable"
        }],
        "narrowing_risks": [],
        "validator_version": "attention-configuration-v1",
        "normalized_config_hash": "0".repeat(64),
        "validation_receipt": null
    }))
    .expect("blocking wire");
    assert_eq!(
        blocking.blocking_errors[0].code.as_str(),
        "expression_unparseable"
    );

    let narrowing: ConfigurationValidationResultWireV1 =
        serde_json::from_value(serde_json::json!({
            "contract_version": 1,
            "blocking_errors": [],
            "narrowing_risks": [{
                "code": "all_sources_disabled",
                "condition_key": "configuration.risk.all_sources_disabled.condition",
                "consequence_key": "configuration.risk.all_sources_disabled.consequence"
            }],
            "validator_version": "attention-configuration-v1",
            "normalized_config_hash": "1".repeat(64),
            "validation_receipt": {
                "token": "A".repeat(43),
                "normalized_config_hash": "1".repeat(64),
                "validator_version": "attention-configuration-v1"
            }
        }))
        .expect("narrowing wire");
    assert_eq!(
        narrowing.narrowing_risks[0].code.as_str(),
        "all_sources_disabled"
    );
    assert_eq!(
        narrowing.validation_receipt.expect("receipt").token.len(),
        43
    );
}

#[test]
fn source_wire_alias_consumes_the_shared_golden_shape() {
    let fixture: serde_json::Value =
        serde_json::from_str(SOURCE_VIEW_GOLDEN).expect("source golden fixture");
    let wire: SourceViewWireV1 =
        serde_json::from_value(fixture["expected"].clone()).expect("source wire shape");

    assert_eq!(wire.contract_version, 1);
    assert_eq!(wire.source_id, "source:783ed4dcd26da36777ce2baa");
    assert_eq!(wire.display_url, "https://example.com/feed.xml");
    assert!(matches!(
        wire.status,
        radar_core::contracts::dto::source::SourceStatusV1::Ready
    ));
    assert!(wire.last_success_at.is_none());
}
