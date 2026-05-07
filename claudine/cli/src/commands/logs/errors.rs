use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::TableColumn;
use claudine::reporting::ErrorsReport;

use crate::log;
use crate::table_utils::base_table;

use super::common::truncate_str;

pub(super) fn render_errors_report(report: &ErrorsReport) {
    let term = crate::log::terminal();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Errors</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );

    let mut table = base_table(vec![
        TableColumn::new("Time"),
        TableColumn::new("Provider"),
        TableColumn::new("Session"),
        TableColumn::new("Error"),
    ]);

    for item in &report.errors {
        let session_display = item.session_id.as_deref().unwrap_or("—").to_string();
        let error_display = if item.error.is_empty() {
            "(no details)".to_string()
        } else {
            truncate_str(&item.error, 120)
        };
        table.add_row(vec![
            item.timestamp.format("%Y-%m-%d %H:%M").to_string().into(),
            item.provider.to_string().into(),
            session_display.into(),
            error_display.into(),
        ]);
    }

    log::data(&table.render(&term));

    // Show additional detail per error when there's info beyond what the table shows.
    for (index, item) in report.errors.iter().enumerate() {
        let has_detail = item.prompt.is_some() || item.tool_name.is_some() || item.model.is_some();
        if !has_detail {
            continue;
        }

        let mut lines = vec![format!("<dim>─── Error {} ───</dim>", index + 1)];
        if let Some(model) = &item.model {
            lines.push(format!("  <dim>Model:</dim>  {model}"));
        }
        if let Some(tool) = &item.tool_name {
            lines.push(format!("  <dim>Tool:</dim>   {tool}"));
        }
        if let Some(prompt) = &item.prompt {
            let display = truncate_str(prompt, 200);
            lines.push(format!("  <dim>Prompt:</dim> {display}"));
        }

        log::data(&Prose::new(lines.join("\n")).render(&term));
    }
}
