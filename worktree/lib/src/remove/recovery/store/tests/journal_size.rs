//! Storage size of a large removal: a completed operation's folder against
//! the version 5 layout, and version 3 journal batches against version 2.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::remove::outcome::ReportNote;
use crate::remove::recovery::journal::{JournalSink, MAX_BATCH_ENTRIES, summarize};

/// How many files the large removal deletes.
const FILES: usize = 100_000;

/// The length of every file path, relative to the quarantine.
const PATH_LEN: usize = 150;

/// File `n` of the large removal: one shared directory, padding, a unique
/// six-digit number, and an extension, [`PATH_LEN`] bytes in all.
fn paths() -> Vec<String> {
    let padding = "p".repeat(PATH_LEN - "target/".len() - 6 - ".o".len());
    (0..FILES).map(|n| format!("target/{padding}{n:06}.o")).collect()
}

/// The deterministic result of file `n`: mostly deleted, with some already
/// absent and some kept.
fn result_of(n: usize) -> ResultState {
    match n % 100 {
        0 => ResultState::ExternallyAbsent,
        1 => ResultState::Kept,
        _ => ResultState::Deleted,
    }
}

/// The recovered state of file `n` and its spelling in a saved report:
/// [`result_of`], except that a few had no durable result and are gone.
fn state_of(n: usize) -> (EntryState, &'static str) {
    match (n % 1000, result_of(n)) {
        (2, _) => (EntryState::Uncertain, "uncertain"),
        (_, ResultState::ExternallyAbsent) => (EntryState::ExternallyAbsent, "externally_absent"),
        (_, ResultState::Kept) => (EntryState::Kept, "kept"),
        (_, ResultState::Deleted) => (EntryState::Deleted, "deleted"),
    }
}

/// The batch that planned file `n`, filling every batch but the last.
fn batch_of(n: usize) -> u64 {
    (n / MAX_BATCH_ENTRIES + 1) as u64
}

/// Every file of `dir` (recursively) with its logical length.
fn file_sizes(dir: &Path) -> Vec<(String, u64)> {
    let mut sizes = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if metadata.is_dir() {
            sizes.extend(file_sizes(&entry.path()).into_iter().map(|(inner, len)| (format!("{name}/{inner}"), len)));
        } else {
            sizes.push((name, metadata.len()));
        }
    }
    sizes.sort();
    sizes
}

/// Completing a removal of 100,000 files leaves its record, a version 6
/// report, and the compressed entries file, which together take under 5%
/// of what the record and a version 5 report listing every entry inline
/// took. Completed journals were deleted in both layouts, so neither side
/// counts them.
#[test]
fn a_completed_removal_of_100k_entries_takes_under_5_percent_of_the_version_5_folder() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let op = begin(&f, saved.clone());
    let dir = op.dir().to_path_buf();
    let quarantine = saved.quarantine.as_path().to_path_buf();

    let paths = paths();
    let mut entries = Vec::with_capacity(FILES);
    let mut inline = Vec::with_capacity(FILES);
    for (n, path) in paths.iter().enumerate() {
        assert_eq!(path.len(), PATH_LEN, "{path}");
        let (state, spelled) = state_of(n);
        let batch = batch_of(n);
        entries.push(JournalEntry { attempt: 1, batch, path: NativeString::from(path.as_str()), kind: EntryKind::File, state });
        // Frozen: a version 5 report's inline entry.
        inline.push(json!({"attempt": 1, "batch": batch, "path": {"display": path}, "kind": "file", "state": spelled}));
    }

    // Filled as the lifecycle fills a final report.
    let mut report = OperationReport::new(Some(saved.operation_id.clone()), RemovalOutcome::CleanupFailed, saved.original.clone());
    report.journal = summarize(&entries);
    let deleted = entries.iter().filter(|entry| entry.state == EntryState::Deleted);
    report.set_deleted(deleted.map(|entry| NativeString::from(quarantine.join(entry.path.as_path()).as_path())).collect());
    report.notes.push(ReportNote::ConcurrentWriteLimit);
    report.set_entries(entries);
    assert_eq!(op.complete(&report, true).unwrap(), Vec::new());

    // The baseline: a version 5 report with the same members, listing every
    // entry inline.
    let version_5 = json!({
        "report_version": 5,
        "operation_id": report.operation_id,
        "outcome": report.outcome,
        "original": report.original,
        "original_state": report.original_state,
        "quarantine": report.quarantine,
        "registration": report.registration,
        "timeline": report.timeline,
        "discovery": report.discovery,
        "activity": report.activity,
        "journal": report.journal,
        "journal_entries": inline,
        "deleted": report.deleted,
        "notes": report.notes,
        "branches": report.branches,
        "errors": report.errors,
        "truncation": report.truncation,
        "earlier_attempts": report.earlier_attempts,
        "observation": report.observation,
    });
    let old_report = serde_json::to_vec_pretty(&version_5).unwrap().len() as u64;
    let mut same_members = version_5;
    let members = same_members.as_object_mut().unwrap();
    members.remove("journal_entries");
    members.insert("report_version".into(), json!(6));
    members.insert("entries_file".into(), json!(ENTRIES_FILE));
    let saved_report: Value = serde_json::from_slice(&fs::read(dir.join(REPORT_FILE)).unwrap()).unwrap();
    assert_eq!(saved_report, same_members, "the baseline differs only in where the entries are");

    let sizes = file_sizes(&dir);
    let names: Vec<&str> = sizes.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, [ENTRIES_FILE, RECORD_FILE, REPORT_FILE], "no journal or temporary remains");
    let record_bytes = sizes[1].1;
    let new_bytes: u64 = sizes.iter().map(|(_, len)| len).sum();
    let old_bytes = record_bytes + old_report;
    let measured = format!(
        "new folder {new_bytes} bytes ({}); version 5 folder {old_bytes} bytes ({RECORD_FILE} {record_bytes}, {REPORT_FILE} {old_report}); ratio {:.3}%",
        sizes.iter().map(|(name, len)| format!("{name} {len}")).collect::<Vec<_>>().join(", "),
        new_bytes as f64 * 100.0 / old_bytes as f64,
    );
    eprintln!("{measured}");
    assert!(new_bytes * 100 < old_bytes * 5, "{measured}");

    assert!(read_entries(&dir.join(ENTRIES_FILE)) == report.journal_entries, "the entries file lists every entry");
}

/// A journal sink that keeps every byte in memory.
#[derive(Clone, Default)]
struct Memory(Rc<RefCell<Vec<u8>>>);

impl JournalSink for Memory {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The same 100,000 files planned in full batches of one directory, as
/// this build writes them (version 3) and as an older `wt` did (frozen
/// version 2 bytes after the same inventory line).
struct Journals {
    id: OperationId,
    v3: Vec<u8>,
    /// The length of the header and inventory lines.
    v3_opening: usize,
    v2: Vec<u8>,
    v2_opening: usize,
}

fn journals() -> Journals {
    let id = OperationId::parse("0123456789abcdef0123456789abcdef").unwrap();
    let paths = paths();
    let listed: Vec<&str> = paths.iter().map(String::as_str).collect();
    let inventory = Arc::new(DeletionInventory::listing(identity(3), &listed));
    assert_eq!(inventory.entries().len(), FILES + 1, "the files and their directory");
    assert_eq!(inventory.position(Path::new(&paths[0])), Some(1));

    let sink = Memory::default();
    let mut writer = JournalWriter::start(sink.clone(), &id, Arc::clone(&inventory), journal::limit_for(&inventory)).unwrap();
    let v3_opening = sink.0.borrow().len();
    let mut v2 = format!(r#"{{"header":{{"journal_version":2,"operation_id":"{id}"}}}}"#).into_bytes();
    v2.push(b'\n');
    // The inventory line is the same in both versions; serializing a
    // second copy of its 32 MB would only repeat the writer's work.
    {
        let v3 = sink.0.borrow();
        let header_end = v3.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        assert!(v3[header_end..v3_opening].starts_with(br#"{"inventory":{"root":"#));
        v2.extend_from_slice(&v3[header_end..v3_opening]);
    }
    let v2_opening = v2.len();

    for (index, chunk) in paths.chunks(MAX_BATCH_ENTRIES).enumerate() {
        let first = index * MAX_BATCH_ENTRIES;
        let states: Vec<ResultState> = (first..first + chunk.len()).map(result_of).collect();
        let batch = writer.begin_batch(chunk.iter().map(|path| file(path)).collect()).unwrap();
        assert_eq!(batch, batch_of(first));
        writer.finish_batch(&states).unwrap();

        // Frozen: version 2 batch lines as an older `wt` wrote them.
        let planned: Vec<String> = chunk.iter().map(|path| format!(r#"{{"path":{{"display":"{path}"}},"kind":"file"}}"#)).collect();
        let results: Vec<String> = chunk
            .iter()
            .zip(&states)
            .map(|(path, state)| {
                let state = match state {
                    ResultState::Deleted => "deleted",
                    ResultState::ExternallyAbsent => "externally_absent",
                    ResultState::Kept => "kept",
                };
                format!(r#"{{"path":{{"display":"{path}"}},"state":"{state}"}}"#)
            })
            .collect();
        v2.extend_from_slice(format!(r#"{{"intent":{{"batch":{batch},"entries":[{}]}}}}"#, planned.join(",")).as_bytes());
        v2.push(b'\n');
        v2.extend_from_slice(format!(r#"{{"results":{{"batch":{batch},"entries":[{}]}}}}"#, results.join(",")).as_bytes());
        v2.push(b'\n');
    }
    drop(writer);
    let v3 = sink.0.take();
    Journals { id, v3, v3_opening, v2, v2_opening }
}

/// What both versions of [`journals`] must read back as: every file in
/// order, in full batches, each with its result. Each test reads one
/// version against it, so the two versions read back alike without one test
/// parsing the 32 MB inventory twice.
fn assert_reads_back_as_planned(bytes: &[u8], id: &OperationId) {
    let log = journal::parse(bytes, id).unwrap();
    let paths = paths();
    assert_eq!(log.batches.len(), FILES.div_ceil(MAX_BATCH_ENTRIES));
    let expected = paths.chunks(MAX_BATCH_ENTRIES).enumerate().map(|(index, chunk)| {
        let first = index * MAX_BATCH_ENTRIES;
        journal::Batch {
            number: batch_of(first),
            entries: chunk.iter().map(|path| file(path)).collect(),
            results: Some((first..first + chunk.len()).map(result_of).collect()),
        }
    });
    assert!(log.batches.iter().cloned().eq(expected), "every batch reads back as planned");
    assert!(!log.torn_tail);
    let replayed = log.replay(|_| unreachable!("every batch has results")).unwrap();
    assert_eq!(replayed.len(), FILES);
    let states_match = replayed.iter().enumerate().all(|(n, entry)| {
        entry.path.as_path() == Path::new(&paths[n]) && entry.batch == batch_of(n) && entry.state == state_of_result(n)
    });
    assert!(states_match, "every entry replays with its result");
}

/// The state a replayed file `n` has: its durable result, never uncertain.
fn state_of_result(n: usize) -> EntryState {
    match result_of(n) {
        ResultState::Deleted => EntryState::Deleted,
        ResultState::ExternallyAbsent => EntryState::ExternallyAbsent,
        ResultState::Kept => EntryState::Kept,
    }
}

/// The byte pairs where `new` and `old` differ, position by position.
/// Equal 4 KiB blocks are skipped with one slice comparison: stepping
/// through the 32 MB openings byte by byte takes a quarter second in a
/// debug build.
fn differences<'a>(new: &'a [u8], old: &'a [u8]) -> impl Iterator<Item = (u8, u8)> + 'a {
    new.chunks(4096)
        .zip(old.chunks(4096))
        .filter(|(new, old)| new != old)
        .flat_map(|(new, old)| new.iter().zip(old).filter(|(new, old)| new != old).map(|(&new, &old)| (new, old)))
}

/// The openings of both versions differ only in the version digit, and the
/// version 3 batches take fewer bytes; the frozen version 2 journal reads
/// back as planned.
#[test]
fn version_3_batches_of_100k_entries_are_smaller_than_version_2s() {
    let Journals { id, v3, v3_opening, v2, v2_opening } = journals();

    // At most two, so a failure never prints megabytes.
    let opening_differences: Vec<(u8, u8)> = differences(&v3[..v3_opening], &v2[..v2_opening]).take(2).collect();
    let v3_batches = (v3.len() - v3_opening) as u64;
    let v2_batches = (v2.len() - v2_opening) as u64;
    let measured = format!(
        "opening: version 3 {v3_opening} bytes, version 2 {v2_opening} bytes; batches: version 3 {v3_batches} bytes, version 2 {v2_batches} bytes ({:.3}%)",
        v3_batches as f64 * 100.0 / v2_batches as f64,
    );
    eprintln!("{measured}");
    assert_eq!(v3_opening, v2_opening, "{measured}");
    assert_eq!(opening_differences, [(b'3', b'2')], "only the version differs");
    assert!(v3_batches < v2_batches, "{measured}");

    assert_reads_back_as_planned(&v2, &id);
}

/// The version 3 journal of the same batches reads back exactly as the
/// version 2 one does.
#[test]
fn a_version_3_journal_of_100k_entries_reads_back_as_planned() {
    let Journals { id, v3, .. } = journals();
    assert_reads_back_as_planned(&v3, &id);
}
