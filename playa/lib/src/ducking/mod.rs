//! Audio ducking support for attenuating system audio during playback.
//!
//! This module provides the ability to temporarily lower the volume of other audio
//! sources while Playa plays, then restore them to their original levels.
//!
//! ## Feature Flags
//!
//! Audio ducking is gated behind the `audio-ducking` feature flag. Platform-specific
//! backends are automatically selected via `cfg(target_os)`:
//!
//! - macOS: CoreAudio virtual master volume
//! - Windows: WASAPI per-session volume
//! - Linux: PulseAudio/PipeWire sink inputs (with ALSA fallback)
//!
//! ## Example
//!
//! ```ignore
//! use playa::{Playa, DuckConfig};
//!
//! // Play with default ducking (1s ramp, 0.2 floor)
//! Playa::from_path("audio.mp3")?
//!     .with_ducked_audio(DuckConfig::default())
//!     .play()?;
//!
//! // Custom ducking parameters
//! let config = DuckConfig::new(500, 0.1)?;
//! Playa::from_path("audio.mp3")?
//!     .with_ducked_audio(config)
//!     .play()?;
//! ```

mod backend;
mod config;
mod envelope;
mod error;
mod factory;
mod guard;
mod types;

#[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
mod macos;

#[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
mod media_keys;

#[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]
mod windows;

#[cfg(all(target_os = "linux", feature = "audio-ducking-linux"))]
mod linux;

pub use backend::{DuckResult, DuckingBackend, NoopBackend};
pub use config::DuckConfig;
pub use envelope::{FadeStep, compute_fade_steps, interpolate_volume};
pub use error::DuckingError;
pub use factory::{backend_name, create_backend};
pub use guard::DuckGuard;
pub use types::{SessionId, SessionVolume, VolumeSnapshot};

#[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
pub use macos::MacOsBackend;

#[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
pub use media_keys::MediaKeysBackend;

#[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]
pub use windows::WindowsBackend;

#[cfg(all(target_os = "linux", feature = "audio-ducking-linux"))]
pub use linux::{AlsaBackend, LinuxBackend};

#[cfg(test)]
mod tests;
