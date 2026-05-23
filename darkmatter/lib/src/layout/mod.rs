//! Page-level layout primitive for darkmatter terminal and browser
//! rendering.
//!
//! [`DarkmatterPage`] owns layout state (margin, padding, page background,
//! max width, line numbers, per-component alignment, and per-component fill)
//! and orchestrates the existing [`TerminalOptions`] knobs through builder
//! pass-throughs.
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::terminal::Terminal;
//! use darkmatter::layout::{DarkmatterPage, PageBackground};
//!
//! let term = Terminal::new_optimistic(120);
//! let page = DarkmatterPage::new(&term)
//!     .with_margin(2)
//!     .with_padding(1)
//!     .with_page_background(PageBackground::Subtle)
//!     .with_max_width(100);
//!
//! let md: darkmatter::markdown::Markdown = "# Hello\n\nWorld\n".into();
//! let output = page.render(&md).unwrap();
//! ```
//!
//! ## Overview
//!
//! The layout module provides a single entry point — [`DarkmatterPage`] — that
//! captures terminal capabilities at construction and delegates to the existing
//! terminal and HTML renderers, threading a [`LayoutContext`] through the render
//! pipeline so per-component alignment and fill are applied to images, block
//! quotes, tables, code blocks, and lists.
//!
//! With no builder calls, [`DarkmatterPage::render`] is byte-for-byte equivalent
//! to `for_terminal(&md, TerminalOptions::default())`.
//!
//! ## Migration deferral (Spec A)
//!
//! The deprecated page-layout types (`PageMargin`, `PagePadding`,
//! `PageAlignment`, `PageFill`) remain the internal storage of
//! [`DarkmatterPage`] and [`LayoutContext`]. This is a **deliberate deferral**
//! of the full migration to `renderable::layout::Layout` on the document root:
//!
//! - The deprecated types have complete `From`/`TryFrom` conversion bridges
//!   onto their `renderable::layout` counterparts (see `types.rs`).
//! - The deprecation bridge is the accepted compatibility boundary: callers
//!   that construct a [`DarkmatterPage`] via the builder produce results
//!   identical to constructing an equivalent `renderable::layout::Layout`
//!   and converting through the bridge.
//! - The full migration (replacing `DarkmatterPage`'s internal storage and
//!   `LayoutContext` derivation with `renderable::layout::Layout`) is deferred
//!   to a follow-up milestone that will also migrate the page assembler.
//!
//! [`TerminalOptions`]: crate::markdown::output::terminal::TerminalOptions

// The page-layout types (`PageMargin`, `PagePadding`, `PageAlignment`,
// `PageFill`) are deprecated in favor of `renderable::layout::Layout`. They
// remain part of this module's public surface because the darkmatter CLI
// still drives them through the `DarkmatterPage` builder; the re-exports
// below legitimately reference them.
#![allow(deprecated)]

mod context;
mod error;
mod page;
mod types;

pub(crate) use context::LayoutContext;
pub use error::PageRenderError;
pub use page::DarkmatterPage;
pub use types::{
    PageAlignment, PageBackground, PageComponent, PageFill, PageMargin, PagePadding, WidthUnit,
};
