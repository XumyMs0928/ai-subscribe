use radar_core::contracts::dto::configuration_validation::{
    AttentionConfigurationV1, AttentionTrackV1, NotificationFrequencyV1, QuietHoursV1,
    SourcePreferenceV1,
};
use radar_core::domain::rules::configuration_validation::configuration_hash;
use radar_core::domain::rules::intelligence_value::{
    EvaluationError, INTELLIGENCE_VALUE_RULE_VERSION, ImportanceV1, IntelligenceValueContext,
    RuleEvaluationV1, StreamDispositionV1, evaluate_intelligence_value,
};

const HOUR_MS: u64 = 3_600_000;
const EVALUATED_AT_MS: u64 = 1_767_268_800_000; // 2026-01-01T12:00:00Z

fn configuration() -> AttentionConfigurationV1 {
    AttentionConfigurationV1 {
        contract_version: 1,
        tracks: vec![AttentionTrackV1 {
            id: "foundation_models".into(),
            name: "基础模型".into(),
            enabled: true,
        }],
        include_expression: "release".into(),
        exclude_expression: "abandoned".into(),
        source_preferences: vec![SourcePreferenceV1 {
            source_kind: "rss".into(),
            identifier: "https://example.com/feed.xml".into(),
            enabled: true,
            trust: 100,
        }],
        refresh_enabled: true,
        refresh_interval_minutes: 60,
        minimum_trust: 40,
        maximum_trust: 100,
        alert_threshold: 80,
        quiet_hours: QuietHoursV1 {
            enabled: false,
            start: "22:00".into(),
            end: "07:00".into(),
        },
        notification_frequency: NotificationFrequencyV1 {
            enabled: false,
            max_per_24h: None,
        },
        active_from: None,
        active_until: None,
    }
}

fn context() -> IntelligenceValueContext {
    IntelligenceValueContext {
        fact_revision: 3,
        configuration_revision: 7,
        configuration_hash: configuration_hash(&configuration()),
        source_kind: "rss_atom".into(),
        source_identifier: "https://example.com/feed.xml".into(),
        publisher: "example.com".into(),
        original_url: "https://example.com/releases/model-v2".into(),
        title: "Foundation model release improves reasoning benchmark".into(),
        source_summary: Some("New capability and context window".into()),
        published_at: Some("2026-01-01T11:00:00Z".into()),
        collected_at: "2026-01-01T12:00:00Z".into(),
        evaluated_at_ms: EVALUATED_AT_MS,
    }
}

fn evaluate(
    configuration: &AttentionConfigurationV1,
    context: &IntelligenceValueContext,
) -> Result<RuleEvaluationV1, EvaluationError> {
    let mut context = context.clone();
    context.configuration_hash = configuration_hash(configuration);
    evaluate_intelligence_value(configuration, &context)
}

#[test]
fn v1_rule_is_deterministic_explainable_and_high_value_without_ai() {
    let first = evaluate(&configuration(), &context()).expect("evaluation");
    let second = evaluate(&configuration(), &context()).expect("replay");

    assert_eq!(first, second);
    assert_eq!(first.rule_version, INTELLIGENCE_VALUE_RULE_VERSION);
    assert_eq!(first.score, 100);
    assert_eq!(first.importance, ImportanceV1::High);
    assert_eq!(first.disposition, StreamDispositionV1::HighValue);
    assert_eq!(first.matched_track_ids, ["foundation_models"]);
    assert_eq!(first.factors.len(), 5);
    assert!(first.filter_reasons.is_empty());
    assert_eq!(first.ai_status.as_str(), "unavailable");
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/fixtures/golden/rss-intelligence-rule-v1.json"
    ))
    .expect("golden fixture");
    assert_eq!(
        serde_json::to_value(first).expect("rule JSON"),
        golden["high_value"]
    );
}

#[test]
fn hard_gates_keep_high_scoring_items_as_explainable_ordinary_candidates() {
    let mut excluded = context();
    excluded.title.push_str(" abandoned");
    let result = evaluate(&configuration(), &excluded).expect("evaluation");
    assert_eq!(result.disposition, StreamDispositionV1::OrdinaryCandidate);
    assert!(
        result
            .filter_reasons
            .iter()
            .any(|reason| reason.code == "exclude_expression_matched")
    );

    let mut missing_published = context();
    missing_published.published_at = None;
    let result = evaluate(&configuration(), &missing_published).expect("evaluation");
    assert_eq!(result.disposition, StreamDispositionV1::HighValue);
    assert!(
        result.factors[2]
            .reason_codes
            .contains(&"freshness.collected_at_fallback".to_owned())
    );
    assert!(
        result.factors[2]
            .reason_codes
            .contains(&"freshness.within_24h".to_owned())
    );
}

#[test]
fn threshold_and_importance_boundaries_are_inclusive_and_independent() {
    let mut config = configuration();
    let mut value_context = context();
    config.include_expression.clear();
    value_context.source_summary = None;
    value_context.title = "Foundation model update".into();

    config.alert_threshold = 80;
    let at_threshold = evaluate(&config, &value_context).expect("at threshold");
    assert_eq!(at_threshold.score, 80);
    assert_eq!(at_threshold.importance, ImportanceV1::High);
    assert_eq!(at_threshold.disposition, StreamDispositionV1::HighValue);

    config.alert_threshold = 79;
    let above_threshold = evaluate(&config, &value_context).expect("above threshold");
    assert_eq!(above_threshold.disposition, StreamDispositionV1::HighValue);

    config.alert_threshold = 81;
    let below = evaluate(&config, &value_context).expect("below threshold");
    assert_eq!(below.importance, ImportanceV1::High);
    assert_eq!(below.disposition, StreamDispositionV1::OrdinaryCandidate);
    assert_eq!(
        below.filter_reasons.last().expect("reason").code,
        "score_below_threshold"
    );
}

#[test]
fn importance_boundaries_49_50_79_80_are_exact() {
    let cases = [
        (76, false, 49, ImportanceV1::Low),
        (80, false, 50, ImportanceV1::Medium),
        (96, true, 79, ImportanceV1::Medium),
        (100, true, 80, ImportanceV1::High),
    ];
    for (trust, track_enabled, expected_score, expected_importance) in cases {
        let mut config = configuration();
        config.include_expression.clear();
        config.alert_threshold = 0;
        config.source_preferences[0].trust = trust;
        config.tracks[0].enabled = track_enabled;
        let mut input = context();
        input.title = "Foundation model update".to_owned();
        input.source_summary = None;
        let result = evaluate(&config, &input).expect("boundary");
        assert_eq!(result.score, expected_score);
        assert_eq!(result.importance, expected_importance);
    }
}

#[test]
fn freshness_boundaries_and_future_time_are_stable() {
    let cases = [
        (24 * HOUR_MS, 20, "freshness.within_24h"),
        (24 * HOUR_MS + 1, 15, "freshness.within_7d"),
        (7 * 24 * HOUR_MS + 1, 8, "freshness.within_30d"),
        (30 * 24 * HOUR_MS + 1, 0, "freshness.older_than_30d"),
    ];
    for (age, expected_points, reason) in cases {
        let mut input = context();
        input.published_at = Some(
            match age {
                86_400_000 => "2025-12-31T12:00:00Z",
                86_400_001 => "2025-12-31T11:59:59.999Z",
                604_800_001 => "2025-12-25T11:59:59.999Z",
                2_592_000_001 => "2025-12-02T11:59:59.999Z",
                _ => panic!("unsupported boundary age"),
            }
            .to_owned(),
        );
        let result = evaluate(&configuration(), &input).expect("freshness");
        assert_eq!(result.factors[2].points, expected_points);
        assert_eq!(result.factors[2].reason_codes[0], reason);
    }

    let mut future = context();
    future.published_at = Some("2026-01-01T13:00:00Z".to_owned());
    let result = evaluate(&configuration(), &future).expect("future");
    assert_eq!(result.factors[2].points, 20);
    assert_eq!(
        result.factors[2].reason_codes[0],
        "freshness.future_timestamp"
    );
}

#[test]
fn source_alias_custom_tracks_and_ascii_boundaries_are_explicit() {
    let mut config = configuration();
    config.tracks = vec![AttentionTrackV1 {
        id: "tooling".into(),
        name: "Tooling".into(),
        enabled: true,
    }];
    config.include_expression.clear();
    let mut input = context();
    input.title = "Tooling release".into();
    input.source_summary = Some("A costume refresh".into());
    let result = evaluate(&config, &input).expect("custom track");
    assert_eq!(result.matched_track_ids, ["tooling"]);
    assert!(
        !result.factors[3]
            .reason_codes
            .contains(&"technical_impact.cost".to_owned())
    );
    assert!(
        result.factors[3]
            .reason_codes
            .contains(&"technical_impact.technical_selection".to_owned())
    );
}

#[test]
fn technical_impact_categories_are_complete_ordered_and_ai_independent() {
    let cases = [
        ("multimodal capability", "technical_impact.model_capability"),
        (
            "breaking change sdk",
            "technical_impact.development_framework",
        ),
        ("serving runtime", "technical_impact.deployment"),
        ("token price", "technical_impact.cost"),
        ("cve patch", "technical_impact.security"),
        ("ga lts migration", "technical_impact.technical_selection"),
    ];
    for (summary, expected) in cases {
        let mut config = configuration();
        config.include_expression.clear();
        let mut input = context();
        input.title = "Foundation model update".to_owned();
        input.source_summary = Some(summary.to_owned());
        let result = evaluate(&config, &input).expect("category");
        assert!(
            result.factors[3]
                .reason_codes
                .contains(&expected.to_owned())
        );
        assert_eq!(result.factors[3].points, 20);
        assert_eq!(result.ai_status.as_str(), "unavailable");
    }
}

#[test]
fn disabled_untrusted_and_inactive_sources_are_explicit_hard_gates() {
    let mut disabled = configuration();
    disabled.source_preferences[0].enabled = false;
    let result = evaluate(&disabled, &context()).expect("disabled");
    assert_eq!(result.factors[1].points, 0);
    assert!(
        result
            .filter_reasons
            .iter()
            .any(|reason| reason.code == "source_disabled")
    );

    let mut untrusted = configuration();
    untrusted.source_preferences[0].trust = 39;
    let result = evaluate(&untrusted, &context()).expect("untrusted");
    assert!(
        result
            .filter_reasons
            .iter()
            .any(|reason| reason.code == "trust_outside_range")
    );

    let mut above_maximum = configuration();
    above_maximum.maximum_trust = 80;
    let result = evaluate(&above_maximum, &context()).expect("above maximum");
    let reason = result
        .filter_reasons
        .iter()
        .find(|reason| reason.code == "trust_outside_range")
        .expect("maximum trust reason");
    assert_eq!(reason.actual, Some(100));
    assert_eq!(reason.threshold, Some(80));

    let mut inactive = configuration();
    inactive.active_from = Some("2026-01-02T00:00:00Z".to_owned());
    inactive.active_until = Some("2026-01-03T00:00:00Z".to_owned());
    let result = evaluate(&inactive, &context()).expect("inactive");
    assert!(
        result
            .filter_reasons
            .iter()
            .any(|reason| reason.code == "outside_active_window")
    );
}

#[test]
fn unicode_include_and_exclude_use_the_shared_expression_semantics() {
    let mut config = configuration();
    config.include_expression = "\"多模态\"".to_owned();
    config.exclude_expression.clear();
    let mut input = context();
    input.title = "基础模型 多模态 发布".to_owned();
    let included = evaluate(&config, &input).expect("unicode include");
    assert!(
        included
            .filter_reasons
            .iter()
            .all(|reason| reason.code != "include_expression_not_matched")
    );

    config.exclude_expression = "\"多模态\"".to_owned();
    let excluded = evaluate(&config, &input).expect("unicode exclude");
    assert!(
        excluded
            .filter_reasons
            .iter()
            .any(|reason| reason.code == "exclude_expression_matched")
    );
}

#[test]
fn evaluator_rejects_unvalidated_configuration_and_context_identity() {
    let mut invalid_trust = configuration();
    invalid_trust.source_preferences[0].trust = 101;
    assert_eq!(
        evaluate(&invalid_trust, &context()),
        Err(EvaluationError::InvalidConfiguration)
    );

    let mut invalid_expression = configuration();
    invalid_expression.include_expression = "release AND (".to_owned();
    assert_eq!(
        evaluate(&invalid_expression, &context()),
        Err(EvaluationError::InvalidConfiguration)
    );

    let mut invalid_context = context();
    invalid_context.evaluated_at_ms = 0;
    assert_eq!(
        evaluate(&configuration(), &invalid_context),
        Err(EvaluationError::InvalidConfiguration)
    );
}
