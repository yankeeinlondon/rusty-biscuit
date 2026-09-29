//! Best-effort discovery of steerable sessions for the current user.
//!
//! Two kinds of source feed one listing:
//!
//! - **Managed** — live control registrations of Claudine-owned executions,
//!   as the local Rendezvous daemon holds them ([`ManagedSource`]). Each row
//!   already carries its owner's availability verdict.
//! - **Native** — provider-specific discoverers, one per generated research
//!   discovery method ([`NativeDiscoverer`]). A researched native method this
//!   build does not implement is reported as a [`DiscoveryGap`], not an error.
//!
//! Sources run concurrently under one [`DISCOVERY_DEADLINE`], with at most
//! [`MAX_CONCURRENT_PROVIDERS`] provider tasks in flight. A source that fails
//! or times out becomes a [`DiscoveryError`] beside the sessions the others
//! found; only when every source fails does discovery fail. Discovery never
//! starts a provider session, and neither session history nor replicated mesh
//! presence is an input: neither proves local ownership, liveness, or a
//! writable channel.
//!
//! Observations merge only when their identities establish the same session:
//! an identical target ID, or a native observation whose provider process and
//! conversation equal a managed registration's. Merged rows keep the managed
//! ID (routed through its owner) and the union of their sources. Rows sort by
//! research-roster order, then directory, then full ID.
//!
//! Topic: `claudine/docs/topics/steering-routing.md`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::eligibility::{AvailabilitySummary, SessionFacts};
use super::identity::{ManagedTarget, ProcessStartIdentity, SteeringTargetId};
use super::vocabulary::{DiscoveryMethod, ExecutionState, HostOs, LaunchMode, LaunchOrigin, SteeringAvailability};
use crate::provider_id::Provider;

#[cfg(test)]
mod tests;

/// Deadline for a whole discovery pass.
pub const DISCOVERY_DEADLINE: Duration = Duration::from_secs(5);
/// Provider discovery tasks in flight at once.
pub const MAX_CONCURRENT_PROVIDERS: usize = 4;

/// Which kind of source observed a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    Managed,
    Native,
}

/// One listed session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionListing {
    pub id: SteeringTargetId,
    #[serde(serialize_with = "super::serialize_slug")]
    pub provider: Provider,
    pub name: Option<String>,
    pub cwd: Option<String>,
    pub state: ExecutionState,
    /// Sources that observed this session, sorted and unique.
    pub origins: Vec<ObservationSource>,
    pub launch_profile: Option<String>,
    pub provider_version: Option<String>,
    #[serde(flatten)]
    pub availability: AvailabilitySummary,
    pub observed_at: DateTime<Utc>,
    /// The provider process and conversation, when known. Two observations
    /// with equal values are the same session.
    #[serde(skip)]
    pub session_key: Option<(ProcessStartIdentity, String)>,
    /// The owner's binding when this row was listed (managed rows only). A
    /// request routed from this row names it as `expected`, so a wrapper
    /// restart or conversation switch since listing is refused, not followed.
    #[serde(skip)]
    pub binding: Option<ManagedTarget>,
}

/// What one native discoverer saw of one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeObservation {
    pub process: ProcessStartIdentity,
    pub conversation: String,
    pub name: Option<String>,
    pub cwd: Option<String>,
    /// `Unknown` unless the provider reliably distinguishes working and idle.
    pub state: ExecutionState,
    pub launch_profile: Option<String>,
    pub launch_mode: LaunchMode,
    pub provider_version: Option<String>,
}

/// Why a discovery source (or one of its rows) failed, with its typed cause.
pub type SourceError = Box<dyn std::error::Error + Send + Sync>;

/// A discovery in progress.
pub type DiscoveryFuture<T> = Pin<Box<dyn Future<Output = Result<T, SourceError>> + Send>>;

/// Readable managed registrations, plus one problem per registration that
/// could not be read. A malformed row is reported, never silently dropped.
#[derive(Debug, Default)]
pub struct ManagedListing {
    pub sessions: Vec<SessionListing>,
    pub problems: Vec<SourceError>,
}

/// The local daemon's managed control registrations.
pub trait ManagedSource: Send + Sync {
    fn list(&self) -> DiscoveryFuture<ManagedListing>;
}

/// One provider-specific implementation of a researched discovery method.
pub trait NativeDiscoverer: Send + Sync {
    fn provider(&self) -> Provider;
    /// The generated research discovery ID this implements.
    fn discovery_id(&self) -> &'static str;
    fn discover(&self) -> DiscoveryFuture<Vec<NativeObservation>>;
}

/// A source did not finish before the discovery deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("discovery did not finish within {} ms", .deadline.as_millis())]
pub struct DiscoveryTimedOut {
    pub deadline: Duration,
}

/// A source that failed; the other sources' sessions are still listed.
#[derive(Debug, Clone)]
pub struct DiscoveryError {
    /// `managed`, or the provider slug.
    pub source: String,
    cause: Arc<dyn std::error::Error + Send + Sync>,
}

impl DiscoveryError {
    pub fn new(source: impl Into<String>, cause: SourceError) -> Self {
        Self { source: source.into(), cause: Arc::from(cause) }
    }

    /// The typed cause.
    pub fn cause(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self.cause.as_ref()
    }

    /// The cause and its source chain as one secret-masked line: the
    /// listing's rendering of this error.
    pub fn message(&self) -> String {
        let mut text = self.cause.to_string();
        let mut next = self.cause.source();
        while let Some(cause) = next {
            text.push_str(": ");
            text.push_str(&cause.to_string());
            next = cause.source();
        }
        crate::secrets::mask_secrets(&text).into_owned()
    }
}

impl PartialEq for DiscoveryError {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.message() == other.message()
    }
}

impl Eq for DiscoveryError {}

impl Serialize for DiscoveryError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut record = serializer.serialize_struct("DiscoveryError", 2)?;
        record.serialize_field("source", &self.source)?;
        record.serialize_field("message", &self.message())?;
        record.end()
    }
}

/// A researched native discovery method this build does not implement, so
/// its sessions cannot be listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveryGap {
    #[serde(serialize_with = "super::serialize_slug")]
    pub provider: Provider,
    pub discovery_id: &'static str,
    pub method: DiscoveryMethod,
}

/// The result of one discovery pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveryReport {
    pub sessions: Vec<SessionListing>,
    pub errors: Vec<DiscoveryError>,
    pub gaps: Vec<DiscoveryGap>,
}

/// Every source that ran failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("session discovery failed for every source")]
pub struct DiscoveryFailed {
    pub errors: Vec<DiscoveryError>,
}

/// Discovers sessions with the production bounds.
pub async fn discover(
    managed: Option<Arc<dyn ManagedSource>>,
    native: &[Arc<dyn NativeDiscoverer>],
) -> Result<DiscoveryReport, DiscoveryFailed> {
    discover_with(managed, native, super::host_os(), DISCOVERY_DEADLINE).await
}

/// [`discover`] with an explicit host OS and deadline.
pub async fn discover_with(
    managed: Option<Arc<dyn ManagedSource>>,
    native: &[Arc<dyn NativeDiscoverer>],
    os: HostOs,
    deadline: Duration,
) -> Result<DiscoveryReport, DiscoveryFailed> {
    let budget = deadline;
    let deadline = tokio::time::Instant::now() + budget;
    let limit = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_PROVIDERS));
    let mut tasks = tokio::task::JoinSet::new();

    let mut ran = 0usize;
    if let Some(managed) = managed {
        ran += 1;
        tasks.spawn(async move {
            let outcome = bounded(deadline, budget, managed.list()).await;
            ("managed".to_string(), outcome.map(|listing| (listing.sessions, listing.problems)))
        });
    }
    for provider in super::roster_order().iter().copied() {
        let discoverers: Vec<_> = native.iter().filter(|d| d.provider() == provider).cloned().collect();
        if discoverers.is_empty() {
            continue;
        }
        ran += 1;
        let limit = Arc::clone(&limit);
        tasks.spawn(async move {
            let outcome = async {
                let _permit = limit.acquire_owned().await?;
                let mut rows = Vec::new();
                for discoverer in discoverers {
                    let observations = discoverer.discover().await?;
                    rows.extend(observations.into_iter().map(|o| native_listing(provider, os, o)));
                }
                Ok((rows, Vec::new()))
            };
            (provider.as_slug().to_string(), bounded(deadline, budget, outcome).await)
        });
    }

    let mut sessions = Vec::new();
    let mut errors = Vec::new();
    let mut failed = 0usize;
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok((source, Ok((rows, problems)))) => {
                sessions.extend(rows);
                errors.extend(problems.into_iter().map(|problem| DiscoveryError::new(source.clone(), problem)));
            }
            Ok((source, Err(cause))) => {
                failed += 1;
                errors.push(DiscoveryError::new(source, cause));
            }
            Err(join) => {
                failed += 1;
                errors.push(DiscoveryError::new("discovery", Box::new(join)));
            }
        }
    }
    errors.sort_by(|a, b| a.source.cmp(&b.source));
    if ran > 0 && failed == ran {
        return Err(DiscoveryFailed { errors });
    }
    let mut sessions = deduplicate(sessions);
    sort(&mut sessions);
    Ok(DiscoveryReport { sessions, errors, gaps: gaps(native, os) })
}

async fn bounded<T>(
    deadline: tokio::time::Instant,
    budget: Duration,
    work: impl Future<Output = Result<T, SourceError>>,
) -> Result<T, SourceError> {
    match tokio::time::timeout_at(deadline, work).await {
        Ok(outcome) => outcome,
        Err(_) => Err(Box::new(DiscoveryTimedOut { deadline: budget })),
    }
}

fn native_listing(provider: Provider, os: HostOs, observation: NativeObservation) -> SessionListing {
    let availability = match observation.launch_profile.as_deref() {
        Some(profile_id) => super::eligibility::evaluate(&SessionFacts {
            provider,
            profile_id,
            os,
            launch_mode: observation.launch_mode,
            origin: LaunchOrigin::Native,
            state: observation.state,
            provider_version: observation.provider_version.as_deref(),
        })
        .manual
        .summary(),
        None => AvailabilitySummary::unavailable("the session's launch profile could not be established"),
    };
    SessionListing {
        id: SteeringTargetId::Native {
            provider,
            process: observation.process.clone(),
            conversation: observation.conversation.clone(),
        },
        provider,
        name: observation.name,
        cwd: observation.cwd,
        state: observation.state,
        origins: vec![ObservationSource::Native],
        launch_profile: observation.launch_profile,
        provider_version: observation.provider_version,
        availability,
        observed_at: Utc::now(),
        session_key: Some((observation.process, observation.conversation)),
        binding: None,
    }
}

/// Merges observations the identities prove are one session.
fn deduplicate(observations: Vec<SessionListing>) -> Vec<SessionListing> {
    // Managed rows first, so a native duplicate folds into the routable row.
    let (mut merged, native): (Vec<_>, Vec<_>) =
        observations.into_iter().partition(|row| matches!(row.id, SteeringTargetId::Managed { .. }));
    let fold = |merged: &mut Vec<SessionListing>, row: SessionListing| {
        let same = merged.iter_mut().find(|existing| {
            existing.id == row.id
                || (existing.provider == row.provider
                    && existing.session_key.is_some()
                    && existing.session_key == row.session_key)
        });
        match same {
            Some(existing) => {
                existing.origins.extend(row.origins);
                existing.origins.sort();
                existing.origins.dedup();
            }
            None => merged.push(row),
        }
    };
    let managed = std::mem::take(&mut merged);
    for row in managed.into_iter().chain(native) {
        fold(&mut merged, row);
    }
    merged
}

fn sort(sessions: &mut [SessionListing]) {
    let rank = |provider: Provider| {
        super::roster_order().iter().position(|p| *p == provider).unwrap_or(usize::MAX)
    };
    sessions.sort_by(|a, b| {
        (rank(a.provider), &a.cwd, a.id.to_string()).cmp(&(rank(b.provider), &b.cwd, b.id.to_string()))
    });
}

fn gaps(native: &[Arc<dyn NativeDiscoverer>], os: HostOs) -> Vec<DiscoveryGap> {
    let mut gaps = Vec::new();
    for provider in super::roster_order().iter().copied() {
        for record in super::facts(provider).discovery {
            let native_method = record.origin == LaunchOrigin::Native && record.method != DiscoveryMethod::ClaudineRegistration;
            let implemented = native.iter().any(|d| d.provider() == provider && d.discovery_id() == record.id);
            if record.os == os && native_method && !implemented {
                gaps.push(DiscoveryGap { provider, discovery_id: record.id, method: record.method });
            }
        }
    }
    gaps
}

impl SessionListing {
    /// Whether the listing row can be selected for steering.
    pub fn is_selectable(&self) -> bool {
        self.availability.availability != SteeringAvailability::Unavailable
    }
}
