//! Portable file primitives: durable writes, per-file atomic replacement with
//! a bounded Windows retry, directory fsync (Unix only), hashing, and the
//! publication lock.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Total transient-conflict retries observed in this process (evidence only).
pub static RETRIES: AtomicU32 = AtomicU32::new(0);

/// Bounded retry for Windows `ERROR_ACCESS_DENIED` (5) / `ERROR_SHARING_VIOLATION` (32)
/// during rename-over. Unix never produces these for a rename-over.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub attempts: u32,
    pub delay: Duration,
}

impl Default for RetryPolicy {
    /// Mirrors `playa::detached::replace_path`: 40 x 25 ms (~1 s).
    fn default() -> Self {
        Self { attempts: 40, delay: Duration::from_millis(25) }
    }
}

pub fn xxh64_hex(bytes: &[u8]) -> String {
    format!("{:016x}", xxhash_rust::xxh64::xxh64(bytes, 0))
}

pub fn is_transient_handle_conflict(error: &io::Error) -> bool {
    cfg!(windows) && matches!(error.raw_os_error(), Some(5) | Some(32))
}

/// Create/truncate `path`, write, and `sync_all`. Used for staging and temps.
pub fn write_durable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Flush a directory entry change. Unix: fsync the directory. Windows: std
/// cannot open a directory handle (needs FILE_FLAG_BACKUP_SEMANTICS), so this
/// is a no-op; see findings for NTFS durability notes.
pub fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// Sibling temp name for replacing `dest`; deterministic per transaction so
/// recovery can find and remove leftovers.
pub fn sibling_temp(dest: &Path, tag: &str) -> std::path::PathBuf {
    let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    dest.with_file_name(format!(".{name}.{tag}.publish-tmp"))
}

/// Rename `from` over `to` (same directory), retrying transient Windows
/// handle conflicts. Never deletes `to` first: the destination always holds
/// complete old or complete new bytes.
pub fn rename_over(from: &Path, to: &Path, retry: RetryPolicy) -> io::Result<()> {
    let mut attempt = 1;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error) if attempt < retry.attempts && is_transient_handle_conflict(&error) => {
                RETRIES.fetch_add(1, Ordering::Relaxed);
                attempt += 1;
                std::thread::sleep(retry.delay);
            }
            Err(error) => return Err(error),
        }
    }
}

/// Atomically replace `dest` with `bytes` via a durable sibling temp.
/// On failure the temp is removed and `dest` is untouched.
pub fn replace_file(dest: &Path, bytes: &[u8], tag: &str, retry: RetryPolicy) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = sibling_temp(dest, tag);
    write_durable(&temp, bytes)?;
    if let Err(error) = rename_over(&temp, dest, retry) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if let Some(parent) = dest.parent() {
        sync_dir(parent);
    }
    Ok(())
}

/// Remove a file if present, retrying transient Windows conflicts.
pub fn remove_if_present(path: &Path, retry: RetryPolicy) -> io::Result<()> {
    let mut attempt = 1;
    loop {
        match fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) if attempt < retry.attempts && is_transient_handle_conflict(&error) => {
                RETRIES.fetch_add(1, Ordering::Relaxed);
                attempt += 1;
                std::thread::sleep(retry.delay);
            }
            Err(error) => return Err(error),
        }
    }
}

pub fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Exclusive, process-scoped publication lock (`File::try_lock`: flock on
/// Unix, LockFileEx on Windows). Released by the OS when the process dies,
/// so a crashed publisher never leaves a stale lock.
pub struct PublicationLock {
    _file: File,
}

impl PublicationLock {
    pub fn try_acquire(path: &Path) -> Result<Self, crate::SpikeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(crate::io_err("create state dir"))?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(crate::io_err("open lock"))?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(fs::TryLockError::WouldBlock) => Err(crate::SpikeError::Locked),
            Err(fs::TryLockError::Error(source)) => Err(crate::SpikeError::Io { context: "lock".into(), source }),
        }
    }
}
