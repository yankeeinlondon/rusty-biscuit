//! Claudine's checks, registered into Darkmatter's preflight.
//!
//! Each check keeps the body and error it had before preflight was
//! Darkmatter's, so its prompts, diagnostics, and their order do not change.
//! A failure before the document's run starts ([`CheckStage::BeforeRun`]:
//! agent resolution and the eager schema verdict) fires no lifecycle event;
//! one inside the run ([`CheckStage::InRun`]) is the run's preflight failure
//! and its call site routes `blocked` then `finalize`.

use std::sync::OnceLock;

use darkmatter::lifecycle::preflight::{CheckSpec, CheckStage, PreflightPlan};
use darkmatter::lifecycle::run::EntryReason;

/// Resolving the provider the document launches.
pub const AGENT_RESOLUTION: &str = "agent_resolution";
/// The launch's schema verdict.
pub const LAUNCH_SCHEMA: &str = "launch_schema";
/// Approving the lifecycle's shell commands.
pub const LIFECYCLE_SHELL_AUDIT: &str = "lifecycle_shell_audit";

/// A document's checks, in run order.
pub fn claudine_preflight() -> &'static PreflightPlan {
    static PLAN: OnceLock<PreflightPlan> = OnceLock::new();
    PLAN.get_or_init(|| plan(&[AGENT_RESOLUTION, LAUNCH_SCHEMA, LIFECYCLE_SHELL_AUDIT]))
}

/// A sequence's checks, in run order: the graph's front-loaded shell
/// approval precedes per-task provider selection.
pub fn sequence_preflight() -> &'static PreflightPlan {
    static PLAN: OnceLock<PreflightPlan> = OnceLock::new();
    PLAN.get_or_init(|| plan(&[LIFECYCLE_SHELL_AUDIT, AGENT_RESOLUTION]))
}

fn plan(names: &[&'static str]) -> PreflightPlan {
    names
        .iter()
        .try_fold(PreflightPlan::new(), |plan, &name| plan.register(CheckSpec { name }))
        .expect("Claudine registers each check once")
}

/// Runs one check of `plan` at a call site that owns no other check, and
/// returns its body's own result.
///
/// ## Errors
///
/// The body's error, unchanged.
pub fn run_check<T, E>(
    plan: &PreflightPlan,
    name: &'static str,
    stage: CheckStage,
    entry: EntryReason,
    body: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    plan.begin(entry).check(name, stage, body).map_err(|failure| failure.error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claudine_registers_its_checks_in_todays_run_order() {
        assert_eq!(
            claudine_preflight().names().collect::<Vec<_>>(),
            [AGENT_RESOLUTION, LAUNCH_SCHEMA, LIFECYCLE_SHELL_AUDIT]
        );
        assert_eq!(sequence_preflight().names().collect::<Vec<_>>(), [LIFECYCLE_SHELL_AUDIT, AGENT_RESOLUTION]);
    }

    #[test]
    fn a_check_returns_its_bodys_own_error() {
        let error = run_check(claudine_preflight(), AGENT_RESOLUTION, CheckStage::BeforeRun, EntryReason::Initial, || {
            Err::<(), _>("no agent")
        })
        .unwrap_err();
        assert_eq!(error, "no agent");
    }
}
