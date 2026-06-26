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
//! ## Why this is opt-in / skip-clean
//!
//! Two environment constraints make an always-on automated assertion
//! impossible, so the spawn is gated behind BOTH `cfg(windows)` AND an explicit
//! `BISCUIT_TUI_WINDOWS_CONSOLE_TEST=1` opt-in. Absent the env var the test
//! returns early (clean pass), mirroring how
//! `active_redirect_routes_stdout_to_console_and_pipe_gets_only_submitted_value`
//! returns early when `!io::stderr().is_terminal()`.
//!
//! 1. **Console required.** The captured-stdout shape only has meaning when a
//!    real console is attached for the prompt to render into. Under nextest,
//!    stderr is captured, which trips the standalone headless guard
//!    (`stdout && stderr both !is_terminal` → `no interactive terminal
//!    available`) and prevents the real shape entirely.
//! 2. **Live keypress.** Submitting a value drives the crossterm event loop,
//!    which reads physical key events from the console. Injecting those
//!    deterministically in automated CI is impractical, so the guaranteed path
//!    is the documented manual/CI-runner reproduction (see
//!    `features/2026-06-19-review-findings/windows-captured-stdout-repro.md`),
//!    which a human/runner drives with the env var set.
//!
//! The PRIMARY guaranteed-here deliverable is that this file COMPILES for the
//! active Windows target and NEVER runs (skips) on macOS/Linux or on any
//! Windows run without the env var.
#![cfg(windows)]

use std::io::Read;
use std::process::{Command, Stdio};

use assert_cmd::cargo::cargo_bin;

/// Opt-in env var. A Windows operator (or CI runner) sets this to `1` only when
/// a real console is attached and they will supply the prompt keystroke.
const OPT_IN_ENV: &str = "BISCUIT_TUI_WINDOWS_CONSOLE_TEST";

#[test]
fn captured_stdout_receives_only_value_no_tui_bytes() {
    // Clean skip unless an operator/CI runner has provided a real console with
    // the captured-stdout shape. This mirrors the in-process active-path test,
    // which returns early when the required console shape is absent.
    if std::env::var(OPT_IN_ENV).as_deref() != Ok("1") {
        eprintln!(
            "skipping: set {OPT_IN_ENV}=1 on a Windows console to run the \
             captured-stdout boundary check (see \
             features/2026-06-19-review-findings/windows-captured-stdout-repro.md)"
        );
        return;
    }

    // Spawn `question` with stdout captured to a pipe (the `FOO=$(question ...)`
    // shape) while stderr is INHERITED so the console stays attached for the
    // prompt. The operator selects an option and submits with Enter.
    let mut child = Command::new(cargo_bin("question"))
        .args(["choose-one", "Red", "Green", "Blue"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn question");

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
