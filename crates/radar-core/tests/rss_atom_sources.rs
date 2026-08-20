use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

use radar_core::application::demo::DemoStore;
use radar_core::application::sources::ProbedSource;
use radar_core::contracts::dto::source::SaveSourceInputV1;
use radar_core::contracts::errors::ErrorCode;
use radar_core::domain::sources::{
    CandidateDisposition, FetchIncrementalResult, RawSourceCandidate,
};
use radar_core::infrastructure::http::source_http_policy::{
    MAX_RESPONSE_BYTES, MAX_RETRY_DELAY_MS, canonicalize_public_https_url, parse_retry_after_ms,
    retry_delay_ms, validate_public_ips,
};
use radar_core::infrastructure::sources::rss_atom::parse_feed;

const RSS: &[u8] = include_bytes!("../../../contracts/fixtures/rss-atom/rss2-v1.xml");
const ATOM: &[u8] = include_bytes!("../../../contracts/fixtures/rss-atom/atom-v1.xml");
const TRANSPORT_CASES: &str =
    include_str!("../../../contracts/fixtures/rss-atom/transport-cases-v1.json");
const SOURCE_GOLDEN: &str = include_str!("../../../contracts/fixtures/golden/source_view_v1.json");
static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn save_fixture_source(
    store: &mut DemoStore,
    input: &SaveSourceInputV1,
) -> Result<radar_core::contracts::dto::source::SourceViewV1, radar_core::contracts::errors::AppError>
{
    let probed = ProbedSource::from_adapter_result(
        &input.url,
        FetchIncrementalResult {
            candidates: Vec::new(),
            etag: None,
            last_modified: None,
            adapter_cursor: Some("fixture-probe".to_owned()),
            not_modified: false,
        },
    )?;
    store.save_probed_source(input, &probed)
}

#[test]
fn authoritative_transport_fixture_covers_the_required_story_scenarios() {
    let fixture: serde_json::Value = serde_json::from_str(TRANSPORT_CASES).expect("scenario JSON");
    assert_eq!(fixture["contract_version"], 1);
    let cases = fixture["cases"].as_array().expect("cases");
    let ids = cases
        .iter()
        .map(|case| case["id"].as_str().expect("case id"))
        .collect::<HashSet<_>>();
    for required in [
        "rss2-first-200",
        "atom-first-200",
        "conditional-304",
        "rss2-updated-200",
        "missing-optional-fields",
        "invalid-optional-time",
        "rate-limit-seconds",
        "rate-limit-date",
        "server-error",
        "malformed-xml",
        "oversized-stream",
        "slow-stream",
        "public-redirect-chain",
        "private-redirect-rejected",
    ] {
        assert!(ids.contains(required), "missing scenario {required}");
    }
    assert_eq!(ids.len(), cases.len(), "scenario ids must be unique");
}

struct ScopedTestDir(PathBuf);

impl ScopedTestDir {
    fn new() -> Self {
        let root = std::env::current_dir()
            .expect("cwd")
            .join("target/story-2-2-source-tests");
        std::fs::create_dir_all(&root).expect("test root");
        for _ in 0..100 {
            let path = root.join(format!(
                "{}-{}",
                std::process::id(),
                TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            if std::fs::create_dir(&path).is_ok() {
                return Self(path);
            }
        }
        panic!("unique test directory");
    }
}

impl Drop for ScopedTestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn source_url_policy_is_https_only_and_rejects_non_public_addresses() {
    let canonical = canonicalize_public_https_url("HTTPS://Example.COM:443/feed.xml#part")
        .expect("public HTTPS URL");
    assert_eq!(canonical.as_str(), "https://example.com/feed.xml");
    assert!(canonicalize_public_https_url("http://example.com/feed").is_err());
    assert!(canonicalize_public_https_url("https://user:pass@example.com/feed").is_err());

    for ip in [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1)),
        IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        "::ffff:127.0.0.1".parse().expect("mapped address"),
        "100::1".parse().expect("discard address"),
        "64:ff9b:1::1".parse().expect("local translation address"),
        "2001:20::1".parse().expect("orchid address"),
    ] {
        assert!(validate_public_ips(&[ip]).is_err(), "accepted {ip}");
    }
    assert!(validate_public_ips(&["93.184.216.34".parse().unwrap()]).is_ok());
}

#[test]
fn retry_backoff_is_bounded_by_server_advice_and_never_shrinks() {
    assert_eq!(retry_delay_ms(1, None), 60_000);
    assert_eq!(retry_delay_ms(2, None), 120_000);
    assert_eq!(retry_delay_ms(3, None), 240_000);
    assert_eq!(retry_delay_ms(3, Some(600_000)), 600_000);
    assert_eq!(parse_retry_after_ms("120", 0), Some(120_000));
    assert_eq!(
        parse_retry_after_ms("Tue, 18 Aug 2026 10:02:00 GMT", 1_787_047_200),
        Some(120_000),
    );
    assert_eq!(parse_retry_after_ms("not-a-date", 0), None);
    assert_eq!(retry_delay_ms(1, Some(MAX_RETRY_DELAY_MS + 1)), 60_000);
    let error = radar_core::contracts::errors::AppError::from_code(
        ErrorCode::RateLimitedSource,
        "retry-after-test",
    )
    .with_retry_after_ms(120_000);
    assert_eq!(error.retry_after_ms(), Some(120_000));
}

#[test]
fn rss_and_atom_fixtures_preserve_optional_fields_and_stable_identity() {
    let rss = parse_feed(RSS).expect("RSS fixture");
    assert_eq!(rss.len(), 2);
    assert_eq!(rss[0].stable_external_id, "rust-1");
    assert_eq!(rss[1].author, None);
    assert_eq!(rss[1].summary, None);

    let atom = parse_feed(ATOM).expect("Atom fixture");
    assert_eq!(atom.len(), 1);
    assert_eq!(atom[0].stable_external_id, "tag:openai.com,2026:one");
    assert_eq!(atom[0].author.as_deref(), Some("OpenAI"));
}

#[test]
fn fallback_identity_uses_the_canonical_url_and_preserves_path_and_query() {
    // [P0] When RSS omits guid, only canonical URL equivalence may reuse identity.
    fn candidate(link: &str) -> RawSourceCandidate {
        let feed = format!(
            "<rss version=\"2.0\"><channel><item><title>Fallback identity</title><link>{link}</link></item></channel></rss>"
        );
        parse_feed(feed.as_bytes())
            .expect("valid fallback-identity feed")
            .into_iter()
            .next()
            .expect("one candidate")
    }

    let explicit_default_port = candidate("https://example.com:443/story?id=7#first-observation");
    let canonical_replay = candidate("https://example.com/story?id=7#replay");
    let different_path = candidate("https://example.com/other?id=7");
    let different_query = candidate("https://example.com/story?id=8");

    assert_eq!(
        explicit_default_port.original_url.as_deref(),
        Some("https://example.com/story?id=7")
    );
    assert_eq!(
        explicit_default_port.original_url,
        canonical_replay.original_url
    );
    assert!(explicit_default_port.stable_external_id.starts_with("url:"));
    assert_eq!(
        explicit_default_port.stable_external_id,
        canonical_replay.stable_external_id
    );
    assert_ne!(
        explicit_default_port.stable_external_id,
        different_path.stable_external_id
    );
    assert_ne!(
        explicit_default_port.stable_external_id,
        different_query.stable_external_id
    );
}

#[test]
fn malformed_or_oversized_feed_is_rejected_without_partial_candidates() {
    assert!(
        parse_feed(include_bytes!(
            "../../../contracts/fixtures/rss-atom/malformed.xml"
        ))
        .is_err()
    );
    assert!(parse_feed(&vec![b'x'; MAX_RESPONSE_BYTES + 1]).is_err());
}

#[test]
fn source_input_wire_contract_is_versioned() {
    let input = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "https://example.com/feed.xml".to_owned(),
        expected_configuration_revision: 1,
        idempotency_key: "source-save-1".to_owned(),
    };
    let value = serde_json::to_value(input).expect("serialize");
    assert_eq!(value["contract_version"], 1);
    assert_eq!(value["source_kind"], "rss_atom");
}

#[test]
fn source_golden_executes_the_production_save_contract() {
    let fixture: serde_json::Value = serde_json::from_str(SOURCE_GOLDEN).expect("source golden");
    let mut input: SaveSourceInputV1 =
        serde_json::from_value(fixture["input"].clone()).expect("source input");
    let mut store = DemoStore::open_in_memory().expect("schema v5");
    input.expected_configuration_revision = store.get_configuration().unwrap().revision;
    let actual = save_fixture_source(&mut store, &input).expect("production save");
    let expected = &fixture["expected"];

    assert_eq!(actual.source_id, expected["source_id"].as_str().unwrap());
    assert_eq!(
        actual.source_kind,
        expected["source_kind"].as_str().unwrap()
    );
    assert_eq!(
        actual.display_url,
        expected["display_url"].as_str().unwrap()
    );
    assert_eq!(actual.enabled, expected["enabled"].as_bool().unwrap());
    assert_eq!(actual.revision, expected["revision"].as_u64().unwrap());
    assert_eq!(
        serde_json::to_value(actual.status).unwrap(),
        expected["status"]
    );
    assert_eq!(
        serde_json::to_value(actual.retryability).unwrap(),
        expected["retryability"]
    );
    assert!(actual.last_success_at.is_none());
    assert!(actual.next_allowed_at.is_none());
    assert!(actual.created_at.contains('T') && actual.created_at.ends_with('Z'));
    assert_eq!(actual.created_at, actual.updated_at);
}

#[test]
fn source_save_is_atomic_idempotent_and_query_uses_keyset_cursor() {
    let mut store = DemoStore::open_in_memory().expect("schema v5");
    let revision = store.get_configuration().expect("configuration").revision;
    let input = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "https://example.com/feed.xml?channel=rust#ignored".to_owned(),
        expected_configuration_revision: revision,
        idempotency_key: "source-save-atomic".to_owned(),
    };
    let first = save_fixture_source(&mut store, &input).expect("save source");
    let replay = save_fixture_source(&mut store, &input).expect("idempotent replay");
    assert_eq!(first, replay);
    assert_eq!(first.display_url, "https://example.com/feed.xml");
    assert_eq!(
        store.query_sources(None, 1).expect("page").items,
        vec![first.clone()]
    );
    assert!(store.query_sources(Some("offset:1"), 1).is_err());

    let candidates = parse_feed(RSS).expect("fixture");
    let updated = store
        .apply_incremental_result(
            &first.source_id,
            first.revision,
            &FetchIncrementalResult {
                candidates,
                etag: Some("etag-v1".to_owned()),
                last_modified: Some("Sat, 01 Aug 2026 08:00:00 GMT".to_owned()),
                adapter_cursor: Some("cursor-v1".to_owned()),
                not_modified: false,
            },
        )
        .expect("commit checkpoint");
    assert_eq!(updated.source.revision, first.revision + 1);
    assert!(
        updated
            .candidates
            .iter()
            .all(|item| item.disposition == CandidateDisposition::New)
    );
    let persisted = store
        .source_fetch_state(&first.source_id)
        .expect("persisted cursor");
    assert_eq!(persisted.adapter_cursor.as_deref(), Some("cursor-v1"));
    let unchanged = store
        .apply_incremental_result(
            &updated.source.source_id,
            updated.source.revision,
            &FetchIncrementalResult {
                candidates: vec![],
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: true,
            },
        )
        .expect("304");
    assert_eq!(unchanged.source.revision, updated.source.revision + 1);
    let after_304 = store
        .source_fetch_state(&first.source_id)
        .expect("304 cursor");
    assert_eq!(after_304.adapter_cursor, persisted.adapter_cursor);
    let failed = store
        .record_source_failure(
            &updated.source.source_id,
            unchanged.source.revision,
            ErrorCode::RateLimitedSource,
            Some(600_000),
            1_787_047_200_000,
        )
        .expect("source-scoped retry state");
    assert_eq!(
        failed.status,
        radar_core::contracts::dto::source::SourceStatusV1::RetryWait
    );
    assert_eq!(
        failed.retryability,
        radar_core::contracts::dto::source::SourceRetryabilityV1::After
    );
    assert_eq!(
        failed.next_allowed_at.as_deref(),
        Some("2026-08-18T10:10:00.000Z")
    );
}

#[test]
fn parser_rejects_non_feed_xml_and_unsafe_identity_but_accepts_an_empty_feed() {
    assert!(parse_feed(b"<root><item><guid>x</guid></item></root>").is_err());
    assert!(
        parse_feed(b"<rss version=\"2.0\"><channel><title>Empty</title></channel></rss>")
            .expect("valid empty RSS")
            .is_empty()
    );
    assert!(
        parse_feed(
            b"<rss version=\"2.0\"><channel><item><link>javascript:alert(1)</link></item></channel></rss>"
        )
        .is_err()
    );
    assert!(
        parse_feed(
            b"<rss version=\"2.0\"><channel><item><link>http://example.com/post</link></item></channel></rss>"
        )
        .is_err()
    );
    let numeric_time = parse_feed(
        b"<rss version=\"2.0\"><channel><item><guid>x</guid><pubDate>120</pubDate></item></channel></rss>",
    )
    .expect("invalid optional time is not invented");
    assert!(numeric_time[0].published_at.is_none());
    assert_eq!(numeric_time[0].warnings.len(), 1);
    assert_eq!(
        numeric_time[0].warnings[0].code,
        "source.invalid_optional_time"
    );

    let invalid_fixture = parse_feed(include_bytes!(
        "../../../contracts/fixtures/rss-atom/rss2-invalid-time.xml"
    ))
    .expect("invalid optional time fixture remains a valid feed");
    assert!(invalid_fixture[0].published_at.is_none());
    assert_eq!(invalid_fixture[0].warnings[0].field, "published_at");
}

#[test]
fn parser_bounds_structure_text_encoding_and_atom_link_selection() {
    let deep = format!(
        "<rss version=\"2.0\"><channel><item><guid>x</guid>{}{}</item></channel></rss>",
        "<x>".repeat(65),
        "</x>".repeat(65)
    );
    assert!(parse_feed(deep.as_bytes()).is_err());

    let segmented = format!(
        "<rss version=\"2.0\"><channel><item><guid>x</guid><description><![CDATA[{}]]><![CDATA[{}]]></description></item></channel></rss>",
        "a".repeat(130_000),
        "b".repeat(130_000)
    );
    assert!(parse_feed(segmented.as_bytes()).is_err());
    assert!(
        parse_feed(
            b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><rss version=\"2.0\"><channel/></rss>"
        )
        .is_err()
    );

    let atom = parse_feed(
        br#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><id>x</id><link rel="self" href="https://example.com/api/x"></link><link rel="alternate" href="https://EXAMPLE.com:443/post#fragment"></link></entry></feed>"#,
    )
    .expect("atom links");
    assert_eq!(
        atom[0].original_url.as_deref(),
        Some("https://example.com/post")
    );

    let prefixed_atom = parse_feed(
        br#"<atom:feed xmlns:atom="http://www.w3.org/2005/Atom"><atom:entry><atom:id>prefixed</atom:id><atom:title>Accepted</atom:title></atom:entry></atom:feed>"#,
    )
    .expect("correctly bound prefixed Atom feed");
    assert_eq!(prefixed_atom[0].stable_external_id, "prefixed");
    assert_eq!(prefixed_atom[0].title.as_deref(), Some("Accepted"));

    assert!(
        parse_feed(
            br#"<evil:feed xmlns:atom="http://www.w3.org/2005/Atom"><evil:entry><evil:id>x</evil:id></evil:entry></evil:feed>"#,
        )
        .is_err()
    );
    assert!(
        parse_feed(
            br#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><id>x</id><evil:title xmlns:evil="urn:evil">Injected</evil:title></entry></feed>"#,
        )
        .is_err()
    );
}

#[test]
fn incremental_result_rejects_contradictory_or_duplicate_candidates() {
    let mut store = DemoStore::open_in_memory().expect("schema v5");
    let revision = store.get_configuration().expect("configuration").revision;
    let source_input = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "https://example.com/feed.xml".to_owned(),
        expected_configuration_revision: revision,
        idempotency_key: "source-result-invariants".to_owned(),
    };
    let source = save_fixture_source(&mut store, &source_input).expect("source");
    let candidate = RawSourceCandidate {
        stable_external_id: "same".to_owned(),
        title: None,
        original_url: None,
        author: None,
        summary: None,
        published_at: None,
        updated_at: None,
        content_hash: "a".repeat(64),
        warnings: Vec::new(),
    };
    assert!(
        store
            .apply_incremental_result(
                &source.source_id,
                source.revision,
                &FetchIncrementalResult {
                    candidates: vec![candidate.clone()],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: true,
                },
            )
            .is_err()
    );
    assert!(
        store
            .apply_incremental_result(
                &source.source_id,
                source.revision,
                &FetchIncrementalResult {
                    candidates: vec![candidate.clone(), candidate],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .is_err()
    );
}

#[test]
fn incremental_application_classifies_new_changed_and_unchanged_candidates() {
    let mut store = DemoStore::open_in_memory().expect("schema v5");
    let revision = store.get_configuration().expect("configuration").revision;
    let source_input = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "https://example.com/dispositions.xml".to_owned(),
        expected_configuration_revision: revision,
        idempotency_key: "source-dispositions".to_owned(),
    };
    let source = save_fixture_source(&mut store, &source_input).expect("source");
    let original = RawSourceCandidate {
        stable_external_id: "entry-1".to_owned(),
        title: Some("Original".to_owned()),
        original_url: Some("https://example.com/entry-1".to_owned()),
        author: None,
        summary: None,
        published_at: None,
        updated_at: None,
        content_hash: "a".repeat(64),
        warnings: Vec::new(),
    };
    let first = store
        .apply_incremental_result(
            &source.source_id,
            source.revision,
            &FetchIncrementalResult {
                candidates: vec![original.clone()],
                etag: Some("v1".to_owned()),
                last_modified: None,
                adapter_cursor: Some("cursor-v1".to_owned()),
                not_modified: false,
            },
        )
        .expect("first result");
    assert_eq!(first.candidates[0].disposition, CandidateDisposition::New);

    let unchanged = RawSourceCandidate {
        stable_external_id: "entry-1".to_owned(),
        content_hash: "a".repeat(64),
        ..original_candidate("entry-1")
    };
    let added = original_candidate("entry-2");
    let second = store
        .apply_incremental_result(
            &source.source_id,
            first.source.revision,
            &FetchIncrementalResult {
                candidates: vec![unchanged, added],
                etag: Some("v2".to_owned()),
                last_modified: None,
                adapter_cursor: Some("cursor-v2".to_owned()),
                not_modified: false,
            },
        )
        .expect("second result");
    assert_eq!(
        second
            .candidates
            .iter()
            .map(|item| item.disposition)
            .collect::<Vec<_>>(),
        vec![CandidateDisposition::Unchanged, CandidateDisposition::New]
    );

    let mut changed = original_candidate("entry-1");
    changed.content_hash = "b".repeat(64);
    let third = store
        .apply_incremental_result(
            &source.source_id,
            second.source.revision,
            &FetchIncrementalResult {
                candidates: vec![changed],
                etag: Some("v3".to_owned()),
                last_modified: None,
                adapter_cursor: Some("cursor-v3".to_owned()),
                not_modified: false,
            },
        )
        .expect("changed result");
    assert_eq!(
        third.candidates[0].disposition,
        CandidateDisposition::Changed
    );
}

fn original_candidate(id: &str) -> RawSourceCandidate {
    RawSourceCandidate {
        stable_external_id: id.to_owned(),
        title: Some("Original".to_owned()),
        original_url: Some(format!("https://example.com/{id}")),
        author: None,
        summary: None,
        published_at: None,
        updated_at: None,
        content_hash: "a".repeat(64),
        warnings: Vec::new(),
    }
}

#[test]
fn permanent_failure_and_resave_have_consistent_source_state() {
    let mut store = DemoStore::open_in_memory().expect("schema v5");
    let revision = store.get_configuration().expect("configuration").revision;
    let mut input = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "https://example.com/feed.xml".to_owned(),
        expected_configuration_revision: revision,
        idempotency_key: "source-state-first".to_owned(),
    };
    let source = save_fixture_source(&mut store, &input).expect("source");
    let failed = store
        .record_source_failure(
            &source.source_id,
            source.revision,
            ErrorCode::SourceFormatRssAtom,
            None,
            1_787_047_200_000,
        )
        .expect("permanent failure");
    assert_eq!(
        failed.status,
        radar_core::contracts::dto::source::SourceStatusV1::Error
    );
    assert_eq!(
        failed.retryability,
        radar_core::contracts::dto::source::SourceRetryabilityV1::Never
    );
    assert!(failed.next_allowed_at.is_none());

    input.expected_configuration_revision = store.get_configuration().unwrap().revision;
    input.idempotency_key = "source-state-resave".to_owned();
    let resaved = save_fixture_source(&mut store, &input).expect("resave");
    assert_eq!(
        resaved.status,
        radar_core::contracts::dto::source::SourceStatusV1::Ready
    );
    assert!(resaved.next_allowed_at.is_none());
}

#[test]
fn concurrent_idempotent_source_saves_replay_one_authoritative_response() {
    let directory = ScopedTestDir::new();
    let database = directory.0.join("source.sqlite3");
    let initial = DemoStore::open(&database).expect("initialize store");
    let revision = initial.get_configuration().expect("configuration").revision;
    drop(initial);

    let input = Arc::new(SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "https://example.com/concurrent.xml".to_owned(),
        expected_configuration_revision: revision,
        idempotency_key: "source-concurrent-replay".to_owned(),
    });
    let barrier = Arc::new(Barrier::new(2));
    let stores = (0..2)
        .map(|_| DemoStore::open(&database).expect("concurrent store"))
        .collect::<Vec<_>>();
    let handles = stores
        .into_iter()
        .map(|mut store| {
            let input = Arc::clone(&input);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                save_fixture_source(&mut store, &input)
            })
        })
        .collect::<Vec<_>>();
    let responses = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .expect("writer thread")
                .expect("idempotent save")
        })
        .collect::<Vec<_>>();
    assert_eq!(responses[0], responses[1]);
}

#[test]
fn incremental_harness_reopens_with_validators_cursor_and_persists_retry_after() {
    let directory = ScopedTestDir::new();
    let database = directory.0.join("incremental.sqlite3");
    let mut store = DemoStore::open(&database).expect("initialize store");
    let revision = store.get_configuration().expect("configuration").revision;
    let input = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "https://example.com/incremental.xml".to_owned(),
        expected_configuration_revision: revision,
        idempotency_key: "source-incremental-harness".to_owned(),
    };
    let source = save_fixture_source(&mut store, &input).expect("source");
    let prepared = store
        .prepare_incremental_fetch(&source.source_id)
        .expect("prepare first fetch");
    assert!(prepared.etag.is_none());
    assert!(prepared.last_modified.is_none());
    let committed = store
        .commit_incremental_fetch(
            &prepared,
            &FetchIncrementalResult {
                candidates: parse_feed(RSS).expect("RSS"),
                etag: Some("\"rss-v1\"".to_owned()),
                last_modified: Some("Sat, 01 Aug 2026 08:00:00 GMT".to_owned()),
                adapter_cursor: Some("rss-cursor-v1".to_owned()),
                not_modified: false,
            },
        )
        .expect("commit first fetch");
    drop(store);

    let mut reopened = DemoStore::open(&database).expect("reopen store");
    let conditional = reopened
        .prepare_incremental_fetch(&source.source_id)
        .expect("prepare conditional fetch");
    assert_eq!(conditional.expected_revision, committed.source.revision);
    assert_eq!(conditional.etag.as_deref(), Some("\"rss-v1\""));
    assert_eq!(
        conditional.last_modified.as_deref(),
        Some("Sat, 01 Aug 2026 08:00:00 GMT")
    );
    assert_eq!(conditional.adapter_cursor.as_deref(), Some("rss-cursor-v1"));

    let rate_limited = radar_core::contracts::errors::AppError::from_code(
        ErrorCode::RateLimitedSource,
        "fixture-rate-limit",
    )
    .with_source_id(&source.source_id)
    .with_retry_after_ms(120_000);
    let failed = reopened
        .commit_incremental_failure(&conditional, &rate_limited, 1_787_047_200_000)
        .expect("persist retry-after");
    assert_eq!(
        failed.next_allowed_at.as_deref(),
        Some("2026-08-18T10:02:00.000Z")
    );
}

#[test]
fn rejected_source_write_leaves_configuration_and_sources_unchanged() {
    let mut store = DemoStore::open_in_memory().expect("schema v5");
    let before = store.get_configuration().expect("configuration");
    let rejected = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: "http://127.0.0.1/feed.xml".to_owned(),
        expected_configuration_revision: before.revision,
        idempotency_key: "source-rejected".to_owned(),
    };
    assert!(save_fixture_source(&mut store, &rejected).is_err());
    assert_eq!(store.get_configuration().unwrap().revision, before.revision);
    assert!(store.query_sources(None, 10).unwrap().items.is_empty());
}
