use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use claudine::reporting::{ErrorRecord, ErrorsReport};

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
        if let Some(detail) = error_detail(index, item) {
            log::data(&detail.render(&term));
        }
    }
}

/// The detail rows (heading, then model, tool, and prompt) for one error, or
/// `None` when the table already shows everything there is.
fn error_detail(index: usize, item: &ErrorRecord) -> Option<Prose> {
    let has_detail = item.prompt.is_some() || item.tool_name.is_some() || item.model.is_some();
    if !has_detail {
        return None;
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
    Some(Prose::new(lines.join("\n")).with_line_breaks(LineBreaks::Hard))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_detail_puts_each_field_on_its_own_row() {
        let record = ErrorRecord {
            timestamp: chrono::Utc::now(),
            provider: claudine::provider::Provider::Claude,
            event: claudine::events::AgenticEvent::SessionEnd,
            session_key: "s".into(),
            session_id: None,
            repo_name: None,
            tool_name: Some("Bash".into()),
            model: Some("opus".into()),
            error: "boom".into(),
            prompt: Some("do it".into()),
            tool_input: None,
            notification_message: None,
            extra: None,
            claudine_pid: None,
            agent_pid: None,
        };
        let term = biscuit_terminal::terminal::Terminal::builder()
            .width(300)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
            .build();
        let rendered = biscuit_terminal::utils::escape_codes::strip_escape_codes(
            error_detail(0, &record).expect("detail rows").render(&term),
        );
        let rows: Vec<&str> = rendered.lines().map(str::trim).collect();
        assert_eq!(
            rows,
            ["─── Error 1 ───", "Model:  opus", "Tool:   Bash", "Prompt: do it"],
            "{rendered:?}"
        );
    }
}
