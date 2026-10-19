//! What one attempt's observation measured, for the report and for a
//! startup failure that has no report yet: bounded to one value per field,
//! with no per-drain samples.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::CoverageLost;

/// The native adapter that observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationBackend {
    /// macOS.
    FsEvents,
    /// Linux, including WSL2.
    Inotify,
    /// Windows.
    ReadDirectoryChanges,
}

impl ObservationBackend {
    /// This platform's adapter. Only the scripted test observer asks.
    #[cfg(test)]
    pub(crate) const NATIVE: Self = if cfg!(target_os = "macos") {
        Self::FsEvents
    } else if cfg!(windows) {
        Self::ReadDirectoryChanges
    } else {
        Self::Inotify
    };
}

/// Where in the removal a drain first returned the loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStage {
    /// Confirming that the new watch delivers.
    Startup,
    /// The checks before the move.
    PreMove,
    /// The checks after the move (or, on a resumed attempt, before its
    /// deletion).
    PostMove,
    Deletion,
    Unregistration,
}

/// The session's first coverage loss and where it was received.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationLoss {
    pub stage: ObservationStage,
    pub reason: CoverageLost,
    /// [`ObservationStage::Deletion`] only: entries the deletion engine
    /// processed since its previous drain. Timing context, not evidence that
    /// the removal caused a system-wide drop.
    pub entries_since_drain: Option<u64>,
}

/// One attempt's observation, as persisted in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationDiagnostics {
    pub backend: ObservationBackend,
    /// Starting the watch, including confirming that it delivers; `None` for
    /// a session this attempt did not time.
    pub startup_ms: Option<u64>,
    /// The latest and the longest confirmed marker round trip: from writing a
    /// marker to its acknowledgment. They time markers, not every watched
    /// change, and are `None` until a marker is confirmed (always on Linux,
    /// which reads until empty instead).
    pub last_round_trip_ms: Option<u64>,
    pub longest_round_trip_ms: Option<u64>,
    pub loss: Option<ObservationLoss>,
}

impl ObservationDiagnostics {
    pub(crate) fn new(backend: ObservationBackend) -> Self {
        Self { backend, startup_ms: None, last_round_trip_ms: None, longest_round_trip_ms: None, loss: None }
    }

    pub(crate) fn set_round_trips(&mut self, round_trips: RoundTrips) {
        self.last_round_trip_ms = round_trips.last.map(millis);
        self.longest_round_trip_ms = round_trips.longest.map(millis);
    }
}

/// An adapter's confirmed marker round trips.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RoundTrips {
    pub last: Option<Duration>,
    pub longest: Option<Duration>,
}

impl RoundTrips {
    pub fn record(&mut self, round_trip: Duration) {
        self.last = Some(round_trip);
        self.longest = Some(self.longest.map_or(round_trip, |longest| longest.max(round_trip)));
    }
}

/// A live view of a session's [`ObservationDiagnostics`] that outlives the
/// session, so a report saved after the observer is dropped still has them.
#[derive(Debug, Clone)]
pub struct DiagnosticsHandle(Arc<Mutex<ObservationDiagnostics>>);

impl DiagnosticsHandle {
    pub(crate) fn new(diagnostics: ObservationDiagnostics) -> Self {
        Self(Arc::new(Mutex::new(diagnostics)))
    }

    pub fn snapshot(&self) -> ObservationDiagnostics {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub(crate) fn update(&self, change: impl FnOnce(&mut ObservationDiagnostics)) {
        change(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner));
    }
}

pub(crate) fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// A [`Duration`] persisted as whole milliseconds.
pub(crate) mod duration_ms {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(super::millis(*duration))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        u64::deserialize(deserializer).map(Duration::from_millis)
    }
}
