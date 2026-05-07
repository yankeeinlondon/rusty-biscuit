//! Heartbeat and live-metrics wiring for [`LiveSemanticSink`].
//!
//! The 30-second flush-if-idle / heartbeat thread lives in the exec layer
//! (`exec.rs`) and polls [`LiveMetrics`] via [`LiveSemanticSink::live_metrics`].

use super::LiveSemanticSink;
use claudine::stream::progress::LiveMetrics;

impl LiveSemanticSink {
    /// Return a clone of the [`LiveMetrics`] handle for heartbeat tracking.
    pub(crate) fn live_metrics(&self) -> LiveMetrics {
        self.live_metrics.clone()
    }
}
