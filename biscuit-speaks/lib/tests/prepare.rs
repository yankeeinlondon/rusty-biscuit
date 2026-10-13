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
    // Synthesis writes a staging sibling that is renamed into place.
    let staged = syntheses[0].output.as_ref().unwrap();
    assert_eq!(staged.parent(), cache[0].parent());
    assert!(!staged.exists());
    assert_eq!(fixture.temp_entries(), cache);
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

// ============================================================================
// Preparation
// ============================================================================

use biscuit_speaks::{
    EchogardenProvider, ESpeakProvider, GttsProvider, HostTtsProvider, SapiProvider, Speak,
    TtsError, TtsFailoverStrategy, TtsProvider,
};

/// A silent configuration pinned to `host`.
fn pinned(host: HostTtsProvider) -> TtsConfig {
    TtsConfig::new()
        .with_volume(VolumeLevel::Explicit(0.0))
        .with_failover(TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(host)))
}

/// Prepare through the provider's own trait method rather than `Speak`.
async fn prepare_directly(host: HostTtsProvider, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
    match host {
        HostTtsProvider::KokoroTts => KokoroTtsProvider::new().prepare(text, config).await,
        HostTtsProvider::Gtts => GttsProvider::new().prepare(text, config).await,
        HostTtsProvider::EchoGarden => EchogardenProvider::new().prepare(text, config).await,
        HostTtsProvider::Say => biscuit_speaks::SayProvider.prepare(text, config).await,
        other => panic!("{other:?} does not produce files"),
    }
}

/// Fail unless `result` is the no-`playa` build refusing to play.
#[track_caller]
fn assert_no_player<T: std::fmt::Debug>(result: Result<T, TtsError>) {
    let refused = match &result {
        Err(TtsError::NoAudioPlayer) => true,
        Err(TtsError::AllProvidersFailed(failed)) => failed
            .errors
            .iter()
            .all(|(_, error)| matches!(error, TtsError::NoAudioPlayer)),
        _ => false,
    };
    assert!(refused, "{result:?}");
}

async fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Two sequential prepares synthesize once, publish one complete file, play
/// nothing, and touch no spool; the returned object and a fresh object then
/// reuse that file through every playback entry point.
async fn prepared_file_is_reused(tool: FakeTool, host: HostTtsProvider, voice: &str) {
    let fixture = TtsFixture::new();
    fixture.install(tool, Behavior::Synthesize);
    fixture.install(FakeTool::Mpv, Behavior::Synthesize);
    let config = pinned(host);
    let text = format!("This is a test message: prepared {host:?}");

    for _ in 0..2 {
        prepare_directly(host, &text, &config).await.expect("direct preparation");
    }
    let prepared = Speak::new(&text).with_config(config.clone()).prepare().await.unwrap();
    assert!(prepared.is_prepared());

    assert_eq!(fixture.syntheses(tool).len(), 1, "sequential prepares synthesize once");
    let cache = fixture.audio_cache_entries();
    assert_eq!(cache.len(), 1);
    assert_eq!(std::fs::read(&cache[0]).unwrap(), fake_audio(tool));
    assert_eq!(fixture.temp_entries(), cache, "no staging file is left behind");
    assert!(fixture.invocations(FakeTool::Mpv).is_empty(), "preparation plays nothing");
    fixture.assert_spool_absent();

    let fresh = || Speak::new(&text).with_config(config.clone());
    let returned = prepared.clone().play().await;
    let fresh_play = fresh().play().await;
    let with_result = fresh().play_with_result().await;
    if cfg!(feature = "playa") {
        returned.expect("the prepared object plays the cached file");
        fresh_play.expect("a fresh object plays the cached file");
        let result = with_result.expect("a fresh object reports the cached file");
        assert_eq!(result.provider, TtsProvider::Host(host));
        assert_eq!(result.voice.name, voice);
        assert!(result.cache_hit);
        assert_eq!(result.audio_file_path.as_deref(), Some(cache[0].as_path()));
        let played = fixture.invocations(FakeTool::Mpv);
        assert_eq!(played.len(), 3);
        let path = cache[0].to_string_lossy();
        assert!(played.iter().all(|call| call.args.iter().any(|arg| *arg == path)), "{played:?}");
    } else {
        assert_no_player(returned);
        assert_no_player(fresh_play);
        assert_no_player(with_result);
    }
    fixture.assert_spool_absent();

    #[cfg(feature = "playa")]
    {
        let spool = fixture.lock_spool();
        assert_eq!(biscuit_speaks::run_if_worker().await, None);
        let id = fresh().play_detached().await.unwrap();
        let snapshot = playa::detached::snapshot().unwrap();
        assert_eq!(snapshot.pending.len(), 1);
        assert_eq!(snapshot.pending[0].job_id.as_str(), id.as_str());
        assert_eq!(snapshot.pending[0].state, "ready");
        spool.clear_pending().expect("pending work is removed while owned");
    }
    assert_eq!(fixture.syntheses(tool).len(), 1, "every reuse was a cache hit");
}

#[tokio::test]
#[serial_test::serial]
async fn kokoro_prepared_file_is_reused() {
    prepared_file_is_reused(FakeTool::KokoroTts, HostTtsProvider::KokoroTts, "af_heart").await;
}

#[tokio::test]
#[serial_test::serial]
async fn gtts_prepared_file_is_reused() {
    // The fake lists no languages, so the metadata falls back to the code.
    prepared_file_is_reused(FakeTool::GttsCli, HostTtsProvider::Gtts, "en").await;
}

#[tokio::test]
#[serial_test::serial]
async fn echogarden_prepared_file_is_reused() {
    prepared_file_is_reused(FakeTool::Echogarden, HostTtsProvider::EchoGarden, "Heart").await;
}

#[tokio::test]
#[serial_test::serial]
async fn synthesis_settings_change_the_key_and_volume_does_not() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::KokoroTts, Behavior::Synthesize);
    fixture.install(FakeTool::Mpv, Behavior::Synthesize);
    let text = "This is a test message: key sensitivity";
    let config = pinned(HostTtsProvider::KokoroTts);
    Speak::new(text).with_config(config.clone()).prepare().await.unwrap();

    let louder = Speak::new(text)
        .with_config(config.clone())
        .with_volume(VolumeLevel::Explicit(0.5))
        .prepare()
        .await
        .unwrap();
    assert!(louder.is_prepared());
    assert_eq!(fixture.syntheses(FakeTool::KokoroTts).len(), 1, "volume is not in the key");

    let other_voice = config.clone().with_voice("am_adam");
    Speak::new(text).with_config(other_voice).prepare().await.unwrap();
    let syntheses = fixture.syntheses(FakeTool::KokoroTts);
    assert_eq!(syntheses.len(), 2, "a different voice is a different file");
    assert_eq!(fixture.audio_cache_entries().len(), 2);
    assert!(syntheses.iter().any(|call| call.args.contains(&"am_adam".to_string())));

    if cfg!(feature = "playa") {
        louder.play().await.unwrap();
        let played = fixture.invocations(FakeTool::Mpv);
        assert!(played[0].args.contains(&"--volume=50".to_string()), "{played:?}");
    }
}

#[cfg(feature = "playa")]
#[tokio::test]
#[serial_test::serial]
async fn baked_speed_is_applied_once_and_playback_speed_is_applied_at_playback() {
    use biscuit_speaks::SpeedLevel;

    let fixture = TtsFixture::new();
    fixture.install(FakeTool::Echogarden, Behavior::Synthesize);
    fixture.install(FakeTool::KokoroTts, Behavior::Synthesize);
    fixture.install(FakeTool::Mpv, Behavior::Synthesize);
    let text = "This is a test message: speed";
    let fast = TtsConfig::new()
        .with_volume(VolumeLevel::Explicit(0.0))
        .with_speed(SpeedLevel::Fast);

    EchogardenProvider::new().prepare(text, &fast).await.unwrap();
    EchogardenProvider::new().speak(text, &fast).await.unwrap();
    let synthesis = fixture.syntheses(FakeTool::Echogarden);
    assert_eq!(synthesis.len(), 1);
    assert!(synthesis[0].args.contains(&"--speed=1.25".to_string()), "{synthesis:?}");
    let played = fixture.invocations(FakeTool::Mpv);
    assert!(played[0].args.contains(&"--speed=1".to_string()), "{played:?}");

    KokoroTtsProvider::new().prepare(text, &fast).await.unwrap();
    KokoroTtsProvider::new().speak(text, &fast).await.unwrap();
    let synthesis = fixture.syntheses(FakeTool::KokoroTts);
    assert_eq!(synthesis.len(), 1);
    assert!(!synthesis[0].args.iter().any(|arg| arg.contains("speed")), "{synthesis:?}");
    let played = fixture.invocations(FakeTool::Mpv);
    assert_eq!(played.len(), 2);
    assert!(
        played.iter().any(|call| call.args.contains(&"--speed=1.25".to_string())),
        "{played:?}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn dry_run_prepares_nothing_before_any_discovery() {
    let fixture = TtsFixture::new();
    let tools = [FakeTool::KokoroTts, FakeTool::GttsCli, FakeTool::Echogarden, FakeTool::Say, FakeTool::Osascript];
    for tool in tools {
        fixture.install(tool, Behavior::Synthesize);
    }
    fixture.set_system_voice_lookup(Some(r#"{"voice":"Ava (Premium)","rate":175}"#));
    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "1");
    let text = "This is a test message: dry run";

    let unavailable = Speak::new(text)
        .with_config(pinned(HostTtsProvider::Pico2Wave))
        .prepare()
        .await
        .expect("dry run succeeds without looking for a provider");
    assert!(!unavailable.is_prepared());
    for host in [
        HostTtsProvider::KokoroTts,
        HostTtsProvider::Gtts,
        HostTtsProvider::EchoGarden,
        HostTtsProvider::Say,
    ] {
        let speak = Speak::new(text).with_config(pinned(host)).prepare().await.unwrap();
        assert!(!speak.is_prepared(), "{host:?}");
        prepare_directly(host, text, &pinned(host)).await.unwrap();
    }

    for tool in tools {
        assert!(fixture.invocations(tool).is_empty(), "{tool:?} must not run");
    }
    assert!(fixture.temp_entries().is_empty());
    assert!(!fixture.capability_cache().exists(), "no capability discovery");
    fixture.assert_spool_absent();
}

#[tokio::test]
#[serial_test::serial]
async fn dry_run_keeps_an_existing_preparation() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::KokoroTts, Behavior::Synthesize);
    let text = "This is a test message: retained under dry run";
    let prepared = Speak::new(text)
        .with_config(pinned(HostTtsProvider::KokoroTts))
        .prepare()
        .await
        .unwrap();
    for entry in fixture.audio_cache_entries() {
        std::fs::remove_file(entry).unwrap();
    }

    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "yes");
    let again = prepared.prepare().await.unwrap();
    assert!(again.is_prepared(), "dry run returns the builder unchanged");
    assert_eq!(fixture.syntheses(FakeTool::KokoroTts).len(), 1);
    assert!(fixture.temp_entries().is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn an_unresolvable_echogarden_default_is_a_no_op_and_plays_uncached() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::Echogarden, Behavior::Synthesize);
    fixture.install(FakeTool::Mpv, Behavior::Synthesize);
    // No requested voice, a non-English language, and a voice list with no
    // match leave echogarden to choose its own default.
    let config = pinned(HostTtsProvider::EchoGarden)
        .with_language(biscuit_speaks::Language::Custom("ja".into()));
    let text = "This is a test message: unresolved";

    let speak = Speak::new(text).with_config(config.clone()).prepare().await.unwrap();
    assert!(!speak.is_prepared());
    assert!(fixture.syntheses(FakeTool::Echogarden).is_empty());
    let listings = fixture.invocations(FakeTool::Echogarden);
    assert_eq!(listings.len(), 1);
    assert_eq!(listings[0].args, ["list-voices", "kokoro"]);
    assert!(fixture.temp_entries().is_empty());

    for _ in 0..2 {
        let result = Speak::new(text).with_config(config.clone()).play().await;
        if cfg!(feature = "playa") {
            result.unwrap();
        } else {
            assert_no_player(result);
        }
    }
    let syntheses = fixture.syntheses(FakeTool::Echogarden);
    assert_eq!(syntheses.len(), 2, "unresolved audio is never reused");
    for synthesis in &syntheses {
        assert!(!synthesis.args.iter().any(|arg| arg.starts_with("--voice")), "{synthesis:?}");
        let name = synthesis.output.as_ref().unwrap().file_name().unwrap().to_string_lossy().into_owned();
        assert!(!name.starts_with("biscuit-speaks-"), "not a cache entry: {name}");
    }
    assert!(fixture.temp_entries().is_empty(), "uncached audio is deleted after playback");
    fixture.assert_spool_absent();
}

#[tokio::test]
#[serial_test::serial]
async fn the_first_candidates_error_propagates_without_trying_another() {
    let fixture = TtsFixture::new();
    let _default_order = test_toolkit::EnvGuard::remove_safe("TTS_PROVIDER");
    fixture.install(FakeTool::KokoroTts, Behavior::FailAfterPartialWrite);
    fixture.install(FakeTool::Echogarden, Behavior::Synthesize);
    let config = TtsConfig::new()
        .with_volume(VolumeLevel::Explicit(0.0))
        .with_failover(TtsFailoverStrategy::FirstAvailable);
    assert_eq!(
        biscuit_speaks::get_providers_for_strategy(&config.failover_strategy).first(),
        Some(&TtsProvider::Host(HostTtsProvider::KokoroTts)),
        "Kokoro leads the default stack"
    );

    let result = Speak::new("This is a test message: failing").with_config(config).prepare().await;
    assert!(
        matches!(&result, Err(TtsError::ProcessFailed { provider, stderr }) if provider == "kokoro-tts" && stderr.contains("scripted failure")),
        "{result:?}"
    );
    assert_eq!(fixture.syntheses(FakeTool::KokoroTts).len(), 1);
    assert!(fixture.invocations(FakeTool::Echogarden).is_empty(), "no other provider is prepared");
    assert!(fixture.temp_entries().is_empty(), "the partial output is not published");
    fixture.assert_spool_absent();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn cancelled_preparation_kills_the_child_and_publishes_nothing() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::GttsCli, Behavior::HangAfterPartialWrite);
    let config = pinned(HostTtsProvider::Gtts);
    let task = tokio::spawn(async move {
        Speak::new("This is a test message: cancelled").with_config(config).prepare().await
    });

    wait_until("the hanging fake to start", || !fixture.syntheses(FakeTool::GttsCli).is_empty()).await;
    let synthesis = fixture.syntheses(FakeTool::GttsCli).remove(0);
    wait_until("the partial write", || synthesis.output.as_ref().unwrap().is_file()).await;
    assert!(common::process_running(synthesis.pid));

    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    wait_until("the child to be killed", || !common::process_running(synthesis.pid)).await;
    assert!(fixture.temp_entries().is_empty(), "{:?}", fixture.temp_entries());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn concurrent_preparation_publishes_a_complete_file() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::KokoroTts, Behavior::Synthesize);
    let text = "This is a test message: concurrent";
    let config = pinned(HostTtsProvider::KokoroTts);
    let (first, second) = tokio::join!(
        Speak::new(text).with_config(config.clone()).prepare(),
        Speak::new(text).with_config(config.clone()).prepare(),
    );
    assert!(first.unwrap().is_prepared() && second.unwrap().is_prepared());

    // Simultaneous callers may both synthesize; only the result is asserted.
    assert!((1..=2).contains(&fixture.syntheses(FakeTool::KokoroTts).len()));
    let cache = fixture.audio_cache_entries();
    assert_eq!(cache.len(), 1);
    assert_eq!(std::fs::read(&cache[0]).unwrap(), fake_audio(FakeTool::KokoroTts));
    assert_eq!(fixture.temp_entries(), cache);
}

#[tokio::test]
#[serial_test::serial]
async fn direct_speakers_prepare_as_a_no_op() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::ESpeakNg, Behavior::Synthesize);
    let text = "This is a test message: direct speech";
    let config = pinned(HostTtsProvider::ESpeak);

    ESpeakProvider::new().prepare(text, &config).await.unwrap();
    SapiProvider::new().prepare(text, &config).await.unwrap();
    let speak = Speak::new(text).with_config(config).prepare().await.unwrap();
    assert!(!speak.is_prepared(), "nothing was prepared");

    let spoken: Vec<_> = fixture
        .invocations(FakeTool::ESpeakNg)
        .into_iter()
        .filter(|call| call.args.iter().any(|arg| arg.contains(text)))
        .collect();
    assert!(spoken.is_empty(), "{spoken:?}");
    assert!(fixture.temp_entries().is_empty());
    fixture.assert_spool_absent();
}

#[tokio::test]
#[serial_test::serial]
async fn an_enum_only_provider_reports_that_it_is_not_implemented() {
    let fixture = TtsFixture::new();
    fixture.install(FakeTool::Piper, Behavior::Synthesize);
    let result = Speak::new("This is a test message: piper")
        .with_config(pinned(HostTtsProvider::Piper))
        .prepare()
        .await;
    assert!(
        matches!(&result, Err(TtsError::ProviderFailed { provider, message }) if provider == "Piper" && message == "Provider not yet implemented"),
        "{result:?}"
    );
    assert!(fixture.invocations(FakeTool::Piper).is_empty());
}

// ============================================================================
// Say: resolved system voice, unresolved defaults
// ============================================================================

use biscuit_speaks::{SayProvider, SpeedLevel};

const AVA: &str = "Ava (Premium)       en_US    # Hello! My name is Ava.";
const SAMANTHA: &str = "Samantha            en_US    # Hello! My name is Samantha.";

/// A Say fixture whose system-voice lookup reports `voice` at 175 WPM.
fn say_fixture(voice: &str) -> TtsFixture {
    let fixture = TtsFixture::new();
    for tool in [FakeTool::Say, FakeTool::Osascript, FakeTool::Mpv, FakeTool::Afplay] {
        fixture.install(tool, Behavior::Synthesize);
    }
    fixture.set_say_voices(&[AVA, SAMANTHA]);
    fixture.set_system_voice_lookup(Some(&format!(r#"{{"voice":"{voice}","rate":175}}"#)));
    fixture
}

/// The player that recorded `path` being played, for the build's feature set.
fn players(fixture: &TtsFixture) -> Vec<common::Invocation> {
    let player = if cfg!(feature = "playa") { FakeTool::Mpv } else { FakeTool::Afplay };
    fixture.invocations(player)
}

fn has_args(args: &[String], expected: &[&str]) -> bool {
    args.windows(expected.len()).any(|window| window == expected)
}

/// With no requested voice, an explicit speed and a resolvable system voice
/// prepare once; a fresh object then reuses the file through ordinary
/// playback (with and without `playa`), `speak_with_result`, and the detached
/// cache lookup.
#[tokio::test]
#[serial_test::serial]
async fn say_prepares_the_system_voice_and_reuses_it_everywhere() {
    let fixture = say_fixture("Ava (Premium)");
    let config = TtsConfig::new()
        .with_volume(VolumeLevel::Explicit(0.0))
        .with_speed(SpeedLevel::Fast);
    let text = "This is a test message: prepared Say";

    for _ in 0..2 {
        SayProvider.prepare(text, &config).await.expect("direct preparation");
    }
    let syntheses = fixture.syntheses(FakeTool::Say);
    assert_eq!(syntheses.len(), 1, "sequential prepares synthesize once");
    assert!(has_args(&syntheses[0].args, &["-v", "Ava (Premium)"]), "{syntheses:?}");
    assert!(has_args(&syntheses[0].args, &["-r", "219"]), "{syntheses:?}");
    assert_eq!(syntheses[0].stdin.as_deref(), Some(text));
    let cache = fixture.audio_cache_entries();
    assert_eq!(cache.len(), 1);
    assert_eq!(std::fs::read(&cache[0]).unwrap(), fake_audio(FakeTool::Say));
    assert_eq!(fixture.temp_entries(), cache, "no staging file is left behind");
    assert!(players(&fixture).is_empty(), "preparation plays nothing");
    fixture.assert_spool_absent();

    SayProvider.speak(text, &config).await.expect("ordinary playback of the cached file");
    let result = SayProvider.speak_with_result(text, &config).await.unwrap();
    assert_eq!(result.provider, TtsProvider::Host(HostTtsProvider::Say));
    assert_eq!(result.voice.name, "Ava (Premium)");
    assert!(result.cache_hit);
    assert_eq!(result.audio_file_path.as_deref(), Some(cache[0].as_path()));

    let played = players(&fixture);
    assert_eq!(played.len(), 2, "{played:?}");
    let path = cache[0].to_string_lossy();
    assert!(played.iter().all(|call| call.args.iter().any(|arg| *arg == path)), "{played:?}");
    if cfg!(feature = "playa") {
        assert!(played.iter().all(|call| call.args.contains(&"--speed=1".to_string())), "{played:?}");
    } else {
        assert!(played.iter().all(|call| has_args(&call.args, &["-v", "0"])), "{played:?}");
    }

    #[cfg(feature = "playa")]
    {
        let job = SayProvider.cached_detached_job(text, &config).await.unwrap();
        assert!(job.is_some(), "the detached lookup finds the prepared file");
    }
    assert_eq!(fixture.syntheses(FakeTool::Say).len(), 1, "every reuse was a cache hit");
    fixture.assert_spool_absent();
}

/// A changed system voice makes a fresh object miss the old entry and
/// synthesize with the new voice.
#[tokio::test]
#[serial_test::serial]
async fn a_changed_system_voice_makes_a_fresh_say_object_miss() {
    let fixture = say_fixture("Ava (Premium)");
    let config = TtsConfig::new()
        .with_volume(VolumeLevel::Explicit(0.0))
        .with_speed(SpeedLevel::Fast);
    let text = "This is a test message: changed default";
    SayProvider.prepare(text, &config).await.unwrap();

    fixture.set_system_voice_lookup(Some(r#"{"voice":"Samantha","rate":175}"#));
    #[cfg(feature = "playa")]
    assert!(SayProvider.cached_detached_job(text, &config).await.unwrap().is_none());
    SayProvider.prepare(text, &config).await.unwrap();
    let syntheses = fixture.syntheses(FakeTool::Say);
    assert_eq!(syntheses.len(), 2);
    assert!(syntheses.iter().any(|call| has_args(&call.args, &["-v", "Samantha"])), "{syntheses:?}");
    assert_eq!(fixture.audio_cache_entries().len(), 2, "each voice has its own entry");
}

/// Normal speed leaves the system rate unresolved, and an unrecognized lookup
/// or an unlisted voice leaves the system voice unresolved. None of them
/// prepares; playback synthesizes with only the known settings each time,
/// never through the shared cache, and ignores a legacy `system-default`
/// entry for the same text.
#[tokio::test]
#[serial_test::serial]
async fn unresolved_say_defaults_prepare_nothing_and_never_use_the_cache() {
    let fixture = say_fixture("Ava (Premium)");
    let text = "This is a test message: unresolved Say";
    let legacy = biscuit_speaks::audio_cache::CacheKey::new(
        "say",
        "system-default;rate=system-default",
        text,
        "wav",
    )
    .cache_path();
    std::fs::write(&legacy, b"legacy").unwrap();
    let silent = TtsConfig::new().with_volume(VolumeLevel::Explicit(0.0));

    // Normal speed: the rate is unresolved, so no lookup runs at all.
    for config in [silent.clone(), silent.clone().with_gender(biscuit_speaks::Gender::Male)] {
        SayProvider.prepare(text, &config).await.unwrap();
    }
    assert!(fixture.invocations(FakeTool::Osascript).is_empty());
    assert!(fixture.invocations(FakeTool::Say).is_empty());

    // An explicit rate with an unrecognized lookup or an unlisted voice.
    let fast = silent.clone().with_speed(SpeedLevel::Fast);
    for lookup in [Some("not json"), Some(r#"{"voice":"Zarvox","rate":175}"#), None] {
        fixture.set_system_voice_lookup(lookup);
        SayProvider.prepare(text, &fast).await.unwrap();
    }
    assert_eq!(fixture.invocations(FakeTool::Osascript).len(), 3);
    assert!(fixture.syntheses(FakeTool::Say).is_empty());
    #[cfg(feature = "playa")]
    assert!(SayProvider.cached_detached_job(text, &fast).await.unwrap().is_none());

    for config in [&silent, &silent.clone().with_gender(biscuit_speaks::Gender::Male), &fast] {
        for _ in 0..2 {
            SayProvider.speak(text, config).await.expect("uncached playback");
        }
    }
    let syntheses = fixture.syntheses(FakeTool::Say);
    assert_eq!(syntheses.len(), 6, "unresolved audio is never reused");
    let with_voice = syntheses.iter().filter(|call| has_args(&call.args, &["-v", "Alex"])).count();
    let with_rate = syntheses.iter().filter(|call| has_args(&call.args, &["-r", "219"])).count();
    assert_eq!((with_voice, with_rate), (2, 2), "{syntheses:?}");
    for synthesis in &syntheses {
        let name = synthesis.output.as_ref().unwrap().file_name().unwrap().to_string_lossy().into_owned();
        assert!(!name.starts_with("biscuit-speaks-"), "not a cache entry: {name}");
    }
    let played = players(&fixture);
    assert_eq!(played.len(), 6);
    let legacy_path = legacy.to_string_lossy();
    assert!(!played.iter().any(|call| call.args.iter().any(|arg| *arg == legacy_path)), "{played:?}");
    assert_eq!(fixture.temp_entries(), [legacy], "uncached audio is deleted after playback");
    fixture.assert_spool_absent();
}

#[tokio::test]
#[serial_test::serial]
async fn a_failed_say_synthesis_publishes_nothing() {
    let fixture = say_fixture("Ava (Premium)");
    fixture.install(FakeTool::Say, Behavior::FailAfterPartialWrite);
    let config = TtsConfig::new().with_voice("Samantha").with_speed(SpeedLevel::Slow);
    let result = SayProvider.prepare("This is a test message: failing Say", &config).await;
    assert!(
        matches!(&result, Err(TtsError::ProcessFailed { provider, stderr }) if provider == "say" && stderr.contains("scripted failure")),
        "{result:?}"
    );
    assert_eq!(fixture.syntheses(FakeTool::Say).len(), 1);
    assert!(fixture.temp_entries().is_empty(), "{:?}", fixture.temp_entries());
}

/// `Speak` selects Say only on macOS.
#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn speak_retains_say_preparation_only_when_both_settings_resolve() {
    let fixture = say_fixture("Ava (Premium)");
    let text = "This is a test message: Speak with Say";
    let fast = Speak::new(text)
        .with_config(pinned(HostTtsProvider::Say).with_speed(SpeedLevel::Fast))
        .prepare()
        .await
        .unwrap();
    assert!(fast.is_prepared());
    let normal = Speak::new(text).with_config(pinned(HostTtsProvider::Say)).prepare().await.unwrap();
    assert!(!normal.is_prepared(), "the system rate is unresolved");
    assert_eq!(fixture.syntheses(FakeTool::Say).len(), 1);
}
