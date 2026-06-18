//! Integration tests for `claudine sequence`.
//!
//! Exercises the wrapper-grade sequence orchestrator end-to-end: step
//! iteration, fail-fast semantics, external file references, FAIL_FAST
//! propagation, and cross-step shell approval reuse.

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
// External file references
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_resolves_external_file_via_relative_path() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    // External YAML in the same directory as the source markdown.
    let steps_yaml = workspace.path().join("steps.yaml");
    fs::write(&steps_yaml, "sequence:\n  - alpha\n  - beta\n").unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(&md_file, "---\nsequence: steps.yaml\n---\nStep {{state}}\n").unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "2", "both YAML-defined steps should run");
}

#[cfg(unix)]
#[test]
fn sequence_resolves_external_file_via_magic_reference() {
    // Initialize a real git repo so FileReference's @ magic (driven by
    // git2::Repository::discover) can find the repo root.
    let workspace = tempdir().unwrap();
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    let git_init_ok = std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !git_init_ok {
        eprintln!("git init unavailable; skipping magic reference test");
        return;
    }
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    // Magic reference target: @fixtures/steps.yaml resolves from git root.
    let fixtures_dir = repo_root.join("fixtures");
    fs::create_dir_all(&fixtures_dir).unwrap();
    fs::write(
        fixtures_dir.join("steps.yaml"),
        "sequence:\n  - one\n  - two\n  - three\n",
    )
    .unwrap();

    let md_dir = repo_root.join("prompts");
    fs::create_dir_all(&md_dir).unwrap();
    let md_file = md_dir.join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence: '@fixtures/steps.yaml'\n---\nStep {{state}}\n",
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
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(&repo_root)
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "3",
        "@ magic reference should resolve to fixtures/steps.yaml and all 3 steps should run"
    );
}

#[cfg(unix)]
#[test]
fn sequence_magic_reference_uses_source_doc_location_not_cwd() {
    // A `@` magic reference in the sequence frontmatter MUST resolve
    // relative to the markdown source document's location, not the
    // process CWD. Otherwise a user running
    // `claudine sequence /abs/path/to/seq.md` from some other directory
    // would get the wrong file or a spurious "not found" error.
    //
    // This test sets up TWO distinct git repos: `repo_root/` (where the
    // source doc lives) and `unrelated/` (the process CWD). Each repo
    // has its own `fixtures/steps.yaml` with a different step count.
    // If resolution was driven by CWD, the wrong file would be loaded.
    let workspace = tempdir().unwrap();

    // --- Primary repo: source doc + correct fixtures (2 steps) ---
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    let git_init_ok = std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !git_init_ok {
        eprintln!("git init unavailable; skipping magic reference test");
        return;
    }
    let correct_fixtures = repo_root.join("fixtures");
    fs::create_dir_all(&correct_fixtures).unwrap();
    fs::write(
        correct_fixtures.join("steps.yaml"),
        "sequence:\n  - alpha\n  - beta\n",
    )
    .unwrap();
    let md_dir = repo_root.join("prompts");
    fs::create_dir_all(&md_dir).unwrap();
    let md_file = md_dir.join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence: '@fixtures/steps.yaml'\n---\nStep {{state}}\n",
    )
    .unwrap();

    // --- Unrelated repo: decoy fixtures (3 steps) used iff resolution is CWD-driven ---
    let unrelated = workspace.path().join("unrelated");
    fs::create_dir_all(&unrelated).unwrap();
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&unrelated)
        .status()
        .ok();
    let decoy_fixtures = unrelated.join("fixtures");
    fs::create_dir_all(&decoy_fixtures).unwrap();
    fs::write(
        decoy_fixtures.join("steps.yaml"),
        "sequence:\n  - wrong1\n  - wrong2\n  - wrong3\n",
    )
    .unwrap();

    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
exit 0
"#,
    );

    // Run FROM the unrelated repo, but target the doc inside repo_root.
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(&unrelated)
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "2",
        "@ magic reference must resolve from the source document's location \
         (repo_root/fixtures/steps.yaml — 2 steps), not the process CWD \
         (unrelated/fixtures/steps.yaml — 3 steps). Got {calls} invocations."
    );
}

#[cfg(unix)]
#[test]
fn sequence_external_file_not_found_fails_clearly() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(&md_file, "---\nsequence: does-not-exist.yaml\n---\nBody\n").unwrap();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["sequence", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("external sequence") || plain.contains("does-not-exist.yaml"),
        "error should mention the missing external file; stderr: {plain}"
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
COLOR={{state}} STEP={{step}}/{{total_steps}}
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

// ============================================================================
// Phase 5: schema validation
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_aggregates_missing_required_properties_across_steps() {
    // A non-TTY sequence run with `$schema` declaring a required property
    // that no step supplies must abort BEFORE any provider session is
    // launched, surfacing every failing step in a single error.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
sequence:
  - alpha
  - beta
---
Step about {{topic}}.
"#,
    )
    .unwrap();

    // Provider stub records every invocation so we can prove it was
    // never called.
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // Strong assertion: error must be Claudine's typed CompositionError
    // surface AND per-step aggregated (`sequence missing properties`),
    // not Darkmatter's raw MarkdownError or a single-doc MissingProperties.
    // Catches regressions where the per-step preflight short-circuits
    // aggregation.
    assert!(
        plain.contains("CompositionError"),
        "expected typed CompositionError surface (not raw MarkdownError); stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("sequence missing properties"),
        "expected aggregated `sequence missing properties` (not single-doc); stderr:\n{plain}"
    );
    assert!(
        !plain.contains("MarkdownError"),
        "raw MarkdownError leaked instead of typed Claudine error; stderr:\n{plain}"
    );
    // Both steps must appear in the aggregated report.
    assert!(
        plain.contains("Step 1"),
        "expected per-step `Step 1` header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("Step 2"),
        "expected per-step `Step 2` header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("alpha") && plain.contains("beta"),
        "expected both step names in the aggregated report; stderr:\n{plain}"
    );
    assert!(
        plain.contains("topic"),
        "expected the `topic` property name; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched; stub recorded a call"
    );
}

#[cfg(unix)]
#[test]
fn sequence_set_override_satisfies_required_schema() {
    // The aggregated path retries cleanly when the user supplies the
    // missing value via `--set` so no error reaches the provider.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
sequence:
  - one
  - two
---
Working on {{topic}} ({{state}}).
"#,
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
        .args([
            "sequence",
            "--goose",
            md_file.to_str().unwrap(),
            "topic=async",
        ])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn sequence_unsupported_shape_surfaces_typed_error_under_tty_pref() {
    // When the run is non-TTY (stdin/stderr are pipes here), the sequence
    // path MUST emit an aggregated `sequence missing properties` report
    // rather than promoting the first unsupported shape to
    // `UnsupportedInteractiveSchema`. This matches the direct `compose`
    // path's behavior in `pre_validate_with_interactive_collection`,
    // which only short-circuits to the unsupported error when interactive
    // collection is actually allowed.
    //
    // Regression target (review-5 medium): we previously promoted the
    // unsupported shape unconditionally, which forced users to see a
    // shape-specific error even when they could have fixed the sequence
    // by editing two files in one pass via the aggregated report.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  config: 'object(required)'
sequence:
  - alpha
  - beta
---
Step about {{config}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // Strong assertions: non-TTY must produce the aggregated
    // sequence-missing surface, NOT the unsupported-shape error.
    assert!(
        plain.to_lowercase().contains("sequence missing properties"),
        "expected aggregated `sequence missing properties` surface; stderr:\n{plain}"
    );
    assert!(
        !plain.to_lowercase().contains("unsupported interactive schema"),
        "non-TTY must NOT short-circuit to UnsupportedInteractiveSchema; stderr:\n{plain}"
    );
    // Both step labels should appear in the aggregated report so a user
    // can fix the full sequence in one pass.
    assert!(
        plain.contains("Step 1") && plain.contains("Step 2"),
        "expected per-step `Step 1` and `Step 2` headers; stderr:\n{plain}"
    );
    assert!(
        plain.contains("alpha") && plain.contains("beta"),
        "expected both step names; stderr:\n{plain}"
    );
    assert!(
        plain.contains("config"),
        "expected the `config` property name in error report; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched; stub recorded a call"
    );
}

// ============================================================================
// Per-step step_timeout override
// ============================================================================

/// Verifies that a step-level `step_timeout` (declared in a sequence step's
/// raw state object) overrides the document-level `step_timeout`. The step
/// overlay passes the step's raw state unchanged under the `state` key, so
/// per-step `step_timeout` is surfaced via `{{ state.step_timeout || ... }}`
/// interpolation in the document frontmatter.
///
/// Test shape:
/// - Document `step_timeout` falls back to `30s` when the step does not
///   declare one, but uses `state.step_timeout` when the step does.
/// - Step 1 has no `step_timeout`, so the effective deadline is `30s`. The
///   fake provider completes quickly and the step succeeds.
/// - Step 2 declares `step_timeout: 1s` at the step level. The fake
///   provider emits a single start event and then stalls, so the
///   step-silence deadline fires at ~1s and the step is killed with a
///   `step_timeout` error well before the 30s document fallback would.
#[cfg(unix)]
#[test]
fn sequence_per_step_step_timeout_override() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    // Document-level step_timeout defaults to 30s; a step may override by
    // setting `step_timeout` at the step level, which the overlay exposes
    // via `state.step_timeout`. Step 2 sets it to 1s so the test completes
    // quickly once its fake provider stalls.
    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
step_timeout: '{{ state.step_timeout || "30s" }}'
sequence:
  - name: fast
  - name: slow
    step_timeout: 1s
---
Run step {{ state.name }}
"#,
    )
    .unwrap();

    // Fake opencode tracks invocation count. Step 1 emits a structured
    // session quickly and exits; step 2 emits a start event only and then
    // sleeps well past the step-level 1s deadline. The wait loop must
    // terminate the silent child without letting the test block for the
    // document-level 30s fallback.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
    echo '[]'
    exit 0
fi
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"

if [ "$count" = "1" ]; then
  printf '%s\n' '{"type":"step_start","sessionID":"ses_step1"}'
  printf '%s\n' '{"type":"text","text":"fast step done"}'
  printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":10}'
  exit 0
fi

# Step 2: emit start + finish so both `last_event_at` and
# `provider_status` are populated, then stall so the step-silence
# deadline fires. The OpenCode `provider_status` grace requires at
# least one observed `step_finish` boundary before allowing
# `step_timeout` to fire.
printf '%s\n' '{"type":"step_start","sessionID":"ses_step2"}'
printf '%s\n' '{"type":"step_finish","sessionID":"ses_step2","part":{"reason":"tool-calls","tokens":{"input":1,"output":1,"total":2}}}'
sleep 30
exit 0
"#,
    );

    let run_start = std::time::Instant::now();
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .timeout(std::time::Duration::from_secs(25))
        .args(["sequence", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .failure();
    let elapsed = run_start.elapsed();

    // The step-level 1s budget (plus the 5s SIGTERM grace) must win well
    // before the 30s document fallback. Allow generous slack for slow CI.
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "per-step step_timeout override should fire quickly; run took {elapsed:?}"
    );

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "2",
        "both steps should be launched before the stall on step 2 is detected"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("1 succeeded") && plain.contains("1 failed"),
        "summary should record step 1 success and step 2 failure; stderr: {plain}"
    );
}

// ============================================================================
// Post-shell-expansion schema validation in sequence (review-6 high finding)
// ============================================================================

#[cfg(unix)]
fn write_shell_whitelist(home: &std::path::Path, prefixes: &[&str]) {
    let body: String = prefixes.iter().map(|p| format!("prefix {p}\n")).collect();
    fs::write(home.join(".darkmatter-shell-whitelist"), body).unwrap();
}

#[cfg(unix)]
#[test]
fn sequence_shell_expanded_value_violating_schema_aborts_step() {
    // A sequence step whose post-shell effective frontmatter violates the
    // schema must surface a SchemaValidation error and not launch the
    // provider for that step.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_shell_whitelist(workspace.path(), &["echo"]);
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  tier: 'enum(small, medium, large; required)'
sequence:
  - alpha
tier: $(echo huge)
---
Step {{state}} with tier {{tier}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("schema validation"),
        "expected SchemaValidation surface for shell-expanded sequence step; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider launch should occur on post-shell invalid value"
    );
    // Silence unused linter for the predicates import that other tests use.
    let _ = contains("");
}

// ============================================================================
// Inline mode: a `prompt` frontmatter property switches each step from
// compose (body-as-prompt) to inline-compose (prompt-as-prompt + body
// write-back), mirroring the top-level `inline-compose` command.
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_with_prompt_property_runs_each_step_inline() {
    // When the source document declares a `prompt` frontmatter property,
    // every sequence step must run as an inline composition: the agent
    // prompt is the composed `prompt` (with per-step `{{state}}`
    // interpolation), NOT the document body, and the provider's output
    // replaces the body on disk. Regression target: the sequence
    // orchestrator previously hardcoded compose mode and sent the body.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let prompts_path = workspace.path().join("all-prompts.txt");
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        r#"---
prompt: |-
  Work on state {{state}}.
sequence:
  - alpha
  - beta
---
SENTINEL ORIGINAL BODY — must never be sent as the agent prompt.
"#,
    )
    .unwrap();

    // Goose stub: record the `-t <prompt>` argument (the agent prompt) for
    // every invocation, then emit a distinct replacement body on stdout so
    // the inline closure can rewrite the document.
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
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
printf 'Body produced by inline step %s\n' "$count"
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_PROMPTS_FILE", &prompts_path)
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    // Both steps ran.
    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "2", "both inline steps should run");

    let captured = fs::read_to_string(&prompts_path).unwrap();
    // The agent prompt is the composed `prompt` frontmatter, per step.
    assert!(
        captured.contains("Work on state alpha"),
        "step 1 agent prompt should be the composed `prompt` with state=alpha; captured:\n{captured}"
    );
    assert!(
        captured.contains("Work on state beta"),
        "step 2 agent prompt should be the composed `prompt` with state=beta; captured:\n{captured}"
    );
    // Core regression assertion: the body must NOT have been sent as the
    // agent prompt (that is the old compose behavior).
    assert!(
        !captured.contains("SENTINEL ORIGINAL BODY"),
        "the document body must never be sent as the agent prompt in inline mode; captured:\n{captured}"
    );

    // Inline closure rewrote the document body with the provider output and
    // preserved the original frontmatter (`prompt`, `sequence`). The last
    // write-back (step 2) wins on disk.
    let final_doc = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_doc.contains("Body produced by inline step 2"),
        "the document body should be replaced by the final step's output; doc:\n{final_doc}"
    );
    assert!(
        !final_doc.contains("SENTINEL ORIGINAL BODY"),
        "the original body should have been replaced; doc:\n{final_doc}"
    );
    assert!(
        final_doc.contains("prompt:") && final_doc.contains("sequence:"),
        "inline closure must preserve the original frontmatter; doc:\n{final_doc}"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("2 succeeded"),
        "summary should record both inline steps succeeding; stderr:\n{plain}"
    );
}

// ============================================================================
// Frontmatter `interactive: true` is hard-rejected for sequence
// (2026-06-14-interactive, review-1 medium finding)
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_rejects_interactive_true_frontmatter_via_cli() {
    // A sequence document that authors `interactive: true` must be rejected
    // up front with the sequence-specific diagnostic, before any provider
    // step is launched. This exercises the rendered CLI error surface (not
    // just the unit-level `reject_sequence_interactive`).
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\ninteractive: true\nsequence:\n  - alpha\n  - beta\n---\nStep {{state}}.\n",
    )
    .unwrap();

    // Provider stub records every invocation so the test can prove no step
    // ever launched.
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // The styled BlockError word-wraps the body, so tokens like
    // `--interactive` and `inline-compose` may break across lines at hyphens,
    // with the frame's `┃` border glyph reinserted at each wrapped line.
    // Strip whitespace and the border glyph before substring-matching so wrap
    // points don't defeat the assertion.
    let collapsed: String = plain
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '┃')
        .collect();
    // The rendered diagnostic must name `interactive: true`, point to the
    // compose/inline-compose commands, and mention the `--interactive`
    // single-run override.
    assert!(
        collapsed.contains("interactive:true"),
        "error should quote the rejected `interactive: true` key; stderr:\n{plain}"
    );
    assert!(
        collapsed.contains("inline-compose"),
        "error should point to compose / inline-compose for dialog prompts; stderr:\n{plain}"
    );
    assert!(
        collapsed.contains("--interactive"),
        "error should mention the --interactive single-run override; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider step should launch when interactive: true is rejected"
    );
}

#[cfg(unix)]
#[test]
fn sequence_rejects_non_string_prompt_property() {
    // A `prompt` frontmatter property that is present but not a string is
    // an inline-mode contract violation and must be rejected up front with
    // the same typed error `inline-compose` raises — before any step runs.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        "---\nprompt: 42\nsequence:\n  - alpha\n---\nBody\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("prompt") && plain.contains("number"),
        "expected a prompt wrong-type error naming the `number` type; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should launch when the prompt property is invalid"
    );
}
