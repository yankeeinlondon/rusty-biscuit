//! Internal bridge module for playa integration.
//!
//! This module provides conversion utilities between biscuit-speaks types
//! and playa's audio playback types. It is NOT re-exported - for internal
//! use only.
//!
//! ## Type Mappings
//!
//! | biscuit-speaks        | playa                                           |
//! |-----------------------|-------------------------------------------------|
//! | `AudioFormat::Wav`    | `AudioFormat::new(Wav, Some(Pcm))`              |
//! | `AudioFormat::Mp3`    | `AudioFormat::new(Mp3, Some(Mp3))`              |
//! | `AudioFormat::Ogg`    | `AudioFormat::new(Ogg, Some(Vorbis))`           |
//! | `AudioFormat::Pcm`    | `AudioFormat::new(Wav, Some(Pcm))` (in WAV)     |
//! | `VolumeLevel`         | `PlaybackOptions.volume` (f32)                  |
//! | `SpeedLevel`          | `PlaybackOptions.speed` (f32)                   |

// Note: The feature gate is already applied at the module level in lib.rs,
// so we don't need #![cfg(feature = "playa")] here.

use std::sync::Arc;

use crate::types::{AudioFormat, SpeedLevel, VolumeLevel};

/// Convert biscuit-speaks `AudioFormat` to playa's `AudioFormat`.
///
/// ## Mapping Rules
///
/// - `Wav` -> `AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm))`
/// - `Mp3` -> `AudioFormat::new(AudioFileFormat::Mp3, Some(Codec::Mp3))`
/// - `Ogg` -> `AudioFormat::new(AudioFileFormat::Ogg, Some(Codec::Vorbis))`
/// - `Pcm` -> `AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm))` (raw PCM wrapped in WAV container)
pub(crate) fn to_playa_format(format: AudioFormat) -> playa::AudioFormat {
    use playa::{AudioFileFormat, Codec};

    match format {
        AudioFormat::Wav => playa::AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm)),
        AudioFormat::Mp3 => playa::AudioFormat::new(AudioFileFormat::Mp3, Some(Codec::Mp3)),
        AudioFormat::Ogg => playa::AudioFormat::new(AudioFileFormat::Ogg, Some(Codec::Vorbis)),
        // Raw PCM data is typically wrapped in a WAV container for playback
        AudioFormat::Pcm => playa::AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm)),
    }
}

/// Convert biscuit-speaks volume and speed levels to playa `PlaybackOptions`.
///
/// ## Volume Mapping
///
/// | VolumeLevel        | f32 value |
/// |--------------------|-----------|
/// | `Loud`             | 1.0       |
/// | `Soft`             | 0.5       |
/// | `Normal`           | 0.75      |
/// | `Explicit(v)`      | v (clamped 0.0-1.0) |
///
/// ## Speed Mapping
///
/// | SpeedLevel         | f32 value |
/// |--------------------|-----------|
/// | `Fast`             | 1.25      |
/// | `Slow`             | 0.75      |
/// | `Normal`           | 1.0       |
/// | `Explicit(v)`      | v (clamped 0.25-4.0) |
pub(crate) fn to_playa_options(volume: VolumeLevel, speed: SpeedLevel) -> playa::PlaybackOptions {
    playa::PlaybackOptions::new()
        .with_volume(volume.value())
        .with_speed(speed.value())
}

/// Wrap audio bytes in an `Arc` for use with playa's `AudioData::Bytes`.
///
/// Playa expects bytes wrapped in `Arc<Vec<u8>>` for efficient sharing
/// without copying.
pub(crate) fn to_playa_audio_data(bytes: Vec<u8>) -> playa::AudioData {
    playa::AudioData::Bytes(Arc::new(bytes))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use playa::{AudioFileFormat, Codec};

    // ========================================================================
    // AudioFormat conversion tests
    // ========================================================================

    #[test]
    fn test_wav_format_conversion() {
        let playa_format = to_playa_format(AudioFormat::Wav);
        assert_eq!(playa_format.file_format, AudioFileFormat::Wav);
        assert_eq!(playa_format.codec, Some(Codec::Pcm));
    }

    #[test]
    fn test_mp3_format_conversion() {
        let playa_format = to_playa_format(AudioFormat::Mp3);
        assert_eq!(playa_format.file_format, AudioFileFormat::Mp3);
        assert_eq!(playa_format.codec, Some(Codec::Mp3));
    }

    #[test]
    fn test_ogg_format_conversion() {
        let playa_format = to_playa_format(AudioFormat::Ogg);
        assert_eq!(playa_format.file_format, AudioFileFormat::Ogg);
        assert_eq!(playa_format.codec, Some(Codec::Vorbis));
    }

    #[test]
    fn test_pcm_format_conversion() {
        // Raw PCM is wrapped in WAV container for playback
        let playa_format = to_playa_format(AudioFormat::Pcm);
        assert_eq!(playa_format.file_format, AudioFileFormat::Wav);
        assert_eq!(playa_format.codec, Some(Codec::Pcm));
    }

    // ========================================================================
    // Volume conversion tests
    // ========================================================================

    #[test]
    fn test_volume_loud_is_1_0() {
        let options = to_playa_options(VolumeLevel::Loud, SpeedLevel::Normal);
        assert_eq!(options.volume, Some(1.0));
    }

    #[test]
    fn test_volume_soft_is_0_5() {
        let options = to_playa_options(VolumeLevel::Soft, SpeedLevel::Normal);
        assert_eq!(options.volume, Some(0.5));
    }

    #[test]
    fn test_volume_normal_is_0_75() {
        let options = to_playa_options(VolumeLevel::Normal, SpeedLevel::Normal);
        assert_eq!(options.volume, Some(0.75));
    }

    #[test]
    fn test_volume_explicit_value() {
        let options = to_playa_options(VolumeLevel::Explicit(0.3), SpeedLevel::Normal);
        assert_eq!(options.volume, Some(0.3));
    }

    #[test]
    fn test_volume_explicit_clamped_high() {
        let options = to_playa_options(VolumeLevel::Explicit(1.5), SpeedLevel::Normal);
        assert_eq!(options.volume, Some(1.0)); // Clamped to max
    }

    #[test]
    fn test_volume_explicit_clamped_low() {
        let options = to_playa_options(VolumeLevel::Explicit(-0.5), SpeedLevel::Normal);
        assert_eq!(options.volume, Some(0.0)); // Clamped to min
    }

    // ========================================================================
    // Speed conversion tests
    // ========================================================================

    #[test]
    fn test_speed_fast_is_1_25() {
        let options = to_playa_options(VolumeLevel::Normal, SpeedLevel::Fast);
        assert_eq!(options.speed, Some(1.25));
    }

    #[test]
    fn test_speed_slow_is_0_75() {
        let options = to_playa_options(VolumeLevel::Normal, SpeedLevel::Slow);
        assert_eq!(options.speed, Some(0.75));
    }

    #[test]
    fn test_speed_normal_is_1_0() {
        let options = to_playa_options(VolumeLevel::Normal, SpeedLevel::Normal);
        assert_eq!(options.speed, Some(1.0));
    }

    #[test]
    fn test_speed_explicit_value() {
        let options = to_playa_options(VolumeLevel::Normal, SpeedLevel::Explicit(1.5));
        assert_eq!(options.speed, Some(1.5));
    }

    #[test]
    fn test_speed_explicit_clamped_high() {
        let options = to_playa_options(VolumeLevel::Normal, SpeedLevel::Explicit(5.0));
        assert_eq!(options.speed, Some(4.0)); // Clamped to max
    }

    #[test]
    fn test_speed_explicit_clamped_low() {
        let options = to_playa_options(VolumeLevel::Normal, SpeedLevel::Explicit(0.1));
        assert_eq!(options.speed, Some(0.25)); // Clamped to min
    }

    // ========================================================================
    // Combined options tests
    // ========================================================================

    #[test]
    fn test_combined_volume_and_speed() {
        let options = to_playa_options(VolumeLevel::Soft, SpeedLevel::Fast);
        assert_eq!(options.volume, Some(0.5));
        assert_eq!(options.speed, Some(1.25));
    }

    // ========================================================================
    // AudioData conversion tests
    // ========================================================================

    #[test]
    fn test_audio_data_bytes_wrapping() {
        let bytes = vec![0x52, 0x49, 0x46, 0x46]; // RIFF header start
        let audio_data = to_playa_audio_data(bytes.clone());

        match audio_data {
            playa::AudioData::Bytes(arc_bytes) => {
                assert_eq!(*arc_bytes, bytes);
            }
            _ => panic!("Expected AudioData::Bytes variant"),
        }
    }

    #[test]
    fn test_audio_data_empty_bytes() {
        let bytes = Vec::new();
        let audio_data = to_playa_audio_data(bytes);

        match audio_data {
            playa::AudioData::Bytes(arc_bytes) => {
                assert!(arc_bytes.is_empty());
            }
            _ => panic!("Expected AudioData::Bytes variant"),
        }
    }
}
