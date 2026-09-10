//! Early-termination signaling.
//!
//! [`fire_early_termination`] is the single choke point where the bridge
//! requests that `run_child_stream_semantic` terminate the child process
//! early. It is idempotent (guarded by `early_terminate_fired`) and also
//! mirrors the termination into the run's signal pipeline.

use tracing::warn;

use crate::signals::SignalSource;
use crate::stream::semantic::SemanticEventSink;

use super::{EarlyTermination, OpenCodeLogBridge};

impl<S: SemanticEventSink> OpenCodeLogBridge<S> {
    /// Fire an [`EarlyTermination`] signal exactly once. Subsequent calls
    /// are no-ops — the bridge never requests more than one early
    /// termination per run, even if multiple termination conditions are
    /// observed.
    pub(super) fn fire_early_termination(&mut self, termination: EarlyTermination) {
        if self.early_terminate_fired {
            return;
        }
        self.early_terminate_fired = true;
        // Bespoke signal mirror at the single choke point where a
        // termination fires. The CLI's post-wait synthesis emits the same
        // event; the sink's correlation window folds the double-fire.
        if let Some(hub) = &self.signal_hub {
            hub.emit_bespoke(
                termination.to_signal_event(),
                SignalSource::StderrPromoted,
            );
        }
        let Some(sender) = self.early_terminate.as_ref() else {
            return;
        };
        if let Err(err) = sender.send(termination) {
            warn!(
                error = %err,
                "failed to deliver OpenCode early-termination signal; receiver dropped",
            );
        }
    }
}
