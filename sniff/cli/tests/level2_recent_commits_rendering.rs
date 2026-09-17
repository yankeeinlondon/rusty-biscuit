//! Level 2 test for `sniff repo recent-commits` rendered in a real terminal.
//!
//! The library authors the report as Prose and the CLI renders it once with
//! word wrapping. Level 1 pins the Prose text and a piped render, but only an
//! emulator shows whether a long description wraps at word boundaries inside
//! the pane, whether the bold/blue markup reaches the grid as SGR, and how
//! file links degrade (OSC8, or the `[label](file://…)` fallback in tmux).
//!
//! The test owns a narrow tmux pane, because wrapping is a function of width.
//!
//! Skip-clean: when tmux is unavailable the test prints a skip notice and
//! passes. Gated behind `test-fixtures`, which pulls in `biscuit-test-harness`
//! and `common::capture_until`; the `test-l2` recipe enables it.
#![cfg(feature = "test-fixtures")]

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use std::path::Path;
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

mod common;

const RENDER_DEADLINE: Duration = Duration::from_secs(15);
const PANE_COLS: u32 = 60;
const PANE_ROWS: u32 = 50;
const MARKER: &str = "RECENT-COMMITS-DONE";

const DESCRIPTION: &str = "This deliberately long description paragraph must wrap \
across several rows of a sixty column pane without splitting any word in half.";

/// A repository with one verbose conventional commit whose body wraps.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("src/main.rs")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Test", "test@test.com").unwrap();
    let message = format!("feat(cli): render reports. {DESCRIPTION}\n\n- keep the first bullet");
    repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &[])
        .unwrap();
    dir
}

#[test]
fn level2_recent_commits_verbose_report_wraps_styles_and_links_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let repository = repository();
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux spawn_shell failed");
    harness.resize(PANE_COLS, PANE_ROWS).expect("tmux resize failed");

    // The environment prefix lives in the command string so it binds to
    // `sniff`, not to `clear`.
    let binary = cargo_bin("sniff").display().to_string();
    harness
        .send_command_with_env(
            &format!(
                "clear; FORCE_COLOR=1 '{binary}' --base '{}' repo recent-commits -v; printf '\\n%s\\n' {MARKER}",
                repository.path().display()
            ),
            &[],
        )
        .expect("send_command_with_env failed");

    let frame = common::capture_until(&mut harness, RENDER_DEADLINE, |frame| {
        frame.plain.lines().any(|line| line.trim() == MARKER)
            && frame.plain.contains("Files Impacted:")
    });
    let plain = &frame.plain;
    let report_start = plain
        .find("- [")
        .unwrap_or_else(|| panic!("no commit header in pane:\n{plain}"));
    let report = &plain[report_start..];

    // Every word of the description survives whole: a mid-word soft wrap by
    // the terminal would split one into two tokens.
    let report_words: Vec<&str> = report.split_whitespace().collect();
    let expected: Vec<&str> = DESCRIPTION.split_whitespace().collect();
    assert!(
        report_words
            .windows(expected.len())
            .any(|window| window == expected.as_slice()),
        "description must wrap at word boundaries:\n{plain}"
    );
    let description_rows = report
        .lines()
        .filter(|line| {
            expected
                .iter()
                .any(|word| line.split_whitespace().any(|token| token == *word))
        })
        .count();
    assert!(description_rows > 1, "fixture must actually wrap:\n{plain}");

    for evidence in ["] feat(cli) at ", ": render reports", "Details:", "- keep the first bullet"] {
        assert!(report.contains(evidence), "expected {evidence:?} in pane:\n{plain}");
    }
    for leaked in ["<bold>", "<blue>", "<italic>", "\\[", "\\_", "**"] {
        assert!(!report.contains(leaked), "{leaked:?} leaked into pane:\n{plain}");
    }

    // Bold hash and blue operation reach the grid as SGR.
    assert!(
        frame.raw.contains("[1m") || frame.raw.contains(";1m"),
        "expected a bold SGR in raw capture:\n{}",
        frame.raw
    );
    assert!(
        frame.raw.contains("[34m") || frame.raw.contains(";34m"),
        "expected a blue SGR in raw capture:\n{}",
        frame.raw
    );

    // File links: OSC8 on capable terminals, the Markdown fallback in tmux.
    assert!(
        frame.raw.contains("]8;;") || report.contains("](file"),
        "expected a file link or its fallback:\nplain:\n{plain}\nraw:\n{}",
        frame.raw
    );
}
