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
//! The render-tree work is staged. This module currently contains only the
//! verified parser-event [`inventory`] — the exhaustive, compile-checked
//! catalog of every [`pulldown_cmark`] 0.13 [`Event`]/[`Tag`] variant and the
//! disposition the eventual fold must apply to it. The fold itself (the
//! `pulldown-cmark` event stream walker that builds the
//! [`renderable::tree::Document`]) is **not** implemented here yet; it arrives
//! in a later phase and will rely on the dispositions recorded in
//! [`inventory`].
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
//! that fails to build if `pulldown-cmark` ever changes its enums, forcing the
//! inventory (and later the fold) to be re-verified.

pub mod inventory;
