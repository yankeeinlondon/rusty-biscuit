//! End-to-end proof that a content-guard trip flows
//! detector → `Aborted` → `classify_failure` → programmatic handler payload.
//!
//! Ties Phases 1 (types) + 2 (detector) + 7.1 (payload threading) together:
//! the pure detector recognizes the captured 6-line runaway cycle, the
//! resulting outcome classifies as `AgentFailure` (never `Timeout`, never a
//! suppressed `Interrupted`), and the programmatic `handle` command receives
//! `CLAUDINE_ERROR_KIND=runaway_repetition` in its environment plus the
//! `error_kind` + cycle detail in the JSON stdin payload.

#![cfg(unix)]

use claudine::harness::{
    ApprovedRuntimeCommand, AttemptOutcome, FailureEvent, GuardContext, HandlerTable,
    ProcessTermination, build_agent_failure_context, build_attempt_outcome, classify_failure,
    resolve_handler,
};
use claudine::runaway::{CompiledExitExpressions, ContentDetector, DetectorConfig, Trip};
use claudine::stream::summary::StreamExecutionSummary;

/// A handler script that captures the JSON stdin payload to `$1`, the
/// `CLAUDINE_ERROR_KIND` env var to `$2`, then echoes `true` so
/// `resolve_handler` resolves to a `Retry` action (proving it ran).
const CAPTURE_HANDLER_SCRIPT: &str =
    r#"cat > "$1"; printf '%s' "$CLAUDINE_ERROR_KIND" > "$2"; printf 'true'"#;

/// Build the capture handler whose payload/env land in the given temp paths.
fn capture_handler(
    payload_path: &std::path::Path,
    env_path: &std::path::Path,
) -> ApprovedRuntimeCommand {
    ApprovedRuntimeCommand {
        raw: "handler".to_string(),
        executable: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            CAPTURE_HANDLER_SCRIPT.to_string(),
            "claudine-test-handler".to_string(),
            payload_path.to_string_lossy().into_owned(),
            env_path.to_string_lossy().into_owned(),
        ],
    }
}

/// The exact 6-line runaway cycle captured from the spec's incident.
const RUNAWAY_CYCLE: &[&str] = &[
    "I will now continue.",
    "Let me proceed step by step.",
    "First, I check the file.",
    "Then I apply the change.",
    "Finally, I verify the result.",
    "Done.",
];

/// Drive the pure detector with the captured cycle until it trips.
fn detect_runaway_repetition() -> Trip {
    let mut detector =
        ContentDetector::new(DetectorConfig::default(), CompiledExitExpressions::empty());
    // 60 cycles is comfortably past the 30-cycle threshold.
    for _ in 0..60 {
        for line in RUNAWAY_CYCLE {
            if let Some(trip) = detector.feed(&format!("{line}\n")) {
                return trip;
            }
        }
    }
    panic!("detector did not trip on the captured runaway cycle");
}

#[test]
fn runaway_trip_flows_to_handler_payload() {
    // 1. The pure detector recognizes the captured 6-line cycle.
    let trip = detect_runaway_repetition();
    let (cycle_len, repeats) = match trip {
        Trip::RunawayRepetition { cycle_len, repeats } => (cycle_len, repeats),
        other => panic!("expected RunawayRepetition, got {other:?}"),
    };
    assert_eq!(cycle_len, 6, "captured cycle is 6 lines long");
    assert!(repeats >= 30, "must trip at the conservative 30-cycle floor");

    // 2. The trip's outcome carries the honest guard label + structured detail.
    let guard_context = GuardContext {
        cycle_len: Some(cycle_len),
        repeats: Some(repeats),
        ..Default::default()
    };
    let outcome = AttemptOutcome {
        attempt: 1,
        session_id: None,
        final_response: String::new(),
        exit_code: 1,
        termination: ProcessTermination::Aborted,
        stderr_text: None,
        error_kind: Some("runaway_repetition".to_string()),
        guard_context: Some(guard_context.clone()),
    };

    // 3. The outcome classifies as a fail-fast AgentFailure, never a retry.
    assert_eq!(
        classify_failure(&outcome),
        Some(FailureEvent::AgentFailure),
        "Aborted must route to AgentFailure"
    );

    // 4. The programmatic handler receives the guard context in env + stdin.
    let out_dir = tempfile::tempdir().expect("create temp dir");
    let payload_path = out_dir.path().join("payload.json");
    let env_path = out_dir.path().join("env.txt");

    // `sh -c '<script>' <argv0> <payload_path> <env_path>` — inside the script
    // `$1`/`$2` are the capture paths. The handler echoes `true` so
    // `resolve_handler` returns a Retry action, proving the command ran.
    let script = r#"cat > "$1"; printf '%s' "$CLAUDINE_ERROR_KIND" > "$2"; printf 'true'"#;
    let handler = ApprovedRuntimeCommand {
        raw: "handler".to_string(),
        executable: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            script.to_string(),
            "claudine-test-handler".to_string(),
            payload_path.to_string_lossy().into_owned(),
            env_path.to_string_lossy().into_owned(),
        ],
    };

    let failure = build_agent_failure_context(
        "opencode",
        std::path::Path::new("/repo/prompts/runaway.md"),
        FailureEvent::AgentFailure,
        "runaway repetition detected".to_string(),
        1,
        None,
        Some(outcome),
        Some("runaway_repetition".to_string()),
        Some(&guard_context),
    );

    let action = resolve_handler(&failure, &HandlerTable::default(), Some(&handler))
        .expect("handler resolves")
        .expect("handler returns a Retry action");
    assert!(
        matches!(action, claudine::harness::HandlerAction::Retry { .. }),
        "echoing `true` yields a Retry action"
    );

    // 5a. The env var carries the honest per-guard label.
    let env_kind = std::fs::read_to_string(&env_path).expect("read env capture");
    assert_eq!(
        env_kind, "runaway_repetition",
        "CLAUDINE_ERROR_KIND must reach the handler env"
    );

    // 5b. The JSON stdin payload carries error_kind + the cycle detail.
    let payload_raw = std::fs::read_to_string(&payload_path).expect("read payload capture");
    let payload: serde_json::Value =
        serde_json::from_str(&payload_raw).expect("payload is valid JSON");
    assert_eq!(
        payload["error_kind"], "runaway_repetition",
        "JSON payload must carry the guard label"
    );
    assert_eq!(
        payload["guard_context"]["cycle_len"], cycle_len,
        "JSON payload must carry the detected cycle length"
    );
    assert_eq!(
        payload["guard_context"]["repeats"], repeats,
        "JSON payload must carry the observed repeat count"
    );
}

#[test]
fn exit_expression_trip_carries_pattern_into_handler_payload() {
    let guard_context = GuardContext {
        pattern: Some("STOP.".to_string()),
        scope: Some("opencode/kimi-for-coding/k2p7".to_string()),
        ..Default::default()
    };
    let outcome = AttemptOutcome {
        attempt: 1,
        session_id: None,
        final_response: String::new(),
        exit_code: 1,
        termination: ProcessTermination::Aborted,
        stderr_text: None,
        error_kind: Some("exit_expression".to_string()),
        guard_context: Some(guard_context.clone()),
    };
    assert_eq!(classify_failure(&outcome), Some(FailureEvent::AgentFailure));

    let out_dir = tempfile::tempdir().expect("create temp dir");
    let payload_path = out_dir.path().join("payload.json");
    let env_path = out_dir.path().join("env.txt");

    let script = r#"cat > "$1"; printf '%s' "$CLAUDINE_ERROR_KIND" > "$2"; printf 'true'"#;
    let handler = ApprovedRuntimeCommand {
        raw: "handler".to_string(),
        executable: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            script.to_string(),
            "claudine-test-handler".to_string(),
            payload_path.to_string_lossy().into_owned(),
            env_path.to_string_lossy().into_owned(),
        ],
    };

    let failure = build_agent_failure_context(
        "opencode",
        std::path::Path::new("/repo/prompts/runaway.md"),
        FailureEvent::AgentFailure,
        "exit expression matched".to_string(),
        1,
        None,
        Some(outcome),
        Some("exit_expression".to_string()),
        Some(&guard_context),
    );

    resolve_handler(&failure, &HandlerTable::default(), Some(&handler))
        .expect("handler resolves")
        .expect("handler returns an action");

    let env_kind = std::fs::read_to_string(&env_path).expect("read env capture");
    assert_eq!(env_kind, "exit_expression");

    let payload_raw = std::fs::read_to_string(&payload_path).expect("read payload capture");
    let payload: serde_json::Value =
        serde_json::from_str(&payload_raw).expect("payload is valid JSON");
    assert_eq!(payload["error_kind"], "exit_expression");
    assert_eq!(payload["guard_context"]["pattern"], "STOP.");
    assert_eq!(
        payload["guard_context"]["scope"],
        "opencode/kimi-for-coding/k2p7"
    );
}
