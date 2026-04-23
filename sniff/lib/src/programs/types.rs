//! Type definitions for program detection and installation.
//!
//! This module provides:
//! - `ExecutableSource`: Describes where a program executable was discovered
//! - `ProgramDetector`: Trait for structs that detect and manage installed programs
//! - `InstallationMethod`: Enum describing how to install a program
//! - `CategoryDetector<E>`: Generic detector for any category enum

use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::programs::enums::CategoryEnum;
use crate::programs::find_program::{
    ExecutableIndex, find_programs_with_source_from_index, find_programs_with_source_parallel,
};
use crate::programs::schema::{ProgramError, ProgramMetadata};

/// Describes where a program executable was discovered.
///
/// Distinguishes between traditional PATH-based executables, macOS `.app`
/// bundles, and Windows-specific fallback sources (registry App Paths, shallow
/// install-root walk). Non-PATH sources are "fallback" sources — they are
/// consulted only when PATH lookup misses.
///
/// ## Examples
///
/// ```
/// use sniff::programs::ExecutableSource;
///
/// let source = ExecutableSource::Path;
/// assert!(!source.is_app_bundle());
/// assert!(!source.is_fallback());
///
/// let bundle = ExecutableSource::MacOsAppBundle;
/// assert!(bundle.is_app_bundle());
/// assert!(bundle.is_fallback());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableSource {
    /// Found via PATH lookup (traditional executable).
    Path,
    /// Found as a macOS `.app` bundle.
    MacOsAppBundle,
    /// Found via the Windows `App Paths` registry key
    /// (`HKCU|HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths`).
    WindowsAppPaths,
    /// Found via a shallow walk of a Windows install root
    /// (`%ProgramFiles%`, `%ProgramFiles(x86)%`, `%LocalAppData%\Programs`).
    WindowsInstallRoot,
}

impl ExecutableSource {
    /// Returns `true` if this source is a macOS app bundle.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::programs::ExecutableSource;
    ///
    /// assert!(!ExecutableSource::Path.is_app_bundle());
    /// assert!(ExecutableSource::MacOsAppBundle.is_app_bundle());
    /// ```
    #[must_use]
    pub fn is_app_bundle(&self) -> bool {
        matches!(self, Self::MacOsAppBundle)
    }

    /// Returns `true` if this source is a non-PATH fallback source.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::programs::ExecutableSource;
    ///
    /// assert!(!ExecutableSource::Path.is_fallback());
    /// assert!(ExecutableSource::MacOsAppBundle.is_fallback());
    /// assert!(ExecutableSource::WindowsAppPaths.is_fallback());
    /// assert!(ExecutableSource::WindowsInstallRoot.is_fallback());
    /// ```
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        !matches!(self, Self::Path)
    }
}

impl std::fmt::Display for ExecutableSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutableSource::Path => write!(f, "PATH"),
            ExecutableSource::MacOsAppBundle => write!(f, "macOS App Bundle"),
            ExecutableSource::WindowsAppPaths => write!(f, "Windows App Paths"),
            ExecutableSource::WindowsInstallRoot => write!(f, "Windows Install Root"),
        }
    }
}

/// Describes an installation method for installing some piece of software.
///
/// This installation takes two broad forms:
///
/// 1. Using a package manager (OS level _or_ Language specific)
/// 2. Downloading a bash script and executing it locally
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "manager", content = "target", rename_all = "snake_case")]
pub enum InstallationMethod {
    // Language Package Managers
    /// Default Node.js package manager. [Website](https://www.npmjs.com)
    Npm(&'static str),
    /// Disk-efficient Node.js package manager. [Website](https://pnpm.io)
    Pnpm(&'static str),
    /// Alternative Node.js package manager. [Website](https://yarnpkg.com)
    Yarn(&'static str),
    /// All-in-one JS runtime with built-in package manager. [Website](https://bun.sh)
    Bun(&'static str),
    /// Official Rust package manager and build tool. [Website](https://doc.rust-lang.org/cargo)
    Cargo(&'static str),
    /// Built-in Go dependency system. [Website](https://go.dev/ref/mod)
    GoModules(&'static str),
    /// Dependency manager for modern PHP applications. [Website](https://getcomposer.org)
    Composer(&'static str),
    /// Official Swift dependency manager. [Website](https://www.swift.org/package-manager)
    SwiftPm(&'static str),
    /// Standard package manager for Lua modules. [Website](https://luarocks.org)
    LuaRocks(&'static str),
    /// Cross-platform C/C++ dependency manager. [Website](https://vcpkg.io)
    VcPkg(&'static str),
    /// Decentralized C/C++ package manager. [Website](https://conan.io)
    Conan(&'static str),
    /// Official package manager for .NET and C#. [Website](https://www.nuget.org)
    Nuget(&'static str),
    /// Package manager for the BEAM ecosystem. [Website](https://hex.pm)
    Hex(&'static str),
    /// Traditional Python package installer. [Website](https://pip.pypa.io)
    Pip(&'static str),
    /// High-performance Python package manager. [Website](https://astral.sh/uv)
    Uv(&'static str),
    /// Python dependency manager with lockfile support. [Website](https://python-poetry.org)
    Poetry(&'static str),
    /// Canonical archive and installer for Perl modules. [Website](https://www.cpan.org)
    Cpan(&'static str),
    /// Lightweight, scriptable CPAN client. [Website](https://metacpan.org/pod/App::cpanminus)
    Cpanm(&'static str),

    // OS Package Managers
    /// Debian/Ubuntu primary package manager. [Website](https://tracker.debian.org/pkg/apt)
    Apt(&'static str),
    /// Modern apt frontend with parallel downloads. [Website](https://github.com/volitank/nala)
    Nala(&'static str),
    /// macOS/Linux community package manager. [Website](https://brew.sh)
    Brew(&'static str),
    /// Fedora/RHEL primary package manager. [Website](https://github.com/rpm-software-management/dnf)
    Dnf(&'static str),
    /// Arch Linux package manager. [Website](https://archlinux.org/pacman/)
    Pacman(&'static str),
    /// Windows Package Manager. [Website](https://github.com/microsoft/winget-cli)
    Winget(&'static str),
    /// Windows community package manager. [Website](https://chocolatey.org)
    Chocolatey(&'static str),
    /// Windows command-line installer. [Website](https://scoop.sh)
    Scoop(&'static str),
    /// Nix package manager. [Website](https://nixos.org)
    Nix(&'static str),

    /// Install by downloading a bash script from a URL and then
    /// piping it to the host's `bash` command for installation.
    RemoteBash(&'static str),

    /// Install `pkg` via `uv tool install`, bootstrapping uv from
    /// `astral.sh/uv/install.sh` (or `install.ps1` on Windows) first if
    /// `uv` is not already on `PATH`. Runnable whenever bash (Unix) or
    /// PowerShell (Windows) is available — no Python on the host is
    /// required because the astral installer is self-contained and
    /// `uv tool install` manages Python on its own. Uses the
    /// `RemoteBash` consent flow via the existing `approve_remote_bash`
    /// option; consent is demanded even when the bootstrap step will
    /// be skipped at execute time.
    UvWithInstall(&'static str),
}

impl InstallationMethod {
    /// Returns the package name for this installation method.
    pub fn package_name(&self) -> &'static str {
        match self {
            // Language package managers
            InstallationMethod::Npm(pkg) => pkg,
            InstallationMethod::Pnpm(pkg) => pkg,
            InstallationMethod::Yarn(pkg) => pkg,
            InstallationMethod::Bun(pkg) => pkg,
            InstallationMethod::Cargo(pkg) => pkg,
            InstallationMethod::GoModules(pkg) => pkg,
            InstallationMethod::Composer(pkg) => pkg,
            InstallationMethod::SwiftPm(pkg) => pkg,
            InstallationMethod::LuaRocks(pkg) => pkg,
            InstallationMethod::VcPkg(pkg) => pkg,
            InstallationMethod::Conan(pkg) => pkg,
            InstallationMethod::Nuget(pkg) => pkg,
            InstallationMethod::Hex(pkg) => pkg,
            InstallationMethod::Pip(pkg) => pkg,
            InstallationMethod::Uv(pkg) => pkg,
            InstallationMethod::Poetry(pkg) => pkg,
            InstallationMethod::Cpan(pkg) => pkg,
            InstallationMethod::Cpanm(pkg) => pkg,
            // OS package managers
            InstallationMethod::Apt(pkg) => pkg,
            InstallationMethod::Nala(pkg) => pkg,
            InstallationMethod::Brew(pkg) => pkg,
            InstallationMethod::Dnf(pkg) => pkg,
            InstallationMethod::Pacman(pkg) => pkg,
            InstallationMethod::Winget(pkg) => pkg,
            InstallationMethod::Chocolatey(pkg) => pkg,
            InstallationMethod::Scoop(pkg) => pkg,
            InstallationMethod::Nix(pkg) => pkg,
            // Remote bash
            InstallationMethod::RemoteBash(url) => url,
            // Uv with optional bootstrap
            InstallationMethod::UvWithInstall(pkg) => pkg,
        }
    }

    /// Returns the package manager name for this installation method.
    pub fn manager_name(&self) -> &'static str {
        match self {
            InstallationMethod::Npm(_) => "npm",
            InstallationMethod::Pnpm(_) => "pnpm",
            InstallationMethod::Yarn(_) => "yarn",
            InstallationMethod::Bun(_) => "bun",
            InstallationMethod::Cargo(_) => "cargo",
            InstallationMethod::GoModules(_) => "go",
            InstallationMethod::Composer(_) => "composer",
            InstallationMethod::SwiftPm(_) => "swift",
            InstallationMethod::LuaRocks(_) => "luarocks",
            InstallationMethod::VcPkg(_) => "vcpkg",
            InstallationMethod::Conan(_) => "conan",
            InstallationMethod::Nuget(_) => "nuget",
            InstallationMethod::Hex(_) => "mix",
            InstallationMethod::Pip(_) => "pip",
            InstallationMethod::Uv(_) => "uv",
            InstallationMethod::Poetry(_) => "poetry",
            InstallationMethod::Cpan(_) => "cpan",
            InstallationMethod::Cpanm(_) => "cpanm",
            InstallationMethod::Apt(_) => "apt",
            InstallationMethod::Nala(_) => "nala",
            InstallationMethod::Brew(_) => "brew",
            InstallationMethod::Dnf(_) => "dnf",
            InstallationMethod::Pacman(_) => "pacman",
            InstallationMethod::Winget(_) => "winget",
            InstallationMethod::Chocolatey(_) => "choco",
            InstallationMethod::Scoop(_) => "scoop",
            InstallationMethod::Nix(_) => "nix",
            InstallationMethod::RemoteBash(_) => "bash",
            InstallationMethod::UvWithInstall(_) => "uv",
        }
    }

    /// Returns true if this is an OS-level package manager.
    pub fn is_os_package_manager(&self) -> bool {
        matches!(
            self,
            InstallationMethod::Apt(_)
                | InstallationMethod::Nala(_)
                | InstallationMethod::Brew(_)
                | InstallationMethod::Dnf(_)
                | InstallationMethod::Pacman(_)
                | InstallationMethod::Winget(_)
                | InstallationMethod::Chocolatey(_)
                | InstallationMethod::Scoop(_)
                | InstallationMethod::Nix(_)
        )
    }

    /// Returns true if this is a remote bash installation.
    pub fn is_remote_bash(&self) -> bool {
        matches!(self, InstallationMethod::RemoteBash(_))
    }

    /// Returns the binary name of the package manager executable.
    ///
    /// This is the executable that must be present on the system
    /// to use this installation method.
    pub fn manager_binary(&self) -> &'static str {
        match self {
            InstallationMethod::Npm(_) => "npm",
            InstallationMethod::Pnpm(_) => "pnpm",
            InstallationMethod::Yarn(_) => "yarn",
            InstallationMethod::Bun(_) => "bun",
            InstallationMethod::Cargo(_) => "cargo",
            InstallationMethod::GoModules(_) => "go",
            InstallationMethod::Composer(_) => "composer",
            InstallationMethod::SwiftPm(_) => "swift",
            InstallationMethod::LuaRocks(_) => "luarocks",
            InstallationMethod::VcPkg(_) => "vcpkg",
            InstallationMethod::Conan(_) => "conan",
            InstallationMethod::Nuget(_) => "nuget",
            InstallationMethod::Hex(_) => "mix",
            InstallationMethod::Pip(_) => "pip",
            InstallationMethod::Uv(_) => "uv",
            InstallationMethod::Poetry(_) => "poetry",
            InstallationMethod::Cpan(_) => "cpan",
            InstallationMethod::Cpanm(_) => "cpanm",
            InstallationMethod::Apt(_) => "apt",
            InstallationMethod::Nala(_) => "nala",
            InstallationMethod::Brew(_) => "brew",
            InstallationMethod::Dnf(_) => "dnf",
            InstallationMethod::Pacman(_) => "pacman",
            InstallationMethod::Winget(_) => "winget",
            InstallationMethod::Chocolatey(_) => "choco",
            InstallationMethod::Scoop(_) => "scoop",
            InstallationMethod::Nix(_) => "nix",
            InstallationMethod::RemoteBash(_) => "bash",
            InstallationMethod::UvWithInstall(_) => "uv",
        }
    }
}

/// How to detect whether a `SystemPrerequisite` is already installed on the
/// host. The probe decides whether the prereq's install command needs to run.
///
/// ## Notes
///
/// Windows behavior for `SharedLibrary`: always reports satisfied. On Windows,
/// shared libraries travel with the Python/npm package that consumes them
/// (e.g., the `sounddevice` wheel bundles `portaudio.dll`), so a system-wide
/// probe has no meaningful target. Reporting satisfied silently skips the
/// prereq on Windows, which is correct for every v1 consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrereqProbe {
    /// Shared-library lookup via the dynamic linker search path.
    /// Linux: `ldconfig -p` cache. macOS: dyld default search paths.
    /// Windows: always satisfied (see type-level Notes).
    SharedLibrary(&'static str),
    /// Binary lookup on PATH.
    Binary(&'static str),
}

/// A system-level dependency that must be present before a program's
/// tool-level install runs. Resolved to a single `InstallationMethod` per
/// host using the same bucket logic as `build_install_plan`.
#[derive(Debug, Clone, Copy)]
pub struct SystemPrerequisite {
    /// User-facing name shown in the combined install plan rendering.
    pub name: &'static str,
    /// Presence check used to decide whether installation is needed.
    pub probe: PrereqProbe,
    /// OS-specific install methods. Exactly one wins per host.
    pub methods: &'static [InstallationMethod],
}

/// Generic program detector for any category enum.
///
/// Stores detection results (path + source) indexed by enum variant ordinal.
/// Replaces the per-category `InstalledEditors`, `InstalledUtilities`, etc.
/// structs with a single generic implementation.
#[derive(Debug, Clone)]
pub struct CategoryDetector<E: CategoryEnum> {
    results: Vec<Option<(PathBuf, ExecutableSource)>>,
    _phantom: PhantomData<E>,
}

impl<E: CategoryEnum> Default for CategoryDetector<E> {
    fn default() -> Self {
        Self {
            results: vec![None; E::COUNT],
            _phantom: PhantomData,
        }
    }
}

impl<E: CategoryEnum> PartialEq for CategoryDetector<E> {
    fn eq(&self, other: &Self) -> bool {
        self.results == other.results
    }
}

impl<E: CategoryEnum> Eq for CategoryDetector<E> {}

impl<E: CategoryEnum> Serialize for CategoryDetector<E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(E::COUNT))?;
        for variant in E::iter() {
            let key = variant.serde_key();
            let info = variant.info();
            let entry = match self.path_with_source(variant) {
                Some((path, source)) => {
                    crate::programs::schema::ProgramEntry::installed(info, path, source)
                }
                None => crate::programs::schema::ProgramEntry::not_installed(info),
            };
            map.serialize_entry(key, &entry)?;
        }
        map.end()
    }
}

/// Helper for deserializing both boolean and ProgramEntry values.
#[derive(Deserialize)]
#[serde(untagged)]
enum BoolOrEntry {
    Bool(bool),
    Entry { installed: bool },
}

impl<'de, E: CategoryEnum> Deserialize<'de> for CategoryDetector<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CategoryDetectorVisitor::<E>(PhantomData))
    }
}

struct CategoryDetectorVisitor<E>(PhantomData<E>);

impl<'de, E: CategoryEnum> serde::de::Visitor<'de> for CategoryDetectorVisitor<E> {
    type Value = CategoryDetector<E>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a map of program names to booleans or program entries"
        )
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        // Build key -> variant index lookup
        let key_to_index: HashMap<&'static str, usize> = E::iter()
            .map(|v| (v.serde_key(), v.variant_index()))
            .collect();

        let mut results = vec![None; E::COUNT];

        while let Some(key) = map.next_key::<String>()? {
            if let Some(&idx) = key_to_index.get(key.as_str()) {
                let value: BoolOrEntry = map.next_value()?;
                let installed = match value {
                    BoolOrEntry::Bool(b) => b,
                    BoolOrEntry::Entry { installed } => installed,
                };
                if installed {
                    results[idx] = Some((PathBuf::new(), ExecutableSource::Path));
                }
            } else {
                let _: serde::de::IgnoredAny = map.next_value()?;
            }
        }

        Ok(CategoryDetector {
            results,
            _phantom: PhantomData,
        })
    }
}

impl<E: CategoryEnum> CategoryDetector<E> {
    /// Detect installed programs by scanning PATH.
    pub fn new() -> Self {
        let mut names_to_search: Vec<&'static str> = Vec::new();
        for variant in E::iter() {
            let info = variant.info();
            names_to_search.push(info.binary_name);
            names_to_search.extend_from_slice(info.alternate_binary_names);
        }

        let found = find_programs_with_source_parallel(&names_to_search);
        Self::from_search_results(&found)
    }

    /// Detect installed programs using a pre-built executable index.
    pub fn new_with_index(index: &ExecutableIndex) -> Self {
        let mut names_to_search: Vec<&'static str> = Vec::new();
        for variant in E::iter() {
            let info = variant.info();
            names_to_search.push(info.binary_name);
            names_to_search.extend_from_slice(info.alternate_binary_names);
        }

        let found = find_programs_with_source_from_index(index, &names_to_search);
        Self::from_search_results(&found)
    }

    /// Construct from search results HashMap.
    fn from_search_results(found: &HashMap<String, Option<(PathBuf, ExecutableSource)>>) -> Self {
        let mut results = vec![None; E::COUNT];

        for variant in E::iter() {
            let idx = variant.variant_index();

            // Check platform override first (e.g., Windows SAPI)
            if let Some(override_result) = variant.platform_override() {
                results[idx] = Some(override_result);
                continue;
            }

            // Try primary binary name
            let info = variant.info();
            if let Some(result) = found.get(info.binary_name).and_then(|r| r.clone()) {
                results[idx] = Some(result);
                continue;
            }

            // Try alternate binary names
            for alt in info.alternate_binary_names {
                if let Some(result) = found.get(*alt).and_then(|r| r.clone()) {
                    results[idx] = Some(result);
                    break;
                }
            }
        }

        Self {
            results,
            _phantom: PhantomData,
        }
    }

    /// Re-check program availability and update internal state.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns true if the specified program is installed.
    pub fn is_installed(&self, program: E) -> bool {
        self.results[program.variant_index()].is_some()
    }

    /// Returns the path to the specified program's binary if installed.
    pub fn path(&self, program: E) -> Option<PathBuf> {
        self.results[program.variant_index()]
            .as_ref()
            .map(|(p, _)| p.clone())
    }

    /// Returns the path and source of the specified program if installed.
    pub fn path_with_source(&self, program: E) -> Option<(PathBuf, ExecutableSource)> {
        self.results[program.variant_index()].clone()
    }

    /// Returns the version of the specified program if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if the program is not installed or version detection fails.
    pub fn version(&self, program: E) -> Result<String, ProgramError> {
        if !self.is_installed(program) {
            return Err(ProgramError::NotFound(program.binary_name().to_string()));
        }
        program.version()
    }

    /// Returns the official website URL for the specified program.
    pub fn website(&self, program: E) -> &'static str {
        program.website()
    }

    /// Returns a one-line description of the specified program.
    pub fn description(&self, program: E) -> &'static str {
        program.description()
    }

    /// Returns a list of all installed programs in this category.
    pub fn installed(&self) -> Vec<E> {
        E::iter().filter(|p| self.is_installed(*p)).collect()
    }

    /// Builder method: mark a program as installed with given path and source.
    ///
    /// Useful for testing.
    pub fn with_program(mut self, program: E, path: PathBuf, source: ExecutableSource) -> Self {
        self.results[program.variant_index()] = Some((path, source));
        self
    }
}

impl<E: CategoryEnum> ProgramDetector for CategoryDetector<E> {
    type Program = E;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: E) -> bool {
        CategoryDetector::is_installed(self, program)
    }

    fn path(&self, program: E) -> Option<PathBuf> {
        CategoryDetector::path(self, program)
    }

    fn path_with_source(&self, program: E) -> Option<(PathBuf, ExecutableSource)> {
        CategoryDetector::path_with_source(self, program)
    }

    fn version(&self, program: E) -> Result<String, crate::programs::ProgramError> {
        CategoryDetector::version(self, program)
    }

    fn website(&self, program: E) -> &'static str {
        CategoryDetector::website(self, program)
    }

    fn description(&self, program: E) -> &'static str {
        CategoryDetector::description(self, program)
    }

    fn installed(&self) -> Vec<E> {
        CategoryDetector::installed(self)
    }

    fn installable(&self, program: E) -> bool {
        self.install_plan(program).successful
    }

    fn install(&self, program: E) -> Result<(), SniffInstallationError> {
        let plan = self.install_plan(program);
        if !plan.successful {
            return Err(SniffInstallationError::NoViableMethod {
                pkg: program.display_name().to_string(),
                detail: format!(
                    "evaluated {} method(s); none are runnable",
                    plan.options.len()
                ),
            });
        }
        let _ = plan.execute(&crate::programs::installer::InstallOptions::default())?;
        Ok(())
    }

    fn install_version(&self, program: E, version: &str) -> Result<(), SniffInstallationError> {
        let plan = self.install_plan(program);
        let chosen = plan
            .chosen()
            .ok_or_else(|| SniffInstallationError::NoViableMethod {
                pkg: program.display_name().to_string(),
                detail: format!(
                    "evaluated {} method(s); none are runnable",
                    plan.options.len()
                ),
            })?;

        if matches!(
            chosen.kind,
            InstallationMethod::RemoteBash(_) | InstallationMethod::UvWithInstall(_)
        ) {
            let url = match &chosen.kind {
                InstallationMethod::RemoteBash(u) => (*u).to_string(),
                InstallationMethod::UvWithInstall(_) => {
                    crate::programs::installer::astral_installer_url().to_string()
                }
                _ => unreachable!(),
            };
            return Err(SniffInstallationError::RemoteBashConsentRequired {
                pkg: program.display_name().to_string(),
                url,
            });
        }

        let _ = crate::programs::installer::execute_versioned_install(
            &chosen.kind,
            version,
            &crate::programs::installer::InstallOptions::default(),
        )?;
        Ok(())
    }
}

/// Trait for structs that detect and manage programs of a specific category.
///
/// Implementors track installation status for a set of related programs
/// (e.g., editors, utilities, TTS clients) and provide methods to query
/// metadata, check installation status, and install programs.
///
/// ## Associated Type
///
/// The `Program` associated type specifies the enum type representing
/// the programs in this category. It must implement `ProgramMetadata`
/// for metadata access and `Copy` for efficient parameter passing.
///
/// ## Examples
///
/// ```ignore
/// use sniff::programs::{ProgramDetector, InstalledEditors, Editor};
///
/// let editors = InstalledEditors::new();
/// if editors.is_installed(Editor::Vim) {
///     println!("Vim is installed at {:?}", editors.path(Editor::Vim));
/// }
/// ```
pub trait ProgramDetector {
    /// The enum type representing programs in this category.
    type Program: ProgramMetadata + Copy;

    /// Re-check program availability and update internal state.
    fn refresh(&mut self);

    /// Returns true if the specified program is installed.
    fn is_installed(&self, program: Self::Program) -> bool;

    /// Returns the path to the specified program's binary if installed.
    fn path(&self, program: Self::Program) -> Option<PathBuf>;

    /// Returns the path and source of the specified program's binary if installed.
    ///
    /// This extends `path()` by also reporting how the executable was discovered:
    /// - `ExecutableSource::Path` for traditional PATH-based executables
    /// - `ExecutableSource::MacOsAppBundle` for macOS app bundles
    ///
    /// The default implementation wraps `path()` and assumes `ExecutableSource::Path`.
    /// Implementors can override this to provide more accurate source information.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use sniff::programs::{ProgramDetector, InstalledEditors, Editor, ExecutableSource};
    ///
    /// let editors = InstalledEditors::new();
    /// if let Some((path, source)) = editors.path_with_source(Editor::Vscode) {
    ///     match source {
    ///         ExecutableSource::Path => println!("Found in PATH: {}", path.display()),
    ///         ExecutableSource::MacOsAppBundle => println!("Found as macOS app: {}", path.display()),
    ///     }
    /// }
    /// ```
    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        self.path(program).map(|p| (p, ExecutableSource::Path))
    }

    /// Returns the version of the specified program if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The program is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    fn version(&self, program: Self::Program) -> Result<String, crate::programs::ProgramError>;

    /// Returns the official website URL for the specified program.
    fn website(&self, program: Self::Program) -> &'static str;

    /// Returns a one-line description of the specified program.
    fn description(&self, program: Self::Program) -> &'static str;

    /// Returns the description formatted for terminal display.
    ///
    /// The description uses OSC8 hyperlinks for clickable URLs and
    /// ANSI escape codes for styling (bold/blue for the program name).
    ///
    /// ## Notes
    ///
    /// Assumes the terminal supports OSC8 and ANSI color codes.
    fn description_for_terminal(&self, program: Self::Program) -> String {
        let info = program.info();
        let name = info.display_name;
        let url = info.website;
        let desc = info.description;

        // OSC8 hyperlink: \x1b]8;;URL\x07TEXT\x1b]8;;\x07
        // Bold blue: \x1b[1;34mTEXT\x1b[0m
        format!(
            "\x1b]8;;{url}\x07\x1b[1;34m{name}\x1b[0m\x1b]8;;\x07 - {desc}",
            url = url,
            name = name,
            desc = desc
        )
    }

    /// Returns a list of all installed programs in this category.
    fn installed(&self) -> Vec<Self::Program>;

    /// Returns true if the specified program can be installed on this system.
    ///
    /// Checks:
    /// - OS compatibility
    /// - Available package managers
    /// - Defined installation methods
    fn installable(&self, program: Self::Program) -> bool;

    /// Attempts to install the program onto the host.
    ///
    /// ## Notes
    ///
    /// - Language package managers install globally (to PATH)
    /// - OS package managers are preferred over language package managers
    /// - Package managers are preferred over bash script installations
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - No installation method is available for this OS
    /// - The required package manager is not installed
    /// - The installation command fails
    fn install(&self, program: Self::Program) -> Result<(), SniffInstallationError>;

    /// Attempts to install a specific version of the program.
    ///
    /// ## Notes
    ///
    /// Remote bash installations do NOT support versioned installs and will
    /// return an error.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - Version-specific installation is not supported
    /// - The installation fails
    fn install_version(
        &self,
        program: Self::Program,
        version: &str,
    ) -> Result<(), SniffInstallationError>;

    /// Returns every installation method the program declares, ignoring host
    /// constraints. This is the static metadata.
    fn known_methods(&self, program: Self::Program) -> &'static [InstallationMethod] {
        program.info().installation_methods
    }

    /// Returns the subset of known methods whose required package manager is
    /// actually installed on this host and whose program is permitted on the
    /// current OS.
    fn available_methods(&self, program: Self::Program) -> Vec<InstallationMethod> {
        use crate::programs::host_capability::HostCapabilities;
        use crate::programs::installer::method_available;

        let info = program.info();
        let host = HostCapabilities::load_or_detect();

        let os_ok = info.os_availability.is_empty() || info.os_availability.contains(&host.os_type);
        if !os_ok {
            return Vec::new();
        }

        info.installation_methods
            .iter()
            .filter(|m| method_available(m, &host))
            .cloned()
            .collect()
    }

    /// Returns a full install plan for this program against cached host
    /// capabilities.
    fn install_plan(&self, program: Self::Program) -> crate::programs::install_plan::InstallPlan {
        use crate::programs::host_capability::HostCapabilities;
        use crate::programs::install_plan::build_install_plan;

        let host = HostCapabilities::load_or_detect();
        build_install_plan(&program, &host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::schema::{ProgramError, ProgramInfo, ProgramMetadata};

    // ============================================
    // Mock implementation for testing ProgramDetector trait
    // ============================================

    static MOCK_INSTALLED_INFO: ProgramInfo = ProgramInfo::standard(
        "mock-installed",
        "Mock Installed",
        "A mock installed program",
        "https://example.com/installed",
    );

    static MOCK_NOT_INSTALLED_INFO: ProgramInfo = ProgramInfo::standard(
        "mock-not-installed",
        "Mock Not Installed",
        "A mock not-installed program",
        "https://example.com/not-installed",
    );

    /// Mock program enum for testing the ProgramDetector trait.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MockProgram {
        Installed,
        NotInstalled,
    }

    impl ProgramMetadata for MockProgram {
        fn info(&self) -> &'static ProgramInfo {
            match self {
                MockProgram::Installed => &MOCK_INSTALLED_INFO,
                MockProgram::NotInstalled => &MOCK_NOT_INSTALLED_INFO,
            }
        }
    }

    /// Mock detector that implements ProgramDetector for testing default methods.
    struct MockDetector;

    impl ProgramDetector for MockDetector {
        type Program = MockProgram;

        fn refresh(&mut self) {}

        fn is_installed(&self, program: Self::Program) -> bool {
            matches!(program, MockProgram::Installed)
        }

        fn path(&self, program: Self::Program) -> Option<PathBuf> {
            match program {
                MockProgram::Installed => Some(PathBuf::from("/usr/bin/mock-installed")),
                MockProgram::NotInstalled => None,
            }
        }

        fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
            match program {
                MockProgram::Installed => Ok("1.0.0".to_string()),
                MockProgram::NotInstalled => {
                    Err(ProgramError::NotFound("mock-not-installed".to_string()))
                }
            }
        }

        fn website(&self, program: Self::Program) -> &'static str {
            program.info().website
        }

        fn description(&self, program: Self::Program) -> &'static str {
            program.info().description
        }

        fn installed(&self) -> Vec<Self::Program> {
            vec![MockProgram::Installed]
        }

        fn installable(&self, _program: Self::Program) -> bool {
            false
        }

        fn install(&self, _program: Self::Program) -> Result<(), SniffInstallationError> {
            Err(SniffInstallationError::NotInstallableOnOs {
                pkg: "mock".to_string(),
                os: "mock".to_string(),
            })
        }

        fn install_version(
            &self,
            _program: Self::Program,
            _version: &str,
        ) -> Result<(), SniffInstallationError> {
            Err(SniffInstallationError::NotInstallableOnOs {
                pkg: "mock".to_string(),
                os: "mock".to_string(),
            })
        }
    }

    // ============================================
    // ProgramDetector::path_with_source tests
    // ============================================

    #[test]
    fn test_path_with_source_default_returns_path_source_when_installed() {
        let detector = MockDetector;
        let result = detector.path_with_source(MockProgram::Installed);

        assert!(result.is_some());
        let (path, source) = result.unwrap();
        assert_eq!(path, PathBuf::from("/usr/bin/mock-installed"));
        assert_eq!(source, ExecutableSource::Path);
    }

    #[test]
    fn test_path_with_source_default_returns_none_when_not_installed() {
        let detector = MockDetector;
        let result = detector.path_with_source(MockProgram::NotInstalled);

        assert!(result.is_none());
    }

    // ============================================
    // ExecutableSource tests
    // ============================================

    #[test]
    fn test_executable_source_is_app_bundle() {
        assert!(!ExecutableSource::Path.is_app_bundle());
        assert!(ExecutableSource::MacOsAppBundle.is_app_bundle());
    }

    #[test]
    fn test_executable_source_display() {
        assert_eq!(ExecutableSource::Path.to_string(), "PATH");
        assert_eq!(
            ExecutableSource::MacOsAppBundle.to_string(),
            "macOS App Bundle"
        );
    }

    #[test]
    fn test_executable_source_debug() {
        assert_eq!(format!("{:?}", ExecutableSource::Path), "Path");
        assert_eq!(
            format!("{:?}", ExecutableSource::MacOsAppBundle),
            "MacOsAppBundle"
        );
    }

    #[test]
    fn test_executable_source_clone_and_copy() {
        let source = ExecutableSource::Path;
        let cloned = source;
        let copied = source; // Copy
        assert_eq!(source, cloned);
        assert_eq!(source, copied);
    }

    #[test]
    fn test_executable_source_equality() {
        assert_eq!(ExecutableSource::Path, ExecutableSource::Path);
        assert_eq!(
            ExecutableSource::MacOsAppBundle,
            ExecutableSource::MacOsAppBundle
        );
        assert_ne!(ExecutableSource::Path, ExecutableSource::MacOsAppBundle);
    }

    #[test]
    fn test_executable_source_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ExecutableSource::Path);
        set.insert(ExecutableSource::MacOsAppBundle);
        set.insert(ExecutableSource::WindowsAppPaths);
        set.insert(ExecutableSource::WindowsInstallRoot);
        set.insert(ExecutableSource::Path); // Duplicate

        assert_eq!(set.len(), 4);
        assert!(set.contains(&ExecutableSource::Path));
        assert!(set.contains(&ExecutableSource::MacOsAppBundle));
        assert!(set.contains(&ExecutableSource::WindowsAppPaths));
        assert!(set.contains(&ExecutableSource::WindowsInstallRoot));
    }

    #[test]
    fn test_executable_source_serialize_json() {
        let path = ExecutableSource::Path;
        let bundle = ExecutableSource::MacOsAppBundle;

        let path_json = serde_json::to_string(&path).unwrap();
        let bundle_json = serde_json::to_string(&bundle).unwrap();

        assert_eq!(path_json, "\"path\"");
        assert_eq!(bundle_json, "\"mac_os_app_bundle\"");
    }

    #[test]
    fn test_executable_source_deserialize_json() {
        let path: ExecutableSource = serde_json::from_str("\"path\"").unwrap();
        let bundle: ExecutableSource = serde_json::from_str("\"mac_os_app_bundle\"").unwrap();

        assert_eq!(path, ExecutableSource::Path);
        assert_eq!(bundle, ExecutableSource::MacOsAppBundle);
    }

    #[test]
    fn test_executable_source_roundtrip() {
        for source in [
            ExecutableSource::Path,
            ExecutableSource::MacOsAppBundle,
            ExecutableSource::WindowsAppPaths,
            ExecutableSource::WindowsInstallRoot,
        ] {
            let json = serde_json::to_string(&source).unwrap();
            let deserialized: ExecutableSource = serde_json::from_str(&json).unwrap();
            assert_eq!(source, deserialized);
        }
    }

    #[test]
    fn test_executable_source_windows_variants_serialize() {
        let app_paths = ExecutableSource::WindowsAppPaths;
        let install_root = ExecutableSource::WindowsInstallRoot;

        assert_eq!(
            serde_json::to_string(&app_paths).unwrap(),
            "\"windows_app_paths\""
        );
        assert_eq!(
            serde_json::to_string(&install_root).unwrap(),
            "\"windows_install_root\""
        );
    }

    #[test]
    fn test_executable_source_windows_variants_deserialize() {
        let ap: ExecutableSource = serde_json::from_str("\"windows_app_paths\"").unwrap();
        let ir: ExecutableSource = serde_json::from_str("\"windows_install_root\"").unwrap();

        assert_eq!(ap, ExecutableSource::WindowsAppPaths);
        assert_eq!(ir, ExecutableSource::WindowsInstallRoot);
    }

    #[test]
    fn test_executable_source_windows_variants_display() {
        assert_eq!(
            ExecutableSource::WindowsAppPaths.to_string(),
            "Windows App Paths"
        );
        assert_eq!(
            ExecutableSource::WindowsInstallRoot.to_string(),
            "Windows Install Root"
        );
    }

    #[test]
    fn test_executable_source_windows_variants_not_app_bundle() {
        assert!(!ExecutableSource::WindowsAppPaths.is_app_bundle());
        assert!(!ExecutableSource::WindowsInstallRoot.is_app_bundle());
    }

    #[test]
    fn test_executable_source_is_fallback() {
        assert!(!ExecutableSource::Path.is_fallback());
        assert!(ExecutableSource::MacOsAppBundle.is_fallback());
        assert!(ExecutableSource::WindowsAppPaths.is_fallback());
        assert!(ExecutableSource::WindowsInstallRoot.is_fallback());
    }

    // ============================================
    // InstallationMethod tests
    // ============================================

    #[test]
    fn test_installation_method_package_name() {
        assert_eq!(
            InstallationMethod::Brew("ripgrep").package_name(),
            "ripgrep"
        );
        assert_eq!(InstallationMethod::Cargo("bat").package_name(), "bat");
        assert_eq!(
            InstallationMethod::RemoteBash("https://example.com/install.sh").package_name(),
            "https://example.com/install.sh"
        );
    }

    #[test]
    fn test_installation_method_manager_name() {
        assert_eq!(InstallationMethod::Brew("ripgrep").manager_name(), "brew");
        assert_eq!(InstallationMethod::Cargo("bat").manager_name(), "cargo");
        assert_eq!(InstallationMethod::Npm("typescript").manager_name(), "npm");
    }

    #[test]
    fn test_installation_method_manager_binary() {
        assert_eq!(InstallationMethod::Brew("vim").manager_binary(), "brew");
        assert_eq!(InstallationMethod::Apt("vim").manager_binary(), "apt");
        assert_eq!(
            InstallationMethod::Cargo("ripgrep").manager_binary(),
            "cargo"
        );
        assert_eq!(
            InstallationMethod::Npm("typescript").manager_binary(),
            "npm"
        );
        assert_eq!(
            InstallationMethod::RemoteBash("url").manager_binary(),
            "bash"
        );
        assert_eq!(
            InstallationMethod::Chocolatey("vim").manager_binary(),
            "choco"
        );
        assert_eq!(
            InstallationMethod::Hex("hex_package").manager_binary(),
            "mix"
        );
    }

    #[test]
    fn test_installation_method_is_os_package_manager() {
        assert!(InstallationMethod::Brew("ripgrep").is_os_package_manager());
        assert!(InstallationMethod::Apt("ripgrep").is_os_package_manager());
        assert!(!InstallationMethod::Cargo("bat").is_os_package_manager());
        assert!(!InstallationMethod::Npm("typescript").is_os_package_manager());
    }

    // ============================================
    // InstallationMethod comprehensive tests
    // ============================================

    #[test]
    fn test_installation_method_is_remote_bash() {
        assert!(InstallationMethod::RemoteBash("https://example.com/install.sh").is_remote_bash());
        assert!(!InstallationMethod::Brew("ripgrep").is_remote_bash());
        assert!(!InstallationMethod::Cargo("bat").is_remote_bash());
    }

    #[test]
    fn test_installation_method_all_language_managers() {
        let methods = [
            InstallationMethod::Npm("pkg"),
            InstallationMethod::Pnpm("pkg"),
            InstallationMethod::Yarn("pkg"),
            InstallationMethod::Bun("pkg"),
            InstallationMethod::Cargo("pkg"),
            InstallationMethod::GoModules("pkg"),
            InstallationMethod::Composer("pkg"),
            InstallationMethod::SwiftPm("pkg"),
            InstallationMethod::LuaRocks("pkg"),
            InstallationMethod::VcPkg("pkg"),
            InstallationMethod::Conan("pkg"),
            InstallationMethod::Nuget("pkg"),
            InstallationMethod::Hex("pkg"),
            InstallationMethod::Pip("pkg"),
            InstallationMethod::Uv("pkg"),
            InstallationMethod::Poetry("pkg"),
            InstallationMethod::Cpan("pkg"),
            InstallationMethod::Cpanm("pkg"),
        ];

        for method in &methods {
            assert!(
                !method.is_os_package_manager(),
                "{:?} should not be OS pkg mgr",
                method
            );
            assert!(
                !method.is_remote_bash(),
                "{:?} should not be remote bash",
                method
            );
            assert_eq!(method.package_name(), "pkg");
        }
    }

    #[test]
    fn test_installation_method_all_os_managers() {
        let methods = [
            InstallationMethod::Apt("pkg"),
            InstallationMethod::Nala("pkg"),
            InstallationMethod::Brew("pkg"),
            InstallationMethod::Dnf("pkg"),
            InstallationMethod::Pacman("pkg"),
            InstallationMethod::Winget("pkg"),
            InstallationMethod::Chocolatey("pkg"),
            InstallationMethod::Scoop("pkg"),
            InstallationMethod::Nix("pkg"),
        ];

        for method in &methods {
            assert!(
                method.is_os_package_manager(),
                "{:?} should be OS pkg mgr",
                method
            );
            assert!(
                !method.is_remote_bash(),
                "{:?} should not be remote bash",
                method
            );
            assert_eq!(method.package_name(), "pkg");
        }
    }

    #[test]
    fn test_installation_method_manager_name_all_variants() {
        // Test all manager names are non-empty strings
        let all_methods = [
            InstallationMethod::Npm("x"),
            InstallationMethod::Pnpm("x"),
            InstallationMethod::Yarn("x"),
            InstallationMethod::Bun("x"),
            InstallationMethod::Cargo("x"),
            InstallationMethod::GoModules("x"),
            InstallationMethod::Composer("x"),
            InstallationMethod::SwiftPm("x"),
            InstallationMethod::LuaRocks("x"),
            InstallationMethod::VcPkg("x"),
            InstallationMethod::Conan("x"),
            InstallationMethod::Nuget("x"),
            InstallationMethod::Hex("x"),
            InstallationMethod::Pip("x"),
            InstallationMethod::Uv("x"),
            InstallationMethod::Poetry("x"),
            InstallationMethod::Cpan("x"),
            InstallationMethod::Cpanm("x"),
            InstallationMethod::Apt("x"),
            InstallationMethod::Nala("x"),
            InstallationMethod::Brew("x"),
            InstallationMethod::Dnf("x"),
            InstallationMethod::Pacman("x"),
            InstallationMethod::Winget("x"),
            InstallationMethod::Chocolatey("x"),
            InstallationMethod::Scoop("x"),
            InstallationMethod::Nix("x"),
            InstallationMethod::RemoteBash("x"),
            InstallationMethod::UvWithInstall("x"),
        ];

        for method in &all_methods {
            let name = method.manager_name();
            assert!(
                !name.is_empty(),
                "{:?} should have non-empty manager name",
                method
            );
        }
    }

    #[test]
    fn test_uv_with_install_package_name_and_manager() {
        let method = InstallationMethod::UvWithInstall("aider-chat");
        assert_eq!(method.package_name(), "aider-chat");
        assert_eq!(method.manager_name(), "uv");
        assert_eq!(method.manager_binary(), "uv");
        assert!(!method.is_os_package_manager());
        assert!(!method.is_remote_bash());
    }

    // ============================================
    // ExecutableSource additional tests
    // ============================================

    #[test]
    fn test_executable_source_default_is_not_app_bundle() {
        // Verify that Path (the most common case) is not an app bundle
        let source = ExecutableSource::Path;
        assert!(!source.is_app_bundle());
    }

    #[test]
    fn test_executable_source_pattern_matching() {
        fn describe_source(source: ExecutableSource) -> &'static str {
            match source {
                ExecutableSource::Path => "path",
                ExecutableSource::MacOsAppBundle => "bundle",
                ExecutableSource::WindowsAppPaths => "app_paths",
                ExecutableSource::WindowsInstallRoot => "install_root",
            }
        }

        assert_eq!(describe_source(ExecutableSource::Path), "path");
        assert_eq!(describe_source(ExecutableSource::MacOsAppBundle), "bundle");
        assert_eq!(
            describe_source(ExecutableSource::WindowsAppPaths),
            "app_paths"
        );
        assert_eq!(
            describe_source(ExecutableSource::WindowsInstallRoot),
            "install_root"
        );
    }

    #[test]
    fn test_executable_source_deserialize_invalid_json() {
        // Invalid JSON value should fail to deserialize
        let result: Result<ExecutableSource, _> = serde_json::from_str("\"invalid_source\"");
        assert!(result.is_err(), "Invalid source should fail to deserialize");
    }

    // ============================================
    // CategoryDetector tests
    // ============================================

    use crate::programs::enums::Editor;

    #[test]
    fn test_category_detector_default_has_nothing_installed() {
        let detector = CategoryDetector::<Editor>::default();
        assert!(detector.installed().is_empty());
        assert!(!detector.is_installed(Editor::Vim));
        assert!(detector.path(Editor::Vim).is_none());
        assert!(detector.path_with_source(Editor::Vim).is_none());
    }

    #[test]
    fn test_category_detector_with_program_marks_installed() {
        let detector = CategoryDetector::<Editor>::default().with_program(
            Editor::Vim,
            PathBuf::from("/usr/bin/vim"),
            ExecutableSource::Path,
        );
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::Neovim));
        assert_eq!(
            detector.path(Editor::Vim),
            Some(PathBuf::from("/usr/bin/vim"))
        );
    }

    #[test]
    fn test_category_detector_installed_returns_only_installed() {
        let detector = CategoryDetector::<Editor>::default()
            .with_program(
                Editor::Vim,
                PathBuf::from("/usr/bin/vim"),
                ExecutableSource::Path,
            )
            .with_program(
                Editor::Neovim,
                PathBuf::from("/usr/bin/nvim"),
                ExecutableSource::Path,
            );
        let installed = detector.installed();
        assert_eq!(installed.len(), 2);
        assert!(installed.contains(&Editor::Vim));
        assert!(installed.contains(&Editor::Neovim));
    }

    // ============================================
    // CategoryDetector Serialization/Deserialization tests
    // ============================================

    #[test]
    fn test_category_detector_serialize_produces_program_entries() {
        let detector = CategoryDetector::<Editor>::default();
        let json = serde_json::to_string(&detector).unwrap();
        // Should produce ProgramEntry objects with full metadata
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"vim\":{"));
        assert!(json.contains("\"name\":\"Vim\""));
    }

    #[test]
    fn test_category_detector_deserialize_from_booleans() {
        let json = r#"{"vim": true, "vscode": false}"#;
        let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::VSCode));
    }

    #[test]
    fn test_category_detector_deserialize_partial_json() {
        let json = r#"{"vim": true}"#;
        let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::Neovim));
    }

    #[test]
    fn test_category_detector_serialize_includes_path_and_source() {
        let detector = CategoryDetector::<Editor>::default().with_program(
            Editor::Vim,
            PathBuf::from("/usr/bin/vim"),
            ExecutableSource::Path,
        );
        let json = serde_json::to_string(&detector).unwrap();
        // Vim should be installed
        assert!(json.contains("\"vim\":{"));
        assert!(json.contains("\"installed\":true"));
        assert!(json.contains("\"path\":\"/usr/bin/vim\""));
        assert!(json.contains("\"source\":\"path\""));
        // VSCode should not be installed
        assert!(json.contains("\"vscode\":{"));
        // Should contain at least one installed:false
        assert!(json.contains("\"installed\":false"));
    }

    #[test]
    fn test_category_detector_roundtrip_serialization() {
        let detector1 = CategoryDetector::<Editor>::default()
            .with_program(
                Editor::Vim,
                PathBuf::from("/usr/bin/vim"),
                ExecutableSource::Path,
            )
            .with_program(
                Editor::Neovim,
                PathBuf::from("/usr/bin/nvim"),
                ExecutableSource::Path,
            );

        let json = serde_json::to_string(&detector1).unwrap();
        let detector2: CategoryDetector<Editor> = serde_json::from_str(&json).unwrap();

        assert_eq!(
            detector1.is_installed(Editor::Vim),
            detector2.is_installed(Editor::Vim)
        );
        assert_eq!(
            detector1.is_installed(Editor::Neovim),
            detector2.is_installed(Editor::Neovim)
        );
        assert_eq!(
            detector1.is_installed(Editor::VSCode),
            detector2.is_installed(Editor::VSCode)
        );
    }

    #[test]
    fn test_category_detector_deserialize_from_program_entries() {
        let json = r#"{
            "vim": {
                "installed": true,
                "name": "Vim",
                "description": "Classic modal text editor",
                "website": "https://www.vim.org",
                "path": "/usr/bin/vim",
                "source": "path"
            },
            "vscode": {
                "installed": false,
                "name": "Visual Studio Code",
                "description": "Modern code editor by Microsoft",
                "website": "https://code.visualstudio.com"
            }
        }"#;
        let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::VSCode));
    }

    // ============================================
    // InstallationMethod Serialize tests
    // ============================================

    #[test]
    fn test_installation_method_serializes_with_manager_target_shape() {
        let method = InstallationMethod::Brew("ripgrep");
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#"{"manager":"brew","target":"ripgrep"}"#);
    }

    #[test]
    fn test_installation_method_serializes_remote_bash_as_tagged_shape() {
        let method = InstallationMethod::RemoteBash("https://sh.rustup.rs");
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(
            json,
            r#"{"manager":"remote_bash","target":"https://sh.rustup.rs"}"#
        );
    }

    #[test]
    fn test_installation_method_serializes_cargo_as_tagged_shape() {
        let method = InstallationMethod::Cargo("bat");
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#"{"manager":"cargo","target":"bat"}"#);
    }

    // ============================================
    // CategoryDetector ProgramDetector trait tests
    // ============================================

    #[test]
    fn test_category_detector_program_detector_trait() {
        let detector = CategoryDetector::<Editor>::default().with_program(
            Editor::Vim,
            PathBuf::from("/usr/bin/vim"),
            ExecutableSource::Path,
        );

        // Test through ProgramDetector trait interface
        let pd: &dyn ProgramDetector<Program = Editor> = &detector;
        assert!(pd.is_installed(Editor::Vim));
        assert!(!pd.is_installed(Editor::Neovim));
        assert_eq!(pd.path(Editor::Vim), Some(PathBuf::from("/usr/bin/vim")));
        let installed = pd.installed();
        assert_eq!(installed, vec![Editor::Vim]);
    }

    #[test]
    fn category_detector_known_methods_matches_metadata() {
        let detector = CategoryDetector::<Editor>::default();
        let methods = detector.known_methods(Editor::Vim);
        assert_eq!(methods, Editor::Vim.info().installation_methods);
    }

    #[test]
    fn category_detector_available_methods_filters_by_os() {
        // On the current host, VSCode's methods should produce a deterministic
        // subset — we just assert the call compiles and returns a Vec.
        let detector = CategoryDetector::<Editor>::default();
        let _available = detector.available_methods(Editor::VSCode);
    }

    #[test]
    fn category_detector_install_plan_returns_plan_for_program() {
        let detector = CategoryDetector::<Editor>::default();
        let plan = detector.install_plan(Editor::Vim);
        assert_eq!(plan.program, Editor::Vim.display_name());
    }

    #[test]
    fn installable_mirrors_plan_successful() {
        use strum::IntoEnumIterator;
        let detector = CategoryDetector::<Editor>::default();
        for editor in Editor::iter() {
            let plan = detector.install_plan(editor);
            assert_eq!(
                detector.installable(editor),
                plan.successful,
                "installable() must mirror install_plan().successful for {:?}",
                editor
            );
        }
    }
}

#[cfg(test)]
mod prereq_type_tests {
    use super::*;

    #[test]
    fn system_prerequisite_is_constructible_as_const() {
        const PREREQ: SystemPrerequisite = SystemPrerequisite {
            name: "PortAudio",
            probe: PrereqProbe::SharedLibrary("libportaudio.so.2"),
            methods: &[InstallationMethod::Apt("libportaudio2")],
        };
        assert_eq!(PREREQ.name, "PortAudio");
        assert!(matches!(PREREQ.probe, PrereqProbe::SharedLibrary(_)));
    }

    #[test]
    fn prereq_probe_binary_variant() {
        const PROBE: PrereqProbe = PrereqProbe::Binary("ffmpeg");
        assert!(matches!(PROBE, PrereqProbe::Binary("ffmpeg")));
    }
}
