//! Core contract types for program detection.
//!
//! This module provides the foundational types and traits used across the
//! programs module. It is intentionally a leaf module to avoid circular
//! dependencies between `types`, `schema`, and `enums`.
//!
//! ## Types
//!
//! - `ExecutableSource` — Describes how a program executable was discovered
//! - `InstallationMethod` — Describes how to install a program
//! - `SystemPrerequisite` — A system-level dependency required before installation
//! - `ProgramError` — Errors during program version detection
//! - `CategoryEnum` — Trait bridging category enums to generic detectors

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    /// Found in a project-local bin directory (e.g. `node_modules/.bin`).
    ProjectLocal,
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
            ExecutableSource::ProjectLocal => write!(f, "Project Local"),
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
    /// Package manager for the Java ecosystem. [Website](https://maven.apache.org)
    Maven(&'static str),
    /// Package manager for the Ruby ecosystem. [Website](https://rubygems.org)
    Gem(&'static str),
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
            InstallationMethod::Maven(pkg) => pkg,
            InstallationMethod::Gem(pkg) => pkg,
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
            InstallationMethod::Maven(_) => "maven",
            InstallationMethod::Gem(_) => "gem",
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
            InstallationMethod::Maven(_) => "mvn",
            InstallationMethod::Gem(_) => "gem",
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

/// Errors that can occur during program version detection.
#[derive(Debug, Error)]
pub enum ProgramError {
    /// The program was not found in PATH.
    #[error("program '{0}' not found in PATH")]
    NotFound(String),

    /// Failed to execute the version command.
    #[error("failed to execute version command for '{program}': {source}")]
    ExecutionFailed {
        program: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse version output.
    #[error("failed to parse version output for '{program}': {details}")]
    ParseFailed { program: String, details: String },

    /// The version command returned non-zero exit code.
    #[error("version command for '{program}' failed with exit code {code}")]
    NonZeroExit { program: String, code: i32 },
}

/// Trait bridging category enums to the generic `CategoryDetector<E>`.
///
/// Implementors provide category-level metadata and variant indexing
/// that enables a single generic detector struct to work across all
/// program categories.
///
/// Note: this trait intentionally does not extend `ProgramMetadata`. Callers
/// that need both should use `E: CategoryEnum + ProgramMetadata`.
pub trait CategoryEnum:
    strum::IntoEnumIterator
    + strum::EnumCount
    + Copy
    + Clone
    + Eq
    + std::hash::Hash
    + fmt::Debug
    + fmt::Display
    + Send
    + Sync
    + 'static
{
    /// Human-readable category name (e.g., "editors", "utilities").
    fn category_name() -> &'static str;

    /// Returns the ordinal index of this variant (0-based, contiguous).
    fn variant_index(&self) -> usize;

    /// Serialization key for JSON output (snake_case variant name).
    fn serde_key(&self) -> &'static str;

    /// Platform-specific detection override.
    ///
    /// Returns `Some(...)` to inject a synthetic detection result instead of
    /// searching PATH. Used for Windows SAPI which isn't a real executable.
    fn platform_override(&self) -> Option<(std::path::PathBuf, ExecutableSource)> {
        None
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
