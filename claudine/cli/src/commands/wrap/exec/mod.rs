use std::io::IsTerminal;
use std::path::Path;
use std::process::Child;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use claudine::stream::parser::SemanticStreamParser;
use color_eyre::eyre::Result;

pub(crate) mod control;
pub(crate) mod exit;
pub(crate) mod codex_app_server;
pub(crate) mod pi_rpc;
pub(crate) mod reader_join;
pub(crate) mod spawn;
pub(crate) mod stream_capture;
#[cfg(test)]
pub(crate) mod task_frame_fixtures;
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
    /// Bounded tails of the raw lines the child wrote, for the native-exit
    /// failure report. Populated only by the semantic spawn path, which reads
    /// both streams; the captured path returns whole streams in `data`, and
    /// the inherited and wire paths leave this `None`.
    pub(crate) stream_tails: Option<StreamTails>,
    /// Warnings for output readers that panicked, timed out, or failed to
    /// forward after the child exited. Never a provider failure: the child's
    /// own outcome stands, only its output may be incomplete.
    ///
    /// Each spawn path queues these for stderr; callers also store them on
    /// the session-end record and the attempt's
    /// [`OutputStatus`](claudine::harness::OutputStatus), where they survive
    /// a terminal that could not show them.
    pub(crate) reader_warnings: Vec<String>,
}

/// The last raw lines of a structured child's stdout and stderr, bounded by
/// [`claudine::signals::EXIT_STDOUT_TAIL_LINES`] and
/// [`claudine::signals::EXIT_STDERR_TAIL_LINES`].
///
/// The stderr lines are the ones that passed noise filtering and were not
/// consumed by a stderr bridge; every one of them was also streamed to the
/// terminal, or echoed there when a suppressed run failed.
#[derive(Clone, Default)]
pub(crate) struct StreamTails {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

/// The native-exit evidence of a structured run. Its stderr was already shown
/// (see [`StreamTails`]); its stdout went to the stream parser, not the
/// terminal. A run with no tails carries none.
pub(crate) fn structured_native_exit(
    exit_code: i32,
    termination: claudine::harness::ProcessTermination,
    tails: Option<&StreamTails>,
) -> crate::output::native_exit::NativeExit {
    let exit = crate::output::native_exit::NativeExit::new(exit_code, termination);
    match tails {
        Some(tails) => exit.with_stdout(&tails.stdout, false).with_stderr(&tails.stderr, true),
        None => exit,
    }
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
/// by the gutter's width once the bar is prepended. A nonzero inset on a
/// terminal also holds partial lines until they are complete, so they are
/// rendered (and folded) rather than streamed raw. Pass `0` for an
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
    let hold_partial_lines = inset > 0 && term.is_some();
    claudine::render::AssistantStream::new(term, terminal_options)
        .holding_partial_lines(hold_partial_lines)
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

/// After the main child exits, kill any orphaned descendant processes so
/// inherited pipe fds are closed and reader threads unblock. Without this,
/// a subagent spawned by the child (e.g. OpenCode Task tool) that inherits
/// stdout/stderr can keep the pipe open indefinitely, causing the reader
/// threads to hang on `BufReader::lines()`.
///
/// The grace period ends early once the group is empty, and a user
/// interrupt (Ctrl+C) cuts it short to `SIGKILL`. For its whole duration the
/// teardown counts as an active wait loop, so a repeated Ctrl+C is deferred
/// here rather than force-exiting the wrapper past the run's `failure` and
/// `finalize` lifecycle events.
#[cfg(unix)]
fn kill_process_group(child: &mut Child) {
    // With process_group(0), the pgid == child pid.
    let pgid = child.id() as i32;
    // Derive the grace period from the same `TimeoutConfig` knob that
    // governs SIGTERM->SIGKILL escalation in the streaming wait loop,
    // so the two termination paths stay consistent.
    let kill_grace = timeouts::TimeoutConfig::resolve(None, None).kill_grace;
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();
    terminate_process_group(pgid, kill_grace);
    // After `terminate_process_group` however it ended, so the held window
    // does not depend on whether the group outlived its SIGTERM.
    #[cfg(feature = "terminal-tests")]
    hold_teardown_for_terminal_tests();
}

/// `SIGTERM`, then `SIGKILL` once `kill_grace` elapses or a user interrupt is
/// observed; returns early once the group is empty.
#[cfg(unix)]
fn terminate_process_group(pgid: i32, kill_grace: Duration) {
    // SAFETY: kill(2) with a negative pid signals the whole process group; a
    // failure (no surviving member) means there is nothing left to reap.
    if unsafe { libc::kill(-pgid, libc::SIGTERM) } != 0 {
        return;
    }
    let deadline = Instant::now() + kill_grace;
    while Instant::now() < deadline {
        // Signal 0 probes for any surviving member without delivering anything.
        if unsafe { libc::kill(-pgid, 0) } != 0 {
            return;
        }
        if crate::output::user_interrupt_observed() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // SAFETY: as above; this is the final, unconditional sweep.
    unsafe {
        libc::kill(-pgid, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill_process_group(_child: &mut Child) {}

/// Test seam, compiled only with the `terminal-tests` feature (never into a
/// default or installed build): with `CLAUDINE_TEST_TEARDOWN_HOLD=<path>` set,
/// the teardown creates `<path>.reached` once its signalling has ended — by
/// the final `SIGKILL`, by the group emptying, or by `SIGTERM` finding no
/// member — and then stays inside its wait-loop guard until `<path>.release`
/// exists, for at most 60 s.
///
/// The first Ctrl+C ends the real teardown within one 50 ms poll, and a group
/// that exits on `SIGTERM` ends it with no press at all, so without this no
/// keypress can be aimed at the window in which a repeat press must defer.
/// The terminal-tier `…_repeat_ctrl_c_during_orphan_teardown_…` tests hold
/// the teardown here to land their second press inside it.
#[cfg(all(unix, feature = "terminal-tests"))]
fn hold_teardown_for_terminal_tests() {
    let Some(hold) = std::env::var_os("CLAUDINE_TEST_TEARDOWN_HOLD") else {
        return;
    };
    let marker = |suffix: &str| {
        let mut path = hold.clone();
        path.push(suffix);
        std::path::PathBuf::from(path)
    };
    let release = marker(".release");
    if std::fs::write(marker(".reached"), b"").is_err() {
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    while !release.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
}

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

    /// A process group that ignores `SIGTERM`, `SIGKILL`ed on drop so a failed
    /// assertion cannot leak the survivor.
    #[cfg(unix)]
    struct TermSurvivingGroup {
        pgid: i32,
    }

    #[cfg(unix)]
    impl TermSurvivingGroup {
        /// Spawn the group, returning once its `SIGTERM` disposition is in
        /// place so the teardown's `SIGTERM` cannot empty it.
        fn spawn() -> (Self, Child) {
            use std::io::BufRead as _;
            use std::os::unix::process::CommandExt as _;

            let mut leader = std::process::Command::new("/bin/sh")
                .args([
                    "-c",
                    "trap '' TERM; echo ready; while :; do /bin/sleep 0.05; done",
                ])
                .process_group(0)
                .stdout(std::process::Stdio::piped())
                .spawn()
                .expect("spawn TERM-surviving group");
            let group = Self {
                pgid: leader.id() as i32,
            };
            let mut ready = String::new();
            std::io::BufReader::new(leader.stdout.take().expect("piped stdout"))
                .read_line(&mut ready)
                .expect("read readiness line");
            assert_eq!(ready.trim(), "ready");
            (group, leader)
        }
    }

    #[cfg(unix)]
    impl Drop for TermSurvivingGroup {
        fn drop(&mut self) {
            // SAFETY: `kill(2)` on the group this fixture created; ESRCH means
            // it is already gone.
            unsafe {
                libc::kill(-self.pgid, libc::SIGKILL);
            }
        }
    }

    /// For the whole post-exit teardown a repeat Ctrl+C must defer rather than
    /// force-exit past the run's `failure`/`finalize` events. The first press
    /// ends the teardown within one poll, so no subprocess test can reliably
    /// land a second press inside it; this observes the flag the SIGINT
    /// handler reads while a `SIGTERM`-surviving group holds the teardown open.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn kill_process_group_defers_repeat_interrupts_until_ctrl_c_ends_it() {
        use crate::commands::compose::interrupt::{PressRung, press_rung};
        use std::os::unix::process::ExitStatusExt as _;

        // SAFETY: serial_test::serial prevents concurrent env access.
        let _kill_grace = unsafe { test_toolkit::EnvGuard::set("CLAUDINE_KILL_GRACE", "60s") };
        let (_group, mut leader) = TermSurvivingGroup::spawn();
        let teardown = thread::spawn(move || {
            kill_process_group(&mut leader);
            leader
        });

        let deadline = Instant::now() + Duration::from_secs(5);
        while !crate::output::wait_loop_active() {
            assert!(
                !teardown.is_finished(),
                "the teardown returned while its group was still alive"
            );
            assert!(
                Instant::now() < deadline,
                "the teardown never marked itself as an active wait loop"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            press_rung(
                2,
                crate::output::wait_loop_active(),
                claudine::interrupt::terminal_lifecycle_active()
            ),
            PressRung::Defer,
            "a repeat press during the teardown must be deferred"
        );
        assert!(!teardown.is_finished());

        crate::output::mark_user_interrupted();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !teardown.is_finished() {
            assert!(
                Instant::now() < deadline,
                "Ctrl+C must cut the 60s kill grace short"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let mut leader = teardown.join().expect("teardown thread");
        crate::output::clear_user_interrupt_for_tests();

        assert!(
            !crate::output::wait_loop_active(),
            "the teardown must release the wait-loop flag when it returns"
        );
        let status = leader.wait().expect("reap group leader");
        assert_eq!(status.signal(), Some(libc::SIGKILL));
    }
}
