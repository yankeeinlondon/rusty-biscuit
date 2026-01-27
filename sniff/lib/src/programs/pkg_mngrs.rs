use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::find_program::find_programs_with_source_parallel;
use crate::programs::installer::{
    execute_install, execute_versioned_install, method_available, select_best_method,
    InstallOptions,
};
use crate::programs::schema::{ProgramEntry, ProgramError, ProgramMetadata};
use crate::programs::types::{ExecutableSource, ProgramDetector};
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
///
/// Stores path and discovery source for each installed package manager.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledLanguagePackageManagers {
    npm: Option<(PathBuf, ExecutableSource)>,
    pnpm: Option<(PathBuf, ExecutableSource)>,
    yarn: Option<(PathBuf, ExecutableSource)>,
    bun: Option<(PathBuf, ExecutableSource)>,
    cargo: Option<(PathBuf, ExecutableSource)>,
    go_modules: Option<(PathBuf, ExecutableSource)>,
    composer: Option<(PathBuf, ExecutableSource)>,
    swift_pm: Option<(PathBuf, ExecutableSource)>,
    luarocks: Option<(PathBuf, ExecutableSource)>,
    vcpkg: Option<(PathBuf, ExecutableSource)>,
    conan: Option<(PathBuf, ExecutableSource)>,
    nuget: Option<(PathBuf, ExecutableSource)>,
    hex: Option<(PathBuf, ExecutableSource)>,
    pip: Option<(PathBuf, ExecutableSource)>,
    uv: Option<(PathBuf, ExecutableSource)>,
    poetry: Option<(PathBuf, ExecutableSource)>,
    cpan: Option<(PathBuf, ExecutableSource)>,
    cpanm: Option<(PathBuf, ExecutableSource)>,
}

impl InstalledLanguagePackageManagers {
    /// Detect which popular language package managers are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "npm", "pnpm", "yarn", "bun", "cargo", "go", "composer", "swift", "luarocks", "vcpkg",
            "conan", "dotnet", "nuget", "mix", "pip", "pip3", "uv", "poetry", "cpan", "cpanm",
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
            npm: get("npm"),
            pnpm: get("pnpm"),
            yarn: get("yarn"),
            bun: get("bun"),
            cargo: get("cargo"),
            go_modules: get("go"),
            composer: get("composer"),
            swift_pm: get("swift"),
            luarocks: get("luarocks"),
            vcpkg: get("vcpkg"),
            conan: get("conan"),
            nuget: get_any(&["dotnet", "nuget"]),
            hex: get("mix"),
            pip: get_any(&["pip", "pip3"]),
            uv: get("uv"),
            poetry: get("poetry"),
            cpan: get("cpan"),
            cpanm: get("cpanm"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified package manager's binary if installed.
    pub fn path(&self, pkg_mgr: LanguagePackageManager) -> Option<PathBuf> {
        self.path_with_source(pkg_mgr).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified package manager if installed.
    pub fn path_with_source(
        &self,
        pkg_mgr: LanguagePackageManager,
    ) -> Option<(PathBuf, ExecutableSource)> {
        match pkg_mgr {
            LanguagePackageManager::Npm => self.npm.clone(),
            LanguagePackageManager::Pnpm => self.pnpm.clone(),
            LanguagePackageManager::Yarn => self.yarn.clone(),
            LanguagePackageManager::Bun => self.bun.clone(),
            LanguagePackageManager::Cargo => self.cargo.clone(),
            LanguagePackageManager::GoModules => self.go_modules.clone(),
            LanguagePackageManager::Composer => self.composer.clone(),
            LanguagePackageManager::SwiftPm => self.swift_pm.clone(),
            LanguagePackageManager::Luarocks => self.luarocks.clone(),
            LanguagePackageManager::Vcpkg => self.vcpkg.clone(),
            LanguagePackageManager::Conan => self.conan.clone(),
            LanguagePackageManager::Nuget => self.nuget.clone(),
            LanguagePackageManager::Hex => self.hex.clone(),
            LanguagePackageManager::Pip => self.pip.clone(),
            LanguagePackageManager::Uv => self.uv.clone(),
            LanguagePackageManager::Poetry => self.poetry.clone(),
            LanguagePackageManager::Cpan => self.cpan.clone(),
            LanguagePackageManager::Cpanm => self.cpanm.clone(),
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
            LanguagePackageManager::Npm => self.npm.is_some(),
            LanguagePackageManager::Pnpm => self.pnpm.is_some(),
            LanguagePackageManager::Yarn => self.yarn.is_some(),
            LanguagePackageManager::Bun => self.bun.is_some(),
            LanguagePackageManager::Cargo => self.cargo.is_some(),
            LanguagePackageManager::GoModules => self.go_modules.is_some(),
            LanguagePackageManager::Composer => self.composer.is_some(),
            LanguagePackageManager::SwiftPm => self.swift_pm.is_some(),
            LanguagePackageManager::Luarocks => self.luarocks.is_some(),
            LanguagePackageManager::Vcpkg => self.vcpkg.is_some(),
            LanguagePackageManager::Conan => self.conan.is_some(),
            LanguagePackageManager::Nuget => self.nuget.is_some(),
            LanguagePackageManager::Hex => self.hex.is_some(),
            LanguagePackageManager::Pip => self.pip.is_some(),
            LanguagePackageManager::Uv => self.uv.is_some(),
            LanguagePackageManager::Poetry => self.poetry.is_some(),
            LanguagePackageManager::Cpan => self.cpan.is_some(),
            LanguagePackageManager::Cpanm => self.cpanm.is_some(),
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

impl Serialize for InstalledLanguagePackageManagers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use strum::IntoEnumIterator;

        let entry = |mgr: LanguagePackageManager| -> ProgramEntry {
            let info = mgr.info();
            match self.path_with_source(mgr) {
                Some((path, source)) => ProgramEntry::installed(info, path, source),
                None => ProgramEntry::not_installed(info),
            }
        };

        let mut state = serializer.serialize_struct("InstalledLanguagePackageManagers", 18)?;
        for mgr in LanguagePackageManager::iter() {
            let field_name = match mgr {
                LanguagePackageManager::Npm => "npm",
                LanguagePackageManager::Pnpm => "pnpm",
                LanguagePackageManager::Yarn => "yarn",
                LanguagePackageManager::Bun => "bun",
                LanguagePackageManager::Cargo => "cargo",
                LanguagePackageManager::GoModules => "go_modules",
                LanguagePackageManager::Composer => "composer",
                LanguagePackageManager::SwiftPm => "swift_pm",
                LanguagePackageManager::Luarocks => "luarocks",
                LanguagePackageManager::Vcpkg => "vcpkg",
                LanguagePackageManager::Conan => "conan",
                LanguagePackageManager::Nuget => "nuget",
                LanguagePackageManager::Hex => "hex",
                LanguagePackageManager::Pip => "pip",
                LanguagePackageManager::Uv => "uv",
                LanguagePackageManager::Poetry => "poetry",
                LanguagePackageManager::Cpan => "cpan",
                LanguagePackageManager::Cpanm => "cpanm",
            };
            state.serialize_field(field_name, &entry(mgr))?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledLanguagePackageManagers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoolLangPkgMgrs {
            #[serde(default)]
            npm: bool,
            #[serde(default)]
            pnpm: bool,
            #[serde(default)]
            yarn: bool,
            #[serde(default)]
            bun: bool,
            #[serde(default)]
            cargo: bool,
            #[serde(default)]
            go_modules: bool,
            #[serde(default)]
            composer: bool,
            #[serde(default)]
            swift_pm: bool,
            #[serde(default)]
            luarocks: bool,
            #[serde(default)]
            vcpkg: bool,
            #[serde(default)]
            conan: bool,
            #[serde(default)]
            nuget: bool,
            #[serde(default)]
            hex: bool,
            #[serde(default)]
            pip: bool,
            #[serde(default)]
            uv: bool,
            #[serde(default)]
            poetry: bool,
            #[serde(default)]
            cpan: bool,
            #[serde(default)]
            cpanm: bool,
        }

        let b = BoolLangPkgMgrs::deserialize(deserializer)?;

        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledLanguagePackageManagers {
            npm: to_opt(b.npm),
            pnpm: to_opt(b.pnpm),
            yarn: to_opt(b.yarn),
            bun: to_opt(b.bun),
            cargo: to_opt(b.cargo),
            go_modules: to_opt(b.go_modules),
            composer: to_opt(b.composer),
            swift_pm: to_opt(b.swift_pm),
            luarocks: to_opt(b.luarocks),
            vcpkg: to_opt(b.vcpkg),
            conan: to_opt(b.conan),
            nuget: to_opt(b.nuget),
            hex: to_opt(b.hex),
            pip: to_opt(b.pip),
            uv: to_opt(b.uv),
            poetry: to_opt(b.poetry),
            cpan: to_opt(b.cpan),
            cpanm: to_opt(b.cpanm),
        })
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

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledLanguagePackageManagers::path_with_source(self, program)
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
///
/// Stores path and discovery source for each installed package manager.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledOsPackageManagers {
    apt: Option<(PathBuf, ExecutableSource)>,
    nala: Option<(PathBuf, ExecutableSource)>,
    brew: Option<(PathBuf, ExecutableSource)>,
    dnf: Option<(PathBuf, ExecutableSource)>,
    pacman: Option<(PathBuf, ExecutableSource)>,
    winget: Option<(PathBuf, ExecutableSource)>,
    chocolatey: Option<(PathBuf, ExecutableSource)>,
    scoop: Option<(PathBuf, ExecutableSource)>,
    nix: Option<(PathBuf, ExecutableSource)>,
}

impl InstalledOsPackageManagers {
    /// Detect which popular OS package managers are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "apt", "nala", "brew", "dnf", "yum", "pacman", "winget", "choco", "scoop", "nix",
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
            apt: get("apt"),
            nala: get("nala"),
            brew: get("brew"),
            dnf: get_any(&["dnf", "yum"]),
            pacman: get("pacman"),
            winget: get("winget"),
            chocolatey: get("choco"),
            scoop: get("scoop"),
            nix: get("nix"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified package manager's binary if installed.
    pub fn path(&self, pkg_mgr: OsPackageManager) -> Option<PathBuf> {
        self.path_with_source(pkg_mgr).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified package manager if installed.
    pub fn path_with_source(
        &self,
        pkg_mgr: OsPackageManager,
    ) -> Option<(PathBuf, ExecutableSource)> {
        match pkg_mgr {
            OsPackageManager::Apt => self.apt.clone(),
            OsPackageManager::Nala => self.nala.clone(),
            OsPackageManager::Brew => self.brew.clone(),
            OsPackageManager::Dnf => self.dnf.clone(),
            OsPackageManager::Pacman => self.pacman.clone(),
            OsPackageManager::Winget => self.winget.clone(),
            OsPackageManager::Chocolatey => self.chocolatey.clone(),
            OsPackageManager::Scoop => self.scoop.clone(),
            OsPackageManager::Nix => self.nix.clone(),
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
            OsPackageManager::Apt => self.apt.is_some(),
            OsPackageManager::Nala => self.nala.is_some(),
            OsPackageManager::Brew => self.brew.is_some(),
            OsPackageManager::Dnf => self.dnf.is_some(),
            OsPackageManager::Pacman => self.pacman.is_some(),
            OsPackageManager::Winget => self.winget.is_some(),
            OsPackageManager::Chocolatey => self.chocolatey.is_some(),
            OsPackageManager::Scoop => self.scoop.is_some(),
            OsPackageManager::Nix => self.nix.is_some(),
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

impl Serialize for InstalledOsPackageManagers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use strum::IntoEnumIterator;

        let entry = |mgr: OsPackageManager| -> ProgramEntry {
            let info = mgr.info();
            match self.path_with_source(mgr) {
                Some((path, source)) => ProgramEntry::installed(info, path, source),
                None => ProgramEntry::not_installed(info),
            }
        };

        let mut state = serializer.serialize_struct("InstalledOsPackageManagers", 9)?;
        for mgr in OsPackageManager::iter() {
            let field_name = match mgr {
                OsPackageManager::Apt => "apt",
                OsPackageManager::Nala => "nala",
                OsPackageManager::Brew => "brew",
                OsPackageManager::Dnf => "dnf",
                OsPackageManager::Pacman => "pacman",
                OsPackageManager::Winget => "winget",
                OsPackageManager::Chocolatey => "chocolatey",
                OsPackageManager::Scoop => "scoop",
                OsPackageManager::Nix => "nix",
            };
            state.serialize_field(field_name, &entry(mgr))?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledOsPackageManagers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoolOsPkgMgrs {
            #[serde(default)]
            apt: bool,
            #[serde(default)]
            nala: bool,
            #[serde(default)]
            brew: bool,
            #[serde(default)]
            dnf: bool,
            #[serde(default)]
            pacman: bool,
            #[serde(default)]
            winget: bool,
            #[serde(default)]
            chocolatey: bool,
            #[serde(default)]
            scoop: bool,
            #[serde(default)]
            nix: bool,
        }

        let b = BoolOsPkgMgrs::deserialize(deserializer)?;

        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledOsPackageManagers {
            apt: to_opt(b.apt),
            nala: to_opt(b.nala),
            brew: to_opt(b.brew),
            dnf: to_opt(b.dnf),
            pacman: to_opt(b.pacman),
            winget: to_opt(b.winget),
            chocolatey: to_opt(b.chocolatey),
            scoop: to_opt(b.scoop),
            nix: to_opt(b.nix),
        })
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

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledOsPackageManagers::path_with_source(self, program)
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

#[cfg(test)]
mod tests {
    use super::*;

    mod lang_pkg_mgrs {
        use super::*;

        #[test]
        fn test_path_with_source_returns_none_when_not_installed() {
            let mgrs = InstalledLanguagePackageManagers::default();
            assert!(mgrs.path_with_source(LanguagePackageManager::Npm).is_none());
        }

        #[test]
        fn test_is_installed_returns_false_for_default() {
            let mgrs = InstalledLanguagePackageManagers::default();
            assert!(!mgrs.is_installed(LanguagePackageManager::Npm));
            assert!(!mgrs.is_installed(LanguagePackageManager::Cargo));
        }

        #[test]
        fn test_serialize_produces_program_entries() {
            let mgrs = InstalledLanguagePackageManagers::default();
            let json = serde_json::to_string(&mgrs).unwrap();
            assert!(json.contains("\"installed\":false"));
            assert!(json.contains("\"npm\":{"));
            assert!(json.contains("\"name\":\"npm\""));
        }

        #[test]
        fn test_deserialize_from_boolean_fields() {
            let json = r#"{"npm": true, "cargo": false}"#;
            let mgrs: InstalledLanguagePackageManagers = serde_json::from_str(json).unwrap();
            assert!(mgrs.is_installed(LanguagePackageManager::Npm));
            assert!(!mgrs.is_installed(LanguagePackageManager::Cargo));
        }

        #[test]
        fn test_serialize_to_json() {
            let original = InstalledLanguagePackageManagers::default();
            let json = serde_json::to_string(&original).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert!(parsed.is_object());
            assert!(parsed.get("npm").is_some());
        }
    }

    mod os_pkg_mgrs {
        use super::*;

        #[test]
        fn test_path_with_source_returns_none_when_not_installed() {
            let mgrs = InstalledOsPackageManagers::default();
            assert!(mgrs.path_with_source(OsPackageManager::Brew).is_none());
        }

        #[test]
        fn test_is_installed_returns_false_for_default() {
            let mgrs = InstalledOsPackageManagers::default();
            assert!(!mgrs.is_installed(OsPackageManager::Brew));
            assert!(!mgrs.is_installed(OsPackageManager::Apt));
        }

        #[test]
        fn test_serialize_produces_program_entries() {
            let mgrs = InstalledOsPackageManagers::default();
            let json = serde_json::to_string(&mgrs).unwrap();
            assert!(json.contains("\"installed\":false"));
            assert!(json.contains("\"brew\":{"));
            assert!(json.contains("\"name\":\"Homebrew\""));
        }

        #[test]
        fn test_deserialize_from_boolean_fields() {
            let json = r#"{"brew": true, "apt": false}"#;
            let mgrs: InstalledOsPackageManagers = serde_json::from_str(json).unwrap();
            assert!(mgrs.is_installed(OsPackageManager::Brew));
            assert!(!mgrs.is_installed(OsPackageManager::Apt));
        }

        #[test]
        fn test_serialize_to_json() {
            let original = InstalledOsPackageManagers::default();
            let json = serde_json::to_string(&original).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert!(parsed.is_object());
            assert!(parsed.get("brew").is_some());
        }
    }
}
