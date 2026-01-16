use crate::types::VolumeLevel;

/// **Speak** struct
///
/// This is the primary primitive which will be used
/// for TTS functionality.
pub struct Speak {
    /// the text which will be spoken
    pub text: String,
    /// cached audio of cloud based TTS
    audio: Option<Vec<u8>>,

    /// the volume which the text should be spoken
    pub volume: VolumeLevel,
    /// the voice a user has requested to use
    requested_voice: Option<String>,
    pub voice: Voice,
}

impl Speak {
    pub fn new<T: Into<String>>(text: T) -> Speak {
        Speak {
            text: text.into(),
            audio: None,
            volume: VolumeLevel::Normal,

            voice: Voice,
        }
    }
}
