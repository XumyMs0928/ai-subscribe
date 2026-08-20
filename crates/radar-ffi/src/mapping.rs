//! DTO mapping only; no business rules belong here.

use std::fmt::Write as _;

use radar_core::contracts::dto::HealthStatus;
use radar_core::contracts::errors::AppError;

pub use radar_core::contracts::dto::configuration_validation::{
    AttentionConfigurationV1 as AttentionConfigurationWireV1,
    ConfigurationValidationResultV1 as ConfigurationValidationResultWireV1,
    ConfigurationViewV1 as ConfigurationViewWireV1,
    SaveConfigurationInputV1 as SaveConfigurationInputWireV1,
    ValidateConfigurationInputV1 as ValidateConfigurationInputWireV1,
};
pub use radar_core::contracts::dto::source::{
    SaveSourceInputV1 as SaveSourceInputWireV1, SourcePageV1 as SourcePageWireV1,
    SourceViewV1 as SourceViewWireV1,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthStatusWire {
    pub contract_version: u32,
    pub status: String,
    pub checked_at: Option<String>,
}

impl From<HealthStatus> for HealthStatusWire {
    fn from(value: HealthStatus) -> Self {
        Self {
            contract_version: value.contract_version,
            status: value.status,
            checked_at: value.checked_at,
        }
    }
}

impl HealthStatusWire {
    #[must_use]
    pub fn to_json(&self) -> String {
        let checked_at = self.checked_at.as_deref().map_or_else(
            || "null".to_owned(),
            |value| format!("\"{}\"", escape_json(value)),
        );
        format!(
            "{{\"contract_version\":{},\"status\":\"{}\",\"checked_at\":{checked_at}}}",
            self.contract_version,
            escape_json(&self.status)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppErrorWire {
    pub contract_version: u32,
    pub code: &'static str,
    pub category: &'static str,
    pub message_key: &'static str,
    pub retryability: &'static str,
    pub source_id: Option<String>,
    pub task_id: Option<String>,
    pub details_allowlisted: String,
    pub correlation_id: String,
}

impl From<&AppError> for AppErrorWire {
    fn from(value: &AppError) -> Self {
        Self {
            contract_version: value.contract_version(),
            code: value.code(),
            category: value.category().as_str(),
            message_key: value.message_key(),
            retryability: value.retryability().as_str(),
            source_id: value.source_id().map(str::to_owned),
            task_id: value.task_id().map(str::to_owned),
            details_allowlisted: value.details_allowlisted().to_owned(),
            correlation_id: value.correlation_id().to_owned(),
        }
    }
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character <= '\u{001f}' => {
                write!(escaped, "\\u{:04x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
            character => escaped.push(character),
        }
    }
    escaped
}
