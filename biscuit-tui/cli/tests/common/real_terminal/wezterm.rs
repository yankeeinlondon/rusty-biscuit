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

    /// Returns a unique window title we stamp on the spawned WezTerm
    /// window so System Events can target it precisely. Includes the
    /// pane id (already unique within a WezTerm instance) so concurrent
    /// runs of the same harness don't collide.
    fn unique_window_title(&self) -> String {
        format!("biscuit-tui-test-pane-{}", self.pane_id())
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
        //
        // We use `set-tab-title`, not `set-window-title`. Most users'
        // `wezterm.lua` defines a `format-window-title` event that
        // ignores `set-window-title` and instead derives the OS-level
        // NSWindow title from the active *tab*'s title. So setting the
        // tab title actually propagates to what System Events sees.
        // (See real-terminal screenshots in the feature folder.)
        let out = Command::new("wezterm")
            .args(["cli", "set-tab-title", "--pane-id", id, &title])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "wezterm cli set-tab-title failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        // 2. Activate intra-WezTerm pane focus (unchanged).
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
        // On other platforms there's nothing useful to do here — the
        // PTY-based bytes injection used by Level 1/2 doesn't depend
        // on key focus, and Level 3 cliclick is macOS-only by design.
        #[cfg(target_os = "macos")]
        {
            // We don't hardcode the exact WezTerm process name —
            // different builds register as "WezTerm", "wezterm-gui",
            // or "wezterm" depending on how the bundle was built.
            // AppleScript's `contains` is case-insensitive by
            // default, so the substring `"wezterm"` matches all
            // common variants in one filter.
            //
            // Walking every process on the system was prohibitively
            // slow on heavily-loaded macs (each `windows of p` is a
            // synchronous AX query that some apps service slowly).
            // The narrowed filter is two orders of magnitude faster.
            //
            // `with timeout of 5 seconds` guards against AX queries
            // hanging — if WezTerm itself is wedged we'd rather fail
            // the test fast with a clear error than block the whole
            // suite waiting for cliclick injection that will never
            // land.
            // Title-matching strategy:
            //
            //   1. Prefer our unique stamp (`biscuit-tui-test-pane-N`)
            //      set via `wezterm cli set-window-title`. Honored
            //      when the user's `wezterm.lua` does not override
            //      window titling via a `format-window-title` event.
            //
            //   2. Fall back to the binary basename (`question`),
            //      which is what WezTerm auto-titles a window with by
            //      default — most `format-window-title` configs derive
            //      the title from the active pane's foreground command.
            //
            // The fallback is unambiguous in this harness because
            // `--test-threads=1` guarantees only one spawned `question`
            // window exists at a time, AND `WezTermHarness::Drop`
            // tears its pane down before the next test starts.
            //
            // Caveat: if you happen to have another WezTerm window
            // running `question` for unrelated reasons during a Level-3
            // run, the harness may raise that one instead. Don't.
            // The script raises the matching window AND echoes its
            // screen position+size so the caller can synthesise a
            // real OS click into it. AXRaise alone doesn't transfer
            // macOS keyWindow when the test runner lives in a
            // different app (e.g. iTerm2 hosts cargo, WezTerm hosts
            // the spawned pane); a real click does.
            //
            // Output format on success: "x y w h\n" (integers in
            // global screen coords).
            // First activate WezTerm via Apple Events. System Events
            // `set frontmost to true` is unreliable when the calling
            // app (iTerm2 hosting cargo) is currently active —
            // modern macOS sometimes ignores the activation request
            // unless the app explicitly issues an Apple Event. The
            // first time this fires from a new parent, macOS may
            // prompt to grant Automation permission (System Settings
            // → Privacy & Security → Automation → <parent app>);
            // grant it once and the prompts stop.
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

            // Parse "x y w h" and return the click anchor. We use
            // (x+50, y+5) — squarely in the title bar on most macOS
            // setups, or just inside the top edge if title bars are
            // disabled. The caller is responsible for synthesising
            // the click as part of a single batched cliclick
            // invocation alongside the actual key injection — see
            // `cli/tests/common/real_terminal/cliclick.rs::click_then_*`.
            // Splitting the click and the keys across separate
            // cliclick processes is exactly what the cliclick docs
            // warn against; one process per key sequence is the only
            // way to keep focus transfer atomic with the press.
            let stdout = String::from_utf8_lossy(&activate.stdout);
            let parts: Vec<i32> = stdout
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() == 4 {
                let (x, y, w, h) = (parts[0], parts[1], parts[2], parts[3]);
                // Diagnostic so failures are debuggable. Click the
                // dead center of the window — clicking the title bar
                // (y+5) was risky because some WezTerm configs hide
                // title bars and y+5 might land in the macOS menu bar
                // for top-edge windows.
                let click_x = x + w / 2;
                let click_y = y + h / 2;
                eprintln!(
                    "[focus_spawned_pane] WezTerm window pos=({x},{y}) size=({w},{h}) → click target ({click_x},{click_y})"
                );
                std::thread::sleep(Duration::from_millis(200));
                return Ok(Some((click_x, click_y)));
            }
        }

        // Non-macOS (or AppleScript output unparseable) — caller
        // gets no coords and falls back to whatever focus the
        // platform managed to establish.
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
