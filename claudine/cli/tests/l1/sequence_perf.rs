//! Integration tests for `claudine sequence --perf`.
//!
//! Validates that sequence performance reporting aggregates step-level
//! metrics into a single end-of-run report, including CLI overhead
//! propagation and partial-metrics handling on fail-fast.

use std::fs;
use crate::common;
use common::{CliProcessFixture, strip_ansi, write_executable};

// ---------------------------------------------------------------------------
// Single aggregated report for a successful multi-step sequence
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn sequence_perf_renders_single_aggregated_report() {
    let fixture = CliProcessFixture::named("sequence-perf-aggregated");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - alpha
  - beta
---
Step {{seq}}
"#,
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    let assert = fixture
        .command()
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
    let fixture = CliProcessFixture::named("sequence-perf-startup-timings");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - alpha
  - beta
  - gamma
---
Step {{seq}}
"#,
    )
    .unwrap();

    // Step 1 succeeds, step 2 fails, step 3 should not run.
    write_executable(
        &fixture.bin_dir().join("goose"),
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

    let assert = fixture
        .command()
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .args([
            "sequence",
            "--goose",
            "--perf",
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
    let fixture = CliProcessFixture::named("sequence-perf-partial");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - alpha
---
Step {{seq}}
"#,
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    let assert = fixture
        .command()
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

/// R2-F2 control: a sequence composes an initialized step at its turn, inside
/// the `step N` window, so its composition (here outlasting everything prepared
/// before `initialize`) never lands in a window that closed earlier.
#[cfg(unix)]
#[test]
fn sequence_perf_initialized_step_composition_nests_under_its_step() {
    let fixture = CliProcessFixture::named("sequence-perf-initialized-step");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact sleep 0.1\n");

    // The step is an inline prompt task: its `prompt` is what it composes.
    let step_file = fixture.cwd().join("step.md");
    fs::write(
        &step_file,
        "---\nprompt: |\n  Update the body.\n\n  ::shell sleep 0.1\ninitialize:\n  - {stdout: initialized}\n---\n# Body\n",
    )
    .unwrap();
    let md_file = fixture.cwd().join("seq.md");
    fs::write(&md_file, "---\nsequence:\n  - name: one\n    prompt: step.md\n---\nSequence\n").unwrap();
    common::InlineAgentStub::new(&step_file)
        .body("Replacement body\n")
        .install(fixture.bin_dir(), "goose");

    let output = fixture
        .command()
        .args(["sequence", "--goose", "--perf", md_file.to_str().unwrap()])
        .output()
        .unwrap();
    let plain = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(0), "{plain}");
    let report = &plain[plain.find("Performance").unwrap_or_else(|| panic!("report missing:\n{plain}"))..];
    let step = report
        .find("step 1: one")
        .unwrap_or_else(|| panic!("step missing:\n{report}"));
    assert!(
        report[step..].contains("composition"),
        "the step's composition nests under its step:\n{report}"
    );
}
