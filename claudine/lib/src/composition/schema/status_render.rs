//! Shared terminal projection for schema status reports.

use biscuit_terminal::components::prose::Prose;

use super::{PropertyState, PropertyStatus, SchemaStatusReport};

/// Build the reusable [`Prose`] representation of a schema status report.
pub fn schema_status_report_prose(report: &SchemaStatusReport) -> Prose {
    let path_display = biscuit_file::to_portable_string(&report.source_path);
    let path_escaped = escape_schema_prose(&path_display);
    let absolute = report.source_path.canonicalize().ok().or_else(|| {
        report.source_path.is_absolute().then(|| report.source_path.clone())
    });
    let path_reference = absolute
        .and_then(|path| url::Url::from_file_path(path).ok())
        .map_or_else(
            || path_escaped.clone(),
            |href| format!("[{path_escaped}]({})", href.as_str()),
        );
    let mut body = format!("- The {path_reference} prompt has the following schema:");

    if report.raw_json_schema {
        body.push_str("\n  <dim><i>(raw JSON Schema — per-property metadata unavailable)</i></dim>");
        return Prose::new(body);
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

    Prose::new(body)
}

#[doc(hidden)]
pub fn render_required_line(status: &PropertyStatus) -> String {
    let name = escape_schema_prose(&status.name);
    let ty = escape_schema_prose(&status.type_label);
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
        PropertyState::Deferred => format!(
            "<blue>…</blue> <inverse>{name}</inverse>: {ty} <i><dim>- not defined yet; the run must supply it by completion</dim></i>{desc}"
        ),
    }
}

#[doc(hidden)]
pub fn render_optional_line(status: &PropertyStatus) -> String {
    let name = escape_schema_prose(&status.name);
    let ty = escape_schema_prose(&status.type_label);
    let desc = description_suffix(status.description.as_deref());
    match status.state {
        PropertyState::Valid => format!(
            "<green>✓</green> <dim><i><inverse>{name}</inverse>: {ty}</i></dim>{desc}"
        ),
        PropertyState::Missing | PropertyState::Deferred => format!(
            "<grey>⍉</grey> <dim><i><inverse>{name}</inverse>: {ty}</i></dim>{desc}"
        ),
        PropertyState::Invalid => format!(
            "<yellow>!</yellow> <dim><i><inverse>{name}</inverse>: {ty}</i></dim>{desc}"
        ),
    }
}

#[doc(hidden)]
pub fn description_suffix(description: Option<&str>) -> String {
    match description.filter(|description| !description.trim().is_empty()) {
        Some(description) => format!(
            " <i><dim>— {}</dim></i>",
            escape_schema_prose(description)
        ),
        None => String::new(),
    }
}

#[doc(hidden)]
pub fn escape_schema_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' | '<' | '>' | '{' | '"' => {
                out.push('\\');
                out.push(character);
            }
            other => out.push(other),
        }
    }
    out
}
