use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::HeadlessAudio;
use crate::programs::find_program::find_programs_parallel;
use crate::programs::installer::{
    execute_install, execute_versioned_install, method_available, select_best_method,
    InstallOptions,
};
use crate::programs::schema::{ProgramError, ProgramMetadata};
use crate::programs::types::ProgramDetector;
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
        HeadlessAudio::PulseaudioPaplay => Program::PulseaudioPaplay,
        HeadlessAudio::Pipewire => Program::Pipewire,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Headless audio players found on the system.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct InstalledHeadlessAudio {
    /// mpv media player. [Website](https://mpv.io/)
    pub mpv: bool,
    /// FFplay from FFmpeg. [Website](https://www.ffmpeg.org/ffplay.html)
    pub ffplay: bool,
    /// VLC media player (cvlc). [Website](https://wiki.videolan.org/VLC_command-line_help/)
    pub vlc: bool,
    /// MPlayer media player. [Website](https://www.mplayerhq.hu/)
    pub mplayer: bool,
    /// GStreamer gst-play tool. [Website](https://gstreamer.freedesktop.org/)
    pub gstreamer_gst_play: bool,
    /// SoX play command. [Website](https://linux.die.net/man/1/sox)
    pub sox: bool,
    /// mpg123 MP3 player. [Website](https://www.mpg123.de/)
    pub mpg123: bool,
    /// ogg123 Vorbis player. [Website](https://github.com/xiph/vorbis-tools)
    pub ogg123: bool,
    /// ALSA aplay utility. [Website](https://linux.die.net/man/1/aplay)
    pub alsa_aplay: bool,
    /// PulseAudio paplay utility. [Website](https://manpages.ubuntu.com/)
    pub pulseaudio_paplay: bool,
    /// PipeWire pw-play/pw-cat. [Website](https://docs.pipewire.org/)
    pub pipewire: bool,
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
            "paplay",
            "pacat",
            "pw-cat",
            "pw-play",
        ];

        let results = find_programs_parallel(&programs);

        let has = |name: &str| results.get(name).and_then(|r| r.as_ref()).is_some();
        let any = |names: &[&str]| names.iter().any(|&name| has(name));

        Self {
            mpv: has("mpv"),
            ffplay: has("ffplay"),
            vlc: any(&["vlc", "cvlc"]),
            mplayer: has("mplayer"),
            gstreamer_gst_play: any(&["gst-play-1.0", "gst-play"]),
            sox: any(&["play", "sox"]),
            mpg123: has("mpg123"),
            ogg123: has("ogg123"),
            alsa_aplay: has("aplay"),
            pulseaudio_paplay: any(&["paplay", "pacat"]),
            pipewire: any(&["pw-cat", "pw-play"]),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified audio player's binary if installed.
    pub fn path(&self, player: HeadlessAudio) -> Option<PathBuf> {
        if self.is_installed(player) {
            player.path()
        } else {
            None
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
            HeadlessAudio::Mpv => self.mpv,
            HeadlessAudio::Ffplay => self.ffplay,
            HeadlessAudio::Vlc => self.vlc,
            HeadlessAudio::MPlayer => self.mplayer,
            HeadlessAudio::GstreamerGstPlay => self.gstreamer_gst_play,
            HeadlessAudio::Sox => self.sox,
            HeadlessAudio::Mpg123 => self.mpg123,
            HeadlessAudio::Ogg123 => self.ogg123,
            HeadlessAudio::AlsaAplay => self.alsa_aplay,
            HeadlessAudio::PulseaudioPaplay => self.pulseaudio_paplay,
            HeadlessAudio::Pipewire => self.pipewire,
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
