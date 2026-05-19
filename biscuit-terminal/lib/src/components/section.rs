use renderable::style::{Style, TextEmphasis};
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

    /// The declared heading [`Style`] for this level.
    ///
    /// Replaces the hard-coded heading SGR: levels h1-h3 declare bold
    /// emphasis, h4-h5 declare italic, and h6 declares no emphasis. The
    /// renderers lower the [`TextEmphasis`] to the target instead of
    /// hand-writing escape codes.
    pub fn heading_style(&self) -> Style {
        Style {
            emphasis: heading_emphasis(self.level()),
            ..Style::default()
        }
    }
}

/// The [`TextEmphasis`] declared for a heading depth (1-6).
///
/// Depths 1-3 are bold, 4-5 italic, and 6 carries no emphasis.
pub(crate) fn heading_emphasis(depth: u8) -> TextEmphasis {
    match depth {
        1..=3 => TextEmphasis {
            bold: true,
            ..TextEmphasis::default()
        },
        4 | 5 => TextEmphasis {
            italic: true,
            ..TextEmphasis::default()
        },
        _ => TextEmphasis::default(),
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
    /// section.push(Prose::new("<bold>Styled</bold> text"));
    /// ```
    pub fn push<T: Into<RenderableTerminalContent>>(&mut self, item: T) -> &mut Self {
        self.content.push(item.into());
        self
    }

    /// Render the section with heading styling based on level.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let mut result = String::new();

        // The Markdown-style prefix per level.
        let prefix = match self.level {
            HeadingLevel::h1 => "# ",
            HeadingLevel::h2 => "## ",
            HeadingLevel::h3 => "### ",
            HeadingLevel::h4 => "#### ",
            HeadingLevel::h5 => "##### ",
            HeadingLevel::h6 => "###### ",
        };

        // The heading is rendered by lowering the level's declared `Style`
        // through the shared terminal style applier — the same path the tree
        // renderer uses — rather than hand-splicing SGR escapes. A `Terminal`
        // is required; the optimistic-width path manufactures one.
        let owned_term;
        let term_ref = match term {
            Some(t) => t,
            None => {
                owned_term = Terminal::new_optimistic(term_width);
                &owned_term
            }
        };
        let styled_heading = crate::render_tree::style::apply_style(
            &format!("{prefix}{}", self.title),
            &self.level.heading_style(),
            term_ref,
            term_width,
        );

        // Render the heading
        result.push_str(&styled_heading);
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

        let mut node = RenderNode::section(depth, heading, children);
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h1_section() {
        let section = Section::new(HeadingLevel::h1, "Title");
        let result = section.render_optimistic(None);
        // The heading lowers the declared bold `Style`: a `\x1b[1m` open run
        // and a single `\x1b[0m` reset, matching the shared style applier.
        assert_eq!(result, "\x1b[1m# Title\x1b[0m");
    }

    #[test]
    fn test_section_with_content() {
        let mut section = Section::new(HeadingLevel::h2, "Header");
        section.add_string("Some content here.");
        let result = section.render_optimistic(None);
        assert_eq!(result, "\x1b[1m## Header\x1b[0m\nSome content here.");
    }

    #[test]
    fn test_heading_levels() {
        assert_eq!(HeadingLevel::h1.level(), 1);
        assert_eq!(HeadingLevel::h6.level(), 6);
    }

    #[test]
    fn section_render_tree_node_carries_layout_when_margins_set() {
        use crate::utils::layout::{Length, Margin};
        let mut section = Section::new(HeadingLevel::h1, "Title");
        section.layout.margin = Margin::x(Length::ch(2));
        let node = section.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }
}
