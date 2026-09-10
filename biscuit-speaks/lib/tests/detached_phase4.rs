#![cfg(feature = "playa")]

use std::ffi::OsString;
use std::fs::OpenOptions;

use biscuit_speaks::{
    CloudTtsProvider, ESpeakProvider, EchogardenProvider, ElevenLabsProvider, Gender,
    GttsProvider, HostTtsProvider, KokoroTtsProvider, Language, Speak, SpeakPlaybackReport,
    SpeakResult, SpeedLevel, TtsConfig, TtsError, TtsExecutor, TtsFailoverStrategy, TtsProvider,
    Voice, VolumeLevel, run_if_worker,
};
use fs4::fs_std::FileExt as _;
use playa::detached::{OsValue, SpoolJob};
use playa::{AudioPlayer, PlaybackReport, PlaybackRoute, PlaybackVerdict};

#[test]
fn playa_feature_enables_native_playback() {
    let route = PlaybackRoute::Native;
    let _job = playa::detached::JobId::dry_run();
    assert_eq!(route, PlaybackRoute::Native);
}

#[test]
fn speak_result_playback_serde() {
    let old = r#"{
        "provider":{"host":"say"},
        "voice":{"name":"Samantha","gender":"female","quality":"good","languages":["english"],"identifier":null,"description":null,"priority":0,"model_file":null}
    }"#;
    let old_result: SpeakResult = serde_json::from_str(old).expect("old stored result must parse");
    assert_eq!(old_result.playback, None);

    let playback = SpeakPlaybackReport::from(PlaybackReport {
        route: PlaybackRoute::Host(AudioPlayer::Mpv),
        expected: Some(std::time::Duration::from_millis(5_970)),
        elapsed: std::time::Duration::from_millis(6_330),
        verdict: PlaybackVerdict::Complete,
    });
    let original = SpeakResult::new(
        TtsProvider::Host(HostTtsProvider::Say),
        Voice::new("Samantha"),
    )
    .with_playback(playback);
    let first = serde_json::to_vec(&original).expect("result must serialize");
    let read: SpeakResult = serde_json::from_slice(&first).expect("result must deserialize");
    let second = serde_json::to_vec(&read).expect("read result must serialize again");
    let reread: SpeakResult = serde_json::from_slice(&second).expect("second read must succeed");
    assert_eq!(reread, original);
}

#[tokio::test]
async fn espeak_detached_job_preserves_original_text_and_arguments() {
    let executable = std::env::current_exe().expect("test executable path");
    let provider = ESpeakProvider::with_binary(executable.to_string_lossy());
    let config = TtsConfig::new()
        .with_voice("en+f3")
        .with_language(Language::English)
        .with_gender(Gender::Female)
        .with_speed(SpeedLevel::Fast)
        .with_volume(VolumeLevel::Soft);
    let original = "This is a test message.";

    let job = provider
        .detached_job(original, &config)
        .await
        .expect("streaming provider should produce a command job");
    let SpoolJob::Command { program, args } = job else {
        panic!("eSpeak must produce a direct command job");
    };
    assert_eq!(program.to_os_string().unwrap(), executable.into_os_string());
    let args = args
        .iter()
        .map(OsValue::to_os_string)
        .collect::<Result<Vec<OsString>, _>>()
        .unwrap();
    assert_eq!(
        args,
        ["-v", "en+f3", "-s", "219", "-a", "50", original]
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
    );
}

#[test]
fn tts_config_round_trips_for_private_preparation() {
    let original = TtsConfig::new()
        .with_voice("af_heart")
        .with_model("kokoro-v1.0")
        .with_gender(Gender::Female)
        .with_language(Language::Custom("en-US".into()))
        .with_volume(VolumeLevel::Explicit(0.42))
        .with_speed(SpeedLevel::Explicit(1.15))
        .with_failover(TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(
            HostTtsProvider::KokoroTts,
        )));
    let first = serde_json::to_vec(&original).unwrap();
    let read: TtsConfig = serde_json::from_slice(&first).unwrap();
    let second = serde_json::to_vec(&read).unwrap();
    let reread: TtsConfig = serde_json::from_slice(&second).unwrap();
    assert_eq!(reread, original);
}

#[test]
fn preparation_config_defaults_missing_fields_and_rejects_malformed_values() {
    let missing: TtsConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, TtsConfig::default());

    let malformed = serde_json::from_str::<TtsConfig>(r#"{"volume":{"explicit":"loud"}}"#);
    assert!(malformed.is_err());
}

#[test]
fn shipped_preparation_config_corpus_remains_compatible() {
    let shipped = include_str!("fixtures/v1-preparation-config.json");
    let parsed: TtsConfig = serde_json::from_str(shipped).unwrap();
    assert_eq!(parsed.requested_voice.as_deref(), Some("af_heart"));
    assert_eq!(parsed.language, Language::Custom("en-US".into()));
    assert_eq!(parsed.volume, VolumeLevel::Explicit(0.42));
    assert!(matches!(
        parsed.failover_strategy,
        TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(HostTtsProvider::KokoroTts))
    ));
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn say_detached_job_preserves_lossless_arguments() {
    use biscuit_speaks::SayProvider;
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("say");
    std::fs::write(&script, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$SAY_TEST_ARGS\"\n/bin/cat > \"$SAY_TEST_TEXT\"\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", temp.path());
    let args_file = temp.path().join("args");
    let text_file = temp.path().join("text");
    let _args = test_toolkit::EnvGuard::set_safe("SAY_TEST_ARGS", &args_file);
    let _text = test_toolkit::EnvGuard::set_safe("SAY_TEST_TEXT", &text_file);
    let original = format!("This is a test message: quoted 'speech' and unicode 世界 {}", std::process::id());
    for volume in [0.0, 0.42] {
        let config = TtsConfig::new().with_voice("Samantha")
            .with_speed(SpeedLevel::Slow).with_volume(VolumeLevel::Explicit(volume));
        let SpoolJob::PlayFile { path, playback, delete_after } = SayProvider
            .detached_job(&original, &config).await.unwrap() else {
                panic!("say must synthesize a volume-controlled file job");
            };
        let path = std::path::PathBuf::from(path.to_os_string().unwrap());
        assert_eq!(std::fs::read_to_string(&text_file).unwrap(), original);
        let args = std::fs::read_to_string(&args_file).unwrap();
        let recorded: Vec<_> = args.lines().collect();
        assert_eq!(recorded[0], "-o");
        assert_eq!(&recorded[2..], ["--file-format=WAVE", "--data-format=LEI16", "-v", "Samantha", "-r", "131"]);
        assert_eq!(playback.options.volume, Some(volume));
        assert_eq!(playback.options.speed, Some(1.0));
        assert!(!delete_after);
        assert!(!std::path::Path::new(args.lines().nth(1).unwrap()).exists());
        let cached = SayProvider.cached_detached_job(&original, &config).await.unwrap().unwrap();
        let SpoolJob::PlayFile { playback, .. } = cached else { panic!("expected cache job") };
        assert_eq!(playback.options.volume, Some(volume));
        assert_eq!(playback.options.speed, Some(1.0));
        assert!(SayProvider.cached_detached_job(&original, &config.clone().with_voice("Alex")).await.unwrap().is_none());
        assert!(SayProvider.cached_detached_job(&original, &config.clone().with_speed(SpeedLevel::Fast)).await.unwrap().is_none());
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn sapi_detached_job_is_direct_and_lossless() {
    use biscuit_speaks::SapiProvider;

    let original = "This is a test message: quoted 'speech' and unicode 世界";
    for volume in [0.0, 0.42] {
        let config = TtsConfig::new().with_voice("test voice").with_volume(VolumeLevel::Explicit(volume));
        let SpoolJob::Command { program, args } = SapiProvider::new()
            .detached_job(original, &config).await.unwrap() else {
                panic!("SAPI must produce a direct command job");
            };
        assert!(std::path::PathBuf::from(program.to_os_string().unwrap()).is_absolute());
        let args: Vec<_> = args.iter().map(|arg| arg.to_os_string().unwrap()).collect();
        assert!(args.iter().any(|arg| arg == original));
        assert_eq!(args.last().unwrap(), &std::ffi::OsString::from(volume.to_string()));
        assert!(args[3].to_string_lossy().contains("$s.Volume = [Math]::Round([double]$args[3] * 100.0)"));
    }
}

fn assert_cached_file(job: SpoolJob, expected: &std::path::Path) {
    let SpoolJob::PlayFile {
        path,
        playback,
        delete_after,
    } = job
    else {
        panic!("file producer must return a play-file job");
    };
    assert_eq!(path.to_os_string().unwrap(), expected.as_os_str());
    assert_eq!(playback.options.volume, Some(0.0));
    assert_eq!(playback.options.speed, Some(1.0));
    assert!(!delete_after);
}

#[tokio::test]
async fn detached_file_provider_table_returns_ready_cache_jobs() {
    let text = format!(
        "This is a test message: cache provider table {} {}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let config = TtsConfig::new().with_voice("af_heart").with_volume(VolumeLevel::Explicit(0.0));
    let cases = [
        (
            biscuit_speaks::audio_cache::CacheKey::new("kokoro", "af_heart", &text, "wav"),
            HostTtsProvider::KokoroTts,
        ),
        (
            biscuit_speaks::audio_cache::CacheKey::new(
                "echogarden",
                "kokoro:af_heart",
                &text,
                "wav",
            ),
            HostTtsProvider::EchoGarden,
        ),
        (
            biscuit_speaks::audio_cache::CacheKey::new("gtts", "af_heart", &text, "mp3"),
            HostTtsProvider::Gtts,
        ),
    ];

    for (key, provider) in cases {
        let path = key.cache_path();
        std::fs::write(&path, b"cached audio").unwrap();
        let job = match provider {
            HostTtsProvider::KokoroTts => KokoroTtsProvider::new()
                .cached_detached_job(&text, &config)
                .await
                .unwrap(),
            HostTtsProvider::EchoGarden => EchogardenProvider::new()
                .cached_detached_job(&text, &config)
                .await
                .unwrap(),
            HostTtsProvider::Gtts => GttsProvider::new()
                .cached_detached_job(&text, &config)
                .await
                .unwrap(),
            _ => unreachable!(),
        }
        .expect("cache hit must be immediately ready");
        assert_cached_file(job, &path);
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
#[serial_test::serial]
async fn elevenlabs_cached_job_is_ready_without_network() {
    let _api_key = test_toolkit::EnvGuard::set_safe("ELEVEN_LABS_API_KEY", "phase-4-test-key");
    let text = format!("This is a test message: ElevenLabs cache {}", std::process::id());
    let config = TtsConfig::new()
        .with_voice("voice-id")
        .with_model("model-id").with_volume(VolumeLevel::Explicit(0.0));
    let key = biscuit_speaks::audio_cache::CacheKey::new(
        "elevenlabs",
        "voice-id-model-id",
        &text,
        "mp3",
    );
    let path = key.cache_path();
    std::fs::write(&path, b"cached audio").unwrap();
    let provider = ElevenLabsProvider::new().unwrap();
    let job = provider
        .cached_detached_job(&text, &config)
        .await
        .unwrap()
        .expect("cache hit must not call the network");
    assert_cached_file(job, &path);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn enum_only_provider_is_explicitly_unsupported() {
    struct EnumOnly;
    impl TtsExecutor for EnumOnly {
        async fn speak(&self, _text: &str, _config: &TtsConfig) -> Result<(), TtsError> {
            Ok(())
        }

        async fn speak_with_result(
            &self,
            _text: &str,
            _config: &TtsConfig,
        ) -> Result<SpeakResult, TtsError> {
            unreachable!()
        }

        fn info(&self) -> &str {
            "Festival"
        }
    }
    let error = EnumOnly
        .detached_job("not synthesized", &TtsConfig::new())
        .await
        .expect_err("enum-only provider must not silently fall back");
    assert!(matches!(error, TtsError::DetachedUnsupported { provider } if provider == "Festival"));
}

#[test]
fn cloud_provider_enum_remains_serializable_in_preparation_config() {
    let config = TtsConfig::new().with_failover(TtsFailoverStrategy::SpecificProvider(
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
    ));
    let value = serde_json::to_value(config).unwrap();
    assert_eq!(value["failover_strategy"]["specific_provider"]["cloud"], "eleven_labs");
}

struct PendingJobsGuard<'a>(&'a std::path::Path);

impl Drop for PendingJobsGuard<'_> {
    fn drop(&mut self) {
        if let Ok(entries) = std::fs::read_dir(self.0) {
            for entry in entries.filter_map(Result::ok) {
                if entry.file_name().to_string_lossy().ends_with(".pending.json") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn play_detached_uses_foreground_specific_provider_selection() {
    let temp = tempfile::tempdir().unwrap();
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let kokoro_executable = if cfg!(windows) {
        bin.join("kokoro-tts.exe")
    } else {
        bin.join("kokoro-tts")
    };
    std::fs::copy(std::env::current_exe().unwrap(), kokoro_executable).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())),
    )
    .unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", path);
    let root = temp.path().join("spool");
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        std::fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    }
    #[cfg(windows)]
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(root.join("files")).unwrap();
    std::fs::create_dir(root.join("requests")).unwrap();
    let worker = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("worker.lock"))
        .unwrap();
    assert!(worker.try_lock_exclusive().unwrap());
    // Pending jobs are removed on unwind before the worker lock can be released.
    let pending = PendingJobsGuard(&root);
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &root);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(run_if_worker().await, None);

    let text = format!("This is a test message: specific cached Kokoro {}", std::process::id());
    let config: TtsConfig = serde_json::from_str(include_str!(
        "fixtures/v1-preparation-config.json"
    ))
    .unwrap();
    let key = biscuit_speaks::audio_cache::CacheKey::new("kokoro", "af_heart", &text, "wav");
    let cache = key.cache_path();
    std::fs::write(&cache, b"cached audio").unwrap();
    let id = Speak::new(&text)
        .with_config(config)
        .play_detached()
        .await
        .unwrap();
    let snapshot = playa::detached::snapshot().unwrap();
    assert_eq!(snapshot.pending.len(), 1);
    assert_eq!(snapshot.pending[0].job_id.as_str(), id.as_str());
    assert_eq!(snapshot.pending[0].state, "ready");
    assert_eq!(snapshot.pending[0].source_kind, playa::detached::JournalSourceKind::File);

    std::fs::remove_file(cache).unwrap();
    for entry in std::fs::read_dir(&root).unwrap().filter_map(Result::ok) {
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(".pending.json") {
            std::fs::remove_file(entry.path()).unwrap();
        }
    }
    drop(pending);
    fs4::fs_std::FileExt::unlock(&worker).unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn say_cache_separates_system_rate_and_exact_synthesized_wpm() {
    use biscuit_speaks::SayProvider;
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("say");
    std::fs::write(&script, r#"#!/bin/sh
rate=system-default
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) output="$2"; shift 2 ;;
    -r) rate="$2"; shift 2 ;;
    *) shift ;;
  esac
done
/bin/cat > /dev/null
printf '%s' "$rate" > "$output"
"#).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", temp.path());
    let text = format!("This is a test message: rate identity {}", std::process::id());
    let base = TtsConfig::new().with_voice("Samantha").with_volume(VolumeLevel::Explicit(0.0));
    let mut paths = Vec::new();
    for (speed, expected) in [
        (SpeedLevel::Normal, "system-default"),
        (SpeedLevel::Explicit(1.0), "175"),
        (SpeedLevel::Explicit(1.0028), "175"),
        (SpeedLevel::Explicit(1.0029), "176"),
    ] {
        let config = base.clone().with_speed(speed);
        let SpoolJob::PlayFile { path, playback, .. } = SayProvider.detached_job(&text, &config).await.unwrap() else {
            panic!("expected synthesized file");
        };
        let path = std::path::PathBuf::from(path.to_os_string().unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
        assert_eq!(playback.options.volume, Some(0.0));
        assert_eq!(playback.options.speed, Some(1.0));
        paths.push(path);
    }
    assert_ne!(paths[0], paths[1]);
    assert_eq!(paths[1], paths[2]);
    assert_ne!(paths[2], paths[3]);
    assert!(SayProvider.cached_detached_job(&text, &base).await.unwrap().is_none());
    for path in paths.into_iter().collect::<std::collections::HashSet<_>>() {
        std::fs::remove_file(path).unwrap();
    }
}
