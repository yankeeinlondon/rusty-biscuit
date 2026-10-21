//! Invocation cancellation shared by a primitive and its admitted descendants.
//!
//! The token, its thread-scoped binding, and [`CleanupScope`] are Darkmatter's
//! ([`darkmatter::lifecycle::cancellation`]), re-exported here. This module
//! supplies Claudine's process-wide user interrupt to the probes and Claudine's
//! diagnostic catalog to [`CancellationCauseExt::error`].

pub use darkmatter::lifecycle::cancellation::{
    CancellationCause, CancellationScope, CancellationToken, CleanupScope, current,
};

use super::context::LifecycleErrorInfo;
use crate::diagnostics::ClaudineDiagnostics;

/// [`CancellationCause`]'s `err` value under Claudine's diagnostic catalog.
pub trait CancellationCauseExt {
    /// Diagnostic exposed to cancelled cleanup as `err`; see
    /// [`CancellationCause::error_with`].
    fn error(&self) -> LifecycleErrorInfo;
}

impl CancellationCauseExt for CancellationCause {
    fn error(&self) -> LifecycleErrorInfo {
        self.error_with(&ClaudineDiagnostics)
    }
}

/// Cancellation requested for this worker; user interrupts remain process-wide.
pub fn cause() -> Option<CancellationCause> {
    darkmatter::lifecycle::cancellation::cause_with(crate::interrupt::interrupted)
}

/// Whether running child work must settle before cleanup.
pub fn requested() -> bool {
    darkmatter::lifecycle::cancellation::requested_with(crate::interrupt::interrupted)
}
