use clap::Parser;
use sniff_lib::{detect_with_config, SniffConfig};
use std::path::PathBuf;

mod output;

/// Detect system and repository information
#[derive(Parser)]
#[command(name = "sniff", version, about)]
struct Cli {
    /// Base directory for filesystem analysis
    #[arg(short, long)]
    base: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    format: OutputFormat,

    /// Skip hardware detection
    #[arg(long)]
    skip_hardware: bool,

    /// Skip network detection
    #[arg(long)]
    skip_network: bool,

    /// Skip filesystem detection
    #[arg(long)]
    skip_filesystem: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Canonicalize path if provided
    let base_dir = cli.base.map(|p| {
        std::fs::canonicalize(&p).unwrap_or(p)
    });

    let mut config = SniffConfig::new();

    if let Some(base) = base_dir {
        config = config.base_dir(base);
    }

    if cli.skip_hardware {
        config = config.skip_hardware();
    }

    if cli.skip_network {
        config = config.skip_network();
    }

    if cli.skip_filesystem {
        config = config.skip_filesystem();
    }

    let result = detect_with_config(config)?;

    match cli.format {
        OutputFormat::Text => output::print_text(&result),
        OutputFormat::Json => output::print_json(&result)?,
    }

    Ok(())
}
