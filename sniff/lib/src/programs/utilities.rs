use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::Utility;
use crate::programs::find_program::find_programs_parallel;
use crate::programs::installer::{
    execute_install, execute_versioned_install, method_available, select_best_method,
    InstallOptions,
};
use crate::programs::schema::{ProgramError, ProgramMetadata};
use crate::programs::types::ProgramDetector;
use crate::programs::{
    InstalledLanguagePackageManagers, InstalledOsPackageManagers, Program, PROGRAM_LOOKUP,
};

fn utility_details(utility: Utility) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match utility {
        Utility::Exa => Program::Exa,
        Utility::Eza => Program::Eza,
        Utility::Ripgrep => Program::Ripgrep,
        Utility::Dust => Program::Dust,
        Utility::Bat => Program::Bat,
        Utility::Fd => Program::Fd,
        Utility::Procs => Program::Procs,
        Utility::Bottom => Program::Bottom,
        Utility::Fzf => Program::Fzf,
        Utility::Zoxide => Program::Zoxide,
        Utility::Starship => Program::Starship,
        Utility::Direnv => Program::Direnv,
        Utility::Jq => Program::Jq,
        Utility::Delta => Program::Delta,
        Utility::Tealdeer => Program::Tealdeer,
        Utility::Lazygit => Program::Lazygit,
        Utility::Gh => Program::Gh,
        Utility::Htop => Program::Htop,
        Utility::Btop => Program::Btop,
        Utility::Tmux => Program::Tmux,
        Utility::Zellij => Program::Zellij,
        Utility::Httpie => Program::Httpie,
        Utility::Curlie => Program::Curlie,
        Utility::Mise => Program::Mise,
        Utility::Hyperfine => Program::Hyperfine,
        Utility::Tokei => Program::Tokei,
        Utility::Xh => Program::Xh,
        Utility::Curl => Program::Curl,
        Utility::Wget => Program::Wget,
        Utility::Iperf3 => Program::Iperf3,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// Popular modern utility programs found on macOS, Linux, or Windows.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct InstalledUtilities {
    /// a replacement for `ls`; but older than the modern `eza`. [Website](https://the.exa.website/)
    pub exa: bool,
    /// a modern replacement for `ls`. [Website](https://eza.rocks/)
    pub eza: bool,
    /// a faster and more feature rich grep utility. [Website](https://github.com/BurntSushi/ripgrep)
    pub ripgrep: bool,
    /// a modern and more visually pleasing version of `du`. [Website](https://github.com/bootandy/dust)
    pub dust: bool,
    /// a cat(1) clone with syntax highlighting and Git integration. [Website](https://github.com/sharkdp/bat)
    pub bat: bool,
    /// a simple, fast and user-friendly alternative to 'find'. [Website](https://github.com/sharkdp/fd)
    pub fd: bool,
    /// a modern replacement for ps. [Website](https://github.com/dalance/procs)
    pub procs: bool,
    /// a cross-platform graphical process/system monitor. [Website](https://github.com/ClementTsang/bottom)
    pub bottom: bool,
    /// a command-line fuzzy finder. [Website](https://github.com/junegunn/fzf)
    pub fzf: bool,
    /// a smarter cd command. [Website](https://github.com/ajeetdsouza/zoxide)
    pub zoxide: bool,
    /// the minimal, blazing-fast, and infinitely customizable prompt for any shell. [Website](https://starship.rs/)
    pub starship: bool,
    /// unclutter your .profile (environment manager). [Website](https://direnv.net/)
    pub direnv: bool,
    /// command-line JSON processor. [Website](https://jqlang.github.io/jq/)
    pub jq: bool,
    /// a viewer for git and diff output. [Website](https://github.com/dandavison/delta)
    pub delta: bool,
    /// a fast tldr client that displays simplified man pages. [Website](https://github.com/dbrgn/tealdeer)
    pub tealdeer: bool,
    /// simple terminal UI for git commands. [Website](https://github.com/jesseduffield/lazygit)
    pub lazygit: bool,
    /// GitHub CLI. [Website](https://cli.github.com/)
    pub gh: bool,
    /// interactive process viewer. [Website](https://htop.dev/)
    pub htop: bool,
    /// a monitor of resources. [Website](https://github.com/aristocratos/btop)
    pub btop: bool,
    /// Terminal multiplexer. [Website](https://github.com/tmux/tmux/wiki)
    pub tmux: bool,
    /// A modern terminal multiplexer. [Website](https://zellij.dev/)
    pub zellij: bool,
    /// HTTP client for the CLI. [Website](https://httpie.io/)
    pub httpie: bool,
    /// A simple, fast and user-friendly alternative to 'curl'. [Website](https://github.com/rs/curlie)
    pub curlie: bool,
    /// A fast, all-in-one tool for your development workflow. [Website](https://mise.jdx.dev/)
    pub mise: bool,
    /// A command-line benchmarking tool. [Website](https://github.com/sharkdp/hyperfine)
    pub hyperfine: bool,
    /// A program that displays statistics about your code. [Website](https://github.com/XAMPPRocky/tokei)
    pub tokei: bool,
    /// A fast and friendly tool for sending HTTP requests. [Website](https://github.com/ducaale/xh)
    pub xh: bool,
    /// Command line tool for transferring data with URLs. [Website](https://curl.se/)
    pub curl: bool,
    /// Network utility to retrieve content from web servers. [Website](https://www.gnu.org/software/wget/)
    pub wget: bool,
    /// A tool for real-time measurements of the maximum achievable bandwidth on IP networks. [Website](https://iperf.fr/)
    pub iperf3: bool,
}

impl InstalledUtilities {
    /// Detect which popular utilities are installed on the system.
    pub fn new() -> Self {
        let programs = [
            "exa",
            "eza",
            "rg",
            "ripgrep",
            "dust",
            "bat",
            "batcat",
            "fd",
            "fdfind",
            "procs",
            "btm",
            "bottom",
            "fzf",
            "zoxide",
            "starship",
            "direnv",
            "jq",
            "delta",
            "tldr",
            "tealdeer",
            "lazygit",
            "gh",
            "htop",
            "btop",
            "tmux",
            "zellij",
            "http",
            "https",
            "httpie",
            "curlie",
            "mise",
            "hyperfine",
            "tokei",
            "xh",
            "xhs",
            "curl",
            "wget",
            "iperf3",
        ];

        let results = find_programs_parallel(&programs);

        let has = |name: &str| results.get(name).and_then(|r| r.as_ref()).is_some();
        let any = |names: &[&str]| names.iter().any(|&name| has(name));

        Self {
            exa: has("exa"),
            eza: has("eza"),
            ripgrep: any(&["rg", "ripgrep"]),
            dust: has("dust"),
            bat: any(&["bat", "batcat"]),
            fd: any(&["fd", "fdfind"]),
            procs: has("procs"),
            bottom: any(&["btm", "bottom"]),
            fzf: has("fzf"),
            zoxide: has("zoxide"),
            starship: has("starship"),
            direnv: has("direnv"),
            jq: has("jq"),
            delta: has("delta"),
            tealdeer: any(&["tldr", "tealdeer"]),
            lazygit: has("lazygit"),
            gh: has("gh"),
            htop: has("htop"),
            btop: has("btop"),
            tmux: has("tmux"),
            zellij: has("zellij"),
            httpie: any(&["http", "https", "httpie"]),
            curlie: has("curlie"),
            mise: has("mise"),
            hyperfine: has("hyperfine"),
            tokei: has("tokei"),
            xh: any(&["xh", "xhs"]),
            curl: has("curl"),
            wget: has("wget"),
            iperf3: has("iperf3"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns the path to the specified utility's binary if installed.
    pub fn path(&self, utility: Utility) -> Option<PathBuf> {
        if self.is_installed(utility) {
            utility.path()
        } else {
            None
        }
    }

    /// Returns the version of the specified utility if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The utility is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, utility: Utility) -> Result<String, ProgramError> {
        if !self.is_installed(utility) {
            return Err(ProgramError::NotFound(utility.binary_name().to_string()));
        }
        utility.version()
    }

    /// Returns the official website URL for the specified utility.
    pub fn website(&self, utility: Utility) -> &'static str {
        utility.website()
    }

    /// Returns a one-line description of the specified utility.
    pub fn description(&self, utility: Utility) -> &'static str {
        utility.description()
    }

    /// Checks if the specified utility is installed.
    pub fn is_installed(&self, utility: Utility) -> bool {
        match utility {
            Utility::Exa => self.exa,
            Utility::Eza => self.eza,
            Utility::Ripgrep => self.ripgrep,
            Utility::Dust => self.dust,
            Utility::Bat => self.bat,
            Utility::Fd => self.fd,
            Utility::Procs => self.procs,
            Utility::Bottom => self.bottom,
            Utility::Fzf => self.fzf,
            Utility::Zoxide => self.zoxide,
            Utility::Starship => self.starship,
            Utility::Direnv => self.direnv,
            Utility::Jq => self.jq,
            Utility::Delta => self.delta,
            Utility::Tealdeer => self.tealdeer,
            Utility::Lazygit => self.lazygit,
            Utility::Gh => self.gh,
            Utility::Htop => self.htop,
            Utility::Btop => self.btop,
            Utility::Tmux => self.tmux,
            Utility::Zellij => self.zellij,
            Utility::Httpie => self.httpie,
            Utility::Curlie => self.curlie,
            Utility::Mise => self.mise,
            Utility::Hyperfine => self.hyperfine,
            Utility::Tokei => self.tokei,
            Utility::Xh => self.xh,
            Utility::Curl => self.curl,
            Utility::Wget => self.wget,
            Utility::Iperf3 => self.iperf3,
        }
    }

    /// Returns a list of all installed utilities.
    pub fn installed(&self) -> Vec<Utility> {
        use strum::IntoEnumIterator;
        Utility::iter().filter(|u| self.is_installed(*u)).collect()
    }
}

impl ProgramDetector for InstalledUtilities {
    type Program = Utility;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledUtilities::path(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledUtilities::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledUtilities::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledUtilities::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledUtilities::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = utility_details(program) else {
            return false;
        };

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return false;
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();

        details
            .installation_methods
            .iter()
            .any(|method| method_available(method, &os_pkg_mgrs, &lang_pkg_mgrs))
    }

    fn install(&self, program: Self::Program) -> Result<(), SniffInstallationError> {
        let details =
            utility_details(program).ok_or_else(|| SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            })?;

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: os_type.to_string(),
            });
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();
        let method = select_best_method(details.installation_methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .ok_or_else(|| SniffInstallationError::MissingPackageManager {
                pkg: program.display_name().to_string(),
                manager: "package manager".to_string(),
            })?;

        let _result = execute_install(method, &InstallOptions::default())?;
        Ok(())
    }

    fn install_version(
        &self,
        program: Self::Program,
        version: &str,
    ) -> Result<(), SniffInstallationError> {
        let details =
            utility_details(program).ok_or_else(|| SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            })?;

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: os_type.to_string(),
            });
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();
        let method = select_best_method(details.installation_methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .ok_or_else(|| SniffInstallationError::MissingPackageManager {
                pkg: program.display_name().to_string(),
                manager: "package manager".to_string(),
            })?;

        let _result = execute_versioned_install(method, version, &InstallOptions::default())?;
        Ok(())
    }
}
