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
    PreflightBlockedOutcome, emit_preflight_blocked_and_finalize, preflight_blocked_control_error,
};
use super::target::composition_dispatch_context;
use super::SingleCompositionOutcome;
use crate::commands::wrap::env::EnvPlan;
use crate::commands::wrap::policy::StructuredCodexOutput;
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
    pub child_cwd: &'a Path,
    pub use_structured: bool,
    pub structured_codex_output: &'a Option<StructuredCodexOutput>,
    pub stdout_noise: &'static [&'static str],
    pub stderr_noise: &'static [&'static str],
    pub show_checks: bool,
    pub stream_verbosity: Verbosity,
    pub detail_requested: bool,
    pub env_plan: &'a EnvPlan,
    pub effective_sp: &'a ResolvedSystemPrompt,
    pub verbose: u8,
}

/// Execute a single composition iteration: harness-plan parse, pre-flight
/// shell approval, dry-run seam, env/prompt output, and the harness loop.
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
    let child_cwd = ctx.child_cwd;
    let use_structured = ctx.use_structured;
    let structured_codex_output = ctx.structured_codex_output;
    let stdout_noise = ctx.stdout_noise;
    let stderr_noise = ctx.stderr_noise;
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
    // Validate that the harness plan can be parsed before proceeding.
    let plan = claudine::harness::parse_harness_plan(
        &request.prepared.effective_frontmatter,
        &request.prepared.resolved_path,
    )
    .map_err(|e| {
        // Route through the stack-aware runner so `blocked.stack` and
        // `finalize.stack` fire (spec.md:436/650/652), not just the
        // legacy top-level surface.
        let preflight_outcome = emit_preflight_blocked_and_finalize(
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
            frontmatter,
            document_start,
            claudine::composition::LifecycleErrorInfo::from_action_failure(
                "harness_plan",
                e.to_string(),
            ),
        );
        match preflight_outcome {
            PreflightBlockedOutcome::EvaluationError(ce) => ce.into(),
            PreflightBlockedOutcome::Control(control) => {
                match preflight_blocked_control_error(control, &request.prepared.resolved_path) {
                    Some(ce) => ce.into(),
                    None => color_eyre::eyre::eyre!("{e}"),
                }
            }
        }
    })?;

    // The parsed harness plan is used only for shell-command audit and
    // timeout configuration; there are no longer pre/post validation
    // checks that need an effective-plan transform.

    // ── Pre-flight shell approval for harness commands ───────────
    if !skip_preflight {
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
            let preflight_outcome = emit_preflight_blocked_and_finalize(
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
                frontmatter,
                document_start,
                claudine::composition::LifecycleErrorInfo::from_action_failure(
                    "shell_approval",
                    e.to_string(),
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
                        None => color_eyre::eyre::eyre!("{e}"),
                    }
                }
            }
        })?;

        // Emit the preflight-complete indicator for direct compose and
        // inline-compose runs. This must sit *before* the dry-run seam below:
        // dry-run returns early, so a completion message placed after it would
        // never render for dry-run — leaving the "Starting pre-flight checks"
        // spinner without its matching "complete" line. Sequence runs handle
        // their own preflight messaging in the orchestrator
        // (`wrap::sequence::execute_sequence`) and must not re-emit per step.
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

    // --dry-run seam: the full composition pipeline (compose, real shell
    // expansion, shell approval, harness pre-checks) has now run. Stop here —
    // before any provider launches — and emit the composed artifacts:
    //   - the composed body → stdout (the data product; pipeable/redirectable)
    //   - the finalized frontmatter (highlighted YAML) → stderr
    //   - a metadata table → stderr (after the frontmatter)
    // `--quiet` / `--silent` do not suppress this render: the dry-run output
    // *is* the command's purpose.
    if request.dry_run {
        // Dry-run never launches the provider or mutates the source.
        // Pre-check validation has been removed; only timeout parsing
        // and shell-command audit run during composition preflight.
        //
        // `initial_transition` is deliberately dropped here, and this is the
        // one place that is correct. Dry-run *does* fire `initialize` (its
        // `blocked`/`finalize` routing is contract, not accident), so a
        // router's `initialize` can hand off — but a dry run reports what the
        // document it was given would do, and traversing the hand-off would
        // mean composing, approving, and rendering a different document than
        // the one named on the command line. The seam owns that decision
        // rather than leaving it to a caller that forgot to look.
        let render = super::dry_run::DryRunRender::from_request(request);

        crate::log::data(&render.body);
        crate::log::message(&super::dry_run::render_hr(term));
        crate::log::message(&super::dry_run::render_frontmatter_heading(term));
        crate::log::message("");
        crate::log::message(&super::dry_run::render_frontmatter(&render.frontmatter, term));
        crate::log::message(&super::dry_run::render_metadata_table(&render, term));

        if let Some(collector) = perf_collector.as_mut() {
            collector.set_dry_run();
        }
        let outcome = SingleCompositionOutcome {
            exit_code: 0,
            provider,
            agent_perf: None,
            // Dry-run never produces a per-iteration summary.
            iteration_signals: None,
            terminal_signal: None,
            initialize_handoff: None,
        };
        // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
        // The perf report is always emitted to stderr when requested.
        if let Some(collector) = perf_collector.take() {
            crate::perf::emit_report(&collector.into_report());
        }
        return Ok(outcome);
    }

    // Plan is validated; the harness loop re-parses from the materialized
    // frontmatter, so the live path no longer needs this copy.
    drop(plan);

    // -- Preflight output (env details + prompt block) ---------------------
    // The execution header was already emitted (up front by compose /
    // inline-compose, or above for callers that did not pre-render). Now
    // emit the env details and prompt block with the full env_plan.

    // Detect the environment from the source repo root when available so
    // that git/repo metadata reflects the composition source, not the
    // caller's CWD (which may be in a different repo entirely).
    //
    // Phase 4 (2026-05-09-slow-prep): `detect_environment_fast` is still on
    // the critical path after Phases 1–2, but its direct cost is minimal
    // (~8 ms for git summary + repo structure). The `compose_prep.environment`
    // span added in Phase 3 makes this cost visible in traces. Making the
    // context truly lazy would require invasive changes to LiveSemanticSink,
    // DispatchRuntimeContext, and the wire-session path because the context is
    // consumed synchronously before the child spawns. Per the spec, when lazy
    // creation is too invasive we instrument and defer deeper work.
    let env_detect_root = effective_repo_root.unwrap_or(launch_cwd);
    let env_context = {
        let _span = tracing::info_span!("compose_prep.environment").entered();
        // Phase fix (2026-05-09-slow-prep): reuse the cached
        // `EnvironmentContext` when the prep-time sniff already covers the
        // requested env_detect_root. The cached scan was rooted at the
        // launch CWD, but sniff walks up to find the enclosing git/repo
        // root, so the resulting env_context is equivalent to one rooted
        // at `env_detect_root` whenever:
        //   1. env_detect_root == launch_cwd (trivial), OR
        //   2. launch_cwd is a subdirectory of env_detect_root AND the
        //      cached env_context's git repo_root or repo root matches
        //      env_detect_root (the common monorepo-subdir case).
        // When neither holds (e.g. `--repo` pins a different root or the
        // source lives in an unrelated repo), fall back to a fresh scan.
        let cached_matches = request.prep_env_context.as_ref().is_some_and(|prep| {
            if env_detect_root == launch_cwd.as_path() {
                return true;
            }
            if !launch_cwd.starts_with(env_detect_root) {
                return false;
            }
            let git_root_match = prep
                .git
                .as_ref()
                .map(|g| g.repo_root.as_path() == env_detect_root)
                .unwrap_or(false);
            let repo_root_match = prep
                .repo
                .as_ref()
                .map(|r| r.root.as_path() == env_detect_root)
                .unwrap_or(false);
            git_root_match || repo_root_match
        });
        if cached_matches {
            request
                .prep_env_context
                .as_ref()
                .expect("cached_matches implies Some")
                .clone()
        } else {
            claudine::events::detect_environment_fast(env_detect_root)
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
        // handed the run off: `request.prepared.prompt` is the proxying
        // *source* document's body, which never reaches the agent. The harness
        // loop emits the settled *target* document's prompt after the hand-off.
        if effective_non_interactive && !hands_off {
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

    let dispatch_context = composition_dispatch_context(request, target);

    let harness_mode = if is_inline {
        HarnessPromptMode::Inline
    } else {
        HarnessPromptMode::Compose
    };

    // The prompt state always starts at the document this request prepared —
    // the *router*, when `initialize` handed off. Repointing it is the
    // coordinator's job and its alone; the harness loop commits
    // `initial_transition` before its first attempt. Seed
    // `initial_materialized = None` on a hand-off so the loop composes the
    // adopted target rather than reusing the proxying document's prepared
    // prompt.
    let seed_materialized = (!hands_off)
        .then(|| materialized_harness_prompt_from_prepared(&request.prepared));

    let mut prompt_state = HarnessPromptState {
        mode: harness_mode,
        source_path: request.prepared.resolved_path.clone(),
        original_ref: request.file_ref.clone(),
        base_prompt: None,
        overlay: indexmap::IndexMap::new(),
        prompt_tail: Vec::new(),
        // Carried so a `retry`/`resume`/`proxy` re-materialization re-applies
        // the caller's `--set` params, launch-area file-ref anchor, and
        // pre-approved shell commands (a proxy target with a `$schema` needs
        // the same inputs the original document was prepared with).
        input_layers: request.prepared.input_layers.clone(),
        entry: request.prepared.entry,
    };

    let mut harness_base_args = args_before_prompt.clone();
    if !use_structured {
        profile.prepare_captured_output(&mut harness_base_args);
    }

    let (exit_code, harness_perf, harness_signals) = run_harness_loop(
        provider,
        profile,
        binary_path.as_path(),
        child_cwd,
        effective_non_interactive,
        request.timeout.clone(),
        request.step_timeout.clone(),
        request.stall_timeout.clone(),
        request.model.clone(),
        request.yolo,
        &harness_base_args,
        &env_plan.env,
        &mut prompt_state,
        effective_repo_root,
        shell_options.clone(),
        use_structured,
        structured_codex_output.as_ref(),
        stdout_noise,
        stderr_noise,
        profile.suppress_structured_stderr_on_success(),
        show_checks,
        stream_verbosity,
        detail_requested,
        silent,
        &env_context,
        &dispatch_context,
        seed_materialized,
        term,
        guard,
        initial_transition,
        true,
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
        // Initialize proxies are surfaced before the harness loop begins (see
        // `provider_run_handoff`); a run that reached the harness never carries one.
        initialize_handoff: None,
    };
    // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
    // The perf report is always emitted to stderr when requested.
    if let Some(collector) = perf_collector.take() {
        crate::perf::emit_report(&collector.into_report());
    }
    Ok(outcome)
}
