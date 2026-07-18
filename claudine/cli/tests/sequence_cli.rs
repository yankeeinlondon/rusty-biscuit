//! Integration tests for `claudine sequence`.
//!
//! Exercises the wrapper-grade sequence orchestrator end-to-end: step
//! iteration, fail-fast semantics, FAIL_FAST propagation, per-step state
//! injection, cross-step shell approval reuse, and summary output.
//!
//! Themed coverage lives in sibling binaries: external-file and
//! magic-reference resolution in `sequence_magic_reference.rs`, schema
//! validation and `step_timeout` in `sequence_schema.rs`, and inline
//! `prompt`-property behavior in `sequence_prompt_property.rs`.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

// ============================================================================
// Validation tests
// ============================================================================

#[test]
fn sequence_requires_positional_arg() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["sequence"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("ARG"), "usage should show ARG positional");
}

#[test]
fn sequence_missing_file_with_setter_only() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["sequence", "topic=async"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("missing file reference"),
        "expected missing-file error, got: {plain}"
    );
}

#[test]
fn sequence_errors_when_source_has_no_sequence_property() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("plain.md");
    fs::write(
        &md_file,
        "---\ntitle: No sequence here\n---\n# Plain content\n",
    )
    .unwrap();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["sequence", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("sequence"),
        "error should mention missing `sequence` property; stderr: {plain}"
    );
}

#[test]
fn sequence_malformed_root_frontmatter_fence_surfaces_typed_parse_error() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("malformed-sequence.md");
    fs::write(
        &md_file,
        "----\nsequence:\n  - alpha\ndescription: near-miss fence\n----\nStep {{state}}\n",
    )
    .unwrap();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["sequence", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("frontmatter"),
        "expected frontmatter parse context; stderr:\n{plain}"
    );
    assert!(
        plain.contains("exactly three dashes") || plain.contains("FrontmatterFenceMismatch"),
        "expected malformed-fence hint or typed fence mismatch; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("No model specified") && !plain.contains("No runnable providers"),
        "source-load failures must not degrade into provider/model errors; stderr:\n{plain}"
    );
}

// ============================================================================
// fail-fast semantics
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_fail_fast_true_stops_on_first_failure() {
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
Run step {{state}}
"#,
    )
    .unwrap();

    // Provider fails on every call. With fail-fast=true, only the first
    // step should run and the sequence should abort.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
exit 7
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args([
            "sequence",
            "--goose",
            "--fail-fast",
            "true",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "1",
        "fail-fast should stop after first failing step"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("1 failed"),
        "summary should note exactly one failure; stderr: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn sequence_fail_fast_false_continues_and_exits_one_on_any_failure() {
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
fail_fast: false
---
Run step {{state}}
"#,
    )
    .unwrap();

    // Provider fails on the second call only. All three steps should run.
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
  exit 11
fi
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "3",
        "fail-fast=false should keep running after a failure"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("2 succeeded") && plain.contains("1 failed"),
        "summary should record 2 successes and 1 failure; stderr: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn sequence_opencode_requires_model_when_missing() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("opencode-args.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - only
---
Run step {{state}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
    echo '[]'
    exit 0
fi
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env_remove("MODEL")
        .current_dir(workspace.path())
        .args(["sequence", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("No model specified!"),
        "stderr should explain the missing non-interactive OpenCode model; stderr: {plain}"
    );
    assert!(
        plain.contains("OPENCODE_MODEL"),
        "stderr should tell the user how to provide the model; stderr: {plain}"
    );
    assert!(
        !args_path.exists(),
        "OpenCode should not launch when the model is missing"
    );
}

#[cfg(unix)]
#[test]
fn sequence_cli_fail_fast_flag_overrides_document_default() {
    // Document sets fail_fast: false, but CLI overrides to true.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - one
  - two
  - three
fail_fast: false
---
Do step {{state}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
exit 3
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args([
            "sequence",
            "--goose",
            "--fail-fast",
            "true",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "1",
        "CLI --fail-fast=true must override document fail_fast=false"
    );
}

// ============================================================================
// FAIL_FAST propagation
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_propagates_fail_fast_to_child_env_and_prompt() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let env_path = workspace.path().join("child-env.txt");
    let prompt_path = workspace.path().join("child-stdin.txt");

    let md_file = workspace.path().join("seq.md");
    // The document body interpolates {{env.CLAUDINE_FAIL_FAST}} so we can verify
    // the composed prompt saw the same value as the child env.
    fs::write(
        &md_file,
        r#"---
sequence:
  - only
---
CLAUDINE_FAIL_FAST={{env.CLAUDINE_FAIL_FAST}} STATE={{state}}
"#,
    )
    .unwrap();

    // Provider: record CLAUDINE_FAIL_FAST from env and capture the `-t <prompt>`
    // argument (Goose delivers the composed prompt via -t) so the test
    // can inspect both.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf 'CLAUDINE_FAIL_FAST=%s\n' "$CLAUDINE_FAIL_FAST" > "$CLAUDINE_ENV_FILE"
prev=""
for arg in "$@"; do
  if [ "$prev" = "-t" ]; then
    printf '%s' "$arg" > "$CLAUDINE_STDIN_FILE"
  fi
  prev="$arg"
done
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_ENV_FILE", &env_path)
        .env("CLAUDINE_STDIN_FILE", &prompt_path)
        .current_dir(workspace.path())
        .args([
            "sequence",
            "--goose",
            "--fail-fast",
            "false",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let child_env = fs::read_to_string(&env_path).unwrap();
    assert!(
        child_env.contains("CLAUDINE_FAIL_FAST=false"),
        "child env should expose CLAUDINE_FAIL_FAST=false; env_file: {child_env}"
    );

    let child_prompt = fs::read_to_string(&prompt_path).unwrap();
    assert!(
        child_prompt.contains("CLAUDINE_FAIL_FAST=false"),
        "composed prompt should interpolate {{{{env.CLAUDINE_FAIL_FAST}}}} to false; stdin was: {child_prompt}"
    );
    assert!(
        child_prompt.contains("STATE=only"),
        "composed prompt should interpolate {{{{state}}}} per step; stdin was: {child_prompt}"
    );
}

#[cfg(unix)]
#[test]
fn sequence_shorthand_override_reaches_prompt() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let prompt_path = workspace.path().join("child-stdin.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - only
---
TOPIC={{topic}} STATE={{state}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
prev=""
for arg in "$@"; do
  if [ "$prev" = "-t" ]; then
    printf '%s' "$arg" > "$CLAUDINE_STDIN_FILE"
  fi
  prev="$arg"
done
exit 0
"#,
    );

    // Setter placed BEFORE the file reference to exercise the positional parser.
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_STDIN_FILE", &prompt_path)
        .current_dir(workspace.path())
        .args([
            "sequence",
            "--goose",
            "topic=async-traits",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let child_prompt = fs::read_to_string(&prompt_path).unwrap();
    assert!(
        child_prompt.contains("TOPIC=async-traits"),
        "composed prompt should interpolate the shorthand `topic` override; stdin was: {child_prompt}"
    );
}

#[cfg(unix)]
#[test]
fn sequence_shorthand_wins_over_set_flag() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let prompt_path = workspace.path().join("child-stdin.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - only
---
MODE={{mode}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
prev=""
for arg in "$@"; do
  if [ "$prev" = "-t" ]; then
    printf '%s' "$arg" > "$CLAUDINE_STDIN_FILE"
  fi
  prev="$arg"
done
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_STDIN_FILE", &prompt_path)
        .current_dir(workspace.path())
        .args([
            "sequence",
            "--goose",
            "--set",
            r#"{"mode":"slow"}"#,
            "mode=fast",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let child_prompt = fs::read_to_string(&prompt_path).unwrap();
    assert!(
        child_prompt.contains("MODE=fast"),
        "shorthand setter should beat --set on overlapping keys; stdin was: {child_prompt}"
    );
}

// ============================================================================
// Per-step state injection
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_injects_per_step_state_into_prompt() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let prompts_path = workspace.path().join("all-prompts.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - red
  - green
  - blue
---
COLOR={{state}} STEP={{state.index}}/{{state.count}}
"#,
    )
    .unwrap();

    // Append each invocation's prompt (delivered via Goose's `-t <prompt>`
    // flag) to a single file so we can verify step-by-step state/step
    // interpolation.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
prev=""
for arg in "$@"; do
  if [ "$prev" = "-t" ]; then
    {
      printf -- '--- invocation ---\n'
      printf '%s\n' "$arg"
    } >> "$CLAUDINE_PROMPTS_FILE"
  fi
  prev="$arg"
done
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_PROMPTS_FILE", &prompts_path)
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let captured = fs::read_to_string(&prompts_path).unwrap();
    assert!(
        captured.contains("COLOR=red STEP=1/3"),
        "first step should have state=red step=1; captured: {captured}"
    );
    assert!(
        captured.contains("COLOR=green STEP=2/3"),
        "middle step should have state=green step=2; captured: {captured}"
    );
    assert!(
        captured.contains("COLOR=blue STEP=3/3"),
        "last step should have state=blue step=3; captured: {captured}"
    );
}

// ============================================================================
// Cross-step approval reuse
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_preflight_applies_whitelist_on_every_step() {
    // Smoke test: verifies that pre-flight shell approval runs for every
    // step of a sequence and accepts whitelisted commands without a TTY.
    //
    // NOTE: because `echo` is whitelisted, pre-flight short-circuits
    // *before* ever touching the shared approval cache (see
    // `check_whitelist` in `harness::shell`). The end-to-end guarantee
    // that "allow once" approvals survive across sequence steps is
    // verified at the library level in:
    //   - `composition::preflight` tests
    //     (`warm_cache_prevents_second_handler_invocation`,
    //      `shared_cache_across_distinct_options_prevents_reprompt`,
    //      `shared_cache_covers_harness_command_path`,
    //      `shared_cache_spans_template_and_harness_sources`)
    // Those exercise non-whitelisted commands with mock approval handlers.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // Whitelist echo in the workspace policy root.
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence:
  - one
  - two
---
Step {{state}}: ::shell echo approved
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    // Successful exit means every step's pre-flight accepted the
    // whitelisted `echo` command without a TTY.
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();
}

// ============================================================================
// Summary output
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_summary_emits_final_line() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n  - alpha\n  - beta\n---\nStep {{state}}\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success()
        .stderr(contains("Sequence finished"));
}
