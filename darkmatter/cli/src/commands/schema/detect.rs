//! `md schema detect` implementation.

use crate::args::SchemaDetectFormat;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DetectOptions, detect_schema, schema_to_yaml, to_json_schema};
use std::path::{Path, PathBuf};

/// Run `md schema detect`.
///
/// Exits with `0` on success, `3` when at least one input file's
/// frontmatter cannot be parsed (matching the validate convention), and
/// `2` when conversion to JSON Schema fails (only possible with
/// `--format json`).
pub fn run_detect(files: &[PathBuf], format: SchemaDetectFormat, merge: bool) -> Result<()> {
    let terminal = Terminal::default();
    let mut docs: Vec<Markdown> = Vec::with_capacity(files.len());
    let mut any_parse_error = false;

    for file in files {
        match load_document(file) {
            Ok(md) => docs.push(md),
            Err(err) => {
                emit_error_line(&terminal, file, &err);
                any_parse_error = true;
            }
        }
    }

    if any_parse_error {
        std::process::exit(3);
    }

    if merge || docs.len() <= 1 {
        let refs: Vec<&Markdown> = docs.iter().collect();
        let schema = detect_schema(&refs, DetectOptions { merge });
        emit_schema(format, files, &schema);
    } else {
        // Without --merge, emit one schema per file with a header comment.
        for (file, md) in files.iter().zip(docs.iter()) {
            let schema = detect_schema(&[md], DetectOptions { merge: false });
            print_header(format, file);
            emit_schema(format, std::slice::from_ref(file), &schema);
        }
    }

    Ok(())
}

fn load_document(path: &Path) -> Result<Markdown, String> {
    Markdown::try_from(path).map_err(|e| e.to_string())
}

fn emit_schema(
    format: SchemaDetectFormat,
    sources: &[PathBuf],
    schema: &darkmatter::markdown::schemas::SimplifiedSchema,
) {
    match format {
        SchemaDetectFormat::Yaml => {
            // Source-file header is helpful when the result is piped or
            // copied into a doc — kept on a single comment line.
            if !sources.is_empty() {
                let joined = sources
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("# detected from: {joined}");
            }
            print!("{}", schema_to_yaml(schema));
        }
        SchemaDetectFormat::Json => match to_json_schema(schema) {
            Ok(value) => match serde_json::to_string_pretty(&value) {
                Ok(text) => println!("{text}"),
                Err(err) => {
                    emit_json_error(&format!("could not serialise JSON Schema: {err}"));
                    std::process::exit(2);
                }
            },
            Err(err) => {
                emit_json_error(&format!("could not build JSON Schema: {err}"));
                std::process::exit(2);
            }
        },
    }
}

fn print_header(format: SchemaDetectFormat, file: &Path) {
    if matches!(format, SchemaDetectFormat::Yaml) {
        println!("# --- {} ---", file.display());
    }
}

fn emit_error_line(terminal: &Terminal, file: &Path, err: &str) {
    let body = format!(
        "<red><b>Error:</b></red> {}: {}",
        escape_prose(&file.display().to_string()),
        escape_prose(err)
    );
    eprintln!("{}", Prose::new(body).render(terminal));
}

fn emit_json_error(message: &str) {
    let terminal = Terminal::default();
    let body = format!("<red><b>Error:</b></red> {}", escape_prose(message));
    eprintln!("{}", Prose::new(body).render(&terminal));
}

fn escape_prose(input: &str) -> String {
    input.replace('<', "&lt;").replace('>', "&gt;")
}
