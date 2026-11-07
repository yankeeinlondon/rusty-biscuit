//! `md get` / `md set` / `md rm` / `md edit` frontmatter subcommand implementations.

use crate::args::Cli;
use crate::commands::render::document_value;
use crate::commands::schema::validate::document_schemas;
use crate::commands::trigger_findings::{emit_warnings, finding_diagnostic};
use crate::io::{DocumentArgumentError, OpenedArgument, load_markdown, open_argument, parse_argument, resolve_file_path};
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use clap::CommandFactory as _;
use color_eyre::eyre::{Context, Report, Result, eyre};
use darkmatter::markdown::cleanup::{reflow_to_width, strip_incidental_newlines};
use darkmatter::markdown::hash::{PathEdit, PathEditRefusal, edited_document_text};
use darkmatter::markdown::output::command::{CommandValue, EditData, RemovedData, SetData, prose_document};
use darkmatter::markdown::schemas::completion::declared_shape;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, ValidationOptions, frontmatter_as_json};
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::{Markdown, MarkdownError, extract_frontmatter_block};
use renderable::Width;
use renderable::tree::{Document, RenderNode};
use serde_json::Value;
use simplified_schema::simplified::parse_yaml_schema;
use simplified_schema::{
    PositionMap, PropertyPath, RemovalPlan, ResolveStop, SchemaError, SetterValue, ValidationProblem, classify_schema_reference, set_at,
};
use std::path::PathBuf;
use tracing::instrument;

/// Parses a property path, or returns a usage error: nothing has run yet.
fn parse_path(argument: &str) -> Result<PropertyPath> {
    PropertyPath::parse(argument)
        .map_err(|error| Cli::command().error(clap::error::ErrorKind::ValueValidation, error.to_string()).into())
}

fn parse_paths(arguments: &[String]) -> Result<Vec<PropertyPath>> {
    arguments.iter().map(|argument| parse_path(argument)).collect()
}

/// Get frontmatter values from a markdown document, by property path.
///
/// Every path is parsed before the document is read, so a malformed one is a
/// usage error with no input consumed. The value prints no prose, so `--width`
/// changes nothing here.
#[instrument(skip_all)]
pub fn run_get(input: &PathBuf, props: &[String], output: OutputTarget, compact: bool, cli: &Cli, request: &MdRequest) -> Result<()> {
    if compact && !matches!(output, OutputTarget::Json | OutputTarget::Json5) {
        return Err(Cli::command()
            .error(clap::error::ErrorKind::ArgumentConflict, "`--compact` is valid only with `--output json` or `--output json5`")
            .into());
    }
    let paths = parse_paths(props)?;
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Bare)?.compact_json(compact);
    let result = load_markdown(Some(input), request).map(|md| {
        let value = requested_value(&md, &paths);
        // `println!` always printed a final newline, even for an empty value.
        let raw = format!("{}\n", format_raw(&value));
        let terminal = raw.clone();
        let prose = raw_document(&raw);
        Completed::success(
            CommandValue::new(value, prose).with_terminal(move |_| terminal.clone()).with_markdown(raw),
        )
    });
    run.finish(result)
}

/// One path's value, or a flat object keyed by the paths as written. A path
/// that is missing or cannot be reached is `""`; an explicit `null` stays `null`.
fn requested_value(md: &Markdown, paths: &[PropertyPath]) -> Value {
    let map = md.frontmatter().as_map();
    let value_of = |path: &PropertyPath| path.resolve_map(map).value().cloned().unwrap_or_else(|| Value::String(String::new()));
    match paths {
        [path] => value_of(path),
        _ => Value::Object(paths.iter().map(|path| (path.as_written().to_string(), value_of(path))).collect()),
    }
}

/// `raw`'s lines, as they are, in one paragraph.
fn raw_document(raw: &str) -> Document {
    let mut children = Vec::new();
    for line in raw.trim_end_matches('\n').lines() {
        if !children.is_empty() {
            children.push(RenderNode::hard_break());
        }
        children.push(RenderNode::text(line));
    }
    Document::new(RenderNode::root(vec![RenderNode::paragraph(children)]))
}

/// Set the value at a property path in a markdown document.
///
/// By default the modified document is the deliverable and the source file is
/// not changed. With `--save` the file is updated in place, and a formatted
/// output prints nothing; a data output always reports the result. A malformed
/// path is a usage error; a value, edit, or schema failure is a run failure, and
/// the file is not touched.
#[instrument(skip_all)]
pub fn run_set(input: &PathBuf, prop: &str, raw_value: &str, save: bool, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let path = parse_path(prop)?;
    let file = input_label(input);
    let skeleton = SetData { file: file.clone(), property: prop.to_string(), value: Value::Null, document: None, saved: false };
    let mut run = CommandRun::begin(output, cli.width, WidthRequest::Source, cli.verbose, DataShape::typed(&skeleton))?.exact_markdown();
    let result = set_value(input, &path, skeleton, raw_value, save, &mut run, cli, request);
    run.finish(result)
}

#[allow(clippy::too_many_arguments)]
fn set_value(
    input: &PathBuf,
    path: &PropertyPath,
    mut data: SetData,
    raw_value: &str,
    save: bool,
    run: &mut CommandRun,
    cli: &Cli,
    request: &MdRequest,
) -> Result<Completed<CommandValue<SetData>>> {
    if save && data.file.is_none() {
        return Err(eyre!("--save requires an input file path (stdin is not supported)"));
    }

    let parsed = SetterValue::parse(raw_value).map_err(|error| eyre!("Invalid value for `{}`: {error}", path.as_written()))?;
    let (md, opened) = open_for_edit(input, request)?;
    if let Some((_, document)) = &opened {
        run.set_document(document);
    }

    let (md, value) = if names_schema_key(path) {
        set_schema_key(md, path, &parsed)?
    } else {
        set_checked(md, path, &parsed, opened.as_ref(), run, request)?
    };
    data.value = value;

    let steps = path.resolve_map(md.frontmatter().as_map()).steps().to_vec();
    let edited = edited_document_text(&md, &[PathEdit::Set(steps)]).map_err(refused_edit)?;
    let output = run.output();
    let width = output.explicit_width.then_some(output.options.width);
    let text = at_width(edited, width);
    data.document = Some(text.clone());

    if save {
        let resolved = resolve_file_path(input, request.launch_context()?)?;
        std::fs::write(&resolved, &text).wrap_err_with(|| format!("Failed to write to {resolved:?}"))?;
        data.saved = true;
        if !run.is_data() {
            let prose = darkmatter::markdown::output::command::empty_document();
            return Ok(Completed::success(CommandValue::new(data, prose).with_terminal(|_| String::new()).with_markdown(String::new())));
        }
    }
    let resolved = resolved_input(input, request);
    document_value(data, &md, text, resolved.as_ref(), output, cli).map(Completed::success)
}

/// The document to edit, with the opened argument and resolved file that give it
/// a context; both are absent for stdin.
///
/// The canonical spelling is the one schema discovery compares against, as in
/// `md schema validate`.
fn open_for_edit(input: &PathBuf, request: &MdRequest) -> Result<(Markdown, Option<(OpenedArgument, PathBuf)>)> {
    if input.to_str() == Some("-") {
        return Ok((load_markdown(Some(input), request)?, None));
    }
    let opened = open_argument(input, request)?;
    let document = biscuit_file::canonicalize_simplified(opened.path()).unwrap_or_else(|_| opened.path().to_path_buf());
    let md = Markdown::try_from(document.as_path()).wrap_err_with(|| format!("Failed to read file: {document:?}"))?;
    Ok((md, Some((opened, document))))
}

fn names_schema_key(path: &PropertyPath) -> bool {
    path.segments().first().is_some_and(|segment| segment == "$schema")
}

/// Sets `$schema` or a value inside an inline one, judging only that the result
/// can be a schema; the rest of the document is not validated against it, and no
/// schema file is read.
fn set_schema_key(mut md: Markdown, path: &PropertyPath, parsed: &SetterValue) -> Result<(Markdown, Value)> {
    let value = match parsed {
        // A reference is text whatever it looks like: a bare name may be `2`.
        SetterValue::Text { raw } if path.segments().len() == 1 => Value::String(raw.clone()),
        other => other.bind(None),
    };
    set_at(md.frontmatter_mut().as_map_mut(), path, value.clone())?;
    let schema = &md.frontmatter().as_map()["$schema"];
    let checked = match schema {
        Value::String(reference) => classify_schema_reference(reference).map(drop),
        Value::Object(_) | Value::Array(_) => {
            let yaml = biscuit_file::serde_yaml_ng::to_value(schema).wrap_err("Failed to read the inline `$schema`")?;
            parse_yaml_schema(&yaml).map(drop)
        }
        other => return Err(eyre!("`$schema` must be a schema reference or an inline schema, not {other}")),
    };
    checked.map_err(|error| schema_failure(&md, error))?;
    Ok((md, value))
}

/// Applies `parsed` at `path` and judges the result, returning the edited document
/// and the value it holds.
///
/// The edit is made first with the undeclared reading, because the value can
/// change which schema applies; the schema of that state declares the type the
/// value is then read through, once. The document keeps the value as validation
/// reads it ([`value_as_judged`]), because that is the value that is judged. It
/// differs from the bound one where the declaration
/// could not name the type: a raw JSON Schema `$ref` projects to `unknown`, so the
/// value arrives as text and only the validator knows it is a number. Nothing is
/// written here, so a failure of any kind leaves the source alone.
fn set_checked(
    md: Markdown,
    path: &PropertyPath,
    parsed: &SetterValue,
    opened: Option<&(OpenedArgument, PathBuf)>,
    run: &mut CommandRun,
    request: &MdRequest,
) -> Result<(Markdown, Value)> {
    let guess = parsed.bind(None);
    let mut overlay = md.clone();
    set_at(overlay.frontmatter_mut().as_map_mut(), path, guess.clone())?;

    let schemas = edit_schemas(&md, opened, run, request)?;
    let effective_for = |document: &Markdown| schemas.effective_for(document).map_err(|error| schema_failure(document, error));
    let before = effective_for(&md)?;
    let overlay_schema = effective_for(&overlay)?;

    let declared = overlay_schema.as_ref().and_then(|effective| {
        let steps = path.resolve_map(overlay.frontmatter().as_map());
        declared_shape(effective, &frontmatter_as_json(&overlay), steps.steps(), Some(parsed))
    });
    let mut value = parsed.bind(declared.as_ref());
    let (mut edited, mut after) = if value == guess {
        (overlay, overlay_schema)
    } else {
        let mut edited = md.clone();
        set_at(edited.frontmatter_mut().as_map_mut(), path, value.clone())?;
        let after = effective_for(&edited)?;
        (edited, after)
    };
    if let Some(judged) = after.as_ref().and_then(|schema| value_as_judged(schema, &edited, path)).filter(|judged| *judged != value) {
        set_at(edited.frontmatter_mut().as_map_mut(), path, judged.clone())?;
        after = effective_for(&edited)?;
        value = judged;
    }

    let problems = blocking_problems(
        &problems_of(before.as_ref(), &md),
        problems_of(after.as_ref(), &edited),
        path.segments(),
    );
    if problems.is_empty() {
        return Ok((edited, value));
    }
    Err(MarkdownError::SchemaValidationFailed {
        path: document_label(&md),
        problems,
        summary: format!("`{}` was not set", path.as_written()),
        description: description_of(&md),
        source: None,
    }
    .into())
}

/// The value at `path` in the copy of `document` that `schema` judges.
fn value_as_judged(schema: &EffectiveSchema, document: &Markdown, path: &PropertyPath) -> Option<Value> {
    let instance = frontmatter_as_json(document);
    let copy = schema.data_copy(&instance);
    path.resolve(&copy).value().cloned()
}

/// The schema entry point for one document, set up as `md schema validate` sets
/// it up. A trigger that discovery skipped is a warning, delivered here so that
/// a failure further on still reports it.
fn edit_schemas(
    md: &Markdown,
    opened: Option<&(OpenedArgument, PathBuf)>,
    run: &mut CommandRun,
    request: &MdRequest,
) -> Result<DarkmatterSchemas> {
    let context = match opened {
        Some((argument, document)) => request.document_context(Some(argument.reference()), document)?,
        None => request.launch_context()?.clone(),
    };
    let schemas = document_schemas(request, context).map_err(|error| schema_failure(md, error))?;
    if run.is_data() {
        run.add_diagnostics(schemas.discovery_findings().iter().map(finding_diagnostic));
    } else {
        emit_warnings(schemas.discovery_findings(), &crate::output::stderr_terminal());
    }
    Ok(schemas)
}

/// `document`'s problems under `schema`. A value still waiting on a template or
/// shell expression is not judged, as composition would not judge it yet. Nothing
/// is evaluated, and no eager file value is normalized.
fn problems_of(schema: Option<&EffectiveSchema>, document: &Markdown) -> Vec<ValidationProblem> {
    schema.map_or_else(Vec::new, |schema| {
        schema
            .validate_with_options(&frontmatter_as_json(document), &PositionMap::new(), &ValidationOptions::default())
            .problems
    })
}

/// The problems of the edited document that block `md set`: those at or under
/// the changed path, and any the edit introduced. A problem the document already
/// had elsewhere stays out of the way, which keeps `md set` usable as a repair.
///
/// Problems compare on where they are and what they are, never on a message,
/// which embeds instance values, or on a position, which an edit moves.
fn blocking_problems(before: &[ValidationProblem], after: Vec<ValidationProblem>, changed: &[String]) -> Vec<ValidationProblem> {
    let same = |after: &ValidationProblem, earlier: &ValidationProblem| {
        after.instance_path.segments() == earlier.instance_path.segments()
            && after.code == earlier.code
            && after.property == earlier.property
            && after.offending_properties.iter().all(|key| earlier.offending_properties.contains(key))
    };
    let at_or_under = |problem: &ValidationProblem, child: Option<&String>| {
        let target = problem.instance_path.segments().iter().chain(child);
        changed.len() <= target.clone().count() && target.zip(changed).all(|(left, right)| left == right)
    };
    after
        .into_iter()
        .filter(|problem| {
            // A required or unknown-key problem points at the parent, and names
            // its children separately.
            let touched = if problem.property.is_none() && problem.offending_properties.is_empty() {
                at_or_under(problem, None)
            } else {
                problem.property.iter().chain(&problem.offending_properties).any(|child| at_or_under(problem, Some(child)))
            };
            touched || !before.iter().any(|earlier| same(problem, earlier))
        })
        .collect()
}

/// A schema that could not be prepared, as the error composition reports for one.
fn schema_failure(document: &Markdown, error: SchemaError) -> Report {
    MarkdownError::SchemaValidationFailed {
        path: document_label(document),
        problems: Vec::new(),
        summary: error.to_string(),
        description: description_of(document),
        source: Some(Box::new(error)),
    }
    .into()
}

fn document_label(document: &Markdown) -> PathBuf {
    match document.source() {
        Some(ComposeSource::File(path)) => path.to_path_buf(),
        _ => PathBuf::from("<stdin>"),
    }
}

fn description_of(document: &Markdown) -> Option<String> {
    document.frontmatter().as_map().get("description").and_then(Value::as_str).map(String::from)
}

fn refused_edit(refusal: PathEditRefusal) -> Report {
    eyre!("Nothing was written: {refusal}")
}

/// `text` with its body at `width`, its frontmatter bytes untouched.
fn at_width(text: String, width: Option<Width>) -> String {
    let Some(width) = width.filter(|width| *width != Width::Source) else {
        return text;
    };
    let body_start = match extract_frontmatter_block(&text) {
        Ok(Some(extraction)) => extraction.body_span.start,
        _ => 0,
    };
    let (frontmatter, body) = text.split_at(body_start);
    let body = match width {
        Width::Columns(columns) => reflow_to_width(body, columns.max(1) as usize),
        _ => strip_incidental_newlines(body),
    };
    format!("{frontmatter}{body}")
}

fn input_label(input: &std::path::Path) -> Option<String> {
    (input.to_str() != Some("-")).then(|| input.display().to_string())
}

fn resolved_input(input: &std::path::Path, request: &MdRequest) -> Option<PathBuf> {
    if input.to_str() == Some("-") {
        return None;
    }
    resolve_file_path(input, request.launch_context().ok()?).ok()
}

/// Remove the values at one or more property paths from a markdown document.
///
/// Every path is parsed before the document is read, and every removal is planned
/// against the unedited document: a missing target or a conflict between two
/// paths cancels the command, and array indexes are the original ones.
///
/// Saves the file in place. The terminal outputs print nothing on success; `-v`
/// adds a summary on stderr. An explicit Markdown or HTML output, and any data
/// output, reports the result.
#[instrument(skip_all)]
pub fn run_rm(input: &PathBuf, props: &[String], output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let paths = parse_paths(props)?;
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Object)?;
    let result = rm_value(input, props, &paths, &run, cli, request);
    run.finish(result)
}

fn rm_value(
    input: &PathBuf,
    props: &[String],
    paths: &[PropertyPath],
    run: &CommandRun,
    cli: &Cli,
    request: &MdRequest,
) -> Result<Completed<CommandValue<RemovedData>>> {
    let resolved = resolve_file_path(input, request.launch_context()?)?;
    let mut md = load_markdown(Some(input), request)?;

    let not_found: Vec<&str> = paths
        .iter()
        .filter(|path| path.resolve_map(md.frontmatter().as_map()).stop() == ResolveStop::MissingKey)
        .map(PropertyPath::as_written)
        .collect();
    if !not_found.is_empty() {
        let missing = not_found.join(", ");
        return Err(eyre!(
            "Property {} not found in frontmatter",
            if not_found.len() == 1 {
                format!("\"{}\"", missing)
            } else {
                format!(
                    "[{}]",
                    not_found
                        .iter()
                        .map(|p| format!("\"{p}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        ));
    }

    let plan = RemovalPlan::new(md.frontmatter().as_map(), paths)?;
    plan.apply(md.frontmatter_mut().as_map_mut());
    let edits: Vec<PathEdit<'_>> = plan.targets().map(PathEdit::Remove).collect();
    let text = edited_document_text(&md, &edits).map_err(refused_edit)?;
    std::fs::write(&resolved, text).wrap_err_with(|| format!("Failed to write to {:?}", resolved))?;

    let removed = props.to_vec();
    let remaining: Vec<String> = md.frontmatter().as_map().keys().cloned().collect();
    let summary = rm_summary(&removed, &remaining);
    if cli.verbose > 0 && !run.is_data() {
        eprintln!("{}", Prose::new(summary.clone()).render(&crate::output::stderr_terminal()));
    }
    let data = RemovedData { removed, remaining, filename: resolved.to_string_lossy().into_owned() };
    let prose = prose_document(&rm_markdown(&data));
    Ok(Completed::success(CommandValue::new(data, prose).with_terminal(|_| String::new())))
}

fn rm_summary(removed: &[String], remaining: &[String]) -> String {
    let props_label = if removed.len() == 1 {
        format!("<b>{}</b> property", removed[0])
    } else {
        format!(
            "<b>{}</b> properties",
            removed.iter().map(|p| format!("\"{p}\"")).collect::<Vec<_>>().join(", ")
        )
    };
    format!(
        "- removed the {props_label} from frontmatter (<dim>remaining: <i>{}</i></dim>)",
        remaining.join(", ")
    )
}

fn rm_markdown(data: &RemovedData) -> String {
    let spans = |names: &[String]| names.iter().map(|name| renderable::markdown::code_span(name)).collect::<Vec<_>>().join(", ");
    format!("Removed {} from the frontmatter. Remaining: {}.\n", spans(&data.removed), spans(&data.remaining))
}

/// Format a JSON value as raw output (no JSON quoting).
///
/// - Strings: printed without quotes
/// - Null: empty string
/// - Booleans/numbers: their natural representation
/// - Arrays: one element per line (each element formatted raw)
/// - Objects: one `key: value` pair per line (values formatted raw)
fn format_raw(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr.iter().map(format_raw).collect::<Vec<_>>().join("\n"),
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| format!("{k}: {}", format_raw(v)))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Open a file in the user's preferred editor, blocking until the editor exits.
///
/// The argument is a file reference resolved in the request's launch context
/// ([`parse_argument`] then `resolve_in_context`): malformed reference syntax
/// and a relative reference that climbs out of the launch repository are
/// refused before any file is created or editor launched. A reference that
/// matches no file names a new file at its first candidate location (the
/// launch directory for a relative path, the repository root for `&`, and so
/// on); a recursive (`%`) reference that matches nothing is an error. Creates
/// the file if it doesn't exist. After the editor exits, validates that the
/// file exists and is non-empty (after trimming whitespace). The fully
/// qualified path is the result.
pub fn run_edit(raw_file: &str, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let skeleton = EditData { file: None, editor: None, created: false };
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::typed(&skeleton))?;
    let result = edit_value(raw_file, request);
    run.finish(result)
}

fn edit_value(raw_file: &str, request: &MdRequest) -> Result<Completed<CommandValue<EditData>>> {
    let argument = std::path::Path::new(raw_file);
    let reference = parse_argument(argument)?;
    let context = request.launch_context()?;
    let path = match reference.resolve_in_context(context) {
        Ok(Some(path)) => path,
        Ok(None) if !reference.class().recursive => reference
            .candidate_plan(context)
            .map_err(|source| DocumentArgumentError::new(raw_file.to_string(), source))?
            .first()
            .map(|candidate| candidate.path().to_path_buf())
            .ok_or_else(|| DocumentArgumentError::no_match(raw_file.to_string()))?,
        Ok(None) => return Err(DocumentArgumentError::no_match(raw_file.to_string()).into()),
        Err(source) => return Err(DocumentArgumentError::new(raw_file.to_string(), source).into()),
    };

    // Before anything is created: a refused edit leaves nothing behind.
    let editor = darkmatter::editor::resolve_editor_command()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Create the file if it doesn't exist
    let created = !path.exists();
    if created {
        std::fs::write(&path, "")
            .wrap_err_with(|| format!("Failed to create file: {}", path.display()))?;
    }

    // --- Launch the editor ---
    // `canonicalize_simplified`: the printed path must not carry a Windows
    // verbatim `\\?\` prefix.
    let canonical = biscuit_file::canonicalize_simplified(&path)
        .wrap_err_with(|| format!("Failed to canonicalize path: {}", path.display()))?;
    let edited = edit_in_place(&canonical);
    if edited.is_err() && created {
        discard_if_empty(&canonical);
    }
    edited?;

    let shown = canonical.display().to_string();
    let terminal = shown.clone();
    let data = EditData {
        file: Some(biscuit_file::to_portable_string(&canonical)),
        // The command may carry arguments; the executable is its first word.
        editor: editor.split_whitespace().next().map(str::to_string),
        created,
    };
    let prose = raw_document(&shown);
    Ok(Completed::success(CommandValue::new(data, prose).with_terminal(move |_| terminal.clone())))
}

/// Runs the editor on `path` and judges what it left.
fn edit_in_place(path: &std::path::Path) -> Result<()> {
    darkmatter::editor::launch_editor_on_path(path)?;

    if !path.exists() {
        return Err(eyre!("File was deleted during editing: {}", path.display()));
    }

    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read file after editing: {}", path.display()))?;

    if content.trim().is_empty() {
        return Err(eyre!("File is empty after editing: {}", path.display()));
    }
    Ok(())
}

/// Removes a file this command created for an edit that failed, unless the
/// editor wrote something to it. The failure being reported outranks one here.
fn discard_if_empty(path: &std::path::Path) {
    if std::fs::read_to_string(path).is_ok_and(|content| content.trim().is_empty()) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;

    #[test]
    fn raw_values_keep_todays_layout() {
        assert_eq!(format_raw(&serde_json::json!(null)), "");
        assert_eq!(format_raw(&serde_json::json!("a")), "a");
        assert_eq!(format_raw(&serde_json::json!(["a", 1])), "a\n1");
        assert_eq!(format_raw(&serde_json::json!({ "k": "v" })), "k: v");
    }

    #[test]
    fn an_explicit_width_reflows_only_the_body() {
        let text = "---\ntitle: a long title that must stay as it is\n---\none two\nthree four\n".to_string();
        let joined = at_width(text.clone(), Some(Width::Unlimited));
        assert!(joined.starts_with("---\ntitle: a long title that must stay as it is\n---\n"), "{joined}");
        assert!(joined.contains("one two three four"), "{joined}");
        assert_eq!(at_width(text.clone(), None), text);
        assert_eq!(at_width(text.clone(), Some(Width::Source)), text);
    }

    #[test]
    fn the_summary_names_what_was_removed() {
        let terminal = Terminal::builder().width(200).color_depth(biscuit_terminal::discovery::detection::ColorDepth::None).build();
        let row = Prose::new(rm_summary(&["a".to_string()], &["b".to_string()])).render(&terminal);
        assert!(row.contains("removed the a property"), "{row}");
        assert!(row.contains("remaining: b"), "{row}");
    }
}
