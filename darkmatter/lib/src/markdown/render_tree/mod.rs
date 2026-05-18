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
//! Two darkmatter inline conveniences are **intentionally deferred** to a
//! follow-up feature: `==mark==` / dim inline styles and horizontal rules with
//! attribute blocks. Both are produced by darkmatter's `InlineStyleProcessor` /
//! `RuleProcessor`, iterator adapters that cannot consume the fold's
//! offset-carrying stream and that discard source byte ranges. Folding them
//! without losing every node's `SourceLocation` needs a separate design
//! decision — see the [`inventory`] module docs for the full rationale.
//! Frontmatter wiring is likewise deferred to a later phase.
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
pub mod fold;
pub mod inventory;
pub mod source;

pub use code_renderer::TerminalCodeRenderer;
pub use fold::fold_markdown_to_document;
