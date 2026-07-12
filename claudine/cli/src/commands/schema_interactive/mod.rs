//! Interactive collection of schema-required properties (Phase 3).
//!
//! `CompositionError` is a large enum (several variants carry `PathBuf`
//! plus structured detail), and clippy flags `Result<_, CompositionError>`
//! returns as a "large `Err`" anti-pattern. The signatures in this
//! module match the existing CLI composition helpers (see
//! `commands/compose.rs`), so boxing the error here would force every
//! caller to unbox. Mirror the `compose.rs` allow-attribute instead.
#![allow(clippy::result_large_err)]
//!
//! When [`claudine::composition::prepare_direct_with_schema`] (or the
//! inline variant) returns [`CompositionError::MissingProperties`] and
//! [`InteractiveSchemaOptions::allowed`] resolves to `true`, the CLI
//! invokes [`collect_missing_values`] to drive a `biscuit-tui` prompt for
//! each missing property and produces a JSON override map that the
//! caller merges into the existing `--set` / shorthand overrides before
//! retrying preparation.
//!
//! The status report is rendered separately via [`render_status_report`]
//! (see the [`status`] submodule). Tests should assert on the structured
//! [`SchemaStatusReport`](claudine::composition::SchemaStatusReport) returned
//! by the library rather than the rendered terminal output.

use std::io;
use std::path::Path;

use biscuit_file::FileReference;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::{
    CompositionError, DroppedOptional, FileDetail, InteractiveSchemaOptions, InteractiveShape,
    MissingProperty, PreValidatedSchema, ResolvedCompositionSource, TextFormat,
    build_schema_status_report, extract_markdown_detail, pre_validate_schema,
};
use biscuit_tui::prelude::*;

use crate::completion::autocomplete_ui::{choose_many_files, choose_one_file, confirm_one_file};
use crate::completion::schema_completion::file_candidate_paths;
use crate::completion::scopes::{self, ScopeContext};
use crate::log;

mod status;

pub use status::render_status_report;
use status::escape_prose;

/// Resolve [`InteractiveSchemaOptions`] from the user config plus
/// current stdin/stderr TTY state and the `--silent` flag.
///
/// `prompt_for_missing` reads the user-scoped config (silently defaults
/// to `true` when the config cannot be loaded). The TTY flags are
/// detected against `std::io::stdin()` and `std::io::stderr()`.
pub fn resolve_interactive_options(silent: bool) -> InteractiveSchemaOptions {
    use std::io::IsTerminal;

    let prompt_for_missing = claudine::dispatch::loader::load_claudine_config(None, None)
        .ok()
        .map(|c| c.prompt_for_missing)
        .unwrap_or(true);

    InteractiveSchemaOptions {
        prompt_for_missing,
        stdin_is_tty: std::io::stdin().is_terminal(),
        stderr_is_tty: std::io::stderr().is_terminal(),
        silent,
    }
}

/// Pre-prepare schema validation with interactive collection of missing
/// required values when allowed.
///
/// Wraps [`pre_validate_schema`]. On `MissingProperties` and when
/// `interactive.allowed()` is `true`:
///
/// 1. Properties with no [`InteractiveShape`] short-circuit to
///    [`CompositionError::UnsupportedInteractiveSchema`] — matching the
///    direct `compose` path.
/// 2. The schema status report is rendered to `term`.
/// 3. [`collect_missing_values`] drives a `biscuit-tui` prompt per
///    missing property.
/// 4. Collected values are merged into `set_overrides` and validation
///    is re-run.
///
/// Use BEFORE the pre-flight phase so all schema-related errors surface
/// as typed [`CompositionError`] rather than Darkmatter's raw
/// `MarkdownError::SchemaValidationFailed`.
///
/// ## Errors
///
/// Returns the typed `CompositionError` from [`pre_validate_schema`] when
/// validation cannot be satisfied (including after interactive
/// collection). User cancellation of the prompt bubbles back as the
/// original `MissingProperties` error so the CLI shows the non-TTY
/// remediation block.
pub fn pre_validate_with_interactive_collection(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    interactive: InteractiveSchemaOptions,
    term: &Terminal,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<PreValidatedSchema, CompositionError> {
    let first = pre_validate_schema(source, set_overrides, file_ref_fallback_dir);
    let Err(err) = first else {
        return first;
    };

    // A provided `file(match)` partial that failed literal resolution is
    // resolved via a glob+substring confirmation dialog / chooser, symmetric to
    // the missing-property flow below.
    if matches!(err, CompositionError::UnresolvedFileReference { .. }) {
        return resolve_unresolved_file_reference(
            source,
            set_overrides,
            interactive,
            err,
            file_ref_fallback_dir,
        );
    }

    let CompositionError::MissingProperties {
        ref missing,
        ref source_path,
        ..
    } = err
    else {
        return Err(err);
    };

    if !interactive.allowed() {
        return Err(err);
    }

    if let Some(prop) = missing.iter().find(|p| p.interactive_shape.is_none()) {
        return Err(CompositionError::UnsupportedInteractiveSchema {
            source_path: source_path.clone(),
            property: prop.name.clone(),
            shape: prop
                .type_label
                .clone()
                .unwrap_or_else(|| "(unknown)".to_string()),
        });
    }

    if let Ok(Some(report)) = build_schema_status_report(source, set_overrides, file_ref_fallback_dir) {
        render_status_report(&report, term);
    }

    let collected = match collect_missing_values(missing) {
        Ok(map) => map,
        Err(_) => return Err(err),
    };
    if collected.is_empty() {
        return Err(err);
    }

    let merged = merge_overrides(set_overrides, collected);
    pre_validate_schema(source, Some(&serde_json::Value::Object(merged)), file_ref_fallback_dir)
}

/// Resolve a [`CompositionError::UnresolvedFileReference`] by treating the
/// provided value as a partial: walk the property's `match(...)` glob from the
/// launch area, filter by the provided substring, and drive a confirmation
/// dialog (single match) or chooser (multiple). On selection, rewrite the
/// property override and re-run [`pre_validate_schema`] once.
///
/// The original error is preserved unchanged when the session is not
/// interactive, when zero candidates match the partial, or when the user
/// declines / cancels — so the caller renders the actionable "no existing file
/// matched reference" text just as before.
fn resolve_unresolved_file_reference(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    interactive: InteractiveSchemaOptions,
    err: CompositionError,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<PreValidatedSchema, CompositionError> {
    let CompositionError::UnresolvedFileReference {
        ref property,
        ref provided,
        ref patterns,
        is_array,
        ..
    } = err
    else {
        return Err(err);
    };

    // Non-interactive → keep the pre-existing schema-validation surface so
    // scripts / CI output stay byte-identical to before this feature.
    if !interactive.allowed() {
        return Err(downgrade_to_schema_validation(err));
    }

    let resolved = match resolve_provided_file_reference(property, provided, patterns, is_array) {
        Ok(Some(value)) => value,
        // Zero glob+substring matches, the user declined / cancelled, or the
        // chooser failed → fall back to the original schema-validation error,
        // preserving the actionable "no existing file matched reference" text.
        Ok(None) | Err(_) => return Err(downgrade_to_schema_validation(err)),
    };

    let mut collected = serde_json::Map::new();
    collected.insert(property.clone(), resolved);
    let merged = merge_overrides(set_overrides, collected);
    pre_validate_schema(
        source,
        Some(&serde_json::Value::Object(merged)),
        file_ref_fallback_dir,
    )
}

/// Convert an [`CompositionError::UnresolvedFileReference`] back to the
/// equivalent [`CompositionError::SchemaValidation`] it was classified from.
///
/// `UnresolvedFileReference` is an internal signal that lets the CLI offer an
/// interactive glob+substring resolution. When that resolution is unavailable
/// (non-interactive) or unsuccessful (zero matches / declined), the user must
/// see exactly the schema-validation surface they saw before this feature — the
/// reconstruction mirrors the lib's `build_schema_validation_error` for a single
/// problem (`"/{property}: {reason}"`).
fn downgrade_to_schema_validation(err: CompositionError) -> CompositionError {
    match err {
        CompositionError::UnresolvedFileReference {
            source_path,
            property,
            reason,
            ..
        } => {
            let pointer = format!("/{property}");
            CompositionError::SchemaValidation {
                source_path,
                message: format!("{pointer}: {reason}"),
                problems: vec![pointer],
            }
        }
        other => other,
    }
}

/// Walk a `file(match)` property's glob candidates, filter by the provided
/// partial, and drive the confirmation dialog / chooser.
///
/// Returns `Ok(None)` on zero matches or user cancellation/decline. The
/// resolved value is an absolute path string, array-wrapped for `file[]`.
fn resolve_provided_file_reference(
    property: &str,
    provided: &str,
    patterns: &[String],
    is_array: bool,
) -> io::Result<Option<serde_json::Value>> {
    let ctx = ScopeContext::discover();
    let candidates = provided_partial_candidates(patterns, provided, &ctx);
    if candidates.is_empty() {
        return Ok(None);
    }

    let options: Vec<ChoiceOption<FileDetail>> = candidates
        .iter()
        .map(|path| {
            let detail = extract_file_detail(path);
            // As in `collect_file`: `match(...)` candidates routinely share a
            // basename, so the visible label is the cwd/repo-relative path.
            let label = path_label(path, &ctx);
            ChoiceOption::new(label.clone(), label, detail)
        })
        .collect();

    let term = crate::log::terminal();
    let intro = format!(
        "The value provided for {property} did not match a file directly; \
         resolving `{provided}` against the closest candidate:",
    );
    eprintln!("{}", Prose::new(intro).render(&term));

    let selected = match options.len() {
        1 => {
            let detail = &options[0].value;
            if confirm_one_file(detail)? {
                Some(detail.clone())
            } else {
                None
            }
        }
        _ => choose_one_file(options)?,
    };

    let Some(detail) = selected else {
        return Ok(None);
    };
    let value = resolve_file_value(&detail.path)?;
    if is_array {
        Ok(Some(serde_json::Value::Array(vec![value])))
    } else {
        Ok(Some(value))
    }
}

/// Glob candidates filtered by the provided partial substring.
///
/// The testable core of [`resolve_provided_file_reference`]: walks the
/// `match(...)` patterns from the launch area
/// ([`scopes::property_value_root`]) and retains candidates whose path contains
/// `provided` (case-insensitive), sorted for stable ordering.
fn provided_partial_candidates(
    patterns: &[String],
    provided: &str,
    ctx: &ScopeContext,
) -> Vec<std::path::PathBuf> {
    let mut candidates = file_candidate_paths(patterns, ctx);
    let needle = provided.to_ascii_lowercase();
    candidates.retain(|path| scopes::path_matches_query(path, &needle));
    candidates.sort();
    candidates
}

fn merge_overrides(
    base: Option<&serde_json::Value>,
    collected: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut out = match base {
        Some(serde_json::Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    for (k, v) in collected {
        out.insert(k, v);
    }
    out
}

/// Emit a user-visible stderr warning per dropped optional property.
///
/// Schema validation silently elides optional properties whose value
/// fails validation so the run can continue; without a CLI-level warning
/// users never learn that their supplied value was removed. This
/// surfaces each drop via [`log::warn`] (yellow, always-on) so the
/// behavior is auditable on every run.
pub fn emit_dropped_optional_warnings(drops: &[DroppedOptional]) {
    for drop in drops {
        log::warn(&format!(
            "dropped optional schema property `{property}` (from {source}, at {stage}): {reason}",
            property = drop.property,
            source = drop.source.label(),
            stage = drop.stage.label(),
            reason = drop.reason,
        ));
    }
}

/// Drive `biscuit-tui` prompts for each missing required property and
/// return the collected values as a JSON object.
///
/// ## Errors
///
/// Returns an `io::Error` when the user cancels the prompt (Ctrl-C or
/// Esc) or when a property has no [`InteractiveShape`] mapping (raw
/// JSON Schema, `object`, `any`, property-level union). The caller is
/// expected to surface these as a typed
/// [`CompositionError::UnsupportedInteractiveSchema`][unsupported] or
/// fall through to the non-interactive `MissingProperties` report.
///
/// [unsupported]: claudine::composition::CompositionError::UnsupportedInteractiveSchema
pub fn collect_missing_values(
    missing: &[MissingProperty],
) -> io::Result<serde_json::Map<String, serde_json::Value>> {
    let mut out = serde_json::Map::new();
    for prop in missing {
        let Some(shape) = prop.interactive_shape.as_ref() else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "property `{name}` cannot be collected interactively",
                    name = prop.name
                ),
            ));
        };
        let value = prompt_for_property(prop, shape)?;
        out.insert(prop.name.clone(), value);
    }
    Ok(out)
}

fn prompt_for_property(
    prop: &MissingProperty,
    shape: &InteractiveShape,
) -> io::Result<serde_json::Value> {
    let term = crate::log::terminal();

    match shape {
        InteractiveShape::Boolean => {
            render_intro(prop, "boolean", &term);
            let label = Label::new(prop.name.clone(), LabelPosition::Above);
            let state = BooleanSwitchState::new().with_label(label);
            let value: bool = run_standalone(BooleanSwitch::new(), state, inline_height(2))?;
            Ok(serde_json::Value::Bool(value))
        }
        InteractiveShape::EnumOne { members } => {
            render_intro(prop, "enum", &term);
            let label = Label::new(prop.name.clone(), LabelPosition::Above);
            let options: Vec<ChoiceOption<String>> = members
                .iter()
                .map(|m| ChoiceOption::new(m.as_str(), m.as_str(), m.clone()))
                .collect();
            let state = ChooseOneState::from_options(options).with_label(label);
            let selected: Option<String> = run_standalone(ChooseOne::new(), state, inline_height(8))?;
            match selected {
                Some(s) => Ok(serde_json::Value::String(s)),
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "no value selected",
                )),
            }
        }
        InteractiveShape::EnumMany { members } => {
            render_intro(prop, "enum", &term);
            let label = Label::new(prop.name.clone(), LabelPosition::Above);
            let options: Vec<ChoiceOption<String>> = members
                .iter()
                .map(|m| ChoiceOption::new(m.as_str(), m.as_str(), m.clone()))
                .collect();
            let state = ChooseManyState::from_options(options).with_label(label);
            let selected: Vec<String> = run_standalone(ChooseMany::new(), state, inline_height(8))?;
            Ok(serde_json::Value::Array(
                selected.into_iter().map(serde_json::Value::String).collect(),
            ))
        }
        InteractiveShape::Number { integer, min, max } => {
            render_intro(prop, "number", &term);
            collect_number(prop, *integer, *min, *max)
        }
        InteractiveShape::Text { format, min_len, max_len } => {
            render_intro(prop, "string", &term);
            collect_text(prop, *format, *min_len, *max_len)
        }
        InteractiveShape::File { is_array, patterns } => {
            collect_file(prop, *is_array, patterns)
        }
    }
}

fn render_intro(prop: &MissingProperty, kind: &str, term: &Terminal) {
    let message = match kind {
        "string" => format!(
            "The {} requires a string value; please input a value to continue:",
            prop.name
        ),
        "number" => format!(
            "The {} requires a numeric value; please input a value to continue:",
            prop.name
        ),
        "boolean" => format!("The {} requires a boolean value:", prop.name),
        _ => format!("Please provide a value for {}:", prop.name),
    };
    let mut prose = Prose::new(message);
    if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
        prose = Prose::new(format!("{}\n\n<i><dim>{}</dim></i>", prose.content(), escape_prose(desc)));
    }
    eprintln!("{}", prose.render(term));
}

fn collect_text(
    prop: &MissingProperty,
    _format: TextFormat,
    min_len: Option<usize>,
    max_len: Option<usize>,
) -> io::Result<serde_json::Value> {
    let label = Label::new(prop.name.clone(), LabelPosition::Above);
    let mut state = TextInputState::new().with_label(label);
    if let Some(max) = max_len {
        state = state.with_max_length(max);
    }
    let value: String = run_standalone(TextInput::new(), state, inline_height(2))?;
    if let Some(min) = min_len
        && value.chars().count() < min
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("value must be at least {min} characters"),
        ));
    }
    Ok(serde_json::Value::String(value))
}

/// Run a text input, parsing into a JSON number. On parse failure
/// re-prompt with an inline validation error and keep the previous
/// buffer contents so the user can correct the value.
fn collect_number(
    prop: &MissingProperty,
    integer: bool,
    min: Option<f64>,
    max: Option<f64>,
) -> io::Result<serde_json::Value> {
    let label = Label::new(prop.name.clone(), LabelPosition::Above);
    let mut previous: Option<(String, String)> = None;
    loop {
        let mut state = TextInputState::new().with_label(label.clone());
        if let Some((buf, err)) = previous.take() {
            state = state.with_value(&buf);
            state.set_validation_error(err);
        }
        // Height 4 (label, input, error, hint) keeps the inline viewport's
        // bottom-row help-hint overlay off the validation-error row
        // (`inner_area.y + 1`). At height 3 the hint and error collide on the
        // last row and the hint wins, hiding the parse error on a retry.
        let raw: String = run_standalone(TextInput::new(), state, inline_height(4))?;
        match parse_number(&raw, integer, min, max) {
            Ok(value) => return Ok(value),
            Err(message) => {
                previous = Some((raw, message));
            }
        }
    }
}

fn parse_number(
    raw: &str,
    integer: bool,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("value required".to_string());
    }
    let value = if integer {
        if let Ok(n) = trimmed.parse::<i64>() {
            serde_json::Value::Number(serde_json::Number::from(n))
        } else if let Ok(n) = trimmed.parse::<f64>()
            && n.fract() == 0.0
            && n.is_finite()
            && let Some(num) = serde_json::Number::from_f64(n)
        {
            serde_json::Value::Number(num)
        } else {
            return Err(format!("`{trimmed}` is not a valid integer"));
        }
    } else {
        if let Ok(n) = trimmed.parse::<i64>() {
            serde_json::Value::Number(serde_json::Number::from(n))
        } else if let Ok(n) = trimmed.parse::<f64>()
            && let Some(num) = serde_json::Number::from_f64(n)
        {
            serde_json::Value::Number(num)
        } else {
            return Err(format!("`{trimmed}` is not a valid number"));
        }
    };

    let float_value = value.as_f64().unwrap_or(f64::NAN);
    if let Some(lo) = min
        && float_value < lo
    {
        return Err(format!("value must be at least {lo}"));
    }
    if let Some(hi) = max
        && float_value > hi
    {
        return Err(format!("value must be at most {hi}"));
    }
    Ok(value)
}

fn inline_height(rows: u16) -> Option<HeightSpec> {
    Some(HeightSpec::Cells(rows))
}

fn collect_file(
    prop: &MissingProperty,
    is_array: bool,
    patterns: &[String],
) -> io::Result<serde_json::Value> {
    let ctx = ScopeContext::discover();
    let mut paths = file_candidate_paths(patterns, &ctx);

    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no files available for `{}`", prop.name),
        ));
    }

    paths.sort();
    let options: Vec<ChoiceOption<FileDetail>> = paths
        .into_iter()
        .map(|path| {
            let detail = extract_file_detail(&path);
            // Display the repo/cwd-relative path (with extension), not
            // `detail.name`: `match(...)` candidates routinely share a basename
            // (every `**/*spec*.md` hit is `spec`), so the bare name leaves the
            // choices indistinguishable. The path is both the option id and the
            // visible label; the detail pane carries the rest.
            let label = path_label(&path, &ctx);
            ChoiceOption::new(label.clone(), label, detail)
        })
        .collect();

    let term = crate::log::terminal();
    let intro = format!(
        "The {} requires a valid file reference; choose from the files below:",
        prop.name
    );
    eprintln!("{}", Prose::new(intro).render(&term));
    if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
        eprintln!(
            "{}",
            Prose::new(format!("<i><dim>{}</dim></i>", escape_prose(desc))).render(&term)
        );
    }

    let selected = if is_array {
        let selected = choose_many_files(options)?;
        if selected.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no files selected",
            ));
        }
        selected
            .into_iter()
            .map(|d| resolve_file_value(&d.path))
            .collect::<io::Result<Vec<_>>>()?
    } else {
        let selected = choose_one_file(options)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "no file selected")
        })?;
        vec![resolve_file_value(&selected.path)?]
    };

    if is_array {
        Ok(serde_json::Value::Array(selected))
    } else {
        Ok(selected.into_iter().next().unwrap_or(serde_json::Value::Null))
    }
}

fn extract_file_detail(path: &Path) -> FileDetail {
    if let Some(ext) = path.extension().and_then(|e| e.to_str())
        && matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown")
    {
        return extract_markdown_detail(path, "FILE");
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string());
    FileDetail {
        badge: "FILE".to_string(),
        name,
        path: path.to_path_buf(),
        description: None,
        schema_lines: Vec::new(),
        has_custom_name: false,
    }
}

fn path_label(path: &Path, ctx: &ScopeContext) -> String {
    if let Some(root) = crate::completion::scopes::effective_repo_root(ctx)
        && let Ok(rel) = path.strip_prefix(root)
        && let Some(rel_str) = rel.to_str()
        && !rel_str.is_empty()
    {
        return rel_str.to_string();
    }
    if let Some(home) = &ctx.home
        && path.starts_with(home)
        && let Ok(rel) = path.strip_prefix(home)
        && let Some(rel_str) = rel.to_str()
        && !rel_str.is_empty()
    {
        return format!("~/{rel_str}");
    }
    path.display().to_string()
}

fn resolve_file_value(path: &Path) -> io::Result<serde_json::Value> {
    let reference = FileReference::new(path.to_str().unwrap_or(""))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let resolved = reference
        .resolve()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("file not found: {}", path.display()),
            )
        })?;
    Ok(serde_json::Value::String(resolved.display().to_string()))
}

#[allow(dead_code)]
fn format_label(prop: &MissingProperty) -> String {
    let type_part = prop
        .type_label
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(|t| format!(" ({t})"))
        .unwrap_or_default();
    let mut out = format!("{name}{type_part}", name = prop.name);
    if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
        out.push_str(" — ");
        out.push_str(desc);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::status::{description_suffix, render_optional_line, render_required_line};
    use super::*;
    use claudine::composition::{PropertyState, PropertyStatus};

    fn missing_with_shape(name: &str, shape: InteractiveShape) -> MissingProperty {
        MissingProperty {
            name: name.to_string(),
            type_label: Some("string".to_string()),
            description: None,
            interactive_shape: Some(shape),
        }
    }

    #[test]
    fn collect_missing_values_rejects_unsupported_shape() {
        let missing = vec![MissingProperty {
            name: "config".to_string(),
            type_label: Some("object".to_string()),
            description: None,
            interactive_shape: None,
        }];
        let err = collect_missing_values(&missing).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        assert!(err.to_string().contains("config"));
    }

    #[test]
    fn parse_number_accepts_integer() {
        let value = parse_number("42", false, None, None).unwrap();
        assert_eq!(value, serde_json::json!(42));
    }

    #[test]
    fn parse_number_accepts_float() {
        let value = parse_number("3.14", false, None, None).unwrap();
        assert!(value.as_f64().is_some());
    }

    #[test]
    fn parse_number_rejects_non_numeric() {
        assert!(parse_number("hello", false, None, None).is_err());
    }

    #[test]
    fn parse_number_integer_mode_rejects_non_integer() {
        assert!(parse_number("3.14", true, None, None).is_err());
    }

    #[test]
    fn parse_number_integer_mode_accepts_whole_float() {
        let value = parse_number("3.0", true, None, None).unwrap();
        assert!(value.as_i64().is_some() || value.as_f64().is_some());
    }

    #[test]
    fn parse_number_rejects_empty() {
        assert!(parse_number("   ", false, None, None).is_err());
    }

    #[test]
    fn parse_number_enforces_minimum() {
        assert!(parse_number("5", false, Some(10.0), None).is_err());
        assert_eq!(parse_number("10", false, Some(10.0), None).unwrap(), serde_json::json!(10));
    }

    #[test]
    fn parse_number_enforces_maximum() {
        assert!(parse_number("15", false, None, Some(10.0)).is_err());
        assert_eq!(parse_number("10", false, None, Some(10.0)).unwrap(), serde_json::json!(10));
    }

    #[test]
    fn format_label_combines_name_type_and_description() {
        let prop = missing_with_shape(
            "tier",
            InteractiveShape::Text {
                format: TextFormat::Plain,
                min_len: None,
                max_len: None,
            },
        );
        let label = format_label(&prop);
        assert!(label.starts_with("tier"));
        assert!(label.contains("string"));
    }

    #[test]
    fn format_label_includes_description_when_present() {
        let prop = MissingProperty {
            name: "tier".to_string(),
            type_label: Some("enum(a|b)".to_string()),
            description: Some("the tier to use".to_string()),
            interactive_shape: Some(InteractiveShape::EnumOne {
                members: vec!["a".into(), "b".into()],
            }),
        };
        let label = format_label(&prop);
        assert!(label.contains("the tier to use"));
    }

    #[test]
    fn render_required_line_includes_property_name_and_type() {
        let status = PropertyStatus {
            name: "title".to_string(),
            type_label: "string".to_string(),
            description: None,
            state: PropertyState::Missing,
        };
        let line = render_required_line(&status);
        assert!(line.contains("title"));
        assert!(line.contains("string"));
        assert!(line.contains("required"));
    }

    #[test]
    fn render_required_line_marks_invalid_with_wrong_type_message() {
        let status = PropertyStatus {
            name: "count".to_string(),
            type_label: "number".to_string(),
            description: None,
            state: PropertyState::Invalid,
        };
        let line = render_required_line(&status);
        assert!(line.contains("wrong type"));
    }

    #[test]
    fn render_optional_line_marks_valid_with_dim_styling() {
        let status = PropertyStatus {
            name: "description".to_string(),
            type_label: "string".to_string(),
            description: None,
            state: PropertyState::Valid,
        };
        let line = render_optional_line(&status);
        assert!(line.contains("<dim>"));
        assert!(line.contains("<green>"));
    }

    fn seed_spec_tree(root: &Path) {
        use std::fs;
        fs::create_dir_all(root.join(".git")).unwrap();
        let target = root.join("features/2026-06-30-style-everywhere/spec.md");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "---\ntitle: Everywhere\n---\nbody\n").unwrap();
        let other = root.join("features/2026-06-01-other/spec.md");
        fs::create_dir_all(other.parent().unwrap()).unwrap();
        fs::write(&other, "---\ntitle: Other\n---\nbody\n").unwrap();
    }

    #[test]
    fn provided_partial_candidates_filters_to_single_substring_match() {
        let tmp = tempfile::TempDir::new().unwrap();
        seed_spec_tree(tmp.path());

        let ctx = ScopeContext::discover_from(tmp.path());
        let patterns = vec!["**/*spec*.md".to_string()];
        let got = provided_partial_candidates(&patterns, "everywhere", &ctx);
        assert_eq!(got.len(), 1, "expected exactly one substring match: {got:?}");
        assert!(
            got[0].ends_with("2026-06-30-style-everywhere/spec.md"),
            "expected the style-everywhere spec, got {got:?}",
        );
    }

    #[test]
    fn provided_partial_candidates_is_case_insensitive() {
        let tmp = tempfile::TempDir::new().unwrap();
        seed_spec_tree(tmp.path());

        let ctx = ScopeContext::discover_from(tmp.path());
        let patterns = vec!["**/*spec*.md".to_string()];
        let got = provided_partial_candidates(&patterns, "EVERYWHERE", &ctx);
        assert_eq!(got.len(), 1, "case-insensitive substring match: {got:?}");
    }

    #[test]
    fn provided_partial_candidates_zero_matches_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        seed_spec_tree(tmp.path());

        let ctx = ScopeContext::discover_from(tmp.path());
        let patterns = vec!["**/*spec*.md".to_string()];
        let got = provided_partial_candidates(&patterns, "no-such-partial", &ctx);
        assert!(got.is_empty(), "expected zero matches: {got:?}");
    }

    #[test]
    fn merge_overrides_combines_base_and_collected() {
        let base = serde_json::json!({ "a": "1" });
        let mut collected = serde_json::Map::new();
        collected.insert("b".to_string(), serde_json::json!("2"));
        let merged = merge_overrides(Some(&base), collected);
        assert_eq!(merged.get("a"), Some(&serde_json::json!("1")));
        assert_eq!(merged.get("b"), Some(&serde_json::json!("2")));
    }

    #[test]
    fn pre_validate_with_interactive_returns_missing_when_not_allowed() {
        use std::fs;
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(
            &file,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        )
        .unwrap();
        let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
            .unwrap();

        // Build interactive options that DENY (default: all false).
        let interactive = InteractiveSchemaOptions::default();
        assert!(!interactive.allowed());

        let term = Terminal::default();
        let err = pre_validate_with_interactive_collection(&source, None, interactive, &term, None)
            .unwrap_err();
        assert!(
            matches!(err, CompositionError::MissingProperties { .. }),
            "expected MissingProperties when interactive not allowed: {err:?}"
        );
    }

    #[test]
    fn pre_validate_with_interactive_returns_missing_for_file_property_when_not_allowed() {
        use std::fs;
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(
            &file,
            "---\n$schema:\n  cover: 'file(required)'\n---\nbody\n",
        )
        .unwrap();
        let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
            .unwrap();

        // Non-TTY options deny prompting, so a missing `file` property must
        // still surface as MissingProperties rather than trying to drive a
        // chooser.
        let interactive = InteractiveSchemaOptions::default();
        assert!(!interactive.allowed());

        let term = Terminal::default();
        let err = pre_validate_with_interactive_collection(&source, None, interactive, &term, None)
            .unwrap_err();
        assert!(
            matches!(err, CompositionError::MissingProperties { .. }),
            "expected MissingProperties for file property when not interactive: {err:?}"
        );
    }

    #[test]
    fn pre_validate_with_interactive_succeeds_when_overrides_supply_value() {
        use std::fs;
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(
            &file,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        )
        .unwrap();
        let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
            .unwrap();

        let overrides = serde_json::json!({ "title": "Plan" });
        let term = Terminal::default();
        let pre = pre_validate_with_interactive_collection(
            &source,
            Some(&overrides),
            InteractiveSchemaOptions::default(),
            &term,
            None,
        )
        .unwrap();
        let fm = pre.set_overrides.unwrap();
        assert_eq!(
            fm.get("title").and_then(|v| v.as_str()),
            Some("Plan")
        );
    }

    #[test]
    fn pre_validate_with_interactive_returns_unsupported_for_object_shape() {
        use std::fs;
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(
            &file,
            "---\n$schema:\n  config: 'object(required)'\n---\nbody\n",
        )
        .unwrap();
        let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
            .unwrap();

        // Allow interactive so the helper attempts to enter the loop.
        let interactive = InteractiveSchemaOptions {
            prompt_for_missing: true,
            stdin_is_tty: true,
            stderr_is_tty: true,
            silent: false,
        };
        assert!(interactive.allowed());

        let term = Terminal::default();
        let err = pre_validate_with_interactive_collection(&source, None, interactive, &term, None)
            .unwrap_err();
        assert!(
            matches!(err, CompositionError::UnsupportedInteractiveSchema { .. }),
            "expected UnsupportedInteractiveSchema: {err:?}"
        );
    }

    #[test]
    fn merge_overrides_collected_wins_over_base() {
        let base = serde_json::json!({ "k": "old" });
        let mut collected = serde_json::Map::new();
        collected.insert("k".to_string(), serde_json::json!("new"));
        let merged = merge_overrides(Some(&base), collected);
        assert_eq!(merged.get("k"), Some(&serde_json::json!("new")));
    }

    #[test]
    fn description_suffix_handles_empty() {
        assert!(description_suffix(None).is_empty());
        assert!(description_suffix(Some("")).is_empty());
        assert!(description_suffix(Some("   ")).is_empty());
        assert!(description_suffix(Some("hello")).contains("hello"));
    }
}
