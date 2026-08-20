//! Canonical normalization, validation, risk evaluation, hashing, and receipts.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;

use sha2::{Digest, Sha256};
use url::Url;

use crate::contracts::dto::configuration_validation::{
    AttentionConfigurationV1, BlockingCodeV1, ConfigurationBlockingErrorV1,
    ConfigurationCandidateContext, ConfigurationNarrowingRiskV1, ConfigurationValidationReceiptV1,
    ConfigurationValidationResultV1, NarrowingRiskCodeV1,
};
use crate::contracts::effects::normalize_rfc3339_utc;

pub const VALIDATOR_VERSION: &str = "attention-configuration-v1";
pub const RECEIPT_TTL_MS: u64 = 5 * 60 * 1_000;
pub const MAX_RECEIPTS: usize = 64;
pub const MIN_TRACKS: usize = 1;
pub const MAX_TRACKS: usize = 32;
pub const MAX_TRACK_NAME_SCALARS: usize = 64;
pub const MAX_TRACK_ID_BYTES: usize = 128;
pub const MIN_SOURCE_PREFERENCES: usize = 1;
pub const MAX_SOURCE_PREFERENCES: usize = 64;
pub const MAX_SOURCE_IDENTIFIER_BYTES: usize = 2_048;
pub const MIN_REFRESH_MINUTES: u32 = 15;
pub const MAX_REFRESH_MINUTES: u32 = 10_080;
pub const MAX_PERCENT: u8 = 100;
pub const HIGH_TRUST_THRESHOLD: u8 = 80;
pub const MIN_NOTIFICATION_CAP: u8 = 1;
pub const MAX_NOTIFICATION_CAP: u8 = 100;
pub const MAX_EXPRESSION_BYTES: usize = 512;
pub const MAX_EXPRESSION_TERMS: usize = 32;
pub const MAX_TERM_SCALARS: usize = 64;
pub const MAX_EXPRESSION_DEPTH: u8 = 4;

#[derive(Clone)]
struct ReceiptEntry {
    hash: String,
    canonical_identity: Vec<u8>,
    risks: Vec<NarrowingRiskCodeV1>,
    expires_at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationRuntimeError {
    EntropyUnavailable,
}

enum EntropySource {
    System,
    Deterministic(VecDeque<[u8; 32]>),
}

pub struct ReceiptRegistry {
    now_ms: u64,
    entropy: EntropySource,
    order: VecDeque<String>,
    entries: HashMap<String, ReceiptEntry>,
}

impl Default for ReceiptRegistry {
    fn default() -> Self {
        Self {
            now_ms: 0,
            entropy: EntropySource::System,
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }
}

impl ReceiptRegistry {
    #[must_use]
    pub fn for_tests(now_ms: u64, entropy: Vec<[u8; 32]>) -> Self {
        Self {
            now_ms,
            entropy: EntropySource::Deterministic(entropy.into()),
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    pub fn set_test_time(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    pub fn set_time_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    fn issue(
        &mut self,
        hash: &str,
        canonical_identity: &[u8],
        risks: &[NarrowingRiskCodeV1],
    ) -> Result<ConfigurationValidationReceiptV1, ValidationRuntimeError> {
        self.evict_expired();
        while self.entries.len() >= MAX_RECEIPTS {
            let oldest = self
                .order
                .pop_front()
                .ok_or(ValidationRuntimeError::EntropyUnavailable)?;
            self.entries.remove(&oldest);
        }
        let mut bytes = [0_u8; 32];
        match &mut self.entropy {
            EntropySource::System => getrandom::fill(&mut bytes)
                .map_err(|_| ValidationRuntimeError::EntropyUnavailable)?,
            EntropySource::Deterministic(values) => {
                bytes = values
                    .pop_front()
                    .ok_or(ValidationRuntimeError::EntropyUnavailable)?;
            }
        }
        let token = base64url_no_pad(&bytes);
        self.entries.insert(
            token.clone(),
            ReceiptEntry {
                hash: hash.to_owned(),
                canonical_identity: canonical_identity.to_vec(),
                risks: risks.to_vec(),
                expires_at_ms: self.now_ms.saturating_add(RECEIPT_TTL_MS),
            },
        );
        self.order.push_back(token.clone());
        Ok(ConfigurationValidationReceiptV1 {
            token,
            normalized_config_hash: hash.to_owned(),
            validator_version: VALIDATOR_VERSION.to_owned(),
        })
    }

    #[must_use]
    pub fn consume(
        &mut self,
        receipt: &ConfigurationValidationReceiptV1,
        hash: &str,
        canonical_identity: &[u8],
        risks: &[NarrowingRiskCodeV1],
    ) -> bool {
        self.evict_expired();
        let Some(entry) = self.entries.get(&receipt.token) else {
            return false;
        };
        if entry.hash != hash
            || entry.canonical_identity != canonical_identity
            || entry.risks != risks
            || receipt.normalized_config_hash != hash
            || receipt.validator_version != VALIDATOR_VERSION
        {
            return false;
        }
        self.entries.remove(&receipt.token);
        self.order.retain(|token| token != &receipt.token);
        true
    }

    #[must_use]
    pub fn is_valid(
        &mut self,
        receipt: &ConfigurationValidationReceiptV1,
        hash: &str,
        canonical_identity: &[u8],
        risks: &[NarrowingRiskCodeV1],
    ) -> bool {
        self.evict_expired();
        self.entries.get(&receipt.token).is_some_and(|entry| {
            entry.hash == hash
                && entry.canonical_identity == canonical_identity
                && entry.risks == risks
                && receipt.normalized_config_hash == hash
                && receipt.validator_version == VALIDATOR_VERSION
        })
    }

    fn evict_expired(&mut self) {
        let now = self.now_ms;
        self.entries.retain(|_, entry| entry.expires_at_ms > now);
        self.order.retain(|token| self.entries.contains_key(token));
    }
}

#[must_use]
pub fn deterministic_entropy(seed: u8) -> Vec<[u8; 32]> {
    (0_u8..=64)
        .map(|offset| [seed.wrapping_add(offset); 32])
        .collect()
}

/// Validates a configuration and issues a receipt when confirmation is required.
///
/// # Errors
/// Returns [`ValidationRuntimeError::EntropyUnavailable`] when a narrowing-risk
/// receipt is required but the configured entropy source cannot provide one.
pub fn validate_configuration(
    configuration: &AttentionConfigurationV1,
    candidates: &ConfigurationCandidateContext,
    receipts: &mut ReceiptRegistry,
) -> Result<ConfigurationValidationResultV1, ValidationRuntimeError> {
    evaluate(configuration, candidates, Some(receipts))
}

#[must_use]
/// Assesses a configuration without issuing a confirmation receipt.
///
/// # Panics
/// Panics only if the receipt-free evaluation path unexpectedly requests entropy.
pub fn assess_configuration(
    configuration: &AttentionConfigurationV1,
    candidates: &ConfigurationCandidateContext,
) -> ConfigurationValidationResultV1 {
    evaluate(configuration, candidates, None).expect("assessment never requests entropy")
}

fn evaluate(
    configuration: &AttentionConfigurationV1,
    candidates: &ConfigurationCandidateContext,
    receipts: Option<&mut ReceiptRegistry>,
) -> Result<ConfigurationValidationResultV1, ValidationRuntimeError> {
    let normalized = normalize(configuration);
    let canonical_identity = configuration_identity(&normalized);
    let hash = hash_identity(&canonical_identity);
    let blocking_errors = blocking_errors(&normalized);
    let mut narrowing_risks = Vec::new();
    if blocking_errors.is_empty() {
        if normalized
            .source_preferences
            .iter()
            .all(|source| !source.enabled)
        {
            narrowing_risks.push(risk(NarrowingRiskCodeV1::AllSourcesDisabled));
        }
        if high_trust_candidates_filtered(&normalized, candidates) {
            narrowing_risks.push(risk(NarrowingRiskCodeV1::AllHighTrustCandidatesFiltered));
        }
    }
    let risk_codes: Vec<_> = narrowing_risks.iter().map(|risk| risk.code).collect();
    let validation_receipt = if blocking_errors.is_empty() && !risk_codes.is_empty() {
        match receipts {
            Some(registry) => Some(registry.issue(&hash, &canonical_identity, &risk_codes)?),
            None => None,
        }
    } else {
        None
    };
    Ok(ConfigurationValidationResultV1 {
        contract_version: 1,
        blocking_errors,
        narrowing_risks,
        validator_version: VALIDATOR_VERSION.to_owned(),
        normalized_config_hash: hash,
        validation_receipt,
    })
}

#[must_use]
pub fn normalize(configuration: &AttentionConfigurationV1) -> AttentionConfigurationV1 {
    let mut value = configuration.clone();
    for track in &mut value.tracks {
        track.id = track.id.trim().to_owned();
        track.name = track.name.trim().to_owned();
    }
    value
        .tracks
        .sort_by(|left, right| left.id.as_bytes().cmp(right.id.as_bytes()));
    for source in &mut value.source_preferences {
        let source_kind = source.source_kind.trim().to_ascii_lowercase();
        canonical_source_kind(&source_kind)
            .unwrap_or(source_kind.as_str())
            .clone_into(&mut source.source_kind);
        let identifier = source.identifier.trim();
        source.identifier =
            canonical_source_identifier(identifier).unwrap_or_else(|| identifier.to_owned());
    }
    value.source_preferences.sort_by(|left, right| {
        (&left.source_kind, &left.identifier).cmp(&(&right.source_kind, &right.identifier))
    });
    value.include_expression = value.include_expression.trim().to_owned();
    value.exclude_expression = value.exclude_expression.trim().to_owned();
    value
}

pub(crate) fn configuration_is_valid(configuration: &AttentionConfigurationV1) -> bool {
    blocking_errors(&normalize(configuration)).is_empty()
}

pub(crate) fn canonical_source_kind(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "rss" | "rss_atom" => Some("rss"),
        "github" => Some("github"),
        "arxiv" => Some("arxiv"),
        _ => None,
    }
}

pub(crate) fn canonical_source_identifier(value: &str) -> Option<String> {
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.has_host()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    if (url.scheme() == "https" && url.port() == Some(443))
        || (url.scheme() == "http" && url.port() == Some(80))
    {
        url.set_port(None).ok()?;
    }
    Some(url.to_string())
}

#[must_use]
/// Computes the lowercase SHA-256 identity of canonical JSON.
///
/// # Panics
/// Panics only if the closed, non-floating configuration DTO unexpectedly stops serializing.
pub fn configuration_hash(configuration: &AttentionConfigurationV1) -> String {
    hash_identity(&configuration_identity(configuration))
}

#[must_use]
/// Serializes the normalized configuration into its canonical identity bytes.
///
/// # Panics
/// Panics only if the closed, non-floating configuration DTO unexpectedly stops
/// serializing.
pub fn configuration_identity(configuration: &AttentionConfigurationV1) -> Vec<u8> {
    serde_json::to_vec(configuration).expect("configuration DTO is serializable")
}

fn hash_identity(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[allow(clippy::too_many_lines)] // Deterministic field-order validation remains a single auditable pass over the closed V1 DTO.
fn blocking_errors(configuration: &AttentionConfigurationV1) -> Vec<ConfigurationBlockingErrorV1> {
    let mut errors = Vec::new();
    if configuration.contract_version != 1
        || !(MIN_TRACKS..=MAX_TRACKS).contains(&configuration.tracks.len())
    {
        errors.push(error("tracks", BlockingCodeV1::ValueOutOfRange));
    }
    let mut track_names = HashSet::new();
    let mut track_ids = HashSet::new();
    for (index, track) in configuration.tracks.iter().enumerate() {
        let folded = track.name.to_lowercase();
        let id_invalid = track.id.is_empty()
            || track.id.len() > MAX_TRACK_ID_BYTES
            || !track.id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
            || !track_ids.insert(track.id.as_str());
        if id_invalid {
            errors.push(error(
                &format!("tracks[{index}].id"),
                BlockingCodeV1::ValueOutOfRange,
            ));
        }
        if !(1..=MAX_TRACK_NAME_SCALARS).contains(&track.name.chars().count())
            || !track_names.insert(folded)
        {
            errors.push(error(
                &format!("tracks[{index}].name"),
                BlockingCodeV1::ValueOutOfRange,
            ));
        }
    }
    for (field, expression) in [
        (
            "include_expression",
            configuration.include_expression.as_str(),
        ),
        (
            "exclude_expression",
            configuration.exclude_expression.as_str(),
        ),
    ] {
        if !expression_is_valid(expression) {
            errors.push(error(field, BlockingCodeV1::ExpressionUnparseable));
        }
    }
    if !(MIN_SOURCE_PREFERENCES..=MAX_SOURCE_PREFERENCES)
        .contains(&configuration.source_preferences.len())
    {
        errors.push(error("source_preferences", BlockingCodeV1::ValueOutOfRange));
    }
    let mut source_identities = HashSet::new();
    for (index, source) in configuration.source_preferences.iter().enumerate() {
        let identifier_path = format!("source_preferences[{index}].identifier");
        let source_kind_path = format!("source_preferences[{index}].source_kind");
        let trust_path = format!("source_preferences[{index}].trust");
        let identifier_out_of_range =
            source.identifier.is_empty() || source.identifier.len() > MAX_SOURCE_IDENTIFIER_BYTES;
        let duplicate_identity =
            !source_identities.insert((source.source_kind.as_str(), source.identifier.as_str()));
        if identifier_out_of_range || duplicate_identity {
            errors.push(error(&identifier_path, BlockingCodeV1::ValueOutOfRange));
        }
        if source.trust > MAX_PERCENT {
            errors.push(error(&trust_path, BlockingCodeV1::ValueOutOfRange));
        }
        if !matches!(source.source_kind.as_str(), "rss" | "github" | "arxiv") {
            errors.push(error(
                &source_kind_path,
                BlockingCodeV1::InvalidSourceOrUnsupportedProtocol,
            ));
        } else if !identifier_out_of_range && !valid_http_source_identifier(&source.identifier) {
            errors.push(error(
                &identifier_path,
                BlockingCodeV1::InvalidSourceOrUnsupportedProtocol,
            ));
        }
    }
    if !(MIN_REFRESH_MINUTES..=MAX_REFRESH_MINUTES)
        .contains(&configuration.refresh_interval_minutes)
    {
        errors.push(error(
            "refresh_interval_minutes",
            BlockingCodeV1::ValueOutOfRange,
        ));
    }
    for (field_path, value) in [
        ("minimum_trust", configuration.minimum_trust),
        ("maximum_trust", configuration.maximum_trust),
        ("alert_threshold", configuration.alert_threshold),
    ] {
        if value > MAX_PERCENT {
            errors.push(error(field_path, BlockingCodeV1::ValueOutOfRange));
        }
    }
    if configuration.minimum_trust > configuration.maximum_trust {
        errors.push(error(
            "minimum_trust",
            BlockingCodeV1::LowerBoundAboveUpperBound,
        ));
    }
    if !valid_quiet_hours(configuration)
        || !valid_frequency(configuration)
        || !valid_active_window(configuration)
    {
        errors.push(error("schedule", BlockingCodeV1::ValueOutOfRange));
    } else if configuration
        .active_from
        .as_ref()
        .zip(configuration.active_until.as_ref())
        .is_some_and(|(from, until)| {
            instant_sort_key(from)
                .zip(instant_sort_key(until))
                .is_some_and(|(from_key, until_key)| from_key > until_key)
        })
    {
        errors.push(error(
            "active_from",
            BlockingCodeV1::LowerBoundAboveUpperBound,
        ));
    }
    errors.sort_by(|left, right| {
        left.field_path
            .as_bytes()
            .cmp(right.field_path.as_bytes())
            .then_with(|| code_name(left.code).cmp(code_name(right.code)))
    });
    errors
}

fn valid_quiet_hours(configuration: &AttentionConfigurationV1) -> bool {
    let valid_time = |value: &str| {
        value.len() == 5
            && value.as_bytes()[2] == b':'
            && value[..2].parse::<u8>().is_ok_and(|hour| hour < 24)
            && value[3..].parse::<u8>().is_ok_and(|minute| minute < 60)
    };
    valid_time(&configuration.quiet_hours.start)
        && valid_time(&configuration.quiet_hours.end)
        && (!configuration.quiet_hours.enabled
            || configuration.quiet_hours.start != configuration.quiet_hours.end)
}

fn valid_frequency(configuration: &AttentionConfigurationV1) -> bool {
    match (
        configuration.notification_frequency.enabled,
        configuration.notification_frequency.max_per_24h,
    ) {
        (true, Some(value)) => (MIN_NOTIFICATION_CAP..=MAX_NOTIFICATION_CAP).contains(&value),
        (false, None) => true,
        _ => false,
    }
}

fn valid_active_window(configuration: &AttentionConfigurationV1) -> bool {
    configuration
        .active_from
        .iter()
        .chain(configuration.active_until.iter())
        .all(|value| normalize_rfc3339_utc(value).as_deref() == Some(value.as_str()))
}

fn instant_sort_key(value: &str) -> Option<String> {
    if normalize_rfc3339_utc(value).as_deref() != Some(value) {
        return None;
    }
    let body = value.strip_suffix('Z')?;
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let mut key = String::with_capacity(19 + 49);
    key.push_str(whole);
    key.push_str(fraction);
    key.extend(std::iter::repeat_n(
        '0',
        49_usize.checked_sub(fraction.len())?,
    ));
    Some(key)
}

fn valid_http_source_identifier(identifier: &str) -> bool {
    canonical_source_identifier(identifier).is_some()
}

fn high_trust_candidates_filtered(
    configuration: &AttentionConfigurationV1,
    context: &ConfigurationCandidateContext,
) -> bool {
    let before = context
        .real_candidates
        .iter()
        .filter(|candidate| {
            configuration.source_preferences.iter().any(|source| {
                canonical_source_kind(&source.source_kind)
                    == canonical_source_kind(&candidate.source_kind)
                    && source.trust >= HIGH_TRUST_THRESHOLD
            })
        })
        .count();
    if before == 0 {
        return false;
    }
    let after = context
        .real_candidates
        .iter()
        .filter(|candidate| {
            configuration.source_preferences.iter().any(|source| {
                source.enabled
                    && canonical_source_kind(&source.source_kind)
                        == canonical_source_kind(&candidate.source_kind)
                    && source.trust >= HIGH_TRUST_THRESHOLD
                    && (configuration.include_expression.is_empty()
                        || expression_matches(
                            &configuration.include_expression,
                            &candidate.searchable_text,
                        ))
                    && !expression_matches(
                        &configuration.exclude_expression,
                        &candidate.searchable_text,
                    )
            })
        })
        .count();
    after == 0
}

pub(crate) fn expression_matches(expression: &str, text: &str) -> bool {
    if expression.trim().is_empty() {
        return false;
    }
    let folded = text.to_lowercase();
    parse_expression(expression).is_some_and(|expression| expression.evaluate(&folded))
}

fn expression_is_valid(expression: &str) -> bool {
    if expression.is_empty() {
        return true;
    }
    parse_expression(expression).is_some()
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExpressionToken {
    Term(String),
    And,
    Or,
    Left,
    Right,
}

fn expression_tokens(expression: &str) -> Option<Vec<ExpressionToken>> {
    if expression.len() > MAX_EXPRESSION_BYTES {
        return None;
    }
    let mut tokens = Vec::new();
    let mut term_count = 0_usize;
    let mut characters = expression.chars().peekable();
    while let Some(character) = characters.next() {
        if character.is_whitespace() {
            continue;
        }
        match character {
            '(' => tokens.push(ExpressionToken::Left),
            ')' => tokens.push(ExpressionToken::Right),
            '"' => {
                let mut term = String::new();
                let mut closed = false;
                while let Some(inner) = characters.next() {
                    if inner == '"' {
                        closed = true;
                        break;
                    }
                    if inner == '\\' {
                        let escaped = characters.next()?;
                        if !matches!(escaped, '\\' | '"') {
                            return None;
                        }
                        term.push(escaped);
                        continue;
                    }
                    term.push(inner);
                }
                if !closed || term.is_empty() || term.chars().count() > MAX_TERM_SCALARS {
                    return None;
                }
                term_count += 1;
                tokens.push(ExpressionToken::Term(term));
            }
            _ => {
                let mut word = String::from(character);
                while characters
                    .peek()
                    .is_some_and(|next| !next.is_whitespace() && !matches!(next, '(' | ')'))
                {
                    word.push(characters.next()?);
                }
                tokens.push(if word.eq_ignore_ascii_case("AND") {
                    ExpressionToken::And
                } else if word.eq_ignore_ascii_case("OR") {
                    ExpressionToken::Or
                } else if word.eq_ignore_ascii_case("NOT") || word.contains(['\\', '"']) {
                    return None;
                } else {
                    if word.chars().count() > MAX_TERM_SCALARS {
                        return None;
                    }
                    term_count += 1;
                    ExpressionToken::Term(word)
                });
            }
        }
    }
    (term_count > 0 && term_count <= MAX_EXPRESSION_TERMS).then_some(tokens)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Expression {
    Term(String),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
}

impl Expression {
    fn evaluate(&self, folded_text: &str) -> bool {
        match self {
            Self::Term(term) => folded_text.contains(&term.to_lowercase()),
            Self::And(left, right) => left.evaluate(folded_text) && right.evaluate(folded_text),
            Self::Or(left, right) => left.evaluate(folded_text) || right.evaluate(folded_text),
        }
    }
}

fn parse_expression(expression: &str) -> Option<Expression> {
    let tokens = expression_tokens(expression)?;
    let mut cursor = 0;
    let parsed = parse_or(&tokens, &mut cursor, 0)?;
    (cursor == tokens.len()).then_some(parsed)
}

fn parse_or(tokens: &[ExpressionToken], cursor: &mut usize, depth: u8) -> Option<Expression> {
    let mut expression = parse_and(tokens, cursor, depth)?;
    while tokens.get(*cursor) == Some(&ExpressionToken::Or) {
        *cursor += 1;
        let right = parse_and(tokens, cursor, depth)?;
        expression = Expression::Or(Box::new(expression), Box::new(right));
    }
    Some(expression)
}

fn parse_and(tokens: &[ExpressionToken], cursor: &mut usize, depth: u8) -> Option<Expression> {
    let mut expression = parse_factor(tokens, cursor, depth)?;
    while tokens.get(*cursor) == Some(&ExpressionToken::And) {
        *cursor += 1;
        let right = parse_factor(tokens, cursor, depth)?;
        expression = Expression::And(Box::new(expression), Box::new(right));
    }
    Some(expression)
}

fn parse_factor(tokens: &[ExpressionToken], cursor: &mut usize, depth: u8) -> Option<Expression> {
    match tokens.get(*cursor) {
        Some(ExpressionToken::Term(term)) => {
            *cursor += 1;
            Some(Expression::Term(term.clone()))
        }
        Some(ExpressionToken::Left) if depth < MAX_EXPRESSION_DEPTH => {
            *cursor += 1;
            let expression = parse_or(tokens, cursor, depth + 1)?;
            if tokens.get(*cursor) != Some(&ExpressionToken::Right) {
                return None;
            }
            *cursor += 1;
            Some(expression)
        }
        _ => None,
    }
}

fn error(field_path: &str, code: BlockingCodeV1) -> ConfigurationBlockingErrorV1 {
    ConfigurationBlockingErrorV1 {
        field_path: field_path.to_owned(),
        code,
        message_key: format!("configuration.fix.{}", code_name(code)),
    }
}

const fn code_name(code: BlockingCodeV1) -> &'static str {
    code.as_str()
}

fn risk(code: NarrowingRiskCodeV1) -> ConfigurationNarrowingRiskV1 {
    let name = code.as_str();
    ConfigurationNarrowingRiskV1 {
        code,
        condition_key: format!("configuration.risk.{name}.condition"),
        consequence_key: format!("configuration.risk.{name}.consequence"),
    }
}

fn base64url_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((value >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(value & 63) as usize] as char);
        }
    }
    output
}
