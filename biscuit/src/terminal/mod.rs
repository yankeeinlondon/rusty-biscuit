//! Terminal capability utilities
//!
//! This module provides functions for detecting terminal color support
//! and capabilities, useful for callers that need to adapt their output
//! to the terminal's capabilities.

pub mod ansi;
mod supports;

pub use ansi::AnsiBuilder;
pub use supports::{
    color_depth, supported_underline_variants, supports_italics, supports_setting_foreground,
    supports_underline, UnderlineSupport, UnderlineVariants,
    // Color depth constants
    TRUE_COLOR_DEPTH, COLORS_256_DEPTH, COLORS_16_DEPTH, COLORS_8_DEPTH,
};

#[cfg(test)]
mod tests;
