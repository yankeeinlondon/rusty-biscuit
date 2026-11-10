//! What Git knows about individual paths, answered without a working-tree
//! status walk. See [`GitRepo::tracked_state`].

use std::borrow::Cow;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use gix::bstr::{BStr, BString};
use gix::prelude::Find as _;

use super::types::GitRepo;
use crate::performance::{self, counters};
use crate::{Result, SniffError};

/// What Git knows about one path.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrackedState {
    /// A regular file in the index at the unconflicted stage with no staged
    /// change (its index blob and mode equal `HEAD`'s) and no unstaged change
    /// (its worktree bytes, after Git's worktree-to-index conversion, hash to
    /// the index blob, and its executable bit matches where `core.fileMode`
    /// applies).
    Clean {
        /// Full hex object id of the index blob.
        index_blob: String,
        /// Full hex object id of the `HEAD` blob; always equal to `index_blob`.
        head_blob: String,
    },
    /// In the index, but a staged or unstaged change exists: added but not in
    /// `HEAD`, deleted from the worktree, a type or mode change, different
    /// content, or a symbolic link (never followed, so never proven clean).
    Dirty,
    /// Not in the index and not ignored. Also reported for a path that does
    /// not exist and for a directory, since directories are not index entries.
    Untracked,
    /// Not in the index and matched by an ignore rule (including a rule that
    /// ignores one of its parent directories).
    Ignored,
    /// Has unmerged (conflict-stage) index entries.
    Conflicted,
    /// In the index, but a `filter` attribute applies (for example
    /// `filter=lfs`): the worktree bytes are not what Git stores, so this is
    /// never clean, whatever the content.
    Filtered,
    /// Not under this repository's working tree: outside its root, inside its
    /// `.git` directory, or inside a nested repository or submodule.
    Outside,
}

impl GitRepo {
    /// The [`TrackedState`] of each of `paths`, one result per input path in
    /// the same order.
    ///
    /// Paths must be absolute and canonical (as
    /// [`biscuit_file::canonicalize_simplified`] produces them); they are
    /// compared component-wise with the canonical repository root, and a path
    /// that is not under it is [`TrackedState::Outside`].
    ///
    /// The index and `HEAD` tree are read once per call. No status walk runs:
    /// each path costs an index lookup, a `HEAD` tree lookup, an ignore or
    /// attribute check, a metadata probe per parent directory (to detect a
    /// nested repository), and, for a tracked regular file whose recorded
    /// size matches, one read of the file. Reads are counted as
    /// [`counters::FS_FILE_OPENS`] and [`counters::FS_BYTES_READ`].
    ///
    /// ## Errors
    ///
    /// - [`SniffError::BareRepository`]: the repository has no working tree.
    /// - [`SniffError::Io`]: the repository root cannot be canonicalized, or a
    ///   path's metadata or bytes cannot be read for a reason other than its
    ///   absence.
    /// - [`SniffError::Git`]: unreadable index, `HEAD` tree, attributes, or
    ///   ignore files.
    pub fn tracked_state(&self, paths: &[PathBuf]) -> Result<Vec<TrackedState>> {
        let root = self.canonical_worktree_root()?;
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        self.with_gix(|repo| {
            let index = super::open::read_index(repo)?;
            let head_tree_id = repo
                .head_tree_id_or_empty()
                .map_err(|error| SniffError::git("head tree", error))?;
            let head_tree = repo
                .find_tree(head_tree_id)
                .map_err(|error| SniffError::git("head tree", error))?;
            let mut probe = Probe::new(repo, &index)?;
            paths
                .iter()
                .map(|path| probe.classify(&root, path, &head_tree))
                .collect()
        })
    }

    /// The full hex Git blob id `bytes` would have if they were the worktree
    /// content of `path` (absolute and canonical, under the repository root).
    ///
    /// Git's worktree-to-index conversion for `path` is applied before
    /// hashing (`core.autocrlf`, `core.eol`, the `text`/`eol`/`ident`/
    /// `working-tree-encoding` attributes), so a CRLF checkout of a clean LF
    /// file hashes to its index blob. External filter drivers are never run.
    ///
    /// ## Errors
    ///
    /// - [`SniffError::BareRepository`]: the repository has no working tree.
    /// - [`SniffError::Git`] tagged `"worktree_blob_id"`: `path` is not under
    ///   the repository root, or a `filter` attribute applies to it (its
    ///   stored form depends on a driver this method will not run).
    /// - [`SniffError::Git`]: unreadable index, attributes, or conversion
    ///   failure.
    pub fn worktree_blob_id(&self, path: &Path, bytes: &[u8]) -> Result<String> {
        let root = self.canonical_worktree_root()?;
        let relative = path
            .strip_prefix(&root)
            .ok()
            .and_then(git_relative_path)
            .ok_or_else(|| invalid_input("path is not under the repository working tree"))?;
        self.with_gix(|repo| {
            let index = super::open::read_index(repo)?;
            let mut probe = Probe::new(repo, &index)?;
            if probe.has_filter(relative.as_ref(), None)? {
                return Err(invalid_input(
                    "a filter attribute applies, so the stored form depends on an external driver",
                ));
            }
            probe
                .blob_id(relative.as_ref(), bytes)
                .map(|id| id.to_string())
        })
    }

    fn canonical_worktree_root(&self) -> Result<PathBuf> {
        if self.is_bare() {
            return Err(SniffError::BareRepository {
                path: self.repo_root().to_path_buf(),
            });
        }
        performance::increment_counter(counters::FS_CANONICALIZATIONS, 1);
        Ok(biscuit_file::canonicalize_simplified(self.repo_root())?)
    }
}

fn invalid_input(message: &'static str) -> SniffError {
    SniffError::git(
        "worktree_blob_id",
        std::io::Error::new(std::io::ErrorKind::InvalidInput, message),
    )
}

/// `relative` in Git's slash-separated spelling, or `None` when it is empty
/// or not made only of normal components.
fn git_relative_path(relative: &Path) -> Option<BString> {
    if relative.as_os_str().is_empty()
        || !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(gix::path::to_unix_separators_on_windows(gix::path::into_bstr(relative)).into_owned())
}

fn metadata(path: &Path) -> Result<Option<std::fs::Metadata>> {
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The attribute-and-ignore stack and the driver-free conversion pipeline one
/// call shares across its paths.
struct Probe<'repo> {
    repo: &'repo gix::Repository,
    index: &'repo gix::index::State,
    stack: gix::worktree::Stack,
    filter: gix::filter::plumbing::Pipeline,
    file_mode: bool,
}

impl<'repo> Probe<'repo> {
    fn new(repo: &'repo gix::Repository, index: &'repo gix::index::State) -> Result<Self> {
        use gix::worktree::stack::state::{attributes, ignore};

        let stack = repo
            .attributes(
                index,
                attributes::Source::WorktreeThenIdMapping,
                ignore::Source::WorktreeThenIdMappingIfNotSkipped,
                None,
            )
            .map_err(|error| SniffError::git("attributes", error))?
            .detach();
        let (mut filter, stack) = gix::filter::Pipeline::new(repo, stack)
            .map_err(|error| SniffError::git("filter pipeline", error))?
            .into_parts();
        // Without drivers a `filter` attribute can never spawn a process;
        // such paths are reported `Filtered` before conversion anyway.
        filter.options_mut().drivers.clear();
        // The round-trip check only warns or refuses; it never changes bytes.
        filter.options_mut().crlf_roundtrip_check =
            gix::filter::plumbing::pipeline::CrlfRoundTripCheck::Skip;
        let file_mode = repo
            .config_snapshot()
            .boolean("core.fileMode")
            .unwrap_or(true);
        Ok(Self {
            repo,
            index,
            stack,
            filter,
            file_mode,
        })
    }

    fn classify(
        &mut self,
        root: &Path,
        path: &Path,
        head_tree: &gix::Tree<'_>,
    ) -> Result<TrackedState> {
        let Ok(relative_path) = path.strip_prefix(root) else {
            return Ok(TrackedState::Outside);
        };
        if relative_path.as_os_str().is_empty() {
            return Ok(TrackedState::Untracked);
        }
        let Some(relative) = git_relative_path(relative_path) else {
            return Ok(TrackedState::Outside);
        };
        if relative_path.components().next() == Some(Component::Normal(".git".as_ref()))
            || self.inside_nested_repository(root, relative_path, relative.as_ref())?
        {
            return Ok(TrackedState::Outside);
        }

        let relative: &BStr = relative.as_ref();
        if let Some(range) = self.index.entry_range(relative)
            && self.index.entries()[range]
                .iter()
                .any(|entry| entry.stage_raw() != 0)
        {
            return Ok(TrackedState::Conflicted);
        }
        let Some(entry) = self
            .index
            .entry_by_path_and_stage(relative, gix::index::entry::Stage::Unconflicted)
        else {
            let mode = metadata(path)?
                .filter(std::fs::Metadata::is_dir)
                .map(|_| gix::index::entry::Mode::DIR);
            let excluded = self
                .stack
                .at_entry(relative, mode, self.repo)
                .map_err(|error| SniffError::git("excludes", error))?
                .is_excluded();
            return Ok(if excluded {
                TrackedState::Ignored
            } else {
                TrackedState::Untracked
            });
        };

        if self.has_filter(relative, Some(entry.mode))? {
            return Ok(TrackedState::Filtered);
        }
        let index_id = entry.id;
        let index_mode = entry.mode;
        let recorded_size = entry.stat.size;
        let head_matches = head_tree
            .lookup_entry_by_path(relative_path)
            .map_err(|error| SniffError::git("head tree", error))?
            .is_some_and(|head| {
                head.object_id() == index_id && index_mode.to_tree_entry_mode() == Some(head.mode())
            });
        if !head_matches {
            return Ok(TrackedState::Dirty);
        }
        if index_mode != gix::index::entry::Mode::FILE
            && index_mode != gix::index::entry::Mode::FILE_EXECUTABLE
        {
            return Ok(TrackedState::Dirty);
        }
        let Some(metadata) = metadata(path)?.filter(std::fs::Metadata::is_file) else {
            return Ok(TrackedState::Dirty);
        };
        if self.file_mode && executable_differs(&metadata, index_mode) {
            return Ok(TrackedState::Dirty);
        }
        // Git's own rule: a nonzero recorded size that differs from the
        // worktree size is a change, decided without reading content.
        if recorded_size != 0 && recorded_size != metadata.len() as u32 {
            return Ok(TrackedState::Dirty);
        }

        performance::increment_counter(counters::FS_FILE_OPENS, 1);
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(TrackedState::Dirty);
            }
            Err(error) => return Err(error.into()),
        };
        performance::increment_counter(counters::FS_BYTES_READ, bytes.len() as u64);
        if self.blob_id(relative, &bytes)? != index_id {
            return Ok(TrackedState::Dirty);
        }
        let id = index_id.to_string();
        Ok(TrackedState::Clean {
            index_blob: id.clone(),
            head_blob: id,
        })
    }

    /// Whether a parent directory of `relative_path` is a submodule or holds
    /// its own `.git`, or `relative_path` is itself a submodule entry.
    fn inside_nested_repository(
        &self,
        root: &Path,
        relative_path: &Path,
        relative: &BStr,
    ) -> Result<bool> {
        let is_gitlink = |path: &BStr| {
            self.index
                .entry_by_path(path)
                .is_some_and(|entry| entry.mode == gix::index::entry::Mode::COMMIT)
        };
        if is_gitlink(relative) {
            return Ok(true);
        }
        for parent in relative_path
            .ancestors()
            .skip(1)
            .take_while(|parent| !parent.as_os_str().is_empty())
        {
            let Some(parent_relative) = git_relative_path(parent) else {
                return Ok(true);
            };
            if is_gitlink(parent_relative.as_ref())
                || metadata(&root.join(parent).join(".git"))?.is_some()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether a `filter` attribute is set on `relative` (with or without a
    /// driver name).
    fn has_filter(
        &mut self,
        relative: &BStr,
        mode: Option<gix::index::entry::Mode>,
    ) -> Result<bool> {
        let mut outcome = self.stack.selected_attribute_matches(["filter"]);
        self.stack
            .at_entry(relative, mode, self.repo)
            .map_err(|error| SniffError::git("attributes", error))?
            .matching_attributes(&mut outcome);
        Ok(outcome.iter_selected().next().is_some_and(|matched| {
            matches!(
                matched.assignment.state,
                gix::attrs::StateRef::Set | gix::attrs::StateRef::Value(_)
            )
        }))
    }

    /// The blob id of `bytes` after the worktree-to-index conversion for
    /// `relative`.
    fn blob_id(&mut self, relative: &BStr, bytes: &[u8]) -> Result<gix::ObjectId> {
        let repo = self.repo;
        let index = self.index;
        let platform = self
            .stack
            .at_entry(relative, None, repo)
            .map_err(|error| SniffError::git("attributes", error))?;
        let rela_path = gix::path::from_bstr(relative);
        // Auto-CRLF consults the index blob: Git leaves CRs alone in a file
        // whose committed form already contains them.
        let mut index_object = |buffer: &mut Vec<u8>| {
            let Some(entry) = index.entry_by_path(relative) else {
                return Ok(None);
            };
            performance::increment_counter(counters::GIT_BLOB_LOADS, 1);
            let object = repo.objects.try_find(&entry.id, buffer)?;
            Ok(object
                .filter(|object| object.kind == gix::object::Kind::Blob)
                .map(|_| ()))
        };
        let outcome = self
            .filter
            .convert_to_git(
                bytes,
                &rela_path,
                &mut |_, attributes| {
                    platform.matching_attributes(attributes);
                },
                &mut index_object,
            )
            .map_err(|error| SniffError::git("convert_to_git", error))?;
        let converted: Cow<'_, [u8]> = match outcome {
            gix::filter::plumbing::pipeline::convert::ToGitOutcome::Unchanged(_) => {
                Cow::Borrowed(bytes)
            }
            gix::filter::plumbing::pipeline::convert::ToGitOutcome::Buffer(buffer) => {
                Cow::Borrowed(buffer)
            }
            gix::filter::plumbing::pipeline::convert::ToGitOutcome::Process(mut read) => {
                let mut buffer = Vec::new();
                read.read_to_end(&mut buffer)?;
                Cow::Owned(buffer)
            }
        };
        gix::objs::compute_hash(repo.object_hash(), gix::object::Kind::Blob, &converted)
            .map_err(|error| SniffError::git("hash", error))
    }
}

#[cfg(unix)]
fn executable_differs(metadata: &std::fs::Metadata, mode: gix::index::entry::Mode) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    let executable = metadata.permissions().mode() & 0o100 != 0;
    executable != (mode == gix::index::entry::Mode::FILE_EXECUTABLE)
}

#[cfg(not(unix))]
fn executable_differs(_metadata: &std::fs::Metadata, _mode: gix::index::entry::Mode) -> bool {
    false
}
