//! Revalidation, ready, live refresh, the open-buffer overlay, and a store
//! failure after ready, over real store files. Parse and read counts come
//! from the process-wide work counters, which nextest's process-per-test
//! model keeps private to one test.

use std::collections::BTreeSet;

use super::*;
use crate::clock::SystemClock;
use crate::index_store::{FactsSource, StoreFacts, StoreFailure, UpsertReport, store_path};
use crate::graph::HeadingFact;
use crate::work_counters::{WorkCounts, work_counts};
use crate::workspace::startup::SilentProgress;

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    location: StoreLocation,
}

impl Fixture {
    /// A canonical workspace root and a store directory beside it.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let base = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let root = base.join("ws");
        std::fs::create_dir(&root).unwrap();
        let location = StoreLocation::prepare(Some(base.join("cache").as_os_str()), None);
        Self { _dir: dir, root, location }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, text).unwrap();
        path
    }

    /// A process start: the stores opened afresh.
    fn open(&self) -> WorkspaceFacts {
        WorkspaceFacts::open(std::slice::from_ref(&self.root), &self.location, Arc::new(SystemClock), "test")
    }

    fn store_dir(&self) -> &Path {
        match &self.location {
            StoreLocation::Directory(dir) => dir,
            other => panic!("expected a directory, got {other:?}"),
        }
    }
}

fn revalidate(facts: &mut WorkspaceFacts, files: &[PathBuf]) -> (PassReport, WorkCounts) {
    let before = work_counts();
    let report = facts.revalidate(files, &BTreeMap::new(), &SilentProgress, &AtomicBool::new(false));
    (report, work_counts().since(before))
}

fn listed(files: &[PathBuf]) -> BTreeSet<PathBuf> {
    files.iter().cloned().collect()
}

fn headings(documents: &BTreeMap<PathBuf, DocumentIndex>, path: &Path) -> Vec<String> {
    documents[path].headings.iter().map(|heading: &HeadingFact| heading.title.clone()).collect()
}

/// A workspace of `count` files, each linking the next.
fn chain(fixture: &Fixture, count: usize) -> Vec<PathBuf> {
    (0..count).map(|n| fixture.write(&format!("f{n}.md"), &format!("# F{n}\n\n[next](f{}.md)\n", n + 1))).collect()
}

/// Acceptance 1: a restart whose files are unchanged reads and hashes each
/// file once and parses none.
#[test]
fn a_restart_with_unchanged_files_parses_nothing() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 12);
    let (cold, work) = revalidate(&mut fixture.open(), &files);
    assert_eq!((cold.read, cold.reparsed, cold.unchanged, work.parses), (12, 12, 0, 12), "{cold:?}");

    let mut restarted = fixture.open();
    let (warm, work) = revalidate(&mut restarted, &files);
    assert_eq!((warm.read, warm.reparsed, warm.unchanged, warm.removed), (12, 0, 12, 0), "{warm:?}");
    assert_eq!((work.parses, work.reads, work.hashes), (0, 12, 12), "{work:?}");
    assert!(warm.failed.is_empty() && warm.missing.is_empty() && !warm.switched && !warm.cancelled);
}

/// Acceptance 2: one file changed between restarts is the only parse, and
/// its row now holds the new facts.
#[test]
fn one_file_changed_between_restarts_is_the_only_parse() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 6);
    revalidate(&mut fixture.open(), &files);
    std::fs::write(&files[2], "# Changed\n\n[back](f0.md)\n").unwrap();

    let mut restarted = fixture.open();
    let (report, work) = revalidate(&mut restarted, &files);
    assert_eq!((report.reparsed, report.unchanged, work.parses), (1, 5, 1), "{report:?}");
    let documents = restarted.request_documents(&files[2..3], []);
    assert_eq!(headings(&documents, &files[2]), vec!["Changed".to_string()]);
}

/// A file deleted between restarts loses its row even though the new file
/// list no longer names it; a listed file that is missing is reported.
#[test]
fn a_deleted_file_loses_its_row() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 3);
    revalidate(&mut fixture.open(), &files);
    std::fs::remove_file(&files[0]).unwrap();
    std::fs::remove_file(&files[1]).unwrap();

    let mut restarted = fixture.open();
    // `f0.md` is gone from the list too; `f1.md` is still listed (the list
    // was taken before the delete).
    let (report, _) = revalidate(&mut restarted, &files[1..]);
    assert_eq!(report.missing, vec![files[1].clone()]);
    assert_eq!(report.removed, 2, "{report:?}");
    let (store, _) = StoreFacts::open(&store_path(fixture.store_dir(), &fixture.root), &fixture.root, Arc::new(SystemClock)).unwrap();
    let rows: BTreeSet<PathBuf> = store.store().hashes().unwrap().into_keys().collect();
    assert_eq!(rows, listed(&files[2..]));
}

/// A listed file that exists but cannot be read as text fails the pass and
/// gets no row; a zero-byte file and a heading with an empty title are valid
/// rows.
#[test]
fn an_unreadable_file_fails_and_empty_files_are_valid_rows() {
    let fixture = Fixture::new();
    let empty = fixture.write("empty.md", "");
    let bare = fixture.write("bare.md", "#\n");
    let binary = fixture.root.join("binary.md");
    std::fs::write(&binary, [0xff, 0xfe, 0x00]).unwrap();
    let files = vec![bare.clone(), binary.clone(), empty.clone()];

    let mut facts = fixture.open();
    let (report, _) = revalidate(&mut facts, &files);
    assert_eq!(report.failed.iter().map(|(path, _)| path.clone()).collect::<Vec<_>>(), vec![binary.clone()]);
    assert_eq!(report.reparsed, 2, "{report:?}");
    assert!(facts.verified_hash(&binary).is_none(), "a failed read is not verified");
    facts.mark_ready();
    assert!(!facts.is_ready(&listed(&files), std::slice::from_ref(&fixture.root)), "an unread listed file withholds ready");

    let documents = facts.request_documents(&files, []);
    assert_eq!(documents.keys().cloned().collect::<Vec<_>>(), vec![bare.clone(), empty.clone()]);
    assert_eq!(headings(&documents, &bare), vec![String::new()]);
    assert!(documents[&empty].headings.is_empty());

    let (warm, work) = revalidate(&mut fixture.open(), &[bare, empty]);
    assert_eq!((warm.unchanged, work.parses), (2, 0), "empty rows are reused: {warm:?}");
}

/// A parse the caller already holds is used when its hash matches the bytes
/// read, and never when it does not.
#[test]
fn a_known_parse_is_reused_only_when_its_hash_matches() {
    let fixture = Fixture::new();
    let current = fixture.write("current.md", "# Current\n");
    let stale = fixture.write("stale.md", "# New\n");
    let known: BTreeMap<PathBuf, DocumentIndex> = [
        (current.clone(), index_document(&current, "# Current\n")),
        (stale.clone(), index_document(&stale, "# Old\n")),
    ]
    .into_iter()
    .collect();
    let mut facts = fixture.open();
    let before = work_counts();
    let report = facts.revalidate(&[current.clone(), stale.clone()], &known, &SilentProgress, &AtomicBool::new(false));
    assert_eq!((report.reparsed, work_counts().since(before).parses), (2, 1), "only the stale one parses");
    let documents = facts.request_documents(std::slice::from_ref(&stale), []);
    assert_eq!(headings(&documents, &stale), vec!["New".to_string()]);
}

#[test]
fn a_cancelled_revalidation_reads_nothing_and_writes_nothing() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 4);
    let mut facts = fixture.open();
    let before = work_counts();
    let report = facts.revalidate(&files, &BTreeMap::new(), &SilentProgress, &AtomicBool::new(true));
    assert!(report.cancelled);
    assert_eq!((report.read, work_counts().since(before).reads), (0, 0));
    assert!(facts.request_documents(&files, []).is_empty());
}

/// Ready needs the caller's mark, the same roots, and every listed file read
/// in this process.
#[test]
fn ready_needs_a_mark_the_same_roots_and_every_listed_file_read() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 3);
    let roots = std::slice::from_ref(&fixture.root);
    let mut facts = fixture.open();
    revalidate(&mut facts, &files);
    assert!(!facts.is_ready(&listed(&files), roots), "not before the mark");
    facts.mark_ready();
    assert!(facts.is_ready(&listed(&files), roots));
    assert!(!facts.is_ready(&listed(&files), &[fixture.root.join("other")]), "other roots");
    let joined = fixture.write("joined.md", "# Joined\n");
    let mut grown = listed(&files);
    grown.insert(joined.clone());
    assert!(!facts.is_ready(&grown, roots), "a file that joined the list is unread");
    facts.refresh(&[joined], &BTreeMap::new());
    assert!(facts.is_ready(&grown, roots), "a refresh reads it");
    facts.withdraw_ready();
    assert!(!facts.is_ready(&grown, roots));
}

/// R6, after ready: a refresh re-reads only the given paths, re-parses the
/// changed one, deletes the row of the deleted one, and keeps ready.
#[test]
fn a_refresh_updates_one_row_and_deletes_a_missing_one() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 8);
    let mut facts = fixture.open();
    revalidate(&mut facts, &files);
    facts.mark_ready();
    std::fs::write(&files[0], "# Edited\n").unwrap();
    std::fs::remove_file(&files[1]).unwrap();

    let before = work_counts();
    let report = facts.refresh(&files[..2], &BTreeMap::new());
    let work = work_counts().since(before);
    assert_eq!((report.read, report.reparsed, report.removed), (1, 1, 1), "{report:?}");
    assert_eq!((work.reads, work.parses), (2, 1), "{work:?}");
    let remaining = listed(&files[2..]).into_iter().chain([files[0].clone()]).collect();
    assert!(facts.is_ready(&remaining, std::slice::from_ref(&fixture.root)));
    let documents = facts.request_documents(&files[..2], []);
    assert_eq!(documents.keys().cloned().collect::<Vec<_>>(), vec![files[0].clone()]);
    assert_eq!(headings(&documents, &files[0]), vec!["Edited".to_string()]);
}

/// R5: every open buffer, unsaved or saved but differing from its row,
/// replaces its row for the request, and no buffer writes a row.
#[test]
fn open_buffers_replace_rows_and_never_write_one() {
    let fixture = Fixture::new();
    let saved = fixture.write("saved.md", "# Disk\n");
    let other = fixture.write("other.md", "# Other\n");
    let mut facts = fixture.open();
    revalidate(&mut facts, &[other.clone(), saved.clone()]);
    let row_hashes = |facts: &mut WorkspaceFacts| -> HashMap<PathBuf, u64> {
        facts.roots.iter_mut().flat_map(|(_, root)| root.hashes()).collect()
    };
    let before = row_hashes(&mut facts);

    // `saved.md` was saved with new text (no event yet); `unsaved.md` exists
    // only in a buffer.
    std::fs::write(&saved, "# Saved buffer\n").unwrap();
    let unsaved = fixture.root.join("unsaved.md");
    let buffers = [index_document(&saved, "# Saved buffer\n"), index_document(&unsaved, "# Unsaved\n")];
    let documents = facts.request_documents(&[other.clone(), saved.clone()], &buffers);
    assert_eq!(headings(&documents, &saved), vec!["Saved buffer".to_string()]);
    assert_eq!(headings(&documents, &unsaved), vec!["Unsaved".to_string()]);
    assert_eq!(headings(&documents, &other), vec!["Other".to_string()]);
    assert_eq!(row_hashes(&mut facts), before, "a buffer writes no row");
}

/// A file under two nested roots has a row in each store, and a restart
/// reuses both.
#[test]
fn a_file_under_nested_roots_has_a_row_in_each_store() {
    let fixture = Fixture::new();
    let nested = fixture.root.join("nested");
    let inner = fixture.write("nested/inner.md", "# Inner\n");
    let outer = fixture.write("outer.md", "# Outer\n");
    let roots = vec![fixture.root.clone(), nested.clone()];
    let open = || WorkspaceFacts::open(&roots, &fixture.location, Arc::new(SystemClock), "test");
    let mut facts = open();
    let files = [inner.clone(), outer.clone()];
    revalidate(&mut facts, &files);
    let rows: Vec<usize> = facts.roots.iter_mut().map(|(_, root)| root.hashes().len()).collect();
    assert_eq!(rows, vec![2, 1]);
    let (warm, work) = revalidate(&mut open(), &files);
    assert_eq!((warm.unchanged, work.parses), (2, 0), "{warm:?}");
}

/// A backend that fails every call once `broken` is set, as a store whose
/// disk failed after ready would.
#[derive(Debug)]
struct Breakable {
    inner: StoreFacts,
    broken: Arc<AtomicBool>,
}

impl Breakable {
    fn gate(&self) -> Result<(), StoreFailure> {
        if self.broken.load(Ordering::SeqCst) { Err(StoreFailure::Unavailable("disk failed".into())) } else { Ok(()) }
    }
}

impl FactsSource for Breakable {
    fn hashes(&mut self) -> Result<HashMap<PathBuf, u64>, StoreFailure> {
        self.gate()?;
        self.inner.hashes()
    }
    fn load(&mut self, paths: &[PathBuf]) -> Result<Vec<FileFacts>, StoreFailure> {
        self.gate()?;
        self.inner.load(paths)
    }
    fn upsert(&mut self, facts: &[FileFacts]) -> Result<UpsertReport, StoreFailure> {
        self.gate()?;
        self.inner.upsert(facts)
    }
    fn remove_missing(&mut self) -> Result<usize, StoreFailure> {
        self.gate()?;
        self.inner.remove_missing()
    }
    fn remove_paths(&mut self, paths: &[PathBuf]) -> Result<usize, StoreFailure> {
        self.gate()?;
        self.inner.remove_paths(paths)
    }
    fn candidates_for(&mut self, target: &Path) -> Result<Vec<PathBuf>, StoreFailure> {
        self.gate()?;
        self.inner.candidates_for(target)
    }
    fn scan_headings(&mut self) -> Result<Vec<(PathBuf, Vec<HeadingFact>)>, StoreFailure> {
        self.gate()?;
        self.inner.scan_headings()
    }
}

/// A store failure after ready switches the root to memory without failing
/// the refresh in flight: the facts it was writing are kept, ready is
/// withdrawn, and the next revalidation refills memory completely.
#[test]
fn a_store_failure_after_ready_switches_to_memory_without_losing_written_facts() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 5);
    let path = store_path(fixture.store_dir(), &fixture.root);
    let (store, _) = StoreFacts::open(&path, &fixture.root, Arc::new(SystemClock)).unwrap();
    let broken = Arc::new(AtomicBool::new(false));
    let backend = Breakable { inner: store, broken: Arc::clone(&broken) };
    let root = RootFacts::with_source(Box::new(backend), Some(path), "test");
    let mut facts = WorkspaceFacts::from_roots(vec![(fixture.root.clone(), root)]);
    let roots = std::slice::from_ref(&fixture.root);
    revalidate(&mut facts, &files);
    facts.mark_ready();
    assert!(facts.is_ready(&listed(&files), roots));

    broken.store(true, Ordering::SeqCst);
    std::fs::write(&files[3], "# Edited\n").unwrap();
    let report = facts.refresh(&files[3..4], &BTreeMap::new());
    assert!(report.switched, "{report:?}");
    assert!(!facts.is_ready(&listed(&files), roots), "memory holds only what was salvaged");
    // Nothing could be salvaged from the broken store, but the facts being
    // written survive.
    let documents = facts.request_documents(&files, []);
    assert_eq!(documents.keys().cloned().collect::<Vec<_>>(), vec![files[3].clone()]);
    assert_eq!(headings(&documents, &files[3]), vec!["Edited".to_string()]);

    let (refill, _) = revalidate(&mut facts, &files);
    assert!(!refill.switched && refill.failed.is_empty(), "{refill:?}");
    facts.mark_ready();
    assert!(facts.is_ready(&listed(&files), roots));
    assert_eq!(facts.request_documents(&files, []).len(), 5);
    assert!(facts.roots.iter().all(|(_, root)| root.in_memory()));
}

/// `DMLS_CACHE_DIR=off`: every root is memory, nothing is written to disk,
/// and revalidation still answers.
#[test]
fn disabled_stores_keep_facts_in_memory() {
    let fixture = Fixture::new();
    let files = chain(&fixture, 3);
    let mut facts = WorkspaceFacts::open(std::slice::from_ref(&fixture.root), &StoreLocation::Disabled, Arc::new(SystemClock), "test");
    let (report, _) = revalidate(&mut facts, &files);
    assert_eq!(report.reparsed, 3);
    assert_eq!(facts.request_documents(&files, []).len(), 3);
    assert!(!fixture.store_dir().exists() || std::fs::read_dir(fixture.store_dir()).unwrap().next().is_none());
}
