//! Loop execution scaffolding and shared prep helpers for composition.
//!
//! `run_loop_with_overrides` seeds the iteration engine from resolved control
//! variables; `build_loop_iteration_output` translates a single execution
//! outcome into the loop engine's typed iteration result. The prep-timing and
//! warning helpers are shared by both compose entrypoints.

use darkmatter::markdown::compose::ComposeWarning;

use super::interrupt::USER_INTERRUPT_EXIT_CODE;

/// Record a named prep work unit (P-5a) when `--perf` is active.
///
/// Each unit is a disjoint sub-window of the `compose_entry` → request prep
/// window, so the recorded timings carve `prep_phase` into reconciling
/// `Structural` children (`shell approval` being the usual dominant cost),
/// leaving the small remainder in `prep → unattributed`.
pub(crate) fn record_prep_substage(
    out: &mut Vec<crate::perf::SubstageTiming>,
    enabled: bool,
    name: &'static str,
    started: std::time::Instant,
) {
    if enabled {
        out.push(crate::perf::SubstageTiming::new(name, started.elapsed()));
    }
}

/// Emits non-fatal Darkmatter compose warnings to stderr, unless `--silent` is set.
pub(crate) fn emit_compose_warnings(warnings: &[ComposeWarning], silent: bool) {
    if silent {
        return;
    }
    for warning in warnings {
        let mut message = warning.message.clone();
        if let Some(line) = warning.line_number {
            message = format!("[{}] line {line}: {message}", warning.stage);
        } else {
            message = format!("[{}] {message}", warning.stage);
        }
        crate::log::warn(&message);
    }
}

/// Render a `LoopRateLimited` error inline so the user sees the styled
/// halt notice before the wrapper exits with `EX_TEMPFAIL`.
pub(crate) fn emit_rate_limit_halt(error: &claudine::composition::CompositionError) {
    use biscuit_terminal::errors::BlockError;
    let term = crate::log::terminal();
    let rendered = error.report_block_error(&term);
    crate::log::message("");
    crate::log::message(&rendered);
    crate::log::message("");
}

/// Translate a [`SingleCompositionOutcome`] into a
/// [`claudine::composition::LoopIterationOutput`] that the loop engine can
/// inspect for rate-limit policy and honest error classification.
///
/// On a non-zero `outcome.exit_code` this builds a
/// [`claudine::composition::CompositionError::LoopIterationFailed`] with a
/// `reason` and `exit_reason` pulled from the iteration's session_end
/// signals (e.g. `step_timeout`) — never the old
/// `LoopInvalid("provider exited with code N")` overload, which was
/// reserved for malformed `loop:` frontmatter.
///
/// Always attaches the iteration's rate-limit trailer and
/// provider/model attribution so the engine can apply the configured
/// [`claudine::composition::OnRateLimit`] policy between iterations.
///
/// [`SingleCompositionOutcome`]:
///     crate::commands::wrap::composition::SingleCompositionOutcome
pub(crate) fn build_loop_iteration_output(
    iteration: usize,
    prompt_path: &std::path::Path,
    outcome: crate::commands::wrap::composition::SingleCompositionOutcome,
) -> claudine::composition::LoopIterationOutput {
    let signals = outcome.iteration_signals.unwrap_or_default();
    let rate_limit = signals.rate_limit.clone();
    let exit_reason = signals.exit_reason.clone();
    let provider_id = signals.provider_id.clone();
    let model_id = signals.model_id.clone();

    if outcome.exit_code == 0 {
        // The captured entry this iteration committed to `outputs` is the same
        // text `_loop_last_output` names, so the two agree by construction
        // rather than the ambient reporting a placeholder empty string.
        claudine::composition::LoopIterationOutput::success(
            outcome.final_output.unwrap_or_default(),
        )
            .with_rate_limit(rate_limit)
            .with_exit_reason(exit_reason)
            .with_attribution(provider_id, model_id)
            .with_terminal_signal(outcome.terminal_signal)
    } else {
        // Build a human-readable cause that surfaces the structured
        // exit_reason at the top. Watchdog detail (e.g. "no stream
        // activity for 30m; terminating due to step_timeout") rides
        // along on the next line.
        let reason = match (&exit_reason, &signals.error_message) {
            (Some(kind), Some(detail)) => format!("{kind}\n  ↳ {detail}"),
            (Some(kind), None) => kind.clone(),
            (None, Some(detail)) => detail.clone(),
            (None, None) => "provider exited non-zero".to_string(),
        };
        let error = claudine::composition::CompositionError::LoopIterationFailed {
            iteration,
            prompt_path: prompt_path.to_path_buf(),
            exit_code: outcome.exit_code,
            reason,
            exit_reason: exit_reason.clone(),
        };
        claudine::composition::LoopIterationOutput::failure("", outcome.exit_code, error)
            .with_rate_limit(rate_limit)
            .with_exit_reason(exit_reason)
            .with_attribution(provider_id, model_id)
            .with_terminal_signal(outcome.terminal_signal)
    }
}

/// Run a composition loop seeded from resolved control variables.
///
/// Returns `Ok(None)` when the source has no `loop` frontmatter, matching
/// [`claudine::composition::execute_loop`].
///
/// The caller is responsible for building the loop seed (control-variable
/// frontmatter) and the lifecycle runtime dependencies. This function wraps
/// the executor with user-interrupt short-circuiting and CWD restoration,
/// then drives the loop through [`claudine::composition::execute_loop_with_lifecycle`]
/// so `initialize` is emitted exactly once and the post-`finalize` loop gate
/// runs in the required order.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_loop_with_overrides<F>(
    source: &claudine::composition::ResolvedCompositionSource,
    config: &claudine::composition::LoopConfig,
    initial_frontmatter: serde_json::Map<String, serde_json::Value>,
    options: claudine::composition::LoopExecutionOptions,
    lifecycle_config: &claudine::composition::LifecycleConfig,
    lifecycle_ctx: &claudine::composition::LifecycleRuntimeContext<'_>,
    effect_engine: &darkmatter::effects::EffectEngine,
    shell_runner: &dyn claudine::composition::ShellRunner,
    emitter: &dyn claudine::composition::LifecycleEmitter,
    mut executor: F,
) -> std::result::Result<
    Option<claudine::composition::LoopExecutionResult>,
    claudine::composition::CompositionError,
>
where
    F: FnMut(
        claudine::composition::LoopIterationContext,
        &mut claudine::composition::LifecycleRunGuard<'_>,
    ) -> std::result::Result<
        claudine::composition::LoopIterationOutput,
        claudine::composition::CompositionError,
    >,
{
    let prompt_path = source.resolved_path.clone();

    // Capture the launch CWD before any iteration runs so we can restore it
    // between iterations. The wrap layer's `switch_process_cwd` mutates the
    // process-global CWD to the detected repo/git root inside each iteration;
    // without restoration, iteration 2's prepare resolves any CLI-supplied
    // `file(required)` setter against the post-switch root rather than the
    // user's original launch directory. `PWD` is injected onto the child
    // `Command` env map in `build_child_env_with_launch`, so we do not need
    // to mutate the process-global `PWD` here.
    let launch_cwd = std::env::current_dir().ok();

    // The Ctrl+C SIGINT handler is installed at the top of the compose
    // subcommand (see `install_user_interrupt_guard`) so it covers the
    // entire prep window, not just the loop. The wrapped executor below
    // simply observes the process-scoped flag and short-circuits
    // remaining iterations once the user has interrupted.
    let prompt_path_for_executor = prompt_path.clone();
    let wrapped_executor = move |
        ctx: claudine::composition::LoopIterationContext,
        guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    | {
        if crate::output::user_interrupt_observed() {
            return Ok(claudine::composition::LoopIterationOutput::failure(
                "",
                USER_INTERRUPT_EXIT_CODE,
                claudine::composition::CompositionError::LoopInterrupted {
                    prompt_path: prompt_path_for_executor.clone(),
                },
            ));
        }
        // Restore launch CWD before each iteration so per-iteration compose
        // (which uses ambient CWD via `validate_file_reference`) sees the
        // same root that the pre-loop validation saw.
        if let Some(ref cwd) = launch_cwd {
            let _ = std::env::set_current_dir(cwd);
        }
        let output = executor(ctx, guard)?;
        if crate::output::user_interrupt_observed() {
            return Ok(claudine::composition::LoopIterationOutput::failure(
                output.output,
                USER_INTERRUPT_EXIT_CODE,
                claudine::composition::CompositionError::LoopInterrupted {
                    prompt_path: prompt_path_for_executor.clone(),
                },
            ));
        }
        Ok(output)
    };

    let mut result = claudine::composition::execute_loop_with_lifecycle(
        &source.resolved_path,
        config,
        initial_frontmatter,
        options,
        lifecycle_config,
        lifecycle_ctx,
        effect_engine,
        shell_runner,
        emitter,
        wrapped_executor,
    )?;

    if crate::output::user_interrupt_observed() {
        // Force the interrupt outcome regardless of fail-fast: under
        // `fail_fast: false` the engine would have continued past the
        // first short-circuited iteration, so overwrite the result so
        // callers see exit code 130 and a `LoopInterrupted` error.
        result.final_exit_code = USER_INTERRUPT_EXIT_CODE;
        result.error = Some(claudine::composition::CompositionError::LoopInterrupted {
            prompt_path: prompt_path.clone(),
        });
    }

    Ok(Some(result))
}
