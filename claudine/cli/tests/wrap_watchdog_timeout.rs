#![cfg(unix)]

//! Integration tests: watchdog / timeout termination behavior across stream-idle, wall-clock, and non-harness CLI timeouts.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
use std::time::Duration;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{strip_ansi, write_executable};

/// Replay the reference hang shape: 9 task_started, 7 task_completed, then
/// silence. With a low subagent-idle threshold the watchdog should terminate
/// the run and name the 2 stuck subagents in stderr.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn watchdog_subagent_hang_terminates_and_names_stuck_ids() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: watchdog test\n---\nHello\n").unwrap();

    // Build a shell script that emits 9 task_started, 7 task_completed, then blocks.
    let mut script = String::from(
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"hang-test","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"hang-test"}'
printf '%s\n' '{"type":"step_finish","sessionID":"hang-test","part":{"reason":"tool-calls","tokens":{"input":1,"output":1,"total":2}}}'
"#,
    );
    for i in 1..=9 {
        script.push_str(&format!(
            r#"printf '%s\n' '{{"type":"task_started","task_id":"sa{i}","name":"Task {i}"}}'
"#,
        ));
    }
    for i in 1..=7 {
        script.push_str(&format!(
            r#"printf '%s\n' '{{"type":"task_completed","task_id":"sa{i}","name":"Task {i}","status":"success"}}'
"#,
        ));
    }
    script.push_str("while :; do /bin/sleep 1; done\n");

    write_executable(&path_dir.join("opencode"), &script);

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "1s")
        .env("CLAUDINE_KILL_GRACE", "1s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // The Agent Error block from the watchdog should report a step_timeout
    // and name the stuck subagents in the diagnostic block.
    assert!(
        plain.contains("no stream activity"),
        "stderr should contain step_timeout breach message; got: {plain}"
    );
    assert!(
        plain.contains("2 subagents were still outstanding"),
        "stderr should enumerate stuck subagent count; got: {plain}"
    );
    assert!(
        plain.contains("sa8 \"Task 8\""),
        "stderr should name stuck subagent 8 with id and name; got: {plain}"
    );
    assert!(
        plain.contains("sa9 \"Task 9\""),
        "stderr should name stuck subagent 9 with id and name; got: {plain}"
    );

    // Assert the JSONL summary field.
    let log_path = today_log_path(workspace.path());
    if log_path.exists() {
        let log = fs::read_to_string(&log_path).unwrap();
        let last = log.lines().last().unwrap();
        let entry: serde_json::Value = serde_json::from_str(last).unwrap();
        assert_eq!(
            entry
                .get("extra")
                .and_then(|e| e.get("exit_reason"))
                .and_then(|v| v.as_str()),
            Some("step_timeout"),
            "JSONL session_end must have extra.exit_reason=step_timeout; last entry: {last}"
        );
    }
}

/// Stream-idle watchdog fires when no subagents are outstanding and the
/// provider stream goes completely silent after a tool call.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn watchdog_stream_idle_timeout_after_tool_call_hang() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: idle test\n---\nHello\n").unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"idle-test","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"idle-test"}'
printf '%s\n' '{"type":"step_finish","sessionID":"idle-test","part":{"reason":"tool-calls","tokens":{"input":1,"output":1,"total":2}}}'
printf '%s\n' '{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls"}}}'
while :; do /bin/sleep 1; done
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "1s")
        .env("CLAUDINE_KILL_GRACE", "1s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("no stream activity"),
        "stderr should contain step_timeout breach message; got: {plain}"
    );
}

/// Wall-clock watchdog fires when the run exceeds the configured `timeout`
/// even if the parent stream keeps emitting events. Asserts termination
/// with exit reason `timeout` (rendered breach message contains
/// "wall-clock") and that the synthesised summary trailer reflects the
/// wall-clock kill.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn watchdog_wall_clock_timeout_terminates_active_stream() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: wall-clock test\n---\nHello\n").unwrap();

    // Fake provider keeps emitting events forever (~10/sec) so the parent
    // stream is never silent — only the wall-clock budget can stop it.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"wall-clock-test","model":"test-model"}'
i=0
while :; do
  printf '%s\n' "{\"type\":\"task_started\",\"task_id\":\"sa$i\",\"name\":\"Task $i\"}"
  printf '%s\n' "{\"type\":\"task_completed\",\"task_id\":\"sa$i\",\"name\":\"Task $i\",\"status\":\"success\"}"
  i=$((i + 1))
  /bin/sleep 0.1
done
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_TIMEOUT", "2s")
        // Disable step_timeout so only the wall-clock rule can fire.
        .env("CLAUDINE_STEP_TIMEOUT", "0s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "1s")
        .env("CLAUDINE_KILL_GRACE", "1s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // The Agent Error block from the watchdog should report the
    // wall-clock breach. The breach message format is
    // "wall-clock budget exceeded after <duration>".
    assert!(
        plain.contains("wall-clock"),
        "stderr should mention the wall-clock budget; got: {plain}"
    );
    // It must NOT report a step_timeout breach since the silence rule was
    // disabled.
    assert!(
        !plain.contains("step_timeout"),
        "stderr should not mention step_timeout when only wall-clock fires; got: {plain}"
    );

    // Assert the JSONL summary field.
    let log_path = today_log_path(workspace.path());
    if log_path.exists() {
        let log = fs::read_to_string(&log_path).unwrap();
        let last = log.lines().last().unwrap();
        let entry: serde_json::Value = serde_json::from_str(last).unwrap();
        assert_eq!(
            entry
                .get("extra")
                .and_then(|e| e.get("exit_reason"))
                .and_then(|v| v.as_str()),
            Some("timeout"),
            "JSONL session_end must have extra.exit_reason=timeout; last entry: {last}"
        );
    }
}

/// Phase 1 reproduction for the 2026-05-10 OpenCode timeout regression.
///
/// This test replays the OpenCode stream shape that triggers the false-hang
/// regression described in
/// `claudine/fixes/2026-05-10-opencode-timeout-regression/plan.md`:
///
/// - `step_start` then a sequence of `text` events (parent prose).
/// - Several `tool_use` (post-completion) events with **no** matching
///   `tool_start` — OpenCode does not emit `tool_start` and does not emit
///   `task_started` for its `task` subagent tool, so the wrapper's in-flight
///   tracking is never populated.
/// - A final parent-text event (the model's closing prose).
/// - Silence shorter than `step_timeout`, after which the fake OpenCode
///   binary exits 0 cleanly.
///
/// Under the regression the wrapper misclassified even *short* legitimate
/// silence as a hang because in-flight tracking was empty. Under the fix
/// (byte heartbeat in Phase 2 + OpenCode per-step grace in Phase 3) the
/// wrapper waits for the child to exit cleanly and the run succeeds.
///
/// **Note (2026-05-11):** the silence window here is intentionally bounded
/// below `step_timeout`. The per-step grace was tightened so that mid-step
/// silence with BOTH the event clock and byte clock stale beyond the budget
/// *does* fire — that case is now the
/// `opencode_step_started_but_never_finished_fires_on_stale_clocks` unit
/// test in [`watchdog.rs`](../src/commands/wrap/exec/watchdog.rs).
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn watchdog_opencode_post_fanout_silence_does_not_kill_prematurely() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: opencode regression repro\n---\nHello\n",
    )
    .unwrap();

    // Fake OpenCode that mirrors the real stream shape from the user's
    // failing transcript:
    //   1. init + step_start
    //   2. assistant text describing the plan
    //   3. several tool_use (post-completion) events with no tool_start
    //   4. a final closing text ("Now running sniff repo:")
    //   5. silence longer than step_timeout
    //   6. clean exit 0
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"oc-regress","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"oc-regress"}'
printf '%s\n' '{"type":"text","text":"Planning commit work"}'
printf '%s\n' '{"type":"tool_use","part":{"id":"t1","tool":"bash","state":{"status":"completed","input":{"command":"git status"},"output":"clean"}}}'
printf '%s\n' '{"type":"tool_use","part":{"id":"t2","tool":"read","state":{"status":"completed","input":{"path":"README.md"},"output":"..."}}}'
printf '%s\n' '{"type":"tool_use","part":{"id":"t3","tool":"bash","state":{"status":"completed","input":{"command":"just lint"},"output":"ok"}}}'
printf '%s\n' '{"type":"text","text":"Now running sniff repo to show the final state:"}'
/bin/sleep 1
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("OPENCODE_MODEL", "test-model")
        // step_timeout must be longer than the post-text silence above (1s)
        // so the grace can apply and the child exits cleanly before the
        // budget elapses. Use a generous 15s budget — the test intent is
        // unchanged (silence < step_timeout), but the wide margin prevents
        // spurious watchdog fires under heavy parallel-test contention,
        // where wrapper startup + watchdog tick scheduling can stretch the
        // wall clock past a tight 3s budget. A real silence regression
        // would still fail because the byte heartbeat never lands.
        .env("CLAUDINE_STEP_TIMEOUT", "15s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "2s")
        .env("CLAUDINE_KILL_GRACE", "1s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(30))
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // Under the fix, no step_timeout breach should be rendered.
    assert!(
        !plain.contains("no stream activity"),
        "stderr must NOT contain step_timeout breach message under the fix; got: {plain}"
    );
    assert!(
        !plain.contains("Agent Error"),
        "stderr must NOT contain Agent Error block under the fix; got: {plain}"
    );

    // The synthesised JSONL summary must NOT report a step_timeout exit
    // reason — the child exited cleanly, so any exit_reason should reflect
    // a normal completion (or be absent).
    let log_path = today_log_path(workspace.path());
    if log_path.exists() {
        let log = fs::read_to_string(&log_path).unwrap();
        if let Some(last) = log.lines().last() {
            let entry: serde_json::Value = serde_json::from_str(last).unwrap();
            let exit_reason = entry
                .get("extra")
                .and_then(|e| e.get("exit_reason"))
                .and_then(|v| v.as_str());
            assert_ne!(
                exit_reason,
                Some("step_timeout"),
                "JSONL session_end must NOT have extra.exit_reason=step_timeout under the fix; last entry: {last}"
            );
        }
    }
}

// Removed 2026-05-12 (fix `2026-05-12-opencode-stderr-returns` Phase 4):
// the `watchdog_opencode_task_synthesis_populates_recent_subagents_in_breach`
// integration test specifically validated the now-removed stdout NDJSON
// synthesis of `SubagentStart`/`SubagentStop` from `task` tool completions.
// Phase 4 makes the stderr log bridge the authoritative source for
// OpenCode subagent lifecycle, so the synthesis path no longer exists and
// the recent-subagents listing must instead be exercised from the
// structured stderr fixture introduced in Phase 6.

/// Non-harness compose respects --timeout CLI flag (duration grammar).
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_non_harness_respects_cli_timeout() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: cli timeout test\n---\nHello\n").unwrap();

    // Fake provider emits events forever so only wall-clock can stop it.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"cli-timeout-test","model":"test-model"}'
while :; do
  printf '%s\n' '{"type":"thinking","text":"working..."}'
  /bin/sleep 0.1
done
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "0s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "1s")
        .env("CLAUDINE_KILL_GRACE", "1s")
        .args([
            "compose",
            "--opencode",
            "--timeout",
            "2s",
            md_file.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("wall-clock"),
        "stderr should mention wall-clock budget from CLI --timeout; got: {plain}"
    );
    assert!(
        !plain.contains("step_timeout"),
        "stderr should not mention step_timeout when disabled; got: {plain}"
    );
}

// ---------------------------------------------------------------------------
// Phase 4 acceptance tests: explicit --claude / --codex must not invoke
// `opencode models` (2026-05-09-slow-prep)
// ---------------------------------------------------------------------------

/// Non-harness inline-compose respects --step-timeout CLI flag.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn inline_compose_non_harness_respects_cli_step_timeout() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: cli step timeout test\nprompt: hello\n---\nBody\n",
    )
    .unwrap();

    // Fake provider emits one event then blocks forever.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"cli-step-test","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"cli-step-test"}'
printf '%s\n' '{"type":"step_finish","sessionID":"cli-step-test","part":{"reason":"tool-calls","tokens":{"input":1,"output":1,"total":2}}}'
printf '%s\n' '{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls"}}}'
while :; do /bin/sleep 1; done
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_TIMEOUT", "0s")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "1s")
        .env("CLAUDINE_KILL_GRACE", "1s")
        .args([
            "inline-compose",
            "--opencode",
            "--step-timeout",
            "2s",
            md_file.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("no stream activity"),
        "stderr should contain step_timeout breach message from CLI --step-timeout; got: {plain}"
    );

    let log_path = today_log_path(workspace.path());
    if log_path.exists() {
        let log = fs::read_to_string(&log_path).unwrap();
        let last = log.lines().last().unwrap();
        let entry: serde_json::Value = serde_json::from_str(last).unwrap();
        assert_eq!(
            entry
                .get("extra")
                .and_then(|e| e.get("exit_reason"))
                .and_then(|v| v.as_str()),
            Some("step_timeout"),
            "JSONL session_end must have extra.exit_reason=step_timeout; last entry: {last}"
        );
    }
}
