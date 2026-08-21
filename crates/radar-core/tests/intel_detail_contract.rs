use radar_core::contracts::dto::intel_detail::{
    IntelEvidenceDetailV1, OpenIntelOriginalInputV1, QueryIntelEvidenceDetailInputV1,
};

const FIXTURE: &str = include_str!("../../../contracts/fixtures/intel-detail/phase1-v1.json");
const LOCAL_STATES: &str =
    include_str!("../../../contracts/fixtures/intel-detail/phase1-local-states-v1.json");

#[test]
fn phase1_fixture_is_the_exact_versioned_detail_contract() {
    let detail: IntelEvidenceDetailV1 = serde_json::from_str(FIXTURE).expect("detail fixture");
    assert_eq!(detail.contract_version, 1);
    assert_eq!(detail.provenance.len(), 2);
    assert_eq!(detail.provenance[0].role.as_str(), "primary");
    assert_eq!(detail.ai_status.as_str(), "unavailable");
}

#[test]
fn local_state_fixture_family_covers_unavailable_and_stale_without_copying_facts() {
    let base: serde_json::Value = serde_json::from_str(FIXTURE).expect("base fixture");
    let family: serde_json::Value = serde_json::from_str(LOCAL_STATES).expect("state family");
    let cases = family["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 2);
    for case in cases {
        let mut candidate = base.clone();
        candidate["facts"]["source_summary"] = case["source_summary"].clone();
        candidate["rule_status"] = case["rule_status"].clone();
        candidate["rule_issue_code"] = case["rule_issue_code"].clone();
        candidate["rule"] = serde_json::Value::Null;
        serde_json::from_value::<IntelEvidenceDetailV1>(candidate)
            .expect("local state remains an exact detail contract");
    }
}

#[test]
fn detail_and_open_inputs_reject_unknown_fields_and_missing_provenance() {
    let query = serde_json::from_str::<QueryIntelEvidenceDetailInputV1>(
        r#"{"contract_version":1,"intel_item_id":"intel:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","url":"https://forbidden.example"}"#,
    );
    assert!(query.is_err());

    let open = serde_json::from_str::<OpenIntelOriginalInputV1>(
        r#"{"contract_version":1,"intel_item_id":"intel:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
    );
    assert!(open.is_err());
}
