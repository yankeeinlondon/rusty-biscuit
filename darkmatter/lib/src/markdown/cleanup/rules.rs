//! The `style.markdown` formatting rules: their values, validation, and
//! precedence.
//!
//! This module is the single validator for rule values. The document reader
//! ([`document_rules`]), DMLS's `formatting.markdown` reader
//! ([`rules_from_json`]), and the `style:` parser's registration all delegate
//! here, so a value is accepted or rejected the same way everywhere.
//!
//! Each precedence level contributes a [`MarkdownRuleOverrides`] whose leaves
//! record whether a value was absent, explicitly `null`, or set. Absent and
//! `null` both fall through to the next level, but they stay distinct so a
//! reader can report what was authored. [`resolve`] picks each leaf
//! independently: CLI, then document, then DMLS workspace, then the runtime
//! default. Runtime defaults never depend on the baseline schema and are
//! never written into frontmatter.

use biscuit_file::{SourceSpan, YamlPathSegment, locate_yaml_key, locate_yaml_value, serde_yaml_ng};
use serde_json::{Map, Value};

use super::diagnostic::CleanDiagnostic;
use crate::markdown::literal_token::holds_pending_syntax;

/// The width `fixed-width: true` stands for.
pub const FIXED_WIDTH_TRUE_COLUMNS: u16 = 120;

/// The widest accepted `fixed-width`.
pub const FIXED_WIDTH_MAX_COLUMNS: u16 = 1000;

/// The rule names, in schema order.
pub const RULE_NAMES: [&str; 10] = [
    "frontmatter",
    "table-layout",
    "fixed-width",
    "unwrap",
    "indentation",
    "ordered-lists",
    "list-padding",
    "list-item-padding",
    "list-interior-padding",
    "default-language",
];

/// One rule value as one precedence level authored it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Leaf<T> {
    /// The key was not written.
    #[default]
    Absent,
    /// The key was written as `null`. Falls through like [`Leaf::Absent`].
    Null,
    /// An explicit, validated value.
    Value(T),
}

impl<T> Leaf<T> {
    /// The explicit value, if any.
    pub fn value(&self) -> Option<&T> {
        match self {
            Leaf::Value(value) => Some(value),
            Leaf::Absent | Leaf::Null => None,
        }
    }
}

/// `fixed-width`: re-wrap prose to a display width, or never re-wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedWidth {
    /// `false`: no re-wrapping.
    Off,
    /// Re-wrap to this many columns (`1..=1000`).
    Columns(u16),
}

/// `indentation`: spaces per nested-list level, or leave lists as authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indentation {
    Preserve,
    Two,
    Four,
    Eight,
}

impl Indentation {
    /// The requested spaces per level; `None` for `preserve`.
    #[must_use]
    pub fn spaces(self) -> Option<usize> {
        match self {
            Indentation::Preserve => None,
            Indentation::Two => Some(2),
            Indentation::Four => Some(4),
            Indentation::Eight => Some(8),
        }
    }

    /// The value for `spaces` per level; `None` unless it is 2, 4, or 8.
    #[must_use]
    pub fn from_spaces(spaces: usize) -> Option<Self> {
        match spaces {
            2 => Some(Indentation::Two),
            4 => Some(Indentation::Four),
            8 => Some(Indentation::Eight),
            _ => None,
        }
    }
}

/// `list-item-padding`: blank lines between sibling items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListItemPadding {
    Preserve,
    None,
    One,
}

/// `list-interior-padding`: blank lines before blocks nested in an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListInteriorPadding {
    Preserve,
    None,
    ExceptNestedLists,
    All,
}

/// `default-language`: the info string added to an unlabeled fence.
///
/// Holds no line break, control character, backtick, or surrounding
/// whitespace, so it can never end or reshape the fence it is added to. The
/// empty string means "add none".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultLanguage(String);

impl DefaultLanguage {
    /// Validates `language`.
    ///
    /// ## Errors
    ///
    /// Returns the reason the value could break a fence.
    pub fn new(language: impl Into<String>) -> Result<Self, String> {
        let language = language.into();
        if language.chars().any(char::is_control) {
            return Err("must not contain line breaks or control characters".to_string());
        }
        if language.contains('`') {
            return Err("must not contain backticks".to_string());
        }
        if language.trim() != language {
            return Err("must not start or end with whitespace".to_string());
        }
        Ok(Self(language))
    }

    /// The info string; empty when no language is added.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The rule values one precedence level authored.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarkdownRuleOverrides {
    pub frontmatter: Leaf<bool>,
    pub table_layout: Leaf<bool>,
    pub fixed_width: Leaf<FixedWidth>,
    pub unwrap: Leaf<bool>,
    pub indentation: Leaf<Indentation>,
    pub ordered_lists: Leaf<bool>,
    pub list_padding: Leaf<bool>,
    pub list_item_padding: Leaf<ListItemPadding>,
    pub list_interior_padding: Leaf<ListInteriorPadding>,
    pub default_language: Leaf<DefaultLanguage>,
}

impl MarkdownRuleOverrides {
    /// Explicit values that turn every body rule off; frontmatter repair is
    /// left to the other levels.
    #[must_use]
    pub fn all_body_rules_off() -> Self {
        Self {
            frontmatter: Leaf::Absent,
            table_layout: Leaf::Value(false),
            fixed_width: Leaf::Value(FixedWidth::Off),
            unwrap: Leaf::Value(false),
            indentation: Leaf::Value(Indentation::Preserve),
            ordered_lists: Leaf::Value(false),
            list_padding: Leaf::Value(false),
            list_item_padding: Leaf::Value(ListItemPadding::Preserve),
            list_interior_padding: Leaf::Value(ListInteriorPadding::Preserve),
            default_language: Leaf::Value(DefaultLanguage(String::new())),
        }
    }

    /// `true` when no leaf holds a value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        [
            self.frontmatter.value().is_none(),
            self.table_layout.value().is_none(),
            self.fixed_width.value().is_none(),
            self.unwrap.value().is_none(),
            self.indentation.value().is_none(),
            self.ordered_lists.value().is_none(),
            self.list_padding.value().is_none(),
            self.list_item_padding.value().is_none(),
            self.list_interior_padding.value().is_none(),
            self.default_language.value().is_none(),
        ]
        .iter()
        .all(|none| *none)
    }
}

/// The precedence level a resolved value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSource {
    Cli,
    Document,
    Workspace,
    Default,
}

/// A resolved rule value and the level that supplied it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved<T> {
    pub value: T,
    pub source: RuleSource,
}

/// Every rule's effective value for one cleanup run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMarkdownRules {
    pub frontmatter: Resolved<bool>,
    pub table_layout: Resolved<bool>,
    pub fixed_width: Resolved<FixedWidth>,
    pub unwrap: Resolved<bool>,
    pub indentation: Resolved<Indentation>,
    pub ordered_lists: Resolved<bool>,
    pub list_padding: Resolved<bool>,
    pub list_item_padding: Resolved<ListItemPadding>,
    pub list_interior_padding: Resolved<ListInteriorPadding>,
    pub default_language: Resolved<DefaultLanguage>,
}

impl ResolvedMarkdownRules {
    /// The runtime defaults, with no override at any level.
    #[must_use]
    pub fn defaults() -> Self {
        resolve(&MarkdownRuleOverrides::default(), None, None)
    }

    /// Every body rule off, as if requested on the command line.
    #[must_use]
    pub fn all_body_rules_off() -> Self {
        resolve(&MarkdownRuleOverrides::all_body_rules_off(), None, None)
    }

    /// Whether soft breaks inside paragraphs may be joined: `unwrap`, or a
    /// resolved positive width, which implies it. A width of `false` does not
    /// cancel an `unwrap: true` from any level.
    #[must_use]
    pub fn joins_soft_breaks(&self) -> bool {
        self.unwrap.value || matches!(self.fixed_width.value, FixedWidth::Columns(_))
    }

    /// The re-wrap width, when one is in effect.
    #[must_use]
    pub fn wrap_columns(&self) -> Option<u16> {
        match self.fixed_width.value {
            FixedWidth::Columns(columns) => Some(columns),
            FixedWidth::Off => None,
        }
    }

    /// Whether `list-item-padding` was explicitly set to a value that may
    /// change a list between tight and loose.
    #[must_use]
    pub fn permits_item_padding_change(&self) -> bool {
        self.list_item_padding.source != RuleSource::Default
            && self.list_item_padding.value != ListItemPadding::Preserve
    }

    /// Whether `list-interior-padding` was explicitly set to a value that may
    /// change a list between tight and loose.
    #[must_use]
    pub fn permits_interior_padding_change(&self) -> bool {
        self.list_interior_padding.source != RuleSource::Default
            && self.list_interior_padding.value != ListInteriorPadding::Preserve
    }

    /// Whether either padding rule permits a tight/loose change.
    #[must_use]
    pub fn permits_tightness_change(&self) -> bool {
        self.permits_item_padding_change() || self.permits_interior_padding_change()
    }
}

/// Resolves each rule independently: `cli`, then `document`, then
/// `workspace`, then the runtime default.
#[must_use]
pub fn resolve(
    cli: &MarkdownRuleOverrides,
    document: Option<&MarkdownRuleOverrides>,
    workspace: Option<&MarkdownRuleOverrides>,
) -> ResolvedMarkdownRules {
    fn pick<T: Clone>(
        cli: &Leaf<T>,
        document: Option<&Leaf<T>>,
        workspace: Option<&Leaf<T>>,
        default: T,
    ) -> Resolved<T> {
        [
            (Some(cli), RuleSource::Cli),
            (document, RuleSource::Document),
            (workspace, RuleSource::Workspace),
        ]
        .into_iter()
        .find_map(|(leaf, source)| {
            leaf.and_then(Leaf::value).map(|value| Resolved {
                value: value.clone(),
                source,
            })
        })
        .unwrap_or(Resolved {
            value: default,
            source: RuleSource::Default,
        })
    }

    macro_rules! leaf {
        ($field:ident, $default:expr) => {
            pick(
                &cli.$field,
                document.map(|level| &level.$field),
                workspace.map(|level| &level.$field),
                $default,
            )
        };
    }

    ResolvedMarkdownRules {
        frontmatter: leaf!(frontmatter, true),
        table_layout: leaf!(table_layout, true),
        fixed_width: leaf!(fixed_width, FixedWidth::Off),
        unwrap: leaf!(unwrap, false),
        indentation: leaf!(indentation, Indentation::Four),
        ordered_lists: leaf!(ordered_lists, true),
        list_padding: leaf!(list_padding, true),
        list_item_padding: leaf!(list_item_padding, ListItemPadding::Preserve),
        list_interior_padding: leaf!(list_interior_padding, ListInteriorPadding::Preserve),
        default_language: leaf!(default_language, DefaultLanguage("text".to_string())),
    }
}

/// Why a rules object, or one of its leaves, was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    /// The offending rule name; `None` when the object itself is wrong.
    pub key: Option<String>,
    /// What is wrong, phrased for the author.
    pub message: String,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.key {
            Some(key) => write!(f, "`{key}`: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// Reads one level's rules from a JSON object (a converted YAML mapping, a
/// TOML table, or editor settings).
///
/// Leaves that are `null` are recorded as [`Leaf::Null`]. Unknown names are
/// rejected, never ignored.
///
/// ## Errors
///
/// Every problem found, in key order. One invalid leaf invalidates the whole
/// object.
pub fn rules_from_json(value: &Value) -> Result<MarkdownRuleOverrides, Vec<RuleError>> {
    let Value::Object(map) = value else {
        return Err(vec![RuleError {
            key: None,
            message: format!("expected a mapping of rules, found {}", type_name(value)),
        }]);
    };
    let mut overrides = MarkdownRuleOverrides::default();
    let mut errors = Vec::new();
    for (key, leaf) in map {
        if let Err(message) = read_leaf(&mut overrides, key, leaf) {
            errors.push(RuleError {
                key: Some(key.clone()),
                message,
            });
        }
    }
    if errors.is_empty() { Ok(overrides) } else { Err(errors) }
}

/// Validates one rule's value without recording it; `null` is valid.
///
/// ## Errors
///
/// Why `value` is not a valid value for `name`, or that `name` is unknown.
pub fn validate_rule(name: &str, value: &Value) -> Result<(), String> {
    read_leaf(&mut MarkdownRuleOverrides::default(), name, value)
}

fn read_leaf(overrides: &mut MarkdownRuleOverrides, name: &str, value: &Value) -> Result<(), String> {
    match name {
        "frontmatter" => overrides.frontmatter = leaf(value, boolean)?,
        "table-layout" => overrides.table_layout = leaf(value, boolean)?,
        "fixed-width" => overrides.fixed_width = leaf(value, fixed_width)?,
        "unwrap" => overrides.unwrap = leaf(value, boolean)?,
        "indentation" => overrides.indentation = leaf(value, indentation)?,
        "ordered-lists" => overrides.ordered_lists = leaf(value, boolean)?,
        "list-padding" => overrides.list_padding = leaf(value, boolean)?,
        "list-item-padding" => overrides.list_item_padding = leaf(value, list_item_padding)?,
        "list-interior-padding" => {
            overrides.list_interior_padding = leaf(value, list_interior_padding)?;
        }
        "default-language" => overrides.default_language = leaf(value, default_language)?,
        _ => {
            return Err(format!(
                "unknown rule; expected one of {}",
                RULE_NAMES.join(", ")
            ));
        }
    }
    Ok(())
}

fn leaf<T>(value: &Value, read: fn(&Value) -> Result<T, String>) -> Result<Leaf<T>, String> {
    match value {
        Value::Null => Ok(Leaf::Null),
        other => read(other).map(Leaf::Value),
    }
}

fn boolean(value: &Value) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("expected true or false, found {}", type_name(value)))
}

fn fixed_width(value: &Value) -> Result<FixedWidth, String> {
    match value {
        Value::Bool(false) => Ok(FixedWidth::Off),
        Value::Bool(true) => Ok(FixedWidth::Columns(FIXED_WIDTH_TRUE_COLUMNS)),
        Value::Number(number) => match number.as_u64() {
            Some(columns @ 1..=1000) => Ok(FixedWidth::Columns(columns as u16)),
            _ => Err(format!(
                "expected a whole number from 1 to {FIXED_WIDTH_MAX_COLUMNS}, found {number}"
            )),
        },
        other => Err(format!(
            "expected a whole number from 1 to {FIXED_WIDTH_MAX_COLUMNS}, true, or false, found {}",
            type_name(other)
        )),
    }
}

fn indentation(value: &Value) -> Result<Indentation, String> {
    match value {
        Value::Number(number) => match number.as_u64() {
            Some(2) => Ok(Indentation::Two),
            Some(4) => Ok(Indentation::Four),
            Some(8) => Ok(Indentation::Eight),
            _ => Err(format!("expected 2, 4, 8, or preserve, found {number}")),
        },
        Value::String(text) if text == "preserve" => Ok(Indentation::Preserve),
        Value::String(text) if matches!(text.as_str(), "2" | "4" | "8") => Err(format!(
            "expected the number {text}, not the quoted string \"{text}\""
        )),
        other => Err(format!(
            "expected 2, 4, 8, or preserve, found {}",
            type_name(other)
        )),
    }
}

fn list_item_padding(value: &Value) -> Result<ListItemPadding, String> {
    match value.as_str() {
        Some("preserve") => Ok(ListItemPadding::Preserve),
        Some("none") => Ok(ListItemPadding::None),
        Some("one") => Ok(ListItemPadding::One),
        _ => Err(format!(
            "expected preserve, none, or one, found {}",
            type_name(value)
        )),
    }
}

fn list_interior_padding(value: &Value) -> Result<ListInteriorPadding, String> {
    match value.as_str() {
        Some("preserve") => Ok(ListInteriorPadding::Preserve),
        Some("none") => Ok(ListInteriorPadding::None),
        Some("except-nested-lists") => Ok(ListInteriorPadding::ExceptNestedLists),
        Some("all") => Ok(ListInteriorPadding::All),
        _ => Err(format!(
            "expected preserve, none, except-nested-lists, or all, found {}",
            type_name(value)
        )),
    }
}

fn default_language(value: &Value) -> Result<DefaultLanguage, String> {
    match value {
        Value::String(text) => DefaultLanguage::new(text.clone()),
        other => Err(format!("expected a string, found {}", type_name(other))),
    }
}

/// Describes `value` for an error message: scalars by their text, so
/// `"4"` and `4` read differently, collections by their kind.
fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => format!("\"{text}\""),
        Value::Array(_) => "a list".to_string(),
        Value::Object(_) => "a mapping".to_string(),
    }
}

/// Reads the document's `style.markdown` rules from its parsed frontmatter.
///
/// Passive: nothing is evaluated. A string that still holds `{{ … }}` or
/// `$( … )` syntax is unusable as a setting. `yaml_source` is the frontmatter
/// YAML the value was parsed from, and `yaml_offset` its byte offset in the
/// document, so diagnostics carry document coordinates.
///
/// ## Returns
///
/// [`Leaf::Absent`] or [`Leaf::Null`] when `style` or `style.markdown` is
/// missing or `null`; otherwise the validated overrides.
///
/// ## Errors
///
/// One `clean.invalid-rules` diagnostic per problem. Any problem invalidates
/// the whole `style.markdown` object; the other precedence levels still
/// apply.
pub fn document_rules(
    frontmatter: &serde_yaml_ng::Value,
    yaml_source: &str,
    yaml_offset: usize,
) -> Result<Leaf<MarkdownRuleOverrides>, Vec<CleanDiagnostic>> {
    let style_path = [YamlPathSegment::Key("style".to_string())];
    let markdown_path = [
        YamlPathSegment::Key("style".to_string()),
        YamlPathSegment::Key("markdown".to_string()),
    ];
    let whole_document = 0..yaml_source.len();
    let at_value = |path: &[YamlPathSegment]| {
        locate_yaml_value(yaml_source, path)
            .map(|location| location.span)
            .or_else(|| locate_yaml_key(yaml_source, path))
            .unwrap_or_else(|| whole_document.clone())
    };
    let diagnostic = |span: SourceSpan, message: String| {
        CleanDiagnostic::invalid_rules(span.start + yaml_offset..span.end + yaml_offset, message)
    };

    let Some(style) = frontmatter.as_mapping().and_then(|map| map.get("style")) else {
        return Ok(Leaf::Absent);
    };
    let style = match style {
        serde_yaml_ng::Value::Null => return Ok(Leaf::Null),
        serde_yaml_ng::Value::Mapping(map) => map,
        other => {
            return Err(vec![diagnostic(
                at_value(&style_path),
                format!(
                    "`style` must be a mapping, so its `markdown` rules were ignored (found {})",
                    yaml_kind(other)
                ),
            )]);
        }
    };
    let markdown = match style.get("markdown") {
        None => return Ok(Leaf::Absent),
        Some(serde_yaml_ng::Value::Null) => return Ok(Leaf::Null),
        Some(markdown) => markdown,
    };
    let ignored = "so `style.markdown` was ignored";
    let serde_yaml_ng::Value::Mapping(rules) = markdown else {
        return Err(vec![diagnostic(
            at_value(&markdown_path),
            format!(
                "`style.markdown` must be a mapping of rules, found {}; {ignored}",
                yaml_kind(markdown)
            ),
        )]);
    };

    let mut diagnostics = Vec::new();
    let mut json = Map::new();
    for (key, value) in rules {
        let Some(name) = key.as_str() else {
            diagnostics.push(diagnostic(
                at_value(&markdown_path),
                format!("`style.markdown` has a non-string key; {ignored}"),
            ));
            continue;
        };
        let leaf_path = [
            YamlPathSegment::Key("style".to_string()),
            YamlPathSegment::Key("markdown".to_string()),
            YamlPathSegment::Key(name.to_string()),
        ];
        if let serde_yaml_ng::Value::String(text) = value
            && holds_pending_syntax(text)
        {
            diagnostics.push(diagnostic(
                at_value(&leaf_path),
                format!(
                    "`style.markdown.{name}` holds an expression; formatting settings are \
                     never evaluated, {ignored}"
                ),
            ));
            continue;
        }
        let converted = match serde_json::to_value(value) {
            Ok(converted) => converted,
            Err(error) => {
                diagnostics.push(diagnostic(
                    at_value(&leaf_path),
                    format!("`style.markdown.{name}` cannot be read ({error}); {ignored}"),
                ));
                continue;
            }
        };
        if let Err(message) = validate_rule(name, &converted) {
            let span = if RULE_NAMES.contains(&name) {
                at_value(&leaf_path)
            } else {
                locate_yaml_key(yaml_source, &leaf_path).unwrap_or_else(|| at_value(&markdown_path))
            };
            diagnostics.push(diagnostic(
                span,
                format!("`style.markdown.{name}`: {message}; {ignored}"),
            ));
            continue;
        }
        json.insert(name.to_string(), converted);
    }
    if !diagnostics.is_empty() {
        diagnostics.sort_by_key(|entry| entry.span.start);
        return Err(diagnostics);
    }
    rules_from_json(&Value::Object(json))
        .map(Leaf::Value)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| diagnostic(at_value(&markdown_path), format!("{error}; {ignored}")))
                .collect()
        })
}

fn yaml_kind(value: &serde_yaml_ng::Value) -> &'static str {
    match value {
        serde_yaml_ng::Value::Null => "null",
        serde_yaml_ng::Value::Bool(_) => "a boolean",
        serde_yaml_ng::Value::Number(_) => "a number",
        serde_yaml_ng::Value::String(_) => "a string",
        serde_yaml_ng::Value::Sequence(_) => "a list",
        serde_yaml_ng::Value::Mapping(_) => "a mapping",
        serde_yaml_ng::Value::Tagged(_) => "a tagged value",
    }
}
