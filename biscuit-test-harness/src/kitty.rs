//! Kitty-driven [`TerminalHarness`].
//!
//! Kitty's remote-control protocol is opt-in via `allow_remote_control
//! yes` in `kitty.conf` plus either a `--listen-on` flag or running
//! the test inside a kitty session that exports `KITTY_LISTEN_ON`.
//! When that surface isn't reachable we [`available`](Self::available)
//! returns `false` and dependent tests skip cleanly.
//!
//! The harness shells out to `kitty @` (alias for `kitty +kitten
//! send-text` etc.) for control. [`KittyHarness`] drives an existing kitty
//! session; [`KittyInstance`] starts a private, unfocused kitty (macOS) for
//! tests that need a known window size or a screenshot of what kitty drew.

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
    /// `--to` address for every `kitty @` call; `None` uses `KITTY_LISTEN_ON`.
    to: Option<String>,
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
            to: None,
            spawn_visibility: SpawnVisibility::default(),
            owned: true,
        }
    }

    /// Returns a harness that references an existing kitty window by id
    /// without taking ownership of its lifecycle. [`Drop`] is a no-op.
    pub fn attach(window_id: impl Into<String>) -> Self {
        Self {
            window_id: Some(window_id.into()),
            to: None,
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

    /// A `kitty @` command aimed at this harness's kitty instance.
    fn remote(&self) -> Command {
        remote_command(self.to.as_deref())
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
        let mut cmd = self.remote();
        cmd.arg("ls");
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
            let mut cmd = self.remote();
            cmd.args(["close-window", "--match", &format!("id:{id}")])
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
    /// deterministic. The interactive shell the harness drives runs with
    /// its rc files suppressed — see
    /// [`configure_login_shell`](super::configure_login_shell).
    fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("kitty remote control not available"));
        }
        let shell = super::detect_shell();
        let mut cmd = self.remote();
        cmd.args(["launch", "--type=window", "--no-response=false"]);
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
        let mut cmd = self.remote();
        cmd.args(["launch", "--type=window", "--no-response=false"]);
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
        let mut cmd = self.remote();
        cmd.args([
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
        self.capture_extent("screen")
    }
}

impl KittyHarness {
    /// [`capture`](TerminalHarness::capture) with `kitty @ get-text`'s
    /// `--extent` (`screen`, `all` for screen plus scrollback, ...).
    ///
    /// `get-text` returns a soft-wrapped line as one line, without trailing
    /// blanks, so a line longer than the window spans several screen rows.
    ///
    /// ## Errors
    ///
    /// Returns an error when `kitty @ get-text` fails.
    pub fn capture_extent(&self, extent: &str) -> io::Result<CapturedFrame> {
        let id = self.window_id().to_string();
        let mut cmd = self.remote();
        cmd.args([
            "get-text",
            "--match",
            &format!("id:{id}"),
            &format!("--extent={extent}"),
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

/// `kitty @`, addressed to `to` when given and to `KITTY_LISTEN_ON` otherwise.
fn remote_command(to: Option<&str>) -> Command {
    let mut cmd = Command::new("kitty");
    cmd.arg("@");
    if let Some(to) = to {
        cmd.args(["--to", to]);
    }
    cmd
}

/// Prefix of a [`KittyInstance`]'s socket directory; the owning pid follows.
const INSTANCE_DIR_PREFIX: &str = "biscuit-kitty-";

/// A private kitty GUI with its own remote-control socket and one OS window of
/// a fixed size, for tests that need a known pane geometry or a screenshot of
/// what kitty actually drew (images, not just cell text).
///
/// The instance never takes focus: it is started with `open -g`, and kitty
/// does not activate itself. Its window is visible and unfocused, because a
/// `--start-as=hidden` window is never drawn and screenshots of it are black.
/// macOS only; [`can_launch`](Self::can_launch) is `false` elsewhere.
///
/// [`Drop`] quits the instance. An instance whose test process died is quit by
/// the next [`launch`](Self::launch) on the host.
pub struct KittyInstance {
    /// Holds the socket; removed on drop.
    _dir: tempfile::TempDir,
    to: String,
    window_id: String,
    platform_window_id: u64,
}

impl KittyInstance {
    /// `true` on macOS when `kitty` and `screencapture` are on `$PATH`.
    pub fn can_launch() -> bool {
        cfg!(target_os = "macos") && which("kitty") && which("screencapture") && which("open")
    }

    /// Starts kitty with one `columns` × `lines` cell window running the
    /// harness's rc-suppressed login shell, and waits for its prompt.
    ///
    /// The default configuration (`--config NONE`) is used, so colors, font,
    /// and padding do not depend on the host's `kitty.conf`: the background
    /// is black and the window has no padding.
    ///
    /// ## Errors
    ///
    /// Returns an error off macOS, or when kitty does not start or its socket
    /// does not answer within [`SPAWN_TIMEOUT`].
    pub fn launch(columns: u32, lines: u32) -> io::Result<Self> {
        if !Self::can_launch() {
            return Err(io::Error::other("a private kitty instance needs macOS, kitty, and screencapture"));
        }
        cleanup_stale_kitty_instances();
        // `/tmp`, not `$TMPDIR`: a unix socket path is limited to 104 bytes.
        let dir = tempfile::Builder::new()
            .prefix(&format!("{INSTANCE_DIR_PREFIX}{}-", super::owner_process_id()))
            .tempdir_in("/tmp")?;
        let to = format!("unix:{}", dir.path().join("kitty.sock").display());

        let mut cmd = Command::new("open");
        // `-g` keeps kitty in the background; `-n` starts a new instance even
        // when the user already runs kitty. `open` passes this environment on.
        cmd.args(["-g", "-n", "-a", "kitty", "--args", "--config", "NONE"]);
        for option in [
            "allow_remote_control=socket-only".to_string(),
            "remember_window_size=no".to_string(),
            format!("initial_window_width={columns}c"),
            format!("initial_window_height={lines}c"),
            // Otherwise quitting with a shell running opens a confirmation window.
            "confirm_os_window_close=0".to_string(),
        ] {
            cmd.args(["-o", &option]);
        }
        cmd.args(["--listen-on", &to]);
        cmd.args(super::login_shell_argv(&super::detect_shell(), false));
        super::apply_color_forcing_env(&mut cmd);
        let out = run_with_timeout(&mut cmd, SPAWN_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "open -a kitty failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        let deadline = std::time::Instant::now() + SPAWN_TIMEOUT;
        let (window_id, platform_window_id) = loop {
            if let Some(ids) = first_window(&to) {
                break ids;
            }
            if std::time::Instant::now() > deadline {
                quit_instance(&to);
                return Err(io::Error::other("the private kitty instance never answered on its socket"));
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let instance = Self { _dir: dir, to, window_id, platform_window_id };
        wait_for_prompt(&mut instance.harness())?;
        Ok(instance)
    }

    /// A harness attached to the instance's window. It does not own the
    /// window; the instance closes it.
    pub fn harness(&self) -> KittyHarness {
        let mut harness = KittyHarness::attach(self.window_id.clone());
        harness.to = Some(self.to.clone());
        harness
    }

    /// Writes a PNG of the instance's OS window (title bar included) to
    /// `path`, without raising or focusing it.
    ///
    /// The capture's pixels are kitty's device pixels: the cell grid is
    /// `columns × cell width` wide, centered between equal side borders and
    /// flush with the bottom border.
    ///
    /// ## Errors
    ///
    /// Returns an error when `screencapture` fails. Without the Screen
    /// Recording permission for the calling terminal, macOS returns an image
    /// without the window's contents rather than an error.
    pub fn screenshot(&self, path: &std::path::Path) -> io::Result<()> {
        let mut cmd = Command::new("screencapture");
        cmd.args(["-x", "-o", "-l", &self.platform_window_id.to_string()]).arg(path);
        let out = run_with_timeout(&mut cmd, CAPTURE_TIMEOUT)?;
        if !out.status.success() || !path.exists() {
            return Err(io::Error::other(format!(
                "screencapture failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
}

impl Drop for KittyInstance {
    fn drop(&mut self) {
        quit_instance(&self.to);
    }
}

/// The first window's id and its OS window's `platform_window_id` (the macOS
/// CGWindowID), once the instance answers `kitty @ ls`.
fn first_window(to: &str) -> Option<(String, u64)> {
    let mut cmd = remote_command(Some(to));
    cmd.arg("ls");
    let out = run_with_timeout(&mut cmd, QUERY_TIMEOUT).ok()?;
    if !out.status.success() {
        return None;
    }
    let listing: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let os_window = listing.as_array()?.first()?;
    let platform = os_window.get("platform_window_id")?.as_u64()?;
    let window = os_window.get("tabs")?.as_array()?.first()?.get("windows")?.as_array()?.first()?;
    Some((window.get("id")?.as_u64()?.to_string(), platform))
}

fn quit_instance(to: &str) {
    let mut cmd = remote_command(Some(to));
    cmd.args(["action", "quit"]).stdout(Stdio::null()).stderr(Stdio::null());
    let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
}

/// Quits private instances whose owning process is gone. A test killed by a
/// timeout skips [`Drop`], and its window would otherwise stay on screen.
fn cleanup_stale_kitty_instances() {
    let Ok(entries) = std::fs::read_dir("/tmp") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(owner) = name
            .strip_prefix(INSTANCE_DIR_PREFIX)
            .and_then(|rest| rest.split('-').next())
            .and_then(|pid| pid.parse::<u32>().ok())
        else {
            continue;
        };
        if super::process_is_alive(owner) {
            continue;
        }
        let socket = entry.path().join("kitty.sock");
        if socket.exists() {
            quit_instance(&format!("unix:{}", socket.display()));
        }
        let _ = std::fs::remove_dir_all(entry.path());
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
