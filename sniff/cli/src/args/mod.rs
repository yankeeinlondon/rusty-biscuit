use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

use crate::output::OutputFilter;

mod repo;
pub use repo::{RepoAction, RepoSubcommand};
pub(crate) use repo::{repo_package_area_candidates, repo_package_candidates};

/// Default number of recent commits to display in git output.
pub const DEFAULT_COMMIT_COUNT: usize = 10;

/// Shared arguments for commands that list file paths.
#[derive(clap::Args, Debug, Clone)]
pub struct FileListArgs {
    /// Scope to a specific package
    #[arg(long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
    pub package: Option<String>,

    /// Scope to a specific package area
    #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
    pub package_area: Option<String>,

    /// Output as bullet list (one item per line with `- ` prefix)
    #[arg(long, conflicts_with = "csv")]
    pub list: bool,

    /// Output as comma-separated values on a single line
    #[arg(long, conflicts_with = "list")]
    pub csv: bool,

    /// Show only basename (hide directory path)
    #[arg(long)]
    pub no_path: bool,

    /// Exit 0 with no output when no results found (default is exit 1)
    #[arg(long)]
    pub no_error: bool,

    /// Message to display when no results found
    #[arg(long, value_name = "MESSAGE")]
    pub on_error: Option<String>,

    /// Filter paths by substring match (OR logic)
    pub filter: Vec<String>,
}

// ---------------------------------------------------------------------------
// Blast-radius scope argument
// ---------------------------------------------------------------------------

/// Which change scope to inspect for blast-radius analysis.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum BlastRadiusScopeArg {
    #[default]
    Dirty,
    Staged,
    LastCommit,
}

/// Output shape for the `repo packages` subcommand.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PackagesFormat {
    /// Comma-separated values on a single line (default).
    #[default]
    Csv,
    /// Markdown unordered list (`- name` per line).
    Markdown,
    /// Plain list (one entry per line, no bullet).
    List,
}

/// Detect system and repository information
#[derive(Parser)]
#[command(
    name = "sniff",
    version,
    about,
    after_help = AFTER_HELP,
    help_template = HELP_TEMPLATE,
    disable_help_subcommand = true,
)]
pub struct Cli {
    /// Base directory for filesystem analysis
    #[arg(short, long, global = true)]
    pub base: Option<PathBuf>,

    /// Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
    #[arg(long, global = true)]
    pub json: bool,

    /// Include structured performance timings and counters in the output
    #[arg(long, global = true)]
    pub perf: bool,

    /// Increase output verbosity (styled user output only; never raw tracing)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Emit raw developer tracing to stderr; repeat for higher verbosity
    #[arg(long, action = clap::ArgAction::Count, global = true)]
    pub debug: u8,

    /// Strip terminal escape codes from text output
    #[arg(long, global = true)]
    pub plain: bool,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL", hide = true)]
    pub completions: Option<Shell>,

    /// Subcommand to filter output to a specific section
    #[command(subcommand)]
    pub command: Option<Commands>,
}

// ---------------------------------------------------------------------------
// Shared install flag group
// ---------------------------------------------------------------------------

/// Shared flag group for `install` and `install-plan` subcommands across every
/// program category.
#[derive(clap::Args, Debug, Clone, Default)]
pub struct InstallCommandArgs {
    /// Program name to install (binary name or identifier)
    pub program: Option<String>,

    /// Build the plan and print what would happen; do not execute
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the interactive confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Force a specific package manager (e.g. `brew`, `cargo`, `pnpm`)
    #[arg(long, value_name = "MANAGER")]
    pub via: Option<String>,

    /// Force the plan builder to treat sudo as unavailable
    #[arg(long)]
    pub no_sudo: bool,

    /// Bypass the host capability cache and rebuild it
    #[arg(short = 'f', long)]
    pub force: bool,
}

/// Discriminator returned by `Commands::install_command_args()` so dispatch
/// can tell `install` and `install-plan` apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCommandKind {
    Install,
    InstallPlan,
}

// ---------------------------------------------------------------------------
// Per-category action enums with shell completion candidates
// ---------------------------------------------------------------------------

macro_rules! define_program_action {
    ($action_name:ident, $candidates_fn:ident, $program_enum:ty) => {
        #[allow(dead_code)]
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
            Install(InstallCommandArgs),

            /// Show the install plan without executing anything
            #[command(name = "install-plan")]
            InstallPlan(InstallCommandArgs),
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
#[allow(dead_code)]
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
    Install(InstallCommandArgs),

    /// Show the install plan without executing anything
    #[command(name = "install-plan")]
    InstallPlan(InstallCommandArgs),
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

    /// Show only repository/monorepo structure
    Repo {
        /// Query package registries for latest dependency versions and report available updates
        #[arg(long)]
        latest_versions: bool,
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,

        #[command(subcommand)]
        repo_subcommand: Option<RepoSubcommand>,
    },

    /// Show broad file associations
    Files {
        /// Filter to a specific file association
        #[arg(long, value_enum)]
        association: Option<FileAssociationArg>,
    },

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

        /// Show only documents that have a blast_radius frontmatter key
        #[arg(long)]
        blast_radius: bool,

        /// Filter documents by substring match on filepath/filename
        filter: Vec<String>,
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
    AudioPlayers {
        #[command(subcommand)]
        action: Option<AudioAction>,
    },

    /// Show only AI agent/CLI tools
    Agents {
        #[command(subcommand)]
        action: Option<AgentAction>,
    },

    /// Show only desktop notification helpers
    NotificationHelpers,

    /// Show only system services (init system and service list)
    Services {
        /// Filter services by state
        #[arg(long, value_enum, default_value = "running")]
        state: ServiceStateArg,
    },

    /// Find documents whose blast_radius intersects with changed source files
    BlastRadius {
        /// Which changes to inspect (default: dirty)
        #[arg(value_enum, default_value_t = BlastRadiusScopeArg::Dirty)]
        scope: BlastRadiusScopeArg,

        /// Scope changed files to a specific package
        #[arg(long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,

        /// Scope changed files to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,

        /// Output as bullet list
        #[arg(long, conflicts_with = "csv")]
        list: bool,

        /// Output as comma-separated values
        #[arg(long, conflicts_with = "list")]
        csv: bool,

        /// Show only basename (hide directory path)
        #[arg(long)]
        no_path: bool,

        /// Exit 0 with no output when no results found
        #[arg(long)]
        no_error: bool,

        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE")]
        on_error: Option<String>,
    },

    /// Detect justfiles and their recipes
    Just {
        /// Filter justfiles by path substring (OR logic: matches any filter)
        filter: Vec<String>,
        /// Filter to justfiles containing a specific recipe
        #[arg(long, value_name = "RECIPE")]
        with: Option<String>,
        /// Group justfiles that share identical recipe bodies (requires --with)
        #[arg(long, requires = "with")]
        grouped: bool,
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
            Commands::Repo { .. } => OutputFilter::Repo,
            Commands::Files { .. } => OutputFilter::Files,
            Commands::Docs { .. } => OutputFilter::Docs,
            Commands::Programs { .. } => OutputFilter::Programs,
            Commands::Editors { .. } => OutputFilter::Editors,
            Commands::Utilities { .. } => OutputFilter::Utilities,
            Commands::LanguagePackageManagers { .. } => OutputFilter::LanguagePackageManagers,
            Commands::OsPackageManagers { .. } => OutputFilter::OsPackageManagers,
            Commands::TtsClients { .. } => OutputFilter::TtsClients,
            Commands::TerminalApps { .. } => OutputFilter::TerminalApps,
            Commands::AudioPlayers { .. } => OutputFilter::HeadlessAudio,
            Commands::Agents { .. } => OutputFilter::AiClients,
            Commands::NotificationHelpers => OutputFilter::NotificationHelpers,
            Commands::Services { .. } => OutputFilter::Services,
            Commands::BlastRadius { .. } => OutputFilter::BlastRadius,
            Commands::Just { .. } => OutputFilter::Just,
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
                | Commands::AudioPlayers { .. }
                | Commands::Agents { .. }
        )
    }

    /// Returns true if this command has a plain `install` action (not `install-plan`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_install_action(&self) -> bool {
        self.install_command_args()
            .map(|(k, _)| k == InstallCommandKind::Install)
            .unwrap_or(false)
    }

    /// Returns true if this command is an `install-plan` action.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_install_plan_action(&self) -> bool {
        self.install_command_args()
            .map(|(k, _)| k == InstallCommandKind::InstallPlan)
            .unwrap_or(false)
    }

    /// Returns the program name from an install-like action, if present.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn install_program_name(&self) -> Option<&str> {
        self.install_command_args()
            .and_then(|(_, args)| args.program.as_deref())
    }

    /// Returns the kind (install vs install-plan) and the shared args for
    /// install-like commands. Returns `None` for non-install commands.
    pub fn install_command_args(&self) -> Option<(InstallCommandKind, &InstallCommandArgs)> {
        use Commands::*;
        match self {
            Programs {
                action: Some(AllProgramAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            Programs {
                action: Some(AllProgramAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            Editors {
                action: Some(EditorAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            Editors {
                action: Some(EditorAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            Utilities {
                action: Some(UtilityAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            Utilities {
                action: Some(UtilityAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            LanguagePackageManagers {
                action: Some(LangPkgMgrAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            LanguagePackageManagers {
                action: Some(LangPkgMgrAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            OsPackageManagers {
                action: Some(OsPkgMgrAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            OsPackageManagers {
                action: Some(OsPkgMgrAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            TtsClients {
                action: Some(TtsClientAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            TtsClients {
                action: Some(TtsClientAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            TerminalApps {
                action: Some(TerminalAppAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            TerminalApps {
                action: Some(TerminalAppAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            AudioPlayers {
                action: Some(AudioAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            AudioPlayers {
                action: Some(AudioAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
            Agents {
                action: Some(AgentAction::Install(args)),
            } => Some((InstallCommandKind::Install, args)),
            Agents {
                action: Some(AgentAction::InstallPlan(args)),
            } => Some((InstallCommandKind::InstallPlan, args)),
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
            Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::GitStatus { history, .. }),
                ..
            } => *history,
            _ => DEFAULT_COMMIT_COUNT,
        }
    }

    /// Check if this command requests remote refresh data.
    pub fn refresh_remotes(&self) -> bool {
        matches!(
            self,
            Commands::Filesystem {
                refresh_remotes: true,
                ..
            } | Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::GitStatus {
                    refresh_remotes: true,
                    ..
                }),
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

    /// Get docs filter flags if this is a docs command.
    pub fn docs_filter(&self) -> DocsFilter {
        match self {
            Commands::Docs {
                readme,
                plan,
                src,
                has_prompt,
                blast_radius,
                filter,
            } => DocsFilter {
                readme: *readme,
                plan: *plan,
                src: *src,
                has_prompt: *has_prompt,
                blast_radius: *blast_radius,
                filter: filter.clone(),
            },
            _ => DocsFilter::default(),
        }
    }

    /// Get file-association filter if this is a files command.
    pub fn files_filter(&self) -> FilesFilter {
        match self {
            Commands::Files { association } => FilesFilter {
                association: association.map(Into::into),
            },
            _ => FilesFilter::default(),
        }
    }

    /// Normalize a Repo command into a RepoAction for dispatch.
    pub fn to_repo_action(&self) -> Option<RepoAction> {
        match self {
            Commands::Repo {
                latest_versions,
                filter,
                repo_subcommand,
            } => Some(match repo_subcommand {
                None => RepoAction::Structure {
                    filter: filter.clone(),
                    latest_versions: *latest_versions,
                },
                Some(RepoSubcommand::Structure { filter: sub_filter }) => RepoAction::Structure {
                    filter: if sub_filter.is_empty() {
                        filter.clone()
                    } else {
                        sub_filter.clone()
                    },
                    latest_versions: *latest_versions,
                },
                Some(RepoSubcommand::GitStatus {
                    history,
                    refresh_remotes,
                    compact,
                    package,
                }) => RepoAction::GitStatus {
                    history: *history,
                    refresh_remotes: *refresh_remotes,
                    compact: *compact,
                    package: package.clone(),
                },
                Some(RepoSubcommand::Hash { sha }) => RepoAction::Hash { sha: sha.clone() },
                Some(RepoSubcommand::StagedFiles(args)) => RepoAction::StagedFiles(args.clone()),
                Some(RepoSubcommand::UnstagedFiles { package }) => RepoAction::UnstagedFiles {
                    package: package.clone(),
                },
                Some(RepoSubcommand::UntrackedFiles { package }) => RepoAction::UntrackedFiles {
                    package: package.clone(),
                },
                Some(RepoSubcommand::Remote { remote }) => RepoAction::Remote {
                    remote: remote.clone(),
                },
                Some(RepoSubcommand::Deps {
                    ui,
                    filter: sub_filter,
                }) => RepoAction::Deps {
                    filter: if sub_filter.is_empty() {
                        filter.clone()
                    } else {
                        sub_filter.clone()
                    },
                    ui: *ui,
                },
                Some(RepoSubcommand::Packages {
                    filter: sub_filter,
                    package_area,
                    md,
                    list,
                }) => RepoAction::Packages {
                    filter: if sub_filter.is_empty() {
                        filter.clone()
                    } else {
                        sub_filter.clone()
                    },
                    package_area: package_area.clone(),
                    format: if *md {
                        PackagesFormat::Markdown
                    } else if *list {
                        PackagesFormat::List
                    } else {
                        PackagesFormat::Csv
                    },
                },
                Some(RepoSubcommand::PackageAreas {
                    filter: sub_filter,
                    package_area,
                    md,
                    list,
                }) => RepoAction::PackageAreas {
                    filter: if sub_filter.is_empty() {
                        filter.clone()
                    } else {
                        sub_filter.clone()
                    },
                    package_area: package_area.clone(),
                    format: if *md {
                        PackagesFormat::Markdown
                    } else if *list {
                        PackagesFormat::List
                    } else {
                        PackagesFormat::Csv
                    },
                },
                Some(RepoSubcommand::Package { no_error, on_error }) => RepoAction::Package {
                    no_error: *no_error,
                    on_error: on_error.clone(),
                },
                Some(RepoSubcommand::PackageArea { no_error, on_error }) => {
                    RepoAction::PackageArea {
                        no_error: *no_error,
                        on_error: on_error.clone(),
                    }
                }
                Some(RepoSubcommand::DirtyPackages { filter: sub_filter }) => {
                    RepoAction::DirtyPackages {
                        filter: if sub_filter.is_empty() {
                            filter.clone()
                        } else {
                            sub_filter.clone()
                        },
                    }
                }
                Some(RepoSubcommand::DirtyPackageAreas { filter: sub_filter }) => {
                    RepoAction::DirtyPackageAreas {
                        filter: if sub_filter.is_empty() {
                            filter.clone()
                        } else {
                            sub_filter.clone()
                        },
                    }
                }
                Some(RepoSubcommand::StagedPackages { filter: sub_filter }) => {
                    RepoAction::StagedPackages {
                        filter: if sub_filter.is_empty() {
                            filter.clone()
                        } else {
                            sub_filter.clone()
                        },
                    }
                }
                Some(RepoSubcommand::StagedPackageAreas { filter: sub_filter }) => {
                    RepoAction::StagedPackageAreas {
                        filter: if sub_filter.is_empty() {
                            filter.clone()
                        } else {
                            sub_filter.clone()
                        },
                    }
                }
                Some(RepoSubcommand::UnstagedPackages { filter: sub_filter }) => {
                    RepoAction::UnstagedPackages {
                        filter: if sub_filter.is_empty() {
                            filter.clone()
                        } else {
                            sub_filter.clone()
                        },
                    }
                }
                Some(RepoSubcommand::UnstagedPackageAreas { filter: sub_filter }) => {
                    RepoAction::UnstagedPackageAreas {
                        filter: if sub_filter.is_empty() {
                            filter.clone()
                        } else {
                            sub_filter.clone()
                        },
                    }
                }
                Some(RepoSubcommand::PackageRoot) => RepoAction::PackageRoot,
                Some(RepoSubcommand::PackageAreaRoot) => RepoAction::PackageAreaRoot,
                Some(RepoSubcommand::Root) => RepoAction::Root,
                Some(RepoSubcommand::IsCurrentPackageAreaDirty) => {
                    RepoAction::IsCurrentPackageAreaDirty
                }
                Some(RepoSubcommand::PackageAreaHasSourceCodeChanges) => {
                    RepoAction::PackageAreaHasSourceCodeChanges
                }
                Some(RepoSubcommand::HasMergeConflict) => RepoAction::HasMergeConflict,
                Some(RepoSubcommand::DirtySourceCode(args)) => {
                    RepoAction::DirtySourceCode(args.clone())
                }
                Some(RepoSubcommand::StagedSourceCode(args)) => {
                    RepoAction::StagedSourceCode(args.clone())
                }
                Some(RepoSubcommand::UnstagedSourceCode(args)) => {
                    RepoAction::UnstagedSourceCode(args.clone())
                }
                Some(RepoSubcommand::DirtyFiles(args)) => RepoAction::DirtyFiles(args.clone()),
                Some(RepoSubcommand::RecentCommits {
                    period,
                    actions,
                    package,
                    package_area,
                    no_error,
                    on_error,
                }) => RepoAction::RecentCommits {
                    period: period.clone(),
                    actions: actions.clone(),
                    package: package.clone(),
                    package_area: package_area.clone(),
                    no_error: *no_error,
                    on_error: on_error.clone(),
                },
                Some(RepoSubcommand::SourceCodeChanges {
                    period,
                    actions,
                    package,
                    package_area,
                    no_error,
                    on_error,
                }) => RepoAction::SourceCodeChanges {
                    period: period.clone(),
                    actions: actions.clone(),
                    package: package.clone(),
                    package_area: package_area.clone(),
                    no_error: *no_error,
                    on_error: on_error.clone(),
                },
                Some(RepoSubcommand::DocumentationChanges {
                    period,
                    actions,
                    package,
                    package_area,
                    no_error,
                    on_error,
                }) => RepoAction::DocumentationChanges {
                    period: period.clone(),
                    actions: actions.clone(),
                    package: package.clone(),
                    package_area: package_area.clone(),
                    no_error: *no_error,
                    on_error: on_error.clone(),
                },
                Some(RepoSubcommand::Pr { status }) => RepoAction::Pr {
                    status: *status,
                    verbose: false,
                },
                Some(RepoSubcommand::Language { breakdown }) => RepoAction::Language {
                    breakdown: *breakdown,
                },
            }),
            _ => None,
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
    /// Show only documents that have a blast_radius frontmatter key.
    pub blast_radius: bool,
    /// Substring filter on filepath/filename (case-insensitive).
    pub filter: Vec<String>,
}

/// Filter options for the files subcommand.
#[derive(Debug, Clone, Default)]
pub struct FilesFilter {
    pub association: Option<sniff::filesystem::FileAssociation>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FileAssociationArg {
    ProgrammingLanguage,
    FrameworkFile,
    Configuration,
    Styling,
    Documentation,
    Data,
    Image,
    Binary,
    BinaryExecutable,
    Archive,
    Font,
    Audio,
    Video,
    Unknown,
}

impl From<FileAssociationArg> for sniff::filesystem::FileAssociation {
    fn from(value: FileAssociationArg) -> Self {
        match value {
            FileAssociationArg::ProgrammingLanguage => Self::ProgrammingLanguage,
            FileAssociationArg::FrameworkFile => Self::FrameworkFile,
            FileAssociationArg::Configuration => Self::Configuration,
            FileAssociationArg::Styling => Self::Styling,
            FileAssociationArg::Documentation => Self::Documentation,
            FileAssociationArg::Data => Self::Data,
            FileAssociationArg::Image => Self::Image,
            FileAssociationArg::Binary => Self::Binary,
            FileAssociationArg::BinaryExecutable => Self::BinaryExecutable,
            FileAssociationArg::Archive => Self::Archive,
            FileAssociationArg::Font => Self::Font,
            FileAssociationArg::Audio => Self::Audio,
            FileAssociationArg::Video => Self::Video,
            FileAssociationArg::Unknown => Self::Unknown,
        }
    }
}

/// Service state filter for services subcommand.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum ServiceStateArg {
    All,
    #[default]
    Running,
    Stopped,
}

/// Conventional commit action filter for `repo recent-commits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RecentCommitActionArg {
    Feat,
    Chore,
    Refactor,
    Test,
    Style,
    Fix,
}

impl RecentCommitActionArg {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Feat => "feat",
            Self::Chore => "chore",
            Self::Refactor => "refactor",
            Self::Test => "test",
            Self::Style => "style",
            Self::Fix => "fix",
        }
    }
}

pub const HELP_TEMPLATE: &str = "\
{name} {version}
{about}

Usage: {usage}

{options}
{after-help}";

pub const AFTER_HELP: &str = "\
Commands:
  System:
    sniff os              Show OS information
    sniff hardware        Show hardware information
    sniff network         Show network information
    sniff cpu             Show CPU information
    sniff gpu             Show GPU information
    sniff memory          Show memory information
    sniff storage         Show storage/disk information
    sniff audio-devices   Show audio devices

  Repository & Filesystem:
    sniff repo            Show repository structure (use --help for all repo commands)
    sniff filesystem      Show full filesystem report
    sniff language        Show language detection
    sniff files           Show file associations
    sniff docs            Show markdown documents
    sniff blast-radius    Find docs affected by changed source files

  Programs:
    sniff programs                        Show all installed programs
    sniff programs install-plan <name>    Explain how a program would be installed
    sniff editors                         Show editors (supports 'install' and 'install-plan')
    sniff utilities                       Show utilities
    sniff agents                          Show AI agent CLI tools
    sniff notification-helpers            Show desktop notification helpers

  Services:
    sniff services        Show running services

  Justfiles:
    sniff just            Detect justfiles and recipes
    sniff just sniff      Filter to justfiles with \"sniff\" in their path
    sniff just --with build   Show justfiles containing a \"build\" recipe
    sniff just -v         Show justfiles with recipe details

  Discovery:
    sniff topics          Show all subsection topics

Output modes:
  No subcommand: show this help (use --json for full JSON)
  With subcommand: text by default, --json for JSON, --plain for unstyled text
";

pub const REPO_AFTER_HELP: &str = "\
Structure:
  sniff repo                          Show repository/monorepo structure
  sniff repo biscuit darkmatter       Filter to packages matching \"biscuit\" or \"darkmatter\"
  sniff repo structure @sniff         Filter to packages in \"sniff\" area

Git:
  sniff repo git-status               Show git status and recent commits
  sniff repo git-status --history 20  Show more commits
  sniff repo git-status --compact     Show only the Status section
  sniff repo hash HEAD                Show latest commit details
  sniff repo staged-files             List staged files
  sniff repo unstaged-files           List unstaged files
  sniff repo untracked-files          List untracked files
  sniff repo dirty-source-code        List dirty source code files
  sniff repo staged-source-code       List staged source code files
  sniff repo dirty-files              List all dirty files
  sniff repo remote origin            Inspect the 'origin' remote
  sniff repo pr                       List open pull requests
  sniff repo pr --status merged       List merged pull requests
  sniff repo pr -v                    Verbose PR block output

Recent Commits:
  sniff repo recent-commits           Show commits from last 3 days
  sniff repo recent-commits 1w        Show commits from last week
  sniff repo recent-commits 10        Show the last 10 commits
  sniff repo source-code-changes 1w   Source code changes in last week
  sniff repo documentation-changes 1w Documentation changes in last week

Packages:
  sniff repo packages                 List all package names
  sniff repo dirty-packages           Packages with uncommitted changes
  sniff repo package                  Package name for current directory

Languages:
  sniff repo language                 Primary programming language for the repository
  sniff repo language --json          Same, as { \"language\": \"Rust\" }

Dependencies:
  sniff repo deps                     Text dependency list
  sniff repo deps --ui                Mermaid dependency diagram
  sniff repo --latest-versions        Check registries for updates
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
                parse_args(&["files"]).unwrap().command,
                Some(Commands::Files { .. })
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
                parse_args(&["topics"]).unwrap().command,
                Some(Commands::Topics)
            ));
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
                assert_eq!(filter, vec!["research".to_string()]);
            } else {
                panic!("Expected Docs command");
            }
        }

        #[test]
        fn files_association_filter_parse() {
            let cli = parse_args(&["files", "--association", "documentation"]).unwrap();
            if let Some(Commands::Files {
                association: Some(FileAssociationArg::Documentation),
            }) = cli.command
            {
            } else {
                panic!("Expected Files command with documentation association");
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
                assert_eq!(filter, vec!["@sniff".to_string()]);
            } else {
                panic!("Expected Repo command");
            }
        }

        #[test]
        fn repo_subcommands_parse() {
            let cli = parse_args(&["repo", "deps"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Deps { ui: false, .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "deps", "--ui"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Deps { ui: true, .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "packages"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Packages { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "package-areas"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::PackageAreas { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "package"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Package { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "package-area"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::PackageArea { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "dirty-packages"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::DirtyPackages { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "dirty-package-areas"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::DirtyPackageAreas { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "staged-packages"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::StagedPackages { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "staged-package-areas"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::StagedPackageAreas { .. }),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "is-current-package-area-dirty"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::IsCurrentPackageAreaDirty),
                    ..
                })
            ));

            let cli = parse_args(&["repo", "package-area-has-source-code-changes"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::PackageAreaHasSourceCodeChanges),
                    ..
                })
            ));
        }

        #[test]
        fn repo_recent_commits_actions_parse() {
            let cli = parse_args(&[
                "repo",
                "recent-commits",
                "--action",
                "feat",
                "--action",
                "fix",
            ])
            .unwrap();

            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::RecentCommits { actions, .. }),
                    ..
                }) if actions == vec![RecentCommitActionArg::Feat, RecentCommitActionArg::Fix]
            ));
        }

        #[test]
        fn repo_source_code_changes_actions_parse() {
            let cli = parse_args(&[
                "repo",
                "source-code-changes",
                "--action",
                "feat",
                "--action",
                "fix",
            ])
            .unwrap();

            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand:
                        Some(RepoSubcommand::SourceCodeChanges { actions, .. }),
                    ..
                }) if actions == vec![RecentCommitActionArg::Feat, RecentCommitActionArg::Fix]
            ));
        }

        #[test]
        fn repo_documentation_changes_actions_parse() {
            let cli = parse_args(&["repo", "documentation-changes", "--action", "chore"]).unwrap();

            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand:
                        Some(RepoSubcommand::DocumentationChanges { actions, .. }),
                    ..
                }) if actions == vec![RecentCommitActionArg::Chore]
            ));
        }

        #[test]
        fn repo_pr_default_status_open() {
            let cli = parse_args(&["repo", "pr"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Pr {
                        status: sniff::remote::PullRequestState::Open,
                    }),
                    ..
                })
            ));
        }

        #[test]
        fn repo_pr_status_merged() {
            let cli = parse_args(&["repo", "pr", "--status", "merged"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Pr {
                        status: sniff::remote::PullRequestState::Merged,
                    }),
                    ..
                })
            ));
        }

        #[test]
        fn repo_pr_status_draft_with_json() {
            let cli = parse_args(&["repo", "pr", "--status", "draft", "--json"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Pr {
                        status: sniff::remote::PullRequestState::Draft,
                    }),
                    ..
                })
            ));
            assert!(cli.json);
        }

        #[test]
        fn repo_pr_verbose_global_flag() {
            let cli = parse_args(&["repo", "pr", "-v"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::Pr {
                        status: sniff::remote::PullRequestState::Open,
                    }),
                    ..
                })
            ));
            assert_eq!(cli.verbose, 1);
        }

        #[test]
        fn repo_pr_invalid_status_shows_valid_values() {
            match parse_args(&["repo", "pr", "--status", "invalid"]) {
                Ok(_) => panic!("Expected parsing to fail for invalid status"),
                Err(err) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("invalid PR state 'invalid'"),
                        "Error should mention invalid state: {msg}"
                    );
                    assert!(msg.contains("open"), "Error should mention 'open': {msg}");
                    assert!(
                        msg.contains("closed"),
                        "Error should mention 'closed': {msg}"
                    );
                    assert!(
                        msg.contains("merged"),
                        "Error should mention 'merged': {msg}"
                    );
                    assert!(msg.contains("draft"), "Error should mention 'draft': {msg}");
                    assert!(msg.contains("all"), "Error should mention 'all': {msg}");
                }
            }
        }

        #[test]
        fn editors_install_no_name_parses() {
            let cli = parse_args(&["editors", "install"]).unwrap();
            if let Some(Commands::Editors {
                action: Some(EditorAction::Install(args)),
            }) = cli.command
            {
                assert!(args.program.is_none());
            } else {
                panic!("Expected Editors install with no program");
            }
        }

        #[test]
        fn editors_install_with_name_parses() {
            let cli = parse_args(&["editors", "install", "vim"]).unwrap();
            if let Some(Commands::Editors {
                action: Some(EditorAction::Install(args)),
            }) = cli.command
            {
                assert_eq!(args.program.as_deref(), Some("vim"));
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
            if let Some(Commands::Programs {
                action: Some(AllProgramAction::Install(args)),
            }) = cli.command
            {
                assert!(args.program.is_none());
            } else {
                panic!("Expected Programs install with no program");
            }
        }

        #[test]
        fn programs_install_with_name_parses() {
            let cli = parse_args(&["programs", "install", "nvim"]).unwrap();
            if let Some(Commands::Programs {
                action: Some(AllProgramAction::Install(args)),
            }) = cli.command
            {
                assert_eq!(args.program.as_deref(), Some("nvim"));
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
                    latest_versions: false,
                    filter: vec![],
                    repo_subcommand: None,
                }
                .to_output_filter(),
                OutputFilter::Repo
            );
            assert_eq!(
                Commands::Files { association: None }.to_output_filter(),
                OutputFilter::Files
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
                action: Some(EditorAction::Install(InstallCommandArgs {
                    program: None,
                    ..Default::default()
                })),
            };
            assert!(cmd.is_install_action());

            let cmd = Commands::Programs {
                action: Some(AllProgramAction::Install(InstallCommandArgs {
                    program: Some("vim".to_string()),
                    ..Default::default()
                })),
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
                action: Some(EditorAction::Install(InstallCommandArgs {
                    program: Some("vim".to_string()),
                    ..Default::default()
                })),
            };
            assert_eq!(cmd.install_program_name(), Some("vim"));

            let cmd = Commands::Editors {
                action: Some(EditorAction::Install(InstallCommandArgs {
                    program: None,
                    ..Default::default()
                })),
            };
            assert_eq!(cmd.install_program_name(), None);

            let cmd = Commands::Editors { action: None };
            assert_eq!(cmd.install_program_name(), None);
        }

        #[test]
        fn repo_accessors_work() {
            let cmd = Commands::Repo {
                latest_versions: true,
                filter: vec!["biscuit".to_string()],
                repo_subcommand: None,
            };

            assert!(cmd.latest_versions());
            // Normalization captures filter
            if let Some(RepoAction::Structure {
                filter,
                latest_versions,
            }) = cmd.to_repo_action()
            {
                assert_eq!(filter, vec!["biscuit".to_string()]);
                assert!(latest_versions);
            } else {
                panic!("Expected Structure action");
            }

            // Subcommand filter takes precedence
            let cmd = Commands::Repo {
                latest_versions: false,
                filter: vec!["top-level".to_string()],
                repo_subcommand: Some(RepoSubcommand::Deps {
                    ui: true,
                    filter: vec!["sub-level".to_string()],
                }),
            };
            if let Some(RepoAction::Deps { filter, ui }) = cmd.to_repo_action() {
                assert_eq!(filter, vec!["sub-level".to_string()]);
                assert!(ui);
            } else {
                panic!("Expected Deps action");
            }

            // Falls back to top-level filter when subcommand has none
            let cmd = Commands::Repo {
                latest_versions: false,
                filter: vec!["top-level".to_string()],
                repo_subcommand: Some(RepoSubcommand::Deps {
                    ui: false,
                    filter: vec![],
                }),
            };
            if let Some(RepoAction::Deps { filter, .. }) = cmd.to_repo_action() {
                assert_eq!(filter, vec!["top-level".to_string()]);
            } else {
                panic!("Expected Deps action");
            }

            let cmd = Commands::Repo {
                latest_versions: false,
                filter: vec![],
                repo_subcommand: Some(RepoSubcommand::RecentCommits {
                    period: Some("1w".to_string()),
                    actions: vec![RecentCommitActionArg::Feat, RecentCommitActionArg::Fix],
                    package: None,
                    package_area: None,
                    no_error: false,
                    on_error: None,
                }),
            };
            if let Some(RepoAction::RecentCommits {
                period, actions, ..
            }) = cmd.to_repo_action()
            {
                assert_eq!(period.as_deref(), Some("1w"));
                assert_eq!(
                    actions,
                    vec![RecentCommitActionArg::Feat, RecentCommitActionArg::Fix]
                );
            } else {
                panic!("Expected RecentCommits action");
            }
        }

        #[test]
        fn docs_accessors_work() {
            let docs = Commands::Docs {
                readme: true,
                plan: false,
                src: true,
                has_prompt: false,
                blast_radius: false,
                filter: vec!["homelab".to_string()],
            };
            let filter = docs.docs_filter();
            assert!(filter.readme);
            assert!(filter.src);
            assert!(!filter.blast_radius);
            assert_eq!(filter.filter, vec!["homelab".to_string()]);
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
        fn base_flag_works_before_subcommand() {
            let cli = parse_args(&["-b", "/tmp", "filesystem"]).unwrap();
            assert_eq!(cli.base, Some(PathBuf::from("/tmp")));
            assert!(matches!(cli.command, Some(Commands::Filesystem { .. })));
        }

        #[test]
        fn scoped_flags_parse_for_supported_commands() {
            let filesystem =
                parse_args(&["filesystem", "--refresh-remotes", "--latest-versions"]).unwrap();
            assert!(matches!(
                filesystem.command,
                Some(Commands::Filesystem {
                    refresh_remotes: true,
                    latest_versions: true,
                })
            ));

            let git = parse_args(&["repo", "git-status", "--refresh-remotes"]).unwrap();
            assert!(git.command.as_ref().is_some_and(Commands::refresh_remotes));

            let repo = parse_args(&["repo", "--latest-versions"]).unwrap();
            assert!(repo.command.as_ref().is_some_and(Commands::latest_versions));
        }
    }

    mod new_repo_subcommands {
        use super::*;

        #[test]
        fn repo_structure_parses() {
            let cli = parse_args(&["repo", "structure"]).unwrap();
            if let Some(Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::Structure { filter }),
                ..
            }) = cli.command
            {
                assert!(filter.is_empty());
            } else {
                panic!("Expected repo structure");
            }
        }

        #[test]
        fn repo_structure_with_filter_parses() {
            let cli = parse_args(&["repo", "structure", "biscuit"]).unwrap();
            if let Some(Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::Structure { filter }),
                ..
            }) = cli.command
            {
                assert_eq!(filter, vec!["biscuit".to_string()]);
            } else {
                panic!("Expected repo structure with filter");
            }
        }

        #[test]
        fn repo_structure_with_multiple_filters_parses() {
            let cli = parse_args(&["repo", "structure", "biscuit", "sniff"]).unwrap();
            if let Some(Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::Structure { filter }),
                ..
            }) = cli.command
            {
                assert_eq!(filter, vec!["biscuit".to_string(), "sniff".to_string()]);
            } else {
                panic!("Expected repo structure with multiple filters");
            }
        }

        #[test]
        fn repo_git_status_parses() {
            let cli = parse_args(&["repo", "git-status"]).unwrap();
            if let Some(Commands::Repo {
                repo_subcommand:
                    Some(RepoSubcommand::GitStatus {
                        history,
                        refresh_remotes,
                        compact,
                        package,
                    }),
                ..
            }) = cli.command
            {
                assert_eq!(history, DEFAULT_COMMIT_COUNT);
                assert!(!refresh_remotes);
                assert!(!compact);
                assert!(package.is_none());
            } else {
                panic!("Expected repo git-status");
            }
        }

        #[test]
        fn repo_git_status_with_flags_parses() {
            let cli = parse_args(&[
                "repo",
                "git-status",
                "--history",
                "20",
                "--refresh-remotes",
                "--compact",
                "--package",
                "homelab",
            ])
            .unwrap();
            if let Some(Commands::Repo {
                repo_subcommand:
                    Some(RepoSubcommand::GitStatus {
                        history,
                        refresh_remotes,
                        compact,
                        package,
                    }),
                ..
            }) = cli.command
            {
                assert_eq!(history, 20);
                assert!(refresh_remotes);
                assert!(compact);
                assert_eq!(package.as_deref(), Some("homelab"));
            } else {
                panic!("Expected repo git-status with flags");
            }
        }

        #[test]
        fn repo_hash_parses() {
            let cli = parse_args(&["repo", "hash", "HEAD"]).unwrap();
            if let Some(Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::Hash { sha }),
                ..
            }) = cli.command
            {
                assert_eq!(sha, "HEAD");
            } else {
                panic!("Expected repo hash");
            }
        }

        #[test]
        fn repo_staged_files_parses() {
            let cli = parse_args(&["repo", "staged-files"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::StagedFiles(_)),
                    ..
                })
            ));
        }

        #[test]
        fn repo_unstaged_files_parses() {
            let cli = parse_args(&["repo", "unstaged-files"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::UnstagedFiles { package: None }),
                    ..
                })
            ));
        }

        #[test]
        fn repo_untracked_files_parses() {
            let cli = parse_args(&["repo", "untracked-files"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::UntrackedFiles { package: None }),
                    ..
                })
            ));
        }

        #[test]
        fn repo_remote_parses() {
            let cli = parse_args(&["repo", "remote", "origin"]).unwrap();
            if let Some(Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::Remote { remote }),
                ..
            }) = cli.command
            {
                assert_eq!(remote, "origin");
            } else {
                panic!("Expected repo remote");
            }
        }
    }

    mod repo_action_normalization {
        use super::*;

        #[test]
        fn to_repo_action_structure_default() {
            let cmd = Commands::Repo {
                latest_versions: true,
                filter: vec!["biscuit".to_string()],
                repo_subcommand: None,
            };
            match cmd.to_repo_action() {
                Some(RepoAction::Structure {
                    filter,
                    latest_versions,
                }) => {
                    assert_eq!(filter, vec!["biscuit".to_string()]);
                    assert!(latest_versions);
                }
                _ => panic!("Expected Structure action"),
            }
        }

        #[test]
        fn to_repo_action_git_status() {
            let cmd = Commands::Repo {
                latest_versions: false,
                filter: vec![],
                repo_subcommand: Some(RepoSubcommand::GitStatus {
                    history: 25,
                    refresh_remotes: true,
                    compact: true,
                    package: Some("homelab".to_string()),
                }),
            };
            match cmd.to_repo_action() {
                Some(RepoAction::GitStatus {
                    history,
                    refresh_remotes,
                    compact,
                    package,
                }) => {
                    assert_eq!(history, 25);
                    assert!(refresh_remotes);
                    assert!(compact);
                    assert_eq!(package.as_deref(), Some("homelab"));
                }
                _ => panic!("Expected GitStatus action"),
            }
        }

        #[test]
        fn to_repo_action_pr() {
            let cmd = Commands::Repo {
                latest_versions: false,
                filter: vec![],
                repo_subcommand: Some(RepoSubcommand::Pr {
                    status: sniff::remote::PullRequestState::Merged,
                }),
            };
            match cmd.to_repo_action() {
                Some(RepoAction::Pr { status, verbose }) => {
                    assert_eq!(status, sniff::remote::PullRequestState::Merged);
                    assert!(!verbose);
                }
                _ => panic!("Expected Pr action"),
            }
        }
    }

    mod blast_radius_commands {
        use super::*;

        #[test]
        fn blast_radius_default_scope() {
            let cli = parse_args(&["blast-radius"]).unwrap();
            if let Some(Commands::BlastRadius { scope, .. }) = cli.command {
                assert!(matches!(scope, BlastRadiusScopeArg::Dirty));
            } else {
                panic!("Expected BlastRadius command");
            }
        }

        #[test]
        fn blast_radius_staged_scope() {
            let cli = parse_args(&["blast-radius", "staged"]).unwrap();
            if let Some(Commands::BlastRadius { scope, .. }) = cli.command {
                assert!(matches!(scope, BlastRadiusScopeArg::Staged));
            } else {
                panic!("Expected BlastRadius staged");
            }
        }

        #[test]
        fn blast_radius_last_commit_scope() {
            let cli = parse_args(&["blast-radius", "last-commit"]).unwrap();
            if let Some(Commands::BlastRadius { scope, .. }) = cli.command {
                assert!(matches!(scope, BlastRadiusScopeArg::LastCommit));
            } else {
                panic!("Expected BlastRadius last-commit");
            }
        }

        #[test]
        fn blast_radius_with_package() {
            let cli = parse_args(&["blast-radius", "--package", "sniff"]).unwrap();
            if let Some(Commands::BlastRadius { package, .. }) = cli.command {
                assert_eq!(package.as_deref(), Some("sniff"));
            } else {
                panic!("Expected BlastRadius with package");
            }
        }

        #[test]
        fn blast_radius_list_csv_conflict() {
            let result = parse_args(&["blast-radius", "--list", "--csv"]);
            assert!(result.is_err(), "--list and --csv should conflict");
        }

        #[test]
        fn docs_blast_radius_flag_parses() {
            let cli = parse_args(&["docs", "--blast-radius"]).unwrap();
            if let Some(Commands::Docs { blast_radius, .. }) = cli.command {
                assert!(blast_radius);
            } else {
                panic!("Expected Docs with blast_radius flag");
            }
        }

        #[test]
        fn repo_dirty_source_code_parses() {
            let cli = parse_args(&["repo", "dirty-source-code"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::DirtySourceCode(_)),
                    ..
                })
            ));
        }

        #[test]
        fn repo_staged_source_code_parses() {
            let cli = parse_args(&["repo", "staged-source-code"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::StagedSourceCode(_)),
                    ..
                })
            ));
        }

        #[test]
        fn repo_unstaged_source_code_parses() {
            let cli = parse_args(&["repo", "unstaged-source-code"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::UnstagedSourceCode(_)),
                    ..
                })
            ));
        }

        #[test]
        fn repo_dirty_files_parses() {
            let cli = parse_args(&["repo", "dirty-files"]).unwrap();
            assert!(matches!(
                cli.command,
                Some(Commands::Repo {
                    repo_subcommand: Some(RepoSubcommand::DirtyFiles(_)),
                    ..
                })
            ));
        }

        #[test]
        fn staged_files_list_csv_conflict() {
            let result = parse_args(&["repo", "staged-files", "--list", "--csv"]);
            assert!(result.is_err(), "--list and --csv should conflict");
        }

        #[test]
        fn staged_files_list_flag_parses() {
            let cli = parse_args(&["repo", "staged-files", "--list"]).unwrap();
            if let Some(Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::StagedFiles(args)),
                ..
            }) = cli.command
            {
                assert!(args.list);
                assert!(!args.csv);
            } else {
                panic!("Expected StagedFiles with --list");
            }
        }

        #[test]
        fn no_error_and_on_error_coexist() {
            let cli = parse_args(&[
                "repo",
                "dirty-source-code",
                "--no-error",
                "--on-error",
                "No changes found",
            ])
            .unwrap();
            if let Some(Commands::Repo {
                repo_subcommand: Some(RepoSubcommand::DirtySourceCode(args)),
                ..
            }) = cli.command
            {
                assert!(args.no_error);
                assert_eq!(args.on_error.as_deref(), Some("No changes found"));
            } else {
                panic!("Expected DirtySourceCode with no_error + on_error");
            }
        }
    }

    mod install_command_args {
        use super::*;

        #[test]
        fn editors_install_parses_with_dry_run_and_yes() {
            let cli = parse_args(&["editors", "install", "vim", "--dry-run", "-y"]).unwrap();
            if let Some(Commands::Editors {
                action: Some(EditorAction::Install(args)),
            }) = cli.command
            {
                assert_eq!(args.program.as_deref(), Some("vim"));
                assert!(args.dry_run);
                assert!(args.yes);
                assert!(!args.no_sudo);
                assert!(!args.force);
                assert!(args.via.is_none());
            } else {
                panic!("Expected Editors install with InstallCommandArgs");
            }
        }

        #[test]
        fn editors_install_plan_parses() {
            let cli = parse_args(&["editors", "install-plan", "vim"]).unwrap();
            assert!(cli.command.as_ref().unwrap().is_install_plan_action());
            assert_eq!(
                cli.command.as_ref().unwrap().install_program_name(),
                Some("vim")
            );
        }

        #[test]
        fn editors_install_via_and_no_sudo_parse() {
            let cli = parse_args(&[
                "editors",
                "install",
                "vim",
                "--via",
                "brew",
                "--no-sudo",
                "--force",
            ])
            .unwrap();
            if let Some(Commands::Editors {
                action: Some(EditorAction::Install(args)),
            }) = cli.command
            {
                assert_eq!(args.via.as_deref(), Some("brew"));
                assert!(args.no_sudo);
                assert!(args.force);
            } else {
                panic!("Expected editors install with via+no_sudo+force");
            }
        }
    }
}
