use std::io::IsTerminal;
use std::path::Path;
use std::process::Child;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use claudine::stream::parser::{SemanticStreamParser, StreamParseError};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;

pub(crate) mod exit;
pub(crate) mod spawn;
pub(crate) mod stream_capture;
pub(crate) mod subagent_watchdog;
pub(crate) mod termination;
pub(crate) mod timeouts;
pub(crate) mod watchdog;
pub(crate) mod wiring;

pub(crate) use exit::{cleanup_mcp_injection, exit_code_from_status, resolve_first_response};
pub(crate) use spawn::{run_child, run_child_capture, run_child_stream_semantic};

/// Switch the wrapper process cwd to the child's cwd and sync `PWD`.
///
/// Rust's `set_current_dir` calls `chdir(2)` but does NOT touch the
/// `PWD` environment variable — the shell convention is that `PWD`
/// tracks "where the user thinks they are", which can differ from
/// `getcwd(3)`. Several downstream tools (notably OpenCode's
/// `run.ts:276` resolving `process.env.PWD ?? process.cwd()`) trust
/// `PWD` over the real cwd. If we don't sync them, the spawned
/// child inherits the user's pre-chdir `PWD` (e.g. a package
/// subdirectory the user ran `just commit` from) and resolves paths
/// against the wrong root.
///
/// # Safety
///
/// Single-threaded wrapper startup; no other thread reads or writes
/// `PWD` concurrently with this call.
pub(crate) fn switch_process_cwd(child_cwd: &Path) -> Result<()> {
    let current = std::env::current_dir()?;
    if current != child_cwd {
        std::env::set_current_dir(child_cwd)?;
    }
    unsafe {
        std::env::set_var("PWD", child_cwd.as_os_str());
    }
    Ok(())
}

pub(crate) struct ChildIoOptions<'a> {
    pub(crate) stdout_noise_prefixes: &'a [&'a str],
    pub(crate) stderr_noise_prefixes: &'a [&'a str],
    pub(crate) stdin_seed: Option<&'a str>,
}

/// Execution telemetry collected for a single child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessTelemetry {
    pub total_elapsed: Duration,
    pub first_response_latency: Option<Duration>,
}

impl ProcessTelemetry {
    /// Convert telemetry into the shared [`AgentExecutionPerf`] model.
    pub(crate) fn into_agent_perf(
        self,
        api_duration_ms: Option<u64>,
    ) -> crate::perf::AgentExecutionPerf {
        crate::perf::AgentExecutionPerf {
            launches: 1,
            total_elapsed: self.total_elapsed,
            first_response_latency: self.first_response_latency,
            provider_api_duration: api_duration_ms.map(Duration::from_millis),
        }
    }
}

/// Result of a child process execution, enriched with termination info.
pub(crate) struct ProcessResult<T> {
    pub(crate) data: T,
    pub(crate) termination: claudine::harness::ProcessTermination,
    pub(crate) telemetry: ProcessTelemetry,
    /// Immediate child PID returned by `std::process::Command::spawn()`,
    /// captured immediately after a successful spawn. `None` only appears
    /// in fabricated results (e.g. parse-failure fallbacks) — a real
    /// spawn either produces `Some(child.id())` here or returns `Err`
    /// before `ProcessResult` is constructed.
    ///
    /// Per-attempt by construction: every call to a spawn function
    /// returns a fresh `ProcessResult`, so harness retries and
    /// composition iterations never inherit a stale PID from a prior
    /// attempt.
    //
    // Read in Phase 3 by the dispatch / stream-summary / reporting
    // surfaces; flagged here so the Phase 2 capture-only change does
    // not trip `-D warnings`.
    #[allow(dead_code)]
    pub(crate) agent_pid: Option<u32>,
    /// Structured runaway-guard context (Phase 6), populated when the run
    /// ended on a content-guard trip (exit-expression / repetition / volume).
    /// `None` for ordinary completions, timeouts, and rate-limit aborts.
    /// Carried so the attempt outcome can thread `error_kind` + guard detail
    /// into the failure-handler payload (C3a).
    pub(crate) guard_context: Option<claudine::harness::GuardContext>,
    /// Signals the declarative detection engine collected from the stdout
    /// stream (Phase E4). Populated only by the semantic spawn path
    /// (`run_child_stream_semantic`); the other spawn paths always carry an
    /// empty vector. Consumed by the rate-limit projection and persisted
    /// as `extra["signals"]` on the SessionEnd JSONL summary row.
    pub(crate) signals: Vec<claudine::signals::ObservedSignal>,
}

/// Construct the streaming assistant-text renderer over stdout.
///
/// The CLI owns the sink decisions per the render-components design (Ruling
/// 4): `stdout().is_terminal()` selects rendered Markdown vs raw passthrough,
/// and the cached [`TerminalOptions`] (image rendering disabled) are built
/// once here to avoid repeated theme detection on the streaming hot path. The
/// state machine itself lives in [`claudine::render::AssistantStream`].
///
/// `inset` reserves columns on the left for a caller that decorates the
/// rendered lines afterwards. A sequence task frames this stream's lines with a
/// bar gutter *after* rendering, so the renderer must wrap to a width that
/// leaves room for it — otherwise every full-width line overflows the terminal
/// by the gutter's width once the bar is prepended. Pass `0` for an
/// undecorated stream.
///
/// [`TerminalOptions`]: darkmatter::markdown::output::terminal::TerminalOptions
pub(crate) fn new_assistant_stream_inset(inset: u32) -> claudine::render::AssistantStream {
    use darkmatter::markdown::output::terminal::{TerminalImageMode, TerminalOptions};
    let term = std::io::stdout().is_terminal().then(|| {
        let mut term = crate::log::terminal();
        if inset > 0 {
            // Pinned to a fixed width, not just narrowed: `width()` falls back
            // to live detection whenever `fixed_width` is `None`, which would
            // silently discard the inset. Saturating because a terminal
            // narrower than the gutter has nothing left to give.
            term.fixed_width = Some(term.width().saturating_sub(inset).max(1));
        }
        term
    });
    let terminal_options = term.as_ref().map(|_| {
        let mut opts = TerminalOptions::default();
        opts.image_mode = TerminalImageMode::Never;
        opts
    });
    claudine::render::AssistantStream::new(term, terminal_options)
}

/// Callback type used by [`run_child_stream_semantic`] for assistant text.
pub(crate) type OutputTextCallback = Box<dyn FnMut(&str) + Send + 'static>;

/// Callback type used by [`run_child_stream_semantic`] for reasoning text.
pub(crate) type ReasoningCallback = Box<dyn FnMut(&str) + Send + 'static>;

/// Factory signature used by [`run_child_stream_semantic`] to construct the
/// parser inside the stdout reader thread.
///
/// The caller receives two callbacks: one for stdout markdown
/// ([`SemanticEvent::OutputText`]) and one for reasoning text
/// ([`SemanticEvent::Reasoning`]). The reasoning callback is currently a
/// no-op in the structured-stream path because `LiveSemanticSink` renders
/// reasoning directly through its section-aware stderr emitter. The second
/// parameter is retained for signature compatibility.
///
/// The trailing `Option<u32>` is the immediate child PID captured after a
/// successful spawn (invoked from the reader thread, so the PID is already
/// known). Builders stamp it onto their sink so live records carry
/// `EventMeta.agent_pid`.
///
/// [`SemanticEvent::OutputText`]: claudine::stream::semantic::SemanticEvent::OutputText
/// [`SemanticEvent::Reasoning`]: claudine::stream::semantic::SemanticEvent::Reasoning
/// [`LiveSemanticSink`]: super::live_semantic_sink::LiveSemanticSink
pub(crate) type SemanticParserBuilder = Box<
    dyn FnOnce(OutputTextCallback, ReasoningCallback, Option<u32>) -> Box<dyn SemanticStreamParser>
        + Send
        + 'static,
>;

/// Minimal fallback parser used when the real parser thread panics.
struct ErrorParser {
    exit_code: i32,
}

impl SemanticStreamParser for ErrorParser {
    fn feed_line(&mut self, _line: &str) -> std::result::Result<(), StreamParseError> {
        Ok(())
    }

    fn finish(self: Box<Self>, _exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            is_error: true,
            error_kind: Some("parse_failure".into()),
            error_message: Some("Stream parser thread panicked".into()),
            exit_code: self.exit_code,
            ..Default::default()
        }
    }
}

/// Join a thread with a timeout. Returns `true` if the thread joined
/// successfully within the deadline, `false` if it timed out.
///
/// On timeout the thread is **leaked** (detached) rather than panicked,
/// because the reader threads only terminate when their pipe closes and
/// there is no safe way to interrupt a blocking `BufReader::lines()` call
/// from outside.
fn join_with_timeout(handle: thread::JoinHandle<()>, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        // `is_finished()` is available on Rust 1.69+ and does not block.
        if handle.is_finished() {
            let _ = handle.join();
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    tracing::warn!(
        "reader thread did not exit within {:?}; detaching (pipe may still be held open by a descendant process)",
        timeout
    );
    std::mem::forget(handle);
    false
}

/// Join a thread that returns a value, with a timeout. Returns the value
/// on success or a fallback on timeout.
fn join_with_timeout_or<T>(handle: thread::JoinHandle<T>, timeout: Duration, fallback: T) -> T {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if handle.is_finished() {
            return handle.join().unwrap_or(fallback);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    tracing::warn!(
        "reader thread did not exit within {:?}; using fallback result",
        timeout
    );
    std::mem::forget(handle);
    fallback
}

/// After the main child exits, kill any orphaned descendant processes so
/// inherited pipe fds are closed and reader threads unblock. Without this,
/// a subagent spawned by the child (e.g. OpenCode Task tool) that inherits
/// stdout/stderr can keep the pipe open indefinitely, causing the reader
/// threads to hang on `BufReader::lines()`.
#[cfg(unix)]
fn kill_process_group(child: &mut Child) {
    let pid = child.id() as i32;
    // Derive the grace period from the same `TimeoutConfig` knob that
    // governs SIGTERM->SIGKILL escalation in the streaming wait loop,
    // so the two termination paths stay consistent.
    let kill_grace = timeouts::TimeoutConfig::resolve(None, None).kill_grace;
    // Send SIGTERM to the process group first (graceful), then SIGKILL.
    unsafe {
        // kill(-pgid, ...) sends to the entire process group.
        // With process_group(0), the pgid == child pid.
        if libc::kill(-pid, libc::SIGTERM) == 0 {
            // Give descendants the configured grace period to exit.
            std::thread::sleep(kill_grace);
            // Ensure everything is dead.
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill_process_group(_child: &mut Child) {}

/// Cooperative cancellation with an interruptible sleep for the wrap
/// ticker threads.
///
/// A bare `AtomicBool` polled between fixed `thread::sleep` calls leaves
/// teardown waiting out the in-flight sleep — up to ~1 s per ticker, paid
/// on every non-interactive run at process exit. Backing the flag with a
/// condvar lets [`TickerCancel::cancel`] wake a sleeping ticker
/// immediately, so [`stop_timing_ticker`]'s `join()` returns at once.
#[derive(Clone)]
pub(crate) struct TickerCancel {
    inner: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
}

impl TickerCancel {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new())),
        }
    }

    /// Request cancellation and wake any thread parked in [`Self::sleep`].
    pub(crate) fn cancel(&self) {
        let (lock, cvar) = &*self.inner;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }

    /// `true` once [`Self::cancel`] has been called.
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.inner.0.lock().unwrap()
    }

    /// Sleep up to `dur`, returning the instant `cancel` is called.
    ///
    /// ## Returns
    ///
    /// `true` if cancellation was observed (so the caller should stop), or
    /// `false` if the full `dur` elapsed without cancellation.
    pub(crate) fn sleep(&self, dur: Duration) -> bool {
        let (lock, cvar) = &*self.inner;
        let guard = lock.lock().unwrap();
        if *guard {
            return true;
        }
        let (guard, _timed_out) = cvar.wait_timeout(guard, dur).unwrap();
        *guard
    }
}

/// Signal a timing ticker thread to stop and join it.
///
/// Shared by the flush-if-idle ticker and the prompt-timing monitor —
/// both return the same `(cancel, handle)` pair and need identical
/// teardown. `None` is accepted so callers can pass through optional
/// handles without an extra match.
fn stop_timing_ticker(ticker: Option<(TickerCancel, thread::JoinHandle<()>)>) {
    if let Some((cancel, handle)) = ticker {
        cancel.cancel();
        let _ = handle.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_telemetry_into_agent_perf_populates_all_fields() {
        let telemetry = ProcessTelemetry {
            total_elapsed: Duration::from_secs(3),
            first_response_latency: Some(Duration::from_millis(500)),
        };
        let perf = telemetry.into_agent_perf(Some(1200));
        assert_eq!(perf.launches, 1);
        assert_eq!(perf.total_elapsed, Duration::from_secs(3));
        assert_eq!(
            perf.first_response_latency,
            Some(Duration::from_millis(500))
        );
        assert_eq!(
            perf.provider_api_duration,
            Some(Duration::from_millis(1200))
        );
    }

    #[test]
    fn process_telemetry_into_agent_perf_omits_api_when_none() {
        let telemetry = ProcessTelemetry {
            total_elapsed: Duration::from_secs(1),
            first_response_latency: None,
        };
        let perf = telemetry.into_agent_perf(None);
        assert_eq!(perf.launches, 1);
        assert_eq!(perf.provider_api_duration, None);
        assert_eq!(perf.first_response_latency, None);
    }
}
