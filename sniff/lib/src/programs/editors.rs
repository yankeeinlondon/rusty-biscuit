use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::Editor;
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct InstalledEditors {
    /// The classic vi editor. [Website](https://www.vim.org/)
    pub vi: bool,
    /// Vim (Vi IMproved). [Website](https://www.vim.org/)
    pub vim: bool,
    /// Neovim, a hyper-extensible Vim-based text editor. [Website](https://neovim.io/)
    pub neovim: bool,
    /// GNU Emacs, an extensible, customizable, self-documenting, real-time display editor. [Website](https://www.gnu.org/software/emacs/)
    pub emacs: bool,
    /// XEmacs, a version of Emacs that branched from GNU Emacs. [Website](http://www.xemacs.org/)
    pub xemacs: bool,
    /// GNU nano, a small and friendly text editor. [Website](https://www.nano-editor.org/)
    pub nano: bool,
    /// Helix, a post-modern modal text editor. [Website](https://helix-editor.com/)
    pub helix: bool,
    /// Visual Studio Code, a code editor redefined and optimized for building and debugging modern web and cloud applications. [Website](https://code.visualstudio.com/)
    pub vscode: bool,
    /// VSCodium, free/libre open source software binaries of VS Code. [Website](https://vscodium.com/)
    pub vscodium: bool,
    /// Sublime Text, a sophisticated text editor for code, markup and prose. [Website](https://www.sublimetext.com/)
    pub sublime: bool,
    /// Zed, a high-performance, multiplayer code editor from the creators of Atom and Tree-sitter. [Website](https://zed.dev/)
    pub zed: bool,
    /// Micro, a terminal-based text editor that aims to be easy to use and intuitive. [Website](https://micro-editor.github.io/)
    pub micro: bool,
    /// Kakoune, a modal editor inspired by Vi but with a different selection-based editing model. [Website](https://kakoune.org/)
    pub kakoune: bool,
    /// Amp, a modal text editor for the terminal inspired by Vi/Vim. [Website](https://amp.readme.io/)
    pub amp: bool,
    /// Lapce, a lightning-fast and powerful code editor written in Rust. [Website](https://lapce.dev/)
    pub lapce: bool,
    /// PhpStorm, a lightning-smart PHP IDE by JetBrains. [Website](https://www.jetbrains.com/phpstorm/)
    pub phpstorm: bool,
    /// IntelliJ IDEA, a capable and ergonomic IDE for JVM-based languages. [Website](https://www.jetbrains.com/idea/)
    pub intellij_idea: bool,
    /// PyCharm, the Python IDE for professional developers. [Website](https://www.jetbrains.com/pycharm/)
    pub pycharm: bool,
    /// WebStorm, the smartest JavaScript IDE. [Website](https://www.jetbrains.com/webstorm/)
    pub webstorm: bool,
    /// CLion, a cross-platform C and C++ IDE. [Website](https://www.jetbrains.com/clion/)
    pub clion: bool,
    /// GoLand, a cross-platform Go IDE. [Website](https://www.jetbrains.com/go/)
    pub goland: bool,
    /// Rider, a fast and powerful cross-platform .NET IDE. [Website](https://www.jetbrains.com/rider/)
    pub rider: bool,
    /// TextMate, a versatile plain text editor for macOS. [Website](https://macromates.com/)
    pub textmate: bool,
    /// BBEdit, a professional HTML and text editor for macOS. [Website](https://www.barebones.com/products/bbedit/)
    pub bbedit: bool,
    /// Geany, a powerful, stable and lightweight programmer's text editor. [Website](https://www.geany.org/)
    pub geany: bool,
    /// Kate, a multi-document, multi-view text editor by KDE. [Website](https://kate-editor.org/)
    pub kate: bool,
}

impl InstalledEditors {
    /// Detect which popular editors are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "vi", "vim", "nvim", "emacs", "xemacs", "nano", "hx", "code", "codium", "subl", "zed",
            "micro", "kak", "amp", "lapce", "phpstorm", "idea", "pycharm", "webstorm", "clion",
            "goland", "rider", "mate", "bbedit", "geany", "kate",
        ];

        let results = find_programs_parallel(&programs);

        let has = |name: &str| results.get(name).and_then(|r| r.as_ref()).is_some();

        Self {
            vi: has("vi"),
            vim: has("vim"),
            neovim: has("nvim"),
            emacs: has("emacs"),
            xemacs: has("xemacs"),
            nano: has("nano"),
            helix: has("hx"),
            vscode: has("code"),
            vscodium: has("codium"),
            sublime: has("subl"),
            zed: has("zed"),
            micro: has("micro"),
            kakoune: has("kak"),
            amp: has("amp"),
            lapce: has("lapce"),
            phpstorm: has("phpstorm"),
            intellij_idea: has("idea"),
            pycharm: has("pycharm"),
            webstorm: has("webstorm"),
            clion: has("clion"),
            goland: has("goland"),
            rider: has("rider"),
            textmate: has("mate"),
            bbedit: has("bbedit"),
            geany: has("geany"),
            kate: has("kate"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified editor's binary if installed.
    pub fn path(&self, editor: Editor) -> Option<PathBuf> {
        if self.is_installed(editor) {
            editor.path()
        } else {
            None
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
            Editor::Vi => self.vi,
            Editor::Vim => self.vim,
            Editor::Neovim => self.neovim,
            Editor::Emacs => self.emacs,
            Editor::XEmacs => self.xemacs,
            Editor::Nano => self.nano,
            Editor::Helix => self.helix,
            Editor::VSCode => self.vscode,
            Editor::VSCodium => self.vscodium,
            Editor::Sublime => self.sublime,
            Editor::Zed => self.zed,
            Editor::Micro => self.micro,
            Editor::Kakoune => self.kakoune,
            Editor::Amp => self.amp,
            Editor::Lapce => self.lapce,
            Editor::PhpStorm => self.phpstorm,
            Editor::IntellijIdea => self.intellij_idea,
            Editor::PyCharm => self.pycharm,
            Editor::WebStorm => self.webstorm,
            Editor::CLion => self.clion,
            Editor::GoLand => self.goland,
            Editor::Rider => self.rider,
            Editor::TextMate => self.textmate,
            Editor::BBEdit => self.bbedit,
            Editor::Geany => self.geany,
            Editor::Kate => self.kate,
        }
    }

    /// Returns a list of all installed editors.
    pub fn installed(&self) -> Vec<Editor> {
        use strum::IntoEnumIterator;
        Editor::iter().filter(|e| self.is_installed(*e)).collect()
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
