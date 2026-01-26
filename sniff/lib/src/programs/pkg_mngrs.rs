use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::find_program::find_programs_parallel;
use crate::programs::installer::{
    execute_install, execute_versioned_install, method_available, select_best_method,
    InstallOptions,
};
use crate::programs::schema::{ProgramError, ProgramMetadata};
use crate::programs::types::ProgramDetector;
use crate::programs::{Program, PROGRAM_LOOKUP};

fn lang_pkg_mgr_details(
    pkg_mgr: LanguagePackageManager,
) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match pkg_mgr {
        LanguagePackageManager::Npm => Program::Npm,
        LanguagePackageManager::Pnpm => Program::Pnpm,
        LanguagePackageManager::Yarn => Program::Yarn,
        LanguagePackageManager::Bun => Program::Bun,
        LanguagePackageManager::Cargo => Program::Cargo,
        LanguagePackageManager::GoModules => Program::GoModules,
        LanguagePackageManager::Composer => Program::Composer,
        LanguagePackageManager::SwiftPm => Program::SwiftPm,
        LanguagePackageManager::Luarocks => Program::Luarocks,
        LanguagePackageManager::Vcpkg => Program::Vcpkg,
        LanguagePackageManager::Conan => Program::Conan,
        LanguagePackageManager::Nuget => Program::Nuget,
        LanguagePackageManager::Hex => Program::Hex,
        LanguagePackageManager::Pip => Program::Pip,
        LanguagePackageManager::Uv => Program::Uv,
        LanguagePackageManager::Poetry => Program::Poetry,
        LanguagePackageManager::Cpan => Program::Cpan,
        LanguagePackageManager::Cpanm => Program::Cpanm,
    };

    PROGRAM_LOOKUP.get(&program)
}

fn os_pkg_mgr_details(
    pkg_mgr: OsPackageManager,
) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match pkg_mgr {
        OsPackageManager::Apt => Program::Apt,
        OsPackageManager::Nala => Program::Nala,
        OsPackageManager::Brew => Program::Brew,
        OsPackageManager::Dnf => Program::Dnf,
        OsPackageManager::Pacman => Program::Pacman,
        OsPackageManager::Winget => Program::Winget,
        OsPackageManager::Chocolatey => Program::Chocolatey,
        OsPackageManager::Scoop => Program::Scoop,
        OsPackageManager::Nix => Program::Nix,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Language-specific package managers found on the system.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct InstalledLanguagePackageManagers {
    /// Default Node.js package manager; registry, dependency resolution, scripts. [Website](https://www.npmjs.com)
    pub npm: bool,
    /// Disk-efficient Node.js package manager using a content-addressable store. [Website](https://pnpm.io)
    pub pnpm: bool,
    /// Alternative Node.js package manager; workspaces, Plug’n’Play. [Website](https://yarnpkg.com)
    pub yarn: bool,
    /// All-in-one JS runtime with built-in package manager. [Website](https://bun.sh)
    pub bun: bool,
    /// Official Rust package manager and build tool. [Website](https://doc.rust-lang.org/cargo)
    pub cargo: bool,
    /// Built-in Go dependency system integrated with the go tool. [Website](https://go.dev/ref/mod)
    pub go_modules: bool,
    /// Dependency manager for modern PHP applications. [Website](https://getcomposer.org)
    pub composer: bool,
    /// Official Swift dependency manager, integrated with the Swift toolchain. [Website](https://www.swift.org/package-manager)
    pub swift_pm: bool,
    /// Standard package manager for Lua modules. [Website](https://luarocks.org)
    pub luarocks: bool,
    /// Cross-platform C/C++ dependency manager backed by Microsoft. [Website](https://vcpkg.io)
    pub vcpkg: bool,
    /// Decentralized C/C++ package manager with build-system integration. [Website](https://conan.io)
    pub conan: bool,
    /// Official package manager for .NET and C# ecosystems. [Website](https://www.nuget.org)
    pub nuget: bool,
    /// Package manager for the BEAM (Elixir, Erlang) ecosystem. [Website](https://hex.pm)
    pub hex: bool,
    /// Traditional Python package installer. [Website](https://pip.pypa.io)
    pub pip: bool,
    /// High-performance Python package manager and virtual environment tool. [Website](https://astral.sh/uv)
    pub uv: bool,
    /// Dependency manager and build system with lockfile support. [Website](https://python-poetry.org)
    pub poetry: bool,
    /// Canonical archive and installer for Perl modules. [Website](https://www.cpan.org)
    pub cpan: bool,
    /// Lightweight, scriptable CPAN client. [Website](https://metacpan.org/pod/App::cpanminus)
    pub cpanm: bool,
}

impl InstalledLanguagePackageManagers {
    /// Detect which popular language package managers are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "npm", "pnpm", "yarn", "bun", "cargo", "go", "composer", "swift", "luarocks", "vcpkg",
            "conan", "dotnet", "nuget", "mix", "pip", "pip3", "uv", "poetry", "cpan", "cpanm",
        ];

        let results = find_programs_parallel(&programs);

        let has = |name: &str| results.get(name).and_then(|r| r.as_ref()).is_some();
        let any = |names: &[&str]| names.iter().any(|&name| has(name));

        Self {
            npm: has("npm"),
            pnpm: has("pnpm"),
            yarn: has("yarn"),
            bun: has("bun"),
            cargo: has("cargo"),
            go_modules: has("go"),
            composer: has("composer"),
            swift_pm: has("swift"),
            luarocks: has("luarocks"),
            vcpkg: has("vcpkg"),
            conan: has("conan"),
            nuget: any(&["dotnet", "nuget"]),
            hex: has("mix"),
            pip: any(&["pip", "pip3"]),
            uv: has("uv"),
            poetry: has("poetry"),
            cpan: has("cpan"),
            cpanm: has("cpanm"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified package manager's binary if installed.
    pub fn path(&self, pkg_mgr: LanguagePackageManager) -> Option<PathBuf> {
        if self.is_installed(pkg_mgr) {
            pkg_mgr.path()
        } else {
            None
        }
    }

    /// Returns the version of the specified package manager if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The package manager is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, pkg_mgr: LanguagePackageManager) -> Result<String, ProgramError> {
        if !self.is_installed(pkg_mgr) {
            return Err(ProgramError::NotFound(pkg_mgr.binary_name().to_string()));
        }
        pkg_mgr.version()
    }

    /// Returns the official website URL for the specified package manager.
    pub fn website(&self, pkg_mgr: LanguagePackageManager) -> &'static str {
        pkg_mgr.website()
    }

    /// Returns a one-line description of the specified package manager.
    pub fn description(&self, pkg_mgr: LanguagePackageManager) -> &'static str {
        pkg_mgr.description()
    }

    /// Checks if the specified package manager is installed.
    pub fn is_installed(&self, pkg_mgr: LanguagePackageManager) -> bool {
        match pkg_mgr {
            LanguagePackageManager::Npm => self.npm,
            LanguagePackageManager::Pnpm => self.pnpm,
            LanguagePackageManager::Yarn => self.yarn,
            LanguagePackageManager::Bun => self.bun,
            LanguagePackageManager::Cargo => self.cargo,
            LanguagePackageManager::GoModules => self.go_modules,
            LanguagePackageManager::Composer => self.composer,
            LanguagePackageManager::SwiftPm => self.swift_pm,
            LanguagePackageManager::Luarocks => self.luarocks,
            LanguagePackageManager::Vcpkg => self.vcpkg,
            LanguagePackageManager::Conan => self.conan,
            LanguagePackageManager::Nuget => self.nuget,
            LanguagePackageManager::Hex => self.hex,
            LanguagePackageManager::Pip => self.pip,
            LanguagePackageManager::Uv => self.uv,
            LanguagePackageManager::Poetry => self.poetry,
            LanguagePackageManager::Cpan => self.cpan,
            LanguagePackageManager::Cpanm => self.cpanm,
        }
    }

    /// Returns a list of all installed language package managers.
    pub fn installed(&self) -> Vec<LanguagePackageManager> {
        use strum::IntoEnumIterator;
        LanguagePackageManager::iter()
            .filter(|p| self.is_installed(*p))
            .collect()
    }
}

impl ProgramDetector for InstalledLanguagePackageManagers {
    type Program = LanguagePackageManager;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledLanguagePackageManagers::path(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledLanguagePackageManagers::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledLanguagePackageManagers::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledLanguagePackageManagers::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledLanguagePackageManagers::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = lang_pkg_mgr_details(program) else {
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
        let details = lang_pkg_mgr_details(program).ok_or_else(|| {
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
        let details = lang_pkg_mgr_details(program).ok_or_else(|| {
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

/// OS-level package managers found on the system.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct InstalledOsPackageManagers {
    /// Debian/Ubuntu primary package manager. [Website](https://tracker.debian.org/pkg/apt)
    pub apt: bool,
    /// Modern apt frontend with parallel downloads. [Website](https://github.com/volitank/nala)
    pub nala: bool,
    /// macOS/Linux community package manager. [Website](https://brew.sh)
    pub brew: bool,
    /// Fedora/RHEL primary package manager. [Website](https://github.com/rpm-software-management/dnf)
    pub dnf: bool,
    /// Arch Linux package manager. [Website](https://archlinux.org/pacman/)
    pub pacman: bool,
    /// Windows Package Manager. [Website](https://github.com/microsoft/winget-cli)
    pub winget: bool,
    /// Windows community package manager. [Website](https://chocolatey.org)
    pub chocolatey: bool,
    /// Windows command-line installer. [Website](https://scoop.sh)
    pub scoop: bool,
    /// Nix package manager. [Website](https://nixos.org)
    pub nix: bool,
}

impl InstalledOsPackageManagers {
    /// Detect which popular OS package managers are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "apt", "nala", "brew", "dnf", "yum", "pacman", "winget", "choco", "scoop", "nix",
        ];

        let results = find_programs_parallel(&programs);

        let has = |name: &str| results.get(name).and_then(|r| r.as_ref()).is_some();
        let any = |names: &[&str]| names.iter().any(|&name| has(name));

        Self {
            apt: has("apt"),
            nala: has("nala"),
            brew: has("brew"),
            dnf: any(&["dnf", "yum"]),
            pacman: has("pacman"),
            winget: has("winget"),
            chocolatey: has("choco"),
            scoop: has("scoop"),
            nix: has("nix"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified package manager's binary if installed.
    pub fn path(&self, pkg_mgr: OsPackageManager) -> Option<PathBuf> {
        if self.is_installed(pkg_mgr) {
            pkg_mgr.path()
        } else {
            None
        }
    }

    /// Returns the version of the specified package manager if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The package manager is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, pkg_mgr: OsPackageManager) -> Result<String, ProgramError> {
        if !self.is_installed(pkg_mgr) {
            return Err(ProgramError::NotFound(pkg_mgr.binary_name().to_string()));
        }
        pkg_mgr.version()
    }

    /// Returns the official website URL for the specified package manager.
    pub fn website(&self, pkg_mgr: OsPackageManager) -> &'static str {
        pkg_mgr.website()
    }

    /// Returns a one-line description of the specified package manager.
    pub fn description(&self, pkg_mgr: OsPackageManager) -> &'static str {
        pkg_mgr.description()
    }

    /// Checks if the specified package manager is installed.
    pub fn is_installed(&self, pkg_mgr: OsPackageManager) -> bool {
        match pkg_mgr {
            OsPackageManager::Apt => self.apt,
            OsPackageManager::Nala => self.nala,
            OsPackageManager::Brew => self.brew,
            OsPackageManager::Dnf => self.dnf,
            OsPackageManager::Pacman => self.pacman,
            OsPackageManager::Winget => self.winget,
            OsPackageManager::Chocolatey => self.chocolatey,
            OsPackageManager::Scoop => self.scoop,
            OsPackageManager::Nix => self.nix,
        }
    }

    /// Returns a list of all installed OS package managers.
    pub fn installed(&self) -> Vec<OsPackageManager> {
        use strum::IntoEnumIterator;
        OsPackageManager::iter()
            .filter(|p| self.is_installed(*p))
            .collect()
    }
}

impl ProgramDetector for InstalledOsPackageManagers {
    type Program = OsPackageManager;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledOsPackageManagers::path(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledOsPackageManagers::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledOsPackageManagers::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledOsPackageManagers::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledOsPackageManagers::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = os_pkg_mgr_details(program) else {
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
        let details = os_pkg_mgr_details(program).ok_or_else(|| {
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
        let details = os_pkg_mgr_details(program).ok_or_else(|| {
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
