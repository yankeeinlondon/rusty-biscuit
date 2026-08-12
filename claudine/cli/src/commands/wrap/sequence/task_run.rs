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
use std::sync::Arc;

use claudine::composition::lifecycle_executor::{StackExecutionContext, SystemShellRunner};
use claudine::composition::{
    self, CompositionError, CompositionExecutionRequest, CompositionMode, DefaultLifecycleEmitter,
    PreparedComposition, ResolvedExecutionTarget, RuntimeState, SequenceTaskResult,
};
use claudine::composition::sequence::preflight::{PreflightAction, PreflightGroup, PreflightTask};
use claudine::composition::sequence::task::{
    PromptRunOutcome, PromptTaskRequest, PromptTaskRunner, SystemTaskShell, TaskExecution,
    TaskOutcome, TaskStatus,
};
use claudine::diagnostics::DiagnosticSnapshot;
use claudine::invocation_context::{InvocationContext, SourceContext};
use claudine::render::{TaskBar, TaskLiveOutput, TaskStream, TaskStreamSink};
use claudine::system_prompt::SystemPromptArgs;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::EffectiveStateBuilder;
use serde_json::Value;

use crate::commands::wrap::composition::execute_composition_request_inner;

use super::iterate::{SequenceRunContext, StepOutcome};
use super::task_frames::SequenceTaskSink;

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

    let (task_source_context, task_context) =
        prepare_task_context(
            &run.prep_context.invocation,
            task,
            &prepared.compose_context,
            run.compose.launch_requirements,
            env_overrides,
        );

    // A task stack mutates relative to the document that authored the task,
    // including when that document lives outside the sequence repository.
    let mutation_root = task.origin_dir.clone();
    let engine = EffectEngine::builder().mutation_root(&mutation_root).build();
    let emitter = DefaultLifecycleEmitter;
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = crate::log::terminal();
    let live = std::sync::Mutex::new(frontmatter.clone());

    let stack = StackExecutionContext {
        signal: claudine::composition::LifecycleSignal::Start,
        frontmatter: &frontmatter,
        live_frontmatter: Some(&live),
        runtime_state: Some(runtime_state),
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: Some(task_source_context.base_dir()),
        ctx_base_dir: Some(run.prep_context.launch_workspace.launch_cwd.as_path()),
        prepared_context: Some(&task_context),
        file_resolution_context: Some(task_source_context.file_resolution_context()),
        effect_engine: &engine,
        shell_runner: &SystemShellRunner,
        emitter: &emitter,
        term: &term,
        source_path: &task.origin_path,
        repo_root: task_source_context.repository_root(),
        messaging: &messaging,
        settings: &settings,
    };

    let prompt_runner = WrapperPromptRunner::new(run, env_overrides, target, runtime_state);
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
    epoch_context: &darkmatter::markdown::compose::ComposeContext,
    captured_requirements: &darkmatter::markdown::compose::ContextRequirements,
    env_overrides: &BTreeMap<String, String>,
) -> (SourceContext, darkmatter::markdown::compose::ComposeContext) {
    let task_source_context = invocation
        .derive_source(&task.origin_path)
        .expect("resolved task document always has a parent directory");
    let scan = task_context_scan(task);
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(&scan);
    let mut task_context = epoch_context.clone();
    let mut captured = captured_requirements.clone();
    invocation.extend_launch_context(
        &mut task_context,
        &mut captured,
        &requirements,
    );
    for (key, value) in env_overrides {
        task_context.env_mut().insert(key.clone(), value.clone());
    }

    (task_source_context, task_context)
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
        PreflightAction::SideEffect { action } => push_json_scan(action, scan),
        PreflightAction::Group(group) => {
            let PreflightGroup {
                name,
                execution: _,
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

fn push_json_scan<T: serde::Serialize>(value: &T, scan: &mut String) {
    push_scan(&serde_json::to_string(value).unwrap_or_default(), scan);
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
    ) -> Self {
        Self {
            run,
            env_overrides,
            target,
            runtime_state,
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
}

impl PromptTaskRunner for WrapperPromptRunner<'_> {
    fn run(&self, request: &PromptTaskRequest) -> Result<PromptRunOutcome, CompositionError> {
        *self.record() = PromptRunRecord::default();

        // Just-in-time, like the step itself: the referenced document is read
        // now, so a document an earlier step rewrote is composed as it stands.
        let source = composition::resolve_composition_source_in_context(
            &request.path.display().to_string(),
            self.run.compose.file_resolution_context,
        )?;
        let prompt_source_context = self
            .run
            .prep_context
            .invocation
            .derive_source(&source.resolved_path)
            .expect("resolved prompt document always has a parent directory");
        let prompt_compose_context = self
            .run
            .compose
            .for_referenced_document(request.inline_compose, &prompt_source_context);
        let composed = super::jit::compose_step(
            &source,
            &prompt_compose_context,
            &request.set_overrides,
            self.env_overrides,
            self.run.approved.clone(),
            // A referenced prompt document *is* its body; composing it to
            // nothing is the error the empty-body guard exists to name.
            false,
        )?;

        let shared = self.run.shared;
        let resolved =
            shared.resolve_session_interactivity(composed.prepared.selection_hints.interactive);
        let mut env_overrides = self.env_overrides.clone();
        if let Some(operation) = &request.operation {
            env_overrides.insert("OPERATION".to_string(), operation.clone());
        }

        let execution = CompositionExecutionRequest {
            mode: if request.inline_compose {
                CompositionMode::InlineFrontmatterPrompt
            } else {
                CompositionMode::ChainedDocument
            },
            file_ref: request.reference.clone(),
            prepared: composed.prepared,
            resolved_target: self.target.cloned(),
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
            shared_approval_cache: Some(Arc::clone(&self.run.compose.approval_cache)),
            installed_snapshot: Some(self.run.prep_context.installed_snapshot.clone()),
            invocation_context: Some(self.run.prep_context.invocation.clone()),
            source_context: Some(prompt_source_context),
            epoch_context_requirements: None,
            prep_launch_workspace: Some(self.run.prep_context.launch_workspace.clone()),
            prep_launch_context: Some(self.run.prep_context.launch_context.clone()),
            prep_env_context: Some(self.run.prep_context.env_context.clone()),
            prep_launch_detection_error: self.run.prep_context.launch_detection_error.clone(),
            header_emitted: false,
            provider_args: shared.provider_args.clone(),
            provider_args_explicit: shared.provider_args_explicit,
            // The task executor's cell, not the sequence's: a parallel group
            // member hands over its private buffer, so the launched document's
            // own lifecycle `set` stays invisible to its siblings.
            runtime_state: request
                .runtime
                .clone()
                .or_else(|| Some(std::sync::Arc::clone(self.runtime_state))),
            // The task executor publishes the entry after `teardown`, not here.
            suppress_output_commit: true,
            // Carries this task's bar to the thread draining the child's stdout.
            task_frame_writer: request.frame_writer.clone(),
            proxy_overlay: indexmap::IndexMap::new(),
            handoff_ledger: None,
            adopted_handoff: None,
        };

        let outcome = execute_composition_request_inner(
            execution,
            self.run.verbose,
            None,
            self.run.perf_enabled,
        )
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
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use claudine::composition::sequence::preflight::{GroupExecution, PreflightAction};
    use serde_json::{Map, json};

    use super::*;

    /// A `ctx.*` reference whose group (`Repo`) is never captured by default, so
    /// its presence in the composed context proves the field was scanned.
    const CTX_REF: &str = "{{ ctx.repo_root }}";

    fn init_git_repo(path: &Path) {
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(path)
            .status()
            .expect("git must be available for repository-context tests");
        assert!(status.success());
    }

    /// A sequence launched in one repository authoring its task in another.
    struct Fixture {
        _root: tempfile::TempDir,
        launch_repo: PathBuf,
        task_repo: PathBuf,
        task_dir: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let launch_repo = root.path().join("launch");
            let task_repo = root.path().join("tasks");
            let task_dir = task_repo.join("nested");
            std::fs::create_dir_all(&launch_repo).unwrap();
            std::fs::create_dir_all(&task_dir).unwrap();
            init_git_repo(&launch_repo);
            init_git_repo(&task_repo);
            Self {
                _root: root,
                launch_repo,
                task_repo,
                task_dir,
            }
        }

        fn origin_path(&self) -> PathBuf {
            self.task_dir.join("task.yaml")
        }

        /// A task carrying no expression at all, for a caller to plant one in.
        fn task(&self) -> PreflightTask {
            PreflightTask {
                name: Some("external".to_string()),
                label: "external".to_string(),
                action: PreflightAction::Shell {
                    commands: vec!["true".to_string()],
                },
                params: Map::new(),
                timeout: None,
                operation: None,
                flow: None,
                setup: None,
                teardown: None,
                origin_dir: self.task_dir.clone(),
                origin_path: self.origin_path(),
            }
        }
    }

    fn prepare_for_test(
        invocation: &InvocationContext,
        task: &PreflightTask,
        env: &BTreeMap<String, String>,
    ) -> (SourceContext, darkmatter::markdown::compose::ComposeContext) {
        let requirements = darkmatter::markdown::compose::ContextRequirements::for_content("");
        let context = invocation.capture_launch_context(&requirements);
        prepare_task_context(invocation, task, &context, &requirements, env)
    }

    #[test]
    fn task_stack_keeps_source_resolution_but_uses_the_launch_repository_context() {
        let fixture = Fixture::new();
        let origin_path = fixture.origin_path();
        let mut task = fixture.task();
        task.action = PreflightAction::SideEffect {
            action: json!({"stderr": CTX_REF}),
        };
        let invocation = InvocationContext::capture_at(&fixture.launch_repo);
        let env = BTreeMap::from([("TASK_MARKER".to_string(), "owned".to_string())]);

        let (source, context) = prepare_for_test(&invocation, &task, &env);

        assert_eq!(source.base_dir(), fixture.task_dir);
        assert_eq!(source.repository_root(), Some(fixture.task_repo.as_path()));
        assert_eq!(
            source.file_resolution_context().source_path(),
            Some(origin_path.as_path())
        );
        assert_eq!(
            source.file_resolution_context().repository_root(),
            Some(fixture.task_repo.as_path())
        );
        assert_eq!(
            context.get("repo_root").and_then(Value::as_str),
            Some(fixture.launch_repo.to_string_lossy().as_ref())
        );
        assert_eq!(context.env().get("TASK_MARKER").map(String::as_str), Some("owned"));
        let work = invocation.work_snapshot();
        assert_eq!(work.launch_context_constructions, 1);
        assert_eq!(work.launch_context_extensions, 1);
        assert_eq!(work.ambient_fallbacks, 0);
    }

    /// The control the per-field cases are read against: without a reference the
    /// group is genuinely absent, so `repo_root` being present below can only
    /// have come from the planted expression.
    #[test]
    fn a_task_with_no_ctx_reference_captures_no_repository_group() {
        let fixture = Fixture::new();
        let invocation = InvocationContext::capture_at(&fixture.launch_repo);

        let (_, context) = prepare_for_test(&invocation, &fixture.task(), &BTreeMap::new());

        assert_eq!(context.get("repo_root"), None);
    }

    #[test]
    fn every_expression_bearing_field_is_scanned_for_ctx_references() {
        let fixture = Fixture::new();
        let invocation = InvocationContext::capture_at(&fixture.launch_repo);

        #[allow(clippy::type_complexity)]
        let cases: Vec<(&str, Box<dyn Fn(&mut PreflightTask)>)> = vec![
            (
                "action (side_effect)",
                Box::new(|task| {
                    task.action = PreflightAction::SideEffect {
                        action: json!({"stderr": CTX_REF}),
                    };
                }),
            ),
            (
                "action (shell)",
                Box::new(|task| {
                    task.action = PreflightAction::Shell {
                        commands: vec![format!("echo {CTX_REF}")],
                    };
                }),
            ),
            (
                "action (prompt reference)",
                Box::new(|task| {
                    task.action = PreflightAction::Prompt {
                        path: PathBuf::from("/resolved/prompt.md"),
                        reference: format!("{CTX_REF}/prompt.md"),
                    };
                }),
            ),
            (
                "params",
                Box::new(|task| {
                    task.params.insert("target".to_string(), json!(CTX_REF));
                }),
            ),
            ("timeout", Box::new(|task| task.timeout = Some(json!(CTX_REF)))),
            (
                "operation",
                Box::new(|task| task.operation = Some(CTX_REF.to_string())),
            ),
            ("flow", Box::new(|task| task.flow = Some(CTX_REF.to_string()))),
            (
                "setup",
                Box::new(|task| task.setup = Some(json!([{"shell": format!("echo {CTX_REF}")}]))),
            ),
            (
                "teardown",
                Box::new(|task| {
                    task.teardown = Some(json!([{"shell": format!("echo {CTX_REF}")}]));
                }),
            ),
            (
                "name",
                Box::new(|task| task.name = Some(CTX_REF.to_string())),
            ),
            ("label", Box::new(|task| task.label = CTX_REF.to_string())),
        ];

        for (field, plant) in cases {
            let mut task = fixture.task();
            plant(&mut task);

            let (_, context) = prepare_for_test(&invocation, &task, &BTreeMap::new());

            assert_eq!(
                context.get("repo_root").and_then(Value::as_str),
                Some(fixture.launch_repo.to_string_lossy().as_ref()),
                "`{field}` was not scanned for runtime-context references"
            );
        }
    }

    #[test]
    fn group_variables_and_member_tasks_are_scanned_for_ctx_references() {
        let fixture = Fixture::new();
        let invocation = InvocationContext::capture_at(&fixture.launch_repo);

        let group_of = |variables: Map<String, Value>, member: PreflightTask| PreflightAction::Group(
            PreflightGroup {
                name: "batch".to_string(),
                execution: GroupExecution::Serial,
                max_parallel: None,
                variables,
                tasks: vec![member],
                origin_path: fixture.origin_path(),
            },
        );

        let mut member = fixture.task();
        member.teardown = Some(json!([{"shell": format!("echo {CTX_REF}")}]));

        for (label, action) in [
            (
                "group variables",
                group_of(
                    Map::from_iter([("target".to_string(), json!(CTX_REF))]),
                    fixture.task(),
                ),
            ),
            ("group member task", group_of(Map::new(), member)),
        ] {
            let mut task = fixture.task();
            task.action = action;

            let (_, context) = prepare_for_test(&invocation, &task, &BTreeMap::new());

            assert_eq!(
                context.get("repo_root").and_then(Value::as_str),
                Some(fixture.launch_repo.to_string_lossy().as_ref()),
                "`{label}` was not scanned for runtime-context references"
            );
        }
    }
}
