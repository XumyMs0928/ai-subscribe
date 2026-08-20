use radar_core::application::demo::DemoStore;

/// [P1] Invalid pagination boundaries are redacted and leave the store queryable.
#[test]
fn pagination_failures_are_redacted_and_do_not_poison_the_store() {
    let mut store = DemoStore::open_in_memory().expect("demo store");
    store.bootstrap().expect("bootstrap demo fixture");

    for (cursor, limit, correlation_prefix) in [
        (None, 0, "demo-page-limit-"),
        (None, 101, "demo-page-limit-"),
        (Some("offset:4"), 1, "demo-page-cursor-"),
        (Some("v1:not-hex:not-hex"), 1, "demo-page-cursor-"),
    ] {
        let error = store
            .list_page(None, cursor, limit)
            .expect_err("invalid pagination must fail");

        assert_eq!(error.code(), "internal.unexpected");
        assert_eq!(error.category().as_str(), "internal");
        assert_eq!(error.message_key(), "error.internal");
        assert_eq!(error.retryability().as_str(), "manual");
        assert!(error.details_allowlisted().is_empty());
        assert!(error.correlation_id().starts_with(correlation_prefix));
    }

    let page = store
        .list_page(None, None, 2)
        .expect("valid pagination remains available");
    assert_eq!(page.items.len(), 2);
    assert!(
        page.next_cursor
            .as_deref()
            .is_some_and(|cursor| cursor.starts_with("v1:"))
    );

    let matches = store
        .search("Rust", None)
        .expect("search remains available after pagination failures");
    assert_eq!(matches.items.len(), 1);
    assert_eq!(matches.items[0].publisher, "Rust Project");
}
