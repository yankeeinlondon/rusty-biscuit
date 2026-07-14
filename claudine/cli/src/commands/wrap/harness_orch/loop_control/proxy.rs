//! Proxy hand-off bookkeeping and the proxy target document's `initialize`
//! re-entry.

use super::*;

/// Run a proxy target document's `initialize` event after re-parsing its
/// lifecycle, respecting target-side `Skip`, `Proxy`, `Error`, and action-error
/// routing.
///
/// Called when `proxy_tracking.pending` is consumed at the top of the harness
/// loop. Resets the guard so the target gets a fresh `initialize` emission
/// before pre-flight checks run.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_target_initialize(
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    loop_start: std::time::Instant,
) -> TargetInitializeAction {
    lifecycle_guard.reset_for_proxy();
    let outcome = run_lifecycle_event(
        lifecycle_guard,
        LifecycleSignal::Initialize,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        None,
        loop_start,
    );
    // A late-binding evaluation error on the target's `initialize` routes
    // through `failure` → `finalize` and aborts the hand-off (Decision #5).
    if let Some(err) = handle_setup_evaluation_error(
        &outcome,
        "initialize",
        lifecycle_guard,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        loop_start,
    ) {
        return TargetInitializeAction::Abort(err);
    }
    if let Some(control) = outcome.control.as_ref() {
        match control {
            StackControl::Skip => TargetInitializeAction::ExitCleanly,
            StackControl::Error { reason } => {
                let msg = reason
                    .clone()
                    .unwrap_or_else(|| "lifecycle initialize error".to_string());
                let action_error = LifecycleErrorInfo::from_action_failure("error", msg.clone());
                if let Some(ce) = emit_failure_finalize_with_err(
                    lifecycle_guard,
                    materialized,
                    source_path,
                    repo_root,
                    term,
                    effect_engine,
                    &action_error,
                    loop_start,
                ) {
                    return TargetInitializeAction::Abort(ce.into());
                }
                TargetInitializeAction::Abort(eyre!(msg))
            }
            StackControl::Proxy { target } => {
                let resolved = match claudine::composition::resolve_proxy_target(
                    target,
                    source_path,
                    repo_root,
                ) {
                    Ok(path) => path,
                    Err(e) => {
                        return TargetInitializeAction::Abort(eyre!(
                            "lifecycle initialize proxy: {e}"
                        ))
                    }
                };
                TargetInitializeAction::Repoint { resolved }
            }
            StackControl::Stop => TargetInitializeAction::Proceed,
            StackControl::Retry { .. }
            | StackControl::Resume { .. }
            | StackControl::Defer { .. } => TargetInitializeAction::Abort(eyre!(
                "lifecycle control action {control:?} is not valid at initialize"
            )),
        }
    } else if outcome.routes_to_failure(LifecycleSignal::Initialize) {
        let err = outcome.action_error.as_ref();
        // `emit_failure_finalize_with_err` requires a non-optional error; when
        // the outcome had no action_error, synthesize one so failure/finalize
        // still run with an `err` global.
        let synthetic =
            LifecycleErrorInfo::from_action_failure("error", "lifecycle initialize failed");
        let err_ref = err.cloned().unwrap_or(synthetic);
        if let Some(ce) = emit_failure_finalize_with_err(
            lifecycle_guard,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            &err_ref,
            loop_start,
        ) {
            return TargetInitializeAction::Abort(ce.into());
        }
        TargetInitializeAction::Abort(eyre!("lifecycle initialize failed"))
    } else {
        TargetInitializeAction::Proceed
    }
}

/// Proxy hand-off bookkeeping for one `run_harness_loop` call.
///
/// `chain` is the ordered list of resolved documents visited by proxy,
/// including the originating document once the first hand-off is accepted; it
/// drives the cycle/hop-limit guard.
/// `pending` is set by the `Proxy` dispatch arm and consumed at the loop top,
/// signalling that the guard's lifecycle config must be re-parsed from the
/// newly materialized target before its events fire.
#[derive(Default)]
pub(super) struct ProxyTracking {
    pub(super) chain: Vec<std::path::PathBuf>,
    pub(super) pending: bool,
}

/// What the loop should do after running a proxy target document's
/// `initialize` event.
#[derive(Debug)]
pub(super) enum TargetInitializeAction {
    /// Target's `initialize` completed cleanly; proceed to pre-flight/start.
    Proceed,
    /// Target's `initialize` opted out via `skip`; exit the run cleanly.
    ExitCleanly,
    /// Target's `initialize` could not be honored; abort with this error.
    Abort(color_eyre::eyre::Report),
    /// Target's `initialize` proxied again; repoint the loop and continue.
    Repoint { resolved: std::path::PathBuf },
}
