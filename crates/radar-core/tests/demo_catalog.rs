use radar_core::application::demo::{
    DataOrigin, DemoEffectPorts, DemoSideEffect, DemoStore, dispatch_origin_side_effects,
};

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
    assert_eq!(detail.data_origin, DataOrigin::Demo);

    let first_page = store.list_page(None, None, 2).expect("first page");
    assert_eq!(first_page.items.len(), 2);
    let second_page = store
        .list_page(None, first_page.next_cursor.as_deref(), 2)
        .expect("second page");
    assert_eq!(second_page.items.len(), 1);
    assert!(second_page.next_cursor.is_none());
    assert!(store.list_page(None, Some("invalid"), 2).is_err());
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
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/story-1-6-file-backed-test/ai-subscribe.sqlite3");
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
