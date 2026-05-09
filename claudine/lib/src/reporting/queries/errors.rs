use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};

use crate::error::Result;
use crate::reporting::types::{DateRange, ErrorRecord, ErrorsReport, ReportingFilters};

use super::common::{
    WhereBuilder, parse_event, parse_json_value, parse_provider, parse_timestamp, validate_range,
};

pub(crate) fn errors(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
    top_n: usize,
) -> Result<ErrorsReport> {
    validate_range(range)?;

    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters)
        .with_alias("e");
    let sql = builder.finish(
        r#"
        SELECT e.timestamp, e.provider, e.event, e.session_key, e.session_id, e.repo_name,
               e.tool_name,
               COALESCE(e.model, s.model) AS model,
               e.error,
               COALESCE(
                   e.prompt_text,
                   (SELECT p.prompt_text FROM events p
                    WHERE p.session_key = e.session_key AND p.prompt_text IS NOT NULL
                      AND p.timestamp <= e.timestamp
                    ORDER BY p.timestamp DESC LIMIT 1)
               ) AS prompt_text,
               e.tool_input_json, e.notification_message, e.extra_json
        FROM events e
        LEFT JOIN sessions s ON e.session_key = s.session_key
        "#,
    ) + " AND e.event IN ('tool_error', 'turn_error') ORDER BY e.timestamp DESC LIMIT ?";

    let mut params = builder.params;
    params.push(SqlValue::Integer(i64::try_from(top_n).unwrap_or(i64::MAX)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
        ))
    })?;

    let mut errors = Vec::new();
    for row in rows {
        let (
            timestamp,
            provider,
            event,
            session_key,
            session_id,
            repo_name,
            tool_name,
            model,
            error,
            prompt,
            tool_input_json,
            notification_message,
            extra_json,
        ) = row?;
        errors.push(ErrorRecord {
            timestamp: parse_timestamp(&timestamp)?,
            provider: parse_provider(&provider)?,
            event: parse_event(&event)?,
            session_key,
            session_id,
            repo_name,
            tool_name,
            model,
            error,
            prompt,
            tool_input: parse_json_value(tool_input_json),
            notification_message,
            extra: parse_json_value(extra_json),
        });
    }

    Ok(ErrorsReport { range, errors })
}
