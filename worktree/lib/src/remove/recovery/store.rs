//! The recovery store at `<common-git-dir>/wt-removal/`: creating an
//! operation's directory, finding pending ones, and reading one back for
//! recovery. The module docs of [`super`] state the crash guarantees.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::journal::{
    self, EntryState, JournalEntry, JournalInvalid, JournalLog, JournalSizeOverflow, JournalWriteError, JournalWriter,
};
use super::lock::{self, LOCKS_DIR, LockError, OperationLock};
use super::record::{self, Phase, RecordInvalid, RecoveryRecord, RepositoryBinding};
use super::space::SpaceSource;
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
    JournalSize(#[from] JournalSizeOverflow),
    /// The new attempt's journal does not fit in the space available; nothing
    /// of the attempt was written.
    #[error(
        "{} has {available} bytes available, but this removal's deletion journal needs {needed}",
        path.display()
    )]
    InsufficientSpace { path: PathBuf, needed: u64, available: u64 },
    /// The space available could not be read, so the journal is not known to
    /// fit; nothing of the attempt was written.
    #[error("the space available at {} could not be read, so this removal's deletion journal ({needed} bytes) is not known to fit: {reason}", path.display())]
    SpaceUnknown { path: PathBuf, needed: u64, reason: String },
    /// The approval could not be saved; nothing was unlocked, moved, or
    /// deleted.
    #[error("the removal could not be saved, so nothing was deleted: {0}")]
    NotSaved(#[source] io::Error),
    /// A wait for another removal's lock was cancelled.
    #[error("the wait for another wt remove of this worktree was cancelled")]
    Cancelled,
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
    /// What the current attempt was approved to delete, as it was when that
    /// attempt started.
    pub inventory: DeletionInventory,
    /// Every attempt's entries; see [`Operation::journal_entries`].
    pub entries: Vec<JournalEntry>,
    /// The current journal's final line was cut short by a crash.
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
    ///
    /// The journal's batch limit is computed from `inventory`
    /// ([`journal::journal_size`]). Before anything of the operation is
    /// written, `space` must show room for the journal's header, inventory,
    /// and every batch, plus the record; otherwise this refuses with
    /// [`StoreError::InsufficientSpace`] or [`StoreError::SpaceUnknown`] and
    /// no operation directory exists. The reading reserves nothing: a later
    /// write can still fail and is handled as any journal write failure.
    pub fn begin(
        &self,
        lock: OperationLock,
        record: RecoveryRecord,
        inventory: Arc<DeletionInventory>,
        space: &dyn SpaceSource,
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

        let size = journal::journal_size(&id, &inventory)?;
        let record_json = record.to_json();
        let needed = size.total().and_then(|journal| journal.checked_add(record_json.len() as u64)).ok_or(JournalSizeOverflow)?;
        ensure_space(space, &self.root, needed)?;

        let staging = self.root.join(format!(".{id}.tmp"));
        let dir = self.operation_dir(&id);
        let staged = (|| -> Result<(), StoreError> {
            storage::create_private_dir(&staging)?;
            let file = storage::create_private_file(&staging.join(JOURNAL_FILE))?;
            // Closed before the rename: Windows refuses to rename a
            // directory with an open file anywhere inside it.
            drop(JournalWriter::start(file, &id, Arc::clone(&inventory), size.batches)?);
            storage::write_durable(&staging.join(RECORD_FILE), &record_json)?;
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
        let journal = JournalWriter::after_header(file, inventory, size.batches);
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
    ///
    /// `journal.jsonl` stays the previous attempt's journal until the new one
    /// is complete and flushed under a hidden name, and is then replaced by a
    /// rename. A crash or failure at any point before that leaves the
    /// previous attempt recoverable and authorizes nothing new.
    ///
    /// As in [`RecoveryStore::begin`], the new journal's limit is computed
    /// from `inventory`, and before anything is archived or created `space`
    /// must show room for the new journal plus, when the previous journal
    /// needs a new archive copy, its full length (an identical newest archive
    /// costs nothing). A refusal leaves the previous journal and every
    /// archive as they were.
    pub fn reopen(
        &self,
        lock: OperationLock,
        id: &OperationId,
        expected: &RepositoryBinding,
        inventory: Arc<DeletionInventory>,
        space: &dyn SpaceSource,
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
        let previous = storage::read_private_file(&current).map_err(storage_error(&current))?;
        // The held lock keeps every other `wt` out of this operation's
        // directory.
        let archive = archive_target(&dir, &previous);
        let size = journal::journal_size(id, &inventory)?;
        let archived_bytes = if archive.is_some() { previous.len() as u64 } else { 0 };
        let needed = size.total().and_then(|journal| journal.checked_add(archived_bytes)).ok_or(JournalSizeOverflow)?;
        ensure_space(space, &dir, needed)?;
        if let Some(archive) = archive {
            storage::write_durable(&archive, &previous)?;
        }
        fault::at(ReopenStep::Archived, &current)?;
        let next = storage::temp_beside(&current)?;
        let staged = (|| -> Result<(), StoreError> {
            let file = storage::create_private_file(&next)?;
            fault::at(ReopenStep::Created, &next)?;
            // Closed before the rename, as in `begin`.
            drop(JournalWriter::start(file, id, Arc::clone(&inventory), size.batches)?);
            fault::at(ReopenStep::Initialized, &next)?;
            Ok(storage::publish(&next, &current)?)
        })();
        if let Err(error) = staged {
            if !fault::crashed(&error) {
                let _ = fs::remove_file(&next);
            }
            return Err(error);
        }
        let file = storage::open_append(&current)?;
        let journal = JournalWriter::after_header(file, inventory, size.batches);
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
        let (log, entries) = replay_generations(&self.operation_dir(id), id, present)?;
        if *log.inventory.root() != record.checkout_identity {
            return Err(StoreError::InventoryMismatch);
        }
        Ok(Recovered { record, inventory: log.inventory, entries, torn_tail: log.torn_tail })
    }

    /// The report the last attempt of operation `id` saved, or `None` when no
    /// attempt saved one.
    pub fn read_report(&self, id: &OperationId) -> Result<Option<OperationReport>, StoreError> {
        let path = self.operation_dir(id).join(REPORT_FILE);
        let bytes = match storage::read_private_file(&path) {
            Ok(bytes) => bytes,
            Err(StorageProblem::Missing) => return Ok(None),
            Err(problem) => return Err(storage_error(&path)(problem)),
        };
        let report = serde_json::from_slice(&bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(Some(report))
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

    /// Every attempt's journal replayed, oldest first, as
    /// [`RecoveryStore::recover`] does. `present` reports whether a
    /// quarantine-relative path still exists, and is asked only about the
    /// current attempt's unconfirmed entries.
    ///
    /// An earlier attempt contributes only the entries that were gone when
    /// the next attempt began (deleted, already absent, or uncertain): one
    /// still present then is in the next attempt's inventory and is that
    /// attempt's to resolve. Whether an earlier unconfirmed entry was present
    /// is read from that next inventory, never from the filesystem now, so an
    /// entry is never counted twice and an uncertain one never becomes
    /// confirmed.
    pub fn journal_entries(&self, present: impl FnMut(&Path) -> io::Result<bool>) -> Result<Vec<JournalEntry>, StoreError> {
        Ok(replay_generations(&self.dir, self.id(), present)?.1)
    }

    /// Deletes this operation's directory. Only for an operation that
    /// changed nothing: its record then describes nothing that happened.
    ///
    /// The directory is first renamed to a hidden name that is no operation,
    /// so a crash part-way through deleting it never leaves a pending record
    /// without its journal.
    pub fn discard(self) -> Result<(), StoreError> {
        let Self { dir, journal, record, _lock } = self;
        // Closed first: Windows can neither rename a directory with an open
        // file inside nor delete an open file.
        drop(journal);
        let root = dir.parent().unwrap_or(&dir);
        let doomed = root.join(format!(".{}.discarded", record.operation_id));
        fs::rename(&dir, &doomed)?;
        storage::sync_dir(root)?;
        fs::remove_dir_all(&doomed)?;
        Ok(storage::sync_dir(root)?)
    }

    /// Saves `report` as `report.json`, replacing any earlier one.
    pub fn save_report(&self, report: &OperationReport) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec_pretty(report).map_err(io::Error::from)?;
        Ok(storage::write_durable(&self.report_path(), &bytes)?)
    }

    /// Saves the final `report`, then persists completion, then deletes every
    /// journal when `journal_reported` says `report` lists all of their
    /// entries ([`OperationReport::journal_entries`]); otherwise they are
    /// kept as the only full record. Completion is durable before a journal
    /// goes, so a failure to delete one is only a [`BookkeepingWarning`].
    pub fn complete(mut self, report: &OperationReport, journal_reported: bool) -> Result<Vec<BookkeepingWarning>, StoreError> {
        self.save_report(report)?;
        self.persist_phase(Phase::Completed)?;
        let Self { dir, journal, .. } = self;
        drop(journal);
        if !journal_reported {
            return Ok(Vec::new());
        }
        let mut journals = vec![dir.join(JOURNAL_FILE)];
        journals.extend((1..).map(|generation| archive_path(&dir, generation)).take_while(|path| path.exists()));
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

/// Where `journal` (the bytes of `dir`'s current journal) is kept: the next
/// unused `journal-<n>.jsonl`, or `None` when the newest archive already
/// holds exactly these bytes (a retry whose `reopen` stopped after archiving
/// the same journal). Writes nothing.
fn archive_target(dir: &Path, journal: &[u8]) -> Option<PathBuf> {
    let generation = |n: u32| archive_path(dir, n);
    let unused = (1..)
        .find(|&n| matches!(fs::symlink_metadata(generation(n)), Err(error) if error.kind() == io::ErrorKind::NotFound))
        .expect("an unused generation exists");
    let identical = unused > 1 && storage::read_private_file(&generation(unused - 1)).is_ok_and(|newest| newest == journal);
    (!identical).then(|| generation(unused))
}

/// Refuses unless `space` reads at least `needed` bytes available at `path`.
fn ensure_space(space: &dyn SpaceSource, path: &Path, needed: u64) -> Result<(), StoreError> {
    match space.available_bytes(path) {
        Ok(available) if available >= needed => Ok(()),
        Ok(available) => Err(StoreError::InsufficientSpace { path: path.to_path_buf(), needed, available }),
        Err(reason) => Err(StoreError::SpaceUnknown { path: path.to_path_buf(), needed, reason }),
    }
}

/// The `n`th archived journal of the operation in `dir`.
fn archive_path(dir: &Path, n: u32) -> PathBuf {
    dir.join(format!("journal-{n}.jsonl"))
}

/// The current journal of operation `id` in `dir`, parsed, and every
/// attempt's entries; see [`Operation::journal_entries`].
fn replay_generations(
    dir: &Path,
    id: &OperationId,
    present: impl FnMut(&Path) -> io::Result<bool>,
) -> Result<(JournalLog, Vec<JournalEntry>), StoreError> {
    let read = |path: &Path| storage::read_private_file(path).map_err(storage_error(path));
    let parse = |bytes: &[u8]| journal::parse(bytes, id).map_err(|reason| StoreError::Journal { id: id.clone(), reason });
    let current_bytes = read(&dir.join(JOURNAL_FILE))?;
    let current = parse(&current_bytes)?;
    let mut archives = Vec::new();
    for n in 1.. {
        let path = archive_path(dir, n);
        match storage::read_private_file(&path) {
            Ok(bytes) => archives.push(bytes),
            Err(StorageProblem::Missing) => break,
            Err(problem) => return Err(storage_error(&path)(problem)),
        }
    }
    // A retry stopped after archiving but before publishing its journal
    // leaves the newest archive identical to the current journal: one
    // attempt, not two.
    if archives.last() == Some(&current_bytes) {
        archives.pop();
    }
    let earlier = archives.iter().map(|bytes| parse(bytes)).collect::<Result<Vec<_>, _>>()?;
    let mut entries = Vec::new();
    for (index, log) in earlier.iter().enumerate() {
        let next = earlier.get(index + 1).map_or(&current.inventory, |next| &next.inventory);
        let attempt = index as u32 + 1;
        entries.extend(
            log.replay(|path| Ok(next.get(path).is_some()))?
                .into_iter()
                .filter(|entry| matches!(entry.state, EntryState::Deleted | EntryState::ExternallyAbsent | EntryState::Uncertain))
                .map(|entry| JournalEntry { attempt, ..entry }),
        );
    }
    let attempt = earlier.len() as u32 + 1;
    entries.extend(current.replay(present)?.into_iter().map(|entry| JournalEntry { attempt, ..entry }));
    Ok((current, entries))
}

/// The points inside [`RecoveryStore::reopen`] where a test can stop it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReopenStep {
    /// The previous journal is archived; no replacement exists yet.
    Archived,
    /// The replacement exists under its hidden name and is empty.
    Created,
    /// The replacement is complete and flushed, not yet published.
    Initialized,
}

#[cfg(not(test))]
mod fault {
    use std::path::Path;

    use super::{ReopenStep, StoreError};

    pub(super) fn at(_step: ReopenStep, _path: &Path) -> Result<(), StoreError> {
        Ok(())
    }

    pub(super) fn crashed(_error: &StoreError) -> bool {
        false
    }
}

/// Test-only fault injection for [`RecoveryStore::reopen`].
#[cfg(test)]
pub(crate) mod fault {
    use std::cell::RefCell;
    use std::io;
    use std::path::Path;

    use super::{ReopenStep, StoreError};

    /// What an injected fault does at a [`ReopenStep`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Fault {
        /// Stop as if the process died: nothing more is written or cleaned
        /// up.
        Crash,
        /// Fail as a write would, through the ordinary error path.
        Fail,
    }

    type Injector = Box<dyn FnMut(ReopenStep, &Path) -> Option<Fault>>;

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

    /// Calls `injector` at each step of every `reopen` on this thread until
    /// the guard drops. The path is the file the step concerns: the current
    /// journal, then the hidden replacement.
    #[must_use]
    pub(crate) fn inject(injector: impl FnMut(ReopenStep, &Path) -> Option<Fault> + 'static) -> Injected {
        INJECTOR.with(|slot| *slot.borrow_mut() = Some(Box::new(injector)));
        Injected
    }

    pub(super) fn at(step: ReopenStep, path: &Path) -> Result<(), StoreError> {
        let fault = INJECTOR.with(|slot| slot.borrow_mut().as_mut().and_then(|injector| injector(step, path)));
        match fault {
            None => Ok(()),
            Some(Fault::Crash) => Err(StoreError::Io(io::Error::other(CRASHED))),
            Some(Fault::Fail) => Err(StoreError::Io(io::Error::other("injected write failure"))),
        }
    }

    pub(super) fn crashed(error: &StoreError) -> bool {
        matches!(error, StoreError::Io(error) if error.to_string() == CRASHED)
    }
}

mod compact;

pub use compact::{PendingRemoval, Stored, StoredOperation};

#[cfg(test)]
mod tests;
