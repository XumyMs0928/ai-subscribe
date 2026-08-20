//! Versioned real RSS intelligence feed contracts.

use serde::{Deserialize, Serialize};

pub const INTEL_FEED_MAX_PAGE_SIZE: u32 = 100;
pub const INTEL_FEED_DEFAULT_PAGE_SIZE: u32 = 30;
pub const INTEL_FEED_MAX_CURSOR_BYTES: usize = 1024;
pub const LIST_EXCERPT_MAX_CHARS: usize = 280;
pub const INTEL_FEED_MAX_TRACK_FILTERS: usize = 32;
pub const INTEL_FEED_MAX_SOURCE_FILTERS: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntelFeedStreamV1 {
    HighValue,
    OrdinaryCandidate,
}

impl IntelFeedStreamV1 {
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
pub enum IntelFeedTimeWindowV1 {
    AllTime,
    #[serde(rename = "last_24h")]
    Last24h,
    #[serde(rename = "last_7d")]
    Last7d,
    #[serde(rename = "last_30d")]
    Last30d,
}

impl IntelFeedTimeWindowV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AllTime => "all_time",
            Self::Last24h => "last_24h",
            Self::Last7d => "last_7d",
            Self::Last30d => "last_30d",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntelFeedSortV1 {
    ScoreDesc,
}

impl IntelFeedSortV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "score_desc"
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntelFeedFiltersV1 {
    pub track_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub time_window: IntelFeedTimeWindowV1,
    pub importance: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueryIntelFeedInputV1 {
    pub contract_version: u32,
    pub stream: IntelFeedStreamV1,
    pub filters: IntelFeedFiltersV1,
    pub sort: IntelFeedSortV1,
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntelFeedItemV1 {
    pub contract_version: u32,
    pub intel_item_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub publisher: String,
    pub title: String,
    pub source_excerpt: Option<String>,
    pub excerpt_truncated: bool,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub importance: String,
    pub score: u8,
    pub matched_track_ids: Vec<String>,
    pub stream_disposition: IntelFeedStreamV1,
    pub ai_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntelFeedPageV1 {
    pub contract_version: u32,
    pub stream: IntelFeedStreamV1,
    pub filters: IntelFeedFiltersV1,
    pub sort: IntelFeedSortV1,
    pub rule_version: String,
    pub configuration_revision: u64,
    pub configuration_hash: String,
    pub as_of_ms: u64,
    pub items: Vec<IntelFeedItemV1>,
    pub next_cursor: Option<String>,
}
