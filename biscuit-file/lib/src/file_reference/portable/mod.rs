//! Portable references: the most portable verified spelling of a target.
//!
//! [`PortablePath`] evaluates an ordered [`PortabilityPreference`] strategy
//! against a target, verifies each candidate with the resolver in one
//! prepared [`FileResolutionContext`](crate::FileResolutionContext) or checks
//! launch shadows, and
//! reports every attempt as typed data. [`PathIdentity`] is the shared
//! lexical identity behind every prefix test and relative route; the
//! crate-internal `text` seam turns generated references into text.

// Evaluation uses the filesystem, so it and the
// diagnostics, anchors, and text rendering it alone uses need
// `file-reference`; path identity and the strategy vocabulary are passive.
#[cfg(feature = "file-reference")]
mod diagnostics;
#[cfg(feature = "file-reference")]
mod env_anchor;
#[cfg(feature = "file-reference")]
mod evaluate;
mod path_identity;
#[cfg(feature = "file-reference")]
mod shadow;
mod strategy;
#[cfg(feature = "file-reference")]
mod text;

#[cfg(feature = "file-reference")]
pub use diagnostics::{
    Attempt, AttemptOutcome, ConfigurationProblem, EnvAnchorProblem, FilterProblem, Finding,
    InvalidTarget, NotApplicable, PortablePathError, ProbeError, ResolutionProblem, SpellingProblem,
};
#[cfg(feature = "file-reference")]
pub use env_anchor::PORTABLE_ENV_VARIABLES;
#[cfg(feature = "file-reference")]
pub use evaluate::{PortablePath, PortableReference};
pub use path_identity::{PathIdentity, RelativeRoute};
pub(crate) use path_identity::normalize_native;
#[cfg(feature = "file-reference")]
pub(crate) use path_identity::first_seen_by_identity;
#[cfg(feature = "file-reference")]
pub use shadow::{FsShadowProbe, ShadowCache, ShadowProbe};
pub use strategy::{IntentForms, PortabilityPreference};

/// The valid portable variable names `env` declares.
#[cfg(feature = "file-reference")]
pub(crate) fn declared_portable_names(env: &std::collections::HashMap<String, String>) -> std::collections::BTreeSet<String> {
    env_anchor::portable_names(env, &[]).names
}
