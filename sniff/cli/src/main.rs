use clap::Parser;
use sniff_lib::package::enrich_dependencies;
use sniff_lib::{detect_with_config, SniffConfig, SniffResult};
use std::path::PathBuf;

mod output;
use output::OutputFilter;

/// Detect system and repository information
#[derive(Parser)]
#[command(name = "sniff", version, about, after_help = AFTER_HELP)]
struct Cli {
    /// Base directory for filesystem analysis
    #[arg(short, long)]
    base: Option<PathBuf>,

    /// Output as JSON instead of text
    #[arg(long)]
    json: bool,

    /// Increase output verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    // === Skip flags (default behavior) ===
    /// Skip hardware detection
    #[arg(long)]
    skip_hardware: bool,

    /// Skip network detection
    #[arg(long)]
    skip_network: bool,

    /// Skip filesystem detection
    #[arg(long)]
    skip_filesystem: bool,

    // === Include-only flags ===
    /// Include ONLY hardware section (enables include-only mode)
    #[arg(long)]
    hardware: bool,

    /// Include ONLY network section (enables include-only mode)
    #[arg(long)]
    network: bool,

    /// Include ONLY filesystem section (enables include-only mode)
    #[arg(long)]
    filesystem: bool,

    /// Enable deep git inspection (queries remotes for branch info)
    #[arg(long)]
    deep: bool,

    // === Filter flags (mutually exclusive) ===
    /// Show only OS information
    #[arg(long, help_heading = "Filter Flags")]
    os: bool,

    /// Show only git repository information
    #[arg(long, help_heading = "Filter Flags")]
    git: bool,

    /// Show only repository/monorepo structure
    #[arg(long, help_heading = "Filter Flags")]
    repo: bool,

    /// Show only language detection
    #[arg(long, help_heading = "Filter Flags")]
    language: bool,

    /// Show only CPU information
    #[arg(long, help_heading = "Filter Flags")]
    cpu: bool,

    /// Show only GPU information
    #[arg(long, help_heading = "Filter Flags")]
    gpu: bool,

    /// Show only memory information
    #[arg(long, help_heading = "Filter Flags")]
    memory: bool,

    /// Show only storage information
    #[arg(long, help_heading = "Filter Flags")]
    storage: bool,
}

impl Cli {
    /// Collect all active filter flags into a vector of flag names.
    ///
    /// Filter flags are mutually exclusive: only one can be active at a time.
    /// Note: --hardware, --network, --filesystem are include-only flags (not filter flags)
    /// and can be combined with each other.
    fn active_filter_flags(&self) -> Vec<&'static str> {
        let mut flags = Vec::new();

        // The --os flag is a filter (shows only OS section)
        if self.os {
            flags.push("--os");
        }

        // Detail-level filter flags (mutually exclusive)
        if self.git {
            flags.push("--git");
        }
        if self.repo {
            flags.push("--repo");
        }
        if self.language {
            flags.push("--language");
        }
        if self.cpu {
            flags.push("--cpu");
        }
        if self.gpu {
            flags.push("--gpu");
        }
        if self.memory {
            flags.push("--memory");
        }
        if self.storage {
            flags.push("--storage");
        }

        flags
    }

    /// Validate that at most one filter flag is active.
    ///
    /// ## Errors
    ///
    /// Returns an error if more than one filter flag is specified.
    fn validate_filter_flags(&self) -> Result<(), String> {
        let active = self.active_filter_flags();
        if active.len() > 1 {
            Err(format!(
                "{} are mutually exclusive. Only one filter flag can be used at a time.",
                active.join(" and ")
            ))
        } else {
            Ok(())
        }
    }

    /// Determine the output filter based on active filter flags.
    ///
    /// Only detail-level filter flags trigger filter mode.
    /// Top-level flags (--os, --hardware, --filesystem, --network) use include-only mode.
    fn output_filter(&self) -> OutputFilter {
        // Only use filter mode for detail-level flags (mutually exclusive)
        // Top-level include flags (--hardware, --network, --filesystem) are handled by include-only mode
        if self.os {
            return OutputFilter::Os;
        }
        if self.cpu {
            return OutputFilter::Cpu;
        }
        if self.gpu {
            return OutputFilter::Gpu;
        }
        if self.memory {
            return OutputFilter::Memory;
        }
        if self.storage {
            return OutputFilter::Storage;
        }
        if self.git {
            return OutputFilter::Git;
        }
        if self.repo {
            return OutputFilter::Repo;
        }
        if self.language {
            return OutputFilter::Language;
        }

        OutputFilter::All
    }
}

const AFTER_HELP: &str = "\
INCLUDE-ONLY MODE:
  When --hardware, --network, or --filesystem flags are used, sniff enters
  include-only mode. Only the specified sections are output; skip flags are ignored.

  Examples:
    sniff --hardware              # Output only hardware section
    sniff --network --filesystem  # Output network and filesystem, skip hardware
    sniff --hardware --skip-network  # Skip flag ignored in include-only mode

  Without include flags, existing skip flag behavior is preserved:
    sniff --skip-hardware         # Output network and filesystem

FILTER FLAGS (mutually exclusive):
  Filter flags show only specific subsections of output. Only one filter flag
  can be used at a time. Combining multiple filter flags produces an error.

  Top-level filters:
    --os          Show only OS information (name, kernel, locale, timezone)
    --filesystem  Show only filesystem information (git, languages, monorepo)
    --hardware    Show only hardware information (CPU, GPU, memory, storage)

  Detail-level filters:
    --git         Show only git repository information
    --repo        Show only repository/monorepo structure
    --language    Show only language detection results
    --cpu         Show only CPU information
    --gpu         Show only GPU information
    --memory      Show only memory information
    --storage     Show only storage/disk information

  Examples:
    sniff --cpu               # Show only CPU details
    sniff --git               # Show only git status
    sniff --cpu --memory      # ERROR: mutually exclusive
";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Validate mutually exclusive filter flags early
    if let Err(e) = cli.validate_filter_flags() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
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

    // Determine output filter
    let output_filter = cli.output_filter();

    // Apply skip logic based on filter mode
    match output_filter {
        // --os filter: show only OS section
        OutputFilter::Os => {
            config = config.skip_hardware().skip_network().skip_filesystem();
        }
        // Hardware detail filters: show only hardware section (no OS needed for subsections)
        OutputFilter::Cpu | OutputFilter::Gpu | OutputFilter::Memory | OutputFilter::Storage => {
            config = config.skip_os().skip_network().skip_filesystem();
        }
        // Filesystem detail filters: show only filesystem section
        OutputFilter::Git | OutputFilter::Repo | OutputFilter::Language => {
            config = config.skip_os().skip_hardware().skip_network();
        }
        // OutputFilter::Hardware and OutputFilter::Filesystem are not used directly
        // They're handled by include-only mode below
        OutputFilter::Hardware | OutputFilter::Filesystem | OutputFilter::All => {
            // Check include-only mode for section selection
            let include_only_mode = cli.hardware || cli.network || cli.filesystem;

            if include_only_mode {
                // In include-only mode, skip everything not explicitly included
                // OS is included with hardware, but skipped for other sections
                if !cli.hardware {
                    config = config.skip_os().skip_hardware();
                }
                if !cli.network {
                    config = config.skip_network();
                }
                if !cli.filesystem {
                    config = config.skip_filesystem();
                }
            } else {
                // Default behavior: respect skip flags
                if cli.skip_hardware {
                    config = config.skip_hardware();
                }
                if cli.skip_network {
                    config = config.skip_network();
                }
                if cli.skip_filesystem {
                    config = config.skip_filesystem();
                }
            }
        }
    }

    let mut result = detect_with_config(config)?;

    // Enrich dependencies with latest versions when --deep is enabled
    if deep_enabled {
        result = enrich_result_dependencies(result).await;
    }

    if cli.json {
        output::print_json(&result, output_filter)?;
    } else {
        output::print_text(&result, cli.verbose, output_filter);
    }

    Ok(())
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

    mod filter_flag_validation {
        use super::*;

        #[test]
        fn no_filter_flags_is_valid() {
            let cli = parse_args(&[]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_os_flag_is_valid() {
            let cli = parse_args(&["--os"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_cpu_flag_is_valid() {
            let cli = parse_args(&["--cpu"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_gpu_flag_is_valid() {
            let cli = parse_args(&["--gpu"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_memory_flag_is_valid() {
            let cli = parse_args(&["--memory"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_storage_flag_is_valid() {
            let cli = parse_args(&["--storage"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_git_flag_is_valid() {
            let cli = parse_args(&["--git"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_repo_flag_is_valid() {
            let cli = parse_args(&["--repo"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_language_flag_is_valid() {
            let cli = parse_args(&["--language"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_filesystem_flag_is_valid() {
            let cli = parse_args(&["--filesystem"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn single_hardware_flag_is_valid() {
            let cli = parse_args(&["--hardware"]).unwrap();
            assert!(cli.validate_filter_flags().is_ok());
        }

        #[test]
        fn cpu_and_memory_are_mutually_exclusive() {
            let cli = parse_args(&["--cpu", "--memory"]).unwrap();
            let result = cli.validate_filter_flags();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.contains("--cpu"));
            assert!(err.contains("--memory"));
            assert!(err.contains("mutually exclusive"));
        }

        #[test]
        fn git_and_repo_are_mutually_exclusive() {
            // --filesystem is an include-only flag, not a filter flag
            // Test with two detail-level filter flags instead
            let cli = parse_args(&["--git", "--repo"]).unwrap();
            let result = cli.validate_filter_flags();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.contains("--git"));
            assert!(err.contains("--repo"));
            assert!(err.contains("mutually exclusive"));
        }

        #[test]
        fn filesystem_can_combine_with_filter() {
            // --filesystem is an include-only flag, can combine with filter flags
            // (though the filter flag takes precedence)
            let cli = parse_args(&["--filesystem", "--git"]).unwrap();
            let result = cli.validate_filter_flags();
            // Only --git is a filter flag, so only 1 active = valid
            assert!(result.is_ok());
        }

        #[test]
        fn hardware_can_combine_with_filter() {
            // --hardware is an include-only flag, not a filter flag
            let cli = parse_args(&["--hardware", "--cpu"]).unwrap();
            let result = cli.validate_filter_flags();
            // Only --cpu is a filter flag, so only 1 active = valid
            assert!(result.is_ok());
        }

        #[test]
        fn os_and_storage_are_mutually_exclusive() {
            let cli = parse_args(&["--os", "--storage"]).unwrap();
            let result = cli.validate_filter_flags();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.contains("--os"));
            assert!(err.contains("--storage"));
        }

        #[test]
        fn three_flags_are_mutually_exclusive() {
            let cli = parse_args(&["--cpu", "--gpu", "--memory"]).unwrap();
            let result = cli.validate_filter_flags();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.contains("--cpu"));
            assert!(err.contains("--gpu"));
            assert!(err.contains("--memory"));
        }

        #[test]
        fn all_detail_level_flags_are_mutually_exclusive() {
            let cli = parse_args(&[
                "--git",
                "--repo",
                "--language",
                "--cpu",
                "--gpu",
                "--memory",
                "--storage",
            ])
            .unwrap();
            let result = cli.validate_filter_flags();
            assert!(result.is_err());
            let err = result.unwrap_err();
            // All should be mentioned
            assert!(err.contains("--git"));
            assert!(err.contains("--repo"));
            assert!(err.contains("--language"));
            assert!(err.contains("--cpu"));
            assert!(err.contains("--gpu"));
            assert!(err.contains("--memory"));
            assert!(err.contains("--storage"));
        }

        #[test]
        fn error_message_format_is_correct() {
            let cli = parse_args(&["--cpu", "--memory"]).unwrap();
            let result = cli.validate_filter_flags();
            let err = result.unwrap_err();
            // Should follow the format: "X and Y are mutually exclusive..."
            assert!(err.contains("and"));
            assert!(err.contains("Only one filter flag can be used at a time"));
        }
    }

    mod active_filter_flags {
        use super::*;

        #[test]
        fn returns_empty_when_no_flags() {
            let cli = parse_args(&[]).unwrap();
            assert!(cli.active_filter_flags().is_empty());
        }

        #[test]
        fn returns_single_flag() {
            let cli = parse_args(&["--cpu"]).unwrap();
            let flags = cli.active_filter_flags();
            assert_eq!(flags, vec!["--cpu"]);
        }

        #[test]
        fn returns_multiple_flags_in_order() {
            let cli = parse_args(&["--cpu", "--gpu", "--memory"]).unwrap();
            let flags = cli.active_filter_flags();
            // Order matches the order in active_filter_flags implementation
            assert_eq!(flags, vec!["--cpu", "--gpu", "--memory"]);
        }

        #[test]
        fn os_comes_before_detail_flags() {
            let cli = parse_args(&["--cpu", "--os", "--git"]).unwrap();
            let flags = cli.active_filter_flags();
            // --os comes before detail flags in the implementation
            // Note: --hardware and --filesystem are NOT filter flags
            assert_eq!(flags, vec!["--os", "--git", "--cpu"]);
        }

        #[test]
        fn hardware_is_not_a_filter_flag() {
            // --hardware is an include-only flag, not a filter flag
            let cli = parse_args(&["--hardware"]).unwrap();
            let flags = cli.active_filter_flags();
            assert!(flags.is_empty());
        }

        #[test]
        fn filesystem_is_not_a_filter_flag() {
            // --filesystem is an include-only flag, not a filter flag
            let cli = parse_args(&["--filesystem"]).unwrap();
            let flags = cli.active_filter_flags();
            assert!(flags.is_empty());
        }
    }

    mod cli_parsing {
        use super::*;

        #[test]
        fn filter_flags_do_not_conflict_with_skip_flags() {
            // Filter flags and skip flags should be independent in parsing
            let cli = parse_args(&["--cpu", "--skip-hardware"]).unwrap();
            assert!(cli.cpu);
            assert!(cli.skip_hardware);
            // But validation should fail for filter flag combinations
            assert!(cli.validate_filter_flags().is_ok()); // Only one filter flag
        }

        #[test]
        fn filter_flags_work_with_other_options() {
            let cli = parse_args(&["--cpu", "--json", "-v"]).unwrap();
            assert!(cli.cpu);
            assert!(cli.json);
            assert_eq!(cli.verbose, 1);
            assert!(cli.validate_filter_flags().is_ok());
        }
    }
}
