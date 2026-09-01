//! Level 2 coverage for explicit operation-file misses.
//!
//! The PTY and non-PTY inline-compose cases invoke the same authored reference
//! and assert the same typed diagnostic content. The sequence case protects its
//! separate source-resolution seam. Every case must terminate without opening
//! the operation-file picker.

#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, claudine_bin, init_git_repo};

const EXPLICIT_INLINE_REFERENCE: &str = "./docs/unifi/access.md";
const INLINE_SUGGESTION: &str = "homelab/docs/unifi/access.md";
const EXPLICIT_SEQUENCE_REFERENCE: &str = "./docs/missing-sequence.md";
const SEQUENCE_SUGGESTION: &str = "flows/missing-sequence.md";
const DIAGNOSTIC_CODE: &str = "CompositionError: Unresolvable file reference";
const CONFIRMATION_PROMPT: &str = "Use this file? (Y/n)";
const CHOOSER_HINT: &str = "Enter=Submit";

fn stage_workspace(name: &str) -> TestWorkspace {
    let workspace = TestWorkspace::named(name);
    assert!(init_git_repo(workspace.path()));

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    write_fixture(
        workspace.path(),
        INLINE_SUGGESTION,
        "---\nprompt: Draft the body\n---\nOriginal body.\n",
    );
    write_fixture(
        workspace.path(),
        SEQUENCE_SUGGESTION,
        "---\nsequence:\n  - one\n---\nSequence body.\n",
    );

    workspace
}

fn write_fixture(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture has a parent")).unwrap();
    fs::write(path, content).unwrap();
}

fn command_args<'a>(subcommand: &'a str, reference: &'a str) -> [&'a str; 4] {
    [subcommand, "--goose", "--dry-run", reference]
}

fn run_without_pty(workspace: &TestWorkspace, subcommand: &str, reference: &str) -> Output {
    Command::new(claudine_bin())
        .current_dir(workspace.path())
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&workspace.path().join("bin")))
        .env("NO_COLOR", "1")
        .args(command_args(subcommand, reference))
        .output()
        .expect("run claudine without a PTY")
}

fn run_in_pty(
    harness: &mut TmuxHarness,
    workspace: &TestWorkspace,
    subcommand: &str,
    reference: &str,
) -> String {
    let done = workspace.path().join("command.done");
    let home = workspace.path().to_string_lossy();
    let path = augmented_path(&workspace.path().join("bin"));

    harness
        .send_command_with_env(
            &format!("cd '{}'", workspace.path().display()),
            &[("HOME", home.as_ref())],
        )
        .expect("cd into fixture repository");

    let args = command_args(subcommand, reference).join(" ");
    let command = format!("{} {args}; touch '{}'", claudine_bin(), done.display());
    harness
        .send_command_with_env(
            &command,
            &[
                ("HOME", home.as_ref()),
                ("PATH", path.to_str().unwrap_or("/usr/bin:/bin")),
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
            ],
        )
        .expect("send explicit operation-file miss");

    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let frame = harness.capture().expect("capture tmux pane");
        if done.exists() {
            return frame.plain;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let frame = harness.capture().expect("capture timed-out tmux pane");
    panic!(
        "explicit operation-file miss waited for input or failed to terminate; plain:\n{}",
        frame.plain
    );
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_explicit_no_match(body: &str, reference: &str, suggestion: &str, base_dir: &Path) {
    let canonical_base = fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
    let base_dir = canonical_base.to_string_lossy();
    let compact_body: String = body
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '┃')
        .collect();
    for expected in [DIAGNOSTIC_CODE, reference, suggestion, base_dir.as_ref()] {
        let compact_expected: String = expected.chars().filter(|ch| !ch.is_whitespace()).collect();
        assert!(
            compact_body.contains(&compact_expected),
            "explicit no-match report must contain {expected:?}; body:\n{body}"
        );
    }
    assert!(
        !body.contains("no autocomplete matches")
            && !body.contains("autocomplete not available")
            && !body.contains(CONFIRMATION_PROMPT)
            && !body.contains(CHOOSER_HINT),
        "explicit no-match must not enter autocomplete; body:\n{body}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_explicit_inline_compose_miss_matches_with_and_without_pty() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let workspace = stage_workspace("explicit-inline-operation-file-miss");
    let non_pty = run_without_pty(&workspace, "inline-compose", EXPLICIT_INLINE_REFERENCE);
    assert!(
        !non_pty.status.success(),
        "explicit non-PTY miss must fail"
    );
    let non_pty_body = combined_output(&non_pty);
    assert_explicit_no_match(
        &non_pty_body,
        EXPLICIT_INLINE_REFERENCE,
        INLINE_SUGGESTION,
        workspace.path(),
    );

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn owned tmux harness");
    let pty_body = run_in_pty(
        &mut harness,
        &workspace,
        "inline-compose",
        EXPLICIT_INLINE_REFERENCE,
    );
    assert_explicit_no_match(
        &pty_body,
        EXPLICIT_INLINE_REFERENCE,
        INLINE_SUGGESTION,
        workspace.path(),
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_explicit_sequence_source_miss_uses_typed_diagnostic_without_picker() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let workspace = stage_workspace("explicit-sequence-operation-file-miss");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn owned tmux harness");
    let body = run_in_pty(
        &mut harness,
        &workspace,
        "sequence",
        EXPLICIT_SEQUENCE_REFERENCE,
    );
    assert_explicit_no_match(
        &body,
        EXPLICIT_SEQUENCE_REFERENCE,
        SEQUENCE_SUGGESTION,
        workspace.path(),
    );
}
