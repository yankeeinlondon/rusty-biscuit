//! TTS client detection — type alias with backward-compatible accessors.

use std::path::PathBuf;

use crate::programs::enums::TtsClient;
use crate::programs::types::{CategoryDetector, ExecutableSource};

/// Text-to-speech clients found on the system.
pub type InstalledTtsClients = CategoryDetector<TtsClient>;

/// Backward-compatible convenience methods for InstalledTtsClients.
impl InstalledTtsClients {
    /// Returns true if macOS Say is installed.
    pub fn say(&self) -> bool {
        self.is_installed(TtsClient::Say)
    }

    /// Returns true if eSpeak is installed.
    pub fn espeak(&self) -> bool {
        self.is_installed(TtsClient::Espeak)
    }

    /// Returns true if eSpeak-NG is installed.
    pub fn espeak_ng(&self) -> bool {
        self.is_installed(TtsClient::EspeakNg)
    }

    /// Returns true if Festival is installed.
    pub fn festival(&self) -> bool {
        self.is_installed(TtsClient::Festival)
    }

    /// Returns true if Mimic is installed.
    pub fn mimic(&self) -> bool {
        self.is_installed(TtsClient::Mimic)
    }

    /// Returns true if Mimic3 is installed.
    pub fn mimic3(&self) -> bool {
        self.is_installed(TtsClient::Mimic3)
    }

    /// Returns true if Piper is installed.
    pub fn piper(&self) -> bool {
        self.is_installed(TtsClient::Piper)
    }

    /// Returns true if Echogarden is installed.
    pub fn echogarden(&self) -> bool {
        self.is_installed(TtsClient::Echogarden)
    }

    /// Returns true if Balcon is installed.
    pub fn balcon(&self) -> bool {
        self.is_installed(TtsClient::Balcon)
    }

    /// Returns true if Windows SAPI is available.
    pub fn windows_sapi(&self) -> bool {
        self.is_installed(TtsClient::WindowsSapi)
    }

    /// Returns true if gTTS CLI is installed.
    pub fn gtts_cli(&self) -> bool {
        self.is_installed(TtsClient::GttsCli)
    }

    /// Returns true if Coqui TTS is installed.
    pub fn coqui_tts(&self) -> bool {
        self.is_installed(TtsClient::CoquiTts)
    }

    /// Returns true if Sherpa-ONNX is installed.
    pub fn sherpa_onnx(&self) -> bool {
        self.is_installed(TtsClient::SherpaOnnx)
    }

    /// Returns true if Kokoro TTS is installed.
    pub fn kokoro_tts(&self) -> bool {
        self.is_installed(TtsClient::KokoroTts)
    }

    /// Returns true if Pico2Wave is installed.
    pub fn pico2wave(&self) -> bool {
        self.is_installed(TtsClient::Pico2Wave)
    }

    /// Mark a client as installed (for testing purposes).
    pub fn with_client(self, client: TtsClient) -> Self {
        use crate::programs::schema::ProgramMetadata;
        let info = client.info();
        let fake_path = PathBuf::from(format!("/usr/bin/{}", info.binary_name));
        self.with_program(client, fake_path, ExecutableSource::Path)
    }
}
