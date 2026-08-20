//! Versioned attention-configuration wire contract.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttentionTrackV1 {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourcePreferenceV1 {
    pub source_kind: String,
    pub identifier: String,
    pub enabled: bool,
    pub trust: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuietHoursV1 {
    pub enabled: bool,
    pub start: String,
    pub end: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NotificationFrequencyV1 {
    pub enabled: bool,
    pub max_per_24h: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttentionConfigurationV1 {
    pub contract_version: u32,
    pub tracks: Vec<AttentionTrackV1>,
    pub include_expression: String,
    pub exclude_expression: String,
    pub source_preferences: Vec<SourcePreferenceV1>,
    pub refresh_enabled: bool,
    pub refresh_interval_minutes: u32,
    pub minimum_trust: u8,
    pub maximum_trust: u8,
    pub alert_threshold: u8,
    pub quiet_hours: QuietHoursV1,
    pub notification_frequency: NotificationFrequencyV1,
    pub active_from: Option<String>,
    pub active_until: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockingCodeV1 {
    ExpressionUnparseable,
    ValueOutOfRange,
    LowerBoundAboveUpperBound,
    InvalidSourceOrUnsupportedProtocol,
}

impl BlockingCodeV1 {
    pub const ALL: [Self; 4] = [
        Self::ExpressionUnparseable,
        Self::ValueOutOfRange,
        Self::LowerBoundAboveUpperBound,
        Self::InvalidSourceOrUnsupportedProtocol,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExpressionUnparseable => "expression_unparseable",
            Self::ValueOutOfRange => "value_out_of_range",
            Self::LowerBoundAboveUpperBound => "lower_bound_above_upper_bound",
            Self::InvalidSourceOrUnsupportedProtocol => "invalid_source_or_unsupported_protocol",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationBlockingErrorV1 {
    pub field_path: String,
    pub code: BlockingCodeV1,
    pub message_key: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NarrowingRiskCodeV1 {
    AllSourcesDisabled,
    AllHighTrustCandidatesFiltered,
}

impl NarrowingRiskCodeV1 {
    pub const ALL: [Self; 2] = [
        Self::AllSourcesDisabled,
        Self::AllHighTrustCandidatesFiltered,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AllSourcesDisabled => "all_sources_disabled",
            Self::AllHighTrustCandidatesFiltered => "all_high_trust_candidates_filtered",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationNarrowingRiskV1 {
    pub code: NarrowingRiskCodeV1,
    pub condition_key: String,
    pub consequence_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationValidationReceiptV1 {
    pub token: String,
    pub normalized_config_hash: String,
    pub validator_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidateConfigurationInputV1 {
    pub contract_version: u32,
    pub configuration: AttentionConfigurationV1,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationValidationResultV1 {
    pub contract_version: u32,
    pub blocking_errors: Vec<ConfigurationBlockingErrorV1>,
    pub narrowing_risks: Vec<ConfigurationNarrowingRiskV1>,
    pub validator_version: String,
    pub normalized_config_hash: String,
    pub validation_receipt: Option<ConfigurationValidationReceiptV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaveConfigurationInputV1 {
    pub contract_version: u32,
    pub configuration: AttentionConfigurationV1,
    pub expected_revision: u64,
    pub expected_normalized_config_hash: String,
    pub idempotency_key: String,
    pub validation_receipt: Option<ConfigurationValidationReceiptV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationViewV1 {
    pub contract_version: u32,
    pub revision: u64,
    pub validator_version: String,
    pub normalized_config_hash: String,
    pub configuration: AttentionConfigurationV1,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigurationCandidateContext {
    pub real_candidates: Vec<ConfigurationCandidateV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigurationCandidateV1 {
    pub source_kind: String,
    pub searchable_text: String,
}
