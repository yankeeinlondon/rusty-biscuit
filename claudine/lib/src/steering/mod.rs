//! Steering running agent sessions.
//!
//! This module holds the provider-neutral contracts every steering path
//! shares — target identities ([`identity`]), request and result shapes
//! ([`contract`]), and evidence-gated eligibility ([`eligibility`]) — over the
//! generated research facts and reviewed activation policy, plus the
//! execution-owned [`controller`] that serializes delivery and the
//! [`discovery`] aggregator that lists steerable sessions.
//!
//! A case is selectable only when research, a reviewed exact activation
//! grant, and an implemented adapter ([`adapters::IMPLEMENTED_ADAPTERS`]) all
//! agree, and the reviewed policy does not block its launch profile. Anything
//! less leaves it unavailable with a specific [`eligibility::Blocker`].
//!
//! Design authority: `2026-09-08-steering`.

pub mod adapters;
pub mod audit;
pub mod contract;
pub mod controller;
pub mod discovery;
pub mod eligibility;
mod generated;
pub mod identity;

pub use claudine_catalog_types::steering as vocabulary;

use crate::provider_id::Provider;
use vocabulary::{ActivationGrant, AdapterRef, ExecutionSelection, ProfileBlock, ProviderSteering};

/// The OS this build runs on. WSL is Linux; native Windows is distinct.
pub fn host_os() -> vocabulary::HostOs {
    if cfg!(windows) {
        vocabulary::HostOs::Windows
    } else if cfg!(target_os = "macos") {
        vocabulary::HostOs::Macos
    } else {
        vocabulary::HostOs::Linux
    }
}

/// The provider whose slug is exactly `slug`. Steering identifiers and wire
/// values use slugs; unlike CLI name parsing, nothing is fuzzy-matched.
pub fn provider_by_slug(slug: &str) -> Option<Provider> {
    crate::provider_id::PROVIDERS_DISPLAY_ORDER.iter().copied().find(|provider| provider.as_slug() == slug)
}

/// Serializes a provider as its slug.
pub(crate) fn serialize_slug<S: serde::Serializer>(provider: &Provider, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(provider.as_slug())
}

/// Wired providers in research-roster order, the order listings sort by.
pub fn roster_order() -> &'static [Provider] {
    generated::ROSTER_ORDER
}

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

/// Launch profiles the activation policy refuses, with their reasons.
pub fn profile_blocks() -> &'static [ProfileBlock] {
    generated::PROFILE_BLOCKS
}

/// The researched preferred and fallback execution interfaces for
/// `provider`, when its research selects one.
pub fn execution_selection(provider: Provider) -> Option<&'static ExecutionSelection> {
    facts(provider).execution_selection.as_ref()
}
