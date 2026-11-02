//! Portable references: the most portable verified spelling of a target.
//!
//! [`PortablePath`] evaluates an ordered [`PortabilityPreference`] strategy
//! against a target, verifies each candidate with the resolver in one
//! prepared [`FileResolutionContext`](crate::FileResolutionContext), and
//! reports every attempt as typed data. [`PathIdentity`] is the shared
//! lexical identity behind every prefix test and relative route; the
//! crate-internal `text` seam turns generated references into text.

// Evaluation verifies every spelling with the resolver, so it and the
// diagnostics, anchors, and text rendering it alone uses need
// `file-reference`; path identity and the strategy vocabulary are passive.
#[cfg(feature = "file-reference")]
mod diagnostics;
#[cfg(feature = "file-reference")]
mod env_anchor;
#[cfg(feature = "file-reference")]
mod evaluate;
mod path_identity;
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
pub use strategy::{IntentForms, PortabilityPreference};
