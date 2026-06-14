//! L2 real-terminal *emulator* capture for the inline-compose / sequence
//! mismatch diagnostic.
//!
//! Complements `level1_inline_compose_mismatch_pty.rs` (raw PTY, authoritative
//! for the exact emitted SGR + OSC 8 bytes) by driving the real `claudine`
//! binary inside real terminal emulators (`tmux` and `WezTerm`) and asserting
//! on the surface the emulator actually re-rendered. This is the "Level 2
//! real-terminal capture" the second High finding in
//! `claudine/fixes/2026-06-06-inline-sequence/review-2.md` calls for: it proves
//! the styled diagnostic and the linked document render correctly through an
//! emulator, not just that the string was assembled.
//!
//! - `tmux` (portable, headless): the visible text and SGR contract.
//! - `WezTerm` (OSC8 fidelity): the resolved document is a real OSC8 `file://`
//!   hyperlink and the diagnostic carries diagnostic-specific SGR styling.
//!
//! Skip-clean via `Harness::available()`; `BISCUIT_TEST_LEVEL_REQUIRED=2`
//! turns a missing backend into a hard failure. Run via `just test-l2`.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
use common::TestWorkspace;

/// Mismatch fixture exercising the YAML fidelity surface (comment, anchor +
/// alias, block scalar, non-canonical order). Both `prompt` and `sequence` are
/// non-null, so this is an inline-compose / sequence mismatch.
const MISMATCH_FIXTURE: &str = "---\n# leading comment\nsequence: &seq\n  - name: Hello\n  - name: Goodbye\nprompt: |-\n  multi\n  line\nalias: *seq\n---\nbody\n";

/// A staged workspace with a minimal config (so the first-run wizard never
/// intercepts) and the mismatch fixture document.
struct Staged {
    workspace: TestWorkspace,
    doc: std::path::PathBuf,
}

fn stage() -> Staged {
    let workspace = TestWorkspace::named("claudine-inline-mismatch-l2");
    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, MISMATCH_FIXTURE).unwrap();

    Staged { workspace, doc }
}

/// Poll the pane until `marker` (after escape stripping) appears, returning the
/// frame that first contains it. Panics on timeout.
fn wait_for_pane_marker(
    harness: &mut TmuxHarness,
    marker: &str,
    deadline: Duration,
) -> CapturedFrame {
    let stop = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if frame.plain.contains(marker) {
            return frame;
        }
        if Instant::now() >= stop {
            panic!(
                "marker {marker:?} did not appear within {deadline:?}.\nplain:\n{}",
                frame.plain
            );
        }
        harness.settle();
    }
}

/// The mismatch diagnostic, rendered through a real terminal emulator, shows
/// the styled diagnostic, the linked document, and the verbatim authored YAML.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_mismatch_renders_styled_diagnostic_with_yaml() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage();
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let claudine = cargo_bin!("claudine").display().to_string();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let cmd = format!("{claudine} inline-compose {}", staged.doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("FORCE_COLOR", "1"),
                // Deterministic width so the YAML block is not soft-wrapped.
                ("COLUMNS", "100"),
            ],
        )
        .expect("send inline-compose");

    // The verbatim YAML is appended last, so the final fragment proves the full
    // diagnostic rendered.
    let frame = wait_for_pane_marker(&mut harness, "alias: *seq", Duration::from_secs(15));

    // Visible-text contract (assert on `plain`): the diagnostic directs the
    // user to `claudine sequence`, identifies the document, and reproduces the
    // authored YAML verbatim.
    assert!(
        frame.plain.contains("claudine sequence"),
        "diagnostic must direct the user to `claudine sequence`.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("doc.md"),
        "the linked document name must be visible.\nplain:\n{}",
        frame.plain,
    );
    for fragment in [
        "# leading comment",
        "sequence: &seq",
        "prompt: |-",
        "alias: *seq",
    ] {
        assert!(
            frame.plain.contains(fragment),
            "verbatim YAML fragment `{fragment}` missing.\nplain:\n{}",
            frame.plain,
        );
    }

    // Styling contract (assert on `raw`): the emulator re-rendered a *styled*
    // surface — the status-block border and inline tags carry SGR escapes. This
    // is what makes the test a genuine real-terminal capture rather than an L1
    // string comparison.
    assert!(
        frame.raw.contains('\u{1b}'),
        "expected the rendered diagnostic to carry styling (escape sequences) \
         through the real terminal.\nraw:\n{}",
        frame.raw,
    );
}

/// WezTerm preserves OSC 8 hyperlinks through its `get-text` capture, so it is
/// the backend that proves the resolved document is a real `file://` link (and,
/// for free, the same SGR contract).
#[test]
#[serial(level2_terminal)]
fn level2_wezterm_mismatch_renders_osc8_link_and_diagnostic_sgr() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    let staged = stage();
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let claudine = cargo_bin!("claudine").display().to_string();

    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let cmd = format!("{claudine} inline-compose {}", staged.doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("FORCE_COLOR", "1"),
                ("COLUMNS", "100"),
            ],
        )
        .expect("send inline-compose");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(Duration::from_millis(400));

    let frame = harness.capture().expect("capture failed");

    // Visible-text contract.
    assert!(
        frame.plain.contains("claudine sequence"),
        "diagnostic must direct the user to `claudine sequence`.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("doc.md"),
        "the linked document name must be visible.\nplain:\n{}",
        frame.plain,
    );

    // OSC 8 contract: the resolved document is rendered as a `file://` link.
    assert!(
        frame.raw.contains("\x1b]8;;file://"),
        "expected an OSC8 `file://` hyperlink for the resolved document.\nraw:\n{}",
        frame.raw,
    );

    // Diagnostic-specific SGR contract: the `<cyan>` inline tags in the
    // mismatch diagnostic emit `\x1b[36m` — not just any escape byte.
    assert!(
        frame.raw.contains("\x1b[36m"),
        "expected diagnostic-specific cyan styling (SGR 36).\nraw:\n{}",
        frame.raw,
    );
}
