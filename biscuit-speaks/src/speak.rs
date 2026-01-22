use crate::types::VolumeLevel;

/// **Speak** struct
///
/// This is the primary primitive which will be used
/// for TTS functionality.
///
/// Note: This is work-in-progress and currently unused.
/// The main TTS functionality is provided by `lib.rs`.
pub struct Speak {
    /// the text which will be spoken
    pub text: String,
    /// cached audio
    audio: Option<Vec<u8>>,

    /// the volume which the text should be spoken
    pub volume: VolumeLevel,
    /// the voice a user has requested to use
    requested_voice: Option<String>,
}

impl Speak {
    pub fn new<T: Into<String>>(text: T) -> Speak {
        Speak {
            text: text.into(),
            audio: None,
            volume: VolumeLevel::Normal,
            requested_voice: None,
        }
    }

    /// Starts generating the audio from the text.
    ///
    /// In some cases you may not need or want to
    /// call this as the TTS process is fast enough
    /// that any latency it may introduce is deemed
    /// acceptable.
    pub fn prepare(self) -> Speak {
        todo!()
    }
}
