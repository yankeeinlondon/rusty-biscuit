use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::Utility;
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

fn utility_details(utility: Utility) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match utility {
        Utility::Exa => Program::Exa,
        Utility::Eza => Program::Eza,
        Utility::Ripgrep => Program::Ripgrep,
        Utility::Dust => Program::Dust,
        Utility::Bat => Program::Bat,
        Utility::Fd => Program::Fd,
        Utility::Procs => Program::Procs,
        Utility::Bottom => Program::Bottom,
        Utility::Fzf => Program::Fzf,
        Utility::Zoxide => Program::Zoxide,
        Utility::Starship => Program::Starship,
        Utility::Direnv => Program::Direnv,
        Utility::Jq => Program::Jq,
        Utility::Delta => Program::Delta,
        Utility::Tealdeer => Program::Tealdeer,
        Utility::Lazygit => Program::Lazygit,
        Utility::Gh => Program::Gh,
        Utility::Htop => Program::Htop,
        Utility::Btop => Program::Btop,
        Utility::Tmux => Program::Tmux,
        Utility::Zellij => Program::Zellij,
        Utility::Httpie => Program::Httpie,
        Utility::Curlie => Program::Curlie,
        Utility::Mise => Program::Mise,
        Utility::Hyperfine => Program::Hyperfine,
        Utility::Tokei => Program::Tokei,
        Utility::Xh => Program::Xh,
        Utility::Curl => Program::Curl,
        Utility::Wget => Program::Wget,
        Utility::Iperf3 => Program::Iperf3,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Popular modern utility programs found on macOS, Linux, or Windows.
///
/// Stores path and discovery source for each installed utility.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledUtilities {
    exa: Option<(PathBuf, ExecutableSource)>,
    eza: Option<(PathBuf, ExecutableSource)>,
    ripgrep: Option<(PathBuf, ExecutableSource)>,
    dust: Option<(PathBuf, ExecutableSource)>,
    bat: Option<(PathBuf, ExecutableSource)>,
    fd: Option<(PathBuf, ExecutableSource)>,
    procs: Option<(PathBuf, ExecutableSource)>,
    bottom: Option<(PathBuf, ExecutableSource)>,
    fzf: Option<(PathBuf, ExecutableSource)>,
    zoxide: Option<(PathBuf, ExecutableSource)>,
    starship: Option<(PathBuf, ExecutableSource)>,
    direnv: Option<(PathBuf, ExecutableSource)>,
    jq: Option<(PathBuf, ExecutableSource)>,
    delta: Option<(PathBuf, ExecutableSource)>,
    tealdeer: Option<(PathBuf, ExecutableSource)>,
    lazygit: Option<(PathBuf, ExecutableSource)>,
    gh: Option<(PathBuf, ExecutableSource)>,
    htop: Option<(PathBuf, ExecutableSource)>,
    btop: Option<(PathBuf, ExecutableSource)>,
    tmux: Option<(PathBuf, ExecutableSource)>,
    zellij: Option<(PathBuf, ExecutableSource)>,
    httpie: Option<(PathBuf, ExecutableSource)>,
    curlie: Option<(PathBuf, ExecutableSource)>,
    mise: Option<(PathBuf, ExecutableSource)>,
    hyperfine: Option<(PathBuf, ExecutableSource)>,
    tokei: Option<(PathBuf, ExecutableSource)>,
    xh: Option<(PathBuf, ExecutableSource)>,
    curl: Option<(PathBuf, ExecutableSource)>,
    wget: Option<(PathBuf, ExecutableSource)>,
    iperf3: Option<(PathBuf, ExecutableSource)>,
}

impl InstalledUtilities {
    /// Detect which popular utilities are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "exa",
            "eza",
            "rg",
            "ripgrep",
            "dust",
            "bat",
            "batcat",
            "fd",
            "fdfind",
            "procs",
            "btm",
            "bottom",
            "fzf",
            "zoxide",
            "starship",
            "direnv",
            "jq",
            "delta",
            "tldr",
            "tealdeer",
            "lazygit",
            "gh",
            "htop",
            "btop",
            "tmux",
            "zellij",
            "http",
            "https",
            "httpie",
            "curlie",
            "mise",
            "hyperfine",
            "tokei",
            "xh",
            "xhs",
            "curl",
            "wget",
            "iperf3",
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
            exa: get("exa"),
            eza: get("eza"),
            ripgrep: get_any(&["rg", "ripgrep"]),
            dust: get("dust"),
            bat: get_any(&["bat", "batcat"]),
            fd: get_any(&["fd", "fdfind"]),
            procs: get("procs"),
            bottom: get_any(&["btm", "bottom"]),
            fzf: get("fzf"),
            zoxide: get("zoxide"),
            starship: get("starship"),
            direnv: get("direnv"),
            jq: get("jq"),
            delta: get("delta"),
            tealdeer: get_any(&["tldr", "tealdeer"]),
            lazygit: get("lazygit"),
            gh: get("gh"),
            htop: get("htop"),
            btop: get("btop"),
            tmux: get("tmux"),
            zellij: get("zellij"),
            httpie: get_any(&["http", "https", "httpie"]),
            curlie: get("curlie"),
            mise: get("mise"),
            hyperfine: get("hyperfine"),
            tokei: get("tokei"),
            xh: get_any(&["xh", "xhs"]),
            curl: get("curl"),
            wget: get("wget"),
            iperf3: get("iperf3"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified utility's binary if installed.
    pub fn path(&self, utility: Utility) -> Option<PathBuf> {
        self.path_with_source(utility).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified utility if installed.
    pub fn path_with_source(&self, utility: Utility) -> Option<(PathBuf, ExecutableSource)> {
        match utility {
            Utility::Exa => self.exa.clone(),
            Utility::Eza => self.eza.clone(),
            Utility::Ripgrep => self.ripgrep.clone(),
            Utility::Dust => self.dust.clone(),
            Utility::Bat => self.bat.clone(),
            Utility::Fd => self.fd.clone(),
            Utility::Procs => self.procs.clone(),
            Utility::Bottom => self.bottom.clone(),
            Utility::Fzf => self.fzf.clone(),
            Utility::Zoxide => self.zoxide.clone(),
            Utility::Starship => self.starship.clone(),
            Utility::Direnv => self.direnv.clone(),
            Utility::Jq => self.jq.clone(),
            Utility::Delta => self.delta.clone(),
            Utility::Tealdeer => self.tealdeer.clone(),
            Utility::Lazygit => self.lazygit.clone(),
            Utility::Gh => self.gh.clone(),
            Utility::Htop => self.htop.clone(),
            Utility::Btop => self.btop.clone(),
            Utility::Tmux => self.tmux.clone(),
            Utility::Zellij => self.zellij.clone(),
            Utility::Httpie => self.httpie.clone(),
            Utility::Curlie => self.curlie.clone(),
            Utility::Mise => self.mise.clone(),
            Utility::Hyperfine => self.hyperfine.clone(),
            Utility::Tokei => self.tokei.clone(),
            Utility::Xh => self.xh.clone(),
            Utility::Curl => self.curl.clone(),
            Utility::Wget => self.wget.clone(),
            Utility::Iperf3 => self.iperf3.clone(),
        }
    }

    /// Returns the version of the specified utility if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The utility is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, utility: Utility) -> Result<String, ProgramError> {
        if !self.is_installed(utility) {
            return Err(ProgramError::NotFound(utility.binary_name().to_string()));
        }
        utility.version()
    }

    /// Returns the official website URL for the specified utility.
    pub fn website(&self, utility: Utility) -> &'static str {
        utility.website()
    }

    /// Returns a one-line description of the specified utility.
    pub fn description(&self, utility: Utility) -> &'static str {
        utility.description()
    }

    /// Checks if the specified utility is installed.
    pub fn is_installed(&self, utility: Utility) -> bool {
        match utility {
            Utility::Exa => self.exa.is_some(),
            Utility::Eza => self.eza.is_some(),
            Utility::Ripgrep => self.ripgrep.is_some(),
            Utility::Dust => self.dust.is_some(),
            Utility::Bat => self.bat.is_some(),
            Utility::Fd => self.fd.is_some(),
            Utility::Procs => self.procs.is_some(),
            Utility::Bottom => self.bottom.is_some(),
            Utility::Fzf => self.fzf.is_some(),
            Utility::Zoxide => self.zoxide.is_some(),
            Utility::Starship => self.starship.is_some(),
            Utility::Direnv => self.direnv.is_some(),
            Utility::Jq => self.jq.is_some(),
            Utility::Delta => self.delta.is_some(),
            Utility::Tealdeer => self.tealdeer.is_some(),
            Utility::Lazygit => self.lazygit.is_some(),
            Utility::Gh => self.gh.is_some(),
            Utility::Htop => self.htop.is_some(),
            Utility::Btop => self.btop.is_some(),
            Utility::Tmux => self.tmux.is_some(),
            Utility::Zellij => self.zellij.is_some(),
            Utility::Httpie => self.httpie.is_some(),
            Utility::Curlie => self.curlie.is_some(),
            Utility::Mise => self.mise.is_some(),
            Utility::Hyperfine => self.hyperfine.is_some(),
            Utility::Tokei => self.tokei.is_some(),
            Utility::Xh => self.xh.is_some(),
            Utility::Curl => self.curl.is_some(),
            Utility::Wget => self.wget.is_some(),
            Utility::Iperf3 => self.iperf3.is_some(),
        }
    }

    /// Returns a list of all installed utilities.
    pub fn installed(&self) -> Vec<Utility> {
        use strum::IntoEnumIterator;
        Utility::iter().filter(|u| self.is_installed(*u)).collect()
    }
}

impl Serialize for InstalledUtilities {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstalledUtilities", 30)?;
        state.serialize_field("exa", &self.exa.is_some())?;
        state.serialize_field("eza", &self.eza.is_some())?;
        state.serialize_field("ripgrep", &self.ripgrep.is_some())?;
        state.serialize_field("dust", &self.dust.is_some())?;
        state.serialize_field("bat", &self.bat.is_some())?;
        state.serialize_field("fd", &self.fd.is_some())?;
        state.serialize_field("procs", &self.procs.is_some())?;
        state.serialize_field("bottom", &self.bottom.is_some())?;
        state.serialize_field("fzf", &self.fzf.is_some())?;
        state.serialize_field("zoxide", &self.zoxide.is_some())?;
        state.serialize_field("starship", &self.starship.is_some())?;
        state.serialize_field("direnv", &self.direnv.is_some())?;
        state.serialize_field("jq", &self.jq.is_some())?;
        state.serialize_field("delta", &self.delta.is_some())?;
        state.serialize_field("tealdeer", &self.tealdeer.is_some())?;
        state.serialize_field("lazygit", &self.lazygit.is_some())?;
        state.serialize_field("gh", &self.gh.is_some())?;
        state.serialize_field("htop", &self.htop.is_some())?;
        state.serialize_field("btop", &self.btop.is_some())?;
        state.serialize_field("tmux", &self.tmux.is_some())?;
        state.serialize_field("zellij", &self.zellij.is_some())?;
        state.serialize_field("httpie", &self.httpie.is_some())?;
        state.serialize_field("curlie", &self.curlie.is_some())?;
        state.serialize_field("mise", &self.mise.is_some())?;
        state.serialize_field("hyperfine", &self.hyperfine.is_some())?;
        state.serialize_field("tokei", &self.tokei.is_some())?;
        state.serialize_field("xh", &self.xh.is_some())?;
        state.serialize_field("curl", &self.curl.is_some())?;
        state.serialize_field("wget", &self.wget.is_some())?;
        state.serialize_field("iperf3", &self.iperf3.is_some())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledUtilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoolUtilities {
            #[serde(default)]
            exa: bool,
            #[serde(default)]
            eza: bool,
            #[serde(default)]
            ripgrep: bool,
            #[serde(default)]
            dust: bool,
            #[serde(default)]
            bat: bool,
            #[serde(default)]
            fd: bool,
            #[serde(default)]
            procs: bool,
            #[serde(default)]
            bottom: bool,
            #[serde(default)]
            fzf: bool,
            #[serde(default)]
            zoxide: bool,
            #[serde(default)]
            starship: bool,
            #[serde(default)]
            direnv: bool,
            #[serde(default)]
            jq: bool,
            #[serde(default)]
            delta: bool,
            #[serde(default)]
            tealdeer: bool,
            #[serde(default)]
            lazygit: bool,
            #[serde(default)]
            gh: bool,
            #[serde(default)]
            htop: bool,
            #[serde(default)]
            btop: bool,
            #[serde(default)]
            tmux: bool,
            #[serde(default)]
            zellij: bool,
            #[serde(default)]
            httpie: bool,
            #[serde(default)]
            curlie: bool,
            #[serde(default)]
            mise: bool,
            #[serde(default)]
            hyperfine: bool,
            #[serde(default)]
            tokei: bool,
            #[serde(default)]
            xh: bool,
            #[serde(default)]
            curl: bool,
            #[serde(default)]
            wget: bool,
            #[serde(default)]
            iperf3: bool,
        }

        let b = BoolUtilities::deserialize(deserializer)?;

        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledUtilities {
            exa: to_opt(b.exa),
            eza: to_opt(b.eza),
            ripgrep: to_opt(b.ripgrep),
            dust: to_opt(b.dust),
            bat: to_opt(b.bat),
            fd: to_opt(b.fd),
            procs: to_opt(b.procs),
            bottom: to_opt(b.bottom),
            fzf: to_opt(b.fzf),
            zoxide: to_opt(b.zoxide),
            starship: to_opt(b.starship),
            direnv: to_opt(b.direnv),
            jq: to_opt(b.jq),
            delta: to_opt(b.delta),
            tealdeer: to_opt(b.tealdeer),
            lazygit: to_opt(b.lazygit),
            gh: to_opt(b.gh),
            htop: to_opt(b.htop),
            btop: to_opt(b.btop),
            tmux: to_opt(b.tmux),
            zellij: to_opt(b.zellij),
            httpie: to_opt(b.httpie),
            curlie: to_opt(b.curlie),
            mise: to_opt(b.mise),
            hyperfine: to_opt(b.hyperfine),
            tokei: to_opt(b.tokei),
            xh: to_opt(b.xh),
            curl: to_opt(b.curl),
            wget: to_opt(b.wget),
            iperf3: to_opt(b.iperf3),
        })
    }
}

impl ProgramDetector for InstalledUtilities {
    type Program = Utility;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledUtilities::path(self, program)
    }

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledUtilities::path_with_source(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledUtilities::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledUtilities::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledUtilities::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledUtilities::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = utility_details(program) else {
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
        let details =
            utility_details(program).ok_or_else(|| SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
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
        let details =
            utility_details(program).ok_or_else(|| SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
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
        let utils = InstalledUtilities::default();
        assert!(utils.path_with_source(Utility::Ripgrep).is_none());
    }

    #[test]
    fn test_is_installed_returns_false_for_default() {
        let utils = InstalledUtilities::default();
        assert!(!utils.is_installed(Utility::Ripgrep));
        assert!(!utils.is_installed(Utility::Bat));
    }

    #[test]
    fn test_serialize_produces_boolean_fields() {
        let utils = InstalledUtilities::default();
        let json = serde_json::to_string(&utils).unwrap();
        assert!(json.contains("\"ripgrep\":false"));
        assert!(json.contains("\"bat\":false"));
    }

    #[test]
    fn test_deserialize_from_boolean_fields() {
        let json = r#"{"ripgrep": true, "bat": false}"#;
        let utils: InstalledUtilities = serde_json::from_str(json).unwrap();
        assert!(utils.is_installed(Utility::Ripgrep));
        assert!(!utils.is_installed(Utility::Bat));
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = InstalledUtilities::default();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: InstalledUtilities = serde_json::from_str(&json).unwrap();
        for util in original.installed() {
            assert!(deserialized.is_installed(util));
        }
    }
}
