use std::any::Any;
use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::style::{Style as RStyle, TextEmphasis};
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{
    ListRenderHints, RenderNode, RenderStrictness, TaskHints, TaskState, TreeRenderable,
};
use serde::{Deserialize, Serialize};

use crate::components::prose::Prose;
use crate::components::renderable::{BrowserRenderable, TerminalRenderable};
use crate::discovery::detection::ColorDepth;
use crate::render_tree::{TerminalRenderOptions, render_terminal_node};
use crate::terminal::Terminal;
use crate::utils::styling::{FontWeight, Style, Stylist};
use crate::utils::{
    color::{BasicColor, TermColor},
    layout::{Layout, LayoutTerminalExt},
};

/// box outline shape for an _open_ **TODO**
const NERD_CHECKBOX_OPEN: &str = "\u{f0131}";

const NERD_CHECKBOX_COMPLETED: &str = "\u{f4a7}";
/// a badged checkbox nerd icon representing a _blocked_ **TODO**
const NERD_CHECKBOX_BLOCKED: &str = "\u{f0117}";
/// intermediate nerd icon representing a _in progress_ **TODO**
const NERD_CHECKBOX_IN_PROGRESS: &str = "\u{f0856}";
/// an box off nerd icon representing a _cancelled_ **TODO**
const NERD_CHECKBOX_CANCELLED: &str = "\u{f12ed}";

// const NERD_CHECKBOX_FILLED: &'static str = "\u{f012e}";

/// fallback representation for an _open_ **TODO**
pub static FB_CHECKBOX_OPEN: &str = "[ ]";
/// In-progress fallback with color
pub static FB_CHECKBOX_IN_PROGRESS: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::Green.fg("⏺")));
/// Completed fallback with color
pub static FB_CHECKBOX_COMPLETED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::Green.fg("✔")));
/// Cancelled fallback with color
pub static FB_CHECKBOX_CANCELLED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::BrightRed.fg("-")));
/// Blocked fallback with color
pub static FB_CHECKBOX_BLOCKED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::BrightRed.fg("⏺")));

/// No-color fallback representations for terminals without color support
pub static FB_CHECKBOX_IN_PROGRESS_NOCOLOR: &str = "[>]";
pub static FB_CHECKBOX_COMPLETED_NOCOLOR: &str = "[x]";
pub static FB_CHECKBOX_CANCELLED_NOCOLOR: &str = "[-]";
pub static FB_CHECKBOX_BLOCKED_NOCOLOR: &str = "[!]";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum TodoState {
    Open,
    InProgress,
    Completed,
    Cancelled,
    Blocked,
}

impl From<TodoState> for TaskState {
    fn from(state: TodoState) -> Self {
        match state {
            TodoState::Open => TaskState::Open,
            TodoState::InProgress => TaskState::InProgress,
            TodoState::Completed => TaskState::Completed,
            TodoState::Cancelled => TaskState::Cancelled,
            TodoState::Blocked => TaskState::Blocked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TodoStateRep {
    pub nerd: &'static str,
    pub fallback: &'static str,
}

/// **TODO_CHAR_LOOKUP** provides a way to lookup what string representation to use for each
/// _state_ of a TODO. This lookup provides a representation for both a nerd font and the
/// fallback representation.
pub static TODO_CHAR_LOOKUP: LazyLock<HashMap<TodoState, TodoStateRep>> = LazyLock::new(|| {
    let mut m = HashMap::with_capacity(5);

    m.insert(
        TodoState::Open,
        TodoStateRep {
            nerd: NERD_CHECKBOX_OPEN,
            fallback: FB_CHECKBOX_OPEN,
        },
    );
    m.insert(
        TodoState::InProgress,
        TodoStateRep {
            nerd: NERD_CHECKBOX_IN_PROGRESS,
            fallback: &FB_CHECKBOX_IN_PROGRESS,
        },
    );
    m.insert(
        TodoState::Completed,
        TodoStateRep {
            nerd: NERD_CHECKBOX_COMPLETED,
            fallback: &FB_CHECKBOX_COMPLETED,
        },
    );
    m.insert(
        TodoState::Cancelled,
        TodoStateRep {
            nerd: NERD_CHECKBOX_CANCELLED,
            fallback: &FB_CHECKBOX_CANCELLED,
        },
    );
    m.insert(
        TodoState::Blocked,
        TodoStateRep {
            nerd: NERD_CHECKBOX_BLOCKED,
            fallback: &FB_CHECKBOX_BLOCKED,
        },
    );

    m
});

/// A TODO item with state tracking for terminal rendering.
///
/// Todo represents a task or item that can be in one of several states:
/// - **Open**: Not yet started, rendered with `[ ]` or ⬜
/// - **InProgress**: Currently being worked on, rendered with `[▶]` or ⏺
/// - **Completed**: Finished task, rendered with `[✔]` or ✓
/// - **Cancelled**: Abandoned task, rendered with `[-]` (strikethrough)
/// - **Blocked**: Cannot proceed, rendered with `[!]` or ⏺
///
/// The rendering automatically adapts to the terminal:
/// - Nerd Font terminals use icon glyphs (e.g., ⬜, ⏺, ✓)
/// - Regular terminals use ASCII fallbacks (e.g., `[ ]`, `[x]`, `[-]`)
/// - Colorless terminals strip all styling
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::todo::Todo;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// // Create a new open TODO
/// let todo = Todo::new("Review pull request #42");
///
/// // Render to string
/// let output = todo.render_optimistic(Some(80));
/// assert!(output.contains("Review pull request #42"));
/// ```
///
/// ## State Rendering
///
/// | State        | Nerd Font  | Fallback | Color |
/// |--------------|------------|----------|-------|
/// | Open         | ⬜         | `[ ]`    | none  |
/// | InProgress   | ⏺         | `[▶]`    | green |
/// | Completed    | ✓          | `[✔]`    | green |
/// | Cancelled    | ⃠          | `[-]`    | dim   |
/// | Blocked      | ⏺         | `[!]`    | red   |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    state: TodoState,
    description: String,
    created: DateTime<Utc>,
    last_updated: DateTime<Utc>,
    #[serde(skip)]
    use_prose: bool,
    #[serde(skip)]
    layout: Layout,
}

impl PartialEq for Todo {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.description == other.description
            && self.created == other.created
            && self.last_updated == other.last_updated
    }
}

impl Eq for Todo {}

impl std::hash::Hash for Todo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.hash(state);
        self.description.hash(state);
        self.created.hash(state);
        self.last_updated.hash(state);
    }
}

impl Default for Todo {
    fn default() -> Self {
        Self {
            state: TodoState::Open,
            description: "".to_string(),
            created: Utc::now(),
            last_updated: Utc::now(),
            use_prose: false,
            layout: Layout::default(),
        }
    }
}

impl From<&Todo> for Todo {
    /// allows the `state` and `description` of a Todo reference to
    /// be cloned into a new Todo which has `created` and `last_updated`
    /// set to now.
    fn from(value: &Todo) -> Self {
        Todo {
            state: value.state.clone(),
            description: value.description.clone(),
            ..Todo::default()
        }
    }
}

impl Todo {
    /// Create a new TODO item with just a description, the TODO's state
    /// will start as being "open".
    pub fn new<T: Into<String>>(desc: T) -> Todo {
        Todo {
            description: desc.into(),
            ..Todo::default()
        }
    }

    /// Create a new TODO item with a prose-formatted description.
    ///
    /// The description is rendered through [`Prose`] at render time,
    /// enabling markup like `<b>bold</b>` and `<red>color</red>`.
    pub fn from_prose<T: Into<String>>(desc: T) -> Todo {
        Todo {
            description: desc.into(),
            use_prose: true,
            ..Todo::default()
        }
    }

    /// Set the state of this Todo, returning `self` for builder-style chaining.
    pub fn with_state(mut self, state: TodoState) -> Self {
        self.state = state;
        self
    }

    /// Set the state in place via a mutable reference.
    pub fn set_state(&mut self, state: TodoState) -> &mut Self {
        self.state = state;
        self
    }

    /// Returns the Todo's current state.
    pub fn state(&self) -> &TodoState {
        &self.state
    }

    /// Returns the Todo's description text.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns whether this Todo's description should be rendered through
    /// [`Prose`].
    pub fn uses_prose(&self) -> bool {
        self.use_prose
    }

    /// Returns the state class name used as a stable CSS hook (`todo-open`,
    /// `todo-in-progress`, `todo-completed`, `todo-blocked`, `todo-cancelled`).
    fn state_class(&self) -> &'static str {
        match self.state {
            TodoState::Open => "todo-open",
            TodoState::InProgress => "todo-in-progress",
            TodoState::Completed => "todo-completed",
            TodoState::Blocked => "todo-blocked",
            TodoState::Cancelled => "todo-cancelled",
        }
    }

    /// Returns the cancellable strikethrough/dim style for `Cancelled` items.
    ///
    /// All other states return `None` — checkbox glyph styling is handled by
    /// the terminal tree renderer via the typed [`TaskHints`] payload, not the
    /// generic `Style` system (the color is widget-specific to the glyph and
    /// does not map cleanly to `Style.color`).
    fn state_style(&self) -> Option<RStyle> {
        match self.state {
            TodoState::Cancelled => Some(RStyle {
                emphasis: TextEmphasis {
                    dim: true,
                    strikethrough: true,
                    ..TextEmphasis::default()
                },
                ..RStyle::default()
            }),
            _ => None,
        }
    }

    /// Extracts plain text from a description that may carry [`Prose`] markup.
    ///
    /// When `use_prose` is set, the description is rendered through `Prose`
    /// against an optimistic terminal and ANSI-stripped to recover plain text.
    /// This lossy projection matches the BlockQuote precedent — inline emphasis
    /// is dropped at the tree boundary; only the textual content survives.
    fn description_text(&self) -> String {
        if self.use_prose {
            let plain = Prose::new(&self.description).render_optimistic(None);
            crate::utils::escape_codes::strip_escape_codes(&plain)
        } else {
            self.description.clone()
        }
    }

    /// Single private projection helper.
    ///
    /// Returns a `NodeKind::List` containing one `NodeKind::ListItem` with
    /// the typed [`TaskHints`] payload that drives state-aware terminal
    /// rendering. The `checked` field on the `ListItem` is `Some(true)` only
    /// for `Completed`; every other state uses `Some(false)` so Markdown and
    /// Browser output remain valid GFM task-list items.
    ///
    /// Both [`TreeRenderable::render_tree`] and the legacy
    /// [`TerminalRenderable::render_tree_node`] hook delegate to this so
    /// every render target sees the same projection.
    fn to_render_node(&self) -> RenderNode {
        let checked = Some(matches!(self.state, TodoState::Completed));

        let desc_text = self.description_text();

        // Cancelled wraps the description in:
        //   Span (style = dim+strikethrough) → Delete → Text
        //
        // Each layer serves a different target:
        //   - Markdown: the `Delete` lowers to GFM `~~description~~` (the
        //     `Span` and its `Style` are ignored by Markdown by contract).
        //   - Browser: the projected `Delete` is the inline strikethrough
        //     wrapper; the `Style` on the `ListItem` carries dim+strikethrough
        //     CSS on the `<li>` itself (`opacity` + `text-decoration-line`).
        //   - Terminal: the inline `Span` carries the dim+strikethrough
        //     emphasis through `render_inline_node` — `Style` set directly on
        //     `ListItem` is not consulted by the list-rendering path, so the
        //     SGR must live on an inline node that the inline pipeline does
        //     consult.
        let desc_node = match self.state {
            TodoState::Cancelled => {
                let mut span = RenderNode::span(
                    Vec::new(),
                    vec![RenderNode::delete(vec![RenderNode::text(desc_text)])],
                );
                if let Some(style) = self.state_style() {
                    span.attrs.set_style(&style);
                }
                span
            }
            _ => RenderNode::text(desc_text),
        };

        let mut item =
            RenderNode::list_item(checked, vec![RenderNode::paragraph(vec![desc_node])]);
        item.attrs.classes = vec![self.state_class().to_string()];
        item.attrs.set_task_hints(&TaskHints {
            state: TaskState::from(self.state.clone()),
        });

        if let Some(style) = self.state_style() {
            // Carry the same emphasis on the `<li>` for Browser CSS
            // lowering; ignored by terminal list rendering but read by the
            // browser tree renderer.
            item.attrs.set_style(&style);
        }

        let mut list = RenderNode::list(false, None, vec![item]);
        list.attrs.set_list_hints(&ListRenderHints {
            bullet: Some(String::new()),
            hanging_indent: true,
            indent_children: Some(0),
        });

        // Layout lives on the `List` (the component's visible block) so
        // `render_with_layout` applies margins around the rendered output.
        // Setting it on the inner `ListItem` would be ignored by the list
        // rendering path.
        if self.layout != Layout::default() {
            list.attrs.set_layout(&self.layout);
        }

        list
    }

    /// Renders this `Todo` through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl so terminal output flows
    /// through the same tree the Browser and Markdown paths consume. Failures
    /// are logged via `tracing::error!` and fall back to an empty string —
    /// `TerminalRenderable::render` is infallible by contract.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_node();
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Todo",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders via the pre-tree bespoke path.
    ///
    /// Retained for parity testing. The active [`TerminalRenderable::render`]
    /// path now delegates to [`Self::render_via_tree`] so user-facing terminal
    /// output flows through the canonical tree.
    ///
    /// ## Notes
    ///
    /// Not part of the stable surface; will be removed once the tree renderer
    /// is the universal default and no parity comparison is needed.
    #[doc(hidden)]
    pub fn render_bespoke(&self, term: &Terminal) -> String {
        let width = term.width();
        let content = self.to_terminal(term);
        self.layout.apply_layout(&content, width)
    }

    /// Reports the Todo item to the terminal. Using a nerd font representation
    /// if the terminal has detected that the font is a nerd font. Otherwise it
    /// uses basic characters which should be in all font variants.
    ///
    /// When the terminal does not support colors (`ColorDepth::None`), plain
    /// ASCII fallbacks are used without any ANSI escape codes.
    ///
    /// Pre-tree implementation, used only by [`Self::render_bespoke`].
    fn to_terminal(&self, term: &Terminal) -> String {
        let todo_icon = TODO_CHAR_LOOKUP
            .get(&self.state)
            .unwrap_or(&TODO_CHAR_LOOKUP[&TodoState::Open]);

        let desc = if self.use_prose {
            Prose::new(&self.description).render(term)
        } else {
            self.description.clone()
        };

        // Check if terminal supports colors
        let has_color = term.color_depth != ColorDepth::None;

        // Get the appropriate fallback icon based on color support
        let fallback_icon = if has_color {
            todo_icon.fallback
        } else {
            match self.state {
                TodoState::Open => FB_CHECKBOX_OPEN,
                TodoState::InProgress => FB_CHECKBOX_IN_PROGRESS_NOCOLOR,
                TodoState::Completed => FB_CHECKBOX_COMPLETED_NOCOLOR,
                TodoState::Cancelled => FB_CHECKBOX_CANCELLED_NOCOLOR,
                TodoState::Blocked => FB_CHECKBOX_BLOCKED_NOCOLOR,
            }
        };

        match self.state {
            TodoState::Cancelled => match term.is_nerd_font {
                Some(true) if has_color => {
                    FontWeight::Dim.wrap(format!("{} {}", todo_icon.nerd, desc))
                }
                Some(true) => {
                    // Nerd font but no color - just use the icon without dim styling
                    format!("{} {}", todo_icon.nerd, desc)
                }
                _ if has_color => FontWeight::Dim.wrap(format!(
                    "{} {}",
                    fallback_icon,
                    Style::Strikethrough.term_wrap(&desc, term)
                )),
                _ => {
                    // No color - plain text with no styling
                    format!("{} {}", fallback_icon, desc)
                }
            },
            _ => match term.is_nerd_font {
                Some(true) => {
                    format!("{} {}", todo_icon.nerd, desc)
                }
                _ => {
                    format!("{} {}", fallback_icon, desc)
                }
            },
        }
    }
}

impl TerminalRenderable for Todo {
    /// Renders to a terminal string at an explicit width.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`]
    /// so terminal output matches the Browser and Markdown paths for the same
    /// component. The legacy bespoke output is retained on
    /// [`Self::render_bespoke`] for parity testing.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        self.render_via_tree(&term)
    }

    /// Renders to the supplied terminal.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`].
    /// The legacy bespoke output is retained on [`Self::render_bespoke`] for
    /// parity testing.
    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    /// Projects this `Todo` into a `NodeKind::List` render-tree node.
    ///
    /// Delegates to the single private projection helper
    /// [`Self::to_render_node`], shared with [`TreeRenderable::render_tree`]
    /// so the terminal compatibility hook and the canonical tree producer
    /// cannot drift.
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_node())
    }
}

impl TreeRenderable for Todo {
    /// Projects the Todo into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`Todo::to_render_node`] so this canonical entry point and the
    /// terminal-compatibility [`TerminalRenderable::render_tree_node`] hook
    /// share one source of truth.
    fn render_tree(&self) -> RenderNode {
        self.to_render_node()
    }
}

impl MarkdownRenderable for Todo {
    /// Renders the Todo as portable Markdown via the canonical render tree.
    ///
    /// The Markdown tree renderer emits `- [x] description` for `Completed`
    /// and `- [ ] description` for every other state. `Cancelled` additionally
    /// wraps the description in `~~...~~` via the projected `Delete` node.
    ///
    /// The `todo-*` state classes are dropped — Markdown has no portable peer
    /// for them. The list-level [`ListRenderHints`] are ignored by the
    /// Markdown renderer by design.
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        match render_markdown_node(&node, &MarkdownRenderOptions::default()) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Todo",
                    dialect = "Markdown",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders the Todo as MarkdownPlus via the canonical render tree.
    ///
    /// Identical to portable Markdown for `Todo` — every state degrades to
    /// GFM task-list syntax (`- [ ]` / `- [x]`), and `Cancelled` uses
    /// `~~...~~` strikethrough which is valid in both dialects.
    fn render_markdown_plus(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Todo",
                    dialect = "MarkdownPlus",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl BrowserRenderable for Todo {
    /// Renders the Todo as an HTML fragment via the canonical render tree.
    ///
    /// Produces a `<ul>` wrapping a single `<li class="todo-<state>">` with
    /// the standard disabled task-list checkbox. `Cancelled` items additionally
    /// wrap the description in `<del>` via the projected `Delete` node.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Todo",
                    error = %error,
                    "render_browser_node failed; emitting empty fragment"
                );
                BrowserFragment::new()
                    .define_as_text_fragment(String::new())
                    .finalize()
            }
        }
    }

    /// Wraps the HTML fragment in a complete [`HtmlPage`], optionally applying
    /// the supplied [`PageOptions`].
    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a terminal with no color support for testing
    fn no_color_terminal() -> Terminal {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::None)
            .build();
        // Ensure no nerd font support for consistent fallback output
        term.is_nerd_font = Some(false);
        term
    }

    /// Create a terminal with color support for testing
    fn color_terminal() -> Terminal {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::TrueColor)
            .build();
        // Ensure no nerd font support for consistent fallback output
        term.is_nerd_font = Some(false);
        term
    }

    /// Helper to create Todo with a specific state
    fn todo_with_state(desc: &str, state: TodoState) -> Todo {
        Todo {
            description: desc.to_string(),
            state,
            ..Todo::default()
        }
    }

    #[test]
    fn test_no_color_open_todo() {
        let term = no_color_terminal();
        let todo = Todo::new("Buy groceries");
        let result = todo.to_terminal(&term);
        // Should use plain ASCII fallback "[ ]" without color codes
        assert_eq!(result, "[ ] Buy groceries");
    }

    #[test]
    fn test_no_color_completed_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Done task", TodoState::Completed);
        let result = todo.to_terminal(&term);
        // Should use plain "[x]" without color codes
        assert_eq!(result, "[x] Done task");
    }

    #[test]
    fn test_no_color_in_progress_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Working on it", TodoState::InProgress);
        let result = todo.to_terminal(&term);
        // Should use plain "[>]" without color codes
        assert_eq!(result, "[>] Working on it");
    }

    #[test]
    fn test_no_color_cancelled_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Dropped task", TodoState::Cancelled);
        let result = todo.to_terminal(&term);
        // Should use plain "[-]" without color codes or strikethrough
        assert_eq!(result, "[-] Dropped task");
    }

    #[test]
    fn test_no_color_blocked_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Waiting on dependency", TodoState::Blocked);
        let result = todo.to_terminal(&term);
        // Should use plain "[!]" without color codes
        assert_eq!(result, "[!] Waiting on dependency");
    }

    #[test]
    fn test_color_completed_todo_has_ansi() {
        let term = color_terminal();
        let todo = todo_with_state("Done task", TodoState::Completed);
        let result = todo.to_terminal(&term);
        // With color support, should contain ANSI escape codes
        assert!(
            result.contains('\x1b'),
            "Expected ANSI codes in colored output: {:?}",
            result
        );
    }

    #[test]
    fn with_state_builder_sets_state() {
        let todo = Todo::new("X").with_state(TodoState::Blocked);
        assert_eq!(todo.state(), &TodoState::Blocked);
    }

    #[test]
    fn tree_projection_is_one_item_list_with_task_hints() {
        use renderable::tree::NodeKind;
        let todo = Todo::new("Do thing").with_state(TodoState::InProgress);
        let node = <Todo as TreeRenderable>::render_tree(&todo);
        match &node.kind {
            NodeKind::List { children, .. } => {
                assert_eq!(children.len(), 1, "one list item");
                let item = &children[0];
                match &item.kind {
                    NodeKind::ListItem { checked, .. } => {
                        // Only Completed sets checked=true; InProgress stays unchecked.
                        assert_eq!(*checked, Some(false));
                    }
                    other => panic!("expected ListItem, got {other:?}"),
                }
                let hints = item.attrs.task_hints().expect("task hints present");
                assert_eq!(hints.state, TaskState::InProgress);
                assert!(
                    item.attrs.classes.contains(&"todo-in-progress".to_string()),
                    "state class on item: {:?}",
                    item.attrs.classes
                );
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn tree_renderable_and_render_tree_node_share_projection() {
        let todo = Todo::new("X").with_state(TodoState::Cancelled);
        let canonical = <Todo as TreeRenderable>::render_tree(&todo);
        let compat = todo.render_tree_node().expect("tree node");
        assert_eq!(
            serde_json::to_value(&canonical).unwrap(),
            serde_json::to_value(&compat).unwrap(),
            "canonical and compatibility projections must serialize identically"
        );
    }
}
