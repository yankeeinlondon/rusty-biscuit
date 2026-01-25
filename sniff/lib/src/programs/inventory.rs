//! Program inventory with metadata lookup.
//!
//! This module provides the `Program` enum containing all known programs
//! and the `PROGRAM_LOOKUP` static map for accessing their `ProgramDetails`.

use std::{collections::HashMap, sync::LazyLock};

use serde::{Deserialize, Serialize};

use crate::os::OsType;
use crate::programs::types::{InstallationMethod, ProgramDetails};

/// An inventory of programs which this library is aware of and
/// has metadata for.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Program {
    // Editors
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

    // Utilities
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

    // Language Package Managers
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

    // OS Package Managers
    Apt,
    Nala,
    Brew,
    Dnf,
    Pacman,
    Winget,
    Chocolatey,
    Scoop,
    Nix,

    // TTS Clients
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

    // Headless Audio
    Mpv,
    Ffplay,
    Vlc,
    MPlayer,
    GstreamerGstPlay,
    Sox,
    Mpg123,
    Ogg123,
    AlsaAplay,
    PulseaudioPaplay,
    Pipewire,

    // Terminal Apps
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
// Installation method arrays (static for use in ProgramDetails)
// ============================================================================

// Editors
static VIM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vim"),
    InstallationMethod::Apt("vim"),
    InstallationMethod::Dnf("vim"),
    InstallationMethod::Pacman("vim"),
    InstallationMethod::Chocolatey("vim"),
    InstallationMethod::Scoop("vim"),
];

static NEOVIM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("neovim"),
    InstallationMethod::Apt("neovim"),
    InstallationMethod::Dnf("neovim"),
    InstallationMethod::Pacman("neovim"),
    InstallationMethod::Chocolatey("neovim"),
    InstallationMethod::Scoop("neovim"),
    InstallationMethod::Cargo("neovim"),
];

static HELIX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("helix"),
    InstallationMethod::Cargo("helix"),
    InstallationMethod::Pacman("helix"),
    InstallationMethod::Scoop("helix"),
];

static VSCODE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("visual-studio-code"),
    InstallationMethod::Chocolatey("vscode"),
    InstallationMethod::Scoop("vscode"),
    InstallationMethod::Winget("Microsoft.VisualStudioCode"),
];

static ZED_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("zed"),
];

// Utilities
static RIPGREP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("ripgrep"),
    InstallationMethod::Cargo("ripgrep"),
    InstallationMethod::Apt("ripgrep"),
    InstallationMethod::Dnf("ripgrep"),
    InstallationMethod::Pacman("ripgrep"),
    InstallationMethod::Chocolatey("ripgrep"),
    InstallationMethod::Scoop("ripgrep"),
];

static BAT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("bat"),
    InstallationMethod::Cargo("bat"),
    InstallationMethod::Apt("bat"),
    InstallationMethod::Dnf("bat"),
    InstallationMethod::Pacman("bat"),
    InstallationMethod::Chocolatey("bat"),
    InstallationMethod::Scoop("bat"),
];

static FD_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("fd"),
    InstallationMethod::Cargo("fd-find"),
    InstallationMethod::Apt("fd-find"),
    InstallationMethod::Dnf("fd-find"),
    InstallationMethod::Pacman("fd"),
    InstallationMethod::Chocolatey("fd"),
    InstallationMethod::Scoop("fd"),
];

static FZF_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("fzf"),
    InstallationMethod::Apt("fzf"),
    InstallationMethod::Dnf("fzf"),
    InstallationMethod::Pacman("fzf"),
    InstallationMethod::Chocolatey("fzf"),
    InstallationMethod::Scoop("fzf"),
];

static EZA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("eza"),
    InstallationMethod::Cargo("eza"),
    InstallationMethod::Pacman("eza"),
];

static JQ_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("jq"),
    InstallationMethod::Apt("jq"),
    InstallationMethod::Dnf("jq"),
    InstallationMethod::Pacman("jq"),
    InstallationMethod::Chocolatey("jq"),
    InstallationMethod::Scoop("jq"),
];

static GH_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("gh"),
    InstallationMethod::Apt("gh"),
    InstallationMethod::Dnf("gh"),
    InstallationMethod::Pacman("github-cli"),
    InstallationMethod::Chocolatey("gh"),
    InstallationMethod::Scoop("gh"),
    InstallationMethod::Winget("GitHub.cli"),
];

static LAZYGIT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("lazygit"),
    InstallationMethod::Pacman("lazygit"),
    InstallationMethod::Scoop("lazygit"),
    InstallationMethod::GoModules("github.com/jesseduffield/lazygit@latest"),
];

static DELTA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("git-delta"),
    InstallationMethod::Cargo("git-delta"),
    InstallationMethod::Pacman("git-delta"),
    InstallationMethod::Chocolatey("delta"),
    InstallationMethod::Scoop("delta"),
];

static STARSHIP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("starship"),
    InstallationMethod::Cargo("starship"),
    InstallationMethod::Chocolatey("starship"),
    InstallationMethod::Scoop("starship"),
    InstallationMethod::RemoteBash("https://starship.rs/install.sh"),
];

// Package Managers
static BREW_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::RemoteBash("https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh"),
];

static CARGO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::RemoteBash("https://sh.rustup.rs"),
];

static NPM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("node"),
    InstallationMethod::Apt("nodejs"),
    InstallationMethod::Dnf("nodejs"),
    InstallationMethod::Pacman("nodejs"),
    InstallationMethod::Chocolatey("nodejs"),
    InstallationMethod::Scoop("nodejs"),
];

static PNPM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("pnpm"),
    InstallationMethod::Npm("pnpm"),
    InstallationMethod::RemoteBash("https://get.pnpm.io/install.sh"),
];

static UV_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("uv"),
    InstallationMethod::Cargo("uv"),
    InstallationMethod::Pip("uv"),
    InstallationMethod::RemoteBash("https://astral.sh/uv/install.sh"),
];

static PIP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("python3-pip"),
    InstallationMethod::Dnf("python3-pip"),
    InstallationMethod::Pacman("python-pip"),
];

// TTS Clients
static ESPEAK_NG_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("espeak-ng"),
    InstallationMethod::Apt("espeak-ng"),
    InstallationMethod::Dnf("espeak-ng"),
    InstallationMethod::Pacman("espeak-ng"),
];

static PIPER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("piper"),
    InstallationMethod::Pip("piper-tts"),
];

static SHERPA_ONNX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Pip("sherpa-onnx"),
];

// Audio Players
static MPV_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mpv"),
    InstallationMethod::Apt("mpv"),
    InstallationMethod::Dnf("mpv"),
    InstallationMethod::Pacman("mpv"),
    InstallationMethod::Chocolatey("mpv"),
    InstallationMethod::Scoop("mpv"),
];

static FFPLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("ffmpeg"),
    InstallationMethod::Apt("ffmpeg"),
    InstallationMethod::Dnf("ffmpeg"),
    InstallationMethod::Pacman("ffmpeg"),
    InstallationMethod::Chocolatey("ffmpeg"),
    InstallationMethod::Scoop("ffmpeg"),
];

static SOX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("sox"),
    InstallationMethod::Apt("sox"),
    InstallationMethod::Dnf("sox"),
    InstallationMethod::Pacman("sox"),
    InstallationMethod::Chocolatey("sox"),
];

// Terminal Apps (detection only - no install methods for GUI apps)
static ALACRITTY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("alacritty"),
    InstallationMethod::Cargo("alacritty"),
    InstallationMethod::Pacman("alacritty"),
    InstallationMethod::Scoop("alacritty"),
];

static KITTY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("kitty"),
    InstallationMethod::Apt("kitty"),
    InstallationMethod::Dnf("kitty"),
    InstallationMethod::Pacman("kitty"),
];

static WEZTERM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("wezterm"),
    InstallationMethod::Chocolatey("wezterm"),
    InstallationMethod::Scoop("wezterm"),
];

// ============================================================================
// OS Availability arrays
// ============================================================================

static ALL_OS: &[OsType] = &[OsType::MacOS, OsType::Linux, OsType::Windows];
static UNIX_ONLY: &[OsType] = &[OsType::MacOS, OsType::Linux];
static MACOS_ONLY: &[OsType] = &[OsType::MacOS];
static LINUX_ONLY: &[OsType] = &[OsType::Linux];
static WINDOWS_ONLY: &[OsType] = &[OsType::Windows];

/// Metadata lookup for known programs.
///
/// This map is populated lazily on first access and provides `ProgramDetails`
/// for programs that have installation methods defined.
///
/// ## Notes
///
/// Not all programs in the `Program` enum have entries here. Programs are
/// added incrementally as installation methods are researched and verified.
pub static PROGRAM_LOOKUP: LazyLock<HashMap<Program, ProgramDetails>> = LazyLock::new(|| {
    let mut lookup = HashMap::new();

    // ========================================================================
    // Editors (5 entries)
    // ========================================================================
    lookup.insert(Program::Vim, ProgramDetails::full(
        "Vim",
        "Vi IMproved text editor",
        ALL_OS,
        "https://www.vim.org/",
        Some("https://github.com/vim/vim"),
        VIM_INSTALL,
    ));

    lookup.insert(Program::Neovim, ProgramDetails::full(
        "Neovim",
        "Hyperextensible Vim-based text editor",
        ALL_OS,
        "https://neovim.io/",
        Some("https://github.com/neovim/neovim"),
        NEOVIM_INSTALL,
    ));

    lookup.insert(Program::Helix, ProgramDetails::full(
        "Helix",
        "Post-modern modal text editor",
        ALL_OS,
        "https://helix-editor.com/",
        Some("https://github.com/helix-editor/helix"),
        HELIX_INSTALL,
    ));

    lookup.insert(Program::VSCode, ProgramDetails::full(
        "Visual Studio Code",
        "Code editor for modern web and cloud applications",
        ALL_OS,
        "https://code.visualstudio.com/",
        Some("https://github.com/microsoft/vscode"),
        VSCODE_INSTALL,
    ));

    lookup.insert(Program::Zed, ProgramDetails::full(
        "Zed",
        "High-performance multiplayer code editor",
        MACOS_ONLY,
        "https://zed.dev/",
        Some("https://github.com/zed-industries/zed"),
        ZED_INSTALL,
    ));

    // ========================================================================
    // Utilities (10 entries)
    // ========================================================================
    lookup.insert(Program::Ripgrep, ProgramDetails::full(
        "ripgrep",
        "Fast grep alternative with smart defaults",
        ALL_OS,
        "https://github.com/BurntSushi/ripgrep",
        Some("https://github.com/BurntSushi/ripgrep"),
        RIPGREP_INSTALL,
    ));

    lookup.insert(Program::Bat, ProgramDetails::full(
        "bat",
        "A cat clone with syntax highlighting",
        ALL_OS,
        "https://github.com/sharkdp/bat",
        Some("https://github.com/sharkdp/bat"),
        BAT_INSTALL,
    ));

    lookup.insert(Program::Fd, ProgramDetails::full(
        "fd",
        "Simple, fast alternative to find",
        ALL_OS,
        "https://github.com/sharkdp/fd",
        Some("https://github.com/sharkdp/fd"),
        FD_INSTALL,
    ));

    lookup.insert(Program::Fzf, ProgramDetails::full(
        "fzf",
        "Command-line fuzzy finder",
        ALL_OS,
        "https://github.com/junegunn/fzf",
        Some("https://github.com/junegunn/fzf"),
        FZF_INSTALL,
    ));

    lookup.insert(Program::Eza, ProgramDetails::full(
        "eza",
        "A modern replacement for ls",
        ALL_OS,
        "https://eza.rocks/",
        Some("https://github.com/eza-community/eza"),
        EZA_INSTALL,
    ));

    lookup.insert(Program::Jq, ProgramDetails::full(
        "jq",
        "Command-line JSON processor",
        ALL_OS,
        "https://jqlang.github.io/jq/",
        Some("https://github.com/jqlang/jq"),
        JQ_INSTALL,
    ));

    lookup.insert(Program::Gh, ProgramDetails::full(
        "GitHub CLI",
        "GitHub's official CLI",
        ALL_OS,
        "https://cli.github.com/",
        Some("https://github.com/cli/cli"),
        GH_INSTALL,
    ));

    lookup.insert(Program::Lazygit, ProgramDetails::full(
        "lazygit",
        "Simple terminal UI for git commands",
        ALL_OS,
        "https://github.com/jesseduffield/lazygit",
        Some("https://github.com/jesseduffield/lazygit"),
        LAZYGIT_INSTALL,
    ));

    lookup.insert(Program::Delta, ProgramDetails::full(
        "delta",
        "Viewer for git and diff output",
        ALL_OS,
        "https://github.com/dandavison/delta",
        Some("https://github.com/dandavison/delta"),
        DELTA_INSTALL,
    ));

    lookup.insert(Program::Starship, ProgramDetails::full(
        "Starship",
        "Minimal, blazing-fast shell prompt",
        ALL_OS,
        "https://starship.rs/",
        Some("https://github.com/starship/starship"),
        STARSHIP_INSTALL,
    ));

    // ========================================================================
    // Package Managers (6 entries)
    // ========================================================================
    lookup.insert(Program::Brew, ProgramDetails::full(
        "Homebrew",
        "macOS/Linux community package manager",
        UNIX_ONLY,
        "https://brew.sh/",
        Some("https://github.com/Homebrew/brew"),
        BREW_INSTALL,
    ));

    lookup.insert(Program::Cargo, ProgramDetails::full(
        "Cargo",
        "Rust package manager and build tool",
        ALL_OS,
        "https://doc.rust-lang.org/cargo/",
        Some("https://github.com/rust-lang/cargo"),
        CARGO_INSTALL,
    ));

    lookup.insert(Program::Npm, ProgramDetails::full(
        "npm",
        "Node.js package manager",
        ALL_OS,
        "https://www.npmjs.com/",
        Some("https://github.com/npm/cli"),
        NPM_INSTALL,
    ));

    lookup.insert(Program::Pnpm, ProgramDetails::full(
        "pnpm",
        "Fast, disk-efficient package manager",
        ALL_OS,
        "https://pnpm.io/",
        Some("https://github.com/pnpm/pnpm"),
        PNPM_INSTALL,
    ));

    lookup.insert(Program::Pip, ProgramDetails::full(
        "pip",
        "Python package installer",
        ALL_OS,
        "https://pip.pypa.io/",
        Some("https://github.com/pypa/pip"),
        PIP_INSTALL,
    ));

    lookup.insert(Program::Uv, ProgramDetails::full(
        "uv",
        "Fast Python package manager",
        ALL_OS,
        "https://astral.sh/uv",
        Some("https://github.com/astral-sh/uv"),
        UV_INSTALL,
    ));

    // ========================================================================
    // TTS Clients (4 entries)
    // ========================================================================
    lookup.insert(Program::Say, ProgramDetails::full(
        "say",
        "macOS built-in speech synthesis",
        MACOS_ONLY,
        "https://ss64.com/osx/say.html",
        None,
        &[], // Built-in, no install needed
    ));

    lookup.insert(Program::EspeakNg, ProgramDetails::full(
        "eSpeak NG",
        "Multi-lingual speech synthesizer",
        ALL_OS,
        "https://github.com/espeak-ng/espeak-ng",
        Some("https://github.com/espeak-ng/espeak-ng"),
        ESPEAK_NG_INSTALL,
    ));

    lookup.insert(Program::Piper, ProgramDetails::full(
        "Piper",
        "Fast local neural TTS using ONNX",
        ALL_OS,
        "https://github.com/rhasspy/piper",
        Some("https://github.com/rhasspy/piper"),
        PIPER_INSTALL,
    ));

    lookup.insert(Program::SherpaOnnx, ProgramDetails::full(
        "Sherpa-ONNX",
        "Streaming/non-streaming TTS using ONNX",
        ALL_OS,
        "https://k2-fsa.github.io/sherpa/onnx/",
        Some("https://github.com/k2-fsa/sherpa-onnx"),
        SHERPA_ONNX_INSTALL,
    ));

    // ========================================================================
    // Audio Players (3 entries)
    // ========================================================================
    lookup.insert(Program::Mpv, ProgramDetails::full(
        "mpv",
        "CLI media player for audio-only playback",
        ALL_OS,
        "https://mpv.io/",
        Some("https://github.com/mpv-player/mpv"),
        MPV_INSTALL,
    ));

    lookup.insert(Program::Ffplay, ProgramDetails::full(
        "FFplay",
        "Minimal CLI player shipped with FFmpeg",
        ALL_OS,
        "https://www.ffmpeg.org/ffplay.html",
        Some("https://github.com/FFmpeg/FFmpeg"),
        FFPLAY_INSTALL,
    ));

    lookup.insert(Program::Sox, ProgramDetails::full(
        "SoX play",
        "Swiss-army knife for audio playback",
        ALL_OS,
        "https://sox.sourceforge.net/",
        Some("https://sourceforge.net/projects/sox/"),
        SOX_INSTALL,
    ));

    // ========================================================================
    // Terminal Apps (3 entries - detection only for most)
    // ========================================================================
    lookup.insert(Program::Alacritty, ProgramDetails::full(
        "Alacritty",
        "Fast, GPU-accelerated terminal emulator",
        ALL_OS,
        "https://alacritty.org/",
        Some("https://github.com/alacritty/alacritty"),
        ALACRITTY_INSTALL,
    ));

    lookup.insert(Program::Kitty, ProgramDetails::full(
        "kitty",
        "Fast, feature-rich, GPU-based terminal",
        UNIX_ONLY,
        "https://sw.kovidgoyal.net/kitty/",
        Some("https://github.com/kovidgoyal/kitty"),
        KITTY_INSTALL,
    ));

    lookup.insert(Program::WezTerm, ProgramDetails::full(
        "WezTerm",
        "GPU-accelerated terminal emulator and multiplexer",
        ALL_OS,
        "https://wezfurlong.org/wezterm/",
        Some("https://github.com/wez/wezterm"),
        WEZTERM_INSTALL,
    ));

    lookup
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_lookup_has_entries() {
        let lookup = &*PROGRAM_LOOKUP;
        assert!(lookup.len() >= 30, "Expected at least 30 programs, got {}", lookup.len());
    }

    #[test]
    fn test_program_lookup_vim_has_details() {
        let details = PROGRAM_LOOKUP.get(&Program::Vim).expect("Vim should be in lookup");
        assert_eq!(details.name, "Vim");
        assert!(!details.installation_methods.is_empty());
    }

    #[test]
    fn test_program_lookup_ripgrep_has_cargo() {
        let details = PROGRAM_LOOKUP.get(&Program::Ripgrep).expect("Ripgrep should be in lookup");
        assert!(
            details.installation_methods.iter().any(|m| matches!(m, InstallationMethod::Cargo(_))),
            "Ripgrep should have Cargo installation method"
        );
    }

    #[test]
    fn test_program_copy_derive() {
        let p1 = Program::Vim;
        let p2 = p1; // Copy
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_all_programs_in_lookup_have_valid_details() {
        for (program, details) in PROGRAM_LOOKUP.iter() {
            assert!(!details.name.is_empty(), "{:?} has empty name", program);
            assert!(!details.description.is_empty(), "{:?} has empty description", program);
            assert!(!details.website.is_empty(), "{:?} has empty website", program);
            assert!(!details.os_availability.is_empty(), "{:?} has no OS availability", program);
        }
    }
}
