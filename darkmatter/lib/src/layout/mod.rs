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
//! [`TerminalOptions`]: crate::markdown::output::terminal::TerminalOptions

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
