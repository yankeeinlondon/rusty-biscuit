use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fs4::fs_std::FileExt as _;

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

fn worker_lock(root: &Path) -> File {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("worker.lock"))
        .expect("worker lock should open");
    assert!(file.try_lock_exclusive().expect("worker lock should work"));
    file
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
    let lock = worker_lock(&root.0);
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
    for publisher in publishers {
        assert!(publisher.join().expect("publisher should not panic").success());
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

    fs4::fs_std::FileExt::unlock(&lock).expect("worker lock should release");
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
