//! Device-local progressive setup owned by the shared core.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use super::configuration::{append_configuration_version, read_configuration};
use super::demo::DemoStore;
use crate::contracts::dto::configuration_validation::AttentionTrackV1;
use crate::contracts::errors::{AppError, ErrorCode};

const DEFAULTS_JSON: &str = include_str!("../../../../contracts/fixtures/setup/defaults-v1.json");
static SETUP_ERROR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SetupStepId {
    Tracks,
    SourceExamples,
    RefreshCadence,
    AiDataDisclosure,
}

impl SetupStepId {
    pub const ALL: [Self; 4] = [
        Self::Tracks,
        Self::SourceExamples,
        Self::RefreshCadence,
        Self::AiDataDisclosure,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tracks => "tracks",
            Self::SourceExamples => "source_examples",
            Self::RefreshCadence => "refresh_cadence",
            Self::AiDataDisclosure => "ai_data_disclosure",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SetupStepStatus {
    NotStarted,
    InProgress,
    Skipped,
    PartiallyCompleted,
    Completed,
}

impl SetupStepStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Skipped => "skipped",
            Self::PartiallyCompleted => "partially_completed",
            Self::Completed => "completed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SetupAction {
    Save,
    Skip,
    Later,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupOptionV1 {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub is_demo: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupDefaultsV1 {
    pub contract_version: u32,
    pub fixture_id: String,
    pub default_track_ids: Vec<String>,
    pub default_source_example_ids: Vec<String>,
    pub default_refresh_cadence: String,
    pub tracks: Vec<SetupOptionV1>,
    pub source_examples: Vec<SetupOptionV1>,
    pub refresh_cadences: Vec<SetupOptionV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupStepProgressV1 {
    pub contract_version: u32,
    pub step_id: SetupStepId,
    pub status: SetupStepStatus,
    pub saved_fields_version: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupSavedConfigV1 {
    pub track_ids: Vec<String>,
    pub source_example_ids: Vec<String>,
    pub refresh_cadence: Option<String>,
    pub ai_data_disclosure_acknowledged: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupProgressV1 {
    pub contract_version: u32,
    pub revision: u64,
    pub configuration_revision: u64,
    pub overall_status: SetupStepStatus,
    pub steps: Vec<SetupStepProgressV1>,
    pub next_step_id: Option<SetupStepId>,
    pub defaults: SetupDefaultsV1,
    pub saved_config: SetupSavedConfigV1,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaveSetupStepInputV1 {
    pub contract_version: u32,
    pub step_id: SetupStepId,
    pub action: SetupAction,
    pub selected_values: Vec<String>,
    pub expected_revision: u64,
    pub expected_configuration_revision: u64,
    pub idempotency_key: String,
}

impl DemoStore {
    /// Returns core-owned defaults, saved values, and the next applicable step.
    ///
    /// # Errors
    /// Returns a stable storage error if device-local progress cannot be read.
    pub fn get_setup_progress(&self) -> Result<SetupProgressV1, AppError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "read-transaction"))?;
        let defaults = setup_defaults_for_connection(&transaction)?;
        let progress = read_setup_progress(&transaction, defaults)?;
        transaction
            .rollback()
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "read-rollback"))?;
        Ok(progress)
    }

    /// Atomically saves one setup intent and its progress marker.
    ///
    /// # Errors
    /// Rejects invalid input, stale revisions, or storage failures with stable errors.
    pub fn save_setup_step(
        &mut self,
        input: &SaveSetupStepInputV1,
    ) -> Result<SetupProgressV1, AppError> {
        let fingerprint = request_fingerprint(input);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "transaction"))?;
        let defaults = setup_defaults_for_connection(&transaction)?;
        validate_input(input, &defaults)?;
        if let Some((existing, response_json)) = transaction
            .query_row(
                "SELECT request_fingerprint,response_json FROM setup_idempotency WHERE idempotency_key=?1",
                [&input.idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "read-idempotency"))?
        {
            if existing != fingerprint {
                return Err(setup_error(
                    ErrorCode::ValidationSetupInput,
                    "idempotency-reuse",
                ));
            }
            let progress = serde_json::from_str(&response_json)
                .map_err(|_| setup_error(ErrorCode::StorageSetup, "idempotency-response"))?;
            transaction
                .rollback()
                .map_err(|_| setup_error(ErrorCode::StorageSetup, "replay-rollback"))?;
            return Ok(progress);
        }
        let current = read_setup_progress(&transaction, defaults.clone())?;
        if current.revision != input.expected_revision {
            return Err(setup_error(
                ErrorCode::ConflictSetupRevision,
                "stale-revision",
            ));
        }
        if current.configuration_revision != input.expected_configuration_revision {
            return Err(setup_error(
                ErrorCode::ConflictConfigurationRevision,
                "stale-configuration-revision",
            ));
        }
        let current_step = current
            .steps
            .iter()
            .find(|step| step.step_id == input.step_id)
            .ok_or_else(|| setup_error(ErrorCode::StorageSetup, "missing-step"))?;
        if current_step.status == SetupStepStatus::Completed && input.action != SetupAction::Save {
            return Err(setup_error(
                ErrorCode::ValidationSetupInput,
                "completed-transition",
            ));
        }
        let status = match input.action {
            SetupAction::Save => SetupStepStatus::Completed,
            SetupAction::Skip => SetupStepStatus::Skipped,
            SetupAction::Later => SetupStepStatus::InProgress,
        };
        let next_revision = current
            .revision
            .checked_add(1)
            .ok_or_else(|| setup_error(ErrorCode::ValidationSetupInput, "revision-overflow"))?;
        let next_revision_sql = i64::try_from(next_revision)
            .map_err(|_| setup_error(ErrorCode::ValidationSetupInput, "revision-overflow"))?;
        apply_setup_configuration(&transaction, input, &defaults)?;
        persist_setup_intent(
            &transaction,
            input,
            &fingerprint,
            status,
            next_revision,
            next_revision_sql,
            current_step.saved_fields_version,
        )?;
        let response = read_setup_progress(&transaction, defaults)?;
        let response_json = serde_json::to_string(&response)
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "response-json"))?;
        let updated = transaction
            .execute(
                "UPDATE setup_idempotency SET response_json=?1 WHERE idempotency_key=?2",
                params![response_json, input.idempotency_key],
            )
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "idempotency-response-write"))?;
        if updated != 1 {
            return Err(setup_error(
                ErrorCode::StorageSetup,
                "idempotency-response-missing",
            ));
        }
        transaction
            .commit()
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "commit"))?;
        Ok(response)
    }
}

#[allow(clippy::too_many_arguments)]
fn persist_setup_intent(
    transaction: &Transaction<'_>,
    input: &SaveSetupStepInputV1,
    fingerprint: &str,
    status: SetupStepStatus,
    next_revision: u64,
    next_revision_sql: i64,
    existing_fields_version: Option<u32>,
) -> Result<(), AppError> {
    if input.action == SetupAction::Save {
        let (key, value) = config_entry(input.step_id, &input.selected_values)?;
        transaction
            .execute(
                "INSERT INTO settings_metadata(key,value) VALUES (?1,?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "save-config"))?;
    }
    transaction
        .execute(
            "INSERT INTO setup_progress(step_id,status,revision,saved_fields_version,updated_at_ms)
             VALUES (?1,?2,?3,?4,CAST(unixepoch('subsec') * 1000 AS INTEGER))
             ON CONFLICT(step_id) DO UPDATE SET status=excluded.status,revision=excluded.revision,
             saved_fields_version=excluded.saved_fields_version,updated_at_ms=excluded.updated_at_ms",
            params![
                input.step_id.as_str(),
                status.as_str(),
                next_revision_sql,
                if input.action == SetupAction::Save {
                    Some(1_u32)
                } else {
                    existing_fields_version
                }
            ],
        )
        .and_then(|_| {
            transaction.execute(
                "INSERT INTO app_metadata(key,value) VALUES ('setup_revision',?1)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                [next_revision.to_string()],
            )
        })
        .and_then(|_| {
            transaction.execute(
                "INSERT INTO setup_idempotency(idempotency_key,request_fingerprint,response_revision)
                 VALUES (?1,?2,?3)",
                params![input.idempotency_key, fingerprint, next_revision_sql],
            )
        })
        .and_then(|_| {
            transaction.execute(
                "DELETE FROM setup_idempotency
                 WHERE rowid NOT IN (
                   SELECT rowid FROM setup_idempotency
                   ORDER BY response_revision DESC, rowid DESC LIMIT 64
                 )",
                [],
            )
        })
        .map_err(|_| setup_error(ErrorCode::StorageSetup, "save-progress"))?;
    Ok(())
}

fn read_setup_progress(
    connection: &Connection,
    defaults: SetupDefaultsV1,
) -> Result<SetupProgressV1, AppError> {
    let configuration_revision = connection
        .query_row(
            "SELECT version FROM configuration_current WHERE singleton_id=1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| setup_error(ErrorCode::StorageSetup, "read-configuration-revision"))
        .and_then(|value| {
            u64::try_from(value)
                .map_err(|_| setup_error(ErrorCode::StorageSetup, "configuration-revision"))
        })?;
    let revision = connection
        .query_row(
            "SELECT value FROM app_metadata WHERE key='setup_revision'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| setup_error(ErrorCode::StorageSetup, "read-revision"))?
        .map_or(Ok(0), |value| {
            value
                .parse::<u64>()
                .map_err(|_| setup_error(ErrorCode::StorageSetup, "parse-revision"))
        })?;
    let mut steps = Vec::with_capacity(SetupStepId::ALL.len());
    for step_id in SetupStepId::ALL {
        let row = connection
            .query_row(
                "SELECT status,saved_fields_version FROM setup_progress WHERE step_id=?1",
                [step_id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<u32>>(1)?)),
            )
            .optional()
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "read-step"))?;
        let (status, saved_fields_version) = row.map_or(
            Ok((SetupStepStatus::NotStarted, None)),
            |(status, version)| Ok((parse_status(&status)?, version)),
        )?;
        steps.push(SetupStepProgressV1 {
            contract_version: 1,
            step_id,
            status,
            saved_fields_version,
        });
    }
    let saved_config = read_saved_config(connection, &defaults)?;
    Ok(build_progress(
        revision,
        configuration_revision,
        steps,
        defaults,
        saved_config,
    ))
}

fn read_saved_config(
    connection: &Connection,
    defaults: &SetupDefaultsV1,
) -> Result<SetupSavedConfigV1, AppError> {
    let configuration = read_configuration(connection)?;
    let read = |key: &str| {
        connection
            .query_row(
                "SELECT value FROM settings_metadata WHERE key=?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| setup_error(ErrorCode::StorageSetup, "read-config"))
    };
    Ok(SetupSavedConfigV1 {
        track_ids: configuration
            .configuration
            .tracks
            .iter()
            .filter(|track| track.enabled)
            .map(|track| track.id.clone())
            .collect(),
        source_example_ids: read("source_example_ids")?.map_or(
            Ok(defaults.default_source_example_ids.clone()),
            |value| {
                serde_json::from_str(&value)
                    .map_err(|_| setup_error(ErrorCode::StorageSetup, "source-json"))
            },
        )?,
        refresh_cadence: Some(configuration_cadence(
            configuration.configuration.refresh_enabled,
            configuration.configuration.refresh_interval_minutes,
        )),
        ai_data_disclosure_acknowledged: read("ai_data_disclosure_acknowledged")?.as_deref()
            == Some("true"),
    })
}

fn setup_defaults_for_connection(connection: &Connection) -> Result<SetupDefaultsV1, AppError> {
    let mut defaults = parse_defaults()?;
    let configuration = read_configuration(connection)?;
    for track in &configuration.configuration.tracks {
        if let Some(option) = defaults
            .tracks
            .iter_mut()
            .find(|option| option.id == track.id)
        {
            option.label.clone_from(&track.name);
            option.is_demo = false;
        } else {
            defaults.tracks.push(SetupOptionV1 {
                id: track.id.clone(),
                label: track.name.clone(),
                is_demo: false,
            });
        }
    }
    let cadence = configuration_cadence(
        configuration.configuration.refresh_enabled,
        configuration.configuration.refresh_interval_minutes,
    );
    if !defaults
        .refresh_cadences
        .iter()
        .any(|option| option.id == cadence)
    {
        defaults.refresh_cadences.push(SetupOptionV1 {
            id: cadence.clone(),
            label: format!(
                "每 {} 分钟",
                configuration.configuration.refresh_interval_minutes
            ),
            is_demo: false,
        });
    }
    Ok(defaults)
}

fn configuration_cadence(enabled: bool, minutes: u32) -> String {
    if enabled {
        match minutes {
            60 => "hourly".to_owned(),
            1_440 => "daily".to_owned(),
            10_080 => "weekly".to_owned(),
            value => format!("minutes:{value}"),
        }
    } else {
        "manual".to_owned()
    }
}

fn cadence_configuration(cadence: &str) -> Option<(bool, u32)> {
    match cadence {
        "manual" => Some((false, 1_440)),
        "hourly" => Some((true, 60)),
        "daily" => Some((true, 1_440)),
        "weekly" => Some((true, 10_080)),
        value => value
            .strip_prefix("minutes:")
            .and_then(|minutes| minutes.parse::<u32>().ok())
            .map(|minutes| (true, minutes)),
    }
}

fn apply_setup_configuration(
    transaction: &Transaction<'_>,
    input: &SaveSetupStepInputV1,
    defaults: &SetupDefaultsV1,
) -> Result<(), AppError> {
    if input.action != SetupAction::Save
        || !matches!(
            input.step_id,
            SetupStepId::Tracks | SetupStepId::RefreshCadence
        )
    {
        return Ok(());
    }
    let current = read_configuration(transaction)?;
    let mut configuration = current.configuration;
    match input.step_id {
        SetupStepId::Tracks => {
            configuration.tracks = input
                .selected_values
                .iter()
                .map(|id| {
                    configuration
                        .tracks
                        .iter()
                        .find(|track| track.id == *id)
                        .cloned()
                        .or_else(|| {
                            defaults
                                .tracks
                                .iter()
                                .find(|option| option.id == *id)
                                .map(|option| AttentionTrackV1 {
                                    id: option.id.clone(),
                                    name: option.label.clone(),
                                    enabled: true,
                                })
                        })
                        .ok_or_else(|| setup_error(ErrorCode::ValidationSetupInput, "track-option"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            configuration
                .tracks
                .iter_mut()
                .for_each(|track| track.enabled = true);
        }
        SetupStepId::RefreshCadence => {
            let (enabled, minutes) = cadence_configuration(&input.selected_values[0])
                .ok_or_else(|| setup_error(ErrorCode::ValidationSetupInput, "cadence"))?;
            configuration.refresh_enabled = enabled;
            configuration.refresh_interval_minutes = minutes;
        }
        SetupStepId::SourceExamples | SetupStepId::AiDataDisclosure => unreachable!(),
    }
    append_configuration_version(
        transaction,
        &configuration,
        input.expected_configuration_revision,
    )?;
    Ok(())
}

fn parse_defaults() -> Result<SetupDefaultsV1, AppError> {
    let defaults: SetupDefaultsV1 = serde_json::from_str(DEFAULTS_JSON)
        .map_err(|_| setup_error(ErrorCode::ValidationSetupInput, "defaults-json"))?;
    let valid = defaults.contract_version == 1
        && defaults.fixture_id == "setup-defaults-v1"
        && !defaults.tracks.is_empty()
        && !defaults.source_examples.is_empty()
        && !defaults.refresh_cadences.is_empty()
        && defaults.source_examples.iter().all(|value| value.is_demo)
        && unique_options(&defaults.tracks)
        && unique_options(&defaults.source_examples)
        && unique_options(&defaults.refresh_cadences)
        && ids_are_unique_members(&defaults.default_track_ids, &defaults.tracks)
        && ids_are_unique_members(
            &defaults.default_source_example_ids,
            &defaults.source_examples,
        )
        && defaults
            .refresh_cadences
            .iter()
            .any(|option| option.id == defaults.default_refresh_cadence);
    if valid {
        Ok(defaults)
    } else {
        Err(setup_error(
            ErrorCode::ValidationSetupInput,
            "defaults-invalid",
        ))
    }
}

fn ids_are_unique_members(ids: &[String], options: &[SetupOptionV1]) -> bool {
    let allowed: HashSet<&str> = options.iter().map(|option| option.id.as_str()).collect();
    let mut seen = HashSet::new();
    !ids.is_empty()
        && ids
            .iter()
            .all(|id| allowed.contains(id.as_str()) && seen.insert(id))
}

fn unique_options(options: &[SetupOptionV1]) -> bool {
    let mut ids = HashSet::new();
    options.iter().all(|option| {
        !option.id.is_empty()
            && option.id.len() <= 64
            && option.id.is_ascii()
            && !option.label.trim().is_empty()
            && ids.insert(&option.id)
    })
}

fn validate_input(
    input: &SaveSetupStepInputV1,
    defaults: &SetupDefaultsV1,
) -> Result<(), AppError> {
    let key_valid = !input.idempotency_key.is_empty()
        && input.idempotency_key.len() <= 128
        && input.idempotency_key.is_ascii()
        && input
            .idempotency_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'));
    let mut values = HashSet::new();
    let values_valid = input.selected_values.len() <= 16
        && input.selected_values.iter().all(|value| {
            !value.is_empty() && value.len() <= 64 && value.is_ascii() && values.insert(value)
        });
    let allowed: HashSet<&str> = match input.step_id {
        SetupStepId::Tracks => defaults
            .tracks
            .iter()
            .map(|value| value.id.as_str())
            .collect(),
        SetupStepId::SourceExamples => defaults
            .source_examples
            .iter()
            .map(|value| value.id.as_str())
            .collect(),
        SetupStepId::RefreshCadence => defaults
            .refresh_cadences
            .iter()
            .map(|value| value.id.as_str())
            .collect(),
        SetupStepId::AiDataDisclosure => HashSet::from(["acknowledged"]),
    };
    let action_values_valid = match input.action {
        SetupAction::Save => {
            !input.selected_values.is_empty()
                && input
                    .selected_values
                    .iter()
                    .all(|value| allowed.contains(value.as_str()))
                && (input.step_id != SetupStepId::RefreshCadence
                    || input.selected_values.len() == 1)
        }
        SetupAction::Skip | SetupAction::Later => input.selected_values.is_empty(),
    };
    if input.contract_version == 1 && key_valid && values_valid && action_values_valid {
        Ok(())
    } else {
        Err(setup_error(ErrorCode::ValidationSetupInput, "input"))
    }
}

fn config_entry(step: SetupStepId, values: &[String]) -> Result<(&'static str, String), AppError> {
    match step {
        SetupStepId::Tracks => Ok((
            "track_ids",
            serde_json::to_string(values)
                .map_err(|_| setup_error(ErrorCode::StorageSetup, "encode-tracks"))?,
        )),
        SetupStepId::SourceExamples => Ok((
            "source_example_ids",
            serde_json::to_string(values)
                .map_err(|_| setup_error(ErrorCode::StorageSetup, "encode-sources"))?,
        )),
        SetupStepId::RefreshCadence => Ok(("refresh_cadence", values[0].clone())),
        SetupStepId::AiDataDisclosure => Ok(("ai_data_disclosure_acknowledged", "true".to_owned())),
    }
}

fn request_fingerprint(input: &SaveSetupStepInputV1) -> String {
    format!(
        "{}|{:?}|{}|{}|{}",
        input.step_id.as_str(),
        input.action,
        input.expected_revision,
        input.expected_configuration_revision,
        input.selected_values.join(",")
    )
}

fn parse_status(value: &str) -> Result<SetupStepStatus, AppError> {
    match value {
        "not_started" => Ok(SetupStepStatus::NotStarted),
        "in_progress" => Ok(SetupStepStatus::InProgress),
        "skipped" => Ok(SetupStepStatus::Skipped),
        "partially_completed" => Ok(SetupStepStatus::PartiallyCompleted),
        "completed" => Ok(SetupStepStatus::Completed),
        _ => Err(setup_error(ErrorCode::StorageSetup, "status")),
    }
}

fn build_progress(
    revision: u64,
    configuration_revision: u64,
    steps: Vec<SetupStepProgressV1>,
    defaults: SetupDefaultsV1,
    saved_config: SetupSavedConfigV1,
) -> SetupProgressV1 {
    let next_step_id = steps
        .iter()
        .find(|step| step.status != SetupStepStatus::Completed)
        .map(|step| step.step_id);
    let overall_status = if steps
        .iter()
        .all(|step| step.status == SetupStepStatus::Completed)
    {
        SetupStepStatus::Completed
    } else if steps
        .iter()
        .all(|step| step.status == SetupStepStatus::NotStarted)
    {
        SetupStepStatus::NotStarted
    } else if steps
        .iter()
        .any(|step| step.status == SetupStepStatus::Completed)
    {
        SetupStepStatus::PartiallyCompleted
    } else if steps
        .iter()
        .any(|step| step.status == SetupStepStatus::Skipped)
    {
        SetupStepStatus::Skipped
    } else {
        SetupStepStatus::InProgress
    };
    SetupProgressV1 {
        contract_version: 1,
        revision,
        configuration_revision,
        overall_status,
        steps,
        next_step_id,
        defaults,
        saved_config,
    }
}

pub(crate) fn setup_error(code: ErrorCode, boundary: &'static str) -> AppError {
    let sequence = SETUP_ERROR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    AppError::from_code(code, format!("setup-{boundary}-{sequence:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        step_id: SetupStepId,
        action: SetupAction,
        values: &[&str],
        revision: u64,
        configuration_revision: u64,
        key: &str,
    ) -> SaveSetupStepInputV1 {
        SaveSetupStepInputV1 {
            contract_version: 1,
            step_id,
            action,
            selected_values: values.iter().map(|value| (*value).to_owned()).collect(),
            expected_revision: revision,
            expected_configuration_revision: configuration_revision,
            idempotency_key: key.to_owned(),
        }
    }

    #[test]
    fn failed_progress_write_rolls_back_configuration_and_revision() {
        let mut store = DemoStore::open_in_memory().expect("store");
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_setup_progress BEFORE INSERT ON setup_progress
                 BEGIN SELECT RAISE(ABORT, 'controlled test failure'); END;",
            )
            .expect("failure trigger");
        let error = store
            .save_setup_step(&input(
                SetupStepId::Tracks,
                SetupAction::Save,
                &["ai_agents"],
                0,
                1,
                "atomic-rollback",
            ))
            .expect_err("transaction must fail");
        assert_eq!(error.code(), "storage.setup");
        let stored: Option<String> = store
            .connection
            .query_row(
                "SELECT value FROM settings_metadata WHERE key='track_ids'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("read rolled back config");
        assert_eq!(stored, None);
        assert_eq!(store.get_setup_progress().expect("progress").revision, 0);
    }

    #[test]
    fn corrupted_persisted_status_fails_closed() {
        let store = DemoStore::open_in_memory().expect("store");
        store
            .connection
            .execute_batch(
                "PRAGMA ignore_check_constraints=ON;
                 INSERT INTO setup_progress(step_id,status,revision,saved_fields_version,updated_at_ms)
                 VALUES ('tracks','unknown',1,NULL,1);
                 PRAGMA ignore_check_constraints=OFF;",
            )
            .expect("inject corrupt status");
        let error = store
            .get_setup_progress()
            .expect_err("unknown stored status must fail");
        assert_eq!(error.code(), "storage.setup");
    }

    #[test]
    fn skipped_progress_is_resumable_and_completed_steps_cannot_be_downgraded() {
        let mut store = DemoStore::open_in_memory().expect("store");
        let skipped = store
            .save_setup_step(&input(
                SetupStepId::Tracks,
                SetupAction::Skip,
                &[],
                0,
                1,
                "skip-resumable",
            ))
            .expect("skip");
        assert_eq!(skipped.overall_status, SetupStepStatus::Skipped);
        assert_eq!(skipped.next_step_id, Some(SetupStepId::Tracks));

        let completed = store
            .save_setup_step(&input(
                SetupStepId::Tracks,
                SetupAction::Save,
                &["ai_agents"],
                1,
                1,
                "complete-after-skip",
            ))
            .expect("complete");
        let error = store
            .save_setup_step(&input(
                SetupStepId::Tracks,
                SetupAction::Later,
                &[],
                2,
                2,
                "cannot-downgrade",
            ))
            .expect_err("completed step cannot be downgraded");
        assert_eq!(error.code(), "validation.setup_input");
        assert_eq!(
            store
                .get_setup_progress()
                .expect("unchanged")
                .saved_config
                .track_ids,
            completed.saved_config.track_ids
        );
    }

    #[test]
    fn timestamps_are_real_and_idempotency_history_is_bounded() {
        let mut store = DemoStore::open_in_memory().expect("store");
        for revision in 0..70 {
            store
                .save_setup_step(&input(
                    SetupStepId::Tracks,
                    SetupAction::Save,
                    &["ai_agents"],
                    revision,
                    revision + 1,
                    &format!("bounded-{revision}"),
                ))
                .expect("repeat saved value");
        }
        let (updated_at_ms, idempotency_count): (i64, i64) = store
            .connection
            .query_row(
                "SELECT
                   (SELECT updated_at_ms FROM setup_progress WHERE step_id='tracks'),
                   (SELECT COUNT(*) FROM setup_idempotency)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("stored metadata");
        assert!(updated_at_ms > 1_700_000_000_000);
        assert_eq!(idempotency_count, 64);
    }
}
