//! Breach-message formatting tests for the watchdog.
//!
//! `format_step_timeout_breach_message` must consistently surface the
//! budget, any stuck subagent / tool inventory, and the OpenCode-specific
//! recent-subagent diagnostic.

use super::super::*;

#[test]
fn format_step_timeout_breach_message_no_outstanding() {
    let msg = format_step_timeout_breach_message(Duration::from_secs(180), &[], &[], &[], None);
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
        recent_subagents: recent,
        now: Instant::now(),
    };
    let msg =
        format_step_timeout_breach_message(Duration::from_secs(180), &[], &[], &[], Some(ctx));
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
        recent_subagents: recent,
        now: Instant::now(),
    };
    let msg =
        format_step_timeout_breach_message(Duration::from_secs(180), &[], &[], &[], Some(ctx));
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
        recent_subagents: std::collections::VecDeque::new(),
        now: Instant::now(),
    };
    let msg =
        format_step_timeout_breach_message(Duration::from_secs(180), &[], &[], &[], Some(ctx));
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
