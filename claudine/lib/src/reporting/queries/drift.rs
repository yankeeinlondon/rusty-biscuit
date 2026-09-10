use std::collections::HashMap;

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};
use serde_json::Value;

use crate::error::Result;
use crate::reporting::types::{AliasResolution, DateRange, DriftReport, ReportingFilters};

use super::common::{parse_provider, parse_timestamp, validate_range};
use crate::reporting::types::DriftSignalSummary;

/// The signal kinds this report covers — the model-catalog family only
/// (`model_catalog_drift`, `model_resolved`, `model_fallback`), NOT the
/// whole `session_signals` table. Other kinds (rate limits, guards, …)
/// belong to their own future subreports.
const DRIFT_KINDS: &str = "('model_catalog_drift', 'model_resolved', 'model_fallback')";

/// Recent model-catalog signal activity plus `family_latest` alias stamps.
///
/// Two sections:
/// - `signals`: per-provider aggregates of the [`DRIFT_KINDS`] rows in
///   `session_signals`, with the most recent `model_catalog_drift`
///   payload's `unexpected`/`missing`/`observed_via` surfaced.
/// - `aliases`: the `top_n` most recent sessions whose SessionEnd row
///   carried a `family_latest` stamp (lifted onto `sessions`).
///
/// Filters: date range applies to both sections; of
/// [`ReportingFilters`], only `provider` applies — `session_signals`
/// carries no repo/package columns.
pub(crate) fn drift(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
    top_n: usize,
) -> Result<DriftReport> {
    validate_range(range)?;

    let signals = load_signal_summaries(conn, range, filters)?;
    let aliases = load_alias_resolutions(conn, range, filters, top_n)?;

    Ok(DriftReport {
        range,
        signals,
        aliases,
    })
}

fn range_params(range: DateRange) -> [SqlValue; 2] {
    [
        SqlValue::Text(range.from.format("%Y-%m-%d").to_string()),
        SqlValue::Text(range.to.format("%Y-%m-%d").to_string()),
    ]
}

fn load_signal_summaries(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<Vec<DriftSignalSummary>> {
    let mut sql = format!(
        r#"
        SELECT provider, kind, COUNT(DISTINCT session_key), SUM(occurrences), MAX(timestamp)
        FROM session_signals
        WHERE source_date >= ? AND source_date <= ? AND kind IN {DRIFT_KINDS}
        "#,
    );
    let mut params: Vec<SqlValue> = range_params(range).to_vec();
    if let Some(provider) = filters.provider {
        sql.push_str(" AND provider = ?");
        params.push(SqlValue::Text(provider.as_slug().to_string()));
    }
    sql.push_str(" GROUP BY provider, kind ORDER BY provider ASC, kind ASC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let latest_drift = load_latest_drift_payloads(conn, range, filters)?;

    let mut summaries = Vec::new();
    for row in rows {
        let (provider, kind, session_count, occurrences, last_seen) = row?;
        let detail = (kind == "model_catalog_drift")
            .then(|| latest_drift.get(provider.as_str()))
            .flatten();
        summaries.push(DriftSignalSummary {
            provider: parse_provider(&provider)?,
            kind,
            session_count: session_count.max(0) as u64,
            occurrences: occurrences.max(0) as u64,
            last_seen: parse_timestamp(&last_seen)?,
            unexpected: detail.map(|d| d.unexpected.clone()).unwrap_or_default(),
            missing: detail.map(|d| d.missing.clone()).unwrap_or_default(),
            observed_via: detail.and_then(|d| d.observed_via.clone()),
        });
    }
    Ok(summaries)
}

/// `unexpected`/`missing`/`observed_via` from one drift payload.
struct DriftDetail {
    unexpected: Vec<String>,
    missing: Vec<String>,
    observed_via: Option<String>,
}

/// The most recent `model_catalog_drift` payload per provider in range.
fn load_latest_drift_payloads(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<HashMap<String, DriftDetail>> {
    let mut sql = String::from(
        r#"
        SELECT provider, payload_json
        FROM session_signals
        WHERE source_date >= ? AND source_date <= ? AND kind = 'model_catalog_drift'
        "#,
    );
    let mut params: Vec<SqlValue> = range_params(range).to_vec();
    if let Some(provider) = filters.provider {
        sql.push_str(" AND provider = ?");
        params.push(SqlValue::Text(provider.as_slug().to_string()));
    }
    sql.push_str(" ORDER BY timestamp DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut latest: HashMap<String, DriftDetail> = HashMap::new();
    for row in rows {
        let (provider, payload_json) = row?;
        // Rows arrive newest-first; keep only the first per provider.
        if latest.contains_key(&provider) {
            continue;
        }
        let Ok(payload) = serde_json::from_str::<Value>(&payload_json) else {
            continue;
        };
        let event = payload.get("event");
        let string_list = |key: &str| -> Vec<String> {
            event
                .and_then(|e| e.get(key))
                .and_then(Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default()
        };
        latest.insert(
            provider,
            DriftDetail {
                unexpected: string_list("unexpected"),
                missing: string_list("missing"),
                observed_via: event
                    .and_then(|e| e.get("observed_via"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            },
        );
    }
    Ok(latest)
}

fn load_alias_resolutions(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
    top_n: usize,
) -> Result<Vec<AliasResolution>> {
    let mut sql = String::from(
        r#"
        SELECT session_key, session_id, provider, ended_at, family_latest
        FROM sessions
        WHERE family_latest IS NOT NULL
          AND date(ended_at) >= ? AND date(ended_at) <= ?
        "#,
    );
    let mut params: Vec<SqlValue> = range_params(range).to_vec();
    if let Some(provider) = filters.provider {
        sql.push_str(" AND provider = ?");
        params.push(SqlValue::Text(provider.as_slug().to_string()));
    }
    sql.push_str(" ORDER BY ended_at DESC LIMIT ?");
    params.push(SqlValue::Integer(i64::try_from(top_n).unwrap_or(i64::MAX)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut aliases = Vec::new();
    for row in rows {
        let (session_key, session_id, provider, ended_at, family_latest) = row?;
        // A malformed stamp is skipped, not fatal: the raw JSON is still
        // in the sessions row for inspection.
        let Ok(stamp) = serde_json::from_str::<Value>(&family_latest) else {
            continue;
        };
        let text = |key: &str| -> Option<String> {
            stamp
                .get(key)
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        };
        let (Some(alias), Some(identity_key)) = (text("alias"), text("identity_key")) else {
            continue;
        };
        aliases.push(AliasResolution {
            session_key,
            session_id,
            provider: parse_provider(&provider)?,
            ended_at: parse_timestamp(&ended_at)?,
            alias,
            identity_key,
            family_key: text("family_key"),
            stale: stamp
                .get("stale")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            age_days: stamp.get("age_days").and_then(Value::as_u64),
        });
    }
    Ok(aliases)
}
