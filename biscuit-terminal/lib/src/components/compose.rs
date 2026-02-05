use std::any::Any;

use crate::components::{
    list::{OrderedList, UnorderedList},
    prose::Prose,
    renderable::{Renderable, RenderableContent},
};
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

/// The **Compose** struct allows you to _compose_
/// 1 or more _renderable_ components together.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::compose::Compose;
/// use biscuit_terminal::components::prose::Prose;
///
/// let mut compose = Compose::new();
/// compose.add_text("Hello, ").add_prose(Prose::new("{{bold}}world{{reset}}!"));
/// ```
#[derive(Debug)]
pub struct Compose {
    parts: Vec<RenderableContent>,
    layout: Layout,
}

impl Default for Compose {
    fn default() -> Self {
        Self::new()
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

impl Renderable for Compose {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.fallback_render(&term)
    }

    fn fallback_render(&self, term: &Terminal) -> String {
        let mut output = String::new();
        for part in &self.parts {
            match part {
                RenderableContent::String(s) => output.push_str(s),
                RenderableContent::Component(c) => output.push_str(&c.fallback_render(term)),
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
    /// Creates a new empty Compose instance.
    pub fn new() -> Self {
        Compose {
            parts: Vec::new(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compose_new() {
        let compose = Compose::new();
        assert_eq!(compose.render(Some(80)), "");
    }

    #[test]
    fn test_compose_from_string() {
        let compose = Compose::from("Hello");
        assert_eq!(compose.render(Some(80)), "Hello");
    }

    #[test]
    fn test_compose_add_text_chaining() {
        let mut compose = Compose::new();
        compose.add_text("Hello, ").add_text("world!");
        assert_eq!(compose.render(Some(80)), "Hello, world!");
    }

    #[test]
    fn test_compose_add_prose() {
        let mut compose = Compose::new();
        compose.add_prose(Prose::new("styled text"));
        let result = compose.render(Some(80));
        assert!(result.contains("styled text"));
    }

    #[test]
    fn test_compose_implements_renderable() {
        let compose = Compose::from("test");
        let term = Terminal::new_optimistic(80);
        let _ = compose.fallback_render(&term);
        let _ = compose.layout();
    }

    #[test]
    fn test_compose_layout_access() {
        let mut compose = Compose::new();
        // Test that layout_mut returns &mut Layout
        compose.layout_mut().alignment = crate::utils::layout::Alignment::Center;
        assert_eq!(
            compose.layout().alignment,
            crate::utils::layout::Alignment::Center
        );
    }
}
