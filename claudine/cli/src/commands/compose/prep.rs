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
    LIFECYCLE_EVENT_KEYS, LifecycleRuntimeContext,
    LoopExecutionOptions, LoopExecutionResult,
    PrepareOptions, PreparedComposition, ProxyHandoff, ResolvedCompositionSource,
    ResolvedExecutionTarget, RunLedger, SharedApprovalCache, SharedRunLedger, SurfacedHandoff,
    SystemShellRunner, build_loop_seed_with_lifecycle, commit_proxy_in_context,
    resolve_loop_config,
};
use claudine::diagnostics::DiagnosticSnapshot;
use claudine::invocation_context::{InvocationContext, SourceContext};
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::{Result, WrapErr, eyre};
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
    CompositionPrepContext, eagerly_resolve_target, execute_composition_request_inner,
    execute_composition_attempt, install_agent_env_for_composition,
};
use crate::commands::wrap::overlay::merge_frontmatter_overlay;
use crate::commands::wrap::wrap_terminal;
use crate::output::emit_execution_header;

/// Whether this document owes its schema verdict to the stabilized reread taken
/// after its own `initialize` runs.
///
/// R4 puts `initialize` before schema validation precisely so a document can add
/// or repair a schema property from its own bootstrap. Reaching the verdict
/// first would fail the document for a violation the next stage is about to fix.
/// A document that declares no `initialize` has nothing to wait for and is
/// judged where it is read.
///
/// Keyed on the *authored* key, not on the parsed lifecycle: a malformed or
/// empty `initialize:` must still defer, because the pipeline routes it either
/// way and something downstream owes the verdict.
fn defers_schema_verdict_to_initialize(source: &ResolvedCompositionSource) -> bool {
    source
        .markdown
        .frontmatter()
        .as_map()
        .contains_key("initialize")
}

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

/// What running one active document produced.
///
/// The composition command owns an active-document coordinator above both the
/// document-loop engine and the provider harness (R1). Running one document
/// either finishes the whole command or hands the run off to a target at
/// `initialize`; loop recognition then reruns for the *target*, so a proxied
/// target acquires the same document loop it would receive when invoked
/// directly (R7).
pub(crate) enum ActiveDocumentOutcome {
    /// The active document (and any loop it owned) ran to completion.
    Done(i32),
    /// A proxy handed the run off to a target. The coordinator re-prepares the
    /// target through the canonical launch pipeline and reruns loop-vs-single on
    /// it.
    ///
    /// A [`SurfacedHandoff::Request`] arrived from an `initialize` route awaiting
    /// commit by this command's ledger; a [`SurfacedHandoff::Committed`] arrived
    /// from a terminal-recovery / target-initialize-chain route the provider
    /// harness already committed against the shared invocation ledger.
    Handoff(SurfacedHandoff),
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
    let invocation = InvocationContext::capture()?;
    let provisional_context = invocation.launch_file_resolution_context().clone();
    let source = resolve_composition_source(
        &file,
        kind,
        &shared,
        &provisional_context,
    )?;
    // Derive the definitive source bundle from the same owner so a
    // top-level document selected from a different repository keeps that
    // repository's nested references (D2/D10, AC12). The launch projection is
    // used only for resolving the top-level argument; downstream surfaces
    // receive this source bundle.
    let mut source_context = invocation.derive_source(&source.resolved_path)?;
    let mut file_resolution_context = source_context.file_resolution_context().clone();
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
    // Use the owner's frozen launch CWD as the `file`-typed property fallback
    // anchor so caller-supplied area-relative
    // paths resolve here exactly as they will at prepare time and event time,
    // rather than depending on the soon-to-be-mutated process CWD.
    let launch_area_fallback = Some(invocation.launch_cwd().to_path_buf());
    // A document whose own `initialize` may supply or repair a schema property
    // is not judged here: R4 puts `initialize` first, and the post-`initialize`
    // stabilized reread reaches the verdict instead. Interactive collection goes
    // with it — prompting the caller for a value the document is about to write
    // itself would be a question with a wrong answer.
    let defer_verdict = defers_schema_verdict_to_initialize(&source);
    let (source, set_overrides) = if defer_verdict {
        (source, set_overrides)
    } else {
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

    // The invocation-wide approval cache and the run ledger outlive any single
    // document (R2). An already-approved command must not re-prompt after a
    // handoff, and hop/cycle accounting bounds the whole chain of documents, so
    // both are created once, before the active-document loop. The ledger is
    // seeded with the caller's document so a self-proxy is a cycle from the
    // first hop.
    let shared_approval_cache: SharedApprovalCache = Arc::new(Mutex::new(HashMap::new()));
    // One invocation-wide ledger, shared with the provider harness (R1/R2). The
    // command loop owns it; the harness receives a clone of the `Arc` so a
    // terminal-event proxy commits against this same chain rather than a
    // harness-local copy. Both the command-level `initialize`-route commit below
    // and the harness's terminal-route commit reach the one hop/cycle accounting.
    let ledger: SharedRunLedger = Arc::new(Mutex::new(RunLedger::new(
        source.resolved_path.clone(),
        Arc::clone(&shared_approval_cache),
    )));

    // The active document. `compose`/`inline-compose` begin at the caller's
    // document; an `initialize` proxy replaces it with the target and loop
    // recognition reruns for that target, so a proxied target acquires the same
    // document loop it would receive when invoked directly (R7). Immutable
    // caller `--set` overrides survive every handoff as the highest-precedence
    // input layer.
    let mut source = source;
    let mut current_file = file;
    let mut inline_state = inline_state;
    let mut startup_timings = startup_timings;
    let mut first = true;
    // The immediate proxy overlay for the active document. Empty for the caller's
    // document; set to the committed handoff's overlay after a proxy so the
    // adopted target carries it through every re-materialization (retry/resume),
    // matching the pre-schema handoff input exactly as canonical preparation left
    // it (AC26). Replaced — never merged — on each handoff: forwarding is
    // explicit, so a downstream hop that omits `with:` starts from empty.
    let mut current_overlay: indexmap::IndexMap<String, serde_json::Value> =
        indexmap::IndexMap::new();
    // The already-committed handoff the *next* active document adopts for its
    // staged bootstrap. `None` for the caller's document; set to each committed
    // handoff below so the target re-runs its own `initialize`/reread/audit (R4)
    // rather than having the setup pipeline route `initialize` a second time.
    let mut adopted_handoff: Option<Box<ProxyHandoff>> = None;

    loop {
        let active_file_resolution_context = source_context.file_resolution_context().clone();
        let (outcome, _commit_repo_root) = prepare_and_run_active_document(
            &shared,
            kind,
            source,
            &current_file,
            &current_overlay,
            adopted_handoff.take(),
            first,
            &mut inline_state,
            &system_prompt_args,
            set_overrides.clone(),
            &launch_area_fallback,
            &shared_approval_cache,
            &ledger,
            verbose,
            startup_timings.take(),
            std::mem::take(&mut prep_substages),
            compose_entry,
            invocation.clone(),
            source_context,
        )?;

        match outcome {
            ActiveDocumentOutcome::Done(code) => return Ok(code),
            ActiveDocumentOutcome::Handoff(surfaced) => {
                // Resolve the surfaced handoff to a committed one against the one
                // invocation ledger. An `initialize`-route request is committed
                // here (the source's stacks already fired); a terminal-route
                // handoff was already committed by the harness against this same
                // shared ledger while the source's stacks were still live to
                // catch a refusal, so it is used directly — the target is never
                // resolved or hop-checked twice.
                let handoff: ProxyHandoff = match surfaced {
                    SurfacedHandoff::Request(request) => {
                        commit_proxy_in_context(
                            &mut ledger.lock().unwrap(),
                            request,
                            &active_file_resolution_context,
                        )
                            .map_err(color_eyre::eyre::Report::from)?
                    }
                    SurfacedHandoff::Committed(handoff) => *handoff,
                };
                let target_ref = handoff.resolved_target().to_string_lossy().into_owned();
                let mut target_source = resolve_composition_source(
                    &target_ref,
                    kind,
                    &shared,
                    &file_resolution_context,
                )?;
                // Precedence, low→high: target-authored frontmatter < the proxy's
                // `with:` overlay < caller `--set`. The overlay lands in the
                // authored map here, before composition, so it participates in the
                // target's interpolation and schema stages; the caller's
                // `set_overrides` ride on the compose options and are applied on
                // top, so a router cannot silently outrank an explicit caller value.
                merge_frontmatter_overlay(
                    target_source.markdown.frontmatter_mut().as_map_mut(),
                    handoff.overlay(),
                );
                source = target_source;
                source_context = invocation.derive_source(&source.resolved_path)?;
                file_resolution_context = source_context.file_resolution_context().clone();
                current_file = handoff.authored_target().to_string();
                // Carry the committed overlay onto the target's request so it is
                // re-applied on every re-materialization of the target
                // (retry/resume), not just baked once into the first prepared
                // composition. Replaced, not merged: an omitted downstream `with:`
                // starts the next target from empty (forwarding is explicit).
                current_overlay = handoff.overlay().clone();
                // The target adopts this committed handoff for its staged
                // bootstrap on the next iteration.
                adopted_handoff = Some(Box::new(handoff));
                first = false;
            }
        }
    }
}

/// Prepare one active document through the canonical launch pipeline and run
/// loop-vs-single on it, returning the outcome plus this document's repository
/// root (used to resolve any surfaced `@repo/…` proxy target).
///
/// This is the single canonical active-document preparation step (R6/R7): it
/// rebuilds the full launch bundle (prep context, eager target/provider/model,
/// env, header, shell preflight) for whatever `source` it is handed and then
/// runs `execute_loop_or_single`, so a proxied target acquires exactly the same
/// launch state and document loop it would receive when invoked directly.
///
/// Both the `compose`/`inline-compose` active-document coordinator
/// ([`run_composition_inner`]) and the sequence step's contained coordinator
/// ([`crate::commands::wrap::sequence`]) drive their own thin loop around this
/// helper, committing each surfaced handoff against their own ledger — there is
/// one canonical preparation, not a per-command reimplementation.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub(crate) fn prepare_and_run_active_document(
    shared: &SharedComposeArgs,
    kind: CompositionKind,
    source: ResolvedCompositionSource,
    current_file: &str,
    current_overlay: &indexmap::IndexMap<String, serde_json::Value>,
    adopted_handoff: Option<Box<ProxyHandoff>>,
    first: bool,
    inline_state: &mut Option<crate::commands::compose::InlinePromptState>,
    system_prompt_args: &SystemPromptArgs,
    set_overrides: Option<serde_json::Value>,
    launch_area_fallback: &Option<std::path::PathBuf>,
    shared_approval_cache: &SharedApprovalCache,
    ledger: &SharedRunLedger,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
    mut prep_substages: Vec<crate::perf::SubstageTiming>,
    compose_entry: std::time::Instant,
    invocation: InvocationContext,
    source_context: SourceContext,
) -> Result<(ActiveDocumentOutcome, Option<std::path::PathBuf>)> {
    let perf_enabled = shared.perf;
    // Captured for frontmatter-excerpt enrichment of any error rendered below;
    // gates whether the YAML block is shown (TTY or FORCE_COLOR) or withheld.
    let stderr_is_tty =
        std::io::stderr().is_terminal() || std::env::var_os("FORCE_COLOR").is_some();

    let file_resolution_context = source_context.file_resolution_context().clone();

    // Running a document switches the process CWD to that document's child
    // CWD (the repo root). Restore the launch area before preparing the next
    // active document so a proxied target's context is anchored exactly as a
    // direct invocation from the launch area would anchor it — `ctx.area`,
    // file-reference fallbacks, and the launch workspace must not inherit the
    // previous document's mutated CWD (R5). A no-op on the first iteration.
    if let Some(anchor) = launch_area_fallback {
        let _ = std::env::set_current_dir(anchor);
    }

    // Announce the flow-control redirect for a proxied target (never the caller's
    // own first document, which was not reached through a `proxy`). A handoff
    // surfaced to this command coordinator no longer passes through the harness's
    // in-harness `report_proxy_handoff`, so the announcement is emitted here
    // instead — the operator still sees *why* the running prompt changed before
    // the target's own header and prompt render, identically to the legacy
    // harness-local adoption path. Covers both `compose`/`inline-compose` handoffs
    // and each sequence step's contained proxy (R1).
    if !first && !shared.silent {
        claudine::harness::report::report_proxy_handoff(&source.resolved_path, &wrap_terminal());
    }

    // Who owns this document's schema verdict. A proxied target always defers:
    // the harness's staged bootstrap reads it once before its `initialize` and
    // once after, and only the second read judges it. The caller's own document
    // defers when it declares an `initialize` of its own — the setup pipeline
    // routes that event, and the harness then stabilizes and judges.
    let schema_stage = if first && !defers_schema_verdict_to_initialize(&source) {
        claudine::composition::SchemaStage::Validate
    } else {
        claudine::composition::SchemaStage::DeferToStabilizedReread
    };

    // Build per-document provider-selection state from the invocation owner
    // and definitive source bundle without another repository observation.
    let prep_ctx_t = std::time::Instant::now();
    let prep_context = CompositionPrepContext::from_invocation(
        invocation.clone(),
        source_context,
        &shared.excluded(),
    )?;
    record_prep_substage(&mut prep_substages, perf_enabled, "prep context", prep_ctx_t);
    // This document's own repository root resolves its `@repo/…` proxy
    // targets; captured before `execute_loop_or_single` consumes the context.
    let commit_repo_root = prep_context.source_repo_root.clone();

    // -- Eager target resolution -------------------------------------------
    // Resolve the execution target *before* composing templates so that
    // `{{env.AGENT}}` in the body resolves to the chosen provider's slug.
    // Hints come from the raw frontmatter (no compose). For a proxied target
    // this reads the *target's* hints, so provider/model launch state is
    // rebuilt per document (R6) under the caller's explicit-CLI precedence.
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
            current_file,
            prep_context.launch_workspace.package_context.clone(),
            &crate::log::terminal(),
        ),
        _ => false,
    };

    // Inline-compose's deferred body reporting is a property of the caller's
    // document. A proxied target owns the eventual inline closure but does
    // not re-run the router's deferred report, so this fires only for the
    // first (caller) document.
    if first {
        kind.on_header_emitted(&source, shared, current_file, inline_state.take())?;
    }

    // ── The document-epoch snapshot ──────────────────────────────────────
    //
    // One target-adjusted early-binding `ctx.*`/`env.*` snapshot per document
    // preparation epoch: captured once, after provider/model resolution and
    // before shell preflight, anchored on the invocation's immutable launch
    // context (the area the caller launched from) and populated from that
    // invocation's retained launch repository/topology evidence. Every stage
    // of this epoch — narrow/full shell preflight, body and effective
    // frontmatter composition, schema evaluation, loop seed/condition work,
    // and lifecycle execution — receives this exact snapshot, so moving the
    // prompt file cannot change launch-facing `ctx.*` values and no stage can
    // observe a second, separately constructed capture. The resolved target's
    // identity overrides are applied exactly once, here.
    //
    // `current.ctx.*` stays live event-time state and is captured separately
    // downstream. The active `SourceContext` (not this snapshot) remains
    // authoritative for file resolution, transclusion, and `$schema`.
    let prepared_context = {
        let requirements = darkmatter::markdown::compose::ContextRequirements::for_document(
            &source.markdown,
        );
        let mut ctx = invocation.capture_launch_context(&requirements);
        for (key, value) in &env_overrides {
            ctx.env_mut().insert(key.clone(), value.clone());
        }
        ctx
    };

    // Preflight must see the same resolved environment as the main prepare
    // pass, including frontmatter values derived from `env.AGENT`.
    // ── Pre-flight shell approval ────────────────────────────────────────
    //
    // Composes against the epoch snapshot above, so the commands this audit
    // discovers are the commands the main prepare expands: same snapshot, same
    // `AGENT`, same target identity. The snapshot is launch-anchored, never
    // process-CWD-anchored — the wrapper mutates the parent CWD to the repo
    // root, so an ambient capture would answer `ctx.area` differently
    // depending on when it ran.
    let compose_options = {
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(
            prepared_context.clone(),
        )
        .with_source_file(&source.resolved_path)
        .with_file_resolution_context(file_resolution_context.clone())
        // Lifecycle subtrees remain deferred because their file references
        // may target artifacts created before the event fires. Their shell
        // commands are audited separately by `collect_lifecycle_shell_commands`.
        .with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied())
        // Retain the captured launch directory as diagnostic metadata;
        // document-authored references use the request-scoped resolver.
        .with_file_ref_fallback_dir(prep_context.launch_workspace.launch_cwd.clone())
        // Discovery, not judgment: this pass exists to find `::shell`
        // directives. The verdict belongs to canonical preparation — and for
        // a document with an `initialize`, to the reread taken after it — so
        // reporting one here would put the schema ahead of `initialize`
        // again (R4) and would render Darkmatter's raw error instead of the
        // typed one the direct route renders (AC28).
        .with_deferred_schema_verdict(true);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let approval_options = crate::commands::wrap::apply_composition_shell_overrides(
        crate::commands::wrap::build_harness_shell_options_for_source_with_cache(
            &source.resolved_path,
            prep_context.source_context.repository_root(),
            Some(Arc::clone(shared_approval_cache)),
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
        return Ok((
            ActiveDocumentOutcome::Done(USER_INTERRUPT_EXIT_CODE),
            commit_repo_root,
        ));
    }

    let outcome = execute_loop_or_single(
        shared,
        current_file.to_string(),
        source,
        schema_stage,
        prep_context,
        resolved_target,
        env_overrides,
        header_emitted,
        preflight,
        Arc::clone(shared_approval_cache),
        Arc::clone(ledger),
        system_prompt_args,
        set_overrides,
        kind,
        verbose,
        startup_timings,
        prep_substages,
        compose_entry,
        current_overlay.clone(),
        adopted_handoff,
        file_resolution_context,
        prepared_context,
    )?;

    Ok((outcome, commit_repo_root))
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
            .wrap_err("invalid --timeout value")?;
    }
    Ok(())
}

/// Resolve the composition source, with inline-specific error reporting
/// for reference-resolution failures and an ENTER-path autocomplete fallback
/// when the file reference is not found.
pub(crate) fn resolve_composition_source(
    file: &str,
    kind: CompositionKind,
    shared: &SharedComposeArgs,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedCompositionSource> {
    let stderr_is_tty = std::io::stderr().is_terminal()
        || std::env::var_os("FORCE_COLOR").is_some();
    match claudine::composition::resolve_composition_source_in_context(file, file_resolution_context) {
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
            claudine::composition::resolve_composition_source_in_context(
                &selected,
                file_resolution_context,
            ).map_err(|e| {
                claudine::composition::enrich_composition_source_load_error_in_context(
                    &selected,
                    e,
                    stderr_is_tty,
                    file_resolution_context,
                )
                .into()
            })
        }
        Err(e) => {
            let e = claudine::composition::enrich_composition_source_load_error_in_context(
                file,
                e,
                stderr_is_tty,
                file_resolution_context,
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
/// Iteration 1 prepares at `schema_stage`; later iterations skip schema
/// validation and shell approval; the loop engine emits `initialize` once and
/// delegates `start`/terminal/`finalize` to the shared
/// [`claudine::composition::LifecycleRunGuard`].
#[allow(clippy::too_many_arguments)]
fn build_and_run_loop(
    source: &ResolvedCompositionSource,
    loop_prepare_options: &PrepareOptions,
    loop_options: LoopExecutionOptions,
    // Whether iteration 1 owns this document's schema verdict, or owes it to the
    // post-`initialize` stabilized reread. Threaded from the coordinator so a
    // looping document is judged at the same lifecycle point as a single one.
    schema_stage: claudine::composition::SchemaStage,
    // Whether this document arrived through a committed proxy handoff, which is
    // the entry reason iteration 1 records on its prepared composition.
    is_proxy_target: bool,
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
    proxy_overlay: &indexmap::IndexMap<String, serde_json::Value>,
    handoff_ledger: &SharedRunLedger,
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
    // One cell for the whole `--loop` run: a `set` written in iteration 1 and
    // every committed `outputs` entry stay visible to later iterations.
    let runtime_state = std::sync::Arc::new(claudine::composition::RuntimeState::new());

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
        loop_prepare_options.file_resolution_context.as_ref(),
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
                // Iteration 1 prepares at the stage the coordinator selected;
                // re-entry passes skip schema validation and shell pre-flight
                // because the seed/frontmatter state was already judged on
                // iteration 1.
                //
                // At the deferred stage iteration 1 is still a pre-`initialize`
                // read of the document *body* — the engine emitted `initialize`
                // just above, but this compose runs against the source snapshot
                // taken before it. Recording the deferral on the prepared
                // composition is what hands the verdict to the harness's
                // stabilized reread (`stabilize_after_initialize` in
                // `wrap::composition::runner`), which re-reads from disk, re-runs
                // the full audit, and judges the document `initialize` left
                // behind. Reaching the verdict here instead would put the schema
                // ahead of `initialize` for every looping document (R4).
                let mut iteration_options = loop_prepare_options.clone();
                iteration_options.set_overrides = Some(
                    claudine::composition::layered_set_overrides(
                        Some(&ctx.as_set_overrides()),
                        Some(&runtime_state.snapshot()),
                        None,
                    ),
                );
                if ctx.iteration == 1 {
                    kind.prepare_staged(
                        source,
                        iteration_options,
                        if is_proxy_target {
                            claudine::composition::DocumentEntryReason::ProxyTarget
                        } else {
                            claudine::composition::DocumentEntryReason::Direct
                        },
                        schema_stage,
                    )?
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
                std::sync::Arc::clone(&runtime_state),
                proxy_overlay,
                Arc::clone(handoff_ledger),
                // A looping proxy target runs its `initialize` through the loop
                // engine, not the staged bootstrap, so it is never adopted here.
                None,
            );

            let outcome = execute_composition_attempt(
                request,
                verbose,
                shared.perf,
                guard,
                ctx.iteration > 1,
            )
            .map_err(|e| {
                // A concrete `CompositionError` keeps its own identity. Since
                // iteration 1 may defer its verdict, the attempt itself now
                // carries preparation failures the closure used to raise
                // directly — a schema verdict from the staged boot's stabilized
                // reread above all. Flattening those into `LoopIterationFailed`
                // would make a looping document render a different diagnostic
                // than the identical document without `loop:`.
                //
                // Everything else is pre-spawn execution wiring (binary lookup,
                // env build). Those are runtime problems, not malformed loop
                // frontmatter, and stay an iteration failure.
                match e.downcast::<CompositionError>() {
                    Ok(typed) => typed,
                    Err(other) => CompositionError::LoopIterationFailed {
                        iteration: ctx.iteration,
                        prompt_path: source.resolved_path.clone(),
                        exit_code: 1,
                        reason: other.to_string(),
                        exit_reason: None,
                        // `Report` boxes its source rather than discarding it,
                        // so preserve a typed wiring diagnostic when one is
                        // still reachable at this boundary.
                        snapshot: DiagnosticSnapshot::select(other.as_ref()).map(Box::new),
                    },
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
    runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
    proxy_overlay: &indexmap::IndexMap<String, serde_json::Value>,
    handoff_ledger: SharedRunLedger,
    adopted_handoff: Option<Box<ProxyHandoff>>,
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
        invocation_context: Some(prep_context.invocation.clone()),
        source_context: Some(prep_context.source_context.clone()),
        prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
        prep_launch_context: Some(prep_context.launch_context.clone()),
        prep_env_context: Some(prep_context.env_context.clone()),
        prep_launch_detection_error: prep_context.launch_detection_error.clone(),
        header_emitted,
        provider_args: shared.provider_args.clone(),
        provider_args_explicit: shared.provider_args_explicit,
        runtime_state: Some(runtime_state),
        suppress_output_commit: false,
        // Not a sequence task: nothing owns a bar for this run.
        task_frame_writer: None,
        // The immediate proxy overlay for a proxied target (empty for a
        // directly-invoked document), re-applied on every re-materialization so a
        // retry/resume keeps the pre-schema handoff input (AC26).
        proxy_overlay: proxy_overlay.clone(),
        // The one invocation-wide ledger, threaded from the command coordinator so
        // a terminal-event proxy commits against the same hop/cycle chain and
        // surfaces back up rather than being adopted in-harness (R6/R7).
        handoff_ledger: Some(handoff_ledger),
        // The already-committed handoff a proxied target adopts for its staged
        // bootstrap (R4), or `None` for the caller's own document.
        adopted_handoff,
    }
}

/// Run the loop engine when the document declares `loop:` frontmatter,
/// otherwise fall through to the single execution path.
///
/// Returns [`ActiveDocumentOutcome::Handoff`] when the document's `initialize`
/// proxies before its first provider attempt (whether the document owned a
/// `loop:` or not), so the caller's active-document coordinator can commit the
/// target and rerun loop-vs-single on it. Both the loop route (the engine's
/// initialize transition) and the single route (the pipeline's surfaced
/// `initialize_handoff`) converge on that one outcome.
///
/// `--dry-run` bypasses the iteration engine entirely: a single
/// composition + single render (Decision 4). Loop detection is skipped so
/// a doc with `loop:` frontmatter renders once rather than iterating.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::result_large_err)]
fn execute_loop_or_single(
    shared: &SharedComposeArgs,
    file: String,
    source: ResolvedCompositionSource,
    // Whether this preparation owns the document's schema verdict, or defers it
    // to the post-`initialize` stabilized reread.
    schema_stage: claudine::composition::SchemaStage,
    prep_context: CompositionPrepContext,
    resolved_target: Option<ResolvedExecutionTarget>,
    env_overrides: BTreeMap<String, String>,
    header_emitted: bool,
    preflight: claudine::composition::PreFlightResult,
    shared_approval_cache: SharedApprovalCache,
    handoff_ledger: SharedRunLedger,
    system_prompt_args: &SystemPromptArgs,
    set_overrides: Option<serde_json::Value>,
    kind: CompositionKind,
    verbose: u8,
    mut startup_timings: Option<crate::perf::StartupTimings>,
    prep_substages: Vec<crate::perf::SubstageTiming>,
    compose_entry: std::time::Instant,
    // The immediate proxy overlay for this active document (empty for the
    // caller's document), re-applied on every re-materialization of a proxied
    // target so a retry/resume keeps the pre-schema handoff input (AC26).
    proxy_overlay: indexmap::IndexMap<String, serde_json::Value>,
    // The already-committed handoff a proxied target adopts for its staged
    // bootstrap on the single (non-loop) path, or `None` for the caller's
    // document. A looping target routes its `initialize` through the loop engine
    // instead, so this is consumed only by the single-execution branch below.
    adopted_handoff: Option<Box<ProxyHandoff>>,
    file_resolution_context: biscuit_file::FileResolutionContext,
    // The document-epoch snapshot: the one target-adjusted launch capture the
    // coordinator built after provider/model resolution, already carrying this
    // target's identity overrides. Every preparation below composes against
    // this exact snapshot.
    prepared_context: darkmatter::markdown::compose::ComposeContext,
) -> Result<ActiveDocumentOutcome> {
    let loop_options = build_loop_options(shared);

    // Captured once for frontmatter-excerpt enrichment of any error rendered
    // below; gates whether the YAML block is shown (TTY or FORCE_COLOR) or
    // withheld (pipe/CI/NO_COLOR).
    let stderr_is_tty = std::io::stderr().is_terminal()
        || std::env::var_os("FORCE_COLOR").is_some();

    let file_for_loop = file.clone();

    // The early-binding snapshot is the coordinator's epoch capture, received
    // as a parameter: preflight, body compose, effective frontmatter, loop
    // iterations, and lifecycle events all reuse this one exact instance, so
    // `ctx.*`/`env.*` cannot diverge between stages and there is no per-event
    // re-scan. The harness's post-`initialize` stabilized reread extends this
    // same snapshot (never re-anchoring) when the rewritten document demands
    // context groups it was not captured with. `current.*` stays event-time and
    // is captured separately.

    let loop_prepare_options = PrepareOptions {
        invocation_context: Some(prep_context.invocation.clone()),
        set_overrides: set_overrides.clone(),
        pre_approved_commands: Some(preflight.approved_commands.clone()),
        env_overrides: env_overrides.clone(),
        perf_enabled: shared.perf,
        source_repo_root: prep_context.source_repo_root.clone(),
        shell_working_directory: Some(prep_context.launch_workspace.child_cwd.clone()),
        prepared_context: Some(prepared_context.clone()),
        file_ref_fallback_dir: Some(prep_context.launch_workspace.launch_cwd.clone()),
        file_resolution_context: Some(file_resolution_context.clone()),
        // Name coercion is a sequence-only concern; standalone compose/loop
        // prep injects no `state` object, so there is nothing to coerce.
        name_coercion_keys: Vec::new(),
        allow_empty_body: false,
        // R4 applies to a looping document exactly as it does to a single one.
        // Every read the loop route takes *before* the engine emits `initialize`
        // — the seed compose here and the iteration-1 compose below — must
        // withhold the verdict when the coordinator selected the deferred stage,
        // or a document whose own `initialize` supplies a required property is
        // rejected before the event that would have supplied it. Hard-coding
        // `false` here is what made a looping target fail where the identical
        // document invoked without `loop:` succeeded.
        defer_schema_verdict: schema_stage
            == claudine::composition::SchemaStage::DeferToStabilizedReread,
    };

    if !shared.dry_run {
        let loop_result = build_and_run_loop(
            &source,
            &loop_prepare_options,
            loop_options,
            schema_stage,
            adopted_handoff.is_some(),
            kind,
            &file_for_loop,
            resolved_target.clone(),
            system_prompt_args,
            &env_overrides,
            &shared_approval_cache,
            header_emitted,
            &prep_context,
            &prepared_context,
            verbose,
            shared,
            &proxy_overlay,
            &handoff_ledger,
        )?;
        if let Some(loop_result) = loop_result {
            if let Some(error) = loop_result.error {
                // The interrupt path already announced itself via the INFO
                // status line; suppress the red `Error:` echo and exit with
                // the conventional 130 code so the shell sees a clean Ctrl+C.
                if matches!(error, CompositionError::LoopInterrupted { .. }) {
                    return Ok(ActiveDocumentOutcome::Done(loop_result.final_exit_code));
                }
                // Rate-limit halt has its own conventional exit code
                // (`EX_TEMPFAIL` = 75) so shell wrappers can recognize a
                // transient halt and retry-after-cool-off. Render the styled
                // error block inline, then return `Ok(75)` so the outer
                // process exits with the right code.
                if matches!(error, CompositionError::LoopRateLimited { .. }) {
                    emit_rate_limit_halt(&error);
                    return Ok(ActiveDocumentOutcome::Done(
                        claudine::composition::LOOP_RATE_LIMITED_EXIT_CODE,
                    ));
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
            // The engine's surfaced handoff is consumed here, never dropped
            // (R1). A loop document whose `initialize`, an iteration's terminal
            // stack, or the loop gate proxies ends the source and hands the run
            // to the active-document coordinator, which reruns loop-vs-single for
            // the target (R7). It is not an extra iteration of the source loop.
            if let Some(handoff) = loop_result.handoff {
                return Ok(ActiveDocumentOutcome::Handoff(handoff));
            }
            return Ok(ActiveDocumentOutcome::Done(loop_result.final_exit_code));
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
        kind.prepare_staged(
            &source,
            PrepareOptions {
                invocation_context: Some(prep_context.invocation.clone()),
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
                file_resolution_context: Some(file_resolution_context.clone()),
                name_coercion_keys: Vec::new(),
                allow_empty_body: false,
                defer_schema_verdict: false,
            },
            if adopted_handoff.is_some() {
                claudine::composition::DocumentEntryReason::ProxyTarget
            } else {
                claudine::composition::DocumentEntryReason::Direct
            },
            schema_stage,
        )
        .map_err(|e| e.enrich_frontmatter(&source, stderr_is_tty))?
    };
    emit_dropped_optional_warnings(&prepared.dropped_optionals);
    emit_compose_warnings(&prepared.warnings, shared.silent);

    let request = build_execution_request(
        shared,
        file,
        prepared,
        resolved_target,
        system_prompt_args,
        &env_overrides,
        Some(shared_approval_cache),
        header_emitted,
        kind.mode(),
        &prep_context,
        std::sync::Arc::new(claudine::composition::RuntimeState::new()),
        &proxy_overlay,
        handoff_ledger,
        adopted_handoff,
    );

    if let Some(ref mut timings) = startup_timings {
        timings.prep_phase = compose_entry.elapsed();
        timings.prep_substages = prep_substages;
    }

    let outcome =
        execute_composition_request_inner(request, verbose, startup_timings, shared.perf)?;
    // The single route surfaces a proxy the same way the loop route surfaces its
    // handoff: the pipeline (for an `initialize` route) or the provider harness
    // (for a terminal-recovery / target-initialize-chain route) hoists it here
    // rather than adopting it in place, so the coordinator re-prepares the target
    // through the canonical launch pipeline and reruns loop-vs-single (R6/R7).
    match outcome.initialize_handoff {
        Some(handoff) => Ok(ActiveDocumentOutcome::Handoff(handoff)),
        None => Ok(ActiveDocumentOutcome::Done(outcome.exit_code)),
    }
}

#[cfg(test)]
mod tests;
