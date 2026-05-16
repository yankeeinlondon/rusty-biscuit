//! Target renderers that fold a canonical [`RenderNode`] tree into concrete
//! output formats.
//!
//! Each renderer takes a [`RenderNode`] (or a whole [`Document`]) plus
//! target-specific options and produces a [`Rendered`] value carrying the
//! output and any non-fatal [`Diagnostic`]s, or a fatal [`RenderError`].
//!
//! [`RenderNode`]: crate::tree::RenderNode
//! [`Document`]: crate::tree::Document
//! [`Rendered`]: crate::tree::Rendered
//! [`Diagnostic`]: crate::tree::Diagnostic
//! [`RenderError`]: crate::tree::RenderError

pub mod markdown;

pub use markdown::{
    render_markdown_document, render_markdown_node, MarkdownDialect, MarkdownRenderOptions,
    MarkdownStyleOptions,
};
