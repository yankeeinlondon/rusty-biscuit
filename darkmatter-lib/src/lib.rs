//! Markdown parsing, rendering, and Mermaid diagram support.
//!
//! This library provides markdown document manipulation with frontmatter support,
//! syntax highlighting, terminal and HTML rendering, and Mermaid diagram theming.
//!
//! ## Modules
//!
//! - [`markdown`] - Markdown document manipulation with frontmatter support
//! - [`mermaid`] - Mermaid diagram theming and rendering
//! - [`render`] - Hyperlink rendering utilities
//! - [`terminal`] - Terminal color detection utilities
//! - [`testing`] - Testing utilities for terminal output verification

pub mod markdown;
pub mod mermaid;
pub mod render;
pub mod terminal;

pub mod testing;
