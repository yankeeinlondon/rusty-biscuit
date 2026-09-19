//! Replacement behavior while a reader holds a fixed artifact open. Runs on
//! every OS; prints `EVIDENCE` lines recording what the host actually did.

mod common;

use std::fs::File;
use std::io::Read;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::{Seen, classify, new, old};
use publication_spike::fsutil::{RETRIES, RetryPolicy};
use publication_spike::journal::{Options, Recovery, publish, recover};
use publication_spike::model::read_verified;
use publication_spike::{SpikeError, resolve};

const CATALOG: &str = "messenger/docs/research/platforms/catalog.json";

fn short_retry() -> Options {
    Options { retry: RetryPolicy { attempts: 5, delay: Duration::from_millis(20) }, ..Default::default() }
}

/// Rust std's default Windows share mode (read | write | delete).
const SHARE_STD_DEFAULT: u32 = 0x7;
/// A foreign reader without FILE_SHARE_DELETE (CRT `_wopen`, many editors).
const SHARE_NO_DELETE: u32 = 0x3;

fn open_reader(path: &std::path::Path, share: u32) -> File {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().read(true).share_mode(share).open(path).unwrap()
    }
    #[cfg(not(windows))]
    {
        let _ = share;
        File::open(path).unwrap()
    }
}

fn held_reader_case(share: u32) -> String {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    publish(root, &old(), Options::default()).unwrap();
    let mut held = open_reader(&resolve(root, CATALOG), share);
    let result = publish(root, &new(), short_retry());
    let observed = match &result {
        Ok(_) => {
            let mut buf = Vec::new();
            held.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, old().artifacts[CATALOG], "held handle keeps reading the old bytes");
            assert_eq!(classify(read_verified(root)), Seen::New);
            "publish=Ok; held handle still reads old bytes; verified read=New".to_string()
        }
        Err(SpikeError::Io { context, source }) => {
            let code = source.raw_os_error();
            assert!(matches!(code, Some(5) | Some(32)), "unexpected error {source:?}");
            let pre = classify(read_verified(root));
            assert!(matches!(pre, Seen::Refused(_)), "partial replacement must be refused: {pre:?}");
            drop(held);
            assert_eq!(recover(root, Options::default()).unwrap(), Recovery::RolledBack);
            assert_eq!(classify(read_verified(root)), Seen::Old);
            publish(root, &new(), Options::default()).unwrap();
            format!(
                "publish=Err({context}: raw_os_error={code:?} kind={:?}); verified read=Refused; \
                 recover=RolledBack -> Old; republish after handle closed=Ok",
                source.kind()
            )
        }
        Err(other) => panic!("unexpected {other}"),
    };
    format!("share_mode={share:#x}: {observed}")
}

#[test]
fn replace_while_reader_holds_std_default_handle() {
    eprintln!("EVIDENCE open-handle {}: {}", std::env::consts::OS, held_reader_case(SHARE_STD_DEFAULT));
}

#[test]
fn replace_while_reader_holds_no_delete_share_handle() {
    eprintln!("EVIDENCE open-handle {}: {}", std::env::consts::OS, held_reader_case(SHARE_NO_DELETE));
}

/// Compare the two rename paths available to production helpers:
/// `std::fs::rename` (worktree `atomic_write`, this spike) and
/// `tempfile::NamedTempFile::persist` (claudine `atomic_write`, darkmatter).
#[test]
fn rename_primitives_over_held_destination() {
    let mut rows = Vec::new();
    for share in [SHARE_STD_DEFAULT, SHARE_NO_DELETE] {
        for method in ["std::fs::rename", "tempfile::persist"] {
            let dir = tempfile::tempdir().unwrap();
            let dest = dir.path().join("catalog.json");
            std::fs::write(&dest, b"old").unwrap();
            let mut held = open_reader(&dest, share);
            let result = if method == "std::fs::rename" {
                let src = dir.path().join("src.tmp");
                std::fs::write(&src, b"new").unwrap();
                std::fs::rename(&src, &dest)
            } else {
                let mut tmp = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
                std::io::Write::write_all(&mut tmp, b"new").unwrap();
                tmp.persist(&dest).map(|_| ()).map_err(|e| e.error)
            };
            let mut via_held = Vec::new();
            held.read_to_end(&mut via_held).unwrap();
            drop(held);
            let on_disk = std::fs::read(&dest).unwrap();
            rows.push(format!(
                "share={share:#x} {method:<17} result={:?} held_reads={:?} path_now={:?}",
                result.map_err(|e| (e.raw_os_error(), e.kind())),
                String::from_utf8_lossy(&via_held),
                String::from_utf8_lossy(&on_disk)
            ));
        }
    }
    eprintln!("EVIDENCE rename-primitives {}\n{}", std::env::consts::OS, rows.join("\n"));
}

#[test]
fn handle_released_during_retry_window_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    publish(root, &old(), Options::default()).unwrap();
    let held = open_reader(&resolve(root, CATALOG), SHARE_NO_DELETE);
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        drop(held);
    });
    let before = RETRIES.load(Ordering::Relaxed);
    let started = Instant::now();
    publish(root, &new(), Options::default()).unwrap();
    releaser.join().unwrap();
    assert_eq!(classify(read_verified(root)), Seen::New);
    eprintln!(
        "EVIDENCE retry-window {}: publish=Ok after {:?}, transient retries={}",
        std::env::consts::OS,
        started.elapsed(),
        RETRIES.load(Ordering::Relaxed) - before
    );
}

/// Informs strategy B: can a directory holding an open file be renamed?
#[test]
fn rename_directory_containing_open_file() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("gen-a");
    std::fs::create_dir(&a).unwrap();
    std::fs::write(a.join("f.md"), b"x").unwrap();
    let held = File::open(a.join("f.md")).unwrap();
    let result = std::fs::rename(&a, dir.path().join("gen-b"));
    let removal = std::fs::remove_dir_all(if result.is_ok() { dir.path().join("gen-b") } else { a.clone() });
    drop(held);
    eprintln!(
        "EVIDENCE dir-rename {}: rename={:?} remove_dir_all_while_open={:?}",
        std::env::consts::OS,
        result.map_err(|e| (e.raw_os_error(), e.kind())),
        removal.map_err(|e| (e.raw_os_error(), e.kind()))
    );
}

/// Concurrent readers never accept a mixture while a writer alternates snapshots.
#[test]
fn concurrent_readers_see_only_complete_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    publish(&root, &old(), Options::default()).unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let readers: Vec<_> = (0..2)
        .map(|_| {
            let root = root.clone();
            let stop = stop.clone();
            std::thread::spawn(move || {
                let (mut complete, mut refused) = (0u32, 0u32);
                while !stop.load(Ordering::Relaxed) {
                    match classify(read_verified(&root)) {
                        Seen::Old | Seen::New => complete += 1,
                        Seen::Refused(_) => refused += 1,
                        other => panic!("reader accepted {other:?}"),
                    }
                }
                (complete, refused)
            })
        })
        .collect();
    // A foreign reader (no FILE_SHARE_DELETE) repeatedly opening one artifact.
    let foreign = {
        let path = resolve(&root, CATALOG);
        let stop = stop.clone();
        std::thread::spawn(move || {
            let (mut opened, mut failed) = (0u32, 0u32);
            while !stop.load(Ordering::Relaxed) {
                #[cfg(windows)]
                let attempt = {
                    use std::os::windows::fs::OpenOptionsExt;
                    std::fs::OpenOptions::new().read(true).share_mode(SHARE_NO_DELETE).open(&path)
                };
                #[cfg(not(windows))]
                let attempt = File::open(&path);
                match attempt {
                    Ok(mut f) => {
                        let mut buf = Vec::new();
                        let _ = f.read_to_end(&mut buf);
                        opened += 1;
                    }
                    Err(_) => failed += 1,
                }
            }
            (opened, failed)
        })
    };
    let retries_before = RETRIES.load(Ordering::Relaxed);
    let (mut ok, mut writer_errors, mut recoveries) = (0u32, 0u32, 0u32);
    for i in 0..60 {
        let snapshot = if i % 2 == 0 { new() } else { old() };
        match publish(&root, &snapshot, Options::default()) {
            Ok(_) => ok += 1,
            Err(_) => {
                writer_errors += 1;
                while recover(&root, Options::default()).is_err() {
                    std::thread::sleep(Duration::from_millis(10));
                }
                recoveries += 1;
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    let totals: Vec<(u32, u32)> = readers.into_iter().map(|r| r.join().unwrap()).collect();
    let foreign = foreign.join().unwrap();
    assert!(matches!(classify(read_verified(&root)), Seen::Old | Seen::New));
    eprintln!(
        "EVIDENCE concurrency {}: publishes ok={ok} failed={writer_errors} recoveries={recoveries}; \
         reader (complete, refused)={totals:?}; foreign no-delete-share reader (opened, failed)={foreign:?}; \
         transient retries during test={}",
        std::env::consts::OS,
        RETRIES.load(Ordering::Relaxed) - retries_before
    );
}
