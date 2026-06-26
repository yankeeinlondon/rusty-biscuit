use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::components::table::table::{Table, TableCellContent};
use biscuit_terminal::terminal::Terminal;
use clap::Args;
use color_eyre::eyre::Result;
use darkmatter::effects::{effect_descriptors, EffectDescriptor, EffectSafety};
use darkmatter::markdown::compose::context::{
    context_variable_descriptors, ContextValueType, ContextVariableDescriptor,
};
use darkmatter::markdown::compose::expression::{
    expression_function_descriptors, ExpressionFunctionDescriptor,
};
use darkmatter::markdown::compose::expression::semantics::{
    arithmetic_operator_descriptors, comparison_operator_descriptors, mode_descriptors,
    null_propagation_descriptors, operator_descriptors, truthiness_descriptors,
    unary_operator_descriptors, variable_access_descriptors,
};
use darkmatter::markdown::compose::ComposeContext;

use crate::commands::context_render::{
    configure_shared_table, context_column_widths, function_first_column_width, inline_code_text,
    middle_elide, render_context_section, render_table_resilient, report_column,
    render_unordered_list, MAX_REPORT_WIDTH, TableLayout, EXAMPLE_COLUMN_MIN_WIDTH,
};
#[cfg(test)]
use crate::commands::context_render::render_table_within_contract;
use crate::log;

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

fn group_effect_descriptors() -> Vec<(&'static str, Vec<&'static EffectDescriptor>)> {
    let mut groups: Vec<(&'static str, Vec<&'static EffectDescriptor>)> = Vec::new();
    let mut current_cat: Option<&'static str> = None;

    for desc in effect_descriptors() {
        if current_cat != Some(desc.category) {
            current_cat = Some(desc.category);
            groups.push((desc.category, Vec::new()));
        }
        groups.last_mut().unwrap().1.push(desc);
    }
    groups
}

// ------------------------------------------------------------------
// Formatting helpers
// ------------------------------------------------------------------

fn display_property(name: &str) -> String {
    format!("ctx.{name}")
}

fn format_context_value_type(ty: ContextValueType, term: &Terminal) -> String {
    Prose::new(context_value_type_markup(ty)).render(term)
}

/// Builds the Prose markup for a type label, coloring each type by category.
///
/// `Nullable(inner)` wraps the inner type's own colored markup in grey
/// `Nullable(…)` so the report shows `Nullable(String)`, `Nullable(Integer)`,
/// etc. rather than a bare `Nullable`.
fn context_value_type_markup(ty: ContextValueType) -> String {
    match ty {
        ContextValueType::Nullable(inner) => {
            format!(
                "<grey>Nullable(</grey>{}<grey>)</grey>",
                context_value_type_markup(*inner)
            )
        }
        ContextValueType::String
        | ContextValueType::Csv
        | ContextValueType::MarkdownList
        | ContextValueType::NestedMarkdownList => {
            format!("<blue>{}</blue>", context_value_type_label(ty))
        }
        ContextValueType::Number | ContextValueType::Integer => {
            format!("<green>{}</green>", context_value_type_label(ty))
        }
        ContextValueType::Boolean => format!("<orange>{}</orange>", context_value_type_label(ty)),
        ContextValueType::Date
        | ContextValueType::DateTime
        | ContextValueType::Time
        | ContextValueType::Timezone => {
            format!("<violet>{}</violet>", context_value_type_label(ty))
        }
        ContextValueType::Object => format!("<cyan>{}</cyan>", context_value_type_label(ty)),
    }
}

fn context_value_type_label(ty: ContextValueType) -> &'static str {
    match ty {
        ContextValueType::Date => "Date",
        ContextValueType::DateTime => "DateTime",
        ContextValueType::Time => "Time",
        ContextValueType::Timezone => "Timezone",
        ContextValueType::Integer => "Integer",
        ContextValueType::Number => "Number",
        ContextValueType::Boolean => "Boolean",
        ContextValueType::String => "String",
        ContextValueType::Csv => "Csv",
        ContextValueType::MarkdownList => "MarkdownList",
        ContextValueType::NestedMarkdownList => "NestedMarkdownList",
        ContextValueType::Object => "Object",
        ContextValueType::Nullable(_) => "Nullable",
    }
}

fn format_value(value: &serde_json::Value, term: &Terminal) -> String {
    match value {
        serde_json::Value::Null => Prose::new("<dim>null</dim>").render(term),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| format_array_element(v, term)).collect();
            items.join(", ")
        }
        serde_json::Value::Object(_) => value.to_string(),
    }
}

/// Formats a single array element for the values report.
///
/// Scalar elements use the same plain rules as top-level values (e.g. a string
/// renders as `alpha`, not the JSON-quoted `"alpha"`). Nested arrays and objects
/// retain compact JSON serialization, since there is no flat plain form for them.
fn format_array_element(value: &serde_json::Value, term: &Terminal) -> String {
    match value {
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => value.to_string(),
        scalar => format_value(scalar, term),
    }
}

fn format_safety(safety: EffectSafety, term: &Terminal) -> String {
    let text = match safety {
        EffectSafety::FilesystemWrite => "FilesystemWrite",
        EffectSafety::Network => "Network",
        EffectSafety::MarkdownMutation => "MarkdownMutation",
    };
    let colored = match safety {
        EffectSafety::FilesystemWrite => format!("<orange>{text}</orange>"),
        EffectSafety::Network => format!("<red>{text}</red>"),
        EffectSafety::MarkdownMutation => format!("<blue>{text}</blue>"),
    };
    Prose::new(colored).render(term)
}

/// Renders a descriptor's optional example for table cells.
fn format_example(example: Option<&darkmatter::catalog::Example>, term: &Terminal) -> String {
    match example {
        Some(ex) => inline_code_text(&format!("{} → {}", ex.invocation, ex.result), term),
        None => String::new(),
    }
}

/// Compact example formatter for side-effect capabilities.
///
/// The signature is already shown in the `Capability` column, so the Example
/// column only displays the result arrow and value, keeping the four-column
/// report within the minimum supported terminal width.
fn format_effect_example(example: Option<&darkmatter::catalog::Example>, term: &Terminal) -> String {
    match example {
        Some(ex) => inline_code_text(&format!("→ {}", ex.result), term),
        None => String::new(),
    }
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
                .map(|v| format_context_value_type(v.display_type, &term))
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
                        format_context_value_type(desc.display_type, &term).into(),
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
                .map(|v| format_context_value_type(v.display_type, &term))
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
                        format_context_value_type(desc.display_type, &term).into(),
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

    // Operator Precedence
    render_expressions_precedence(&term);

    // Truthiness
    render_expressions_truthiness(&term);

    // Unary operators
    render_expressions_unary(&term);

    // Comparison Operators
    render_expressions_comparison(&term);

    // Arithmetic Operators
    render_expressions_arithmetic(&term);

    // Variable Access
    render_expressions_variable_access(&term);

    // Interpolation vs. Condition Mode
    render_expressions_modes(&term);

    // Null Propagation Summary
    render_expressions_null_propagation(&term);

    // Functions
    render_expressions_functions(&term);

    render_footer(true);
}

/// Whether the metadata reports should render their optional `Example` column at
/// `term`'s width.
///
/// Below the threshold the four-column layout cannot satisfy the table planner
/// at the minimum supported width, so the column is dropped (the documented
/// "where layout permits" contract). Shared by every `render_expressions_*`
/// sub-table and the side-effect report.
fn show_examples(term: &Terminal) -> bool {
    term.width() >= EXAMPLE_COLUMN_MIN_WIDTH
}

/// Renders one expression sub-table with a pinned first column, a middle column,
/// and an optional trailing `Example` column.
///
/// `first_header`/`first_width` describe the signature/operator column pinned to
/// align sections; `middle_header` is the second column. `rows` yields each
/// row's `(first, middle, example)` cells already rendered. The `Example` column
/// — header and per-row cell — is included only when [`show_examples`] permits,
/// keeping narrow widths within the minimum-supported-width floor.
fn render_expression_table(
    term: &Terminal,
    first_header: impl Into<String>,
    first_width: usize,
    middle_header: impl Into<String>,
    rows: Vec<(TableCellContent, TableCellContent, TableCellContent)>,
) -> String {
    let first_header = first_header.into();
    let middle_header = middle_header.into();
    let with_examples = show_examples(term);

    render_table_resilient(term, |layout| {
        let mut first = report_column(first_header.clone());
        if layout == TableLayout::Pinned {
            first = first.with_min_width(first_width).with_max_width(first_width);
        }
        let mut columns = vec![first, report_column(middle_header.clone())];
        if with_examples {
            columns.push(report_column("Example"));
        }
        let mut table = Table::new().with_columns(columns);
        configure_shared_table(&mut table);

        for (first_cell, middle_cell, example_cell) in &rows {
            let mut row: Vec<TableCellContent> = vec![first_cell.clone(), middle_cell.clone()];
            if with_examples {
                row.push(example_cell.clone());
            }
            table.add_row(row);
        }
        table
    })
}

fn render_expressions_precedence(term: &Terminal) {
    let heading = Prose::new("<blue><b>Operator Precedence</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let items = operator_descriptors();
    let first_width = function_first_column_width("Precedence", items.iter().map(|d| d.name));

    let rows = items
        .iter()
        .enumerate()
        .map(|(i, desc)| {
            (
                format!("{}", i + 1).into(),
                inline_code_text(desc.operators, term).into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(term, "Precedence", first_width, "Operators", rows);

    log::data(&rendered);
    log::data("");
}

fn render_expressions_truthiness(term: &Terminal) {
    let heading = Prose::new("<blue><b>Truthiness</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let items = truthiness_descriptors();
    let first_width = function_first_column_width("Value", items.iter().map(|d| d.form));

    let rows = items
        .iter()
        .map(|desc| {
            (
                inline_code_text(desc.form, term).into(),
                (if desc.is_falsy { "yes" } else { "no" }).into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(term, "Value", first_width, "Falsy", rows);

    log::data(&rendered);
    log::data("");
}

fn render_expressions_unary(term: &Terminal) {
    let heading = Prose::new("<blue><b>Unary Operators</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let items = unary_operator_descriptors();
    let first_width = function_first_column_width("Operator", items.iter().map(|d| d.syntax));

    let rows = items
        .iter()
        .map(|desc| {
            (
                inline_code_text(desc.syntax, term).into(),
                inline_code_text(desc.description, term).into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(term, "Operator", first_width, "Description", rows);

    log::data(&rendered);
    log::data("");
}

fn render_expressions_comparison(term: &Terminal) {
    let heading = Prose::new("<blue><b>Comparison Operators</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let items = comparison_operator_descriptors();
    let first_width = function_first_column_width("Operator", items.iter().map(|d| d.syntax));

    let rows = items
        .iter()
        .map(|desc| {
            (
                inline_code_text(desc.syntax, term).into(),
                inline_code_text(desc.description, term).into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(term, "Operator", first_width, "Description", rows);

    log::data(&rendered);
    log::data("");
}

fn render_expressions_arithmetic(term: &Terminal) {
    let heading = Prose::new("<blue><b>Arithmetic Operators</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let items = arithmetic_operator_descriptors();
    let first_width = function_first_column_width("Operator", items.iter().map(|d| d.syntax));

    let rows = items
        .iter()
        .map(|desc| {
            (
                inline_code_text(desc.syntax, term).into(),
                inline_code_text(desc.description, term).into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(term, "Operator", first_width, "Description", rows);

    log::data(&rendered);
    log::data("");
}

fn render_expressions_variable_access(term: &Terminal) {
    let heading = Prose::new("<blue><b>Variable Access</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let items = variable_access_descriptors();
    let first_width = function_first_column_width("Form", items.iter().map(|d| d.form));

    let rows = items
        .iter()
        .map(|desc| {
            (
                inline_code_text(desc.form, term).into(),
                inline_code_text(desc.description, term).into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(term, "Form", first_width, "Description", rows);

    log::data(&rendered);
    log::data("");
}

fn render_expressions_modes(term: &Terminal) {
    let heading = Prose::new("<blue><b>Interpolation vs. Condition Mode</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let intro = Prose::new(
        "The parser supports two modes with different operator behavior:",
    );
    log::data(&intro.render(term));
    log::data("");

    let items = mode_descriptors();
    let first_width = function_first_column_width("Surface", items.iter().map(|d| d.surface));

    let rows = items
        .iter()
        .map(|desc| {
            (
                inline_code_text(desc.surface, term).into(),
                desc.pipe_meaning.into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(
        term,
        "Surface",
        first_width,
        inline_code_text("`||` meaning", term),
        rows,
    );

    log::data(&rendered);

    let consequence_items = [
        "`when=\"a || b\"` is logical OR and evaluates to a boolean",
        "`{{ a || \"default\" }}` is fallback sugar and expands to the first truthy value",
        "`{{ a && b }}` is rejected at parse time",
        "`when=\"a && b\"` is logical AND",
        "the function-call forms `and(...)` and `or(...)` are valid in both modes",
    ];
    log::data(&render_unordered_list(
        &consequence_items
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        term,
    ));
}

fn render_expressions_null_propagation(term: &Terminal) {
    let heading = Prose::new("<blue><b>Null Propagation Summary</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let items = null_propagation_descriptors();
    let first_width = function_first_column_width("Operation", items.iter().map(|d| d.operation));

    let rows = items
        .iter()
        .map(|desc| {
            (
                inline_code_text(desc.operation, term).into(),
                inline_code_text(desc.behavior, term).into(),
                format_example(desc.example.as_ref(), term).into(),
            )
        })
        .collect();
    let rendered = render_expression_table(term, "Operation", first_width, "Behavior", rows);

    log::data(&rendered);
    log::data("");
}

fn render_expressions_functions(term: &Terminal) {
    let heading = Prose::new("<blue><b>Functions</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let groups = group_expression_descriptors();

    let all_signatures: Vec<String> = groups
        .iter()
        .flat_map(|(_, funcs)| funcs.iter().map(|f| f.signature.to_string()))
        .collect();
    let sig_refs: Vec<&str> = all_signatures.iter().map(|s| s.as_str()).collect();
    let func_width = function_first_column_width("Function", sig_refs.into_iter());

    for (category, functions) in &groups {
        let sub_heading = Prose::new(format!("<b>{category}</b>"));
        log::data(&sub_heading.render(term));

        let rows = functions
            .iter()
            .map(|func| {
                (
                    inline_code_text(func.signature, term).into(),
                    inline_code_text(func.description, term).into(),
                    format_example(func.example.as_ref(), term).into(),
                )
            })
            .collect();
        let rendered = render_expression_table(term, "Function", func_width, "Description", rows);
        log::data(&rendered);
        log::data("");
    }
}

// ------------------------------------------------------------------
// Side-effects report
// ------------------------------------------------------------------

fn render_side_effects_report() {
    let term = log::terminal();

    let title = Prose::new("<blue><b>Darkmatter Side-Effect Capabilities</b></blue>");
    log::data(&title.render(&term));
    log::data("");

    let intro = Prose::new(
        "This report documents Darkmatter's side-effect capabilities. It is documentation-only and does not invoke any side effects.",
    );
    log::data(&intro.render(&term));

    let constraint_items = [
        "this report is documentation only and does not invoke side effects",
        "only an external orchestrator invokes side effects",
        "filesystem writes are restricted to the configured mutation root",
        "network operations are restricted by the host allowlist, which is deny-all by default",
        "Markdown mutations honor Darkmatter's auto-rehash behavior",
        "capability catalog membership does not imply authorization or availability",
    ];
    log::data(&render_unordered_list(
        &constraint_items
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        &term,
    ));

    let groups = group_effect_descriptors();

    let all_signatures: Vec<String> = groups
        .iter()
        .flat_map(|(_, effects)| effects.iter().map(|e| e.signature.to_string()))
        .collect();
    let sig_refs: Vec<&str> = all_signatures.iter().map(|s| s.as_str()).collect();
    let cap_width = function_first_column_width("Capability", sig_refs.into_iter());

    // The Example column is useful on wider terminals, but at the minimum
    // supported width the four-column report cannot satisfy the table planner.
    // Drop the column below the shared threshold rather than emitting a
    // width-error diagnostic.
    let with_examples = show_examples(&term);

    for (category, effects) in &groups {
        let cat_heading = Prose::new(format!("<blue><b>{category}</b></blue>"));
        log::data(&cat_heading.render(&term));

        let rendered = render_table_resilient(&term, |layout| {
            let mut capability = report_column("Capability");
            if layout == TableLayout::Pinned {
                capability = capability.with_min_width(cap_width).with_max_width(cap_width);
            }
            let mut columns = vec![
                capability,
                report_column("Description"),
                report_column("Safety"),
            ];
            if with_examples {
                columns.insert(2, report_column("Example"));
            }
            let mut table = Table::new().with_columns(columns);
            configure_shared_table(&mut table);
            for effect in effects {
                let mut row: Vec<TableCellContent> = vec![
                    inline_code_text(effect.signature, &term).into(),
                    inline_code_text(effect.description, &term).into(),
                    format_safety(effect.safety, &term).into(),
                ];
                if with_examples {
                    row.insert(
                        2,
                        format_effect_example(effect.example.as_ref(), &term).into(),
                    );
                }
                table.add_row(row);
            }
            table
        });
        log::data(&rendered);
        log::data("");
    }
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
        render_side_effects_report();
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

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::components::table::table::{Table, TableColumn};
    use biscuit_terminal::terminal::Terminal;

    /// The default, expression, and side-effect reports are documentation
    /// only: the spec forbids them from instantiating or probing an effect
    /// engine or touching the network. Rather than asserting that by code
    /// inspection, this drives the real render functions and checks Darkmatter's
    /// process-wide instrumentation counters did not move — catching a
    /// regression that constructs an `EffectEngine` (the gateway to
    /// allowlist/config reads and host discovery) or attempts an HTTP request.
    #[test]
    fn metadata_reports_construct_no_engine_and_attempt_no_network() {
        use darkmatter::effects::{engine_build_count, network_attempt_count};

        let builds_before = engine_build_count();
        let network_before = network_attempt_count();

        render_default_report();
        render_expressions_report();
        render_side_effects_report();

        assert_eq!(
            engine_build_count(),
            builds_before,
            "documentation-only reports must not construct an EffectEngine",
        );
        assert_eq!(
            network_attempt_count(),
            network_before,
            "documentation-only reports must attempt no network access",
        );
    }

    /// The values report must capture the runtime context exactly once per
    /// invocation — never per section or per row. The capture seam lets us
    /// assert that automatically rather than by code inspection.
    #[test]
    fn values_report_captures_context_exactly_once() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        render_values_report_with(|| {
            calls.set(calls.get() + 1);
            ComposeContext::capture()
        });
        assert_eq!(
            calls.get(),
            1,
            "values report must invoke capture exactly once; got {} calls",
            calls.get(),
        );
    }

    /// The spec keeps `Property`, `Type`, and the final column in the default
    /// and values reports at every supported width, relying on wrapping rather
    /// than a Claudine-specific narrow layout. At the documented minimum
    /// supported width (`MIN_SUPPORTED_REPORT_WIDTH`), `render_context_section`
    /// must keep all three columns — even with the widest unbreakable property
    /// name *and* the binding `NestedMarkdownList` type token — by letting the
    /// columns wrap instead of dropping `Type` or emitting the planner's
    /// width-error string inline. Representative content from every column must
    /// survive too, not merely the headers.
    #[test]
    fn default_report_preserves_all_columns_at_minimum_supported_width() {
        use crate::commands::context_render::MIN_SUPPORTED_REPORT_WIDTH;

        // The widest real property; 41 unbreakable cells on spaces.
        let property_width = "ctx.current_package_area_has_staged_files".len();
        // The widest real type token — the constraint that fixes the floor.
        let type_width = "NestedMarkdownList".len();
        let term = Terminal::new_optimistic(MIN_SUPPORTED_REPORT_WIDTH);

        let output = render_context_section(
            &term,
            property_width,
            type_width,
            "Description",
            |table| {
                table.add_row(vec![
                    "ctx.current_package_area_has_staged_files".into(),
                    "Boolean".into(),
                    "Whether the current package area has staged files.".into(),
                ]);
                table.add_row(vec![
                    "ctx.docs_outline".into(),
                    "NestedMarkdownList".into(),
                    "Nested outline of the in-scope docs.".into(),
                ]);
            },
        );

        assert!(
            !output.contains("could not be rendered"),
            "render at the minimum supported width must wrap content rather than \
             emit the width-error string; output was:\n{output}",
        );
        // All three columns survive — no alternate narrow layout drops Type.
        for header in ["Property", "Type", "Description"] {
            assert!(
                output.contains(header),
                "minimum-width report must keep the `{header}` column; output was:\n{output}",
            );
        }
        // Representative content from each column survives the wrap (tokens may
        // wrap across rows, so assert fragments that stay intact).
        for fragment in ["ctx.", "Boolean", "NestedMarkdownList", "staged"] {
            assert!(
                output.contains(fragment),
                "minimum-width report must retain `{fragment}` content; output was:\n{output}",
            );
        }
        // Output fits the terminal width without intentional overflow.
        let max = output
            .lines()
            .map(|l| biscuit_terminal::utils::block_constraint::visible_width(l) as usize)
            .max()
            .unwrap_or(0);
        assert!(
            max <= MIN_SUPPORTED_REPORT_WIDTH as usize,
            "minimum-width report must fit within the terminal width; max={max}; output:\n{output}",
        );
    }

    /// Every context section must use the same computed Property/Type widths.
    /// The widths are derived from the complete catalog and reused per section.
    #[test]
    fn shared_property_type_widths_across_sections() {
        let term = Terminal::new_optimistic(200);
        let groups = group_context_descriptors();

        let all_properties: Vec<String> = groups
            .iter()
            .flat_map(|(_, vars)| vars.iter().map(|v| format!("ctx.{}", v.name)))
            .collect();
        let all_types: Vec<String> = groups
            .iter()
            .flat_map(|(_, vars)| {
                vars.iter()
                    .map(|v| format_context_value_type(v.display_type, &term))
            })
            .collect();

        let property_labels: Vec<&str> = all_properties.iter().map(|s| s.as_str()).collect();
        let type_labels: Vec<&str> = all_types.iter().map(|s| s.as_str()).collect();
        let (property_width, type_width) =
            context_column_widths(&property_labels, &type_labels);

        // Widths must be positive and independently computed.
        assert!(
            property_width > 0,
            "Property width must be positive; got {property_width}"
        );
        assert!(
            type_width > 0,
            "Type width must be positive; got {type_width}"
        );

        // Verify the widths cover the longest entries in the catalog.
        let max_property = property_labels
            .iter()
            .map(|s| biscuit_terminal::utils::block_constraint::visible_width(s) as usize)
            .max()
            .unwrap_or(0);
        let max_type = type_labels
            .iter()
            .map(|s| biscuit_terminal::utils::block_constraint::visible_width(s) as usize)
            .max()
            .unwrap_or(0);

        assert!(
            property_width >= max_property,
            "Property width ({property_width}) must cover longest property ({max_property})"
        );
        assert!(
            type_width >= max_type,
            "Type width ({type_width}) must cover longest type ({max_type})"
        );
    }

    /// The expression report groups the complete function catalog by metadata
    /// category — each category emitted exactly once, even though the catalog is
    /// physically laid out by implementation grouping (so `Math` and `Collection`
    /// appear in non-adjacent runs). Within each category the signatures must
    /// follow the descriptors' stable `order`, not catalog position.
    #[test]
    fn expression_groups_consolidate_categories_in_metadata_order() {
        use std::collections::HashSet;

        let groups = group_expression_descriptors();

        let mut seen = HashSet::new();
        for (category, _) in &groups {
            assert!(
                seen.insert(*category),
                "category `{category}` emitted more than once",
            );
        }

        for (category, functions) in &groups {
            let orders: Vec<usize> = functions.iter().map(|f| f.order).collect();
            let mut expected = orders.clone();
            expected.sort_unstable();
            assert_eq!(
                orders, expected,
                "category `{category}` signatures must follow metadata order; got {orders:?}",
            );
        }
    }

    /// The expression report's `Example` column follows the documented "where
    /// layout permits" contract: it is dropped below `EXAMPLE_COLUMN_MIN_WIDTH`
    /// (mirroring the side-effect report) and present at or above it. The guard
    /// is what keeps the narrow report short enough to stay within the L2
    /// capture pane. Drive `render_expression_table` with a real example cell and
    /// assert the `Example` header's presence flips at the threshold.
    #[test]
    fn expression_table_drops_example_column_below_threshold() {
        use crate::commands::context_render::EXAMPLE_COLUMN_MIN_WIDTH;

        let example_cell: TableCellContent = "min(1, 2) → 1".into();
        let build_rows = || {
            vec![(
                "min(a, b)".into(),
                "smaller of two values".into(),
                example_cell.clone(),
            )]
        };

        // At the threshold the Example column (header and content) is present.
        let at = Terminal::new_optimistic(EXAMPLE_COLUMN_MIN_WIDTH);
        let wide = render_expression_table(&at, "Function", 9, "Description", build_rows());
        assert!(
            wide.contains("Example"),
            "Example column must be present at the threshold width; output:\n{wide}",
        );

        // One cell below the threshold the column is dropped entirely — no empty
        // cells, no header.
        let below = Terminal::new_optimistic(EXAMPLE_COLUMN_MIN_WIDTH - 1);
        let narrow = render_expression_table(&below, "Function", 9, "Description", build_rows());
        assert!(
            !narrow.contains("Example"),
            "Example column must be dropped below the threshold width; output:\n{narrow}",
        );
        // The remaining columns survive.
        for header in ["Function", "Description"] {
            assert!(
                narrow.contains(header),
                "narrow expression table must keep the `{header}` column; output:\n{narrow}",
            );
        }
    }

    /// `show_examples` is the single threshold both the expression sub-tables and
    /// the side-effect report consult, so its boundary must hold exactly at
    /// `EXAMPLE_COLUMN_MIN_WIDTH`.
    #[test]
    fn show_examples_threshold_is_inclusive() {
        use crate::commands::context_render::EXAMPLE_COLUMN_MIN_WIDTH;

        assert!(!show_examples(&Terminal::new_optimistic(
            EXAMPLE_COLUMN_MIN_WIDTH - 1
        )));
        assert!(show_examples(&Terminal::new_optimistic(EXAMPLE_COLUMN_MIN_WIDTH)));
        assert!(show_examples(&Terminal::new_optimistic(
            EXAMPLE_COLUMN_MIN_WIDTH + 1
        )));
    }

    /// Scalar array elements render with plain values — strings without JSON
    /// quotes — while nested arrays/objects keep compact JSON serialization.
    #[test]
    fn format_value_renders_arrays_plainly() {
        let term = Terminal::new_optimistic(200);

        assert_eq!(
            format_value(&serde_json::json!(["alpha", "beta"]), &term),
            "alpha, beta",
        );
        assert_eq!(
            format_value(&serde_json::json!([1, 2, 3]), &term),
            "1, 2, 3",
        );
        assert_eq!(
            format_value(&serde_json::json!([true, false]), &term),
            "true, false",
        );
        // Nested structured elements keep compact JSON.
        assert_eq!(
            format_value(&serde_json::json!([{"k": 1}, ["x"]]), &term),
            r#"{"k":1}, ["x"]"#,
        );
    }

    /// Verify that expression and side-effect reports render without
    /// exceeding the 140ch contract at wide terminal widths.
    #[test]
    fn reports_render_within_140ch_contract_at_wide_terminals() {
        use biscuit_terminal::utils::block_constraint::visible_width;

        let term = Terminal::new_optimistic(200);

        // Build an expression-function table similar to the real report.
        let func_groups = group_expression_descriptors();
        let all_sigs: Vec<String> = func_groups
            .iter()
            .flat_map(|(_, funcs)| funcs.iter().map(|f| f.signature.to_string()))
            .collect();
        let sig_refs: Vec<&str> = all_sigs.iter().map(|s| s.as_str()).collect();
        let func_width = function_first_column_width("Function", sig_refs.into_iter());

        for (category, functions) in &func_groups {
            let columns = vec![
                TableColumn::new("Function")
                    .with_min_width(func_width)
                    .with_max_width(func_width),
                TableColumn::new("Description"),
                TableColumn::new("Example"),
            ];
            let mut table = Table::new().with_columns(columns);
            configure_shared_table(&mut table);

            for func in functions {
                table.add_row(vec![
                    inline_code_text(func.signature, &term).into(),
                    inline_code_text(func.description, &term).into(),
                    format_example(func.example.as_ref(), &term).into(),
                ]);
            }

            let out = render_table_within_contract(&table, &term);
            let max = out.lines().map(|l| visible_width(l) as usize).max().unwrap_or(0);
            assert!(
                max <= 140,
                "expression table for '{category}' exceeded 140ch (max={max})"
            );
        }

        // Build a side-effect table similar to the real report.
        let effect_groups = group_effect_descriptors();
        let all_effect_sigs: Vec<String> = effect_groups
            .iter()
            .flat_map(|(_, effects)| effects.iter().map(|e| e.signature.to_string()))
            .collect();
        let effect_sig_refs: Vec<&str> = all_effect_sigs.iter().map(|s| s.as_str()).collect();
        let cap_width = function_first_column_width("Capability", effect_sig_refs.into_iter());

        for (category, effects) in &effect_groups {
            let out = render_table_resilient(&term, |layout| {
                let mut capability = report_column("Capability");
                if layout == TableLayout::Pinned {
                    capability = capability.with_min_width(cap_width).with_max_width(cap_width);
                }
                let mut table = Table::new().with_columns(vec![
                    capability,
                    report_column("Description"),
                    report_column("Example"),
                    report_column("Safety"),
                ]);
                configure_shared_table(&mut table);

                for effect in effects {
                    table.add_row(vec![
                        inline_code_text(effect.signature, &term).into(),
                        inline_code_text(effect.description, &term).into(),
                        format_effect_example(effect.example.as_ref(), &term).into(),
                        format_safety(effect.safety, &term).into(),
                    ]);
                }
                table
            });
            let max = out.lines().map(|l| visible_width(l) as usize).max().unwrap_or(0);
            assert!(
                max <= 140,
                "side-effect table for '{category}' exceeded 140ch (max={max})"
            );
        }
    }
}
