//! The liveness lock beside each store (`<store>.lock`).
//!
//! Deleting a store is a liveness test on Windows only: there every file of an
//! open store refuses deletion (os error 32), while Unix unlinks it and the
//! holder keeps writing to an orphaned inode. So every holder of a store takes
//! a **shared** OS advisory lock on the lock file for the connection's
//! lifetime, and anything that deletes a store (the sweep, corruption repair,
//! a root-mismatch refill) first takes an **exclusive non-blocking** lock and
//! skips the store when it is held.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long [`StoreLock::shared`] waits out a brief exclusive holder (a sweep
/// or repair deleting files) before reporting the store busy.
pub const SHARED_LOCK_WAIT: Duration = Duration::from_secs(2);

const RETRY_SLEEP: Duration = Duration::from_millis(5);

/// A held lock on a store's lock file; released on drop.
#[derive(Debug)]
pub struct StoreLock {
    file: File,
    path: PathBuf,
}

impl StoreLock {
    /// Takes the shared lock on `path`, creating the file (`0600` on Unix).
    ///
    /// ## Errors
    ///
    /// [`io::ErrorKind::WouldBlock`] when an exclusive holder kept the lock for
    /// all of `wait`; any error opening or locking the file.
    pub fn shared(path: &Path, wait: Duration) -> io::Result<Self> {
        let deadline = Instant::now() + wait;
        loop {
            let file = open_lock_file(path)?;
            match file.try_lock_shared() {
                // The holder that blocked us may have deleted the file we
                // opened; a lock on that orphan protects nothing.
                Ok(()) if is_same_file(&file, path) => return Ok(Self { file, path: path.to_path_buf() }),
                Ok(()) => {}
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(error)) => return Err(error),
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "store lock held exclusively"));
            }
            std::thread::sleep(RETRY_SLEEP);
        }
    }

    /// Takes the exclusive lock on `path` without waiting, creating the file.
    ///
    /// ## Returns
    ///
    /// `None` when any other holder, shared or exclusive, has the lock.
    pub fn try_exclusive(path: &Path) -> io::Result<Option<Self>> {
        let file = open_lock_file(path)?;
        match file.try_lock() {
            Ok(()) if is_same_file(&file, path) => Ok(Some(Self { file, path: path.to_path_buf() })),
            Ok(()) | Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(error)) => Err(error),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        // Closing the handle releases the lock too; unlocking first makes the
        // release independent of when the handle is closed.
        let _ = self.file.unlock();
    }
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Whether `file` is still the file at `path`.
#[cfg(unix)]
fn is_same_file(file: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (file.metadata(), std::fs::metadata(path)) {
        (Ok(held), Ok(current)) => held.ino() == current.ino() && held.dev() == current.dev(),
        _ => false,
    }
}

/// Whether `file` is still the file at `path`. Windows cannot replace a file
/// another handle holds open, so the path still existing is enough.
#[cfg(not(unix))]
fn is_same_file(_file: &File, path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_holders_coexist_and_block_an_exclusive_taker() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.sqlite.lock");
        let first = StoreLock::shared(&path, Duration::ZERO).unwrap();
        let second = StoreLock::shared(&path, Duration::ZERO).unwrap();
        assert!(StoreLock::try_exclusive(&path).unwrap().is_none());
        drop((first, second));
        assert!(StoreLock::try_exclusive(&path).unwrap().is_some());
    }

    #[test]
    fn an_exclusive_holder_makes_a_shared_taker_report_would_block() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.sqlite.lock");
        let _exclusive = StoreLock::try_exclusive(&path).unwrap().unwrap();
        let error = StoreLock::shared(&path, Duration::from_millis(20)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    }

    #[cfg(unix)]
    #[test]
    fn a_lock_file_deleted_by_a_sweep_is_recreated_on_the_next_take() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.sqlite.lock");
        let exclusive = StoreLock::try_exclusive(&path).unwrap().unwrap();
        // A sweep deletes the lock file while holding it, then lets go.
        std::fs::remove_file(&path).unwrap();
        drop(exclusive);
        let shared = StoreLock::shared(&path, Duration::from_millis(200)).unwrap();
        assert!(is_same_file(&shared.file, &path));
    }

    #[cfg(unix)]
    #[test]
    fn the_lock_file_is_private_to_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.sqlite.lock");
        let _lock = StoreLock::shared(&path, Duration::ZERO).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
