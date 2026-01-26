use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::TerminalApp;
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct InstalledTerminalApps {
    /// A fast, GPU-accelerated terminal emulator. [Website](https://alacritty.org/)
    pub alacritty: bool,
    /// A fast, feature-rich, GPU-based terminal emulator. [Website](https://sw.kovidgoyal.net/kitty/)
    pub kitty: bool,
    /// A terminal emulator for macOS that does amazing things. [Website](https://iterm2.com/)
    pub iterm2: bool,
    /// A GPU-accelerated cross-platform terminal emulator and multiplexer. [Website](https://wezfurlong.org/wezterm/)
    pub wezterm: bool,
    /// A fast, feature-rich, GPU-accelerated terminal emulator written in Zig. [Website](https://ghostty.org/)
    pub ghostty: bool,
    /// A modern, Rust-based terminal with built-in AI. [Website](https://www.warp.dev/)
    pub warp: bool,
    /// A hardware-accelerated GPU terminal emulator focusing on performance. [Website](https://github.com/raphamorim/rio)
    pub rio: bool,
    /// A terminal for a more modern age. [Website](https://tabby.sh/)
    pub tabby: bool,
    /// A fast, lightweight and minimalistic Wayland terminal emulator. [Website](https://codeberg.org/dnkl/foot)
    pub foot: bool,
    /// The default terminal emulator for the GNOME desktop environment. [Website](https://help.gnome.org/users/gnome-terminal/stable/)
    pub gnome_terminal: bool,
    /// A terminal emulator by KDE. [Website](https://konsole.kde.org/)
    pub konsole: bool,
    /// Terminal emulator for Xfce. [Website](https://docs.xfce.org/apps/xfce4-terminal/start)
    pub xfce_terminal: bool,
    /// A terminal emulator, and more, based on Enlightenment Foundation Libraries. [Website](https://www.enlightenment.org/about-terminology)
    pub terminology: bool,
    /// simple terminal (st) is a simple terminal emulator for X which sucks less. [Website](https://st.suckless.org/)
    pub st: bool,
    /// The standard terminal emulator for the X Window System. [Website](https://invisible-island.net/xterm/)
    pub xterm: bool,
    /// A terminal built on web technologies. [Website](https://hyper.is/)
    pub hyper: bool,
    /// A modern, fast, efficient, powerful, and productive terminal application for Windows. [Website](https://github.com/microsoft/terminal)
    pub windows_terminal: bool,
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

        let results = find_programs_parallel(&programs);

        let has = |name: &str| results.get(name).and_then(|r| r.as_ref()).is_some();
        let any = |names: &[&str]| names.iter().any(|&name| has(name));

        Self {
            alacritty: has("alacritty"),
            kitty: has("kitty"),
            iterm2: has("iterm2"),
            wezterm: has("wezterm"),
            ghostty: has("ghostty"),
            warp: any(&["warp-terminal", "warp"]),
            rio: has("rio"),
            tabby: has("tabby"),
            foot: has("foot"),
            gnome_terminal: has("gnome-terminal"),
            konsole: has("konsole"),
            xfce_terminal: has("xfce4-terminal"),
            terminology: has("terminology"),
            st: has("st"),
            xterm: has("xterm"),
            hyper: has("hyper"),
            windows_terminal: has("wt"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified terminal app's binary if installed.
    pub fn path(&self, app: TerminalApp) -> Option<PathBuf> {
        if self.is_installed(app) {
            app.path()
        } else {
            None
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
            TerminalApp::Alacritty => self.alacritty,
            TerminalApp::Kitty => self.kitty,
            TerminalApp::ITerm2 => self.iterm2,
            TerminalApp::WezTerm => self.wezterm,
            TerminalApp::Ghostty => self.ghostty,
            TerminalApp::Warp => self.warp,
            TerminalApp::Rio => self.rio,
            TerminalApp::Tabby => self.tabby,
            TerminalApp::Foot => self.foot,
            TerminalApp::GnomeTerminal => self.gnome_terminal,
            TerminalApp::Konsole => self.konsole,
            TerminalApp::XfceTerminal => self.xfce_terminal,
            TerminalApp::Terminology => self.terminology,
            TerminalApp::St => self.st,
            TerminalApp::Xterm => self.xterm,
            TerminalApp::Hyper => self.hyper,
            TerminalApp::WindowsTerminal => self.windows_terminal,
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
