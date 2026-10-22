//! Concurrent group legs use isolated runtime branches and declaration-order
//! admission and folding. A failed atomic group publishes no leg state or result.

use crate::composition::lifecycle::cancellation::CancellationCauseExt;
use crate::composition::LifecycleErrorInfoExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder};
use serde_json::{Map, Value};

use super::super::super::error::CompositionError;
use super::super::super::lifecycle::context::LifecycleErrorInfo;
use super::super::super::lifecycle::executor::StackExecutionContext;
use super::super::super::runtime_state::{RuntimeSnapshot, RuntimeState};
use super::super::preflight::{PreflightGroup, PreflightTask};
use super::{
    PrimaryOutcome, TaskDiagnostic, TaskExecution, TaskOutcome, TaskRun, TaskStage, TaskStatus,
};
use crate::render::{TaskBar, TaskLiveOutput, TaskStream};
use crate::composition::lifecycle::bindings::{GroupBinding, GroupLegResult, LegOutcome};
use crate::composition::looping::LoopResultKind;

/// A poisoned result slot means a member task panicked; the group's result is
/// incomplete and cannot be reported honestly.
const SLOT_POISONED: &str = "group result slot poisoned by a panicking task";

/// Stack reserved for each parallel member thread.
///
/// A member runs a prompt task's whole compose pipeline in-process, so it needs
/// the same 8 MiB the `claudine` main thread gets on every OS (see
/// `claudine/cli/build.rs`); Rust's 2 MiB spawn default left a debug build under
/// 4x headroom. The reservation is address space, committed only as used.
const MEMBER_THREAD_STACK_BYTES: usize = 8 * 1024 * 1024;

/// One finished member, with the wall clock the scheduler measured around it.
struct MemberRun {
    outcome: TaskOutcome,
    duration: Duration,
}

/// One member task's contribution to the owning step's summary.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupTaskResult {
    /// The task's authored `name:`, or its generated label.
    pub name: String,
    /// How the task finished.
    pub status: TaskStatus,
    /// `true` when the leg's `when:` guard was false and nothing ran.
    pub skipped: bool,
    /// Wall-clock time across the leg's execution.
    pub duration: Duration,
}

pub(super) struct ValidatedGroup {
    variables: Map<String, Value>,
    members: Vec<super::ValidatedTask>,
    runs: Vec<Option<TaskRun>>,
    group_run: Option<TaskRun>,
}

impl TaskExecution<'_> {
    pub(super) fn run_group_container(&self, group: &PreflightGroup) -> PrimaryOutcome {
        if self.interrupted() { return PrimaryOutcome::interrupted(String::new()); }
        use crate::composition::lifecycle::{LifecycleRunGuard, LifecycleRunGuardExt, LifecycleRuntimeContext, LifecycleSignal};
        use crate::composition::lifecycle::executor::DisabledShellRunner;
        use crate::composition::lifecycle::policy::{RefusalRoute, SubjectKind};
        use crate::composition::lifecycle::runtime::{ContainerEventDecision as Decision, ContainerRunner};

        let before = self.runtime.map_or_else(RuntimeSnapshot::default, |runtime| runtime.snapshot());
        let runtime = Arc::new(RuntimeState::from_snapshot(before));
        let live = Mutex::new(self.stack.live_frontmatter_snapshot()
            .unwrap_or_else(|| self.stack.frontmatter.clone()));
        let private_stack = self.stack.with_private_cells(&runtime, Some(&live));
        let execution = TaskExecution { runtime: Some(&runtime), stack: &private_stack, ..*self };
        let context = LifecycleRuntimeContext {
            settings: self.stack.settings, messaging: self.stack.messaging, term: self.stack.term,
            source_path: &group.origin_path, repo_root: self.stack.repo_root,
            launch_area: self.stack.ctx_base_dir, context: self.stack.prepared_context,
        };
        let mut runner = ContainerRunner::new(LifecycleRunGuard::new(
            &group.lifecycle, &context, self.stack.emitter)).with_subject(SubjectKind::Group);
        let mut frame = match self.open_loop(group.loop_config.as_deref(), LoopResultKind::Group, self.stack.frontmatter, &runtime) {
            Ok(frame) => frame,
            Err(error) => return PrimaryOutcome::failed(TaskDiagnostic::from_composition(TaskStage::Primary, &error)),
        };
        // Taken as each pass enters `start`; a retry of the pass returns here.
        let mut checkpoint = crate::composition::looping::PassCheckpoint::take(&runtime);
        let mut binding = GroupBinding::default();
        let mut signal = LifecycleSignal::Initialize;
        let mut result = PrimaryOutcome::succeeded(String::new());
        let mut error = None;
        let mut shell_enabled = false;
        let mut prepared = None;
        let mut current_group = None;
        loop {
            // The `loop` values this group's events and legs read: its own
            // loop's, else the enclosing loop's.
            let own_ambient = frame.as_ref().map(crate::composition::looping::LoopFrame::ambient);
            let loop_stack = StackExecutionContext {
                loop_values: own_ambient.as_ref().or(private_stack.loop_values),
                ..private_stack.with_signal(private_stack.signal)
            };
            let execution = TaskExecution { stack: &loop_stack, ..execution };
            if !matches!(signal, LifecycleSignal::Cancelled | LifecycleSignal::Finalize)
                && self.interrupted()
            {
                runner.cancel(crate::composition::lifecycle::cancellation::cause()
                    .unwrap_or(crate::composition::lifecycle::cancellation::CancellationCause::Interrupt));
                error = runner.primary_error().cloned();
                signal = LifecycleSignal::Cancelled;
            }
            if signal == LifecycleSignal::Start {
                shell_enabled = false;
                let preparation = execution.member_state().and_then(|state| {
                    let context = self.stack.prepared_context.cloned()
                        .unwrap_or_else(|| state.context().clone());
                    let fresh = if self.graph.sources_unchanged() {
                        group.clone()
                    } else {
                        let (fresh, graph) = super::super::preflight::prepare_nested_group(
                            self.task, group, &state, context, self.stack.file_resolution_context,
                        )?;
                        self.verify_nested_graph(&graph)?;
                        fresh
                    };
                    let validated = (TaskExecution { state: &state, ..execution }).validate_group(&fresh)?;
                    Ok((fresh, validated))
                });
                match preparation {
                    Ok((fresh, validated)) => {
                        runner.guard_mut().set_config((*fresh.lifecycle).clone());
                        binding.variables = validated.variables.clone();
                        current_group = Some(fresh);
                        prepared = Some(validated);
                        shell_enabled = true;
                    }
                    Err(failure) => {
                        let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &failure);
                        runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info.clone());
                        result = PrimaryOutcome::failed(diagnostic); signal = LifecycleSignal::Blocked;
                    }
                }
            }
            if signal == LifecycleSignal::Start && self.interrupted() {
                runner.cancel(crate::composition::lifecycle::cancellation::cause()
                    .unwrap_or(crate::composition::lifecycle::cancellation::CancellationCause::Interrupt));
                error = runner.primary_error().cloned();
                signal = LifecycleSignal::Cancelled;
            }
            let group = current_group.as_ref().unwrap_or(group);
            live.lock().expect(SLOT_POISONED).extend(runtime.snapshot().mutations);
            let group_value = binding.event_value(signal);
            let mut stack = loop_stack.with_signal(signal);
            stack.scope = None;
            stack.group = Some(&group_value);
            stack.err = error.as_ref();
            if !shell_enabled { stack.shell_runner = &DisabledShellRunner; }
            let event = if self.prompt.is_dry_run() {
                let outcome = Default::default();
                let decision = runner.record_outcome(signal, &outcome);
                crate::composition::lifecycle::runtime::ContainerEventResult { outcome, decision }
            } else { runner.execute_event(signal, &stack) };
            match event.decision {
                Decision::Continue if signal == LifecycleSignal::Initialize => {
                    checkpoint = crate::composition::looping::PassCheckpoint::take(&runtime);
                    signal = LifecycleSignal::Start;
                }
                Decision::Continue => {
                    let state = match execution.member_state() {
                        Ok(state) => state,
                        Err(failure) => {
                            let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &failure);
                            runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info.clone());
                            result = PrimaryOutcome::failed(diagnostic); signal = LifecycleSignal::Blocked;
                            continue;
                        }
                    };
                    let attempt = TaskExecution { state: &state, ..execution };
                    match prepared.take().map_or_else(|| attempt.validate_group(group), Ok) {
                        Ok(validated) => {
                            result = attempt.run_group(group, &validated);
                            binding = result.group_binding.take().unwrap_or_default();
                            binding.variables = validated.variables;
                            // The pass's dictionary is its result: its terminal
                            // event, `finalize`, and gate read it.
                            if let Some(frame) = frame.as_mut()
                                && !matches!(result.result, super::PrimaryResult::Interrupted)
                            {
                                frame.record_pass(
                                    result.group_output.clone().unwrap_or_else(|| LoopResultKind::Group.empty()),
                                    i32::from(matches!(result.result, super::PrimaryResult::Failed(_))),
                                );
                            }
                            match &result.result {
                                // A `break` fires no further event but `finalize`.
                                super::PrimaryResult::Succeeded
                                    if loop_stack.loop_values.is_some_and(|ambient| ambient.breaks.is_pending()) =>
                                {
                                    runner.guard_mut().defuse();
                                    signal = LifecycleSignal::Finalize;
                                }
                                super::PrimaryResult::Succeeded => signal = LifecycleSignal::Success,
                                super::PrimaryResult::Failed(diagnostic) => {
                                    runner.set_error(diagnostic.info.clone());
                                    error = Some(diagnostic.info.clone()); signal = LifecycleSignal::Failure;
                                }
                                super::PrimaryResult::Interrupted => {
                                    runner.cancel(crate::composition::lifecycle::cancellation::cause()
                                        .unwrap_or(crate::composition::lifecycle::cancellation::CancellationCause::Interrupt));
                                    error = runner.primary_error().cloned();
                                    signal = LifecycleSignal::Cancelled;
                                }
                            }
                        }
                        Err(failure) => {
                            let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &failure);
                            runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info.clone());
                            result = PrimaryOutcome::failed(diagnostic); signal = LifecycleSignal::Blocked;
                        }
                    }
                }
                Decision::Failure { error: raised } => {
                    error = raised.or(error); signal = LifecycleSignal::Failure;
                }
                Decision::Finalize { error: raised, .. } => {
                    error = raised.or(error); signal = LifecycleSignal::Finalize;
                }
                Decision::Finished { .. } if runner.is_cancelled() => {
                    result.secondary.extend(runner.secondary_errors().iter().cloned().map(|info|
                        TaskDiagnostic { stage: TaskStage::Primary, info }));
                    result.result = super::PrimaryResult::Interrupted;
                    return result;
                }
                // A dry run previews one pass: it never runs the gate or reads
                // the condition.
                Decision::Finished { failed: false } if frame.is_some() && !self.prompt.is_dry_run() => {
                    let frame = frame.as_mut().expect("guarded by the match");
                    let ambient = frame.ambient();
                    let gate_value = binding.event_value(LifecycleSignal::Loop);
                    let gate_frontmatter = crate::composition::looping::container_state(self.stack.frontmatter, &runtime);
                    let mut gate = loop_stack.with_signal(LifecycleSignal::Loop);
                    gate.scope = None;
                    gate.group = Some(&gate_value);
                    gate.err = None;
                    gate.frontmatter = &gate_frontmatter;
                    gate.loop_values = Some(&ambient);
                    let end = crate::composition::looping::finish_container_pass(frame, runner.guard_mut(), &gate, &runtime);
                    if let Ok(crate::composition::looping::ContainerPassEnd::Broken(broken)) = &end {
                        self.stack.emitter.emit_info(&broken.summary(), self.stack.term);
                    }
                    match end {
                        Ok(crate::composition::looping::ContainerPassEnd::Continue) => {
                            // The next pass rebuilds its leg results and
                            // summaries from nothing.
                            runner.begin_next_pass();
                            checkpoint = crate::composition::looping::PassCheckpoint::take(&runtime);
                            binding = GroupBinding { variables: binding.variables.clone(), legs: Vec::new() };
                            result = PrimaryOutcome::succeeded(String::new());
                            error = None;
                            signal = LifecycleSignal::Start;
                            continue;
                        }
                        Ok(crate::composition::looping::ContainerPassEnd::Complete | crate::composition::looping::ContainerPassEnd::Broken(_)) => {
                            if let Some(parent) = self.runtime
                                && let Err(failure) = parent.set_batch(self.stack.effect_engine,
                                    &runtime.snapshot().mutations, &Map::new())
                            {
                                return PrimaryOutcome::failed(TaskDiagnostic {
                                    stage: TaskStage::Primary,
                                    info: LifecycleErrorInfo::from_error_or_action("set", &failure),
                                });
                            }
                            return result;
                        }
                        // The loop owner failed after the pass settled: no
                        // pass state reaches the parent.
                        Err(failure) => {
                            return PrimaryOutcome::failed(TaskDiagnostic::from_composition(TaskStage::Primary, &failure));
                        }
                    }
                }
                Decision::Finished { failed } => {
                    if failed {
                        // A failure beats a pending `break`.
                        if let Some(ambient) = loop_stack.loop_values { ambient.breaks.discard(); }
                        let info = runner.primary_error().cloned().or(error)
                            .unwrap_or_else(|| LifecycleErrorInfo::from_action_failure("error", "group finalization failed"));
                        result.secondary.extend(runner.secondary_errors().iter().cloned().map(|info|
                            TaskDiagnostic { stage: TaskStage::Primary, info }));
                        result.result = super::PrimaryResult::Failed(TaskDiagnostic { stage: TaskStage::Primary, info });
                    } else if let Some(parent) = self.runtime
                        && let Err(failure) = parent.set_batch(self.stack.effect_engine,
                            &runtime.snapshot().mutations, &Map::new())
                    {
                        return PrimaryOutcome::failed(TaskDiagnostic {
                            stage: TaskStage::Primary,
                            info: LifecycleErrorInfo::from_error_or_action("set", &failure),
                        });
                    }
                    return result;
                }
                Decision::Retry { delay, overlay } => {
                    binding = GroupBinding::default();
                    // A looping group replaces only the failed attempt of
                    // this pass: its runtime writes return to the pass's
                    // entry before the retry's own values apply.
                    if let Some(frame) = frame.as_mut() {
                        frame.retry_pass();
                        checkpoint.restore(&runtime);
                    }
                    if let Err(failure) = runtime.set_batch(self.stack.effect_engine, &overlay, &Map::new()) {
                        let diagnostic = TaskDiagnostic {
                            stage: TaskStage::Primary,
                            info: LifecycleErrorInfo::from_error_or_action("set", &failure),
                        };
                        runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info.clone());
                        result = PrimaryOutcome::failed(diagnostic); signal = LifecycleSignal::Blocked;
                        continue;
                    }
                    if !delay.is_zero() { std::thread::sleep(delay); }
                    error = None;
                    // Re-enter preparation without re-emitting initialize.
                    signal = LifecycleSignal::Start;
                }
                Decision::Skip => return PrimaryOutcome::succeeded(String::new()),
                decision @ (Decision::Abort(_) | Decision::SetupRecoveryUnsupported { .. }) => {
                    let (Some(refusal), Some(route)) = (decision.refusal(), runner.refusal_route(&decision)) else {
                        unreachable!("a refusal decision carries its refusal: {decision:?}");
                    };
                    let Some(failure) = CompositionError::from_lifecycle_refusal(&refusal, &group.origin_path) else {
                        unreachable!("a container refuses only a setup recovery, a resume, or a defer: {refusal:?}");
                    };
                    let diagnostic = TaskDiagnostic::from_composition(TaskStage::Primary, &failure);
                    runner.set_error(diagnostic.info.clone()); error = Some(diagnostic.info.clone());
                    result = PrimaryOutcome::failed(diagnostic);
                    signal = match route {
                        RefusalRoute::FailureThenFinalize => LifecycleSignal::Failure,
                        RefusalRoute::FinalizeIfOwedReadingRefusal | RefusalRoute::FinalizeIfOwed => LifecycleSignal::Finalize,
                    };
                }
                Decision::Proxy { target, overlay } => {
                    let result = execution.run_container_proxy(&target, overlay);
                    if matches!(result.result, super::PrimaryResult::Succeeded)
                        && let Some(parent) = self.runtime
                        && let Err(failure) = parent.set_batch(self.stack.effect_engine,
                            &runtime.snapshot().mutations, &Map::new())
                    {
                        return PrimaryOutcome::failed(TaskDiagnostic {
                            stage: TaskStage::Primary,
                            info: LifecycleErrorInfo::from_error_or_action("set", &failure),
                        });
                    }
                    return result;
                }
            }
        }
    }

    /// The attributed live stream for one member task, when a sink is wired.
    ///
    /// `None` — the `--silent` shape — costs nothing: no frame is rendered at
    /// all, rather than rendered and dropped.
    fn task_stream(&self, task: &PreflightTask, bar: TaskBar) -> Option<Arc<TaskLiveOutput>> {
        self.stream.map(|sink| {
            Arc::new(TaskLiveOutput::new(
                TaskStream::new(
                    task.name.clone().unwrap_or_else(|| task.label.clone()),
                    bar,
                    self.stack.term.clone(),
                ),
                Arc::clone(sink),
            ))
        })
    }

    /// Freeze every member's late shell inputs before the group begins effects.
    pub(super) fn validate_group(&self, group: &PreflightGroup) -> Result<ValidatedGroup, CompositionError> {
        let variables = self.resolve_group_variables(group)?;
        let overlay = self.member_overlay();
        let stack = self.stack.with_group(&variables);
        let state = self.member_state()?;
        let group_run = self.prompt.open_group_run(group);
        let mut members = Vec::with_capacity(group.tasks.len());
        let mut runs = Vec::with_capacity(group.tasks.len());
        for task in &group.tasks {
            let run = self.prompt.open_leg_run(task, group_run.as_ref());
            let member_stack = StackExecutionContext {
                source_path: &task.diagnostic.source_path,
                prepared_context: run.as_ref().map(|opened| &opened.context)
                    .or(group_run.as_ref().map(|opened| &opened.context))
                    .or(stack.prepared_context),
                ..stack.with_signal(stack.signal)
            };
            let member = TaskExecution {
                task, state: &state, stack: &member_stack, overlay: Some(&overlay),
                run_evidence: run.as_ref().map(|opened| &opened.run)
                    .or(group_run.as_ref().map(|opened| &opened.run)).or(self.run_evidence),
                document: run.as_ref().and_then(|opened| opened.document.as_ref()).or(self.document),
                ..*self
            };
            let mut validated = member.validate_task()?;
            let has_late_document = validated.prompt.as_ref()
                .is_some_and(|request| !request.document_shell_values.is_empty());
            if !has_late_document {
                validated.prompt = None;
            }
            members.push(validated);
            runs.push(run);
        }
        Ok(ValidatedGroup { variables, members, runs, group_run })
    }

    pub(super) fn run_group(&self, group: &PreflightGroup, validated: &ValidatedGroup) -> PrimaryOutcome {
        let variables = &validated.variables;
        let scope_overlay = self.member_overlay();
        let scope_stack = self.stack.with_group(variables);
        self.run_parallel(group, &scope_overlay, &scope_stack, validated)
    }

    /// Bounded-concurrency execution with snapshot isolation.
    ///
    /// Three things make the result independent of completion order (spec →
    /// *Concurrency*): every member composes against **one** state built at group
    /// start, every member writes into its **own** runtime buffer, and every
    /// post-run fold — outputs, mutations, summaries — walks the tasks in
    /// declaration order.
    fn run_parallel(
        &self,
        group: &PreflightGroup,
        scope_overlay: &Value,
        scope_stack: &StackExecutionContext<'_>,
        validated: &ValidatedGroup,
    ) -> PrimaryOutcome {
        // Taken once. A sibling's `set` or `outputs` entry is invisible until the
        // merge below, and no live file is re-read between siblings.
        let base = self
            .runtime
            .map_or_else(RuntimeSnapshot::default, |runtime| runtime.snapshot());
        let member_state = match self.member_state_from(&base) {
            Ok(state) => state,
            Err(error) => {
                return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                    &error,
                ));
            }
        };

        let buffers: Vec<Arc<RuntimeState>> = group
            .tasks
            .iter()
            .map(|_| Arc::new(RuntimeState::from_snapshot(base.clone())))
            .collect();
        let slots: Vec<Mutex<Option<MemberRun>>> =
            group.tasks.iter().map(|_| Mutex::new(None)).collect();
        // The live document cell is per-*attempt* and therefore shared by
        // construction; a member needs its own, seeded from the group-start
        // view, for the same reason it needs its own runtime buffer. The copies
        // are dropped with the group: a member's `set_frontmatter` already
        // reached disk, and the next *step* re-reads the file — a sibling never
        // does (spec → *Concurrency*).
        let live_cells: Vec<Mutex<Map<String, Value>>> = scope_stack
            .live_frontmatter_snapshot()
            .into_iter()
            .flat_map(|seed| group.tasks.iter().map(move |_| Mutex::new(seed.clone())))
            .collect();

        // Absent cap means "launch all"; declaration-order admission falls out of
        // a single shared cursor, so a freed slot always takes the next task.
        // One run for the whole group, captured before any sibling starts;
        // a sibling that re-enters later opens its own.
        let group_run = validated.group_run.as_ref();
        let group_context = group_run
            .as_ref()
            .map(|opened| &opened.context)
            .or(scope_stack.prepared_context);
        let group_evidence = group_run
            .as_ref()
            .map(|opened| &opened.run)
            .or(self.run_evidence);
        // Each leg is a document of its own and captures its own environment,
        // in declaration order and before any leg starts, so which leg holds
        // which capture does not depend on scheduling. The legs share the
        // group's run, not one another's document.
        let leg_runs = &validated.runs;

        let workers = group
            .max_parallel
            .unwrap_or(group.tasks.len())
            .clamp(1, group.tasks.len().max(1));
        let admission = Mutex::new((0usize, false, None));
        let parent_cancellation = crate::composition::lifecycle::cancellation::current();
        // Each leg calls `prep` targets through the same runner, on a branch
        // of the active call chain of its own.
        let prep_scope = crate::composition::lifecycle::prep::inherit();
        let cancellations: Vec<_> = group.tasks.iter().map(|_| parent_cancellation.child()).collect();
        // A leg records its `break` in a slot of its own, so a pending break
        // stops no sibling and no admission; the fold below picks the first
        // breaking leg in declaration order once every leg has settled.
        let leg_loops: Vec<Option<crate::composition::LoopAmbient>> = group.tasks.iter()
            .map(|_| scope_stack.loop_values.map(crate::composition::LoopAmbient::with_isolated_breaks))
            .collect();
        let cancellable: Vec<_> = group.tasks.iter().map(|task| match &task.action {
            super::super::preflight::PreflightAction::Sequence(sequence) => sequence.lifecycle.cancelled_declared,
            super::super::preflight::PreflightAction::Group(group) => group.lifecycle.cancelled_declared,
            super::super::preflight::PreflightAction::Prompt { path, .. } => self.graph.prompt_documents.iter()
                .find(|document| &document.path == path)
                .is_some_and(|document| document.markdown.frontmatter().as_map().contains_key("cancelled")),
            _ => false,
        }).collect();

        std::thread::scope(|scope| {
            for _ in 0..workers {
                std::thread::Builder::new()
                    .stack_size(MEMBER_THREAD_STACK_BYTES)
                    .spawn_scoped(scope, || {
                    loop {
                        let index = {
                            let mut admission = admission.lock().expect(SLOT_POISONED);
                            if self.interrupted() || admission.0 >= group.tasks.len() || (group.fail_fast && admission.1) {
                                break;
                            }
                            let index = admission.0;
                            admission.0 += 1;
                            index
                        };
                        let _cancellation_scope = cancellations[index].enter();
                        let _prep_scope = prep_scope.enter();
                        let task = &group.tasks[index];
                        let started = Instant::now();
                        // Each concurrent task takes its own palette entry, so
                        // interleaved frames stay attributable without relying
                        // on arrival order.
                        let stream = self.task_stream(task, TaskBar::for_index(index));
                        if let Some(stream) = &stream {
                            stream.open();
                        }
                        // The stack's cells must be the member's too: a lifecycle
                        // `set` inside the task accumulates through the stack,
                        // not through `TaskExecution::runtime`.
                        let leg_run = leg_runs[index].as_ref();
                        let task_stack = StackExecutionContext {
                            source_path: &task.diagnostic.source_path,
                            prepared_context: leg_run
                                .map(|opened| &opened.context)
                                .or(group_context),
                            loop_values: leg_loops[index].as_ref(),
                            ..scope_stack.with_signal(scope_stack.signal)
                        };
                        let member_stack = task_stack
                            .with_private_cells(&buffers[index], live_cells.get(index));
                        let member = TaskExecution {
                            task,
                            state: &member_state,
                            stack: &member_stack,
                            overlay: Some(scope_overlay),
                            runtime: Some(&buffers[index]),
                            live: stream.as_ref(),
                            run_evidence: leg_run
                                .map(|opened| &opened.run)
                                .or(group_evidence),
                            document: leg_run
                                .and_then(|opened| opened.document.as_ref())
                                .or(self.document),
                            ..*self
                        };
                        // The guard reads the group-start state the member was built
                        // from, even for a leg admitted late.
                        let outcome = member.run_guarded(Some(&validated.members[index]));
                        let duration = started.elapsed();
                        if let Some(stream) = &stream {
                            stream.close(outcome.status.into(), duration);
                        }
                        if outcome.status != TaskStatus::Succeeded {
                            let mut admission = admission.lock().expect(SLOT_POISONED);
                            admission.1 = true;
                            if admission.2.is_none() && outcome.status == TaskStatus::Failed {
                                admission.2 = outcome.error.as_ref().map(|error| error.info.clone());
                                if group.fail_fast {
                                    let cause = crate::composition::lifecycle::cancellation::CancellationCause::SiblingFailure {
                                        group: group.name.clone(), leg: task.name.clone().unwrap_or_else(|| task.label.clone()),
                                        error: outcome.error.as_ref().expect("failed task has a diagnostic").info.clone(),
                                    };
                                    for sibling in 0..admission.0 {
                                        if sibling != index && cancellable[sibling] {
                                            cancellations[sibling].cancel(cause.clone());
                                        }
                                    }
                                }
                            }
                        }
                        *slots[index].lock().expect(SLOT_POISONED) =
                            Some(MemberRun { duration, outcome });
                    }
                })
                    .expect("spawn a parallel group member thread");
            }
        });

        let triggering_failure = admission.into_inner().expect(SLOT_POISONED).2;
        let statuses: Vec<Option<TaskStatus>> = slots.iter()
            .map(|slot| slot.lock().expect(SLOT_POISONED).as_ref().map(|run| run.outcome.status))
            .collect();
        let outcome = self.fold_parallel_results(group, slots, triggering_failure);
        // A settled, successful group carries the first breaking leg's
        // `break` to its own loop slot. A leg whose own failure beat its break
        // is not a breaking leg, and a failed group carries none.
        if matches!(outcome.result, super::PrimaryResult::Succeeded)
            && let Some(ambient) = scope_stack.loop_values
            && let Some(broken) = statuses.iter().zip(&leg_loops)
                .filter(|(status, _)| **status == Some(TaskStatus::Succeeded))
                .find_map(|(_, leg)| leg.as_ref().and_then(|leg| leg.breaks.pending()))
        {
            ambient.breaks.record(broken);
        }
        outcome
    }

    /// Reduce the finished members into one group outcome, in declaration order.
    fn fold_parallel_results(
        &self,
        group: &PreflightGroup,
        slots: Vec<Mutex<Option<MemberRun>>>,
        triggering_failure: Option<LifecycleErrorInfo>,
    ) -> PrimaryOutcome {
        let runs: Vec<Option<MemberRun>> = slots
            .into_iter()
            .map(|slot| slot.into_inner().expect(SLOT_POISONED))
            .collect();

        let mut results = Vec::with_capacity(runs.len());
        let mut entries = Map::new();
        let mut secondary: Vec<TaskDiagnostic> = Vec::new();
        let mut failure: Option<TaskDiagnostic> = None;
        let mut interrupted = false;
        let mut binding = GroupBinding { variables: Map::new(), legs: Vec::new() };
        let cancellation_error = triggering_failure.or_else(||
            crate::composition::lifecycle::cancellation::cause().map(|cause| cause.error()))
            .or_else(|| self.interrupted().then(||
                crate::composition::lifecycle::cancellation::CancellationCause::Interrupt.error()));
        let cancelled_error = cancellation_error.as_ref().map_or(Value::Null, LifecycleErrorInfo::to_value);

        for (task, run) in group.tasks.iter().zip(&runs) {
            let name = task.name.clone().unwrap_or_else(|| task.label.clone());
            let Some(run) = run else {
                binding.legs.push(GroupLegResult { name, outcome: LegOutcome::Cancelled,
                    output: Value::Null, err: cancelled_error.clone() });
                continue;
            };
            binding.legs.push(GroupLegResult {
                name: name.clone(),
                outcome: match run.outcome.status {
                    TaskStatus::Succeeded if run.outcome.result.is_null() && !run.outcome.output_committed => LegOutcome::Skipped,
                    TaskStatus::Succeeded => LegOutcome::Succeeded,
                    TaskStatus::Failed => LegOutcome::Failed,
                    TaskStatus::Interrupted => LegOutcome::Cancelled,
                },
                output: run.outcome.result.clone(),
                err: run.outcome.error.as_ref().map_or_else(
                    || if run.outcome.status == TaskStatus::Interrupted { cancelled_error.clone() } else { Value::Null },
                    |error| error.info.to_value()),
            });
            results.push(GroupTaskResult {
                name: name.clone(),
                status: run.outcome.status,
                skipped: run.outcome.skipped,
                duration: run.duration,
            });
            // Failed legs retain their native partial result for terminal summaries.
            entries.insert(name, run.outcome.result.clone());
            secondary.extend(run.outcome.secondary_errors.clone());
            match run.outcome.status {
                TaskStatus::Succeeded => {}
                TaskStatus::Interrupted => interrupted = true,
                TaskStatus::Failed => {
                    if failure.is_none() {
                        failure = run.outcome.error.clone();
                    } else {
                        secondary.extend(run.outcome.error.clone());
                    }
                }
            }
        }

        let stdout = runs.iter().flatten()
            .map(|run| run.outcome.stdout.as_str()).collect::<Vec<_>>().join("\n");

        // An interrupted group commits nothing: the run is being torn down, and a
        // half-scheduled group's state is not a state any later step should read.
        let outcome = if interrupted && (self.interrupted() || failure.is_none()) {
            PrimaryOutcome::interrupted(stdout)
        } else if group.fail_fast && failure.is_some() {
            PrimaryOutcome::failed_with(stdout, failure.expect("failed leg has a diagnostic"))
                .with_group_output(Value::Object(entries))
        } else {
            secondary.extend(self.merge_parallel_mutations(group, &runs));
            PrimaryOutcome::succeeded(stdout).with_group_output(Value::Object(entries))
        };
        let mut outcome = outcome.with_group_tasks(results).with_secondary(secondary);
        outcome.group_binding = Some(binding);
        outcome
    }

    /// Fold every member's mutation delta into the sequence cell.
    ///
    /// Declaration order, not completion order, so the surviving value for a
    /// contested key is the same on every run. The expected case is disjoint
    /// keys; a collision is legal but warned about, because "both tasks wrote
    /// `count`" is nearly always an authoring mistake.
    ///
    /// ## Returns
    ///
    /// Diagnostics for writes the cell refused. A member accepted each of these
    /// keys against the same reserved-key policy, so a refusal here means the two
    /// checks disagree — reportable, but never the reason the group failed.
    fn merge_parallel_mutations(
        &self,
        group: &PreflightGroup,
        runs: &[Option<MemberRun>],
    ) -> Vec<TaskDiagnostic> {
        let mut refused = Vec::new();
        let Some(runtime) = self.runtime else {
            return refused;
        };
        let mut written: HashMap<&str, String> = HashMap::new();
        for (task, run) in group.tasks.iter().zip(runs) {
            let Some(run) = run else { continue; };
            let name = task.name.clone().unwrap_or_else(|| task.label.clone());
            for mutation in &run.outcome.mutations {
                if let Some(earlier) = written.get(mutation.key.as_str()) {
                    self.stack.emitter.emit_warn(
                        &format!(
                            "group `{}`: `{}` and `{name}` both wrote `{}`; \
                             the later-declared task wins",
                            group.name, earlier, mutation.key
                        ),
                        self.stack.term,
                    );
                }
                written.insert(mutation.key.as_str(), name.clone());
                if let Err(error) = runtime.set(
                    self.stack.effect_engine,
                    &mutation.key,
                    mutation.value.clone(),
                    &Map::new(),
                ) {
                    refused.push(TaskDiagnostic {
                        stage: TaskStage::Primary,
                        info: LifecycleErrorInfo::from_error_or_action("set", &error),
                    });
                }
            }
        }
        refused
    }

    /// Evaluate the group's `variables:` against the state the group starts in.
    ///
    /// Interpolation happens once, at group start, so every member task reads
    /// the same values regardless of what a sibling mutates.
    fn resolve_group_variables(
        &self,
        group: &PreflightGroup,
    ) -> Result<Map<String, Value>, CompositionError> {
        let mut resolved = Map::new();
        for (key, value) in &group.variables {
            resolved.insert(
                key.clone(),
                self.resolve_value(value, &format!("group `{}` variables.{key}", group.name))?,
            );
        }
        Ok(resolved)
    }

    /// Apply accumulated runtime writes without exposing group variables as frontmatter.
    fn member_state(
        &self,
    ) -> Result<EffectiveState, CompositionError> {
        let snapshot = self
            .runtime
            .map_or_else(RuntimeSnapshot::default, |runtime| runtime.snapshot());
        self.member_state_from(&snapshot)
    }

    /// All concurrent siblings compose against the same group-entry snapshot.
    fn member_state_from(
        &self,
        snapshot: &RuntimeSnapshot,
    ) -> Result<EffectiveState, CompositionError> {
        let mut data = self.state.data().clone();
        if self.runtime.is_some() {
            for (key, value) in &snapshot.mutations {
                data.insert(key.clone(), value.clone());
            }
        }
        if let Some(Value::Object(overlay)) = self.overlay {
            for (key, value) in overlay {
                if key != "seq" { data.insert(key.clone(), value.clone()); }
            }
        }
        EffectiveStateBuilder::new()
            .with_frontmatter(data)
            .with_context(self.state.context().clone())
            .with_allow_ctx_override(true)
            .build()
            .map_err(|source| CompositionError::PreFlightStateBuildFailed { source })
    }

    /// Group variables travel through the lexical stack and prompt request,
    /// never the frontmatter overlay.
    fn member_overlay(&self) -> Value {
        self.overlay.cloned().unwrap_or_else(|| Value::Object(Map::new()))
    }
}
