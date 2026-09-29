//! Steering running agent sessions.
//!
//! This module holds the provider-neutral contracts every steering path
//! shares — target identities ([`identity`]), request and result shapes
//! ([`contract`]), and evidence-gated eligibility ([`eligibility`]) — over the
//! generated research facts and reviewed activation policy.
//!
//! A case is selectable only when research, a reviewed exact activation
//! grant, and an implemented adapter ([`adapters::IMPLEMENTED_ADAPTERS`]) all
//! agree. Anything less leaves it unavailable with a specific [`eligibility::Blocker`].
//!
//! Design authority: `2026-09-08-steering`.

pub mod adapters;
pub mod audit;
pub mod contract;
pub mod eligibility;
mod generated;
pub mod identity;

pub use claudine_catalog_types::steering as vocabulary;

use crate::provider_id::Provider;
use vocabulary::{ActivationGrant, AdapterRef, ProviderSteering};

/// Researched steering facts for `provider`.
pub fn facts(provider: Provider) -> &'static ProviderSteering {
    generated::provider_steering(provider)
}

/// Reviewed exact activation grants from the activation policy.
pub fn activation_grants() -> &'static [ActivationGrant] {
    generated::ACTIVATION_GRANTS
}

/// Reviewed adapter revisions named by the activation policy.
pub fn reviewed_adapters() -> &'static [AdapterRef] {
    generated::REVIEWED_ADAPTERS
}
