//! Structured log parsing for provider stderr streams.
//!
//! OpenCode emits structured log records on stderr when launched with
//! `--print-logs --log-level ERROR`; Codex emits `tracing-subscriber`
//! formatted lines unconditionally. Both shapes are parsed here so the
//! wrapper layer can consume stderr log lines without pulling
//! provider-specific logic out of the CLI.

pub mod codex;
pub mod opencode;

pub use opencode::{
    EarlyTermination, StalledGenerationContext, StderrIngestOutcome, StuckSubagentInfo,
};

use std::sync::mpsc::Receiver;

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

impl<S> StderrLogBridge for codex::CodexLogBridge<S>
where
    S: crate::stream::semantic::SemanticEventSink,
{
    fn ingest(&mut self, line: &str) -> StderrIngestOutcome {
        codex::CodexLogBridge::ingest(self, line)
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
/// state into the final summary. The `early_terminate` receiver is polled
/// from the main wait loop so a pre-stream usage-cap failure can abort the
/// child process without blocking on `child.wait()`.
///
/// The three pieces are separate because the bridge is owned by the stderr
/// thread, the receiver is owned by the main wait loop, and the finalizer
/// runs on the main thread after both reader threads have joined.
pub struct StderrBridgeHandle {
    pub bridge: Box<dyn StderrLogBridge>,
    pub finalize: SummaryFinalizer,
    pub early_terminate: Option<Receiver<EarlyTermination>>,
}
