use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::components::table::table::TableCellContent;
use clap::Args;
use color_eyre::eyre::Result;
use darkmatter::markdown::compose::context::{
    context_variable_descriptors, ContextVariableDescriptor,
};
use darkmatter::markdown::compose::expression::{
    expression_function_descriptors, ExpressionFunctionDescriptor,
};
use darkmatter::markdown::compose::ComposeContext;

use crate::commands::context_render::{
    context_column_widths, inline_code_text, middle_elide, render_context_section, MAX_REPORT_WIDTH,
};
use crate::log;

mod effects;
mod expressions;
mod format;
#[cfg(test)]
mod tests;

use format::{display_property, format_context_value_type, format_value};

/// Arguments for the `claudine context` subcommand.
#[derive(Debug, Args)]
#[group(id = "context_report", multiple = false)]
pub struct ContextArgs {
    /// Show live values for each context variable.
    #[arg(long, group = "context_report")]
    pub values: bool,

    /// Show the expression engine's operations and functions.
    #[arg(long, group = "context_report")]
    pub expressions: bool,

    /// Show the available side-effect capabilities.
    #[arg(long, group = "context_report")]
    pub side_effects: bool,
}

// ------------------------------------------------------------------
// Grouping helpers
// ------------------------------------------------------------------

fn group_context_descriptors(
) -> Vec<((&'static str, &'static str), Vec<&'static ContextVariableDescriptor>)> {
    let mut groups: Vec<((&'static str, &'static str), Vec<&'static ContextVariableDescriptor>)> =
        Vec::new();
    let mut current_key: Option<(&'static str, &'static str)> = None;

    for desc in context_variable_descriptors() {
        let key = (desc.category, desc.subsection);
        if current_key != Some(key) {
            current_key = Some(key);
            groups.push((key, Vec::new()));
        }
        groups.last_mut().unwrap().1.push(desc);
    }
    groups
}

fn group_expression_descriptors(
) -> Vec<(&'static str, Vec<&'static ExpressionFunctionDescriptor>)> {
    // The catalog is physically laid out by implementation grouping, so a
    // single category (e.g. `Math`, `Collection`) can appear in non-adjacent
    // runs. Consolidate every category into one group, ordered by first
    // appearance, then honor each descriptor's `order` as the stable display
    // order within the category.
    let mut groups: Vec<(&'static str, Vec<&'static ExpressionFunctionDescriptor>)> = Vec::new();

    for desc in expression_function_descriptors() {
        match groups.iter_mut().find(|(cat, _)| *cat == desc.category) {
            Some((_, funcs)) => funcs.push(desc),
            None => groups.push((desc.category, vec![desc])),
        }
    }

    for (_, funcs) in &mut groups {
        funcs.sort_by_key(|f| f.order);
    }

    groups
}

// ------------------------------------------------------------------
// Default report
// ------------------------------------------------------------------

fn render_default_report() {
    let term = log::terminal();
    let groups = group_context_descriptors();

    let all_properties: Vec<String> = groups
        .iter()
        .flat_map(|(_, vars)| vars.iter().map(|v| display_property(v.name)))
        .collect();
    let all_types: Vec<String> = groups
        .iter()
        .flat_map(|(_, vars)| {
            vars.iter()
                .map(|v| format_context_value_type(&v.display_type, &term))
        })
        .collect();

    let property_labels: Vec<&str> = all_properties.iter().map(|s| s.as_str()).collect();
    let type_labels: Vec<&str> = all_types.iter().map(|s| s.as_str()).collect();
    let (property_width, type_width) = context_column_widths(&property_labels, &type_labels);

    for ((category, subsection), descriptors) in &groups {
        if subsection.is_empty() {
            let heading = Prose::new(format!("<blue><b>{category}</b></blue>"));
            log::data(&heading.render(&term));
        } else {
            let heading =
                Prose::new(format!("<blue><b>{category}</b></blue> — <b>{subsection}</b>"));
            log::data(&heading.render(&term));
        }

        let rendered = render_context_section(
            &term,
            property_width,
            type_width,
            "Description",
            |table| {
                for desc in descriptors {
                    let row: Vec<TableCellContent> = vec![
                        display_property(desc.name).into(),
                        format_context_value_type(&desc.display_type, &term).into(),
                        inline_code_text(desc.description, &term).into(),
                    ];
                    table.add_row(row);
                }
            },
        );
        log::data(&rendered);
        log::data("");
    }
}

// ------------------------------------------------------------------
// Values report
// ------------------------------------------------------------------

fn render_values_report() {
    render_values_report_with(ComposeContext::capture);
}

/// Renders the values report from a single injected capture.
///
/// `capture` is invoked exactly once — the report reuses one captured
/// `ComposeContext` for every section and row rather than re-capturing per
/// section. Tests inject a counting closure to enforce that contract (and the
/// host/repository discovery it implies) does not regress.
fn render_values_report_with(capture: impl FnOnce() -> ComposeContext) {
    let term = log::terminal();
    let ctx = capture();
    let values = ctx.values();
    let groups = group_context_descriptors();

    let all_properties: Vec<String> = groups
        .iter()
        .flat_map(|(_, vars)| vars.iter().map(|v| display_property(v.name)))
        .collect();
    let all_types: Vec<String> = groups
        .iter()
        .flat_map(|(_, vars)| {
            vars.iter()
                .map(|v| format_context_value_type(&v.display_type, &term))
        })
        .collect();

    let property_labels: Vec<&str> = all_properties.iter().map(|s| s.as_str()).collect();
    let type_labels: Vec<&str> = all_types.iter().map(|s| s.as_str()).collect();
    let (property_width, type_width) = context_column_widths(&property_labels, &type_labels);

    // Budget for the trailing `Value` column at the resolved render width. The
    // report-content column keeps path-separator break characters (so tables
    // render at the minimum supported width), but wrapping a path at `/` yields
    // a line ending in `/` that reads as a complete parent directory. Pre-eliding
    // a single-token value (a path) that would otherwise wrap keeps it whole and
    // unambiguous. Chrome = leading space + four `│` borders + per-cell padding.
    const TABLE_CHROME: usize = 12;
    let render_width = term.width().min(MAX_REPORT_WIDTH) as usize;
    let value_budget = render_width.saturating_sub(property_width + type_width + TABLE_CHROME);

    for ((category, subsection), descriptors) in &groups {
        if subsection.is_empty() {
            let heading = Prose::new(format!("<blue><b>{category}</b></blue>"));
            log::data(&heading.render(&term));
        } else {
            let heading =
                Prose::new(format!("<blue><b>{category}</b></blue> — <b>{subsection}</b>"));
            log::data(&heading.render(&term));
        }

        let rendered = render_context_section(
            &term,
            property_width,
            type_width,
            "Value",
            |table| {
                for desc in descriptors {
                    let value = values.get(desc.name).unwrap_or(&serde_json::Value::Null);
                    // Single-token string values (filesystem paths) are
                    // middle-elided to the column budget so they never wrap at a
                    // `/` into a deceptively-complete parent path. Values with
                    // whitespace (CSVs, prose, lists) keep normal wrapping.
                    let value_str = if value.is_null() {
                        Prose::new("<dim>null</dim>").render(&term)
                    } else if let Some(s) =
                        value.as_str().filter(|s| !s.chars().any(char::is_whitespace))
                    {
                        middle_elide(s, value_budget)
                    } else {
                        format_value(value, &term)
                    };

                    let row: Vec<TableCellContent> = vec![
                        display_property(desc.name).into(),
                        format_context_value_type(&desc.display_type, &term).into(),
                        value_str.into(),
                    ];
                    table.add_row(row);
                }
            },
        );
        log::data(&rendered);
        log::data("");
    }
}

// ------------------------------------------------------------------
// Expressions report
// ------------------------------------------------------------------

fn render_expressions_report() {
    let term = log::terminal();

    // Title
    let title = Prose::new("<blue><b>Darkmatter Expression Engine</b></blue>");
    log::data(&title.render(&term));
    log::data("");

    // Brief description
    let desc = Prose::new(
        "Expressions evaluate in <b>interpolation</b> <dim>{{ ... }}</dim> and <b>conditions</b> <dim>when=\"...\"</dim> surfaces.",
    );
    log::data(&desc.render(&term));
    log::data("");

    expressions::render_expressions_precedence(&term);
    expressions::render_expressions_truthiness(&term);
    expressions::render_expressions_unary(&term);
    expressions::render_expressions_comparison(&term);
    expressions::render_expressions_arithmetic(&term);
    expressions::render_expressions_variable_access(&term);
    expressions::render_expressions_modes(&term);
    expressions::render_expressions_null_propagation(&term);
    expressions::render_expressions_functions(&term);

    render_footer(true);
}

// ------------------------------------------------------------------
// Footer
// ------------------------------------------------------------------

fn render_footer(show_values_hint: bool) {
    let term = log::terminal();

    if show_values_hint {
        let msg = Status::from_prose(
            "<dim><i>use <blue>--values</blue> to convert the descriptions to actual host values</i></dim>",
        )
        .state(StatusState::Info);
        log::message(&msg.render(&term));
    }

    let msg1 = Status::from_prose(
        "<dim><i>use <blue>--expressions</blue> to see the expression engine's operations and functions</i></dim>",
    )
    .state(StatusState::Info);
    log::message(&msg1.render(&term));

    let msg2 = Status::from_prose(
        "<dim><i>use <blue>--side-effects</blue> to see the Darkmatter capabilities</i></dim>",
    )
    .state(StatusState::Info);
    log::message(&msg2.render(&term));
}

// ------------------------------------------------------------------
// Entry point
// ------------------------------------------------------------------

/// Show Darkmatter runtime context, expression engine, and side effects.
pub fn run(args: ContextArgs) -> Result<()> {
    if args.expressions {
        render_expressions_report();
        return Ok(());
    }

    if args.side_effects {
        effects::render_side_effects_report();
        render_footer(true);
        return Ok(());
    }

    if args.values {
        render_values_report();
        render_footer(false);
    } else {
        render_default_report();
        render_footer(true);
    }

    Ok(())
}
