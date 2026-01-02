//! Terminal capability utilities
//!
//! This module provides functions for detecting terminal color support
//! and capabilities, useful for callers that need to adapt their output
//! to the terminal's capabilities.

mod supports;

pub use supports::{color_depth, supports_italics, supports_setting_foreground};

#[cfg(test)]
mod tests;
