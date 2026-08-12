//! Native audio playback via rodio/symphonia.
//!
//! When the `native-playback` feature is enabled, audio files are decoded
//! in-process using symphonia codecs and played through the default output
//! device via rodio. This avoids spawning external player subprocesses for
//! all formats that symphonia can handle.
//!
//! Formats that symphonia cannot decode (currently only Opus) fall back to
//! host player delegation automatically.
//!
//! Native playback never terminates the process. If a native device-open
//! operation times out, playa disables further native playback attempts for
//! the rest of the process and future calls fall back to host players.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use rodio::{Decoder, DeviceSinkBuilder, Player};
use thiserror::Error;

use crate::audio::AudioData;
use crate::native_audio::{
    NATIVE_DEVICE_TIMEOUT, NativeAudioFailureKind, log_native_audio_disabled_once,
    native_audio_available, open_with_channel_fallback, run_with_timeout,
    trip_native_audio_breaker,
};
use crate::types::{AudioFileFormat, AudioFormat, Codec, PlaybackOptions};

/// Maximum time to wait for native audio playback to complete.
/// Most audio files finish well within 5 minutes; anything longer
/// should use a host player (mpv, ffplay, etc.).
const PLAYBACK_TIMEOUT: Duration = Duration::from_secs(300);

/// Default time without forward playback progress before the device is
/// considered wedged.
const DEFAULT_STALL_WINDOW: Duration = Duration::from_secs(5);

/// Process-wide cache for the default-device mixer sink.
///
/// Opened lazily on the first native playback call. Once cached, every
/// default-device playback connects a fresh `Player` to this sink's
/// mixer instead of opening a new CoreAudio stream. This dramatically
/// reduces device-open churn against `coreaudiod` on macOS.
///
/// The slot is a `Mutex<Option<Arc<MixerDeviceSink>>>` so a failed
/// initial open does not poison the slot — subsequent calls retry.
static SHARED_DEFAULT_SINK: OnceLock<Mutex<Option<Arc<rodio::MixerDeviceSink>>>> = OnceLock::new();

fn shared_default_sink_slot() -> &'static Mutex<Option<Arc<rodio::MixerDeviceSink>>> {
    SHARED_DEFAULT_SINK.get_or_init(|| Mutex::new(None))
}

/// Acquire (lazily open) the cached default-device sink and run `body`
/// against its mixer.
///
/// On first call, `open` is invoked to construct the sink; the result
/// is cached for subsequent calls. If `open` fails, the cache stays
/// empty and the next call retries.
///
/// `MixerDeviceSink::log_on_drop(false)` is set so the cached sink
/// never logs spurious "dropping DeviceSink" noise during process
/// shutdown.
fn with_cached_default_mixer<F>(
    open: impl FnOnce() -> Result<rodio::MixerDeviceSink, NativePlaybackError>,
    body: F,
) -> Result<(), NativePlaybackError>
where
    F: FnOnce(&rodio::mixer::Mixer) -> Result<(), NativePlaybackError>,
{
    let slot = shared_default_sink_slot();
    let mut guard = slot.lock().expect("default sink mutex poisoned");

    if guard.is_none() {
        let mut sink = open()?;
        sink.log_on_drop(false);
        *guard = Some(Arc::new(sink));
    }

    let sink = guard.as_ref().expect("just inserted").clone();
    drop(guard);

    body(sink.mixer())
}

#[cfg(test)]
fn reset_default_sink_cache_for_tests() {
    if let Some(slot) = SHARED_DEFAULT_SINK.get() {
        let mut guard = slot.lock().expect("default sink mutex poisoned");
        *guard = None;
    }
}

/// Reads `PLAYA_PLAYBACK_STALL_SECONDS` from the environment.
///
/// Returns the parsed duration, or [`DEFAULT_STALL_WINDOW`] if the var
/// is unset, empty, or invalid.  Invalid values emit a single
/// `tracing::warn!`.
fn resolved_stall_window() -> Duration {
    match std::env::var("PLAYA_PLAYBACK_STALL_SECONDS") {
        Ok(raw) if !raw.is_empty() => match raw.parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => {
                tracing::warn!(
                    raw = %raw,
                    "PLAYA_PLAYBACK_STALL_SECONDS is not a positive integer; using default"
                );
                DEFAULT_STALL_WINDOW
            }
        },
        _ => DEFAULT_STALL_WINDOW,
    }
}

/// Minimal player surface used by the progress-aware wait loop.
///
/// Extracted as a trait so the wait logic can be tested with a fake
/// player whose progress and emptiness are scriptable.
trait PlayerProgress {
    /// Returns true once all queued audio has finished playing.
    fn empty(&self) -> bool;
    /// Returns the current playback position.
    fn get_pos(&self) -> Duration;
    /// Stops playback and clears the queue.
    fn stop(&self);
}

impl PlayerProgress for Player {
    fn empty(&self) -> bool {
        Player::empty(self)
    }
    fn get_pos(&self) -> Duration {
        Player::get_pos(self)
    }
    fn stop(&self) {
        Player::stop(self);
    }
}

/// Open an audio output stream with a timeout.
///
/// Resolves an optional requested channel and opens the requested or default
/// device within a single bounded deadline.
fn open_stream_with_timeout(
    timeout: Duration,
    options: &PlaybackOptions,
) -> Result<rodio::MixerDeviceSink, NativePlaybackError> {
    let deadline = Instant::now() + timeout;
    open_with_channel_fallback(
        options.channel.as_deref(),
        deadline,
        crate::channels::find_device_by_id_or_name_with_timeout,
        open_device_stream_with_timeout,
        open_default_stream_with_timeout,
        || NativePlaybackError::DeviceOpenTimeout(timeout.as_secs()),
    )
}

fn open_device_stream_with_timeout(
    device: rodio::Device,
    timeout: Duration,
) -> Result<rodio::MixerDeviceSink, NativePlaybackError> {
    if timeout.is_zero() {
        return Err(NativePlaybackError::DeviceOpenTimeout(
            NATIVE_DEVICE_TIMEOUT.as_secs(),
        ));
    }

    run_with_timeout(
        timeout,
        move || {
            DeviceSinkBuilder::from_device(device)
                .and_then(|builder| builder.open_stream())
                .map_err(NativePlaybackError::Stream)
        },
        native_device_open_timeout,
    )
}

fn open_default_stream_with_timeout(
    timeout: Duration,
) -> Result<rodio::MixerDeviceSink, NativePlaybackError> {
    if timeout.is_zero() {
        return Err(NativePlaybackError::DeviceOpenTimeout(
            NATIVE_DEVICE_TIMEOUT.as_secs(),
        ));
    }

    run_with_timeout(
        timeout,
        || DeviceSinkBuilder::open_default_sink().map_err(NativePlaybackError::Stream),
        native_device_open_timeout,
    )
}

fn native_device_open_timeout(timeout: Duration) -> NativePlaybackError {
    trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);
    NativePlaybackError::DeviceOpenTimeout(timeout.as_secs())
}

/// Errors from native audio playback.
#[derive(Debug, Error)]
pub enum NativePlaybackError {
    /// The audio format is not supported by the native decoder.
    #[error("format not supported for native playback: {0:?}")]
    UnsupportedFormat(AudioFormat),

    /// Native playback does not support URL sources.
    #[error("native playback does not support URL sources")]
    UrlNotSupported,

    /// Native playback was disabled earlier in this process.
    #[error("native playback is disabled for this process after an earlier device-open timeout")]
    NativePlaybackDisabled,

    /// Failed to open an audio output stream.
    #[error("failed to open audio stream: {0}")]
    Stream(#[from] rodio::DeviceSinkError),

    /// Audio device did not respond within the allotted time.
    #[error("audio device did not respond within {0}s")]
    DeviceOpenTimeout(u64),

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

impl NativePlaybackError {
    /// Whether callers should fall back to a host player instead of reporting
    /// the native failure directly.
    pub(crate) fn should_fallback_to_host(&self) -> bool {
        matches!(
            self,
            Self::UnsupportedFormat(_) | Self::UrlNotSupported | Self::Decode(_)
        )
    }
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
/// or decoding/playback fails. A device-open timeout disables future native
/// playback attempts for the current process so callers can fall back directly
/// to host playback.
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

    if !native_audio_available() {
        log_native_audio_disabled_once();
        return Err(NativePlaybackError::NativePlaybackDisabled);
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
    if options.channel.is_some() {
        return play_source_one_shot(source, options);
    }

    with_cached_default_mixer(
        || open_default_stream_with_timeout(NATIVE_DEVICE_TIMEOUT),
        |mixer| {
            let player = Player::connect_new(mixer);

            if let Some(vol) = options.volume {
                player.set_volume(vol);
            }
            if let Some(speed) = options.speed {
                player.set_speed(speed);
            }

            player.append(source);
            wait_with_progress(
                &player,
                PLAYBACK_TIMEOUT,
                resolved_stall_window(),
                Duration::from_millis(50),
            )?;

            Ok(())
        },
    )
}

/// One-shot device-open path for channel-override playback.
///
/// Channel routing bypasses the cached default-device sink so each
/// call resolves the requested device fresh.
fn play_source_one_shot(
    source: Decoder<impl std::io::Read + std::io::Seek + Send + Sync + 'static>,
    options: &PlaybackOptions,
) -> Result<(), NativePlaybackError> {
    let stream = open_stream_with_timeout(NATIVE_DEVICE_TIMEOUT, options)?;
    let player = Player::connect_new(stream.mixer());

    if let Some(vol) = options.volume {
        player.set_volume(vol);
    }
    if let Some(speed) = options.speed {
        player.set_speed(speed);
    }

    player.append(source);
    wait_with_progress(
        &player,
        PLAYBACK_TIMEOUT,
        resolved_stall_window(),
        Duration::from_millis(50),
    )?;

    Ok(())
}

/// Wait for the player to drain, abandoning the device if no playback
/// progress occurs for the stall window.
///
/// `absolute_timeout` is the wall-clock backstop (existing 300 s
/// behavior). `stall_window` is the maximum gap between advances of
/// `Player::get_pos()`. `poll_interval` is the loop sleep cadence.
///
/// On stall, calls `player.stop()`, trips the native breaker, and
/// returns [`NativePlaybackError::Timeout`] so subsequent native
/// attempts in this process route directly to host playback.
fn wait_with_progress<P: PlayerProgress>(
    player: &P,
    absolute_timeout: Duration,
    stall_window: Duration,
    poll_interval: Duration,
) -> Result<(), NativePlaybackError> {
    let start = Instant::now();
    let mut last_pos = player.get_pos();
    let mut last_progress_at = Instant::now();

    while !player.empty() {
        let now = Instant::now();
        if now.duration_since(start) >= absolute_timeout {
            player.stop();
            trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);
            eprintln!(
                "playa: audio playback timed out after {}s — audio device may be unresponsive",
                absolute_timeout.as_secs()
            );
            return Err(NativePlaybackError::Timeout(absolute_timeout.as_secs()));
        }

        let pos = player.get_pos();
        if pos != last_pos {
            last_pos = pos;
            last_progress_at = now;
        } else if now.duration_since(last_progress_at) >= stall_window {
            player.stop();
            trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);
            eprintln!(
                "playa: audio playback stalled — no progress for {}s, treating device as unresponsive",
                stall_window.as_secs()
            );
            return Err(NativePlaybackError::Timeout(stall_window.as_secs()));
        }

        std::thread::sleep(poll_interval);
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

    #[test]
    fn play_native_short_circuits_when_native_audio_is_disabled() {
        let _guard = crate::native_audio::lock_native_audio_test_state();
        crate::native_audio::trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);

        let data = AudioData::Bytes(std::sync::Arc::new(vec![0u8; 4]));
        let format = AudioFormat::new(AudioFileFormat::Mp3, Some(Codec::Mp3));
        let result = play_native(&data, format, &PlaybackOptions::default());

        assert!(matches!(
            result,
            Err(NativePlaybackError::NativePlaybackDisabled)
        ));
    }

    #[test]
    fn decode_errors_do_not_trip_native_audio_breaker() {
        let _guard = crate::native_audio::lock_native_audio_test_state();

        let data = AudioData::Bytes(std::sync::Arc::new(vec![0u8; 4]));
        let format = AudioFormat::new(AudioFileFormat::Mp3, Some(Codec::Mp3));
        let result = play_native(&data, format, &PlaybackOptions::default());

        assert!(matches!(result, Err(NativePlaybackError::Decode(_))));
        assert!(crate::native_audio::native_audio_available());
    }

    mod sink_cache {
        use super::*;
        use std::sync::atomic::{AtomicU32, Ordering};

        #[test]
        fn failed_open_does_not_poison_cache() {
            reset_default_sink_cache_for_tests();

            let opens = AtomicU32::new(0);
            let try_open = || {
                opens.fetch_add(1, Ordering::SeqCst);
                Err::<rodio::MixerDeviceSink, NativePlaybackError>(
                    NativePlaybackError::DeviceOpenTimeout(0),
                )
            };

            let r1 = with_cached_default_mixer(try_open, |_mixer| Ok(()));
            assert!(r1.is_err());

            let r2 = with_cached_default_mixer(try_open, |_mixer| Ok(()));
            assert!(r2.is_err());

            assert_eq!(opens.load(Ordering::SeqCst), 2);
        }
    }

    mod progress_wait {
        use super::*;
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

        struct FakePlayer {
            empty: AtomicBool,
            pos_micros: AtomicU64,
            stop_count: Mutex<u32>,
        }

        impl FakePlayer {
            fn new() -> Self {
                Self {
                    empty: AtomicBool::new(false),
                    pos_micros: AtomicU64::new(0),
                    stop_count: Mutex::new(0),
                }
            }
            fn set_empty(&self) {
                self.empty.store(true, Ordering::SeqCst);
            }
            fn advance_pos(&self, by: Duration) {
                self.pos_micros
                    .fetch_add(by.as_micros() as u64, Ordering::SeqCst);
            }
            fn stops(&self) -> u32 {
                *self.stop_count.lock().unwrap()
            }
        }

        impl PlayerProgress for FakePlayer {
            fn empty(&self) -> bool {
                self.empty.load(Ordering::SeqCst)
            }
            fn get_pos(&self) -> Duration {
                Duration::from_micros(self.pos_micros.load(Ordering::SeqCst))
            }
            fn stop(&self) {
                *self.stop_count.lock().unwrap() += 1;
            }
        }

        #[test]
        fn returns_ok_when_player_empties() {
            let _guard = crate::native_audio::lock_native_audio_test_state();
            let fake = FakePlayer::new();
            fake.set_empty();
            let result = wait_with_progress(
                &fake,
                Duration::from_secs(60),
                Duration::from_millis(100),
                Duration::from_millis(1),
            );
            assert!(result.is_ok());
            assert_eq!(fake.stops(), 0);
            assert!(crate::native_audio::native_audio_available());
        }

        #[test]
        fn trips_breaker_on_stall() {
            let _guard = crate::native_audio::lock_native_audio_test_state();
            let fake = FakePlayer::new();
            // Position never advances and player never empties.
            let result = wait_with_progress(
                &fake,
                Duration::from_secs(60),
                Duration::from_millis(50),
                Duration::from_millis(1),
            );
            assert!(matches!(result, Err(NativePlaybackError::Timeout(_))));
            assert_eq!(fake.stops(), 1);
            assert!(!crate::native_audio::native_audio_available());
        }

        #[test]
        fn absolute_deadline_takes_precedence() {
            let _guard = crate::native_audio::lock_native_audio_test_state();
            let fake = FakePlayer::new();
            // Stall window > absolute timeout, so the absolute deadline fires first.
            let result = wait_with_progress(
                &fake,
                Duration::from_millis(30),
                Duration::from_secs(60),
                Duration::from_millis(1),
            );
            assert!(matches!(result, Err(NativePlaybackError::Timeout(_))));
            assert_eq!(fake.stops(), 1);
            assert!(!crate::native_audio::native_audio_available());
        }

        #[test]
        fn progress_resets_stall_clock() {
            let _guard = crate::native_audio::lock_native_audio_test_state();
            let fake = std::sync::Arc::new(FakePlayer::new());
            // Spawn a thread that advances pos, then sets empty before stall fires.
            let fake_clone = fake.clone();
            let handle = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                fake_clone.advance_pos(Duration::from_millis(10));
                std::thread::sleep(Duration::from_millis(20));
                fake_clone.advance_pos(Duration::from_millis(10));
                std::thread::sleep(Duration::from_millis(20));
                fake_clone.set_empty();
            });
            // The stall window is 2s against 20ms progress ticks — a ~100x
            // margin. It was 40ms, giving 2x, which is below scheduler jitter on
            // a loaded CI runner: the macOS canary reported "no progress for 0s"
            // and failed on the first CI run that ever executed this test.
            //
            // The assertion is that progress RESETS the clock, not that it does
            // so within any particular window, so the margin costs nothing. A
            // regression still fails, just after 2s instead of 40ms. That the
            // clock fires at all when progress stops is covered by
            // `trips_breaker_on_stall` above, which keeps a tight window
            // because it wants the stall.
            let result = wait_with_progress(
                fake.as_ref(),
                Duration::from_secs(30),
                Duration::from_secs(2),
                Duration::from_millis(1),
            );
            handle.join().unwrap();
            assert!(result.is_ok(), "expected ok, got {result:?}");
            assert!(crate::native_audio::native_audio_available());
        }
    }
}
