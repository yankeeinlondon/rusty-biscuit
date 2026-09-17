//! Ownership of one launch's overlay root.
//!
//! An overlay root belongs to exactly one launch. [`OverlayLease`] creates it,
//! holds an exclusive OS file lock on a sibling `<root>.lock` for as long as the
//! launch lives, and removes both when the last holder drops it.
//!
//! The lock — not a PID — is the liveness signal, because the OS releases it
//! whenever the owning process ends, including an `_exit` on a second Ctrl+C or
//! a crash that never runs `Drop`. [`sweep_abandoned_overlays`] reclaims those
//! roots: a root whose lock it can take has no live owner.
//!
//! Ordering is what makes the sweep safe against a launch that is still
//! starting. A lease takes the lock *before* it creates the root, and the sweep
//! only considers roots that already exist, so it can never observe a root
//! whose owner has not yet locked it.
//!
//! The sweep reclaims a root only when its lock file exists, can be locked, and
//! no `<root>.retained` marker exists — checked again once the lock is held. A
//! lock file is removed only by a holder of its lock: together with its root,
//! or to retain the root.
//!
//! A lease released in-process first applies its [`WriteBack`]; a swept root
//! never does. When a changed file cannot be written back, the root is the only
//! copy of that state, so release keeps it and, while still holding the lock,
//! renames the lock file to the marker. That rename allocates no data, so it
//! protects the root even on the full disk that may have failed the write-back;
//! if it fails the lock file is removed instead, and failing that a new marker
//! is created. The notice naming what to recover is then written into the
//! marker, best effort — it also goes to stderr. Only the user removes a
//! retained root.

use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::write_back::{HostIo, OverlayIo, WriteBack, WriteBackOutcome};
use crate::provider::Provider;

/// What releasing a lease did with its root.
#[derive(Debug)]
pub enum OverlayRelease {
    /// Nothing changed was left unpersisted, so the root was removed. A root
    /// that could not be removed now is reclaimed by a later sweep.
    Removed(WriteBackOutcome),
    /// A changed file could not be written back, so the root was kept.
    Retained {
        provider: Provider,
        root: PathBuf,
        /// The `<root>.retained` marker holding the recovery notice; `None`
        /// when the notice could not be written there.
        marker: Option<PathBuf>,
        /// Whether a later sweep is kept from reclaiming `root`. `false` only
        /// when the lock file could be neither renamed nor removed and no marker
        /// could be created.
        protected: bool,
        outcome: WriteBackOutcome,
    },
}

impl OverlayRelease {
    /// The actionable report for a retained root: every unpersisted file's
    /// overlay copy and source path, and the error. Never file contents.
    ///
    /// `None` when the root was removed.
    pub fn recovery_notice(&self) -> Option<String> {
        let Self::Retained {
            provider,
            root,
            marker,
            protected,
            outcome,
        } = self
        else {
            return None;
        };
        let mut notice = retention_notice(*provider, root, outcome);
        if !protected {
            notice.push_str(
                "The root could not be protected from later sweeps, so a later launch may remove it: recover the files now.\n",
            );
        } else if marker.is_none() {
            notice.push_str("This notice could not be saved beside the root, but later launches will not remove it.\n");
        }
        Some(notice)
    }
}

/// A launch's claim on its overlay root. Releasing it — explicitly through
/// [`release`](Self::release), or by dropping it — applies the attached
/// [`WriteBack`], then removes the root unless a changed file could not be
/// written back.
///
/// Removal never follows a symbolic link, so the Unix mirror's links into the
/// user's provider root are unlinked, not traversed.
#[derive(Debug)]
pub struct OverlayLease {
    root: PathBuf,
    lock_path: PathBuf,
    lock: Option<File>,
    write_back: Option<WriteBack>,
    io: Arc<dyn OverlayIo>,
}

impl OverlayLease {
    /// Create `root`, which must not exist, and lock it for this launch.
    ///
    /// ## Errors
    ///
    /// Any failure creating the parent directory, the lock file, or the root.
    /// `AlreadyExists` means another launch owns the name. Nothing is left
    /// behind on failure.
    pub fn acquire(root: &Path) -> io::Result<Self> {
        let parent = root.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "overlay root has no parent")
        })?;
        fs::create_dir_all(parent)?;
        let lock_path = lock_path_for(root);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&lock_path)?;
        if let Err(error) = lock.lock().and_then(|()| fs::create_dir(root)) {
            // The root is not removed here: on `AlreadyExists` it is not ours.
            drop(lock);
            let _ = fs::remove_file(&lock_path);
            return Err(error);
        }
        Ok(Self {
            root: root.to_path_buf(),
            lock_path,
            lock: Some(lock),
            write_back: None,
            io: Arc::new(HostIo),
        })
    }

    #[cfg(test)]
    pub(super) fn set_io(&mut self, io: Arc<dyn OverlayIo>) {
        self.io = io;
    }

    /// The write-back record applied when this lease is released, created
    /// empty for `provider` on first use.
    pub fn write_back(&mut self, provider: crate::provider::Provider) -> &mut WriteBack {
        self.write_back.get_or_insert_with(|| WriteBack::new(provider))
    }

    /// The overlay root this lease owns.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Apply the write-back and remove or retain the root, reporting which.
    ///
    /// Dropping an unreleased lease does the same and prints
    /// [`OverlayRelease::recovery_notice`] on stderr.
    pub fn release(mut self) -> OverlayRelease {
        self.finish().expect("an unreleased lease still holds its lock")
    }

    /// `None` once released: the lock is taken exactly once.
    fn finish(&mut self) -> Option<OverlayRelease> {
        let lock = self.lock.take()?;
        let io = &*self.io;
        let outcome = self
            .write_back
            .as_ref()
            .map(|write_back| write_back.apply_with(io))
            .unwrap_or_default();
        let Some(write_back) = self.write_back.as_ref().filter(|_| !outcome.failed.is_empty()) else {
            match remove_tree(&self.root) {
                Ok(()) => {
                    let _ = fs::remove_file(&self.lock_path);
                }
                // The lock file stays so a later sweep reclaims the root.
                Err(error) => tracing::debug!(root = %self.root.display(), %error, "overlay root not removed"),
            }
            drop(lock);
            return Some(OverlayRelease::Removed(outcome));
        };

        let marker = retained_marker_for(&self.root);
        let lock_file_gone = match io.rename(&self.lock_path, &marker) {
            Ok(()) => true,
            Err(error) => {
                tracing::error!(root = %self.root.display(), %error, "overlay lock file not renamed to the recovery marker");
                io.remove_file(&self.lock_path).is_ok()
            }
        };
        // Once the lock file is gone the root is protected, and Windows would
        // reject writing the renamed file through another handle while it is
        // locked. Otherwise the lock is held until the marker exists.
        let lock = (!lock_file_gone).then_some(lock);
        let notice = retention_notice(write_back.provider(), &self.root, &outcome);
        let marker = match io.write(&marker, notice.as_bytes()) {
            Ok(()) => Some(marker),
            Err(error) => {
                tracing::error!(root = %self.root.display(), %error, "overlay recovery notice not written");
                None
            }
        };
        drop(lock);
        Some(OverlayRelease::Retained {
            provider: write_back.provider(),
            root: self.root.clone(),
            protected: lock_file_gone || marker.is_some(),
            marker,
            outcome,
        })
    }
}

impl PartialEq for OverlayLease {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl Eq for OverlayLease {}

impl Drop for OverlayLease {
    fn drop(&mut self) {
        // The last plan clone drops after the child exits, so stderr is the
        // only channel left that reaches the user.
        if let Some(notice) = self.finish().as_ref().and_then(OverlayRelease::recovery_notice) {
            tracing::error!(root = %self.root.display(), "provider state kept in the overlay root after a failed write-back");
            eprintln!("{notice}");
        }
    }
}

/// Remove every overlay root under `launches_dir` whose owning launch is gone.
///
/// `launches_dir` holds one directory per provider, each holding launch roots.
/// A root is removed only when its lock file exists and its lock can be taken,
/// which the OS permits only once no live process holds it, and never when a
/// `<root>.retained` marker says the root holds state a write-back could not
/// persist. Every failure keeps the root rather than being returned: a sweep is
/// housekeeping and must never stop a launch.
///
/// ## Returns
///
/// How many roots were removed.
pub fn sweep_abandoned_overlays(launches_dir: &Path) -> usize {
    let Ok(providers) = fs::read_dir(launches_dir) else {
        return 0;
    };
    let mut removed = 0;
    for provider in providers.flatten() {
        if !provider.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let Ok(roots) = fs::read_dir(provider.path()) else {
            continue;
        };
        for root in roots.flatten() {
            if root.file_type().is_ok_and(|kind| kind.is_dir()) && reclaim(&root.path()) {
                removed += 1;
            }
        }
    }
    removed
}

fn reclaim(root: &Path) -> bool {
    let marker = retained_marker_for(root);
    // An unreadable marker counts as present: keeping a root is recoverable.
    if marker.try_exists().unwrap_or(true) {
        return false;
    }
    let lock_path = lock_path_for(root);
    let Ok(lock) = OpenOptions::new().read(true).write(true).open(&lock_path) else {
        return false;
    };
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock | TryLockError::Error(_)) => return false,
    }
    // The owner may have retained the root between the first check and this
    // lock, renaming or removing the lock file this handle still names.
    if !matches!(lock_path.try_exists(), Ok(true)) || marker.try_exists().unwrap_or(true) {
        return false;
    }
    let removed = remove_tree(root).is_ok();
    if removed {
        let _ = fs::remove_file(&lock_path);
    }
    removed
}

fn lock_path_for(root: &Path) -> PathBuf {
    sibling(root, ".lock")
}

fn retained_marker_for(root: &Path) -> PathBuf {
    sibling(root, ".retained")
}

fn sibling(root: &Path, suffix: &str) -> PathBuf {
    let mut name = root.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    root.with_file_name(name)
}

fn retention_notice(provider: Provider, root: &Path, outcome: &WriteBackOutcome) -> String {
    let mut notice = format!(
        "claudine: {provider} state changed during the launch could not be written back, so the overlay root {} was kept.\n",
        root.display()
    );
    for failure in &outcome.failed {
        let _ = writeln!(
            notice,
            "  {}: {}\n    changed copy: {}\n    source:       {}",
            failure.name,
            failure.error,
            failure.overlay.display(),
            failure.source.display()
        );
    }
    let _ = writeln!(
        notice,
        "Copy each changed copy over its source to keep it, then remove {} and its sibling `.retained` marker, if any.",
        root.display()
    );
    notice
}

fn remove_tree(root: &Path) -> io::Result<()> {
    match fs::remove_dir_all(root) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
