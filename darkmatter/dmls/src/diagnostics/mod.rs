//! The diagnostics pipeline: stable codes, scheduling tiers, and the push
//! publisher.
//!
//! The diagnostic *analysis* lives in the provider registry
//! ([`crate::providers`], "diagnostics: union"); this module owns the
//! user-facing code taxonomy ([`codes`]), the tier model + publish path
//! ([`DiagnosticsScheduler`]), and the raw notification sender
//! ([`DiagnosticsPublisher`]).

pub mod codes;
pub mod frontmatter;
mod publisher;
mod scheduler;

pub use publisher::DiagnosticsPublisher;
pub use scheduler::{DiagnosticTier, DiagnosticsScheduler};
