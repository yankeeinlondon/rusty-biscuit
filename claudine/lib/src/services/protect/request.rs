use std::sync::Arc;

use crate::events::{AgenticEvent, Provider};
use crate::permissions::{PolicyContext, ProviderCliOverrides};

use super::config::ProtectPhase;
use super::observe::ProtectObservation;

/// Session-level context for protect evaluations.
#[derive(Debug, Clone)]
pub struct ProtectSessionContext {
    pub provider: Provider,
    pub policy_context: PolicyContext,
    pub cli: ProtectCliContext,
    pub interactive: bool,
    pub yolo: bool,
    pub session_id: Option<String>,
}

/// CLI context for determining effective vs configured policy.
///
/// `Parsed` wraps overrides in `Arc` because `ProviderCliOverrides`
/// contains a `Box<dyn Any>` and is not `Clone`.
#[derive(Debug, Clone)]
pub enum ProtectCliContext {
    /// No CLI information available.
    None,
    /// Raw argv from the wrapper.
    Argv(Vec<String>),
    /// Pre-parsed overrides (shared via Arc since inner is not Clone).
    Parsed(Arc<ProviderCliOverrides>),
}

/// Full protect evaluation request.
#[derive(Debug)]
pub struct ProtectRequest {
    pub provider: Provider,
    pub event: AgenticEvent,
    pub phase: ProtectPhase,
    pub session: ProtectSessionContext,
    pub observation: ProtectObservation,
}
