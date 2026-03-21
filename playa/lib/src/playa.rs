use std::path::PathBuf;

use crate::audio::Audio;
#[cfg(feature = "native-playback")]
use crate::audio::AudioSourceKind;
use crate::error::{InvalidAudio, PlaybackError};
use crate::playback::playa_with_player_and_options;
use crate::player::{AudioPlayer, PLAYER_LOOKUP, Player, match_available_players};
use crate::types::{AudioFormat, PlaybackOptions};

#[cfg(feature = "audio-ducking")]
use crate::ducking::{DuckConfig, DuckGuard, create_backend};

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
    /// decoding via rodio/symphonia first. Falls back to a host player
    /// subprocess if native decoding fails or is unavailable.
    ///
    /// Note: If ducking is configured, use [`play_async`] instead as ducking
    /// requires an async runtime.
    pub fn play(self) -> Result<(), PlaybackError> {
        let format = self.audio.format();

        // Try native playback first (non-URL, non-forced-host)
        #[cfg(feature = "native-playback")]
        if !self.force_host && !matches!(self.audio.source_kind(), AudioSourceKind::Url) {
            match crate::native_player::play_native(self.audio.data_ref(), format, &self.options) {
                Ok(()) => {
                    if self.show_meta {
                        self.print_native_meta(format);
                    }
                    return Ok(());
                }
                Err(crate::native_player::NativePlaybackError::Timeout(_)) => {
                    // Audio device timed out — the entire audio subsystem is
                    // likely unresponsive. Host players (afplay, mpv, etc.)
                    // will also hang, so bail immediately.
                    return Err(PlaybackError::AudioDeviceUnavailable(
                        "audio device timed out — audio subsystem may be unresponsive".into(),
                    ));
                }
                Err(_) => {
                    // Format/decode/IO error — fall through to host player
                }
            }
        }

        // Host player fallback
        let player = self.select_player(format)?;
        if self.show_meta {
            self.print_meta(player, format);
        }
        playa_with_player_and_options(player, self.audio.into_data(), self.options)
    }

    /// Play the audio asynchronously with optional ducking support.
    ///
    /// When the `native-playback` feature is enabled, attempts in-process
    /// decoding via rodio/symphonia first. Falls back to a host player
    /// subprocess if native decoding fails or is unavailable.
    ///
    /// Ducking (if configured) is set up **before** the native/host decision
    /// so both playback paths benefit from audio attenuation.
    ///
    /// Requires the `audio-ducking` feature flag for ducking support.
    #[cfg(feature = "audio-ducking")]
    pub async fn play_async(self) -> Result<(), PlaybackError> {
        let format = self.audio.format();

        // Set up ducking BEFORE playback (covers both native and host paths)
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

        // Try native playback first (non-URL, non-forced-host)
        #[cfg(feature = "native-playback")]
        if !self.force_host && !matches!(self.audio.source_kind(), AudioSourceKind::Url) {
            match crate::native_player::play_native(self.audio.data_ref(), format, &self.options) {
                Ok(()) => {
                    if self.show_meta {
                        self.print_native_meta(format);
                    }
                    if let Some(guard) = guard {
                        guard.restore().await;
                    }
                    return Ok(());
                }
                Err(crate::native_player::NativePlaybackError::Timeout(_)) => {
                    if let Some(guard) = guard {
                        guard.restore().await;
                    }
                    return Err(PlaybackError::AudioDeviceUnavailable(
                        "audio device timed out — audio subsystem may be unresponsive".into(),
                    ));
                }
                Err(_) => {
                    // Format/decode/IO error — fall through to host player
                }
            }
        }

        // Host player fallback
        let player = self.select_player(format)?;
        if self.show_meta {
            self.print_meta(player, format);
        }
        let result = playa_with_player_and_options(player, self.audio.into_data(), self.options);

        if let Some(guard) = guard {
            guard.restore().await;
        }

        result
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

    #[test]
    fn force_host_builder_sets_flag() {
        // We can't easily construct a Playa without valid audio, but we can
        // test the builder method indirectly by checking the struct field.
        // Use from_bytes with minimal WAV header.
        let wav_header: Vec<u8> = {
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
        };
        let playa = Playa::from_bytes(wav_header).unwrap();
        assert!(!playa.force_host);
        let playa = playa.force_host();
        assert!(playa.force_host);
    }
}
