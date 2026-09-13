#![cfg(feature = "playa")]

use biscuit_speaks::{
    HostTtsProvider, Speak, SpeakPlaybackRoute, SpeakPlaybackVerdict, TtsFailoverStrategy,
    TtsProvider, VolumeLevel,
};

fn required() -> bool {
    std::env::var("PLAYA_REAL_AUDIO_REQUIRED").as_deref() == Ok("1")
}

#[tokio::test]
async fn real_kokoro_provider_reports_muted_native_complete() {
    if matches!(
        std::env::var("PLAYA_DRY_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    ) {
        assert!(!required(), "real audio is required but PLAYA_DRY_RUN is set");
        eprintln!("SKIP: PLAYA_DRY_RUN disables real audio");
        return;
    }
    let result = Speak::new("This is a test message.")
        .with_voice("af_heart")
        .with_failover(TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(
            HostTtsProvider::KokoroTts,
        )))
        .with_volume(VolumeLevel::Explicit(0.0))
        .play_with_result()
        .await;
    let report = match result {
        Ok(result) => result.playback,
        Err(error) if required() => panic!("Kokoro af_heart provider is required: {error}"),
        Err(error) => {
            eprintln!("SKIP: no concrete Kokoro af_heart provider is ready: {error}");
            return;
        }
    };
    let Some(report) = report else {
        if required() {
            panic!("Kokoro provider used unverified direct streaming speech");
        }
        eprintln!("SKIP: Kokoro provider uses direct streaming speech");
        return;
    };
    assert_eq!(report.route, SpeakPlaybackRoute::Native);
    assert_eq!(report.verdict, SpeakPlaybackVerdict::Complete);
}
