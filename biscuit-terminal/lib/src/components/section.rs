use crate::{
    components::renderable::{Renderable, RenderableContent},
    terminal::Terminal,
    utils::layout::Layout,
};

/// Heading level for sections, from h1 (largest) to h6 (smallest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum HeadingLevel {
    h1,
    h2,
    h3,
    h4,
    h5,
    h6,
}

impl HeadingLevel {
    /// Get the numeric level (1-6).
    pub fn level(&self) -> u8 {
        match self {
            HeadingLevel::h1 => 1,
            HeadingLevel::h2 => 2,
            HeadingLevel::h3 => 3,
            HeadingLevel::h4 => 4,
            HeadingLevel::h5 => 5,
            HeadingLevel::h6 => 6,
        }
    }
}

/// A section with a heading and content.
#[derive(Debug)]
pub struct Section {
    level: HeadingLevel,
    title: String,
    content: Vec<RenderableContent>,
    layout: Layout,
}

impl Section {
    /// Create a new section with a heading level and title.
    pub fn new<T: Into<String>>(level: HeadingLevel, title: T) -> Self {
        Section {
            level,
            title: title.into(),
            content: Vec::new(),
            layout: Layout::default(),
        }
    }

    /// Add content to the section.
    pub fn with_content(mut self, content: Vec<RenderableContent>) -> Self {
        self.content = content;
        self
    }

    /// Add a string item to the content.
    pub fn add_string<T: Into<String>>(&mut self, s: T) {
        self.content.push(RenderableContent::String(s.into()));
    }

    /// Render the section with heading styling based on level.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let mut result = String::new();

        // Apply heading style based on level
        let (prefix, style_open, style_close) = match self.level {
            HeadingLevel::h1 => ("# ", "\x1b[1m", "\x1b[22m"), // Bold
            HeadingLevel::h2 => ("## ", "\x1b[1m", "\x1b[22m"), // Bold
            HeadingLevel::h3 => ("### ", "\x1b[1m", "\x1b[22m"), // Bold
            HeadingLevel::h4 => ("#### ", "\x1b[3m", "\x1b[23m"), // Italic
            HeadingLevel::h5 => ("##### ", "\x1b[3m", "\x1b[23m"), // Italic
            HeadingLevel::h6 => ("###### ", "", ""),           // Plain
        };

        // Render the heading
        result.push_str(style_open);
        result.push_str(prefix);
        result.push_str(&self.title);
        result.push_str(style_close);
        result.push('\n');

        // Render content
        for item in &self.content {
            let content_str = match item {
                RenderableContent::String(s) => s.clone(),
                RenderableContent::Component(component) => {
                    if let Some(t) = term {
                        component.fallback_render(t)
                    } else {
                        component.render(Some(term_width))
                    }
                }
            };
            result.push_str(&content_str);
            result.push('\n');
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }
}

impl Renderable for Section {
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

    #[test]
    fn test_h1_section() {
        let section = Section::new(HeadingLevel::h1, "Title");
        let result = section.render(None);
        assert_eq!(result, "\x1b[1m# Title\x1b[22m");
    }

    #[test]
    fn test_section_with_content() {
        let mut section = Section::new(HeadingLevel::h2, "Header");
        section.add_string("Some content here.");
        let result = section.render(None);
        assert_eq!(result, "\x1b[1m## Header\x1b[22m\nSome content here.");
    }

    #[test]
    fn test_heading_levels() {
        assert_eq!(HeadingLevel::h1.level(), 1);
        assert_eq!(HeadingLevel::h6.level(), 6);
    }
}
