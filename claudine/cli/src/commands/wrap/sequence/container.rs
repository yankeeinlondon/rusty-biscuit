//! Root sequence lifecycle. Child results stay in the invocation cell until cleanup settles.

use claudine::composition::LifecycleErrorInfoExt;
use std::sync::Arc;
use claudine::composition::{CompositionError, RuntimeState};
use claudine::composition::lifecycle::{LifecycleRunGuard, LifecycleRunGuardExt, LifecycleRuntimeContext, LifecycleSignal};
use claudine::composition::lifecycle::context::LifecycleErrorInfo;
use claudine::composition::lifecycle::executor::StackExecutionContext;
use claudine::composition::lifecycle::policy::{RefusalRoute, SubjectKind};
use claudine::composition::lifecycle::runtime::{ContainerEventDecision, ContainerRunner};
use claudine::composition::looping::{ContainerPassEnd, LoopFrame, PassCheckpoint, container_state, finish_container_pass};
use color_eyre::eyre::Result;

pub(super) struct RootRun<'a> {
    pub(super) runner: ContainerRunner<'a>,
    pub(super) stack: StackExecutionContext<'a>,
    pub(super) runtime: Arc<RuntimeState>,
    pub(super) started: bool,
    pub(super) approved: Option<darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals>,
    /// The task graph the first attempt discovered, which every retry runs.
    pub(super) graph: Option<claudine::composition::PreflightGraph>,
    pub(super) pending: Option<ContainerEventDecision>,
    pub(super) child_error: Option<LifecycleErrorInfo>,
    pub(super) proxy_location: Option<claudine::composition::ActionLocation>,
    /// The root's loop, when it repeats; allocated before `initialize`.
    pub(super) frame: Option<LoopFrame>,
    /// The runtime state the current pass entered `start` with; `None`
    /// until a pass enters. A retry of the pass returns to it.
    pub(super) checkpoint: Option<PassCheckpoint>,
    prewarm: claudine::composition::lifecycle::prewarm::PrewarmHandle,
}

impl<'a> RootRun<'a> {
    pub(super) fn new(
        config: &'a claudine::composition::LifecycleConfig,
        context: &'a LifecycleRuntimeContext<'a>,
        stack: StackExecutionContext<'a>,
        runtime: Arc<RuntimeState>,
        frame: Option<LoopFrame>,
        prewarm: claudine::composition::lifecycle::prewarm::PrewarmHandle,
    ) -> Self {
        Self { runner: ContainerRunner::new(LifecycleRunGuard::new(config, context, stack.emitter)).with_subject(SubjectKind::RootSequence),
            stack, runtime, started: false, approved: None, graph: None, pending: None, child_error: None, proxy_location: None,
            frame, checkpoint: None, prewarm }
    }

    /// The `loop` values of the pass being run, when the root loops.
    pub(super) fn loop_values(&self) -> Option<claudine::composition::LoopAmbient> {
        self.frame.as_ref().map(LoopFrame::ambient)
    }

    /// Report a looping root's pass result, its own slice of the cumulative
    /// outputs, before the pass's terminal event reads it.
    pub(super) fn record_pass(&mut self, exit_code: i32) {
        let Some(frame) = self.frame.as_mut() else { return };
        let outputs = self.runtime.snapshot().outputs;
        let start = self.checkpoint.as_ref().map_or(0, PassCheckpoint::outputs).min(outputs.len());
        frame.record_pass(serde_json::Value::Array(outputs[start..].to_vec()), exit_code);
    }

    /// The process exit code of a root loop a `break` ended, with its
    /// summary line printed; `None` when no break is pending.
    pub(super) fn broken_exit_code(&self) -> Option<i32> {
        let broken = self.frame.as_ref()?.pending_break()?;
        self.stack.emitter.emit_info(&broken.summary(), self.stack.term);
        Some(i32::from(broken.code))
    }

    /// Settle a looping root's successful pass: its `success` event and
    /// `finalize`, or `finalize` alone when a `break` is pending.
    pub(super) fn settle_pass(&mut self) -> Result<ContainerEventDecision> {
        if self.frame.as_ref().is_some_and(|frame| frame.pending_break().is_some()) {
            self.runner.guard_mut().defuse();
            return self.event(LifecycleSignal::Finalize, None);
        }
        self.event(LifecycleSignal::Success, None)
    }

    /// Record the runtime state a new pass enters `start` with. A retried
    /// pass keeps the checkpoint it entered with.
    pub(super) fn enter_pass(&mut self) {
        if self.frame.is_some() && self.checkpoint.is_none() {
            self.checkpoint = Some(PassCheckpoint::take(&self.runtime));
        }
    }

    /// After a looping root's pass settled through `finalize`: run its gate,
    /// condition, cap, and action, and prepare the next pass's `start`.
    ///
    /// ## Errors
    ///
    /// The loop owner's failure; nothing more runs for the pass.
    #[allow(clippy::result_large_err)]
    pub(super) fn finish_pass(&mut self) -> std::result::Result<ContainerPassEnd, CompositionError> {
        let Self { frame, runner, stack, runtime, checkpoint, .. } = self;
        let frame = frame.as_mut().expect("only a looping root finishes passes");
        let ambient = frame.ambient();
        let state = container_state(stack.frontmatter, runtime);
        let gate = StackExecutionContext {
            err: None, frontmatter: &state, loop_values: Some(&ambient),
            ..stack.with_signal(LifecycleSignal::Loop)
        };
        let end = finish_container_pass(frame, runner.guard_mut(), &gate, runtime)?;
        if matches!(end, ContainerPassEnd::Continue) {
            runner.begin_next_pass();
            *checkpoint = None;
        }
        Ok(end)
    }

    pub(super) fn event(&mut self, mut signal: LifecycleSignal, mut error: Option<LifecycleErrorInfo>) -> Result<ContainerEventDecision> {
        if !matches!(signal, LifecycleSignal::Cancelled | LifecycleSignal::Finalize)
            && claudine::composition::lifecycle::cancellation::requested()
        {
            self.runner.cancel(claudine::composition::lifecycle::cancellation::cause()
                .unwrap_or(claudine::composition::lifecycle::cancellation::CancellationCause::Interrupt));
            signal = LifecycleSignal::Cancelled;
            error = self.runner.primary_error().cloned();
        }
        if let Some(error) = &error { self.runner.set_error(error.clone()); }
        loop {
            if let Some(live) = self.stack.live_frontmatter {
                let mut live = live.lock().expect("root frontmatter poisoned");
                live.extend(self.runtime.snapshot().mutations);
            }
            let disabled = claudine::composition::lifecycle::executor::DisabledShellRunner;
            let ambient = self.loop_values();
            let stack = StackExecutionContext { signal, err: error.as_ref(), shell_runner: if self.started { self.stack.shell_runner } else { &disabled },
                loop_values: ambient.as_ref().or(self.stack.loop_values), ..self.stack.with_signal(signal) };
            if matches!(signal, LifecycleSignal::Initialize | LifecycleSignal::Start) {
                claudine::composition::lifecycle::prewarm::collect_and_submit(
                    &self.prewarm, stack.settings, self.runner.guard_mut().config(), || {
                        claudine::composition::lifecycle::prewarm::PrewarmInputs::shared(
                            stack.live_frontmatter_snapshot().unwrap_or_else(|| stack.frontmatter.clone()),
                            stack.prepared_context.expect("root sequence has a captured context").clone(),
                        )
                    },
                );
            }
            let result = self.runner.execute_event(signal, &stack);
            if let Some(claudine::composition::lifecycle::executor::StackControl::Proxy { location, .. }) = &result.outcome.control {
                self.proxy_location = Some(location.clone());
            }
            match result.decision {
                ContainerEventDecision::Failure { error: raised } => {
                    error = error.or(raised);
                    signal = LifecycleSignal::Failure;
                }
                ContainerEventDecision::Finalize { error: raised, .. } => {
                    error = error.or(raised);
                    signal = LifecycleSignal::Finalize;
                }
                decision @ (ContainerEventDecision::Abort(_) | ContainerEventDecision::SetupRecoveryUnsupported { .. }) => {
                    let (Some(refusal), Some(RefusalRoute::FinalizeIfOwed)) = (decision.refusal(), self.runner.refusal_route(&decision)) else {
                        unreachable!("a root sequence finalizes only what a refusal leaves owed: {decision:?}");
                    };
                    let Some(typed) = CompositionError::from_lifecycle_refusal(&refusal, self.stack.source_path) else {
                        unreachable!("a container refuses only a setup recovery, a resume, or a defer: {refusal:?}");
                    };
                    // A refused recovery is a failure: the error is recorded,
                    // and a `finalize` the run already owes still runs.
                    self.runner.set_error(LifecycleErrorInfo::from_composition_error(&typed));
                    self.runner.execute_event(LifecycleSignal::Finalize, &stack.with_signal(LifecycleSignal::Finalize));
                    return Err(typed.into());
                }
                decision => {
                    if matches!(decision, ContainerEventDecision::Finished { .. }) && self.runner.is_cancelled() {
                        for error in self.runner.secondary_errors() {
                            self.stack.emitter.emit_warn(&error.msg, self.stack.term);
                        }
                    }
                    return Ok(decision);
                },
            }
        }
    }

    pub(super) fn retry(&mut self, delay: std::time::Duration, overlay: &indexmap::IndexMap<String, serde_json::Value>) -> Result<()> {
        self.started = false;
        // The failure being retried beat any `break` its pass fired.
        if let Some(frame) = &self.frame { frame.discard_break(); }
        // A looping root replaces only the failed attempt of this pass: its
        // outputs and runtime writes return to the pass's entry. Without a
        // loop the attempt's results reset and its writes survive.
        match (&self.checkpoint, self.frame.as_mut()) {
            (Some(checkpoint), Some(frame)) => {
                frame.retry_pass();
                checkpoint.restore(&self.runtime);
            }
            _ => self.runtime.clear_outputs(),
        }
        if let Err(error) = self.runtime.set_batch(self.stack.effect_engine, overlay, self.stack.frontmatter) {
            let info = LifecycleErrorInfo::from_error_or_action("retry", &error);
            self.event(LifecycleSignal::Blocked, Some(info))?;
            return Err(error.into());
        }
        self.child_error = None;
        if !delay.is_zero() { std::thread::sleep(delay); }
        Ok(())
    }
}

/// Body steps compose the container's source but do not enter its lifecycle again.
pub(super) fn mask_body_events(overrides: &mut claudine::composition::LayeredOverrides) {
    for key in claudine::composition::claudine_registry().authored_event_keys() {
        if key == "loop" { continue; }
        overrides.insert(darkmatter::markdown::compose::OverrideOrigin::Data, key.to_string(), serde_json::json!([]));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_proxy(
    source: &claudine::composition::ResolvedCompositionSource,
    shared: &crate::commands::compose::SharedComposeArgs,
    invocation: &claudine::invocation_context::InvocationContext,
    source_context: &claudine::invocation_context::SourceContext,
    resolution: &biscuit_file::FileResolutionContext,
    setters: Option<&serde_json::Value>,
    caller_input_records: &darkmatter::markdown::compose::CallerInputRecords,
    runtime: &Arc<RuntimeState>,
    location: claudine::composition::ActionLocation,
    target: String,
    overlay: indexmap::IndexMap<String, serde_json::Value>,
    verbose: u8,
    approved: Option<&darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals>,
) -> Result<i32> {
    use claudine::composition::{self, SurfacedHandoff};
    use crate::commands::compose::prep::{ActiveDocumentOutcome, StepScope, prepare_and_run_active_document, resolve_composition_source};
    use crate::commands::compose::CompositionKind;
    let cache = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let ledger = Arc::new(std::sync::Mutex::new(composition::RunLedger::new(source.resolved_path.clone(), Arc::clone(&cache))));
    let provenance = composition::ProxyProvenance::new(source.resolved_path.clone(),
        location, vec![source.resolved_path.clone()]);
    let mut handoff = SurfacedHandoff::Request(composition::EvaluatedProxyRequest::new(target, overlay, provenance));
    let overrides = composition::layered_set_overrides(composition::LayeredOverrides::authored(setters), Some(&runtime.snapshot()), None);
    let scope = StepScope { shell_approvals: approved.cloned(), caller_overrides: overrides.clone(), lifecycle_group: None, runtime_state: Arc::clone(runtime), suppress_output_commit: false,
        task_frame_writer: None, env_overrides: Default::default(), operation: shared.operation.clone(),
        // The root's own handoff reports no step status line.
        launch_log: None };
    let mut caller_overrides = Some(overrides.to_value());
    let mut records = caller_input_records.clone();
    let mut active_resolution = source_context.file_resolution_context().clone();
    loop {
        let committed = match handoff {
            SurfacedHandoff::Request(request) => composition::commit_proxy(&mut ledger.lock().unwrap(), request, &active_resolution)?,
            SurfacedHandoff::Committed(committed) => *committed,
        };
        let target_ref = committed.resolved_target().to_string_lossy().into_owned();
        let mut target_source = resolve_composition_source(&target_ref, CompositionKind::Direct, shared, resolution)?;
        super::super::overlay::merge_frontmatter_overlay(target_source.markdown.frontmatter_mut().as_map_mut(), committed.overlay());
        // Anchored at the source, whose `with` authored the values.
        let overlay = composition::AnchoredOverlay::new(committed.overlay().clone(), &active_resolution);
        let target_file = committed.authored_target().to_string();
        let document = invocation.capture_document();
        let context = document.derive_source(&target_source.resolved_path)?;
        active_resolution = context.file_resolution_context().clone();
        let (outcome, _) = prepare_and_run_active_document(shared, CompositionKind::Direct, target_source, &target_file,
            &overlay, Some(Box::new(committed)), false, &mut None, &shared.system_prompt_args(), &mut caller_overrides,
            &mut records, composition::InteractiveSchemaOptions::default(), &Some(resolution.request_cwd().to_path_buf()),
            &cache, &ledger, verbose, None, Vec::new(), std::time::Instant::now(), document, context, Some(&scope))?;
        match outcome {
            ActiveDocumentOutcome::Done { exit_code, .. } => return Ok(exit_code),
            ActiveDocumentOutcome::Handoff(next) => handoff = next,
        }
    }
}
