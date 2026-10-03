//! A hook `message` action under `claudine handle` is delivered, or reported
//! unfinished, before the handler exits, and the exit drain never runs past
//! the handler's own overall deadline (`CLAUDINE_HANDLE_DEADLINE_SECONDS`).
//!
//! Every case posts to a loopback [`WebhookListener`] through the same
//! environment-backed route as `lifecycle_message_drain`.

use crate::common;
use crate::lifecycle_message_drain::{CHILD_WATCHDOG, PipedRun, WITHHOLD_WINDOW, pending_warning};

use common::CliProcessFixture;
use common::webhook_listener::{
    DUMMY_TOKEN, ListenerMode, ROUTE_NAME, WebhookListener, write_config_with_webhook_route,
};
use std::time::{Duration, Instant};

/// A Claude `SessionEnd` hook payload.
const SESSION_END_PAYLOAD: &str = r#"{"hook_event_name":"SessionEnd","session_id":"drain-handle"}"#;

/// The handler deadline for the capped cases. Far enough under the 10 s drain
/// budget that a drain ignoring the cap cannot pass.
const HANDLE_DEADLINE_SECONDS: u64 = 2;

/// Slack on top of the handler deadline for scheduling under a loaded suite.
/// An uncapped drain would run about 8 s longer than the deadline, so this
/// still separates the two.
const DEADLINE_ALLOWANCE: Duration = Duration::from_secs(4);

/// How long the `call` action in the over-deadline case keeps the handler
/// busy; comfortably past the deadline plus [`DEADLINE_ALLOWANCE`].
const BUSY_SECONDS: u64 = 8;

/// Write a user config whose `session_end` hook runs `actions`, next to the
/// webhook route.
fn write_session_end_actions(fixture: &CliProcessFixture, actions: serde_json::Value) {
    write_config_with_webhook_route(
        fixture.home(),
        serde_json::json!({
            "tts": false,
            "logging": false,
            "protect": { "enabled": false },
            "actions": { "session_end": actions },
        }),
    );
}

/// Spawn `claudine handle session_end --provider claude` against `listener`,
/// with `deadline_seconds` as the handler's overall deadline when given.
fn spawn_handle(
    fixture: &CliProcessFixture,
    listener: &WebhookListener,
    deadline_seconds: Option<u64>,
) -> PipedRun {
    let mut command = fixture.command_std();
    listener.apply_route_env(&mut command);
    if let Some(seconds) = deadline_seconds {
        command.env("CLAUDINE_HANDLE_DEADLINE_SECONDS", seconds.to_string());
    }
    command.args(["handle", "session_end", "--provider", "claude"]);
    PipedRun::spawn(command, Some(SESSION_END_PAYLOAD))
}

/// A `call` action that keeps the handler busy for [`BUSY_SECONDS`] with a
/// program the fixture's minimal `PATH` carries.
fn busy_call_action() -> serde_json::Value {
    #[cfg(unix)]
    let (command, args) = ("sleep", vec![BUSY_SECONDS.to_string()]);
    // `ping -n N` waits about N-1 seconds; it is the console-safe sleep.
    #[cfg(windows)]
    let (command, args) = (
        "ping",
        vec!["-n".to_string(), (BUSY_SECONDS + 1).to_string(), "127.0.0.1".to_string()],
    );
    serde_json::json!({ "type": "call", "command": command, "args": args, "timeout_ms": 60_000 })
}

#[test]
fn a_hook_message_is_delivered_before_the_handler_exits() {
    let fixture = CliProcessFixture::named("handle-message-drain");
    write_session_end_actions(
        &fixture,
        serde_json::json!([{ "type": "message", "message": "handle-drain-marker" }]),
    );
    let listener = WebhookListener::start(ListenerMode::WithholdUntilReleased);
    let run = spawn_handle(&fixture, &listener, None);

    let (run, _) = run.await_requests(&listener, 1);
    let run = run.assert_running_for(WITHHOLD_WINDOW, "while the hook message's reply was withheld");
    listener.release();
    let finished = run.finish(CHILD_WATCHDOG);
    assert_eq!(finished.status.code(), Some(0), "stderr:\n{}", finished.stderr);

    let requests = listener.requests();
    assert_eq!(requests.len(), 1, "one POST per hook message: {requests:?}");
    assert!(requests[0].body.contains("handle-drain-marker"));
    assert!(
        !finished.stderr.contains("still sending at exit"),
        "a delivered message is not reported pending:\n{}",
        finished.stderr
    );
}

/// A stalled send consumes only what is left of the handler's deadline, not
/// the 10 s drain budget, and a handler that finished its work exits `0`.
#[test]
fn a_stalled_hook_message_waits_only_for_the_rest_of_the_handler_deadline() {
    let fixture = CliProcessFixture::named("handle-message-stalled");
    write_session_end_actions(
        &fixture,
        serde_json::json!([{ "type": "message", "message": "handle-stalled-body" }]),
    );
    let listener = WebhookListener::start(ListenerMode::NeverReply);
    let run = spawn_handle(&fixture, &listener, Some(HANDLE_DEADLINE_SECONDS));

    let (run, _) = run.await_requests(&listener, 1);
    let sent = Instant::now();
    let finished = run.finish(CHILD_WATCHDOG);
    let waited = finished.exited_at.duration_since(sent);
    let stderr = &finished.stderr;

    assert_eq!(finished.status.code(), Some(0), "stderr:\n{stderr}");
    assert!(
        waited < Duration::from_secs(HANDLE_DEADLINE_SECONDS) + DEADLINE_ALLOWANCE,
        "the drain is capped by the handler deadline ({waited:?})\n{stderr}"
    );
    let warning = pending_warning(stderr)
        .unwrap_or_else(|| panic!("no pending-delivery warning:\n{stderr}"));
    assert!(warning.contains(&format!("Route {ROUTE_NAME} was")), "{warning}");
    assert!(!warning.contains("handle-stalled-body"), "{warning}");
    assert!(!stderr.contains(DUMMY_TOKEN), "{stderr}");
    assert!(
        !stderr.contains("deadline exceeded"),
        "the handler finished its own work in time:\n{stderr}"
    );
}

/// A handler whose own work outlives its deadline still exits `124` on the
/// deadline, reporting the message it had started without waiting longer.
#[test]
fn a_handler_past_its_deadline_exits_124_and_reports_the_pending_message() {
    let fixture = CliProcessFixture::named("handle-message-deadline");
    write_session_end_actions(
        &fixture,
        serde_json::json!([
            { "type": "message", "message": "handle-deadline-body" },
            busy_call_action(),
        ]),
    );
    let listener = WebhookListener::start(ListenerMode::NeverReply);
    let run = spawn_handle(&fixture, &listener, Some(HANDLE_DEADLINE_SECONDS));

    let (run, _) = run.await_requests(&listener, 1);
    let sent = Instant::now();
    let finished = run.finish(CHILD_WATCHDOG);
    let waited = finished.exited_at.duration_since(sent);
    let stderr = &finished.stderr;

    assert_eq!(finished.status.code(), Some(124), "stderr:\n{stderr}");
    assert!(
        waited < Duration::from_secs(HANDLE_DEADLINE_SECONDS) + DEADLINE_ALLOWANCE,
        "no drain wait after the deadline ({waited:?})\n{stderr}"
    );
    assert!(stderr.contains("deadline exceeded"), "{stderr}");
    let warning = pending_warning(stderr)
        .unwrap_or_else(|| panic!("no pending-delivery warning:\n{stderr}"));
    assert!(warning.contains(&format!("Route {ROUTE_NAME} was")), "{warning}");
    assert!(
        stderr.find("deadline exceeded") < stderr.find("still sending at exit"),
        "the deadline diagnostic comes before the drain's warning:\n{stderr}"
    );

    // The busy `call` child outlives the handler. On Windows a process whose
    // working directory is inside the fixture would block its cleanup.
    if cfg!(windows) {
        let busy_until = sent + Duration::from_secs(BUSY_SECONDS + 1);
        if let Some(remaining) = busy_until.checked_duration_since(Instant::now()) {
            std::thread::sleep(remaining);
        }
    }
}
