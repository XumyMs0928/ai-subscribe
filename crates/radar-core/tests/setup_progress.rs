use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

use radar_core::application::demo::DemoStore;
use radar_core::application::setup::{
    SaveSetupStepInputV1, SetupAction, SetupStepId, SetupStepStatus,
};
use radar_core::contracts::dto::configuration_validation::SaveConfigurationInputV1;

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct ScopedTestDirectory(PathBuf);

impl ScopedTestDirectory {
    fn new() -> Self {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/story-1-8-setup-tests")
            .join(format!("{}-{sequence}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create isolated test directory");
        Self(path)
    }

    fn database(&self) -> PathBuf {
        self.0.join("ai-subscribe.sqlite3")
    }
}

impl Drop for ScopedTestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn input(
    step_id: SetupStepId,
    action: SetupAction,
    values: &[&str],
    revision: u64,
    configuration_revision: u64,
    idempotency_key: &str,
) -> SaveSetupStepInputV1 {
    SaveSetupStepInputV1 {
        contract_version: 1,
        step_id,
        action,
        selected_values: values.iter().map(|value| (*value).to_owned()).collect(),
        expected_revision: revision,
        expected_configuration_revision: configuration_revision,
        idempotency_key: idempotency_key.to_owned(),
    }
}

#[test]
fn fresh_progress_uses_shared_defaults_and_core_owned_next_step() {
    let store = DemoStore::open_in_memory().expect("store");
    let progress = store.get_setup_progress().expect("progress");

    assert_eq!(progress.contract_version, 1);
    assert_eq!(progress.revision, 0);
    assert_eq!(progress.next_step_id, Some(SetupStepId::Tracks));
    assert_eq!(progress.steps.len(), 4);
    assert_eq!(progress.defaults.tracks.len(), 3);
    assert_eq!(
        progress.saved_config.track_ids,
        vec!["ai_agents", "foundation_models", "local_models"]
    );
    assert_eq!(
        progress.saved_config.source_example_ids,
        vec!["github_releases", "arxiv_topics", "rss_feeds"]
    );
    assert_eq!(
        progress.saved_config.refresh_cadence.as_deref(),
        Some("manual")
    );
    assert!(
        progress
            .defaults
            .source_examples
            .iter()
            .all(|source| source.is_demo)
    );
}

#[test]
fn saved_values_survive_skip_later_and_reopen() {
    let directory = ScopedTestDirectory::new();
    let database = directory.database();
    let mut store = DemoStore::open(&database).expect("store");
    let saved = store
        .save_setup_step(&input(
            SetupStepId::Tracks,
            SetupAction::Save,
            &["ai_agents", "local_models"],
            0,
            1,
            "setup-save-tracks-1",
        ))
        .expect("save tracks");
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.next_step_id, Some(SetupStepId::SourceExamples));

    let skipped = store
        .save_setup_step(&input(
            SetupStepId::SourceExamples,
            SetupAction::Skip,
            &[],
            1,
            2,
            "setup-skip-sources-1",
        ))
        .expect("skip sources");
    assert_eq!(skipped.steps[1].status, SetupStepStatus::Skipped);

    let later = store
        .save_setup_step(&input(
            SetupStepId::SourceExamples,
            SetupAction::Later,
            &[],
            2,
            2,
            "setup-later-refresh-1",
        ))
        .expect("later");
    assert_eq!(later.next_step_id, Some(SetupStepId::SourceExamples));
    assert_eq!(
        later.saved_config.track_ids,
        vec!["ai_agents", "local_models"]
    );
    drop(store);
    let reopened = DemoStore::open(&database)
        .expect("reopen store")
        .get_setup_progress()
        .expect("reopened progress");
    assert_eq!(reopened.revision, 3);
    assert_eq!(
        reopened.saved_config.track_ids,
        later.saved_config.track_ids
    );
}

#[test]
fn concurrent_writers_allow_one_revision_winner_without_lost_updates() {
    let directory = ScopedTestDirectory::new();
    let database = directory.database();
    drop(DemoStore::open(&database).expect("initialize store"));
    let barrier = Arc::new(Barrier::new(2));
    let handles = ["ai_agents", "local_models"].map(|value| {
        let database = database.clone();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            let mut store = DemoStore::open(&database).expect("open concurrent store");
            barrier.wait();
            store.save_setup_step(&input(
                SetupStepId::Tracks,
                SetupAction::Save,
                &[value],
                0,
                1,
                &format!("concurrent-{value}"),
            ))
        })
    });
    let results = handles.map(|handle| handle.join().expect("join writer"));
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .filter(|error| error.code() == "conflict.setup_revision")
            .count(),
        1
    );
    assert_eq!(
        DemoStore::open(&database)
            .expect("reopen winner")
            .get_setup_progress()
            .expect("winner progress")
            .revision,
        1
    );
}

#[test]
fn retries_are_idempotent_and_stale_revisions_fail_closed() {
    let mut store = DemoStore::open_in_memory().expect("store");
    let request = input(
        SetupStepId::Tracks,
        SetupAction::Save,
        &["ai_agents"],
        0,
        1,
        "setup-idempotent-1",
    );
    let first = store.save_setup_step(&request).expect("first save");
    let replay = store.save_setup_step(&request).expect("idempotent replay");
    assert_eq!(first, replay);

    let later = input(
        SetupStepId::SourceExamples,
        SetupAction::Later,
        &[],
        first.revision,
        first.configuration_revision,
        "setup-idempotent-later",
    );
    let advanced = store.save_setup_step(&later).expect("advance setup");
    assert!(advanced.revision > first.revision);
    assert_eq!(
        store.save_setup_step(&request).expect("historical replay"),
        first,
        "replay must return the originally committed response"
    );

    let error = store
        .save_setup_step(&input(
            SetupStepId::SourceExamples,
            SetupAction::Save,
            &["github_releases"],
            0,
            1,
            "setup-stale-1",
        ))
        .expect_err("stale revision must fail");
    assert_eq!(error.code(), "conflict.setup_revision");
    assert_eq!(
        store.get_setup_progress().expect("recover").revision,
        advanced.revision
    );
}

#[test]
fn setup_cannot_overwrite_a_newer_rules_configuration_and_projects_custom_tracks() {
    let mut store = DemoStore::open_in_memory().expect("store");
    let initial_setup = store.get_setup_progress().expect("setup");
    let mut configuration = store
        .get_configuration()
        .expect("configuration")
        .configuration;
    configuration.tracks[0].name = "自定义 AI 智能体".into();
    let expected_hash = radar_core::domain::rules::configuration_validation::configuration_hash(
        &radar_core::domain::rules::configuration_validation::normalize(&configuration),
    );
    let saved = store
        .save_attention_configuration(&SaveConfigurationInputV1 {
            contract_version: 1,
            configuration,
            expected_revision: initial_setup.configuration_revision,
            expected_normalized_config_hash: expected_hash,
            idempotency_key: "rules-wins-before-setup".into(),
            validation_receipt: None,
        })
        .expect("rules save");

    let stale_setup = input(
        SetupStepId::Tracks,
        SetupAction::Save,
        &["ai_agents"],
        initial_setup.revision,
        initial_setup.configuration_revision,
        "stale-setup-after-rules",
    );
    let error = store
        .save_setup_step(&stale_setup)
        .expect_err("stale setup must not overwrite rules");
    assert_eq!(error.code(), "conflict.configuration_revision");
    assert_eq!(store.get_configuration().expect("unchanged"), saved);

    let projected = store.get_setup_progress().expect("projected setup");
    assert_eq!(projected.configuration_revision, saved.revision);
    assert!(projected.defaults.tracks.iter().any(|option| {
        option.id == "ai_agents" && option.label == "自定义 AI 智能体" && !option.is_demo
    }));
    assert!(
        projected
            .saved_config
            .track_ids
            .contains(&"ai_agents".into())
    );
}

#[test]
fn completing_steps_does_not_damage_demo_catalog() {
    let mut store = DemoStore::open_in_memory().expect("store");
    let before = store.bootstrap().expect("demo bootstrap");
    let actions = [
        (SetupStepId::Tracks, vec!["ai_agents"]),
        (SetupStepId::SourceExamples, vec!["github_releases"]),
        (SetupStepId::RefreshCadence, vec!["manual"]),
        (SetupStepId::AiDataDisclosure, vec!["acknowledged"]),
    ];
    let configuration_revisions = [1_u64, 2, 2, 3];
    for (index, (step, values)) in actions.into_iter().enumerate() {
        store
            .save_setup_step(&input(
                step,
                SetupAction::Save,
                &values,
                index as u64,
                configuration_revisions[index],
                &format!("setup-complete-{index}"),
            ))
            .expect("complete step");
    }
    let progress = store.get_setup_progress().expect("progress");
    assert_eq!(progress.next_step_id, None);
    assert_eq!(progress.overall_status, SetupStepStatus::Completed);
    assert_eq!(store.bootstrap().expect("demo remains").items, before.items);
}

#[test]
fn malformed_inputs_are_redacted_and_leave_saved_configuration_unchanged() {
    let mut store = DemoStore::open_in_memory().expect("store");
    let valid = input(
        SetupStepId::Tracks,
        SetupAction::Save,
        &["ai_agents"],
        0,
        1,
        "setup-valid-1",
    );
    store.save_setup_step(&valid).expect("valid save");

    for invalid in [
        SaveSetupStepInputV1 {
            contract_version: 2,
            expected_revision: 1,
            idempotency_key: "invalid-version".to_owned(),
            ..valid.clone()
        },
        SaveSetupStepInputV1 {
            selected_values: vec!["unknown_track".to_owned()],
            expected_revision: 1,
            idempotency_key: "invalid-option".to_owned(),
            ..valid.clone()
        },
        SaveSetupStepInputV1 {
            selected_values: vec!["ai_agents".to_owned(), "ai_agents".to_owned()],
            expected_revision: 1,
            idempotency_key: "duplicate-option".to_owned(),
            ..valid.clone()
        },
        SaveSetupStepInputV1 {
            selected_values: Vec::new(),
            expected_revision: 1,
            idempotency_key: "invalid-empty".to_owned(),
            ..valid.clone()
        },
    ] {
        let error = store.save_setup_step(&invalid).expect_err("invalid input");
        assert_eq!(error.code(), "validation.setup_input");
        assert!(error.details_allowlisted().is_empty());
    }
    let recovered = store.get_setup_progress().expect("recover");
    assert_eq!(recovered.revision, 1);
    assert_eq!(recovered.saved_config.track_ids, vec!["ai_agents"]);

    for malformed in [
        r#"{"contract_version":1,"step_id":"unknown","action":"save","selected_values":["ai_agents"],"expected_revision":1,"expected_configuration_revision":1,"idempotency_key":"unknown"}"#,
        r#"{"contract_version":1,"step_id":"tracks","action":"unknown","selected_values":["ai_agents"],"expected_revision":1,"expected_configuration_revision":1,"idempotency_key":"unknown"}"#,
    ] {
        assert!(serde_json::from_str::<SaveSetupStepInputV1>(malformed).is_err());
    }
}
