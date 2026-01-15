use clap::Parser;
use sniff_lib::{SniffConfig, detect_with_config};
use std::path::PathBuf;

mod output;

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
    fn active_filter_flags(&self) -> Vec<&'static str> {
        let mut flags = Vec::new();

        // Top-level filter flags
        if self.os {
            flags.push("--os");
        }
        if self.filesystem {
            flags.push("--filesystem");
        }
        if self.hardware {
            flags.push("--hardware");
        }

        // Detail-level filter flags
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Validate mutually exclusive filter flags early
    if let Err(e) = cli.validate_filter_flags() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Canonicalize path if provided
    let base_dir = cli.base.map(|p| std::fs::canonicalize(&p).unwrap_or(p));

    let mut config = SniffConfig::new();

    if let Some(base) = base_dir {
        config = config.base_dir(base);
    }

    if cli.deep {
        config = config.deep(true);
    }

    // Check if we're in include-only mode (any include flag is set)
    let include_only_mode = cli.hardware || cli.network || cli.filesystem;

    if include_only_mode {
        // In include-only mode, skip everything not explicitly included
        // Skip flags are ignored
        // OS is included with hardware, but skipped for other sections
        if !cli.hardware {
            config = config.skip_os();
        }
        if !cli.hardware {
            config = config.skip_hardware();
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

    let result = detect_with_config(config)?;

    if cli.json {
        output::print_json(&result)?;
    } else {
        output::print_text(&result, cli.verbose, include_only_mode);
    }

    Ok(())
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
        fn filesystem_and_git_are_mutually_exclusive() {
            let cli = parse_args(&["--filesystem", "--git"]).unwrap();
            let result = cli.validate_filter_flags();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.contains("--filesystem"));
            assert!(err.contains("--git"));
            assert!(err.contains("mutually exclusive"));
        }

        #[test]
        fn hardware_and_cpu_are_mutually_exclusive() {
            let cli = parse_args(&["--hardware", "--cpu"]).unwrap();
            let result = cli.validate_filter_flags();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.contains("--hardware"));
            assert!(err.contains("--cpu"));
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
        fn top_level_flags_come_first() {
            let cli = parse_args(&["--cpu", "--os", "--git"]).unwrap();
            let flags = cli.active_filter_flags();
            // os comes before cpu in the implementation
            assert_eq!(flags, vec!["--os", "--git", "--cpu"]);
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
