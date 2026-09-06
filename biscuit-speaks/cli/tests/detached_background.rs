//! `so-you-say --background` process regressions.
//!
//! This target is `harness = false` so the test binary can double as the
//! `kokoro-tts` and `mpv` fixtures: each test copies its own executable into a
//! private `bin` directory under those names and `main` dispatches on the
//! executable's file stem. Production resolves both programs by bare name on
//! `PATH`, which on Windows finds only real executables, so shell-script stubs
//! cannot stand in and the repo forbids fixture binaries in production crates.

use std::env;
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use biscuit_test_harness::bin_exe;

const TESTS: &[(&str, fn())] = &[
    (
        "background_cache_miss_returns_after_reservation_then_completes_same_slot",
        background_cache_miss_returns_after_reservation_then_completes_same_slot,
    ),
    (
        "background_dry_run_creates_no_spool_or_worker_side_effects",
        background_dry_run_creates_no_spool_or_worker_side_effects,
    ),
];

fn main() -> ExitCode {
    let exe = env::current_exe().expect("current executable path");
    match exe.file_stem().and_then(|stem| stem.to_str()) {
        Some("kokoro-tts") => return stub_kokoro_tts(),
        Some("mpv") => return stub_mpv(),
        _ => {}
    }
    run_tests(&env::args().skip(1).collect::<Vec<_>>())
}

/// Blocks like a real synthesis, then writes a fake wave to its second
/// positional argument (the output path in `kokoro-tts <input> <output> ...`).
fn stub_kokoro_tts() -> ExitCode {
    touch_marker("BISCUIT_TEST_SYNTHESIS_STARTED");
    if !wait_for_release("BISCUIT_TEST_SYNTHESIS_RELEASE") {
        return ExitCode::from(124);
    }
    let output = env::args_os().nth(2).expect("kokoro-tts output path");
    fs::write(output, b"not-a-real-wave").expect("write fake wave");
    ExitCode::SUCCESS
}

fn stub_mpv() -> ExitCode {
    touch_marker("BISCUIT_TEST_PLAYBACK_STARTED");
    if !wait_for_release("BISCUIT_TEST_PLAYBACK_RELEASE") {
        return ExitCode::from(124);
    }
    ExitCode::SUCCESS
}

fn touch_marker(var: &str) {
    let path = env::var_os(var).unwrap_or_else(|| panic!("{var} must name the marker file"));
    fs::write(path, b"").expect("touch marker");
}

fn wait_for_release(var: &str) -> bool {
    let path = env::var_os(var).unwrap_or_else(|| panic!("{var} must name the release file"));
    let path = PathBuf::from(path);
    for _ in 0..500 {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// The libtest CLI subset nextest drives: `--list --format terse` (plus an
/// `--ignored` pass that must list nothing) during discovery, then one
/// `<name> --exact --nocapture` invocation per test.
fn run_tests(args: &[String]) -> ExitCode {
    let mut filters = Vec::new();
    let mut list = false;
    let mut ignored = false;
    let mut exact = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--list" => list = true,
            "--ignored" => ignored = true,
            "--exact" => exact = true,
            "--format" | "--skip" | "--test-threads" | "--color" | "--logfile" | "-Z" => {
                iter.next();
            }
            flag if flag.starts_with('-') => {}
            name => filters.push(name.to_string()),
        }
    }
    let matches = |name: &str, filter: &String| {
        if exact {
            name == filter
        } else {
            name.contains(filter.as_str())
        }
    };
    let selected = TESTS.iter().filter(|(name, _)| {
        filters.is_empty() || filters.iter().any(|filter| matches(name, filter))
    });
    if list {
        if !ignored {
            for (name, _) in selected {
                println!("{name}: test");
            }
        }
        return ExitCode::SUCCESS;
    }

    let mut failed = Vec::new();
    for (name, test) in selected {
        match panic::catch_unwind(test) {
            Ok(()) => println!("test {name} ... ok"),
            Err(_) => {
                println!("test {name} ... FAILED");
                failed.push(*name);
            }
        }
    }
    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("failures: {failed:?}");
        ExitCode::from(101)
    }
}

struct ReleaseOnDrop(Vec<PathBuf>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::write(path, b"release");
        }
    }
}

struct RemoveOnDrop(PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Install this executable under `name` so `PATH` lookups and `Command::new`
/// find a real executable whose stem selects the stub role in `main`.
fn install_fixture(bin: &Path, name: &str) {
    let exe = env::current_exe().unwrap();
    fs::copy(&exe, bin.join(format!("{name}{}", env::consts::EXE_SUFFIX))).unwrap();
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
    install_fixture(&bin, "kokoro-tts");
    install_fixture(&bin, "mpv");

    let original = "Phase 1 of the plan in the claudine package area, was implemented successfully";
    let cache = biscuit_speaks::audio_cache::CacheKey::new("kokoro", "af_heart", original, "wav")
        .cache_path();
    let _ = fs::remove_file(&cache);
    let _cache_cleanup = RemoveOnDrop(cache);
    let output = Command::new(bin_exe!("so-you-say"))
        .args(["--background", "--provider", "kokoro", original])
        .env("PATH", &bin)
        .env("PATHEXT", ".EXE")
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
