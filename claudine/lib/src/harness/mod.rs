//! Harness module for typed pre/post validations, timeouts, and handlers.
//!
//! Turns Markdown-backed non-interactive prompts into a small, typed job
//! harness with pre-run checks, post-run checks, per-page timeout
//! configuration, and typed recovery handlers.

pub mod error;
pub mod handlers;
pub mod model;
pub mod parse;
pub mod resolve;
pub mod runtime;
pub mod shell;
pub mod speech;
pub mod timeout;
pub mod validate;

pub use error::HarnessError;
pub use handlers::{
    FailureContext, build_agent_failure_context, build_validation_failure_context,
    classify_failure, execute_deviate_command, resolve_handler, validate_resume,
};
pub use model::*;
pub use parse::{has_harness_properties, parse_harness_plan, parse_harness_plan_with_shell};
pub use resolve::{HarnessResolutionContext, resolve_harness_path};
pub use runtime::build_attempt_outcome;
pub use shell::{
    ShellApprovalOptions, execute_approved_command, validate_and_approve_command,
    validate_and_approve_command_parts,
};
pub use speech::speak_when_able;
pub use timeout::parse_timeout;
pub use validate::{capture_pre_run_snapshot, evaluate_post_checks, evaluate_pre_checks};
