//! Integration tests: provider-specific wrapper flag handling (Gemini/Goose/Kimi/Qwen yolo and non-interactive translation).
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{write_executable};

#[cfg(unix)]
#[test]
fn gemini_wrapper_applies_yolo_as_approval_mode() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["gemini", "--yolo", "--", "-p", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(args.contains(&"--approval-mode"));
    assert!(args.contains(&"yolo"));
}

// ---------------------------------------------------------------------------
// Goose wrapper (review 6.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn goose_wrapper_yolo_sets_env_var() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf 'GOOSE_MODE=%s\n' "$GOOSE_MODE" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["goose", "--yolo", "summarize"])
        .assert()
        .success();

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("GOOSE_MODE=auto"));
}

#[cfg(unix)]
#[test]
fn goose_wrapper_non_interactive_prepends_run() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["goose", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(args.first(), Some(&"run"));
    assert!(args.contains(&"summarize"));
}

// ---------------------------------------------------------------------------
// Kimi wrapper (review 6.2)
// ---------------------------------------------------------------------------

/// Non-interactive Kimi runs use `--wire` JSON-RPC mode.
///
/// Phase 4 of the Kimi fix retired Kimi's legacy `--print` /
/// `--output-format stream-json` path: structured output now flows over
/// the JSON-RPC wire protocol on stdin and stdout. The wrapper appends
/// `--wire` instead of `--print`, and the prompt is delivered as a typed
/// `prompt` request rather than seeded on stdin.
#[cfg(unix)]
#[test]
fn kimi_wrapper_non_interactive_appends_wire() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");
    let stdin_path = workspace.path().join("stdin.txt");

    // Stub kimi: capture argv to one file and the first two stdin lines
    // (the JSON-RPC `initialize` and `prompt` requests) into a separate
    // file so the test can assert on the prompt envelope. The stub
    // produces a minimal initialize response between reads so the
    // semantic parser advances past handshake, then emits a final
    // `prompt` response so claudine's wait loop can shut down cleanly.
    write_executable(
        &path_dir.join("kimi"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
: > "$CLAUDINE_STDIN_FILE"
read INIT_LINE
printf '%s\n' "$INIT_LINE" >> "$CLAUDINE_STDIN_FILE"
printf '%s\n' '{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.9","server":{"name":"kimi","version":"1.38.0"},"capabilities":{}}}'
read PROMPT_LINE
printf '%s\n' "$PROMPT_LINE" >> "$CLAUDINE_STDIN_FILE"
printf '%s\n' '{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}'
printf '%s\n' '{"jsonrpc":"2.0","id":"prompt-2","result":{"status":"finished"}}'
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .args(["kimi", "hi"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(
        args.contains(&"--wire"),
        "kimi non-interactive run must append --wire; got args: {args:?}"
    );
    assert!(
        !args.contains(&"--print"),
        "kimi non-interactive run must not append --print; got args: {args:?}"
    );
    assert!(
        !args.contains(&"--output-format"),
        "kimi non-interactive run must not pass --output-format; got args: {args:?}"
    );

    // Wire-mode delivers the prompt as a JSON-RPC `prompt` request, not
    // via stdin. Verify the prompt envelope reached the child instead of
    // a bare `hi` stdin seed.
    let stdin = fs::read_to_string(&stdin_path).unwrap();
    assert!(
        stdin.contains("\"method\":\"prompt\""),
        "kimi wire mode must send a JSON-RPC prompt method on stdin; got stdin: {stdin}"
    );
    assert!(
        stdin.contains("\"user_input\":\"hi\""),
        "kimi wire mode must carry the prompt as params.user_input; got stdin: {stdin}"
    );
}

// ---------------------------------------------------------------------------
// Qwen wrapper (review 6.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn qwen_wrapper_rejects_direct_approval_mode_yolo() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("qwen"),
        r#"#!/bin/sh
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["qwen", "--approval-mode", "yolo", "--", "-p", "hi"])
        .assert()
        .code(1)
        .stderr(contains("do not pass"))
        .stderr(contains("--approval-mode yolo"))
        .stderr(contains("--yolo"));
}
