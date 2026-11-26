//! Ordered nested containers keep their output accumulator lexical while
//! retaining invocation mutations and the outer invocation identity.

use crate::composition::LifecycleErrorInfoExt;
use std::sync::Arc;

use darkmatter::markdown::compose::{EffectiveStateBuilder, SharedFrontmatter};
use serde_json::{Map, Value};

use super::{PrimaryOutcome, TaskDiagnostic, TaskExecution, TaskStage, TaskStatus};
use crate::composition::error::CompositionError;
use crate::composition::lifecycle::{LifecycleRunGuard, LifecycleRunGuardExt, LifecycleRuntimeContext, LifecycleSignal};
use crate::composition::lifecycle::bindings::SequenceBinding;
use crate::composition::lifecycle::executor::StackExecutionContext;
use crate::composition::lifecycle::policy::{RefusalRoute, SubjectKind};
use crate::composition::lifecycle::runtime::{ContainerEventDecision, ContainerRunner, LifecycleTransitionAbort};
use crate::composition::looping::{ContainerPassEnd, LoopFrame, LoopResultKind, PassCheckpoint, finish_container_pass};
use crate::composition::runtime_state::RuntimeState;
use crate::composition::sequence::preflight::{PreflightAction, PreflightSequence};

impl TaskExecution<'_> {
    pub(super) fn run_sequence(&self, sequence: &PreflightSequence) -> PrimaryOutcome {
        if self.interrupted() { return PrimaryOutcome::interrupted(String::new()); }
        let owned_runtime = Arc::new(RuntimeState::new_sequence());
        let runtime = self.runtime.unwrap_or(&owned_runtime);
        // A referenced document's `initialize` reads its prepared values
        // before its schema has judged them: that event may still supply or
        // correct a value, and the verdict waits for `start`.
        let base = match self.referenced_owner_values(sequence, runtime, false) {
            Ok(base) => base,
            Err(error) => return PrimaryOutcome::failed(TaskDiagnostic::from_composition(TaskStage::Primary, &error)),
        };
        // The enclosing scope moves out and back; its outputs are never copied.
        let (outer_seq, outer_outputs) = runtime.replace_sequence_scope(None, Vec::new());
        // The position `sequence_binding` reads: the runtime's, else the overlay's.
        let run_id = outer_seq.as_ref().or_else(|| self.overlay.and_then(|overlay| overlay.get("seq")))
            .and_then(|seq| seq.get("run_id")).and_then(Value::as_str)
            .unwrap_or(&sequence.plan.sequence_id).to_string();
        runtime.set_sequence_binding(SequenceBinding::container(&run_id, Vec::new()).to_value());
        let runtime_context = LifecycleRuntimeContext {
            settings: self.stack.settings, messaging: self.stack.messaging, term: self.stack.term,
            source_path: &sequence.origin_path, repo_root: self.stack.repo_root,
            launch_area: self.stack.ctx_base_dir, context: self.stack.prepared_context,
        };
        let guard = LifecycleRunGuard::new(&sequence.lifecycle, &runtime_context, self.stack.emitter);
        let mut runner = ContainerRunner::new(guard).with_subject(SubjectKind::Sequence);
        let state = base.clone().unwrap_or_else(|| self.stack.frontmatter.clone());
        // The container's events, gate, loop cap, proxy, and children read its
        // own document: its path and that document's context travel together.
        // Its reread (`prepare_nested_sequence`) resolves the reference from
        // the calling task's, so keeps `self.stack`'s.
        let result = match self.authoring_context(&sequence.origin_path) {
            Ok(own_context) => {
                let own_stack = StackExecutionContext {
                    source_path: &sequence.origin_path,
                    file_resolution_context: &own_context,
                    ..self.stack.with_signal(self.stack.signal)
                };
                match (TaskExecution { stack: &own_stack, ..*self }).open_loop(sequence.loop_config.as_ref(), LoopResultKind::Sequence, &state, runtime) {
                    Ok(mut frame) => self.run_sequence_attempts(sequence, runtime, &run_id, &mut runner, frame.as_mut(), OwnDocument { stack: &own_stack, base }),
                    Err(error) => PrimaryOutcome::failed(TaskDiagnostic::from_composition(TaskStage::Primary, &error)),
                }
            }
            Err(error) => PrimaryOutcome::failed(TaskDiagnostic::from_composition(TaskStage::Primary, &error)),
        };
        runtime.replace_sequence_scope(outer_seq, outer_outputs);
        result
    }

    fn run_sequence_attempts(
        &self, sequence: &PreflightSequence, runtime: &Arc<RuntimeState>, run_id: &str,
        runner: &mut ContainerRunner<'_>, mut frame: Option<&mut LoopFrame>,
        own: OwnDocument<'_, '_>,
    ) -> PrimaryOutcome {
        let OwnDocument { stack: own_stack, mut base } = own;
        let file_resolution_context = own_stack.file_resolution_context;
        let mut signal = LifecycleSignal::Initialize;
        // Taken as each pass enters `start`; a retry of the pass returns here.
        let mut checkpoint = PassCheckpoint::take(runtime);
        let mut shell_enabled = false;
        let mut error = None;
        let mut secondary_errors = Vec::new();
        let mut interrupted_stdout = String::new();
        let mut prepared = None;
        // A referenced document's events share their own live state, as a
        // directly run document's events do, rather than the caller's.
        let own_live = sequence.frontmatter.is_some().then(|| std::sync::Mutex::new(SharedFrontmatter::default()));
        let live_frontmatter = own_live.as_ref().or(self.stack.live_frontmatter);
        loop {
            // The `loop` values this container's events and children read:
            // its own loop's, else the enclosing loop's.
            let own_ambient = frame.as_deref().map(LoopFrame::ambient);
            let loop_values = own_ambient.as_ref().or(self.stack.loop_values);
            if !matches!(signal, LifecycleSignal::Cancelled | LifecycleSignal::Finalize)
                && self.interrupted()
            {
                runner.cancel(crate::composition::lifecycle::cancellation::cause()
                    .unwrap_or(crate::composition::lifecycle::cancellation::CancellationCause::Interrupt));
                error = runner.primary_error().cloned();
                signal = LifecycleSignal::Cancelled;
            }
            if signal == LifecycleSignal::Start {
                let context = self.stack.prepared_context.cloned().unwrap_or_else(|| self.state.context().clone());
                let mut supplied = match self.user_setters { Some(Value::Object(setters)) => setters.clone(), _ => Map::new() };
                supplied.extend(runtime.mutations());
                match crate::composition::sequence::preflight::prepare_nested_sequence(
                    self.task, sequence, self.state, supplied, context, self.stack.file_resolution_context,
                    self.stack.group.is_some(), self.graph.nested_sources(), self.document,
                ).and_then(|(fresh, graph)| {
                    self.verify_nested_graph(&graph)?;
                    // Each entry judges the check as it reads now (a reread
                    // may have changed it) against the values of this entry,
                    // which on the first entry include `initialize`'s writes
                    // (admission judged the values before it).
                    self.check_inline_container(&fresh, Some(runtime))?;
                    // A whole referenced document is judged by its own
                    // schema against the values of this entry, as a directly
                    // run document is at each step.
                    let values = self.referenced_owner_values(&fresh, runtime, true)?;
                    Ok((fresh, values))
                }) {
                    Ok((fresh, values)) => {
                        base = values;
                        runner.guard_mut().set_config((*fresh.lifecycle).clone());
                        prepared = Some(fresh);
                        shell_enabled = true;
                    }
                    Err(cause) => {
                        let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &cause);
                        error = Some(diagnostic.info);
                        runner.set_error(error.as_ref().unwrap().clone());
                        signal = LifecycleSignal::Blocked;
                    }
                }
            }
            if signal == LifecycleSignal::Start && self.interrupted() {
                runner.cancel(crate::composition::lifecycle::cancellation::cause()
                    .unwrap_or(crate::composition::lifecycle::cancellation::CancellationCause::Interrupt));
                error = runner.primary_error().cloned();
                signal = LifecycleSignal::Cancelled;
            }
            let sequence = prepared.as_ref().unwrap_or(sequence);
            let (mutations, outputs) = runtime.mutations_and_outputs();
            let seq = SequenceBinding::container(run_id, outputs).to_value();
            let mut event_frontmatter = base.clone().unwrap_or_else(|| self.stack.frontmatter.clone());
            event_frontmatter.extend(mutations.clone());
            if let Some(live) = &own_live {
                layer_onto(&mut live.lock().expect("sequence live frontmatter poisoned"), &event_frontmatter);
            }
            if let Some(live) = self.stack.live_frontmatter {
                live.lock().expect("sequence live frontmatter poisoned").extend(mutations);
            }
            let disabled = crate::composition::lifecycle::executor::DisabledShellRunner;
            let stack = StackExecutionContext {
                shell_runner: if shell_enabled { self.stack.shell_runner } else { &disabled },
                seq: Some(&seq), runtime_state: Some(runtime.as_ref()), frontmatter: &event_frontmatter,
                live_frontmatter, source_path: &sequence.origin_path, err: error.as_ref(), loop_values,
                file_resolution_context,
                ..self.stack.with_signal(signal)
            };
            if matches!(signal, LifecycleSignal::Initialize | LifecycleSignal::Start) {
                self.prewarm_container(runner.guard_mut().config(), &event_frontmatter);
            }
            let event = runner.execute_event(signal, &stack);
            if error.is_none() {
                error = event.outcome.evaluation_error.clone().or_else(|| event.outcome.action_error.clone());
            }
            match event.decision {
                ContainerEventDecision::Continue if signal == LifecycleSignal::Initialize => {
                    checkpoint = PassCheckpoint::take(runtime);
                    signal = LifecycleSignal::Start;
                }
                ContainerEventDecision::Continue => {
                    let mut plan = sequence.plan.clone();
                    plan.sequence_id = run_id.to_string();
                    for step in &mut plan.steps { step.state.sequence_id = run_id.to_string(); }
                    error = None;
                    for (index, step) in sequence.steps.iter().enumerate() {
                        if self.interrupted() {
                            signal = LifecycleSignal::Cancelled;
                            break;
                        }
                        // A pending `break` starts no later child: neither in
                        // this container nor in any between it and its loop.
                        if loop_values.is_some_and(|ambient| ambient.breaks.is_pending()) {
                            break;
                        }
                        let overlay = crate::composition::sequence::build_step_overlay(&plan, index);
                        let (mutations, outputs) = runtime.mutations_and_outputs();
                        let seq = SequenceBinding::step(&overlay, outputs).to_value();
                        runtime.set_sequence_binding(seq.clone());
                        let mut data = base.clone().unwrap_or_else(|| self.state.data().clone());
                        data.extend(mutations);
                        let state = match EffectiveStateBuilder::new().with_shared_frontmatter(data)
                            .with_context(self.state.context().clone()).with_allow_ctx_override(true).build()
                        {
                            Ok(state) => state,
                            Err(source) => {
                                let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary,
                                    &CompositionError::PreFlightStateBuildFailed { source });
                                runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info);
                                break;
                            },
                        };
                        let scope_overlay = Value::Object(Map::from_iter([("seq".into(), seq.clone())]));
                        let body_task;
                        let task = match &step.task {
                            Some(task) => task,
                            None => {
                                body_task = {
                                    let mut task = self.task.clone();
                                    task.name = Some(step.name.clone());
                                    task.label = format!("{} → {}", self.task.label, step.name);
                                    task.action = PreflightAction::Prompt {
                                        path: sequence.origin_path.clone(), reference: sequence.origin_path.display().to_string(),
                                    };
                                    task.with.clear(); task.setup = None; task.teardown = None;
                                    task.shell_sites.clear(); task.preflight_approvals.clear();
                                    task.identity = self.task.identity.sequence_child(index);
                                    task.provider = None;
                                    task
                                };
                                &body_task
                            }
                        };
                        let run = self.prompt.open_member_run(task);
                        let child_stack = StackExecutionContext {
                            seq: Some(&seq), runtime_state: Some(runtime.as_ref()),
                            frontmatter: state.data(), live_frontmatter, source_path: &task.origin_path, loop_values,
                            prepared_context: run.as_ref().map(|opened| &opened.context).or(self.stack.prepared_context),
                            file_resolution_context,
                            ..self.stack.with_signal(LifecycleSignal::Start)
                        };
                        let child = TaskExecution {
                            task, runtime: Some(runtime), state: &state, stack: &child_stack,
                            overlay: Some(&scope_overlay),
                            document: run.as_ref().and_then(|opened| opened.document.as_ref()).or(self.document),
                            run_evidence: run.as_ref().map(|opened| &opened.run).or(self.run_evidence),
                            ..*self
                        };
                        let outcome = if step.task.is_none() {
                            let mut request = match child.prompt_request(&sequence.origin_path, &sequence.origin_path.display().to_string()) {
                                Ok(request) => request,
                                Err(failure) => {
                                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &failure);
                                    runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info);
                                    break;
                                },
                            };
                            request.body_only = true;
                            match self.prompt.validate(&request) {
                                Ok(()) => {
                                    let primary = child.run_prompt_request(&request);
                                    match primary.result {
                                        super::PrimaryResult::Succeeded => {
                                            runtime.append_output_entry(Value::String(primary.stdout));
                                            continue;
                                        }
                                        super::PrimaryResult::Failed(diagnostic) => {
                                            error = Some(diagnostic.info);
                                            runner.set_error(error.as_ref().unwrap().clone());
                                            break;
                                        }
                                        super::PrimaryResult::Interrupted => { interrupted_stdout = primary.stdout; signal = LifecycleSignal::Cancelled; break; },
                                    }
                                }
                                Err(failure) => {
                                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &failure);
                                    runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info);
                                    break;
                                },
                            }
                        } else { child.run() };
                        if outcome.status == TaskStatus::Interrupted {
                            interrupted_stdout = outcome.stdout;
                            secondary_errors.extend(outcome.secondary_errors);
                            signal = LifecycleSignal::Cancelled;
                            break;
                        }
                        secondary_errors.extend(outcome.secondary_errors);
                        if outcome.status == TaskStatus::Failed {
                            error = outcome.error.map(|diagnostic| diagnostic.info);
                            if let Some(error) = &error { runner.set_error(error.clone()); }
                            break;
                        }
                    }
                    runtime.set_sequence_binding(SequenceBinding::container(run_id, Vec::new()).to_value());
                    if signal == LifecycleSignal::Cancelled {
                        runner.cancel(crate::composition::lifecycle::cancellation::cause()
                            .unwrap_or(crate::composition::lifecycle::cancellation::CancellationCause::Interrupt));
                        error = runner.primary_error().cloned();
                    } else {
                        // The pass's own slice of the cumulative outputs is its
                        // result: its terminal event, `finalize`, and gate read it.
                        if let Some(frame) = frame.as_deref_mut() {
                            let pass = runtime.outputs_from(checkpoint.outputs());
                            frame.record_pass(Value::Array(pass), i32::from(error.is_some()));
                        }
                        let breaking = loop_values.filter(|ambient| ambient.breaks.is_pending());
                        signal = match (&error, breaking) {
                            // A failure beats a pending `break`.
                            (Some(_), breaking) => {
                                if let Some(ambient) = breaking { ambient.breaks.discard(); }
                                LifecycleSignal::Failure
                            }
                            // A `break` fires no further event but `finalize`.
                            (None, Some(_)) => {
                                runner.guard_mut().defuse();
                                LifecycleSignal::Finalize
                            }
                            (None, None) => LifecycleSignal::Success,
                        };
                    }
                }
                ContainerEventDecision::Skip => {
                    return PrimaryOutcome::succeeded(String::new());
                }
                ContainerEventDecision::Failure { error: raised } => {
                    error = raised; signal = LifecycleSignal::Failure;
                }
                ContainerEventDecision::Finalize { error: raised, .. } => {
                    error = raised.or(error); signal = LifecycleSignal::Finalize;
                }
                ContainerEventDecision::Finished { .. } if runner.is_cancelled() => {
                    secondary_errors.extend(runner.secondary_errors().iter().cloned().map(|info|
                        TaskDiagnostic { stage: TaskStage::Primary, info }));
                    return PrimaryOutcome::interrupted(interrupted_stdout).with_secondary(secondary_errors);
                }
                ContainerEventDecision::Finished { failed: false } => {
                    if let Some(frame) = frame.as_deref_mut() {
                        let ambient = frame.ambient();
                        let mut gate_frontmatter = base.clone().unwrap_or_else(|| self.stack.frontmatter.clone());
                        let (mutations, outputs) = runtime.mutations_and_outputs();
                        gate_frontmatter.extend(mutations);
                        let gate_seq = SequenceBinding::container(run_id, outputs).to_value();
                        let gate = StackExecutionContext {
                            seq: Some(&gate_seq), runtime_state: Some(runtime.as_ref()), frontmatter: &gate_frontmatter,
                            live_frontmatter, source_path: &sequence.origin_path, err: None, loop_values: Some(&ambient),
                            file_resolution_context,
                            ..self.stack.with_signal(LifecycleSignal::Loop)
                        };
                        match finish_container_pass(frame, runner.guard_mut(), &gate, runtime) {
                            // The break ends this loop: its pass's completed
                            // results publish, and the parent continues.
                            Ok(ContainerPassEnd::Broken(broken)) => {
                                self.stack.emitter.emit_info(&broken.summary(), self.stack.term);
                            }
                            Ok(ContainerPassEnd::Continue) => {
                                runner.begin_next_pass();
                                checkpoint = PassCheckpoint::take(runtime);
                                error = None; secondary_errors.clear(); signal = LifecycleSignal::Start;
                                continue;
                            }
                            Ok(ContainerPassEnd::Complete) => {}
                            // The loop owner failed after the pass settled:
                            // nothing more runs and nothing is published.
                            Err(failure) => {
                                return PrimaryOutcome::failed(TaskDiagnostic::from_composition(TaskStage::Primary, &failure))
                                    .with_secondary(secondary_errors);
                            }
                        }
                    }
                    return PrimaryOutcome::succeeded(String::new())
                        .with_secondary(secondary_errors)
                        .with_group_output(runtime.outputs_value());
                }
                ContainerEventDecision::Finished { failed: true } => {
                    // A failure beats a pending `break`, including one its own
                    // events or cleanup fired.
                    if let Some(ambient) = loop_values { ambient.breaks.discard(); }
                    let info = runner.primary_error().cloned().or(error).unwrap_or_else(|| crate::composition::lifecycle::context::LifecycleErrorInfo::from_action_failure(
                        "sequence", "nested sequence failed",
                    ));
                    secondary_errors.extend(runner.secondary_errors().iter().cloned().map(|info|
                        TaskDiagnostic { stage: TaskStage::Primary, info }));
                    return PrimaryOutcome::failed(TaskDiagnostic { stage: TaskStage::Primary, info })
                        .with_secondary(secondary_errors)
                        .with_group_output(runtime.outputs_value());
                }
                ContainerEventDecision::Retry { delay, overlay } => {
                    if !delay.is_zero() { std::thread::sleep(delay); }
                    let state = base.clone().unwrap_or_else(|| self.state.data().clone());
                    // A looping container replaces only the failed attempt
                    // of this pass: its outputs and runtime writes return to
                    // the pass's entry. Without a loop the attempt's results
                    // reset and its writes survive.
                    match frame.as_deref_mut() {
                        Some(frame) => {
                            frame.retry_pass();
                            checkpoint.restore(runtime);
                        }
                        None => runtime.clear_outputs(),
                    }
                    if let Err(failure) = runtime.set_batch(self.stack.effect_engine, &overlay, &state) {
                        let diagnostic = TaskDiagnostic { stage: TaskStage::Primary,
                            info: crate::composition::lifecycle::context::LifecycleErrorInfo::from_error_or_action("retry", &failure) };
                        runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info);
                        signal = LifecycleSignal::Blocked;
                        continue;
                    }
                    error = None; secondary_errors.clear(); signal = LifecycleSignal::Start;
                }
                // A `defer` is not reported as a refusal here: it falls to the
                // unsupported-recovery arm below.
                decision @ (ContainerEventDecision::Abort(LifecycleTransitionAbort::ResumeWithoutSession)
                | ContainerEventDecision::SetupRecoveryUnsupported { .. }) => {
                    let (Some(refusal), Some(RefusalRoute::FailureThenFinalize)) = (decision.refusal(), runner.refusal_route(&decision)) else {
                        unreachable!("a nested sequence fails a refused recovery through its catch events: {decision:?}");
                    };
                    let Some(failure) = CompositionError::from_lifecycle_refusal(&refusal, &sequence.origin_path) else {
                        unreachable!("a resume without a session and a setup recovery map to errors");
                    };
                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &failure);
                    error = Some(diagnostic.info);
                    runner.set_error(error.as_ref().unwrap().clone());
                    signal = if signal == LifecycleSignal::Failure { LifecycleSignal::Finalize } else { LifecycleSignal::Failure };
                }
                ContainerEventDecision::Proxy { target, overlay } => {
                    return (TaskExecution { stack: own_stack, ..*self }).run_container_proxy(&target, overlay);
                }
                decision => {
                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary,
                        &CompositionError::SequenceTaskUnsupported {
                            task: self.label(), construct: "nested container recovery".into(), detail: format!("{decision:?}"),
                            site: Box::new(self.task.error_site()),
                        });
                    error = Some(diagnostic.info);
                    runner.set_error(error.as_ref().unwrap().clone());
                    signal = if signal == LifecycleSignal::Failure { LifecycleSignal::Finalize } else { LifecycleSignal::Failure };
                }
            }
        }
    }
}

impl TaskExecution<'_> {
    /// Judge `sequence`'s inline `$schema` check, when it declares one,
    /// against the values visible at its turn: the enclosing values, the
    /// caller's setters, and every runtime write so far.
    ///
    /// The check is passive: the converted or defaulted copy it judges is
    /// discarded, and nothing it reads is changed.
    ///
    /// ## Errors
    ///
    /// [`CompositionError::SequenceContainerSchemaViolation`] for the first
    /// problem.
    pub(super) fn check_inline_container(
        &self, sequence: &PreflightSequence, runtime: Option<&RuntimeState>,
    ) -> Result<(), CompositionError> {
        self.judge_inline_check(sequence.inline_schema.as_ref(), runtime)
    }

    /// Admit `sequence` at this task's turn: judge its inline `$schema` as
    /// the declaring file reads now, not as the approved graph froze it, so a
    /// declaration an earlier task edited refuses the container before its
    /// `initialize` runs.
    ///
    /// ## Errors
    ///
    /// As [`crate::composition::sequence::preflight::reread_inline_check`]
    /// for a declaration that no longer reads, and as
    /// [`Self::check_inline_container`] for values it refuses.
    pub(super) fn admit_inline_container(
        &self, sequence: &PreflightSequence, runtime: Option<&RuntimeState>,
    ) -> Result<(), CompositionError> {
        let current = crate::composition::sequence::preflight::reread_inline_check(
            self.task, sequence, self.stack.file_resolution_context, self.graph.nested_sources(), self.document,
        )?;
        self.judge_inline_check(current.as_ref(), runtime)
    }

    fn judge_inline_check(
        &self, check: Option<&crate::composition::sequence::declarations::PreparedSequenceSchema>,
        runtime: Option<&RuntimeState>,
    ) -> Result<(), CompositionError> {
        let Some(check) = check else { return Ok(()) };
        let container = check.property().strip_suffix(".$schema").unwrap_or(check.property());
        let mut snapshot: Map<String, Value> = self.state.data().iter()
            .map(|(key, value)| (key.clone(), value.clone())).collect();
        if let Some(Value::Object(setters)) = self.user_setters {
            snapshot.extend(setters.clone());
        }
        if let Some(runtime) = runtime {
            snapshot.extend(runtime.mutations());
        }
        let Some(report) = check.validate(&Value::Object(snapshot)) else { return Ok(()) };
        match report.problems.first() {
            None => Ok(()),
            Some(problem) => Err(CompositionError::SequenceContainerSchemaViolation {
                source_path: check.source_path().to_path_buf(),
                container: container.to_string(),
                property: match &problem.property {
                    Some(name) if problem.path.is_empty() => name.clone(),
                    _ => problem.path.clone(),
                },
                message: problem.message.clone(),
            }),
        }
    }

    /// The base state of a whole referenced sequence document's events and
    /// children: the document's prepared values (its own properties, coerced,
    /// defaulted and derived by its `$schema`), under the invocation's caller
    /// inputs and then every runtime write so far. `None` for an inline list
    /// or a step selection, which read the enclosing state.
    ///
    /// With `verdict`, the document's schema judges those values, which is
    /// what a direct run does at each of its steps; without, they are only
    /// prepared. Runtime writes made after this call layer above the result
    /// at each read, unprepared: no check runs between children of one pass.
    ///
    /// ## Errors
    ///
    /// The document's preparation errors, located in the document:
    /// [`CompositionError::SchemaValidation`] or
    /// [`CompositionError::MissingProperties`] for a value its schema refuses.
    fn referenced_owner_values(
        &self, sequence: &PreflightSequence, runtime: &RuntimeState, verdict: bool,
    ) -> Result<Option<SharedFrontmatter>, CompositionError> {
        if sequence.frontmatter.is_none() { return Ok(None) }
        let mut options = crate::composition::PrepareOptions::new(self.stack.file_resolution_context.clone());
        match self.graph.caller_layers() {
            Some(layers) => options = layers.apply_to(options),
            None => options.set_overrides = self.user_setters.cloned(),
        }
        let mut overrides = options.layered_overrides();
        for (key, value) in runtime.mutations() {
            overrides.insert(darkmatter::markdown::compose::OverrideOrigin::Data, key, value);
        }
        options.set_layered_overrides(overrides);
        options.prepared_context = Some(self.stack.prepared_context.cloned().unwrap_or_else(|| self.state.context().clone()));
        options.defer_schema_verdict = !verdict;
        let prepared = self.graph.prepare_whole_document(&sequence.origin_path, options, self.document)?;
        Ok(Some(prepared.values.into()))
    }

    pub(super) fn verify_nested_graph(&self, fresh: &crate::composition::sequence::preflight::PreflightGraph) -> Result<(), CompositionError> {
        let unapproved = fresh.shell_commands.iter().find(|command| !self.graph.contains_shell_command(&command.command));
        if let Some(command) = unapproved {
            return Err(CompositionError::SequenceShellResolution {
                command: command.command.clone(), task: self.label(),
                message: "nested source introduced a command after sequence approval closed".into(), source: None,
                site: Box::new(self.task.error_site()),
            });
        }
        for candidate in &fresh.template_candidates {
            if self.graph.candidates_authored(candidate.authored()).next().is_none()
                || self.graph.shell_authorization().is_some_and(|(approved, _)| !candidate.authorized_by(&approved)) {
                return Err(CompositionError::SequenceShellResolution {
                    command: candidate.authored().unwrap_or("").into(), task: self.label(),
                    message: "nested source introduced a template after sequence approval closed".into(), source: None,
                    site: Box::new(self.task.error_site()),
                });
            }
        }
        for document in &fresh.prompt_documents {
            if self.graph.prompt_document(&document.path).is_none() {
                return Err(CompositionError::sequence_invalid("nested source introduced an unapproved prompt document", self.task.error_site()));
            }
        }
        if let Some(authorization) = self.graph.shell_authorization() {
            fresh.settle_container_events(&authorization.0);
        }
        Ok(())
    }

    pub(super) fn run_container_proxy(
        &self, target: &str, overlay: indexmap::IndexMap<String, Value>,
    ) -> PrimaryOutcome {
        let result = (|| {
            let source = match &self.task.action {
                PreflightAction::Sequence(sequence) => &sequence.origin_path,
                PreflightAction::Group(group) => &group.origin_path,
                _ => &self.task.origin_path,
            };
            // A container proxy's target is a container document.
            let path = crate::composition::sequence::source::resolve_sequence_reference(
                target, crate::composition::sequence::model::ExecutableField::Sequence, source, self.stack.file_resolution_context,
            )
            .map_err(|error| error.locate_sequence(source, "", Some(&self.label())))?;
            let path = biscuit_file::canonicalize_simplified(&path).unwrap_or(path);
            let owned_runtime = Arc::new(RuntimeState::new_sequence());
            let runtime = self.runtime.unwrap_or(&owned_runtime);
            let _chain = runtime.enter_container_proxy(source, &path, target)?;
            if let Some(runtime) = self.runtime
                && let Err(error) = runtime.set_batch(self.stack.effect_engine, &overlay, self.state.data())
            {
                return Ok(PrimaryOutcome::failed(TaskDiagnostic { stage: TaskStage::Primary,
                    info: crate::composition::lifecycle::context::LifecycleErrorInfo::from_error_or_action("proxy", &error) }));
            }
            if let Some(sequence) = approved_sequence(&self.graph.steps, &path) {
                let mut task = self.task.clone();
                task.action = PreflightAction::Sequence(Box::new(sequence.clone()));
                task.origin_path = sequence.origin_path.clone();
                task.origin_dir = sequence.origin_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
                task.authored = Default::default();
                // A proxy enters its container through the same admission as
                // a task's turn, never around it. Its target is a file, whose
                // container has no authored task here to reread a check at,
                // so the approved check judges.
                let execution = TaskExecution { task: &task, runtime: Some(runtime), ..*self };
                execution.check_inline_container(sequence, Some(runtime))?;
                return Ok(execution.run_sequence(sequence));
            }
            if self.graph.prompt_document(&path).is_none() {
                return Err(CompositionError::SequenceTaskUnsupported {
                    task: self.label(), construct: "container proxy".into(),
                    detail: format!("target `{target}` was not approved during sequence preflight"),
                    site: Box::new(self.task.error_site()),
                });
            }
            let mut task = self.task.clone();
            task.action = PreflightAction::Prompt { path: path.clone(), reference: target.into() };
            task.with.clear(); task.setup = None; task.teardown = None;
            let execution = TaskExecution { task: &task, ..*self };
            let request = execution.prompt_request(&path, target)?;
            self.prompt.validate(&request)?;
            Ok(execution.run_prompt_request(&request))
        })();
        result.unwrap_or_else(|error| PrimaryOutcome::failed(TaskDiagnostic::from_composition(TaskStage::Primary, &error)))
    }
}

fn approved_sequence<'a>(steps: &'a [crate::composition::sequence::preflight::PreflightStep], path: &std::path::Path) -> Option<&'a PreflightSequence> {
    fn find<'a>(task: &'a crate::composition::sequence::preflight::PreflightTask, path: &std::path::Path) -> Option<&'a PreflightSequence> {
        match &task.action {
            PreflightAction::Sequence(sequence) if sequence.origin_path == path => Some(sequence),
            PreflightAction::Sequence(sequence) => approved_sequence(&sequence.steps, path),
            PreflightAction::Group(group) => group.tasks.iter().find_map(|task| find(task, path)),
            _ => None,
        }
    }
    steps.iter().filter_map(|step| step.task.as_ref()).find_map(|task| find(task,path))
}

/// Write every entry of `values` into `live`, as [`Extend`] would, copying
/// only the entries whose value `live` does not already hold.
///
/// A referenced document's own live cell receives its whole event state at
/// each event; the first write shares that state's base, and a later one
/// writes only what changed, so no event deep-copies the document.
fn layer_onto(live: &mut SharedFrontmatter, values: &SharedFrontmatter) {
    if live.is_empty() {
        *live = values.clone();
        return;
    }
    for (key, value) in values.iter() {
        if live.get(key).is_none_or(|current| !std::ptr::eq(current, value) && current != value) {
            live.insert(key.clone(), value.clone());
        }
    }
}

/// What a sequence container reads from its own document: the stack its
/// events, gate, proxy, and children run in, and the document's prepared
/// values (`None` for an inline list, which reads the caller's).
struct OwnDocument<'s, 'c> {
    stack: &'s StackExecutionContext<'c>,
    base: Option<SharedFrontmatter>,
}
