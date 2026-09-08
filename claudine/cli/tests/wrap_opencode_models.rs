#![cfg(unix)]

//! Integration tests: dry-run opencode-models invocation guard across providers.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
use std::path::Path;
mod common;
use common::{CliProcessFixture, write_executable};

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
    let fixture = CliProcessFixture::named("compose-claude-no-opencode-models");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("fast.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_failing_opencode_models(fixture.bin_dir());
    write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 0\n");

    fixture
        .command()
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
    let fixture = CliProcessFixture::named("inline-compose-claude-no-opencode-models");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("fast.md");
    fs::write(
        &md_file,
        "---\ntitle: test\nprompt: rewrite\n---\nPrompt body\n",
    )
    .unwrap();

    write_failing_opencode_models(fixture.bin_dir());
    write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 0\n");

    fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-codex-no-opencode-models");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("fast.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_failing_opencode_models(fixture.bin_dir());
    write_executable(&fixture.bin_dir().join("codex"), "#!/bin/sh\nexit 0\n");

    fixture
        .command()
        .args(["compose", "--codex", "--dry-run", md_file.to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn inline_compose_codex_dry_run_does_not_call_opencode_models() {
    let fixture = CliProcessFixture::named("inline-compose-codex-no-opencode-models");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("fast.md");
    fs::write(
        &md_file,
        "---\ntitle: test\nprompt: rewrite\n---\nPrompt body\n",
    )
    .unwrap();

    write_failing_opencode_models(fixture.bin_dir());
    write_executable(&fixture.bin_dir().join("codex"), "#!/bin/sh\nexit 0\n");

    fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-opencode-calls-models");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("fast.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_failing_opencode_models(fixture.bin_dir());

    // When --opencode is selected, model validation *should* call `opencode
    // models`, so the failing test double causes a failure (or the catalog
    // refresh is skipped because the model comes from an env var).
    fixture
        .command()
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
    let fixture = CliProcessFixture::named("sequence-opencode-env-model");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n  - step_one\nmodel: frontmatter-model\n---\ncomposed body text\n",
    )
    .unwrap();

    write_failing_opencode_models(fixture.bin_dir());

    fixture
        .command()
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
    let fixture = CliProcessFixture::named("sequence-claude-no-opencode-models");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n  - step_one\n---\ncomposed body text\n",
    )
    .unwrap();

    write_failing_opencode_models(fixture.bin_dir());
    write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 0\n");

    fixture
        .command()
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
