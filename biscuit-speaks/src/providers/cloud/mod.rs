//! Cloud-based TTS provider implementations.
//!
//! These providers use HTTP APIs for TTS generation.

mod elevenlabs;

pub use elevenlabs::ElevenLabsProvider;
