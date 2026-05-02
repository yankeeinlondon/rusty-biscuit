//! Kitty-driven [`TerminalHarness`].
//!
//! Kitty's remote-control protocol is opt-in via `allow_remote_control
//! yes` in `kitty.conf` plus either a `--listen-on` flag or running
//! the test inside a kitty session that exports `KITTY_LISTEN_ON`.
//! When that surface isn't reachable we [`available`](Self::available)
//! returns `false` and dependent tests skip cleanly.
//!
//! The harness shells out to `kitty @` (alias for `kitty +kitten
//! send-text` etc.) for control. We deliberately do not start our own
//! kitty GUI — these tests must run inside an existing kitty session
//! to be useful.

#![allow(dead_code)]

use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::{CapturedFrame, SpawnVisibility, TerminalHarness, wait_for_prompt};

/// Harness that drives a running kitty GUI via `kitty @`.
pub struct KittyHarness {
    window_id: Option<String>,
    spawn_visibility: SpawnVisibility,
}

impl KittyHarness {
    /// Returns a fresh harness with [`SpawnVisibility::Background`].
    pub fn new() -> Self {
        Self {
            window_id: None,
            spawn_visibility: SpawnVisibility::default(),
        }
    }

    /// Builder-style override of the default
    /// [`SpawnVisibility::Background`]. Use
    /// [`SpawnVisibility::Foreground`] for tests that need the kitty
    /// window to receive focus immediately on spawn.
    pub fn with_spawn_visibility(mut self, visibility: SpawnVisibility) -> Self {
        self.spawn_visibility = visibility;
        self
    }

    /// Returns `true` when the `kitty` binary is on `$PATH` and a
    /// listen socket is reachable. Inside a kitty shell the GUI sets
    /// `KITTY_LISTEN_ON` automatically when remote control is enabled.
    pub fn available() -> bool {
        which("kitty") && env::var_os("KITTY_LISTEN_ON").is_some()
    }

    fn window_id(&self) -> &str {
        self.window_id
            .as_deref()
            .expect("KittyHarness::spawn_shell must be called before send_text/capture")
    }

    /// Returns the active window's column count (cells wide).
    ///
    /// Wraps `kitty @ ls` and locates the spawned window by its id,
    /// returning the `columns` field from the window entry. Mirrors
    /// [`crate::wezterm::WezTermHarness::pane_size`] for the column
    /// dimension, which is what Level-2 image and diagram tests need
    /// for geometry assertions.
    ///
    /// ## Errors
    ///
    /// Returns an error when `kitty @ ls` fails, the JSON cannot be
    /// parsed, or the spawned window id is not present in the listing.
    pub fn pane_cols(&self) -> io::Result<u32> {
        let want = self.window_id().to_string();
        let out = Command::new("kitty")
            .args(["@", "ls"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "kitty @ ls failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let entries: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| io::Error::other(format!("kitty @ ls: invalid json: {e}")))?;
        let want_id: u64 = want
            .parse()
            .map_err(|e| io::Error::other(format!("kitty window id {want:?} not a u64: {e}")))?;
        let os_windows = entries
            .as_array()
            .ok_or_else(|| io::Error::other("kitty @ ls: top level not an array"))?;
        for os_win in os_windows {
            let tabs = match os_win.get("tabs").and_then(|t| t.as_array()) {
                Some(t) => t,
                None => continue,
            };
            for tab in tabs {
                let windows = match tab.get("windows").and_then(|w| w.as_array()) {
                    Some(w) => w,
                    None => continue,
                };
                for win in windows {
                    let wid = win.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    if wid == want_id {
                        // Kitty exposes columns directly on the window
                        // entry. Older versions used `columns`; current
                        // versions use the same key. Try both.
                        if let Some(cols) = win
                            .get("columns")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32)
                        {
                            return Ok(cols);
                        }
                        if let Some(cols) =
                            win.get("cols").and_then(|v| v.as_u64()).map(|v| v as u32)
                        {
                            return Ok(cols);
                        }
                        return Err(io::Error::other(
                            "kitty @ ls: window entry missing columns/cols field",
                        ));
                    }
                }
            }
        }
        Err(io::Error::other(format!(
            "kitty window id {want} not found in kitty @ ls"
        )))
    }

    fn close_window(&mut self) {
        if let Some(id) = self.window_id.take() {
            let _ = Command::new("kitty")
                .args(["@", "close-window", "--match", &format!("id:{id}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

impl Default for KittyHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KittyHarness {
    fn drop(&mut self) {
        self.close_window();
    }
}

impl TerminalHarness for KittyHarness {
    /// Spawns a login shell (`$SHELL`, `bash`, or `sh`) in a fresh
    /// kitty window and waits for the shell prompt to appear.
    ///
    /// The cargo target directory containing `bt` and `question` is
    /// prepended to `PATH` so CLI binaries resolve without an absolute
    /// path. Color-forcing env vars are applied so SGR output is
    /// deterministic.
    fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("kitty remote control not available"));
        }
        let shell = super::detect_shell();
        let mut cmd = Command::new("kitty");
        cmd.args(["@", "launch", "--type=window", "--no-response=false"]);
        if self.spawn_visibility == SpawnVisibility::Background {
            cmd.arg("--keep-focus");
        }
        cmd.arg("--");
        cmd.arg(&shell);
        cmd.arg("-l");

        if let Some(bin_dir) =
            super::cargo_bin_dir("bt").or_else(|| super::cargo_bin_dir("question"))
        {
            let current_path = env::var_os("PATH").unwrap_or_default();
            let mut new_path = OsString::from(bin_dir);
            new_path.push(":");
            new_path.push(current_path);
            cmd.env("PATH", new_path);
        }

        // Force color on the spawned shell so `bt`'s color detection is
        // deterministic regardless of how the test runner inherits TTY
        // state.
        super::apply_color_forcing_env(&mut cmd);

        let out = cmd.output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "kitty @ launch failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let window_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if window_id.is_empty() {
            return Err(io::Error::other("kitty @ launch returned empty window id"));
        }
        self.window_id = Some(window_id);
        wait_for_prompt(self)?;
        Ok(())
    }

    /// Direct-spawn escape hatch: launches `program` with `args` in a
    /// fresh kitty window via `kitty @ launch -- …`. No shell, no
    /// `PATH` augmentation, no prompt-readiness wait.
    fn spawn_program(&mut self, program: &str, args: &[&str]) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("kitty remote control not available"));
        }
        let mut cmd = Command::new("kitty");
        cmd.args(["@", "launch", "--type=window", "--no-response=false"]);
        if self.spawn_visibility == SpawnVisibility::Background {
            cmd.arg("--keep-focus");
        }
        cmd.arg("--");
        cmd.arg(program);
        for a in args {
            cmd.arg(a);
        }
        let out = cmd.output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "kitty @ launch failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let window_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if window_id.is_empty() {
            return Err(io::Error::other("kitty @ launch returned empty window id"));
        }
        self.window_id = Some(window_id);
        std::thread::sleep(Duration::from_millis(400));
        Ok(())
    }

    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
        let id = self.window_id().to_string();
        let mut child = Command::new("kitty")
            .args([
                "@",
                "send-text",
                "--match",
                &format!("id:{id}"),
                "--from-file",
                "/dev/stdin",
            ])
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
                "kitty @ send-text failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }

    fn capture(&mut self) -> io::Result<CapturedFrame> {
        let id = self.window_id().to_string();
        let out = Command::new("kitty")
            .args([
                "@",
                "get-text",
                "--match",
                &format!("id:{id}"),
                "--extent=screen",
                "--ansi",
            ])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "kitty @ get-text failed: {}",
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
