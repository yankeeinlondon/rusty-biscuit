//! Diagnostics scheduling tiers and the document publish path.
//!
//! The design defines three scheduling tiers; Phase 4 ships the substrate
//! subset. Because dispatch is currently synchronous (AD-3's worker pool moves
//! diagnostics off the loop thread in a later slice), the scheduler publishes
//! immediately after each re-index rather than running a debounce timer — the
//! configured [`DiagnosticsScheduler::debounce`] is carried now so the timer
//! can be introduced without a config change. Supersession is by document
//! `version`: a client drops a publish an newer edit has already replaced.

use std::time::Duration;

use crossbeam_channel::Sender;
use lsp_server::Message;
use lsp_types::{Diagnostic, Uri};

use crate::config::DmlsConfig;

use super::publisher::DiagnosticsPublisher;

/// The three diagnostic scheduling tiers (design "Diagnostics Pipeline").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTier {
    /// Single-document syntax / well-formedness, debounced on `didChange`.
    Immediate,
    /// Link/anchor validity, duplicate slugs — after re-index. The Phase-4
    /// substrate diagnostics are all this tier.
    Graph,
    /// Cross-file sweeps, on demand / on save.
    Workspace,
}

/// Owns the publisher and the configured debounce, and publishes a document's
/// diagnostics against its version.
pub struct DiagnosticsScheduler {
    publisher: DiagnosticsPublisher,
    debounce: Duration,
}

impl DiagnosticsScheduler {
    /// Builds the scheduler from the connection sender and effective config.
    pub fn new(sender: Sender<Message>, config: &DmlsConfig) -> Self {
        Self {
            publisher: DiagnosticsPublisher::new(sender),
            debounce: Duration::from_millis(config.diagnostics.debounce_ms),
        }
    }

    /// The configured immediate-tier debounce.
    pub fn debounce(&self) -> Duration {
        self.debounce
    }

    /// Publishes `diagnostics` for `uri` at `version`.
    pub fn publish(&self, uri: Uri, version: Option<i32>, diagnostics: Vec<Diagnostic>) {
        self.publisher.publish(uri, version, diagnostics);
    }
}
