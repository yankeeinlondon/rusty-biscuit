//! Synthesis, cache reuse, and publication observed through recording fakes.
//!
//! Every test owns a [`common::TtsFixture`], so no test reads the host's TTS
//! tools, audio cache, capability cache, or spool.

#![cfg(unix)]

mod common;

use std::time::{Duration, Instant};

use biscuit_speaks::{KokoroTtsProvider, TtsConfig, TtsExecutor, VolumeLevel};
use common::{Behavior, FakeTool, TtsFixture, fake_audio, process_alive};

fn wait_for(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[serial_test::serial]
fn fixture_owns_every_location_synthesis_can_touch() {
    let tmpdir_before = std::env::var_os("TMPDIR");
    {
        let fixture = TtsFixture::new();
        assert_eq!(std::env::temp_dir(), fixture.temp_dir());
        let key = biscuit_speaks::audio_cache::CacheKey::new("kokoro", "af_heart", "text", "wav");
        assert_eq!(key.cache_path().parent(), Some(fixture.temp_dir().as_path()));
        assert_eq!(std::env::var_os("PATH"), Some(fixture.bin_dir().into_os_string()));
        assert_eq!(
            std::env::var_os("BISCUIT_SPEAKS_CACHE"),
            Some(fixture.capability_cache().into_os_string())
        );
        assert_eq!(
            std::env::var_os("PLAYA_SPOOL_DIR"),
            Some(fixture.spool_dir().into_os_string())
        );
        assert_eq!(std::env::var_os("PLAYA_DRY_RUN"), None);
        fixture.assert_spool_absent();
        assert!(fixture.temp_entries().is_empty());
    }
    assert_eq!(std::env::var_os("TMPDIR"), tmpdir_before);
}

#[test]
#[serial_test::serial]
fn fakes_record_each_tool_argument_shape() {
    let fixture = TtsFixture::new();
    let output = |name: &str| fixture.temp_dir().join(name);
    let text = "This is a test message: quoted 'speech'";
    let cases: [(FakeTool, Vec<std::ffi::OsString>, &str); 4] = [
        (
            FakeTool::Say,
            vec!["-o".into(), output("say.wav").into(), "--file-format=WAVE".into(), "-v".into(), "Samantha".into()],
            "say.wav",
        ),
        (
            FakeTool::KokoroTts,
            vec!["input.txt".into(), output("kokoro.wav").into(), "--voice".into(), "af_heart".into()],
            "kokoro.wav",
        ),
        (
            FakeTool::GttsCli,
            vec!["--output".into(), output("gtts.mp3").into(), "--lang".into(), "en".into(), text.into()],
            "gtts.mp3",
        ),
        (
            FakeTool::Echogarden,
            vec!["speak".into(), text.into(), output("echogarden.wav").into(), "--engine=kokoro".into()],
            "echogarden.wav",
        ),
    ];
    for (tool, args, file) in cases {
        let program = fixture.install(tool, Behavior::Synthesize);
        let mut child = std::process::Command::new(program)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write as _;
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
        assert!(child.wait().unwrap().success());

        let syntheses = fixture.syntheses(tool);
        assert_eq!(syntheses.len(), 1, "{tool:?}");
        let expected_args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect();
        assert_eq!(syntheses[0].args, expected_args, "{tool:?}");
        assert_eq!(syntheses[0].output.as_deref(), Some(output(file).as_path()), "{tool:?}");
        assert_eq!(std::fs::read(output(file)).unwrap(), fake_audio(tool), "{tool:?}");
        let expected_stdin = (tool == FakeTool::Say).then(|| text.to_string());
        assert_eq!(syntheses[0].stdin, expected_stdin, "{tool:?}");
    }

    for (tool, args) in [
        (FakeTool::KokoroTts, vec!["--help-voices"]),
        (FakeTool::Echogarden, vec!["list-voices", "kokoro"]),
    ] {
        let program = fixture.bin_dir().join(tool.program());
        assert!(std::process::Command::new(program).args(&args).status().unwrap().success());
        assert_eq!(fixture.invocations(tool).len(), 2, "{tool:?}");
        assert_eq!(fixture.syntheses(tool).len(), 1, "a listing is not a synthesis: {tool:?}");
    }
}

#[test]
#[serial_test::serial]
fn failing_and_hanging_fakes_leave_partial_output_behind() {
    let fixture = TtsFixture::new();
    let failed = fixture.temp_dir().join("failed.mp3");
    let program = fixture.install(FakeTool::GttsCli, Behavior::FailAfterPartialWrite);
    let result = std::process::Command::new(program)
        .args(["--output".as_ref(), failed.as_os_str(), "--lang".as_ref(), "en".as_ref()])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&result.stderr).trim(), "scripted failure");
    assert_eq!(std::fs::read(&failed).unwrap(), b"partial");

    let hung = fixture.temp_dir().join("hung.wav");
    let program = fixture.install(FakeTool::KokoroTts, Behavior::HangAfterPartialWrite);
    let mut child = std::process::Command::new(program)
        .args(["input.txt".as_ref(), hung.as_os_str()])
        .spawn()
        .unwrap();
    wait_for("the hanging fake to write", || hung.is_file());
    let pid = fixture.syntheses(FakeTool::KokoroTts)[0].pid;
    assert_eq!(pid, child.id());
    assert!(process_alive(pid));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(!process_alive(pid));
}

#[tokio::test]
#[serial_test::serial]
async fn kokoro_synthesizes_once_into_the_fixture_cache() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::KokoroTts, Behavior::Synthesize);
    fixture.install(FakeTool::Mpv, Behavior::Synthesize);
    let config = TtsConfig::new()
        .with_voice("af_heart")
        .with_volume(VolumeLevel::Explicit(0.0));
    let text = "This is a test message: recorded Kokoro";
    for _ in 0..2 {
        let result = KokoroTtsProvider::new().speak(text, &config).await;
        if cfg!(feature = "playa") {
            result.expect("fake player accepts the cached file");
        } else {
            assert!(matches!(result, Err(biscuit_speaks::TtsError::NoAudioPlayer)), "{result:?}");
        }
    }

    let syntheses = fixture.syntheses(FakeTool::KokoroTts);
    assert_eq!(syntheses.len(), 1, "the second call reuses the cached file");
    let cache = fixture.audio_cache_entries();
    assert_eq!(cache.len(), 1);
    assert_eq!(syntheses[0].output.as_ref(), Some(&cache[0]));
    assert_eq!(std::fs::read(&cache[0]).unwrap(), fake_audio(FakeTool::KokoroTts));
    let played = fixture.invocations(FakeTool::Mpv);
    if cfg!(feature = "playa") {
        assert_eq!(played.len(), 2);
        let path = cache[0].to_string_lossy();
        assert!(played.iter().all(|call| call.args.iter().any(|arg| *arg == path)), "{played:?}");
    } else {
        assert!(played.is_empty());
    }
    fixture.assert_spool_absent();
}

#[cfg(feature = "playa")]
#[tokio::test]
#[serial_test::serial]
async fn locked_spool_receives_a_ready_job_for_a_cached_file() {
    use biscuit_speaks::{HostTtsProvider, Speak, TtsFailoverStrategy, TtsProvider};

    let fixture = TtsFixture::new();
    fixture.install(FakeTool::KokoroTts, Behavior::Synthesize);
    let spool = fixture.lock_spool();
    assert_eq!(biscuit_speaks::run_if_worker().await, None);
    let config = TtsConfig::new()
        .with_voice("af_heart")
        .with_volume(VolumeLevel::Explicit(0.0))
        .with_failover(TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(
            HostTtsProvider::KokoroTts,
        )));
    let text = "This is a test message: published Kokoro";
    KokoroTtsProvider::new().detached_job(text, &config).await.unwrap();
    assert_eq!(fixture.syntheses(FakeTool::KokoroTts).len(), 1);

    let id = Speak::new(text).with_config(config).play_detached().await.unwrap();
    let snapshot = playa::detached::snapshot().unwrap();
    assert_eq!(snapshot.pending.len(), 1);
    assert_eq!(snapshot.pending[0].job_id.as_str(), id.as_str());
    assert_eq!(snapshot.pending[0].state, "ready");
    assert_eq!(snapshot.pending[0].source_kind, playa::detached::JournalSourceKind::File);
    assert_eq!(fixture.syntheses(FakeTool::KokoroTts).len(), 1, "a cache hit does not synthesize");

    spool.clear_pending().expect("pending work is removed while owned");
    assert!(playa::detached::snapshot().unwrap().pending.is_empty());
}
