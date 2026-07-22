//! Program installation: planning, command building, execution, and the
//! interactive retry interview.
//!
//! Layers (each depends only on lower layers):
//!
//! 1. [`options`] — `InstallOptions`, `InstallResult`, captured-result types.
//! 2. [`command`] — pure command building, validation, prose builders, and
//!    host eligibility (`method_available`, `select_best_method`).
//! 3. [`execute`] — subprocess execution, including the `UvWithInstall`
//!    bootstrap.
//! 4. [`plan`] — `InstallPlan`, `InstallPlanReason`, and the bucket-based
//!    selection logic.
//! 5. [`interview`] — caller-driven interactive retry runner.

pub mod command;
pub mod execute;
pub mod interview;
pub mod options;
pub mod plan;

pub(crate) use command::method_available;
pub use command::{
    astral_installer_url, build_install_announcement, build_install_failure_status,
    build_install_success_status, build_install_timeout_warning, build_retry_choice_prose,
    build_retry_quit_prose, get_install_command, get_versioned_install_command,
};
pub use execute::{
    execute_install, execute_install_captured, execute_versioned_install,
    execute_versioned_install_captured,
};
pub use interview::{
    InstallInterviewDelegate, InstallInterviewEvent, InstallInterviewInput,
    InstallInterviewOptions, InstallInterviewOutcome, InstallOutputStream, InstallStatusKind,
    RetryChoice, RetryPrompt, RetryPromptChoice, run_install_interview,
};
pub use options::{InstallCapturedOutcome, InstallCapturedResult, InstallOptions, InstallResult};
pub use plan::{InstallPlan, InstallPlanOption, InstallPlanReason, build_install_plan};
