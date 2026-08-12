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
use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::{
    CAPTURE_TIMEOUT, CLEANUP_TIMEOUT, CapturedFrame, QUERY_TIMEOUT, SEND_TIMEOUT, SPAWN_TIMEOUT,
    SpawnVisibility, TerminalHarness, run_with_stdin_timeout, run_with_timeout, wait_for_prompt,
};

/// Environment variable read by [`KittyHarness::shared_or_spawn`] to
/// attach to a window that was pre-spawned by an outer process (e.g.
/// the `_test_l2` recipe via `biscuit-harness-broker`).
pub const SHARED_WINDOW_ENV: &str = "BISCUIT_SHARED_KITTY_WINDOW_ID";

/// Harness that drives a running kitty GUI via `kitty @`.
pub struct KittyHarness {
    window_id: Option<String>,
    spawn_visibility: SpawnVisibility,
    /// When `true`, [`Drop`] closes the kitty window. When `false`
    /// (set by [`KittyHarness::attach`]) the window is left alone.
    owned: bool,
}

impl KittyHarness {
    /// Returns a fresh harness with [`SpawnVisibility::Background`].
    pub fn new() -> Self {
        Self {
            window_id: None,
            spawn_visibility: SpawnVisibility::default(),
            owned: true,
        }
    }

    /// Returns a harness that references an existing kitty window by id
    /// without taking ownership of its lifecycle. [`Drop`] is a no-op.
    pub fn attach(window_id: impl Into<String>) -> Self {
        Self {
            window_id: Some(window_id.into()),
            spawn_visibility: SpawnVisibility::default(),
            owned: false,
        }
    }

    /// If [`SHARED_WINDOW_ENV`] is set, returns an
    /// [`attach`](Self::attach)-style harness pointing at the
    /// pre-spawned window. Otherwise spawns a fresh window (owned).
    ///
    /// ## Errors
    ///
    /// Propagates whatever [`spawn_shell`](TerminalHarness::spawn_shell)
    /// returns when no shared window id is available.
    pub fn shared_or_spawn() -> io::Result<Self> {
        if let Ok(id) = env::var(SHARED_WINDOW_ENV) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return Ok(Self::attach(trimmed));
            }
        }
        let mut h = Self::new();
        h.spawn_shell()?;
        Ok(h)
    }

    /// Builder-style override of the default
    /// [`SpawnVisibility::Background`]. Use
    /// [`SpawnVisibility::Foreground`] for tests that need the kitty
    /// window to receive focus immediately on spawn.
    ///
    /// ## Invariant
    ///
    /// L2 (shared) tests MUST keep the default
    /// [`SpawnVisibility::Background`] so test runs do not steal focus
    /// from the developer's foreground app. Only L3 tests (which own
    /// their own per-test harness and need OS keyboard injection)
    /// should override to [`SpawnVisibility::Foreground`]. Attached
    /// (shared-window) harnesses panic from this setter in debug
    /// builds because the broker-spawned window was already created in
    /// background mode and cannot be re-spawned with a different
    /// visibility.
    pub fn with_spawn_visibility(mut self, visibility: SpawnVisibility) -> Self {
        debug_assert!(
            self.owned,
            "with_spawn_visibility on an attached (shared) KittyHarness has no effect — \
             the window was already spawned by biscuit-harness-broker in Background mode",
        );
        self.spawn_visibility = visibility;
        self
    }

    /// Returns `true` when the `kitty` binary is on `$PATH` and a
    /// listen socket is reachable. Inside a kitty shell the GUI sets
    /// `KITTY_LISTEN_ON` automatically when remote control is enabled.
    pub fn available() -> bool {
        which("kitty") && env::var_os("KITTY_LISTEN_ON").is_some()
    }

    /// Returns the active window id (panicking if the harness has not
    /// spawned or attached yet).
    pub fn window_id_str(&self) -> &str {
        self.window_id()
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
        let mut cmd = Command::new("kitty");
        cmd.args(["@", "ls"]);
        let out = run_with_timeout(&mut cmd, QUERY_TIMEOUT)?;
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
            let mut cmd = Command::new("kitty");
            cmd.args(["@", "close-window", "--match", &format!("id:{id}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
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
        if self.owned {
            self.close_window();
        }
    }
}

/// Closes the kitty window with the given id by shelling out to
/// `kitty @ close-window`. Used by `biscuit-harness-broker kill` to tear
/// down shared windows whose `KittyHarness` was leaked in a different
/// process. Best-effort: any error is swallowed.
pub fn close_window_by_id(window_id: &str) {
    let mut cmd = Command::new("kitty");
    cmd.args(["@", "close-window", "--match", &format!("id:{window_id}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
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
        let bin_dir = super::cargo_bin_dir("bt").or_else(|| super::cargo_bin_dir("question"));
        super::configure_login_shell(&mut cmd, &shell, bin_dir.as_deref());

        // Force color on the spawned shell so `bt`'s color detection is
        // deterministic regardless of how the test runner inherits TTY
        // state.
        super::apply_color_forcing_env(&mut cmd);

        let out = run_with_timeout(&mut cmd, SPAWN_TIMEOUT)?;
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
        let out = run_with_timeout(&mut cmd, SPAWN_TIMEOUT)?;
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
        let mut cmd = Command::new("kitty");
        cmd.args([
            "@",
            "send-text",
            "--match",
            &format!("id:{id}"),
            "--from-file",
            "/dev/stdin",
        ]);
        let out = run_with_stdin_timeout(&mut cmd, bytes, SEND_TIMEOUT)?;
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
        let mut cmd = Command::new("kitty");
        cmd.args([
            "@",
            "get-text",
            "--match",
            &format!("id:{id}"),
            "--extent=screen",
            "--ansi",
        ]);
        let out = run_with_timeout(&mut cmd, CAPTURE_TIMEOUT)?;
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
