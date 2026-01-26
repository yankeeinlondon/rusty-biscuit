//! Biscuit Speaks
//!
//! Cross-platform text-to-speech library with support for multiple TTS providers.
//!
//! ## Features
//!
//! - **Multi-provider support**: Host-based CLI tools (macOS `say`, eSpeak, etc.)
//!   and cloud APIs (ElevenLabs)
//! - **Automatic failover**: Tries providers in priority order until one succeeds
//! - **OS-aware defaults**: Prioritizes native TTS on each platform
//! - **Async-first**: Built on tokio for non-blocking TTS operations
//! - **Builder pattern**: Ergonomic configuration via `TtsConfig`
//!
//! ## Quick Start
//!
//! ```ignore
//! use biscuit_speaks::{Speak, TtsConfig};
//!
//! // Simple usage with defaults
//! Speak::new("Hello, world!").play().await?;
//!
//! // With configuration
//! Speak::new("Custom voice")
//!     .with_voice("Samantha")
//!     .with_volume(VolumeLevel::Soft)
//!     .play()
//!     .await?;
//! ```
//!
//! ## Module Structure
//!
//! - [`types`] - Core type definitions (providers, config, audio formats)
//! - [`errors`] - Error types for TTS operations
//! - [`traits`] - The `TtsExecutor` trait for provider implementations
//! - [`speak`] - The main `Speak` struct for TTS operations

pub mod cache;
pub mod detection;
pub mod errors;
pub mod gender_inference;
pub mod playback;
#[cfg(feature = "playa")]
mod playa_bridge;
pub mod providers;
pub mod speak;
pub mod traits;
pub mod types;

// Re-export main types at crate root for convenience
pub use cache::{
    bust_host_capability_cache, populate_cache_for_all_providers, populate_cache_for_provider,
    read_from_cache, update_provider_in_cache,
};
pub use detection::{get_available_providers, get_providers_for_strategy, parse_provider_name};
pub use gender_inference::infer_gender;
pub use errors::{AllProvidersFailed, TtsError};
pub use providers::cloud::ElevenLabsProvider;
pub use providers::host::{
    EchogardenEngine, EchogardenProvider, ESpeakProvider, GttsProvider, KokoroTtsProvider,
    SapiProvider, SayProvider,
};
pub use speak::{speak, speak_when_able, speak_with_result, Speak};
pub use traits::{TtsExecutor, TtsVoiceInventory};
pub use types::{
    AudioFormat, CloudTtsProvider, Gender, HostTtsCapabilities, HostTtsCapability,
    HostTtsProvider, Language, SpeakResult, SpeedLevel, TtsConfig, TtsFailoverStrategy,
    TtsProvider, Voice, VoiceQuality, VolumeLevel,
};

// Playa-based playback functions (feature-gated)
#[cfg(feature = "playa")]
pub use crate::playback::{play_audio_bytes_playa, play_audio_file_playa};
