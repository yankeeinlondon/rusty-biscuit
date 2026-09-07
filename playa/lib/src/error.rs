use thiserror::Error;

use crate::player::AudioPlayer;
use crate::types::AudioFormat;

/// Errors returned by audio format detection.
#[derive(Debug, Error)]
pub enum DetectionError {
    /// The format could not be determined.
    #[error("unknown audio format")]
    UnknownFormat,
    /// Content was detected as non-audio.
    #[error("not an audio file: detected {mime}")]
    NotAudio {
        /// The detected MIME type.
        mime: String,
    },
    /// The data is too short to identify reliably.
    #[error("insufficient data for detection (need at least {required} bytes, got {actual})")]
    InsufficientData {
        /// Minimum number of bytes required.
        required: usize,
        /// Actual number of bytes provided.
        actual: usize,
    },
    /// An IO error occurred while reading from disk.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// An HTTP error occurred during URL inspection.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// URL parsing failed.
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
}

/// Errors returned by playback helpers.
#[derive(Debug, Error)]
pub enum PlaybackError {
    /// The host executable did not register Playa's detached worker entry seam.
    #[error("the host executable did not install the detached Playa worker seam")]
    NoDetachedWorker,
    /// A detached spool record failed validation or used an incompatible protocol.
    #[error("detached playback protocol error: {detail}")]
    DetachedProtocol {
        /// Validation or compatibility failure detail.
        detail: String,
    },
    /// A detached worker could not be launched.
    #[error("failed to launch detached playback worker: {source}")]
    DetachedWorkerSpawn {
        /// The process-launch failure.
        source: std::io::Error,
    },
    /// Playback speed must be finite and greater than zero.
    #[error("invalid playback speed {speed}; expected a finite value greater than zero")]
    InvalidPlaybackSpeed {
        /// Rejected speed multiplier.
        speed: f32,
    },
    /// Detection failed while preparing audio for playback.
    #[error("audio detection failed: {0}")]
    Detection(#[from] DetectionError),
    /// No installed player can handle the requested format.
    #[error(
        "no compatible player available for {format:?}{}",
        format_install_hint()
    )]
    NoCompatiblePlayer {
        /// The requested format.
        format: AudioFormat,
    },
    /// No installed player supports the required capabilities.
    #[error(
        "no player for {format:?} with required capabilities (speed: {needs_speed}, volume: {needs_volume})"
    )]
    NoPlayerWithCapabilities {
        /// The requested audio format.
        format: AudioFormat,
        /// Whether speed control was required.
        needs_speed: bool,
        /// Whether volume control was required.
        needs_volume: bool,
    },
    /// Player metadata could not be found in the lookup table.
    #[error("player metadata missing for {player:?}")]
    MissingPlayerMetadata {
        /// The missing player.
        player: AudioPlayer,
    },
    /// Playback is not implemented for the chosen player yet.
    #[error("player {player:?} is not supported for playback yet")]
    UnsupportedPlayer {
        /// The unsupported player.
        player: AudioPlayer,
    },
    /// The player cannot handle the specified source type.
    #[error("player {player:?} cannot handle {source_kind} sources")]
    UnsupportedSource {
        /// The player being used.
        player: AudioPlayer,
        /// The source label.
        source_kind: &'static str,
    },
    /// Failed to spawn the player process.
    #[error("failed to spawn player {player:?}: {source}")]
    Spawn {
        /// The player being spawned.
        player: AudioPlayer,
        /// The underlying IO error.
        source: std::io::Error,
    },
    /// The player process exited with a non-zero status.
    #[error("player {player:?} failed with exit code {exit_code:?}")]
    PlayerFailed {
        /// The player that failed.
        player: AudioPlayer,
        /// The exit code, if available.
        exit_code: Option<i32>,
    },
    /// Native playback hit an audio-device failure that should be reported
    /// directly instead of silently falling through to another path.
    #[error("audio subsystem problem: {detail}")]
    AudioSubsystem {
        /// Human-readable details from the failing backend.
        detail: String,
    },
    /// The audio state lock was poisoned.
    #[error("audio state lock poisoned")]
    StateLock,
    /// Pause was requested but audio is not currently playing.
    #[error("audio is not currently playing")]
    NotPlaying,
    /// Resume was requested but audio is not paused.
    #[error("audio is not currently paused")]
    NotPaused,
    /// A generic IO failure occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors returned when constructing an Audio instance.
#[derive(Debug, Error)]
pub enum InvalidAudio {
    /// Audio detection failed.
    #[error("audio detection failed: {0}")]
    Detection(#[from] DetectionError),
}

#[cfg(target_os = "windows")]
fn format_install_hint() -> &'static str {
    " — install mpv (https://mpv.io) or FFmpeg (https://ffmpeg.org) for audio playback"
}

#[cfg(not(target_os = "windows"))]
fn format_install_hint() -> &'static str {
    ""
}
