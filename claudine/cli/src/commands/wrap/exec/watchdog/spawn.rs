//! Monitor-spawner threads for the watchdog subsystem.
//!
//! Three ticker threads live here: the idle-flush ticker (drains buffered
//! markdown + subagent idle diagnostics), the timeout watchdog ticker (drives
//! [`evaluate_timeout_tick`] and routes breaches to termination), and the
//! prompt-scoped timing monitor (periodic timing header + the fire-once
//! `timeout_warn` / `step_timeout_warn` messages).

use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use claudine::stream::progress::LiveMetrics;
use claudine::stream::prompt_timing as prompt_timing_mod;
use claudine::stream::prompt_timing::{HeaderKind, PromptTimingContext};

use super::super::subagent_watchdog::WatchdogState;
use super::super::timeouts::TimeoutConfig;
use super::super::TickerCancel;
use claudine::render::{AssistantStream, StreamRenderable};
use super::super::super::section::{Section, SectionStream, SectionTracker};
use super::super::super::stream_io::StreamOutput;
use super::breach::{format_duration, render_watchdog_error_to_stream};
use super::evaluate::{WatchdogTickResult, evaluate_timeout_tick};

/// Spawn a dedicated ticker that runs `AssistantStream`'s idle flush
/// (`StreamRenderable::flush_idle`) every 30 seconds.
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
    text_renderer: Arc<std::sync::Mutex<AssistantStream>>,
    watchdog_state: Option<Arc<std::sync::Mutex<WatchdogState>>>,
    section_tracker: Option<Arc<Mutex<SectionTracker>>>,
    timeout_config: TimeoutConfig,
    framed_writer: Option<Arc<std::sync::Mutex<claudine::render::TaskFrameWriter>>>,
) -> (TickerCancel, thread::JoinHandle<()>) {
    const SILENCE_WINDOW: Duration = Duration::from_secs(30);
    const CADENCE: Duration = Duration::from_secs(30);

    let cancel = TickerCancel::new();
    let cancel_flag = cancel.clone();
    let handle = thread::spawn(move || {
        let section_stream = section_tracker
            .map(|tracker| SectionStream::with_tracker(stream_output.clone(), tracker));
        let mut next_tick = Instant::now() + CADENCE;
        while !cancel_flag.is_cancelled() {
            let now = Instant::now();
            if now >= next_tick {
                flush_idle_to_stream(
                    &text_renderer,
                    &framed_writer,
                    &stream_output,
                    SILENCE_WINDOW,
                );

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
            if cancel_flag.sleep(sleep_for) {
                break;
            }
        }
    });

    (cancel, handle)
}

/// Drain one idle flush to whichever stdout path is live.
///
/// The framed arm exists because a flush is ordinary task *data* arriving late:
/// a long-running task that holds a partial Markdown block past the idle window
/// would otherwise have that block appear with no bar, breaking attribution
/// mid-stream (spec → *Reporting Concurrency*).
pub(crate) fn flush_idle_to_stream(
    text_renderer: &Arc<std::sync::Mutex<AssistantStream>>,
    framed: &Option<Arc<std::sync::Mutex<claudine::render::TaskFrameWriter>>>,
    stream_output: &Arc<StreamOutput>,
    window: Duration,
) {
    let Ok(mut renderer) = text_renderer.lock() else {
        return;
    };
    let frames = renderer.flush_idle(window);
    drop(renderer);
    match framed {
        Some(framed) => {
            if let Ok(mut framed) = framed.lock() {
                for frame in frames {
                    framed.write(&frame);
                }
                // The stream is not over, but the point of an idle flush is to
                // surface text now — a fragment still held would defeat it.
                framed.flush();
            }
        }
        None => {
            let mut writer = stream_output.stdout_writer();
            for frame in frames {
                let _ = writer.write_all(frame.as_bytes());
            }
            let _ = writer.flush();
        }
    }
}

/// Spawn the unified timeout watchdog ticker.
///
/// Evaluates the two timeout rules — wall-clock (`timeout`) and
/// stream-silence (`step_timeout`) — on the configured cadence
/// ([`TimeoutConfig::interval`], default 5 s). When a rule breaches,
/// renders an `AgentNative` error block to stderr and sends a
/// [`WatchdogTermination`](super::super::termination::WatchdogTermination)
/// request to the exec wait loop so the child process group receives SIGTERM
/// with the configured `kill_grace`.
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
    watchdog_tx: std::sync::mpsc::Sender<super::super::termination::WatchdogTermination>,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
) -> (TickerCancel, thread::JoinHandle<()>) {
    let cancel = TickerCancel::new();
    let cancel_flag = cancel.clone();
    let fired = Arc::new(AtomicBool::new(false));
    let cadence = config.interval;

    let handle = thread::spawn(move || {
        let mut next_tick = Instant::now() + cadence;
        while !cancel_flag.is_cancelled() {
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
            if cancel_flag.sleep(sleep_for) {
                break;
            }
        }
    });

    (cancel, handle)
}

/// Spawn a minimal wall-clock-only timeout ticker for the non-streaming
/// spawn paths (`run_child` / `run_child_capture`).
///
/// The full [`spawn_timeout_watchdog_ticker`] needs `LiveMetrics`, a
/// `WatchdogState`, and a `StreamOutput` coordinator that only the structured
/// streaming path owns. The direct and capture paths have no semantic stream,
/// so only the wall-clock (`timeout`) rule applies. This ticker watches the
/// elapsed time and, once the budget is exceeded, sends a single `Timeout`
/// [`WatchdogTermination`](super::super::termination::WatchdogTermination) so
/// the unified wait loop performs the same group-targeted SIGTERM→SIGKILL
/// escalation every other path uses — closing the gap where a configured
/// `timeout` silently disabled Ctrl+C on the capture path.
///
/// Elapsed time is compared with [`Instant::saturating_duration_since`] (not a
/// precomputed deadline `Instant`), so an absurd budget such as `u64::MAX`
/// seconds never overflows the clock — it simply never fires.
pub(crate) fn spawn_wall_clock_timeout_ticker(
    timeout: Duration,
    started_at: Instant,
    watchdog_tx: std::sync::mpsc::Sender<super::super::termination::WatchdogTermination>,
) -> (TickerCancel, thread::JoinHandle<()>) {
    use super::super::termination::{WatchdogTermination, WatchdogTerminationReason};

    let cancel = TickerCancel::new();
    let cancel_flag = cancel.clone();
    let poll_interval = Duration::from_millis(100);

    let handle = thread::spawn(move || {
        loop {
            if cancel_flag.is_cancelled() {
                break;
            }
            let elapsed = Instant::now().saturating_duration_since(started_at);
            if elapsed >= timeout {
                let _ = watchdog_tx.send(WatchdogTermination {
                    reason: WatchdogTerminationReason::Timeout,
                    message: format!(
                        "wall-clock budget exceeded after {}",
                        format_duration(elapsed),
                    ),
                    stuck_subagents: Vec::new(),
                });
                break;
            }
            if cancel_flag.sleep(poll_interval) {
                break;
            }
        }
    });

    (cancel, handle)
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
) -> (TickerCancel, thread::JoinHandle<()>) {
    let cancel = TickerCancel::new();
    let cancel_flag = cancel.clone();
    let prompt_path_display = biscuit_file::to_portable_string(&prompt_timing.absolute_path);

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

        while !cancel_flag.is_cancelled() {
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
            if cancel_flag.sleep(sleep_for) {
                break;
            }
        }
    });

    (cancel, handle)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::wrap::exec::new_assistant_stream_inset;
    use crate::commands::wrap::exec::task_frame_fixtures::colored_writer;

    /// A long-running task holds a partial Markdown block past the idle window.
    /// Before the framed arm existed the flush went straight out through
    /// `stdout_writer`, so the task visibly lost its bar mid-run.
    #[test]
    fn idle_flush_carries_the_task_bar() {
        let (writer, frames, gutter) = colored_writer();
        let framed = Some(Arc::new(std::sync::Mutex::new(writer)));

        let renderer = Arc::new(std::sync::Mutex::new(new_assistant_stream_inset(2)));
        // A block with no terminating paragraph boundary: `append` holds it, so
        // only the idle flush can surface it.
        let held = renderer.lock().unwrap().append("partial block text\n");
        assert!(
            held.is_empty(),
            "fixture must hold the block, else the flush is not what emits it"
        );

        flush_idle_to_stream(
            &renderer,
            &framed,
            &StreamOutput::new(),
            Duration::ZERO,
        );

        let recorded = frames.lock().unwrap();
        assert!(
            !recorded.data.is_empty(),
            "the held block must reach the data channel on idle flush"
        );
        assert!(
            recorded.data.iter().all(|line| line.starts_with(&gutter)),
            "every flushed line must carry the task gutter: {:?}",
            recorded.data
        );
        assert!(
            recorded.status.is_empty(),
            "flushed body text is data, never status"
        );
    }

    /// Without a task the flush must stay on the plain stdout path.
    #[test]
    fn idle_flush_without_a_task_emits_nothing_to_a_sink() {
        let renderer = Arc::new(std::sync::Mutex::new(new_assistant_stream_inset(0)));
        renderer.lock().unwrap().append("text\n");
        // Asserting only that the undecorated arm is reachable and total; the
        // bytes land on the real stdout, which a unit test cannot observe.
        flush_idle_to_stream(&renderer, &None, &StreamOutput::new(), Duration::ZERO);
    }
}
