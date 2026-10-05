//! `md schema detect` implementation.

use crate::args::SchemaDetectFormat;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::{
    DetectOptions, detect_schema_with_contexts, schema_to_yaml, to_json_schema,
};

use crate::io::{DocumentArgumentError, open_argument};
use crate::request::MdRequest;
use biscuit_terminal::errors::BlockError as _;
use std::path::{Path, PathBuf};

/// Run `md schema detect`.
///
/// Exits with `0` on success, `3` when at least one input file's
/// frontmatter cannot be parsed (matching the validate convention), and
/// `2` when conversion to JSON Schema fails (only possible with
/// `--format json`).
///
/// Each document infers `file` values in its own context
/// ([`MdRequest::document_context`] from its opening reference), never the
/// launch context, so a document in another repository, outside every
/// repository, or named through a symlinked spelling classifies its
/// references as it resolves them.
pub fn run_detect(
    files: &[PathBuf],
    format: SchemaDetectFormat,
    merge: bool,
    request: &MdRequest,
) -> Result<()> {
    let terminal = Terminal::default();
    let mut docs: Vec<Markdown> = Vec::with_capacity(files.len());
    let mut contexts: Vec<FileResolutionContext> = Vec::with_capacity(files.len());
    let mut any_parse_error = false;

    for file in files {
        match load_document(file, request) {
            Ok((md, context)) => {
                docs.push(md);
                contexts.push(context);
            }
            Err(LoadError::Unopened(err)) => {
                match err.chain().find_map(|cause| cause.downcast_ref::<DocumentArgumentError>()) {
                    Some(argument) => eprintln!("{}", argument.report_block_error(&terminal)),
                    None => emit_error_line(&terminal, file, &format!("{err:#}")),
                }
                any_parse_error = true;
            }
            Err(LoadError::Parse(err)) => {
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
        let schema = detect_schema_with_contexts(&refs, DetectOptions { merge }, &contexts);
        emit_schema(format, files, &schema);
    } else {
        // Without --merge, emit one schema per file with a header comment.
        for ((file, md), context) in files.iter().zip(docs.iter()).zip(contexts) {
            let schema =
                detect_schema_with_contexts(&[md], DetectOptions { merge: false }, &[context]);
            print_header(format, file);
            emit_schema(format, std::slice::from_ref(file), &schema);
        }
    }

    Ok(())
}

enum LoadError {
    /// The argument did not open through the reference grammar, or the
    /// opened document's context could not be built.
    Unopened(color_eyre::eyre::Report),
    /// The opened file did not parse.
    Parse(String),
}

/// Opens one argument and returns the document with the context its own
/// references resolve in.
fn load_document(
    argument: &Path,
    request: &MdRequest,
) -> Result<(Markdown, FileResolutionContext), LoadError> {
    let opened = open_argument(argument, request).map_err(LoadError::Unopened)?;
    // Canonical, as `schema validate` and `schema triggers` do: the document
    // and its context then agree on one spelling whichever symlinked alias
    // (macOS `/var` for `/private/var`) named the file.
    let document_path = biscuit_file::canonicalize_simplified(opened.path())
        .unwrap_or_else(|_| opened.path().to_path_buf());
    let markdown =
        Markdown::try_from(document_path.as_path()).map_err(|e| LoadError::Parse(e.to_string()))?;
    let context = request
        .document_context(Some(opened.reference()), &document_path)
        .map_err(LoadError::Unopened)?;
    Ok((markdown, context))
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

/// Escape text so it renders exactly as written inside Prose markup; code
/// spans it marks with backticks stay literal.
fn escape_prose(input: &str) -> String {
    Prose::escape_text_outside_code_spans(input)
}
