//! The schema triggers discovery skipped as invalid during one top-level
//! invocation, and which of them the user has already been shown.
//!
//! Claudine prepares a document's effective schema several times before a
//! provider launches (pre-validation, optional-value cleanup, the launch
//! schema, drop-and-retry legs, status reports, interactive re-runs, and every
//! loop or retry re-preparation). Each entry point discovers triggers and
//! reports the same skipped ones again; one pre-validation call discovers once
//! and shares the result across its cleanup and verdict passes.
//! [`TriggerFindings`] keeps them once each, by
//! the finding's full source identity, in the order first observed, and a
//! delivery cursor so a host shows each one exactly once per invocation —
//! including when the run later fails for an unrelated reason.
//!
//! A ledger belongs to one invocation
//! ([`InvocationContext::trigger_findings`](crate::invocation_context::InvocationContext::trigger_findings));
//! it is never process-global, so a new invocation reports its warnings again
//! even when Darkmatter reuses a cached discovery.

use std::sync::{Arc, Mutex, PoisonError};

use darkmatter::markdown::compose::ComposeWarning;
use darkmatter::markdown::schemas::{InvalidTriggerFinding, TriggerSource};

/// One skipped trigger, as it is shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerWarning {
    /// The trigger's normalized disk path, or its embedded bundle and name.
    pub source: TriggerSource,
    /// Darkmatter's message: names the trigger source and why it was skipped,
    /// never the document that discovered it.
    pub message: String,
}

#[derive(Debug, Default)]
struct Ledger {
    warnings: Vec<TriggerWarning>,
    delivered: usize,
}

impl Ledger {
    fn admit(&mut self, source: &TriggerSource, message: impl FnOnce() -> String) {
        // First observation wins: a later pass that sees a different cause for
        // the same trigger never replaces the one already recorded.
        if !self.warnings.iter().any(|warning| &warning.source == source) {
            self.warnings.push(TriggerWarning {
                source: source.clone(),
                message: message(),
            });
        }
    }
}

/// Invocation-scoped record of skipped schema triggers. Clones share one
/// record.
#[derive(Debug, Clone, Default)]
pub struct TriggerFindings {
    ledger: Arc<Mutex<Ledger>>,
}

impl TriggerFindings {
    fn ledger(&self) -> std::sync::MutexGuard<'_, Ledger> {
        self.ledger.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Records discovery's findings.
    pub fn record(&self, findings: &[InvalidTriggerFinding]) {
        let mut ledger = self.ledger();
        for finding in findings {
            ledger.admit(finding.identity(), || finding.message());
        }
    }

    /// Records the invalid-trigger warnings of a composition report, which
    /// include triggers only a transcluded child discovered. Other warnings
    /// are ignored.
    pub fn record_compose_warnings(&self, warnings: &[ComposeWarning]) {
        let mut ledger = self.ledger();
        for warning in warnings {
            if let Some(source) = warning.invalid_trigger_source() {
                ledger.admit(&source, || warning.message.clone());
            }
        }
    }

    /// Every trigger recorded so far, in the order first observed.
    #[must_use]
    pub fn warnings(&self) -> Vec<TriggerWarning> {
        self.ledger().warnings.clone()
    }

    /// The triggers recorded since the last call, which the caller now
    /// shows; each trigger is returned by exactly one call.
    #[must_use]
    pub fn take_undelivered(&self) -> Vec<TriggerWarning> {
        let mut ledger = self.ledger();
        let start = ledger.delivered;
        ledger.delivered = ledger.warnings.len();
        ledger.warnings[start..].to_vec()
    }
}
