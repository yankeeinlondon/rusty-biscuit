use clap::Parser;
use sniff_lib::{detect_with_config, SniffConfig};
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
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

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
