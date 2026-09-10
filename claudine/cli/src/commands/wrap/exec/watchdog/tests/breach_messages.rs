//! Breach-message formatting tests for the watchdog.
//!
//! `format_step_timeout_breach_message` must consistently surface the
//! budget, any stuck subagent / tool inventory, and the OpenCode-specific
//! recent-subagent diagnostic.

use super::super::*;
use claudine::stream::progress::SilenceOrigin;

#[test]
fn format_step_timeout_breach_message_no_outstanding() {
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Activity,
        &[],
        &[],
        &[],
        None,
    );
    assert!(msg.contains("3m 0s"));
    assert!(msg.contains("step_timeout"));
    assert!(!msg.contains("subagent"));
    assert!(!msg.contains("tool"));
}

#[test]
fn format_step_timeout_breach_message_lists_outstanding() {
    let snap = crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot {
        id: "ses_a".into(),
        name: Some("Commit work".into()),
        started_at: Instant::now(),
        last_progress_at: Instant::now(),
        elapsed_since_start: Duration::from_secs(900),
        elapsed_since_progress: Duration::from_secs(900),
    };
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(1800),
        SilenceOrigin::Activity,
        std::slice::from_ref(&snap),
        &[],
        &[],
        None,
    );
    assert!(msg.contains("30m 0s"));
    assert!(msg.contains("1 subagent"));
    assert!(msg.contains("ses_a"));
    assert!(msg.contains("Commit work"));
    assert!(msg.contains("idle 15m 0s"));
}

#[test]
fn format_step_timeout_breach_message_lists_stuck_tools() {
    let tool = claudine::stream::progress::InFlightTool {
        name: Some("Bash".into()),
        started_at: Instant::now() - Duration::from_secs(600),
        last_progress_at: Instant::now() - Duration::from_secs(600),
    };
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Activity,
        &[],
        std::slice::from_ref(&tool),
        &[],
        None,
    );
    assert!(msg.contains("3m 0s"));
    assert!(msg.contains("1 tool"));
    assert!(msg.contains("Bash"));
}

#[test]
fn format_step_timeout_breach_message_opencode_names_subagent_count() {
    let mut recent = std::collections::VecDeque::new();
    recent.push_back(
        crate::commands::wrap::exec::subagent_watchdog::RecentSubagentInfo {
            id: "sa-1".into(),
            name: Some("Alpha".into()),
            description: Some("do alpha".into()),
            completed_at: Instant::now(),
            status: Some("success".into()),
        },
    );
    let ctx = OpenCodeBreachContext {
        subagent_done_count: 3,
        step_in_flight: true,
        first_step_completed: true,
        recent_subagents: recent,
        now: Instant::now(),
    };
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Activity,
        &[],
        &[],
        &[],
        Some(ctx),
    );
    assert!(msg.contains("3 subagents observed"), "got: {msg}");
    assert!(msg.contains("step boundary was still open"), "got: {msg}");
}

#[test]
fn format_step_timeout_breach_message_opencode_lists_recent_descriptions() {
    let mut recent = std::collections::VecDeque::new();
    // Simulate newest-first ring buffer: push_front so Desc-4 (newest)
    // ends up at the front of the deque.
    for i in 0..5 {
        recent.push_front(
            crate::commands::wrap::exec::subagent_watchdog::RecentSubagentInfo {
                id: format!("sa-{i}"),
                name: Some(format!("Name-{i}")),
                description: Some(format!("Desc-{i}")),
                completed_at: Instant::now() - Duration::from_secs((5 - i) as u64 * 60),
                status: Some("success".into()),
            },
        );
    }
    let ctx = OpenCodeBreachContext {
        subagent_done_count: 5,
        step_in_flight: false,
        first_step_completed: true,
        recent_subagents: recent,
        now: Instant::now(),
    };
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Activity,
        &[],
        &[],
        &[],
        Some(ctx),
    );
    assert!(msg.contains("5 subagents observed"), "got: {msg}");
    assert!(msg.contains("Recent subagents:"), "got: {msg}");
    // Newest first: Desc-4 should appear before Desc-3
    let idx_4 = msg.find("Desc-4").expect("Desc-4 should be present");
    let idx_3 = msg.find("Desc-3").expect("Desc-3 should be present");
    assert!(idx_4 < idx_3, "newest-first order required: {msg}");
}

#[test]
fn format_step_timeout_breach_message_opencode_no_recent_subagents() {
    let ctx = OpenCodeBreachContext {
        subagent_done_count: 0,
        step_in_flight: true,
        first_step_completed: true,
        recent_subagents: std::collections::VecDeque::new(),
        now: Instant::now(),
    };
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Activity,
        &[],
        &[],
        &[],
        Some(ctx),
    );
    assert!(msg.contains("step_timeout"), "got: {msg}");
    assert!(
        !msg.contains("subagents observed"),
        "count 0 must not render: {msg}"
    );
    assert!(
        msg.contains("step boundary was still open"),
        "step_in_flight hint must appear: {msg}"
    );
}

/// A child that never wrote anything is a launch problem, not a mid-run
/// stall, and the operator needs to be told which one they have.
#[test]
fn format_step_timeout_breach_message_names_a_startup_stall() {
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Launch,
        &[],
        &[],
        &[],
        None,
    );
    assert!(
        msg.contains("no output since the wrapped process launched 3m 0s ago"),
        "startup breach must be worded from launch: {msg}"
    );
    assert!(msg.contains("step_timeout"), "got: {msg}");
    assert!(
        !msg.contains("no stream activity"),
        "startup wording must replace the mid-run phrasing: {msg}"
    );
}

/// The other half of the distinction the spec requires: OpenCode did emit
/// activity, but never reached a `step_finish`, so the stall happened before
/// its first completed step.
#[test]
fn format_step_timeout_breach_message_opencode_names_a_stall_before_the_first_step() {
    let ctx = OpenCodeBreachContext {
        subagent_done_count: 0,
        step_in_flight: false,
        first_step_completed: false,
        recent_subagents: std::collections::VecDeque::new(),
        now: Instant::now(),
    };
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Activity,
        &[],
        &[],
        &[],
        Some(ctx),
    );
    assert!(
        msg.contains("no stream activity for 3m 0s"),
        "activity-anchored wording is required here: {msg}"
    );
    assert!(
        msg.contains("never completed a step"),
        "a pre-first-step stall must be named: {msg}"
    );
}

/// Counter-case: once a step boundary has been observed the pre-first-step
/// note must not appear, otherwise it says nothing.
#[test]
fn format_step_timeout_breach_message_opencode_omits_first_step_note_after_a_step_finish() {
    let ctx = OpenCodeBreachContext {
        subagent_done_count: 0,
        step_in_flight: false,
        first_step_completed: true,
        recent_subagents: std::collections::VecDeque::new(),
        now: Instant::now(),
    };
    let msg = format_step_timeout_breach_message(
        Duration::from_secs(180),
        SilenceOrigin::Activity,
        &[],
        &[],
        &[],
        Some(ctx),
    );
    assert!(
        !msg.contains("never completed a step"),
        "a completed step must suppress the pre-first-step note: {msg}"
    );
}
