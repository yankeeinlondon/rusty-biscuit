//! Pure runtime-dispatch decisions for lifecycle [`StackControl`] outcomes.
//!
//! The decisions — control admission, retry backoff, the proxy-chain budget and
//! identity, and delay parsing — are Darkmatter's
//! ([`darkmatter::lifecycle::control`]), re-exported here. This module adds
//! [`resolve_proxy_target`], which resolves a hand-off target through
//! Claudine's harness path grammar.

pub use darkmatter::lifecycle::control::*;

#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
use super::actions::RetryBackoff;
#[cfg(test)]
use super::executor::StackControl;

/// Resolve a `Proxy` target reference to an existing prompt file.
///
/// Delegates to [`crate::harness::resolve_harness_path`] with the request's
/// context, the thin adapter over
/// the shared [`biscuit_file::FileReference`] grammar: implicit references are
/// source-relative then repository-relative, `@` is a magic-root search, `&`
/// pins to the repository root, `^` searches package, package-area, then
/// repository roots, `~` is home-pinned, and explicit `./`/`../` stay pinned
/// to the source. Resolution
/// probes the filesystem, so a hand-off to a missing document fails loudly with
/// a typed [`crate::harness::HarnessError`] rather than producing an
/// empty/garbage run. The proxied target document is the source for its own
/// nested references (D6): callers must pass the *target's* path as
/// `source_path` on the next hop.
///
/// ## Errors
///
/// Returns a [`crate::harness::HarnessError`] when the reference is empty, has
/// no anchoring directory, resolves to no existing file, or the shared resolver
/// rejects it (invalid syntax, absent context anchor, I/O failure, remote URL).
pub fn resolve_proxy_target(
    target: &str,
    source_path: &std::path::Path,
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<std::path::PathBuf, crate::harness::HarnessError> {
    crate::harness::resolve_harness_path(target, source_path, request_context)
}

#[cfg(test)]
mod tests;
