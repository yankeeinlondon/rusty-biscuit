
/// Errors that can occur during TTS operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    /// TTS engine initialization failed.
    #[error("TTS initialization failed")]
    InitFailed {
        /// The underlying error from the TTS engine.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Voice selection failed.
    #[error("Voice selection failed: {reason}")]
    VoiceSelectionFailed {
        /// Description of why voice selection failed.
        reason: String,
    },

    /// Speech synthesis failed.
    #[error("Speech failed")]
    SpeechFailed {
        /// The underlying error from the TTS engine.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// No suitable voice was found for the requested language.
    #[error("No suitable voice found (language: {language})")]
    NoSuitableVoice {
        /// The language that was requested.
        language: String,
    },
}
