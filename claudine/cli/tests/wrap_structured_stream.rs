#![cfg(unix)]

//! Integration tests: structured-stream reconstruction, verbosity control, and JSONL summary behavior.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{strip_ansi, write_executable};

#[cfg(unix)]
#[test]
fn codex_structured_mode_reconstructs_stdout_and_writes_summary_event() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
LAST=""
while [ $# -gt 0 ]; do
  case "$1" in
    --output-last-message)
      LAST="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
printf '%s\n' '{"type":"thread.started","thread_id":"thread-123"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.completed","item":{"id":"msg-1","type":"agent_message","text":"fallback from stream"}}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":12,"output_tokens":5},"duration_ms":1000,"status":"completed"}'
printf '%s' 'Final assistant response' > "$LAST"
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["codex", "--model", "codex-mini", "summarize repo"])
        .assert()
        .success()
        .stdout("Final assistant response\n");

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        stderr.contains("Codex") && stderr.contains("session ID") && stderr.contains("thread-123")
    );
    assert!(stderr.contains("codex-mini"));
    assert!(stderr.contains("✓ 1.0s"));

    let args = fs::read_to_string(&args_path).unwrap();
    assert!(args.lines().any(|line| line == "--json"));
    assert!(args.lines().any(|line| line == "--output-last-message"));

    let log_path = today_log_path(&fake_home);
    let log_contents = fs::read_to_string(log_path).unwrap();
    assert_eq!(
        log_contents
            .lines()
            .filter(|line| line.contains("\"synthetic_kind\":\"stream_wrapper_summary\""))
            .count(),
        1
    );
    assert!(log_contents.contains("\"provider_summary\":{\"raw_summary\""));
}

// ---------------------------------------------------------------------------
// Phase 5 — PID capture end-to-end verification
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn wrapper_structured_summary_includes_pids_in_jsonl() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"thread.started","thread_id":"thread-pid-test"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.completed","item":{"id":"msg-1","type":"agent_message","text":"pid test"}}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":12,"output_tokens":5},"duration_ms":1000,"status":"completed"}'
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["codex", "--model", "codex-mini", "test prompt"])
        .assert()
        .success();

    let log_path = today_log_path(&fake_home);
    let log_contents = fs::read_to_string(log_path).unwrap();

    let summary_line = log_contents
        .lines()
        .find(|line| line.contains("\"synthetic_kind\":\"stream_wrapper_summary\""))
        .expect("summary event should be written");

    let summary: serde_json::Value = serde_json::from_str(summary_line).unwrap();

    let claudine_pid = summary
        .get("env")
        .and_then(|e| e.get("claudine_pid"))
        .and_then(|v| v.as_u64());
    assert!(
        claudine_pid.is_some(),
        "summary event must include env.claudine_pid; got: {summary_line}"
    );

    let agent_pid = summary.get("agent_pid").and_then(|v| v.as_u64());
    assert!(
        agent_pid.is_some(),
        "summary event must include agent_pid after successful spawn; got: {summary_line}"
    );

    // Review-1 Finding 1: live per-event records (not just the lifecycle
    // summary) must carry agent_pid once the child has spawned. These are the
    // synthetic `stream_semantic_event` rows the sink logs for every event.
    let semantic_line = log_contents
        .lines()
        .find(|line| line.contains("\"synthetic_kind\":\"stream_semantic_event\""))
        .expect("at least one live semantic event should be logged");
    let semantic: serde_json::Value = serde_json::from_str(semantic_line).unwrap();
    assert!(
        semantic.get("agent_pid").and_then(|v| v.as_u64()).is_some(),
        "live semantic event must include agent_pid after spawn; got: {semantic_line}"
    );
    assert!(
        semantic
            .get("env")
            .and_then(|e| e.get("claudine_pid"))
            .and_then(|v| v.as_u64())
            .is_some(),
        "live semantic event must include env.claudine_pid; got: {semantic_line}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_dry_run_does_not_fabricate_agent_pid_in_jsonl() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
echo "SHOULD NOT RUN"
exit 1
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--dry-run", "--", "--version"])
        .assert()
        .success();

    let log_path = today_log_path(workspace.path());
    assert!(
        !log_path.exists(),
        "dry run must not create a JSONL log file; found: {log_path:?}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_missing_binary_does_not_fabricate_agent_pid_in_jsonl() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // Do NOT create a codex binary — binary resolution will fail.

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--", "--version"])
        .assert()
        .failure();

    let log_path = today_log_path(workspace.path());
    assert!(
        !log_path.exists(),
        "failed binary resolution must not create a JSONL log file; found: {log_path:?}"
    );
}

#[cfg(unix)]
#[test]
fn structured_verbosity_controls_stream_stderr_lines() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"init","session_id":"gem-1","model":"gemini-2.5-pro"}'
printf '%s\n' 'not-json'
printf '%s\n' '{"type":"message","role":"assistant","content":"Hello"}'
printf '%s\n' '{"type":"error","severity":"warning","message":"Loop detected"}'
printf '%s\n' '{"type":"result","status":"success","stats":{"total_tokens":30,"input_tokens":20,"output_tokens":10,"cached":5,"input":15,"duration_ms":1500,"tool_calls":0}}'
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "say hi"])
        .assert()
        .success()
        .stdout("Hello\n");
    let default_stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(default_stderr.contains("session ID gem-1"));
    assert!(!default_stderr.contains("Malformed JSON"));
    assert!(default_stderr.contains("Loop detected"));
    assert!(default_stderr.contains("\n\n✓ 1.5s"));
    assert!(default_stderr.contains("20 input tokens"));
    assert!(default_stderr.contains("10 output tokens"));
    assert!(default_stderr.contains("5 cached tokens"));
    assert!(default_stderr.contains("no tool calls"));

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--quiet", "say hi"])
        .assert()
        .success()
        .stdout("Hello\n");
    let quiet_stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // Agent session ID is always emitted regardless of --quiet/--silent
    assert!(quiet_stderr.contains("session ID gem-1"));
    assert!(!quiet_stderr.contains("Malformed JSON"));
    assert!(quiet_stderr.contains("Loop detected"));
    assert!(quiet_stderr.contains("\n\n✓ 1.5s"));
    assert!(quiet_stderr.contains("20 input tokens"));
    assert!(quiet_stderr.contains("10 output tokens"));
    assert!(quiet_stderr.contains("5 cached tokens"));
    assert!(quiet_stderr.contains("no tool calls"));

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--silent", "say hi"])
        .assert()
        .success()
        .stdout("Hello\n");
    let silent_stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // Agent session ID is always emitted regardless of --quiet/--silent
    assert!(silent_stderr.contains("session ID gem-1"));
    assert!(!silent_stderr.contains("Malformed JSON"));
    assert!(!silent_stderr.contains("Loop detected"));
    assert!(!silent_stderr.contains("✓ 1.5s"));
}

#[cfg(unix)]
#[test]
fn gemini_structured_success_suppresses_provider_stderr_noise() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
cat >&2 <<'EOF'
    at throwErrorIfNotOK (file:///tmp/fake.mjs:1:1)
    at process.processTicksAndRejections (node:internal/process/task_queues:105:5)
EOF
printf '%s\n' '{"type":"init","session_id":"gem-err","model":"gemini-2.5-pro"}'
printf '%s\n' '{"type":"message","role":"assistant","content":"Recovered answer"}'
printf '%s\n' '{"type":"result","status":"success","stats":{"total_tokens":30,"input_tokens":20,"output_tokens":10,"cached":5,"input":15,"duration_ms":1500,"tool_calls":0}}'
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--quiet", "say hi"])
        .assert()
        .success()
        .stdout("Recovered answer\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!stderr_plain.contains("throwErrorIfNotOK"));
    assert!(!stderr_plain.contains("processTicksAndRejections"));
    assert!(stderr_plain.contains("20 input tokens"));
    assert!(stderr_plain.contains("10 output tokens"));
    assert!(stderr_plain.contains("5 cached tokens"));
    assert!(stderr_plain.contains("no tool calls"));
}

#[cfg(unix)]
#[test]
fn structured_completion_summary_is_separated_on_stderr() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"init","session_id":"gem-2","model":"gemini-2.5-pro"}'
printf '%s\n' '{"type":"message","role":"assistant","content":"Hello without newline"}'
printf '%s\n' '{"type":"result","status":"success","stats":{"total_tokens":30,"input_tokens":20,"output_tokens":10,"cached":5,"input":15,"duration_ms":1500,"tool_calls":0}}'
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--quiet", "say hi"])
        .assert()
        .success()
        .stdout("Hello without newline\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr_plain.contains("\n\n✓ 1.5s"));
    assert!(stderr_plain.contains("20 input tokens"));
    assert!(stderr_plain.contains("10 output tokens"));
    assert!(stderr_plain.contains("5 cached tokens"));
    assert!(stderr_plain.contains("no tool calls"));
}

#[cfg(unix)]
#[test]
fn structured_verbose_summary_restores_rich_prose_fields() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"init","session_id":"gem-3","model":"gemini-2.5-pro"}'
printf '%s\n' '{"type":"message","role":"assistant","content":"Verbose summary"}'
printf '%s\n' '{"type":"tool_use","tool_name":"search","tool_id":"tool-1","parameters":{"query":"sky"}}'
printf '%s\n' '{"type":"tool_result","tool_id":"tool-1","status":"success","output":{"hits":1}}'
printf '%s\n' '{"type":"result","status":"success","cost_usd":0.02,"stats":{"total_tokens":63,"input_tokens":3,"output_tokens":60,"cached":11,"input":3,"duration_ms":4600,"tool_calls":1}}'
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "-v", "say hi"])
        .assert()
        .success()
        .stdout("Verbose summary\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr_plain.contains("4.6s"));
    assert!(stderr_plain.contains("3 input tokens"));
    assert!(stderr_plain.contains("60 output tokens"));
    assert!(stderr_plain.contains("11 cached tokens"));
    assert!(stderr_plain.contains("$0.02 cost basis"));
    assert!(stderr_plain.contains("1 tool call"));
    assert!(stderr_plain.contains("tools used: search"));
    assert!(stderr_plain.contains("model: gemini-2.5-pro"));
    assert!(stderr_plain.contains("stop reason: success"));
}

#[cfg(unix)]
#[test]
fn structured_quiet_verbose_uses_old_verbose_summary_renderer() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"claude-1","model":"claude-sonnet-4"}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"Quiet verbose summary"}]}}'
printf '%s\n' '{"type":"tool_use","name":"read_file","input":{"path":"sky.md"}}'
printf '%s\n' '{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":2,"duration_ms":4600,"usage":{"input_tokens":3,"output_tokens":60,"cache_read_input_tokens":11},"total_cost_usd":0.02}'
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["claude", "--quiet", "-v", "say hi"])
        .assert()
        .success()
        .stdout("Quiet verbose summary\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!stderr_plain.contains("claude session claude-1"));
    assert!(stderr_plain.contains("4.6s"));
    assert!(stderr_plain.contains("3 input tokens"));
    assert!(stderr_plain.contains("60 output tokens"));
    assert!(stderr_plain.contains("11 cached tokens"));
    assert!(stderr_plain.contains("$0.02 cost basis"));
    assert!(stderr_plain.contains("1 tool call"));
    assert!(stderr_plain.contains("tools used: read_file"));
    assert!(stderr_plain.contains("model: claude-sonnet-4"));
    assert!(stderr_plain.contains("turns: 2"));
    assert!(stderr_plain.contains("stop reason: end_turn"));
}

#[cfg(unix)]
#[test]
fn structured_verbose_summary_reports_no_tool_calls_when_absent() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"claude-2","model":"claude-sonnet-4"}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"No tools here"}],"role":"assistant"}}'
printf '%s\n' '{"type":"result","duration_ms":4600,"total_cost_usd":0.02,"usage":{"input_tokens":3,"output_tokens":60,"cache_read_input_tokens":11}}'
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["claude", "-v", "say hi"])
        .assert()
        .success()
        .stdout("No tools here\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr_plain.contains("no tool calls"));
}

// ===========================================================================
// Composition tests
// ===========================================================================

#[cfg(unix)]
#[test]
fn codex_structured_compose_filters_stdin_banner() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello Codex\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
last_message=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output-last-message" ]; then
    last_message="$arg"
  fi
  prev="$arg"
done

printf '%s\n' 'Reading prompt from stdin...' >&2
/bin/cat > /dev/null
if [ -n "$last_message" ]; then
  printf '%s\n' 'Recovered answer' > "$last_message"
fi
printf '%s\n' '{"type":"thread.started","thread_id":"codex-1"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":20,"output_tokens":10,"cached_input_tokens":5}}'
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["compose", "--codex", "--quiet", md_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout("Recovered answer\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!stderr_plain.contains("Reading prompt from stdin..."));
}

#[cfg(unix)]
#[test]
fn codex_structured_compose_surfaces_live_tool_progress() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello Codex\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
last_message=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output-last-message" ]; then
    last_message="$arg"
  fi
  prev="$arg"
done

/bin/cat > /dev/null
printf '%s\n' '{"type":"thread.started","thread_id":"codex-1"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.started","item":{"id":"t1","type":"command_exec","tool_name":"shell","input":{"cmd":"git status"}}}'
printf '%s\n' '{"type":"item.completed","item":{"id":"t1","type":"command_exec","tool_name":"shell","output":"ok"}}'
printf '%s\n' '{"type":"item.started","item":{"id":"t2","type":"view_image","tool_name":"view_image"}}'
printf '%s\n' '{"type":"item.completed","item":{"id":"t2","type":"view_image","output":"ok"}}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":20,"output_tokens":10,"cached_input_tokens":5}}'
if [ -n "$last_message" ]; then
  printf '%s\n' 'Recovered answer' > "$last_message"
fi
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["compose", "--codex", "--quiet", md_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout("Recovered answer\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // The sink now renders the first non-empty string value as the tool
    // input preview instead of a truncated JSON blob (Plan 3 hardening).
    // Post response-refinement humanization (Task 1.2), tool names are
    // title-cased (`shell` → `Shell`, `view_image` → `View Image`).
    // Per the 2026-04-16 more-is-more Phase 1 change, tool call lines
    // render as `Name(summary)` instead of `Name · summary`.
    assert!(
        stderr_plain.contains("Shell(git status)"),
        "missing Shell call with 'git status' preview in parens in:\n{stderr_plain}"
    );
    assert!(
        stderr_plain.contains("View Image"),
        "missing View Image line in:\n{stderr_plain}"
    );
}
