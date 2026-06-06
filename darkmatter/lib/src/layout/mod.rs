//! Page-level layout primitive for darkmatter terminal and browser
//! rendering.
//!
//! [`DarkmatterPage`] owns a slim, renderable-typed page frame (margin, padding,
//! page background, max width) and a per-component [`ComponentPolicy`] map
//! (`layout` + optional `style`) populated from `style:` frontmatter. The page
//! delegates to the render-tree terminal and HTML renderers; per-component
//! width, padding, alignment, and color are written onto tree nodes by
//! [`decorate_document`](crate::markdown::render_tree::decorate::decorate_document)
//! and resolved by the shared renderer folds.
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
//! terminal and HTML renderers. [`LayoutContext`] carries only page-frame
//! residue (effective width, resolved background color, and the
//! `PageBackground::Pronounced` render-mode flip). Per-component layout math
//! (`resolve_component_width`, `alignment_padding`, `component_side_padding`,
//! `build_component_css`) has been deleted; the renderer folds now perform all
//! width, padding, alignment, and CSS resolution from `Layout`/`Style` node
//! attributes.
//!
//! With no builder calls, [`DarkmatterPage::render`] is byte-for-byte equivalent
//! to `for_terminal(&md, TerminalOptions::default())`.
//!
//! ## Direct `style:` lowering
//!
//! `style:` frontmatter is lowered **directly** into `renderable::layout::Layout`
//! and `renderable::style::Style` via [`ComponentPolicy`], with no
//! down-conversion to deprecated types:
//!
//! - `align` → `Layout.alignment`
//! - `fill: pad <len>` → `Layout.padding` (symmetric or aligned side)
//! - `fill: indent <len>` → `Layout.padding` on the aligned side
//! - `fill: max <len>` → `Layout.max_width`
//! - `width <len>` → `Layout.width = Width::Fixed`
//! - `margin-*` → `Layout.margin` (`Edges`)
//! - `color`/`bg-color` → `ComponentPolicy.style` (`Style.color` / `Style.background`)
//!
//! The deprecated `PageMargin`, `PagePadding`, `PageAlignment`, `PageFill`,
//! `WidthUnit`, and `PageComponent::Lists` vocabulary has been removed.
//!
//! [`TerminalOptions`]: crate::markdown::output::terminal::TerminalOptions

mod context;
mod error;
mod page;
mod types;

pub(crate) use context::LayoutContext;
pub use error::PageRenderError;
pub use page::{ComponentPolicy, DarkmatterPage};
pub use types::{PageBackground, PageComponent};
