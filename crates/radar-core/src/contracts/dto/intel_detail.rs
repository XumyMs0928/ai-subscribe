//! Versioned real RSS evidence-detail and original-link intent contracts.

use serde::{Deserialize, Serialize};

use crate::domain::rules::intelligence_value::{
    FilterReasonV1, ImportanceV1, RuleFactorV1, StreamDispositionV1,
};

pub const INTEL_DETAIL_MAX_PROVENANCE: usize = 64;
pub const INTEL_DETAIL_MAX_SUMMARY_CHARS: usize = 16_384;
pub const INTEL_DETAIL_MAX_TEXT_CHARS: usize = 2_048;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueryIntelEvidenceDetailInputV1 {
    pub contract_version: u32,
    pub intel_item_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleEvidenceStatusV1 {
    Current,
    Unavailable,
    Stale,
}

impl RuleEvidenceStatusV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Unavailable => "unavailable",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiEvidenceStatusV1 {
    Unavailable,
}

impl AiEvidenceStatusV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "unavailable"
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceRoleV1 {
    Primary,
    Associated,
}

impl ProvenanceRoleV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Associated => "associated",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssociationEvidenceStatusV1 {
    Complete,
    Incomplete,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceFactsV1 {
    pub intel_item_id: String,
    pub fact_revision: u64,
    pub content_hash: String,
    pub content_state: String,
    pub publisher: String,
    pub title: String,
    pub source_summary: Option<String>,
    pub published_at: Option<String>,
    pub collected_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleExplanationV1 {
    pub rule_version: String,
    pub configuration_revision: u64,
    pub configuration_hash: String,
    pub evaluated_at_ms: u64,
    pub score: u8,
    pub importance: ImportanceV1,
    pub disposition: StreamDispositionV1,
    pub matched_track_ids: Vec<String>,
    pub factors: Vec<RuleFactorV1>,
    pub filter_reasons: Vec<FilterReasonV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntelProvenanceV1 {
    pub provenance_id: String,
    pub intel_item_id: String,
    pub role: ProvenanceRoleV1,
    pub source_id: String,
    pub source_kind: String,
    pub publisher: String,
    pub author: Option<String>,
    pub author_availability: String,
    pub original_title: String,
    pub display_url: String,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub first_discovered_at: String,
    pub last_updated_at: String,
    pub availability_status: String,
    pub can_open_original: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssociationEvidenceV1 {
    pub status: AssociationEvidenceStatusV1,
    pub issue_code: Option<String>,
    pub relation_type: Option<String>,
    pub evidence_basis: Option<String>,
    pub basis_version: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntelEvidenceDetailV1 {
    pub contract_version: u32,
    pub facts: SourceFactsV1,
    pub rule_status: RuleEvidenceStatusV1,
    pub rule_issue_code: Option<String>,
    pub rule: Option<RuleExplanationV1>,
    pub ai_status: AiEvidenceStatusV1,
    pub provenance: Vec<IntelProvenanceV1>,
    pub association: AssociationEvidenceV1,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OpenIntelOriginalInputV1 {
    pub contract_version: u32,
    pub intel_item_id: String,
    pub provenance_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OpenOriginalReceiptV1 {
    pub contract_version: u32,
    pub intel_item_id: String,
    pub provenance_id: String,
    pub status: String,
}
