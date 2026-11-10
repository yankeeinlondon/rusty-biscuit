//! `md schema detect` implementation.

use crate::args::Cli;
use crate::args::output::SchemaDetectOutput;
use crate::commands::code_block::code_value;
use crate::io::{DocumentArgumentError, open_argument};
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::output::output_diagnostic;
use crate::output::report::ClassifiedFailure;
use crate::request::MdRequest;
use biscuit_file::FileResolutionContext;
use biscuit_terminal::output::WidthRequest;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
use darkmatter::markdown::compose::report_diagnostics::{FRONTMATTER_PARSE_CODE, INPUT_READ_CODE};
use darkmatter::markdown::output::command::{CommandValue, DetectData, DetectedFile};
use darkmatter::markdown::schemas::{DetectOptions, detect_schema_with_contexts, schema_to_yaml};
use darkmatter::markdown::{CodeBlock, Markdown};
use serde_json::Value;
use simplified_schema::{SchemaRoot, to_json_schema};
use std::path::{Path, PathBuf};

/// Run `md schema detect`.
///
/// Prints the inferred SimplifiedSchema as YAML in a code block, as data with a
/// data output, or as JSON Schema with `json-schema`. Several inputs without
/// `--merge` give one schema per input. An input that cannot be read or parsed
/// fails the run (exit `1`) and reports every such input; so does a schema with
/// no JSON Schema form.
///
/// Each document infers `file` values in its own context
/// ([`MdRequest::document_context`] from its opening reference), never the
/// launch context, so a document in another repository, outside every
/// repository, or named through a symlinked spelling classifies its
/// references as it resolves them.
pub fn run_detect(files: &[PathBuf], output: SchemaDetectOutput, merge: bool, cli: &Cli, request: &MdRequest) -> Result<()> {
    let run = CommandRun::begin(output.target(), cli.width, WidthRequest::Auto, cli.verbose, DataShape::Bare)?;
    let result = detect_value(files, output, merge, request);
    run.finish(result)
}

fn detect_value(files: &[PathBuf], output: SchemaDetectOutput, merge: bool, request: &MdRequest) -> Result<Completed<CommandValue<DetectData>>> {
    let mut docs: Vec<Markdown> = Vec::with_capacity(files.len());
    let mut contexts: Vec<FileResolutionContext> = Vec::with_capacity(files.len());
    let mut failures: Vec<Diagnostic> = Vec::new();

    for file in files {
        match load_document(file, request) {
            Ok((md, context)) => {
                docs.push(md);
                contexts.push(context);
            }
            Err(LoadError::Unopened(err)) => {
                failures.push(match err.chain().find_map(|cause| cause.downcast_ref::<DocumentArgumentError>()) {
                    Some(argument) => argument.diagnostic(),
                    None => output_diagnostic(INPUT_READ_CODE, Severity::Error, &format!("{}: {err:#}", file.display())),
                });
            }
            Err(LoadError::Parse(err)) => {
                failures.push(output_diagnostic(FRONTMATTER_PARSE_CODE, Severity::Error, &format!("{}: {err}", file.display())));
            }
        }
    }
    if !failures.is_empty() {
        let summary = failures.iter().map(|failure| failure.message.as_str()).collect::<Vec<_>>().join("\n");
        return Err(ClassifiedFailure::report(eyre!("{summary}"), failures));
    }

    if merge || docs.len() <= 1 {
        let refs: Vec<&Markdown> = docs.iter().collect();
        let schema = detect_schema_with_contexts(&refs, DetectOptions { merge }, &contexts);
        let value = schema_value(&schema, output)?;
        let source = source_text(&schema, &value, output)?;
        return Ok(Completed::success(code_value(DetectData::Single(value), code_block(&source, output))));
    }

    let mut entries = Vec::with_capacity(docs.len());
    let mut combined = String::new();
    for ((file, md), context) in files.iter().zip(docs.iter()).zip(contexts) {
        let schema = detect_schema_with_contexts(&[md], DetectOptions { merge: false }, &[context]);
        let value = schema_value(&schema, output)?;
        combined.push_str(&format!("# --- {} ---\n{}", file.display(), source_text(&schema, &value, output)?));
        if !combined.ends_with('\n') {
            combined.push('\n');
        }
        entries.push(DetectedFile { file: file.display().to_string(), schema: value });
    }
    Ok(Completed::success(code_value(DetectData::Files { files: entries }, code_block(&combined, output))))
}

fn code_block(source: &str, output: SchemaDetectOutput) -> CodeBlock {
    let language = if output == SchemaDetectOutput::JsonSchema { "json" } else { "yaml" };
    CodeBlock::new(source.trim_end().to_string()).with_fence_language(language)
}

/// `schema` as data: the SimplifiedSchema, or for `json-schema` its Draft 2020-12
/// form.
fn schema_value(schema: &SchemaRoot, output: SchemaDetectOutput) -> Result<Value> {
    match output {
        SchemaDetectOutput::JsonSchema => to_json_schema(schema).map_err(|error| eyre!("could not build JSON Schema: {error}")),
        SchemaDetectOutput::Target(_) => {
            serde_yaml_ng::from_str(&schema_to_yaml(schema)).wrap_err("could not read the detected schema as data")
        }
    }
}

/// `schema`'s text in its formatted code block.
fn source_text(schema: &SchemaRoot, value: &Value, output: SchemaDetectOutput) -> Result<String> {
    match output {
        SchemaDetectOutput::JsonSchema => serde_json::to_string_pretty(value).wrap_err("could not serialize JSON Schema"),
        SchemaDetectOutput::Target(_) => Ok(schema_to_yaml(schema)),
    }
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
