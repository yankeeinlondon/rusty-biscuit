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
//! - `WezTerm`: the authored-frontmatter YAML CodeBlock re-renders with syntax
//!   styling through a real emulator. (The OSC 8 `file://` document link lives
//!   in the diagnostic, which the tall YAML block scrolls past WezTerm's short
//!   visible pane — and the capture has no scrollback — so OSC 8 + cyan-SGR
//!   fidelity for this error is asserted by the L1 PTY test, which reads the
//!   full raw transcript.)
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
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, assert_row_is_styled, clear_no_color};

/// Mismatch fixture exercising the YAML fidelity surface (comment, anchor +
/// alias, block scalar, non-canonical order). Both `prompt` and `sequence` are
/// non-null, so this is an inline-compose / sequence mismatch.
const MISMATCH_FIXTURE: &str = "---\n# leading comment\nsequence: &seq\n  - name: Hello\n  - name: Goodbye\nprompt: |-\n  multi\n  line\nalias: *seq\n---\nbody\n";

/// A compact mismatch fixture whose authored frontmatter renders to a short
/// YAML block. Used by the WezTerm test so the full diagnostic — including the
/// OSC 8 document link at the top — stays within the visible pane (the capture
/// has no scrollback, and the tall fidelity fixture would scroll the diagnostic
/// off the WezTerm window).
const MINIMAL_FIXTURE: &str = "---\nprompt: Do something\nsequence: []\n---\nbody\n";

/// A staged workspace with a minimal config (so the first-run wizard never
/// intercepts) and the mismatch fixture document.
struct Staged {
    workspace: TestWorkspace,
    doc: std::path::PathBuf,
}

fn stage(fixture: &str) -> Staged {
    let workspace = TestWorkspace::named("claudine-inline-mismatch-l2");
    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, fixture).unwrap();

    Staged { workspace, doc }
}

/// Poll the pane until `marker` (after escape stripping) appears, returning the
/// frame that first contains it. Panics on timeout.
fn wait_for_pane_marker(harness: &mut TmuxHarness, marker: &str, deadline: Duration) -> CapturedFrame {
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
/// the styled diagnostic, the linked document, and the authored YAML rendered
/// as a CodeBlock.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_mismatch_renders_styled_diagnostic_with_yaml() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);
    let staged = stage(MISMATCH_FIXTURE);
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

    // The YAML CodeBlock is appended last, so its final line proves the full
    // diagnostic rendered.
    let frame = wait_for_pane_marker(&mut harness, "alias: *seq", Duration::from_secs(15));

    // Visible-text contract (assert on `plain`): the diagnostic directs the
    // user to `claudine sequence`, identifies the document, and shows the
    // authored YAML as a CodeBlock.
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
    for fragment in ["# leading comment", "sequence: &seq", "prompt: |-", "alias: *seq"] {
        assert!(
            frame.plain.contains(fragment),
            "YAML block fragment `{fragment}` missing.\nplain:\n{}",
            frame.plain,
        );
    }

    // Styling contract, anchored on the diagnostic's own row. A bare
    // `frame.raw.contains(ESC)` is satisfied by any escape anywhere in the pane
    // — a colored shell prompt included — so it can hold on a run that rendered
    // the diagnostic with no styling at all.
    assert_row_is_styled(
        &frame.raw,
        "claudine sequence",
        "inline-mismatch diagnostic (tmux)",
    );
}

/// WezTerm re-renders the authored-frontmatter YAML CodeBlock with syntax
/// styling. The OSC 8 document link in the diagnostic scrolls past WezTerm's
/// short visible pane (no scrollback capture), so its fidelity is asserted by
/// the L1 PTY test instead.
#[test]
#[serial(level2_terminal)]
fn level2_wezterm_mismatch_renders_yaml_codeblock() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);
    let staged = stage(MINIMAL_FIXTURE);
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let claudine = cargo_bin!("claudine").display().to_string();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let cmd = format!(
        "{claudine} inline-compose {} 2>&1 | sed -n '1,80p'",
        staged.doc.display()
    );
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

    // Visible-text contract: the authored frontmatter renders as a YAML
    // CodeBlock — its keys are visible in the re-rendered pane.
    assert!(
        frame.plain.contains("prompt: Do something"),
        "YAML CodeBlock must show the authored `prompt`.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("sequence:"),
        "YAML CodeBlock must show the authored `sequence`.\nplain:\n{}",
        frame.plain,
    );

    // Styling contract: WezTerm re-rendered a *styled* surface — the CodeBlock's
    // syntax highlighting carries SGR escapes, proving a genuine real-terminal
    // capture rather than a plain string.
    assert!(
        frame.raw.contains('\u{1b}'),
        "expected the YAML CodeBlock to carry styling (escape sequences) \
         through WezTerm.\nraw:\n{}",
        frame.raw,
    );
}
