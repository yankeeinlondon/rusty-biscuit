use std::any::Any;

use crate::components::{
    filesystem::FileSystem,
    list::{OrderedList, UnorderedList},
    prose::Prose,
    renderable::{Renderable, RenderableContent},
    section::{HeadingLevel, Section},
    table::table::Table,
};
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

/// Composes multiple renderable components into a single renderable output.
///
/// This struct allows combining text, styled prose, tables, lists, and other
/// renderable components into one cohesive output for terminal display.
/// Parts are rendered sequentially with no automatic spacing between them.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // From a vec of pre-converted items
/// let compose = Compose::new(vec![
///     RenderableContent::from("Hello, "),
///     RenderableContent::from(Prose::new("{{bold}}world{{reset}}!")),
/// ]);
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Builder-style with fluent API
/// let mut compose = Compose::default();
/// compose
///     .add_text("Hello, ")
///     .add_prose(Prose::new("{{bold}}world{{reset}}!"));
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Using From implementations for ergonomic creation
/// let text: Compose = "Hello, ".into();
/// let prose = Prose::new("{{bold}}bold text{{reset}}");
/// let combined = Compose::new(vec![text.into(), RenderableContent::from(prose)]);
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Building a mixed content document
/// let mut doc = Compose::default();
/// doc
///     .add_heading("Project Overview", 1)
///     .add_text("This project contains ")
///     .add_prose(Prose::new("{{bold}}important{{reset}} files"))
///     .add_text(" for processing.");
/// ```
#[derive(Debug)]
pub struct Compose {
    parts: Vec<RenderableContent>,
    layout: Layout,
}

impl Default for Compose {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl From<String> for Compose {
    fn from(value: String) -> Self {
        Compose {
            parts: vec![RenderableContent::String(value)],
            layout: Layout::default(),
        }
    }
}

impl From<&str> for Compose {
    fn from(value: &str) -> Self {
        Compose {
            parts: vec![RenderableContent::String(value.into())],
            layout: Layout::default(),
        }
    }
}

impl From<RenderableContent> for Compose {
    fn from(value: RenderableContent) -> Self {
        Compose {
            parts: vec![value],
            layout: Layout::default(),
        }
    }
}

impl From<Vec<RenderableContent>> for Compose {
    fn from(items: Vec<RenderableContent>) -> Self {
        Compose {
            parts: items,
            layout: Layout::default(),
        }
    }
}

impl Renderable for Compose {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        tracing::trace!(parts = self.parts.len(), "Compose rendering");
        let mut output = String::new();
        for part in &self.parts {
            match part {
                RenderableContent::String(s) => output.push_str(s),
                RenderableContent::Component(c) => output.push_str(&c.render(term)),
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

impl Compose {
    /// Creates a new `Compose` from a vector of renderable items.
    pub fn new(items: Vec<RenderableContent>) -> Self {
        Compose {
            parts: items,
            layout: Layout::default(),
        }
    }

    /// Adds a block of _prose_ which is text that is allowed
    /// to embed styling tokens in it that can be rendered lazily
    /// when we're ready to send to the terminal.
    pub fn add_prose(&mut self, content: Prose) -> &mut Self {
        self.parts.push(RenderableContent::from(content));
        self
    }

    /// Adds plain text content.
    pub fn add_text<T: Into<String>>(&mut self, content: T) -> &mut Self {
        let text = content.into();
        self.parts.push(RenderableContent::from(text));
        self
    }

    /// Adds an unordered list.
    pub fn add_unordered_list(&mut self, content: UnorderedList) -> &mut Self {
        self.parts.push(RenderableContent::from(content));
        self
    }

    /// Adds an ordered list.
    pub fn add_ordered_list(&mut self, content: OrderedList) -> &mut Self {
        self.parts.push(RenderableContent::from(content));
        self
    }

    /// Adds a [`FileSystem`] tree component.
    pub fn add_file_system(&mut self, content: FileSystem) -> &mut Self {
        self.parts.push(RenderableContent::from(content));
        self
    }

    /// Adds a [`Table`] component.
    pub fn add_table(&mut self, content: Table) -> &mut Self {
        self.parts.push(RenderableContent::from(content));
        self
    }

    /// Adds a heading as a [`Section`] component.
    ///
    /// The `level` parameter maps to heading levels 1-6 (h1-h6).
    pub fn add_heading<T: Into<String>>(&mut self, title: T, level: u8) -> &mut Self {
        let heading_level = match level {
            1 => HeadingLevel::h1,
            2 => HeadingLevel::h2,
            3 => HeadingLevel::h3,
            4 => HeadingLevel::h4,
            5 => HeadingLevel::h5,
            _ => HeadingLevel::h6,
        };
        let section = Section::new(heading_level, title);
        self.parts.push(RenderableContent::from(section));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::inline_content::InlineContent;
    use crate::components::table::TableColumn;
    use crate::components::text_block::TextBlock;
    use crate::utils::layout::{Alignment, Margin, RowFill};
    use crate::utils::wrap_policy::WordWrap;

    // =====================================================================
    // Construction
    // =====================================================================

    #[test]
    fn test_new_empty_vec() {
        let compose = Compose::new(Vec::new());
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_new_with_items() {
        let compose = Compose::new(vec![
            RenderableContent::from("foo"),
            RenderableContent::from("bar"),
        ]);
        assert_eq!(compose.render_optimistic(Some(80)), "foobar");
    }

    #[test]
    fn test_new_with_mixed_items() {
        let compose = Compose::new(vec![
            RenderableContent::from("text "),
            RenderableContent::from(Prose::new("styled")),
        ]);
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("text "));
        assert!(output.contains("styled"));
    }

    #[test]
    fn test_default_is_empty() {
        let compose = Compose::default();
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_new_empty_and_default_equivalent() {
        assert_eq!(
            Compose::new(Vec::new()).render_optimistic(Some(80)),
            Compose::default().render_optimistic(Some(80)),
        );
    }

    // =====================================================================
    // From implementations
    // =====================================================================

    #[test]
    fn test_from_str() {
        let compose = Compose::from("Hello");
        assert_eq!(compose.render_optimistic(Some(80)), "Hello");
    }

    #[test]
    fn test_from_string() {
        let compose = Compose::from(String::from("Hello"));
        assert_eq!(compose.render_optimistic(Some(80)), "Hello");
    }

    #[test]
    fn test_from_renderable_content_string_variant() {
        let content = RenderableContent::String("direct".into());
        let compose = Compose::from(content);
        assert_eq!(compose.render_optimistic(Some(80)), "direct");
    }

    #[test]
    fn test_from_renderable_content_component_variant() {
        let content = RenderableContent::from(Prose::new("component"));
        let compose = Compose::from(content);
        assert!(compose.render_optimistic(Some(80)).contains("component"));
    }

    #[test]
    fn test_from_vec_renderable_content() {
        let items = vec![RenderableContent::from("x"), RenderableContent::from("y")];
        let compose = Compose::from(items);
        assert_eq!(compose.render_optimistic(Some(80)), "xy");
    }

    #[test]
    fn test_from_empty_str() {
        let compose = Compose::from("");
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_from_empty_string() {
        let compose = Compose::from(String::new());
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    // =====================================================================
    // add_text
    // =====================================================================

    #[test]
    fn test_add_text_single() {
        let mut compose = Compose::default();
        compose.add_text("hello");
        assert_eq!(compose.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn test_add_text_chaining() {
        let mut compose = Compose::default();
        compose.add_text("Hello, ").add_text("world!");
        assert_eq!(compose.render_optimistic(Some(80)), "Hello, world!");
    }

    #[test]
    fn test_add_text_owned_string() {
        let mut compose = Compose::default();
        compose.add_text(String::from("owned"));
        assert_eq!(compose.render_optimistic(Some(80)), "owned");
    }

    #[test]
    fn test_add_text_multiple() {
        let mut compose = Compose::default();
        compose
            .add_text("a")
            .add_text("b")
            .add_text("c")
            .add_text("d");
        assert_eq!(compose.render_optimistic(Some(80)), "abcd");
    }

    // =====================================================================
    // add_prose
    // =====================================================================

    #[test]
    fn test_add_prose_plain() {
        let mut compose = Compose::default();
        compose.add_prose(Prose::new("plain"));
        assert!(compose.render_optimistic(Some(80)).contains("plain"));
    }

    #[test]
    fn test_add_prose_with_bold_tokens() {
        let mut compose = Compose::default();
        compose.add_prose(Prose::new("{{bold}}bold{{reset}}"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("\x1b[1m"));
        assert!(output.contains("bold"));
    }

    #[test]
    fn test_add_prose_with_html_tags() {
        let mut compose = Compose::default();
        compose.add_prose(Prose::new("<red>error</red>"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("error"));
    }

    #[test]
    fn test_add_prose_chaining() {
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("first"))
            .add_prose(Prose::new("second"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("first"));
        assert!(output.contains("second"));
    }

    // =====================================================================
    // add_unordered_list / add_ordered_list
    // =====================================================================

    #[test]
    fn test_add_unordered_list() {
        let mut compose = Compose::default();
        compose.add_unordered_list(UnorderedList::new(vec!["item1", "item2"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("item1"));
        assert!(output.contains("item2"));
    }

    #[test]
    fn test_add_ordered_list() {
        let mut compose = Compose::default();
        compose.add_ordered_list(OrderedList::new(vec!["first", "second"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("first"));
        assert!(output.contains("second"));
    }

    #[test]
    fn test_add_unordered_list_chaining() {
        let mut compose = Compose::default();
        compose
            .add_text("List:\n")
            .add_unordered_list(UnorderedList::new(vec!["a", "b"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("List:"));
        assert!(output.contains("a"));
        assert!(output.contains("b"));
    }

    #[test]
    fn test_add_ordered_list_chaining() {
        let mut compose = Compose::default();
        compose
            .add_text("Steps:\n")
            .add_ordered_list(OrderedList::new(vec!["do this", "then that"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Steps:"));
        assert!(output.contains("do this"));
    }

    // =====================================================================
    // add_file_system
    // =====================================================================

    fn make_fs_fixture() -> (tempfile::TempDir, FileSystem) {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join("hello.txt"), "world").expect("create file");
        std::fs::create_dir(temp.path().join("src")).expect("create dir");
        std::fs::write(temp.path().join("src/main.rs"), "fn main() {}").expect("create file");
        let mut fs = FileSystem::new(temp.path()).unwrap();
        fs.ensure_tree_built();
        (temp, fs)
    }

    #[test]
    fn test_add_file_system() {
        let (_tmp, fs) = make_fs_fixture();
        let mut compose = Compose::default();
        compose.add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("hello.txt"));
        assert!(output.contains("src"));
    }

    #[test]
    fn test_add_file_system_chaining() {
        let (_tmp, fs) = make_fs_fixture();
        let mut compose = Compose::default();
        compose.add_text("Files:\n").add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("Files:\n"));
        assert!(output.contains("hello.txt"));
    }

    #[test]
    fn test_add_file_system_with_prose() {
        let (_tmp, fs) = make_fs_fixture();
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("{{bold}}Directory listing{{reset}}\n"))
            .add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Directory listing"));
        assert!(output.contains("hello.txt"));
    }

    #[test]
    fn test_add_file_system_with_depth() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join("a/b")).expect("create dirs");
        std::fs::write(temp.path().join("a/b/deep.txt"), "").expect("create file");
        let mut fs = FileSystem::new(temp.path()).unwrap().depth(1);
        fs.ensure_tree_built();
        let mut compose = Compose::default();
        compose.add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        // depth(1) shows the first level but not nested deep.txt
        assert!(!output.is_empty());
    }

    // =====================================================================
    // add_table
    // =====================================================================

    #[test]
    fn test_add_table() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Value")])
            .with_data(vec![
                vec!["Alice".into(), "100".into()],
                vec!["Bob".into(), "200".into()],
            ]);
        let mut compose = Compose::default();
        compose.add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Name"));
        assert!(output.contains("Value"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn test_add_table_chaining() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Col")])
            .with_data(vec![vec!["data".into()]]);
        let mut compose = Compose::default();
        compose.add_text("Results:\n").add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("Results:\n"));
        assert!(output.contains("Col"));
        assert!(output.contains("data"));
    }

    #[test]
    fn test_add_table_with_title() {
        let table = Table::new()
            .with_title("Summary")
            .with_columns(vec![TableColumn::new("Item")])
            .with_data(vec![vec!["test".into()]]);
        let mut compose = Compose::default();
        compose.add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Summary"));
        assert!(output.contains("Item"));
    }

    #[test]
    fn test_add_table_empty() {
        let table = Table::new().with_columns(vec![TableColumn::new("Empty")]);
        let mut compose = Compose::default();
        compose.add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Empty"));
    }

    #[test]
    fn test_add_table_with_prose() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Key"), TableColumn::new("Val")])
            .with_data(vec![vec!["k".into(), "v".into()]]);
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("{{bold}}Table:{{reset}}\n"))
            .add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Table:"));
        assert!(output.contains("Key"));
    }

    // =====================================================================
    // Mixed add methods
    // =====================================================================

    #[test]
    fn test_mixed_text_and_prose() {
        let mut compose = Compose::default();
        compose
            .add_text("normal ")
            .add_prose(Prose::new("styled"))
            .add_text(" normal");
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("normal "));
        assert!(output.contains("styled"));
        assert!(output.ends_with(" normal"));
    }

    #[test]
    fn test_mixed_all_types() {
        let mut compose = Compose::default();
        compose
            .add_text("Header\n")
            .add_prose(Prose::new("description\n"))
            .add_unordered_list(UnorderedList::new(vec!["item"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Header"));
        assert!(output.contains("description"));
        assert!(output.contains("item"));
    }

    #[test]
    fn test_mixed_table_and_file_system() {
        let (_tmp, fs) = make_fs_fixture();
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Metric")])
            .with_data(vec![vec!["count".into()]]);
        let mut compose = Compose::default();
        compose.add_table(table).add_text("\n").add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Metric"));
        assert!(output.contains("count"));
        assert!(output.contains("hello.txt"));
    }

    #[test]
    fn test_from_then_add() {
        let mut compose = Compose::from("start");
        compose.add_text(" end");
        assert_eq!(compose.render_optimistic(Some(80)), "start end");
    }

    // =====================================================================
    // No newlines between items (core behavior)
    // =====================================================================

    #[test]
    fn test_no_newlines_between_string_items() {
        let mut compose = Compose::default();
        compose.add_text("a").add_text("b").add_text("c");
        let output = compose.render_optimistic(Some(80));
        assert!(!output.contains('\n'));
        assert_eq!(output, "abc");
    }

    #[test]
    fn test_no_newlines_between_mixed_items() {
        let mut compose = Compose::default();
        compose.add_text("text").add_prose(Prose::new("prose"));
        let output = compose.render_optimistic(Some(80));
        assert!(!output.contains('\n'));
    }

    #[test]
    fn test_concatenation_preserves_spaces() {
        let mut compose = Compose::default();
        compose.add_text("hello ").add_text("world");
        assert_eq!(compose.render_optimistic(Some(80)), "hello world");
    }

    // =====================================================================
    // Renderable trait — render / render_optimistic
    // =====================================================================

    #[test]
    fn test_render_with_explicit_width() {
        let compose = Compose::from("test");
        assert_eq!(compose.render_optimistic(Some(120)), "test");
    }

    #[test]
    fn test_render_with_none_width() {
        let compose = Compose::from("test");
        assert_eq!(compose.render_optimistic(None), "test");
    }

    #[test]
    fn test_render() {
        let mut compose = Compose::default();
        compose.add_text("hello ").add_text("world");
        let term = Terminal::new_optimistic(80);
        assert_eq!(compose.render(&term), "hello world");
    }

    #[test]
    fn test_render_and_render_optimistic_consistent_for_plain_text() {
        let mut compose = Compose::default();
        compose.add_text("hello ").add_text("world");
        let term = Terminal::new_optimistic(80);
        assert_eq!(compose.render_optimistic(Some(80)), compose.render(&term),);
    }

    #[test]
    fn test_render_empty() {
        assert_eq!(Compose::default().render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_render_empty_with_terminal() {
        let term = Terminal::new_optimistic(80);
        assert_eq!(Compose::default().render(&term), "");
    }

    // =====================================================================
    // Renderable trait — display
    // =====================================================================

    #[test]
    fn test_display_adds_newline() {
        let compose = Compose::from("no newline");
        let term = Terminal::new_optimistic(80);
        let output = compose.display(&term);
        assert!(output.ends_with('\n'));
        assert_eq!(output, "no newline\n");
    }

    #[test]
    fn test_display_does_not_double_newline() {
        let compose = Compose::from("has newline\n");
        let term = Terminal::new_optimistic(80);
        let output = compose.display(&term);
        assert!(output.ends_with('\n'));
        assert!(!output.ends_with("\n\n"));
        assert_eq!(output, "has newline\n");
    }

    #[test]
    fn test_display_empty_produces_newline() {
        let compose = Compose::default();
        let term = Terminal::new_optimistic(80);
        assert_eq!(compose.display(&term), "\n");
    }

    // =====================================================================
    // Renderable trait — is_block_level
    // =====================================================================

    #[test]
    fn test_is_not_block_level() {
        assert!(!Compose::default().is_block_level());
    }

    #[test]
    fn test_is_not_block_level_with_content() {
        let compose = Compose::from("content");
        assert!(!compose.is_block_level());
    }

    // =====================================================================
    // Renderable trait — layout builder methods
    // =====================================================================

    #[test]
    fn test_layout_access() {
        let mut compose = Compose::default();
        compose.layout_mut().alignment = Alignment::Center;
        assert_eq!(compose.layout().alignment, Alignment::Center);
    }

    #[test]
    fn test_left_margin_builder() {
        let compose = Compose::from("test").left_margin(Margin::Chars(4));
        assert_eq!(compose.layout().left_margin, Margin::Chars(4));
    }

    #[test]
    fn test_right_margin_builder() {
        let compose = Compose::from("test").right_margin(Margin::Chars(4));
        assert_eq!(compose.layout().right_margin, Margin::Chars(4));
    }

    #[test]
    fn test_top_margin_builder() {
        let compose = Compose::from("test").top_margin(Margin::Chars(2));
        assert_eq!(compose.layout().top_margin, Margin::Chars(2));
    }

    #[test]
    fn test_bottom_margin_builder() {
        let compose = Compose::from("test").bottom_margin(Margin::Chars(2));
        assert_eq!(compose.layout().bottom_margin, Margin::Chars(2));
    }

    #[test]
    fn test_alignment_builder() {
        let compose = Compose::from("test").alignment(Alignment::Right);
        assert_eq!(compose.layout().alignment, Alignment::Right);
    }

    #[test]
    fn test_row_fill_strategy_builder() {
        let compose = Compose::from("test").row_fill_strategy(RowFill::Fill);
        assert_eq!(compose.layout().row_fill_strategy, RowFill::Fill);
    }

    #[test]
    fn test_word_wrap_builder() {
        let compose = Compose::from("test").word_wrap(WordWrap::None);
        assert_eq!(compose.layout().word_wrap, WordWrap::None);
    }

    #[test]
    fn test_chained_layout_builders() {
        let compose = Compose::from("test")
            .left_margin(Margin::Chars(2))
            .right_margin(Margin::Chars(2))
            .alignment(Alignment::Center);
        assert_eq!(compose.layout().left_margin, Margin::Chars(2));
        assert_eq!(compose.layout().right_margin, Margin::Chars(2));
        assert_eq!(compose.layout().alignment, Alignment::Center);
    }

    // =====================================================================
    // Renderable trait — as_any / Debug
    // =====================================================================

    #[test]
    fn test_as_any_downcast() {
        let compose = Compose::from("test");
        let any_ref = compose.as_any();
        assert!(any_ref.downcast_ref::<Compose>().is_some());
    }

    #[test]
    fn test_as_any_wrong_type() {
        let compose = Compose::from("test");
        let any_ref = compose.as_any();
        assert!(any_ref.downcast_ref::<Prose>().is_none());
    }

    #[test]
    fn test_debug_output() {
        let compose = Compose::from("debug me");
        let debug = format!("{:?}", compose);
        assert!(debug.contains("Compose"));
    }

    #[test]
    fn test_debug_shows_parts() {
        let mut compose = Compose::default();
        compose.add_text("a").add_text("b");
        let debug = format!("{:?}", compose);
        assert!(debug.contains("Compose"));
        assert!(debug.contains("parts"));
    }

    // =====================================================================
    // Edge cases — unicode, emoji, special characters
    // =====================================================================

    #[test]
    fn test_unicode_content() {
        let mut compose = Compose::default();
        compose.add_text("Hello ").add_text("世界");
        assert_eq!(compose.render_optimistic(Some(80)), "Hello 世界");
    }

    #[test]
    fn test_emoji_content() {
        let mut compose = Compose::default();
        compose.add_text("Status: ").add_text("✅");
        assert_eq!(compose.render_optimistic(Some(80)), "Status: ✅");
    }

    #[test]
    fn test_mixed_unicode_scripts() {
        let mut compose = Compose::default();
        compose
            .add_text("English")
            .add_text(" • ")
            .add_text("日本語")
            .add_text(" • ")
            .add_text("العربية");
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("English"));
        assert!(output.contains("日本語"));
        assert!(output.contains("العربية"));
    }

    // =====================================================================
    // Edge cases — empty and whitespace items
    // =====================================================================

    #[test]
    fn test_empty_string_items() {
        let mut compose = Compose::default();
        compose.add_text("").add_text("content").add_text("");
        assert_eq!(compose.render_optimistic(Some(80)), "content");
    }

    #[test]
    fn test_whitespace_only_items() {
        let mut compose = Compose::default();
        compose.add_text("   ").add_text("text").add_text("   ");
        assert_eq!(compose.render_optimistic(Some(80)), "   text   ");
    }

    #[test]
    fn test_single_character() {
        let compose = Compose::from("X");
        assert_eq!(compose.render_optimistic(Some(80)), "X");
    }

    #[test]
    fn test_tab_characters() {
        let mut compose = Compose::default();
        compose.add_text("col1").add_text("\t").add_text("col2");
        assert_eq!(compose.render_optimistic(Some(80)), "col1\tcol2");
    }

    // =====================================================================
    // Edge cases — many items
    // =====================================================================

    #[test]
    fn test_many_items() {
        let mut compose = Compose::default();
        for i in 0..100 {
            compose.add_text(i.to_string());
        }
        let output = compose.render_optimistic(Some(1000));
        assert!(output.starts_with("0123"));
        assert!(output.ends_with("99"));
    }

    // =====================================================================
    // Prose styling integration
    // =====================================================================

    #[test]
    fn test_prose_bold_renders_inline() {
        let mut compose = Compose::default();
        compose
            .add_text("normal ")
            .add_prose(Prose::new("{{bold}}bold{{reset}}"))
            .add_text(" normal");
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("\x1b[1m"));
        assert!(output.contains("bold"));
        assert!(output.starts_with("normal "));
        assert!(output.ends_with(" normal"));
    }

    #[test]
    fn test_multiple_styled_prose() {
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("{{bold}}key{{reset}}"))
            .add_text(": ")
            .add_prose(Prose::new("{{dim}}value{{reset}}"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("key"));
        assert!(output.contains(": "));
        assert!(output.contains("value"));
    }

    // =====================================================================
    // Nesting — component interop
    // =====================================================================

    #[test]
    fn test_compose_inside_compose() {
        let mut inner = Compose::default();
        inner.add_text("inner");
        // Use From<RenderableContent> to wrap inner Compose
        let content = RenderableContent::from(inner);
        let mut outer = Compose::from(content);
        outer.add_text(" after");
        let output = outer.render_optimistic(Some(80));
        assert!(output.contains("inner"));
        assert!(output.contains(" after"));
    }

    #[test]
    fn test_inline_content_inside_compose() {
        let inline = InlineContent::default().with("a").with("b");
        let content = RenderableContent::from(inline);
        let mut compose = Compose::from(content);
        compose.add_text(" end");
        assert_eq!(compose.render_optimistic(Some(80)), "ab end");
    }

    #[test]
    fn test_text_block_via_renderable_content() {
        let block = TextBlock::new("styled");
        let content = RenderableContent::from(block);
        let compose = Compose::from(content);
        assert!(compose.render_optimistic(Some(80)).contains("styled"));
    }
}
