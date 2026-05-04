//! Terminal.app-driven [`TerminalHarness`] (macOS).
//!
//! Drives Apple's bundled Terminal.app via `osascript` so Level-2 tests
//! can verify rendering against the real lower-capability display path
//! (no images, no OSC8, single-underline only).
//!
//! ## Capture limitation
//!
//! Terminal.app exposes only the **plain visible text** of a tab via
//! its scripting interface. There is no way to retrieve the underlying
//! ANSI/SGR byte stream. As a result, [`CapturedFrame::raw`] and
//! [`CapturedFrame::plain`] are populated with the same string, and
//! callers cannot directly assert "no OSC8 bytes were emitted" — they
//! can only assert "no literal escape garbage is visible".
//!
//! Level-1 PTY tests cover the byte-level negative assertions; this
//! harness covers the real-display visibility assertions.
//!
//! ## Skip semantics
//!
//! [`AppleTerminalHarness::available`] returns `false` when:
//!
//! - The host is not macOS.
//! - `CI=1` is set (Terminal.app cannot be exercised in CI).
//! - `osascript` cannot resolve the `Terminal` application bundle.
//!
//! Tests should early-return with
//! [`crate::skip_with_reason`](super::skip_with_reason) when this
//! returns `false`.
//!
//! ## Concurrency
//!
//! Terminal.app exposes one global application state to AppleScript;
//! spawning multiple harnesses in parallel races on window indices and
//! focus. Level-2 tests that use this harness must serialize via
//! `serial_test::serial(level2_terminal)` or equivalent.
//!
//! ## AppleScript escape contract
//!
//! [`applescript_escape`] handles only the characters that have a
//! defined representation inside an AppleScript double-quoted literal:
//! backslash (`\\`), double-quote (`"`), newline (`\n`), and tab
//! (`\t`). Bytes outside printable UTF-8 plus LF (0x0A) and HT (0x09)
//! are not escaped; CR (0x0D), NUL (0x00), BEL (0x07), ESC (0x1B), and
//! the Unicode line / paragraph separators (U+2028 / U+2029) will
//! produce an AppleScript syntax error and are rejected via
//! `debug_assert!` in debug builds. Release builds remain best-effort
//! so that an unexpected byte does not crash the harness.

#![allow(dead_code)]

use std::env;
use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::{CapturedFrame, TerminalHarness};

/// Harness that drives Terminal.app via `osascript`.
///
/// On `spawn_shell` opens a fresh window running a login shell with
/// `PATH` augmented to find the workspace's CLI binaries, captures the
/// AppleScript window id, and miniaturizes the window so the developer
/// can keep working. On `Drop` the window is closed without saving.
pub struct AppleTerminalHarness {
    window_id: Option<i64>,
}

impl AppleTerminalHarness {
    /// Returns a fresh harness. Call
    /// [`spawn_shell`](TerminalHarness::spawn_shell) to actually open a
    /// Terminal.app window.
    pub fn new() -> Self {
        Self { window_id: None }
    }

    /// Returns `true` when this harness can be used:
    /// macOS, not in CI, and `osascript` can address Terminal.app.
    pub fn available() -> bool {
        if !cfg!(target_os = "macos") {
            return false;
        }
        if env::var("CI").as_deref() == Ok("1") {
            return false;
        }
        Command::new("osascript")
            .args(["-e", "id of application \"Terminal\""])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Borrows the spawned window id, panicking with a clear message
    /// when no window has been spawned yet.
    fn window_id(&self) -> i64 {
        self.window_id
            .expect("AppleTerminalHarness::spawn_shell must be called before send_text/capture")
    }

    /// Closes the spawned window without saving its session.
    ///
    /// Best-effort: errors are surfaced via `eprintln!` so a stuck
    /// window in CI is diagnosable, but they are not propagated up.
    fn close_window(&mut self) {
        if let Some(id) = self.window_id.take() {
            let script = format!(
                "tell application \"Terminal\" to close (every window whose id is {id}) saving no",
            );
            match Command::new("osascript")
                .args(["-e", &script])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
            {
                Ok(output) if !output.status.success() => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!(
                        "warning: failed to close Terminal.app window {id}: {}",
                        stderr.trim()
                    );
                }
                Err(err) => {
                    eprintln!("warning: failed to close Terminal.app window {id}: {err}");
                }
                Ok(_) => {}
            }
        }
    }

    /// Runs `osascript -e <script>` and returns its trimmed stdout.
    ///
    /// ## Errors
    ///
    /// Returns `io::Error` when the spawn fails or osascript exits
    /// non-zero, with stderr embedded in the message.
    fn run_script(script: &str) -> io::Result<String> {
        let out = Command::new("osascript").args(["-e", script]).output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "osascript failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

impl Default for AppleTerminalHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AppleTerminalHarness {
    fn drop(&mut self) {
        self.close_window();
    }
}

impl TerminalHarness for AppleTerminalHarness {
    /// Opens a fresh Terminal.app window running a login shell, captures
    /// its AppleScript window id, and miniaturizes the window so it does
    /// not steal focus.
    ///
    /// Cargo's target binary directory is prepended to `PATH` so CLI
    /// binaries (`bt`, `question`) resolve without an absolute path.
    /// Color-forcing env vars (`FORCE_COLOR`, `CLICOLOR_FORCE`,
    /// optionally `TERM` / `COLORTERM`) are set so SGR output in
    /// captures is deterministic — Terminal.app's plain-text capture
    /// strips SGR anyway, but the variables propagate to `bt` and other
    /// child processes that gate output on TTY heuristics.
    fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("Terminal.app via osascript not available"));
        }
        let shell = super::detect_shell();
        let mut shell_cmd = String::new();

        if let Some(bin_dir) =
            super::cargo_bin_dir("bt").or_else(|| super::cargo_bin_dir("question"))
        {
            shell_cmd.push_str("PATH=");
            shell_cmd.push_str(&shell_quote(&bin_dir.to_string_lossy()));
            shell_cmd.push_str(":$PATH ");
        }
        shell_cmd.push_str("FORCE_COLOR=1 CLICOLOR_FORCE=1 ");
        if env::var_os("TERM").is_none() {
            shell_cmd.push_str("TERM=xterm-256color ");
        }
        if env::var_os("COLORTERM").is_none() {
            shell_cmd.push_str("COLORTERM=truecolor ");
        }
        shell_cmd.push_str("exec ");
        shell_cmd.push_str(&shell);
        shell_cmd.push_str(" -l");

        let script = format!(
            r#"tell application "Terminal"
                activate
                set newTab to do script "{cmd}"
                delay 0.1
                set winId to id of front window
                set miniaturized of front window to true
                return winId as text
            end tell"#,
            cmd = applescript_escape(&shell_cmd),
        );
        let stdout = Self::run_script(&script)?;
        let id: i64 = stdout
            .parse()
            .map_err(|e| io::Error::other(format!("unparseable window id {stdout:?}: {e}")))?;
        self.window_id = Some(id);

        // Poll for the shell prompt instead of sleeping a fixed
        // 800 ms — `wait_for_prompt` returns early once `$`/`#`/`%`
        // appears on the last line, with a 5 s ceiling. If polling
        // returns without a prompt match, fall back to a small settle
        // (≤ 200 ms) so subsequent `do script` calls still race-free
        // dispatch into a ready prompt on slow hosts.
        super::wait_for_prompt(self)?;
        std::thread::sleep(Duration::from_millis(200));
        Ok(())
    }

    /// Sends a single command line into the spawned tab.
    ///
    /// Bytes are interpreted as UTF-8 and any trailing newline is
    /// stripped before being handed to AppleScript's `do script "…" in
    /// selected tab` — `do script` runs the string as a shell command
    /// regardless of whether it ends in a newline, and a literal `\n`
    /// confuses the AppleScript escape layer.
    ///
    /// ## Errors
    ///
    /// Returns `io::Error` when the bytes are not valid UTF-8 or
    /// `osascript` returns a non-zero exit status.
    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
        let id = self.window_id();
        let s = std::str::from_utf8(bytes)
            .map_err(|e| io::Error::other(format!("send_text non-utf8: {e}")))?;
        let cmd = s.trim_end_matches(['\r', '\n']);
        if cmd.is_empty() {
            return Ok(());
        }
        let script = format!(
            r#"tell application "Terminal"
                do script "{cmd}" in selected tab of (first window whose id is {id})
            end tell"#,
            cmd = applescript_escape(cmd),
            id = id,
        );
        Self::run_script(&script)?;
        Ok(())
    }

    /// Returns the visible plain-text contents of the spawned tab.
    ///
    /// Terminal.app does not expose ANSI/SGR bytes; both `raw` and
    /// `plain` fields of the returned [`CapturedFrame`] hold the same
    /// string.
    fn capture(&mut self) -> io::Result<CapturedFrame> {
        let id = self.window_id();
        let script = format!(
            r#"tell application "Terminal"
                return contents of selected tab of (first window whose id is {id})
            end tell"#,
        );
        let text = Self::run_script(&script)?;
        Ok(CapturedFrame {
            raw: text.clone(),
            plain: text,
        })
    }

    /// Sleeps long enough for `do script` to dispatch and Terminal.app
    /// to redraw. Terminal.app's AppleScript layer is noticeably slower
    /// than `wezterm cli` / `kitty @`, so we wait a bit longer than the
    /// 200 ms default.
    fn settle(&self) {
        std::thread::sleep(Duration::from_millis(400));
    }
}

/// Escapes a string so it can be embedded inside an AppleScript
/// double-quoted literal.
///
/// AppleScript string literals support only `\"` and `\\` as escapes
/// inside `"..."`. Newlines and tabs are not legal inside the literal
/// at all; we replace them with a `" & linefeed & "` /
/// `" & tab & "` concatenation so the resulting AppleScript still
/// parses.
///
/// ## Byte contract
///
/// Bytes outside printable UTF-8 plus LF (0x0A) and HT (0x09) are not
/// escaped. CR (0x0D), NUL (0x00), BEL (0x07), ESC (0x1B), and the
/// Unicode line / paragraph separators (U+2028 / U+2029) will produce
/// an AppleScript syntax error and are rejected via `debug_assert!` in
/// debug builds. Release builds pass the bytes through unmodified to
/// preserve best-effort behaviour.
///
/// ## Allocation
///
/// The output buffer is pre-allocated based on input shape: line or
/// tab characters expand to ~17 bytes apiece (`" & linefeed & "`), so
/// inputs containing either receive `s.len() * 2` capacity. Plain
/// inputs use the existing `s.len() + 8` budget.
///
/// ## Panics
///
/// In debug builds, panics via `debug_assert!` when `s` contains any
/// byte forbidden by the contract above.
fn applescript_escape(s: &str) -> String {
    let needs_concat = s.contains(['\n', '\t']);
    let mut out = if needs_concat {
        String::with_capacity(s.len() * 2)
    } else {
        String::with_capacity(s.len() + 8)
    };
    for ch in s.chars() {
        debug_assert!(
            is_applescript_safe(ch),
            "applescript_escape: forbidden character U+{:04X} would break AppleScript parse",
            ch as u32,
        );
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\" & linefeed & \""),
            '\t' => out.push_str("\" & tab & \""),
            _ => out.push(ch),
        }
    }
    out
}

/// Returns `true` when `ch` can be safely embedded in an AppleScript
/// double-quoted literal (after [`applescript_escape`] handles the
/// special-case characters `\\`, `"`, `\n`, `\t`).
///
/// Forbidden characters: CR (0x0D), NUL (0x00), BEL (0x07), ESC
/// (0x1B), and the Unicode line / paragraph separators (U+2028 /
/// U+2029). Each of these triggers a parse error in `osascript`.
fn is_applescript_safe(ch: char) -> bool {
    !matches!(
        ch,
        '\r' | '\u{0000}' | '\u{0007}' | '\u{001B}' | '\u{2028}' | '\u{2029}'
    )
}

/// Wraps `s` in single quotes and escapes embedded single quotes using
/// the POSIX `'\''` trick, matching the convention used by
/// [`crate::TerminalHarness::send_command_with_env`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applescript_escape_double_quote() {
        assert_eq!(applescript_escape(r#"a"b"#), r#"a\"b"#);
    }

    #[test]
    fn applescript_escape_backslash() {
        assert_eq!(applescript_escape(r"a\b"), r"a\\b");
    }

    #[test]
    fn applescript_escape_newline_concat() {
        assert_eq!(applescript_escape("a\nb"), "a\" & linefeed & \"b");
    }

    #[test]
    fn applescript_escape_tab_concat() {
        assert_eq!(applescript_escape("a\tb"), "a\" & tab & \"b");
    }

    #[test]
    fn applescript_escape_plain_passthrough() {
        assert_eq!(applescript_escape("hello world"), "hello world");
    }

    #[test]
    fn applescript_escape_combined() {
        // The shell command we'd send for the OSC8 fixture must round-trip
        // through the escape layer without losing its inner quotes.
        let s = r#"bt prose "<a href=\"https://example.com\">click here</a>""#;
        let got = applescript_escape(s);
        assert!(got.contains(r#"\""#));
        assert!(!got.contains('\n'));
    }

    #[test]
    fn shell_quote_simple() {
        assert_eq!(shell_quote("/usr/local/bin"), "'/usr/local/bin'");
    }

    #[test]
    fn shell_quote_embedded_single_quote() {
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    /// On non-macOS hosts `available()` must be false unconditionally so
    /// that Linux / CI environments skip these tests cleanly without
    /// shelling out to a missing `osascript`.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn available_is_false_off_macos() {
        assert!(!AppleTerminalHarness::available());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_rejects_cr() {
        let _ = applescript_escape("a\rb");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_rejects_nul() {
        let _ = applescript_escape("a\u{0000}b");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_rejects_esc() {
        let _ = applescript_escape("a\u{001B}b");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_unicode_line_separator_panics() {
        let _ = applescript_escape("a\u{2028}b");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_unicode_paragraph_separator_panics() {
        let _ = applescript_escape("a\u{2029}b");
    }

    /// Smoke test that the multi-line allocation policy still produces
    /// a coherent output for a 1024-char line/tab-heavy input.
    #[test]
    fn applescript_escape_preallocates_for_multiline() {
        // 1024 chars, half newlines + half tabs interleaved with text.
        let mut input = String::with_capacity(1024);
        for i in 0..256 {
            input.push('a');
            input.push('\n');
            input.push('b');
            input.push('\t');
            // Suppress unused-warning loops.
            let _ = i;
        }
        assert_eq!(input.len(), 1024);
        let got = applescript_escape(&input);
        // Every newline becomes `" & linefeed & "` (16 chars),
        // every tab becomes `" & tab & "` (11 chars), every plain
        // `a` / `b` stays one char.
        let linefeed_repl = "\" & linefeed & \"";
        let tab_repl = "\" & tab & \"";
        let expected_len = 256 * (1 + linefeed_repl.len() + 1 + tab_repl.len());
        assert_eq!(got.len(), expected_len);
        assert!(got.contains(linefeed_repl));
        assert!(got.contains(tab_repl));
    }

    /// Compile-time check: `wait_for_prompt` is the canonical
    /// shell-readiness path used by [`AppleTerminalHarness::spawn_shell`].
    /// This helper is never invoked at runtime; its sole purpose is to
    /// fail compilation if the call signature drifts.
    #[allow(dead_code)]
    fn _wait_for_prompt_path_typecheck(harness: &mut AppleTerminalHarness) {
        let _ = super::super::wait_for_prompt(harness);
    }

    /// On macOS `available()` must be false when `CI=1` so the harness
    /// does not try to open a Terminal.app window in CI.
    #[cfg(target_os = "macos")]
    #[test]
    fn available_is_false_in_ci() {
        // SAFETY: this test mutates a process-wide env var. Run it
        // serially (the cfg below already gates it to macOS, where the
        // serial cost is acceptable).
        let prev = env::var_os("CI");
        // SAFETY: single-threaded test process; no other threads
        // observe the env var while we toggle it. set_var is unsafe in
        // 2024 edition.
        unsafe {
            env::set_var("CI", "1");
        }
        let avail = AppleTerminalHarness::available();
        unsafe {
            match prev {
                Some(v) => env::set_var("CI", v),
                None => env::remove_var("CI"),
            }
        }
        assert!(!avail, "expected CI=1 to disable AppleTerminalHarness");
    }
}
