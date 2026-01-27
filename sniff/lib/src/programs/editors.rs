use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::Editor;
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

fn editor_details(editor: Editor) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match editor {
        Editor::Vi => Program::Vi,
        Editor::Vim => Program::Vim,
        Editor::Neovim => Program::Neovim,
        Editor::Emacs => Program::Emacs,
        Editor::XEmacs => Program::XEmacs,
        Editor::Nano => Program::Nano,
        Editor::Helix => Program::Helix,
        Editor::VSCode => Program::VSCode,
        Editor::VSCodium => Program::VSCodium,
        Editor::Sublime => Program::Sublime,
        Editor::Zed => Program::Zed,
        Editor::Micro => Program::Micro,
        Editor::Kakoune => Program::Kakoune,
        Editor::Amp => Program::Amp,
        Editor::Lapce => Program::Lapce,
        Editor::PhpStorm => Program::PhpStorm,
        Editor::IntellijIdea => Program::IntellijIdea,
        Editor::PyCharm => Program::PyCharm,
        Editor::WebStorm => Program::WebStorm,
        Editor::CLion => Program::CLion,
        Editor::GoLand => Program::GoLand,
        Editor::Rider => Program::Rider,
        Editor::TextMate => Program::TextMate,
        Editor::BBEdit => Program::BBEdit,
        Editor::Geany => Program::Geany,
        Editor::Kate => Program::Kate,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Popular text editors and IDEs found on the system.
///
/// Stores path and discovery source for each installed editor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledEditors {
    vi: Option<(PathBuf, ExecutableSource)>,
    vim: Option<(PathBuf, ExecutableSource)>,
    neovim: Option<(PathBuf, ExecutableSource)>,
    emacs: Option<(PathBuf, ExecutableSource)>,
    xemacs: Option<(PathBuf, ExecutableSource)>,
    nano: Option<(PathBuf, ExecutableSource)>,
    helix: Option<(PathBuf, ExecutableSource)>,
    vscode: Option<(PathBuf, ExecutableSource)>,
    vscodium: Option<(PathBuf, ExecutableSource)>,
    sublime: Option<(PathBuf, ExecutableSource)>,
    zed: Option<(PathBuf, ExecutableSource)>,
    micro: Option<(PathBuf, ExecutableSource)>,
    kakoune: Option<(PathBuf, ExecutableSource)>,
    amp: Option<(PathBuf, ExecutableSource)>,
    lapce: Option<(PathBuf, ExecutableSource)>,
    phpstorm: Option<(PathBuf, ExecutableSource)>,
    intellij_idea: Option<(PathBuf, ExecutableSource)>,
    pycharm: Option<(PathBuf, ExecutableSource)>,
    webstorm: Option<(PathBuf, ExecutableSource)>,
    clion: Option<(PathBuf, ExecutableSource)>,
    goland: Option<(PathBuf, ExecutableSource)>,
    rider: Option<(PathBuf, ExecutableSource)>,
    textmate: Option<(PathBuf, ExecutableSource)>,
    bbedit: Option<(PathBuf, ExecutableSource)>,
    geany: Option<(PathBuf, ExecutableSource)>,
    kate: Option<(PathBuf, ExecutableSource)>,
}

impl InstalledEditors {
    /// Detect which popular editors are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "vi", "vim", "nvim", "emacs", "xemacs", "nano", "hx", "code", "codium", "subl", "zed",
            "micro", "kak", "amp", "lapce", "phpstorm", "idea", "pycharm", "webstorm", "clion",
            "goland", "rider", "mate", "bbedit", "geany", "kate",
        ];

        let results = find_programs_with_source_parallel(&programs);

        let get = |name: &str| results.get(name).and_then(|r| r.clone());

        Self {
            vi: get("vi"),
            vim: get("vim"),
            neovim: get("nvim"),
            emacs: get("emacs"),
            xemacs: get("xemacs"),
            nano: get("nano"),
            helix: get("hx"),
            vscode: get("code"),
            vscodium: get("codium"),
            sublime: get("subl"),
            zed: get("zed"),
            micro: get("micro"),
            kakoune: get("kak"),
            amp: get("amp"),
            lapce: get("lapce"),
            phpstorm: get("phpstorm"),
            intellij_idea: get("idea"),
            pycharm: get("pycharm"),
            webstorm: get("webstorm"),
            clion: get("clion"),
            goland: get("goland"),
            rider: get("rider"),
            textmate: get("mate"),
            bbedit: get("bbedit"),
            geany: get("geany"),
            kate: get("kate"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified editor's binary if installed.
    pub fn path(&self, editor: Editor) -> Option<PathBuf> {
        self.path_with_source(editor).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified editor if installed.
    pub fn path_with_source(&self, editor: Editor) -> Option<(PathBuf, ExecutableSource)> {
        match editor {
            Editor::Vi => self.vi.clone(),
            Editor::Vim => self.vim.clone(),
            Editor::Neovim => self.neovim.clone(),
            Editor::Emacs => self.emacs.clone(),
            Editor::XEmacs => self.xemacs.clone(),
            Editor::Nano => self.nano.clone(),
            Editor::Helix => self.helix.clone(),
            Editor::VSCode => self.vscode.clone(),
            Editor::VSCodium => self.vscodium.clone(),
            Editor::Sublime => self.sublime.clone(),
            Editor::Zed => self.zed.clone(),
            Editor::Micro => self.micro.clone(),
            Editor::Kakoune => self.kakoune.clone(),
            Editor::Amp => self.amp.clone(),
            Editor::Lapce => self.lapce.clone(),
            Editor::PhpStorm => self.phpstorm.clone(),
            Editor::IntellijIdea => self.intellij_idea.clone(),
            Editor::PyCharm => self.pycharm.clone(),
            Editor::WebStorm => self.webstorm.clone(),
            Editor::CLion => self.clion.clone(),
            Editor::GoLand => self.goland.clone(),
            Editor::Rider => self.rider.clone(),
            Editor::TextMate => self.textmate.clone(),
            Editor::BBEdit => self.bbedit.clone(),
            Editor::Geany => self.geany.clone(),
            Editor::Kate => self.kate.clone(),
        }
    }

    /// Returns the version of the specified editor if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The editor is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, editor: Editor) -> Result<String, ProgramError> {
        if !self.is_installed(editor) {
            return Err(ProgramError::NotFound(editor.binary_name().to_string()));
        }
        editor.version()
    }

    /// Returns the official website URL for the specified editor.
    pub fn website(&self, editor: Editor) -> &'static str {
        editor.website()
    }

    /// Returns a one-line description of the specified editor.
    pub fn description(&self, editor: Editor) -> &'static str {
        editor.description()
    }

    /// Checks if the specified editor is installed.
    pub fn is_installed(&self, editor: Editor) -> bool {
        match editor {
            Editor::Vi => self.vi.is_some(),
            Editor::Vim => self.vim.is_some(),
            Editor::Neovim => self.neovim.is_some(),
            Editor::Emacs => self.emacs.is_some(),
            Editor::XEmacs => self.xemacs.is_some(),
            Editor::Nano => self.nano.is_some(),
            Editor::Helix => self.helix.is_some(),
            Editor::VSCode => self.vscode.is_some(),
            Editor::VSCodium => self.vscodium.is_some(),
            Editor::Sublime => self.sublime.is_some(),
            Editor::Zed => self.zed.is_some(),
            Editor::Micro => self.micro.is_some(),
            Editor::Kakoune => self.kakoune.is_some(),
            Editor::Amp => self.amp.is_some(),
            Editor::Lapce => self.lapce.is_some(),
            Editor::PhpStorm => self.phpstorm.is_some(),
            Editor::IntellijIdea => self.intellij_idea.is_some(),
            Editor::PyCharm => self.pycharm.is_some(),
            Editor::WebStorm => self.webstorm.is_some(),
            Editor::CLion => self.clion.is_some(),
            Editor::GoLand => self.goland.is_some(),
            Editor::Rider => self.rider.is_some(),
            Editor::TextMate => self.textmate.is_some(),
            Editor::BBEdit => self.bbedit.is_some(),
            Editor::Geany => self.geany.is_some(),
            Editor::Kate => self.kate.is_some(),
        }
    }

    /// Returns a list of all installed editors.
    pub fn installed(&self) -> Vec<Editor> {
        use strum::IntoEnumIterator;
        Editor::iter().filter(|e| self.is_installed(*e)).collect()
    }
}

impl Serialize for InstalledEditors {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use strum::IntoEnumIterator;

        // Helper to create a ProgramEntry for an editor
        let entry = |editor: Editor| -> ProgramEntry {
            let info = editor.info();
            match self.path_with_source(editor) {
                Some((path, source)) => ProgramEntry::installed(info, path, source),
                None => ProgramEntry::not_installed(info),
            }
        };

        let mut state = serializer.serialize_struct("InstalledEditors", 26)?;
        for editor in Editor::iter() {
            let field_name = match editor {
                Editor::Vi => "vi",
                Editor::Vim => "vim",
                Editor::Neovim => "neovim",
                Editor::Emacs => "emacs",
                Editor::XEmacs => "xemacs",
                Editor::Nano => "nano",
                Editor::Helix => "helix",
                Editor::VSCode => "vscode",
                Editor::VSCodium => "vscodium",
                Editor::Sublime => "sublime",
                Editor::Zed => "zed",
                Editor::Micro => "micro",
                Editor::Kakoune => "kakoune",
                Editor::Amp => "amp",
                Editor::Lapce => "lapce",
                Editor::PhpStorm => "phpstorm",
                Editor::IntellijIdea => "intellij_idea",
                Editor::PyCharm => "pycharm",
                Editor::WebStorm => "webstorm",
                Editor::CLion => "clion",
                Editor::GoLand => "goland",
                Editor::Rider => "rider",
                Editor::TextMate => "textmate",
                Editor::BBEdit => "bbedit",
                Editor::Geany => "geany",
                Editor::Kate => "kate",
            };
            state.serialize_field(field_name, &entry(editor))?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledEditors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize from boolean fields - path info is lost but that's acceptable
        // for deserialization use cases (typically for display/reporting purposes).
        #[derive(Deserialize)]
        struct BoolEditors {
            #[serde(default)]
            vi: bool,
            #[serde(default)]
            vim: bool,
            #[serde(default)]
            neovim: bool,
            #[serde(default)]
            emacs: bool,
            #[serde(default)]
            xemacs: bool,
            #[serde(default)]
            nano: bool,
            #[serde(default)]
            helix: bool,
            #[serde(default)]
            vscode: bool,
            #[serde(default)]
            vscodium: bool,
            #[serde(default)]
            sublime: bool,
            #[serde(default)]
            zed: bool,
            #[serde(default)]
            micro: bool,
            #[serde(default)]
            kakoune: bool,
            #[serde(default)]
            amp: bool,
            #[serde(default)]
            lapce: bool,
            #[serde(default)]
            phpstorm: bool,
            #[serde(default)]
            intellij_idea: bool,
            #[serde(default)]
            pycharm: bool,
            #[serde(default)]
            webstorm: bool,
            #[serde(default)]
            clion: bool,
            #[serde(default)]
            goland: bool,
            #[serde(default)]
            rider: bool,
            #[serde(default)]
            textmate: bool,
            #[serde(default)]
            bbedit: bool,
            #[serde(default)]
            geany: bool,
            #[serde(default)]
            kate: bool,
        }

        let b = BoolEditors::deserialize(deserializer)?;

        // Convert bools to Option with placeholder path and default source
        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledEditors {
            vi: to_opt(b.vi),
            vim: to_opt(b.vim),
            neovim: to_opt(b.neovim),
            emacs: to_opt(b.emacs),
            xemacs: to_opt(b.xemacs),
            nano: to_opt(b.nano),
            helix: to_opt(b.helix),
            vscode: to_opt(b.vscode),
            vscodium: to_opt(b.vscodium),
            sublime: to_opt(b.sublime),
            zed: to_opt(b.zed),
            micro: to_opt(b.micro),
            kakoune: to_opt(b.kakoune),
            amp: to_opt(b.amp),
            lapce: to_opt(b.lapce),
            phpstorm: to_opt(b.phpstorm),
            intellij_idea: to_opt(b.intellij_idea),
            pycharm: to_opt(b.pycharm),
            webstorm: to_opt(b.webstorm),
            clion: to_opt(b.clion),
            goland: to_opt(b.goland),
            rider: to_opt(b.rider),
            textmate: to_opt(b.textmate),
            bbedit: to_opt(b.bbedit),
            geany: to_opt(b.geany),
            kate: to_opt(b.kate),
        })
    }
}

impl ProgramDetector for InstalledEditors {
    type Program = Editor;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledEditors::path(self, program)
    }

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledEditors::path_with_source(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledEditors::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledEditors::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledEditors::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledEditors::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = editor_details(program) else {
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
            editor_details(program).ok_or_else(|| SniffInstallationError::NotInstallableOnOs {
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
            editor_details(program).ok_or_else(|| SniffInstallationError::NotInstallableOnOs {
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
        let editors = InstalledEditors::default();
        assert!(editors.path_with_source(Editor::Vim).is_none());
    }

    #[test]
    fn test_is_installed_returns_false_for_default() {
        let editors = InstalledEditors::default();
        assert!(!editors.is_installed(Editor::Vim));
        assert!(!editors.is_installed(Editor::VSCode));
    }

    #[test]
    fn test_serialize_produces_program_entries() {
        let editors = InstalledEditors::default();
        let json = serde_json::to_string(&editors).unwrap();
        // Now produces ProgramEntry objects with full metadata
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"vim\":{"));
        assert!(json.contains("\"name\":\"Vim\""));
    }

    #[test]
    fn test_deserialize_from_boolean_fields() {
        let json = r#"{"vim": true, "vscode": false}"#;
        let editors: InstalledEditors = serde_json::from_str(json).unwrap();
        assert!(editors.is_installed(Editor::Vim));
        assert!(!editors.is_installed(Editor::VSCode));
    }

    #[test]
    fn test_serialize_to_json() {
        // Serialization produces rich ProgramEntry objects
        let original = InstalledEditors::default();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("vim").is_some());
    }

    #[test]
    fn test_path_returns_none_when_not_installed() {
        let editors = InstalledEditors::default();
        // path() should also return None for uninstalled editors
        assert!(editors.path(Editor::Vim).is_none());
        assert!(editors.path(Editor::VSCode).is_none());
    }

    #[test]
    fn test_installed_returns_empty_for_default() {
        let editors = InstalledEditors::default();
        assert!(editors.installed().is_empty());
    }

    #[test]
    fn test_version_returns_not_found_for_uninstalled() {
        let editors = InstalledEditors::default();
        let result = editors.version(Editor::Vim);
        assert!(result.is_err());
        if let Err(ProgramError::NotFound(name)) = result {
            assert_eq!(name, "vim");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_website_returns_static_str() {
        let editors = InstalledEditors::default();
        // Website should work regardless of installation status
        let website = editors.website(Editor::Vim);
        assert!(!website.is_empty());
        assert!(website.starts_with("http"));
    }

    #[test]
    fn test_description_returns_static_str() {
        let editors = InstalledEditors::default();
        let desc = editors.description(Editor::Vim);
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_deserialize_all_false_produces_empty_installed() {
        let json = r#"{
            "vi": false, "vim": false, "neovim": false, "emacs": false,
            "xemacs": false, "nano": false, "helix": false, "vscode": false,
            "vscodium": false, "sublime": false, "zed": false, "micro": false,
            "kakoune": false, "amp": false, "lapce": false, "phpstorm": false,
            "intellij_idea": false, "pycharm": false, "webstorm": false,
            "clion": false, "goland": false, "rider": false, "textmate": false,
            "bbedit": false, "geany": false, "kate": false
        }"#;
        let editors: InstalledEditors = serde_json::from_str(json).unwrap();
        assert!(editors.installed().is_empty());
    }

    #[test]
    fn test_deserialize_partial_json() {
        // Should handle partial JSON with missing fields
        let json = r#"{"vim": true}"#;
        let editors: InstalledEditors = serde_json::from_str(json).unwrap();
        assert!(editors.is_installed(Editor::Vim));
        // Other editors should default to false/not installed
        assert!(!editors.is_installed(Editor::VSCode));
        assert!(!editors.is_installed(Editor::Neovim));
    }

    #[test]
    fn test_default_equals_all_none() {
        let default = InstalledEditors::default();
        let explicit = InstalledEditors {
            vi: None,
            vim: None,
            neovim: None,
            emacs: None,
            xemacs: None,
            nano: None,
            helix: None,
            vscode: None,
            vscodium: None,
            sublime: None,
            zed: None,
            micro: None,
            kakoune: None,
            amp: None,
            lapce: None,
            phpstorm: None,
            intellij_idea: None,
            pycharm: None,
            webstorm: None,
            clion: None,
            goland: None,
            rider: None,
            textmate: None,
            bbedit: None,
            geany: None,
            kate: None,
        };
        assert_eq!(default, explicit);
    }

    #[test]
    fn test_clone_produces_equal_struct() {
        let editors = InstalledEditors::default();
        let cloned = editors.clone();
        assert_eq!(editors, cloned);
    }

    #[test]
    fn test_path_with_source_all_editors_default() {
        let editors = InstalledEditors::default();
        // Test all editor variants return None for default
        use strum::IntoEnumIterator;
        for editor in Editor::iter() {
            assert!(
                editors.path_with_source(editor).is_none(),
                "{:?} should return None for default",
                editor
            );
        }
    }

    #[test]
    fn test_is_installed_all_editors_default() {
        let editors = InstalledEditors::default();
        use strum::IntoEnumIterator;
        for editor in Editor::iter() {
            assert!(
                !editors.is_installed(editor),
                "{:?} should not be installed for default",
                editor
            );
        }
    }

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;

        #[test]
        fn test_path_with_source_detects_source_correctly() {
            // This test verifies that detection happens - actual results depend on
            // what's installed on the test machine
            let editors = InstalledEditors::new();

            // Check any installed editor has correct source info
            for editor in editors.installed() {
                let result = editors.path_with_source(editor);
                assert!(result.is_some(), "{:?} should have path info", editor);
                let (path, source) = result.unwrap();
                assert!(!path.as_os_str().is_empty(), "Path should not be empty");
                // Source should be either Path or MacOsAppBundle
                assert!(
                    source == ExecutableSource::Path
                        || source == ExecutableSource::MacOsAppBundle,
                    "Source should be valid"
                );
            }
        }
    }
}
