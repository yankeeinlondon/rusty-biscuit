use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::components::table::types::ColumnType;
use biscuit_terminal::utils::layout::{Alignment, WordWrap};
use claudine::reporting::TrendsReport;

use crate::log;
use crate::table_utils::base_table;

use super::common::{
    error_hint_markup, format_dim_zero, format_errors, format_percent, render_provider_split,
    render_repos,
};

pub(super) fn render_trends_report(report: &TrendsReport, error_hint: Option<&str>) {
    let term = crate::log::terminal();
    let compact_repos = term.width() < 150;
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Trends</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );
    log::data("");

    let _tool_calls = if term.width() > 120 {
        Some(TableColumn::new("Tool\nCalls").with_type(ColumnType::Integer))
    } else {
        None
    };

    let mut table = base_table(vec![
        TableColumn::new("Date").with_fixed_width(10),
        TableColumn::new("Wrap")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center),
        TableColumn::new("Unwrap")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center),
        TableColumn::new("Non-\nInt")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center),
        TableColumn::new("Yolo")
            .with_alignment(Alignment::Center)
            .with_fixed_width(4),
        TableColumn::new("Tool\nCalls")
            .with_type(ColumnType::Integer)
            .with_word_wrap(WordWrap::None),
        TableColumn::new("Turn\nErrs")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center)
            .with_fixed_width(4),
        TableColumn::new("Repos").with_word_wrap(WordWrap::WrapProse(None, None)),
        TableColumn::new("Providers").with_word_wrap(WordWrap::WrapProse(None, None)),
    ])
    .alternate_background_color();

    for point in &report.points {
        table.add_row(vec![
            point.date.to_string().into(),
            format_dim_zero(point.wrapped).into(),
            format_dim_zero(point.unwrapped).into(),
            format_dim_zero(point.non_interactive).into(),
            format_percent(point.yolo_percent).into(),
            point.tool_calls.to_string().into(),
            format_errors(point.turn_errors).into(),
            render_repos(&point.repos, compact_repos).into(),
            render_provider_split(&point.providers).into(),
        ]);
    }

    log::data(&table.render(&term));

    // Definitions footer
    let mut definitions = vec![
        RenderableContent::from(Prose::new(
            "<b>Wrapped:</b> <i><dim>interactive sessions where <blue>claudine</blue> is wrapping the execution of an Agent (e.g. `claudine claude`, `claudine codex`, etc.)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Unwrapped:</b> <i><dim>sessions detected via hooks, execution was not wrapped by <blue>claudine</blue></dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Non Interactive:</b> <i><dim>non-interactive sessions wrapped by <blue>claudine</blue></dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Yolo:</b> <i><dim>percentage of daily sessions that ran with YOLO enabled</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Tools:</b> <i><dim>total tool invocations across all sessions (Read, Edit, Bash, etc.)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Tool Err:</b> <i><dim>failed tool calls (file not found, permission denied, command failed)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Turn Err:</b> <i><dim>failed response cycles (API errors, rate limits, context overflow)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Repos:</b> <i><dim>repositories worked on that day</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Providers:</b> <i><dim>active agentic providers with usage dashboard links; providers with errors are shown in red</dim></i>",
        )),
    ];
    if report
        .points
        .iter()
        .any(|point| point.tool_errors + point.turn_errors > 0)
    {
        definitions.push(RenderableContent::from(Prose::new(error_hint_markup(
            error_hint.unwrap_or("week"),
        ))));
    }
    let definitions = UnorderedList::from(definitions).with_bullet("  ");
    log::data(&definitions.render(&term));
}
