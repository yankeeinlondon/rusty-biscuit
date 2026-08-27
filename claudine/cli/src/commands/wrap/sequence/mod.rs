//! Serial sequence orchestrator.

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
use color_eyre::eyre::Result;
use tracing::info_span;

use crate::commands::compose::SharedComposeArgs;
use crate::log;

mod iterate;
mod jit;
mod phase1c;
mod report;
mod resolve;
mod task_frames;
mod task_run;

use jit::StepComposeContext;
use phase1c::run_phase_1c_with_schema;
use resolve::{apply_user_set_to_hints, dry_run_sequence_target, is_auto_selectable_state};
#[cfg(test)]
use phase1c::find_first_unsupported;

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
/// The graph's own commands arrive already resolved, so what the gate approves
/// is byte-identical to what execution runs. Referenced prompt documents
/// contribute their template `::shell` directives through the same pass, which
/// is what makes "no approval prompts once the sequence starts" honest for work
/// several hops from the sequence document. Schema verdicts are deferred during
/// this discovery-only compose because prompt-task `params` bind just in time;
/// the task's canonical prepare validates the resulting effective values.
#[allow(clippy::too_many_arguments)]
fn approve_preflight_graph(
    graph: &composition::PreflightGraph,
    source: &ResolvedCompositionSource,
    source_context: &claudine::invocation_context::SourceContext,
    approval_cache: composition::SharedApprovalCache,
    shared: &SharedComposeArgs,
    launch_area: Option<&std::path::Path>,
    invocation: &claudine::invocation_context::InvocationContext,
    graph_context: &darkmatter::markdown::compose::ComposeContext,
) -> Result<HashSet<String>> {
    if graph.shell_commands.is_empty() && graph.prompt_documents.is_empty() {
        return Ok(HashSet::new());
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

    let compose_options = |path: &std::path::Path| {
        let source_context = invocation
            .derive_source(path)
            .expect("resolved prompt document always has a parent directory");
        let document = darkmatter::markdown::Markdown::try_from(path)
            .expect("preflight graph already loaded the prompt document");
        // The graph's launch snapshot, extended with any group this referenced
        // document needs: plain `ctx.*` stays launch-anchored no matter where
        // the prompt document is stored, while the document's own
        // `SourceContext` below still drives its file resolution.
        let requirements = darkmatter::markdown::compose::ContextRequirements::for_document(
            &document,
        );
        let mut context = graph_context.clone();
        invocation.extend_launch_context(&mut context, &requirements);
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(context)
            .with_source_file(path)
            .with_file_resolution_context(source_context.file_resolution_context().clone())
            .with_deferred_schema_verdict(true)
            // Defer the lifecycle subtree exactly as the per-step template
            // preflight does: this pass exists to discover `::shell`
            // directives, and resolving a deferred `success:`/`failure:` read
            // -side file reference here would trip on a file that event has
            // not created yet.
            .with_exclude_keys(claudine::composition::LIFECYCLE_EVENT_KEYS.iter().copied());
        if let Some(area) = launch_area {
            opts = opts.with_file_ref_fallback_dir(area.to_path_buf());
        }
        opts
    };

    let result =
        composition::resolve_graph_shell_approvals(graph, &approval_options, &compose_options)?;
    Ok(result.approved_commands)
}

/// Execute a full sequence: iterate steps, compose each, and report results.
#[allow(deprecated)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_sequence(
    source: &ResolvedCompositionSource,
    plan: SequencePlan,
    shared: &SharedComposeArgs,
    user_set_overrides: Option<serde_json::Value>,
    execution_options: SequenceExecutionOptions,
    verbose: u8,
    perf_enabled: bool,
    startup_timings: Option<crate::perf::StartupTimings>,
    invocation: claudine::invocation_context::InvocationContext,
    source_context: claudine::invocation_context::SourceContext,
    file_resolution_context: biscuit_file::FileResolutionContext,
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
            "<b>Sequence:</b> <yellow>{}</yellow> step(s), <i>fail_fast</i> is set to <blue>{}</blue>",
            total_steps, effective_fail_fast
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

    // ── Phase 1a: resolve target for every step (eager, pre-compose) ────
    // Hints come from raw source frontmatter so {{env.AGENT}} resolves
    // during per-step composition. User --set overrides on `agent`/`model`
    // are honored if present at the top level of `--set`.
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
    // ones behind `when:` guards that read false today — is resolved to bytes
    // and approved. A failure at this point is abort-all regardless of
    // `fail_fast`, and `--dry-run` performs this identical walk.
    // One base launch context for the whole graph, captured before per-task
    // target selection: every launch-facing graph expression (`when:` guards,
    // command interpolation, task/group variable defaults) projects the
    // caller's launch repository and package area from this snapshot, no
    // matter which repository a referenced task, group, or prompt document is
    // stored in. Per-task epochs below clone or extend this base and apply
    // their own resolved target overrides (D4).
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_document(
        &source.markdown,
    );
    let graph_context = prep_context.invocation.capture_launch_context(&requirements);
    prep_context.invocation.record_prepared_context_consumer(
        claudine::invocation_context::PreparedContextConsumer::Preflight,
    );
    let graph = composition::build_preflight_graph_with_invocation(
        &plan,
        source,
        graph_context.clone(),
        &prep_context.invocation,
        &prep_context.source_context,
    )?;
    let preflight_approved = approve_preflight_graph(
        &graph,
        source,
        &prep_context.source_context,
        Arc::clone(&shared_approval_cache),
        shared,
        Some(prep_context.launch_workspace.launch_cwd.as_path()),
        &prep_context.invocation,
        &graph_context,
    )?;
    let catalog = match prep_context.selection_config.as_ref() {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    // Phase 1 (2026-05-09-slow-prep): refresh is provider-scoped and only
    // happens when a frontmatter `model` hint will actually be validated
    // against the catalog. The previous unconditional `refresh_blocking()`
    // shelled out to `opencode models` even for `--claude` runs.
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
    let provider_locked = explicit_provider.is_some();
    let model_locked = cli_model.is_some();

    // ── Phase 1a/1b: resolve a target (or leave a render state) per step ─
    //
    // Under --dry-run the legacy non-TTY resolver must NOT run: it either
    // auto-picks a provider or returns a legacy selection error, both of
    // which would stop the per-step dry-run seam in
    // `execute_composition_request_inner` from rendering the unresolved /
    // invalid / not-installed agent states the spec requires. Classify the
    // shared frontmatter agent hint exactly like the direct compose
    // --dry-run path instead: auto-selectable states get a concrete target
    // so `{{env.AGENT}}` interpolates and the model resolves; every other
    // state becomes `None` and the seam renders it from the installed
    // snapshot. No picker fires and no agent-resolution failure aborts.
    let resolved_targets: Vec<Option<claudine::composition::ResolvedExecutionTarget>> = if shared
        .dry_run
    {
        let target = dry_run_sequence_target(explicit_provider, &raw_hints, snapshot, cli_model);
        (0..total_steps).map(|_| target.clone()).collect()
    } else {
        // Live path: every step shares the document-level `agent` hint, so
        // classify it once and apply the same TTY-only gate the direct
        // compose path enforces in `resolve_live_target_with_tty`
        // (composition/mod.rs). The legacy non-TTY resolver is deliberately
        // *not* used here: it ignores `agent_invalid` and silently falls back
        // to the configured favorite/default, which would auto-run a provider
        // for a state the dry-run table reports as prompting — the exact
        // drift this feature exists to prevent. An explicit `--<provider>`
        // flag still wins and bypasses the gate.
        let shared_state = if explicit_provider.is_some() {
            None
        } else {
            Some(claudine::composition::classify_agent_resolution(
                &raw_hints, snapshot,
            ))
        };

        // No-TTY prompting-state gate: a prompting state (no agent, invalid
        // scalar, not-installed scalar, multi-installed list, zero-installed
        // list) in a no-TTY session aborts before any provider runs, emitting
        // the same styled `AgentResolutionFailed` message the dry-run table
        // predicts. Mirrors the non-TTY arm of `resolve_live_target_with_tty`.
        if let Some(state) = &shared_state
            && !is_auto_selectable_state(state)
            && !is_tty
        {
            return Err(CompositionError::AgentResolutionFailed {
                source_path: source.resolved_path.clone(),
                state: state.clone(),
                installed: snapshot.runnable.clone(),
            }
            .into());
        }

        let mut drafts: Vec<SequenceStepDraft> = Vec::with_capacity(total_steps);
        let mut refreshed_providers: std::collections::BTreeSet<claudine::provider::Provider> =
            std::collections::BTreeSet::new();

        for step_index in 0..total_steps {
            let step = &plan.steps[step_index];

            // Picker plan scoped to the classified state. Reusing the compose
            // path's `scoped_picker_plan_for_state` narrows
            // `ListMultipleInstalled` to the installed-from-list providers;
            // every other state offers all installed agents. Explicit-provider
            // runs have no classified state and use the unscoped plan.
            let provider_plan = match shared_state.as_ref() {
                Some(state) => super::composition::scoped_picker_plan_for_state(
                    state, &raw_hints, snapshot, favorite,
                ),
                None => claudine::composition::build_picker_plan_with_hints(
                    &raw_hints, snapshot, favorite,
                ),
            }
            .unwrap_or(claudine::composition::ProviderPickerPlan {
                options: Vec::new(),
                default_index: 0,
            });

            // Provisional provider for model probing and the review-screen
            // default. Explicit flag wins; otherwise the classified state
            // decides: auto-selectable states resolve directly, TTY prompting
            // states fall back to the scoped plan's default (the user confirms
            // or changes it on the review screen). No-TTY prompting states
            // already aborted above.
            let resolved_provider = if let Some(provider) = explicit_provider {
                Some(provider)
            } else {
                match shared_state.as_ref() {
                    Some(claudine::composition::AgentResolutionState::Selected { provider }) => {
                        Some(*provider)
                    }
                    Some(claudine::composition::AgentResolutionState::ListOneInstalled {
                        selected,
                        ..
                    }) => Some(*selected),
                    _ => provider_plan
                        .options
                        .get(provider_plan.default_index)
                        .map(|o| o.provider),
                }
            };

            let (model, model_reason) = if let Some(provider) = resolved_provider {
                // Probe model resolution without catalog so the refresh gate
                // can observe whether CLI / provider env / generic MODEL would
                // override the frontmatter `model` hint. Refresh is skipped in
                // those cases (matches the direct compose path).
                let (_, probe_reason) = claudine::composition::resolve_model_with_hints(
                    provider, &raw_hints, cli_model, None,
                );
                // Refresh once per unique provider, and only when the
                // frontmatter `model` hint will actually be validated against
                // the catalog.
                if refreshed_providers.insert(provider) {
                    let _span = tracing::info_span!("compose_prep.model_catalog", provider = %provider.as_slug(), step = step_index).entered();
                    super::composition::refresh_for_model_validation(
                        &catalog,
                        provider,
                        &raw_hints,
                        Some(&probe_reason),
                    );
                }
                claudine::composition::resolve_model_with_hints(
                    provider,
                    &raw_hints,
                    cli_model,
                    Some(&catalog),
                )
            } else {
                (
                    None,
                    claudine::composition::ModelResolutionReason::ProviderDefault,
                )
            };

            drafts.push(SequenceStepDraft {
                step_index,
                step_name: step.name.clone(),
                provider_plan,
                proposed_model: model.clone(),
                model_reason,
                provider_locked,
                model_locked,
                resolved_provider,
            });
        }

        // ── Phase 1b: review (TTY) or resolve directly ────────────────────
        //
        // Auto-selectable states (`Selected` / `ListOneInstalled`) resolve to a
        // provider without prompting and must bypass the review screen, exactly
        // like direct compose's `resolve_live_target_with_tty` returns before
        // the picker for those states. Only prompting states reach the review
        // UI, and only on a TTY. `shared_state` is `None` only when an explicit
        // `--<provider>` flag is set, so `is_some_and` routes those — and the
        // no-TTY case — into the deterministic `else` branch.
        let needs_review = is_tty
            && shared_state
                .as_ref()
                .is_some_and(|state| !is_auto_selectable_state(state));
        let live_targets: Vec<claudine::composition::ResolvedExecutionTarget> =
            if needs_review {
                // Emit the state-specific pre-prompt message before the review
                // table renders, mirroring direct compose's
                // `prompt_for_agent_state`: a styled `Invalid Agent:` line for
                // a scalar invalid hint, the zero-installed-list breakdown for
                // an all-uninstallable list. Auto-selectable and plain picker
                // states never reach this arm (they take the `else` branch), so
                // every state here returns a message or `None` for a plain
                // picker. The message shares its source of truth with the
                // dry-run table cell and the no-TTY abort body, so the three
                // surfaces cannot drift. `shared_state` is always `Some` here
                // because `needs_review` requires it.
                if let Some(state) = shared_state.as_ref()
                    && let Some(markup) =
                        super::composition::agent_prompt_message(state, &source.resolved_path)
                {
                    log::message(&Prose::new(markup).render(&log::terminal()));
                }
                // TTY review screen. The --dry-run arm above returns before
                // this point, so the dry-run seam never invokes a picker.
                match super::selection_ui::review_sequence(drafts, &catalog) {
                    Ok(targets) => targets,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::Other
                            && e.to_string().contains("cancelled")
                        {
                            return Ok(130); // Treat as interrupt
                        }
                        return Err(e.into());
                    }
                }
            } else {
                // Auto-selectable, explicit flag, or non-TTY: each step's
                // provider was resolved deterministically onto the draft.
                let list_one = matches!(
                    shared_state.as_ref(),
                    Some(claudine::composition::AgentResolutionState::ListOneInstalled { .. })
                );
                drafts
                    .into_iter()
                    .map(|draft| {
                        let provider = draft
                            .resolved_provider
                            .unwrap_or(claudine::provider::Provider::Claude);
                        let provider_reason = if explicit_provider.is_some() {
                            claudine::composition::ProviderResolutionReason::ExplicitFlag
                        } else if list_one {
                            claudine::composition::ProviderResolutionReason::FrontmatterList
                        } else {
                            claudine::composition::ProviderResolutionReason::FrontmatterSingle
                        };
                        claudine::composition::ResolvedExecutionTarget {
                            provider,
                            provider_reason,
                            model: draft.proposed_model,
                            model_reason: draft.model_reason,
                        }
                    })
                    .collect()
            };
        live_targets.into_iter().map(Some).collect()
    };

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
        source_repo_root: source_repo_root.as_deref(),
        child_cwd: &prep_context.launch_workspace.child_cwd,
        launch_area: Some(prep_context.launch_workspace.launch_cwd.as_path()),
        shared,
        approval_cache: Arc::clone(&shared_approval_cache),
        inline_mode,
        file_resolution_context: &file_resolution_context,
        invocation: &prep_context.invocation,
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

    let run_context = iterate::SequenceRunContext {
        plan: &plan,
        graph: &graph,
        resolved_targets: &resolved_targets,
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
    };
    let (summary, interrupt_observed) =
        iterate::run_sequence_steps(&run_context, &mut perf_accumulator)?;

    if let Some(acc) = perf_accumulator.as_mut() {
        acc.set_invocation_work(&prep_context.invocation.work_snapshot());
    }

    report::emit_sequence_summary(
        &summary,
        perf_accumulator,
        &interrupted,
        interrupt_observed,
        silent,
    )
}

#[cfg(test)]
mod tests;
