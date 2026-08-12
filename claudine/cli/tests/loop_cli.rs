#![cfg(unix)]

//! Integration tests for `claudine compose` and `claudine inline-compose` with
//! looping frontmatter.
//!
//! Exercises the loop engine end-to-end through the CLI: iteration counting,
//! max-iterations overrides, fail-fast semantics, and FAIL_FAST deprecation.

use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

// ============================================================================
// Basic loop execution
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_loop_runs_iterations() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "counter < 2"
  actions:
    - "increment(counter)"
---
Count is {{counter}}
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
cat > /dev/null
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "3", "loop should run 3 iterations");
}

#[cfg(unix)]
#[test]
fn inline_compose_loop_runs_iterations() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
prompt: "Generate a number"
loop:
  while: "counter < 1"
  actions:
    - "increment(counter)"
---
Body
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
cat > /dev/null
printf 'Generated body %s' "$count"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "2", "inline loop should run 2 iterations");
}

/// Regression: an `inline-compose` document whose prompt lives in the
/// `prompt:` frontmatter key and whose body is empty must still seed and run.
/// Before the seed path was parameterized by composition mode, seeding called
/// `prepare_direct`, which composes the (empty) body and failed with
/// `ComposedBodyEmpty` before iteration 1 — even though the iteration
/// executor composes the `prompt:` frontmatter value.
#[cfg(unix)]
#[test]
fn inline_compose_loop_with_prompt_frontmatter_and_empty_body_runs() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
prompt: "Generate a number"
loop:
  while: "counter < 1"
  actions:
    - "increment(counter)"
---
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
cat > /dev/null
printf 'Generated body %s' "$count"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "2",
        "inline loop with prompt frontmatter and empty body should run 2 iterations"
    );
}

// ============================================================================
// max-iterations override
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_max_iterations_flag_overrides_frontmatter() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "counter < 100"
  actions:
    - "increment(counter)"
---
Count is {{counter}}
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
cat > /dev/null
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            "--max-iterations",
            "2",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "2", "--max-iterations should cap at 2");
}

#[cfg(unix)]
#[test]
fn compose_max_iterations_env_overrides_default() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "counter < 100"
  actions:
    - "increment(counter)"
---
Count is {{counter}}
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
cat > /dev/null
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .env("CLAUDINE_MAX_ITERATIONS", "2")
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "2", "CLAUDINE_MAX_ITERATIONS should cap at 2");
}

// ============================================================================
// FAIL_FAST deprecation
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_fail_fast_deprecated_env_emits_warning() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "counter < 3"
  actions:
    - "increment(counter)"
---
Count is {{counter}}
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
cat > /dev/null
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .env("FAIL_FAST", "true")
        .env_remove("CLAUDINE_FAIL_FAST")
        .env("RUST_LOG", "warn")
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("deprecated")
            || plain.contains("FAIL_FAST")
            || plain.contains("CLAUDINE_FAIL_FAST"),
        "should emit deprecation warning for FAIL_FAST; stderr: {plain}"
    );
}

// ============================================================================
// Sequence CLAUDINE_FAIL_FAST propagation
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_uses_claudine_fail_fast_env() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");
    let env_path = workspace.path().join("child-env.txt");

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

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
printf 'CLAUDINE_FAIL_FAST=%s\n' "$CLAUDINE_FAIL_FAST" > "$CLAUDINE_ENV_FILE"
cat > /dev/null
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .env_remove("FAIL_FAST")
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
}

// ============================================================================
// Loop with failing iteration and fail_fast=false
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_loop_fail_fast_false_continues_after_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "counter < 2"
  actions:
    - "increment(counter)"
  fail_fast: false
---
Count is {{counter}}
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
cat > /dev/null
if [ "$count" = "1" ]; then
  exit 5
fi
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "3",
        "fail_fast=false should continue after iteration 1 fails (3 total: 1 fail, 2 ok, 3 ok)"
    );
}

// ============================================================================
// Loop with failing iteration and fail_fast=true
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_loop_fail_fast_true_stops_on_first_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "counter < 3"
  actions:
    - "increment(counter)"
  fail_fast: true
---
Count is {{counter}}
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
cat > /dev/null
exit 7
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "1",
        "fail_fast=true should stop after first failing iteration"
    );
}

// ============================================================================
// Loop cap exceeded
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_loop_max_iterations_cap_exceeded() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "true"
  actions:
    - "increment(counter)"
  max: 2
---
Count is {{counter}}
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
cat > /dev/null
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "2",
        "loop should run exactly 2 iterations before cap exceeded"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("limit exceeded") || plain.contains("LoopLimitExceeded"),
        "should report loop limit exceeded; stderr: {plain}"
    );
}

// ============================================================================
// Honest error classification (LoopIterationFailed, not LoopInvalid)
// ============================================================================

/// Regression for the "invalid loop definition: provider exited with code 1"
/// mislabeling. When a loop iteration's child is killed by `step_timeout`,
/// the top-level error must surface `step_timeout` — never the
/// `LoopInvalid` overload that used to wrap all non-zero child exits.
#[cfg(unix)]
#[test]
fn compose_loop_step_timeout_surfaces_as_iteration_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  until: "done"
  actions:
    - "increment(counter)"
  max: 3
---
Iteration {{counter}}
"#,
    )
    .unwrap();

    // Fake opencode that emits a session header (so the structured stream
    // engages) and then hangs silently — triggers the step_timeout
    // watchdog kill on the very first iteration.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"loop-timeout","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"loop-timeout"}'
while :; do /bin/sleep 1; done
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "1s")
        .env("CLAUDINE_KILL_GRACE", "1s")
        .current_dir(workspace.path())
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // The watchdog detail message must still appear as before.
    assert!(
        plain.contains("no stream activity"),
        "stderr must carry the watchdog detail; got: {plain}"
    );

    // The composition-level error must NOT be the legacy `LoopInvalid`
    // overload that reused the "invalid loop definition" phrase for
    // runtime failures.
    assert!(
        !plain.contains("invalid loop definition"),
        "step_timeout must not be mislabeled as `invalid loop definition`; got: {plain}"
    );

    // It MUST surface as a loop iteration failure with the structured
    // exit_reason in the header.
    assert!(
        plain.contains("loop iteration") || plain.contains("step_timeout"),
        "stderr must surface the iteration / step_timeout cause; got: {plain}"
    );
}

// ============================================================================
// Rate-limit-aware iteration policy
// ============================================================================

/// `--on-rate-limit abort` halts the loop with exit code 75 (EX_TEMPFAIL)
/// when an iteration emits a rate-limit trailer, even if that iteration's
/// own exit code was 0.
#[cfg(unix)]
#[test]
fn compose_loop_rate_limit_abort_exits_75() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "true"
  actions:
    - "increment(counter)"
  max: 3
---
Iteration {{counter}}
"#,
    )
    .unwrap();

    // Fake opencode that emits a rate-limit stderr trailer with a far-future
    // reset clock, then exits 0. The stderr classifier converts this into
    // `summary.rate_limit.is_throttled = true` for the iteration. With
    // `--on-rate-limit abort`, the loop must halt and return exit code 75.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"loop-rate","model":"test-model"}'
printf '%s\n' '{"type":"finish","sessionID":"loop-rate"}'
printf '%s\n' 'ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2099-01-01 00:00:00\"}}"}]}}' >&2
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args([
            "compose",
            "--opencode",
            "--on-rate-limit",
            "abort",
            md_file.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    let exit_code = assert.get_output().status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        75,
        "rate-limit abort must exit with EX_TEMPFAIL (75); got {exit_code}, stderr: {}",
        String::from_utf8_lossy(&assert.get_output().stderr)
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("rate limited") || plain.contains("rate_limit"),
        "stderr must surface the rate-limit halt; got: {plain}"
    );
    assert!(
        !plain.contains("invalid loop definition"),
        "rate-limit halt must not be mislabeled; got: {plain}"
    );
}

/// Sibling to `compose_loop_rate_limit_abort_exits_75`, exercising the same
/// rate-limit abort contract on a document with minimal harness frontmatter.
/// The always-harness unification routes parsed-harness documents through
/// Rate-limit abort on a loop document.
///
/// Prior to the lifecycle refactor this test used `post_checks: []` to force
/// the parsed-harness plan path; that key is now rejected, so the loop itself
/// drives the iteration path.
#[cfg(unix)]
#[test]
fn compose_loop_rate_limit_abort_exits_75_on_loop_doc() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "true"
  actions:
    - "increment(counter)"
  max: 3
---
Iteration {{counter}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"loop-rate","model":"test-model"}'
printf '%s\n' '{"type":"finish","sessionID":"loop-rate"}'
printf '%s\n' 'ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2099-01-01 00:00:00\"}}"}]}}' >&2
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args([
            "compose",
            "--opencode",
            "--on-rate-limit",
            "abort",
            md_file.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    let exit_code = assert.get_output().status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        75,
        "parsed-harness rate-limit abort must exit with EX_TEMPFAIL (75); got {exit_code}, stderr: {}",
        String::from_utf8_lossy(&assert.get_output().stderr)
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("rate limited") || plain.contains("rate_limit"),
        "stderr must surface the rate-limit halt; got: {plain}"
    );
    assert!(
        !plain.contains("invalid loop definition"),
        "rate-limit halt must not be mislabeled; got: {plain}"
    );
}

/// `--on-rate-limit pause` waits out a usable reset clock and then runs
/// the next iteration. A reset_at ~3s in the future should produce a
/// noticeable but bounded delay (no unbounded blocking) before iteration 2
/// runs.
///
/// Marked `serial` because the ~8s pause makes the test sensitive to
/// shared-test-runner contention; isolating it stabilizes the timing
/// assertions.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_loop_rate_limit_pause_waits_then_continues() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "counter < 1"
  actions:
    - "increment(counter)"
---
Iteration {{counter}}
"#,
    )
    .unwrap();

    // First call: emit a 429 with a reset clock ~3s in the future. Second
    // call: just succeed. The engine should pause between iterations and
    // both calls should land. `CLAUDINE_PAUSE_RESET_MARGIN` (set below)
    // trims the production 5s safety margin to 1s so the test exercises the
    // real pause-then-continue path without the full wall-clock wait.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi

count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"

printf '%s\n' '{"type":"init","session_id":"loop-pause","model":"test-model"}'
printf '%s\n' '{"type":"finish","sessionID":"loop-pause"}'

if [ "$count" = "1" ]; then
  # Reset ~3s in the future: comfortably ahead of the tail of iteration 1
  # (stream parse + teardown, well under 1s) so the engine still sees a
  # future reset, but short enough to keep the test fast.
  reset_at=$(/bin/date -u -v+3S '+%Y-%m-%d %H:%M:%S' 2>/dev/null || /bin/date -u -d '+3 seconds' '+%Y-%m-%d %H:%M:%S')
  printf '%s\n' "ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={\"error\":{\"name\":\"AI_RetryError\",\"reason\":\"maxRetriesExceeded\",\"errors\":[{\"name\":\"AI_APICallError\",\"statusCode\":429,\"responseBody\":\"{\\\"error\\\":{\\\"code\\\":\\\"1308\\\",\\\"message\\\":\\\"Usage limit reached. Your limit will reset at $reset_at\\\"}}\"}]}}" >&2
fi
exit 0
"#,
    );

    let start = std::time::Instant::now();
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .env("CLAUDINE_PAUSE_RESET_MARGIN", "1s")
        .current_dir(workspace.path())
        .args([
            "compose",
            "--opencode",
            "--on-rate-limit",
            "pause",
            md_file.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
    let elapsed = start.elapsed();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "2",
        "pause policy must let the second iteration run; got {} calls",
        calls.trim()
    );
    // 3s reset + 1s margin ≈ 4s pause. The floor must stay clearly above the
    // ~2s no-pause path (two back-to-back subprocess spawns) so a regression
    // that skips the wait is still caught.
    assert!(
        elapsed >= std::time::Duration::from_secs(3),
        "pause should produce a noticeable delay; elapsed = {elapsed:?}"
    );
}

/// Default policy is `pause` — but if `reset_at` is absent or already past,
/// the engine falls back to `abort` to avoid an unbounded sleep. The exit
/// code must still be 75.
#[cfg(unix)]
#[test]
fn compose_loop_rate_limit_pause_without_reset_aborts() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "true"
  actions:
    - "increment(counter)"
  max: 3
---
Iteration {{counter}}
"#,
    )
    .unwrap();

    // Rate-limit trailer with no reset_at — the upstream parser will see
    // `is_throttled = true` but no usable wait window. Default `pause`
    // policy must fall through to abort.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"loop-no-reset","model":"test-model"}'
printf '%s\n' '{"type":"finish","sessionID":"loop-no-reset"}'
printf '%s\n' 'ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached\"}}"}]}}' >&2
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure();

    let exit_code = assert.get_output().status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        75,
        "no reset_at must still abort cleanly with EX_TEMPFAIL (75); got {exit_code}, stderr: {}",
        String::from_utf8_lossy(&assert.get_output().stderr)
    );
}

// ============================================================================
// CWD preservation across loop iterations
// ============================================================================

/// Regression: a `file(required)` setter that resolves against the user's
/// launch CWD must keep validating across every iteration, not just the
/// first. The wrap layer's `switch_process_cwd` mutates the process-global
/// CWD to the detected repo/git root inside each iteration; without
/// restoring the launch CWD before the next iteration's
/// `prepare_direct_with_schema`, a relative `plan=...` setter that
/// validated on iteration 1 fails iteration 2 as `not a "darkmatter-file"`
/// even though the in-memory state is identical.
#[cfg(unix)]
#[test]
fn compose_loop_file_required_setter_revalidates_against_launch_cwd_each_iteration() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // Create a git repo at the workspace so the wrap layer's
    // `detect_repo_root` resolves to a directory ABOVE the launch CWD.
    // Then run from a subdirectory of the repo where the relative
    // `plan=...` path resolves. Without the CWD-restore fix, iteration 2
    // would resolve `plan` against the repo root instead of the subdir
    // and validation would fail.
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(workspace.path())
        .status()
        .unwrap();

    let subdir = workspace.path().join("package");
    fs::create_dir_all(&subdir).unwrap();
    let plan_dir = subdir.join("features");
    fs::create_dir_all(&plan_dir).unwrap();
    let plan_file = plan_dir.join("plan.md");
    fs::write(&plan_file, "# plan\n").unwrap();

    // Prompt is at the repo root (not the subdir) so we point at it via
    // `../` from the subdir, matching the real-world layout where the
    // user runs from a package area against a repo-root prompts file.
    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  phase: 'number(required)'
  total_phases: 'number(required)'
  plan: 'file(required)'
phase: 1
total_phases: 0
loop:
  until: "phase >= total_phases"
  action: "increment(phase)"
---
Phase {{phase}} of {{total_phases}}.
"#,
    )
    .unwrap();

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
cat > /dev/null
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(&subdir)
        .args([
            "compose",
            "--goose",
            "../loop.md",
            "plan=features/plan.md",
            "total_phases=3",
        ])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap_or_default();
    assert_eq!(
        calls.trim(),
        "3",
        "all three iterations must run; loop bailing on iteration 2 \
         with a schema validation error indicates the launch CWD was \
         not restored between iterations. stderr:\n{}",
        strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr))
    );
}

// ============================================================================
// Phase 1: loop iteration signals from structured stream
// ============================================================================

/// Regression safety net for the always-harness unification work. A loop
/// whose iteration emits a structured `exit_reason` must surface that reason
/// in `LoopIterationFailed` (honest error cause). This variant has no harness
/// frontmatter, so it exercises the current non-harness path and should pass
/// today.
#[cfg(unix)]
#[test]
fn compose_loop_exit_reason_surfaces_on_non_harness_doc() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "true"
  max: 1
---
Iteration {{counter}}
"#,
    )
    .unwrap();

    // Fake opencode emits an error event with a structured error_kind and
    // exits non-zero. The loop must surface the exit_reason honestly instead
    // of mislabeling it as an invalid loop definition.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"loop-exit","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"loop-exit"}'
printf '%s\n' '{"type":"error","error_type":"api_timeout","error_message":"upstream timeout"}'
exit 1
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Iteration 1 exited with code 1"),
        "stderr must surface the loop iteration failure; got: {plain}"
    );
    assert!(
        plain.contains("api_timeout"),
        "stderr must carry the structured exit_reason; got: {plain}"
    );
    assert!(
        !plain.contains("invalid loop definition"),
        "honest exit_reason must not be mislabeled as invalid loop definition; got: {plain}"
    );
}

/// Sibling to the above test, exercising the same signal contract on a
/// document with minimal harness frontmatter. Phase 2 of the always-harness
/// plan surfaces terminal-attempt signals from `run_loop`, so this
/// test should now pass without the retired harness frontmatter key.
#[cfg(unix)]
#[test]
fn compose_loop_exit_reason_surfaces_on_loop_doc() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        r#"---
loop:
  while: "true"
  max: 1
---
Iteration {{counter}}
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"loop-exit","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"loop-exit"}'
printf '%s\n' '{"type":"error","error_type":"api_timeout","error_message":"upstream timeout"}'
exit 1
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Iteration 1 exited with code 1"),
        "stderr must surface the loop iteration failure; got: {plain}"
    );
    assert!(
        plain.contains("api_timeout"),
        "stderr must carry the structured exit_reason; got: {plain}"
    );
    assert!(
        !plain.contains("invalid loop definition"),
        "honest exit_reason must not be mislabeled as invalid loop definition; got: {plain}"
    );
}
