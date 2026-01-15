//! Package manager detection and abstraction layer.
//!
//! This module provides unified types for working with both operating system-level
//! and language ecosystem package managers. It includes:
//!
//! - [`OsPackageManager`]: System-level package managers (apt, homebrew, pacman, etc.)
//! - [`LanguagePackageManager`]: Language ecosystem package managers (npm, cargo, pip, etc.)
//! - [`PackageManager`]: Unified wrapper enum for both types
//! - [`PackageManagerShape`]: Trait for package manager operations
//!
//! ## Examples
//!
//! ```
//! use sniff_lib::package::{OsPackageManager, LanguagePackageManager, PackageManager};
//!
//! let os_mgr = PackageManager::Os(OsPackageManager::Homebrew);
//! let lang_mgr = PackageManager::Language(LanguagePackageManager::Cargo);
//!
//! println!("OS manager: {}", os_mgr);
//! println!("Language manager: {}", lang_mgr);
//! ```

mod network;
mod registry;
mod stubs;

pub use network::{
    enrich_dependencies, enrich_dependency, BunNetwork, CargoNetwork, NpmNetwork, PnpmNetwork,
    YarnNetwork,
};
pub use registry::{get_package_manager, is_registered, registered_managers};
pub use stubs::PackageInfo;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;

use crate::Result;

// ============================================================================
// OsPackageManager Enum
// ============================================================================

/// Operating system-level package manager identification.
///
/// Represents all known system-level package managers across different
/// operating systems and distributions. Each variant corresponds to a
/// specific package management tool.
///
/// ## Categories
///
/// - **Debian family**: Apt, Aptitude, Dpkg, Nala, AptFast
/// - **RedHat family**: Dnf, Yum, Microdnf, Rpm
/// - **Arch family**: Pacman, Makepkg, Yay, Paru, Pamac
/// - **SUSE**: Zypper
/// - **Gentoo**: Portage (emerge)
/// - **Alpine**: Apk
/// - **Void**: Xbps
/// - **Slackware**: Pkgtool
/// - **Cross-distro**: Snap, Flatpak, Guix, Nix, NixEnv
/// - **macOS**: Homebrew, MacPorts, Fink, Softwareupdate
/// - **Windows**: Winget, Dism, Chocolatey, Scoop, Msys2Pacman
/// - **BSD**: Pkg, Ports, PkgAdd, Pkgin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OsPackageManager {
    // ===== Debian family =====
    /// APT (Advanced Package Tool) - Debian/Ubuntu primary package manager
    Apt,
    /// Aptitude - High-level Debian package manager with ncurses interface
    Aptitude,
    /// dpkg - Low-level Debian package manager
    Dpkg,
    /// Nala - Modern apt frontend with parallel downloads
    Nala,
    /// apt-fast - apt accelerator using aria2c
    AptFast,

    // ===== RedHat family =====
    /// DNF (Dandified YUM) - Fedora/RHEL 8+ package manager
    Dnf,
    /// YUM (Yellowdog Updater Modified) - Legacy RHEL/CentOS package manager
    Yum,
    /// microdnf - Minimal DNF for containers
    Microdnf,
    /// RPM - Low-level RedHat package manager
    Rpm,

    // ===== Arch family =====
    /// pacman - Arch Linux package manager
    Pacman,
    /// makepkg - Arch build tool for AUR packages
    Makepkg,
    /// yay - Yet Another Yogurt, AUR helper
    Yay,
    /// paru - Feature-rich AUR helper written in Rust
    Paru,
    /// pamac - Manjaro's graphical package manager
    Pamac,

    // ===== SUSE =====
    /// zypper - SUSE/openSUSE package manager
    Zypper,

    // ===== Gentoo =====
    /// Portage/emerge - Gentoo source-based package manager
    Portage,

    // ===== Alpine =====
    /// apk - Alpine Linux package manager
    Apk,

    // ===== Void =====
    /// xbps - X Binary Package System for Void Linux
    Xbps,

    // ===== Slackware =====
    /// pkgtool - Slackware package manager
    Pkgtool,

    // ===== Cross-distro =====
    /// Snap - Canonical's universal package format
    Snap,
    /// Flatpak - Cross-distro application sandboxing
    Flatpak,
    /// GNU Guix - Functional package manager
    Guix,
    /// Nix - Nix package manager (nix-env, nix profile)
    Nix,
    /// nix-env - Legacy Nix profile management
    NixEnv,

    // ===== macOS =====
    /// Homebrew - macOS/Linux community package manager
    Homebrew,
    /// MacPorts - macOS package manager (formerly DarwinPorts)
    MacPorts,
    /// Fink - Debian-based macOS package manager
    Fink,
    /// softwareupdate - macOS system update tool
    Softwareupdate,

    // ===== Windows =====
    /// winget - Windows Package Manager
    Winget,
    /// DISM - Windows Deployment Image Servicing and Management
    Dism,
    /// Chocolatey - Windows community package manager
    Chocolatey,
    /// Scoop - Windows command-line installer
    Scoop,
    /// MSYS2's pacman - Windows Unix-like environment
    Msys2Pacman,

    // ===== BSD =====
    /// pkg - FreeBSD package manager
    Pkg,
    /// ports - BSD ports collection
    Ports,
    /// pkg_add - OpenBSD package manager
    PkgAdd,
    /// pkgin - NetBSD binary package manager
    Pkgin,
}

impl OsPackageManager {
    /// Returns the command-line executable name for this package manager.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::package::OsPackageManager;
    ///
    /// assert_eq!(OsPackageManager::Apt.executable_name(), "apt");
    /// assert_eq!(OsPackageManager::Homebrew.executable_name(), "brew");
    /// assert_eq!(OsPackageManager::Portage.executable_name(), "emerge");
    /// ```
    #[must_use]
    pub const fn executable_name(&self) -> &'static str {
        match self {
            // Debian family
            OsPackageManager::Apt => "apt",
            OsPackageManager::Aptitude => "aptitude",
            OsPackageManager::Dpkg => "dpkg",
            OsPackageManager::Nala => "nala",
            OsPackageManager::AptFast => "apt-fast",
            // RedHat family
            OsPackageManager::Dnf => "dnf",
            OsPackageManager::Yum => "yum",
            OsPackageManager::Microdnf => "microdnf",
            OsPackageManager::Rpm => "rpm",
            // Arch family
            OsPackageManager::Pacman | OsPackageManager::Msys2Pacman => "pacman",
            OsPackageManager::Makepkg => "makepkg",
            OsPackageManager::Yay => "yay",
            OsPackageManager::Paru => "paru",
            OsPackageManager::Pamac => "pamac",
            // SUSE
            OsPackageManager::Zypper => "zypper",
            // Gentoo
            OsPackageManager::Portage => "emerge",
            // Alpine
            OsPackageManager::Apk => "apk",
            // Void
            OsPackageManager::Xbps => "xbps-install",
            // Slackware
            OsPackageManager::Pkgtool => "pkgtool",
            // Cross-distro
            OsPackageManager::Snap => "snap",
            OsPackageManager::Flatpak => "flatpak",
            OsPackageManager::Guix => "guix",
            OsPackageManager::Nix => "nix",
            OsPackageManager::NixEnv => "nix-env",
            // macOS
            OsPackageManager::Homebrew => "brew",
            OsPackageManager::MacPorts => "port",
            OsPackageManager::Fink => "fink",
            OsPackageManager::Softwareupdate => "softwareupdate",
            // Windows
            OsPackageManager::Winget => "winget",
            OsPackageManager::Dism => "dism",
            OsPackageManager::Chocolatey => "choco",
            OsPackageManager::Scoop => "scoop",
            // BSD
            OsPackageManager::Pkg => "pkg",
            OsPackageManager::Ports => "make",
            OsPackageManager::PkgAdd => "pkg_add",
            OsPackageManager::Pkgin => "pkgin",
        }
    }
}

impl fmt::Display for OsPackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.executable_name())
    }
}

// ============================================================================
// LanguagePackageManager Enum
// ============================================================================

/// Language ecosystem package manager identification.
///
/// Represents package managers for specific programming language ecosystems.
/// Each variant corresponds to a tool used to manage dependencies within
/// a language's package registry.
///
/// ## Categories
///
/// - **JavaScript/TypeScript**: Npm, Pnpm, Yarn, YarnClassic, YarnBerry, Bun, Deno, Jspm
/// - **Rust**: Cargo
/// - **Python**: Pip, Pipx, Poetry, Pdm, Uv, Conda, Mamba, Micromamba, Hatch, Flit, Setuptools, Rye, Pixi
/// - **Ruby**: Gem, Bundler
/// - **PHP**: Composer
/// - **Go**: GoModules
/// - **Java/Kotlin**: Maven, Gradle, Sbt, Mill, Leiningen, Ant, Ivy
/// - **C/C++**: Vcpkg, Conan, Hunter, Cpm, Meson, Xmake
/// - **.NET**: Nuget, DotnetCli, Paket
/// - **Lua**: Luarocks
/// - **Perl**: Cpan, Cpanm
/// - **R**: Cran, Renv, Pak
/// - **Haskell**: Cabal, Stack
/// - **Elixir**: Mix, Hex
/// - **Erlang**: Rebar3
/// - **OCaml**: Opam, Dune
/// - **Dart/Flutter**: Pub
/// - **Swift**: SwiftPm, Carthage, Cocoapods
/// - **Zig**: Zigmod, Gyro
/// - **Nim**: Nimble
/// - **Julia**: JuliaPkg
/// - **Clojure**: ClojureDeps
/// - **Scala**: Coursier
/// - **Crystal**: Shards
/// - **V**: Vpm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LanguagePackageManager {
    // ===== JavaScript/TypeScript =====
    /// npm - Node Package Manager, the default for Node.js
    Npm,
    /// pnpm - Performant npm, disk-space efficient
    Pnpm,
    /// Yarn - Facebook's package manager (generic, version-agnostic)
    Yarn,
    /// Yarn Classic (v1.x) - Original Yarn implementation
    YarnClassic,
    /// Yarn Berry (v2+) - Modern Yarn with Plug'n'Play
    YarnBerry,
    /// Bun - All-in-one JavaScript runtime and package manager
    Bun,
    /// Deno - Secure runtime with built-in package management
    Deno,
    /// jspm - ES module package manager
    Jspm,

    // ===== Rust =====
    /// Cargo - Rust's official package manager and build tool
    Cargo,

    // ===== Python =====
    /// pip - Python's standard package installer
    Pip,
    /// pipx - Install and run Python applications in isolated environments
    Pipx,
    /// Poetry - Python dependency management and packaging
    Poetry,
    /// PDM - Python Development Master, PEP 582 compliant
    Pdm,
    /// uv - Extremely fast Python package installer (by Astral)
    Uv,
    /// Conda - Cross-platform package manager for data science
    Conda,
    /// Mamba - Fast drop-in replacement for Conda
    Mamba,
    /// Micromamba - Minimal Mamba implementation
    Micromamba,
    /// Hatch - Modern Python project management
    Hatch,
    /// Flit - Simple Python packaging tool
    Flit,
    /// Setuptools - Traditional Python build system
    Setuptools,
    /// Rye - Comprehensive Python project manager by Astral
    Rye,
    /// Pixi - Fast package manager for Conda environments
    Pixi,

    // ===== Ruby =====
    /// RubyGems - Ruby's package manager
    Gem,
    /// Bundler - Ruby dependency manager
    Bundler,

    // ===== PHP =====
    /// Composer - PHP dependency manager
    Composer,

    // ===== Go =====
    /// Go Modules - Go's built-in dependency management
    GoModules,

    // ===== Java/Kotlin =====
    /// Maven - Java project management and build tool
    Maven,
    /// Gradle - Build automation for Java/Kotlin/Android
    Gradle,
    /// sbt - Scala Build Tool
    Sbt,
    /// Mill - Fast JVM build tool
    Mill,
    /// Leiningen - Clojure build tool and dependency manager
    Leiningen,
    /// Apache Ant - Java build tool
    Ant,
    /// Apache Ivy - Dependency manager for Ant
    Ivy,

    // ===== C/C++ =====
    /// vcpkg - Microsoft's C/C++ package manager
    Vcpkg,
    /// Conan - C/C++ package manager
    Conan,
    /// Hunter - CMake-driven cross-platform package manager
    Hunter,
    /// CPM.cmake - CMake package manager
    Cpm,
    /// Meson - Build system with dependency management
    Meson,
    /// xmake - Cross-platform build utility
    Xmake,

    // ===== .NET =====
    /// NuGet - .NET package manager
    Nuget,
    /// dotnet CLI - .NET command-line interface
    DotnetCli,
    /// Paket - .NET dependency manager
    Paket,

    // ===== Lua =====
    /// LuaRocks - Lua package manager
    Luarocks,

    // ===== Perl =====
    /// CPAN - Comprehensive Perl Archive Network
    Cpan,
    /// cpanm - CPAN minus, minimal CPAN client
    Cpanm,

    // ===== R =====
    /// CRAN - Comprehensive R Archive Network
    Cran,
    /// renv - R environment management
    Renv,
    /// pak - Fast R package installer
    Pak,

    // ===== Haskell =====
    /// Cabal - Haskell build tool and package manager
    Cabal,
    /// Stack - Haskell build tool with curated package sets
    Stack,

    // ===== Elixir =====
    /// Mix - Elixir's build tool
    Mix,
    /// Hex - Elixir/Erlang package manager
    Hex,

    // ===== Erlang =====
    /// Rebar3 - Erlang build tool
    Rebar3,

    // ===== OCaml =====
    /// opam - OCaml package manager
    Opam,
    /// Dune - OCaml build system
    Dune,

    // ===== Dart/Flutter =====
    /// pub - Dart package manager
    Pub,

    // ===== Swift =====
    /// Swift Package Manager
    SwiftPm,
    /// Carthage - Decentralized dependency manager for Cocoa
    Carthage,
    /// CocoaPods - Dependency manager for Swift/Objective-C
    Cocoapods,

    // ===== Zig =====
    /// zigmod - Zig package manager
    Zigmod,
    /// gyro - Zig package manager
    Gyro,

    // ===== Nim =====
    /// Nimble - Nim package manager
    Nimble,

    // ===== Julia =====
    /// Julia Pkg - Julia's built-in package manager
    JuliaPkg,

    // ===== Clojure =====
    /// Clojure tools.deps
    ClojureDeps,

    // ===== Scala =====
    /// Coursier - Scala artifact fetcher
    Coursier,

    // ===== Crystal =====
    /// Shards - Crystal dependency manager
    Shards,

    // ===== V =====
    /// VPM - V package manager
    Vpm,
}

impl LanguagePackageManager {
    /// Returns the command-line executable name for this package manager.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::package::LanguagePackageManager;
    ///
    /// assert_eq!(LanguagePackageManager::Npm.executable_name(), "npm");
    /// assert_eq!(LanguagePackageManager::Cargo.executable_name(), "cargo");
    /// assert_eq!(LanguagePackageManager::Pip.executable_name(), "pip");
    /// ```
    #[must_use]
    pub const fn executable_name(&self) -> &'static str {
        match self {
            // JavaScript/TypeScript
            LanguagePackageManager::Npm => "npm",
            LanguagePackageManager::Pnpm => "pnpm",
            LanguagePackageManager::Yarn
            | LanguagePackageManager::YarnClassic
            | LanguagePackageManager::YarnBerry => "yarn",
            LanguagePackageManager::Bun => "bun",
            LanguagePackageManager::Deno => "deno",
            LanguagePackageManager::Jspm => "jspm",
            // Rust
            LanguagePackageManager::Cargo => "cargo",
            // Python
            LanguagePackageManager::Pip => "pip",
            LanguagePackageManager::Pipx => "pipx",
            LanguagePackageManager::Poetry => "poetry",
            LanguagePackageManager::Pdm => "pdm",
            LanguagePackageManager::Uv => "uv",
            LanguagePackageManager::Conda => "conda",
            LanguagePackageManager::Mamba => "mamba",
            LanguagePackageManager::Micromamba => "micromamba",
            LanguagePackageManager::Hatch => "hatch",
            LanguagePackageManager::Flit => "flit",
            LanguagePackageManager::Setuptools => "python", // setuptools via python -m
            LanguagePackageManager::Rye => "rye",
            LanguagePackageManager::Pixi => "pixi",
            // Ruby
            LanguagePackageManager::Gem => "gem",
            LanguagePackageManager::Bundler => "bundle",
            // PHP
            LanguagePackageManager::Composer => "composer",
            // Go
            LanguagePackageManager::GoModules => "go",
            // Java/Kotlin
            LanguagePackageManager::Maven => "mvn",
            LanguagePackageManager::Gradle => "gradle",
            LanguagePackageManager::Sbt => "sbt",
            LanguagePackageManager::Mill => "mill",
            LanguagePackageManager::Leiningen => "lein",
            LanguagePackageManager::Ant => "ant",
            LanguagePackageManager::Ivy => "ivy",
            // C/C++
            LanguagePackageManager::Vcpkg => "vcpkg",
            LanguagePackageManager::Conan => "conan",
            LanguagePackageManager::Hunter => "cmake", // Hunter is CMake-based
            LanguagePackageManager::Cpm => "cmake",    // CPM is CMake-based
            LanguagePackageManager::Meson => "meson",
            LanguagePackageManager::Xmake => "xmake",
            // .NET
            LanguagePackageManager::Nuget => "nuget",
            LanguagePackageManager::DotnetCli => "dotnet",
            LanguagePackageManager::Paket => "paket",
            // Lua
            LanguagePackageManager::Luarocks => "luarocks",
            // Perl
            LanguagePackageManager::Cpan => "cpan",
            LanguagePackageManager::Cpanm => "cpanm",
            // R
            LanguagePackageManager::Cran => "R",
            LanguagePackageManager::Renv => "R", // renv via R
            LanguagePackageManager::Pak => "R",  // pak via R
            // Haskell
            LanguagePackageManager::Cabal => "cabal",
            LanguagePackageManager::Stack => "stack",
            // Elixir
            LanguagePackageManager::Mix => "mix",
            LanguagePackageManager::Hex => "mix", // Hex via mix
            // Erlang
            LanguagePackageManager::Rebar3 => "rebar3",
            // OCaml
            LanguagePackageManager::Opam => "opam",
            LanguagePackageManager::Dune => "dune",
            // Dart/Flutter
            LanguagePackageManager::Pub => "dart",
            // Swift
            LanguagePackageManager::SwiftPm => "swift",
            LanguagePackageManager::Carthage => "carthage",
            LanguagePackageManager::Cocoapods => "pod",
            // Zig
            LanguagePackageManager::Zigmod => "zigmod",
            LanguagePackageManager::Gyro => "gyro",
            // Nim
            LanguagePackageManager::Nimble => "nimble",
            // Julia
            LanguagePackageManager::JuliaPkg => "julia",
            // Clojure
            LanguagePackageManager::ClojureDeps => "clj",
            // Scala
            LanguagePackageManager::Coursier => "cs",
            // Crystal
            LanguagePackageManager::Shards => "shards",
            // V
            LanguagePackageManager::Vpm => "v",
        }
    }

    /// Returns the primary programming language for this package manager.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::package::LanguagePackageManager;
    ///
    /// assert_eq!(LanguagePackageManager::Npm.language(), "JavaScript");
    /// assert_eq!(LanguagePackageManager::Cargo.language(), "Rust");
    /// assert_eq!(LanguagePackageManager::Pip.language(), "Python");
    /// ```
    #[must_use]
    pub const fn language(&self) -> &'static str {
        match self {
            LanguagePackageManager::Npm
            | LanguagePackageManager::Pnpm
            | LanguagePackageManager::Yarn
            | LanguagePackageManager::YarnClassic
            | LanguagePackageManager::YarnBerry
            | LanguagePackageManager::Bun
            | LanguagePackageManager::Deno
            | LanguagePackageManager::Jspm => "JavaScript",
            LanguagePackageManager::Cargo => "Rust",
            LanguagePackageManager::Pip
            | LanguagePackageManager::Pipx
            | LanguagePackageManager::Poetry
            | LanguagePackageManager::Pdm
            | LanguagePackageManager::Uv
            | LanguagePackageManager::Conda
            | LanguagePackageManager::Mamba
            | LanguagePackageManager::Micromamba
            | LanguagePackageManager::Hatch
            | LanguagePackageManager::Flit
            | LanguagePackageManager::Setuptools
            | LanguagePackageManager::Rye
            | LanguagePackageManager::Pixi => "Python",
            LanguagePackageManager::Gem | LanguagePackageManager::Bundler => "Ruby",
            LanguagePackageManager::Composer => "PHP",
            LanguagePackageManager::GoModules => "Go",
            LanguagePackageManager::Maven
            | LanguagePackageManager::Gradle
            | LanguagePackageManager::Ant
            | LanguagePackageManager::Ivy => "Java",
            LanguagePackageManager::Sbt
            | LanguagePackageManager::Mill
            | LanguagePackageManager::Coursier => "Scala",
            LanguagePackageManager::Leiningen | LanguagePackageManager::ClojureDeps => "Clojure",
            LanguagePackageManager::Vcpkg
            | LanguagePackageManager::Conan
            | LanguagePackageManager::Hunter
            | LanguagePackageManager::Cpm
            | LanguagePackageManager::Meson
            | LanguagePackageManager::Xmake => "C/C++",
            LanguagePackageManager::Nuget
            | LanguagePackageManager::DotnetCli
            | LanguagePackageManager::Paket => ".NET",
            LanguagePackageManager::Luarocks => "Lua",
            LanguagePackageManager::Cpan | LanguagePackageManager::Cpanm => "Perl",
            LanguagePackageManager::Cran
            | LanguagePackageManager::Renv
            | LanguagePackageManager::Pak => "R",
            LanguagePackageManager::Cabal | LanguagePackageManager::Stack => "Haskell",
            LanguagePackageManager::Mix | LanguagePackageManager::Hex => "Elixir",
            LanguagePackageManager::Rebar3 => "Erlang",
            LanguagePackageManager::Opam | LanguagePackageManager::Dune => "OCaml",
            LanguagePackageManager::Pub => "Dart",
            LanguagePackageManager::SwiftPm
            | LanguagePackageManager::Carthage
            | LanguagePackageManager::Cocoapods => "Swift",
            LanguagePackageManager::Zigmod | LanguagePackageManager::Gyro => "Zig",
            LanguagePackageManager::Nimble => "Nim",
            LanguagePackageManager::JuliaPkg => "Julia",
            LanguagePackageManager::Shards => "Crystal",
            LanguagePackageManager::Vpm => "V",
        }
    }
}

impl fmt::Display for LanguagePackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            // JavaScript/TypeScript
            LanguagePackageManager::Npm => "npm",
            LanguagePackageManager::Pnpm => "pnpm",
            LanguagePackageManager::Yarn => "yarn",
            LanguagePackageManager::YarnClassic => "yarn-classic",
            LanguagePackageManager::YarnBerry => "yarn-berry",
            LanguagePackageManager::Bun => "bun",
            LanguagePackageManager::Deno => "deno",
            LanguagePackageManager::Jspm => "jspm",
            // Rust
            LanguagePackageManager::Cargo => "cargo",
            // Python
            LanguagePackageManager::Pip => "pip",
            LanguagePackageManager::Pipx => "pipx",
            LanguagePackageManager::Poetry => "poetry",
            LanguagePackageManager::Pdm => "pdm",
            LanguagePackageManager::Uv => "uv",
            LanguagePackageManager::Conda => "conda",
            LanguagePackageManager::Mamba => "mamba",
            LanguagePackageManager::Micromamba => "micromamba",
            LanguagePackageManager::Hatch => "hatch",
            LanguagePackageManager::Flit => "flit",
            LanguagePackageManager::Setuptools => "setuptools",
            LanguagePackageManager::Rye => "rye",
            LanguagePackageManager::Pixi => "pixi",
            // Ruby
            LanguagePackageManager::Gem => "gem",
            LanguagePackageManager::Bundler => "bundler",
            // PHP
            LanguagePackageManager::Composer => "composer",
            // Go
            LanguagePackageManager::GoModules => "go-modules",
            // Java/Kotlin
            LanguagePackageManager::Maven => "maven",
            LanguagePackageManager::Gradle => "gradle",
            LanguagePackageManager::Sbt => "sbt",
            LanguagePackageManager::Mill => "mill",
            LanguagePackageManager::Leiningen => "leiningen",
            LanguagePackageManager::Ant => "ant",
            LanguagePackageManager::Ivy => "ivy",
            // C/C++
            LanguagePackageManager::Vcpkg => "vcpkg",
            LanguagePackageManager::Conan => "conan",
            LanguagePackageManager::Hunter => "hunter",
            LanguagePackageManager::Cpm => "cpm",
            LanguagePackageManager::Meson => "meson",
            LanguagePackageManager::Xmake => "xmake",
            // .NET
            LanguagePackageManager::Nuget => "nuget",
            LanguagePackageManager::DotnetCli => "dotnet",
            LanguagePackageManager::Paket => "paket",
            // Lua
            LanguagePackageManager::Luarocks => "luarocks",
            // Perl
            LanguagePackageManager::Cpan => "cpan",
            LanguagePackageManager::Cpanm => "cpanm",
            // R
            LanguagePackageManager::Cran => "cran",
            LanguagePackageManager::Renv => "renv",
            LanguagePackageManager::Pak => "pak",
            // Haskell
            LanguagePackageManager::Cabal => "cabal",
            LanguagePackageManager::Stack => "stack",
            // Elixir
            LanguagePackageManager::Mix => "mix",
            LanguagePackageManager::Hex => "hex",
            // Erlang
            LanguagePackageManager::Rebar3 => "rebar3",
            // OCaml
            LanguagePackageManager::Opam => "opam",
            LanguagePackageManager::Dune => "dune",
            // Dart/Flutter
            LanguagePackageManager::Pub => "pub",
            // Swift
            LanguagePackageManager::SwiftPm => "swift-pm",
            LanguagePackageManager::Carthage => "carthage",
            LanguagePackageManager::Cocoapods => "cocoapods",
            // Zig
            LanguagePackageManager::Zigmod => "zigmod",
            LanguagePackageManager::Gyro => "gyro",
            // Nim
            LanguagePackageManager::Nimble => "nimble",
            // Julia
            LanguagePackageManager::JuliaPkg => "julia-pkg",
            // Clojure
            LanguagePackageManager::ClojureDeps => "clojure-deps",
            // Scala
            LanguagePackageManager::Coursier => "coursier",
            // Crystal
            LanguagePackageManager::Shards => "shards",
            // V
            LanguagePackageManager::Vpm => "vpm",
        };
        write!(f, "{name}")
    }
}

// ============================================================================
// PackageManager Wrapper Enum
// ============================================================================

/// Unified package manager type encompassing both OS and language package managers.
///
/// This wrapper enum provides a single type for working with any kind of
/// package manager, whether it's an operating system-level manager like apt
/// or homebrew, or a language ecosystem manager like npm or cargo.
///
/// ## Examples
///
/// ```
/// use sniff_lib::package::{PackageManager, OsPackageManager, LanguagePackageManager};
///
/// let managers: Vec<PackageManager> = vec![
///     PackageManager::Os(OsPackageManager::Apt),
///     PackageManager::Language(LanguagePackageManager::Npm),
///     PackageManager::Os(OsPackageManager::Homebrew),
///     PackageManager::Language(LanguagePackageManager::Cargo),
/// ];
///
/// for mgr in &managers {
///     println!("{} (executable: {})", mgr, mgr.executable_name());
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PackageManager {
    /// Operating system-level package manager
    Os(OsPackageManager),
    /// Language ecosystem package manager
    Language(LanguagePackageManager),
}

impl PackageManager {
    /// Returns the command-line executable name for this package manager.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::package::{PackageManager, OsPackageManager, LanguagePackageManager};
    ///
    /// let apt = PackageManager::Os(OsPackageManager::Apt);
    /// assert_eq!(apt.executable_name(), "apt");
    ///
    /// let npm = PackageManager::Language(LanguagePackageManager::Npm);
    /// assert_eq!(npm.executable_name(), "npm");
    /// ```
    #[must_use]
    pub const fn executable_name(&self) -> &'static str {
        match self {
            PackageManager::Os(os) => os.executable_name(),
            PackageManager::Language(lang) => lang.executable_name(),
        }
    }

    /// Returns whether this is an OS-level package manager.
    #[must_use]
    pub const fn is_os(&self) -> bool {
        matches!(self, PackageManager::Os(_))
    }

    /// Returns whether this is a language ecosystem package manager.
    #[must_use]
    pub const fn is_language(&self) -> bool {
        matches!(self, PackageManager::Language(_))
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManager::Os(os) => write!(f, "{os}"),
            PackageManager::Language(lang) => write!(f, "{lang}"),
        }
    }
}

impl From<OsPackageManager> for PackageManager {
    fn from(os: OsPackageManager) -> Self {
        PackageManager::Os(os)
    }
}

impl From<LanguagePackageManager> for PackageManager {
    fn from(lang: LanguagePackageManager) -> Self {
        PackageManager::Language(lang)
    }
}

// ============================================================================
// PackageManagerShape Trait
// ============================================================================

/// Boxed future type for async trait methods.
///
/// This type alias provides dyn-compatible async method returns.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait defining the interface for package manager operations.
///
/// This trait is dyn-compatible (object-safe) by using boxed futures for
/// async methods. Implementors must be `Send + Sync` to support concurrent
/// operations and storage in static registries.
///
/// ## Examples
///
/// ```ignore
/// use sniff_lib::package::{PackageManagerShape, PackageInfo};
///
/// async fn check_package(mgr: &dyn PackageManagerShape, name: &str) {
///     if mgr.is_available() {
///         match mgr.find_package(name).await {
///             Ok(Some(info)) => println!("Found: {} v{}", info.name, info.version),
///             Ok(None) => println!("Package not found"),
///             Err(e) => eprintln!("Error: {}", e),
///         }
///     }
/// }
/// ```
pub trait PackageManagerShape: Send + Sync {
    /// Returns the executable name for this package manager.
    fn executable_name(&self) -> &'static str;

    /// Checks if this package manager is available on the system.
    ///
    /// This performs a filesystem check (no process spawning) to determine
    /// if the package manager executable exists in the PATH.
    fn is_available(&self) -> bool;

    /// Finds a package by name, returning metadata if found.
    ///
    /// ## Arguments
    ///
    /// * `name` - The package name to search for
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(PackageInfo))` if the package is found
    /// - `Ok(None)` if the package is not found
    /// - `Err(_)` if an error occurred during the search
    fn find_package(&self, name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>>;

    /// Gets the latest version of a package.
    ///
    /// ## Arguments
    ///
    /// * `name` - The package name to query
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(version))` if the package is found with a version
    /// - `Ok(None)` if the package is not found or has no version
    /// - `Err(_)` if an error occurred during the query
    fn latest_version(&self, name: &str) -> BoxFuture<'_, Result<Option<String>>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_package_manager_executable_names() {
        assert_eq!(OsPackageManager::Apt.executable_name(), "apt");
        assert_eq!(OsPackageManager::Homebrew.executable_name(), "brew");
        assert_eq!(OsPackageManager::Pacman.executable_name(), "pacman");
        assert_eq!(OsPackageManager::Portage.executable_name(), "emerge");
        assert_eq!(OsPackageManager::Winget.executable_name(), "winget");
    }

    #[test]
    fn test_os_package_manager_display() {
        assert_eq!(format!("{}", OsPackageManager::Apt), "apt");
        assert_eq!(format!("{}", OsPackageManager::Homebrew), "brew");
        assert_eq!(format!("{}", OsPackageManager::Chocolatey), "choco");
    }

    #[test]
    fn test_language_package_manager_executable_names() {
        assert_eq!(LanguagePackageManager::Npm.executable_name(), "npm");
        assert_eq!(LanguagePackageManager::Cargo.executable_name(), "cargo");
        assert_eq!(LanguagePackageManager::Pip.executable_name(), "pip");
        assert_eq!(LanguagePackageManager::GoModules.executable_name(), "go");
        assert_eq!(LanguagePackageManager::Maven.executable_name(), "mvn");
    }

    #[test]
    fn test_language_package_manager_display() {
        assert_eq!(format!("{}", LanguagePackageManager::Npm), "npm");
        assert_eq!(format!("{}", LanguagePackageManager::Cargo), "cargo");
        assert_eq!(format!("{}", LanguagePackageManager::YarnBerry), "yarn-berry");
        assert_eq!(format!("{}", LanguagePackageManager::GoModules), "go-modules");
    }

    #[test]
    fn test_language_package_manager_language() {
        assert_eq!(LanguagePackageManager::Npm.language(), "JavaScript");
        assert_eq!(LanguagePackageManager::Cargo.language(), "Rust");
        assert_eq!(LanguagePackageManager::Pip.language(), "Python");
        assert_eq!(LanguagePackageManager::GoModules.language(), "Go");
        assert_eq!(LanguagePackageManager::Maven.language(), "Java");
    }

    #[test]
    fn test_package_manager_wrapper() {
        let os_mgr = PackageManager::Os(OsPackageManager::Apt);
        let lang_mgr = PackageManager::Language(LanguagePackageManager::Npm);

        assert!(os_mgr.is_os());
        assert!(!os_mgr.is_language());
        assert!(!lang_mgr.is_os());
        assert!(lang_mgr.is_language());

        assert_eq!(os_mgr.executable_name(), "apt");
        assert_eq!(lang_mgr.executable_name(), "npm");
    }

    #[test]
    fn test_package_manager_display() {
        let os_mgr = PackageManager::Os(OsPackageManager::Homebrew);
        let lang_mgr = PackageManager::Language(LanguagePackageManager::Cargo);

        assert_eq!(format!("{os_mgr}"), "brew");
        assert_eq!(format!("{lang_mgr}"), "cargo");
    }

    #[test]
    fn test_package_manager_from_impls() {
        let os: PackageManager = OsPackageManager::Apt.into();
        let lang: PackageManager = LanguagePackageManager::Npm.into();

        assert!(matches!(os, PackageManager::Os(OsPackageManager::Apt)));
        assert!(matches!(lang, PackageManager::Language(LanguagePackageManager::Npm)));
    }

    #[test]
    fn test_serde_roundtrip_os() {
        let mgr = OsPackageManager::Homebrew;
        let json = serde_json::to_string(&mgr).unwrap();
        let restored: OsPackageManager = serde_json::from_str(&json).unwrap();
        assert_eq!(mgr, restored);
    }

    #[test]
    fn test_serde_roundtrip_language() {
        let mgr = LanguagePackageManager::Cargo;
        let json = serde_json::to_string(&mgr).unwrap();
        let restored: LanguagePackageManager = serde_json::from_str(&json).unwrap();
        assert_eq!(mgr, restored);
    }

    #[test]
    fn test_serde_roundtrip_wrapper() {
        let mgr = PackageManager::Language(LanguagePackageManager::Poetry);
        let json = serde_json::to_string(&mgr).unwrap();
        let restored: PackageManager = serde_json::from_str(&json).unwrap();
        assert_eq!(mgr, restored);
    }
}
