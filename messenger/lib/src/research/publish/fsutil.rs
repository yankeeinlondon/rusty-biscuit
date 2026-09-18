//! Portable file primitives for publication: durable writes, per-file atomic
//! replacement with a bounded Windows retry, Unix directory fsync, and the
//! process-scoped publication lock.
//!
//! Replacement uses `std::fs::rename`, not `tempfile::persist`: on Windows
//! std renames with POSIX semantics, so a reader holding a std-default handle
//! does not block it, while `persist` fails with error 5 against the same
//! holder (measured in the Phase 1 publication spike).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Bounded retry for Windows `ERROR_ACCESS_DENIED` (5) and
/// `ERROR_SHARING_VIOLATION` (32) while a scanner or reader briefly holds a
/// destination without delete sharing. Unix never produces these.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub attempts: u32,
    pub delay: Duration,
}

impl Default for RetryPolicy {
    /// 40 × 25 ms, matching `playa`'s detached replacement.
    fn default() -> Self {
        Self {
            attempts: 40,
            delay: Duration::from_millis(25),
        }
    }
}

fn is_transient_handle_conflict(error: &io::Error) -> bool {
    cfg!(windows) && matches!(error.raw_os_error(), Some(5) | Some(32))
}

/// Creates or truncates `path`, writes `bytes`, and `sync_all`s.
pub(super) fn write_durable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Flushes directory entries on Unix. std cannot open a directory handle on
/// Windows; NTFS journals rename metadata, and verification catches a lost
/// rename after power loss.
pub(super) fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// The sibling temp for replacing `dest`. The `.publish-tmp` suffix lets
/// recovery find leftovers; renames never cross a volume.
pub(super) fn sibling_temp(dest: &Path, tag: &str) -> PathBuf {
    let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    dest.with_file_name(format!(".{name}.{tag}.publish-tmp"))
}

fn with_retry(retry: RetryPolicy, mut operation: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    let mut attempt = 1;
    loop {
        match operation() {
            Err(error) if attempt < retry.attempts && is_transient_handle_conflict(&error) => {
                attempt += 1;
                std::thread::sleep(retry.delay);
            }
            other => return other,
        }
    }
}

/// Atomically replaces `dest` with `bytes` through a durable sibling temp.
/// Never deletes `dest` first, so it always holds complete old or new bytes.
pub(super) fn replace_file(dest: &Path, bytes: &[u8], tag: &str, retry: RetryPolicy) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = sibling_temp(dest, tag);
    write_durable(&temp, bytes)?;
    if let Err(error) = with_retry(retry, || fs::rename(&temp, dest)) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if let Some(parent) = dest.parent() {
        sync_dir(parent);
    }
    Ok(())
}

/// Removes a file if present.
pub(super) fn remove_if_present(path: &Path, retry: RetryPolicy) -> io::Result<()> {
    with_retry(retry, || match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    })
}

/// Reads a whole file and closes it, or `None` when it does not exist.
/// Readers must not hold artifact handles across a publication.
pub(super) fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Exclusive, process-scoped lock (`File::try_lock`: `flock` on Unix,
/// `LockFileEx` on Windows). The OS releases it when the process dies, so a
/// crashed publisher never leaves a stale lock.
pub(super) struct PublicationLock {
    _file: File,
}

pub(super) enum LockError {
    Held,
    Io(io::Error),
}

impl PublicationLock {
    pub(super) fn try_acquire(path: &Path) -> Result<Self, LockError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(LockError::Io)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(LockError::Io)?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(fs::TryLockError::WouldBlock) => Err(LockError::Held),
            Err(fs::TryLockError::Error(error)) => Err(LockError::Io(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_file_leaves_no_temp_and_overwrites_in_place() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("nested/catalog.json");
        replace_file(&dest, b"old", "t1", RetryPolicy::default()).expect("first");
        replace_file(&dest, b"new", "t2", RetryPolicy::default()).expect("second");
        assert_eq!(fs::read(&dest).expect("read"), b"new");
        let leftovers: Vec<_> = fs::read_dir(dest.parent().expect("parent"))
            .expect("list")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".publish-tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn a_second_lock_holder_is_refused_until_the_first_drops() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("state/lock");
        let first = PublicationLock::try_acquire(&path).ok().expect("first lock");
        assert!(matches!(PublicationLock::try_acquire(&path), Err(LockError::Held)));
        drop(first);
        assert!(PublicationLock::try_acquire(&path).is_ok());
    }
}
