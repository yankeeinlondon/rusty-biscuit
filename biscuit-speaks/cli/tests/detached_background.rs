//! `so-you-say --background` process regressions.
//!
//! This target is `harness = false` so the test binary can double as the
//! `kokoro-tts`, `say`, and `mpv` fixtures: each test copies its own executable into a
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
use fs4::fs_std::FileExt as _;
use sysinfo::{ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind};

const TESTS: &[(&str, fn())] = &[
    ("background_zero_volume_survives_preparation_and_delegate", background_zero_volume_survives_preparation_and_delegate),
    ("background_timeout_terminates_only_fixture_processes", background_timeout_terminates_only_fixture_processes),
    (
        "background_cache_miss_returns_after_reservation_then_completes_same_slot",
        background_cache_miss_returns_after_reservation_then_completes_same_slot,
    ),
    (
        "background_cleanup_waits_for_completion_after_assertion_failure",
        background_cleanup_waits_for_completion_after_assertion_failure,
    ),
    #[cfg(target_os = "macos")]
    (
        "background_say_prepares_speech_before_volume_controlled_playback",
        background_say_prepares_speech_before_volume_controlled_playback,
    ),
    #[cfg(target_os = "macos")]
    (
        "background_say_cleanup_waits_for_completion_after_assertion_failure",
        background_say_cleanup_waits_for_completion_after_assertion_failure,
    ),
    (
        "background_dry_run_creates_no_spool_or_worker_side_effects",
        background_dry_run_creates_no_spool_or_worker_side_effects,
    ),
];

fn main() -> ExitCode {
    let exe = env::current_exe().expect("current executable path");
    match exe.file_stem().and_then(|stem| stem.to_str()) {
        Some("audio-enqueuer") => return enqueue_muted_test_speech(),
        Some("kokoro-tts") => return stub_kokoro_tts(),
        Some("mpv") => return stub_mpv(),
        #[cfg(target_os = "macos")]
        Some("say") => return stub_say(),
        _ => {}
    }
    run_tests(&env::args().skip(1).collect::<Vec<_>>())
}

fn enqueue_muted_test_speech() -> ExitCode {
    use biscuit_speaks::{HostTtsProvider, Speak, TtsFailoverStrategy, TtsProvider, VolumeLevel};

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        if let Some(code) = biscuit_speaks::run_if_worker().await {
            return ExitCode::from(u8::try_from(code).unwrap_or(1));
        }
        Speak::new("This is a test message.")
            .with_voice("af_heart")
            .with_failover(TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(HostTtsProvider::KokoroTts)))
            .with_volume(VolumeLevel::Explicit(0.0))
            .play_detached()
            .await
            .expect("publish muted preparation");
        ExitCode::SUCCESS
    })
}

/// Blocked synthesis writes invalid audio so no real backend can play it.
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
    let capture = env::var_os("BISCUIT_TEST_PLAYBACK_ARGS").expect("playback argument capture");
    fs::write(capture, env::args().skip(1).collect::<Vec<_>>().join("\n")).unwrap();
    touch_marker("BISCUIT_TEST_PLAYBACK_STARTED");
    if !wait_for_release("BISCUIT_TEST_PLAYBACK_RELEASE") {
        return ExitCode::from(124);
    }
    ExitCode::SUCCESS
}

#[cfg(target_os = "macos")]
fn stub_say() -> ExitCode {
    use std::io::Read as _;

    let args = env::args().skip(1).collect::<Vec<_>>();
    if args == ["-v", "?"] {
        println!("Samantha en_US # This is a test message.");
        return ExitCode::SUCCESS;
    }
    let marker = PathBuf::from(env::var_os("BISCUIT_TEST_SYNTHESIS_STARTED").unwrap());
    fs::write(marker.with_extension("args"), args.join("\n")).unwrap();
    let output = args.windows(2).find(|pair| pair[0] == "-o").expect("file synthesis output");
    assert!(args.iter().any(|arg| arg == "--file-format=WAVE"));
    assert!(args.iter().any(|arg| arg == "--data-format=LEI16"));
    assert!(args.windows(2).any(|pair| pair == ["-v", "Samantha"]));
    assert!(args.windows(2).any(|pair| pair == ["-r", "131"]));
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).unwrap();
    assert_eq!(text, "This is a test message.");
    touch_marker("BISCUIT_TEST_SYNTHESIS_STARTED");
    if !wait_for_release("BISCUIT_TEST_SYNTHESIS_RELEASE") {
        return ExitCode::from(124);
    }
    fs::write(&output[1], b"not-a-real-wave").unwrap();
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
        if path.exists() && env::var_os("BISCUIT_TEST_IGNORE_RELEASE").is_none() {
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

struct ReleaseOnDrop {
    markers: Vec<PathBuf>,
    spool: PathBuf,
    bin: PathBuf,
    cooperative_timeout: Duration,
}

impl ReleaseOnDrop {
    /// Returns whether cleanup needed the forced-termination path.
    fn finish(&self) -> std::io::Result<bool> {
        let released = self.markers.iter().all(|path| fs::write(path, b"release").is_ok());
        let timeout = if released { self.cooperative_timeout } else { Duration::ZERO };
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.is_idle() && self.fixture_processes(false)? == 0 {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if self.fixture_processes(true)? == 0 {
                break;
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::other("fixture-owned processes did not exit after termination"));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if self.spool.exists() {
            let open_lock = |name: &str| fs::OpenOptions::new()
                .read(true).write(true).create(true).truncate(false).open(self.spool.join(name));
            let queue = open_lock("queue.lock")?;
            if !queue.try_lock_exclusive()? {
                return Err(std::io::Error::other("fixture queue lock remained held after termination"));
            }
            let worker = open_lock("worker.lock")?;
            if !worker.try_lock_exclusive()? {
                return Err(std::io::Error::other("fixture worker lock remained held after termination"));
            }
            for entry in fs::read_dir(&self.spool)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(".pending.json") || name.ends_with(".in-flight.json") {
                    fs::remove_file(entry.path())?;
                }
            }
        }
        Ok(true)
    }

    fn fixture_processes(&self, terminate: bool) -> std::io::Result<usize> {
        // Both sides belong to this closed canonical key space. A copied CLI
        // makes scheduler, preparation, and delegate ownership unambiguous even
        // after setsid/reparenting; the real CLI and test runner are outside it.
        let bin = fs::canonicalize(&self.bin)?;
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
        );
        let owned = |process: &sysinfo::Process| process.exe()
            .and_then(|path| fs::canonicalize(path).ok())
            .is_some_and(|path| path.parent() == Some(bin.as_path()));
        let candidates = system.processes().values()
            .filter(|process| owned(process) && process.status() != ProcessStatus::Zombie)
            .map(|process| (process.pid(), process.start_time()))
            .collect::<Vec<_>>();
        if terminate && !candidates.is_empty() {
            let pids = candidates.iter().map(|(pid, _)| *pid).collect::<Vec<_>>();
            system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&pids),
                true,
                ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
            );
            for (pid, started) in &candidates {
                if let Some(process) = system.process(*pid)
                    && process.start_time() == *started
                    && owned(process)
                {
                    let _ = process.kill();
                }
            }
        }
        Ok(candidates.len())
    }

    fn is_idle(&self) -> bool {
        if !self.spool.exists() {
            return true;
        }
        let Ok(entries) = fs::read_dir(&self.spool) else { return false };
        for entry in entries {
            let Ok(entry) = entry else { return false };
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".pending.json") || name.ends_with(".in-flight.json") {
                return false;
            }
        }
        // The terminal journal transition precedes the scheduler's final-empty
        // lock release. Both boundaries must pass before removing its spool.
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.spool.join("worker.lock"))
            .is_ok_and(|worker| worker.try_lock_exclusive().unwrap_or(false))
    }
}

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            // Do not hide cleanup failure behind the assertion being unwound.
            let _ = fs::write(self.spool.with_extension("cleanup-error"), error.to_string());
            if std::thread::panicking() {
                eprintln!("fixture audio cleanup failed: {error}");
            } else {
                panic!("fixture audio cleanup failed: {error}");
            }
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
    run_background_case("kokoro", "af_heart", BackgroundCase::Normal);
}

fn background_cleanup_waits_for_completion_after_assertion_failure() {
    run_background_case("kokoro", "af_heart", BackgroundCase::Unwind);
}

#[cfg(target_os = "macos")]
fn background_say_prepares_speech_before_volume_controlled_playback() {
    run_background_case("say", "Samantha", BackgroundCase::Normal);
}

#[cfg(target_os = "macos")]
fn background_say_cleanup_waits_for_completion_after_assertion_failure() {
    run_background_case("say", "Samantha", BackgroundCase::Unwind);
}

#[derive(Clone, Copy, PartialEq)]
enum BackgroundCase { Normal, Unwind, Muted, Timeout }

fn background_zero_volume_survives_preparation_and_delegate() {
    run_background_case("kokoro", "af_heart", BackgroundCase::Muted);
}

fn background_timeout_terminates_only_fixture_processes() {
    run_background_case("kokoro", "af_heart", BackgroundCase::Timeout);
}

fn run_background_case(provider: &str, voice: &str, case: BackgroundCase) {
    let simulate_failure = case == BackgroundCase::Unwind;
    let muted = case == BackgroundCase::Muted;
    let force_timeout = case == BackgroundCase::Timeout;
    let temp = tempfile::tempdir().unwrap();
    let bin = temp.path().join("bin");
    let spool = temp.path().join("spool");
    let synthesis_started = temp.path().join("synthesis-started");
    let synthesis_release = temp.path().join("synthesis-release");
    let playback_started = temp.path().join("playback-started");
    let playback_release = temp.path().join("playback-release");
    fs::create_dir(&bin).unwrap();
    install_fixture(&bin, if provider == "say" { "say" } else { "kokoro-tts" });
    install_fixture(&bin, "mpv");
    let enqueuer = if muted {
        install_fixture(&bin, "audio-enqueuer");
        bin.join(format!("audio-enqueuer{}", env::consts::EXE_SUFFIX))
    } else {
        let copied_cli = bin.join(format!("so-you-say{}", env::consts::EXE_SUFFIX));
        fs::copy(bin_exe!("so-you-say"), &copied_cli).unwrap();
        copied_cli
    };

    let original = "This is a test message.";
    let cache_dir = temp.path().join("cache");
    fs::create_dir(&cache_dir).unwrap();
    let cache_voice = if provider == "say" { format!("{voice};rate=131") } else { voice.to_string() };
    let key = biscuit_speaks::audio_cache::CacheKey::new(provider, cache_voice, original, "wav");
    let cache_name = key.cache_path();
    let cache = cache_dir.join(cache_name.file_name().unwrap());
    let _cache_cleanup = RemoveOnDrop(cache.clone());
    let result = panic::catch_unwind(|| {
        let _release = ReleaseOnDrop {
            markers: vec![synthesis_release.clone(), playback_release.clone()],
            spool: spool.clone(),
            bin: bin.clone(),
            cooperative_timeout: if force_timeout { Duration::ZERO } else { Duration::from_secs(20) },
        };
        let mut command = Command::new(&enqueuer);
        if force_timeout {
            command.env("BISCUIT_TEST_IGNORE_RELEASE", "1");
        }
        let output = command
            .args(["--background", "--provider", provider, "--voice", voice, "--soft", original])
            .args((provider == "say").then_some("--slow"))
            .env("BISCUIT_SPEAKS_CACHE", cache_dir.join("voices.json"))
            .env("TMPDIR", &cache_dir)
            .env("TMP", &cache_dir)
            .env("TEMP", &cache_dir)
            .env("PATH", &bin)
            .env("PATHEXT", ".EXE")
            .env("PLAYA_SPOOL_DIR", &spool)
            .env("BISCUIT_TEST_SYNTHESIS_STARTED", &synthesis_started)
            .env("BISCUIT_TEST_SYNTHESIS_RELEASE", &synthesis_release)
            .env("BISCUIT_TEST_PLAYBACK_STARTED", &playback_started)
            .env("BISCUIT_TEST_PLAYBACK_RELEASE", &playback_release)
            .env("BISCUIT_TEST_PLAYBACK_ARGS", temp.path().join("playback-args"))
            .env_remove("PLAYA_DRY_RUN")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "background handoff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        wait_for("blocked synthesis helper", || {
            let journal = fs::read_to_string(spool.join("journal.jsonl")).unwrap_or_default();
            assert!(!journal.contains("\"transition\":\"failed\""),
                "preparation failed; synthesis args: {:?}; journal: {journal}",
                fs::read_to_string(synthesis_started.with_extension("args")));
            synthesis_started.exists()
        });
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
        assert!(preparing.contains(if muted {
            "\"volume\":{\"explicit\":0.0}"
        } else {
            "\"volume\":\"soft\""
        }));
        if force_timeout {
            // The synthesis-started marker proves an owned blocked process is
            // present before exercising the zero cooperative deadline.
            assert!(_release.fixture_processes(false).unwrap() > 0);
            assert!(_release.finish().unwrap(), "timeout must exercise termination");
            return;
        }
        assert!(!simulate_failure, "simulated assertion failure during synthesis");

        fs::write(&synthesis_release, b"release").unwrap();
        wait_for("blocked delegated playback", || playback_started.exists());
        assert!(!playback_release.exists());
        let playback_args = fs::read_to_string(temp.path().join("playback-args")).unwrap();
        assert!(playback_args.lines().any(|arg| arg == if muted { "--volume=0" } else { "--volume=50" }));
        let in_flight = pending.path().with_file_name(
            pending.file_name().to_string_lossy().replace(".pending.json", ".in-flight.json"),
        );
        let ready = fs::read_to_string(in_flight).unwrap();
        assert!(ready.contains("\"state\":\"ready\""));
        assert!(ready.contains(if muted { "\"volume\":0.0" } else { "\"volume\":0.5" }));
        assert!(ready.contains("\"sequence\":1"));
        assert_eq!(fs::read(&cache).unwrap(), b"not-a-real-wave");
        fs::write(&playback_release, b"release").unwrap();
        wait_for("completed journal entry", || {
            fs::read_to_string(spool.join("journal.jsonl"))
                .is_ok_and(|journal| journal.contains("\"transition\":\"completed\""))
        });
        assert!(!_release.finish().unwrap(), "normal completion must not require termination");
    });
    if simulate_failure {
        let panic = result.expect_err("the simulated assertion must fail");
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"simulated assertion failure during synthesis"));
    } else if let Err(panic) = result {
        panic::resume_unwind(panic);
    }
    let cleanup = ReleaseOnDrop {
        markers: vec![synthesis_release, playback_release],
        spool: spool.clone(),
        bin: bin.clone(),
        cooperative_timeout: Duration::from_secs(20),
    };
    assert!(!spool.with_extension("cleanup-error").exists());
    assert_eq!(cleanup.fixture_processes(false).unwrap(), 0);
    assert!(cleanup.is_idle(), "unwinding must drain fixture work before returning");
    if !force_timeout {
        assert!(fs::read_to_string(spool.join("journal.jsonl"))
            .is_ok_and(|journal| journal.contains("\"transition\":\"completed\"")));
    }
}

fn background_dry_run_creates_no_spool_or_worker_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("spool");
    let output = Command::new(bin_exe!("so-you-say"))
        .args(["--background", "--provider", "kokoro", "This is a test message."])
        .env("PLAYA_SPOOL_DIR", &spool)
        .env("PLAYA_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!spool.exists());
}
