use std::collections::BTreeMap;

use chrono::NaiveDate;
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};

use super::common::{
    load_provider_split, load_tool_stats, parse_provider, repo_label, validate_range,
    WhereBuilder,
};
use crate::reporting::types::{DateRange, ProviderSplit, ReportingFilters, TrendPoint, TrendsReport};
use crate::error::Result;
use super::common::SessionBreakdown;

pub(crate) fn trends(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
    top_n: usize,
) -> Result<TrendsReport> {
    validate_range(range)?;

    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"
        SELECT
            source_date,
            COUNT(*) AS total_events,
            COUNT(DISTINCT session_key) AS session_count,
            COALESCE(SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END), 0) AS turn_count,
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) AS tool_call_count,
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) AS tool_error_count,
            COALESCE(SUM(CASE WHEN event = 'turn_error' THEN 1 ELSE 0 END), 0) AS turn_error_count,
            COALESCE(SUM(total_tokens), 0) AS sum_total_tokens,
            COALESCE(SUM(cost_usd), 0) AS sum_cost_usd
        FROM events
        "#,
    ) + " GROUP BY source_date ORDER BY source_date ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, f64>(8)?,
        ))
    })?;

    let provider_points = load_provider_points(conn, range, filters)?;
    let session_breakdown = load_session_breakdown(conn, range, filters)?;
    let daily_repos = load_daily_repos(conn, range, filters)?;
    let mut points = Vec::new();
    for row in rows {
        let (
            date,
            events,
            _sessions,
            turns,
            tools,
            tool_errors,
            turn_errors,
            total_tokens,
            cost_usd,
        ) = row?;
        let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
        let breakdown = session_breakdown.get(&date).copied().unwrap_or_default();
        let yolo_percent = if breakdown.total_sessions == 0 {
            0.0
        } else {
            (breakdown.yolo_sessions as f64 / breakdown.total_sessions as f64) * 100.0
        };
        points.push(TrendPoint {
            date,
            events: events.max(0) as u64,
            wrapped: breakdown.wrapped,
            unwrapped: breakdown.unwrapped,
            non_interactive: breakdown.non_interactive,
            yolo_percent,
            turns: turns.max(0) as u64,
            tool_calls: tools.max(0) as u64,
            tool_errors: tool_errors.max(0) as u64,
            turn_errors: turn_errors.max(0) as u64,
            total_tokens: total_tokens.max(0) as u64,
            cost_usd: cost_usd.max(0.0),
            repos: daily_repos.get(&date).cloned().unwrap_or_default(),
            providers: provider_points.get(&date).cloned().unwrap_or_default(),
        });
    }

    Ok(TrendsReport {
        range,
        provider_split: load_provider_split(conn, range, filters)?,
        top_tools: load_tool_stats(conn, range, filters, top_n)?,
        points,
    })
}

fn load_provider_points(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<BTreeMap<NaiveDate, Vec<ProviderSplit>>> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"SELECT source_date, provider, COUNT(*),
           COALESCE(SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END), 0),
           COALESCE(SUM(CASE WHEN event IN ('tool_error', 'turn_error') THEN 1 ELSE 0 END), 0)
           FROM events"#,
    ) + " GROUP BY source_date, provider ORDER BY source_date ASC, COUNT(*) DESC, provider ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    let mut map: BTreeMap<NaiveDate, Vec<ProviderSplit>> = BTreeMap::new();
    for row in rows {
        let (date, provider, count, turns, error_count) = row?;
        map.entry(NaiveDate::parse_from_str(&date, "%Y-%m-%d")?)
            .or_default()
            .push(ProviderSplit {
                provider: parse_provider(&provider)?,
                count: count.max(0) as u64,
                turns: turns.max(0) as u64,
                error_count: error_count.max(0) as u64,
            });
    }
    Ok(map)
}

/// Session breakdown per day: wrapped (interactive=1), unwrapped (interactive IS NULL),
/// non-interactive (interactive=0).
fn load_session_breakdown(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<BTreeMap<NaiveDate, SessionBreakdown>> {
    // Build WHERE clauses with table-qualified column names for the join query.
    let mut clauses = vec![
        "e.source_date >= ?".to_string(),
        "e.source_date <= ?".to_string(),
    ];
    let mut params: Vec<SqlValue> = vec![
        SqlValue::Text(range.from.format("%Y-%m-%d").to_string()),
        SqlValue::Text(range.to.format("%Y-%m-%d").to_string()),
    ];
    if let Some(provider) = filters.provider {
        clauses.push("e.provider = ?".to_string());
        params.push(SqlValue::Text(provider.as_slug().to_string()));
    }
    if let Some(repo) = filters.repo.as_deref() {
        clauses.push("(e.repo_name = ? OR (e.repo_org || '/' || e.repo_name) = ?)".to_string());
        params.push(SqlValue::Text(repo.to_string()));
        params.push(SqlValue::Text(repo.to_string()));
    }
    if let Some(area) = filters.package_area.as_deref() {
        clauses.push("e.package_area = ?".to_string());
        params.push(SqlValue::Text(area.to_string()));
    }
    if let Some(pkg) = filters.package.as_deref() {
        clauses.push("e.package = ?".to_string());
        params.push(SqlValue::Text(pkg.to_string()));
    }

    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };

    // Wrapped: explicitly interactive (wrapper sets INTERACTIVE=true),
    //   OR pre-tracking session with a real session ID (non-anonymous).
    // Unwrapped: pre-tracking anonymous sessions (no real session ID, no wrapper).
    // Non-interactive: explicitly non-interactive (wrapper sets INTERACTIVE=false).
    let sql = format!(
        r#"SELECT e.source_date,
           COUNT(DISTINCT CASE
               WHEN s.interactive = 1 THEN e.session_key
               WHEN s.interactive IS NULL AND e.anonymous_session = 0 THEN e.session_key
           END),
           COUNT(DISTINCT CASE
               WHEN s.interactive IS NULL AND e.anonymous_session = 1 THEN e.session_key
           END),
           COUNT(DISTINCT CASE WHEN s.interactive = 0 THEN e.session_key END),
           COUNT(DISTINCT e.session_key),
           COUNT(DISTINCT CASE
               WHEN json_extract(e.extra_json, '$.yolo') IN ('true', 1) THEN e.session_key
               WHEN LOWER(COALESCE(e.permission_mode, '')) = 'yolo' THEN e.session_key
           END)
           FROM events e LEFT JOIN sessions s ON e.session_key = s.session_key
           {where_clause}
           GROUP BY e.source_date ORDER BY e.source_date ASC"#
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;

    let mut map = BTreeMap::new();
    for row in rows {
        let (date, wrapped, unwrapped, non_interactive, total_sessions, yolo_sessions) = row?;
        let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
        map.insert(
            date,
            SessionBreakdown {
                wrapped: wrapped.max(0) as u64,
                unwrapped: unwrapped.max(0) as u64,
                non_interactive: non_interactive.max(0) as u64,
                total_sessions: total_sessions.max(0) as u64,
                yolo_sessions: yolo_sessions.max(0) as u64,
            },
        );
    }
    Ok(map)
}

/// Distinct repositories per day, using `org/name` when available.
fn load_daily_repos(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<BTreeMap<NaiveDate, Vec<String>>> {
    let mut builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    builder.clauses.push("repo_name IS NOT NULL".to_string());

    let sql = builder.finish(
        r#"SELECT source_date, repo_org, repo_name
           FROM events"#,
    ) + " GROUP BY source_date, repo_org, repo_name \
         ORDER BY source_date ASC, repo_org ASC, repo_name ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut map: BTreeMap<NaiveDate, Vec<String>> = BTreeMap::new();
    for row in rows {
        let (date, repo_org, repo_name) = row?;
        let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
        let repo = repo_label(repo_org.as_deref(), Some(repo_name.as_str()));
        let repos = map.entry(date).or_default();
        if !repos.contains(&repo) {
            repos.push(repo);
        }
    }
    Ok(map)
}
