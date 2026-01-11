//! Content isolation and interpolation for markdown and HTML documents.
//!
//! This module provides utilities for extracting specific content from structured
//! documents (isolation) and replacing content based on patterns (interpolation).
//!
//! ## Modules
//!
//! - [`mod@types`] - Core types: [`IsolateAction`], [`IsolateResult`], and error types
//! - [`mod@md_scope`] - Markdown element targeting via [`MarkdownScope`]
//! - [`md_isolate()`] - Markdown content extraction using pulldown-cmark
//! - [`mod@html_scope`] - HTML element targeting via [`HtmlScope`] and [`HtmlTag`]
//! - [`html_isolate()`] - HTML content extraction using DOM parsing
//!
//! ## Examples
//!
//! ```
//! use shared::isolate::{IsolateAction, MarkdownScope, IsolateResult, md_isolate};
//!
//! let content = "# Heading\n\nSome **bold** text.";
//!
//! // Extract headings
//! let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector);
//! assert!(result.is_ok());
//! ```

pub mod html_isolate;
pub mod html_scope;
pub mod md_isolate;
pub mod md_scope;
pub mod types;

// Re-export core types for convenient access
pub use html_isolate::html_isolate;
pub use html_scope::{HtmlScope, HtmlTag};
pub use md_isolate::md_isolate;
pub use md_scope::MarkdownScope;
pub use types::{
    InterpolateError, InterpolateResultType, IsolateAction, IsolateError, IsolateResult,
    IsolateResultType,
};
