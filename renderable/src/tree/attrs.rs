//! Presentational attributes attached to a [`RenderNode`].
//!
//! [`RenderNode`]: crate::tree::RenderNode

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A namespace for render hints stored in [`NodeAttrs::data`].
///
/// Namespaces provide a structured way to organize hints without key collisions.
/// The full key is constructed as `"{namespace}.{key}"`.
///
/// ## Examples
///
/// ```
/// use renderable::tree::{HintNamespace, NodeAttrs};
/// use serde_json::json;
///
/// let mut attrs = NodeAttrs::default();
/// attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(2));
/// assert_eq!(attrs.get_hint(HintNamespace::LAYOUT, "margin_top"), Some(&json!(2)));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HintNamespace(pub &'static str);

impl HintNamespace {
    /// Layout hints (margins, alignment, etc.).
    pub const LAYOUT: HintNamespace = HintNamespace("renderable.layout");
    /// Style hints (color, background, emphasis, border, fill).
    pub const STYLE: HintNamespace = HintNamespace("renderable.style");
    /// List-specific hints (bullet style, numbering, etc.).
    pub const LIST: HintNamespace = HintNamespace("renderable.list");
    /// Table-specific hints (column widths, borders, etc.).
    pub const TABLE: HintNamespace = HintNamespace("renderable.table");
    /// Code block hints (language, highlighting, etc.).
    pub const CODE: HintNamespace = HintNamespace("renderable.code");
    /// Terminal-specific hints (colors, escape sequences, etc.).
    pub const TERMINAL: HintNamespace = HintNamespace("renderable.terminal");
    /// Progress widget hints.
    pub const WIDGET_PROGRESS: HintNamespace = HintNamespace("renderable.widget.progress");
    /// Column widget hints.
    pub const WIDGET_COLUMNS: HintNamespace = HintNamespace("renderable.widget.columns");
    /// Task-list widget hints.
    pub const WIDGET_TASK: HintNamespace = HintNamespace("renderable.widget.task");
}

/// How a sequence of children is joined when rendered.
///
/// Normal document [`NodeKind::Root`] rendering treats children as document
/// blocks and joins them with blank-line separators. A `Compose`-style
/// sequence preserves ordered children without that document-block spacing.
///
/// [`NodeKind::Root`]: crate::tree::NodeKind::Root
///
/// ## Examples
///
/// ```
/// use renderable::tree::{NodeAttrs, RenderNode, SequenceJoin};
///
/// let mut root = RenderNode::root(vec![RenderNode::text("foo")]);
/// root.attrs.set_sequence_join(SequenceJoin::None);
/// assert_eq!(root.attrs.sequence_join(), Some(SequenceJoin::None));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SequenceJoin {
    /// Render children in order with no renderer-inserted separator.
    #[default]
    None,
}

impl Serialize for SequenceJoin {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.to_token())
    }
}

impl<'de> Deserialize<'de> for SequenceJoin {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let token = String::deserialize(d)?;
        SequenceJoin::from_token(&token)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid SequenceJoin token: {token}")))
    }
}

impl SequenceJoin {
    /// Returns the compact string token for this join policy.
    #[must_use]
    pub fn to_token(self) -> &'static str {
        match self {
            SequenceJoin::None => "none",
        }
    }

    /// Parses a compact string token into a join policy.
    ///
    /// Returns `None` for an unrecognized token.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "none" => Some(SequenceJoin::None),
            _ => None,
        }
    }
}

/// How a list presents its item markers.
///
/// Normal ordered and unordered lists use the default marker presentation
/// ([`ListMarkerPolicy::Default`]). A component with a bespoke marker layout —
/// such as `FileSystem`, whose terminal output uses connector geometry — can
/// request a different policy without baking presentation into text nodes.
///
/// ## Examples
///
/// ```
/// use renderable::tree::ListMarkerPolicy;
///
/// assert_eq!(ListMarkerPolicy::default(), ListMarkerPolicy::Default);
/// assert_eq!(ListMarkerPolicy::TreeConnectors.to_token(), "tree_connectors");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListMarkerPolicy {
    /// The renderer's normal marker presentation (ordinal numbers or bullets).
    #[default]
    Default,
    /// No marker at all — items render without a leading bullet or number.
    None,
    /// Terminal box-drawing connector geometry (`├──`, `└──`, `│`).
    ///
    /// Renderers that cannot faithfully represent connector geometry degrade
    /// this to a native nested list or no-marker presentation.
    TreeConnectors,
}

impl ListMarkerPolicy {
    /// Returns the compact string token for this marker policy.
    #[must_use]
    pub fn to_token(self) -> &'static str {
        match self {
            ListMarkerPolicy::Default => "default",
            ListMarkerPolicy::None => "none",
            ListMarkerPolicy::TreeConnectors => "tree_connectors",
        }
    }

    /// Parses a compact string token into a marker policy.
    ///
    /// Returns `None` for an unrecognized token.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "default" => Some(ListMarkerPolicy::Default),
            "none" => Some(ListMarkerPolicy::None),
            "tree_connectors" => Some(ListMarkerPolicy::TreeConnectors),
            _ => None,
        }
    }

    /// Whether this is the implicit [`ListMarkerPolicy::Default`] policy.
    ///
    /// Used as the `skip_serializing_if` predicate for the typed
    /// [`NodeAttrs::list_marker_policy`] field so the common case adds no key.
    pub(crate) fn is_default(&self) -> bool {
        *self == ListMarkerPolicy::default()
    }
}

impl Serialize for ListMarkerPolicy {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.to_token())
    }
}

impl<'de> Deserialize<'de> for ListMarkerPolicy {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let token = String::deserialize(d)?;
        ListMarkerPolicy::from_token(&token).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid ListMarkerPolicy token: {token}"))
        })
    }
}

/// The state of a task-list item.
///
/// A `Todo` component has five public states, but GFM task-list syntax only
/// distinguishes checked from unchecked. The render tree carries the richer
/// state in [`TaskHints`] so terminal rendering can present a state-specific
/// glyph while Markdown degrades to portable `- [x]` / `- [ ]` syntax.
///
/// ## Examples
///
/// ```
/// use renderable::tree::TaskState;
///
/// assert_eq!(TaskState::default(), TaskState::Open);
/// assert_eq!(TaskState::Completed.to_token(), "completed");
/// assert_eq!(TaskState::from_token("blocked"), Some(TaskState::Blocked));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskState {
    /// An open, not-yet-started task.
    #[default]
    Open,
    /// A task that is in progress.
    InProgress,
    /// A finished task.
    Completed,
    /// A task that is blocked.
    Blocked,
    /// A task that has been cancelled.
    Cancelled,
}

impl TaskState {
    /// Returns the compact string token for this task state.
    #[must_use]
    pub fn to_token(self) -> &'static str {
        match self {
            TaskState::Open => "open",
            TaskState::InProgress => "in_progress",
            TaskState::Completed => "completed",
            TaskState::Blocked => "blocked",
            TaskState::Cancelled => "cancelled",
        }
    }

    /// Parses a compact string token into a task state.
    ///
    /// Returns `None` for an unrecognized token.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "open" => Some(TaskState::Open),
            "in_progress" => Some(TaskState::InProgress),
            "completed" => Some(TaskState::Completed),
            "blocked" => Some(TaskState::Blocked),
            "cancelled" => Some(TaskState::Cancelled),
            _ => None,
        }
    }
}

impl Serialize for TaskState {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.to_token())
    }
}

impl<'de> Deserialize<'de> for TaskState {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let token = String::deserialize(d)?;
        TaskState::from_token(&token)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid TaskState token: {token}")))
    }
}

/// Render hints for a task-list item.
///
/// A `Todo` component projects to a one-item [`NodeKind::List`] with a
/// [`NodeKind::ListItem`] carrying these hints. Renderers that recognize the
/// hint present a state-specific marker; renderers that do not fall back to
/// the GFM checkbox derived from the item's `checked` field.
///
/// [`NodeKind::List`]: crate::tree::NodeKind::List
/// [`NodeKind::ListItem`]: crate::tree::NodeKind::ListItem
///
/// ## Examples
///
/// ```
/// use renderable::tree::{NodeAttrs, TaskHints, TaskState};
///
/// let mut attrs = NodeAttrs::default();
/// attrs.set_task_hints(&TaskHints { state: TaskState::InProgress });
/// assert_eq!(attrs.task_hints().unwrap().state, TaskState::InProgress);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TaskHints {
    /// The task's state.
    pub state: TaskState,
}

/// Render hints for list nodes.
///
/// Components that produce a [`NodeKind::List`] node can attach these hints
/// to its [`NodeAttrs`] so the terminal renderer reproduces bespoke list
/// formatting (custom bullets, hanging indent, child indentation).
///
/// [`NodeKind::List`]: crate::tree::NodeKind::List
///
/// ## Examples
///
/// ```
/// use renderable::tree::ListRenderHints;
///
/// let hints = ListRenderHints {
///     bullet: Some("* ".into()),
///     ..Default::default()
/// };
/// assert!(hints.hanging_indent);
/// assert_eq!(hints.indent_children, None);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListRenderHints {
    /// Custom bullet string for unordered lists.
    pub bullet: Option<String>,
    /// Whether continuation lines align after the prefix.
    pub hanging_indent: bool,
    /// Indentation width for nested block children.
    pub indent_children: Option<u32>,
}

impl Default for ListRenderHints {
    fn default() -> Self {
        Self {
            bullet: None,
            hanging_indent: true,
            indent_children: None,
        }
    }
}

/// Render hints for code-block nodes.
///
/// Components that produce a [`NodeKind::Code`] node can attach these hints
/// to its [`NodeAttrs`] so renderers reproduce bespoke code-block formatting
/// (a language header row, an explicit label, syntax highlighting).
///
/// [`NodeKind::Code`]: crate::tree::NodeKind::Code
///
/// ## Examples
///
/// ```
/// use renderable::tree::CodeRenderHints;
///
/// let hints = CodeRenderHints {
///     header_row: true,
///     language_label: Some("yaml".into()),
///     highlight: true,
/// };
/// assert!(hints.header_row);
/// assert_eq!(hints.language_label.as_deref(), Some("yaml"));
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CodeRenderHints {
    /// Whether to render a header row with the language label.
    pub header_row: bool,
    /// Explicit language label override.
    pub language_label: Option<String>,
    /// Whether syntax highlighting is requested.
    pub highlight: bool,
}

/// Render hints for progress-bar widgets.
///
/// A `Progress` component projects to a [`NodeKind::Paragraph`] carrying these
/// hints in its [`NodeAttrs`]. Renderers that recognize the hints draw a
/// progress bar; renderers that do not fall back to the paragraph's plain text.
///
/// [`NodeKind::Paragraph`]: crate::tree::NodeKind::Paragraph
///
/// ## Examples
///
/// ```
/// use renderable::tree::ProgressHints;
///
/// let hints = ProgressHints {
///     value: 0.75,
///     ..Default::default()
/// };
/// assert_eq!(hints.bar_width, 20);
/// assert_eq!(hints.fill_char, '█');
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressHints {
    /// Completion value, 0.0..=1.0.
    pub value: f32,
    /// Width of the bar in characters.
    pub bar_width: u32,
    /// Character for the filled portion.
    pub fill_char: char,
    /// Character for the empty portion.
    pub empty_char: char,
    /// Left bracket character.
    pub left_bracket: char,
    /// Right bracket character.
    pub right_bracket: char,
    /// Color for the filled portion of the track, if any. A `Color` so it
    /// degrades across terminal color depths through the shared lowering.
    pub filled_color: Option<crate::color::Color>,
    /// Color for the empty portion of the track, if any.
    pub empty_color: Option<crate::color::Color>,
    /// Color for the left and right bracket glyphs, if any.
    pub bracket_color: Option<crate::color::Color>,
}

impl Default for ProgressHints {
    fn default() -> Self {
        Self {
            value: 0.0,
            bar_width: 20,
            fill_char: '█',
            empty_char: '·',
            left_bracket: '[',
            right_bracket: ']',
            filled_color: None,
            empty_color: None,
            bracket_color: None,
        }
    }
}

/// How a column's width is specified.
///
/// ## Examples
///
/// ```
/// use renderable::tree::ColumnWidthKind;
///
/// let fixed = ColumnWidthKind::Fixed(30);
/// let half = ColumnWidthKind::Percent(0.5);
/// assert_ne!(fixed, half);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColumnWidthKind {
    /// Fixed width in characters.
    Fixed(u32),
    /// Fraction of available width, 0.0..=1.0.
    Percent(f32),
}

/// Render hints for two-column layout widgets.
///
/// A `TwoColumn` component projects to a [`NodeKind::BlockQuote`] container
/// carrying these hints in its [`NodeAttrs`]. Renderers that recognize the
/// hints lay the children out side by side; renderers that do not fall back
/// to a plain block quote.
///
/// [`NodeKind::BlockQuote`]: crate::tree::NodeKind::BlockQuote
///
/// ## Examples
///
/// ```
/// use renderable::tree::ColumnsHints;
///
/// let hints = ColumnsHints {
///     left_count: 1,
///     ..Default::default()
/// };
/// assert_eq!(hints.gap, 3);
/// assert!(hints.stack_below);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnsHints {
    /// Character gap between the two columns.
    pub gap: u32,
    /// How the left column's width is determined.
    pub left_width: ColumnWidthKind,
    /// Number of leading children belonging to the left column;
    /// remaining children belong to the right column.
    pub left_count: usize,
    /// Whether to stack the columns vertically below a width threshold.
    pub stack_below: bool,
}

impl Default for ColumnsHints {
    fn default() -> Self {
        Self {
            gap: 3,
            left_width: ColumnWidthKind::Percent(0.5),
            left_count: 0,
            stack_below: true,
        }
    }
}

/// A column's conditional visibility, keyed to the renderable width.
///
/// Mirrors the `Conditional` enum used by the bespoke `Table` component.
/// Serialized to a compact string form: `"always"`, `"gt:{n}"`, `"le:{n}"`.
///
/// ## Examples
///
/// ```
/// use renderable::tree::ColumnConditional;
///
/// assert_eq!(ColumnConditional::Always.to_token(), "always");
/// assert_eq!(ColumnConditional::WidthGreaterThan(80).to_token(), "gt:80");
/// assert_eq!(
///     ColumnConditional::from_token("le:40"),
///     Some(ColumnConditional::LessThanOrEqual(40)),
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnConditional {
    /// Column is always visible.
    #[default]
    Always,
    /// Visible when renderable width is greater than the threshold.
    WidthGreaterThan(u32),
    /// Visible when renderable width is less than or equal to the threshold.
    LessThanOrEqual(u32),
}

impl ColumnConditional {
    /// Returns the compact string token for this conditional.
    #[must_use]
    pub fn to_token(self) -> String {
        match self {
            ColumnConditional::Always => "always".to_string(),
            ColumnConditional::WidthGreaterThan(n) => format!("gt:{n}"),
            ColumnConditional::LessThanOrEqual(n) => format!("le:{n}"),
        }
    }

    /// Parses a compact string token into a conditional.
    ///
    /// Returns `None` for an unrecognized or malformed token.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        if token == "always" {
            return Some(ColumnConditional::Always);
        }
        if let Some(rest) = token.strip_prefix("gt:") {
            return rest.parse().ok().map(ColumnConditional::WidthGreaterThan);
        }
        if let Some(rest) = token.strip_prefix("le:") {
            return rest.parse().ok().map(ColumnConditional::LessThanOrEqual);
        }
        None
    }
}

impl Serialize for ColumnConditional {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_token())
    }
}

impl<'de> Deserialize<'de> for ColumnConditional {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let token = String::deserialize(d)?;
        ColumnConditional::from_token(&token).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid ColumnConditional token: {token}"))
        })
    }
}

/// Per-column render hints for a projected [`NodeKind::Table`] node.
///
/// A `Table` component records one set of these per column in the table
/// node's [`TableHints::columns`] map, keyed by column index. Renderers that
/// recognize the hints reproduce bespoke column width and visibility
/// behavior; renderers that do not fall back to plain table layout.
///
/// [`NodeKind::Table`]: crate::tree::NodeKind::Table
///
/// ## Examples
///
/// ```
/// use renderable::tree::{ColumnConditional, NodeAttrs, TableColumnHints};
///
/// let mut attrs = NodeAttrs::default();
/// attrs.set_table_column_hints(0, &TableColumnHints {
///     min_width: Some(8),
///     conditional: ColumnConditional::WidthGreaterThan(80),
///     ..Default::default()
/// });
/// let hints = attrs.table_column_hints(0);
/// assert_eq!(hints.min_width, Some(8));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TableColumnHints {
    /// Minimum column width in characters.
    pub min_width: Option<u32>,
    /// Maximum column width in characters.
    pub max_width: Option<u32>,
    /// Fixed column width in characters.
    pub fixed_width: Option<u32>,
    /// Conditional visibility keyed to the renderable width.
    pub conditional: ColumnConditional,
    /// Whether this column may be dropped when the table cannot otherwise
    /// fit in the available width. A column may be droppable with or
    /// without a [`Self::drop_note`]; this flag is the authoritative
    /// "is droppable" signal and must be preserved across the render tree.
    pub droppable: bool,
    /// Note appended after the table when this column is dropped. A
    /// non-empty note implies [`Self::droppable`] is `true`, but a column
    /// can be droppable with no note (silent drop).
    pub drop_note: Option<String>,
    /// Whether all cells in this column align uniformly.
    pub uniform_alignment: bool,
}

/// Render hints for a single projected [`NodeKind::TableCell`] node.
///
/// A `Table` component records these on each data cell so renderers can
/// recover the original typed value, the cell semantics, and per-cell
/// alignment.
///
/// [`NodeKind::TableCell`]: crate::tree::NodeKind::TableCell
///
/// ## Examples
///
/// ```
/// use renderable::tree::{NodeAttrs, TableCellHints};
/// use serde_json::json;
///
/// let mut attrs = NodeAttrs::default();
/// attrs.set_table_cell_hints(&TableCellHints {
///     kind: "currency".into(),
///     raw_value: json!(1234.56),
///     alignment: "right".into(),
///     vertical_alignment: "top".into(),
/// });
/// let hints = attrs.table_cell_hints().expect("cell hints present");
/// assert_eq!(hints.kind, "currency");
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableCellHints {
    /// Cell kind: `"text"`, `"integer"`, `"float"`, or `"currency"`.
    pub kind: String,
    /// The original typed value, preserved as JSON.
    pub raw_value: serde_json::Value,
    /// Horizontal alignment: `"left"`, `"center"`, or `"right"`.
    pub alignment: String,
    /// Vertical alignment: `"top"`, `"middle"`, or `"bottom"`.
    pub vertical_alignment: String,
}

/// Terminal-specific render hints for a projected [`NodeKind::Table`] node.
///
/// These are stored in the table node's [`TableHints::terminal`] and
/// influence terminal-only behaviors. Markdown and browser renderers ignore
/// them.
///
/// [`NodeKind::Table`]: crate::tree::NodeKind::Table
///
/// ## Examples
///
/// ```
/// use renderable::tree::{NodeAttrs, TableTerminalHints};
///
/// let mut attrs = NodeAttrs::default();
/// attrs.set_table_terminal_hints(&TableTerminalHints {
///     prefer_cursor_alignment: true,
///     alternate_background: true,
///     ..Default::default()
/// });
/// let hints = attrs.table_terminal_hints();
/// assert!(hints.prefer_cursor_alignment);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TableTerminalHints {
    /// Whether to use cursor positioning for cell alignment.
    pub prefer_cursor_alignment: bool,
    /// Whether even data rows receive an alternating background tint.
    pub alternate_background: bool,
    /// Whether even data rows receive an alternating text tint.
    pub alternate_text_color: bool,
    /// Explicit background stripe color. `None` selects the renderer's
    /// adaptive default. A [`Color`](crate::color::Color) so it degrades
    /// across terminal color depths through the shared lowering.
    pub stripe_bg: Option<crate::color::Color>,
    /// Explicit text stripe color. `None` selects the adaptive default.
    pub stripe_text: Option<crate::color::Color>,
}

/// A [`NodeKind::Table`] node's hints, grouped together.
///
/// Column hints are keyed per column index, so columns can be addressed
/// independently while the terminal striping and title hints stay co-resident
/// on the same table node.
///
/// [`NodeKind::Table`]: crate::tree::NodeKind::Table
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableHints {
    /// Per-column hints, keyed by zero-based column index.
    pub columns: BTreeMap<usize, TableColumnHints>,
    /// Terminal-only striping and cursor-alignment hints.
    pub terminal: Option<TableTerminalHints>,
    /// Optional table title/caption.
    pub title: Option<String>,
}

/// Per-component render hints; only nodes of the matching kind carry one.
///
/// A render node carries at most one `ComponentHints`, matched to its
/// [`NodeKind`](crate::tree::NodeKind): a list node may hold
/// [`ComponentHints::List`], a code node [`ComponentHints::Code`], and so on.
/// Stored as the typed [`NodeAttrs::component`] field rather than serialized
/// into [`NodeAttrs::data`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum ComponentHints {
    /// Hints for a list node.
    List(ListRenderHints),
    /// Hints for a code-block node.
    Code(CodeRenderHints),
    /// Hints for a projected progress-bar paragraph node.
    Progress(ProgressHints),
    /// Hints for a projected two-column block-quote node.
    Columns(ColumnsHints),
    /// Hints for a projected task-list item node.
    Task(TaskHints),
    /// Hints for a table node.
    Table(TableHints),
    /// Hints for a single table-cell node.
    TableCell(TableCellHints),
}

/// Optional presentational attributes carried by every render node.
///
/// All fields are optional; the [`Default`] value is an empty set of
/// attributes. First-class presentation — `layout`, `style`, the two hot
/// per-node hints (`sequence_join`, `list_marker_policy`), and the per-kind
/// `component` hint group — lives in typed sparse fields, so reads cost no
/// serde round-trip. `data` carries only package-local extension namespaces
/// (e.g. `darkmatter.hr.*`).
///
/// ## Examples
///
/// ```
/// use renderable::tree::NodeAttrs;
///
/// let attrs = NodeAttrs::default();
/// assert!(attrs.id.is_none());
/// assert!(attrs.classes.is_empty());
/// assert!(attrs.layout.is_none());
/// assert!(attrs.component.is_none());
/// assert!(attrs.data.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAttrs {
    /// Optional unique identifier for the node.
    pub id: Option<String>,
    /// CSS-style class names associated with the node.
    pub classes: Vec<String>,
    /// Block-positioning layout, when set. Permitted on block-level nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Box<crate::layout::Layout>>,
    /// Target-agnostic appearance, when set. Permitted on block nodes and
    /// inline `Span` nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Box<crate::style::Style>>,
    /// Child-sequence join policy (Root nodes only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_join: Option<SequenceJoin>,
    /// List marker presentation policy.
    #[serde(default, skip_serializing_if = "ListMarkerPolicy::is_default")]
    pub list_marker_policy: ListMarkerPolicy,
    /// Per-component render hints, when the node kind carries any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<Box<ComponentHints>>,
    /// Extension-namespace structured data keyed by name (e.g.
    /// `darkmatter.hr.*`). Not used for first-class `renderable.*` hints.
    pub data: BTreeMap<String, serde_json::Value>,
}

// Test-only `data`-bag access counter, partitioned by namespace. The tuple is
// `(renderable_owned, extension)`: bag accesses whose namespace starts with
// `"renderable."` increment the first slot, all others the second. The
// structural perf gate folds a styled corpus and asserts the first slot stayed
// at zero — every first-class hint must read from a typed field, never
// round-trip through `NodeAttrs::data`.
#[cfg(test)]
thread_local! {
    pub(crate) static HINT_ACCESSES: std::cell::Cell<(u64, u64)> =
        const { std::cell::Cell::new((0, 0)) };
}

/// Resets the [`HINT_ACCESSES`] counter to `(0, 0)`.
#[cfg(test)]
pub(crate) fn reset_hint_accesses() {
    HINT_ACCESSES.with(|c| c.set((0, 0)));
}

/// Returns the current `(renderable_owned, extension)` bag-access counts.
#[cfg(test)]
pub(crate) fn hint_accesses() -> (u64, u64) {
    HINT_ACCESSES.with(std::cell::Cell::get)
}

/// Classifies one bag access by namespace and bumps the matching counter.
#[cfg(test)]
fn record_hint_access(ns: HintNamespace) {
    HINT_ACCESSES.with(|c| {
        let (r, e) = c.get();
        if ns.0.starts_with("renderable.") {
            c.set((r + 1, e));
        } else {
            c.set((r, e + 1));
        }
    });
}

impl NodeAttrs {
    /// Sets a hint value in the given namespace.
    ///
    /// The full key is constructed as `"{namespace}.{key}"`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{HintNamespace, NodeAttrs};
    /// use serde_json::json;
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(2));
    /// ```
    pub fn set_hint(&mut self, ns: HintNamespace, key: &str, value: serde_json::Value) {
        #[cfg(test)]
        record_hint_access(ns);
        let full_key = format!("{}.{}", ns.0, key);
        self.data.insert(full_key, value);
    }

    /// Gets a hint value from the given namespace.
    ///
    /// Returns `None` if the hint does not exist.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{HintNamespace, NodeAttrs};
    /// use serde_json::json;
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(2));
    /// assert_eq!(attrs.get_hint(HintNamespace::LAYOUT, "margin_top"), Some(&json!(2)));
    /// assert_eq!(attrs.get_hint(HintNamespace::LAYOUT, "nonexistent"), None);
    /// ```
    pub fn get_hint(&self, ns: HintNamespace, key: &str) -> Option<&serde_json::Value> {
        #[cfg(test)]
        record_hint_access(ns);
        // The common node carries no hints at all (plain text runs, attribute-less
        // table cells, list items). Skip building the `"{ns}.{key}"` lookup string
        // for them — every renderer probes `style()` / `layout()` per node, so this
        // avoids a `format!` allocation per probe on the hot tree-walk path.
        if self.data.is_empty() {
            return None;
        }
        let full_key = format!("{}.{}", ns.0, key);
        self.data.get(&full_key)
    }

    /// Removes a hint from the given namespace.
    ///
    /// Returns the removed value, or `None` if the hint did not exist.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{HintNamespace, NodeAttrs};
    /// use serde_json::json;
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(2));
    /// let removed = attrs.remove_hint(HintNamespace::LAYOUT, "margin_top");
    /// assert_eq!(removed, Some(json!(2)));
    /// assert_eq!(attrs.get_hint(HintNamespace::LAYOUT, "margin_top"), None);
    /// ```
    pub fn remove_hint(&mut self, ns: HintNamespace, key: &str) -> Option<serde_json::Value> {
        #[cfg(test)]
        record_hint_access(ns);
        let full_key = format!("{}.{}", ns.0, key);
        self.data.remove(&full_key)
    }

    /// Returns a mutable reference to this node's [`TableHints`], initializing
    /// the [`ComponentHints::Table`] variant if the node does not already carry
    /// one. Lets the per-column, terminal, and title setters mutate their own
    /// slice of the table hints without clobbering a co-resident sub-hint.
    fn table_hints_mut(&mut self) -> &mut TableHints {
        if !matches!(self.component.as_deref(), Some(ComponentHints::Table(_))) {
            self.component = Some(Box::new(ComponentHints::Table(TableHints::default())));
        }
        match self.component.as_deref_mut() {
            Some(ComponentHints::Table(t)) => t,
            _ => unreachable!("just initialized to Table"),
        }
    }

    /// Stores [`ListRenderHints`] as the node's [`ComponentHints::List`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{ListRenderHints, NodeAttrs};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_list_hints(&ListRenderHints {
    ///     bullet: Some("* ".into()),
    ///     hanging_indent: false,
    ///     indent_children: Some(4),
    /// });
    /// let hints = attrs.list_hints();
    /// assert_eq!(hints.bullet.as_deref(), Some("* "));
    /// assert!(!hints.hanging_indent);
    /// assert_eq!(hints.indent_children, Some(4));
    /// ```
    pub fn set_list_hints(&mut self, hints: &ListRenderHints) {
        self.component = Some(Box::new(ComponentHints::List(hints.clone())));
    }

    /// Reads this node's [`ListRenderHints`], if it carries any.
    ///
    /// A node without list hints falls back to [`ListRenderHints::default`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // An empty attribute set yields the default hints.
    /// let attrs = NodeAttrs::default();
    /// let hints = attrs.list_hints();
    /// assert_eq!(hints.bullet, None);
    /// assert!(hints.hanging_indent);
    /// ```
    #[must_use]
    pub fn list_hints(&self) -> ListRenderHints {
        match self.component.as_deref() {
            Some(ComponentHints::List(h)) => h.clone(),
            _ => ListRenderHints::default(),
        }
    }

    /// Borrowed accessor for renderer hot paths (no clone).
    ///
    /// Returns `None` when the node carries no list hints.
    #[must_use]
    pub fn list_hints_ref(&self) -> Option<&ListRenderHints> {
        match self.component.as_deref() {
            Some(ComponentHints::List(h)) => Some(h),
            _ => None,
        }
    }

    /// Stores [`CodeRenderHints`] as the node's [`ComponentHints::Code`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{CodeRenderHints, NodeAttrs};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_code_hints(&CodeRenderHints {
    ///     header_row: true,
    ///     language_label: Some("yaml".into()),
    ///     highlight: true,
    /// });
    /// let hints = attrs.code_hints();
    /// assert!(hints.header_row);
    /// assert_eq!(hints.language_label.as_deref(), Some("yaml"));
    /// assert!(hints.highlight);
    /// ```
    pub fn set_code_hints(&mut self, hints: &CodeRenderHints) {
        self.component = Some(Box::new(ComponentHints::Code(hints.clone())));
    }

    /// Reads this node's [`CodeRenderHints`], if it carries any.
    ///
    /// A node without code hints falls back to [`CodeRenderHints::default`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // An empty attribute set yields the default hints.
    /// let attrs = NodeAttrs::default();
    /// let hints = attrs.code_hints();
    /// assert!(!hints.header_row);
    /// assert_eq!(hints.language_label, None);
    /// assert!(!hints.highlight);
    /// ```
    #[must_use]
    pub fn code_hints(&self) -> CodeRenderHints {
        match self.component.as_deref() {
            Some(ComponentHints::Code(h)) => h.clone(),
            _ => CodeRenderHints::default(),
        }
    }

    /// Borrowed accessor for renderer hot paths (no clone).
    ///
    /// Returns `None` when the node carries no code hints.
    #[must_use]
    pub fn code_hints_ref(&self) -> Option<&CodeRenderHints> {
        match self.component.as_deref() {
            Some(ComponentHints::Code(h)) => Some(h),
            _ => None,
        }
    }

    /// Stores [`ProgressHints`] as the node's [`ComponentHints::Progress`].
    ///
    /// The presence of the hint marks the paragraph as a projected progress
    /// widget; [`NodeAttrs::progress_hints`] returns `Some` for it.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{NodeAttrs, ProgressHints};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_progress_hints(&ProgressHints {
    ///     value: 0.5,
    ///     ..Default::default()
    /// });
    /// assert_eq!(attrs.progress_hints().unwrap().value, 0.5);
    /// ```
    pub fn set_progress_hints(&mut self, hints: &ProgressHints) {
        self.component = Some(Box::new(ComponentHints::Progress(hints.clone())));
    }

    /// Reads this node's [`ProgressHints`], if it carries any.
    ///
    /// Returns `None` when the node is not a projected progress widget, so
    /// renderers can detect whether a paragraph should draw a bar.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // A plain paragraph carries no progress hints.
    /// assert!(NodeAttrs::default().progress_hints().is_none());
    /// ```
    #[must_use]
    pub fn progress_hints(&self) -> Option<ProgressHints> {
        match self.component.as_deref() {
            Some(ComponentHints::Progress(h)) => Some(h.clone()),
            _ => None,
        }
    }

    /// Stores [`ColumnsHints`] as the node's [`ComponentHints::Columns`].
    ///
    /// The presence of the hint marks the block quote as a projected
    /// two-column widget; [`NodeAttrs::columns_hints`] returns `Some` for it.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{ColumnsHints, NodeAttrs};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_columns_hints(&ColumnsHints {
    ///     left_count: 2,
    ///     ..Default::default()
    /// });
    /// assert_eq!(attrs.columns_hints().unwrap().left_count, 2);
    /// ```
    pub fn set_columns_hints(&mut self, hints: &ColumnsHints) {
        self.component = Some(Box::new(ComponentHints::Columns(hints.clone())));
    }

    /// Reads this node's [`ColumnsHints`], if it carries any.
    ///
    /// Returns `None` when the node is not a projected two-column widget.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // A plain block quote carries no column hints.
    /// assert!(NodeAttrs::default().columns_hints().is_none());
    /// ```
    #[must_use]
    pub fn columns_hints(&self) -> Option<ColumnsHints> {
        match self.component.as_deref() {
            Some(ComponentHints::Columns(h)) => Some(h.clone()),
            _ => None,
        }
    }

    /// Stores [`TableColumnHints`] for column `index` in this node's
    /// [`ComponentHints::Table`], keyed by column index.
    ///
    /// Co-resident with the terminal and title table hints; setting one does
    /// not clobber the others.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{NodeAttrs, TableColumnHints};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_table_column_hints(2, &TableColumnHints {
    ///     fixed_width: Some(12),
    ///     ..Default::default()
    /// });
    /// assert_eq!(attrs.table_column_hints(2).fixed_width, Some(12));
    /// ```
    pub fn set_table_column_hints(&mut self, index: usize, hints: &TableColumnHints) {
        self.table_hints_mut().columns.insert(index, hints.clone());
    }

    /// Reads [`TableColumnHints`] for column `index` from this node's
    /// [`ComponentHints::Table`].
    ///
    /// A column without stored hints falls back to
    /// [`TableColumnHints::default`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // An empty attribute set yields the default column hints.
    /// let hints = NodeAttrs::default().table_column_hints(0);
    /// assert_eq!(hints.min_width, None);
    /// ```
    #[must_use]
    pub fn table_column_hints(&self, index: usize) -> TableColumnHints {
        match self.component.as_deref() {
            Some(ComponentHints::Table(t)) => t.columns.get(&index).cloned().unwrap_or_default(),
            _ => TableColumnHints::default(),
        }
    }

    /// Stores [`TableCellHints`] as the node's [`ComponentHints::TableCell`].
    ///
    /// The presence of the hint marks the cell as a projected typed cell;
    /// [`NodeAttrs::table_cell_hints`] returns `Some` for it.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{NodeAttrs, TableCellHints};
    /// use serde_json::json;
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_table_cell_hints(&TableCellHints {
    ///     kind: "integer".into(),
    ///     raw_value: json!(42),
    ///     alignment: "right".into(),
    ///     vertical_alignment: "top".into(),
    /// });
    /// assert!(attrs.table_cell_hints().is_some());
    /// ```
    pub fn set_table_cell_hints(&mut self, hints: &TableCellHints) {
        self.component = Some(Box::new(ComponentHints::TableCell(hints.clone())));
    }

    /// Reads this node's [`TableCellHints`], if it carries any.
    ///
    /// Returns `None` when the cell is not a projected typed cell.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // A plain table cell carries no cell hints.
    /// assert!(NodeAttrs::default().table_cell_hints().is_none());
    /// ```
    #[must_use]
    pub fn table_cell_hints(&self) -> Option<TableCellHints> {
        match self.component.as_deref() {
            Some(ComponentHints::TableCell(h)) => Some(h.clone()),
            _ => None,
        }
    }

    /// Stores [`TableTerminalHints`] in this node's [`ComponentHints::Table`].
    ///
    /// Co-resident with the per-column and title table hints.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{NodeAttrs, TableTerminalHints};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_table_terminal_hints(&TableTerminalHints {
    ///     alternate_text_color: true,
    ///     ..Default::default()
    /// });
    /// assert!(attrs.table_terminal_hints().alternate_text_color);
    /// ```
    pub fn set_table_terminal_hints(&mut self, hints: &TableTerminalHints) {
        self.table_hints_mut().terminal = Some(hints.clone());
    }

    /// Reads this node's [`TableTerminalHints`] from its
    /// [`ComponentHints::Table`].
    ///
    /// A table without terminal hints falls back to
    /// [`TableTerminalHints::default`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // An empty attribute set yields all-false terminal hints.
    /// let hints = NodeAttrs::default().table_terminal_hints();
    /// assert!(!hints.prefer_cursor_alignment);
    /// ```
    #[must_use]
    pub fn table_terminal_hints(&self) -> TableTerminalHints {
        match self.component.as_deref() {
            Some(ComponentHints::Table(t)) => t.terminal.clone().unwrap_or_default(),
            _ => TableTerminalHints::default(),
        }
    }

    /// Stores a [`Layout`](crate::layout::Layout) as the typed
    /// [`NodeAttrs::layout`] field.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::layout::Layout;
    /// use renderable::tree::NodeAttrs;
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_layout(&Layout::default());
    /// assert_eq!(attrs.layout(), Some(Layout::default()));
    /// ```
    pub fn set_layout(&mut self, layout: &crate::layout::Layout) {
        self.layout = Some(Box::new(layout.clone()));
    }

    /// Reads the [`Layout`](crate::layout::Layout) stored on this node, if any.
    ///
    /// Returns `None` when no layout is set.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // An empty attribute set carries no layout.
    /// assert!(NodeAttrs::default().layout().is_none());
    /// ```
    #[must_use]
    pub fn layout(&self) -> Option<crate::layout::Layout> {
        self.layout.as_deref().cloned()
    }

    /// Borrowed accessor for renderer hot paths (no clone).
    ///
    /// Returns `None` when no layout is set.
    #[must_use]
    pub fn layout_ref(&self) -> Option<&crate::layout::Layout> {
        self.layout.as_deref()
    }

    /// Stores a [`Style`](crate::style::Style) as the typed
    /// [`NodeAttrs::style`] field.
    ///
    /// `Style` may attach to block nodes and inline `Span` nodes — it is not
    /// block-only.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::style::Style;
    /// use renderable::tree::NodeAttrs;
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_style(&Style::default());
    /// assert_eq!(attrs.style(), Some(Style::default()));
    /// ```
    pub fn set_style(&mut self, style: &crate::style::Style) {
        self.style = Some(Box::new(style.clone()));
    }

    /// Reads the [`Style`](crate::style::Style) stored on this node, if any.
    ///
    /// Returns `None` when no style is set.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// // An empty attribute set carries no style.
    /// assert!(NodeAttrs::default().style().is_none());
    /// ```
    #[must_use]
    pub fn style(&self) -> Option<crate::style::Style> {
        self.style.as_deref().cloned()
    }

    /// Borrowed accessor for renderer hot paths (no clone).
    ///
    /// Returns `None` when no style is set.
    #[must_use]
    pub fn style_ref(&self) -> Option<&crate::style::Style> {
        self.style.as_deref()
    }

    /// Stores a [`SequenceJoin`] policy as the typed
    /// [`NodeAttrs::sequence_join`] field.
    ///
    /// A node carrying this hint is rendered as an ordered sequence with no
    /// renderer-inserted block separators.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{NodeAttrs, SequenceJoin};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_sequence_join(SequenceJoin::None);
    /// assert_eq!(attrs.sequence_join(), Some(SequenceJoin::None));
    /// ```
    pub fn set_sequence_join(&mut self, join: SequenceJoin) {
        self.sequence_join = Some(join);
    }

    /// Reads the [`SequenceJoin`] policy stored on this node, if any.
    ///
    /// Returns `None` when no sequence-join policy is set.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// assert!(NodeAttrs::default().sequence_join().is_none());
    /// ```
    #[must_use]
    pub fn sequence_join(&self) -> Option<SequenceJoin> {
        self.sequence_join
    }

    /// Stores a [`ListMarkerPolicy`] as the typed
    /// [`NodeAttrs::list_marker_policy`] field.
    ///
    /// [`ListMarkerPolicy::Default`] is the implicit policy and serializes to
    /// nothing so the hint footprint stays minimal.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{ListMarkerPolicy, NodeAttrs};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_list_marker_policy(ListMarkerPolicy::TreeConnectors);
    /// assert_eq!(attrs.list_marker_policy(), ListMarkerPolicy::TreeConnectors);
    /// ```
    pub fn set_list_marker_policy(&mut self, policy: ListMarkerPolicy) {
        self.list_marker_policy = policy;
    }

    /// Reads the [`ListMarkerPolicy`] stored on this node.
    ///
    /// Defaults to [`ListMarkerPolicy::Default`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{ListMarkerPolicy, NodeAttrs};
    ///
    /// assert_eq!(NodeAttrs::default().list_marker_policy(), ListMarkerPolicy::Default);
    /// ```
    #[must_use]
    pub fn list_marker_policy(&self) -> ListMarkerPolicy {
        self.list_marker_policy
    }

    /// Stores [`TaskHints`] as the node's [`ComponentHints::Task`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{NodeAttrs, TaskHints, TaskState};
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_task_hints(&TaskHints { state: TaskState::Completed });
    /// assert_eq!(attrs.task_hints().unwrap().state, TaskState::Completed);
    /// ```
    pub fn set_task_hints(&mut self, hints: &TaskHints) {
        self.component = Some(Box::new(ComponentHints::Task(*hints)));
    }

    /// Reads this node's [`TaskHints`], if it carries any.
    ///
    /// Returns `None` when the list item is not a projected task-list item.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// assert!(NodeAttrs::default().task_hints().is_none());
    /// ```
    #[must_use]
    pub fn task_hints(&self) -> Option<TaskHints> {
        match self.component.as_deref() {
            Some(ComponentHints::Task(h)) => Some(*h),
            _ => None,
        }
    }

    /// Stores a table title/caption in this node's [`ComponentHints::Table`].
    ///
    /// A whitespace-only title is stored as-is; renderers are responsible for
    /// ignoring an empty or whitespace-only title. Co-resident with the
    /// per-column and terminal table hints.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// let mut attrs = NodeAttrs::default();
    /// attrs.set_table_title("Quarterly Results");
    /// assert_eq!(attrs.table_title().as_deref(), Some("Quarterly Results"));
    /// ```
    pub fn set_table_title(&mut self, title: impl Into<String>) {
        self.table_hints_mut().title = Some(title.into());
    }

    /// Reads the table title/caption stored on this node, if any.
    ///
    /// Returns `None` when no title is set.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::NodeAttrs;
    ///
    /// assert!(NodeAttrs::default().table_title().is_none());
    /// ```
    #[must_use]
    pub fn table_title(&self) -> Option<String> {
        match self.component.as_deref() {
            Some(ComponentHints::Table(t)) => t.title.clone(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn typed_layout_does_not_touch_data() {
        let mut attrs = NodeAttrs::default();
        attrs.set_layout(&crate::layout::Layout::default());
        assert!(attrs.layout.is_some()); // typed field set
        assert!(attrs.data.is_empty()); // NOT in the bag
        assert_eq!(attrs.layout(), Some(crate::layout::Layout::default()));
    }

    #[test]
    fn table_column_hints_keep_per_index_addressing() {
        let mut attrs = NodeAttrs::default();
        let c0 = TableColumnHints {
            min_width: Some(3),
            ..Default::default()
        };
        let c2 = TableColumnHints {
            max_width: Some(9),
            ..Default::default()
        };
        attrs.set_table_column_hints(0, &c0);
        attrs.set_table_column_hints(2, &c2);
        assert_eq!(attrs.table_column_hints(0), c0);
        assert_eq!(attrs.table_column_hints(2), c2);
        assert_eq!(attrs.table_column_hints(1), TableColumnHints::default());
        assert!(attrs.data.is_empty());
    }

    #[test]
    fn default_node_serializes_without_new_fields() {
        let json = serde_json::to_string(&NodeAttrs::default()).unwrap();
        // sparse: no layout/style/component/sequenceJoin keys
        assert!(!json.contains("layout"));
        assert!(!json.contains("component"));
        assert!(!json.contains("sequenceJoin"));
    }

    #[test]
    fn extension_hint_still_uses_data() {
        let mut attrs = NodeAttrs::default();
        attrs.set_hint(HintNamespace("darkmatter.hr"), "kind", json!("solid"));
        assert!(attrs.data.contains_key("darkmatter.hr.kind"));
    }

    #[test]
    fn token_enums_serialize_as_compact_tokens() {
        // Token enums serialize to their `to_token()` string, not the variant name.
        assert_eq!(
            serde_json::to_value(SequenceJoin::None).unwrap(),
            serde_json::Value::String(SequenceJoin::None.to_token().to_string())
        );
        let back: SequenceJoin =
            serde_json::from_value(serde_json::to_value(SequenceJoin::None).unwrap()).unwrap();
        assert_eq!(back, SequenceJoin::None);

        assert_eq!(
            serde_json::to_value(ListMarkerPolicy::TreeConnectors).unwrap(),
            serde_json::Value::String(ListMarkerPolicy::TreeConnectors.to_token().to_string())
        );
        assert_eq!(
            serde_json::to_value(TaskState::Completed).unwrap(),
            serde_json::Value::String(TaskState::Completed.to_token().to_string())
        );
        assert_eq!(
            serde_json::to_value(ColumnConditional::WidthGreaterThan(80)).unwrap(),
            serde_json::Value::String(ColumnConditional::WidthGreaterThan(80).to_token())
        );
        let back: ColumnConditional =
            serde_json::from_str(&serde_json::to_string(&ColumnConditional::LessThanOrEqual(40)).unwrap())
                .unwrap();
        assert_eq!(back, ColumnConditional::LessThanOrEqual(40));
    }

    #[test]
    fn hint_structs_serde_roundtrip() {
        let list = ListRenderHints {
            bullet: Some("* ".into()),
            ..Default::default()
        };
        let back: ListRenderHints =
            serde_json::from_str(&serde_json::to_string(&list).unwrap()).unwrap();
        assert_eq!(list, back);

        let cols = TableColumnHints::default();
        let back: TableColumnHints =
            serde_json::from_str(&serde_json::to_string(&cols).unwrap()).unwrap();
        assert_eq!(cols, back);

        let columns = ColumnsHints {
            left_width: ColumnWidthKind::Fixed(30),
            ..Default::default()
        };
        let back: ColumnsHints =
            serde_json::from_str(&serde_json::to_string(&columns).unwrap()).unwrap();
        assert_eq!(columns, back);

        let task = TaskHints {
            state: TaskState::Blocked,
        };
        let back: TaskHints =
            serde_json::from_str(&serde_json::to_string(&task).unwrap()).unwrap();
        assert_eq!(task, back);
    }

    #[test]
    fn hint_round_trip_set_get_remove() {
        let mut attrs = NodeAttrs::default();

        // Set hint
        attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(2));
        assert_eq!(
            attrs.get_hint(HintNamespace::LAYOUT, "margin_top"),
            Some(&json!(2))
        );

        // Overwrite hint
        attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(5));
        assert_eq!(
            attrs.get_hint(HintNamespace::LAYOUT, "margin_top"),
            Some(&json!(5))
        );

        // Remove hint
        let removed = attrs.remove_hint(HintNamespace::LAYOUT, "margin_top");
        assert_eq!(removed, Some(json!(5)));
        assert_eq!(attrs.get_hint(HintNamespace::LAYOUT, "margin_top"), None);

        // Remove nonexistent hint
        let removed = attrs.remove_hint(HintNamespace::LAYOUT, "margin_top");
        assert_eq!(removed, None);
    }

    #[test]
    fn namespaced_keys_do_not_collide_with_adhoc_keys() {
        let mut attrs = NodeAttrs::default();

        // Set a namespaced hint (full key: "renderable.layout.margin")
        attrs.set_hint(HintNamespace::LAYOUT, "margin", json!(10));

        // Set an ad-hoc key that looks similar but uses a different prefix
        attrs
            .data
            .insert("layout.margin".to_string(), json!("adhoc_value"));

        // The namespaced hint is unaffected by the ad-hoc key
        assert_eq!(
            attrs.get_hint(HintNamespace::LAYOUT, "margin"),
            Some(&json!(10))
        );

        // The ad-hoc key exists separately
        assert_eq!(attrs.data.get("layout.margin"), Some(&json!("adhoc_value")));

        // Both keys coexist in data
        assert_eq!(attrs.data.get("renderable.layout.margin"), Some(&json!(10)));
    }

    #[test]
    fn different_namespaces_do_not_collide() {
        let mut attrs = NodeAttrs::default();

        attrs.set_hint(HintNamespace::LAYOUT, "width", json!(100));
        attrs.set_hint(HintNamespace::TABLE, "width", json!(200));
        attrs.set_hint(HintNamespace::WIDGET_COLUMNS, "width", json!(300));

        assert_eq!(
            attrs.get_hint(HintNamespace::LAYOUT, "width"),
            Some(&json!(100))
        );
        assert_eq!(
            attrs.get_hint(HintNamespace::TABLE, "width"),
            Some(&json!(200))
        );
        assert_eq!(
            attrs.get_hint(HintNamespace::WIDGET_COLUMNS, "width"),
            Some(&json!(300))
        );
    }

    #[test]
    fn json_serialization_preserves_namespaced_keys() {
        let mut attrs = NodeAttrs::default();
        attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(2));
        attrs.set_hint(HintNamespace::TABLE, "border", json!(true));

        let json_str = serde_json::to_string(&attrs).expect("serialize");
        let decoded: NodeAttrs = serde_json::from_str(&json_str).expect("deserialize");

        assert_eq!(
            decoded.get_hint(HintNamespace::LAYOUT, "margin_top"),
            Some(&json!(2))
        );
        assert_eq!(
            decoded.get_hint(HintNamespace::TABLE, "border"),
            Some(&json!(true))
        );
    }

    #[test]
    fn layout_roundtrips_through_node_attrs() {
        use crate::layout::{Alignment, Layout, Length, Edges};

        let layout = Layout {
            margin: Edges::x(Length::ch(2)),
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let mut attrs = NodeAttrs::default();
        assert!(attrs.layout().is_none());
        attrs.set_layout(&layout);
        assert_eq!(attrs.layout(), Some(layout));
    }

    #[test]
    fn style_roundtrips_through_node_attrs() {
        use crate::color::{Color, Tailwind};
        use crate::layout::TargetValue;
        use crate::style::{PerMode, Style, TextEmphasis};

        let style = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
                Tailwind::Blue500,
            )))),
            emphasis: TextEmphasis {
                bold: true,
                ..Default::default()
            },
            ..Style::default()
        };
        let mut attrs = NodeAttrs::default();
        assert!(attrs.style().is_none());
        attrs.set_style(&style);
        assert_eq!(attrs.style(), Some(style));
    }

    #[test]
    fn style_namespace_is_distinct_from_layout() {
        let mut attrs = NodeAttrs::default();
        attrs.set_hint(HintNamespace::STYLE, "style", json!("a"));
        attrs.set_hint(HintNamespace::LAYOUT, "layout", json!("b"));
        assert_eq!(attrs.data.get("renderable.style.style"), Some(&json!("a")));
        assert_eq!(
            attrs.data.get("renderable.layout.layout"),
            Some(&json!("b"))
        );
    }

    #[test]
    fn code_hints_round_trip_set_and_read() {
        let mut attrs = NodeAttrs::default();
        attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            language_label: Some("yaml".into()),
            highlight: true,
        });

        let hints = attrs.code_hints();
        assert!(hints.header_row);
        assert_eq!(hints.language_label.as_deref(), Some("yaml"));
        assert!(hints.highlight);
    }

    #[test]
    fn code_hints_default_fields_are_left_unset() {
        let mut attrs = NodeAttrs::default();
        attrs.set_code_hints(&CodeRenderHints::default());

        // No keys are written for an all-default hint set.
        assert!(attrs.get_hint(HintNamespace::CODE, "header_row").is_none());
        assert!(
            attrs
                .get_hint(HintNamespace::CODE, "language_label")
                .is_none()
        );
        assert!(attrs.get_hint(HintNamespace::CODE, "highlight").is_none());

        let hints = attrs.code_hints();
        assert_eq!(hints, CodeRenderHints::default());
    }

    #[test]
    fn progress_hints_default_matches_progress_component() {
        let hints = ProgressHints::default();
        assert_eq!(hints.value, 0.0);
        assert_eq!(hints.bar_width, 20);
        assert_eq!(hints.fill_char, '█');
        assert_eq!(hints.empty_char, '·');
        assert_eq!(hints.left_bracket, '[');
        assert_eq!(hints.right_bracket, ']');
    }

    #[test]
    fn progress_hints_round_trip_set_and_read() {
        let mut attrs = NodeAttrs::default();
        attrs.set_progress_hints(&ProgressHints {
            value: 0.42,
            bar_width: 10,
            fill_char: '#',
            empty_char: '-',
            left_bracket: '(',
            right_bracket: ')',
            filled_color: Some(crate::color::Color::BasicColor(
                crate::color::BasicColor::Green,
            )),
            empty_color: None,
            bracket_color: Some(crate::color::Color::BasicColor(
                crate::color::BasicColor::Cyan,
            )),
        });

        let hints = attrs.progress_hints().expect("progress hints present");
        assert!((hints.value - 0.42).abs() < 1e-6);
        assert_eq!(hints.bar_width, 10);
        assert_eq!(hints.fill_char, '#');
        assert_eq!(hints.empty_char, '-');
        assert_eq!(hints.left_bracket, '(');
        assert_eq!(hints.right_bracket, ')');
        assert_eq!(
            hints.filled_color,
            Some(crate::color::Color::BasicColor(
                crate::color::BasicColor::Green
            ))
        );
        assert_eq!(hints.empty_color, None);
        assert_eq!(
            hints.bracket_color,
            Some(crate::color::Color::BasicColor(
                crate::color::BasicColor::Cyan
            ))
        );
    }

    #[test]
    fn progress_hints_absent_returns_none() {
        let attrs = NodeAttrs::default();
        assert!(attrs.progress_hints().is_none());
    }

    #[test]
    fn columns_hints_default_matches_two_column_component() {
        let hints = ColumnsHints::default();
        assert_eq!(hints.gap, 3);
        assert_eq!(hints.left_width, ColumnWidthKind::Percent(0.5));
        assert_eq!(hints.left_count, 0);
        assert!(hints.stack_below);
    }

    #[test]
    fn columns_hints_round_trip_percent() {
        let mut attrs = NodeAttrs::default();
        attrs.set_columns_hints(&ColumnsHints {
            gap: 5,
            left_width: ColumnWidthKind::Percent(0.7),
            left_count: 3,
            stack_below: false,
        });

        let hints = attrs.columns_hints().expect("columns hints present");
        assert_eq!(hints.gap, 5);
        assert_eq!(hints.left_width, ColumnWidthKind::Percent(0.7));
        assert_eq!(hints.left_count, 3);
        assert!(!hints.stack_below);
    }

    #[test]
    fn columns_hints_round_trip_fixed() {
        let mut attrs = NodeAttrs::default();
        attrs.set_columns_hints(&ColumnsHints {
            gap: 2,
            left_width: ColumnWidthKind::Fixed(40),
            left_count: 1,
            stack_below: true,
        });

        let hints = attrs.columns_hints().expect("columns hints present");
        assert_eq!(hints.gap, 2);
        assert_eq!(hints.left_width, ColumnWidthKind::Fixed(40));
        assert_eq!(hints.left_count, 1);
        assert!(hints.stack_below);
    }

    #[test]
    fn columns_hints_absent_returns_none() {
        let attrs = NodeAttrs::default();
        assert!(attrs.columns_hints().is_none());
    }

    #[test]
    fn column_conditional_token_round_trip() {
        for c in [
            ColumnConditional::Always,
            ColumnConditional::WidthGreaterThan(80),
            ColumnConditional::LessThanOrEqual(40),
        ] {
            assert_eq!(ColumnConditional::from_token(&c.to_token()), Some(c));
        }
        assert_eq!(ColumnConditional::from_token("bogus"), None);
        assert_eq!(ColumnConditional::from_token("gt:notanumber"), None);
    }

    #[test]
    fn table_column_hints_round_trip() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_column_hints(
            1,
            &TableColumnHints {
                min_width: Some(4),
                max_width: Some(40),
                fixed_width: Some(12),
                conditional: ColumnConditional::WidthGreaterThan(80),
                droppable: true,
                drop_note: Some("notes hidden".into()),
                uniform_alignment: true,
            },
        );

        let hints = attrs.table_column_hints(1);
        assert_eq!(hints.min_width, Some(4));
        assert_eq!(hints.max_width, Some(40));
        assert_eq!(hints.fixed_width, Some(12));
        assert_eq!(hints.conditional, ColumnConditional::WidthGreaterThan(80));
        assert!(hints.droppable);
        assert_eq!(hints.drop_note.as_deref(), Some("notes hidden"));
        assert!(hints.uniform_alignment);
    }

    /// A column may be droppable without carrying a drop note — the
    /// `droppable` flag must round-trip independently so the silent-drop
    /// case is preserved across the render tree.
    #[test]
    fn table_column_hints_droppable_without_note_round_trips() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_column_hints(
            0,
            &TableColumnHints {
                droppable: true,
                drop_note: None,
                ..Default::default()
            },
        );

        let hints = attrs.table_column_hints(0);
        assert!(
            hints.droppable,
            "silent-drop columns must round-trip as droppable",
        );
        assert!(hints.drop_note.is_none());
    }

    #[test]
    fn table_column_hints_default_fields_left_unset() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_column_hints(0, &TableColumnHints::default());
        assert!(attrs.data.is_empty());
        assert_eq!(attrs.table_column_hints(0), TableColumnHints::default());
    }

    #[test]
    fn table_column_hints_indexed_independently() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_column_hints(
            0,
            &TableColumnHints {
                min_width: Some(5),
                ..Default::default()
            },
        );
        attrs.set_table_column_hints(
            1,
            &TableColumnHints {
                min_width: Some(9),
                ..Default::default()
            },
        );
        assert_eq!(attrs.table_column_hints(0).min_width, Some(5));
        assert_eq!(attrs.table_column_hints(1).min_width, Some(9));
    }

    #[test]
    fn table_cell_hints_round_trip() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_cell_hints(&TableCellHints {
            kind: "currency".into(),
            raw_value: json!(1234.56),
            alignment: "right".into(),
            vertical_alignment: "middle".into(),
        });

        let hints = attrs.table_cell_hints().expect("cell hints present");
        assert_eq!(hints.kind, "currency");
        assert_eq!(hints.raw_value, json!(1234.56));
        assert_eq!(hints.alignment, "right");
        assert_eq!(hints.vertical_alignment, "middle");
    }

    #[test]
    fn table_cell_hints_absent_returns_none() {
        assert!(NodeAttrs::default().table_cell_hints().is_none());
    }

    #[test]
    fn table_terminal_hints_round_trip() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_terminal_hints(&TableTerminalHints {
            prefer_cursor_alignment: true,
            alternate_background: true,
            alternate_text_color: false,
            stripe_bg: Some(crate::color::Color::BasicColor(
                crate::color::BasicColor::Blue,
            )),
            stripe_text: None,
        });
        let hints = attrs.table_terminal_hints();
        assert!(hints.prefer_cursor_alignment);
        assert!(hints.alternate_background);
        assert!(!hints.alternate_text_color);
        assert_eq!(
            hints.stripe_bg,
            Some(crate::color::Color::BasicColor(
                crate::color::BasicColor::Blue
            ))
        );
        assert_eq!(hints.stripe_text, None);
    }

    #[test]
    fn table_terminal_hints_default_fields_left_unset() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_terminal_hints(&TableTerminalHints::default());
        assert!(attrs.data.is_empty());
        assert_eq!(attrs.table_terminal_hints(), TableTerminalHints::default());
    }

    #[test]
    fn sequence_join_round_trip() {
        let mut attrs = NodeAttrs::default();
        assert!(attrs.sequence_join().is_none());
        attrs.set_sequence_join(SequenceJoin::None);
        assert_eq!(attrs.sequence_join(), Some(SequenceJoin::None));
        assert_eq!(SequenceJoin::from_token("none"), Some(SequenceJoin::None));
        assert_eq!(SequenceJoin::from_token("bogus"), None);
    }

    #[test]
    fn list_marker_policy_round_trip() {
        let mut attrs = NodeAttrs::default();
        // The default policy is left unset so the hint footprint stays minimal.
        attrs.set_list_marker_policy(ListMarkerPolicy::Default);
        assert!(attrs.data.is_empty());
        assert_eq!(attrs.list_marker_policy(), ListMarkerPolicy::Default);

        attrs.set_list_marker_policy(ListMarkerPolicy::None);
        assert_eq!(attrs.list_marker_policy(), ListMarkerPolicy::None);

        attrs.set_list_marker_policy(ListMarkerPolicy::TreeConnectors);
        assert_eq!(attrs.list_marker_policy(), ListMarkerPolicy::TreeConnectors);

        // Setting back to Default clears the hint.
        attrs.set_list_marker_policy(ListMarkerPolicy::Default);
        assert!(attrs.data.is_empty());

        for p in [
            ListMarkerPolicy::Default,
            ListMarkerPolicy::None,
            ListMarkerPolicy::TreeConnectors,
        ] {
            assert_eq!(ListMarkerPolicy::from_token(p.to_token()), Some(p));
        }
        assert_eq!(ListMarkerPolicy::from_token("bogus"), None);
    }

    #[test]
    fn task_hints_round_trip_all_states() {
        for state in [
            TaskState::Open,
            TaskState::InProgress,
            TaskState::Completed,
            TaskState::Blocked,
            TaskState::Cancelled,
        ] {
            let mut attrs = NodeAttrs::default();
            assert!(attrs.task_hints().is_none());
            attrs.set_task_hints(&TaskHints { state });
            assert_eq!(attrs.task_hints().unwrap().state, state);
            assert_eq!(TaskState::from_token(state.to_token()), Some(state));
        }
        assert_eq!(TaskState::from_token("bogus"), None);
    }

    #[test]
    fn task_hints_stored_as_typed_component_not_data() {
        let mut attrs = NodeAttrs::default();
        attrs.set_task_hints(&TaskHints {
            state: TaskState::Blocked,
        });
        // The task state rides on the typed `component` field, never the bag.
        assert!(attrs.data.is_empty());
        assert!(matches!(
            attrs.component.as_deref(),
            Some(ComponentHints::Task(TaskHints {
                state: TaskState::Blocked
            }))
        ));
    }

    #[test]
    fn table_title_round_trip() {
        let mut attrs = NodeAttrs::default();
        assert!(attrs.table_title().is_none());
        attrs.set_table_title("Quarterly Results");
        assert_eq!(attrs.table_title().as_deref(), Some("Quarterly Results"));
        // The title rides on the typed `component` field, never the bag.
        assert!(attrs.data.is_empty());
    }

    /// The per-column, terminal, and title table hints are co-resident: each
    /// setter routes through `table_hints_mut()` so setting one does not
    /// clobber the others.
    #[test]
    fn table_hints_coexist_on_one_node() {
        let mut attrs = NodeAttrs::default();
        attrs.set_table_column_hints(
            0,
            &TableColumnHints {
                min_width: Some(7),
                ..Default::default()
            },
        );
        attrs.set_table_terminal_hints(&TableTerminalHints {
            alternate_background: true,
            ..Default::default()
        });
        attrs.set_table_title("Totals");

        assert_eq!(attrs.table_column_hints(0).min_width, Some(7));
        assert!(attrs.table_terminal_hints().alternate_background);
        assert_eq!(attrs.table_title().as_deref(), Some("Totals"));
    }

    #[test]
    fn custom_namespace_works() {
        // Verify external crates can define their own namespaces
        const CUSTOM_NS: HintNamespace = HintNamespace("myapp.custom");

        let mut attrs = NodeAttrs::default();
        attrs.set_hint(CUSTOM_NS, "setting", json!("value"));

        assert_eq!(attrs.get_hint(CUSTOM_NS, "setting"), Some(&json!("value")));
        assert_eq!(
            attrs.data.get("myapp.custom.setting"),
            Some(&json!("value"))
        );
    }

    /// The perf-gate counter must classify a `renderable.*` bag access as
    /// renderable-owned and any other namespace as an extension access, across
    /// `set_hint` / `get_hint` / `remove_hint`. This proves the gate below is
    /// not vacuous: the counter genuinely increments when the bag is touched.
    #[test]
    fn hint_access_counter_classifies_by_namespace() {
        let mut attrs = NodeAttrs::default();
        super::reset_hint_accesses();

        attrs.set_hint(HintNamespace::LAYOUT, "margin_top", json!(2)); // renderable-owned
        attrs.set_hint(HintNamespace("darkmatter.hr"), "kind", json!("solid")); // extension
        let _ = attrs.get_hint(HintNamespace("darkmatter.hr"), "kind"); // extension
        let _ = attrs.remove_hint(HintNamespace::LAYOUT, "margin_top"); // renderable-owned

        let (renderable_owned, extension) = super::hint_accesses();
        assert_eq!(renderable_owned, 2, "two `renderable.*` bag accesses");
        assert_eq!(extension, 2, "two extension-namespace bag accesses");
    }

    /// Builds a styled corpus exercising layout, inherited style, a list
    /// (marker policy + list hints), a table (column + title), a code block, a
    /// task item, and one `darkmatter.hr` extension hint. The first-class
    /// presentation rides on typed fields; only the extension hint lives in the
    /// bag, so a behaving fold touches `data` zero times for `renderable.*`.
    #[cfg(test)]
    fn styled_corpus_document() -> crate::tree::Document {
        use crate::color::{Color, Tailwind};
        use crate::layout::{Layout, TargetValue};
        use crate::style::{PerMode, Style, TextEmphasis};
        use crate::tree::{
            Document, DocumentMetadata, HeadingDepth, RenderNode, SourceRegistry,
        };

        let red = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
                Tailwind::Red500,
            )))),
            emphasis: TextEmphasis {
                bold: true,
                ..Default::default()
            },
            ..Style::default()
        };

        // Heading carrying layout + an inheritable style.
        let mut heading =
            RenderNode::heading(HeadingDepth::new(2).unwrap(), vec![RenderNode::text("Title")]);
        heading.attrs.set_layout(&Layout::default());
        heading.attrs.set_style(&red);

        // List with a marker policy and list hints; one task item carries a
        // `darkmatter.hr` extension hint to populate the bag.
        let mut task_item = RenderNode::list_item(Some(false), vec![RenderNode::paragraph(vec![
            RenderNode::text("todo"),
        ])]);
        task_item.attrs.set_task_hints(&TaskHints {
            state: TaskState::InProgress,
        });
        task_item
            .attrs
            .set_hint(HintNamespace("darkmatter.hr"), "kind", json!("solid"));
        let mut list = RenderNode::list(false, None, vec![task_item]);
        list.attrs.set_list_marker_policy(ListMarkerPolicy::None);
        list.attrs.set_list_hints(&ListRenderHints {
            bullet: Some("* ".into()),
            ..Default::default()
        });

        // Table with a column hint and a title.
        let cell = RenderNode::table_cell(vec![RenderNode::text("c0")]);
        let row = RenderNode::table_row(vec![cell]);
        let mut table = RenderNode::table(vec![crate::tree::ColumnAlign::Left], vec![row]);
        table.attrs.set_table_column_hints(
            0,
            &TableColumnHints {
                min_width: Some(4),
                ..Default::default()
            },
        );
        table.attrs.set_table_title("Totals");

        // Code block with hints.
        let mut code = RenderNode::code(Some("rust".into()), None, "let x = 1;");
        code.attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            language_label: Some("rust".into()),
            highlight: true,
        });

        Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![heading, list, table, code]),
        }
    }

    /// Structural perf gate: folding the styled corpus to Markdown must not
    /// round-trip any renderable-owned hint through [`NodeAttrs::data`]. Every
    /// first-class hint is a typed field, so the renderable-owned counter stays
    /// at zero. (Terminal and browser folds gate the same invariant in their
    /// own crates; this is the AC #5 minimum.)
    #[test]
    fn fold_does_zero_renderable_owned_hint_roundtrips() {
        let doc = styled_corpus_document();

        super::reset_hint_accesses();
        let _ = crate::tree::render::render_markdown_document(
            &doc,
            &crate::tree::render::MarkdownRenderOptions::default(),
        )
        .expect("markdown fold");
        let (renderable_owned, _extension) = super::hint_accesses();

        assert_eq!(
            renderable_owned, 0,
            "the fold must not round-trip any renderable-owned hint through `data`",
        );
    }
}
