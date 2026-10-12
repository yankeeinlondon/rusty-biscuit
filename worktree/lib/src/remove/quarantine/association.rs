//! Moving a checkout into its quarantine while Git keeps it registered.
//!
//! The checkout stays the same worktree throughout: its admin directory, index,
//! branch, and lock keep applying to it at the new path. Three Git facts shape
//! the move (spike S3 in `2026-10-05-graceful-file-contention`):
//!
//! - `git worktree move` replaces an existing empty directory, so it is never
//!   used. The move is [`rename_no_replace`] to a same-depth sibling.
//! - After the rename, exactly one file needs rewriting: the admin directory's
//!   `gitdir`, in the format it already has (absolute, or relative to the
//!   admin directory itself). The checkout's `.git` file is left unchanged,
//!   which only works when its link survives the move; a link that would not
//!   refuses before anything moves. `git worktree repair` is never run, since
//!   it rewrites other worktrees' links.
//! - Git will not move or remove a locked worktree or one with submodules, and
//!   this move refuses both too, before any change. A lock is never removed.
//!
//! [`plan_move`] checks all of this and fixes the quarantine path, which the
//! recovery record then binds. [`move_to_quarantine`] checks it again
//! immediately before the rename, then moves, rewrites, and verifies the
//! result by reading it back with Git. When the rewrite fails, the move is
//! undone only while that is provably safe; otherwise the quarantine is left
//! in place and its actual state reported.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use sniff::filesystem::query::FileIdentity;

use crate::availability::{self, EntryKind};
use crate::git::{git_from, git_from_bytes_with_env};
use crate::remove::admin_entry::{
    self, AdminEntry, AssociationError, BackReferenceError, path_from_bytes, read_back_reference, same_location,
};
use crate::remove::handoff::is_within;
use crate::remove::outcome::RegistrationState;
use crate::remove::recovery::{OperationId, storage};
use crate::remove::repair::identity_change;
use crate::worktree::{WorktreeEntry, parse_worktree_list};

use super::naming;
use super::rename::{self, RenameError, RenameFailure};

/// A checked move of one linked checkout to its quarantine. Every field is
/// what the checks saw; [`move_to_quarantine`] requires each to still hold.
#[derive(Debug, Clone)]
pub struct MovePlan {
    pub original: PathBuf,
    /// A hidden sibling of `original`; see [`naming::quarantine_path`].
    pub quarantine: PathBuf,
    /// The checkout directory's identity, which the move preserves.
    pub identity: FileIdentity,
    pub common_dir: PathBuf,
    pub admin: AdminEntry,
    entry: WorktreeEntry,
    /// The admin `gitdir` file's bytes, restored by a rollback.
    back_reference: Vec<u8>,
    /// The same file naming the quarantine, in the same format.
    rewritten: Vec<u8>,
}

/// Why the checkout cannot be moved. Nothing was changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoveRefusal {
    #[error("{} is not a real directory", .0.display())]
    NotARealDirectory(PathBuf),
    /// Git's lock on the worktree, with its reason when one was given.
    #[error("the worktree is locked{}", .reason.as_deref().map(|reason| format!(": {reason}")).unwrap_or_default())]
    Locked { reason: Option<String> },
    /// A submodule: a gitlink entry in the index (its path), or the admin
    /// directory's `modules` directory.
    #[error("the worktree contains a submodule ({}), which Git cannot move or remove", .entry.display())]
    Submodules { entry: PathBuf },
    /// A shape this move does not support, such as the main checkout or a
    /// `.git` link that would need rewriting.
    #[error("the worktree can't be moved aside: {0}")]
    Unsupported(String),
    #[error(transparent)]
    Association(AssociationError),
    /// What the plan was made from no longer holds.
    #[error("the worktree changed after it was checked: {0}")]
    Changed(String),
    #[error("the worktree can't be inspected: {0}")]
    Uninspectable(String),
}

/// Why the rewritten registration could not be confirmed after the rename.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssociationFailure {
    /// The directory at the quarantine is not the checkout that was moved.
    #[error("the directory at the quarantine is not the checkout that was moved")]
    IdentityChanged,
    #[error("Git's record of the worktree's location could not be rewritten: {0}")]
    Write(String),
    #[error("Git does not read the worktree at the quarantine: {0}")]
    Unverified(String),
}

/// Where the checkout directory is, by identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckoutLocation {
    Original,
    Quarantine,
    Unknown,
}

/// The state a move that was not undone left behind, observed afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveState {
    pub checkout: CheckoutLocation,
    /// Something other than the checkout is at the original path. It is
    /// never moved, overwritten, or traversed.
    pub original_recreated: bool,
    pub registration: RegistrationState,
}

/// Why [`move_to_quarantine`] did not finish.
#[derive(Debug, thiserror::Error)]
pub enum MoveError {
    /// Refused before the rename; nothing changed.
    #[error(transparent)]
    Refused(MoveRefusal),
    /// The rename failed; nothing moved.
    #[error("the worktree could not be moved aside: {0}")]
    Rename(RenameError),
    /// The rename succeeded and the registration could not be updated, so
    /// the checkout was moved back and Git's record restored, both verified.
    #[error("{cause}; the worktree was moved back unchanged")]
    RolledBack { cause: AssociationFailure },
    /// The rename succeeded, the registration could not be updated, and the
    /// move was not (or could not be) undone.
    #[error("{cause}; the worktree was left at the quarantine because {reason}")]
    Stranded { cause: AssociationFailure, reason: String, state: MoveState },
}

/// How a [`MoveError`] is reported, which the caller maps to an exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveErrorClass {
    /// Ineligible or changed state before mutation (exit 3).
    Refused,
    /// This environment cannot make the move, and nothing changed (exit 4):
    /// a Windows holder (sharing) or a filesystem without a no-replacement
    /// rename.
    BlockedByEnvironment,
    /// An operational failure. Never evidence of another process's usage.
    Failed,
}

impl MoveError {
    pub fn class(&self) -> MoveErrorClass {
        match self {
            Self::Refused(_) => MoveErrorClass::Refused,
            Self::Rename(error) if matches!(error.kind, RenameFailure::Sharing | RenameFailure::Unsupported) => {
                MoveErrorClass::BlockedByEnvironment
            }
            Self::Rename(_) | Self::RolledBack { .. } | Self::Stranded { .. } => MoveErrorClass::Failed,
        }
    }

    /// Whether the checkout and its registration are where they were. A
    /// rolled-back move counts: both were verified restored.
    pub fn nothing_changed(&self) -> bool {
        !matches!(self, Self::Stranded { .. })
    }
}

/// Checks that `entry`, a linked worktree, can be moved to its quarantine
/// under `operation`, and fixes the quarantine path.
///
/// ## Errors
///
/// A [`MoveRefusal`]: the target is not a real directory, the main checkout,
/// locked, or holds submodules; its links are in a shape the move cannot
/// keep; or Git's records of it can't be read or disagree.
pub fn plan_move(base: &Path, entry: &WorktreeEntry, operation: &OperationId) -> Result<MovePlan, MoveRefusal> {
    if entry.is_main {
        return Err(MoveRefusal::Unsupported("it is the repository's main checkout".into()));
    }
    let original = entry.path.clone();
    let quarantine = naming::quarantine_path(&original, operation)
        .ok_or_else(|| MoveRefusal::Unsupported("its path has no parent directory".into()))?;
    let identity = real_directory_identity(&original)?;
    let common_dir = admin_entry::common_git_dir(base)
        .map_err(|error| MoveRefusal::Association(AssociationError::CommonDirUnknown(error.to_string())))?;
    let admin = admin_entry::admin_entry_in(&common_dir, &original).map_err(MoveRefusal::Association)?;
    let back_reference = read_gitdir_file(&admin)?;
    let (Some(from), Some(to)) = (original.file_name(), quarantine.file_name()) else {
        return Err(MoveRefusal::Unsupported("its path has no name".into()));
    };
    let rewritten = rewrite_back_reference(&back_reference, from, to).ok_or_else(|| {
        MoveRefusal::Unsupported("Git's record spells the worktree's name differently from its path".into())
    })?;
    let plan = MovePlan { original, quarantine, identity, common_dir, admin, entry: entry.clone(), back_reference, rewritten };
    plan.check(base)?;
    Ok(plan)
}

/// Moves the checkout to `plan.quarantine` and points Git's record at it.
///
/// Immediately before the rename every [`plan_move`] check runs again
/// against the plan. After it, the moved directory's identity is compared,
/// the admin `gitdir` is replaced atomically, and a fresh `git worktree list`
/// must show the same worktree, not prunable, at the quarantine and nothing
/// at the original. On Windows the quarantine is then marked hidden.
///
/// When that update fails, the move is undone only if the original path is
/// still absent and `activity_observed` reports nothing unexpected (nothing
/// has been deleted yet, by construction). Undoing never replaces anything at
/// the original path.
///
/// ## Errors
///
/// [`MoveError`]: [`MoveError::Refused`] and [`MoveError::Rename`] changed
/// nothing; [`MoveError::RolledBack`] restored everything; and
/// [`MoveError::Stranded`] describes what was left where.
pub fn move_to_quarantine(base: &Path, plan: &MovePlan, activity_observed: &dyn Fn() -> bool) -> Result<(), MoveError> {
    move_with(base, plan, activity_observed, &Faults::default())
}

/// Test seams for the steps after the rename.
#[derive(Default)]
pub(crate) struct Faults {
    /// Fail the `gitdir` rewrite before writing.
    pub fail_write: bool,
    /// Write these bytes instead of the rewritten record.
    pub write_instead: Option<Vec<u8>>,
    /// Runs right after the rename succeeds.
    pub after_rename: Option<Box<dyn Fn()>>,
}

pub(crate) fn move_with(
    base: &Path,
    plan: &MovePlan,
    activity_observed: &dyn Fn() -> bool,
    faults: &Faults,
) -> Result<(), MoveError> {
    plan.check(base).map_err(MoveError::Refused)?;
    rename::rename_no_replace(&plan.original, &plan.quarantine).map_err(MoveError::Rename)?;
    if let Some(after_rename) = &faults.after_rename {
        after_rename();
    }
    if let Err(cause) = associate(base, plan, faults) {
        return Err(roll_back(base, plan, cause, activity_observed));
    }
    // Cosmetic: the leading dot already marks the quarantine elsewhere, and
    // a visible quarantine is still a correct one.
    #[cfg(windows)]
    let _ = naming::hide(&plan.quarantine);
    Ok(())
}

impl MovePlan {
    /// Every condition the move relies on, read fresh.
    fn check(&self, base: &Path) -> Result<(), MoveRefusal> {
        if real_directory_identity(&self.original)? != self.identity {
            return Err(MoveRefusal::Changed("a different directory is now at its path".into()));
        }
        let entries = worktree_list(base).map_err(MoveRefusal::Changed)?;
        listed_once(&entries, &self.original, &self.entry).map_err(MoveRefusal::Changed)?;
        let admin = admin_entry::admin_entry_in(&self.common_dir, &self.original).map_err(MoveRefusal::Association)?;
        if admin != self.admin {
            return Err(MoveRefusal::Changed(format!("Git now records it in {}", admin.dir.display())));
        }
        if read_gitdir_file(&self.admin)? != self.back_reference {
            return Err(MoveRefusal::Changed("Git's record of its location changed".into()));
        }
        check_unlocked(&self.admin)?;
        check_forward_link(&self.original, &self.admin.dir)?;
        check_no_submodules(base, &self.admin)
    }
}

fn real_directory_identity(path: &Path) -> Result<FileIdentity, MoveRefusal> {
    match availability::inspect(path) {
        Ok(EntryKind::Directory) => {}
        Ok(_) => return Err(MoveRefusal::NotARealDirectory(path.to_path_buf())),
        Err(error) => return Err(MoveRefusal::Uninspectable(format!("{}: {error}", path.display()))),
    }
    rename::identity(path).map_err(|error| MoveRefusal::Uninspectable(format!("{}: {error}", path.display())))
}

fn read_gitdir_file(admin: &AdminEntry) -> Result<Vec<u8>, MoveRefusal> {
    let path = admin.dir.join("gitdir");
    fs::read(&path).map_err(|error| MoveRefusal::Uninspectable(format!("{}: {error}", path.display())))
}

/// `recorded` (an admin `gitdir` file) with its checkout name `from`
/// replaced by `to`, keeping every other byte: the prefix, whether it is
/// relative, its separators, and its line ending. `None` unless the record
/// ends in `<sep><from><sep>.git`.
fn rewrite_back_reference(recorded: &[u8], from: &OsStr, to: &OsStr) -> Option<Vec<u8>> {
    let is_separator = |byte: &u8| *byte == b'/' || (cfg!(windows) && *byte == b'\\');
    let end = recorded.iter().rposition(|byte| !matches!(byte, b'\n' | b'\r')).map_or(0, |last| last + 1);
    let (line, ending) = recorded.split_at(end);
    let (separator, rest) = line.strip_suffix(b".git")?.split_last()?;
    if !is_separator(separator) {
        return None;
    }
    let name_start = rest.iter().rposition(is_separator).map_or(0, |last| last + 1);
    if &rest[name_start..] != from.as_encoded_bytes() {
        return None;
    }
    let mut rewritten = rest[..name_start].to_vec();
    rewritten.extend_from_slice(to.as_encoded_bytes());
    rewritten.push(*separator);
    rewritten.extend_from_slice(b".git");
    rewritten.extend_from_slice(ending);
    Some(rewritten)
}

fn check_unlocked(admin: &AdminEntry) -> Result<(), MoveRefusal> {
    let lock = admin.dir.join("locked");
    match availability::inspect(&lock) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(MoveRefusal::Uninspectable(format!("{}: {error}", lock.display()))),
        // Git treats any entry named `locked` as a lock.
        Ok(_) => {
            let reason = fs::read(&lock).ok().map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string());
            Err(MoveRefusal::Locked { reason: reason.filter(|reason| !reason.is_empty()) })
        }
    }
}

/// Requires `checkout/.git` to be a file linking to `admin_dir` in a form
/// that still resolves from a same-depth sibling: absolute, or relative with
/// only leading `..` components, since the first `..` leaves the checkout
/// for the parent both share.
fn check_forward_link(checkout: &Path, admin_dir: &Path) -> Result<(), MoveRefusal> {
    let dot_git = checkout.join(".git");
    match availability::inspect(&dot_git) {
        Ok(EntryKind::File) => {}
        Ok(EntryKind::Directory) => return Err(MoveRefusal::Unsupported("its .git is a directory, not a worktree link".into())),
        Ok(EntryKind::Link) => return Err(MoveRefusal::Unsupported("its .git is a link".into())),
        Err(error) => return Err(MoveRefusal::Uninspectable(format!("{}: {error}", dot_git.display()))),
    }
    let bytes = fs::read(&dot_git).map_err(|error| MoveRefusal::Uninspectable(format!("{}: {error}", dot_git.display())))?;
    let unsupported = |why: &str| MoveRefusal::Unsupported(format!("its .git file {why}"));
    let end = bytes.iter().rposition(|byte| !matches!(byte, b'\n' | b'\r')).map_or(0, |last| last + 1);
    let target = bytes[..end].strip_prefix(b"gitdir: ").ok_or_else(|| unsupported("does not start with \"gitdir: \""))?;
    if target.is_empty() || target.iter().any(|byte| matches!(byte, b'\n' | b'\r' | 0)) {
        return Err(unsupported("is not one path"));
    }
    let target = path_from_bytes(target).map_err(|_: BackReferenceError| unsupported("is not valid UTF-8"))?;
    let resolved = if target.is_relative() {
        let mut parents = 0;
        let mut named = false;
        for component in target.components() {
            match component {
                Component::ParentDir if !named => parents += 1,
                Component::Normal(_) => named = true,
                _ => return Err(unsupported("has a relative link that would need rewriting after a move")),
            }
        }
        if parents == 0 {
            return Err(unsupported("has a relative link that would need rewriting after a move"));
        }
        checkout.join(&target)
    } else {
        target
    };
    if !same_location(&resolved, admin_dir) {
        return Err(MoveRefusal::Changed(format!("its .git file names {}", resolved.display())));
    }
    if is_within(admin_dir, checkout) {
        return Err(unsupported("names a worktree record inside the checkout"));
    }
    Ok(())
}

/// Refuses as Git does (`validate_no_submodules`): a `modules` directory in
/// the admin directory, or a gitlink (mode `160000`) in the worktree's index.
fn check_no_submodules(base: &Path, admin: &AdminEntry) -> Result<(), MoveRefusal> {
    let modules = admin.dir.join("modules");
    match availability::inspect(&modules) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(MoveRefusal::Uninspectable(format!("{}: {error}", modules.display()))),
        Ok(_) => return Err(MoveRefusal::Submodules { entry: modules }),
    }
    let index = admin.index();
    let listing = git_from_bytes_with_env(base, base, &["ls-files", "--stage", "-z"], &[("GIT_INDEX_FILE", index.as_os_str())])
        .map_err(|error| MoveRefusal::Uninspectable(format!("{}: {error}", index.display())))?;
    // Each record is `<mode> <object> <stage>\t<path>`.
    for record in listing.split(|&byte| byte == 0) {
        if let Some(rest) = record.strip_prefix(b"160000 ") {
            let path = rest.iter().position(|&byte| byte == b'\t').map_or(&[][..], |tab| &rest[tab + 1..]);
            let entry = path_from_bytes(path).unwrap_or_else(|_| PathBuf::from(String::from_utf8_lossy(path).into_owned()));
            return Err(MoveRefusal::Submodules { entry });
        }
    }
    Ok(())
}

fn worktree_list(base: &Path) -> Result<Vec<WorktreeEntry>, String> {
    git_from(base, base, &["worktree", "list", "--porcelain"])
        .map(|listing| parse_worktree_list(&listing))
        .map_err(|error| format!("Git's worktree list could not be read: {error}"))
}

/// Requires `entries` to show `entry`'s worktree exactly once at `path`, not
/// prunable.
fn listed_once(entries: &[WorktreeEntry], path: &Path, entry: &WorktreeEntry) -> Result<(), String> {
    let found: Vec<&WorktreeEntry> = entries.iter().filter(|listed| same_location(&listed.path, path)).collect();
    let [listed] = found.as_slice() else {
        return Err(format!("Git's worktree list shows {} {} times", path.display(), found.len()));
    };
    if listed.prunable.is_some() {
        return Err(format!("Git marks {} prunable", path.display()));
    }
    match identity_change(entry, listed) {
        Some(change) => Err(format!("Git's worktree list {change}")),
        None => Ok(()),
    }
}

fn associate(base: &Path, plan: &MovePlan, faults: &Faults) -> Result<(), AssociationFailure> {
    match rename::identity(&plan.quarantine) {
        Ok(identity) if identity == plan.identity => {}
        _ => return Err(AssociationFailure::IdentityChanged),
    }
    if faults.fail_write {
        return Err(AssociationFailure::Write("injected failure".into()));
    }
    let bytes = faults.write_instead.as_deref().unwrap_or(&plan.rewritten);
    write_gitdir(&plan.admin.dir, bytes).map_err(|error| AssociationFailure::Write(error.to_string()))?;
    verify_registration(base, plan, &plan.quarantine, &plan.original).map_err(AssociationFailure::Unverified)
}

/// Requires Git's record and a fresh listing to place the worktree at `at`
/// and nowhere at `not_at`, and its `.git` file to still link back.
fn verify_registration(base: &Path, plan: &MovePlan, at: &Path, not_at: &Path) -> Result<(), String> {
    match read_back_reference(&plan.admin.dir) {
        Ok(named) if same_location(&named, &at.join(".git")) => {}
        Ok(named) => return Err(format!("its record names {}", named.display())),
        Err(error) => return Err(error.to_string()),
    }
    check_forward_link(at, &plan.admin.dir).map_err(|refusal| refusal.to_string())?;
    let entries = worktree_list(base)?;
    listed_once(&entries, at, &plan.entry)?;
    if entries.iter().any(|listed| same_location(&listed.path, not_at)) {
        return Err(format!("Git's worktree list still shows {}", not_at.display()));
    }
    Ok(())
}

/// Replaces `<admin_dir>/gitdir` atomically with `bytes`, keeping its
/// permissions: Git's own file, which other users of a shared repository
/// may need to read.
fn write_gitdir(admin_dir: &Path, bytes: &[u8]) -> io::Result<()> {
    let path = admin_dir.join("gitdir");
    let permissions = fs::symlink_metadata(&path)?.permissions();
    let nonce = OperationId::generate().map_err(|error| io::Error::other(error.to_string()))?;
    let temp = admin_dir.join(format!("gitdir.wt-{nonce}.tmp"));
    let result = (|| {
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.write_all(bytes)?;
        file.set_permissions(permissions)?;
        storage::flush(&file)?;
        drop(file);
        // Replacing Git's file is the point; the no-replacement rule is for
        // the checkout move.
        fs::rename(&temp, &path)?;
        storage::sync_dir(admin_dir)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Undoes the move when that is provably safe, and reports which happened.
fn roll_back(base: &Path, plan: &MovePlan, cause: AssociationFailure, activity_observed: &dyn Fn() -> bool) -> MoveError {
    let stranded = |cause: AssociationFailure, reason: &str| MoveError::Stranded {
        cause,
        reason: reason.to_string(),
        state: observe(plan),
    };
    // Moving back whatever is at the quarantine would put a stranger's
    // directory at the original path.
    if cause == AssociationFailure::IdentityChanged {
        return stranded(cause, "the directory there is not the one that was moved");
    }
    if activity_observed() {
        return stranded(cause, "unexpected activity was observed");
    }
    match availability::inspect(&plan.original) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Ok(_) => return stranded(cause, "something now exists at the original path"),
        Err(error) => return stranded(cause, &format!("the original path can't be inspected: {error}")),
    }
    // No replacement: a directory recreated since the check above stays.
    if let Err(error) = rename::rename_no_replace(&plan.quarantine, &plan.original) {
        return stranded(cause, &format!("moving it back failed: {error}"));
    }
    if fs::read(plan.admin.dir.join("gitdir")).ok().as_deref() != Some(plan.back_reference.as_slice())
        && let Err(error) = write_gitdir(&plan.admin.dir, &plan.back_reference)
    {
        return stranded(cause, &format!("Git's record could not be restored: {error}"));
    }
    let restored = match rename::identity(&plan.original) {
        Ok(identity) if identity == plan.identity => verify_registration(base, plan, &plan.original, &plan.quarantine),
        _ => Err("the directory moved back is not the checkout".into()),
    };
    match restored {
        Ok(()) => MoveError::RolledBack { cause },
        Err(problem) => stranded(cause, &format!("the move back could not be verified: {problem}")),
    }
}

/// What is where now, read without trusting what the move intended.
fn observe(plan: &MovePlan) -> MoveState {
    let original = rename::identity(&plan.original).ok();
    let checkout = if original == Some(plan.identity) {
        CheckoutLocation::Original
    } else if rename::identity(&plan.quarantine).ok() == Some(plan.identity) {
        CheckoutLocation::Quarantine
    } else {
        CheckoutLocation::Unknown
    };
    let registration = match read_back_reference(&plan.admin.dir) {
        Ok(named) if same_location(&named, &plan.original.join(".git")) => RegistrationState::AtOriginal,
        Ok(named) if same_location(&named, &plan.quarantine.join(".git")) => RegistrationState::AtQuarantine,
        Err(BackReferenceError::Missing) if !plan.admin.dir.exists() => RegistrationState::Removed,
        _ => RegistrationState::Unknown,
    };
    MoveState { checkout, original_recreated: original.is_some_and(|identity| identity != plan.identity), registration }
}

#[cfg(test)]
mod tests;
