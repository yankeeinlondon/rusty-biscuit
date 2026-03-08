use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};

use crate::error::{ClaudineError, Result};
use crate::events::{AgenticEvent, Provider};

use super::metrics::{RecoveryEvent, classify_tool, summarize_metrics};
use super::types::{
    DailySummary, DailyToolStat, DateRange, ErrorRecord, ErrorsReport, LabeledCount, ProviderSplit,
    RepoActivity, ReportingFilters, ReposReport, SessionInfo, SessionsReport, ToolsReport,
    TrendPoint, TrendsReport,
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

    fn finish(&self, sql: &str) -> String {
        if self.clauses.is_empty() {
            sql.to_string()
        } else {
            format!("{sql} WHERE {}", self.clauses.join(" AND "))
        }
    }
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
}

pub(crate) fn daily_summary(
    conn: &Connection,
    date: NaiveDate,
    filters: &ReportingFilters,
) -> Result<DailySummary> {
    let range = DateRange::single(date);
    let totals = load_totals(conn, range, filters)?;
    let sessions = load_sessions(conn, range, filters)?;
    let tools = load_tool_stats(conn, range, filters, 5)?;
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
        top_tools: tools.clone(),
        permission_modes: load_labeled_counts(conn, "permission_mode", range, filters)?,
        models: load_labeled_counts(conn, "model", range, filters)?,
        metrics: summarize_metrics(
            totals.total_turns,
            totals.total_tool_calls,
            totals.total_tool_errors,
            totals.total_compactions,
            &sessions,
            &tools,
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
    let tools = load_tool_stats(conn, range, filters, 25)?;
    let recovery_events = load_recovery_events(conn, range, filters)?;

    Ok(SessionsReport {
        range,
        metrics: summarize_metrics(
            totals.total_turns,
            totals.total_tool_calls,
            totals.total_tool_errors,
            totals.total_compactions,
            &sessions,
            &tools,
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

    let tools = load_tool_stats(conn, range, filters, top_n)?;
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
            &tools,
            &recovery_events,
        ),
        tools,
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
        .with_filters(filters);
    let sql = builder.finish(
        r#"
        SELECT timestamp, provider, event, session_key, repo_name, tool_name, error,
               prompt_text, tool_input_json, notification_message
        FROM events
        "#,
    ) + " AND event IN ('tool_error', 'turn_error') ORDER BY timestamp DESC LIMIT ?";

    let mut params = builder.params;
    params.push(SqlValue::Integer(i64::try_from(top_n).unwrap_or(i64::MAX)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        let prompt: Option<String> = row.get(7)?;
        let tool_input_json: Option<String> = row.get(8)?;
        let notification_message: Option<String> = row.get(9)?;

        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
            short_context(prompt, tool_input_json, notification_message),
        ))
    })?;

    let mut errors = Vec::new();
    for row in rows {
        let (timestamp, provider, event, session_key, repo_name, tool_name, error, context) = row?;
        errors.push(ErrorRecord {
            timestamp: parse_timestamp(&timestamp)?,
            provider: parse_provider(&provider)?,
            event: parse_event(&event)?,
            session_key,
            repo_name,
            tool_name,
            error: truncate(&error, 120),
            context,
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
        repo_org: Option<String>,
        event_count: u64,
        sessions: HashSet<String>,
        head_shas: HashSet<String>,
        dirty_transitions: u64,
        last_dirty: Option<bool>,
        branches: BTreeMap<String, u64>,
    }

    let mut repos: HashMap<String, RepoAccumulator> = HashMap::new();
    for row in rows {
        let (_, repo_name, repo_org, branch, head_sha, is_dirty, session_key) = row?;
        let entry = repos.entry(repo_name).or_default();
        entry.event_count += 1;
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
        .into_iter()
        .map(|(repo_name, accumulator)| RepoActivity {
            repo_name,
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
            COALESCE(SUM(CASE WHEN event IN ('tool_error', 'turn_error') THEN 1 ELSE 0 END), 0) AS error_count
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
        ))
    })?;

    let provider_points = load_provider_points(conn, range, filters)?;
    let mut points = Vec::new();
    for row in rows {
        let (date, events, sessions, turns, tools, errors) = row?;
        let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
        points.push(TrendPoint {
            date,
            events: events.max(0) as u64,
            sessions: sessions.max(0) as u64,
            turns: turns.max(0) as u64,
            tool_calls: tools.max(0) as u64,
            errors: errors.max(0) as u64,
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
            COUNT(DISTINCT session_key) AS session_count,
            COALESCE(SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END), 0) AS turn_count,
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0) AS tool_call_count,
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0) AS tool_error_count,
            COALESCE(SUM(CASE WHEN event = 'turn_error' THEN 1 ELSE 0 END), 0) AS turn_error_count,
            COALESCE(SUM(CASE WHEN event = 'subagent_start' THEN 1 ELSE 0 END), 0) AS subagent_count,
            COALESCE(SUM(CASE WHEN event = 'before_compact' THEN 1 ELSE 0 END), 0) AS compaction_count,
            COALESCE(SUM(CASE WHEN event = 'permission_request' THEN 1 ELSE 0 END), 0) AS permission_request_count,
            COALESCE(SUM(CASE WHEN event = 'human_in_the_loop' THEN 1 ELSE 0 END), 0) AS human_in_loop_count,
            COUNT(DISTINCT provider) AS provider_count,
            COUNT(DISTINCT repo_name) AS repo_count
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
    let sql = builder.finish("SELECT provider, COUNT(*) FROM events")
        + " GROUP BY provider ORDER BY COUNT(*) DESC, provider ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut providers = Vec::new();
    for row in rows {
        let (provider, count) = row?;
        providers.push(ProviderSplit {
            provider: parse_provider(&provider)?,
            count: count.max(0) as u64,
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
    let sql = builder.finish(
        r#"
        SELECT
            session_key,
            MAX(session_id),
            MAX(provider),
            MIN(timestamp),
            MAX(timestamp),
            MAX(cwd),
            MAX(repo_name),
            MAX(repo_org),
            MAX(branch),
            MAX(package_area),
            MAX(package),
            MAX(model),
            MAX(permission_mode),
            MAX(hostname),
            MAX(primary_language),
            COUNT(*),
            COALESCE(SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'turn_error' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN event = 'subagent_start' THEN 1 ELSE 0 END), 0)
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
        });
    }

    Ok(sessions)
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

    let mut tools = Vec::new();
    for row in rows {
        let (tool_name, call_count, error_count) = row?;
        tools.push(DailyToolStat {
            classification: classify_tool(&tool_name),
            tool_name,
            call_count: call_count.max(0) as u64,
            error_count: error_count.max(0) as u64,
        });
    }
    Ok(tools)
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
    let sql = builder.finish("SELECT source_date, provider, COUNT(*) FROM events")
        + " GROUP BY source_date, provider ORDER BY source_date ASC, COUNT(*) DESC, provider ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut map: BTreeMap<NaiveDate, Vec<ProviderSplit>> = BTreeMap::new();
    for row in rows {
        let (date, provider, count) = row?;
        map.entry(NaiveDate::parse_from_str(&date, "%Y-%m-%d")?)
            .or_default()
            .push(ProviderSplit {
                provider: parse_provider(&provider)?,
                count: count.max(0) as u64,
            });
    }
    Ok(map)
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

fn short_context(
    prompt: Option<String>,
    tool_input_json: Option<String>,
    notification_message: Option<String>,
) -> Option<String> {
    prompt
        .map(|value| truncate(&value, 120))
        .or_else(|| tool_input_json.map(|value| truncate(&value, 120)))
        .or_else(|| notification_message.map(|value| truncate(&value, 120)))
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }

    value
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>()
        + "…"
}
