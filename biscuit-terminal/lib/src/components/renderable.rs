use std::any::Any;
use std::rc::Rc;

use renderable::style::Style;
use renderable::tree::RenderNode;

use crate::terminal::Terminal;
use crate::utils::layout::{Alignment, Layout, Length, TargetValue, add_cells};
use crate::utils::wrap_policy::WordWrap;

/// A struct or enum which implements the **TerminalRenderable** trait
/// can be reduced down to a string designed to be displayed
/// in a terminal.
///
/// Every implementer owns a [`Layout`] that controls margins,
/// alignment, word-wrap, and row-fill strategy. The required
/// accessors `layout()` / `layout_mut()` expose it, while
/// the provided builder methods let callers configure it
/// fluently.
pub trait TerminalRenderable: std::fmt::Debug + Any {
    /// **Terminal-Aware Render**
    ///
    /// Renders the component using capabilities from the provided
    /// [`Terminal`] (width, color depth, image support, etc.).
    fn render(&self, term: &Terminal) -> String;

    /// **Optimistic Render**
    ///
    /// Renders without environment detection by assuming a modern
    /// terminal capability set.
    ///
    /// `term_width` provides the terminal width in columns.
    /// When `None`, the component uses 80 columns.
    ///
    /// **NOTE:** you should always use `render()` over `render_optimistic()`
    /// unless there is a good reason not to.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render(&term)
    }

    /// Render with all capabilities copied from `term` but a fixed width override.
    ///
    /// **NOTE:** you should always use `render()` over `render_in_width()`
    /// unless there is a good reason not to.
    fn render_in_width(&self, term: &Terminal, width: u32) -> String {
        let mut term_with_width = Terminal::from(term);
        term_with_width.fixed_width = Some(width);
        self.render(&term_with_width)
    }

    /// Returns a shared reference to the component's layout.
    fn layout(&self) -> &Layout;

    /// Returns a mutable reference to the component's layout.
    fn layout_mut(&mut self) -> &mut Layout;

    /// Set the left margin on this component's layout.
    fn left_margin(mut self, margin: TargetValue<Length>) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().margin.left = margin;
        self
    }

    /// Set the right margin on this component's layout.
    fn right_margin(mut self, margin: TargetValue<Length>) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().margin.right = margin;
        self
    }

    /// Set the top margin on this component's layout.
    fn top_margin(mut self, margin: TargetValue<Length>) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().margin.top = margin;
        self
    }

    /// Set the bottom margin on this component's layout.
    fn bottom_margin(mut self, margin: TargetValue<Length>) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().margin.bottom = margin;
        self
    }

    /// Set the text alignment on this component's layout.
    fn alignment(mut self, alignment: Alignment) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().alignment = alignment;
        self
    }

    /// Set the word-wrap policy on this component's layout.
    fn word_wrap(mut self, wrap: WordWrap) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().word_wrap = wrap;
        self
    }

    /// Replace the entire layout on this component.
    fn with_layout(mut self, layout: Layout) -> Self
    where
        Self: Sized,
    {
        *self.layout_mut() = layout;
        self
    }

    /// Returns the component's appearance attributes.
    ///
    /// ## Default
    ///
    /// Returns [`Style::default`] (an empty style). Components that carry
    /// appearance — typically those that route `render(&term)` through the
    /// canonical render tree — override this and [`style_mut`] to expose their
    /// stored style so the bespoke `render(&term)` surface and the
    /// `render_tree` fold surface agree.
    ///
    /// [`style_mut`]: TerminalRenderable::style_mut
    fn style(&self) -> Style {
        Style::default()
    }

    /// Returns a mutable reference to the component's stored appearance, when
    /// it carries one.
    ///
    /// ## Default
    ///
    /// Returns `None`. Components that store a `Style` field override this to
    /// return `Some(&mut <field>)`, which lets the provided [`with_style`]
    /// builder store a caller-supplied style. Components that do not carry
    /// appearance leave the default; [`with_style`] is then a no-op for them.
    ///
    /// [`with_style`]: TerminalRenderable::with_style
    fn style_mut(&mut self) -> Option<&mut Style> {
        None
    }

    /// Overlays `style` onto the component's stored appearance.
    ///
    /// Provided method: delegates to [`style_mut`] and merges `style` onto the
    /// stored value via [`Style::overlay_onto`], so a component's own defaults
    /// (for example `BlockQuote`'s left border) survive a caller setting a
    /// single property such as `background`. Components without a stored
    /// appearance (the [`style_mut`] default returns `None`) keep the input
    /// unchanged — the style is silently dropped, matching the pre-feature
    /// behavior for components that have no appearance surface.
    ///
    /// [`style_mut`]: TerminalRenderable::style_mut
    fn with_style(mut self, style: Style) -> Self
    where
        Self: Sized,
    {
        if let Some(slot) = self.style_mut() {
            *slot = style.overlay_onto(slot);
        }
        self
    }

    /// Whether this component is block-level (occupies the full width).
    ///
    /// Block-level components are treated differently during composition;
    /// for example, they cannot be placed side-by-side.
    fn is_block_level(&self) -> bool {
        false
    }

    /// Adjust this component's margins to nest inside a parent layout.
    ///
    /// Adds `left_offset` cells to the parent's left margin and `right_offset`
    /// cells to the parent's right margin. Only universal cell-based margins
    /// can absorb the offset; per-target margins are inherited unchanged.
    fn with_parent_layout(mut self, parent: &Layout, left_offset: u32, right_offset: u32) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().margin.left = add_cells(&parent.margin.left, left_offset);
        self.layout_mut().margin.right = add_cells(&parent.margin.right, right_offset);
        self
    }

    fn as_any(&self) -> &dyn Any;

    /// Render for direct terminal output.
    ///
    /// This is the method CLI programs should use when printing a
    /// component directly to the terminal. It is a thin wrapper around
    /// [`render()`](TerminalRenderable::render) that guarantees the returned
    /// string ends with a newline.
    ///
    /// ## Why not `render()` directly?
    ///
    /// `render()` does not append a trailing newline because its output
    /// is designed for **composition**, where one component's output is
    /// embedded inside another. When that output is sent directly to the
    /// terminal via `print!`, the missing newline causes zsh to display
    /// an inverted `%` glyph at the end of the line.
    ///
    /// `display()` solves this by delegating to `render()` and then
    /// ensuring the output is newline-terminated.
    ///
    /// | Method                  | Trailing `\n` | Terminal-aware | Use for                      |
    /// |-------------------------|---------------|---------------|------------------------------|
    /// | `render_optimistic()`   | No            | No            | Composition, embedding       |
    /// | `render()`              | No            | Yes           | Composition, embedding       |
    /// | **`display()`**         | **Yes**       | **Yes**       | **Direct terminal output**   |
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::prelude::*;
    ///
    /// let table = Table::new()
    ///     .with_columns(vec![TableColumn::new("Name")])
    ///     .with_data(vec![vec!["Alice".into()]]);
    ///
    /// let term = Terminal::default();
    /// print!("{}", table.display(&term));
    /// ```
    fn display(&self, term: &Terminal) -> String {
        let rendered = self.render(term);
        if rendered.ends_with('\n') {
            rendered
        } else {
            format!("{rendered}\n")
        }
    }

    /// Projects this component to a canonical render tree node.
    ///
    /// Components that support tree-based rendering return `Some(node)` with
    /// their content projected into the canonical [`RenderNode`] structure.
    /// Components that only support bespoke terminal rendering return `None`.
    ///
    /// ## Default
    ///
    /// Returns `None`. Components opt into tree projection by overriding
    /// this method.
    ///
    /// ## Notes
    ///
    /// The returned node uses [`SourceSpan::synthetic()`] since the content
    /// is generated at runtime rather than parsed from source.
    ///
    /// [`SourceSpan::synthetic()`]: renderable::tree::SourceSpan::synthetic
    fn render_tree_node(&self) -> Option<RenderNode> {
        None
    }

    /// Returns the stable Rust type name for this component.
    ///
    /// Used by the render tree projection layer to produce stable, derive-free
    /// diagnostic labels and observability events when a component falls back
    /// from canonical tree projection to ANSI-stripped text.
    ///
    /// ## Default
    ///
    /// Returns [`std::any::type_name::<Self>()`]. Implementors should not
    /// override this — the default is the authoritative source.
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Re-export of the browser render trait, now homed in `renderable`.
///
/// The trait moved to the `renderable` crate so non-terminal render
/// targets can depend on it without depending on `biscuit-terminal`.
pub use renderable::browser::BrowserRenderable;

/// Overlays `overlay` onto the style already carried by `node`.
///
/// Used by component projection helpers to merge a caller-supplied style
/// (set via [`TerminalRenderable::with_style`]) onto the root node they
/// return, so both the bespoke `render(&term)` path (which routes through
/// the projection) and the `render_tree` fold path carry the same
/// appearance. A component that already sets a style on the node keeps it:
/// `overlay` wins per field and unions `emphasis`.
pub(crate) fn overlay_style_onto_node(node: &mut RenderNode, overlay: &Style) {
    if overlay.is_empty() {
        return;
    }
    let base = node.attrs.style_ref().cloned().unwrap_or_default();
    node.attrs.set_style(&overlay.overlay_onto(&base));
}

/// Content that can be rendered as either plain text or a component.
///
/// This enum allows unified handling of both simple string content and
/// complex renderable components that implement the `TerminalRenderable` trait.
///
/// ## Variants
///
/// - **`String(String)`**: Plain text content that is rendered directly.
/// - **`Component(Rc<dyn TerminalRenderable>)**: A reference-counted pointer to a
///   component that implements the `TerminalRenderable` trait.
///
/// ## Examples
///
/// ```rust
/// use biscuit_terminal::components::renderable::{TerminalRenderable, RenderableTerminalContent};
/// use biscuit_terminal::terminal::Terminal;
///
/// // Create from a plain string
/// let content: RenderableTerminalContent = RenderableTerminalContent::String("Hello, world!".to_string());
///
/// // Or use the From impl for seamless conversion
/// let string_content: RenderableTerminalContent = "Hello".into();
/// ```
///
/// ```rust
/// use biscuit_terminal::components::renderable::{TerminalRenderable, RenderableTerminalContent};
/// use biscuit_terminal::components::prose::Prose;
///
/// // Create from a Prose component
/// let prose = Prose::new("<bold>Styled text</bold>");
/// let component_content: RenderableTerminalContent = prose.into();
/// ```
///
/// ## Extracting Content
///
/// ```rust
/// use biscuit_terminal::components::renderable::RenderableTerminalContent;
///
/// let string_content = RenderableTerminalContent::String("hello".to_string());
/// let text = string_content.as_text();
/// assert_eq!(text, "hello");
/// ```
#[derive(Debug)]
pub enum RenderableTerminalContent {
    String(String),
    Component(Rc<dyn TerminalRenderable>),
}

impl Clone for RenderableTerminalContent {
    fn clone(&self) -> Self {
        match self {
            RenderableTerminalContent::String(s) => RenderableTerminalContent::String(s.clone()),
            RenderableTerminalContent::Component(c) => {
                RenderableTerminalContent::Component(Rc::clone(c))
            }
        }
    }
}

impl<T: TerminalRenderable + 'static> From<T> for RenderableTerminalContent {
    fn from(value: T) -> Self {
        RenderableTerminalContent::Component(Rc::new(value))
    }
}

impl RenderableTerminalContent {
    /// Returns the text content as a string.
    ///
    /// For String variants, returns the string directly.
    /// For Component variants, renders with default terminal detection.
    pub fn as_text(&self) -> String {
        match self {
            RenderableTerminalContent::String(s) => s.clone(),
            RenderableTerminalContent::Component(c) => {
                let term = Terminal::default();
                c.render(&term)
            }
        }
    }
}

impl From<String> for RenderableTerminalContent {
    fn from(value: String) -> Self {
        RenderableTerminalContent::String(value)
    }
}

impl From<&str> for RenderableTerminalContent {
    fn from(value: &str) -> Self {
        RenderableTerminalContent::String(value.to_string())
    }
}

impl<'a> From<&'a String> for RenderableTerminalContent {
    fn from(value: &'a String) -> Self {
        RenderableTerminalContent::String(value.clone())
    }
}
