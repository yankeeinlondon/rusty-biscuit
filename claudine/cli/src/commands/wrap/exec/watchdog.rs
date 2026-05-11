use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use claudine::stream::progress::LiveMetrics;
use claudine::stream::prompt_timing as prompt_timing_mod;
use claudine::stream::prompt_timing::{HeaderKind, PromptTimingContext};

use super::subagent_watchdog::WatchdogState;
use super::termination::{WatchdogTermination, WatchdogTerminationReason};
use super::timeouts::TimeoutConfig;

/// Result of evaluating the unified two-rule timeout watchdog on a single tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTickResult {
    /// No rules triggered; continue monitoring.
    Ok,
    /// A timeout rule breached; terminate with this request.
    Breach(WatchdogTermination),
}
use super::super::section::{Section, SectionTracker};
use super::super::stream_io::StreamOutput;

/// Spawn a dedicated ticker that runs [`StreamTextRenderer::flush_if_idle`]
/// every 30 seconds.
///
/// Independent from the prompt-scoped timing monitor so buffered markdown
/// reaches stdout even on runs that have no prompt context (wrapper
/// passthrough) and regardless of whether any periodic header is being
/// emitted. The 30-second cadence and 30-second silence window are the
/// tuning preserved from the previous heartbeat thread.
///
/// When `watchdog_state` is provided and the unified `step_timeout` rule
/// is enabled, the ticker also emits at most one diagnostic line per
/// active subagent per silence window:
///
///   `⏳ Awaiting subagent: <name-or-id> (<elapsed-since-start>)`
///
/// These lines route through the shared [`SectionTracker`] so spacing stays
/// consistent with the live sink.
pub(crate) fn spawn_flush_if_idle_ticker(
    stream_output: Arc<StreamOutput>,
    text_renderer: Arc<std::sync::Mutex<super::StreamTextRenderer>>,
    watchdog_state: Option<Arc<std::sync::Mutex<WatchdogState>>>,
    section_tracker: Option<Arc<Mutex<SectionTracker>>>,
    timeout_config: TimeoutConfig,
) -> (Arc<AtomicBool>, thread::JoinHandle<()>) {
    const SILENCE_WINDOW: Duration = Duration::from_secs(30);
    const CADENCE: Duration = Duration::from_secs(30);

    let done = Arc::new(AtomicBool::new(false));
    let done_flag = Arc::clone(&done);
    let handle = thread::spawn(move || {
        let section_stream = section_tracker.map(|tracker| {
            super::super::section::SectionStream::with_tracker(stream_output.clone(), tracker)
        });
        let mut next_tick = Instant::now() + CADENCE;
        while !done_flag.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now >= next_tick {
                if let Ok(mut r) = text_renderer.lock() {
                    let mut writer = stream_output.stdout_writer();
                    r.flush_if_idle(&mut writer, SILENCE_WINDOW);
                }

                // Emit subagent idle diagnostics only when the unified
                // step_timeout rule is enabled. If the user disabled the
                // silence rule, the idle diagnostic is also suppressed.
                if timeout_config.step_timeout_enabled()
                    && let Some(ref state) = watchdog_state
                    && let Ok(mut guard) = state.lock()
                {
                    let lines = guard.diagnostic_lines(now, SILENCE_WINDOW);
                    drop(guard);
                    for line in lines {
                        let elapsed_text = format_duration(line.elapsed_since_start);
                        let diagnostic = format!(
                            " ⏳ Awaiting subagent: {} ({})",
                            line.display_name, elapsed_text
                        );
                        if let Some(ref ss) = section_stream {
                            ss.emit_stderr(Section::ToolUseAndEvents, &diagnostic);
                        } else {
                            stream_output.emit_stderr_line(&diagnostic);
                        }
                    }
                }

                next_tick += CADENCE;
                continue;
            }
            let sleep_for = next_tick
                .saturating_duration_since(now)
                .min(Duration::from_secs(1));
            thread::sleep(sleep_for);
        }
    });

    (done, handle)
}

/// Spawn the unified timeout watchdog ticker.
///
/// Evaluates the two timeout rules — wall-clock (`timeout`) and
/// stream-silence (`step_timeout`) — on the configured cadence
/// ([`TimeoutConfig::interval`], default 5 s). When a rule breaches,
/// renders an `AgentNative` error block to stderr and sends a
/// [`WatchdogTermination`] request to the exec wait loop so the child
/// process group receives SIGTERM with the configured `kill_grace`.
///
/// The ticker holds only weak conceptual coupling to the live sink: it
/// reads the same `WatchdogState` the sink updates (for diagnostic
/// enrichment of `step_timeout` breaches) and emits through the same
/// `StreamOutput` coordinator so stderr lines land on fresh rows even
/// when stdout is mid-line. A one-shot atomic `fired` guard inside
/// [`evaluate_timeout_tick`] prevents double-fire across the two rules.
pub(crate) fn spawn_timeout_watchdog_ticker(
    config: TimeoutConfig,
    started_at: Instant,
    watchdog_state: Arc<std::sync::Mutex<WatchdogState>>,
    watchdog_tx: std::sync::mpsc::Sender<WatchdogTermination>,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
) -> (Arc<AtomicBool>, thread::JoinHandle<()>) {
    let done = Arc::new(AtomicBool::new(false));
    let done_flag = Arc::clone(&done);
    let fired = Arc::new(AtomicBool::new(false));
    let cadence = config.interval;

    let handle = thread::spawn(move || {
        let mut next_tick = Instant::now() + cadence;
        while !done_flag.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now >= next_tick {
                match evaluate_timeout_tick(
                    &config,
                    now,
                    started_at,
                    &watchdog_state,
                    &live_metrics,
                    &fired,
                ) {
                    WatchdogTickResult::Ok => {}
                    WatchdogTickResult::Breach(ref term) => {
                        render_watchdog_error_to_stream(term, &stream_output);
                        let _ = watchdog_tx.send(term.clone());
                    }
                }
                next_tick += cadence;
                continue;
            }
            let sleep_for = next_tick
                .saturating_duration_since(now)
                .min(Duration::from_secs(1));
            thread::sleep(sleep_for);
        }
    });

    (done, handle)
}

/// Spawn the prompt-scoped timing monitor.
///
/// Emits the periodic timing header anchored on the prompt's start time
/// (`t=0`, `t=10m`, `t=20m`, …) plus two fire-once `Status::Warning`
/// messages when the user-configured `timeout_warn` / `step_timeout_warn`
/// thresholds cross. Only spawned for structured runs that carry a
/// prompt context (every `claudine compose` / `inline-compose` /
/// `sequence` run); wrapper passthrough runs skip the monitor entirely.
///
/// When both warnings cross on the same poll cycle, the step-scoped
/// warning is emitted first per the feature spec.
pub(crate) fn spawn_prompt_timing_monitor(
    started_at: Instant,
    started_at_wall: chrono::DateTime<chrono::Local>,
    prompt_timing: PromptTimingContext,
    hard_timeout: Option<Duration>,
    hard_step_timeout: Option<Duration>,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
) -> (Arc<AtomicBool>, thread::JoinHandle<()>) {
    let done = Arc::new(AtomicBool::new(false));
    let done_flag = Arc::clone(&done);
    let prompt_path_display = prompt_timing.absolute_path.display().to_string();

    let handle = thread::spawn(move || {
        let term = crate::log::terminal();

        emit_timing_header(
            HeaderKind::Zero,
            Duration::ZERO,
            &prompt_timing,
            &prompt_path_display,
            &term,
            &stream_output,
        );

        let mut next_header_tick = started_at + prompt_timing_mod::HEADER_CADENCE;
        let mut timeout_warn_fired = false;
        let poll_interval = Duration::from_secs(1);

        while !done_flag.load(Ordering::Relaxed) {
            let now = Instant::now();
            let elapsed = now.saturating_duration_since(started_at);

            if now >= next_header_tick {
                emit_timing_header(
                    HeaderKind::Tick,
                    elapsed,
                    &prompt_timing,
                    &prompt_path_display,
                    &term,
                    &stream_output,
                );
                next_header_tick += prompt_timing_mod::HEADER_CADENCE;
            }

            // Step-scoped warning must be emitted BEFORE the prompt-scoped
            // warning when both cross on the same poll cycle (feature spec
            // §2, "Ordering when both cross in the same cycle").
            if let Some(threshold) = prompt_timing.step_timeout_warn {
                maybe_emit_step_timeout_warn(
                    &prompt_timing,
                    &prompt_path_display,
                    &live_metrics,
                    threshold,
                    hard_step_timeout,
                    started_at_wall,
                    started_at,
                    now,
                    &term,
                    &stream_output,
                );
            }

            if let Some(threshold) = prompt_timing.timeout_warn
                && !timeout_warn_fired
                && elapsed >= threshold
            {
                emit_timeout_warn(
                    &prompt_timing,
                    &prompt_path_display,
                    elapsed,
                    threshold,
                    hard_timeout,
                    started_at_wall,
                    started_at,
                    &term,
                    &stream_output,
                );
                timeout_warn_fired = true;
            }

            let sleep_for = next_header_tick
                .saturating_duration_since(now)
                .min(poll_interval);
            thread::sleep(sleep_for);
        }
    });

    (done, handle)
}

fn emit_timing_header(
    kind: HeaderKind,
    elapsed: Duration,
    prompt_timing: &PromptTimingContext,
    prompt_path_display: &str,
    term: &Terminal,
    stream_output: &StreamOutput,
) {
    let body = prompt_timing_mod::render_header_prose(kind, elapsed, prompt_timing);
    let rendered = Prose::new(body).render(term);
    stream_output.emit_stderr_line(&rendered);
    tracing::info!(
        prompt_path = %prompt_path_display,
        elapsed_secs = elapsed.as_secs(),
        tick_kind = match kind {
            HeaderKind::Zero => "zero",
            HeaderKind::Tick => "tick",
        },
        "prompt timing header emitted",
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_timeout_warn(
    prompt_timing: &PromptTimingContext,
    prompt_path_display: &str,
    elapsed: Duration,
    threshold: Duration,
    hard_timeout: Option<Duration>,
    started_at_wall: chrono::DateTime<chrono::Local>,
    started_at: Instant,
    term: &Terminal,
    stream_output: &StreamOutput,
) {
    let hard_remaining = hard_timeout.and_then(|hard| {
        let deadline_instant = started_at + hard;
        let remaining = deadline_instant.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            None
        } else {
            let deadline_wall = started_at_wall + chrono::Duration::from_std(hard).ok()?;
            Some((remaining, deadline_wall))
        }
    });
    let body = prompt_timing_mod::render_timeout_warn_prose(elapsed, hard_remaining, prompt_timing);
    let rendered = Status::from_prose(body)
        .state(StatusState::Warning)
        .render(term);
    stream_output.emit_stderr_line(&rendered);
    tracing::info!(
        prompt_path = %prompt_path_display,
        elapsed_secs = elapsed.as_secs(),
        threshold_secs = threshold.as_secs(),
        remaining_secs = hard_remaining.map(|(r, _)| r.as_secs()).unwrap_or(0),
        hard_timeout_set = hard_timeout.is_some(),
        "timeout_warn emitted",
    );
}

#[allow(clippy::too_many_arguments)]
fn maybe_emit_step_timeout_warn(
    prompt_timing: &PromptTimingContext,
    prompt_path_display: &str,
    live_metrics: &LiveMetrics,
    threshold: Duration,
    hard_step_timeout: Option<Duration>,
    started_at_wall: chrono::DateTime<chrono::Local>,
    started_at: Instant,
    now: Instant,
    term: &Terminal,
    stream_output: &StreamOutput,
) {
    let (silence, last_event_at) = {
        let mut state = match live_metrics.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        if !claudine::stream::progress::should_warn_stall(&state, now, threshold) {
            return;
        }
        state.last_stall_warning_at = Some(now);
        let Some(last_event) = state.last_event_at else {
            return;
        };
        (now.saturating_duration_since(last_event), last_event)
    };

    let hard_remaining = hard_step_timeout.and_then(|hard| {
        let deadline_instant = last_event_at + hard;
        let remaining = deadline_instant.saturating_duration_since(now);
        if remaining.is_zero() {
            None
        } else {
            let last_event_wall = instant_to_local(started_at_wall, started_at, last_event_at);
            let deadline_wall = last_event_wall + chrono::Duration::from_std(hard).ok()?;
            Some((remaining, deadline_wall))
        }
    });

    let body =
        prompt_timing_mod::render_step_timeout_warn_prose(silence, hard_remaining, prompt_timing);
    let rendered = Status::from_prose(body)
        .state(StatusState::Warning)
        .render(term);
    stream_output.emit_stderr_line(&rendered);
    tracing::info!(
        prompt_path = %prompt_path_display,
        silence_secs = silence.as_secs(),
        threshold_secs = threshold.as_secs(),
        remaining_secs = hard_remaining.map(|(r, _)| r.as_secs()).unwrap_or(0),
        hard_step_timeout_set = hard_step_timeout.is_some(),
        "step_timeout_warn emitted",
    );
}

/// Translate a monotonic [`Instant`] into a wall-clock `DateTime<Local>`
/// using the prompt-scoped anchor (`started_at_wall` / `started_at`).
fn instant_to_local(
    anchor_wall: chrono::DateTime<chrono::Local>,
    anchor_instant: Instant,
    sample: Instant,
) -> chrono::DateTime<chrono::Local> {
    if sample >= anchor_instant {
        let delta = sample.duration_since(anchor_instant);
        match chrono::Duration::from_std(delta) {
            Ok(d) => anchor_wall + d,
            Err(_) => anchor_wall,
        }
    } else {
        let delta = anchor_instant.duration_since(sample);
        match chrono::Duration::from_std(delta) {
            Ok(d) => anchor_wall - d,
            Err(_) => anchor_wall,
        }
    }
}

/// Evaluate the unified `timeout` and `step_timeout` rules for one tick.
///
/// Rules:
///
/// 1. **Wall-clock (`timeout`).** If `config.timeout` is set and
///    `now - started_at >= timeout`, fire `Timeout`.
/// 2. **Stream-silence (`step_timeout`).** If `config.step_timeout` is set
///    AND at least one activity event has been observed
///    (`LiveMetrics.last_event_at.is_some()`) AND no tools or subagents
///    are currently in flight AND
///    `now - last_event_at >= step_timeout`, fire `StepTimeout` with
///    `outstanding = watchdog_state.active_subagents(now)` for diagnostic
///    enrichment.
///
/// The wall-clock rule is evaluated first so a deadline that elapses on
/// the same tick as a silence breach is reported as `timeout` rather than
/// `step_timeout`.
///
/// `fired` is an atomic flag that prevents double-fire across both rules;
/// once set to `true`, all subsequent evaluations return `Ok`.
pub(crate) fn evaluate_timeout_tick(
    config: &TimeoutConfig,
    now: Instant,
    started_at: Instant,
    watchdog_state: &Arc<std::sync::Mutex<WatchdogState>>,
    live_metrics: &LiveMetrics,
    fired: &AtomicBool,
) -> WatchdogTickResult {
    if fired.load(Ordering::SeqCst) {
        return WatchdogTickResult::Ok;
    }

    // Rule 1: wall-clock budget.
    if let Some(budget) = config.timeout {
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed >= budget {
            fired.store(true, Ordering::SeqCst);
            let message = format!(
                "wall-clock budget exceeded after {}",
                format_duration(elapsed),
            );
            return WatchdogTickResult::Breach(WatchdogTermination {
                reason: WatchdogTerminationReason::Timeout,
                message,
                stuck_subagents: Vec::new(),
            });
        }
    }

    // Rule 2: stream silence. Requires that at least one activity signal
    // has been observed past initial session start, matching the existing
    // first-event grace semantics. Activity is the more recent of the
    // structured-event clock (`last_event_at`) and the raw-byte clock
    // (`last_byte_at`), so providers whose stream is sparse enough that
    // structured events lag behind real progress (notably OpenCode) still
    // refresh the silence reference whenever bytes flow.
    //
    // Stuck-aware evaluation: a tool or subagent is "stuck" when its
    // `last_progress_at` is older than the step_timeout budget. The rule
    // is suppressed only when ALL in-flight items are active (none stuck).
    // If any item is stuck, the silence rule is allowed to fire so hung
    // work does not block termination indefinitely.
    if let Some(budget) = config.step_timeout {
        let (
            last_activity_at,
            last_event_at,
            last_byte_at,
            stuck_tools,
            stuck_subagents,
            any_active,
            any_stuck,
            provider_status_seen,
            step_in_flight,
            subagent_done_count,
        ) = match live_metrics.lock() {
            Ok(g) => {
                let stuck_tools: Vec<claudine::stream::progress::InFlightTool> =
                    g.stuck_tools(now, budget).into_iter().cloned().collect();
                let stuck_subagents: Vec<claudine::stream::progress::InFlightSubagent> = g
                    .stuck_subagents(now, budget)
                    .into_iter()
                    .cloned()
                    .collect();
                let any_stuck = !stuck_tools.is_empty() || !stuck_subagents.is_empty();
                let any_active = !g.in_flight.is_empty() || !g.in_flight_subagents.is_empty();
                let provider_status_seen = g.provider_status.is_some();
                let step_in_flight = g.step_in_flight;
                let subagent_done_count = g.subagent_done_count;
                (
                    g.last_activity_at(),
                    g.last_event_at,
                    g.last_byte_at,
                    stuck_tools,
                    stuck_subagents,
                    any_active,
                    any_stuck,
                    provider_status_seen,
                    step_in_flight,
                    subagent_done_count,
                )
            }
            Err(_) => return WatchdogTickResult::Ok,
        };
        // OpenCode-specific grace: this provider does not emit
        // `tool_start` or `task_started` events, so `in_flight` /
        // `in_flight_subagents` stay empty during legitimate work and
        // the stuck-aware suppression above has nothing to suppress
        // against. Two conditions suppress the silence rule:
        //
        // 1. `step_in_flight` is true — a step is currently open between
        //    `step_start` and the next `step_finish`. Mid-step silence is
        //    expected while subagents work.  However, if BOTH the
        //    structured-event clock and the raw-byte clock are stale
        //    beyond the budget, the breach still fires — the per-step
        //    grace must not override the byte-heartbeat backstop.
        // 2. `provider_status` is None — no `step_finish` has ever been
        //    observed, protecting slow startup / slow first turns.
        //
        // The wall-clock `timeout` rule above remains the unconditional
        // backstop in both cases.
        let both_clocks_stale = match (last_event_at, last_byte_at) {
            (Some(e), Some(b)) => {
                now.saturating_duration_since(e) >= budget
                    && now.saturating_duration_since(b) >= budget
            }
            _ => false,
        };
        if config.provider == Some(claudine::provider::Provider::OpenCode)
            && ((step_in_flight && !both_clocks_stale) || !provider_status_seen)
        {
            return WatchdogTickResult::Ok;
        }
        // Suppress step_timeout when all in-flight items are active (none stuck).
        if any_active && !any_stuck {
            return WatchdogTickResult::Ok;
        }
        if let Some(last) = last_activity_at {
            let silence = now.saturating_duration_since(last);
            if silence >= budget {
                let (outstanding, recent_subagents) = match watchdog_state.lock() {
                    Ok(g) => {
                        let outstanding = g.outstanding_at_breach(now);
                        let recent = g.recent_subagents.clone();
                        (outstanding, recent)
                    }
                    Err(_) => (Vec::new(), std::collections::VecDeque::new()),
                };
                let is_opencode = config.provider == Some(claudine::provider::Provider::OpenCode);
                fired.store(true, Ordering::SeqCst);
                let message = format_step_timeout_breach_message(
                    silence,
                    &outstanding,
                    &stuck_tools,
                    &stuck_subagents,
                    is_opencode.then_some(OpenCodeBreachContext {
                        subagent_done_count,
                        step_in_flight,
                        recent_subagents,
                        now,
                    }),
                );
                return WatchdogTickResult::Breach(WatchdogTermination {
                    reason: WatchdogTerminationReason::StepTimeout,
                    message,
                    stuck_subagents: outstanding,
                });
            }
        }
    }

    WatchdogTickResult::Ok
}

/// Extra diagnostic context used when enriching an OpenCode `step_timeout`
/// breach. Only populated when the provider is OpenCode and the standard
/// `outstanding_at_breach()` snapshot is empty.
#[derive(Debug, Clone)]
pub(crate) struct OpenCodeBreachContext {
    pub(crate) subagent_done_count: u32,
    pub(crate) step_in_flight: bool,
    pub(crate) recent_subagents: std::collections::VecDeque<super::subagent_watchdog::RecentSubagentInfo>,
    pub(crate) now: Instant,
}

/// Format the human-readable breach message for a `step_timeout` event.
///
/// When `outstanding` is non-empty the message enumerates the subagents
/// that were still in flight at the moment the silence rule fired
/// (id, optional name, elapsed since last progress) so the operator
/// knows which workers stalled. When empty, only the silence duration
/// is reported.
///
/// When `stuck_tools` or `stuck_subagents` is non-empty the message also
/// enumerates those stuck items.
///
/// For OpenCode, when `outstanding` is empty, `opencode_context` provides
/// enriched diagnostics: subagent completion count, whether a step was in
/// flight, and the most recent completed subagents from the ring-buffer.
pub(crate) fn format_step_timeout_breach_message(
    silence: Duration,
    outstanding: &[super::subagent_watchdog::ActiveSubagentSnapshot],
    stuck_tools: &[claudine::stream::progress::InFlightTool],
    stuck_subagents: &[claudine::stream::progress::InFlightSubagent],
    opencode_context: Option<OpenCodeBreachContext>,
) -> String {
    let silence_text = format_duration(silence);
    if outstanding.is_empty() && stuck_tools.is_empty() && stuck_subagents.is_empty() {
        let mut msg = format!("no stream activity for {silence_text}; terminating due to step_timeout");
        if let Some(ctx) = opencode_context {
            if ctx.subagent_done_count > 0 {
                let plural = if ctx.subagent_done_count == 1 { "subagent" } else { "subagents" };
                let count = ctx.subagent_done_count;
                msg = format!("{count} {plural} observed in this step. {msg}");
            }
            if let Some(last) = ctx.recent_subagents.front() {
                let since = format_duration(ctx.now.saturating_duration_since(last.completed_at));
                msg.push_str(&format!(" Last subagent completion {since} ago."));
            }
            if ctx.step_in_flight {
                // A breach with `step_in_flight=true` can only happen on the
                // byte-heartbeat backstop path (both event and byte clocks
                // stale) — the per-step grace otherwise suppresses the
                // silence rule while a step is open. Anchor the wording so
                // operators understand the child genuinely produced no
                // bytes during the silence window, not that a step closed
                // and then went silent.
                msg.push_str(&format!(
                    " A step boundary was still open and the child produced no bytes for {silence_text} — \
                     the parent agent may have been waiting on a parallel subagent that has not yet returned.",
                ));
            }
            if !ctx.recent_subagents.is_empty() {
                msg.push_str("\n\nRecent subagents:");
                for info in &ctx.recent_subagents {
                    let since = format_duration(ctx.now.saturating_duration_since(info.completed_at));
                    let desc = info.description.as_ref().or(info.name.as_ref());
                    let label = desc.map(|s| s.as_str()).unwrap_or("(unnamed)");
                    let status = info.status.as_deref().unwrap_or("unknown");
                    msg.push_str(&format!("\n  ← {label} ({since} ago, {status})"));
                }
            }
        }
        return msg;
    }

    let mut lines =
        format!("no stream activity for {silence_text}. The wrapped process was terminated.");

    if !stuck_tools.is_empty() {
        let count = stuck_tools.len();
        let plural = if count == 1 { "tool" } else { "tools" };
        lines.push_str(&format!(
            " {count} {plural} were stuck when the timeout fired:\n"
        ));
        for tool in stuck_tools {
            let name = tool.name.as_deref().unwrap_or("(unnamed)");
            lines.push_str(&format!("  • \"{name}\"\n"));
        }
    }

    if !stuck_subagents.is_empty() {
        let count = stuck_subagents.len();
        let plural = if count == 1 { "subagent" } else { "subagents" };
        lines.push_str(&format!(
            " {count} {plural} were stuck when the timeout fired:\n"
        ));
        for subagent in stuck_subagents {
            let name = subagent.name.as_deref().unwrap_or("(unnamed)");
            lines.push_str(&format!("  • \"{name}\"\n"));
        }
    }

    if !outstanding.is_empty() {
        let count = outstanding.len();
        let plural = if count == 1 { "subagent" } else { "subagents" };
        lines.push_str(&format!(
            " {count} {plural} were still outstanding when the timeout fired:\n"
        ));
        for snap in outstanding {
            let idle = format_duration(snap.elapsed_since_progress);
            let name = snap.name.as_deref().unwrap_or("(unnamed)");
            lines.push_str(&format!("  • {} \"{name}\" (idle {idle})\n", snap.id));
        }
    }

    lines
}

/// Format a duration in a human-readable way (e.g. "3m 0s").
pub(crate) fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3_600, (secs % 3_600) / 60, secs % 60)
    }
}

/// Render a watchdog breach as an `AgentNative` error block on stderr.
///
/// This is the standalone rendering path used by the watchdog ticker
/// thread so it can surface the error before the child is killed,
/// without needing access to the `LiveSemanticSink` section tracker.
pub(crate) fn render_watchdog_error_to_stream(
    termination: &WatchdogTermination,
    stream_output: &StreamOutput,
) {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::components::status::StatusState;
    use biscuit_terminal::prelude::StatusBlock;
    use biscuit_terminal::utils::color::{Color, Tailwind};
    use biscuit_terminal::utils::layout::{Margin, WordWrap};

    let term = crate::log::terminal();
    let border_color = Color::Tailwind(Tailwind::Red700);
    let body_text = escape_prose(&termination.message);
    let body = format!("<red><b>Agent Error</b></red>\n{body_text}");
    let prose = Prose::new(body).with_word_wrap(WordWrap::WrapProse(None, None));
    let block = StatusBlock::new(StatusState::Error)
        .body(prose)
        .border_color(border_color)
        .left_margin(Margin::Chars(0))
        .right_margin(Margin::Chars(0));
    let rendered = block.render(&term);
    for line in rendered.lines() {
        stream_output.emit_stderr_line(line);
    }
}

/// Escape user-controlled text so it can be safely interpolated into
/// biscuit-terminal prose markup.
fn escape_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[allow(dead_code)]
    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn evaluate_timeout_tick_ok_when_no_rule_enabled() {
        let config = TimeoutConfig::default();
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let now_t = Instant::now();
        let result = evaluate_timeout_tick(&config, now_t, now_t, &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok);
    }

    #[test]
    fn evaluate_timeout_tick_wall_clock_breach() {
        let config = TimeoutConfig {
            timeout: Some(Duration::from_secs(5)),
            step_timeout: None,
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let started_at = Instant::now() - Duration::from_secs(10);

        let result = evaluate_timeout_tick(
            &config,
            Instant::now(),
            started_at,
            &state,
            &metrics,
            &fired,
        );
        assert!(
            matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::Timeout),
            "expected wall-clock Timeout breach, got: {result:?}"
        );
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_breach_with_outstanding_subagents() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut s = state.lock().unwrap();
            s.subagent_started("sa1".into(), Some("Researcher".into()), t0);
        }
        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert_eq!(w.stuck_subagents.len(), 1);
                assert_eq!(w.stuck_subagents[0].id, "sa1");
                assert!(w.message.contains("Researcher"), "got: {}", w.message);
            }
            other => panic!("expected StepTimeout breach, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_breach_without_subagents() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(w.stuck_subagents.is_empty());
                assert!(w.message.contains("step_timeout"), "got: {}", w.message);
            }
            other => panic!("expected StepTimeout breach, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_does_not_fire_silence_without_first_event() {
        // Spec: silence rule requires at least one observed activity event
        // (matches `last_event_at: Option<Instant>` first-event grace).
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let started_at = Instant::now() - Duration::from_secs(60);

        let result = evaluate_timeout_tick(
            &config,
            Instant::now(),
            started_at,
            &state,
            &metrics,
            &fired,
        );
        assert_eq!(result, WatchdogTickResult::Ok);
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_wall_clock_wins_over_silence_on_same_tick() {
        let config = TimeoutConfig {
            timeout: Some(Duration::from_secs(5)),
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert!(
            matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::Timeout),
            "wall-clock must win; got: {result:?}"
        );
    }

    #[test]
    fn evaluate_timeout_tick_one_shot_guard() {
        let config = TimeoutConfig {
            timeout: Some(Duration::from_secs(1)),
            step_timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(true); // already fired
        let started_at = Instant::now() - Duration::from_secs(60);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(started_at);
        }

        let result = evaluate_timeout_tick(
            &config,
            Instant::now(),
            started_at,
            &state,
            &metrics,
            &fired,
        );
        assert_eq!(result, WatchdogTickResult::Ok);
    }

    #[test]
    fn evaluate_timeout_tick_silence_suppressed_by_in_flight_tool() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight.insert(
                "tool-1".into(),
                claudine::stream::progress::InFlightTool {
                    name: Some("Task".into()),
                    started_at: t0,
                    last_progress_at: t0,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        // Stuck tool (no progress for 10s >= 5s budget) does NOT suppress step_timeout.
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(
                    w.message.contains("Task"),
                    "stuck tool should be named: {}",
                    w.message
                );
            }
            other => panic!("expected StepTimeout breach for stuck tool, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_suppressed_by_in_flight_subagent() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight_subagents.insert(
                "sa-1".into(),
                claudine::stream::progress::InFlightSubagent {
                    name: Some("rust-developer".into()),
                    started_at: t0,
                    last_progress_at: t0,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        // Stuck subagent (no progress for 10s >= 5s budget) does NOT suppress step_timeout.
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!("expected StepTimeout breach for stuck subagent, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_fires_after_in_flight_cleared() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);
        let fresh = Instant::now();

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight.insert(
                "tool-1".into(),
                claudine::stream::progress::InFlightTool {
                    name: Some("Task".into()),
                    started_at: t0,
                    last_progress_at: fresh,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "must not fire while active tool is in-flight"
        );

        {
            let mut m = metrics.lock().unwrap();
            m.in_flight.clear();
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!("expected StepTimeout breach after in-flight cleared, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_suppressed_when_tool_is_active() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);
        let fresh = Instant::now();

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight.insert(
                "tool-1".into(),
                claudine::stream::progress::InFlightTool {
                    name: Some("Task".into()),
                    started_at: t0,
                    last_progress_at: fresh,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "active tool must suppress step_timeout"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_suppressed_when_subagent_is_active() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);
        let fresh = Instant::now();

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight_subagents.insert(
                "sa-1".into(),
                claudine::stream::progress::InFlightSubagent {
                    name: Some("rust-developer".into()),
                    started_at: t0,
                    last_progress_at: fresh,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "active subagent must suppress step_timeout"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_fires_when_tool_is_stuck() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight.insert(
                "tool-1".into(),
                claudine::stream::progress::InFlightTool {
                    name: Some("Task".into()),
                    started_at: t0,
                    last_progress_at: t0,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(w.message.contains("Task"));
            }
            other => panic!("expected StepTimeout breach for stuck tool, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_fires_when_subagent_is_stuck() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight_subagents.insert(
                "sa-1".into(),
                claudine::stream::progress::InFlightSubagent {
                    name: Some("rust-developer".into()),
                    started_at: t0,
                    last_progress_at: t0,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(w.message.contains("rust-developer"));
            }
            other => panic!("expected StepTimeout breach for stuck subagent, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_mixed_active_and_stuck_fires() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);
        let fresh = Instant::now();

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight.insert(
                "tool-active".into(),
                claudine::stream::progress::InFlightTool {
                    name: Some("ActiveTask".into()),
                    started_at: t0,
                    last_progress_at: fresh,
                },
            );
            m.in_flight.insert(
                "tool-stuck".into(),
                claudine::stream::progress::InFlightTool {
                    name: Some("StuckTask".into()),
                    started_at: t0,
                    last_progress_at: t0,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(w.message.contains("StuckTask"));
            }
            other => {
                panic!("expected StepTimeout breach when mix of active and stuck, got: {other:?}")
            }
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_opencode_grace_suppresses_silence_until_step_finish() {
        // OpenCode + step_timeout + no `step_finish` boundary observed
        // (provider_status is None) + silence beyond budget → suppressed.
        // This is the regression guard for the
        // 2026-05-10-opencode-timeout-regression fix.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            // provider_status intentionally left at None.
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "OpenCode without an observed step_finish must suppress step_timeout"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_opencode_grace_releases_after_step_finish() {
        // OpenCode + step_timeout + `step_finish` boundary observed
        // (provider_status is Some) + silence beyond budget → fires.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.provider_status = Some("stop".into());
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!(
                "expected StepTimeout breach once a step_finish boundary has been observed, \
                 got: {other:?}"
            ),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_opencode_grace_does_not_block_wall_clock() {
        // OpenCode + wall-clock timeout breach must still fire even when
        // provider_status is None. The OpenCode grace only suppresses the
        // step_timeout silence rule, never the wall-clock backstop.
        let config = TimeoutConfig {
            timeout: Some(Duration::from_secs(5)),
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let started_at = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(started_at);
            // provider_status intentionally left at None.
        }

        let result = evaluate_timeout_tick(
            &config,
            Instant::now(),
            started_at,
            &state,
            &metrics,
            &fired,
        );
        assert!(
            matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::Timeout),
            "wall-clock timeout must still fire on OpenCode regardless of provider_status; got: {result:?}"
        );
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_grace_does_not_apply_to_other_providers() {
        // Claude (or any non-OpenCode provider) does not get the
        // provider_status grace: silence beyond budget with no
        // step_finish observed still fires step_timeout.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::Claude),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            // provider_status intentionally left at None.
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!(
                "Claude provider must not get OpenCode-specific grace; expected StepTimeout, got: {other:?}"
            ),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_grace_does_not_apply_when_provider_unset() {
        // No provider plumbed (None) → no grace; step_timeout fires
        // normally so wrapper passthrough paths don't accidentally inherit
        // OpenCode-specific behaviour.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: None,
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert!(
            matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::StepTimeout),
            "provider=None must not enable OpenCode grace; got: {result:?}"
        );
    }

    #[test]
    fn format_step_timeout_breach_message_no_outstanding() {
        let msg = format_step_timeout_breach_message(Duration::from_secs(180), &[], &[], &[], None);
        assert!(msg.contains("3m 0s"));
        assert!(msg.contains("step_timeout"));
        assert!(!msg.contains("subagent"));
        assert!(!msg.contains("tool"));
    }

    #[test]
    fn format_step_timeout_breach_message_lists_outstanding() {
        let snap = crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot {
            id: "ses_a".into(),
            name: Some("Commit work".into()),
            started_at: Instant::now(),
            last_progress_at: Instant::now(),
            elapsed_since_start: Duration::from_secs(900),
            elapsed_since_progress: Duration::from_secs(900),
        };
        let msg = format_step_timeout_breach_message(
            Duration::from_secs(1800),
            std::slice::from_ref(&snap),
            &[],
            &[],
            None,
        );
        assert!(msg.contains("30m 0s"));
        assert!(msg.contains("1 subagent"));
        assert!(msg.contains("ses_a"));
        assert!(msg.contains("Commit work"));
        assert!(msg.contains("idle 15m 0s"));
    }

    #[test]
    fn format_step_timeout_breach_message_lists_stuck_tools() {
        let tool = claudine::stream::progress::InFlightTool {
            name: Some("Bash".into()),
            started_at: Instant::now() - Duration::from_secs(600),
            last_progress_at: Instant::now() - Duration::from_secs(600),
        };
        let msg = format_step_timeout_breach_message(
            Duration::from_secs(180),
            &[],
            std::slice::from_ref(&tool),
            &[],
            None,
        );
        assert!(msg.contains("3m 0s"));
        assert!(msg.contains("1 tool"));
        assert!(msg.contains("Bash"));
    }

    // ---------------------------------------------------------------------
    // Byte-heartbeat tests (Phase 2 of 2026-05-10-opencode-timeout-regression)
    // ---------------------------------------------------------------------
    //
    // The silence rule reads `LiveMetricsState::last_activity_at()`, which
    // returns the more recent of `last_event_at` and `last_byte_at`. These
    // tests pin the contract that raw byte activity protects against false
    // silence kills on sparse-stream providers, while a true zero-byte hang
    // still fires.

    #[test]
    fn evaluate_timeout_tick_silence_suppressed_by_recent_byte_activity() {
        // Stale `last_event_at`, but `last_byte_at` is fresh — bytes are
        // still flowing from the child even though the structured parser
        // has not produced a new SemanticEvent. The silence rule must
        // pick the byte clock as the activity reference and suppress.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let now_t = Instant::now();
        let stale = now_t - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(stale);
            m.last_byte_at = Some(now_t); // fresh byte activity
        }

        let result = evaluate_timeout_tick(&config, now_t, stale, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "recent byte activity must suppress step_timeout, got: {result:?}"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_fires_when_neither_clock_recent() {
        // Both `last_event_at` and `last_byte_at` are stale beyond budget,
        // and no in-flight items — the byte heartbeat must NOT mask a
        // genuine zero-byte hang.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let stale = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(stale);
            m.last_byte_at = Some(stale);
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), stale, &state, &metrics, &fired);
        assert!(
            matches!(
                result,
                WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::StepTimeout
            ),
            "stale byte clock must allow step_timeout to fire; got: {result:?}"
        );
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_suppressed_when_only_byte_clock_set() {
        // Defensive case: no structured events ever fired, but bytes are
        // flowing (a provider that emits only raw text on stdout). The
        // first-event grace on `last_event_at` is satisfied transitively
        // by `last_byte_at` via `last_activity_at()`.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let now_t = Instant::now();

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = None;
            m.last_byte_at = Some(now_t);
        }

        let result = evaluate_timeout_tick(&config, now_t, now_t, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "byte clock alone must satisfy first-event grace and suppress; got: {result:?}"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    // ---------------------------------------------------------------------
    // Claude-shaped regression guard for the in-flight gate.
    // ---------------------------------------------------------------------
    //
    // The OpenCode `provider_status` grace must not change behaviour on
    // rich-stream providers. With Provider::Claude, an active in-flight
    // subagent must continue to suppress the silence rule via the
    // existing stuck-aware gate (no provider grace required).

    #[test]
    fn evaluate_timeout_tick_claude_in_flight_gate_still_suppresses() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::Claude),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(30);
        let fresh = Instant::now();

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight_subagents.insert(
                "sa-1".into(),
                claudine::stream::progress::InFlightSubagent {
                    name: Some("rust-developer".into()),
                    started_at: t0,
                    last_progress_at: fresh,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "Claude active subagent must suppress step_timeout via in-flight gate; got: {result:?}"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_claude_in_flight_gate_fires_when_stuck() {
        // Counter-test for the previous: when a Claude subagent is stuck
        // (no progress for longer than budget) the in-flight gate releases
        // and step_timeout fires normally, just as on any other provider.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::Claude),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(30);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight_subagents.insert(
                "sa-1".into(),
                claudine::stream::progress::InFlightSubagent {
                    name: Some("rust-developer".into()),
                    started_at: t0,
                    last_progress_at: t0,
                },
            );
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(w.message.contains("rust-developer"));
            }
            other => panic!(
                "Claude stuck subagent must release the in-flight gate; expected StepTimeout, got: {other:?}"
            ),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    // ------------------------------------------------------------------
    // Phase 4 — OpenCode breach-diagnostic enrichment (unit tests)
    // ------------------------------------------------------------------

    #[test]
    fn format_step_timeout_breach_message_opencode_names_subagent_count() {
        let mut recent = std::collections::VecDeque::new();
        recent.push_back(crate::commands::wrap::exec::subagent_watchdog::RecentSubagentInfo {
            id: "sa-1".into(),
            name: Some("Alpha".into()),
            description: Some("do alpha".into()),
            completed_at: Instant::now(),
            status: Some("success".into()),
        });
        let ctx = OpenCodeBreachContext {
            subagent_done_count: 3,
            step_in_flight: true,
            recent_subagents: recent,
            now: Instant::now(),
        };
        let msg = format_step_timeout_breach_message(
            Duration::from_secs(180),
            &[],
            &[],
            &[],
            Some(ctx),
        );
        assert!(msg.contains("3 subagents observed"), "got: {msg}");
        assert!(
            msg.contains("step boundary was still open"),
            "got: {msg}"
        );
    }

    #[test]
    fn format_step_timeout_breach_message_opencode_lists_recent_descriptions() {
        let mut recent = std::collections::VecDeque::new();
        // Simulate newest-first ring buffer: push_front so Desc-4 (newest)
        // ends up at the front of the deque.
        for i in 0..5 {
            recent.push_front(crate::commands::wrap::exec::subagent_watchdog::RecentSubagentInfo {
                id: format!("sa-{i}"),
                name: Some(format!("Name-{i}")),
                description: Some(format!("Desc-{i}")),
                completed_at: Instant::now() - Duration::from_secs((5 - i) as u64 * 60),
                status: Some("success".into()),
            });
        }
        let ctx = OpenCodeBreachContext {
            subagent_done_count: 5,
            step_in_flight: false,
            recent_subagents: recent,
            now: Instant::now(),
        };
        let msg = format_step_timeout_breach_message(
            Duration::from_secs(180),
            &[],
            &[],
            &[],
            Some(ctx),
        );
        assert!(msg.contains("5 subagents observed"), "got: {msg}");
        assert!(msg.contains("Recent subagents:"), "got: {msg}");
        // Newest first: Desc-4 should appear before Desc-3
        let idx_4 = msg.find("Desc-4").expect("Desc-4 should be present");
        let idx_3 = msg.find("Desc-3").expect("Desc-3 should be present");
        assert!(idx_4 < idx_3, "newest-first order required: {msg}");
    }

    #[test]
    fn format_step_timeout_breach_message_opencode_no_recent_subagents() {
        let ctx = OpenCodeBreachContext {
            subagent_done_count: 0,
            step_in_flight: true,
            recent_subagents: std::collections::VecDeque::new(),
            now: Instant::now(),
        };
        let msg = format_step_timeout_breach_message(
            Duration::from_secs(180),
            &[],
            &[],
            &[],
            Some(ctx),
        );
        assert!(msg.contains("step_timeout"), "got: {msg}");
        assert!(
            !msg.contains("subagents observed"),
            "count 0 must not render: {msg}"
        );
        assert!(
            msg.contains("step boundary was still open"),
            "step_in_flight hint must appear: {msg}"
        );
    }

    // ------------------------------------------------------------------
    // Phase 5 — per-step grace + subagent lifecycle synthesis tests
    // ------------------------------------------------------------------

    #[test]
    fn opencode_step_in_flight_suppresses_silence() {
        // OpenCode + step_timeout + step_finish observed (provider_status is
        // Some) BUT step_in_flight is true → silence beyond budget must be
        // suppressed. Once step_in_flight is cleared, the breach should fire.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.provider_status = Some("stop".into());
            m.step_in_flight = true;
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "OpenCode with step_in_flight=true must suppress step_timeout even after step_finish"
        );
        assert!(!fired.load(Ordering::SeqCst));

        // Now clear step_in_flight (emulate step_finish) and re-evaluate
        {
            let mut m = metrics.lock().unwrap();
            m.step_in_flight = false;
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!(
                "expected StepTimeout breach once step_in_flight is cleared, got: {other:?}"
            ),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn opencode_step_in_flight_resets_per_step() {
        // The per-step grace must reset for each new step_start/step_finish
        // cycle, not be a one-shot guard. This test drives `observe_event`
        // through a sequence of real `Info{step_phase=start|finish}` events
        // on a SINGLE `LiveMetricsState` instance so the toggling logic is
        // exercised end-to-end — not just the predicate behavior against
        // a hand-set field.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let session_start = Instant::now() - Duration::from_secs(60);

        // --- Cycle 1: step_start fed through observe_event ---
        {
            let mut m = metrics.lock().unwrap();
            m.observe_event(
                &claudine::stream::semantic::SemanticEvent::Info {
                    message: "step_start".into(),
                    extra: serde_json::json!({"step_phase": "start"}),
                },
                Instant::now() - Duration::from_secs(30),
            );
            assert!(
                m.step_in_flight,
                "observe_event must set step_in_flight=true on step_start"
            );
        }
        let result =
            evaluate_timeout_tick(&config, Instant::now(), session_start, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "first step_start must suppress step_timeout (silence stale, step_in_flight=true)"
        );
        assert!(!fired.load(Ordering::SeqCst));

        // --- Cycle 1: step_finish fed through observe_event ---
        {
            let mut m = metrics.lock().unwrap();
            m.observe_event(
                &claudine::stream::semantic::SemanticEvent::Info {
                    message: "step_finish".into(),
                    extra: serde_json::json!({"step_phase": "finish", "reason": "tool-calls"}),
                },
                Instant::now() - Duration::from_secs(20),
            );
            assert!(
                !m.step_in_flight,
                "observe_event must clear step_in_flight on step_finish"
            );
            assert_eq!(m.provider_status.as_deref(), Some("tool-calls"));
        }
        let result =
            evaluate_timeout_tick(&config, Instant::now(), session_start, &state, &metrics, &fired);
        assert!(
            matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::StepTimeout),
            "step_finish must release suppression and allow step_timeout to fire; got: {result:?}"
        );
        assert!(fired.load(Ordering::SeqCst));

        // --- Cycle 2: step_start fed through observe_event again (per-step reset) ---
        fired.store(false, Ordering::SeqCst);
        {
            let mut m = metrics.lock().unwrap();
            m.observe_event(
                &claudine::stream::semantic::SemanticEvent::Info {
                    message: "step_start".into(),
                    extra: serde_json::json!({"step_phase": "start"}),
                },
                Instant::now() - Duration::from_secs(10),
            );
            assert!(
                m.step_in_flight,
                "second step_start must toggle step_in_flight back to true (per-step reset)"
            );
            // provider_status remains Some from the first step_finish — proves
            // the grace does NOT rely on the cold-start (`provider_status=None`)
            // protection; it must come from `step_in_flight` alone.
            assert_eq!(m.provider_status.as_deref(), Some("tool-calls"));
        }
        let result =
            evaluate_timeout_tick(&config, Instant::now(), session_start, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "second step_start must suppress step_timeout again (per-step grace reset)"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn opencode_byte_heartbeat_still_catches_zero_byte_hang() {
        // Even when step_in_flight is true, if BOTH the structured-event
        // clock and the raw-byte clock are stale beyond step_timeout, the
        // breach must fire. The per-step grace must not override the byte
        // heartbeat backstop.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let stale = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(stale);
            m.last_byte_at = Some(stale);
            m.step_in_flight = true;
            m.provider_status = Some("stop".into());
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), stale, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!(
                "expected StepTimeout breach when both byte and event clocks are stale, even with step_in_flight; got: {other:?}"
            ),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn opencode_cold_start_suppresses_even_when_both_clocks_stale() {
        // Cold-start counter-test: `provider_status` is None (no `step_finish`
        // has ever been observed) and BOTH the structured-event clock and
        // the raw-byte clock are stale past `step_timeout`. The documented
        // OpenCode-grace contract says the breach must NOT fire — cold-start
        // protection is unconditional on the byte-heartbeat backstop because
        // the wrapper is still inside the slow-first-turn window.
        //
        // Pair with `opencode_byte_heartbeat_still_catches_zero_byte_hang`
        // which covers the post-cold-start `step_in_flight=true` arm where
        // the byte-heartbeat DOES escape suppression.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let stale = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(stale);
            m.last_byte_at = Some(stale);
            // provider_status stays None — no step_finish has been observed.
            // step_in_flight stays false — no step_start either.
            assert!(m.provider_status.is_none());
            assert!(!m.step_in_flight);
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), stale, &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "cold-start path (provider_status=None) must suppress step_timeout even when both clocks are stale; got: {result:?}"
        );
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn opencode_breach_diagnostic_names_subagent_count() {
        // When 3 synthesized Task completions have been observed and then a
        // silence breach fires, the rendered message must contain the count.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut s = state.lock().unwrap();
            for i in 0..3 {
                s.subagent_stopped(
                    &format!("sa-{i}"),
                    Some(format!("Task {i}")),
                    Some(format!("Description {i}")),
                    Some("success".into()),
                    t0,
                );
            }
        }

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.provider_status = Some("stop".into());
            m.step_in_flight = false;
            m.subagent_done_count = 3;
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(
                    w.message.contains("3 subagents observed"),
                    "breach message should name subagent count: {}",
                    w.message
                );
            }
            other => panic!(
                "expected StepTimeout breach with subagent count diagnostic; got: {other:?}"
            ),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn opencode_breach_diagnostic_lists_recent_subagent_descriptions() {
        // When 5 synthesized Task completions with distinct descriptions have
        // been observed, the breach message must list all 5 in newest-first
        // order.
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            provider: Some(claudine::provider::Provider::OpenCode),
            ..Default::default()
        };
        let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut s = state.lock().unwrap();
            for i in 0..5 {
                s.subagent_stopped(
                    &format!("sa-{i}"),
                    Some(format!("Name {i}")),
                    Some(format!("Desc-{i}")),
                    Some("success".into()),
                    t0 + Duration::from_secs(i as u64),
                );
            }
        }

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.provider_status = Some("stop".into());
            m.step_in_flight = false;
            m.subagent_done_count = 5;
        }

        let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
                assert!(
                    w.message.contains("5 subagents observed"),
                    "breach message should name subagent count: {}",
                    w.message
                );
                assert!(
                    w.message.contains("Recent subagents:"),
                    "breach message should list recent subagents: {}",
                    w.message
                );
                // Newest first: Desc-4 should appear before Desc-3
                let idx_4 = w.message.find("Desc-4").expect("Desc-4 should be present");
                let idx_3 = w.message.find("Desc-3").expect("Desc-3 should be present");
                assert!(idx_4 < idx_3, "newest-first order required: {}", w.message);
            }
            other => panic!(
                "expected StepTimeout breach with recent subagent descriptions; got: {other:?}"
            ),
        }
        assert!(fired.load(Ordering::SeqCst));
    }
}
