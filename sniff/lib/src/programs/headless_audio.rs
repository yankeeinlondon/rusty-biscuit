use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::HeadlessAudio;
use crate::programs::find_program::find_programs_with_source_parallel;
use crate::programs::installer::{
    execute_install, execute_versioned_install, method_available, select_best_method,
    InstallOptions,
};
use crate::programs::schema::{ProgramEntry, ProgramError, ProgramMetadata};
use crate::programs::types::{ExecutableSource, ProgramDetector};
use crate::programs::{
    InstalledLanguagePackageManagers, InstalledOsPackageManagers, Program, PROGRAM_LOOKUP,
};

fn headless_audio_details(
    player: HeadlessAudio,
) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match player {
        HeadlessAudio::Mpv => Program::Mpv,
        HeadlessAudio::Ffplay => Program::Ffplay,
        HeadlessAudio::Vlc => Program::Vlc,
        HeadlessAudio::MPlayer => Program::MPlayer,
        HeadlessAudio::GstreamerGstPlay => Program::GstreamerGstPlay,
        HeadlessAudio::Sox => Program::Sox,
        HeadlessAudio::Mpg123 => Program::Mpg123,
        HeadlessAudio::Ogg123 => Program::Ogg123,
        HeadlessAudio::AlsaAplay => Program::AlsaAplay,
        HeadlessAudio::MacOsAfplay => Program::MacOsAfplay,
        HeadlessAudio::PulseaudioPaplay => Program::PulseaudioPaplay,
        HeadlessAudio::PulseaudioPacat => Program::PulseaudioPacat,
        HeadlessAudio::Pipewire => Program::Pipewire,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Headless audio players found on the system.
///
/// Stores path and discovery source for each installed audio player.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledHeadlessAudio {
    mpv: Option<(PathBuf, ExecutableSource)>,
    ffplay: Option<(PathBuf, ExecutableSource)>,
    vlc: Option<(PathBuf, ExecutableSource)>,
    mplayer: Option<(PathBuf, ExecutableSource)>,
    gstreamer_gst_play: Option<(PathBuf, ExecutableSource)>,
    sox: Option<(PathBuf, ExecutableSource)>,
    mpg123: Option<(PathBuf, ExecutableSource)>,
    ogg123: Option<(PathBuf, ExecutableSource)>,
    alsa_aplay: Option<(PathBuf, ExecutableSource)>,
    macos_afplay: Option<(PathBuf, ExecutableSource)>,
    pulseaudio_paplay: Option<(PathBuf, ExecutableSource)>,
    pulseaudio_pacat: Option<(PathBuf, ExecutableSource)>,
    pipewire: Option<(PathBuf, ExecutableSource)>,
}

impl InstalledHeadlessAudio {
    /// Detect which headless audio players are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "mpv",
            "ffplay",
            "vlc",
            "cvlc",
            "mplayer",
            "gst-play-1.0",
            "gst-play",
            "play",
            "sox",
            "mpg123",
            "ogg123",
            "aplay",
            "afplay",
            "paplay",
            "pacat",
            "pw-cat",
            "pw-play",
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
            mpv: get("mpv"),
            ffplay: get("ffplay"),
            vlc: get_any(&["vlc", "cvlc"]),
            mplayer: get("mplayer"),
            gstreamer_gst_play: get_any(&["gst-play-1.0", "gst-play"]),
            sox: get_any(&["play", "sox"]),
            mpg123: get("mpg123"),
            ogg123: get("ogg123"),
            alsa_aplay: get("aplay"),
            macos_afplay: get("afplay"),
            pulseaudio_paplay: get("paplay"),
            pulseaudio_pacat: get("pacat"),
            pipewire: get_any(&["pw-cat", "pw-play"]),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified audio player's binary if installed.
    pub fn path(&self, player: HeadlessAudio) -> Option<PathBuf> {
        self.path_with_source(player).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified audio player if installed.
    pub fn path_with_source(&self, player: HeadlessAudio) -> Option<(PathBuf, ExecutableSource)> {
        match player {
            HeadlessAudio::Mpv => self.mpv.clone(),
            HeadlessAudio::Ffplay => self.ffplay.clone(),
            HeadlessAudio::Vlc => self.vlc.clone(),
            HeadlessAudio::MPlayer => self.mplayer.clone(),
            HeadlessAudio::GstreamerGstPlay => self.gstreamer_gst_play.clone(),
            HeadlessAudio::Sox => self.sox.clone(),
            HeadlessAudio::Mpg123 => self.mpg123.clone(),
            HeadlessAudio::Ogg123 => self.ogg123.clone(),
            HeadlessAudio::AlsaAplay => self.alsa_aplay.clone(),
            HeadlessAudio::MacOsAfplay => self.macos_afplay.clone(),
            HeadlessAudio::PulseaudioPaplay => self.pulseaudio_paplay.clone(),
            HeadlessAudio::PulseaudioPacat => self.pulseaudio_pacat.clone(),
            HeadlessAudio::Pipewire => self.pipewire.clone(),
        }
    }

    /// Returns the version of the specified audio player if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The audio player is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, player: HeadlessAudio) -> Result<String, ProgramError> {
        if !self.is_installed(player) {
            return Err(ProgramError::NotFound(player.binary_name().to_string()));
        }
        player.version()
    }

    /// Returns the official website URL for the specified audio player.
    pub fn website(&self, player: HeadlessAudio) -> &'static str {
        player.website()
    }

    /// Returns a one-line description of the specified audio player.
    pub fn description(&self, player: HeadlessAudio) -> &'static str {
        player.description()
    }

    /// Checks if the specified audio player is installed.
    pub fn is_installed(&self, player: HeadlessAudio) -> bool {
        match player {
            HeadlessAudio::Mpv => self.mpv.is_some(),
            HeadlessAudio::Ffplay => self.ffplay.is_some(),
            HeadlessAudio::Vlc => self.vlc.is_some(),
            HeadlessAudio::MPlayer => self.mplayer.is_some(),
            HeadlessAudio::GstreamerGstPlay => self.gstreamer_gst_play.is_some(),
            HeadlessAudio::Sox => self.sox.is_some(),
            HeadlessAudio::Mpg123 => self.mpg123.is_some(),
            HeadlessAudio::Ogg123 => self.ogg123.is_some(),
            HeadlessAudio::AlsaAplay => self.alsa_aplay.is_some(),
            HeadlessAudio::MacOsAfplay => self.macos_afplay.is_some(),
            HeadlessAudio::PulseaudioPaplay => self.pulseaudio_paplay.is_some(),
            HeadlessAudio::PulseaudioPacat => self.pulseaudio_pacat.is_some(),
            HeadlessAudio::Pipewire => self.pipewire.is_some(),
        }
    }

    /// Returns a list of all installed headless audio players.
    pub fn installed(&self) -> Vec<HeadlessAudio> {
        use strum::IntoEnumIterator;
        HeadlessAudio::iter()
            .filter(|p| self.is_installed(*p))
            .collect()
    }
}

impl Serialize for InstalledHeadlessAudio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use strum::IntoEnumIterator;

        let entry = |player: HeadlessAudio| -> ProgramEntry {
            let info = player.info();
            match self.path_with_source(player) {
                Some((path, source)) => ProgramEntry::installed(info, path, source),
                None => ProgramEntry::not_installed(info),
            }
        };

        let mut state = serializer.serialize_struct("InstalledHeadlessAudio", 13)?;
        for player in HeadlessAudio::iter() {
            let field_name = match player {
                HeadlessAudio::Mpv => "mpv",
                HeadlessAudio::Ffplay => "ffplay",
                HeadlessAudio::Vlc => "vlc",
                HeadlessAudio::MPlayer => "mplayer",
                HeadlessAudio::GstreamerGstPlay => "gstreamer_gst_play",
                HeadlessAudio::Sox => "sox",
                HeadlessAudio::Mpg123 => "mpg123",
                HeadlessAudio::Ogg123 => "ogg123",
                HeadlessAudio::AlsaAplay => "alsa_aplay",
                HeadlessAudio::MacOsAfplay => "macos_afplay",
                HeadlessAudio::PulseaudioPaplay => "pulseaudio_paplay",
                HeadlessAudio::PulseaudioPacat => "pulseaudio_pacat",
                HeadlessAudio::Pipewire => "pipewire",
            };
            state.serialize_field(field_name, &entry(player))?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledHeadlessAudio {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoolHeadlessAudio {
            #[serde(default)]
            mpv: bool,
            #[serde(default)]
            ffplay: bool,
            #[serde(default)]
            vlc: bool,
            #[serde(default)]
            mplayer: bool,
            #[serde(default)]
            gstreamer_gst_play: bool,
            #[serde(default)]
            sox: bool,
            #[serde(default)]
            mpg123: bool,
            #[serde(default)]
            ogg123: bool,
            #[serde(default)]
            alsa_aplay: bool,
            #[serde(default)]
            macos_afplay: bool,
            #[serde(default)]
            pulseaudio_paplay: bool,
            #[serde(default)]
            pulseaudio_pacat: bool,
            #[serde(default)]
            pipewire: bool,
        }

        let b = BoolHeadlessAudio::deserialize(deserializer)?;

        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledHeadlessAudio {
            mpv: to_opt(b.mpv),
            ffplay: to_opt(b.ffplay),
            vlc: to_opt(b.vlc),
            mplayer: to_opt(b.mplayer),
            gstreamer_gst_play: to_opt(b.gstreamer_gst_play),
            sox: to_opt(b.sox),
            mpg123: to_opt(b.mpg123),
            ogg123: to_opt(b.ogg123),
            alsa_aplay: to_opt(b.alsa_aplay),
            macos_afplay: to_opt(b.macos_afplay),
            pulseaudio_paplay: to_opt(b.pulseaudio_paplay),
            pulseaudio_pacat: to_opt(b.pulseaudio_pacat),
            pipewire: to_opt(b.pipewire),
        })
    }
}

impl ProgramDetector for InstalledHeadlessAudio {
    type Program = HeadlessAudio;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledHeadlessAudio::path(self, program)
    }

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledHeadlessAudio::path_with_source(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledHeadlessAudio::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledHeadlessAudio::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledHeadlessAudio::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledHeadlessAudio::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = headless_audio_details(program) else {
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
        let details = headless_audio_details(program).ok_or_else(|| {
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
        let details = headless_audio_details(program).ok_or_else(|| {
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
        let players = InstalledHeadlessAudio::default();
        assert!(players.path_with_source(HeadlessAudio::Ffplay).is_none());
    }

    #[test]
    fn test_is_installed_returns_false_for_default() {
        let players = InstalledHeadlessAudio::default();
        assert!(!players.is_installed(HeadlessAudio::Ffplay));
        assert!(!players.is_installed(HeadlessAudio::Mpv));
    }

    #[test]
    fn test_serialize_produces_program_entries() {
        let players = InstalledHeadlessAudio::default();
        let json = serde_json::to_string(&players).unwrap();
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"mpv\":{"));
        assert!(json.contains("\"name\":\"mpv\""));
    }

    #[test]
    fn test_deserialize_from_boolean_fields() {
        let json = r#"{"ffplay": true, "mpv": false}"#;
        let players: InstalledHeadlessAudio = serde_json::from_str(json).unwrap();
        assert!(players.is_installed(HeadlessAudio::Ffplay));
        assert!(!players.is_installed(HeadlessAudio::Mpv));
    }

    #[test]
    fn test_serialize_to_json() {
        let original = InstalledHeadlessAudio::default();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("mpv").is_some());
    }

    #[test]
    fn test_path_returns_none_when_not_installed() {
        let players = InstalledHeadlessAudio::default();
        assert!(players.path(HeadlessAudio::Ffplay).is_none());
        assert!(players.path(HeadlessAudio::Mpv).is_none());
    }

    #[test]
    fn test_installed_returns_empty_for_default() {
        let players = InstalledHeadlessAudio::default();
        assert!(players.installed().is_empty());
    }

    #[test]
    fn test_version_returns_not_found_for_uninstalled() {
        let players = InstalledHeadlessAudio::default();
        let result = players.version(HeadlessAudio::Ffplay);
        assert!(result.is_err());
        if let Err(ProgramError::NotFound(name)) = result {
            assert_eq!(name, "ffplay");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_website_returns_static_str() {
        let players = InstalledHeadlessAudio::default();
        let website = players.website(HeadlessAudio::Ffplay);
        assert!(!website.is_empty());
        assert!(website.starts_with("http"));
    }

    #[test]
    fn test_description_returns_static_str() {
        let players = InstalledHeadlessAudio::default();
        let desc = players.description(HeadlessAudio::Ffplay);
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_deserialize_partial_json() {
        let json = r#"{"ffplay": true}"#;
        let players: InstalledHeadlessAudio = serde_json::from_str(json).unwrap();
        assert!(players.is_installed(HeadlessAudio::Ffplay));
        assert!(!players.is_installed(HeadlessAudio::Mpv));
    }

    #[test]
    fn test_clone_produces_equal_struct() {
        let players = InstalledHeadlessAudio::default();
        let cloned = players.clone();
        assert_eq!(players, cloned);
    }

    #[test]
    fn test_path_with_source_all_players_default() {
        let players = InstalledHeadlessAudio::default();
        use strum::IntoEnumIterator;
        for player in HeadlessAudio::iter() {
            assert!(
                players.path_with_source(player).is_none(),
                "{:?} should return None for default",
                player
            );
        }
    }

    #[test]
    fn test_is_installed_all_players_default() {
        let players = InstalledHeadlessAudio::default();
        use strum::IntoEnumIterator;
        for player in HeadlessAudio::iter() {
            assert!(
                !players.is_installed(player),
                "{:?} should not be installed for default",
                player
            );
        }
    }

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;

        #[test]
        fn test_afplay_detected_on_macos() {
            // afplay is a built-in macOS command and should always be available
            let players = InstalledHeadlessAudio::new();
            assert!(
                players.is_installed(HeadlessAudio::MacOsAfplay),
                "afplay should be installed on macOS"
            );
        }

        #[test]
        fn test_path_with_source_detects_source_correctly() {
            let players = InstalledHeadlessAudio::new();
            for player in players.installed() {
                let result = players.path_with_source(player);
                assert!(result.is_some(), "{:?} should have path info", player);
                let (path, source) = result.unwrap();
                assert!(!path.as_os_str().is_empty(), "Path should not be empty");
                // Headless audio players are typically PATH-based, not app bundles
                assert!(
                    source == ExecutableSource::Path
                        || source == ExecutableSource::MacOsAppBundle,
                    "Source should be valid"
                );
            }
        }
    }
}
