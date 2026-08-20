//! Device-local versioned attention configuration use cases.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use super::demo::DemoStore;
use crate::contracts::dto::configuration_validation::{
    AttentionConfigurationV1, AttentionTrackV1, ConfigurationCandidateContext,
    ConfigurationCandidateV1, ConfigurationViewV1, NarrowingRiskCodeV1, NotificationFrequencyV1,
    QuietHoursV1, SaveConfigurationInputV1, SourcePreferenceV1, ValidateConfigurationInputV1,
};
use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::rules::configuration_validation::{
    VALIDATOR_VERSION, assess_configuration, configuration_hash, configuration_identity, normalize,
    validate_configuration,
};

pub(crate) fn create_initial_configuration(connection: &Connection) -> Result<(), AppError> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM configuration_current WHERE singleton_id=1)",
            [],
            |row| row.get(0),
        )
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "initial-read"))?;
    if exists {
        return Ok(());
    }
    let configuration = default_configuration(connection)?;
    let normalized = normalize(&configuration);
    let validation = assess_configuration(&normalized, &ConfigurationCandidateContext::default());
    if !validation.blocking_errors.is_empty() {
        return Err(configuration_error(
            ErrorCode::MigrationSetup,
            "initial-invalid",
        ));
    }
    let hash = configuration_hash(&normalized);
    let json = serde_json::to_string(&normalized)
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "initial-json"))?;
    let now = now_ms()?;
    let now_sql = i64::try_from(now)
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "initial-time"))?;
    connection.execute(
            "INSERT INTO configuration_versions(version,validator_version,normalized_config_hash,configuration_json,created_at_ms) VALUES(1,?1,?2,?3,?4)",
            params![VALIDATOR_VERSION, hash, json, now_sql],
        ).map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "initial-version"))?;
    connection
        .execute(
            "INSERT INTO configuration_current(singleton_id,version) VALUES(1,1)",
            [],
        )
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "initial-current"))?;
    Ok(())
}

impl DemoStore {
    /// Returns the current immutable configuration view.
    ///
    /// # Errors
    /// Returns a stable storage error for missing, corrupt, or unreadable state.
    pub fn get_configuration(&self) -> Result<ConfigurationViewV1, AppError> {
        read_configuration(&self.connection)
    }
}

pub(crate) fn read_configuration(connection: &Connection) -> Result<ConfigurationViewV1, AppError> {
    let view = connection.query_row(
        "SELECT v.version,v.validator_version,v.normalized_config_hash,v.configuration_json,v.created_at_ms
             FROM configuration_current c JOIN configuration_versions v ON v.version=c.version
             WHERE c.singleton_id=1",
            [],
            |row| {
                let json: String = row.get(3)?;
                let configuration = serde_json::from_str(&json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(json.len(), rusqlite::types::Type::Text, Box::new(error))
                })?;
                Ok(ConfigurationViewV1 {
                    contract_version: 1,
                    revision: sql_u64(row.get(0)?, 0)?,
                    validator_version: row.get(1)?,
                    normalized_config_hash: row.get(2)?,
                    configuration,
                    updated_at_ms: sql_u64(row.get(4)?, 4)?,
                })
            },
        ).map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "read"))?;
    let normalized = normalize(&view.configuration);
    let validation = assess_configuration(&normalized, &ConfigurationCandidateContext::default());
    if view.contract_version != 1
        || view.validator_version != VALIDATOR_VERSION
        || view.configuration != normalized
        || view.normalized_config_hash != configuration_hash(&normalized)
        || !validation.blocking_errors.is_empty()
    {
        return Err(configuration_error(
            ErrorCode::StorageConfiguration,
            "read-contract",
        ));
    }
    Ok(view)
}

pub(crate) fn append_configuration_version(
    transaction: &Transaction<'_>,
    configuration: &AttentionConfigurationV1,
    expected_revision: u64,
) -> Result<ConfigurationViewV1, AppError> {
    let current = read_configuration(transaction)?;
    if current.revision != expected_revision {
        return Err(configuration_error(
            ErrorCode::ConflictConfigurationRevision,
            "setup-stale-revision",
        ));
    }
    let normalized = normalize(configuration);
    let validation = assess_configuration(&normalized, &ConfigurationCandidateContext::default());
    if !validation.blocking_errors.is_empty() {
        return Err(configuration_error(
            ErrorCode::ValidationConfiguration,
            "setup-blocking",
        ));
    }
    let next_revision = current.revision.checked_add(1).ok_or_else(|| {
        configuration_error(ErrorCode::ConflictConfigurationRevision, "setup-overflow")
    })?;
    let next_revision_sql = i64::try_from(next_revision).map_err(|_| {
        configuration_error(
            ErrorCode::ConflictConfigurationRevision,
            "setup-revision-range",
        )
    })?;
    let current_revision_sql = i64::try_from(current.revision).map_err(|_| {
        configuration_error(
            ErrorCode::ConflictConfigurationRevision,
            "setup-current-range",
        )
    })?;
    let now = now_ms()?;
    let now_sql = i64::try_from(now)
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "setup-time"))?;
    let configuration_json = serde_json::to_string(&normalized)
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "setup-json"))?;
    transaction.execute(
        "INSERT INTO configuration_versions(version,validator_version,normalized_config_hash,configuration_json,created_at_ms) VALUES(?1,?2,?3,?4,?5)",
        params![next_revision_sql, VALIDATOR_VERSION, validation.normalized_config_hash, configuration_json, now_sql],
    ).map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "setup-version"))?;
    let changed = transaction
        .execute(
            "UPDATE configuration_current SET version=?1 WHERE singleton_id=1 AND version=?2",
            params![next_revision_sql, current_revision_sql],
        )
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "setup-current"))?;
    if changed != 1 {
        return Err(configuration_error(
            ErrorCode::ConflictConfigurationRevision,
            "setup-race",
        ));
    }
    let response = ConfigurationViewV1 {
        contract_version: 1,
        revision: next_revision,
        validator_version: VALIDATOR_VERSION.to_owned(),
        normalized_config_hash: validation.normalized_config_hash,
        configuration: normalized,
        updated_at_ms: now,
    };
    crate::infrastructure::database::rule_evaluation_repository::reevaluate_all(
        transaction,
        &response,
        now,
    )
    .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "setup-rule-refresh"))?;
    Ok(response)
}

impl DemoStore {
    /// Validates without writing configuration state.
    ///
    /// # Errors
    /// Returns a stable validation or storage error if the request/context is unavailable.
    pub fn validate_attention_configuration(
        &mut self,
        input: &ValidateConfigurationInputV1,
    ) -> Result<
        crate::contracts::dto::configuration_validation::ConfigurationValidationResultV1,
        AppError,
    > {
        if input.contract_version != 1 {
            return Err(configuration_error(
                ErrorCode::ValidationConfiguration,
                "contract",
            ));
        }
        self.configuration_receipts.set_time_ms(now_ms()?);
        let candidates = candidate_context(&self.connection)?;
        validate_configuration(
            &input.configuration,
            &candidates,
            &mut self.configuration_receipts,
        )
        .map_err(|_| configuration_error(ErrorCode::InternalUnexpected, "receipt-entropy"))
    }

    /// Atomically appends and selects a configuration version.
    ///
    /// # Errors
    /// Returns stable validation, conflict, or storage errors without partial writes.
    #[allow(clippy::too_many_lines)] // The ordered idempotency/receipt/transaction boundary is kept together for auditability.
    pub fn save_attention_configuration(
        &mut self,
        input: &SaveConfigurationInputV1,
    ) -> Result<ConfigurationViewV1, AppError> {
        if input.contract_version != 1
            || input.idempotency_key.is_empty()
            || input.idempotency_key.len() > 128
        {
            return Err(configuration_error(
                ErrorCode::ValidationConfiguration,
                "input",
            ));
        }
        let fingerprint = request_fingerprint(input)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "transaction"))?;
        if let Some((saved_fingerprint, response_json)) = transaction.query_row(
            "SELECT request_fingerprint,response_json FROM configuration_idempotency WHERE idempotency_key=?1",
            [&input.idempotency_key],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ).optional().map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "idempotency-read"))? {
            if saved_fingerprint != fingerprint {
                return Err(configuration_error(ErrorCode::ValidationConfiguration, "idempotency-reuse"));
            }
            return serde_json::from_str(&response_json)
                .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "idempotency-json"));
        }
        let current = read_configuration(&transaction)?;
        if current.revision != input.expected_revision {
            return Err(configuration_error(
                ErrorCode::ConflictConfigurationRevision,
                "stale-revision",
            ));
        }
        let candidates = candidate_context(&transaction)?;
        let validation = assess_configuration(&input.configuration, &candidates);
        if !validation.blocking_errors.is_empty() {
            return Err(configuration_error(
                ErrorCode::ValidationConfiguration,
                "blocking",
            ));
        }
        let risk_codes: Vec<NarrowingRiskCodeV1> = validation
            .narrowing_risks
            .iter()
            .map(|risk| risk.code)
            .collect();
        if input.expected_normalized_config_hash != validation.normalized_config_hash {
            return Err(configuration_error(
                ErrorCode::ValidationConfiguration,
                "normalized-hash-mismatch",
            ));
        }
        let normalized = normalize(&input.configuration);
        let canonical_identity = configuration_identity(&normalized);
        if !risk_codes.is_empty() {
            let Some(receipt) = input.validation_receipt.as_ref() else {
                return Err(configuration_error(
                    ErrorCode::ValidationStaleReceipt,
                    "missing",
                ));
            };
            self.configuration_receipts.set_time_ms(now_ms()?);
            if !self.configuration_receipts.is_valid(
                receipt,
                &validation.normalized_config_hash,
                &canonical_identity,
                &risk_codes,
            ) {
                return Err(configuration_error(
                    ErrorCode::ValidationStaleReceipt,
                    "stale",
                ));
            }
        } else if input.validation_receipt.is_some() {
            return Err(configuration_error(
                ErrorCode::ValidationStaleReceipt,
                "unexpected",
            ));
        }
        let next_revision = current.revision.checked_add(1).ok_or_else(|| {
            configuration_error(ErrorCode::ConflictConfigurationRevision, "overflow")
        })?;
        let now = now_ms()?;
        let next_revision_sql = i64::try_from(next_revision).map_err(|_| {
            configuration_error(ErrorCode::ConflictConfigurationRevision, "revision-range")
        })?;
        let current_revision_sql = i64::try_from(current.revision).map_err(|_| {
            configuration_error(ErrorCode::ConflictConfigurationRevision, "current-range")
        })?;
        let now_sql = i64::try_from(now)
            .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "time-range"))?;
        let configuration_json = serde_json::to_string(&normalized)
            .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "serialize"))?;
        let response = ConfigurationViewV1 {
            contract_version: 1,
            revision: next_revision,
            validator_version: VALIDATOR_VERSION.to_owned(),
            normalized_config_hash: validation.normalized_config_hash.clone(),
            configuration: normalized,
            updated_at_ms: now,
        };
        let response_json = serde_json::to_string(&response)
            .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "response-json"))?;
        transaction.execute(
            "INSERT INTO configuration_versions(version,validator_version,normalized_config_hash,configuration_json,created_at_ms) VALUES(?1,?2,?3,?4,?5)",
            params![next_revision_sql, VALIDATOR_VERSION, validation.normalized_config_hash, configuration_json, now_sql],
        ).map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "version"))?;
        let changed = transaction
            .execute(
                "UPDATE configuration_current SET version=?1 WHERE singleton_id=1 AND version=?2",
                params![next_revision_sql, current_revision_sql],
            )
            .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "current"))?;
        if changed != 1 {
            return Err(configuration_error(
                ErrorCode::ConflictConfigurationRevision,
                "race",
            ));
        }
        crate::infrastructure::database::rule_evaluation_repository::reevaluate_all(
            &transaction,
            &response,
            now,
        )
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "rule-refresh"))?;
        transaction.execute(
            "INSERT INTO configuration_idempotency(idempotency_key,request_fingerprint,response_version,response_json) VALUES(?1,?2,?3,?4)",
            params![input.idempotency_key, fingerprint, next_revision_sql, response_json],
        ).map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "idempotency-write"))?;
        transaction.execute(
            "DELETE FROM configuration_idempotency WHERE idempotency_key NOT IN (SELECT idempotency_key FROM configuration_idempotency ORDER BY response_version DESC, rowid DESC LIMIT 64)",
            [],
        ).map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "idempotency-trim"))?;
        transaction
            .commit()
            .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "commit"))?;
        if let Some(receipt) = input.validation_receipt.as_ref() {
            let _ = self.configuration_receipts.consume(
                receipt,
                &validation.normalized_config_hash,
                &canonical_identity,
                &risk_codes,
            );
        }
        Ok(response)
    }
}

fn default_configuration(
    connection: &rusqlite::Connection,
) -> Result<AttentionConfigurationV1, AppError> {
    let track_ids = setting(connection, "track_ids")?
        .unwrap_or_else(|| "[\"ai_agents\",\"foundation_models\",\"local_models\"]".to_owned());
    let ids: Vec<String> = serde_json::from_str(&track_ids)
        .map_err(|_| configuration_error(ErrorCode::MigrationSetup, "track-ids"))?;
    let cadence = setting(connection, "refresh_cadence")?.unwrap_or_else(|| "manual".to_owned());
    let (refresh_enabled, refresh_interval_minutes) = match cadence.as_str() {
        "manual" => (false, 1_440),
        "hourly" => (true, 60),
        "daily" => (true, 1_440),
        "weekly" => (true, 10_080),
        _ => {
            return Err(configuration_error(
                ErrorCode::MigrationSetup,
                "refresh-cadence",
            ));
        }
    };
    Ok(AttentionConfigurationV1 {
        contract_version: 1,
        tracks: ids
            .into_iter()
            .enumerate()
            .map(|(index, id)| AttentionTrackV1 {
                name: match id.as_str() {
                    "ai_agents" => "AI 智能体".to_owned(),
                    "foundation_models" => "基础模型".to_owned(),
                    "local_models" => "本地模型".to_owned(),
                    _ => format!("关注赛道 {}", index + 1),
                },
                id,
                enabled: true,
            })
            .collect(),
        include_expression: String::new(),
        exclude_expression: String::new(),
        source_preferences: vec![SourcePreferenceV1 {
            source_kind: "rss".into(),
            identifier: "https://example.invalid/feed.xml".into(),
            enabled: true,
            trust: 80,
        }],
        refresh_enabled,
        refresh_interval_minutes,
        minimum_trust: 0,
        maximum_trust: 100,
        alert_threshold: 80,
        quiet_hours: QuietHoursV1 {
            enabled: false,
            start: "22:00".into(),
            end: "07:00".into(),
        },
        notification_frequency: NotificationFrequencyV1 {
            enabled: false,
            max_per_24h: None,
        },
        active_from: None,
        active_until: None,
    })
}

fn setting(connection: &rusqlite::Connection, key: &str) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT value FROM settings_metadata WHERE key=?1",
            [key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "setting-read"))
}

fn candidate_context(
    connection: &rusqlite::Connection,
) -> Result<ConfigurationCandidateContext, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT p.source_kind, i.title || ' ' || COALESCE(c.source_summary,'')
         FROM intel_items i
         JOIN item_provenance p ON p.item_id=i.id
         LEFT JOIN intel_contents c ON c.item_id=i.id
         WHERE i.data_origin='real' ORDER BY i.id",
        )
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "candidate-prepare"))?;
    let real_candidates = statement
        .query_map([], |row| {
            Ok(ConfigurationCandidateV1 {
                source_kind: row.get(0)?,
                searchable_text: row.get(1)?,
            })
        })
        .and_then(|rows| rows.collect())
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "candidate-read"))?;
    Ok(ConfigurationCandidateContext { real_candidates })
}

fn request_fingerprint(input: &SaveConfigurationInputV1) -> Result<String, AppError> {
    let bytes = serde_json::to_vec(input)
        .map_err(|_| configuration_error(ErrorCode::ValidationConfiguration, "fingerprint"))?;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(output)
}

pub(super) fn now_ms() -> Result<u64, AppError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "clock"))?
        .as_millis();
    u64::try_from(millis)
        .map_err(|_| configuration_error(ErrorCode::StorageConfiguration, "clock-range"))
}

fn configuration_error(code: ErrorCode, boundary: &'static str) -> AppError {
    AppError::from_code(code, format!("configuration-{boundary}"))
}

fn sql_u64(value: i64, column: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_transaction_keeps_risk_receipt_available_for_the_same_intent() {
        let mut store = DemoStore::open_in_memory().expect("store");
        let initial = store.get_configuration().expect("initial");
        let mut configuration = initial.configuration;
        configuration
            .source_preferences
            .iter_mut()
            .for_each(|source| source.enabled = false);
        let validation = store
            .validate_attention_configuration(&ValidateConfigurationInputV1 {
                contract_version: 1,
                configuration: configuration.clone(),
            })
            .expect("validation");
        let input = SaveConfigurationInputV1 {
            contract_version: 1,
            expected_normalized_config_hash: validation.normalized_config_hash,
            configuration,
            expected_revision: initial.revision,
            idempotency_key: "receipt-retry-after-storage-failure".into(),
            validation_receipt: validation.validation_receipt,
        };
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_configuration_version BEFORE INSERT ON configuration_versions
                 BEGIN SELECT RAISE(ABORT, 'controlled failure'); END;",
            )
            .expect("trigger");

        let error = store
            .save_attention_configuration(&input)
            .expect_err("controlled storage failure");
        assert_eq!(error.code(), "storage.configuration");
        assert_eq!(store.get_configuration().expect("unchanged").revision, 1);
        store
            .connection
            .execute("DROP TRIGGER reject_configuration_version", [])
            .expect("drop trigger");
        assert_eq!(
            store
                .save_attention_configuration(&input)
                .expect("same receipt can retry")
                .revision,
            2
        );
    }

    #[test]
    fn immutable_versions_and_idempotency_fingerprint_fail_closed() {
        let mut store = DemoStore::open_in_memory().expect("store");
        let initial = store.get_configuration().expect("initial");
        let input = SaveConfigurationInputV1 {
            contract_version: 1,
            expected_normalized_config_hash: initial.normalized_config_hash.clone(),
            configuration: initial.configuration.clone(),
            expected_revision: initial.revision,
            idempotency_key: "immutable-version-one".into(),
            validation_receipt: None,
        };
        let saved = store
            .save_attention_configuration(&input)
            .expect("save version");
        let mut reused = input;
        reused.configuration.tracks[0].name = "different intent".into();
        let error = store
            .save_attention_configuration(&reused)
            .expect_err("same key different payload");
        assert_eq!(error.code(), "validation.configuration");
        let count: u32 = store
            .connection
            .query_row("SELECT COUNT(*) FROM configuration_versions", [], |row| {
                row.get(0)
            })
            .expect("version count");
        assert_eq!(count, 2);
        assert_eq!(store.get_configuration().expect("current"), saved);
    }

    #[test]
    fn reads_fail_closed_when_current_json_is_mutated_after_startup() {
        let store = DemoStore::open_in_memory().expect("store");
        let mut value =
            serde_json::to_value(&store.get_configuration().expect("current").configuration)
                .expect("configuration JSON");
        value["tracks"] = serde_json::json!([]);
        store
            .connection
            .execute(
                "UPDATE configuration_versions SET configuration_json=?1 WHERE version=1",
                [serde_json::to_string(&value).expect("mutated JSON")],
            )
            .expect("mutate current JSON");

        let error = store
            .get_configuration()
            .expect_err("corrupt current configuration must fail");
        assert_eq!(error.code(), "storage.configuration");
    }

    #[test]
    fn revision_beyond_sqlite_range_fails_as_a_conflict_without_writes() {
        let mut store = DemoStore::open_in_memory().expect("store");
        let current = store.get_configuration().expect("current");
        let json = serde_json::to_string(&current.configuration).expect("JSON");
        store
            .connection
            .execute(
                "INSERT INTO configuration_versions(version,validator_version,normalized_config_hash,configuration_json,created_at_ms) VALUES(?1,?2,?3,?4,?5)",
                params![i64::MAX, VALIDATOR_VERSION, current.normalized_config_hash, json, 1_i64],
            )
            .expect("max revision");
        store
            .connection
            .execute(
                "UPDATE configuration_current SET version=?1 WHERE singleton_id=1",
                [i64::MAX],
            )
            .expect("select max revision");
        let selected = store.get_configuration().expect("selected max");
        let input = SaveConfigurationInputV1 {
            contract_version: 1,
            configuration: selected.configuration,
            expected_revision: selected.revision,
            expected_normalized_config_hash: selected.normalized_config_hash,
            idempotency_key: "revision-overflow".into(),
            validation_receipt: None,
        };

        let error = store
            .save_attention_configuration(&input)
            .expect_err("out-of-range next revision");
        assert_eq!(error.code(), "conflict.configuration_revision");
        let count: u32 = store
            .connection
            .query_row("SELECT COUNT(*) FROM configuration_versions", [], |row| {
                row.get(0)
            })
            .expect("version count");
        assert_eq!(count, 2);
    }
}
