//! Program inventory with metadata lookup.
//!
//! This module provides the `Program` enum containing all known programs
//! and the `PROGRAM_LOOKUP` static map for accessing their `ProgramDetails`.

use std::{collections::HashMap, sync::LazyLock};

use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter};

use crate::os::OsType;
use crate::programs::types::{InstallationMethod, ProgramDetails};

/// An inventory of programs which this library is aware of and
/// has metadata for.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, EnumCount)]
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

static VI_INSTALL: &[InstallationMethod] = VIM_INSTALL;

static EMACS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("emacs"),
    InstallationMethod::Apt("emacs"),
    InstallationMethod::Dnf("emacs"),
    InstallationMethod::Pacman("emacs"),
    InstallationMethod::Chocolatey("emacs"),
    InstallationMethod::Scoop("emacs"),
];

static XEMACS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("xemacs"),
    InstallationMethod::Apt("xemacs"),
    InstallationMethod::Dnf("xemacs"),
    InstallationMethod::Pacman("xemacs"),
];

static NANO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("nano"),
    InstallationMethod::Apt("nano"),
    InstallationMethod::Dnf("nano"),
    InstallationMethod::Pacman("nano"),
    InstallationMethod::Chocolatey("nano"),
    InstallationMethod::Scoop("nano"),
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

static VSCODIUM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vscodium"),
    InstallationMethod::Chocolatey("vscodium"),
    InstallationMethod::Scoop("vscodium"),
    InstallationMethod::Winget("VSCodium.VSCodium"),
];

static SUBLIME_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("sublime-text"),
    InstallationMethod::Apt("sublime-text"),
    InstallationMethod::Dnf("sublime-text"),
    InstallationMethod::Pacman("sublime-text"),
    InstallationMethod::Chocolatey("sublimetext4"),
    InstallationMethod::Scoop("sublime-text"),
    InstallationMethod::Winget("SublimeHQ.SublimeText.4"),
];

static ZED_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("zed")];

static MICRO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("micro"),
    InstallationMethod::Apt("micro"),
    InstallationMethod::Dnf("micro"),
    InstallationMethod::Pacman("micro"),
    InstallationMethod::Scoop("micro"),
];

static KAKOUNE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("kakoune"),
    InstallationMethod::Apt("kakoune"),
    InstallationMethod::Dnf("kakoune"),
    InstallationMethod::Pacman("kakoune"),
];

static AMP_INSTALL: &[InstallationMethod] = &[InstallationMethod::Cargo("amp")];

static LAPCE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("lapce"),
    InstallationMethod::Cargo("lapce"),
];

static PHPSTORM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("phpstorm"),
    InstallationMethod::Winget("JetBrains.PhpStorm"),
];

static INTELLIJ_IDEA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("intellij-idea"),
    InstallationMethod::Winget("JetBrains.IntelliJIDEA.Community"),
];

static PYCHARM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("pycharm"),
    InstallationMethod::Winget("JetBrains.PyCharm.Community"),
];

static WEBSTORM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("webstorm"),
    InstallationMethod::Winget("JetBrains.WebStorm"),
];

static CLION_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("clion"),
    InstallationMethod::Winget("JetBrains.CLion"),
];

static GOLAND_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("goland"),
    InstallationMethod::Winget("JetBrains.GoLand"),
];

static RIDER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("rider"),
    InstallationMethod::Winget("JetBrains.Rider"),
];

static TEXTMATE_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("textmate")];

static BBEDIT_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("bbedit")];

static GEANY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("geany"),
    InstallationMethod::Apt("geany"),
    InstallationMethod::Dnf("geany"),
    InstallationMethod::Pacman("geany"),
];

static KATE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("kate"),
    InstallationMethod::Apt("kate"),
    InstallationMethod::Dnf("kate"),
    InstallationMethod::Pacman("kate"),
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

static EXA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("exa"),
    InstallationMethod::Cargo("exa"),
    InstallationMethod::Apt("exa"),
    InstallationMethod::Pacman("exa"),
];

static DUST_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("dust"),
    InstallationMethod::Cargo("dust"),
    InstallationMethod::Apt("dust"),
    InstallationMethod::Dnf("dust"),
    InstallationMethod::Pacman("dust"),
];

static PROCS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("procs"),
    InstallationMethod::Cargo("procs"),
    InstallationMethod::Apt("procs"),
    InstallationMethod::Pacman("procs"),
];

static BOTTOM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("bottom"),
    InstallationMethod::Cargo("bottom"),
    InstallationMethod::Apt("bottom"),
    InstallationMethod::Pacman("bottom"),
];

static ZOXIDE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("zoxide"),
    InstallationMethod::Cargo("zoxide"),
    InstallationMethod::Apt("zoxide"),
    InstallationMethod::Dnf("zoxide"),
    InstallationMethod::Pacman("zoxide"),
];

static DIRENV_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("direnv"),
    InstallationMethod::Apt("direnv"),
    InstallationMethod::Dnf("direnv"),
    InstallationMethod::Pacman("direnv"),
];

static TEALDEER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tealdeer"),
    InstallationMethod::Cargo("tealdeer"),
    InstallationMethod::Apt("tealdeer"),
    InstallationMethod::Pacman("tealdeer"),
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

static HTOP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("htop"),
    InstallationMethod::Apt("htop"),
    InstallationMethod::Dnf("htop"),
    InstallationMethod::Pacman("htop"),
];

static BTOP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("btop"),
    InstallationMethod::Apt("btop"),
    InstallationMethod::Dnf("btop"),
    InstallationMethod::Pacman("btop"),
];

static TMUX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tmux"),
    InstallationMethod::Apt("tmux"),
    InstallationMethod::Dnf("tmux"),
    InstallationMethod::Pacman("tmux"),
];

static ZELLIJ_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("zellij"),
    InstallationMethod::Cargo("zellij"),
    InstallationMethod::Apt("zellij"),
    InstallationMethod::Pacman("zellij"),
];

static HTTPIE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("httpie"),
    InstallationMethod::Apt("httpie"),
    InstallationMethod::Dnf("httpie"),
    InstallationMethod::Pacman("httpie"),
    InstallationMethod::Pip("httpie"),
];

static CURLIE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("curlie"),
    InstallationMethod::Cargo("curlie"),
    InstallationMethod::Scoop("curlie"),
];

static MISE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mise"),
    InstallationMethod::Cargo("mise"),
    InstallationMethod::Scoop("mise"),
];

static HYPERFINE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("hyperfine"),
    InstallationMethod::Cargo("hyperfine"),
    InstallationMethod::Apt("hyperfine"),
    InstallationMethod::Pacman("hyperfine"),
];

static TOKEI_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tokei"),
    InstallationMethod::Cargo("tokei"),
    InstallationMethod::Apt("tokei"),
    InstallationMethod::Pacman("tokei"),
];

static XH_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("xh"),
    InstallationMethod::Cargo("xh"),
    InstallationMethod::Apt("xh"),
    InstallationMethod::Pacman("xh"),
    InstallationMethod::Scoop("xh"),
];

static CURL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("curl"),
    InstallationMethod::Apt("curl"),
    InstallationMethod::Dnf("curl"),
    InstallationMethod::Pacman("curl"),
];

static WGET_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("wget"),
    InstallationMethod::Apt("wget"),
    InstallationMethod::Dnf("wget"),
    InstallationMethod::Pacman("wget"),
];

static IPERF3_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("iperf3"),
    InstallationMethod::Apt("iperf3"),
    InstallationMethod::Dnf("iperf3"),
    InstallationMethod::Pacman("iperf3"),
];

// Package Managers
static BREW_INSTALL: &[InstallationMethod] = &[InstallationMethod::RemoteBash(
    "https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh",
)];

static CARGO_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::RemoteBash("https://sh.rustup.rs")];

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

static YARN_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("yarn"),
    InstallationMethod::Npm("yarn"),
];

static BUN_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("bun"),
    InstallationMethod::Npm("bun"),
];

static GO_MODULES_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("go"),
    InstallationMethod::Apt("golang"),
    InstallationMethod::Dnf("golang"),
    InstallationMethod::Pacman("go"),
    InstallationMethod::Chocolatey("golang"),
    InstallationMethod::Scoop("go"),
];

static COMPOSER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("composer"),
    InstallationMethod::Apt("composer"),
    InstallationMethod::Dnf("composer"),
    InstallationMethod::Pacman("composer"),
    InstallationMethod::Chocolatey("composer"),
];

static SWIFTPM_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("swift")];

static LUAROCKS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("luarocks"),
    InstallationMethod::Apt("luarocks"),
    InstallationMethod::Dnf("luarocks"),
    InstallationMethod::Pacman("luarocks"),
];

static VCPKG_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vcpkg"),
    InstallationMethod::Chocolatey("vcpkg"),
];

static CONAN_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("conan")];

static NUGET_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("nuget"),
    InstallationMethod::Chocolatey("nuget"),
    InstallationMethod::Scoop("nuget"),
];

static HEX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("elixir"),
    InstallationMethod::Apt("elixir"),
    InstallationMethod::Dnf("elixir"),
    InstallationMethod::Pacman("elixir"),
    InstallationMethod::Chocolatey("elixir"),
];

static UV_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("uv"),
    InstallationMethod::Cargo("uv"),
    InstallationMethod::Pip("uv"),
    InstallationMethod::RemoteBash("https://astral.sh/uv/install.sh"),
];

static POETRY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("poetry"),
    InstallationMethod::Pip("poetry"),
];

static PIP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("python3-pip"),
    InstallationMethod::Dnf("python3-pip"),
    InstallationMethod::Pacman("python-pip"),
];

static CPAN_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("perl"),
    InstallationMethod::Apt("perl"),
    InstallationMethod::Dnf("perl"),
    InstallationMethod::Pacman("perl"),
    InstallationMethod::Chocolatey("strawberryperl"),
];

static CPANM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("cpanminus"),
    InstallationMethod::Apt("cpanminus"),
    InstallationMethod::Dnf("cpanminus"),
    InstallationMethod::Pacman("perl-app-cpanminus"),
];

// TTS Clients
static ESPEAK_NG_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("espeak-ng"),
    InstallationMethod::Apt("espeak-ng"),
    InstallationMethod::Dnf("espeak-ng"),
    InstallationMethod::Pacman("espeak-ng"),
];

static ESPEAK_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("espeak"),
    InstallationMethod::Apt("espeak"),
    InstallationMethod::Dnf("espeak"),
    InstallationMethod::Pacman("espeak"),
];

static FESTIVAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("festival"),
    InstallationMethod::Apt("festival"),
    InstallationMethod::Dnf("festival"),
    InstallationMethod::Pacman("festival"),
];

static MIMIC_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("mimic")];

static MIMIC3_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("mimic3-tts")];

static PIPER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("piper"),
    InstallationMethod::Pip("piper-tts"),
];

static SHERPA_ONNX_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("sherpa-onnx")];

static ECHOGARDEN_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("echogarden")];

static BALCON_INSTALL: &[InstallationMethod] = &[InstallationMethod::Chocolatey("balcon")];

static WINDOWS_SAPI_INSTALL: &[InstallationMethod] = &[];

static GTTS_CLI_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("gTTS")];

static COQUI_TTS_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("TTS")];

static KOKORO_TTS_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("kokoro-tts")];

static PICO2WAVE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("libttspico-utils"),
    InstallationMethod::Pacman("svox-pico"),
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

static VLC_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vlc"),
    InstallationMethod::Apt("vlc"),
    InstallationMethod::Dnf("vlc"),
    InstallationMethod::Pacman("vlc"),
    InstallationMethod::Chocolatey("vlc"),
    InstallationMethod::Scoop("vlc"),
];

static MPLAYER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mplayer"),
    InstallationMethod::Apt("mplayer"),
    InstallationMethod::Dnf("mplayer"),
    InstallationMethod::Pacman("mplayer"),
];

static GSTREAMER_GST_PLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("gstreamer"),
    InstallationMethod::Apt("gstreamer1.0-tools"),
    InstallationMethod::Dnf("gstreamer1-plugins-base"),
    InstallationMethod::Pacman("gstreamer"),
];

static MPG123_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mpg123"),
    InstallationMethod::Apt("mpg123"),
    InstallationMethod::Dnf("mpg123"),
    InstallationMethod::Pacman("mpg123"),
];

static OGG123_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vorbis-tools"),
    InstallationMethod::Apt("vorbis-tools"),
    InstallationMethod::Dnf("vorbis-tools"),
    InstallationMethod::Pacman("vorbis-tools"),
];

static ALSA_APLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("alsa-utils"),
    InstallationMethod::Dnf("alsa-utils"),
    InstallationMethod::Pacman("alsa-utils"),
];

static PULSEAUDIO_PAPLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("pulseaudio-utils"),
    InstallationMethod::Dnf("pulseaudio-utils"),
    InstallationMethod::Pacman("pulseaudio"),
];

static PIPEWIRE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("pipewire"),
    InstallationMethod::Dnf("pipewire"),
    InstallationMethod::Pacman("pipewire"),
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

static ITERM2_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("iterm2")];

static GHOSTTY_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("ghostty")];

static WARP_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("warp")];

static RIO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("rio"),
    InstallationMethod::Scoop("rio"),
];

static TABBY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tabby"),
    InstallationMethod::Scoop("tabby"),
];

static FOOT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("foot"),
    InstallationMethod::Dnf("foot"),
    InstallationMethod::Pacman("foot"),
];

static GNOME_TERMINAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("gnome-terminal"),
    InstallationMethod::Dnf("gnome-terminal"),
    InstallationMethod::Pacman("gnome-terminal"),
];

static KONSOLE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("konsole"),
    InstallationMethod::Dnf("konsole"),
    InstallationMethod::Pacman("konsole"),
];

static XFCE_TERMINAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("xfce4-terminal"),
    InstallationMethod::Dnf("xfce4-terminal"),
    InstallationMethod::Pacman("xfce4-terminal"),
];

static TERMINOLOGY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("terminology"),
    InstallationMethod::Dnf("terminology"),
    InstallationMethod::Pacman("terminology"),
];

static ST_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("st"),
    InstallationMethod::Pacman("st"),
];

static XTERM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("xterm"),
    InstallationMethod::Dnf("xterm"),
    InstallationMethod::Pacman("xterm"),
];

static HYPER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("hyper"),
    InstallationMethod::Scoop("hyper"),
];

static WINDOWS_TERMINAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Winget("Microsoft.WindowsTerminal"),
    InstallationMethod::Scoop("windows-terminal"),
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
/// All programs in the `Program` enum have entries here.
pub static PROGRAM_LOOKUP: LazyLock<HashMap<Program, ProgramDetails>> = LazyLock::new(|| {
    let mut lookup = HashMap::new();

    // ========================================================================
    // Editors (26 entries)
    // ========================================================================
    lookup.insert(
        Program::Vi,
        ProgramDetails::full(
            "Vi",
            "Classic vi text editor",
            ALL_OS,
            "https://www.vim.org/",
            Some("https://github.com/vim/vim"),
            VI_INSTALL,
        ),
    );
    lookup.insert(
        Program::Vim,
        ProgramDetails::full(
            "Vim",
            "Vi IMproved text editor",
            ALL_OS,
            "https://www.vim.org/",
            Some("https://github.com/vim/vim"),
            VIM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Neovim,
        ProgramDetails::full(
            "Neovim",
            "Hyperextensible Vim-based text editor",
            ALL_OS,
            "https://neovim.io/",
            Some("https://github.com/neovim/neovim"),
            NEOVIM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Emacs,
        ProgramDetails::full(
            "GNU Emacs",
            "Extensible, customizable text editor",
            ALL_OS,
            "https://www.gnu.org/software/emacs/",
            Some("https://git.savannah.gnu.org/cgit/emacs.git"),
            EMACS_INSTALL,
        ),
    );

    lookup.insert(
        Program::XEmacs,
        ProgramDetails::full(
            "XEmacs",
            "Emacs variant with additional features",
            UNIX_ONLY,
            "http://www.xemacs.org/",
            Some("https://github.com/xemacs/xemacs"),
            XEMACS_INSTALL,
        ),
    );

    lookup.insert(
        Program::Nano,
        ProgramDetails::full(
            "GNU nano",
            "Small and friendly text editor",
            ALL_OS,
            "https://www.nano-editor.org/",
            Some("https://git.savannah.gnu.org/cgit/nano.git"),
            NANO_INSTALL,
        ),
    );

    lookup.insert(
        Program::Helix,
        ProgramDetails::full(
            "Helix",
            "Post-modern modal text editor",
            ALL_OS,
            "https://helix-editor.com/",
            Some("https://github.com/helix-editor/helix"),
            HELIX_INSTALL,
        ),
    );

    lookup.insert(
        Program::VSCode,
        ProgramDetails::full(
            "Visual Studio Code",
            "Code editor for modern web and cloud applications",
            ALL_OS,
            "https://code.visualstudio.com/",
            Some("https://github.com/microsoft/vscode"),
            VSCODE_INSTALL,
        ),
    );

    lookup.insert(
        Program::VSCodium,
        ProgramDetails::full(
            "VSCodium",
            "Free/libre open source binaries of VS Code",
            ALL_OS,
            "https://vscodium.com/",
            Some("https://github.com/VSCodium/vscodium"),
            VSCODIUM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Sublime,
        ProgramDetails::full(
            "Sublime Text",
            "Sophisticated text editor for code and prose",
            ALL_OS,
            "https://www.sublimetext.com/",
            Some("https://github.com/sublimehq"),
            SUBLIME_INSTALL,
        ),
    );

    lookup.insert(
        Program::Zed,
        ProgramDetails::full(
            "Zed",
            "High-performance multiplayer code editor",
            MACOS_ONLY,
            "https://zed.dev/",
            Some("https://github.com/zed-industries/zed"),
            ZED_INSTALL,
        ),
    );

    lookup.insert(
        Program::Micro,
        ProgramDetails::full(
            "Micro",
            "Modern terminal-based text editor",
            ALL_OS,
            "https://micro-editor.github.io/",
            Some("https://github.com/zyedidia/micro"),
            MICRO_INSTALL,
        ),
    );

    lookup.insert(
        Program::Kakoune,
        ProgramDetails::full(
            "Kakoune",
            "Modal editor with selection-based editing model",
            UNIX_ONLY,
            "https://kakoune.org/",
            Some("https://github.com/mawww/kakoune"),
            KAKOUNE_INSTALL,
        ),
    );

    lookup.insert(
        Program::Amp,
        ProgramDetails::full(
            "Amp",
            "Modal text editor inspired by Vi",
            ALL_OS,
            "https://amp.readme.io/",
            Some("https://github.com/jmacdonald/amp"),
            AMP_INSTALL,
        ),
    );

    lookup.insert(
        Program::Lapce,
        ProgramDetails::full(
            "Lapce",
            "Lightning-fast code editor written in Rust",
            ALL_OS,
            "https://lapce.dev/",
            Some("https://github.com/lapce/lapce"),
            LAPCE_INSTALL,
        ),
    );

    lookup.insert(
        Program::PhpStorm,
        ProgramDetails::full(
            "PhpStorm",
            "Lightning-smart PHP IDE by JetBrains",
            ALL_OS,
            "https://www.jetbrains.com/phpstorm/",
            Some("https://www.jetbrains.com/phpstorm/"),
            PHPSTORM_INSTALL,
        ),
    );

    lookup.insert(
        Program::IntellijIdea,
        ProgramDetails::full(
            "IntelliJ IDEA",
            "Capable and ergonomic IDE for JVM-based languages",
            ALL_OS,
            "https://www.jetbrains.com/idea/",
            Some("https://www.jetbrains.com/idea/"),
            INTELLIJ_IDEA_INSTALL,
        ),
    );

    lookup.insert(
        Program::PyCharm,
        ProgramDetails::full(
            "PyCharm",
            "Python IDE for professional developers",
            ALL_OS,
            "https://www.jetbrains.com/pycharm/",
            Some("https://www.jetbrains.com/pycharm/"),
            PYCHARM_INSTALL,
        ),
    );

    lookup.insert(
        Program::WebStorm,
        ProgramDetails::full(
            "WebStorm",
            "JetBrains IDE for JavaScript and TypeScript",
            ALL_OS,
            "https://www.jetbrains.com/webstorm/",
            Some("https://www.jetbrains.com/webstorm/"),
            WEBSTORM_INSTALL,
        ),
    );

    lookup.insert(
        Program::CLion,
        ProgramDetails::full(
            "CLion",
            "Cross-platform C and C++ IDE",
            ALL_OS,
            "https://www.jetbrains.com/clion/",
            Some("https://www.jetbrains.com/clion/"),
            CLION_INSTALL,
        ),
    );

    lookup.insert(
        Program::GoLand,
        ProgramDetails::full(
            "GoLand",
            "Cross-platform Go IDE",
            ALL_OS,
            "https://www.jetbrains.com/go/",
            Some("https://www.jetbrains.com/go/"),
            GOLAND_INSTALL,
        ),
    );

    lookup.insert(
        Program::Rider,
        ProgramDetails::full(
            "Rider",
            "Cross-platform .NET IDE",
            ALL_OS,
            "https://www.jetbrains.com/rider/",
            Some("https://www.jetbrains.com/rider/"),
            RIDER_INSTALL,
        ),
    );

    lookup.insert(
        Program::TextMate,
        ProgramDetails::full(
            "TextMate",
            "Versatile plain text editor for macOS",
            MACOS_ONLY,
            "https://macromates.com/",
            Some("https://github.com/textmate/textmate"),
            TEXTMATE_INSTALL,
        ),
    );

    lookup.insert(
        Program::BBEdit,
        ProgramDetails::full(
            "BBEdit",
            "Professional HTML and text editor for macOS",
            MACOS_ONLY,
            "https://www.barebones.com/products/bbedit/",
            None,
            BBEDIT_INSTALL,
        ),
    );

    lookup.insert(
        Program::Geany,
        ProgramDetails::full(
            "Geany",
            "Lightweight programmer's text editor",
            UNIX_ONLY,
            "https://www.geany.org/",
            Some("https://github.com/geany/geany"),
            GEANY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Kate,
        ProgramDetails::full(
            "Kate",
            "Multi-document text editor by KDE",
            UNIX_ONLY,
            "https://kate-editor.org/",
            Some("https://invent.kde.org/utilities/kate"),
            KATE_INSTALL,
        ),
    );

    // ========================================================================
    // Utilities (30 entries)
    // ========================================================================
    lookup.insert(
        Program::Ripgrep,
        ProgramDetails::full(
            "ripgrep",
            "Fast grep alternative with smart defaults",
            ALL_OS,
            "https://github.com/BurntSushi/ripgrep",
            Some("https://github.com/BurntSushi/ripgrep"),
            RIPGREP_INSTALL,
        ),
    );

    lookup.insert(
        Program::Bat,
        ProgramDetails::full(
            "bat",
            "A cat clone with syntax highlighting",
            ALL_OS,
            "https://github.com/sharkdp/bat",
            Some("https://github.com/sharkdp/bat"),
            BAT_INSTALL,
        ),
    );

    lookup.insert(
        Program::Fd,
        ProgramDetails::full(
            "fd",
            "Simple, fast alternative to find",
            ALL_OS,
            "https://github.com/sharkdp/fd",
            Some("https://github.com/sharkdp/fd"),
            FD_INSTALL,
        ),
    );

    lookup.insert(
        Program::Fzf,
        ProgramDetails::full(
            "fzf",
            "Command-line fuzzy finder",
            ALL_OS,
            "https://github.com/junegunn/fzf",
            Some("https://github.com/junegunn/fzf"),
            FZF_INSTALL,
        ),
    );

    lookup.insert(
        Program::Exa,
        ProgramDetails::full(
            "exa",
            "Modern replacement for ls (deprecated)",
            ALL_OS,
            "https://the.exa.website/",
            Some("https://github.com/ogham/exa"),
            EXA_INSTALL,
        ),
    );

    lookup.insert(
        Program::Eza,
        ProgramDetails::full(
            "eza",
            "A modern replacement for ls",
            ALL_OS,
            "https://eza.rocks/",
            Some("https://github.com/eza-community/eza"),
            EZA_INSTALL,
        ),
    );

    lookup.insert(
        Program::Dust,
        ProgramDetails::full(
            "dust",
            "A more intuitive version of du",
            ALL_OS,
            "https://github.com/bootandy/dust",
            Some("https://github.com/bootandy/dust"),
            DUST_INSTALL,
        ),
    );

    lookup.insert(
        Program::Procs,
        ProgramDetails::full(
            "procs",
            "A modern replacement for ps",
            ALL_OS,
            "https://github.com/dalance/procs",
            Some("https://github.com/dalance/procs"),
            PROCS_INSTALL,
        ),
    );

    lookup.insert(
        Program::Bottom,
        ProgramDetails::full(
            "bottom",
            "Cross-platform graphical process monitor",
            ALL_OS,
            "https://github.com/ClementTsang/bottom",
            Some("https://github.com/ClementTsang/bottom"),
            BOTTOM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Zoxide,
        ProgramDetails::full(
            "zoxide",
            "Smarter cd command",
            ALL_OS,
            "https://github.com/ajeetdsouza/zoxide",
            Some("https://github.com/ajeetdsouza/zoxide"),
            ZOXIDE_INSTALL,
        ),
    );

    lookup.insert(
        Program::Direnv,
        ProgramDetails::full(
            "direnv",
            "Environment switcher for the shell",
            ALL_OS,
            "https://direnv.net/",
            Some("https://github.com/direnv/direnv"),
            DIRENV_INSTALL,
        ),
    );

    lookup.insert(
        Program::Tealdeer,
        ProgramDetails::full(
            "tealdeer",
            "Fast tldr client for simplified man pages",
            ALL_OS,
            "https://github.com/dbrgn/tealdeer",
            Some("https://github.com/dbrgn/tealdeer"),
            TEALDEER_INSTALL,
        ),
    );

    lookup.insert(
        Program::Jq,
        ProgramDetails::full(
            "jq",
            "Command-line JSON processor",
            ALL_OS,
            "https://jqlang.github.io/jq/",
            Some("https://github.com/jqlang/jq"),
            JQ_INSTALL,
        ),
    );

    lookup.insert(
        Program::Gh,
        ProgramDetails::full(
            "GitHub CLI",
            "GitHub's official CLI",
            ALL_OS,
            "https://cli.github.com/",
            Some("https://github.com/cli/cli"),
            GH_INSTALL,
        ),
    );

    lookup.insert(
        Program::Lazygit,
        ProgramDetails::full(
            "lazygit",
            "Simple terminal UI for git commands",
            ALL_OS,
            "https://github.com/jesseduffield/lazygit",
            Some("https://github.com/jesseduffield/lazygit"),
            LAZYGIT_INSTALL,
        ),
    );

    lookup.insert(
        Program::Delta,
        ProgramDetails::full(
            "delta",
            "Viewer for git and diff output",
            ALL_OS,
            "https://github.com/dandavison/delta",
            Some("https://github.com/dandavison/delta"),
            DELTA_INSTALL,
        ),
    );

    lookup.insert(
        Program::Starship,
        ProgramDetails::full(
            "Starship",
            "Minimal, blazing-fast shell prompt",
            ALL_OS,
            "https://starship.rs/",
            Some("https://github.com/starship/starship"),
            STARSHIP_INSTALL,
        ),
    );

    lookup.insert(
        Program::Htop,
        ProgramDetails::full(
            "htop",
            "Interactive process viewer",
            UNIX_ONLY,
            "https://htop.dev/",
            Some("https://github.com/htop-dev/htop"),
            HTOP_INSTALL,
        ),
    );

    lookup.insert(
        Program::Btop,
        ProgramDetails::full(
            "btop",
            "Resource monitor with CPU, memory, disk, network stats",
            UNIX_ONLY,
            "https://github.com/aristocratos/btop",
            Some("https://github.com/aristocratos/btop"),
            BTOP_INSTALL,
        ),
    );

    lookup.insert(
        Program::Tmux,
        ProgramDetails::full(
            "tmux",
            "Terminal multiplexer",
            UNIX_ONLY,
            "https://github.com/tmux/tmux/wiki",
            Some("https://github.com/tmux/tmux"),
            TMUX_INSTALL,
        ),
    );

    lookup.insert(
        Program::Zellij,
        ProgramDetails::full(
            "Zellij",
            "Modern terminal multiplexer",
            UNIX_ONLY,
            "https://zellij.dev/",
            Some("https://github.com/zellij-org/zellij"),
            ZELLIJ_INSTALL,
        ),
    );

    lookup.insert(
        Program::Httpie,
        ProgramDetails::full(
            "HTTPie",
            "User-friendly HTTP client",
            ALL_OS,
            "https://httpie.io/",
            Some("https://github.com/httpie/cli"),
            HTTPIE_INSTALL,
        ),
    );

    lookup.insert(
        Program::Curlie,
        ProgramDetails::full(
            "curlie",
            "User-friendly alternative to curl",
            ALL_OS,
            "https://github.com/rs/curlie",
            Some("https://github.com/rs/curlie"),
            CURLIE_INSTALL,
        ),
    );

    lookup.insert(
        Program::Mise,
        ProgramDetails::full(
            "mise",
            "Polyglot development environment manager",
            ALL_OS,
            "https://mise.jdx.dev/",
            Some("https://github.com/jdx/mise"),
            MISE_INSTALL,
        ),
    );

    lookup.insert(
        Program::Hyperfine,
        ProgramDetails::full(
            "hyperfine",
            "Command-line benchmarking tool",
            ALL_OS,
            "https://github.com/sharkdp/hyperfine",
            Some("https://github.com/sharkdp/hyperfine"),
            HYPERFINE_INSTALL,
        ),
    );

    lookup.insert(
        Program::Tokei,
        ProgramDetails::full(
            "tokei",
            "Count lines of code quickly",
            ALL_OS,
            "https://github.com/XAMPPRocky/tokei",
            Some("https://github.com/XAMPPRocky/tokei"),
            TOKEI_INSTALL,
        ),
    );

    lookup.insert(
        Program::Xh,
        ProgramDetails::full(
            "xh",
            "Friendly and fast HTTP client",
            ALL_OS,
            "https://github.com/ducaale/xh",
            Some("https://github.com/ducaale/xh"),
            XH_INSTALL,
        ),
    );

    lookup.insert(
        Program::Curl,
        ProgramDetails::full(
            "curl",
            "Transfer data with URLs",
            ALL_OS,
            "https://curl.se/",
            Some("https://github.com/curl/curl"),
            CURL_INSTALL,
        ),
    );

    lookup.insert(
        Program::Wget,
        ProgramDetails::full(
            "wget",
            "Network utility to retrieve content from web servers",
            ALL_OS,
            "https://www.gnu.org/software/wget/",
            Some("https://git.savannah.gnu.org/cgit/wget.git"),
            WGET_INSTALL,
        ),
    );

    lookup.insert(
        Program::Iperf3,
        ProgramDetails::full(
            "iperf3",
            "Network bandwidth measurement tool",
            ALL_OS,
            "https://iperf.fr/",
            Some("https://github.com/esnet/iperf"),
            IPERF3_INSTALL,
        ),
    );

    // ========================================================================
    // Package Managers (27 entries)
    // ========================================================================
    lookup.insert(
        Program::Brew,
        ProgramDetails::full(
            "Homebrew",
            "macOS/Linux community package manager",
            UNIX_ONLY,
            "https://brew.sh/",
            Some("https://github.com/Homebrew/brew"),
            BREW_INSTALL,
        ),
    );

    lookup.insert(
        Program::Cargo,
        ProgramDetails::full(
            "Cargo",
            "Rust package manager and build tool",
            ALL_OS,
            "https://doc.rust-lang.org/cargo/",
            Some("https://github.com/rust-lang/cargo"),
            CARGO_INSTALL,
        ),
    );

    lookup.insert(
        Program::Npm,
        ProgramDetails::full(
            "npm",
            "Node.js package manager",
            ALL_OS,
            "https://www.npmjs.com/",
            Some("https://github.com/npm/cli"),
            NPM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Pnpm,
        ProgramDetails::full(
            "pnpm",
            "Fast, disk-efficient package manager",
            ALL_OS,
            "https://pnpm.io/",
            Some("https://github.com/pnpm/pnpm"),
            PNPM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Yarn,
        ProgramDetails::full(
            "Yarn",
            "Alternative Node.js package manager",
            ALL_OS,
            "https://yarnpkg.com/",
            Some("https://github.com/yarnpkg/berry"),
            YARN_INSTALL,
        ),
    );

    lookup.insert(
        Program::Bun,
        ProgramDetails::full(
            "Bun",
            "All-in-one JS runtime with package manager",
            ALL_OS,
            "https://bun.sh/",
            Some("https://github.com/oven-sh/bun"),
            BUN_INSTALL,
        ),
    );

    lookup.insert(
        Program::GoModules,
        ProgramDetails::full(
            "Go Modules",
            "Built-in Go dependency system",
            ALL_OS,
            "https://go.dev/ref/mod",
            Some("https://github.com/golang/go"),
            GO_MODULES_INSTALL,
        ),
    );

    lookup.insert(
        Program::Composer,
        ProgramDetails::full(
            "Composer",
            "PHP dependency manager",
            ALL_OS,
            "https://getcomposer.org/",
            Some("https://github.com/composer/composer"),
            COMPOSER_INSTALL,
        ),
    );

    lookup.insert(
        Program::SwiftPm,
        ProgramDetails::full(
            "Swift Package Manager",
            "Swift dependency manager",
            UNIX_ONLY,
            "https://www.swift.org/package-manager/",
            Some("https://github.com/apple/swift-package-manager"),
            SWIFTPM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Luarocks,
        ProgramDetails::full(
            "LuaRocks",
            "Package manager for Lua modules",
            ALL_OS,
            "https://luarocks.org/",
            Some("https://github.com/luarocks/luarocks"),
            LUAROCKS_INSTALL,
        ),
    );

    lookup.insert(
        Program::Vcpkg,
        ProgramDetails::full(
            "vcpkg",
            "C/C++ dependency manager by Microsoft",
            ALL_OS,
            "https://vcpkg.io/",
            Some("https://github.com/microsoft/vcpkg"),
            VCPKG_INSTALL,
        ),
    );

    lookup.insert(
        Program::Conan,
        ProgramDetails::full(
            "Conan",
            "Decentralized C/C++ package manager",
            ALL_OS,
            "https://conan.io/",
            Some("https://github.com/conan-io/conan"),
            CONAN_INSTALL,
        ),
    );

    lookup.insert(
        Program::Nuget,
        ProgramDetails::full(
            "NuGet",
            ".NET package manager",
            ALL_OS,
            "https://www.nuget.org/",
            Some("https://github.com/NuGet/NuGet.Client"),
            NUGET_INSTALL,
        ),
    );

    lookup.insert(
        Program::Hex,
        ProgramDetails::full(
            "Hex",
            "Package manager for BEAM ecosystem",
            ALL_OS,
            "https://hex.pm/",
            Some("https://github.com/hexpm/hex"),
            HEX_INSTALL,
        ),
    );

    lookup.insert(
        Program::Pip,
        ProgramDetails::full(
            "pip",
            "Python package installer",
            ALL_OS,
            "https://pip.pypa.io/",
            Some("https://github.com/pypa/pip"),
            PIP_INSTALL,
        ),
    );

    lookup.insert(
        Program::Uv,
        ProgramDetails::full(
            "uv",
            "Fast Python package manager",
            ALL_OS,
            "https://astral.sh/uv",
            Some("https://github.com/astral-sh/uv"),
            UV_INSTALL,
        ),
    );

    lookup.insert(
        Program::Poetry,
        ProgramDetails::full(
            "Poetry",
            "Python dependency manager with lockfiles",
            ALL_OS,
            "https://python-poetry.org/",
            Some("https://github.com/python-poetry/poetry"),
            POETRY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Cpan,
        ProgramDetails::full(
            "CPAN",
            "Perl module archive",
            ALL_OS,
            "https://www.cpan.org/",
            Some("https://github.com/Perl/perl5"),
            CPAN_INSTALL,
        ),
    );

    lookup.insert(
        Program::Cpanm,
        ProgramDetails::full(
            "cpanminus",
            "Lightweight CPAN client",
            ALL_OS,
            "https://metacpan.org/pod/App::cpanminus",
            Some("https://github.com/miyagawa/cpanminus"),
            CPANM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Apt,
        ProgramDetails::full(
            "APT",
            "Debian/Ubuntu package manager",
            LINUX_ONLY,
            "https://tracker.debian.org/pkg/apt",
            Some("https://salsa.debian.org/apt-team/apt"),
            &[],
        ),
    );

    lookup.insert(
        Program::Nala,
        ProgramDetails::full(
            "Nala",
            "Modern apt frontend with parallel downloads",
            LINUX_ONLY,
            "https://github.com/volitank/nala",
            Some("https://github.com/volitank/nala"),
            &[],
        ),
    );

    lookup.insert(
        Program::Dnf,
        ProgramDetails::full(
            "DNF",
            "Fedora/RHEL package manager",
            LINUX_ONLY,
            "https://github.com/rpm-software-management/dnf",
            Some("https://github.com/rpm-software-management/dnf"),
            &[],
        ),
    );

    lookup.insert(
        Program::Pacman,
        ProgramDetails::full(
            "Pacman",
            "Arch Linux package manager",
            LINUX_ONLY,
            "https://archlinux.org/pacman/",
            Some("https://gitlab.archlinux.org/pacman/pacman"),
            &[],
        ),
    );

    lookup.insert(
        Program::Winget,
        ProgramDetails::full(
            "winget",
            "Windows Package Manager",
            WINDOWS_ONLY,
            "https://github.com/microsoft/winget-cli",
            Some("https://github.com/microsoft/winget-cli"),
            &[],
        ),
    );

    lookup.insert(
        Program::Chocolatey,
        ProgramDetails::full(
            "Chocolatey",
            "Windows community package manager",
            WINDOWS_ONLY,
            "https://chocolatey.org/",
            Some("https://github.com/chocolatey/choco"),
            &[],
        ),
    );

    lookup.insert(
        Program::Scoop,
        ProgramDetails::full(
            "Scoop",
            "Windows command-line installer",
            WINDOWS_ONLY,
            "https://scoop.sh/",
            Some("https://github.com/ScoopInstaller/Scoop"),
            &[],
        ),
    );

    lookup.insert(
        Program::Nix,
        ProgramDetails::full(
            "Nix",
            "Nix package manager",
            UNIX_ONLY,
            "https://nixos.org/",
            Some("https://github.com/NixOS/nix"),
            &[],
        ),
    );

    // ========================================================================
    // TTS Clients (15 entries)
    // ========================================================================
    lookup.insert(
        Program::Say,
        ProgramDetails::full(
            "say",
            "macOS built-in speech synthesis",
            MACOS_ONLY,
            "https://ss64.com/osx/say.html",
            None,
            &[], // Built-in, no install needed
        ),
    );

    lookup.insert(
        Program::EspeakNg,
        ProgramDetails::full(
            "eSpeak NG",
            "Multi-lingual speech synthesizer",
            ALL_OS,
            "https://github.com/espeak-ng/espeak-ng",
            Some("https://github.com/espeak-ng/espeak-ng"),
            ESPEAK_NG_INSTALL,
        ),
    );

    lookup.insert(
        Program::Espeak,
        ProgramDetails::full(
            "eSpeak",
            "Open source speech synthesizer",
            ALL_OS,
            "http://espeak.sourceforge.net/",
            Some("https://github.com/espeak-ng/espeak-ng"),
            ESPEAK_INSTALL,
        ),
    );

    lookup.insert(
        Program::Festival,
        ProgramDetails::full(
            "Festival",
            "General multi-lingual speech synthesis",
            UNIX_ONLY,
            "http://www.cstr.ed.ac.uk/projects/festival/",
            Some("https://github.com/festvox/festival"),
            FESTIVAL_INSTALL,
        ),
    );

    lookup.insert(
        Program::Mimic,
        ProgramDetails::full(
            "Mimic",
            "Mycroft's TTS engine based on Flite",
            ALL_OS,
            "https://github.com/MycroftAI/mimic",
            Some("https://github.com/MycroftAI/mimic"),
            MIMIC_INSTALL,
        ),
    );

    lookup.insert(
        Program::Mimic3,
        ProgramDetails::full(
            "Mimic 3",
            "Mycroft's neural TTS engine",
            ALL_OS,
            "https://github.com/MycroftAI/mycroft-mimic3-tts",
            Some("https://github.com/MycroftAI/mycroft-mimic3-tts"),
            MIMIC3_INSTALL,
        ),
    );

    lookup.insert(
        Program::Piper,
        ProgramDetails::full(
            "Piper",
            "Fast local neural TTS using ONNX",
            ALL_OS,
            "https://github.com/rhasspy/piper",
            Some("https://github.com/rhasspy/piper"),
            PIPER_INSTALL,
        ),
    );

    lookup.insert(
        Program::Echogarden,
        ProgramDetails::full(
            "Echogarden",
            "Speech processing engine",
            ALL_OS,
            "https://echogarden.io/",
            Some("https://github.com/echogarden-project/echogarden"),
            ECHOGARDEN_INSTALL,
        ),
    );

    lookup.insert(
        Program::Balcon,
        ProgramDetails::full(
            "Balcon",
            "Command line TTS utility for Windows",
            WINDOWS_ONLY,
            "http://www.cross-plus-a.com/balcon.htm",
            None,
            BALCON_INSTALL,
        ),
    );

    lookup.insert(
        Program::WindowsSapi,
        ProgramDetails::full(
            "Windows SAPI",
            "Windows Speech API",
            WINDOWS_ONLY,
            "https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms723627(v=vs.85)",
            None,
            WINDOWS_SAPI_INSTALL,
        ),
    );

    lookup.insert(
        Program::GttsCli,
        ProgramDetails::full(
            "gTTS",
            "Google Text-to-Speech CLI tool",
            ALL_OS,
            "https://github.com/pndurette/gTTS",
            Some("https://github.com/pndurette/gTTS"),
            GTTS_CLI_INSTALL,
        ),
    );

    lookup.insert(
        Program::CoquiTts,
        ProgramDetails::full(
            "Coqui TTS",
            "Deep learning for Text-to-Speech",
            ALL_OS,
            "https://github.com/coqui-ai/TTS",
            Some("https://github.com/coqui-ai/TTS"),
            COQUI_TTS_INSTALL,
        ),
    );

    lookup.insert(
        Program::SherpaOnnx,
        ProgramDetails::full(
            "Sherpa-ONNX",
            "Streaming/non-streaming TTS using ONNX",
            ALL_OS,
            "https://k2-fsa.github.io/sherpa/onnx/",
            Some("https://github.com/k2-fsa/sherpa-onnx"),
            SHERPA_ONNX_INSTALL,
        ),
    );

    lookup.insert(
        Program::KokoroTts,
        ProgramDetails::full(
            "Kokoro TTS",
            "High-quality neural TTS using Kokoro-82M model",
            ALL_OS,
            "https://github.com/nazdridoy/kokoro-tts",
            Some("https://github.com/nazdridoy/kokoro-tts"),
            KOKORO_TTS_INSTALL,
        ),
    );

    lookup.insert(
        Program::Pico2Wave,
        ProgramDetails::full(
            "SVOX Pico",
            "Lightweight TTS for embedded systems",
            UNIX_ONLY,
            "https://github.com/naggety/picmotts",
            Some("https://github.com/naggety/picmotts"),
            PICO2WAVE_INSTALL,
        ),
    );

    // ========================================================================
    // Audio Players (11 entries)
    // ========================================================================
    lookup.insert(
        Program::Mpv,
        ProgramDetails::full(
            "mpv",
            "CLI media player for audio-only playback",
            ALL_OS,
            "https://mpv.io/",
            Some("https://github.com/mpv-player/mpv"),
            MPV_INSTALL,
        ),
    );

    lookup.insert(
        Program::Ffplay,
        ProgramDetails::full(
            "FFplay",
            "Minimal CLI player shipped with FFmpeg",
            ALL_OS,
            "https://www.ffmpeg.org/ffplay.html",
            Some("https://github.com/FFmpeg/FFmpeg"),
            FFPLAY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Sox,
        ProgramDetails::full(
            "SoX play",
            "Swiss-army knife for audio playback",
            ALL_OS,
            "https://sox.sourceforge.net/",
            Some("https://sourceforge.net/projects/sox/"),
            SOX_INSTALL,
        ),
    );

    lookup.insert(
        Program::Vlc,
        ProgramDetails::full(
            "VLC",
            "Headless VLC playback via cvlc",
            ALL_OS,
            "https://wiki.videolan.org/VLC_command-line_help/",
            Some("https://github.com/videolan/vlc"),
            VLC_INSTALL,
        ),
    );

    lookup.insert(
        Program::MPlayer,
        ProgramDetails::full(
            "MPlayer",
            "Classic CLI-oriented media player",
            ALL_OS,
            "https://www.mplayerhq.hu/",
            Some("https://github.com/mplayerhq/mplayer"),
            MPLAYER_INSTALL,
        ),
    );

    lookup.insert(
        Program::GstreamerGstPlay,
        ProgramDetails::full(
            "GStreamer gst-play",
            "CLI front-end to GStreamer pipelines",
            UNIX_ONLY,
            "https://gstreamer.freedesktop.org/",
            Some("https://gitlab.freedesktop.org/gstreamer/gstreamer"),
            GSTREAMER_GST_PLAY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Mpg123,
        ProgramDetails::full(
            "mpg123",
            "Lightweight console MP3 player",
            UNIX_ONLY,
            "https://www.mpg123.de/",
            Some("https://github.com/madebr/mpg123"),
            MPG123_INSTALL,
        ),
    );

    lookup.insert(
        Program::Ogg123,
        ProgramDetails::full(
            "ogg123",
            "CLI player for Ogg/Vorbis files",
            UNIX_ONLY,
            "https://github.com/xiph/vorbis-tools",
            Some("https://github.com/xiph/vorbis-tools"),
            OGG123_INSTALL,
        ),
    );

    lookup.insert(
        Program::AlsaAplay,
        ProgramDetails::full(
            "aplay",
            "ALSA low-level playback utility",
            LINUX_ONLY,
            "https://linux.die.net/man/1/aplay",
            None,
            ALSA_APLAY_INSTALL,
        ),
    );

    lookup.insert(
        Program::PulseaudioPaplay,
        ProgramDetails::full(
            "paplay",
            "Simple PulseAudio playback tool",
            LINUX_ONLY,
            "https://manpages.ubuntu.com/",
            None,
            PULSEAUDIO_PAPLAY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Pipewire,
        ProgramDetails::full(
            "PipeWire",
            "PipeWire CLI playback tool",
            LINUX_ONLY,
            "https://docs.pipewire.org/",
            Some("https://gitlab.freedesktop.org/pipewire/pipewire"),
            PIPEWIRE_INSTALL,
        ),
    );

    // ========================================================================
    // Terminal Apps (17 entries)
    // ========================================================================
    lookup.insert(
        Program::Alacritty,
        ProgramDetails::full(
            "Alacritty",
            "Fast, GPU-accelerated terminal emulator",
            ALL_OS,
            "https://alacritty.org/",
            Some("https://github.com/alacritty/alacritty"),
            ALACRITTY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Kitty,
        ProgramDetails::full(
            "kitty",
            "Fast, feature-rich, GPU-based terminal",
            UNIX_ONLY,
            "https://sw.kovidgoyal.net/kitty/",
            Some("https://github.com/kovidgoyal/kitty"),
            KITTY_INSTALL,
        ),
    );

    lookup.insert(
        Program::WezTerm,
        ProgramDetails::full(
            "WezTerm",
            "GPU-accelerated terminal emulator and multiplexer",
            ALL_OS,
            "https://wezfurlong.org/wezterm/",
            Some("https://github.com/wez/wezterm"),
            WEZTERM_INSTALL,
        ),
    );

    lookup.insert(
        Program::ITerm2,
        ProgramDetails::full(
            "iTerm2",
            "Terminal emulator for macOS",
            MACOS_ONLY,
            "https://iterm2.com/",
            Some("https://github.com/gnachman/iTerm2"),
            ITERM2_INSTALL,
        ),
    );

    lookup.insert(
        Program::Ghostty,
        ProgramDetails::full(
            "Ghostty",
            "Fast, feature-rich GPU terminal written in Zig",
            UNIX_ONLY,
            "https://ghostty.org/",
            Some("https://github.com/ghostty-org/ghostty"),
            GHOSTTY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Warp,
        ProgramDetails::full(
            "Warp",
            "Modern, Rust-based terminal with AI",
            MACOS_ONLY,
            "https://www.warp.dev/",
            Some("https://www.warp.dev/"),
            WARP_INSTALL,
        ),
    );

    lookup.insert(
        Program::Rio,
        ProgramDetails::full(
            "Rio",
            "Hardware-accelerated GPU terminal emulator",
            ALL_OS,
            "https://github.com/raphamorim/rio",
            Some("https://github.com/raphamorim/rio"),
            RIO_INSTALL,
        ),
    );

    lookup.insert(
        Program::Tabby,
        ProgramDetails::full(
            "Tabby",
            "Terminal for a more modern age",
            ALL_OS,
            "https://tabby.sh/",
            Some("https://github.com/Eugeny/tabby"),
            TABBY_INSTALL,
        ),
    );

    lookup.insert(
        Program::Foot,
        ProgramDetails::full(
            "foot",
            "Fast, lightweight Wayland terminal emulator",
            LINUX_ONLY,
            "https://codeberg.org/dnkl/foot",
            Some("https://codeberg.org/dnkl/foot"),
            FOOT_INSTALL,
        ),
    );

    lookup.insert(
        Program::GnomeTerminal,
        ProgramDetails::full(
            "GNOME Terminal",
            "Default terminal for GNOME desktop",
            LINUX_ONLY,
            "https://help.gnome.org/users/gnome-terminal/stable/",
            Some("https://gitlab.gnome.org/GNOME/gnome-terminal"),
            GNOME_TERMINAL_INSTALL,
        ),
    );

    lookup.insert(
        Program::Konsole,
        ProgramDetails::full(
            "Konsole",
            "Terminal emulator by KDE",
            LINUX_ONLY,
            "https://konsole.kde.org/",
            Some("https://invent.kde.org/utilities/konsole"),
            KONSOLE_INSTALL,
        ),
    );

    lookup.insert(
        Program::XfceTerminal,
        ProgramDetails::full(
            "Xfce Terminal",
            "Terminal emulator for Xfce",
            LINUX_ONLY,
            "https://docs.xfce.org/apps/xfce4-terminal/start",
            Some("https://gitlab.xfce.org/apps/xfce4-terminal"),
            XFCE_TERMINAL_INSTALL,
        ),
    );

    lookup.insert(
        Program::Terminology,
        ProgramDetails::full(
            "Terminology",
            "Terminal based on Enlightenment libraries",
            LINUX_ONLY,
            "https://www.enlightenment.org/about-terminology",
            Some("https://github.com/Enlightenment/terminology"),
            TERMINOLOGY_INSTALL,
        ),
    );

    lookup.insert(
        Program::St,
        ProgramDetails::full(
            "st",
            "Simple terminal for X which sucks less",
            LINUX_ONLY,
            "https://st.suckless.org/",
            Some("https://git.suckless.org/st"),
            ST_INSTALL,
        ),
    );

    lookup.insert(
        Program::Xterm,
        ProgramDetails::full(
            "xterm",
            "Standard terminal for X Window System",
            LINUX_ONLY,
            "https://invisible-island.net/xterm/",
            Some("https://invisible-island.net/xterm/"),
            XTERM_INSTALL,
        ),
    );

    lookup.insert(
        Program::Hyper,
        ProgramDetails::full(
            "Hyper",
            "Terminal built on web technologies",
            ALL_OS,
            "https://hyper.is/",
            Some("https://github.com/vercel/hyper"),
            HYPER_INSTALL,
        ),
    );

    lookup.insert(
        Program::WindowsTerminal,
        ProgramDetails::full(
            "Windows Terminal",
            "Modern terminal for Windows",
            WINDOWS_ONLY,
            "https://github.com/microsoft/terminal",
            Some("https://github.com/microsoft/terminal"),
            WINDOWS_TERMINAL_INSTALL,
        ),
    );

    lookup
});

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_program_lookup_has_entries() {
        let lookup = &*PROGRAM_LOOKUP;
        assert_eq!(
            lookup.len(),
            Program::COUNT,
            "Expected {} programs, got {}",
            Program::COUNT,
            lookup.len()
        );
    }

    #[test]
    fn test_program_lookup_vim_has_details() {
        let details = PROGRAM_LOOKUP
            .get(&Program::Vim)
            .expect("Vim should be in lookup");
        assert_eq!(details.name, "Vim");
        assert!(!details.installation_methods.is_empty());
    }

    #[test]
    fn test_program_lookup_ripgrep_has_cargo() {
        let details = PROGRAM_LOOKUP
            .get(&Program::Ripgrep)
            .expect("Ripgrep should be in lookup");
        assert!(
            details
                .installation_methods
                .iter()
                .any(|m| matches!(m, InstallationMethod::Cargo(_))),
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
            assert!(
                !details.description.is_empty(),
                "{:?} has empty description",
                program
            );
            assert!(
                !details.website.is_empty(),
                "{:?} has empty website",
                program
            );
            assert!(
                !details.os_availability.is_empty(),
                "{:?} has no OS availability",
                program
            );
        }
    }

    #[test]
    fn test_all_program_variants_in_lookup() {
        for program in Program::iter() {
            assert!(
                PROGRAM_LOOKUP.contains_key(&program),
                "{:?} should have an entry in PROGRAM_LOOKUP",
                program
            );
        }
    }
}
