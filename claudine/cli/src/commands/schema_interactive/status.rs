//! Status-report rendering for the schema-required-property flow.
//!
//! Renders the per-property status table (required/optional, valid/missing/
//! invalid) to stderr. Tests should assert on the structured
//! [`SchemaStatusReport`] returned by the library rather than the rendered
//! terminal output.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::{PropertyState, PropertyStatus, SchemaStatusReport};

use crate::log;

/// Render the schema status report to stderr using `biscuit-terminal::Prose`.
///
/// The report shows every required and optional property in declaration
/// order, with state-specific glyphs:
///
/// - required + valid:   `<green>✓</green>`
/// - required + missing: `<red>⍉</red>`
/// - required + invalid: `!`
/// - optional + valid:   `<green>✓</green>` (dim)
/// - optional + missing: `<grey>⍉</grey>` (dim)
/// - optional + invalid: `<yellow>!</yellow>` (dim)
///
/// When at least one optional property has an invalid value, a trailing
/// note explains that the value will be dropped from the prompt context.
pub fn render_status_report(report: &SchemaStatusReport, term: &Terminal) {
    let path_display = report.source_path.display().to_string();
    let path_escaped = escape_prose(&path_display);
    let mut body = format!(
        "- The [{path_label}]({path_url}) prompt has the following schema:",
        path_label = path_escaped,
        path_url = path_escaped,
    );

    if report.raw_json_schema {
        body.push_str("\n  <dim><i>(raw JSON Schema — per-property metadata unavailable)</i></dim>");
        let prose = Prose::new(body);
        log::message(&prose.render(term));
        return;
    }

    for status in &report.required {
        body.push('\n');
        body.push_str(&render_required_line(status));
    }
    for status in &report.optional {
        body.push('\n');
        body.push_str(&render_optional_line(status));
    }

    if report.has_invalid_optional {
        body.push_str(
            "\n- **Note:** _optional properties with invalid values will be dropped and the \
             prompt will execute without them_",
        );
    }

    let prose = Prose::new(body);
    log::message(&prose.render(term));
}

pub(super) fn render_required_line(status: &PropertyStatus) -> String {
    let name = escape_prose(&status.name);
    let ty = escape_prose(&status.type_label);
    let desc = description_suffix(status.description.as_deref());
    match status.state {
        PropertyState::Valid => format!(
            "<green>✓</green> <inverse>{name}</inverse>: {ty} <i><dim>- was defined correctly</dim></i>{desc}"
        ),
        PropertyState::Invalid => format!(
            "! <inverse>{name}</inverse>: {ty} <i><dim>- was defined but with the wrong type</dim></i>{desc}"
        ),
        PropertyState::Missing => format!(
            "<red>⍉</red> <inverse>{name}</inverse>: {ty} <i><dim>- was not defined but is required</dim></i>{desc}"
        ),
    }
}

pub(super) fn render_optional_line(status: &PropertyStatus) -> String {
    let name = escape_prose(&status.name);
    let ty = escape_prose(&status.type_label);
    let desc = description_suffix(status.description.as_deref());
    match status.state {
        PropertyState::Valid => format!(
            "<green>✓</green> <dim><i><inverse>{name}</inverse>: {ty}</i></dim>{desc}"
        ),
        PropertyState::Missing => format!(
            "<grey>⍉</grey> <dim><i><inverse>{name}</inverse>: {ty}</i></dim>{desc}"
        ),
        PropertyState::Invalid => format!(
            "<yellow>!</yellow> <dim><i><inverse>{name}</inverse>: {ty}</i></dim>{desc}"
        ),
    }
}

pub(super) fn description_suffix(description: Option<&str>) -> String {
    match description.filter(|d| !d.trim().is_empty()) {
        Some(desc) => format!(" <i><dim>— {}</dim></i>", escape_prose(desc)),
        None => String::new(),
    }
}

pub(super) fn escape_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}
