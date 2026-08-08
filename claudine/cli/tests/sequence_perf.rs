#![cfg(unix)]

//! Integration tests for `claudine sequence --perf`.
//!
//! Validates that sequence performance reporting aggregates step-level
//! metrics into a single end-of-run report, including CLI overhead
//! propagation and partial-metrics handling on fail-fast.

use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

// ---------------------------------------------------------------------------
// Single aggregated report for a successful multi-step sequence
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn sequence_perf_renders_single_aggregated_report() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - alpha
  - beta
---
Step {{state}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    // Run from the isolated workspace (as
    // `sequence_perf_propagates_startup_timings` does): with the ambient
    // monorepo as cwd, each step's repository discovery pays a full git
    // worktree-metadata refresh that can exceed nextest's termination
    // ceiling under full-suite contention.
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // Exactly one Performance block
    let perf_count = plain.matches("Performance").count();
    assert_eq!(
        perf_count, 1,
        "expected exactly one Performance block, found {perf_count}; stderr: {plain}"
    );

    assert!(
        plain.contains("pre-dispatch"),
        "stderr should contain the pre-dispatch bucket; got: {plain}"
    );

    // TM-3: the sequence renders a `steps` Structural node with one per-step
    // subtree, not a single aggregate `agent execution` bucket. Execution lives
    // under each step, so the per-step `agent` breakdown — not a top-level
    // `agent execution` node — is what appears.
    assert!(
        plain.contains("steps"),
        "stderr should contain the steps bucket; got: {plain}"
    );
    assert!(
        plain.contains("step 1: alpha") && plain.contains("step 2: beta"),
        "stderr should contain per-step subtrees `step N: <name>`; got: {plain}"
    );
    assert!(
        !plain.contains("agent execution"),
        "sequence should not render a top-level aggregate `agent execution` node; got: {plain}"
    );

    // Sequence finished should appear before the perf block
    let seq_finished_pos = plain
        .find("Sequence finished")
        .expect("should contain 'Sequence finished'");
    let perf_pos = plain
        .find("Performance")
        .expect("should contain Performance block");
    assert!(
        seq_finished_pos < perf_pos,
        "Sequence finished should appear before Performance block"
    );

    // Just-in-time orchestration composes each step *at its turn*, inside the
    // window `steps → step N` measures, so composition nests under its own step
    // rather than under a separate `environment setup → step preparation`
    // bucket. The G-2 hazard that split guarded against — a compose rendered
    // larger than its parent — cannot arise once compose is inside the parent.
    assert!(
        !plain.contains("step preparation"),
        "composition is no longer metered outside the step window; got: {plain}"
    );
    assert!(
        plain.contains("composition"),
        "each step should carry its own composition detail; got: {plain}"
    );
    assert_eq!(
        plain.matches("step 1: alpha").count(),
        1,
        "each step appears once, under `steps`; stderr: {plain}"
    );
    assert_eq!(
        plain.matches("step 2: beta").count(),
        1,
        "each step appears once, under `steps`; stderr: {plain}"
    );
}

// ---------------------------------------------------------------------------
// Partial report on fail-fast interruption
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn sequence_perf_with_fail_fast_still_renders_partial_report() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - alpha
  - beta
  - gamma
---
Step {{state}}
"#,
    )
    .unwrap();

    // Step 1 succeeds, step 2 fails, step 3 should not run.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
if [ "$count" = "2" ]; then
  exit 7
fi
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args([
            "sequence",
            "--goose",
            "--perf",
            "--fail-fast",
            "true",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    let perf_count = plain.matches("Performance").count();
    assert_eq!(
        perf_count, 1,
        "expected exactly one Performance block, found {perf_count}; stderr: {plain}"
    );

    assert!(
        plain.contains("partial sequence metrics"),
        "stderr should contain partial sequence metrics note; got: {plain}"
    );
}

// ---------------------------------------------------------------------------
// Startup timings propagation through the sequence command
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn sequence_perf_propagates_startup_timings() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - alpha
---
Step {{state}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("arg parsing"),
        "stderr should contain arg parsing timing; got: {plain}"
    );
    assert!(
        plain.contains("config loading"),
        "stderr should contain config loading timing; got: {plain}"
    );
    assert!(
        plain.contains("tracing init"),
        "stderr should contain tracing init timing; got: {plain}"
    );
}
