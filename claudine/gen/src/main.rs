//! `claudine-gen` — the provider-catalog generator binary.
//!
//! Dev-facing tool: plain-text output, no rendering dependencies.
//! `claudine providers generate` shells out to this binary so the
//! user-facing UX lives in claudine-cli without the CLI linking the
//! generator (bootstrap rule).

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use claudine_gen::{CheckOutcome, Decision, GenError, PROVIDER_SLUGS, Provenance};

#[derive(Debug, Parser)]
#[command(
    name = "claudine-gen",
    about = "Deterministic provider-catalog generator (research -> data.rs + catalog.json)"
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
    /// Regenerate `lib/src/provider/<slug>/data.rs` plus the committed
    /// `docs/providers/catalog.json`, confirming each drifted file on a
    /// TTY (`[y/N/q]`). Non-TTY stdin is report-only; declined or
    /// unwritten drift exits non-zero.
    Generate {
        /// Regenerate a single provider's data.rs (default: all).
        /// catalog.json always spans every provider.
        slug: Option<String>,
        /// Write every drifted file without prompting (CI-style
        /// unconditional writes — the pre-v1 `generate` behavior).
        #[arg(long)]
        yes: bool,
        /// Report-only regardless of TTY: print diffs, write nothing.
        #[arg(long, conflicts_with = "yes")]
        dry_run: bool,
    },
    /// Report-only drift check: regenerate from committed inputs and
    /// byte-compare against the committed data.rs files and catalog.json.
    /// Exits non-zero on drift. Same code path as the nextest drift test.
    Check {
        /// Check a single provider's data.rs (default: all).
        /// catalog.json is always checked against the full scope.
        slug: Option<String>,
    },
}

/// `Some(slug)` → a one-element list; `None` → every provider.
fn slug_scope(slug: &Option<String>) -> Vec<&str> {
    match slug {
        Some(one) => vec![one.as_str()],
        None => PROVIDER_SLUGS.to_vec(),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let command = if cli.mapping {
        Command::Mapping
    } else {
        cli.command.unwrap_or(Command::Check { slug: None })
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
        Command::Generate { slug, yes, dry_run } => {
            let area = resolve_area(area)?;
            run_generate(&area, &slug, yes, dry_run)
        }
        Command::Check { slug } => {
            let area = resolve_area(area)?;
            let mut drifted = false;
            let mut generations = Vec::with_capacity(PROVIDER_SLUGS.len());
            let scope = slug_scope(&slug);
            // catalog.json spans every provider, so generate the full
            // roster even when only one data.rs is being checked.
            for slug in PROVIDER_SLUGS {
                let (generation, outcome) = claudine_gen::check_area(&area, slug)?;
                if scope.contains(slug) {
                    match outcome {
                        CheckOutcome::Clean => {
                            println!("{slug}: clean (inputs match the committed data.rs)");
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
                                "{slug}: committed data.rs missing at {} — run `claudine-gen generate`",
                                path.display()
                            );
                        }
                    }
                    report_provenance(&generation);
                }
                generations.push(generation);
            }
            report_artifact_warning(&generations);
            match claudine_gen::check_catalog(&area, &generations)? {
                CheckOutcome::Clean => {
                    println!("catalog.json: clean (inputs match the committed catalog)");
                }
                CheckOutcome::Drift { details } => {
                    drifted = true;
                    println!("catalog.json: DRIFT — regenerate with `claudine-gen generate`:");
                    print_capped_diff(&details, "  ");
                }
                CheckOutcome::MissingCommitted { path } => {
                    drifted = true;
                    println!(
                        "catalog.json missing at {} — run `claudine-gen generate`",
                        path.display()
                    );
                }
            }
            match claudine_gen::check_signals(&area)? {
                CheckOutcome::Clean => {
                    println!(
                        "signals generated.rs: clean (inputs match the committed tables)"
                    );
                }
                CheckOutcome::Drift { details } => {
                    drifted = true;
                    println!(
                        "signals generated.rs: DRIFT — regenerate with `claudine-gen generate`:"
                    );
                    print_capped_diff(&details, "  ");
                }
                CheckOutcome::MissingCommitted { path } => {
                    drifted = true;
                    println!(
                        "signals generated.rs missing at {} — run `claudine-gen generate`",
                        path.display()
                    );
                }
            }
            let family_count = claudine_gen::compiled_family_keys(&generations).len();
            match claudine_gen::check_families(&area, &generations)? {
                CheckOutcome::Clean => {
                    println!(
                        "families generated.rs: clean ({family_count} family keys compiled)"
                    );
                }
                CheckOutcome::Drift { details } => {
                    drifted = true;
                    println!(
                        "families generated.rs: DRIFT — regenerate with `claudine-gen generate`:"
                    );
                    print_capped_diff(&details, "  ");
                }
                CheckOutcome::MissingCommitted { path } => {
                    drifted = true;
                    println!(
                        "families generated.rs missing at {} — run `claudine-gen generate`",
                        path.display()
                    );
                }
            }
            Ok(if drifted {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}

/// The `generate` UX: per-file diff + confirmation, decline → override
/// scaffolding, non-zero exit while drift remains unreconciled.
fn run_generate(
    area: &std::path::Path,
    slug: &Option<String>,
    yes: bool,
    dry_run: bool,
) -> Result<ExitCode, GenError> {
    let scope = slug_scope(slug);
    let generations = claudine_gen::generate_all(area)?;
    report_artifact_warning(&generations);
    // The signals and families artifacts are full-scope like catalog.json:
    // always rebuilt, written through the same per-file confirmation flow.
    let signals = claudine_gen::build_signals(area)?;
    let families = claudine_gen::build_families(area, &generations)?;
    println!(
        "families: {} family keys compiled from expected-offering joins",
        claudine_gen::compiled_family_keys(&generations).len()
    );

    let interactive = !yes && !dry_run && std::io::stdin().is_terminal();
    let report_only = dry_run || (!yes && !interactive);
    if report_only && !dry_run {
        println!("stdin is not a TTY — report-only (pass --yes to write unconditionally)");
    }

    let mut decide = |path: &std::path::Path, diff: &[String]| -> Decision {
        println!(
            "{}: {} differing line{}",
            path.display(),
            diff.len(),
            if diff.len() == 1 { "" } else { "s" }
        );
        print_capped_diff(diff, "  ");
        if yes {
            println!("  writing (--yes)");
            return Decision::Accept;
        }
        if report_only {
            return Decision::Decline;
        }
        prompt_decision(path)
    };
    let outcome = claudine_gen::apply_generations(
        area,
        &scope,
        &generations,
        &signals,
        &families,
        &mut decide,
    )?;

    for path in &outcome.written {
        println!("wrote {}", path.display());
    }
    for generation in &generations {
        if scope.contains(&generation.slug.as_str()) {
            report_provenance(generation);
        }
    }

    if outcome.declined.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    println!();
    if report_only {
        println!(
            "{} file(s) drifted — rerun on a TTY to confirm per file, or pass --yes",
            outcome.declined.len()
        );
    } else {
        println!(
            "{} declined file(s) remain unreconciled — pin the committed value with an \
             override, or revert the offending input, then regenerate:",
            outcome.declined.len()
        );
    }
    for declined in &outcome.declined {
        println!("  declined: {}", declined.path.display());
        if let Some(snippet) = &declined.override_snippet {
            for line in snippet.lines() {
                println!("    {line}");
            }
        }
    }
    Ok(ExitCode::FAILURE)
}

/// `[y/N/q]` prompt on stdin (default No; `q` stops prompting entirely).
fn prompt_decision(path: &std::path::Path) -> Decision {
    loop {
        print!(
            "write {}? [y/N/q] ",
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        );
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if std::io::stdin().lock().read_line(&mut line).is_err() || line.is_empty() {
            // EOF mid-session: stop prompting, decline the rest.
            return Decision::Quit;
        }
        match line.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Decision::Accept,
            "" | "n" | "no" => return Decision::Decline,
            "q" | "quit" => return Decision::Quit,
            other => println!("unrecognized `{other}` — expected y, n, or q"),
        }
    }
}

/// Compact diff: the first lines verbatim, then an elision count.
fn print_capped_diff(details: &[String], indent: &str) {
    const MAX_LINES: usize = 20;
    for line in details.iter().take(MAX_LINES) {
        println!("{indent}{line}");
    }
    if details.len() > MAX_LINES {
        println!("{indent}… {} more differing lines", details.len() - MAX_LINES);
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

/// Prints the models-catalog staleness warning once per run (the warning
/// is artifact-global, so every generation carries the same one).
fn report_artifact_warning(generations: &[claudine_gen::Generation]) {
    if let Some(warning) = generations
        .iter()
        .find_map(|generation| generation.artifact_warning.as_deref())
    {
        println!("WARNING: {warning}");
    }
}

/// Lists active overrides (with the value they suppress), staleness,
/// coercion skips (input records a coercion was pointed at but dropped —
/// silent drops would otherwise surface only after the field graduates off
/// its override), and expected-offering artifact-join coverage.
fn report_provenance(generation: &claudine_gen::Generation) {
    let join = &generation.offering_join;
    println!(
        "  expected offerings: {}/{} joined to the models-catalog artifact",
        join.joined, join.total
    );
    for id in &join.unjoined {
        println!("    unjoined: {id}");
    }
    for alias in &join.ambiguous_aliases {
        println!(
            "    ambiguous alias: `{alias}` spans multiple families — resolves mark dropped"
        );
    }
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
