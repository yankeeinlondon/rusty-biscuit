use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::utils::layout::Alignment;
use claudine::reporting::ToolsReport;

use crate::log;
use crate::table_utils::base_table;

use super::common::{format_errors, render_metrics_line};

pub(super) fn render_tools_report(report: &ToolsReport) {
    let term = crate::log::terminal();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Tools</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );
    render_metrics_line(&report.metrics);

    let mut table = base_table(vec![
        TableColumn::new("Tool"),
        TableColumn::new("Calls").with_alignment(Alignment::Right),
        TableColumn::new("Errors").with_alignment(Alignment::Right),
        TableColumn::new("Error %").with_alignment(Alignment::Right),
        TableColumn::new("Class"),
    ]);

    for tool in &report.tools {
        let error_rate = if tool.call_count == 0 {
            0.0
        } else {
            (tool.error_count as f64 / tool.call_count as f64) * 100.0
        };
        table.add_row(vec![
            tool.tool_name.clone().into(),
            tool.call_count.to_string().into(),
            format_errors(tool.error_count).into(),
            format!("{error_rate:.1}").into(),
            format!("{:?}", tool.classification).to_lowercase().into(),
        ]);
    }

    log::data(&table.render(&term));
}
