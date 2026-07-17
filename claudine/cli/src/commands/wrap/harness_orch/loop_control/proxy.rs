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
        let message = if matches!(
            catch_result.control,
            Some(StackControl::Error { .. })
        ) {
            setup_error.msg.clone()
        } else {
            "lifecycle initialize failed".to_string()
        };
        return TargetInitializeAction::Abort(eyre!(message));
    }
    if let Some(control) = catch_result.control.as_ref() {
        match control {
            StackControl::Skip => TargetInitializeAction::ExitCleanly,
            StackControl::Error { .. } => {
                unreachable!("the catch protocol consumes initialize error control")
            }
            StackControl::Proxy { target } => {
                let resolved = match claudine::composition::resolve_proxy_target(
                    target,
                    source_path,
                    repo_root,
                ) {
                    Ok(path) => path,
                    Err(e) => {
                        return TargetInitializeAction::Abort(
                            claudine::composition::CompositionError::InvalidFileReference {
                                context: Box::new(
                                    claudine::composition::FileReferenceContext {
                                        source_path: source_path.to_path_buf(),
                                        event: Some("initialize".to_string()),
                                        property: "initialize".to_string(),
                                        reference: target.clone(),
                                        hint:
                                            crate::commands::wrap::composition::PROXY_TARGET_HINT
                                                .to_string(),
                                    },
                                ),
                                source: e,
                            }
                            .into(),
                        );
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
