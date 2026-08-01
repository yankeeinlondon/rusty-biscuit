//! Process-scoped SIGINT (Ctrl+C) handling for composition commands.
//!
//! The guard is installed at the top of the compose / inline-compose run so
//! it covers the entire prep window, not just the loop. Downstream surfaces
//! branch on the process-scoped `USER_INTERRUPTED` flag.

/// Exit code emitted when Ctrl+C is observed during a compose run.
/// Matches the standard `128 + SIGINT(2)` convention used by shells.
pub(crate) const USER_INTERRUPT_EXIT_CODE: i32 = 130;

/// RAII guard returned by [`install_user_interrupt_guard`]. Drops the
/// underlying registration when the compose subcommand returns: on Unix the
/// `signal_hook` handler, restoring whatever handler was previously installed;
/// on Windows the notice registration plus this run's hold on the process-wide
/// console handler.
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

/// Force-exit notice written before the force-exit on a second Ctrl+C that
/// lands outside a child wait loop. Static bytes so the Unix write stays
/// async-signal-safe (no allocation, no formatting).
const FORCE_EXIT_NOTICE: &[u8] =
    "\n\u{26a0} second interrupt — force-exiting compose\n".as_bytes();

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
pub(crate) const fn press_rung(count: u8, wait_loop_active: bool) -> PressRung {
    if count == 1 {
        PressRung::Notice
    } else if wait_loop_active {
        PressRung::Defer
    } else {
        PressRung::ForceExit
    }
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
    let bytes = std::sync::Arc::new(format_user_interrupt_message(prompt_argv).into_bytes());
    let presses = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));

    #[cfg(unix)]
    {
        let bytes_handler = std::sync::Arc::clone(&bytes);
        let presses_handler = std::sync::Arc::clone(&presses);
        let hook = unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                crate::output::mark_user_interrupted();
                let count = presses_handler.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                match press_rung(count, crate::output::wait_loop_active()) {
                    // `write(2)` on a file descriptor is async-signal-safe; the
                    // Rust stdio macros (`eprintln!`, `println!`) are not, and
                    // any allocation or `tracing` call would be unsafe here.
                    PressRung::Notice => {
                        let buf = bytes_handler.as_slice();
                        libc::write(
                            libc::STDERR_FILENO,
                            buf.as_ptr() as *const libc::c_void,
                            buf.len(),
                        );
                    }
                    PressRung::Defer => {}
                    PressRung::ForceExit => {
                        libc::write(
                            libc::STDERR_FILENO,
                            FORCE_EXIT_NOTICE.as_ptr() as *const libc::c_void,
                            FORCE_EXIT_NOTICE.len(),
                        );
                        // `_exit` (not `exit`) is async-signal-safe — it skips
                        // atexit handlers and destructors.
                        libc::_exit(USER_INTERRUPT_EXIT_CODE);
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
    /// Write [`FORCE_EXIT_NOTICE`] and end the process with
    /// [`USER_INTERRUPT_EXIT_CODE`].
    ForceExit,
}

/// Resolve a process-wide press count against the registered compose run.
#[cfg_attr(unix, allow(dead_code))]
pub(crate) fn classify_console_interrupt(
    count: u8,
    wait_loop_active: bool,
) -> ComposeInterruptEffect {
    let Some(notice) = lock_notice().clone() else {
        return ComposeInterruptEffect::Inactive;
    };
    match press_rung(count, wait_loop_active) {
        PressRung::Notice => ComposeInterruptEffect::Notice(notice),
        PressRung::Defer => ComposeInterruptEffect::Defer,
        PressRung::ForceExit => ComposeInterruptEffect::ForceExit,
    }
}

/// Windows counterpart of the Unix SIGINT handler, called from the process-wide
/// console-control handler with that handler's process-wide press count.
///
/// Writes straight to stderr rather than through the synchronized render sink,
/// for the reason `termination::windows::emit_interrupt_feedback` documents at
/// length: the console handler is a context-free `extern "system" fn(u32)`
/// Windows invokes on its own thread, while the sink is per-run state behind an
/// `Arc<Mutex<…>>` in the call chain. The payload is a single pre-rendered
/// buffer written in one `write_all` and `Stderr` locks internally, so the
/// bytes cannot interleave with another writer's; what it costs is that the
/// sink's cursor bookkeeping does not learn about this line.
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
    let effect = classify_console_interrupt(count, crate::output::wait_loop_active());
    if effect == ComposeInterruptEffect::Inactive {
        return;
    }
    crate::output::mark_user_interrupted();
    match effect {
        ComposeInterruptEffect::Inactive | ComposeInterruptEffect::Defer => {}
        ComposeInterruptEffect::Notice(bytes) => {
            let _ = std::io::Write::write_all(&mut std::io::stderr(), &bytes);
        }
        ComposeInterruptEffect::ForceExit => {
            let _ = std::io::Write::write_all(&mut std::io::stderr(), FORCE_EXIT_NOTICE);
            force_exit();
        }
    }
}

/// End the process without running destructors — the Windows analog of the Unix
/// handler's `_exit`.
///
/// `ExitProcess` is used rather than `std::process::exit` because the caller is
/// a console-handler thread while the main thread is, by construction, wedged
/// in the synchronous call the user is trying to escape: running atexit
/// handlers and destructors from here is exactly the wedge being escaped.
#[cfg(windows)]
fn force_exit() -> ! {
    unsafe { windows::Win32::System::Threading::ExitProcess(USER_INTERRUPT_EXIT_CODE as u32) }
}

/// Present so the ladder above compiles — and is therefore testable — on hosts
/// whose console handler does not exist. Nothing calls
/// [`on_console_interrupt`] there.
#[cfg(not(windows))]
fn force_exit() -> ! {
    std::process::exit(USER_INTERRUPT_EXIT_CODE)
}

/// Build the rendered interrupt notice (with a leading newline so the
/// terminal's echoed `^C` does not share a line) for async-signal-safe
/// `libc::write(2)` emission from the SIGINT handler.
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
        .and_then(|p| p.canonicalize().ok())
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
        assert_eq!(press_rung(1, false), PressRung::Notice);
        assert_eq!(
            press_rung(1, true),
            PressRung::Notice,
            "a wait loop must not suppress the one-and-only notice"
        );
    }

    #[test]
    fn a_repeat_press_force_exits_only_when_no_wait_loop_owns_the_ladder() {
        assert_eq!(press_rung(2, false), PressRung::ForceExit);
        assert_eq!(press_rung(2, true), PressRung::Defer);
        assert_eq!(press_rung(9, false), PressRung::ForceExit);
        assert_eq!(press_rung(9, true), PressRung::Defer);
    }

    #[test]
    #[serial_test::serial]
    fn a_press_with_no_compose_run_registered_is_inert() {
        assert_eq!(
            classify_console_interrupt(1, false),
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
                classify_console_interrupt(1, false),
                ComposeInterruptEffect::Notice(bytes)
            );
        }
        assert_eq!(
            classify_console_interrupt(1, false),
            ComposeInterruptEffect::Inactive,
            "a press after the compose subcommand returned must not act on it"
        );
    }

    #[test]
    #[serial_test::serial]
    fn a_registered_run_defers_or_force_exits_on_the_second_press() {
        let _registration = NoticeRegistration::install(notice_bytes());

        assert_eq!(
            classify_console_interrupt(2, true),
            ComposeInterruptEffect::Defer
        );
        assert_eq!(
            classify_console_interrupt(2, false),
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
}
