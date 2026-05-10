//! Integration tests for `claudine compose` and `claudine inline-compose` with
//! looping frontmatter.
//!
//! Exercises the loop engine end-to-end through the CLI: iteration counting,
//! max-iterations overrides, fail-fast semantics, and FAIL_FAST deprecation.

use assert_cmd::cargo::cargo_bin_cmd;
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

    cargo_bin_cmd!("claudine")
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
  while: "counter < 2"
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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .env("FAIL_FAST", "true")
        .env_remove("CLAUDINE_FAIL_FAST")
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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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

    let assert = cargo_bin_cmd!("claudine")
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
