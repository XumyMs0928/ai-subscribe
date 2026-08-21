//! Deterministic, AI-independent intelligence value rule V1.

use crate::contracts::dto::configuration_validation::AttentionConfigurationV1;
use crate::contracts::effects::normalize_rfc3339_utc;
use crate::domain::rules::configuration_validation::{
    canonical_source_identifier, canonical_source_kind, configuration_hash, configuration_is_valid,
    expression_matches, normalize,
};
use serde::{Deserialize, Serialize};

pub const INTELLIGENCE_VALUE_RULE_VERSION: &str = "rss-intelligence-value-v1";
pub const TRACK_WEIGHT: u8 = 25;
pub const SOURCE_TRUST_WEIGHT: u8 = 25;
pub const FRESHNESS_WEIGHT: u8 = 20;
pub const TECHNICAL_IMPACT_WEIGHT: u8 = 20;
pub const USER_RULE_WEIGHT: u8 = 10;
pub const FRESHNESS_24H_MS: u64 = 86_400_000;
pub const FRESHNESS_7D_MS: u64 = 604_800_000;
pub const FRESHNESS_30D_MS: u64 = 2_592_000_000;
pub const FRESHNESS_7D_POINTS: u8 = 15;
pub const FRESHNESS_30D_POINTS: u8 = 8;
pub const IMPORTANCE_LOW_MAX: u8 = 49;
pub const IMPORTANCE_MEDIUM_MIN: u8 = 50;
pub const IMPORTANCE_MEDIUM_MAX: u8 = 79;
pub const IMPORTANCE_HIGH_MIN: u8 = 80;
pub const IMPORTANCE_HIGH_MAX: u8 = 100;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportanceV1 {
    Low,
    Medium,
    High,
}

impl ImportanceV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamDispositionV1 {
    HighValue,
    OrdinaryCandidate,
}

impl StreamDispositionV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HighValue => "high_value",
            Self::OrdinaryCandidate => "ordinary_candidate",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiRuleStatusV1 {
    Unavailable,
}

impl AiRuleStatusV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "unavailable"
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleFactorKindV1 {
    Track,
    SourceTrust,
    Freshness,
    TechnicalImpact,
    UserRule,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleFactorV1 {
    pub factor: RuleFactorKindV1,
    pub points: u8,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FilterReasonV1 {
    pub code: String,
    pub actual: Option<u8>,
    pub threshold: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleEvaluationV1 {
    pub contract_version: u32,
    pub rule_version: String,
    pub fact_revision: u64,
    pub configuration_revision: u64,
    pub configuration_hash: String,
    pub evaluated_at_ms: u64,
    pub score: u8,
    pub importance: ImportanceV1,
    pub disposition: StreamDispositionV1,
    pub matched_track_ids: Vec<String>,
    pub factors: Vec<RuleFactorV1>,
    pub filter_reasons: Vec<FilterReasonV1>,
    pub ai_status: AiRuleStatusV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntelligenceValueContext {
    pub fact_revision: u64,
    pub configuration_revision: u64,
    pub configuration_hash: String,
    pub source_kind: String,
    pub source_identifier: String,
    pub publisher: String,
    pub original_url: String,
    pub title: String,
    pub source_summary: Option<String>,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub evaluated_at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationError {
    InvalidConfiguration,
    InvalidContext,
}

struct EvaluationSignals {
    matched_track_ids: Vec<String>,
    source: SourceMatch,
    trust: u8,
    track_points: u8,
    trust_points: u8,
    freshness_points: u8,
    freshness_reasons: Vec<String>,
    technical_points: u8,
    technical_reasons: Vec<String>,
    include: MatchState,
    exclude: MatchState,
    user_points: u8,
    score: u8,
}

#[derive(Clone, Copy)]
enum SourceMatch {
    Missing,
    Disabled,
    Enabled,
}

#[derive(Clone, Copy)]
enum MatchState {
    Matched,
    NotMatched,
}

impl MatchState {
    const fn is_matched(self) -> bool {
        matches!(self, Self::Matched)
    }
}

/// Evaluates an RSS/Atom fact using the frozen V1 rule contract.
///
/// # Errors
/// Returns an error when the versioned configuration or normalized fact context
/// violates an invariant that must already hold at the application boundary.
pub fn evaluate_intelligence_value(
    configuration: &AttentionConfigurationV1,
    context: &IntelligenceValueContext,
) -> Result<RuleEvaluationV1, EvaluationError> {
    let configuration = normalize(configuration);
    if !configuration_is_valid(&configuration)
        || context.fact_revision == 0
        || context.configuration_revision == 0
        || context.evaluated_at_ms == 0
        || !is_lower_hex_64(&context.configuration_hash)
        || configuration_hash(&configuration) != context.configuration_hash
        || canonical_source_kind(&context.source_kind) != Some("rss")
        || parse_rfc3339_ms(&context.collected_at).is_none()
        || context
            .published_at
            .as_deref()
            .is_some_and(|value| parse_rfc3339_ms(value).is_none())
    {
        return Err(EvaluationError::InvalidConfiguration);
    }

    let signals = evaluation_signals(&configuration, context)?;
    let factors = evaluation_factors(&signals);
    let filter_reasons = evaluation_filters(&configuration, context, &signals);

    Ok(RuleEvaluationV1 {
        contract_version: 1,
        rule_version: INTELLIGENCE_VALUE_RULE_VERSION.to_owned(),
        fact_revision: context.fact_revision,
        configuration_revision: context.configuration_revision,
        configuration_hash: context.configuration_hash.clone(),
        evaluated_at_ms: context.evaluated_at_ms,
        score: signals.score,
        importance: match signals.score {
            0..=IMPORTANCE_LOW_MAX => ImportanceV1::Low,
            IMPORTANCE_MEDIUM_MIN..=IMPORTANCE_MEDIUM_MAX => ImportanceV1::Medium,
            _ => ImportanceV1::High,
        },
        disposition: if filter_reasons.is_empty() {
            StreamDispositionV1::HighValue
        } else {
            StreamDispositionV1::OrdinaryCandidate
        },
        matched_track_ids: signals.matched_track_ids,
        factors,
        filter_reasons,
        ai_status: AiRuleStatusV1::Unavailable,
    })
}

fn evaluation_signals(
    configuration: &AttentionConfigurationV1,
    context: &IntelligenceValueContext,
) -> Result<EvaluationSignals, EvaluationError> {
    let searchable = searchable_text(&context.title, context.source_summary.as_deref());
    let matched_track_ids = matched_tracks(configuration, &searchable);
    let track_points = u8::from(!matched_track_ids.is_empty()) * TRACK_WEIGHT;
    let source = configuration.source_preferences.iter().find(|source| {
        canonical_source_kind(&source.source_kind) == Some("rss")
            && same_source_identifier(&source.identifier, &context.source_identifier)
    });
    let trust = source.map_or(0, |source| source.trust);
    let enabled_multiplier = u16::from(source.is_some_and(|source| source.enabled));
    let trust_points = u8::try_from(
        ((u16::from(trust) * u16::from(SOURCE_TRUST_WEIGHT)) / 100) * enabled_multiplier,
    )
    .map_err(|_| EvaluationError::InvalidConfiguration)?;
    let (freshness_points, freshness_reasons) = freshness(context)?;
    let technical_reasons = technical_impact_reasons(&searchable);
    let technical_points = u8::from(!technical_reasons.is_empty()) * TECHNICAL_IMPACT_WEIGHT;
    let include_matches = configuration.include_expression.is_empty()
        || expression_matches(&configuration.include_expression, &searchable);
    let exclude_matches = expression_matches(&configuration.exclude_expression, &searchable);
    let user_points = u8::from(include_matches) * USER_RULE_WEIGHT;
    let score = track_points
        .saturating_add(trust_points)
        .saturating_add(freshness_points)
        .saturating_add(technical_points)
        .saturating_add(user_points)
        .min(100);
    Ok(EvaluationSignals {
        matched_track_ids,
        source: match source {
            None => SourceMatch::Missing,
            Some(source) if source.enabled => SourceMatch::Enabled,
            Some(_) => SourceMatch::Disabled,
        },
        trust,
        track_points,
        trust_points,
        freshness_points,
        freshness_reasons,
        technical_points,
        technical_reasons,
        include: if include_matches {
            MatchState::Matched
        } else {
            MatchState::NotMatched
        },
        exclude: if exclude_matches {
            MatchState::Matched
        } else {
            MatchState::NotMatched
        },
        user_points,
        score,
    })
}

fn evaluation_factors(signals: &EvaluationSignals) -> Vec<RuleFactorV1> {
    vec![
        factor(
            RuleFactorKindV1::Track,
            signals.track_points,
            vec![
                if signals.matched_track_ids.is_empty() {
                    "track.no_match"
                } else {
                    "track.matched"
                }
                .to_owned(),
            ],
        ),
        factor(
            RuleFactorKindV1::SourceTrust,
            signals.trust_points,
            vec![
                if matches!(signals.source, SourceMatch::Missing) {
                    "source_trust.not_configured"
                } else {
                    "source_trust.configured"
                }
                .to_owned(),
            ],
        ),
        factor(
            RuleFactorKindV1::Freshness,
            signals.freshness_points,
            signals.freshness_reasons.clone(),
        ),
        factor(
            RuleFactorKindV1::TechnicalImpact,
            signals.technical_points,
            if signals.technical_reasons.is_empty() {
                vec!["technical_impact.no_match".to_owned()]
            } else {
                signals.technical_reasons.clone()
            },
        ),
        factor(
            RuleFactorKindV1::UserRule,
            signals.user_points,
            vec![
                if signals.include.is_matched() {
                    "user_rule.include_satisfied"
                } else {
                    "user_rule.include_not_matched"
                }
                .to_owned(),
            ],
        ),
    ]
}

fn evaluation_filters(
    configuration: &AttentionConfigurationV1,
    context: &IntelligenceValueContext,
    signals: &EvaluationSignals,
) -> Vec<FilterReasonV1> {
    let mut filters = Vec::new();
    match signals.source {
        SourceMatch::Missing => filters.push(reason("source_not_configured")),
        SourceMatch::Disabled => filters.push(reason("source_disabled")),
        SourceMatch::Enabled => {}
    }
    if !(configuration.minimum_trust..=configuration.maximum_trust).contains(&signals.trust) {
        filters.push(FilterReasonV1 {
            code: "trust_outside_range".to_owned(),
            actual: Some(signals.trust),
            threshold: Some(if signals.trust < configuration.minimum_trust {
                configuration.minimum_trust
            } else {
                configuration.maximum_trust
            }),
        });
    }
    for (blocked, code) in [
        (signals.matched_track_ids.is_empty(), "track_not_matched"),
        (
            !signals.include.is_matched(),
            "include_expression_not_matched",
        ),
        (signals.exclude.is_matched(), "exclude_expression_matched"),
        (
            !inside_active_window(configuration, context.evaluated_at_ms),
            "outside_active_window",
        ),
        (context.publisher.trim().is_empty(), "publisher_missing"),
        (
            !valid_https_url(&context.original_url),
            "original_url_invalid",
        ),
    ] {
        if blocked {
            filters.push(reason(code));
        }
    }
    if signals.score < configuration.alert_threshold {
        filters.push(FilterReasonV1 {
            code: "score_below_threshold".to_owned(),
            actual: Some(signals.score),
            threshold: Some(configuration.alert_threshold),
        });
    }
    filters
}

fn factor(factor: RuleFactorKindV1, points: u8, reason_codes: Vec<String>) -> RuleFactorV1 {
    RuleFactorV1 {
        factor,
        points,
        reason_codes,
    }
}

fn reason(code: &str) -> FilterReasonV1 {
    FilterReasonV1 {
        code: code.to_owned(),
        actual: None,
        threshold: None,
    }
}

fn searchable_text(title: &str, summary: Option<&str>) -> String {
    format!("{} {}", title.trim(), summary.unwrap_or_default().trim())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn matched_tracks(configuration: &AttentionConfigurationV1, searchable: &str) -> Vec<String> {
    configuration
        .tracks
        .iter()
        .filter(|track| track.enabled)
        .filter(|track| {
            track_terms(&track.id, &track.name)
                .iter()
                .any(|term| token_matches(searchable, term))
        })
        .map(|track| track.id.clone())
        .collect()
}

fn track_terms(id: &str, name: &str) -> Vec<String> {
    let folded_id = id.to_ascii_lowercase();
    let defaults: &[&str] = match folded_id.as_str() {
        "ai_agents" | "ai-agents" => &["agent", "agents", "智能体"],
        "foundation_models" | "foundation-models" => &[
            "foundation model",
            "large language model",
            "llm",
            "基础模型",
            "大模型",
        ],
        "local_models" | "local-models" => {
            &["local model", "on-device model", "本地模型", "端侧模型"]
        }
        _ => &[],
    };
    let mut terms = defaults
        .iter()
        .map(|term| (*term).to_owned())
        .collect::<Vec<_>>();
    terms.push(name.to_lowercase());
    terms.push(id.replace(['_', '-'], " ").to_lowercase());
    terms.sort();
    terms.dedup();
    terms
}

fn technical_impact_reasons(searchable: &str) -> Vec<String> {
    const CATEGORIES: [(&str, &[&str]); 6] = [
        (
            "model_capability",
            &[
                "capability",
                "benchmark",
                "context window",
                "reasoning",
                "multimodal",
                "能力",
                "基准",
                "上下文窗口",
                "推理",
                "多模态",
            ],
        ),
        (
            "development_framework",
            &[
                "framework",
                "sdk",
                "api",
                "breaking change",
                "框架",
                "开发包",
                "接口",
                "破坏性变更",
            ],
        ),
        (
            "deployment",
            &[
                "deploy",
                "deployment",
                "serving",
                "runtime",
                "部署",
                "推理服务",
                "运行时",
            ],
        ),
        (
            "cost",
            &[
                "price",
                "pricing",
                "cost",
                "token price",
                "价格",
                "定价",
                "成本",
            ],
        ),
        (
            "security",
            &[
                "security",
                "vulnerability",
                "cve",
                "patch",
                "安全",
                "漏洞",
                "补丁",
            ],
        ),
        (
            "technical_selection",
            &[
                "stable",
                "release",
                "deprecated",
                "migration",
                "ga",
                "lts",
                "稳定版",
                "发布",
                "弃用",
                "迁移",
                "技术选型",
            ],
        ),
    ];
    CATEGORIES
        .iter()
        .filter(|(_, terms)| terms.iter().any(|term| token_matches(searchable, term)))
        .map(|(category, _)| format!("technical_impact.{category}"))
        .collect()
}

fn token_matches(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    if !needle.is_ascii() {
        return haystack.contains(needle);
    }
    haystack.match_indices(needle).any(|(start, _)| {
        let end = start + needle.len();
        let left = haystack[..start].chars().next_back();
        let right = haystack[end..].chars().next();
        left.is_none_or(|character| !character.is_ascii_alphanumeric())
            && right.is_none_or(|character| !character.is_ascii_alphanumeric())
    })
}

fn freshness(context: &IntelligenceValueContext) -> Result<(u8, Vec<String>), EvaluationError> {
    let (timestamp, fallback) = match context.published_at.as_deref() {
        Some(value) => (
            parse_rfc3339_ms(value).ok_or(EvaluationError::InvalidContext)?,
            false,
        ),
        None => (
            parse_rfc3339_ms(&context.collected_at).ok_or(EvaluationError::InvalidContext)?,
            true,
        ),
    };
    let age = context.evaluated_at_ms.saturating_sub(timestamp);
    let (points, reason) = if timestamp > context.evaluated_at_ms {
        (FRESHNESS_WEIGHT, "freshness.future_timestamp")
    } else if age <= FRESHNESS_24H_MS {
        (FRESHNESS_WEIGHT, "freshness.within_24h")
    } else if age <= FRESHNESS_7D_MS {
        (FRESHNESS_7D_POINTS, "freshness.within_7d")
    } else if age <= FRESHNESS_30D_MS {
        (FRESHNESS_30D_POINTS, "freshness.within_30d")
    } else {
        (0, "freshness.older_than_30d")
    };
    let mut reasons = Vec::with_capacity(2);
    if fallback {
        reasons.push("freshness.collected_at_fallback".to_owned());
    }
    reasons.push(reason.to_owned());
    Ok((points, reasons))
}

fn inside_active_window(configuration: &AttentionConfigurationV1, now_ms: u64) -> bool {
    configuration
        .active_from
        .as_deref()
        .and_then(parse_rfc3339_ms)
        .is_none_or(|from| now_ms >= from)
        && configuration
            .active_until
            .as_deref()
            .and_then(parse_rfc3339_ms)
            .is_none_or(|until| now_ms <= until)
}

fn same_source_identifier(left: &str, right: &str) -> bool {
    canonical_source_identifier(left)
        .zip(canonical_source_identifier(right))
        .is_some_and(|(left, right)| left == right)
}

fn valid_https_url(value: &str) -> bool {
    canonical_source_identifier(value).is_some_and(|canonical| canonical.starts_with("https://"))
}

fn is_lower_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn parse_rfc3339_ms(value: &str) -> Option<u64> {
    if normalize_rfc3339_utc(value).as_deref() != Some(value) {
        return None;
    }
    let body = value.strip_suffix('Z')?;
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let (date, time) = whole.split_once('T')?;
    let mut date_parts = date.split('-');
    let year = date_parts.next()?.parse::<i64>().ok()?;
    let month = date_parts.next()?.parse::<i64>().ok()?;
    let day = date_parts.next()?.parse::<i64>().ok()?;
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<u64>().ok()?;
    let minute = time_parts.next()?.parse::<u64>().ok()?;
    let second = time_parts.next()?.parse::<u64>().ok()?.min(59);
    let days = days_from_civil(year, month, day);
    let seconds = u64::try_from(days)
        .ok()?
        .checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)?;
    let mut fraction_ms = 0_u64;
    for (index, byte) in fraction.bytes().take(3).enumerate() {
        fraction_ms = fraction_ms
            .checked_add(u64::from(byte.checked_sub(b'0')?) * [100_u64, 10, 1][index])?;
    }
    seconds.checked_mul(1_000)?.checked_add(fraction_ms)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}
