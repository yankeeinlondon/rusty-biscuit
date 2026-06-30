//! F2 — Windows captured-stdout CLI-boundary verification.
//!
//! Closes the High review finding "Windows captured-stdout behavior is not
//! verified at the required level" by exercising the real `question` process
//! boundary (not the in-process `SetStdHandle` handle tests in
//! `lib/src/core/standalone/tests.rs`).
//!
//! Contract under test (spec F2): on Windows, `question` invoked with stdout
//! captured to a pipe while a console is still attached must render the prompt
//! to the console (`CONOUT$`) and deliver ONLY the submitted value to the
//! captured stream — no TUI chrome, no ANSI/escape (`0x1b`) bytes.
//!
//! ## Why this is an executable, cross-compile-checked test (not a manual gate)
//!
//! Earlier this file returned early unless `BISCUIT_TUI_WINDOWS_CONSOLE_TEST=1`
//! AND a human pressed Enter, so it "never ran" in any automated suite. That gap
//! is closed here: the Enter keystroke is now injected **deterministically**
//! into the attached console's input buffer with `WriteConsoleInputW`, so no
//! human keypress is required. The test follows the proven claudine precedent
//! (`claudine/cli/tests/level3_wrap_ctrl_c.rs`): it is `#[cfg(windows)]` and
//! `#[ignore]`d (not early-returning), so it COMPILES on every Windows target
//! and RUNS only when explicitly invoked with `--ignored` on a real Windows
//! console.
//!
//! ## Why `WriteConsoleInputW` (not crossterm event injection)
//!
//! The prompt's event loop reads physical key events from the console input
//! buffer via the Win32 console API. Writing synthetic `KEY_EVENT` records with
//! `WriteConsoleInputW` is exactly what a physical keypress produces at the OS
//! boundary, so the submit travels the same path a user's Enter would — no
//! crossterm-internal seam, no behavior shortcut.
//!
//! ## Why stderr is inherited
//!
//! stdout is piped (the `FOO=$(question ...)` capture shape under test). stderr
//! stays inherited so the console remains attached for the prompt to render
//! into; piping both would trip the standalone headless guard (both streams
//! `!is_terminal` → `no interactive terminal available`) and the captured shape
//! would never occur.
//!
//! ## Verification status
//!
//! The dev host is macOS, so this test's runtime pass must be observed on a
//! Windows host / CI; on macOS it is *cross-compile-checked* for
//! `x86_64-pc-windows-gnu`, which is the maximum honest verification from this
//! host. The Windows-host gates are the path-filtered
//! `.github/workflows/biscuit-tui-windows-captured-stdout.yml` workflow and
//! `just test-windows-captured-stdout`. The repro recipe lives in
//! `features/2026-06-19-review-findings/windows-captured-stdout-repro.md`.
//!
//! Until a green Windows-host run is recorded the path is **compile-checked, not
//! yet runtime-confirmed** — this test plus its workflow/just recipe are the
//! executable harness that closes the gap, not a claim it has already passed on
//! Windows.
#![cfg(windows)]

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::TRUE;
use windows::Win32::System::Console::{
    GetStdHandle, INPUT_RECORD, INPUT_RECORD_0, KEY_EVENT, KEY_EVENT_RECORD,
    KEY_EVENT_RECORD_0, STD_INPUT_HANDLE, WriteConsoleInputW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;

/// Write a single Enter (VK_RETURN) keydown+keyup pair into the attached
/// console's input buffer so the running `question choose-one` prompt submits
/// the default-highlighted first option.
///
/// Returns whether both records were accepted by the console.
fn inject_enter() -> bool {
    // SAFETY: `GetStdHandle(STD_INPUT_HANDLE)` returns the process's standard
    // input handle (the attached console's input buffer). It does not transfer
    // ownership and must not be closed; we only read from it via
    // `WriteConsoleInputW` below. An invalid/redirected handle surfaces as an
    // `Err` here, which the caller treats as "could not inject".
    let stdin = match unsafe { GetStdHandle(STD_INPUT_HANDLE) } {
        Ok(h) => h,
        Err(_) => return false,
    };

    // One keydown record and one keyup record. `uChar` carries `\r` so consumers
    // that read the translated character (not just the virtual-key code) also see
    // a carriage return, matching a physical Enter.
    let make = |down: bool| INPUT_RECORD {
        EventType: KEY_EVENT as u16,
        Event: INPUT_RECORD_0 {
            KeyEvent: KEY_EVENT_RECORD {
                bKeyDown: if down { TRUE } else { Default::default() },
                wRepeatCount: 1,
                wVirtualKeyCode: VK_RETURN.0,
                wVirtualScanCode: 0,
                uChar: KEY_EVENT_RECORD_0 {
                    UnicodeChar: u16::from(b'\r'),
                },
                dwControlKeyState: 0,
            },
        },
    };
    let records = [make(true), make(false)];

    let mut written: u32 = 0;
    // SAFETY: `records` is a valid, fully-initialized slice of two
    // `INPUT_RECORD`s that outlives this single synchronous call; `&mut written`
    // is a valid out-param. `WriteConsoleInputW` neither retains the pointer nor
    // mutates beyond `written`.
    let ok = unsafe { WriteConsoleInputW(stdin, &records, &mut written) };
    ok.is_ok() && written == records.len() as u32
}

/// On a real Windows console, `question` with stdout captured to a pipe must
/// render its prompt to the console and deliver ONLY the submitted value to the
/// captured stream — no ESC (`0x1b`) byte, i.e. no TUI/ANSI chrome.
///
/// `#[ignore]`d because it needs a real attached console to render into and to
/// accept the injected console-input record. The path-filtered
/// `.github/workflows/biscuit-tui-windows-captured-stdout.yml` workflow runs it
/// in CI; to run the same gate manually on a Windows host:
///
/// ```text
/// just test-windows-captured-stdout
/// ```
#[test]
#[ignore = "requires a Windows host with an attached console; cross-compile-checked but not runtime-run on the macOS dev host"]
fn captured_stdout_receives_only_value_no_tui_bytes() {
    // Spawn `question` with stdout captured to a pipe (the `FOO=$(question ...)`
    // shape) while stderr/stdin stay attached so the console renders the prompt
    // and our injected key event reaches it.
    let mut child = Command::new(env!("CARGO_BIN_EXE_question"))
        .args(["choose-one", "Red", "Green", "Blue"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn question");

    // Let the prompt render its first frame and install its event loop before we
    // inject the submit keystroke; injecting earlier could race the loop and the
    // record would be discarded. The delay is bounded (deterministic, no human
    // in the loop).
    thread::sleep(Duration::from_millis(750));
    let injected = inject_enter();
    if !injected {
        // A second deterministic Enter covers the rare case where the first
        // record landed before the loop was reading. Still no human keypress.
        thread::sleep(Duration::from_millis(250));
        let _ = inject_enter();
    }

    let mut captured = Vec::new();
    child
        .stdout
        .take()
        .expect("captured stdout pipe")
        .read_to_end(&mut captured)
        .expect("read captured stdout");
    let status = child.wait().expect("wait for question");

    assert!(status.success(), "question exited non-zero: {status:?}");

    // The captured stream must carry ONLY the submitted value: no ESC (0x1b)
    // and therefore no ANSI/TUI chrome — the prompt went to the console.
    assert!(
        !captured.contains(&0x1b),
        "captured stdout must contain no ESC/TUI bytes, got: {captured:?}",
    );

    let value = String::from_utf8_lossy(&captured);
    let value = value.trim();
    assert!(
        ["Red", "Green", "Blue"].contains(&value),
        "captured stdout must hold exactly the submitted option value, got {value:?}",
    );
}
