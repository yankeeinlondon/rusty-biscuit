use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use clap_complete::Shell;
use sniff::package::enrich_dependencies;
use sniff::programs::ProgramsInfo;
use sniff::services::{ServiceState, detect_services};
use sniff::{SniffConfig, SniffResult, detect_with_config};
use std::path::PathBuf;

mod output;
use output::OutputFilter;

/// Detect system and repository information
#[derive(Parser)]
#[command(
    name = "sniff",
    version,
    about,
    after_help = AFTER_HELP,
    help_template = HELP_TEMPLATE
)]
struct Cli {
    /// Base directory for filesystem analysis
    #[arg(short, long, global = true)]
    base: Option<PathBuf>,

    /// Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
    #[arg(long, global = true)]
    json: bool,

    /// Enable deep git inspection (queries remotes for branch info)
    #[arg(long, global = true)]
    deep: bool,

    /// Increase output verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL", hide = true)]
    completions: Option<Shell>,

    /// Subcommand to filter output to a specific section
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Subcommands for filtering output to specific sections.
///
/// Each subcommand shows only the specified section of system information.
/// Without a subcommand, all data is output as JSON.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    // === Top-level sections ===
    /// Show only OS information (name, kernel, locale, timezone)
    Os,

    /// Show only hardware information (CPU, GPU, memory, storage)
    Hardware,

    /// Show only network information (interfaces, IP addresses)
    Network,

    /// Show only filesystem information (git, languages, monorepo)
    Filesystem,

    // === Hardware detail sections ===
    /// Show only CPU information
    Cpu,

    /// Show only GPU information
    Gpu,

    /// Show only memory information
    Memory,

    /// Show only storage/disk information
    Storage,

    // === Filesystem detail sections ===
    /// Show only git repository information
    #[command(disable_help_flag = true)]
    Git {
        /// Number of recent commits to display (default: 5)
        #[arg(short = 'h', long, default_value = "5")]
        history: usize,
    },

    /// Show only repository/monorepo structure
    Repo,

    /// Show only language detection results
    Language,

    // === Programs sections ===
    /// Show all installed programs detection
    Programs {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only installed editors
    Editors {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only installed utilities
    Utilities {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only language package managers
    LanguagePackageManagers {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only OS package managers
    OsPackageManagers {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only TTS clients
    TtsClients {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only terminal apps
    TerminalApps {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only headless audio players
    Audio {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    /// Show only AI agent/CLI tools
    Agents {
        /// JSON output format: "simple" (default) or "full" (rich metadata)
        #[arg(long, value_name = "FORMAT")]
        json_format: Option<String>,
    },

    // === Services section ===
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
            // Top-level sections
            Commands::Os => OutputFilter::Os,
            Commands::Hardware => OutputFilter::Hardware,
            Commands::Network => OutputFilter::Network,
            Commands::Filesystem => OutputFilter::Filesystem,

            // Hardware detail sections
            Commands::Cpu => OutputFilter::Cpu,
            Commands::Gpu => OutputFilter::Gpu,
            Commands::Memory => OutputFilter::Memory,
            Commands::Storage => OutputFilter::Storage,

            // Filesystem detail sections
            Commands::Git { .. } => OutputFilter::Git,
            Commands::Repo => OutputFilter::Repo,
            Commands::Language => OutputFilter::Language,

            // Programs sections
            Commands::Programs { .. } => OutputFilter::Programs,
            Commands::Editors { .. } => OutputFilter::Editors,
            Commands::Utilities { .. } => OutputFilter::Utilities,
            Commands::LanguagePackageManagers { .. } => OutputFilter::LanguagePackageManagers,
            Commands::OsPackageManagers { .. } => OutputFilter::OsPackageManagers,
            Commands::TtsClients { .. } => OutputFilter::TtsClients,
            Commands::TerminalApps { .. } => OutputFilter::TerminalApps,
            Commands::Audio { .. } => OutputFilter::HeadlessAudio,
            Commands::Agents { .. } => OutputFilter::AiClients,

            // Services section
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

    /// Get json_format if this is a programs command.
    pub fn json_format(&self) -> Option<&str> {
        match self {
            Commands::Programs { json_format, .. }
            | Commands::Editors { json_format, .. }
            | Commands::Utilities { json_format, .. }
            | Commands::LanguagePackageManagers { json_format, .. }
            | Commands::OsPackageManagers { json_format, .. }
            | Commands::TtsClients { json_format, .. }
            | Commands::TerminalApps { json_format, .. }
            | Commands::Audio { json_format, .. }
            | Commands::Agents { json_format, .. } => json_format.as_deref(),
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
            Commands::Git { history } => *history,
            _ => 5, // default
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

const HELP_TEMPLATE: &str = "\
{name} {version}
{about}

Usage: {usage}

{options}
{after-help}";

const AFTER_HELP: &str = "\
Commands:
  Top-level sections:
    sniff os          Show only OS information
    sniff hardware    Show only hardware information
    sniff network     Show only network information
    sniff filesystem  Show only filesystem information

  Hardware details:
    sniff cpu         Show only CPU information
    sniff gpu         Show only GPU information
    sniff memory      Show only memory information
    sniff storage     Show only storage/disk information

  Filesystem details:
    sniff git         Show only git repository information
    sniff repo        Show only repository/monorepo structure
    sniff language    Show only language detection results

  Programs:
    sniff programs                   Show all installed programs
    sniff editors                    Show only installed editors
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
  - No subcommand: JSON output (all data)
  - With subcommand: Text output by default, use --json for JSON

Examples:
  sniff                      # Full system info as JSON
  sniff cpu                  # CPU info as text
  sniff cpu --json           # CPU info as JSON
  sniff --json cpu           # Same as above (flag position flexible)
  sniff programs             # Programs as text
  sniff programs --json      # Programs as JSON
  sniff -b /path/to/repo filesystem  # Analyze specific directory
";

const COMPLETIONS_HELP: &str = "\
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle dynamic shell completions (invoked by shell completion scripts)
    CompleteEnv::with_factory(Cli::command).complete();

    if wants_completions_help() {
        print_completions_help();
        return Ok(());
    }

    let cli = Cli::parse();

    // Handle --completions first (prints setup instructions)
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    // Determine output filter based on subcommand
    let output_filter = cli
        .command
        .as_ref()
        .map(Commands::to_output_filter)
        .unwrap_or(OutputFilter::All);

    // Handle programs mode separately (doesn't use SniffResult)
    if let Some(ref cmd) = cli.command {
        if cmd.is_programs_mode() {
            let programs = ProgramsInfo::detect();
            if cli.json {
                let format = cmd.json_format().unwrap_or("simple");
                output::print_programs_json(&programs, output_filter, format)?;
            } else {
                // Default: text output (using markdown renderer for now)
                output::print_programs_markdown(&programs, cli.verbose, output_filter);
            }
            return Ok(());
        }

        // Handle services mode separately (doesn't use SniffResult)
        if let Some(state_arg) = cmd.state() {
            let services_info = detect_services();
            let state_filter = match state_arg {
                ServiceStateArg::All => ServiceState::All,
                ServiceStateArg::Running => ServiceState::Running,
                ServiceStateArg::Stopped => ServiceState::Stopped,
            };
            if cli.json {
                output::print_services_json(&services_info, state_filter)?;
            } else {
                output::print_services_text(&services_info, cli.verbose, state_filter);
            }
            return Ok(());
        }
    }

    // Canonicalize path if provided
    let base_dir = cli
        .base
        .clone()
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p));

    let mut config = SniffConfig::new();

    if let Some(base) = base_dir {
        config = config.base_dir(base);
    }

    let deep_enabled = cli.deep;
    if deep_enabled {
        config = config.deep(true);
    }

    // Set commit count from history flag (or default 5)
    let history_count = cli.command.as_ref().map_or(5, |c| c.history());
    config = config.commit_count(history_count);

    // Apply skip logic based on filter mode
    match output_filter {
        // Top-level section filters: skip all OTHER sections
        OutputFilter::Os => {
            config = config.skip_hardware().skip_network().skip_filesystem();
        }
        OutputFilter::Hardware => {
            config = config.skip_os().skip_network().skip_filesystem();
        }
        OutputFilter::Network => {
            config = config.skip_os().skip_hardware().skip_filesystem();
        }
        OutputFilter::Filesystem => {
            config = config.skip_os().skip_hardware().skip_network();
        }
        // Hardware detail filters: show only hardware section
        OutputFilter::Cpu | OutputFilter::Gpu | OutputFilter::Memory | OutputFilter::Storage => {
            config = config.skip_os().skip_network().skip_filesystem();
        }
        // Filesystem detail filters: show only filesystem section
        OutputFilter::Git | OutputFilter::Repo | OutputFilter::Language => {
            config = config.skip_os().skip_hardware().skip_network();
        }
        // All: no subcommand means full detection
        OutputFilter::All => {
            // No filtering - detect everything
        }
        // Programs and Services filters are handled earlier in main, should not reach here
        OutputFilter::Programs
        | OutputFilter::Editors
        | OutputFilter::Utilities
        | OutputFilter::LanguagePackageManagers
        | OutputFilter::OsPackageManagers
        | OutputFilter::TtsClients
        | OutputFilter::TerminalApps
        | OutputFilter::HeadlessAudio
        | OutputFilter::AiClients
        | OutputFilter::Services => {
            unreachable!("Programs and Services mode should be handled before this point")
        }
    }

    let mut result = detect_with_config(config)?;

    // Enrich dependencies with latest versions when --deep is enabled
    if deep_enabled {
        result = enrich_result_dependencies(result).await;
    }

    // Output logic:
    // - No subcommand: always JSON
    // - With subcommand: text by default, --json for JSON
    let use_json = cli.command.is_none() || cli.json;

    if use_json {
        output::print_json(&result, output_filter)?;
    } else {
        output::print_text(&result, cli.verbose, output_filter, history_count);
    }

    Ok(())
}

/// Prints shell completions setup instructions.
///
/// With dynamic completions, the shell sources a command that calls back to the CLI.
/// This outputs the appropriate setup command for each shell.
fn print_completions(shell: Shell) {
    let (setup_cmd, config_file) = match shell {
        Shell::Bash => ("source <(COMPLETE=bash sniff)", "~/.bashrc"),
        Shell::Zsh => ("source <(COMPLETE=zsh sniff)", "~/.zshrc"),
        Shell::Fish => ("COMPLETE=fish sniff | source", "~/.config/fish/config.fish"),
        Shell::PowerShell => (
            r#"$env:COMPLETE = "powershell"; sniff | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE"#,
            "$PROFILE",
        ),
        Shell::Elvish => ("eval (E:COMPLETE=elvish sniff | slurp)", "~/.elvish/rc.elv"),
        _ => {
            eprintln!("Shell {:?} is not supported for dynamic completions", shell);
            return;
        }
    };

    println!("# Add this line to {}:", config_file);
    println!("{}", setup_cmd);
}

fn wants_completions_help() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();

    for (index, arg) in args.iter().enumerate() {
        if arg == "--completions" {
            if let Some(next) = args.get(index + 1) {
                return next == "--help" || next == "-h";
            }
        }
    }

    false
}

fn print_completions_help() {
    println!("{}", COMPLETIONS_HELP);
}

/// Enriches all dependencies in a SniffResult with latest versions from package registries.
///
/// This function iterates through the filesystem.repo section and enriches:
/// - Non-monorepo dependencies (on RepoInfo)
/// - Monorepo package dependencies (on each PackageLocation)
async fn enrich_result_dependencies(mut result: SniffResult) -> SniffResult {
    let Some(ref mut filesystem) = result.filesystem else {
        return result;
    };

    let Some(ref mut repo) = filesystem.repo else {
        return result;
    };

    // Enrich non-monorepo dependencies
    if let Some(deps) = repo.dependencies.take() {
        repo.dependencies = Some(enrich_dependencies(deps).await);
    }
    if let Some(deps) = repo.dev_dependencies.take() {
        repo.dev_dependencies = Some(enrich_dependencies(deps).await);
    }
    if let Some(deps) = repo.peer_dependencies.take() {
        repo.peer_dependencies = Some(enrich_dependencies(deps).await);
    }
    if let Some(deps) = repo.optional_dependencies.take() {
        repo.optional_dependencies = Some(enrich_dependencies(deps).await);
    }

    // Enrich monorepo package dependencies
    if let Some(ref mut packages) = repo.packages {
        for pkg in packages.iter_mut() {
            if let Some(deps) = pkg.dependencies.take() {
                pkg.dependencies = Some(enrich_dependencies(deps).await);
            }
            if let Some(deps) = pkg.dev_dependencies.take() {
                pkg.dev_dependencies = Some(enrich_dependencies(deps).await);
            }
            if let Some(deps) = pkg.peer_dependencies.take() {
                pkg.peer_dependencies = Some(enrich_dependencies(deps).await);
            }
            if let Some(deps) = pkg.optional_dependencies.take() {
                pkg.optional_dependencies = Some(enrich_dependencies(deps).await);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Helper to create a Cli struct from args (for testing).
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
        fn os_subcommand_parses() {
            let cli = parse_args(&["os"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Os)));
        }

        #[test]
        fn cpu_subcommand_parses() {
            let cli = parse_args(&["cpu"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Cpu)));
        }

        #[test]
        fn gpu_subcommand_parses() {
            let cli = parse_args(&["gpu"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Gpu)));
        }

        #[test]
        fn memory_subcommand_parses() {
            let cli = parse_args(&["memory"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Memory)));
        }

        #[test]
        fn storage_subcommand_parses() {
            let cli = parse_args(&["storage"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Storage)));
        }

        #[test]
        fn git_subcommand_parses() {
            let cli = parse_args(&["git"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Git { .. })));
        }

        #[test]
        fn git_subcommand_with_history_flag() {
            let cli = parse_args(&["git", "--history", "10"]).unwrap();
            if let Some(Commands::Git { history }) = cli.command {
                assert_eq!(history, 10);
            } else {
                panic!("Expected Git command");
            }
        }

        #[test]
        fn git_subcommand_with_short_history_flag() {
            let cli = parse_args(&["git", "-h", "3"]).unwrap();
            if let Some(Commands::Git { history }) = cli.command {
                assert_eq!(history, 3);
            } else {
                panic!("Expected Git command");
            }
        }

        #[test]
        fn git_subcommand_default_history() {
            let cli = parse_args(&["git"]).unwrap();
            if let Some(Commands::Git { history }) = cli.command {
                assert_eq!(history, 5);
            } else {
                panic!("Expected Git command");
            }
        }

        #[test]
        fn repo_subcommand_parses() {
            let cli = parse_args(&["repo"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Repo)));
        }

        #[test]
        fn language_subcommand_parses() {
            let cli = parse_args(&["language"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Language)));
        }

        #[test]
        fn filesystem_subcommand_parses() {
            let cli = parse_args(&["filesystem"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Filesystem)));
        }

        #[test]
        fn hardware_subcommand_parses() {
            let cli = parse_args(&["hardware"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Hardware)));
        }

        #[test]
        fn network_subcommand_parses() {
            let cli = parse_args(&["network"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Network)));
        }

        #[test]
        fn audio_subcommand_parses() {
            let cli = parse_args(&["audio"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Audio { .. })));
        }

        #[test]
        fn programs_subcommand_parses() {
            let cli = parse_args(&["programs"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Programs { .. })));
        }

        #[test]
        fn services_subcommand_parses() {
            let cli = parse_args(&["services"]).unwrap();
            assert!(matches!(cli.command, Some(Commands::Services { .. })));
        }
    }

    mod to_output_filter {
        use super::*;

        #[test]
        fn os_maps_to_os_filter() {
            let cmd = Commands::Os;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Os);
        }

        #[test]
        fn hardware_maps_to_hardware_filter() {
            let cmd = Commands::Hardware;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Hardware);
        }

        #[test]
        fn network_maps_to_network_filter() {
            let cmd = Commands::Network;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Network);
        }

        #[test]
        fn filesystem_maps_to_filesystem_filter() {
            let cmd = Commands::Filesystem;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Filesystem);
        }

        #[test]
        fn cpu_maps_to_cpu_filter() {
            let cmd = Commands::Cpu;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Cpu);
        }

        #[test]
        fn gpu_maps_to_gpu_filter() {
            let cmd = Commands::Gpu;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Gpu);
        }

        #[test]
        fn memory_maps_to_memory_filter() {
            let cmd = Commands::Memory;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Memory);
        }

        #[test]
        fn storage_maps_to_storage_filter() {
            let cmd = Commands::Storage;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Storage);
        }

        #[test]
        fn git_maps_to_git_filter() {
            let cmd = Commands::Git { history: 5 };
            assert_eq!(cmd.to_output_filter(), OutputFilter::Git);
        }

        #[test]
        fn repo_maps_to_repo_filter() {
            let cmd = Commands::Repo;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Repo);
        }

        #[test]
        fn language_maps_to_language_filter() {
            let cmd = Commands::Language;
            assert_eq!(cmd.to_output_filter(), OutputFilter::Language);
        }

        #[test]
        fn programs_maps_to_programs_filter() {
            let cmd = Commands::Programs {
                json_format: None,
            };
            assert_eq!(cmd.to_output_filter(), OutputFilter::Programs);
        }

        #[test]
        fn editors_maps_to_editors_filter() {
            let cmd = Commands::Editors {
                json_format: None,
            };
            assert_eq!(cmd.to_output_filter(), OutputFilter::Editors);
        }

        #[test]
        fn audio_maps_to_headless_audio_filter() {
            let cmd = Commands::Audio {
                json_format: None,
            };
            assert_eq!(cmd.to_output_filter(), OutputFilter::HeadlessAudio);
        }

        #[test]
        fn services_maps_to_services_filter() {
            let cmd = Commands::Services {
                state: ServiceStateArg::Running,
            };
            assert_eq!(cmd.to_output_filter(), OutputFilter::Services);
        }
    }

    mod services_state_default {
        use super::*;

        #[test]
        fn services_defaults_to_running() {
            let cli = parse_args(&["services"]).unwrap();
            if let Some(Commands::Services { state }) = cli.command {
                assert!(matches!(state, ServiceStateArg::Running));
            } else {
                panic!("Expected Services command");
            }
        }

        #[test]
        fn services_accepts_all_state() {
            let cli = parse_args(&["services", "--state", "all"]).unwrap();
            if let Some(Commands::Services { state }) = cli.command {
                assert!(matches!(state, ServiceStateArg::All));
            } else {
                panic!("Expected Services command");
            }
        }

        #[test]
        fn services_accepts_stopped_state() {
            let cli = parse_args(&["services", "--state", "stopped"]).unwrap();
            if let Some(Commands::Services { state }) = cli.command {
                assert!(matches!(state, ServiceStateArg::Stopped));
            } else {
                panic!("Expected Services command");
            }
        }
    }

    mod global_flags {
        use super::*;

        #[test]
        fn json_flag_before_subcommand() {
            let cli = parse_args(&["--json", "cpu"]).unwrap();
            assert!(cli.json);
            assert!(matches!(cli.command, Some(Commands::Cpu)));
        }

        #[test]
        fn json_flag_after_subcommand() {
            let cli = parse_args(&["cpu", "--json"]).unwrap();
            assert!(cli.json);
            assert!(matches!(cli.command, Some(Commands::Cpu)));
        }

        #[test]
        fn verbose_flag_before_subcommand() {
            let cli = parse_args(&["-v", "cpu"]).unwrap();
            assert_eq!(cli.verbose, 1);
            assert!(matches!(cli.command, Some(Commands::Cpu)));
        }

        #[test]
        fn verbose_flag_after_subcommand() {
            let cli = parse_args(&["cpu", "-v"]).unwrap();
            assert_eq!(cli.verbose, 1);
            assert!(matches!(cli.command, Some(Commands::Cpu)));
        }

        #[test]
        fn deep_flag_works_globally() {
            let cli = parse_args(&["--deep", "git"]).unwrap();
            assert!(cli.deep);
            assert!(matches!(cli.command, Some(Commands::Git { .. })));
        }

        #[test]
        fn base_flag_works_globally() {
            let cli = parse_args(&["-b", "/tmp", "filesystem"]).unwrap();
            assert_eq!(cli.base, Some(PathBuf::from("/tmp")));
            assert!(matches!(cli.command, Some(Commands::Filesystem)));
        }

        #[test]
        fn multiple_verbose_flags() {
            let cli = parse_args(&["-vvv", "cpu"]).unwrap();
            assert_eq!(cli.verbose, 3);
        }
    }

    mod programs_flags {
        use super::*;

        #[test]
        fn programs_json_format_flag() {
            let cli = parse_args(&["programs", "--json-format", "full"]).unwrap();
            if let Some(Commands::Programs { json_format, .. }) = cli.command {
                assert_eq!(json_format, Some("full".to_string()));
            } else {
                panic!("Expected Programs command");
            }
        }
    }

    mod is_programs_mode {
        use super::*;

        #[test]
        fn programs_is_programs_mode() {
            let cmd = Commands::Programs {
                json_format: None,
            };
            assert!(cmd.is_programs_mode());
        }

        #[test]
        fn editors_is_programs_mode() {
            let cmd = Commands::Editors {
                json_format: None,
            };
            assert!(cmd.is_programs_mode());
        }

        #[test]
        fn audio_is_programs_mode() {
            let cmd = Commands::Audio {
                json_format: None,
            };
            assert!(cmd.is_programs_mode());
        }

        #[test]
        fn cpu_is_not_programs_mode() {
            let cmd = Commands::Cpu;
            assert!(!cmd.is_programs_mode());
        }

        #[test]
        fn services_is_not_programs_mode() {
            let cmd = Commands::Services {
                state: ServiceStateArg::Running,
            };
            assert!(!cmd.is_programs_mode());
        }
    }
}
