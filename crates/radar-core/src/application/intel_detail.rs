//! Read-only real RSS evidence-detail query.

use super::configuration::read_configuration;
use super::demo::DemoStore;
use crate::contracts::dto::intel_detail::{
    AiEvidenceStatusV1, AssociationEvidenceStatusV1, AssociationEvidenceV1,
    INTEL_DETAIL_MAX_PROVENANCE, INTEL_DETAIL_MAX_SUMMARY_CHARS, INTEL_DETAIL_MAX_TEXT_CHARS,
    IntelEvidenceDetailV1, IntelProvenanceV1, ProvenanceRoleV1, QueryIntelEvidenceDetailInputV1,
    RuleEvidenceStatusV1, RuleExplanationV1,
};
use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::rules::intelligence_value::{
    FilterReasonV1, IMPORTANCE_HIGH_MIN, IMPORTANCE_MEDIUM_MIN, INTELLIGENCE_VALUE_RULE_VERSION,
    ImportanceV1, RuleFactorV1, StreamDispositionV1, parse_rfc3339_ms,
};
use crate::infrastructure::database::intel_detail_repository::{
    RuleRow, query_association, query_facts, query_rule, resolve_original_url,
};
use crate::infrastructure::http::source_http_policy::{
    canonicalize_public_https_url, validate_public_ips,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedOriginalLink {
    intel_item_id: String,
    provenance_id: String,
    url: String,
}

impl ValidatedOriginalLink {
    #[must_use]
    pub fn intel_item_id(&self) -> &str {
        &self.intel_item_id
    }

    #[must_use]
    pub fn provenance_id(&self) -> &str {
        &self.provenance_id
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl DemoStore {
    /// Returns one bounded, read-only evidence detail for a real RSS item.
    ///
    /// # Errors
    /// Returns a stable validation error for an invalid/missing identity and a storage error when
    /// the reliable fact/provenance projection cannot be read.
    pub fn query_intel_evidence_detail(
        &self,
        input: &QueryIntelEvidenceDetailInputV1,
    ) -> Result<IntelEvidenceDetailV1, AppError> {
        validate_input(input)?;
        let mut facts = query_facts(&self.connection, &input.intel_item_id)
            .map_err(|_| detail_error(ErrorCode::StorageSource, "detail-facts-read"))?
            .ok_or_else(|| detail_error(ErrorCode::NotFoundIntelDetail, "detail-not-found"))?;
        validate_facts(&facts.facts)?;
        if !normalize_provenance(&mut facts.primary)
            || !provenance_matches_facts(&facts.primary, &facts.facts)
        {
            return Err(detail_error(
                ErrorCode::StorageSource,
                "detail-primary-provenance",
            ));
        }
        let configuration = read_configuration(&self.connection)?;
        let rule_row = query_rule(&self.connection, facts.item_id)
            .map_err(|_| detail_error(ErrorCode::StorageSource, "detail-rule-read"))?;
        let (rule_status, rule_issue_code, rule) = rule_projection(
            rule_row,
            facts.facts.fact_revision,
            configuration.revision,
            &configuration.normalized_config_hash,
            configuration.configuration.alert_threshold,
        );
        let association = query_association(&self.connection, facts.item_id)
            .map_err(|_| detail_error(ErrorCode::StorageSource, "detail-association-read"))?;
        let (mut provenance, mut association) = association.map_or_else(
            || {
                (
                    vec![facts.primary],
                    AssociationEvidenceV1 {
                        status: AssociationEvidenceStatusV1::Complete,
                        issue_code: None,
                        relation_type: None,
                        evidence_basis: None,
                        basis_version: None,
                    },
                )
            },
            |association| (association.provenance, association.evidence),
        );
        if provenance.first_mut().is_none_or(|primary| {
            !normalize_provenance(primary) || !provenance_matches_facts(primary, &facts.facts)
        }) {
            return Err(detail_error(
                ErrorCode::StorageSource,
                "detail-primary-provenance",
            ));
        }
        let mut associated_invalid = false;
        provenance.retain_mut(|entry| {
            if entry.role == ProvenanceRoleV1::Primary {
                true
            } else {
                let valid = normalize_provenance(entry);
                associated_invalid |= !valid;
                valid
            }
        });
        if associated_invalid {
            association.status = AssociationEvidenceStatusV1::Incomplete;
            association.issue_code = Some("association.incomplete".to_owned());
        }
        if provenance.is_empty()
            || provenance.len() > INTEL_DETAIL_MAX_PROVENANCE
            || provenance[0].intel_item_id != input.intel_item_id
        {
            return Err(detail_error(
                ErrorCode::StorageSource,
                "detail-provenance-identity",
            ));
        }
        Ok(IntelEvidenceDetailV1 {
            contract_version: 1,
            facts: facts.facts,
            rule_status,
            rule_issue_code,
            rule,
            ai_status: AiEvidenceStatusV1::Unavailable,
            provenance,
            association,
        })
    }

    /// Resolves and revalidates one persisted provenance URL without performing network I/O.
    ///
    /// # Errors
    /// Returns a stable validation error for invalid IDs, unavailable/non-member provenance, or
    /// a URL that violates the Phase 1 canonical HTTPS/literal-host policy.
    pub fn resolve_intel_original(
        &self,
        input: &crate::contracts::dto::intel_detail::OpenIntelOriginalInputV1,
    ) -> Result<ValidatedOriginalLink, AppError> {
        validate_input(&QueryIntelEvidenceDetailInputV1 {
            contract_version: input.contract_version,
            intel_item_id: input.intel_item_id.clone(),
        })?;
        if !safe_id(&input.provenance_id) {
            return Err(detail_error(
                ErrorCode::ValidationSource,
                "original-provenance-id",
            ));
        }
        let raw =
            resolve_original_url(&self.connection, &input.intel_item_id, &input.provenance_id)
                .map_err(|_| detail_error(ErrorCode::StorageSource, "original-resolve"))?
                .ok_or_else(|| {
                    detail_error(ErrorCode::ValidationSource, "original-not-available")
                })?;
        let canonical = validate_original_url(&raw)?;
        Ok(ValidatedOriginalLink {
            intel_item_id: input.intel_item_id.clone(),
            provenance_id: input.provenance_id.clone(),
            url: canonical,
        })
    }
}

fn validate_original_url(raw: &str) -> Result<String, AppError> {
    let parsed = url::Url::parse(raw)
        .map_err(|_| detail_error(ErrorCode::ValidationSource, "original-url-parse"))?;
    if parsed.fragment().is_some() {
        return Err(detail_error(
            ErrorCode::ValidationSource,
            "original-url-fragment",
        ));
    }
    let canonical = canonicalize_public_https_url(raw)?;
    if canonical.as_str() != raw {
        return Err(detail_error(
            ErrorCode::ValidationSource,
            "original-url-not-canonical",
        ));
    }
    match canonical.host() {
        Some(url::Host::Ipv4(ip)) => validate_public_ips(&[ip.into()])?,
        Some(url::Host::Ipv6(ip)) => validate_public_ips(&[ip.into()])?,
        Some(url::Host::Domain(host))
            if host.eq_ignore_ascii_case("localhost")
                || host.to_ascii_lowercase().ends_with(".localhost") =>
        {
            return Err(detail_error(
                ErrorCode::ValidationSource,
                "original-host-policy",
            ));
        }
        Some(url::Host::Domain(_)) => {}
        None => return Err(detail_error(ErrorCode::ValidationSource, "original-host")),
    }
    Ok(canonical.into())
}

fn validate_input(input: &QueryIntelEvidenceDetailInputV1) -> Result<(), AppError> {
    if input.contract_version != 1
        || input.intel_item_id.len() != 70
        || !input.intel_item_id.starts_with("intel:")
        || !input.intel_item_id[6..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(detail_error(ErrorCode::ValidationSource, "detail-input"));
    }
    Ok(())
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn validate_facts(
    facts: &crate::contracts::dto::intel_detail::SourceFactsV1,
) -> Result<(), AppError> {
    if facts.fact_revision == 0
        || facts.content_hash.len() != 64
        || facts.publisher.chars().count() > INTEL_DETAIL_MAX_TEXT_CHARS
        || facts.title.chars().count() > INTEL_DETAIL_MAX_TEXT_CHARS
        || facts
            .source_summary
            .as_ref()
            .is_some_and(|value| value.chars().count() > INTEL_DETAIL_MAX_SUMMARY_CHARS)
    {
        return Err(detail_error(ErrorCode::StorageSource, "detail-fact-bounds"));
    }
    Ok(())
}

fn normalize_provenance(provenance: &mut IntelProvenanceV1) -> bool {
    let valid_time = |value: &str| parse_rfc3339_ms(value).is_some();
    if !safe_id(&provenance.provenance_id)
        || !safe_id(&provenance.source_id)
        || validate_input(&QueryIntelEvidenceDetailInputV1 {
            contract_version: 1,
            intel_item_id: provenance.intel_item_id.clone(),
        })
        .is_err()
        || provenance.source_kind != "rss_atom"
        || provenance.publisher.is_empty()
        || provenance.publisher.chars().count() > INTEL_DETAIL_MAX_TEXT_CHARS
        || provenance.author.as_ref().is_some_and(|value| {
            value.is_empty() || value.chars().count() > INTEL_DETAIL_MAX_TEXT_CHARS
        })
        || !matches!(
            provenance.author_availability.as_str(),
            "available" | "unavailable"
        )
        || provenance.original_title.is_empty()
        || provenance.original_title.chars().count() > INTEL_DETAIL_MAX_TEXT_CHARS
        || provenance.display_url.chars().count() > INTEL_DETAIL_MAX_TEXT_CHARS
        || provenance
            .published_at
            .as_deref()
            .is_some_and(|value| !valid_time(value))
        || !valid_time(&provenance.collected_at)
        || !valid_time(&provenance.first_discovered_at)
        || !valid_time(&provenance.last_updated_at)
        || !matches!(
            provenance.availability_status.as_str(),
            "available" | "unavailable"
        )
    {
        return false;
    }
    if let Ok(canonical) = validate_original_url(&provenance.display_url) {
        provenance.display_url = canonical;
        provenance.can_open_original = provenance.availability_status == "available";
    } else {
        "原文地址不可用".clone_into(&mut provenance.display_url);
        provenance.can_open_original = false;
    }
    true
}

fn provenance_matches_facts(
    provenance: &IntelProvenanceV1,
    facts: &crate::contracts::dto::intel_detail::SourceFactsV1,
) -> bool {
    provenance.role == ProvenanceRoleV1::Primary
        && provenance.intel_item_id == facts.intel_item_id
        && provenance.publisher == facts.publisher
        && provenance.original_title == facts.title
        && provenance.published_at == facts.published_at
        && provenance.collected_at == facts.collected_at
}

fn rule_projection(
    row: Option<RuleRow>,
    fact_revision: u64,
    configuration_revision: u64,
    configuration_hash: &str,
    alert_threshold: u8,
) -> (
    RuleEvidenceStatusV1,
    Option<String>,
    Option<RuleExplanationV1>,
) {
    let Some(row) = row else {
        return (
            RuleEvidenceStatusV1::Unavailable,
            Some("rule.unavailable".to_owned()),
            None,
        );
    };
    let identity_is_current = row.rule_version.as_deref() == Some(INTELLIGENCE_VALUE_RULE_VERSION)
        && row.configuration_revision == Some(configuration_revision)
        && row.configuration_hash.as_deref() == Some(configuration_hash)
        && row.fact_revision == Some(fact_revision)
        && row.ai_status.as_deref() == Some("unavailable");
    if !identity_is_current {
        return (
            RuleEvidenceStatusV1::Stale,
            Some("rule.stale".to_owned()),
            None,
        );
    }
    parse_rule(row, alert_threshold).map_or_else(
        || {
            (
                RuleEvidenceStatusV1::Stale,
                Some("rule.malformed".to_owned()),
                None,
            )
        },
        |rule| (RuleEvidenceStatusV1::Current, None, Some(rule)),
    )
}

fn parse_rule(row: RuleRow, alert_threshold: u8) -> Option<RuleExplanationV1> {
    let matched_track_ids: Vec<String> =
        serde_json::from_str(row.matched_tracks_json.as_deref()?).ok()?;
    let factors: Vec<RuleFactorV1> = serde_json::from_str(row.factors_json.as_deref()?).ok()?;
    let filter_reasons: Vec<FilterReasonV1> =
        serde_json::from_str(row.filter_reasons_json.as_deref()?).ok()?;
    let score = row.score?;
    let importance = match row.importance.as_str() {
        "low" => ImportanceV1::Low,
        "medium" => ImportanceV1::Medium,
        "high" => ImportanceV1::High,
        _ => return None,
    };
    let disposition = match row.disposition.as_deref()? {
        "high_value" => StreamDispositionV1::HighValue,
        "ordinary_candidate" => StreamDispositionV1::OrdinaryCandidate,
        _ => return None,
    };
    let expected_importance = if score >= IMPORTANCE_HIGH_MIN {
        ImportanceV1::High
    } else if score >= IMPORTANCE_MEDIUM_MIN {
        ImportanceV1::Medium
    } else {
        ImportanceV1::Low
    };
    let expected_disposition = if score >= alert_threshold && filter_reasons.is_empty() {
        StreamDispositionV1::HighValue
    } else {
        StreamDispositionV1::OrdinaryCandidate
    };
    if matched_track_ids.len() > 32
        || !is_sorted_unique_bounded(&matched_track_ids, 128)
        || factors.len() > 16
        || filter_reasons.len() > 32
        || factors.iter().any(|factor| {
            factor.points > 100 || !is_sorted_unique_bounded(&factor.reason_codes, 128)
        })
        || filter_reasons.iter().any(|reason| !safe_id(&reason.code))
        || row.configuration_hash.as_deref().is_none_or(|value| {
            value.len() != 64
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        })
        || row.evaluated_at_ms.is_none_or(|value| value == 0)
        || importance != expected_importance
        || disposition != expected_disposition
    {
        return None;
    }
    Some(RuleExplanationV1 {
        rule_version: row.rule_version?,
        configuration_revision: row.configuration_revision?,
        configuration_hash: row.configuration_hash?,
        evaluated_at_ms: row.evaluated_at_ms?,
        score,
        importance,
        disposition,
        matched_track_ids,
        factors,
        filter_reasons,
    })
}

fn is_sorted_unique_bounded(values: &[String], maximum_chars: usize) -> bool {
    values
        .iter()
        .all(|value| safe_id(value) && value.chars().count() <= maximum_chars)
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn detail_error(code: ErrorCode, boundary: &'static str) -> AppError {
    AppError::from_code(code, boundary)
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::super::configuration::read_configuration;
    use super::super::demo::DemoStore;
    use super::validate_original_url;
    use crate::contracts::dto::intel_detail::{
        AssociationEvidenceStatusV1, OpenIntelOriginalInputV1, QueryIntelEvidenceDetailInputV1,
        RuleEvidenceStatusV1,
    };
    use crate::domain::rules::intelligence_value::INTELLIGENCE_VALUE_RULE_VERSION;

    const ITEM_A: &str = "intel:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ITEM_B: &str = "intel:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn seed_detail(store: &mut DemoStore, with_rule: bool) {
        let configuration = read_configuration(&store.connection).expect("configuration");
        let tx = store.connection.transaction().expect("detail seed");
        for (index, intel_item_id, source_id, provenance_id, publisher) in [
            (
                1_i64,
                ITEM_A,
                "source:aaaaaaaaaaaaaaaaaaaaaaaa",
                "prov:aaaaaaaaaaaaaaaaaaaaaaaa",
                "Primary Publisher",
            ),
            (
                2_i64,
                ITEM_B,
                "source:bbbbbbbbbbbbbbbbbbbbbbbb",
                "prov:bbbbbbbbbbbbbbbbbbbbbbbb",
                "Associated Publisher",
            ),
        ] {
            let hash = format!("{index:064x}");
            let url = "https://evidence.example/releases/runtime";
            tx.execute(
                "INSERT INTO sources(source_id,configuration_version,source_kind,canonical_url,enabled,revision,status,consecutive_failures,retryability,created_at_ms,updated_at_ms)
                 VALUES(?1,1,'rss_atom',?2,1,1,'ready',0,'manual',1,1)",
                params![source_id, format!("https://feed-{index}.example/rss")],
            ).expect("source");
            tx.execute(
                "INSERT INTO intel_items(intel_item_id,external_id,data_origin,source_id,source_kind,stable_external_id,content_hash,revision,publisher,title,original_url,published_at,collected_at)
                 VALUES(?1,?2,'real',?3,'rss_atom',?2,?4,2,?5,?6,?7,'2026-08-20T08:00:00Z','2026-08-20T08:05:00Z')",
                params![intel_item_id, format!("external-{index}"), source_id, hash, publisher, format!("Runtime release {index}"), url],
            ).expect("item");
            let item_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO intel_contents(item_id,what_happened,facts_json,source_summary,content_hash,content_state)
                 VALUES(?1,NULL,NULL,?2,?3,'metadata_only')",
                params![item_id, format!("Summary {index}"), hash],
            ).expect("content");
            tx.execute(
                "INSERT INTO item_provenance(provenance_id,item_id,source_id,stable_external_id,source_kind,publisher,author,author_availability,original_title,original_url,published_at,collected_at,first_discovered_at,last_updated_at,availability_status,warnings_json,content_hash,deterministic_association_basis)
                 VALUES(?1,?2,?3,?4,'rss_atom',?5,NULL,'unavailable',?6,?7,'2026-08-20T08:00:00Z','2026-08-20T08:05:00Z','2026-08-20T08:05:00Z','2026-08-20T08:05:00Z','available','[]',?8,'source_kind+canonical_external_id')",
                params![provenance_id, item_id, source_id, format!("external-{index}"), publisher, format!("Runtime release {index}"), url, hash],
            ).expect("provenance");
            if with_rule && index == 1 {
                tx.execute(
                    "INSERT INTO rule_evaluations(item_id,why_it_matters,possible_impact,importance,reasons_json,rule_version,configuration_revision,configuration_hash,fact_revision,evaluated_at_ms,score,stream_disposition,matched_tracks_json,factor_results_json,filter_reasons_json,ai_status)
                     VALUES(?1,'rules.value.explainable','rules.value.current_projection','high','[]',?2,?3,?4,2,1787213160000,90,'high_value','[\"ai_agents\"]','[{\"factor\":\"track\",\"points\":25,\"reason_codes\":[\"track.matched\"]}]','[]','unavailable')",
                    params![item_id, INTELLIGENCE_VALUE_RULE_VERSION, i64::try_from(configuration.revision).expect("revision"), configuration.normalized_config_hash],
                ).expect("rule");
            }
        }
        tx.execute(
            "INSERT INTO intel_associations(association_id,relation_type,evidence_basis,basis_version,basis_hash,first_observed_at,last_observed_at)
             VALUES('assoc:aaaaaaaaaaaaaaaaaaaaaaaa','same_event','normalized_original_url',1,'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','2026-08-20T08:05:00Z','2026-08-20T08:05:00Z')", [],
        ).expect("association");
        for item_id in [1_i64, 2_i64] {
            tx.execute(
                "INSERT INTO intel_association_members(association_id,item_id,first_observed_at,last_observed_at)
                 VALUES('assoc:aaaaaaaaaaaaaaaaaaaaaaaa',?1,'2026-08-20T08:05:00Z','2026-08-20T08:05:00Z')", [item_id],
            ).expect("member");
        }
        tx.commit().expect("seed commit");
    }

    fn input() -> QueryIntelEvidenceDetailInputV1 {
        QueryIntelEvidenceDetailInputV1 {
            contract_version: 1,
            intel_item_id: ITEM_A.to_owned(),
        }
    }

    #[test]
    fn returns_current_rule_and_independent_provenance_without_writes() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        let changes = store.connection.total_changes();
        let detail = store.query_intel_evidence_detail(&input()).expect("detail");
        assert_eq!(detail.rule_status, RuleEvidenceStatusV1::Current);
        assert_eq!(detail.provenance.len(), 2);
        assert_eq!(detail.provenance[0].intel_item_id, ITEM_A);
        assert_eq!(detail.provenance[1].intel_item_id, ITEM_B);
        assert_eq!(store.connection.total_changes(), changes);
    }

    #[test]
    fn missing_rule_is_local_unavailable_not_a_blocking_detail_error() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, false);
        let detail = store
            .query_intel_evidence_detail(&input())
            .expect("facts remain readable");
        assert_eq!(detail.rule_status, RuleEvidenceStatusV1::Unavailable);
        assert!(detail.rule.is_none());
        assert_eq!(detail.facts.title, "Runtime release 1");
    }

    #[test]
    fn stale_or_malformed_rule_never_blocks_reliable_facts() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store
            .connection
            .execute(
                "UPDATE rule_evaluations SET configuration_revision=999 WHERE item_id=1",
                [],
            )
            .expect("stale rule");
        let stale = store
            .query_intel_evidence_detail(&input())
            .expect("stale facts");
        assert_eq!(stale.rule_status, RuleEvidenceStatusV1::Stale);
        assert!(stale.rule.is_none());
        store.connection.execute(
            "UPDATE rule_evaluations SET configuration_revision=1,factor_results_json='not-json' WHERE item_id=1",
            [],
        ).expect("malformed rule");
        let malformed = store
            .query_intel_evidence_detail(&input())
            .expect("malformed facts");
        assert_eq!(malformed.rule_status, RuleEvidenceStatusV1::Stale);
        assert_eq!(malformed.rule_issue_code.as_deref(), Some("rule.malformed"));
    }

    #[test]
    fn single_source_and_ordinary_candidate_remain_queryable() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store.connection.execute_batch(
            "DELETE FROM intel_association_members; DELETE FROM intel_associations;
             UPDATE rule_evaluations SET score=40,importance='low',stream_disposition='ordinary_candidate',
               filter_reasons_json='[{\"code\":\"score_below_threshold\",\"actual\":40,\"threshold\":80}]'
             WHERE item_id=1;",
        ).expect("ordinary single source");
        let detail = store
            .query_intel_evidence_detail(&input())
            .expect("ordinary detail");
        assert_eq!(detail.provenance.len(), 1);
        assert!(detail.association.relation_type.is_none());
        let rule = detail.rule.expect("current ordinary rule");
        assert_eq!(rule.disposition.as_str(), "ordinary_candidate");
        assert_eq!(rule.filter_reasons.len(), 1);
    }

    #[test]
    fn missing_optional_facts_remain_null_and_damaged_association_is_local_incomplete() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store.connection.execute_batch(
            "UPDATE intel_items SET published_at=NULL WHERE id=1;
             UPDATE item_provenance SET published_at=NULL WHERE item_id=1;
             UPDATE intel_contents SET source_summary=NULL WHERE item_id=1;
             UPDATE intel_contents SET content_hash='ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff' WHERE item_id=2;",
        ).expect("partial association damage");

        let detail = store
            .query_intel_evidence_detail(&input())
            .expect("primary survives");
        assert!(detail.facts.source_summary.is_none());
        assert!(detail.facts.published_at.is_none());
        assert_eq!(detail.provenance.len(), 1);
        assert_eq!(detail.provenance[0].intel_item_id, ITEM_A);
        assert_eq!(
            detail.association.status,
            AssociationEvidenceStatusV1::Incomplete
        );
        assert_eq!(
            detail.association.issue_code.as_deref(),
            Some("association.incomplete")
        );
    }

    #[test]
    fn legacy_provenance_is_excluded_without_poisoning_reliable_primary_evidence() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store
            .connection
            .execute(
                "UPDATE item_provenance SET availability_status='unknown_legacy' WHERE item_id=2",
                [],
            )
            .expect("legacy associated provenance");
        let detail = store
            .query_intel_evidence_detail(&input())
            .expect("primary survives legacy association");
        assert_eq!(detail.provenance.len(), 1);
        assert_eq!(
            detail.association.status,
            AssociationEvidenceStatusV1::Incomplete
        );

        store
            .connection
            .execute(
                "UPDATE item_provenance SET author_availability='unknown_legacy' WHERE item_id=1",
                [],
            )
            .expect("legacy primary provenance");
        assert!(store.query_intel_evidence_detail(&input()).is_err());
    }

    #[test]
    fn unsafe_display_url_is_redacted_and_action_is_locally_disabled() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store
            .connection
            .execute(
                "UPDATE intel_items SET original_url='https://user:secret@example.com/private?token=canary' WHERE id=1",
                [],
            )
            .expect("unsafe item URL");
        store
            .connection
            .execute(
                "UPDATE item_provenance SET original_url='https://user:secret@example.com/private?token=canary' WHERE item_id=1",
                [],
            )
            .expect("unsafe provenance URL");

        let detail = store
            .query_intel_evidence_detail(&input())
            .expect("facts remain readable");
        assert_eq!(detail.provenance[0].display_url, "原文地址不可用");
        assert!(!detail.provenance[0].can_open_original);
        assert!(!format!("{detail:?}").contains("secret"));
    }

    #[test]
    fn malformed_rule_columns_and_semantics_are_local_stale() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store
            .connection
            .execute(
                "UPDATE rule_evaluations SET configuration_revision='broken' WHERE item_id=1",
                [],
            )
            .expect("malformed numeric identity");
        let malformed_column = store
            .query_intel_evidence_detail(&input())
            .expect("facts survive malformed rule column");
        assert_eq!(malformed_column.rule_status, RuleEvidenceStatusV1::Stale);

        store
            .connection
            .execute(
                "UPDATE rule_evaluations SET configuration_revision=1,importance='low',matched_tracks_json='[\"z\",\"a\"]' WHERE item_id=1",
                [],
            )
            .expect("contradictory rule semantics");
        let contradictory = store
            .query_intel_evidence_detail(&input())
            .expect("facts survive contradictory rule");
        assert_eq!(contradictory.rule_status, RuleEvidenceStatusV1::Stale);
        assert_eq!(
            contradictory.rule_issue_code.as_deref(),
            Some("rule.malformed")
        );
    }

    #[test]
    fn malformed_or_single_member_association_is_incomplete_not_blocking() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store
            .connection
            .execute_batch(
                "UPDATE intel_items SET publisher=zeroblob(4) WHERE id=2;
                 UPDATE item_provenance SET publisher=zeroblob(4) WHERE item_id=2;",
            )
            .expect("malformed associated row");
        let malformed = store
            .query_intel_evidence_detail(&input())
            .expect("primary survives malformed member");
        assert_eq!(malformed.provenance.len(), 1);
        assert_eq!(
            malformed.association.status,
            AssociationEvidenceStatusV1::Incomplete
        );

        let mut single = DemoStore::open_in_memory().expect("single store");
        seed_detail(&mut single, true);
        single
            .connection
            .execute("DELETE FROM intel_association_members WHERE item_id=2", [])
            .expect("single-member association");
        let detail = single
            .query_intel_evidence_detail(&input())
            .expect("single member remains readable");
        assert_eq!(
            detail.association.status,
            AssociationEvidenceStatusV1::Incomplete
        );
    }

    #[test]
    fn missing_detail_uses_the_stable_not_found_code() {
        let store = DemoStore::open_in_memory().expect("store");
        let error = store
            .query_intel_evidence_detail(&input())
            .expect_err("missing detail");
        assert_eq!(error.code(), "not_found.intel_detail");
        assert_eq!(error.category().as_str(), "not_found");
    }

    #[test]
    fn inconsistent_primary_provenance_fails_closed() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        store.connection.execute(
            "UPDATE item_provenance SET content_hash='ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff' WHERE item_id=1",
            [],
        ).expect("provenance drift");
        assert!(store.query_intel_evidence_detail(&input()).is_err());
    }

    #[test]
    fn original_link_is_resolved_by_membership_and_revalidated_without_network() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        let input = OpenIntelOriginalInputV1 {
            contract_version: 1,
            intel_item_id: ITEM_A.to_owned(),
            provenance_id: "prov:bbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
        };
        let link = store
            .resolve_intel_original(&input)
            .expect("associated link");
        assert_eq!(link.provenance_id(), input.provenance_id);
        assert_eq!(link.url(), "https://evidence.example/releases/runtime");
        store.connection.execute(
            "UPDATE item_provenance SET availability_status='unavailable' WHERE provenance_id=?1",
            [&input.provenance_id],
        ).expect("unavailable");
        assert!(store.resolve_intel_original(&input).is_err());
    }

    #[test]
    fn original_link_rejects_nonmember_and_non_public_literal_targets() {
        let mut store = DemoStore::open_in_memory().expect("store");
        seed_detail(&mut store, true);
        let input = OpenIntelOriginalInputV1 {
            contract_version: 1,
            intel_item_id: ITEM_A.to_owned(),
            provenance_id: "prov:bbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
        };
        store
            .connection
            .execute_batch("DELETE FROM intel_association_members; DELETE FROM intel_associations;")
            .expect("remove membership");
        assert!(store.resolve_intel_original(&input).is_err());
        store.connection.execute(
            "UPDATE intel_items SET original_url='https://127.0.0.1/private' WHERE intel_item_id=?1",
            [ITEM_A],
        ).expect("private item");
        store.connection.execute(
            "UPDATE item_provenance SET original_url='https://127.0.0.1/private' WHERE item_id=1",
            [],
        ).expect("private provenance");
        let primary = OpenIntelOriginalInputV1 {
            contract_version: 1,
            intel_item_id: ITEM_A.to_owned(),
            provenance_id: "prov:aaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
        };
        assert!(store.resolve_intel_original(&primary).is_err());
    }

    #[test]
    fn original_url_policy_rejects_unsafe_schemes_credentials_fragments_and_literal_hosts() {
        assert_eq!(
            validate_original_url("https://example.com/releases/runtime").expect("public domain"),
            "https://example.com/releases/runtime"
        );
        for rejected in [
            "http://example.com/item",
            "file:///C:/secret.txt",
            "javascript:alert(1)",
            "data:text/plain,secret",
            "https://user:password@example.com/item",
            "https://example.com/item#fragment",
            "https://localhost/item",
            "https://sub.localhost/item",
            "https://127.0.0.1/item",
            "https://10.0.0.1/item",
            "https://169.254.1.1/item",
            "https://224.0.0.1/item",
            "https://[::1]/item",
            "https://[fc00::1]/item",
            "https://[fe80::1]/item",
            "not a url",
        ] {
            assert!(
                validate_original_url(rejected).is_err(),
                "accepted {rejected}"
            );
        }
        assert!(
            validate_original_url(&format!("https://example.com/{}", "a".repeat(2_048))).is_err()
        );
    }
}
