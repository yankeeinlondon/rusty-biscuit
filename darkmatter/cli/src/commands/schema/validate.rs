//! `md schema validate` implementation.

use crate::args::SchemaValidateFormat;
use crate::commands::schema::assignment::{self, Assignment, PositionalKind};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, SchemaError, ValidationProblem, ValidationProblemKind,
};
use std::path::{Path, PathBuf};

/// Environment variable used as a fallback baseline when `--schema` is absent.
pub const BASELINE_SCHEMA_ENV: &str = "BASELINE_SCHEMA";

/// Per-file validation outcome.
enum FileOutcome {
    /// Validation completed (may or may not be valid).
    Validated {
        report_valid: bool,
        problems: Vec<ValidationProblem>,
        schema_label: Option<String>,
        no_schema: bool,
    },
    /// Frontmatter could not be parsed.
    ParseError(String),
    /// Schema could not be resolved/built for this file.
    SchemaError(Box<SchemaError>),
}

/// Run `md schema validate`.
///
/// Drives validation of each input file and exits with the appropriate code.
/// When multiple kinds of failure occur in the same invocation, the more
/// specific failure wins (parse → schema-load → validation):
///
/// - `0` — all files validated successfully
/// - `1` — one or more files failed validation
/// - `2` — schema or baseline could not be loaded (CLI baseline _or_ a
///   per-document `$schema` reference)
/// - `3` — at least one file's frontmatter could not be parsed
pub fn run_validate(
    inputs: &[String],
    schema: Option<&Path>,
    format: SchemaValidateFormat,
    quiet: bool,
    no_trigger_schemas: bool,
) -> Result<()> {
    let terminal = Terminal::default();

    let (files, assignments) = match split_inputs(inputs) {
        Ok(split) => split,
        Err(message) => {
            let line = format!("<red>Error:</red> {}", escape_prose(&message));
            eprintln!("{}", Prose::new(line).render(&terminal));
            std::process::exit(64);
        }
    };

    if files.is_empty() {
        let line = "<red>Error:</red> no Markdown files were provided".to_string();
        eprintln!("{}", Prose::new(line).render(&terminal));
        std::process::exit(64);
    }

    let api = match load_api(schema) {
        Ok(api) => api,
        Err(err) => {
            emit_schema_error(&err);
            std::process::exit(2);
        }
    };

    let mut any_parse_error = false;
    let mut any_schema_error = false;
    let mut any_validation_failure = false;

    for file in &files {
        let outcome = validate_one(&api, file, &assignments, no_trigger_schemas);
        match &outcome {
            FileOutcome::Validated { report_valid, .. } if !report_valid => {
                any_validation_failure = true;
            }
            FileOutcome::ParseError(_) => {
                any_parse_error = true;
            }
            FileOutcome::SchemaError(_) => {
                any_schema_error = true;
            }
            FileOutcome::Validated { .. } => {}
        }
        emit_outcome(file, &outcome, format, quiet, &terminal);
    }

    if any_parse_error {
        std::process::exit(3);
    }
    if any_schema_error {
        std::process::exit(2);
    }
    if any_validation_failure {
        std::process::exit(1);
    }
    Ok(())
}

/// Builds the [`DarkmatterSchemas`] entry point, applying the CLI baseline
/// flag or `BASELINE_SCHEMA` env var fallback.
fn load_api(schema: Option<&Path>) -> Result<DarkmatterSchemas, SchemaError> {
    let baseline_path = schema
        .map(PathBuf::from)
        .or_else(|| std::env::var(BASELINE_SCHEMA_ENV).ok().map(PathBuf::from));

    let api = DarkmatterSchemas::new();
    match baseline_path {
        Some(path) => api.with_baseline_from_file(path),
        None => Ok(api),
    }
}

/// Validates a single file, capturing parse/schema failures separately so the
/// caller can map them to the spec's exit codes.
fn validate_one(
    api: &DarkmatterSchemas,
    file: &Path,
    assignments: &[Assignment],
    no_trigger_schemas: bool,
) -> FileOutcome {
    let discovery_path = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let mut md = match Markdown::try_from(discovery_path.as_path()) {
        Ok(md) => md,
        Err(err) => return FileOutcome::ParseError(err.to_string()),
    };

    let api = if no_trigger_schemas {
        api.clone()
    } else if let Some(boundary) =
        darkmatter::markdown::compose::find_git_root_from(&discovery_path)
    {
        match api
            .clone()
            .with_trigger_discovery(&discovery_path, boundary)
        {
            Ok(api) => api,
            Err(err) => return FileOutcome::SchemaError(Box::new(err)),
        }
    } else {
        api.clone()
    };

    if !assignments.is_empty() {
        // Build the pre-assignment effective schema solely to coerce assignment
        // RHS values to declared property types. Validation resolves again
        // below because assignments may change trigger activation.
        let effective = match api.effective_for(&md) {
            Ok(effective) => effective,
            Err(err) => return FileOutcome::SchemaError(Box::new(err)),
        };
        assignment::apply_all(
            md.frontmatter_mut().as_map_mut(),
            assignments,
            effective.as_ref(),
        );
    }

    let schema_label = schema_label_from(&md);
    // Resolve again after assignments: assignments may change which trigger
    // arms match, so the pre-assignment schema is only valid for coercion.
    let no_schema = match api.effective_for(&md) {
        Ok(effective) => effective.is_none(),
        Err(err) => return FileOutcome::SchemaError(Box::new(err)),
    };

    match api.validate(&md) {
        Ok(report) => FileOutcome::Validated {
            report_valid: report.valid,
            problems: report.problems,
            schema_label,
            no_schema,
        },
        Err(err) => FileOutcome::SchemaError(Box::new(err)),
    }
}

/// Splits positional inputs into file paths and parsed assignments. Returns
/// an error message describing the first invalid assignment encountered.
fn split_inputs(inputs: &[String]) -> Result<(Vec<PathBuf>, Vec<Assignment>), String> {
    let mut files = Vec::new();
    let mut assignments = Vec::new();
    for raw in inputs {
        match assignment::classify(raw) {
            PositionalKind::File(p) => files.push(PathBuf::from(p)),
            PositionalKind::Assignment(a) => assignments.push(a),
            PositionalKind::Invalid { raw, message } => {
                return Err(format!("invalid assignment `{raw}`: {message}"));
            }
        }
    }
    Ok((files, assignments))
}

fn schema_label_from(md: &Markdown) -> Option<String> {
    md.frontmatter()
        .as_map()
        .get("$schema")
        .and_then(|value| value.as_str())
        .map(String::from)
}

fn emit_outcome(
    file: &Path,
    outcome: &FileOutcome,
    format: SchemaValidateFormat,
    quiet: bool,
    terminal: &Terminal,
) {
    match format {
        SchemaValidateFormat::Pretty => emit_pretty(file, outcome, quiet, terminal),
        SchemaValidateFormat::Json => emit_json(file, outcome),
    }
}

fn emit_pretty(file: &Path, outcome: &FileOutcome, quiet: bool, terminal: &Terminal) {
    let link = document_link(file);

    match outcome {
        FileOutcome::Validated {
            report_valid: true,
            no_schema,
            ..
        } => {
            if quiet {
                return;
            }
            let tail = if *no_schema {
                "doesn't have a schema definition so is valid by default."
            } else {
                "has a schema definition and is valid."
            };
            let line =
                format!("- <green>✔</green> _<dim>the document</dim>_ {link} _<dim>{tail}</dim>_");
            println!("{}", Prose::new(line).render(terminal));
        }
        FileOutcome::Validated {
            report_valid: false,
            problems,
            ..
        } => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>failed schema validation:</dim>_"
            );
            println!("{}", Prose::new(header).render(terminal));
            for problem in problems {
                emit_problem_bullet(problem, terminal);
            }
        }
        FileOutcome::ParseError(message) => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>has unparseable frontmatter:</dim>_"
            );
            println!("{}", Prose::new(header).render(terminal));
            let bullet = format!("    - {}", escape_prose(message));
            println!("{}", Prose::new(bullet).render(terminal));
        }
        FileOutcome::SchemaError(err) => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>has a schema definition error:</dim>_"
            );
            println!("{}", Prose::new(header).render(terminal));
            let block = err.status_block(terminal);
            println!("{}", block.render(terminal));
        }
    }
}

/// Renders one `ValidationProblem` as an indented Prose list item, with the
/// offending property name (or root-union arm) prefix and an optional
/// source-location suffix.
fn emit_problem_bullet(problem: &ValidationProblem, terminal: &Terminal) {
    let mut prefix_parts: Vec<String> = Vec::new();
    if let Some(arm_index) = problem.arm_index {
        prefix_parts.push(format!("arm[{arm_index}]:"));
    }
    if !problem.path.is_empty() {
        prefix_parts.push(format!(
            "<inverse>{}</inverse>",
            escape_prose(strip_pointer_prefix(&problem.path))
        ));
    } else if let Some(property) = problem.property.as_deref() {
        prefix_parts.push(format!("<inverse>{}</inverse>", escape_prose(property)));
    } else if problem.arm_index.is_none() {
        prefix_parts.push("<inverse>root</inverse>".to_string());
    }
    let prefix = if prefix_parts.is_empty() {
        String::new()
    } else {
        format!("{} ", prefix_parts.join(" "))
    };

    let location = format_location(problem);
    let location_suffix = if location.is_empty() {
        String::new()
    } else {
        format!(" <dim>(at {location})</dim>")
    };

    let message = trim_redundant_property_prefix(&problem.message, problem.property.as_deref());
    let bullet = format!("    - {prefix}{}{location_suffix}", escape_prose(message));
    println!("{}", Prose::new(bullet).render(terminal));

    // Surface the declared property description on its own dimmed sub-line,
    // one indent level beneath the bullet (Decision #7). Enrichment already
    // suppressed empty / message-equal descriptions, so a `Some` here renders.
    if let Some(description) = problem.description.as_deref() {
        let sub_line = format!("        <dim>{}</dim>", escape_prose(description));
        println!("{}", Prose::new(sub_line).render(terminal));
    }
}

/// When `jsonschema` reports a `Required` failure, the message already starts
/// with the quoted property name (e.g. `"name" is a required property`). Since
/// we now surface that name as the bullet prefix, strip the duplicate from the
/// message so it reads naturally: `name is a required property`.
fn trim_redundant_property_prefix<'a>(message: &'a str, property: Option<&str>) -> &'a str {
    let Some(name) = property else {
        return message;
    };
    let quoted = format!("\"{name}\" ");
    message.strip_prefix(&quoted).unwrap_or(message)
}

/// Strips the leading `/` from a JSON pointer so the prefix reads as a
/// human-friendly property reference (`age` rather than `/age`,
/// `tags/2` rather than `/tags/2`).
fn strip_pointer_prefix(pointer: &str) -> &str {
    pointer.strip_prefix('/').unwrap_or(pointer)
}

/// Builds the styled, OSC8-hyperlinked Prose markup for a document. The
/// visible label is the path the user passed on the CLI; the underlying
/// `file://` href is the absolute form so terminal-launched openers can
/// resolve it regardless of the user's current directory.
fn document_link(file: &Path) -> String {
    let absolute = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    format!(
        "<blue>[{label}](file://{href})</blue>",
        label = file.display(),
        href = absolute.display(),
    )
}

fn emit_json(file: &Path, outcome: &FileOutcome) {
    use serde_json::json;

    let file_str = file.to_string_lossy().into_owned();
    let value = match outcome {
        FileOutcome::Validated {
            report_valid,
            problems,
            schema_label,
            ..
        } => {
            let problems_json: Vec<serde_json::Value> = problems
                .iter()
                .map(|p| {
                    json!({
                        "path": p.path,
                        "property": p.property,
                        "message": p.message,
                        "kind": kind_str(p.kind),
                        "line": p.line,
                        "column": p.column,
                        "arm_index": p.arm_index,
                        "description": p.description,
                    })
                })
                .collect();
            json!({
                "file": file_str,
                "valid": report_valid,
                "schema": schema_label,
                "problems": problems_json,
            })
        }
        FileOutcome::ParseError(message) => json!({
            "file": file_str,
            "valid": false,
            "schema": serde_json::Value::Null,
            "error": "frontmatter_parse",
            "message": message,
            "problems": [],
        }),
        FileOutcome::SchemaError(err) => json!({
            "file": file_str,
            "valid": false,
            "schema": serde_json::Value::Null,
            "error": "schema",
            "message": err.to_string(),
            "problems": [],
        }),
    };
    println!("{}", value);
}

/// Stable lowercase token for a [`ValidationProblemKind`], used in JSON output.
fn kind_str(kind: ValidationProblemKind) -> &'static str {
    match kind {
        ValidationProblemKind::Missing => "missing",
        ValidationProblemKind::Type => "type",
        ValidationProblemKind::Invalid => "invalid",
    }
}

fn format_location(problem: &ValidationProblem) -> String {
    // The position map only ever records column 1 for top-level frontmatter
    // keys (see `build_position_map` in the schemas validator), so the
    // column information would always be redundant — line alone is enough.
    match problem.line {
        Some(line) => format!("line {line} of frontmatter"),
        None => String::new(),
    }
}

/// Escape angle-bracketed snippets in arbitrary messages so they don't get
/// interpreted as Prose markup.
fn escape_prose(input: &str) -> String {
    input.replace('<', "&lt;").replace('>', "&gt;")
}

fn emit_schema_error(err: &SchemaError) {
    let terminal = Terminal::default();
    let block = err.status_block(&terminal);
    eprintln!("{}", block.render(&terminal));
}
