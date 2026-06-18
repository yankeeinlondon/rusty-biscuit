use super::*;

/// Consumes any pending events in crossterm's input buffer without
/// blocking.
///
/// Used at the tail of a prompt to discard orphan terminal responses
/// (notably DSR cursor-position replies) that would otherwise leak
/// into the next reader of `/dev/tty`. crossterm's parser handles
/// non-public events like [`crossterm::event::Event::CursorPosition`]
/// by reading and discarding them, so a `poll(0)` + `read` loop is
/// sufficient — we simply discard whatever it returns.
pub(super) fn drain_pending_events() {
    while let Ok(true) = event::poll(std::time::Duration::ZERO) {
        if event::read().is_err() {
            break;
        }
    }
}

/// Prepares the terminal for raw-mode TUI rendering.
///
/// Enables raw mode, enters the alternate screen when `fullscreen` is
/// true, and attempts to push kitty keyboard enhancement flags so
/// that modifier-only key events are reported.
///
/// Returns `Ok(true)` when the keyboard enhancement flags were
/// successfully pushed. The caller must pass this flag back to
/// [`restore_terminal`] so the flags are popped symmetrically.
pub(super) fn prepare_terminal(fullscreen: bool) -> io::Result<bool> {
    enable_raw_mode()?;
    let mut out: Stdout = io::stdout();
    if fullscreen {
        execute!(out, EnterAlternateScreen)?;
    }
    // REPORT_ALL_KEYS_AS_ESCAPE_CODES is the kitty-protocol flag that
    // makes terminals emit press/release events for *bare* modifier
    // keys (Ctrl, Alt, Shift, Super) when no other key is involved.
    // Without it, most kitty-aware terminals (WezTerm in particular)
    // only emit modifier events as part of a chord — which means
    // holding bare Ctrl never produces a key event and the
    // hotkey-badge UX silently does nothing. REPORT_EVENT_TYPES is
    // still required so press/release are distinguishable;
    // DISAMBIGUATE_ESCAPE_CODES keeps Esc from looking like a CSI
    // prefix.
    let kbd_pushed = execute!(
        out,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
        )
    )
    .is_ok();
    out.flush().ok();
    Ok(kbd_pushed)
}

/// Restores the terminal after a standalone prompt.
///
/// Pops keyboard enhancement flags when `kbd_pushed` is true, leaves
/// the alternate screen when `fullscreen` is true, and disables raw
/// mode.
pub(super) fn restore_terminal(fullscreen: bool, kbd_pushed: bool) {
    if kbd_pushed {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    if fullscreen {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
    let _ = disable_raw_mode();
}

/// Guard that calls [`restore_terminal`] on drop.
///
/// Ensures the terminal is restored even when the caller panics.
pub(super) struct TerminalGuard {
    fullscreen: bool,
    kbd_pushed: bool,
    done: bool,
}

impl TerminalGuard {
    pub(super) fn new(fullscreen: bool, kbd_pushed: bool) -> Self {
        Self {
            fullscreen,
            kbd_pushed,
            done: false,
        }
    }

    pub(super) fn dismiss(&mut self) {
        self.done = true;
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.done {
            restore_terminal(self.fullscreen, self.kbd_pushed);
        }
    }
}

/// Redirects fd 1 (`stdout`) to `/dev/tty` for the duration of a
/// standalone prompt when the process's real stdout is not a TTY but
/// stderr still is — the shell command-substitution shape
/// (`FOO=$(question ...)`).
///
/// This is required because `crossterm::cursor::position` — which
/// `ratatui` calls implicitly when constructing an inline-viewport
/// terminal — writes the DSR (`ESC[6n`) cursor-position query to
/// `io::stdout()` directly. If stdout is a pipe, the query never
/// reaches the terminal and crossterm times out with "The cursor
/// position could not be read within a normal duration".
///
/// When *both* stdout and stderr are piped (the test-harness shape:
/// `assert_cmd` / `cargo nextest`) the redirect stays inactive so the
/// subprocess does not leak ANSI escape sequences onto `/dev/tty`,
/// which would otherwise interleave with the parent runner's own
/// terminal output and corrupt its progress display.
///
/// On `Drop` the original stdout file descriptor is restored, so the
/// caller's result-emitting `println!` / `writeln!` calls reach the
/// captured pipe as expected.
///
/// On non-Unix platforms this is a no-op.
#[cfg(unix)]
pub(super) struct StdoutTtyRedirect {
    saved_fd: Option<libc::c_int>,
    tty_fd: Option<libc::c_int>,
}

#[cfg(unix)]
impl StdoutTtyRedirect {
    /// Activates the redirect when stdout is not a TTY.
    ///
    /// Returns an inactive guard (no-op on drop) when stdout is
    /// already a TTY, when stderr is *also* not a TTY (the test-harness
    /// shape: both stdio streams captured by a parent — leaking ANSI
    /// to `/dev/tty` would corrupt the parent's own progress display),
    /// when `/dev/tty` cannot be opened, or when any of the
    /// `dup`/`dup2` calls fail. In those failure modes the caller
    /// still proceeds — they will see the underlying error (e.g.
    /// crossterm's cursor-position timeout) rather than a confusing
    /// redirect-related failure.
    ///
    /// The stderr check distinguishes the two piped-stdout scenarios:
    /// shell command substitution (`FOO=$(question ...)`) inherits
    /// stderr from the terminal, so the redirect activates and the
    /// user still sees the prompt; under `assert_cmd` / `cargo nextest`
    /// stderr is also piped, so the redirect stays disabled and the
    /// subprocess does not interleave ANSI with the parent runner's
    /// own terminal output.
    pub(super) fn activate_if_piped() -> Self {
        if io::stdout().is_terminal() || !io::stderr().is_terminal() {
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        // SAFETY: passing a static, NUL-terminated C string and
        // standard libc flags. The returned fd is checked for error.
        let tty_fd = unsafe { libc::open(c"/dev/tty".as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC) };
        if tty_fd < 0 {
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        // SAFETY: STDOUT_FILENO is a valid open fd at process start.
        let saved_fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
        if saved_fd < 0 {
            unsafe { libc::close(tty_fd) };
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        // SAFETY: tty_fd is open; STDOUT_FILENO is valid.
        if unsafe { libc::dup2(tty_fd, libc::STDOUT_FILENO) } < 0 {
            unsafe {
                libc::close(saved_fd);
                libc::close(tty_fd);
            }
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        Self {
            saved_fd: Some(saved_fd),
            tty_fd: Some(tty_fd),
        }
    }
}

#[cfg(unix)]
impl Drop for StdoutTtyRedirect {
    fn drop(&mut self) {
        // Flush any TUI bytes still buffered in Rust's stdout handle
        // BEFORE the fd swap, so they land on /dev/tty rather than
        // leaking into the caller's captured pipe.
        let _ = io::stdout().flush();
        if let Some(saved) = self.saved_fd.take() {
            // SAFETY: saved is an fd we created via dup; STDOUT_FILENO
            // is valid.
            unsafe {
                libc::dup2(saved, libc::STDOUT_FILENO);
                libc::close(saved);
            }
        }
        if let Some(tty) = self.tty_fd.take() {
            unsafe { libc::close(tty) };
        }
    }
}
