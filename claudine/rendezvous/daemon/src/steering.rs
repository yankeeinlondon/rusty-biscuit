//! In-memory steering control routing.
//!
//! The router maps each live managed execution to its owner's control stream
//! and carries one requester's steering request to that owner and the owner's
//! reply back. The owner — the Claudine wrapper, which alone holds the
//! provider's I/O — decides and reports every delivery outcome; the router
//! only reports what it can establish itself (no owner, stale binding,
//! duplicate request, full channel) and, for a forwarded request that never
//! got a reply, `UNKNOWN`.
//!
//! Everything here lives in daemon memory. A registration exists exactly as
//! long as its stream, a message payload only while its delivery is in
//! flight, and none of it reaches the register store, the session log, a
//! durable queue, or a mesh peer: steering is local to this host and user,
//! whose boundary the local endpoint already enforces. The router never
//! retries, never redirects to a different target, and after a reconnection
//! never replays an unresolved request.
//!
//! Contract: `claudine/docs/rendezvous/local-ipc.md` §15.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rendezvous_core::{
    MAX_ROUTE_DEADLINE_MS, ManagedTargetInfo, RouteOutcome, RouteSteeringRequest, RouteSteeringResponse,
    SteeringControlDown, SteeringReply, TargetBinding, steering_control_down,
};
use tokio::sync::{mpsc, oneshot};
use tonic::Status;

#[cfg(test)]
mod tests;

/// Deliveries buffered toward one owner. It mirrors the owner's own pending
/// bound, so a flood is refused as busy here instead of queuing unboundedly.
pub const OWNER_CHANNEL_CAPACITY: usize = 16;

/// Routed request IDs remembered for duplicate refusal.
pub const RECENT_REQUEST_CAPACITY: usize = 4096;

/// Frames sent down an owner's control stream.
pub type ControlSender = mpsc::Sender<Result<SteeringControlDown, Status>>;

/// Why a registration was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegisterError {
    #[error("managed target registration is missing `{0}`")]
    Missing(&'static str),
    #[error("execution is already registered by a live owner")]
    AlreadyRegistered,
    #[error("an update cannot change the execution or its wrapper process")]
    IdentityChanged,
    #[error("an update cannot lower the conversation generation")]
    GenerationRegressed,
    #[error("the daemon is shutting down")]
    Closed,
}

impl From<RegisterError> for Status {
    fn from(error: RegisterError) -> Self {
        match error {
            RegisterError::AlreadyRegistered => Status::already_exists(error.to_string()),
            RegisterError::Closed => Status::unavailable(error.to_string()),
            _ => Status::invalid_argument(error.to_string()),
        }
    }
}

/// A live registration's handle, held by its stream task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    execution_id: String,
    connection: u64,
}

struct Owner {
    connection: u64,
    info: ManagedTargetInfo,
    deliveries: ControlSender,
    pending: HashMap<String, oneshot::Sender<SteeringReply>>,
}

#[derive(Default)]
struct RouterState {
    owners: HashMap<String, Owner>,
    recent: VecDeque<String>,
    recent_set: HashSet<String>,
    next_connection: u64,
    closed: bool,
}

impl RouterState {
    fn remember(&mut self, request_id: &str) {
        if self.recent.len() == RECENT_REQUEST_CAPACITY
            && let Some(oldest) = self.recent.pop_front()
        {
            self.recent_set.remove(&oldest);
        }
        self.recent.push_back(request_id.to_string());
        self.recent_set.insert(request_id.to_string());
    }
}

/// The daemon's live steering registrations. Cloning shares the same state.
#[derive(Clone)]
pub struct SteeringRouter {
    state: Arc<Mutex<RouterState>>,
    closing: Arc<tokio::sync::watch::Sender<bool>>,
}

impl Default for SteeringRouter {
    fn default() -> Self {
        Self { state: Arc::default(), closing: Arc::new(tokio::sync::watch::channel(false).0) }
    }
}

impl std::fmt::Debug for SteeringRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SteeringRouter").field("owners", &self.lock().owners.len()).finish()
    }
}

fn binding(info: &ManagedTargetInfo) -> Result<&TargetBinding, RegisterError> {
    let binding = info.binding.as_ref().ok_or(RegisterError::Missing("binding"))?;
    if binding.execution_id.is_empty() {
        return Err(RegisterError::Missing("binding.execution_id"));
    }
    if binding.wrapper_start.is_empty() {
        return Err(RegisterError::Missing("binding.wrapper_start"));
    }
    if info.provider.is_empty() {
        return Err(RegisterError::Missing("provider"));
    }
    Ok(binding)
}

/// Why `expected` no longer names the owner's current binding.
fn stale_reason(current: &TargetBinding, expected: &TargetBinding) -> Option<&'static str> {
    if current.wrapper_pid != expected.wrapper_pid || current.wrapper_start != expected.wrapper_start {
        return Some("the owning wrapper process changed");
    }
    if current.generation != expected.generation || current.conversation != expected.conversation {
        return Some("the provider conversation was replaced");
    }
    None
}

fn response(outcome: RouteOutcome, detail: &str) -> RouteSteeringResponse {
    RouteSteeringResponse { outcome: outcome as i32, reply: None, detail: detail.to_string() }
}

impl SteeringRouter {
    fn lock(&self) -> std::sync::MutexGuard<'_, RouterState> {
        self.state.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    /// Registers a live owner whose stream receives `deliveries`.
    pub fn register(&self, info: ManagedTargetInfo, deliveries: ControlSender) -> Result<Registration, RegisterError> {
        let execution_id = binding(&info)?.execution_id.clone();
        let mut state = self.lock();
        if state.closed {
            return Err(RegisterError::Closed);
        }
        if state.owners.contains_key(&execution_id) {
            return Err(RegisterError::AlreadyRegistered);
        }
        state.next_connection += 1;
        let connection = state.next_connection;
        state.owners.insert(execution_id.clone(), Owner { connection, info, deliveries, pending: HashMap::new() });
        tracing::debug!(execution = %execution_id, "steering owner registered");
        Ok(Registration { execution_id, connection })
    }

    /// Replaces a registration's description. The execution and wrapper
    /// identity are fixed for its lifetime; the generation only advances.
    pub fn update(&self, registration: &Registration, info: ManagedTargetInfo) -> Result<(), RegisterError> {
        let next = binding(&info)?.clone();
        let mut state = self.lock();
        let Some(owner) = state.owners.get_mut(&registration.execution_id).filter(|o| o.connection == registration.connection)
        else {
            return Ok(());
        };
        let current = owner.info.binding.clone().unwrap_or_default();
        if next.execution_id != current.execution_id
            || next.wrapper_pid != current.wrapper_pid
            || next.wrapper_start != current.wrapper_start
        {
            return Err(RegisterError::IdentityChanged);
        }
        if next.generation < current.generation {
            return Err(RegisterError::GenerationRegressed);
        }
        owner.info = info;
        Ok(())
    }

    /// Hands an owner's reply to the request waiting for it. A reply for an
    /// unknown or already-resolved request is dropped: it can never become a
    /// new result for someone else.
    pub fn complete(&self, registration: &Registration, reply: SteeringReply) {
        let mut state = self.lock();
        let waiting = state
            .owners
            .get_mut(&registration.execution_id)
            .filter(|owner| owner.connection == registration.connection)
            .and_then(|owner| owner.pending.remove(&reply.request_id));
        if let Some(waiting) = waiting {
            let _ = waiting.send(reply);
        }
    }

    /// Removes the registration. Its unanswered deliveries resolve as
    /// unknown for their requesters, since each may have been submitted.
    pub fn unregister(&self, registration: &Registration) {
        let mut state = self.lock();
        if state.owners.get(&registration.execution_id).is_some_and(|owner| owner.connection == registration.connection) {
            state.owners.remove(&registration.execution_id);
            tracing::debug!(execution = %registration.execution_id, "steering owner unregistered");
        }
    }

    /// Ends every owner's stream and refuses new registrations. Called when
    /// the daemon shuts down: a control stream is long-lived, so without this
    /// a graceful shutdown would wait on it forever. In-flight requests
    /// resolve as unknown.
    pub fn close(&self) {
        let mut state = self.lock();
        state.closed = true;
        state.owners.clear();
        self.closing.send_replace(true);
    }

    /// Resolves once [`Self::close`] has run. A stream task stops reading its
    /// owner's upstream then, so an owner that never closes its side (a
    /// suspended process) cannot hold the transport open.
    pub async fn closed(&self) {
        let mut closing = self.closing.subscribe();
        let _ = closing.wait_for(|closed| *closed).await;
    }

    /// Every live registration, ordered by execution ID.
    pub fn list(&self) -> Vec<ManagedTargetInfo> {
        let state = self.lock();
        let mut targets: Vec<_> = state.owners.values().map(|owner| owner.info.clone()).collect();
        targets.sort_by(|a, b| {
            let id = |info: &ManagedTargetInfo| info.binding.as_ref().map(|b| b.execution_id.clone());
            id(a).cmp(&id(b))
        });
        targets
    }

    /// Routes one request to its owner and waits, at most `deadline_ms`,
    /// for the owner's reply.
    pub async fn route(&self, request: RouteSteeringRequest) -> Result<RouteSteeringResponse, Status> {
        let delivery = request.delivery.ok_or_else(|| Status::invalid_argument("missing `delivery`"))?;
        let expected = delivery.expected.clone().ok_or_else(|| Status::invalid_argument("missing `delivery.expected`"))?;
        if delivery.request_id.is_empty() {
            return Err(Status::invalid_argument("missing `delivery.request_id`"));
        }
        if request.deadline_ms == 0 || request.deadline_ms > MAX_ROUTE_DEADLINE_MS {
            return Err(Status::invalid_argument(format!("`deadline_ms` must be 1..={MAX_ROUTE_DEADLINE_MS}")));
        }
        let request_id = delivery.request_id.clone();

        let (reply_tx, reply_rx) = oneshot::channel();
        {
            let mut state = self.lock();
            let Some(owner) = state.owners.get(&expected.execution_id) else {
                return Ok(response(RouteOutcome::NoOwner, "no live owner is registered for this execution"));
            };
            let current = owner.info.binding.clone().unwrap_or_default();
            if state.recent_set.contains(&request_id) {
                return Ok(response(RouteOutcome::DuplicateRequest, "this request ID was already routed"));
            }
            if let Some(reason) = stale_reason(&current, &expected) {
                return Ok(response(RouteOutcome::StaleTarget, reason));
            }
            let frame = SteeringControlDown { kind: Some(steering_control_down::Kind::Delivery(Box::new(delivery))) };
            let owner = state.owners.get_mut(&expected.execution_id).expect("owner looked up under the same lock");
            match owner.deliveries.try_send(Ok(frame)) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    return Ok(response(RouteOutcome::Busy, "the owner's delivery channel is full"));
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Ok(response(RouteOutcome::NoOwner, "the owner's control stream has closed"));
                }
            }
            owner.pending.insert(request_id.clone(), reply_tx);
            state.remember(&request_id);
        }

        match tokio::time::timeout(Duration::from_millis(u64::from(request.deadline_ms)), reply_rx).await {
            Ok(Ok(reply)) => Ok(RouteSteeringResponse {
                outcome: RouteOutcome::OwnerReplied as i32,
                reply: Some(reply),
                detail: String::new(),
            }),
            Ok(Err(_)) => Ok(response(
                RouteOutcome::Unknown,
                "the owner disconnected before replying; the request may have been submitted",
            )),
            Err(_) => {
                let mut state = self.lock();
                if let Some(owner) = state.owners.get_mut(&expected.execution_id) {
                    owner.pending.remove(&request_id);
                }
                Ok(response(
                    RouteOutcome::Unknown,
                    "no owner reply before the deadline; the request may have been submitted",
                ))
            }
        }
    }
}
