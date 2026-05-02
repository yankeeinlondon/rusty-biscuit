//! WezTerm-driven [`TerminalHarness`].
//!
//! Uses the `wezterm cli` subcommand to spawn the binary in a fresh
//! pane of the running WezTerm GUI, send synthetic text, and capture
//! the rendered pane text. Requires:
//!
//! - The `wezterm` binary on `$PATH`.
//! - A running WezTerm GUI we can reach (`WEZTERM_UNIX_SOCKET` env
//!   set — the GUI exports this for child shells).
//!
//! When either is missing, [`available`](Self::available) returns
//! `false` and tests that depend on this harness early-return with a
//! "skipping: requires WezTerm" message.

#![allow(dead_code)]

use std::env;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::{CapturedFrame, TerminalHarness};

/// Harness that talks to a running WezTerm GUI via `wezterm cli`.
pub struct WezTermHarness {
    pane_id: Option<String>,
}

impl WezTermHarness {
    /// Returns a fresh harness. Call [`spawn`](TerminalHarness::spawn)
    /// to actually create a pane.
    pub fn new() -> Self {
        Self { pane_id: None }
    }

    /// Returns `true` when both the `wezterm` binary and a reachable
    /// WezTerm GUI socket are available.
    pub fn available() -> bool {
        env::var_os("WEZTERM_UNIX_SOCKET").is_some() && which("wezterm")
    }

    /// Borrows the active pane id, panicking with a clear message when
    /// no pane has been spawned yet.
    fn pane_id(&self) -> &str {
        self.pane_id
            .as_deref()
            .expect("WezTermHarness::spawn must be called before send_text/capture")
    }

    /// Kills the current pane via `wezterm cli kill-pane`, ignoring
    /// any error (best effort).
    fn kill_pane(&mut self) {
        if let Some(id) = self.pane_id.take() {
            let _ = Command::new("wezterm")
                .args(["cli", "kill-pane", "--pane-id", &id])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

    /// Brings the spawned pane to the foreground inside WezTerm
    /// (`wezterm cli activate-pane --pane-id N`) AND raises the
    /// WezTerm app to the front of the macOS window stack via
    /// `osascript`.
    ///
    /// Required before any OS-level keyboard injection (e.g.
    /// `cliclick`) — without this, key events land in whatever
    /// window the user happened to have focused, which is almost
    /// never the spawned pane in a `cargo test` run.
    pub fn focus_spawned_pane(&self) -> io::Result<()> {
        let id = self.pane_id();
        let out = Command::new("wezterm")
            .args(["cli", "activate-pane", "--pane-id", id])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli activate-pane failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        // Activating the pane handles intra-WezTerm focus; raise the
        // app itself so cliclick events route to WezTerm rather than
        // whatever app currently owns key focus.
        let activate = Command::new("osascript")
            .args(["-e", "tell application \"WezTerm\" to activate"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !activate.success() {
            return Err(io::Error::other("osascript activate WezTerm failed"));
        }
        // Allow the WindowServer to settle focus before the caller
        // injects keys.
        std::thread::sleep(Duration::from_millis(400));
        Ok(())
    }
}

impl Default for WezTermHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WezTermHarness {
    fn drop(&mut self) {
        self.kill_pane();
    }
}

impl TerminalHarness for WezTermHarness {
    fn spawn(&mut self, program: &str, args: &[&str]) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("WezTerm not available"));
        }
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "spawn", "--new-window", "--"]);
        cmd.arg(program);
        for a in args {
            cmd.arg(a);
        }
        let out = cmd.output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli spawn failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let pane_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if pane_id.is_empty() {
            return Err(io::Error::other("wezterm cli spawn returned empty pane id"));
        }
        self.pane_id = Some(pane_id);
        // Give the binary a moment to render its first frame before
        // tests start poking at it.
        std::thread::sleep(Duration::from_millis(400));
        Ok(())
    }

    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
        let id = self.pane_id().to_string();
        let mut child = Command::new("wezterm")
            .args(["cli", "send-text", "--pane-id", &id, "--no-paste"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(bytes)?;
        }
        let out = child.wait_with_output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli send-text failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }

    fn capture(&mut self) -> io::Result<CapturedFrame> {
        let id = self.pane_id().to_string();
        // `--escapes` returns the pane content with ANSI sequences
        // intact, which lets us verify SGR styling when needed.
        let out = Command::new("wezterm")
            .args(["cli", "get-text", "--pane-id", &id, "--escapes"])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli get-text failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        Ok(CapturedFrame::from_raw(raw))
    }
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
