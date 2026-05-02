//! Shared real-terminal test harness for the rusty-biscuit monorepo.
//!
//! Provides [`TerminalHarness`] implementations for WezTerm, Kitty, and
//! tmux, plus common utilities like [`CapturedFrame`], [`strip_ansi`],
//! and [`skip_with_reason`].
//!
//! ## Testing vocabulary
//!
//! - **Level 1** — PTY-based tests. We generate input bytes; the binary
//!   parses them. No real terminal required.
//! - **Level 2** — Run-in-real-terminal with IPC. The binary renders
//!   through the real terminal's display path so glyph-width / SGR /
//!   scroll-handling regressions are observable in captured pane text.
//!   Input is injected as bytes via the terminal's CLI.
//! - **Level 3** — OS-level keyboard injection (not covered by this
//!   harness; see individual test suites for platform-specific tools).
//!
//! Every harness's `available()` probe returns `false` cleanly when its
//! required tooling is missing — tests then print `skipping: requires`
//! and return `ok` without exercising the harness.

#![allow(dead_code)]

use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub mod cliclick;
pub mod kitty;
pub mod tmux;
pub mod wezterm;

/// Result returned by any [`TerminalHarness`] when it captures the
/// rendered pane text plus useful metadata.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    /// Text content of the pane, including ANSI sequences when the
    /// underlying tool returns them.
    pub raw: String,
    /// Plain-text content with ANSI/CSI sequences stripped.
    pub plain: String,
}

impl CapturedFrame {
    pub fn from_raw(raw: String) -> Self {
        let plain = strip_ansi(&raw);
        Self { raw, plain }
    }
}

/// Common contract every real-terminal harness implements.
///
/// Implementors are responsible for spawning the binary, sending
/// synthetic input as raw bytes, capturing the rendered pane text,
/// and tearing the session down on drop.
pub trait TerminalHarness {
    /// Spawns `program` with `args` inside a fresh pane / window.
    ///
    /// Returns `Ok(())` when the spawn succeeded. Implementors should
    /// arrange that subsequent calls to [`send_text`](Self::send_text)
    /// and [`capture`](Self::capture) target the new pane.
    fn spawn(&mut self, program: &str, args: &[&str]) -> io::Result<()>;

    /// Sends raw bytes to the spawned pane's stdin.
    ///
    /// Use this for terminal escape sequences (Ctrl+chord, kitty
    /// keyboard-protocol bytes, etc.). Note: the terminal's *input
    /// encoder* is bypassed — you are writing what would normally
    /// appear on the pane's stdin.
    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Captures the current rendered text of the spawned pane.
    fn capture(&mut self) -> io::Result<CapturedFrame>;

    /// Sleeps long enough for the spawned process to react and the
    /// terminal to finish a redraw. Implementors override this when
    /// their tooling needs more (or less) settling time.
    fn settle(&self) {
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Strips CSI / OSC / SGR / charset-designation escape sequences from
/// `s`, returning a plain-text rendering of the visible glyphs.
///
/// Used by `CapturedFrame::from_raw` so test assertions can match on
/// option labels without contending with embedded styling sequences.
///
/// Implements ECMA-48 §5.4 escape-sequence shape:
/// `ESC (intermediate 0x20-0x2F)* (final 0x30-0x7E)`. This matters
/// because terminals often emit `ESC ( B` (designate ASCII) around
/// styled regions; a naive "ESC + 1 byte" stripper would leave the
/// final `B` behind and let it masquerade as text.
pub fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == b'[' {
                // CSI: 0x1b '[' params* final
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (0x40..=0x7e).contains(&b) {
                        break;
                    }
                }
                continue;
            }
            if next == b']' {
                // OSC: 0x1b ']' ... BEL or ESC '\\'
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if b == 0x07 {
                        break;
                    }
                    if b == 0x1b && i < bytes.len() && bytes[i] == b'\\' {
                        i += 1;
                        break;
                    }
                }
                continue;
            }
            // Generic ESC sequence: ESC (intermediate*) final.
            // Intermediate bytes are 0x20-0x2F, final byte is 0x30-0x7E.
            i += 1; // skip ESC
            while i < bytes.len() && (0x20..=0x2f).contains(&bytes[i]) {
                i += 1; // consume intermediate
            }
            if i < bytes.len() {
                i += 1; // consume final byte
            }
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Convenience: prints a "skipping: requires X" message to stderr and
/// returns `true` so tests can early-return.
///
/// ## Example
///
/// ```ignore
/// if !wezterm::WezTermHarness::available() {
///     return skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
/// }
/// ```
pub fn skip_with_reason(what: &str) -> bool {
    eprintln!("skipping: requires {what}");
    true
}

/// Detects a suitable shell using the fallback chain:
/// `$SHELL` → `bash` → `sh`.
pub fn detect_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .and_then(|s| {
            let path = std::path::Path::new(&s);
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .filter(|name| which(name))
        .or_else(|| {
            if which("bash") {
                Some("bash".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "sh".to_string())
}

/// Returns the directory that contains the built cargo binary named
/// `bin_name`, if it can be located.
///
/// This is useful for prepending to `PATH` in shell-model tests so
/// that the CLI binary (e.g. `bt` or `question`) resolves without an
/// absolute path.
pub fn cargo_bin_dir(bin_name: &str) -> Option<PathBuf> {
    // Try the path returned by assert_cmd when available.
    // We avoid a direct dependency by probing CARGO_TARGET_DIR first.
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        let dir = PathBuf::from(target_dir);
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let bin = dir.join(profile).join(bin_name);
        if bin.exists() {
            return Some(bin.parent()?.to_path_buf());
        }
    }

    // Derive from the current test executable location:
    // <target_dir>/<profile>/deps/<exe>  →  <target_dir>/<profile>
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf(); // deps/
    dir = dir.parent()?.to_path_buf(); // <profile>/
    let bin = dir.join(bin_name);
    if bin.exists() {
        return Some(dir);
    }
    None
}

/// Waits for a shell prompt to appear in the harness output.
///
/// Polls [`TerminalHarness::capture`] every 100 ms, looking for a
/// trailing `$`, `#`, or `%` character on the last line. Times out
/// after 5 seconds and returns silently so that callers don't hang.
pub fn wait_for_prompt(harness: &mut impl TerminalHarness) -> io::Result<()> {
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        let frame = harness.capture()?;
        if let Some(last_line) = frame.plain.lines().last() {
            let trimmed = last_line.trim_end();
            if trimmed.ends_with('$') || trimmed.ends_with('#') || trimmed.ends_with('%') {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn which(bin: &str) -> bool {
    std::process::Command::new("which")
        .arg(bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_csi() {
        assert_eq!(strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
    }

    #[test]
    fn strip_ansi_removes_nested_sgr() {
        assert_eq!(strip_ansi("\x1b[1;38;5;202m^F\x1b[0m"), "^F");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn strip_ansi_removes_osc() {
        assert_eq!(strip_ansi("before\x1b]0;title\x07after"), "beforeafter");
    }

    #[test]
    fn strip_ansi_removes_charset_designation() {
        assert_eq!(strip_ansi("\x1b(BHello"), "Hello");
        assert_eq!(strip_ansi("a\x1b(Bb\x1b(0c"), "abc");
    }
}
