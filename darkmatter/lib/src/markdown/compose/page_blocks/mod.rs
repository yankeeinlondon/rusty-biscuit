//! Page block conditional content regions.
//!
//! This module provides `::block when="..."` / `::end-block` paired directives
//! that conditionally include or exclude document regions during the Inline Pre
//! phase of composition. [`BodyGuards`] gives the sites inside a block the
//! guard facts of its `when` condition.

pub mod engine;
pub mod guards;
pub mod parser;
pub mod types;

pub use guards::{BodyGuard, BodyGuards};
pub use types::{PageBlockError, PageBlockOptions, PageBlockRegion};
