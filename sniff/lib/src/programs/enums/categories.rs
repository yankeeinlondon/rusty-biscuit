//! Program category enums with strum derives.
//!
//! This module defines enums for each program category. Each enum carries
//! metadata lookup support via the `ProgramMetadata` trait from schema.rs.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use strum::{Display, EnumCount, EnumIter, EnumString, IntoStaticStr};

use crate::os::OsType;

pub(crate) static ALL_OS: &[OsType] = &[OsType::MacOS, OsType::Linux, OsType::Windows];
pub(crate) static UNIX_ONLY: &[OsType] = &[OsType::MacOS, OsType::Linux];
pub(crate) static MACOS_ONLY: &[OsType] = &[OsType::MacOS];
pub(crate) static LINUX_ONLY: &[OsType] = &[OsType::Linux];
pub(crate) static WINDOWS_ONLY: &[OsType] = &[OsType::Windows];

/// Trait bridging category enums to the generic `CategoryDetector<E>`.
///
/// Implementors provide category-level metadata and variant indexing
/// that enables a single generic detector struct to work across all
/// program categories.
pub trait CategoryEnum:
    ProgramMetadata
    + strum::IntoEnumIterator
    + strum::EnumCount
    + Copy
    + Clone
    + Eq
    + Hash
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
    fn platform_override(
        &self,
    ) -> Option<(std::path::PathBuf, crate::programs::types::ExecutableSource)> {
        None
    }
}

use crate::programs::schema::ProgramMetadata;

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
