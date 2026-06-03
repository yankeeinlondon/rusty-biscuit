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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq, Default)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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

/// Per-column render hints for a projected [`NodeKind::Table`] node.
///
/// A `Table` component records one set of these per column under the
/// [`HintNamespace::TABLE`] namespace, keyed `column.{i}.*`. Renderers that
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone, PartialEq, Default)]
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
/// These are stored under the [`HintNamespace::TERMINAL`] namespace and
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

/// Optional presentational attributes carried by every render node.
///
/// All fields are optional; the [`Default`] value is an empty set of
/// attributes (no id, no classes, no data).
///
/// ## Examples
///
/// ```
/// use renderable::tree::NodeAttrs;
///
/// let attrs = NodeAttrs::default();
/// assert!(attrs.id.is_none());
/// assert!(attrs.classes.is_empty());
/// assert!(attrs.data.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAttrs {
    /// Optional unique identifier for the node.
    pub id: Option<String>,
    /// CSS-style class names associated with the node.
    pub classes: Vec<String>,
    /// Arbitrary structured data keyed by name.
    pub data: BTreeMap<String, serde_json::Value>,
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
        let full_key = format!("{}.{}", ns.0, key);
        self.data.remove(&full_key)
    }

    /// Stores [`ListRenderHints`] under the [`HintNamespace::LIST`] namespace.
    ///
    /// Only non-default fields are written: a `None` bullet, a `true`
    /// `hanging_indent`, and a `None` `indent_children` are left unset so the
    /// hint footprint stays minimal.
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
        match &hints.bullet {
            Some(bullet) => self.set_hint(
                HintNamespace::LIST,
                "bullet",
                serde_json::Value::String(bullet.clone()),
            ),
            None => {
                self.remove_hint(HintNamespace::LIST, "bullet");
            }
        }

        if hints.hanging_indent {
            self.remove_hint(HintNamespace::LIST, "hanging_indent");
        } else {
            self.set_hint(
                HintNamespace::LIST,
                "hanging_indent",
                serde_json::Value::Bool(false),
            );
        }

        match hints.indent_children {
            Some(indent) => self.set_hint(
                HintNamespace::LIST,
                "indent_children",
                serde_json::Value::from(indent),
            ),
            None => {
                self.remove_hint(HintNamespace::LIST, "indent_children");
            }
        }
    }

    /// Reads [`ListRenderHints`] from the [`HintNamespace::LIST`] namespace.
    ///
    /// Missing hints fall back to [`ListRenderHints::default`].
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
        let bullet = self
            .get_hint(HintNamespace::LIST, "bullet")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);

        let hanging_indent = self
            .get_hint(HintNamespace::LIST, "hanging_indent")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let indent_children = self
            .get_hint(HintNamespace::LIST, "indent_children")
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u32::try_from(v).ok());

        ListRenderHints {
            bullet,
            hanging_indent,
            indent_children,
        }
    }

    /// Stores [`CodeRenderHints`] under the [`HintNamespace::CODE`] namespace.
    ///
    /// Only non-default fields are written: a `false` `header_row`, a `None`
    /// `language_label`, and a `false` `highlight` are left unset so the hint
    /// footprint stays minimal.
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
        if hints.header_row {
            self.set_hint(
                HintNamespace::CODE,
                "header_row",
                serde_json::Value::Bool(true),
            );
        } else {
            self.remove_hint(HintNamespace::CODE, "header_row");
        }

        match &hints.language_label {
            Some(label) => self.set_hint(
                HintNamespace::CODE,
                "language_label",
                serde_json::Value::String(label.clone()),
            ),
            None => {
                self.remove_hint(HintNamespace::CODE, "language_label");
            }
        }

        if hints.highlight {
            self.set_hint(
                HintNamespace::CODE,
                "highlight",
                serde_json::Value::Bool(true),
            );
        } else {
            self.remove_hint(HintNamespace::CODE, "highlight");
        }
    }

    /// Reads [`CodeRenderHints`] from the [`HintNamespace::CODE`] namespace.
    ///
    /// Missing hints fall back to [`CodeRenderHints::default`].
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
        let header_row = self
            .get_hint(HintNamespace::CODE, "header_row")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let language_label = self
            .get_hint(HintNamespace::CODE, "language_label")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);

        let highlight = self
            .get_hint(HintNamespace::CODE, "highlight")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        CodeRenderHints {
            header_row,
            language_label,
            highlight,
        }
    }

    /// Stores [`ProgressHints`] under the [`HintNamespace::WIDGET_PROGRESS`]
    /// namespace.
    ///
    /// All six fields are written so [`NodeAttrs::progress_hints`] can detect
    /// the presence of progress hints and reproduce the bar faithfully.
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
        self.set_hint(
            HintNamespace::WIDGET_PROGRESS,
            "value",
            serde_json::Value::from(hints.value),
        );
        self.set_hint(
            HintNamespace::WIDGET_PROGRESS,
            "bar_width",
            serde_json::Value::from(hints.bar_width),
        );
        self.set_hint(
            HintNamespace::WIDGET_PROGRESS,
            "fill_char",
            serde_json::Value::String(hints.fill_char.to_string()),
        );
        self.set_hint(
            HintNamespace::WIDGET_PROGRESS,
            "empty_char",
            serde_json::Value::String(hints.empty_char.to_string()),
        );
        self.set_hint(
            HintNamespace::WIDGET_PROGRESS,
            "left_bracket",
            serde_json::Value::String(hints.left_bracket.to_string()),
        );
        self.set_hint(
            HintNamespace::WIDGET_PROGRESS,
            "right_bracket",
            serde_json::Value::String(hints.right_bracket.to_string()),
        );
        // Slot colors are optional: stored only when set, so a glyph-only
        // progress bar carries no color keys.
        for (key, color) in [
            ("filled_color", hints.filled_color),
            ("empty_color", hints.empty_color),
            ("bracket_color", hints.bracket_color),
        ] {
            if let Some(color) = color
                && let Ok(value) = serde_json::to_value(color)
            {
                self.set_hint(HintNamespace::WIDGET_PROGRESS, key, value);
            }
        }
    }

    /// Reads [`ProgressHints`] from the [`HintNamespace::WIDGET_PROGRESS`]
    /// namespace.
    ///
    /// Returns `None` when no progress `value` hint is present, so renderers
    /// can detect whether a paragraph is a projected progress widget. Any
    /// individual missing field falls back to [`ProgressHints::default`].
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
        let value = self
            .get_hint(HintNamespace::WIDGET_PROGRESS, "value")
            .and_then(serde_json::Value::as_f64)? as f32;

        let defaults = ProgressHints::default();

        let bar_width = self
            .get_hint(HintNamespace::WIDGET_PROGRESS, "bar_width")
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(defaults.bar_width);

        let read_char = |key: &str, fallback: char| -> char {
            self.get_hint(HintNamespace::WIDGET_PROGRESS, key)
                .and_then(serde_json::Value::as_str)
                .and_then(|s| s.chars().next())
                .unwrap_or(fallback)
        };

        let read_color = |key: &str| -> Option<crate::color::Color> {
            self.get_hint(HintNamespace::WIDGET_PROGRESS, key)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        };

        Some(ProgressHints {
            value,
            bar_width,
            fill_char: read_char("fill_char", defaults.fill_char),
            empty_char: read_char("empty_char", defaults.empty_char),
            left_bracket: read_char("left_bracket", defaults.left_bracket),
            right_bracket: read_char("right_bracket", defaults.right_bracket),
            filled_color: read_color("filled_color"),
            empty_color: read_color("empty_color"),
            bracket_color: read_color("bracket_color"),
        })
    }

    /// Stores [`ColumnsHints`] under the [`HintNamespace::WIDGET_COLUMNS`]
    /// namespace.
    ///
    /// All fields are written so [`NodeAttrs::columns_hints`] can detect the
    /// presence of column hints and reconstruct the two-column layout.
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
        self.set_hint(
            HintNamespace::WIDGET_COLUMNS,
            "gap",
            serde_json::Value::from(hints.gap),
        );
        let (kind, value) = match hints.left_width {
            ColumnWidthKind::Fixed(chars) => ("fixed", serde_json::Value::from(chars)),
            ColumnWidthKind::Percent(percent) => ("percent", serde_json::Value::from(percent)),
        };
        self.set_hint(
            HintNamespace::WIDGET_COLUMNS,
            "left_width_kind",
            serde_json::Value::String(kind.to_string()),
        );
        self.set_hint(HintNamespace::WIDGET_COLUMNS, "left_width", value);
        self.set_hint(
            HintNamespace::WIDGET_COLUMNS,
            "left_count",
            serde_json::Value::from(hints.left_count as u64),
        );
        self.set_hint(
            HintNamespace::WIDGET_COLUMNS,
            "stack_below",
            serde_json::Value::Bool(hints.stack_below),
        );
    }

    /// Reads [`ColumnsHints`] from the [`HintNamespace::WIDGET_COLUMNS`]
    /// namespace.
    ///
    /// Returns `None` when no `gap` hint is present, so renderers can detect
    /// whether a block quote is a projected two-column widget. Any individual
    /// missing field falls back to [`ColumnsHints::default`].
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
        let gap = self
            .get_hint(HintNamespace::WIDGET_COLUMNS, "gap")
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())?;

        let defaults = ColumnsHints::default();

        let left_width = match self
            .get_hint(HintNamespace::WIDGET_COLUMNS, "left_width_kind")
            .and_then(serde_json::Value::as_str)
        {
            Some("fixed") => self
                .get_hint(HintNamespace::WIDGET_COLUMNS, "left_width")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok())
                .map(ColumnWidthKind::Fixed)
                .unwrap_or(defaults.left_width),
            Some("percent") => self
                .get_hint(HintNamespace::WIDGET_COLUMNS, "left_width")
                .and_then(serde_json::Value::as_f64)
                .map(|v| ColumnWidthKind::Percent(v as f32))
                .unwrap_or(defaults.left_width),
            _ => defaults.left_width,
        };

        let left_count = self
            .get_hint(HintNamespace::WIDGET_COLUMNS, "left_count")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(defaults.left_count);

        let stack_below = self
            .get_hint(HintNamespace::WIDGET_COLUMNS, "stack_below")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(defaults.stack_below);

        Some(ColumnsHints {
            gap,
            left_width,
            left_count,
            stack_below,
        })
    }

    /// Stores [`TableColumnHints`] for column `index` under the
    /// [`HintNamespace::TABLE`] namespace.
    ///
    /// Keys are prefixed `column.{index}.`. Only non-default fields are
    /// written so the hint footprint stays minimal.
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
        let key = |suffix: &str| format!("column.{index}.{suffix}");

        for (suffix, value) in [
            ("min_width", hints.min_width),
            ("max_width", hints.max_width),
            ("fixed_width", hints.fixed_width),
        ] {
            match value {
                Some(width) => self.set_hint(
                    HintNamespace::TABLE,
                    &key(suffix),
                    serde_json::Value::from(width),
                ),
                None => {
                    self.remove_hint(HintNamespace::TABLE, &key(suffix));
                }
            }
        }

        if hints.conditional == ColumnConditional::Always {
            self.remove_hint(HintNamespace::TABLE, &key("conditional"));
        } else {
            self.set_hint(
                HintNamespace::TABLE,
                &key("conditional"),
                serde_json::Value::String(hints.conditional.to_token()),
            );
        }

        match &hints.drop_note {
            Some(note) => self.set_hint(
                HintNamespace::TABLE,
                &key("drop_note"),
                serde_json::Value::String(note.clone()),
            ),
            None => {
                self.remove_hint(HintNamespace::TABLE, &key("drop_note"));
            }
        }

        // `droppable` is the authoritative droppability signal — a column
        // may be droppable without a `drop_note` (silent drop), so it must
        // round-trip independently. Only emit the hint when true to keep
        // default attribute sets empty.
        if hints.droppable {
            self.set_hint(
                HintNamespace::TABLE,
                &key("droppable"),
                serde_json::Value::Bool(true),
            );
        } else {
            self.remove_hint(HintNamespace::TABLE, &key("droppable"));
        }

        if hints.uniform_alignment {
            self.set_hint(
                HintNamespace::TABLE,
                &key("uniform_alignment"),
                serde_json::Value::Bool(true),
            );
        } else {
            self.remove_hint(HintNamespace::TABLE, &key("uniform_alignment"));
        }
    }

    /// Reads [`TableColumnHints`] for column `index` from the
    /// [`HintNamespace::TABLE`] namespace.
    ///
    /// Missing hints fall back to [`TableColumnHints::default`]. Malformed
    /// conditional tokens fall back to [`ColumnConditional::Always`].
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
        let key = |suffix: &str| format!("column.{index}.{suffix}");

        let read_width = |suffix: &str| -> Option<u32> {
            self.get_hint(HintNamespace::TABLE, &key(suffix))
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok())
        };

        let conditional = self
            .get_hint(HintNamespace::TABLE, &key("conditional"))
            .and_then(serde_json::Value::as_str)
            .and_then(ColumnConditional::from_token)
            .unwrap_or_default();

        let drop_note = self
            .get_hint(HintNamespace::TABLE, &key("drop_note"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);

        // A column with a non-empty `drop_note` is implicitly droppable —
        // honor that as a fallback so legacy attribute sets written before
        // the explicit `droppable` hint existed still round-trip correctly.
        let droppable = self
            .get_hint(HintNamespace::TABLE, &key("droppable"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
            || drop_note.is_some();

        let uniform_alignment = self
            .get_hint(HintNamespace::TABLE, &key("uniform_alignment"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        TableColumnHints {
            min_width: read_width("min_width"),
            max_width: read_width("max_width"),
            fixed_width: read_width("fixed_width"),
            conditional,
            droppable,
            drop_note,
            uniform_alignment,
        }
    }

    /// Stores [`TableCellHints`] under the [`HintNamespace::TABLE`] namespace.
    ///
    /// All four fields are written so [`NodeAttrs::table_cell_hints`] can
    /// detect the presence of cell hints.
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
        self.set_hint(
            HintNamespace::TABLE,
            "cell.kind",
            serde_json::Value::String(hints.kind.clone()),
        );
        self.set_hint(
            HintNamespace::TABLE,
            "cell.raw_value",
            hints.raw_value.clone(),
        );
        self.set_hint(
            HintNamespace::TABLE,
            "cell.alignment",
            serde_json::Value::String(hints.alignment.clone()),
        );
        self.set_hint(
            HintNamespace::TABLE,
            "cell.vertical_alignment",
            serde_json::Value::String(hints.vertical_alignment.clone()),
        );
    }

    /// Reads [`TableCellHints`] from the [`HintNamespace::TABLE`] namespace.
    ///
    /// Returns `None` when no `cell.kind` hint is present, so renderers can
    /// detect whether a table cell carries typed-cell hints.
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
        let kind = self
            .get_hint(HintNamespace::TABLE, "cell.kind")
            .and_then(serde_json::Value::as_str)?
            .to_string();

        let raw_value = self
            .get_hint(HintNamespace::TABLE, "cell.raw_value")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let read_str = |key: &str, fallback: &str| -> String {
            self.get_hint(HintNamespace::TABLE, key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or(fallback)
                .to_string()
        };

        Some(TableCellHints {
            kind,
            raw_value,
            alignment: read_str("cell.alignment", "left"),
            vertical_alignment: read_str("cell.vertical_alignment", "top"),
        })
    }

    /// Stores [`TableTerminalHints`] under the [`HintNamespace::TERMINAL`]
    /// namespace.
    ///
    /// Only `true` flags are written so the hint footprint stays minimal.
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
        for (key, flag) in [
            ("prefer_cursor_alignment", hints.prefer_cursor_alignment),
            ("alternate_background", hints.alternate_background),
            ("alternate_text_color", hints.alternate_text_color),
        ] {
            if flag {
                self.set_hint(HintNamespace::TERMINAL, key, serde_json::Value::Bool(true));
            } else {
                self.remove_hint(HintNamespace::TERMINAL, key);
            }
        }
        // Explicit stripe colors are optional: stored only when set.
        for (key, color) in [
            ("stripe_bg", hints.stripe_bg),
            ("stripe_text", hints.stripe_text),
        ] {
            match color.and_then(|c| serde_json::to_value(c).ok()) {
                Some(value) => self.set_hint(HintNamespace::TERMINAL, key, value),
                None => {
                    self.remove_hint(HintNamespace::TERMINAL, key);
                }
            }
        }
    }

    /// Reads [`TableTerminalHints`] from the [`HintNamespace::TERMINAL`]
    /// namespace.
    ///
    /// Missing hints fall back to [`TableTerminalHints::default`].
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
        let read_flag = |key: &str| -> bool {
            self.get_hint(HintNamespace::TERMINAL, key)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        };
        let read_color = |key: &str| -> Option<crate::color::Color> {
            self.get_hint(HintNamespace::TERMINAL, key)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        };

        TableTerminalHints {
            prefer_cursor_alignment: read_flag("prefer_cursor_alignment"),
            alternate_background: read_flag("alternate_background"),
            alternate_text_color: read_flag("alternate_text_color"),
            stripe_bg: read_color("stripe_bg"),
            stripe_text: read_color("stripe_text"),
        }
    }

    /// Stores a [`Layout`](crate::layout::Layout) under the
    /// [`HintNamespace::LAYOUT`] namespace.
    ///
    /// The layout is serialized to JSON and recovered by
    /// [`NodeAttrs::layout`].
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
        if let Ok(value) = serde_json::to_value(layout) {
            self.set_hint(HintNamespace::LAYOUT, "layout", value);
        }
    }

    /// Reads the [`Layout`](crate::layout::Layout) stored on this node, if any.
    ///
    /// Returns `None` when no layout hint is present or the stored value
    /// fails to deserialize.
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
        let value = self.get_hint(HintNamespace::LAYOUT, "layout")?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Stores a [`Style`](crate::style::Style) under the
    /// [`HintNamespace::STYLE`] namespace.
    ///
    /// The style is serialized to JSON and recovered by
    /// [`NodeAttrs::style`]. `Style` may attach to block nodes and inline
    /// `Span` nodes — it is not block-only.
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
        if let Ok(value) = serde_json::to_value(style) {
            self.set_hint(HintNamespace::STYLE, "style", value);
        }
    }

    /// Reads the [`Style`](crate::style::Style) stored on this node, if any.
    ///
    /// Returns `None` when no style hint is present or the stored value
    /// fails to deserialize.
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
        let value = self.get_hint(HintNamespace::STYLE, "style")?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Stores a [`SequenceJoin`] policy under the [`HintNamespace::LAYOUT`]
    /// namespace.
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
        self.set_hint(
            HintNamespace::LAYOUT,
            "sequence_join",
            serde_json::Value::String(join.to_token().to_string()),
        );
    }

    /// Reads the [`SequenceJoin`] policy stored on this node, if any.
    ///
    /// Returns `None` when no sequence-join hint is present or the stored
    /// token is unrecognized.
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
        self.get_hint(HintNamespace::LAYOUT, "sequence_join")
            .and_then(serde_json::Value::as_str)
            .and_then(SequenceJoin::from_token)
    }

    /// Stores a [`ListMarkerPolicy`] under the [`HintNamespace::LIST`]
    /// namespace.
    ///
    /// [`ListMarkerPolicy::Default`] is the implicit policy and is left
    /// unset so the hint footprint stays minimal.
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
        if policy == ListMarkerPolicy::Default {
            self.remove_hint(HintNamespace::LIST, "marker_policy");
        } else {
            self.set_hint(
                HintNamespace::LIST,
                "marker_policy",
                serde_json::Value::String(policy.to_token().to_string()),
            );
        }
    }

    /// Reads the [`ListMarkerPolicy`] stored on this node.
    ///
    /// Missing or unrecognized hints fall back to
    /// [`ListMarkerPolicy::Default`].
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
        self.get_hint(HintNamespace::LIST, "marker_policy")
            .and_then(serde_json::Value::as_str)
            .and_then(ListMarkerPolicy::from_token)
            .unwrap_or_default()
    }

    /// Stores [`TaskHints`] under the [`HintNamespace::WIDGET_TASK`] namespace.
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
        self.set_hint(
            HintNamespace::WIDGET_TASK,
            "state",
            serde_json::Value::String(hints.state.to_token().to_string()),
        );
    }

    /// Reads [`TaskHints`] from the [`HintNamespace::WIDGET_TASK`] namespace.
    ///
    /// Returns `None` when no `state` hint is present, so renderers can detect
    /// whether a list item is a projected task-list item.
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
        let state = self
            .get_hint(HintNamespace::WIDGET_TASK, "state")
            .and_then(serde_json::Value::as_str)
            .and_then(TaskState::from_token)?;
        Some(TaskHints { state })
    }

    /// Stores a table title/caption under the [`HintNamespace::TABLE`]
    /// namespace.
    ///
    /// A whitespace-only title is stored as-is; renderers are responsible for
    /// ignoring an empty or whitespace-only title.
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
        self.set_hint(
            HintNamespace::TABLE,
            "title",
            serde_json::Value::String(title.into()),
        );
    }

    /// Reads the table title/caption stored on this node, if any.
    ///
    /// Returns `None` when no title hint is present.
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
        self.get_hint(HintNamespace::TABLE, "title")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        use crate::layout::{Alignment, Layout, Length, Margin};

        let layout = Layout {
            margin: Margin::x(Length::ch(2)),
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
    fn progress_hints_missing_fields_fall_back_to_defaults() {
        // Only `value` is present; the rest fall back to defaults.
        let mut attrs = NodeAttrs::default();
        attrs.set_hint(HintNamespace::WIDGET_PROGRESS, "value", json!(0.25));

        let hints = attrs.progress_hints().expect("value present");
        assert!((hints.value - 0.25).abs() < 1e-6);
        assert_eq!(
            hints,
            ProgressHints {
                value: hints.value,
                ..Default::default()
            }
        );
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

    /// Legacy attribute sets written before the explicit `droppable` hint
    /// existed only carried `drop_note`. Reading those must still report
    /// the column as droppable so older serialized trees keep behaving.
    #[test]
    fn table_column_hints_legacy_drop_note_implies_droppable() {
        let mut attrs = NodeAttrs::default();
        attrs.set_hint(
            HintNamespace::TABLE,
            "column.0.drop_note",
            serde_json::Value::String("legacy note".into()),
        );

        let hints = attrs.table_column_hints(0);
        assert!(
            hints.droppable,
            "drop_note presence must imply droppable for backwards compat",
        );
        assert_eq!(hints.drop_note.as_deref(), Some("legacy note"));
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
    fn table_cell_hints_missing_fields_fall_back() {
        // Only `cell.kind` is set; the rest fall back to defaults.
        let mut attrs = NodeAttrs::default();
        attrs.set_hint(HintNamespace::TABLE, "cell.kind", json!("text"));
        let hints = attrs.table_cell_hints().expect("kind present");
        assert_eq!(hints.kind, "text");
        assert_eq!(hints.alignment, "left");
        assert_eq!(hints.vertical_alignment, "top");
        assert_eq!(hints.raw_value, serde_json::Value::Null);
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
    fn task_hints_stored_under_widget_task_namespace() {
        let mut attrs = NodeAttrs::default();
        attrs.set_task_hints(&TaskHints {
            state: TaskState::Blocked,
        });
        assert_eq!(
            attrs.data.get("renderable.widget.task.state"),
            Some(&json!("blocked"))
        );
    }

    #[test]
    fn table_title_round_trip() {
        let mut attrs = NodeAttrs::default();
        assert!(attrs.table_title().is_none());
        attrs.set_table_title("Quarterly Results");
        assert_eq!(attrs.table_title().as_deref(), Some("Quarterly Results"));
        // The title is stored under the table hint namespace.
        assert_eq!(
            attrs.data.get("renderable.table.title"),
            Some(&json!("Quarterly Results"))
        );
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
}
