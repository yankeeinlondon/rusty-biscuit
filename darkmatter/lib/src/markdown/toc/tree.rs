//! Terminal rendering for a Markdown Table of Contents.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use super::types::{MarkdownToc, MarkdownTocNode};

/// A [`MarkdownToc`] prepared for terminal output.
///
/// This is a terminal-only view (ADR-2). It preserves the visual shape of
/// the previous CLI `print_toc_tree` output: document icon, optional bold
/// filename, tree connectors, and verbose metadata.
#[derive(Debug, Clone)]
pub struct TocTree {
    /// Underlying TOC to render.
    toc: MarkdownToc,
    /// Whether to include hashes and summary details.
    verbose: bool,
    /// Optional filename to display next to the document icon.
    filename: Option<String>,
    /// Layout for margins, alignment, and wrapping.
    layout: Layout,
}

impl TocTree {
    /// Creates a terminal view of the given TOC.
    pub fn new(toc: MarkdownToc) -> Self {
        Self {
            toc,
            verbose: false,
            filename: None,
            layout: Layout::default(),
        }
    }

    /// Enables verbose metadata output.
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Sets the filename shown next to the document icon.
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }
}

impl TerminalRenderable for TocTree {
    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn render(&self, term: &Terminal) -> String {
        let mut out = String::new();

        // Breathing room before the TOC.
        out.push('\n');

        if self.toc.title.is_some() {
            if let Some(name) = &self.filename {
                out.push_str(&format!("📄 {}\n", Prose::new(format!("<bold>{name}</bold>")).render(term)));
            } else {
                out.push_str("📄\n");
            }

            if self.verbose {
                out.push_str(&format!(
                    "   Page hash: {:016x} (trimmed: {:016x})\n",
                    self.toc.page_hash, self.toc.page_hash_trimmed
                ));
            }
        }

        // Tree structure.
        for (i, node) in self.toc.structure.iter().enumerate() {
            let is_last = i == self.toc.structure.len() - 1;
            render_node(&mut out, term, node, "", is_last, self.verbose);
        }

        // Breathing room after the TOC.
        out.push('\n');

        if self.verbose {
            out.push_str(&format!(
                "Total: {} heading{}\n",
                self.toc.heading_count(),
                if self.toc.heading_count() == 1 { "" } else { "s" }
            ));

            if !self.toc.code_blocks.is_empty() {
                out.push_str(&format!("Code blocks: {}\n", self.toc.code_blocks.len()));
            }

            if !self.toc.internal_links.is_empty() {
                let broken_count = self.toc.broken_links().len();
                if broken_count > 0 {
                    out.push_str(&format!(
                        "Internal links: {} ({} broken)\n",
                        self.toc.internal_links.len(),
                        broken_count
                    ));
                } else {
                    out.push_str(&format!("Internal links: {}\n", self.toc.internal_links.len()));
                }
            }
        }

        out
    }
}

fn render_node(
    out: &mut String,
    term: &Terminal,
    node: &MarkdownTocNode,
    prefix: &str,
    is_last: bool,
    verbose: bool,
) {
    let connector = if is_last { "└── " } else { "├── " };
    let child_prefix = if is_last { "    " } else { "│   " };

    if verbose {
        out.push_str(&format!(
            "{}{}{} ({:016x})\n",
            prefix,
            connector,
            Prose::new(&node.title).render(term),
            node.prelude_hash_normalized()
        ));
    } else {
        out.push_str(&format!(
            "{}{}{}\n",
            prefix,
            connector,
            Prose::new(&node.title).render(term)
        ));
    }

    let new_prefix = format!("{}{}", prefix, child_prefix);
    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == node.children.len() - 1;
        render_node(out, term, child, &new_prefix, child_is_last, verbose);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::normalize::HeadingLevel;

    fn node(title: &str, level: HeadingLevel, children: Vec<MarkdownTocNode>) -> MarkdownTocNode {
        let mut node = MarkdownTocNode::new(
            level,
            title.to_string(),
            title.to_lowercase().replace(' ', "-"),
            (0, 0),
            (1, 2),
        );
        node.children = children;
        node
    }

    fn toc_with_nodes(nodes: Vec<MarkdownTocNode>) -> MarkdownToc {
        let mut toc = MarkdownToc::new();
        toc.title = Some("Doc".to_string());
        toc.structure = nodes;
        toc
    }

    #[test]
    fn toc_tree_renders_document_icon_and_headings() {
        let toc = toc_with_nodes(vec![
            node("One", HeadingLevel::H1, vec![node("Two", HeadingLevel::H2, vec![])]),
        ]);
        let rendered = TocTree::new(toc).render(&Terminal::new_optimistic(80));

        assert!(rendered.starts_with('\n'));
        assert!(rendered.contains("📄"));
        assert!(rendered.contains("└── One"));
        assert!(rendered.contains("└── Two"));
        assert!(rendered.ends_with('\n'));
    }

    #[test]
    fn toc_tree_empty_structure_renders_icon_only() {
        let toc = toc_with_nodes(vec![]);
        let rendered = TocTree::new(toc).render(&Terminal::new_optimistic(80));

        assert!(rendered.contains("📄"));
        assert!(!rendered.contains("├──"));
    }

    #[test]
    fn toc_tree_verbose_includes_summary_and_hashes() {
        let toc = toc_with_nodes(vec![node("One", HeadingLevel::H1, vec![])]);
        let rendered = TocTree::new(toc)
            .verbose()
            .render(&Terminal::new_optimistic(80));

        assert!(rendered.contains("Total: 1 heading"));
        assert!(rendered.contains("Page hash:"));
    }

    #[test]
    fn toc_tree_with_filename_renders_bold_name() {
        let toc = toc_with_nodes(vec![node("One", HeadingLevel::H1, vec![])]);
        let rendered = TocTree::new(toc)
            .with_filename("README.md")
            .render(&Terminal::new_forced());

        assert!(rendered.contains("README.md"));
    }
}
