use radar_core::contracts::dto::sync::SyncResultItemV1;
use radar_core::domain::intel::{
    AuthorAvailability, NormalizationIssueCode, normalize_rss_candidate,
};
use radar_core::domain::sources::{RawSourceCandidate, SourceFieldWarning};

fn candidate() -> RawSourceCandidate {
    RawSourceCandidate {
        stable_external_id: "entry-1".to_owned(),
        title: Some("  Rust 1.97 released  ".to_owned()),
        original_url: Some("https://blog.rust-lang.org/releases/1.97".to_owned()),
        author: None,
        summary: Some("Compiler and tooling updates".to_owned()),
        published_at: Some("2026-08-19T01:02:03+00:00".to_owned()),
        updated_at: None,
        content_hash: "a".repeat(64),
        warnings: Vec::new(),
    }
}

fn assert_normalization_issue(
    candidate: &RawSourceCandidate,
    expected: NormalizationIssueCode,
    case_name: &str,
) {
    let issue = normalize_rss_candidate(
        "source:aaaaaaaaaaaaaaaaaaaaaaaa",
        "blog.rust-lang.org",
        "2026-08-19T02:03:04Z",
        candidate,
    )
    .expect_err(case_name);
    assert_eq!(issue.code, expected, "{case_name}");
}

#[test]
fn normalization_is_stable_source_scoped_and_does_not_invent_author() {
    let first = normalize_rss_candidate(
        "source:aaaaaaaaaaaaaaaaaaaaaaaa",
        "blog.rust-lang.org",
        "2026-08-19T02:03:04Z",
        &candidate(),
    )
    .expect("valid candidate");
    let replay = normalize_rss_candidate(
        "source:aaaaaaaaaaaaaaaaaaaaaaaa",
        "blog.rust-lang.org",
        "2026-08-19T02:03:04Z",
        &candidate(),
    )
    .expect("stable replay");
    let other_source = normalize_rss_candidate(
        "source:bbbbbbbbbbbbbbbbbbbbbbbb",
        "blog.rust-lang.org",
        "2026-08-19T02:03:04Z",
        &candidate(),
    )
    .expect("other source");

    assert_eq!(first.intel_item_id, replay.intel_item_id);
    assert_eq!(first.canonical_external_id, replay.canonical_external_id);
    assert_ne!(first.intel_item_id, other_source.intel_item_id);
    assert!(first.intel_item_id.as_str().starts_with("intel:"));
    assert_eq!(first.intel_item_id.as_str().len(), 70);
    assert_eq!(first.original_title, "Rust 1.97 released");
    assert_eq!(first.published_at.as_deref(), Some("2026-08-19T01:02:03Z"));
    assert_eq!(first.author, None);
    assert_eq!(first.author_availability, AuthorAvailability::Unavailable);
}

#[test]
fn invalid_optional_time_becomes_null_with_allowlisted_warning() {
    let mut input = candidate();
    input.published_at = Some("not-a-time".to_owned());
    input.warnings.push(SourceFieldWarning {
        field: "updated_at".to_owned(),
        code: "source.invalid_optional_time".to_owned(),
    });
    let normalized = normalize_rss_candidate(
        "source:aaaaaaaaaaaaaaaaaaaaaaaa",
        "blog.rust-lang.org",
        "2026-08-19T02:03:04Z",
        &input,
    )
    .expect("optional time warning");

    assert_eq!(normalized.published_at, None);
    assert!(normalized.warnings.iter().any(|warning| {
        warning.field == "published_at"
            && warning.code == NormalizationIssueCode::InvalidOptionalTime
    }));
    assert!(normalized.warnings.iter().any(|warning| {
        warning.field == "updated_at" && warning.code == NormalizationIssueCode::InvalidOptionalTime
    }));
}

#[test]
fn required_and_bounded_fields_fail_closed() {
    let mut empty_title = candidate();
    empty_title.title = Some("  ".to_owned());
    assert_normalization_issue(
        &empty_title,
        NormalizationIssueCode::Required,
        "empty title",
    );

    let mut oversized_author = candidate();
    oversized_author.author = Some("作".repeat(513));
    assert_normalization_issue(
        &oversized_author,
        NormalizationIssueCode::TooLong,
        "oversized author",
    );

    let mut unsafe_url = candidate();
    unsafe_url.original_url = Some("http://127.0.0.1/private".to_owned());
    assert_normalization_issue(
        &unsafe_url,
        NormalizationIssueCode::InvalidUrl,
        "unsafe url",
    );

    for non_public in [
        "https://224.0.0.1/multicast",
        "https://203.0.113.8/documentation",
        "https://[ff02::1]/multicast",
    ] {
        let mut unsafe_literal = candidate();
        unsafe_literal.original_url = Some(non_public.to_owned());
        assert_normalization_issue(
            &unsafe_literal,
            NormalizationIssueCode::InvalidUrl,
            "non-public IP literal",
        );
    }

    let mut bad_hash = candidate();
    bad_hash.content_hash = "A".repeat(64);
    assert_normalization_issue(&bad_hash, NormalizationIssueCode::InvalidHash, "bad hash");
}

#[test]
fn serde_boundaries_reject_unknown_candidate_and_warning_fields() {
    let candidate_json = serde_json::json!({
        "stable_external_id": "entry-1",
        "title": "Title",
        "original_url": "https://example.com/item",
        "author": null,
        "summary": null,
        "published_at": null,
        "updated_at": null,
        "content_hash": "a".repeat(64),
        "warnings": [],
        "unexpected": true
    });
    assert!(serde_json::from_value::<RawSourceCandidate>(candidate_json).is_err());

    let warning_json = serde_json::json!({
        "field": "published_at",
        "code": "source.invalid_optional_time",
        "unexpected": true
    });
    assert!(serde_json::from_value::<SourceFieldWarning>(warning_json).is_err());
}

#[test]
fn sync_result_item_v1_accepts_legacy_missing_link_but_rejects_extra_fields() {
    let legacy = serde_json::json!({
        "contract_version": 1,
        "result_item_id": "result:0123456789abcdef01234567",
        "sync_run_id": "run:0123456789abcdef01234567",
        "source_id": "source:0123456789abcdef01234567",
        "source_kind": "rss_atom",
        "publisher": "example.com",
        "original_title": "Title",
        "published_at": null,
        "collected_at": "2026-08-19T02:03:04Z",
        "original_url": "https://example.com/item",
        "disposition": "inserted"
    });
    let decoded: SyncResultItemV1 =
        serde_json::from_value(legacy.clone()).expect("legacy missing link");
    assert_eq!(decoded.intel_item_id, None);
    let mut extra = legacy;
    extra["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<SyncResultItemV1>(extra).is_err());
}
