//! Prompt reporting foundation for Claudine.
//!
//! This module provides the types, precedence logic, frontmatter parsing,
//! token estimation, text truncation, markdown formatting, and block-quote
//! styling needed for the prompt reporting feature.

pub mod formatting;
pub mod frontmatter;
pub mod precedence;
pub mod tokens;
pub mod truncation;
pub mod types;

pub use formatting::{
    collapse_blank_lines, create_system_prompt_blockquote, create_user_prompt_blockquote,
    render_markdown_for_terminal,
};
pub use frontmatter::parse_frontmatter_verbosity;
pub use precedence::{
    resolve_system_prompt_report_config, resolve_user_prompt_report_config,
};
pub use tokens::{estimate_system_prompt_tokens, estimate_tokens, estimate_tokens_dense};
pub use truncation::{strip_leading_whitespace, truncate_front_back};
pub use types::{
    PromptReportFormat, PromptVerbosity, SystemPromptReportConfig, TruncationMode,
    UserPromptReportConfig,
};
