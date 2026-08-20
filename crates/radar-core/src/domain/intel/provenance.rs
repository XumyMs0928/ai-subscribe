//! Provenance value objects that never invent missing source facts.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorAvailability {
    Available,
    Unavailable,
    UnknownLegacy,
}

impl AuthorAvailability {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::UnknownLegacy => "unknown_legacy",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NormalizedWarning {
    pub field: String,
    pub code: super::NormalizationIssueCode,
}
