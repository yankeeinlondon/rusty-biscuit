use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

use crate::output::OutputFilter;

/// Default number of recent commits to display in git output.
pub const DEFAULT_COMMIT_COUNT: usize = 10;

/// Detect system and repository information
#[derive(Parser)]
#[command(
    name = "sniff",
    version,
    about,
    after_help = AFTER_HELP,
    help_template = HELP_TEMPLATE
)]
pub struct Cli {
    /// Base directory for filesystem analysis
    #[arg(short, long, global = true)]
    pub base: Option<PathBuf>,

    /// Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
    #[arg(long, global = true)]
    pub json: bool,

    /// Increase output verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL", hide = true)]
    pub completions: Option<Shell>,

    /// Subcommand to filter output to a specific section
    #[command(subcommand)]
    pub command: Option<Commands>,
}

// ---------------------------------------------------------------------------
// Per-category action enums with shell completion candidates
// ---------------------------------------------------------------------------

macro_rules! define_program_action {
    ($action_name:ident, $candidates_fn:ident, $program_enum:ty) => {
        fn $candidates_fn() -> Vec<clap_complete::engine::CompletionCandidate> {
            use clap_complete::engine::CompletionCandidate;
            use sniff::programs::ProgramMetadata;
            use strum::IntoEnumIterator;

            <$program_enum>::iter()
                .flat_map(|p| {
                    let mut candidates = vec![
                        CompletionCandidate::new(p.binary_name())
                            .help(Some(p.description().into())),
                    ];
                    let snake = p.to_string();
                    if snake != p.binary_name() {
                        candidates.push(
                            CompletionCandidate::new(snake).help(Some(p.description().into())),
                        );
                    }
                    candidates
                })
                .collect()
        }

        #[derive(Subcommand, Debug, Clone)]
        pub enum $action_name {
            /// Install a program (interactive picker if no name given)
            Install {
                /// Program name to install (binary name or identifier)
                #[arg(add = clap_complete::engine::ArgValueCandidates::new($candidates_fn))]
                program: Option<String>,
            },
        }
    };
}

define_program_action!(EditorAction, editor_candidates, sniff::programs::Editor);
define_program_action!(UtilityAction, utility_candidates, sniff::programs::Utility);
define_program_action!(
    LangPkgMgrAction,
    lang_pkg_mgr_candidates,
    sniff::programs::LanguagePackageManager
);
define_program_action!(
    OsPkgMgrAction,
    os_pkg_mgr_candidates,
    sniff::programs::OsPackageManager
);
define_program_action!(
    TtsClientAction,
    tts_client_candidates,
    sniff::programs::TtsClient
);
define_program_action!(
    TerminalAppAction,
    terminal_app_candidates,
    sniff::programs::TerminalApp
);
define_program_action!(
    AudioAction,
    audio_candidates,
    sniff::programs::HeadlessAudio
);
define_program_action!(AgentAction, agent_candidates, sniff::programs::AiCli);

/// Generates merged completion candidates across all program categories.
fn all_program_candidates() -> Vec<clap_complete::engine::CompletionCandidate> {
    let mut all = editor_candidates();
    all.extend(utility_candidates());
    all.extend(lang_pkg_mgr_candidates());
    all.extend(os_pkg_mgr_candidates());
    all.extend(tts_client_candidates());
    all.extend(terminal_app_candidates());
    all.extend(audio_candidates());
    all.extend(agent_candidates());
    all
}

#[derive(Subcommand, Debug, Clone)]
pub enum AllProgramAction {
    /// Install a program (interactive picker if no name given)
    Install {
        /// Program name to install (binary name or identifier)
        #[arg(add = clap_complete::engine::ArgValueCandidates::new(all_program_candidates))]
        program: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Main Commands enum
// ---------------------------------------------------------------------------

/// Subcommands for filtering output to specific sections.
///
/// Each subcommand shows only the specified section of system information.
/// Without a subcommand, all data is output as JSON.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Show only OS information (name, kernel, locale, timezone)
    Os,

    /// Show only hardware information (CPU, GPU, memory, storage)
    Hardware,

    /// Show only network information (interfaces, IP addresses)
    Network,

    /// Show only filesystem information (git, languages, monorepo)
    Filesystem {
        /// Refresh remote-tracking data before reporting branch and sync status
        #[arg(long)]
        refresh_remotes: bool,

        /// Query package registries for latest dependency versions and report available updates
        #[arg(long)]
        latest_versions: bool,
    },

    /// Show a table of subsection topics for each top-level command
    Topics,

    /// Show only CPU information
    Cpu,

    /// Show only GPU information
    Gpu,

    /// Show only memory information
    Memory,

    /// Show only storage/disk information
    Storage,

    /// Show only audio devices (inputs, outputs, sample rates)
    AudioDevices,

    /// Show git repository information, or inspect a remote by name/URL
    #[command(disable_help_flag = true)]
    Git {
        /// Number of recent commits to display (default: 10)
        #[arg(short = 'h', long, default_value_t = DEFAULT_COMMIT_COUNT)]
        history: usize,

        /// Refresh remote-tracking data before reporting branch and sync status
        #[arg(long, conflicts_with_all = ["remote", "hash"])]
        refresh_remotes: bool,

        /// Show details for a specific commit by SHA
        #[arg(long, value_name = "SHA", conflicts_with = "package")]
        hash: Option<String>,

        /// Filter to commits and changes within a package
        #[arg(short, long, value_name = "PKG", conflicts_with = "hash")]
        package: Option<String>,

        /// Remote name (e.g., "origin"), URL, or owner/repo shorthand to inspect
        #[arg(value_name = "REMOTE")]
        remote: Option<String>,

        /// Print help
        #[arg(long, action = clap::ArgAction::Help)]
        help: Option<bool>,
    },

    /// Show only repository/monorepo structure
    Repo {
        /// Render an internal dependency diagram
        #[arg(long)]
        deps: bool,
        /// Output only package names as a comma-separated list
        #[arg(long)]
        packages: bool,
        /// Output the package name for the current directory
        #[arg(long)]
        package: bool,
        /// Output the package area for the current directory
        #[arg(long)]
        package_area: bool,
        /// Output only package names that have uncommitted changes
        #[arg(long)]
        dirty_packages: bool,
        /// Output only package area names that have uncommitted changes
        #[arg(long)]
        dirty_package_areas: bool,
        /// Query package registries for latest dependency versions and report available updates
        #[arg(
            long,
            conflicts_with_all = [
                "deps",
                "packages",
                "package",
                "package_area",
                "dirty_packages",
                "dirty_package_areas",
            ]
        )]
        latest_versions: bool,
        /// Use visual (Mermaid) rendering for --deps instead of text
        #[arg(long)]
        ui: bool,
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Option<String>,
    },

    /// Show only language detection results
    Language,

    /// Show markdown documents in the repository
    Docs {
        /// Show only README files
        #[arg(long)]
        readme: bool,

        /// Show only documents with "plan" in the filename or path
        #[arg(long)]
        plan: bool,

        /// Show only documents under a `src/` directory
        #[arg(long)]
        src: bool,

        /// Show only documents with a prompt in frontmatter
        #[arg(long)]
        has_prompt: bool,

        /// Filter documents by substring match on filepath/filename
        filter: Option<String>,
    },

    /// Show all installed programs detection
    Programs {
        #[command(subcommand)]
        action: Option<AllProgramAction>,
    },

    /// Show only installed editors
    Editors {
        #[command(subcommand)]
        action: Option<EditorAction>,
    },

    /// Show only installed utilities
    Utilities {
        #[command(subcommand)]
        action: Option<UtilityAction>,
    },

    /// Show only language package managers
    LanguagePackageManagers {
        #[command(subcommand)]
        action: Option<LangPkgMgrAction>,
    },

    /// Show only OS package managers
    OsPackageManagers {
        #[command(subcommand)]
        action: Option<OsPkgMgrAction>,
    },

    /// Show only TTS clients
    TtsClients {
        #[command(subcommand)]
        action: Option<TtsClientAction>,
    },

    /// Show only terminal apps
    TerminalApps {
        #[command(subcommand)]
        action: Option<TerminalAppAction>,
    },

    /// Show only headless audio players
    Audio {
        #[command(subcommand)]
        action: Option<AudioAction>,
    },

    /// Show only AI agent/CLI tools
    Agents {
        #[command(subcommand)]
        action: Option<AgentAction>,
    },

    /// Show only system services (init system and service list)
    Services {
        /// Filter services by state
        #[arg(long, value_enum, default_value = "running")]
        state: ServiceStateArg,
    },
}

impl Commands {
    /// Convert command to the corresponding output filter.
    pub fn to_output_filter(&self) -> OutputFilter {
        match self {
            Commands::Os => OutputFilter::Os,
            Commands::Hardware => OutputFilter::Hardware,
            Commands::Network => OutputFilter::Network,
            Commands::Filesystem { .. } => OutputFilter::Filesystem,
            Commands::Topics => OutputFilter::All,
            Commands::Cpu => OutputFilter::Cpu,
            Commands::Gpu => OutputFilter::Gpu,
            Commands::Memory => OutputFilter::Memory,
            Commands::Storage => OutputFilter::Storage,
            Commands::AudioDevices => OutputFilter::AudioDevices,
            Commands::Git { .. } => OutputFilter::Git,
            Commands::Repo { .. } => OutputFilter::Repo,
            Commands::Language => OutputFilter::Language,
            Commands::Docs { .. } => OutputFilter::Docs,
            Commands::Programs { .. } => OutputFilter::Programs,
            Commands::Editors { .. } => OutputFilter::Editors,
            Commands::Utilities { .. } => OutputFilter::Utilities,
            Commands::LanguagePackageManagers { .. } => OutputFilter::LanguagePackageManagers,
            Commands::OsPackageManagers { .. } => OutputFilter::OsPackageManagers,
            Commands::TtsClients { .. } => OutputFilter::TtsClients,
            Commands::TerminalApps { .. } => OutputFilter::TerminalApps,
            Commands::Audio { .. } => OutputFilter::HeadlessAudio,
            Commands::Agents { .. } => OutputFilter::AiClients,
            Commands::Services { .. } => OutputFilter::Services,
        }
    }

    /// Check if this is a programs-related command.
    pub fn is_programs_mode(&self) -> bool {
        matches!(
            self,
            Commands::Programs { .. }
                | Commands::Editors { .. }
                | Commands::Utilities { .. }
                | Commands::LanguagePackageManagers { .. }
                | Commands::OsPackageManagers { .. }
                | Commands::TtsClients { .. }
                | Commands::TerminalApps { .. }
                | Commands::Audio { .. }
                | Commands::Agents { .. }
        )
    }

    /// Returns true if this command has an install action.
    pub fn is_install_action(&self) -> bool {
        matches!(
            self,
            Commands::Programs {
                action: Some(AllProgramAction::Install { .. }),
            } | Commands::Editors {
                action: Some(EditorAction::Install { .. }),
            } | Commands::Utilities {
                action: Some(UtilityAction::Install { .. }),
            } | Commands::LanguagePackageManagers {
                action: Some(LangPkgMgrAction::Install { .. }),
            } | Commands::OsPackageManagers {
                action: Some(OsPkgMgrAction::Install { .. }),
            } | Commands::TtsClients {
                action: Some(TtsClientAction::Install { .. }),
            } | Commands::TerminalApps {
                action: Some(TerminalAppAction::Install { .. }),
            } | Commands::Audio {
                action: Some(AudioAction::Install { .. }),
            } | Commands::Agents {
                action: Some(AgentAction::Install { .. }),
            }
        )
    }

    /// Returns the program name from an install action, if present.
    pub fn install_program_name(&self) -> Option<&str> {
        match self {
            Commands::Programs {
                action: Some(AllProgramAction::Install { program }),
            }
            | Commands::Editors {
                action: Some(EditorAction::Install { program }),
            }
            | Commands::Utilities {
                action: Some(UtilityAction::Install { program }),
            }
            | Commands::LanguagePackageManagers {
                action: Some(LangPkgMgrAction::Install { program }),
            }
            | Commands::OsPackageManagers {
                action: Some(OsPkgMgrAction::Install { program }),
            }
            | Commands::TtsClients {
                action: Some(TtsClientAction::Install { program }),
            }
            | Commands::TerminalApps {
                action: Some(TerminalAppAction::Install { program }),
            }
            | Commands::Audio {
                action: Some(AudioAction::Install { program }),
            }
            | Commands::Agents {
                action: Some(AgentAction::Install { program }),
            } => program.as_deref(),
            _ => None,
        }
    }

    /// Get state filter if this is a services command.
    pub fn state(&self) -> Option<ServiceStateArg> {
        match self {
            Commands::Services { state } => Some(*state),
            _ => None,
        }
    }

    /// Get history count if this is a git command.
    pub fn history(&self) -> usize {
        match self {
            Commands::Git { history, .. } => *history,
            _ => DEFAULT_COMMIT_COUNT,
        }
    }

    /// Check if this is a repo command with `--deps` flag.
    pub fn deps(&self) -> bool {
        matches!(self, Commands::Repo { deps: true, .. })
    }

    /// Check if this is a repo command with `--packages` flag.
    pub fn packages(&self) -> bool {
        matches!(self, Commands::Repo { packages: true, .. })
    }

    /// Check if this is a repo command with `--package` flag.
    pub fn package(&self) -> bool {
        matches!(self, Commands::Repo { package: true, .. })
    }

    /// Check if this is a repo command with `--package-area` flag.
    pub fn package_area(&self) -> bool {
        matches!(
            self,
            Commands::Repo {
                package_area: true,
                ..
            }
        )
    }

    /// Check if this is a repo command with `--dirty-packages` flag.
    pub fn dirty_packages(&self) -> bool {
        matches!(
            self,
            Commands::Repo {
                dirty_packages: true,
                ..
            }
        )
    }

    /// Check if this is a repo command with `--dirty-package-areas` flag.
    pub fn dirty_package_areas(&self) -> bool {
        matches!(
            self,
            Commands::Repo {
                dirty_package_areas: true,
                ..
            }
        )
    }

    /// Check if this command requests remote refresh data.
    pub fn refresh_remotes(&self) -> bool {
        matches!(
            self,
            Commands::Filesystem {
                refresh_remotes: true,
                ..
            } | Commands::Git {
                refresh_remotes: true,
                ..
            }
        )
    }

    /// Check if this command requests dependency latest-version enrichment.
    pub fn latest_versions(&self) -> bool {
        matches!(
            self,
            Commands::Filesystem {
                latest_versions: true,
                ..
            } | Commands::Repo {
                latest_versions: true,
                ..
            }
        )
    }

    /// Check if this is a repo command with `--ui` flag.
    pub fn ui(&self) -> bool {
        matches!(self, Commands::Repo { ui: true, .. })
    }

    /// Get the repo filter string if this is a repo command with a filter arg.
    pub fn repo_filter(&self) -> Option<&str> {
        match self {
            Commands::Repo { filter, .. } => filter.as_deref(),
            _ => None,
        }
    }

    /// Get remote name/URL if this is a git command with a remote arg.
    pub fn git_remote(&self) -> Option<&str> {
        match self {
            Commands::Git { remote, .. } => remote.as_deref(),
            _ => None,
        }
    }

    /// Get commit SHA if this is a git command with `--hash`.
    pub fn git_hash(&self) -> Option<&str> {
        match self {
            Commands::Git { hash, .. } => hash.as_deref(),
            _ => None,
        }
    }

    /// Get package name if this is a git command with `--package`.
    pub fn git_package(&self) -> Option<&str> {
        match self {
            Commands::Git { package, .. } => package.as_deref(),
            _ => None,
        }
    }

    /// Get docs filter flags if this is a docs command.
    pub fn docs_filter(&self) -> DocsFilter {
        match self {
            Commands::Docs {
                readme,
                plan,
                src,
                has_prompt,
                filter,
            } => DocsFilter {
                readme: *readme,
                plan: *plan,
                src: *src,
                has_prompt: *has_prompt,
                filter: filter.clone(),
            },
            _ => DocsFilter::default(),
        }
    }
}

/// Filter options for the docs subcommand.
#[derive(Debug, Clone, Default)]
pub struct DocsFilter {
    /// Show only README files (case-insensitive).
    pub readme: bool,
    /// Show only documents with "plan" in the filename or path.
    pub plan: bool,
    /// Show only documents under a `src/` directory.
    pub src: bool,
    /// Show only documents with a prompt in frontmatter.
    pub has_prompt: bool,
    /// Substring filter on filepath/filename (case-insensitive).
    pub filter: Option<String>,
}

/// Service state filter for services subcommand.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum ServiceStateArg {
    All,
    #[default]
    Running,
    Stopped,
}

pub const HELP_TEMPLATE: &str = "\
{name} {version}
{about}

Usage: {usage}

{options}
{after-help}";

pub const AFTER_HELP: &str = "\
Commands:
  Top-level sections:
    sniff os          Show only OS information
    sniff hardware    Show only hardware information
    sniff network     Show only network information
    sniff filesystem  Show only filesystem information
    sniff topics      Show subsection topics as a table

  Hardware details:
    sniff cpu             Show only CPU information
    sniff gpu             Show only GPU information
    sniff memory          Show only memory information
    sniff storage         Show only storage/disk information
    sniff audio-devices   Show only audio devices

  Filesystem details:
    sniff git                        Show only git repository information
    sniff git --refresh-remotes      Refresh remotes before reporting sync status
    sniff git --hash HEAD            Show details for the latest commit
    sniff git --hash abc1234         Show details for a specific commit
    sniff git --package homelab      Scope to commits within a package
    sniff git origin                 Inspect the 'origin' remote
    sniff git owner/repo             Inspect by owner/repo shorthand
    sniff git https://github.com/... Inspect a remote by URL
    sniff repo                       Show only repository/monorepo structure
    sniff repo --latest-versions     Check registries for dependency updates
    sniff repo biscuit               Filter to packages matching \"biscuit\"
    sniff repo !biscuit              Exclude packages matching \"biscuit\"
    sniff repo @sniff                Filter to packages in the \"sniff\" area
    sniff repo --deps                Show internal dependency list (text)
    sniff repo --deps --ui           Show internal dependency diagram (Mermaid)
    sniff repo --deps biscuit        Filtered text dependency list
    sniff repo --packages biscuit    Filtered CSV package names
    sniff language                   Show only language detection results
    sniff docs                       Show markdown documents in the repository
    sniff docs --readme              Show only README.md files
    sniff docs --plan                Show only plan-related documents
    sniff docs --src                 Show only documents under src/ directories
    sniff docs --has-prompt          Show only documents with a prompt
    sniff docs homelab               Filter documents matching \"homelab\"

  Programs:
    sniff programs                   Show all installed programs
    sniff editors                    Show only installed editors
    sniff editors install            Interactive install picker for editors
    sniff editors install vim        Install vim directly
    sniff utilities                  Show only installed utilities
    sniff language-package-managers  Show only language package managers
    sniff os-package-managers        Show only OS package managers
    sniff tts-clients                Show only TTS clients
    sniff terminal-apps              Show only terminal apps
    sniff audio                      Show only headless audio players
    sniff agents                     Show only AI agent CLI tools

  Services:
    sniff services              Show running services (default)
    sniff services --state all  Show all services

Output modes:
  - No subcommand: Show this help (use --json for full JSON output)
  - With subcommand: Text output by default, use --json for JSON

Examples:
  sniff                      # Show this help
  sniff --json               # Full system info as JSON
  sniff cpu                  # CPU info as text
  sniff cpu --json           # CPU info as JSON
  sniff --json cpu           # Same as above (flag position flexible)
  sniff programs             # Programs as text
  sniff programs --json      # Programs as JSON
  sniff filesystem --refresh-remotes --latest-versions  # Enriched filesystem report
  sniff editors install      # Interactive editor install picker
  sniff -b /path/to/repo filesystem  # Analyze specific directory
";

pub const COMPLETIONS_HELP: &str = "\
Shell completions

Usage:
  sniff --completions <SHELL>

Available shells:
  bash, elvish, fish, powershell, zsh

Setup commands:
  Bash (add to ~/.bashrc):
    source <(COMPLETE=bash sniff)

  Zsh (add to ~/.zshrc):
    source <(COMPLETE=zsh sniff)

  Fish (add to ~/.config/fish/config.fish):
    COMPLETE=fish sniff | source

  PowerShell (add to $PROFILE):
    $env:COMPLETE = \"powershell\"; sniff | Out-String | Invoke-Expression; Remove-Item Env:\\COMPLETE

  Elvish (add to ~/.elvish/rc.elv):
    eval (E:COMPLETE=elvish sniff | slurp)
";

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse_args(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("sniff").chain(args.iter().copied()))
    }

    mod subcommand_parsing {
        use super::*;

        #[test]
        fn no_subcommand_parses() {
            let cli = parse_args(&[]).unwrap();
            assert!(cli.command.is_none());
        }

        #[test]
        fn common_subcommands_parse() {
            assert!(matches!(
                parse_args(&["os"]).unwrap().command,
                Some(Commands::Os)
            ));
            assert!(matches!(
                parse_args(&["hardware"]).unwrap().command,
                Some(Commands::Hardware)
            ));
            assert!(matches!(
                parse_args(&["network"]).unwrap().command,
                Some(Commands::Network)
            ));
            assert!(matches!(
                parse_args(&["filesystem"]).unwrap().command,
                Some(Commands::Filesystem { .. })
            ));
            assert!(matches!(
                parse_args(&["cpu"]).unwrap().command,
                Some(Commands::Cpu)
            ));
            assert!(matches!(
                parse_args(&["gpu"]).unwrap().command,
                Some(Commands::Gpu)
            ));
            assert!(matches!(
                parse_args(&["memory"]).unwrap().command,
                Some(Commands::Memory)
            ));
            assert!(matches!(
                parse_args(&["storage"]).unwrap().command,
                Some(Commands::Storage)
            ));
            assert!(matches!(
                parse_args(&["audio-devices"]).unwrap().command,
                Some(Commands::AudioDevices)
            ));
            assert!(matches!(
                parse_args(&["language"]).unwrap().command,
                Some(Commands::Language)
            ));
            assert!(matches!(
                parse_args(&["topics"]).unwrap().command,
                Some(Commands::Topics)
            ));
        }

        #[test]
        fn git_flags_and_remote_parse() {
            let cli = parse_args(&["git", "--history", "10", "--refresh-remotes"]).unwrap();
            if let Some(Commands::Git {
                history,
                refresh_remotes,
                remote,
                ..
            }) = cli.command
            {
                assert_eq!(history, 10);
                assert!(refresh_remotes);
                assert!(remote.is_none());
            } else {
                panic!("Expected Git command");
            }
        }

        #[test]
        fn docs_flags_and_filter_parse() {
            let cli = parse_args(&["docs", "--readme", "--has-prompt", "research"]).unwrap();
            if let Some(Commands::Docs {
                readme,
                has_prompt,
                filter,
                ..
            }) = cli.command
            {
                assert!(readme);
                assert!(has_prompt);
                assert_eq!(filter.as_deref(), Some("research"));
            } else {
                panic!("Expected Docs command");
            }
        }

        #[test]
        fn repo_flags_and_filter_parse() {
            let cli = parse_args(&["repo", "--latest-versions", "@sniff"]).unwrap();
            if let Some(Commands::Repo {
                filter,
                latest_versions,
                ..
            }) = cli.command
            {
                assert!(latest_versions);
                assert_eq!(filter.as_deref(), Some("@sniff"));
            } else {
                panic!("Expected Repo command");
            }
        }

        #[test]
        fn editors_install_no_name_parses() {
            let cli = parse_args(&["editors", "install"]).unwrap();
            if let Some(Commands::Editors {
                action: Some(EditorAction::Install { program }),
            }) = cli.command
            {
                assert!(program.is_none());
            } else {
                panic!("Expected Editors install with no program");
            }
        }

        #[test]
        fn editors_install_with_name_parses() {
            let cli = parse_args(&["editors", "install", "vim"]).unwrap();
            if let Some(Commands::Editors {
                action: Some(EditorAction::Install { program }),
            }) = cli.command
            {
                assert_eq!(program.as_deref(), Some("vim"));
            } else {
                panic!("Expected Editors install with program name");
            }
        }

        #[test]
        fn editors_without_action_parses() {
            let cli = parse_args(&["editors"]).unwrap();
            if let Some(Commands::Editors { action }) = cli.command {
                assert!(action.is_none());
            } else {
                panic!("Expected Editors with no action");
            }
        }

        #[test]
        fn programs_install_no_name_parses() {
            let cli = parse_args(&["programs", "install"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Programs {
                    action: Some(AllProgramAction::Install { program: None }),
                })
            ));
        }

        #[test]
        fn programs_install_with_name_parses() {
            let cli = parse_args(&["programs", "install", "nvim"]).unwrap();
            if let Some(Commands::Programs {
                action: Some(AllProgramAction::Install { program }),
            }) = cli.command
            {
                assert_eq!(program.as_deref(), Some("nvim"));
            } else {
                panic!("Expected Programs install with program name");
            }
        }
    }

    mod accessors {
        use super::*;

        #[test]
        fn output_filter_mapping_covers_modes() {
            assert_eq!(Commands::Os.to_output_filter(), OutputFilter::Os);
            assert_eq!(Commands::Cpu.to_output_filter(), OutputFilter::Cpu);
            assert_eq!(
                Commands::AudioDevices.to_output_filter(),
                OutputFilter::AudioDevices
            );
            assert_eq!(
                Commands::Repo {
                    deps: false,
                    packages: false,
                    package: false,
                    package_area: false,
                    dirty_packages: false,
                    dirty_package_areas: false,
                    latest_versions: false,
                    ui: false,
                    filter: None,
                }
                .to_output_filter(),
                OutputFilter::Repo
            );
            assert_eq!(
                Commands::Programs { action: None }.to_output_filter(),
                OutputFilter::Programs
            );
            assert_eq!(
                Commands::Services {
                    state: ServiceStateArg::Running,
                }
                .to_output_filter(),
                OutputFilter::Services
            );
        }

        #[test]
        fn programs_mode_accessors_work() {
            let cmd = Commands::Programs { action: None };
            assert!(cmd.is_programs_mode());

            let non_programs = Commands::Cpu;
            assert!(!non_programs.is_programs_mode());
        }

        #[test]
        fn is_install_action_returns_true() {
            let cmd = Commands::Editors {
                action: Some(EditorAction::Install { program: None }),
            };
            assert!(cmd.is_install_action());

            let cmd = Commands::Programs {
                action: Some(AllProgramAction::Install {
                    program: Some("vim".to_string()),
                }),
            };
            assert!(cmd.is_install_action());
        }

        #[test]
        fn is_install_action_returns_false() {
            let cmd = Commands::Editors { action: None };
            assert!(!cmd.is_install_action());

            let cmd = Commands::Cpu;
            assert!(!cmd.is_install_action());
        }

        #[test]
        fn install_program_name_extracts_name() {
            let cmd = Commands::Editors {
                action: Some(EditorAction::Install {
                    program: Some("vim".to_string()),
                }),
            };
            assert_eq!(cmd.install_program_name(), Some("vim"));

            let cmd = Commands::Editors {
                action: Some(EditorAction::Install { program: None }),
            };
            assert_eq!(cmd.install_program_name(), None);

            let cmd = Commands::Editors { action: None };
            assert_eq!(cmd.install_program_name(), None);
        }

        #[test]
        fn repo_accessors_work() {
            let cmd = Commands::Repo {
                deps: true,
                packages: true,
                package: true,
                package_area: true,
                dirty_packages: true,
                dirty_package_areas: true,
                latest_versions: true,
                ui: true,
                filter: Some("biscuit".to_string()),
            };

            assert!(cmd.deps());
            assert!(cmd.packages());
            assert!(cmd.package());
            assert!(cmd.package_area());
            assert!(cmd.dirty_packages());
            assert!(cmd.dirty_package_areas());
            assert!(cmd.latest_versions());
            assert!(cmd.ui());
            assert_eq!(cmd.repo_filter(), Some("biscuit"));
        }

        #[test]
        fn git_and_docs_accessors_work() {
            let git = Commands::Git {
                history: 3,
                refresh_remotes: true,
                hash: None,
                package: None,
                remote: Some("owner/repo".to_string()),
                help: None,
            };
            assert_eq!(git.history(), 3);
            assert_eq!(git.git_remote(), Some("owner/repo"));
            assert!(git.refresh_remotes());

            let docs = Commands::Docs {
                readme: true,
                plan: false,
                src: true,
                has_prompt: false,
                filter: Some("homelab".to_string()),
            };
            let filter = docs.docs_filter();
            assert!(filter.readme);
            assert!(filter.src);
            assert_eq!(filter.filter.as_deref(), Some("homelab"));
        }

        #[test]
        fn services_state_accessor_works() {
            let running = parse_args(&["services"]).unwrap();
            if let Some(Commands::Services { state }) = running.command {
                assert!(matches!(state, ServiceStateArg::Running));
            } else {
                panic!("Expected Services command");
            }

            let all = parse_args(&["services", "--state", "all"]).unwrap();
            if let Some(Commands::Services { state }) = all.command {
                assert!(matches!(state, ServiceStateArg::All));
            } else {
                panic!("Expected Services command");
            }
        }
    }

    mod global_flags {
        use super::*;

        #[test]
        fn global_flags_parse_before_or_after_subcommand() {
            let before = parse_args(&["--json", "-v", "cpu"]).unwrap();
            assert!(before.json);
            assert_eq!(before.verbose, 1);
            assert!(matches!(before.command, Some(Commands::Cpu)));

            let after = parse_args(&["cpu", "--json", "-vv"]).unwrap();
            assert!(after.json);
            assert_eq!(after.verbose, 2);
            assert!(matches!(after.command, Some(Commands::Cpu)));
        }

        #[test]
        fn base_flag_works_globally() {
            let cli = parse_args(&["-b", "/tmp", "filesystem"]).unwrap();
            assert_eq!(cli.base, Some(PathBuf::from("/tmp")));
            assert!(matches!(cli.command, Some(Commands::Filesystem { .. })));
        }

        #[test]
        fn scoped_flags_parse_for_supported_commands() {
            let filesystem = parse_args(&["filesystem", "--refresh-remotes", "--latest-versions"])
                .unwrap();
            assert!(matches!(
                filesystem.command,
                Some(Commands::Filesystem {
                    refresh_remotes: true,
                    latest_versions: true,
                })
            ));

            let git = parse_args(&["git", "--refresh-remotes"]).unwrap();
            assert!(git
                .command
                .as_ref()
                .is_some_and(Commands::refresh_remotes));

            let repo = parse_args(&["repo", "--latest-versions"]).unwrap();
            assert!(repo
                .command
                .as_ref()
                .is_some_and(Commands::latest_versions));
        }

        #[test]
        fn unsupported_combinations_fail() {
            assert!(parse_args(&["repo", "--deps", "--latest-versions"]).is_err());
            assert!(parse_args(&["git", "origin", "--refresh-remotes"]).is_err());
        }
    }
}
