//! Terminal renderer for the canonical `renderable` render tree.
//!
//! This module folds a [`RenderNode`](renderable::tree::RenderNode) tree into
//! terminal output, reusing biscuit-terminal's existing components rather
//! than re-implementing terminal formatting.
//!
//! ## Entry points
//!
//! - [`render_terminal_node`] / [`render_terminal_document`] — render a tree
//!   or whole document to a terminal string.
//! - [`TreeComponent`] — adapts any
//!   [`TreeRenderable`](renderable::tree::TreeRenderable) into a
//!   [`TerminalRenderable`](crate::components::renderable::TerminalRenderable).
//! - [`TerminalRenderOptions`] / [`TerminalRenderContext`] — render options
//!   and the terminal-capability snapshot the renderer consults.
//!
//! ## Examples
//!
//! ```
//! use renderable::tree::RenderNode;
//! use biscuit_terminal::render_tree::{render_terminal_node, TerminalRenderOptions};
//!
//! let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
//!     RenderNode::text("Hello, terminal"),
//! ])]);
//! let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
//! assert!(rendered.output.contains("Hello, terminal"));
//! ```

mod component;
mod options;
mod render;

pub use component::TreeComponent;
pub use options::{TerminalRenderContext, TerminalRenderOptions};
pub use render::{render_terminal_document, render_terminal_node};
