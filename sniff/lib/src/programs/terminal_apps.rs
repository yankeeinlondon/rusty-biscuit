use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::TerminalApp;
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

fn terminal_app_details(app: TerminalApp) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match app {
        TerminalApp::Alacritty => Program::Alacritty,
        TerminalApp::Kitty => Program::Kitty,
        TerminalApp::ITerm2 => Program::ITerm2,
        TerminalApp::WezTerm => Program::WezTerm,
        TerminalApp::Ghostty => Program::Ghostty,
        TerminalApp::Warp => Program::Warp,
        TerminalApp::Rio => Program::Rio,
        TerminalApp::Tabby => Program::Tabby,
        TerminalApp::Foot => Program::Foot,
        TerminalApp::GnomeTerminal => Program::GnomeTerminal,
        TerminalApp::Konsole => Program::Konsole,
        TerminalApp::XfceTerminal => Program::XfceTerminal,
        TerminalApp::Terminology => Program::Terminology,
        TerminalApp::St => Program::St,
        TerminalApp::Xterm => Program::Xterm,
        TerminalApp::Hyper => Program::Hyper,
        TerminalApp::WindowsTerminal => Program::WindowsTerminal,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Popular terminal applications found on macOS, Linux, or Windows.
///
/// Stores path and discovery source for each installed terminal app.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledTerminalApps {
    alacritty: Option<(PathBuf, ExecutableSource)>,
    kitty: Option<(PathBuf, ExecutableSource)>,
    iterm2: Option<(PathBuf, ExecutableSource)>,
    wezterm: Option<(PathBuf, ExecutableSource)>,
    ghostty: Option<(PathBuf, ExecutableSource)>,
    warp: Option<(PathBuf, ExecutableSource)>,
    rio: Option<(PathBuf, ExecutableSource)>,
    tabby: Option<(PathBuf, ExecutableSource)>,
    foot: Option<(PathBuf, ExecutableSource)>,
    gnome_terminal: Option<(PathBuf, ExecutableSource)>,
    konsole: Option<(PathBuf, ExecutableSource)>,
    xfce_terminal: Option<(PathBuf, ExecutableSource)>,
    terminology: Option<(PathBuf, ExecutableSource)>,
    st: Option<(PathBuf, ExecutableSource)>,
    xterm: Option<(PathBuf, ExecutableSource)>,
    hyper: Option<(PathBuf, ExecutableSource)>,
    windows_terminal: Option<(PathBuf, ExecutableSource)>,
}

impl InstalledTerminalApps {
    /// Detect which popular terminal apps are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "alacritty",
            "kitty",
            "iterm2",
            "wezterm",
            "ghostty",
            "warp-terminal",
            "warp",
            "rio",
            "tabby",
            "foot",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "terminology",
            "st",
            "xterm",
            "hyper",
            "wt",
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
            alacritty: get("alacritty"),
            kitty: get("kitty"),
            iterm2: get("iterm2"),
            wezterm: get("wezterm"),
            ghostty: get("ghostty"),
            warp: get_any(&["warp-terminal", "warp"]),
            rio: get("rio"),
            tabby: get("tabby"),
            foot: get("foot"),
            gnome_terminal: get("gnome-terminal"),
            konsole: get("konsole"),
            xfce_terminal: get("xfce4-terminal"),
            terminology: get("terminology"),
            st: get("st"),
            xterm: get("xterm"),
            hyper: get("hyper"),
            windows_terminal: get("wt"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified terminal app's binary if installed.
    pub fn path(&self, app: TerminalApp) -> Option<PathBuf> {
        self.path_with_source(app).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified terminal app if installed.
    pub fn path_with_source(&self, app: TerminalApp) -> Option<(PathBuf, ExecutableSource)> {
        match app {
            TerminalApp::Alacritty => self.alacritty.clone(),
            TerminalApp::Kitty => self.kitty.clone(),
            TerminalApp::ITerm2 => self.iterm2.clone(),
            TerminalApp::WezTerm => self.wezterm.clone(),
            TerminalApp::Ghostty => self.ghostty.clone(),
            TerminalApp::Warp => self.warp.clone(),
            TerminalApp::Rio => self.rio.clone(),
            TerminalApp::Tabby => self.tabby.clone(),
            TerminalApp::Foot => self.foot.clone(),
            TerminalApp::GnomeTerminal => self.gnome_terminal.clone(),
            TerminalApp::Konsole => self.konsole.clone(),
            TerminalApp::XfceTerminal => self.xfce_terminal.clone(),
            TerminalApp::Terminology => self.terminology.clone(),
            TerminalApp::St => self.st.clone(),
            TerminalApp::Xterm => self.xterm.clone(),
            TerminalApp::Hyper => self.hyper.clone(),
            TerminalApp::WindowsTerminal => self.windows_terminal.clone(),
        }
    }

    /// Returns the version of the specified terminal app if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The terminal app is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, app: TerminalApp) -> Result<String, ProgramError> {
        if !self.is_installed(app) {
            return Err(ProgramError::NotFound(app.binary_name().to_string()));
        }
        app.version()
    }

    /// Returns the official website URL for the specified terminal app.
    pub fn website(&self, app: TerminalApp) -> &'static str {
        app.website()
    }

    /// Returns a one-line description of the specified terminal app.
    pub fn description(&self, app: TerminalApp) -> &'static str {
        app.description()
    }

    /// Checks if the specified terminal app is installed.
    pub fn is_installed(&self, app: TerminalApp) -> bool {
        match app {
            TerminalApp::Alacritty => self.alacritty.is_some(),
            TerminalApp::Kitty => self.kitty.is_some(),
            TerminalApp::ITerm2 => self.iterm2.is_some(),
            TerminalApp::WezTerm => self.wezterm.is_some(),
            TerminalApp::Ghostty => self.ghostty.is_some(),
            TerminalApp::Warp => self.warp.is_some(),
            TerminalApp::Rio => self.rio.is_some(),
            TerminalApp::Tabby => self.tabby.is_some(),
            TerminalApp::Foot => self.foot.is_some(),
            TerminalApp::GnomeTerminal => self.gnome_terminal.is_some(),
            TerminalApp::Konsole => self.konsole.is_some(),
            TerminalApp::XfceTerminal => self.xfce_terminal.is_some(),
            TerminalApp::Terminology => self.terminology.is_some(),
            TerminalApp::St => self.st.is_some(),
            TerminalApp::Xterm => self.xterm.is_some(),
            TerminalApp::Hyper => self.hyper.is_some(),
            TerminalApp::WindowsTerminal => self.windows_terminal.is_some(),
        }
    }

    /// Returns a list of all installed terminal apps.
    pub fn installed(&self) -> Vec<TerminalApp> {
        use strum::IntoEnumIterator;
        TerminalApp::iter()
            .filter(|a| self.is_installed(*a))
            .collect()
    }
}

impl Serialize for InstalledTerminalApps {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use strum::IntoEnumIterator;

        // Helper to create a ProgramEntry for a terminal app
        let entry = |app: TerminalApp| -> ProgramEntry {
            let info = app.info();
            match self.path_with_source(app) {
                Some((path, source)) => ProgramEntry::installed(info, path, source),
                None => ProgramEntry::not_installed(info),
            }
        };

        let mut state = serializer.serialize_struct("InstalledTerminalApps", 17)?;
        for app in TerminalApp::iter() {
            let field_name = match app {
                TerminalApp::Alacritty => "alacritty",
                TerminalApp::Kitty => "kitty",
                TerminalApp::ITerm2 => "iterm2",
                TerminalApp::WezTerm => "wezterm",
                TerminalApp::Ghostty => "ghostty",
                TerminalApp::Warp => "warp",
                TerminalApp::Rio => "rio",
                TerminalApp::Tabby => "tabby",
                TerminalApp::Foot => "foot",
                TerminalApp::GnomeTerminal => "gnome_terminal",
                TerminalApp::Konsole => "konsole",
                TerminalApp::XfceTerminal => "xfce_terminal",
                TerminalApp::Terminology => "terminology",
                TerminalApp::St => "st",
                TerminalApp::Xterm => "xterm",
                TerminalApp::Hyper => "hyper",
                TerminalApp::WindowsTerminal => "windows_terminal",
            };
            state.serialize_field(field_name, &entry(app))?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledTerminalApps {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoolTerminalApps {
            #[serde(default)]
            alacritty: bool,
            #[serde(default)]
            kitty: bool,
            #[serde(default)]
            iterm2: bool,
            #[serde(default)]
            wezterm: bool,
            #[serde(default)]
            ghostty: bool,
            #[serde(default)]
            warp: bool,
            #[serde(default)]
            rio: bool,
            #[serde(default)]
            tabby: bool,
            #[serde(default)]
            foot: bool,
            #[serde(default)]
            gnome_terminal: bool,
            #[serde(default)]
            konsole: bool,
            #[serde(default)]
            xfce_terminal: bool,
            #[serde(default)]
            terminology: bool,
            #[serde(default)]
            st: bool,
            #[serde(default)]
            xterm: bool,
            #[serde(default)]
            hyper: bool,
            #[serde(default)]
            windows_terminal: bool,
        }

        let b = BoolTerminalApps::deserialize(deserializer)?;

        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledTerminalApps {
            alacritty: to_opt(b.alacritty),
            kitty: to_opt(b.kitty),
            iterm2: to_opt(b.iterm2),
            wezterm: to_opt(b.wezterm),
            ghostty: to_opt(b.ghostty),
            warp: to_opt(b.warp),
            rio: to_opt(b.rio),
            tabby: to_opt(b.tabby),
            foot: to_opt(b.foot),
            gnome_terminal: to_opt(b.gnome_terminal),
            konsole: to_opt(b.konsole),
            xfce_terminal: to_opt(b.xfce_terminal),
            terminology: to_opt(b.terminology),
            st: to_opt(b.st),
            xterm: to_opt(b.xterm),
            hyper: to_opt(b.hyper),
            windows_terminal: to_opt(b.windows_terminal),
        })
    }
}

impl ProgramDetector for InstalledTerminalApps {
    type Program = TerminalApp;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledTerminalApps::path(self, program)
    }

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledTerminalApps::path_with_source(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledTerminalApps::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledTerminalApps::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledTerminalApps::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledTerminalApps::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = terminal_app_details(program) else {
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
        let details = terminal_app_details(program).ok_or_else(|| {
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
        let details = terminal_app_details(program).ok_or_else(|| {
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
        let apps = InstalledTerminalApps::default();
        assert!(apps.path_with_source(TerminalApp::Alacritty).is_none());
    }

    #[test]
    fn test_is_installed_returns_false_for_default() {
        let apps = InstalledTerminalApps::default();
        assert!(!apps.is_installed(TerminalApp::Alacritty));
        assert!(!apps.is_installed(TerminalApp::WezTerm));
    }

    #[test]
    fn test_serialize_produces_boolean_fields() {
        let apps = InstalledTerminalApps::default();
        let json = serde_json::to_string(&apps).unwrap();
        assert!(json.contains("\"alacritty\":false"));
        assert!(json.contains("\"wezterm\":false"));
    }

    #[test]
    fn test_deserialize_from_boolean_fields() {
        let json = r#"{"alacritty": true, "wezterm": false}"#;
        let apps: InstalledTerminalApps = serde_json::from_str(json).unwrap();
        assert!(apps.is_installed(TerminalApp::Alacritty));
        assert!(!apps.is_installed(TerminalApp::WezTerm));
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = InstalledTerminalApps::default();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: InstalledTerminalApps = serde_json::from_str(&json).unwrap();
        for app in original.installed() {
            assert!(deserialized.is_installed(app));
        }
    }

    #[test]
    fn test_path_returns_none_when_not_installed() {
        let apps = InstalledTerminalApps::default();
        assert!(apps.path(TerminalApp::Alacritty).is_none());
        assert!(apps.path(TerminalApp::WezTerm).is_none());
    }

    #[test]
    fn test_installed_returns_empty_for_default() {
        let apps = InstalledTerminalApps::default();
        assert!(apps.installed().is_empty());
    }

    #[test]
    fn test_version_returns_not_found_for_uninstalled() {
        let apps = InstalledTerminalApps::default();
        let result = apps.version(TerminalApp::Alacritty);
        assert!(result.is_err());
        if let Err(ProgramError::NotFound(name)) = result {
            assert_eq!(name, "alacritty");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_website_returns_static_str() {
        let apps = InstalledTerminalApps::default();
        let website = apps.website(TerminalApp::Alacritty);
        assert!(!website.is_empty());
        assert!(website.starts_with("http"));
    }

    #[test]
    fn test_description_returns_static_str() {
        let apps = InstalledTerminalApps::default();
        let desc = apps.description(TerminalApp::Alacritty);
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_deserialize_partial_json() {
        let json = r#"{"alacritty": true}"#;
        let apps: InstalledTerminalApps = serde_json::from_str(json).unwrap();
        assert!(apps.is_installed(TerminalApp::Alacritty));
        assert!(!apps.is_installed(TerminalApp::WezTerm));
    }

    #[test]
    fn test_clone_produces_equal_struct() {
        let apps = InstalledTerminalApps::default();
        let cloned = apps.clone();
        assert_eq!(apps, cloned);
    }

    #[test]
    fn test_path_with_source_all_apps_default() {
        let apps = InstalledTerminalApps::default();
        use strum::IntoEnumIterator;
        for app in TerminalApp::iter() {
            assert!(
                apps.path_with_source(app).is_none(),
                "{:?} should return None for default",
                app
            );
        }
    }

    #[test]
    fn test_is_installed_all_apps_default() {
        let apps = InstalledTerminalApps::default();
        use strum::IntoEnumIterator;
        for app in TerminalApp::iter() {
            assert!(
                !apps.is_installed(app),
                "{:?} should not be installed for default",
                app
            );
        }
    }

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;

        #[test]
        fn test_path_with_source_detects_source_correctly() {
            let apps = InstalledTerminalApps::new();
            for app in apps.installed() {
                let result = apps.path_with_source(app);
                assert!(result.is_some(), "{:?} should have path info", app);
                let (path, source) = result.unwrap();
                assert!(!path.as_os_str().is_empty(), "Path should not be empty");
                assert!(
                    source == ExecutableSource::Path
                        || source == ExecutableSource::MacOsAppBundle,
                    "Source should be valid"
                );
            }
        }
    }
}
