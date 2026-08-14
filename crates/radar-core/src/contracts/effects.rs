//! Platform effect contract and pure in-memory state machine.

use crate::contracts::dto::OpaqueId;
use std::collections::BTreeMap;

use crate::contracts::errors::{AppError, ErrorCode};

pub const CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    ContractProbe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectStatus {
    Pending,
    Delivered,
    Denied,
    Expired,
    Failed,
}

impl EffectStatus {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }

    /// Parses a v1 effect status at an untrusted boundary.
    ///
    /// # Errors
    ///
    /// Returns a stable validation error for unknown enum values.
    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "pending" => Ok(Self::Pending),
            "delivered" => Ok(Self::Delivered),
            "denied" => Ok(Self::Denied),
            "expired" => Ok(Self::Expired),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::from_code(
                ErrorCode::ValidationEffectStatus,
                "contract-effect-status",
            )),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlatformEffect {
    contract_version: u32,
    effect_id: OpaqueId,
    idempotency_key: OpaqueId,
    kind: EffectKind,
    payload_version: u32,
    expires_at: String,
    status: EffectStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportResult {
    Applied,
    AlreadyApplied,
}

impl PlatformEffect {
    /// Creates the side-effect-free probe used by the v1 contract suite.
    ///
    /// # Errors
    ///
    /// Returns a stable validation error when an identifier or UTC timestamp is invalid.
    pub fn new_contract_probe(
        effect_id: impl Into<String>,
        idempotency_key: impl Into<String>,
        expires_at: impl Into<String>,
    ) -> Result<Self, AppError> {
        let effect_id = OpaqueId::parse(effect_id.into(), ErrorCode::ValidationEffectId)?;
        let idempotency_key =
            OpaqueId::parse(idempotency_key.into(), ErrorCode::ValidationIdempotencyKey)?;
        let expires_at = expires_at.into();
        let expires_at = validate_utc(&expires_at)?;

        Ok(Self {
            contract_version: CONTRACT_VERSION,
            effect_id,
            idempotency_key,
            kind: EffectKind::ContractProbe,
            payload_version: 1,
            expires_at,
            status: EffectStatus::Pending,
        })
    }

    #[must_use]
    pub const fn contract_version(&self) -> u32 {
        self.contract_version
    }

    #[must_use]
    pub fn effect_id(&self) -> &str {
        self.effect_id.as_str()
    }

    #[must_use]
    pub fn idempotency_key(&self) -> &str {
        self.idempotency_key.as_str()
    }

    #[must_use]
    pub const fn kind(&self) -> EffectKind {
        self.kind
    }

    #[must_use]
    pub const fn payload_version(&self) -> u32 {
        self.payload_version
    }

    #[must_use]
    pub fn expires_at(&self) -> &str {
        &self.expires_at
    }

    #[must_use]
    pub const fn status(&self) -> EffectStatus {
        self.status
    }

    /// Applies a terminal platform report idempotently.
    ///
    /// # Errors
    ///
    /// Returns validation for a non-terminal report and conflict for a different
    /// report after the effect has already reached a terminal state.
    fn report(&mut self, reported: EffectStatus) -> Result<ReportResult, AppError> {
        if !reported.is_terminal() {
            return Err(AppError::from_code(
                ErrorCode::ValidationEffectStatus,
                "contract-effect-status",
            ));
        }
        if self.status == EffectStatus::Pending {
            self.status = reported;
            return Ok(ReportResult::Applied);
        }
        if self.status == reported {
            return Ok(ReportResult::AlreadyApplied);
        }
        Err(AppError::from_code(
            ErrorCode::ConflictEffectAlreadyReported,
            "contract-effect-conflict",
        ))
    }
}

#[derive(Default)]
pub struct EffectLedger {
    effects_by_key: BTreeMap<Box<str>, PlatformEffect>,
}

impl EffectLedger {
    /// Registers one effect under its stable idempotency key.
    ///
    /// # Errors
    ///
    /// Returns conflict when the key is already associated with another effect.
    pub fn register(&mut self, effect: PlatformEffect) -> Result<(), AppError> {
        let key: Box<str> = effect.idempotency_key().into();
        if let Some(existing) = self.effects_by_key.get(&key) {
            if existing.effect_id() == effect.effect_id() {
                return Ok(());
            }
            return Err(AppError::from_code(
                ErrorCode::ConflictEffectAlreadyReported,
                "contract-effect-identity-conflict",
            ));
        }
        self.effects_by_key.insert(key, effect);
        Ok(())
    }

    /// Reports a terminal result through the keyed in-memory authority.
    ///
    /// # Errors
    ///
    /// Returns validation for an unknown key or identity and delegates state
    /// transition errors to the registered effect.
    pub fn report(
        &mut self,
        effect_id: &str,
        idempotency_key: &str,
        reported: EffectStatus,
    ) -> Result<ReportResult, AppError> {
        let effect = self
            .effects_by_key
            .get_mut(idempotency_key)
            .ok_or_else(|| {
                AppError::from_code(
                    ErrorCode::ValidationIdempotencyKey,
                    "contract-effect-key-missing",
                )
            })?;
        if effect.effect_id() != effect_id {
            return Err(AppError::from_code(
                ErrorCode::ConflictEffectAlreadyReported,
                "contract-effect-identity-conflict",
            ));
        }
        effect.report(reported)
    }

    #[must_use]
    pub fn status(&self, idempotency_key: &str) -> Option<EffectStatus> {
        self.effects_by_key
            .get(idempotency_key)
            .map(PlatformEffect::status)
    }
}

fn validate_utc(value: &str) -> Result<String, AppError> {
    normalize_rfc3339_utc(value).ok_or_else(|| {
        AppError::from_code(ErrorCode::ValidationRfc3339Utc, "contract-time-validation")
    })
}

pub(crate) fn normalize_rfc3339_utc(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.len() > 69
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !matches!(bytes.get(10), Some(b'T' | b't'))
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }
    if !bytes[..19]
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13 | 16) || byte.is_ascii_digit())
    {
        return None;
    }
    let (time_suffix, utc_suffix_len) = if matches!(bytes.last(), Some(b'Z' | b'z')) {
        (&bytes[19..bytes.len() - 1], 1)
    } else if bytes.ends_with(b"+00:00") {
        (&bytes[19..bytes.len() - 6], 6)
    } else {
        return None;
    };
    let year = parse_digits(&bytes[0..4]);
    let month = parse_digits(&bytes[5..7]);
    let day = parse_digits(&bytes[8..10]);
    let hour = parse_digits(&bytes[11..13]);
    let minute = parse_digits(&bytes[14..16]);
    let second = parse_digits(&bytes[17..19]);
    let valid_fraction = time_suffix.is_empty()
        || (time_suffix[0] == b'.'
            && time_suffix.len() > 1
            && time_suffix[1..].iter().all(u8::is_ascii_digit));
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    };
    let valid_leap_second = second <= 59
        || (second == 60
            && hour == 23
            && minute == 59
            && is_announced_utc_leap_second_date(year, month, day));
    if !(valid_fraction
        && day > 0
        && day <= days_in_month
        && hour <= 23
        && minute <= 59
        && valid_leap_second)
    {
        return None;
    }

    let canonical_len = bytes.len() - utc_suffix_len + 1;
    if canonical_len > 64 {
        return None;
    }
    let mut canonical = String::with_capacity(canonical_len);
    canonical.push_str(&value[..10]);
    canonical.push('T');
    canonical.push_str(&value[11..bytes.len() - utc_suffix_len]);
    canonical.push('Z');
    Some(canonical)
}

fn parse_digits(value: &[u8]) -> u32 {
    value
        .iter()
        .fold(0, |number, digit| number * 10 + u32::from(digit - b'0'))
}

const fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

const fn is_announced_utc_leap_second_date(year: u32, month: u32, day: u32) -> bool {
    matches!(
        (year, month, day),
        (
            1972 | 1981 | 1982 | 1983 | 1985 | 1992..=1994 | 1997 | 2012 | 2015,
            6,
            30,
        ) | (
            1972 | 1973..=1979 | 1987 | 1989 | 1990 | 1995 | 1998 | 2005 | 2008 | 2016,
            12,
            31,
        )
    )
}
