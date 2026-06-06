#[cfg(test)]
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};

use crate::error::{ClaudineError, Result};
use crate::events::AgenticEvent;
use crate::provider::Provider;
use crate::reporting::metrics::{RecoveryEvent, classify_tool, normalize_tool_name};
use crate::reporting::types::{
    DailyToolStat, DateRange, LabeledCount, ProviderSplit, ReportingFilters, SessionInfo,
    UsageTotals,
};

#[derive(Debug, Default)]
pub(crate) struct WhereBuilder {
    pub(crate) clauses: Vec<String>,
    pub(crate) params: Vec<SqlValue>,
}

impl WhereBuilder {
    pub(crate) fn with_range(mut self, range: DateRange) -> Self {
        self.clauses.push("source_date >= ?".to_string());
        self.params
            .push(SqlValue::Text(range.from.format("%Y-%m-%d").to_string()));
        self.clauses.push("source_date <= ?".to_string());
        self.params
            .push(SqlValue::Text(range.to.format("%Y-%m-%d").to_string()));
        self
    }

    pub(crate) fn with_filters(mut self, filters: &ReportingFilters) -> Self {
        if let Some(provider) = filters.provider {
            self.clauses.push("provider = ?".to_string());
            self.params
                .push(SqlValue::Text(provider.as_slug().to_string()));
        }

        if let Some(repo) = filters.repo.as_deref() {
            self.clauses
                .push("(repo_name = ? OR (repo_org || '/' || repo_name) = ?)".to_string());
            self.params.push(SqlValue::Text(repo.to_string()));
            self.params.push(SqlValue::Text(repo.to_string()));
        }

        if let Some(package_area) = filters.package_area.as_deref() {
            self.clauses.push("package_area = ?".to_string());
            self.params.push(SqlValue::Text(package_area.to_string()));
        }

        if let Some(package) = filters.package.as_deref() {
            self.clauses.push("package = ?".to_string());
            self.params.push(SqlValue::Text(package.to_string()));
        }

        self
    }

    /// Prefix all clauses with a table alias (e.g. `"e"` → `e.source_date`).
    pub(crate) fn with_alias(mut self, alias: &str) -> Self {
        self.clauses = self
            .clauses
            .into_iter()
            .map(|clause| prefix_clause_columns(alias, &clause))
            .collect();
        self
    }

    pub(crate) fn finish(&self, sql: &str) -> String {
        if self.clauses.is_empty() {
            sql.to_string()
        } else {
            format!("{sql} WHERE {}", self.clauses.join(" AND "))
        }
    }
}

/// Prefix known column names in a WHERE clause with a table alias.
pub(crate) fn prefix_clause_columns(alias: &str, clause: &str) -> String {
    const COLUMNS: &[&str] = &[
        "source_date",
        "provider",
        "repo_name",
        "repo_org",
        "package_area",
        "package",
    ];
    let mut result = clause.to_string();
    for col in COLUMNS {
        // Only replace bare column names (not already prefixed).
        result = result.replace(&format!("{col} "), &format!("{alias}.{col} "));
        result = result.replace(&format!("({col}"), &format!("({alias}.{col}"));
    }
    result
}

#[derive(Debug, Default)]
pub(crate) struct Totals {
    pub(crate) total_events: u64,
    pub(crate) session_count: u64,
    pub(crate) total_turns: u64,
    pub(crate) total_tool_calls: u64,
    pub(crate) total_tool_errors: u64,
    pub(crate) total_turn_errors: u64,
    pub(crate) total_subagents: u64,
    pub(crate) total_compactions: u64,
    pub(crate) total_permission_requests: u64,
    pub(crate) total_human_in_loop: u64,
    pub(crate) provider_count: u64,
    pub(crate) repo_count: u64,
    pub(crate) usage: UsageTotals,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SessionBreakdown {
    pub(crate) wrapped: u64,
    pub(crate) unwrapped: u64,
    pub(crate) non_interactive: u64,
    pub(crate) total_sessions: u64,
    pub(crate) yolo_sessions: u64,
}

pub(crate) fn validate_range(range: DateRange) -> Result<()> {
    if range.from > range.to {
        return Err(ClaudineError::InvalidReportingDateRange {
            from: range.from.to_string(),
            to: range.to.to_string(),
        });
    }

    Ok(())
}

pub(crate) fn parse_provider(slug: &str) -> Result<Provider> {
    Provider::parse_cli_name(slug).ok_or_else(|| {
        ClaudineError::ConfigValidation(format!("unknown provider in reporting database: {slug}"))
    })
}

pub(crate) fn parse_event(slug: &str) -> Result<AgenticEvent> {
    AgenticEvent::from_slug(slug).ok_or_else(|| {
        ClaudineError::ConfigValidation(format!("unknown event in reporting database: {slug}"))
    })
}

pub(crate) fn parse_timestamp(raw: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(raw)?.with_timezone(&Utc))
}

/// Parse a JSON string into a `serde_json::Value`, returning `None` for empty objects.
pub(crate) fn parse_json_value(raw: Option<String>) -> Option<serde_json::Value> {
    raw.and_then(|s| {
        if s == "{}" {
            return None;
        }
        serde_json::from_str(&s).ok()
    })
}

pub(crate) fn repo_label(repo_org: Option<&str>, repo_name: Option<&str>) -> String {
    match (repo_org, repo_name) {
        (Some(org), Some(name)) => format!("{org}/{name}"),
        (None, Some(name)) => name.to_string(),
        _ => "—".to_string(),
    }
}

pub(crate) fn load_totals(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<Totals> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"
        SELECT
            COUNT(*) AS total_events,
            COALESCE(SUM(CASE WHEN event = 'session_start' THEN 1 ELSE 0 END), 0) AS session_count,
            COALESCE(SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END), 0) AS turn_count,
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) AS tool_call_count,
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) AS tool_error_count,
            COALESCE(SUM(CASE WHEN event = 'turn_error' THEN 1 ELSE 0 END), 0) AS turn_error_count,
            COALESCE(SUM(CASE WHEN event = 'subagent_start' THEN 1 ELSE 0 END), 0) AS subagent_count,
            COALESCE(SUM(CASE WHEN event = 'before_compact' THEN 1 ELSE 0 END), 0) AS compaction_count,
            COALESCE(SUM(CASE WHEN event = 'permission_request' THEN 1 ELSE 0 END), 0) AS permission_request_count,
            COALESCE(SUM(CASE WHEN event = 'human_in_the_loop' THEN 1 ELSE 0 END), 0) AS human_in_loop_count,
            COUNT(DISTINCT provider) AS provider_count,
            COUNT(DISTINCT CASE WHEN repo_org IS NOT NULL THEN repo_org || '/' || repo_name ELSE repo_name END) AS repo_count,
            COALESCE(SUM(input_tokens), 0) AS total_input_tokens,
            COALESCE(SUM(output_tokens), 0) AS total_output_tokens,
            COALESCE(SUM(total_tokens), 0) AS sum_total_tokens,
            COALESCE(SUM(cache_read_tokens), 0) AS total_cache_read_tokens,
            COALESCE(SUM(cost_usd), 0) AS total_cost_usd
        FROM events
        "#,
    );

    conn.query_row(&sql, params_from_iter(builder.params.iter()), |row| {
        Ok(Totals {
            total_events: row.get::<_, i64>(0)?.max(0) as u64,
            session_count: row.get::<_, i64>(1)?.max(0) as u64,
            total_turns: row.get::<_, i64>(2)?.max(0) as u64,
            total_tool_calls: row.get::<_, i64>(3)?.max(0) as u64,
            total_tool_errors: row.get::<_, i64>(4)?.max(0) as u64,
            total_turn_errors: row.get::<_, i64>(5)?.max(0) as u64,
            total_subagents: row.get::<_, i64>(6)?.max(0) as u64,
            total_compactions: row.get::<_, i64>(7)?.max(0) as u64,
            total_permission_requests: row.get::<_, i64>(8)?.max(0) as u64,
            total_human_in_loop: row.get::<_, i64>(9)?.max(0) as u64,
            provider_count: row.get::<_, i64>(10)?.max(0) as u64,
            repo_count: row.get::<_, i64>(11)?.max(0) as u64,
            usage: UsageTotals {
                total_input_tokens: row.get::<_, i64>(12)?.max(0) as u64,
                total_output_tokens: row.get::<_, i64>(13)?.max(0) as u64,
                total_tokens: row.get::<_, i64>(14)?.max(0) as u64,
                total_cache_read_tokens: row.get::<_, i64>(15)?.max(0) as u64,
                total_cost_usd: row.get::<_, f64>(16)?.max(0.0),
            },
        })
    })
    .map_err(Into::into)
}

pub(crate) fn load_provider_split(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<Vec<ProviderSplit>> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"SELECT provider, COUNT(*),
           COALESCE(SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END), 0),
           COALESCE(SUM(CASE WHEN event IN ('tool_error', 'turn_error') THEN 1 ELSE 0 END), 0)
           FROM events"#,
    ) + " GROUP BY provider ORDER BY COUNT(*) DESC, provider ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;

    let mut providers = Vec::new();
    for row in rows {
        let (provider, count, turns, error_count) = row?;
        providers.push(ProviderSplit {
            provider: parse_provider(&provider)?,
            count: count.max(0) as u64,
            turns: turns.max(0) as u64,
            error_count: error_count.max(0) as u64,
        });
    }
    Ok(providers)
}

pub(crate) fn load_labeled_counts(
    conn: &Connection,
    field: &str,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<Vec<LabeledCount>> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(&format!("SELECT {field}, COUNT(*) FROM events"))
        + &format!(" AND {field} IS NOT NULL GROUP BY {field} ORDER BY COUNT(*) DESC, {field} ASC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut items = Vec::new();
    for row in rows {
        let (label, count) = row?;
        items.push(LabeledCount {
            label,
            count: count.max(0) as u64,
        });
    }
    Ok(items)
}

pub(crate) fn load_sessions(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<Vec<SessionInfo>> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    // Use MIN for identity fields (first seen) and last-seen for mutable fields
    // to give meaningful labels when sessions cross branches/repos/models.
    let sql = builder.finish(
        r#"
        SELECT
            session_key,
            MIN(session_id),
            MIN(provider),
            MIN(timestamp),
            MAX(timestamp),
            MIN(cwd),
            MIN(repo_name),
            MIN(repo_org),
            MIN(branch),
            MIN(package_area),
            MIN(package),
            -- model and permission_mode use MAX (last seen) since they can change mid-session
            MAX(model),
            MAX(permission_mode),
            MIN(hostname),
            MIN(primary_language),
            COUNT(*),
            COALESCE(SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'turn_error' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'subagent_start' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(cache_read_tokens), 0),
            COALESCE(SUM(cost_usd), 0),
            MAX(claudine_pid),
            MAX(agent_pid)
        FROM events
        "#,
    ) + " GROUP BY session_key ORDER BY MIN(timestamp) DESC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, Option<String>>(14)?,
            row.get::<_, i64>(15)?,
            row.get::<_, i64>(16)?,
            row.get::<_, i64>(17)?,
            row.get::<_, i64>(18)?,
            row.get::<_, i64>(19)?,
            row.get::<_, i64>(20)?,
            row.get::<_, i64>(21)?,
            row.get::<_, i64>(22)?,
            row.get::<_, i64>(23)?,
            row.get::<_, i64>(24)?,
            row.get::<_, f64>(25)?,
            row.get::<_, Option<i64>>(26)?,
            row.get::<_, Option<i64>>(27)?,
        ))
    })?;

    let mut sessions = Vec::new();
    for row in rows {
        let (
            session_key,
            session_id,
            provider,
            started_at,
            ended_at,
            cwd,
            repo_name,
            repo_org,
            branch,
            package_area,
            package,
            model,
            permission_mode,
            hostname,
            primary_language,
            event_count,
            turn_count,
            tool_call_count,
            tool_error_count,
            turn_error_count,
            subagent_count,
            total_input_tokens,
            total_output_tokens,
            total_tokens,
            total_cache_read_tokens,
            total_cost_usd,
            claudine_pid,
            agent_pid,
        ) = row?;
        let started_at = parse_timestamp(&started_at)?;
        let ended_at = parse_timestamp(&ended_at)?;
        sessions.push(SessionInfo {
            session_key,
            session_id,
            provider: parse_provider(&provider)?,
            started_at,
            ended_at,
            duration_seconds: (ended_at - started_at).num_seconds().max(0),
            cwd,
            repo_name,
            repo_org,
            branch,
            package_area,
            package,
            model,
            permission_mode,
            hostname,
            primary_language,
            event_count: event_count.max(0) as u64,
            turn_count: turn_count.max(0) as u64,
            tool_call_count: tool_call_count.max(0) as u64,
            tool_error_count: tool_error_count.max(0) as u64,
            turn_error_count: turn_error_count.max(0) as u64,
            subagent_count: subagent_count.max(0) as u64,
            total_input_tokens: total_input_tokens.max(0) as u64,
            total_output_tokens: total_output_tokens.max(0) as u64,
            total_tokens: total_tokens.max(0) as u64,
            total_cache_read_tokens: total_cache_read_tokens.max(0) as u64,
            total_cost_usd: total_cost_usd.max(0.0),
            claudine_pid: claudine_pid.and_then(|v| v.try_into().ok()),
            agent_pid: agent_pid.and_then(|v| v.try_into().ok()),
        });
    }

    Ok(sessions)
}

/// Load all tool stats without a row limit, for accurate derived metrics.
pub(crate) fn load_all_tool_stats(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<Vec<DailyToolStat>> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"
        SELECT
            tool_name,
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) AS call_count,
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) AS error_count
        FROM events
        "#,
    ) + " AND tool_name IS NOT NULL AND event IN ('before_tool', 'tool_error')
         GROUP BY tool_name
         HAVING COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) > 0
             OR COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) > 0
         ORDER BY call_count DESC, error_count DESC, tool_name ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut raw_tools = Vec::new();
    for row in rows {
        let (tool_name, call_count, error_count) = row?;
        raw_tools.push(DailyToolStat {
            classification: classify_tool(&tool_name),
            tool_name: normalize_tool_name(&tool_name).to_string(),
            call_count: call_count.max(0) as u64,
            error_count: error_count.max(0) as u64,
        });
    }
    Ok(merge_tool_stats(raw_tools))
}

pub(crate) fn load_tool_stats(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
    top_n: usize,
) -> Result<Vec<DailyToolStat>> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"
        SELECT
            tool_name,
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) AS call_count,
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) AS error_count
        FROM events
        "#,
    ) + " AND tool_name IS NOT NULL AND event IN ('before_tool', 'tool_error')
         GROUP BY tool_name
         HAVING COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) > 0
             OR COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) > 0
         ORDER BY call_count DESC, error_count DESC, tool_name ASC LIMIT ?";

    let mut params = builder.params;
    params.push(SqlValue::Integer(i64::try_from(top_n).unwrap_or(i64::MAX)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut raw_tools = Vec::new();
    for row in rows {
        let (tool_name, call_count, error_count) = row?;
        raw_tools.push(DailyToolStat {
            classification: classify_tool(&tool_name),
            tool_name: normalize_tool_name(&tool_name).to_string(),
            call_count: call_count.max(0) as u64,
            error_count: error_count.max(0) as u64,
        });
    }
    // Merge after normalization, then apply top_n limit
    let mut merged = merge_tool_stats(raw_tools);
    merged.truncate(top_n);
    Ok(merged)
}

/// Merge tool stats that share the same normalized name.
///
/// After normalization, multiple SQL rows may map to the same tool name
/// (e.g. `grep_search` → `Grep`). This merges their counts and sorts by
/// call count descending.
pub(crate) fn merge_tool_stats(stats: Vec<DailyToolStat>) -> Vec<DailyToolStat> {
    use std::collections::HashMap;
    let mut merged: HashMap<String, DailyToolStat> = HashMap::new();
    for stat in stats {
        let entry = merged
            .entry(stat.tool_name.clone())
            .or_insert(DailyToolStat {
                classification: stat.classification,
                tool_name: stat.tool_name,
                call_count: 0,
                error_count: 0,
            });
        entry.call_count += stat.call_count;
        entry.error_count += stat.error_count;
    }
    let mut result: Vec<DailyToolStat> = merged.into_values().collect();
    result.sort_by(|a, b| {
        b.call_count
            .cmp(&a.call_count)
            .then(b.error_count.cmp(&a.error_count))
            .then(a.tool_name.cmp(&b.tool_name))
    });
    result
}

pub(crate) fn load_recovery_events(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<Vec<RecoveryEvent>> {
    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish("SELECT session_key, tool_name, event FROM events")
        + " AND event IN ('tool_error', 'after_tool') ORDER BY session_key ASC, timestamp ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok(RecoveryEvent {
            session_key: row.get(0)?,
            tool_name: row.get(1)?,
            event: AgenticEvent::from_slug(&row.get::<_, String>(2)?)
                .unwrap_or(AgenticEvent::Notification),
        })
    })?;

    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_range_rejects_reversed_dates() {
        let range = DateRange {
            from: NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            to: NaiveDate::from_ymd_opt(2026, 4, 3).unwrap(),
        };

        assert!(matches!(
            validate_range(range),
            Err(ClaudineError::InvalidReportingDateRange { .. })
        ));
    }

    #[test]
    fn repo_label_formats_org_and_repo_name() {
        assert_eq!(
            repo_label(Some("openai"), Some("claudine")),
            "openai/claudine"
        );
        assert_eq!(repo_label(None, Some("claudine")), "claudine");
        assert_eq!(repo_label(None, None), "—");
    }

    #[test]
    fn parse_provider_and_json_value_handle_invalid_inputs() {
        assert_eq!(parse_provider("claude").unwrap(), Provider::Claude);
        assert!(parse_provider("not-a-provider").is_err());
        assert_eq!(parse_json_value(Some("{}".to_string())), None);
        assert_eq!(
            parse_json_value(Some("{\"ok\":true}".to_string())),
            Some(serde_json::json!({ "ok": true }))
        );
    }
}
