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
mod error;
mod node;
pub mod render;
mod source;
mod validate;

pub use attrs::{
    CodeRenderHints, ColumnConditional, ColumnWidthKind, ColumnsHints, HintNamespace, LayoutHints,
    ListRenderHints, NodeAttrs, ProgressHints, TableCellHints, TableColumnHints, TableTerminalHints,
};
pub use diagnostic::{Diagnostic, DiagnosticKind, Severity};
pub use document::{Document, DocumentMetadata, Frontmatter, FrontmatterFormat};
pub use error::{RenderError, RenderStrictness, Rendered};
pub use node::{ColumnAlign, HeadingDepth, HeadingDepthError, NodeKind, RenderNode};
pub use render::{
    BrowserRenderOptions, CodeRenderer, MarkdownDialect, MarkdownRenderOptions,
    MarkdownStyleOptions, RawHtmlPolicy, render_browser_document, render_browser_node,
    render_markdown_document, render_markdown_node,
};

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

    /// Returns layout hints for tree rendering, if any.
    ///
    /// Components that want to influence layout during tree rendering can
    /// override this to provide hints. The default returns `None`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::tree::{LayoutHints, RenderNode, TreeRenderable};
    ///
    /// struct MyComponent;
    ///
    /// impl TreeRenderable for MyComponent {
    ///     fn render_tree(&self) -> RenderNode {
    ///         RenderNode::text("Hello")
    ///     }
    ///
    ///     fn tree_layout_hints(&self) -> Option<LayoutHints> {
    ///         Some(LayoutHints {
    ///             top_margin: Some(1),
    ///             ..Default::default()
    ///         })
    ///     }
    /// }
    /// ```
    fn tree_layout_hints(&self) -> Option<LayoutHints> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
