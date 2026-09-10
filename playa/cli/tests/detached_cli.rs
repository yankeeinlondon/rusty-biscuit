use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use fs4::fs_std::FileExt as _;
use test_toolkit::LockedAudioSpool;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "playa-cli-phase3-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos()
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(&path).unwrap();
        }
        #[cfg(windows)]
        fs::create_dir_all(&path).unwrap();
        fs::create_dir_all(path.join("files")).unwrap();
        fs::create_dir_all(path.join("requests")).unwrap();
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_original_kokoro_shape(path: &Path) {
    let sample_count = 24_000_u32 * 597 / 100;
    let data_size = sample_count * 2;
    let mut file = File::create(path).expect("WAV fixture should create");
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&24_000_u32.to_le_bytes()).unwrap();
    file.write_all(&48_000_u32.to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.set_len(u64::from(44 + data_size)).unwrap();
}

fn playa_command(root: &Path, cwd: &Path) -> Command {
    // `bin_exe!` reads nextest's run-time binary location, so the archive leg
    // (wsl2-ubuntu) does not launch the compile-time path from another host.
    let mut command = Command::new(biscuit_test_harness::bin_exe!("playa"));
    command
        .current_dir(cwd)
        .env("PLAYA_SPOOL_DIR", root)
        .env_remove("PLAYA_DRY_RUN");
    command
}

#[test]
#[serial_test::serial]
fn multiprocess_background_publication_preserves_options_and_unique_order() {
    let root = TestRoot::new("multiprocess");
    let audio = root.0.join("kokoro-24khz-mono-5.97s.wav");
    write_original_kokoro_shape(&audio);
    let spool = LockedAudioSpool::new(&root.0);
    let root_path = Arc::new(root.0.clone());

    let mut publishers = Vec::new();
    for _ in 0..8 {
        let root_path = Arc::clone(&root_path);
        publishers.push(std::thread::spawn(move || {
            playa_command(&root_path, &root_path)
                .args([
                    "play",
                    "kokoro-24khz-mono-5.97s.wav",
                    "--background",
                    "--force-host",
                    "--speed",
                    "1.25",
                    "--volume",
                    "0.5",
                    "--channel",
                    "USB DAC",
                ])
                .status()
                .expect("publisher process should launch")
        }));
    }
    let outcomes: Vec<_> = publishers.into_iter().map(|publisher| publisher.join()).collect();
    for outcome in outcomes {
        assert!(outcome.expect("publisher should not panic").success());
    }

    // SAFETY: the serial guard prevents concurrent environment mutation in this binary.
    unsafe { std::env::set_var("PLAYA_SPOOL_DIR", &root.0) };
    let snapshot = playa::detached::snapshot().expect("isolated spool should inspect");
    assert_eq!(
        snapshot
            .pending
            .iter()
            .map(|job| job.sequence)
            .collect::<Vec<_>>(),
        (1..=8).collect::<Vec<_>>()
    );

    let pending_path = fs::read_dir(&root.0)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().ends_with(".pending.json"))
        .expect("a pending envelope should exist");
    let envelope: serde_json::Value =
        serde_json::from_slice(&fs::read(pending_path).unwrap()).unwrap();
    assert_eq!(envelope["payload"]["playback"]["routing"], "force_host");
    assert_eq!(envelope["payload"]["playback"]["speed"], 1.25);
    assert_eq!(envelope["payload"]["playback"]["volume"], 0.5);
    assert_eq!(
        envelope["payload"]["playback"]["channel"],
        "USB DAC"
    );

    let output = playa_command(&root.0, &root.0)
        .arg("spool")
        .output()
        .expect("spool command should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Detached audio spool"));
    assert!(stdout.contains("preparing") || stdout.contains("ready"));

    spool.clear_pending().expect("pending work should be removed while owned");
    assert!(playa::detached::snapshot().unwrap().pending.is_empty());
    drop(spool);
}

/// Longest the queue holder waits for the fixture-owning scope to unwind
/// before it commits its record and releases `queue.lock`.
///
/// Only the passing path pays this. Cleanup that takes `queue.lock` cannot
/// finish while the holder owns it, so the wait runs to the deadline; cleanup
/// that skips the lock finishes immediately and the holder proceeds at once.
const QUEUE_HOLD_BUDGET: Duration = Duration::from_millis(250);

#[test]
fn publication_unwind_removes_pending_before_releasing_worker_ownership() {
    let root = TestRoot::new("publication-unwind");
    let pending = root.0.join("1.pending.json");

    // A record written with no concurrent queue mutator cannot distinguish
    // cleanup that takes `queue.lock` from cleanup that does not. This holder
    // reproduces `enqueue_state_at`'s ordering instead: it owns `queue.lock`
    // across its commit, and delays that commit until the fixture-owning scope
    // has unwound — which can only happen first if cleanup skipped the lock.
    let queue_held = Arc::new(AtomicBool::new(false));
    let owner_finished = Arc::new(AtomicBool::new(false));

    let publisher = std::thread::spawn({
        let queue_path = LockedAudioSpool::queue_lock_path(&root.0);
        let pending = pending.clone();
        let queue_held = Arc::clone(&queue_held);
        let owner_finished = Arc::clone(&owner_finished);
        move || {
            let queue = LockedAudioSpool::open_lock(&queue_path).expect("queue lock should open");
            queue
                .lock_exclusive()
                .expect("publisher should own the queue");
            queue_held.store(true, Ordering::SeqCst);

            let deadline = Instant::now() + QUEUE_HOLD_BUDGET;
            while !owner_finished.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            fs::write(&pending, b"record committed under the queue lock").unwrap();

            fs4::fs_std::FileExt::unlock(&queue).expect("publisher should release the queue");
        }
    });
    while !queue_held.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(1));
    }

    let result = std::panic::catch_unwind(|| {
        let _spool = LockedAudioSpool::new(&root.0);
        panic!("simulated assertion failure");
    });

    owner_finished.store(true, Ordering::SeqCst);
    publisher.join().expect("publisher should not panic");

    assert!(result.is_err());
    assert!(
        !pending.exists(),
        "a record committed under the queue lock survived fixture cleanup"
    );
    // Worker ownership is reacquirable only because nothing runnable remains.
    let _spool = LockedAudioSpool::new(&root.0);
}

#[test]
#[serial_test::serial]
fn requester_exit_is_followed_by_worker_failure_journal_and_clean_exit() {
    let root = TestRoot::new("requester-exit");
    let invalid = root.0.join("not-really-audio.wav");
    fs::write(&invalid, b"not audio").expect("invalid fixture should write");

    let status = playa_command(&root.0, &root.0)
        .args(["play", "not-really-audio.wav", "--background"])
        .status()
        .expect("requester should launch");
    assert!(status.success(), "requester returns after durable publication");

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let journal = root.0.join("journal.jsonl");
    while !journal.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let contents = fs::read_to_string(&journal).expect("detached worker should journal");
    assert!(contents.contains("playback_failed"));
    assert!(!contents.contains("not-really-audio.wav"));

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.0.join("worker.lock"))
            .expect("worker lock should remain");
        if lock.try_lock_exclusive().expect("worker lock should probe") {
            fs4::fs_std::FileExt::unlock(&lock).expect("probe should release");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "detached scheduler should exit after the queue becomes empty"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
