//! tmux-driven [`TerminalHarness`].
//!
//! Unlike WezTerm and Kitty, tmux is fully programmable in headless
//! mode — `tmux new-session -d` gives us a session we can drive and
//! capture without any GUI. The trade-off: tmux strips kitty
//! keyboard-protocol bytes by default, so it can verify chord-based
//! features (Ctrl+Space, Enter, navigation) but is *not* the right
//! harness for bare-modifier-press tests.
//!
//! tmux is the most portable harness — when WezTerm and Kitty are
//! both unavailable, tests should fall back to this for any feature
//! it can verify.

#![allow(dead_code)]

use std::ffi::OsString;
use std::io;
use std::process::{Command, Stdio};
use std::sync::Once;
use std::time::Duration;

use super::{
    CAPTURE_TIMEOUT, CLEANUP_TIMEOUT, CapturedFrame, QUERY_TIMEOUT, SEND_TIMEOUT, SPAWN_TIMEOUT,
    TerminalHarness, current_process_id, process_is_alive, run_with_timeout, wait_for_prompt,
};

const SESSION_PREFIX: &str = "biscuit_test_";
const SERVER_EXITED_UNEXPECTEDLY: &str = "server exited unexpectedly";
static CLEANUP_ONCE: Once = Once::new();

/// Spawns a detached login shell in a named tmux session.
///
/// A tmux client can lose the server-start race when several independent test
/// processes create sessions concurrently. Retrying that one explicit server
/// error is safe because a failed `new-session` created no named session; all
/// other failures remain immediate and retain tmux's diagnostic.
pub fn spawn_shell_session(session: &str, cols: u32, rows: u32) -> io::Result<()> {
    spawn_shell_session_with_env(session, cols, rows, &[])
}

/// [`spawn_shell_session`] with variables applied to the tmux launch environment.
pub fn spawn_shell_session_with_env(
    session: &str,
    cols: u32,
    rows: u32,
    environment: &[(&str, &str)],
) -> io::Result<()> {
    let shell_cmd = format!("{} -l", super::detect_shell());
    let mut cmd = Command::new("tmux");
    cmd.args([
        "new-session",
        "-d",
        "-s",
        session,
        "-x",
        &cols.to_string(),
        "-y",
        &rows.to_string(),
        &shell_cmd,
    ]);
    cmd.envs(environment.iter().copied());
    run_new_session(&mut cmd)
}

fn run_new_session(cmd: &mut Command) -> io::Result<()> {
    for attempt in 0..2 {
        let out = run_with_timeout(cmd, SPAWN_TIMEOUT)?;
        if out.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&out.stderr);
        if should_retry_server_exit(attempt, &stderr) {
            std::thread::yield_now();
            continue;
        }

        return Err(io::Error::other(format!(
            "tmux new-session failed: {}",
            stderr.trim()
        )));
    }

    unreachable!("the retry loop always returns")
}

fn should_retry_server_exit(attempt: usize, stderr: &str) -> bool {
    attempt == 0 && stderr.contains(SERVER_EXITED_UNEXPECTEDLY)
}

/// Environment variable read by [`TmuxHarness::shared_or_spawn`] to
/// attach to a tmux session that was pre-spawned by an outer process
/// (e.g. the `_test_l2` recipe via `biscuit-harness-broker`).
pub const SHARED_SESSION_ENV: &str = "BISCUIT_SHARED_TMUX_SESSION";

/// Harness that runs each spawned binary inside a fresh detached tmux
/// session.
pub struct TmuxHarness {
    session: Option<String>,
    /// When `true`, [`Drop`] kills the tmux session. When `false`
    /// (set by [`TmuxHarness::attach`]) the session is left alone.
    owned: bool,
}

impl TmuxHarness {
    pub fn new() -> Self {
        Self {
            session: None,
            owned: true,
        }
    }

    /// Returns a harness that references an existing tmux session by
    /// name without taking ownership of its lifecycle. [`Drop`] is a
    /// no-op.
    pub fn attach(session: impl Into<String>) -> Self {
        Self {
            session: Some(session.into()),
            owned: false,
        }
    }

    /// If [`SHARED_SESSION_ENV`] is set, returns an
    /// [`attach`](Self::attach)-style harness pointing at the
    /// pre-spawned session. Otherwise spawns a fresh session (owned).
    ///
    /// ## Errors
    ///
    /// Propagates whatever [`spawn_shell`](TerminalHarness::spawn_shell)
    /// returns when no shared session is available.
    pub fn shared_or_spawn() -> io::Result<Self> {
        if let Ok(name) = std::env::var(SHARED_SESSION_ENV) {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return Ok(Self::attach(trimmed));
            }
        }
        let mut h = Self::new();
        h.spawn_shell()?;
        Ok(h)
    }

    /// Returns `true` when the `tmux` binary is on `$PATH`. tmux is
    /// fully self-contained — no parent terminal required.
    pub fn available() -> bool {
        which("tmux")
    }

    /// Returns the active session name (panicking if the harness has
    /// not spawned or attached yet).
    pub fn session_name(&self) -> &str {
        self.session()
    }

    /// Returns the pane's text width in columns, queried live from tmux.
    ///
    /// Reading `#{pane_width}` rather than assuming the spawn geometry keeps
    /// callers correct whether the session was spawned by this harness
    /// (120 columns) or pre-spawned by `biscuit-harness-broker`.
    ///
    /// ## Errors
    ///
    /// Returns an error when the `tmux display-message` query fails or its
    /// output is not a column count.
    pub fn pane_cols(&self) -> io::Result<u32> {
        let session = self.session().to_string();
        let mut cmd = Command::new("tmux");
        cmd.args(["display-message", "-p", "-t", &session, "#{pane_width}"]);
        let out = run_with_timeout(&mut cmd, QUERY_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "tmux display-message failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .map_err(|e| io::Error::other(format!("tmux pane_width not a u32: {e}")))
    }

    fn session(&self) -> &str {
        self.session
            .as_deref()
            .expect("TmuxHarness::spawn_shell must be called before send_text/capture")
    }

    fn kill_session(&mut self) {
        if let Some(name) = self.session.take() {
            let mut cmd = Command::new("tmux");
            cmd.args(["kill-session", "-t", &name])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
        }
    }

    /// Sends a tmux key-name (e.g. `"C-space"`, `"M-Tab"`, `"Enter"`).
    ///
    /// Unlike [`send_text`](TerminalHarness::send_text) — which uses
    /// `-l` for literal byte input — this routes through tmux's
    /// key-translation layer so callers can name chords symbolically.
    /// Use this for `Ctrl+Space` / `Alt+Space` and similar control
    /// chords, which contain bytes (NUL) that the underlying
    /// `Command::arg` API rejects when passed as raw text.
    pub fn send_key(&mut self, key_name: &str) -> io::Result<()> {
        let session = self.session().to_string();
        let mut cmd = Command::new("tmux");
        cmd.args(["send-keys", "-t", &session, key_name]);
        let out = run_with_timeout(&mut cmd, SEND_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other("tmux send-keys (key-name) failed"));
        }
        Ok(())
    }

    /// Resizes the session's window (and therefore its pane) to
    /// `cols`×`rows` characters.
    ///
    /// Detached harness sessions are fixed at their creation size, so a
    /// program that reads its winsize via `TIOCGWINSZ` always sees
    /// 120×40 until this is called. Switching the session to manual
    /// sizing first makes `resize-window` stick with no client attached;
    /// a program spawned afterwards then observes the new dimensions.
    ///
    /// ## Errors
    ///
    /// Returns an error if either `tmux` invocation fails to run or
    /// `resize-window` reports a non-zero status.
    pub fn resize(&mut self, cols: u32, rows: u32) -> io::Result<()> {
        let session = self.session().to_string();
        let mut set = Command::new("tmux");
        set.args(["set-option", "-t", &session, "window-size", "manual"]);
        run_with_timeout(&mut set, SEND_TIMEOUT)?;
        let mut cmd = Command::new("tmux");
        cmd.args([
            "resize-window",
            "-t",
            &session,
            "-x",
            &cols.to_string(),
            "-y",
            &rows.to_string(),
        ]);
        let out = run_with_timeout(&mut cmd, SEND_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "tmux resize-window failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
}

/// Removes stale tmux sessions created by earlier
/// [`TmuxHarness`] instances whose owning test process no longer exists.
///
/// Session names include the creating process id
/// (`biscuit_test_<pid>_<seq>`), so this avoids killing active sessions
/// from another concurrent test process.
pub fn cleanup_stale_tmux_sessions() {
    if !TmuxHarness::available() {
        return;
    }
    let mut cmd = Command::new("tmux");
    cmd.arg("ls");
    let Ok(out) = run_with_timeout(&mut cmd, Duration::from_secs(2)) else {
        return;
    };
    if !out.status.success() {
        return;
    }
    // Never reap the active shared session: the broker spawns it and exits,
    // so its name carries a dead pid and would otherwise be a reap target,
    // killing the session every attaching test depends on.
    let shared = std::env::var(SHARED_SESSION_ENV).ok();
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        let Some(name) = line.split(':').next() else {
            continue;
        };
        if shared.as_deref() == Some(name) {
            continue;
        }
        let Some(pid) = tmux_pid_from_session(name) else {
            continue;
        };
        if pid == current_process_id() || process_is_alive(pid) {
            continue;
        }
        let mut kill = Command::new("tmux");
        kill.args(["kill-session", "-t", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = run_with_timeout(&mut kill, CLEANUP_TIMEOUT);
    }
}

impl Default for TmuxHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TmuxHarness {
    fn drop(&mut self) {
        if self.owned {
            self.kill_session();
        }
    }
}

/// Kills the tmux session with the given name by shelling out to
/// `tmux kill-session`. Used by `biscuit-harness-broker kill` to tear
/// down shared sessions whose `TmuxHarness` was leaked in a different
/// process. Best-effort: any error is swallowed.
pub fn kill_session_by_name(session: &str) {
    let mut cmd = Command::new("tmux");
    cmd.args(["kill-session", "-t", session])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
}

impl TerminalHarness for TmuxHarness {
    /// Spawns a login shell (`$SHELL`, `bash`, or `sh`) in a fresh
    /// detached tmux session and waits for the shell prompt to appear.
    ///
    /// The cargo target directory containing `bt` and `question` is
    /// prepended to `PATH` so CLI binaries resolve without an absolute
    /// path. Color-forcing env vars are applied so SGR output is
    /// deterministic.
    fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("tmux not available"));
        }
        CLEANUP_ONCE.call_once(cleanup_stale_tmux_sessions);
        let session = unique_session_name();
        let shell = super::detect_shell();
        let shell_cmd = format!("{} -l", shell);

        let mut cmd = Command::new("tmux");
        cmd.args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "120",
            "-y",
            "40",
            &shell_cmd,
        ]);

        if let Some(bin_dir) =
            super::cargo_bin_dir("bt").or_else(|| super::cargo_bin_dir("question"))
        {
            let current_path = std::env::var_os("PATH").unwrap_or_default();
            let mut new_path = OsString::from(bin_dir);
            new_path.push(":");
            new_path.push(current_path);
            cmd.env("PATH", new_path);
        }

        // Force color on the spawned shell so `bt`'s color detection is
        // deterministic regardless of how the test runner inherits TTY
        // state.
        super::apply_color_forcing_env(&mut cmd);

        run_new_session(&mut cmd)?;
        self.session = Some(session);
        wait_for_prompt(self)?;
        Ok(())
    }

    /// Direct-spawn escape hatch: launches `program` with `args` as the
    /// initial command of a fresh detached tmux session. No login
    /// shell, no `PATH` augmentation, no prompt-readiness wait.
    fn spawn_program(&mut self, program: &str, args: &[&str]) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("tmux not available"));
        }
        CLEANUP_ONCE.call_once(cleanup_stale_tmux_sessions);
        let session = unique_session_name();
        let mut shell_cmd = shell_quote(program);
        for a in args {
            shell_cmd.push(' ');
            shell_cmd.push_str(&shell_quote(a));
        }
        let mut cmd = Command::new("tmux");
        cmd.args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "120",
            "-y",
            "40",
            &shell_cmd,
        ]);
        super::apply_color_forcing_env(&mut cmd);
        run_new_session(&mut cmd)?;
        self.session = Some(session);
        std::thread::sleep(Duration::from_millis(400));
        Ok(())
    }

    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
        let session = self.session().to_string();
        let s = std::str::from_utf8(bytes)
            .map_err(|e| io::Error::other(format!("send_text non-utf8: {e}")))?;
        let mut cmd = Command::new("tmux");
        cmd.args(["send-keys", "-t", &session, "-l", s]);
        let out = run_with_timeout(&mut cmd, SEND_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other("tmux send-keys failed"));
        }
        Ok(())
    }

    fn capture(&mut self) -> io::Result<CapturedFrame> {
        let session = self.session().to_string();
        let mut cmd = Command::new("tmux");
        cmd.args(["capture-pane", "-t", &session, "-p", "-e"]);
        let out = run_with_timeout(&mut cmd, CAPTURE_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "tmux capture-pane failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        Ok(CapturedFrame::from_raw(raw))
    }
}

/// Generates a unique tmux session name so concurrent test runs don't
/// collide. Mixes process id and a monotonic counter to keep cleanup
/// under control if the harness is dropped without `kill_session`.
fn unique_session_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("biscuit_test_{}_{n}", std::process::id())
}

fn tmux_pid_from_session(name: &str) -> Option<u32> {
    let rest = name.strip_prefix(SESSION_PREFIX)?;
    rest.split('_').next()?.parse().ok()
}

fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_only_the_first_unexpected_server_exit() {
        assert!(should_retry_server_exit(0, "server exited unexpectedly"));
        assert!(!should_retry_server_exit(1, "server exited unexpectedly"));
        assert!(!should_retry_server_exit(0, "duplicate session: fixture"));
    }

    #[test]
    fn tmux_pid_from_session_extracts_pid() {
        assert_eq!(tmux_pid_from_session("biscuit_test_123_0"), Some(123));
    }

    #[test]
    fn tmux_pid_from_session_rejects_wrong_prefix() {
        assert_eq!(tmux_pid_from_session("other_123_0"), None);
    }

    #[test]
    fn tmux_pid_from_session_rejects_invalid_pid() {
        assert_eq!(tmux_pid_from_session("biscuit_test_nope_0"), None);
    }
}
