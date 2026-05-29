//! Prompt reporting foundation for Claudine.
//!
//! This module provides the types, precedence logic, frontmatter parsing,
//! token estimation, text truncation, markdown formatting, and block-quote
//! styling needed for the prompt reporting feature.

pub mod formatting;
pub mod frontmatter;
pub mod precedence;
pub mod state;
pub mod system_prompt;
pub mod tokens;
pub mod truncation;
pub mod types;
pub mod user_prompt;

pub use formatting::{
    collapse_blank_lines, create_system_prompt_blockquote, create_user_prompt_blockquote,
    render_markdown_for_terminal, system_prompt_blockquote_styled,
};
pub use frontmatter::parse_frontmatter_verbosity;
pub use precedence::{
    resolve_agent_prompt_report_mode, resolve_system_prompt_report_config,
    resolve_system_prompt_report_config_with_change, resolve_system_prompt_report_mode,
    resolve_user_prompt_report_config,
};
pub use system_prompt::{
    render_system_prompt_body, render_system_prompt_header, render_system_prompt_summary,
    report_system_prompt, report_system_prompt_empty, report_system_prompt_with_base,
};
pub use tokens::{estimate_system_prompt_tokens, estimate_tokens, estimate_tokens_dense};
pub use truncation::{strip_leading_whitespace, truncate_front_back};
pub use types::{
    PromptReportFormat, PromptVerbosity, ReportMode, SystemPromptReportConfig, TruncationMode,
    UserPromptReportConfig,
};
pub use user_prompt::{render_user_prompt_body, render_user_prompt_header, report_user_prompt};
