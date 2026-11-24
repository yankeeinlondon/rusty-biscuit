//! The publisher lock under `~/.claudine/registrations/`.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::time::{Duration, Instant};

use super::{LOCK_BACKOFF, PublishError, access_log, io_error};

/// A held publisher lock, released when dropped. The lock file stays.
pub(super) struct PublisherLock {
    _file: File,
}

pub(super) fn acquire_lock(lock: &Path, deadline: Duration) -> Result<PublisherLock, PublishError> {
    access_log::note("lock", lock);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock)
        .map_err(io_error("open the lock", lock))?;
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(PublisherLock { _file: file }),
            Err(TryLockError::WouldBlock) => {
                let waited = started.elapsed();
                if waited >= deadline {
                    return Err(PublishError::LockTimeout { lock: lock.to_path_buf(), waited });
                }
                std::thread::sleep(LOCK_BACKOFF.min(deadline - waited));
            }
            Err(TryLockError::Error(source)) => {
                return Err(PublishError::Io { action: "lock", path: lock.to_path_buf(), source });
            }
        }
    }
}
