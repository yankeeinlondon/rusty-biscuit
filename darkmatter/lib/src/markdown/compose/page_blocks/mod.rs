//! Page block conditional content regions.
//!
//! This module provides `::block when="..."` / `::end-block` paired directives
//! that conditionally include or exclude document regions during Stage 2
//! composition.

pub mod engine;
pub mod parser;
pub mod types;

pub use types::{PageBlockError, PageBlockOptions, PageBlockRegion};
