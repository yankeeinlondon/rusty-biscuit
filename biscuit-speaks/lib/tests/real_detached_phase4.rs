#![cfg(feature = "playa")]

use biscuit_speaks::{Speak, SpeakPlaybackRoute, SpeakPlaybackVerdict};

fn required() -> bool {
    std::env::var("PLAYA_REAL_AUDIO_REQUIRED").as_deref() == Ok("1")
}

#[tokio::test]
async fn real_default_provider_reports_native_complete() {
    let _provider = test_toolkit::EnvGuard::set_safe("TTS_PROVIDER", "kokoro");
    if matches!(
        std::env::var("PLAYA_DRY_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    ) {
        assert!(!required(), "real audio is required but PLAYA_DRY_RUN is set");
        eprintln!("SKIP: PLAYA_DRY_RUN disables real audio");
        return;
    }
    let result = Speak::new("Phase four playback is complete.")
        .play_with_result()
        .await;
    let report = match result {
        Ok(result) => result.playback,
        Err(error) if required() => panic!("default TTS provider is required: {error}"),
        Err(error) => {
            eprintln!("SKIP: no concrete default TTS provider is ready: {error}");
            return;
        }
    };
    let Some(report) = report else {
        if required() {
            panic!("default provider used unverified direct streaming speech");
        }
        eprintln!("SKIP: default provider uses direct streaming speech");
        return;
    };
    assert_eq!(report.route, SpeakPlaybackRoute::Native);
    assert_eq!(report.verdict, SpeakPlaybackVerdict::Complete);
}
