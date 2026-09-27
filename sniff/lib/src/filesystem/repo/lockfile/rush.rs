//! Rush layout classification (ruling R3).
//!
//! Phase 2 placeholder: it reports which of Rush's manager lockfiles exist
//! without reading `rush.json`, so no layout is compared yet.

use std::path::Path;

use tracing::debug;

use super::super::detection::ManifestStore;
use super::{LockfileObservation, LockfilePresence, LockfileReason};

/// The lockfile each Rush package manager writes, relative to the repo root.
const MANAGER_LOCKFILES: [&str; 3] = [
    "common/config/rush/npm-shrinkwrap.json",
    "common/config/rush/pnpm-lock.yaml",
    "common/config/rush/yarn.lock",
];

/// Observe a Rush layer's lockfile.
pub(super) fn observe(root: &Path, wants: bool, store: &ManifestStore) -> LockfileObservation {
    let mut paths = Vec::new();
    for candidate in MANAGER_LOCKFILES {
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
    if wants {
        LockfileObservation::unverifiable(paths, LockfileReason::UnsupportedLayout)
    } else {
        LockfileObservation::not_requested(paths)
    }
}
