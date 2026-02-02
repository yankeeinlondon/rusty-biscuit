use crate::{
    components::renderable::{Renderable, RenderableContent},
    terminal::Terminal,
    utils::layout::{Layout, WordWrap},
};


/// An **OrderedList** contains a list of renderable items
/// which will be rendered into an **ordered** list.
#[derive(Debug)]
pub struct OrderedList {
    items: Vec<RenderableContent>,
}

impl Default for OrderedList {
    fn default() -> Self {
        OrderedList {
          items: vec![]
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
        OrderedList { items: value }
    }
}

impl From<Vec<&RenderableContent>> for OrderedList {
    fn from(value: Vec<&RenderableContent>) -> Self {
        OrderedList {
            items: value.into_iter().cloned().collect(),
        }
    }
}

impl OrderedList {
    /// Create a new ordered list from items.
    pub fn new<T: Into<String>>(items: Vec<T>) -> Self {
        Self::from(items)
    }

    /// Render the list with numbering.
    fn render_content(&self, term: Option<&Terminal>, layout: Option<&Layout>) -> String {
        let mut result = String::new();

        for (i, item) in self.items.iter().enumerate() {
            let number = i + 1;
            let prefix = format!("{}. ", number);
            result.push_str(&prefix);

            let content = match item {
                RenderableContent::String(s) => s.clone(),
                RenderableContent::Component(component) => {
                    if let Some(t) = term {
                        component.fallback_render(t, layout)
                    } else {
                        component.render(layout)
                    }
                }
            };
            result.push_str(&content);

            if i < self.items.len() - 1 {
                result.push('\n');
            }
        }

        result
    }
}

impl Renderable for OrderedList {
    fn render(&self, layout: Option<&Layout>) -> String {
        self.render_content(None, layout)
    }

    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String {
        self.render_content(Some(term), layout)
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
}

impl Default for UnorderedList {
    fn default() -> Self {
        UnorderedList { items: vec![], bullet: "• ".to_string(), hanging_indent: true }
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

impl UnorderedList {
    /// Create a new unordered list from items.
    pub fn new<T: Into<String>>(items: Vec<T>) -> Self {
        Self::from(items)
    }

    /// Set a custom bullet character.
    pub fn with_bullet<T: Into<String>>(mut self, bullet: T) -> Self {
        self.bullet = bullet.into();
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
    fn render_content(&self, term: Option<&Terminal>, layout: Option<&Layout>) -> String {
        let mut result = String::new();
        let bullet_width = crate::utils::block_constraint::visible_width(&self.bullet) as u32;
        let mut item_layout = match layout {
            Some(l) => l.clone(),
            None => Layout::default(),
        };
        if self.hanging_indent {
            item_layout.word_wrap = WordWrap::WrapProse(Some(8), Some(bullet_width));
        }


        for (i, item) in self.items.iter().enumerate() {
            result.push_str(&self.bullet);

            let content = match item {
                RenderableContent::String(s) => s.clone(),
                RenderableContent::Component(component) => {
                    if let Some(t) = term {
                        component.fallback_render(t, Some(&item_layout))
                    } else {
                        component.render(Some(&item_layout))
                    }
                }
            };
            result.push_str(&content);

            if i < self.items.len() - 1 {
                result.push('\n');
            }
        }

        result
    }
}

impl Renderable for UnorderedList {
    fn render(&self, layout: Option<&Layout>) -> String {
        self.render_content(None, layout)
    }

    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String {
        self.render_content(Some(term), layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_list_simple() {
        let list = OrderedList::new(vec!["First", "Second", "Third"]);
        let result = list.render(None);
        assert_eq!(result, "1. First\n2. Second\n3. Third");
    }

    #[test]
    fn test_unordered_list_simple() {
        let list = UnorderedList::new(vec!["Apple", "Banana", "Cherry"]);
        let result = list.render(None);
        assert_eq!(result, "• Apple\n• Banana\n• Cherry");
    }

    #[test]
    fn test_unordered_list_custom_bullet() {
        let list = UnorderedList::new(vec!["Item 1", "Item 2"]).with_bullet("- ");
        let result = list.render(None);
        assert_eq!(result, "- Item 1\n- Item 2");
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
}
