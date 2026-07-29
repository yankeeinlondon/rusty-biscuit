//! Level 2 real-terminal capture test for the shell-approval prompt under
//! `--dry-run`.
//!
//! Closes the spec acceptance criterion:
//!
//! > Interactive approval prompts for unapproved shell commands appear
//! > exactly as in normal mode.
//!
//! The sibling `level2_dry_run_pty.rs` proves the same parity by injecting
//! bytes into a raw pseudo-terminal (`/dev/ptmx`) and reading the child's
//! output back. That establishes the *handler* fires identically in both
//! modes, but it never routes the prompt through a real terminal emulator's
//! render-and-capture path. This file does: it drives
//! `claudine compose [--dry-run]` inside a real terminal (`tmux`), captures
//! the *pane* the emulator actually displayed, and compares the rendered
//! approval-prompt surface between normal mode and `--dry-run` mode.
//!
//! Why this is the L2 complement: the value here is the emulator's own
//! re-rendering. Shell approval runs in the prepare phase *before* the
//! dry-run seam and is served by the same `CliShellApprovalHandler` in both
//! modes (`dry_run` only rewrites the non-interactive *no-handler* branch), so
//! the captured surface must be identical. A `FORCE_COLOR=1` run routes the
//! prompt through an optimistic terminal so the styling is emitted, and the
//! proof is then the emulator's capture (`frame.raw` / `frame.plain`).
//!
//! Skip-clean: checks `TmuxHarness::available()` first and returns early when
//! the backend is absent (no `#[ignore]`). `BISCUIT_TEST_LEVEL_REQUIRED=2`
//! flips a missing harness into a hard failure.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, clear_no_color, write_executable};

/// A document whose body holds one unapproved `::shell` command, so composing
/// it surfaces the approval prompt. The `echo` marker also lets the
/// `appears_and_allows` PTY sibling assert the command ran; here we always
/// deny, so it is only the prompt surface that matters.
const FIXTURE_DOC: &str = "\
---
title: dry-run approval
---
::shell echo tty-approved-marker
";

/// A staged workspace: a minimal config (so the first-run wizard does not
/// intercept), a `goose` provider stub (present for resolution, never launched
/// under `--dry-run`), and the approval fixture document.
struct Staged {
    workspace: TestWorkspace,
    bin_dir: std::path::PathBuf,
    doc: std::path::PathBuf,
}

fn stage() -> Staged {
    let workspace = TestWorkspace::named("claudine-dryrun-approval-l2");
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, FIXTURE_DOC).unwrap();

    Staged {
        workspace,
        bin_dir,
        doc,
    }
}

/// Poll the pane until `marker` (after escape stripping) appears, returning the
/// frame that first contains it. Panics on timeout.
///
/// The handler renders the whole prompt with a single forward write, so once
/// the last static option (`Blacklist and stop`) is visible the surface is
/// complete and safe to snapshot.
fn wait_for_pane_marker<H: TerminalHarness>(
    harness: &mut H,
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

/// Extract the approval-prompt block — the lines from the
/// `Shell Approval Required` header through the final `Blacklist and stop`
/// option — from a captured frame.
///
/// Returns the plain (escape-stripped) lines and the index-aligned raw
/// (escape-bearing) lines. `CapturedFrame` strips `raw` into `plain`
/// line-for-line, so the same `start..=end` indices select both. Starting at
/// the header excludes the echoed command line — which differs by `--dry-run`
/// between the two runs — so only the handler-rendered surface is compared.
fn prompt_region(frame: &CapturedFrame) -> (Vec<String>, Vec<String>) {
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let start = plain_lines
        .iter()
        .position(|l| l.contains("Shell Approval Required"));
    let end = plain_lines
        .iter()
        .position(|l| l.contains("Blacklist and stop"));
    match (start, end) {
        (Some(s), Some(e)) if e >= s => {
            let plain = plain_lines[s..=e]
                .iter()
                .map(|l| l.trim_end().to_string())
                .collect();
            let raw = raw_lines
                .get(s..=e)
                .map(|sl| sl.iter().map(|l| l.to_string()).collect())
                .unwrap_or_default();
            (plain, raw)
        }
        _ => (Vec::new(), Vec::new()),
    }
}

/// Run `claudine compose [--dry-run] --goose <doc>` in the pane, wait for the
/// approval prompt to render, snapshot it, then deny (option 4) so the run
/// aborts before any provider launch and the shell prompt returns. Returns the
/// captured frame.
///
/// `clear` runs first so the shared pane shows only this run — otherwise a
/// prior run's prompt (still on screen) would be the one `prompt_region`
/// extracts.
fn capture_prompt_in_mode(harness: &mut TmuxHarness, staged: &Staged, dry_run: bool) -> CapturedFrame {
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(harness);

    let home = staged.workspace.path().to_string_lossy().into_owned();
    let path = augmented_path(&staged.bin_dir);
    let path = path.to_string_lossy().into_owned();
    let claudine = cargo_bin("claudine").display().to_string();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let mut cmd = format!("{claudine} compose --goose");
    if dry_run {
        cmd.push_str(" --dry-run");
    }
    cmd.push(' ');
    cmd.push_str(&staged.doc.display().to_string());

    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
                // Deterministic width so both runs render the prompt identically.
                ("COLUMNS", "80"),
            ],
        )
        .expect("send compose");

    let frame = wait_for_pane_marker(harness, "Blacklist and stop", Duration::from_secs(15));

    // Deny so the composition aborts before any provider launch and the shell
    // prompt returns, leaving the pane ready for the next run.
    harness.send_text(b"4\n").expect("send deny choice");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    frame
}

/// The approval prompt rendered through a real terminal is identical in normal
/// mode and `--dry-run` mode.
///
/// One shared fixture keeps the prompt's `Source:` line (the document path)
/// identical across both runs; denying persists nothing, so the second run
/// sees the same state as the first.
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_approval_prompt_matches_normal_mode_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage();

    let normal = capture_prompt_in_mode(&mut harness, &staged, false);
    let dry_run = capture_prompt_in_mode(&mut harness, &staged, true);

    let (normal_plain, _) = prompt_region(&normal);
    let (dry_plain, dry_raw) = prompt_region(&dry_run);

    assert!(
        normal_plain
            .iter()
            .any(|l| l.contains("Shell Approval Required"))
            && normal_plain.iter().any(|l| l.contains("tty-approved-marker"))
            && normal_plain.iter().any(|l| l.contains("Allow once")),
        "normal-mode capture did not contain the expected approval prompt; got:\n{normal_plain:#?}\nplain:\n{}",
        normal.plain,
    );

    assert_eq!(
        dry_plain, normal_plain,
        "the --dry-run approval prompt must render exactly as in normal mode \
         through a real terminal.\nnormal:\n{normal_plain:#?}\ndry-run:\n{dry_plain:#?}",
    );

    // The comparison would still pass on plain text alone; assert the emulator
    // actually displayed a *styled* prompt (the bold/yellow header carries SGR)
    // so this is a genuine real-terminal capture, not an L1 byte comparison.
    assert!(
        dry_raw.iter().any(|l| l.contains('\u{1b}')),
        "expected the captured approval prompt to carry styling (SGR escapes) \
         through the real terminal.\nraw region:\n{dry_raw:#?}",
    );
}
