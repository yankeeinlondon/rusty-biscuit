//! The wrapper's side of managed steering: one controller per provider child
//! and the control link that registers it with the local Rendezvous daemon.
//!
//! [`ExecutionSteering`] brackets one provider child exactly as
//! `SessionPresence` does, but it is a different thing: presence is a
//! replicated, best-effort dashboard entry governed by
//! `CLAUDINE_RENDEZVOUS_REPORT`; control registration is an in-memory,
//! local-only route that exists while the stream is open, and that switch
//! does not govern it.
//!
//! The link never blocks or fails the wrapped task. Each connection attempt
//! is time-bounded, and after [`MAX_CONSECUTIVE_FAILURES`] failed attempts
//! the link stops; the execution simply has no external route, which a
//! requester reports as unavailable. The controller itself needs no daemon,
//! so in-owner automatic help keeps working. A reconnection registers the
//! current binding again and never replays a request: the daemon resolved
//! the old connection's requests as unknown, and the controller refuses a
//! request ID it has already seen.

use std::sync::Arc;
use std::time::Duration;

use claudine::steering::controller::{
    ControllerConfig, DeliveryDeadlines, DeliveryFuture, ExecutionFacts, SteeringController, SteeringExecutor,
};
use claudine::steering::audit::DeliveryReport;
use claudine::steering::contract::{SendOutcome, SteeringRequest, SteeringResult};
use claudine::steering::eligibility::Route;
use claudine::steering::identity::ProcessStartIdentity;
use rendezvous_core::{SteeringControlDown, SteeringControlUp, SteeringDelivery, steering_control_down, steering_control_up};
use tokio::sync::mpsc;

use super::{DaemonAccessError, wire};

/// Bound on one connection or registration attempt.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
/// Failed attempts in a row after which the link stops trying.
pub(crate) const MAX_CONSECUTIVE_FAILURES: u32 = 5;
/// Waits between attempts, by consecutive failure count (the last repeats).
const RETRY_BACKOFF: [Duration; 4] =
    [Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4), Duration::from_secs(8)];
/// Frames buffered toward the daemon: updates plus replies.
const UPSTREAM_CAPACITY: usize = 64;

/// The production executor until a provider adapter exists. Eligibility
/// already blocks every route without a reviewed adapter; this answers the
/// same way if a route is ever selected without one.
struct NoAdapter;

impl SteeringExecutor for NoAdapter {
    fn deliver(&self, request: SteeringRequest, _route: Route, _deadlines: DeliveryDeadlines) -> DeliveryFuture {
        Box::pin(async move {
            let mut report = DeliveryReport::new(SteeringResult::submitted(&request, None, SendOutcome::Unavailable));
            report.error = Some("this build implements no steering adapter for this execution".into());
            report
        })
    }
}

/// Steering ownership of one provider child. Dropping it stops the
/// controller and closes the control link, which removes the registration.
pub(crate) struct ExecutionSteering {
    controller: SteeringController,
    link: tokio::task::JoinHandle<()>,
}

impl ExecutionSteering {
    /// Starts steering ownership for a wrapped `provider` child working in
    /// `cwd`. No launch profile is mapped yet, so the execution registers
    /// as unavailable with that reason until a provider adapter maps one.
    pub(crate) fn for_wrapped_child(provider: claudine::provider::Provider, interactive: bool, cwd: &std::path::Path) -> Option<Self> {
        Self::start(ExecutionFacts {
            provider,
            profile_id: None,
            os: claudine::steering::host_os(),
            launch_mode: if interactive {
                claudine::steering::vocabulary::LaunchMode::Interactive
            } else {
                claudine::steering::vocabulary::LaunchMode::NonInteractive
            },
            provider_version: None,
            cwd: Some(biscuit_file::to_portable_string(cwd)),
            name: None,
        })
    }

    /// Starts steering ownership for a child described by `facts`. `None`
    /// when no Tokio runtime is running or the wrapper's own process start
    /// cannot be read: without it the target would rest on a bare PID.
    pub(crate) fn start(facts: ExecutionFacts) -> Option<Self> {
        tokio::runtime::Handle::try_current().ok()?;
        let pid = std::process::id();
        let Some(start) = crate::cli_utils::process_start(pid) else {
            tracing::debug!(target: "claudine::steering", "wrapper process start unavailable; steering not registered");
            return None;
        };
        let wrapper = ProcessStartIdentity::new(pid, start.to_string()).ok()?;
        Some(Self::start_with(ControllerConfig::new(wrapper, facts), Arc::new(NoAdapter)))
    }

    /// [`Self::start`] with an explicit configuration and executor.
    pub(crate) fn start_with(config: ControllerConfig, executor: Arc<dyn SteeringExecutor>) -> Self {
        let controller = SteeringController::spawn(config, executor);
        let link = tokio::spawn(run_link(controller.clone()));
        Self { controller, link }
    }

    #[cfg_attr(not(test), expect(dead_code, reason = "automatic help and provider adapters are its production callers"))]
    pub(crate) fn controller(&self) -> &SteeringController {
        &self.controller
    }
}

#[cfg(test)]
impl ExecutionSteering {
    pub(crate) fn link_is_finished(&self) -> bool {
        self.link.is_finished()
    }
}

impl Drop for ExecutionSteering {
    fn drop(&mut self) {
        self.controller.shutdown();
        self.link.abort();
    }
}

fn frame(kind: steering_control_up::Kind) -> SteeringControlUp {
    SteeringControlUp { kind: Some(kind) }
}

async fn run_link(controller: SteeringController) {
    let mut failures = 0u32;
    loop {
        match serve(&controller).await {
            Ok(()) => failures = 0,
            Err(error) => {
                failures += 1;
                tracing::debug!(
                    target: "claudine::steering",
                    error = &error as &dyn std::error::Error,
                    failures,
                    "steering control link unavailable"
                );
                if failures >= MAX_CONSECUTIVE_FAILURES {
                    tracing::debug!(target: "claudine::steering", "steering control link stopped; this execution has no external route");
                    return;
                }
            }
        }
        let index = (failures.max(1) as usize - 1).min(RETRY_BACKOFF.len() - 1);
        tokio::time::sleep(RETRY_BACKOFF[index]).await;
    }
}

/// One registered connection. `Ok` means it was registered and later
/// closed; `Err` means it never registered.
async fn serve(controller: &SteeringController) -> Result<(), DaemonAccessError> {
    let endpoint = rendezvous_core::default_local_endpoint()?;
    let mut client = tokio::time::timeout(CONNECT_TIMEOUT, rendezvous_client::connect(&endpoint))
        .await
        .map_err(|_| DaemonAccessError::TimedOut)??;

    let mut updates = controller.subscribe();
    let (upstream, frames) = mpsc::channel(UPSTREAM_CAPACITY);
    let register = wire::snapshot_to_info(&updates.borrow_and_update());
    // The channel is empty, so the first frame always fits.
    let _ = upstream.try_send(frame(steering_control_up::Kind::Register(register)));
    let open = client.steering_control(tokio_stream::wrappers::ReceiverStream::new(frames));
    let mut down = tokio::time::timeout(CONNECT_TIMEOUT, open)
        .await
        .map_err(|_| DaemonAccessError::TimedOut)??
        .into_inner();
    match tokio::time::timeout(CONNECT_TIMEOUT, down.message()).await {
        Ok(Ok(Some(SteeringControlDown { kind: Some(steering_control_down::Kind::Accepted(_)) }))) => {}
        Ok(Err(status)) => return Err(status.into()),
        _ => return Err(DaemonAccessError::NotAccepted),
    }

    loop {
        tokio::select! {
            changed = updates.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
                let info = wire::snapshot_to_info(&updates.borrow_and_update());
                if upstream.send(frame(steering_control_up::Kind::Update(info))).await.is_err() {
                    return Ok(());
                }
            }
            message = down.message() => match message {
                Ok(Some(SteeringControlDown { kind: Some(steering_control_down::Kind::Delivery(delivery)) })) => {
                    tokio::spawn(answer(controller.clone(), upstream.clone(), *delivery));
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => return Ok(()),
            },
        }
    }
}

/// Submits one routed delivery and sends the owner's reply upstream. If the
/// stream is gone by then, the reply is dropped: the controller already
/// recorded the outcome, and the requester was told it is unknown.
async fn answer(controller: SteeringController, upstream: mpsc::Sender<SteeringControlUp>, delivery: SteeringDelivery) {
    let request_id = delivery.request_id.clone();
    let reply = match wire::delivery_to_submission(delivery) {
        Ok(submission) => wire::reply_to_wire(&request_id, &controller.submit(submission).await),
        Err(error) => wire::refusal(&request_id, &error),
    };
    let _ = upstream.send(frame(steering_control_up::Kind::Reply(reply))).await;
}
