//! Shared stderr-side state accumulated by the bridge and merged into the
//! final execution summary.

use std::sync::Arc;
use std::sync::Mutex;

use crate::stream::badges::derive_badges;
use crate::stream::logs::opencode::classify::merge_rate_limit;
use crate::stream::summary::{RateLimitInfo, StderrDiagnostics, StreamExecutionSummary};

/// Shared stderr-side state accumulated by the bridge as it parses lines.
///
/// Held behind a `Mutex` so the bridge can be cloned cheaply across threads
/// and the main wait loop can read the accumulated diagnostics at the end
/// of the run. The bridge never mutates [`StreamExecutionSummary`]
/// directly; merging is the wrapper layer's responsibility.
#[derive(Debug, Default)]
pub struct SharedStderrState {
    pub diagnostics: StderrDiagnostics,
    pub rate_limit: Option<RateLimitInfo>,
    /// First `providerID` observed on a `service=llm ... mode=primary`
    /// stderr log line. OpenCode routes the primary turn through a single
    /// model provider per session, so the first observation is canonical.
    pub primary_provider_id: Option<String>,
    /// First `modelID` observed on a `service=llm ... mode=primary` stderr
    /// log line. Used to enrich [`StreamExecutionSummary::model`]
    /// when the stdout NDJSON stream does not include an `init` payload.
    pub primary_model_id: Option<String>,
}

impl SharedStderrState {
    /// Convenience accessor: does the shared state contain any parsed
    /// structured log records?
    pub fn any_records(&self) -> bool {
        self.diagnostics.log_records_parsed > 0
    }
}

/// Merge the bridge's accumulated [`SharedStderrState`] into a summary.
///
/// Called once by the wrapper layer after the stderr thread has joined.
/// Always sets `summary.stderr_diagnostics` when the bridge parsed at least
/// one structured log record. Always merges `summary.rate_limit` when the
/// bridge accumulated stderr-side rate-limit state. Backfills
/// `summary.model` from the first observed `mode=primary` LLM call when the
/// stdout NDJSON stream did not surface a model. Always recomputes
/// `summary.badges` via [`derive_badges`] so the
/// stderr-derived badge categories (rate-limit resets, malformed-asset
/// warnings) appear in the final output.
pub fn merge_stderr_state_into_summary(
    state: &Arc<Mutex<SharedStderrState>>,
    summary: &mut StreamExecutionSummary,
) {
    let Ok(state) = state.lock() else {
        return;
    };
    if state.any_records() {
        summary.stderr_diagnostics = Some(state.diagnostics.clone());
    }
    if let Some(stderr_rl) = state.rate_limit.clone() {
        summary.rate_limit = Some(merge_rate_limit(summary.rate_limit.clone(), stderr_rl));
    }
    if summary.model.is_none()
        && let Some(model) = state.primary_model_id.clone()
    {
        summary.model = Some(model);
    }
    drop(state);
    summary.badges = derive_badges(summary, summary.provider);
}
