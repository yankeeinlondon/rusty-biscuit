use std::path::PathBuf;
#[cfg(feature = "native-playback")]
use std::time::Instant;

use crate::audio::Audio;
#[cfg(feature = "native-playback")]
use crate::audio::AudioSourceKind;
use crate::error::{InvalidAudio, PlaybackError};
use crate::playback::playa_with_player_and_options_report_inner;
#[cfg(feature = "async")]
use crate::playback::playa_with_player_and_options_async_report_inner;
use crate::player::{AudioPlayer, PLAYER_LOOKUP, Player, match_available_players};
use crate::report::PlaybackReport;
#[cfg(feature = "native-playback")]
use crate::report::PlaybackRoute;
use crate::types::{AudioFormat, PlaybackOptions};

#[cfg(feature = "audio-ducking")]
use crate::ducking::{DuckConfig, DuckGuard, create_backend};

/// Returns `true` if the process-wide dry-run env var is enabled.
fn dry_run_env_enabled() -> bool {
    matches!(
        std::env::var("PLAYA_DRY_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

/// Builder for audio playback with optional metadata display.
///
/// `Playa` provides a fluent builder interface for configuring and playing audio.
/// Use the `show_meta()` method to enable metadata output to STDOUT when `play()`
/// is called.
///
/// ## Examples
///
/// ```no_run
/// use playa::Playa;
///
/// // Basic playback
/// Playa::from_path("song.mp3")?.play()?;
///
/// // Playback with metadata display
/// Playa::from_path("song.mp3")?
///     .volume(0.8)
///     .speed(1.25)
///     .show_meta()
///     .play()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct Playa {
    audio: Audio,
    options: PlaybackOptions,
    show_meta: bool,
    force_host: bool,
    dry_run: bool,
    #[cfg(feature = "audio-ducking")]
    duck_config: Option<DuckConfig>,
}

impl Playa {
    /// Create a new `Playa` from an `Audio` instance.
    pub fn new(audio: Audio) -> Self {
        Self {
            audio,
            options: PlaybackOptions::default(),
            show_meta: false,
            force_host: false,
            dry_run: false,
            #[cfg(feature = "audio-ducking")]
            duck_config: None,
        }
    }

    /// Create a `Playa` from a file path.
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, InvalidAudio> {
        let audio = Audio::from_path(path)?;
        Ok(Self::new(audio))
    }

    /// Create a `Playa` from raw audio bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, InvalidAudio> {
        let audio = Audio::from_bytes(bytes)?;
        Ok(Self::new(audio))
    }

    /// Create a builder from already classified audio data.
    pub fn from_data(data: crate::AudioData, format: AudioFormat) -> Self {
        Self::new(Audio::new(data, format))
    }

    /// Enable metadata display to STDOUT when `play()` is called.
    ///
    /// When enabled, displays:
    /// - Player software chosen
    /// - Volume setting
    /// - Speed setting
    /// - Codec
    /// - File format
    pub fn show_meta(mut self) -> Self {
        self.show_meta = true;
        self
    }

    /// Set the volume level (0.0 = silent, 1.0 = normal, >1.0 = amplified).
    pub fn volume(mut self, volume: f32) -> Self {
        self.options = self.options.with_volume(volume);
        self
    }

    /// Set the playback speed multiplier (1.0 = normal).
    pub fn speed(mut self, speed: f32) -> Self {
        self.options = self.options.with_speed(speed);
        self
    }

    /// Set playback options directly.
    pub fn with_options(mut self, options: PlaybackOptions) -> Self {
        self.options = options;
        self
    }

    /// Force host player playback, bypassing the native decoder.
    pub fn force_host(mut self) -> Self {
        self.force_host = true;
        self
    }

    /// Skip all audio output.
    ///
    /// `play()` and `play_async()` log at debug level and return
    /// `Ok(())` without opening any device, decoding any bytes, or
    /// spawning any subprocess. Equivalent to setting the
    /// `PLAYA_DRY_RUN=1` environment variable.
    ///
    /// Useful in tests, headless CI, and sandboxed builds where audio
    /// output is unavailable or undesirable.
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Hand playback to Playa's private ordered worker and return after publication.
    pub fn play_detached(self) -> Result<crate::detached::JobId, PlaybackError> {
        let playback = crate::detached::DetachedPlayback {
            options: self.options,
            routing: if self.force_host {
                crate::detached::PlaybackRouting::ForceHost
            } else {
                crate::detached::PlaybackRouting::Auto
            },
            #[cfg(feature = "audio-ducking")]
            ducking: self.duck_config.map(|config| crate::detached::DetachedDucking {
                ramp_ms: config.ramp_ms(),
                floor_scalar: config.floor_scalar(),
            }),
            #[cfg(not(feature = "audio-ducking"))]
            ducking: None,
        };
        crate::detached::enqueue_data(self.audio.into_data(), playback, self.dry_run)
    }

    /// Enable audio ducking during playback.
    ///
    /// When enabled, system audio will be attenuated to the configured floor
    /// level before playback starts, then restored to original levels after
    /// playback completes.
    ///
    /// Requires the `audio-ducking` feature flag.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// use playa::{Playa, ducking::DuckConfig};
    ///
    /// // Play with default ducking (1s ramp, 0.2 floor)
    /// Playa::from_path("audio.mp3")?
    ///     .with_ducked_audio(DuckConfig::default())
    ///     .play_async()
    ///     .await?;
    /// ```
    #[cfg(feature = "audio-ducking")]
    pub fn with_ducked_audio(mut self, config: DuckConfig) -> Self {
        self.duck_config = Some(config);
        self
    }

    /// Return the detected audio format.
    pub fn format(&self) -> AudioFormat {
        self.audio.format()
    }

    /// Play the audio using the best available player.
    ///
    /// When the `native-playback` feature is enabled, attempts in-process
    /// decoding via rodio/symphonia first. Any native failure raised before
    /// audio reaches the device (unsupported format, decode or file error,
    /// device-open failure or timeout, or a breaker tripped earlier in this
    /// process) falls back to a host player in the same call. A device that
    /// stops making progress after audio was submitted returns
    /// [`PlaybackError::AudioSubsystem`] instead of replaying through a host
    /// player. A device-open timeout or stall also disables native playback
    /// for the rest of the process, so later calls skip the device entirely.
    ///
    /// Note: If ducking is configured, use [`play_async`] instead as ducking
    /// requires an async runtime.
    pub fn play(self) -> Result<(), PlaybackError> {
        self.play_with_report().map(|_| ())
    }

    /// Play the audio and report the selected route and completion timing.
    pub fn play_with_report(self) -> Result<PlaybackReport, PlaybackError> {
        if self.dry_run || dry_run_env_enabled() {
            tracing::debug!("playa: dry-run enabled, skipping playback");
            return Ok(PlaybackReport::default());
        }
        validate_speed(&self.options)?;

        let format = self.audio.format();
        let probed = crate::probe_audio_metadata(self.audio.data_ref());

        #[cfg(feature = "native-playback")]
        if self.wants_native() {
            let result = match self.attempt_native(format, probed) {
                NativeAttempt::Complete(report) => Ok(report),
                NativeAttempt::Fallback => self.play_host(format, probed),
                NativeAttempt::Fatal(detail) => Err(PlaybackError::AudioSubsystem { detail }),
            };
            return result.map(PlaybackReport::warn_if_truncated);
        }

        self.play_host(format, probed)
            .map(PlaybackReport::warn_if_truncated)
    }

    #[cfg(feature = "native-playback")]
    fn wants_native(&self) -> bool {
        !self.force_host && !matches!(self.audio.source_kind(), AudioSourceKind::Url)
    }

    /// One native attempt, classified by the retry boundary in
    /// [`crate::native_player::NativePlaybackError::should_fallback_to_host`].
    #[cfg(feature = "native-playback")]
    fn attempt_native(
        &self,
        format: AudioFormat,
        probed: Option<crate::ProbedAudioMetadata>,
    ) -> NativeAttempt {
        let started = Instant::now();
        match crate::native_player::play_native(self.audio.data_ref(), format, &self.options) {
            Ok(()) => {
                if self.show_meta {
                    self.print_native_meta(format);
                }
                NativeAttempt::Complete(PlaybackReport::completed(
                    PlaybackRoute::Native,
                    probed,
                    started.elapsed(),
                    self.options.speed.unwrap_or(1.0),
                ))
            }
            Err(error) if error.should_fallback_to_host() => NativeAttempt::Fallback,
            Err(error) => NativeAttempt::Fatal(error.to_string()),
        }
    }

    fn play_host(
        self,
        format: AudioFormat,
        probed: Option<crate::ProbedAudioMetadata>,
    ) -> Result<PlaybackReport, PlaybackError> {
        let player = self.select_player(format)?;
        if self.show_meta {
            self.print_meta(player, format);
        }
        playa_with_player_and_options_report_inner(
            player,
            self.audio.into_data(),
            self.options,
            probed,
        )
    }

    /// Play the audio asynchronously with optional ducking support.
    ///
    /// Native-versus-host routing follows the same retry boundary as
    /// [`play`](Self::play): pre-submit native failures fall back to a host
    /// player in the same call, a post-submit stall is fatal, and a device-open
    /// timeout or stall disables native playback for the rest of the process.
    ///
    /// Ducking (if configured) is set up **before** the native/host decision
    /// so both playback paths benefit from audio attenuation.
    ///
    /// Requires the `audio-ducking` feature flag for ducking support.
    #[cfg(feature = "async")]
    pub async fn play_async(self) -> Result<(), PlaybackError> {
        self.play_async_with_report().await.map(|_| ())
    }

    /// Play asynchronously and report the selected route and completion timing.
    #[cfg(feature = "async")]
    pub async fn play_async_with_report(self) -> Result<PlaybackReport, PlaybackError> {
        if self.dry_run || dry_run_env_enabled() {
            tracing::debug!("playa: dry-run enabled, skipping async playback");
            return Ok(PlaybackReport::default());
        }
        validate_speed(&self.options)?;

        let format = self.audio.format();
        let probed = crate::probe_audio_metadata(self.audio.data_ref());

        // Set up ducking BEFORE playback (covers both native and host paths)
        #[cfg(feature = "audio-ducking")]
        let guard = if let Some(config) = self.duck_config {
            let backend = create_backend();
            match DuckGuard::new(backend, config).await {
                Ok(guard) => Some(guard),
                Err(e) => {
                    eprintln!("Warning: audio ducking failed to initialize: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let result = 'playback: {
            #[cfg(feature = "native-playback")]
            if self.wants_native() {
                match self.attempt_native(format, probed) {
                    NativeAttempt::Complete(report) => break 'playback Ok(report),
                    NativeAttempt::Fatal(detail) => {
                        break 'playback Err(PlaybackError::AudioSubsystem { detail });
                    }
                    NativeAttempt::Fallback => {}
                }
            }

            let player = match self.select_player(format) {
                Ok(player) => player,
                Err(error) => break 'playback Err(error),
            };
            if self.show_meta {
                self.print_meta(player, format);
            }
            break 'playback playa_with_player_and_options_async_report_inner(
                player,
                self.audio.into_data(),
                self.options,
                probed,
            )
            .await;
        };

        #[cfg(feature = "audio-ducking")]
        if let Some(guard) = guard {
            guard.restore().await;
        }

        result.map(PlaybackReport::warn_if_truncated)
    }

    /// Select the best available player for the audio format and options.
    fn select_player(&self, format: AudioFormat) -> Result<AudioPlayer, PlaybackError> {
        let players = match_available_players(format);
        let selected = players.into_iter().find(|candidate| {
            let Some(metadata) = PLAYER_LOOKUP.get(candidate) else {
                return false;
            };
            if self.options.requires_speed_control() && !metadata.supports_speed_control {
                return false;
            }
            if self.options.requires_volume_control() && !metadata.supports_volume_control {
                return false;
            }
            true
        });

        selected.ok_or_else(|| {
            if self.options.requires_speed_control() || self.options.requires_volume_control() {
                PlaybackError::NoPlayerWithCapabilities {
                    format,
                    needs_speed: self.options.requires_speed_control(),
                    needs_volume: self.options.requires_volume_control(),
                }
            } else {
                PlaybackError::NoCompatiblePlayer { format }
            }
        })
    }

    /// Print playback metadata for native (rodio/symphonia) playback.
    #[cfg(feature = "native-playback")]
    fn print_native_meta(&self, format: AudioFormat) {
        println!("Player: native (rodio/symphonia)");
        println!(
            "Volume: {}",
            self.options
                .volume
                .map(|v| format!("{}%", (v * 100.0) as i32))
                .unwrap_or_else(|| "default".to_string())
        );
        println!(
            "Speed: {}",
            self.options
                .speed
                .map(|s| format!("{}x", s))
                .unwrap_or_else(|| "1.0x".to_string())
        );
        println!(
            "Codec: {}",
            format
                .codec
                .map(format_codec)
                .unwrap_or_else(|| "unknown".to_string())
        );
        println!("Format: {}", format_file_format(format.file_format));
    }

    /// Print playback metadata to STDOUT.
    fn print_meta(&self, player: AudioPlayer, format: AudioFormat) {
        let player_name = PLAYER_LOOKUP
            .get(&player)
            .map(Player::display_name)
            .unwrap_or("unknown");

        println!("Player: {}", player_name);
        println!(
            "Volume: {}",
            self.options
                .volume
                .map(|v| format!("{}%", (v * 100.0) as i32))
                .unwrap_or_else(|| "default".to_string())
        );
        println!(
            "Speed: {}",
            self.options
                .speed
                .map(|s| format!("{}x", s))
                .unwrap_or_else(|| "1.0x".to_string())
        );
        println!(
            "Codec: {}",
            format
                .codec
                .map(format_codec)
                .unwrap_or_else(|| "unknown".to_string())
        );
        println!("Format: {}", format_file_format(format.file_format));
    }
}

fn validate_speed(options: &PlaybackOptions) -> Result<(), PlaybackError> {
    if let Some(speed) = options.speed
        && (!speed.is_finite() || speed <= 0.0)
    {
        return Err(PlaybackError::InvalidPlaybackSpeed { speed });
    }
    Ok(())
}

#[cfg(feature = "native-playback")]
enum NativeAttempt {
    Complete(PlaybackReport),
    Fallback,
    Fatal(String),
}

fn format_codec(codec: crate::types::Codec) -> String {
    use crate::types::Codec;
    match codec {
        Codec::Pcm => "PCM",
        Codec::Flac => "FLAC",
        Codec::Alac => "ALAC",
        Codec::Mp3 => "MP3",
        Codec::Aac => "AAC",
        Codec::Vorbis => "Vorbis",
        Codec::Opus => "Opus",
    }
    .to_string()
}

fn format_file_format(format: crate::types::AudioFileFormat) -> String {
    use crate::types::AudioFileFormat;
    match format {
        AudioFileFormat::Wav => ".wav",
        AudioFileFormat::Aiff => ".aiff",
        AudioFileFormat::Flac => ".flac",
        AudioFileFormat::Mp3 => ".mp3",
        AudioFileFormat::Ogg => ".ogg",
        AudioFileFormat::M4a => ".m4a",
        AudioFileFormat::Webm => ".webm",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudioFileFormat, Codec};

    #[test]
    fn format_codec_displays_correctly() {
        assert_eq!(format_codec(Codec::Pcm), "PCM");
        assert_eq!(format_codec(Codec::Mp3), "MP3");
        assert_eq!(format_codec(Codec::Opus), "Opus");
    }

    #[test]
    fn format_file_format_displays_correctly() {
        assert_eq!(format_file_format(AudioFileFormat::Wav), ".wav");
        assert_eq!(format_file_format(AudioFileFormat::Mp3), ".mp3");
        assert_eq!(format_file_format(AudioFileFormat::Ogg), ".ogg");
    }

    /// Minimal valid WAV header that `Audio::from_bytes` will accept.
    fn minimal_wav() -> Vec<u8> {
        let mut h = Vec::new();
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&36u32.to_le_bytes()); // file size - 8
        h.extend_from_slice(b"WAVE");
        h.extend_from_slice(b"fmt ");
        h.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        h.extend_from_slice(&1u16.to_le_bytes()); // PCM
        h.extend_from_slice(&1u16.to_le_bytes()); // mono
        h.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
        h.extend_from_slice(&88200u32.to_le_bytes()); // byte rate
        h.extend_from_slice(&2u16.to_le_bytes()); // block align
        h.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        h.extend_from_slice(b"data");
        h.extend_from_slice(&0u32.to_le_bytes()); // data size
        h
    }

    #[test]
    fn force_host_builder_sets_flag() {
        let playa = Playa::from_bytes(minimal_wav()).unwrap();
        assert!(!playa.force_host);
        let playa = playa.force_host();
        assert!(playa.force_host);
    }

    #[test]
    fn builder_dry_run_skips_playback() {
        let result = Playa::from_bytes(minimal_wav())
            .expect("minimal WAV should be accepted")
            .dry_run()
            .play();
        assert!(result.is_ok(), "dry-run play should succeed: {result:?}");
    }

    /// Native-versus-host routing through the public pipeline, with the
    /// device replaced by the injected native backend seam.
    #[cfg(feature = "native-playback")]
    mod native_route {
        use std::ffi::OsString;
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, MutexGuard};

        use super::*;
        use crate::native_audio::{
            NativeAudioTestGuard, lock_native_audio_test_state, native_audio_available,
        };
        use crate::native_player::{
            NativeBackendSeamGuard, NativePlaybackError, install_native_backend_seam_for_tests,
        };
        use crate::{AudioData, PlaybackRoute};

        static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

        /// One WAV fixture plus a test-private `PATH`.
        ///
        /// On Unix the `PATH` holds a fake `mpv` that exits 0, so the real
        /// host route (sniff detection, ranking, spawn) runs end to end
        /// without a player installed. Elsewhere the directory is empty, so
        /// reaching the host route ends in `NoCompatiblePlayer`.
        struct HostFixture {
            dir: PathBuf,
            audio: PathBuf,
            previous_path: Option<OsString>,
            seam_calls: Arc<AtomicUsize>,
            _seam: NativeBackendSeamGuard,
            _native: NativeAudioTestGuard,
            _env: MutexGuard<'static, ()>,
        }

        impl HostFixture {
            fn new(
                native: impl Fn() -> Result<(), NativePlaybackError> + Send + Sync + 'static,
            ) -> Self {
                let env = ENV_LOCK
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let native_guard = lock_native_audio_test_state();
                let seam_calls = Arc::new(AtomicUsize::new(0));
                let calls = Arc::clone(&seam_calls);
                let seam = install_native_backend_seam_for_tests(Box::new(move |_, _, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    native()
                }));

                let dir = std::env::temp_dir().join(format!(
                    "playa-native-route-{}-{}",
                    std::process::id(),
                    FIXTURE_SEQUENCE.fetch_add(1, Ordering::SeqCst)
                ));
                std::fs::create_dir_all(&dir).expect("fixture dir should be created");
                let audio = dir.join("fixture.wav");
                std::fs::write(&audio, minimal_wav()).expect("fixture audio should write");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    let fake = dir.join("mpv");
                    std::fs::write(&fake, "#!/bin/sh\nexit 0\n").expect("fake player should write");
                    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
                        .expect("fake player should be executable");
                }

                let previous_path = std::env::var_os("PATH");
                // SAFETY: ENV_LOCK is held for the fixture's lifetime; no other
                // test in this binary reads or writes PATH concurrently.
                unsafe {
                    std::env::set_var("PATH", &dir);
                }

                Self {
                    dir,
                    audio,
                    previous_path,
                    seam_calls,
                    _seam: seam,
                    _native: native_guard,
                    _env: env,
                }
            }

            fn audio(&self) -> AudioData {
                AudioData::FilePath(self.audio.clone())
            }

            fn seam_calls(&self) -> usize {
                self.seam_calls.load(Ordering::SeqCst)
            }
        }

        impl Drop for HostFixture {
            fn drop(&mut self) {
                // SAFETY: ENV_LOCK is still held (it drops after this body).
                unsafe {
                    match self.previous_path.take() {
                        Some(previous) => std::env::set_var("PATH", previous),
                        None => std::env::remove_var("PATH"),
                    }
                }
                let _ = std::fs::remove_dir_all(&self.dir);
            }
        }

        fn wav() -> AudioFormat {
            AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm))
        }

        fn play(fixture: &HostFixture) -> Result<PlaybackReport, PlaybackError> {
            crate::playa_explicit_with_options_and_report(
                wav(),
                fixture.audio(),
                PlaybackOptions::default(),
            )
        }

        #[cfg(feature = "async")]
        async fn play_async(fixture: &HostFixture) -> Result<PlaybackReport, PlaybackError> {
            crate::playa_explicit_with_options_async_and_report(
                wav(),
                fixture.audio(),
                PlaybackOptions::default(),
            )
            .await
        }

        #[test]
        fn free_functions_take_native_route_when_available() {
            let fixture = HostFixture::new(|| Ok(()));

            let report = play(&fixture).expect("native playback should succeed");
            assert_eq!(report.route, PlaybackRoute::Native);
            assert_eq!(fixture.seam_calls(), 1);
            assert!(native_audio_available());

            crate::playa_explicit_with_options(wav(), fixture.audio(), PlaybackOptions::default())
                .expect("plain free function should succeed");
            assert_eq!(fixture.seam_calls(), 2);
        }

        #[cfg(feature = "async")]
        #[tokio::test]
        async fn free_functions_take_native_route_when_available_async() {
            let fixture = HostFixture::new(|| Ok(()));

            let report = play_async(&fixture)
                .await
                .expect("native playback should succeed");
            assert_eq!(report.route, PlaybackRoute::Native);
            assert_eq!(fixture.seam_calls(), 1);
            assert!(native_audio_available());

            crate::playa_explicit_with_options_async(
                wav(),
                fixture.audio(),
                PlaybackOptions::default(),
            )
            .await
            .expect("plain free function should succeed");
            assert_eq!(fixture.seam_calls(), 2);
        }

        #[cfg(unix)]
        #[test]
        fn device_open_timeout_falls_back_to_host_in_the_same_call_and_trips_the_breaker() {
            let fixture = HostFixture::new(|| Err(NativePlaybackError::DeviceOpenTimeout(5)));

            let report = play(&fixture).expect("device-open timeout must reach the host route");
            assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
            assert!(!native_audio_available(), "device-open timeout trips the breaker");
            assert_eq!(fixture.seam_calls(), 1);

            let report = play(&fixture).expect("tripped breaker must reach the host route");
            assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
            assert_eq!(fixture.seam_calls(), 1, "a tripped breaker never touches the device");
        }

        #[cfg(all(unix, feature = "async"))]
        #[tokio::test]
        async fn device_open_timeout_falls_back_to_host_in_the_same_call_async() {
            let fixture = HostFixture::new(|| Err(NativePlaybackError::DeviceOpenTimeout(5)));

            let report = play_async(&fixture)
                .await
                .expect("device-open timeout must reach the host route");
            assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
            assert!(!native_audio_available(), "device-open timeout trips the breaker");
            assert_eq!(fixture.seam_calls(), 1);

            let report = play_async(&fixture)
                .await
                .expect("tripped breaker must reach the host route");
            assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
            assert_eq!(fixture.seam_calls(), 1, "a tripped breaker never touches the device");
        }

        #[cfg(unix)]
        #[test]
        fn stream_open_failure_falls_back_to_host_without_tripping_the_breaker() {
            let fixture = HostFixture::new(|| {
                Err(NativePlaybackError::Stream(rodio::DeviceSinkError::NoDevice))
            });

            let report = play(&fixture).expect("stream failure must reach the host route");
            assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
            assert!(native_audio_available(), "a stream error is not a breaker trip");

            let report = play(&fixture).expect("stream failure must reach the host route");
            assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
            assert_eq!(fixture.seam_calls(), 2, "native is retried while the breaker is intact");
        }

        #[cfg(all(unix, feature = "async"))]
        #[tokio::test]
        async fn stream_open_failure_falls_back_to_host_async() {
            let fixture = HostFixture::new(|| {
                Err(NativePlaybackError::Stream(rodio::DeviceSinkError::NoDevice))
            });

            let report = play_async(&fixture)
                .await
                .expect("stream failure must reach the host route");
            assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
            assert!(native_audio_available(), "a stream error is not a breaker trip");
        }

        #[test]
        fn post_submit_stall_is_fatal_and_never_replays_through_host() {
            let fixture = HostFixture::new(|| Err(NativePlaybackError::Timeout(5)));

            let error = play(&fixture).expect_err("a post-submit stall must not replay");
            assert!(
                matches!(error, PlaybackError::AudioSubsystem { .. }),
                "unexpected error: {error:?}"
            );
            assert!(!native_audio_available(), "a stall trips the breaker");
            assert_eq!(fixture.seam_calls(), 1);

            // The stall itself is never replayed, but the breaker it tripped
            // sends the next call straight to the host route.
            #[cfg(unix)]
            {
                let report = play(&fixture).expect("tripped breaker must reach the host route");
                assert_eq!(report.route, PlaybackRoute::Host(AudioPlayer::Mpv));
                assert_eq!(fixture.seam_calls(), 1, "a tripped breaker never touches the device");
            }
        }

        #[cfg(feature = "async")]
        #[tokio::test]
        async fn post_submit_stall_is_fatal_and_never_replays_through_host_async() {
            let fixture = HostFixture::new(|| Err(NativePlaybackError::Timeout(5)));

            let error = play_async(&fixture)
                .await
                .expect_err("a post-submit stall must not replay");
            assert!(
                matches!(error, PlaybackError::AudioSubsystem { .. }),
                "unexpected error: {error:?}"
            );
            assert!(!native_audio_available(), "a stall trips the breaker");
            assert_eq!(fixture.seam_calls(), 1);
        }
    }

    #[test]
    fn env_var_dry_run_skips_playback() {
        // Tests that mutate process env vars share a serial guard so they
        // do not race other tests in this binary.
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");

        // SAFETY: we hold the env mutex; no other test in this binary
        // reads or writes PLAYA_DRY_RUN concurrently.
        unsafe {
            std::env::set_var("PLAYA_DRY_RUN", "1");
        }

        let result = Playa::from_bytes(minimal_wav())
            .expect("minimal WAV should be accepted")
            .play();

        // SAFETY: same as above.
        unsafe {
            std::env::remove_var("PLAYA_DRY_RUN");
        }

        assert!(
            result.is_ok(),
            "env var dry-run play should succeed: {result:?}"
        );
    }

    /// Process-local mutex shared by tests that read/write env vars.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
