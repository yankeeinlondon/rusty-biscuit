//! Public-path regression for the `Truncated` playback verdict.
//!
//! A host player that exits with status 0 long before the probed duration
//! must surface as `PlaybackVerdict::Truncated` through the public
//! explicit-player entry points, return `Ok`, and log exactly one warning.
//! This is the failure mode of the original mpv mono clipping; a direct test
//! of the private warning helper cannot detect the pipeline disconnecting
//! from it.
//!
//! This target is `harness = false` so the test binary can double as the
//! `mpv` fixture: each test copies its own executable into a private `bin`
//! directory under that name and `main` dispatches on the executable's file
//! stem. Production spawns the player by bare name on `PATH`, which on
//! Windows finds only real executables, so shell-script stubs cannot stand in
//! and the repo forbids fixture binaries in production crates.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use playa::{
    AudioData, AudioPlayer, PlaybackOptions, PlaybackReport, PlaybackRoute, PlaybackVerdict,
    probe_audio_metadata,
};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

const TRUNCATION_WARNING: &str = "audio playback ended before the expected duration";

const TESTS: &[(&str, fn())] = &[
    (
        "truncated_verdict_warns_once_and_returns_ok",
        truncated_verdict_warns_once_and_returns_ok,
    ),
    #[cfg(feature = "async")]
    (
        "truncated_verdict_warns_once_and_returns_ok_async",
        truncated_verdict_warns_once_and_returns_ok_async,
    ),
];

fn main() -> ExitCode {
    let exe = env::current_exe().expect("current executable path");
    if exe.file_stem().and_then(|stem| stem.to_str()) == Some("mpv") {
        return stub_mpv();
    }
    run_tests(&env::args().skip(1).collect::<Vec<_>>())
}

/// Exits successfully well before the two-second source could have played;
/// the arguments production passes (`--no-video ... <file>`) are ignored.
fn stub_mpv() -> ExitCode {
    std::thread::sleep(Duration::from_millis(100));
    ExitCode::SUCCESS
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

/// Private per-test directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let path = env::temp_dir().join(format!(
            "playa-truncated-verdict-{}-{nanos}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create private temp dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Restricts `PATH` to one directory for the test body and restores the
/// original value on drop.
struct PathGuard(Option<OsString>);

impl PathGuard {
    fn only(dir: &Path) -> Self {
        let original = env::var_os("PATH");
        // SAFETY: this binary is `harness = false` and runs its tests one at a
        // time on the main thread, so no other thread reads the environment
        // while it is modified.
        unsafe { env::set_var("PATH", dir) };
        Self(original)
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        // SAFETY: see `PathGuard::only`.
        unsafe {
            match self.0.take() {
                Some(original) => env::set_var("PATH", original),
                None => env::remove_var("PATH"),
            }
        }
    }
}

/// Install this executable as `mpv` so `Command::new("mpv")` finds a real
/// executable whose stem selects the stub role in `main`.
fn install_fake_mpv(bin: &Path) {
    let exe = env::current_exe().expect("current executable path");
    fs::create_dir_all(bin).expect("create fixture bin dir");
    fs::copy(&exe, bin.join(format!("mpv{}", env::consts::EXE_SUFFIX)))
        .expect("copy test binary as mpv fixture");
}

/// 24 kHz mono 16-bit PCM silence of the given duration.
fn pcm_wav(sample_rate: u32, channels: u16, duration: Duration) -> Vec<u8> {
    let frames = (duration.as_secs_f64() * f64::from(sample_rate)).round() as u32;
    let data_len = frames * u32::from(channels) * 2;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
    wav.extend_from_slice(&(channels * 2).to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.resize(44 + data_len as usize, 0);
    wav
}

/// Records the message of every WARN-level event on the installing thread.
#[derive(Clone, Default)]
struct WarnCollector {
    messages: Arc<Mutex<Vec<String>>>,
}

impl WarnCollector {
    fn messages(&self) -> Vec<String> {
        self.messages.lock().expect("warn collector poisoned").clone()
    }
}

struct MessageVisitor(Option<String>);

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = Some(format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = Some(value.to_string());
        }
    }
}

impl Subscriber for WarnCollector {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() == Level::WARN
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        if *event.metadata().level() != Level::WARN {
            return;
        }
        let mut visitor = MessageVisitor(None);
        event.record(&mut visitor);
        self.messages
            .lock()
            .expect("warn collector poisoned")
            .push(visitor.0.unwrap_or_default());
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

struct Fixture {
    _temp: TempDir,
    _path: PathGuard,
    audio: AudioData,
}

const SOURCE_DURATION: Duration = Duration::from_secs(2);

/// Fake `mpv` on a private `PATH` plus a probed two-second mono WAV.
fn fixture() -> Fixture {
    let temp = TempDir::new();
    let bin = temp.path().join("bin");
    install_fake_mpv(&bin);
    let path = PathGuard::only(&bin);

    let wav = temp.path().join("silence.wav");
    fs::write(&wav, pcm_wav(24_000, 1, SOURCE_DURATION)).expect("write wav fixture");
    let audio = AudioData::FilePath(wav);

    let probed = probe_audio_metadata(&audio).expect("wav fixture must probe");
    assert_eq!(probed.duration, SOURCE_DURATION);
    assert_eq!(probed.channels, 1);

    Fixture {
        _temp: temp,
        _path: path,
        audio,
    }
}

fn assert_truncated(report: &PlaybackReport, warnings: &[String]) {
    assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
    assert_eq!(report.expected, Some(SOURCE_DURATION));
    assert!(
        report.elapsed < Duration::from_secs(1),
        "fake mpv exits after ~100 ms; elapsed {:?}",
        report.elapsed
    );
    match report.verdict {
        PlaybackVerdict::Truncated { missing } => assert!(
            missing >= Duration::from_millis(1_000),
            "missing {missing:?} for elapsed {:?}",
            report.elapsed
        ),
        other => panic!("expected Truncated verdict, got {other:?} ({report:?})"),
    }
    assert_eq!(warnings, [TRUNCATION_WARNING.to_string()]);
}

fn truncated_verdict_warns_once_and_returns_ok() {
    let fixture = fixture();
    let collector = WarnCollector::default();
    let report = {
        let _subscriber = tracing::subscriber::set_default(collector.clone());
        playa::playa_with_player_and_options_with_report(
            AudioPlayer::Mpv,
            fixture.audio.clone(),
            PlaybackOptions::default(),
        )
        .expect("truncated playback must still return Ok")
    };
    assert_truncated(&report, &collector.messages());
}

#[cfg(feature = "async")]
fn truncated_verdict_warns_once_and_returns_ok_async() {
    let fixture = fixture();
    let collector = WarnCollector::default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build current-thread runtime");
    let report = {
        let _subscriber = tracing::subscriber::set_default(collector.clone());
        runtime
            .block_on(playa::playa_with_player_and_options_async_with_report(
                AudioPlayer::Mpv,
                fixture.audio.clone(),
                PlaybackOptions::default(),
            ))
            .expect("truncated playback must still return Ok")
    };
    assert_truncated(&report, &collector.messages());
}
