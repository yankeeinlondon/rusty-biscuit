//! Shared preparation and loop/single-execution scaffolding for the
//! `compose` and `inline-compose` commands.
//!
//! Both entrypoints flow through [`run_composition_inner`]. The small set
//! of command-specific differences (composition mode, prepare function,
//! inline-specific pre-validation and deferred reporting) are dispatched
//! through [`CompositionKind`].

// `CompositionError` carries variants with several `PathBuf` and other
// owned fields (e.g. `LoopIterationFailed`, `LoopRateLimited`) so the
// enum-on-the-stack is sizable. Boxing the inner data would ripple
// through every existing call site for marginal benefit; the closures
// in this file legitimately propagate the typed error and that's the
// shape they need to keep.
#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, HashMap};
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};

use claudine::composition::{
    CompositionError, CompositionExecutionRequest, CompositionMode, DefaultLifecycleEmitter,
    LIFECYCLE_EVENT_KEYS, LifecycleRuntimeContext, LoopExecutionOptions, LoopExecutionResult,
    PrepareOptions, PreparedComposition, ResolvedCompositionSource, ResolvedExecutionTarget,
    SharedApprovalCache, SystemShellRunner, build_loop_seed_with_lifecycle, resolve_loop_config,
};
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::{Result, eyre};
use tracing::info_span;

use super::interrupt::{USER_INTERRUPT_EXIT_CODE, install_user_interrupt_guard};
use super::loop_run::{
    build_loop_iteration_output, emit_compose_warnings, emit_rate_limit_halt,
    record_prep_substage, run_loop_with_overrides,
};
use super::setters::{merge_set_overrides, parse_composition_positionals};
use super::{CompositionKind, SharedComposeArgs};
use crate::commands::schema_interactive::{
    emit_dropped_optional_warnings, pre_validate_with_interactive_collection,
    resolve_interactive_options,
};
use crate::commands::wrap::composition::{
    CompositionPrepContext, eagerly_resolve_target, execute_composition_request,
    execute_composition_attempt, install_agent_env_for_composition,
};
use crate::commands::wrap::wrap_terminal;
use crate::output::emit_execution_header;

/// Enrich a `color_eyre::Report` with a frontmatter excerpt when its root
/// cause is a frontmatter-rooted [`CompositionError`].
///
/// Used at boundaries that have already mapped the typed error into a `Report`
/// (e.g. eager target resolution); typed-`CompositionError` boundaries call
/// [`CompositionError::enrich_frontmatter`] directly instead.
fn enrich_report(
    err: color_eyre::Report,
    source: &ResolvedCompositionSource,
    stderr_is_tty: bool,
) -> color_eyre::Report {
    match err.downcast::<CompositionError>() {
        Ok(typed) => typed.enrich_frontmatter(source, stderr_is_tty).into(),
        Err(other) => other,
    }
}

/// Shared implementation for `compose` and `inline-compose`.
///
/// `kind` selects the command-specific prepare function, execution mode,
/// and inline-specific validation/reporting hooks. All other behavior
/// (positional parsing, timeout validation, source resolution, schema
/// pre-validation, prep context, eager target resolution, header emission,
/// shell preflight, loop detection, and single execution) is shared.
#[allow(clippy::too_many_lines)]
pub(crate) fn run_composition_inner(
    shared: SharedComposeArgs,
    args: Vec<String>,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
    kind: CompositionKind,
) -> Result<i32> {
    let compose_entry = std::time::Instant::now();
    let perf_enabled = shared.perf;
    let mut prep_substages: Vec<crate::perf::SubstageTiming> = Vec::new();
    let parsed = parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;

    // Install the process-scoped SIGINT handler now so Ctrl+C during the
    // (potentially slow) compose prep phase produces the same INFO notice
    // and exit-130 outcome the loop emits during its iterations.
    let _user_interrupt_guard = install_user_interrupt_guard(&file);

    validate_timeout_flags(&shared)?;
    shared.step_timeout_secs()?;
    shared.stall_timeout_secs()?;
    let set_overrides = merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?;
    let system_prompt_args = shared.system_prompt_args();

    let frontmatter_load_t = std::time::Instant::now();
    let source = resolve_composition_source(&file, kind, &shared)?;
    record_prep_substage(
        &mut prep_substages,
        perf_enabled,
        "frontmatter load",
        frontmatter_load_t,
    );

    // For inline-compose, create the tracing span immediately after source
    // resolution to match the original command timing. For compose, the span
    // is created after schema pre-validation below.
    let mut _span_guard: Option<tracing::span::EnteredSpan> = None;
    if kind.is_inline() {
        _span_guard = Some(
            info_span!(
                "inline_compose",
                file = %source.resolved_path.display(),
                provider = ?shared.explicit_provider(),
                interactive = shared.interactive,
            )
            .entered(),
        );
    }

    let (source, inline_state) = kind.on_source_resolved(source, &shared)?;

    // Captured once for frontmatter-excerpt enrichment of any error rendered
    // below; gates whether the YAML block is shown (TTY or FORCE_COLOR) or
    // withheld (pipe/CI/NO_COLOR).
    let stderr_is_tty = std::io::stderr().is_terminal()
        || std::env::var_os("FORCE_COLOR").is_some();

    // Schema-aware pre-prepare validation. Runs BEFORE the preflight
    // compose pass so the user-visible error surface is Claudine's
    // typed `CompositionError` rather than Darkmatter's raw
    // `MarkdownError::SchemaValidationFailed`. Drives interactive
    // collection of missing required values when allowed and merges the
    // collected values into `set_overrides`. Invalid optional values are
    // dropped in place. Schema-load failures, invalid required values,
    // and missing required values (non-interactive) surface as typed
    // errors here, never as Darkmatter raw errors.
    let schema_t = std::time::Instant::now();
    // Pre-validation runs BEFORE the wrapper's `switch_process_cwd`, so the
    // ambient CWD is still the launch area. Capture it explicitly as the
    // `file`-typed property fallback anchor so caller-supplied area-relative
    // paths resolve here exactly as they will at prepare time and event time,
    // rather than depending on the soon-to-be-mutated process CWD.
    let launch_area_fallback = std::env::current_dir().ok();
    let (source, set_overrides) = {
        let interactive_opts = resolve_interactive_options(shared.silent);
        let term = crate::log::terminal();
        let pre = pre_validate_with_interactive_collection(
            &source,
            set_overrides.as_ref(),
            interactive_opts,
            &term,
            launch_area_fallback.as_deref(),
        )
        .map_err(|e| e.enrich_frontmatter(&source, stderr_is_tty))?;
        emit_dropped_optional_warnings(&pre.dropped_optionals);
        (pre.source, pre.set_overrides)
    };
    // Includes any interactive collection wait when stdin is a TTY; in the
    // common non-interactive / dry-run `--perf` case this is pure validation.
    record_prep_substage(
        &mut prep_substages,
        perf_enabled,
        "schema validation",
        schema_t,
    );

    if !kind.is_inline() {
        _span_guard = Some(
            info_span!(
                "compose",
                file = %source.resolved_path.display(),
                provider = ?shared.explicit_provider(),
                interactive = shared.interactive,
            )
            .entered(),
        );
    }

    // Phase 2 (2026-05-09-slow-prep): build the per-invocation prep context
    // immediately after source resolution. The context owns the single
    // source-repo-root discovery, the loaded selection config, and the
    // installed-provider snapshot used by every later prep phase.
    let prep_ctx_t = std::time::Instant::now();
    let prep_context =
        CompositionPrepContext::new(&file, &source.resolved_path, &shared.excluded())?;
    record_prep_substage(&mut prep_substages, perf_enabled, "prep context", prep_ctx_t);

    // -- Eager target resolution -------------------------------------------
    // Resolve the execution target *before* composing templates so that
    // `{{env.AGENT}}` in the body resolves to the chosen provider's slug.
    // Hints come from the raw frontmatter (no compose).
    let raw_hints =
        claudine::composition::parse_selection_hints_from_frontmatter(source.markdown.frontmatter())?;
    let resolved_target = {
        let _span = info_span!("compose_prep.eager_target").entered();
        eagerly_resolve_target(
            &prep_context,
            &raw_hints,
            shared.explicit_provider(),
            shared.model.as_deref(),
            shared.dry_run,
            &source.resolved_path,
        )
        .map_err(|e| enrich_report(e, &source, stderr_is_tty))?
    };

    let mut env_overrides: BTreeMap<String, String> = BTreeMap::new();
    if let Some(ref target) = resolved_target {
        install_agent_env_for_composition(target, shared.yolo, &mut env_overrides);
    }

    // Render the execution line the moment the agent is known. Eager
    // resolution above prompts the interactive picker when the agent is
    // ambiguous, so by this point the target is settled for every real
    // run — and this lands *before* the expensive prepare/compose work, so
    // the user sees immediate feedback. The lone case without a target is
    // a `--dry-run` with an unresolved agent; the executor emits the
    // header itself there after rendering the unresolved state.
    //
    // The header must describe the *resolved* session mode, not the raw
    // `-i` flag: a document with `interactive: true` and no flag still runs
    // interactively. `raw_hints.interactive` carries the authored
    // frontmatter value, so resolving it here keeps the eager header in
    // agreement with the executor's `session_interactive`.
    let header_interactive = shared.resolve_session_interactivity(raw_hints.interactive).value;
    let header_emitted = match (shared.silent, resolved_target.as_ref()) {
        (false, Some(target)) => emit_execution_header(
            target.provider,
            shared.yolo,
            header_interactive,
            verbose > 0,
            shared.repo,
            kind.is_inline(),
            false, // sequence
            shared.operation.as_deref(),
            &file,
            prep_context.launch_workspace.package_context.clone(),
            &crate::log::terminal(),
        ),
        _ => false,
    };

    kind.on_header_emitted(&source, &shared, &file, inline_state)?;

    // ── Pre-flight shell approval ────────────────────────────────────────
    //
    // `ComposeContext::capture()` + `env_mut().insert(...)` installs the
    // resolved `AGENT` (and any other composition env overrides) into
    // the same Darkmatter compose pipeline that the preflight pass
    // walks. Without this, frontmatter values that derive from env
    // (e.g. `runtime_agent: '{{ env.AGENT }}'`) fail Darkmatter's
    // built-in schema validation during preflight, before reaching
    // `prepare_direct_with_schema`.
    let compose_options = {
        let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
        for (key, value) in &env_overrides {
            ctx.env_mut().insert(key.clone(), value.clone());
        }
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx)
            .with_source_file(&source.resolved_path)
            // Defer the lifecycle event keys (DM1), exactly as the main prepare
            // passes do. The preflight runs a full compose pipeline purely to
            // discover template `::shell` directives; without the exclusion it
            // also resolves the deferred lifecycle subtree at compose time, so a
            // `success`/`failure` event's read-side file reference (e.g.
            // `frontmatter(plan, …)` over a plan this very run is about to
            // create) trips the now-fatal file-ref check before the event that
            // would make the file exist ever fires. Lifecycle shell commands are
            // audited separately via `collect_lifecycle_shell_commands`.
            .with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied())
            // Anchor `file`-typed schema validation on the launch area so a
            // caller-supplied area-relative path resolves here document-first
            // then launch-area — exactly as the main prepare pass and the
            // lifecycle events do. Without it the preflight's built-in schema
            // validation would fall back to the (already-mutated) process CWD.
            .with_file_ref_fallback_dir(prep_context.launch_workspace.launch_cwd.clone());
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let shared_approval_cache: SharedApprovalCache = Arc::new(Mutex::new(HashMap::new()));

    let approval_options = crate::commands::wrap::apply_composition_shell_overrides(
        crate::commands::wrap::build_harness_shell_options_with_cache(
            &source.resolved_path,
            prep_context.source_repo_root.as_deref(),
            Some(Arc::clone(&shared_approval_cache)),
        ),
        shared.dry_run,
        shared.yolo,
    );

    let shell_approval_t = std::time::Instant::now();
    let preflight = {
        let _span = info_span!("compose_prep.shell_preflight").entered();
        claudine::composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            &approval_options,
            None,
            None,
        )?
    };
    record_prep_substage(
        &mut prep_substages,
        perf_enabled,
        "shell approval",
        shell_approval_t,
    );

    // Post-prep interrupt checkpoint. If the user pressed Ctrl+C during
    // any of the prep phases (compose source parse, target resolution,
    // env install, shell preflight) the SIGINT handler has already
    // emitted the INFO notice — short-circuit before launching the agent
    // or entering the loop.
    if crate::output::user_interrupt_observed() {
        return Ok(USER_INTERRUPT_EXIT_CODE);
    }

    execute_loop_or_single(
        shared,
        file,
        source,
        prep_context,
        resolved_target,
        env_overrides,
        header_emitted,
        preflight,
        shared_approval_cache,
        system_prompt_args,
        set_overrides,
        kind,
        verbose,
        startup_timings,
        prep_substages,
        compose_entry,
    )
}

/// Validate timeout flags against interactive mode and parse their values.
///
/// Early validation gives fast syntax feedback before any expensive prep
/// work begins. The authoritative interactive/timeout conflict check also
/// runs inside the executor against the *resolved* session mode.
fn validate_timeout_flags(shared: &SharedComposeArgs) -> Result<()> {
    if shared.timeout.is_some() && shared.interactive {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }
    if shared.step_timeout.is_some() && shared.interactive {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    if let Some(ref raw) = shared.timeout {
        claudine::harness::parse_timeout(raw, std::path::Path::new("<--timeout>"))
            .map_err(|e| eyre!("invalid --timeout value: {e}"))?;
    }
    Ok(())
}

/// Resolve the composition source, with inline-specific error reporting
/// for reference-resolution failures and an ENTER-path autocomplete fallback
/// when the file reference is not found.
fn resolve_composition_source(
    file: &str,
    kind: CompositionKind,
    shared: &SharedComposeArgs,
) -> Result<ResolvedCompositionSource> {
    let stderr_is_tty = std::io::stderr().is_terminal()
        || std::env::var_os("FORCE_COLOR").is_some();
    match claudine::composition::resolve_composition_source(file) {
        Ok(source) => Ok(source),
        Err(CompositionError::FileNotFound(_)) => {
            let mode = match kind {
                CompositionKind::Direct => crate::completion::scopes::ComposeMode::Compose,
                CompositionKind::Inline => {
                    crate::completion::scopes::ComposeMode::InlineCompose
                }
            };
            let selected =
                crate::completion::operation_file::autocomplete_operation_file(file, mode)?;
            claudine::composition::resolve_composition_source(&selected).map_err(|e| {
                claudine::composition::enrich_composition_source_load_error(
                    &selected,
                    e,
                    stderr_is_tty,
                )
                .into()
            })
        }
        Err(e) => {
            let e = claudine::composition::enrich_composition_source_load_error(
                file,
                e,
                stderr_is_tty,
            );
            if kind.is_inline() {
                // Only a genuine reference-resolution failure means the file
                // was not found. A file that resolved but failed to load/parse
                // (e.g. malformed frontmatter) must not be reported as
                // "no match" — its own typed error already names the file and
                // the cause.
                if !shared.silent && matches!(e, CompositionError::InvalidReference { .. }) {
                    let term = crate::log::terminal();
                    claudine::harness::report::report_source_file(
                        file,
                        std::path::Path::new(""),
                        &term,
                    );
                }
            }
            Err(e.into())
        }
    }
}

/// Build the loop execution options from CLI flags and environment.
fn build_loop_options(shared: &SharedComposeArgs) -> LoopExecutionOptions {
    LoopExecutionOptions {
        max_iterations: shared
            .max_iterations
            .or(claudine::composition::resolve_max_iterations_from_env()),
        fail_fast: claudine::composition::resolve_fail_fast_from_env(),
        on_rate_limit: shared.on_rate_limit.map(Into::into),
        // Engine polls this during rate-limit pause sleeps so Ctrl+C
        // surfaces as `LoopInterrupted` instead of waiting out the timer.
        interrupt_check: Some(crate::output::user_interrupt_observed),
        pause_reset_margin: claudine::composition::resolve_pause_reset_margin_from_env(),
    }
}

/// Build the loop seed, lifecycle runtime context, and run the loop engine.
///
/// Iteration 1 uses full schema-aware prepare + shell pre-flight. Later
/// iterations skip schema validation and shell approval; the loop engine
/// emits `initialize` once and delegates `start`/terminal/`finalize` to the
/// shared [`claudine::composition::LifecycleRunGuard`].
#[allow(clippy::too_many_arguments)]
fn build_and_run_loop(
    source: &ResolvedCompositionSource,
    loop_prepare_options: &PrepareOptions,
    loop_options: LoopExecutionOptions,
    kind: CompositionKind,
    file_for_loop: &str,
    resolved_target: Option<ResolvedExecutionTarget>,
    system_prompt_args: &SystemPromptArgs,
    env_overrides: &BTreeMap<String, String>,
    shared_approval_cache: &SharedApprovalCache,
    header_emitted: bool,
    prep_context: &CompositionPrepContext,
    prepared_context: &darkmatter::markdown::compose::ComposeContext,
    verbose: u8,
    shared: &SharedComposeArgs,
) -> std::result::Result<Option<LoopExecutionResult>, CompositionError> {
    let config = resolve_loop_config(source)?;
    let Some(config) = config else {
        return Ok(None);
    };

    // Build the seed AND parse lifecycle from the document's full composed
    // frontmatter. The seed lifts only iteration-control variables, so parsing
    // lifecycle from it would drop every event block and leave the loop with no
    // `initialize`/`start`/terminal/`finalize` or `loop:` gate concerns. The
    // non-loop path parses lifecycle from `prepared.lifecycle`; this matches it.
    let loop_seed = build_loop_seed_with_lifecycle(
        source,
        &config,
        loop_prepare_options.clone(),
        kind.mode(),
    )?;
    let initial_frontmatter = loop_seed.seed;
    let lifecycle_config = loop_seed.lifecycle;

    let effective_repo_root = prep_context.source_repo_root.as_deref();
    let (lifecycle_settings, lifecycle_messaging) = if lifecycle_config.is_empty() {
        (
            claudine::events::GlobalSettings::default(),
            claudine::messaging::RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
        )
    } else {
        match claudine::dispatch::loader::load_claudine_config(None, effective_repo_root) {
            Ok(config) => (
                claudine::dispatch::loader::bridge_tts_settings(&config),
                claudine::dispatch::loader::bridge_messaging_settings(&config),
            ),
            Err(_) => (
                claudine::events::GlobalSettings::default(),
                claudine::messaging::RuntimeMessagingSettings {
                    user: None,
                    repo: None,
                },
            ),
        }
    };

    let term = wrap_terminal();
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &lifecycle_settings,
        messaging: &lifecycle_messaging,
        term: &term,
        source_path: &source.resolved_path,
        repo_root: effective_repo_root,
        launch_area: Some(prep_context.launch_workspace.launch_cwd.as_path()),
        context: Some(prepared_context),
    };

    let lifecycle_mutation_root = effective_repo_root
        .unwrap_or(prep_context.launch_workspace.child_cwd.as_path());
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(lifecycle_mutation_root)
        .auto_rehash(false)
        .build();
    let shell_runner = SystemShellRunner;
    let emitter = DefaultLifecycleEmitter;

    run_loop_with_overrides(
        source,
        &config,
        initial_frontmatter,
        loop_options,
        &lifecycle_config,
        &lifecycle_ctx,
        &effect_engine,
        &shell_runner,
        &emitter,
        |ctx, guard| {
            let prepared = {
                let _span = match kind {
                    CompositionKind::Direct => {
                        info_span!("compose_prep.prepare_direct").entered()
                    }
                    CompositionKind::Inline => {
                        info_span!("compose_prep.prepare_inline").entered()
                    }
                };
                // Iteration 1 validates against `$schema`; re-entry passes
                // skip schema validation and shell pre-flight because the
                // seed/frontmatter state was already judged on iteration 1.
                let mut iteration_options = loop_prepare_options.clone();
                iteration_options.set_overrides = Some(ctx.as_set_overrides());
                if ctx.iteration == 1 {
                    kind.prepare_with_schema(source, iteration_options)?
                } else {
                    kind.prepare_without_schema(source, iteration_options)?
                }
            };

            emit_compose_warnings(&prepared.warnings, shared.silent);

            // Repoint the guard at THIS iteration's freshly composed lifecycle.
            // The guard was built from the seed (iteration-1) lifecycle, whose
            // `{{ }}` interpolations are frozen at the seed frontmatter. Each
            // iteration re-composes with the live frontmatter, so its
            // `prepared.lifecycle` carries the current iteration's values
            // (e.g. a `loop:` gate `'gate:{{phase}}'` resolves to the live
            // `phase`). The loop gate fires through this same guard after the
            // executor returns, so swapping here keeps both the per-iteration
            // terminal events and the post-finalize gate on live state.
            guard.set_config(prepared.lifecycle.clone());

            let request = build_execution_request(
                shared,
                file_for_loop.to_string(),
                prepared,
                resolved_target.clone(),
                system_prompt_args,
                env_overrides,
                Some(Arc::clone(shared_approval_cache)),
                header_emitted,
                kind.mode(),
                prep_context,
            );

            let outcome = execute_composition_attempt(
                request,
                verbose,
                shared.perf,
                guard,
                ctx.iteration > 1,
            )
            .map_err(|e| {
                // Pre-spawn execution wiring failed (binary lookup, env
                // build, etc.). Surface as an iteration failure — these are
                // runtime problems, not malformed loop frontmatter.
                CompositionError::LoopIterationFailed {
                    iteration: ctx.iteration,
                    prompt_path: source.resolved_path.clone(),
                    exit_code: 1,
                    reason: e.to_string(),
                    exit_reason: None,
                }
            })?;

            Ok(build_loop_iteration_output(
                ctx.iteration,
                &source.resolved_path,
                outcome,
            ))
        },
    )
}

/// Build a [`CompositionExecutionRequest`] from the prepared state.
///
/// Used for both loop iterations (where expensive fields are cloned) and
/// the single-run path (where fields are moved). Taking references keeps
/// the helper usable from both ownership contexts.
#[allow(clippy::too_many_arguments)]
fn build_execution_request(
    shared: &SharedComposeArgs,
    file_ref: String,
    prepared: PreparedComposition,
    resolved_target: Option<ResolvedExecutionTarget>,
    system_prompt_args: &SystemPromptArgs,
    env_overrides: &BTreeMap<String, String>,
    shared_approval_cache: Option<SharedApprovalCache>,
    header_emitted: bool,
    mode: CompositionMode,
    prep_context: &CompositionPrepContext,
) -> CompositionExecutionRequest {
    let resolved = shared.resolve_session_interactivity(prepared.selection_hints.interactive);
    CompositionExecutionRequest {
        mode,
        file_ref,
        prepared,
        resolved_target,
        explicit_provider: shared.explicit_provider(),
        excluded: shared.excluded(),
        yolo: shared.yolo,
        include: shared.include.clone(),
        model: shared.model.clone(),
        output: shared.output,
        system_prompt_args: system_prompt_args.clone(),
        timeout: shared.timeout.clone(),
        step_timeout: shared.step_timeout.clone(),
        stall_timeout: shared.stall_timeout.clone(),
        operation: shared.operation.clone(),
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
        env_overrides: env_overrides.clone(),
        shared_approval_cache,
        sequence: false,
        installed_snapshot: Some(prep_context.installed_snapshot.clone()),
        prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
        prep_launch_context: Some(prep_context.launch_context.clone()),
        prep_env_context: Some(prep_context.env_context.clone()),
        prep_launch_detection_error: prep_context.launch_detection_error.clone(),
        header_emitted,
        provider_args: shared.provider_args.clone(),
        provider_args_explicit: shared.provider_args_explicit,
    }
}

/// Run the loop engine when the document declares `loop:` frontmatter,
/// otherwise fall through to the single execution path.
///
/// `--dry-run` bypasses the iteration engine entirely: a single
/// composition + single render (Decision 4). Loop detection is skipped so
/// a doc with `loop:` frontmatter renders once rather than iterating.
#[allow(clippy::too_many_arguments)]
fn execute_loop_or_single(
    shared: SharedComposeArgs,
    file: String,
    source: ResolvedCompositionSource,
    prep_context: CompositionPrepContext,
    resolved_target: Option<ResolvedExecutionTarget>,
    env_overrides: BTreeMap<String, String>,
    header_emitted: bool,
    preflight: claudine::composition::PreFlightResult,
    shared_approval_cache: SharedApprovalCache,
    system_prompt_args: SystemPromptArgs,
    set_overrides: Option<serde_json::Value>,
    kind: CompositionKind,
    verbose: u8,
    mut startup_timings: Option<crate::perf::StartupTimings>,
    prep_substages: Vec<crate::perf::SubstageTiming>,
    compose_entry: std::time::Instant,
) -> Result<i32> {
    let loop_options = build_loop_options(&shared);

    // Captured once for frontmatter-excerpt enrichment of any error rendered
    // below; gates whether the YAML block is shown (TTY or FORCE_COLOR) or
    // withheld (pipe/CI/NO_COLOR).
    let stderr_is_tty = std::io::stderr().is_terminal()
        || std::env::var_os("FORCE_COLOR").is_some();

    let file_for_loop = file.clone();

    // Capture the early-binding context ONCE, against the launch area (the
    // package area the caller launched from), and reuse the same snapshot for
    // body compose, shell preflight, and lifecycle events. This is the single
    // source of truth for `ctx.*`/`env.*`: the body and the lifecycle can no
    // longer diverge, and there is no per-event re-scan. `capture_for_document`
    // is demand-driven over both frontmatter and body, so the lifecycle
    // `{{ctx.*}}` strings in frontmatter pull in the groups they need.
    // `current.*` stays event-time and is captured separately.
    let prepared_context = {
        let mut ctx = darkmatter::markdown::compose::ComposeContext::capture_for_document(
            prep_context.launch_workspace.launch_cwd.as_path(),
            &source.markdown,
        );
        for (key, value) in &env_overrides {
            ctx.env_mut().insert(key.clone(), value.clone());
        }
        ctx
    };

    let loop_prepare_options = PrepareOptions {
        set_overrides: set_overrides.clone(),
        pre_approved_commands: Some(preflight.approved_commands.clone()),
        env_overrides: env_overrides.clone(),
        perf_enabled: shared.perf,
        source_repo_root: prep_context.source_repo_root.clone(),
        shell_working_directory: Some(prep_context.launch_workspace.child_cwd.clone()),
        prepared_context: Some(prepared_context.clone()),
        file_ref_fallback_dir: Some(prep_context.launch_workspace.launch_cwd.clone()),
    };

    if !shared.dry_run {
        let loop_result = build_and_run_loop(
            &source,
            &loop_prepare_options,
            loop_options,
            kind,
            &file_for_loop,
            resolved_target.clone(),
            &system_prompt_args,
            &env_overrides,
            &shared_approval_cache,
            header_emitted,
            &prep_context,
            &prepared_context,
            verbose,
            &shared,
        )?;
        if let Some(loop_result) = loop_result {
            if let Some(error) = loop_result.error {
                // The interrupt path already announced itself via the INFO
                // status line; suppress the red `Error:` echo and exit with
                // the conventional 130 code so the shell sees a clean Ctrl+C.
                if matches!(error, CompositionError::LoopInterrupted { .. }) {
                    return Ok(loop_result.final_exit_code);
                }
                // Rate-limit halt has its own conventional exit code
                // (`EX_TEMPFAIL` = 75) so shell wrappers can recognize a
                // transient halt and retry-after-cool-off. Render the styled
                // error block inline, then return `Ok(75)` so the outer
                // process exits with the right code.
                if matches!(error, CompositionError::LoopRateLimited { .. }) {
                    emit_rate_limit_halt(&error);
                    return Ok(claudine::composition::LOOP_RATE_LIMITED_EXIT_CODE);
                }
                // A late-binding evaluation error surfaced by the loop engine
                // (`initialize` / `loop` gate) has no stderr renderer in the
                // lib; emit its styled block here, at the consumption boundary,
                // before returning (no further lifecycle events fire). Marks it
                // already-emitted so the outer renderer does not double-emit.
                let error = crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                    error,
                    &wrap_terminal(),
                );
                return Err(error.enrich_frontmatter(&source, stderr_is_tty).into());
            }
            return Ok(loop_result.final_exit_code);
        }
    }

    // ── Single execution path (no loop) ──────────────────────────────────
    // Pre-validation (with interactive collection) has already run, so
    // schema requirements are satisfied. Call the schema-aware prepare
    // directly — typed errors still surface for any residual issues
    // (e.g. drop-and-retry on a latent optional).
    let prepared = {
        let _span = match kind {
            CompositionKind::Direct => info_span!("compose_prep.prepare_direct").entered(),
            CompositionKind::Inline => info_span!("compose_prep.prepare_inline").entered(),
        };
        kind.prepare_with_schema(
            &source,
            PrepareOptions {
                set_overrides,
                pre_approved_commands: Some(preflight.approved_commands),
                env_overrides: env_overrides.clone(),
                perf_enabled: shared.perf,
                source_repo_root: prep_context.source_repo_root.clone(),
                shell_working_directory: Some(
                    prep_context.launch_workspace.child_cwd.clone(),
                ),
                prepared_context: Some(prepared_context.clone()),
                file_ref_fallback_dir: Some(prep_context.launch_workspace.launch_cwd.clone()),
            },
        )
        .map_err(|e| e.enrich_frontmatter(&source, stderr_is_tty))?
    };
    emit_dropped_optional_warnings(&prepared.dropped_optionals);
    emit_compose_warnings(&prepared.warnings, shared.silent);

    let request = build_execution_request(
        &shared,
        file,
        prepared,
        resolved_target,
        &system_prompt_args,
        &env_overrides,
        Some(shared_approval_cache),
        header_emitted,
        kind.mode(),
        &prep_context,
    );

    if let Some(ref mut timings) = startup_timings {
        timings.prep_phase = compose_entry.elapsed();
        timings.prep_substages = prep_substages;
    }

    execute_composition_request(request, verbose, startup_timings, shared.perf)
}

#[cfg(test)]
mod tests;
