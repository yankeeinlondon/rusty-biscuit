#![cfg(unix)]

//! Integration tests for `claudine inline-compose`.
//!
//! Phase 1 of the `2026-06-03-always-harness` feature. Pins current observable
//! behavior so divergence introduced by later phases fails loudly.

use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, init_git_repo, write, write_executable};

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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
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

#[cfg(unix)]
#[test]
fn inline_compose_uses_source_doc_repository_not_launch_cwd() {
    let workspace = tempdir().unwrap();
    let source_root = workspace.path().join("source");
    let launch_root = workspace.path().join("launch");
    fs::create_dir_all(&source_root).unwrap();
    fs::create_dir_all(&launch_root).unwrap();
    assert!(init_git_repo(&source_root));
    assert!(init_git_repo(&launch_root));

    write(
        &source_root.join("snippet.md"),
        "SOURCE_REPOSITORY_INLINE_MARKER\n",
    );
    write(
        &launch_root.join("snippet.md"),
        "LAUNCH_REPOSITORY_INLINE_MARKER\n",
    );
    let source = source_root.join("prompts/inline.md");
    write(
        &source,
        "---\nprompt: |\n  ::file snippet.md\n---\nOriginal body.\n",
    );

    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(&launch_root)
        .args([
            "inline-compose",
            "--goose",
            "--dry-run",
            source.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("SOURCE_REPOSITORY_INLINE_MARKER"),
        "inline prompt transclusion must use the source repository; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("LAUNCH_REPOSITORY_INLINE_MARKER"),
        "launch repository must not leak into inline prompt transclusion; stdout:\n{stdout}"
    );
}
