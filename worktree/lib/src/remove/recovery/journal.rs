//! The deletion journal: an append-only JSON Lines file recording which
//! entries of the quarantine each deletion batch planned to remove and what
//! became of them.
//!
//! ```text
//! {"header":{"journal_version":2,"operation_id":"…"}}
//! {"inventory":{"root":{…},"entries":[{"path":{"display":"target/a.o"},"identity":{…},"shape":{"file":{…}}}]}}
//! {"intent":{"batch":1,"entries":[{"path":{"display":"target/a.o"},"kind":"file"}]}}
//! {"results":{"batch":1,"entries":[{"path":{"display":"target/a.o"},"state":"deleted"}]}}
//! ```
//!
//! The second line is the operation-start [`DeletionInventory`]: every entry
//! the removal is approved to delete. It is written with the header, before
//! the move, and a batch may only plan entries it lists, with the same kind.
//! Paths are relative to the quarantine and use Sniff's lossless
//! [`NativeString`] form. A batch holds at most [`MAX_BATCH_ENTRIES`]
//! entries of one directory. Its intent is flushed before any entry is
//! unlinked and its results after, so an intent without results is the one
//! batch a crash interrupted: [`JournalLog::replay`] calls its absent entries
//! [`EntryState::Uncertain`], since nothing establishes who removed them.
//!
//! Every line is written whole and newline-terminated. An unterminated final
//! line is a write the crash cut short and is ignored ([`JournalLog::torn_tail`]);
//! a malformed terminated line makes the whole journal unreadable.
//!
//! An attempt's batch lines are bounded by [`journal_size`], computed from its
//! inventory before anything is written: the deletion engine plans each entry
//! exactly once ([`BatchPlan`]), so the largest journal it can write is known.
//! The bound is attempt-local and never stored in the journal.

use std::collections::HashSet;
use std::fs::File;
use std::sync::Arc;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sniff::filesystem::query::NativeString;

use super::OperationId;
use crate::remove::observation::deletion::BatchPlan;
use crate::remove::observation::inventory::{DeletionInventory, Shape, is_plain_relative};
use crate::remove::outcome::{EntryCounts, JournalSummary};

/// The only `journal_version` this build reads or writes.
pub const JOURNAL_VERSION: u32 = 2;

/// The most entries one deletion batch may plan. A batch also never spans
/// two directories; S2 found 256-event bursts arrive intact on FSEvents.
pub const MAX_BATCH_ENTRIES: usize = 256;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    File,
    Directory,
    /// A symlink or junction; removing it never touches its target.
    Link,
}

impl EntryKind {
    /// The kind of an inventory entry with `shape`.
    pub fn of(shape: &Shape) -> Self {
        match shape {
            Shape::File { .. } => Self::File,
            Shape::Directory { .. } => Self::Directory,
            Shape::Link { .. } => Self::Link,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedEntry {
    pub path: NativeString,
    pub kind: EntryKind,
}

/// What a batch observed for one planned entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultState {
    /// This operation removed the entry.
    Deleted,
    /// The entry was already gone when the operation reached it.
    ExternallyAbsent,
    /// The entry was left in place (changed, or the operation stopped).
    Kept,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryResult {
    path: NativeString,
    state: ResultState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Line {
    Header { journal_version: u32, operation_id: OperationId },
    Inventory(DeletionInventory),
    Intent { batch: u64, entries: Vec<PlannedEntry> },
    Results { batch: u64, entries: Vec<EntryResult> },
}

/// [`Line::Inventory`] without a copy of an inventory as large as the
/// checkout; serializes byte for byte as that line does.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum InventoryLine<'a> {
    Inventory(&'a DeletionInventory),
}

/// Why a journal cannot be read. Recovery refuses to resume and keeps the
/// quarantine as it is.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JournalInvalid {
    #[error("deletion journal line {line} is malformed: {reason}")]
    Malformed { line: usize, reason: String },
    #[error("the deletion journal has version {0}, which this wt does not read")]
    UnsupportedVersion(u64),
    #[error("the deletion journal belongs to another operation")]
    OperationMismatch,
}

/// One deletion batch as read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub number: u64,
    pub entries: Vec<PlannedEntry>,
    /// One state per entry, in order; `None` when the results never became
    /// durable.
    pub results: Option<Vec<ResultState>>,
}

/// A journal as read back, before the filesystem is consulted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalLog {
    pub inventory: DeletionInventory,
    pub batches: Vec<Batch>,
    /// An unterminated final line was dropped.
    pub torn_tail: bool,
}

/// Where a planned entry stands after recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryState {
    Deleted,
    ExternallyAbsent,
    Kept,
    /// No durable result, and the entry is still present.
    Planned,
    /// No durable result, and the entry is gone: it may or may not have been
    /// removed by this operation.
    Uncertain,
}

/// One planned entry with its recovered state; listed in full in the
/// operation report, so it outlives the journal it was read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    /// The attempt whose journal planned it: 1 for the first, then one more
    /// for each retry that started a new journal.
    pub attempt: u32,
    /// The batch within that attempt's journal.
    pub batch: u64,
    /// Relative to the quarantine.
    pub path: NativeString,
    pub kind: EntryKind,
    pub state: EntryState,
}

/// Reads the journal of `operation`.
pub fn parse(bytes: &[u8], operation: &OperationId) -> Result<JournalLog, JournalInvalid> {
    let mut lines: Vec<&[u8]> = bytes.split(|byte| *byte == b'\n').collect();
    // `split` yields an empty last piece exactly when the file ends with a
    // newline; anything else there is a torn write.
    let tail = lines.pop().unwrap_or_default();
    let torn_tail = !tail.is_empty();
    let malformed = |line: usize, reason: &str| JournalInvalid::Malformed { line, reason: reason.to_string() };

    let Some((first, rest)) = lines.split_first() else {
        return Err(malformed(1, "the header is missing"));
    };
    match serde_json::from_slice::<Line>(first) {
        Ok(Line::Header { journal_version, operation_id }) => {
            if journal_version != JOURNAL_VERSION {
                return Err(JournalInvalid::UnsupportedVersion(journal_version.into()));
            }
            if &operation_id != operation {
                return Err(JournalInvalid::OperationMismatch);
            }
        }
        Ok(_) => return Err(malformed(1, "the header is missing")),
        Err(error) => return Err(malformed(1, &error.to_string())),
    }
    let Some((second, rest)) = rest.split_first() else {
        return Err(malformed(2, "the inventory is missing"));
    };
    let inventory = match serde_json::from_slice::<Line>(second) {
        Ok(Line::Inventory(inventory)) => inventory,
        Ok(_) => return Err(malformed(2, "the inventory is missing")),
        Err(error) => return Err(malformed(2, &error.to_string())),
    };
    let mut log = JournalLog { inventory, batches: Vec::new(), torn_tail };

    for (index, text) in rest.iter().enumerate() {
        let line = index + 3;
        match serde_json::from_slice::<Line>(text).map_err(|error| malformed(line, &error.to_string()))? {
            Line::Header { .. } => return Err(malformed(line, "a second header")),
            Line::Inventory(_) => return Err(malformed(line, "a second inventory")),
            Line::Intent { batch, entries } => {
                if log.batches.last().is_some_and(|open| open.results.is_none()) {
                    return Err(malformed(line, "an intent before the previous batch's results"));
                }
                if batch != log.batches.last().map_or(1, |last| last.number + 1) {
                    return Err(malformed(line, "batch numbers are not consecutive"));
                }
                check_batch(&entries, &log.inventory).map_err(|reason| malformed(line, reason))?;
                log.batches.push(Batch { number: batch, entries, results: None });
            }
            Line::Results { batch, entries } => {
                let open = log
                    .batches
                    .last_mut()
                    .filter(|open| open.number == batch && open.results.is_none())
                    .ok_or_else(|| malformed(line, "results for a batch that is not open"))?;
                let aligned = entries.len() == open.entries.len()
                    && entries.iter().zip(&open.entries).all(|(result, planned)| result.path == planned.path);
                if !aligned {
                    return Err(malformed(line, "results do not match the batch's planned entries"));
                }
                open.results = Some(entries.into_iter().map(|result| result.state).collect());
            }
        }
    }
    Ok(log)
}

impl JournalLog {
    /// Every planned entry with its recovered state. `present` reports
    /// whether a quarantine-relative path still exists, without following
    /// links; it is asked only about entries whose batch has no results.
    /// Every entry is of attempt 1; a caller reading a later generation
    /// renumbers them.
    pub fn replay(&self, mut present: impl FnMut(&Path) -> io::Result<bool>) -> io::Result<Vec<JournalEntry>> {
        let mut entries = Vec::new();
        for batch in &self.batches {
            for (index, planned) in batch.entries.iter().enumerate() {
                let state = match batch.results.as_ref().map(|results| results[index]) {
                    Some(ResultState::Deleted) => EntryState::Deleted,
                    Some(ResultState::ExternallyAbsent) => EntryState::ExternallyAbsent,
                    Some(ResultState::Kept) => EntryState::Kept,
                    None if present(planned.path.as_path())? => EntryState::Planned,
                    None => EntryState::Uncertain,
                };
                entries.push(JournalEntry {
                    attempt: 1,
                    batch: batch.number,
                    path: planned.path.clone(),
                    kind: planned.kind,
                    state,
                });
            }
        }
        Ok(entries)
    }
}

/// Counts `entries` by state and kind for the operation report.
pub fn summarize(entries: &[JournalEntry]) -> JournalSummary {
    let mut summary = JournalSummary::default();
    for entry in entries {
        let counts: &mut EntryCounts = match entry.state {
            EntryState::Deleted => &mut summary.deleted,
            EntryState::ExternallyAbsent => &mut summary.externally_absent,
            EntryState::Kept => &mut summary.kept,
            EntryState::Planned => &mut summary.planned,
            EntryState::Uncertain => &mut summary.uncertain,
        };
        match entry.kind {
            EntryKind::File => counts.files += 1,
            EntryKind::Directory => counts.directories += 1,
            EntryKind::Link => counts.links += 1,
        }
    }
    summary
}

/// A batch the writer would refuse, and the reader too.
fn check_batch(entries: &[PlannedEntry], inventory: &DeletionInventory) -> Result<(), &'static str> {
    if entries.is_empty() || entries.len() > MAX_BATCH_ENTRIES {
        return Err("a batch plans no entries or more than the batch bound");
    }
    let parent = entries[0].path.as_path().parent();
    let mut seen = HashSet::new();
    for entry in entries {
        let path = entry.path.as_path();
        if !is_plain_relative(path) {
            return Err("an entry path is not a plain relative path");
        }
        if inventory.get(path).is_none_or(|listed| EntryKind::of(&listed.shape) != entry.kind) {
            return Err("a batch plans an entry the inventory does not list with that kind");
        }
        if path.parent() != parent {
            return Err("a batch spans more than one directory");
        }
        if !seen.insert(path) {
            return Err("a batch plans the same entry twice");
        }
    }
    Ok(())
}

/// The bytes one attempt's journal can need, computed from its inventory
/// alone with the serializer the writer uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalSize {
    /// The header and inventory lines that [`JournalWriter::start`] writes.
    pub opening: u64,
    /// The sum, over every batch [`BatchPlan`] gives the inventory, of its
    /// intent line and its longest results line (every entry
    /// [`ResultState::ExternallyAbsent`]): exactly what
    /// [`JournalWriter::begin_batch`] reserves, so it is the attempt's batch
    /// limit. An attempt that deletes everything writes no more than this; a
    /// journal of `deleted` results writes less.
    pub batches: u64,
}

impl JournalSize {
    pub fn total(&self) -> Option<u64> {
        self.opening.checked_add(self.batches)
    }
}

/// A journal size that does not fit in 64 bits; nothing is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the deletion journal's size cannot be computed: it exceeds {} bytes", u64::MAX)]
pub struct JournalSizeOverflow;

/// The [`JournalSize`] of `operation`'s journal for `inventory`. Reads
/// nothing from the filesystem, and holds at most one batch's lines in
/// memory: each line is counted as it is serialized.
///
/// ## Errors
///
/// [`JournalSizeOverflow`] when a line or the total exceeds `u64::MAX`.
pub fn journal_size(operation: &OperationId, inventory: &DeletionInventory) -> Result<JournalSize, JournalSizeOverflow> {
    let header = counted(&Line::Header { journal_version: JOURNAL_VERSION, operation_id: operation.clone() })?;
    let opening = header.checked_add(counted(&InventoryLine::Inventory(inventory))?).ok_or(JournalSizeOverflow)?;
    Ok(JournalSize { opening, batches: batch_bound(inventory)? })
}

/// [`JournalSize::batches`] of `inventory`.
fn batch_bound(inventory: &DeletionInventory) -> Result<u64, JournalSizeOverflow> {
    let plan = BatchPlan::of(inventory);
    let reservations = plan.numbered().map(|(batch, entries)| {
        let planned: Vec<PlannedEntry> =
            entries.iter().map(|entry| PlannedEntry { path: entry.path.clone(), kind: EntryKind::of(&entry.shape) }).collect();
        let intent = counted(&Line::Intent { batch, entries: planned.clone() })?;
        intent.checked_add(worst_results_len(batch, &planned)?).ok_or(JournalSizeOverflow)
    });
    checked_sum(reservations)
}

/// The length of the longest results line a batch of `entries` can need:
/// every entry [`ResultState::ExternallyAbsent`], the longest state name.
fn worst_results_len(batch: u64, entries: &[PlannedEntry]) -> Result<u64, JournalSizeOverflow> {
    let worst = entries.iter().map(|planned| EntryResult { path: planned.path.clone(), state: ResultState::ExternallyAbsent }).collect();
    counted(&Line::Results { batch, entries: worst })
}

/// The sum of `sizes`, refusing on the first error or on overflow.
fn checked_sum(mut sizes: impl Iterator<Item = Result<u64, JournalSizeOverflow>>) -> Result<u64, JournalSizeOverflow> {
    sizes.try_fold(0u64, |total, size| total.checked_add(size?).ok_or(JournalSizeOverflow))
}

/// The length of `value`'s journal line, newline included, as [`line`]
/// writes it.
fn counted(value: &impl Serialize) -> Result<u64, JournalSizeOverflow> {
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| JournalSizeOverflow)?;
    counter.0.checked_add(1).ok_or(JournalSizeOverflow)
}

/// Counts what is written to it and keeps none of it.
struct Counter(u64);

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self.0.checked_add(bytes.len() as u64).ok_or_else(|| io::Error::other(JournalSizeOverflow))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Where journal lines go. A [`File`] in production; tests inject failures.
pub trait JournalSink {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn sync(&mut self) -> io::Result<()>;
}

impl JournalSink for File {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.write_all(bytes)
    }

    fn sync(&mut self) -> io::Result<()> {
        super::storage::flush(self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JournalWriteError {
    #[error("invalid deletion batch: {0}")]
    InvalidBatch(&'static str),
    /// Nothing was written; deletion must stop. With a limit from
    /// [`journal_size`] only a batch the size did not plan reaches it, a
    /// defect in `wt` rather than anything the user can change, so the
    /// message names no byte count.
    #[error("wt reached an internal limit of its deletion journal; a retry continues")]
    LimitReached { limit: u64 },
    /// The line may be partly written. Deletion must stop, and every later
    /// write is refused.
    #[error("could not write the deletion journal: {0}")]
    Io(#[from] io::Error),
    #[error("the deletion journal failed earlier and accepts no more writes")]
    Poisoned,
}

/// Appends batches to a journal whose header it wrote.
pub struct JournalWriter<S: JournalSink = File> {
    sink: S,
    inventory: Arc<DeletionInventory>,
    written: u64,
    limit: u64,
    last_batch: u64,
    open: Option<Vec<PlannedEntry>>,
    poisoned: bool,
}

impl<S: JournalSink> JournalWriter<S> {
    /// Writes and flushes the header and `inventory` of `operation`'s
    /// journal to `sink`. `limit` bounds the batch lines that follow; an
    /// attempt passes its [`JournalSize::batches`], which every batch it can
    /// plan fits, so reaching the limit is a defect in `wt`.
    pub fn start(
        sink: S,
        operation: &OperationId,
        inventory: Arc<DeletionInventory>,
        limit: u64,
    ) -> Result<Self, JournalWriteError> {
        let header = line(&Line::Header { journal_version: JOURNAL_VERSION, operation_id: operation.clone() });
        let listed = line(&InventoryLine::Inventory(&inventory));
        let mut writer = Self::after_header(sink, inventory, limit);
        let mut opening = header;
        opening.extend_from_slice(&listed);
        writer.write(&opening)?;
        // The inventory is as large as the checkout; only batches count
        // against the limit.
        writer.written = 0;
        Ok(writer)
    }

    /// Continues, through a newly opened `sink`, a journal that holds only
    /// its header and inventory.
    pub(super) fn after_header(sink: S, inventory: Arc<DeletionInventory>, limit: u64) -> Self {
        Self { sink, inventory, written: 0, limit, last_batch: 0, open: None, poisoned: false }
    }

    pub fn inventory(&self) -> &Arc<DeletionInventory> {
        &self.inventory
    }

    /// The bytes of batch lines this writer accepts.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Flushes the intent to delete `entries`, which must be the children
    /// of one directory. Returns the batch number. No entry may be unlinked
    /// before this returns `Ok`.
    pub fn begin_batch(&mut self, entries: Vec<PlannedEntry>) -> Result<u64, JournalWriteError> {
        if self.poisoned {
            return Err(JournalWriteError::Poisoned);
        }
        if self.open.is_some() {
            return Err(JournalWriteError::InvalidBatch("the previous batch has no results"));
        }
        check_batch(&entries, &self.inventory).map_err(JournalWriteError::InvalidBatch)?;
        let batch = self.last_batch + 1;
        let intent = line(&Line::Intent { batch, entries: entries.clone() });
        // Reserve the longest results line this batch could need.
        let reserve = worst_results_len(batch, &entries).map_err(|_| JournalWriteError::LimitReached { limit: self.limit })?;
        let needed = (intent.len() as u64).checked_add(reserve).and_then(|batch| batch.checked_add(self.written));
        if needed.is_none_or(|needed| needed > self.limit) {
            return Err(JournalWriteError::LimitReached { limit: self.limit });
        }
        self.write(&intent)?;
        self.last_batch = batch;
        self.open = Some(entries);
        Ok(batch)
    }

    /// Flushes the results of the open batch, one state per planned entry in
    /// order.
    pub fn finish_batch(&mut self, states: &[ResultState]) -> Result<(), JournalWriteError> {
        if self.poisoned {
            return Err(JournalWriteError::Poisoned);
        }
        let Some(entries) = self.open.as_ref() else {
            return Err(JournalWriteError::InvalidBatch("no batch is open"));
        };
        if states.len() != entries.len() {
            return Err(JournalWriteError::InvalidBatch("one result per planned entry is required"));
        }
        let results = results_line(self.last_batch, entries, states);
        self.write(&results)?;
        self.open = None;
        Ok(())
    }

    /// The batch whose intent is durable and whose results are not.
    pub fn open_batch(&self) -> Option<u64> {
        self.open.as_ref().map(|_| self.last_batch)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), JournalWriteError> {
        let result = self.sink.append(bytes).and_then(|()| self.sink.sync());
        if let Err(error) = result {
            self.poisoned = true;
            return Err(JournalWriteError::Io(error));
        }
        self.written += bytes.len() as u64;
        Ok(())
    }
}

fn results_line(batch: u64, entries: &[PlannedEntry], states: &[ResultState]) -> Vec<u8> {
    let entries = entries
        .iter()
        .zip(states)
        .map(|(planned, state)| EntryResult { path: planned.path.clone(), state: *state })
        .collect();
    line(&Line::Results { batch, entries })
}

fn line(line: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(line).expect("a journal line always serializes");
    bytes.push(b'\n');
    bytes
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A limit no test journal reaches.
    const UNBOUNDED: u64 = u64::MAX;

    fn id() -> OperationId {
        OperationId::parse("0123456789abcdef0123456789abcdef").unwrap()
    }

    fn file(path: &str) -> PlannedEntry {
        PlannedEntry { path: NativeString::from(path), kind: EntryKind::File }
    }

    /// Lists every file a test plans, so batches name inventory entries.
    fn listed() -> Arc<DeletionInventory> {
        let mut files: Vec<String> = ["d/a", "d/b", "d/c", "e/x", "e/y", "e/b", "a", "b", "x"].map(String::from).to_vec();
        files.extend((0..=MAX_BATCH_ENTRIES).map(|n| format!("d/{n}")));
        Arc::new(DeletionInventory::listing(sniff::filesystem::query::FileIdentity::Unix { device: 1, inode: 1 }, &files.iter().map(String::as_str).collect::<Vec<_>>()))
    }

    fn start(sink: MemorySink, limit: u64) -> Result<JournalWriter<MemorySink>, JournalWriteError> {
        JournalWriter::start(sink, &id(), listed(), limit)
    }

    /// An in-memory sink that can fail its `n`th append or sync.
    #[derive(Clone, Default)]
    struct MemorySink {
        bytes: Arc<Mutex<Vec<u8>>>,
        fail_append: Option<usize>,
        fail_sync: Option<usize>,
        appends: usize,
        syncs: usize,
        /// Bytes of a failing append that still land, as a torn write would.
        torn: usize,
    }

    impl JournalSink for MemorySink {
        fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.appends += 1;
            if self.fail_append == Some(self.appends) {
                self.bytes.lock().unwrap().extend_from_slice(&bytes[..self.torn.min(bytes.len())]);
                return Err(io::Error::other("injected append failure"));
            }
            self.bytes.lock().unwrap().extend_from_slice(bytes);
            Ok(())
        }

        fn sync(&mut self) -> io::Result<()> {
            self.syncs += 1;
            if self.fail_sync == Some(self.syncs) {
                return Err(io::Error::other("injected sync failure"));
            }
            Ok(())
        }
    }

    impl MemorySink {
        fn contents(&self) -> Vec<u8> {
            self.bytes.lock().unwrap().clone()
        }
    }

    fn states(log: &JournalLog, present: &BTreeSet<&str>) -> Vec<(String, EntryState)> {
        log.replay(|path| Ok(present.contains(path.to_str().unwrap())))
            .unwrap()
            .into_iter()
            .map(|entry| (entry.path.display().into_owned(), entry.state))
            .collect()
    }

    #[test]
    fn a_journal_without_batches_means_nothing_was_attempted() {
        let sink = MemorySink::default();
        start(sink.clone(), UNBOUNDED).unwrap();
        let log = parse(&sink.contents(), &id()).unwrap();
        assert!(log.batches.is_empty());
        assert!(!log.torn_tail);
        assert_eq!(summarize(&log.replay(|_| unreachable!()).unwrap()), JournalSummary::default());
    }

    #[test]
    fn confirmed_results_and_an_interrupted_batch_recover_distinctly() {
        let sink = MemorySink::default();
        let mut writer = start(sink.clone(), UNBOUNDED).unwrap();
        writer.begin_batch(vec![file("d/a"), file("d/b"), file("d/c")]).unwrap();
        writer.finish_batch(&[ResultState::Deleted, ResultState::ExternallyAbsent, ResultState::Kept]).unwrap();
        writer.begin_batch(vec![file("e/x"), file("e/y")]).unwrap();
        assert_eq!(writer.open_batch(), Some(2));
        // The crash: `e/x` was unlinked, its result never written.
        let log = parse(&sink.contents(), &id()).unwrap();
        let present = BTreeSet::from(["e/y", "d/c"]);
        assert_eq!(
            states(&log, &present),
            vec![
                ("d/a".into(), EntryState::Deleted),
                ("d/b".into(), EntryState::ExternallyAbsent),
                ("d/c".into(), EntryState::Kept),
                ("e/x".into(), EntryState::Uncertain),
                ("e/y".into(), EntryState::Planned),
            ]
        );
        let summary = summarize(&log.replay(|path| Ok(present.contains(path.to_str().unwrap()))).unwrap());
        assert_eq!((summary.deleted.files, summary.uncertain.files, summary.planned.files), (1, 1, 1));
    }

    #[test]
    fn a_torn_results_line_leaves_its_batch_uncertain() {
        let sink = MemorySink { fail_append: Some(3), torn: 20, ..MemorySink::default() };
        let mut writer = start(sink.clone(), UNBOUNDED).unwrap();
        writer.begin_batch(vec![file("a")]).unwrap();
        let error = writer.finish_batch(&[ResultState::Deleted]).unwrap_err();
        assert!(matches!(error, JournalWriteError::Io(_)), "{error}");
        assert!(matches!(writer.begin_batch(vec![file("b")]), Err(JournalWriteError::Poisoned)));

        let log = parse(&sink.contents(), &id()).unwrap();
        assert!(log.torn_tail);
        assert_eq!(states(&log, &BTreeSet::new()), vec![("a".into(), EntryState::Uncertain)]);
    }

    #[test]
    fn a_failed_intent_flush_refuses_the_batch_and_every_later_write() {
        let sink = MemorySink { fail_sync: Some(2), ..MemorySink::default() };
        let mut writer = start(sink.clone(), UNBOUNDED).unwrap();
        assert!(matches!(writer.begin_batch(vec![file("a")]), Err(JournalWriteError::Io(_))));
        assert_eq!(writer.open_batch(), None);
        assert!(matches!(writer.finish_batch(&[ResultState::Deleted]), Err(JournalWriteError::Poisoned)));
    }

    #[test]
    fn the_limit_refuses_an_intent_while_its_results_still_fit() {
        let opening = {
            let probe = MemorySink::default();
            start(probe.clone(), 0).unwrap();
            probe.contents().len()
        };
        let entries = vec![file("a")];
        let intent = line(&Line::Intent { batch: 1, entries: entries.clone() }).len() as u64;
        let worst = results_line(1, &entries, &[ResultState::ExternallyAbsent]).len() as u64;

        // The header and inventory never count: a zero limit still opens.
        // One byte short of intent plus worst-case results: refused, nothing written.
        let sink = MemorySink::default();
        let mut writer = start(sink.clone(), intent + worst - 1).unwrap();
        assert!(matches!(writer.begin_batch(entries.clone()), Err(JournalWriteError::LimitReached { .. })));
        assert_eq!(sink.contents().len(), opening);

        // Exactly enough: the batch and its results both fit.
        let sink = MemorySink::default();
        let mut writer = start(sink.clone(), intent + worst).unwrap();
        writer.begin_batch(entries).unwrap();
        writer.finish_batch(&[ResultState::ExternallyAbsent]).unwrap();
        let log = parse(&sink.contents(), &id()).unwrap();
        assert_eq!(states(&log, &BTreeSet::new()), vec![("a".into(), EntryState::ExternallyAbsent)]);
    }

    #[test]
    fn a_batch_names_only_inventory_entries_of_their_kind() {
        let mut writer = start(MemorySink::default(), UNBOUNDED).unwrap();
        for entries in [
            vec![file("d/unlisted")],
            vec![PlannedEntry { path: NativeString::from("d/a"), kind: EntryKind::Directory }],
            vec![PlannedEntry { path: NativeString::from("d"), kind: EntryKind::Link }],
        ] {
            let shown = format!("{entries:?}");
            assert!(matches!(writer.begin_batch(entries), Err(JournalWriteError::InvalidBatch(_))), "{shown}");
        }
        assert_eq!(writer.begin_batch(vec![PlannedEntry { path: NativeString::from("d"), kind: EntryKind::Directory }]).unwrap(), 1);
    }

    #[test]
    fn batches_outside_the_bound_are_refused_by_writer_and_reader() {
        let mut writer = start(MemorySink::default(), UNBOUNDED).unwrap();
        let too_many: Vec<_> = (0..=MAX_BATCH_ENTRIES).map(|n| file(&format!("d/{n}"))).collect();
        let full: Vec<_> = (0..MAX_BATCH_ENTRIES).map(|n| file(&format!("d/{n}"))).collect();
        let invalid = [
            vec![],
            too_many,
            vec![file("d/a"), file("e/b")],
            vec![file("d/a"), file("d/a")],
            vec![file("../escape")],
            vec![file("/absolute")],
            vec![file("")],
        ];
        for entries in invalid {
            let shown = format!("{:?}", entries.iter().take(2).collect::<Vec<_>>());
            assert!(matches!(writer.begin_batch(entries), Err(JournalWriteError::InvalidBatch(_))), "{shown}");
        }
        assert_eq!(writer.begin_batch(full).unwrap(), 1);
        assert!(matches!(writer.begin_batch(vec![file("x")]), Err(JournalWriteError::InvalidBatch(_))));
        assert!(matches!(writer.finish_batch(&[ResultState::Deleted]), Err(JournalWriteError::InvalidBatch(_))));
    }

    #[test]
    fn another_operations_journal_is_refused() {
        let sink = MemorySink::default();
        start(sink.clone(), UNBOUNDED).unwrap();
        let other = OperationId::parse("ffffffffffffffffffffffffffffffff").unwrap();
        assert_eq!(parse(&sink.contents(), &other), Err(JournalInvalid::OperationMismatch));
    }

    use std::ffi::OsString;
    use std::path::PathBuf;

    use sniff::filesystem::query::FileIdentity;

    use crate::remove::observation::deletion::BatchPlan;
    use crate::remove::observation::inventory::{Content, InventoryEntry, Timestamp};

    /// A sink that counts what it is given and keeps nothing.
    #[derive(Default)]
    struct CountingSink(u64);

    impl JournalSink for CountingSink {
        fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.0 += bytes.len() as u64;
            Ok(())
        }

        fn sync(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn entry(path: impl Into<PathBuf>, inode: u64, directory: bool) -> InventoryEntry {
        let path: PathBuf = path.into();
        let shape = if directory {
            Shape::Directory { mode: 0o755 }
        } else {
            Shape::File { size: 1, modified: Timestamp { secs: 1, nanos: 0 }, mode: 0o644, content: Content::Unhashed }
        };
        InventoryEntry { path: NativeString::from(path.as_path()), identity: FileIdentity::Unix { device: 1, inode }, shape }
    }

    /// A name the platform allows that is not valid UTF-8: a lone `0xff`
    /// byte on Unix, an unpaired surrogate on Windows.
    fn non_utf8_name() -> OsString {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(b"obj-\xff.o".to_vec())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[0x6f, 0xd800, 0x6f])
        }
    }

    /// Every shape the bound must count: short and long paths, names JSON
    /// escapes (quotes, backslashes, control characters, non-ASCII), a
    /// non-UTF-8 name, hard links (one identity, two paths), many small
    /// directories, more than `MAX_BATCH_ENTRIES` siblings, a link, and
    /// `.git`; over a hundred batches, so batch numbers cross both digit
    /// transitions.
    fn varied() -> DeletionInventory {
        let mut entries = vec![
            entry(".git", 2, true),
            entry(".git/HEAD", 3, false),
            entry("long", 4, true),
            entry(format!("long/{}", "segment-".repeat(60)), 5, false),
            entry("odd", 6, true),
            // A backslash is a separator, not a name character, on Windows.
            entry(if cfg!(windows) { "odd/quote\"" } else { "odd/quote\"back\\slash" }, 7, false),
            entry("odd/control\u{1}\u{1f}tab\t", 8, false),
            entry("odd/ünïcødé-名前", 9, false),
            entry(PathBuf::from("odd").join(non_utf8_name()), 10, false),
            // A hard link: the same identity at a second path.
            entry("odd/alias-of-quote", 7, false),
            InventoryEntry {
                path: NativeString::from("odd/link"),
                identity: FileIdentity::Unix { device: 1, inode: 11 },
                shape: Shape::Link { target: NativeString::from("../elsewhere") },
            },
            entry("wide", 12, true),
        ];
        entries.extend((0..MAX_BATCH_ENTRIES + 1).map(|n| entry(format!("wide/{n}"), 100 + n as u64, false)));
        let mut inode = 1_000;
        for directory in 0..110 {
            inode += 2;
            entries.push(entry(format!("small-{directory}"), inode, true));
            entries.push(entry(format!("small-{directory}/f"), inode + 1, false));
        }
        DeletionInventory::of_entries(FileIdentity::Unix { device: 1, inode: 1 }, entries).unwrap()
    }

    /// Writes every planned batch of `inventory` to `writer`, with results
    /// from `state`, and returns the batches that were written.
    fn replay<S: JournalSink>(
        writer: &mut JournalWriter<S>,
        inventory: &DeletionInventory,
        state: impl Fn(usize) -> ResultState,
    ) -> Result<u64, JournalWriteError> {
        let mut written = 0;
        for (number, batch) in BatchPlan::of(inventory).numbered() {
            let planned = batch.iter().map(|entry| PlannedEntry { path: entry.path.clone(), kind: EntryKind::of(&entry.shape) }).collect();
            assert_eq!(writer.begin_batch(planned)?, number, "the writer numbers batches as the plan does");
            let states: Vec<ResultState> = (0..batch.len()).map(&state).collect();
            writer.finish_batch(&states)?;
            written = number;
        }
        Ok(written)
    }

    #[test]
    fn the_opening_size_is_what_start_writes() {
        for inventory in [varied(), DeletionInventory::empty(FileIdentity::Unix { device: 1, inode: 1 })] {
            let size = journal_size(&id(), &inventory).unwrap();
            let sink = MemorySink::default();
            JournalWriter::start(sink.clone(), &id(), Arc::new(inventory.clone()), size.batches).unwrap();
            assert_eq!(size.opening, sink.contents().len() as u64);
            // The borrowed inventory line reads back as the inventory.
            assert_eq!(parse(&sink.contents(), &id()).unwrap().inventory, inventory);
        }
    }

    #[test]
    fn an_empty_inventory_needs_no_batch_bytes() {
        let size = journal_size(&id(), &DeletionInventory::empty(FileIdentity::Unix { device: 1, inode: 1 })).unwrap();
        assert_eq!(size.batches, 0);
        assert_eq!(size.total(), Some(size.opening));
    }

    /// The bound is exactly the worst case: replaying every batch with
    /// `externally_absent` results fills it to the byte, and one byte less
    /// refuses the final reservation and writes nothing of that batch.
    #[test]
    fn worst_case_results_fill_the_bound_exactly_and_one_byte_less_refuses_the_last_batch() {
        let inventory = varied();
        let size = journal_size(&id(), &inventory).unwrap();
        let batches = BatchPlan::of(&inventory).numbered().count() as u64;
        assert!(batches > 100, "{batches} batches do not cross the 9→10 and 99→100 transitions");

        let sink = MemorySink::default();
        let mut writer = JournalWriter::start(sink.clone(), &id(), Arc::new(inventory.clone()), size.batches).unwrap();
        assert_eq!(writer.limit(), size.batches);
        assert_eq!(replay(&mut writer, &inventory, |_| ResultState::ExternallyAbsent).unwrap(), batches);
        assert_eq!(sink.contents().len() as u64, size.opening + size.batches, "filled exactly");
        let log = parse(&sink.contents(), &id()).unwrap();
        assert_eq!(log.batches.len() as u64, batches);

        let sink = MemorySink::default();
        let mut writer = JournalWriter::start(sink.clone(), &id(), Arc::new(inventory.clone()), size.batches - 1).unwrap();
        let error = replay(&mut writer, &inventory, |_| ResultState::ExternallyAbsent).expect_err("one byte short still fit");
        assert!(matches!(error, JournalWriteError::LimitReached { limit } if limit == size.batches - 1), "{error}");
        let log = parse(&sink.contents(), &id()).unwrap();
        assert_eq!(log.batches.len() as u64, batches - 1, "only the final reservation is refused");
        assert!(log.batches.iter().all(|batch| batch.results.is_some()), "no intent is left without room for its results");
    }

    /// Every result state, and any mix of them, fits the bound.
    #[test]
    fn every_result_state_fits_the_bound() {
        let inventory = varied();
        let size = journal_size(&id(), &inventory).unwrap();
        let mixed = |n: usize| [ResultState::Deleted, ResultState::Kept, ResultState::ExternallyAbsent][n % 3];
        for state in [&(|_| ResultState::Deleted) as &dyn Fn(usize) -> ResultState, &|_| ResultState::Kept, &mixed] {
            let mut writer = JournalWriter::start(CountingSink::default(), &id(), Arc::new(inventory.clone()), size.batches).unwrap();
            replay(&mut writer, &inventory, state).unwrap();
        }
    }

    #[test]
    fn a_size_past_u64_is_refused_not_wrapped() {
        assert_eq!(checked_sum([Ok(u64::MAX), Ok(1)].into_iter()), Err(JournalSizeOverflow));
        assert_eq!(checked_sum([Ok(1), Err(JournalSizeOverflow), Ok(1)].into_iter()), Err(JournalSizeOverflow));
        assert_eq!(checked_sum([Ok(u64::MAX - 1), Ok(1)].into_iter()), Ok(u64::MAX));
        assert_eq!(JournalSize { opening: u64::MAX, batches: 1 }.total(), None);
        let mut counter = Counter(u64::MAX - 1);
        assert!(counter.write_all(b"xy").is_err());
    }

    /// The fixed 64 MiB limit stopped real removals. An inventory whose
    /// bound exceeds it now streams every batch, through a sink that keeps
    /// nothing, without reaching its limit or passing its bound.
    #[test]
    fn a_journal_past_the_former_64_mib_ceiling_streams_without_reaching_its_limit() {
        const FORMER_LIMIT: u64 = 64 * 1024 * 1024;
        // Long paths, as under `target/`, keep the entry count (and the
        // test's time) down while the bytes pass the old ceiling.
        // A control character serializes as a six-byte escape, so names of
        // them pass the ceiling with a sixth of the entries and of the
        // per-byte work an ordinary name would need. The inventory line is
        // not a batch line and never counted against the limit, so the writer
        // starts after it and the sizing skips it.
        let directory = format!("target/{}", "debug-incremental-".repeat(20));
        let mut entries = vec![entry("target", 2, true), entry(&directory, 3, true)];
        let leaf = "\u{1}".repeat(700);
        entries.extend((0..7_400u64).map(|n| entry(format!("{directory}/{leaf}-{n}"), 10 + n, false)));
        let inventory = Arc::new(DeletionInventory::of_entries(FileIdentity::Unix { device: 1, inode: 1 }, entries).unwrap());
        let bound = batch_bound(&inventory).unwrap();
        assert!(bound > FORMER_LIMIT, "the bound {bound} does not pass the former ceiling");

        let mut writer = JournalWriter::after_header(CountingSink::default(), Arc::clone(&inventory), bound);
        replay(&mut writer, &inventory, |_| ResultState::Deleted).unwrap();
        let emitted = writer.sink.0;
        assert!(emitted > FORMER_LIMIT && emitted <= bound, "{emitted} of {bound}");
    }

}
