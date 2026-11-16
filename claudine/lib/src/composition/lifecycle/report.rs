//! The one place a lifecycle failure that no error return carries is shown.
//!
//! An event's executor records a failed action (a refused write, a failed
//! send, an editor that could not run) in its outcome. When the host's policy
//! keeps that failure out of the command's error return, because a terminal
//! phase leaves the outcome unchanged or a container folds it into an exit
//! code, the host reports it here, once, so the author still learns which
//! action failed and why.

use biscuit_terminal::terminal::Terminal;

use super::LifecycleEmitter;
use super::LifecycleSignal;
use super::context::LifecycleErrorInfo;
use super::executor::LifecycleEventOutcome;

/// The text a reported failure shows the author: the complete message, never
/// the clamped `err.msg`, so a refused write keeps its whole target and root.
fn actionable_text(error: &LifecycleErrorInfo) -> &str {
    error.actionable_msg()
}

/// Warn about a document event's action error that the run's error does not
/// carry.
///
/// An action error in `initialize` or `start` fails the document and is
/// reported as its error, so nothing is written for it here. Everywhere else
/// the warning is the failure's only trace: a terminal-phase event
/// (`success`, `failure`, `cancelled`, `finalize`, `loop`) leaves the outcome
/// unchanged, and `blocked` already runs for an earlier error, which the run
/// reports instead. An action under `no_error: true` never reaches the
/// outcome.
pub fn warn_tolerated_action_error(
    emitter: &dyn LifecycleEmitter,
    term: &Terminal,
    signal: LifecycleSignal,
    outcome: &LifecycleEventOutcome,
) {
    if matches!(signal, LifecycleSignal::Initialize | LifecycleSignal::Start) {
        return;
    }
    if let Some(error) = outcome.action_error.as_ref() {
        let event = signal.property_name();
        emitter.emit_warn(&format!("`{event}` action failed: {}", actionable_text(error)), term);
    }
}

/// Report a failure a container recorded and folded into its exit code.
///
/// `subject` names the container (`sequence`). The failure line has the
/// error state a failed step's line has.
pub fn report_container_failure(
    emitter: &dyn LifecycleEmitter,
    term: &Terminal,
    subject: &str,
    error: &LifecycleErrorInfo,
) {
    emitter.emit_error(&format!("{subject} failed: {}", actionable_text(error)), term);
}
