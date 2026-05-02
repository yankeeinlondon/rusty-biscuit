//! Level-3 OS keyboard injection via `cliclick` (macOS).
//!
//! `cliclick` is a small Homebrew utility that synthesises real
//! `CGEventCreateKeyboardEvent` events at the macOS Quartz event
//! layer. Unlike `wezterm cli send-text` and `tmux send-keys` — which
//! write bytes to the pane's stdin and bypass the terminal's input
//! encoder — cliclick emits OS-level key presses that the *terminal*
//! must encode and forward. This is the only test surface that
//! verifies "what bytes does WezTerm actually emit when the user
//! presses bare Ctrl?", which is exactly the regression the
//! `REPORT_ALL_KEYS_AS_ESCAPE_CODES` flag fixed.
//!
//! ## Skip semantics
//!
//! [`available`] returns `false` when `cliclick` is not on `$PATH`.
//! Tests that depend on it must early-return with a "skipping:
//! requires cliclick" message rather than fail. macOS-only by
//! construction; on Linux/Windows callers should reach for a
//! platform-equivalent (`xdotool`, etc.) — none implemented here yet.

#![allow(dead_code)]

use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Returns `true` when the `cliclick` binary is reachable.
pub fn available() -> bool {
    which("cliclick")
}

/// Holds the modifier `key` (`"ctrl"` / `"alt"` / `"shift"` / `"cmd"`)
/// down for `duration`, then releases it.
///
/// Internally this issues two cliclick commands: `kd:<key>` to press
/// and `ku:<key>` to release. The terminal that has focus during the
/// hold receives real OS keyboard events.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn hold_modifier(key: &str, duration: Duration) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run(&[&format!("kd:{key}")])?;
    std::thread::sleep(duration);
    run(&[&format!("ku:{key}")])?;
    Ok(())
}

/// Presses `key` down without releasing it. Caller MUST balance with
/// [`key_up`] to avoid leaving the system in a stuck-modifier state.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn key_down(key: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run(&[&format!("kd:{key}")])
}

/// Releases a previously [`key_down`]'d key. Idempotent at the
/// cliclick level: a `ku:` for a key that isn't down is a no-op.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn key_up(key: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run(&[&format!("ku:{key}")])
}

/// Presses `key` once with no modifiers.
///
/// `key` follows cliclick's keystroke vocabulary (e.g. `"return"`,
/// `"space"`, `"esc"`, single characters via `t:c`).
pub fn press(key: &str) -> io::Result<()> {
    run(&[&format!("kp:{key}")])
}

/// Types the literal text `s` into the focused window.
pub fn type_text(s: &str) -> io::Result<()> {
    run(&[&format!("t:{s}")])
}

/// Clicks at the given absolute screen coordinates `(x, y)`.
///
/// Used by harnesses to force a specific window to become macOS
/// `keyWindow` after AXRaise — visual raising via System Events
/// doesn't always transfer keyboard focus across applications, but a
/// real OS click does. Pick coordinates inside the target window.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn click_at(x: i32, y: i32) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run(&[&format!("c:{x},{y}")])
}

/// Clicks at `(x, y)` and immediately presses `modifier` (e.g.
/// `"ctrl"`) **inside a single cliclick invocation**.
///
/// This is the focus-correct way to issue a held-modifier press to a
/// non-frontmost window: combining the focus-transfer click and the
/// modifier press into one cliclick process makes them atomic at the
/// OS event-tap level, so no other app can grab keyboard focus
/// between the click and the press.
///
/// `-w 100` inserts a 100ms wait between events (cliclick's `-w` is
/// additive across all events in the invocation), giving the
/// WindowServer time to deliver the click-induced focus change
/// before the modifier press fires.
///
/// The modifier is **left held** at the OS level after this returns;
/// the caller MUST balance with [`key_up`] or release will leak and
/// affect subsequent OS keyboard input.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn click_then_press(x: i32, y: i32, modifier: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run_verbose(&[
        "-m",
        "verbose",
        "-w",
        "100",
        &format!("c:{x},{y}"),
        &format!("kd:{modifier}"),
    ])
}

/// Clicks at `(x, y)` and dispatches a Ctrl+`key` chord **in a single
/// cliclick invocation**.
///
/// Same atomicity rationale as [`click_then_press`]. Use this for any
/// momentary chord (Ctrl-letter, Alt-letter, etc.) where you want
/// focus transfer and the chord to land together.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn click_then_ctrl_chord(x: i32, y: i32, key: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run_verbose(&[
        "-m",
        "verbose",
        "-w",
        "100",
        &format!("c:{x},{y}"),
        "kd:ctrl",
        &format!("t:{key}"),
        "ku:ctrl",
    ])
}

/// Holds a bare modifier (e.g. `"control"`, `"option"`, `"command"`,
/// `"shift"`) down via AppleScript / System Events — **not** cliclick.
///
/// On macOS, bare-modifier presses propagate through AppKit's
/// `flagsChanged` event type. cliclick's `kd:ctrl` uses
/// `CGEventCreateKeyboardEvent`, which dispatches a regular keyboard
/// event and rarely reaches AppKit apps as a flagsChanged event.
/// System Events `key down <modifier>` is the AppleScript-level
/// equivalent that DOES dispatch a real flagsChanged event, because
/// System Events is itself an AppKit consumer/producer.
///
/// Use this for the held-modifier portion of bare-Ctrl Level-3 tests.
/// Pair with [`system_events_key_up`] for symmetric release.
///
/// ## Errors
///
/// Returns `io::Error` when osascript fails (typically: missing
/// Accessibility permission for the calling app).
pub fn system_events_key_down(modifier: &str) -> io::Result<()> {
    let script = format!(
        r#"tell application "System Events" to key down {modifier}"#
    );
    let out = Command::new("osascript").args(["-e", &script]).output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "System Events key down {modifier} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(())
}

/// Releases a bare modifier previously pressed by
/// [`system_events_key_down`]. Idempotent at the AppleScript level.
///
/// ## Errors
///
/// Returns `io::Error` when osascript fails.
pub fn system_events_key_up(modifier: &str) -> io::Result<()> {
    let script = format!(
        r#"tell application "System Events" to key up {modifier}"#
    );
    let out = Command::new("osascript").args(["-e", &script]).output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "System Events key up {modifier} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(())
}

/// Activates a macOS application by name (e.g. `"WezTerm"`,
/// `"kitty"`) so that subsequent keyboard injection lands in its
/// frontmost window. Implemented via `osascript`; cliclick has no
/// "activate app" primitive of its own.
pub fn activate_app(app_name: &str) -> io::Result<()> {
    let script = format!("tell application \"{app_name}\" to activate");
    let status = Command::new("osascript")
        .args(["-e", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "osascript activate of {app_name} failed"
        )));
    }
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

fn run(args: &[&str]) -> io::Result<()> {
    let out = Command::new("cliclick").args(args).output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "cliclick {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

/// Same as [`run`] but mirrors cliclick's stdout/stderr (verbose mode
/// prints what's being executed) to the test process's stderr so
/// Level-3 failures produce actionable diagnostic output. cliclick's
/// `-m verbose` output also reveals when the OS Accessibility check
/// failed.
fn run_verbose(args: &[&str]) -> io::Result<()> {
    let out = Command::new("cliclick").args(args).output()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stdout.trim().is_empty() {
        eprintln!("[cliclick stdout] {}", stdout.trim());
    }
    if !stderr.trim().is_empty() {
        eprintln!("[cliclick stderr] {}", stderr.trim());
    }
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "cliclick {args:?} failed: {stderr}"
        )));
    }
    Ok(())
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
