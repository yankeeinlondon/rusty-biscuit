//! Level-3 OS keyboard injection on Windows.
//!
//! The Windows counterpart to [`crate::cliclick`] and [`crate::xdotool`].
//!
//! ## Why this is Level 3 and `GenerateConsoleCtrlEvent` is not
//!
//! `GenerateConsoleCtrlEvent` posts a console-control notification directly to
//! a process group. It is a genuine test of the *application's* signal path,
//! but it starts downstream of the keyboard and of the terminal's input
//! encoder — no key is pressed and no terminal translates one. A requirement
//! phrased as "the user presses Ctrl+C" is only discharged by an event that
//! enters where a real keypress enters.
//!
//! [`SendKeys.SendWait`] does exactly that: it drives `keybd_event`/`SendInput`,
//! so the keystroke is injected into the system input queue, delivered to
//! whichever window owns focus, and encoded by that terminal. Both layers are
//! worth keeping — the console-control fixture stays as the lower-level
//! diagnostic that isolates the signal path when the Level-3 test fails.
//!
//! ## The foreground lock
//!
//! Windows refuses `SetForegroundWindow` from a process that does not already
//! own the foreground, to stop applications from stealing focus. The documented
//! release condition is user keyboard input, so [`focus_then_ctrl_chord`] taps
//! ALT through `keybd_event` immediately before the call. Without that nudge the
//! activation is silently downgraded to a flashing taskbar button and the chord
//! lands in whatever window the user was actually using.
//!
//! ## Skip semantics
//!
//! [`available`] is `false` off Windows and on any host without a working
//! `powershell`, so Level-3 tests skip cleanly instead of red-failing.
//!
//! [`SendKeys.SendWait`]: https://learn.microsoft.com/dotnet/api/system.windows.forms.sendkeys.sendwait

#![allow(dead_code)]

use std::io;
use std::process::{Command, Stdio};

/// The PowerShell driver: resolve one window by title, take the foreground,
/// then inject a real Ctrl+`key` chord.
///
/// The title token and the key arrive through the environment rather than being
/// interpolated into this source, so no caller-supplied text is ever parsed as
/// PowerShell.
const FOCUS_AND_CHORD: &str = r#"
$ErrorActionPreference = 'Stop'
$token = $env:BISCUIT_L3_TITLE_TOKEN
$key   = $env:BISCUIT_L3_CHORD_KEY

Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class BiscuitL3 {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern void keybd_event(
        byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
}
"@

$hits = @(Get-Process |
    Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -like "*$token*" })
if ($hits.Count -ne 1) {
    $seen = ($hits | ForEach-Object { $_.MainWindowTitle }) -join '; '
    throw "$($hits.Count) windows matched '$token'; expected exactly one. Matched: $seen"
}
$handle = $hits[0].MainWindowHandle

# SW_RESTORE: a minimized window cannot receive the injected keystroke.
[void][BiscuitL3]::ShowWindow($handle, 9)

# Tap ALT to release the foreground lock (see module docs), then activate.
[BiscuitL3]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
[BiscuitL3]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
[void][BiscuitL3]::SetForegroundWindow($handle)
Start-Sleep -Milliseconds 400

if ([BiscuitL3]::GetForegroundWindow() -ne $handle) {
    throw "window '$token' did not become foreground; the chord would land elsewhere"
}

[System.Windows.Forms.SendKeys]::SendWait("^$key")
"#;

/// Returns `true` when real Windows key injection is possible on this host.
#[must_use]
pub fn available() -> bool {
    cfg!(windows) && powershell_usable()
}

/// Focuses the window whose title contains `title_token` and presses Ctrl+`key`.
///
/// Selection is by unique match: zero or several matching windows is an error
/// rather than a first-match guess, because injecting Ctrl+C into the wrong
/// window looks identical to a broken product.
///
/// ## Errors
///
/// Returns an error when Windows injection is unavailable, when `title_token`
/// does not identify exactly one window, when the window cannot be brought to
/// the foreground, or when the PowerShell driver fails.
pub fn focus_then_ctrl_chord(title_token: &str, key: &str) -> io::Result<()> {
    if !available() {
        return Err(io::Error::other(
            "Windows key injection unavailable (needs Windows + powershell)",
        ));
    }
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", FOCUS_AND_CHORD])
        .env("BISCUIT_L3_TITLE_TOKEN", title_token)
        .env("BISCUIT_L3_CHORD_KEY", key)
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "Windows Ctrl+{key} injection into window {title_token:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(())
}

fn powershell_usable() -> bool {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
