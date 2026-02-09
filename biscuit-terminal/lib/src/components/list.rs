use crate::{
    components::renderable::{Renderable, RenderableContent}, prelude::Prose, terminal::Terminal, utils::layout::Layout
};

/// An **OrderedList** contains a list of renderable items
/// which will be rendered into an **ordered** list.
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
    /// This is a convenience method for building lists incrementally.
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
        self.items.push(item.into());
        self
    }

    /// Set the indentation width for child block-level components.
    pub fn with_indent_children(mut self, indent: u32) -> Self {
        self.indent_children = indent;
        self
    }

    /// Render the list with numbering.
    ///
    /// Block-level child components are rendered without a number
    /// prefix and their output is indented by `indent_children`
    /// spaces. Inline items and strings get the normal `"N. "` prefix.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let mut result = String::new();
        let indent = self.indent_children;

        for (i, item) in self.items.iter().enumerate() {
            let number = i + 1;
            let prefix = format!("{}. ", number);

            match item {
                RenderableContent::String(s) => {
                    result.push_str(&prefix);
                    result.push_str(s);
                }
                RenderableContent::Component(component) if component.is_block_level() => {
                    // Block-level child: no prefix, reduced width, indent output
                    let child_width = term_width.saturating_sub(indent);
                    let content = if let Some(t) = term {
                        component.fallback_render(t)
                    } else {
                        component.render(Some(child_width))
                    };
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
                    // Inline component: normal prefix
                    result.push_str(&prefix);
                    let content = if let Some(t) = term {
                        component.fallback_render(t)
                    } else {
                        component.render(Some(term_width))
                    };
                    result.push_str(&content);
                }
            }

            result.push('\n');
        }

        result
    }
}

impl Renderable for OrderedList {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let available = self.layout.available_width(width);
        let content = self.render_content(None, available);
        self.layout.apply_layout(&content, width)
    }

    fn fallback_render(&self, term: &Terminal) -> String {
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

/// An **UnorderedList** contains a list of renderable items
/// which will be rendered into an **unordered** list, a bullet
/// point will precede each line.
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
            bullet: "• ".to_string(),
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
    fn from(value: Vec<RenderableContent>) -> Self {
        UnorderedList {
            items: value,
            ..UnorderedList::default()
        }
    }
}

impl From<Vec<&RenderableContent>> for UnorderedList {
    fn from(value: Vec<&RenderableContent>) -> Self {
        UnorderedList {
            items: value.into_iter().cloned().collect(),
            ..UnorderedList::default()
        }
    }
}

impl From<Prose> for UnorderedList {
    fn from(value: Prose) -> Self {
        UnorderedList {
            items: vec![value.into()],
            ..UnorderedList::default()
        }
    }
}

impl From<&Prose> for UnorderedList {
    fn from(value: &Prose) -> Self {
        UnorderedList {
            items: vec![value.clone().into()],
            ..UnorderedList::default()
        }
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
    /// This is a convenience method for building lists incrementally.
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
        self.items.push(item.into());
        self
    }

    /// Set a custom bullet character.
    pub fn with_bullet<T: Into<String>>(mut self, bullet: T) -> Self {
        self.bullet = bullet.into();
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
    /// Block-level child components are rendered without a bullet
    /// and their output is indented. Inline items and strings get
    /// the normal bullet prefix.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let mut result = String::new();
        let bullet_width = crate::utils::block_constraint::visible_width(&self.bullet) as u32;
        let indent = self.indent_children.unwrap_or(bullet_width);

        for item in &self.items {
            match item {
                RenderableContent::String(s) => {
                    result.push_str(&self.bullet);
                    result.push_str(s);
                }
                RenderableContent::Component(component) if component.is_block_level() => {
                    // Block-level child: no bullet, reduced width, indent output
                    let child_width = term_width.saturating_sub(indent);
                    let content = if let Some(t) = term {
                        component.fallback_render(t)
                    } else {
                        component.render(Some(child_width))
                    };
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
                    // Inline component: normal bullet
                    result.push_str(&self.bullet);
                    let content = if let Some(t) = term {
                        component.fallback_render(t)
                    } else {
                        component.render(Some(term_width))
                    };
                    result.push_str(&content);
                }
            }

            result.push('\n');
        }

        result
    }
}

impl Renderable for UnorderedList {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let available = self.layout.available_width(width);
        let content = self.render_content(None, available);
        self.layout.apply_layout(&content, width)
    }

    fn fallback_render(&self, term: &Terminal) -> String {
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
    use std::sync::Arc;

    #[test]
    fn test_ordered_list_simple() {
        let list = OrderedList::new(vec!["First", "Second", "Third"]);
        let result = list.render(None);
        assert_eq!(result, "1. First\n2. Second\n3. Third\n");
    }

    #[test]
    fn test_unordered_list_simple() {
        let list = UnorderedList::new(vec!["Apple", "Banana", "Cherry"]);
        let result = list.render(None);
        assert_eq!(result, "• Apple\n• Banana\n• Cherry\n");
    }

    #[test]
    fn test_unordered_list_custom_bullet() {
        let list = UnorderedList::new(vec!["Item 1", "Item 2"]).with_bullet("- ");
        let result = list.render(None);
        assert_eq!(result, "- Item 1\n- Item 2\n");
    }

    #[test]
    fn test_empty_ordered_list() {
        let list: OrderedList = OrderedList::new(Vec::<String>::new());
        let result = list.render(None);
        assert_eq!(result, "");
    }

    #[test]
    fn test_empty_unordered_list() {
        let list: UnorderedList = UnorderedList::new(Vec::<String>::new());
        let result = list.render(None);
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
            RenderableContent::Component(Arc::new(inner)),
        ];
        let list = OrderedList::from(items);
        let result = list.render(Some(80));
        // "First" gets prefix "1. First"
        // The nested OrderedList is block-level, so it gets indented (4 spaces)
        assert_eq!(result, "1. First\n    1. Nested A\n    2. Nested B\n");
    }

    #[test]
    fn test_three_level_nesting_width_compounds() {
        // Inner-most list
        let inner = OrderedList::new(vec!["Deep"]);
        // Middle list containing the inner
        let middle = OrderedList::from(vec![RenderableContent::Component(Arc::new(inner))]);
        // Outer list containing middle
        let outer = OrderedList::from(vec![RenderableContent::Component(Arc::new(middle))]);

        let result = outer.render(Some(80));
        // outer indents middle by 4, middle indents inner by 4 = 8 total
        assert_eq!(result, "        1. Deep\n");
    }

    #[test]
    fn test_mixed_inline_and_block_children_ordered() {
        use crate::components::prose::Prose;

        let inner_list = OrderedList::new(vec!["Sub item"]);
        let prose = Prose::new("Inline text");
        let items = vec![
            RenderableContent::String("Plain string".to_string()),
            RenderableContent::Component(Arc::new(prose)), // inline (is_block_level = false)
            RenderableContent::Component(Arc::new(inner_list)), // block-level
        ];
        let list = OrderedList::from(items);
        let result = list.render(Some(80));

        let lines: Vec<&str> = result.lines().collect();
        // String: "1. Plain string"
        assert_eq!(lines[0], "1. Plain string");
        // Prose (inline): "2. Inline text"
        assert_eq!(lines[1], "2. Inline text");
        // Block-level OrderedList: indented, no prefix
        assert_eq!(lines[2], "    1. Sub item");
    }

    #[test]
    fn test_nested_unordered_list() {
        let inner = UnorderedList::new(vec!["Sub A", "Sub B"]);
        let items = vec![
            RenderableContent::String("Top".to_string()),
            RenderableContent::Component(Arc::new(inner)),
        ];
        let list = UnorderedList::from(items);
        let result = list.render(Some(80));

        // "Top" gets bullet "• Top"
        // The nested UnorderedList is block-level, indented by bullet width (2 for "• ")
        assert_eq!(result, "• Top\n  • Sub A\n  • Sub B\n");
    }

    #[test]
    fn test_ordered_list_containing_unordered_child() {
        let inner = UnorderedList::new(vec!["Apple", "Banana"]);
        let items = vec![
            RenderableContent::String("Fruits:".to_string()),
            RenderableContent::Component(Arc::new(inner)),
        ];
        let list = OrderedList::from(items);
        let result = list.render(Some(80));
        assert_eq!(result, "1. Fruits:\n    • Apple\n    • Banana\n");
    }

    #[test]
    fn test_empty_nested_list() {
        let inner = OrderedList::new(Vec::<String>::new());
        let items = vec![
            RenderableContent::String("Before".to_string()),
            RenderableContent::Component(Arc::new(inner)),
            RenderableContent::String("After".to_string()),
        ];
        let list = OrderedList::from(items);
        let result = list.render(Some(80));
        // Empty nested list renders as empty string with no lines to indent
        // The separator newlines still appear between items
        assert_eq!(result, "1. Before\n\n3. After\n");
    }

    #[test]
    fn test_no_output_exceeds_term_width() {
        let inner = OrderedList::new(vec!["Short"]);
        let items = vec![RenderableContent::Component(Arc::new(inner))];
        let list = OrderedList::from(items);
        let width = 40u32;
        let result = list.render(Some(width));

        for line in result.lines() {
            let visible = crate::utils::block_constraint::visible_width(line) as u32;
            assert!(
                visible <= width,
                "Line exceeds term_width {}: \"{}\" (visible width: {})",
                width,
                line,
                visible
            );
        }
    }
}
