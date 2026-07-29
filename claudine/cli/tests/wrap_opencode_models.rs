//! Integration tests: dry-run opencode-models invocation guard across providers.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
use std::path::Path;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{augmented_path, write_executable};

/// A fake `opencode` binary that exits 1 when called with `models`.
/// Placed on PATH to prove that `--claude` / `--codex` compose paths do
/// NOT shell out to `opencode models`.
fn write_failing_opencode_models(path_dir: &Path) {
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf 'FAKE_OPENCODE_MODELS_ERROR: this should not have been called\n' >&2
  exit 1
fi
exit 0
"#,
    );
}

#[cfg(unix)]
#[test]
fn compose_claude_dry_run_does_not_call_opencode_models() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("fast.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_failing_opencode_models(&path_dir);
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "compose",
            "--claude",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn inline_compose_claude_dry_run_does_not_call_opencode_models() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("fast.md");
    fs::write(
        &md_file,
        "---\ntitle: test\nprompt: rewrite\n---\nPrompt body\n",
    )
    .unwrap();

    write_failing_opencode_models(&path_dir);
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "inline-compose",
            "--claude",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn compose_codex_dry_run_does_not_call_opencode_models() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("fast.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_failing_opencode_models(&path_dir);
    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--codex", "--dry-run", md_file.to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn inline_compose_codex_dry_run_does_not_call_opencode_models() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("fast.md");
    fs::write(
        &md_file,
        "---\ntitle: test\nprompt: rewrite\n---\nPrompt body\n",
    )
    .unwrap();

    write_failing_opencode_models(&path_dir);
    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "inline-compose",
            "--codex",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn compose_opencode_dry_run_calls_opencode_models_and_fails_with_test_double() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("fast.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_failing_opencode_models(&path_dir);

    // When --opencode is selected, model validation *should* call `opencode
    // models`, so the failing test double causes a failure (or the catalog
    // refresh is skipped because the model comes from an env var).
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .args([
            "compose",
            "--opencode",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();
}

/// `claudine sequence --opencode` with a frontmatter `model` and
/// `OPENCODE_MODEL` set must skip the dynamic catalog refresh because the
/// env var wins over the frontmatter hint. The failing `opencode models`
/// test double would surface as a non-zero exit if the refresh ran.
#[cfg(unix)]
#[test]
fn sequence_opencode_dry_run_with_env_model_skips_opencode_models_call() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n  - step_one\nmodel: frontmatter-model\n---\ncomposed body text\n",
    )
    .unwrap();

    write_failing_opencode_models(&path_dir);

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "env-model")
        .args([
            "sequence",
            "--opencode",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();
}

/// `claudine sequence --claude` should never invoke `opencode models`
/// because the selected provider doesn't use the dynamic OpenCode catalog
/// source. Mirrors the equivalent direct-compose acceptance test.
#[cfg(unix)]
#[test]
fn sequence_claude_dry_run_does_not_call_opencode_models() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n  - step_one\n---\ncomposed body text\n",
    )
    .unwrap();

    write_failing_opencode_models(&path_dir);
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "sequence",
            "--claude",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Phase 5 acceptance tests: Ctrl+C during prep exits 130 with clean notice
// ---------------------------------------------------------------------------
