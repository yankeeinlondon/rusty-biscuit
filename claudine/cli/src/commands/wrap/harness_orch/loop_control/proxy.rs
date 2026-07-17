//! Proxy hand-off bookkeeping and the proxy target document's `initialize`
//! re-entry.

use super::*;

/// Run a proxy target document's `initialize` event after re-parsing its
/// lifecycle, respecting target-side `Skip`, `Proxy`, `Error`, and action-error
/// routing.
///
/// Called when the coordinator's bootstrap-pending flag is consumed at the top
/// of the harness loop. Resets the guard so the target gets a fresh
/// `initialize` emission before pre-flight checks run.
///
/// A target that proxies on returns a request, never a resolved path: chaining
/// a second hop is the same commit the first one went through, so it goes
/// through the same coordinator.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_target_initialize(
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    chain: &[std::path::PathBuf],
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
    let initialize_early = outcome.evaluation_error.as_ref().map(|info| {
        crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            source_path,
            "initialize",
            info,
            term,
        )
    });
    let catch_result = run_catch_protocol(
        lifecycle_guard,
        LifecycleSignal::Initialize,
        outcome.clone(),
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        None,
        loop_start,
    );
    if catch_result.evaluation_error_signal.is_some() {
        return TargetInitializeAction::Abort(surface_protocol_evaluation(
            &catch_result,
            LifecycleSignal::Initialize,
            source_path,
            initialize_early,
            term,
        ));
    }
    if let Some(setup_error) = catch_result.setup_error.as_ref() {
        // `LifecycleInitializeFailed` already exists for exactly this and is
        // what the non-loop route returns, so the two agree on the typed
        // identity of "the target's `initialize` raised".
        let reason = if matches!(catch_result.control, Some(StackControl::Error { .. })) {
            setup_error.msg.clone()
        } else {
            "lifecycle initialize failed".to_string()
        };
        return TargetInitializeAction::Abort(
            CompositionError::LifecycleInitializeFailed {
                source_path: source_path.to_path_buf(),
                reason,
            }
            .into(),
        );
    }
    if let Some(control) = catch_result.control.as_ref() {
        match control {
            StackControl::Skip => TargetInitializeAction::ExitCleanly,
            StackControl::Error { .. } => {
                unreachable!("the catch protocol consumes initialize error control")
            }
            StackControl::Proxy {
                target,
                overlay,
                location,
            } => TargetInitializeAction::Reproxy(EvaluatedProxyRequest::new(
                target.clone(),
                overlay.clone(),
                ProxyProvenance::new(source_path.to_path_buf(), *location, chain.to_vec()),
            )),
            StackControl::Stop => TargetInitializeAction::Proceed,
            // Authored placement is legal — `is_valid_for` makes every control
            // except `skip` universally placeable — but there is no provider
            // attempt at `initialize` for these to act on. So the diagnostic
            // names the stage rather than calling the action invalid, which
            // would send the user looking for an authoring mistake they did
            // not make.
            StackControl::Retry { .. }
            | StackControl::Resume { .. }
            | StackControl::Defer { .. } => TargetInitializeAction::Abort(
                CompositionError::LifecycleTransitionUnownedAtStage {
                    source_path: source_path.to_path_buf(),
                    // Only `proxy` carries an `ActionLocation`, so these name
                    // the event rather than the exact action index. The event
                    // is the actionable part regardless: `initialize` is where
                    // the stack is.
                    property: "initialize.stack".to_string(),
                    verb: match control {
                        StackControl::Retry { .. } => "retry",
                        StackControl::Resume { .. } => "resume",
                        _ => "defer",
                    },
                    stage: "initialize",
                    reason: "no provider attempt has run yet, so there is nothing to \
                             retry, resume, or defer"
                        .to_string(),
                }
                .into(),
            ),
        }
    } else {
        TargetInitializeAction::Proceed
    }
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
    /// Target's `initialize` proxied on; hand the request to the coordinator.
    Reproxy(EvaluatedProxyRequest),
}
