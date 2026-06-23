//! Harness module for timeouts, shell audit, and handler execution.
//!
//! Turns Markdown-backed non-interactive prompts into a small, typed job
//! harness with per-page timeout configuration and typed recovery handlers.

pub mod audit;
pub mod error;
pub mod handlers;
pub mod model;
pub mod parse;
pub mod report;
pub mod resolve;
pub mod runtime;
pub mod shell;
pub mod speech;
pub mod timeout;

pub use audit::{audit_shell_commands, collect_auditable_commands};
pub use error::HarnessError;
pub use handlers::{
    FailureContext, build_agent_failure_context, build_audit_failure_context, classify_failure,
    execute_deviate_command, resolve_handler, validate_resume,
};
pub use model::*;
pub use parse::{has_harness_properties, parse_harness_plan};
pub use resolve::{HarnessResolutionContext, resolve_harness_path};
pub use runtime::build_attempt_outcome;
pub use shell::{
    ShellApprovalOptions, execute_approved_command, validate_and_approve_command,
    validate_and_approve_command_parts,
};
pub use speech::speak_when_able;
pub use timeout::parse_timeout;
