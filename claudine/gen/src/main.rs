//! `claudine-gen` — the provider-catalog generator binary.
//!
//! Dev-facing tool. Human-facing reports render through the
//! [`claudine_gen::report`] module (biscuit-terminal `Prose` /
//! `UnorderedList`), so styling degrades cleanly on a pipe / `NO_COLOR` and
//! honors `FORCE_COLOR`. Machine-facing output — the `mapping` JSON document
//! and the `agent-errors` findings file — stays raw. `claudine providers
//! generate` shells out to this binary so the user-facing UX lives in
//! claudine-cli without the CLI linking the generator (bootstrap rule).

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use biscuit_terminal::terminal::Terminal;
use clap::{Parser, Subcommand};

use claudine_gen::{CheckOutcome, Decision, GenError, report};

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
        /// Scaffold a newly wired provider before generating its data.rs:
        /// write a TODO facts skeleton (first run, then stop) and the
        /// hand-owned mod.rs/behavior.rs stubs (never overwriting). Requires
        /// a slug whose `Provider` variant is already wired.
        #[arg(long)]
        scaffold: bool,
    },
    /// Report-only drift check: regenerate from committed inputs and
    /// byte-compare against the committed data.rs files and catalog.json.
    /// Exits non-zero on drift. Same code path as the nextest drift test.
    Check {
        /// Check a single provider's data.rs (default: all).
        /// catalog.json is always checked against the full scope.
        slug: Option<String>,
    },
    /// Deterministic validate-and-resume gate for the `agent-errors` research
    /// topic (spec D10). Its subcommands are the mechanical half of the fleet
    /// lifecycle: the fleet `success` stack runs `check`, then branches on the
    /// explicit outcome report status.
    AgentErrors {
        #[command(subcommand)]
        command: AgentErrorsCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AgentErrorsCommand {
    /// Check one provider's `agent-errors/<slug>.md` research document against
    /// its facts seed, writing an explicit Markdown outcome report. Input and
    /// schema failures are reported as `gate_error`; report persistence errors
    /// exit non-zero so lifecycle processing cannot consume stale state.
    Check {
        /// Provider slug (the research document stem).
        slug: String,
        /// Outcome-report path (default:
        /// `docs/research/agent-errors/.findings/<slug>.md`).
        #[arg(long)]
        findings: Option<PathBuf>,
    },
}

/// `Some(slug)` → a one-element list; `None` → every wired provider.
fn slug_scope(slug: &Option<String>) -> Vec<&str> {
    match slug {
        Some(one) => vec![one.as_str()],
        None => claudine_gen::provider_slugs(),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let command = if cli.mapping {
        Command::Mapping
    } else {
        cli.command.unwrap_or(Command::Check { slug: None })
    };
    let term = report::output_terminal();
    match run(&term, cli.area, command) {
        Ok(code) => code,
        Err(err) => {
            eprint!("{}", report::fatal(&term, &err));
            ExitCode::FAILURE
        }
    }
}

fn run(term: &Terminal, area: Option<PathBuf>, command: Command) -> Result<ExitCode, GenError> {
    match command {
        Command::Mapping => {
            // Machine-facing: raw JSON on stdout, never routed through the
            // terminal renderer.
            println!(
                "{}",
                serde_json::to_string_pretty(&claudine_gen::mapping_json())
                    .expect("registry JSON is always serializable")
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Generate {
            slug,
            yes,
            dry_run,
            scaffold,
        } => {
            let area = resolve_area(area)?;
            run_generate(term, &area, &slug, yes, dry_run, scaffold)
        }
        Command::Check { slug } => {
            let area = resolve_area(area)?;
            let mut drifted = false;
            let slugs = claudine_gen::provider_slugs();
            let mut generations = Vec::with_capacity(slugs.len());
            let scope = slug_scope(&slug);
            // catalog.json spans every provider, so generate the full
            // roster even when only one data.rs is being checked.
            for slug in &slugs {
                let (generation, outcome) = claudine_gen::check_area(&area, slug)?;
                if scope.contains(slug) {
                    if !matches!(outcome, CheckOutcome::Clean) {
                        drifted = true;
                    }
                    print!("{}", report::provider_check(term, slug, &outcome));
                    print!("{}", report::provenance(term, &generation));
                }
                generations.push(generation);
            }
            print!("{}", report::artifact_warning(term, &generations));

            let catalog = claudine_gen::check_catalog(&area, &generations)?;
            drifted |= !matches!(catalog, CheckOutcome::Clean);
            print!(
                "{}",
                report::artifact_check(
                    term,
                    "catalog.json",
                    "(inputs match the committed catalog)",
                    &catalog,
                )
            );

            let signals = claudine_gen::check_signals(&area)?;
            drifted |= !matches!(signals, CheckOutcome::Clean);
            print!(
                "{}",
                report::artifact_check(
                    term,
                    "signals generated.rs",
                    "(inputs match the committed tables)",
                    &signals,
                )
            );

            let vocabulary = claudine_gen::check_vocabulary(&area)?;
            drifted |= !matches!(vocabulary, CheckOutcome::Clean);
            print!(
                "{}",
                report::artifact_check(
                    term,
                    "stream vocabulary.rs",
                    "(inputs match the committed tables)",
                    &vocabulary,
                )
            );

            let family_count = claudine_gen::compiled_family_keys(&generations).len();
            let families = claudine_gen::check_families(&area, &generations)?;
            drifted |= !matches!(families, CheckOutcome::Clean);
            print!(
                "{}",
                report::artifact_check(
                    term,
                    "families generated.rs",
                    &format!("({family_count} family keys compiled)"),
                    &families,
                )
            );

            // Roster ↔ wired-set cross-validation: a wired slug with no
            // active roster entry is a loud error (propagates out); active
            // roster slugs with no wired variant are the "researched but not
            // yet code-supported" set, reported informationally.
            let cross = claudine_gen::cross_validate_roster(&area)?;
            print!("{}", report::roster(term, &cross.unwired_active));

            Ok(if drifted {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::AgentErrors { command } => {
            let area = resolve_area(area)?;
            match command {
                AgentErrorsCommand::Check { slug, findings } => {
                    let findings_path = findings
                        .unwrap_or_else(|| claudine_gen::default_findings_path(&area, &slug));
                    let report = claudine_gen::check_agent_errors(&area, &slug, &findings_path)?;
                    match report.status {
                        claudine_gen::GateStatus::Clean => {
                            print!("{}", report::agent_errors_clean(term, &slug));
                        }
                        claudine_gen::GateStatus::Findings => {
                            print!(
                                "{}",
                                report::agent_errors_findings(
                                    term,
                                    &slug,
                                    &report.findings,
                                    &findings_path,
                                )
                            );
                        }
                        claudine_gen::GateStatus::GateError => {
                            print!(
                                "{}",
                                report::agent_errors_gate_error(
                                    term,
                                    &slug,
                                    &findings_path,
                                    report.error.as_deref(),
                                )
                            );
                        }
                    }
                    // Every persisted outcome exits zero so the fleet can
                    // branch on its typed status. Persistence errors propagate.
                    Ok(ExitCode::SUCCESS)
                }
            }
        }
    }
}

/// The `generate` UX: optional provider scaffolding, then per-file diff +
/// confirmation, decline → override scaffolding, non-zero exit while drift
/// remains unreconciled.
fn run_generate(
    term: &Terminal,
    area: &std::path::Path,
    slug: &Option<String>,
    yes: bool,
    dry_run: bool,
    scaffold: bool,
) -> Result<ExitCode, GenError> {
    if scaffold {
        // --scaffold is slug-scoped: it seeds one provider's hand-owned
        // inputs before generating that provider's data.rs.
        let Some(slug) = slug.as_deref() else {
            return Err(GenError::ScaffoldRequiresSlug);
        };
        // Resolve the variant first so an unwired slug fails with the "wire
        // the enum variant + PROVIDER_VARIANTS entry first" message before
        // any file is written.
        let variant = claudine_gen::scaffold::provider_variant(slug)?;
        if claudine_gen::scaffold_facts(area, slug)? {
            print!("{}", report::scaffold_facts_notice(term, slug));
            return Ok(ExitCode::SUCCESS);
        }
        print!(
            "{}",
            report::scaffold_stub(
                term,
                claudine_gen::scaffold_mod(area, slug, variant)?,
                &format!("lib/src/provider/{slug}/mod.rs"),
            )
        );
        print!(
            "{}",
            report::scaffold_stub(
                term,
                claudine_gen::scaffold_behavior(area, slug, variant)?,
                &format!("lib/src/provider/{slug}/behavior.rs"),
            )
        );
        // Fall through to the normal data.rs generate/apply for this slug.
    }
    let scope = slug_scope(slug);
    let generations = claudine_gen::generate_all(area)?;
    print!("{}", report::artifact_warning(term, &generations));
    // The signals and families artifacts are full-scope like catalog.json:
    // always rebuilt, written through the same per-file confirmation flow.
    let signals = claudine_gen::build_signals(area)?;
    let families = claudine_gen::build_families(area, &generations)?;
    let vocabulary = claudine_gen::build_vocabulary(area)?;
    print!(
        "{}",
        report::families_count(term, claudine_gen::compiled_family_keys(&generations).len())
    );

    let interactive = !yes && !dry_run && std::io::stdin().is_terminal();
    let report_only = dry_run || (!yes && !interactive);
    if report_only && !dry_run {
        print!("{}", report::report_only_notice(term));
    }

    let mut decide = |path: &std::path::Path, diff: &[String]| -> Decision {
        print!("{}", report::decide_header(term, path, diff));
        if yes {
            print!("{}", report::writing_yes(term));
            return Decision::Accept;
        }
        if report_only {
            return Decision::Decline;
        }
        prompt_decision(term, path)
    };
    let outcome = claudine_gen::apply_generations(
        area,
        &scope,
        &generations,
        &signals,
        &families,
        &vocabulary,
        &mut decide,
    )?;

    for path in &outcome.written {
        print!("{}", report::wrote(term, path));
    }
    for generation in &generations {
        if scope.contains(&generation.slug.as_str()) {
            print!("{}", report::provenance(term, generation));
        }
    }

    if outcome.declined.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    print!(
        "{}",
        report::declined_summary(term, report_only, &outcome.declined)
    );
    Ok(ExitCode::FAILURE)
}

/// `[y/N/q]` prompt on stdin (default No; `q` stops prompting entirely).
fn prompt_decision(term: &Terminal, path: &std::path::Path) -> Decision {
    loop {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        print!("{}", report::prompt(term, &name));
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
            other => print!("{}", report::prompt_unrecognized(term, other)),
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
