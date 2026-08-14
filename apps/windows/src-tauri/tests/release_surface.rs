use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_command_table_contains_only_approved_story_1_6_commands() {
    let source = fs::read_to_string(crate_root().join("src/lib.rs")).expect("read Tauri lib");
    let code = source
        .lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<String>()
        .split_whitespace()
        .collect::<String>();
    let handlers = code.match_indices("generate_handler![").collect::<Vec<_>>();
    assert_eq!(handlers.len(), 1, "release must define exactly one handler");
    let handler = code[handlers[0].0..]
        .split("generate_handler![")
        .nth(1)
        .and_then(|tail| tail.split(']').next())
        .expect("release invoke handler");
    assert_eq!(
        handler.trim(),
        "commands::health_v1,commands::demo_bootstrap_v1,commands::demo_search_v1,commands::demo_list_v1,commands::demo_filter_v1,commands::demo_detail_v1"
    );
    assert!(!source.contains("secret_probe"));
    assert!(!source.contains("panic_probe"));
    assert!(!source.contains("effect_probe"));
}

#[test]
fn release_configuration_has_a_local_only_csp_and_minimal_capability() {
    let config = fs::read_to_string(crate_root().join("tauri.conf.json")).expect("read config");
    let capability =
        fs::read_to_string(crate_root().join("capabilities/main.json")).expect("read capability");
    let parsed: serde_json::Value = serde_json::from_str(&config).expect("parse Tauri config");
    let parsed_capability: serde_json::Value =
        serde_json::from_str(&capability).expect("parse release capability");
    let csp = parsed["app"]["security"]["csp"]
        .as_str()
        .expect("release CSP string");

    assert_eq!(
        csp,
        "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src ipc: http://ipc.localhost"
    );
    assert!(
        !parsed["build"]
            .as_object()
            .is_some_and(|build| build.contains_key("devUrl"))
    );
    assert_eq!(parsed_capability["identifier"], "main-capability");
    assert_eq!(parsed_capability["windows"], serde_json::json!(["main"]));
    assert_eq!(
        parsed_capability["permissions"],
        serde_json::json!(["core:default"])
    );
}
