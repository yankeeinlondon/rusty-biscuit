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

use super::{wait_for_prompt, CapturedFrame, TerminalHarness};

/// Harness that drives a running kitty GUI via `kitty @`.
pub struct KittyHarness {
    window_id: Option<String>,
}

impl KittyHarness {
    pub fn new() -> Self {
        Self { window_id: None }
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
            .expect("KittyHarness::spawn must be called before send_text/capture")
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

    /// Spawns a login shell (`$SHELL`, `bash`, or `sh`) in a fresh
    /// kitty window and waits for the shell prompt to appear.
    ///
    /// The cargo target directory containing `bt` and `question` is
    /// prepended to `PATH` so CLI binaries resolve without an absolute
    /// path.
    pub fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("kitty remote control not available"));
        }
        let shell = super::detect_shell();
        let mut cmd = Command::new("kitty");
        cmd.args(["@", "launch", "--type=window", "--no-response=false", "--"]);
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
    fn spawn(&mut self, program: &str, args: &[&str]) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("kitty remote control not available"));
        }
        let mut cmd = Command::new("kitty");
        cmd.args(["@", "launch", "--type=window", "--no-response=false", "--"]);
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
