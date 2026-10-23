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

use std::collections::HashSet;
use std::fs::File;
use std::sync::Arc;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sniff::filesystem::query::NativeString;

use super::OperationId;
use crate::remove::observation::inventory::{DeletionInventory, Shape, is_plain_relative};
use crate::remove::outcome::{EntryCounts, JournalSummary};

/// The only `journal_version` this build reads or writes.
pub const JOURNAL_VERSION: u32 = 2;

/// The most entries one deletion batch may plan. A batch also never spans
/// two directories; S2 found 256-event bursts arrive intact on FSEvents.
pub const MAX_BATCH_ENTRIES: usize = 256;

/// The journal's size limit for the deletion batches of `inventory`: the
/// most bytes their lines can take when every listed entry is planned once,
/// each in a batch of its own, with the longest kind and result. A journal
/// plans each entry at most once, so the limit binds only a writer that has
/// gone wrong, never a removal, however large the checkout. The header and
/// inventory lines do not count against it.
pub fn limit_for(inventory: &DeletionInventory) -> u64 {
    let empty: [PlannedEntry; 0] = [];
    let per_batch = (line(&Line::Intent { batch: u64::MAX, entries: empty.to_vec() }).len()
        + results_line(u64::MAX, &empty, &[]).len()) as u64;
    inventory
        .entries()
        .iter()
        .map(|entry| {
            let path = entry.path.clone();
            // Each element, with the comma that separates it from the next.
            let planned = json_len(&PlannedEntry { path: path.clone(), kind: EntryKind::Directory }) + 1;
            let result = json_len(&EntryResult { path, state: ResultState::ExternallyAbsent }) + 1;
            planned + result + per_batch
        })
        .sum()
}

/// The length of `value` as JSON, without keeping the bytes.
fn json_len(value: &impl Serialize) -> u64 {
    struct Count(u64);
    impl Write for Count {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 += bytes.len() as u64;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value).expect("a journal element always serializes");
    count.0
}

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
    /// Nothing was written; deletion must stop.
    #[error("the deletion journal reached its {limit}-byte limit")]
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
    /// Which inventory entries an earlier batch of this journal planned.
    planned: Vec<bool>,
    last_batch: u64,
    open: Option<Vec<PlannedEntry>>,
    poisoned: bool,
}

impl<S: JournalSink> JournalWriter<S> {
    /// Writes and flushes the header and `inventory` of `operation`'s
    /// journal to `sink`.
    pub fn start(
        sink: S,
        operation: &OperationId,
        inventory: Arc<DeletionInventory>,
        limit: u64,
    ) -> Result<Self, JournalWriteError> {
        let header = line(&Line::Header { journal_version: JOURNAL_VERSION, operation_id: operation.clone() });
        let listed = line(&Line::Inventory((*inventory).clone()));
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
        let planned = vec![false; inventory.entries().len()];
        Self { sink, inventory, written: 0, limit, planned, last_batch: 0, open: None, poisoned: false }
    }

    pub fn inventory(&self) -> &Arc<DeletionInventory> {
        &self.inventory
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
        // `limit_for` counts every entry once.
        let positions: Vec<usize> =
            entries.iter().filter_map(|entry| self.inventory.position(entry.path.as_path())).collect();
        if positions.iter().any(|&at| self.planned[at]) {
            return Err(JournalWriteError::InvalidBatch("a batch plans an entry an earlier batch planned"));
        }
        let batch = self.last_batch + 1;
        let intent = line(&Line::Intent { batch, entries: entries.clone() });
        // Reserve the longest results line this batch could need.
        let reserve = results_line(batch, &entries, &vec![ResultState::ExternallyAbsent; entries.len()]).len();
        if self.written + (intent.len() + reserve) as u64 > self.limit {
            return Err(JournalWriteError::LimitReached { limit: self.limit });
        }
        self.write(&intent)?;
        for at in positions {
            self.planned[at] = true;
        }
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

fn line(line: &Line) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(line).expect("a journal line always serializes");
    bytes.push(b'\n');
    bytes
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::{Arc, Mutex};

    use super::*;

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
        start(sink.clone(), limit_for(&listed())).unwrap();
        let log = parse(&sink.contents(), &id()).unwrap();
        assert!(log.batches.is_empty());
        assert!(!log.torn_tail);
        assert_eq!(summarize(&log.replay(|_| unreachable!()).unwrap()), JournalSummary::default());
    }

    #[test]
    fn confirmed_results_and_an_interrupted_batch_recover_distinctly() {
        let sink = MemorySink::default();
        let mut writer = start(sink.clone(), limit_for(&listed())).unwrap();
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
        let mut writer = start(sink.clone(), limit_for(&listed())).unwrap();
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
        let mut writer = start(sink.clone(), limit_for(&listed())).unwrap();
        assert!(matches!(writer.begin_batch(vec![file("a")]), Err(JournalWriteError::Io(_))));
        assert_eq!(writer.open_batch(), None);
        assert!(matches!(writer.finish_batch(&[ResultState::Deleted]), Err(JournalWriteError::Poisoned)));
    }

    /// The limit for an inventory holds its worst case: every entry in a
    /// batch of its own, each result the longest.
    #[test]
    fn the_limit_fits_every_entry_planned_alone_with_the_longest_result() {
        let inventory = listed();
        let sink = MemorySink::default();
        let mut writer = JournalWriter::start(sink, &id(), Arc::clone(&inventory), limit_for(&inventory)).unwrap();
        for entry in inventory.entries() {
            let planned = PlannedEntry { path: entry.path.clone(), kind: EntryKind::of(&entry.shape) };
            writer.begin_batch(vec![planned]).unwrap();
            writer.finish_batch(&[ResultState::ExternallyAbsent]).unwrap();
        }
        assert!(writer.written <= writer.limit, "{} of {}", writer.written, writer.limit);
    }

    /// The limit counts each entry once, so planning one again is refused
    /// before anything is written, in any later batch.
    #[test]
    fn an_entry_an_earlier_batch_planned_is_refused() {
        let sink = MemorySink::default();
        let mut writer = start(sink.clone(), limit_for(&listed())).unwrap();
        writer.begin_batch(vec![file("d/a"), file("d/b")]).unwrap();
        writer.finish_batch(&[ResultState::Kept, ResultState::Deleted]).unwrap();
        let before = sink.contents().len();

        assert!(matches!(writer.begin_batch(vec![file("d/c"), file("d/a")]), Err(JournalWriteError::InvalidBatch(_))));
        assert_eq!(sink.contents().len(), before, "nothing written");
        writer.begin_batch(vec![file("d/c")]).unwrap();
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
        let mut writer = start(MemorySink::default(), limit_for(&listed())).unwrap();
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
        let mut writer = start(MemorySink::default(), limit_for(&listed())).unwrap();
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
        start(sink.clone(), limit_for(&listed())).unwrap();
        let other = OperationId::parse("ffffffffffffffffffffffffffffffff").unwrap();
        assert_eq!(parse(&sink.contents(), &other), Err(JournalInvalid::OperationMismatch));
    }
}
