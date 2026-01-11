//! Content interpolation for strings, markdown, and HTML documents.
//!
//! This module provides utilities for replacing content based on patterns.
//! It includes both context-unaware (simple string/regex) and context-aware
//! (markdown/HTML structure-preserving) interpolation.
//!
//! ## Modules
//!
//! - [`interpolate()`] - Simple string find/replace
//! - [`interpolate_regex()`] - Regex-based find/replace with capture groups
//! - [`md_interpolate()`] - Markdown-aware content replacement (preserves structure)
//! - [`html_interpolate()`] - HTML-aware content replacement (preserves structure)
//!
//! ## Examples
//!
//! ```
//! use shared::interpolate::{interpolate, interpolate_regex};
//!
//! // Simple string replacement
//! let result = interpolate("Hello world", "world", "Rust");
//! assert_eq!(result, "Hello Rust");
//!
//! // Regex replacement with capture groups
//! let result = interpolate_regex("Hello 123", r"\d+", "456").unwrap();
//! assert_eq!(result, "Hello 456");
//! ```

pub mod html_interpolate;
pub mod interpolate;
pub mod interpolate_regex;
pub mod md_interpolate;

// Re-export core functions for convenient access
pub use html_interpolate::{html_interpolate, html_interpolate_regex};
pub use interpolate::interpolate;
pub use interpolate_regex::interpolate_regex;
pub use md_interpolate::{md_interpolate, md_interpolate_regex};
