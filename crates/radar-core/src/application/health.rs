use crate::contracts::dto::HealthStatus;

/// Returns the deterministic shared-core health contract.
#[must_use]
pub fn health_check() -> HealthStatus {
    HealthStatus {
        contract_version: 1,
        status: "ok".to_owned(),
        checked_at: None,
    }
}
