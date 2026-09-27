//! Metadata-only sources: dependency locks that exist beside a workspace but
//! do not enumerate its members. They are probed, never opened.

use std::path::Path;

use tracing::debug;

use super::super::detection::ManifestStore;
use super::{LockfileObservation, LockfilePresence, LockfileReason};

/// A fallback lockfile source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FallbackSource {
    /// Go `go.work.sum`: checksums only.
    GoWorkSum,
    /// Gradle's root dependency lock.
    Gradle,
    /// Bazel's `MODULE.bazel.lock`: external module resolution.
    BazelModule,
}

impl FallbackSource {
    /// Root-relative candidates, every one of which is reported when present.
    fn candidates(self) -> &'static [&'static str] {
        match self {
            Self::GoWorkSum => &["go.work.sum"],
            Self::Gradle => &["gradle.lockfile"],
            Self::BazelModule => &["MODULE.bazel.lock"],
        }
    }
}

/// Observe a fallback source from metadata alone.
///
/// A present file is `not_requested` when the request declines
/// corroboration, otherwise `unverifiable` with `no_membership_data`; its
/// contents are never inspected either way.
pub(super) fn observe(
    source: FallbackSource,
    root: &Path,
    wants: bool,
    store: &ManifestStore,
) -> LockfileObservation {
    let mut paths = Vec::new();
    for &candidate in source.candidates() {
        let path = root.join(candidate);
        match store.lockfile_presence(&path) {
            LockfilePresence::Absent => {}
            LockfilePresence::Present => paths.push(candidate.to_owned()),
            LockfilePresence::Failed(kind) => {
                debug!(path = %path.display(), ?kind, "lockfile metadata probe failed");
                return LockfileObservation::unreadable(
                    Vec::new(),
                    LockfileReason::MetadataFailed,
                );
            }
        }
    }
    if paths.is_empty() {
        return LockfileObservation::absent();
    }
    paths.sort();
    if wants {
        LockfileObservation::unverifiable(paths, LockfileReason::NoMembershipData)
    } else {
        LockfileObservation::not_requested(paths)
    }
}
