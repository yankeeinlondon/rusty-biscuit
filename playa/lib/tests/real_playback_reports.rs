use std::sync::Arc;
use std::time::Duration;

use playa::{
    AudioData, AudioFileFormat, AudioFormat, AudioPlayer, Codec, PlaybackOptions, PlaybackRoute,
    PlaybackVerdict, match_available_players, playa_with_player_and_options_with_report,
    playa_with_player_with_report,
};
#[cfg(feature = "native-playback")]
use playa::Playa;

fn silent_pcm_wav(channels: u16, duration: Duration) -> Vec<u8> {
    let sample_rate = 24_000_u32;
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

fn required() -> bool {
    std::env::var("PLAYA_REAL_AUDIO_REQUIRED").as_deref() == Ok("1")
}

fn wav_format() -> AudioFormat {
    AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm))
}

#[cfg(feature = "native-playback")]
#[test]
fn real_native_mono_reports_complete() {
    let result = Playa::from_data(
        AudioData::Bytes(Arc::new(silent_pcm_wav(
            1,
            Duration::from_millis(5_970),
        ))),
        wav_format(),
    )
    .play_with_report();

    let report = match result {
        Ok(report) if report.route == PlaybackRoute::Native => report,
        Ok(report) if required() => panic!("native route required, got {report:?}"),
        Err(error) if required() => panic!("native playback required: {error}"),
        other => {
            eprintln!("SKIP: native audio route unavailable: {other:?}");
            return;
        }
    };
    eprintln!("EVIDENCE native_report={report:?}");
    assert_eq!(report.expected, Some(Duration::from_millis(5_970)));
    assert_eq!(report.verdict, PlaybackVerdict::Complete);
}

#[test]
fn real_explicit_mpv_mono_reports_complete() {
    if !match_available_players(wav_format()).contains(&AudioPlayer::Mpv) {
        assert!(!required(), "mpv is required");
        eprintln!("SKIP: mpv is not installed");
        return;
    }

    let report = playa_with_player_with_report(
        AudioPlayer::Mpv,
        AudioData::Bytes(Arc::new(silent_pcm_wav(
            1,
            Duration::from_millis(5_970),
        ))),
    )
    .unwrap_or_else(|error| panic!("mpv playback failed: {error}"));
    eprintln!("EVIDENCE mpv_report={report:?}");
    assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
    assert_eq!(report.expected, Some(Duration::from_millis(5_970)));
    assert_eq!(report.verdict, PlaybackVerdict::Complete);
}

#[test]
fn real_each_installed_host_player_reports_mono_completion() {
    let players = match_available_players(wav_format());
    if players.is_empty() {
        assert!(!required(), "at least one host player is required");
        eprintln!("SKIP: no compatible host audio player installed");
        return;
    }

    for player in players {
        let report = playa_with_player_with_report(
            player,
            AudioData::Bytes(Arc::new(silent_pcm_wav(
                1,
                Duration::from_millis(5_970),
            ))),
        )
        .unwrap_or_else(|error| panic!("{player:?} playback failed: {error}"));
        assert_eq!(report.route, PlaybackRoute::Host(player));
        assert_eq!(
            report.verdict,
            PlaybackVerdict::Complete,
            "{player:?} did not complete mono playback: {report:?}"
        );
    }
}

#[test]
fn real_zero_volume_stereo_control_reports_complete() {
    let Some(player) = match_available_players(wav_format())
        .into_iter()
        .find(|player| playa::PLAYER_LOOKUP[player].supports_volume_control)
    else {
        assert!(!required(), "a volume-capable host player is required");
        eprintln!("SKIP: no volume-capable host audio player installed");
        return;
    };

    let report = playa_with_player_and_options_with_report(
        player,
        AudioData::Bytes(Arc::new(silent_pcm_wav(2, Duration::from_secs(1)))),
        PlaybackOptions::new().with_volume(0.0),
    )
    .unwrap_or_else(|error| panic!("stereo control playback failed: {error}"));
    assert_eq!(report.verdict, PlaybackVerdict::Complete);
}
