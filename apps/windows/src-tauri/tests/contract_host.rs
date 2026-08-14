use ai_subscribe_windows::commands::health_v1;
use radar_core::contracts::effects::{EffectLedger, EffectStatus, PlatformEffect, ReportResult};
use radar_core::contracts::errors::AppError;
use radar_ffi::error::{map_unknown, run_guarded};
use radar_ffi::mapping::AppErrorWire;
use serde::Deserialize;

const TEST_COMMAND: &str = "contract_probe_v1";

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ProbeRequest {
    Validation { effect_id: String },
    Unknown { private_text: String },
    Panic { private_text: String },
    Effect { status: String },
}

struct WindowsTestCommandHost {
    ledger: EffectLedger,
    observable_side_effects: usize,
}

impl WindowsTestCommandHost {
    fn new() -> Self {
        let effect = PlatformEffect::new_contract_probe(
            "effect:windows:1",
            "idempotency:windows:1",
            "2026-01-01T00:00:00Z",
        )
        .expect("valid effect");
        let mut ledger = EffectLedger::default();
        ledger.register(effect).expect("register effect");
        Self {
            ledger,
            observable_side_effects: 0,
        }
    }

    fn invoke(&mut self, command: &str, payload: &str) -> Result<String, String> {
        assert_eq!(command, TEST_COMMAND, "test host exposes one fixed command");
        let request: ProbeRequest =
            serde_json::from_str(payload).expect("deserialize command input");
        self.dispatch(request)
            .and_then(|value| {
                serde_json::to_string(&value)
                    .map_err(|_| AppError::internal_generated("windows-test-serialize"))
            })
            .map_err(|error| {
                serde_json::to_string(&error_json(&error)).expect("serialize command error")
            })
    }

    fn dispatch(&mut self, request: ProbeRequest) -> Result<serde_json::Value, AppError> {
        match request {
            ProbeRequest::Validation { effect_id } => {
                PlatformEffect::new_contract_probe(&effect_id, "key:1", "2026-01-01T00:00:00Z")
                    .map(|_| serde_json::json!({ "accepted": true }))
            }
            ProbeRequest::Unknown { private_text } => {
                map_unknown::<serde_json::Value>(private_text)
            }
            ProbeRequest::Panic { private_text } => run_guarded(|| {
                panic!("{private_text}");
                #[allow(unreachable_code)]
                Ok(serde_json::Value::Null)
            }),
            ProbeRequest::Effect { status } => {
                let status = match status.as_str() {
                    "delivered" => EffectStatus::Delivered,
                    "failed" => EffectStatus::Failed,
                    _ => return Err(AppError::internal_generated("windows-test-effect-status")),
                };
                let result =
                    self.ledger
                        .report("effect:windows:1", "idempotency:windows:1", status)?;
                if result == ReportResult::Applied {
                    self.observable_side_effects += 1;
                }
                Ok(serde_json::json!({
                    "result": match result {
                        ReportResult::Applied => "applied",
                        ReportResult::AlreadyApplied => "already_applied",
                    }
                }))
            }
        }
    }
}

fn error_json(error: &AppError) -> serde_json::Value {
    let wire = AppErrorWire::from(error);
    serde_json::json!({
        "contract_version": wire.contract_version,
        "code": wire.code,
        "category": wire.category,
        "message_key": wire.message_key,
        "retryability": wire.retryability,
        "source_id": wire.source_id,
        "task_id": wire.task_id,
        "details_allowlisted": wire.details_allowlisted,
        "correlation_id": wire.correlation_id,
    })
}

fn invoke_error(
    host: &mut WindowsTestCommandHost,
    payload: &serde_json::Value,
) -> serde_json::Value {
    let error = host
        .invoke(TEST_COMMAND, &payload.to_string())
        .expect_err("probe must return a command error");
    serde_json::from_str(&error).expect("deserialize command error")
}

#[test]
fn windows_commands_preserve_health_validation_unknown_and_panic_contracts() {
    let health = serde_json::to_value(health_v1().expect("health command succeeds"))
        .expect("serialize health command response");
    assert_eq!(health["contract_version"], 1);
    assert_eq!(health["status"], "ok");
    assert!(health["checked_at"].is_null());

    let mut host = WindowsTestCommandHost::new();
    let validation = invoke_error(
        &mut host,
        &serde_json::json!({ "kind": "validation", "effect_id": "invalid/id" }),
    );
    assert_eq!(validation["code"], "validation.effect_id");
    assert_eq!(validation["category"], "validation");

    for (kind, private_text) in [
        ("unknown", "private provider detail"),
        ("panic", "private panic credential detail"),
    ] {
        let error = invoke_error(
            &mut host,
            &serde_json::json!({ "kind": kind, "private_text": private_text }),
        );
        assert_eq!(error["contract_version"], 1);
        assert_eq!(error["code"], "internal.unexpected");
        assert_eq!(error["category"], "internal");
        assert_eq!(error["retryability"], "manual");
        assert_eq!(error["message_key"], "error.internal");
        assert!(error["source_id"].is_null());
        assert!(error["task_id"].is_null());
        assert_eq!(error["details_allowlisted"], "");
        assert!(
            error["correlation_id"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(!error.to_string().contains(private_text));
    }
}

#[test]
fn windows_effect_command_executes_the_observable_side_effect_once() {
    let mut host = WindowsTestCommandHost::new();
    let first = host
        .invoke(
            TEST_COMMAND,
            &serde_json::json!({ "kind": "effect", "status": "delivered" }).to_string(),
        )
        .expect("first report");
    let repeat = host
        .invoke(
            TEST_COMMAND,
            &serde_json::json!({ "kind": "effect", "status": "delivered" }).to_string(),
        )
        .expect("repeat report");
    let conflict = invoke_error(
        &mut host,
        &serde_json::json!({ "kind": "effect", "status": "failed" }),
    );

    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&first).unwrap()["result"],
        "applied"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&repeat).unwrap()["result"],
        "already_applied"
    );
    assert_eq!(conflict["code"], "conflict.effect_already_reported");
    assert_eq!(host.observable_side_effects, 1);
    assert_eq!(
        host.ledger.status("idempotency:windows:1"),
        Some(EffectStatus::Delivered)
    );
}
