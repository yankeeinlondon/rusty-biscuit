//! Sparse style layers and their merge into one effective style.
//!
//! A render's style comes from several sources, each a [`StyleLayer`] holding
//! only the fields it sets: the document's `style:` value, each `--style`
//! file in occurrence order, and the style setters (`style.margin=2`) as one
//! final layer. [`prepare_style_layers`] validates every layer on its own,
//! then merges them in that order over the built-in defaults:
//!
//! ```text
//! defaults < document < file #1 < file #2 < ... < setters
//! ```
//!
//! Each layer is normalized before it meets another (see
//! [`super::normalize`]), so a later layer's `margin: 2` beats an earlier
//! layer's `ml: 6`. Objects merge member by member; scalars and arrays
//! replace; a value that changes between a scalar and an object (a border
//! going from `thin` to `{ left: false }`) replaces the whole value. Setting
//! `width` or `max-width` in a later layer removes the inherited other, and a
//! later component `padding` removes inherited `padding-left`/`padding-right`.
//!
//! ## Two checks
//!
//! **Schema admission** asks the document's effective schema which keys a
//! layer may hold, with the layer wrapped as `{ style: ... }` and only
//! problems under `style` kept. A missing required field is not a problem of
//! one sparse layer; required style fields are checked once on the merged
//! value. A document that declares its own `style` replaces the built-in
//! definition, so its declaration alone decides what is admitted.
//!
//! **Runtime support** is decided by the built-in style descriptor
//! ([`super::descriptor::SCHEMA`]). Every admitted field the renderer knows
//! goes through the typed validators and lands in
//! [`PreparedStyle::style`]; any other admitted value (a cleanup rule under
//! `markdown`, a custom schema field) is retained in
//! [`PreparedStyle::retained`] without a warning and without effect. A
//! schema that widens a known field cannot make an invalid value renderable.
//!
//! ## Unknown keys by source
//!
//! | Source | A key the schema does not admit |
//! |---|---|
//! | Document | a warning under [`UnknownStyleKeyPolicy::Warn`], an error under [`UnknownStyleKeyPolicy::Error`]; excluded either way |
//! | File | excluded with a warning |
//! | Setters | an error, `dm.schema.unknown_key` |
//!
//! [`StylePreparationPolicy::deny_style`] turns every unknown-key and
//! deprecation warning into an error.
//!
//! ## No schema
//!
//! With no effective schema (the baseline disabled and no `$schema`),
//! admission falls back to the descriptor: a key the built-in walker does not
//! know is unknown, and setter values are read by the descriptor's leaf type
//! instead of a declared type.
//!
//! ## Notes
//!
//! Preparation is passive. It reads no file, argument, environment, or
//! process state, executes no expression, and resolves no stylesheet; each
//! winning field keeps the [`FileResolutionContext`] of the layer that
//! supplied it, for the caller to resolve assets with.

use std::collections::HashMap;
use std::fmt;

use biscuit_file::FileResolutionContext;
use indexmap::IndexMap;
use serde_json::{Map, Value};
use simplified_schema::coercion::Shape;
use simplified_schema::completion::PathStep;
use simplified_schema::{PropertyPath, SetterValue, ValidationProblemCode};

use crate::markdown::compose::policy::{STYLE_DEPRECATED_CODE, STYLE_UNKNOWN_KEY_CODE};
use crate::markdown::schemas::EffectiveSchema;
use crate::markdown::schemas::completion::declared_shape;
use crate::style::descriptor::{LeafType, is_canonical_container, kebabify, leaf_for_canonical, nearest_key, suggested_key};
use crate::style::normalize::{self, NormalizedLayer, Segments, flatten, insert_at, unflatten};
use crate::style::parse;
use crate::style::schema::StyleFrontmatter;
use crate::style::walker;
use crate::style::warning::{StyleSpan, StyleWarning, StyleWarningKind};

/// A supplied style value the runtime validator rejects (`max-width: abc`),
/// or a merged style that cannot be read.
pub const STYLE_INVALID_VALUE_CODE: &str = "dm.style.invalid_value";
/// Two spellings of one field in one layer (`ml` beside `page.margin-left`),
/// or `width` beside `max-width` in one layer.
pub const STYLE_CONFLICT_CODE: &str = "dm.style.conflict";
/// A style file that supplies no non-null value once unknown keys are
/// excluded.
pub const STYLE_EMPTY_FILE_CODE: &str = "dm.style.empty_file";
/// A style file or `style` setter whose value is not a mapping.
pub const STYLE_INVALID_ROOT_CODE: &str = "dm.style.invalid_root";
/// A style setter that cannot be read: its path does not start with `style`,
/// its value is not admissible, or it sets a member of an open object such as
/// `page.meta`, which takes a whole-object value.
pub const STYLE_INVALID_SETTER_CODE: &str = "dm.style.invalid_setter";

/// Where a style layer comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleSourceKind {
    /// The document's own `style:` frontmatter.
    Document,
    /// A `--style` file.
    File,
    /// The style setters of one command line.
    Setters,
}

/// Where one style value came from.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleOrigin {
    pub kind: StyleSourceKind,
    /// The document's identity, the `--style` argument, or the setter
    /// argument, as the caller wrote it.
    pub source: String,
    /// The canonical property path, relative to `style` (`page.margin-left`).
    /// A problem on a shorthand keeps the shorthand's name under `page`
    /// (`page.ml`).
    pub path: String,
    /// The path as written in its source, relative to `style` (`ml`).
    pub written: String,
    /// The written key's position: the document's frontmatter coordinates,
    /// or the style file's. A setter has none.
    pub span: Option<StyleSpan>,
    /// The context a file-valued field (`page.stylesheet`) resolves in: the
    /// document's for a document value, the style file's own for a file
    /// value, the launch context for a setter.
    pub context: FileResolutionContext,
}

/// A setter's value: the text after its first `=`, or a value already read
/// by [`SetterValue::parse`].
#[derive(Debug, Clone, PartialEq)]
pub enum StyleSetterValue {
    Raw(String),
    Parsed(SetterValue),
}

/// One style setter, such as `style.table.border=thin`.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSetter {
    /// The whole argument as written, for messages and origins.
    pub argument: String,
    /// The path from the frontmatter root; it starts with `style`.
    pub path: PropertyPath,
    pub value: StyleSetterValue,
}

impl StyleSetter {
    /// A setter whose value is still text.
    pub fn new(argument: impl Into<String>, path: PropertyPath, raw: impl Into<String>) -> Self {
        Self {
            argument: argument.into(),
            path,
            value: StyleSetterValue::Raw(raw.into()),
        }
    }

    /// A setter whose value is already read.
    pub fn parsed(argument: impl Into<String>, path: PropertyPath, value: SetterValue) -> Self {
        Self {
            argument: argument.into(),
            path,
            value: StyleSetterValue::Parsed(value),
        }
    }
}

#[derive(Debug, Clone)]
enum LayerInput {
    Value(Value),
    Setters(Vec<StyleSetter>),
}

/// One source of style values, kept raw and sparse until preparation.
///
/// A setter layer binds its values during preparation, because a value's
/// type comes from the effective schema.
#[derive(Debug, Clone)]
pub struct StyleLayer {
    kind: StyleSourceKind,
    source: String,
    input: LayerInput,
    context: FileResolutionContext,
    positions: Option<IndexMap<String, StyleSpan>>,
}

impl StyleLayer {
    /// The document's `style:` value (`Value::Null` when it has none).
    ///
    /// `positions` is [`build_yaml_position_map`](super::build_yaml_position_map)
    /// over the frontmatter, keyed from the frontmatter root (`style.page.color`).
    pub fn document(
        value: Value,
        identity: impl Into<String>,
        context: FileResolutionContext,
        positions: Option<IndexMap<String, StyleSpan>>,
    ) -> Self {
        Self {
            kind: StyleSourceKind::Document,
            source: identity.into(),
            input: LayerInput::Value(value),
            context,
            positions,
        }
    }

    /// A `--style` file's value, already read and selected at its offset.
    ///
    /// `positions` is keyed relative to that value (`page.color`).
    pub fn file(
        value: Value,
        argument: impl Into<String>,
        context: FileResolutionContext,
        positions: Option<IndexMap<String, StyleSpan>>,
    ) -> Self {
        Self {
            kind: StyleSourceKind::File,
            source: argument.into(),
            input: LayerInput::Value(value),
            context,
            positions,
        }
    }

    /// The style setters of one command line, in command-line order, with the
    /// launch context.
    pub fn setters(setters: Vec<StyleSetter>, context: FileResolutionContext) -> Self {
        Self {
            kind: StyleSourceKind::Setters,
            source: "style setters".to_string(),
            input: LayerInput::Setters(setters),
            context,
            positions: None,
        }
    }

    pub fn kind(&self) -> StyleSourceKind {
        self.kind
    }

    /// The document identity or `--style` argument.
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn context(&self) -> &FileResolutionContext {
        &self.context
    }
}

/// What a document's unknown `style:` keys are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownStyleKeyPolicy {
    /// A warning: `md render` only shows the document.
    Warn,
    /// An error: `md compose` produces a document.
    Error,
}

/// The target's validation policy for [`prepare_style_layers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StylePreparationPolicy {
    pub document_unknown_keys: UnknownStyleKeyPolicy,
    /// `--deny style`: every unknown-key and deprecation warning, from any
    /// layer, is an error.
    pub deny_style: bool,
}

impl StylePreparationPolicy {
    /// `md render`: a document's unknown keys warn.
    pub const fn render() -> Self {
        Self {
            document_unknown_keys: UnknownStyleKeyPolicy::Warn,
            deny_style: false,
        }
    }

    /// `md compose`: a document's unknown keys are errors.
    pub const fn compose() -> Self {
        Self {
            document_unknown_keys: UnknownStyleKeyPolicy::Error,
            deny_style: false,
        }
    }

    #[must_use]
    pub const fn with_deny_style(mut self, deny: bool) -> Self {
        self.deny_style = deny;
        self
    }
}

impl Default for StylePreparationPolicy {
    fn default() -> Self {
        Self::render()
    }
}

/// A non-fatal finding about one layer's style.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleDiagnostic {
    /// [`STYLE_UNKNOWN_KEY_CODE`] or [`STYLE_DEPRECATED_CODE`] for a warning;
    /// `None` for a `KnownButInactive` notice, which only a debug log shows.
    pub code: Option<&'static str>,
    /// The warning, with the path as written and its span when known.
    pub warning: StyleWarning,
    pub origin: StyleOrigin,
}

impl StyleDiagnostic {
    /// The reader-facing text, or `None` for a notice.
    pub fn message(&self) -> Option<String> {
        self.code.map(|_| warning_message(&self.warning))
    }
}

/// One reason preparation failed.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleProblem {
    /// A shared code: `dm.schema.unknown_key`, `dm.schema.type_mismatch` and
    /// the other schema codes, [`STYLE_UNKNOWN_KEY_CODE`],
    /// [`STYLE_DEPRECATED_CODE`], or one of this module's `STYLE_*_CODE`s.
    pub code: &'static str,
    pub message: String,
    /// The canonical property path relative to `style`; empty for the whole
    /// value.
    pub path: String,
    /// The layer and key that supplied the value; `None` for a problem of the
    /// merged style, such as a missing required field.
    pub origin: Option<StyleOrigin>,
}

impl fmt::Display for StyleProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A schema message (`"abc" is not valid ...`) names no property; a
        // setter's argument may spell a shorthand, and a file's span is only
        // a line, so the canonical path is always said.
        let property = format!("style.{}", self.path);
        if self.path.is_empty() || self.message.contains(&property) {
            write!(f, "{}", self.message)?;
        } else {
            write!(f, "`{property}`: {}", self.message)?;
        }
        let Some(origin) = &self.origin else {
            return Ok(());
        };
        match origin.kind {
            StyleSourceKind::Document => write!(f, " (in `{}`", origin.source)?,
            StyleSourceKind::File => write!(f, " (in the style file `{}`", origin.source)?,
            StyleSourceKind::Setters => write!(f, " (from the setter `{}`", origin.source)?,
        }
        if let Some(span) = &origin.span {
            write!(f, " at line {}, column {}", span.line, span.column)?;
        }
        write!(f, ")")
    }
}

/// Why [`prepare_style_layers`] produced no style: every problem found, and
/// the warnings gathered on the way.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{}", summary(.problems))]
pub struct StylePreparationError {
    pub problems: Vec<StyleProblem>,
    pub diagnostics: Vec<StyleDiagnostic>,
}

fn summary(problems: &[StyleProblem]) -> String {
    match problems {
        [] => "invalid style".to_string(),
        [only] => format!("invalid style: {only}"),
        [first, rest @ ..] => format!("invalid style: {first} (and {} more)", rest.len()),
    }
}

/// The effective style of a render.
#[derive(Debug, Clone)]
pub struct PreparedStyle {
    value: Value,
    style: StyleFrontmatter,
    retained: Value,
    origins: IndexMap<String, StyleOrigin>,
    diagnostics: Vec<StyleDiagnostic>,
}

impl PreparedStyle {
    /// The merged, schema-admitted style on canonical paths, rendering
    /// fields and retained values together.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// The fields the renderer supports, read once after the merge.
    pub fn style(&self) -> &StyleFrontmatter {
        &self.style
    }

    pub fn into_style(self) -> StyleFrontmatter {
        self.style
    }

    /// Admitted values with no rendering field: cleanup rules under
    /// `markdown` and custom schema fields. Always an object.
    pub fn retained(&self) -> &Value {
        &self.retained
    }

    /// The origin of the winning value at the canonical leaf `path`
    /// (`page.stylesheet`, `table.border.color`).
    pub fn origin(&self, path: &str) -> Option<&StyleOrigin> {
        self.origins.get(path)
    }

    /// Every winning leaf's origin, keyed by canonical path.
    pub fn origins(&self) -> &IndexMap<String, StyleOrigin> {
        &self.origins
    }

    /// Warnings and notices, by layer in precedence order.
    pub fn diagnostics(&self) -> &[StyleDiagnostic] {
        &self.diagnostics
    }
}

/// Validates and merges `layers`, given in precedence order: the document,
/// then each `--style` file in occurrence order, then the one setter layer.
///
/// `schema` is the original document's effective schema, selected once;
/// `None` falls back to the built-in descriptor (see the module docs).
///
/// ## Errors
///
/// A [`StylePreparationError`] holding every problem found: an inadmissible
/// or invalid value in any file or setter (even one a later layer would
/// replace), two spellings of one field, `width` beside `max-width` in one
/// layer, an empty style file, a missing required style field, and, under the
/// policy, unknown and deprecated keys.
pub fn prepare_style_layers(
    layers: &[StyleLayer],
    schema: Option<&EffectiveSchema>,
    policy: StylePreparationPolicy,
) -> Result<PreparedStyle, StylePreparationError> {
    let mut report = Report::default();
    let prepared: Vec<PreparedLayer> = layers
        .iter()
        .filter_map(|layer| prepare_layer(layer, schema, policy, &mut report))
        .collect();

    if policy.deny_style {
        let (denied, kept): (Vec<_>, Vec<_>) =
            report.diagnostics.drain(..).partition(|diagnostic| diagnostic.warning.is_schema_issue());
        report.diagnostics = kept;
        report.problems.extend(denied.into_iter().map(|diagnostic| StyleProblem {
            code: diagnostic.code.unwrap_or(STYLE_UNKNOWN_KEY_CODE),
            message: warning_message(&diagnostic.warning),
            path: diagnostic.origin.path.clone(),
            origin: Some(diagnostic.origin),
        }));
    }
    if !report.problems.is_empty() {
        return Err(report.into_error());
    }

    let merged = merge(&prepared);
    let value = unflatten(merged.iter().map(|(path, entry)| (path, &entry.value)));
    if let Some(schema) = schema {
        check_required(schema, &prepared, &value, &mut report);
    }

    let (projection, retained): (Vec<_>, Vec<_>) = merged.iter().partition(|(path, _)| is_rendering_field(path));
    let projection = unflatten(projection.into_iter().map(|(path, entry)| (path, &entry.value)));
    let retained = unflatten(retained.into_iter().map(|(path, entry)| (path, &entry.value)));
    let style = match serde_json::from_value::<StyleFrontmatter>(projection) {
        Ok(style) => style,
        Err(error) => {
            report.problem(STYLE_INVALID_VALUE_CODE, format!("the merged style cannot be read: {error}"), String::new(), None);
            StyleFrontmatter::default()
        }
    };
    if !report.problems.is_empty() {
        return Err(report.into_error());
    }

    Ok(PreparedStyle {
        value,
        style,
        retained,
        origins: merged.into_iter().map(|(path, entry)| (path.join("."), entry.origin)).collect(),
        diagnostics: report.diagnostics,
    })
}

#[derive(Default)]
struct Report {
    problems: Vec<StyleProblem>,
    diagnostics: Vec<StyleDiagnostic>,
}

impl Report {
    fn problem(&mut self, code: &'static str, message: String, path: String, origin: Option<StyleOrigin>) {
        self.problems.push(StyleProblem {
            code,
            message,
            path,
            origin,
        });
    }

    fn into_error(self) -> StylePreparationError {
        StylePreparationError {
            problems: self.problems,
            diagnostics: self.diagnostics,
        }
    }
}

#[derive(Debug, Clone)]
struct MergedEntry {
    value: Value,
    origin: StyleOrigin,
}

/// One layer after its own checks.
struct PreparedLayer {
    entries: IndexMap<Segments, MergedEntry>,
    /// The admitted value as written, for the required-field check.
    admitted: Value,
}

/// Names the origin of a path in one layer.
struct Origins<'a> {
    layer: &'a StyleLayer,
    /// A document's raw path for each alias-rewritten path.
    raw_of: HashMap<Segments, Segments>,
    /// The surviving setters' paths (relative to `style`) and arguments.
    setter_paths: Vec<(Segments, String)>,
}

impl Origins<'_> {
    fn raw(&self, written: &[String]) -> Segments {
        self.raw_of.get(written).cloned().unwrap_or_else(|| written.to_vec())
    }

    /// The origin of `written` (in the layer's admission spelling).
    fn of(&self, written: &[String], canonical: &[String]) -> StyleOrigin {
        self.of_raw(&self.raw(written), canonical)
    }

    fn of_raw(&self, raw: &[String], canonical: &[String]) -> StyleOrigin {
        let source = match self.layer.kind {
            StyleSourceKind::Setters => self
                .setter_paths
                .iter()
                .filter(|(path, _)| raw.starts_with(path))
                .max_by_key(|(path, _)| path.len())
                .map(|(_, argument)| argument.clone())
                .unwrap_or_else(|| self.layer.source.clone()),
            _ => self.layer.source.clone(),
        };
        StyleOrigin {
            kind: self.layer.kind,
            source,
            path: canonical.join("."),
            written: raw.join("."),
            span: self.span(raw),
            context: self.layer.context.clone(),
        }
    }

    /// The span of `raw` or of its nearest authored ancestor.
    fn span(&self, raw: &[String]) -> Option<StyleSpan> {
        let positions = self.layer.positions.as_ref()?;
        let prefix = match self.layer.kind {
            StyleSourceKind::Document => "style",
            StyleSourceKind::File => "",
            StyleSourceKind::Setters => return None,
        };
        (0..=raw.len()).rev().find_map(|depth| {
            let key = std::iter::once(prefix)
                .filter(|prefix| !prefix.is_empty())
                .chain(raw[..depth].iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(".");
            positions.get(&key).cloned()
        })
    }
}

/// What the schema (or, without one, the descriptor) says about a layer.
#[derive(Default)]
struct Admission {
    unknown: Vec<Segments>,
    /// Value problems: path, code, message.
    problems: Vec<(Segments, &'static str, String)>,
}

fn prepare_layer(
    layer: &StyleLayer,
    schema: Option<&EffectiveSchema>,
    policy: StylePreparationPolicy,
    report: &mut Report,
) -> Option<PreparedLayer> {
    let mut origins = Origins {
        layer,
        raw_of: HashMap::new(),
        setter_paths: Vec::new(),
    };
    let root_origin = |origins: &Origins<'_>| origins.of_raw(&[], &[]);

    let mut view = match (&layer.input, layer.kind) {
        (LayerInput::Value(value), StyleSourceKind::Document) => {
            let value = match value {
                Value::Null => Value::Object(Map::new()),
                Value::Object(_) => value.clone(),
                other => {
                    let message = format!("`style` must be a mapping, not {}", type_name(other));
                    report.problem(STYLE_INVALID_VALUE_CODE, message, String::new(), Some(root_origin(&origins)));
                    return None;
                }
            };
            let (rewritten, raw_of, conflicts) = normalize::canonicalize_aliases(&value);
            origins.raw_of = raw_of;
            for conflict in conflicts {
                let origin = origins.of_raw(&conflict.second, &conflict.canonical);
                report.problem(STYLE_CONFLICT_CODE, conflict_message(&conflict), conflict.canonical.join("."), Some(origin));
            }
            document_notices(&value, &origins, report);
            rewritten
        }
        (LayerInput::Value(value), _) => {
            let Value::Object(_) = value else {
                let message = format!("a style file holds a mapping, not {}", type_name(value));
                report.problem(STYLE_INVALID_ROOT_CODE, message, String::new(), Some(root_origin(&origins)));
                return None;
            };
            let mut value = value.clone();
            strip_nulls(&mut value);
            value
        }
        (LayerInput::Setters(setters), _) => {
            let (value, paths) = assemble_setters(setters, layer, schema, report)?;
            origins.setter_paths = paths;
            value
        }
    };

    // Schema admission, before the built-in walker can call an admitted
    // custom key unknown.
    let admission = admit(&view, schema, layer.kind);
    for path in &admission.unknown {
        report_unknown(path, &origins, policy, report);
        remove_path(&mut view, path);
    }
    if layer.kind != StyleSourceKind::Document {
        let mut notices = Vec::new();
        parse::annotate_known_but_inactive(&view, &mut notices);
        for warning in notices {
            push_diagnostic(warning, &origins, report);
        }
        for (path, code, message) in &admission.problems {
            let origin = origins.of(path, &canonical_spelling(path));
            report.problem(code, message.clone(), canonical_spelling(path).join("."), Some(origin));
        }
    }

    let leaves = flatten(&view);
    if layer.kind == StyleSourceKind::File && !leaves.iter().any(|(_, value)| !is_empty_array(value)) {
        let message = "the style file sets no style value once unknown keys and nulls are removed".to_string();
        report.problem(STYLE_EMPTY_FILE_CODE, message, String::new(), Some(root_origin(&origins)));
    }

    let normalized = match normalize::normalize_leaves(leaves) {
        Ok(normalized) => normalized,
        Err(conflicts) => {
            for conflict in conflicts {
                let origin = origins.of(&conflict.second, &conflict.canonical);
                report.problem(STYLE_CONFLICT_CODE, conflict_message(&conflict), conflict.canonical.join("."), Some(origin));
            }
            return None;
        }
    };
    check_width_pairs(&normalized, &origins, report);
    // A document's values were never schema-checked by rendering; their
    // runtime check stays the only one. A file or setter value with a schema
    // problem already failed and is not checked twice.
    let schema_rejected: Vec<&Segments> = match layer.kind {
        StyleSourceKind::Document => Vec::new(),
        _ => admission.problems.iter().map(|(path, _, _)| path).collect(),
    };
    validate_runtime(&normalized, &schema_rejected, &origins, report);

    let entries = normalized
        .into_iter()
        .map(|(path, entry)| {
            let origin = origins.of(&entry.written, &path);
            (
                path,
                MergedEntry {
                    value: entry.value,
                    origin,
                },
            )
        })
        .collect();
    strip_nulls(&mut view);
    Some(PreparedLayer { entries, admitted: view })
}

/// A document's deprecation and inactive-key notices, as the typed parser
/// reports them. Its unknown keys come from admission instead.
fn document_notices(value: &Value, origins: &Origins<'_>, report: &mut Report) {
    let mut warnings: Vec<StyleWarning> = walker::walk(value)
        .into_iter()
        .filter(|warning| warning.kind != StyleWarningKind::UnknownKey)
        .collect();
    parse::annotate_known_but_inactive(value, &mut warnings);
    parse::scan_deprecated_enum_aliases(value, &mut warnings);
    for warning in warnings {
        push_diagnostic(warning, origins, report);
    }
}

/// Records `warning` (its path as written, `style.`-prefixed) with its origin.
fn push_diagnostic(mut warning: StyleWarning, origins: &Origins<'_>, report: &mut Report) {
    let raw = segments_of_warning(&warning.path);
    let canonical = match &warning.kind {
        // A key's replacement is its canonical path; a value's (`centered`)
        // is not a path.
        StyleWarningKind::Deprecated { replacement } if replacement.split('.').count() == raw.len() => {
            replacement.split('.').map(str::to_string).collect()
        }
        _ => canonical_spelling(&raw.iter().map(|segment| kebabify(segment)).collect::<Vec<_>>()),
    };
    let origin = origins.of_raw(&raw, &canonical);
    if warning.source_span.is_none() {
        warning.source_span = origin.span.clone();
    }
    let code = match warning.kind {
        StyleWarningKind::UnknownKey => Some(STYLE_UNKNOWN_KEY_CODE),
        StyleWarningKind::Deprecated { .. } => Some(STYLE_DEPRECATED_CODE),
        StyleWarningKind::KnownButInactive { .. } => None,
    };
    report.diagnostics.push(StyleDiagnostic { code, warning, origin });
}

fn report_unknown(path: &[String], origins: &Origins<'_>, policy: StylePreparationPolicy, report: &mut Report) {
    let raw = origins.raw(path);
    let kebab: Vec<String> = path.iter().map(|segment| kebabify(segment)).collect();
    let suggestion = suggested_key(&kebab.join(".")).filter(|key| *key != path.join("."));
    let mut warning = StyleWarning::new(format!("style.{}", raw.join(".")), StyleWarningKind::UnknownKey);
    if let Some(key) = &suggestion {
        warning = warning.with_suggestion(key.clone());
    }
    let canonical = canonical_spelling(path);
    match (origins.layer.kind, policy.document_unknown_keys) {
        (StyleSourceKind::File, _) | (StyleSourceKind::Document, UnknownStyleKeyPolicy::Warn) => {
            push_diagnostic(warning, origins, report);
        }
        (StyleSourceKind::Document, UnknownStyleKeyPolicy::Error) => {
            let origin = origins.of(path, &canonical);
            report.problem(STYLE_UNKNOWN_KEY_CODE, warning_message(&warning), canonical.join("."), Some(origin));
        }
        (StyleSourceKind::Setters, _) => {
            let (key, parent) = path.split_last().map_or(("", &[][..]), |(key, parent)| (key.as_str(), parent));
            let parent = std::iter::once("style").chain(parent.iter().map(String::as_str)).collect::<Vec<_>>().join(".");
            let mut message = format!("`{key}` is not a valid property under `{parent}`");
            // A setter is typed by hand, so a typo also gets the nearest
            // sibling; files and documents keep the rename-only suggestion.
            if let Some(key) = suggestion.or_else(|| nearest_key(&kebab.join("."))) {
                message.push_str(&format!("; did you mean `style.{key}`?"));
            }
            let origin = origins.of(path, &canonical);
            report.problem(
                ValidationProblemCode::UnknownKey.diagnostic_code(),
                message,
                canonical.join("."),
                Some(origin),
            );
        }
    }
}

/// Asks the schema (or the descriptor) which of `view`'s keys are admitted.
fn admit(view: &Value, schema: Option<&EffectiveSchema>, kind: StyleSourceKind) -> Admission {
    let mut admission = Admission::default();
    let Some(schema) = schema else {
        for warning in walker::walk(view) {
            // Outside a document, a snake-case alias is not a spelling at all.
            let unknown = match warning.kind {
                StyleWarningKind::UnknownKey => true,
                StyleWarningKind::Deprecated { .. } => kind != StyleSourceKind::Document,
                StyleWarningKind::KnownButInactive { .. } => false,
            };
            if unknown {
                admission.unknown.push(segments_of_warning(&warning.path));
            }
        }
        return admission;
    };

    // A value that fails to convert hides the other problems of its report,
    // so validate again without each rejected path until nothing new appears.
    let mut remaining = view.clone();
    for _ in 0..MAX_ADMISSION_PASSES {
        let found = admission.unknown.len() + admission.problems.len();
        admit_pass(&remaining, schema, &mut admission);
        if admission.unknown.len() + admission.problems.len() == found {
            break;
        }
        for path in admission.unknown.iter().chain(admission.problems.iter().map(|(path, _, _)| path)) {
            remove_path(&mut remaining, path);
        }
    }
    admission.unknown.dedup();
    admission
}

/// The most validations [`admit`] runs for one layer.
const MAX_ADMISSION_PASSES: usize = 4;

fn admit_pass(view: &Value, schema: &EffectiveSchema, admission: &mut Admission) {
    let mut instance = Map::new();
    instance.insert("style".to_string(), view.clone());
    for problem in schema.validate(&Value::Object(instance)).problems {
        if problem.code == ValidationProblemCode::MissingRequired {
            continue;
        }
        let offending: Vec<String> = if problem.offending_properties.is_empty() {
            problem.offending_property.iter().cloned().collect()
        } else {
            problem.offending_properties.clone()
        };
        let path = match problem.instance_path.segments().split_first() {
            Some((first, rest)) if first == "style" => rest.to_vec(),
            // A root that admits no `style` at all.
            None if problem.code == ValidationProblemCode::UnknownKey && offending.iter().any(|key| key == "style") => {
                if let Value::Object(map) = view {
                    admission.unknown.extend(map.keys().map(|key| vec![key.clone()]));
                }
                continue;
            }
            _ => continue,
        };
        if problem.code == ValidationProblemCode::UnknownKey {
            admission.unknown.extend(offending.into_iter().map(|key| {
                let mut unknown = path.clone();
                unknown.push(key);
                unknown
            }));
        } else if let undeclared @ [_, ..] = undeclared_members(view, &path, schema).as_slice() {
            // A union's object arm (`table.border`) fails as a whole value, never
            // as an unknown key; its undeclared members are unknown keys, and
            // [`admit`]'s next pass judges what remains.
            admission.unknown.extend(undeclared.iter().cloned());
        } else {
            admission.problems.push((path, problem.code.diagnostic_code(), problem.message));
        }
    }
}

/// The members of the object at `path` in `view` that the schema declares at
/// no arm of its definition.
fn undeclared_members(view: &Value, path: &[String], schema: &EffectiveSchema) -> Vec<Segments> {
    let Some(Value::Object(members)) = path.iter().try_fold(view, |value, key| value.get(key)) else {
        return Vec::new();
    };
    let mut frontmatter = Map::new();
    frontmatter.insert("style".to_string(), view.clone());
    let frontmatter = Value::Object(frontmatter);
    members
        .keys()
        .filter(|key| {
            let steps: Vec<PathStep<'_>> = std::iter::once("style")
                .chain(path.iter().map(String::as_str))
                .chain(std::iter::once(key.as_str()))
                .map(PathStep::Key)
                .collect();
            declared_shape(schema, &frontmatter, &steps, None).is_none()
        })
        .map(|key| path.iter().cloned().chain(std::iter::once(key.clone())).collect())
        .collect()
}

/// Fails a layer that sets both `width` and `max-width` of one component.
fn check_width_pairs(layer: &NormalizedLayer, origins: &Origins<'_>, report: &mut Report) {
    for (path, entry) in layer {
        let Some((last, parent)) = path.split_last() else { continue };
        if last != "width" || !is_leaf(path) {
            continue;
        }
        let mut sibling = parent.to_vec();
        sibling.push("max-width".to_string());
        if layer.contains_key(&sibling) {
            let parent = parent.join(".");
            let message = format!("`{parent}.width` and `{parent}.max-width` are exclusive; set one");
            report.problem(STYLE_CONFLICT_CODE, message, path.join("."), Some(origins.of(&entry.written, path)));
        }
    }
}

/// Runs the typed validators over each rendering field and the cleanup rules.
fn validate_runtime(layer: &NormalizedLayer, schema_rejected: &[&Segments], origins: &Origins<'_>, report: &mut Report) {
    let overlaps = |written: &Segments| {
        schema_rejected
            .iter()
            .any(|rejected| written.starts_with(rejected) || rejected.starts_with(written))
    };
    let mut groups: IndexMap<Segments, Vec<&Segments>> = IndexMap::new();
    let mut rules: Vec<&Segments> = Vec::new();
    for (path, entry) in layer {
        if overlaps(&entry.written) {
            continue;
        }
        if path.first().is_some_and(|first| first == "markdown") {
            rules.push(path);
        } else if let Some(group) = rendering_field(path) {
            groups.entry(group).or_default().push(path);
        }
    }

    for (group, paths) in groups {
        let value = relative_value(layer, &group, &paths);
        let mut wrapped = Map::new();
        insert_at(&mut wrapped, &group, value);
        let wrapped = Value::Object(wrapped);
        let result = match parse::pre_validate(&wrapped) {
            Ok(()) => serde_json::from_value::<StyleFrontmatter>(wrapped).map(drop).map_err(Into::into),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            let origin = origins.of(&layer[paths[0]].written, &group);
            report.problem(STYLE_INVALID_VALUE_CODE, error.to_string(), group.join("."), Some(origin));
        }
    }

    if let Some(first) = rules.first() {
        let group = vec!["markdown".to_string()];
        let value = relative_value(layer, &group, &rules);
        if let Err(error) = parse::validate_markdown_rules(&value) {
            let origin = origins.of(&layer[*first].written, first);
            report.problem(STYLE_INVALID_VALUE_CODE, error.to_string(), "markdown".to_string(), Some(origin));
        }
    }
}

/// The value at `group` rebuilt from the layer's leaves at `paths`.
fn relative_value(layer: &NormalizedLayer, group: &[String], paths: &[&Segments]) -> Value {
    match paths {
        [only] if only.len() == group.len() => layer[*only].value.clone(),
        _ => {
            let relative: Vec<(Segments, Value)> = paths
                .iter()
                .map(|path| (path[group.len()..].to_vec(), layer[*path].value.clone()))
                .collect();
            unflatten(relative.iter().map(|(path, value)| (path, value)))
        }
    }
}

/// Reads the surviving setters into one style value, with each surviving
/// setter's path and argument.
///
/// The same decoded path given twice keeps its last value, so repeating
/// `style` replaces that whole assignment. Shorter paths apply first, and a
/// longer one below a scalar replaces it with an object, which is how
/// `style.table.border.left=false` selects the border's object form after
/// `style.table.border=thin`. This is style-only: the shared property editor
/// refuses a scalar parent.
fn assemble_setters(
    setters: &[StyleSetter],
    layer: &StyleLayer,
    schema: Option<&EffectiveSchema>,
    report: &mut Report,
) -> Option<(Value, Vec<(Segments, String)>)> {
    struct Planned {
        argument: String,
        path: Segments,
        value: SetterValue,
    }
    let failures = report.problems.len();
    let mut planned: Vec<Planned> = Vec::new();
    for setter in setters {
        let segments = setter.path.segments();
        let path: Segments = segments.iter().skip(1).cloned().collect();
        let mut fail = |message: String| {
            let origin = StyleOrigin {
                kind: StyleSourceKind::Setters,
                source: setter.argument.clone(),
                path: path.join("."),
                written: path.join("."),
                span: None,
                context: layer.context.clone(),
            };
            report.problem(STYLE_INVALID_SETTER_CODE, message, path.join("."), Some(origin));
        };
        if segments[0] != "style" {
            fail(format!("a style setter's path starts with `style`, not `{}`", segments[0]));
            continue;
        }
        let value = match &setter.value {
            StyleSetterValue::Raw(raw) => match SetterValue::parse(raw) {
                Ok(value) => value,
                Err(error) => {
                    fail(error.to_string());
                    continue;
                }
            },
            StyleSetterValue::Parsed(value) => value.clone(),
        };
        if let Some(open) = open_object_ancestor(&path) {
            fail(format!("`style.{open}` takes a whole-object value, such as `style.{open}='{{ ... }}'`"));
            continue;
        }
        match planned.iter_mut().find(|kept| kept.path == path) {
            Some(kept) => {
                kept.argument = setter.argument.clone();
                kept.value = value;
            }
            None => planned.push(Planned {
                argument: setter.argument.clone(),
                path,
                value,
            }),
        }
    }
    if report.problems.len() > failures {
        return None;
    }

    planned.sort_by_key(|setter| setter.path.len());
    let mut root = Map::new();
    let mut paths = Vec::new();
    for setter in planned {
        let bound = bind_setter(&setter.value, &setter.path, schema, &root);
        if setter.path.is_empty() {
            match bound {
                Value::Object(map) => root = map,
                other => {
                    let origin = StyleOrigin {
                        kind: StyleSourceKind::Setters,
                        source: setter.argument.clone(),
                        path: String::new(),
                        written: String::new(),
                        span: None,
                        context: layer.context.clone(),
                    };
                    let message = format!("`style` takes a mapping, not {}", type_name(&other));
                    report.problem(STYLE_INVALID_ROOT_CODE, message, String::new(), Some(origin));
                    continue;
                }
            }
        } else {
            insert_at(&mut root, &setter.path, bound);
        }
        paths.push((setter.path, setter.argument));
    }
    if report.problems.len() > failures {
        return None;
    }
    let mut value = Value::Object(root);
    strip_nulls(&mut value);
    Some((value, paths))
}

/// Reads a setter's value through the type the schema declares at its path.
///
/// A rendering field the schema leaves untyped (`unknown`, or a pattern key)
/// is read by its descriptor type instead, so `style.margin=4` is the number
/// `4` under `"<string>": unknown`.
fn bind_setter(value: &SetterValue, path: &[String], schema: Option<&EffectiveSchema>, root: &Map<String, Value>) -> Value {
    let Some(schema) = schema else {
        return bind_by_descriptor(value, path);
    };
    let steps: Vec<PathStep<'_>> = std::iter::once(PathStep::Key("style"))
        .chain(path.iter().map(|segment| PathStep::Key(segment.as_str())))
        .collect();
    let mut frontmatter = Map::new();
    frontmatter.insert("style".to_string(), Value::Object(root.clone()));
    match declared_shape(schema, &Value::Object(frontmatter), &steps, Some(value)) {
        Some(Shape::Any) | None if descriptor_leaf(path).is_some() => bind_by_descriptor(value, path),
        declared => value.bind(declared.as_ref()),
    }
}

/// Reads unquoted text as a string where the descriptor's field is textual,
/// and guesses its type elsewhere.
fn bind_by_descriptor(value: &SetterValue, path: &[String]) -> Value {
    let textual = descriptor_leaf(path).is_some_and(|leaf_type| {
        matches!(
            leaf_type,
            LeafType::Alignment
                | LeafType::Color
                | LeafType::BackgroundEnum
                | LeafType::StringValue
                | LeafType::ThemeName
                | LeafType::CodeBlockMode
                | LeafType::HrKind
                | LeafType::HrWeight
                | LeafType::HrAlignment
                | LeafType::WordWrap
        )
    });
    match value {
        SetterValue::Text { raw } if textual => Value::String(raw.clone()),
        other => other.bind(None),
    }
}

/// The descriptor type of the field `path` (as written) sets exactly.
fn descriptor_leaf(path: &[String]) -> Option<LeafType> {
    leaf_for_canonical(&canonical_spelling(path).join(".")).map(|leaf| leaf.leaf_type)
}

/// The open-object field (`page.meta`) strictly above `path`, if any.
fn open_object_ancestor(path: &[String]) -> Option<&'static str> {
    (1..path.len()).find_map(|depth| {
        leaf_for_canonical(&path[..depth].join("."))
            .filter(|leaf| leaf.leaf_type == LeafType::OpaqueValue)
            .map(|leaf| leaf.canonical)
    })
}

/// Merges prepared layers in order (see the module docs).
fn merge(layers: &[PreparedLayer]) -> IndexMap<Segments, MergedEntry> {
    let mut merged: IndexMap<Segments, MergedEntry> = IndexMap::new();
    for layer in layers {
        for path in layer.entries.keys() {
            for replaced in replaced_siblings(path) {
                merged.shift_remove(&replaced);
            }
        }
        for (path, entry) in &layer.entries {
            merged.retain(|kept, _| !(is_proper_prefix(kept, path) || is_proper_prefix(path, kept)));
            merged.insert(path.clone(), entry.clone());
        }
    }
    merged
}

/// The inherited fields a layer's value at `path` removes: the other of
/// `width`/`max-width`, and a component's `padding-left`/`padding-right` for
/// its `padding`.
fn replaced_siblings(path: &[String]) -> Vec<Segments> {
    let Some((last, parent)) = path.split_last() else {
        return Vec::new();
    };
    let siblings: &[&str] = match last.as_str() {
        "width" => &["max-width"],
        "max-width" => &["width"],
        "padding" => &["padding-left", "padding-right"],
        _ => &[],
    };
    if !is_leaf(path) {
        return Vec::new();
    }
    siblings
        .iter()
        .map(|sibling| {
            let mut path = parent.to_vec();
            path.push((*sibling).to_string());
            path
        })
        .filter(|path| is_leaf(path))
        .collect()
}

/// Checks the schema's required style fields against every layer's admitted
/// spelling together with the merged canonical value, so a required
/// shorthand or top-level key is found where an author wrote it.
fn check_required(schema: &EffectiveSchema, layers: &[PreparedLayer], merged: &Value, report: &mut Report) {
    let mut combined = Value::Object(Map::new());
    for layer in layers {
        deep_merge(&mut combined, &layer.admitted);
    }
    deep_merge(&mut combined, merged);
    let mut instance = Map::new();
    instance.insert("style".to_string(), combined);
    for problem in schema.validate(&Value::Object(instance)).problems {
        if problem.code != ValidationProblemCode::MissingRequired {
            continue;
        }
        let Some((first, rest)) = problem.instance_path.segments().split_first() else {
            continue;
        };
        if first != "style" {
            continue;
        }
        let mut path = rest.to_vec();
        path.extend(problem.property.iter().cloned());
        let dotted = path.join(".");
        report.problem(
            ValidationProblemCode::MissingRequired.diagnostic_code(),
            format!("the required style property `style.{dotted}` is not set by any style source"),
            dotted,
            None,
        );
    }
}

/// The rendering field holding `path`: the descriptor leaf at or above it, or
/// `path` itself when it names a container (`page: 5`).
fn rendering_field(path: &[String]) -> Option<Segments> {
    if is_canonical_container(&path.join(".")) {
        return Some(path.to_vec());
    }
    (1..=path.len())
        .rev()
        .find(|depth| {
            leaf_for_canonical(&path[..*depth].join(".")).is_some_and(|leaf| leaf.leaf_type != LeafType::MarkdownRule)
        })
        .map(|depth| path[..depth].to_vec())
}

fn is_rendering_field(path: &&Segments) -> bool {
    path.first().is_none_or(|first| first != "markdown") && rendering_field(path).is_some()
}

fn is_leaf(path: &[String]) -> bool {
    leaf_for_canonical(&path.join(".")).is_some()
}

/// `path` with a top-level page key moved under `page`.
fn canonical_spelling(path: &[String]) -> Segments {
    let mut canonical = path.to_vec();
    if path.first().is_some_and(|key| normalize::is_top_level_page_key(key)) {
        canonical.insert(0, "page".to_string());
    }
    canonical
}

fn segments_of_warning(path: &str) -> Segments {
    let relative = path.strip_prefix("style.").unwrap_or(path);
    relative.split('.').map(str::to_string).collect()
}

fn is_proper_prefix(ancestor: &[String], descendant: &[String]) -> bool {
    ancestor.len() < descendant.len() && descendant.starts_with(ancestor)
}

fn is_empty_array(value: &Value) -> bool {
    value.as_array().is_some_and(Vec::is_empty)
}

/// Removes every null member, at any depth, from objects.
fn strip_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, child| !child.is_null());
            map.values_mut().for_each(strip_nulls);
        }
        Value::Array(items) => items.iter_mut().for_each(strip_nulls),
        _ => {}
    }
}

/// Removes the value at `path`; the whole object's members for an empty one.
fn remove_path(value: &mut Value, path: &[String]) {
    let Value::Object(map) = value else { return };
    match path {
        [] => map.clear(),
        [last] => {
            map.remove(last);
        }
        [first, rest @ ..] => {
            if let Some(child) = map.get_mut(first) {
                remove_path(child, rest);
            }
        }
    }
}

/// Merges objects member by member; anything else replaces.
fn deep_merge(target: &mut Value, overlay: &Value) {
    match (target, overlay) {
        (Value::Object(target), Value::Object(overlay)) => {
            for (key, value) in overlay {
                match target.get_mut(key) {
                    Some(existing) if existing.is_object() && value.is_object() => deep_merge(existing, value),
                    _ => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, overlay) => *target = overlay.clone(),
    }
}

fn conflict_message(conflict: &normalize::SpellingConflict) -> String {
    format!(
        "`style.{}` and `style.{}` both set `style.{}`; keep one",
        conflict.first.join("."),
        conflict.second.join("."),
        conflict.canonical.join(".")
    )
}

fn warning_message(warning: &StyleWarning) -> String {
    match &warning.kind {
        StyleWarningKind::UnknownKey => match &warning.suggestion {
            Some(key) => format!("unknown style key `{}`; use `{key}`", warning.path),
            None => format!("unknown style key `{}`", warning.path),
        },
        StyleWarningKind::Deprecated { replacement } => {
            format!("deprecated style key `{}`; use `{replacement}`", warning.path)
        }
        StyleWarningKind::KnownButInactive { .. } => format!("style key `{}` is not yet rendered", warning.path),
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "a mapping",
    }
}
