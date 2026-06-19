//! Prompt reporting foundation for Claudine.
//!
//! This module provides the types, precedence logic, frontmatter parsing,
//! token estimation, text truncation, markdown formatting, and block-quote
//! styling needed for the prompt reporting feature.
//!
//! The public surface is intentionally small: two report types, two enums,
//! two resolvers, and one frontmatter parser.

mod formatting;
mod frontmatter;
mod precedence;
mod system_prompt;
mod tokens;
mod truncation;
mod types;
mod user_prompt;

pub use frontmatter::parse_frontmatter_verbosity;
pub use precedence::{
    resolve_agent_prompt_report_mode, resolve_system_prompt_report_mode,
};
pub use system_prompt::SystemPromptReport;
pub use types::{ReportMode, TruncationMode};
pub use user_prompt::AgentPromptReport;
