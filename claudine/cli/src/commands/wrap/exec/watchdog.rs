use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
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
                        let elapsed_text =
                            format_duration(line.elapsed_since_start);
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

    // Rule 2: stream silence. Requires that at least one activity event
    // has been observed past initial session start, matching the existing
    // `last_event_at: Option<Instant>` first-event grace semantics. When
    // in-flight tools or subagents exist the rule is suppressed: a
    // long-running Task/subagent call produces parent-stream silence by
    // design while the child works. The wall-clock `timeout` rule serves as
    // the backstop for truly stuck tool calls.
    if let Some(budget) = config.step_timeout {
        let guarded = match live_metrics.lock() {
            Ok(g) => {
                let has_in_flight = !g.in_flight.is_empty() || !g.in_flight_subagents.is_empty();
                (g.last_event_at, has_in_flight)
            }
            Err(_) => return WatchdogTickResult::Ok,
        };
        let (last_event_at, has_in_flight) = guarded;
        if has_in_flight {
            return WatchdogTickResult::Ok;
        }
        if let Some(last) = last_event_at {
            let silence = now.saturating_duration_since(last);
            if silence >= budget {
                let outstanding = match watchdog_state.lock() {
                    Ok(g) => g.outstanding_at_breach(now),
                    Err(_) => Vec::new(),
                };
                fired.store(true, Ordering::SeqCst);
                let message = format_step_timeout_breach_message(silence, &outstanding);
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

/// Format the human-readable breach message for a `step_timeout` event.
///
/// When `outstanding` is non-empty the message enumerates the subagents
/// that were still in flight at the moment the silence rule fired
/// (id, optional name, elapsed since last progress) so the operator
/// knows which workers stalled. When empty, only the silence duration
/// is reported.
pub(crate) fn format_step_timeout_breach_message(
    silence: Duration,
    outstanding: &[super::subagent_watchdog::ActiveSubagentSnapshot],
) -> String {
    let silence_text = format_duration(silence);
    if outstanding.is_empty() {
        return format!("no stream activity for {silence_text}; terminating due to step_timeout");
    }

    let count = outstanding.len();
    let plural = if count == 1 { "subagent" } else { "subagents" };
    let mut lines = format!(
        "no stream activity for {silence_text}. The wrapped process was terminated. {count} {plural} were still outstanding when the timeout fired:\n"
    );
    for snap in outstanding {
        let idle = format_duration(snap.elapsed_since_progress);
        let name = snap.name.as_deref().unwrap_or("(unnamed)");
        lines.push_str(&format!("  • {} \"{name}\" (idle {idle})\n", snap.id));
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

        let result =
            evaluate_timeout_tick(&config, Instant::now(), started_at, &state, &metrics, &fired);
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

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
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

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
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

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
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
                },
            );
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok);
        assert!(!fired.load(Ordering::SeqCst));
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
                },
            );
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok);
        assert!(!fired.load(Ordering::SeqCst));
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

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
            m.in_flight.insert(
                "tool-1".into(),
                claudine::stream::progress::InFlightTool {
                    name: Some("Task".into()),
                    started_at: t0,
                },
            );
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok, "must not fire while tool in-flight");

        {
            let mut m = metrics.lock().unwrap();
            m.in_flight.clear();
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!("expected StepTimeout breach after in-flight cleared, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn format_step_timeout_breach_message_no_outstanding() {
        let msg = format_step_timeout_breach_message(Duration::from_secs(180), &[]);
        assert!(msg.contains("3m 0s"));
        assert!(msg.contains("step_timeout"));
        assert!(!msg.contains("subagent"));
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
        let msg =
            format_step_timeout_breach_message(Duration::from_secs(1800), std::slice::from_ref(&snap));
        assert!(msg.contains("30m 0s"));
        assert!(msg.contains("1 subagent"));
        assert!(msg.contains("ses_a"));
        assert!(msg.contains("Commit work"));
        assert!(msg.contains("idle 15m 0s"));
    }
}
