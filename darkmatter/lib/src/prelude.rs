//! Curated re-exports of Darkmatter's renderable components.
//!
//! This prelude collects every type defined in Darkmatter that implements
//! [`TerminalRenderable`] and/or [`BrowserRenderable`], alongside the render
//! traits themselves, so downstream consumers can pull in the entire
//! rendering surface with a single glob import:
//!
//! ```
//! use darkmatter::prelude::*;
//!
//! let block = CodeBlock::rust("fn main() {}");
//! let term = biscuit_terminal::terminal::Terminal::default();
//! let out = block.render(&term);
//! assert!(!out.is_empty());
//! ```
//!
//! ## Scope
//!
//! Only types defined in Darkmatter that directly implement a renderable
//! trait are re-exported here. For the underlying render-tree IR, layout
//! primitives, terminal capability detection, and the broader component
//! catalog (tables, lists, prose, etc.), use the [`biscuit_terminal`] prelude
//! directly:
//!
//! ```
//! use biscuit_terminal::prelude::*;
//! ```
//!
//! For non-renderable Darkmatter surfaces (compose pipeline, frontmatter,
//! hashing, schemas, reference graphs), reach into the relevant
//! [`markdown`](crate::markdown), [`layout`](crate::layout), or [`style`](crate::style)
//! submodules.
//!
//! ## Components
//!
//! | Component | Source | Targets |
//! |-----------|--------|---------|
//! | [`CodeBlock`] | [`markdown::code_block`] | terminal + browser |
//! | [`DarkmatterPage`] | [`layout`] | terminal |
//! | [`DeltaReport`] | [`markdown::delta`] | terminal |
//! | [`FileTree`] | [`markdown::reference::file_tree`] | terminal |
//! | [`TocTree`] | [`markdown::toc`] | terminal |
//! | [`ValidationReportView`] | [`markdown::reference::validate`] | terminal |
//! | [`YamlBlock`] (deprecated) | [`markdown::yaml_block`] | terminal + browser |
//!
//! [`markdown::code_block`]: crate::markdown::code_block
//! [`markdown::delta`]: crate::markdown::delta
//! [`markdown::reference::file_tree`]: crate::markdown::reference::file_tree
//! [`markdown::reference::validate`]: crate::markdown::reference::validate
//! [`markdown::toc`]: crate::markdown::toc
//! [`markdown::yaml_block`]: crate::markdown::yaml_block

// Render traits — re-exported from `biscuit_terminal` so callers can invoke
// `.render(&term)` / `.render_html_fragment()` on the components below without
// a second import.
pub use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};

// Renderable components defined in Darkmatter.
pub use crate::layout::DarkmatterPage;
pub use crate::markdown::code_block::CodeBlock;
pub use crate::markdown::delta::DeltaReport;
pub use crate::markdown::reference::file_tree::FileTree;
pub use crate::markdown::reference::validate::ValidationReportView;
pub use crate::markdown::toc::TocTree;
#[allow(deprecated)]
pub use crate::markdown::yaml_block::YamlBlock;
