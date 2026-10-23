//! Plain deletion of a checkout that the user approved deleting.
//!
//! [`delete_tree`] deletes everything under one directory, then the
//! directory itself, with no inventory to compare against and no watcher:
//! what is there when deletion reaches it is deleted. Approval covers the
//! checkout, not a list of files, so a file created during deletion is
//! deleted too. Its only stops are the boundaries that keep it inside that
//! checkout:
//!
//! - **No link is followed.** Every directory is reached through a handle on
//!   its parent and checked against the identity seen a moment before, so a
//!   directory swapped for a link is deleted as the link it now is. A link,
//!   junction, or other reparse point (a cloud placeholder, a deduplicated
//!   file) is removed as an entry and its target is never touched. FIFOs,
//!   sockets, and devices are unlinked without being opened, and a hard link
//!   loses only its own name.
//! - **No mount is crossed.** A directory listed in the mount table, or on
//!   another filesystem than the root (checked again before each descent, so
//!   a tree mounted during deletion is caught too), is kept along with the
//!   directories above it ([`Deletion::RetainedMounts`]).
//! - **The root must still be the bound directory.** A root that is now a
//!   link, a file, or another directory is outside the approval
//!   ([`Deletion::RootReplaced`]), and nothing is deleted.
//!
//! Failures are sorted by [`classify`]. A permanent one (a read-only disk, a
//! denial that making the directory writable did not fix) is recorded and
//! never retried, and deletion carries on with everything else
//! ([`Deletion::Permanent`]). A temporary one (another program has the file
//! open) and a directory that refills [`REFILL_PASSES`] times in a row are
//! waited out: after a full pass, the engine names the programs involved
//! through [`Holders`], reports a [`Wait`], and retries what is left after
//! each [`Backoff`] delay on the injected [`Clock`] until it is gone or the
//! user cancels ([`Deletion::Cancelled`]). The directory itself is the list
//! of what is left, so a rerun after a crash just deletes it again.

pub mod classify;
mod platform;
pub mod wait;

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use cap_std::ambient_authority;
use cap_std::fs::{Dir, Metadata};
use sniff::filesystem::query::FileIdentity;

pub use classify::{ErrorClass, classify, plain_cause};
pub use wait::{Holder, HolderReport, Holders, UsageHolders, Wait, WaitCause};

use super::clock::{Backoff, Clock, Slept};
use super::faults::{FsFaults, FsOp};
use super::mounts::{Mount, mounts_within};

/// Failed removals of one directory, each after relisting and deleting what
/// reappeared, before deletion stops and waits on it (N2).
pub const REFILL_PASSES: u32 = 3;

/// Deleted entries between two [`Progress::batch`] calls.
pub const BATCH_ENTRIES: u64 = 1024;

/// The directory to delete and the identity it was bound to when the removal
/// was approved: the quarantine after a move, or the checkout itself when
/// deleting in place.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub root: &'a Path,
    pub identity: &'a FileIdentity,
}

/// What deletion depends on outside the tree, all injected.
pub struct Env<'a> {
    pub clock: &'a dyn Clock,
    pub faults: &'a dyn FsFaults,
    /// The host's mount table (see [`mounts_within`]).
    pub mounts: &'a [Mount],
    pub holders: &'a dyn Holders,
    /// Polled at every entry and by every wait.
    pub cancelled: &'a dyn Fn() -> bool,
}

/// Told about deletion as it goes.
pub trait Progress {
    /// Called each time another [`BATCH_ENTRIES`] entries are deleted, with
    /// the total so far.
    fn batch(&mut self, _deleted: u64) {}

    /// Called when a wait starts, and again only when what it reports
    /// changes.
    fn waiting(&mut self, _wait: &Wait) {}

    /// Called once, when deletion ends however it ends, with the number of
    /// entries it deleted.
    fn finished(&mut self, _deleted: u64) {}
}

/// [`Progress`] that ignores everything.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Progress for Silent {}

/// An entry that could not be deleted, and why.
#[derive(Debug)]
pub struct Failure {
    pub path: PathBuf,
    pub cause: io::Error,
}

impl fmt::Display for Failure {
    /// One plain sentence, as R29 asks for.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} could not be deleted: {}", self.path.display(), plain_cause(&self.cause))
    }
}

/// How [`delete_tree`] ended.
#[derive(Debug)]
pub enum Deletion {
    /// Nothing is left at the root, including when nothing was there to
    /// begin with.
    Done,
    /// Everything else is deleted; these mounts, and the directories above
    /// them, are kept until they are unmounted.
    RetainedMounts(Vec<Mount>),
    /// Everything else is deleted; these entries (and the directories above
    /// them) are kept, and retrying will not help until their cause is
    /// fixed. Mounts kept as well are listed too.
    Permanent { failures: Vec<Failure>, retained_mounts: Vec<Mount> },
    /// The user cancelled; `waiting_on` is what a wait in progress was
    /// waiting for (empty when none was).
    Cancelled { waiting_on: Vec<PathBuf> },
    /// The root is no longer the bound directory. Nothing more was deleted.
    RootReplaced,
}

/// Deletes `target` (see the module docs).
pub fn delete_tree(target: Target<'_>, env: &Env<'_>, progress: &mut dyn Progress) -> Deletion {
    let mut run = Run {
        env,
        progress,
        root: target.root,
        root_volume: platform::volume(target.identity),
        expected: target.identity,
        nested: mounts_within(env.mounts, target.root).into_iter().cloned().collect(),
        failures: Vec::new(),
        failed: HashSet::new(),
        retained: Vec::new(),
        blocked: Vec::new(),
        denied: Vec::new(),
        deleted: 0,
        cancelled: false,
    };
    let deletion = run.drive(target, env);
    run.progress.finished(run.deleted);
    deletion
}

impl Run<'_, '_> {
    /// Passes over the tree until nothing is left, only what cannot be
    /// deleted is, or the user cancels, waiting between passes while
    /// something is only temporarily blocked.
    fn drive(&mut self, target: Target<'_>, env: &Env<'_>) -> Deletion {
        let run = self;
        let mut backoff = Backoff::new();
        let mut explained: Option<(Vec<PathBuf>, HolderReport)> = None;
        let mut reported: Option<Wait> = None;
        loop {
            let left = match run.pass() {
                Ok(left) => left,
                Err(RootChanged) => return Deletion::RootReplaced,
            };
            if run.cancelled {
                return Deletion::Cancelled { waiting_on: waiting_on(&reported) };
            }
            if left == Left::Gone {
                return Deletion::Done;
            }
            let mut blocked = std::mem::take(&mut run.blocked);
            let denied = std::mem::take(&mut run.denied);
            if !denied.is_empty() {
                // A denial is a wait only when a program is seen holding the
                // entry (a running program's image on Windows).
                let paths: Vec<PathBuf> = denied.iter().map(|(path, _)| path.clone()).collect();
                let report = env.holders.query(target.root, &paths);
                for (path, cause) in denied {
                    if report.holds(&path) {
                        blocked.push((path, WaitCause::InUse));
                    } else {
                        run.fail(path, cause);
                    }
                }
                let mut held: Vec<PathBuf> = blocked.iter().map(|(path, _)| path.clone()).collect();
                held.sort();
                explained = Some((held, report));
            }
            if blocked.is_empty() {
                return run.finish();
            }
            blocked.sort_by(|(left, _), (right, _)| left.cmp(right));
            let paths: Vec<PathBuf> = blocked.iter().map(|(path, _)| path.clone()).collect();
            let holders = match &explained {
                Some((known, report)) if *known == paths => report.clone(),
                _ => {
                    let report = env.holders.query(target.root, &paths);
                    explained = Some((paths.clone(), report.clone()));
                    report
                }
            };
            if reported.as_ref().is_some_and(|wait| wait.paths.len() > paths.len()) {
                backoff.reset();
            }
            let cause = if blocked.iter().all(|(_, cause)| *cause == WaitCause::Refilling) { WaitCause::Refilling } else { WaitCause::InUse };
            let wait = Wait { cause, paths, holders };
            if reported.as_ref() != Some(&wait) {
                run.progress.waiting(&wait);
                reported = Some(wait);
            }
            if backoff.wait(env.clock, env.cancelled) == Slept::Cancelled {
                return Deletion::Cancelled { waiting_on: waiting_on(&reported) };
            }
        }
    }
}

fn waiting_on(reported: &Option<Wait>) -> Vec<PathBuf> {
    reported.as_ref().map(|wait| wait.paths.clone()).unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Left {
    Gone,
    Kept,
}

/// What processing one entry came to.
enum Step {
    Done(Left),
    /// The entry changed between being looked at and being acted on; look
    /// again.
    Changed,
}

/// The root is not the bound directory.
struct RootChanged;

/// The marker error of a Windows deletion that found another object at the
/// path than the one it looked at.
#[derive(Debug)]
pub(crate) struct Replaced;

impl fmt::Display for Replaced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the entry was replaced while it was being deleted")
    }
}

impl std::error::Error for Replaced {}

fn is_replaced(error: &io::Error) -> bool {
    error.get_ref().is_some_and(|inner| inner.is::<Replaced>())
}

/// Looks before a changing entry counts as a failure.
const LOOKS: usize = 3;

struct Run<'a, 'p> {
    env: &'a Env<'a>,
    progress: &'p mut dyn Progress,
    root: &'a Path,
    expected: &'a FileIdentity,
    root_volume: u64,
    nested: Vec<Mount>,
    failures: Vec<Failure>,
    /// Paths of `failures`, never tried again.
    failed: HashSet<PathBuf>,
    retained: Vec<Mount>,
    /// This pass's temporary blocks.
    blocked: Vec<(PathBuf, WaitCause)>,
    /// This pass's denials that a holder may explain.
    denied: Vec<(PathBuf, io::Error)>,
    deleted: u64,
    cancelled: bool,
}

impl Run<'_, '_> {
    /// Deletes what it can under the root, then the root.
    fn pass(&mut self) -> Result<Left, RootChanged> {
        let (Some(parent_path), Some(name)) = (self.root.parent(), self.root.file_name()) else {
            self.fail(self.root.to_path_buf(), io::Error::new(io::ErrorKind::InvalidInput, "it has no parent directory"));
            return Ok(Left::Kept);
        };
        let parent = match Dir::open_ambient_dir(parent_path, ambient_authority()) {
            Ok(parent) => parent,
            Err(error) if classify(&error) == ErrorClass::Gone => return Ok(Left::Gone),
            Err(error) => return Ok(self.settle(self.root, error)),
        };
        let metadata = match parent.symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) => return Ok(self.settle(self.root, error)),
        };
        if metadata.file_type().is_symlink() || platform::is_reparse_point(&metadata) || !metadata.is_dir() {
            return Err(RootChanged);
        }
        let identity = match platform::identity(&metadata, self.root) {
            Ok(identity) => identity,
            Err(error) => return Ok(self.settle(self.root, error)),
        };
        if identity != *self.expected {
            return Err(RootChanged);
        }
        match self.directory(&parent, parent_path, name, self.root, &metadata, &identity, true) {
            Step::Done(left) => Ok(left),
            Step::Changed => Err(RootChanged),
        }
    }

    /// Deletes the entry `name` of `parent` (at `full`), looking at it afresh.
    fn entry(&mut self, parent: &Dir, parent_full: &Path, name: &OsStr) -> Left {
        let full = parent_full.join(name);
        if self.failed.contains(&full) || self.retained.iter().any(|mount| mount.mount_point == full) {
            return Left::Kept;
        }
        for _ in 0..LOOKS {
            let metadata = match parent.symlink_metadata(name) {
                Ok(metadata) => metadata,
                Err(error) => return self.settle(&full, error),
            };
            let identity = match platform::identity(&metadata, &full) {
                Ok(identity) => identity,
                Err(error) => return self.settle(&full, error),
            };
            let file_type = metadata.file_type();
            let step = if file_type.is_symlink() || platform::is_reparse_point(&metadata) {
                if file_type.is_dir() && platform::is_volume_mount(&full) {
                    return self.retain(&full);
                }
                self.remove(FsOp::RemoveFile, parent, parent_full, &full, &|| platform::remove_file(parent, name, &full, &identity))
            } else if file_type.is_dir() {
                if platform::volume(&identity) != self.root_volume || self.nested.iter().any(|mount| mount.mount_point == full) {
                    return self.retain(&full);
                }
                self.directory(parent, parent_full, name, &full, &metadata, &identity, false)
            } else {
                self.remove(FsOp::RemoveFile, parent, parent_full, &full, &|| platform::remove_file(parent, name, &full, &identity))
            };
            match step {
                Step::Done(left) => return left,
                Step::Changed => continue,
            }
        }
        self.fail(full, io::Error::other("it kept changing while it was being deleted"));
        Left::Kept
    }

    /// Empties the directory `name` of `parent`, then removes it, relisting
    /// what reappears up to [`REFILL_PASSES`] times.
    #[allow(clippy::too_many_arguments)]
    fn directory(&mut self, parent: &Dir, parent_full: &Path, name: &OsStr, full: &Path, metadata: &Metadata, identity: &FileIdentity, is_root: bool) -> Step {
        let mut refills = 0;
        loop {
            let dir = match self.open_dir(parent, name, full, metadata, identity) {
                Ok(dir) => dir,
                Err(step) => return step,
            };
            let emptied = self.clear(&dir, full, is_root);
            // Closed first: Windows cannot remove a directory this process
            // holds open.
            drop(dir);
            if !emptied {
                return Step::Done(Left::Kept);
            }
            let removed = self.attempt(FsOp::RemoveDir, parent, parent_full, full, &|| platform::remove_dir(parent, name, full, identity));
            match removed {
                Ok(()) => {
                    self.count_deleted();
                    return Step::Done(Left::Gone);
                }
                Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => {
                    refills += 1;
                    if refills >= REFILL_PASSES {
                        self.blocked.push((full.to_path_buf(), WaitCause::Refilling));
                        return Step::Done(Left::Kept);
                    }
                }
                Err(error) if is_replaced(&error) => return Step::Changed,
                Err(error) => return Step::Done(self.settle(full, error)),
            }
        }
    }

    /// Opens the directory `name` of `parent`, which must still be the one
    /// with `identity`. A directory this user cannot list is made listable
    /// once (on Unix).
    fn open_dir(&mut self, parent: &Dir, name: &OsStr, full: &Path, metadata: &Metadata, identity: &FileIdentity) -> Result<Dir, Step> {
        let open = || match self.env.faults.inject(FsOp::ReadDir, full) {
            Some(error) => Err(error),
            None => parent.open_dir(name),
        };
        // A failed open may be a link swapped in since the look (which the
        // handle refuses to follow out of the tree); that is a change, not a
        // failure, and must never be made writable.
        let unchanged = || {
            parent.symlink_metadata(name).is_ok_and(|now| {
                !now.file_type().is_symlink()
                    && !platform::is_reparse_point(&now)
                    && platform::identity(&now, full).is_ok_and(|now| now == *identity)
            })
        };
        let opened = match open() {
            Err(_) if !unchanged() => return Err(Step::Changed),
            Err(error) if platform::FIX_DENIAL_BY_MODE && classify(&error) == ErrorClass::Denied => {
                let fixed = match self.env.faults.inject(FsOp::MakeWritable, full) {
                    Some(error) => Err(error),
                    None => platform::make_traversable(parent, name, metadata),
                };
                if fixed.is_ok() { open() } else { Err(error) }
            }
            opened => opened,
        };
        let dir = opened.map_err(|error| Step::Done(self.settle(full, error)))?;
        match platform::dir_identity(&dir, full) {
            Ok(now) if now == *identity => Ok(dir),
            Ok(_) => Err(Step::Changed),
            Err(error) => Err(Step::Done(self.settle(full, error))),
        }
    }

    /// Deletes every entry of `dir`; whether all of them are gone. The root's
    /// `.git` goes last, so an interrupted deletion still leaves a checkout
    /// Git recognizes until the rest is gone.
    fn clear(&mut self, dir: &Dir, full: &Path, is_root: bool) -> bool {
        let list = || match self.env.faults.inject(FsOp::ReadDir, full) {
            Some(error) => Err(error),
            None => dir.entries().and_then(|entries| entries.map(|entry| entry.map(|entry| entry.file_name())).collect::<io::Result<Vec<OsString>>>()),
        };
        // On Linux the directory handle needs no permission to open, so an
        // unlistable directory is refused here rather than at the open.
        let listed = match list() {
            Err(error) if platform::FIX_DENIAL_BY_MODE && classify(&error) == ErrorClass::Denied => {
                let fixed = match self.env.faults.inject(FsOp::MakeWritable, full) {
                    Some(error) => Err(error),
                    None => platform::make_writable(dir),
                };
                if fixed.is_ok() { list() } else { Err(error) }
            }
            listed => listed,
        };
        let mut names = match listed {
            Ok(names) => names,
            Err(error) => {
                self.settle(full, error);
                return false;
            }
        };
        if is_root && let Some(at) = names.iter().position(|name| name == ".git") {
            let git = names.remove(at);
            names.push(git);
        }
        let mut emptied = true;
        for name in names {
            if self.cancelled || (self.env.cancelled)() {
                self.cancelled = true;
                return false;
            }
            if self.entry(dir, full, &name) == Left::Kept {
                emptied = false;
            }
        }
        emptied
    }

    /// Removes a non-directory entry.
    fn remove(&mut self, op: FsOp, parent: &Dir, parent_full: &Path, full: &Path, call: &dyn Fn() -> io::Result<()>) -> Step {
        match self.attempt(op, parent, parent_full, full, call) {
            Ok(()) => {
                self.count_deleted();
                Step::Done(Left::Gone)
            }
            Err(error) if is_replaced(&error) => Step::Changed,
            Err(error) => Step::Done(self.settle(full, error)),
        }
    }

    /// Runs `call`; on a Unix permission denial, makes `parent` writable
    /// through its handle and runs it once more.
    fn attempt(&mut self, op: FsOp, parent: &Dir, parent_full: &Path, full: &Path, call: &dyn Fn() -> io::Result<()>) -> io::Result<()> {
        let run = || match self.env.faults.inject(op, full) {
            Some(error) => Err(error),
            None => call(),
        };
        match run() {
            Err(error) if platform::FIX_DENIAL_BY_MODE && classify(&error) == ErrorClass::Denied => {
                let fixed = match self.env.faults.inject(FsOp::MakeWritable, parent_full) {
                    Some(error) => Err(error),
                    None => platform::make_writable(parent),
                };
                if fixed.is_ok() { run() } else { Err(error) }
            }
            result => result,
        }
    }

    /// Sorts a failure at `full` into gone, a wait, or a permanent failure.
    fn settle(&mut self, full: &Path, error: io::Error) -> Left {
        match classify(&error) {
            ErrorClass::Gone => Left::Gone,
            ErrorClass::Temporary => {
                self.blocked.push((full.to_path_buf(), WaitCause::InUse));
                Left::Kept
            }
            ErrorClass::Denied if !platform::FIX_DENIAL_BY_MODE => {
                self.denied.push((full.to_path_buf(), error));
                Left::Kept
            }
            ErrorClass::Denied | ErrorClass::Permanent => {
                self.fail(full.to_path_buf(), error);
                Left::Kept
            }
        }
    }

    fn fail(&mut self, path: PathBuf, cause: io::Error) {
        if self.failed.insert(path.clone()) {
            self.failures.push(Failure { path, cause });
        }
    }

    fn retain(&mut self, full: &Path) -> Left {
        let mount = self
            .nested
            .iter()
            .find(|mount| mount.mount_point == full)
            .cloned()
            .unwrap_or_else(|| Mount { mount_point: full.to_path_buf(), source: "another filesystem".to_string(), fs_type: String::new(), super_options: String::new() });
        if !self.retained.contains(&mount) {
            self.retained.push(mount);
        }
        Left::Kept
    }

    fn count_deleted(&mut self) {
        self.deleted += 1;
        if self.deleted.is_multiple_of(BATCH_ENTRIES) {
            self.progress.batch(self.deleted);
        }
    }

    fn finish(&mut self) -> Deletion {
        let (mut failures, retained, root) = (std::mem::take(&mut self.failures), std::mem::take(&mut self.retained), self.root);
        if failures.is_empty() && retained.is_empty() {
            // Kept with no recorded reason cannot happen; never report it as
            // done.
            failures.push(Failure { path: root.to_path_buf(), cause: io::Error::other("it could not be emptied") });
        }
        if failures.is_empty() { Deletion::RetainedMounts(retained) } else { Deletion::Permanent { failures, retained_mounts: retained } }
    }
}

#[cfg(test)]
mod tests;
