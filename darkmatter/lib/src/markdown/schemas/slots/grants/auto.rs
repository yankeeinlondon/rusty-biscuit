//! The automatic grant: which reads a slot may make with no grant at all.
//!
//! A slot may read a file of its own repository that is exactly what Git
//! has committed. The rule, for a slot written in a schema file (its
//! *defining* file):
//!
//! 1. the defining file lies in the request's repository (the root the
//!    request captured, never a new discovery); a schema anywhere else gets
//!    no automatic grant for any read;
//! 2. the target, canonicalized (a nonexistent target by its parent), lies
//!    under that same root, so a symlink pointing out of the repository is
//!    judged by where it points;
//! 3. for a content read of an existing file: Git reports it clean (in the
//!    index and in `HEAD` with the same blob, no filter attribute), the Git
//!    blob id of its bytes equals that blob, and no open editor buffer for
//!    it differs from disk. The BLAKE3 of those bytes is kept so the read
//!    the slot then makes can be checked to have seen the same bytes.
//!
//! Existence probes, and content reads of a target that does not exist, need
//! only (1) and (2): they are answered as a probe is, tracked or not. A
//! listing, a glob, or a URL is never automatic.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sniff::filesystem::git::{GitRepo, TrackedState};

use crate::markdown::compose::expression::functions::ReadKind;
use crate::markdown::compose::expression::read_gate::{ReadLocation, ReadTarget};

/// Why a read is not granted automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoRefusal {
    /// The request has no repository.
    NoRepository,
    /// The slot's defining file is not in the request's repository.
    SchemaOutsideRepository,
    /// The target is not in the request's repository.
    TargetOutsideRepository,
    /// A URL; only a grant allows one.
    Remote,
    /// A folder listing or a glob; only a folder grant allows one.
    Listing,
    /// Tracked, with staged or unstaged changes.
    Dirty,
    /// Not in the index.
    Untracked,
    /// Not in the index and ignored.
    Ignored,
    /// In a merge conflict.
    Conflicted,
    /// A `filter` attribute (such as Git LFS) applies, so the bytes on disk
    /// are not what Git stores.
    Filtered,
    /// The bytes read do not hash to the committed blob (changed since Git
    /// last looked, or a line-ending conversion Git would not undo).
    ChangedSinceStatus,
    /// An open editor buffer for the file differs from disk.
    UnsavedBuffer,
    /// Git could not answer, or the file could not be read.
    Unreadable,
}

/// The automatic-grant verdict for one read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AutoVerdict {
    /// Granted. `expected` is the BLAKE3 of the committed bytes for a content
    /// read of an existing file, which the slot's own read must reproduce;
    /// `None` for a probe or an absent target.
    Granted { expected: Option<[u8; 32]> },
    /// Not granted automatically.
    Refused(AutoRefusal),
}

/// The request's repository, as the automatic grant judges against it.
///
/// Opens the repository at the captured root once, and asks Git about each
/// path once for the life of the value (one run).
#[derive(Debug)]
pub(crate) struct RepositoryGrounds {
    root: Option<PathBuf>,
    repo: Mutex<Option<Option<GitRepo>>>,
    states: Mutex<HashMap<PathBuf, TrackedState>>,
}

impl RepositoryGrounds {
    /// Grounds for the repository rooted at `root` (the request's captured
    /// repository observation); `None` for a request with no repository.
    pub(crate) fn new(root: Option<&Path>) -> Self {
        Self {
            root: root.map(canonical_target),
            repo: Mutex::new(None),
            states: Mutex::new(HashMap::new()),
        }
    }

    /// The canonical repository root, if any.
    pub(crate) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Whether `target`, read by a slot defined at `defining`, is granted
    /// automatically. `target`'s local path must already be canonical
    /// ([`canonical_target`]).
    pub(crate) fn judge(&self, defining: &Path, target: &ReadTarget) -> AutoVerdict {
        let path = match &target.location {
            ReadLocation::Remote(_) => return AutoVerdict::Refused(AutoRefusal::Remote),
            ReadLocation::Glob(_) => return AutoVerdict::Refused(AutoRefusal::Listing),
            ReadLocation::Local(path) => path,
        };
        if target.kind == ReadKind::Listing {
            return AutoVerdict::Refused(AutoRefusal::Listing);
        }
        let Some(root) = self.root.as_deref() else {
            return AutoVerdict::Refused(AutoRefusal::NoRepository);
        };
        if !canonical_target(defining).starts_with(root) {
            return AutoVerdict::Refused(AutoRefusal::SchemaOutsideRepository);
        }
        if !path.starts_with(root) {
            return AutoVerdict::Refused(AutoRefusal::TargetOutsideRepository);
        }
        // A probe, or a read of nothing, reveals only whether the path
        // exists.
        if target.kind == ReadKind::Probe || !path.exists() {
            return AutoVerdict::Granted { expected: None };
        }
        match self.clean_content(path) {
            Ok(expected) => AutoVerdict::Granted { expected: Some(expected) },
            Err(refusal) => AutoVerdict::Refused(refusal),
        }
    }

    /// The BLAKE3 of `path`'s bytes when they are exactly its committed
    /// content.
    fn clean_content(&self, path: &Path) -> Result<[u8; 32], AutoRefusal> {
        let (index_blob, head_blob) = match self.tracked_state(path)? {
            TrackedState::Clean { index_blob, head_blob } => (index_blob, head_blob),
            TrackedState::Dirty => return Err(AutoRefusal::Dirty),
            TrackedState::Untracked => return Err(AutoRefusal::Untracked),
            TrackedState::Ignored => return Err(AutoRefusal::Ignored),
            TrackedState::Conflicted => return Err(AutoRefusal::Conflicted),
            TrackedState::Filtered => return Err(AutoRefusal::Filtered),
            TrackedState::Outside => return Err(AutoRefusal::TargetOutsideRepository),
            _ => return Err(AutoRefusal::Unreadable),
        };
        let bytes = std::fs::read(path).map_err(|_| AutoRefusal::Unreadable)?;
        if simplified_schema::resolve::buffers::text(path).is_some_and(|open| open.as_bytes() != bytes.as_slice()) {
            return Err(AutoRefusal::UnsavedBuffer);
        }
        let blob = self.with_repo(|repo| repo.worktree_blob_id(path, &bytes).map_err(|_| AutoRefusal::Unreadable))??;
        if blob != index_blob || blob != head_blob {
            return Err(AutoRefusal::ChangedSinceStatus);
        }
        Ok(biscuit_hash::blake3_hash_bytes(&bytes))
    }

    /// Git's state for `path`, asked once per path.
    fn tracked_state(&self, path: &Path) -> Result<TrackedState, AutoRefusal> {
        let mut states = self.states.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(state) = states.get(path) {
            return Ok(state.clone());
        }
        let state = self
            .with_repo(|repo| repo.tracked_state(&[path.to_path_buf()]).map_err(|_| AutoRefusal::Unreadable))??
            .pop()
            .ok_or(AutoRefusal::Unreadable)?;
        states.insert(path.to_path_buf(), state.clone());
        Ok(state)
    }

    /// Runs `query` against the repository opened at the root (once).
    fn with_repo<T>(&self, query: impl FnOnce(&GitRepo) -> T) -> Result<T, AutoRefusal> {
        let root = self.root.as_deref().ok_or(AutoRefusal::NoRepository)?;
        let mut repo = self.repo.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let opened = repo.get_or_insert_with(|| {
            // The repository the request observed; a discovery that lands
            // anywhere else is not it.
            GitRepo::discover(root)
                .ok()
                .flatten()
                .filter(|found| canonical_target(found.repo_root()) == root)
        });
        opened.as_ref().map(query).ok_or(AutoRefusal::NoRepository)
    }
}

/// `path` canonicalized for comparison: an existing path through
/// [`biscuit_file::canonicalize_simplified`] (symlinks resolved), a
/// nonexistent one by its canonical parent and its file name, and the path
/// itself when neither exists.
pub(crate) fn canonical_target(path: &Path) -> PathBuf {
    if let Ok(canonical) = biscuit_file::canonicalize_simplified(path) {
        return canonical;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => canonical_target(parent).join(name),
        _ => path.to_path_buf(),
    }
}
