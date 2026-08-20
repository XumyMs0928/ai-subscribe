use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const APPROVED_TAURI_COMMANDS: &str =
    include_str!("../../../contracts/fixtures/tauri/approved-commands-v1.txt");

use radar_core::application::demo::{DemoStore, validate_demo_fixture};
use radar_core::application::health_check;
use radar_core::contracts::dto::configuration_validation::{
    AttentionConfigurationV1, AttentionTrackV1, ConfigurationCandidateContext,
    ConfigurationValidationResultV1, NotificationFrequencyV1, QuietHoursV1, SourcePreferenceV1,
};
use radar_core::contracts::dto::intel_feed::{IntelFeedItemV1, QueryIntelFeedInputV1};
use radar_core::contracts::dto::source::{SaveSourceInputV1, SourceViewV1};
use radar_core::contracts::effects::{EffectLedger, EffectStatus, PlatformEffect, ReportResult};
use radar_core::contracts::manifest::{contract_manifest_json, error_codes_json};
use radar_core::contracts::secrets::SecretLeaseInput;
use radar_core::domain::rules::configuration_validation::{
    assess_configuration, configuration_hash,
};
use radar_core::domain::rules::intelligence_value::{
    IntelligenceValueContext, evaluate_intelligence_value,
};
use radar_core::infrastructure::sources::rss_atom::parse_feed;
use radar_ffi::error::map_unknown;

pub fn run_from_args<I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<_> = args.into_iter().collect();
    if args.as_slice() != ["contracts"] {
        return Err("usage: cargo run -p xtask -- contracts".to_owned());
    }

    let root = workspace_root()?;
    check_equal(
        &root.join("contracts/schemas/contract-manifest-v1.json"),
        &contract_manifest_json(),
    )?;
    check_equal(
        &root.join("contracts/snapshots/error-codes-v1.json"),
        &error_codes_json(),
    )?;
    check_required_fixtures(&root)?;
    check_boundaries(&root)?;
    println!("contracts: PASS (v1 schemas, snapshots, fixtures, boundaries)");
    Ok(())
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "unable to resolve workspace root".to_owned())
}

fn check_equal(path: &Path, expected: &str) -> Result<(), String> {
    let actual =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if compact_json(&actual)? != compact_json(expected)? {
        return Err(format!("contract drift: {}", path.display()));
    }
    Ok(())
}

fn compact_json(value: &str) -> Result<String, String> {
    let mut parser = JsonParser::new(value);
    let mut compact = String::with_capacity(value.len());
    parser.parse_value(&mut compact)?;
    parser.skip_whitespace();
    if parser.position != parser.bytes.len() || !compact.starts_with('{') {
        return Err("invalid JSON: expected one object envelope".to_owned());
    }
    Ok(compact)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> JsonParser<'a> {
    fn new(value: &'a str) -> Self {
        Self {
            bytes: value.as_bytes(),
            position: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn parse_value(&mut self, output: &mut String) -> Result<(), String> {
        self.skip_whitespace();
        match self.bytes.get(self.position).copied() {
            Some(b'{') => self.parse_collection(output, b'{', b'}', true),
            Some(b'[') => self.parse_collection(output, b'[', b']', false),
            Some(b'"') => self.parse_string(output),
            Some(b't') => self.parse_literal(output, b"true"),
            Some(b'f') => self.parse_literal(output, b"false"),
            Some(b'n') => self.parse_literal(output, b"null"),
            Some(b'-' | b'0'..=b'9') => self.parse_number(output),
            _ => Err("invalid JSON: malformed value".to_owned()),
        }
    }

    fn parse_collection(
        &mut self,
        output: &mut String,
        open: u8,
        close: u8,
        object: bool,
    ) -> Result<(), String> {
        self.position += 1;
        output.push(char::from(open));
        self.skip_whitespace();
        if self.bytes.get(self.position) == Some(&close) {
            self.position += 1;
            output.push(char::from(close));
            return Ok(());
        }
        loop {
            if object {
                self.parse_string(output)?;
                self.skip_whitespace();
                if self.bytes.get(self.position) != Some(&b':') {
                    return Err("invalid JSON: object key is missing a colon".to_owned());
                }
                self.position += 1;
                output.push(':');
            }
            self.parse_value(output)?;
            self.skip_whitespace();
            match self.bytes.get(self.position).copied() {
                Some(value) if value == close => {
                    self.position += 1;
                    output.push(char::from(close));
                    return Ok(());
                }
                Some(b',') => {
                    self.position += 1;
                    output.push(',');
                    self.skip_whitespace();
                }
                _ => return Err("invalid JSON: malformed collection".to_owned()),
            }
        }
    }

    fn parse_string(&mut self, output: &mut String) -> Result<(), String> {
        if self.bytes.get(self.position) != Some(&b'"') {
            return Err("invalid JSON: object key must be a string".to_owned());
        }
        let start = self.position;
        self.position += 1;
        while let Some(byte) = self.bytes.get(self.position).copied() {
            match byte {
                b'"' => {
                    self.position += 1;
                    output.push_str(
                        std::str::from_utf8(&self.bytes[start..self.position])
                            .map_err(|_| "invalid JSON: invalid UTF-8".to_owned())?,
                    );
                    return Ok(());
                }
                b'\\' => {
                    self.position += 1;
                    match self.bytes.get(self.position).copied() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.position += 1;
                        }
                        Some(b'u') => {
                            self.position += 1;
                            for _ in 0..4 {
                                if !self
                                    .bytes
                                    .get(self.position)
                                    .is_some_and(u8::is_ascii_hexdigit)
                                {
                                    return Err("invalid JSON: malformed unicode escape".to_owned());
                                }
                                self.position += 1;
                            }
                        }
                        _ => return Err("invalid JSON: malformed escape".to_owned()),
                    }
                }
                0..=0x1f => return Err("invalid JSON: control byte in string".to_owned()),
                _ => self.position += 1,
            }
        }
        Err("invalid JSON: unterminated string".to_owned())
    }

    fn parse_literal(&mut self, output: &mut String, literal: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.position..self.position + literal.len()) != Some(literal) {
            return Err("invalid JSON: malformed literal".to_owned());
        }
        self.position += literal.len();
        output.push_str(std::str::from_utf8(literal).expect("JSON literal is UTF-8"));
        Ok(())
    }

    fn parse_number(&mut self, output: &mut String) -> Result<(), String> {
        let start = self.position;
        if self.bytes.get(self.position) == Some(&b'-') {
            self.position += 1;
        }
        match self.bytes.get(self.position).copied() {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => {
                self.position += 1;
                while self
                    .bytes
                    .get(self.position)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.position += 1;
                }
            }
            _ => return Err("invalid JSON: malformed number".to_owned()),
        }
        if self.bytes.get(self.position) == Some(&b'.') {
            self.position += 1;
            let fraction = self.position;
            while self
                .bytes
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
            if self.position == fraction {
                return Err("invalid JSON: malformed number fraction".to_owned());
            }
        }
        if matches!(self.bytes.get(self.position), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.bytes.get(self.position), Some(b'+' | b'-')) {
                self.position += 1;
            }
            let exponent = self.position;
            while self
                .bytes
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
            if self.position == exponent {
                return Err("invalid JSON: malformed number exponent".to_owned());
            }
        }
        output.push_str(
            std::str::from_utf8(&self.bytes[start..self.position])
                .map_err(|_| "invalid JSON: invalid UTF-8".to_owned())?,
        );
        Ok(())
    }
}

#[allow(clippy::too_many_lines)] // The closed fixture inventory and its structural validators remain auditable in one gate.
fn check_required_fixtures(root: &Path) -> Result<(), String> {
    let fixtures = [
        ("health_success_v1.json", HEALTH_FIXTURE),
        ("validation_failure_v1.json", VALIDATION_FIXTURE),
        ("internal_error_v1.json", INTERNAL_FIXTURE),
        ("effect_report_v1.json", EFFECT_FIXTURE),
        ("secret_lease_v1.json", SECRET_FIXTURE),
    ];
    for (name, expected) in fixtures {
        let path = root.join("contracts/fixtures/golden").join(name);
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("missing fixture {}: {error}", path.display()))?;
        if compact_json(&contents)? != compact_json(expected)? {
            return Err(format!("fixture drift: {}", path.display()));
        }
        for forbidden in ["secret_value", "authorization", "api_key"] {
            if contents.to_ascii_lowercase().contains(forbidden) {
                return Err(format!(
                    "forbidden field `{forbidden}` in {}",
                    path.display()
                ));
            }
        }
    }

    for relative in [
        "contracts/fixtures/golden/configuration_validation_v1.json",
        "contracts/fixtures/golden/setup_progress_v1.json",
        "contracts/fixtures/golden/source_view_v1.json",
        "contracts/fixtures/intel-feed/phase1-v1.json",
        "contracts/fixtures/configuration-validation/blocking/cases-v1.json",
        "contracts/fixtures/configuration-validation/narrowing/cases-v1.json",
        "contracts/fixtures/configuration-validation/valid/basic-v1.json",
    ] {
        let path = root.join(relative);
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("missing fixture {}: {error}", path.display()))?;
        compact_json(&contents)?;
        for forbidden in ["secret_value", "authorization", "api_key"] {
            if contents.to_ascii_lowercase().contains(forbidden) {
                return Err(format!(
                    "forbidden field `{forbidden}` in {}",
                    path.display()
                ));
            }
        }
    }

    let feed_fixture_path = root.join("contracts/fixtures/intel-feed/phase1-v1.json");
    let feed_fixture: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&feed_fixture_path)
            .map_err(|error| format!("{}: {error}", feed_fixture_path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", feed_fixture_path.display()))?;
    let root_keys = feed_fixture
        .as_object()
        .ok_or_else(|| "intel-feed fixture root must be an object".to_owned())?
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if root_keys != BTreeSet::from(["contract_version", "items", "query", "rules"])
        || feed_fixture["contract_version"] != 1
    {
        return Err("intel-feed fixture root contract drift".to_owned());
    }
    serde_json::from_value::<QueryIntelFeedInputV1>(feed_fixture["query"].clone())
        .map_err(|error| format!("intel-feed fixture query: {error}"))?;
    for key in ["high_value", "ordinary_candidate"] {
        serde_json::from_value::<IntelFeedItemV1>(feed_fixture["items"][key].clone())
            .map_err(|error| format!("intel-feed fixture item {key}: {error}"))?;
    }
    if feed_fixture["rules"]
        != serde_json::json!({
            "cross_dimension": "and",
            "within_dimension": "or",
            "sort": "score_desc_item_id_asc",
            "cursor": "projection_and_filter_bound",
            "excerpt_max_chars": 280
        })
    {
        return Err("intel-feed fixture rule contract drift".to_owned());
    }

    let source_golden_path = root.join("contracts/fixtures/golden/source_view_v1.json");
    let source_golden: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&source_golden_path)
            .map_err(|error| format!("{}: {error}", source_golden_path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", source_golden_path.display()))?;
    serde_json::from_value::<SaveSourceInputV1>(source_golden["input"].clone())
        .map_err(|error| format!("source fixture input: {error}"))?;
    serde_json::from_value::<SourceViewV1>(source_golden["expected"].clone())
        .map_err(|error| format!("source fixture expected: {error}"))?;

    for (relative, should_parse) in [
        ("contracts/fixtures/rss-atom/rss2-v1.xml", true),
        ("contracts/fixtures/rss-atom/rss2-updated.xml", true),
        ("contracts/fixtures/rss-atom/rss2-invalid-time.xml", true),
        ("contracts/fixtures/rss-atom/atom-v1.xml", true),
        ("contracts/fixtures/rss-atom/malformed.xml", false),
    ] {
        let path = root.join(relative);
        let bytes = fs::read(&path)
            .map_err(|error| format!("missing fixture {}: {error}", path.display()))?;
        if parse_feed(&bytes).is_ok() != should_parse {
            return Err(format!(
                "RSS/Atom fixture behavior drift: {}",
                path.display()
            ));
        }
    }
    check_rss_transport_fixture(root)?;
    check_intelligence_value_fixture(root)?;

    let demo_path = root.join("contracts/fixtures/demo/manifest-v1.json");
    let demo_contents = fs::read_to_string(&demo_path)
        .map_err(|error| format!("missing demo fixture {}: {error}", demo_path.display()))?;
    compact_json(&demo_contents)?;
    validate_demo_fixture(&demo_contents)
        .map_err(|_| format!("demo fixture contract drift: {}", demo_path.display()))?;
    for forbidden in ["secret_value", "authorization", "api_key"] {
        if demo_contents.to_ascii_lowercase().contains(forbidden) {
            return Err(format!(
                "forbidden field `{forbidden}` in {}",
                demo_path.display()
            ));
        }
    }
    execute_fixture_behaviors(root)?;
    Ok(())
}

fn check_intelligence_value_fixture(root: &Path) -> Result<(), String> {
    let path = root.join("contracts/fixtures/golden/rss-intelligence-rule-v1.json");
    let fixture: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", path.display()))?;
    let base_configuration = intelligence_configuration();
    let base_context = intelligence_context(&base_configuration);
    let high_value = evaluate_intelligence_value(&base_configuration, &base_context)
        .map_err(|_| "intelligence fixture base evaluation failed".to_owned())?;
    if serde_json::to_value(high_value).map_err(|error| error.to_string())? != fixture["high_value"]
    {
        return Err(format!("rule fixture drift: {}", path.display()));
    }
    let scenarios = fixture["scenario_expectations"]
        .as_array()
        .ok_or_else(|| "rule fixture scenarios must be an array".to_owned())?;
    for scenario in scenarios {
        let name = scenario["name"]
            .as_str()
            .ok_or_else(|| "rule fixture scenario missing name".to_owned())?;
        let mut configuration = base_configuration.clone();
        let mut context = base_context.clone();
        match name {
            "score_below_threshold" => {
                configuration.include_expression.clear();
                configuration.alert_threshold = 81;
                "Foundation model update".clone_into(&mut context.title);
                context.source_summary = None;
            }
            "exclude" => context.title.push_str(" abandoned"),
            "disabled_source" => configuration.source_preferences[0].enabled = false,
            "untrusted_source" => configuration.source_preferences[0].trust = 39,
            "no_track" => configuration.tracks[0].enabled = false,
            "technical_no_evidence" => {
                configuration.include_expression.clear();
                "Foundation model update".clone_into(&mut context.title);
                context.source_summary = None;
            }
            "ai_unavailable" => {}
            _ => return Err(format!("unknown rule fixture scenario: {name}")),
        }
        context.configuration_hash = configuration_hash(&configuration);
        let evaluation = evaluate_intelligence_value(&configuration, &context)
            .map_err(|_| format!("rule fixture scenario failed: {name}"))?;
        if let Some(expected) = scenario["disposition"].as_str()
            && evaluation.disposition.as_str() != expected
        {
            return Err(format!("rule fixture disposition drift: {name}"));
        }
        if let Some(expected) = scenario["reason"].as_str() {
            let found = evaluation
                .filter_reasons
                .iter()
                .any(|reason| reason.code == expected)
                || evaluation
                    .factors
                    .iter()
                    .flat_map(|factor| &factor.reason_codes)
                    .any(|reason| reason == expected);
            if !found {
                return Err(format!("rule fixture reason drift: {name}"));
            }
        }
        if let Some(expected) = scenario["technical_points"].as_u64()
            && u64::from(evaluation.factors[3].points) != expected
        {
            return Err(format!("rule fixture technical score drift: {name}"));
        }
        if let Some(expected) = scenario["ai_status"].as_str()
            && evaluation.ai_status.as_str() != expected
        {
            return Err(format!("rule fixture AI status drift: {name}"));
        }
    }
    Ok(())
}

fn intelligence_configuration() -> AttentionConfigurationV1 {
    AttentionConfigurationV1 {
        contract_version: 1,
        tracks: vec![AttentionTrackV1 {
            id: "foundation_models".to_owned(),
            name: "基础模型".to_owned(),
            enabled: true,
        }],
        include_expression: "release".to_owned(),
        exclude_expression: "abandoned".to_owned(),
        source_preferences: vec![SourcePreferenceV1 {
            source_kind: "rss".to_owned(),
            identifier: "https://example.com/feed.xml".to_owned(),
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
            start: "22:00".to_owned(),
            end: "07:00".to_owned(),
        },
        notification_frequency: NotificationFrequencyV1 {
            enabled: false,
            max_per_24h: None,
        },
        active_from: None,
        active_until: None,
    }
}

fn intelligence_context(configuration: &AttentionConfigurationV1) -> IntelligenceValueContext {
    IntelligenceValueContext {
        fact_revision: 3,
        configuration_revision: 7,
        configuration_hash: configuration_hash(configuration),
        source_kind: "rss_atom".to_owned(),
        source_identifier: "https://example.com/feed.xml".to_owned(),
        publisher: "example.com".to_owned(),
        original_url: "https://example.com/releases/model-v2".to_owned(),
        title: "Foundation model release improves reasoning benchmark".to_owned(),
        source_summary: Some("New capability and context window".to_owned()),
        published_at: Some("2026-01-01T11:00:00Z".to_owned()),
        collected_at: "2026-01-01T12:00:00Z".to_owned(),
        evaluated_at_ms: 1_767_268_800_000,
    }
}

fn check_rss_transport_fixture(root: &Path) -> Result<(), String> {
    let transport_path = root.join("contracts/fixtures/rss-atom/transport-cases-v1.json");
    let transport: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&transport_path)
            .map_err(|error| format!("{}: {error}", transport_path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", transport_path.display()))?;
    if transport["contract_version"] != 1 {
        return Err("RSS/Atom transport fixture version drift".to_owned());
    }
    let cases = transport["cases"]
        .as_array()
        .ok_or_else(|| "RSS/Atom transport cases must be an array".to_owned())?;
    let ids = cases
        .iter()
        .map(|case| {
            case["id"]
                .as_str()
                .ok_or_else(|| "RSS/Atom transport case missing id".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if ids.len() != cases.len() || ids.len() < 14 {
        return Err("RSS/Atom transport fixture coverage or identity drift".to_owned());
    }
    let allowed_outcomes = BTreeSet::from([
        "success",
        "success_with_missing_optional_fields",
        "success_with_time_omitted",
        "not_modified",
        "validation.source",
        "network.source",
        "rate_limited.source",
        "source_format.rss_atom",
    ]);
    for case in cases {
        let outcome = case["expected"]
            .as_str()
            .ok_or_else(|| "RSS/Atom transport case missing expected outcome".to_owned())?;
        if !allowed_outcomes.contains(outcome) {
            return Err(format!(
                "RSS/Atom transport case has non-contract outcome `{outcome}`"
            ));
        }
    }
    Ok(())
}

fn execute_fixture_behaviors(root: &Path) -> Result<(), String> {
    let health = health_check();
    if health.contract_version != 1 || health.status != "ok" || health.checked_at.is_some() {
        return Err("health_success_v1 behavior mismatch".to_owned());
    }

    let validation =
        PlatformEffect::new_contract_probe("", "probe:fixture", "2026-08-13T00:00:00Z")
            .expect_err("empty effect ID is invalid");
    if validation.code() != "validation.effect_id" {
        return Err("validation_failure_v1 behavior mismatch".to_owned());
    }

    let internal = map_unknown::<()>("private fixture fault")
        .expect_err("unknown fixture error must map to AppError");
    if internal.code() != "internal.unexpected" || internal.category().as_str() != "internal" {
        return Err("internal_error_v1 behavior mismatch".to_owned());
    }

    let effect = PlatformEffect::new_contract_probe(
        "effect:opaque-1",
        "probe:opaque-1",
        "2026-08-13T00:00:00Z",
    )
    .map_err(|error| format!("effect_report_v1 setup failed: {error:?}"))?;
    let mut ledger = EffectLedger::default();
    ledger
        .register(effect)
        .map_err(|error| format!("effect_report_v1 registration failed: {error:?}"))?;
    if ledger.report("effect:opaque-1", "probe:opaque-1", EffectStatus::Delivered)
        != Ok(ReportResult::Applied)
        || ledger.report("effect:opaque-1", "probe:opaque-1", EffectStatus::Delivered)
            != Ok(ReportResult::AlreadyApplied)
    {
        return Err("effect_report_v1 idempotent result mismatch".to_owned());
    }
    let conflict = ledger
        .report("effect:opaque-1", "probe:opaque-1", EffectStatus::Failed)
        .expect_err("different terminal report must conflict");
    if conflict.code() != "conflict.effect_already_reported" {
        return Err("effect_report_v1 conflict mismatch".to_owned());
    }

    let canary = b"runtime-fixture-canary";
    let mut observed = false;
    let mut lease = SecretLeaseInput::new("secret:test", canary.to_vec())
        .map_err(|error| format!("secret_lease_v1 setup failed: {error:?}"))?;
    lease
        .with_secret(|bytes| {
            observed = bytes == canary;
            Ok(())
        })
        .map_err(|error| format!("secret_lease_v1 first use failed: {error:?}"))?;
    if !observed
        || lease
            .with_secret(|_| Ok(()))
            .expect_err("second use must fail")
            .code()
            != "conflict.secret_lease_consumed"
    {
        return Err("secret_lease_v1 behavior mismatch".to_owned());
    }

    let configuration_fixture_path =
        root.join("contracts/fixtures/golden/configuration_validation_v1.json");
    let configuration_fixture: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&configuration_fixture_path)
            .map_err(|error| format!("{}: {error}", configuration_fixture_path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", configuration_fixture_path.display()))?;
    let configuration: AttentionConfigurationV1 =
        serde_json::from_value(configuration_fixture["input"].clone())
            .map_err(|error| format!("configuration fixture input: {error}"))?;
    let expected: ConfigurationValidationResultV1 =
        serde_json::from_value(configuration_fixture["expected"].clone())
            .map_err(|error| format!("configuration fixture expected: {error}"))?;
    let actual = assess_configuration(&configuration, &ConfigurationCandidateContext::default());
    if actual != expected {
        return Err("configuration_validation_v1 behavior mismatch".to_owned());
    }

    let setup_fixture_path = root.join("contracts/fixtures/golden/setup_progress_v1.json");
    let expected_setup: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&setup_fixture_path)
            .map_err(|error| format!("{}: {error}", setup_fixture_path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", setup_fixture_path.display()))?;
    let actual_setup = DemoStore::open_in_memory()
        .and_then(|store| store.get_setup_progress())
        .map_err(|error| format!("setup_progress_v1 execution failed: {error:?}"))?;
    let actual_setup = serde_json::to_value(actual_setup)
        .map_err(|error| format!("setup_progress_v1 serialization failed: {error}"))?;
    if actual_setup != expected_setup {
        return Err(format!(
            "setup_progress_v1 behavior mismatch: expected={expected_setup} actual={actual_setup}"
        ));
    }
    Ok(())
}

const HEALTH_FIXTURE: &str = r#"{"scenario":"health_success_v1","input":null,"expected":{"contract_version":1,"status":"ok","checked_at":null},"forbidden_fields":[]}"#;
const VALIDATION_FIXTURE: &str = r#"{"scenario":"validation_failure_v1","input":{"effect_id":""},"expected":{"contract_version":1,"code":"validation.effect_id","category":"validation"},"forbidden_fields":[]}"#;
const INTERNAL_FIXTURE: &str = r#"{"scenario":"internal_error_v1","input":{"fault":"opaque_test_fault"},"expected":{"contract_version":1,"code":"internal.unexpected","category":"internal"},"forbidden_fields":["private_error_text"]}"#;
const EFFECT_FIXTURE: &str = r#"{"scenario":"effect_report_v1","input":{"effect_id":"effect:opaque-1","first":"delivered","repeat":"delivered","conflict":"failed"},"expected":{"first":"applied","repeat":"already_applied","conflict":"conflict.effect_already_reported"},"forbidden_fields":[]}"#;
const SECRET_FIXTURE: &str = r#"{"scenario":"secret_lease_v1","input":{"secret_ref":"secret:test","runtime_provider":"in_memory_fake"},"expected":{"first_use":"available","second_use":"conflict.secret_lease_consumed","observable_canary_hits":0},"forbidden_fields":["secret_bytes","secret_plaintext"]}"#;
fn check_boundaries(root: &Path) -> Result<(), String> {
    check_windows_scope(root)?;
    for path in walk_files(root)? {
        check_file_boundary(root, &path)?;
    }
    Ok(())
}

fn check_file_boundary(root: &Path, path: &Path) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let relative_lower = relative.to_ascii_lowercase();
    let is_windows_source = relative_lower.starts_with("apps/windows/");
    let is_rust_integration_test = extension == "rs" && relative_lower.contains("/tests/");
    let is_frontend_test = matches!(extension.as_str(), "ts" | "tsx")
        && (relative_lower.contains("/test/") || relative_lower.contains(".test."));

    if is_windows_source
        && (relative_lower.contains("/fixtures/")
            || matches!(
                file_name.as_str(),
                "health_success_v1.json"
                    | "validation_failure_v1.json"
                    | "internal_error_v1.json"
                    | "effect_report_v1.json"
                    | "secret_lease_v1.json"
            ))
    {
        return Err(format!(
            "copied contract fixture is not allowed: {}",
            path.display()
        ));
    }
    if matches!(file_name.as_str(), ".env" | "id_rsa" | "id_ed25519")
        || (file_name.starts_with(".env.") && relative_lower != ".env.example")
    {
        return Err(format!("forbidden sensitive file: {}", path.display()));
    }
    if matches!(
        extension.as_str(),
        "db" | "sqlite" | "sqlite3" | "pem" | "key" | "pfx" | "p12" | "secret"
    ) {
        return Err(format!("forbidden project artifact: {}", path.display()));
    }
    if !is_scannable_extension(&extension) {
        return Ok(());
    }

    let contents =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let lower = contents.to_ascii_lowercase();
    let normalized_whitespace = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    check_forbidden_content(path, &lower, &normalized_whitespace)?;
    if is_windows_source && !is_rust_integration_test && !is_frontend_test {
        let production_lower = contents.to_ascii_lowercase();
        let production_whitespace = production_lower
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        check_windows_production_content(
            &relative_lower,
            &production_lower,
            &production_whitespace,
        )?;
    }
    Ok(())
}

fn is_scannable_extension(extension: &str) -> bool {
    extension.is_empty()
        || matches!(
            extension,
            "rs" | "toml"
                | "json"
                | "md"
                | "yaml"
                | "yml"
                | "txt"
                | "sql"
                | "cmd"
                | "ps1"
                | "js"
                | "jsx"
                | "mjs"
                | "mts"
                | "cts"
                | "ts"
                | "tsx"
                | "html"
                | "css"
        )
}

fn check_forbidden_content(
    path: &Path,
    lower: &str,
    normalized_whitespace: &str,
) -> Result<(), String> {
    let path_lower = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let forbidden_patterns = [
        (["create", "table"].join(" "), true),
        (["rus", "qlite"].concat(), false),
        (["authorization:", "bearer"].join(" "), false),
        (["client", "secret"].join("_"), false),
        (["private", "key"].join("_"), false),
    ];
    for (forbidden, normalize_whitespace) in forbidden_patterns {
        let approved_intel_feed_test_only = !normalize_whitespace
            && forbidden == ["rus", "qlite"].concat()
            && path_lower.ends_with("crates/radar-core/src/application/intel_feed.rs")
            && lower
                .split_once("#[cfg(test)]")
                .is_some_and(|(production, _)| !production.contains(&forbidden));
        let approved_demo_database_surface = (normalize_whitespace
            && (path_lower.ends_with("crates/radar-core/src/application/demo.rs")
                || path_lower.contains("/vendor/libsqlite3-sys/")))
            || (!normalize_whitespace
                && forbidden == ["rus", "qlite"].concat()
                && (path_lower.ends_with("crates/radar-core/src/application/demo.rs")
                    || path_lower.ends_with("crates/radar-core/src/application/setup.rs")
                    || path_lower.ends_with("crates/radar-core/src/application/configuration.rs")
                    || path_lower.ends_with("crates/radar-core/src/application/sources.rs")
                    || path_lower.ends_with(
                        "crates/radar-core/src/infrastructure/database/intel_repository.rs",
                    )
                    || path_lower.ends_with(
                        "crates/radar-core/src/infrastructure/database/association_repository.rs",
                    )
                    || path_lower.ends_with(
                        "crates/radar-core/src/infrastructure/database/rule_evaluation_repository.rs",
                    )
                    || path_lower.ends_with(
                        "crates/radar-core/src/infrastructure/database/intel_feed_repository.rs",
                    )
                    || path_lower.ends_with("crates/radar-core/cargo.toml")
                    || path_lower.ends_with("cargo.lock")
                    || path_lower.contains("/vendor/libsqlite3-sys/")))
            || approved_intel_feed_test_only;
        if approved_demo_database_surface {
            continue;
        }
        let searchable = if normalize_whitespace {
            normalized_whitespace
        } else {
            lower
        };
        if searchable.contains(&forbidden) {
            return Err(format!(
                "forbidden pattern `{forbidden}` in {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn check_windows_scope(root: &Path) -> Result<(), String> {
    if root.join("migrations").exists() {
        return Err("out-of-scope directory exists: migrations".to_owned());
    }
    for entry in fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        if name.to_string_lossy().eq_ignore_ascii_case("apps") && name != "apps" {
            return Err("out-of-scope platform directory casing: expected `apps`".to_owned());
        }
    }
    let apps = root.join("apps");
    if apps.exists() {
        for entry in fs::read_dir(&apps).map_err(|error| format!("{}: {error}", apps.display()))? {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("windows")
                || entry.file_name() != "windows"
            {
                return Err(format!(
                    "out-of-scope platform directory exists: apps/{}",
                    entry.file_name().to_string_lossy()
                ));
            }
        }
    }
    for forbidden in [
        "apps/windows/src/database",
        "apps/windows/src/stores",
        "apps/windows/src-tauri/migrations",
    ] {
        if root.join(forbidden).exists() {
            return Err(format!("out-of-scope Windows surface exists: {forbidden}"));
        }
    }
    for path in walk_files(root)? {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase();
        if relative.starts_with("apps/windows/") && !is_allowed_windows_shell_path(&relative) {
            return Err(format!("out-of-scope Windows surface exists: {relative}"));
        }
    }
    Ok(())
}

fn is_allowed_windows_shell_path(relative: &str) -> bool {
    const ROOT_FILES: &[&str] = &[
        "apps/windows/components.json",
        "apps/windows/.prettierignore",
        "apps/windows/eslint.config.js",
        "apps/windows/index.html",
        "apps/windows/package.json",
        "apps/windows/tsconfig.app.json",
        "apps/windows/tsconfig.app.tsbuildinfo",
        "apps/windows/tsconfig.json",
        "apps/windows/tsconfig.node.json",
        "apps/windows/tsconfig.node.tsbuildinfo",
        "apps/windows/vite.config.ts",
        "apps/windows/src/main.tsx",
        "apps/windows/src/lib/utils.ts",
        "apps/windows/src/lib/query-client.ts",
        "apps/windows/src-tauri/build.rs",
        "apps/windows/src-tauri/cargo.toml",
        "apps/windows/src-tauri/src/lib.rs",
        "apps/windows/src-tauri/src/main.rs",
        "apps/windows/src-tauri/tauri.conf.json",
    ];
    const PREFIXES: &[&str] = &[
        "apps/windows/src/app/providers/",
        "apps/windows/src/app/router/",
        "apps/windows/src/app/shell/",
        "apps/windows/src/features/demo-intelligence/",
        "apps/windows/src/features/intel-feed/",
        "apps/windows/src/features/configuration-validation/",
        "apps/windows/src/features/settings/",
        "apps/windows/src/features/setup-guide/",
        "apps/windows/src/features/sources/",
        "apps/windows/src/features/sync-results/",
        "apps/windows/src/components/ui/",
        "apps/windows/src/lib/desktop-api/",
        "apps/windows/src/styles/",
        "apps/windows/src/test/",
        "apps/windows/src-tauri/capabilities/",
        "apps/windows/src-tauri/icons/",
        "apps/windows/src-tauri/src/commands/",
        "apps/windows/src-tauri/src/platform/",
        "apps/windows/src-tauri/tests/",
    ];
    ROOT_FILES.contains(&relative) || PREFIXES.iter().any(|prefix| relative.starts_with(prefix))
}

fn check_windows_production_content(
    relative: &str,
    lower: &str,
    normalized_whitespace: &str,
) -> Result<(), String> {
    let desktop_transport = "apps/windows/src/lib/desktop-api/tauri-desktop-api.ts";
    let compact = lower
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if relative.starts_with("apps/windows/src/")
        && relative != desktop_transport
        && (lower.contains("@tauri-apps/api/core")
            || lower.contains("__tauri_internals__")
            || compact.contains("invoke(")
            || compact.contains("__tauri_internals__.invoke(")
            || compact.contains("invoke.call(")
            || compact.contains("__tauri_internals__[\"invoke\"]")
            || compact.contains("__tauri_internals__['invoke']"))
    {
        return Err(format!("raw Tauri invoke outside DesktopApi: {relative}"));
    }
    let imports_test_source = lower.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("import ") && (line.contains("/test/") || line.contains(".test"))
    });
    if relative.starts_with("apps/windows/src/") && imports_test_source {
        return Err(format!(
            "production import reaches test-only source: {relative}"
        ));
    }

    for forbidden in [
        "@tauri-apps/plugin-shell",
        "@tauri-apps/plugin-fs",
        "node:fs",
        "std::process::command",
        "std::fs::",
        "shell:",
        "fs:",
    ] {
        if lower.contains(forbidden) {
            return Err(format!(
                "forbidden file/shell API `{forbidden}` in {relative}"
            ));
        }
    }

    let without_approved_metadata = lower
        .replace("https://schema.tauri.app/config/2", "")
        .replace("https://ui.shadcn.com/schema.json", "")
        .replace("http://ipc.localhost", "");
    if without_approved_metadata.contains("http://")
        || without_approved_metadata.contains("https://")
    {
        return Err(format!("arbitrary remote origin in {relative}"));
    }

    let logging_call = lower.contains("console.log")
        || compact.contains("println!(")
        || compact.contains("eprintln!(")
        || lower.contains("tracing::");
    let secret_term = ["secret", "canary", "password", "api_key", "authorization"]
        .iter()
        .any(|term| lower.contains(term));
    if logging_call && secret_term {
        return Err(format!("secret logging surface in {relative}"));
    }

    let sql_ddl = ["create", "table"].join(" ");
    if normalized_whitespace.contains(&sql_ddl) {
        return Err(format!("forbidden SQL DDL in {relative}"));
    }
    if relative == "apps/windows/src-tauri/src/lib.rs" {
        let rust_code = rust_without_comments(lower);
        let compact = rust_code
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let marker = "generate_handler![";
        let handlers = compact.match_indices(marker).collect::<Vec<_>>();
        if handlers.len() != 1 {
            return Err("release command table must contain exactly one handler".to_owned());
        }
        let tail = &compact[handlers[0].0 + marker.len()..];
        let commands = tail
            .split(']')
            .next()
            .ok_or_else(|| "release command handler is malformed".to_owned())?;
        if commands != APPROVED_TAURI_COMMANDS.trim() {
            return Err("release command allowlist contains an unapproved command".to_owned());
        }
    }
    Ok(())
}

fn rust_without_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut position = 0;
    let mut block_depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    while position < bytes.len() {
        if block_depth > 0 {
            if bytes.get(position..position + 2) == Some(b"/*") {
                block_depth += 1;
                position += 2;
            } else if bytes.get(position..position + 2) == Some(b"*/") {
                block_depth -= 1;
                position += 2;
            } else {
                position += 1;
            }
            output.push(' ');
            continue;
        }
        if !in_string && bytes.get(position..position + 2) == Some(b"//") {
            while position < bytes.len() && bytes[position] != b'\n' {
                position += 1;
            }
            output.push('\n');
            continue;
        }
        if !in_string && bytes.get(position..position + 2) == Some(b"/*") {
            block_depth = 1;
            position += 2;
            output.push(' ');
            continue;
        }
        let byte = bytes[position];
        output.push(char::from(byte));
        position += 1;
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        }
    }
    output
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut visited = BTreeSet::new();
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    while let Some(directory) = pending.pop() {
        let canonical = directory
            .canonicalize()
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        if !canonical.starts_with(&canonical_root) || !visited.insert(canonical) {
            return Err(format!(
                "directory link escapes or cycles: {}",
                directory.display()
            ));
        }
        for entry in
            fs::read_dir(&directory).map_err(|error| format!("{}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            if matches!(
                relative.as_str(),
                ".toolchains"
                    | ".agents"
                    | ".codex"
                    | ".git"
                    | "node_modules"
                    | "target"
                    | "_agentic-out"
                    | "agentic-workflow"
            ) || relative == "apps/windows/node_modules"
                || relative == "apps/windows/dist"
                || relative == "apps/windows/src-tauri/gen"
            {
                continue;
            }
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                return Err(format!(
                    "directory/file link is not allowed: {}",
                    path.display()
                ));
            }
            if file_type.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}
