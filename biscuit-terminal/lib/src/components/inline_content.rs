use std::any::Any;

use crate::components::prose::Prose;
use crate::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use crate::components::text_block::TextBlock;
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

/// A component that concatenates multiple items into a single line without inserting
/// newlines between them.
///
/// Each item can be a plain string or any [`TerminalRenderable`] component.
/// When rendered, every item is resolved to its string form and the
/// results are joined directly — or with an optional separator if one
/// is configured via [`with_separator`](InlineContent::with_separator).
///
/// Unlike block-level components (like [`TextBlock`] or [`Section`]), `InlineContent`
/// does not add line breaks between items. This makes it ideal for creating
/// horizontal sequences of text, styled spans, or comma-separated lists.
///
/// ## Layout & Style Contract
///
/// `InlineContent` is an inline sequence component (spec C7). It carries no
/// block box, so `Layout` box properties (`margin`, `padding`, `width`,
/// `max_width`, `alignment`) are **N/A** — the containing block owns the box.
/// `Style::color` and `Style::emphasis` flow through child components such as
/// [`Prose`]. `Style::background` may be honored by an inline `Span` child,
/// painting only the inline content. `Style::border` is N/A.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Owned builder chain (no mut needed)
/// let inline = InlineContent::default()
///     .with("Hello, ")
///     .with(Prose::new("<b>world</b>"));
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Mutable push chain
/// let mut inline = InlineContent::default();
/// inline.push("foo").push(Prose::new("<b>bar</b>"));
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // From a vec of pre-converted items
/// let inline = InlineContent::new(vec![
///     RenderableTerminalContent::from("foo"),
///     RenderableTerminalContent::from(Prose::new("<b>bar</b>")),
/// ]);
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // With separator
/// let inline = InlineContent::default()
///     .with_separator(" | ")
///     .with("one")
///     .with("two")
///     .with("three");
/// assert_eq!(inline.render_optimistic(Some(80)), "one | two | three");
/// ```
///
/// ## Comparison with Block-Level Components
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // InlineContent: no newlines between items
/// let inline = InlineContent::default()
///     .with("a")
///     .with("b")
///     .with("c");
/// assert!(!inline.render_optimistic(Some(80)).contains('\n'));
/// // Output: "abc"
///
/// // TextBlock: each item on its own line (block-level)
/// // let block = TextBlock::new("a").push("b").push("c");
/// // Output: "a\nb\nc"
/// ```
#[derive(Debug)]
pub struct InlineContent {
    parts: Vec<RenderableTerminalContent>,
    separator: Option<String>,
    layout: Layout,
}

// =========================================================================
// Construction
// =========================================================================

impl InlineContent {
    /// Creates a new `InlineContent` from a vector of renderable items.
    pub fn new(items: Vec<RenderableTerminalContent>) -> Self {
        InlineContent {
            parts: items,
            separator: None,
            layout: Layout::default(),
        }
    }
}

impl Default for InlineContent {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

// =========================================================================
// From implementations
// =========================================================================

impl From<String> for InlineContent {
    fn from(value: String) -> Self {
        InlineContent {
            parts: vec![RenderableTerminalContent::String(value)],
            separator: None,
            layout: Layout::default(),
        }
    }
}

impl From<&str> for InlineContent {
    fn from(value: &str) -> Self {
        InlineContent {
            parts: vec![RenderableTerminalContent::String(value.into())],
            separator: None,
            layout: Layout::default(),
        }
    }
}

impl From<&String> for InlineContent {
    fn from(value: &String) -> Self {
        InlineContent {
            parts: vec![RenderableTerminalContent::String(value.clone())],
            separator: None,
            layout: Layout::default(),
        }
    }
}

impl From<Vec<RenderableTerminalContent>> for InlineContent {
    fn from(items: Vec<RenderableTerminalContent>) -> Self {
        InlineContent {
            parts: items,
            separator: None,
            layout: Layout::default(),
        }
    }
}

impl From<RenderableTerminalContent> for InlineContent {
    fn from(value: RenderableTerminalContent) -> Self {
        InlineContent {
            parts: vec![value],
            separator: None,
            layout: Layout::default(),
        }
    }
}

impl From<Prose> for InlineContent {
    fn from(value: Prose) -> Self {
        InlineContent {
            parts: vec![RenderableTerminalContent::from(value)],
            separator: None,
            layout: Layout::default(),
        }
    }
}

impl From<TextBlock> for InlineContent {
    fn from(value: TextBlock) -> Self {
        InlineContent {
            parts: vec![RenderableTerminalContent::from(value)],
            separator: None,
            layout: Layout::default(),
        }
    }
}

// =========================================================================
// Builder methods (owned — returns Self)
// =========================================================================

impl InlineContent {
    /// Appends an item and returns ownership for fluent chaining.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::prelude::*;
    ///
    /// let inline = InlineContent::default()
    ///     .with("Hello, ")
    ///     .with(Prose::new("<b>world</b>"));
    /// ```
    pub fn with<T: Into<RenderableTerminalContent>>(mut self, item: T) -> Self {
        self.parts.push(item.into());
        self
    }

    /// Sets a separator string inserted between items during rendering.
    ///
    /// By default items are concatenated directly with no separator.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::prelude::*;
    ///
    /// let inline = InlineContent::default()
    ///     .with_separator(", ")
    ///     .with("a")
    ///     .with("b")
    ///     .with("c");
    /// assert_eq!(inline.render_optimistic(Some(80)), "a, b, c");
    /// ```
    pub fn with_separator<T: Into<String>>(mut self, sep: T) -> Self {
        self.separator = Some(sep.into());
        self
    }
}

// =========================================================================
// Builder methods (mutable — returns &mut Self)
// =========================================================================

impl InlineContent {
    /// Appends any item that converts to [`RenderableTerminalContent`].
    pub fn push<T: Into<RenderableTerminalContent>>(&mut self, item: T) -> &mut Self {
        self.parts.push(item.into());
        self
    }

    /// Appends plain text content.
    pub fn add_text<T: Into<String>>(&mut self, content: T) -> &mut Self {
        self.parts
            .push(RenderableTerminalContent::String(content.into()));
        self
    }

    /// Appends a [`Prose`] component.
    pub fn add_prose(&mut self, content: Prose) -> &mut Self {
        self.parts.push(RenderableTerminalContent::from(content));
        self
    }

    /// Appends a [`TextBlock`] component.
    pub fn add_text_block(&mut self, content: TextBlock) -> &mut Self {
        self.parts.push(RenderableTerminalContent::from(content));
        self
    }
}

// =========================================================================
// Utility methods
// =========================================================================

impl InlineContent {
    /// Returns the number of items.
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// Returns `true` if there are no items.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

// =========================================================================
// TerminalRenderable implementation
// =========================================================================

impl TerminalRenderable for InlineContent {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        let mut output = String::new();
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0
                && let Some(ref sep) = self.separator
            {
                output.push_str(sep);
            }
            match part {
                RenderableTerminalContent::String(s) => output.push_str(s),
                RenderableTerminalContent::Component(c) => output.push_str(&c.render(term)),
            }
        }
        output
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::compose::Compose;
    use crate::utils::layout::{Alignment, Length, TargetValue};
    use crate::utils::wrap_policy::WordWrap;

    // =====================================================================
    // Construction
    // =====================================================================

    #[test]
    fn test_default_is_empty() {
        let inline = InlineContent::default();
        assert!(inline.is_empty());
        assert_eq!(inline.len(), 0);
        assert_eq!(inline.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_new_with_items() {
        let inline = InlineContent::new(vec![
            RenderableTerminalContent::from("a"),
            RenderableTerminalContent::from("b"),
        ]);
        assert_eq!(inline.len(), 2);
        assert_eq!(inline.render_optimistic(Some(80)), "ab");
    }

    #[test]
    fn test_new_with_empty_vec() {
        let inline = InlineContent::new(Vec::new());
        assert!(inline.is_empty());
        assert_eq!(inline.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_new_empty_vec_matches_default() {
        let from_new = InlineContent::new(Vec::new());
        let from_default = InlineContent::default();
        assert_eq!(
            from_new.render_optimistic(Some(80)),
            from_default.render_optimistic(Some(80)),
        );
    }

    // =====================================================================
    // From implementations
    // =====================================================================

    #[test]
    fn test_from_str() {
        let inline = InlineContent::from("hello");
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn test_from_string() {
        let inline = InlineContent::from(String::from("hello"));
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn test_from_string_ref() {
        let s = String::from("hello");
        let inline = InlineContent::from(&s);
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn test_from_string_ref_does_not_consume_original() {
        let s = String::from("borrow me");
        let _inline = InlineContent::from(&s);
        // original is still usable
        assert_eq!(s, "borrow me");
    }

    #[test]
    fn test_from_vec_renderable_content() {
        let items = vec![
            RenderableTerminalContent::from("x"),
            RenderableTerminalContent::from("y"),
        ];
        let inline = InlineContent::from(items);
        assert_eq!(inline.len(), 2);
        assert_eq!(inline.render_optimistic(Some(80)), "xy");
    }

    #[test]
    fn test_from_vec_renderable_content_single_item() {
        let items = vec![RenderableTerminalContent::from("solo")];
        let inline = InlineContent::from(items);
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "solo");
    }

    #[test]
    fn test_from_renderable_content_string_variant() {
        let content = RenderableTerminalContent::String("direct".into());
        let inline = InlineContent::from(content);
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "direct");
    }

    #[test]
    fn test_from_renderable_content_component_variant() {
        let content = RenderableTerminalContent::from(Prose::new("component"));
        let inline = InlineContent::from(content);
        assert_eq!(inline.len(), 1);
        assert!(inline.render_optimistic(Some(80)).contains("component"));
    }

    #[test]
    fn test_from_prose() {
        let inline = InlineContent::from(Prose::new("styled"));
        assert_eq!(inline.len(), 1);
        assert!(inline.render_optimistic(Some(80)).contains("styled"));
    }

    #[test]
    fn test_from_text_block() {
        let inline = InlineContent::from(TextBlock::new("block"));
        assert_eq!(inline.len(), 1);
        assert!(inline.render_optimistic(Some(80)).contains("block"));
    }

    #[test]
    fn test_from_empty_str() {
        let inline = InlineContent::from("");
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_from_empty_string() {
        let inline = InlineContent::from(String::new());
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "");
    }

    // =====================================================================
    // Owned builder (with)
    // =====================================================================

    #[test]
    fn test_with_chaining() {
        let inline = InlineContent::default().with("a").with("b").with("c");
        assert_eq!(inline.len(), 3);
        assert_eq!(inline.render_optimistic(Some(80)), "abc");
    }

    #[test]
    fn test_with_mixed_types() {
        let inline = InlineContent::default()
            .with("text ")
            .with(Prose::new("styled"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.starts_with("text "));
        assert!(output.contains("styled"));
    }

    #[test]
    fn test_with_from_starting_point() {
        let inline = InlineContent::from("start").with(" middle").with(" end");
        assert_eq!(inline.render_optimistic(Some(80)), "start middle end");
    }

    #[test]
    fn test_with_string_owned() {
        let inline = InlineContent::default().with(String::from("owned"));
        assert_eq!(inline.render_optimistic(Some(80)), "owned");
    }

    #[test]
    fn test_with_renderable_content_directly() {
        let content = RenderableTerminalContent::from("via rc");
        let inline = InlineContent::default().with(content);
        assert_eq!(inline.render_optimistic(Some(80)), "via rc");
    }

    #[test]
    fn test_with_text_block() {
        let inline = InlineContent::default()
            .with("prefix ")
            .with(TextBlock::new("block"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.starts_with("prefix "));
        assert!(output.contains("block"));
    }

    #[test]
    fn test_with_single_item() {
        let inline = InlineContent::default().with("only");
        assert_eq!(inline.len(), 1);
        assert_eq!(inline.render_optimistic(Some(80)), "only");
    }

    // =====================================================================
    // Separator
    // =====================================================================

    #[test]
    fn test_no_separator_by_default() {
        let inline = InlineContent::default().with("a").with("b").with("c");
        assert_eq!(inline.render_optimistic(Some(80)), "abc");
    }

    #[test]
    fn test_with_separator_comma() {
        let inline = InlineContent::default()
            .with_separator(", ")
            .with("one")
            .with("two")
            .with("three");
        assert_eq!(inline.render_optimistic(Some(80)), "one, two, three");
    }

    #[test]
    fn test_separator_with_single_item() {
        let inline = InlineContent::default().with_separator(" | ").with("only");
        assert_eq!(inline.render_optimistic(Some(80)), "only");
    }

    #[test]
    fn test_separator_with_two_items() {
        let inline = InlineContent::default()
            .with_separator(" | ")
            .with("a")
            .with("b");
        assert_eq!(inline.render_optimistic(Some(80)), "a | b");
    }

    #[test]
    fn test_separator_empty_string() {
        let inline = InlineContent::default()
            .with_separator("")
            .with("a")
            .with("b");
        assert_eq!(inline.render_optimistic(Some(80)), "ab");
    }

    #[test]
    fn test_separator_no_items() {
        let inline = InlineContent::default().with_separator(", ");
        assert_eq!(inline.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_separator_with_components() {
        let inline = InlineContent::default()
            .with_separator(" + ")
            .with(Prose::new("alpha"))
            .with(Prose::new("beta"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.contains("alpha"));
        assert!(output.contains(" + "));
        assert!(output.contains("beta"));
    }

    #[test]
    fn test_separator_with_mixed_string_and_component() {
        let inline = InlineContent::default()
            .with_separator(": ")
            .with("label")
            .with(Prose::new("value"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.starts_with("label: "));
        assert!(output.contains("value"));
    }

    #[test]
    fn test_separator_override_replaces_previous() {
        let inline = InlineContent::default()
            .with_separator("WRONG")
            .with_separator(", ")
            .with("a")
            .with("b");
        assert_eq!(inline.render_optimistic(Some(80)), "a, b");
    }

    #[test]
    fn test_separator_with_owned_string() {
        let inline = InlineContent::default()
            .with_separator(String::from(" -> "))
            .with("a")
            .with("b");
        assert_eq!(inline.render_optimistic(Some(80)), "a -> b");
    }

    #[test]
    fn test_separator_single_char() {
        let inline = InlineContent::default()
            .with_separator("/")
            .with("usr")
            .with("local")
            .with("bin");
        assert_eq!(inline.render_optimistic(Some(80)), "usr/local/bin");
    }

    // =====================================================================
    // Mutable builder (push / add_*)
    // =====================================================================

    #[test]
    fn test_push_chaining() {
        let mut inline = InlineContent::default();
        inline.push("a").push("b").push("c");
        assert_eq!(inline.render_optimistic(Some(80)), "abc");
    }

    #[test]
    fn test_push_mixed_types() {
        let mut inline = InlineContent::default();
        inline.push("text ").push(Prose::new("styled"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.starts_with("text "));
        assert!(output.contains("styled"));
    }

    #[test]
    fn test_push_owned_string() {
        let mut inline = InlineContent::default();
        inline.push(String::from("owned"));
        assert_eq!(inline.render_optimistic(Some(80)), "owned");
    }

    #[test]
    fn test_push_renderable_content() {
        let mut inline = InlineContent::default();
        inline.push(RenderableTerminalContent::from("content"));
        assert_eq!(inline.render_optimistic(Some(80)), "content");
    }

    #[test]
    fn test_add_text_str() {
        let mut inline = InlineContent::default();
        inline.add_text("hello").add_text(" world");
        assert_eq!(inline.render_optimistic(Some(80)), "hello world");
    }

    #[test]
    fn test_add_text_owned_string() {
        let mut inline = InlineContent::default();
        inline.add_text(String::from("owned"));
        assert_eq!(inline.render_optimistic(Some(80)), "owned");
    }

    #[test]
    fn test_add_prose() {
        let mut inline = InlineContent::default();
        inline.add_text("prefix: ").add_prose(Prose::new("content"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.starts_with("prefix: "));
        assert!(output.contains("content"));
    }

    #[test]
    fn test_add_text_block() {
        let mut inline = InlineContent::default();
        inline
            .add_text("before ")
            .add_text_block(TextBlock::new("styled"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.starts_with("before "));
        assert!(output.contains("styled"));
    }

    #[test]
    fn test_mixed_add_methods() {
        let mut inline = InlineContent::default();
        inline
            .add_text("plain ")
            .add_prose(Prose::new("prose "))
            .add_text_block(TextBlock::new("block"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.starts_with("plain "));
        assert!(output.contains("prose"));
        assert!(output.contains("block"));
        assert_eq!(inline.len(), 3);
    }

    // =====================================================================
    // Mixed builder styles
    // =====================================================================

    #[test]
    fn test_with_then_push() {
        let mut inline = InlineContent::default().with("a");
        inline.push("b");
        assert_eq!(inline.render_optimistic(Some(80)), "ab");
    }

    #[test]
    fn test_from_then_push() {
        let mut inline = InlineContent::from("start");
        inline.push(" end");
        assert_eq!(inline.render_optimistic(Some(80)), "start end");
    }

    #[test]
    fn test_from_then_add_text() {
        let mut inline = InlineContent::from("label");
        inline.add_text(": value");
        assert_eq!(inline.render_optimistic(Some(80)), "label: value");
    }

    // =====================================================================
    // No newlines between items
    // =====================================================================

    #[test]
    fn test_no_newlines_between_string_items() {
        let inline = InlineContent::new(vec![
            RenderableTerminalContent::from("line1"),
            RenderableTerminalContent::from("line2"),
            RenderableTerminalContent::from("line3"),
        ]);
        let output = inline.render_optimistic(Some(80));
        assert!(!output.contains('\n'));
        assert_eq!(output, "line1line2line3");
    }

    #[test]
    fn test_no_newlines_between_mixed_items() {
        let inline = InlineContent::default()
            .with("text")
            .with(Prose::new("prose"))
            .with(TextBlock::new("block"));
        let output = inline.render_optimistic(Some(80));
        assert!(!output.contains('\n'));
    }

    // =====================================================================
    // Utility methods
    // =====================================================================

    #[test]
    fn test_len_empty() {
        assert_eq!(InlineContent::default().len(), 0);
    }

    #[test]
    fn test_len_after_push() {
        let mut inline = InlineContent::default();
        inline.push("a");
        assert_eq!(inline.len(), 1);
        inline.push("b");
        assert_eq!(inline.len(), 2);
    }

    #[test]
    fn test_len_after_with() {
        let inline = InlineContent::default().with("a").with("b").with("c");
        assert_eq!(inline.len(), 3);
    }

    #[test]
    fn test_is_empty_true() {
        assert!(InlineContent::default().is_empty());
    }

    #[test]
    fn test_is_empty_false() {
        let inline = InlineContent::from("x");
        assert!(!inline.is_empty());
    }

    #[test]
    fn test_len_from_single_item() {
        assert_eq!(InlineContent::from("x").len(), 1);
        assert_eq!(InlineContent::from(String::from("x")).len(), 1);
        assert_eq!(InlineContent::from(Prose::new("x")).len(), 1);
        assert_eq!(InlineContent::from(TextBlock::new("x")).len(), 1);
        assert_eq!(
            InlineContent::from(RenderableTerminalContent::from("x")).len(),
            1
        );
    }

    // =====================================================================
    // TerminalRenderable trait — render / render_optimistic
    // =====================================================================

    #[test]
    fn test_render() {
        let inline = InlineContent::default().with("hello ").with("world");
        let term = Terminal::new_optimistic(80);
        assert_eq!(inline.render(&term), "hello world");
    }

    #[test]
    fn test_render_default_width() {
        let inline = InlineContent::from("test");
        assert_eq!(inline.render_optimistic(None), "test");
    }

    #[test]
    fn test_render_with_explicit_width() {
        let inline = InlineContent::from("test");
        assert_eq!(inline.render_optimistic(Some(120)), "test");
    }

    #[test]
    fn test_render_and_render_optimistic_consistent_for_plain_text() {
        let inline = InlineContent::default().with("hello ").with("world");
        let term = Terminal::new_optimistic(80);
        assert_eq!(inline.render_optimistic(Some(80)), inline.render(&term),);
    }

    #[test]
    fn test_render_empty() {
        assert_eq!(InlineContent::default().render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_render_empty_with_terminal() {
        let term = Terminal::new_optimistic(80);
        assert_eq!(InlineContent::default().render(&term), "");
    }

    // =====================================================================
    // TerminalRenderable trait — display
    // =====================================================================

    #[test]
    fn test_display_adds_newline() {
        let inline = InlineContent::from("no newline");
        let term = Terminal::new_optimistic(80);
        let output = inline.display(&term);
        assert!(output.ends_with('\n'));
        assert_eq!(output, "no newline\n");
    }

    #[test]
    fn test_display_does_not_double_newline() {
        // If content already ends with newline, display should not add another
        let inline = InlineContent::from("has newline\n");
        let term = Terminal::new_optimistic(80);
        let output = inline.display(&term);
        assert!(output.ends_with('\n'));
        assert!(!output.ends_with("\n\n"));
        assert_eq!(output, "has newline\n");
    }

    #[test]
    fn test_display_empty_produces_newline() {
        let inline = InlineContent::default();
        let term = Terminal::new_optimistic(80);
        let output = inline.display(&term);
        assert_eq!(output, "\n");
    }

    // =====================================================================
    // TerminalRenderable trait — is_block_level
    // =====================================================================

    #[test]
    fn test_is_not_block_level() {
        let inline = InlineContent::default();
        assert!(!inline.is_block_level());
    }

    #[test]
    fn test_is_not_block_level_with_content() {
        let inline = InlineContent::from("content");
        assert!(!inline.is_block_level());
    }

    // =====================================================================
    // TerminalRenderable trait — layout builder methods
    // =====================================================================

    #[test]
    fn test_layout_access() {
        let mut inline = InlineContent::default();
        inline.layout_mut().alignment = Alignment::Center;
        assert_eq!(inline.layout().alignment, Alignment::Center);
    }

    #[test]
    fn test_left_margin_builder() {
        let inline = InlineContent::from("test").left_margin(TargetValue::universal(Length::ch(4)));
        assert_eq!(
            inline.layout().margin.left,
            TargetValue::universal(Length::ch(4))
        );
    }

    #[test]
    fn test_right_margin_builder() {
        let inline =
            InlineContent::from("test").right_margin(TargetValue::universal(Length::ch(4)));
        assert_eq!(
            inline.layout().margin.right,
            TargetValue::universal(Length::ch(4))
        );
    }

    #[test]
    fn test_top_margin_builder() {
        let inline = InlineContent::from("test").top_margin(TargetValue::universal(Length::ch(2)));
        assert_eq!(
            inline.layout().margin.top,
            TargetValue::universal(Length::ch(2))
        );
    }

    #[test]
    fn test_bottom_margin_builder() {
        let inline =
            InlineContent::from("test").bottom_margin(TargetValue::universal(Length::ch(2)));
        assert_eq!(
            inline.layout().margin.bottom,
            TargetValue::universal(Length::ch(2))
        );
    }

    #[test]
    fn test_alignment_builder() {
        let inline = InlineContent::from("test").alignment(Alignment::Right);
        assert_eq!(inline.layout().alignment, Alignment::Right);
    }

    #[test]
    fn test_word_wrap_builder() {
        let inline = InlineContent::from("test").word_wrap(WordWrap::None);
        assert_eq!(inline.layout().word_wrap, WordWrap::None);
    }

    #[test]
    fn test_chained_layout_builders() {
        let inline = InlineContent::from("test")
            .left_margin(TargetValue::universal(Length::ch(2)))
            .right_margin(TargetValue::universal(Length::ch(2)))
            .alignment(Alignment::Center);
        assert_eq!(
            inline.layout().margin.left,
            TargetValue::universal(Length::ch(2))
        );
        assert_eq!(
            inline.layout().margin.right,
            TargetValue::universal(Length::ch(2))
        );
        assert_eq!(inline.layout().alignment, Alignment::Center);
    }

    #[test]
    fn test_layout_builders_chain_with_content_builders() {
        let inline = InlineContent::default()
            .with("hello")
            .left_margin(TargetValue::universal(Length::ch(4)))
            .with(" world");
        assert_eq!(
            inline.layout().margin.left,
            TargetValue::universal(Length::ch(4))
        );
        assert_eq!(inline.len(), 2);
    }

    // =====================================================================
    // TerminalRenderable trait — as_any / Debug
    // =====================================================================

    #[test]
    fn test_as_any_downcast() {
        let inline = InlineContent::from("test");
        let any_ref = inline.as_any();
        assert!(any_ref.downcast_ref::<InlineContent>().is_some());
    }

    #[test]
    fn test_as_any_wrong_type() {
        let inline = InlineContent::from("test");
        let any_ref = inline.as_any();
        assert!(any_ref.downcast_ref::<Prose>().is_none());
    }

    #[test]
    fn test_debug_output() {
        let inline = InlineContent::from("debug me");
        let debug = format!("{:?}", inline);
        assert!(debug.contains("InlineContent"));
    }

    #[test]
    fn test_debug_shows_parts() {
        let inline = InlineContent::default().with("a").with("b");
        let debug = format!("{:?}", inline);
        assert!(debug.contains("InlineContent"));
        assert!(debug.contains("parts"));
    }

    // =====================================================================
    // Edge cases — unicode, emoji, special characters
    // =====================================================================

    #[test]
    fn test_unicode_content() {
        let inline = InlineContent::default().with("Hello ").with("世界");
        assert_eq!(inline.render_optimistic(Some(80)), "Hello 世界");
    }

    #[test]
    fn test_emoji_content() {
        let inline = InlineContent::default().with("Status: ").with("✅");
        assert_eq!(inline.render_optimistic(Some(80)), "Status: ✅");
    }

    #[test]
    fn test_mixed_unicode_scripts() {
        let inline = InlineContent::default()
            .with_separator(" • ")
            .with("English")
            .with("日本語")
            .with("العربية");
        let output = inline.render_optimistic(Some(80));
        assert!(output.contains("English"));
        assert!(output.contains("日本語"));
        assert!(output.contains("العربية"));
        assert!(output.contains(" • "));
    }

    #[test]
    fn test_emoji_separator() {
        let inline = InlineContent::default()
            .with_separator(" 🔸 ")
            .with("a")
            .with("b");
        assert_eq!(inline.render_optimistic(Some(80)), "a 🔸 b");
    }

    // =====================================================================
    // Edge cases — empty and whitespace items
    // =====================================================================

    #[test]
    fn test_empty_string_items() {
        let inline = InlineContent::default().with("").with("content").with("");
        assert_eq!(inline.render_optimistic(Some(80)), "content");
        assert_eq!(inline.len(), 3);
    }

    #[test]
    fn test_whitespace_only_items() {
        let inline = InlineContent::default()
            .with("   ")
            .with("text")
            .with("   ");
        assert_eq!(inline.render_optimistic(Some(80)), "   text   ");
    }

    #[test]
    fn test_single_character() {
        let inline = InlineContent::from("X");
        assert_eq!(inline.render_optimistic(Some(80)), "X");
    }

    #[test]
    fn test_tab_characters() {
        let inline = InlineContent::default()
            .with("col1")
            .with("\t")
            .with("col2");
        assert_eq!(inline.render_optimistic(Some(80)), "col1\tcol2");
    }

    // =====================================================================
    // Edge cases — many items
    // =====================================================================

    #[test]
    fn test_many_items() {
        let mut inline = InlineContent::default();
        for i in 0..100 {
            inline.push(i.to_string());
        }
        assert_eq!(inline.len(), 100);
        let output = inline.render_optimistic(Some(1000));
        assert!(output.starts_with("0123"));
        assert!(output.ends_with("99"));
    }

    #[test]
    fn test_many_items_with_separator() {
        let mut inline = InlineContent::default().with_separator(",");
        for i in 0..5 {
            inline.push(i.to_string());
        }
        assert_eq!(inline.render_optimistic(Some(80)), "0,1,2,3,4");
    }

    // =====================================================================
    // Prose styling integration
    // =====================================================================

    #[test]
    fn test_prose_bold_renders_inline() {
        let inline = InlineContent::default()
            .with("normal ")
            .with(Prose::new("<bold>bold</bold>"))
            .with(" normal");
        let output = inline.render_optimistic(Some(80));
        // Bold wraps with escape codes: \x1b[1m ... \x1b[22m
        assert!(output.contains("\x1b[1m"));
        assert!(output.contains("bold"));
        assert!(output.starts_with("normal "));
        assert!(output.ends_with(" normal"));
    }

    #[test]
    fn test_prose_html_tags_render_inline() {
        let inline = InlineContent::default()
            .with(Prose::new("<red>error</red>"))
            .with(": something broke");
        let output = inline.render_optimistic(Some(80));
        assert!(output.contains("error"));
        assert!(output.contains(": something broke"));
    }

    #[test]
    fn test_multiple_styled_prose_inline() {
        let inline = InlineContent::default()
            .with(Prose::new("<bold>key</bold>"))
            .with(": ")
            .with(Prose::new("<dim>value</dim>"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.contains("key"));
        assert!(output.contains(": "));
        assert!(output.contains("value"));
    }

    // =====================================================================
    // Nesting — InlineContent inside InlineContent
    // =====================================================================

    #[test]
    fn test_nested_inline_content() {
        let inner = InlineContent::default().with("inner1").with("inner2");
        let outer = InlineContent::default()
            .with("outer ")
            .with(inner)
            .with(" end");
        assert_eq!(outer.render_optimistic(Some(80)), "outer inner1inner2 end");
    }

    #[test]
    fn test_nested_inline_content_with_separator() {
        let inner = InlineContent::default()
            .with_separator("+")
            .with("a")
            .with("b");
        let outer = InlineContent::default()
            .with_separator(" | ")
            .with("start")
            .with(inner)
            .with("end");
        assert_eq!(outer.render_optimistic(Some(80)), "start | a+b | end");
    }

    // =====================================================================
    // Nesting — Compose inside InlineContent
    // =====================================================================

    #[test]
    fn test_compose_inside_inline_content() {
        let mut compose = Compose::default();
        compose.add_text("composed");
        let inline = InlineContent::default()
            .with("before ")
            .with(compose)
            .with(" after");
        assert_eq!(inline.render_optimistic(Some(80)), "before composed after");
    }

    // =====================================================================
    // Separator respects push/add methods too
    // =====================================================================

    #[test]
    fn test_separator_with_push() {
        let mut inline = InlineContent::default().with_separator(" - ");
        inline.push("x").push("y").push("z");
        assert_eq!(inline.render_optimistic(Some(80)), "x - y - z");
    }

    #[test]
    fn test_separator_with_add_text() {
        let mut inline = InlineContent::default().with_separator(".");
        inline.add_text("com").add_text("example").add_text("www");
        assert_eq!(inline.render_optimistic(Some(80)), "com.example.www");
    }

    #[test]
    fn test_separator_with_add_prose() {
        let mut inline = InlineContent::default().with_separator(" ");
        inline
            .add_prose(Prose::new("hello"))
            .add_prose(Prose::new("world"));
        let output = inline.render_optimistic(Some(80));
        assert!(output.contains("hello"));
        assert!(output.contains("world"));
    }
}
