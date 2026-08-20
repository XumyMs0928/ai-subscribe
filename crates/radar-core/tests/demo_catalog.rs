use radar_core::application::demo::{
    DataOrigin, DemoEffectPorts, DemoSideEffect, DemoStore, dispatch_origin_side_effects,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static FILE_BACKED_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct ScopedTestDir {
    path: PathBuf,
}

impl ScopedTestDir {
    fn new(label: &str) -> Self {
        let parent =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/story-1-6-file-backed-test");
        fs::create_dir_all(&parent).expect("file-backed test parent");
        loop {
            let sequence = FILE_BACKED_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("{label}-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("file-backed test directory: {error}"),
            }
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScopedTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Default)]
struct Counters([u32; 5]);

impl DemoEffectPorts for Counters {
    fn network(&mut self) {
        self.0[0] += 1;
    }
    fn ai_task(&mut self) {
        self.0[1] += 1;
    }
    fn notification(&mut self) {
        self.0[2] += 1;
    }
    fn validation_metric(&mut self) {
        self.0[3] += 1;
    }
    fn real_dedupe(&mut self) {
        self.0[4] += 1;
    }
}

#[test]
fn shared_demo_fixture_seeds_once_and_is_queryable() {
    let mut store = DemoStore::open_in_memory().expect("demo store");
    let first = store.bootstrap().expect("first bootstrap");
    let second = store.bootstrap().expect("repeat bootstrap");

    assert_eq!(first.dataset_id, "demo-v1");
    assert_eq!(first.items.len(), 3);
    assert_eq!(first, second);
    assert!(
        first
            .items
            .iter()
            .all(|item| item.data_origin == DataOrigin::Demo)
    );
    assert!(first.items.iter().all(|item| item.id.starts_with("demo:")));

    let matches = store.search("Rust", None).expect("search");
    assert_eq!(matches.items.len(), 1);
    assert_eq!(matches.items[0].publisher, "Rust Project");

    let short_matches = store.search("AI", None).expect("short search");
    assert!(!short_matches.items.is_empty());
    assert!(
        short_matches
            .items
            .iter()
            .any(|item| item.publisher == "OpenAI")
    );
    assert!(
        store
            .search("\"(", None)
            .expect("literal punctuation")
            .items
            .is_empty()
    );
    assert_eq!(
        store
            .search("", Some("   "))
            .expect("blank track")
            .items
            .len(),
        3
    );

    let filtered = store.search("", Some("本地模型")).expect("filter");
    assert_eq!(filtered.items.len(), 1);
    assert_eq!(filtered.items[0].track, "本地模型");

    let detail = store.detail("demo:openai-agents-sdk-001").expect("detail");
    assert_eq!(detail.contract_version, 1);
    assert_eq!(detail.dataset_id, "demo-v1");
    assert_eq!(detail.data_origin, DataOrigin::Demo);
    assert_eq!(detail.importance.as_str(), "high");
    assert_eq!(detail.ai_confidence_percent, 88);
    assert!(!detail.facts.is_empty());
    assert!(!detail.rule_reasons.is_empty());
    assert_eq!(detail.provenance.source_kind, "official_release");
    assert_eq!(detail.provenance.availability_status.as_str(), "available");

    let first_page = store.list_page(None, None, 2).expect("first page");
    assert_eq!(first_page.items.len(), 2);
    let second_page = store
        .list_page(None, first_page.next_cursor.as_deref(), 2)
        .expect("second page");
    assert_eq!(second_page.items.len(), 1);
    assert!(second_page.next_cursor.is_none());
    assert!(store.list_page(None, Some("invalid"), 2).is_err());
    assert!(
        first_page
            .next_cursor
            .as_deref()
            .is_some_and(|cursor| cursor.starts_with("v1:"))
    );
}

#[test]
fn demo_origin_blocks_every_real_side_effect() {
    for side_effect in DemoSideEffect::ALL {
        assert!(!side_effect.allowed_for_demo(), "{side_effect:?}");
    }

    let mut ports = Counters::default();
    dispatch_origin_side_effects(DataOrigin::Demo, &mut ports);
    assert_eq!(ports.0, [0; 5]);
    dispatch_origin_side_effects(DataOrigin::Real, &mut ports);
    assert_eq!(ports.0, [1; 5]);
}

#[test]
fn demo_and_real_namespaces_never_share_identity() {
    let store = DemoStore::open_in_memory().expect("demo store");
    assert_ne!(
        store.identity_key(DataOrigin::Demo, "shared-id"),
        store.identity_key(DataOrigin::Real, "shared-id")
    );
}

#[test]
fn file_backed_demo_store_bootstraps_with_the_release_database_contract() {
    let test_dir = ScopedTestDir::new("demo-catalog");
    let path = test_dir.path().join("ai-subscribe.sqlite3");
    let mut store = DemoStore::open(&path).expect("file-backed demo store");
    assert_eq!(
        store
            .bootstrap()
            .expect("file-backed bootstrap")
            .items
            .len(),
        3
    );
}
