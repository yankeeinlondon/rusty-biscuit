//! Shared real-terminal test harness for the rusty-biscuit monorepo.
//!
//! Provides [`TerminalHarness`] implementations for WezTerm, Kitty, and
//! tmux, plus common utilities like [`CapturedFrame`], [`strip_ansi`],
//! and [`skip_with_reason`].
//!
//! ## Testing vocabulary
//!
//! - **Level 1** — PTY-based tests. We generate input bytes; the binary
//!   parses them. No real terminal required.
//! - **Level 2** — Run-in-real-terminal with IPC. The binary renders
//!   through the real terminal's display path so glyph-width / SGR /
//!   scroll-handling regressions are observable in captured pane text.
//!   Input is injected as bytes via the terminal's CLI.
//! - **Level 3** — OS-level keyboard injection. The key event enters
//!   where a physical keypress enters, so the terminal's *input encoder*
//!   fires — the only way to observe what bytes a terminal emits for a
//!   given chord. One injector module per platform: [`cliclick`] (macOS
//!   Quartz events), [`xdotool`] (Linux X11 XTEST), and [`win_input`]
//!   (Windows `SendKeys`/`SendInput` via PowerShell).
//!
//! Every harness's — and every Level-3 injector's — `available()` probe
//! returns `false` cleanly when its required tooling or platform is
//! missing; tests then print `skipping: requires` and return `ok`
//! without exercising it. [`cliclick::available`] is the one exception:
//! it only proves the binary is on `$PATH`, checking neither the host
//! platform nor macOS Accessibility trust. Pair it with
//! [`cliclick::accessibility_trusted`], which covers both.

#![allow(dead_code)]

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub mod apple_terminal;
pub mod bin_exe;
pub mod cliclick;
pub mod kitty;
pub mod layout_invariants;
pub mod shared;
pub mod tmux;
pub mod wezterm;
pub mod win_input;
pub mod xdotool;

/// Prefix used to mark resources owned by this test harness.
///
/// Backends include the process id immediately after the backend-specific
/// prefix, which lets later test runs clean resources from dead test
/// processes without touching an active concurrent run.
pub const HARNESS_RESOURCE_PREFIX: &str = "biscuit-test-";

/// Controls whether a freshly spawned harness window is allowed to
/// take keyboard focus / be visible on the active desktop.
///
/// The default is [`SpawnVisibility::Background`] so that running
/// real-terminal tests does not interrupt a developer working on the
/// same machine. Tests that need OS-level keyboard injection
/// (`cliclick`, etc.) must opt in with [`SpawnVisibility::Foreground`]
/// before calling
/// [`WezTermHarness::focus_spawned_pane`](wezterm::WezTermHarness::focus_spawned_pane).
///
/// ## How each backend honors this
///
/// - **WezTerm `Background`** — spawns into a dedicated workspace
///   (`biscuit-bg`) other than the user's active workspace. The window
///   is created but not visible until the user switches workspaces.
/// - **WezTerm `Foreground`** — no workspace override; the window
///   appears on the active desktop and may briefly take focus.
/// - **Kitty `Background`** — passes `--keep-focus` to
///   `kitty @ launch` so the new window is created without stealing
///   the active window's focus.
/// - **Kitty `Foreground`** — no `--keep-focus`; standard launch.
/// - **tmux** — N/A (tmux runs detached; visibility is irrelevant).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpawnVisibility {
    /// Spawn the window without grabbing focus or appearing on the
    /// active desktop. Default for both harnesses.
    #[default]
    Background,
    /// Spawn the window in the active workspace / with focus. Required
    /// before any OS-level keyboard injection can target it.
    Foreground,
}

/// Result returned by any [`TerminalHarness`] when it captures the
/// rendered pane text plus useful metadata.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    /// Text content of the pane, including ANSI sequences when the
    /// underlying tool returns them.
    pub raw: String,
    /// Plain-text content with ANSI/CSI sequences stripped.
    pub plain: String,
}

impl CapturedFrame {
    pub fn from_raw(raw: String) -> Self {
        let plain = strip_ansi(&raw);
        Self { raw, plain }
    }
}

/// Common contract every real-terminal harness implements.
///
/// The contract is **shell-model first**: callers spawn a fresh login
/// shell with [`spawn_shell`](Self::spawn_shell), then drive the binary
/// under test through [`send_text`](Self::send_text) — typically by
/// writing `bt some-command\n` (or `question some-command\n`) into the
/// shell's stdin. The cargo target directory is prepended to `PATH`
/// inside the shell, so CLI binaries resolve without an absolute path.
///
/// [`spawn_program`](Self::spawn_program) is a lower-level escape hatch
/// for tests that must launch a binary directly (no enclosing shell,
/// no prompt-readiness, no PATH augmentation). It defaults to
/// returning [`io::ErrorKind::Unsupported`]; implementors override it
/// when direct-spawn is meaningful for that backend.
///
/// Implementors are responsible for sending synthetic input as raw
/// bytes, capturing the rendered pane text, and tearing the session
/// down on drop.
///
/// ## Shell portability
///
/// The harness selects a POSIX shell (`bash`/`sh`) by default — see
/// [`detect_shell`] — so POSIX commands sent via [`send_text`] always
/// parse, regardless of the developer's login shell. However,
/// `send_text` itself remains a raw byte channel; tests that need to
/// scope environment variables to a single command should prefer
/// [`send_command_with_env`](Self::send_command_with_env), which uses
/// the portable `KEY=value command` inline-assignment syntax and
/// shell-escapes values safely.
pub trait TerminalHarness {
    /// Spawns a fresh login shell inside a new pane / window and waits
    /// for the shell prompt to appear.
    ///
    /// The cargo target directory containing the workspace's CLI
    /// binaries (`bt`, `question`, …) is prepended to `PATH` so tests
    /// can invoke `send_text(b"bt …\n")` without an absolute path.
    /// Color-forcing env vars ([`apply_color_forcing_env`]) are applied
    /// so SGR output is deterministic regardless of the test runner's
    /// TTY state.
    ///
    /// ## Errors
    ///
    /// Returns an error when the underlying terminal CLI rejects the
    /// spawn request (missing tooling, no reachable GUI socket, etc.).
    fn spawn_shell(&mut self) -> io::Result<()>;

    /// Sends raw bytes to the spawned pane's stdin.
    ///
    /// Use this for terminal escape sequences (Ctrl+chord, kitty
    /// keyboard-protocol bytes, etc.). Note: the terminal's *input
    /// encoder* is bypassed — you are writing what would normally
    /// appear on the pane's stdin.
    ///
    /// ## Notes
    ///
    /// `send_text` is a raw byte channel. The bytes are interpreted by
    /// whichever shell the harness spawned (see [`detect_shell`]).
    /// The harness defaults to bash/sh, but if a future override
    /// selects a non-POSIX shell, POSIX-isms like `export VAR=val` or
    /// inline `KEY=val cmd` will not parse correctly. Tests that need
    /// to scope env vars to a single command should prefer
    /// [`send_command_with_env`](Self::send_command_with_env).
    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Sends a single shell command preceded by inline environment
    /// variable assignments using the POSIX `KEY=value command` syntax.
    ///
    /// Because the harness defaults to bash/sh (see [`detect_shell`]),
    /// inline assignment is portable. Values are shell-escaped using
    /// single quotes; embedded single quotes are escaped with the
    /// `'\''` POSIX trick.
    ///
    /// `cmd` is the full command line (e.g. `"bt prose \"...\""`),
    /// without a trailing newline. `env` is a slice of `(name, value)`
    /// pairs.
    ///
    /// Equivalent to typing `KEY1='v1' KEY2='v2' cmd\n` into the shell
    /// and waiting for the harness to settle.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// harness.send_command_with_env(
    ///     "bt prose \"<red>x</red>\"",
    ///     &[("NO_COLOR", "1")],
    /// )?;
    /// ```
    ///
    /// ## Errors
    ///
    /// Propagates whatever error [`send_text`](Self::send_text) returns.
    fn send_command_with_env(&mut self, cmd: &str, env: &[(&str, &str)]) -> io::Result<()> {
        use std::fmt::Write as _;
        let mut line = String::new();
        for (k, v) in env {
            // POSIX single-quote escape: replace each `'` with `'\''`.
            let escaped = v.replace('\'', "'\\''");
            // Writing into a String never fails; ignore the Result.
            let _ = write!(line, "{k}='{escaped}' ");
        }
        line.push_str(cmd);
        line.push('\n');
        self.send_text(line.as_bytes())?;
        self.settle();
        Ok(())
    }

    /// Captures the current rendered text of the spawned pane.
    fn capture(&mut self) -> io::Result<CapturedFrame>;

    /// Sleeps long enough for the spawned process to react and the
    /// terminal to finish a redraw. Implementors override this when
    /// their tooling needs more (or less) settling time.
    fn settle(&self) {
        std::thread::sleep(Duration::from_millis(200));
    }

    /// Lower-level escape hatch: spawn a specific program directly in
    /// a fresh pane / window, bypassing the shell model.
    ///
    /// Default implementation returns [`io::ErrorKind::Unsupported`].
    /// Backends that can usefully launch a program directly (WezTerm,
    /// Kitty, tmux) override this with the equivalent of
    /// `wezterm cli spawn -- <program> <args>` /
    /// `kitty @ launch -- <program> <args>` /
    /// `tmux new-session -d <program> <args>`. There is no prompt-
    /// readiness wait, no `PATH` augmentation, and no color-forcing —
    /// callers are expected to supply absolute paths and handle their
    /// own settling.
    ///
    /// Most tests should prefer [`spawn_shell`](Self::spawn_shell) and
    /// drive the binary via `send_text`.
    ///
    /// ## Errors
    ///
    /// Returns [`io::ErrorKind::Unsupported`] by default. Overrides
    /// return whatever error the underlying terminal CLI yields.
    fn spawn_program(&mut self, _program: &str, _args: &[&str]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "spawn_program not implemented for this harness; use spawn_shell",
        ))
    }
}

/// Strips CSI / OSC / APC-DCS-PM / SGR / charset-designation escape sequences
/// from `s`, returning a plain-text rendering of the visible glyphs.
///
/// Used by `CapturedFrame::from_raw` so test assertions can match on
/// option labels without contending with embedded styling sequences.
///
/// Implements ECMA-48 §5.4 escape-sequence shape:
/// `ESC (intermediate 0x20-0x2F)* (final 0x30-0x7E)`. This matters
/// because terminals often emit `ESC ( B` (designate ASCII) around
/// styled regions; a naive "ESC + 1 byte" stripper would leave the
/// final `B` behind and let it masquerade as text.
pub fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == b'[' {
                // CSI: 0x1b '[' params* final
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (0x40..=0x7e).contains(&b) {
                        break;
                    }
                }
                continue;
            }
            if next == b']' {
                // OSC: 0x1b ']' ... BEL or ESC '\\'
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if b == 0x07 {
                        break;
                    }
                    if b == 0x1b && i < bytes.len() && bytes[i] == b'\\' {
                        i += 1;
                        break;
                    }
                }
                continue;
            }
            // String-terminator sequences: APC (`ESC _`, Kitty graphics), DCS
            // (`ESC P`), and PM (`ESC ^`) all run until ST (`ESC \`). Without
            // this, a Kitty image's base64 payload leaks into the visible-width
            // measurement (the payload has no other escape framing).
            if matches!(next, b'_' | b'P' | b'^') {
                i += 2;
                while i < bytes.len() {
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            // Generic ESC sequence: ESC (intermediate*) final.
            // Intermediate bytes are 0x20-0x2F, final byte is 0x30-0x7E.
            i += 1; // skip ESC
            while i < bytes.len() && (0x20..=0x2f).contains(&bytes[i]) {
                i += 1; // consume intermediate
            }
            if i < bytes.len() {
                i += 1; // consume final byte
            }
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Convenience: prints a "skipping: requires X" message to stderr and
/// returns `true` so tests can early-return.
///
/// ## Example
///
/// ```ignore
/// if !wezterm::WezTermHarness::available() {
///     return skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
/// }
/// ```
pub fn skip_with_reason(what: &str) -> bool {
    eprintln!("skipping: requires {what}");
    true
}

/// Detects a suitable shell for use as a generic command interpreter
/// in real-terminal tests.
///
/// Prefers POSIX shells (`bash` → `sh`) over the developer's `$SHELL`
/// so that POSIX commands sent via [`TerminalHarness::send_text`] always
/// parse, regardless of whether the user's login shell is fish, zsh,
/// xonsh, etc. Tests that need a non-POSIX shell's behavior must
/// override the harness directly.
///
/// ## Notes
///
/// The harness only uses the shell as a generic command interpreter.
/// Rcfile differences (`.bashrc` vs `.zshrc`) are irrelevant to the
/// portable subset of POSIX commands the test suite issues. The
/// preference order is: `bash` → `sh` → `$SHELL` → literal `"sh"`.
pub fn detect_shell() -> String {
    if which("bash") {
        return "bash".to_string();
    }
    if which("sh") {
        return "sh".to_string();
    }
    std::env::var("SHELL")
        .ok()
        .and_then(|s| {
            let path = std::path::Path::new(&s);
            path.file_name().map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "sh".to_string())
}

/// Returns the directory that contains the built cargo binary named
/// `bin_name`, if it can be located.
///
/// This is useful for prepending to `PATH` in shell-model tests so
/// that the CLI binary (e.g. `bt` or `question`) resolves without an
/// absolute path.
pub fn cargo_bin_dir(bin_name: &str) -> Option<PathBuf> {
    // Try the path returned by assert_cmd when available.
    // We avoid a direct dependency by probing CARGO_TARGET_DIR first.
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        let dir = PathBuf::from(target_dir);
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let bin = dir.join(profile).join(bin_name);
        if bin.exists() {
            return Some(bin.parent()?.to_path_buf());
        }
    }

    // Derive from the current executable location:
    // - test binary: <target_dir>/<profile>/deps/<exe> → <target_dir>/<profile>
    // - broker/bin:  <target_dir>/<profile>/<exe>      → <target_dir>/<profile>
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf();
    if dir.file_name().and_then(|n| n.to_str()) == Some("deps") {
        dir = dir.parent()?.to_path_buf();
    }
    let bin = dir.join(bin_name);
    if bin.exists() {
        return Some(dir);
    }
    None
}

/// Appends a login-shell invocation that preserves the Cargo binary directory.
///
/// Login startup files may replace `PATH`, particularly when a terminal GUI
/// launches from the desktop rather than an existing shell. The outer login
/// shell therefore prepends the binary directory after startup, then replaces
/// itself with the interactive shell used by the harness.
pub(crate) fn configure_login_shell(
    cmd: &mut Command,
    shell: &str,
    bin_dir: Option<&Path>,
) {
    cmd.arg(shell);
    if let Some(bin_dir) = bin_dir {
        cmd.args([
            "-l",
            "-c",
            "export PATH=\"$BISCUIT_TEST_BIN_DIR:$PATH\"; \
             unset BISCUIT_TEST_BIN_DIR; exec \"$0\" -i",
            shell,
        ]);
        cmd.env("BISCUIT_TEST_BIN_DIR", bin_dir);
    } else {
        cmd.arg("-l");
    }
}

/// Sets the conventional color-forcing env vars on `cmd` so the
/// spawned shell (and any process it execs, including `bt`) emits SGR
/// regardless of whether stdout is a TTY.
///
/// Sets:
///
/// - removes `NO_COLOR` — otherwise crossterm suppresses foreground /
///   background SGR even when force-color vars are present.
/// - `FORCE_COLOR=1` — honored by `bt` and most ecosystem crates.
/// - `CLICOLOR_FORCE=1` — alternative convention (`clicolors`,
///   `colored`).
/// - `TERM=xterm-256color` — only when `TERM` is not already set in
///   the parent env, so callers can override.
/// - `COLORTERM=truecolor` — only when `COLORTERM` is not already set
///   in the parent env.
///
/// ## Notes
///
/// Real-terminal harnesses spawn the shell via the host terminal's
/// `cli spawn`-style command. Env vars set on the *outer* `Command`
/// propagate to the spawned shell process, which exports them for
/// child processes (e.g. `bt`).
pub fn apply_color_forcing_env(cmd: &mut std::process::Command) {
    cmd.env_remove("NO_COLOR");
    cmd.env("FORCE_COLOR", "1");
    cmd.env("CLICOLOR_FORCE", "1");
    if std::env::var_os("TERM").is_none() {
        cmd.env("TERM", "xterm-256color");
    }
    if std::env::var_os("COLORTERM").is_none() {
        cmd.env("COLORTERM", "truecolor");
    }
}

/// Trailing glyphs that mark the end of an interactive shell prompt.
///
/// The POSIX trio (`$`, `#`, `%`) only covers a shell running its *stock*
/// prompt. The harness spawns a login shell, so it inherits whatever the host's
/// dotfiles install, and modern prompt generators end on a glyph instead:
/// starship / pure / spaceship default to `❯`, and assorted themes use `»` or
/// `λ`. Themes whose distinguishing glyph *leads* the prompt (oh-my-zsh's
/// `robbyrussell` `➜`) are not detectable this way and fall back to the timeout.
///
/// An unrecognized terminator is silent but expensive rather than fatal: it
/// costs [`wait_for_prompt`] its full 5 s budget and every [`capture_settled`]
/// its full 20 s budget instead of roughly one poll. That was enough to push
/// `level2_render_tree_style_in_tmux` (22 captures) past nextest's 30 s
/// termination ceiling on a host whose `~/.bashrc` initializes starship.
///
/// `>` is deliberately absent. It is a plausible trailing character of ordinary
/// rendered output, and reading one as "the prompt is back" settles a capture on
/// a half-drawn frame.
const PROMPT_TERMINATORS: &[char] = &['$', '#', '%', '❯', '»', 'λ'];

/// Returns `true` when `line` ends like an interactive shell prompt.
pub fn looks_like_prompt(line: &str) -> bool {
    line.trim_end().ends_with(PROMPT_TERMINATORS)
}

/// Returns `true` when the bottom-most non-blank line of a captured frame's
/// plain text ends like an interactive shell prompt.
///
/// ## Notes
///
/// tmux's `capture-pane` pads the capture with trailing blank lines, so the
/// literal last line is usually empty. Scanning only `lines().last()` never
/// matched the prompt and burned the full timeout on every tmux capture,
/// multiplied across the many `bt` invocations in a single test.
pub fn frame_shows_prompt(plain: &str) -> bool {
    plain
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .is_some_and(looks_like_prompt)
}

/// Waits for a shell prompt to appear in the harness output.
///
/// Polls [`TerminalHarness::capture`] every 100 ms until
/// [`frame_shows_prompt`] holds. Times out after 5 seconds and returns
/// silently so that callers don't hang.
pub fn wait_for_prompt(harness: &mut impl TerminalHarness) -> io::Result<()> {
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        let frame = match harness.capture() {
            Ok(f) => f,
            Err(_) => continue,
        };
        if frame_shows_prompt(&frame.plain) {
            return Ok(());
        }
    }
    Ok(())
}

/// Captures the pane repeatedly until its rendered output has *settled* —
/// two consecutive captures are byte-identical and a shell prompt is
/// visible — or `MAX` elapses, whichever comes first.
///
/// This is the adaptive replacement for the fixed post-command sleep dance
/// (`settle` + a blind [`wait_for_prompt`] poll + an explicit `sleep` + a
/// single `capture`). On a quiet machine it returns after the first stable
/// pair (~one poll interval); under load it keeps polling up to the budget
/// rather than capturing a half-drawn frame.
///
/// ## Notes
///
/// Callers reach here through [`TerminalHarness::send_command_with_env`], which
/// already spent a [`TerminalHarness::settle`] interval after pressing Enter.
/// Without that head start the pre-command prompt would still be the
/// bottom-most line and a stable pair could settle before the command ran.
pub fn capture_settled(harness: &mut impl TerminalHarness) -> io::Result<CapturedFrame> {
    const POLL: Duration = Duration::from_millis(40);
    const MAX: Duration = Duration::from_secs(20);
    let deadline = Instant::now() + MAX;
    let mut prev: Option<String> = None;
    loop {
        let frame = harness.capture()?;
        if frame_shows_prompt(&frame.plain) && prev.as_deref() == Some(frame.raw.as_str()) {
            return Ok(frame);
        }
        if Instant::now() >= deadline {
            return Ok(frame);
        }
        prev = Some(frame.raw.clone());
        std::thread::sleep(POLL);
    }
}

/// Best-effort cleanup for stale resources owned by the real-terminal
/// harnesses.
///
/// This is safe to call before a test starts. Tagged resources are removed
/// only when their owning process id is no longer alive. Backend-specific
/// cleanup functions intentionally ignore errors so missing terminal tools or
/// unavailable GUI automation never make tests fail.
pub fn cleanup_stale_terminal_harness_resources() {
    wezterm::cleanup_stale_wezterm_panes();
    tmux::cleanup_stale_tmux_sessions();
    apple_terminal::cleanup_stale_apple_terminal_windows();
}

pub(crate) fn current_process_id() -> u32 {
    std::process::id()
}

pub(crate) fn process_is_alive(pid: u32) -> bool {
    if pid == current_process_id() {
        return true;
    }
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(crate) fn pid_from_tag(tag: &str, prefix: &str) -> Option<u32> {
    let rest = tag.strip_prefix(prefix)?;
    let pid = rest.split('-').next()?;
    pid.parse().ok()
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ------------------------------------------------------------------
// Timeout helpers
// ------------------------------------------------------------------

/// Default timeout for spawning a new terminal pane or window.
pub const SPAWN_TIMEOUT: Duration = Duration::from_secs(15);
/// Default timeout for sending text/keystrokes to a terminal.
pub const SEND_TIMEOUT: Duration = Duration::from_secs(10);
/// Default timeout for capturing pane or window contents.
pub const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);
/// Default timeout for query operations (list panes, geometry, etc.).
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(10);
/// Default timeout for cleanup operations (kill pane, close window).
pub const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

/// Runs `cmd` with a wall-clock timeout.
///
/// Internally polls [`Child::try_wait`](std::process::Child::try_wait)
/// every 50 ms. If the timeout is reached before the child exits, the
/// child is killed and a [`TimedOut`](io::ErrorKind::TimedOut) error
/// is returned.
///
/// ## Stderr / stdout
///
/// The command's stdout and stderr are captured into the returned
/// [`Output`](std::process::Output). If the command times out, neither
/// stream is available.
///
/// ## Pipe-deadlock safety
///
/// stdout and stderr are drained on background threads so that a
/// chatty child cannot deadlock itself by filling either pipe buffer
/// before the polling loop has a chance to reap it.
pub fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> io::Result<std::process::Output> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

    // Drain pipes on background threads to avoid pipe-buffer deadlock.
    let stdout_handle = child.stdout.take().map(|mut out| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = Read::read_to_end(&mut out, &mut buf);
            buf
        })
    });
    let stderr_handle = child.stderr.take().map(|mut err| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = Read::read_to_end(&mut err, &mut buf);
            buf
        })
    });

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(status) => {
                let stdout = stdout_handle
                    .map(|h| h.join().unwrap_or_default())
                    .unwrap_or_default();
                let stderr = stderr_handle
                    .map(|h| h.join().unwrap_or_default())
                    .unwrap_or_default();
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            None => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("command timed out after {:?}", timeout),
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Spawns `cmd`, writes `stdin_bytes` to its stdin, then waits for the
/// child to exit with a timeout.
///
/// This is the [`run_with_timeout`] equivalent for commands that need
/// data piped to their standard input (e.g. `wezterm cli send-text` or
/// `kitty @ send-text`).
pub fn run_with_stdin_timeout(
    cmd: &mut Command,
    stdin_bytes: &[u8],
    timeout: Duration,
) -> io::Result<std::process::Output> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_bytes)?;
        // Dropping stdin closes the pipe so the child sees EOF.
    }

    // Drain pipes on background threads to avoid pipe-buffer deadlock.
    let stdout_handle = child.stdout.take().map(|mut out| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = Read::read_to_end(&mut out, &mut buf);
            buf
        })
    });
    let stderr_handle = child.stderr.take().map(|mut err| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = Read::read_to_end(&mut err, &mut buf);
            buf
        })
    });

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(status) => {
                let stdout = stdout_handle
                    .map(|h| h.join().unwrap_or_default())
                    .unwrap_or_default();
                let stderr = stderr_handle
                    .map(|h| h.join().unwrap_or_default())
                    .unwrap_or_default();
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            None => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("command timed out after {:?}", timeout),
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_csi() {
        assert_eq!(strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
    }

    #[test]
    fn strip_ansi_removes_nested_sgr() {
        assert_eq!(strip_ansi("\x1b[1;38;5;202m^F\x1b[0m"), "^F");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn strip_ansi_removes_osc() {
        assert_eq!(strip_ansi("before\x1b]0;title\x07after"), "beforeafter");
    }

    #[test]
    fn strip_ansi_removes_charset_designation() {
        assert_eq!(strip_ansi("\x1b(BHello"), "Hello");
        assert_eq!(strip_ansi("a\x1b(Bb\x1b(0c"), "abc");
    }

    /// Locks in the shell-model trait contract: a harness that only
    /// implements the required methods inherits an
    /// `Unsupported`-returning [`TerminalHarness::spawn_program`].
    /// Tests that need direct-spawn must explicitly override it.
    #[test]
    fn trait_default_spawn_program_is_unsupported() {
        struct Stub;
        impl TerminalHarness for Stub {
            fn spawn_shell(&mut self) -> io::Result<()> {
                Ok(())
            }
            fn send_text(&mut self, _: &[u8]) -> io::Result<()> {
                Ok(())
            }
            fn capture(&mut self) -> io::Result<CapturedFrame> {
                Ok(CapturedFrame::from_raw(String::new()))
            }
        }
        let mut s = Stub;
        let err = s.spawn_program("x", &[]).expect_err("default must error");
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    /// Captures bytes sent through [`TerminalHarness::send_text`] so we
    /// can assert on the exact line constructed by
    /// [`TerminalHarness::send_command_with_env`].
    struct CapturingHarness {
        sent: Vec<u8>,
    }

    impl CapturingHarness {
        fn new() -> Self {
            Self { sent: Vec::new() }
        }

        fn sent_string(&self) -> String {
            String::from_utf8(self.sent.clone()).expect("sent bytes are utf-8")
        }
    }

    impl TerminalHarness for CapturingHarness {
        fn spawn_shell(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.sent.extend_from_slice(bytes);
            Ok(())
        }
        fn capture(&mut self) -> io::Result<CapturedFrame> {
            Ok(CapturedFrame::from_raw(String::new()))
        }
        // Override so unit tests do not actually sleep.
        fn settle(&self) {}
    }

    #[test]
    fn send_command_with_env_formats_inline_env() {
        let mut h = CapturingHarness::new();
        h.send_command_with_env(
            "bt prose \"<red>x</red>\"",
            &[("NO_COLOR", "1"), ("FORCE_COLOR", "0")],
        )
        .expect("send_command_with_env failed");
        assert_eq!(
            h.sent_string(),
            "NO_COLOR='1' FORCE_COLOR='0' bt prose \"<red>x</red>\"\n",
        );
    }

    #[test]
    fn send_command_with_env_handles_empty_env() {
        let mut h = CapturingHarness::new();
        h.send_command_with_env("bt image --debug 13x13.png", &[])
            .expect("send_command_with_env failed");
        assert_eq!(h.sent_string(), "bt image --debug 13x13.png\n");
    }

    #[test]
    fn send_command_with_env_escapes_single_quotes() {
        let mut h = CapturingHarness::new();
        h.send_command_with_env("echo hi", &[("MSG", "it's fine")])
            .expect("send_command_with_env failed");
        // POSIX trick: a single quote inside single-quoted string is
        // closed, escaped, and reopened: `'\''`.
        assert_eq!(h.sent_string(), "MSG='it'\\''s fine' echo hi\n");
    }

    /// Confirms the `bash`/`sh` preference: on any developer host where
    /// either binary is on PATH (the common case), `detect_shell()`
    /// must NOT return the user's `$SHELL` (e.g. fish, zsh).
    #[test]
    fn detect_shell_prefers_posix_over_user_shell() {
        let shell = detect_shell();
        if which("bash") {
            assert_eq!(shell, "bash", "expected bash preference; got {shell}");
        } else if which("sh") {
            assert_eq!(shell, "sh", "expected sh fallback; got {shell}");
        } else {
            // No POSIX shell at all on PATH — extremely unusual on a
            // developer host. Just confirm we returned *something* and
            // skip the strict assertion.
            assert!(!shell.is_empty());
        }
    }

    #[test]
    fn looks_like_prompt_accepts_posix_terminators() {
        assert!(looks_like_prompt("ken@host:~/src$ "));
        assert!(looks_like_prompt("bash-5.2# "));
        assert!(looks_like_prompt("host% "));
    }

    #[test]
    fn looks_like_prompt_accepts_prompt_generator_glyphs() {
        // starship / pure / spaceship, plus assorted glyph themes.
        assert!(looks_like_prompt("🐧 ❯ "));
        assert!(looks_like_prompt("~/src » "));
        assert!(looks_like_prompt("λ "));
    }

    #[test]
    fn looks_like_prompt_rejects_command_echo_and_output() {
        assert!(!looks_like_prompt("❯ bt block \"bordered notice\""));
        assert!(!looks_like_prompt("└──────────────┘"));
        assert!(!looks_like_prompt(""));
    }

    #[test]
    fn looks_like_prompt_rejects_trailing_angle_bracket() {
        // `>` is excluded on purpose: rendered output ending in it would
        // otherwise read as "the prompt is back" mid-draw.
        assert!(!looks_like_prompt("<div>"));
    }

    #[test]
    fn frame_shows_prompt_skips_trailing_blank_padding() {
        // tmux's `capture-pane` pads a short pane out to its full height.
        let frame = "❯ bt progress 50\n 50% Loading\n🐧 ❯ \n\n\n";
        assert!(frame_shows_prompt(frame));
    }

    #[test]
    fn frame_shows_prompt_is_false_while_a_command_is_still_running() {
        let frame = "🐧 ❯ bt table --columns Name,Score\n\n\n";
        assert!(!frame_shows_prompt(frame));
    }

    #[test]
    fn pid_from_tag_extracts_owner_pid() {
        assert_eq!(pid_from_tag("biscuit-test-pane-123-456", "biscuit-test-pane-"), Some(123));
    }

    #[test]
    fn pid_from_tag_rejects_wrong_prefix() {
        assert_eq!(pid_from_tag("other-123-456", "biscuit-test-pane-"), None);
    }

    #[test]
    fn pid_from_tag_rejects_invalid_pid() {
        assert_eq!(
            pid_from_tag("biscuit-test-pane-not-a-pid-456", "biscuit-test-pane-"),
            None,
        );
    }
}
