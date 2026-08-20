//! Versioned RSS/Atom source contracts.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaveSourceInputV1 {
    pub contract_version: u32,
    pub source_kind: String,
    pub url: String,
    pub expected_configuration_revision: u64,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatusV1 {
    Ready,
    Error,
    RetryWait,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceRetryabilityV1 {
    Never,
    Manual,
    Automatic,
    After,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceViewV1 {
    pub contract_version: u32,
    pub source_id: String,
    pub source_kind: String,
    /// Safe display form. Query and fragment are deliberately omitted.
    pub display_url: String,
    pub enabled: bool,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
    pub last_success_at: Option<String>,
    pub freshness: Option<String>,
    pub status: SourceStatusV1,
    pub retryability: SourceRetryabilityV1,
    pub next_allowed_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourcePageV1 {
    pub contract_version: u32,
    pub items: Vec<SourceViewV1>,
    pub next_cursor: Option<String>,
}
