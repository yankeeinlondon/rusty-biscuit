//! Root sequence lifecycle. Child results stay in the invocation cell until cleanup settles.

use std::sync::Arc;
use claudine::composition::{CompositionError, RuntimeState};
use claudine::composition::lifecycle::{LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal};
use claudine::composition::lifecycle::context::LifecycleErrorInfo;
use claudine::composition::lifecycle::executor::StackExecutionContext;
use claudine::composition::lifecycle::runtime::{ContainerEventDecision, ContainerRunner, LifecycleTransitionAbort};
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
}

impl<'a> RootRun<'a> {
    pub(super) fn new(
        config: &'a claudine::composition::LifecycleConfig,
        context: &'a LifecycleRuntimeContext<'a>,
        stack: StackExecutionContext<'a>,
        runtime: Arc<RuntimeState>,
    ) -> Self {
        Self { runner: ContainerRunner::new(LifecycleRunGuard::new(config, context, stack.emitter)),
            stack, runtime, started: false, approved: None, graph: None, pending: None, child_error: None, proxy_location: None }
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
            let stack = StackExecutionContext { signal, err: error.as_ref(), shell_runner: if self.started { self.stack.shell_runner } else { &disabled }, ..self.stack.with_signal(signal) };
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
                ContainerEventDecision::Abort(LifecycleTransitionAbort::ResumeWithoutSession) => {
                    // Invalid recovery is a failure and still owes cleanup.
                    let typed = CompositionError::LifecycleResumeWithoutSession { source_path: self.stack.source_path.to_path_buf() };
                    self.runner.set_error(LifecycleErrorInfo::from_composition_error(&typed));
                    let stack = stack.with_signal(LifecycleSignal::Finalize);
                    self.runner.execute_event(LifecycleSignal::Finalize, &stack);
                    return Err(typed.into());
                }
                ContainerEventDecision::Abort(_) => {
                    let typed = CompositionError::LifecycleDeferNotImplemented { source_path: self.stack.source_path.to_path_buf() };
                    self.runner.set_error(LifecycleErrorInfo::from_composition_error(&typed));
                    self.runner.execute_event(LifecycleSignal::Finalize, &stack.with_signal(LifecycleSignal::Finalize));
                    return Err(typed.into());
                }
                ContainerEventDecision::SetupRecoveryUnsupported { event, action } => {
                    let typed = CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                        source_path: self.stack.source_path.to_path_buf(), event: format!("{event:?}").to_lowercase(), action: action.to_string(),
                    };
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
        self.runtime.clear_outputs();
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
    for key in claudine::composition::LIFECYCLE_EVENT_KEYS {
        if *key == "loop" { continue; }
        overrides.insert(darkmatter::markdown::compose::OverrideOrigin::Data, (*key).to_string(), serde_json::json!([]));
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
        task_frame_writer: None, env_overrides: Default::default(), operation: shared.operation.clone() };
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
        let overlay = committed.overlay().clone();
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
