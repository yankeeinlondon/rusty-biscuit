//! Process-scoped SIGINT (Ctrl+C) handling for composition commands.
//!
//! The guard is installed at the top of the compose / inline-compose run so
//! it covers the entire prep window, not just the loop. Downstream surfaces
//! branch on the process-scoped `USER_INTERRUPTED` flag.

/// Exit code emitted when Ctrl+C is observed during a compose run.
/// Matches the standard `128 + SIGINT(2)` convention used by shells.
pub(crate) const USER_INTERRUPT_EXIT_CODE: i32 = 130;

/// RAII guard returned by [`install_user_interrupt_guard`]. Drops the
/// underlying `signal_hook` registration when the compose subcommand
/// returns, restoring whatever handler was previously installed.
pub(crate) struct UserInterruptGuard {
    #[cfg(unix)]
    _hook: Option<signal_hook::SigId>,
}

/// Install a process-scoped SIGINT handler that covers the **entire**
/// compose / inline-compose run — including the slow prep phase before
/// the loop is entered. The handler:
///
/// - Marks the process-scoped `USER_INTERRUPTED` flag so any downstream
///   surface (loop executor, live semantic sink, post-prep checkpoints)
///   can branch on it.
/// - Writes a pre-rendered INFO notice to stderr exactly once via
///   async-signal-safe `libc::write(2)`. The notice has a leading `\n`
///   so it lands at column 1 (off the terminal's echoed `^C`) and the
///   prompt is rendered as an OSC8 hyperlink whose visible text is the
///   user's CLI argument verbatim.
///
/// `signal_hook::low_level::register` stacks handlers, so this one
/// composes cleanly with the per-iteration SIGINT handler the wrapper
/// installs around each agent child.
pub(crate) fn install_user_interrupt_guard(prompt_argv: &str) -> UserInterruptGuard {
    let bytes = std::sync::Arc::new(format_user_interrupt_message(prompt_argv).into_bytes());
    let printed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    #[cfg(unix)]
    {
        let bytes_handler = std::sync::Arc::clone(&bytes);
        let printed_handler = std::sync::Arc::clone(&printed);
        let hook = unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                crate::output::mark_user_interrupted();
                // Print our notice exactly once. `write(2)` on a file
                // descriptor is async-signal-safe; the Rust stdio
                // macros (`eprintln!`, `println!`) are not, and any
                // allocation or `tracing` call would be unsafe here.
                if !printed_handler.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    let buf = bytes_handler.as_slice();
                    libc::write(
                        libc::STDERR_FILENO,
                        buf.as_ptr() as *const libc::c_void,
                        buf.len(),
                    );
                }
            })
        }
        .ok();
        UserInterruptGuard { _hook: hook }
    }
    #[cfg(not(unix))]
    {
        let _ = (bytes, printed);
        UserInterruptGuard {}
    }
}

/// Build the rendered interrupt notice (with a leading newline so the
/// terminal's echoed `^C` does not share a line) for async-signal-safe
/// `libc::write(2)` emission from the SIGINT handler.
///
/// At install time we have only the user's CLI argument (e.g. the
/// relative path they typed). We use that verbatim as the OSC8 visible
/// text, and best-effort canonicalise it against the current working
/// directory to produce an absolute `file://` link target. If
/// canonicalisation fails (path doesn't exist yet, permission denied,
/// etc.) we fall back to a plain (non-hyperlinked) prose line.
pub(crate) fn format_user_interrupt_message(prompt_argv: &str) -> String {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::components::status::{Status, StatusState};

    let absolute = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join(prompt_argv))
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.display().to_string());

    let prose = if let Some(absolute) = absolute {
        format!("User interrupted compose operation in [{prompt_argv}](file://{absolute})")
    } else {
        format!("User interrupted compose operation in <yellow>{prompt_argv}</yellow>")
    };

    let term = crate::log::terminal();
    let body = Status::from_prose(prose)
        .state(StatusState::Info)
        .render(&term);

    format!("\n{body}")
}
