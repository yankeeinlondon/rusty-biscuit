use std::rc::Rc;

use crate::{
    components::renderable::{Renderable, RenderableContent},
    prelude::Prose,
    terminal::Terminal,
    utils::{
        block_constraint::{split_lines, visible_width, wrap_lines},
        layout::{Layout, WordWrap},
    },
};

/// Configure word wrap on an inline component so that wrapped continuation
/// lines align with content after the list prefix (bullet or number).
///
/// Sets the hanging indent only when one is not already configured,
/// preserving any explicit value the caller set on the component.
fn configure_component_wrap(content: &mut RenderableContent, hanging_indent: u32) {
    if let RenderableContent::Component(arc) = content
        && let Some(component) = Rc::get_mut(arc)
        && !component.is_block_level()
    {
        let layout = component.layout_mut();
        layout.word_wrap = layout
            .word_wrap
            .clone()
            .with_hanging_indent_if_none(hanging_indent);
    }
}

/// Force-update the hanging indent on a component, replacing any previously
/// configured value. Used when the bullet changes after initial construction.
fn force_component_hanging_indent(content: &mut RenderableContent, hanging_indent: u32) {
    if let RenderableContent::Component(arc) = content
        && let Some(component) = Rc::get_mut(arc)
        && !component.is_block_level()
    {
        let layout = component.layout_mut();
        layout.word_wrap = layout
            .word_wrap
            .clone()
            .with_hanging_indent(hanging_indent);
    }
}

// =============================================================================
// OrderedList
// =============================================================================

/// An **OrderedList** contains a list of renderable items
/// which will be rendered into an **ordered** (numbered) list.
///
/// OrderedList renders items with numeric prefixes (1., 2., 3., etc.)
/// and handles word-wrapping with proper indentation so continuation
/// lines align with the start of the item text.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::list::OrderedList;
///
/// // Create from a vec of strings
/// let list = OrderedList::new(vec!["First item", "Second item", "Third item"]);
/// // Renders as:
/// // 1. First item
/// // 2. Second item
/// // 3. Third item
///
/// // Build incrementally with add()
/// let mut list = OrderedList::empty();
/// list.add("Install dependencies").add("Run build").add("Deploy");
///
/// // Add with custom indentation for nested content
/// let list = OrderedList::new(vec!["Parent item"])
///     .with_indent_children(8);  // 8 spaces for nested content
/// ```
#[derive(Debug)]
pub struct OrderedList {
    items: Vec<RenderableContent>,
    layout: Layout,
    indent_children: u32,
}

impl Default for OrderedList {
    fn default() -> Self {
        OrderedList {
            items: vec![],
            layout: Layout::default(),
            indent_children: 4,
        }
    }
}

impl<T: Into<String>> From<Vec<T>> for OrderedList {
    fn from(value: Vec<T>) -> Self {
        OrderedList {
            items: value
                .into_iter()
                .map(|f| RenderableContent::String(f.into()))
                .collect(),
            ..OrderedList::default()
        }
    }
}

impl From<Vec<RenderableContent>> for OrderedList {
    fn from(value: Vec<RenderableContent>) -> Self {
        OrderedList {
            items: value,
            ..OrderedList::default()
        }
    }
}

impl From<Vec<&RenderableContent>> for OrderedList {
    fn from(value: Vec<&RenderableContent>) -> Self {
        OrderedList {
            items: value.into_iter().cloned().collect(),
            ..OrderedList::default()
        }
    }
}

impl OrderedList {
    /// Create a new ordered list from items.
    pub fn new<T: Into<String>>(items: Vec<T>) -> Self {
        Self::from(items)
    }

    /// Create an empty ordered list.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add an item that can be converted to RenderableContent.
    ///
    /// Inline components automatically get word wrap configured with
    /// the correct hanging indent so continuation lines align after
    /// the number prefix.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::list::OrderedList;
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// let mut list = OrderedList::empty();
    /// list.add("First item").add("Second item");
    /// ```
    pub fn add<T: Into<RenderableContent>>(&mut self, item: T) -> &mut Self {
        let mut content = item.into();
        // Prefix width depends on the item number: "1. " = 3, "10. " = 4, etc.
        let number = self.items.len() + 1;
        let prefix = format!("{}. ", number);
        let prefix_width = visible_width(&prefix);
        configure_component_wrap(&mut content, prefix_width);
        self.items.push(content);
        self
    }

    /// Set the indentation width for child block-level components.
    pub fn with_indent_children(mut self, indent: u32) -> Self {
        self.indent_children = indent;
        self
    }

    /// Render the list with numbering.
    ///
    /// - **String items** are word-wrapped with the prefix width as
    ///   hanging indent so continuation lines align after the number.
    /// - **Inline components** are rendered at `child_width`; their
    ///   word wrap was configured at add time so they handle their own
    ///   continuation alignment.
    /// - **Block-level components** are rendered without a prefix and
    ///   their output is indented by `indent_children` spaces.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let mut result = String::new();
        let indent = self.indent_children;

        for (i, item) in self.items.iter().enumerate() {
            let number = i + 1;
            let prefix = format!("{}. ", number);
            let prefix_width = visible_width(&prefix);

            match item {
                RenderableContent::String(s) => {
                    result.push_str(&prefix);
                    let child_width = term_width.saturating_sub(prefix_width);
                    let lines = split_lines(s.as_str());
                    let wrapped = wrap_lines(
                        lines,
                        &WordWrap::WrapProse(None, Some(prefix_width)),
                        child_width,
                    );
                    result.push_str(&wrapped.join("\n"));
                }
                RenderableContent::Component(component) if component.is_block_level() => {
                    // Block-level child: no prefix, reduced width, indent output.
                    let child_width = term_width.saturating_sub(indent);
                    let content = term.map_or_else(
                        || component.render_optimistic(Some(child_width)),
                        |t| component.render_in_width(t, child_width),
                    );
                    let indent_str = " ".repeat(indent as usize);
                    for (j, line) in content.lines().enumerate() {
                        if j > 0 {
                            result.push('\n');
                        }
                        result.push_str(&indent_str);
                        result.push_str(line);
                    }
                }
                RenderableContent::Component(component) => {
                    // Inline component: word wrap was configured at add time.
                    result.push_str(&prefix);
                    let child_width = term_width.saturating_sub(prefix_width);
                    let content = term.map_or_else(
                        || component.render_optimistic(Some(child_width)),
                        |t| component.render_in_width(t, child_width),
                    );
                    result.push_str(&content);
                }
            }

            result.push('\n');
        }

        result
    }
}

impl Renderable for OrderedList {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let available = self.layout.available_width(width);
        let content = self.render_content(None, available);
        self.layout.apply_layout(&content, width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_layout(&content, width)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }
}

// =============================================================================
// UnorderedList
// =============================================================================

/// An **UnorderedList** contains a list of renderable items
/// which will be rendered into an **unordered** (bullet-point) list.
///
/// Each item is prefixed with a bullet character (default: `- `) and
/// supports proper word-wrapping with hanging indentation so that
/// continuation lines align with the start of the item text.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::list::UnorderedList;
///
/// // Create from a vec of strings
/// let list = UnorderedList::new(vec!["First item", "Second item", "Third item"]);
/// // Renders as:
/// // - First item
/// // - Second item
/// // - Third item
///
/// // Build incrementally with add()
/// let mut list = UnorderedList::empty();
/// list.add("Install dependencies").add("Run build").add("Deploy");
///
/// // Custom bullet character
/// let list = UnorderedList::new(vec!["Option A", "Option B", "Option C"])
///     .with_bullet("→ ");  // Renders with → instead of -
///
/// // Disable hanging indent for wrapped lines
/// let list = UnorderedList::new(vec!["Long item that wraps"])
///     .without_hanging_indent();
/// ```
///
/// ## Features
///
/// - **Hanging indent**: Continuation lines align after the bullet (enabled by default)
/// - **Custom bullets**: Use any character or string as the bullet marker
/// - **Nested content**: Block-level children are indented without bullets
/// - **Mixed content**: Supports both string items and renderable components
#[derive(Debug)]
pub struct UnorderedList {
    items: Vec<RenderableContent>,
    bullet: String,
    hanging_indent: bool,
    layout: Layout,
    indent_children: Option<u32>,
}

impl Default for UnorderedList {
    fn default() -> Self {
        UnorderedList {
            items: vec![],
            bullet: "- ".to_string(),
            hanging_indent: true,
            layout: Layout::default(),
            indent_children: None,
        }
    }
}

impl<T: Into<String>> From<Vec<T>> for UnorderedList {
    fn from(value: Vec<T>) -> Self {
        UnorderedList {
            items: value
                .into_iter()
                .map(|f| RenderableContent::String(f.into()))
                .collect(),
            ..UnorderedList::default()
        }
    }
}

impl From<Vec<RenderableContent>> for UnorderedList {
    fn from(mut value: Vec<RenderableContent>) -> Self {
        let list = UnorderedList::default();
        if list.hanging_indent {
            let indent = list.indent_children.unwrap_or(visible_width(&list.bullet));
            for item in &mut value {
                configure_component_wrap(item, indent);
            }
        }
        UnorderedList {
            items: value,
            ..list
        }
    }
}

impl From<Vec<&RenderableContent>> for UnorderedList {
    fn from(value: Vec<&RenderableContent>) -> Self {
        let mut items: Vec<RenderableContent> = value.into_iter().cloned().collect();
        let list = UnorderedList::default();
        if list.hanging_indent {
            let indent = list.indent_children.unwrap_or(visible_width(&list.bullet));
            for item in &mut items {
                configure_component_wrap(item, indent);
            }
        }
        UnorderedList { items, ..list }
    }
}

impl From<Prose> for UnorderedList {
    fn from(value: Prose) -> Self {
        let mut list = UnorderedList::default();
        let mut content: RenderableContent = value.into();
        if list.hanging_indent {
            let indent = list.indent_children.unwrap_or(visible_width(&list.bullet));
            configure_component_wrap(&mut content, indent);
        }
        list.items = vec![content];
        list
    }
}

impl From<&Prose> for UnorderedList {
    fn from(value: &Prose) -> Self {
        UnorderedList::from(value.clone())
    }
}

impl UnorderedList {
    /// Create a new unordered list from items.
    pub fn new<T: Into<String>>(items: Vec<T>) -> Self {
        Self::from(items)
    }

    /// Create an empty unordered list.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add an item that can be converted to RenderableContent.
    ///
    /// When `hanging_indent` is enabled (the default), inline components
    /// automatically get word wrap configured with the correct hanging
    /// indent so continuation lines align after the bullet.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::list::UnorderedList;
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// let mut list = UnorderedList::empty();
    /// list.add("First item").add("Second item");
    /// ```
    pub fn add<T: Into<RenderableContent>>(&mut self, item: T) -> &mut Self {
        let mut content = item.into();
        if self.hanging_indent {
            let indent = self.indent_children.unwrap_or(visible_width(&self.bullet));
            configure_component_wrap(&mut content, indent);
        }
        self.items.push(content);
        self
    }

    /// Set a custom bullet character.
    ///
    /// When the new bullet has a different visible width than the current one,
    /// the hanging indent on all inline component items is updated to match
    /// so that continuation lines stay aligned with the start of the item text.
    pub fn with_bullet<T: Into<String>>(mut self, bullet: T) -> Self {
        let new_bullet: String = bullet.into();
        if self.hanging_indent {
            let old_width = visible_width(&self.bullet);
            let new_width = visible_width(&new_bullet);
            if old_width != new_width {
                let new_indent = self.indent_children.unwrap_or(new_width);
                for item in &mut self.items {
                    force_component_hanging_indent(item, new_indent);
                }
            }
        }
        self.bullet = new_bullet;
        self
    }

    /// Set the indentation width for child block-level components.
    ///
    /// When `None` (default), uses the visible width of the bullet.
    pub fn with_indent_children(mut self, indent: Option<u32>) -> Self {
        self.indent_children = indent;
        self
    }

    /// Disable hanging indent on wrapped lines.
    pub fn without_hanging_indent(mut self) -> Self {
        self.hanging_indent = false;
        self
    }

    /// Enable hanging indent on wrapped lines (default).
    pub fn with_hanging_indent(mut self) -> Self {
        self.hanging_indent = true;
        self
    }

    /// Render the list with bullets.
    ///
    /// - **String items** are word-wrapped with the bullet width as
    ///   hanging indent so continuation lines align after the bullet.
    /// - **Inline components** are rendered at `child_width`; their
    ///   word wrap was configured at add time so they handle their own
    ///   continuation alignment.
    /// - **Block-level components** are rendered without a bullet and
    ///   their output is indented by `indent` spaces.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let mut result = String::new();
        let bullet_width = visible_width(&self.bullet);
        let indent = self.indent_children.unwrap_or(bullet_width);

        for item in &self.items {
            match item {
                RenderableContent::String(s) => {
                    result.push_str(&self.bullet);
                    if self.hanging_indent {
                        let child_width = term_width.saturating_sub(bullet_width);
                        let lines = split_lines(s.as_str());
                        let wrapped = wrap_lines(
                            lines,
                            &WordWrap::WrapProse(None, Some(indent)),
                            child_width,
                        );
                        result.push_str(&wrapped.join("\n"));
                    } else {
                        result.push_str(s);
                    }
                }
                RenderableContent::Component(component) if component.is_block_level() => {
                    // Block-level child: no bullet, reduced width, indent output.
                    let child_width = term_width.saturating_sub(indent);
                    let content = term.map_or_else(
                        || component.render_optimistic(Some(child_width)),
                        |t| component.render_in_width(t, child_width),
                    );
                    let indent_str = " ".repeat(indent as usize);
                    for (j, line) in content.lines().enumerate() {
                        if j > 0 {
                            result.push('\n');
                        }
                        result.push_str(&indent_str);
                        result.push_str(line);
                    }
                }
                RenderableContent::Component(component) => {
                    // Inline component: word wrap was configured at add time.
                    result.push_str(&self.bullet);
                    let child_width = term_width.saturating_sub(bullet_width);
                    let content = term.map_or_else(
                        || component.render_optimistic(Some(child_width)),
                        |t| component.render_in_width(t, child_width),
                    );
                    result.push_str(&content);
                }
            }

            result.push('\n');
        }

        result
    }
}

impl Renderable for UnorderedList {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let available = self.layout.available_width(width);
        let content = self.render_content(None, available);
        self.layout.apply_layout(&content, width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_layout(&content, width)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_list_simple() {
        let list = OrderedList::new(vec!["First", "Second", "Third"]);
        let result = list.render_optimistic(None);
        assert_eq!(result, "1. First\n2. Second\n3. Third\n");
    }

    #[test]
    fn test_unordered_list_simple() {
        let list = UnorderedList::new(vec!["Apple", "Banana", "Cherry"]);
        let result = list.render_optimistic(None);
        assert_eq!(result, "- Apple\n- Banana\n- Cherry\n");
    }

    #[test]
    fn test_unordered_list_custom_bullet() {
        let list = UnorderedList::new(vec!["Item 1", "Item 2"]).with_bullet("- ");
        let result = list.render_optimistic(None);
        assert_eq!(result, "- Item 1\n- Item 2\n");
    }

    #[test]
    fn test_empty_ordered_list() {
        let list: OrderedList = OrderedList::new(Vec::<String>::new());
        let result = list.render_optimistic(None);
        assert_eq!(result, "");
    }

    #[test]
    fn test_empty_unordered_list() {
        let list: UnorderedList = UnorderedList::new(Vec::<String>::new());
        let result = list.render_optimistic(None);
        assert_eq!(result, "");
    }

    // =========================================================================
    // Recursive Rendering Tests
    // =========================================================================

    #[test]
    fn test_nested_ordered_list() {
        let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
        let items = vec![
            RenderableContent::String("First".to_string()),
            RenderableContent::Component(Rc::new(inner)),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "1. First\n    1. Nested A\n    2. Nested B\n");
    }

    #[test]
    fn test_three_level_nesting_width_compounds() {
        let inner = OrderedList::new(vec!["Deep"]);
        let middle = OrderedList::from(vec![RenderableContent::Component(Rc::new(inner))]);
        let outer = OrderedList::from(vec![RenderableContent::Component(Rc::new(middle))]);
        let result = outer.render_optimistic(Some(80));
        assert_eq!(result, "        1. Deep\n");
    }

    #[test]
    fn test_mixed_inline_and_block_children_ordered() {
        use crate::components::prose::Prose;

        let inner_list = OrderedList::new(vec!["Sub item"]);
        let prose = Prose::new("Inline text");
        let items = vec![
            RenderableContent::String("Plain string".to_string()),
            RenderableContent::Component(Rc::new(prose)),
            RenderableContent::Component(Rc::new(inner_list)),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "1. Plain string");
        assert_eq!(lines[1], "2. Inline text");
        assert_eq!(lines[2], "    1. Sub item");
    }

    #[test]
    fn test_nested_unordered_list() {
        let inner = UnorderedList::new(vec!["Sub A", "Sub B"]);
        let items = vec![
            RenderableContent::String("Top".to_string()),
            RenderableContent::Component(Rc::new(inner)),
        ];
        let list = UnorderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "- Top\n  - Sub A\n  - Sub B\n");
    }

    #[test]
    fn test_ordered_list_containing_unordered_child() {
        let inner = UnorderedList::new(vec!["Apple", "Banana"]);
        let items = vec![
            RenderableContent::String("Fruits:".to_string()),
            RenderableContent::Component(Rc::new(inner)),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "1. Fruits:\n    - Apple\n    - Banana\n");
    }

    #[test]
    fn test_empty_nested_list() {
        let inner = OrderedList::new(Vec::<String>::new());
        let items = vec![
            RenderableContent::String("Before".to_string()),
            RenderableContent::Component(Rc::new(inner)),
            RenderableContent::String("After".to_string()),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "1. Before\n\n3. After\n");
    }

    #[test]
    fn test_no_output_exceeds_term_width() {
        let inner = OrderedList::new(vec!["Short"]);
        let items = vec![RenderableContent::Component(Rc::new(inner))];
        let list = OrderedList::from(items);
        let width = 40u32;
        let result = list.render_optimistic(Some(width));
        for line in result.lines() {
            let vis = visible_width(line) as u32;
            assert!(
                vis <= width,
                "Line exceeds width {}: {:?} ({})",
                width,
                line,
                vis
            );
        }
    }

    // =========================================================================
    // Hanging Indent Tests
    // =========================================================================

    #[test]
    fn test_unordered_string_wraps_with_hanging_indent() {
        // "- " is 2 chars wide, so with width=20 content gets 18 chars.
        let list = UnorderedList::new(vec!["This is a long item that wraps"]);
        let result = list.render_optimistic(Some(20));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        assert!(lines[0].starts_with("- "));
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Should have 2-space indent: {:?}",
                line
            );
            assert!(
                !line.starts_with("   "),
                "Should not exceed 2-space indent: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_unordered_prose_gets_automatic_wrap() {
        // A Prose with no explicit word wrap gets WrapProse at add time.
        use crate::components::prose::Prose;

        let mut list = UnorderedList::empty();
        list.add(Prose::new(
            "This is a long prose item that should wrap automatically",
        ));
        let result = list.render_optimistic(Some(25));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        assert!(lines[0].starts_with("- "));
        // Continuation lines aligned after bullet (2 spaces)
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Should align after bullet: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_unordered_prose_bespoke_wrap_no_double_indent() {
        // A Prose with BespokeProse(_, _, None) gets hanging indent filled
        // by the list. No double-indentation.
        use crate::components::prose::Prose;

        let prose = Prose::new("aaa, bbb, ccc, ddd, eee, fff, ggg")
            .with_word_wrap(WordWrap::BespokeProse(Some(50), vec![' ', ','], None));
        let mut list = UnorderedList::empty();
        list.add(prose);

        let result = list.render_optimistic(Some(20));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        assert!(lines[0].starts_with("- "));
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Should have 2-space indent: {:?}",
                line
            );
            assert!(
                !line.starts_with("    "),
                "Should NOT have 4-space double indent: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_unordered_no_hanging_indent() {
        let list = UnorderedList::new(vec!["Short"]).without_hanging_indent();
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "- Short\n");
    }

    #[test]
    fn test_prose_explicit_indent_respected() {
        // When a Prose explicitly sets its own hanging indent, the list
        // should not override it.
        use crate::components::prose::Prose;

        let prose = Prose::new("aaa bbb ccc ddd eee fff")
            .with_word_wrap(WordWrap::WrapProse(None, Some(4)));
        let mut list = UnorderedList::empty();
        list.add(prose);

        let result = list.render_optimistic(Some(20));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        // Explicit Some(4) should be preserved, not overwritten to 2
        for line in &lines[1..] {
            assert!(
                line.starts_with("    "),
                "Should have 4-space indent: {:?}",
                line
            );
        }
    }
}
