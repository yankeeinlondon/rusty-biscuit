//! `claudine-gen` — the provider-catalog generator binary.
//!
//! Dev-facing tool: plain-text output, no rendering dependencies.
//! `claudine providers generate` shells out to this binary so the
//! user-facing UX lives in claudine-cli without the CLI linking the
//! generator (bootstrap rule).

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use claudine_gen::{CheckOutcome, GenError, Provenance};

#[derive(Debug, Parser)]
#[command(
    name = "claudine-gen",
    about = "Deterministic provider-catalog generator (research -> data.rs)"
)]
struct Cli {
    /// Claudine package-area root (the directory containing docs/providers.yaml).
    /// Default: walk upward from the current directory.
    #[arg(long, global = true)]
    area: Option<PathBuf>,

    /// Print the mapping registry as JSON (equivalent to the `mapping` subcommand).
    #[arg(long)]
    mapping: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the mapping registry (field -> source -> coercion) as JSON.
    Mapping,
    /// Regenerate committed data fragments and report provenance.
    Generate,
    /// Report-only drift check: regenerate from committed inputs and
    /// byte-compare against the committed fragment. Exits non-zero on
    /// drift. Same code path as the nextest drift test.
    Check,
    /// One-time bootstrap: read a serialized ProviderInfo array
    /// (`claudine providers --describe --format json`) on stdin and print
    /// the facts YAML for the provider. Retire after seeding.
    ScrapeFacts {
        /// Provider slug to scrape (e.g. `claude`).
        slug: String,
    },
}

const PROVIDER_SLUGS: &[&str] = &["claude"];

fn main() -> ExitCode {
    let cli = Cli::parse();
    let command = if cli.mapping {
        Command::Mapping
    } else {
        cli.command.unwrap_or(Command::Check)
    };
    match run(cli.area, command) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("claudine-gen error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(area: Option<PathBuf>, command: Command) -> Result<ExitCode, GenError> {
    match command {
        Command::Mapping => {
            println!(
                "{}",
                serde_json::to_string_pretty(&claudine_gen::mapping_json())
                    .expect("registry JSON is always serializable")
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Generate => {
            let area = resolve_area(area)?;
            for slug in PROVIDER_SLUGS {
                let generation = claudine_gen::generate_for_area(&area, slug)?;
                let path = claudine_gen::committed_fragment_path(&area, slug);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| GenError::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
                std::fs::write(&path, &generation.fragment).map_err(|source| GenError::Io {
                    path: path.clone(),
                    source,
                })?;
                println!("wrote {}", path.display());
                report_provenance(&generation);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Check => {
            let area = resolve_area(area)?;
            let mut drifted = false;
            for slug in PROVIDER_SLUGS {
                let (generation, outcome) = claudine_gen::check_area(&area, slug)?;
                match outcome {
                    CheckOutcome::Clean => {
                        println!("{slug}: clean (inputs match the committed fragment)");
                    }
                    CheckOutcome::Drift { details } => {
                        drifted = true;
                        println!("{slug}: DRIFT — regenerate with `claudine-gen generate`:");
                        for line in details {
                            println!("  {line}");
                        }
                    }
                    CheckOutcome::MissingCommitted { path } => {
                        drifted = true;
                        println!(
                            "{slug}: committed fragment missing at {} — run `claudine-gen generate`",
                            path.display()
                        );
                    }
                }
                report_provenance(&generation);
            }
            Ok(if drifted {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::ScrapeFacts { slug } => {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|source| GenError::Io {
                    path: PathBuf::from("<stdin>"),
                    source,
                })?;
            let payload: serde_json::Value =
                serde_json::from_str(&input).map_err(|err| GenError::Json {
                    message: err.to_string(),
                })?;
            print!("{}", claudine_gen::scrape_facts(&payload, &slug)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn resolve_area(area: Option<PathBuf>) -> Result<PathBuf, GenError> {
    match area {
        Some(area) => Ok(area),
        None => {
            let cwd = std::env::current_dir().map_err(|source| GenError::Io {
                path: PathBuf::from("."),
                source,
            })?;
            claudine_gen::find_area(&cwd)
        }
    }
}

/// Lists active overrides (with the value they suppress), staleness, and
/// coercion skips (input records a coercion was pointed at but dropped —
/// silent drops would otherwise surface only after the field graduates off
/// its override).
fn report_provenance(generation: &claudine_gen::Generation) {
    for skip in &generation.skips {
        println!(
            "  coercion skipped {} record{} for {} ({}):",
            skip.records.len(),
            if skip.records.len() == 1 { "" } else { "s" },
            skip.field,
            skip.reason
        );
        for record in &skip.records {
            println!("    {record:?}");
        }
    }
    for field in &generation.fields {
        if let Provenance::Override {
            reason,
            suppressed,
            stale,
        } = &field.provenance
        {
            let suppressed = suppressed
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<source unresolvable>".to_string());
            println!(
                "  override active: {} (suppressing {suppressed}) — {reason}",
                field.field
            );
            if *stale {
                println!(
                    "  STALE override: {} now equals the source value — delete it",
                    field.field
                );
            }
        }
    }
}
