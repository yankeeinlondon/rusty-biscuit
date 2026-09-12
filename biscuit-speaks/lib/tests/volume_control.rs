#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use biscuit_speaks::{ESpeakProvider, TtsConfig, TtsExecutor, VolumeLevel};

fn executable(path: &std::path::Path, script: &str) {
    std::fs::write(path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[tokio::test]
#[serial_test::serial]
async fn espeak_foreground_and_detached_apply_volume_before_speech() {
    let temp = tempfile::tempdir().unwrap();
    let program = temp.path().join("espeak-fixture");
    let args_file = temp.path().join("args");
    let text_file = temp.path().join("text");
    executable(&program, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TTS_TEST_ARGS\"\n/bin/cat > \"$TTS_TEST_TEXT\"\n");
    let _args = test_toolkit::EnvGuard::set_safe("TTS_TEST_ARGS", &args_file);
    let _text = test_toolkit::EnvGuard::set_safe("TTS_TEST_TEXT", &text_file);
    let provider = ESpeakProvider::with_binary(program.to_string_lossy());
    let text = "This is a test message: quoted 'speech' and unicode 世界";
    for (volume, amplitude) in [(0.0, "0"), (0.42, "42"), (1.0, "100")] {
        let config = TtsConfig::new().with_voice("en+f3").with_volume(VolumeLevel::Explicit(volume));
        provider.speak(text, &config).await.unwrap();
        assert_eq!(std::fs::read_to_string(&text_file).unwrap(), text);
        let recorded = std::fs::read_to_string(&args_file).unwrap();
        assert_eq!(recorded.lines().collect::<Vec<_>>(), ["-v", "en+f3", "-a", amplitude]);
        #[cfg(feature = "playa")]
        {
            let playa::detached::SpoolJob::Command { args, .. } = provider.detached_job(text, &config).await.unwrap() else {
                panic!("expected eSpeak command job");
            };
            let args: Vec<_> = args.iter().map(|arg| arg.to_os_string().unwrap()).collect();
            assert_eq!(args, ["-v", "en+f3", "-a", amplitude, text].map(std::ffi::OsString::from));
        }
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn say_foreground_preserves_voice_and_applies_output_volume() {
    use biscuit_speaks::{SayProvider, SpeedLevel};

    let temp = tempfile::tempdir().unwrap();
    let args_file = temp.path().join("args");
    let text_file = temp.path().join("text");
    let player_file = temp.path().join("player");
    executable(&temp.path().join("say"), "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TTS_TEST_ARGS\"\n/bin/cat > \"$TTS_TEST_TEXT\"\n");
    // Empty synthesized bytes force native decoding to fail without submitting audio.
    let player = if cfg!(feature = "playa") { "mpv" } else { "afplay" };
    executable(&temp.path().join(player), "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TTS_TEST_PLAYER\"\n");
    let _path = test_toolkit::EnvGuard::set_safe("PATH", temp.path());
    let _args = test_toolkit::EnvGuard::set_safe("TTS_TEST_ARGS", &args_file);
    let _text = test_toolkit::EnvGuard::set_safe("TTS_TEST_TEXT", &text_file);
    let _player = test_toolkit::EnvGuard::set_safe("TTS_TEST_PLAYER", &player_file);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    let text = "This is a test message: quoted 'speech' and unicode 世界";
    for (volume, percent) in [(0.0, "0"), (0.42, "42")] {
        let config = TtsConfig::new().with_voice("Samantha").with_speed(SpeedLevel::Slow)
            .with_volume(VolumeLevel::Explicit(volume));
        SayProvider.speak(text, &config).await.unwrap();
        assert_eq!(std::fs::read_to_string(&text_file).unwrap(), text);
        let args = std::fs::read_to_string(&args_file).unwrap();
        let args: Vec<_> = args.lines().collect();
        assert_eq!(&args[2..], ["--file-format=WAVE", "--data-format=LEI16", "-v", "Samantha", "-r", "131"]);
        assert!(!std::path::Path::new(args[1]).exists(), "temporary speech must be cleaned after playback");
        let player_args = std::fs::read_to_string(&player_file).unwrap();
        if cfg!(feature = "playa") {
            assert!(player_args.lines().any(|arg| arg == format!("--volume={percent}")), "{player_args}");
        } else {
            assert_eq!(player_args.lines().take(2).collect::<Vec<_>>(), ["-v".to_string(), volume.to_string()]);
        }
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn say_synthesis_failure_removes_temporary_audio_without_playback() {
    let temp = tempfile::tempdir().unwrap();
    let args_file = temp.path().join("args");
    executable(&temp.path().join("say"), "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TTS_TEST_ARGS\"\n/bin/cat > /dev/null\nexit 1\n");
    let _path = test_toolkit::EnvGuard::set_safe("PATH", temp.path());
    let _args = test_toolkit::EnvGuard::set_safe("TTS_TEST_ARGS", &args_file);
    let config = TtsConfig::new().with_voice("Samantha").with_volume(VolumeLevel::Explicit(0.0));
    let error = biscuit_speaks::SayProvider.speak("This is a test message.", &config).await.unwrap_err();
    assert!(matches!(error, biscuit_speaks::TtsError::ProcessFailed { provider, .. } if provider == "say"));
    let args = std::fs::read_to_string(args_file).unwrap();
    assert!(!std::path::Path::new(args.lines().nth(1).unwrap()).exists());
}
