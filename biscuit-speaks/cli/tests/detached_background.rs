#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use biscuit_test_harness::bin_exe;

struct ReleaseOnDrop(Vec<std::path::PathBuf>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::write(path, b"release");
        }
    }
}

struct RemoveOnDrop(std::path::PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn wait_for(label: &str, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {label}");
}

#[test]
fn background_cache_miss_returns_after_reservation_then_completes_same_slot() {
    let temp = tempfile::tempdir().unwrap();
    let bin = temp.path().join("bin");
    let spool = temp.path().join("spool");
    let synthesis_started = temp.path().join("synthesis-started");
    let synthesis_release = temp.path().join("synthesis-release");
    let playback_started = temp.path().join("playback-started");
    let playback_release = temp.path().join("playback-release");
    let _release = ReleaseOnDrop(vec![synthesis_release.clone(), playback_release.clone()]);
    fs::create_dir(&bin).unwrap();

    write_executable(
        &bin.join("kokoro-tts"),
        "#!/bin/sh\n/usr/bin/touch \"$BISCUIT_TEST_SYNTHESIS_STARTED\"\ni=0\nwhile [ ! -f \"$BISCUIT_TEST_SYNTHESIS_RELEASE\" ]; do\n  i=$((i + 1))\n  [ \"$i\" -ge 500 ] && exit 124\n  /bin/sleep 0.02\ndone\n/usr/bin/printf 'not-a-real-wave' > \"$2\"\n",
    );
    write_executable(
        &bin.join("mpv"),
        "#!/bin/sh\n/usr/bin/touch \"$BISCUIT_TEST_PLAYBACK_STARTED\"\ni=0\nwhile [ ! -f \"$BISCUIT_TEST_PLAYBACK_RELEASE\" ]; do\n  i=$((i + 1))\n  [ \"$i\" -ge 500 ] && exit 124\n  /bin/sleep 0.02\ndone\nexit 0\n",
    );

    let original = "Phase 1 of the plan in the claudine package area, was implemented successfully";
    let cache = biscuit_speaks::audio_cache::CacheKey::new("kokoro", "af_heart", original, "wav")
        .cache_path();
    let _ = fs::remove_file(&cache);
    let _cache_cleanup = RemoveOnDrop(cache);
    let output = Command::new(bin_exe!("so-you-say"))
        .args(["--background", "--provider", "kokoro", original])
        .env("PATH", &bin)
        .env("PLAYA_SPOOL_DIR", &spool)
        .env("BISCUIT_TEST_SYNTHESIS_STARTED", &synthesis_started)
        .env("BISCUIT_TEST_SYNTHESIS_RELEASE", &synthesis_release)
        .env("BISCUIT_TEST_PLAYBACK_STARTED", &playback_started)
        .env("BISCUIT_TEST_PLAYBACK_RELEASE", &playback_release)
        .env_remove("PLAYA_DRY_RUN")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "background handoff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    wait_for("blocked synthesis helper", || synthesis_started.exists());
    let pending = fs::read_dir(&spool)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().ends_with(".pending.json"))
        .expect("reserved preparation record must be durable");
    let preparing = fs::read_to_string(pending.path()).unwrap();
    assert!(preparing.contains("\"state\":\"preparing\""));
    assert!(preparing.contains(original));
    assert!(!preparing.contains("\"providers\""));
    assert!(!preparing.contains("API_KEY"));

    fs::write(&synthesis_release, b"release").unwrap();
    wait_for("blocked delegated playback", || playback_started.exists());
    assert!(!playback_release.exists());
    fs::write(&playback_release, b"release").unwrap();
    wait_for("completed journal entry", || {
        fs::read_to_string(spool.join("journal.jsonl"))
            .is_ok_and(|journal| journal.contains("\"transition\":\"completed\""))
    });
}

#[test]
fn background_dry_run_creates_no_spool_or_worker_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("spool");
    let output = Command::new(bin_exe!("so-you-say"))
        .args(["--background", "--provider", "kokoro", "dry run speech"])
        .env("PLAYA_SPOOL_DIR", &spool)
        .env("PLAYA_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!spool.exists());
}
