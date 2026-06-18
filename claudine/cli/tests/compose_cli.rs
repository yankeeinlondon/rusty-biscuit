//! Integration tests for `claudine compose`.
//!
//! Phase 1 of the `2026-06-03-always-harness` feature. Pins current observable
//! behavior so divergence introduced by later phases fails loudly.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, write_executable};

// ============================================================================
// Phase 1: convergence between non-harness and harness-enabled direct compose
// ============================================================================

/// Build a fake `opencode` binary that emits a short structured stream and
/// exits successfully.
fn stage_opencode_body_writer(path_dir: &std::path::Path) {
    write_executable(
        &path_dir.join("opencode"),
        r##"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"conv","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"conv"}'
printf '%s\n' '{"type":"text","text":"# Generated Plan\n\nThis is the generated body."}'
printf '%s\n' '{"type":"finish","sessionID":"conv"}'
exit 0
"##,
    );
}

#[cfg(unix)]
#[test]
fn compose_direct_non_harness_and_harness_produce_identical_stdout_body() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    stage_opencode_body_writer(&path_dir);

    let bare_file = workspace.path().join("bare.md");
    fs::write(
        &bare_file,
        "---\n---\nCompose this document.\n",
    )
    .unwrap();

    let harness_file = workspace.path().join("harness.md");
    fs::write(
        &harness_file,
        "---\npost_checks: []\n---\nCompose this document.\n",
    )
    .unwrap();

    let run = |path: &std::path::Path| {
        cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", augmented_path(&path_dir))
            .env("OPENCODE_MODEL", "test-model")
            .current_dir(workspace.path())
            .args(["compose", "--opencode", path.to_str().unwrap()])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };

    let bare_bytes = run(&bare_file);
    let harness_bytes = run(&harness_file);
    let bare_stdout = String::from_utf8_lossy(&bare_bytes);
    let harness_stdout = String::from_utf8_lossy(&harness_bytes);
    let expected = "# Generated Plan\n\nThis is the generated body.\n";
    assert_eq!(
        bare_stdout, expected,
        "direct compose stdout body mismatch; got: {bare_stdout:?}"
    );
    assert_eq!(
        harness_stdout, expected,
        "harness-enabled direct compose stdout body mismatch; got: {harness_stdout:?}"
    );
    assert_eq!(
        bare_stdout, harness_stdout,
        "direct compose stdout must be identical with and without harness frontmatter"
    );
}
