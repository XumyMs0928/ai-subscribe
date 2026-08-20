//! Deterministic, evidence-keyed intelligence associations.

use std::fmt::Write;

use sha2::{Digest, Sha256};

use super::normalized_item::canonical_item_url;

const ASSOCIATION_BASIS_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntelAssociationId(String);

impl IntelAssociationId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationRelationType {
    SameEvent,
}

impl AssociationRelationType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "same_event"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationEvidenceBasis {
    NormalizedOriginalUrl,
}

impl AssociationEvidenceBasis {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "normalized_original_url"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssociationEvidence {
    pub association_id: IntelAssociationId,
    pub relation: AssociationRelationType,
    pub basis: AssociationEvidenceBasis,
    pub basis_version: u16,
    pub basis_hash: String,
}

/// Creates the only association evidence allowed in the RSS-only phase.
/// The input must already be the exact canonical HTTPS URL emitted by normalization.
#[must_use]
pub fn association_for_normalized_url(value: &str) -> Option<AssociationEvidence> {
    if canonical_item_url(value, "association-evidence")
        .ok()
        .as_deref()
        != Some(value)
    {
        return None;
    }
    let relation = AssociationRelationType::SameEvent;
    let basis = AssociationEvidenceBasis::NormalizedOriginalUrl;
    let basis_hash = digest_hex(value.as_bytes());
    let association_id =
        association_id_from_basis_hash(relation, basis, ASSOCIATION_BASIS_VERSION, &basis_hash)?;
    Some(AssociationEvidence {
        association_id,
        relation,
        basis,
        basis_version: ASSOCIATION_BASIS_VERSION,
        basis_hash,
    })
}

#[must_use]
pub(crate) fn association_id_from_basis_hash(
    relation: AssociationRelationType,
    basis: AssociationEvidenceBasis,
    basis_version: u16,
    basis_hash: &str,
) -> Option<IntelAssociationId> {
    if basis_version != ASSOCIATION_BASIS_VERSION
        || basis_hash.len() != 64
        || !basis_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return None;
    }
    Some(IntelAssociationId(format!(
        "assoc:{}",
        digest_hex(
            format!(
                "ai-subscribe:association:v1\0{}\0{}\0{}\0{basis_hash}",
                relation.as_str(),
                basis.as_str(),
                basis_version
            )
            .as_bytes()
        )
    )))
}

fn digest_hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_url_association_is_stable_and_typed() {
        let first = association_for_normalized_url("https://example.com/release?a=1").unwrap();
        let replay = association_for_normalized_url("https://example.com/release?a=1").unwrap();

        assert_eq!(first, replay);
        assert_eq!(first.relation, AssociationRelationType::SameEvent);
        assert_eq!(first.basis, AssociationEvidenceBasis::NormalizedOriginalUrl);
        assert_eq!(first.basis_version, 1);
        assert_eq!(first.basis_hash.len(), 64);
        assert_eq!(first.association_id.as_str().len(), 70);
    }

    #[test]
    fn noncanonical_or_unsafe_shape_is_rejected() {
        for value in [
            "http://example.com/item",
            "https://example.com:443/item",
            "https://example.com/item#fragment",
            "https://user@example.com/item",
            "https://localhost/item",
            "https://127.0.0.1/item",
            "not-a-url",
        ] {
            assert!(association_for_normalized_url(value).is_none(), "{value}");
        }
    }

    #[test]
    fn distinct_url_evidence_never_collapses_or_forms_a_transitive_group() {
        let a = association_for_normalized_url("https://example.com/event-a").unwrap();
        let b = association_for_normalized_url("https://example.com/event-b").unwrap();
        let c = association_for_normalized_url("https://example.com/event-c").unwrap();

        assert_ne!(a.association_id, b.association_id);
        assert_ne!(b.association_id, c.association_id);
        assert_ne!(a.association_id, c.association_id);
    }
}
