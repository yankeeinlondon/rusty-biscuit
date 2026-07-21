//! Harness module for timeouts, shell audit, and runtime classification.
//!
//! Turns Markdown-backed non-interactive prompts into a small, typed job
//! harness with per-page timeout configuration and lifecycle recovery.

pub mod audit;
pub mod error;
pub mod model;
pub mod parse;
pub mod report;
pub mod resolve;
pub mod runtime;
pub mod shell;
pub mod speech;
pub mod timeout;

pub use audit::{audit_shell_commands, collect_auditable_commands};
pub use error::{HarnessError, PathResolutionFailure, ResolutionDetail};
pub use model::*;
pub use parse::{has_harness_properties, parse_harness_plan};
pub use resolve::{
    HarnessResolutionContext, resolve_harness_path, resolve_harness_path_in_context,
};
pub use runtime::{build_attempt_outcome, classify_failure, concise_message, failure_message};
pub use shell::{
    ShellApprovalOptions, execute_approved_command, validate_and_approve_command,
    validate_and_approve_command_parts,
};
pub use speech::speak_when_able;
pub use timeout::{parse_timeout, parse_timeout_allow_zero};
