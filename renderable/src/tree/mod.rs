//! The canonical owned render tree.
//!
//! A [`Document`] holds a [`RenderNode`] tree alongside a [`SourceRegistry`]
//! of origins and document [`DocumentMetadata`]. The tree is a single
//! canonical representation that sits between content sources and render
//! targets.
//!
//! ## Examples
//!
//! ```
//! use renderable::tree::{HeadingDepth, RenderNode};
//!
//! let tree = RenderNode::root(vec![
//!     RenderNode::heading(
//!         HeadingDepth::new(1).unwrap(),
//!         vec![RenderNode::text("Title")],
//!     ),
//!     RenderNode::paragraph(vec![RenderNode::text("Body text.")]),
//! ]);
//!
//! assert_eq!(tree.children().len(), 2);
//! ```

mod attrs;
mod diagnostic;
mod document;
pub mod embed;
mod error;
pub mod graphics;
mod inherit;
mod node;
pub mod render;
mod source;
mod validate;

pub use attrs::{
    AriaAttrName, BrowserAttrNameError, BrowserAttrs, CodeRenderHints, ColumnConditional,
    ColumnWidthKind, ColumnsHints, ComponentHints, DataAttrName, DisclosureStyleHints,
    HintNamespace, HrAlignment, HrKind, HrWeight, ImageBrowserAttrs, ImageDecoding, ImageLoading,
    LinkBrowserAttrs, LinkRelation, LinkTarget, ListMarkerPolicy, ListRenderHints, NodeAttrs,
    ProgressHints, SequenceJoin, TableCellHints, TableColumnHints, TableHints, TableTerminalHints,
    TaskHints, TaskState, TextLayoutHints, TextOverflow, ThematicBreakAttrs,
};
// Test-only instrumentation (gated): exposed so a downstream crate's perf-gate
// test can observe the `NodeAttrs::data` hint-access counter while folding a
// corpus through renderable. See the `hint-access-counter` feature.
#[cfg(any(test, feature = "hint-access-counter"))]
pub use attrs::{hint_accesses, reset_hint_accesses};
pub use diagnostic::{Diagnostic, DiagnosticKind, Severity};
pub use embed::{
    EMBED_CLOSE, EMBED_MARKER_SUFFIX, EMBED_OPEN_PREFIX, EmbedError, decode_embedded_open,
    encode_embedded_subtree, is_embedded_close,
};
pub use inherit::InheritedStyle;
pub use document::{Document, DocumentMetadata, Frontmatter, FrontmatterFormat};
pub use error::{RenderError, RenderStrictness, Rendered};
pub use graphics::horizontal_rule_svg;
pub use node::{ColumnAlign, HeadingDepth, HeadingDepthError, NodeKind, RenderNode};
pub use render::{
    BrowserDocumentBody, BrowserRenderOptions, CodeRenderer, MarkdownDialect,
    MarkdownRenderOptions, MarkdownStyleOptions, RawHtmlPolicy, render_browser_document,
    render_browser_document_body, render_browser_document_html, render_browser_node,
    render_markdown_document, render_markdown_node,
};

/// The graphics fidelity tier a renderer should use.
///
/// The [`Default`] is [`GraphicsMode::Rich`] — the highest tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphicsMode {
    /// No graphics: render text or structural fallbacks only.
    Off,
    /// Vector graphics only: SVG, box-drawing, and other scalable forms.
    Vector,
    /// Full graphics: raster images, interactive diagrams, and all rich media.
    #[default]
    Rich,
}

/// How Mermaid diagrams are rendered in browser output.
///
/// The [`Default`] is [`BrowserMermaidMode::Code`] — a plain fenced code block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrowserMermaidMode {
    /// Render as a plain `<pre><code>` block.
    #[default]
    Code,
    /// Render as a static inline `<svg>`.
    StaticSvg,
    /// Render as an interactive Mermaid diagram (e.g. with pan/zoom).
    Interactive,
}

/// How Mermaid diagrams are rendered in terminal output.
///
/// [`GraphicsMode`] is the fidelity *ceiling*; this is the orthogonal *opt-in*
/// that decides whether a `lang="mermaid"` code block is promoted at all.
/// The [`Default`] is [`TerminalMermaidMode::Code`] — Mermaid fences stay
/// ordinary code blocks unless the caller opts in, preserving public defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalMermaidMode {
    /// Keep Mermaid fences as code blocks (no promotion).
    #[default]
    Code,
    /// Promote Mermaid fences to rasterized images where the tier and
    /// terminal capability allow (rasterized only at [`GraphicsMode::Rich`]).
    Image,
}

// Re-export terminal capability types for tree-render consumers.
pub use crate::color::{ColorDepth, ColorMode, TerminalCodeContext};
pub use source::{
    Provenance, SourceDescriptor, SourceId, SourceLocation, SourceRegistry, SourceSpan,
};
pub use validate::{
    ValidationError, ValidationFinding, ValidationMode, ValidationReport, ensure_valid, validate,
};

/// A component capable of rendering itself to a canonical [`RenderNode`] tree.
///
/// This is the entry point for the render-tree pipeline: a component
/// produces a tree, which downstream renderers fold into concrete targets.
pub trait TreeRenderable {
    /// Renders the component to a canonical render tree.
    fn render_tree(&self) -> RenderNode;

    /// Optional layout for this component, seeded on its root node by the
    /// tree renderers. Defaults to `None`.
    fn tree_layout(&self) -> Option<crate::layout::Layout> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_renderable_can_supply_a_layout() {
        use crate::layout::{Layout, Length, Edges};

        struct Demo;
        impl TreeRenderable for Demo {
            fn render_tree(&self) -> RenderNode {
                RenderNode::paragraph(vec![RenderNode::text("hi")])
            }
            fn tree_layout(&self) -> Option<Layout> {
                Some(Layout {
                    margin: Edges::x(Length::ch(1)),
                    ..Layout::default()
                })
            }
        }
        assert!(Demo.tree_layout().is_some());
    }

    #[test]
    fn heading_depth_rejects_out_of_range() {
        assert!(HeadingDepth::new(0).is_err());
        assert!(HeadingDepth::new(7).is_err());
        assert!(HeadingDepth::new(255).is_err());
    }

    #[test]
    fn heading_depth_accepts_valid_range() {
        for depth in 1..=6 {
            let value = HeadingDepth::new(depth).expect("depth should be valid");
            assert_eq!(value.get(), depth);
        }
    }

    #[test]
    fn builders_default_to_synthetic_span_and_empty_attrs() {
        let node = RenderNode::text("hello");
        assert_eq!(node.span.provenance, Provenance::Synthetic);
        assert!(node.span.location.is_none());
        assert_eq!(node.attrs, NodeAttrs::default());
    }

    #[test]
    fn span_builder_sets_classes() {
        let node = RenderNode::span(vec!["highlight".into()], vec![]);
        assert_eq!(node.attrs.classes, vec!["highlight".to_string()]);
    }

    #[test]
    fn children_returns_slices_for_containers() {
        let child = || RenderNode::text("x");
        let containers: Vec<RenderNode> = vec![
            RenderNode::root(vec![child()]),
            RenderNode::heading(HeadingDepth::new(1).unwrap(), vec![child()]),
            RenderNode::paragraph(vec![child()]),
            RenderNode::block_quote(vec![child()]),
            RenderNode::list(false, None, vec![child()]),
            RenderNode::list_item(None, vec![child()]),
            RenderNode::table(vec![ColumnAlign::Left], vec![child()]),
            RenderNode::table_row(vec![child()]),
            RenderNode::table_cell(vec![child()]),
            RenderNode::emphasis(vec![child()]),
            RenderNode::strong(vec![child()]),
            RenderNode::delete(vec![child()]),
            RenderNode::span(vec![], vec![child()]),
            RenderNode::link("url", None, vec![child()]),
        ];
        for mut node in containers {
            assert_eq!(node.children().len(), 1, "kind: {:?}", node.kind);
            assert!(node.children_mut().is_some());
        }

        // Container kinds without builders.
        let footnote = RenderNode {
            kind: NodeKind::FootnoteDefinition {
                identifier: "1".into(),
                children: vec![child()],
            },
            span: SourceSpan::synthetic(),
            attrs: NodeAttrs::default(),
        };
        assert_eq!(footnote.children().len(), 1);
    }

    #[test]
    fn children_returns_empty_for_leaves() {
        let mut leaves: Vec<RenderNode> = vec![
            RenderNode::text("x"),
            RenderNode::code(None, None, "code"),
            RenderNode::thematic_break(),
            RenderNode::inline_code("c"),
            RenderNode::image("url", None, "alt"),
            RenderNode::soft_break(),
            RenderNode::hard_break(),
            RenderNode::html("<br>", false),
            RenderNode::unsupported("thing"),
        ];
        leaves.push(RenderNode {
            kind: NodeKind::FootnoteReference {
                identifier: "1".into(),
            },
            span: SourceSpan::synthetic(),
            attrs: NodeAttrs::default(),
        });
        for mut node in leaves {
            assert!(node.children().is_empty(), "kind: {:?}", node.kind);
            assert!(node.children_mut().is_none());
        }
    }

    #[test]
    fn render_node_serde_round_trip() {
        let node = RenderNode::root(vec![
            RenderNode::heading(
                HeadingDepth::new(2).unwrap(),
                vec![RenderNode::text("Title")],
            ),
            RenderNode::paragraph(vec![
                RenderNode::strong(vec![RenderNode::text("bold")]),
                RenderNode::link("https://example.com", Some("t".into()), vec![]),
            ]),
        ]);
        let json = serde_json::to_string(&node).expect("serialize");
        let decoded: RenderNode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(node, decoded);
    }

    #[test]
    fn document_serde_round_trip() {
        let mut sources = SourceRegistry::default();
        let id = sources.register(SourceDescriptor::File {
            path: "docs/example.md".into(),
        });
        sources.register(SourceDescriptor::Virtual {
            name: "inline".into(),
        });

        let mut paragraph = RenderNode::paragraph(vec![RenderNode::text("Body")]);
        paragraph.span = SourceSpan {
            provenance: Provenance::Transcluded { origin: id },
            location: Some(SourceLocation {
                source: id,
                bytes: 0..4,
            }),
        };

        let document = Document {
            sources,
            metadata: DocumentMetadata {
                frontmatter: Some(Frontmatter {
                    format: FrontmatterFormat::Yaml,
                    raw: "title: Example".into(),
                }),
            },
            root: RenderNode::root(vec![paragraph]),
        };

        let json = serde_json::to_string(&document).expect("serialize");
        let decoded: Document = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(document, decoded);
    }
}
