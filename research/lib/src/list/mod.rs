//! List command implementation for the research CLI.
//!
//! This module provides functionality to list all research topics from the filesystem,
//! with support for filtering and multiple output formats.

pub mod discovery;
pub mod filter;
pub mod format;
pub mod types;

// Re-export main types and functions for convenience
pub use discovery::{DiscoveryError, discover_topics};
pub use filter::{FilterError, apply_filters};
pub use format::{format_json, format_terminal};
pub use types::{ResearchOutput, TopicInfo};
