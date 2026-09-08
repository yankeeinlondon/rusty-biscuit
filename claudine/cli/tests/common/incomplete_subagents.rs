//! The 2026-08-31 silent-success incident stream, shared by the tests that
//! replay it.
//!
//! Two binaries drive the same fake provider from opposite ends:
//! `wrap_incomplete_subagents.rs` (Level 1) asserts lifecycle side effects and
//! the synthesized `session_end` row, while
//! `level2_incomplete_subagents_capture.rs` asserts what a real terminal
//! displayed. Both need the *same* NDJSON shape — a parent turn that succeeds
//! and a native exit of 0, with the failure carried only by the terminal
//! `task_notification` records — so it is authored once here rather than typed
//! out twice with a chance of diverging.
//!
//! Task identities stay with the callers: L1 picks names that overflow the
//! 240-character headline budget, L2 picks names long enough to force the
//! rendered list to wrap at the minimum supported width. Only the framing is
//! shared.

#![allow(dead_code)]

#[cfg(windows)]
use super::write;
#[cfg(unix)]
use super::write_executable;
use std::path::Path;

/// A `task_started` record for one dispatched sub-agent.
pub fn task_started(id: &str, name: &str) -> String {
    format!(r#"{{"type":"task_started","task_id":"{id}","name":"{name}"}}"#)
}

/// A `task_notification` record reporting `status` for one sub-agent.
pub fn task_notification(id: &str, name: &str, status: &str) -> String {
    format!(
        r#"{{"type":"task_notification","task_id":"{id}","name":"{name}","status":"{status}"}}"#
    )
}

/// The incident stream: session init, the `task_started` records, an assistant
/// turn that reads as success, the terminal notifications, and a `result` with
/// no `is_error`.
///
/// The parent turn succeeding is exactly what made the incident invisible, so
/// the framing is part of the fixture rather than incidental scaffolding.
pub fn incident_stream(start_lines: &[String], event_lines: &[String]) -> Vec<String> {
    let mut events = vec![
        r#"{"type":"system","subtype":"init","session_id":"incident","model":"claude-opus"}"#
            .to_string(),
    ];
    events.extend(start_lines.iter().cloned());
    events.push(
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"All done."}]}}"#
            .to_string(),
    );
    events.extend(event_lines.iter().cloned());
    events.push(
        r#"{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":1,"duration_ms":600000}"#
            .to_string(),
    );
    events
}

/// Install a fake `claude` in `bin_dir` that replays `events` and exits 0.
///
/// The script drains stdin first so the wrapper's composed-prompt write cannot
/// race the child's exit.
pub fn write_replay_provider(bin_dir: &Path, events: &[String]) {
    write_replay_provider_exiting(bin_dir, events, 0);
}

/// [`write_replay_provider`] with a chosen native exit code, for the case
/// where the provider itself fails *and* leaves tasks unfinished.
pub fn write_replay_provider_exiting(bin_dir: &Path, events: &[String], exit_code: i32) {
    #[cfg(unix)]
    {
        let body = events
            .iter()
            .map(|line| format!("printf '%s\\n' '{line}'\n"))
            .collect::<String>();
        let script =
            format!("#!/bin/sh\ncat > /dev/null 2>/dev/null\n{body}exit {exit_code}\n");
        write_executable(&bin_dir.join("claude"), &script);
    }
    #[cfg(windows)]
    {
        let body = events
            .iter()
            .map(|line| format!("echo {line}\r\n"))
            .collect::<String>();
        let script = format!("@echo off\r\n{body}exit /b {exit_code}\r\n");
        write(&bin_dir.join("claude.cmd"), &script);
    }
}

/// Install a fake `claude` in `bin_dir` that emits `events` and then goes
/// silent forever, so only a wrapper timeout can end the run.
///
/// Stdin is left alone: the wrapper's prompt write fits the pipe buffer, and
/// draining it would just be another way of stalling. The sleep loop's
/// coarsest granularity is Windows' `ping -n 2` (~1 s), which a 2 s silence
/// budget clears with margin.
pub fn write_stalling_provider(bin_dir: &Path, events: &[String]) {
    #[cfg(unix)]
    {
        let body = events
            .iter()
            .map(|line| format!("printf '%s\\n' '{line}'\n"))
            .collect::<String>();
        let script = format!("#!/bin/sh\n{body}while :; do /bin/sleep 1; done\n");
        write_executable(&bin_dir.join("claude"), &script);
    }
    #[cfg(windows)]
    {
        // `ping -n 2 127.0.0.1` is the console-safe ~1s sleep; `timeout /t`
        // needs an interactive console handle and fails under a piped stdin.
        let body = events
            .iter()
            .map(|line| format!("echo {line}\r\n"))
            .collect::<String>();
        let script =
            format!("@echo off\r\n{body}:loop\r\nping -n 2 127.0.0.1 >nul\r\ngoto loop\r\n");
        write(&bin_dir.join("claude.cmd"), &script);
    }
}

/// A compose document whose `success` and `failure` stacks each leave a
/// distinguishable trace, so a test can prove which one ran.
pub const REPLAY_DOCUMENT: &str = r#"---
title: incomplete subagent replay
success:
  stack:
    - action: {append_line: ["events.log", "success-stack-ran"]}
failure:
  stack:
    - action: {append_line: ["events.log", "{{ 'failure-code=' + err.code }}"]}
    - action: {append_line: ["events.log", "{{ 'failure-category=' + err.category }}"]}
    - action: {append_line: ["events.log", "{{ 'failure-msg=' + err.msg }}"]}
---
Replay of the 2026-08-31 silent-success incident.
"#;
