//! Invocation-owned, detached runtime worktrees: the committed tree of a
//! reviewed revision, materialized so that revision's own planner runs on
//! exactly what was committed.
//!
//! Created and removed through git2 because gix 0.90 can list and open linked
//! worktrees but cannot create one. libgit2 adds a worktree only on a branch
//! (with no branch it creates one named after the worktree, at HEAD), so
//! [`RuntimeWorktree::create`] points a uniquely named temporary branch at the
//! commit, adds the worktree on it, detaches the worktree's HEAD at the same
//! commit, and deletes the branch.
//!
//! Every artifact is named after this invocation (`ci-push-review-<pid>-<random>`),
//! and removal prunes exactly that one worktree record: never a global
//! `git worktree prune`, which would also reap a concurrent invocation's
//! worktree. Removal opens its own repository handle and drops it, and holds
//! no handle into the worktree, so Windows can delete the directory.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use gix::ObjectId;

/// The prefix of every runtime worktree name, temporary branch, and
/// temporary directory; the three share one name.
pub const NAME_PREFIX: &str = "ci-push-review-";

/// A runtime worktree could not be created. Whatever creation left behind
/// has already been cleaned up, or is listed in `cleanup`.
#[derive(Debug)]
pub struct CreateError {
    pub commit: ObjectId,
    pub message: String,
    pub cleanup: Option<CleanupFailure>,
}

impl fmt::Display for CreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "could not materialize the committed tree of {} in a temporary worktree: {}",
            self.commit, self.message
        )?;
        if let Some(cleanup) = &self.cleanup {
            write!(formatter, "\n{cleanup}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CreateError {}

/// What kind of invocation-owned artifact survived removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    /// The worktree directory with its administrative record still present.
    Worktree,
    /// A directory Git no longer tracks (the temporary parent, or a worktree
    /// whose record is already gone).
    Directory,
    /// `<common dir>/worktrees/<name>` without its worktree.
    AdministrativeRecord,
    /// The temporary branch.
    Ref,
}

/// One artifact removal left behind, and how to remove it by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedArtifact {
    pub kind: ArtifactKind,
    /// A path, or a full ref name for [`ArtifactKind::Ref`].
    pub location: String,
    /// The exact command that removes it.
    pub removal: String,
}

/// Removal left artifacts behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupFailure {
    pub retained: Vec<RetainedArtifact>,
    /// The errors that stopped removal, in the order they happened.
    pub errors: Vec<String>,
}

impl fmt::Display for CleanupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("The temporary review worktree could not be removed completely")?;
        if !self.errors.is_empty() {
            write!(formatter, " ({})", self.errors.join("; "))?;
        }
        formatter.write_str(". Remove what remains by hand:")?;
        for artifact in &self.retained {
            write!(
                formatter,
                "\n  {}\n    {}",
                artifact.location, artifact.removal
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for CleanupFailure {}

/// A detached linked worktree owned by this invocation. Dropping it removes
/// it; [`RuntimeWorktree::remove`] does the same and reports what remains.
#[derive(Debug)]
pub struct RuntimeWorktree {
    /// The worktree's directory.
    path: PathBuf,
    /// The temporary directory holding `path`, removed with it.
    parent: PathBuf,
    name: String,
    commit: ObjectId,
    /// The checkout that created it; manual commands run against it.
    repository: PathBuf,
    /// `<common dir>/worktrees/<name>`.
    record: PathBuf,
    removed: bool,
}

impl RuntimeWorktree {
    /// Adds a worktree of the repository at `repository` (any checkout of it),
    /// with HEAD detached at `commit` and its tree checked out.
    pub fn create(repository: &Path, commit: ObjectId) -> Result<Self, CreateError> {
        let failed = |message: String| CreateError {
            commit,
            message,
            cleanup: None,
        };
        let parent = tempfile::Builder::new()
            .prefix(&format!("{NAME_PREFIX}{}-", std::process::id()))
            .tempdir()
            .map_err(|error| failed(format!("cannot create a temporary directory: {error}")))?
            .keep();
        // tempfile's random suffix is alphanumeric, so the name is a valid
        // branch and worktree name.
        let name = parent
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let common_dir = git2::Repository::open(repository)
            .map(|opened| opened.commondir().to_path_buf())
            .map_err(|error| {
                failed(format!(
                    "cannot open {}: {}",
                    repository.display(),
                    error.message()
                ))
            });
        let common_dir = match common_dir {
            Ok(dir) => dir,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&parent);
                return Err(error);
            }
        };
        let mut worktree = RuntimeWorktree {
            path: parent.join("tree"),
            record: common_dir.join("worktrees").join(&name),
            parent,
            name,
            commit,
            repository: repository.to_path_buf(),
            removed: false,
        };
        match worktree.add() {
            Ok(()) => Ok(worktree),
            Err(error) => {
                let cleanup = worktree.remove().err();
                Err(CreateError {
                    commit,
                    message: error.message().to_owned(),
                    cleanup,
                })
            }
        }
    }

    fn add(&mut self) -> Result<(), git2::Error> {
        let repository = git2::Repository::open(&self.repository)?;
        let oid = git2::Oid::from_bytes(self.commit.as_bytes())?;
        let commit = repository.find_commit(oid)?;
        let mut branch = repository.branch(&self.name, &commit, false)?;
        let added = (|| {
            let mut options = git2::WorktreeAddOptions::new();
            options.reference(Some(branch.get()));
            let worktree = repository.worktree(&self.name, &self.path, Some(&options))?;
            git2::Repository::open_from_worktree(&worktree)?.set_head_detached(oid)
        })();
        // The branch goes whether or not the add succeeded; `remove` retries
        // and reports it if this fails.
        let deleted = branch.delete();
        added.and(deleted)
    }

    /// The worktree's directory: the root of the reviewed commit's tree.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The commit checked out (detached) in the worktree.
    pub fn commit(&self) -> ObjectId {
        self.commit
    }

    /// The worktree's name under `<common dir>/worktrees/`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The full name of the temporary branch, which exists only during
    /// [`RuntimeWorktree::create`].
    pub fn temporary_ref(&self) -> String {
        format!("refs/heads/{}", self.name)
    }

    /// Removes the worktree's directory, its administrative record, and the
    /// temporary branch, touching nothing else. Retrying after a failure
    /// resumes with what remains.
    pub fn remove(&mut self) -> Result<(), CleanupFailure> {
        let mut errors = Vec::new();
        match git2::Repository::open(&self.repository) {
            Ok(repository) => {
                // The record first: libgit2 refuses to delete a branch that a
                // worktree still has checked out.
                for step in [self.prune(&repository), self.delete_branch(&repository)] {
                    if let Err(error) = step {
                        errors.push(error.message().to_owned());
                    }
                }
            }
            Err(error) => errors.push(error.message().to_owned()),
        }
        for (directory, what) in [(&self.record, "record"), (&self.parent, "directory")] {
            if let Err(error) = remove_directory(directory) {
                errors.push(format!(
                    "cannot remove the worktree {what} {}: {error}",
                    directory.display()
                ));
            }
        }
        let retained = self.retained();
        if retained.is_empty() {
            self.removed = true;
            Ok(())
        } else {
            Err(CleanupFailure { retained, errors })
        }
    }

    /// Prunes this one worktree's record and directory.
    fn prune(&self, repository: &git2::Repository) -> Result<(), git2::Error> {
        if !self.record.exists() {
            return Ok(());
        }
        let worktree = repository.find_worktree(&self.name)?;
        let mut options = git2::WorktreePruneOptions::new();
        options.valid(true).working_tree(true);
        worktree.prune(Some(&mut options))
    }

    /// Deletes the temporary branch if it survived creation.
    fn delete_branch(&self, repository: &git2::Repository) -> Result<(), git2::Error> {
        match repository.find_branch(&self.name, git2::BranchType::Local) {
            Ok(mut branch) => branch.delete(),
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// What exists now, in the order a person should remove it.
    fn retained(&self) -> Vec<RetainedArtifact> {
        let mut retained = Vec::new();
        let tree_exists = self.path.exists();
        let record_exists = self.record.exists();
        let git = |args: &str| {
            format!(
                "git -C {} {args}",
                quote(&self.repository.to_string_lossy())
            )
        };
        if tree_exists && record_exists {
            retained.push(RetainedArtifact {
                kind: ArtifactKind::Worktree,
                location: self.path.display().to_string(),
                removal: git(&format!(
                    "worktree remove --force {}",
                    quote(&self.path.to_string_lossy())
                )),
            });
        } else if record_exists {
            retained.push(artifact(ArtifactKind::AdministrativeRecord, &self.record));
        }
        if self.parent.exists() {
            retained.push(artifact(ArtifactKind::Directory, &self.parent));
        }
        let branch_exists =
            git2::Repository::open(&self.repository)
                .ok()
                .is_some_and(|repository| {
                    repository
                        .find_branch(&self.name, git2::BranchType::Local)
                        .is_ok()
                });
        if branch_exists {
            let name = self.temporary_ref();
            retained.push(RetainedArtifact {
                kind: ArtifactKind::Ref,
                removal: git(&format!("update-ref -d {}", quote(&name))),
                location: name,
            });
        }
        retained
    }
}

impl Drop for RuntimeWorktree {
    fn drop(&mut self) {
        if self.removed {
            return;
        }
        if let Err(failure) = self.remove() {
            // Drop cannot return the report, and a retained worktree must
            // not go unmentioned.
            eprintln!("{failure}");
        }
    }
}

fn remove_directory(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

fn artifact(kind: ArtifactKind, path: &Path) -> RetainedArtifact {
    RetainedArtifact {
        kind,
        location: path.display().to_string(),
        removal: remove_command(&path.to_string_lossy()),
    }
}

#[cfg(not(windows))]
fn remove_command(path: &str) -> String {
    format!("rm -rf {}", quote(path))
}

#[cfg(windows)]
fn remove_command(path: &str) -> String {
    format!("Remove-Item -LiteralPath {} -Recurse -Force", quote(path))
}

/// Single-quoted for the platform's shell: POSIX `sh`, or PowerShell, where
/// a quote inside single quotes is doubled.
#[cfg(not(windows))]
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

#[cfg(windows)]
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}
