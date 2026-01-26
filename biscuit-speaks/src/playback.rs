//! Audio playback utilities for TTS output.
//!
//! This module provides cross-platform audio playback functionality.
//!
//! ## Migration to Playa
//!
//! The original playback functions (`play_audio_bytes`, `play_audio_file`, etc.) use
//! a simple player detection system based on the `which` crate. These are now deprecated
//! in favor of the playa-based functions which provide:
//!
//! - More comprehensive player support across platforms
//! - Volume and speed control where the player supports it
//! - Better player capability detection
//!
//! To migrate, enable the `playa` feature and use:
//! - `play_audio_bytes_playa` instead of `play_audio_bytes`
//! - `play_audio_file_playa` instead of `play_audio_file` / `play_audio_file_with_format`
//!
//! The deprecated functions remain available for backwards compatibility.

use std::path::Path;

use tempfile::NamedTempFile;

use crate::errors::TtsError;
use crate::types::AudioFormat;

// ============================================================================
// OS-Specific Audio Players
// ============================================================================

/// Audio players by platform preference for WAV/PCM formats.
#[cfg(target_os = "macos")]
const WAV_PLAYERS: &[&str] = &["afplay"];

/// Audio players by platform preference for WAV/PCM formats.
/// paplay and aplay are preferred for WAV since they're lightweight.
#[cfg(target_os = "linux")]
const WAV_PLAYERS: &[&str] = &["paplay", "aplay", "play", "mpv", "ffplay"];

/// Audio players by platform preference for WAV/PCM formats.
#[cfg(target_os = "windows")]
const WAV_PLAYERS: &[&str] = &["powershell"];

/// Fallback for other platforms (WAV).
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
const WAV_PLAYERS: &[&str] = &["ffplay", "play"];

/// Audio players that support MP3 format.
/// On Linux, paplay/aplay do NOT support MP3 - they produce static!
/// We must use players with codec support (mpv, ffplay, play with MP3 support).
#[cfg(target_os = "macos")]
const MP3_PLAYERS: &[&str] = &["afplay"];

/// Audio players that support MP3 format on Linux.
/// IMPORTANT: paplay and aplay are excluded because they only support WAV/PCM.
#[cfg(target_os = "linux")]
const MP3_PLAYERS: &[&str] = &["mpv", "ffplay", "play"];

/// Audio players that support MP3 format on Windows.
#[cfg(target_os = "windows")]
const MP3_PLAYERS: &[&str] = &["powershell"];

/// Fallback MP3 players for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
const MP3_PLAYERS: &[&str] = &["mpv", "ffplay", "play"];


// ============================================================================
// Player Detection
// ============================================================================

/// Get the first available audio player on the system.
///
/// This function checks for audio players in priority order and returns
/// the first one found. The check is synchronous since it only involves
/// path lookups.
///
/// ## Deprecated
///
/// This function is deprecated. Enable the `playa` feature and use
/// `play_audio_bytes_playa` or `play_audio_file_playa` instead, which
/// handle player detection internally with better cross-platform support.
///
/// ## Returns
///
/// Returns `Some(&str)` with the player name if found, `None` otherwise.
///
/// ## Examples
///
/// ```
/// use biscuit_speaks::playback::get_audio_player;
///
/// if let Some(player) = get_audio_player() {
///     println!("Using audio player: {}", player);
/// }
/// ```
#[deprecated(since = "0.2.0", note = "Use play_audio_bytes_playa or play_audio_file_playa with the playa feature instead")]
#[allow(deprecated)]
pub fn get_audio_player() -> Option<&'static str> {
    get_audio_player_for_format(AudioFormat::Wav)
}

/// Get an audio player that supports the given format.
///
/// This is important on Linux where `paplay` and `aplay` only support
/// WAV/PCM formats. Trying to play MP3 through them produces static noise.
///
/// ## Deprecated
///
/// This function is deprecated. Enable the `playa` feature and use
/// `play_audio_bytes_playa` or `play_audio_file_playa` instead, which
/// handle player detection internally with better format support.
///
/// ## Arguments
///
/// * `format` - The audio format to find a player for.
///
/// ## Returns
///
/// Returns `Some(&str)` with the player name if found, `None` otherwise.
#[deprecated(since = "0.2.0", note = "Use play_audio_bytes_playa or play_audio_file_playa with the playa feature instead")]
pub fn get_audio_player_for_format(format: AudioFormat) -> Option<&'static str> {
    let players = match format {
        AudioFormat::Mp3 => MP3_PLAYERS,
        AudioFormat::Wav | AudioFormat::Pcm => WAV_PLAYERS,
        // For Ogg, use MP3 players since they generally support multiple codecs
        AudioFormat::Ogg => MP3_PLAYERS,
    };

    for &player in players {
        if which::which(player).is_ok() {
            return Some(player);
        }
    }
    None
}

// ============================================================================
// Playback Functions
// ============================================================================

/// Play audio bytes by writing to a temporary file and invoking the system player.
///
/// The temporary file is automatically cleaned up when the function returns.
///
/// ## Deprecated
///
/// This function is deprecated. Enable the `playa` feature and use
/// [`play_audio_bytes_playa`] instead, which provides volume/speed control
/// and better cross-platform player support.
///
/// ## Arguments
///
/// * `data` - The raw audio bytes to play.
/// * `format` - The audio format of the data.
///
/// ## Errors
///
/// Returns `TtsError` if:
/// - No audio player is available
/// - Temp file creation fails
/// - Audio playback fails
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::playback::play_audio_bytes;
/// use biscuit_speaks::AudioFormat;
///
/// let wav_data: Vec<u8> = /* ... */;
/// play_audio_bytes(&wav_data, AudioFormat::Wav).await?;
/// ```
#[deprecated(since = "0.2.0", note = "Use play_audio_bytes_playa with the playa feature instead")]
#[allow(deprecated)]
pub async fn play_audio_bytes(data: &[u8], format: AudioFormat) -> Result<(), TtsError> {
    let extension = format.extension();

    // Create temp file with correct extension
    let temp_file = NamedTempFile::with_suffix(&format!(".{}", extension))
        .map_err(|e| TtsError::TempFileError { source: e })?;

    // Write audio data
    tokio::fs::write(temp_file.path(), data).await?;

    // Play the file with a format-aware player
    play_audio_file_with_format(temp_file.path(), format).await

    // temp_file is automatically cleaned up on drop
}

/// Play an audio file using the system audio player.
///
/// Assumes WAV format. For other formats, use `play_audio_file_with_format`.
///
/// ## Deprecated
///
/// This function is deprecated. Enable the `playa` feature and use
/// [`play_audio_file_playa`] instead, which provides volume/speed control
/// and better cross-platform player support.
///
/// ## Arguments
///
/// * `path` - Path to the audio file to play.
///
/// ## Errors
///
/// Returns `TtsError` if:
/// - No audio player is available
/// - The player process fails to start
/// - The player exits with an error
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::playback::play_audio_file;
/// use std::path::Path;
///
/// play_audio_file(Path::new("/tmp/audio.wav")).await?;
/// ```
#[deprecated(since = "0.2.0", note = "Use play_audio_file_playa with the playa feature instead")]
#[allow(deprecated)]
pub async fn play_audio_file(path: &Path) -> Result<(), TtsError> {
    play_audio_file_with_format(path, AudioFormat::Wav).await
}

/// Play an audio file using a player that supports the given format.
///
/// This is important on Linux where `paplay` and `aplay` only support
/// WAV/PCM formats. Trying to play MP3 through them produces static noise.
///
/// ## Deprecated
///
/// This function is deprecated. Enable the `playa` feature and use
/// [`play_audio_file_playa`] instead, which provides volume/speed control
/// and better cross-platform player support.
///
/// ## Arguments
///
/// * `path` - Path to the audio file to play.
/// * `format` - The audio format of the file.
///
/// ## Errors
///
/// Returns `TtsError` if:
/// - No audio player is available for this format
/// - The player process fails to start
/// - The player exits with an error
#[deprecated(since = "0.2.0", note = "Use play_audio_file_playa with the playa feature instead")]
#[allow(deprecated)]
pub async fn play_audio_file_with_format(path: &Path, format: AudioFormat) -> Result<(), TtsError> {
    let player = get_audio_player_for_format(format).ok_or(TtsError::NoAudioPlayer)?;

    let args = build_player_args(player, path);

    tracing::debug!(
        player = player,
        path = %path.display(),
        format = ?format,
        "Playing audio file"
    );

    let output = tokio::process::Command::new(player)
        .args(&args)
        .output()
        .await
        .map_err(|e| TtsError::ProcessSpawnFailed {
            provider: player.to_string(),
            source: e,
        })?;

    if !output.status.success() {
        return Err(TtsError::PlaybackFailed {
            player: player.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Build the command-line arguments for the audio player.
fn build_player_args(player: &str, path: &Path) -> Vec<String> {
    let path_str = path.to_string_lossy().to_string();

    match player {
        "powershell" => {
            // Windows: Use PowerShell to play audio
            vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                format!(
                    "(New-Object Media.SoundPlayer '{}').PlaySync()",
                    path_str.replace('\'', "''")
                ),
            ]
        }
        "ffplay" => {
            // ffplay: Hide output, auto-exit
            vec![
                "-nodisp".to_string(),
                "-autoexit".to_string(),
                "-loglevel".to_string(),
                "quiet".to_string(),
                path_str,
            ]
        }
        "mpv" => {
            // mpv: No video, no terminal output
            vec![
                "--no-video".to_string(),
                "--really-quiet".to_string(),
                path_str,
            ]
        }
        _ => {
            // Default: just pass the file path (afplay, paplay, aplay, play)
            vec![path_str]
        }
    }
}

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
/// use biscuit_speaks::playback::play_audio_bytes_playa;
/// use biscuit_speaks::{AudioFormat, TtsConfig, VolumeLevel, SpeedLevel};
///
/// let wav_data: Vec<u8> = /* ... */;
/// let config = TtsConfig::new()
///     .with_volume(VolumeLevel::Soft)
///     .with_speed(SpeedLevel::Fast);
/// play_audio_bytes_playa(&wav_data, AudioFormat::Wav, &config).await?;
/// ```
#[cfg(feature = "playa")]
pub async fn play_audio_bytes_playa(
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
/// use biscuit_speaks::playback::play_audio_file_playa;
/// use biscuit_speaks::{AudioFormat, TtsConfig, VolumeLevel};
/// use std::path::Path;
///
/// let config = TtsConfig::new().with_volume(VolumeLevel::Loud);
/// play_audio_file_playa(Path::new("/tmp/audio.mp3"), AudioFormat::Mp3, &config).await?;
/// ```
#[cfg(feature = "playa")]
pub async fn play_audio_file_playa(
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
    use super::*;

    #[test]
    fn test_wav_players_not_empty() {
        assert!(!WAV_PLAYERS.is_empty());
    }

    #[test]
    fn test_mp3_players_not_empty() {
        assert!(!MP3_PLAYERS.is_empty());
    }

    #[test]
    fn test_build_player_args_default() {
        let args = build_player_args("afplay", Path::new("/tmp/test.wav"));
        assert_eq!(args, vec!["/tmp/test.wav"]);
    }

    #[test]
    fn test_build_player_args_powershell() {
        let args = build_player_args("powershell", Path::new("/tmp/test.wav"));
        assert_eq!(args.len(), 4);
        assert_eq!(args[0], "-NoProfile");
        assert!(args[3].contains("PlaySync"));
    }

    #[test]
    fn test_build_player_args_ffplay() {
        let args = build_player_args("ffplay", Path::new("/tmp/test.wav"));
        assert!(args.contains(&"-nodisp".to_string()));
        assert!(args.contains(&"-autoexit".to_string()));
    }

    #[test]
    fn test_build_player_args_mpv() {
        let args = build_player_args("mpv", Path::new("/tmp/test.wav"));
        assert!(args.contains(&"--no-video".to_string()));
    }

    // This test checks if get_audio_player works (may or may not find a player)
    #[test]
    #[allow(deprecated)]
    fn test_get_audio_player_does_not_panic() {
        let _ = get_audio_player();
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_audio_player_for_format_does_not_panic() {
        let _ = get_audio_player_for_format(AudioFormat::Wav);
        let _ = get_audio_player_for_format(AudioFormat::Mp3);
        let _ = get_audio_player_for_format(AudioFormat::Ogg);
        let _ = get_audio_player_for_format(AudioFormat::Pcm);
    }

    /// Regression test: MP3 playback on Linux should not use paplay/aplay.
    ///
    /// Bug: ElevenLabs returns MP3 audio. On Linux, paplay and aplay were
    /// selected first, but they only support WAV/PCM formats. Playing MP3
    /// through them produces static noise instead of audio.
    ///
    /// Fix: MP3_PLAYERS on Linux excludes paplay and aplay, preferring
    /// mpv, ffplay, or play (SoX) which support MP3 decoding.
    #[test]
    #[cfg(target_os = "linux")]
    fn test_mp3_players_excludes_paplay_aplay_on_linux() {
        // paplay and aplay should NOT be in the MP3 player list on Linux
        assert!(
            !MP3_PLAYERS.contains(&"paplay"),
            "paplay should not be in MP3_PLAYERS - it only supports WAV"
        );
        assert!(
            !MP3_PLAYERS.contains(&"aplay"),
            "aplay should not be in MP3_PLAYERS - it only supports WAV"
        );

        // mpv and ffplay should be available for MP3
        assert!(
            MP3_PLAYERS.contains(&"mpv") || MP3_PLAYERS.contains(&"ffplay"),
            "MP3_PLAYERS should include mpv or ffplay"
        );
    }

    /// Verify that WAV players include the lightweight options on Linux.
    #[test]
    #[cfg(target_os = "linux")]
    fn test_wav_players_includes_paplay_aplay_on_linux() {
        // For WAV, paplay and aplay should be preferred (they're lightweight)
        assert!(
            WAV_PLAYERS.contains(&"paplay") || WAV_PLAYERS.contains(&"aplay"),
            "WAV_PLAYERS should include paplay or aplay on Linux"
        );
    }

    // ========================================================================
    // Playa-based playback tests (feature-gated)
    // ========================================================================

    #[cfg(feature = "playa")]
    mod playa_tests {
        use super::*;
        use crate::types::{SpeedLevel, TtsConfig, VolumeLevel};

        /// Test that play_audio_bytes_playa has correct async signature.
        ///
        /// This is a compile-time test - if it compiles, the signature is correct.
        #[allow(dead_code)]
        fn assert_play_audio_bytes_playa_signature() {
            fn _check<F: std::future::Future<Output = Result<(), TtsError>>>(_f: F) {}

            let config = TtsConfig::new();
            let data: &[u8] = &[];
            _check(play_audio_bytes_playa(data, AudioFormat::Wav, &config));
        }

        /// Test that play_audio_file_playa has correct async signature.
        ///
        /// This is a compile-time test - if it compiles, the signature is correct.
        #[allow(dead_code)]
        fn assert_play_audio_file_playa_signature() {
            fn _check<F: std::future::Future<Output = Result<(), TtsError>>>(_f: F) {}

            let config = TtsConfig::new();
            let path = std::path::Path::new("/tmp/test.wav");
            _check(play_audio_file_playa(path, AudioFormat::Wav, &config));
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
