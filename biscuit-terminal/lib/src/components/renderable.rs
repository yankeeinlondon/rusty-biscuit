use std::any::Any;
use std::sync::Arc;

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
    /// **Opportunistic Render**
    ///
    /// Renders without knowledge of the underlying terminal's
    /// capabilities with an "opportunistic" view that the
    /// terminal supports all capabilities.
    ///
    /// `term_width` provides the terminal width in columns.
    /// When `None`, the component should assume a reasonable
    /// default (typically 80).
    fn render(&self, term_width: Option<u32>) -> String;

    /// **Fallback Render**
    ///
    /// Renders the component based on the capabilities of the
    /// passed in `Terminal`. Will provide graceful fallbacks
    /// when possible.
    fn fallback_render(&self, term: &Terminal) -> String;

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
    fn as_child_of(mut self, parent: &Layout, left_offset: u32, right_offset: u32) -> Self
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
    /// [`fallback_render()`](Renderable::fallback_render) that
    /// guarantees the returned string ends with a newline.
    ///
    /// ## Why not `render()` or `fallback_render()`?
    ///
    /// Neither `render()` nor `fallback_render()` append a trailing
    /// newline — their output is designed for **composition**, where
    /// one component's output is embedded inside another. When that
    /// output is sent directly to the terminal via `print!`, the
    /// missing newline causes zsh to display an inverted `%` glyph
    /// at the end of the line.
    ///
    /// `display()` solves this by delegating to `fallback_render()`
    /// (so you get capability-aware rendering with graceful
    /// degradation) and then ensuring the output is
    /// newline-terminated.
    ///
    /// | Method             | Trailing `\n` | Terminal-aware | Use for                      |
    /// |--------------------|---------------|---------------|------------------------------|
    /// | `render()`         | No            | No            | Composition, embedding       |
    /// | `fallback_render()`| No            | Yes           | Composition, embedding       |
    /// | **`display()`**    | **Yes**       | **Yes**       | **Direct terminal output**   |
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
    /// // Detect the real terminal (width, color depth, image support, …)
    /// let term = Terminal::default();
    /// print!("{}", table.display(&term));
    /// ```
    fn display(&self, term: &Terminal) -> String {
        let rendered = self.fallback_render(term);
        if rendered.ends_with('\n') {
            rendered
        } else {
            format!("{rendered}\n")
        }
    }
}

#[derive(Debug)]
pub enum RenderableContent {
    String(String),
    Component(Arc<dyn Renderable>),
}

impl Clone for RenderableContent {
    fn clone(&self) -> Self {
        match self {
            RenderableContent::String(s) => RenderableContent::String(s.clone()),
            RenderableContent::Component(c) => RenderableContent::Component(Arc::clone(c)),
        }
    }
}

impl<T: Renderable + 'static> From<T> for RenderableContent {
    fn from(value: T) -> Self {
        RenderableContent::Component(Arc::new(value))
    }
}

impl RenderableContent {
    /// Returns the text content as a string.
    ///
    /// For String variants, returns the string directly.
    /// For Component variants, renders using fallback with default terminal.
    pub fn as_text(&self) -> String {
        match self {
            RenderableContent::String(s) => s.clone(),
            RenderableContent::Component(c) => {
                let term = Terminal::default();
                c.fallback_render(&term)
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
