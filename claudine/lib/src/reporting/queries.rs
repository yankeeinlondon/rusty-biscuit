use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};

use crate::error::{ClaudineError, Result};
use crate::events::AgenticEvent;
use crate::provider::Provider;
use super::metrics::{RecoveryEvent, classify_tool, normalize_tool_name, summarize_metrics};
use super::types::{
    DailySummary, DailyToolStat, DateRange, ErrorRecord, ErrorsReport, LabeledCount, ProviderSplit,
    RepoActivity, ReportingFilters, ReposReport, SessionDetailReport, SessionEvent, SessionInfo,
    SessionsReport, ToolsReport, TrendPoint, TrendsReport, UsageTotals,
};

#[derive(Debug, Default)]
struct WhereBuilder {
    clauses: Vec<String>,
    params: Vec<SqlValue>,
}

impl WhereBuilder {
    fn with_range(mut self, range: DateRange) -> Self {
        self.clauses.push("source_date >= ?".to_string());
        self.params
            .push(SqlValue::Text(range.from.format("%Y-%m-%d").to_string()));
        self.clauses.push("source_date <= ?".to_string());
        self.params
            .push(SqlValue::Text(range.to.format("%Y-%m-%d").to_string()));
        self
    }

    fn with_filters(mut self, filters: &ReportingFilters) -> Self {
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
    fn with_alias(mut self, alias: &str) -> Self {
        self.clauses = self
            .clauses
            .into_iter()
            .map(|clause| prefix_clause_columns(alias, &clause))
            .collect();
        self
    }

    fn finish(&self, sql: &str) -> String {
        if self.clauses.is_empty() {
            sql.to_string()
        } else {
            format!("{sql} WHERE {}", self.clauses.join(" AND "))
        }
    }
}

/// Prefix known column names in a WHERE clause with a table alias.
fn prefix_clause_columns(alias: &str, clause: &str) -> String {
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
struct Totals {
    total_events: u64,
    session_count: u64,
    total_turns: u64,
    total_tool_calls: u64,
    total_tool_errors: u64,
    total_turn_errors: u64,
    total_subagents: u64,
    total_compactions: u64,
    total_permission_requests: u64,
    total_human_in_loop: u64,
    provider_count: u64,
    repo_count: u64,
    usage: UsageTotals,
}

#[derive(Debug, Default, Clone, Copy)]
struct SessionBreakdown {
    wrapped: u64,
    unwrapped: u64,
    non_interactive: u64,
    total_sessions: u64,
    yolo_sessions: u64,
}

pub(crate) fn daily_summary(
    conn: &Connection,
    date: NaiveDate,
    filters: &ReportingFilters,
) -> Result<DailySummary> {
    let range = DateRange::single(date);
    let totals = load_totals(conn, range, filters)?;
    let sessions = load_sessions(conn, range, filters)?;
    let top_tools = load_tool_stats(conn, range, filters, 5)?;
    let all_tools = load_all_tool_stats(conn, range, filters)?;
    let recovery_events = load_recovery_events(conn, range, filters)?;

    Ok(DailySummary {
        date,
        total_events: totals.total_events,
        session_count: totals.session_count,
        total_turns: totals.total_turns,
        total_tool_calls: totals.total_tool_calls,
        total_tool_errors: totals.total_tool_errors,
        total_turn_errors: totals.total_turn_errors,
        total_subagents: totals.total_subagents,
        total_compactions: totals.total_compactions,
        total_permission_requests: totals.total_permission_requests,
        total_human_in_loop: totals.total_human_in_loop,
        provider_count: totals.provider_count,
        repo_count: totals.repo_count,
        providers: load_provider_split(conn, range, filters)?,
        top_tools,
        permission_modes: load_labeled_counts(conn, "permission_mode", range, filters)?,
        models: load_labeled_counts(conn, "model", range, filters)?,
        usage: totals.usage,
        metrics: summarize_metrics(
            totals.total_turns,
            totals.total_tool_calls,
            totals.total_tool_errors,
            totals.total_compactions,
            &sessions,
            &all_tools,
            &recovery_events,
        ),
    })
}

pub(crate) fn sessions(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<SessionsReport> {
    validate_range(range)?;

    let sessions = load_sessions(conn, range, filters)?;
    let totals = load_totals(conn, range, filters)?;
    let all_tools = load_all_tool_stats(conn, range, filters)?;
    let recovery_events = load_recovery_events(conn, range, filters)?;

    Ok(SessionsReport {
        range,
        metrics: summarize_metrics(
            totals.total_turns,
            totals.total_tool_calls,
            totals.total_tool_errors,
            totals.total_compactions,
            &sessions,
            &all_tools,
            &recovery_events,
        ),
        sessions,
    })
}

pub(crate) fn tools(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
    top_n: usize,
) -> Result<ToolsReport> {
    validate_range(range)?;

    let top_tools = load_tool_stats(conn, range, filters, top_n)?;
    let all_tools = load_all_tool_stats(conn, range, filters)?;
    let sessions = load_sessions(conn, range, filters)?;
    let totals = load_totals(conn, range, filters)?;
    let recovery_events = load_recovery_events(conn, range, filters)?;

    Ok(ToolsReport {
        range,
        metrics: summarize_metrics(
            totals.total_turns,
            totals.total_tool_calls,
            totals.total_tool_errors,
            totals.total_compactions,
            &sessions,
            &all_tools,
            &recovery_events,
        ),
        tools: top_tools,
    })
}

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

pub(crate) fn repos(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<ReposReport> {
    validate_range(range)?;

    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"
        SELECT timestamp, repo_name, repo_org, branch, head_sha, is_dirty, session_key
        FROM events
        "#,
    ) + " AND repo_name IS NOT NULL ORDER BY repo_name ASC, timestamp ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;

    #[derive(Default)]
    struct RepoAccumulator {
        repo_name: String,
        repo_org: Option<String>,
        event_count: u64,
        sessions: HashSet<String>,
        head_shas: HashSet<String>,
        dirty_transitions: u64,
        last_dirty: Option<bool>,
        branches: BTreeMap<String, u64>,
    }

    // Key repos by org/name composite to avoid conflating org-a/foo and org-b/foo.
    let mut repos: HashMap<String, RepoAccumulator> = HashMap::new();
    for row in rows {
        let (_, repo_name, repo_org, branch, head_sha, is_dirty, session_key) = row?;
        let repo_key = match &repo_org {
            Some(org) => format!("{org}/{repo_name}"),
            None => repo_name.clone(),
        };
        let entry = repos.entry(repo_key).or_default();
        entry.event_count += 1;
        entry.repo_name = repo_name;
        entry.repo_org = entry.repo_org.clone().or(repo_org);
        entry.sessions.insert(session_key);

        if let Some(head_sha) = head_sha {
            entry.head_shas.insert(head_sha);
        }

        if let Some(branch) = branch {
            *entry.branches.entry(branch).or_default() += 1;
        }

        if let Some(is_dirty) = is_dirty {
            let dirty = is_dirty != 0;
            if let Some(previous) = entry.last_dirty
                && previous != dirty
            {
                entry.dirty_transitions += 1;
            }
            entry.last_dirty = Some(dirty);
        }
    }

    let mut items = repos
        .into_values()
        .map(|accumulator| RepoActivity {
            repo_name: accumulator.repo_name,
            repo_org: accumulator.repo_org,
            event_count: accumulator.event_count,
            session_count: accumulator.sessions.len() as u64,
            head_sha_count: accumulator.head_shas.len() as u64,
            dirty_transitions: accumulator.dirty_transitions,
            branches: accumulator
                .branches
                .into_iter()
                .map(|(label, count)| LabeledCount { label, count })
                .collect(),
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .event_count
            .cmp(&left.event_count)
            .then_with(|| left.repo_name.cmp(&right.repo_name))
    });

    Ok(ReposReport {
        range,
        repos: items,
    })
}

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

/// Look up a single session by session_key or session_id and return all its data.
pub(crate) fn session_detail(conn: &Connection, id: &str) -> Result<SessionDetailReport> {
    // Resolve the session_key: try exact match on session_key first, then session_id.
    let session_key: String = conn
        .query_row(
            "SELECT session_key FROM events WHERE session_key = ?1 LIMIT 1",
            [id],
            |row| row.get(0),
        )
        .or_else(|_| {
            conn.query_row(
                "SELECT session_key FROM events WHERE session_id = ?1 LIMIT 1",
                [id],
                |row| row.get(0),
            )
        })
        .map_err(|_| {
            ClaudineError::ConfigValidation(format!("no session found matching `{id}`"))
        })?;

    // Load session summary by constructing a filter-less query scoped to this session_key.
    let session = load_session_by_key(conn, &session_key)?;

    // Load all events for the session.
    let events = load_session_events(conn, &session_key)?;

    // Load tool stats for this session.
    let tools = load_session_tool_stats(conn, &session_key)?;

    // Load errors for this session.
    let errors = load_session_errors(conn, &session_key)?;

    Ok(SessionDetailReport {
        session,
        events,
        tools,
        errors,
    })
}

fn load_session_by_key(conn: &Connection, session_key: &str) -> Result<SessionInfo> {
    let row = conn.query_row(
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
            COALESCE(SUM(cost_usd), 0)
        FROM events
        WHERE session_key = ?1
        GROUP BY session_key
        "#,
        [session_key],
        |row| {
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
            ))
        },
    )?;

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
    ) = row;

    let started_at = parse_timestamp(&started_at)?;
    let ended_at = parse_timestamp(&ended_at)?;

    Ok(SessionInfo {
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
    })
}

fn load_session_events(conn: &Connection, session_key: &str) -> Result<Vec<SessionEvent>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT timestamp, event, tool_name, agent_type, model, permission_mode,
               cwd, repo_name, repo_org, branch, head_sha, is_dirty, hostname,
               error, prompt_text, notification_type, notification_message,
               tool_input_json, tool_response_json,
               COALESCE(input_tokens, 0), COALESCE(output_tokens, 0),
               COALESCE(total_tokens, 0), COALESCE(cache_read_tokens, 0),
               COALESCE(cost_usd, 0),
               extra_json, env_json
        FROM events
        WHERE session_key = ?1
        ORDER BY timestamp ASC
        "#,
    )?;

    let rows = stmt.query_map([session_key], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<i64>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, Option<String>>(14)?,
            row.get::<_, Option<String>>(15)?,
            row.get::<_, Option<String>>(16)?,
            row.get::<_, Option<String>>(17)?,
            row.get::<_, Option<String>>(18)?,
            row.get::<_, i64>(19)?,
            row.get::<_, i64>(20)?,
            row.get::<_, i64>(21)?,
            row.get::<_, i64>(22)?,
            row.get::<_, f64>(23)?,
            row.get::<_, String>(24)?,
            row.get::<_, String>(25)?,
        ))
    })?;

    let mut events = Vec::new();
    for row in rows {
        let (
            timestamp,
            event,
            tool_name,
            agent_type,
            model,
            permission_mode,
            cwd,
            repo_name,
            repo_org,
            branch,
            head_sha,
            is_dirty,
            hostname,
            error,
            prompt,
            notification_type,
            notification_message,
            tool_input_json,
            tool_response_json,
            input_tokens,
            output_tokens,
            total_tokens,
            cache_read_tokens,
            cost_usd,
            extra_json,
            env_json,
        ) = row?;

        events.push(SessionEvent {
            timestamp: parse_timestamp(&timestamp)?,
            event: parse_event(&event)?,
            tool_name,
            agent_type,
            model,
            permission_mode,
            cwd,
            repo_name,
            repo_org,
            branch,
            head_sha,
            is_dirty: is_dirty.map(|v| v != 0),
            hostname,
            error,
            prompt,
            notification_type,
            notification_message,
            tool_input: parse_json_value(tool_input_json),
            tool_response: parse_json_value(tool_response_json),
            input_tokens: input_tokens.max(0) as u64,
            output_tokens: output_tokens.max(0) as u64,
            total_tokens: total_tokens.max(0) as u64,
            cache_read_tokens: cache_read_tokens.max(0) as u64,
            cost_usd: cost_usd.max(0.0),
            extra: parse_json_value(Some(extra_json)),
            env: parse_json_value(Some(env_json)),
        });
    }

    Ok(events)
}

fn load_session_tool_stats(conn: &Connection, session_key: &str) -> Result<Vec<DailyToolStat>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            tool_name,
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) AS call_count,
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) AS error_count
        FROM events
        WHERE session_key = ?1
          AND tool_name IS NOT NULL
          AND event IN ('before_tool', 'tool_error')
        GROUP BY tool_name
        HAVING COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) > 0
            OR COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) > 0
        ORDER BY call_count DESC, error_count DESC, tool_name ASC
        "#,
    )?;

    let rows = stmt.query_map([session_key], |row| {
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

fn load_session_errors(conn: &Connection, session_key: &str) -> Result<Vec<ErrorRecord>> {
    let mut stmt = conn.prepare(
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
        WHERE e.session_key = ?1
          AND e.event IN ('tool_error', 'turn_error')
        ORDER BY e.timestamp ASC
        "#,
    )?;

    let rows = stmt.query_map([session_key], |row| {
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
            sk,
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
            session_key: sk,
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

    Ok(errors)
}

fn validate_range(range: DateRange) -> Result<()> {
    if range.from > range.to {
        return Err(ClaudineError::InvalidReportingDateRange {
            from: range.from.to_string(),
            to: range.to.to_string(),
        });
    }

    Ok(())
}

fn load_totals(conn: &Connection, range: DateRange, filters: &ReportingFilters) -> Result<Totals> {
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

fn load_provider_split(
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

fn load_labeled_counts(
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

fn load_sessions(
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
            COALESCE(SUM(cost_usd), 0)
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
        });
    }

    Ok(sessions)
}

/// Load all tool stats without a row limit, for accurate derived metrics.
fn load_all_tool_stats(
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

fn load_tool_stats(
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
fn merge_tool_stats(stats: Vec<DailyToolStat>) -> Vec<DailyToolStat> {
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

fn load_recovery_events(
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

fn repo_label(repo_org: Option<&str>, repo_name: Option<&str>) -> String {
    match (repo_org, repo_name) {
        (Some(org), Some(name)) => format!("{org}/{name}"),
        (None, Some(name)) => name.to_string(),
        _ => "—".to_string(),
    }
}

fn parse_provider(slug: &str) -> Result<Provider> {
    Provider::parse_cli_name(slug).ok_or_else(|| {
        ClaudineError::ConfigValidation(format!("unknown provider in reporting database: {slug}"))
    })
}

fn parse_event(slug: &str) -> Result<AgenticEvent> {
    AgenticEvent::from_slug(slug).ok_or_else(|| {
        ClaudineError::ConfigValidation(format!("unknown event in reporting database: {slug}"))
    })
}

fn parse_timestamp(raw: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(raw)?.with_timezone(&Utc))
}

/// Parse a JSON string into a `serde_json::Value`, returning `None` for empty objects.
fn parse_json_value(raw: Option<String>) -> Option<serde_json::Value> {
    raw.and_then(|s| {
        if s == "{}" {
            return None;
        }
        serde_json::from_str(&s).ok()
    })
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
