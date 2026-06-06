//! Serial sequence orchestrator.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::horizontal_rule::{
    HorizontalRule, RuleAlignment, RuleStyle, RuleWeight,
};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::{
    self, CompositionError, CompositionExecutionRequest, CompositionMode, MissingProperty,
    PrepareOptions, PreparedComposition, ResolvedCompositionSource, SequenceExecutionOptions,
    SequenceMissingPropertiesStep, SequencePlan, SequenceRunSummary, SequenceStepDraft,
    SequenceStepResult,
};
use claudine::harness::{HarnessResolutionContext, has_harness_properties, parse_harness_plan};
use color_eyre::eyre::{Result, eyre};
use tracing::{debug, info_span};

use crate::commands::compose::SharedComposeArgs;
use crate::commands::schema_interactive::{
    collect_missing_values, emit_dropped_optional_warnings, render_status_report,
    resolve_interactive_options,
};
use crate::log;

/// Exit code emitted when Ctrl+C is observed during a sequence run.
/// Matches the standard `128 + SIGINT(2)` convention used by shells.
const SEQUENCE_INTERRUPT_EXIT_CODE: i32 = 130;

/// Context data for a single step in the sequence.
///
/// Captures the per-step environment overrides and the prepared composition
/// so that Phase 2 execution can reuse the work done during Phase 1 pre-flight.
struct StepContext {
    env_overrides: BTreeMap<String, String>,
    prepared: PreparedComposition,
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

    // `--step-timeout` applies to every step in the sequence. Early
    // validation happens in the CLI entry point; the raw string is
    // resolved per-step by the composition executor.
    let _ = shared.step_timeout_secs()?;

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

    let mut summary = SequenceRunSummary {
        total_steps,
        succeeded: 0,
        failed: 0,
        steps: Vec::with_capacity(total_steps),
    };

    // Persistent SIGINT tracker for the duration of the sequence run.
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
            let _ = interrupted_handler;
            Option::<()>::None
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
    let prep_context = super::composition::CompositionPrepContext::new(
        &source.original_ref,
        &source.resolved_path,
        &shared.excluded(),
    )?;
    let snapshot = &prep_context.installed_snapshot;
    let source_repo_root = prep_context.source_repo_root.clone();
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

    // ── Phase 1c: per-step compose with resolved AGENT in env_overrides ─
    //
    // Phase 5 schema validation: each step is run through
    // `prepare_direct_with_schema`, which surfaces `MissingProperties`
    // when the prompt's `$schema` declares required fields that the
    // (post-override) frontmatter does not satisfy. Missing-property
    // failures are aggregated across all steps so the user can fix the
    // full sequence in one edit. Invalid-required failures
    // (`SchemaValidation`) abort the sequence before any provider
    // session is launched.
    let (step_contexts, _cumulative_approved) = run_phase_1c_with_schema(
        source,
        &plan,
        &resolved_targets,
        &user_set_overrides,
        source_repo_root.as_deref(),
        shared,
        effective_fail_fast,
        inline_mode,
        Arc::clone(&shared_approval_cache),
        HashSet::new(),
        &interrupted,
        silent,
    )?;
    if step_contexts.is_empty() && total_steps > 0 {
        if let Some(mut acc) = perf_accumulator {
            acc.mark_env_setup_complete();
            acc.set_partial();
            crate::perf::emit_report(&acc.into_report());
        }
        return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
    }

    if !silent {
        let term = log::terminal();
        let status = Status::from_prose("Starting pre-flight checks".to_string())
            .state(StatusState::Info)
            .theme(biscuit_terminal::components::status::StatusTheme::Circular);
        log::message(&status.render(&term));
    }

    // ── Phase 1d: shell pre-flight for finalized steps ─────────────────
    let _preflight_span = info_span!("sequence_preflight", total_steps).entered();
    // (Shell approvals were already collected during Phase 1c)

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

    // ── Phase 2: execute each step ─────────────────────────────────────
    drop(_preflight_span);

    let mut interrupt_observed = false;
    for (step_index, step_ctx) in step_contexts.iter().enumerate() {
        if interrupted.load(Ordering::SeqCst) {
            interrupt_observed = true;
            if let Some(ref mut acc) = perf_accumulator {
                acc.set_partial();
            }
            break;
        }
        let step = &plan.steps[step_index];

        // Dry-run: separate each document's rendered metadata block with a
        // full-width horizontal rule on stderr. The per-document render
        // (frontmatter + table → stderr, body → stdout) happens inside
        // `execute_composition_request_inner`; the rule sits *between*
        // documents, so it precedes docs 2..M. `--quiet` / `--silent` have
        // no effect on dry-run output, so this emits unconditionally.
        if shared.dry_run && step_index > 0 {
            let hr = HorizontalRule::new()
                .style(RuleStyle::Dashes)
                .alignment(RuleAlignment::Full)
                .weight(RuleWeight::Medium)
                .render(&log::terminal());
            log::message(&hr);
        }

        let _step_span = info_span!(
            "sequence_step",
            step_index = step_index + 1,
            step_name = %step.name
        )
        .entered();

        if !silent {
            let status = Status::from_prose(format!(
                "[<yellow>{}/{}</yellow>] <i>starting</i> <b>{}</b>",
                step_index + 1,
                total_steps,
                step.name
            ))
            .state(StatusState::Info);
            log::message(&status.render(&log::terminal()));
        }

        let start = std::time::Instant::now();
        let prepared = step_ctx.prepared.clone();
        // `resolved_targets[step_index]` is itself an `Option` (a `--dry-run`
        // step with an unresolved agent state has no target); flatten so the
        // dry-run seam in `execute_composition_request_inner` sees `None` and
        // renders the classified state.
        let resolved_target = resolved_targets.get(step_index).cloned().flatten();
        // Captured before `resolved_target` moves into the request: a dry-run
        // step with no target is an unresolved agent state whose provider is a
        // placeholder, so the success status must not attribute a provider.
        let target_unresolved = resolved_target.is_none();

        let system_prompt_args = claudine::system_prompt::SystemPromptArgs {
            append_file: shared.append_system_prompt.clone(),
            replace_file: shared.replace_system_prompt.clone(),
        };

        let request = CompositionExecutionRequest {
            mode: if inline_mode {
                CompositionMode::InlineFrontmatterPrompt
            } else {
                CompositionMode::ChainedDocument
            },
            file_ref: source.original_ref.clone(),
            prepared,
            resolved_target,
            explicit_provider: shared.explicit_provider(),
            excluded: shared.excluded(),
            sequence: true,
            yolo: shared.yolo,
            include: shared.include.clone(),
            model: shared.model.clone(),
            output: shared.output,
            system_prompt_args,
            timeout: shared.timeout.clone(),
            step_timeout: shared.step_timeout.clone(),
            operation: shared.operation.clone(),
            sandbox: shared.sandbox,
            repo: shared.repo,
            dry_run: shared.dry_run,
            mcp: shared.mcp,
            mcp_use: shared.mcp_use.clone(),
            strict: shared.strict,
            session_interactive: shared.interactive,
            quiet: shared.quiet,
            silent: shared.silent,
            env_overrides: step_ctx.env_overrides.clone(),
            shared_approval_cache: Some(Arc::clone(&shared_approval_cache)),
            installed_snapshot: Some(prep_context.installed_snapshot.clone()),
            prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
            prep_launch_context: Some(prep_context.launch_context.clone()),
            prep_env_context: Some(prep_context.env_context.clone()),
            prep_launch_detection_error: prep_context.launch_detection_error.clone(),
            // Sequence steps render their header in-pipeline: per-step
            // prep already ran up front, so the executor's emit is timely.
            header_emitted: false,
        };

        let step_result = super::composition::execute_composition_request_inner(
            request,
            verbose,
            None,
            perf_enabled,
        );

        let duration = start.elapsed();

        match step_result {
            Ok(outcome) if outcome.exit_code == 0 => {
                if let Some(ref mut acc) = perf_accumulator {
                    acc.add_step(crate::perf::SequenceStepPerf {
                        step_index,
                        step_name: step.name.clone(),
                        wall_clock: duration,
                        compose_perf: step_ctx.prepared.compose_perf.clone(),
                        agent_perf: outcome.agent_perf,
                    });
                }
                summary.succeeded += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: true,
                    error: None,
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, provider = %outcome.provider, exit_code = 0, "sequence step succeeded");
                if !silent {
                    // A `--dry-run` step with an unresolved agent state has no
                    // real provider (`outcome.provider` is a placeholder), so
                    // drop the misleading "via {provider}" attribution; the
                    // rendered dry-run table already reports the agent state.
                    let prose = if shared.dry_run && target_unresolved {
                        format!(
                            "step <b><yellow>{}/{}</yellow></b> rendered (<dim><i>dry-run</i></dim>)",
                            step_index + 1,
                            total_steps,
                        )
                    } else {
                        format!(
                            "step <b><yellow>{}/{}</yellow></b> succeeded (<dim><i>via {}</i></dim>)",
                            step_index + 1,
                            total_steps,
                            outcome.provider
                        )
                    };
                    let status = Status::from_prose(prose).state(StatusState::Success);
                    log::message(&status.render(&log::terminal()));
                }
            }
            Ok(outcome)
                if outcome.exit_code == SEQUENCE_INTERRUPT_EXIT_CODE
                    || interrupted.load(Ordering::SeqCst) =>
            {
                if let Some(ref mut acc) = perf_accumulator {
                    acc.add_step(crate::perf::SequenceStepPerf {
                        step_index,
                        step_name: step.name.clone(),
                        wall_clock: duration,
                        compose_perf: step_ctx.prepared.compose_perf.clone(),
                        agent_perf: outcome.agent_perf,
                    });
                    acc.set_partial();
                }
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some("interrupted by SIGINT".into()),
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, "sequence step interrupted");
                if !silent {
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> interrupted by Ctrl+C",
                        step_index + 1,
                        total_steps,
                    ))
                    .state(StatusState::Failure);
                    log::message(&status.render(&log::terminal()));
                }
                interrupt_observed = true;
                break;
            }
            Ok(outcome) => {
                if let Some(ref mut acc) = perf_accumulator {
                    acc.add_step(crate::perf::SequenceStepPerf {
                        step_index,
                        step_name: step.name.clone(),
                        wall_clock: duration,
                        compose_perf: step_ctx.prepared.compose_perf.clone(),
                        agent_perf: outcome.agent_perf,
                    });
                }
                let error_msg = format!(
                    "provider {} exited with code {}",
                    outcome.provider, outcome.exit_code
                );
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, provider = %outcome.provider, exit_code = outcome.exit_code, error = %error_msg, "sequence step failed");
                if !silent {
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ))
                    .state(StatusState::Failure);
                    log::message(&status.render(&log::terminal()));
                }
                if effective_fail_fast {
                    if let Some(ref mut acc) = perf_accumulator {
                        acc.set_partial();
                    }
                    debug!(step_index = step_index + 1, step_name = %step.name, fail_fast = %effective_fail_fast, "sequence fail-fast triggered");
                    break;
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, error = %error_msg, "sequence step error");
                if !silent {
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ))
                    .state(StatusState::Failure);
                    log::message(&status.render(&log::terminal()));
                }
                if effective_fail_fast {
                    if let Some(ref mut acc) = perf_accumulator {
                        acc.set_partial();
                    }
                    debug!(step_index = step_index + 1, step_name = %step.name, fail_fast = %effective_fail_fast, "sequence fail-fast triggered");
                    break;
                }
            }
        }
    }

    // Final summary
    if !silent {
        eprintln!();
        if summary.failed == 0 {
            let status = Status::from_prose(format!(
                "Sequence finished: <green>{}</green> succeeded, 0 failed",
                summary.succeeded
            ))
            .state(StatusState::Success);
            log::message(&status.render(&log::terminal()));
        } else {
            let status = Status::from_prose(format!(
                "Sequence finished: <green>{}</green> succeeded, <red>{}</red> failed",
                summary.succeeded, summary.failed
            ))
            .state(StatusState::Failure);
            log::message(&status.render(&log::terminal()));
        }
    }

    // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
    // The perf report is always emitted to stderr when requested.
    if let Some(acc) = perf_accumulator {
        crate::perf::emit_report(&acc.into_report());
    }

    if interrupt_observed || interrupted.load(Ordering::SeqCst) {
        return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
    }
    if summary.failed > 0 { Ok(1) } else { Ok(0) }
}

/// Apply user `--set` overrides to raw selection hints.
///
/// Frontmatter `agent`/`model` can be overridden at the top level of
/// `--set`. The override is interpreted with the same parsing rules
/// (`agent` may be a string or array of strings; `model` may be a string
/// or array of strings).
fn apply_user_set_to_hints(
    mut hints: claudine::composition::EffectiveSelectionHints,
    user_set_overrides: Option<&serde_json::Value>,
) -> Result<claudine::composition::EffectiveSelectionHints> {
    let Some(serde_json::Value::Object(map)) = user_set_overrides else {
        return Ok(hints);
    };

    if let Some(agent_value) = map.get("agent") {
        let mut fm = darkmatter::markdown::Frontmatter::new();
        fm.insert("agent", agent_value.clone())
            .map_err(|e| eyre!("invalid --set agent: {e}"))?;
        let parsed = composition::parse_selection_hints_from_frontmatter(&fm)?;
        hints.agent = parsed.agent;
    }
    if let Some(model_value) = map.get("model") {
        let mut fm = darkmatter::markdown::Frontmatter::new();
        fm.insert("model", model_value.clone())
            .map_err(|e| eyre!("invalid --set model: {e}"))?;
        let parsed = composition::parse_selection_hints_from_frontmatter(&fm)?;
        hints.model = parsed.model;
    }

    Ok(hints)
}

/// Whether a classified agent state resolves to a provider without prompting.
///
/// Only [`Selected`](claudine::composition::AgentResolutionState::Selected) and
/// [`ListOneInstalled`](claudine::composition::AgentResolutionState::ListOneInstalled)
/// auto-select; every other state is a prompting state that must show the
/// picker on a TTY or abort in a no-TTY session. Mirrors the selected-provider
/// branch of the direct compose path's `resolve_live_target_with_tty`.
fn is_auto_selectable_state(state: &claudine::composition::AgentResolutionState) -> bool {
    use claudine::composition::AgentResolutionState;
    matches!(
        state,
        AgentResolutionState::Selected { .. } | AgentResolutionState::ListOneInstalled { .. }
    )
}

/// Resolve the per-step execution target for a `--dry-run` sequence.
///
/// Mirrors the dry-run branch of
/// [`eagerly_resolve_target`](super::composition::eagerly_resolve_target):
/// an explicit `--<provider>` flag and the auto-selectable frontmatter
/// states ([`Selected`](claudine::composition::AgentResolutionState::Selected),
/// [`ListOneInstalled`](claudine::composition::AgentResolutionState::ListOneInstalled))
/// resolve to a concrete target so `{{env.AGENT}}` interpolates and the
/// model resolves; every other agent-resolution state returns `None` so the
/// per-step dry-run seam renders it from the installed snapshot. The model
/// catalog is never consulted under `--dry-run` (catalog `None`), matching
/// the direct compose path.
///
/// Sequence steps share one source frontmatter `agent` hint, so the same
/// target applies to every step.
fn dry_run_sequence_target(
    explicit_provider: Option<claudine::provider::Provider>,
    raw_hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    cli_model: Option<&str>,
) -> Option<claudine::composition::ResolvedExecutionTarget> {
    use claudine::composition::{
        AgentResolutionState, ProviderResolutionReason, ResolvedExecutionTarget,
        classify_agent_resolution, resolve_model_with_hints,
    };

    if let Some(provider) = explicit_provider {
        let (model, model_reason) = resolve_model_with_hints(provider, raw_hints, cli_model, None);
        return Some(ResolvedExecutionTarget {
            provider,
            provider_reason: ProviderResolutionReason::ExplicitFlag,
            model,
            model_reason,
        });
    }

    let (provider, provider_reason) = match classify_agent_resolution(raw_hints, snapshot) {
        AgentResolutionState::Selected { provider } => {
            (provider, ProviderResolutionReason::FrontmatterSingle)
        }
        AgentResolutionState::ListOneInstalled { selected, .. } => {
            (selected, ProviderResolutionReason::FrontmatterList)
        }
        _ => return None,
    };
    let (model, model_reason) = resolve_model_with_hints(provider, raw_hints, cli_model, None);
    Some(ResolvedExecutionTarget {
        provider,
        provider_reason,
        model,
        model_reason,
    })
}

/// Run Phase 1c (per-step compose) with schema validation and aggregated
/// missing-property handling.
///
/// On the first pass each step is composed via
/// [`composition::prepare_direct_with_schema`]. Missing-property errors
/// are collected per-step; all other failures short-circuit immediately
/// (including [`CompositionError::SchemaValidation`] for invalid required
/// values). When at least one step reports missing properties and
/// Interactive Mode is allowed, the deduplicated missing set is collected
/// via `biscuit-tui` prompts and the loop re-runs with the merged
/// overrides. When Interactive Mode is not allowed, an aggregated
/// [`CompositionError::SequenceMissingProperties`] is returned so the
/// user can fix the full sequence in one edit.
#[allow(clippy::too_many_arguments)]
fn run_phase_1c_with_schema(
    source: &ResolvedCompositionSource,
    plan: &SequencePlan,
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    user_set_overrides: &Option<serde_json::Value>,
    source_repo_root: Option<&std::path::Path>,
    shared: &SharedComposeArgs,
    effective_fail_fast: bool,
    inline_mode: bool,
    shared_approval_cache: composition::SharedApprovalCache,
    initial_cumulative_approved: HashSet<String>,
    interrupted: &Arc<AtomicBool>,
    silent: bool,
) -> Result<(Vec<StepContext>, HashSet<String>)> {
    let mut overrides = user_set_overrides.clone();
    // At most one interactive collection pass — collected values are
    // shared across all steps that need the same property, and a second
    // attempt should always succeed (or fail with a different error).
    for attempt in 0..2 {
        let attempt_result = run_phase_1c_attempt(
            source,
            plan,
            resolved_targets,
            &overrides,
            source_repo_root,
            shared,
            effective_fail_fast,
            inline_mode,
            Arc::clone(&shared_approval_cache),
            initial_cumulative_approved.clone(),
            interrupted,
        )?;

        match attempt_result {
            Phase1cAttempt::Interrupted => {
                return Ok((Vec::new(), initial_cumulative_approved));
            }
            Phase1cAttempt::Success(contexts, approved) => {
                return Ok((contexts, approved));
            }
            Phase1cAttempt::Missing(contexts) => {
                if attempt > 0 {
                    // We already collected values once. If we're still
                    // seeing missing values, surface them.
                    return Err(into_sequence_missing(contexts).into());
                }
                // Honor `interactive.allowed()` first so non-TTY runs
                // produce the aggregated per-step report — matching the
                // direct `compose` path in
                // `pre_validate_with_interactive_collection`. Promoting
                // unsupported shapes ahead of this check would force
                // every non-TTY sequence with e.g. `object(required)` to
                // surface as `UnsupportedInteractiveSchema` instead of
                // the actionable aggregated `MissingProperties` report.
                let interactive = resolve_interactive_options(shared.silent);
                if !interactive.allowed() {
                    return Err(into_sequence_missing(contexts).into());
                }
                // Interactive Mode is allowed: now promote the first
                // missing property whose schema shape cannot be collected
                // via `biscuit-tui` (raw JSON Schema, `object`, `any`,
                // property-level union) so the user sees a targeted
                // `UnsupportedInteractiveSchema` error rather than a
                // generic aggregated report we can't satisfy.
                if let Some((source_path, property, shape)) =
                    find_first_unsupported(&failures_slice(&contexts))
                {
                    return Err(CompositionError::UnsupportedInteractiveSchema {
                        source_path,
                        property,
                        shape,
                    }
                    .into());
                }
                let collected = match collect_sequence_missing_values(&contexts, silent) {
                    Ok(values) => values,
                    Err(_) => {
                        // Ctrl-C / Esc / unsupported shape — surface the
                        // aggregated error so the user sees the actionable
                        // non-TTY report.
                        return Err(into_sequence_missing(contexts).into());
                    }
                };
                if collected.is_empty() {
                    return Err(into_sequence_missing(contexts).into());
                }
                overrides = Some(merge_overrides(overrides.as_ref(), collected));
            }
        }
    }
    // Loop bound enforces two attempts; the inner match returns from
    // either branch so this point is unreachable.
    unreachable!("phase 1c attempt loop exited without resolving")
}

/// Per-step missing-properties context.
///
/// Pairs the typed `SequenceMissingPropertiesStep` (returned to the user
/// as part of `CompositionError::SequenceMissingProperties`) with the
/// post-overlay effective override map for that step. The effective
/// overrides are needed by [`build_schema_status_report`] so the
/// pre-prompt diagnostic reflects what the prepare pipeline will
/// validate against — values supplied by reserved per-step overlay keys
/// and CLI `--set` / shorthand setters must show as `Valid` (or omitted)
/// rather than as `Missing`.
struct StepMissingContext {
    failure: SequenceMissingPropertiesStep,
    effective_overrides: Option<serde_json::Value>,
}

enum Phase1cAttempt {
    Success(Vec<StepContext>, HashSet<String>),
    Missing(Vec<StepMissingContext>),
    Interrupted,
}

/// Borrow just the failure records out of a `[StepMissingContext]` slice
/// for helpers that already operate on `&[SequenceMissingPropertiesStep]`.
fn failures_slice(contexts: &[StepMissingContext]) -> Vec<SequenceMissingPropertiesStep> {
    contexts.iter().map(|c| c.failure.clone()).collect()
}

/// Convert per-step contexts into the typed aggregated error variant.
fn into_sequence_missing(contexts: Vec<StepMissingContext>) -> CompositionError {
    let failures: Vec<SequenceMissingPropertiesStep> =
        contexts.into_iter().map(|c| c.failure).collect();
    CompositionError::SequenceMissingProperties {
        failure_count: failures.len(),
        failures,
    }
}

#[allow(clippy::too_many_arguments)]
fn run_phase_1c_attempt(
    source: &ResolvedCompositionSource,
    plan: &SequencePlan,
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    user_set_overrides: &Option<serde_json::Value>,
    source_repo_root: Option<&std::path::Path>,
    shared: &SharedComposeArgs,
    effective_fail_fast: bool,
    inline_mode: bool,
    shared_approval_cache: composition::SharedApprovalCache,
    initial_cumulative_approved: HashSet<String>,
    interrupted: &Arc<AtomicBool>,
) -> Result<Phase1cAttempt> {
    let total_steps = plan.steps.len();
    let mut cumulative_approved = initial_cumulative_approved;
    let mut step_contexts: Vec<StepContext> = Vec::with_capacity(total_steps);
    let mut missing_contexts: Vec<StepMissingContext> = Vec::new();

    for step_index in 0..total_steps {
        if interrupted.load(Ordering::SeqCst) {
            return Ok(Phase1cAttempt::Interrupted);
        }
        let overlay = build_step_overlay(plan, step_index);
        let step_set_overrides = overlay.as_set_overrides(user_set_overrides.clone());

        let mut env_overrides: BTreeMap<String, String> = BTreeMap::new();
        env_overrides.insert(
            "CLAUDINE_FAIL_FAST".to_string(),
            effective_fail_fast.to_string(),
        );
        let target = resolved_targets
            .get(step_index)
            .ok_or_else(|| eyre!("missing resolved target for step {}", step_index + 1))?;
        // A `--dry-run` step with an unresolved agent state has no target;
        // leave `AGENT` unset so composition matches the direct compose
        // --dry-run path (`{{env.AGENT}}` resolves to empty for those states).
        if let Some(target) = target {
            env_overrides.insert("AGENT".to_string(), target.provider.as_slug().to_string());
        }

        // Per-step schema pre-validation BEFORE preflight. If a step's
        // effective frontmatter (source + overlay overrides) is missing
        // required schema values, capture them for aggregation and
        // continue — do NOT let Darkmatter's preflight compose pass
        // surface a raw `SchemaValidationFailed` here, because that
        // would short-circuit aggregation across remaining steps.
        let (step_source, step_overrides) =
            match composition::pre_validate_schema(source, Some(&step_set_overrides)) {
                Ok(pre) => {
                    emit_dropped_optional_warnings(&pre.dropped_optionals);
                    (
                        pre.source,
                        pre.set_overrides
                            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
                    )
                }
                Err(CompositionError::MissingProperties {
                    source_path,
                    missing,
                    frontmatter_description,
                    pointer_paths,
                }) => {
                    let step = &plan.steps[step_index];
                    missing_contexts.push(StepMissingContext {
                        failure: SequenceMissingPropertiesStep {
                            step: step_index + 1,
                            step_name: step.name.clone(),
                            source_path,
                            missing,
                            frontmatter_description,
                            pointer_paths,
                        },
                        effective_overrides: Some(step_set_overrides.clone()),
                    });
                    continue;
                }
                Err(other) => return Err(other.into()),
            };

        let compose_options = {
            let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
            for (key, value) in &env_overrides {
                ctx.env_mut().insert(key.clone(), value.clone());
            }
            let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx)
                .with_source_file(&step_source.resolved_path);
            opts = opts.with_set_overrides(step_overrides.clone());
            opts
        };

        let approval_options = super::build_harness_shell_options_with_cache(
            &step_source.resolved_path,
            source_repo_root,
            Some(Arc::clone(&shared_approval_cache)),
        );

        let template_preflight = composition::resolve_shell_approvals(
            Some(&step_source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )?;
        cumulative_approved.extend(template_preflight.approved_commands.iter().cloned());

        let prepare_options = PrepareOptions {
            set_overrides: Some(step_overrides.clone()),
            pre_approved_commands: Some(cumulative_approved.clone()),
            env_overrides: env_overrides.clone(),
            perf_enabled: shared.perf,
            source_repo_root: source_repo_root.map(std::path::Path::to_path_buf),
        };

        // Inline steps prepare via `prepare_inline_with_schema` so the
        // composed `prompt` frontmatter becomes the agent prompt and the
        // prepared closure is `Inline` (drives body write-back in Phase 2).
        // Compose steps keep the body-as-prompt behavior.
        let prepare_result = if inline_mode {
            composition::prepare_inline_with_schema(&step_source, prepare_options)
        } else {
            composition::prepare_direct_with_schema(&step_source, prepare_options)
        };
        let prepared = match prepare_result {
            Ok(prepared) => {
                emit_dropped_optional_warnings(&prepared.dropped_optionals);
                prepared
            }
            Err(CompositionError::MissingProperties {
                source_path,
                missing,
                frontmatter_description,
                pointer_paths,
            }) => {
                let step = &plan.steps[step_index];
                missing_contexts.push(StepMissingContext {
                    failure: SequenceMissingPropertiesStep {
                        step: step_index + 1,
                        step_name: step.name.clone(),
                        source_path,
                        missing,
                        frontmatter_description,
                        pointer_paths,
                    },
                    effective_overrides: Some(step_overrides.clone()),
                });
                // Skip harness preflight for the failed step; continue so
                // we accumulate every step's missing properties.
                continue;
            }
            Err(other) => return Err(other.into()),
        };

        if has_harness_properties(&prepared.effective_frontmatter) {
            let effective_repo_root = prepared.source_repo_root.as_deref();
            let resolve_ctx = HarnessResolutionContext {
                source_path: &prepared.resolved_path,
                repo_root: effective_repo_root,
            };
            let harness_plan = parse_harness_plan(
                &prepared.effective_frontmatter,
                &prepared.resolved_path,
                &resolve_ctx,
            )
            .map_err(|e| eyre!("{e}"))?;
            let harness_preflight = composition::resolve_shell_approvals(
                None,
                None,
                Some(&harness_plan),
                &approval_options,
            )
            .map_err(|e| eyre!("{e}"))?;
            cumulative_approved.extend(harness_preflight.approved_commands.iter().cloned());
        }

        step_contexts.push(StepContext {
            env_overrides,
            prepared,
        });
    }

    if !missing_contexts.is_empty() {
        return Ok(Phase1cAttempt::Missing(missing_contexts));
    }
    Ok(Phase1cAttempt::Success(step_contexts, cumulative_approved))
}

/// Dedupe missing properties across all failed steps and prompt for each
/// unique `(name, type_label, description)` exactly once.
///
/// Returns a JSON map of `{ property_name: collected_value }` suitable for
/// merging into the user `--set` overrides. The first failure that
/// declared the property supplies the metadata used for the prompt.
///
/// The per-step status report uses `StepMissingContext::effective_overrides`
/// so a required property already satisfied by a reserved per-step overlay
/// value or by a CLI `--set` / shorthand setter is not reported as
/// `Missing` while the user is being prompted for a different property
/// (matches the direct `compose` status-drift fix from review-5).
fn collect_sequence_missing_values(
    contexts: &[StepMissingContext],
    silent: bool,
) -> std::io::Result<serde_json::Map<String, serde_json::Value>> {
    if !silent {
        let term = log::terminal();
        // Render one status report per step so the user sees the same
        // diagnostic they get for direct compose.
        for ctx in contexts {
            let failure = &ctx.failure;
            // Build a minimal source view from the recorded path for the
            // status report. The library helper expects a resolved source
            // so we re-resolve here; if that fails we fall through with
            // a header-only note.
            if let Ok(source) =
                composition::resolve_composition_source(&failure.source_path.display().to_string())
                && let Ok(Some(report)) = composition::build_schema_status_report(
                    &source,
                    ctx.effective_overrides.as_ref(),
                )
            {
                let status = Status::from_prose(format!(
                    "<b>Step {}:</b> <cyan>{}</cyan>",
                    failure.step, failure.step_name
                ))
                .state(StatusState::Info);
                log::message(&status.render(&term));
                render_status_report(&report, &term);
            }
        }
    }

    // Build a deduped list, keyed by (name, type_label, description).
    let mut seen_keys: HashSet<(String, Option<String>, Option<String>)> = HashSet::new();
    let mut unique: Vec<MissingProperty> = Vec::new();
    for ctx in contexts {
        for prop in &ctx.failure.missing {
            let key = (
                prop.name.clone(),
                prop.type_label.clone(),
                prop.description.clone(),
            );
            if seen_keys.insert(key) {
                unique.push(prop.clone());
            }
        }
    }

    collect_missing_values(&unique)
}

/// Locate the first missing property across all sequence step failures
/// whose schema shape cannot be collected interactively.
///
/// Returns `(source_path, property_name, shape_label)` so the caller can
/// build a typed [`CompositionError::UnsupportedInteractiveSchema`] that
/// matches the direct `compose` path. The shape label falls back to
/// `(unknown)` when the property has no recorded type label.
fn find_first_unsupported(
    failures: &[SequenceMissingPropertiesStep],
) -> Option<(std::path::PathBuf, String, String)> {
    for failure in failures {
        for prop in &failure.missing {
            if prop.interactive_shape.is_none() {
                return Some((
                    failure.source_path.clone(),
                    prop.name.clone(),
                    prop.type_label
                        .clone()
                        .unwrap_or_else(|| "(unknown)".to_string()),
                ));
            }
        }
    }
    None
}

fn merge_overrides(
    base: Option<&serde_json::Value>,
    collected: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut out = match base {
        Some(serde_json::Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    for (k, v) in collected {
        out.insert(k, v);
    }
    serde_json::Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::composition::{InteractiveShape, TextFormat};
    use std::path::PathBuf;

    fn step(
        n: usize,
        path: &str,
        props: Vec<MissingProperty>,
    ) -> SequenceMissingPropertiesStep {
        SequenceMissingPropertiesStep {
            step: n,
            step_name: format!("step-{n}"),
            source_path: PathBuf::from(path),
            missing: props,
            frontmatter_description: None,
            pointer_paths: Vec::new(),
        }
    }

    fn prop_supported(name: &str) -> MissingProperty {
        MissingProperty {
            name: name.to_string(),
            type_label: Some("string".to_string()),
            description: None,
            interactive_shape: Some(InteractiveShape::Text {
                format: TextFormat::Plain,
            }),
        }
    }

    fn prop_unsupported(name: &str, type_label: Option<&str>) -> MissingProperty {
        MissingProperty {
            name: name.to_string(),
            type_label: type_label.map(str::to_string),
            description: None,
            interactive_shape: None,
        }
    }

    #[test]
    fn auto_selectable_states_are_classified() {
        use claudine::composition::AgentResolutionState;
        use claudine::provider::Provider;

        // Only Selected and ListOneInstalled auto-select; every other state is
        // a prompting state that the no-TTY gate must abort on.
        assert!(is_auto_selectable_state(&AgentResolutionState::Selected {
            provider: Provider::Claude
        }));
        assert!(is_auto_selectable_state(
            &AgentResolutionState::ListOneInstalled {
                selected: Provider::Claude,
                not_installed: Vec::new(),
                invalid: Vec::new(),
            }
        ));

        for state in [
            AgentResolutionState::NoAgent,
            AgentResolutionState::SingleInvalid {
                hint: "nope".into(),
            },
            AgentResolutionState::SingleNotInstalled {
                provider: Provider::Gemini,
            },
            AgentResolutionState::ListMultipleInstalled {
                installed: vec![Provider::Claude, Provider::Codex],
                not_installed: Vec::new(),
                invalid: Vec::new(),
            },
            AgentResolutionState::ZeroInstalledList {
                not_installed: vec![Provider::Gemini],
                invalid: Vec::new(),
            },
        ] {
            assert!(
                !is_auto_selectable_state(&state),
                "prompting state {state:?} must not be auto-selectable"
            );
        }
    }

    #[test]
    fn find_first_unsupported_returns_none_when_all_supported() {
        let failures = vec![step(
            1,
            "/tmp/a.md",
            vec![prop_supported("topic"), prop_supported("tier")],
        )];
        assert!(find_first_unsupported(&failures).is_none());
    }

    #[test]
    fn find_first_unsupported_returns_first_unsupported_property() {
        let failures = vec![
            step(1, "/tmp/a.md", vec![prop_supported("topic")]),
            step(
                2,
                "/tmp/b.md",
                vec![
                    prop_supported("tier"),
                    prop_unsupported("config", Some("object")),
                    prop_unsupported("other", None),
                ],
            ),
        ];
        let (path, name, shape) = find_first_unsupported(&failures).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/b.md"));
        assert_eq!(name, "config");
        assert_eq!(shape, "object");
    }

    #[test]
    fn find_first_unsupported_falls_back_to_unknown_label() {
        let failures = vec![step(
            1,
            "/tmp/a.md",
            vec![prop_unsupported("raw", None)],
        )];
        let (_, _, shape) = find_first_unsupported(&failures).unwrap();
        assert_eq!(shape, "(unknown)");
    }
}

