//! A pinned, read-only view of the tree committed at `HEAD`. See
//! [`GitRepo::committed_tree`].

use gix::bstr::{BStr, ByteSlice as _};
use gix::objs::tree::EntryKind;

use super::types::GitRepo;
use crate::performance::{self, counters};
use crate::{Result, SniffError};

/// What a committed tree holds at one repository-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommittedEntry {
    /// The committed tree has no entry at this path. Also reported for a path
    /// below a file, a symbolic link, or a submodule, since the commit records
    /// nothing there.
    Absent,
    /// A regular file: its committed bytes exactly as stored, with no
    /// end-of-line conversion, smudge filter, or LFS resolution.
    Blob {
        /// The stored bytes.
        bytes: Vec<u8>,
        /// Whether the entry's mode is `100755`.
        executable: bool,
    },
    /// A symbolic link. Never followed: `target` is the link text as
    /// committed, which may be relative, absolute, or dangling.
    Symlink {
        /// The stored link text.
        target: Vec<u8>,
    },
    /// A directory (a tree entry). It has no bytes of its own.
    Directory,
    /// A submodule (a gitlink entry): the commit it pins in another
    /// repository, whose content this snapshot cannot read.
    Submodule {
        /// Full hex id of the pinned commit.
        commit_id: String,
    },
}

/// The tree committed at `HEAD` when [`GitRepo::committed_tree`] ran.
///
/// The commit and tree ids are pinned at capture, so later commits, branch
/// switches, staging, and working-copy edits never change what this snapshot
/// reads. It owns its own thread-safe object access and is `Send + Sync`;
/// every read opens a short-lived thread-local handle, so concurrent reads
/// from several threads are allowed.
///
/// Reads go to the object database only: no filters, external helpers,
/// hooks, or Git subprocesses run. A missing or corrupt object is an error,
/// never [`CommittedEntry::Absent`].
pub struct CommittedTreeSnapshot {
    objects: gix::ThreadSafeRepository,
    /// `None` when `HEAD` was unborn: the view is then empty.
    pinned: Option<Pinned>,
}

#[derive(Clone, Copy)]
struct Pinned {
    commit: gix::ObjectId,
    tree: gix::ObjectId,
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CommittedTreeSnapshot>();
};

impl std::fmt::Debug for CommittedTreeSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommittedTreeSnapshot")
            .field("commit_id", &self.commit_id())
            .field("tree_id", &self.tree_id())
            .finish_non_exhaustive()
    }
}

impl GitRepo {
    /// Capture the tree committed at this repository's `HEAD` (in a linked
    /// worktree, that worktree's own `HEAD`).
    ///
    /// Uses the already-open repository handle; nothing is rediscovered. A
    /// bare repository is accepted, since only `HEAD` and objects are read.
    ///
    /// ## Returns
    ///
    /// An unborn `HEAD` (a repository with no commit yet) is a valid empty
    /// view: [`CommittedTreeSnapshot::commit_id`] is `None`, `paths` is
    /// empty, and every read is [`CommittedEntry::Absent`].
    ///
    /// ## Errors
    ///
    /// [`SniffError::Git`] tagged `"committed tree"` when `HEAD` cannot be
    /// read or peeled, does not name a commit, or its commit or root tree
    /// object is missing or corrupt.
    pub fn committed_tree(&self) -> Result<CommittedTreeSnapshot> {
        self.with_gix(|repo| {
            let head = repo
                .head()
                .map_err(|error| SniffError::git("committed tree", error))?;
            let Some(commit_id) = head
                .try_into_peeled_id()
                .map_err(|error| SniffError::git("committed tree", error))?
            else {
                return Ok(CommittedTreeSnapshot {
                    objects: repo.clone().into_sync(),
                    pinned: None,
                });
            };
            let commit = repo
                .find_commit(commit_id)
                .map_err(|error| SniffError::git("committed tree", error))?;
            let tree = commit
                .tree_id()
                .map_err(|error| SniffError::git("committed tree", error))?
                .detach();
            // Fail at capture, not at first read, when the root tree is gone.
            repo.find_tree(tree)
                .map_err(|error| SniffError::git("committed tree", error))?;
            Ok(CommittedTreeSnapshot {
                objects: repo.clone().into_sync(),
                pinned: Some(Pinned {
                    commit: commit_id.detach(),
                    tree,
                }),
            })
        })
    }

    /// Capture the tree of the commit whose full hex id is `commit_id`,
    /// whatever `HEAD` names: the same read-only view as
    /// [`Self::committed_tree`], pinned to a commit the caller chose (a pull
    /// request's base and head, for example).
    ///
    /// Only a full object id is accepted, never a ref name, an abbreviation,
    /// or a revision expression, so the view cannot move under the caller
    /// and no revision parsing runs.
    ///
    /// ## Errors
    ///
    /// [`SniffError::Git`] tagged `"committed tree at"` when `commit_id` is
    /// not a full hex object id, or the object is absent, is not a commit,
    /// or its root tree is missing or corrupt.
    pub fn committed_tree_at(&self, commit_id: &str) -> Result<CommittedTreeSnapshot> {
        let id = gix::ObjectId::from_hex(commit_id.as_bytes())
            .map_err(|error| SniffError::git("committed tree at", error))?;
        self.with_gix(|repo| {
            let commit = repo
                .find_commit(id)
                .map_err(|error| SniffError::git("committed tree at", error))?;
            let tree = commit
                .tree_id()
                .map_err(|error| SniffError::git("committed tree at", error))?
                .detach();
            repo.find_tree(tree)
                .map_err(|error| SniffError::git("committed tree at", error))?;
            Ok(CommittedTreeSnapshot {
                objects: repo.clone().into_sync(),
                pinned: Some(Pinned { commit: id, tree }),
            })
        })
    }
}

impl CommittedTreeSnapshot {
    /// Full hex id of the pinned commit, or `None` when `HEAD` was unborn.
    pub fn commit_id(&self) -> Option<String> {
        self.pinned.map(|pinned| pinned.commit.to_string())
    }

    /// Full hex id of the pinned commit's root tree, or `None` when `HEAD`
    /// was unborn.
    pub fn tree_id(&self) -> Option<String> {
        self.pinned.map(|pinned| pinned.tree.to_string())
    }

    /// What the pinned tree holds at `path`.
    ///
    /// `path` is repository-relative with `/` separators on every OS. A
    /// symbolic link is reported, never followed, including one standing in
    /// for a parent directory of `path`.
    ///
    /// ## Errors
    ///
    /// - [`SniffError::InvalidRepositoryPath`]: `path` is empty, absolute
    ///   (leading `/` or a drive prefix such as `C:`), has an empty, `.`, or
    ///   `..` component, or contains `\` or NUL. `\` is rejected on every OS,
    ///   Windows included, so a path means the same thing on every host;
    ///   convert native separators before calling.
    /// - [`SniffError::Git`] tagged `"committed tree read"`: a tree or blob
    ///   object on the way is missing or corrupt.
    pub fn read_file(&self, path: &str) -> Result<CommittedEntry> {
        let components = repository_components(path)?;
        let Some(pinned) = self.pinned else {
            return Ok(CommittedEntry::Absent);
        };
        let repo = self.objects.to_thread_local();
        let mut tree_id = pinned.tree;
        let last = components.len() - 1;
        for (position, name) in components.into_iter().enumerate() {
            let tree = repo
                .find_tree(tree_id)
                .map_err(|error| SniffError::git("committed tree read", error))?;
            let decoded = tree
                .decode()
                .map_err(|error| SniffError::git("committed tree read", error))?;
            let Some(entry) = decoded
                .entries
                .iter()
                .find(|entry| entry.filename == BStr::new(name))
            else {
                return Ok(CommittedEntry::Absent);
            };
            let kind = entry.mode.kind();
            let id = entry.oid.to_owned();
            if position < last {
                if kind != EntryKind::Tree {
                    return Ok(CommittedEntry::Absent);
                }
                tree_id = id;
                continue;
            }
            return Ok(match kind {
                EntryKind::Tree => CommittedEntry::Directory,
                EntryKind::Commit => CommittedEntry::Submodule {
                    commit_id: id.to_string(),
                },
                EntryKind::Link => CommittedEntry::Symlink {
                    target: read_blob(&repo, id)?,
                },
                EntryKind::Blob | EntryKind::BlobExecutable => CommittedEntry::Blob {
                    bytes: read_blob(&repo, id)?,
                    executable: kind == EntryKind::BlobExecutable,
                },
            });
        }
        unreachable!("repository_components never returns an empty list")
    }

    /// Every regular-file and symbolic-link path in the pinned tree,
    /// repository-relative with `/` separators, in byte order (the order of
    /// `git ls-tree -r --name-only`). Directories and submodules are not
    /// listed. Empty when `HEAD` was unborn.
    ///
    /// ## Errors
    ///
    /// [`SniffError::Git`] tagged `"committed tree paths"`: a tree object is
    /// missing or corrupt, or a committed path is not valid UTF-8 (reported
    /// rather than spelled lossily, since a lossy path could not be read
    /// back).
    pub fn paths(&self) -> Result<Vec<String>> {
        let Some(pinned) = self.pinned else {
            return Ok(Vec::new());
        };
        let repo = self.objects.to_thread_local();
        let mut paths = Vec::new();
        // Depth-first in tree order, which is byte order of the full path.
        // An explicit stack keeps a deeply nested tree off the call stack.
        let mut stack = vec![(
            tree_entries(&repo, pinned.tree, "")?.into_iter(),
            String::new(),
        )];
        while let Some((entries, prefix)) = stack.last_mut() {
            let Some((kind, name, id)) = entries.next() else {
                stack.pop();
                continue;
            };
            let path = format!("{prefix}{name}");
            match kind {
                EntryKind::Tree => {
                    let prefix = format!("{path}/");
                    stack.push((tree_entries(&repo, id, &prefix)?.into_iter(), prefix));
                }
                EntryKind::Commit => {}
                EntryKind::Blob | EntryKind::BlobExecutable | EntryKind::Link => paths.push(path),
            }
        }
        Ok(paths)
    }
}

/// The entries of tree `id` as owned `(kind, name, id)` triples, in tree
/// order. `prefix` only labels a non-UTF-8 name in the error.
fn tree_entries(
    repo: &gix::Repository,
    id: gix::ObjectId,
    prefix: &str,
) -> Result<Vec<(EntryKind, String, gix::ObjectId)>> {
    let tree = repo
        .find_tree(id)
        .map_err(|error| SniffError::git("committed tree paths", error))?;
    let decoded = tree
        .decode()
        .map_err(|error| SniffError::git("committed tree paths", error))?;
    decoded
        .entries
        .iter()
        .map(|entry| {
            let name = entry.filename.to_str().map_err(|_| {
                SniffError::git(
                    "committed tree paths",
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("committed path is not UTF-8: {prefix}{}", entry.filename),
                    ),
                )
            })?;
            Ok((entry.mode.kind(), name.to_owned(), entry.oid.to_owned()))
        })
        .collect()
}

fn read_blob(repo: &gix::Repository, id: gix::ObjectId) -> Result<Vec<u8>> {
    performance::increment_counter(counters::GIT_BLOB_LOADS, 1);
    let mut blob = repo
        .find_blob(id)
        .map_err(|error| SniffError::git("committed tree read", error))?;
    Ok(blob.take_data())
}

/// The components of a repository-relative `/`-separated path, or a typed
/// error naming why it is not one.
fn repository_components(path: &str) -> Result<Vec<&str>> {
    let reject = |reason: &'static str| SniffError::InvalidRepositoryPath {
        path: path.to_owned(),
        reason,
    };
    if path.is_empty() {
        return Err(reject("path is empty"));
    }
    if path.starts_with('/') {
        return Err(reject("path is absolute"));
    }
    if path.contains('\\') {
        return Err(reject("path contains '\\'; use '/' separators"));
    }
    if path.contains('\0') {
        return Err(reject("path contains NUL"));
    }
    let components: Vec<&str> = path.split('/').collect();
    let first = components[0].as_bytes();
    if first.len() == 2 && first[0].is_ascii_alphabetic() && first[1] == b':' {
        return Err(reject("path has a drive prefix"));
    }
    for component in &components {
        match *component {
            "" => return Err(reject("path has an empty component")),
            "." | ".." => return Err(reject("path has a '.' or '..' component")),
            _ => {}
        }
    }
    Ok(components)
}
