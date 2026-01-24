//! Host-based TTS provider implementations.
//!
//! These providers use CLI tools installed on the host system.

mod say;
mod espeak;

pub use say::SayProvider;
pub use espeak::ESpeakProvider;
