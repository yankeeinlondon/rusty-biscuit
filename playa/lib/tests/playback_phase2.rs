use std::sync::Arc;
use std::time::Duration;

use playa::{
    AudioData, AudioFileFormat, AudioFormat, Codec, PlaybackReport, PlaybackRoute,
    PlaybackVerdict, Playa, ProbedAudioMetadata, probe_audio_metadata,
};

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

#[test]
fn dry_run_does_not_read_a_missing_source() {
    let report = Playa::from_data(
        AudioData::FilePath("/path/that/must/not/be/read.wav".into()),
        AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm)),
    )
    .dry_run()
    .play_with_report()
    .unwrap();

    assert_eq!(report, PlaybackReport::default());
    assert_eq!(report.route, PlaybackRoute::DryRun);
    assert_eq!(report.verdict, PlaybackVerdict::Unverified);
}

#[cfg(feature = "sound-effects")]
#[test]
fn every_bundled_effect_matches_build_time_duration_metadata() {
    for effect in playa::SoundEffect::all() {
        let runtime = probe_audio_metadata(&AudioData::Bytes(Arc::new(effect.bytes().to_vec())))
            .unwrap_or_else(|| panic!("runtime metadata missing for {}", effect.name()));
        let build_time = Duration::from_millis(u64::from(
            effect
                .duration_ms()
                .unwrap_or_else(|| panic!("build metadata missing for {}", effect.name())),
        ));
        assert!(
            runtime.duration.abs_diff(build_time) <= Duration::from_millis(1),
            "duration mismatch for {}: runtime={:?}, build={:?}",
            effect.name(),
            runtime.duration,
            build_time
        );
        assert!(runtime.channels > 0, "channel count missing for {}", effect.name());
    }
}

#[test]
fn original_24khz_mono_5970ms_input_is_probed() {
    let data = AudioData::Bytes(Arc::new(pcm_wav(
        24_000,
        1,
        Duration::from_millis(5_970),
    )));

    assert_eq!(
        probe_audio_metadata(&data),
        Some(ProbedAudioMetadata {
            duration: Duration::from_millis(5_970),
            channels: 1,
        })
    );
}

#[test]
fn metadata_probe_preserves_multichannel_and_rejects_unknown_sources() {
    let multichannel = AudioData::Bytes(Arc::new(pcm_wav(
        48_000,
        6,
        Duration::from_millis(250),
    )));
    assert_eq!(probe_audio_metadata(&multichannel).unwrap().channels, 6);

    let garbage = AudioData::Bytes(Arc::new(b"not audio".to_vec()));
    assert_eq!(probe_audio_metadata(&garbage), None);

    let url = AudioData::Url(url::Url::parse("https://example.invalid/speech.wav").unwrap());
    assert_eq!(probe_audio_metadata(&url), None);
}

#[test]
fn report_schema_round_trips_and_defaults_additive_fields() {
    let report = PlaybackReport {
        route: PlaybackRoute::Native,
        expected: Some(Duration::from_millis(5_970)),
        elapsed: Duration::from_millis(6_330),
        verdict: PlaybackVerdict::Complete,
    };
    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["route"], "native");
    assert_eq!(json["verdict"], "complete");
    assert_eq!(serde_json::from_value::<PlaybackReport>(json).unwrap(), report);

    let host_route = serde_json::to_value(PlaybackRoute::Host(playa::AudioPlayer::Mpv)).unwrap();
    assert_eq!(host_route, serde_json::json!({ "host": "mpv" }));

    let previous = serde_json::json!({
        "route": "dry_run",
        "elapsed_millis": 0,
        "verdict": "unverified"
    });
    let prior_report: PlaybackReport = serde_json::from_value(previous).unwrap();
    assert_eq!(prior_report.expected, None);
    assert_eq!(prior_report.route, PlaybackRoute::DryRun);

    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../claudine/fixes/2026-09-03-tts-not-finishing/fixtures/v1-delegated-report.json"
    ))
    .unwrap();
    let fixture_report: PlaybackReport =
        serde_json::from_value(fixture["outcome"]["report"].clone()).unwrap();
    assert_eq!(fixture_report.expected, Some(Duration::from_millis(5_970)));
    assert_eq!(fixture_report.elapsed, Duration::from_millis(6_330));
}

#[test]
fn verdict_uses_exact_threshold_and_effective_speed() {
    let expected = Duration::from_secs(10);
    assert_eq!(
        PlaybackVerdict::for_timing(Some(expected), Duration::from_millis(8_749), 1.0),
        PlaybackVerdict::Truncated {
            missing: Duration::from_millis(1_251),
        }
    );
    assert_eq!(
        PlaybackVerdict::for_timing(Some(expected), Duration::from_millis(8_750), 1.0),
        PlaybackVerdict::Complete
    );
    assert_eq!(
        PlaybackVerdict::for_timing(Some(expected), Duration::from_millis(4_250), 2.0),
        PlaybackVerdict::Complete
    );
    assert_eq!(
        PlaybackVerdict::for_timing(None, Duration::ZERO, 1.0),
        PlaybackVerdict::Unverified
    );
    assert_eq!(
        PlaybackVerdict::for_timing(Some(expected), Duration::ZERO, 0.0),
        PlaybackVerdict::Unverified
    );
}

#[test]
fn invalid_speed_is_rejected_before_routing() {
    let result = Playa::from_data(
        AudioData::Bytes(Arc::new(pcm_wav(
            24_000,
            1,
            Duration::from_millis(10),
        ))),
        AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm)),
    )
    .speed(0.0)
    .play_with_report();
    assert!(matches!(
        result,
        Err(playa::PlaybackError::InvalidPlaybackSpeed { speed }) if speed == 0.0
    ));
}
