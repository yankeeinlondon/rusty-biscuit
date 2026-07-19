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
use claudine::composition::sequence::preflight::PreflightTask;
use claudine::composition::sequence::task::{
    PromptRunOutcome, PromptTaskRequest, PromptTaskRunner, SystemTaskShell, TaskExecution,
    TaskOutcome, TaskStatus,
};
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
            return StepOutcome::Failed {
                message: CompositionError::PreFlightStateBuildFailed { source }.to_string(),
                agent_perf: None,
                compose_perf,
                tasks: Vec::new(),
            };
        }
    };

    // The engine's mutation root is the document's directory: a `set_frontmatter`
    // in a task stack targets files next to the document that authored it.
    let mutation_root = prepared
        .resolved_path
        .parent()
        .unwrap_or(&prepared.resolved_path)
        .to_path_buf();
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
        base_dir: prepared.resolved_path.parent(),
        ctx_base_dir: Some(run.prep_context.launch_workspace.launch_cwd.as_path()),
        prepared_context: None,
        effect_engine: &engine,
        shell_runner: &SystemShellRunner,
        emitter: &emitter,
        term: &term,
        source_path: &prepared.resolved_path,
        repo_root: run.prep_context.source_repo_root.as_deref(),
        messaging: &messaging,
        settings: &settings,
    };

    let prompt_runner = WrapperPromptRunner::new(run, env_overrides, target, runtime_state);
    let shell = SystemTaskShell;
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
        TaskLiveOutput::new(
            TaskStream::new(
                task.name.clone().unwrap_or_else(|| task.label.clone()),
                TaskBar::Invisible,
                term.clone(),
            ),
            Arc::clone(sink),
        )
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
            agent_perf,
            compose_perf,
            tasks,
        },
    }
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
        let source = composition::resolve_composition_source(&request.path.display().to_string())?;
        let composed = super::jit::compose_step(
            &source,
            &self
                .run
                .compose
                .for_referenced_document(request.inline_compose),
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
