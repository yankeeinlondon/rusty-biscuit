use renderable::tree::{HeadingDepth, RenderNode};

use crate::{
    components::renderable::{RenderableTerminalContent, TerminalRenderable},
    render_tree::projection::TreeProjectionContext,
    terminal::Terminal,
    utils::layout::{Layout, LayoutTerminalExt},
};

/// Heading level for sections, from h1 (largest) to h6 (smallest).
///
/// Levels h1-h3 use bold styling, h4-h5 use italic, and h6 renders as plain text.
/// The level also determines the Markdown-style prefix: `# ` for h1, `## ` for h2, etc.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::section::{Section, HeadingLevel};
///
/// // Create sections at different levels
/// let h1 = Section::new(HeadingLevel::h1, "Title");
/// let h2 = Section::new(HeadingLevel::h2, "Section");
/// let h3 = Section::new(HeadingLevel::h3, "Subsection");
/// let h6 = Section::new(HeadingLevel::h6, "Minor heading");
///
/// // Get numeric level
/// assert_eq!(HeadingLevel::h2.level(), 2);
/// ```
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
///
/// Sections render a Markdown-style heading followed by arbitrary content.
/// The heading level (h1-h6) controls both the visual styling and the
/// Markdown prefix (`#`, `##`, etc.).
///
/// ## Heading Levels
///
/// - Levels h1-h3: Bold styling
/// - Levels h4-h5: Italic styling
/// - Level h6: Plain text (no styling)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::section::{Section, HeadingLevel};
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// // Create a section with heading and content
/// let mut section = Section::new(HeadingLevel::h2, "Getting Started");
/// section
///     .push("Welcome to the tutorial.")
///     .push("Let's begin with installation.");
///
/// // Render to terminal string
/// let output = section.render_optimistic(Some(80));
/// assert!(output.contains("## Getting Started"));
/// ```
///
/// ## Notes
///
/// Content can be strings, [`Prose`][crate::components::prose::Prose],
/// or any type implementing [`Into<RenderableTerminalContent>`][crate::components::renderable::RenderableTerminalContent].
#[derive(Debug)]
pub struct Section {
    level: HeadingLevel,
    title: String,
    content: Vec<RenderableTerminalContent>,
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
    pub fn with_content(mut self, content: Vec<RenderableTerminalContent>) -> Self {
        self.content = content;
        self
    }

    /// Add a string item to the content.
    pub fn add_string<T: Into<String>>(&mut self, s: T) {
        self.content
            .push(RenderableTerminalContent::String(s.into()));
    }

    /// Add any content that can be converted to RenderableTerminalContent.
    ///
    /// This is a convenience method that accepts strings, Prose, and other
    /// renderable components without requiring manual wrapping.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::section::{Section, HeadingLevel};
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// let mut section = Section::new(HeadingLevel::h2, "My Section");
    /// section.push("Plain text");
    /// section.push(Prose::new("{{bold}}Styled{{reset}} text"));
    /// ```
    pub fn push<T: Into<RenderableTerminalContent>>(&mut self, item: T) -> &mut Self {
        self.content.push(item.into());
        self
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
                RenderableTerminalContent::String(s) => s.clone(),
                RenderableTerminalContent::Component(component) => {
                    if let Some(t) = term {
                        component.render_in_width(t, term_width)
                    } else {
                        component.render_optimistic(Some(term_width))
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

impl TerminalRenderable for Section {
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

    /// Projects this section into a [`NodeKind::Section`](renderable::tree::NodeKind::Section)
    /// render-tree node.
    ///
    /// The heading level maps to a [`HeadingDepth`]; the title becomes a single
    /// [`RenderNode::text`] in the heading's phrasing content; each content item
    /// is projected via [`to_tree_nodes`](RenderableTerminalContent::to_tree_nodes)
    /// and collected into the section body.
    ///
    /// ## Notes
    ///
    /// Projection diagnostics are discarded here because `render_tree_node`
    /// returns `Option<RenderNode>` and cannot carry them.
    fn render_tree_node(&self) -> Option<RenderNode> {
        // HeadingLevel::level() always returns 1..=6, which HeadingDepth accepts.
        let depth = HeadingDepth::new(self.level.level()).ok()?;
        let heading = vec![RenderNode::text(&self.title)];

        let mut children = Vec::new();
        for item in &self.content {
            let mut ctx = TreeProjectionContext::default();
            let result = item.to_tree_nodes(&mut ctx);
            children.extend(result.nodes);
        }

        Some(RenderNode::section(depth, heading, children))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h1_section() {
        let section = Section::new(HeadingLevel::h1, "Title");
        let result = section.render_optimistic(None);
        assert_eq!(result, "\x1b[1m# Title\x1b[22m");
    }

    #[test]
    fn test_section_with_content() {
        let mut section = Section::new(HeadingLevel::h2, "Header");
        section.add_string("Some content here.");
        let result = section.render_optimistic(None);
        assert_eq!(result, "\x1b[1m## Header\x1b[22m\nSome content here.");
    }

    #[test]
    fn test_heading_levels() {
        assert_eq!(HeadingLevel::h1.level(), 1);
        assert_eq!(HeadingLevel::h6.level(), 6);
    }
}
