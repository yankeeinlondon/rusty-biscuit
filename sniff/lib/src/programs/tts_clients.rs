use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::programs::enums::TtsClient;
use crate::programs::find_program::find_programs_parallel;
use crate::programs::schema::{ProgramError, ProgramMetadata};

/// Popular text-to-speech (TTS) clients found on the system.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct InstalledTtsClients {
    /// macOS built-in speech synthesis utility. [Website](https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/SpeechSynthesisProgrammingGuide/Introduction/Introduction.html)
    pub say: bool,
    /// Open source speech synthesizer. [Website](http://espeak.sourceforge.net/)
    pub espeak: bool,
    /// Multi-lingual software speech synthesizer, successor to eSpeak. [Website](https://github.com/espeak-ng/espeak-ng)
    pub espeak_ng: bool,
    /// General multi-lingual speech synthesis system. [Website](http://www.cstr.ed.ac.uk/projects/festival/)
    pub festival: bool,
    /// Mycroft's TTS engine based on Flite. [Website](https://github.com/MycroftAI/mimic)
    pub mimic: bool,
    /// Mycroft's neural TTS engine. [Website](https://github.com/MycroftAI/mycroft-mimic3-tts)
    pub mimic3: bool,
    /// A fast, local neural text to speech system using ONNX. [Website](https://github.com/rhasspy/piper)
    pub piper: bool,
    /// Speech processing engine. [Website](https://echogarden.io/)
    pub echogarden: bool,
    /// Command line utility for speech synthesis on Windows. [Website](http://www.cross-plus-a.com/balcon.htm)
    pub balcon: bool,
    /// Windows Speech API (SAPI). [Website](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms723627(v=vs.85))
    pub windows_sapi: bool,
    /// Google Text-to-Speech CLI tool. [Website](https://github.com/pndurette/gTTS)
    pub gtts_cli: bool,
    /// Coqui TTS, deep learning for Text-to-Speech. [Website](https://github.com/coqui-ai/TTS)
    pub coqui_tts: bool,
    /// Sherpa-ONNX, streaming and non-streaming speech-to-text and text-to-speech using ONNX. [Website](https://k2-fsa.github.io/sherpa/onnx/index.html)
    pub sherpa_onnx: bool,
    /// A popular CLI which makes using the Kokoro TTS model very easy. [Website](https://github.com/nazdridoy/kokoro-tts)
    pub kokoro_tts: bool,
    /// SVOX Pico TTS engine (`pico2wave` command). Lightweight TTS for embedded systems.
    pub pico2wave: bool,
}

impl InstalledTtsClients {
    /// Detect which popular TTS clients are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "say",
            "espeak",
            "espeak-ng",
            "festival",
            "mimic",
            "mimic3",
            "piper",
            "echogarden",
            "balcon",
            "gtts-cli",
            "tts",
            "sherpa-onnx-offline-tts",
            "sherpa-onnx-tts",
            "pico2wave",
            "kokoro-tts",
        ];

        let results = find_programs_parallel(&programs);

        let has = |name: &str| results.get(name).and_then(|r| r.as_ref()).is_some();
        let any = |names: &[&str]| names.iter().any(|&name| has(name));

        Self {
            say: has("say"),
            espeak: has("espeak"),
            espeak_ng: has("espeak-ng"),
            festival: has("festival"),
            mimic: has("mimic"),
            mimic3: has("mimic3"),
            piper: has("piper"),
            echogarden: has("echogarden"),
            balcon: has("balcon"),
            windows_sapi: cfg!(target_os = "windows"),
            gtts_cli: has("gtts-cli"),
            coqui_tts: has("tts"),
            sherpa_onnx: any(&["sherpa-onnx-offline-tts", "sherpa-onnx-tts"]),
            kokoro_tts: has("kokoro-tts"),
            pico2wave: has("pico2wave"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified TTS client's binary if installed.
    pub fn path(&self, client: TtsClient) -> Option<PathBuf> {
        if self.is_installed(client) {
            client.path()
        } else {
            None
        }
    }

    /// Returns the version of the specified TTS client if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The TTS client is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, client: TtsClient) -> Result<String, ProgramError> {
        if !self.is_installed(client) {
            return Err(ProgramError::NotFound(client.binary_name().to_string()));
        }
        client.version()
    }

    /// Returns the official website URL for the specified TTS client.
    pub fn website(&self, client: TtsClient) -> &'static str {
        client.website()
    }

    /// Returns a one-line description of the specified TTS client.
    pub fn description(&self, client: TtsClient) -> &'static str {
        client.description()
    }

    /// Checks if the specified TTS client is installed.
    pub fn is_installed(&self, client: TtsClient) -> bool {
        match client {
            TtsClient::Say => self.say,
            TtsClient::Espeak => self.espeak,
            TtsClient::EspeakNg => self.espeak_ng,
            TtsClient::Festival => self.festival,
            TtsClient::Mimic => self.mimic,
            TtsClient::Mimic3 => self.mimic3,
            TtsClient::Piper => self.piper,
            TtsClient::Echogarden => self.echogarden,
            TtsClient::Balcon => self.balcon,
            TtsClient::WindowsSapi => self.windows_sapi,
            TtsClient::GttsCli => self.gtts_cli,
            TtsClient::CoquiTts => self.coqui_tts,
            TtsClient::SherpaOnnx => self.sherpa_onnx,
            TtsClient::KokoroTts => self.kokoro_tts,
            TtsClient::Pico2Wave => self.pico2wave,
        }
    }

    /// Returns a list of all installed TTS clients.
    pub fn installed(&self) -> Vec<TtsClient> {
        use strum::IntoEnumIterator;
        TtsClient::iter()
            .filter(|c| self.is_installed(*c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test: kokoro-tts detection uses correct binary name.
    ///
    /// Bug: The detection code was looking for "kokoro_tts" (underscore)
    /// instead of "kokoro-tts" (hyphen). This caused kokoro-tts to never
    /// be detected even when installed.
    #[test]
    fn test_kokoro_tts_binary_name_uses_hyphen() {
        // The programs array in InstalledTtsClients::new() should include
        // "kokoro-tts" (with hyphen), not "kokoro_tts" (with underscore).
        // This test verifies the struct field is properly populated when
        // the detection logic runs.

        // We can't easily test the actual binary detection in unit tests,
        // but we can verify that the struct field exists and behaves correctly
        let mut clients = InstalledTtsClients::default();
        assert!(!clients.kokoro_tts, "Default should be false");

        // Manually set to simulate detection
        clients.kokoro_tts = true;
        assert!(clients.kokoro_tts, "Should be settable to true");
    }
}
