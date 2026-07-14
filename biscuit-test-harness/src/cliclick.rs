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
//!
//! [`accessibility_trusted`] is the second half of the gate: cliclick can be
//! installed yet still drop every event when the runner lacks macOS
//! Accessibility trust. Fold it into the availability check so a
//! missing-permission host skips like any other unprovisioned host instead of
//! red-failing.

#![allow(dead_code)]

use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Returns `true` when the `cliclick` binary is reachable.
pub fn available() -> bool {
    which("cliclick")
}

/// Returns `true` when the calling process holds macOS Accessibility
/// (assistive-access) trust — the permission cliclick REQUIRES to synthesize
/// real OS keyboard/pointer events.
///
/// Without this trust the WindowServer silently drops every injected event, so
/// a Level-3 interaction test would fail as though the *product* were broken
/// when the real cause is an unprovisioned host. Callers should fold this into
/// their availability gate so a missing-permission host SKIPS like any other
/// unprovisioned Level-3 host rather than red-failing.
///
/// ## Notes
///
/// Probes with a non-mutating System Events window enumeration. This exercises
/// the same Accessibility object graph used to select and raise a target
/// window; a no-op `keystroke ""` can return success even when that access is
/// denied. Any failure — including a missing `osascript` binary or a non-macOS
/// host — is reported as "not trusted" so callers gate conservatively (skip)
/// rather than proceed into a misleading interaction assertion. Always `false`
/// off macOS, which has no such permission model.
#[must_use]
pub fn accessibility_trusted() -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to count every window of every application process"#,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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

/// Moves the pointer to the absolute screen coordinates `(x, y)` without
/// clicking (`cliclick m:x,y`).
///
/// Used by Level-3 hover tests: a real `mouseMoved` over an element engages
/// its `:hover` state through the OS pointer path, unlike a synthetic CDP or
/// JS event.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn move_to(x: i32, y: i32) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run(&[&format!("m:{x},{y}")])
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

/// Click at `(x, y)` and press each key in the same `cliclick` process.
///
/// Keeping focus transfer and keyboard delivery in one invocation avoids a
/// race where another process or macOS activation transition intervenes
/// between separate `cliclick` launches. A 100 ms inter-event wait gives the
/// WindowServer time to make the clicked window key before the first press.
///
/// ## Errors
///
/// Returns an error when cliclick is unavailable or the invocation fails.
pub fn click_then_keys(x: i32, y: i32, keys: &[&str]) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    let mut args = vec!["-w".to_string(), "100".to_string(), format!("c:{x},{y}")];
    args.extend(keys.iter().map(|key| format!("kp:{key}")));
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    run(&args)
}

/// Click at `(x, y)` and type text in the same `cliclick` process.
///
/// This is useful for an OS-keyboard canary because ordinary text is not
/// reserved by browser chrome in the way Escape and function keys can be.
///
/// ## Errors
///
/// Returns an error when cliclick is unavailable or the invocation fails.
pub fn click_then_text(x: i32, y: i32, text: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run(&["-w", "100", &format!("c:{x},{y}"), &format!("t:{text}")])
}

/// Click at one point and move the pointer to another in one invocation.
///
/// The inter-event wait ensures the click has made the intended window key
/// before the movement is delivered, avoiding the same cross-process focus
/// race as [`click_then_keys`].
///
/// ## Errors
///
/// Returns an error when cliclick is unavailable or the invocation fails.
pub fn click_then_move(
    click_x: i32,
    click_y: i32,
    move_x: i32,
    move_y: i32,
) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run(&[
        "-w",
        "100",
        &format!("c:{click_x},{click_y}"),
        &format!("m:{move_x},{move_y}"),
    ])
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

/// Clicks at `(x, y)` and dispatches an Alt+`key` chord **in a single
/// cliclick invocation**.
///
/// Mirror of [`click_then_ctrl_chord`] for the Alt/Option modifier. On
/// macOS, the chord rides along with the letter keyDown as a normal
/// CGEvent (the modifier flag is set on the same event), so this path
/// reaches WezTerm even though bare-Option presses do not — see the
/// commentary in [`system_events_key_down`] for the flagsChanged
/// distinction.
///
/// ## Errors
///
/// Returns `io::Error` when cliclick is missing or returns non-zero.
pub fn click_then_alt_chord(x: i32, y: i32, key: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other("cliclick not installed"));
    }
    run_verbose(&[
        "-m",
        "verbose",
        "-w",
        "100",
        &format!("c:{x},{y}"),
        "kd:alt",
        &format!("t:{key}"),
        "ku:alt",
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
    let script = format!(r#"tell application "System Events" to key down {modifier}"#);
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
    let script = format!(r#"tell application "System Events" to key up {modifier}"#);
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

/// Raise and activate one window owned by an exact macOS process.
///
/// Both `process_id` and `title_contains` participate in selection. This keeps
/// Level-3 input from landing in another window of the same browser application
/// or in a separately running browser instance. The title token is passed as an
/// `osascript` argument rather than interpolated into source.
///
/// ## Errors
///
/// Returns an error when System Events cannot access the process or when the
/// process does not have exactly one window containing the title token.
pub fn activate_process_window(process_id: u32, title_contains: &str) -> io::Result<()> {
    const SCRIPT: &str = r#"
on run argv
    set targetPid to (item 1 of argv) as integer
    set titleToken to item 2 of argv
    tell application "System Events"
        set targetProcess to first application process whose unix id is targetPid
        set matchingWindows to {}
        repeat with candidateWindow in (every window of targetProcess)
            if (name of candidateWindow) contains titleToken then
                set end of matchingWindows to candidateWindow
            end if
        end repeat
        if (count of matchingWindows) is not 1 then
            set errorMessage to "process " & targetPid & " has " & (count of matchingWindows) & " windows containing " & titleToken & "; expected exactly one"
            error errorMessage
        end if
        tell targetProcess
            set frontmost to true
            tell item 1 of matchingWindows
                perform action "AXRaise"
            end tell
        end tell
    end tell
end run
"#;
    let out = Command::new("osascript")
        .args(["-e", SCRIPT, "--", &process_id.to_string(), title_contains])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "failed to activate process {process_id} window containing {title_contains:?}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// Resolve the macOS Accessibility process that owns one nonce-titled window.
///
/// Chrome may hand a headed launch from its initial child process to a distinct
/// application process. A harness can give its page a unique title, resolve
/// that exact window's actual owner here, and then pass the returned PID plus
/// the same title to [`activate_process_window`].
///
/// ## Errors
///
/// Returns an error when System Events cannot enumerate windows or when the
/// title token does not identify exactly one window system-wide.
pub fn process_id_for_window(title_contains: &str) -> io::Result<u32> {
    const SCRIPT: &str = r#"
on run argv
    set titleToken to item 1 of argv
    tell application "System Events"
        set owningProcessIds to {}
        repeat with candidateProcess in every application process
            repeat with candidateWindow in every window of candidateProcess
                if (name of candidateWindow) contains titleToken then
                    set end of owningProcessIds to unix id of candidateProcess
                end if
            end repeat
        end repeat
        if (count of owningProcessIds) is not 1 then
            set errorMessage to "found " & (count of owningProcessIds) & " windows containing " & titleToken & "; expected exactly one"
            error errorMessage
        end if
        return item 1 of owningProcessIds
    end tell
end run
"#;
    let out = Command::new("osascript")
        .args(["-e", SCRIPT, "--", title_contains])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "failed to resolve window containing {title_contains:?}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|error| {
            io::Error::other(format!(
                "window owner PID for {title_contains:?} was not numeric: {error}"
            ))
        })
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
