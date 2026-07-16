//! Best-effort session-presence reporting to the rendezvous daemon.
//!
//! The daemon's `sessions-active/{node_id}` register is the mesh-wide
//! NOW view (dashboard spec); the wrapper is its first producer. A
//! [`SessionPresence`] guard brackets one provider child: construction
//! reports `STARTED`, and dropping it reports `ENDED` — so every exit
//! path (success, error, guard trips) clears the entry without
//! per-path plumbing. `ENDED` carries no payload; it simply removes
//! the session from the register.
//!
//! Reporting is strictly best-effort: a missing (or wedged) daemon
//! must never delay or fail a wrapped session. Every call is capped at
//! [`REPORT_TIMEOUT`] and all failures degrade to a `debug!` log. When
//! the `STARTED` report did not reach a daemon, the drop is a no-op —
//! there is nothing to clear. Kill switch:
//! `CLAUDINE_RENDEZVOUS_REPORT=false`.

use std::sync::Arc;
use std::time::Duration;

use claudine::events::{AgenticEvent, EnvironmentContext};
use claudine::provider::Provider;
use serde_json::json;

/// Hard cap per report call. A missing daemon fails the connect in
/// microseconds; this bound only matters for a live-but-wedged daemon.
const REPORT_TIMEOUT: Duration = Duration::from_millis(250);

/// Kill switch (default: enabled).
const ENABLE_ENV: &str = "CLAUDINE_RENDEZVOUS_REPORT";

/// Env key the wrapper already injects into every child; reused as the
/// session identity so hook-driven records and presence agree.
const SESSION_ID_ENV: &str = "CLAUDINE_SESSION_ID";

/// Presence bracket for one provider child. See the module docs.
pub(crate) struct SessionPresence {
    /// `Some` only when the `STARTED` report reached a daemon.
    session_id: Option<String>,
}

impl SessionPresence {
    /// Report a session start and return the guard whose drop reports
    /// the matching end.
    pub(crate) fn started(
        provider: Provider,
        model: Option<&str>,
        interactive: bool,
        env_context: &EnvironmentContext,
        child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    ) -> Self {
        let disabled = Self { session_id: None };
        if !reporting_enabled() {
            return disabled;
        }
        let Some(handle) = usable_runtime_handle() else {
            return disabled;
        };
        let session_id = child_env
            .get(std::ffi::OsStr::new(SESSION_ID_ENV))
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut details = serde_json::Map::new();
        details.insert("agent".into(), json!(provider.as_slug()));
        details.insert("interactive".into(), json!(interactive));
        // Record whether this provider can even surface a permission
        // signal, so the dashboard can tell "no intervention needed"
        // apart from "signal unavailable for this agent." Sourced from
        // the same capability matrix `claudine hooks --support` renders.
        let permission_signal = if provider.supports_event(&AgenticEvent::PermissionRequest) {
            "supported"
        } else {
            "unsupported"
        };
        details.insert("permission_signal".into(), json!(permission_signal));
        if let Some(model) = model {
            details.insert("model".into(), json!(model));
        }
        if let Some(repo) = env_context.repo.as_ref() {
            details.insert("repo_root".into(), json!(repo.root.display().to_string()));
        }
        if let Some(pid) = env_context.claudine_pid {
            details.insert("claudine_pid".into(), json!(pid));
        }

        // The initial status opinion: the lifecycle boundary asserts the
        // session is Active. Routed through the typed slot so the daemon
        // reducer owns the projected `status`.
        let lifecycle_active = rendezvous_core::proto::StatusContribution {
            state: rendezvous_core::SessionStatusState::Active as i32,
            producer: rendezvous_core::StatusProducer::Lifecycle as i32,
            basis: rendezvous_core::SessionStatusBasis::Lifecycle as i32,
            revision: revision_now(),
        };

        let sent = block_report(
            &handle,
            session_id.clone(),
            rendezvous_core::SessionEventKind::Started,
            details,
            Some(lifecycle_active),
        );
        Self {
            session_id: sent.then_some(session_id),
        }
    }
}

impl SessionPresence {
    /// A cheap, cloneable handle for mid-session status transitions
    /// (dashboard trigger 1). Active only when the `STARTED` report
    /// reached a daemon *and* a usable runtime handle exists; otherwise
    /// every `report` is a no-op. The runtime handle is captured now —
    /// on the runtime thread that ran `started` — so a later `report`
    /// works even when called from an off-runtime stream-reader thread.
    pub(crate) fn status_reporter(&self) -> StatusReporter {
        match (self.session_id.clone(), usable_runtime_handle()) {
            (Some(session_id), Some(handle)) => StatusReporter {
                inner: Some((Arc::from(session_id.as_str()), handle)),
            },
            _ => StatusReporter { inner: None },
        }
    }
}

/// Fire-and-forget reporter for `UPDATED` status transitions on a live
/// session. Unlike the `STARTED`/`ENDED` bracket it never blocks: each
/// `report` spawns a detached, timeout-bounded task so a wedged daemon
/// cannot stall the stream render path it is called from.
#[derive(Clone)]
pub(crate) struct StatusReporter {
    /// `(session_id, runtime handle)` when reporting is live.
    inner: Option<(Arc<str>, tokio::runtime::Handle)>,
}

impl StatusReporter {
    /// A reporter that does nothing — the default before a session is
    /// bracketed, and whenever presence reporting is unavailable.
    pub(crate) fn inert() -> Self {
        Self { inner: None }
    }

    /// Report a status transition (e.g. `waiting_on_user`, `active`) as
    /// the fallback SINK producer's slot contribution, stamped with a
    /// producer-captured unix-ns revision the daemon LWW-guards. No-op
    /// when inert.
    pub(crate) fn report(&self, status: &'static str) {
        let Some((session_id, handle)) = self.inner.clone() else {
            return;
        };
        let contribution = sink_contribution(status);
        handle.spawn(async move {
            let send = async {
                let endpoint = rendezvous_core::socket::legacy_local_endpoint(
            rendezvous_core::socket::default_socket_path(),
        );
                let mut client = rendezvous_client::connect(&endpoint).await?;
                client
                    .report_session_event(rendezvous_core::ReportSessionEventRequest {
                        session_id: session_id.to_string(),
                        kind: rendezvous_core::SessionEventKind::Updated as i32,
                        details_json: String::new(),
                        status: Some(contribution),
                    })
                    .await?;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            };
            match tokio::time::timeout(REPORT_TIMEOUT, send).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => tracing::debug!(
                    target: "claudine::session_report",
                    %err,
                    status,
                    "session status update skipped (daemon unreachable)",
                ),
                Err(_) => tracing::debug!(
                    target: "claudine::session_report",
                    status,
                    "session status update timed out",
                ),
            }
        });
    }
}

impl Drop for SessionPresence {
    fn drop(&mut self) {
        let Some(session_id) = self.session_id.take() else {
            return;
        };
        let Some(handle) = usable_runtime_handle() else {
            return;
        };
        block_report(
            &handle,
            session_id,
            rendezvous_core::SessionEventKind::Ended,
            serde_json::Map::new(),
            None,
        );
    }
}

/// Producer-captured unix-ns wall clock used as a status revision. The
/// daemon rejects a slot contribution whose revision is `<=` the one it
/// already stored, so a reordered same-producer report cannot win.
fn revision_now() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

/// Map a legacy status string into the SINK producer's typed
/// contribution. The sink is the fallback permission signal for
/// providers whose hooks do not expose the ask; `waiting_on_user`
/// carries a permission-ask basis, anything else clears to Active.
fn sink_contribution(status: &str) -> rendezvous_core::proto::StatusContribution {
    let (state, basis) = if status == "waiting_on_user" {
        (
            rendezvous_core::SessionStatusState::WaitingOnUser,
            rendezvous_core::SessionStatusBasis::PermissionAsk,
        )
    } else {
        (
            rendezvous_core::SessionStatusState::Active,
            rendezvous_core::SessionStatusBasis::Lifecycle,
        )
    };
    rendezvous_core::proto::StatusContribution {
        state: state as i32,
        producer: rendezvous_core::StatusProducer::Sink as i32,
        basis: basis as i32,
        revision: revision_now(),
    }
}

/// Best-effort report of the interactive idle producer's (Trigger 2)
/// status transition to the local rendezvous daemon, addressed by an
/// explicit `session_id`.
///
/// This is the reporter the hook path (`claudine handle`) uses: a wrapped
/// interactive session reports `idle` when a turn completes and `active`
/// when the next prompt arrives. Reported as `UPDATED`, so a report for a
/// session the wrapper never STARTED (or already ENDED) is a daemon-side
/// no-op — it never resurrects a phantom session. Unlike [`StatusReporter`]
/// (which spawns a detached task off a hot stream-render path), `handle`
/// is already async and single-shot, so this awaits directly — no
/// `block_in_place`, no runtime-flavor gate. Every failure degrades to a
/// `debug!`; a missing or wedged daemon must never delay the hook. Honors
/// the `CLAUDINE_RENDEZVOUS_REPORT` kill switch.
// Wired into the hook path (`claudine handle`) — the interactive idle
// producer (Trigger 2) calls this on turn-complete / next-prompt.
pub(crate) async fn report_status(session_id: &str, status: &str) {
    if !reporting_enabled() || session_id.is_empty() {
        return;
    }
    let contribution = idle_hook_contribution(status);
    let send = async {
        let endpoint = rendezvous_core::socket::legacy_local_endpoint(
            rendezvous_core::socket::default_socket_path(),
        );
        let mut client = rendezvous_client::connect(&endpoint).await?;
        client
            .report_session_event(rendezvous_core::ReportSessionEventRequest {
                session_id: session_id.to_string(),
                kind: rendezvous_core::SessionEventKind::Updated as i32,
                details_json: String::new(),
                status: Some(contribution),
            })
            .await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    };
    match tokio::time::timeout(REPORT_TIMEOUT, send).await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => tracing::debug!(
            target: "claudine::session_report",
            %err,
            status,
            "idle-hook status report skipped (daemon unreachable)",
        ),
        Err(_) => tracing::debug!(
            target: "claudine::session_report",
            status,
            "idle-hook status report timed out",
        ),
    }
}

/// Map the idle producer's status string into its typed `IdleHook` slot
/// contribution. `idle` carries the interactive-turn-complete basis;
/// anything else (i.e. `active`) clears the `IdleHook` slot back to
/// Active. Writing its own slot means this weaker `idle` can never
/// clobber an unresolved `waiting_on_user` — the reducer's precedence
/// (`waiting_on_user` > `idle` > `active`) resolves that.
fn idle_hook_contribution(status: &str) -> rendezvous_core::proto::StatusContribution {
    let (state, basis) = if status == "idle" {
        (
            rendezvous_core::SessionStatusState::Idle,
            rendezvous_core::SessionStatusBasis::InteractiveTurnComplete,
        )
    } else {
        (
            rendezvous_core::SessionStatusState::Active,
            rendezvous_core::SessionStatusBasis::Lifecycle,
        )
    };
    rendezvous_core::proto::StatusContribution {
        state: state as i32,
        producer: rendezvous_core::StatusProducer::IdleHook as i32,
        basis: basis as i32,
        revision: revision_now(),
    }
}

/// A handle we can safely `block_in_place` on: `block_in_place` panics
/// on a current-thread runtime (the wrapper's main runtime is
/// multi-thread, but unit tests and embedders may not be).
fn usable_runtime_handle() -> Option<tokio::runtime::Handle> {
    let handle = tokio::runtime::Handle::try_current().ok()?;
    (handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread).then_some(handle)
}

fn reporting_enabled() -> bool {
    match std::env::var(ENABLE_ENV) {
        Ok(raw) => !matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        ),
        Err(_) => true,
    }
}

/// Send one report, bounded by [`REPORT_TIMEOUT`]. Returns whether the
/// daemon acknowledged it.
fn block_report(
    handle: &tokio::runtime::Handle,
    session_id: String,
    kind: rendezvous_core::SessionEventKind,
    details: serde_json::Map<String, serde_json::Value>,
    status: Option<rendezvous_core::proto::StatusContribution>,
) -> bool {
    tokio::task::block_in_place(|| {
        handle.block_on(async {
            let send = async {
                let endpoint = rendezvous_core::socket::legacy_local_endpoint(
            rendezvous_core::socket::default_socket_path(),
        );
                let mut client = rendezvous_client::connect(&endpoint).await?;
                client
                    .report_session_event(rendezvous_core::ReportSessionEventRequest {
                        session_id,
                        kind: kind as i32,
                        details_json: if details.is_empty() {
                            String::new()
                        } else {
                            serde_json::Value::Object(details).to_string()
                        },
                        status,
                    })
                    .await?;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            };
            match tokio::time::timeout(REPORT_TIMEOUT, send).await {
                Ok(Ok(())) => true,
                Ok(Err(err)) => {
                    tracing::debug!(
                        target: "claudine::session_report",
                        %err,
                        "session presence report skipped (daemon unreachable)",
                    );
                    false
                }
                Err(_) => {
                    tracing::debug!(
                        target: "claudine::session_report",
                        timeout_ms = REPORT_TIMEOUT.as_millis() as u64,
                        "session presence report timed out",
                    );
                    false
                }
            }
        })
    })
}

#[cfg(test)]
mod tests;
