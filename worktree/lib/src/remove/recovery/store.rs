//! The recovery store at `<common-git-dir>/wt-removal/`: creating an
//! operation's directory, finding pending ones, and reading one back for
//! recovery. The module docs of [`super`] state the crash guarantees.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::journal::{self, DEFAULT_JOURNAL_LIMIT, JournalEntry, JournalInvalid, JournalWriteError, JournalWriter};
use super::lock::{self, LOCKS_DIR, LockError, OperationLock};
use super::record::{self, Phase, RecordInvalid, RecoveryRecord, RepositoryBinding};
use super::storage::{self, StorageProblem};
use super::OperationId;
use crate::remove::observation::inventory::DeletionInventory;
use crate::remove::outcome::OperationReport;

/// The store's directory name inside the common Git directory.
pub const STORE_DIR: &str = "wt-removal";
pub const RECORD_FILE: &str = "record.json";
pub const JOURNAL_FILE: &str = "journal.jsonl";
pub const REPORT_FILE: &str = "report.json";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{} cannot be used: {problem}", path.display())]
    Storage { path: PathBuf, problem: StorageProblem },
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The record does not describe the worktree the lock covers.
    #[error("the removal lock held does not cover this worktree")]
    WrongLock,
    /// An earlier removal of this worktree has not finished.
    #[error("removal {0} of this worktree has not finished")]
    Pending(OperationId),
    #[error("removal {0} already completed")]
    Completed(OperationId),
    #[error("removal record {id} cannot be used: {reason}")]
    Record { id: String, reason: RecordInvalid },
    #[error("the deletion journal of removal {id} cannot be used: {reason}")]
    Journal { id: OperationId, reason: JournalInvalid },
    /// The inventory was not taken of the checkout the record names.
    #[error("the deletion inventory does not describe the recorded checkout")]
    InventoryMismatch,
    #[error("a phase can only move forward, and completion goes through `complete`")]
    PhaseOrder,
    #[error(transparent)]
    JournalWrite(#[from] JournalWriteError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

fn storage_error(path: &Path) -> impl FnOnce(StorageProblem) -> StoreError + '_ {
    move |problem| StoreError::Storage { path: path.to_path_buf(), problem }
}

/// A saved operation that has not completed, or a record that could not be
/// read and so cannot be ruled out.
#[derive(Debug)]
pub struct PendingRecord {
    pub id: OperationId,
    pub record: Result<RecoveryRecord, StoreError>,
}

/// A pending operation as recovery found it.
#[derive(Debug)]
pub struct Recovered {
    pub record: RecoveryRecord,
    /// What the operation was approved to delete, as it was at the start.
    pub inventory: DeletionInventory,
    pub entries: Vec<JournalEntry>,
    /// The journal's final line was cut short by a crash.
    pub torn_tail: bool,
}

/// Housekeeping that failed after completion was persisted. The completed
/// record still never authorizes another removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookkeepingWarning {
    JournalNotCleared { path: PathBuf, error: String },
}

#[derive(Debug, Clone)]
pub struct RecoveryStore {
    root: PathBuf,
}

impl RecoveryStore {
    /// Opens the store of the repository whose common Git directory is
    /// `common_git_dir`, creating it owner-only when absent.
    pub fn open(common_git_dir: &Path) -> Result<Self, StoreError> {
        let metadata = fs::metadata(common_git_dir).map_err(|error| storage_error(common_git_dir)(error.into()))?;
        if !metadata.is_dir() {
            return Err(storage_error(common_git_dir)(StorageProblem::NotDirectory));
        }
        let root = common_git_dir.join(STORE_DIR);
        storage::ensure_private_dir(&root).map_err(storage_error(&root))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory of operation `id`.
    pub fn operation_dir(&self, id: &OperationId) -> PathBuf {
        self.root.join(id.as_str())
    }

    /// Takes the lock of the worktree record `binding` names, without
    /// waiting; [`LockError::Busy`] when another removal holds it.
    pub fn lock(&self, binding: &RepositoryBinding) -> Result<OperationLock, StoreError> {
        let name = binding.admin_name().ok_or(StoreError::WrongLock)?;
        Ok(lock::acquire(&self.root, name)?)
    }

    /// Makes `record` and the deletion `inventory` durable as a new pending
    /// operation, before anything is mutated. Refuses while another operation
    /// on the same worktree record is pending, and while any record cannot be
    /// read, since an unreadable record could describe this worktree.
    pub fn begin(
        &self,
        lock: OperationLock,
        record: RecoveryRecord,
        inventory: Arc<DeletionInventory>,
    ) -> Result<Operation, StoreError> {
        let id = record.operation_id.clone();
        record.validate().map_err(|reason| StoreError::Record { id: id.to_string(), reason })?;
        if record.completed_phase != Phase::Prepared {
            return Err(StoreError::PhaseOrder);
        }
        if *inventory.root() != record.checkout_identity {
            return Err(StoreError::InventoryMismatch);
        }
        let admin = record.repository.admin_name().ok_or(StoreError::WrongLock)?;
        if lock.key() != lock::key_for(admin) {
            return Err(StoreError::WrongLock);
        }
        for pending in self.pending()? {
            match pending.record {
                Ok(other) if other.repository.admin_name() == Some(admin) => return Err(StoreError::Pending(pending.id)),
                Ok(_) => {}
                Err(error) => return Err(error),
            }
        }

        let staging = self.root.join(format!(".{id}.tmp"));
        let dir = self.operation_dir(&id);
        let staged = (|| -> Result<(), StoreError> {
            storage::create_private_dir(&staging)?;
            let file = storage::create_private_file(&staging.join(JOURNAL_FILE))?;
            // Closed before the rename: Windows refuses to rename a
            // directory with an open file anywhere inside it.
            drop(JournalWriter::start(file, &id, Arc::clone(&inventory), DEFAULT_JOURNAL_LIMIT)?);
            storage::write_durable(&staging.join(RECORD_FILE), &record.to_json())?;
            storage::sync_dir(&staging)?;
            fs::rename(&staging, &dir)?;
            storage::sync_dir(&self.root)?;
            Ok(())
        })();
        if let Err(error) = staged {
            // Only the staging directory this call created; nothing else has
            // been touched.
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        // From here the operation is durable and pending; a failure leaves it
        // for recovery.
        let file = storage::open_append(&dir.join(JOURNAL_FILE))?;
        let journal = JournalWriter::after_header(file, inventory, DEFAULT_JOURNAL_LIMIT);
        Ok(Operation { dir, record, journal, _lock: lock })
    }

    /// Every operation that has not completed. A directory that is not
    /// named as an operation (a staging directory a crash left, the locks)
    /// is not one.
    pub fn pending(&self) -> Result<Vec<PendingRecord>, StoreError> {
        let mut pending = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == LOCKS_DIR {
                continue;
            }
            let Some(id) = name.to_str().and_then(OperationId::parse) else { continue };
            match self.read_record(&id) {
                Ok(record) if record.completed_phase == Phase::Completed => {}
                record => pending.push(PendingRecord { id, record }),
            }
        }
        pending.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(pending)
    }

    /// Reads and validates the record of operation `id`.
    pub fn read_record(&self, id: &OperationId) -> Result<RecoveryRecord, StoreError> {
        let dir = self.operation_dir(id);
        storage::validate_dir(&dir).map_err(storage_error(&dir))?;
        let path = dir.join(RECORD_FILE);
        let bytes = storage::read_private_file(&path).map_err(storage_error(&path))?;
        let invalid = |reason| StoreError::Record { id: id.to_string(), reason };
        let record = record::parse(&bytes).map_err(invalid)?;
        if &record.operation_id != id {
            return Err(invalid(RecordInvalid::OperationMismatch));
        }
        Ok(record)
    }

    /// Takes over pending operation `id` to resume it: its record must
    /// describe the worktree record `expected` names and `lock` must cover
    /// it. The journal so far is kept as `journal-<n>.jsonl`, and a new one
    /// starts with `inventory`, a fresh inventory of what survives.
    pub fn reopen(
        &self,
        lock: OperationLock,
        id: &OperationId,
        expected: &RepositoryBinding,
        inventory: Arc<DeletionInventory>,
    ) -> Result<Operation, StoreError> {
        let record = self.read_record(id)?;
        if record.completed_phase == Phase::Completed {
            return Err(StoreError::Completed(id.clone()));
        }
        record.check_binding(expected).map_err(|reason| StoreError::Record { id: id.to_string(), reason })?;
        let admin = record.repository.admin_name().ok_or(StoreError::WrongLock)?;
        if lock.key() != lock::key_for(admin) {
            return Err(StoreError::WrongLock);
        }
        if *inventory.root() != record.checkout_identity {
            return Err(StoreError::InventoryMismatch);
        }
        let dir = self.operation_dir(id);
        let current = dir.join(JOURNAL_FILE);
        let archived = (1..)
            .map(|generation| dir.join(format!("journal-{generation}.jsonl")))
            .find(|path| matches!(fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound))
            .expect("an unused generation exists");
        // The name was just found unused, and the held lock keeps every other
        // `wt` out of this operation's directory.
        fs::rename(&current, &archived)?;
        let file = storage::create_private_file(&current)?;
        let journal = JournalWriter::start(file, id, inventory, DEFAULT_JOURNAL_LIMIT)?;
        storage::sync_dir(&dir)?;
        Ok(Operation { dir, record, journal, _lock: lock })
    }

    /// Reads pending operation `id` for recovery: its record, which must
    /// describe the repository and worktree record `expected` names, and its
    /// journal replayed against the quarantine. `present` reports whether a
    /// quarantine-relative path still exists, without following links.
    pub fn recover(
        &self,
        id: &OperationId,
        expected: &RepositoryBinding,
        present: impl FnMut(&Path) -> io::Result<bool>,
    ) -> Result<Recovered, StoreError> {
        let record = self.read_record(id)?;
        if record.completed_phase == Phase::Completed {
            return Err(StoreError::Completed(id.clone()));
        }
        record.check_binding(expected).map_err(|reason| StoreError::Record { id: id.to_string(), reason })?;
        let path = self.operation_dir(id).join(JOURNAL_FILE);
        let bytes = storage::read_private_file(&path).map_err(storage_error(&path))?;
        let log = journal::parse(&bytes, id).map_err(|reason| StoreError::Journal { id: id.clone(), reason })?;
        if *log.inventory.root() != record.checkout_identity {
            return Err(StoreError::InventoryMismatch);
        }
        let entries = log.replay(present)?;
        Ok(Recovered { record, inventory: log.inventory, entries, torn_tail: log.torn_tail })
    }
}

/// A pending operation this process created and holds the lock for.
pub struct Operation {
    dir: PathBuf,
    record: RecoveryRecord,
    journal: JournalWriter,
    _lock: OperationLock,
}

impl Operation {
    pub fn id(&self) -> &OperationId {
        &self.record.operation_id
    }

    pub fn record(&self) -> &RecoveryRecord {
        &self.record
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn report_path(&self) -> PathBuf {
        self.dir.join(REPORT_FILE)
    }

    pub fn journal(&mut self) -> &mut JournalWriter {
        &mut self.journal
    }

    /// Persists that `phase` has finished. On failure the saved and the
    /// in-memory record both keep the earlier phase.
    pub fn advance(&mut self, phase: Phase) -> Result<(), StoreError> {
        if phase <= self.record.completed_phase || phase == Phase::Completed {
            return Err(StoreError::PhaseOrder);
        }
        self.persist_phase(phase)
    }

    /// [`Operation::advance`], except that a phase already recorded (by an
    /// earlier attempt this one resumes) is left as it is.
    pub fn advance_to(&mut self, phase: Phase) -> Result<(), StoreError> {
        if phase <= self.record.completed_phase {
            return Ok(());
        }
        self.advance(phase)
    }

    /// The current journal replayed, as [`RecoveryStore::recover`] does;
    /// `present` reports whether a quarantine-relative path still exists.
    pub fn journal_entries(&self, present: impl FnMut(&Path) -> io::Result<bool>) -> Result<Vec<JournalEntry>, StoreError> {
        let path = self.dir.join(JOURNAL_FILE);
        let bytes = storage::read_private_file(&path).map_err(storage_error(&path))?;
        let id = self.id().clone();
        let log = journal::parse(&bytes, &id).map_err(|reason| StoreError::Journal { id, reason })?;
        Ok(log.replay(present)?)
    }

    /// Deletes this operation's directory. Only for an operation that
    /// changed nothing: its record then describes nothing that happened.
    pub fn discard(self) -> Result<(), StoreError> {
        let Self { dir, journal, _lock, .. } = self;
        // Closed first: Windows cannot delete an open file.
        drop(journal);
        fs::remove_dir_all(&dir)?;
        Ok(storage::sync_dir(dir.parent().unwrap_or(&dir))?)
    }

    /// Saves `report` as `report.json`, replacing any earlier one.
    pub fn save_report(&self, report: &OperationReport) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec_pretty(report).map_err(io::Error::from)?;
        Ok(storage::write_durable(&self.report_path(), &bytes)?)
    }

    /// Saves the final `report`, then persists completion, then deletes the
    /// journal. Completion is durable before the journal goes, so a failure
    /// to delete it is only a [`BookkeepingWarning`].
    pub fn complete(mut self, report: &OperationReport) -> Result<Vec<BookkeepingWarning>, StoreError> {
        self.save_report(report)?;
        self.persist_phase(Phase::Completed)?;
        let Self { dir, journal, .. } = self;
        drop(journal);
        let mut journals = vec![dir.join(JOURNAL_FILE)];
        journals.extend((1..).map(|generation| dir.join(format!("journal-{generation}.jsonl"))).take_while(|path| path.exists()));
        Ok(journals
            .into_iter()
            .filter_map(|path| match fs::remove_file(&path) {
                Ok(()) => None,
                Err(error) => Some(BookkeepingWarning::JournalNotCleared { path, error: error.to_string() }),
            })
            .collect())
    }

    fn persist_phase(&mut self, phase: Phase) -> Result<(), StoreError> {
        let mut next = self.record.clone();
        next.completed_phase = phase;
        storage::write_durable(&self.dir.join(RECORD_FILE), &next.to_json())?;
        self.record = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
