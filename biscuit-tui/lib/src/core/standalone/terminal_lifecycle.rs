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

/// RAII guard that makes [`prepare_terminal`] transactional.
///
/// Armed immediately after raw mode is enabled. On `Drop` — unless
/// [`PrepareGuard::dismiss`] has been called — it disables raw mode
/// and, if the alternate screen was actually entered, leaves it. The
/// alternate-screen flag is set only after `EnterAlternateScreen`
/// succeeds, so teardown never emits `LeaveAlternateScreen` for a
/// screen the process never entered.
///
/// The guard is generic over its teardown actions so the transactional
/// logic is unit-testable without touching the real terminal:
/// production wires in the crossterm wrappers ([`leave_alt_screen_quiet`]
/// / [`disable_raw_mode_quiet`]); tests wire in counters.
pub(super) struct PrepareGuard<L: FnMut(), R: FnMut()> {
    alt_screen_entered: bool,
    dismissed: bool,
    leave_alt_screen: L,
    disable_raw_mode: R,
}

impl<L: FnMut(), R: FnMut()> PrepareGuard<L, R> {
    pub(super) fn arm(leave_alt_screen: L, disable_raw_mode: R) -> Self {
        Self {
            alt_screen_entered: false,
            dismissed: false,
            leave_alt_screen,
            disable_raw_mode,
        }
    }

    pub(super) fn note_alt_screen_entered(&mut self) {
        self.alt_screen_entered = true;
    }

    pub(super) fn dismiss(&mut self) {
        self.dismissed = true;
    }
}

impl<L: FnMut(), R: FnMut()> Drop for PrepareGuard<L, R> {
    fn drop(&mut self) {
        if self.dismissed {
            return;
        }
        if self.alt_screen_entered {
            (self.leave_alt_screen)();
        }
        (self.disable_raw_mode)();
    }
}

fn leave_alt_screen_quiet() {
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

fn disable_raw_mode_quiet() {
    let _ = disable_raw_mode();
}

/// Testable core of [`prepare_terminal`]: every fallible step is an
/// injected closure so the transactional unwind can be exercised
/// without a real terminal. [`prepare_terminal`] is the only
/// production caller; tests call this directly with fault injection
/// on the alt-screen step.
pub(super) fn prepare_terminal_inner<E, A, K, L, R>(
    fullscreen: bool,
    enable_raw_mode: E,
    enter_alt_screen: A,
    push_keyboard_flags: K,
    leave_alt_screen: L,
    disable_raw_mode: R,
) -> io::Result<bool>
where
    E: FnOnce() -> io::Result<()>,
    A: FnOnce(&mut Stdout) -> io::Result<()>,
    K: FnOnce(&mut Stdout) -> bool,
    L: FnMut(),
    R: FnMut(),
{
    enable_raw_mode()?;
    let mut guard = PrepareGuard::arm(leave_alt_screen, disable_raw_mode);
    let mut out: Stdout = io::stdout();
    if fullscreen {
        enter_alt_screen(&mut out)?;
        guard.note_alt_screen_entered();
    }
    let kbd_pushed = push_keyboard_flags(&mut out);
    out.flush().ok();
    guard.dismiss();
    Ok(kbd_pushed)
}

/// Prepares the terminal for raw-mode TUI rendering.
///
/// Enables raw mode, enters the alternate screen when `fullscreen` is
/// true, and attempts to push kitty keyboard enhancement flags so
/// that modifier-only key events are reported.
///
/// Raw mode is the first side effect and is unwound on any later
/// setup failure within this function via [`PrepareGuard`], so an
/// `Err` return never leaves the caller's shell in raw mode. On the
/// success path the guard is dismissed and the caller's
/// [`TerminalGuard`] takes ownership of teardown.
///
/// Returns `Ok(true)` when the keyboard enhancement flags were
/// successfully pushed. The caller must pass this flag back to
/// [`restore_terminal`] so the flags are popped symmetrically.
pub(super) fn prepare_terminal(fullscreen: bool) -> io::Result<bool> {
    prepare_terminal_inner(
        fullscreen,
        enable_raw_mode,
        |out| execute!(out, EnterAlternateScreen),
        |out| {
            // WHY: PushKeyboardEnhancementFlags is best-effort
            // (.is_ok()) and intentionally outside the PrepareGuard
            // unwind. If it is ever made fallible it must still run
            // after the guard is armed (already the case here) so a
            // failure unwinds raw mode rather than leaking it.
            //
            // REPORT_ALL_KEYS_AS_ESCAPE_CODES is the kitty-protocol
            // flag that makes terminals emit press/release events for
            // *bare* modifier keys (Ctrl, Alt, Shift, Super) when no
            // other key is involved. Without it, most kitty-aware
            // terminals (WezTerm in particular) only emit modifier
            // events as part of a chord — which means holding bare
            // Ctrl never produces a key event and the hotkey-badge UX
            // silently does nothing. REPORT_EVENT_TYPES is still
            // required so press/release are distinguishable;
            // DISAMBIGUATE_ESCAPE_CODES keeps Esc from looking like a
            // CSI prefix.
            execute!(
                out,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
                )
            )
            .is_ok()
        },
        leave_alt_screen_quiet,
        disable_raw_mode_quiet,
    )
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
/// The Windows counterpart (same name, `#[cfg(windows)]`) targets
/// `CONOUT$`; on any other non-Unix platform the redirect is a no-op.
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

// --- F2: Windows console redirect ------------------------------------
//
// On Windows the command-substitution shape (`FOO=$(question ...)`)
// redirects the process's STDOUT handle to a pipe. Unlike Unix, crossterm
// never hangs there: `cursor::position` opens `CONOUT$` directly (bypassing
// the std handle) and reads the cursor via `GetConsoleScreenBufferInfo`.
// The actual failure mode is that every TUI byte written through
// `io::stdout()` lands on the captured pipe instead of the console, so the
// user sees nothing and the captured stream is polluted with ANSI noise.

#[cfg(windows)]
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
#[cfg(windows)]
use windows_sys::Win32::System::Console::{GetStdHandle, SetStdHandle, STD_OUTPUT_HANDLE};

/// NUL-terminated UTF-16 encoding of `CONOUT$` — the device path that
/// resolves to the active console screen buffer even when the process's
/// standard handles have been redirected to pipes (the Windows analog of
/// `/dev/tty`).
#[cfg(windows)]
pub(super) const CONOUT: [u16; 8] = {
    let bytes = b"CONOUT$\0";
    let mut out = [0u16; 8];
    let mut i = 0;
    while i < bytes.len() {
        out[i] = bytes[i] as u16;
        i += 1;
    }
    out
};

/// Windows counterpart of the Unix [`StdoutTtyRedirect`](struct@StdoutTtyRedirect):
/// redirects `io::stdout()` to the real console (`CONOUT$`) for the
/// duration of a standalone prompt when the process's real stdout is a
/// pipe but stderr is still attached to a console.
///
/// Mechanism: open `CONOUT$` via `CreateFileW`, then `SetStdHandle` it
/// onto `STD_OUTPUT_HANDLE`. This works because Rust's `io::stdout()`
/// and crossterm's output writes resolve `STD_OUTPUT_HANDLE` *dynamically*
/// via `GetStdHandle` on every use (neither caches the handle value), so
/// `SetStdHandle` reroutes all subsequent stdout writes — TUI rendering
/// and ANSI escape sequences — onto the console.
///
/// Gating mirrors the Unix sibling: no-op when stdout is already a console,
/// and no-op when stderr is also captured (the test-harness shape).
///
/// On `Drop`, `io::stdout()` is flushed (so buffered bytes land on the
/// console before the handle swap), the original `STD_OUTPUT_HANDLE` is
/// restored via `SetStdHandle`, and our `CONOUT$` handle is closed. The
/// original std handle is restored but never closed — `GetStdHandle` does
/// not transfer ownership. Restore is exactly-once; partial-activation
/// error paths close the handle they opened.
#[cfg(windows)]
pub(super) struct StdoutTtyRedirect {
    saved_stdout: Option<HANDLE>,
    conout: Option<HANDLE>,
}

#[cfg(windows)]
impl StdoutTtyRedirect {
    /// Returns an inactive guard (no-op on drop).
    fn inactive() -> Self {
        Self {
            saved_stdout: None,
            conout: None,
        }
    }

    /// Activates the redirect when stdout is not a console but stderr is.
    ///
    /// Returns an inactive guard when stdout is already a console, when
    /// stderr is *also* captured (the test-harness shape — mod.rs already
    /// returned the explicit "no interactive terminal available" error
    /// before reaching here, so this guard is belt-and-suspenders), or
    /// when any Win32 call fails. In every failure mode the caller still
    /// proceeds and observes the underlying error (e.g. crossterm's
    /// position-query error) rather than a confusing redirect-related one.
    ///
    /// The "is stdout a console" check uses `is_terminal`, which on
    /// Windows is backed by `GetConsoleMode` — the robust console probe
    /// (it succeeds only for real console screen buffers), intentionally
    /// preferred over `GetFileType`, whose `FILE_TYPE_CHAR` classification
    /// is unreliable under ConPTY / pseudoconsoles.
    pub(super) fn activate_if_piped() -> Self {
        use std::io::IsTerminal;

        if io::stdout().is_terminal() || !io::stderr().is_terminal() {
            return Self::inactive();
        }
        // SAFETY: CONOUT is a static, NUL-terminated UTF-16 literal; the
        // access mode and disposition are standard documented values. The
        // returned handle is checked against INVALID_HANDLE_VALUE.
        let conout = unsafe {
            CreateFileW(
                CONOUT.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                core::ptr::null(),
                OPEN_EXISTING,
                0,
                core::ptr::null_mut(),
            )
        };
        if conout == INVALID_HANDLE_VALUE {
            return Self::inactive();
        }
        // SAFETY: STD_OUTPUT_HANDLE is a valid std-handle identifier.
        // The returned handle is the process's current stdout; it is not
        // owned by us and is never closed.
        let saved_stdout = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        if saved_stdout.is_null() || saved_stdout == INVALID_HANDLE_VALUE {
            // SAFETY: conout is the handle we just opened and have not yet
            // redirected onto STD_OUTPUT_HANDLE, so closing it fully undoes
            // the CreateFileW.
            unsafe { CloseHandle(conout) };
            return Self::inactive();
        }
        // SAFETY: STD_OUTPUT_HANDLE is valid; conout is the open console
        // handle from CreateFileW above. On success crossterm's output
        // writes and Rust's io::stdout() — both resolving the std handle
        // dynamically via GetStdHandle — now target CONOUT$.
        if unsafe { SetStdHandle(STD_OUTPUT_HANDLE, conout) } == 0 {
            // SAFETY: redirect never installed; closing conout undoes the
            // only side effect so far.
            unsafe { CloseHandle(conout) };
            return Self::inactive();
        }
        Self {
            saved_stdout: Some(saved_stdout),
            conout: Some(conout),
        }
    }
}

#[cfg(windows)]
impl Drop for StdoutTtyRedirect {
    fn drop(&mut self) {
        // Flush any TUI bytes still buffered in Rust's stdout handle
        // BEFORE the handle swap, so they land on the console rather than
        // leaking into the caller's captured pipe.
        let _ = io::stdout().flush();
        if let Some(saved) = self.saved_stdout.take() {
            // SAFETY: saved was captured from GetStdHandle before the
            // redirect was installed; restoring it is the symmetric inverse
            // of the SetStdHandle in activate_if_piped. It is NOT closed —
            // GetStdHandle returns a borrowed process handle, not an owned
            // one, so closing it would corrupt the caller's real stdout.
            unsafe { SetStdHandle(STD_OUTPUT_HANDLE, saved) };
        }
        if let Some(conout) = self.conout.take() {
            // SAFETY: conout is the handle we created via CreateFileW and
            // the Option::take above guarantees it is closed exactly once.
            // The std handle has already been restored to the original, so
            // closing our CONOUT$ handle does not affect the caller's
            // stdout.
            unsafe { CloseHandle(conout) };
        }
    }
}
