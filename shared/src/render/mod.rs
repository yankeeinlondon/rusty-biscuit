//! Rendering utilities for multiple output formats
//!
//! This module provides types that can render content to different
//! output formats including terminal (with ANSI escape sequences),
//! HTML, and Markdown.

pub mod link;

pub use link::{Link, LinkType};
