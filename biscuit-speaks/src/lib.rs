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

pub mod detection;
pub mod errors;
pub mod playback;
pub mod providers;
pub mod speak;
pub mod traits;
pub mod types;

// Re-export main types at crate root for convenience
pub use detection::{get_available_providers, get_providers_for_strategy, parse_provider_name};
pub use errors::{AllProvidersFailed, TtsError};
pub use providers::cloud::ElevenLabsProvider;
pub use speak::{speak, speak_when_able, Speak};
pub use traits::TtsExecutor;
pub use types::{
    AudioFormat, CloudTtsProvider, Gender, HostTtsProvider, Language, TtsConfig,
    TtsFailoverStrategy, TtsProvider, VolumeLevel,
};
