//! Pure RSS/Atom candidate normalization.

use std::fmt::Write;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::{Host, Url};

use super::{AuthorAvailability, NormalizedWarning};
use crate::domain::sources::{RawSourceCandidate, is_public_ip};

pub const MAX_EXTERNAL_ID_BYTES: usize = 512;
pub const MAX_TITLE_CHARS: usize = 1_024;
pub const MAX_AUTHOR_CHARS: usize = 512;
pub const MAX_SUMMARY_CHARS: usize = 16_384;
pub const MAX_URL_BYTES: usize = 4_096;
pub const MAX_PUBLISHER_CHARS: usize = 253;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct IntelItemId(String);

impl IntelItemId {
    #[must_use]
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (value.len() == 70
            && value.starts_with("intel:")
            && value[6..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')))
        .then_some(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NormalizationIssueCode {
    Required,
    TooLong,
    InvalidUrl,
    InvalidHash,
    InvalidTime,
    InvalidOptionalTime,
    InvalidWarning,
}

impl NormalizationIssueCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::TooLong => "too_long",
            Self::InvalidUrl => "invalid_url",
            Self::InvalidHash => "invalid_hash",
            Self::InvalidTime => "invalid_time",
            Self::InvalidOptionalTime => "invalid_optional_time",
            Self::InvalidWarning => "invalid_warning",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NormalizationIssue {
    pub field: String,
    pub code: NormalizationIssueCode,
    pub candidate_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedIntelCandidate {
    pub intel_item_id: IntelItemId,
    pub canonical_external_id: String,
    pub source_id: String,
    pub stable_external_id: String,
    pub source_kind: &'static str,
    pub publisher: String,
    pub original_title: String,
    pub original_url: String,
    pub author: Option<String>,
    pub author_availability: AuthorAvailability,
    pub source_summary: Option<String>,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub content_hash: String,
    pub warnings: Vec<NormalizedWarning>,
}

/// Validates and normalizes one adapter-produced RSS/Atom candidate without I/O.
///
/// # Errors
/// Returns the first stable record-level issue. Optional time failures are warnings.
pub fn normalize_rss_candidate(
    source_id: &str,
    publisher: &str,
    collected_at: &str,
    candidate: &RawSourceCandidate,
) -> Result<NormalizedIntelCandidate, NormalizationIssue> {
    let candidate_ref = candidate_ref(source_id, &candidate.stable_external_id);
    let stable_external_id = candidate.stable_external_id.trim();
    require_text(
        stable_external_id,
        "stable_external_id",
        MAX_EXTERNAL_ID_BYTES,
        true,
        &candidate_ref,
    )?;
    let publisher = publisher.trim();
    require_text(
        publisher,
        "publisher",
        MAX_PUBLISHER_CHARS,
        false,
        &candidate_ref,
    )?;
    let title = candidate
        .title
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    require_text(
        title,
        "original_title",
        MAX_TITLE_CHARS,
        false,
        &candidate_ref,
    )?;
    let original_url = canonical_item_url(
        candidate.original_url.as_deref().unwrap_or_default(),
        &candidate_ref,
    )?;
    let collected_at =
        crate::contracts::effects::normalize_rfc3339_utc(collected_at).ok_or_else(|| {
            issue(
                "collected_at",
                NormalizationIssueCode::InvalidTime,
                &candidate_ref,
            )
        })?;
    if !valid_content_hash(&candidate.content_hash) {
        return Err(issue(
            "content_hash",
            NormalizationIssueCode::InvalidHash,
            &candidate_ref,
        ));
    }
    let author = optional_text(
        candidate.author.as_deref(),
        "author",
        MAX_AUTHOR_CHARS,
        &candidate_ref,
    )?;
    let source_summary = optional_text(
        candidate.summary.as_deref(),
        "source_summary",
        MAX_SUMMARY_CHARS,
        &candidate_ref,
    )?;
    let mut warnings = candidate
        .warnings
        .iter()
        .map(|warning| normalize_warning(warning, &candidate_ref))
        .collect::<Result<Vec<_>, _>>()?;
    let published_at = normalize_optional_time(
        candidate.published_at.as_deref(),
        "published_at",
        &mut warnings,
    );
    let canonical_external_id = canonical_external_id(source_id, stable_external_id);
    let intel_item_id = derive_intel_item_id("rss_atom", &canonical_external_id);
    let author_availability = if author.is_some() {
        AuthorAvailability::Available
    } else {
        AuthorAvailability::Unavailable
    };
    Ok(NormalizedIntelCandidate {
        intel_item_id,
        canonical_external_id,
        source_id: source_id.to_owned(),
        stable_external_id: stable_external_id.to_owned(),
        source_kind: "rss_atom",
        publisher: publisher.to_owned(),
        original_title: title.to_owned(),
        original_url,
        author,
        author_availability,
        source_summary,
        published_at,
        collected_at,
        content_hash: candidate.content_hash.clone(),
        warnings,
    })
}

fn valid_content_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn normalize_optional_time(
    value: Option<&str>,
    field: &'static str,
    warnings: &mut Vec<NormalizedWarning>,
) -> Option<String> {
    let value = value?;
    if let Some(normalized) = crate::contracts::effects::normalize_rfc3339_utc(value) {
        Some(normalized)
    } else {
        push_warning(warnings, field);
        None
    }
}

#[must_use]
pub fn canonical_external_id(source_id: &str, stable_external_id: &str) -> String {
    format!(
        "rss-entry:{}",
        digest_hex(
            format!("ai-subscribe:source-entry:v1\0{source_id}\0{stable_external_id}").as_bytes()
        )
    )
}

#[must_use]
pub fn derive_intel_item_id(source_kind: &str, canonical_external_id: &str) -> IntelItemId {
    IntelItemId(format!(
        "intel:{}",
        digest_hex(
            format!("ai-subscribe:intel:v1\0{source_kind}\0{canonical_external_id}").as_bytes()
        )
    ))
}

pub(super) fn canonical_item_url(
    value: &str,
    candidate_ref: &str,
) -> Result<String, NormalizationIssue> {
    let value = value.trim();
    if value.is_empty() {
        return Err(issue(
            "original_url",
            NormalizationIssueCode::Required,
            candidate_ref,
        ));
    }
    if value.len() > MAX_URL_BYTES {
        return Err(issue(
            "original_url",
            NormalizationIssueCode::TooLong,
            candidate_ref,
        ));
    }
    let mut url = Url::parse(value).map_err(|_| {
        issue(
            "original_url",
            NormalizationIssueCode::InvalidUrl,
            candidate_ref,
        )
    })?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host().as_ref().is_some_and(is_non_public_host)
    {
        return Err(issue(
            "original_url",
            NormalizationIssueCode::InvalidUrl,
            candidate_ref,
        ));
    }
    if url.port() == Some(443) {
        url.set_port(None).map_err(|()| {
            issue(
                "original_url",
                NormalizationIssueCode::InvalidUrl,
                candidate_ref,
            )
        })?;
    }
    url.set_fragment(None);
    Ok(url.to_string())
}

fn is_non_public_host(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(host) => host.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(ip) => !is_public_ip((*ip).into()),
        Host::Ipv6(ip) => !is_public_ip((*ip).into()),
    }
}

fn require_text(
    value: &str,
    field: &str,
    limit: usize,
    bytes: bool,
    candidate_ref: &str,
) -> Result<(), NormalizationIssue> {
    if value.is_empty() {
        return Err(issue(
            field,
            NormalizationIssueCode::Required,
            candidate_ref,
        ));
    }
    let length = if bytes {
        value.len()
    } else {
        value.chars().count()
    };
    if length > limit {
        return Err(issue(field, NormalizationIssueCode::TooLong, candidate_ref));
    }
    Ok(())
}

fn optional_text(
    value: Option<&str>,
    field: &str,
    limit: usize,
    candidate_ref: &str,
) -> Result<Option<String>, NormalizationIssue> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    value.map_or(Ok(None), |value| {
        require_text(value, field, limit, false, candidate_ref)?;
        Ok(Some(value.to_owned()))
    })
}

fn normalize_warning(
    warning: &crate::domain::sources::SourceFieldWarning,
    candidate_ref: &str,
) -> Result<NormalizedWarning, NormalizationIssue> {
    if !matches!(warning.field.as_str(), "published_at" | "updated_at")
        || warning.code != "source.invalid_optional_time"
    {
        return Err(issue(
            "warnings",
            NormalizationIssueCode::InvalidWarning,
            candidate_ref,
        ));
    }
    Ok(NormalizedWarning {
        field: warning.field.clone(),
        code: NormalizationIssueCode::InvalidOptionalTime,
    })
}

fn push_warning(warnings: &mut Vec<NormalizedWarning>, field: &str) {
    if !warnings.iter().any(|warning| warning.field == field) {
        warnings.push(NormalizedWarning {
            field: field.to_owned(),
            code: NormalizationIssueCode::InvalidOptionalTime,
        });
    }
}

fn candidate_ref(source_id: &str, stable_external_id: &str) -> String {
    format!(
        "candidate:{}",
        &digest_hex(format!("{source_id}\0{stable_external_id}").as_bytes())[..24]
    )
}

fn issue(field: &str, code: NormalizationIssueCode, candidate_ref: &str) -> NormalizationIssue {
    NormalizationIssue {
        field: field.to_owned(),
        code,
        candidate_ref: candidate_ref.to_owned(),
    }
}

fn digest_hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}
