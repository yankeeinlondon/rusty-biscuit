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
