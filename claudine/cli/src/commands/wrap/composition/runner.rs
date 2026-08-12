//! Composition run body — the per-iteration execution closure promoted to a
//! named function.
//!
//! [`run_composition_body`] is invoked twice across a composition run's
//! lifetime: once for the external-guard (loop re-entry) path and once for the
//! owned-guard (single-run / first loop iteration) path. Both share the same
//! captured state, bundled here in [`CompositionRunCtx`] so the setup pipeline
//! hands provider execution one cohesive context.

use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::{DefaultLifecycleEmitter, LifecycleRunGuard};
use claudine::composition::{
    CompositionExecutionRequest, DocumentTransition, ResolvedExecutionTarget,
};
use claudine::events::GlobalSettings;
use claudine::harness::ShellApprovalOptions;
use claudine::messaging::RuntimeMessagingSettings;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use claudine::system_prompt::ResolvedSystemPrompt;
use color_eyre::eyre::Result;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::ComposeContext;

use super::env::LaunchWorkspaceContext;
use super::preflight::{
    PreflightBlockedOutcome, emit_preflight_blocked_and_finalize_in_context,
    preflight_blocked_control_error,
};
use super::target::composition_dispatch_context;
use super::SingleCompositionOutcome;
use crate::commands::wrap::env::EnvPlan;
use crate::commands::wrap::profile::WrapperProfile;
use crate::commands::wrap::{
    HarnessPromptMode, HarnessPromptState, materialized_harness_prompt_from_prepared,
    run_harness_loop,
};
use crate::log;
use crate::perf::CommandPerfCollector;

/// Shared, read-only state captured by the composition run body across both
/// invocation sites (external-guard loop re-entry and owned-guard single run).
///
/// Every field is either a shared reference or a `Copy` value, so constructing
/// a `CompositionRunCtx` at each call site is cheap and borrows only last for
/// the duration of [`run_composition_body`]. The mutable [`CommandPerfCollector`]
/// and the [`LifecycleRunGuard`] vary per call and stay as explicit parameters
/// rather than being bundled here.
pub(super) struct CompositionRunCtx<'a> {
    pub request: &'a CompositionExecutionRequest,
    pub target: &'a ResolvedExecutionTarget,
    pub provider: Provider,
    pub effective_repo_root: Option<&'a Path>,
    pub launch_workspace: &'a LaunchWorkspaceContext,
    pub launch_cwd: &'a PathBuf,
    pub binary_path: &'a PathBuf,
    pub lifecycle_effect_engine: &'a EffectEngine,
    pub emitter: &'a DefaultLifecycleEmitter,
    pub lifecycle_settings: &'a GlobalSettings,
    pub lifecycle_messaging: &'a RuntimeMessagingSettings,
    pub lifecycle_context: &'a ComposeContext,
    pub term: &'a Terminal,
    pub document_start: Instant,
    pub shell_options: &'a ShellApprovalOptions,
    pub silent: bool,
    pub quiet: bool,
    pub is_inline: bool,
    pub profile: &'static dyn WrapperProfile,
    pub effective_non_interactive: bool,
    pub args_before_prompt: &'a Vec<String>,
    /// R8 — the invocation-fixed half of the re-entrant launch-plan builder.
    pub launch_plan_inputs: &'a crate::commands::wrap::launch_plan::LaunchPlanInputs,
    pub child_cwd: &'a Path,
    pub use_structured: bool,
    pub show_checks: bool,
    pub stream_verbosity: Verbosity,
    pub detail_requested: bool,
    pub env_plan: &'a EnvPlan,
    pub effective_sp: &'a ResolvedSystemPrompt,
    pub verbose: u8,
}

/// Execute a single composition iteration: harness-plan parse, pre-flight
/// shell approval, env/prompt output, and the harness loop.
///
/// Live runs only — `--dry-run` returns at the seam in
/// [`super::pipeline::execute_composition_request_inner_with_guard`], before the
/// lifecycle runtime this body depends on is constructed.
///
/// This is the promoted form of the former inline `run_body` closure. `guard` and
/// `perf_collector` are passed separately because they are mutable and vary per
/// call; everything else is bundled in `ctx`.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_composition_body(
    ctx: &CompositionRunCtx<'_>,
    guard: &mut LifecycleRunGuard<'_>,
    perf_collector: &mut Option<CommandPerfCollector>,
    skip_preflight: bool,
    initial_transition: DocumentTransition,
) -> Result<SingleCompositionOutcome> {
    let request = ctx.request;
    let target = ctx.target;
    let provider = ctx.provider;
    let effective_repo_root = ctx.effective_repo_root;
    let launch_workspace = ctx.launch_workspace;
    let launch_cwd = ctx.launch_cwd;
    let binary_path = ctx.binary_path;
    let lifecycle_effect_engine = ctx.lifecycle_effect_engine;
    let emitter = ctx.emitter;
    let lifecycle_settings = ctx.lifecycle_settings;
    let lifecycle_messaging = ctx.lifecycle_messaging;
    let lifecycle_context = ctx.lifecycle_context;
    let term = ctx.term;
    let document_start = ctx.document_start;
    let shell_options = ctx.shell_options;
    let silent = ctx.silent;
    let quiet = ctx.quiet;
    let is_inline = ctx.is_inline;
    let profile = ctx.profile;
    let effective_non_interactive = ctx.effective_non_interactive;
    let args_before_prompt = ctx.args_before_prompt;
    let launch_plan_inputs = ctx.launch_plan_inputs;
    let child_cwd = ctx.child_cwd;
    let use_structured = ctx.use_structured;
    let show_checks = ctx.show_checks;
    let stream_verbosity = ctx.stream_verbosity;
    let detail_requested = ctx.detail_requested;
    let env_plan = ctx.env_plan;
    let effective_sp = ctx.effective_sp;
    let verbose = ctx.verbose;

    // Whether this request's document is handing the run off before its first
    // provider attempt. The transition itself is consumed by the harness
    // loop's coordinator; what the body needs to know is only that
    // `request.prepared` describes a document that will never reach the agent.
    let hands_off = initial_transition.hands_off_source();

    // Whether `request.prepared` is an already-committed proxy target the harness
    // loop will re-stage (narrow gate → `initialize` → stabilized reread → audit)
    // via its bootstrap. When adopting, the target's `initialize` has NOT fired
    // here (the setup pipeline skipped `route_initialize`), so this body must not
    // pre-parse the harness plan or run the pre-flight audit against the
    // bootstrap read — the staged boot does both after the target's `initialize`,
    // so an initialize-time mutation and the typed HarnessError facets are
    // observed. It also delivers the target's prompt itself (seed `None`).
    let adopting = request.adopted_handoff.is_some();

    // Whether this document's canonical preparation withheld its schema verdict
    // because the document declares an `initialize` of its own (R4). The setup
    // pipeline routes that event below; the harness loop then owes the stabilized
    // reread that sees any initialize-time mutation and reaches the verdict. An
    // adopted target is excluded — its full staged bootstrap already covers both,
    // and a document that hands off never reaches a verdict at all.
    let stabilize_after_initialize =
        request.prepared.schema_verdict_deferred && !adopting && !hands_off;

    // Composed frontmatter / source-derived base dir, reused by every
    // composition-preflight failure path so the blocked+finalize stacks
    // see the same `frontmatter` and `base_dir` namespaces the
    // post-closure `initialize` event does.
    let fm_map = request.prepared.effective_frontmatter.as_object();
    let empty_frontmatter = serde_json::Map::new();
    let frontmatter = fm_map.unwrap_or(&empty_frontmatter);
    let base_dir = request
        .prepared
        .resolved_path
        .parent()
        .or(effective_repo_root);
    // Validate that the harness plan can be parsed before proceeding. Skipped
    // for an adopted target: its `initialize` has not run yet, so parsing the
    // plan here would reject the target's (possibly initialize-mutated)
    // frontmatter before the target could initialize, and would attribute the
    // failure to a facet-less `harness_plan` action rather than the typed
    // HarnessError the staged boot's own plan parse surfaces.
    let plan = if adopting {
        None
    } else {
        Some(
            claudine::harness::parse_harness_plan(
                &request.prepared.effective_frontmatter,
                &request.prepared.resolved_path,
            )
            .map_err(|e| {
                // Route through the stack-aware runner so `blocked.stack` and
                // `finalize.stack` fire (spec.md:436/650/652), not just the
                // legacy top-level surface.
                let preflight_outcome = emit_preflight_blocked_and_finalize_in_context(
                    guard,
                    lifecycle_effect_engine,
                    emitter,
                    lifecycle_settings,
                    lifecycle_messaging,
                    term,
                    &request.prepared.resolved_path,
                    effective_repo_root,
                    base_dir,
                    Some(launch_workspace.launch_cwd.as_path()),
                    Some(lifecycle_context),
                    request.prepared.input_layers.file_resolution_context.as_ref(),
                    frontmatter,
                    document_start,
                    claudine::composition::LifecycleErrorInfo::from_error_or_action(
                        "harness_plan",
                        &e,
                    ),
                );
                match preflight_outcome {
                    PreflightBlockedOutcome::EvaluationError(ce) => ce.into(),
                    PreflightBlockedOutcome::Control(control) => {
                        match preflight_blocked_control_error(
                            control,
                            &request.prepared.resolved_path,
                        ) {
                            Some(ce) => ce.into(),
                            None => color_eyre::Report::from(e),
                        }
                    }
                }
            })?,
        )
    };

    // The parsed harness plan is used only for shell-command audit and
    // timeout configuration; there are no longer pre/post validation
    // checks that need an effective-plan transform.

    // ── Pre-flight shell approval for harness commands ───────────
    // Skipped for an adopted target: the staged boot runs the narrow
    // initialize-shell gate and then the full post-stabilization audit itself,
    // so auditing the bootstrap read here (before the target's `initialize` and
    // stabilized reread) would audit a document the run will not execute.
    if !skip_preflight && !adopting {
        let _harness_preflight = claudine::composition::resolve_shell_approvals(
            None, // template commands already approved during compose
            None,
            shell_options,
            Some(&request.prepared.lifecycle),
            Some(&request.prepared.resolved_path),
        )
        .map_err(|e| {
            // Shell-audit denial (or any other shell-approval failure)
            // is a composition-preflight blocked path: route through
            // the stack-aware runner so `blocked.stack` and
            // `finalize.stack` fire.
            let preflight_outcome = emit_preflight_blocked_and_finalize_in_context(
                guard,
                lifecycle_effect_engine,
                emitter,
                lifecycle_settings,
                lifecycle_messaging,
                term,
                &request.prepared.resolved_path,
                effective_repo_root,
                base_dir,
                Some(launch_workspace.launch_cwd.as_path()),
                Some(lifecycle_context),
                request.prepared.input_layers.file_resolution_context.as_ref(),
                frontmatter,
                document_start,
                claudine::composition::LifecycleErrorInfo::from_error_or_action(
                    "shell_approval",
                    &e,
                ),
            );
            match preflight_outcome {
                PreflightBlockedOutcome::EvaluationError(ce) => ce.into(),
                PreflightBlockedOutcome::Control(control) => {
                    match preflight_blocked_control_error(
                        control,
                        &request.prepared.resolved_path,
                    ) {
                        Some(ce) => ce.into(),
                        None => color_eyre::Report::from(e),
                    }
                }
            }
        })?;

        // Emit the preflight-complete indicator for direct compose and
        // inline-compose runs, matching the "Starting pre-flight checks"
        // spinner. Sequence runs handle their own preflight messaging in the
        // orchestrator (`wrap::sequence::execute_sequence`) and must not
        // re-emit per step.
        if !request.sequence && !silent && !quiet {
            let compose_label = if is_inline {
                "inline composition"
            } else {
                "composition"
            };
            let status = Status::from_prose(format!(
                "<b>Preflight:</b> shell commands approved for this {compose_label}"
            ))
            .state(StatusState::Info);
            log::message(&status.render(term));
        }
    }

    // No dry-run seam here. `--dry-run` returns from
    // `pipeline::execute_composition_request_inner_with_guard` before the
    // lifecycle runtime is even constructed, so this body — and every lifecycle
    // event it can reach — is live-run-only.

    // Plan is validated; the harness loop re-parses from the materialized
    // frontmatter, so the live path no longer needs this copy.
    drop(plan);

    // -- Preflight output (env details + prompt block) ---------------------
    // The execution header was already emitted (up front by compose /
    // inline-compose, or above for callers that did not pre-render). Now
    // emit the env details and prompt block with the full env_plan.

    let env_detect_root = effective_repo_root.unwrap_or(launch_cwd);
    let env_context = {
        let _span = tracing::info_span!("compose_prep.environment").entered();
        match (
            request.invocation_context.as_ref(),
            request.source_context.as_ref(),
        ) {
            (Some(invocation), Some(source)) => {
                invocation.environment_context_for_source(source)
            }
            _ => {
                if let Some(invocation) = request.invocation_context.as_ref() {
                    invocation.record_ambient_fallback();
                }
                request.prep_env_context.clone().unwrap_or_else(|| {
                    claudine::events::detect_environment_fast(env_detect_root)
                })
            }
        }
    };

    if !silent {
        if !quiet && (request.session_interactive || detail_requested) {
            crate::output::log_wrapper_env_details(env_plan, None, term, verbose);
        }

        let scope_for_report = effective_repo_root.unwrap_or(launch_cwd);
        crate::output::log_system_prompt_with_scope(
            effective_sp,
            detail_requested,
            silent,
            quiet,
            Some(scope_for_report),
            term,
        );

        if matches!(effective_sp, claudine::system_prompt::ResolvedSystemPrompt::Ready(_))
            && effective_non_interactive
        {
            crate::log::message("");
        }

        // Skip the pre-loop agent-prompt preview when an `initialize` proxy
        // handed the run off (`request.prepared.prompt` is the proxying *source*
        // body, which never reaches the agent) or when adopting a proxy target
        // (its prompt is delivered by the staged boot after its own reread). The
        // harness loop emits the settled *target* document's prompt in both cases.
        if effective_non_interactive && !hands_off && !adopting {
            crate::output::log_compose_prompt(
                &request.prepared.prompt,
                detail_requested,
                silent,
                quiet,
                term,
            );
        }

        if !quiet {
            crate::log::message("");
        }
    }

    // -- Execution --------------------------------------------------------

    // Handed to the harness loop inside the launch-rebuild intent below: each
    // attempt's rebuilt bundle either keeps it verbatim (unchanged document) or
    // recomputes its provider/model selection entries from the refreshed facets.
    let dispatch_context = composition_dispatch_context(request, target);

    let harness_mode = if is_inline {
        HarnessPromptMode::Inline
    } else {
        HarnessPromptMode::Compose
    };

    // A caller running several compositions as one logical run (the `--loop`
    // engine, sequence execution) hands its own cell down so `set` mutations
    // and `outputs` accumulate across them; a standalone run owns a fresh one.
    let runtime_state = request
        .runtime_state
        .clone()
        .unwrap_or_else(|| std::sync::Arc::new(claudine::composition::RuntimeState::new()));
    // Baseline for "did *this* execution commit an entry": a caller-supplied
    // cell may already hold prior loop iterations' or sequence steps' outputs.
    let outputs_before = runtime_state.output_count();

    // The prompt state always starts at the document this request prepared —
    // the *router*, when `initialize` handed off. Repointing it is the
    // coordinator's job and its alone; the harness loop commits
    // `initial_transition` before its first attempt. Seed
    // `initial_materialized = None` on a hand-off so the loop composes the
    // adopted target rather than reusing the proxying document's prepared
    // prompt.
    let seed_materialized = (!hands_off && !adopting)
        .then(|| {
            materialized_harness_prompt_from_prepared(
                &request.prepared,
                std::sync::Arc::clone(&runtime_state),
            )
        });

    let mut prompt_state = HarnessPromptState {
        mode: harness_mode,
        source_path: request.prepared.resolved_path.clone(),
        original_ref: request.file_ref.clone(),
        base_prompt: None,
        // The immediate proxy overlay for a proxied target (empty for a directly
        // invoked document). Re-applied on every re-materialization by
        // `materialize_harness_prompt`, so a retry/resume of a proxied target
        // keeps the pre-schema handoff input rather than losing it once the first
        // prepared composition is spent (AC26). The first attempt uses the seed
        // built from `request.prepared` (overlay already baked in during canonical
        // preparation), so the overlay is applied exactly once per attempt.
        overlay: request.proxy_overlay.clone(),
        prompt_tail: Vec::new(),
        runtime_state: std::sync::Arc::clone(&runtime_state),
        suppress_output_commit: request.suppress_output_commit,
        last_final_output: None,
        input_layers: request.prepared.input_layers.clone(),
        entry: request.prepared.entry,
        invocation_context: request.invocation_context.clone(),
        source_context: request.source_context.clone(),
        epoch_context: Some(request.prepared.compose_context.clone()),
        epoch_context_requirements: request.epoch_context_requirements.clone(),
    };

    let mut harness_base_args = args_before_prompt.clone();
    if !use_structured {
        profile.prepare_captured_output(&mut harness_base_args);
    }

    // The harness loop surfaces a terminal-recovery / start-stack proxy the
    // provider harness already committed against the shared invocation ledger
    // (`SurfacedHandoff::Committed`). It is propagated into the outcome below so
    // the command coordinator re-prepares that target through the full canonical
    // launch pipeline (R6/R7) rather than the harness adopting it in place.
    let (exit_code, harness_perf, harness_signals, surfaced_handoff) = run_harness_loop(
        provider,
        profile,
        child_cwd,
        effective_non_interactive,
        request.timeout.clone(),
        request.step_timeout.clone(),
        request.stall_timeout.clone(),
        request.model.clone(),
        // R8 — the immutable half of every per-attempt launch rebuild. The
        // caller's own decisions (explicit provider, `--yolo`, whether MCP is in
        // play) stay authoritative at every retry and resume; the document half
        // is what a canonical refresh is allowed to move.
        crate::commands::wrap::harness_orch::LaunchRebuildIntent {
            explicit_provider: request.explicit_provider,
            fallback_provider: provider,
            fallback_binary: binary_path.as_path().to_path_buf(),
            installed_snapshot: request.installed_snapshot.clone(),
            default_non_interactive: effective_non_interactive,
            cli_yolo: request.yolo,
            is_inline,
            mcp_enabled: request.mcp || !request.mcp_use.is_empty(),
            fallback_provider_reason: target.provider_reason,
            dispatch_context,
            launch_plan_inputs: launch_plan_inputs.clone(),
        },
        &env_plan.env,
        &mut prompt_state,
        effective_repo_root,
        shell_options.clone(),
        show_checks,
        stream_verbosity,
        detail_requested,
        silent,
        &env_context,
        seed_materialized,
        term,
        guard,
        initial_transition,
        request.handoff_ledger.clone(),
        request.adopted_handoff.clone(),
        stabilize_after_initialize,
        true,
        request.task_frame_writer.clone(),
    )?;
    if let (Some(collector), Some(perf)) = (perf_collector.as_mut(), harness_perf) {
        collector.set_agent_perf(perf);
    }
    let terminal_signal = guard.terminal_signal();
    let outcome = SingleCompositionOutcome {
        exit_code,
        provider,
        agent_perf: perf_collector
            .as_ref()
            .and_then(|c| c.agent_perf())
            .or(harness_perf),
        // The harness loop now surfaces the terminal attempt's iteration
        // signals, so `compose --loop` receives the same rate-limit /
        // exit_reason pickup for every composition document.
        iteration_signals: harness_signals,
        terminal_signal,
        // The harness loop records the captured text on the success path only.
        // `outputs_before` still gates it so a suppressed *and* a committed run
        // report identically, and a skipped or failed run reports `None`.
        final_output: prompt_state.last_final_output.clone().or_else(|| {
            (runtime_state.output_count() > outputs_before)
                .then(|| runtime_state.last_output_text())
                .flatten()
        }),
        // A run that reached the harness carries a handoff only when a terminal
        // recovery / start-stack proxy fired mid-run: the harness committed it
        // against the shared invocation ledger and surfaces it here for the
        // coordinator to re-prepare. `initialize`-route proxies are surfaced
        // earlier (see `provider_run_handoff`) and never reach this path.
        initialize_handoff: surfaced_handoff,
    };
    // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
    // The perf report is always emitted to stderr when requested.
    if let (Some(collector), Some(invocation)) = (
        perf_collector.as_mut(),
        request.invocation_context.as_ref(),
    ) {
        collector.set_invocation_work(&invocation.work_snapshot());
    }
    if let Some(collector) = perf_collector.take() {
        crate::perf::emit_report(&collector.into_report());
    }
    Ok(outcome)
}
