use std::{path::PathBuf, sync::LazyLock};
use reqwest::Url;

use crate::{error::SniffInstallationError, os::OsType, programs::ProgramError};

/// **InstallationMethod**
///
/// Describes an installation method for installing some piece of software.
///
/// This installation takes two broad forms:
///
/// 1. Using a package manager (OS level _or_ Language specific)
/// 2. Downloading and bash script and executing it locally
pub enum InstallationMethod {
    /// Default Node.js package manager; registry, dependency resolution, scripts. [Website](https://www.npmjs.com)
    Npm(String),
    /// Disk-efficient Node.js package manager using a content-addressable store. [Website](https://pnpm.io)
    Pnpm(String),
    /// Alternative Node.js package manager; workspaces, Plug’n’Play. [Website](https://yarnpkg.com)
    Yarn(String),
    /// All-in-one JS runtime with built-in package manager. [Website](https://bun.sh)
    Bun(String),
    /// Official Rust package manager and build tool. [Website](https://doc.rust-lang.org/cargo)
    Cargo(String),
    /// Built-in Go dependency system integrated with the go tool. [Website](https://go.dev/ref/mod)
    GoModules(String),
    /// Dependency manager for modern PHP applications. [Website](https://getcomposer.org)
    Composer(String),
    /// Official Swift dependency manager, integrated with the Swift toolchain. [Website](https://www.swift.org/package-manager)
    SwiftPm(String),
    /// Standard package manager for Lua modules. [Website](https://luarocks.org)
    LuaRocks(String),
    /// Cross-platform C/C++ dependency manager backed by Microsoft. [Website](https://vcpkg.io)
    VcPkg(String),
    /// Decentralized C/C++ package manager with build-system integration. [Website](https://conan.io)
    Conan(String),
    /// Official package manager for .NET and C# ecosystems. [Website](https://www.nuget.org)
    Nuget(String),
    /// Package manager for the BEAM (Elixir, Erlang) ecosystem. [Website](https://hex.pm)
    Hex(String),
    /// Traditional Python package installer. [Website](https://pip.pypa.io)
    Pip(String),
    /// High-performance Python package manager and virtual environment tool. [Website](https://astral.sh/uv)
    Uv(String),
    /// Dependency manager and build system with lockfile support. [Website](https://python-poetry.org)
    Poetry(String),
    /// Canonical archive and installer for Perl modules. [Website](https://www.cpan.org)
    Cpan(String),
    /// Lightweight, scriptable CPAN client. [Website](https://metacpan.org/pod/App::cpanminus)
    Cpanm(String),

    /// Debian/Ubuntu primary package manager. [Website](https://tracker.debian.org/pkg/apt)
    Apt(String),
    /// Modern apt frontend with parallel downloads. [Website](https://github.com/volitank/nala)
    Nala(String),
    /// macOS/Linux community package manager. [Website](https://brew.sh)
    Brew(String),
    /// Fedora/RHEL primary package manager. [Website](https://github.com/rpm-software-management/dnf)
    Dnf(String),
    /// Arch Linux package manager. [Website](https://archlinux.org/pacman/)
    Pacman(String),
    /// Windows Package Manager. [Website](https://github.com/microsoft/winget-cli)
    Winget(String),
    /// Windows community package manager. [Website](https://chocolatey.org)
    Chocolatey(String),
    /// Windows command-line installer. [Website](https://scoop.sh)
    Scoop(String),
    /// Nix package manager. [Website](https://nixos.org)
    Nix(String),


    /// Install by downloading a bash script from a URL and then
    /// piping it to the host's `bash` command for installation.
    RemoteBash(Url)
}

/// **ProgramMetadata**
///
/// provides the metadata for a program to help support this command being used as
/// part of the `ProgramDetector` struct.
pub struct ProgramMetadata {
    /// the name of the software
    pub name: &'static str,
    /// a description of the software
    pub description: &'static str,
    /// the operating systems this software can run on
    pub os_availability: &'static Vec<OsType>,
    /// the primary website describing the program
    pub website: &'static Url,
    /// the repo for the program (if available)
    pub repo: Option<&'static Url>,
    /// describes various methods for installing this software
    pub installation_methods: &'static Vec<InstallationMethod>
}





pub trait ProgramDetector {
    /// re-check program availability
    fn refresh(&self) -> Self;

    /// returns a fully qualified file path to the program
    /// (if it's installed).
    fn path<T>(&self, e: T) -> Option<PathBuf>;

    /// returns the version of the program which is installed
    /// on the host.
    fn version<T>(&self, e: T) -> Result<String, ProgramError>;

    /// Provides a URL to the website for the specified software
    /// package.
    fn website<T>(&self, e: T) -> &'static str;

    /// Returns a short description of the program in Markdown
    /// format. The description will use a Markdown link to
    /// point the Package's name to the website's URL.
    fn description<T>(&self, e: T) -> String;

    /// Returns the same description provided by the description()
    /// function but instead of Markdown content as plain text, it
    /// will return it as text designed for the terminal:
    ///
    /// - the markdown link will be converted to a OSC8 link
    /// - The textual content for the package name will be
    ///   boldfaced and made blue to help it stand out from the
    ///   rest of the text.
    ///
    /// **Note:** the terminal is assumed to support both OSC8
    /// and color escape codes.
    fn description_for_terminal<T>(&self, e: T) -> String;


    /// Provides a list of the installed software for the given
    /// struct.
    fn installed<T>(&self) -> Vec<T>;


    /// returns a boolean flag indicating whether the specified software
    /// can be installed based on the OS, package managers installed,
    /// and the provided installation methods of the package.
    fn installable<T>(&self, e: T) -> bool;

    /// Attempts to install the program onto the host.
    ///
    /// **Note:** if the method of installation is using a programming language's
    /// package manager (versus an OS-level package manager's) the means
    /// of "installing" it will aim for a "global" installation where the
    /// program will ultimately end up in the executable path of the host.
    ///
    /// **Note:** if OS level package managers _and_ language specific package
    /// manager's are both offered as options for installing this software then
    /// the OS level package manager will be preferred (if available).
    ///
    /// **Note:** package manager installation methods are always used over a
    /// bash script based install when they are available.
    fn install<T>(&self, e: T) -> Result<(), SniffInstallationError>;


    /// Attempts to install a _specific version_ of the specified program onto
    /// the host.
    ///
    /// **Note:** `bash` based installation methods will NOT be used as they have
    /// no mechanism for specifying a version.
    fn install_version<T, V: Into<String>>(&self, e: T, version: V) -> Result<(), SniffInstallationError>;
}
