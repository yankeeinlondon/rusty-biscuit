use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::layout::Alignment;
use clap::Args;
use color_eyre::eyre::Result;
use darkmatter::markdown::compose::ComposeContext;

use crate::log;

const CONTEXT_VARIABLES_MD: &str =
    include_str!("../../../../darkmatter/docs/topics/context-variables.md");
const EXPRESSIONS_MD: &str =
    include_str!("../../../../darkmatter/docs/topics/darkmatter-expressions.md");

/// Arguments for the `claudine context` subcommand.
#[derive(Debug, Args)]
pub struct ContextArgs {
    /// Show live values for each context variable.
    #[arg(long)]
    pub values: bool,

    /// Show the expression engine's operations and functions.
    #[arg(long)]
    pub expressions: bool,

    /// Show the available safe side effects that Claudine provides.
    #[arg(long)]
    pub side_effects: bool,
}

/// A single context variable extracted from the documentation.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextVariable {
    /// The property name (e.g., `today`).
    pub property: String,
    /// The type name (e.g., `String`).
    pub type_name: String,
    /// The description.
    pub description: String,
}

/// A section of context variables, grouped by heading and optional subsection.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextSection {
    /// The H3 section heading.
    pub heading: String,
    /// The H4 subsection heading, if any.
    pub subsection: Option<String>,
    /// The variables in this section/subsection.
    pub variables: Vec<ContextVariable>,
}

/// Parse the embedded context-variables.md into sections.
pub fn parse_context_variables() -> Vec<ContextSection> {
    parse_context_variables_content(CONTEXT_VARIABLES_MD)
}

/// Parse context variables from markdown content.
fn parse_context_variables_content(content: &str) -> Vec<ContextSection> {
    let mut sections: Vec<ContextSection> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_subsection: Option<String> = None;
    let mut in_table = false;
    let mut pending_variables: Vec<ContextVariable> = Vec::new();
    let mut current_table_header: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // H3 heading
        if let Some(heading) = trimmed.strip_prefix("### ") {
            // Flush any pending variables from previous section/subsection
            flush_pending(
                &mut sections,
                &current_section,
                &mut current_subsection,
                &mut pending_variables,
            );
            current_section = Some(heading.trim().to_string());
            current_subsection = None;
            in_table = false;
            current_table_header = None;
            continue;
        }

        // H4 heading
        if let Some(subheading) = trimmed.strip_prefix("#### ") {
            flush_pending(
                &mut sections,
                &current_section,
                &mut current_subsection,
                &mut pending_variables,
            );
            current_subsection = Some(subheading.trim().to_string());
            in_table = false;
            current_table_header = None;
            continue;
        }

        // Table header row (appears before the separator)
        if trimmed.starts_with('|') && !in_table && !trimmed.starts_with("|---") {
            let cells = split_table_cells(trimmed);
            // Skip the first empty cell if line starts with |
            let first_cell = if cells.first().map(|s| s.trim().is_empty()).unwrap_or(false) {
                cells.get(1).map(|s| s.trim().to_string())
            } else {
                cells.first().map(|s| s.trim().to_string())
            };
            current_table_header = first_cell;
            continue;
        }

        // Table separator line
        if trimmed.starts_with("|---") {
            in_table = true;
            continue;
        }

        // Table row — only collect from Variable tables
        if trimmed.starts_with('|') && in_table {
            if current_table_header.as_deref() == Some("Variable")
                && let Some(var) = parse_table_row(trimmed)
            {
                pending_variables.push(var);
            }
            continue;
        }

        // Blank line or non-table line ends table mode
        if trimmed.is_empty() {
            in_table = false;
            current_table_header = None;
        }
    }

    // Flush final pending variables
    flush_pending(
        &mut sections,
        &current_section,
        &mut current_subsection,
        &mut pending_variables,
    );

    sections
}

/// Flush pending variables into a section.
fn flush_pending(
    sections: &mut Vec<ContextSection>,
    current_section: &Option<String>,
    current_subsection: &mut Option<String>,
    pending_variables: &mut Vec<ContextVariable>,
) {
    if pending_variables.is_empty() {
        return;
    }

    if let Some(heading) = current_section.clone() {
        sections.push(ContextSection {
            heading,
            subsection: current_subsection.take(),
            variables: std::mem::take(pending_variables),
        });
    }
}

/// Parse a single table row into a ContextVariable.
fn parse_table_row(line: &str) -> Option<ContextVariable> {
    let cells = split_table_cells(line);

    // Skip the first empty cell if line starts with |
    let cells: Vec<&str> = if line.trim_start().starts_with('|')
        && cells.first().map(|s| s.trim().is_empty()).unwrap_or(false)
    {
        cells.iter().skip(1).map(|s| s.as_str()).collect()
    } else {
        cells.iter().map(|s| s.as_str()).collect()
    };

    // We need at least 3 cells: property, type, description
    if cells.len() < 3 {
        return None;
    }

    let property = cells[0].trim().replace("\\|", "|");
    let type_name = cells[1].trim().replace("\\|", "|");
    let description = cells[2].trim().replace("\\|", "|");

    // Skip header rows (when property is "Variable" or empty)
    if property.is_empty() || property == "Variable" {
        return None;
    }

    Some(ContextVariable {
        property,
        type_name,
        description,
    })
}

/// Split a markdown table row into cells, respecting backtick-quoted pipes.
fn split_table_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_backticks = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '`' {
            in_backticks = !in_backticks;
            current.push(ch);
            i += 1;
            continue;
        }

        if ch == '|' && !in_backticks {
            cells.push(current.clone());
            current.clear();
            i += 1;
            continue;
        }

        current.push(ch);
        i += 1;
    }

    cells.push(current);
    cells
}

/// Strip backticks from a property name for value lookup.
fn strip_backticks(s: &str) -> String {
    s.trim_matches('`').to_string()
}

/// Format a property name for display with the required `ctx.` prefix.
fn display_property(property: &str) -> String {
    let key = strip_backticks(property);
    format!("ctx.{key}")
}

/// Determine the string type variant (csv, list, new-line) from the property key and description.
fn string_type_variant(key: &str, description: &str) -> Option<&'static str> {
    if key.ends_with("_list") {
        return Some("list");
    }
    if key == "dirty_files" {
        return Some("new-line");
    }
    if description.contains("Comma-separated") || description.contains("comma-separated") {
        return Some("csv");
    }
    None
}

/// Convert a raw type string to display text, applying bracket-to-postfix and string variants.
fn type_to_display_text(type_name: &str, key: &str, description: &str) -> String {
    let inner = strip_backticks(type_name);
    let variant = string_type_variant(key, description);

    inner
        .split(" | ")
        .map(|token| {
            let token = token.trim();
            // Convert [X] to X[]
            let display_token =
                if let Some(inner) = token.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    format!("{inner}[]")
                } else {
                    token.to_string()
                };

            // Apply variant to String or String[]
            if let Some(variant) = variant
                && (display_token == "String" || display_token == "String[]")
            {
                format!("String({variant})")
            } else {
                display_token
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Map a type token to its display color.
fn type_token_color(token: &str) -> Option<&'static str> {
    if token.starts_with("String") {
        Some("blue")
    } else if token == "Number" || token == "Number[]" {
        Some("green")
    } else if token == "File" || token == "File[]" {
        Some("violet")
    } else if token == "null" {
        Some("grey")
    } else if token == "Bool" || token == "Bool[]" {
        Some("orange")
    } else {
        None
    }
}

/// Render each type token in its designated color.
fn format_type_for_display(
    type_name: &str,
    key: &str,
    description: &str,
    term: &biscuit_terminal::terminal::Terminal,
) -> String {
    let display = type_to_display_text(type_name, key, description);
    let colored = display
        .split(" | ")
        .map(|token| {
            let token = token.trim();
            match type_token_color(token) {
                Some(color) => format!("<{color}>{token}</{color}>"),
                None => token.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    Prose::new(colored).render(term)
}

/// Replace backticks with single quotes in description text.
fn format_description_for_display(description: &str) -> String {
    description.replace('`', "'")
}

/// Compute the visible width of the longest display property across all sections.
fn max_property_width(sections: &[ContextSection]) -> usize {
    sections
        .iter()
        .flat_map(|s| s.variables.iter())
        .map(|v| visible_width(&display_property(&v.property)) as usize)
        .max()
        .unwrap_or(0)
}

/// Render the default context report (Property, Type, Description).
fn render_default_report(sections: &[ContextSection]) {
    let term = log::terminal();
    let property_width = max_property_width(sections);

    for section in sections {
        if let Some(ref subsection) = section.subsection {
            let heading = Prose::new(format!(
                "<blue><b>{}</b></blue> — <b>{subsection}</b>",
                section.heading
            ));
            log::data(&heading.render(&term));
        } else {
            let heading = Prose::new(format!("<blue><b>{}</b></blue>", section.heading));
            log::data(&heading.render(&term));
        }

        let columns = vec![
            TableColumn::new("Property").with_min_width(property_width),
            TableColumn::new("Type")
                .with_alignment(Alignment::Center)
                .drop_when_space_is_limited::<String>(None),
            TableColumn::new("Description"),
        ];
        let mut table = Table::new().with_columns(columns);
        table.layout_mut().margin = biscuit_terminal::utils::layout::Margin::x(
            biscuit_terminal::utils::layout::Length::ch(1),
        );

        for var in &section.variables {
            let key = strip_backticks(&var.property);
            let row: Vec<TableCellContent> = vec![
                display_property(&var.property).into(),
                format_type_for_display(&var.type_name, &key, &var.description, &term).into(),
                format_description_for_display(&var.description).into(),
            ];
            table.add_row(row);
        }

        log::data(&table.render(&term));
        log::data("");
    }
}

/// Render the values report (Property, Type, Value).
fn render_values_report(sections: &[ContextSection]) {
    let term = log::terminal();
    let ctx = ComposeContext::capture();
    let values = ctx.values();
    let property_width = max_property_width(sections);

    for section in sections {
        if let Some(ref subsection) = section.subsection {
            let heading = Prose::new(format!(
                "<blue><b>{}</b></blue> — <b>{subsection}</b>",
                section.heading
            ));
            log::data(&heading.render(&term));
        } else {
            let heading = Prose::new(format!("<blue><b>{}</b></blue>", section.heading));
            log::data(&heading.render(&term));
        }

        let columns = vec![
            TableColumn::new("Property").with_min_width(property_width),
            TableColumn::new("Type")
                .with_alignment(Alignment::Center)
                .drop_when_space_is_limited::<String>(None),
            TableColumn::new("Value"),
        ];
        let mut table = Table::new().with_columns(columns);
        table.layout_mut().margin = biscuit_terminal::utils::layout::Margin::x(
            biscuit_terminal::utils::layout::Length::ch(1),
        );

        for var in &section.variables {
            let key = strip_backticks(&var.property);
            let value_str = match values.get(&key) {
                Some(v) if v.is_null() => Prose::new("<dim>null</dim>").render(&term),
                Some(v) => match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Array(arr) => {
                        let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                        items.join(", ")
                    }
                    serde_json::Value::Object(_) => v.to_string(),
                    serde_json::Value::Null => Prose::new("<dim>null</dim>").render(&term),
                },
                None => Prose::new("<dim>null</dim>").render(&term),
            };

            let row: Vec<TableCellContent> = vec![
                display_property(&var.property).into(),
                format_type_for_display(&var.type_name, &key, &var.description, &term).into(),
                value_str.into(),
            ];
            table.add_row(row);
        }

        log::data(&table.render(&term));
        log::data("");
    }
}

/// Render the side effects report (placeholder).
fn render_side_effects_report() {
    log::data("not implemented yet");
}

/// Render the footer messages to stderr.
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
        "<dim><i>use <blue>--side-effects</blue> to see the safe side effects that <b>Claudine</b> provides without need for being white listed</i></dim>",
    )
    .state(StatusState::Info);
    log::message(&msg2.render(&term));
}

#[derive(Debug, Clone, PartialEq)]
struct ExprSection {
    heading: String,
    level: u8,
    blocks: Vec<ExprBlock>,
}

#[derive(Debug, Clone, PartialEq)]
enum ExprBlock {
    Paragraph(String),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    OrderedList(Vec<String>),
    UnorderedList(Vec<String>),
    CodeBlock(String),
}

fn parse_expressions_doc() -> Vec<ExprSection> {
    parse_expressions_content(EXPRESSIONS_MD)
}

fn parse_expressions_content(content: &str) -> Vec<ExprSection> {
    let mut sections: Vec<ExprSection> = Vec::new();
    let mut current_heading: Option<(String, u8)> = None;
    let mut current_blocks: Vec<ExprBlock> = Vec::new();

    let mut in_table = false;
    let mut table_lines: Vec<String> = Vec::new();
    // The Markdown table header row appears BEFORE the `|---` separator. We
    // stash a candidate header line here so the separator can promote it
    // into the first entry of `table_lines`; otherwise `build_expr_table`
    // mistakes the first data row for the headers.
    let mut pending_table_header: Option<String> = None;
    let mut in_code_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();
    let mut in_ordered_list = false;
    let mut in_unordered_list = false;
    let mut list_items: Vec<String> = Vec::new();
    let mut paragraph = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Headings
        if let Some(h2) = trimmed.strip_prefix("## ") {
            // A non-`|---` line after a pending header means the candidate
            // was not actually a table; drop it.
            pending_table_header = None;
            flush_expr_current(&mut sections, &mut current_heading, &mut current_blocks);
            current_heading = Some((h2.trim().to_string(), 2));
            continue;
        }
        if let Some(h3) = trimmed.strip_prefix("### ") {
            pending_table_header = None;
            flush_expr_current(&mut sections, &mut current_heading, &mut current_blocks);
            current_heading = Some((h3.trim().to_string(), 3));
            continue;
        }

        // Code blocks
        if trimmed.starts_with("```") {
            pending_table_header = None;
            if in_code_block {
                in_code_block = false;
                current_blocks.push(ExprBlock::CodeBlock(code_block_lines.join("\n")));
                code_block_lines.clear();
            } else {
                flush_expr_paragraph(&mut current_blocks, &mut paragraph);
                flush_expr_list(
                    &mut current_blocks,
                    &mut list_items,
                    &mut in_ordered_list,
                    &mut in_unordered_list,
                );
                in_code_block = true;
            }
            continue;
        }
        if in_code_block {
            code_block_lines.push(line.to_string());
            continue;
        }

        // Tables
        if trimmed.starts_with("|---") {
            flush_expr_paragraph(&mut current_blocks, &mut paragraph);
            flush_expr_list(
                &mut current_blocks,
                &mut list_items,
                &mut in_ordered_list,
                &mut in_unordered_list,
            );
            // Promote the pending header line into the table so
            // `build_expr_table` sees `lines[0]` as the actual headers.
            if let Some(header) = pending_table_header.take() {
                table_lines.push(header);
            }
            in_table = true;
            continue;
        }
        if in_table && trimmed.starts_with('|') {
            table_lines.push(trimmed.to_string());
            continue;
        }
        if !in_table && trimmed.starts_with('|') {
            // Candidate table header — keep it aside until we see the
            // `|---` separator on the next line.
            pending_table_header = Some(trimmed.to_string());
            continue;
        }
        if in_table && !trimmed.starts_with('|') {
            if let Some((headers, rows)) = build_expr_table(&table_lines) {
                current_blocks.push(ExprBlock::Table { headers, rows });
            }
            table_lines.clear();
            in_table = false;
        }
        // If we had a pending header line but the next line is not a
        // separator, it was not a table after all.
        pending_table_header = None;

        // List items
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            flush_expr_paragraph(&mut current_blocks, &mut paragraph);
            if in_ordered_list {
                flush_expr_list(
                    &mut current_blocks,
                    &mut list_items,
                    &mut in_ordered_list,
                    &mut in_unordered_list,
                );
            }
            in_unordered_list = true;
            list_items.push(trimmed[2..].to_string());
            continue;
        }
        if let Some((num, rest)) = trimmed.split_once(". ")
            && num.parse::<u32>().is_ok()
            && !trimmed.starts_with('|')
            && !in_table
        {
            flush_expr_paragraph(&mut current_blocks, &mut paragraph);
            if in_unordered_list {
                flush_expr_list(
                    &mut current_blocks,
                    &mut list_items,
                    &mut in_ordered_list,
                    &mut in_unordered_list,
                );
            }
            in_ordered_list = true;
            list_items.push(rest.to_string());
            continue;
        }
        // Continuation of list item
        if (in_ordered_list || in_unordered_list)
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with('|')
            && !trimmed.starts_with("```")
            && !trimmed.starts_with("- ")
            && !trimmed.starts_with("* ")
        {
            if let Some(last) = list_items.last_mut() {
                if !last.ends_with(' ') {
                    last.push(' ');
                }
                last.push_str(trimmed);
            }
            continue;
        }

        // Blank line
        if trimmed.is_empty() {
            flush_expr_paragraph(&mut current_blocks, &mut paragraph);
            flush_expr_list(
                &mut current_blocks,
                &mut list_items,
                &mut in_ordered_list,
                &mut in_unordered_list,
            );
            if in_table {
                if let Some((headers, rows)) = build_expr_table(&table_lines) {
                    current_blocks.push(ExprBlock::Table { headers, rows });
                }
                table_lines.clear();
                in_table = false;
            }
            continue;
        }

        // Paragraph text
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(trimmed);
    }

    // Flush remaining
    flush_expr_paragraph(&mut current_blocks, &mut paragraph);
    flush_expr_list(
        &mut current_blocks,
        &mut list_items,
        &mut in_ordered_list,
        &mut in_unordered_list,
    );
    if in_table
        && !table_lines.is_empty()
        && let Some((headers, rows)) = build_expr_table(&table_lines)
    {
        current_blocks.push(ExprBlock::Table { headers, rows });
    }
    flush_expr_current(&mut sections, &mut current_heading, &mut current_blocks);

    sections
}

fn flush_expr_paragraph(blocks: &mut Vec<ExprBlock>, paragraph: &mut String) {
    if !paragraph.is_empty() {
        blocks.push(ExprBlock::Paragraph(std::mem::take(paragraph)));
    }
}

fn flush_expr_list(
    blocks: &mut Vec<ExprBlock>,
    items: &mut Vec<String>,
    ordered: &mut bool,
    unordered: &mut bool,
) {
    if !items.is_empty() {
        if *ordered {
            blocks.push(ExprBlock::OrderedList(std::mem::take(items)));
        } else {
            blocks.push(ExprBlock::UnorderedList(std::mem::take(items)));
        }
    }
    *ordered = false;
    *unordered = false;
}

fn flush_expr_current(
    sections: &mut Vec<ExprSection>,
    heading: &mut Option<(String, u8)>,
    blocks: &mut Vec<ExprBlock>,
) {
    if let Some((h, level)) = heading.take() {
        sections.push(ExprSection {
            heading: h,
            level,
            blocks: std::mem::take(blocks),
        });
    }
}

fn build_expr_table(lines: &[String]) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    if lines.is_empty() {
        return None;
    }
    let headers: Vec<String> = split_table_cells(&lines[0])
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if headers.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    for line in lines.iter().skip(1) {
        let cells: Vec<String> = split_table_cells(line)
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    Some((headers, rows))
}

/// Render a sequence of expression doc blocks to the terminal.
fn render_expr_blocks(term: &biscuit_terminal::terminal::Terminal, blocks: &[ExprBlock]) {
    for block in blocks {
        match block {
            ExprBlock::Paragraph(text) => {
                let prose = Prose::new(text.clone());
                log::data(&prose.render(term));
            }
            ExprBlock::Table { headers, rows } => {
                let columns: Vec<TableColumn> = headers
                    .iter()
                    .map(|h| TableColumn::new(h.as_str()))
                    .collect();
                let mut table = Table::new().with_columns(columns);
                table.layout_mut().margin = biscuit_terminal::utils::layout::Margin::x(
                    biscuit_terminal::utils::layout::Length::ch(1),
                );
                for row in rows {
                    let cells: Vec<TableCellContent> =
                        row.iter().map(|c| c.clone().into()).collect();
                    table.add_row(cells);
                }
                log::data(&table.render(term));
            }
            ExprBlock::UnorderedList(items) => {
                for item in items {
                    let prose = Prose::new(format!("• {item}"));
                    log::data(&prose.render(term));
                }
            }
            ExprBlock::OrderedList(items) => {
                for (i, item) in items.iter().enumerate() {
                    let prose = Prose::new(format!("{}. {item}", i + 1));
                    log::data(&prose.render(term));
                }
            }
            ExprBlock::CodeBlock(code) => {
                for line in code.lines() {
                    let prose = Prose::new(format!("<dim>    {line}</dim>"));
                    log::data(&prose.render(term));
                }
            }
        }
    }
}

/// Render an H2 section and any following H3 subsections from the expression doc.
fn render_section_with_subsections(
    term: &biscuit_terminal::terminal::Terminal,
    sections: &[ExprSection],
    heading: &str,
) {
    let Some(idx) = sections.iter().position(|s| s.heading == heading) else {
        return;
    };

    let section = &sections[idx];
    let heading_prose = Prose::new(format!("<blue><b>{}</b></blue>", section.heading));
    log::data(&heading_prose.render(term));
    log::data("");

    render_expr_blocks(term, &section.blocks);

    // Render H3 subsections
    let mut i = idx + 1;
    while i < sections.len() && sections[i].level == 3 {
        let sub = &sections[i];
        let sub_heading = Prose::new(format!("<b>{}</b>", sub.heading));
        log::data(&sub_heading.render(term));
        log::data("");
        render_expr_blocks(term, &sub.blocks);
        i += 1;
    }

    log::data("");
}

fn render_expressions_report() {
    let term = log::terminal();
    let sections = parse_expressions_doc();

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
    render_precedence_table(&term, &sections);

    // Truthiness
    render_truthiness_table(&term, &sections);

    // Unary operators (brief)
    render_unary_operators(&term, &sections);

    // Comparison Operators
    render_section_with_subsections(&term, &sections, "Comparison Operators");

    // Arithmetic Operators
    render_section_with_subsections(&term, &sections, "Arithmetic Operators");

    // Variable Access
    render_section_with_subsections(&term, &sections, "Variable Access");

    // Interpolation vs. Condition Mode
    render_section_with_subsections(&term, &sections, "Interpolation vs. Condition Mode");

    // Null Propagation Summary
    render_section_with_subsections(&term, &sections, "Null Propagation Summary");

    // Functions
    render_functions(&term, &sections);

    render_footer(true);
}

fn render_precedence_table(term: &biscuit_terminal::terminal::Terminal, sections: &[ExprSection]) {
    let Some(section) = sections.iter().find(|s| s.heading == "Operator Precedence") else {
        return;
    };

    let heading = Prose::new("<blue><b>Operator Precedence</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let mut items: Vec<(String, String)> = Vec::new();
    for block in &section.blocks {
        if let ExprBlock::OrderedList(list) = block {
            for item in list {
                let text = item.trim();
                // Extract bold text: **Name** — description
                if let Some(start) = text.find("**") {
                    let after_start = &text[start + 2..];
                    if let Some(end) = after_start.find("**") {
                        let op_name = &after_start[..end];
                        let rest = after_start[end + 2..].trim();
                        let desc = rest
                            .strip_prefix("—")
                            .or_else(|| rest.strip_prefix("-"))
                            .unwrap_or(rest)
                            .trim();
                        items.push((op_name.to_string(), desc.to_string()));
                    }
                }
            }
        }
    }

    if items.is_empty() {
        return;
    }

    let columns = vec![
        TableColumn::new("Precedence"),
        TableColumn::new("Operators"),
    ];
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().margin =
        biscuit_terminal::utils::layout::Margin::x(biscuit_terminal::utils::layout::Length::ch(1));

    for (i, (name, desc)) in items.iter().enumerate() {
        let row: Vec<TableCellContent> = vec![
            format!("{}", i + 1).into(),
            format!("{name} — {desc}").into(),
        ];
        table.add_row(row);
    }

    log::data(&table.render(term));
    log::data("");
}

fn render_truthiness_table(term: &biscuit_terminal::terminal::Terminal, sections: &[ExprSection]) {
    let Some(section) = sections.iter().find(|s| s.heading == "Truthiness") else {
        return;
    };

    let heading = Prose::new("<blue><b>Truthiness</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    for block in &section.blocks {
        if let ExprBlock::Table { headers, rows } = block {
            let columns: Vec<TableColumn> = headers
                .iter()
                .map(|h| TableColumn::new(h.as_str()))
                .collect();
            let mut table = Table::new().with_columns(columns);
            table.layout_mut().margin = biscuit_terminal::utils::layout::Margin::x(
                biscuit_terminal::utils::layout::Length::ch(1),
            );

            for row in rows {
                let cells: Vec<TableCellContent> = row.iter().map(|c| c.clone().into()).collect();
                table.add_row(cells);
            }

            log::data(&table.render(term));
            log::data("");
        }
    }
}

fn render_unary_operators(term: &biscuit_terminal::terminal::Terminal, sections: &[ExprSection]) {
    let Some(section) = sections.iter().find(|s| s.heading == "Unary Operators") else {
        return;
    };

    let heading = Prose::new("<blue><b>Unary Operators</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let columns = vec![
        TableColumn::new("Operator"),
        TableColumn::new("Description"),
    ];
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().margin =
        biscuit_terminal::utils::layout::Margin::x(biscuit_terminal::utils::layout::Length::ch(1));

    for block in &section.blocks {
        if let ExprBlock::UnorderedList(list) = block {
            for item in list {
                if let Some((sig, desc)) = parse_function_item(item) {
                    let row: Vec<TableCellContent> = vec![format!("`{sig}`").into(), desc.into()];
                    table.add_row(row);
                }
            }
        }
    }

    log::data(&table.render(term));
    log::data("");
}

fn render_functions(term: &biscuit_terminal::terminal::Terminal, sections: &[ExprSection]) {
    let Some(func_idx) = sections.iter().position(|s| s.heading == "Functions") else {
        return;
    };

    let heading = Prose::new("<blue><b>Functions</b></blue>");
    log::data(&heading.render(term));
    log::data("");

    let mut i = func_idx + 1;
    while i < sections.len() && sections[i].level == 3 {
        let subsection = &sections[i];

        let sub_heading = Prose::new(format!("<b>{}</b>", subsection.heading));
        log::data(&sub_heading.render(term));

        let columns = vec![
            TableColumn::new("Function"),
            TableColumn::new("Description"),
        ];
        let mut table = Table::new().with_columns(columns);
        table.layout_mut().margin = biscuit_terminal::utils::layout::Margin::x(
            biscuit_terminal::utils::layout::Length::ch(1),
        );

        for block in &subsection.blocks {
            if let ExprBlock::UnorderedList(list) = block {
                for item in list {
                    if let Some((sig, desc)) = parse_function_item(item) {
                        let row: Vec<TableCellContent> =
                            vec![format!("`{sig}`").into(), desc.into()];
                        table.add_row(row);
                    } else if !item.trim().is_empty() {
                        let row: Vec<TableCellContent> = vec![item.trim().into(), "".into()];
                        table.add_row(row);
                    }
                }
            }
        }

        log::data(&table.render(term));
        log::data("");

        i += 1;
    }
}

fn parse_function_item(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    let sep = " — ";
    let sep_pos = text.find(sep)?;
    let signature = text[..sep_pos].trim().trim_matches('`').to_string();
    let description = text[sep_pos + sep.len()..].trim().to_string();
    if signature.is_empty() {
        return None;
    }
    Some((signature, description))
}

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

    let sections = parse_context_variables();

    if args.values {
        render_values_report(&sections);
        render_footer(false);
    } else {
        render_default_report(&sections);
        render_footer(true);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MD: &str = r#"
# Context Variables

## Overcoming Conflicts

### Section A

#### Sub A1

| Variable | Type | Description |
|----------|------|-------------|
| `var1` | `String` | First variable |
| `var2` | `Number` | Second variable |

#### Sub A2

| Variable | Type | Description |
|----------|------|-------------|
| `var3` | `Bool` | Third variable |

### Section B

| Variable | Type | Description |
|----------|------|-------------|
| `var4` | `String | null` | Fourth variable |
"#;

    #[test]
    fn test_parse_context_variables_content() {
        let sections = parse_context_variables_content(TEST_MD);

        assert_eq!(
            sections.len(),
            3,
            "expected 3 sections/subsections with variables"
        );

        // Section A — Sub A1
        assert_eq!(sections[0].heading, "Section A");
        assert_eq!(sections[0].subsection, Some("Sub A1".to_string()));
        assert_eq!(sections[0].variables.len(), 2);
        assert_eq!(sections[0].variables[0].property, "`var1`");
        assert_eq!(sections[0].variables[0].type_name, "`String`");
        assert_eq!(sections[0].variables[0].description, "First variable");
        assert_eq!(sections[0].variables[1].property, "`var2`");
        assert_eq!(sections[0].variables[1].type_name, "`Number`");

        // Section A — Sub A2
        assert_eq!(sections[1].heading, "Section A");
        assert_eq!(sections[1].subsection, Some("Sub A2".to_string()));
        assert_eq!(sections[1].variables.len(), 1);
        assert_eq!(sections[1].variables[0].property, "`var3`");

        // Section B (no subsection)
        assert_eq!(sections[2].heading, "Section B");
        assert_eq!(sections[2].subsection, None);
        assert_eq!(sections[2].variables.len(), 1);
        assert_eq!(sections[2].variables[0].property, "`var4`");
        assert_eq!(sections[2].variables[0].type_name, "`String | null`");
        assert_eq!(sections[2].variables[0].description, "Fourth variable");
    }

    #[test]
    fn test_parse_real_context_variables_file() {
        let sections = parse_context_variables();
        assert!(!sections.is_empty(), "should discover at least one section");

        // We expect sections like "Date and Time Information", "Filesystem and Git", etc.
        let has_date_section = sections.iter().any(|s| s.heading.contains("Date and Time"));
        assert!(has_date_section, "should discover Date and Time section");

        let total_vars: usize = sections.iter().map(|s| s.variables.len()).sum();
        assert!(
            total_vars > 10,
            "should discover many variables, got {total_vars}"
        );
    }

    const TEST_EXPR_MD: &str = r#"
# Darkmatter Expressions

## Operator Precedence

1. **Primary** — literals
2. **Unary** — `!`, `-`

## Truthiness

| Value | Falsy |
|-------|-------|
| `null` | yes |
| `false` | no |

## Unary Operators

- `!x` — boolean negation
- `-x` — numeric negation

## Functions

### Logical Helpers

- `and(a, b)` — all arguments truthy
- `or(a, b)` — any argument truthy

### Math

- `min(a, b)` — minimum
"#;

    #[test]
    fn test_parse_expressions_content() {
        let sections = parse_expressions_content(TEST_EXPR_MD);

        assert!(!sections.is_empty(), "should discover sections");

        let precedence = sections.iter().find(|s| s.heading == "Operator Precedence");
        assert!(
            precedence.is_some(),
            "should find Operator Precedence section"
        );

        let truthiness = sections.iter().find(|s| s.heading == "Truthiness");
        assert!(truthiness.is_some(), "should find Truthiness section");

        let unary = sections.iter().find(|s| s.heading == "Unary Operators");
        assert!(unary.is_some(), "should find Unary Operators section");

        let functions = sections.iter().find(|s| s.heading == "Functions");
        assert!(functions.is_some(), "should find Functions section");

        let logical = sections.iter().find(|s| s.heading == "Logical Helpers");
        assert!(logical.is_some(), "should find Logical Helpers subsection");

        let math = sections.iter().find(|s| s.heading == "Math");
        assert!(math.is_some(), "should find Math subsection");
    }

    /// Regression: review-2 finding noted that the table header row was lost
    /// because the parser only began collecting after the `|---` separator.
    /// `build_expr_table` then treated the first data row as headers. This
    /// asserts the actual documented headers survive parsing and the first
    /// data row remains a body row.
    #[test]
    fn expressions_table_keeps_header_row_and_first_data_row() {
        let sections = parse_expressions_content(TEST_EXPR_MD);

        let truthiness = sections
            .iter()
            .find(|s| s.heading == "Truthiness")
            .expect("Truthiness section must be present");

        let table = truthiness
            .blocks
            .iter()
            .find_map(|b| match b {
                ExprBlock::Table { headers, rows } => Some((headers, rows)),
                _ => None,
            })
            .expect("Truthiness section must include a Table block");

        let (headers, rows) = table;
        assert_eq!(
            headers,
            &vec!["Value".to_string(), "Falsy".to_string()],
            "headers must come from the row above `|---`, not the first data row",
        );
        assert_eq!(rows.len(), 2, "both data rows must survive");
        assert_eq!(
            rows[0],
            vec!["`null`".to_string(), "yes".to_string()],
            "first data row must be preserved as a body row",
        );
        assert_eq!(rows[1], vec!["`false`".to_string(), "no".to_string()]);
    }

    /// Regression: the real `darkmatter-expressions.md` Truthiness table
    /// previously parsed with `null`/`yes` as headers. Make sure the
    /// production document parses with the documented `Value`/`Falsy`
    /// headers.
    #[test]
    fn real_expressions_truthiness_table_has_correct_headers() {
        let sections = parse_expressions_doc();
        let truthiness = sections
            .iter()
            .find(|s| s.heading == "Truthiness")
            .expect("Truthiness section must exist in real document");

        let (headers, rows) = truthiness
            .blocks
            .iter()
            .find_map(|b| match b {
                ExprBlock::Table { headers, rows } => Some((headers, rows)),
                _ => None,
            })
            .expect("Truthiness section in real doc must have a Table");

        assert!(
            headers.iter().any(|h| h.eq_ignore_ascii_case("Value")),
            "Truthiness headers must include `Value`, got {headers:?}",
        );
        assert!(
            headers.iter().any(|h| h.eq_ignore_ascii_case("Falsy")),
            "Truthiness headers must include `Falsy`, got {headers:?}",
        );
        assert!(!rows.is_empty(), "Truthiness table must have rows");
    }

    #[test]
    fn test_parse_real_expressions_file() {
        let sections = parse_expressions_doc();
        assert!(!sections.is_empty(), "should discover at least one section");

        let has_precedence = sections.iter().any(|s| s.heading == "Operator Precedence");
        assert!(
            has_precedence,
            "should discover Operator Precedence section"
        );

        let has_truthiness = sections.iter().any(|s| s.heading == "Truthiness");
        assert!(has_truthiness, "should discover Truthiness section");

        let has_functions = sections.iter().any(|s| s.heading == "Functions");
        assert!(has_functions, "should discover Functions section");

        let has_function_sub = sections.iter().any(|s| s.heading == "Logical Helpers");
        assert!(has_function_sub, "should discover a function subsection");
    }

    /// Regression: when `plan_widths` succeeds by dropping a
    /// `drop_when_space_is_limited` column, the live `render` path must do
    /// the same. The context command's three-column "Property / Type /
    /// Description" layout was rendering the "Table could not be rendered"
    /// error string inline at narrow widths because the render-tree path
    /// disagreed with `plan_widths` about whether the Type column should
    /// be dropped.
    #[test]
    fn render_drops_optional_column_consistently_with_plan_widths() {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;

        let columns = vec![
            TableColumn::new("Property").with_min_width(41),
            TableColumn::new("Type")
                .with_min_width(14)
                .with_alignment(Alignment::Center)
                .drop_when_space_is_limited::<String>(None),
            TableColumn::new("Description"),
        ];
        let mut table = Table::new().with_columns(columns);
        table.layout_mut().margin = biscuit_terminal::utils::layout::Margin::x(
            biscuit_terminal::utils::layout::Length::ch(1),
        );

        table.add_row(vec![
            "ctx.docs_readme".into(),
            "String(csv)".into(),
            "Comma-separated README paths, scope-filtered".into(),
        ]);
        table.add_row(vec![
            "ctx.docs_blast_radius".into(),
            "String(csv)".into(),
            "Comma-separated docs with 'blast_radius' frontmatter, scope-filtered".into(),
        ]);

        // `plan_widths` resolves the layout by dropping the optional Type
        // column when 78 columns minus the x:1 margin (= 76) cannot fit all
        // three columns with their minimum widths.
        let plan = table
            .plan_widths(76)
            .expect("plan_widths should drop the optional Type column and succeed");
        assert_eq!(
            plan.dropped_column_indices,
            vec![1],
            "planner must drop the Type column to fit",
        );
        assert_eq!(
            plan.visible_column_indices,
            vec![0, 2],
            "Property and Description must remain visible",
        );

        // The live render path must agree — no inline "could not be
        // rendered" error text in the user-visible output.
        let term = Terminal::new_optimistic(78);
        let output = table.render(&term);
        assert!(
            !output.contains("Table could not be rendered"),
            "render must drop the optional column instead of emitting the \
             width-error string inline; output was:\n{output}",
        );
        assert!(
            output.contains("Property") && output.contains("Description"),
            "render must show Property and Description headers; output was:\n{output}",
        );
        assert!(
            !output.contains("Type"),
            "render must omit the dropped Type column header; output was:\n{output}",
        );
    }
}
