//! Portable references: the most portable verified spelling of a target.
//!
//! [`PortablePath`] evaluates an ordered [`PortabilityPreference`] strategy
//! against a target, verifies each candidate with the resolver in one
//! prepared [`FileResolutionContext`](crate::FileResolutionContext), and
//! reports every attempt as typed data. [`PathIdentity`] is the shared
//! lexical identity behind every prefix test and relative route; the
//! crate-internal `text` seam turns generated references into text.

mod diagnostics;
mod env_anchor;
mod evaluate;
mod path_identity;
mod strategy;
mod text;

pub use diagnostics::{
    Attempt, AttemptOutcome, ConfigurationProblem, EnvAnchorProblem, FilterProblem, Finding,
    InvalidTarget, NotApplicable, PortablePathError, ProbeError, ResolutionProblem, SpellingProblem,
};
pub use env_anchor::PORTABLE_ENV_VARIABLES;
pub use evaluate::{PortablePath, PortableReference};
pub use path_identity::{PathIdentity, RelativeRoute};
pub(crate) use path_identity::{first_seen_by_identity, normalize_native};
pub use strategy::{IntentForms, PortabilityPreference};
