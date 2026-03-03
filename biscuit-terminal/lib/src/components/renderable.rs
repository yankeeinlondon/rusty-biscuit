use std::any::Any;
use std::rc::Rc;

use crate::terminal::Terminal;
use crate::utils::layout::{Alignment, Layout, Margin, RowFill, WordWrap};

/// A struct or enum which implements the **Renderable** trait
/// can be reduced down to a string designed to be displayed
/// in a terminal.
///
/// Every implementor owns a [`Layout`] that controls margins,
/// alignment, word-wrap, and row-fill strategy. The required
/// accessors `layout()` / `layout_mut()` expose it, while
/// the provided builder methods let callers configure it
/// fluently.
pub trait Renderable: std::fmt::Debug + Any {
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
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render(&term)
    }

    /// Render with all capabilities copied from `term` but a fixed width override.
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
    fn left_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().left_margin = margin;
        self
    }

    /// Set the right margin on this component's layout.
    fn right_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().right_margin = margin;
        self
    }

    /// Set the top margin on this component's layout.
    fn top_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().top_margin = margin;
        self
    }

    /// Set the bottom margin on this component's layout.
    fn bottom_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().bottom_margin = margin;
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

    /// Set the row-fill strategy on this component's layout.
    fn row_fill_strategy(mut self, strategy: RowFill) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().row_fill_strategy = strategy;
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

    /// Whether this component is block-level (occupies the full width).
    ///
    /// Block-level components are treated differently during composition;
    /// for example, they cannot be placed side-by-side.
    fn is_block_level(&self) -> bool {
        false
    }

    /// Adjust this component's margins to nest inside a parent layout.
    ///
    /// Adds `left_offset` characters to the parent's left margin and
    /// `right_offset` characters to the parent's right margin, using
    /// lazy `Margin::Offset` composition so percentages resolve later.
    fn with_parent_layout(mut self, parent: &Layout, left_offset: u32, right_offset: u32) -> Self
    where
        Self: Sized,
    {
        self.layout_mut().left_margin = parent.left_margin.clone().add_chars(left_offset);
        self.layout_mut().right_margin = parent.right_margin.clone().add_chars(right_offset);
        self
    }

    fn as_any(&self) -> &dyn Any;

    /// Render for direct terminal output.
    ///
    /// This is the method CLI programs should use when printing a
    /// component directly to the terminal. It is a thin wrapper around
    /// [`render()`](Renderable::render) that guarantees the returned
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
}

/// Content that can be rendered as either plain text or a component.
///
/// This enum allows unified handling of both simple string content and
/// complex renderable components that implement the `Renderable` trait.
///
/// ## Variants
///
/// - **`String(String)`**: Plain text content that is rendered directly.
/// - **`Component(Rc<dyn Renderable>)**: A reference-counted pointer to a
///   component that implements the `Renderable` trait.
///
/// ## Examples
///
/// ```rust
/// use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
/// use biscuit_terminal::terminal::Terminal;
///
/// // Create from a plain string
/// let content: RenderableContent = RenderableContent::String("Hello, world!".to_string());
///
/// // Or use the From impl for seamless conversion
/// let string_content: RenderableContent = "Hello".into();
/// ```
///
/// ```rust
/// use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
/// use biscuit_terminal::components::prose::Prose;
///
/// // Create from a Prose component
/// let prose = Prose::new("<bold>Styled text</bold>");
/// let component_content: RenderableContent = prose.into();
/// ```
///
/// ## Extracting Content
///
/// ```rust
/// use biscuit_terminal::components::renderable::RenderableContent;
///
/// let string_content = RenderableContent::String("hello".to_string());
/// let text = string_content.as_text();
/// assert_eq!(text, "hello");
/// ```
#[derive(Debug)]
pub enum RenderableContent {
    String(String),
    Component(Rc<dyn Renderable>),
}

impl Clone for RenderableContent {
    fn clone(&self) -> Self {
        match self {
            RenderableContent::String(s) => RenderableContent::String(s.clone()),
            RenderableContent::Component(c) => RenderableContent::Component(Rc::clone(c)),
        }
    }
}

impl<T: Renderable + 'static> From<T> for RenderableContent {
    fn from(value: T) -> Self {
        RenderableContent::Component(Rc::new(value))
    }
}

impl RenderableContent {
    /// Returns the text content as a string.
    ///
    /// For String variants, returns the string directly.
    /// For Component variants, renders with default terminal detection.
    pub fn as_text(&self) -> String {
        match self {
            RenderableContent::String(s) => s.clone(),
            RenderableContent::Component(c) => {
                let term = Terminal::default();
                c.render(&term)
            }
        }
    }
}

impl From<String> for RenderableContent {
    fn from(value: String) -> Self {
        RenderableContent::String(value)
    }
}

impl From<&str> for RenderableContent {
    fn from(value: &str) -> Self {
        RenderableContent::String(value.to_string())
    }
}

impl<'a> From<&'a String> for RenderableContent {
    fn from(value: &'a String) -> Self {
        RenderableContent::String(value.clone())
    }
}
