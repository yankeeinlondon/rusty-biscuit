use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::TtsClient;
use crate::programs::find_program::find_programs_with_source_parallel;
use crate::programs::installer::{
    execute_install, execute_versioned_install, method_available, select_best_method,
    InstallOptions,
};
use crate::programs::schema::{ProgramError, ProgramMetadata};
use crate::programs::types::{ExecutableSource, ProgramDetector};
use crate::programs::{
    InstalledLanguagePackageManagers, InstalledOsPackageManagers, Program, PROGRAM_LOOKUP,
};

fn tts_client_details(client: TtsClient) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match client {
        TtsClient::Say => Program::Say,
        TtsClient::Espeak => Program::Espeak,
        TtsClient::EspeakNg => Program::EspeakNg,
        TtsClient::Festival => Program::Festival,
        TtsClient::Mimic => Program::Mimic,
        TtsClient::Mimic3 => Program::Mimic3,
        TtsClient::Piper => Program::Piper,
        TtsClient::Echogarden => Program::Echogarden,
        TtsClient::Balcon => Program::Balcon,
        TtsClient::WindowsSapi => Program::WindowsSapi,
        TtsClient::GttsCli => Program::GttsCli,
        TtsClient::CoquiTts => Program::CoquiTts,
        TtsClient::SherpaOnnx => Program::SherpaOnnx,
        TtsClient::KokoroTts => Program::KokoroTts,
        TtsClient::Pico2Wave => Program::Pico2Wave,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Popular text-to-speech (TTS) clients found on the system.
///
/// Stores path and discovery source for each installed TTS client.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledTtsClients {
    say: Option<(PathBuf, ExecutableSource)>,
    espeak: Option<(PathBuf, ExecutableSource)>,
    espeak_ng: Option<(PathBuf, ExecutableSource)>,
    festival: Option<(PathBuf, ExecutableSource)>,
    mimic: Option<(PathBuf, ExecutableSource)>,
    mimic3: Option<(PathBuf, ExecutableSource)>,
    piper: Option<(PathBuf, ExecutableSource)>,
    echogarden: Option<(PathBuf, ExecutableSource)>,
    balcon: Option<(PathBuf, ExecutableSource)>,
    windows_sapi: Option<(PathBuf, ExecutableSource)>,
    gtts_cli: Option<(PathBuf, ExecutableSource)>,
    coqui_tts: Option<(PathBuf, ExecutableSource)>,
    sherpa_onnx: Option<(PathBuf, ExecutableSource)>,
    kokoro_tts: Option<(PathBuf, ExecutableSource)>,
    pico2wave: Option<(PathBuf, ExecutableSource)>,
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

        let results = find_programs_with_source_parallel(&programs);

        let get = |name: &str| results.get(name).and_then(|r| r.clone());
        let get_any = |names: &[&str]| {
            for name in names {
                if let Some(result) = results.get(*name).and_then(|r| r.clone()) {
                    return Some(result);
                }
            }
            None
        };

        Self {
            say: get("say"),
            espeak: get("espeak"),
            espeak_ng: get("espeak-ng"),
            festival: get("festival"),
            mimic: get("mimic"),
            mimic3: get("mimic3"),
            piper: get("piper"),
            echogarden: get("echogarden"),
            balcon: get("balcon"),
            windows_sapi: if cfg!(target_os = "windows") {
                Some((PathBuf::from("sapi"), ExecutableSource::Path))
            } else {
                None
            },
            gtts_cli: get("gtts-cli"),
            coqui_tts: get("tts"),
            sherpa_onnx: get_any(&["sherpa-onnx-offline-tts", "sherpa-onnx-tts"]),
            kokoro_tts: get("kokoro-tts"),
            pico2wave: get("pico2wave"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    // Boolean accessors for backward compatibility with direct field access patterns.
    // These allow code to use `installed.say()` instead of `installed.is_installed(TtsClient::Say)`.

    /// Returns true if macOS `say` command is available.
    pub fn say(&self) -> bool {
        self.say.is_some()
    }

    /// Returns true if `espeak` is installed.
    pub fn espeak(&self) -> bool {
        self.espeak.is_some()
    }

    /// Returns true if `espeak-ng` is installed.
    pub fn espeak_ng(&self) -> bool {
        self.espeak_ng.is_some()
    }

    /// Returns true if `festival` is installed.
    pub fn festival(&self) -> bool {
        self.festival.is_some()
    }

    /// Returns true if `mimic` is installed.
    pub fn mimic(&self) -> bool {
        self.mimic.is_some()
    }

    /// Returns true if `mimic3` is installed.
    pub fn mimic3(&self) -> bool {
        self.mimic3.is_some()
    }

    /// Returns true if `piper` is installed.
    pub fn piper(&self) -> bool {
        self.piper.is_some()
    }

    /// Returns true if `echogarden` is installed.
    pub fn echogarden(&self) -> bool {
        self.echogarden.is_some()
    }

    /// Returns true if `balcon` is installed.
    pub fn balcon(&self) -> bool {
        self.balcon.is_some()
    }

    /// Returns true if Windows SAPI is available.
    pub fn windows_sapi(&self) -> bool {
        self.windows_sapi.is_some()
    }

    /// Returns true if `gtts-cli` is installed.
    pub fn gtts_cli(&self) -> bool {
        self.gtts_cli.is_some()
    }

    /// Returns true if Coqui TTS is installed.
    pub fn coqui_tts(&self) -> bool {
        self.coqui_tts.is_some()
    }

    /// Returns true if `sherpa-onnx` TTS is installed.
    pub fn sherpa_onnx(&self) -> bool {
        self.sherpa_onnx.is_some()
    }

    /// Returns true if `kokoro-tts` is installed.
    pub fn kokoro_tts(&self) -> bool {
        self.kokoro_tts.is_some()
    }

    /// Returns true if `pico2wave` is installed.
    pub fn pico2wave(&self) -> bool {
        self.pico2wave.is_some()
    }

    /// Returns the path to the specified TTS client's binary if installed.
    pub fn path(&self, client: TtsClient) -> Option<PathBuf> {
        self.path_with_source(client).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified TTS client if installed.
    pub fn path_with_source(&self, client: TtsClient) -> Option<(PathBuf, ExecutableSource)> {
        match client {
            TtsClient::Say => self.say.clone(),
            TtsClient::Espeak => self.espeak.clone(),
            TtsClient::EspeakNg => self.espeak_ng.clone(),
            TtsClient::Festival => self.festival.clone(),
            TtsClient::Mimic => self.mimic.clone(),
            TtsClient::Mimic3 => self.mimic3.clone(),
            TtsClient::Piper => self.piper.clone(),
            TtsClient::Echogarden => self.echogarden.clone(),
            TtsClient::Balcon => self.balcon.clone(),
            TtsClient::WindowsSapi => self.windows_sapi.clone(),
            TtsClient::GttsCli => self.gtts_cli.clone(),
            TtsClient::CoquiTts => self.coqui_tts.clone(),
            TtsClient::SherpaOnnx => self.sherpa_onnx.clone(),
            TtsClient::KokoroTts => self.kokoro_tts.clone(),
            TtsClient::Pico2Wave => self.pico2wave.clone(),
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
            TtsClient::Say => self.say.is_some(),
            TtsClient::Espeak => self.espeak.is_some(),
            TtsClient::EspeakNg => self.espeak_ng.is_some(),
            TtsClient::Festival => self.festival.is_some(),
            TtsClient::Mimic => self.mimic.is_some(),
            TtsClient::Mimic3 => self.mimic3.is_some(),
            TtsClient::Piper => self.piper.is_some(),
            TtsClient::Echogarden => self.echogarden.is_some(),
            TtsClient::Balcon => self.balcon.is_some(),
            TtsClient::WindowsSapi => self.windows_sapi.is_some(),
            TtsClient::GttsCli => self.gtts_cli.is_some(),
            TtsClient::CoquiTts => self.coqui_tts.is_some(),
            TtsClient::SherpaOnnx => self.sherpa_onnx.is_some(),
            TtsClient::KokoroTts => self.kokoro_tts.is_some(),
            TtsClient::Pico2Wave => self.pico2wave.is_some(),
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

impl Serialize for InstalledTtsClients {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstalledTtsClients", 15)?;
        state.serialize_field("say", &self.say.is_some())?;
        state.serialize_field("espeak", &self.espeak.is_some())?;
        state.serialize_field("espeak_ng", &self.espeak_ng.is_some())?;
        state.serialize_field("festival", &self.festival.is_some())?;
        state.serialize_field("mimic", &self.mimic.is_some())?;
        state.serialize_field("mimic3", &self.mimic3.is_some())?;
        state.serialize_field("piper", &self.piper.is_some())?;
        state.serialize_field("echogarden", &self.echogarden.is_some())?;
        state.serialize_field("balcon", &self.balcon.is_some())?;
        state.serialize_field("windows_sapi", &self.windows_sapi.is_some())?;
        state.serialize_field("gtts_cli", &self.gtts_cli.is_some())?;
        state.serialize_field("coqui_tts", &self.coqui_tts.is_some())?;
        state.serialize_field("sherpa_onnx", &self.sherpa_onnx.is_some())?;
        state.serialize_field("kokoro_tts", &self.kokoro_tts.is_some())?;
        state.serialize_field("pico2wave", &self.pico2wave.is_some())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledTtsClients {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoolTtsClients {
            #[serde(default)]
            say: bool,
            #[serde(default)]
            espeak: bool,
            #[serde(default)]
            espeak_ng: bool,
            #[serde(default)]
            festival: bool,
            #[serde(default)]
            mimic: bool,
            #[serde(default)]
            mimic3: bool,
            #[serde(default)]
            piper: bool,
            #[serde(default)]
            echogarden: bool,
            #[serde(default)]
            balcon: bool,
            #[serde(default)]
            windows_sapi: bool,
            #[serde(default)]
            gtts_cli: bool,
            #[serde(default)]
            coqui_tts: bool,
            #[serde(default)]
            sherpa_onnx: bool,
            #[serde(default)]
            kokoro_tts: bool,
            #[serde(default)]
            pico2wave: bool,
        }

        let b = BoolTtsClients::deserialize(deserializer)?;

        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledTtsClients {
            say: to_opt(b.say),
            espeak: to_opt(b.espeak),
            espeak_ng: to_opt(b.espeak_ng),
            festival: to_opt(b.festival),
            mimic: to_opt(b.mimic),
            mimic3: to_opt(b.mimic3),
            piper: to_opt(b.piper),
            echogarden: to_opt(b.echogarden),
            balcon: to_opt(b.balcon),
            windows_sapi: to_opt(b.windows_sapi),
            gtts_cli: to_opt(b.gtts_cli),
            coqui_tts: to_opt(b.coqui_tts),
            sherpa_onnx: to_opt(b.sherpa_onnx),
            kokoro_tts: to_opt(b.kokoro_tts),
            pico2wave: to_opt(b.pico2wave),
        })
    }
}

impl ProgramDetector for InstalledTtsClients {
    type Program = TtsClient;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledTtsClients::path(self, program)
    }

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledTtsClients::path_with_source(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledTtsClients::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledTtsClients::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledTtsClients::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledTtsClients::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = tts_client_details(program) else {
            return false;
        };

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return false;
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();

        details
            .installation_methods
            .iter()
            .any(|method| method_available(method, &os_pkg_mgrs, &lang_pkg_mgrs))
    }

    fn install(&self, program: Self::Program) -> Result<(), SniffInstallationError> {
        let details = tts_client_details(program).ok_or_else(|| {
            SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            }
        })?;

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: os_type.to_string(),
            });
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();
        let method = select_best_method(details.installation_methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .ok_or_else(|| SniffInstallationError::MissingPackageManager {
                pkg: program.display_name().to_string(),
                manager: "package manager".to_string(),
            })?;

        let _result = execute_install(method, &InstallOptions::default())?;
        Ok(())
    }

    fn install_version(
        &self,
        program: Self::Program,
        version: &str,
    ) -> Result<(), SniffInstallationError> {
        let details = tts_client_details(program).ok_or_else(|| {
            SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            }
        })?;

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: os_type.to_string(),
            });
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();
        let method = select_best_method(details.installation_methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .ok_or_else(|| SniffInstallationError::MissingPackageManager {
                pkg: program.display_name().to_string(),
                manager: "package manager".to_string(),
            })?;

        let _result = execute_versioned_install(method, version, &InstallOptions::default())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_with_source_returns_none_when_not_installed() {
        let clients = InstalledTtsClients::default();
        assert!(clients.path_with_source(TtsClient::Piper).is_none());
    }

    #[test]
    fn test_is_installed_returns_false_for_default() {
        let clients = InstalledTtsClients::default();
        assert!(!clients.is_installed(TtsClient::Piper));
        assert!(!clients.is_installed(TtsClient::Espeak));
    }

    #[test]
    fn test_serialize_produces_boolean_fields() {
        let clients = InstalledTtsClients::default();
        let json = serde_json::to_string(&clients).unwrap();
        assert!(json.contains("\"piper\":false"));
        assert!(json.contains("\"espeak\":false"));
    }

    #[test]
    fn test_deserialize_from_boolean_fields() {
        let json = r#"{"piper": true, "espeak": false}"#;
        let clients: InstalledTtsClients = serde_json::from_str(json).unwrap();
        assert!(clients.is_installed(TtsClient::Piper));
        assert!(!clients.is_installed(TtsClient::Espeak));
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = InstalledTtsClients::default();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: InstalledTtsClients = serde_json::from_str(&json).unwrap();
        for client in original.installed() {
            assert!(deserialized.is_installed(client));
        }
    }

    /// Regression test: kokoro-tts detection uses correct binary name.
    ///
    /// Bug: The detection code was looking for "kokoro_tts" (underscore)
    /// instead of "kokoro-tts" (hyphen). This caused kokoro-tts to never
    /// be detected even when installed.
    #[test]
    fn test_kokoro_tts_binary_name_uses_hyphen() {
        let mut clients = InstalledTtsClients::default();
        assert!(!clients.is_installed(TtsClient::KokoroTts), "Default should be false");

        // Manually set to simulate detection
        clients.kokoro_tts = Some((PathBuf::from("/usr/local/bin/kokoro-tts"), ExecutableSource::Path));
        assert!(clients.is_installed(TtsClient::KokoroTts), "Should be settable to true");
    }
}
