//! Source-neutral final intelligence facts and provenance contracts.

mod association;
mod normalized_item;
mod provenance;

pub(crate) use association::association_id_from_basis_hash;
pub use association::{
    AssociationEvidence, AssociationEvidenceBasis, AssociationRelationType, IntelAssociationId,
    association_for_normalized_url,
};
pub use normalized_item::{
    IntelItemId, NormalizationIssue, NormalizationIssueCode, NormalizedIntelCandidate,
    canonical_external_id, derive_intel_item_id, normalize_rss_candidate,
};
pub use provenance::{AuthorAvailability, NormalizedWarning};
