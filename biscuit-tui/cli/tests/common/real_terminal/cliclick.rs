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
    // Give the WindowServer time to settle focus before we inject keys.
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

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
