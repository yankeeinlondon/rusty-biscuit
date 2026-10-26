//! What a worktree's initialized submodules hold that exists nowhere else,
//! so `wt remove` can warn about it before deleting them with the checkout.
//!
//! ## Listing
//!
//! Submodules are listed from the index's gitlink entries (mode `160000`),
//! read in-process through [`crate::git::staged_entries_of`] for the
//! worktree's own index (`<admin>/index`) and [`crate::git::staged_entries_from`]
//! for each submodule's, then recursed into. `git submodule status
//! --recursive` would start a shell-script chain per level and fails as a
//! whole on one broken submodule; the index read starts no process and lets
//! each submodule fail on its own. A gitlink whose directory holds no `.git`
//! is uninitialized: it has no checkout to lose and gets no entry.
//!
//! ## Per submodule
//!
//! Two Git processes, both addressed with `-C` from `base`, neither touching
//! the network:
//!
//! - `status --porcelain=v1 -z --untracked-files=normal
//!   --ignore-submodules=dirty`: uncommitted and untracked files (ignored ones
//!   excluded). A nested submodule's own dirt is left to its own entry; its
//!   new commits still show as a change here.
//! - `rev-list --count HEAD --branches --not --remotes`: commits reachable
//!   from `HEAD` or a local branch but from no `refs/remotes/*` ref, as last
//!   fetched. With no remote configured nothing is excluded, so every local
//!   commit counts.
//!
//! Whether a remote is configured is read in-process with `gix`. A failure at
//! any step is [`Finding::Unchecked`], never [`Finding::Clean`].
//!
//! ## Git data ownership
//!
//! Report-only: the deletion engine never follows links, so a `.git` file
//! pointing elsewhere is deleted as a file and its target kept. Paths are
//! compared after canonicalizing their longest existing ancestor, so symlink
//! aliases (`/var` and `/private/var`) and Windows `\\?\` spellings agree.

use std::path::{Path, PathBuf};

use super::admin_entry::{AdminEntry, path_from_bytes};
use crate::copy_record::canonical_worktree_path;
use crate::error::WorktreeError;

/// What [`inspect`] found in a worktree's initialized submodules.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SubmoduleReport {
    /// One entry per initialized submodule, nested ones included, in path
    /// order. An uninitialized submodule has no entry.
    pub submodules: Vec<Submodule>,
    /// The submodules could not be listed at all, with the reason; their
    /// work could not be checked.
    pub unlisted: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submodule {
    /// Relative to the worktree's checkout.
    pub path: PathBuf,
    pub git_dir: GitDirOwner,
    pub finding: Finding,
}

/// Where a submodule's Git data lives, and so whether removal deletes it.
/// The path is canonicalized as far as it exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDirOwner {
    /// Under this worktree's admin record (`<admin>/modules/<name>`), or a
    /// `.git` directory inside the submodule's checkout: deleted with the
    /// worktree. A `.git` file that names no Git directory is `Owned` by
    /// its own path, since only the file itself is deleted.
    Owned(PathBuf),
    /// Anywhere else: retained, never deleted recursively.
    External(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// Nothing exists only in this submodule.
    Clean,
    /// Work that exists only here.
    AtRisk {
        /// Uncommitted files, relative to the submodule's checkout.
        dirty: Vec<PathBuf>,
        /// Commits reachable from `HEAD` or a local branch but from no
        /// `refs/remotes/*` ref (N6).
        unique_commits: u64,
        /// The submodule has no remote configured, so every local commit
        /// counts as unique.
        no_remote: bool,
    },
    /// Its work could not be checked, with the reason.
    Unchecked(String),
}

/// Inspects every initialized submodule of the worktree checked out at
/// `checkout`, whose admin record is `admin`, in the repository at `base`.
pub fn inspect(base: &Path, admin: &AdminEntry, checkout: &Path) -> SubmoduleReport {
    let listing = crate::git::staged_entries_of(base, &admin.index()).map_err(reason).and_then(|bytes| gitlinks(&bytes));
    let gitlinks = match listing {
        Ok(gitlinks) => gitlinks,
        Err(unlisted) => return SubmoduleReport { submodules: Vec::new(), unlisted: Some(unlisted) },
    };
    let owned_root = canonical_worktree_path(&admin.dir.join("modules")).ok();
    let mut submodules = Vec::new();
    for relative in gitlinks {
        inspect_one(base, owned_root.as_deref(), checkout, relative, &mut submodules);
    }
    submodules.sort_by(|a, b| a.path.cmp(&b.path));
    SubmoduleReport { submodules, unlisted: None }
}

/// Inspects the submodule at `checkout/relative` (when initialized), then its
/// own submodules, pushing each entry onto `found`.
fn inspect_one(base: &Path, owned_root: Option<&Path>, checkout: &Path, relative: PathBuf, found: &mut Vec<Submodule>) {
    let dir = checkout.join(&relative);
    let dot_git = dir.join(".git");
    let Ok(metadata) = std::fs::symlink_metadata(&dot_git) else {
        return;
    };
    let git_dir = owner(owned_root, &dir, &dot_git, &metadata);
    let mut finding = examine(base, &dir);
    let mut nested = Vec::new();
    if !matches!(finding, Finding::Unchecked(_)) {
        match crate::git::staged_entries_from(base, &dir).map_err(reason).and_then(|bytes| gitlinks(&bytes)) {
            Ok(links) => nested = links,
            Err(error) => finding = Finding::Unchecked(format!("its submodules could not be listed: {error}")),
        }
    }
    found.push(Submodule { path: relative.clone(), git_dir, finding });
    for link in nested {
        inspect_one(base, owned_root, checkout, relative.join(link), found);
    }
}

/// Classifies the Git directory the submodule's `.git` entry names.
fn owner(owned_root: Option<&Path>, dir: &Path, dot_git: &Path, metadata: &std::fs::Metadata) -> GitDirOwner {
    let canonical = |path: &Path| canonical_worktree_path(path).unwrap_or_else(|_| path.to_path_buf());
    if metadata.is_dir() {
        return GitDirOwner::Owned(canonical(dot_git));
    }
    let named = if metadata.file_type().is_symlink() { std::fs::read_link(dot_git).ok() } else { gitdir_file(dot_git) };
    let Some(named) = named else {
        return GitDirOwner::Owned(canonical(dot_git));
    };
    let named = canonical(&if named.is_relative() { dir.join(named) } else { named });
    let inside = |root: &Path| named.starts_with(root) && named != root;
    if owned_root.is_some_and(inside) || inside(&canonical(dir)) {
        GitDirOwner::Owned(named)
    } else {
        GitDirOwner::External(named)
    }
}

/// The path a `gitdir: <path>` file names, as Git reads it (one line,
/// trailing whitespace trimmed); `None` when it names none.
fn gitdir_file(dot_git: &Path) -> Option<PathBuf> {
    let bytes = std::fs::read(dot_git).ok()?;
    let rest = bytes.strip_prefix(b"gitdir: ")?;
    let line = rest.split(|byte| *byte == b'\n').next()?.trim_ascii_end();
    (!line.is_empty()).then(|| path_from_bytes(line).ok()).flatten()
}

/// The submodule's uncommitted files and unique commits.
fn examine(base: &Path, dir: &Path) -> Finding {
    let status = ["--no-optional-locks", "status", "--porcelain=v1", "-z", "--untracked-files=normal", "--ignore-submodules=dirty"];
    let dirty = match crate::git::git_from_bytes(base, dir, &status, None).map_err(reason).and_then(|bytes| dirty_paths(&bytes)) {
        Ok(dirty) => dirty,
        Err(error) => return Finding::Unchecked(error),
    };
    let counted = crate::git::git_from(base, dir, &["rev-list", "--count", "HEAD", "--branches", "--not", "--remotes"]);
    let unique_commits = match counted.map_err(reason).and_then(|count| count.parse::<u64>().map_err(|error| error.to_string())) {
        Ok(count) => count,
        Err(error) => return Finding::Unchecked(error),
    };
    if dirty.is_empty() && unique_commits == 0 {
        return Finding::Clean;
    }
    match has_remote(base, dir) {
        Ok(has_remote) => Finding::AtRisk { dirty, unique_commits, no_remote: !has_remote },
        Err(error) => Finding::Unchecked(error),
    }
}

/// Whether the repository at `dir` has any remote configured; read
/// in-process, by `git remote` when `gix` cannot open it.
fn has_remote(base: &Path, dir: &Path) -> Result<bool, String> {
    if let Some(repo) = crate::git_metadata::open(dir) {
        return Ok(!repo.remote_names().is_empty());
    }
    crate::git::git_from(base, dir, &["remote"]).map(|names| !names.is_empty()).map_err(reason)
}

/// The gitlink paths in `ls-files --stage -z` output, once each.
fn gitlinks(listing: &[u8]) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in listing.split(|byte| *byte == 0).filter(|entry| !entry.is_empty()) {
        let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else {
            return Err(format!("unexpected index entry {:?}", String::from_utf8_lossy(entry)));
        };
        if entry.starts_with(b"160000 ") {
            let path = git_path(&entry[tab + 1..])?;
            if paths.last() != Some(&path) {
                paths.push(path);
            }
        }
    }
    Ok(paths)
}

/// The paths in `status --porcelain=v1 -z` output; a rename or copy's
/// original path (the field after it) is skipped.
fn dirty_paths(status: &[u8]) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let mut fields = status.split(|byte| *byte == 0).filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        if field.len() < 4 || field[2] != b' ' {
            return Err(format!("unexpected status entry {:?}", String::from_utf8_lossy(field)));
        }
        paths.push(git_path(&field[3..])?);
        if matches!(field[0], b'R' | b'C') {
            fields.next();
        }
    }
    Ok(paths)
}

/// A `/`-separated path from Git as a native relative path, OS bytes kept.
fn git_path(bytes: &[u8]) -> Result<PathBuf, String> {
    let mut path = PathBuf::new();
    for component in bytes.split(|byte| *byte == b'/').filter(|component| !component.is_empty()) {
        path.push(path_from_bytes(component).map_err(|error| error.to_string())?);
    }
    Ok(path)
}

/// Git's own message for a failed call, without the wrapper's prefix.
fn reason(error: WorktreeError) -> String {
    match error {
        WorktreeError::GitCommand(stderr) => stderr,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests;
