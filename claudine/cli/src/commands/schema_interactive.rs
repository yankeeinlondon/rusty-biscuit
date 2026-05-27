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
//! The status report is rendered separately via [`render_status_report`].
//! Tests should assert on the structured [`SchemaStatusReport`] returned
//! by the library rather than the rendered terminal output.

use std::io;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::{
    CompositionError, InteractiveSchemaOptions, InteractiveShape, MissingProperty, PrepareOptions,
    PreparedComposition, PropertyState, PropertyStatus, ResolvedCompositionSource,
    SchemaStatusReport, TextFormat, build_schema_status_report, prepare_direct_with_schema,
    prepare_inline_with_schema,
};
use tui_chrome::prelude::*;

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

fn render_required_line(status: &PropertyStatus) -> String {
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

fn render_optional_line(status: &PropertyStatus) -> String {
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

fn description_suffix(description: Option<&str>) -> String {
    match description.filter(|d| !d.trim().is_empty()) {
        Some(desc) => format!(" <i><dim>— {}</dim></i>", escape_prose(desc)),
        None => String::new(),
    }
}

fn escape_prose(input: &str) -> String {
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

/// Composition mode selector for [`prepare_with_interactive_collection`].
///
/// Mirrors the two sites in `compose.rs` that call either
/// [`composition::prepare_direct`] or [`composition::prepare_inline`],
/// so this helper can be shared between both subcommands.
#[derive(Clone, Copy)]
pub enum InteractivePrepareMode {
    /// Wrap [`prepare_direct_with_schema`].
    Direct,
    /// Wrap [`prepare_inline_with_schema`].
    Inline,
}

/// Run `prepare_direct_with_schema` / `prepare_inline_with_schema` and, on
/// [`CompositionError::MissingProperties`], drive the interactive
/// collection loop (when [`InteractiveSchemaOptions::allowed`] permits)
/// then retry with the collected values merged into `options.set_overrides`.
///
/// When `interactive` does not allow prompting, the typed
/// `MissingProperties` error is returned unchanged so the caller can
/// render the non-TTY status block.
///
/// ## Errors
///
/// Returns any [`CompositionError`] that the underlying prepare call
/// surfaces. User cancellation during interactive collection (`Ctrl-C`
/// or `Esc`) is mapped to the original `MissingProperties` error so the
/// CLI exits with a clear "missing required value" report.
pub fn prepare_with_interactive_collection(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: InteractivePrepareMode,
    interactive: InteractiveSchemaOptions,
    term: &Terminal,
) -> Result<PreparedComposition, CompositionError> {
    let first_attempt = run_prepare(source, options.clone(), mode);
    let Err(err) = first_attempt else {
        return first_attempt;
    };

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

    // Any unsupported shape short-circuits: prompt would fail, so we
    // promote the first such property to `UnsupportedInteractiveSchema`
    // so the user sees an actionable hint.
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

    // Render the status report once before driving prompts.
    if let Ok(Some(report)) =
        build_schema_status_report(source, options.set_overrides.as_ref())
    {
        render_status_report(&report, term);
    }

    let collected = match collect_missing_values(missing) {
        Ok(map) => map,
        Err(_) => {
            // Ctrl-C / Esc / unsupported shape — bubble the original
            // missing-properties error so the user sees the non-TTY
            // remediation block.
            return Err(err);
        }
    };

    if collected.is_empty() {
        return Err(err);
    }

    let merged_overrides = merge_overrides(options.set_overrides.as_ref(), collected);
    let mut retry_options = options;
    retry_options.set_overrides = Some(serde_json::Value::Object(merged_overrides));
    run_prepare(source, retry_options, mode)
}

fn run_prepare(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: InteractivePrepareMode,
) -> Result<PreparedComposition, CompositionError> {
    match mode {
        InteractivePrepareMode::Direct => prepare_direct_with_schema(source, options),
        InteractivePrepareMode::Inline => prepare_inline_with_schema(source, options),
    }
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
    let label_text = format_label(prop);
    let label = Label::new(label_text, LabelPosition::Above);

    match shape {
        InteractiveShape::Boolean => {
            let state = BooleanSwitchState::new().with_label(label);
            let value: bool = run_standalone(BooleanSwitch::new(), state, None)?;
            Ok(serde_json::Value::Bool(value))
        }
        InteractiveShape::EnumOne { members } => {
            let options: Vec<ChoiceOption<String>> = members
                .iter()
                .map(|m| ChoiceOption::new(m.as_str(), m.as_str(), m.clone()))
                .collect();
            let state = ChooseOneState::from_options(options).with_label(label);
            let selected: Option<String> = run_standalone(ChooseOne::new(), state, None)?;
            match selected {
                Some(s) => Ok(serde_json::Value::String(s)),
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "no value selected",
                )),
            }
        }
        InteractiveShape::EnumMany { members } => {
            let options: Vec<ChoiceOption<String>> = members
                .iter()
                .map(|m| ChoiceOption::new(m.as_str(), m.as_str(), m.clone()))
                .collect();
            let state = ChooseManyState::from_options(options).with_label(label);
            let selected: Vec<String> = run_standalone(ChooseMany::new(), state, None)?;
            Ok(serde_json::Value::Array(
                selected.into_iter().map(serde_json::Value::String).collect(),
            ))
        }
        InteractiveShape::Number { integer } => collect_number(label, *integer),
        InteractiveShape::Text { format } => collect_text(label, *format),
    }
}

fn collect_text(label: Label, _format: TextFormat) -> io::Result<serde_json::Value> {
    let state = TextInputState::new().with_label(label);
    let value: String = run_standalone(TextInput::new(), state, None)?;
    Ok(serde_json::Value::String(value))
}

/// Run a text input, parsing into a JSON number. On parse failure
/// re-prompt with an inline validation error and keep the previous
/// buffer contents so the user can correct the value.
fn collect_number(label: Label, integer: bool) -> io::Result<serde_json::Value> {
    let mut previous: Option<(String, String)> = None;
    loop {
        let mut state = TextInputState::new().with_label(label.clone());
        if let Some((buf, err)) = previous.take() {
            state = state.with_value(&buf);
            state.set_validation_error(err);
        }
        let raw: String = run_standalone(TextInput::new(), state, None)?;
        match parse_number(&raw, integer) {
            Ok(value) => return Ok(value),
            Err(message) => {
                previous = Some((raw, message));
            }
        }
    }
}

fn parse_number(raw: &str, integer: bool) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("value required".to_string());
    }
    if integer {
        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(serde_json::Value::Number(serde_json::Number::from(n)));
        }
        // Accept floats that are exact integers.
        if let Ok(n) = trimmed.parse::<f64>()
            && n.fract() == 0.0
            && n.is_finite()
            && let Some(num) = serde_json::Number::from_f64(n)
        {
            return Ok(serde_json::Value::Number(num));
        }
        Err(format!("`{trimmed}` is not a valid integer"))
    } else {
        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(serde_json::Value::Number(serde_json::Number::from(n)));
        }
        if let Ok(n) = trimmed.parse::<f64>()
            && let Some(num) = serde_json::Number::from_f64(n)
        {
            return Ok(serde_json::Value::Number(num));
        }
        Err(format!("`{trimmed}` is not a valid number"))
    }
}

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
    use super::*;

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
        let value = parse_number("42", false).unwrap();
        assert_eq!(value, serde_json::json!(42));
    }

    #[test]
    fn parse_number_accepts_float() {
        let value = parse_number("3.14", false).unwrap();
        assert!(value.as_f64().is_some());
    }

    #[test]
    fn parse_number_rejects_non_numeric() {
        assert!(parse_number("hello", false).is_err());
    }

    #[test]
    fn parse_number_integer_mode_rejects_non_integer() {
        assert!(parse_number("3.14", true).is_err());
    }

    #[test]
    fn parse_number_integer_mode_accepts_whole_float() {
        let value = parse_number("3.0", true).unwrap();
        assert!(value.as_i64().is_some() || value.as_f64().is_some());
    }

    #[test]
    fn parse_number_rejects_empty() {
        assert!(parse_number("   ", false).is_err());
    }

    #[test]
    fn format_label_combines_name_type_and_description() {
        let prop = missing_with_shape(
            "tier",
            InteractiveShape::Text {
                format: TextFormat::Plain,
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
    fn prepare_with_interactive_collection_returns_missing_when_not_allowed() {
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
        let err = prepare_with_interactive_collection(
            &source,
            PrepareOptions::default(),
            InteractivePrepareMode::Direct,
            interactive,
            &term,
        )
        .unwrap_err();
        assert!(
            matches!(err, CompositionError::MissingProperties { .. }),
            "expected MissingProperties when interactive not allowed: {err:?}"
        );
    }

    #[test]
    fn prepare_with_interactive_collection_succeeds_when_overrides_supply_value() {
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

        let options = PrepareOptions {
            set_overrides: Some(serde_json::json!({ "title": "Plan" })),
            ..Default::default()
        };
        let term = Terminal::default();
        let prepared = prepare_with_interactive_collection(
            &source,
            options,
            InteractivePrepareMode::Direct,
            InteractiveSchemaOptions::default(),
            &term,
        )
        .unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Plan"));
    }

    #[test]
    fn prepare_with_interactive_collection_returns_unsupported_for_object_shape() {
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
        let err = prepare_with_interactive_collection(
            &source,
            PrepareOptions::default(),
            InteractivePrepareMode::Direct,
            interactive,
            &term,
        )
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
