//! The expression-engine report and its shared sub-table renderer.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::compose::expression::semantics::{
    arithmetic_operator_descriptors, comparison_operator_descriptors, mode_descriptors,
    null_propagation_descriptors, operator_descriptors, truthiness_descriptors,
    unary_operator_descriptors, variable_access_descriptors,
};

use crate::commands::context_render::{
    configure_shared_table, example_column_floor, function_first_column_width, inline_code_text,
    render_table_resilient, render_unordered_list, report_column, TableLayout,
    EXAMPLE_COLUMN_MIN_WIDTH,
};
use crate::log;

use super::format::format_example;

/// Whether the metadata reports should render their optional `Example` column at
/// `term`'s width.
///
/// Below the threshold the four-column layout cannot satisfy the table planner
/// at the minimum supported width, so the column is dropped (the documented
/// "where layout permits" contract). Shared by every `render_expressions_*`
/// sub-table and the side-effect report.
pub(super) fn show_examples(term: &Terminal) -> bool {
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
pub(super) fn render_expression_table(
    term: &Terminal,
    first_header: impl Into<String>,
    first_width: usize,
    middle_header: impl Into<String>,
    rows: Vec<(TableCellContent, TableCellContent, TableCellContent)>,
) -> String {
    let first_header = first_header.into();
    let middle_header = middle_header.into();
    let with_examples = show_examples(term);

    // Floor the `Example` column at its own (capped) intrinsic width so a long
    // `Description` line cannot starve it under the planner's greedy surplus
    // distribution. See `example_column_floor`.
    let example_strings: Vec<String> =
        rows.iter().map(|(_, _, example)| example.to_string()).collect();
    let example_width = example_column_floor(example_strings.iter().map(String::as_str));

    render_table_resilient(term, |layout| {
        let mut first = report_column(first_header.clone());
        if layout == TableLayout::Pinned {
            first = first.with_min_width(first_width).with_max_width(first_width);
        }
        let mut columns = vec![first, report_column(middle_header.clone())];
        if with_examples {
            let mut example = report_column("Example");
            if layout == TableLayout::Pinned {
                example = example.with_min_width(example_width);
            }
            columns.push(example);
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

pub(super) fn render_expressions_precedence(term: &Terminal) {
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

pub(super) fn render_expressions_truthiness(term: &Terminal) {
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

pub(super) fn render_expressions_unary(term: &Terminal) {
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

pub(super) fn render_expressions_comparison(term: &Terminal) {
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

pub(super) fn render_expressions_arithmetic(term: &Terminal) {
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

pub(super) fn render_expressions_variable_access(term: &Terminal) {
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

pub(super) fn render_expressions_modes(term: &Terminal) {
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
        "`{{ a && b }}` is logical AND — `&&` is identical in both modes",
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

pub(super) fn render_expressions_null_propagation(term: &Terminal) {
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

pub(super) fn render_expressions_functions(term: &Terminal) {
    let heading = Prose::new("<blue><b>Functions</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let groups = super::group_expression_descriptors();

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
