//! R11: several writers share one store. Interleaved batches converge, a row
//! another writer left for different bytes is replaced, and two processes
//! that create and fill one store at once both end with identical rows
//! (acceptance 9).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dmls::clock::SystemClock;
use dmls::graph::index_document;
use dmls::index_store::{FileFacts, IndexStore, UpsertReport, store_path};

const WORKER_STORE: &str = "DMLS_COLD_FILL_STORE";
const WORKER_ROOT: &str = "DMLS_COLD_FILL_ROOT";
const WORKER_GO: &str = "DMLS_COLD_FILL_GO";

/// `count` Markdown files under `root`, each linking its neighbor.
fn write_files(root: &Path, count: usize) -> Vec<PathBuf> {
    (0..count)
        .map(|n| {
            let path = root.join(format!("d{}/f{n}.md", n % 7));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("# File {n}\n\n[next](../d{}/f{}.md#file-{})\n[[f{}]]\n", (n + 1) % 7, n + 1, n + 1, n + 2)).unwrap();
            path
        })
        .collect()
}

fn listed(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in std::fs::read_dir(root).unwrap() {
        let dir = dir.unwrap().path();
        if dir.is_dir() {
            out.extend(std::fs::read_dir(&dir).unwrap().map(|entry| entry.unwrap().path()));
        }
    }
    out.sort();
    out
}

fn extract(paths: &[PathBuf]) -> Vec<FileFacts> {
    paths.iter().map(|path| FileFacts::from_index(&index_document(path, &std::fs::read_to_string(path).unwrap()))).collect()
}

fn base() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let base = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    (dir, base)
}

#[test]
fn two_connections_writing_interleaved_batches_converge() {
    let (_dir, base) = base();
    let root = base.join("ws");
    let facts = extract(&write_files(&root, 120));
    let path = store_path(&base, &root);
    let (mut first, _) = IndexStore::open(&path, &root, Arc::new(SystemClock)).unwrap();
    let (mut second, _) = IndexStore::open(&path, &root, Arc::new(SystemClock)).unwrap();
    // Overlapping halves, alternating writers: the overlap is written once.
    let (front, back) = (&facts[..80], &facts[40..]);
    let mut written = 0;
    for step in 0..4 {
        let chunk = |half: &[FileFacts]| half.chunks(20).nth(step).map(<[FileFacts]>::to_vec).unwrap_or_default();
        written += first.upsert_batch(&chunk(front)).unwrap().written;
        written += second.upsert_batch(&chunk(back)).unwrap().written;
    }
    assert_eq!(written, facts.len(), "each file written exactly once");
    let paths: Vec<PathBuf> = facts.iter().map(|file| file.path.clone()).collect();
    assert_eq!(first.load(&paths).unwrap(), facts);
    assert_eq!(second.load(&paths).unwrap(), facts);
    assert_eq!(first.hashes().unwrap(), second.hashes().unwrap());
}

#[test]
fn a_row_another_writer_left_for_different_bytes_is_replaced() {
    let (_dir, base) = base();
    let root = base.join("ws");
    let file = root.join("a.md");
    std::fs::create_dir_all(&root).unwrap();
    let path = store_path(&base, &root);
    let (mut foreign, _) = IndexStore::open(&path, &root, Arc::new(SystemClock)).unwrap();
    foreign.upsert_batch(&[FileFacts::from_index(&index_document(&file, "# Old\n"))]).unwrap();

    std::fs::write(&file, "# New\n\n[b](b.md)\n").unwrap();
    let (mut own, _) = IndexStore::open(&path, &root, Arc::new(SystemClock)).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    let own_hash = biscuit_hash::xx_hash_bytes(&bytes);
    // The stored hash is not this process's read: the row is not usable and
    // the file is parsed again.
    assert_ne!(own.hashes().unwrap()[&file], own_hash);
    let reparsed = FileFacts::from_index(&index_document(&file, std::str::from_utf8(&bytes).unwrap()));
    assert_eq!(reparsed.hash, own_hash);
    assert_eq!(own.upsert_batch(std::slice::from_ref(&reparsed)).unwrap(), UpsertReport { written: 1, skipped: 0 });
    assert_eq!(foreign.load(std::slice::from_ref(&file)).unwrap(), vec![reparsed]);
    assert_eq!(foreign.candidates_for(&root.join("b.md")).unwrap(), vec![file]);
}

/// The worker half of the two-process cold fill; a no-op unless spawned by
/// [`two_processes_cold_filling_one_new_store_reach_identical_rows`].
#[test]
fn cold_fill_worker() {
    let (Some(store), Some(root), Some(go)) =
        (std::env::var_os(WORKER_STORE), std::env::var_os(WORKER_ROOT), std::env::var_os(WORKER_GO))
    else {
        return;
    };
    let (store, root, go) = (PathBuf::from(store), PathBuf::from(root), PathBuf::from(go));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !go.exists() {
        assert!(Instant::now() < deadline, "never told to start");
        std::thread::sleep(Duration::from_millis(1));
    }
    // Both processes create the new store at once.
    let (mut index, _) = IndexStore::open(&store, &root, Arc::new(SystemClock)).unwrap();
    let facts = extract(&listed(&root));
    // Small batches, so the two writers' transactions interleave.
    for chunk in facts.chunks(50) {
        index.upsert_batch(chunk).unwrap();
    }
    let paths: Vec<PathBuf> = facts.iter().map(|file| file.path.clone()).collect();
    assert_eq!(index.load(&paths).unwrap(), facts, "this process's view equals its own extraction");
}

#[test]
fn two_processes_cold_filling_one_new_store_reach_identical_rows() {
    let (_dir, base) = base();
    let root = base.join("ws");
    let paths = write_files(&root, 250);
    let store = store_path(&base, &root);
    let go = base.join("go");
    let workers: Vec<_> = (0..2)
        .map(|_| {
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "index_store_concurrency::cold_fill_worker", "--nocapture"])
                .env(WORKER_STORE, &store)
                .env(WORKER_ROOT, &root)
                .env(WORKER_GO, &go)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn cold-fill worker")
        })
        .collect();
    assert!(!store.exists(), "both workers create the store");
    std::fs::write(&go, b"go").unwrap();
    for worker in workers {
        let output = worker.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "worker failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"), "the worker ran its body");
    }
    let mut sorted = paths.clone();
    sorted.sort();
    let (index, report) = IndexStore::open(&store, &root, Arc::new(SystemClock)).unwrap();
    assert_eq!(report.rows, paths.len());
    assert_eq!(index.load(&sorted).unwrap(), extract(&sorted));
}
