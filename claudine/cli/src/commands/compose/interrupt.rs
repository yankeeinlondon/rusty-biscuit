//! Process-scoped SIGINT (Ctrl+C) handling for composition commands.
//!
//! The guard is installed at the top of the compose / inline-compose run so
//! it covers the entire prep window, not just the loop. Downstream surfaces
//! branch on the process-scoped `USER_INTERRUPTED` flag.

/// Exit code emitted when Ctrl+C is observed during a compose run.
/// Matches the standard `128 + SIGINT(2)` convention used by shells.
pub(crate) const USER_INTERRUPT_EXIT_CODE: i32 = 130;

/// RAII guard returned by [`install_user_interrupt_guard`]. On Windows,
/// dropping it withdraws the notice registration and this run's hold on the
/// process-wide console handler. On Unix the `signal_hook` handler is **not**
/// unregistered — [`signal_hook::SigId`] has no `Drop` — so it stays installed
/// for the rest of the process.
pub(crate) struct UserInterruptGuard {
    #[cfg(unix)]
    _hook: Option<signal_hook::SigId>,
    /// Declared first so it clears before the console handler is released: a
    /// press landing between the two drops must find no compose run to act on.
    #[cfg(not(unix))]
    _notice: NoticeRegistration,
    #[cfg(not(unix))]
    _handler: crate::commands::wrap::exec::termination::ComposeInterruptHandlerGuard,
}

/// Force-exit notice shown before the force-exit on a second Ctrl+C that
/// lands outside a child wait loop.
const FORCE_EXIT_NOTICE: &[u8] =
    "\n\u{26a0} second interrupt — force-exiting compose\n".as_bytes();

/// How long a repeat press lets an in-flight terminal lifecycle event
/// (`success`/`blocked`/`failure`/`finalize`) keep running before the wrapper
/// force-exits. A run that finishes sooner exits normally.
pub(crate) const TERMINAL_LIFECYCLE_EXIT_GRACE: std::time::Duration =
    std::time::Duration::from_millis(500);

/// Shown when a repeat press arms the [`TERMINAL_LIFECYCLE_EXIT_GRACE`]
/// deadline.
const GRACE_EXIT_NOTICE: &[u8] =
    "\n\u{26a0} second interrupt — letting the lifecycle event finish (500ms max) before exiting\n"
        .as_bytes();

/// Shown when the grace deadline passes with the run still going.
const GRACE_EXPIRED_NOTICE: &[u8] =
    "\n\u{26a0} lifecycle event still running — force-exiting compose\n".as_bytes();

/// Rung of the compose-scoped interrupt ladder one press lands on.
///
/// Shared by both hosts so the two handlers cannot drift: a Unix signal handler
/// and a Windows console-control thread reach the same verdict from the same
/// two inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressRung {
    /// First press — write the interrupt notice once.
    Notice,
    /// Repeat press while a child wait loop owns the child-targeted ladder.
    /// That loop escalates its own child; force-exiting here would kill the
    /// wrapper out from under a still-reaping child.
    Defer,
    /// Repeat press while a terminal lifecycle event is running and no wait
    /// loop owns the ladder. Force-exiting at once would cut `failure` or
    /// `finalize` off mid-side-effect, so the run gets
    /// [`TERMINAL_LIFECYCLE_EXIT_GRACE`] to finish before a force-exit.
    GraceExit,
    /// Repeat press with no wait loop to defer to. The interrupt flag alone
    /// only short-circuits at the next explicit checkpoint, so a main thread
    /// wedged in a synchronous call (network send, hung TTS subprocess) would
    /// otherwise ignore Ctrl+C entirely.
    ForceExit,
}

/// Resolve one press against the ladder.
///
/// `const` and allocation-free so the Unix SIGINT handler can call it while
/// remaining async-signal-safe.
pub(crate) const fn press_rung(
    count: u8,
    wait_loop_active: bool,
    terminal_lifecycle_active: bool,
) -> PressRung {
    if count == 1 {
        PressRung::Notice
    } else if wait_loop_active {
        PressRung::Defer
    } else if terminal_lifecycle_active {
        PressRung::GraceExit
    } else {
        PressRung::ForceExit
    }
}

/// How long a forced exit lets its notice reach the terminal before ending
/// the process.
const EXIT_NOTICE_SETTLE: std::time::Duration = std::time::Duration::from_millis(250);

/// What the Unix SIGINT handler asks the relay thread to do.
#[cfg(unix)]
const RELAY_NOTICE: u8 = b'N';
#[cfg(unix)]
const RELAY_GRACE_EXIT: u8 = b'G';
#[cfg(unix)]
const RELAY_FORCE_EXIT: u8 = b'F';

/// Write end of the Unix relay's self-pipe, or `-1` before the relay exists.
/// Read from the SIGINT handler, so it is a plain atomic.
#[cfg(unix)]
static RELAY_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// The first-press notice of the most recently installed ladder. The Unix
/// handler is never unregistered, so neither is its notice.
#[cfg(unix)]
static LADDER_NOTICE: std::sync::Mutex<Option<std::sync::Arc<Vec<u8>>>> =
    std::sync::Mutex::new(None);

/// Ask the relay thread to act on one press; `false` when there is no relay
/// or the request could not be queued.
///
/// One non-blocking `write(2)` of one byte, so it is async-signal-safe and
/// cannot wait.
#[cfg(unix)]
fn post_to_relay(request: u8) -> bool {
    let fd = RELAY_FD.load(std::sync::atomic::Ordering::SeqCst);
    let byte = [request];
    fd >= 0 && unsafe { libc::write(fd, byte.as_ptr() as *const libc::c_void, 1) } == 1
}

/// Start (once per process) the thread that shows the ladder's notices and
/// performs its exits.
///
/// The SIGINT handler only counts the press and posts one byte here: a
/// `write(2)` to a terminal that stopped reading would hold the handler, and
/// the thread it interrupted, for good, so the exit rungs would never exit.
/// This thread submits each notice as a diagnostic
/// ([`crate::terminal_gate::write`], queued during a wrapped run), gives an
/// exit notice [`EXIT_NOTICE_SETTLE`] to reach the terminal, and `_exit`s. A
/// [`RELAY_GRACE_EXIT`] arms the [`TERMINAL_LIFECYCLE_EXIT_GRACE`] deadline,
/// which later presses do not extend; a run that finishes in the meantime has
/// already exited. Both pipe ends live for the whole process because the
/// handler that writes to them is never unregistered.
#[cfg(unix)]
fn ensure_relay() {
    use std::os::fd::IntoRawFd as _;

    static RELAY: std::sync::Once = std::sync::Once::new();
    RELAY.call_once(|| {
        let Ok((reader, writer)) = std::os::unix::net::UnixStream::pair() else {
            return;
        };
        if writer.set_nonblocking(true).is_err() {
            return;
        }
        let spawned = std::thread::Builder::new()
            .name("compose-interrupt-relay".into())
            .spawn(move || run_relay(reader));
        if spawned.is_ok() {
            RELAY_FD.store(writer.into_raw_fd(), std::sync::atomic::Ordering::SeqCst);
        }
    });
}

#[cfg(unix)]
fn run_relay(mut reader: std::os::unix::net::UnixStream) {
    use std::io::Read as _;

    let mut grace_deadline: Option<std::time::Instant> = None;
    loop {
        let wait = grace_deadline.map(|deadline| {
            deadline
                .saturating_duration_since(std::time::Instant::now())
                .max(std::time::Duration::from_millis(1))
        });
        if reader.set_read_timeout(wait).is_err() {
            return;
        }
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(1) => match byte[0] {
                RELAY_NOTICE => {
                    let notice = LADDER_NOTICE
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .clone();
                    if let Some(notice) = notice {
                        write_notice(&notice);
                    }
                }
                RELAY_GRACE_EXIT => {
                    write_notice(GRACE_EXIT_NOTICE);
                    grace_deadline.get_or_insert_with(|| {
                        std::time::Instant::now() + TERMINAL_LIFECYCLE_EXIT_GRACE
                    });
                }
                RELAY_FORCE_EXIT => exit_after_notice(FORCE_EXIT_NOTICE),
                _ => {}
            },
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) && grace_deadline.is_some() =>
            {
                exit_after_notice(GRACE_EXPIRED_NOTICE)
            }
            _ => return,
        }
    }
}

/// Show `notice` without waiting on the terminal during a wrapped run.
fn write_notice(notice: &[u8]) {
    crate::terminal_gate::write(crate::commands::wrap::output_worker::Stream::Stderr, notice);
}

/// Show `notice`, give it a bounded chance to reach the terminal, and end the
/// process with [`USER_INTERRUPT_EXIT_CODE`].
fn exit_after_notice(notice: &[u8]) -> ! {
    write_notice(notice);
    crate::terminal_gate::settle(EXIT_NOTICE_SETTLE);
    force_exit()
}

/// Install a process-scoped user-interrupt handler that covers the **entire**
/// compose / inline-compose run — including the slow prep phase before
/// the loop is entered. On every press the handler:
///
/// - Marks the process-scoped `USER_INTERRUPTED` flag so any downstream
///   surface (loop executor, live semantic sink, post-prep checkpoints)
///   can branch on it.
/// - Resolves [`press_rung`] and acts on it. The first press writes a
///   pre-rendered INFO notice to stderr exactly once; the notice has a leading
///   `\n` so it lands at column 1 (off the terminal's echoed `^C`) and the
///   prompt is rendered as an OSC8 hyperlink whose visible text is the user's
///   CLI argument verbatim. A repeat press force-exits with
///   [`USER_INTERRUPT_EXIT_CODE`] unless a child wait loop owns the ladder.
///
/// The two hosts differ only in where presses come from:
///
/// - **Unix** — a `signal_hook` SIGINT registration. `signal_hook::low_level::
///   register` stacks handlers, so this one composes cleanly with the
///   per-iteration SIGINT handler the wrapper installs around each agent child.
/// - **Windows** — Ctrl+C arrives at one process-wide console handler rather
///   than at this thread, so the run registers with the same coordinator every
///   wait loop and every sequence flag uses. The registration is refcounted:
///   dropping this guard while a sequence registration or a child wait loop is
///   still live leaves the shared handler installed.
pub(crate) fn install_user_interrupt_guard(prompt_argv: &str) -> UserInterruptGuard {
    install_ladder(format_user_interrupt_message(prompt_argv), 0)
}

/// Install the same ladder for the exit-time delivery drain of a command that
/// holds no compose registration (`sequence`, the provider wrappers).
///
/// A user who already pressed Ctrl+C during the run starts on the second
/// rung, so their next press force-exits instead of printing a notice. On
/// Windows that already follows from the handler's process-wide press count.
pub(crate) fn install_drain_interrupt_guard() -> UserInterruptGuard {
    let earlier_presses = u8::from(crate::output::user_interrupt_observed());
    install_ladder(format_drain_interrupt_message(), earlier_presses)
}

fn install_ladder(notice: String, earlier_presses: u8) -> UserInterruptGuard {
    let bytes = std::sync::Arc::new(notice.into_bytes());
    let presses = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(earlier_presses));

    #[cfg(unix)]
    {
        ensure_relay();
        *LADDER_NOTICE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(std::sync::Arc::clone(&bytes));
        let presses_handler = std::sync::Arc::clone(&presses);
        let hook = unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                crate::output::mark_user_interrupted();
                let count = presses_handler.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                // Only atomics and one non-blocking `write(2)` to the relay:
                // no allocation, no lock, no `tracing`, and no write to the
                // terminal, which could block this handler for good.
                match press_rung(
                    count,
                    crate::output::wait_loop_active(),
                    claudine::interrupt::terminal_lifecycle_active(),
                ) {
                    PressRung::Notice => {
                        post_to_relay(RELAY_NOTICE);
                    }
                    PressRung::Defer => {}
                    PressRung::GraceExit => {
                        if !post_to_relay(RELAY_GRACE_EXIT) {
                            // No relay to wait out the grace: exit now, without
                            // a notice this handler cannot safely write.
                            libc::_exit(USER_INTERRUPT_EXIT_CODE);
                        }
                    }
                    PressRung::ForceExit => {
                        if !post_to_relay(RELAY_FORCE_EXIT) {
                            // `_exit` (not `exit`) is async-signal-safe — it
                            // skips atexit handlers and destructors.
                            libc::_exit(USER_INTERRUPT_EXIT_CODE);
                        }
                    }
                }
            })
        }
        .ok();
        UserInterruptGuard { _hook: hook }
    }
    #[cfg(not(unix))]
    {
        // Windows delivers Ctrl+C to a console handler, not to this thread, so
        // the press counter lives in the process-scoped coordinator that the
        // handler shares with every wait loop and with a sequence run's flag —
        // `presses` is a Unix-only construct.
        let _ = presses;
        let handler =
            crate::commands::wrap::exec::termination::register_compose_interrupt_handler();
        UserInterruptGuard {
            _notice: NoticeRegistration::install(bytes),
            _handler: handler,
        }
    }
}

/// The compose run currently eligible for interrupt handling, if any.
///
/// A process runs at most one compose subcommand, so this is a cell rather than
/// a registry. It holds the pre-rendered notice so the handler thread — which
/// owns no reference to the run — can emit byte-identical output to the Unix
/// signal handler's.
static COMPOSE_NOTICE: std::sync::Mutex<Option<std::sync::Arc<Vec<u8>>>> =
    std::sync::Mutex::new(None);

/// Publishes a compose run's notice for the console handler, and withdraws it
/// on `Drop` so a press arriving after the subcommand returns is inert.
#[cfg_attr(unix, allow(dead_code))]
struct NoticeRegistration;

#[cfg_attr(unix, allow(dead_code))]
impl NoticeRegistration {
    fn install(bytes: std::sync::Arc<Vec<u8>>) -> Self {
        *lock_notice() = Some(bytes);
        Self
    }
}

impl Drop for NoticeRegistration {
    fn drop(&mut self) {
        *lock_notice() = None;
    }
}

/// A poisoned cell must not stop a later press from being handled, so a panic
/// in an unrelated holder is recovered from rather than propagated.
fn lock_notice() -> std::sync::MutexGuard<'static, Option<std::sync::Arc<Vec<u8>>>> {
    COMPOSE_NOTICE.lock().unwrap_or_else(|e| e.into_inner())
}

/// What one console press implies for the compose run, resolved without
/// touching the process.
///
/// Separated from [`on_console_interrupt`] because the forceful rung ends the
/// process: the decision is assertable in-process, the application of it is
/// not.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(unix, allow(dead_code))]
pub(crate) enum ComposeInterruptEffect {
    /// No compose run is registered — this press is not ours to act on.
    Inactive,
    /// Write these bytes to stderr exactly once.
    Notice(std::sync::Arc<Vec<u8>>),
    /// A child wait loop owns the ladder; do nothing.
    Defer,
    /// Write [`GRACE_EXIT_NOTICE`], let the in-flight terminal lifecycle event
    /// run for [`TERMINAL_LIFECYCLE_EXIT_GRACE`], then force-exit.
    GraceExit,
    /// Write [`FORCE_EXIT_NOTICE`] and end the process with
    /// [`USER_INTERRUPT_EXIT_CODE`].
    ForceExit,
}

/// Resolve a process-wide press count against the registered compose run.
#[cfg_attr(unix, allow(dead_code))]
pub(crate) fn classify_console_interrupt(
    count: u8,
    wait_loop_active: bool,
    terminal_lifecycle_active: bool,
) -> ComposeInterruptEffect {
    let Some(notice) = lock_notice().clone() else {
        return ComposeInterruptEffect::Inactive;
    };
    match press_rung(count, wait_loop_active, terminal_lifecycle_active) {
        PressRung::Notice => ComposeInterruptEffect::Notice(notice),
        PressRung::Defer => ComposeInterruptEffect::Defer,
        PressRung::GraceExit => ComposeInterruptEffect::GraceExit,
        PressRung::ForceExit => ComposeInterruptEffect::ForceExit,
    }
}

/// Windows counterpart of the Unix SIGINT handler, called from the process-wide
/// console-control handler with that handler's process-wide press count.
///
/// The console handler runs on a thread of its own, not in signal context, so
/// unlike the Unix handler it needs no relay: it submits each notice as a
/// diagnostic itself ([`crate::terminal_gate::write`], queued during a wrapped
/// run), so a terminal that stopped reading cannot keep the exit rungs from
/// reaching [`force_exit`].
///
/// ## Notes
///
/// The bytes are the same [`format_user_interrupt_message`] output the Unix
/// handler emits. During the prep window Windows additionally shows the
/// wrapper's own feedback line, because the console handler this run installed
/// is process-wide; on Unix that line appears only once a child wait loop has
/// installed its handler.
#[cfg_attr(unix, allow(dead_code))]
pub(crate) fn on_console_interrupt(count: u8) {
    let effect = classify_console_interrupt(
        count,
        crate::output::wait_loop_active(),
        claudine::interrupt::terminal_lifecycle_active(),
    );
    if effect == ComposeInterruptEffect::Inactive {
        return;
    }
    crate::output::mark_user_interrupted();
    match effect {
        ComposeInterruptEffect::Inactive | ComposeInterruptEffect::Defer => {}
        ComposeInterruptEffect::Notice(bytes) => {
            write_notice(&bytes);
        }
        ComposeInterruptEffect::GraceExit => {
            write_notice(GRACE_EXIT_NOTICE);
            // This is the console handler's own thread, so it can wait out the
            // grace itself; a run that finishes sooner has already exited.
            std::thread::sleep(TERMINAL_LIFECYCLE_EXIT_GRACE);
            exit_after_notice(GRACE_EXPIRED_NOTICE);
        }
        ComposeInterruptEffect::ForceExit => {
            exit_after_notice(FORCE_EXIT_NOTICE);
        }
    }
}

/// End the process without running destructors or atexit handlers.
///
/// The caller is the Unix relay thread or the Windows console-handler thread,
/// while the main thread is, by construction, wedged in the synchronous call
/// the user is trying to escape: running atexit handlers and destructors (or
/// flushing a stdout a stalled terminal is holding) is exactly the wedge being
/// escaped.
#[cfg(windows)]
fn force_exit() -> ! {
    unsafe { windows::Win32::System::Threading::ExitProcess(USER_INTERRUPT_EXIT_CODE as u32) }
}

#[cfg(unix)]
fn force_exit() -> ! {
    // SAFETY: `_exit(2)` ends the process; it touches no Rust state.
    unsafe { libc::_exit(USER_INTERRUPT_EXIT_CODE) }
}

/// Build the rendered interrupt notice, with a leading newline so the
/// terminal's echoed `^C` does not share a line.
///
/// At install time we have only the user's CLI argument (e.g. the
/// relative path they typed). We use that verbatim as the OSC8 visible
/// text, and best-effort canonicalize it against the current working
/// directory to produce an absolute file-URL link target. If
/// canonicalization fails (path doesn't exist yet, permission denied,
/// etc.) we fall back to a plain (non-hyperlinked) prose line.
pub(crate) fn format_user_interrupt_message(prompt_argv: &str) -> String {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::components::status::{Status, StatusState};

    let absolute = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join(prompt_argv))
        .and_then(|p| biscuit_file::canonicalize_simplified(&p).ok())
        .and_then(|p| crate::cli_utils::file_url(&p));

    let prose = if let Some(absolute) = absolute {
        format!("User interrupted compose operation in [{prompt_argv}]({absolute})")
    } else {
        format!("User interrupted compose operation in <yellow>{prompt_argv}</yellow>")
    };

    let term = crate::log::terminal();
    let body = Status::from_prose(prose)
        .state(StatusState::Info)
        .render(&term);

    format!("\n{body}")
}

/// The notice for a first press during the exit-time delivery drain.
pub(crate) fn format_drain_interrupt_message() -> String {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::components::status::{Status, StatusState};

    let body = Status::from_prose(
        "User interrupted while waiting for outbound messages; press Ctrl+C again to exit now",
    )
    .state(StatusState::Info)
    .render(&crate::log::terminal());
    format!("\n{body}")
}

/// The compose ladder's bookkeeping, exercised on every host.
///
/// The Windows console handler that drives it in production cannot run here, so
/// what is pinned is the part that is platform-neutral by construction: the
/// rung each press resolves to, and the registration cell the handler reads.
/// The forceful rung's *application* ends the process and is therefore asserted
/// as a decision rather than performed.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn notice_bytes() -> Arc<Vec<u8>> {
        Arc::new(b"notice\n".to_vec())
    }

    #[test]
    fn first_press_takes_the_notice_rung() {
        assert_eq!(press_rung(1, false, false), PressRung::Notice);
        assert_eq!(press_rung(1, false, true), PressRung::Notice);
        assert_eq!(
            press_rung(1, true, false),
            PressRung::Notice,
            "a wait loop must not suppress the one-and-only notice"
        );
    }

    #[test]
    fn a_repeat_press_force_exits_only_when_no_wait_loop_owns_the_ladder() {
        assert_eq!(press_rung(2, false, false), PressRung::ForceExit);
        assert_eq!(press_rung(2, true, false), PressRung::Defer);
        assert_eq!(press_rung(9, false, false), PressRung::ForceExit);
        assert_eq!(press_rung(9, true, false), PressRung::Defer);
    }

    #[test]
    fn a_repeat_press_during_a_terminal_lifecycle_event_takes_the_grace_rung() {
        assert_eq!(press_rung(2, false, true), PressRung::GraceExit);
        assert_eq!(press_rung(9, false, true), PressRung::GraceExit);
        assert_eq!(
            press_rung(2, true, true),
            PressRung::Defer,
            "a wait loop still owns the ladder while its child runs"
        );
    }

    #[test]
    #[serial_test::serial]
    fn a_press_with_no_compose_run_registered_is_inert() {
        assert_eq!(
            classify_console_interrupt(1, false, false),
            ComposeInterruptEffect::Inactive
        );
    }

    #[test]
    #[serial_test::serial]
    fn registration_publishes_the_notice_and_drop_withdraws_it() {
        let bytes = notice_bytes();
        {
            let _registration = NoticeRegistration::install(Arc::clone(&bytes));
            assert_eq!(
                classify_console_interrupt(1, false, false),
                ComposeInterruptEffect::Notice(bytes)
            );
        }
        assert_eq!(
            classify_console_interrupt(1, false, false),
            ComposeInterruptEffect::Inactive,
            "a press after the compose subcommand returned must not act on it"
        );
    }

    #[test]
    #[serial_test::serial]
    fn a_registered_run_defers_or_force_exits_on_the_second_press() {
        let _registration = NoticeRegistration::install(notice_bytes());

        assert_eq!(
            classify_console_interrupt(2, true, false),
            ComposeInterruptEffect::Defer
        );
        assert_eq!(
            classify_console_interrupt(2, false, true),
            ComposeInterruptEffect::GraceExit
        );
        assert_eq!(
            classify_console_interrupt(2, false, false),
            ComposeInterruptEffect::ForceExit
        );
    }

    /// The finding this seam closes: on Windows the compose run marked nothing,
    /// so `USER_INTERRUPTED` — and every downstream surface that branches on it
    /// — was unreachable.
    #[test]
    #[serial_test::serial]
    fn the_first_press_marks_the_process_interrupted() {
        crate::output::clear_user_interrupt_for_tests();
        let _registration = NoticeRegistration::install(notice_bytes());

        on_console_interrupt(1);

        assert!(crate::output::user_interrupt_observed());
        crate::output::clear_user_interrupt_for_tests();
    }

    /// A press arriving after the run's guard dropped must not resurrect the
    /// flag — the handler stays installed for as long as any other holder wants
    /// it, so presses keep arriving.
    #[test]
    #[serial_test::serial]
    fn a_press_after_deregistration_does_not_mark_the_process() {
        crate::output::clear_user_interrupt_for_tests();
        drop(NoticeRegistration::install(notice_bytes()));

        on_console_interrupt(1);

        assert!(!crate::output::user_interrupt_observed());
    }

    /// The SIGINT handler only posts the press to the relay; the relay queues
    /// the notice. With the output worker stuck on the terminal and std's
    /// stream locks held, a press still returns and the notice is queued.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn a_first_press_notice_is_queued_by_the_relay_not_written_by_the_handler() {
        crate::output::clear_user_interrupt_for_tests();
        let routed = crate::terminal_gate::tests::RoutedToBlockedWorker::new();
        let _guard = install_ladder("relayed notice\n".to_string(), 0);
        let output = Arc::clone(routed.output());

        let returned = crate::terminal_gate::tests::returns_while_std_streams_are_locked(move || {
            unsafe { libc::raise(libc::SIGINT) };
            let started = std::time::Instant::now();
            while output.queued_frames() == 0 {
                assert!(
                    started.elapsed() < std::time::Duration::from_secs(5),
                    "the relay never queued the notice"
                );
                std::thread::yield_now();
            }
        });

        assert!(returned, "the press waited on the terminal");
        assert!(crate::output::user_interrupt_observed());
        let written = routed.release();
        assert_eq!(
            written.last().map(|(_, bytes)| bytes.as_slice()),
            Some(b"relayed notice\n".as_slice())
        );
        crate::output::clear_user_interrupt_for_tests();
    }
}
