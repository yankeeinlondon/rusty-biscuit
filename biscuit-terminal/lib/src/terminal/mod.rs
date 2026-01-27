//! Terminal capability utilities
//!
//! This module provides functions for detecting terminal color support
//! and capabilities, useful for callers that need to adapt their output
//! to the terminal's capabilities.

pub mod ansi;
pub mod color;
pub mod detection;
pub mod eval;
pub mod multiplex;
pub mod render;
pub mod styling;
pub mod supports;
pub mod terminal;
