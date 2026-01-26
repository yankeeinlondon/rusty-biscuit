use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::audio::AudioData;
use crate::detection::{
    detect_audio_format_from_bytes, detect_audio_format_from_path, detect_audio_format_from_url,
};
use crate::error::PlaybackError;
use crate::player::{match_available_players, AudioPlayer, PLAYER_LOOKUP};
use crate::types::AudioFormat;

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
    let player = select_player(format, &audio)?;
    playa_with_player(player, audio)
}

/// Play audio using a specific player.
pub fn playa_with_player(player: AudioPlayer, audio: AudioData) -> Result<(), PlaybackError> {
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
    let mut command = build_player_command(player, metadata, &source)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command
        .spawn()
        .map_err(|source| PlaybackError::Spawn { player, source })?;
    Ok(())
}

fn select_player(format: AudioFormat, audio: &AudioData) -> Result<AudioPlayer, PlaybackError> {
    let players = match_available_players(format);
    let selected = players.into_iter().find(|candidate| {
        let Some(metadata) = PLAYER_LOOKUP.get(candidate) else {
            return false;
        };
        if matches!(audio, AudioData::Url(_)) {
            metadata.takes_stream_input
        } else {
            true
        }
    });

    selected.ok_or(PlaybackError::NoCompatiblePlayer { format })
}

fn build_player_command(
    player: AudioPlayer,
    metadata: &crate::player::Player,
    source: &ResolvedSource,
) -> Result<Command, PlaybackError> {
    let mut command = Command::new(metadata.binary_name());
    match player {
        AudioPlayer::Mpv => {
            command
                .arg("--no-video")
                .arg("--no-terminal")
                .arg("--really-quiet");
            source.apply(&mut command);
        }
        AudioPlayer::FfPlay => {
            command
                .arg("-nodisp")
                .arg("-autoexit")
                .arg("-loglevel")
                .arg("quiet");
            source.apply(&mut command);
        }
        AudioPlayer::AlsaAplay => {
            command.arg("-q");
            source.apply(&mut command);
        }
        _ => {
            return Err(PlaybackError::UnsupportedPlayer { player });
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
}
