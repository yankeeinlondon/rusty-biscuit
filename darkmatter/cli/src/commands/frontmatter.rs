//! `md get` / `md set` / `md rm` / `md edit` frontmatter subcommand implementations.

use crate::args::Cli;
use crate::commands::render::document_value;
use crate::io::{DocumentArgumentError, load_markdown, parse_argument, resolve_file_path};
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use clap::CommandFactory as _;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::{reflow_to_width, strip_incidental_newlines};
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::output::command::{CommandValue, EditData, RemovedData, SetData, prose_document};
use renderable::Width;
use renderable::tree::{Document, RenderNode};
use serde_json::Value;
use std::path::PathBuf;
use tracing::instrument;

/// Get frontmatter properties from a markdown document.
///
/// The value prints no prose, so `--width` changes nothing here.
#[instrument(skip_all)]
pub fn run_get(input: &PathBuf, props: &[String], output: OutputTarget, compact: bool, cli: &Cli, request: &MdRequest) -> Result<()> {
    if compact && !matches!(output, OutputTarget::Json | OutputTarget::Json5) {
        return Err(Cli::command()
            .error(clap::error::ErrorKind::ArgumentConflict, "`--compact` is valid only with `--output json` or `--output json5`")
            .into());
    }
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Bare)?.compact_json(compact);
    let result = load_markdown(Some(input), request).map(|md| {
        let value = requested_value(&md, props);
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

/// One property's value, or an object of the requested keys; a missing key is `""`.
fn requested_value(md: &Markdown, props: &[String]) -> Value {
    let map = md.frontmatter().as_map();
    let value_of = |prop: &String| map.get(prop).cloned().unwrap_or_else(|| Value::String(String::new()));
    match props {
        [prop] => value_of(prop),
        _ => Value::Object(props.iter().map(|prop| (prop.clone(), value_of(prop))).collect()),
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

/// Set a frontmatter property on a markdown document.
///
/// By default the modified document is the deliverable and the source file is
/// not changed. With `--save` the file is updated in place, and a formatted
/// output prints nothing; a data output always reports the result.
#[instrument(skip_all)]
pub fn run_set(input: &PathBuf, prop: &str, raw_value: &str, save: bool, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let file = input_label(input);
    let skeleton = SetData { file: file.clone(), property: prop.to_string(), value: Value::Null, document: None, saved: false };
    let run = CommandRun::begin(output, cli.width, WidthRequest::Source, cli.verbose, DataShape::typed(&skeleton))?.exact_markdown();
    let result = set_value(input, skeleton, raw_value, save, &run, cli, request);
    run.finish(result)
}

fn set_value(
    input: &PathBuf,
    mut data: SetData,
    raw_value: &str,
    save: bool,
    run: &CommandRun,
    cli: &Cli,
    request: &MdRequest,
) -> Result<Completed<CommandValue<SetData>>> {
    if save && data.file.is_none() {
        return Err(eyre!("--save requires an input file path (stdin is not supported)"));
    }

    let mut md = load_markdown(Some(input), request)?;

    let value: Value = serde_json::from_str(raw_value).unwrap_or_else(|_| Value::String(raw_value.to_string()));
    data.value = value.clone();

    md.fm_insert(&data.property, value).map_err(|e| eyre!("Failed to set frontmatter property: {e}"))?;
    let output = run.output();
    let width = output.explicit_width.then_some(output.options.width);
    let text = at_width(edited_text(&md, &[data.property.as_str()]), width);
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
    let path = resolved_input(input, request);
    document_value(data, &md, text, path.as_ref(), output, cli).map(Completed::success)
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

/// Remove one or more frontmatter properties from a markdown document.
///
/// Saves the file in place. The terminal outputs print nothing on success; `-v`
/// adds a summary on stderr. An explicit Markdown or HTML output, and any data
/// output, reports the result.
#[instrument(skip_all)]
pub fn run_rm(input: &PathBuf, props: &[String], output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Object)?;
    let result = rm_value(input, props, &run, cli, request);
    run.finish(result)
}

fn rm_value(input: &PathBuf, props: &[String], run: &CommandRun, cli: &Cli, request: &MdRequest) -> Result<Completed<CommandValue<RemovedData>>> {
    let resolved = resolve_file_path(input, request.launch_context()?)?;
    let mut md = load_markdown(Some(input), request)?;
    let fm = md.frontmatter_mut().as_map_mut();

    let mut removed = Vec::new();
    let mut not_found = Vec::new();

    for prop in props {
        if fm.shift_remove(prop).is_some() {
            removed.push(prop.clone());
        } else {
            not_found.push(prop.clone());
        }
    }

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

    let removed_props: Vec<&str> = removed.iter().map(String::as_str).collect();
    std::fs::write(&resolved, edited_text(&md, &removed_props))
        .wrap_err_with(|| format!("Failed to write to {:?}", resolved))?;

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

/// The document text after `md set` / `md rm` changed `properties`.
///
/// Only those top-level properties are rewritten; every other byte of the
/// loaded source (indentation, comments, quoting, line endings, the body) is
/// kept. When the text-preserving editor refuses the source (for example an
/// indented root mapping), the whole document is re-serialized instead, which
/// is what these commands always did before.
fn edited_text(md: &darkmatter::markdown::Markdown, properties: &[&str]) -> String {
    let edited = md.as_string();
    md.source_text()
        .and_then(|source| {
            darkmatter::markdown::hash::restore_properties_text(source, &edited, properties).ok()
        })
        .map_or(edited, |restored| restored.text)
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
