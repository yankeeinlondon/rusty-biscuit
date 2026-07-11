//! Program category enums with strum derives.
//!
//! This module defines enums for each program category. Each enum carries
//! metadata lookup support via the `ProgramMetadata` trait from schema.rs.

use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum::{Display, EnumCount, EnumIter, EnumString, IntoStaticStr};

use crate::os::OsType;

pub(crate) static ALL_OS: &[OsType] = &[OsType::MacOS, OsType::Linux, OsType::Windows];
pub(crate) static UNIX_ONLY: &[OsType] = &[OsType::MacOS, OsType::Linux];
pub(crate) static MACOS_ONLY: &[OsType] = &[OsType::MacOS];
pub(crate) static LINUX_ONLY: &[OsType] = &[OsType::Linux];
pub(crate) static WINDOWS_ONLY: &[OsType] = &[OsType::Windows];

// ============================================================================
// Editor Enum
// ============================================================================

/// Text editors and IDEs.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum Editor {
    Vi,
    Vim,
    Neovim,
    Emacs,
    XEmacs,
    Nano,
    Helix,
    VSCode,
    VSCodium,
    Sublime,
    Zed,
    Micro,
    Kakoune,
    Amp,
    Lapce,
    PhpStorm,
    IntellijIdea,
    PyCharm,
    WebStorm,
    CLion,
    GoLand,
    Rider,
    TextMate,
    BBEdit,
    Geany,
    Kate,
}

// ============================================================================
// Utility Enum
// ============================================================================

/// Modern command-line utilities.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum Utility {
    Exa,
    Eza,
    Ripgrep,
    Dust,
    Bat,
    Fd,
    Procs,
    Bottom,
    Fzf,
    Zoxide,
    Starship,
    Direnv,
    Jq,
    Delta,
    Tealdeer,
    Lazygit,
    Gh,
    Htop,
    Btop,
    Tmux,
    Zellij,
    Httpie,
    Curlie,
    Mise,
    Hyperfine,
    Tokei,
    Xh,
    Curl,
    Wget,
    Iperf3,
}

// ============================================================================
// Language Package Manager Enum
// ============================================================================

/// Language-specific package managers.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum LanguagePackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Cargo,
    GoModules,
    Composer,
    SwiftPm,
    Luarocks,
    Vcpkg,
    Conan,
    Nuget,
    Hex,
    Pip,
    Uv,
    Poetry,
    Cpan,
    Cpanm,
}

// ============================================================================
// OS Package Manager Enum
// ============================================================================

/// Operating system package managers.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum OsPackageManager {
    Apt,
    Nala,
    Brew,
    Dnf,
    Pacman,
    Winget,
    Chocolatey,
    Scoop,
    Nix,
}

// ============================================================================
// TTS Client Enum
// ============================================================================

/// Text-to-speech clients.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum TtsClient {
    Say,
    Espeak,
    EspeakNg,
    Festival,
    Mimic,
    Mimic3,
    Piper,
    Echogarden,
    Balcon,
    WindowsSapi,
    GttsCli,
    CoquiTts,
    SherpaOnnx,
    KokoroTts,
    Pico2Wave,
}

// ============================================================================
// Terminal App Enum
// ============================================================================

/// Terminal emulators.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum TerminalApp {
    Alacritty,
    Kitty,
    ITerm2,
    WezTerm,
    Ghostty,
    Warp,
    Rio,
    Tabby,
    Foot,
    GnomeTerminal,
    Konsole,
    XfceTerminal,
    Terminology,
    St,
    Xterm,
    Hyper,
    WindowsTerminal,
}

// ============================================================================
// Headless Audio Player Enum
// ============================================================================

/// Headless audio players for CLI/background playback.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum HeadlessAudio {
    Mpv,
    Ffplay,
    Vlc,
    MPlayer,
    GstreamerGstPlay,
    Sox,
    Mpg123,
    Ogg123,
    AlsaAplay,
    MacOsAfplay,
    PulseaudioPaplay,
    PulseaudioPacat,
    Pipewire,
}

// ============================================================================
// AI CLI Enum
// ============================================================================

/// AI-powered command-line interface tools.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum AiCli {
    Claude,
    Opencode,
    Roo,
    GeminiCli,
    Aider,
    Codex,
    Goose,
    KimiCli,
    QwenCli,
    Kilo,
    Pi,
    Antigravity,
}

// ============================================================================
// Notification Helper Enum
// ============================================================================

/// Desktop notification helper utilities.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum NotificationHelper {
    TerminalNotifier,
    Alerter,
    SnoreToast,
    BurntToast,
    Dunstify,
    NotifySend,
}

// ============================================================================
// Test Runner Enum
// ============================================================================

/// Test runners and testing frameworks.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    EnumCount,
    IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum TestRunner {
    // Rust
    CargoTest,
    Nextest,
    // Go
    GoTest,
    Gotestsum,
    Ginkgo,
    // JS/TS
    Vitest,
    Jest,
    Mocha,
    Ava,
    NodeTest,
    Jasmine,
    NodeTap,
    Uvu,
    // Python
    Pytest,
    Unittest,
    Nose2,
    Tox,
    Nox,
    // PHP
    PhpUnit,
    Pest,
    Codeception,
    Behat,
    Atoum,
    // Ruby
    RSpec,
    Minitest,
    TestUnit,
    // JVM
    JUnit5,
    JUnit4,
    TestNg,
    // .NET
    XUnit,
    NUnit,
    MsTest,
    // Elixir
    ExUnit,
    ESpec,
}
