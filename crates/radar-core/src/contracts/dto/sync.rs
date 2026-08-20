//! Versioned RSS/Atom foreground synchronization contracts.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SyncRunIdV1(String);

impl SyncRunIdV1 {
    #[must_use]
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (value.len() == 28
            && value.starts_with("run:")
            && value[4..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')))
        .then_some(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SyncRunIdV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyncTargetV1 {
    AllEnabledRssAtom,
    SourceId { source_id: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartSyncInputV1 {
    pub contract_version: u32,
    pub target: SyncTargetV1,
    pub idempotency_key: String,
    pub foreground_budget_ms: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStateV1 {
    Queued,
    Running,
    RetryWait,
    Succeeded,
    PartiallySucceeded,
    Failed,
    Cancelled,
}

impl TaskStateV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::RetryWait => "retry_wait",
            Self::Succeeded => "succeeded",
            Self::PartiallySucceeded => "partially_succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::PartiallySucceeded | Self::Failed | Self::Cancelled
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskRefV1 {
    pub contract_version: u32,
    pub task_id: String,
    pub state: TaskStateV1,
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncStatusV1 {
    pub contract_version: u32,
    pub source_id: String,
    pub source_revision: u64,
    pub state: TaskStateV1,
    pub last_success_at: Option<String>,
    pub error_code: Option<String>,
    pub next_allowed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskSnapshotV1 {
    pub contract_version: u32,
    pub task_id: String,
    pub target: SyncTargetV1,
    pub state: TaskStateV1,
    pub revision: u64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub updated_at: String,
    pub error_summary: Option<String>,
    pub result_ref: Option<SyncRunIdV1>,
    pub sources: Vec<SourceSyncStatusV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncRunOutcomeV1 {
    SucceededWithResults,
    SucceededZeroResults,
    PartiallySucceeded,
    Failed,
}

impl SyncRunOutcomeV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SucceededWithResults => "succeeded_with_results",
            Self::SucceededZeroResults => "succeeded_zero_results",
            Self::PartiallySucceeded => "partially_succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncResultDispositionV1 {
    Inserted,
    Updated,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetSyncResultInputV1 {
    pub contract_version: u32,
    pub sync_run_id: SyncRunIdV1,
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncResultCountsV1 {
    pub inserted: u32,
    pub updated: u32,
    pub skipped: u32,
    pub failed: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncSourceResultV1 {
    pub contract_version: u32,
    pub source_id: String,
    pub source_revision: u64,
    pub source_kind: String,
    pub publisher: String,
    pub status: String,
    pub counts: SyncResultCountsV1,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncResultItemV1 {
    pub contract_version: u32,
    pub result_item_id: String,
    pub sync_run_id: SyncRunIdV1,
    pub source_id: String,
    #[serde(default)]
    pub intel_item_id: Option<String>,
    pub source_kind: String,
    pub publisher: String,
    pub original_title: String,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub original_url: String,
    pub disposition: SyncResultDispositionV1,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncResultSummaryV1 {
    pub contract_version: u32,
    pub sync_run_id: SyncRunIdV1,
    pub task_id: String,
    pub outcome: SyncRunOutcomeV1,
    pub started_at: String,
    pub finished_at: String,
    pub counts: SyncResultCountsV1,
    pub sources: Vec<SyncSourceResultV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncResultPageV1 {
    pub contract_version: u32,
    pub summary: SyncResultSummaryV1,
    pub items: Vec<SyncResultItemV1>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceReadinessStatusV1 {
    NotConfigured,
    Available,
    Syncing,
    RateLimited,
    Failed,
    Disabled,
    RetryWait,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryReadinessStatusV1 {
    NotConfigured,
    Ready,
    Syncing,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceReadinessV1 {
    pub contract_version: u32,
    pub source_id: Option<String>,
    pub source_kind: String,
    pub status: SourceReadinessStatusV1,
    pub last_success_at: Option<String>,
    pub next_allowed_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceDeliveryReadinessV1 {
    pub contract_version: u32,
    pub required_source_kinds: Vec<String>,
    pub status: DeliveryReadinessStatusV1,
    pub sources: Vec<SourceReadinessV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncHealthSummaryV1 {
    pub contract_version: u32,
    pub latest_task: Option<TaskSnapshotV1>,
    pub pending_task_count: u32,
    pub last_success_at: Option<String>,
    pub freshness: Option<String>,
    pub source_results: Vec<SourceSyncStatusV1>,
    pub readiness: SourceDeliveryReadinessV1,
}
