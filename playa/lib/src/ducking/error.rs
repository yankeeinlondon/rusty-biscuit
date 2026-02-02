//! Error types for audio ducking operations.

use thiserror::Error;

/// Errors that can occur during audio ducking operations.
#[derive(Debug, Error)]
pub enum DuckingError {
    /// The ramp duration must be greater than zero.
    #[error("ramp duration must be > 0 ms, got {0}")]
    InvalidRampDuration(u32),

    /// The floor scalar must be between 0.0 and 1.0 inclusive.
    #[error("floor scalar must be in range 0.0..=1.0, got {0}")]
    InvalidFloorScalar(f32),

    /// Failed to snapshot current volume state.
    #[error("failed to snapshot volumes: {0}")]
    SnapshotFailed(String),

    /// Failed to apply volume fade.
    #[error("failed to apply fade: {0}")]
    FadeFailed(String),

    /// Failed to restore volumes.
    #[error("failed to restore volumes: {0}")]
    RestoreFailed(String),

    /// The ducking backend is not available on this platform.
    #[error("ducking backend unavailable: {0}")]
    BackendUnavailable(String),

    /// Platform-specific error from the audio subsystem.
    #[error("platform error: {0}")]
    Platform(String),
}
