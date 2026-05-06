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
use std::time::Duration;

use super::{CapturedFrame, TerminalHarness, wait_for_prompt};

/// Harness that runs each spawned binary inside a fresh detached tmux
/// session.
pub struct TmuxHarness {
    session: Option<String>,
}

impl TmuxHarness {
    pub fn new() -> Self {
        Self { session: None }
    }

    /// Returns `true` when the `tmux` binary is on `$PATH`. tmux is
    /// fully self-contained — no parent terminal required.
    pub fn available() -> bool {
        which("tmux")
    }

    fn session(&self) -> &str {
        self.session
            .as_deref()
            .expect("TmuxHarness::spawn_shell must be called before send_text/capture")
    }

    fn kill_session(&mut self) {
        if let Some(name) = self.session.take() {
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", &name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
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
        let status = Command::new("tmux")
            .args(["send-keys", "-t", &session, key_name])
            .status()?;
        if !status.success() {
            return Err(io::Error::other("tmux send-keys (key-name) failed"));
        }
        Ok(())
    }
}

impl Default for TmuxHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TmuxHarness {
    fn drop(&mut self) {
        self.kill_session();
    }
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

        let status = cmd.status()?;
        if !status.success() {
            return Err(io::Error::other("tmux new-session failed"));
        }
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
        let status = cmd.status()?;
        if !status.success() {
            return Err(io::Error::other("tmux new-session failed"));
        }
        self.session = Some(session);
        std::thread::sleep(Duration::from_millis(400));
        Ok(())
    }

    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
        let session = self.session().to_string();
        let s = std::str::from_utf8(bytes)
            .map_err(|e| io::Error::other(format!("send_text non-utf8: {e}")))?;
        let status = Command::new("tmux")
            .args(["send-keys", "-t", &session, "-l", s])
            .status()?;
        if !status.success() {
            return Err(io::Error::other("tmux send-keys failed"));
        }
        Ok(())
    }

    fn capture(&mut self) -> io::Result<CapturedFrame> {
        let session = self.session().to_string();
        let out = Command::new("tmux")
            .args(["capture-pane", "-t", &session, "-p", "-e"])
            .output()?;
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
