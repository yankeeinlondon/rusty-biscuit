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
//! ## Why the test self-establishes and PROVES the console precondition
//!
//! A bare `cargo test` under GitHub Actions runs with its stdout/stderr wired to
//! pipes, not a console — Actions captures the process. A child `question` would
//! then inherit a non-console stderr and take the headless bail path
//! (`run_standalone_with_chrome` errors when both stdout AND stderr are
//! non-terminals), so a green run would NOT exercise the attached-console
//! contract at all. `--nocapture` only disables the Rust harness buffer; it does
//! not turn the Actions pipe into a console.
//!
//! To make a green run real evidence, the test does not *assume* a console: it
//! **attaches one** with `AllocConsole` (a no-op-equivalent when one already
//! exists), rewires the process std handles onto `CONOUT$` / `CONIN$` so the
//! inherited child streams are real console buffers, and then **proves** the
//! precondition — `stderr.is_terminal()` AND `CONOUT$` openable — panicking with
//! a specific diagnostic if either fails. Because the test is `#[ignore]`d and
//! run via `--ignored`, that panic surfaces as a visible CI failure. It also
//! prints a `F2 precondition HELD` line so a green Actions log *records* that the
//! precondition genuinely held, exactly as the review demands.
//!
//! ## Why stdout is piped but stderr/stdin are inherited consoles
//!
//! stdout is piped (the `FOO=$(question ...)` capture shape under test). stderr
//! stays a console so the headless guard is not tripped (both streams
//! `!is_terminal` → `no interactive terminal available`) and so the prompt has a
//! real console to render into; stdin stays a console so the injected
//! `WriteConsoleInputW` Enter reaches the child's event loop.
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
//! Because the test now proves the attached-console precondition (and fails
//! loudly if it cannot establish one), a green Windows-host run IS reliable
//! evidence that the captured-stdout-with-console contract held — not merely
//! that the binary compiled.
#![cfg(windows)]

use std::io::{IsTerminal, Read};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, TRUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Console::{
    AllocConsole, GetStdHandle, INPUT_RECORD, INPUT_RECORD_0, KEY_EVENT, KEY_EVENT_RECORD,
    KEY_EVENT_RECORD_0, STD_ERROR_HANDLE, STD_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    SetStdHandle, WriteConsoleInputW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;
use windows::core::PCWSTR;

/// NUL-terminated UTF-16 `CONOUT$` — the console screen buffer device path that
/// resolves to the active console even after the process's standard handles have
/// been redirected (the Windows analog of `/dev/tty`).
const CONOUT: [u16; 8] = utf16z(b"CONOUT$\0");
/// NUL-terminated UTF-16 `CONIN$` — the console input buffer device path.
const CONIN: [u16; 7] = utf16z(b"CONIN$\0");

/// Widen an ASCII, NUL-terminated byte literal to a UTF-16 array of the same
/// length in a `const` context.
const fn utf16z<const N: usize>(bytes: &[u8; N]) -> [u16; N] {
    let mut out = [0u16; N];
    let mut i = 0;
    while i < N {
        out[i] = bytes[i] as u16;
        i += 1;
    }
    out
}

/// Open a console device (`CONOUT$` / `CONIN$`) for read+write, returning the
/// handle or `None` on failure. Callers own the returned handle and must close
/// it.
fn open_console_device(path: &[u16]) -> Option<HANDLE> {
    // SAFETY: `path` is a static, NUL-terminated UTF-16 buffer that outlives the
    // call; the access/share/disposition arguments are documented constants. No
    // security attributes and no template handle are passed (`None`).
    // `CreateFileW` returns `Err` for an invalid handle, which we map to `None`.
    unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
        .ok()
    }
}

/// Point a process standard handle (`STD_OUTPUT_HANDLE` / `STD_ERROR_HANDLE` /
/// `STD_INPUT_HANDLE`) at a freshly opened console device so a child spawned
/// with `Stdio::inherit()` inherits a real console for that stream.
///
/// Returns whether the redirect was installed. The opened device handle is
/// intentionally leaked: it must outlive every child spawn in this test, and the
/// process exits immediately after, so an explicit close would only risk
/// invalidating the std handle the child still needs.
fn redirect_std_handle_to_console(std_handle: STD_HANDLE, device: &[u16]) -> bool {
    let Some(handle) = open_console_device(device) else {
        return false;
    };
    // SAFETY: `std_handle` is a valid STD_* identifier; `handle` is the console
    // device just opened via `CreateFileW`. `SetStdHandle` only records the new
    // value for subsequent `GetStdHandle` / inherited-handle resolution; it does
    // not take ownership of, or close, any prior handle.
    unsafe { SetStdHandle(std_handle, handle).is_ok() }
}

/// Attach a console to this process if it has none, then wire the process std
/// handles to real console buffers so an inheriting child sees a console on
/// stdout (unused here — child stdout is piped), stderr, and stdin.
///
/// `AllocConsole` allocates a fresh console for a process that has none; if one
/// already exists it fails (classically `ERROR_ACCESS_DENIED`), which is the
/// benign "console already present" case — we still rewire the std handles so
/// they point at console devices even if a prior step (e.g. Actions) redirected
/// them to pipes. Returns a human-readable description of the `AllocConsole`
/// outcome for the precondition log.
fn establish_console() -> String {
    // SAFETY: `AllocConsole` takes no arguments and only affects this process's
    // console association. On success it resets the std handles to the new
    // console; on "already attached" it is a no-op for our purposes. Either way
    // we re-derive the std handles below.
    let alloc = match unsafe { AllocConsole() } {
        Ok(()) => "AllocConsole=allocated".to_string(),
        Err(e) => format!("AllocConsole=already-present-or-failed ({e})"),
    };

    // Re-point stderr and stdin at real console devices. AllocConsole resets the
    // std handles on a fresh allocation, but when a console already existed and
    // the parent (CI) had redirected stderr/stdin to pipes, the reset does not
    // happen — opening CONOUT$/CONIN$ and SetStdHandle-ing them guarantees the
    // inherited child streams are console buffers regardless.
    let err_ok = redirect_std_handle_to_console(STD_ERROR_HANDLE, &CONOUT);
    let in_ok = redirect_std_handle_to_console(STD_INPUT_HANDLE, &CONIN);
    // stdout is rewired too for completeness, though this test pipes the child's
    // stdout rather than inheriting it.
    let out_ok = redirect_std_handle_to_console(STD_OUTPUT_HANDLE, &CONOUT);

    format!("{alloc}; std-redirect out={out_ok} err={err_ok} in={in_ok}")
}

/// Prove the F2 precondition: this process's stderr is a real console
/// (`is_terminal`) AND `CONOUT$` is openable. Panics with a specific diagnostic
/// otherwise — under `--ignored` that is a visible CI failure, which is the
/// point: a passing run now means the attached-console contract was genuinely
/// exercised.
fn assert_console_precondition(alloc_summary: &str) {
    let stderr_is_console = std::io::stderr().is_terminal();
    let conout = open_console_device(&CONOUT);
    let conout_usable = conout.is_some();
    if let Some(h) = conout {
        // SAFETY: `h` is the handle just opened by `open_console_device`; it has
        // not been published anywhere, so this is its single close.
        unsafe {
            let _ = CloseHandle(h);
        }
    }

    assert!(
        stderr_is_console && conout_usable,
        "F2 precondition NOT met after console setup [{alloc_summary}]: \
         stderr.is_terminal()={stderr_is_console}, CONOUT$ openable={conout_usable} — \
         this run does NOT verify the captured-stdout-with-console contract",
    );

    // Recorded in the (--nocapture) CI log so a green run documents that the
    // precondition genuinely held, not merely that the binary compiled.
    println!(
        "F2 precondition HELD: console attached, stderr.is_terminal()=true, CONOUT$ usable [{alloc_summary}]"
    );
}

/// Write a single Enter (VK_RETURN) keydown+keyup pair into the attached
/// console's input buffer so the running `question choose-one` prompt submits
/// the default-highlighted first option.
///
/// Returns whether both records were accepted by the console.
fn inject_enter() -> bool {
    // SAFETY: `GetStdHandle(STD_INPUT_HANDLE)` returns the process's standard
    // input handle (the attached console's input buffer). It does not transfer
    // ownership and must not be closed; we only write to it via
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
/// accept the injected console-input record. The test self-establishes that
/// console (`AllocConsole` + `SetStdHandle` onto `CONOUT$`/`CONIN$`) and fails
/// loudly via [`assert_console_precondition`] if it cannot, so a green run is
/// real evidence. The path-filtered
/// `.github/workflows/biscuit-tui-windows-captured-stdout.yml` workflow runs it
/// in CI; to run the same gate manually on a Windows host:
///
/// ```text
/// just test-windows-captured-stdout
/// ```
#[test]
#[ignore = "requires a Windows host; self-attaches a console and proves the precondition, but only runs under --ignored on Windows (cross-compile-checked on the macOS dev host)"]
fn captured_stdout_receives_only_value_no_tui_bytes() {
    // Establish and PROVE the attached-console precondition BEFORE spawning the
    // child. A bail here (panic) is intentional: it means this run cannot verify
    // the F2 contract, and that must be a visible failure, not a silent pass.
    let alloc_summary = establish_console();
    assert_console_precondition(&alloc_summary);

    // Spawn `question` with stdout captured to a pipe (the `FOO=$(question ...)`
    // shape) while stderr/stdin stay attached to the console established above, so
    // the console renders the prompt and our injected key event reaches it.
    let mut child = Command::new(biscuit_test_harness::bin_exe!("question"))
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
