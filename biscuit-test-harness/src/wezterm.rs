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
use std::ffi::OsString;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::{wait_for_prompt, CapturedFrame, TerminalHarness};

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

    /// Spawns a login shell (`$SHELL`, `bash`, or `sh`) in a fresh
    /// WezTerm pane and waits for the shell prompt to appear.
    ///
    /// The cargo target directory containing `bt` and `question` is
    /// prepended to `PATH` so CLI binaries resolve without an absolute
    /// path.
    pub fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("WezTerm not available"));
        }
        let shell = super::detect_shell();
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "spawn", "--new-window", "--"]);
        cmd.arg(&shell);
        cmd.arg("-l");

        if let Some(bin_dir) = super::cargo_bin_dir("bt").or_else(|| super::cargo_bin_dir("question")) {
            let current_path = env::var_os("PATH").unwrap_or_default();
            let mut new_path = OsString::from(bin_dir);
            new_path.push(":");
            new_path.push(current_path);
            cmd.env("PATH", new_path);
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
        wait_for_prompt(self)?;
        Ok(())
    }

    /// Returns a unique window title we stamp on the spawned WezTerm
    /// window so System Events can target it precisely. Includes the
    /// pane id (already unique within a WezTerm instance) so concurrent
    /// runs of the same harness don't collide.
    fn unique_window_title(&self) -> String {
        format!("biscuit-test-pane-{}", self.pane_id())
    }

    /// Brings the spawned pane to the foreground inside WezTerm AND
    /// raises **its specific window** to the macOS front. Required
    /// before any OS-level keyboard injection (e.g. `cliclick`).
    ///
    /// The naive approach — `tell application "WezTerm" to activate` —
    /// fails when the developer has many WezTerm windows already open:
    /// macOS resolves the activation against whichever WezTerm window
    /// was most recently `keyWindow`, which is rarely the brand-new
    /// pane we just spawned. We instead:
    ///
    /// 1. Stamp the new window with a unique title via
    ///    `wezterm cli set-window-title --pane-id N <unique>`.
    /// 2. Activate the pane internally (`wezterm cli activate-pane`).
    /// 3. Use `osascript` + System Events to `AXRaise` the window
    ///    whose title matches the unique stamp — leaving every other
    ///    WezTerm window the developer had open exactly where it was.
    ///
    /// Step 3 requires Accessibility permission for whatever app is
    /// running the test (Terminal.app, iTerm2, WezTerm itself, etc.).
    /// Without it, `AXRaise` silently fails and the event injection
    /// falls back to whatever window currently owns key focus — i.e.
    /// the same broken behaviour as before. We surface this as an
    /// `io::Error` so the failure mode is explicit.
    pub fn focus_spawned_pane(&self) -> io::Result<Option<(i32, i32)>> {
        let id = self.pane_id();
        let title = self.unique_window_title();

        // 1. Stamp a unique title we can target precisely.
        let out = Command::new("wezterm")
            .args(["cli", "set-tab-title", "--pane-id", id, &title])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli set-tab-title failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        // 2. Activate intra-WezTerm pane focus.
        let out = Command::new("wezterm")
            .args(["cli", "activate-pane", "--pane-id", id])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli activate-pane failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        // 3. Raise *our* window via System Events (macOS only).
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("osascript")
                .args(["-e", "tell application \"WezTerm\" to activate"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            std::thread::sleep(Duration::from_millis(150));

            let script = format!(
                r#"with timeout of 5 seconds
                       tell application "System Events"
                           set wtProcs to (every process whose name contains "wezterm")
                           if (count of wtProcs) is 0 then
                               error "no wezterm-like process found in System Events"
                           end if
                           set seenTitles to ""
                           repeat with p in wtProcs
                               try
                                   set hits to windows of p whose title contains "{title}"
                                   if (count of hits) is 0 then
                                       set hits to windows of p whose title is "question"
                                   end if
                                   if (count of hits) > 0 then
                                       set targetWin to item 1 of hits
                                       perform action "AXRaise" of targetWin
                                       set frontmost of p to true
                                       set winPos to position of targetWin
                                       set winSize to size of targetWin
                                       return ((item 1 of winPos) as text) & " " & ¬
                                              ((item 2 of winPos) as text) & " " & ¬
                                              ((item 1 of winSize) as text) & " " & ¬
                                              ((item 2 of winSize) as text)
                                   end if
                                   repeat with w in (windows of p)
                                       try
                                           set seenTitles to seenTitles & "  - " & (title of w) & linefeed
                                       end try
                                   end repeat
                               end try
                           end repeat
                           error "no wezterm window matched {title} or \"question\"; visible titles:" & linefeed & seenTitles
                       end tell
                   end timeout"#,
            );
            let activate = Command::new("osascript")
                .args(["-e", &script])
                .stderr(Stdio::piped())
                .output()?;
            if !activate.status.success() {
                return Err(io::Error::other(format!(
                    "System Events AXRaise of WezTerm window {title:?} failed (is \
                     Accessibility permission granted to the parent terminal app?): \
                     {}",
                    String::from_utf8_lossy(&activate.stderr).trim()
                )));
            }

            let stdout = String::from_utf8_lossy(&activate.stdout);
            let parts: Vec<i32> = stdout
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() == 4 {
                let (x, y, w, h) = (parts[0], parts[1], parts[2], parts[3]);
                let click_x = x + w / 2;
                let click_y = y + h / 2;
                eprintln!(
                    "[focus_spawned_pane] WezTerm window pos=({x},{y}) size=({w},{h}) → click target ({click_x},{click_y})"
                );
                std::thread::sleep(Duration::from_millis(200));
                return Ok(Some((click_x, click_y)));
            }
        }

        std::thread::sleep(Duration::from_millis(400));
        Ok(None)
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
