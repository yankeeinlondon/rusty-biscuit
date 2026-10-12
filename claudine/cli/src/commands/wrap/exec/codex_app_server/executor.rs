//! The `codex-app-server` steering adapter (revision 1): delivery over a
//! managed Codex app-server child.
//!
//! Every delivery holds the session's mutation lock until its last answer, so
//! it never interleaves with another delivery or with the settlement
//! decision. Targeting rests on Codex's own guards rather than a state check
//! followed by a send:
//!
//! - **Steer** (`turn/steer`) names the thread and the running turn as
//!   `expectedTurnId`; Codex refuses the input unless that exact turn is still
//!   active, so a turn that ended or changed receives nothing. An answer means
//!   Codex added the message to that turn's pending input (`queued`); it
//!   reaches the model at the turn's next request, after the running tool
//!   batch or the response being generated, and the turn continues with it.
//! - **Idle turn** (`turn/start`) runs only after a fresh `thread/read` shows
//!   the thread idle.
//! - **Interruption** is the consented `turn/interrupt`, then a wait for that
//!   turn's `turn/completed`, then a `turn/start` for the replacement. Each
//!   phase keeps its own outcome; a cancelled turn whose replacement failed is
//!   a partial interruption, never a restart of the old work.
//!
//! A request that may have reached Codex without an answer is unknown and
//! never resent: Codex does not deduplicate `clientUserMessageId`.

use std::sync::Arc;

use claudine::steering::audit::DeliveryReport;
use claudine::steering::contract::{
    CancellationOutcome, InterruptionOutcome, SendOutcome, SteeringRequest, SteeringResult,
};
use claudine::steering::controller::{DeliveryDeadlines, DeliveryFuture, SteeringExecutor};
use claudine::steering::eligibility::Route;
use claudine::steering::vocabulary::{ExecutionState, OperationIntent};
use claudine::stream::protocol::codex::app_server::{self, ThreadActivity, TurnStatus};
use serde_json::Value;
use tokio::time::Instant;

use super::{Inner, RpcError, commands, lock};
use crate::steering::render_chain;

/// How often an interruption re-checks whether the turn has completed.
const COMPLETION_POLL: std::time::Duration = std::time::Duration::from_millis(25);

pub(super) struct CodexAppServerExecutor {
    inner: Arc<Inner>,
}

impl CodexAppServerExecutor {
    pub(super) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }
}

impl SteeringExecutor for CodexAppServerExecutor {
    fn deliver(&self, request: SteeringRequest, route: Route, deadlines: DeliveryDeadlines) -> DeliveryFuture {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { deliver(&inner, request, route.mechanism.id, deadlines).await })
    }
}

/// Sends one correlated request and waits, until `deadline`, for Codex's
/// result or the reason there is none.
async fn request(inner: &Inner, build: impl FnOnce(&str) -> Value, deadline: Instant) -> Result<Value, RpcError> {
    let receiver = inner.request(build).map_err(RpcError::not_written)?;
    match tokio::time::timeout_at(deadline, receiver).await {
        Ok(Ok(answer)) => answer,
        Ok(Err(_)) => Err(RpcError::Exited),
        Err(_) => Err(RpcError::NoAnswer),
    }
}

async fn activity(inner: &Inner, thread_id: &str, deadline: Instant) -> Result<ThreadActivity, RpcError> {
    let result = request(inner, |id| commands::thread_read(id, thread_id), deadline).await?;
    app_server::thread_activity(&result, thread_id).map_err(RpcError::Unreadable)
}

fn submitted(request: &SteeringRequest, mechanism: &'static str, outcome: SendOutcome, error: Option<String>) -> DeliveryReport {
    let mut report = DeliveryReport::new(SteeringResult::submitted(request, Some(mechanism), outcome));
    report.error = error;
    report
}

fn interrupted(
    request: &SteeringRequest,
    mechanism: &'static str,
    phases: InterruptionOutcome,
    error: Option<String>,
) -> DeliveryReport {
    let mut report = DeliveryReport::new(SteeringResult::interrupted(request, mechanism, phases));
    report.error = error;
    report
}

/// The outcome of a message request: `accepted` is what an answer proves,
/// once `read` confirms the answer names what was asked. `subject` names what
/// was sent, for the explanation.
fn message_outcome(
    sent: Result<Value, RpcError>,
    read: impl FnOnce(&Value) -> Result<(), RpcError>,
    accepted: SendOutcome,
    subject: &str,
) -> (SendOutcome, Option<String>) {
    let sent = sent.and_then(|result| read(&result));
    let (outcome, consequence) = match &sent {
        Ok(()) => return (accepted, None),
        Err(RpcError::Refused(_)) => (SendOutcome::Refused, "was not accepted"),
        Err(failure) if failure.may_have_been_sent() => (SendOutcome::Unknown, "may have been taken and is never resent"),
        Err(_) => (SendOutcome::Unavailable, "was not sent"),
    };
    let context = [subject, consequence].join(" ");
    (outcome, sent.err().map(|failure| render_chain(&context, &failure)))
}

pub(super) async fn deliver(
    inner: &Inner,
    request: SteeringRequest,
    mechanism: &'static str,
    deadlines: DeliveryDeadlines,
) -> DeliveryReport {
    let unavailable = |detail: &str| submitted(&request, mechanism, SendOutcome::Unavailable, Some(detail.to_string()));
    let first_deadline = deadlines.cancellation.unwrap_or(deadlines.acceptance);
    let Ok(_mutation) = tokio::time::timeout_at(first_deadline, inner.mutation.lock()).await else {
        return submitted(
            &request,
            mechanism,
            SendOutcome::Busy,
            Some("another operation held this session until the deadline; nothing was sent".into()),
        );
    };
    let (thread_id, active_turn) = {
        let state = lock(&inner.state);
        if state.closing || state.closed {
            return unavailable("the run has settled and is ending; nothing was sent");
        }
        match state.thread_id.clone() {
            Some(thread_id) => (thread_id, state.active_turn.clone()),
            None => return unavailable("Codex has not started its thread; nothing was sent"),
        }
    };

    let text = request.message.as_str();
    let client_id = request.id.to_string();
    match request.operation {
        OperationIntent::SteerActiveTurn => {
            let Some(turn_id) = active_turn else {
                return unavailable("the session is no longer working; nothing was sent");
            };
            let sent = request_steer(inner, &thread_id, &turn_id, text, &client_id, deadlines.acceptance).await;
            let (outcome, detail) = message_outcome(
                sent,
                |result| match app_server::steered_turn(result) {
                    Ok(accepted) if accepted == turn_id => Ok(()),
                    Ok(other) => Err(RpcError::Unreadable(app_server::MessageError::UnknownValue {
                        field: "turnId",
                        value: other,
                    })),
                    Err(error) => Err(RpcError::Unreadable(error)),
                },
                SendOutcome::Queued,
                "the message",
            );
            submitted(&request, mechanism, outcome, detail)
        }
        OperationIntent::StartIdleTurn => {
            if active_turn.is_some() {
                return unavailable("the session is working, not idle; nothing was sent");
            }
            match activity(inner, &thread_id, deadlines.acceptance).await {
                Ok(ThreadActivity::Idle) => {}
                Ok(_) => return unavailable("Codex does not report the thread idle; nothing was sent"),
                Err(failure) => {
                    return unavailable(&render_chain(
                        "the session's state could not be confirmed, so nothing was sent",
                        &failure,
                    ));
                }
            }
            let (outcome, detail) = start_turn(inner, &thread_id, text, &client_id, deadlines.acceptance, "the message").await;
            submitted(&request, mechanism, outcome, detail)
        }
        OperationIntent::InterruptThenSubmit => {
            if !request.may_interrupt() {
                return submitted(
                    &request,
                    mechanism,
                    SendOutcome::Refused,
                    Some("interruption needs consent for this session and operation".into()),
                );
            }
            let Some(turn_id) = active_turn else {
                return unavailable("the session is not working, so there is nothing to interrupt; nothing was sent");
            };
            interrupt_then_start(inner, &request, mechanism, deadlines, &thread_id, &turn_id).await
        }
        other => unavailable(&format!("the codex-app-server adapter does not carry `{other}`; nothing was sent")),
    }
}

async fn request_steer(
    inner: &Inner,
    thread_id: &str,
    turn_id: &str,
    text: &str,
    client_id: &str,
    deadline: Instant,
) -> Result<Value, RpcError> {
    request(inner, |id| commands::turn_steer(id, thread_id, turn_id, text, client_id), deadline).await
}

/// Starts a turn and records it, so the settlement check keeps the run open.
async fn start_turn(
    inner: &Inner,
    thread_id: &str,
    text: &str,
    client_id: &str,
    deadline: Instant,
    subject: &str,
) -> (SendOutcome, Option<String>) {
    let sent = request(inner, |id| commands::turn_start(id, thread_id, text, client_id), deadline).await;
    message_outcome(
        sent,
        |result| {
            let turn = app_server::started_turn(result).map_err(RpcError::Unreadable)?;
            inner.note_turn_started(&turn.id, false);
            Ok(())
        },
        SendOutcome::Accepted,
        subject,
    )
}

async fn interrupt_then_start(
    inner: &Inner,
    request_: &SteeringRequest,
    mechanism: &'static str,
    deadlines: DeliveryDeadlines,
    thread_id: &str,
    turn_id: &str,
) -> DeliveryReport {
    let cancellation_deadline = deadlines.cancellation.unwrap_or(deadlines.acceptance);
    let phases = |cancellation, replacement| InterruptionOutcome { cancellation, replacement };
    match request(inner, |id| commands::turn_interrupt(id, thread_id, turn_id), cancellation_deadline).await {
        Ok(_) => {}
        Err(failure @ RpcError::Refused(_)) => {
            let detail = render_chain("the turn was not interrupted and the replacement was not sent", &failure);
            return interrupted(request_, mechanism, phases(CancellationOutcome::Refused, None), Some(detail));
        }
        Err(failure) if failure.may_have_been_sent() => {
            let detail = render_chain("the interruption was not confirmed, so the replacement was not sent", &failure);
            return interrupted(request_, mechanism, phases(CancellationOutcome::Unknown, None), Some(detail));
        }
        Err(failure) => {
            return submitted(request_, mechanism, SendOutcome::Unavailable, Some(render_chain("nothing was sent", &failure)));
        }
    }

    // An interrupt answer is not proof the turn stopped: wait for its
    // completion. A turn that finished on its own first is not a
    // cancellation, and gets no replacement.
    let ended = loop {
        if let Some(status) = lock(&inner.state).completed_turns.get(turn_id).copied() {
            break Some(status);
        }
        if Instant::now() + COMPLETION_POLL >= cancellation_deadline {
            break None;
        }
        tokio::time::sleep(COMPLETION_POLL).await;
    };
    match ended {
        Some(TurnStatus::Interrupted) => {}
        Some(status) => {
            let detail = format!(
                "the turn ended as {} before the interruption took effect, so the replacement was not sent",
                match status {
                    TurnStatus::Completed => "completed",
                    TurnStatus::Failed => "failed",
                    _ => "unknown",
                }
            );
            return interrupted(request_, mechanism, phases(CancellationOutcome::Refused, None), Some(detail));
        }
        None => {
            return interrupted(
                request_,
                mechanism,
                phases(CancellationOutcome::Unknown, None),
                Some("cancellation was not confirmed before the deadline, so the replacement was not sent".into()),
            );
        }
    }
    if let Some(controller) = inner.controller() {
        controller.set_state(ExecutionState::Idle);
    }
    if inner.thread_id().as_deref() != Some(thread_id) {
        return interrupted(
            request_,
            mechanism,
            phases(CancellationOutcome::Established, None),
            Some("the turn was cancelled, but the session's thread changed, so the replacement was not sent".into()),
        );
    }
    let (replacement, detail) = start_turn(
        inner,
        thread_id,
        request_.message.as_str(),
        &request_.id.to_string(),
        deadlines.acceptance,
        "the turn was cancelled, but the replacement",
    )
    .await;
    interrupted(request_, mechanism, phases(CancellationOutcome::Established, Some(replacement)), detail)
}
