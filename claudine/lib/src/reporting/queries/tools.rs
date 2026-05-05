use rusqlite::Connection;

use crate::reporting::metrics::summarize_metrics;
use crate::reporting::types::{DateRange, ReportingFilters, ToolsReport};
use crate::error::Result;

use super::common::{
    load_all_tool_stats, load_recovery_events, load_sessions, load_tool_stats, load_totals,
    validate_range,
};

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
