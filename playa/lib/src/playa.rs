use std::path::PathBuf;

use crate::audio::Audio;
use crate::error::{InvalidAudio, PlaybackError};
use crate::playback::playa_with_player_and_options;
use crate::player::{match_available_players, AudioPlayer, Player, PLAYER_LOOKUP};
use crate::types::{AudioFormat, PlaybackOptions};

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
}

impl Playa {
    /// Create a new `Playa` from an `Audio` instance.
    pub fn new(audio: Audio) -> Self {
        Self {
            audio,
            options: PlaybackOptions::default(),
            show_meta: false,
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

    /// Return the detected audio format.
    pub fn format(&self) -> AudioFormat {
        self.audio.format()
    }

    /// Play the audio using the best available player.
    ///
    /// If `show_meta()` was called, prints playback metadata to STDOUT before
    /// starting playback.
    pub fn play(self) -> Result<(), PlaybackError> {
        let format = self.audio.format();
        let player = self.select_player(format)?;

        if self.show_meta {
            self.print_meta(player, format);
        }

        playa_with_player_and_options(player, self.audio.into_data(), self.options)
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
}
