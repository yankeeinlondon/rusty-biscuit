//! Metadata tables and implementations for program categories.

use crate::programs::contract::{CategoryEnum, InstallationMethod};
use crate::programs::schema::{ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy};

use super::categories::{
    ALL_OS, AiCli, Editor, HeadlessAudio, LINUX_ONLY, LanguagePackageManager, MACOS_ONLY,
    NotificationHelper, OsPackageManager, TerminalApp, TestRunner, TtsClient, UNIX_ONLY, Utility,
    WINDOWS_ONLY,
};

pub(crate) static BREW_INSTALL: &[InstallationMethod] = &[InstallationMethod::RemoteBash(
    "https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh",
)];

// Editor installation methods
pub(crate) static VIM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vim"),
    InstallationMethod::Apt("vim"),
    InstallationMethod::Dnf("vim"),
    InstallationMethod::Pacman("vim"),
    InstallationMethod::Chocolatey("vim"),
    InstallationMethod::Scoop("vim"),
];
pub(crate) static VI_INSTALL: &[InstallationMethod] = VIM_INSTALL;
pub(crate) static EMACS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("emacs"),
    InstallationMethod::Apt("emacs"),
    InstallationMethod::Dnf("emacs"),
    InstallationMethod::Pacman("emacs"),
    InstallationMethod::Chocolatey("emacs"),
    InstallationMethod::Scoop("emacs"),
];
pub(crate) static XEMACS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("xemacs"),
    InstallationMethod::Apt("xemacs"),
    InstallationMethod::Dnf("xemacs"),
    InstallationMethod::Pacman("xemacs"),
];
pub(crate) static NANO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("nano"),
    InstallationMethod::Apt("nano"),
    InstallationMethod::Dnf("nano"),
    InstallationMethod::Pacman("nano"),
    InstallationMethod::Chocolatey("nano"),
    InstallationMethod::Scoop("nano"),
];
pub(crate) static NEOVIM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("neovim"),
    InstallationMethod::Apt("neovim"),
    InstallationMethod::Dnf("neovim"),
    InstallationMethod::Pacman("neovim"),
    InstallationMethod::Chocolatey("neovim"),
    InstallationMethod::Scoop("neovim"),
    InstallationMethod::Cargo("neovim"),
];
pub(crate) static HELIX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("helix"),
    InstallationMethod::Cargo("helix"),
    InstallationMethod::Pacman("helix"),
    InstallationMethod::Scoop("helix"),
];
pub(crate) static VSCODE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("visual-studio-code"),
    InstallationMethod::Chocolatey("vscode"),
    InstallationMethod::Scoop("vscode"),
    InstallationMethod::Winget("Microsoft.VisualStudioCode"),
];
pub(crate) static VSCODIUM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vscodium"),
    InstallationMethod::Chocolatey("vscodium"),
    InstallationMethod::Scoop("vscodium"),
    InstallationMethod::Winget("VSCodium.VSCodium"),
];
pub(crate) static SUBLIME_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("sublime-text"),
    InstallationMethod::Apt("sublime-text"),
    InstallationMethod::Dnf("sublime-text"),
    InstallationMethod::Pacman("sublime-text"),
    InstallationMethod::Chocolatey("sublimetext4"),
    InstallationMethod::Scoop("sublime-text"),
    InstallationMethod::Winget("SublimeHQ.SublimeText.4"),
];
pub(crate) static ZED_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("zed")];
pub(crate) static MICRO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("micro"),
    InstallationMethod::Apt("micro"),
    InstallationMethod::Dnf("micro"),
    InstallationMethod::Pacman("micro"),
    InstallationMethod::Scoop("micro"),
];
pub(crate) static KAKOUNE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("kakoune"),
    InstallationMethod::Apt("kakoune"),
    InstallationMethod::Dnf("kakoune"),
    InstallationMethod::Pacman("kakoune"),
];
pub(crate) static AMP_INSTALL: &[InstallationMethod] = &[InstallationMethod::Cargo("amp")];
pub(crate) static LAPCE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("lapce"),
    InstallationMethod::Cargo("lapce"),
];
pub(crate) static PHPSTORM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("phpstorm"),
    InstallationMethod::Winget("JetBrains.PhpStorm"),
];
pub(crate) static INTELLIJ_IDEA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("intellij-idea"),
    InstallationMethod::Winget("JetBrains.IntelliJIDEA.Community"),
];
pub(crate) static PYCHARM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("pycharm"),
    InstallationMethod::Winget("JetBrains.PyCharm.Community"),
];
pub(crate) static WEBSTORM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("webstorm"),
    InstallationMethod::Winget("JetBrains.WebStorm"),
];
pub(crate) static CLION_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("clion"),
    InstallationMethod::Winget("JetBrains.CLion"),
];
pub(crate) static GOLAND_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("goland"),
    InstallationMethod::Winget("JetBrains.GoLand"),
];
pub(crate) static RIDER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("rider"),
    InstallationMethod::Winget("JetBrains.Rider"),
];
pub(crate) static TEXTMATE_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("textmate")];
pub(crate) static BBEDIT_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("bbedit")];
pub(crate) static GEANY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("geany"),
    InstallationMethod::Apt("geany"),
    InstallationMethod::Dnf("geany"),
    InstallationMethod::Pacman("geany"),
];
pub(crate) static KATE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("kate"),
    InstallationMethod::Apt("kate"),
    InstallationMethod::Dnf("kate"),
    InstallationMethod::Pacman("kate"),
];

/// Metadata lookup table for editors.
pub(crate) static EDITOR_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "vi",
        display_name: "Vi",
        description: "The classic vi editor",
        website: "https://www.vim.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/vim/vim"),
        installation_methods: VI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "vim",
        display_name: "Vim",
        description: "Vi IMproved text editor",
        website: "https://www.vim.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/vim/vim"),
        installation_methods: VIM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nvim",
        display_name: "Neovim",
        description: "Hyperextensible Vim-based text editor",
        website: "https://neovim.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/neovim/neovim"),
        installation_methods: NEOVIM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "emacs",
        display_name: "GNU Emacs",
        description: "Extensible, customizable text editor",
        website: "https://www.gnu.org/software/emacs/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::AfterPrefix,
        version_regex: None,
        version_prefix: Some("GNU Emacs "),
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://git.savannah.gnu.org/cgit/emacs.git"),
        installation_methods: EMACS_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "xemacs",
        display_name: "XEmacs",
        description: "A version of Emacs that branched from GNU Emacs",
        website: "http://www.xemacs.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/xemacs/xemacs"),
        installation_methods: XEMACS_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nano",
        display_name: "GNU nano",
        description: "Small and friendly text editor",
        website: "https://www.nano-editor.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::AfterPrefix,
        version_regex: None,
        version_prefix: Some("nano "),
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://git.savannah.gnu.org/cgit/nano.git"),
        installation_methods: NANO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "hx",
        display_name: "Helix",
        description: "Post-modern modal text editor",
        website: "https://helix-editor.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/helix-editor/helix"),
        installation_methods: HELIX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "code",
        display_name: "Visual Studio Code",
        description: "Code editor for modern web and cloud applications",
        website: "https://code.visualstudio.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/microsoft/vscode"),
        installation_methods: VSCODE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "codium",
        display_name: "VSCodium",
        description: "Free/libre open source binaries of VS Code",
        website: "https://vscodium.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/VSCodium/vscodium"),
        installation_methods: VSCODIUM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "subl",
        display_name: "Sublime Text",
        description: "Sophisticated text editor for code and prose",
        website: "https://www.sublimetext.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/sublimehq"),
        installation_methods: SUBLIME_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "zed",
        display_name: "Zed",
        description: "High-performance multiplayer code editor",
        website: "https://zed.dev/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: Some("https://github.com/zed-industries/zed"),
        installation_methods: ZED_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "micro",
        display_name: "Micro",
        description: "Modern and intuitive terminal-based text editor",
        website: "https://micro-editor.github.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/zyedidia/micro"),
        installation_methods: MICRO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "kak",
        display_name: "Kakoune",
        description: "Modal editor with selection-based editing model",
        website: "https://kakoune.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/mawww/kakoune"),
        installation_methods: KAKOUNE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "amp",
        display_name: "Amp",
        description: "Modal text editor for the terminal inspired by Vi",
        website: "https://amp.readme.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/jmacdonald/amp"),
        installation_methods: AMP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "lapce",
        display_name: "Lapce",
        description: "Lightning-fast code editor written in Rust",
        website: "https://lapce.dev/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/lapce/lapce"),
        installation_methods: LAPCE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "phpstorm",
        display_name: "PhpStorm",
        description: "Lightning-smart PHP IDE by JetBrains",
        website: "https://www.jetbrains.com/phpstorm/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://www.jetbrains.com/phpstorm/"),
        installation_methods: PHPSTORM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "idea",
        display_name: "IntelliJ IDEA",
        description: "Capable and ergonomic IDE for JVM-based languages",
        website: "https://www.jetbrains.com/idea/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://www.jetbrains.com/idea/"),
        installation_methods: INTELLIJ_IDEA_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pycharm",
        display_name: "PyCharm",
        description: "The Python IDE for professional developers",
        website: "https://www.jetbrains.com/pycharm/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://www.jetbrains.com/pycharm/"),
        installation_methods: PYCHARM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "webstorm",
        display_name: "WebStorm",
        description: "The smartest JavaScript IDE",
        website: "https://www.jetbrains.com/webstorm/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://www.jetbrains.com/webstorm/"),
        installation_methods: WEBSTORM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "clion",
        display_name: "CLion",
        description: "Cross-platform C and C++ IDE",
        website: "https://www.jetbrains.com/clion/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://www.jetbrains.com/clion/"),
        installation_methods: CLION_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "goland",
        display_name: "GoLand",
        description: "Cross-platform Go IDE",
        website: "https://www.jetbrains.com/go/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://www.jetbrains.com/go/"),
        installation_methods: GOLAND_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "rider",
        display_name: "Rider",
        description: "Fast and powerful cross-platform .NET IDE",
        website: "https://www.jetbrains.com/rider/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://www.jetbrains.com/rider/"),
        installation_methods: RIDER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mate",
        display_name: "TextMate",
        description: "Versatile plain text editor for macOS",
        website: "https://macromates.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: Some("https://github.com/textmate/textmate"),
        installation_methods: TEXTMATE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "bbedit",
        display_name: "BBEdit",
        description: "Professional HTML and text editor for macOS",
        website: "https://www.barebones.com/products/bbedit/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: None,
        installation_methods: BBEDIT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "geany",
        display_name: "Geany",
        description: "Powerful, stable and lightweight text editor",
        website: "https://www.geany.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/geany/geany"),
        installation_methods: GEANY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "kate",
        display_name: "Kate",
        description: "Multi-document, multi-view text editor by KDE",
        website: "https://kate-editor.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://invent.kde.org/utilities/kate"),
        installation_methods: KATE_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for Editor {
    fn info(&self) -> &'static ProgramInfo {
        &EDITOR_INFO[*self as usize]
    }
}

impl CategoryEnum for Editor {
    fn category_name() -> &'static str {
        "editors"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            Editor::Vi => "vi",
            Editor::Vim => "vim",
            Editor::Neovim => "neovim",
            Editor::Emacs => "emacs",
            Editor::XEmacs => "xemacs",
            Editor::Nano => "nano",
            Editor::Helix => "helix",
            Editor::VSCode => "vscode",
            Editor::VSCodium => "vscodium",
            Editor::Sublime => "sublime",
            Editor::Zed => "zed",
            Editor::Micro => "micro",
            Editor::Kakoune => "kakoune",
            Editor::Amp => "amp",
            Editor::Lapce => "lapce",
            Editor::PhpStorm => "phpstorm",
            Editor::IntellijIdea => "intellij_idea",
            Editor::PyCharm => "pycharm",
            Editor::WebStorm => "webstorm",
            Editor::CLion => "clion",
            Editor::GoLand => "goland",
            Editor::Rider => "rider",
            Editor::TextMate => "textmate",
            Editor::BBEdit => "bbedit",
            Editor::Geany => "geany",
            Editor::Kate => "kate",
        }
    }
}

// Utility installation methods
pub(crate) static EXA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("exa"),
    InstallationMethod::Cargo("exa"),
    InstallationMethod::Apt("exa"),
    InstallationMethod::Pacman("exa"),
];
pub(crate) static EZA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("eza"),
    InstallationMethod::Cargo("eza"),
    InstallationMethod::Pacman("eza"),
];
pub(crate) static RIPGREP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("ripgrep"),
    InstallationMethod::Cargo("ripgrep"),
    InstallationMethod::Apt("ripgrep"),
    InstallationMethod::Dnf("ripgrep"),
    InstallationMethod::Pacman("ripgrep"),
    InstallationMethod::Chocolatey("ripgrep"),
    InstallationMethod::Scoop("ripgrep"),
];
pub(crate) static DUST_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("dust"),
    InstallationMethod::Cargo("dust"),
    InstallationMethod::Apt("dust"),
    InstallationMethod::Dnf("dust"),
    InstallationMethod::Pacman("dust"),
];
pub(crate) static BAT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("bat"),
    InstallationMethod::Cargo("bat"),
    InstallationMethod::Apt("bat"),
    InstallationMethod::Dnf("bat"),
    InstallationMethod::Pacman("bat"),
    InstallationMethod::Chocolatey("bat"),
    InstallationMethod::Scoop("bat"),
];
pub(crate) static FD_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("fd"),
    InstallationMethod::Cargo("fd-find"),
    InstallationMethod::Apt("fd-find"),
    InstallationMethod::Dnf("fd-find"),
    InstallationMethod::Pacman("fd"),
    InstallationMethod::Chocolatey("fd"),
    InstallationMethod::Scoop("fd"),
];
pub(crate) static PROCS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("procs"),
    InstallationMethod::Cargo("procs"),
    InstallationMethod::Apt("procs"),
    InstallationMethod::Pacman("procs"),
];
pub(crate) static BOTTOM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("bottom"),
    InstallationMethod::Cargo("bottom"),
    InstallationMethod::Apt("bottom"),
    InstallationMethod::Pacman("bottom"),
];
pub(crate) static FZF_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("fzf"),
    InstallationMethod::Apt("fzf"),
    InstallationMethod::Dnf("fzf"),
    InstallationMethod::Pacman("fzf"),
    InstallationMethod::Chocolatey("fzf"),
    InstallationMethod::Scoop("fzf"),
];
pub(crate) static ZOXIDE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("zoxide"),
    InstallationMethod::Cargo("zoxide"),
    InstallationMethod::Apt("zoxide"),
    InstallationMethod::Dnf("zoxide"),
    InstallationMethod::Pacman("zoxide"),
];
pub(crate) static STARSHIP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("starship"),
    InstallationMethod::Cargo("starship"),
    InstallationMethod::Chocolatey("starship"),
    InstallationMethod::Scoop("starship"),
    InstallationMethod::RemoteBash("https://starship.rs/install.sh"),
];
pub(crate) static DIRENV_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("direnv"),
    InstallationMethod::Apt("direnv"),
    InstallationMethod::Dnf("direnv"),
    InstallationMethod::Pacman("direnv"),
];
pub(crate) static JQ_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("jq"),
    InstallationMethod::Apt("jq"),
    InstallationMethod::Dnf("jq"),
    InstallationMethod::Pacman("jq"),
    InstallationMethod::Chocolatey("jq"),
    InstallationMethod::Scoop("jq"),
];
pub(crate) static DELTA_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("git-delta"),
    InstallationMethod::Cargo("git-delta"),
    InstallationMethod::Pacman("git-delta"),
    InstallationMethod::Chocolatey("delta"),
    InstallationMethod::Scoop("delta"),
];
pub(crate) static TEALDEER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tealdeer"),
    InstallationMethod::Cargo("tealdeer"),
    InstallationMethod::Apt("tealdeer"),
    InstallationMethod::Pacman("tealdeer"),
];
pub(crate) static LAZYGIT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("lazygit"),
    InstallationMethod::Pacman("lazygit"),
    InstallationMethod::Scoop("lazygit"),
    InstallationMethod::GoModules("github.com/jesseduffield/lazygit@latest"),
];
pub(crate) static GH_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("gh"),
    InstallationMethod::Apt("gh"),
    InstallationMethod::Dnf("gh"),
    InstallationMethod::Pacman("github-cli"),
    InstallationMethod::Chocolatey("gh"),
    InstallationMethod::Scoop("gh"),
    InstallationMethod::Winget("GitHub.cli"),
];
pub(crate) static HTOP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("htop"),
    InstallationMethod::Apt("htop"),
    InstallationMethod::Dnf("htop"),
    InstallationMethod::Pacman("htop"),
];
pub(crate) static BTOP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("btop"),
    InstallationMethod::Apt("btop"),
    InstallationMethod::Dnf("btop"),
    InstallationMethod::Pacman("btop"),
];
pub(crate) static TMUX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tmux"),
    InstallationMethod::Apt("tmux"),
    InstallationMethod::Dnf("tmux"),
    InstallationMethod::Pacman("tmux"),
];
pub(crate) static ZELLIJ_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("zellij"),
    InstallationMethod::Cargo("zellij"),
    InstallationMethod::Apt("zellij"),
    InstallationMethod::Pacman("zellij"),
];
pub(crate) static HTTPIE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("httpie"),
    InstallationMethod::Apt("httpie"),
    InstallationMethod::Dnf("httpie"),
    InstallationMethod::Pacman("httpie"),
    InstallationMethod::Pip("httpie"),
];
pub(crate) static CURLIE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("curlie"),
    InstallationMethod::Cargo("curlie"),
    InstallationMethod::Scoop("curlie"),
];
pub(crate) static MISE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mise"),
    InstallationMethod::Cargo("mise"),
    InstallationMethod::Scoop("mise"),
];
pub(crate) static HYPERFINE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("hyperfine"),
    InstallationMethod::Cargo("hyperfine"),
    InstallationMethod::Apt("hyperfine"),
    InstallationMethod::Pacman("hyperfine"),
];
pub(crate) static TOKEI_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tokei"),
    InstallationMethod::Cargo("tokei"),
    InstallationMethod::Apt("tokei"),
    InstallationMethod::Pacman("tokei"),
];
pub(crate) static XH_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("xh"),
    InstallationMethod::Cargo("xh"),
    InstallationMethod::Apt("xh"),
    InstallationMethod::Pacman("xh"),
    InstallationMethod::Scoop("xh"),
];
pub(crate) static CURL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("curl"),
    InstallationMethod::Apt("curl"),
    InstallationMethod::Dnf("curl"),
    InstallationMethod::Pacman("curl"),
];
pub(crate) static WGET_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("wget"),
    InstallationMethod::Apt("wget"),
    InstallationMethod::Dnf("wget"),
    InstallationMethod::Pacman("wget"),
];
pub(crate) static IPERF3_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("iperf3"),
    InstallationMethod::Apt("iperf3"),
    InstallationMethod::Dnf("iperf3"),
    InstallationMethod::Pacman("iperf3"),
];

/// Metadata lookup table for utilities.
pub(crate) static UTILITY_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "exa",
        display_name: "exa",
        description: "A modern replacement for ls (deprecated)",
        website: "https://the.exa.website/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/ogham/exa"),
        installation_methods: EXA_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "eza",
        display_name: "eza",
        description: "A modern replacement for ls",
        website: "https://eza.rocks/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/eza-community/eza"),
        installation_methods: EZA_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "rg",
        display_name: "ripgrep",
        description: "Fast grep alternative with smart defaults",
        website: "https://github.com/BurntSushi/ripgrep",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/BurntSushi/ripgrep"),
        installation_methods: RIPGREP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "dust",
        display_name: "dust",
        description: "A more intuitive version of du",
        website: "https://github.com/bootandy/dust",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/bootandy/dust"),
        installation_methods: DUST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "bat",
        display_name: "bat",
        description: "A cat clone with syntax highlighting",
        website: "https://github.com/sharkdp/bat",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/sharkdp/bat"),
        installation_methods: BAT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "fd",
        display_name: "fd",
        description: "Simple, fast alternative to find",
        website: "https://github.com/sharkdp/fd",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/sharkdp/fd"),
        installation_methods: FD_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "procs",
        display_name: "procs",
        description: "A modern replacement for ps",
        website: "https://github.com/dalance/procs",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/dalance/procs"),
        installation_methods: PROCS_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "btm",
        display_name: "bottom",
        description: "Cross-platform graphical process monitor",
        website: "https://github.com/ClementTsang/bottom",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/ClementTsang/bottom"),
        installation_methods: BOTTOM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "fzf",
        display_name: "fzf",
        description: "Command-line fuzzy finder",
        website: "https://github.com/junegunn/fzf",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/junegunn/fzf"),
        installation_methods: FZF_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "zoxide",
        display_name: "zoxide",
        description: "Smarter cd command",
        website: "https://github.com/ajeetdsouza/zoxide",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/ajeetdsouza/zoxide"),
        installation_methods: ZOXIDE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "starship",
        display_name: "Starship",
        description: "Minimal, blazing-fast shell prompt",
        website: "https://starship.rs/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/starship/starship"),
        installation_methods: STARSHIP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "direnv",
        display_name: "direnv",
        description: "Environment switcher for the shell",
        website: "https://direnv.net/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/direnv/direnv"),
        installation_methods: DIRENV_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "jq",
        display_name: "jq",
        description: "Command-line JSON processor",
        website: "https://jqlang.github.io/jq/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/jqlang/jq"),
        installation_methods: JQ_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "delta",
        display_name: "delta",
        description: "Viewer for git and diff output",
        website: "https://github.com/dandavison/delta",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/dandavison/delta"),
        installation_methods: DELTA_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "tldr",
        display_name: "tealdeer",
        description: "Fast tldr client for simplified man pages",
        website: "https://github.com/dbrgn/tealdeer",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/dbrgn/tealdeer"),
        installation_methods: TEALDEER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "lazygit",
        display_name: "lazygit",
        description: "Simple terminal UI for git commands",
        website: "https://github.com/jesseduffield/lazygit",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/jesseduffield/lazygit"),
        installation_methods: LAZYGIT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "gh",
        display_name: "GitHub CLI",
        description: "GitHub's official CLI",
        website: "https://cli.github.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/cli/cli"),
        installation_methods: GH_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "htop",
        display_name: "htop",
        description: "Interactive process viewer",
        website: "https://htop.dev/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/htop-dev/htop"),
        installation_methods: HTOP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "btop",
        display_name: "btop",
        description: "Resource monitor with CPU, memory, disk, network stats",
        website: "https://github.com/aristocratos/btop",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/aristocratos/btop"),
        installation_methods: BTOP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "tmux",
        display_name: "tmux",
        description: "Terminal multiplexer",
        website: "https://github.com/tmux/tmux/wiki",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/tmux/tmux"),
        installation_methods: TMUX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "zellij",
        display_name: "Zellij",
        description: "Modern terminal multiplexer",
        website: "https://zellij.dev/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/zellij-org/zellij"),
        installation_methods: ZELLIJ_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "http",
        display_name: "HTTPie",
        description: "User-friendly HTTP client",
        website: "https://httpie.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/httpie/cli"),
        installation_methods: HTTPIE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "curlie",
        display_name: "curlie",
        description: "User-friendly alternative to curl",
        website: "https://github.com/rs/curlie",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/rs/curlie"),
        installation_methods: CURLIE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mise",
        display_name: "mise",
        description: "Polyglot development environment manager",
        website: "https://mise.jdx.dev/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/jdx/mise"),
        installation_methods: MISE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "hyperfine",
        display_name: "hyperfine",
        description: "Command-line benchmarking tool",
        website: "https://github.com/sharkdp/hyperfine",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/sharkdp/hyperfine"),
        installation_methods: HYPERFINE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "tokei",
        display_name: "tokei",
        description: "Count lines of code quickly",
        website: "https://github.com/XAMPPRocky/tokei",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/XAMPPRocky/tokei"),
        installation_methods: TOKEI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "xh",
        display_name: "xh",
        description: "Friendly and fast HTTP client",
        website: "https://github.com/ducaale/xh",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/ducaale/xh"),
        installation_methods: XH_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "curl",
        display_name: "curl",
        description: "Transfer data with URLs",
        website: "https://curl.se/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/curl/curl"),
        installation_methods: CURL_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "wget",
        display_name: "wget",
        description: "Network utility to retrieve content from web servers",
        website: "https://www.gnu.org/software/wget/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://git.savannah.gnu.org/cgit/wget.git"),
        installation_methods: WGET_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "iperf3",
        display_name: "iperf3",
        description: "Network bandwidth measurement tool",
        website: "https://iperf.fr/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/esnet/iperf"),
        installation_methods: IPERF3_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for Utility {
    fn info(&self) -> &'static ProgramInfo {
        &UTILITY_INFO[*self as usize]
    }
}

impl CategoryEnum for Utility {
    fn category_name() -> &'static str {
        "utilities"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            Utility::Exa => "exa",
            Utility::Eza => "eza",
            Utility::Ripgrep => "ripgrep",
            Utility::Dust => "dust",
            Utility::Bat => "bat",
            Utility::Fd => "fd",
            Utility::Procs => "procs",
            Utility::Bottom => "bottom",
            Utility::Fzf => "fzf",
            Utility::Zoxide => "zoxide",
            Utility::Starship => "starship",
            Utility::Direnv => "direnv",
            Utility::Jq => "jq",
            Utility::Delta => "delta",
            Utility::Tealdeer => "tealdeer",
            Utility::Lazygit => "lazygit",
            Utility::Gh => "gh",
            Utility::Htop => "htop",
            Utility::Btop => "btop",
            Utility::Tmux => "tmux",
            Utility::Zellij => "zellij",
            Utility::Httpie => "httpie",
            Utility::Curlie => "curlie",
            Utility::Mise => "mise",
            Utility::Hyperfine => "hyperfine",
            Utility::Tokei => "tokei",
            Utility::Xh => "xh",
            Utility::Curl => "curl",
            Utility::Wget => "wget",
            Utility::Iperf3 => "iperf3",
        }
    }
}

// Language package manager installation methods
pub(crate) static NPM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("node"),
    InstallationMethod::Apt("nodejs"),
    InstallationMethod::Dnf("nodejs"),
    InstallationMethod::Pacman("nodejs"),
    InstallationMethod::Chocolatey("nodejs"),
    InstallationMethod::Scoop("nodejs"),
];
pub(crate) static PNPM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("pnpm"),
    InstallationMethod::Npm("pnpm"),
    InstallationMethod::RemoteBash("https://get.pnpm.io/install.sh"),
];
pub(crate) static YARN_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("yarn"),
    InstallationMethod::Npm("yarn"),
];
pub(crate) static BUN_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("bun"),
    InstallationMethod::Npm("bun"),
];
pub(crate) static CARGO_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::RemoteBash("https://sh.rustup.rs")];
pub(crate) static GO_MODULES_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("go"),
    InstallationMethod::Apt("golang"),
    InstallationMethod::Dnf("golang"),
    InstallationMethod::Pacman("go"),
    InstallationMethod::Chocolatey("golang"),
    InstallationMethod::Scoop("go"),
];
pub(crate) static COMPOSER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("composer"),
    InstallationMethod::Apt("composer"),
    InstallationMethod::Dnf("composer"),
    InstallationMethod::Pacman("composer"),
    InstallationMethod::Chocolatey("composer"),
];
pub(crate) static SWIFTPM_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("swift")];
pub(crate) static LUAROCKS_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("luarocks"),
    InstallationMethod::Apt("luarocks"),
    InstallationMethod::Dnf("luarocks"),
    InstallationMethod::Pacman("luarocks"),
];
pub(crate) static VCPKG_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vcpkg"),
    InstallationMethod::Chocolatey("vcpkg"),
];
pub(crate) static CONAN_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("conan")];
pub(crate) static NUGET_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("nuget"),
    InstallationMethod::Chocolatey("nuget"),
    InstallationMethod::Scoop("nuget"),
];
pub(crate) static HEX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("elixir"),
    InstallationMethod::Apt("elixir"),
    InstallationMethod::Dnf("elixir"),
    InstallationMethod::Pacman("elixir"),
    InstallationMethod::Chocolatey("elixir"),
];
pub(crate) static PIP_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("python3-pip"),
    InstallationMethod::Dnf("python3-pip"),
    InstallationMethod::Pacman("python-pip"),
];
pub(crate) static UV_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("uv"),
    InstallationMethod::Cargo("uv"),
    InstallationMethod::Pip("uv"),
    InstallationMethod::RemoteBash("https://astral.sh/uv/install.sh"),
];
pub(crate) static POETRY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("poetry"),
    InstallationMethod::Pip("poetry"),
];
pub(crate) static CPAN_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("perl"),
    InstallationMethod::Apt("perl"),
    InstallationMethod::Dnf("perl"),
    InstallationMethod::Pacman("perl"),
    InstallationMethod::Chocolatey("strawberryperl"),
];
pub(crate) static CPANM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("cpanminus"),
    InstallationMethod::Apt("cpanminus"),
    InstallationMethod::Dnf("cpanminus"),
    InstallationMethod::Pacman("perl-app-cpanminus"),
];

/// Metadata lookup table for language package managers.
pub(crate) static LANG_PKG_MGR_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "npm",
        display_name: "npm",
        description: "Node.js package manager",
        website: "https://www.npmjs.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/npm/cli"),
        installation_methods: NPM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pnpm",
        display_name: "pnpm",
        description: "Fast, disk-efficient package manager",
        website: "https://pnpm.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/pnpm/pnpm"),
        installation_methods: PNPM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "yarn",
        display_name: "Yarn",
        description: "Alternative Node.js package manager",
        website: "https://yarnpkg.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/yarnpkg/berry"),
        installation_methods: YARN_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "bun",
        display_name: "Bun",
        description: "All-in-one JS runtime with package manager",
        website: "https://bun.sh/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/oven-sh/bun"),
        installation_methods: BUN_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "cargo",
        display_name: "Cargo",
        description: "Rust package manager and build tool",
        website: "https://doc.rust-lang.org/cargo/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/rust-lang/cargo"),
        installation_methods: CARGO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "go",
        display_name: "Go Modules",
        description: "Built-in Go dependency system",
        website: "https://go.dev/ref/mod",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::AfterPrefix,
        version_regex: None,
        version_prefix: Some("go version "),
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/golang/go"),
        installation_methods: GO_MODULES_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "composer",
        display_name: "Composer",
        description: "PHP dependency manager",
        website: "https://getcomposer.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::AfterPrefix,
        version_regex: None,
        version_prefix: Some("Composer version "),
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/composer/composer"),
        installation_methods: COMPOSER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "swift",
        display_name: "Swift Package Manager",
        description: "Swift dependency manager",
        website: "https://www.swift.org/package-manager/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/apple/swift-package-manager"),
        installation_methods: SWIFTPM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "luarocks",
        display_name: "LuaRocks",
        description: "Package manager for Lua modules",
        website: "https://luarocks.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/luarocks/luarocks"),
        installation_methods: LUAROCKS_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "vcpkg",
        display_name: "vcpkg",
        description: "C/C++ dependency manager by Microsoft",
        website: "https://vcpkg.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/microsoft/vcpkg"),
        installation_methods: VCPKG_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "conan",
        display_name: "Conan",
        description: "Decentralized C/C++ package manager",
        website: "https://conan.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/conan-io/conan"),
        installation_methods: CONAN_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nuget",
        display_name: "NuGet",
        description: ".NET package manager",
        website: "https://www.nuget.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/NuGet/NuGet.Client"),
        installation_methods: NUGET_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mix",
        display_name: "Hex",
        description: "Package manager for BEAM ecosystem",
        website: "https://hex.pm/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/hexpm/hex"),
        installation_methods: HEX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pip",
        display_name: "pip",
        description: "Python package installer",
        website: "https://pip.pypa.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/pypa/pip"),
        installation_methods: PIP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "uv",
        display_name: "uv",
        description: "Fast Python package manager",
        website: "https://astral.sh/uv",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/astral-sh/uv"),
        installation_methods: UV_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "poetry",
        display_name: "Poetry",
        description: "Python dependency manager with lockfiles",
        website: "https://python-poetry.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/python-poetry/poetry"),
        installation_methods: POETRY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "cpan",
        display_name: "CPAN",
        description: "Perl module archive",
        website: "https://www.cpan.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/Perl/perl5"),
        installation_methods: CPAN_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "cpanm",
        display_name: "cpanminus",
        description: "Lightweight CPAN client",
        website: "https://metacpan.org/pod/App::cpanminus",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/miyagawa/cpanminus"),
        installation_methods: CPANM_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for LanguagePackageManager {
    fn info(&self) -> &'static ProgramInfo {
        &LANG_PKG_MGR_INFO[*self as usize]
    }
}

impl CategoryEnum for LanguagePackageManager {
    fn category_name() -> &'static str {
        "language_package_managers"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            LanguagePackageManager::Npm => "npm",
            LanguagePackageManager::Pnpm => "pnpm",
            LanguagePackageManager::Yarn => "yarn",
            LanguagePackageManager::Bun => "bun",
            LanguagePackageManager::Cargo => "cargo",
            LanguagePackageManager::GoModules => "go_modules",
            LanguagePackageManager::Composer => "composer",
            LanguagePackageManager::SwiftPm => "swift_pm",
            LanguagePackageManager::Luarocks => "luarocks",
            LanguagePackageManager::Vcpkg => "vcpkg",
            LanguagePackageManager::Conan => "conan",
            LanguagePackageManager::Nuget => "nuget",
            LanguagePackageManager::Hex => "hex",
            LanguagePackageManager::Pip => "pip",
            LanguagePackageManager::Uv => "uv",
            LanguagePackageManager::Poetry => "poetry",
            LanguagePackageManager::Cpan => "cpan",
            LanguagePackageManager::Cpanm => "cpanm",
        }
    }
}

/// Metadata lookup table for OS package managers.
pub(crate) static OS_PKG_MGR_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "apt",
        display_name: "APT",
        description: "Debian/Ubuntu package manager",
        website: "https://tracker.debian.org/pkg/apt",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://salsa.debian.org/apt-team/apt"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nala",
        display_name: "Nala",
        description: "Modern apt frontend with parallel downloads",
        website: "https://github.com/volitank/nala",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://github.com/volitank/nala"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "brew",
        display_name: "Homebrew",
        description: "macOS/Linux community package manager",
        website: "https://brew.sh/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/Homebrew/brew"),
        installation_methods: BREW_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "dnf",
        display_name: "DNF",
        description: "Fedora/RHEL package manager",
        website: "https://github.com/rpm-software-management/dnf",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://github.com/rpm-software-management/dnf"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pacman",
        display_name: "Pacman",
        description: "Arch Linux package manager",
        website: "https://archlinux.org/pacman/",
        version_flag: VersionFlag::ShortUpper,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://gitlab.archlinux.org/pacman/pacman"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "winget",
        display_name: "winget",
        description: "Windows Package Manager",
        website: "https://github.com/microsoft/winget-cli",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: Some("https://github.com/microsoft/winget-cli"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "choco",
        display_name: "Chocolatey",
        description: "Windows community package manager",
        website: "https://chocolatey.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: Some("https://github.com/chocolatey/choco"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "scoop",
        display_name: "Scoop",
        description: "Windows command-line installer",
        website: "https://scoop.sh/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: Some("https://github.com/ScoopInstaller/Scoop"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nix",
        display_name: "Nix",
        description: "Nix package manager",
        website: "https://nixos.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/NixOS/nix"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for OsPackageManager {
    fn info(&self) -> &'static ProgramInfo {
        &OS_PKG_MGR_INFO[*self as usize]
    }
}

impl CategoryEnum for OsPackageManager {
    fn category_name() -> &'static str {
        "os_package_managers"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            OsPackageManager::Apt => "apt",
            OsPackageManager::Nala => "nala",
            OsPackageManager::Brew => "brew",
            OsPackageManager::Dnf => "dnf",
            OsPackageManager::Pacman => "pacman",
            OsPackageManager::Winget => "winget",
            OsPackageManager::Chocolatey => "chocolatey",
            OsPackageManager::Scoop => "scoop",
            OsPackageManager::Nix => "nix",
        }
    }
}

// TTS client installation methods
pub(crate) static ESPEAK_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("espeak"),
    InstallationMethod::Apt("espeak"),
    InstallationMethod::Dnf("espeak"),
    InstallationMethod::Pacman("espeak"),
];
pub(crate) static ESPEAK_NG_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("espeak-ng"),
    InstallationMethod::Apt("espeak-ng"),
    InstallationMethod::Dnf("espeak-ng"),
    InstallationMethod::Pacman("espeak-ng"),
];
pub(crate) static FESTIVAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("festival"),
    InstallationMethod::Apt("festival"),
    InstallationMethod::Dnf("festival"),
    InstallationMethod::Pacman("festival"),
];
pub(crate) static MIMIC_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("mimic")];
pub(crate) static MIMIC3_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("mimic3-tts")];
pub(crate) static PIPER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("piper"),
    InstallationMethod::Pip("piper-tts"),
];
pub(crate) static ECHOGARDEN_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Npm("echogarden")];
pub(crate) static BALCON_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Chocolatey("balcon")];
pub(crate) static WINDOWS_SAPI_INSTALL: &[InstallationMethod] = &[];
pub(crate) static GTTS_CLI_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("gTTS")];
pub(crate) static COQUI_TTS_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("TTS")];
pub(crate) static SHERPA_ONNX_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Pip("sherpa-onnx")];
pub(crate) static KOKORO_TTS_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Pip("kokoro-tts")];
pub(crate) static PICO2WAVE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("libttspico-utils"),
    InstallationMethod::Pacman("svox-pico"),
];

/// Metadata lookup table for TTS clients.
pub(crate) static TTS_CLIENT_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "say",
        display_name: "say",
        description: "macOS built-in speech synthesis",
        website: "https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/SpeechSynthesisProgrammingGuide/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: None,
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "espeak",
        display_name: "eSpeak",
        description: "Open source speech synthesizer",
        website: "http://espeak.sourceforge.net/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/espeak-ng/espeak-ng"),
        installation_methods: ESPEAK_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "espeak-ng",
        display_name: "eSpeak NG",
        description: "Multi-lingual speech synthesizer",
        website: "https://github.com/espeak-ng/espeak-ng",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/espeak-ng/espeak-ng"),
        installation_methods: ESPEAK_NG_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "festival",
        display_name: "Festival",
        description: "General multi-lingual speech synthesis",
        website: "http://www.cstr.ed.ac.uk/projects/festival/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/festvox/festival"),
        installation_methods: FESTIVAL_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mimic",
        display_name: "Mimic",
        description: "Mycroft's TTS engine based on Flite",
        website: "https://github.com/MycroftAI/mimic",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/MycroftAI/mimic"),
        installation_methods: MIMIC_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mimic3",
        display_name: "Mimic 3",
        description: "Mycroft's neural TTS engine",
        website: "https://github.com/MycroftAI/mycroft-mimic3-tts",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/MycroftAI/mycroft-mimic3-tts"),
        installation_methods: MIMIC3_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "piper",
        display_name: "Piper",
        description: "Fast local neural TTS using ONNX",
        website: "https://github.com/rhasspy/piper",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/rhasspy/piper"),
        installation_methods: PIPER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "echogarden",
        display_name: "Echogarden",
        description: "Speech processing engine",
        website: "https://echogarden.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/echogarden-project/echogarden"),
        installation_methods: ECHOGARDEN_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "balcon",
        display_name: "Balcon",
        description: "Command line TTS utility for Windows",
        website: "http://www.cross-plus-a.com/balcon.htm",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: None,
        installation_methods: BALCON_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "sapi",
        display_name: "Windows SAPI",
        description: "Windows Speech API",
        website: "https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms723627(v=vs.85)",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: None,
        installation_methods: WINDOWS_SAPI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "gtts-cli",
        display_name: "gTTS",
        description: "Google Text-to-Speech CLI tool",
        website: "https://github.com/pndurette/gTTS",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/pndurette/gTTS"),
        installation_methods: GTTS_CLI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "tts",
        display_name: "Coqui TTS",
        description: "Deep learning for Text-to-Speech",
        website: "https://github.com/coqui-ai/TTS",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/coqui-ai/TTS"),
        installation_methods: COQUI_TTS_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "sherpa-onnx-offline-tts",
        display_name: "Sherpa-ONNX",
        description: "Streaming/non-streaming TTS using ONNX",
        website: "https://k2-fsa.github.io/sherpa/onnx/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &["sherpa-onnx-tts"],
        os_availability: ALL_OS,
        repo: Some("https://github.com/k2-fsa/sherpa-onnx"),
        installation_methods: SHERPA_ONNX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "kokoro-tts",
        display_name: "Kokoro TTS",
        description: "High-quality neural TTS using Kokoro-82M model",
        website: "https://github.com/nazdridoy/kokoro-tts",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/nazdridoy/kokoro-tts"),
        installation_methods: KOKORO_TTS_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pico2wave",
        display_name: "SVOX Pico",
        description: "Lightweight TTS for embedded systems",
        website: "https://github.com/naggety/picmotts",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/naggety/picmotts"),
        installation_methods: PICO2WAVE_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for TtsClient {
    fn info(&self) -> &'static ProgramInfo {
        &TTS_CLIENT_INFO[*self as usize]
    }
}

impl CategoryEnum for TtsClient {
    fn category_name() -> &'static str {
        "tts_clients"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            TtsClient::Say => "say",
            TtsClient::Espeak => "espeak",
            TtsClient::EspeakNg => "espeak_ng",
            TtsClient::Festival => "festival",
            TtsClient::Mimic => "mimic",
            TtsClient::Mimic3 => "mimic3",
            TtsClient::Piper => "piper",
            TtsClient::Echogarden => "echogarden",
            TtsClient::Balcon => "balcon",
            TtsClient::WindowsSapi => "windows_sapi",
            TtsClient::GttsCli => "gtts_cli",
            TtsClient::CoquiTts => "coqui_tts",
            TtsClient::SherpaOnnx => "sherpa_onnx",
            TtsClient::KokoroTts => "kokoro_tts",
            TtsClient::Pico2Wave => "pico2_wave",
        }
    }

    fn platform_override(
        &self,
    ) -> Option<(
        std::path::PathBuf,
        crate::programs::contract::ExecutableSource,
    )> {
        match self {
            TtsClient::WindowsSapi => {
                if cfg!(target_os = "windows") {
                    Some((
                        std::path::PathBuf::from("sapi"),
                        crate::programs::contract::ExecutableSource::Path,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

// Terminal app installation methods
pub(crate) static ALACRITTY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("alacritty"),
    InstallationMethod::Cargo("alacritty"),
    InstallationMethod::Pacman("alacritty"),
    InstallationMethod::Scoop("alacritty"),
];
pub(crate) static KITTY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("kitty"),
    InstallationMethod::Apt("kitty"),
    InstallationMethod::Dnf("kitty"),
    InstallationMethod::Pacman("kitty"),
];
pub(crate) static ITERM2_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("iterm2")];
pub(crate) static WEZTERM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("wezterm"),
    InstallationMethod::Chocolatey("wezterm"),
    InstallationMethod::Scoop("wezterm"),
];
pub(crate) static GHOSTTY_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("ghostty")];
pub(crate) static WARP_INSTALL: &[InstallationMethod] = &[InstallationMethod::Brew("warp")];
pub(crate) static RIO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("rio"),
    InstallationMethod::Scoop("rio"),
];
pub(crate) static TABBY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("tabby"),
    InstallationMethod::Scoop("tabby"),
];
pub(crate) static FOOT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("foot"),
    InstallationMethod::Dnf("foot"),
    InstallationMethod::Pacman("foot"),
];
pub(crate) static GNOME_TERMINAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("gnome-terminal"),
    InstallationMethod::Dnf("gnome-terminal"),
    InstallationMethod::Pacman("gnome-terminal"),
];
pub(crate) static KONSOLE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("konsole"),
    InstallationMethod::Dnf("konsole"),
    InstallationMethod::Pacman("konsole"),
];
pub(crate) static XFCE_TERMINAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("xfce4-terminal"),
    InstallationMethod::Dnf("xfce4-terminal"),
    InstallationMethod::Pacman("xfce4-terminal"),
];
pub(crate) static TERMINOLOGY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("terminology"),
    InstallationMethod::Dnf("terminology"),
    InstallationMethod::Pacman("terminology"),
];
pub(crate) static ST_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("st"),
    InstallationMethod::Pacman("st"),
];
pub(crate) static XTERM_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("xterm"),
    InstallationMethod::Dnf("xterm"),
    InstallationMethod::Pacman("xterm"),
];
pub(crate) static HYPER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("hyper"),
    InstallationMethod::Scoop("hyper"),
];
pub(crate) static WINDOWS_TERMINAL_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Winget("Microsoft.WindowsTerminal"),
    InstallationMethod::Scoop("windows-terminal"),
];

/// Metadata lookup table for terminal apps.
pub(crate) static TERMINAL_APP_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "alacritty",
        display_name: "Alacritty",
        description: "Fast, GPU-accelerated terminal emulator",
        website: "https://alacritty.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/alacritty/alacritty"),
        installation_methods: ALACRITTY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "kitty",
        display_name: "kitty",
        description: "Fast, feature-rich GPU-based terminal",
        website: "https://sw.kovidgoyal.net/kitty/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/kovidgoyal/kitty"),
        installation_methods: KITTY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "iterm2",
        display_name: "iTerm2",
        description: "Terminal emulator for macOS",
        website: "https://iterm2.com/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: Some("https://github.com/gnachman/iTerm2"),
        installation_methods: ITERM2_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "wezterm",
        display_name: "WezTerm",
        description: "GPU-accelerated terminal emulator and multiplexer",
        website: "https://wezfurlong.org/wezterm/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/wez/wezterm"),
        installation_methods: WEZTERM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "ghostty",
        display_name: "Ghostty",
        description: "Fast, feature-rich GPU terminal written in Zig",
        website: "https://ghostty.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/ghostty-org/ghostty"),
        installation_methods: GHOSTTY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "warp-terminal",
        display_name: "Warp",
        description: "Modern, Rust-based terminal with AI",
        website: "https://www.warp.dev/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: Some("https://www.warp.dev/"),
        installation_methods: WARP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "rio",
        display_name: "Rio",
        description: "Hardware-accelerated GPU terminal emulator",
        website: "https://github.com/raphamorim/rio",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/raphamorim/rio"),
        installation_methods: RIO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "tabby",
        display_name: "Tabby",
        description: "Terminal for a more modern age",
        website: "https://tabby.sh/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/Eugeny/tabby"),
        installation_methods: TABBY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "foot",
        display_name: "foot",
        description: "Fast, lightweight Wayland terminal emulator",
        website: "https://codeberg.org/dnkl/foot",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://codeberg.org/dnkl/foot"),
        installation_methods: FOOT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "gnome-terminal",
        display_name: "GNOME Terminal",
        description: "Default terminal for GNOME desktop",
        website: "https://help.gnome.org/users/gnome-terminal/stable/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://gitlab.gnome.org/GNOME/gnome-terminal"),
        installation_methods: GNOME_TERMINAL_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "konsole",
        display_name: "Konsole",
        description: "Terminal emulator by KDE",
        website: "https://konsole.kde.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://invent.kde.org/utilities/konsole"),
        installation_methods: KONSOLE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "xfce4-terminal",
        display_name: "Xfce Terminal",
        description: "Terminal emulator for Xfce",
        website: "https://docs.xfce.org/apps/xfce4-terminal/start",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://gitlab.xfce.org/apps/xfce4-terminal"),
        installation_methods: XFCE_TERMINAL_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "terminology",
        display_name: "Terminology",
        description: "Terminal based on Enlightenment libraries",
        website: "https://www.enlightenment.org/about-terminology",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://github.com/Enlightenment/terminology"),
        installation_methods: TERMINOLOGY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "st",
        display_name: "st",
        description: "Simple terminal for X which sucks less",
        website: "https://st.suckless.org/",
        version_flag: VersionFlag::Short,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://git.suckless.org/st"),
        installation_methods: ST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "xterm",
        display_name: "xterm",
        description: "Standard terminal for X Window System",
        website: "https://invisible-island.net/xterm/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://invisible-island.net/xterm/"),
        installation_methods: XTERM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "hyper",
        display_name: "Hyper",
        description: "Terminal built on web technologies",
        website: "https://hyper.is/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/vercel/hyper"),
        installation_methods: HYPER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "wt",
        display_name: "Windows Terminal",
        description: "Modern terminal for Windows",
        website: "https://github.com/microsoft/terminal",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: Some("https://github.com/microsoft/terminal"),
        installation_methods: WINDOWS_TERMINAL_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for TerminalApp {
    fn info(&self) -> &'static ProgramInfo {
        &TERMINAL_APP_INFO[*self as usize]
    }
}

impl CategoryEnum for TerminalApp {
    fn category_name() -> &'static str {
        "terminal_apps"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            TerminalApp::Alacritty => "alacritty",
            TerminalApp::Kitty => "kitty",
            TerminalApp::ITerm2 => "i_term2",
            TerminalApp::WezTerm => "wez_term",
            TerminalApp::Ghostty => "ghostty",
            TerminalApp::Warp => "warp",
            TerminalApp::Rio => "rio",
            TerminalApp::Tabby => "tabby",
            TerminalApp::Foot => "foot",
            TerminalApp::GnomeTerminal => "gnome_terminal",
            TerminalApp::Konsole => "konsole",
            TerminalApp::XfceTerminal => "xfce_terminal",
            TerminalApp::Terminology => "terminology",
            TerminalApp::St => "st",
            TerminalApp::Xterm => "xterm",
            TerminalApp::Hyper => "hyper",
            TerminalApp::WindowsTerminal => "windows_terminal",
        }
    }
}

// Headless audio player installation methods
pub(crate) static MPV_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mpv"),
    InstallationMethod::Apt("mpv"),
    InstallationMethod::Dnf("mpv"),
    InstallationMethod::Pacman("mpv"),
    InstallationMethod::Chocolatey("mpv"),
    InstallationMethod::Scoop("mpv"),
];
pub(crate) static FFPLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("ffmpeg"),
    InstallationMethod::Apt("ffmpeg"),
    InstallationMethod::Dnf("ffmpeg"),
    InstallationMethod::Pacman("ffmpeg"),
    InstallationMethod::Chocolatey("ffmpeg"),
    InstallationMethod::Scoop("ffmpeg"),
];
pub(crate) static VLC_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vlc"),
    InstallationMethod::Apt("vlc"),
    InstallationMethod::Dnf("vlc"),
    InstallationMethod::Pacman("vlc"),
    InstallationMethod::Chocolatey("vlc"),
    InstallationMethod::Scoop("vlc"),
];
pub(crate) static MPLAYER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mplayer"),
    InstallationMethod::Apt("mplayer"),
    InstallationMethod::Dnf("mplayer"),
    InstallationMethod::Pacman("mplayer"),
];
pub(crate) static GSTREAMER_GST_PLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("gstreamer"),
    InstallationMethod::Apt("gstreamer1.0-tools"),
    InstallationMethod::Dnf("gstreamer1-plugins-base"),
    InstallationMethod::Pacman("gstreamer"),
];
pub(crate) static SOX_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("sox"),
    InstallationMethod::Apt("sox"),
    InstallationMethod::Dnf("sox"),
    InstallationMethod::Pacman("sox"),
    InstallationMethod::Chocolatey("sox"),
];
pub(crate) static MPG123_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("mpg123"),
    InstallationMethod::Apt("mpg123"),
    InstallationMethod::Dnf("mpg123"),
    InstallationMethod::Pacman("mpg123"),
];
pub(crate) static OGG123_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("vorbis-tools"),
    InstallationMethod::Apt("vorbis-tools"),
    InstallationMethod::Dnf("vorbis-tools"),
    InstallationMethod::Pacman("vorbis-tools"),
];
pub(crate) static ALSA_APLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("alsa-utils"),
    InstallationMethod::Dnf("alsa-utils"),
    InstallationMethod::Pacman("alsa-utils"),
];
pub(crate) static MACOS_AFPLAY_INSTALL: &[InstallationMethod] = &[];
pub(crate) static PULSEAUDIO_PAPLAY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("pulseaudio-utils"),
    InstallationMethod::Dnf("pulseaudio-utils"),
    InstallationMethod::Pacman("pulseaudio"),
];
pub(crate) static PULSEAUDIO_PACAT_INSTALL: &[InstallationMethod] = PULSEAUDIO_PAPLAY_INSTALL;
pub(crate) static PIPEWIRE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("pipewire"),
    InstallationMethod::Dnf("pipewire"),
    InstallationMethod::Pacman("pipewire"),
];

/// Metadata lookup table for headless audio players.
pub(crate) static HEADLESS_AUDIO_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "mpv",
        display_name: "mpv",
        description: "CLI media player for audio-only playback",
        website: "https://mpv.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/mpv-player/mpv"),
        installation_methods: MPV_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "ffplay",
        display_name: "FFplay",
        description: "Minimal CLI player shipped with FFmpeg",
        website: "https://www.ffmpeg.org/ffplay.html",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/FFmpeg/FFmpeg"),
        installation_methods: FFPLAY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "cvlc",
        display_name: "VLC",
        description: "Headless VLC playback via cvlc",
        website: "https://wiki.videolan.org/VLC_command-line_help/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/videolan/vlc"),
        installation_methods: VLC_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mplayer",
        display_name: "MPlayer",
        description: "Classic CLI-oriented media player",
        website: "https://www.mplayerhq.hu/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/mplayerhq/mplayer"),
        installation_methods: MPLAYER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "gst-play-1.0",
        display_name: "GStreamer gst-play",
        description: "CLI front-end to GStreamer pipelines",
        website: "https://gstreamer.freedesktop.org/documentation/tools/gst-play-1.0.html",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://gitlab.freedesktop.org/gstreamer/gstreamer"),
        installation_methods: GSTREAMER_GST_PLAY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "play",
        display_name: "SoX play",
        description: "Swiss-army knife for audio playback",
        website: "https://linux.die.net/man/1/sox",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://sourceforge.net/projects/sox/"),
        installation_methods: SOX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mpg123",
        display_name: "mpg123",
        description: "Lightweight console MP3 player",
        website: "https://www.mpg123.de/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/madebr/mpg123"),
        installation_methods: MPG123_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "ogg123",
        display_name: "ogg123",
        description: "CLI player for Ogg/Vorbis files",
        website: "https://github.com/xiph/vorbis-tools",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: UNIX_ONLY,
        repo: Some("https://github.com/xiph/vorbis-tools"),
        installation_methods: OGG123_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "aplay",
        display_name: "aplay",
        description: "ALSA low-level playback utility",
        website: "https://linux.die.net/man/1/aplay",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: None,
        installation_methods: ALSA_APLAY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "afplay",
        display_name: "afplay",
        description: "macOS native audio file player",
        website: "https://ss64.com/osx/afplay.html",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: None,
        installation_methods: MACOS_AFPLAY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "paplay",
        display_name: "paplay",
        description: "Simple PulseAudio playback tool",
        website: "https://manpages.ubuntu.com/manpages/trusty/man1/paplay.1.html",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: None,
        installation_methods: PULSEAUDIO_PAPLAY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pacat",
        display_name: "pacat",
        description: "PulseAudio raw audio streaming",
        website: "https://www.freedesktop.org/wiki/Software/PulseAudio/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: None,
        installation_methods: PULSEAUDIO_PACAT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pw-play",
        display_name: "PipeWire pw-play",
        description: "PipeWire CLI playback tool",
        website: "https://docs.pipewire.org/page_man_pw-cat_1.html",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://gitlab.freedesktop.org/pipewire/pipewire"),
        installation_methods: PIPEWIRE_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for HeadlessAudio {
    fn info(&self) -> &'static ProgramInfo {
        &HEADLESS_AUDIO_INFO[*self as usize]
    }
}

impl CategoryEnum for HeadlessAudio {
    fn category_name() -> &'static str {
        "headless_audio"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            HeadlessAudio::Mpv => "mpv",
            HeadlessAudio::Ffplay => "ffplay",
            HeadlessAudio::Vlc => "vlc",
            HeadlessAudio::MPlayer => "m_player",
            HeadlessAudio::GstreamerGstPlay => "gstreamer_gst_play",
            HeadlessAudio::Sox => "sox",
            HeadlessAudio::Mpg123 => "mpg123",
            HeadlessAudio::Ogg123 => "ogg123",
            HeadlessAudio::AlsaAplay => "alsa_aplay",
            HeadlessAudio::MacOsAfplay => "mac_os_afplay",
            HeadlessAudio::PulseaudioPaplay => "pulseaudio_paplay",
            HeadlessAudio::PulseaudioPacat => "pulseaudio_pacat",
            HeadlessAudio::Pipewire => "pipewire",
        }
    }
}

// AI CLI installation methods
pub(crate) static CLAUDE_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Npm("@anthropic-ai/claude-code")];
pub(crate) static OPENCODE_INSTALL: &[InstallationMethod] = &[InstallationMethod::GoModules(
    "github.com/opencode-ai/opencode@latest",
)];
pub(crate) static ROO_INSTALL: &[InstallationMethod] = &[InstallationMethod::RemoteBash(
    "https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/install.sh",
)];
pub(crate) static GEMINI_CLI_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Npm("@google/gemini-cli")];
pub(crate) static AIDER_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Pip("aider-chat"),
    InstallationMethod::Uv("aider-chat"),
    InstallationMethod::Brew("aider"),
];
pub(crate) static CODEX_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Npm("@openai/codex")];
pub(crate) static GOOSE_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Brew("goose"),
    InstallationMethod::Pip("goose-ai"),
];
pub(crate) static KIMI_CLI_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Uv("kimi-cli"),
    InstallationMethod::RemoteBash("https://code.kimi.com/install.sh"),
];
pub(crate) static QWEN_CLI_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Npm("@qwen-code/qwen-code"),
    InstallationMethod::Brew("qwen-code"),
];
pub(crate) static KILO_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Npm("@kilocode/cli"),
    InstallationMethod::Brew("Kilo-Org/tap/kilo"),
];
pub(crate) static PI_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Npm("@earendil-works/pi-coding-agent")];
pub(crate) static ANTIGRAVITY_INSTALL: &[InstallationMethod] = &[InstallationMethod::RemoteBash(
    "https://antigravity.google/cli/install.sh",
)];

/// Metadata lookup table for AI CLI tools.
pub(crate) static AI_CLI_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "claude",
        display_name: "Claude Code",
        description: "Anthropic's agentic coding tool",
        website: "https://docs.anthropic.com/en/docs/claude-code",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/anthropics/claude-code"),
        installation_methods: CLAUDE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "opencode",
        display_name: "OpenCode",
        description: "AI-powered coding assistant CLI",
        website: "https://github.com/opencode-ai/opencode",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/opencode-ai/opencode"),
        installation_methods: OPENCODE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "roo",
        display_name: "Roo Code",
        description: "AI pair programming in your terminal",
        website: "https://github.com/RooVetGit/Roo-Code",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/RooVetGit/Roo-Code"),
        installation_methods: ROO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "gemini",
        display_name: "Gemini CLI",
        description: "Google's Gemini AI in the terminal",
        website: "https://github.com/google-gemini/gemini-cli",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/google-gemini/gemini-cli"),
        installation_methods: GEMINI_CLI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "aider",
        display_name: "Aider",
        description: "AI pair programming in your terminal",
        website: "https://aider.chat/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/paul-gauthier/aider"),
        installation_methods: AIDER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "codex",
        display_name: "Codex CLI",
        description: "OpenAI lightweight coding agent",
        website: "https://github.com/openai/codex",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/openai/codex"),
        installation_methods: CODEX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "goose",
        display_name: "Goose",
        description: "Block's AI developer agent",
        website: "https://github.com/block/goose",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/block/goose"),
        installation_methods: GOOSE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "kimi",
        display_name: "Kimi Code CLI",
        description: "AI agent that runs in the terminal",
        website: "https://moonshotai.github.io/kimi-cli/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &["kimi-cli"],
        os_availability: ALL_OS,
        repo: Some("https://github.com/MoonshotAI/kimi-cli"),
        installation_methods: KIMI_CLI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "qwen",
        display_name: "Qwen Code CLI",
        description: "Qwen's AI coding agent",
        website: "https://qwenlm.github.io/qwen-code-docs/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/QwenLM/qwen-code"),
        installation_methods: QWEN_CLI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "kilo",
        display_name: "Kilo Code",
        description: "Open-source agentic coding CLI (OpenCode fork)",
        website: "https://kilo.ai/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &["kilocode"],
        os_availability: ALL_OS,
        repo: Some("https://github.com/Kilo-Org/kilocode"),
        installation_methods: KILO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pi",
        display_name: "Pi",
        description: "Multi-provider agentic coding CLI (earendil-works)",
        website: "https://pi.dev/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/earendil-works/pi"),
        installation_methods: PI_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "agy",
        display_name: "Antigravity",
        description: "Google's Antigravity headless coding CLI",
        website: "https://antigravity.google/product/antigravity-cli",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/google-antigravity/antigravity-cli"),
        installation_methods: ANTIGRAVITY_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for AiCli {
    fn info(&self) -> &'static ProgramInfo {
        &AI_CLI_INFO[*self as usize]
    }
}

impl CategoryEnum for AiCli {
    fn category_name() -> &'static str {
        "ai_clients"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            AiCli::Claude => "claude",
            AiCli::Opencode => "opencode",
            AiCli::Roo => "roo",
            AiCli::GeminiCli => "gemini_cli",
            AiCli::Aider => "aider",
            AiCli::Codex => "codex",
            AiCli::Goose => "goose",
            AiCli::KimiCli => "kimi_cli",
            AiCli::QwenCli => "qwen_cli",
            AiCli::Kilo => "kilo",
            AiCli::Pi => "pi",
            AiCli::Antigravity => "antigravity",
        }
    }
}

// Notification helper installation methods
pub(crate) static TERMINAL_NOTIFIER_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Brew("terminal-notifier")];
pub(crate) static ALERTER_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Brew("vjeantet/tap/alerter")];
pub(crate) static SNORETOAST_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Chocolatey("snoretoast"),
    InstallationMethod::Scoop("snoretoast"),
];
pub(crate) static BURNTTOAST_INSTALL: &[InstallationMethod] = &[InstallationMethod::RemoteBash(
    "https://raw.githubusercontent.com/Windos/BurntToast/main/install.ps1",
)];
pub(crate) static DUNSTIFY_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("dunst"),
    InstallationMethod::Dnf("dunst"),
    InstallationMethod::Pacman("dunst"),
];
pub(crate) static NOTIFY_SEND_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Apt("libnotify-bin"),
    InstallationMethod::Dnf("libnotify"),
    InstallationMethod::Pacman("libnotify"),
];

/// Metadata lookup table for notification helpers.
pub(crate) static NOTIFICATION_HELPER_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "terminal-notifier",
        display_name: "terminal-notifier",
        description: "macOS notification helper with rich controls",
        website: "https://github.com/julienXX/terminal-notifier",
        version_flag: VersionFlag::Custom("-help"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: Some("https://github.com/julienXX/terminal-notifier"),
        installation_methods: TERMINAL_NOTIFIER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "alerter",
        display_name: "alerter",
        description: "macOS notification helper with actions and replies",
        website: "https://github.com/vjeantet/alerter",
        version_flag: VersionFlag::Custom("-help"),
        parse_strategy: VersionParseStrategy::Regex,
        version_regex: Some(r"(?i)version\s+([\d.]+)"),
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: MACOS_ONLY,
        repo: Some("https://github.com/vjeantet/alerter"),
        installation_methods: ALERTER_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "snoretoast",
        display_name: "SnoreToast",
        description: "Windows toast notification helper",
        website: "https://github.com/KDE/snoretoast",
        version_flag: VersionFlag::Short,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: Some("https://github.com/KDE/snoretoast"),
        installation_methods: SNORETOAST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "BurntToast",
        display_name: "BurntToast",
        description: "PowerShell module for Windows toast notifications",
        website: "https://github.com/Windos/BurntToast",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: WINDOWS_ONLY,
        repo: Some("https://github.com/Windos/BurntToast"),
        installation_methods: BURNTTOAST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "dunstify",
        display_name: "dunstify",
        description: "Dunst notification helper for Linux",
        website: "https://dunst-project.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://github.com/dunst-project/dunst"),
        installation_methods: DUNSTIFY_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "notify-send",
        display_name: "notify-send",
        description: "libnotify CLI for Linux desktop notifications",
        website: "https://gitlab.gnome.org/GNOME/libnotify",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: LINUX_ONLY,
        repo: Some("https://gitlab.gnome.org/GNOME/libnotify"),
        installation_methods: NOTIFY_SEND_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for NotificationHelper {
    fn info(&self) -> &'static ProgramInfo {
        &NOTIFICATION_HELPER_INFO[*self as usize]
    }
}

impl CategoryEnum for NotificationHelper {
    fn category_name() -> &'static str {
        "notification_helpers"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            NotificationHelper::TerminalNotifier => "terminal_notifier",
            NotificationHelper::Alerter => "alerter",
            NotificationHelper::SnoreToast => "snore_toast",
            NotificationHelper::BurntToast => "burnt_toast",
            NotificationHelper::Dunstify => "dunstify",
            NotificationHelper::NotifySend => "notify_send",
        }
    }

    fn platform_override(
        &self,
    ) -> Option<(
        std::path::PathBuf,
        crate::programs::contract::ExecutableSource,
    )> {
        match self {
            NotificationHelper::BurntToast => {
                if cfg!(target_os = "windows") && is_burnttoast_available() {
                    Some((
                        std::path::PathBuf::from("BurntToast"),
                        crate::programs::contract::ExecutableSource::Path,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Cached per-process result of the BurntToast PowerShell module probe.
static BURNTTOAST_AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Probe whether the BurntToast PowerShell module is installed.
fn is_burnttoast_available() -> bool {
    *BURNTTOAST_AVAILABLE.get_or_init(|| {
        burnttoast_available_with(
            "pwsh",
            &[
                "-NoProfile",
                "-Command",
                "if (Get-Module -ListAvailable BurntToast) { 'yes' } else { 'no' }",
            ],
            crate::process::timeouts::WINDOWS_BURNTTOAST,
        )
    })
}

fn burnttoast_available_with<S, A>(program: S, args: &[A], timeout: std::time::Duration) -> bool
where
    S: AsRef<std::ffi::OsStr>,
    A: AsRef<std::ffi::OsStr>,
{
    crate::process::run_for_stdout(program, args, timeout)
        .is_some_and(|stdout| burnttoast_output_is_available(&stdout))
}

fn burnttoast_output_is_available(stdout: &str) -> bool {
    stdout.trim() == "yes"
}

#[cfg(test)]
mod burnttoast_tests {
    use super::*;

    const SLEEPING_CHILD: &str =
        "programs::enums::metadata::burnttoast_tests::child_sleeps";

    fn test_child_args(name: &str) -> Vec<std::ffi::OsString> {
        [name, "--exact", "--ignored", "--nocapture"]
            .into_iter()
            .map(Into::into)
            .collect()
    }

    #[test]
    #[ignore = "subprocess fixture invoked by the BurntToast probe tests"]
    fn child_sleeps() {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    #[test]
    fn burnttoast_probe_uses_the_shared_bounded_runner() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let start = std::time::Instant::now();
        assert!(!burnttoast_available_with(
            executable,
            &test_child_args(SLEEPING_CHILD),
            std::time::Duration::from_millis(100),
        ));
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn burnttoast_probe_accepts_only_the_available_marker() {
        assert!(burnttoast_output_is_available("yes\r\n"));
        assert!(!burnttoast_output_is_available("no\r\n"));
        assert!(!burnttoast_output_is_available("yes\nwarning"));
    }

    #[test]
    fn burnttoast_timeout_is_named_policy() {
        assert_eq!(
            crate::process::timeouts::WINDOWS_BURNTTOAST,
            std::time::Duration::from_secs(3),
        );
    }
}

// Test runner installation methods
pub(crate) static NEXTEST_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Cargo("cargo-nextest")];
pub(crate) static GOTESTSUM_INSTALL: &[InstallationMethod] = &[InstallationMethod::GoModules(
    "gotest.tools/gotestsum@latest",
)];
pub(crate) static GINKGO_INSTALL: &[InstallationMethod] = &[InstallationMethod::GoModules(
    "github.com/onsi/ginkgo/v2@latest",
)];
pub(crate) static VITEST_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("vitest")];
pub(crate) static JEST_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("jest")];
pub(crate) static MOCHA_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("mocha")];
pub(crate) static AVA_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("ava")];
pub(crate) static JASMINE_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("jasmine")];
pub(crate) static NODE_TAP_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("tap")];
pub(crate) static UVU_INSTALL: &[InstallationMethod] = &[InstallationMethod::Npm("uvu")];
pub(crate) static PYTEST_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("pytest")];
pub(crate) static NOSE2_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("nose2")];
pub(crate) static TOX_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("tox")];
pub(crate) static NOX_INSTALL: &[InstallationMethod] = &[InstallationMethod::Pip("nox")];
pub(crate) static PHPUNIT_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Composer("phpunit/phpunit")];
pub(crate) static PEST_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Composer("pestphp/pest")];
pub(crate) static CODECEPTION_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Composer("codeception/codeception")];
pub(crate) static BEHAT_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Composer("behat/behat")];
pub(crate) static ATOUM_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Composer("atoum/atoum")];
pub(crate) static RSPEC_INSTALL: &[InstallationMethod] = &[InstallationMethod::Gem("rspec")];
pub(crate) static MINITEST_INSTALL: &[InstallationMethod] = &[InstallationMethod::Gem("minitest")];
pub(crate) static TEST_UNIT_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Gem("test-unit")];
pub(crate) static JUNIT5_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Maven("org.junit.jupiter:junit-jupiter")];
pub(crate) static JUNIT4_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Maven("junit:junit")];
pub(crate) static TESTNG_INSTALL: &[InstallationMethod] =
    &[InstallationMethod::Maven("org.testng:testng")];
pub(crate) static XUNIT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Nuget("xunit"),
    InstallationMethod::Nuget("xunit.runner.visualstudio"),
];
pub(crate) static NUNIT_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Nuget("NUnit"),
    InstallationMethod::Nuget("NUnit3TestAdapter"),
];
pub(crate) static MSTEST_INSTALL: &[InstallationMethod] = &[
    InstallationMethod::Nuget("MSTest"),
    InstallationMethod::Nuget("MSTest.TestFramework"),
    InstallationMethod::Nuget("MSTest.TestAdapter"),
];
pub(crate) static ESPEC_INSTALL: &[InstallationMethod] = &[InstallationMethod::Hex("espec")];

/// Metadata lookup table for test runners.
pub(crate) static TEST_RUNNER_INFO: &[ProgramInfo] = &[
    ProgramInfo {
        binary_name: "cargo-test",
        display_name: "cargo test",
        description: "Rust's built-in test runner via Cargo",
        website: "https://doc.rust-lang.org/cargo/commands/cargo-test.html",
        version_flag: VersionFlag::Subcommand("--version"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/rust-lang/cargo"),
        installation_methods: CARGO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "cargo-nextest",
        display_name: "cargo-nextest",
        description: "Next-generation test runner for Rust",
        website: "https://nexte.st/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &["nextest"],
        os_availability: ALL_OS,
        repo: Some("https://github.com/nextest-rs/nextest"),
        installation_methods: NEXTEST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "go-test",
        display_name: "go test",
        description: "Go's built-in testing tool",
        website: "https://pkg.go.dev/testing",
        version_flag: VersionFlag::Subcommand("version"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/golang/go"),
        installation_methods: GO_MODULES_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "gotestsum",
        display_name: "gotestsum",
        description: "Go test runner with improved output",
        website: "https://github.com/gotestyourself/gotestsum",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/gotestyourself/gotestsum"),
        installation_methods: GOTESTSUM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "ginkgo",
        display_name: "Ginkgo",
        description: "BDD testing framework for Go",
        website: "https://onsi.github.io/ginkgo/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/onsi/ginkgo"),
        installation_methods: GINKGO_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "vitest",
        display_name: "Vitest",
        description: "Blazing fast unit test framework powered by Vite",
        website: "https://vitest.dev/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/vitest-dev/vitest"),
        installation_methods: VITEST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "jest",
        display_name: "Jest",
        description: "Delightful JavaScript testing framework",
        website: "https://jestjs.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/jestjs/jest"),
        installation_methods: JEST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mocha",
        display_name: "Mocha",
        description: "Feature-rich JavaScript test framework",
        website: "https://mochajs.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/mochajs/mocha"),
        installation_methods: MOCHA_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "ava",
        display_name: "AVA",
        description: "Futuristic JavaScript test runner",
        website: "https://github.com/avajs/ava",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/avajs/ava"),
        installation_methods: AVA_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "node-test",
        display_name: "Node Test Runner",
        description: "Node.js built-in test runner",
        website: "https://nodejs.org/api/test.html",
        version_flag: VersionFlag::Subcommand("--version"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/nodejs/node"),
        installation_methods: NPM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "jasmine",
        display_name: "Jasmine",
        description: "Behavior-driven development framework for JavaScript",
        website: "https://jasmine.github.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/jasmine/jasmine"),
        installation_methods: JASMINE_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "tap",
        display_name: "node-tap",
        description: "Test Anything Protocol tools for Node.js",
        website: "https://node-tap.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/tapjs/tapjs"),
        installation_methods: NODE_TAP_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "uvu",
        display_name: "uvu",
        description: "Extremely fast and lightweight test runner",
        website: "https://github.com/lukeed/uvu",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/lukeed/uvu"),
        installation_methods: UVU_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pytest",
        display_name: "pytest",
        description: "Python testing framework",
        website: "https://docs.pytest.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &["py.test"],
        os_availability: ALL_OS,
        repo: Some("https://github.com/pytest-dev/pytest"),
        installation_methods: PYTEST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "python-unittest",
        display_name: "unittest",
        description: "Python's built-in unit testing framework",
        website: "https://docs.python.org/3/library/unittest.html",
        version_flag: VersionFlag::Subcommand("--version"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/python/cpython"),
        installation_methods: &[],
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nose2",
        display_name: "nose2",
        description: "Python testing framework extending unittest",
        website: "https://docs.nose2.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/nose-devs/nose2"),
        installation_methods: NOSE2_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "tox",
        display_name: "tox",
        description: "Python test environment manager",
        website: "https://tox.wiki/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/tox-dev/tox"),
        installation_methods: TOX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nox",
        display_name: "nox",
        description: "Flexible test automation for Python",
        website: "https://nox.thea.codes/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/wntrblm/nox"),
        installation_methods: NOX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "phpunit",
        display_name: "PHPUnit",
        description: "Testing framework for PHP",
        website: "https://phpunit.de/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/sebastianbergmann/phpunit"),
        installation_methods: PHPUNIT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "pest",
        display_name: "Pest",
        description: "Elegant PHP testing framework",
        website: "https://pestphp.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/pestphp/pest"),
        installation_methods: PEST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "codecept",
        display_name: "Codeception",
        description: "Full-stack testing framework for PHP",
        website: "https://codeception.com/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &["codeception"],
        os_availability: ALL_OS,
        repo: Some("https://github.com/Codeception/Codeception"),
        installation_methods: CODECEPTION_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "behat",
        display_name: "Behat",
        description: "BDD framework for PHP",
        website: "https://behat.org/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/Behat/Behat"),
        installation_methods: BEHAT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "atoum",
        display_name: "atoum",
        description: "Modern PHP testing framework",
        website: "https://atoum.github.io/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/atoum/atoum"),
        installation_methods: ATOUM_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "rspec",
        display_name: "RSpec",
        description: "Behavior-driven development framework for Ruby",
        website: "https://rspec.info/",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/rspec/rspec"),
        installation_methods: RSPEC_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "ruby-minitest",
        display_name: "Minitest",
        description: "Ruby's built-in testing library",
        website: "https://github.com/minitest/minitest",
        version_flag: VersionFlag::Subcommand("--version"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/minitest/minitest"),
        installation_methods: MINITEST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "ruby-test-unit",
        display_name: "test-unit",
        description: "Unit testing framework for Ruby",
        website: "https://test-unit.github.io/",
        version_flag: VersionFlag::Subcommand("--version"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/test-unit/test-unit"),
        installation_methods: TEST_UNIT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "junit5",
        display_name: "JUnit 5",
        description: "Popular testing framework for JVM languages",
        website: "https://junit.org/junit5/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/junit-team/junit5"),
        installation_methods: JUNIT5_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "junit4",
        display_name: "JUnit 4",
        description: "Earlier version of the JUnit testing framework",
        website: "https://junit.org/junit4/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/junit-team/junit4"),
        installation_methods: JUNIT4_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "testng",
        display_name: "TestNG",
        description: "Testing framework for Java inspired by JUnit",
        website: "https://testng.org/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/cbeust/testng"),
        installation_methods: TESTNG_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "xunit",
        display_name: "xUnit.net",
        description: "Free, open-source testing framework for .NET",
        website: "https://xunit.net/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/xunit/xunit"),
        installation_methods: XUNIT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "nunit",
        display_name: "NUnit",
        description: "Unit-testing framework for .NET",
        website: "https://nunit.org/",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/nunit/nunit"),
        installation_methods: NUNIT_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mstest",
        display_name: "MSTest",
        description: "Microsoft's testing framework for .NET",
        website: "https://github.com/microsoft/testfx",
        version_flag: VersionFlag::None,
        parse_strategy: VersionParseStrategy::Custom,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/microsoft/testfx"),
        installation_methods: MSTEST_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "mix-test",
        display_name: "mix test",
        description: "Elixir's built-in test runner via Mix",
        website: "https://hexdocs.pm/mix/Mix.Tasks.Test.html",
        version_flag: VersionFlag::Subcommand("--version"),
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/elixir-lang/elixir"),
        installation_methods: HEX_INSTALL,
        system_prerequisites: &[],
    },
    ProgramInfo {
        binary_name: "espec",
        display_name: "ESpec",
        description: "BDD testing framework for Elixir",
        website: "https://github.com/antonmi/espec",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: ALL_OS,
        repo: Some("https://github.com/antonmi/espec"),
        installation_methods: ESPEC_INSTALL,
        system_prerequisites: &[],
    },
];

impl ProgramMetadata for TestRunner {
    fn info(&self) -> &'static ProgramInfo {
        &TEST_RUNNER_INFO[*self as usize]
    }
}

impl CategoryEnum for TestRunner {
    fn category_name() -> &'static str {
        "test_runners"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            TestRunner::CargoTest => "cargo_test",
            TestRunner::Nextest => "nextest",
            TestRunner::GoTest => "go_test",
            TestRunner::Gotestsum => "gotestsum",
            TestRunner::Ginkgo => "ginkgo",
            TestRunner::Vitest => "vitest",
            TestRunner::Jest => "jest",
            TestRunner::Mocha => "mocha",
            TestRunner::Ava => "ava",
            TestRunner::NodeTest => "node_test",
            TestRunner::Jasmine => "jasmine",
            TestRunner::NodeTap => "node_tap",
            TestRunner::Uvu => "uvu",
            TestRunner::Pytest => "pytest",
            TestRunner::Unittest => "unittest",
            TestRunner::Nose2 => "nose2",
            TestRunner::Tox => "tox",
            TestRunner::Nox => "nox",
            TestRunner::PhpUnit => "phpunit",
            TestRunner::Pest => "pest",
            TestRunner::Codeception => "codeception",
            TestRunner::Behat => "behat",
            TestRunner::Atoum => "atoum",
            TestRunner::RSpec => "rspec",
            TestRunner::Minitest => "minitest",
            TestRunner::TestUnit => "test_unit",
            TestRunner::JUnit5 => "junit5",
            TestRunner::JUnit4 => "junit4",
            TestRunner::TestNg => "testng",
            TestRunner::XUnit => "xunit",
            TestRunner::NUnit => "nunit",
            TestRunner::MsTest => "mstest",
            TestRunner::ExUnit => "ex_unit",
            TestRunner::ESpec => "espec",
        }
    }
}
