//! Process-wide tracker for in-flight outbound deliveries.
//!
//! Every task that sends a message or a desktop notification is started
//! through [`track`], which records its `JoinHandle` under a
//! [`DeliveryLabel`]. A program that is about to exit calls
//! [`drain_deliveries`] to let those tasks finish, within a deadline, before
//! the process takes them down with it. The `claudine` CLI does this on every
//! ordinary exit; an embedding program opts in by calling it itself.
//!
//! ## Contract
//!
//! - Any helper in `messaging` that starts a delivery task must go through
//!   [`track`]. A bare `tokio::spawn` there would let the task die silently
//!   at exit. `lib/tests/l1/messaging_spawn_guard.rs` fails on any spawn in
//!   `src/messaging/` other than the one inside [`track`].
//! - A label is either a route *name* or the fixed desktop-notification
//!   label. It can never hold a URL, token, image path, or message body, so a
//!   pending-delivery warning cannot leak a secret.
//! - The registry lock is held only to push, take, or prune entries: never
//!   across an `.await` and never while a warning renders.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::Instant;

use super::send::{prose_escape, report_delivery_panic};

/// The longest the CLI waits, in total, for pending deliveries at exit.
///
/// One budget covers every pending delivery; it is not per delivery. A
/// healthy webhook post finishes well inside it; the cap only keeps an
/// unreachable route from holding a finished run open.
pub const DELIVERY_DRAIN_BUDGET: Duration = Duration::from_secs(10);

/// What an in-flight delivery is sending to, safe to show the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryLabel {
    /// A message sent over the messaging route with this name.
    Route(String),
    /// A local desktop notification.
    DesktopNotification,
}

impl fmt::Display for DeliveryLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Route(name) => write!(f, "route {name}"),
            Self::DesktopNotification => f.write_str("desktop notification"),
        }
    }
}

/// The result of [`drain_deliveries`].
#[derive(Debug, Default)]
pub struct DrainOutcome {
    /// Deliveries still running at the deadline, in registration order.
    ///
    /// Each was aborted, so whether its message arrived is unknown.
    pub pending: Vec<DeliveryLabel>,
    /// Deliveries whose task panicked; each was already reported on stderr.
    pub panicked: Vec<DeliveryLabel>,
}

impl DrainOutcome {
    /// Print one `Warning` status on stderr naming every pending delivery.
    ///
    /// Prints nothing when no delivery was pending.
    pub fn report(&self) {
        if let Some(body) = self.warning_prose() {
            let rendered = Status::from_prose(body)
                .state(StatusState::Warning)
                .render(&Terminal::default());
            crate::render::console::write_stderr_line(&rendered);
        }
    }

    fn warning_prose(&self) -> Option<String> {
        let (first, rest) = self.pending.split_first()?;
        let mut names = Vec::with_capacity(self.pending.len());
        for (index, label) in std::iter::once(first).chain(rest).enumerate() {
            names.push(match label {
                DeliveryLabel::Route(name) => format!(
                    "{} <blue-500>{}</blue-500>",
                    if index == 0 { "Route" } else { "route" },
                    prose_escape(name)
                ),
                DeliveryLabel::DesktopNotification if index == 0 => {
                    "Desktop notification".to_string()
                }
                DeliveryLabel::DesktopNotification => "desktop notification".to_string(),
            });
        }
        let verb = if rest.is_empty() { "was" } else { "were" };
        Some(format!(
            "{} {verb} still sending at exit; delivery is unknown",
            names.join(", ")
        ))
    }
}

struct Entry {
    label: DeliveryLabel,
    handle: JoinHandle<()>,
}

static REGISTRY: LazyLock<Mutex<Vec<Entry>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Lock the registry, recovering from poisoning: every critical section only
/// moves entries in or out of the `Vec`, so a panic cannot leave it torn.
fn registry() -> MutexGuard<'static, Vec<Entry>> {
    REGISTRY.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Start `future` as a tracked delivery task on the current Tokio runtime.
///
/// Returns at once. When no runtime is active (a parallel sequence-group
/// member thread, for example) nothing is sent: a warning is logged and the
/// call returns instead of panicking.
pub(crate) fn track<F>(label: DeliveryLabel, future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        tracing::warn!(%label, "Cannot start outbound delivery: no Tokio runtime active");
        return;
    };
    let handle = runtime.spawn(future);

    let finished = {
        let mut entries = registry();
        let (finished, live): (Vec<Entry>, Vec<Entry>) = std::mem::take(&mut *entries)
            .into_iter()
            .partition(|entry| entry.handle.is_finished());
        *entries = live;
        entries.push(Entry { label, handle });
        finished
    };
    for entry in finished {
        if let Some(Err(error)) = poll_finished(entry.handle) {
            report_join_error(&entry.label, &error);
        }
    }
}

/// Whether any tracked delivery is still running.
///
/// A program about to exit can use this to prepare only for a drain that will
/// actually wait (the CLI installs its Ctrl+C ladder only then).
pub fn has_pending_deliveries() -> bool {
    registry().iter().any(|entry| !entry.handle.is_finished())
}

/// Wait for every tracked delivery to finish, until `deadline`.
///
/// The deadline is shared by all deliveries, including any registered while
/// the drain runs. A deadline already in the past polls each delivery once
/// without waiting. Deliveries still running at the deadline are aborted,
/// removed, and listed in [`DrainOutcome::pending`]; nothing is printed for
/// them until the caller invokes [`DrainOutcome::report`]. A panicked
/// delivery is reported on stderr as it is found.
///
/// With nothing tracked this returns immediately.
pub async fn drain_deliveries(deadline: Instant) -> DrainOutcome {
    let mut outcome = DrainOutcome::default();
    loop {
        let batch = std::mem::take(&mut *registry());
        if batch.is_empty() {
            break;
        }
        for entry in batch {
            settle(entry, deadline, &mut outcome).await;
        }
        if Instant::now() >= deadline {
            // Anything registered while the last batch was awaited gets one
            // poll and no wait; the loop stops here so a task that keeps
            // registering new deliveries cannot hold the exit open.
            let late = std::mem::take(&mut *registry());
            for entry in late {
                settle(entry, deadline, &mut outcome).await;
            }
            break;
        }
    }
    outcome
}

async fn settle(mut entry: Entry, deadline: Instant, outcome: &mut DrainOutcome) {
    match tokio::time::timeout_at(deadline, &mut entry.handle).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            if report_join_error(&entry.label, &error) {
                outcome.panicked.push(entry.label);
            }
        }
        Err(_elapsed) => {
            entry.handle.abort();
            outcome.pending.push(entry.label);
        }
    }
}

/// Report a panicked delivery; returns whether `error` was a panic.
///
/// A cancelled task is not a failure: only the drain cancels deliveries, and
/// it reports those as pending instead.
fn report_join_error(label: &DeliveryLabel, error: &JoinError) -> bool {
    if error.is_panic() {
        report_delivery_panic(label);
        true
    } else {
        false
    }
}

/// The result of a handle whose task has already finished.
fn poll_finished(mut handle: JoinHandle<()>) -> Option<Result<(), JoinError>> {
    match Pin::new(&mut handle).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(result) => Some(result),
        Poll::Pending => None,
    }
}

#[cfg(test)]
pub(crate) fn tracked_count() -> usize {
    registry().len()
}

#[cfg(test)]
mod tests;
