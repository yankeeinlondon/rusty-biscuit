//! L2 real-terminal *emulator* capture for the removed validation/handler key
//! diagnostic.
//!
//! Complements `compose_removed_validation_keys.rs` (L1, non-TTY/user-boundary
//! diagnostic) by driving the real `claudine` binary inside real terminal
//! emulators (`tmux` and `WezTerm`) and asserting on the bytes the emulator
//! actually re-rendered. This is the "Level 2 real-terminal capture" the High
//! finding in `claudine/features/2026-06-21-remove-validations/review-2.md`
//! calls for: it proves the styled `removed validation/handler key` diagnostic
//! and the TTY-gated frontmatter excerpt (a line-numbered YAML `CodeBlock`
//! highlighting the offending key) render correctly through an emulator, not
//! just that the non-TTY diagnostic string was assembled.
//!
//! - `tmux` (portable, headless): the visible-text and SGR contract — the
//!   diagnostic header, the offending `pre_checks` key, the `---` delimiters
//!   and the highlighted YAML line all reach the pane, and SGR escapes prove a
//!   genuine styled real-terminal capture.
//! - `WezTerm`: the authored-frontmatter YAML `CodeBlock` re-renders with
//!   syntax styling through a real emulator (the `---` delimiter and
//!   `pre_checks:` key line are visible, and `frame.raw` carries SGR escapes).
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
use common::{
    TestWorkspace, assert_row_is_styled, augmented_path, clear_no_color, write_executable,
};

/// Removed-key fixture exercising the excerpt surface. `pre_checks` is a
/// retired validation/handler key, so `compose` fails fast with the
/// `RemovedValidationKey` diagnostic during prepare — before the provider is
/// ever launched.
const REMOVED_KEY_FIXTURE: &str = "---\npre_checks:\n  - command: test\n---\nBody\n";

/// A staged workspace with a minimal config (so the first-run wizard never
/// intercepts), a `goose` provider stub on `PATH` (so resolution succeeds up to
/// the point the removed-key scan fires — matching the L1
/// `compose_removed_validation_keys.rs` driver), and the fixture document.
struct Staged {
    workspace: TestWorkspace,
    doc: std::path::PathBuf,
    /// Augmented `PATH` string pointing at the stub `bin` dir, kept alive for
    /// the lifetime of the staging so the harness command can resolve `goose`.
    path: String,
    home: String,
}

fn stage() -> Staged {
    let workspace = TestWorkspace::named("claudine-removed-key-l2");
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Minimal config so the first-run setup wizard does not intercept.
    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    // Provider stub: present for resolution, never launched — the removed-key
    // scan fires during prepare, before the provider spawns.
    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, REMOVED_KEY_FIXTURE).unwrap();

    let path = augmented_path(&bin_dir)
        .to_string_lossy()
        .into_owned();
    let home = workspace.path().to_string_lossy().into_owned();

    Staged {
        workspace,
        doc,
        path,
        home,
    }
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

/// The removed-key diagnostic, rendered through a real terminal emulator, shows
/// the styled `removed validation/handler key` header, names the offending
/// `pre_checks` key, and appends the authored frontmatter as a line-numbered
/// YAML `CodeBlock` (delimiters included) with the offending line highlighted.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_removed_key_renders_styled_diagnostic_with_yaml() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);
    let staged = stage();
    let claudine = cargo_bin("claudine").display().to_string();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let cmd = format!("{claudine} compose --goose {}", staged.doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", staged.home.as_str()),
                ("PATH", staged.path.as_str()),
                ("FORCE_COLOR", "1"),
                // Deterministic width so the YAML block is not soft-wrapped.
                ("COLUMNS", "100"),
            ],
        )
        .expect("send compose --goose");

    // The YAML CodeBlock is appended last; its final content line proves the
    // full diagnostic + excerpt rendered.
    let frame = wait_for_pane_marker(&mut harness, "command: test", Duration::from_secs(15));

    // Visible-text contract (assert on `plain`): the diagnostic header names
    // the offending removed key and the lifecycle-stack replacement guidance.
    assert!(
        frame.plain.contains("removed validation/handler key"),
        "diagnostic must carry the `removed validation/handler key` header.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("pre_checks"),
        "diagnostic must name the offending removed key `pre_checks`.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("lifecycle stack"),
        "diagnostic must carry the lifecycle-stack replacement guidance.\nplain:\n{}",
        frame.plain,
    );

    // The frontmatter excerpt renders the authored YAML (delimiters included,
    // so block line N equals file line N) as a CodeBlock with line numbering,
    // highlighting the offending key's line.
    for fragment in ["---", "pre_checks:", "command: test"] {
        assert!(
            frame.plain.contains(fragment),
            "YAML block fragment `{fragment}` missing.\nplain:\n{}",
            frame.plain,
        );
    }

    // Styling contract, anchored on the offending YAML row. A bare
    // `frame.raw.contains(ESC)` is satisfied by any escape anywhere in the pane
    // — a colored shell prompt included — so it can hold on a run that rendered
    // the diagnostic with no styling at all.
    assert_row_is_styled(
        &frame.raw,
        "pre_checks:",
        "removed-validation-key diagnostic (tmux)",
    );
}

/// WezTerm re-renders the authored-frontmatter YAML `CodeBlock` with syntax
/// styling. The excerpt is gated on TTY output, but `FORCE_COLOR=1` forces the
/// render path even through the `2>&1 | sed` pipe that keeps the block within
/// WezTerm's short visible pane.
#[test]
#[serial(level2_terminal)]
fn level2_wezterm_removed_key_renders_yaml_codeblock() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);
    let staged = stage();
    let claudine = cargo_bin("claudine").display().to_string();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let cmd = format!(
        "{claudine} compose --goose {} 2>&1 | sed -n '1,80p'",
        staged.doc.display()
    );
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", staged.home.as_str()),
                ("PATH", staged.path.as_str()),
                ("FORCE_COLOR", "1"),
                ("COLUMNS", "100"),
            ],
        )
        .expect("send compose --goose");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(Duration::from_millis(400));

    let frame = harness.capture().expect("capture failed");

    // Visible-text contract: the authored frontmatter renders as a YAML
    // CodeBlock — its `---` delimiters and offending `pre_checks:` key line are
    // visible in the re-rendered pane. The diagnostic header (`removed
    // validation/handler key`) scrolls off WezTerm's short visible pane (the
    // capture has no scrollback), exactly like the inline-mismatch test's OSC 8
    // document link; the header's TTY/highlight fidelity is therefore asserted
    // by the tmux test, which captures the full pane.
    assert!(
        frame.plain.contains("---"),
        "YAML CodeBlock must show the frontmatter `---` delimiter.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("pre_checks:"),
        "YAML CodeBlock must show the authored offending key line `pre_checks:`.\nplain:\n{}",
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
