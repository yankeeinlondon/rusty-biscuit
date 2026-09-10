//! Prompt render components and their policy foundation.
//!
//! Provides the [`AgentPrompt`] and [`SystemPrompt`] components plus the
//! types, precedence logic, frontmatter parsing, token estimation, text
//! truncation, markdown formatting, and block-quote styling they build on.
//!
//! The public surface is intentionally small: two components, two enums,
//! two resolvers, and one frontmatter parser.

mod agent;
mod formatting;
mod frontmatter;
mod precedence;
mod system;
mod tokens;
mod truncation;
mod types;

pub use agent::AgentPrompt;
pub use frontmatter::parse_frontmatter_verbosity;
pub use precedence::{
    resolve_agent_prompt_report_mode, resolve_system_prompt_report_mode,
};
pub use system::SystemPrompt;
pub use types::{ReportMode, TruncationMode};
