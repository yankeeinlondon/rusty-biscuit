use chrono::NaiveDate;
use rusqlite::Connection;

use crate::error::Result;
use crate::reporting::metrics::summarize_metrics;
use crate::reporting::types::{DailySummary, DateRange, ReportingFilters};

use super::common::{
    load_all_tool_stats, load_labeled_counts, load_provider_split, load_recovery_events,
    load_sessions, load_tool_stats, load_totals,
};

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
