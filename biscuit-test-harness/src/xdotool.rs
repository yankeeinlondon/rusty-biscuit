//! Level-3 OS keyboard injection via `xdotool` (Linux / X11).
//!
//! The Linux counterpart to [`crate::cliclick`]. `xdotool key` synthesizes
//! events through the X11 **XTEST** extension, which injects at the server's
//! input pipeline — the same path a physical keyboard takes. The event is
//! therefore delivered to whichever window currently holds input focus and must
//! be encoded by *that terminal* before the application sees any bytes. This is
//! what makes it Level 3: `tmux send-keys` and `wezterm cli send-text` write
//! bytes straight to a pane's stdin and never exercise the encoder.
//!
//! ## Why `--window` is never passed
//!
//! `xdotool key --window <id>` switches from XTEST to `XSendEvent`, whose
//! events carry the `send_event` flag set. Terminals — WezTerm, xterm, and
//! Kitty among them — deliberately ignore flagged events as untrusted input.
//! Passing `--window` would silently produce a test that injects nothing and
//! then fails as though the product were broken. Focus the target window first
//! and inject globally instead.
//!
//! ## Skip semantics
//!
//! [`available`] requires the `xdotool` binary *and* a live `DISPLAY`. Wayland
//! sessions have no XTEST equivalent reachable this way (they would need
//! `ydotool` plus a uinput permission grant), so a Wayland host reports
//! unavailable and its Level-3 tests skip rather than red-fail.

#![allow(dead_code)]

use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Returns `true` when real X11 key injection is possible on this host.
///
/// Both halves matter: `xdotool` can be installed on a Wayland or headless
/// session where XTEST injection silently goes nowhere, and `DISPLAY` is what
/// distinguishes those from a usable X server.
#[must_use]
pub fn available() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_some() && which("xdotool")
}

/// Resolves the single X11 window whose title matches `title_pattern`.
///
/// `title_pattern` is an `xdotool search --name` regular expression matched
/// against window titles.
///
/// ## Errors
///
/// Returns an error when `xdotool` is unavailable, when no window matches, or
/// when **more than one** matches. Ambiguity is fatal rather than
/// first-match-wins: injecting a Ctrl+C into the wrong window is precisely the
/// failure this gate exists to prevent, and a caller told "3 windows match" can
/// close the extras, whereas a silent wrong-window pick looks like a product
/// regression.
pub fn window_id_for_title(title_pattern: &str) -> io::Result<u64> {
    if !available() {
        return Err(io::Error::other("xdotool + DISPLAY not available"));
    }
    let out = Command::new("xdotool")
        .args(["search", "--name", title_pattern])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "xdotool search --name {title_pattern:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let ids: Vec<u64> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect();
    match ids.as_slice() {
        [id] => Ok(*id),
        [] => Err(io::Error::other(format!(
            "no X11 window title matched {title_pattern:?}"
        ))),
        many => Err(io::Error::other(format!(
            "{} X11 windows matched {title_pattern:?}; expected exactly one \
             (close the others before running Level 3)",
            many.len()
        ))),
    }
}

/// Gives `window` input focus and waits for the X server to confirm it.
///
/// `--sync` blocks until the activation is actually visible, which is the
/// Linux equivalent of the macOS "poll until frontmost" step: `windowactivate`
/// without it returns before the window manager has moved focus, so a chord
/// injected immediately after would land in the previously focused window.
///
/// ## Errors
///
/// Returns an error when `xdotool windowactivate` fails — most often a window
/// manager that refuses programmatic activation.
pub fn activate_window(window: u64) -> io::Result<()> {
    let out = Command::new("xdotool")
        .args(["windowactivate", "--sync", &window.to_string()])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "xdotool windowactivate --sync {window} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    // `--sync` confirms the activation, not that the terminal has finished
    // repainting and re-arming its input handler. A short settle keeps the
    // first chord from being dropped by a client still processing FocusIn.
    std::thread::sleep(Duration::from_millis(250));
    Ok(())
}

/// Presses `modifier`+`key` as a real key event aimed at the focused window.
///
/// `--clearmodifiers` temporarily releases any modifier the *user* is holding
/// (or that a previous injection leaked) and restores it afterwards, so a stuck
/// Shift does not turn the intended chord into a different one.
///
/// ## Errors
///
/// Returns an error when `xdotool` is unavailable or the invocation fails.
pub fn chord(modifier: &str, key: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("xdotool + DISPLAY not available"));
    }
    let combo = format!("{modifier}+{key}");
    let out = Command::new("xdotool")
        .args(["key", "--clearmodifiers", &combo])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "xdotool key {combo} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(())
}

/// Focuses the window titled `title_pattern` and presses Ctrl+`key` in it.
///
/// The Linux analogue of [`crate::cliclick::click_then_ctrl_chord`]: resolve
/// one unambiguous window, take focus, then inject. Kept as one call so callers
/// cannot accidentally inject without having established focus first.
///
/// ## Errors
///
/// Returns an error when the window cannot be uniquely resolved, cannot be
/// activated, or when the injection fails.
pub fn focus_then_ctrl_chord(title_pattern: &str, key: &str) -> io::Result<()> {
    let window = window_id_for_title(title_pattern)?;
    activate_window(window)?;
    chord("ctrl", key)
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
