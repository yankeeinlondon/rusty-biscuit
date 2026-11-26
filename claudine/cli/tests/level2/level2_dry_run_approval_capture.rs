//! Level 2: `compose --dry-run` in a real terminal asks no shell approval.
//!
//! `--dry-run` on `compose` is a passive check: it runs no command, so an
//! unapproved `::shell` directive is reported as `not checked` and left
//! unevaluated in the preview instead of raising the approval prompt. The L1
//! tests in `dry_run_check.rs` prove that with piped stdio, where no prompt
//! could appear anyway. This file proves it where the prompt *would* appear:
//! in a real terminal (`tmux`), against a document that does raise the prompt
//! in a real run.
//!
//! - A real run of the document shows `Shell Approval Required` (the control
//!   that keeps the dry-run assertion from passing vacuously); the test
//!   denies it so nothing runs.
//! - A dry run of the same document in the same kind of pane exits 0 without
//!   any prompt, prints its report and the marked preview, and the command
//!   never runs.
//!
//! Skip-clean: checks `TmuxHarness::available()` first and returns early when
//! the backend is absent (no `#[ignore]`). `BISCUIT_TEST_LEVEL_REQUIRED=2`
//! flips a missing harness into a hard failure. Run via
//! `just test-l2 dry_run_approval`.

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

use crate::common;
use common::{TestWorkspace, augmented_path, claudine_bin, sh_quote, write_executable};

/// A document whose body holds one unapproved `::shell` command. Running the
/// command would create `shell-ran`, so its absence proves it never ran.
const FIXTURE_DOC: &str = "\
---
title: dry-run approval
---
::shell touch shell-ran
";

const APPROVAL_HEADER: &str = "Shell Approval Required";

/// A staged workspace: a minimal config (so the first-run wizard does not
/// intercept), a `goose` provider stub (looked up, never launched here), and
/// the fixture document.
struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    doc: PathBuf,
}

fn stage() -> Staged {
    let workspace = TestWorkspace::named("claudine-dryrun-approval-l2");
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.yaml"), "{}").unwrap();

    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, FIXTURE_DOC).unwrap();

    Staged {
        workspace,
        bin_dir,
        doc,
    }
}

/// Writes a launcher that clears the pane, runs `claudine compose --goose
/// [--dry-run] doc.md` from the workspace, and prints `{marker}:0` or
/// `{marker}:1`. The marker is printed by the script, never typed, so the
/// echoed command line cannot satisfy a wait on it.
fn write_launcher(staged: &Staged, dry_run: bool, marker: &str) -> PathBuf {
    let path = augmented_path(&staged.bin_dir);
    let quoted_path = sh_quote(&path.to_string_lossy());
    let flag = if dry_run { " --dry-run" } else { "" };
    let script = format!(
        "#!/bin/sh\nprintf '\\033[3J\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
         if HOME={home} PATH={quoted_path} COLUMNS=80 {claudine} compose --goose{flag} {doc}; then\n  \
         printf '\\n{marker}:0\\n'\nelse\n  printf '\\n{marker}:1\\n'\nfi\n",
        cwd = sh_quote(&staged.workspace.path().display().to_string()),
        home = sh_quote(&staged.workspace.path().display().to_string()),
        claudine = sh_quote(claudine_bin()),
        doc = sh_quote(&staged.doc.display().to_string()),
    );
    let launcher = staged
        .workspace
        .path()
        .join(if dry_run { "dry-run.sh" } else { "real-run.sh" });
    write_executable(&launcher, &script);
    launcher
}

/// Polls the pane until `done` returns true for a frame, panicking with the
/// last frame after `deadline`.
fn wait_for_frame(
    harness: &mut TmuxHarness,
    deadline: Duration,
    what: &str,
    mut done: impl FnMut(&CapturedFrame) -> bool,
) -> CapturedFrame {
    let stop = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if done(&frame) {
            return frame;
        }
        assert!(Instant::now() < stop, "{what} did not appear within {deadline:?}.\nplain:\n{}", frame.plain);
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn exit_status(frame: &CapturedFrame, marker: &str) -> Option<String> {
    let prefix = format!("{marker}:");
    frame
        .plain
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(|status| status.trim().to_string()))
}

/// The same document raises the approval prompt in a real run and none under
/// `--dry-run`, where the report and the marked preview render and the
/// command never runs.
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_asks_no_shell_approval_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage();
    let shell_ran = staged.workspace.path().join("shell-ran");

    // Control: a real run in a terminal asks. Deny so nothing runs.
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("real-run pane");
    let marker = format!("DRYRUN_APPROVAL_REAL_{}", std::process::id());
    let launcher = write_launcher(&staged, false, &marker);
    harness
        .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
        .expect("send real run");
    let prompt = wait_for_frame(&mut harness, Duration::from_secs(15), "the approval prompt", |frame| {
        frame.plain.contains("Deny for this run")
    });
    assert!(
        prompt.plain.contains(APPROVAL_HEADER) && prompt.plain.contains("touch shell-ran"),
        "a real run must ask before running the command:\n{}",
        prompt.plain
    );
    harness.send_text(b"4\n").expect("deny");
    wait_for_frame(&mut harness, Duration::from_secs(15), "the real run's exit", |frame| {
        exit_status(frame, &marker).is_some()
    });
    drop(harness);
    assert!(!shell_ran.exists(), "a denied command must not run");

    // The dry run, in a fresh pane so the control's prompt is not on screen.
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("dry-run pane");
    let marker = format!("DRYRUN_APPROVAL_DRY_{}", std::process::id());
    let launcher = write_launcher(&staged, true, &marker);
    harness
        .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
        .expect("send dry run");
    let frame = wait_for_frame(&mut harness, Duration::from_secs(15), "the dry run's exit", |frame| {
        // A prompt would block the run until the deadline; fail at once instead.
        assert!(
            !frame.plain.contains(APPROVAL_HEADER),
            "a dry run must not ask for shell approval:\n{}",
            frame.plain
        );
        exit_status(frame, &marker).is_some()
    });

    let plain = &frame.plain;
    assert_eq!(exit_status(&frame, &marker).as_deref(), Some("0"), "the dry run must exit 0:\n{plain}");
    assert!(plain.contains("Dry run of"), "the report header must render:\n{plain}");
    assert!(plain.contains("not checked"), "the command must be reported as not checked:\n{plain}");
    assert!(
        plain.contains("[not evaluated in a dry run]"),
        "the preview must mark the unevaluated directive:\n{plain}"
    );
    assert!(!shell_ran.exists(), "a dry run must not run the command:\n{plain}");
}
