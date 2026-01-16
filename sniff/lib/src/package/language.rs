//! Language ecosystem package manager types.
//!
//! This module provides the [`LanguagePackageManager`] enum representing package
//! managers for specific programming language ecosystems.

use serde::{Deserialize, Serialize};
use std::fmt;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            format!("{}", LanguagePackageManager::YarnBerry),
            "yarn-berry"
        );
        assert_eq!(
            format!("{}", LanguagePackageManager::GoModules),
            "go-modules"
        );
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
    fn test_serde_roundtrip_language() {
        let mgr = LanguagePackageManager::Cargo;
        let json = serde_json::to_string(&mgr).unwrap();
        let restored: LanguagePackageManager = serde_json::from_str(&json).unwrap();
        assert_eq!(mgr, restored);
    }
}
