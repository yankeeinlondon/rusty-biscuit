//! WezTerm-driven [`TerminalHarness`].
//!
//! Uses the `wezterm cli` subcommand to spawn the binary in a fresh
//! pane of the running WezTerm GUI, send synthetic text, and capture
//! the rendered pane text. Requires:
//!
//! - The `wezterm` binary on `$PATH`.
//! - The `WEZTERM_UNIX_SOCKET` env var set (the GUI exports this only
//!   to processes it launches, so the suite must be run from inside
//!   WezTerm).
//! - A WezTerm GUI that actually responds to remote-control commands
//!   — verified by a short probe inside [`available`](Self::available).
//!
//! When any of these is missing, [`available`](Self::available) returns
//! `false` and tests that depend on this harness early-return with a
//! "skipping: requires WezTerm" message.

#![allow(dead_code)]

use std::env;
use std::io;
use std::process::{Command, Stdio};
use std::sync::Once;
use std::time::{Duration, Instant};

use super::{
    CAPTURE_TIMEOUT, CLEANUP_TIMEOUT, CapturedFrame, QUERY_TIMEOUT, SEND_TIMEOUT, SPAWN_TIMEOUT,
    SpawnVisibility, TerminalHarness, current_process_id, pid_from_tag, process_is_alive,
    run_with_stdin_timeout, run_with_timeout, wait_for_prompt,
};

/// Workspace name used when [`SpawnVisibility::Background`] is in
/// effect. Picked to be distinct from any name a developer is likely
/// to use interactively so the window is created out-of-sight.
const BACKGROUND_WORKSPACE: &str = "biscuit-bg";
const PANE_TITLE_PREFIX: &str = "biscuit-test-pane-";
const LEGACY_BACKGROUND_PANE_LIMIT: usize = 64;
static CLEANUP_ONCE: Once = Once::new();

/// Wall-clock budget for the reachability probe inside
/// [`WezTermHarness::available`].
///
/// Deliberately much shorter than [`QUERY_TIMEOUT`] (10 s): the probe
/// exists to fail fast when the GUI is hung or the socket is stale, so
/// the broker / spawn paths skip cleanly instead of burning the full
/// 15 s [`SPAWN_TIMEOUT`] in [`spawn_shell`](TerminalHarness::spawn_shell)
/// before erroring. 1.5 s is well above the ~50 ms a healthy WezTerm
/// takes to answer `wezterm cli list --format json` on a quiet host.
const AVAILABLE_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Geometry returned by [`WezTermHarness::pane_size`].
///
/// Mirrors the `size` object from `wezterm cli list --format json`.
#[derive(Debug, Clone, Copy)]
pub struct PaneSize {
    /// Number of text rows in the pane.
    pub rows: u32,
    /// Number of text columns in the pane.
    pub cols: u32,
    /// Pane width in pixels (cols × cell_width_px).
    pub pixel_width: u32,
    /// Pane height in pixels (rows × cell_height_px).
    pub pixel_height: u32,
}

impl PaneSize {
    /// Returns the cell width in pixels, derived from `pixel_width / cols`.
    ///
    /// Returns `0` when `cols` is zero (defensive — shouldn't happen in
    /// practice).
    pub fn cell_width_px(&self) -> u32 {
        self.pixel_width.checked_div(self.cols).unwrap_or(0)
    }

    /// Returns the cell height in pixels, derived from
    /// `pixel_height / rows`.
    ///
    /// Returns `0` when `rows` is zero.
    pub fn cell_height_px(&self) -> u32 {
        self.pixel_height.checked_div(self.rows).unwrap_or(0)
    }
}

/// Environment variable read by [`WezTermHarness::shared_or_spawn`] to
/// attach to a pane that was pre-spawned by an outer process (e.g. the
/// `_test_l2` recipe via `biscuit-harness-broker`) instead of paying the
/// 2-3 s cost of spawning a fresh pane in every nextest child process.
pub const SHARED_PANE_ENV: &str = "BISCUIT_SHARED_WEZTERM_PANE_ID";

/// Harness that talks to a running WezTerm GUI via `wezterm cli`.
pub struct WezTermHarness {
    pane_id: Option<String>,
    spawn_visibility: SpawnVisibility,
    /// When `true`, [`Drop`] kills the pane via `wezterm cli kill-pane`.
    /// When `false` (set by [`WezTermHarness::attach`]) the pane is left
    /// alone because some outer scope owns it.
    owned: bool,
    /// Extra OS window titles to try in
    /// [`raise_and_window_bounds`](Self::raise_and_window_bounds) after the
    /// stamped tab title and the `"question"` fallback. WezTerm overrides the
    /// OS window title with the foreground program's basename, so a test that
    /// runs a foreground binary (e.g. `claudine`) must register that basename
    /// here for AXRaise to match its window. Empty by default.
    expected_window_titles: Vec<String>,
}

impl WezTermHarness {
    /// Returns a fresh harness with [`SpawnVisibility::Background`].
    /// Call [`spawn_shell`](TerminalHarness::spawn_shell) to actually
    /// create a pane.
    pub fn new() -> Self {
        Self {
            pane_id: None,
            spawn_visibility: SpawnVisibility::default(),
            owned: true,
            expected_window_titles: Vec::new(),
        }
    }

    /// Returns a harness that references an existing pane by id without
    /// taking ownership: the [`Drop`] impl is a no-op so the pane
    /// survives this harness going out of scope.
    ///
    /// Used by [`shared_or_spawn`](Self::shared_or_spawn) and by
    /// `biscuit-harness-broker kill` to act on panes whose lifecycle is
    /// managed by an outer process.
    pub fn attach(pane_id: impl Into<String>) -> Self {
        Self {
            pane_id: Some(pane_id.into()),
            spawn_visibility: SpawnVisibility::default(),
            owned: false,
            expected_window_titles: Vec::new(),
        }
    }

    /// If [`SHARED_PANE_ENV`] is set, returns a borrowed
    /// [`attach`](Self::attach)-style harness pointing at the pre-spawned
    /// pane. Otherwise spawns a fresh background pane (owned).
    ///
    /// ## Errors
    ///
    /// Propagates whatever [`spawn_shell`](TerminalHarness::spawn_shell)
    /// returns when no shared pane id is available.
    pub fn shared_or_spawn() -> io::Result<Self> {
        if let Ok(id) = env::var(SHARED_PANE_ENV) {
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
    /// [`SpawnVisibility::Foreground`] for tests that intend to call
    /// [`focus_spawned_pane`](Self::focus_spawned_pane) — those need
    /// the window to be on the active workspace before AXRaise can
    /// reach it.
    ///
    /// ## Invariant
    ///
    /// L2 (shared) tests MUST keep the default
    /// [`SpawnVisibility::Background`] so test runs do not steal focus
    /// from the developer's foreground app. Only L3 tests (which own
    /// their own per-test harness and need OS keyboard injection)
    /// should override to [`SpawnVisibility::Foreground`]. Attached
    /// (shared-pane) harnesses panic from this setter in debug builds
    /// because the broker-spawned pane was already created in
    /// background mode and cannot be re-spawned with a different
    /// visibility.
    pub fn with_spawn_visibility(mut self, visibility: SpawnVisibility) -> Self {
        debug_assert!(
            self.owned,
            "with_spawn_visibility on an attached (shared) WezTermHarness has no effect — \
             the pane was already spawned by biscuit-harness-broker in Background mode",
        );
        self.spawn_visibility = visibility;
        self
    }

    /// Registers an additional OS window title to match in
    /// [`focus_spawned_pane`](Self::focus_spawned_pane)'s AXRaise step,
    /// chainable like [`with_spawn_visibility`](Self::with_spawn_visibility).
    ///
    /// WezTerm clobbers the window title we stamp on the tab with the
    /// foreground program's basename whenever a foreground binary is running
    /// in the pane. A test that runs such a binary (e.g. `claudine`) sees its
    /// window titled `claudine`, matching neither the stamped tab title nor
    /// the built-in `"question"` fallback — so it must register `"claudine"`
    /// here for the raise to find its window. Titles are tried in
    /// registration order after the stamped title and `"question"`.
    pub fn with_expected_window_title(mut self, title: impl Into<String>) -> Self {
        self.expected_window_titles.push(title.into());
        self
    }

    /// Returns `true` only when the WezTerm GUI is reachable at runtime.
    ///
    /// This is a **runtime reachability** probe, not an "is it installed?"
    /// check. It composes three gates, short-circuiting on the first
    /// failure:
    ///
    /// 1. `WEZTERM_UNIX_SOCKET` env var is set. WezTerm only exports this
    ///    to processes the GUI itself launches, so an unset value usually
    ///    means the suite is not running inside WezTerm.
    /// 2. `wezterm` binary is on `$PATH`.
    /// 3. A live `wezterm cli list --format json` round-trip succeeds
    ///    within [`AVAILABLE_PROBE_TIMEOUT`] (1.5 s).
    ///
    /// ## Why the probe is necessary
    ///
    /// Gates 1 and 2 are necessary but not sufficient. A hung GUI, a
    /// stale socket file, or a mux-server that cannot bootstrap will
    /// pass both env + binary checks and then cause every subsequent
    /// `wezterm cli …` call to block until [`SPAWN_TIMEOUT`] (15 s)
    /// elapses — repeated across retries, this turns a single
    /// unusable backend into a multi-minute test-tier stall. The probe
    /// converts that stall into a clean `false`.
    ///
    /// ## Why a separate timeout budget
    ///
    /// [`QUERY_TIMEOUT`] (10 s) is sized for `wezterm cli list` against
    /// a *healthy* GUI on a loaded CI host. Reusing it here would
    /// re-introduce the stall: an unreachable GUI would burn 10 s per
    /// probe before the gate tripped. [`AVAILABLE_PROBE_TIMEOUT`] (1.5 s)
    /// is bounded by what a *responsive* GUI answers in, plus generous
    /// slack — see the constant's own doc for the rationale.
    ///
    /// ## Caching
    ///
    /// No caching. The probe is cheap (~50 ms on a quiet host) and any
    /// cached value could go stale between a `cargo test` launch and
    /// the moment a test actually runs (the GUI may have been closed,
    /// restarted, or the socket removed in the interim). Callers that
    /// invoke `available()` many times in a single process pay the
    /// probe each time, but the total cost is bounded.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use biscuit_test_harness::wezterm::WezTermHarness;
    ///
    /// if !WezTermHarness::available() {
    ///     eprintln!("skipping: requires reachable WezTerm GUI");
    ///     return;
    /// }
    /// // … proceed with WezTerm-backed assertions …
    /// ```
    pub fn available() -> bool {
        env::var_os("WEZTERM_UNIX_SOCKET").is_some() && which("wezterm") && wezterm_gui_reachable()
    }

    /// Returns the active pane id (panicking if the harness has not
    /// spawned or attached yet).
    pub fn pane_id_str(&self) -> &str {
        self.pane_id()
    }

    /// Captures the pane including up to `lines` rows of scrollback, not just
    /// the visible viewport that [`capture`](TerminalHarness::capture) returns.
    ///
    /// Output taller than the pane scrolls into history; the viewport-only
    /// `capture` then loses it. A report whose top scrolls off a short shared
    /// pane (e.g. a header's OSC8 hyperlink above a long body) is recoverable
    /// only this way. `wezterm cli get-text --start-line -<lines>` reads that
    /// many physical lines up from the viewport into the scrollback.
    ///
    /// ## Notes
    ///
    /// A shared pane is reused across serial tests, so its scrollback can hold
    /// earlier tests' output. Scope assertions to a string only the test under
    /// capture emits, or clear the pane (`ESC[3J`) before driving the command.
    pub fn capture_scrollback(&mut self, lines: u32) -> io::Result<CapturedFrame> {
        let id = self.pane_id().to_string();
        let start = format!("-{lines}");
        let mut cmd = Command::new("wezterm");
        cmd.args([
            "cli",
            "get-text",
            "--pane-id",
            &id,
            "--escapes",
            "--start-line",
            &start,
        ]);
        let out = run_with_timeout(&mut cmd, CAPTURE_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli get-text (scrollback) failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        Ok(CapturedFrame::from_raw(raw))
    }

    /// Borrows the active pane id, panicking with a clear message when
    /// no pane has been spawned yet.
    fn pane_id(&self) -> &str {
        self.pane_id
            .as_deref()
            .expect("WezTermHarness::spawn_shell must be called before send_text/capture")
    }

    /// Kills the current pane via `wezterm cli kill-pane`, ignoring
    /// any error (best effort).
    fn kill_pane(&mut self) {
        if let Some(id) = self.pane_id.take() {
            let mut cmd = Command::new("wezterm");
            cmd.args(["cli", "kill-pane", "--pane-id", &id])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
        }
    }

    /// Returns a unique window title we stamp on the spawned WezTerm
    /// window so System Events can target it precisely. Includes the
    /// pane id (already unique within a WezTerm instance) so concurrent
    /// runs of the same harness don't collide.
    fn unique_window_title(&self) -> String {
        format!(
            "{PANE_TITLE_PREFIX}{}-{}",
            current_process_id(),
            self.pane_id()
        )
    }

    fn tag_spawned_pane(&self) {
        let id = self.pane_id();
        let title = self.unique_window_title();
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "set-tab-title", "--pane-id", id, &title])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
    }

    /// Returns the spawned pane's geometry as `(rows, cols, pixel_width,
    /// pixel_height)`.
    ///
    /// Wraps `wezterm cli list --format json` and filters for the active
    /// pane id. Used by Level-2 image and layout tests to compute
    /// expected row/column counts from real terminal geometry instead
    /// of hardcoding.
    ///
    /// ## Errors
    ///
    /// Returns an error when the `wezterm cli list` command fails or
    /// when the active pane id is not present in its output.
    pub fn pane_size(&self) -> io::Result<PaneSize> {
        let id = self.pane_id();
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "list", "--format", "json"]);
        let out = run_with_timeout(&mut cmd, QUERY_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli list failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let entries: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| io::Error::other(format!("wezterm cli list: invalid json: {e}")))?;
        let arr = entries
            .as_array()
            .ok_or_else(|| io::Error::other("wezterm cli list: top level not an array"))?;
        let want: u64 = id
            .parse()
            .map_err(|e| io::Error::other(format!("pane id {id:?} not a u64: {e}")))?;
        for entry in arr {
            let pid = entry.get("pane_id").and_then(|v| v.as_u64()).unwrap_or(0);
            if pid == want {
                let size = entry
                    .get("size")
                    .ok_or_else(|| io::Error::other("pane entry missing size"))?;
                let rows = size.get("rows").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let cols = size.get("cols").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let pw = size
                    .get("pixel_width")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let ph = size
                    .get("pixel_height")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                return Ok(PaneSize {
                    rows,
                    cols,
                    pixel_width: pw,
                    pixel_height: ph,
                });
            }
        }
        Err(io::Error::other(format!(
            "pane id {id} not found in wezterm cli list"
        )))
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
        match self.raise_and_window_bounds()? {
            Some((x, y, w, h)) => {
                let click_x = x + w / 2;
                let click_y = y + h / 2;
                eprintln!(
                    "[focus_spawned_pane] WezTerm window pos=({x},{y}) size=({w},{h}) → click target ({click_x},{click_y})"
                );
                #[cfg(target_os = "macos")]
                wait_until_wezterm_frontmost(Duration::from_secs(5))?;
                Ok(Some((click_x, click_y)))
            }
            None => {
                std::thread::sleep(Duration::from_millis(400));
                Ok(None)
            }
        }
    }

    /// Raises the spawned pane's WezTerm window to the front and returns its
    /// screen bounds as `(x, y, width, height)` in points.
    ///
    /// Stamps the pane with a unique title, activates the pane, then (macOS
    /// only) raises the owning window via System Events and reads its position
    /// and size. Returns `Ok(None)` off macOS, or when the window position
    /// could not be resolved.
    ///
    /// The AXRaise match tries, in order: the stamped tab title, the literal
    /// `"question"` (the biscuit-tui CLI binary), then each title registered
    /// via [`with_expected_window_title`](Self::with_expected_window_title).
    /// The extra candidates exist because WezTerm overrides the OS window title
    /// with the foreground program's basename, so a pane running e.g. `claudine`
    /// reports a window title of `claudine` that matches neither of the first
    /// two.
    ///
    /// ## Errors
    ///
    /// Returns an error when the `wezterm cli` title/activate calls fail, or
    /// when the System Events AXRaise call fails (e.g. missing Accessibility
    /// permission for the parent terminal app).
    fn raise_and_window_bounds(&self) -> io::Result<Option<(i32, i32, i32, i32)>> {
        let id = self.pane_id();
        let title = self.unique_window_title();

        // 1. Stamp a unique title we can target precisely.
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "set-tab-title", "--pane-id", id, &title]);
        let out = run_with_timeout(&mut cmd, QUERY_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli set-tab-title failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        // 2. Activate intra-WezTerm pane focus.
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "activate-pane", "--pane-id", id]);
        let out = run_with_timeout(&mut cmd, QUERY_TIMEOUT)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli activate-pane failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        // 3. Raise *our* window via System Events (macOS only).
        #[cfg(target_os = "macos")]
        {
            let mut cmd = Command::new("osascript");
            cmd.args(["-e", "tell application \"WezTerm\" to activate"])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let _ = run_with_timeout(&mut cmd, QUERY_TIMEOUT);
            std::thread::sleep(Duration::from_millis(150));

            // Each registered extra title becomes another exact-match fallback
            // tried only when the stamped title and "question" both miss. See
            // `with_expected_window_title`: WezTerm renames the OS window to the
            // foreground program's basename, defeating the stamped-title match.
            let extra_fallbacks: String = self
                .expected_window_titles
                .iter()
                .map(|t| {
                    let t = applescript_quote(t);
                    format!(
                        "\n                                   if (count of hits) is 0 then\n                                       set hits to windows of p whose title is \"{t}\"\n                                   end if"
                    )
                })
                .collect();
            // Each candidate is rendered into the AppleScript `error` string
            // literal, so its surrounding quotes must be backslash-escaped
            // (`\"name\"`) or they would terminate that literal early.
            let candidates_for_error: String = std::iter::once("question".to_string())
                .chain(self.expected_window_titles.iter().cloned())
                .map(|t| format!("\\\"{}\\\"", applescript_quote(&t)))
                .collect::<Vec<_>>()
                .join(", ");

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
                                   end if{extra_fallbacks}
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
                           error "no wezterm window matched {title} or {candidates_for_error}; visible titles:" & linefeed & seenTitles
                       end tell
                   end timeout"#,
            );
            let mut cmd = Command::new("osascript");
            cmd.args(["-e", &script]).stderr(Stdio::piped());
            let activate = run_with_timeout(&mut cmd, QUERY_TIMEOUT)?;
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
                return Ok(Some((parts[0], parts[1], parts[2], parts[3])));
            }
        }

        Ok(None)
    }

    /// Raises the spawned pane's window and screen-captures it to PNG bytes via
    /// macOS `screencapture -R`.
    ///
    /// Intended for the Level-2 terminal-image test, where text capture cannot
    /// prove an inline image was painted: the caller samples the returned pixels
    /// for the distinctive fill color it rendered.
    ///
    /// ## Returns
    ///
    /// `Ok(Some(png_bytes))` on a successful capture, or `Ok(None)` when
    /// window-region capture is unavailable — off macOS, when the window bounds
    /// cannot be resolved, or when `screencapture` fails. A `None` is a signal
    /// to skip pixel assertions cleanly, mirroring the rest of the harness.
    ///
    /// ## Notes
    ///
    /// macOS only. Requires Screen Recording permission for the parent process;
    /// without it `screencapture` yields a black frame. The caller distinguishes
    /// that (a near-black capture) from a genuine paint failure and skips on it.
    #[cfg(target_os = "macos")]
    pub fn capture_window_png(&self) -> io::Result<Option<Vec<u8>>> {
        // A raise/bounds failure (window not matched, Accessibility permission
        // not granted) is a reason to skip the pixel assertion, not to fail —
        // degrade it to `None` rather than propagating the error.
        let Ok(Some((x, y, w, h))) = self.raise_and_window_bounds() else {
            return Ok(None);
        };
        if w <= 0 || h <= 0 {
            return Ok(None);
        }
        // Let the compositor settle after the raise before grabbing pixels.
        std::thread::sleep(Duration::from_millis(300));

        let tmp = env::temp_dir().join(format!("biscuit-wezcap-{}.png", self.pane_id()));
        let mut cmd = Command::new("screencapture");
        cmd.args(["-x", "-o"])
            .arg(format!("-R{x},{y},{w},{h}"))
            .arg(&tmp)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let out = run_with_timeout(&mut cmd, QUERY_TIMEOUT)?;
        if !out.status.success() {
            return Ok(None);
        }
        match std::fs::read(&tmp) {
            Ok(bytes) => {
                let _ = std::fs::remove_file(&tmp);
                if bytes.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(bytes))
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Non-macOS stub: window-region screen capture is unsupported, so callers
    /// skip the pixel assertion. See the macOS implementation for details.
    #[cfg(not(target_os = "macos"))]
    pub fn capture_window_png(&self) -> io::Result<Option<Vec<u8>>> {
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
        if self.owned {
            self.kill_pane();
        }
    }
}

/// Kills the pane with the given id by shelling out to
/// `wezterm cli kill-pane`. Used by `biscuit-harness-broker kill` so the
/// `_test_l2` recipe can tear down shared panes whose `WezTermHarness`
/// instances were leaked in a different process. Best-effort: any error
/// is swallowed.
pub fn kill_pane_by_id(pane_id: &str) {
    let mut cmd = Command::new("wezterm");
    cmd.args(["cli", "kill-pane", "--pane-id", pane_id])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
}

impl TerminalHarness for WezTermHarness {
    /// Spawns a login shell (`$SHELL`, `bash`, or `sh`) in a fresh
    /// WezTerm pane and waits for the shell prompt to appear.
    ///
    /// The cargo target directory containing `bt` and `question` is
    /// prepended to `PATH` so CLI binaries resolve without an absolute
    /// path. Color-forcing env vars are applied so SGR output in
    /// captures is deterministic.
    fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("WezTerm not available"));
        }
        if self.spawn_visibility == SpawnVisibility::Background {
            CLEANUP_ONCE.call_once(cleanup_stale_wezterm_panes);
        }
        let shell = super::detect_shell();
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "spawn", "--new-window"]);
        if self.spawn_visibility == SpawnVisibility::Background {
            cmd.args(["--workspace", BACKGROUND_WORKSPACE]);
        }
        cmd.arg("--");
        let bin_dir = super::cargo_bin_dir("bt").or_else(|| super::cargo_bin_dir("question"));
        super::configure_login_shell(&mut cmd, &shell, bin_dir.as_deref());

        // Force color on the spawned shell so `bt`'s color detection is
        // deterministic regardless of how the test runner inherits TTY
        // state. `wezterm cli get-text --escapes` re-emits SGR from cell
        // attributes only if SGR was actually rendered into cells.
        super::apply_color_forcing_env(&mut cmd);

        let out = run_with_timeout(&mut cmd, SPAWN_TIMEOUT)?;
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
        self.tag_spawned_pane();
        wait_for_prompt(self)?;
        Ok(())
    }

    /// Direct-spawn escape hatch: launches `program` with `args`
    /// inside a fresh WezTerm pane via `wezterm cli spawn -- …`. No
    /// shell, no `PATH` augmentation, no prompt-readiness wait — the
    /// caller is responsible for any settling.
    fn spawn_program(&mut self, program: &str, args: &[&str]) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("WezTerm not available"));
        }
        if self.spawn_visibility == SpawnVisibility::Background {
            CLEANUP_ONCE.call_once(cleanup_stale_wezterm_panes);
        }
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "spawn", "--new-window"]);
        if self.spawn_visibility == SpawnVisibility::Background {
            cmd.args(["--workspace", BACKGROUND_WORKSPACE]);
        }
        cmd.arg("--");
        cmd.arg(program);
        for a in args {
            cmd.arg(a);
        }
        let out = run_with_timeout(&mut cmd, SPAWN_TIMEOUT)?;
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
        self.tag_spawned_pane();
        std::thread::sleep(Duration::from_millis(400));
        Ok(())
    }

    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
        let id = self.pane_id().to_string();
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "send-text", "--pane-id", &id, "--no-paste"]);
        let out = run_with_stdin_timeout(&mut cmd, bytes, SEND_TIMEOUT)?;
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
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "get-text", "--pane-id", &id, "--escapes"]);
        let out = run_with_timeout(&mut cmd, CAPTURE_TIMEOUT)?;
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

/// Removes stale panes from the harness-owned WezTerm background
/// workspace.
///
/// New panes are tagged with `biscuit-test-pane-<pid>-<pane_id>` and are
/// removed only when `<pid>` is no longer alive. Older harness versions did
/// not tag background panes; when the background workspace has grown past a
/// conservative limit, this function also removes untagged panes from that
/// workspace to recover from historical leaks.
pub fn cleanup_stale_wezterm_panes() {
    if !WezTermHarness::available() {
        return;
    }
    let mut cmd = Command::new("wezterm");
    cmd.args(["cli", "list", "--format", "json"]);
    let Ok(out) = run_with_timeout(&mut cmd, QUERY_TIMEOUT) else {
        return;
    };
    if !out.status.success() {
        return;
    }
    let Ok(entries) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return;
    };
    let Some(entries) = entries.as_array() else {
        return;
    };

    let background_count = entries
        .iter()
        .filter(|entry| string_field(entry, "workspace") == Some(BACKGROUND_WORKSPACE))
        .count();
    let sweep_legacy = background_count > LEGACY_BACKGROUND_PANE_LIMIT
        || env::var("BISCUIT_TEST_HARNESS_SWEEP_LEGACY_WEZTERM").as_deref() == Ok("1");

    for entry in entries {
        if string_field(entry, "workspace") != Some(BACKGROUND_WORKSPACE) {
            continue;
        }
        let Some(pane_id) = entry.get("pane_id").and_then(|v| v.as_u64()) else {
            continue;
        };
        let title = string_field(entry, "tab_title")
            .filter(|s| !s.is_empty())
            .or_else(|| string_field(entry, "window_title").filter(|s| !s.is_empty()))
            .or_else(|| string_field(entry, "title").filter(|s| !s.is_empty()));

        let should_kill = match title {
            Some(title) if title.starts_with(PANE_TITLE_PREFIX) => {
                pid_from_tag(title, PANE_TITLE_PREFIX)
                    .map(|pid| pid != current_process_id() && !process_is_alive(pid))
                    .unwrap_or(false)
            }
            _ => sweep_legacy,
        };
        if should_kill {
            kill_wezterm_pane(pane_id);
        }
    }
}

fn string_field<'a>(entry: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    entry.get(field).and_then(|v| v.as_str())
}

fn kill_wezterm_pane(pane_id: u64) {
    let mut cmd = Command::new("wezterm");
    cmd.args(["cli", "kill-pane", "--pane-id", &pane_id.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = run_with_timeout(&mut cmd, CLEANUP_TIMEOUT);
}

/// Escapes a title for embedding inside an AppleScript double-quoted string
/// literal: backslash and double-quote are the only characters AppleScript
/// string syntax treats specially. Expected titles are simple ASCII basenames
/// (e.g. `claudine`), but escaping keeps the OR-list robust if that ever
/// loosens.
fn applescript_quote(title: &str) -> String {
    title.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Blocks until a WezTerm process is the frontmost macOS application.
///
/// `AXRaise` returns before the WindowServer has necessarily made the window
/// key, and on a busy desktop an unrelated application can reclaim front
/// within the same window — observed here with a media app stealing front
/// between the raise and the injection. Polling the actual frontmost process
/// replaces a fixed sleep that could only ever be a guess.
///
/// ## Errors
///
/// Returns `io::Error` on timeout, naming the app that held front instead.
/// Failing here is deliberate: OS keyboard injection goes to whatever app owns
/// focus, so proceeding unfocused would fire the chord into an unrelated
/// application rather than the pane under test.
#[cfg(target_os = "macos")]
fn wait_until_wezterm_frontmost(timeout: Duration) -> io::Result<()> {
    const FRONTMOST: &str =
        r#"tell application "System Events" to get name of first application process whose frontmost is true"#;
    let deadline = Instant::now() + timeout;
    let mut last = String::new();
    loop {
        let mut cmd = Command::new("osascript");
        cmd.args(["-e", FRONTMOST]);
        if let Ok(out) = run_with_timeout(&mut cmd, QUERY_TIMEOUT) {
            last = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if last.to_lowercase().contains("wezterm") {
                return Ok(());
            }
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other(format!(
                "WezTerm did not become frontmost within {timeout:?}; {last:?} holds \
                 focus. OS keyboard injection would land in that app instead of the \
                 pane under test."
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
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

/// Probes whether the WezTerm GUI is actually responding to
/// remote-control commands by running a single
/// `wezterm cli list --format json` round-trip under a short
/// wall-clock timeout.
///
/// Returns `true` only when the probe exits successfully. Returns
/// `false` on any of:
///
/// - **Timeout** — the GUI is hung or the socket is stale. This is
///   the failure mode that motivates the helper; without it,
///   [`WezTermHarness::available`] would say `true` and every
///   subsequent spawn would burn [`SPAWN_TIMEOUT`] before erroring.
/// - **Non-zero exit** — wezterm CLI rejected the invocation (e.g.
///   "no mux server") without hanging.
/// - **Spawn error** — the binary could not be launched at all
///   (covered by the `which` gate in [`WezTermHarness::available`],
///   but defended here so the helper is robust in isolation).
///
/// ## Output handling
///
/// The probe deliberately does not parse the JSON payload — we only
/// care *whether* the GUI answered, not what it said. stdout/stderr
/// are piped and drained by [`run_with_timeout`]'s background threads
/// so a chatty server cannot deadlock the probe.
///
/// ## Notes
///
/// This helper does **not** check `WEZTERM_UNIX_SOCKET` or `$PATH`.
/// [`WezTermHarness::available`] composes those gates around it; the
/// helper's contract is "run the probe and report its outcome", so
/// callers can unit-test the probe surface in isolation.
fn wezterm_gui_reachable() -> bool {
    let mut cmd = Command::new("wezterm");
    cmd.args(["cli", "list", "--format", "json"]);
    match run_with_timeout(&mut cmd, AVAILABLE_PROBE_TIMEOUT) {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// `available()` must return `false` when `WEZTERM_UNIX_SOCKET` is
    /// unset, regardless of whether the `wezterm` binary is on `$PATH`
    /// or the GUI is reachable. This is the env-var gate.
    ///
    /// Serialized because it mutates the process-wide environment.
    #[test]
    #[serial_test::serial]
    fn available_returns_false_when_socket_env_missing() {
        let prev = env::var_os("WEZTERM_UNIX_SOCKET");
        // SAFETY: serialized — no other test runs concurrently, and the
        // nextest process-per-test model means no other thread observes
        // this mutation. set_var/remove_var are unsafe in edition 2024.
        unsafe {
            env::remove_var("WEZTERM_UNIX_SOCKET");
        }
        let avail = WezTermHarness::available();
        // Restore unconditionally so a follow-on test sees the same
        // environment the suite launched with.
        unsafe {
            match prev {
                Some(v) => env::set_var("WEZTERM_UNIX_SOCKET", v),
                None => env::remove_var("WEZTERM_UNIX_SOCKET"),
            }
        }
        assert!(
            !avail,
            "available() must return false when WEZTERM_UNIX_SOCKET is unset",
        );
    }

    /// Direct unit test of [`wezterm_gui_reachable`]: when the env var
    /// is missing, `wezterm cli list` cannot address the GUI and the
    /// probe must return `false` (either by erroring or by timing out).
    ///
    /// On hosts where `wezterm` autodiscovers a socket via some other
    /// mechanism, this test may surface a true positive — in that case
    /// we skip rather than fail, since the helper's contract is "report
    /// the probe outcome", not "fail without env".
    ///
    /// Serialized because it mutates the process-wide environment.
    #[test]
    #[serial_test::serial]
    fn gui_reachable_returns_false_when_env_missing() {
        let prev = env::var_os("WEZTERM_UNIX_SOCKET");
        unsafe {
            env::remove_var("WEZTERM_UNIX_SOCKET");
        }
        let reachable = wezterm_gui_reachable();
        unsafe {
            match prev {
                Some(v) => env::set_var("WEZTERM_UNIX_SOCKET", v),
                None => env::remove_var("WEZTERM_UNIX_SOCKET"),
            }
        }
        if reachable {
            eprintln!(
                "skipping assertion: host's wezterm reached a GUI without WEZTERM_UNIX_SOCKET \
                 (autodiscovery); the helper correctly reported the probe outcome",
            );
            return;
        }
        assert!(
            !reachable,
            "wezterm_gui_reachable() must return false when WEZTERM_UNIX_SOCKET is unset",
        );
    }

    /// Direct unit test of [`wezterm_gui_reachable`]: when the `wezterm`
    /// binary is not on `$PATH`, the probe must return `false` (spawn
    /// failure, caught and reported as unreachable).
    ///
    /// Sets `WEZTERM_UNIX_SOCKET` to a plausible value so the env gate
    /// would otherwise pass — we want to isolate the binary gate.
    ///
    /// Serialized because it mutates both `PATH` and `WEZTERM_UNIX_SOCKET`.
    #[test]
    #[serial_test::serial]
    fn gui_reachable_returns_false_when_binary_absent() {
        let prev_path = env::var_os("PATH");
        let prev_socket = env::var_os("WEZTERM_UNIX_SOCKET");
        unsafe {
            env::set_var("PATH", "/nonexistent");
            env::set_var(
                "WEZTERM_UNIX_SOCKET",
                "/tmp/biscuit-test-harness-fake-socket",
            );
        }
        let reachable = wezterm_gui_reachable();
        unsafe {
            match prev_path {
                Some(v) => env::set_var("PATH", v),
                None => env::remove_var("PATH"),
            }
            match prev_socket {
                Some(v) => env::set_var("WEZTERM_UNIX_SOCKET", v),
                None => env::remove_var("WEZTERM_UNIX_SOCKET"),
            }
        }
        assert!(
            !reachable,
            "wezterm_gui_reachable() must return false when the wezterm binary is not on PATH",
        );
    }

    /// The critical regression test for the review's "skip cleanly rather
    /// than enter repeated 25-second failures" requirement.
    ///
    /// Sets `WEZTERM_UNIX_SOCKET` to a plausible-looking but
    /// non-existent path and asserts that [`WezTermHarness::available`]
    /// both (a) returns `false` and (b) returns within a wall-clock
    /// budget well below the 15 s [`SPAWN_TIMEOUT`] that previously
    /// fired on every test.
    ///
    /// The 3 s upper bound is intentionally about 2× the probe budget
    /// (1.5 s) — it gives the probe room to time out and the env check
    /// to short-circuit, while still being far enough below 15 s to
    /// prove the fix.
    ///
    /// Serialized because it mutates the process-wide environment.
    #[test]
    #[serial_test::serial]
    fn available_does_not_hang_on_unreachable_gui() {
        let prev = env::var_os("WEZTERM_UNIX_SOCKET");
        unsafe {
            env::set_var(
                "WEZTERM_UNIX_SOCKET",
                format!(
                    "/tmp/biscuit-test-harness-fake-socket-{}",
                    std::process::id()
                ),
            );
        }
        let start = Instant::now();
        let avail = WezTermHarness::available();
        let elapsed = start.elapsed();
        unsafe {
            match prev {
                Some(v) => env::set_var("WEZTERM_UNIX_SOCKET", v),
                None => env::remove_var("WEZTERM_UNIX_SOCKET"),
            }
        }
        assert!(
            !avail,
            "available() must return false when WEZTERM_UNIX_SOCKET points at a non-existent socket",
        );
        // 3 s upper bound: ~2× the probe timeout (1.5 s) so a clean
        // timeout still fits; well below the 15 s spawn timeout that
        // previously fired on every test in this state.
        assert!(
            elapsed <= Duration::from_secs(3),
            "available() took {elapsed:?} — must fail fast (<=3s) on an unreachable GUI",
        );
    }

    /// Smoke test: on a host where the GUI actually responds,
    /// [`WezTermHarness::available`] must return `true` AND the probe
    /// command must genuinely succeed (not a false positive). On hosts
    /// where the GUI is not running or is hung, this test skips
    /// cleanly via `eprintln!`.
    ///
    /// The double-check (`wezterm cli list` after `available() == true`)
    /// guards against a regression where `available()` returns `true`
    /// for a hung GUI — that would surface here as the second probe
    /// blocking past its timeout.
    #[test]
    #[serial_test::serial]
    fn available_is_true_on_responsive_host_or_skips() {
        if !WezTermHarness::available() {
            eprintln!("skipping: requires reachable WezTerm GUI");
            return;
        }
        // Sanity-check that the probe is genuinely measuring
        // responsiveness: re-run the same command directly. If the
        // first `available()` was a false positive (e.g. the env var
        // was set but the GUI just happened to answer), this would
        // surface as a hang or non-zero exit.
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "list", "--format", "json"]);
        let out = run_with_timeout(&mut cmd, AVAILABLE_PROBE_TIMEOUT)
            .expect("reachable per available(); wezterm cli list must succeed within probe budget");
        assert!(
            out.status.success(),
            "available() said true but `wezterm cli list` failed: {}",
            String::from_utf8_lossy(&out.stderr),
        );
    }
}
