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
//! ```
//!
//! ## Phase 1 scope
//!
//! Phase 1 introduces the public API surface, defaults that preserve the
//! existing `for_terminal` behavior, builder methods, and validation
//! helpers. Render integration (terminal and browser) lands in later
//! phases; until then, [`DarkmatterPage`] is a configuration container.
//!
//! [`TerminalOptions`]: crate::markdown::output::terminal::TerminalOptions

mod error;
mod page;
mod types;

pub use error::PageRenderError;
pub use page::DarkmatterPage;
pub use types::{
    PageAlignment, PageBackground, PageComponent, PageFill, PageMargin, PagePadding, WidthUnit,
};
