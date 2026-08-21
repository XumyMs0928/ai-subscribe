//! Human-readable contract mirror generation from Rust-owned constants.

use super::errors::{ErrorCategory, ErrorCode, Retryability};
use crate::contracts::dto::configuration_validation::{BlockingCodeV1, NarrowingRiskCodeV1};
use crate::domain::rules::configuration_validation::{
    HIGH_TRUST_THRESHOLD, MAX_EXPRESSION_BYTES, MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_TERMS,
    MAX_NOTIFICATION_CAP, MAX_PERCENT, MAX_RECEIPTS, MAX_REFRESH_MINUTES,
    MAX_SOURCE_IDENTIFIER_BYTES, MAX_SOURCE_PREFERENCES, MAX_TERM_SCALARS, MAX_TRACK_NAME_SCALARS,
    MAX_TRACKS, MIN_NOTIFICATION_CAP, MIN_REFRESH_MINUTES, MIN_SOURCE_PREFERENCES, MIN_TRACKS,
    RECEIPT_TTL_MS, VALIDATOR_VERSION,
};
use crate::domain::rules::intelligence_value::{
    FRESHNESS_WEIGHT, IMPORTANCE_HIGH_MAX, IMPORTANCE_HIGH_MIN, IMPORTANCE_LOW_MAX,
    IMPORTANCE_MEDIUM_MAX, IMPORTANCE_MEDIUM_MIN, INTELLIGENCE_VALUE_RULE_VERSION,
    SOURCE_TRUST_WEIGHT, TECHNICAL_IMPACT_WEIGHT, TRACK_WEIGHT, USER_RULE_WEIGHT,
};

#[must_use]
#[allow(clippy::too_many_lines)] // One generated mirror keeps the exact v1 field ordering reviewable in a single contract boundary.
pub fn contract_manifest_json() -> String {
    let demo_fingerprint = crate::application::demo::demo_fixture_fingerprint();
    concat!(
            "{\"generator\":\"radar-core-contracts-v1\",",
            "\"contract_version\":1,",
            "\"compatibility\":\"wire-additive-only-within-v1\",",
            "\"dtos\":{",
            "\"health_status\":[\"contract_version\",\"status\",\"checked_at\"],",
            "\"app_error\":[\"contract_version\",\"code\",\"category\",\"message_key\",\"retryability\",\"source_id\",\"task_id\",\"details_allowlisted\",\"correlation_id\"],",
            "\"platform_effect\":[\"contract_version\",\"effect_id\",\"idempotency_key\",\"kind\",\"payload_version\",\"expires_at\",\"status\"],",
            "\"secret_lease_input\":[\"contract_version\",\"secret_ref\",\"secret_bytes_non_serializable\"],",
            "\"demo_catalog\":[\"contract_version\",\"dataset_id\",\"items\"],",
            "\"demo_page\":[\"contract_version\",\"dataset_id\",\"items\",\"next_cursor\"],",
            "\"demo_item\":[\"id\",\"data_origin\",\"publisher\",\"title\",\"track\",\"summary\",\"original_url\",\"importance\",\"ai_status\",\"published_at\",\"collected_at\"],",
            "\"demo_evidence_detail\":[\"contract_version\",\"dataset_id\",\"id\",\"data_origin\",\"publisher\",\"title\",\"track\",\"summary\",\"original_url\",\"published_at\",\"collected_at\",\"what_happened\",\"why_it_matters\",\"possible_impact\",\"importance\",\"facts\",\"rule_reasons\",\"ai_content\",\"ai_confidence_percent\",\"ai_status\",\"provenance\"],",
            "\"setup_progress\":[\"contract_version\",\"revision\",\"configuration_revision\",\"overall_status\",\"steps\",\"next_step_id\",\"defaults\",\"saved_config\"],",
            "\"setup_step_progress\":[\"contract_version\",\"step_id\",\"status\",\"saved_fields_version\"],",
            "\"setup_option\":[\"id\",\"label\",\"is_demo\"],",
            "\"setup_defaults\":[\"contract_version\",\"fixture_id\",\"default_track_ids\",\"default_source_example_ids\",\"default_refresh_cadence\",\"tracks\",\"source_examples\",\"refresh_cadences\"],",
            "\"setup_saved_config\":[\"track_ids\",\"source_example_ids\",\"refresh_cadence\",\"ai_data_disclosure_acknowledged\"],",
            "\"save_setup_step_input\":[\"contract_version\",\"step_id\",\"action\",\"selected_values\",\"expected_revision\",\"expected_configuration_revision\",\"idempotency_key\"],",
            "\"attention_track\":[\"id\",\"name\",\"enabled\"],",
            "\"source_preference\":[\"source_kind\",\"identifier\",\"enabled\",\"trust\"],",
            "\"quiet_hours\":[\"enabled\",\"start\",\"end\"],",
            "\"notification_frequency\":[\"enabled\",\"max_per_24h\"],",
            "\"attention_configuration\":[\"contract_version\",\"tracks\",\"include_expression\",\"exclude_expression\",\"source_preferences\",\"refresh_enabled\",\"refresh_interval_minutes\",\"minimum_trust\",\"maximum_trust\",\"alert_threshold\",\"quiet_hours\",\"notification_frequency\",\"active_from\",\"active_until\"],",
            "\"configuration_view\":[\"contract_version\",\"revision\",\"validator_version\",\"normalized_config_hash\",\"configuration\",\"updated_at_ms\"],",
            "\"validate_configuration_input\":[\"contract_version\",\"configuration\"],",
            "\"configuration_validation_result\":[\"contract_version\",\"blocking_errors\",\"narrowing_risks\",\"validator_version\",\"normalized_config_hash\",\"validation_receipt\"],",
            "\"configuration_blocking_error\":[\"field_path\",\"code\",\"message_key\"],",
            "\"configuration_narrowing_risk\":[\"code\",\"condition_key\",\"consequence_key\"],",
            "\"configuration_validation_receipt\":[\"token\",\"normalized_config_hash\",\"validator_version\"],",
            "\"demo_provenance\":[\"source_kind\",\"publisher\",\"author\",\"original_title\",\"original_url\",\"published_at\",\"collected_at\",\"first_discovered_at\",\"last_updated_at\",\"availability_status\",\"deterministic_association_basis\"],",
            "\"save_configuration_input\":[\"contract_version\",\"configuration\",\"expected_revision\",\"expected_normalized_config_hash\",\"idempotency_key\",\"validation_receipt\"],",
            "\"save_source_input\":[\"contract_version\",\"source_kind\",\"url\",\"expected_configuration_revision\",\"idempotency_key\"],",
            "\"source_view\":[\"contract_version\",\"source_id\",\"source_kind\",\"display_url\",\"enabled\",\"revision\",\"created_at\",\"updated_at\",\"last_success_at\",\"freshness\",\"status\",\"retryability\",\"next_allowed_at\"],",
            "\"source_page\":[\"contract_version\",\"items\",\"next_cursor\"],",
            "\"start_sync_input\":[\"contract_version\",\"target\",\"idempotency_key\",\"foreground_budget_ms\"],",
            "\"task_ref\":[\"contract_version\",\"task_id\",\"state\",\"revision\"],",
            "\"source_sync_status\":[\"contract_version\",\"source_id\",\"source_revision\",\"state\",\"last_success_at\",\"error_code\",\"next_allowed_at\",\"updated_at\"],",
            "\"task_snapshot\":[\"contract_version\",\"task_id\",\"target\",\"state\",\"revision\",\"created_at\",\"started_at\",\"finished_at\",\"updated_at\",\"error_summary\",\"result_ref\",\"sources\"],",
            "\"source_readiness\":[\"contract_version\",\"source_id\",\"source_kind\",\"status\",\"last_success_at\",\"next_allowed_at\"],",
            "\"source_delivery_readiness\":[\"contract_version\",\"required_source_kinds\",\"status\",\"sources\"],",
            "\"sync_health_summary\":[\"contract_version\",\"latest_task\",\"pending_task_count\",\"last_success_at\",\"freshness\",\"source_results\",\"readiness\"],",
            "\"get_sync_result_input\":[\"contract_version\",\"sync_run_id\",\"cursor\",\"limit\"],",
            "\"sync_result_counts\":[\"inserted\",\"updated\",\"skipped\",\"failed\"],",
            "\"sync_source_result\":[\"contract_version\",\"source_id\",\"source_revision\",\"source_kind\",\"publisher\",\"status\",\"counts\",\"error_code\"],",
            "\"sync_result_item\":[\"contract_version\",\"result_item_id\",\"sync_run_id\",\"source_id\",\"intel_item_id\",\"source_kind\",\"publisher\",\"original_title\",\"published_at\",\"collected_at\",\"original_url\",\"disposition\"],",
            "\"sync_result_summary\":[\"contract_version\",\"sync_run_id\",\"task_id\",\"outcome\",\"started_at\",\"finished_at\",\"counts\",\"sources\"],",
            "\"sync_result_page\":[\"contract_version\",\"summary\",\"items\",\"next_cursor\"],",
            "\"rule_factor\":[\"factor\",\"points\",\"reason_codes\"],",
            "\"filter_reason\":[\"code\",\"actual\",\"threshold\"],",
            "\"rule_evaluation\":[\"contract_version\",\"rule_version\",\"fact_revision\",\"configuration_revision\",\"configuration_hash\",\"evaluated_at_ms\",\"score\",\"importance\",\"disposition\",\"matched_track_ids\",\"factors\",\"filter_reasons\",\"ai_status\"],",
            "\"intel_feed_filters\":[\"track_ids\",\"source_ids\",\"time_window\",\"importance\"],",
            "\"query_intel_feed_input\":[\"contract_version\",\"stream\",\"filters\",\"sort\",\"cursor\",\"limit\"],",
            "\"intel_feed_item\":[\"contract_version\",\"intel_item_id\",\"source_id\",\"source_kind\",\"publisher\",\"title\",\"source_excerpt\",\"excerpt_truncated\",\"published_at\",\"collected_at\",\"importance\",\"score\",\"matched_track_ids\",\"stream_disposition\",\"ai_status\"],",
            "\"intel_feed_page\":[\"contract_version\",\"stream\",\"filters\",\"sort\",\"rule_version\",\"configuration_revision\",\"configuration_hash\",\"as_of_ms\",\"items\",\"next_cursor\"],",
            "\"query_intel_evidence_detail_input\":[\"contract_version\",\"intel_item_id\"],",
            "\"source_facts\":[\"intel_item_id\",\"fact_revision\",\"content_hash\",\"content_state\",\"publisher\",\"title\",\"source_summary\",\"published_at\",\"collected_at\"],",
            "\"rule_explanation\":[\"rule_version\",\"configuration_revision\",\"configuration_hash\",\"evaluated_at_ms\",\"score\",\"importance\",\"disposition\",\"matched_track_ids\",\"factors\",\"filter_reasons\"],",
            "\"intel_provenance\":[\"provenance_id\",\"intel_item_id\",\"role\",\"source_id\",\"source_kind\",\"publisher\",\"author\",\"author_availability\",\"original_title\",\"display_url\",\"published_at\",\"collected_at\",\"first_discovered_at\",\"last_updated_at\",\"availability_status\",\"can_open_original\"],",
            "\"association_evidence\":[\"status\",\"issue_code\",\"relation_type\",\"evidence_basis\",\"basis_version\"],",
            "\"intel_evidence_detail\":[\"contract_version\",\"facts\",\"rule_status\",\"rule_issue_code\",\"rule\",\"ai_status\",\"provenance\",\"association\"],",
            "\"open_intel_original_input\":[\"contract_version\",\"intel_item_id\",\"provenance_id\"],",
            "\"open_original_receipt\":[\"contract_version\",\"intel_item_id\",\"provenance_id\",\"status\"]},",
        "\"demo_fixture\":{\"dataset_id\":\"demo-v1\",\"item_count\":3,\"fingerprint\":\"__DEMO_HASH__\"},",
            "\"effect_kind\":[\"contract_probe\"],",
            "\"effect_status\":{\"all\":[\"pending\",\"delivered\",\"denied\",\"expired\",\"failed\"],",
            "\"initial\":\"pending\",\"terminal\":[\"delivered\",\"denied\",\"expired\",\"failed\"],",
            "\"same_terminal_report\":\"already_applied\",",
            "\"different_terminal_report\":\"conflict.effect_already_reported\"},",
            "\"report_result\":[\"applied\",\"already_applied\"],",
            "\"field_rules\":{\"ids\":\"ascii_opaque_token_1_to_128\",",
            "\"setup_option_id\":\"ascii_alphanumeric_underscore_hyphen_colon_dot_1_to_64\",",
            "\"time_input\":\"rfc3339_utc_t_or_T_z_or_Z_or_plus_00_announced_leap_second\",",
            "\"time\":\"canonical_rfc3339_utc_uppercase_z_max_64\",",
            "\"missing\":\"explicit_null\"},",
            "\"configuration_rules\":{",
            "\"validator_version\":\"__VALIDATOR_VERSION__\",",
            "\"track_count\":{\"min\":__MIN_TRACKS__,\"max\":__MAX_TRACKS__},",
            "\"track_name_unicode_scalars\":{\"min\":1,\"max\":__MAX_TRACK_NAME_SCALARS__},",
            "\"source_count\":{\"min\":__MIN_SOURCES__,\"max\":__MAX_SOURCES__},",
            "\"source_identifier_bytes\":{\"min\":1,\"max\":__MAX_SOURCE_BYTES__},",
            "\"refresh_minutes\":{\"min\":__MIN_REFRESH__,\"max\":__MAX_REFRESH__},",
            "\"percent\":{\"min\":0,\"max\":__MAX_PERCENT__},",
            "\"high_trust_minimum\":__HIGH_TRUST__,",
            "\"notification_cap\":{\"min\":__MIN_CAP__,\"max\":__MAX_CAP__},",
            "\"expression\":{\"max_bytes\":__MAX_EXPR_BYTES__,\"max_terms\":__MAX_EXPR_TERMS__,\"max_term_unicode_scalars\":__MAX_TERM_SCALARS__,\"max_depth\":__MAX_EXPR_DEPTH__},",
            "\"blocking_codes\":[\"__BLOCKING_CODES__\"],",
            "\"narrowing_codes\":[\"__NARROWING_CODES__\"],",
            "\"hash\":\"sha256_lowercase_hex\",\"receipt\":\"base64url_no_pad_32_bytes_ttl___RECEIPT_TTL__ms_capacity___RECEIPT_CAPACITY__\"},",
            "\"sync_rules\":{\"kind\":\"rss_atom_sync\",\"required_source_kinds\":[\"rss_atom\"],\"task_id\":\"task_colon_24_lower_hex\",\"sync_run_id\":\"run_colon_24_lower_hex\",\"legacy_result_ref\":\"explicit_null\",\"result_cursor\":\"run_bound_existing_boundary\",\"foreground_budget_ms\":30000,\"task_states\":[\"queued\",\"running\",\"retry_wait\",\"succeeded\",\"partially_succeeded\",\"failed\",\"cancelled\"],\"result_outcomes\":[\"succeeded_with_results\",\"succeeded_zero_results\",\"partially_succeeded\",\"failed\"]},",
            "\"intelligence_value_rules\":{\"rule_version\":\"__RULE_VERSION__\",\"weights\":{\"track\":__TRACK_WEIGHT__,\"source_trust\":__SOURCE_TRUST_WEIGHT__,\"freshness\":__FRESHNESS_WEIGHT__,\"technical_impact\":__TECHNICAL_IMPACT_WEIGHT__,\"user_rule\":__USER_RULE_WEIGHT__},\"importance\":{\"low\":[0,__IMPORTANCE_LOW_MAX__],\"medium\":[__IMPORTANCE_MEDIUM_MIN__,__IMPORTANCE_MEDIUM_MAX__],\"high\":[__IMPORTANCE_HIGH_MIN__,__IMPORTANCE_HIGH_MAX__]},\"dispositions\":[\"high_value\",\"ordinary_candidate\"],\"threshold_field\":\"alert_threshold\",\"ai_status\":\"unavailable\"},",
            "\"intel_feed_rules\":{\"streams\":[\"high_value\",\"ordinary_candidate\"],\"time_windows\":[\"all_time\",\"last_24h\",\"last_7d\",\"last_30d\"],\"sort\":\"score_desc_item_id_asc\",\"limit\":{\"default\":30,\"max\":100},\"track_filter_max\":32,\"source_filter_max\":64,\"cursor_max_bytes\":1024,\"excerpt_max_chars\":280,\"data_origin\":\"real\",\"source_kind\":\"rss_atom\"},",
            "\"intel_detail_rules\":{\"provenance_max\":64,\"summary_max_chars\":16384,\"text_max_chars\":2048,\"ai_status\":\"unavailable\",\"open_input\":\"stable_item_and_provenance_ids_only\"}}"
    )
    .replace("__DEMO_HASH__", &demo_fingerprint)
    .replace("__VALIDATOR_VERSION__", VALIDATOR_VERSION)
    .replace("__RULE_VERSION__", INTELLIGENCE_VALUE_RULE_VERSION)
    .replace("__TRACK_WEIGHT__", &TRACK_WEIGHT.to_string())
    .replace("__SOURCE_TRUST_WEIGHT__", &SOURCE_TRUST_WEIGHT.to_string())
    .replace("__FRESHNESS_WEIGHT__", &FRESHNESS_WEIGHT.to_string())
    .replace(
        "__TECHNICAL_IMPACT_WEIGHT__",
        &TECHNICAL_IMPACT_WEIGHT.to_string(),
    )
    .replace("__USER_RULE_WEIGHT__", &USER_RULE_WEIGHT.to_string())
    .replace("__IMPORTANCE_LOW_MAX__", &IMPORTANCE_LOW_MAX.to_string())
    .replace(
        "__IMPORTANCE_MEDIUM_MIN__",
        &IMPORTANCE_MEDIUM_MIN.to_string(),
    )
    .replace(
        "__IMPORTANCE_MEDIUM_MAX__",
        &IMPORTANCE_MEDIUM_MAX.to_string(),
    )
    .replace("__IMPORTANCE_HIGH_MIN__", &IMPORTANCE_HIGH_MIN.to_string())
    .replace("__IMPORTANCE_HIGH_MAX__", &IMPORTANCE_HIGH_MAX.to_string())
    .replace("__MIN_TRACKS__", &MIN_TRACKS.to_string())
    .replace("__MAX_TRACKS__", &MAX_TRACKS.to_string())
    .replace("__MAX_TRACK_NAME_SCALARS__", &MAX_TRACK_NAME_SCALARS.to_string())
    .replace("__MIN_SOURCES__", &MIN_SOURCE_PREFERENCES.to_string())
    .replace("__MAX_SOURCES__", &MAX_SOURCE_PREFERENCES.to_string())
    .replace("__MAX_SOURCE_BYTES__", &MAX_SOURCE_IDENTIFIER_BYTES.to_string())
    .replace("__MIN_REFRESH__", &MIN_REFRESH_MINUTES.to_string())
    .replace("__MAX_REFRESH__", &MAX_REFRESH_MINUTES.to_string())
    .replace("__MAX_PERCENT__", &MAX_PERCENT.to_string())
    .replace("__HIGH_TRUST__", &HIGH_TRUST_THRESHOLD.to_string())
    .replace("__MIN_CAP__", &MIN_NOTIFICATION_CAP.to_string())
    .replace("__MAX_CAP__", &MAX_NOTIFICATION_CAP.to_string())
    .replace("__MAX_EXPR_BYTES__", &MAX_EXPRESSION_BYTES.to_string())
    .replace("__MAX_EXPR_TERMS__", &MAX_EXPRESSION_TERMS.to_string())
    .replace("__MAX_TERM_SCALARS__", &MAX_TERM_SCALARS.to_string())
    .replace("__MAX_EXPR_DEPTH__", &MAX_EXPRESSION_DEPTH.to_string())
    .replace(
        "__BLOCKING_CODES__",
        &BlockingCodeV1::ALL.map(BlockingCodeV1::as_str).join("\",\""),
    )
    .replace(
        "__NARROWING_CODES__",
        &NarrowingRiskCodeV1::ALL
            .map(NarrowingRiskCodeV1::as_str)
            .join("\",\""),
    )
    .replace("__RECEIPT_TTL__", &RECEIPT_TTL_MS.to_string())
    .replace("__RECEIPT_CAPACITY__", &MAX_RECEIPTS.to_string())
}

#[must_use]
pub fn error_codes_json() -> String {
    let categories = ErrorCategory::ALL.map(ErrorCategory::as_str).join("\",\"");
    let retryability = Retryability::ALL.map(Retryability::as_str).join("\",\"");
    let codes = ErrorCode::ALL.map(ErrorCode::as_str).join("\",\"");
    format!(
        "{{\"generator\":\"radar-core-contracts-v1\",\"contract_version\":1,\"categories\":[\"{categories}\"],\"retryability\":[\"{retryability}\"],\"codes\":[\"{codes}\"]}}"
    )
}
