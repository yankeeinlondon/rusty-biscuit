use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "async")]
use std::ffi::OsString;

use crate::audio::AudioData;
use crate::detection::{
    detect_audio_format_from_bytes, detect_audio_format_from_path, detect_audio_format_from_url,
};
use crate::error::PlaybackError;
use crate::player::{match_available_players, AudioPlayer, PLAYER_LOOKUP};
use crate::types::{AudioFormat, PlaybackOptions};

/// Detect the format and play audio with the best available player.
pub async fn playa(audio: AudioData) -> Result<(), PlaybackError> {
    let format = match &audio {
        AudioData::FilePath(path) => detect_audio_format_from_path(path)?,
        AudioData::Url(url) => detect_audio_format_from_url(url.as_str()).await?,
        AudioData::Bytes(bytes) => detect_audio_format_from_bytes(bytes)?,
    };

    playa_explicit(format, audio)
}

/// Play audio using an explicitly provided audio format.
pub fn playa_explicit(format: AudioFormat, audio: AudioData) -> Result<(), PlaybackError> {
    playa_explicit_with_options(format, audio, PlaybackOptions::default())
}

/// Play audio using an explicitly provided audio format with options.
pub fn playa_explicit_with_options(
    format: AudioFormat,
    audio: AudioData,
    options: PlaybackOptions,
) -> Result<(), PlaybackError> {
    let player = select_player(format, &audio, &options)?;
    playa_with_player_and_options(player, audio, options)
}

/// Play audio using a specific player.
pub fn playa_with_player(player: AudioPlayer, audio: AudioData) -> Result<(), PlaybackError> {
    playa_with_player_and_options(player, audio, PlaybackOptions::default())
}

/// Play audio using a specific player with options.
pub fn playa_with_player_and_options(
    player: AudioPlayer,
    audio: AudioData,
    options: PlaybackOptions,
) -> Result<(), PlaybackError> {
    let metadata = PLAYER_LOOKUP
        .get(&player)
        .ok_or(PlaybackError::MissingPlayerMetadata { player })?;

    if matches!(audio, AudioData::Url(_)) && !metadata.takes_stream_input {
        return Err(PlaybackError::UnsupportedSource {
            player,
            source_kind: "url",
        });
    }

    let source = resolve_source(&audio)?;
    let mut command = build_player_command(player, metadata, &source, &options)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let status = command
        .status()
        .map_err(|source| PlaybackError::Spawn { player, source })?;

    if !status.success() {
        return Err(PlaybackError::PlayerFailed {
            player,
            exit_code: status.code(),
        });
    }

    Ok(())
}

// ============================================================================
// Async variants (feature-gated)
// ============================================================================

/// Detect the format and play audio with the best available player (async).
///
/// This is the async version of [`playa`]. It uses `tokio::process::Command`
/// for spawning the player process and `tokio::fs` for writing temporary files.
#[cfg(feature = "async")]
pub async fn playa_async(audio: AudioData) -> Result<(), PlaybackError> {
    let format = match &audio {
        AudioData::FilePath(path) => detect_audio_format_from_path(path)?,
        AudioData::Url(url) => detect_audio_format_from_url(url.as_str()).await?,
        AudioData::Bytes(bytes) => detect_audio_format_from_bytes(bytes)?,
    };

    playa_explicit_async(format, audio).await
}

/// Play audio using an explicitly provided audio format (async).
///
/// This is the async version of [`playa_explicit`].
#[cfg(feature = "async")]
pub async fn playa_explicit_async(
    format: AudioFormat,
    audio: AudioData,
) -> Result<(), PlaybackError> {
    playa_explicit_with_options_async(format, audio, PlaybackOptions::default()).await
}

/// Play audio using an explicitly provided audio format with options (async).
///
/// This is the async version of [`playa_explicit_with_options`].
#[cfg(feature = "async")]
pub async fn playa_explicit_with_options_async(
    format: AudioFormat,
    audio: AudioData,
    options: PlaybackOptions,
) -> Result<(), PlaybackError> {
    let player = select_player(format, &audio, &options)?;
    playa_with_player_and_options_async(player, audio, options).await
}

/// Play audio using a specific player (async).
///
/// This is the async version of [`playa_with_player`].
#[cfg(feature = "async")]
pub async fn playa_with_player_async(
    player: AudioPlayer,
    audio: AudioData,
) -> Result<(), PlaybackError> {
    playa_with_player_and_options_async(player, audio, PlaybackOptions::default()).await
}

/// Play audio using a specific player with options (async).
///
/// This is the async version of [`playa_with_player_and_options`]. It spawns
/// the player process asynchronously using `tokio::process::Command` and
/// waits for completion before returning.
#[cfg(feature = "async")]
pub async fn playa_with_player_and_options_async(
    player: AudioPlayer,
    audio: AudioData,
    options: PlaybackOptions,
) -> Result<(), PlaybackError> {
    let metadata = PLAYER_LOOKUP
        .get(&player)
        .ok_or(PlaybackError::MissingPlayerMetadata { player })?;

    if matches!(audio, AudioData::Url(_)) && !metadata.takes_stream_input {
        return Err(PlaybackError::UnsupportedSource {
            player,
            source_kind: "url",
        });
    }

    let source = resolve_source_async(&audio).await?;
    let (binary, args) = build_player_args(player, metadata, &source, &options)?;

    let mut command = tokio::process::Command::new(binary);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let status = command
        .status()
        .await
        .map_err(|source| PlaybackError::Spawn { player, source })?;

    if !status.success() {
        return Err(PlaybackError::PlayerFailed {
            player,
            exit_code: status.code(),
        });
    }

    Ok(())
}

fn select_player(
    format: AudioFormat,
    audio: &AudioData,
    options: &PlaybackOptions,
) -> Result<AudioPlayer, PlaybackError> {
    let players = match_available_players(format);
    let selected = players.into_iter().find(|candidate| {
        let Some(metadata) = PLAYER_LOOKUP.get(candidate) else {
            return false;
        };
        // Filter by URL capability
        if matches!(audio, AudioData::Url(_)) && !metadata.takes_stream_input {
            return false;
        }
        // Filter by required capabilities
        if options.requires_speed_control() && !metadata.supports_speed_control {
            return false;
        }
        if options.requires_volume_control() && !metadata.supports_volume_control {
            return false;
        }
        true
    });

    selected.ok_or_else(|| {
        if options.requires_speed_control() || options.requires_volume_control() {
            PlaybackError::NoPlayerWithCapabilities {
                format,
                needs_speed: options.requires_speed_control(),
                needs_volume: options.requires_volume_control(),
            }
        } else {
            PlaybackError::NoCompatiblePlayer { format }
        }
    })
}

fn build_player_command(
    player: AudioPlayer,
    metadata: &crate::player::Player,
    source: &ResolvedSource,
    options: &PlaybackOptions,
) -> Result<Command, PlaybackError> {
    let mut command = Command::new(metadata.binary_name());

    match player {
        // Tier 1: Full controllability (speed + volume + stream)
        AudioPlayer::Mpv => {
            command
                .arg("--no-video")
                .arg("--no-terminal")
                .arg("--really-quiet");
            if let Some(vol) = options.volume {
                command.arg(format!("--volume={}", (vol * 100.0) as i32));
            }
            if let Some(speed) = options.speed {
                command.arg(format!("--speed={}", speed));
            }
            source.apply(&mut command);
        }
        AudioPlayer::FfPlay => {
            command
                .arg("-nodisp")
                .arg("-autoexit")
                .arg("-loglevel")
                .arg("quiet");
            if let Some(vol) = options.volume {
                command.arg("-volume").arg(((vol * 100.0) as i32).to_string());
            }
            if let Some(speed) = options.speed {
                // FFplay uses audio filter for tempo; clamp to 0.5-2.0
                let clamped = speed.clamp(0.5, 2.0);
                command.arg("-af").arg(format!("atempo={}", clamped));
            }
            source.apply(&mut command);
        }
        AudioPlayer::Sox => {
            command.arg("-q");
            if let Some(vol) = options.volume {
                command.arg("-v").arg(vol.to_string());
            }
            source.apply(&mut command);
            // Speed effect must come AFTER the source file
            if let Some(speed) = options.speed {
                command.arg("speed").arg(speed.to_string());
            }
        }

        // Tier 2: Volume + stream (no speed control)
        AudioPlayer::Vlc => {
            command.arg("--quiet").arg("--play-and-exit");
            if let Some(vol) = options.volume {
                // VLC gain ranges from 0.0-2.0
                command.arg(format!("--gain={}", vol * 2.0));
            }
            source.apply(&mut command);
        }
        AudioPlayer::MPlayer => {
            command.arg("-really-quiet");
            if let Some(vol) = options.volume {
                command
                    .arg("-softvol")
                    .arg("-volume")
                    .arg(((vol * 100.0) as i32).to_string());
            }
            source.apply(&mut command);
        }
        AudioPlayer::GstreamerGstPlay => {
            command.arg("--quiet");
            if let Some(vol) = options.volume {
                command.arg(format!("--volume={}", vol));
            }
            source.apply(&mut command);
        }

        // Tier 3: Volume only (Linux audio subsystems)
        AudioPlayer::PulseaudioPaplay => {
            if let Some(vol) = options.volume {
                // paplay volume: 0-65536, where 65536 = 100%
                command.arg(format!("--volume={}", (vol * 65536.0) as u32));
            }
            source.apply(&mut command);
        }
        AudioPlayer::Pipewire => {
            if let Some(vol) = options.volume {
                command.arg(format!("--volume={}", vol));
            }
            source.apply(&mut command);
        }

        // Tier 3: Stream only (no volume/speed control)
        AudioPlayer::Mpg123 => {
            command.arg("-q");
            // Note: volume/speed options ignored (not supported)
            source.apply(&mut command);
        }
        AudioPlayer::Ogg123 => {
            command.arg("-q");
            // Note: volume/speed options ignored (not supported)
            source.apply(&mut command);
        }

        // Tier 4: No controllability
        AudioPlayer::AlsaAplay => {
            command.arg("-q");
            // Note: volume/speed options ignored (not supported)
            source.apply(&mut command);
        }

        // Tier 1: macOS native with speed + volume (but no stdin)
        AudioPlayer::MacOsAfplay => {
            // afplay -v <volume 0.0-1.0> -r <rate 0.4-3.0> <file>
            if let Some(vol) = options.volume {
                command.arg("-v").arg(vol.to_string());
            }
            if let Some(speed) = options.speed {
                let clamped = speed.clamp(0.4, 3.0);
                command.arg("-r").arg(clamped.to_string());
            }
            source.apply(&mut command);
        }

        // Tier 4: stdin streaming (playback is default mode)
        AudioPlayer::PulseaudioPacat => {
            // pacat reads from file or stdin by default in playback mode
            // No volume/speed flags available
            source.apply(&mut command);
        }
    }

    Ok(command)
}

fn resolve_source(audio: &AudioData) -> Result<ResolvedSource, PlaybackError> {
    match audio {
        AudioData::FilePath(path) => Ok(ResolvedSource::Path(path.clone())),
        AudioData::Url(url) => Ok(ResolvedSource::Url(url.as_str().to_string())),
        AudioData::Bytes(bytes) => {
            let path = write_temp_audio(bytes.as_ref())?;
            Ok(ResolvedSource::Path(path))
        }
    }
}

fn write_temp_audio(bytes: &[u8]) -> Result<PathBuf, PlaybackError> {
    let mut attempts = 0;
    while attempts < 3 {
        attempts += 1;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_nanos(0))
            .as_nanos();
        let filename = format!("playa-{}-{}.audio", std::process::id(), timestamp);
        let path = std::env::temp_dir().join(filename);

        if path.exists() {
            continue;
        }

        std::fs::write(&path, bytes)?;
        return Ok(path);
    }

    Err(PlaybackError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "failed to create unique temp file",
    )))
}

/// Generate a unique temp file path for audio bytes.
#[cfg(feature = "async")]
fn generate_temp_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_nanos(0))
        .as_nanos();
    let filename = format!("playa-{}-{}.audio", std::process::id(), timestamp);
    std::env::temp_dir().join(filename)
}

#[cfg(feature = "async")]
async fn write_temp_audio_async(bytes: &[u8]) -> Result<PathBuf, PlaybackError> {
    let mut attempts = 0;
    while attempts < 3 {
        attempts += 1;
        let path = generate_temp_path();

        // Use tokio::fs for async file check and write
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            continue;
        }

        tokio::fs::write(&path, bytes).await?;
        return Ok(path);
    }

    Err(PlaybackError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "failed to create unique temp file",
    )))
}

#[cfg(feature = "async")]
async fn resolve_source_async(audio: &AudioData) -> Result<ResolvedSource, PlaybackError> {
    match audio {
        AudioData::FilePath(path) => Ok(ResolvedSource::Path(path.clone())),
        AudioData::Url(url) => Ok(ResolvedSource::Url(url.as_str().to_string())),
        AudioData::Bytes(bytes) => {
            let path = write_temp_audio_async(bytes.as_ref()).await?;
            Ok(ResolvedSource::Path(path))
        }
    }
}

/// Build player arguments without creating a Command.
///
/// Returns the binary name and a list of arguments. This is used by the async
/// variant to construct a `tokio::process::Command`.
#[cfg(feature = "async")]
fn build_player_args(
    player: AudioPlayer,
    metadata: &crate::player::Player,
    source: &ResolvedSource,
    options: &PlaybackOptions,
) -> Result<(&'static str, Vec<OsString>), PlaybackError> {
    let mut args: Vec<OsString> = Vec::new();

    match player {
        // Tier 1: Full controllability (speed + volume + stream)
        AudioPlayer::Mpv => {
            args.push("--no-video".into());
            args.push("--no-terminal".into());
            args.push("--really-quiet".into());
            if let Some(vol) = options.volume {
                args.push(format!("--volume={}", (vol * 100.0) as i32).into());
            }
            if let Some(speed) = options.speed {
                args.push(format!("--speed={}", speed).into());
            }
            source.push_arg(&mut args);
        }
        AudioPlayer::FfPlay => {
            args.push("-nodisp".into());
            args.push("-autoexit".into());
            args.push("-loglevel".into());
            args.push("quiet".into());
            if let Some(vol) = options.volume {
                args.push("-volume".into());
                args.push(((vol * 100.0) as i32).to_string().into());
            }
            if let Some(speed) = options.speed {
                let clamped = speed.clamp(0.5, 2.0);
                args.push("-af".into());
                args.push(format!("atempo={}", clamped).into());
            }
            source.push_arg(&mut args);
        }
        AudioPlayer::Sox => {
            args.push("-q".into());
            if let Some(vol) = options.volume {
                args.push("-v".into());
                args.push(vol.to_string().into());
            }
            source.push_arg(&mut args);
            // Speed effect must come AFTER the source file
            if let Some(speed) = options.speed {
                args.push("speed".into());
                args.push(speed.to_string().into());
            }
        }

        // Tier 2: Volume + stream (no speed control)
        AudioPlayer::Vlc => {
            args.push("--quiet".into());
            args.push("--play-and-exit".into());
            if let Some(vol) = options.volume {
                args.push(format!("--gain={}", vol * 2.0).into());
            }
            source.push_arg(&mut args);
        }
        AudioPlayer::MPlayer => {
            args.push("-really-quiet".into());
            if let Some(vol) = options.volume {
                args.push("-softvol".into());
                args.push("-volume".into());
                args.push(((vol * 100.0) as i32).to_string().into());
            }
            source.push_arg(&mut args);
        }
        AudioPlayer::GstreamerGstPlay => {
            args.push("--quiet".into());
            if let Some(vol) = options.volume {
                args.push(format!("--volume={}", vol).into());
            }
            source.push_arg(&mut args);
        }

        // Tier 3: Volume only (Linux audio subsystems)
        AudioPlayer::PulseaudioPaplay => {
            if let Some(vol) = options.volume {
                args.push(format!("--volume={}", (vol * 65536.0) as u32).into());
            }
            source.push_arg(&mut args);
        }
        AudioPlayer::Pipewire => {
            if let Some(vol) = options.volume {
                args.push(format!("--volume={}", vol).into());
            }
            source.push_arg(&mut args);
        }

        // Tier 3: Stream only (no volume/speed control)
        AudioPlayer::Mpg123 => {
            args.push("-q".into());
            source.push_arg(&mut args);
        }
        AudioPlayer::Ogg123 => {
            args.push("-q".into());
            source.push_arg(&mut args);
        }

        // Tier 4: No controllability
        AudioPlayer::AlsaAplay => {
            args.push("-q".into());
            source.push_arg(&mut args);
        }

        // Tier 1: macOS native with speed + volume (but no stdin)
        AudioPlayer::MacOsAfplay => {
            if let Some(vol) = options.volume {
                args.push("-v".into());
                args.push(vol.to_string().into());
            }
            if let Some(speed) = options.speed {
                let clamped = speed.clamp(0.4, 3.0);
                args.push("-r".into());
                args.push(clamped.to_string().into());
            }
            source.push_arg(&mut args);
        }

        // Tier 4: stdin streaming (playback is default mode)
        AudioPlayer::PulseaudioPacat => {
            source.push_arg(&mut args);
        }
    }

    Ok((metadata.binary_name(), args))
}

enum ResolvedSource {
    Path(PathBuf),
    Url(String),
}

impl ResolvedSource {
    fn apply(&self, command: &mut Command) {
        match self {
            ResolvedSource::Path(path) => {
                command.arg(path);
            }
            ResolvedSource::Url(url) => {
                command.arg(url);
            }
        }
    }

    #[cfg(feature = "async")]
    fn push_arg(&self, args: &mut Vec<std::ffi::OsString>) {
        match self {
            ResolvedSource::Path(path) => {
                args.push(path.as_os_str().to_owned());
            }
            ResolvedSource::Url(url) => {
                args.push(url.into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    #[cfg(feature = "async")]
    use std::ffi::OsString;

    fn mock_source() -> ResolvedSource {
        ResolvedSource::Path(PathBuf::from("/tmp/test.wav"))
    }

    fn get_metadata(player: AudioPlayer) -> &'static crate::player::Player {
        PLAYER_LOOKUP.get(&player).unwrap()
    }

    #[test]
    fn build_command_mpv_basic() {
        let metadata = get_metadata(AudioPlayer::Mpv);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command = build_player_command(AudioPlayer::Mpv, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--no-video")));
        assert!(args.contains(&OsStr::new("--no-terminal")));
        assert!(args.contains(&OsStr::new("--really-quiet")));
    }

    #[test]
    fn build_command_mpv_with_volume_and_speed() {
        let metadata = get_metadata(AudioPlayer::Mpv);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.5).with_speed(1.25);
        let command = build_player_command(AudioPlayer::Mpv, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--volume=50")));
        assert!(args.contains(&OsStr::new("--speed=1.25")));
    }

    #[test]
    fn build_command_ffplay_basic() {
        let metadata = get_metadata(AudioPlayer::FfPlay);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::FfPlay, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-nodisp")));
        assert!(args.contains(&OsStr::new("-autoexit")));
        assert!(args.contains(&OsStr::new("-loglevel")));
        assert!(args.contains(&OsStr::new("quiet")));
    }

    #[test]
    fn build_command_ffplay_with_volume_and_speed() {
        let metadata = get_metadata(AudioPlayer::FfPlay);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.75).with_speed(1.5);
        let command =
            build_player_command(AudioPlayer::FfPlay, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-volume")));
        assert!(args.contains(&OsStr::new("75")));
        assert!(args.contains(&OsStr::new("-af")));
        assert!(args.contains(&OsStr::new("atempo=1.5")));
    }

    #[test]
    fn build_command_sox_basic() {
        let metadata = get_metadata(AudioPlayer::Sox);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command = build_player_command(AudioPlayer::Sox, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-q")));
    }

    #[test]
    fn build_command_sox_with_volume_and_speed() {
        let metadata = get_metadata(AudioPlayer::Sox);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.8).with_speed(1.2);
        let command = build_player_command(AudioPlayer::Sox, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-v")));
        assert!(args.contains(&OsStr::new("0.8")));
        // Speed effect should come after the source file
        let speed_pos = args.iter().position(|a| *a == OsStr::new("speed"));
        let source_pos = args
            .iter()
            .position(|a| *a == OsStr::new("/tmp/test.wav"));
        assert!(
            speed_pos.unwrap() > source_pos.unwrap(),
            "speed effect should come after source"
        );
    }

    #[test]
    fn build_command_vlc_basic() {
        let metadata = get_metadata(AudioPlayer::Vlc);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command = build_player_command(AudioPlayer::Vlc, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--quiet")));
        assert!(args.contains(&OsStr::new("--play-and-exit")));
    }

    #[test]
    fn build_command_vlc_with_volume() {
        let metadata = get_metadata(AudioPlayer::Vlc);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.5);
        let command = build_player_command(AudioPlayer::Vlc, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--gain=1")));
    }

    #[test]
    fn build_command_mplayer_basic() {
        let metadata = get_metadata(AudioPlayer::MPlayer);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::MPlayer, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-really-quiet")));
    }

    #[test]
    fn build_command_mplayer_with_volume() {
        let metadata = get_metadata(AudioPlayer::MPlayer);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.8);
        let command =
            build_player_command(AudioPlayer::MPlayer, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-softvol")));
        assert!(args.contains(&OsStr::new("-volume")));
        assert!(args.contains(&OsStr::new("80")));
    }

    #[test]
    fn build_command_gstreamer_basic() {
        let metadata = get_metadata(AudioPlayer::GstreamerGstPlay);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::GstreamerGstPlay, metadata, &source, &options)
                .unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--quiet")));
    }

    #[test]
    fn build_command_gstreamer_with_volume() {
        let metadata = get_metadata(AudioPlayer::GstreamerGstPlay);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.5);
        let command =
            build_player_command(AudioPlayer::GstreamerGstPlay, metadata, &source, &options)
                .unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--volume=0.5")));
    }

    #[test]
    fn build_command_paplay_basic() {
        let metadata = get_metadata(AudioPlayer::PulseaudioPaplay);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::PulseaudioPaplay, metadata, &source, &options)
                .unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("/tmp/test.wav")));
    }

    #[test]
    fn build_command_paplay_with_volume() {
        let metadata = get_metadata(AudioPlayer::PulseaudioPaplay);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.5);
        let command =
            build_player_command(AudioPlayer::PulseaudioPaplay, metadata, &source, &options)
                .unwrap();

        let args: Vec<_> = command.get_args().collect();
        // 0.5 * 65536 = 32768
        assert!(args.contains(&OsStr::new("--volume=32768")));
    }

    #[test]
    fn build_command_pipewire_basic() {
        let metadata = get_metadata(AudioPlayer::Pipewire);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::Pipewire, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("/tmp/test.wav")));
    }

    #[test]
    fn build_command_pipewire_with_volume() {
        let metadata = get_metadata(AudioPlayer::Pipewire);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.75);
        let command =
            build_player_command(AudioPlayer::Pipewire, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--volume=0.75")));
    }

    #[test]
    fn build_command_mpg123_basic() {
        let metadata = get_metadata(AudioPlayer::Mpg123);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::Mpg123, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-q")));
    }

    #[test]
    fn build_command_ogg123_basic() {
        let metadata = get_metadata(AudioPlayer::Ogg123);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::Ogg123, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-q")));
    }

    #[test]
    fn build_command_aplay_basic() {
        let metadata = get_metadata(AudioPlayer::AlsaAplay);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::AlsaAplay, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-q")));
    }

    #[test]
    fn build_command_afplay_basic() {
        let metadata = get_metadata(AudioPlayer::MacOsAfplay);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::MacOsAfplay, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("/tmp/test.wav")));
    }

    #[test]
    fn build_command_afplay_with_volume_and_speed() {
        let metadata = get_metadata(AudioPlayer::MacOsAfplay);
        let source = mock_source();
        let options = PlaybackOptions::new().with_volume(0.5).with_speed(1.5);
        let command =
            build_player_command(AudioPlayer::MacOsAfplay, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-v")));
        assert!(args.contains(&OsStr::new("0.5")));
        assert!(args.contains(&OsStr::new("-r")));
        assert!(args.contains(&OsStr::new("1.5")));
    }

    #[test]
    fn build_command_afplay_clamps_speed() {
        let metadata = get_metadata(AudioPlayer::MacOsAfplay);
        let source = mock_source();
        // Speed outside 0.4-3.0 range should be clamped
        let options = PlaybackOptions::new().with_speed(5.0);
        let command =
            build_player_command(AudioPlayer::MacOsAfplay, metadata, &source, &options).unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("-r")));
        assert!(args.contains(&OsStr::new("3"))); // clamped to 3.0
    }

    #[test]
    fn build_command_pacat_basic() {
        let metadata = get_metadata(AudioPlayer::PulseaudioPacat);
        let source = mock_source();
        let options = PlaybackOptions::default();
        let command =
            build_player_command(AudioPlayer::PulseaudioPacat, metadata, &source, &options)
                .unwrap();

        // pacat takes file path directly, no special flags
        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("/tmp/test.wav")));
    }

    // Async variant tests (feature-gated)
    #[cfg(feature = "async")]
    mod async_tests {
        use super::*;

        #[test]
        fn build_player_args_mpv_basic() {
            let metadata = get_metadata(AudioPlayer::Mpv);
            let source = mock_source();
            let options = PlaybackOptions::default();
            let (binary, args) =
                build_player_args(AudioPlayer::Mpv, metadata, &source, &options).unwrap();

            assert_eq!(binary, "mpv");
            assert!(args.contains(&OsString::from("--no-video")));
            assert!(args.contains(&OsString::from("--no-terminal")));
            assert!(args.contains(&OsString::from("--really-quiet")));
        }

        #[test]
        fn build_player_args_mpv_with_volume_and_speed() {
            let metadata = get_metadata(AudioPlayer::Mpv);
            let source = mock_source();
            let options = PlaybackOptions::new().with_volume(0.5).with_speed(1.25);
            let (binary, args) =
                build_player_args(AudioPlayer::Mpv, metadata, &source, &options).unwrap();

            assert_eq!(binary, "mpv");
            assert!(args.contains(&OsString::from("--volume=50")));
            assert!(args.contains(&OsString::from("--speed=1.25")));
        }

        #[test]
        fn build_player_args_ffplay_with_options() {
            let metadata = get_metadata(AudioPlayer::FfPlay);
            let source = mock_source();
            let options = PlaybackOptions::new().with_volume(0.75).with_speed(1.5);
            let (binary, args) =
                build_player_args(AudioPlayer::FfPlay, metadata, &source, &options).unwrap();

            assert_eq!(binary, "ffplay");
            assert!(args.contains(&OsString::from("-nodisp")));
            assert!(args.contains(&OsString::from("-volume")));
            assert!(args.contains(&OsString::from("75")));
            assert!(args.contains(&OsString::from("-af")));
            assert!(args.contains(&OsString::from("atempo=1.5")));
        }

        #[test]
        fn build_player_args_sox_speed_after_source() {
            let metadata = get_metadata(AudioPlayer::Sox);
            let source = mock_source();
            let options = PlaybackOptions::new().with_volume(0.8).with_speed(1.2);
            let (binary, args) =
                build_player_args(AudioPlayer::Sox, metadata, &source, &options).unwrap();

            assert_eq!(binary, "play");
            assert!(args.contains(&OsString::from("-q")));
            assert!(args.contains(&OsString::from("-v")));

            // Speed effect should come after the source file
            let speed_pos = args.iter().position(|a| *a == OsString::from("speed"));
            let source_pos = args
                .iter()
                .position(|a| a.to_str() == Some("/tmp/test.wav"));
            assert!(
                speed_pos.unwrap() > source_pos.unwrap(),
                "speed effect should come after source"
            );
        }

        #[tokio::test]
        async fn write_temp_audio_async_creates_file() {
            let bytes = b"test audio data";
            let path = write_temp_audio_async(bytes).await.unwrap();

            assert!(path.exists());
            assert!(path.to_string_lossy().contains("playa-"));

            // Cleanup
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}
