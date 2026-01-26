//! Program enums with strum derives for metadata and iteration.
//!
//! This module defines enums for each program category with full metadata
//! lookup support via the `ProgramMetadata` trait.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumCount, EnumIter, EnumString, IntoStaticStr};

use super::schema::{ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy};

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

/// Metadata lookup table for editors.
static EDITOR_INFO: &[ProgramInfo] = &[
    ProgramInfo::standard("vi", "Vi", "The classic vi editor", "https://www.vim.org/"),
    ProgramInfo::standard(
        "vim",
        "Vim",
        "Vi IMproved text editor",
        "https://www.vim.org/",
    ),
    ProgramInfo::standard(
        "nvim",
        "Neovim",
        "Hyperextensible Vim-based text editor",
        "https://neovim.io/",
    ),
    ProgramInfo::with_prefix(
        "emacs",
        "GNU Emacs",
        "Extensible, customizable text editor",
        "https://www.gnu.org/software/emacs/",
        "GNU Emacs ",
    ),
    ProgramInfo::standard(
        "xemacs",
        "XEmacs",
        "A version of Emacs that branched from GNU Emacs",
        "http://www.xemacs.org/",
    ),
    ProgramInfo::with_prefix(
        "nano",
        "GNU nano",
        "Small and friendly text editor",
        "https://www.nano-editor.org/",
        "nano ",
    ),
    ProgramInfo::standard(
        "hx",
        "Helix",
        "Post-modern modal text editor",
        "https://helix-editor.com/",
    ),
    ProgramInfo::standard(
        "code",
        "Visual Studio Code",
        "Code editor for modern web and cloud applications",
        "https://code.visualstudio.com/",
    ),
    ProgramInfo::standard(
        "codium",
        "VSCodium",
        "Free/libre open source binaries of VS Code",
        "https://vscodium.com/",
    ),
    ProgramInfo::standard(
        "subl",
        "Sublime Text",
        "Sophisticated text editor for code and prose",
        "https://www.sublimetext.com/",
    ),
    ProgramInfo::standard(
        "zed",
        "Zed",
        "High-performance multiplayer code editor",
        "https://zed.dev/",
    ),
    ProgramInfo::standard(
        "micro",
        "Micro",
        "Modern and intuitive terminal-based text editor",
        "https://micro-editor.github.io/",
    ),
    ProgramInfo::standard(
        "kak",
        "Kakoune",
        "Modal editor with selection-based editing model",
        "https://kakoune.org/",
    ),
    ProgramInfo::standard(
        "amp",
        "Amp",
        "Modal text editor for the terminal inspired by Vi",
        "https://amp.readme.io/",
    ),
    ProgramInfo::standard(
        "lapce",
        "Lapce",
        "Lightning-fast code editor written in Rust",
        "https://lapce.dev/",
    ),
    ProgramInfo::standard(
        "phpstorm",
        "PhpStorm",
        "Lightning-smart PHP IDE by JetBrains",
        "https://www.jetbrains.com/phpstorm/",
    ),
    ProgramInfo::standard(
        "idea",
        "IntelliJ IDEA",
        "Capable and ergonomic IDE for JVM-based languages",
        "https://www.jetbrains.com/idea/",
    ),
    ProgramInfo::standard(
        "pycharm",
        "PyCharm",
        "The Python IDE for professional developers",
        "https://www.jetbrains.com/pycharm/",
    ),
    ProgramInfo::standard(
        "webstorm",
        "WebStorm",
        "The smartest JavaScript IDE",
        "https://www.jetbrains.com/webstorm/",
    ),
    ProgramInfo::standard(
        "clion",
        "CLion",
        "Cross-platform C and C++ IDE",
        "https://www.jetbrains.com/clion/",
    ),
    ProgramInfo::standard(
        "goland",
        "GoLand",
        "Cross-platform Go IDE",
        "https://www.jetbrains.com/go/",
    ),
    ProgramInfo::standard(
        "rider",
        "Rider",
        "Fast and powerful cross-platform .NET IDE",
        "https://www.jetbrains.com/rider/",
    ),
    ProgramInfo::standard(
        "mate",
        "TextMate",
        "Versatile plain text editor for macOS",
        "https://macromates.com/",
    ),
    ProgramInfo::standard(
        "bbedit",
        "BBEdit",
        "Professional HTML and text editor for macOS",
        "https://www.barebones.com/products/bbedit/",
    ),
    ProgramInfo::standard(
        "geany",
        "Geany",
        "Powerful, stable and lightweight text editor",
        "https://www.geany.org/",
    ),
    ProgramInfo::standard(
        "kate",
        "Kate",
        "Multi-document, multi-view text editor by KDE",
        "https://kate-editor.org/",
    ),
];

impl ProgramMetadata for Editor {
    fn info(&self) -> &'static ProgramInfo {
        &EDITOR_INFO[*self as usize]
    }
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

/// Metadata lookup table for utilities.
static UTILITY_INFO: &[ProgramInfo] = &[
    ProgramInfo::standard(
        "exa",
        "exa",
        "A modern replacement for ls (deprecated)",
        "https://the.exa.website/",
    ),
    ProgramInfo::standard(
        "eza",
        "eza",
        "A modern replacement for ls",
        "https://eza.rocks/",
    ),
    ProgramInfo::standard(
        "rg",
        "ripgrep",
        "Fast grep alternative with smart defaults",
        "https://github.com/BurntSushi/ripgrep",
    ),
    ProgramInfo::standard(
        "dust",
        "dust",
        "A more intuitive version of du",
        "https://github.com/bootandy/dust",
    ),
    ProgramInfo::standard(
        "bat",
        "bat",
        "A cat clone with syntax highlighting",
        "https://github.com/sharkdp/bat",
    ),
    ProgramInfo::standard(
        "fd",
        "fd",
        "Simple, fast alternative to find",
        "https://github.com/sharkdp/fd",
    ),
    ProgramInfo::standard(
        "procs",
        "procs",
        "A modern replacement for ps",
        "https://github.com/dalance/procs",
    ),
    ProgramInfo::standard(
        "btm",
        "bottom",
        "Cross-platform graphical process monitor",
        "https://github.com/ClementTsang/bottom",
    ),
    ProgramInfo::standard(
        "fzf",
        "fzf",
        "Command-line fuzzy finder",
        "https://github.com/junegunn/fzf",
    ),
    ProgramInfo::standard(
        "zoxide",
        "zoxide",
        "Smarter cd command",
        "https://github.com/ajeetdsouza/zoxide",
    ),
    ProgramInfo::standard(
        "starship",
        "Starship",
        "Minimal, blazing-fast shell prompt",
        "https://starship.rs/",
    ),
    ProgramInfo::standard(
        "direnv",
        "direnv",
        "Environment switcher for the shell",
        "https://direnv.net/",
    ),
    ProgramInfo::standard(
        "jq",
        "jq",
        "Command-line JSON processor",
        "https://jqlang.github.io/jq/",
    ),
    ProgramInfo::standard(
        "delta",
        "delta",
        "Viewer for git and diff output",
        "https://github.com/dandavison/delta",
    ),
    ProgramInfo::standard(
        "tldr",
        "tealdeer",
        "Fast tldr client for simplified man pages",
        "https://github.com/dbrgn/tealdeer",
    ),
    ProgramInfo::standard(
        "lazygit",
        "lazygit",
        "Simple terminal UI for git commands",
        "https://github.com/jesseduffield/lazygit",
    ),
    ProgramInfo::standard(
        "gh",
        "GitHub CLI",
        "GitHub's official CLI",
        "https://cli.github.com/",
    ),
    ProgramInfo::standard(
        "htop",
        "htop",
        "Interactive process viewer",
        "https://htop.dev/",
    ),
    ProgramInfo::standard(
        "btop",
        "btop",
        "Resource monitor with CPU, memory, disk, network stats",
        "https://github.com/aristocratos/btop",
    ),
    ProgramInfo::standard(
        "tmux",
        "tmux",
        "Terminal multiplexer",
        "https://github.com/tmux/tmux/wiki",
    ),
    ProgramInfo::standard(
        "zellij",
        "Zellij",
        "Modern terminal multiplexer",
        "https://zellij.dev/",
    ),
    ProgramInfo::standard(
        "http",
        "HTTPie",
        "User-friendly HTTP client",
        "https://httpie.io/",
    ),
    ProgramInfo::standard(
        "curlie",
        "curlie",
        "User-friendly alternative to curl",
        "https://github.com/rs/curlie",
    ),
    ProgramInfo::standard(
        "mise",
        "mise",
        "Polyglot development environment manager",
        "https://mise.jdx.dev/",
    ),
    ProgramInfo::standard(
        "hyperfine",
        "hyperfine",
        "Command-line benchmarking tool",
        "https://github.com/sharkdp/hyperfine",
    ),
    ProgramInfo::standard(
        "tokei",
        "tokei",
        "Count lines of code quickly",
        "https://github.com/XAMPPRocky/tokei",
    ),
    ProgramInfo::standard(
        "xh",
        "xh",
        "Friendly and fast HTTP client",
        "https://github.com/ducaale/xh",
    ),
    ProgramInfo::standard(
        "curl",
        "curl",
        "Transfer data with URLs",
        "https://curl.se/",
    ),
    ProgramInfo::standard(
        "wget",
        "wget",
        "Network utility to retrieve content from web servers",
        "https://www.gnu.org/software/wget/",
    ),
    ProgramInfo::standard(
        "iperf3",
        "iperf3",
        "Network bandwidth measurement tool",
        "https://iperf.fr/",
    ),
];

impl ProgramMetadata for Utility {
    fn info(&self) -> &'static ProgramInfo {
        &UTILITY_INFO[*self as usize]
    }
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

/// Metadata lookup table for language package managers.
static LANG_PKG_MGR_INFO: &[ProgramInfo] = &[
    ProgramInfo::standard(
        "npm",
        "npm",
        "Node.js package manager",
        "https://www.npmjs.com/",
    ),
    ProgramInfo::standard(
        "pnpm",
        "pnpm",
        "Fast, disk-efficient package manager",
        "https://pnpm.io/",
    ),
    ProgramInfo::standard(
        "yarn",
        "Yarn",
        "Alternative Node.js package manager",
        "https://yarnpkg.com/",
    ),
    ProgramInfo::standard(
        "bun",
        "Bun",
        "All-in-one JS runtime with package manager",
        "https://bun.sh/",
    ),
    ProgramInfo::standard(
        "cargo",
        "Cargo",
        "Rust package manager and build tool",
        "https://doc.rust-lang.org/cargo/",
    ),
    ProgramInfo::with_prefix(
        "go",
        "Go Modules",
        "Built-in Go dependency system",
        "https://go.dev/ref/mod",
        "go version ",
    ),
    ProgramInfo::with_prefix(
        "composer",
        "Composer",
        "PHP dependency manager",
        "https://getcomposer.org/",
        "Composer version ",
    ),
    ProgramInfo::standard(
        "swift",
        "Swift Package Manager",
        "Swift dependency manager",
        "https://www.swift.org/package-manager/",
    ),
    ProgramInfo::standard(
        "luarocks",
        "LuaRocks",
        "Package manager for Lua modules",
        "https://luarocks.org/",
    ),
    ProgramInfo::standard(
        "vcpkg",
        "vcpkg",
        "C/C++ dependency manager by Microsoft",
        "https://vcpkg.io/",
    ),
    ProgramInfo::standard(
        "conan",
        "Conan",
        "Decentralized C/C++ package manager",
        "https://conan.io/",
    ),
    ProgramInfo::standard(
        "nuget",
        "NuGet",
        ".NET package manager",
        "https://www.nuget.org/",
    ),
    ProgramInfo::standard(
        "mix",
        "Hex",
        "Package manager for BEAM ecosystem",
        "https://hex.pm/",
    ),
    ProgramInfo::standard(
        "pip",
        "pip",
        "Python package installer",
        "https://pip.pypa.io/",
    ),
    ProgramInfo::standard(
        "uv",
        "uv",
        "Fast Python package manager",
        "https://astral.sh/uv",
    ),
    ProgramInfo::standard(
        "poetry",
        "Poetry",
        "Python dependency manager with lockfiles",
        "https://python-poetry.org/",
    ),
    ProgramInfo::standard(
        "cpan",
        "CPAN",
        "Perl module archive",
        "https://www.cpan.org/",
    ),
    ProgramInfo::standard(
        "cpanm",
        "cpanminus",
        "Lightweight CPAN client",
        "https://metacpan.org/pod/App::cpanminus",
    ),
];

impl ProgramMetadata for LanguagePackageManager {
    fn info(&self) -> &'static ProgramInfo {
        &LANG_PKG_MGR_INFO[*self as usize]
    }
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

/// Metadata lookup table for OS package managers.
static OS_PKG_MGR_INFO: &[ProgramInfo] = &[
    ProgramInfo::standard(
        "apt",
        "APT",
        "Debian/Ubuntu package manager",
        "https://tracker.debian.org/pkg/apt",
    ),
    ProgramInfo::standard(
        "nala",
        "Nala",
        "Modern apt frontend with parallel downloads",
        "https://github.com/volitank/nala",
    ),
    ProgramInfo::standard(
        "brew",
        "Homebrew",
        "macOS/Linux community package manager",
        "https://brew.sh/",
    ),
    ProgramInfo::standard(
        "dnf",
        "DNF",
        "Fedora/RHEL package manager",
        "https://github.com/rpm-software-management/dnf",
    ),
    ProgramInfo {
        binary_name: "pacman",
        display_name: "Pacman",
        description: "Arch Linux package manager",
        website: "https://archlinux.org/pacman/",
        version_flag: VersionFlag::ShortUpper,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
    },
    ProgramInfo::standard(
        "winget",
        "winget",
        "Windows Package Manager",
        "https://github.com/microsoft/winget-cli",
    ),
    ProgramInfo::standard(
        "choco",
        "Chocolatey",
        "Windows community package manager",
        "https://chocolatey.org/",
    ),
    ProgramInfo::standard(
        "scoop",
        "Scoop",
        "Windows command-line installer",
        "https://scoop.sh/",
    ),
    ProgramInfo::standard("nix", "Nix", "Nix package manager", "https://nixos.org/"),
];

impl ProgramMetadata for OsPackageManager {
    fn info(&self) -> &'static ProgramInfo {
        &OS_PKG_MGR_INFO[*self as usize]
    }
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

/// Metadata lookup table for TTS clients.
static TTS_CLIENT_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "say",
        display_name: "say",
        description: "macOS built-in speech synthesis",
        website: "https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/SpeechSynthesisProgrammingGuide/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
    },
    ProgramInfo::standard(
        "espeak",
        "eSpeak",
        "Open source speech synthesizer",
        "http://espeak.sourceforge.net/",
    ),
    ProgramInfo::standard(
        "espeak-ng",
        "eSpeak NG",
        "Multi-lingual speech synthesizer",
        "https://github.com/espeak-ng/espeak-ng",
    ),
    ProgramInfo::standard(
        "festival",
        "Festival",
        "General multi-lingual speech synthesis",
        "http://www.cstr.ed.ac.uk/projects/festival/",
    ),
    ProgramInfo::standard(
        "mimic",
        "Mimic",
        "Mycroft's TTS engine based on Flite",
        "https://github.com/MycroftAI/mimic",
    ),
    ProgramInfo::standard(
        "mimic3",
        "Mimic 3",
        "Mycroft's neural TTS engine",
        "https://github.com/MycroftAI/mycroft-mimic3-tts",
    ),
    ProgramInfo::standard(
        "piper",
        "Piper",
        "Fast local neural TTS using ONNX",
        "https://github.com/rhasspy/piper",
    ),
    ProgramInfo::standard(
        "echogarden",
        "Echogarden",
        "Speech processing engine",
        "https://echogarden.io/",
    ),
    ProgramInfo::standard(
        "balcon",
        "Balcon",
        "Command line TTS utility for Windows",
        "http://www.cross-plus-a.com/balcon.htm",
    ),
    ProgramInfo {
        binary_name: "sapi",
        display_name: "Windows SAPI",
        description: "Windows Speech API",
        website: "https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms723627(v=vs.85)",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
    },
    ProgramInfo::standard(
        "gtts-cli",
        "gTTS",
        "Google Text-to-Speech CLI tool",
        "https://github.com/pndurette/gTTS",
    ),
    ProgramInfo::standard(
        "tts",
        "Coqui TTS",
        "Deep learning for Text-to-Speech",
        "https://github.com/coqui-ai/TTS",
    ),
    ProgramInfo::standard(
        "sherpa-onnx-offline-tts",
        "Sherpa-ONNX",
        "Streaming/non-streaming TTS using ONNX",
        "https://k2-fsa.github.io/sherpa/onnx/",
    ),
    ProgramInfo::standard(
        "kokoro-tts",
        "Kokoro TTS",
        "High-quality neural TTS using Kokoro-82M model",
        "https://github.com/nazdridoy/kokoro-tts",
    ),
    ProgramInfo::standard(
        "pico2wave",
        "SVOX Pico",
        "Lightweight TTS for embedded systems",
        "https://github.com/naggety/picmotts",
    ),
];

impl ProgramMetadata for TtsClient {
    fn info(&self) -> &'static ProgramInfo {
        &TTS_CLIENT_INFO[*self as usize]
    }
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

/// Metadata lookup table for terminal apps.
static TERMINAL_APP_INFO: &[ProgramInfo] = &[
    ProgramInfo::standard(
        "alacritty",
        "Alacritty",
        "Fast, GPU-accelerated terminal emulator",
        "https://alacritty.org/",
    ),
    ProgramInfo::standard(
        "kitty",
        "kitty",
        "Fast, feature-rich, GPU-based terminal",
        "https://sw.kovidgoyal.net/kitty/",
    ),
    ProgramInfo {
        binary_name: "iterm2",
        display_name: "iTerm2",
        description: "Terminal emulator for macOS",
        website: "https://iterm2.com/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
    },
    ProgramInfo::standard(
        "wezterm",
        "WezTerm",
        "GPU-accelerated terminal emulator and multiplexer",
        "https://wezfurlong.org/wezterm/",
    ),
    ProgramInfo::standard(
        "ghostty",
        "Ghostty",
        "Fast, feature-rich GPU terminal written in Zig",
        "https://ghostty.org/",
    ),
    ProgramInfo {
        binary_name: "warp-terminal",
        display_name: "Warp",
        description: "Modern, Rust-based terminal with AI",
        website: "https://www.warp.dev/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
    },
    ProgramInfo::standard(
        "rio",
        "Rio",
        "Hardware-accelerated GPU terminal emulator",
        "https://github.com/raphamorim/rio",
    ),
    ProgramInfo::standard(
        "tabby",
        "Tabby",
        "Terminal for a more modern age",
        "https://tabby.sh/",
    ),
    ProgramInfo::standard(
        "foot",
        "foot",
        "Fast, lightweight Wayland terminal emulator",
        "https://codeberg.org/dnkl/foot",
    ),
    ProgramInfo::standard(
        "gnome-terminal",
        "GNOME Terminal",
        "Default terminal for GNOME desktop",
        "https://help.gnome.org/users/gnome-terminal/stable/",
    ),
    ProgramInfo::standard(
        "konsole",
        "Konsole",
        "Terminal emulator by KDE",
        "https://konsole.kde.org/",
    ),
    ProgramInfo::standard(
        "xfce4-terminal",
        "Xfce Terminal",
        "Terminal emulator for Xfce",
        "https://docs.xfce.org/apps/xfce4-terminal/start",
    ),
    ProgramInfo::standard(
        "terminology",
        "Terminology",
        "Terminal based on Enlightenment libraries",
        "https://www.enlightenment.org/about-terminology",
    ),
    ProgramInfo {
        binary_name: "st",
        display_name: "st",
        description: "Simple terminal for X which sucks less",
        website: "https://st.suckless.org/",
        version_flag: VersionFlag::Short,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
    },
    ProgramInfo::standard(
        "xterm",
        "xterm",
        "Standard terminal for X Window System",
        "https://invisible-island.net/xterm/",
    ),
    ProgramInfo::standard(
        "hyper",
        "Hyper",
        "Terminal built on web technologies",
        "https://hyper.is/",
    ),
    ProgramInfo::standard(
        "wt",
        "Windows Terminal",
        "Modern terminal for Windows",
        "https://github.com/microsoft/terminal",
    ),
];

impl ProgramMetadata for TerminalApp {
    fn info(&self) -> &'static ProgramInfo {
        &TERMINAL_APP_INFO[*self as usize]
    }
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
    PulseaudioPaplay,
    Pipewire,
}

/// Metadata lookup table for headless audio players.
static HEADLESS_AUDIO_INFO: &[ProgramInfo] = &[
    ProgramInfo::standard(
        "mpv",
        "mpv",
        "CLI media player for audio-only playback",
        "https://mpv.io/",
    ),
    ProgramInfo::standard(
        "ffplay",
        "FFplay",
        "Minimal CLI player shipped with FFmpeg",
        "https://www.ffmpeg.org/ffplay.html",
    ),
    ProgramInfo::standard(
        "cvlc",
        "VLC",
        "Headless VLC playback via cvlc",
        "https://wiki.videolan.org/VLC_command-line_help/",
    ),
    ProgramInfo::standard(
        "mplayer",
        "MPlayer",
        "Classic CLI-oriented media player",
        "https://www.mplayerhq.hu/",
    ),
    ProgramInfo::standard(
        "gst-play-1.0",
        "GStreamer gst-play",
        "CLI front-end to GStreamer pipelines",
        "https://gstreamer.freedesktop.org/documentation/tools/gst-play-1.0.html",
    ),
    ProgramInfo::standard(
        "play",
        "SoX play",
        "Swiss-army knife for audio playback",
        "https://linux.die.net/man/1/sox",
    ),
    ProgramInfo::standard(
        "mpg123",
        "mpg123",
        "Lightweight console MP3 player",
        "https://www.mpg123.de/",
    ),
    ProgramInfo::standard(
        "ogg123",
        "ogg123",
        "CLI player for Ogg/Vorbis files",
        "https://github.com/xiph/vorbis-tools",
    ),
    ProgramInfo::standard(
        "aplay",
        "aplay",
        "ALSA low-level playback utility",
        "https://linux.die.net/man/1/aplay",
    ),
    ProgramInfo::standard(
        "paplay",
        "paplay",
        "Simple PulseAudio playback tool",
        "https://manpages.ubuntu.com/manpages/trusty/man1/paplay.1.html",
    ),
    ProgramInfo::standard(
        "pw-play",
        "PipeWire pw-play",
        "PipeWire CLI playback tool",
        "https://docs.pipewire.org/page_man_pw-cat_1.html",
    ),
];

impl ProgramMetadata for HeadlessAudio {
    fn info(&self) -> &'static ProgramInfo {
        &HEADLESS_AUDIO_INFO[*self as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_editor_count_matches_info() {
        assert_eq!(Editor::COUNT, EDITOR_INFO.len());
    }

    #[test]
    fn test_utility_count_matches_info() {
        assert_eq!(Utility::COUNT, UTILITY_INFO.len());
    }

    #[test]
    fn test_lang_pkg_mgr_count_matches_info() {
        assert_eq!(LanguagePackageManager::COUNT, LANG_PKG_MGR_INFO.len());
    }

    #[test]
    fn test_os_pkg_mgr_count_matches_info() {
        assert_eq!(OsPackageManager::COUNT, OS_PKG_MGR_INFO.len());
    }

    #[test]
    fn test_tts_client_count_matches_info() {
        assert_eq!(TtsClient::COUNT, TTS_CLIENT_INFO.len());
    }

    #[test]
    fn test_terminal_app_count_matches_info() {
        assert_eq!(TerminalApp::COUNT, TERMINAL_APP_INFO.len());
    }

    #[test]
    fn test_headless_audio_count_matches_info() {
        assert_eq!(HeadlessAudio::COUNT, HEADLESS_AUDIO_INFO.len());
    }

    #[test]
    fn test_editor_metadata_access() {
        let vim = Editor::Vim;
        assert_eq!(vim.binary_name(), "vim");
        assert_eq!(vim.display_name(), "Vim");
        assert!(vim.website().starts_with("https://"));
    }

    #[test]
    fn test_enum_iteration() {
        let count = Editor::iter().count();
        assert_eq!(count, Editor::COUNT);
    }
}
