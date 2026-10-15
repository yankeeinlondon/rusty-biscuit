//! Wrapper-side execution of a step's explicit task.
//!
//! The library owns the task stage contract ([`TaskExecution`]) but never
//! launches a provider. This module supplies the two seams it needs — a system
//! shell and a [`PromptTaskRunner`] that composes and runs a referenced document
//! through the wrapper pipeline — and translates the resulting [`TaskOutcome`]
//! into the same step outcome a default-body step produces.
//!
//! A prompt task's provider run **withholds** its own `outputs` commit
//! (`suppress_output_commit`). The task executor publishes the entry only after
//! `teardown` completes, because a teardown that fails converts an otherwise
//! successful task to failure and owes no output.
//!
//! ## Where a prompt task's framing comes from
//!
//! Shell and side-effect body text reaches stdout through the task's
//! [`TaskLiveOutput`]. A `prompt:` task's provider text does not pass through
//! here at all — it is written live by the wrapper's own semantic stream — so
//! it is framed at that end instead, by two decorators threaded down from
//! [`PromptTaskRequest::frame_writer`]:
//!
//! - **data** — a [`TaskFrameWriter`][claudine::render::TaskFrameWriter] reaches
//!   the provider stdout reader, framing assistant chunks, idle flushes, and
//!   post-parser-failure raw fallback lines.
//! - **status** — the writer's gutter also decorates the `StreamOutput` handle
//!   `LiveSemanticSink` is built on, so reasoning, tool announcements, timing
//!   headers, and warnings carry the same bar.
//!
//! Re-framing the *final* assistant text here would print it twice, so this
//! module deliberately does not touch it. The `outputs` entry stays undecorated
//! either way: the bar is added on the rendering path only.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use claudine::composition::lifecycle_executor::{StackExecutionContext, SystemShellRunner};
use claudine::composition::{
    self, CompositionError, CompositionExecutionRequest, CompositionMode, DefaultLifecycleEmitter,
    PreparedComposition, ResolvedExecutionTarget, RunLedger, RuntimeState, SequenceTaskResult,
    SharedRunLedger,
};
use claudine::composition::sequence::preflight::{PreflightAction, PreflightGroup, PreflightTask};
use claudine::composition::sequence::task::{
    PromptRunOutcome, PromptTaskRequest, PromptTaskRunner, SystemTaskShell, TaskExecution,
    TaskOutcome, TaskRun, TaskStatus,
};
use claudine::diagnostics::DiagnosticSnapshot;
use claudine::invocation_context::{InvocationContext, RunEvidence, SourceContext};
use claudine::render::{TaskBar, TaskLiveOutput, TaskStream, TaskStreamSink};
use claudine::system_prompt::SystemPromptArgs;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::EffectiveStateBuilder;
use serde_json::Value;

use crate::commands::compose::CompositionKind;
use crate::commands::compose::prep::StepScope;
use crate::commands::wrap::composition::execute_composition_request_inner;

use super::iterate::{SequenceRunContext, StepOutcome};
use super::task_frames::SequenceTaskSink;
use super::task_prewarm::TaskStackPrewarm;

/// Run a step's explicit task and report it as a step outcome.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_step_task(
    run: &SequenceRunContext<'_>,
    task: &PreflightTask,
    prepared: &PreparedComposition,
    overlay: &Value,
    env_overrides: &BTreeMap<String, String>,
    target: Option<&ResolvedExecutionTarget>,
    runtime_state: &std::sync::Arc<RuntimeState>,
    compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
    // The step's composition run, which the task and its prompt document join.
    step_run: &RunEvidence,
) -> StepOutcome {
    let frontmatter = effective_frontmatter(prepared);
    let state = match EffectiveStateBuilder::new()
        .with_frontmatter(frontmatter.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .with_allow_ctx_override(true)
        .build()
    {
        Ok(state) => state,
        Err(source) => {
            let error = CompositionError::PreFlightStateBuildFailed { source };
            return StepOutcome::Failed {
                message: error.to_string(),
                snapshot: DiagnosticSnapshot::select(&error).map(Box::new),
                agent_perf: None,
                compose_perf,
                tasks: Vec::new(),
            };
        }
    };

    let (task_source_context, task_context, document_epoch) =
        prepare_task_context(&run.prep_context.invocation, task, env_overrides, step_run);
    document_epoch.record_prepared_context_consumer(
        claudine::invocation_context::PreparedContextConsumer::Lifecycle,
    );
    let prewarm = TaskStackPrewarm::new(run, &frontmatter);
    prewarm.submit(&run.prep_context.invocation, task, &task_context);

    // A task stack mutates relative to the document that authored the task,
    // including when that document lives outside the sequence repository.
    let mutation_root = task.origin_dir.clone();
    let engine = EffectEngine::builder().mutation_root(&mutation_root).build();
    let emitter = DefaultLifecycleEmitter;
    let settings = claudine::composition::sequence::task::task_playback_settings();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = crate::log::terminal();
    let live = std::sync::Mutex::new(frontmatter.clone());

    let stack = StackExecutionContext {
        seq: None,
        signal: claudine::composition::LifecycleSignal::Start,
        scope: None,
        frontmatter: &frontmatter,
        live_frontmatter: Some(&live),
        runtime_state: Some(runtime_state),
        err: None,
        timing: None,
        current: Some(run.prep_context.invocation.current_authority().memoized()),
        group: None,
        loop_values: None,
        base_dir: Some(task_source_context.base_dir()),
        ctx_base_dir: Some(run.prep_context.launch_workspace.launch_cwd.as_path()),
        prepared_context: Some(&task_context),
        file_resolution_context: task_source_context.file_resolution_context(),
        effect_engine: &engine,
        // The step's document: its `shell` actions start from its environment.
        shell_runner: run.prep_context.invocation.shell_runner(),
        emitter: &emitter,
        term: &term,
        source_path: &task.origin_path,
        repo_root: task_source_context.repository_root(),
        messaging: &messaging,
        settings: &settings,
    };

    let prompt_runner = WrapperPromptRunner::new(run, env_overrides, target, runtime_state, prewarm);
    let shell = SystemTaskShell::default();
    // `--silent` drops the sink entirely rather than rendering frames nobody
    // reads; the library then skips the render altogether.
    let sink: Option<Arc<dyn TaskStreamSink>> = (!run.silent)
        .then(|| Arc::new(SequenceTaskSink::new()) as Arc<dyn TaskStreamSink>);
    // A lone step already has the step header and footer for attribution, so
    // this stream contributes body framing only — never `open`/`close`. The bar
    // is invisible so serial and group work share one left edge (spec →
    // *Reporting Concurrency*). A `group:` task's members replace it with their
    // own stream, and a group never emits body text of its own.
    let live = sink.as_ref().map(|sink| {
        Arc::new(TaskLiveOutput::new(
            TaskStream::new(
                task.name.clone().unwrap_or_else(|| task.label.clone()),
                TaskBar::Invisible,
                term.clone(),
            ),
            Arc::clone(sink),
        ))
    });

    let outcome = TaskExecution {
        task,
        graph: run.graph,
        stack: &stack,
        state: &state,
        runtime: Some(runtime_state),
        user_setters: run.user_set_overrides.as_ref(),
        overlay: Some(overlay),
        shell: &shell,
        prompt: &prompt_runner,
        interrupt: Some(run.interrupted.as_ref()),
        stream: sink.as_ref(),
        live: live.as_ref(),
        run_evidence: Some(step_run),
        // The step's document: its view already carries the environment the
        // step captured, which the task's approved commands are checked
        // against and its prompt document composes under.
        document: Some(&run.prep_context.invocation),
    }
    .run();

    let agent_perf = prompt_runner.take_perf();
    let tasks = group_task_results(&outcome);
    match outcome.status {
        TaskStatus::Succeeded => StepOutcome::Succeeded {
            provider: prompt_runner.take_provider(),
            agent_perf,
            compose_perf,
            tasks,
        },
        TaskStatus::Interrupted => StepOutcome::Interrupted {
            agent_perf,
            compose_perf,
            tasks,
        },
        TaskStatus::Failed => StepOutcome::Failed {
            message: outcome
                .error
                .as_ref()
                .map_or_else(|| "task failed".to_string(), |d| d.message().to_string()),
            // The task diagnostic already carries the projection `err.*` reads;
            // reuse it rather than re-deriving a second, possibly divergent one.
            snapshot: outcome
                .error
                .as_ref()
                .and_then(|d| d.info.snapshot.clone()),
            agent_perf,
            compose_perf,
            tasks,
        },
    }
}

fn prepare_task_context(
    invocation: &InvocationContext,
    task: &PreflightTask,
    env_overrides: &BTreeMap<String, String>,
    run: &RunEvidence,
) -> (
    SourceContext,
    darkmatter::markdown::compose::ComposeContext,
    claudine::invocation_context::DocumentEpoch,
) {
    let task_source_context = invocation
        .derive_source(&task.origin_path)
        .expect("resolved task document always has a parent directory");
    let scan = task_context_scan(task);
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(&scan);
    // One launch-anchored snapshot per task epoch: launch repository and
    // package facts come from the invocation owner, while the task document's
    // own `SourceContext` (returned alongside) still drives file resolution.
    // The epoch joins the task's run, which its prompt document joins too.
    // `invocation` is the task's document view, so its environment is the one
    // the task captured, never the launch document's.
    let document_epoch = invocation.begin_document_epoch_in(run);
    let mut task_context = document_epoch.capture_launch_context(&requirements);
    for (key, value) in env_overrides {
        task_context.env_mut().insert(key.clone(), value.clone());
    }

    (task_source_context, task_context, document_epoch)
}

/// Every authored surface of `task` that can carry a `{{ … }}` expression,
/// projected into one text [`ContextRequirements::for_content`] can scan.
///
/// The struct patterns below carry no `..` and the action match no `_` arm, so a
/// new authored field or action variant is a compile error here rather than a
/// silent narrowing of the captured runtime context. Under-capturing is a
/// correctness bug — the group is missing from `prepared_context`, so the
/// expression falls through to Darkmatter's lazy ambient `CtxLookup`, the late
/// discovery `fixes/_completed/2026-08-01-propagated-context` (D6) exists to
/// eliminate. Over-capturing only costs a group the invocation cache absorbs.
fn task_context_scan(task: &PreflightTask) -> String {
    let mut scan = String::new();
    push_task_scan(task, &mut scan);
    scan
}

fn push_task_scan(task: &PreflightTask, scan: &mut String) {
    let PreflightTask {
        name,
        label,
        action,
        params,
        timeout,
        operation,
        flow,
        setup,
        teardown,
        // Resolved by preflight to real directories; a path holds no expression.
        origin_dir: _,
        origin_path: _,
        // Authored key names and a JSON Pointer, never expression text.
        authored: _,
        diagnostic: _,
        // Bytes approval already resolved; they hold no expression to read.
        approved_stack_commands: _,
        // Re-resolved from the context preflight approved them under, never
        // from this one.
        preflight_approvals: _,
        position: _,
        // Authored commands the action and stacks above already carry.
        shell_sites: _,
        approved_set_values: _,
    } = task;
    push_json_scan(
        &(name, label, params, timeout, operation, flow, setup, teardown),
        scan,
    );
    push_action_scan(action, scan);
}

fn push_action_scan(action: &PreflightAction, scan: &mut String) {
    match action {
        PreflightAction::Prompt { path, reference } => {
            push_scan(&path.display().to_string(), scan);
            push_scan(reference, scan);
        }
        PreflightAction::Shell { commands } => {
            for command in commands {
                push_scan(command, scan);
            }
        }
        PreflightAction::SideEffect { action, .. } => push_json_scan(action, scan),
        PreflightAction::Sequence(sequence) => {
            for step in &sequence.steps {
                if let Some(task) = &step.task { push_task_scan(task, scan); }
            }
        }
        PreflightAction::Group(group) => {
            let PreflightGroup {
                name,
                execution: _,
                fail_fast: _,
                lifecycle: _,
                max_parallel: _,
                variables,
                tasks,
                origin_path: _,
            } = group;
            push_scan(name, scan);
            push_json_scan(variables, scan);
            for task in tasks {
                push_task_scan(task, scan);
            }
        }
    }
}

/// What every task of `group` mentions, nested groups included: each task's
/// own fields and its prompt document's root page.
///
/// A prompt document is read as it stands now, since an earlier step may have
/// rewritten it; the preflight graph's copy is the fallback.
fn group_requirements(
    group: &PreflightGroup,
    graph: &claudine::composition::sequence::preflight::PreflightGraph,
) -> darkmatter::markdown::compose::ContextRequirements {
    task_requirements(&group.tasks, graph)
}

/// What `tasks` mention, nested groups included: each task's own fields and
/// its prompt document's root page, as [`group_requirements`] reads them.
fn task_requirements(
    tasks: &[PreflightTask],
    graph: &claudine::composition::sequence::preflight::PreflightGraph,
) -> darkmatter::markdown::compose::ContextRequirements {
    let mut scan = String::new();
    // A task's scan already covers nested groups' fields.
    for task in tasks {
        push_task_scan(task, &mut scan);
    }
    let mut documents = darkmatter::markdown::compose::ContextRequirements::default();
    let mut pending: Vec<&PreflightTask> = tasks.iter().collect();
    while let Some(task) = pending.pop() {
        match &task.action {
            PreflightAction::Prompt { path, .. } => {
                let fresh = darkmatter::markdown::Markdown::try_from(path.as_path()).ok();
                let document = fresh
                    .as_ref()
                    .or_else(|| graph.prompt_document(path).map(|document| &document.markdown));
                if let Some(document) = document {
                    documents = documents.union(
                        &darkmatter::markdown::compose::ContextRequirements::for_document(document),
                    );
                }
            }
            PreflightAction::Group(nested) => pending.extend(nested.tasks.iter()),
            PreflightAction::Sequence(nested) => pending.extend(nested.steps.iter().filter_map(|step| step.task.as_ref())),
            PreflightAction::Shell { .. } | PreflightAction::SideEffect { .. } => {}
        }
    }
    darkmatter::markdown::compose::ContextRequirements::for_content(&scan).union(&documents)
}

fn push_json_scan<T: serde::Serialize>(value: &T, scan: &mut String) {
    // An empty scan would silently under-capture; these fields are plain data
    // and always serialize.
    push_scan(
        &serde_json::to_string(value).expect("preflight task fields always serialize"),
        scan,
    );
}

fn push_scan(text: &str, scan: &mut String) {
    if !scan.is_empty() {
        scan.push('\n');
    }
    scan.push_str(text);
}

/// Project a group's member outcomes into the step summary shape.
fn group_task_results(outcome: &TaskOutcome) -> Vec<SequenceTaskResult> {
    outcome
        .group_tasks
        .iter()
        .map(|task| SequenceTaskResult {
            name: task.name.clone(),
            success: task.status == TaskStatus::Succeeded,
            interrupted: task.status == TaskStatus::Interrupted,
            duration: task.duration,
        })
        .collect()
}

/// The effective (composed) frontmatter a task's stacks and `params` read.
fn effective_frontmatter(prepared: &PreparedComposition) -> serde_json::Map<String, Value> {
    prepared
        .effective_frontmatter
        .as_object()
        .cloned()
        .unwrap_or_default()
}

/// Composes and launches a `prompt:` task's document through the wrapper.
struct WrapperPromptRunner<'a> {
    run: &'a SequenceRunContext<'a>,
    env_overrides: &'a BTreeMap<String, String>,
    target: Option<&'a ResolvedExecutionTarget>,
    runtime_state: &'a std::sync::Arc<RuntimeState>,
    /// Submits each group member's stacks when its context is opened.
    prewarm: TaskStackPrewarm<'a>,
    /// The most recent prompt run's perf record and provider.
    ///
    /// Interior mutability because [`PromptTaskRunner::run`] takes `&self` — the
    /// trait is deliberately immutable so the library can hold the runner behind
    /// a shared reference while the task executor runs stages. A `Mutex` rather
    /// than a `RefCell` because a parallel group's members call `run` from
    /// sibling threads; the step summary carries one record, so the last writer
    /// wins exactly as it did when only one task could run at a time.
    last_run: std::sync::Mutex<PromptRunRecord>,
}

/// What a prompt run reports back out of band, for the step status line.
#[derive(Debug, Default)]
struct PromptRunRecord {
    perf: Option<crate::perf::AgentExecutionPerf>,
    provider: Option<claudine::provider::Provider>,
}

impl<'a> WrapperPromptRunner<'a> {
    fn new(
        run: &'a SequenceRunContext<'a>,
        env_overrides: &'a BTreeMap<String, String>,
        target: Option<&'a ResolvedExecutionTarget>,
        runtime_state: &'a std::sync::Arc<RuntimeState>,
        prewarm: TaskStackPrewarm<'a>,
    ) -> Self {
        Self {
            run,
            env_overrides,
            target,
            runtime_state,
            prewarm,
            last_run: std::sync::Mutex::new(PromptRunRecord::default()),
        }
    }

    fn record(&self) -> std::sync::MutexGuard<'_, PromptRunRecord> {
        self.last_run.lock().expect("prompt run record poisoned")
    }

    fn take_perf(&self) -> Option<crate::perf::AgentExecutionPerf> {
        self.record().perf.take()
    }

    fn take_provider(&self) -> Option<claudine::provider::Provider> {
        self.record().provider
    }

    /// The step's target and environment overrides for `document`: its model
    /// re-selected under that document's map (see
    /// [`super::resolve::StepSelection::for_document`]), and `MODEL` naming it.
    fn document_target(
        &self,
        document: &InvocationContext,
    ) -> (Option<ResolvedExecutionTarget>, BTreeMap<String, String>) {
        let target = self.target.cloned().map(|planned| {
            self.run
                .selection
                .for_document(planned, document.document_environment())
        });
        let mut env_overrides = self.env_overrides.clone();
        if let Some(target) = &target {
            match &target.model {
                Some(model) => {
                    env_overrides.insert("MODEL".to_string(), model.clone());
                }
                None => {
                    env_overrides.remove("MODEL");
                }
            }
        }
        (target, env_overrides)
    }
}

impl PromptTaskRunner for WrapperPromptRunner<'_> {
    fn is_dry_run(&self) -> bool { self.run.shared.dry_run }
    fn validate(&self, request: &PromptTaskRequest) -> Result<(), CompositionError> {
        let document = request.document.as_ref().unwrap_or(&self.run.prep_context.invocation);
        let source = composition::resolve_composition_source_in_context(
            &request.path.display().to_string(),
            self.run.compose.file_resolution_context,
        )?;
        let source_context = document.derive_source(&source.resolved_path)
            .expect("resolved prompt document always has a parent directory");
        let mut context = self.run.compose.for_referenced_document(request.inline_compose, &source_context);
        context.lifecycle_group = request.lifecycle_group.clone();
        context.invocation = document;
        context.run_evidence = request.run_evidence.as_ref();
        context.deferred_shell_roots = request.deferred_shell_roots.clone();
        context.document_shell_values = Some(request.document_shell_values.clone());
        let (_, env) = self.document_target(document);
        let mut overrides = request.set_overrides.clone();
        if request.body_only { super::container::mask_body_events(&mut overrides); }
        let options = super::jit::step_prepare_options(
            &source, &context, overrides, &env, false,
        );
        let lifecycle = composition::validate_document_shell_late_values(&source, &options)?;
        let approval_options = super::super::apply_composition_shell_overrides(
            super::super::build_harness_shell_options_for_source_with_cache(
                &source.resolved_path, context.source_repo_root,
                Some(Arc::clone(&context.approval_cache)),
            ), context.shared.dry_run, context.shared.yolo,
        );
        let mut approval_options = approval_options;
        approval_options.closed_approvals = Some(self.run.approved.clone());
        approval_options.approval_handler = None;
        composition::resolve_shell_approvals(None, None, &approval_options, Some(&lifecycle), Some(&source.resolved_path))?;
        *request.document_epoch.lock().expect("prompt document epoch poisoned") = options.document_epoch.clone();
        *request.prepared_context.lock().expect("prompt document context poisoned") = options.prepared_context.clone();
        Ok(())
    }

    fn open_member_run(&self, task: &PreflightTask) -> Option<TaskRun> {
        let run = RunEvidence::default();
        // A serial leg is a document of its own: it captures its environment
        // once, here, before any of its stages run.
        let document = self.run.prep_context.invocation.capture_document();
        let (_, env_overrides) = self.document_target(&document);
        let (_, context, epoch) = prepare_task_context(&document, task, &env_overrides, &run);
        epoch.record_prepared_context_consumer(
            claudine::invocation_context::PreparedContextConsumer::Lifecycle,
        );
        self.prewarm.submit(&document, task, &context);
        Some(TaskRun {
            context,
            run,
            document: Some(document),
        })
    }

    fn refresh_member_run(&self, task: &PreflightTask, previous: &TaskRun) -> TaskRun {
        let document = previous.document.as_ref().expect("serial members own a document");
        let run = RunEvidence::default();
        let (_, env_overrides) = self.document_target(document);
        let (_, context, epoch) = prepare_task_context(document, task, &env_overrides, &run);
        epoch.record_prepared_context_consumer(
            claudine::invocation_context::PreparedContextConsumer::Lifecycle,
        );
        self.prewarm.submit(document, task, &context);
        TaskRun { context, run, document: Some(document.clone()) }
    }

    fn open_leg_run(&self, task: &PreflightTask, group: Option<&TaskRun>) -> Option<TaskRun> {
        // A parallel leg captures its own environment but joins the group's
        // run, so siblings share one observation of the working tree and
        // nothing mutable of one another's.
        let run = group.map_or_else(RunEvidence::default, |group| group.run.clone());
        let document = self.run.prep_context.invocation.capture_document();
        let epoch = document.begin_document_epoch_in(&run);
        let mut context = epoch.capture_launch_context(&task_requirements(
            std::slice::from_ref(task),
            self.run.graph,
        ));
        let (_, env_overrides) = self.document_target(&document);
        for (key, value) in &env_overrides {
            context.env_mut().insert(key.clone(), value.clone());
        }
        epoch.record_prepared_context_consumer(
            claudine::invocation_context::PreparedContextConsumer::Lifecycle,
        );
        self.prewarm.submit(&document, task, &context);
        Some(TaskRun {
            context,
            run,
            document: Some(document),
        })
    }

    fn open_group_run(&self, group: &PreflightGroup) -> Option<TaskRun> {
        let run = RunEvidence::default();
        let invocation = &self.run.prep_context.invocation;
        // Capturing the union here observes every Git group any sibling
        // names, once, before the first sibling starts.
        let epoch = invocation.begin_document_epoch_in(&run);
        let mut context = epoch.capture_launch_context(&group_requirements(group, self.run.graph));
        for (key, value) in self.env_overrides {
            context.env_mut().insert(key.clone(), value.clone());
        }
        epoch.record_prepared_context_consumer(
            claudine::invocation_context::PreparedContextConsumer::Lifecycle,
        );
        // The group is not a document: its shared run is captured under the
        // step's view, and each leg enters its own.
        Some(TaskRun {
            context,
            run,
            document: None,
        })
    }

    fn run(&self, request: &PromptTaskRequest) -> Result<PromptRunOutcome, CompositionError> {
        *self.record() = PromptRunRecord::default();

        // The document the task runs in: a group leg's own view, or the
        // step's. Everything below — composition, the prompt document's
        // lifecycle, and the launch — reads that document's environment.
        // The task's `params` are injected into its prompt document per task,
        // so pre-warm treats them as late there.
        let document = request
            .document
            .clone()
            .unwrap_or_else(|| self.run.prep_context.invocation.clone())
            .with_prewarm_injected_keys(request.set_overrides.values().keys().cloned());
        // Just-in-time, like the step itself: the referenced document is read
        // now, so a document an earlier step rewrote is composed as it stands.
        let source = composition::resolve_composition_source_in_context(
            &request.path.display().to_string(),
            self.run.compose.file_resolution_context,
        )?;
        let prompt_source_context = document
            .derive_source(&source.resolved_path)
            .expect("resolved prompt document always has a parent directory");
        let params_source_context = document
            .derive_source(&request.params_origin_path)
            .expect("a preflight task always has an authoring source");
        let mut caller_input_records =
            claudine::composition::CallerInputLayers::from_caller_overrides(
                Some(Value::Object(request.params.clone())),
                params_source_context.file_resolution_context().clone(),
            )
            .caller_input_records;
        caller_input_records.extend(self.run.compose.caller_input_records.clone());
        if let Some(runtime) = &request.runtime {
            for key in runtime.snapshot().mutations.keys() {
                caller_input_records.remove(key);
            }
        }
        for key in claudine::composition::sequence::reserved::ROOT_OVERLAY_KEYS {
            caller_input_records.remove(*key);
        }
        let mut prompt_compose_context = self
            .run
            .compose
            .for_referenced_document(request.inline_compose, &prompt_source_context);
        prompt_compose_context.lifecycle_group = request.lifecycle_group.clone();
        prompt_compose_context.document_epoch = request.document_epoch.lock().expect("prompt document epoch poisoned").clone();
        prompt_compose_context.prepared_context = request.prepared_context.lock().expect("prompt document context poisoned").clone();
        prompt_compose_context.caller_input_records = &caller_input_records;
        prompt_compose_context.run_evidence = request.run_evidence.as_ref();
        prompt_compose_context.invocation = &document;
        prompt_compose_context.deferred_shell_roots = request.deferred_shell_roots.clone();
        prompt_compose_context.document_shell_values = Some(request.document_shell_values.clone());
        let (document_target, document_env_overrides) = self.document_target(&document);
        let mut overrides = request.set_overrides.clone();
        if request.body_only { super::container::mask_body_events(&mut overrides); }
        let composed = super::jit::compose_step(
            &source,
            &prompt_compose_context,
            &overrides,
            &document_env_overrides,
            self.run.approved.clone(),
            // A referenced prompt document *is* its body; composing it to
            // nothing is the error the empty-body guard exists to name.
            false,
        )?;

        let shared = self.run.shared;
        let resolved =
            shared.resolve_session_interactivity(composed.prepared.selection_hints.interactive);
        let mut env_overrides = document_env_overrides;
        let mut task_owned_env = super::jit::step_owned_env(self.run.effective_fail_fast);
        if let Some(operation) = &request.operation {
            env_overrides.insert("OPERATION".to_string(), operation.clone());
            task_owned_env.insert("OPERATION".to_string(), operation.clone());
        }

        // This task's own handoff chain. Each run of a task starts one, so
        // parallel siblings never share a chain and two of them may adopt the
        // same target; a cycle is still a cycle within one task (R8).
        let task_ledger: SharedRunLedger = Arc::new(Mutex::new(RunLedger::new(
            source.resolved_path.clone(),
            Arc::clone(&self.run.compose.approval_cache),
        )));
        let runtime_state = request
            .runtime
            .clone()
            .unwrap_or_else(|| std::sync::Arc::clone(self.runtime_state));
        let task_file_resolution_context =
            prompt_source_context.file_resolution_context().clone();
        let task_caller_input_records = caller_input_records.clone();

        let execution = CompositionExecutionRequest {
            mode: if request.inline_compose {
                CompositionMode::InlineFrontmatterPrompt
            } else {
                CompositionMode::ChainedDocument
            },
            file_ref: request.reference.clone(),
            prepared: composed.prepared,
            resolved_target: document_target,
            explicit_provider: shared.explicit_provider(),
            excluded: shared.excluded(),
            sequence: true,
            yolo: shared.yolo,
            include: shared.include.clone(),
            model: shared.model.clone(),
            output: shared.output,
            system_prompt_args: SystemPromptArgs {
                append_file: shared.append_system_prompt.clone(),
                replace_file: shared.replace_system_prompt.clone(),
            },
            timeout: shared.timeout.clone(),
            step_timeout: shared.step_timeout.clone(),
            stall_timeout: shared.stall_timeout.clone(),
            operation: request.operation.clone().or_else(|| shared.operation.clone()),
            sandbox: shared.sandbox,
            repo: shared.repo,
            dry_run: shared.dry_run,
            mcp: shared.mcp,
            mcp_use: shared.mcp_use.clone(),
            strict: shared.strict,
            session_interactive: resolved.value,
            session_interactive_source: resolved.source,
            quiet: shared.quiet,
            silent: shared.silent,
            env_overrides,
            closed_shell_approvals: Some(self.run.approved.clone()),
            shared_approval_cache: Some(Arc::clone(&self.run.compose.approval_cache)),
            installed_snapshot: Some(self.run.prep_context.installed_snapshot.clone()),
            prep_env_context: Some(document.environment_context()),
            invocation_context: Some(document.clone()),
            source_context: Some(prompt_source_context),
            prep_launch_workspace: Some(self.run.prep_context.launch_workspace.clone()),
            prep_launch_context: Some(self.run.prep_context.launch_context.clone()),
            prep_launch_detection_error: self.run.prep_context.launch_detection_error.clone(),
            header_emitted: false,
            provider_tail: shared.provider_tail.clone(),
            provider_tail_notices: shared.provider_tail_notices.clone(),
            // The task executor's cell, not the sequence's: a parallel group
            // member hands over its private buffer, so the launched document's
            // own lifecycle `set` stays invisible to its siblings.
            runtime_state: Some(std::sync::Arc::clone(&runtime_state)),
            // The task executor publishes the entry after `teardown`, not here.
            suppress_output_commit: true,
            // Carries this task's bar to the thread draining the child's stdout.
            task_frame_writer: request.frame_writer.clone(),
            proxy_overlay: indexmap::IndexMap::new(),
            handoff_ledger: Some(Arc::clone(&task_ledger)),
            adopted_handoff: None,
        };

        let kind = if request.inline_compose {
            CompositionKind::Inline
        } else {
            CompositionKind::Direct
        };
        // A handoff the document raised stays inside this task: the target
        // keeps the task's inputs, cell, output policy, and bar, and its result
        // is the task's. Setup, teardown, and output publication belong to the
        // task executor around this call, so each still happens once.
        let task_scope = StepScope {
            shell_approvals: Some(self.run.approved.clone()),
            lifecycle_group: request.lifecycle_group.clone(),
            caller_overrides: request.set_overrides.clone(),
            runtime_state,
            suppress_output_commit: true,
            task_frame_writer: request.frame_writer.clone(),
            env_overrides: task_owned_env,
            operation: request.operation.clone(),
        };
        let outcome = execute_composition_request_inner(
            execution,
            self.run.verbose,
            None,
            self.run.perf_enabled,
        )
        .and_then(|mut outcome| {
            if let Some(surfaced) = outcome.handoff.take() {
                (outcome.exit_code, outcome.final_output) = super::iterate::run_step_proxy_loop(
                    surfaced,
                    &task_ledger,
                    self.run,
                    kind,
                    &task_scope,
                    task_caller_input_records,
                    &task_file_resolution_context,
                )?;
            }
            Ok(outcome)
        })
        .map_err(|error| CompositionError::SequenceTaskPromptLaunch {
            task: request.reference.clone(),
            path: request.path.clone(),
            message: error.to_string(),
            // `Report` boxes its source rather than discarding it, so the typed
            // diagnostic the wrapper raised is still reachable by downcast here
            // — the last point that has it.
            snapshot: DiagnosticSnapshot::select(error.as_ref()).map(Box::new),
        })?;

        *self.record() = PromptRunRecord {
            perf: outcome.agent_perf,
            provider: Some(outcome.provider),
        };

        Ok(PromptRunOutcome {
            stdout: outcome.final_output.unwrap_or_default(),
            exit_code: outcome.exit_code,
            interrupted: super::run_was_interrupted(outcome.exit_code, self.run.interrupted),
        })
    }
}

#[cfg(test)]
mod tests;
