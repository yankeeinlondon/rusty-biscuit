use rusqlite::Connection;

use crate::error::{ClaudineError, Result};
use crate::reporting::metrics::{classify_tool, normalize_tool_name, summarize_metrics};
use crate::reporting::types::{
    DailyToolStat, DateRange, ErrorRecord, ReportingFilters, SessionDetailReport, SessionEvent,
    SessionInfo, SessionsReport,
};

use super::common::{
    load_all_tool_stats, load_recovery_events, load_sessions, load_totals, merge_tool_stats,
    parse_event, parse_json_value, parse_provider, parse_timestamp, validate_range,
};

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
