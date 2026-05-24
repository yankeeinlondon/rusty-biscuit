use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::utils::layout::Alignment;
use claudine::reporting::DailySummary;

use crate::log;
use crate::table_utils::base_table;

use super::common::{
    format_errors, render_error_hint, render_labeled_counts, render_metrics_line,
    render_provider_split, render_usage_line,
};

pub(super) fn render_daily_summary(summary: &DailySummary, error_hint: Option<&str>) {
    let term = crate::log::terminal();

    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Claudine Logs</bold></blue> <dim>▸</dim> <bold>{}</bold>",
            summary.date
        ))
        .render(&term),
    );
    log::data(&format!(
        "Events {}  Sessions {}  Turns {}  Tools {}  Tool errors {}  Turn errors {}",
        summary.total_events,
        summary.session_count,
        summary.total_turns,
        summary.total_tool_calls,
        summary.total_tool_errors,
        summary.total_turn_errors
    ));
    log::data(&format!(
        "Subagents {}  Compactions {}  Permissions {}  Human-in-loop {}  Providers {}  Repos {}",
        summary.total_subagents,
        summary.total_compactions,
        summary.total_permission_requests,
        summary.total_human_in_loop,
        summary.provider_count,
        summary.repo_count
    ));
    log::data(&format!(
        "Provider split: {}",
        render_provider_split(&summary.providers)
    ));

    if !summary.permission_modes.is_empty() {
        log::data(&format!(
            "Permission modes: {}",
            render_labeled_counts(&summary.permission_modes)
        ));
    }

    if !summary.models.is_empty() {
        log::data(&format!(
            "Models: {}",
            render_labeled_counts(&summary.models)
        ));
    }

    render_usage_line(&summary.usage);
    render_metrics_line(&summary.metrics);

    if !summary.top_tools.is_empty() {
        log::data("");
        log::data(&Prose::new("<bold>Top Tools</bold>").render(&term));
        let mut table = base_table(vec![
            TableColumn::new("Tool"),
            TableColumn::new("Calls").with_alignment(Alignment::Right),
            TableColumn::new("Errors").with_alignment(Alignment::Right),
            TableColumn::new("Class"),
        ]);
        for tool in &summary.top_tools {
            table.add_row(vec![
                tool.tool_name.clone().into(),
                tool.call_count.to_string().into(),
                format_errors(tool.error_count).into(),
                format!("{:?}", tool.classification).to_lowercase().into(),
            ]);
        }
        log::data(&table.render(&term));
    }

    if summary.total_tool_errors + summary.total_turn_errors > 0 {
        if !summary.top_tools.is_empty() {
            log::data("");
        }
        render_error_hint(&term, error_hint.unwrap_or("today"));
    }
}
