//! Transaction-scoped persistence for the current transparent rule projection.

use rusqlite::{Connection, OptionalExtension, params};

use crate::contracts::dto::configuration_validation::ConfigurationViewV1;
use crate::domain::rules::intelligence_value::{
    INTELLIGENCE_VALUE_RULE_VERSION, IntelligenceValueContext, RuleEvaluationV1,
    evaluate_intelligence_value,
};

pub(crate) fn evaluate_item(
    connection: &Connection,
    item_id: i64,
    configuration: &ConfigurationViewV1,
    evaluated_at_ms: u64,
) -> rusqlite::Result<()> {
    let context = load_context(connection, item_id, configuration, evaluated_at_ms)?;
    let evaluation = evaluate_intelligence_value(&configuration.configuration, &context)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    persist_evaluation(connection, item_id, &evaluation)
}

fn load_context(
    connection: &Connection,
    item_id: i64,
    configuration: &ConfigurationViewV1,
    evaluated_at_ms: u64,
) -> rusqlite::Result<IntelligenceValueContext> {
    connection.query_row(
        "SELECT i.revision,i.source_kind,s.canonical_url,i.publisher,i.original_url,i.title,
                c.source_summary,i.published_at,i.collected_at
         FROM intel_items i
         JOIN sources s ON s.source_id=i.source_id
         LEFT JOIN intel_contents c ON c.item_id=i.id
         JOIN item_provenance p ON p.item_id=i.id
           AND p.source_id=i.source_id AND p.stable_external_id=i.stable_external_id
           AND p.source_kind=i.source_kind AND p.publisher=i.publisher
           AND p.original_title=i.title AND p.original_url=i.original_url
           AND p.published_at IS i.published_at AND p.collected_at=i.collected_at
           AND p.availability_status='available'
         WHERE i.id=?1 AND i.data_origin='real'",
        [item_id],
        |row| {
            Ok(IntelligenceValueContext {
                fact_revision: sql_u64(row.get(0)?)?,
                configuration_revision: configuration.revision,
                configuration_hash: configuration.normalized_config_hash.clone(),
                source_kind: row.get(1)?,
                source_identifier: row.get(2)?,
                publisher: row.get(3)?,
                original_url: row.get(4)?,
                title: row.get(5)?,
                source_summary: row.get(6)?,
                published_at: row.get(7)?,
                collected_at: row.get(8)?,
                evaluated_at_ms,
            })
        },
    )
}

fn persist_evaluation(
    connection: &Connection,
    item_id: i64,
    evaluation: &RuleEvaluationV1,
) -> rusqlite::Result<()> {
    let matched_tracks_json = to_json(&evaluation.matched_track_ids)?;
    let factors_json = to_json(&evaluation.factors)?;
    let filter_reasons_json = to_json(&evaluation.filter_reasons)?;
    let reasons_json = filter_reasons_json.clone();
    let unchanged = connection
        .query_row(
            "SELECT 1 FROM rule_evaluations WHERE item_id=?1 AND rule_version=?2
             AND configuration_revision=?3 AND configuration_hash=?4 AND fact_revision=?5
             AND score=?6 AND importance=?7 AND stream_disposition=?8
             AND matched_tracks_json=?9 AND factor_results_json=?10 AND filter_reasons_json=?11
             AND ai_status=?12",
            params![
                item_id,
                &evaluation.rule_version,
                sql_i64(evaluation.configuration_revision)?,
                &evaluation.configuration_hash,
                sql_i64(evaluation.fact_revision)?,
                evaluation.score,
                evaluation.importance.as_str(),
                evaluation.disposition.as_str(),
                matched_tracks_json,
                factors_json,
                filter_reasons_json,
                evaluation.ai_status.as_str(),
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if unchanged {
        return Ok(());
    }
    connection.execute(
        "INSERT INTO rule_evaluations
         (item_id,why_it_matters,possible_impact,importance,reasons_json,rule_version,
          configuration_revision,configuration_hash,fact_revision,evaluated_at_ms,score,
          stream_disposition,matched_tracks_json,factor_results_json,filter_reasons_json,ai_status)
         VALUES(?1,'rules.value.explainable','rules.value.current_projection',?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
         ON CONFLICT(item_id) DO UPDATE SET
          why_it_matters=excluded.why_it_matters,possible_impact=excluded.possible_impact,
          importance=excluded.importance,reasons_json=excluded.reasons_json,
          rule_version=excluded.rule_version,configuration_revision=excluded.configuration_revision,
          configuration_hash=excluded.configuration_hash,fact_revision=excluded.fact_revision,
          evaluated_at_ms=excluded.evaluated_at_ms,score=excluded.score,
          stream_disposition=excluded.stream_disposition,
          matched_tracks_json=excluded.matched_tracks_json,
          factor_results_json=excluded.factor_results_json,
          filter_reasons_json=excluded.filter_reasons_json,ai_status=excluded.ai_status",
        params![
            item_id,
            evaluation.importance.as_str(),
            reasons_json,
            &evaluation.rule_version,
            sql_i64(evaluation.configuration_revision)?,
            &evaluation.configuration_hash,
            sql_i64(evaluation.fact_revision)?,
            sql_i64(evaluation.evaluated_at_ms)?,
            evaluation.score,
            evaluation.disposition.as_str(),
            matched_tracks_json,
            factors_json,
            filter_reasons_json,
            evaluation.ai_status.as_str(),
        ],
    )?;
    Ok(())
}

pub(crate) fn reevaluate_all(
    connection: &Connection,
    configuration: &ConfigurationViewV1,
    evaluated_at_ms: u64,
) -> rusqlite::Result<()> {
    let ids = connection
        .prepare(
            "SELECT i.id FROM intel_items i JOIN item_provenance p ON p.item_id=i.id
             WHERE i.data_origin='real' AND i.source_kind='rss_atom' AND i.source_id IS NOT NULL
               AND p.availability_status='available'
             ORDER BY i.id",
        )?
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for item_id in ids {
        evaluate_item(connection, item_id, configuration, evaluated_at_ms)?;
    }
    Ok(())
}

pub(crate) fn verify_current(
    connection: &Connection,
    configuration: &ConfigurationViewV1,
) -> rusqlite::Result<bool> {
    let invalid: u32 = connection.query_row(
        "SELECT COUNT(*) FROM intel_items i LEFT JOIN item_provenance p ON p.item_id=i.id
         LEFT JOIN rule_evaluations r ON r.item_id=i.id
         WHERE i.data_origin='real' AND i.source_kind='rss_atom' AND i.source_id IS NOT NULL
           AND (p.item_id IS NULL OR (p.availability_status='available' AND (
           p.source_id IS NOT i.source_id OR p.stable_external_id IS NOT i.stable_external_id
           OR p.source_kind<>i.source_kind OR p.publisher<>i.publisher
           OR p.original_title<>i.title OR p.original_url<>i.original_url
           OR p.published_at IS NOT i.published_at OR p.collected_at<>i.collected_at
           OR r.item_id IS NULL OR r.rule_version<>?1 OR r.configuration_revision<>?2
           OR r.configuration_hash<>?3 OR r.fact_revision<>i.revision
           OR r.evaluated_at_ms IS NULL OR r.evaluated_at_ms<1
           OR r.score NOT BETWEEN 0 AND 100
           OR r.stream_disposition NOT IN ('high_value','ordinary_candidate')
           OR r.ai_status<>'unavailable'
           OR json_valid(r.matched_tracks_json)=0 OR json_valid(r.factor_results_json)=0
           OR json_valid(r.filter_reasons_json)=0)))",
        params![
            INTELLIGENCE_VALUE_RULE_VERSION,
            sql_i64(configuration.revision)?,
            configuration.normalized_config_hash,
        ],
        |row| row.get(0),
    )?;
    if invalid != 0 {
        return Ok(false);
    }
    let ids = connection
        .prepare(
            "SELECT i.id FROM intel_items i JOIN item_provenance p ON p.item_id=i.id
             WHERE i.data_origin='real' AND i.source_kind='rss_atom' AND i.source_id IS NOT NULL
               AND p.availability_status='available'
             ORDER BY i.id",
        )?
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for item_id in ids {
        let stored = stored_evaluation(connection, item_id)?;
        let context = load_context(connection, item_id, configuration, stored.evaluated_at_ms)?;
        let expected = evaluate_intelligence_value(&configuration.configuration, &context)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        if !stored.matches(&expected) {
            return Ok(false);
        }
    }
    Ok(true)
}

struct StoredEvaluation {
    evaluated_at_ms: u64,
    rule_version: String,
    configuration_revision: u64,
    configuration_hash: String,
    fact_revision: u64,
    score: u8,
    importance: String,
    disposition: String,
    matched_tracks_json: String,
    factors_json: String,
    filters_json: String,
    ai_status: String,
}

impl StoredEvaluation {
    fn matches(&self, expected: &RuleEvaluationV1) -> bool {
        self.rule_version == expected.rule_version
            && self.configuration_revision == expected.configuration_revision
            && self.configuration_hash == expected.configuration_hash
            && self.fact_revision == expected.fact_revision
            && self.score == expected.score
            && self.importance == expected.importance.as_str()
            && self.disposition == expected.disposition.as_str()
            && serde_json::from_str::<serde_json::Value>(&self.matched_tracks_json).ok()
                == serde_json::to_value(&expected.matched_track_ids).ok()
            && serde_json::from_str::<serde_json::Value>(&self.factors_json).ok()
                == serde_json::to_value(&expected.factors).ok()
            && serde_json::from_str::<serde_json::Value>(&self.filters_json).ok()
                == serde_json::to_value(&expected.filter_reasons).ok()
            && self.ai_status == expected.ai_status.as_str()
    }
}

fn stored_evaluation(connection: &Connection, item_id: i64) -> rusqlite::Result<StoredEvaluation> {
    connection.query_row(
        "SELECT evaluated_at_ms,rule_version,configuration_revision,configuration_hash,
                fact_revision,score,importance,stream_disposition,matched_tracks_json,
                factor_results_json,filter_reasons_json,ai_status
         FROM rule_evaluations WHERE item_id=?1",
        [item_id],
        |row| {
            Ok(StoredEvaluation {
                evaluated_at_ms: sql_u64(row.get(0)?)?,
                rule_version: row.get(1)?,
                configuration_revision: sql_u64(row.get(2)?)?,
                configuration_hash: row.get(3)?,
                fact_revision: sql_u64(row.get(4)?)?,
                score: row.get(5)?,
                importance: row.get(6)?,
                disposition: row.get(7)?,
                matched_tracks_json: row.get(8)?,
                factors_json: row.get(9)?,
                filters_json: row.get(10)?,
                ai_status: row.get(11)?,
            })
        },
    )
}

fn to_json<T: serde::Serialize>(value: &T) -> rusqlite::Result<String> {
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn sql_i64(value: u64) -> rusqlite::Result<i64> {
    i64::try_from(value).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn sql_u64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
