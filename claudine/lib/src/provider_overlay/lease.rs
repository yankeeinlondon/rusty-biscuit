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
//! whose owner has not yet locked it. A lock file is only ever removed together
//! with its root, by a holder of its lock.
//!
//! A lease released in-process first applies its [`WriteBack`]; a swept root
//! never does.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

use super::write_back::WriteBack;

/// A launch's claim on its overlay root. Dropping it applies the attached
/// [`WriteBack`], then removes the root.
///
/// Removal never follows a symbolic link, so the Unix mirror's links into the
/// user's provider root are unlinked, not traversed.
#[derive(Debug)]
pub struct OverlayLease {
    root: PathBuf,
    lock_path: PathBuf,
    lock: Option<File>,
    write_back: Option<WriteBack>,
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
        })
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
}

impl PartialEq for OverlayLease {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl Eq for OverlayLease {}

impl Drop for OverlayLease {
    fn drop(&mut self) {
        if let Some(write_back) = &self.write_back {
            write_back.apply();
        }
        // Best effort: a root that cannot be removed now is reclaimed by a later
        // sweep once this process has released the lock.
        if let Err(error) = remove_tree(&self.root) {
            tracing::debug!(root = %self.root.display(), %error, "overlay root not removed");
        }
        let _ = fs::remove_file(&self.lock_path);
        self.lock.take();
    }
}

/// Remove every overlay root under `launches_dir` whose owning launch is gone.
///
/// `launches_dir` holds one directory per provider, each holding launch roots.
/// A root is removed only when its lock can be taken, which the OS permits only
/// once no live process holds it. Every failure is skipped rather than
/// returned: a sweep is housekeeping and must never stop a launch.
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
    let lock_path = lock_path_for(root);
    // A root without a lock file lost it mid-`Drop`; a live lease always has
    // one before its root exists, so creating it here cannot race a live owner.
    let Ok(lock) = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
    else {
        return false;
    };
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock | TryLockError::Error(_)) => return false,
    }
    let removed = remove_tree(root).is_ok();
    if removed {
        let _ = fs::remove_file(&lock_path);
    }
    removed
}

fn lock_path_for(root: &Path) -> PathBuf {
    let mut name = root.file_name().unwrap_or_default().to_os_string();
    name.push(".lock");
    root.with_file_name(name)
}

fn remove_tree(root: &Path) -> io::Result<()> {
    match fs::remove_dir_all(root) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
