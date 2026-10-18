//! R10: the sweep deletes stores untouched for 30 days and keeps fresh ones,
//! ones whose root is missing, ones a recent batch touched, and ones another
//! holder has open.
//!
//! Time moves through the sweep's injected `now` and the store's injected
//! clock, never by sleeping.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dmls::clock::{Clock, ManualClock, SystemClock};
use dmls::graph::index_document;
use dmls::index_store::{FileFacts, IndexStore, STALE_STORE_AGE, SweepReport, store_path, sweep_stale_stores, touch_mtime};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const SUFFIXES: [&str; 4] = ["", "-wal", "-shm", ".lock"];

fn sibling(store: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", store.display()))
}

struct Fixture {
    _dir: tempfile::TempDir,
    dir: PathBuf,
    root: PathBuf,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let base = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    let dir = base.join("stores");
    let root = base.join("ws");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    Fixture { _dir: temp, dir, root }
}

/// A store for `root` with a row and all four sibling files, closed.
fn closed_store(dir: &Path, root: &Path) -> PathBuf {
    let path = store_path(dir, root);
    let (store, _) = open_store(&path, root, Arc::new(SystemClock));
    drop(store);
    // SQLite checkpoints and removes `-wal`/`-shm` on the last close; a
    // crashed process leaves them, so put them back.
    for suffix in ["-wal", "-shm"] {
        std::fs::write(sibling(&path, suffix), b"").unwrap();
    }
    path
}

fn open_store(path: &Path, root: &Path, clock: Arc<dyn Clock>) -> (IndexStore, PathBuf) {
    let (mut store, _) = IndexStore::open(path, root, clock).unwrap();
    store.upsert_batch(&[FileFacts::from_index(&index_document(&root.join("a.md"), "# A\n"))]).unwrap();
    (store, path.to_path_buf())
}

fn all_present(store: &Path) -> bool {
    SUFFIXES.iter().all(|suffix| sibling(store, suffix).exists())
}

fn none_present(store: &Path) -> bool {
    SUFFIXES.iter().all(|suffix| !sibling(store, suffix).exists())
}

#[test]
fn a_store_untouched_for_thirty_days_is_deleted_and_a_fresh_one_kept() {
    let fixture = fixture();
    let store = closed_store(&fixture.dir, &fixture.root);
    let unrelated = fixture.dir.join("notes.txt");
    std::fs::write(&unrelated, "keep me").unwrap();
    let now = SystemTime::now();

    let report = sweep_stale_stores(&fixture.dir, STALE_STORE_AGE, now + 29 * DAY).unwrap();
    assert_eq!(report, SweepReport::default());
    assert!(all_present(&store));

    let report = sweep_stale_stores(&fixture.dir, STALE_STORE_AGE, now + 31 * DAY).unwrap();
    assert_eq!(report, SweepReport { removed: 1, skipped_in_use: 0 });
    assert!(none_present(&store), "every sibling, the lock file included, is deleted");
    assert!(unrelated.exists(), "only store files are swept");
}

#[test]
fn the_newest_sibling_decides_a_stores_age() {
    let fixture = fixture();
    let store = closed_store(&fixture.dir, &fixture.root);
    let now = SystemTime::now();
    for suffix in SUFFIXES {
        touch_mtime(&sibling(&store, suffix), now - 40 * DAY).unwrap();
    }
    touch_mtime(&sibling(&store, "-wal"), now - DAY).unwrap();
    assert_eq!(sweep_stale_stores(&fixture.dir, STALE_STORE_AGE, now).unwrap().removed, 0);
    assert!(all_present(&store));
}

#[test]
fn a_store_whose_root_is_missing_is_kept_while_fresh() {
    let fixture = fixture();
    let store = closed_store(&fixture.dir, &fixture.root);
    std::fs::remove_dir_all(&fixture.root).unwrap();
    let report = sweep_stale_stores(&fixture.dir, STALE_STORE_AGE, SystemTime::now() + DAY).unwrap();
    assert_eq!(report, SweepReport::default());
    assert!(all_present(&store));
}

fn age(path: &Path, now: SystemTime) -> Duration {
    now.duration_since(std::fs::metadata(path).unwrap().modified().unwrap()).unwrap_or_default()
}

#[test]
fn a_store_touched_by_a_recent_batch_or_read_is_kept() {
    let fixture = fixture();
    let clock = Arc::new(ManualClock::new());
    let path = store_path(&fixture.dir, &fixture.root);
    let (mut store, _) = open_store(&path, &fixture.root, clock.clone());
    let now = SystemTime::now();
    let age_everything = || {
        for suffix in SUFFIXES {
            touch_mtime(&sibling(&path, suffix), now - 40 * DAY).unwrap();
        }
    };
    let batch = |store: &mut IndexStore, name: &str| {
        let file = fixture.root.join(name);
        store.upsert_batch(&[FileFacts::from_index(&index_document(&file, "# B\n"))]).unwrap();
    };

    // The database file's own mtime: WAL writes refresh only `-wal`.
    age_everything();
    batch(&mut store, "b.md");
    assert!(age(&path, now) >= STALE_STORE_AGE, "within the touch interval a batch does not touch");
    clock.advance(Duration::from_secs(60 * 60));
    batch(&mut store, "c.md");
    assert!(age(&path, now) < DAY, "a batch after the interval touches");

    age_everything();
    clock.advance(Duration::from_secs(60 * 60));
    store.hashes().unwrap();
    assert!(age(&path, now) < DAY, "a read after the interval touches");

    drop(store);
    assert_eq!(sweep_stale_stores(&fixture.dir, STALE_STORE_AGE, now).unwrap(), SweepReport::default());
    assert!(path.exists());
}

/// Unix unlinks an open file, so the lock is the only protection: with the
/// holder's store aged past 30 days, nothing of it is unlinked, and its later
/// writes reach a fresh connection at the same path.
#[test]
fn a_store_another_holder_has_open_is_skipped_and_stays_shared() {
    let fixture = fixture();
    let path = store_path(&fixture.dir, &fixture.root);
    let (mut holder, _) = open_store(&path, &fixture.root, Arc::new(SystemClock));
    let present: Vec<&str> = SUFFIXES.iter().copied().filter(|suffix| sibling(&path, suffix).exists()).collect();
    assert!(present.contains(&"") && present.contains(&".lock"), "{present:?}");

    let report = sweep_stale_stores(&fixture.dir, STALE_STORE_AGE, SystemTime::now() + 31 * DAY).unwrap();
    assert_eq!(report, SweepReport { removed: 0, skipped_in_use: 1 });
    for suffix in &present {
        assert!(sibling(&path, suffix).exists(), "{suffix} was deleted under its holder");
    }

    let later = fixture.root.join("later.md");
    holder.upsert_batch(&[FileFacts::from_index(&index_document(&later, "# Later\n"))]).unwrap();
    let (fresh, _) = IndexStore::open(&path, &fixture.root, Arc::new(SystemClock)).unwrap();
    assert!(fresh.hashes().unwrap().contains_key(&later), "the holder and a new opener share one store");
}

/// Windows: a store whose database another process has open fails to delete
/// with os error 32 even when nobody holds its lock file, and is skipped; the
/// mtime touch through a write handle succeeds on the open file.
#[cfg(windows)]
#[test]
fn windows_skips_a_store_whose_database_is_open_without_its_lock() {
    let fixture = fixture();
    let store = closed_store(&fixture.dir, &fixture.root);
    let raw = rusqlite::Connection::open(&store).unwrap();
    raw.query_row("SELECT count(*) FROM files", [], |row| row.get::<_, i64>(0)).unwrap();
    touch_mtime(&store, SystemTime::now()).unwrap();
    let report = sweep_stale_stores(&fixture.dir, STALE_STORE_AGE, SystemTime::now() + 31 * DAY).unwrap();
    assert_eq!(report, SweepReport { removed: 0, skipped_in_use: 1 });
    assert!(store.exists());
    drop(raw);
}
