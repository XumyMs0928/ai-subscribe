use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use radar_core::application::demo::DemoStore;
use radar_core::application::sources::ProbedSource;
use radar_core::contracts::dto::intel_feed::{
    IntelFeedFiltersV1, IntelFeedSortV1, IntelFeedStreamV1, IntelFeedTimeWindowV1,
    QueryIntelFeedInputV1,
};
use radar_core::contracts::dto::source::SaveSourceInputV1;
use radar_core::contracts::dto::sync::{
    DeliveryReadinessStatusV1, GetSyncResultInputV1, StartSyncInputV1, SyncResultDispositionV1,
    SyncResultItemV1, SyncRunIdV1, SyncRunOutcomeV1, SyncTargetV1, TaskStateV1,
};
use radar_core::contracts::errors::{AppError, ErrorCode};
use radar_core::domain::sources::{FetchIncrementalResult, RawSourceCandidate};
use sha2::{Digest, Sha256};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const OBSERVED_AT_MS: u64 = 1_787_047_200_000;
const FAR_FUTURE_RETRY_AT_MS: u64 = 4_102_444_800_000;

struct ScopedDatabase(PathBuf);

impl ScopedDatabase {
    fn new() -> Self {
        let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::current_dir()
            .expect("project directory")
            .join("target")
            .join("story-2-5-core")
            .join(format!("sync-{}-{sequence}.sqlite3", std::process::id()));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("test database directory");
        }
        Self(path)
    }
}

impl Drop for ScopedDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}

fn save_source(store: &mut DemoStore, url: &str, key: &str) -> String {
    let revision = store.get_configuration().expect("configuration").revision;
    let input = SaveSourceInputV1 {
        contract_version: 1,
        source_kind: "rss_atom".to_owned(),
        url: url.to_owned(),
        expected_configuration_revision: revision,
        idempotency_key: key.to_owned(),
    };
    let probed = ProbedSource::from_adapter_result(
        url,
        FetchIncrementalResult {
            candidates: Vec::new(),
            etag: None,
            last_modified: None,
            adapter_cursor: None,
            not_modified: false,
        },
    )
    .expect("fixture probe");
    store
        .save_probed_source(&input, &probed)
        .expect("save source")
        .source_id
}

fn start_input(target: SyncTargetV1, key: &str) -> StartSyncInputV1 {
    StartSyncInputV1 {
        contract_version: 1,
        target,
        idempotency_key: key.to_owned(),
        foreground_budget_ms: 30_000,
    }
}

fn result_input(
    sync_run_id: SyncRunIdV1,
    cursor: Option<String>,
    limit: u32,
) -> GetSyncResultInputV1 {
    GetSyncResultInputV1 {
        contract_version: 1,
        sync_run_id,
        cursor,
        limit,
    }
}

fn candidate(id: &str, title: Option<&str>, url: Option<&str>, hash: &str) -> RawSourceCandidate {
    let content_hash = if hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        hash.to_owned()
    } else {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(hash.as_bytes()) {
            let _ = write!(&mut output, "{byte:02x}");
        }
        output
    };
    RawSourceCandidate {
        stable_external_id: id.to_owned(),
        title: title.map(str::to_owned),
        original_url: url.map(str::to_owned),
        author: None,
        summary: None,
        published_at: Some("2026-08-18T08:00:00Z".to_owned()),
        updated_at: None,
        content_hash,
        warnings: Vec::new(),
    }
}

#[test]
fn committed_result_is_paged_reopenable_and_counts_invalid_candidates() {
    let database = ScopedDatabase::new();
    let mut store = DemoStore::open(&database.0).expect("v7");
    let source_id = save_source(
        &mut store,
        "https://publisher.example/feed.xml",
        "source-result-page",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-result-page",
        ))
        .expect("start");
    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim");
    store
        .commit_sync_source_success(
            &task.task_id,
            plan.task.revision,
            &plan.sources[0],
            &FetchIncrementalResult {
                candidates: vec![
                    candidate("one", Some("One"), Some("https://example.com/one"), "h1"),
                    candidate("two", Some("Two"), Some("https://example.com/two"), "h2"),
                    candidate("invalid", None, Some("https://example.com/invalid"), "h3"),
                ],
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: false,
            },
        )
        .expect("commit");
    let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
    let first = store
        .get_sync_result(&result_input(
            snapshot.result_ref.clone().expect("v7 run"),
            None,
            1,
        ))
        .expect("first page");
    assert_eq!(first.summary.outcome, SyncRunOutcomeV1::PartiallySucceeded);
    assert_eq!(first.summary.counts.inserted, 2);
    assert_eq!(first.summary.counts.failed, 1);
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].publisher, "publisher.example");
    let second = store
        .get_sync_result(&result_input(
            snapshot.result_ref.clone().expect("v7 run"),
            first.next_cursor,
            1,
        ))
        .expect("second page");
    assert_eq!(second.items.len(), 1);
    assert!(second.next_cursor.is_none());
    drop(store);

    let reopened = DemoStore::open(&database.0).expect("reopen");
    let restored = reopened
        .get_sync_result(&result_input(
            snapshot.result_ref.expect("v7 run"),
            None,
            100,
        ))
        .expect("offline result");
    assert_eq!(restored.items.len(), 2);
    assert_eq!(restored.summary.counts.failed, 1);
}

#[test]
fn final_fact_identity_is_stable_updated_and_source_scoped() {
    let mut store = DemoStore::open_in_memory().expect("v8");
    let source_a = save_source(
        &mut store,
        "https://publisher-a.example/feed.xml",
        "source-final-a",
    );
    let source_b = save_source(
        &mut store,
        "https://publisher-b.example/feed.xml",
        "source-final-b",
    );
    let first = sync_one(
        &mut store,
        &source_a,
        "sync-final-a-v1",
        candidate(
            "shared-entry",
            Some("First title"),
            Some("https://publisher-a.example/item"),
            "version-one",
        ),
    );
    let updated = sync_one(
        &mut store,
        &source_a,
        "sync-final-a-v2",
        candidate(
            "shared-entry",
            Some("Updated title"),
            Some("https://publisher-a.example/item"),
            "version-two",
        ),
    );
    let other = sync_one(
        &mut store,
        &source_b,
        "sync-final-b-v1",
        candidate(
            "shared-entry",
            Some("Other source title"),
            Some("https://publisher-b.example/item"),
            "version-one",
        ),
    );

    assert_eq!(first.intel_item_id, updated.intel_item_id);
    assert_ne!(first.intel_item_id, other.intel_item_id);
    assert_eq!(first.disposition, SyncResultDispositionV1::Inserted);
    assert_eq!(updated.disposition, SyncResultDispositionV1::Updated);
}

#[test]
fn unchanged_second_run_is_explicit_zero_result_not_failure() {
    let mut store = DemoStore::open_in_memory().expect("v7");
    let source_id = save_source(
        &mut store,
        "https://example.com/feed.xml",
        "source-zero-result",
    );
    let item = candidate(
        "same",
        Some("Same"),
        Some("https://example.com/same"),
        "same-hash",
    );
    for (key, expected) in [
        ("sync-first-result", SyncRunOutcomeV1::SucceededWithResults),
        ("sync-zero-result", SyncRunOutcomeV1::SucceededZeroResults),
    ] {
        let task = store
            .start_sync(&start_input(
                SyncTargetV1::SourceId {
                    source_id: source_id.clone(),
                },
                key,
            ))
            .expect("start");
        let plan = store
            .claim_sync_task(&task.task_id, task.revision)
            .expect("claim");
        store
            .commit_sync_source_success(
                &task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![item.clone()],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("commit");
        let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
        let result = store
            .get_sync_result(&result_input(
                snapshot.result_ref.expect("v7 run"),
                None,
                100,
            ))
            .expect("result");
        assert_eq!(result.summary.outcome, expected);
        if expected == SyncRunOutcomeV1::SucceededZeroResults {
            assert_eq!(result.summary.counts.skipped, 1);
            assert!(result.items.is_empty());
        }
    }
}

#[test]
fn invalid_optional_time_is_stored_as_null() {
    let mut store = DemoStore::open_in_memory().expect("v7");
    let source_id = save_source(
        &mut store,
        "https://example.com/repair.xml",
        "source-result-repair",
    );
    let first = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId {
                source_id: source_id.clone(),
            },
            "sync-invalid-result",
        ))
        .expect("start invalid");
    let first_plan = store
        .claim_sync_task(&first.task_id, first.revision)
        .expect("claim invalid");
    let mut malformed_time = candidate(
        "repairable",
        Some("Repairable"),
        Some("https://example.com/repairable"),
        "same-hash",
    );
    malformed_time.published_at = Some("not-a-time".to_owned());
    let first_commit = store
        .commit_sync_source_success(
            &first.task_id,
            first_plan.task.revision,
            &first_plan.sources[0],
            &FetchIncrementalResult {
                candidates: vec![malformed_time],
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: false,
            },
        )
        .expect("commit candidate with optional warning");
    assert_eq!(first_commit.state, TaskStateV1::Succeeded);
    let first_snapshot = store.task_snapshot(&first.task_id).expect("first snapshot");
    let first_result = store
        .get_sync_result(&result_input(
            first_snapshot.result_ref.expect("first run"),
            None,
            100,
        ))
        .expect("first result");
    assert_eq!(first_result.summary.counts.inserted, 1);
    assert_eq!(first_result.items[0].published_at, None);
}

fn sync_one(
    store: &mut DemoStore,
    source_id: &str,
    key: &str,
    value: RawSourceCandidate,
) -> SyncResultItemV1 {
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId {
                source_id: source_id.to_owned(),
            },
            key,
        ))
        .expect("start single candidate sync");
    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim single candidate sync");
    store
        .commit_sync_source_success(
            &task.task_id,
            plan.task.revision,
            &plan.sources[0],
            &FetchIncrementalResult {
                candidates: vec![value],
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: false,
            },
        )
        .expect("commit single candidate sync");
    let snapshot = store.task_snapshot(&task.task_id).expect("task snapshot");
    store
        .get_sync_result(&result_input(
            snapshot.result_ref.expect("result reference"),
            None,
            100,
        ))
        .expect("single candidate result")
        .items
        .into_iter()
        .next()
        .expect("inserted or updated item")
}

#[test]
fn same_hash_replay_is_unchanged_even_when_display_fields_differ() {
    let mut store = DemoStore::open_in_memory().expect("v8");
    let source_id = save_source(
        &mut store,
        "https://example.com/unchanged.xml",
        "source-unchanged-replay",
    );
    sync_one(
        &mut store,
        &source_id,
        "sync-unchanged-first",
        candidate(
            "same-entry",
            Some("First title"),
            Some("https://example.com/same-entry"),
            "same-hash",
        ),
    );

    let second = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-unchanged-second",
        ))
        .expect("start replay");
    let plan = store
        .claim_sync_task(&second.task_id, second.revision)
        .expect("claim replay");
    store
        .commit_sync_source_success(
            &second.task_id,
            plan.task.revision,
            &plan.sources[0],
            &FetchIncrementalResult {
                candidates: vec![candidate(
                    "same-entry",
                    Some("Changed title with same content hash"),
                    Some("https://example.com/same-entry"),
                    "same-hash",
                )],
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: false,
            },
        )
        .expect("commit replay");
    let snapshot = store.task_snapshot(&second.task_id).expect("snapshot");
    let result = store
        .get_sync_result(&result_input(
            snapshot.result_ref.expect("v8 run"),
            None,
            100,
        ))
        .expect("replay result");

    assert_eq!(
        result.summary.outcome,
        SyncRunOutcomeV1::SucceededZeroResults
    );
    assert_eq!(result.summary.counts.skipped, 1);
    assert!(result.items.is_empty());
}

#[test]
fn cursor_is_bound_to_the_run_that_issued_it() {
    let mut store = DemoStore::open_in_memory().expect("v7");
    let source_id = save_source(
        &mut store,
        "https://example.com/cursor.xml",
        "source-result-cursor",
    );
    let mut run_ids = Vec::new();
    let mut first_cursor = None;
    for (index, hash_suffix) in ["first", "second"].into_iter().enumerate() {
        let task = store
            .start_sync(&start_input(
                SyncTargetV1::SourceId {
                    source_id: source_id.clone(),
                },
                &format!("sync-cursor-{index}"),
            ))
            .expect("start");
        let plan = store
            .claim_sync_task(&task.task_id, task.revision)
            .expect("claim");
        store
            .commit_sync_source_success(
                &task.task_id,
                plan.task.revision,
                &plan.sources[0],
                &FetchIncrementalResult {
                    candidates: vec![
                        candidate(
                            "cursor-a",
                            Some("A"),
                            Some("https://example.com/a"),
                            &format!("a-{hash_suffix}"),
                        ),
                        candidate(
                            "cursor-b",
                            Some("B"),
                            Some("https://example.com/b"),
                            &format!("b-{hash_suffix}"),
                        ),
                    ],
                    etag: None,
                    last_modified: None,
                    adapter_cursor: None,
                    not_modified: false,
                },
            )
            .expect("commit");
        let run_id = store
            .task_snapshot(&task.task_id)
            .expect("snapshot")
            .result_ref
            .expect("v7 run");
        if index == 0 {
            first_cursor = store
                .get_sync_result(&result_input(run_id.clone(), None, 1))
                .expect("first page")
                .next_cursor;
        } else {
            let updated = store
                .get_sync_result(&result_input(run_id.clone(), None, 100))
                .expect("updated page");
            assert_eq!(updated.summary.counts.updated, 2);
            assert!(
                updated
                    .items
                    .iter()
                    .all(|item| item.disposition == SyncResultDispositionV1::Updated)
            );
        }
        run_ids.push(run_id);
    }
    let error = store
        .get_sync_result(&result_input(run_ids[1].clone(), first_cursor, 1))
        .expect_err("cross-run cursor rejected");
    assert_eq!(error.code(), "validation.source");
    assert!(
        store
            .get_sync_result(&result_input(run_ids[1].clone(), None, 1))
            .is_ok()
    );
}

#[test]
fn all_failed_run_has_an_explicit_readable_failed_result() {
    let mut store = DemoStore::open_in_memory().expect("v7");
    let source_id = save_source(
        &mut store,
        "https://example.com/failed.xml",
        "source-result-failed",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-result-failed",
        ))
        .expect("start");
    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim");
    store
        .commit_sync_source_internal_failure(&task.task_id, plan.task.revision, &plan.sources[0])
        .expect("fail source");
    let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
    let result = store
        .get_sync_result(&result_input(
            snapshot.result_ref.expect("v7 run"),
            None,
            100,
        ))
        .expect("failed result");
    assert_eq!(result.summary.outcome, SyncRunOutcomeV1::Failed);
    assert_eq!(result.summary.counts.failed, 1);
    assert!(result.items.is_empty());
}

#[test]
fn fresh_v6_health_is_rss_only_and_not_configured() {
    let store = DemoStore::open_in_memory().expect("fresh v6");
    let health = store.sync_health().expect("health");
    assert_eq!(health.contract_version, 1);
    assert_eq!(health.readiness.required_source_kinds, ["rss_atom"]);
    assert_eq!(
        health.readiness.status,
        DeliveryReadinessStatusV1::NotConfigured
    );
    assert_eq!(health.readiness.sources.len(), 1);
    assert!(health.latest_task.is_none());
}

#[test]
fn start_contract_is_exact_rss_only_snake_case_json() {
    let input = start_input(SyncTargetV1::AllEnabledRssAtom, "sync-wire");
    let value = serde_json::to_value(&input).expect("serialize");
    assert_eq!(
        value,
        serde_json::json!({
            "contract_version": 1,
            "target": {"kind": "all_enabled_rss_atom"},
            "idempotency_key": "sync-wire",
            "foreground_budget_ms": 30_000
        })
    );
    let mut unknown = value;
    unknown["generic_command"] = serde_json::json!("fetch https://example.com");
    assert!(serde_json::from_value::<StartSyncInputV1>(unknown).is_err());

    let mut invalid_budget = input;
    invalid_budget.foreground_budget_ms = 29_999;
    let mut store = DemoStore::open_in_memory().expect("v6");
    let error = store
        .start_sync(&invalid_budget)
        .expect_err("V1 budget is core-owned and fixed");
    assert_eq!(error.code(), "validation.source");
}

#[test]
fn start_is_idempotent_conflicting_payload_is_rejected_and_duplicate_active_is_blocked() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let source_id = save_source(&mut store, "https://example.com/one.xml", "source-sync-one");
    let input = start_input(
        SyncTargetV1::SourceId {
            source_id: source_id.clone(),
        },
        "sync-idempotent",
    );
    let first = store.start_sync(&input).expect("first start");
    assert_eq!(first.state, TaskStateV1::Queued);
    assert_eq!(first.task_id.len(), 29);
    assert_eq!(store.start_sync(&input).expect("replay"), first);

    let mut conflicting = input.clone();
    conflicting.target = SyncTargetV1::AllEnabledRssAtom;
    let conflict = store
        .start_sync(&conflicting)
        .expect_err("payload conflict");
    assert_eq!(conflict.code(), "conflict.source_revision");
    assert_eq!(conflict.task_id(), Some(first.task_id.as_str()));

    let duplicate = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-second-click",
        ))
        .expect_err("active source conflict");
    assert_eq!(duplicate.task_id(), Some(first.task_id.as_str()));
}

#[test]
fn claim_and_success_keep_checkpoint_task_and_readiness_consistent() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let source_id = save_source(
        &mut store,
        "https://example.com/success.xml",
        "source-sync-success",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-success",
        ))
        .expect("start");
    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim");
    assert_eq!(plan.task.state, TaskStateV1::Running);
    assert_eq!(plan.sources.len(), 1);

    let completed = store
        .commit_sync_source_success(
            &task.task_id,
            plan.task.revision,
            &plan.sources[0],
            &FetchIncrementalResult {
                candidates: Vec::new(),
                etag: Some("etag-v1".to_owned()),
                last_modified: None,
                adapter_cursor: Some("cursor-v1".to_owned()),
                not_modified: false,
            },
        )
        .expect("commit success");
    assert_eq!(completed.state, TaskStateV1::Succeeded);
    let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
    assert!(snapshot.finished_at.is_some());
    assert_eq!(snapshot.sources[0].state, TaskStateV1::Succeeded);
    assert!(snapshot.sources[0].last_success_at.is_some());
    assert_eq!(
        store.sync_health().expect("health").readiness.status,
        DeliveryReadinessStatusV1::Ready
    );
    let replay = store
        .fail_active_sync_task(&task.task_id)
        .expect("terminal recovery is idempotent");
    let after_replay = store.task_snapshot(&task.task_id).expect("snapshot replay");
    assert_eq!(replay.state, TaskStateV1::Succeeded);
    assert_eq!(after_replay.revision, snapshot.revision);
    assert_eq!(after_replay.finished_at, snapshot.finished_at);
}

#[test]
fn all_sync_preserves_success_when_another_source_enters_retry_wait() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    save_source(&mut store, "https://example.com/a.xml", "source-sync-a");
    save_source(&mut store, "https://example.com/b.xml", "source-sync-b");
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::AllEnabledRssAtom,
            "sync-all-partial",
        ))
        .expect("start all");
    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim all");
    assert_eq!(plan.sources.len(), 2);
    let running = store
        .commit_sync_source_success(
            &task.task_id,
            plan.task.revision,
            &plan.sources[0],
            &FetchIncrementalResult {
                candidates: Vec::new(),
                etag: None,
                last_modified: None,
                adapter_cursor: None,
                not_modified: true,
            },
        )
        .expect("first source");
    assert_eq!(running.state, TaskStateV1::Running);
    let rate_limited = AppError::from_code(ErrorCode::RateLimitedSource, "fixture-rate-limit")
        .with_source_id(&plan.sources[1].source_id)
        .with_retry_after_ms(120_000);
    let partial = store
        .commit_sync_source_failure(
            &task.task_id,
            running.revision,
            &plan.sources[1],
            &rate_limited,
            OBSERVED_AT_MS,
        )
        .expect("second source retry");
    assert_eq!(partial.state, TaskStateV1::PartiallySucceeded);
    let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
    assert_eq!(
        snapshot
            .sources
            .iter()
            .filter(|source| source.state == TaskStateV1::Succeeded)
            .count(),
        1
    );
    assert_eq!(
        snapshot
            .sources
            .iter()
            .filter(|source| source.state == TaskStateV1::RetryWait)
            .count(),
        1
    );
    let result = store
        .get_sync_result(&result_input(
            snapshot.result_ref.expect("v7 run"),
            None,
            100,
        ))
        .expect("partial result");
    assert_eq!(result.summary.outcome, SyncRunOutcomeV1::PartiallySucceeded);
    assert_eq!(result.summary.sources.len(), 2);
}

#[test]
fn retry_wait_reclaim_before_deadline_never_prepares_a_network_request() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let source_id = save_source(
        &mut store,
        "https://example.com/rate-limit.xml",
        "source-sync-rate-limit",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-rate-limit",
        ))
        .expect("start");
    let running = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim");
    let error = AppError::from_code(ErrorCode::RateLimitedSource, "fixture-rate-limit")
        .with_retry_after_ms(120_000);
    let retry_wait = store
        .commit_sync_source_failure(
            &task.task_id,
            running.task.revision,
            &running.sources[0],
            &error,
            FAR_FUTURE_RETRY_AT_MS,
        )
        .expect("persist future deadline");
    assert_eq!(retry_wait.state, TaskStateV1::RetryWait);

    let early = store
        .claim_sync_task(&task.task_id, retry_wait.revision)
        .expect("bounded early observation");
    assert_eq!(early.task.state, TaskStateV1::RetryWait);
    assert!(early.sources.is_empty());
}

#[test]
fn internal_worker_failure_fails_only_the_job_without_mutating_source_state() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let source_id = save_source(
        &mut store,
        "https://example.com/worker-panic.xml",
        "source-sync-worker-panic",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-worker-panic",
        ))
        .expect("start");
    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim");
    let source_before = store
        .source_fetch_state(&plan.sources[0].source_id)
        .expect("source before");

    let failed = store
        .commit_sync_source_internal_failure(&task.task_id, plan.task.revision, &plan.sources[0])
        .expect("record internal worker failure");

    assert_eq!(failed.state, TaskStateV1::Failed);
    let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
    assert_eq!(snapshot.sources[0].state, TaskStateV1::Failed);
    assert_eq!(
        snapshot.sources[0].error_code.as_deref(),
        Some("internal.unexpected")
    );
    let source_after = store
        .source_fetch_state(&plan.sources[0].source_id)
        .expect("source after");
    assert_eq!(source_after, source_before);
}

#[test]
fn reopening_marks_interrupted_running_task_failed_instead_of_resuming_in_background() {
    let database = ScopedDatabase::new();
    let mut store = DemoStore::open(&database.0).expect("v6");
    let source_id = save_source(
        &mut store,
        "https://example.com/recovery.xml",
        "source-sync-recovery",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-recovery",
        ))
        .expect("start");
    let running = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim");
    assert_eq!(running.task.state, TaskStateV1::Running);
    drop(store);

    let reopened = DemoStore::open(&database.0).expect("reopen");
    let recovered = reopened.task_snapshot(&task.task_id).expect("recovered");
    assert_eq!(recovered.state, TaskStateV1::Failed);
    assert_eq!(
        recovered.error_summary.as_deref(),
        Some("internal.unexpected")
    );
    assert_eq!(recovered.sources[0].state, TaskStateV1::Failed);
}

#[test]
fn reopening_marks_orphaned_queued_task_failed() {
    let database = ScopedDatabase::new();
    let mut store = DemoStore::open(&database.0).expect("v6");
    let source_id = save_source(
        &mut store,
        "https://example.com/queued-recovery.xml",
        "source-sync-queued-recovery",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-queued-recovery",
        ))
        .expect("start");
    drop(store);

    let reopened = DemoStore::open(&database.0).expect("reopen");
    let recovered = reopened.task_snapshot(&task.task_id).expect("recovered");
    assert_eq!(recovered.state, TaskStateV1::Failed);
    assert_eq!(recovered.sources[0].state, TaskStateV1::Failed);
}

#[test]
fn claim_rejects_source_revision_changed_after_task_snapshot() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let source_id = save_source(
        &mut store,
        "https://example.com/revision.xml",
        "source-sync-revision",
    );
    let task = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId {
                source_id: source_id.clone(),
            },
            "sync-frozen-revision",
        ))
        .expect("start");
    let request = store
        .prepare_incremental_fetch(&source_id)
        .expect("prepare source mutation");
    store
        .commit_incremental_fetch(
            &request,
            &FetchIncrementalResult {
                candidates: Vec::new(),
                etag: Some("changed".to_owned()),
                last_modified: None,
                adapter_cursor: None,
                not_modified: true,
            },
        )
        .expect("mutate source revision");

    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim records conflict");
    assert!(plan.sources.is_empty());
    assert_eq!(plan.task.state, TaskStateV1::Failed);
    let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
    assert_eq!(
        snapshot.sources[0].error_code.as_deref(),
        Some("conflict.source_revision")
    );
}

#[test]
fn all_sync_isolates_preexisting_retry_after_and_runs_other_sources() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let limited_id = save_source(
        &mut store,
        "https://example.com/a-limited.xml",
        "source-sync-prelimited",
    );
    let ready_id = save_source(
        &mut store,
        "https://example.com/b-ready.xml",
        "source-sync-ready",
    );
    let limited_request = store
        .prepare_incremental_fetch(&limited_id)
        .expect("prepare limited source");
    store
        .commit_incremental_failure(
            &limited_request,
            &AppError::from_code(ErrorCode::RateLimitedSource, "fixture-rate-limit")
                .with_retry_after_ms(120_000),
            FAR_FUTURE_RETRY_AT_MS,
        )
        .expect("persist future retry deadline");

    let task = store
        .start_sync(&start_input(
            SyncTargetV1::AllEnabledRssAtom,
            "sync-prelimited-all",
        ))
        .expect("all sync still starts");
    let plan = store
        .claim_sync_task(&task.task_id, task.revision)
        .expect("claim");
    assert_eq!(plan.sources.len(), 1);
    assert_eq!(plan.sources[0].source_id, ready_id);
    let snapshot = store.task_snapshot(&task.task_id).expect("snapshot");
    assert_eq!(
        snapshot
            .sources
            .iter()
            .find(|source| source.source_id == limited_id)
            .expect("limited source")
            .state,
        TaskStateV1::RetryWait
    );
}

#[test]
fn elapsed_retry_wait_is_terminalized_before_new_intent_claims_source() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let source_id = save_source(
        &mut store,
        "https://example.com/elapsed.xml",
        "source-sync-elapsed",
    );
    let first = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId {
                source_id: source_id.clone(),
            },
            "sync-elapsed-first",
        ))
        .expect("start first");
    let plan = store
        .claim_sync_task(&first.task_id, first.revision)
        .expect("claim first");
    let retry_wait = store
        .commit_sync_source_failure(
            &first.task_id,
            plan.task.revision,
            &plan.sources[0],
            &AppError::from_code(ErrorCode::NetworkSource, "fixture-network"),
            1,
        )
        .expect("persist elapsed retry");
    assert_eq!(retry_wait.state, TaskStateV1::RetryWait);

    let second = store
        .start_sync(&start_input(
            SyncTargetV1::SourceId { source_id },
            "sync-elapsed-second",
        ))
        .expect("new intent after deadline");
    assert_eq!(second.state, TaskStateV1::Queued);
    assert_eq!(
        store.task_ref(&first.task_id).expect("old task").state,
        TaskStateV1::Failed
    );
}

fn feed_input(
    stream: IntelFeedStreamV1,
    source_ids: Vec<String>,
    track_ids: Vec<String>,
    limit: u32,
) -> QueryIntelFeedInputV1 {
    QueryIntelFeedInputV1 {
        contract_version: 1,
        stream,
        filters: IntelFeedFiltersV1 {
            track_ids,
            source_ids,
            time_window: IntelFeedTimeWindowV1::AllTime,
            importance: Vec::new(),
        },
        sort: IntelFeedSortV1::ScoreDesc,
        cursor: None,
        limit,
    }
}

#[test]
fn real_feed_exposes_high_and_ordinary_without_mixing_demo() {
    let mut store = DemoStore::open_in_memory().expect("v10");
    store.bootstrap().expect("demo stays isolated");
    let source_id = save_source(
        &mut store,
        "https://feed-value.example/feed.xml",
        "source-intel-feed",
    );
    sync_one(
        &mut store,
        &source_id,
        "feed-high",
        candidate(
            "high",
            Some("AI agent security release improves deployment"),
            Some("https://feed-value.example/high"),
            "feed-high-v1",
        ),
    );
    sync_one(
        &mut store,
        &source_id,
        "feed-ordinary",
        candidate(
            "ordinary",
            Some("Quarterly community note"),
            Some("https://feed-value.example/ordinary"),
            "feed-ordinary-v1",
        ),
    );

    let high = store
        .query_intel_feed(&feed_input(
            IntelFeedStreamV1::HighValue,
            Vec::new(),
            Vec::new(),
            30,
        ))
        .expect("high feed");
    assert_eq!(high.items.len(), 1);
    assert_eq!(
        high.items[0].title,
        "AI agent security release improves deployment"
    );
    assert_eq!(high.items[0].source_id, source_id);
    assert_eq!(
        high.items[0].stream_disposition,
        IntelFeedStreamV1::HighValue
    );
    assert!(
        high.items[0]
            .matched_track_ids
            .contains(&"ai_agents".to_owned())
    );
    assert!(
        high.items
            .iter()
            .all(|item| item.intel_item_id.starts_with("intel:"))
    );

    let ordinary = store
        .query_intel_feed(&feed_input(
            IntelFeedStreamV1::OrdinaryCandidate,
            Vec::new(),
            Vec::new(),
            30,
        ))
        .expect("ordinary feed");
    assert_eq!(ordinary.items.len(), 1);
    assert_eq!(ordinary.items[0].title, "Quarterly community note");
}

#[test]
fn feed_filters_and_cursor_are_bound_to_the_query_identity() {
    let mut store = DemoStore::open_in_memory().expect("v10");
    let source_id = save_source(
        &mut store,
        "https://feed-page.example/feed.xml",
        "source-intel-page",
    );
    for (identity, title) in [
        ("one", "AI agent security release one"),
        ("two", "AI agent security release two"),
    ] {
        sync_one(
            &mut store,
            &source_id,
            &format!("feed-page-{identity}"),
            candidate(
                identity,
                Some(title),
                Some(&format!("https://feed-page.example/{identity}")),
                &format!("feed-page-{identity}-v1"),
            ),
        );
    }

    let mut input = feed_input(
        IntelFeedStreamV1::HighValue,
        vec![source_id.clone()],
        vec!["ai_agents".to_owned()],
        1,
    );
    let first = store.query_intel_feed(&input).expect("first page");
    assert_eq!(first.items.len(), 1);
    let mut combined = feed_input(
        IntelFeedStreamV1::HighValue,
        vec![source_id.clone()],
        vec!["ai_agents".to_owned()],
        30,
    );
    // The fixed-clock Last7d path is exercised by the isolated AC7 gate. This integration
    // case must not age out as the wall clock advances.
    combined.filters.time_window = IntelFeedTimeWindowV1::AllTime;
    combined.filters.importance = vec!["high".to_owned()];
    assert_eq!(
        store
            .query_intel_feed(&combined)
            .expect("combined filters")
            .items
            .len(),
        2
    );
    combined.filters.track_ids = vec!["foundation_models".to_owned()];
    assert!(
        store
            .query_intel_feed(&combined)
            .expect("track mismatch")
            .items
            .is_empty()
    );
    combined.filters.track_ids = vec!["ai_agents".to_owned()];
    combined.filters.importance = vec!["low".to_owned()];
    assert!(
        store
            .query_intel_feed(&combined)
            .expect("importance mismatch")
            .items
            .is_empty()
    );
    input.cursor = first.next_cursor.clone();
    let second = store.query_intel_feed(&input).expect("second page");
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].intel_item_id, second.items[0].intel_item_id);
    assert!(second.next_cursor.is_none());

    let mut mismatched = input;
    mismatched.filters.source_ids.clear();
    mismatched.cursor = first.next_cursor.clone();
    let error = store
        .query_intel_feed(&mismatched)
        .expect_err("cursor must bind filters");
    assert_eq!(error.code(), "validation.source");

    let mut tampered = feed_input(
        IntelFeedStreamV1::HighValue,
        vec![source_id],
        vec!["ai_agents".to_owned()],
        1,
    );
    tampered.cursor = first.next_cursor.map(|mut cursor| {
        cursor.push('0');
        cursor
    });
    assert_eq!(
        store
            .query_intel_feed(&tampered)
            .expect_err("tampered cursor")
            .code(),
        "validation.source"
    );
}

#[test]
fn health_aggregates_disjoint_active_tasks_without_hiding_sources() {
    let mut store = DemoStore::open_in_memory().expect("v6");
    let first_id = save_source(
        &mut store,
        "https://example.com/health-one.xml",
        "source-health-one",
    );
    let second_id = save_source(
        &mut store,
        "https://example.com/health-two.xml",
        "source-health-two",
    );
    for (source_id, key) in [
        (first_id, "sync-health-one"),
        (second_id, "sync-health-two"),
    ] {
        let task = store
            .start_sync(&start_input(SyncTargetV1::SourceId { source_id }, key))
            .expect("start disjoint task");
        store
            .claim_sync_task(&task.task_id, task.revision)
            .expect("claim disjoint task");
    }
    let health = store.sync_health().expect("health");
    assert_eq!(health.pending_task_count, 2);
    assert_eq!(health.source_results.len(), 2);
    assert!(
        health
            .latest_task
            .is_some_and(|task| task.state == TaskStateV1::Running)
    );
}
