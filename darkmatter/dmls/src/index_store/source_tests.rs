//! The R12 failure ladder, driven through a fault-injecting backend and real
//! store files.

use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use tracing_test::traced_test;

use super::*;
use crate::clock::SystemClock;
use crate::graph::index_document;
use crate::index_store::location::sibling;

/// A backend that answers through `inner` for its first `fail_from` calls and
/// fails every call after that with `failure`.
#[derive(Debug)]
struct FaultyFacts {
    inner: Box<dyn FactsSource>,
    calls: Arc<AtomicUsize>,
    fail_from: usize,
    failure: StoreFailure,
}

impl FaultyFacts {
    fn new(inner: impl FactsSource + 'static, fail_from: usize, failure: StoreFailure) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (Self { inner: Box::new(inner), calls: calls.clone(), fail_from, failure }, calls)
    }

    fn gate(&self) -> Result<(), StoreFailure> {
        if self.calls.fetch_add(1, Ordering::SeqCst) >= self.fail_from { Err(self.failure.clone()) } else { Ok(()) }
    }
}

impl FactsSource for FaultyFacts {
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

fn facts(path: &str, source: &str) -> FileFacts {
    FileFacts::from_index(&index_document(Path::new(path), source))
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}

/// A temporary root and store directory, both canonical.
fn fixture() -> (tempfile::TempDir, PathBuf, StoreLocation) {
    let dir = tempfile::tempdir().unwrap();
    let base = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let root = base.join("ws");
    std::fs::create_dir(&root).unwrap();
    let location = StoreLocation::prepare(Some(base.join("cache").as_os_str()), None);
    (dir, root, location)
}

fn store_dir(location: &StoreLocation) -> &Path {
    match location {
        StoreLocation::Directory(dir) => dir,
        other => panic!("expected a directory, got {other:?}"),
    }
}

fn count_warns(lines: &[&str], message: &str) -> usize {
    lines.iter().filter(|line| line.contains(" WARN ") && line.contains(message)).count()
}

#[traced_test]
#[test]
fn a_write_error_mid_fill_keeps_salvaged_rows_and_the_facts_being_written() {
    let mut memory = MemoryFacts::default();
    memory.put(&[facts("/ws/a.md", "[b](b.md)\n")]);
    // Call 0 (the first upsert) succeeds; every later call fails, the
    // salvage reads after the failing upsert included.
    let (faulty, _calls) = FaultyFacts::new(memory, 1, StoreFailure::Unavailable("disk full".into()));
    let mut root = RootFacts::with_source(Box::new(faulty), None, "test");
    root.upsert(&[facts("/ws/c.md", "[b](b.md)\n")]);
    assert!(!root.in_memory());
    root.upsert(&[facts("/ws/d.md", "[[b]]\n")]);
    assert!(root.in_memory());
    assert!(root.take_switched());
    assert!(!root.take_switched(), "the switch is reported once");
    // Nothing could be salvaged, so memory holds exactly the facts being
    // written when the store failed.
    assert_eq!(root.hashes().into_keys().collect::<Vec<_>>(), vec![PathBuf::from("/ws/d.md")]);
    assert_eq!(root.candidates_for(Path::new("/ws/b.md")), vec![PathBuf::from("/ws/d.md")]);
    logs_assert(|lines| match count_warns(lines, "index store unavailable, using memory") {
        1 => Ok(()),
        n => Err(format!("{n} fallback warns")),
    });
}

#[traced_test]
#[test]
fn a_readable_store_that_fails_a_write_is_salvaged_into_memory() {
    let (_dir, root_dir, location) = fixture();
    let path = store_path(store_dir(&location), &root_dir);
    let (mut store, _) = StoreFacts::open(&path, &root_dir, clock()).unwrap();
    store.upsert(&[facts("/ws/a.md", "[b](b.md)\n"), facts("/ws/c.md", "# C\n")]).unwrap();
    // Only the upsert fails: reads keep answering.
    #[derive(Debug)]
    struct WriteFails(StoreFacts);
    impl FactsSource for WriteFails {
        fn hashes(&mut self) -> Result<HashMap<PathBuf, u64>, StoreFailure> {
            self.0.hashes()
        }
        fn load(&mut self, paths: &[PathBuf]) -> Result<Vec<FileFacts>, StoreFailure> {
            self.0.load(paths)
        }
        fn upsert(&mut self, _: &[FileFacts]) -> Result<UpsertReport, StoreFailure> {
            Err(StoreFailure::Unavailable("disk full".into()))
        }
        fn remove_missing(&mut self) -> Result<usize, StoreFailure> {
            self.0.remove_missing()
        }
        fn remove_paths(&mut self, paths: &[PathBuf]) -> Result<usize, StoreFailure> {
            self.0.remove_paths(paths)
        }
        fn candidates_for(&mut self, target: &Path) -> Result<Vec<PathBuf>, StoreFailure> {
            self.0.candidates_for(target)
        }
        fn scan_headings(&mut self) -> Result<Vec<(PathBuf, Vec<HeadingFact>)>, StoreFailure> {
            self.0.scan_headings()
        }
    }
    let mut root = RootFacts::with_source(Box::new(WriteFails(store)), Some(path), "test");
    let report = root.upsert(&[facts("/ws/d.md", "[[b]]\n")]);
    assert_eq!(report.written, 1);
    assert!(root.in_memory());
    let mut held: Vec<_> = root.hashes().into_keys().collect();
    held.sort();
    assert_eq!(held, ["/ws/a.md", "/ws/c.md", "/ws/d.md"].map(PathBuf::from));
    assert_eq!(
        root.candidates_for(Path::new("/ws/b.md")),
        ["/ws/a.md", "/ws/d.md"].map(PathBuf::from)
    );
    assert_eq!(root.load(&[PathBuf::from("/ws/c.md")])[0].headings[0].title, "C");
}

#[traced_test]
#[test]
fn memory_persists_for_later_requests_and_warns_once() {
    let (faulty, calls) = FaultyFacts::new(MemoryFacts::default(), 0, StoreFailure::Corrupt("bad row".into()));
    let mut root = RootFacts::with_source(Box::new(faulty), None, "test");
    assert!(root.candidates_for(Path::new("/ws/b.md")).is_empty());
    let after_switch = calls.load(Ordering::SeqCst);
    root.upsert(&[facts("/ws/a.md", "[b](b.md)\n")]);
    root.load(&[PathBuf::from("/ws/a.md")]);
    root.scan_headings();
    assert_eq!(root.candidates_for(Path::new("/ws/b.md")), vec![PathBuf::from("/ws/a.md")]);
    assert_eq!(calls.load(Ordering::SeqCst), after_switch, "the failed backend is never asked again");
    // A second root failing in the same process does not warn again.
    let (faulty, _) = FaultyFacts::new(MemoryFacts::default(), 0, StoreFailure::Busy("locked".into()));
    let mut other = RootFacts::with_source(Box::new(faulty), None, "test");
    other.hashes();
    assert!(other.in_memory());
    logs_assert(|lines| match count_warns(lines, "index store unavailable, using memory") {
        1 => Ok(()),
        n => Err(format!("{n} fallback warns")),
    });
}

#[traced_test]
#[test]
fn a_corrupt_store_is_recreated_once() {
    let (_dir, root, location) = fixture();
    let path = store_path(store_dir(&location), &root);
    std::fs::write(&path, b"this is not a database, just garbage bytes long enough").unwrap();
    let (mut facts_source, opened) = RootFacts::open(&root, &location, clock(), "test");
    let RootOpened::Store(StoreOpened { report, recreated: Some(StoreFailure::Corrupt(_)) }) = opened else {
        panic!("expected a recreated store, got {opened:?}");
    };
    assert!(report.created);
    assert!(!facts_source.in_memory());
    facts_source.upsert(&[facts("/ws/a.md", "# A\n")]);
    assert_eq!(facts_source.hashes().len(), 1);
    assert!(logs_contain("index store recreated"));
    assert!(!logs_contain("using memory"));
}

#[traced_test]
#[test]
fn a_corrupt_store_another_process_holds_falls_back_to_memory() {
    let (_dir, root, location) = fixture();
    let path = store_path(store_dir(&location), &root);
    let garbage = b"this is not a database, just garbage bytes long enough";
    std::fs::write(&path, garbage).unwrap();
    let holder = StoreLock::shared(&lock_path(&path), Duration::ZERO).unwrap();
    let (facts_source, opened) = RootFacts::open(&root, &location, clock(), "test");
    assert!(matches!(opened, RootOpened::Memory(_)), "{opened:?}");
    assert!(facts_source.in_memory());
    assert_eq!(std::fs::read(&path).unwrap(), garbage, "the held store is never deleted");
    drop(holder);
    assert!(logs_contain("reason=in-use"));
    assert!(!logs_contain("index store recreated"));
}

#[traced_test]
#[test]
fn a_store_recorded_for_another_root_is_refilled() {
    let (_dir, root, location) = fixture();
    let path = store_path(store_dir(&location), &root);
    let other_root = root.parent().unwrap().join("elsewhere");
    {
        let (mut store, _) = IndexStore::open(&path, &other_root, clock()).unwrap();
        store.upsert_batch(&[facts("/elsewhere/a.md", "# A\n")]).unwrap();
    }
    let (mut facts_source, opened) = RootFacts::open(&root, &location, clock(), "test");
    let RootOpened::Store(StoreOpened { recreated: Some(StoreFailure::RootMismatch { stored }), .. }) = opened else {
        panic!("expected a refill, got {opened:?}");
    };
    assert_eq!(stored, other_root.to_str().unwrap());
    assert!(facts_source.hashes().is_empty(), "the other root's rows are gone");
}

#[traced_test]
#[test]
fn a_wal_switch_busy_past_its_bound_uses_memory_and_never_recreates() {
    let (_dir, root, location) = fixture();
    let path = store_path(store_dir(&location), &root);
    let holder = rusqlite::Connection::open(&path).unwrap();
    holder.execute_batch("CREATE TABLE held (x INTEGER); BEGIN EXCLUSIVE; INSERT INTO held VALUES (1);").unwrap();
    let settings = OpenSettings { wal_retry: Duration::from_millis(50), busy_timeout: Duration::ZERO, ..OpenSettings::default() };
    let (facts_source, opened) = RootFacts::open_with(&root, &location, clock(), settings, "test");
    let RootOpened::Memory(detail) = opened else { panic!("expected memory, got {opened:?}") };
    assert!(detail.contains("WAL switch still busy"), "{detail}");
    assert!(facts_source.in_memory());
    holder.execute_batch("COMMIT").unwrap();
    let rows: i64 = holder.query_row("SELECT count(*) FROM held", [], |row| row.get(0)).unwrap();
    assert_eq!(rows, 1, "the holder's database was not recreated");
    assert!(logs_contain("reason=busy"));
    assert!(!logs_contain("index store recreated"));
}

#[traced_test]
#[test]
fn disabled_location_uses_memory_and_creates_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let location = StoreLocation::prepare(Some(std::ffi::OsStr::new("OFF")), Some(dir.path().to_path_buf()));
    assert_eq!(location, StoreLocation::Disabled);
    let (mut facts_source, opened) = RootFacts::open(dir.path(), &location, clock(), "test");
    assert!(matches!(opened, RootOpened::Memory(_)));
    facts_source.upsert(&[facts("/ws/a.md", "# A\n")]);
    assert_eq!(facts_source.scan_headings().len(), 1);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0, "no store file or directory");
    assert!(logs_contain("reason=disabled"));
}

#[test]
fn removing_a_store_takes_every_sibling_file() {
    let (_dir, root, location) = fixture();
    let path = store_path(store_dir(&location), &root);
    drop(IndexStore::open(&path, &root, clock()).unwrap());
    assert!(lock_path(&path).exists());
    assert!(crate::index_store::location::remove_store(&path).unwrap());
    for suffix in ["", "-wal", "-shm", ".lock"] {
        assert!(!sibling(&path, suffix).exists(), "{suffix} left behind");
    }
}
