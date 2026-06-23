//! Integration tests for `claudine inline-compose`.
//!
//! Phase 1 of the `2026-06-03-always-harness` feature. Pins current observable
//! behavior so divergence introduced by later phases fails loudly.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, write_executable};

// ============================================================================
// Phase 1: convergence between non-harness and harness-enabled inline compose
// ============================================================================

/// Build a fake `opencode` binary that emits interstitial narration, a tool
/// call, and then final body content after the last tool call. The final body
/// must be used for inline write-back, not the interstitial narration.
fn stage_opencode_inline_body_writer(path_dir: &std::path::Path) {
    write_executable(
        &path_dir.join("opencode"),
        r##"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"conv","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"conv"}'
printf '%s\n' '{"type":"text","text":"Let me look up the answer."}'
printf '%s\n' '{"type":"tool_start","part":{"id":"t1","tool":"bash"}}'
printf '%s\n' '{"type":"text","text":"# Final Body\n\nThis is the replacement body."}'
printf '%s\n' '{"type":"finish","sessionID":"conv"}'
exit 0
"##,
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_writes_expected_final_body() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    stage_opencode_inline_body_writer(&path_dir);

    let source = workspace.path().join("bare.md");
    fs::write(
        &source,
        "---\nprompt: Generate the body\n---\nOriginal body.\n",
    )
    .unwrap();

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args(["inline-compose", "--opencode", source.to_str().unwrap()])
        .assert()
        .success();

    let doc = fs::read_to_string(&source).unwrap();
    // The inline closure must use the final response (text after the last
    // tool call), not the interstitial narration.
    assert!(
        doc.contains("# Final Body"),
        "inline body must contain final response heading; doc:\n{doc}"
    );
    assert!(
        doc.contains("This is the replacement body."),
        "inline body must contain final response body; doc:\n{doc}"
    );
    assert!(
        !doc.contains("Let me look up the answer."),
        "inline body must not contain interstitial narration; doc:\n{doc}"
    );
}
