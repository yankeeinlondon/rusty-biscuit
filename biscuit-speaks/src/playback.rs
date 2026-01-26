//! Audio playback utilities for TTS output.
//!
//! This module provides cross-platform audio playback functionality using the
//! playa audio playback library.
//!
//! ## Features
//!
//! - Multi-provider audio playback with automatic player selection
//! - Volume and speed control where the player supports it
//! - Support for WAV, MP3, Ogg, and PCM formats
//!
//! ## Requirements
//!
//! This module requires the `playa` feature to be enabled. Without it, no
//! playback functions are available.
//!
//! ## Examples
//!
//! ```ignore
//! use biscuit_speaks::playback::{play_audio_bytes, play_audio_file};
//! use biscuit_speaks::{AudioFormat, TtsConfig, VolumeLevel, SpeedLevel};
//! use std::path::Path;
//!
//! // Play audio bytes
//! let config = TtsConfig::new()
//!     .with_volume(VolumeLevel::Soft)
//!     .with_speed(SpeedLevel::Fast);
//! play_audio_bytes(&wav_data, AudioFormat::Wav, &config).await?;
//!
//! // Play an audio file
//! play_audio_file(Path::new("/tmp/audio.mp3"), AudioFormat::Mp3, &config).await?;
//! ```

#[cfg(feature = "playa")]
use crate::errors::TtsError;
#[cfg(feature = "playa")]
use crate::types::AudioFormat;

// ============================================================================
// Playa-Based Playback Functions (feature-gated)
// ============================================================================

/// Play audio bytes using playa with config-based volume and speed.
///
/// This function uses playa's multi-provider audio playback system which
/// automatically selects the best available player and supports volume
/// and speed control where the underlying player allows.
///
/// ## Arguments
///
/// * `data` - The raw audio bytes to play.
/// * `format` - The audio format of the data.
/// * `config` - TTS configuration containing volume and speed settings.
///
/// ## Errors
///
/// Returns `TtsError` if:
/// - No compatible audio player is available
/// - The player process fails to spawn
/// - Temp file creation fails (for byte data)
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::playback::play_audio_bytes;
/// use biscuit_speaks::{AudioFormat, TtsConfig, VolumeLevel, SpeedLevel};
///
/// let wav_data: Vec<u8> = /* ... */;
/// let config = TtsConfig::new()
///     .with_volume(VolumeLevel::Soft)
///     .with_speed(SpeedLevel::Fast);
/// play_audio_bytes(&wav_data, AudioFormat::Wav, &config).await?;
/// ```
#[cfg(feature = "playa")]
pub async fn play_audio_bytes(
    data: &[u8],
    format: AudioFormat,
    config: &crate::types::TtsConfig,
) -> Result<(), TtsError> {
    use crate::playa_bridge::{to_playa_audio_data, to_playa_format, to_playa_options};

    let playa_format = to_playa_format(format);
    let options = to_playa_options(config.volume, config.speed);
    let audio_data = to_playa_audio_data(data.to_vec());

    tracing::debug!(
        format = ?format,
        volume = ?config.volume,
        speed = ?config.speed,
        data_len = data.len(),
        "Playing audio bytes via playa"
    );

    playa::playa_explicit_with_options_async(playa_format, audio_data, options)
        .await
        .map_err(TtsError::from)
}

/// Play an audio file using playa with config-based volume and speed.
///
/// This function uses playa's multi-provider audio playback system which
/// automatically selects the best available player and supports volume
/// and speed control where the underlying player allows.
///
/// ## Arguments
///
/// * `path` - Path to the audio file to play.
/// * `format` - The audio format of the file.
/// * `config` - TTS configuration containing volume and speed settings.
///
/// ## Errors
///
/// Returns `TtsError` if:
/// - No compatible audio player is available
/// - The player process fails to spawn
/// - The file path is invalid
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::playback::play_audio_file;
/// use biscuit_speaks::{AudioFormat, TtsConfig, VolumeLevel};
/// use std::path::Path;
///
/// let config = TtsConfig::new().with_volume(VolumeLevel::Loud);
/// play_audio_file(Path::new("/tmp/audio.mp3"), AudioFormat::Mp3, &config).await?;
/// ```
#[cfg(feature = "playa")]
pub async fn play_audio_file(
    path: &std::path::Path,
    format: AudioFormat,
    config: &crate::types::TtsConfig,
) -> Result<(), TtsError> {
    use crate::playa_bridge::{to_playa_format, to_playa_options};

    let playa_format = to_playa_format(format);
    let options = to_playa_options(config.volume, config.speed);
    let audio_data = playa::AudioData::FilePath(path.to_path_buf());

    tracing::debug!(
        path = %path.display(),
        format = ?format,
        volume = ?config.volume,
        speed = ?config.speed,
        "Playing audio file via playa"
    );

    playa::playa_explicit_with_options_async(playa_format, audio_data, options)
        .await
        .map_err(TtsError::from)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    // ========================================================================
    // Playa-based playback tests (feature-gated)
    // ========================================================================

    #[cfg(feature = "playa")]
    mod playa_tests {
        use super::*;
        use crate::types::{SpeedLevel, TtsConfig, VolumeLevel};

        /// Test that play_audio_bytes has correct async signature.
        ///
        /// This is a compile-time test - if it compiles, the signature is correct.
        #[allow(dead_code)]
        fn assert_play_audio_bytes_signature() {
            fn _check<F: std::future::Future<Output = Result<(), TtsError>>>(_f: F) {}

            let config = TtsConfig::new();
            let data: &[u8] = &[];
            _check(play_audio_bytes(data, AudioFormat::Wav, &config));
        }

        /// Test that play_audio_file has correct async signature.
        ///
        /// This is a compile-time test - if it compiles, the signature is correct.
        #[allow(dead_code)]
        fn assert_play_audio_file_signature() {
            fn _check<F: std::future::Future<Output = Result<(), TtsError>>>(_f: F) {}

            let config = TtsConfig::new();
            let path = std::path::Path::new("/tmp/test.wav");
            _check(play_audio_file(path, AudioFormat::Wav, &config));
        }

        /// Test that TtsConfig volume/speed are passed through correctly.
        #[test]
        fn test_config_options_used_in_conversion() {
            use crate::playa_bridge::to_playa_options;

            let config = TtsConfig::new()
                .with_volume(VolumeLevel::Soft)
                .with_speed(SpeedLevel::Fast);

            let options = to_playa_options(config.volume, config.speed);

            // VolumeLevel::Soft = 0.5, SpeedLevel::Fast = 1.25
            assert_eq!(options.volume, Some(0.5));
            assert_eq!(options.speed, Some(1.25));
        }

        /// Test that explicit volume/speed values are passed through.
        #[test]
        fn test_explicit_config_options() {
            use crate::playa_bridge::to_playa_options;

            let config = TtsConfig::new()
                .with_volume(VolumeLevel::Explicit(0.33))
                .with_speed(SpeedLevel::Explicit(1.8));

            let options = to_playa_options(config.volume, config.speed);

            assert_eq!(options.volume, Some(0.33));
            assert_eq!(options.speed, Some(1.8));
        }

        /// Test that default config uses Normal values.
        #[test]
        fn test_default_config_options() {
            use crate::playa_bridge::to_playa_options;

            let config = TtsConfig::new();
            let options = to_playa_options(config.volume, config.speed);

            // VolumeLevel::Normal = 0.75, SpeedLevel::Normal = 1.0
            assert_eq!(options.volume, Some(0.75));
            assert_eq!(options.speed, Some(1.0));
        }

        /// Test format conversion for all AudioFormat variants.
        #[test]
        fn test_all_formats_convert_to_playa() {
            use crate::playa_bridge::to_playa_format;

            // Just verify these don't panic - detailed assertions are in playa_bridge tests
            let _ = to_playa_format(AudioFormat::Wav);
            let _ = to_playa_format(AudioFormat::Mp3);
            let _ = to_playa_format(AudioFormat::Ogg);
            let _ = to_playa_format(AudioFormat::Pcm);
        }
    }
}
