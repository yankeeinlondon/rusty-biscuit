//! `md schema validate` implementation.

use crate::args::SchemaValidateFormat;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaError, ValidationProblem};
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
    files: &[PathBuf],
    schema: Option<&Path>,
    format: SchemaValidateFormat,
    quiet: bool,
) -> Result<()> {
    let api = match load_api(schema) {
        Ok(api) => api,
        Err(err) => {
            emit_schema_error(&err);
            std::process::exit(2);
        }
    };

    let terminal = Terminal::default();
    let mut any_parse_error = false;
    let mut any_schema_error = false;
    let mut any_validation_failure = false;

    for file in files {
        let outcome = validate_one(&api, file);
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
fn validate_one(api: &DarkmatterSchemas, file: &Path) -> FileOutcome {
    let md = match Markdown::try_from(file) {
        Ok(md) => md,
        Err(err) => return FileOutcome::ParseError(err.to_string()),
    };

    let schema_label = schema_label_from(&md);
    let no_schema = md.frontmatter().as_map().get("$schema").is_none();

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
    let display = file.display();
    match outcome {
        FileOutcome::Validated {
            report_valid: true,
            schema_label,
            no_schema,
            ..
        } => {
            if quiet {
                return;
            }
            let suffix = if *no_schema {
                " <dim>(no schema; vacuously valid)</dim>".to_string()
            } else if let Some(label) = schema_label {
                format!(" <dim>(schema: {label})</dim>")
            } else {
                String::new()
            };
            let line = format!("{display}  <green>✓</green> valid{suffix}");
            println!("{}", Prose::new(line).render(terminal));
        }
        FileOutcome::Validated {
            report_valid: false,
            problems,
            ..
        } => {
            let count = problems.len();
            let header = format!(
                "{display}  <red>✗</red> {count} {}",
                if count == 1 { "problem" } else { "problems" }
            );
            println!("{}", Prose::new(header).render(terminal));
            for problem in problems {
                let location = format_location(problem);
                let mut prefix_parts: Vec<String> = Vec::new();
                if let Some(arm_index) = problem.arm_index {
                    prefix_parts.push(format!("arm[{arm_index}]:"));
                }
                if !problem.path.is_empty() {
                    prefix_parts.push(escape_prose(&problem.path));
                } else if problem.arm_index.is_none() {
                    prefix_parts.push("root".to_string());
                }
                let prefix = if prefix_parts.is_empty() {
                    String::new()
                } else {
                    format!("{} ", prefix_parts.join(" "))
                };
                let bullet = format!("  • {prefix}{}", escape_prose(&problem.message));
                println!("{}", Prose::new(bullet).render(terminal));
                if !location.is_empty() {
                    let loc_line = format!("      <dim>at {location}</dim>");
                    println!("{}", Prose::new(loc_line).render(terminal));
                }
            }
        }
        FileOutcome::ParseError(message) => {
            let header = format!("{display}  <red>✗</red> frontmatter parse error");
            println!("{}", Prose::new(header).render(terminal));
            let body = format!("      <dim>{}</dim>", escape_prose(message));
            println!("{}", Prose::new(body).render(terminal));
        }
        FileOutcome::SchemaError(err) => {
            let header = format!("{display}  <red>✗</red> schema error");
            println!("{}", Prose::new(header).render(terminal));
            let block = err.status_block(terminal);
            println!("{}", block.render(terminal));
        }
    }
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
                        "message": p.message,
                        "line": p.line,
                        "column": p.column,
                        "arm_index": p.arm_index,
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

fn format_location(problem: &ValidationProblem) -> String {
    match (problem.line, problem.column) {
        (Some(line), Some(col)) => format!("line {line}, column {col} of frontmatter"),
        (Some(line), None) => format!("line {line} of frontmatter"),
        _ => String::new(),
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
