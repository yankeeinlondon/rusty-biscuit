//! The `pi-rpc` steering adapter (revision 1): delivery over a managed Pi
//! RPC child.
//!
//! Every delivery holds the session's mutation lock from its state check to
//! its last answer, so it never interleaves with another delivery or with the
//! settlement decision. Before sending anything it takes a fresh `get_state`:
//! a different session id or a state the operation cannot serve sends
//! nothing. Outcomes report only what Pi established — a `steer` answer means
//! the message is queued in Pi's memory, a `prompt` answer that Pi accepted
//! it — and a command that may have reached Pi without an answer is unknown,
//! never resent.
//!
//! Interruption is the consented `abort`, then a wait for Pi to stop working,
//! then a fresh check that the same session is still current, then the
//! replacement `prompt`. Pending queues are never cleared. Each phase keeps its
//! own outcome, so a cancelled turn whose replacement failed is a partial
//! interruption, never a restart of the old work.

use std::sync::Arc;

use claudine::steering::audit::DeliveryReport;
use claudine::steering::contract::{
    CancellationOutcome, InterruptionOutcome, SendOutcome, SteeringRequest, SteeringResult,
};
use claudine::steering::controller::{DeliveryDeadlines, DeliveryFuture, SteeringExecutor};
use claudine::steering::eligibility::Route;
use claudine::steering::vocabulary::{ExecutionState, OperationIntent};
use claudine::stream::protocol::pi::rpc::{SessionState, session_state};
use serde_json::Value;
use tokio::time::Instant;

use super::{Inner, RpcError, commands, lock};
use crate::steering::render_chain;

/// How often an interruption re-checks whether Pi has stopped working.
const IDLE_POLL: std::time::Duration = std::time::Duration::from_millis(50);

pub(super) struct PiRpcExecutor {
    inner: Arc<Inner>,
}

impl PiRpcExecutor {
    pub(super) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }
}

impl SteeringExecutor for PiRpcExecutor {
    fn deliver(&self, request: SteeringRequest, route: Route, deadlines: DeliveryDeadlines) -> DeliveryFuture {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { deliver(&inner, request, route.mechanism.id, deadlines).await })
    }
}

/// Sends one correlated command and waits, until `deadline`, for Pi's
/// acceptance (with its data) or the reason there is none.
async fn command(inner: &Inner, build: impl FnOnce(&str) -> Value, deadline: Instant) -> Result<Value, RpcError> {
    let receiver = inner.request(build).map_err(RpcError::not_written)?;
    match tokio::time::timeout_at(deadline, receiver).await {
        Ok(Ok(answer)) => answer?.outcome.map_err(RpcError::Refused),
        Ok(Err(_)) => Err(RpcError::Exited),
        Err(_) => Err(RpcError::NoAnswer),
    }
}

async fn state(inner: &Inner, deadline: Instant) -> Result<SessionState, RpcError> {
    let data = command(inner, commands::get_state, deadline).await?;
    session_state(&data).map_err(RpcError::Unreadable)
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

/// The outcome of a message command: `accepted` is what an answer proves.
/// `subject` names what was sent, for the explanation.
fn message_outcome(sent: Result<Value, RpcError>, accepted: SendOutcome, subject: &str) -> (SendOutcome, Option<String>) {
    let (outcome, consequence) = match &sent {
        Ok(_) => return (accepted, None),
        Err(RpcError::Refused(_)) => (SendOutcome::Refused, "was not accepted"),
        Err(failure) if failure.may_have_been_sent() => {
            (SendOutcome::Unknown, "may have been taken and is never resent")
        }
        Err(_) => (SendOutcome::Unavailable, "was not sent"),
    };
    let context = [subject, consequence].join(" ");
    (outcome, sent.err().map(|failure| render_chain(&context, &failure)))
}

async fn deliver(
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
    {
        let state = lock(&inner.state);
        if state.closing || state.closed {
            return unavailable("the run has settled and is ending; nothing was sent");
        }
    }
    let fresh = match state(inner, first_deadline).await {
        Ok(fresh) => fresh,
        Err(failure) => {
            return unavailable(&render_chain("the session's state could not be confirmed, so nothing was sent", &failure));
        }
    };
    if !same_conversation(inner, &fresh) {
        return unavailable("Pi is now on a different session than the one selected; nothing was sent");
    }

    let text = request.message.as_str();
    match request.operation {
        OperationIntent::SteerActiveTurn => {
            if !fresh.is_streaming {
                note_state(inner, ExecutionState::Idle);
                return unavailable("the session is no longer working; nothing was sent");
            }
            let (outcome, detail) = message_outcome(
                command(inner, |id| commands::steer(id, text), deadlines.acceptance).await,
                SendOutcome::Queued,
                "the message",
            );
            submitted(&request, mechanism, outcome, detail)
        }
        OperationIntent::StartIdleTurn => {
            if !fresh.is_quiescent() {
                return unavailable("the session is not idle; nothing was sent");
            }
            let (outcome, detail) = message_outcome(
                command(inner, |id| commands::prompt(id, text), deadlines.acceptance).await,
                SendOutcome::Accepted,
                "the message",
            );
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
            if !fresh.is_streaming {
                return unavailable("the session is not working, so there is nothing to interrupt; nothing was sent");
            }
            interrupt_then_submit(inner, &request, mechanism, deadlines).await
        }
        other => unavailable(&format!("the pi-rpc adapter does not carry `{other}`; nothing was sent")),
    }
}

async fn interrupt_then_submit(
    inner: &Inner,
    request: &SteeringRequest,
    mechanism: &'static str,
    deadlines: DeliveryDeadlines,
) -> DeliveryReport {
    let cancellation_deadline = deadlines.cancellation.unwrap_or(deadlines.acceptance);
    let phases = |cancellation, replacement| InterruptionOutcome { cancellation, replacement };
    match command(inner, commands::abort, cancellation_deadline).await {
        Ok(_) => {}
        Err(failure @ RpcError::Refused(_)) => {
            let detail = render_chain("the turn was not interrupted and the replacement was not sent", &failure);
            return interrupted(request, mechanism, phases(CancellationOutcome::Refused, None), Some(detail));
        }
        Err(failure) if failure.may_have_been_sent() => {
            let detail = render_chain("the abort was not confirmed, so the replacement was not sent", &failure);
            return interrupted(request, mechanism, phases(CancellationOutcome::Unknown, None), Some(detail));
        }
        Err(failure) => {
            return submitted(request, mechanism, SendOutcome::Unavailable, Some(render_chain("nothing was sent", &failure)));
        }
    }

    // An abort answer is not proof the turn stopped: wait until Pi reports
    // it is no longer working, on the same session.
    let stopped = loop {
        match state(inner, cancellation_deadline).await {
            Ok(fresh) if !fresh.is_streaming => break Ok(fresh),
            Ok(_) if Instant::now() + IDLE_POLL < cancellation_deadline => tokio::time::sleep(IDLE_POLL).await,
            Ok(_) => break Err(None),
            Err(failure) => break Err(Some(failure)),
        }
    };
    let fresh = match stopped {
        Ok(fresh) => fresh,
        Err(failure) => {
            let context = "cancellation was not confirmed, so the replacement was not sent";
            let detail = match failure {
                Some(failure) => render_chain(context, &failure),
                None => [context, "Pi was still working when the cancellation deadline passed"].join(": "),
            };
            return interrupted(request, mechanism, phases(CancellationOutcome::Unknown, None), Some(detail));
        }
    };
    note_state(inner, ExecutionState::Idle);
    if !same_conversation(inner, &fresh) {
        return interrupted(
            request,
            mechanism,
            phases(CancellationOutcome::Established, None),
            Some("the turn was cancelled, but Pi is now on a different session, so the replacement was not sent".into()),
        );
    }
    let (replacement, detail) = message_outcome(
        command(inner, |id| commands::prompt(id, request.message.as_str()), deadlines.acceptance).await,
        SendOutcome::Accepted,
        "the turn was cancelled, but the replacement",
    );
    interrupted(request, mechanism, phases(CancellationOutcome::Established, Some(replacement)), detail)
}

/// Whether `fresh` still names the bound session. A different one is
/// recorded, which starts a new conversation generation, so a target listed
/// before it can no longer be selected.
fn same_conversation(inner: &Inner, fresh: &SessionState) -> bool {
    let bound = lock(&inner.state).conversation.clone();
    if bound.as_deref() == Some(fresh.session_id.as_str()) {
        return true;
    }
    inner.bind_conversation(fresh.session_id.clone());
    false
}

fn note_state(inner: &Inner, state: ExecutionState) {
    if let Some(controller) = inner.controller() {
        controller.set_state(state);
    }
}
