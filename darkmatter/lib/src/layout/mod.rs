//! Page-level layout primitive for darkmatter terminal and browser
//! rendering.
//!
//! [`DarkmatterPage`] owns a slim, renderable-typed page frame (margin, padding,
//! page background, max width) and a per-component [`ComponentPolicy`] map
//! (`layout` plus optional `color` / `bg_color`) populated from `style:`
//! frontmatter. The page delegates to the render-tree terminal and HTML
//! renderers; per-component width, padding, alignment, and color are baked onto
//! tree nodes during construction by the context-aware fold
//! ([`TreeBuildContext`](crate::markdown::render_tree::build_context::TreeBuildContext))
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
//! `style:` frontmatter is lowered **directly** into a per-component
//! [`ComponentPolicy`] — a `renderable::layout::Layout` plus alpha-bearing
//! [`PaintColor`](renderable::style::PaintColor) colors — with no down-conversion
//! to deprecated types:
//!
//! - `align` → `Layout.alignment`
//! - `fill: pad <len>` → `Layout.padding` (symmetric or aligned side)
//! - `fill: indent <len>` → `Layout.padding` on the aligned side
//! - `fill: max <len>` → `Layout.max_width`
//! - `width <len>` → `Layout.width = Width::Fixed`
//! - `margin-*` → `Layout.margin` (`Edges`)
//! - `color`/`bg-color` → `ComponentPolicy.color` / `ComponentPolicy.bg_color`,
//!   lowered to [`PaintColor`](renderable::style::PaintColor) at the parser/apply
//!   boundary so Tailwind/hex opacity survives through the tree to the HTML target
//!
//! The `Layout` and colors are baked onto each render-tree node's
//! [`Style`](renderable::style::Style) during construction by the context-aware
//! fold ([`TreeBuildContext`](crate::markdown::render_tree::build_context::TreeBuildContext)).
//! The terminal target drops alpha (opaque color only); the browser target lowers
//! the full `PaintColor` (including alpha) directly to CSS.
//!
//! The deprecated `PageMargin`, `PagePadding`, `PageAlignment`, `PageFill`,
//! `WidthUnit`, and `PageComponent::Lists` vocabulary has been removed.
//!
//! [`TerminalOptions`]: crate::markdown::output::terminal::TerminalOptions

mod context;
mod error;
mod page;
mod types;

pub use error::PageRenderError;
pub use page::{ComponentPolicy, DarkmatterPage};
pub use types::{PageBackground, PageComponent};
