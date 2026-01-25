//! Host-based TTS provider implementations.
//!
//! These providers use CLI tools installed on the host system.

mod echogarden;
mod espeak;
mod gtts;
mod kokoro;
mod sapi;
mod say;

pub use echogarden::{EchogardenEngine, EchogardenProvider};
pub use espeak::ESpeakProvider;
pub use gtts::GttsProvider;
pub use kokoro::KokoroTtsProvider;
pub use sapi::SapiProvider;
pub use say::SayProvider;
