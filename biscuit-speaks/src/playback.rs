//! Audio playback utilities for TTS output.
//!
//! This module provides cross-platform audio playback functionality using
//! system audio players (afplay on macOS, paplay/aplay on Linux, PowerShell on Windows).

use std::path::Path;

use tempfile::NamedTempFile;

use crate::errors::TtsError;
use crate::types::AudioFormat;

// ============================================================================
// OS-Specific Audio Players
// ============================================================================

/// Audio players by platform preference.
#[cfg(target_os = "macos")]
const AUDIO_PLAYERS: &[&str] = &["afplay"];

/// Audio players by platform preference.
#[cfg(target_os = "linux")]
const AUDIO_PLAYERS: &[&str] = &["paplay", "aplay", "play", "mpv", "ffplay"];

/// Audio players by platform preference.
#[cfg(target_os = "windows")]
const AUDIO_PLAYERS: &[&str] = &["powershell"];

/// Fallback for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
const AUDIO_PLAYERS: &[&str] = &["ffplay", "play"];

// ============================================================================
// Player Detection
// ============================================================================

/// Get the first available audio player on the system.
///
/// This function checks for audio players in priority order and returns
/// the first one found. The check is synchronous since it only involves
/// path lookups.
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
pub fn get_audio_player() -> Option<&'static str> {
    for &player in AUDIO_PLAYERS {
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
pub async fn play_audio_bytes(data: &[u8], format: AudioFormat) -> Result<(), TtsError> {
    let extension = format.extension();

    // Create temp file with correct extension
    let temp_file = NamedTempFile::with_suffix(&format!(".{}", extension))
        .map_err(|e| TtsError::TempFileError { source: e })?;

    // Write audio data
    tokio::fs::write(temp_file.path(), data).await?;

    // Play the file
    play_audio_file(temp_file.path()).await

    // temp_file is automatically cleaned up on drop
}

/// Play an audio file using the system audio player.
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
pub async fn play_audio_file(path: &Path) -> Result<(), TtsError> {
    let player = get_audio_player().ok_or(TtsError::NoAudioPlayer)?;

    let args = build_player_args(player, path);

    tracing::debug!(player = player, path = %path.display(), "Playing audio file");

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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_players_not_empty() {
        assert!(!AUDIO_PLAYERS.is_empty());
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
    fn test_get_audio_player_does_not_panic() {
        let _ = get_audio_player();
    }
}
