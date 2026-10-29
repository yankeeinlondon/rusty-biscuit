//! The services a host supplies to run lifecycle stacks.
//!
//! A host registers what it implements on [`LifecycleServices`]; code that
//! needs a service asks for it and gets a [`MissingServiceError`] when the host
//! did not supply it, rather than a panic or a silently skipped action.

use std::fmt;

use thiserror::Error;

use super::effects::LifecycleEffects;
use super::envelope::DiagnosticCatalog;

/// A service a host supplies to the lifecycle runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleService {
    /// Communication delivery ([`LifecycleEffects`]).
    Effects,
    /// Error classification ([`DiagnosticCatalog`]).
    Diagnostics,
}

impl fmt::Display for LifecycleService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Effects => "effects",
            Self::Diagnostics => "diagnostics",
        })
    }
}

/// A required lifecycle service was not supplied by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the lifecycle `{service}` service is not configured")]
pub struct MissingServiceError {
    /// The service that was asked for.
    pub service: LifecycleService,
}

/// The services a host supplies; each is optional until something requires it.
///
/// ## Examples
///
/// ```
/// use darkmatter::lifecycle::services::{LifecycleService, LifecycleServices};
///
/// let services = LifecycleServices::default();
/// let error = services.effects().err().unwrap();
/// assert_eq!(error.service, LifecycleService::Effects);
/// ```
#[derive(Clone, Copy, Default)]
pub struct LifecycleServices<'a> {
    /// Communication delivery.
    pub effects: Option<&'a dyn LifecycleEffects>,
    /// Error classification.
    pub diagnostics: Option<&'a dyn DiagnosticCatalog>,
}

impl<'a> LifecycleServices<'a> {
    /// The communication service.
    ///
    /// ## Errors
    ///
    /// [`MissingServiceError`] naming [`LifecycleService::Effects`] when the
    /// host supplied none.
    pub fn effects(&self) -> Result<&'a dyn LifecycleEffects, MissingServiceError> {
        self.effects.ok_or(MissingServiceError { service: LifecycleService::Effects })
    }

    /// The diagnostic catalog.
    ///
    /// ## Errors
    ///
    /// [`MissingServiceError`] naming [`LifecycleService::Diagnostics`] when
    /// the host supplied none.
    pub fn diagnostics(&self) -> Result<&'a dyn DiagnosticCatalog, MissingServiceError> {
        self.diagnostics.ok_or(MissingServiceError { service: LifecycleService::Diagnostics })
    }
}

impl fmt::Debug for LifecycleServices<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LifecycleServices")
            .field("effects", &self.effects.is_some())
            .field("diagnostics", &self.diagnostics.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests;
