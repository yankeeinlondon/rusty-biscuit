//! Canonical render-tree construction for darkmatter Markdown.
//!
//! This module is darkmatter's home for the **events → tree fold**: the
//! conversion of a [`pulldown_cmark`] 0.13 [`Event`] stream into a
//! [`renderable::tree::Document`]. The render tree (`renderable::tree`) is the
//! single owned, target-agnostic representation that the Terminal, Markdown,
//! Browser, and AST renderers all consume.
//!
//! [`Event`]: pulldown_cmark::Event
//!
//! ## Phasing
//!
//! The render-tree work is staged. This module contains the verified
//! parser-event [`inventory`] — the exhaustive, compile-checked catalog of
//! every [`pulldown_cmark`] 0.13 [`Event`]/[`Tag`] variant and the disposition
//! the fold applies to it — together with the fold itself
//! ([`fold_markdown_to_document`]), which handles the common CommonMark + GFM
//! subset plus footnotes, grouped raw-HTML blocks, and native
//! superscript/subscript, relying on the dispositions recorded in
//! [`inventory`].
//!
//! Frontmatter wiring is now available via
//! [`fold_markdown_with_frontmatter`]: darkmatter's already extracted
//! frontmatter flows through [`renderable::tree::DocumentMetadata::frontmatter`]
//! without re-parsing through `pulldown-cmark`'s metadata-block options (DMTR-4).
//!
//! `==mark==` / `⌄dim⌄` inline styles are handled by the source-layer rewriter
//! in [`inline_extension`]: it turns those constructs into canonical
//! GFM-strikethrough envelopes before parsing, and the fold's strikethrough
//! dispatcher lowers each envelope to a [`renderable::tree::NodeKind::Extended`]
//! node (`mark` / `dim`) while resolving every span back to the original source
//! through the rewriter's provenance table. HR-attribute paragraphs are still
//! lifted out of the event stream by the `block_extension` module, a dedicated
//! offset-aware block processor that sits between `pulldown-cmark` and the fold.
//! See `renderable/features/2026-05-26-inline-span/spec.md`.
//!
//! The non-spanned `InlineStyleProcessor` now backs only the
//! `scan_inline_hr_warnings` strict-style preflight; the public renderers all
//! route through this spanned fold.
//!
//! [`Tag`]: pulldown_cmark::Tag
//!
//! ## Why an inventory first
//!
//! `pulldown-cmark` extension surface (tables, footnotes, strikethrough,
//! sub/superscript, math, definition lists, metadata blocks) is gated behind
//! [`Options`](pulldown_cmark::Options) flags, and its `Event`/`Tag`/`TagEnd`
//! enums evolve between releases. The fold must handle every variant the
//! parser can emit under darkmatter's chosen options, and degrade predictably
//! for the rest. The [`inventory`] submodule pins that contract: it documents
//! each variant's disposition and carries a compile-time exhaustive-match test
//! that fails to build if `pulldown-cmark` ever changes its enums.
//!
//! ## Fold
//!
//! The [`fold`] submodule implements the Milestone 1 fold itself.
//! [`fold_markdown_to_document`] is the public entry point: it walks a
//! `pulldown-cmark` event stream and builds a [`renderable::tree::Document`].

pub(crate) mod build_context;
pub(crate) mod block_extension;
pub mod disclosure_scan;
pub(crate) mod disclosure_style;
pub mod code_renderer;
// The inline source rewriter backs `fold_markdown_spanned_with_frontmatter`.
// A few `pub(crate)` helpers on its result types (e.g. `InlineRewrite::
// was_rewritten`) are exercised only by the module's own unit tests, so the
// lib-side dead-code lint stays silenced.
#[allow(dead_code)]
pub(crate) mod inline_extension;
pub mod entrypoints;
pub mod fold;
pub mod inventory;
pub mod pipeline;
pub mod source;
#[cfg(test)]
mod structural_gate;
#[cfg(test)]
mod style_tree_parity_tests;
pub mod svg_sanitizer;

#[allow(deprecated)]
pub use code_renderer::TerminalCodeRenderer;
pub use fold::{
    fold_markdown_spanned_with_frontmatter, fold_markdown_to_document,
    fold_markdown_to_document_with_metadata, fold_markdown_with_frontmatter,
};
pub use pipeline::{PipelineRenderResult, PipelineResult};

// The render-tree entry points are the in-crate adapter boundary the public
// `Markdown` / `DarkmatterPage` renderers route through: `render_tree_html`
// backs `Markdown::as_html`, `render_tree_terminal` backs `Markdown::as_terminal`
// / the default-layout `DarkmatterPage::render` path, and the markdown variants
// serve round-trip callers and the parity suite. `to_render_document` stays
// `pub(crate)`: it exposes the raw fold and is an internal helper.
pub use entrypoints::{
    render_tree_html, render_tree_markdown, render_tree_markdown_dialect, render_tree_terminal,
};
#[allow(unused_imports)]
pub(crate) use entrypoints::to_render_document;
