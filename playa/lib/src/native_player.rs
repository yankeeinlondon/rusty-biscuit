//! Native audio playback via rodio/symphonia.
//!
//! When the `native-playback` feature is enabled, audio files are decoded
//! in-process using symphonia codecs and played through the default output
//! device via rodio. This avoids spawning external player subprocesses for
//! all formats that symphonia can handle.
//!
//! Formats that symphonia cannot decode (currently only Opus) fall back to
//! host player delegation automatically.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::time::{Duration, Instant};

use rodio::{Decoder, DeviceSinkBuilder, Player};
use thiserror::Error;

use crate::audio::AudioData;
use crate::types::{AudioFileFormat, AudioFormat, Codec, PlaybackOptions};

/// Maximum time to wait for native audio playback to complete.
/// Files longer than this should use a host player (mpv, ffplay, etc.)
/// which manages its own timeouts.
const PLAYBACK_TIMEOUT: Duration = Duration::from_secs(600);

/// Errors from native audio playback.
#[derive(Debug, Error)]
pub enum NativePlaybackError {
    /// The audio format is not supported by the native decoder.
    #[error("format not supported for native playback: {0:?}")]
    UnsupportedFormat(AudioFormat),

    /// Native playback does not support URL sources.
    #[error("native playback does not support URL sources")]
    UrlNotSupported,

    /// Failed to open an audio output stream.
    #[error("failed to open audio stream: {0}")]
    Stream(#[from] rodio::DeviceSinkError),

    /// Failed to decode audio data.
    #[error("failed to decode audio: {0}")]
    Decode(#[from] rodio::decoder::DecoderError),

    /// Failed to play audio.
    #[error("failed to play audio: {0}")]
    Play(#[from] rodio::PlayError),

    /// An IO error occurred reading the audio file.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Playback timed out waiting for audio device.
    #[error("audio playback timed out after {0}s — audio device may be unresponsive")]
    Timeout(u64),
}

/// Check whether a given audio format can be decoded natively via symphonia.
///
/// Returns `true` for all formats where symphonia has both a demuxer and codec
/// available. Currently, only Opus is unsupported (no symphonia opus codec in
/// the 0.5.x series).
pub fn can_decode_natively(format: AudioFormat) -> bool {
    use AudioFileFormat::*;

    match format.file_format {
        Wav | Aiff | Mp3 | Flac | M4a | Webm => match format.codec {
            None => true,
            Some(Codec::Opus) => false,
            Some(_) => true,
        },
        Ogg => !matches!(format.codec, Some(Codec::Opus)),
    }
}

/// Play audio natively using rodio/symphonia.
///
/// Attempts in-process decoding and playback through the default audio output
/// device. Returns an error if the format is unsupported, the source is a URL,
/// or decoding/playback fails.
///
/// ## Errors
///
/// - `UnsupportedFormat` if `can_decode_natively()` returns false
/// - `UrlNotSupported` if the audio data is a URL
/// - `Stream`, `Decode`, `Play`, or `Io` on runtime failures
pub fn play_native(
    audio: &AudioData,
    format: AudioFormat,
    options: &PlaybackOptions,
) -> Result<(), NativePlaybackError> {
    if !can_decode_natively(format) {
        return Err(NativePlaybackError::UnsupportedFormat(format));
    }

    match audio {
        AudioData::Url(_) => Err(NativePlaybackError::UrlNotSupported),
        AudioData::Bytes(bytes) => play_from_bytes(bytes, options),
        AudioData::FilePath(path) => play_from_file(path, options),
    }
}

/// Play audio from in-memory bytes.
fn play_from_bytes(bytes: &[u8], options: &PlaybackOptions) -> Result<(), NativePlaybackError> {
    let source = Decoder::new(Cursor::new(bytes.to_vec()))?;
    play_source(source, options)
}

/// Play audio from a file path.
fn play_from_file(
    path: &std::path::Path,
    options: &PlaybackOptions,
) -> Result<(), NativePlaybackError> {
    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;
    play_source(source, options)
}

/// Play a decoded audio source through the specified or default output device.
fn play_source(
    source: Decoder<impl std::io::Read + std::io::Seek + Send + Sync + 'static>,
    options: &PlaybackOptions,
) -> Result<(), NativePlaybackError> {
    let stream = if let Some(channel_name) = &options.channel {
        if let Some(device) = crate::channels::find_device_by_id_or_name(channel_name) {
            DeviceSinkBuilder::from_device(device).and_then(|b| b.open_stream())?
        } else {
            DeviceSinkBuilder::open_default_sink()?
        }
    } else {
        DeviceSinkBuilder::open_default_sink()?
    };
    let player = Player::connect_new(stream.mixer());

    if let Some(vol) = options.volume {
        player.set_volume(vol);
    }
    if let Some(speed) = options.speed {
        player.set_speed(speed);
    }

    player.append(source);
    wait_with_timeout(&player, PLAYBACK_TIMEOUT)?;

    Ok(())
}

/// Wait for the player to finish, but give up after `timeout`.
///
/// Polls `player.empty()` every 50 ms. If the deadline is exceeded the
/// player is stopped and a `Timeout` error is returned so the caller can
/// fall back to a host player or report the failure.
fn wait_with_timeout(player: &Player, timeout: Duration) -> Result<(), NativePlaybackError> {
    let deadline = Instant::now() + timeout;
    while !player.empty() {
        if Instant::now() >= deadline {
            player.stop();
            eprintln!(
                "playa: audio playback timed out after {}s — audio device may be unresponsive",
                timeout.as_secs()
            );
            return Err(NativePlaybackError::Timeout(timeout.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AudioFileFormat;

    // ---- can_decode_natively truth table ----

    #[test]
    fn wav_pcm_is_native() {
        let format = AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn wav_no_codec_is_native() {
        let format = AudioFormat::new(AudioFileFormat::Wav, None);
        assert!(can_decode_natively(format));
    }

    #[test]
    fn aiff_pcm_is_native() {
        let format = AudioFormat::new(AudioFileFormat::Aiff, Some(Codec::Pcm));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn mp3_is_native() {
        let format = AudioFormat::new(AudioFileFormat::Mp3, Some(Codec::Mp3));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn flac_is_native() {
        let format = AudioFormat::new(AudioFileFormat::Flac, Some(Codec::Flac));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn m4a_aac_is_native() {
        let format = AudioFormat::new(AudioFileFormat::M4a, Some(Codec::Aac));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn m4a_alac_is_native() {
        let format = AudioFormat::new(AudioFileFormat::M4a, Some(Codec::Alac));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn ogg_vorbis_is_native() {
        let format = AudioFormat::new(AudioFileFormat::Ogg, Some(Codec::Vorbis));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn webm_vorbis_is_native() {
        let format = AudioFormat::new(AudioFileFormat::Webm, Some(Codec::Vorbis));
        assert!(can_decode_natively(format));
    }

    #[test]
    fn ogg_opus_is_not_native() {
        let format = AudioFormat::new(AudioFileFormat::Ogg, Some(Codec::Opus));
        assert!(!can_decode_natively(format));
    }

    #[test]
    fn webm_opus_is_not_native() {
        let format = AudioFormat::new(AudioFileFormat::Webm, Some(Codec::Opus));
        assert!(!can_decode_natively(format));
    }

    #[test]
    fn m4a_opus_is_not_native() {
        let format = AudioFormat::new(AudioFileFormat::M4a, Some(Codec::Opus));
        assert!(!can_decode_natively(format));
    }

    // ---- play_native error paths ----

    #[test]
    fn play_native_url_returns_url_not_supported() {
        let url = url::Url::parse("https://example.com/audio.mp3").unwrap();
        let data = AudioData::Url(url);
        let format = AudioFormat::new(AudioFileFormat::Mp3, Some(Codec::Mp3));
        let result = play_native(&data, format, &PlaybackOptions::default());
        assert!(matches!(result, Err(NativePlaybackError::UrlNotSupported)));
    }

    #[test]
    fn play_native_unsupported_format_returns_error() {
        let data = AudioData::Bytes(std::sync::Arc::new(vec![0u8; 100]));
        let format = AudioFormat::new(AudioFileFormat::Ogg, Some(Codec::Opus));
        let result = play_native(&data, format, &PlaybackOptions::default());
        assert!(matches!(
            result,
            Err(NativePlaybackError::UnsupportedFormat(_))
        ));
    }

    // ---- error Display formatting ----

    #[test]
    fn unsupported_format_error_display() {
        let format = AudioFormat::new(AudioFileFormat::Ogg, Some(Codec::Opus));
        let err = NativePlaybackError::UnsupportedFormat(format);
        let msg = err.to_string();
        assert!(msg.contains("not supported"), "error message: {msg}");
    }

    #[test]
    fn url_not_supported_error_display() {
        let err = NativePlaybackError::UrlNotSupported;
        let msg = err.to_string();
        assert!(msg.contains("URL"), "error message: {msg}");
    }

    #[test]
    fn decode_error_display() {
        let err = NativePlaybackError::Decode(rodio::decoder::DecoderError::UnrecognizedFormat);
        let msg = err.to_string();
        assert!(msg.contains("decode"), "error message: {msg}");
    }
}
