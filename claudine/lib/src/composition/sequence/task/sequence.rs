//! Ordered nested containers keep their output accumulator lexical while
//! retaining invocation mutations and the outer invocation identity.

use std::sync::Arc;

use darkmatter::markdown::compose::EffectiveStateBuilder;
use serde_json::{Map, Value};

use super::{PrimaryOutcome, TaskDiagnostic, TaskExecution, TaskStage, TaskStatus};
use crate::composition::error::CompositionError;
use crate::composition::lifecycle::{LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal};
use crate::composition::lifecycle::bindings::SequenceBinding;
use crate::composition::lifecycle::executor::StackExecutionContext;
use crate::composition::lifecycle::runtime::{ContainerEventDecision, ContainerRunner, LifecycleTransitionAbort};
use crate::composition::runtime_state::RuntimeState;
use crate::composition::sequence::preflight::{PreflightAction, PreflightSequence};

impl TaskExecution<'_> {
    pub(super) fn run_sequence(&self, sequence: &PreflightSequence) -> PrimaryOutcome {
        if self.interrupted() { return PrimaryOutcome::interrupted(String::new()); }
        let owned_runtime = Arc::new(RuntimeState::new());
        let runtime = self.runtime.unwrap_or(&owned_runtime);
        let outer = runtime.snapshot();
        let run_id = self.sequence_binding().get("run_id").and_then(Value::as_str)
            .unwrap_or(&sequence.plan.sequence_id).to_string();
        runtime.replace_sequence_scope(Some(SequenceBinding::container(&run_id, Vec::new()).to_value()), Vec::new());
        let runtime_context = LifecycleRuntimeContext {
            settings: self.stack.settings, messaging: self.stack.messaging, term: self.stack.term,
            source_path: &sequence.origin_path, repo_root: self.stack.repo_root,
            launch_area: self.stack.ctx_base_dir, context: self.stack.prepared_context,
        };
        let guard = LifecycleRunGuard::new(&sequence.lifecycle, &runtime_context, self.stack.emitter);
        let mut runner = ContainerRunner::new(guard);
        let result = self.run_sequence_attempts(sequence, runtime, &run_id, &mut runner);
        runtime.replace_sequence_scope(outer.seq, outer.outputs);
        result
    }

    fn run_sequence_attempts(
        &self, sequence: &PreflightSequence, runtime: &Arc<RuntimeState>, run_id: &str,
        runner: &mut ContainerRunner<'_>,
    ) -> PrimaryOutcome {
        let mut signal = LifecycleSignal::Initialize;
        let mut shell_enabled = false;
        let mut error = None;
        let mut secondary_errors = Vec::new();
        let mut interrupted_stdout = String::new();
        let mut prepared = None;
        // A referenced document's events share their own live state, as a
        // directly run document's events do, rather than the caller's.
        let own_live = sequence.frontmatter.is_some().then(|| std::sync::Mutex::new(Map::new()));
        let live_frontmatter = own_live.as_ref().or(self.stack.live_frontmatter);
        loop {
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
                supplied.extend(runtime.snapshot().mutations);
                match crate::composition::sequence::preflight::prepare_nested_sequence(
                    self.task, sequence, self.state, supplied, context, self.stack.file_resolution_context,
                    self.stack.group.is_some(),
                ).and_then(|(fresh, graph)| {
                    self.verify_nested_graph(&graph)?;
                    Ok(fresh)
                }) {
                    Ok(fresh) => {
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
            let seq = SequenceBinding::container(run_id, runtime.snapshot().outputs).to_value();
            let base = self.sequence_frontmatter(sequence);
            let mut event_frontmatter = base.clone().unwrap_or_else(|| self.stack.frontmatter.clone());
            event_frontmatter.extend(runtime.snapshot().mutations);
            if let Some(live) = &own_live {
                live.lock().expect("sequence live frontmatter poisoned").extend(event_frontmatter.clone());
            }
            if let Some(live) = self.stack.live_frontmatter {
                live.lock().expect("sequence live frontmatter poisoned").extend(runtime.snapshot().mutations);
            }
            let disabled = crate::composition::lifecycle::executor::DisabledShellRunner;
            let stack = StackExecutionContext {
                shell_runner: if shell_enabled { self.stack.shell_runner } else { &disabled },
                seq: Some(&seq), runtime_state: Some(runtime.as_ref()), frontmatter: &event_frontmatter,
                live_frontmatter, source_path: &sequence.origin_path, err: error.as_ref(),
                ..self.stack.with_signal(signal)
            };
            let event = if self.prompt.is_dry_run() {
                let outcome = Default::default();
                let decision = runner.record_outcome(signal, &outcome);
                crate::composition::lifecycle::runtime::ContainerEventResult { outcome, decision }
            } else { runner.execute_event(signal, &stack) };
            if error.is_none() {
                error = event.outcome.evaluation_error.clone().or_else(|| event.outcome.action_error.clone());
            }
            match event.decision {
                ContainerEventDecision::Continue if signal == LifecycleSignal::Initialize => {
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
                        let overlay = crate::composition::sequence::build_step_overlay(&plan, index);
                        let seq = SequenceBinding::step(&overlay, runtime.snapshot().outputs).to_value();
                        runtime.set_sequence_binding(seq.clone());
                        let mut data = base.clone().map_or_else(|| self.state.data().clone(), |base| base.into_iter().collect());
                        data.extend(runtime.snapshot().mutations);
                        let state = match EffectiveStateBuilder::new().with_frontmatter(data)
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
                                    task.params.clear(); task.setup = None; task.teardown = None;
                                    task.shell_sites.clear(); task.preflight_approvals.clear();
                                    task
                                };
                                &body_task
                            }
                        };
                        let run = self.prompt.open_member_run(task);
                        let frontmatter = state.data().iter().map(|(key,value)| (key.clone(), value.clone())).collect();
                        let child_stack = StackExecutionContext {
                            seq: Some(&seq), runtime_state: Some(runtime.as_ref()),
                            frontmatter: &frontmatter, live_frontmatter, source_path: &task.origin_path,
                            prepared_context: run.as_ref().map(|opened| &opened.context).or(self.stack.prepared_context),
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
                        signal = if error.is_some() { LifecycleSignal::Failure } else { LifecycleSignal::Success };
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
                    return PrimaryOutcome::succeeded(String::new())
                        .with_secondary(secondary_errors)
                        .with_group_output(runtime.outputs_value());
                }
                ContainerEventDecision::Finished { failed: true } => {
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
                    let state = base.clone().unwrap_or_else(|| self.state.data().iter().map(|(key,value)| (key.clone(),value.clone())).collect());
                    runtime.clear_outputs();
                    if let Err(failure) = runtime.set_batch(self.stack.effect_engine, &overlay, &state) {
                        let diagnostic = TaskDiagnostic { stage: TaskStage::Primary,
                            info: crate::composition::lifecycle::context::LifecycleErrorInfo::from_error_or_action("retry", &failure) };
                        runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info);
                        signal = LifecycleSignal::Blocked;
                        continue;
                    }
                    error = None; secondary_errors.clear(); signal = LifecycleSignal::Start;
                }
                ContainerEventDecision::Abort(LifecycleTransitionAbort::ResumeWithoutSession) => {
                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary,
                        &CompositionError::LifecycleResumeWithoutSession { source_path: sequence.origin_path.clone() });
                    error = Some(diagnostic.info);
                    runner.set_error(error.as_ref().unwrap().clone());
                    signal = if signal == LifecycleSignal::Failure { LifecycleSignal::Finalize } else { LifecycleSignal::Failure };
                }
                ContainerEventDecision::SetupRecoveryUnsupported { event, action } => {
                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary,
                        &CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                            source_path: sequence.origin_path.clone(), event: event.property_name().into(), action: action.into(),
                        });
                    error = Some(diagnostic.info);
                    runner.set_error(error.as_ref().unwrap().clone());
                    signal = if signal == LifecycleSignal::Failure { LifecycleSignal::Finalize } else { LifecycleSignal::Failure };
                }
                ContainerEventDecision::Proxy { target, overlay } => {
                    return self.run_container_proxy(&target, overlay);
                }
                decision => {
                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary,
                        &CompositionError::SequenceTaskUnsupported {
                            task: self.label(), construct: "nested container recovery".into(), detail: format!("{decision:?}"),
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
    /// The base state of a referenced sequence document's events and
    /// children: its authored properties under the invocation setters.
    /// Runtime mutations are layered above it at each read. `None` for an
    /// inline list or a step selection, which read the enclosing state.
    fn sequence_frontmatter(&self, sequence: &PreflightSequence) -> Option<Map<String, Value>> {
        let mut base: Map<String, Value> = sequence.frontmatter.as_ref()?.iter()
            .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value))).collect();
        if let Some(Value::Object(setters)) = self.user_setters {
            base.extend(setters.clone());
        }
        Some(base)
    }

    pub(super) fn verify_nested_graph(&self, fresh: &crate::composition::sequence::preflight::PreflightGraph) -> Result<(), CompositionError> {
        let unapproved = fresh.shell_commands.iter().find(|command| !self.graph.shell_commands.iter()
            .any(|approved| approved.command == command.command));
        if let Some(command) = unapproved {
            return Err(CompositionError::SequenceShellResolution {
                command: command.command.clone(), task: self.label(),
                message: "nested source introduced a command after sequence approval closed".into(), source: None,
            });
        }
        for candidate in &fresh.template_candidates {
            if !self.graph.template_candidates.iter().any(|approved| approved.authored() == candidate.authored())
                || self.graph.shell_authorization().is_some_and(|(approved, _)| !candidate.authorized_by(&approved)) {
                return Err(CompositionError::SequenceShellResolution {
                    command: candidate.authored().unwrap_or("").into(), task: self.label(),
                    message: "nested source introduced a template after sequence approval closed".into(), source: None,
                });
            }
        }
        for document in &fresh.prompt_documents {
            if self.graph.prompt_document(&document.path).is_none() {
                return Err(CompositionError::SequenceInvalid("nested source introduced an unapproved prompt document".into()));
            }
        }
        if let Some(authorization) = self.graph.shell_authorization() {
            fresh.settle_container_events(&authorization.0, authorization.1.as_deref());
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
            let path = crate::composition::sequence::source::resolve_sequence_reference(
                target, source, self.stack.file_resolution_context,
            )?;
            let path = biscuit_file::canonicalize_simplified(&path).unwrap_or(path);
            let owned_runtime = Arc::new(RuntimeState::new());
            let runtime = self.runtime.unwrap_or(&owned_runtime);
            let _chain = runtime.enter_container_proxy(source, &path, target)?;
            if let Some(runtime) = self.runtime {
                let prior = self.state.data().iter().map(|(key,value)| (key.clone(),value.clone())).collect();
                if let Err(error) = runtime.set_batch(self.stack.effect_engine, &overlay, &prior) {
                    return Ok(PrimaryOutcome::failed(TaskDiagnostic { stage: TaskStage::Primary,
                        info: crate::composition::lifecycle::context::LifecycleErrorInfo::from_error_or_action("proxy", &error) }));
                }
            }
            if let Some(sequence) = approved_sequence(&self.graph.steps, &path) {
                let mut task = self.task.clone();
                task.action = PreflightAction::Sequence(Box::new(sequence.clone()));
                task.origin_path = sequence.origin_path.clone();
                task.origin_dir = sequence.origin_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
                task.authored = Default::default();
                return Ok(TaskExecution { task: &task, runtime: Some(runtime), ..*self }.run_sequence(sequence));
            }
            if self.graph.prompt_document(&path).is_none() {
                return Err(CompositionError::SequenceTaskUnsupported {
                    task: self.label(), construct: "container proxy".into(),
                    detail: format!("target `{target}` was not approved during sequence preflight"),
                });
            }
            let mut task = self.task.clone();
            task.action = PreflightAction::Prompt { path: path.clone(), reference: target.into() };
            task.params.clear(); task.setup = None; task.teardown = None;
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
