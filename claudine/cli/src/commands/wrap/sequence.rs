//! Serial sequence orchestrator.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::{
    self, CompositionError, CompositionExecutionRequest, CompositionMode, PrepareOptions,
    PreparedComposition, ResolvedCompositionSource, SequenceExecutionOptions, SequencePlan,
    SequenceRunSummary, SequenceStepDraft, SequenceStepResult,
};
use claudine::harness::{HarnessResolutionContext, has_harness_properties, parse_harness_plan};
use color_eyre::eyre::{Result, eyre};
use tracing::{debug, info_span};

use crate::commands::compose::SharedComposeArgs;
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
    let sequence_start = std::time::Instant::now();
    let silent = shared.silent;

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

    let mut cumulative_approved: HashSet<String> = HashSet::new();
    let shared_approval_cache: composition::SharedApprovalCache =
        Arc::new(Mutex::new(HashMap::new()));

    // ── Phase 1a: resolve target for every step (eager, pre-compose) ────
    // Hints come from raw source frontmatter so {{env.AGENT}} resolves
    // during per-step composition. User --set overrides on `agent`/`model`
    // are honored if present at the top level of `--set`.
    let raw_hints =
        composition::parse_selection_hints_from_frontmatter(source.markdown.frontmatter())?;
    let raw_hints = apply_user_set_to_hints(raw_hints, user_set_overrides.as_ref())?;

    let clients = sniff::programs::InstalledAiClients::new();
    let installed: Vec<claudine::provider::Provider> = claudine::provider::PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
        .collect();
    let excluded = shared.excluded();
    let snapshot = claudine::composition::build_installed_snapshot(&installed, &excluded);

    let source_repo_root = source.resolved_path.parent().and_then(|parent| {
        sniff::filesystem::git::detect_git(parent, false, 1)
            .ok()
            .flatten()
            .map(|info| info.repo_root)
    });
    let selection_config_path = source_repo_root.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    let selection_config = super::composition::load_selection_config(&selection_config_path);
    let catalog = match &selection_config {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    catalog.refresh_blocking();
    let favorite = selection_config.as_ref().and_then(|c| c.favorite);

    let explicit_provider = shared.explicit_provider();
    let cli_model = shared.model.as_deref();
    let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let provider_locked = explicit_provider.is_some();
    let model_locked = cli_model.is_some();

    let mut drafts: Vec<SequenceStepDraft> = Vec::with_capacity(total_steps);
    let mut failures: Vec<claudine::composition::SequenceSelectionFailure> = Vec::new();

    for step_index in 0..total_steps {
        let step = &plan.steps[step_index];

        // Resolve provider for this step
        let provider_result = if let Some(provider) = explicit_provider {
            Ok(provider)
        } else {
            let target = claudine::composition::resolve_target_non_tty_with_hints(
                None,
                &raw_hints,
                &snapshot,
                favorite,
                cli_model,
                Some(&catalog),
            );
            target.map(|t| t.provider)
        };

        let provider_plan = match claudine::composition::build_picker_plan_with_hints(
            &raw_hints,
            &snapshot,
            favorite,
        ) {
            Ok(plan) => plan,
            Err(_) => claudine::composition::ProviderPickerPlan {
                options: Vec::new(),
                default_index: 0,
            },
        };

        let provider_for_model = explicit_provider.or(provider_result.as_ref().ok().copied());
        let (model, model_reason) = if let Some(provider) = provider_for_model {
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

        match provider_result {
            Ok(_provider) => {
                drafts.push(SequenceStepDraft {
                    step_index,
                    step_name: step.name.clone(),
                    provider_plan,
                    proposed_model: model.clone(),
                    model_reason,
                    provider_locked,
                    model_locked,
                    resolved_provider: explicit_provider,
                });
            }
            Err(ref err) => {
                failures.push(claudine::composition::SequenceSelectionFailure {
                    step: step_index + 1,
                    step_name: step.name.clone(),
                    reason: err.to_string(),
                    installed: snapshot.all_installed.clone(),
                });
            }
        }
    }

    // ── Phase 1b: review (TTY) or validate (non-TTY) ───────────────────
    let resolved_targets: Vec<claudine::composition::ResolvedExecutionTarget> = if !failures
        .is_empty()
    {
        // Non-TTY aggregate failure path
        return Err(CompositionError::SequenceSelectionFailed {
            failure_count: failures.len(),
            failures,
        }
        .into());
    } else if is_tty && explicit_provider.is_none() {
        // TTY review screen
        match super::selection_ui::review_sequence(drafts, &catalog) {
            Ok(targets) => targets,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::Other && e.to_string().contains("cancelled") {
                    return Ok(130); // Treat as interrupt
                }
                return Err(e.into());
            }
        }
    } else {
        // Non-TTY success path: convert drafts to resolved targets directly.
        // When an explicit provider flag was given, every step uses it.
        drafts
            .into_iter()
            .map(|draft| {
                let provider = explicit_provider
                    .or_else(|| {
                        draft
                            .provider_plan
                            .options
                            .get(draft.provider_plan.default_index)
                            .map(|o| o.provider)
                    })
                    .unwrap_or(claudine::provider::Provider::Claude);
                let provider_reason = if explicit_provider.is_some() {
                    claudine::composition::ProviderResolutionReason::ExplicitFlag
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

    // ── Phase 1c: per-step compose with resolved AGENT in env_overrides ─
    let mut step_contexts: Vec<StepContext> = Vec::with_capacity(total_steps);
    for step_index in 0..total_steps {
        if interrupted.load(Ordering::SeqCst) {
            if let Some(mut acc) = perf_accumulator {
                acc.mark_env_setup_complete();
                acc.set_partial();
                let total = sequence_start.elapsed();
                let report = acc.into_report(total);
                eprint!("{}", crate::perf::render_perf_report(&report));
            }
            return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
        }
        let overlay = build_step_overlay(&plan, step_index);
        let step_set_overrides = overlay.as_set_overrides(user_set_overrides.clone());

        let mut env_overrides: BTreeMap<String, String> = BTreeMap::new();
        env_overrides.insert("FAIL_FAST".to_string(), effective_fail_fast.to_string());
        // Inject AGENT for this step using the resolved target so
        // {{env.AGENT}} in the body composes correctly. The mutation of
        // the parent process env happens once below for step 0, then
        // gets refreshed each step in case earlier steps changed it.
        let target = resolved_targets
            .get(step_index)
            .ok_or_else(|| eyre!("missing resolved target for step {}", step_index + 1))?;
        let slug = target.provider.as_slug().to_string();
        env_overrides.insert("AGENT".to_string(), slug.clone());
        // SAFETY: sequence orchestrator runs on the main task; per-step
        // updates of this single env var precede any composition or
        // child-process spawn for that step.
        unsafe {
            std::env::set_var("AGENT", &slug);
        }

        let compose_options = {
            let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
            for (key, value) in &env_overrides {
                ctx.env_mut().insert(key.clone(), value.clone());
            }
            let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx)
                .with_source_file(&source.resolved_path);
            opts = opts.with_set_overrides(step_set_overrides.clone());
            opts
        };

        let approval_options = super::build_harness_shell_options_with_cache(
            &source.resolved_path,
            None,
            Some(Arc::clone(&shared_approval_cache)),
        );

        // Template pre-flight
        let template_preflight = composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )?;
        cumulative_approved.extend(template_preflight.approved_commands.iter().cloned());

        // Prepare composition
        let prepare_options = PrepareOptions {
            set_overrides: Some(step_set_overrides.clone()),
            pre_approved_commands: Some(cumulative_approved.clone()),
            env_overrides: env_overrides.clone(),
            perf_enabled: shared.perf,
        };
        let prepared = composition::prepare_direct(source, prepare_options)?;

        // Harness pre-flight
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
        let resolved_target = resolved_targets.get(step_index).cloned();

        let system_prompt_args = claudine::system_prompt::SystemPromptArgs {
            append_file: shared.append_system_prompt.clone(),
            replace_file: shared.replace_system_prompt.clone(),
        };

        let request = CompositionExecutionRequest {
            mode: CompositionMode::ChainedDocument,
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
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> succeeded (<dim><i>via {}</i></dim>)",
                        step_index + 1,
                        total_steps,
                        outcome.provider
                    ))
                    .state(StatusState::Success);
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
        let total = sequence_start.elapsed();
        let report = acc.into_report(total);
        eprint!("{}", crate::perf::render_perf_report(&report));
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
