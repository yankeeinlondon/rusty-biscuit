//! Structured log parsing for provider stderr streams.
//!
//! OpenCode is the first (and currently only) provider that emits
//! structured log records on stderr when launched with `--print-logs
//! --log-level ERROR`. This module holds pure parsing and classification
//! helpers so the wrapper layer can consume stderr log lines without
//! pulling provider-specific logic out of the CLI.

pub mod opencode;

pub use opencode::StderrIngestOutcome;

use crate::stream::summary::StreamExecutionSummary;

/// Provider-agnostic stderr bridge interface.
///
/// Implemented by provider-specific bridges (today only
/// [`opencode::OpenCodeLogBridge`]) so the CLI's stderr thread can ingest
/// one line at a time without knowing which provider produced it. The
/// bridge decides whether a classified line should be suppressed from raw
/// passthrough by returning [`StderrIngestOutcome::Consumed`].
pub trait StderrLogBridge: Send {
    /// Consume one stderr line and return whether the bridge absorbed it.
    fn ingest(&mut self, line: &str) -> StderrIngestOutcome;
}

impl<S> StderrLogBridge for opencode::OpenCodeLogBridge<S>
where
    S: crate::stream::semantic::SemanticEventSink,
{
    fn ingest(&mut self, line: &str) -> StderrIngestOutcome {
        opencode::OpenCodeLogBridge::ingest(self, line)
    }
}

/// Callback type invoked after the stderr thread has joined so the wrapper
/// can merge stderr-derived diagnostics into the final
/// [`StreamExecutionSummary`].
///
/// Typically captures an `Arc<Mutex<_>>` clone of the bridge's internal
/// shared state so the caller can read accumulated counters without
/// needing access to the bridge itself.
pub type SummaryFinalizer = Box<dyn FnOnce(&mut StreamExecutionSummary) + Send>;

/// Handle passed into the structured stream executor for stderr-aware runs.
///
/// The `bridge` is moved into the stderr reader thread where it classifies
/// one line at a time; the `finalize` closure is invoked after the stderr
/// thread has joined so the wrapper can merge accumulated stderr-side
/// state into the final summary. The two pieces are separate because the
/// bridge is owned by the thread while the summary is owned by the main
/// execution path.
pub struct StderrBridgeHandle {
    pub bridge: Box<dyn StderrLogBridge>,
    pub finalize: SummaryFinalizer,
}
