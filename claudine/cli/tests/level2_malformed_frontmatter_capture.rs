//! Level 2 real-terminal capture for malformed `----` frontmatter fences.
//!
//! Drives the three composition entry points through a real tmux pane and
//! asserts the user-visible diagnostic, including the line-1 fence highlight,
//! reaches the rendered terminal surface.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
use common::{
    TestWorkspace, assert_row_is_styled, augmented_path, clear_no_color, write_executable,
};

const MALFORMED_DOC: &str = "\
----
agent: goose
prompt: Generate a replacement body
sequence:
  - draft
----
Original body.
";

struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    doc: PathBuf,
    launch_count: PathBuf,
}

fn stage(name: &str) -> Staged {
    let workspace = TestWorkspace::named(name);
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    let launch_count = workspace.path().join("provider-launched.txt");
    write_provider_stub(&bin_dir);

    let doc = workspace.path().join("malformed.md");
    fs::write(&doc, MALFORMED_DOC).unwrap();

    Staged {
        workspace,
        bin_dir,
        doc,
        launch_count,
    }
}

fn write_provider_stub(bin_dir: &Path) {
    let script = "#!/bin/sh\nprintf launched >> \"$CLAUDINE_PROVIDER_LAUNCH_COUNT\"\nexit 0\n";
    write_executable(&bin_dir.join("goose"), script);
}

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

fn capture_command(harness: &mut TmuxHarness, subcommand: &str, staged: &Staged) -> CapturedFrame {
    // This fixture runs under `FORCE_COLOR=1`, which an ambient `NO_COLOR` would
    // out-vote — see `common::clear_no_color`.
    clear_no_color(harness);

    let claudine = cargo_bin!("claudine").display().to_string();
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let path = augmented_path(&staged.bin_dir);
    let path = path.to_string_lossy().into_owned();
    let launch_count = staged.launch_count.to_string_lossy().into_owned();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let cmd = format!("{claudine} {subcommand} --goose {}", staged.doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
                ("COLUMNS", "100"),
                ("CLAUDINE_PROVIDER_LAUNCH_COUNT", launch_count.as_str()),
            ],
        )
        .expect("send claudine command");

    let frame = wait_for_pane_marker(
        harness,
        "Use exactly three dashes",
        Duration::from_secs(15),
    );
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    frame
}

fn assert_malformed_fence_diagnostic(frame: &CapturedFrame, subcommand: &str) {
    for needle in [
        "frontmatter fence mismatch",
        "Fence: ---- on line 1",
        "Frontmatter fence mismatch here:",
        "> 1 │ ----",
        "2 │ agent: goose",
        "Use exactly three dashes (---)",
        "yaml",
        "prompt: Generate a replacement body",
        "sequence:",
    ] {
        assert!(
            frame.plain.contains(needle),
            "{subcommand} diagnostic missing {needle:?}.\nplain:\n{}",
            frame.plain
        );
    }

    assert!(
        !frame.plain.contains("Agent Prompt"),
        "{subcommand} must not render raw YAML as an Agent Prompt.\nplain:\n{}",
        frame.plain
    );
    assert_row_is_styled(
        &frame.raw,
        "frontmatter fence mismatch",
        &format!("{subcommand} diagnostic"),
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_malformed_frontmatter_renders_highlighted_diagnostic_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");

    for subcommand in ["compose", "inline-compose", "sequence"] {
        let staged = stage(&format!("claudine-malformed-frontmatter-{subcommand}-l2"));
        let frame = capture_command(&mut harness, subcommand, &staged);
        assert_malformed_fence_diagnostic(&frame, subcommand);
        assert!(
            !staged.launch_count.exists(),
            "{subcommand} should fail before provider launch"
        );
    }
}
