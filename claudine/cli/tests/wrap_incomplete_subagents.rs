//! Integration tests: a provider that abandons its sub-agents and exits 0 is
//! a failure, not a silent success.
//!
//! **Level 1, every platform.** These drive the real wrapper through a fake
//! `claude` on the fixture `PATH` and assert on lifecycle side effects written
//! to a file, the synthesized `session_end` JSONL row, and rendered stderr
//! prose. Nothing here asserts a signal number, a terminal glyph, or a
//! scrollback position, so no L2 harness is involved and no window gains focus.
//! Only the fake-provider script is `cfg`-selected; every assertion is shared.
//!
//! ## The seam this crosses
//!
//! The 2026-08-31 incident: Claude dispatched two commit sub-agents, both went
//! silent, Claude reported both as `stopped` via `task_notification`, completed
//! the parent turn, and exited 0. Claudine fired `commit.md`'s **success**
//! stack — a message plus `shell: just gitnexus` — with no commit in existence.
//!
//! The fix spans four layers, and only a process test crosses all of them at
//! once: the Claude parser must route a terminal `task_notification` through
//! the ledger rather than flattening it to `Info`; the ledger must poison the
//! summary without rewriting the native exit; `AttemptOutcome` must carry
//! `is_error`; and `classify_failure` must honor it. A regression in any one
//! layer restores the original user-facing bug while every isolated unit test
//! stays green.
//!
//! ## Side-effect path resolution
//!
//! `append_line` resolves its first argument against the effect engine's
//! mutation root — the repository root, else the launch CWD. The fixture pins
//! the launch CWD to its own temp workspace and rejects any root inside this
//! checkout, so a relative `events.log` lands at `<fixture cwd>/events.log`.

use std::fs;
use std::time::Duration;

mod common;
use common::wrap::today_log_path;
use common::{CliProcessFixture, strip_ansi};

/// What the fake `claude` reports about the two sub-agents it dispatched.
#[derive(Debug, Clone, Copy)]
enum TaskEnding {
    /// The incident: both tasks force-stopped, parent turn completed, exit 0.
    BothStopped,
    /// Both tasks completed cleanly, parent turn completed, exit 0. The
    /// companion that keeps the fix from collapsing into "always fail".
    BothCompleted,
    /// Both tasks stopped, then both re-reported as completed under the same
    /// provider IDs. A later success for the *same ID* clears the earlier stop.
    StoppedThenCompleted,
    /// Twelve stopped tasks with names long enough that the 240-character
    /// lifecycle headline cannot possibly list them all.
    ManyLongNamedStops,
}

impl TaskEnding {
    /// The stream lines that follow the two `task_started` records.
    fn event_lines(self) -> Vec<String> {
        let notify = |id: &str, name: &str, status: &str| {
            format!(
                r#"{{"type":"task_notification","task_id":"{id}","name":"{name}","status":"{status}"}}"#
            )
        };
        match self {
            Self::BothStopped => vec![
                notify("sa_1", "commit-alpha", "stopped"),
                notify("sa_2", "commit-beta", "stopped"),
            ],
            Self::BothCompleted => vec![
                notify("sa_1", "commit-alpha", "completed"),
                notify("sa_2", "commit-beta", "completed"),
            ],
            Self::StoppedThenCompleted => vec![
                notify("sa_1", "commit-alpha", "stopped"),
                notify("sa_2", "commit-beta", "stopped"),
                notify("sa_1", "commit-alpha", "completed"),
                notify("sa_2", "commit-beta", "completed"),
            ],
            Self::ManyLongNamedStops => (0..LONG_NAMED_STOP_COUNT)
                .map(|index| {
                    notify(
                        &format!("sa_{index}"),
                        &long_task_name(index),
                        "stopped",
                    )
                })
                .collect(),
        }
    }

    /// The `task_started` records this ending dispatches, in stream order.
    /// Ledger facts follow first-observation order, so this is also the order
    /// the machine record and the diagnostic will use.
    fn start_lines(self) -> Vec<String> {
        let start = |id: &str, name: &str| {
            format!(r#"{{"type":"task_started","task_id":"{id}","name":"{name}"}}"#)
        };
        match self {
            Self::ManyLongNamedStops => (0..LONG_NAMED_STOP_COUNT)
                .map(|index| start(&format!("sa_{index}"), &long_task_name(index)))
                .collect(),
            _ => vec![
                start("sa_1", "commit-alpha"),
                start("sa_2", "commit-beta"),
            ],
        }
    }
}

/// Enough tasks with long enough names that the clamped headline must drop
/// some of them.
const LONG_NAMED_STOP_COUNT: usize = 12;

fn long_task_name(index: usize) -> String {
    format!("a-deliberately-long-sub-agent-task-name-number-{index}")
}

/// Install a fake `claude` that replays the incident stream and exits 0.
fn write_provider(fixture: &CliProcessFixture, ending: TaskEnding) {
    let mut events = vec![
        r#"{"type":"system","subtype":"init","session_id":"incident","model":"claude-opus"}"#
            .to_string(),
    ];
    events.extend(ending.start_lines());
    events.push(
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"All done."}]}}"#
            .to_string(),
    );
    events.extend(ending.event_lines());
    // The parent turn itself succeeds — that is exactly what made the incident
    // invisible. `is_error` is absent and the native exit below is 0.
    events.push(
        r#"{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":1,"duration_ms":600000}"#
            .to_string(),
    );

    #[cfg(unix)]
    {
        let body = events
            .iter()
            .map(|line| format!("printf '%s\\n' '{line}'\n"))
            .collect::<String>();
        // Drain the composed prompt so the wrapper's stdin write cannot race
        // the child's exit.
        let script = format!("#!/bin/sh\ncat > /dev/null 2>/dev/null\n{body}exit 0\n");
        common::write_executable(&fixture.bin_dir().join("claude"), &script);
    }
    #[cfg(windows)]
    {
        let body = events
            .iter()
            .map(|line| format!("echo {line}\r\n"))
            .collect::<String>();
        let script = format!("@echo off\r\n{body}exit /b 0\r\n");
        common::write(&fixture.bin_dir().join("claude.cmd"), &script);
    }
}

/// A compose document whose `success` and `failure` stacks each leave a
/// distinguishable trace, so the test can prove which one ran.
const DOCUMENT: &str = r#"---
title: incomplete subagent replay
success:
  stack:
    - action: {append_line: ["events.log", "success-stack-ran"]}
failure:
  stack:
    - action: {append_line: ["events.log", "{{ 'failure-variant=' + err.variant }}"]}
    - action: {append_line: ["events.log", "{{ 'failure-msg=' + err.msg }}"]}
---
Replay of the 2026-08-31 silent-success incident.
"#;

fn replay_fixture(name: &str, ending: TaskEnding) -> (CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    let md_file = fixture.cwd().join("replay.md");
    fs::write(&md_file, DOCUMENT).unwrap();
    write_provider(&fixture, ending);
    (fixture, md_file)
}

/// Lines the lifecycle stacks appended, in order.
fn lifecycle_trace(fixture: &CliProcessFixture) -> Vec<String> {
    fs::read_to_string(fixture.cwd().join("events.log"))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// The `extra` object of the last synthesized `session_end` row.
fn last_session_end_extra(fixture: &CliProcessFixture) -> serde_json::Value {
    let log = fs::read_to_string(today_log_path(fixture.home())).unwrap_or_default();
    let last = log
        .lines()
        .rfind(|line| line.contains("stream_wrapper_summary"))
        .unwrap_or_else(|| panic!("no synthetic session_end row in:\n{log}"))
        .to_string();
    let entry: serde_json::Value = serde_json::from_str(&last).unwrap();
    entry.get("extra").cloned().unwrap_or(serde_json::Value::Null)
}

/// Acceptance criterion 6. The whole incident shape, end to end.
#[test]
fn stopped_subagents_fire_the_failure_stack_and_never_the_success_stack() {
    let (fixture, md_file) = replay_fixture("incomplete-subagents", TaskEnding::BothStopped);

    let assert = fixture
        .command()
        .args(["compose", "--claude", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let trace = lifecycle_trace(&fixture);

    // 1. The success stack — and therefore every success side effect the
    //    incident actually ran — did not fire.
    assert!(
        !trace.iter().any(|line| line == "success-stack-ran"),
        "the success stack must not run when sub-agent work is unresolved; trace: {trace:?}\nstderr: {plain}"
    );

    // 2. The failure stack fired, attributed to the incomplete work rather
    //    than a generic agent failure or a timeout.
    assert!(
        trace
            .iter()
            .any(|line| line == "failure-variant=incomplete_subagents"),
        "the failure stack must observe the incomplete_subagents label; trace: {trace:?}\nstderr: {plain}"
    );
    let message = trace
        .iter()
        .find(|line| line.starts_with("failure-msg="))
        .unwrap_or_else(|| panic!("no failure-msg line; trace: {trace:?}\nstderr: {plain}"));
    assert!(
        message.contains("sub-agent"),
        "the headline must name the failure; got {message:?}"
    );

    // 3. The full terminal diagnostic enumerates every stopped task.
    assert!(
        plain.contains("2 sub-agent tasks did not complete"),
        "the full diagnostic must be rendered; got: {plain}"
    );
    for name in ["commit-alpha", "commit-beta"] {
        assert!(
            plain.contains(name),
            "every stopped task must appear in the diagnostic; {name} missing from: {plain}"
        );
    }

    // 4. The machine record: native exit 0 preserved, semantic failure
    //    recorded, and the complete fact list at the top level of `extra`.
    let extra = last_session_end_extra(&fixture);
    assert_eq!(
        extra["exit_code"],
        serde_json::json!(0),
        "Claudine did not kill the child, so the native exit must stay 0; extra: {extra}"
    );
    assert_eq!(
        extra["exit_reason"],
        serde_json::json!("incomplete_subagents"),
        "extra: {extra}"
    );
    let facts = extra["subagent_outcomes"]
        .as_array()
        .unwrap_or_else(|| panic!("extra.subagent_outcomes must be a top-level array; extra: {extra}"));
    assert_eq!(facts.len(), 2, "extra: {extra}");
    assert_eq!(facts[0]["task_id"], serde_json::json!("sa_1"));
    assert_eq!(facts[0]["name"], serde_json::json!("commit-alpha"));
    assert_eq!(facts[0]["outcome"], serde_json::json!("stopped"));
    assert_eq!(facts[0]["raw_status"], serde_json::json!("stopped"));
    assert_eq!(facts[1]["task_id"], serde_json::json!("sa_2"));
}

/// The survival companion. Same provider, same document, same native exit —
/// only the terminal statuses differ. Without this a fix that failed every run
/// with sub-agents would look correct.
#[test]
fn completed_subagents_still_fire_the_success_stack() {
    let (fixture, md_file) = replay_fixture("complete-subagents", TaskEnding::BothCompleted);

    let assert = fixture
        .command()
        .args(["compose", "--claude", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let trace = lifecycle_trace(&fixture);
    assert!(
        trace.iter().any(|line| line == "success-stack-ran"),
        "a clean sub-agent run must still succeed; trace: {trace:?}\nstderr: {plain}"
    );
    assert!(
        !trace.iter().any(|line| line.starts_with("failure-")),
        "no failure stack may run on a clean sub-agent run; trace: {trace:?}"
    );
    assert!(
        !plain.contains("did not complete"),
        "no incomplete diagnostic may render on a clean run; got: {plain}"
    );

    let extra = last_session_end_extra(&fixture);
    assert!(
        extra.get("subagent_outcomes").is_none(),
        "a clean run must omit the field entirely; extra: {extra}"
    );
}

/// Acceptance criterion 8, through the real wrapper: reconciliation is by
/// provider task ID, and a later success for the same ID clears the stop.
#[test]
fn a_later_success_for_the_same_task_id_clears_the_stop() {
    let (fixture, md_file) =
        replay_fixture("reconciled-subagents", TaskEnding::StoppedThenCompleted);

    let assert = fixture
        .command()
        .args(["compose", "--claude", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let trace = lifecycle_trace(&fixture);
    assert!(
        trace.iter().any(|line| line == "success-stack-ran"),
        "a stop cleared by a later same-ID success must succeed; trace: {trace:?}\nstderr: {plain}"
    );
}

/// The concise headline is clamped to 240 characters; the machine record is
/// not. Prove the two diverge in exactly that direction — the JSONL row keeps
/// every fact the headline had to drop, and the full terminal diagnostic keeps
/// every task name too.
#[test]
fn display_truncation_never_reaches_the_machine_record() {
    let (fixture, md_file) =
        replay_fixture("truncated-subagents", TaskEnding::ManyLongNamedStops);

    let assert = fixture
        .command()
        .args(["compose", "--claude", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let trace = lifecycle_trace(&fixture);

    // The lifecycle headline obeys the shared hygiene contract …
    let headline = trace
        .iter()
        .find(|line| line.starts_with("failure-msg="))
        .unwrap_or_else(|| panic!("no failure-msg line; trace: {trace:?}\nstderr: {plain}"))
        .trim_start_matches("failure-msg=")
        .to_string();
    assert!(
        headline.chars().count() <= 240,
        "the headline must stay within the 240-character budget; got {} chars: {headline}",
        headline.chars().count()
    );
    assert!(
        headline.starts_with("12 sub-agent tasks did not complete"),
        "the headline must carry the incomplete count; got {headline:?}"
    );
    let named_in_headline = (0..LONG_NAMED_STOP_COUNT)
        .filter(|index| headline.contains(&long_task_name(*index)))
        .count();
    assert!(
        named_in_headline < LONG_NAMED_STOP_COUNT,
        "this test is vacuous unless the headline actually had to truncate; it named all {LONG_NAMED_STOP_COUNT}: {headline}"
    );

    // … while the machine record keeps every fact.
    let extra = last_session_end_extra(&fixture);
    let facts = extra["subagent_outcomes"]
        .as_array()
        .unwrap_or_else(|| panic!("extra.subagent_outcomes missing; extra: {extra}"));
    assert_eq!(facts.len(), LONG_NAMED_STOP_COUNT, "extra: {extra}");
    for (index, fact) in facts.iter().enumerate() {
        assert_eq!(
            fact["name"],
            serde_json::json!(long_task_name(index)),
            "extra: {extra}"
        );
    }

    // … and so does the unbudgeted terminal diagnostic.
    for index in 0..LONG_NAMED_STOP_COUNT {
        assert!(
            plain.contains(&long_task_name(index)),
            "the full diagnostic must name every task; {} missing from: {plain}",
            long_task_name(index)
        );
    }
}
