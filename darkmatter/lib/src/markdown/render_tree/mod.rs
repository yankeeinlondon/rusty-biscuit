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
//! A span-aware processor chain for `==mark==` / dim inline styles and
//! horizontal-rule attribute paragraphs lives in [`span`]. It produces
//! [`span::SpannedInlineEvent`]s with byte ranges so the fold can preserve
//! every node's [`renderable::tree::SourceLocation`] when those Darkmatter
//! constructs appear (DMTR-3). The legacy non-spanned `InlineStyleProcessor`
//! and `RuleProcessor` still back the public renderers and are unchanged.
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

pub mod code_renderer;
// `entrypoints` exposes `pub(crate)` adapter functions that are exercised by
// integration tests, benches, and the parity harness — none of which live in
// the lib crate. Silence the lib-side dead-code warnings until the public
// cutover wires the adapter into `Markdown::as_html` / `for_terminal`.
#[allow(dead_code)]
pub(crate) mod entrypoints;
pub mod fold;
pub mod inventory;
pub mod pipeline;
pub mod source;
pub mod span;

pub use code_renderer::TerminalCodeRenderer;
pub use fold::{
    fold_markdown_spanned_with_frontmatter, fold_markdown_to_document,
    fold_markdown_to_document_with_metadata, fold_markdown_with_frontmatter,
};
pub use pipeline::{PipelineRenderResult, PipelineResult};

// Internal experimental entry points (DMTR-1). These are `pub(crate)` so
// parity tests and benchmarks inside `darkmatter` can drive the tree path
// without exposing it to downstream callers. The public `Markdown::as_html`,
// `Markdown::as_terminal`, and `for_terminal` continue to use the legacy
// renderers until cutover; see
// `renderable/features/2026-05-20-darkmatter-tree/entry-point-shape.md`.
#[allow(unused_imports)]
pub(crate) use entrypoints::{
    render_tree_html, render_tree_markdown, render_tree_markdown_dialect, render_tree_terminal,
    to_render_document,
};
