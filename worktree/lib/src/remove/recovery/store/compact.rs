//! The store's side of the compact record: saving an approval, waiting for
//! another removal's lock, resuming an operation (migrating a legacy one
//! first), recording progress, and leaving nothing behind on completion.
//!
//! An operation directory holds `record.json` and nothing else. A journal or
//! report there is a legacy leftover: it is never read to continue deleting
//! and is deleted when the operation is resumed.

use std::fs;
use std::path::{Path, PathBuf};

use super::super::OperationId;
use super::super::compact::{self, COMPACT_RECORD_VERSION, CompactRecord, DeletionMode, Progress, StepState};
use super::super::legacy::{self, LegacyApproval};
use super::super::lock::{self, LOCKS_DIR, LockError, OperationLock};
use super::super::record::{RECORD_VERSION, RecordInvalid, RepositoryBinding};
use super::super::storage;
use super::{RECORD_FILE, REPORT_FILE, RecoveryStore, StoreError, storage_error};
use crate::remove::clock::{Backoff, Clock, Slept};

/// A stored operation, by format.
#[derive(Debug, Clone, PartialEq)]
pub enum Stored {
    Compact(Box<CompactRecord>),
    /// Saved by an earlier `wt`; [`RecoveryStore::resume`] migrates it.
    Legacy(Box<LegacyApproval>),
}

impl Stored {
    pub fn repository(&self) -> &RepositoryBinding {
        match self {
            Self::Compact(record) => &record.repository,
            Self::Legacy(legacy) => &legacy.record.repository,
        }
    }
}

/// An operation directory and what its record turned out to be. An
/// unreadable record cannot be ruled out as this worktree's.
#[derive(Debug)]
pub struct StoredOperation {
    pub id: OperationId,
    pub record: Result<Stored, StoreError>,
}

impl RecoveryStore {
    /// Every operation directory, of either format, by id. A name that is no
    /// operation id (the locks, a hidden staging or discarded directory) is
    /// skipped.
    pub fn operations(&self) -> Result<Vec<StoredOperation>, StoreError> {
        let mut operations = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let name = entry?.file_name();
            if name == LOCKS_DIR {
                continue;
            }
            let Some(id) = name.to_str().and_then(OperationId::parse) else { continue };
            let record = self.read_stored(&id);
            operations.push(StoredOperation { id, record });
        }
        operations.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(operations)
    }

    /// Reads operation `id`'s record in whichever format it was saved, with
    /// a legacy operation's optional report.
    pub fn read_stored(&self, id: &OperationId) -> Result<Stored, StoreError> {
        let dir = self.operation_dir(id);
        storage::validate_dir(&dir).map_err(storage_error(&dir))?;
        let path = dir.join(RECORD_FILE);
        let bytes = storage::read_private_file(&path).map_err(storage_error(&path))?;
        let invalid = |reason| StoreError::Record { id: id.to_string(), reason };
        let version = compact::record_version(&bytes).map_err(invalid)?;
        let stored = if version == u64::from(COMPACT_RECORD_VERSION) {
            Stored::Compact(Box::new(compact::parse(&bytes).map_err(invalid)?))
        } else if version == u64::from(RECORD_VERSION) {
            let report_path = dir.join(REPORT_FILE);
            // An unreadable report is the same as none: it grants nothing.
            let report = storage::read_private_file(&report_path).ok();
            Stored::Legacy(Box::new(legacy::read(&bytes, report.as_deref()).map_err(invalid)?))
        } else {
            return Err(invalid(RecordInvalid::UnsupportedVersion(version)));
        };
        let recorded = match &stored {
            Stored::Compact(record) => &record.operation_id,
            Stored::Legacy(legacy) => &legacy.record.operation_id,
        };
        if recorded != id {
            return Err(invalid(RecordInvalid::OperationMismatch));
        }
        Ok(stored)
    }

    /// Takes the lock of the worktree record `binding` names, waiting while
    /// another removal holds it. Waits back off on `clock` and end with
    /// [`StoreError::Cancelled`] once `cancelled` holds. Whoever held the
    /// lock may have finished or exited: the caller reads the stored state
    /// once it has the lock, and never infers an owner from a process id.
    pub fn lock_waiting(
        &self,
        binding: &RepositoryBinding,
        clock: &dyn Clock,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<OperationLock, StoreError> {
        let mut backoff = Backoff::new();
        loop {
            match self.lock(binding) {
                Err(StoreError::Lock(LockError::Busy)) => {}
                taken => return taken,
            }
            if backoff.wait(clock, cancelled) == Slept::Cancelled {
                return Err(StoreError::Cancelled);
            }
        }
    }

    /// Saves `record`, the user's approval, as a new operation before
    /// anything is unlocked, moved, or deleted. Refuses while another
    /// operation on the same worktree record is stored, and while any record
    /// cannot be read, since it could describe this worktree.
    ///
    /// The operation directory appears all at once (built under a hidden
    /// name, flushed, renamed). When it cannot be saved the result is
    /// [`StoreError::NotSaved`] and no operation exists.
    pub fn approve(&self, lock: OperationLock, record: CompactRecord) -> Result<PendingRemoval, StoreError> {
        let id = record.operation_id.clone();
        record.validate().map_err(|reason| StoreError::Record { id: id.to_string(), reason })?;
        if record.progress != Progress::Approved {
            return Err(StoreError::PhaseOrder);
        }
        let admin = record.repository.admin_name().ok_or(StoreError::WrongLock)?;
        if lock.key() != lock::key_for(admin) {
            return Err(StoreError::WrongLock);
        }
        for stored in self.operations()? {
            match stored.record {
                Ok(other) if other.repository().admin_name() == Some(admin) => return Err(StoreError::Pending(stored.id)),
                Ok(_) => {}
                Err(error) => return Err(error),
            }
        }
        let staging = self.root.join(format!(".{id}.tmp"));
        let dir = self.operation_dir(&id);
        let saved = (|| -> std::io::Result<()> {
            storage::create_private_dir(&staging)?;
            storage::write_durable(&staging.join(RECORD_FILE), &record.to_json())?;
            storage::sync_dir(&staging)?;
            fault::at(SaveStep::Staged)?;
            fs::rename(&staging, &dir)?;
            storage::sync_dir(&self.root)
        })();
        if let Err(source) = saved {
            if !fault::crashed(&source) {
                // Only the staging directory this call created.
                let _ = fs::remove_dir_all(&staging);
            }
            return Err(StoreError::NotSaved(source));
        }
        Ok(PendingRemoval { dir, record, _lock: lock })
    }

    /// Takes over stored operation `id` to finish it. Its record must
    /// describe the worktree record `expected` names, and `lock` must cover
    /// it.
    ///
    /// A legacy operation is migrated first: its compact record is validated
    /// and published over `record.json` (one atomic replacement), and only
    /// then are its journals and report deleted. A crash before the
    /// replacement leaves the legacy files intact for the next attempt; one
    /// after it leaves leftovers that the next attempt deletes. A completed
    /// legacy operation is deleted and reported as [`StoreError::Completed`].
    pub fn resume(&self, lock: OperationLock, id: &OperationId, expected: &RepositoryBinding) -> Result<PendingRemoval, StoreError> {
        let invalid = |reason| StoreError::Record { id: id.to_string(), reason };
        let stored = self.read_stored(id)?;
        let admin = stored.repository().admin_name().ok_or(StoreError::WrongLock)?;
        if lock.key() != lock::key_for(admin) {
            return Err(StoreError::WrongLock);
        }
        let dir = self.operation_dir(id);
        let record = match stored {
            Stored::Compact(record) => {
                record.check_binding(expected).map_err(invalid)?;
                *record
            }
            Stored::Legacy(legacy) => {
                legacy.record.check_binding(expected).map_err(invalid)?;
                if legacy.is_completed() {
                    remove_operation_dir(&dir)?;
                    return Err(StoreError::Completed(id.clone()));
                }
                let record = legacy.to_compact();
                let json = record.to_json();
                // What is published must read back as itself.
                if compact::parse(&json).as_ref() != Ok(&record) {
                    return Err(invalid(RecordInvalid::Malformed("the migrated record does not read back".into())));
                }
                fault::at(SaveStep::Migrated)?;
                storage::write_durable(&dir.join(RECORD_FILE), &json)?;
                fault::at(SaveStep::MigrationPublished)?;
                record
            }
        };
        remove_leftovers(&dir);
        Ok(PendingRemoval { dir, record, _lock: lock })
    }
}

/// A stored operation this process holds the lock for, from approval or
/// resumption until [`PendingRemoval::finish`]. Dropping it releases the
/// lock and keeps the record for the next `wt remove`.
#[derive(Debug)]
pub struct PendingRemoval {
    dir: PathBuf,
    record: CompactRecord,
    _lock: OperationLock,
}

impl PendingRemoval {
    pub fn id(&self) -> &OperationId {
        &self.record.operation_id
    }

    pub fn record(&self) -> &CompactRecord {
        &self.record
    }

    /// Persists that `progress` was reached. A step already recorded (by an
    /// attempt this one resumes) is left as it is. On failure the saved and
    /// the in-memory record both keep the earlier progress.
    pub fn advance(&mut self, progress: Progress) -> Result<(), StoreError> {
        if progress <= self.record.progress {
            return Ok(());
        }
        let mut next = self.record.clone();
        next.progress = progress;
        self.persist(next)
    }

    /// Persists that the checkout is deleted where it stands instead of
    /// being moved: its deletion path becomes the original. Only before the
    /// checkout is placed, and a no-op when already so.
    pub fn mark_in_place(&mut self) -> Result<(), StoreError> {
        if self.record.deletion_mode == DeletionMode::InPlace {
            return Ok(());
        }
        if self.record.progress >= Progress::Placed {
            return Err(StoreError::PhaseOrder);
        }
        let mut next = self.record.clone();
        next.deletion_mode = DeletionMode::InPlace;
        next.deletion_path = next.original.clone();
        self.persist(next)
    }

    /// Persists how branch step `index` ended. Steps run only once the
    /// registration is gone, and a finished step never changes again.
    pub fn finish_step(&mut self, index: usize, state: StepState) -> Result<(), StoreError> {
        let current = self.record.branch_steps.get(index).map(|step| step.state);
        if self.record.progress < Progress::Unregistered || current != Some(StepState::Pending) || state == StepState::Pending {
            return Err(StoreError::PhaseOrder);
        }
        let mut next = self.record.clone();
        next.branch_steps[index].state = state;
        self.persist(next)
    }

    /// Deletes the operation, record and all, so nothing is left. Used when
    /// every step has finished, and for an operation that changed nothing.
    pub fn finish(self) -> Result<(), StoreError> {
        let Self { dir, _lock, .. } = self;
        remove_operation_dir(&dir)
    }

    /// Ends this process's hold on the operation as a crash would: the
    /// record stays as saved and the lock is handed back.
    #[cfg(test)]
    pub(crate) fn into_lock(self) -> OperationLock {
        self._lock
    }

    fn persist(&mut self, next: CompactRecord) -> Result<(), StoreError> {
        storage::write_durable(&self.dir.join(RECORD_FILE), &next.to_json())?;
        self.record = next;
        Ok(())
    }
}

/// Deletes operation directory `dir`. It is first renamed to a hidden name
/// that is no operation, so a crash part-way never leaves a partial record
/// behind; a hidden leftover is never read.
fn remove_operation_dir(dir: &Path) -> Result<(), StoreError> {
    let root = dir.parent().unwrap_or(dir);
    let name = dir.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let doomed = root.join(format!(".{name}.discarded"));
    fs::rename(dir, &doomed)?;
    storage::sync_dir(root)?;
    fs::remove_dir_all(&doomed)?;
    Ok(storage::sync_dir(root)?)
}

/// Deletes everything in `dir` except the record: legacy journals and
/// reports, and hidden temporary files. Best effort, since none is read
/// again and [`PendingRemoval::finish`] deletes the directory anyway.
fn remove_leftovers(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        // A link is removed as itself; a directory is no leftover of ours.
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if entry.file_name() != RECORD_FILE && !is_dir {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// The points in [`RecoveryStore::approve`] and [`RecoveryStore::resume`]
/// where a test can stop them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveStep {
    /// The approval is flushed under its hidden staging name, not yet
    /// renamed into place.
    Staged,
    /// A legacy operation's compact record is built and validated, not yet
    /// published.
    Migrated,
    /// The migrated record is published; the legacy leftovers are not yet
    /// deleted.
    MigrationPublished,
}

#[cfg(not(test))]
mod fault {
    use std::io;

    use super::SaveStep;

    pub(super) fn at(_step: SaveStep) -> io::Result<()> {
        Ok(())
    }

    pub(super) fn crashed(_error: &io::Error) -> bool {
        false
    }
}

/// Test-only fault injection for the compact store.
#[cfg(test)]
pub(crate) mod fault {
    use std::cell::RefCell;
    use std::io;

    use super::SaveStep;
    pub(crate) use crate::remove::recovery::store::fault::Fault;

    type Injector = Box<dyn FnMut(SaveStep) -> Option<Fault>>;

    thread_local! {
        static INJECTOR: RefCell<Option<Injector>> = const { RefCell::new(None) };
    }

    const CRASHED: &str = "injected crash";

    /// Removes the injector when dropped.
    pub(crate) struct Injected;

    impl Drop for Injected {
        fn drop(&mut self) {
            INJECTOR.with(|slot| *slot.borrow_mut() = None);
        }
    }

    /// Calls `injector` at each [`SaveStep`] on this thread until the guard
    /// drops.
    #[must_use]
    pub(crate) fn inject(injector: impl FnMut(SaveStep) -> Option<Fault> + 'static) -> Injected {
        INJECTOR.with(|slot| *slot.borrow_mut() = Some(Box::new(injector)));
        Injected
    }

    pub(super) fn at(step: SaveStep) -> io::Result<()> {
        match INJECTOR.with(|slot| slot.borrow_mut().as_mut().and_then(|injector| injector(step))) {
            None => Ok(()),
            Some(Fault::Crash) => Err(io::Error::other(CRASHED)),
            Some(Fault::Fail) => Err(io::Error::other("injected write failure")),
        }
    }

    pub(super) fn crashed(error: &io::Error) -> bool {
        error.to_string() == CRASHED
    }
}

#[cfg(test)]
mod tests;
