//! Program inventory with metadata lookup.
//!
//! This module provides the `Program` tagged union spanning all program categories
//! and the `PROGRAM_LOOKUP` static map for accessing their `ProgramDetails`.

use std::{collections::HashMap, sync::LazyLock};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::os::OsType;
use crate::programs::enums::{
    AiCli, Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager, TerminalApp, TtsClient,
    Utility,
};
use crate::programs::schema::{ProgramInfo, ProgramMetadata};
use crate::programs::types::{InstallationMethod, ProgramDetails};

/// Unified enum spanning all program categories.
///
/// Each variant wraps a category-specific enum, making the relationship
/// between categories and the unified type structural rather than manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Program {
    Editor(Editor),
    Utility(Utility),
    LanguagePackageManager(LanguagePackageManager),
    OsPackageManager(OsPackageManager),
    TtsClient(TtsClient),
    TerminalApp(TerminalApp),
    HeadlessAudio(HeadlessAudio),
    AiCli(AiCli),
}

// ============================================================================
// ProgramMetadata implementation
// ============================================================================

impl ProgramMetadata for Program {
    fn info(&self) -> &'static ProgramInfo {
        match self {
            Program::Editor(e) => e.info(),
            Program::Utility(u) => u.info(),
            Program::LanguagePackageManager(l) => l.info(),
            Program::OsPackageManager(o) => o.info(),
            Program::TtsClient(t) => t.info(),
            Program::TerminalApp(t) => t.info(),
            Program::HeadlessAudio(h) => h.info(),
            Program::AiCli(a) => a.info(),
        }
    }
}

// ============================================================================
// From conversions
// ============================================================================

impl From<Editor> for Program {
    fn from(e: Editor) -> Self {
        Program::Editor(e)
    }
}

impl From<Utility> for Program {
    fn from(u: Utility) -> Self {
        Program::Utility(u)
    }
}

impl From<LanguagePackageManager> for Program {
    fn from(l: LanguagePackageManager) -> Self {
        Program::LanguagePackageManager(l)
    }
}

impl From<OsPackageManager> for Program {
    fn from(o: OsPackageManager) -> Self {
        Program::OsPackageManager(o)
    }
}

impl From<TtsClient> for Program {
    fn from(t: TtsClient) -> Self {
        Program::TtsClient(t)
    }
}

impl From<TerminalApp> for Program {
    fn from(t: TerminalApp) -> Self {
        Program::TerminalApp(t)
    }
}

impl From<HeadlessAudio> for Program {
    fn from(h: HeadlessAudio) -> Self {
        Program::HeadlessAudio(h)
    }
}

impl From<AiCli> for Program {
    fn from(a: AiCli) -> Self {
        Program::AiCli(a)
    }
}

// ============================================================================
// Display, Serialize, Deserialize
// ============================================================================

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.binary_name())
    }
}

impl Serialize for Program {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.binary_name())
    }
}

impl<'de> Deserialize<'de> for Program {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Program::from_binary_name(&name)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown program: {}", name)))
    }
}

// ============================================================================
// Helper methods
// ============================================================================

impl Program {
    /// Look up a Program by binary name.
    pub fn from_binary_name(name: &str) -> Option<Self> {
        Editor::iter()
            .find(|e| e.binary_name() == name)
            .map(Program::Editor)
            .or_else(|| {
                Utility::iter()
                    .find(|u| u.binary_name() == name)
                    .map(Program::Utility)
            })
            .or_else(|| {
                LanguagePackageManager::iter()
                    .find(|l| l.binary_name() == name)
                    .map(Program::LanguagePackageManager)
            })
            .or_else(|| {
                OsPackageManager::iter()
                    .find(|o| o.binary_name() == name)
                    .map(Program::OsPackageManager)
            })
            .or_else(|| {
                TtsClient::iter()
                    .find(|t| t.binary_name() == name)
                    .map(Program::TtsClient)
            })
            .or_else(|| {
                TerminalApp::iter()
                    .find(|t| t.binary_name() == name)
                    .map(Program::TerminalApp)
            })
            .or_else(|| {
                HeadlessAudio::iter()
                    .find(|h| h.binary_name() == name)
                    .map(Program::HeadlessAudio)
            })
            .or_else(|| {
                AiCli::iter()
                    .find(|a| a.binary_name() == name)
                    .map(Program::AiCli)
            })
    }

    /// Iterate over all programs across all categories.
    pub fn iter() -> impl Iterator<Item = Program> {
        Editor::iter()
            .map(Program::from)
            .chain(Utility::iter().map(Program::from))
            .chain(LanguagePackageManager::iter().map(Program::from))
            .chain(OsPackageManager::iter().map(Program::from))
            .chain(TtsClient::iter().map(Program::from))
            .chain(TerminalApp::iter().map(Program::from))
            .chain(HeadlessAudio::iter().map(Program::from))
            .chain(AiCli::iter().map(Program::from))
    }
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

// macOS afplay is pre-installed, no installation needed
static MACOS_AFPLAY_INSTALL: &[InstallationMethod] = &[];

static PULSEAUDIO_PAPLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("pulseaudio-utils"),
    InstallationMethod::Dnf("pulseaudio-utils"),
    InstallationMethod::Pacman("pulseaudio"),
];

// pacat is part of pulseaudio-utils, same as paplay
static PULSEAUDIO_PACAT_INSTALL: &[InstallationMethod] = PULSEAUDIO_PAPLAY_INSTALL;

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

// AI CLI Tools
static CLAUDE_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Npm("@anthropic-ai/claude-code")];

static OPENCODE_INSTALL: &[InstallationMethod] = &[InstallationMethod::GoModules(
    "github.com/opencode-ai/opencode@latest",
)];

static ROO_INSTALL: &[InstallationMethod] = &[InstallationMethod::RemoteBash(
    "https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/install.sh",
)];

static GEMINI_CLI_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("@google/gemini-cli")];

static AIDER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Pip("aider-chat"),
    InstallationMethod::Uv("aider-chat"),
    InstallationMethod::Brew("aider"),
];

static CODEX_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("@openai/codex")];

static GOOSE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("goose"),
    InstallationMethod::Pip("goose-ai"),
];

static KIMI_CLI_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Uv("kimi-cli"),
    InstallationMethod::RemoteBash("https://code.kimi.com/install.sh"),
];

static QWEN_CLI_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Npm("@qwen-code/qwen-code"),
    InstallationMethod::Brew("qwen-code"),
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
        Program::Editor(Editor::Vi),
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
        Program::Editor(Editor::Vim),
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
        Program::Editor(Editor::Neovim),
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
        Program::Editor(Editor::Emacs),
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
        Program::Editor(Editor::XEmacs),
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
        Program::Editor(Editor::Nano),
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
        Program::Editor(Editor::Helix),
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
        Program::Editor(Editor::VSCode),
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
        Program::Editor(Editor::VSCodium),
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
        Program::Editor(Editor::Sublime),
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
        Program::Editor(Editor::Zed),
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
        Program::Editor(Editor::Micro),
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
        Program::Editor(Editor::Kakoune),
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
        Program::Editor(Editor::Amp),
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
        Program::Editor(Editor::Lapce),
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
        Program::Editor(Editor::PhpStorm),
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
        Program::Editor(Editor::IntellijIdea),
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
        Program::Editor(Editor::PyCharm),
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
        Program::Editor(Editor::WebStorm),
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
        Program::Editor(Editor::CLion),
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
        Program::Editor(Editor::GoLand),
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
        Program::Editor(Editor::Rider),
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
        Program::Editor(Editor::TextMate),
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
        Program::Editor(Editor::BBEdit),
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
        Program::Editor(Editor::Geany),
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
        Program::Editor(Editor::Kate),
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
        Program::Utility(Utility::Ripgrep),
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
        Program::Utility(Utility::Bat),
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
        Program::Utility(Utility::Fd),
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
        Program::Utility(Utility::Fzf),
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
        Program::Utility(Utility::Exa),
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
        Program::Utility(Utility::Eza),
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
        Program::Utility(Utility::Dust),
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
        Program::Utility(Utility::Procs),
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
        Program::Utility(Utility::Bottom),
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
        Program::Utility(Utility::Zoxide),
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
        Program::Utility(Utility::Direnv),
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
        Program::Utility(Utility::Tealdeer),
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
        Program::Utility(Utility::Jq),
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
        Program::Utility(Utility::Gh),
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
        Program::Utility(Utility::Lazygit),
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
        Program::Utility(Utility::Delta),
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
        Program::Utility(Utility::Starship),
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
        Program::Utility(Utility::Htop),
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
        Program::Utility(Utility::Btop),
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
        Program::Utility(Utility::Tmux),
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
        Program::Utility(Utility::Zellij),
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
        Program::Utility(Utility::Httpie),
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
        Program::Utility(Utility::Curlie),
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
        Program::Utility(Utility::Mise),
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
        Program::Utility(Utility::Hyperfine),
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
        Program::Utility(Utility::Tokei),
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
        Program::Utility(Utility::Xh),
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
        Program::Utility(Utility::Curl),
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
        Program::Utility(Utility::Wget),
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
        Program::Utility(Utility::Iperf3),
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
        Program::OsPackageManager(OsPackageManager::Brew),
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
        Program::LanguagePackageManager(LanguagePackageManager::Cargo),
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
        Program::LanguagePackageManager(LanguagePackageManager::Npm),
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
        Program::LanguagePackageManager(LanguagePackageManager::Pnpm),
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
        Program::LanguagePackageManager(LanguagePackageManager::Yarn),
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
        Program::LanguagePackageManager(LanguagePackageManager::Bun),
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
        Program::LanguagePackageManager(LanguagePackageManager::GoModules),
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
        Program::LanguagePackageManager(LanguagePackageManager::Composer),
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
        Program::LanguagePackageManager(LanguagePackageManager::SwiftPm),
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
        Program::LanguagePackageManager(LanguagePackageManager::Luarocks),
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
        Program::LanguagePackageManager(LanguagePackageManager::Vcpkg),
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
        Program::LanguagePackageManager(LanguagePackageManager::Conan),
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
        Program::LanguagePackageManager(LanguagePackageManager::Nuget),
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
        Program::LanguagePackageManager(LanguagePackageManager::Hex),
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
        Program::LanguagePackageManager(LanguagePackageManager::Pip),
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
        Program::LanguagePackageManager(LanguagePackageManager::Uv),
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
        Program::LanguagePackageManager(LanguagePackageManager::Poetry),
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
        Program::LanguagePackageManager(LanguagePackageManager::Cpan),
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
        Program::LanguagePackageManager(LanguagePackageManager::Cpanm),
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
        Program::OsPackageManager(OsPackageManager::Apt),
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
        Program::OsPackageManager(OsPackageManager::Nala),
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
        Program::OsPackageManager(OsPackageManager::Dnf),
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
        Program::OsPackageManager(OsPackageManager::Pacman),
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
        Program::OsPackageManager(OsPackageManager::Winget),
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
        Program::OsPackageManager(OsPackageManager::Chocolatey),
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
        Program::OsPackageManager(OsPackageManager::Scoop),
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
        Program::OsPackageManager(OsPackageManager::Nix),
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
        Program::TtsClient(TtsClient::Say),
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
        Program::TtsClient(TtsClient::EspeakNg),
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
        Program::TtsClient(TtsClient::Espeak),
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
        Program::TtsClient(TtsClient::Festival),
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
        Program::TtsClient(TtsClient::Mimic),
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
        Program::TtsClient(TtsClient::Mimic3),
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
        Program::TtsClient(TtsClient::Piper),
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
        Program::TtsClient(TtsClient::Echogarden),
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
        Program::TtsClient(TtsClient::Balcon),
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
        Program::TtsClient(TtsClient::WindowsSapi),
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
        Program::TtsClient(TtsClient::GttsCli),
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
        Program::TtsClient(TtsClient::CoquiTts),
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
        Program::TtsClient(TtsClient::SherpaOnnx),
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
        Program::TtsClient(TtsClient::KokoroTts),
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
        Program::TtsClient(TtsClient::Pico2Wave),
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
        Program::HeadlessAudio(HeadlessAudio::Mpv),
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
        Program::HeadlessAudio(HeadlessAudio::Ffplay),
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
        Program::HeadlessAudio(HeadlessAudio::Sox),
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
        Program::HeadlessAudio(HeadlessAudio::Vlc),
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
        Program::HeadlessAudio(HeadlessAudio::MPlayer),
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
        Program::HeadlessAudio(HeadlessAudio::GstreamerGstPlay),
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
        Program::HeadlessAudio(HeadlessAudio::Mpg123),
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
        Program::HeadlessAudio(HeadlessAudio::Ogg123),
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
        Program::HeadlessAudio(HeadlessAudio::AlsaAplay),
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
        Program::HeadlessAudio(HeadlessAudio::MacOsAfplay),
        ProgramDetails::full(
            "afplay",
            "macOS native audio file player",
            MACOS_ONLY,
            "https://ss64.com/osx/afplay.html",
            None,
            MACOS_AFPLAY_INSTALL,
        ),
    );

    lookup.insert(
        Program::HeadlessAudio(HeadlessAudio::PulseaudioPaplay),
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
        Program::HeadlessAudio(HeadlessAudio::PulseaudioPacat),
        ProgramDetails::full(
            "pacat",
            "PulseAudio raw audio streaming",
            LINUX_ONLY,
            "https://www.freedesktop.org/wiki/Software/PulseAudio/",
            None,
            PULSEAUDIO_PACAT_INSTALL,
        ),
    );

    lookup.insert(
        Program::HeadlessAudio(HeadlessAudio::Pipewire),
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
        Program::TerminalApp(TerminalApp::Alacritty),
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
        Program::TerminalApp(TerminalApp::Kitty),
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
        Program::TerminalApp(TerminalApp::WezTerm),
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
        Program::TerminalApp(TerminalApp::ITerm2),
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
        Program::TerminalApp(TerminalApp::Ghostty),
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
        Program::TerminalApp(TerminalApp::Warp),
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
        Program::TerminalApp(TerminalApp::Rio),
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
        Program::TerminalApp(TerminalApp::Tabby),
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
        Program::TerminalApp(TerminalApp::Foot),
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
        Program::TerminalApp(TerminalApp::GnomeTerminal),
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
        Program::TerminalApp(TerminalApp::Konsole),
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
        Program::TerminalApp(TerminalApp::XfceTerminal),
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
        Program::TerminalApp(TerminalApp::Terminology),
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
        Program::TerminalApp(TerminalApp::St),
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
        Program::TerminalApp(TerminalApp::Xterm),
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
        Program::TerminalApp(TerminalApp::Hyper),
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
        Program::TerminalApp(TerminalApp::WindowsTerminal),
        ProgramDetails::full(
            "Windows Terminal",
            "Modern terminal for Windows",
            WINDOWS_ONLY,
            "https://github.com/microsoft/terminal",
            Some("https://github.com/microsoft/terminal"),
            WINDOWS_TERMINAL_INSTALL,
        ),
    );

    // ========================================================================
    // AI CLI Tools (9 entries)
    // ========================================================================
    lookup.insert(
        Program::AiCli(AiCli::Claude),
        ProgramDetails::full(
            "Claude Code",
            "Anthropic's agentic coding tool",
            ALL_OS,
            "https://docs.anthropic.com/en/docs/claude-code",
            Some("https://github.com/anthropics/claude-code"),
            CLAUDE_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::Opencode),
        ProgramDetails::full(
            "OpenCode",
            "AI-powered coding assistant CLI",
            ALL_OS,
            "https://github.com/opencode-ai/opencode",
            Some("https://github.com/opencode-ai/opencode"),
            OPENCODE_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::Roo),
        ProgramDetails::full(
            "Roo Code",
            "AI pair programming in your terminal",
            ALL_OS,
            "https://github.com/RooVetGit/Roo-Code",
            Some("https://github.com/RooVetGit/Roo-Code"),
            ROO_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::GeminiCli),
        ProgramDetails::full(
            "Gemini CLI",
            "Google's Gemini AI in the terminal",
            ALL_OS,
            "https://github.com/google-gemini/gemini-cli",
            Some("https://github.com/google-gemini/gemini-cli"),
            GEMINI_CLI_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::Aider),
        ProgramDetails::full(
            "Aider",
            "AI pair programming in your terminal",
            ALL_OS,
            "https://aider.chat/",
            Some("https://github.com/paul-gauthier/aider"),
            AIDER_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::Codex),
        ProgramDetails::full(
            "Codex CLI",
            "OpenAI lightweight coding agent",
            ALL_OS,
            "https://github.com/openai/codex",
            Some("https://github.com/openai/codex"),
            CODEX_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::Goose),
        ProgramDetails::full(
            "Goose",
            "Block's AI developer agent",
            ALL_OS,
            "https://github.com/block/goose",
            Some("https://github.com/block/goose"),
            GOOSE_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::KimiCli),
        ProgramDetails::full(
            "Kimi Code CLI",
            "AI agent that runs in the terminal",
            ALL_OS,
            "https://moonshotai.github.io/kimi-cli/",
            Some("https://github.com/MoonshotAI/kimi-cli"),
            KIMI_CLI_INSTALL,
        ),
    );

    lookup.insert(
        Program::AiCli(AiCli::QwenCli),
        ProgramDetails::full(
            "Qwen Code CLI",
            "Qwen's AI coding agent",
            ALL_OS,
            "https://qwenlm.github.io/qwen-code-docs/",
            Some("https://github.com/QwenLM/qwen-code"),
            QWEN_CLI_INSTALL,
        ),
    );

    lookup
});

#[cfg(test)]
mod tests {
    use super::*;
    use strum::EnumCount;

    #[test]
    fn test_program_from_category_enums() {
        let p = Program::from(Editor::Vim);
        assert_eq!(p.display_name(), "Vim");

        let p = Program::from(Utility::Ripgrep);
        assert_eq!(p.binary_name(), "rg");
    }

    #[test]
    fn test_program_display_uses_binary_name() {
        let p = Program::from(Editor::Vim);
        assert_eq!(p.to_string(), "vim");
    }

    #[test]
    fn test_program_serde_roundtrip() {
        let p = Program::from(Editor::Vim);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"vim\"");
        let p2: Program = serde_json::from_str(&json).unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn test_program_iter_covers_all_categories() {
        let all: Vec<Program> = Program::iter().collect();
        let expected = Editor::COUNT
            + Utility::COUNT
            + LanguagePackageManager::COUNT
            + OsPackageManager::COUNT
            + TtsClient::COUNT
            + TerminalApp::COUNT
            + HeadlessAudio::COUNT
            + AiCli::COUNT;
        assert_eq!(all.len(), expected);
    }

    #[test]
    fn test_all_programs_have_valid_metadata() {
        for program in Program::iter() {
            let info = program.info();
            assert!(
                !info.display_name.is_empty(),
                "{:?} has empty display_name",
                program
            );
            assert!(
                !info.description.is_empty(),
                "{:?} has empty description",
                program
            );
            assert!(!info.website.is_empty(), "{:?} has empty website", program);
        }
    }

    #[test]
    fn test_program_from_binary_name() {
        assert_eq!(
            Program::from_binary_name("vim"),
            Some(Program::Editor(Editor::Vim))
        );
        assert_eq!(
            Program::from_binary_name("rg"),
            Some(Program::Utility(Utility::Ripgrep))
        );
        assert_eq!(Program::from_binary_name("nonexistent"), None);
    }

    #[test]
    fn test_program_copy_derive() {
        let p = Program::from(Editor::Vim);
        let p2 = p;
        assert_eq!(p, p2);
    }

    // Keep PROGRAM_LOOKUP tests until Task 10 removes it
    #[test]
    fn test_program_lookup_has_entries() {
        let lookup = &*PROGRAM_LOOKUP;
        assert!(!lookup.is_empty());
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
