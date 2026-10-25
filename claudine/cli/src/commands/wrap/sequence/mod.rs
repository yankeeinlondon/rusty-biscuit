//! Serial sequence orchestrator.

use claudine::composition::LifecycleErrorInfoExt;
use std::collections::{HashMap, HashSet};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::{
    self, CompositionError, MissingProperty, ResolvedCompositionSource, SequenceExecutionOptions,
    SequenceMissingPropertiesStep, SequencePlan, SequenceStepDraft,
};
use claudine::composition::preflight::checks::{AGENT_RESOLUTION, LIFECYCLE_SHELL_AUDIT, sequence_preflight};
use color_eyre::eyre::Result;
use darkmatter::lifecycle::preflight::CheckStage;
use darkmatter::lifecycle::run::EntryReason;
use tracing::info_span;

use crate::commands::compose::SharedComposeArgs;
use crate::log;

mod container;
mod iterate;
mod jit;
mod phase1c;
mod providers;
mod report;
mod resolve;
mod review;
mod task_frames;
mod task_prewarm;
mod task_run;

use jit::StepComposeContext;
#[cfg(test)]
use phase1c::find_first_unsupported;
use phase1c::run_phase_1c_with_schema;
use resolve::apply_user_set_to_hints;

/// Exit code emitted when Ctrl+C is observed during a sequence run.
/// Matches the standard `128 + SIGINT(2)` convention used by shells.
pub(super) const SEQUENCE_INTERRUPT_EXIT_CODE: i32 = 130;

/// Whether a finished provider run should be recorded as user-interrupted.
///
/// Exit code `130` is one witness, not the contract. It holds for a
/// single-press, SIGINT-terminated child on Unix and nowhere else: the second
/// press sends `SIGKILL` (`137`), a provider that traps `SIGINT` may exit `0`
/// or `1`, and neither Windows rung yields `130` — `CTRL_BREAK_EVENT` gives
/// `0xC000013A` and `TerminateJobObject` gives `1`. The sequence-scoped flag is
/// the host-independent signal, with a SIGINT handler producing it on Unix and
/// the console coordinator on Windows, so both the step path and the group-task
/// path must consult it before calling a run merely failed.
pub(super) fn run_was_interrupted(exit_code: i32, interrupted: &AtomicBool) -> bool {
    exit_code == SEQUENCE_INTERRUPT_EXIT_CODE || interrupted.load(Ordering::SeqCst)
}

/// Approve every shell command the preflight graph can reach.
///
/// The graph's resolved-byte commands arrive already resolved, so what the
/// gate approves is byte-identical to what runs; every other task command is
/// approved by its typed shape. Referenced prompt documents contribute their
/// `::shell` directives through the same pass, once per reference: each
/// declares the sequence's late roots (and the task's late `with` keys) to
/// Darkmatter's discovery, carries the task's approval-time `with` as data,
/// and, for a later task, requires every command with a placeholder to be a
/// template. That is what makes "no approval prompts once the sequence starts"
/// honest for work several hops from the sequence document. Schema verdicts are
/// deferred during this discovery-only compose because prompt-task `with`
/// binds just in time; the task's canonical prepare validates the resulting
/// effective values.
#[allow(clippy::too_many_arguments, clippy::result_large_err)]
fn approve_preflight_graph(
    graph: &composition::PreflightGraph,
    source: &ResolvedCompositionSource,
    source_context: &claudine::invocation_context::SourceContext,
    approval_cache: composition::SharedApprovalCache,
    shared: &SharedComposeArgs,
    launch_area: Option<&std::path::Path>,
    invocation: &claudine::invocation_context::InvocationContext,
    graph_context: &darkmatter::markdown::compose::ComposeContext,
    closed: Option<&darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals>,
) -> Result<darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals> {
    if graph.shell_commands.is_empty()
        && graph.template_candidates.is_empty()
        && graph.prompt_documents.is_empty()
    {
        return Ok(closed.cloned().unwrap_or_default());
    }

    let approval_options = super::apply_composition_shell_overrides(
        super::build_harness_shell_options_for_source_with_cache(
            &source.resolved_path,
            source_context.repository_root(),
            Some(approval_cache),
        ),
        shared.dry_run,
        shared.yolo,
    );

    let mut approval_options = approval_options;
    if let Some(approved) = closed {
        approval_options.closed_approvals = Some(approved.clone());
        approval_options.approval_handler = None;
    }

    // The document the graph walk is composing, so a failure can name it.
    let composing = std::cell::RefCell::new(None::<std::path::PathBuf>);
    // This approval pass is one discovery run: every referenced document (and
    // every file it includes) that names a Git fact reads one observation of
    // it. Extending through the invocation instead makes each extension a run
    // of its own, which re-observed the working tree once per prompt document.
    let approval_epoch = invocation.begin_document_epoch();
    let compose_options = |reference: &composition::PromptReference| -> Result<
        darkmatter::markdown::compose::ComposeRequest,
        claudine::composition::CompositionError,
    > {
        let path = reference.path.as_path();
        composing.replace(Some(path.to_path_buf()));
        let source_context = invocation
            .derive_source(path)
            .expect("resolved prompt document always has a parent directory");
        let document = darkmatter::markdown::Markdown::try_from(path)
            .expect("preflight graph already loaded the prompt document");
        // The graph's launch snapshot, extended with any group this referenced
        // document needs: plain `ctx.*` stays launch-anchored no matter where
        // the prompt document is stored, while the document's own
        // `SourceContext` below still drives its file resolution.
        let requirements =
            darkmatter::markdown::compose::ContextRequirements::for_document(&document);
        let mut context = graph_context.clone();
        approval_epoch.extend_launch_context(&mut context, &requirements);
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(context)
            .with_context_authority(approval_epoch.compose_context_authority())
            .with_source_file(path)
            .with_deferred_schema_verdict(true)
            // Defer the lifecycle subtree exactly as the per-step template
            // preflight does: this pass exists to discover `::shell`
            // directives, and resolving a deferred `success:`/`failure:` read
            // -side file reference here would trip on a file that event has
            // not created yet.
            .with_exclude_keys(claudine::composition::claudine_registry().authored_event_keys())
            .with_deferred_shell_roots(reference.deferred_shell_roots());
        // The task's approval-time `with` reaches its document as data, as
        // it does at the task's turn.
        if !reference.with.is_empty() {
            opts = composition::LayeredOverrides::data(Some(&serde_json::Value::Object(
                reference.with.clone(),
            )))
            .apply_to(opts);
        }
        if let Some(area) = launch_area {
            opts = opts.with_file_ref_fallback_dir(area.to_path_buf());
        }
        Ok(claudine::composition::compose_request(
            opts,
            source_context.file_resolution_context().clone(),
        )?)
    };

    let result =
        composition::resolve_graph_shell_approvals(graph, &approval_options, &compose_options)
            .inspect_err(|error| {
                if is_missing_include(error)
                    && let Some(document) = composing.borrow().as_deref()
                {
                    emit_missing_include_note(document);
                }
            })?;
    Ok(result.approved_commands)
}

/// Submit every early lifecycle phrase the preflight graph reaches for TTS
/// pre-warm, before step one launches.
///
/// Settings load only when pre-warm is live; submission never waits for
/// synthesis.
fn submit_preflight_prewarm(
    graph: &composition::PreflightGraph,
    source: &ResolvedCompositionSource,
    user_set_overrides: Option<&serde_json::Value>,
    repo_root: Option<&std::path::Path>,
    invocation: &claudine::invocation_context::InvocationContext,
    graph_context: &darkmatter::markdown::compose::ComposeContext,
) {
    let handle = invocation.prewarm();
    if !handle.is_enabled() {
        return;
    }
    // As lifecycle playback does, an unreadable configuration plays with the
    // defaults.
    let settings = claudine::dispatch::loader::load_claudine_config(None, repo_root)
        .map(|config| claudine::dispatch::loader::bridge_tts_settings(&config))
        .unwrap_or_default();
    let caller_keys = user_set_overrides
        .and_then(serde_json::Value::as_object)
        .map(|setters| setters.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    claudine::composition::lifecycle::prewarm::submit_sequence_preflight(
        handle,
        &settings,
        graph,
        source,
        caller_keys,
        graph_context,
    );
}

/// Whether static preflight failed because a prompt document's `::file`
/// target does not exist.
///
/// Sequences deliberately do not support an include that the document's own
/// `initialize` (or any earlier step) would create: every prompt document is
/// discovered before the first step runs (fix
/// `2026-09-15-initialize-after-proxy`, OQ1 Option A).
fn is_missing_include(error: &CompositionError) -> bool {
    use darkmatter::markdown::MarkdownError;
    use darkmatter::markdown::compose::TransclusionError;

    matches!(
        error,
        CompositionError::PreFlightDiscoveryFailed(MarkdownError::Transclusion(inner))
            if matches!(inner.as_ref(), TransclusionError::Io(io) if io.kind() == std::io::ErrorKind::NotFound)
    )
}

/// Tell the author how to satisfy a missing include at sequence preflight.
///
/// Emitted ahead of the unchanged typed error, which the top-level renderer
/// prints after this returns.
fn emit_missing_include_note(document: &std::path::Path) {
    let name = document
        .file_name()
        .map_or_else(|| document.display().to_string(), |n| n.to_string_lossy().into_owned());
    let rendered = Prose::new(format!(
        "<orange><bold>note:</bold></orange> a sequence checks every prompt document before its \
         first step runs, so a file that <b>{name}</b> includes must already exist. Neither \
         an earlier step nor the document's own <b>initialize</b> can create it. Create the \
         file before starting the sequence."
    ))
    .render(&log::terminal());
    log::message(&rendered);
}

/// Execute a full sequence: iterate steps, compose each, and report results.
#[allow(deprecated)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_sequence(
    source: &ResolvedCompositionSource,
    plan: SequencePlan,
    shared: &SharedComposeArgs,
    user_set_overrides: Option<serde_json::Value>,
    caller_input_records: darkmatter::markdown::compose::CallerInputRecords,
    execution_options: SequenceExecutionOptions,
    verbose: u8,
    perf_enabled: bool,
    startup_timings: Option<crate::perf::StartupTimings>,
    invocation: claudine::invocation_context::InvocationContext,
    source_context: claudine::invocation_context::SourceContext,
    file_resolution_context: biscuit_file::FileResolutionContext,
) -> Result<i32> {
    use claudine::composition::lifecycle::{DefaultLifecycleEmitter, LifecycleRuntimeContext, LifecycleSignal, parse_lifecycle_config};
    use claudine::composition::lifecycle::bindings::SequenceBinding;
    use claudine::composition::lifecycle::executor::StackExecutionContext;
    use claudine::composition::lifecycle::runtime::ContainerEventDecision;
    use claudine::composition::lifecycle::context::LifecycleErrorInfo;
    let runtime = Arc::new(composition::RuntimeState::new());
    let seq = SequenceBinding::container(&plan.sequence_id, Vec::new()).to_value();
    runtime.set_sequence_binding(seq.clone());
    let mut frontmatter: serde_json::Map<String, serde_json::Value> = source.markdown.frontmatter().as_map().iter().map(|(k,v)| (k.clone(),v.clone())).collect();
    if let Some(values) = user_set_overrides.as_ref().and_then(serde_json::Value::as_object) {
        frontmatter.extend(values.clone());
    }
    let lifecycle = parse_lifecycle_config(&serde_json::Value::Object(frontmatter.clone()), &source.resolved_path)?;
    let config = claudine::dispatch::loader::load_claudine_config(None, source_context.repository_root()).ok();
    let settings = config.as_ref().map(claudine::dispatch::loader::bridge_tts_settings).unwrap_or_default();
    let messaging = config.as_ref().map(claudine::dispatch::loader::bridge_messaging_settings).unwrap_or_else(|| claudine::messaging::RuntimeMessagingSettings { user: None, repo: None });
    let term = log::terminal();
    let emitter = DefaultLifecycleEmitter;
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let context = invocation.capture_launch_context(&requirements);
    let engine = darkmatter::effects::EffectEngine::builder().mutation_root(source_context.base_dir()).build();
    let live = Mutex::new(frontmatter.clone());
    let lifecycle_context = LifecycleRuntimeContext { settings: &settings, messaging: &messaging, term: &term,
        source_path: &source.resolved_path, repo_root: source_context.repository_root(), launch_area: Some(file_resolution_context.request_cwd()), context: Some(&context) };
    let stack = StackExecutionContext { seq: Some(&seq), signal: LifecycleSignal::Initialize, scope: None,
        frontmatter: &frontmatter, live_frontmatter: Some(&live), runtime_state: Some(&runtime), err: None,
        timing: None, current: Some(invocation.current_authority().memoized()), group: None, loop_values: None, base_dir: Some(source_context.base_dir()),
        ctx_base_dir: Some(file_resolution_context.request_cwd()), prepared_context: Some(&context), file_resolution_context: &file_resolution_context,
        effect_engine: &engine, shell_runner: invocation.shell_runner(), emitter: &emitter, term: &term,
        source_path: &source.resolved_path, repo_root: source_context.repository_root(), messaging: &messaging, settings: &settings };
    // A looping root allocates its loop before `initialize`, which already
    // reads the first pass's values. A dry run previews one pass, with pass
    // 1's values, and lists the root's `prep` targets without running them.
    let frame = root_loop_frame(source, &frontmatter, shared, &invocation, &file_resolution_context, &context)?;
    if shared.dry_run {
        use claudine::composition::lifecycle::LifecycleEmitter as _;
        if let (Some(frame), Some(value)) = (&frame, frontmatter.get("loop")) {
            let location = claudine::composition::LoopLocation::document_root(&source.resolved_path);
            if let Ok(block) = claudine::composition::parse_loop_block(value, claudine::composition::LoopOwner::Sequence, &location) {
                emitter.emit_info(&claudine::composition::looping::dry_run_loop_note(&location.property, &block.config, frame.max()), &term);
            }
        }
        for target in lifecycle.literal_prep_targets() {
            log::message(&super::composition::dry_run::render_prep_target(&target, &term));
        }
    }
    let prewarm = invocation.prewarm().with_injected_keys(
        composition::sequence::reserved::ROOT_OVERLAY_KEYS.iter().map(|key| (*key).to_string())
            .chain(user_set_overrides.as_ref().and_then(serde_json::Value::as_object).into_iter().flat_map(|map| map.keys().cloned())),
    );
    let mut root = container::RootRun::new(&lifecycle, &lifecycle_context, stack, Arc::clone(&runtime), frame, prewarm);
    if !shared.dry_run {
        match root.event(LifecycleSignal::Initialize, None)? {
            ContainerEventDecision::Continue => {},
            ContainerEventDecision::Skip => return Ok(0),
            ContainerEventDecision::Proxy { target, overlay } => return container::run_proxy(source, shared, &invocation, &source_context, &file_resolution_context, user_set_overrides.as_ref(), &caller_input_records, &runtime, root.proxy_location.take().expect("proxy decision carries its authored location"), target, overlay, verbose, root.approved.as_ref()),
            ContainerEventDecision::Finished { failed } => return Ok(if root.runner.is_cancelled() { 130 }
                else if let Some(code) = (!failed).then(|| root.broken_exit_code()).flatten() { code }
                else { i32::from(failed) }),
            other => return Err(color_eyre::eyre::eyre!("invalid initialize transition: {other:?}")),
        }
    }
    let mut startup_timings = startup_timings;
    loop {
        let outcome = execute_sequence_inner(source, plan.clone(), shared, user_set_overrides.clone(), caller_input_records.clone(), execution_options.clone(), verbose, perf_enabled, startup_timings.take(), invocation.clone(), source_context.clone(), file_resolution_context.clone(), &mut root);
        if shared.dry_run { return outcome; }
        let (signal, error) = match &outcome {
            Ok(130) => {
                root.runner.cancel(claudine::composition::lifecycle::cancellation::CancellationCause::Interrupt);
                (LifecycleSignal::Cancelled, root.runner.primary_error().cloned())
            },
            Ok(0) => (LifecycleSignal::Success, None),
            Ok(_) => (LifecycleSignal::Failure, root.child_error.take().or_else(|| Some(LifecycleErrorInfo::from_action_failure("sequence", "a sequence child failed")))),
            // A failure before the root reached `start` blocks it.
            Err(error) => (claudine::composition::lifecycle::policy::failure_event(darkmatter::lifecycle::run::FailureStage::Setup, root.started),
                Some(LifecycleErrorInfo::from_error_or_action("sequence", error.as_ref()))),
        };
        runtime.set_sequence_binding(seq.clone());
        if root.pending.is_none() && matches!(signal, LifecycleSignal::Success | LifecycleSignal::Failure) {
            root.record_pass(i32::from(signal == LifecycleSignal::Failure));
        }
        let decision = root.pending.take().map(Ok).unwrap_or_else(|| root.event(signal, error));
        match decision? {
            ContainerEventDecision::Retry { delay, overlay } => root.retry(delay, &overlay)?,
            ContainerEventDecision::Proxy { target, overlay } => return container::run_proxy(source, shared, &invocation, &source_context, &file_resolution_context, user_set_overrides.as_ref(), &caller_input_records, &runtime, root.proxy_location.take().expect("proxy decision carries its authored location"), target, overlay, verbose, root.approved.as_ref()),
            ContainerEventDecision::Finished { failed } => return if root.runner.is_cancelled() { Ok(130) } else if failed { outcome.and(Ok(1)) }
                else { Ok(root.broken_exit_code().unwrap_or(0)) },
            ContainerEventDecision::Skip => return Ok(0),
            other => return Err(color_eyre::eyre::eyre!("invalid terminal transition: {other:?}")),
        }
    }
}

/// The root sequence's loop frame, when its frontmatter declares `loop:`.
///
/// A templated `max` resolves once, here, against the root's values under
/// the caller's setters; `--max-iterations`, then `CLAUDINE_MAX_ITERATIONS`,
/// replaces the authored cap.
#[allow(clippy::result_large_err)]
fn root_loop_frame(
    source: &ResolvedCompositionSource,
    frontmatter: &serde_json::Map<String, serde_json::Value>,
    shared: &SharedComposeArgs,
    invocation: &claudine::invocation_context::InvocationContext,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    context: &darkmatter::markdown::compose::ComposeContext,
) -> std::result::Result<Option<claudine::composition::looping::LoopFrame>, CompositionError> {
    let Some(value) = frontmatter.get("loop") else { return Ok(None) };
    let location = claudine::composition::LoopLocation::document_root(&source.resolved_path);
    let mut config = claudine::composition::parse_loop_block(value, claudine::composition::LoopOwner::Sequence, &location)?.config;
    let ambient = claudine::composition::LoopAmbient::new(1, true, false, serde_json::json!([]), 0);
    let lookup = claudine::composition::LoopExpressionLookup::new(frontmatter, &ambient, file_resolution_context, &source.resolved_path)
        .with_prepared_context(Some(context));
    claudine::composition::resolve_templated_loop_max(&mut config, &lookup, &location)?;
    let cap = match shared.max_iterations {
        Some(cap) => Some(cap),
        None => claudine::composition::resolve_max_iterations_from_env(invocation.document_environment(), &source.resolved_path)?,
    };
    claudine::composition::looping::LoopFrame::new(
        &config, cap, claudine::composition::looping::LoopResultKind::Sequence, &source.resolved_path, &location.property,
    )
    .map(Some)
}

#[allow(clippy::too_many_arguments)]
fn execute_sequence_inner(
    source: &ResolvedCompositionSource,
    plan: SequencePlan,
    shared: &SharedComposeArgs,
    user_set_overrides: Option<serde_json::Value>,
    caller_input_records: darkmatter::markdown::compose::CallerInputRecords,
    execution_options: SequenceExecutionOptions,
    verbose: u8,
    perf_enabled: bool,
    startup_timings: Option<crate::perf::StartupTimings>,
    invocation: claudine::invocation_context::InvocationContext,
    source_context: claudine::invocation_context::SourceContext,
    file_resolution_context: biscuit_file::FileResolutionContext,
    root: &mut container::RootRun<'_>,
) -> Result<i32> {
    let silent = shared.silent;

    // Inline vs compose is decided once for the whole sequence by the
    // presence of a `prompt` frontmatter property — the same signal that
    // splits the top-level `compose` and `inline-compose` commands. When
    // `prompt` is present, every step runs as an inline composition: the
    // composed `prompt` becomes the agent prompt and the provider's output
    // replaces the document body on disk (see `prepare_inline_with_schema`
    // and the `CompositionClosurePlan::Inline` write-back). Each step reads
    // the live file at run time, so it sees the prior step's rewritten body.
    // A present-but-non-string `prompt` is rejected up front with the same
    // typed error `inline-compose` raises, rather than once per step.
    let inline_mode = match source.markdown.frontmatter().as_map().get("prompt") {
        Some(serde_json::Value::String(_)) => true,
        Some(other) => {
            return Err(CompositionError::PromptPropertyWrongType(
                crate::commands::compose::json_type_name(other).to_string(),
            )
            .into());
        }
        None => false,
    };

    let mut perf_accumulator = if perf_enabled {
        startup_timings.map(|timings| {
            let mut acc = crate::perf::SequencePerfAccumulator::new(timings);
            if shared.dry_run {
                acc.set_dry_run();
            }
            acc
        })
    } else {
        None
    };

    let effective_fail_fast = execution_options
        .fail_fast_override
        .unwrap_or(plan.document_fail_fast);

    // `--step-timeout` / `--stall-timeout` apply to every step in the
    // sequence. Early validation happens in the CLI entry point; the raw
    // string is resolved per-step by the composition executor.
    let _ = shared.step_timeout_secs()?;
    let _ = shared.stall_timeout_secs()?;

    let total_steps = plan.steps.len();

    if !silent {
        let term = log::terminal();
        let status = Status::from_prose(format!(
            "<b>Sequence:</b> <yellow>{}</yellow> step(s)",
            total_steps
        ))
        .state(StatusState::Info);
        log::message(&status.render(&term));
    }

    // Persistent interrupt tracker for the duration of the sequence run. Every
    // step boundary and every running shell task polls this flag, so it needs a
    // producer on both hosts: a SIGINT handler on Unix, and a registration with
    // the process-scoped console coordinator on Windows.
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_handler = interrupted.clone();
    let _sigint_guard = {
        #[cfg(unix)]
        {
            unsafe {
                signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                    interrupted_handler.store(true, Ordering::SeqCst);
                })
            }
            .ok()
        }
        #[cfg(not(unix))]
        {
            Some(super::exec::termination::register_sequence_interrupt_flag(
                &interrupted_handler,
            ))
        }
    };

    let shared_approval_cache: composition::SharedApprovalCache =
        Arc::new(Mutex::new(HashMap::new()));

    // The sequence document's own hints, with `--set` `agent`/`model` above
    // them: what its body steps run on, and the outermost fallback for a
    // task with no `agent` of its own (Phase 1a/1b below).
    let raw_hints =
        composition::parse_selection_hints_from_frontmatter(source.markdown.frontmatter())?;
    let raw_hints = apply_user_set_to_hints(raw_hints, user_set_overrides.as_ref())?;

    // Phase 2 (2026-05-09-slow-prep): build the per-invocation prep context
    // once. Later phases (per-step compose, harness preflight, execution
    // request) reuse the same source-repo-root, selection config, and
    // installed-provider snapshot instead of rediscovering them.
    let prep_context = super::composition::CompositionPrepContext::from_invocation(
        invocation,
        source_context,
        &shared.excluded(),
    )?;
    let snapshot = &prep_context.installed_snapshot;
    let source_repo_root = prep_context.source_repo_root.clone();

    // ── Static preflight: the recursive task graph ──────────────────────
    //
    // Everything statically knowable is settled here, before a target is
    // resolved or a provider is launched: every referenced task, group,
    // catalog entry, and prompt document is loaded transitively; blocked
    // constructs are rejected; and every reachable shell command — including
    // ones behind `when:` guards that read false today — is approved: the
    // first task's as resolved bytes when every value exists now, every other
    // command with a placeholder by its typed shape. A failure blocks the root
    // container before any child starts; `--dry-run` performs the same walk.
    // One base launch context for the whole graph, captured before per-task
    // target selection: every launch-facing graph expression (`when:` guards,
    // command interpolation, task/group variable defaults) projects the
    // caller's launch repository and package area from this snapshot, no
    // matter which repository a referenced task, group, or prompt document is
    // stored in. Per-task epochs below clone or extend this base and apply
    // their own resolved target overrides (D4).
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let graph_context = prep_context
        .invocation
        .capture_launch_context(&requirements);
    prep_context.invocation.record_prepared_context_consumer(
        claudine::invocation_context::PreparedContextConsumer::Preflight,
    );
    let graph = composition::build_preflight_graph_with_invocation(
        &plan,
        source,
        graph_context.clone(),
        &prep_context.invocation,
        &prep_context.source_context,
        user_set_overrides.as_ref(),
    )?;
    // A referenced prompt document composes only at its own turn, so its
    // single-pass lifecycle literals are checked here, before any approval
    // prompt or step can run.
    graph.validate_prompt_lifecycle_literals()?;
    submit_preflight_prewarm(
        &graph,
        source,
        user_set_overrides.as_ref(),
        source_repo_root.as_deref(),
        &prep_context.invocation,
        &graph_context,
    );
    // Root `initialize` has run, so a failed check blocks the root container.
    // A retry reuses the first attempt's closed approval.
    let entry = if root.approved.is_some() { EntryReason::Retry } else { EntryReason::Initial };
    let mut preflight = sequence_preflight().begin(entry);
    let preflight_approved = preflight
        .check(LIFECYCLE_SHELL_AUDIT, CheckStage::InRun, || {
            approve_preflight_graph(
                &graph,
                source,
                &prep_context.source_context,
                Arc::clone(&shared_approval_cache),
                shared,
                Some(prep_context.launch_workspace.launch_cwd.as_path()),
                &prep_context.invocation,
                &graph_context,
                root.approved.as_ref(),
            )
        })
        .map_err(|failure| failure.error)?;
    root.approved = Some(preflight_approved.clone());
    // A retry's walk rereads referenced sources only to refuse commands the
    // closed approval never saw; the task graph stays the first attempt's.
    let graph = match &root.graph {
        Some(first) => first.clone(),
        None => {
            root.graph = Some(graph.clone());
            graph
        }
    };
    if let Some(lifecycle) = &graph.root_lifecycle { root.runner.guard_mut().set_config(lifecycle.clone()); }
    let catalog = match prep_context.selection_config.as_ref() {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    // Selection refreshes no model catalog: prediction is read-only, and the
    // shared resolver ranks against the cached and compiled listings.
    let favorite = prep_context
        .selection_config
        .as_ref()
        .and_then(|c| c.favorite);

    let explicit_provider = shared.explicit_provider();
    let cli_model = shared.model.as_deref();
    // Agent-resolution TTY gate keys off `stderr` only — the prompting and
    // status channel — exactly like direct compose's
    // `resolve_live_target_with_tty`. A redirected stdout (`sequence doc.md >
    // out.md`) must still prompt as long as stderr is a terminal; gating on
    // stdout would wrongly abort that normal CLI pattern.
    let is_tty = std::io::stderr().is_terminal();

    // ── Phase 1a/1b: decide every task's provider ─────────────────────────
    //
    // Each provider-launching task is classified from its own `agent`:
    // predicted (resolved now; one that cannot resolve fails the run before
    // any task starts, even behind a false `when:`), decided at launch, or
    // missing a provider, which takes the nearest enclosing sequence
    // document's `agent`. Only a task still missing one gates the run: a
    // terminal reviews it, and anything else fails with
    // `AgentResolutionFailed`. A graph of only shell and side-effect tasks
    // has no site and needs no provider.
    let caller_sets_agent = user_set_overrides
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .is_some_and(|setters| setters.contains_key("agent"));
    let sites = providers::collect_sites(&graph, &plan, &source.resolved_path, &raw_hints, caller_sets_agent);
    let mut selections = preflight
        .check(AGENT_RESOLUTION, CheckStage::InRun, || -> Result<_> {
            let selections = providers::select(
                sites,
                &providers::SelectionInputs {
                    snapshot,
                    catalog: &catalog,
                    cli_model,
                    explicit_provider,
                    favorite,
                    environment: prep_context.invocation.document_environment(),
                    silent,
                    dry_run: shared.dry_run,
                },
            )?;
            if !shared.dry_run && !is_tty && let Some(unresolved) = selections.first_unresolved() {
                let providers::TaskTarget::Unresolved { state, own_hint, .. } = &unresolved.target else {
                    unreachable!("first_unresolved returns an unresolved target");
                };
                // A task naming an ambiguous list fails as that task; a task missing
                // a provider fails as the sequence document that names none.
                return Err(if *own_hint {
                    providers::launch_error(
                        unresolved,
                        claudine::composition::SequenceTaskProviderProblem::Unresolvable {
                            state: state.clone(),
                            installed: snapshot.runnable.clone(),
                        },
                    )
                } else {
                    CompositionError::AgentResolutionFailed {
                        source_path: source.resolved_path.clone(),
                        state: claudine::composition::classify_agent_resolution(&raw_hints, snapshot),
                        installed: snapshot.runnable.clone(),
                    }
                }
                .into());
            }
            Ok(selections)
        })
        .map_err(|failure| failure.error)?;
    if !shared.dry_run && review::needs_review(is_tty, &selections) {
        // The state-specific pre-prompt message (a styled `Invalid Agent:`
        // line, the zero-installed-list breakdown) shares its source of truth
        // with the dry-run table cell and the no-TTY abort body.
        let root_state = claudine::composition::classify_agent_resolution(&raw_hints, snapshot);
        if let Some(markup) = super::composition::agent_prompt_message(&root_state, &source.resolved_path) {
            log::message(&claudine::composition::agent_message_prose(markup).render(&log::terminal()));
        }
        if !review::review_selections(&mut selections, |drafts| {
            super::selection_ui::review_sequence(drafts, &catalog)
        })? {
            return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
        }
    }
    if !silent && (shared.dry_run || verbose > 0) {
        emit_provider_plan(&selections);
    }
    let resolved_targets: Vec<Option<claudine::composition::ResolvedExecutionTarget>> =
        (0..total_steps).map(|index| selections.root_target(index)).collect();

    // The resolved-provider check for every task whose provider is already
    // known, so a forwarded switch a task's provider types differently fails
    // before the first task runs. Tasks decided at launch are checked before
    // their spawn. Sequence tasks are never interactive.
    if !shared.dry_run {
        for selection in selections.iter() {
            if let Some(target) = selection.target.planned()
                && let Some(profile) = super::profile::profile_for_provider(target.provider)
            {
                super::provider_tail_report::SwitchContext::for_launch(profile, true)
                    .check(&shared.provider_tail)?;
            }
        }
    }

    // Announced *before* Phase 1c, not after it. Schema validation and shell
    // approval can be slow or interactive, so this is the progress feedback the
    // user waits behind — emitting it afterwards would describe finished work.
    if !silent {
        let term = log::terminal();
        let status = Status::from_prose("Starting pre-flight checks".to_string())
            .state(StatusState::Info)
            .theme(biscuit_terminal::components::status::StatusTheme::Circular);
        log::message(&status.render(&term));
    }

    // ── Phase 1c: sequence-wide validation and shell approval ──────────
    //
    // Every step is validated against its `$schema` and every reachable shell
    // command is approved, before any provider session launches.
    // Missing-property failures are aggregated across all steps so the user
    // fixes the full sequence in one edit; invalid-required failures abort.
    // No composition is *retained* here — execution re-composes each step at
    // its turn, and this context is what makes the two passes identical.
    let compose_ctx = StepComposeContext {
        lifecycle_group: None,
        prepared_context: None,
        document_epoch: None,
        source_repo_root: source_repo_root.as_deref(),
        child_cwd: &prep_context.launch_workspace.child_cwd,
        launch_area: Some(prep_context.launch_workspace.launch_cwd.as_path()),
        shared,
        approval_cache: Arc::clone(&shared_approval_cache),
        inline_mode,
        file_resolution_context: &file_resolution_context,
        caller_input_records: &caller_input_records,
        invocation: &prep_context.invocation,
        // Each step opens its own run at its turn.
        run_evidence: None,
        deferred_shell_roots: Default::default(),
        loop_values: None,
        validation_probe: false,
        document_shell_values: None,
    };

    let Some(validated) = run_phase_1c_with_schema(
        source,
        &plan,
        &resolved_targets,
        &user_set_overrides,
        &compose_ctx,
        effective_fail_fast,
        preflight_approved,
        &interrupted,
        silent,
    )?
    else {
        if let Some(mut acc) = perf_accumulator {
            acc.mark_env_setup_complete();
            acc.set_partial();
            acc.set_invocation_work(&prep_context.invocation.work_snapshot());
            crate::perf::emit_report(&acc.into_report());
        }
        return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
    };

    let _preflight_span = info_span!("sequence_preflight", total_steps).entered();

    if let Some(ref mut acc) = perf_accumulator {
        acc.mark_env_setup_complete();
    }

    if !silent {
        let status = Status::from_prose(format!(
            "<b>Preflight:</b> shell commands approved for all \
             <yellow>{}</yellow> step(s) in the sequence",
            total_steps
        ))
        .state(StatusState::Info);
        log::message(&status.render(&log::terminal()));
    }

    // ── Phase 2: compose and execute each step, at its turn ────────────
    drop(_preflight_span);

    let mut run_context = iterate::SequenceRunContext {
        plan: &plan,
        graph: &graph,
        resolved_targets: &resolved_targets,
        task_selections: &selections,
        selection: resolve::StepSelection {
            raw_hints: &raw_hints,
            catalog: &catalog,
            cli_model,
        },
        source,
        prep_context: &prep_context,
        shared,
        compose: &compose_ctx,
        approved: validated.approved_commands,
        user_set_overrides: validated.resolved_overrides,
        interrupted: &interrupted,
        effective_fail_fast,
        silent,
        verbose,
        perf_enabled,
        loop_values: None,
    };
    // One pass of the root runs its steps. A looping root settles each
    // successful pass's terminal event, `finalize`, and gate here and enters
    // the next; the last pass, and any pass that did not succeed, settles in
    // the caller as a root without a loop does.
    loop {
        run_context.loop_values = root.loop_values();
        if !shared.dry_run {
            root.enter_pass();
            root.started = true;
            let decision = root.event(claudine::composition::LifecycleSignal::Start, None)?;
            if !matches!(decision, claudine::composition::lifecycle::runtime::ContainerEventDecision::Continue) {
                root.pending = Some(decision);
                return Ok(1);
            }
        }
        let (summary, interrupt_observed) =
            iterate::run_sequence_steps(&run_context, &mut perf_accumulator, &root.runtime)?;

        if let Some(step) = summary.steps.iter().find(|step| !step.success) {
            let mut error = claudine::composition::lifecycle::context::LifecycleErrorInfo::from_action_failure(
                "sequence", step.error.clone().unwrap_or_else(|| "a sequence child failed".into()));
            error.snapshot = step.error_snapshot.clone();
            root.child_error = Some(error);
        }
        if let Some(acc) = perf_accumulator.as_mut() {
            acc.set_invocation_work(&prep_context.invocation.work_snapshot());
        }

        let looping = !shared.dry_run && root.frame.is_some();
        let code = report::emit_sequence_summary(
            &summary,
            if looping { None } else { perf_accumulator.take() },
            &interrupted,
            interrupt_observed,
            silent,
            shared.dry_run,
        )?;
        if !looping || code != 0 {
            emit_perf(perf_accumulator.take());
            return Ok(code);
        }
        root.runtime.set_sequence_binding(
            claudine::composition::lifecycle::bindings::SequenceBinding::container(&plan.sequence_id, Vec::new()).to_value(),
        );
        root.record_pass(0);
        let settled = root.settle_pass()?;
        if !matches!(settled, claudine::composition::lifecycle::runtime::ContainerEventDecision::Finished { failed: false })
            || root.runner.is_cancelled()
        {
            root.pending = Some(settled);
            emit_perf(perf_accumulator.take());
            return Ok(code);
        }
        match root.finish_pass() {
            Ok(claudine::composition::looping::ContainerPassEnd::Continue) => {}
            Ok(claudine::composition::looping::ContainerPassEnd::Complete | claudine::composition::looping::ContainerPassEnd::Broken(_)) => {
                root.pending = Some(claudine::composition::lifecycle::runtime::ContainerEventDecision::Finished { failed: false });
                emit_perf(perf_accumulator.take());
                return Ok(0);
            }
            // The loop owner failed after the pass settled: nothing re-enters
            // the pass's events, and the root fails with it.
            Err(error) => {
                root.pending = Some(claudine::composition::lifecycle::runtime::ContainerEventDecision::Finished { failed: true });
                emit_perf(perf_accumulator.take());
                return Err(error.into());
            }
        }
    }
}

/// List each provider-launching task with the provider it is predicted to
/// run, or how its provider is decided: always under `--dry-run`, and under
/// `--verbose` for a live run.
fn emit_provider_plan(selections: &providers::TaskSelections) {
    let rows = providers::plan_rows(selections);
    if rows.is_empty() {
        return;
    }
    let term = log::terminal();
    log::message(&Status::from_prose("<b>Providers:</b>".to_string()).state(StatusState::Info).render(&term));
    for row in rows {
        log::message(&Status::from_prose(row).state(StatusState::Info).render(&term));
    }
}

/// Emit a looping root's `--perf` report once, after its last pass.
fn emit_perf(accumulator: Option<crate::perf::SequencePerfAccumulator>) {
    if let Some(accumulator) = accumulator {
        crate::perf::emit_report(&accumulator.into_report());
    }
}

#[cfg(test)]
mod tests;
