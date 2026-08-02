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
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
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
    assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
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
    assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success()
        .stderr(contains("Sequence finished"));
}

// ============================================================================
// Sequence Plus: dynamic and file-backed sources (Phase 4)
// ============================================================================

/// A referenced data file drives the sequence end to end: the offset selects
/// the nested list, the operator supplies each step's name, and every step
/// composes with that state through the normal invocation path.
#[test]
fn sequence_resolves_a_data_file_with_offset_and_operator() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let prompts_path = workspace.path().join("all-prompts.txt");

    fs::write(
        workspace.path().join("things.yaml"),
        r#"description: research into things
colors:
    description: the best colors in the spectrum
    data:
        - color: blue
        - color: green
"#,
    )
    .unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
sequence: things.yaml -> colors.data::map(color, name)
---
COLOR={{state}} STEP={{state.index}}/{{state.count}}
"#,
    )
    .unwrap();

    #[cfg(unix)]
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
prev=""
for arg in "$@"; do
  if [ "$prev" = "-t" ]; then
    printf '%s\n' "$arg" >> "$CLAUDINE_PROMPTS_FILE"
  fi
  prev="$arg"
done
exit 0
"#,
    );
    #[cfg(windows)]
    {
        let source = path_dir.join("goose.rs");
        fs::write(
            &source,
            r#"use std::{env, fs::OpenOptions, io::Write};

fn main() {
    let path = env::var_os("CLAUDINE_PROMPTS_FILE").unwrap();
    let mut output = OpenOptions::new().create(true).append(true).open(path).unwrap();
    let mut previous = None;
    for argument in env::args().skip(1) {
        if previous.as_deref() == Some("-t") {
            writeln!(output, "{argument}").unwrap();
        }
        previous = Some(argument);
    }
}
"#,
        )
        .unwrap();
        let status = std::process::Command::new("rustc")
            .args(["--edition=2024"])
            .arg(&source)
            .arg("-o")
            .arg(path_dir.join("goose.exe"))
            .status()
            .unwrap();
        assert!(status.success(), "failed to compile the Goose test fixture");
    }

    assert_cmd::Command::cargo_bin("claudine").unwrap()
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
        captured.contains("COLOR=blue STEP=1/2"),
        "first step should carry the mapped name; captured: {captured}"
    );
    assert!(
        captured.contains("COLOR=green STEP=2/2"),
        "second step should carry the mapped name; captured: {captured}"
    );
}

/// A dynamic source that resolves to nothing is a graceful no-op: a styled
/// notice on stderr, exit `0`, and no provider launched.
#[test]
fn sequence_with_an_empty_dynamic_source_is_a_no_op() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let launched_path = workspace.path().join("launched.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
items: []
sequence: "{{ items }}"
---
Work on {{state}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf 'launched\n' >> "$CLAUDINE_LAUNCHED_FILE"
exit 0
"#,
    );

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_LAUNCHED_FILE", &launched_path)
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .clone();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("0 steps"),
        "an empty dynamic source must report itself; stderr: {stderr}"
    );
    assert!(
        !launched_path.exists(),
        "no provider may be launched for a zero-step sequence"
    );
}

/// A statically empty list keeps its authoring error and a non-zero exit —
/// the graceful no-op is for *dynamic* emptiness only.
#[test]
fn sequence_with_a_static_empty_list_still_fails() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence: []\n---\nWork on {{state}}.\n",
    )
    .unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(contains("empty"));
}

/// The retired `list:` document shape is rejected with a message that names
/// the property authors must migrate to.
#[test]
fn sequence_rejects_the_retired_list_shape() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("legacy.yaml"),
        "kind: sequence\nlist:\n  - name: one\n",
    )
    .unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence: legacy.yaml\n---\nWork on {{state}}.\n",
    )
    .unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(contains("sequence:"));
}

/// A formal sequence YAML invoked *directly* accepts the identical document
/// shape it does when referenced — the asymmetry the retired `list:` form
/// carried is gone.
#[test]
fn sequence_yaml_invoked_directly_uses_the_same_shape() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let prompts_path = workspace.path().join("all-prompts.txt");

    let yaml_file = workspace.path().join("steps.yaml");
    fs::write(
        &yaml_file,
        r#"kind: sequence
sequence:
    - name: alpha
      shell: echo alpha
    - name: beta
      shell: echo beta
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'launched\\n' >> \"$CLAUDINE_PROMPTS_FILE\"\nexit 0\n",
    );

    // Every step carries an executable, so a bodyless YAML source is valid.
    // Phase 4 only has to resolve the plan; execution of `shell:` steps lands
    // in phase 7, so this asserts the shape is *accepted*, not that it runs.
    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_PROMPTS_FILE", &prompts_path)
        .current_dir(workspace.path())
        .args(["sequence", "--goose", yaml_file.to_str().unwrap()])
        .assert()
        .get_output()
        .clone();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        !stderr.contains("must have") && !stderr.contains("wrong structure"),
        "the direct-invocation shape must be accepted; stderr: {stderr}"
    );
}

// ============================================================================
// Phase 5 — static preflight
//
// Every rejection below must land *before* a provider is launched, so each
// test installs a fake `goose` that appends to `CLAUDINE_PROMPTS_FILE` and
// asserts the file never appears. That file is the zero-child-launch witness.
// ============================================================================

/// Install a fake provider that records every launch, and return the witness
/// path that must stay absent when preflight aborts.
#[cfg(unix)]
fn preflight_witness(workspace: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let path_dir = workspace.join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'launched\\n' >> \"$CLAUDINE_PROMPTS_FILE\"\nexit 0\n",
    );
    (path_dir, workspace.join("launches.txt"))
}

/// Run `claudine sequence` against `file`, returning `(stderr, launched)`.
#[cfg(unix)]
fn run_preflight(
    workspace: &std::path::Path,
    file: &std::path::Path,
    extra_args: &[&str],
) -> (String, bool) {
    let (path_dir, witness) = preflight_witness(workspace);
    let mut args: Vec<&str> = vec!["sequence", "--goose"];
    args.extend_from_slice(extra_args);
    let file_arg = file.to_str().unwrap();
    args.push(file_arg);

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace)
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_PROMPTS_FILE", &witness)
        .current_dir(workspace)
        .args(&args)
        .assert()
        .get_output()
        .clone();

    (
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        witness.exists(),
    )
}

/// A `kind: group` document is never directly executable; the rejection names
/// the construct rather than reporting a missing `sequence:` property.
#[test]
#[cfg(unix)]
fn sequence_rejects_direct_group_execution() {
    let workspace = tempdir().unwrap();
    let group = workspace.path().join("group.yaml");
    fs::write(
        &group,
        "kind: group\nname: bundle\ntasks:\n    - shell: echo x\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &group, &[]);
    assert!(
        stderr.contains("kind: group") && stderr.contains("sequence task"),
        "the rejection must name the construct and the fix; stderr: {stderr}"
    );
    assert!(!launched, "no provider may launch for a rejected document");
}

/// A `prompt:` task pointing at a document that itself declares `sequence:` is
/// a nested sequence, which v1 rejects during preflight.
#[test]
#[cfg(unix)]
fn sequence_rejects_a_nested_sequence_prompt_document() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("inner.md"),
        "---\nsequence:\n    - one\n---\nInner.\n",
    )
    .unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      prompt: inner.md\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &[]);
    assert!(
        stderr.contains("nested sequences are not supported"),
        "stderr: {stderr}"
    );
    assert!(!launched, "preflight must abort before any launch");
}

/// A task reference cycle reports the whole chain, not just the repeated file.
#[test]
#[cfg(unix)]
fn sequence_rejects_a_reference_cycle_with_the_full_chain() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("a.yaml"),
        "kind: task\ntask: b.yaml\n",
    )
    .unwrap();
    fs::write(
        workspace.path().join("b.yaml"),
        "kind: task\ntask: a.yaml\n",
    )
    .unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      task: a.yaml\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &[]);
    assert!(stderr.contains("reference cycle"), "stderr: {stderr}");
    // Long absolute paths are hyphen-wrapped by the terminal renderer, so the
    // chain is asserted by its arrow count (entry → a → b → a) rather than by
    // matching path substrings that a line break may have split.
    assert_eq!(
        stderr.matches('\u{2192}').count(),
        3,
        "the chain must name every hop; stderr: {stderr}"
    );
    assert!(!launched);
}

/// Group `loop` commit semantics are unratified, so a group carrying `loop`
/// is blocked with an actionable error instead of invented semantics.
#[test]
#[cfg(unix)]
fn sequence_rejects_group_loop() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      group:\n          name: looper\n          loop:\n              while: \"true\"\n          tasks:\n              - shell: echo x\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &[]);
    assert!(stderr.contains("group `loop`"), "stderr: {stderr}");
    assert!(!launched);
}

/// A shell task that depends on `outputs` could never be approved byte-for-byte
/// up front, so preflight rejects it and points at the alternative.
#[test]
#[cfg(unix)]
fn sequence_rejects_a_shell_command_depending_on_outputs() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      shell: \"echo {{ last(outputs) }}\"\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &[]);
    assert!(
        stderr.contains("outputs") && stderr.contains("preflight"),
        "stderr: {stderr}"
    );
    assert!(!launched);
}

/// Two tasks in one parallel group rewriting the same inline-compose document
/// would race; preflight holds the whole graph and catches it statically.
#[test]
#[cfg(unix)]
fn sequence_rejects_a_parallel_write_back_collision() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("target.md"),
        "---\nprompt: Write something.\n---\nold\n",
    )
    .unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      group:\n          name: racers\n          execution: parallel\n          tasks:\n              - prompt: target.md\n                name: first\n              - prompt: target.md\n                name: second\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &[]);
    assert!(
        stderr.contains("write back")
            && stderr.contains("racers")
            && stderr.contains("(`first` and `second`)"),
        "stderr: {stderr}"
    );
    assert!(!launched);
}

/// Preflight failures are abort-all: `fail_fast: false` governs *execution*
/// outcomes and cannot degrade preparation to best-effort. The later, valid
/// step must never run.
#[test]
#[cfg(unix)]
fn preflight_failure_aborts_even_with_fail_fast_false() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("inner.md"),
        "---\nsequence:\n    - one\n---\nInner.\n",
    )
    .unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nfail_fast: false\nsequence:\n    - name: bad\n      prompt: inner.md\n    - name: good\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &["--fail-fast", "false"]);
    assert!(
        stderr.contains("nested sequences are not supported"),
        "stderr: {stderr}"
    );
    assert!(
        !launched,
        "`fail_fast: false` must not let the valid step run past a preflight failure"
    );
}

/// `--dry-run` performs the identical preflight walk, so a graph that cannot
/// be prepared is reported rather than rendered as a runnable plan.
#[test]
#[cfg(unix)]
fn dry_run_performs_the_same_preflight() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("inner.md"),
        "---\nsequence:\n    - one\n---\nInner.\n",
    )
    .unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      prompt: inner.md\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &["--dry-run"]);
    assert!(
        stderr.contains("nested sequences are not supported"),
        "--dry-run must run the same preflight; stderr: {stderr}"
    );
    assert!(!launched);
}

/// A well-formed graph passes preflight: a `kind: task` file, a serial group,
/// and a whitelisted shell command all resolve without a rejection.
#[test]
#[cfg(unix)]
fn well_formed_graph_passes_preflight() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();
    fs::write(
        workspace.path().join("task.yaml"),
        "kind: task\nshell: echo from-task\n",
    )
    .unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      task: task.yaml\n    - name: two\n      group:\n          name: bundle\n          tasks:\n              - shell: echo grouped\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, _) = run_preflight(workspace.path(), &md_file, &["--dry-run"]);
    for rejection in [
        "reference cycle",
        "not supported",
        "is invalid",
        "could not be resolved",
    ] {
        assert!(
            !stderr.contains(rejection),
            "a well-formed graph must not be rejected (`{rejection}`); stderr: {stderr}"
        );
    }
}

// ============================================================================
// Pre-flight status ordering (review-5 finding 5)
// ============================================================================

/// "Starting pre-flight checks" must reach the user *before* Phase 1c, not
/// after it.
///
/// Phase 1c does the sequence-wide schema validation and shell approval, and it
/// is the part that can stall or prompt. A status emitted after it would be
/// describing finished work while the user stared at nothing throughout.
///
/// The proof is a run that never survives Phase 1c: a `$schema` requiring a
/// property no step supplies aborts the sequence there. If the status still
/// appears, it can only have been written before Phase 1c ran.
#[cfg(unix)]
#[test]
fn starting_preflight_status_precedes_phase_1c_work() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\n$schema:\n  topic: 'string(required)'\nsequence:\n    - alpha\n---\nStep about {{topic}}.\n",
    )
    .unwrap();

    let (stderr, launched) = run_preflight(workspace.path(), &md_file, &[]);
    assert!(
        stderr.to_lowercase().contains("sequence missing properties"),
        "the fixture must abort inside Phase 1c for this test to prove \
         anything; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Starting pre-flight checks"),
        "Phase 1c aborted without the starting status ever being rendered, so \
         the status is still emitted after pre-flight; stderr:\n{stderr}"
    );
    assert!(!launched, "no provider session may launch; stderr:\n{stderr}");
}

/// The two pre-flight statuses bracket the work rather than both trailing it.
#[cfg(unix)]
#[test]
fn preflight_statuses_bracket_phase_1c() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence:\n    - name: one\n      shell: echo hello\n---\nBody.\n",
    )
    .unwrap();

    let (stderr, _) = run_preflight(workspace.path(), &md_file, &["--dry-run"]);
    let starting = stderr
        .find("Starting pre-flight checks")
        .unwrap_or_else(|| panic!("no starting status; stderr:\n{stderr}"));
    // Just the label: the sentence after it folds at the terminal width, so a
    // longer needle would match nothing on a narrow host.
    let approved = stderr
        .find("Preflight:")
        .unwrap_or_else(|| panic!("no preflight-complete status; stderr:\n{stderr}"));
    assert!(
        starting < approved,
        "the starting status must precede the approval status; stderr:\n{stderr}"
    );
}
