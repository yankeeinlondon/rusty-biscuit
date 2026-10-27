//! The Mermaid render benchmark (R-V6): median and p95 per family for a warm
//! full render and a cold one-shot render, checked against the speed limits
//! and the stored baseline on the reference host. See README.md.

mod baseline;
mod corpus;
mod stats;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use biscuit_visualized::artifact::RenderRequest;
use biscuit_visualized::cache::file_cache::CACHE_DIR_ENV;
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
use clap::{Parser, Subcommand};
use tempfile::TempDir;

use crate::baseline::{Baseline, FamilyBaseline, Lookup, Stats};

const EXIT_FLAGGED: u8 = 1;
const EXIT_ERROR: u8 = 2;

#[derive(Parser)]
#[command(
    name = "biscuit-visualized-bench",
    about = "Benchmarks Mermaid rendering per family (warm and cold) against limits and a stored baseline",
    args_conflicts_with_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    child: Option<Child>,

    /// Baseline JSON to compare against (and to write with --write-baseline).
    #[arg(long, value_name = "PATH", default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/baseline.json"))]
    baseline: PathBuf,

    /// Flag a family whose warm median exceeds this many milliseconds.
    #[arg(long, value_name = "MS", default_value_t = 50.0, value_parser = positive_number)]
    warm_limit_ms: f64,

    /// Flag a family whose cold median is this many milliseconds or more.
    #[arg(long, value_name = "MS", default_value_t = 500.0, value_parser = positive_number)]
    cold_limit_ms: f64,

    /// Flag a family whose warm or cold median is more than this percentage
    /// slower than its baseline median.
    #[arg(long, value_name = "PCT", default_value_t = 25.0, value_parser = positive_number)]
    regression_pct: f64,

    /// Comma-separated families to run (default: every family of the corpus).
    #[arg(long, value_delimiter = ',', value_name = "A,B")]
    families: Vec<String>,

    /// Timed iterations per family, for the warm and the cold measurement each.
    #[arg(long, value_name = "N", default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..))]
    iterations: u32,

    /// Enforce the limits and the baseline although this host's name differs
    /// from the baseline's `reference_host`.
    #[arg(long)]
    reference: bool,

    /// Record this run's medians and p95s into the baseline (only on the
    /// reference host, and only in a change that knowingly accepts them).
    #[arg(long)]
    write_baseline: bool,
}

#[derive(Subcommand)]
enum Child {
    /// One render in a fresh process: the timed body of a cold measurement.
    #[command(name = "__one-shot", hide = true)]
    OneShot { family: String },
}

fn positive_number(text: &str) -> Result<f64, String> {
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => Ok(value),
        Ok(_) => Err("must be a positive number".to_owned()),
        Err(error) => Err(error.to_string()),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Some(Child::OneShot { family }) = &cli.child {
        return one_shot(family);
    }
    match run(&cli) {
        Ok(true) => ExitCode::from(EXIT_FLAGGED),
        Ok(false) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("bench: error: {message}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn diagram(source: &str) -> MermaidDiagram {
    MermaidDiagram::new(source).with_theme(MermaidTheme::Default)
}

/// The child side of a cold measurement. Its parent set the cache directory.
fn one_shot(family: &str) -> ExitCode {
    let Some(source) = corpus::source(family) else {
        eprintln!("bench one-shot: unknown family {family}");
        return ExitCode::from(EXIT_ERROR);
    };
    match diagram(source).render(&RenderRequest::default()) {
        Ok(artifact) if artifact.cache_hit => {
            eprintln!(
                "bench one-shot: {family} was a cache hit ({})",
                artifact.path.display()
            );
            ExitCode::from(EXIT_ERROR)
        }
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bench one-shot: {family} failed to render: {error}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

struct Measured {
    family: &'static str,
    warm: Stats,
    cold: Stats,
}

/// Runs the benchmark; `Ok(true)` when a rule flagged.
fn run(cli: &Cli) -> Result<bool, String> {
    let families = select_families(&cli.families)?;
    let known: Vec<&str> = corpus::names().collect();
    let stored = if cli.write_baseline && !cli.baseline.exists() {
        None
    } else {
        Some(baseline::load(&cli.baseline, &known).map_err(|error| error.to_string())?)
    };

    let host = host_name();
    let enforce = cli.reference
        || stored.as_ref().is_some_and(|stored| {
            host.as_deref()
                .is_some_and(|host| same_host(host, &stored.reference_host))
        });
    if cli.write_baseline && !enforce {
        return Err("--write-baseline records the reference host's timings; run it on the reference host or pass --reference".to_owned());
    }
    println!(
        "host {} | reference host {} | {} | {} iteration(s) per family",
        host.as_deref().unwrap_or("(unknown)"),
        stored
            .as_ref()
            .map_or("(no baseline yet)", |stored| stored.reference_host.as_str()),
        if enforce {
            "limits and baseline ENFORCED"
        } else {
            "informational only (not the reference host)"
        },
        cli.iterations,
    );

    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot locate the benchmark binary: {error}"))?;
    let mut measured = Vec::new();
    for family in families {
        eprintln!("bench: {family}");
        let source = corpus::source(family).expect("selected families are in the corpus");
        // One untimed render so the warm samples start in a process that has
        // already loaded fonts and initialized the renderer.
        warm_render(source)?;
        let warm = (0..cli.iterations)
            .map(|_| warm_render(source))
            .collect::<Result<Vec<_>, _>>()?;
        let cold = (0..cli.iterations)
            .map(|_| cold_render(&exe, family))
            .collect::<Result<Vec<_>, _>>()?;
        measured.push(Measured {
            family,
            warm: stats::summarize(&warm),
            cold: stats::summarize(&cold),
        });
    }

    let flagged = report(cli, &measured, stored.as_ref(), enforce);

    if cli.write_baseline {
        let mut updated = stored.unwrap_or_else(|| Baseline {
            reference_host: String::new(),
            families: BTreeMap::new(),
        });
        updated.reference_host =
            host.ok_or("cannot record a baseline: this host's name is unknown")?;
        for run in &measured {
            updated.families.insert(
                run.family.to_owned(),
                FamilyBaseline {
                    warm: run.warm,
                    cold: run.cold,
                },
            );
        }
        write_atomically(&cli.baseline, &updated.to_json())?;
        println!(
            "wrote baseline {} ({} families)",
            cli.baseline.display(),
            updated.families.len()
        );
    }
    Ok(flagged)
}

fn select_families(requested: &[String]) -> Result<Vec<&'static str>, String> {
    if requested.is_empty() {
        return Ok(corpus::names().collect());
    }
    requested
        .iter()
        .map(|family| {
            corpus::names()
                .find(|known| known == family)
                .ok_or_else(|| {
                    format!(
                        "unknown family {family:?} (known: {})",
                        corpus::names().collect::<Vec<_>>().join(", ")
                    )
                })
        })
        .collect()
}

/// A fresh, empty cache directory, verified empty before any timing starts.
fn fresh_cache_dir() -> Result<TempDir, String> {
    let dir = tempfile::Builder::new()
        .prefix("bv-bench-cache-")
        .tempdir()
        .map_err(|error| format!("cannot create a cache directory: {error}"))?;
    if !is_empty_dir(dir.path())? {
        return Err(format!(
            "fresh cache directory {} is not empty",
            dir.path().display()
        ));
    }
    Ok(dir)
}

fn is_empty_dir(path: &Path) -> Result<bool, String> {
    let mut entries = std::fs::read_dir(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(entries.next().is_none())
}

/// After a render, the artifact must sit in this iteration's directory: proof
/// the render used it rather than some shared cache.
fn check_used(dir: &TempDir, what: &str) -> Result<(), String> {
    if is_empty_dir(dir.path())? {
        return Err(format!(
            "{what}: the render wrote nothing to its cache directory; {CACHE_DIR_ENV} was not honored"
        ));
    }
    Ok(())
}

fn warm_render(source: &str) -> Result<f64, String> {
    let dir = fresh_cache_dir()?;
    let diagram = diagram(source);
    let request = RenderRequest::default();
    // SAFETY: the benchmark is single-threaded; nothing reads the environment
    // concurrently.
    unsafe { std::env::set_var(CACHE_DIR_ENV, dir.path()) };

    let start = Instant::now();
    let rendered = diagram.render(&request);
    let elapsed = start.elapsed();

    let artifact = rendered.map_err(|error| format!("warm render failed: {error}"))?;
    if artifact.cache_hit {
        return Err(format!(
            "warm render was a cache hit ({}); the cache bypass is broken",
            artifact.path.display()
        ));
    }
    check_used(&dir, "warm render")?;
    Ok(elapsed.as_secs_f64() * 1000.0)
}

fn cold_render(exe: &Path, family: &str) -> Result<f64, String> {
    let dir = fresh_cache_dir()?;
    let mut command = Command::new(exe);
    command
        .args(["__one-shot", family])
        .env(CACHE_DIR_ENV, dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let start = Instant::now();
    let output = command.output();
    let elapsed = start.elapsed();

    let output = output.map_err(|error| format!("cannot spawn the one-shot child: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cold one-shot of {family} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    check_used(&dir, "cold render")?;
    Ok(elapsed.as_secs_f64() * 1000.0)
}

/// Prints the per-family table and every rule that fired; returns whether any
/// rule flagged (only possible when `enforce`).
fn report(cli: &Cli, measured: &[Measured], stored: Option<&Baseline>, enforce: bool) -> bool {
    println!();
    println!(
        "{:<14} {:>10} {:>10} {:>10} {:>10} {:>9} {:>9}  status",
        "family", "warm med", "warm p95", "cold med", "cold p95", "Δ warm", "Δ cold"
    );
    let mut findings = Vec::new();
    for run in measured {
        let mut rules = Vec::new();
        if run.warm.median_ms > cli.warm_limit_ms {
            rules.push(format!(
                "warm median {:.2} ms is over the {} ms limit",
                run.warm.median_ms, cli.warm_limit_ms
            ));
        }
        if run.cold.median_ms >= cli.cold_limit_ms {
            rules.push(format!(
                "cold median {:.2} ms is at or over the {} ms limit",
                run.cold.median_ms, cli.cold_limit_ms
            ));
        }
        let lookup = stored.map_or(Lookup::NoBaseline, |stored| stored.lookup(run.family));
        let (delta_warm, delta_cold, baseline_state) = match lookup {
            Lookup::Recorded(recorded) => {
                let delta_warm = percent_slower(run.warm.median_ms, recorded.warm.median_ms);
                let delta_cold = percent_slower(run.cold.median_ms, recorded.cold.median_ms);
                // Writing a baseline accepts its timings, so a regression
                // against the old one is reported but not flagged.
                if !cli.write_baseline {
                    for (kind, delta, now, then) in [
                        (
                            "warm",
                            delta_warm,
                            run.warm.median_ms,
                            recorded.warm.median_ms,
                        ),
                        (
                            "cold",
                            delta_cold,
                            run.cold.median_ms,
                            recorded.cold.median_ms,
                        ),
                    ] {
                        if delta > cli.regression_pct {
                            rules.push(format!(
                                "{kind} median {now:.2} ms is {delta:.1}% slower than the baseline {then:.2} ms (limit {}%)",
                                cli.regression_pct
                            ));
                        }
                    }
                }
                (
                    format!("{delta_warm:+.1}%"),
                    format!("{delta_cold:+.1}%"),
                    None,
                )
            }
            Lookup::NoBaseline => {
                if !cli.write_baseline {
                    rules.push("no baseline recorded for this family".to_owned());
                }
                ("-".to_owned(), "-".to_owned(), Some("no baseline"))
            }
        };
        let status = match (rules.is_empty(), enforce) {
            (true, _) => baseline_state.unwrap_or("ok").to_owned(),
            (false, true) => "FLAGGED".to_owned(),
            (false, false) => "would flag (informational)".to_owned(),
        };
        println!(
            "{:<14} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>9} {:>9}  {status}",
            run.family,
            run.warm.median_ms,
            run.warm.p95_ms,
            run.cold.median_ms,
            run.cold.p95_ms,
            delta_warm,
            delta_cold
        );
        findings.extend(
            rules
                .into_iter()
                .map(|rule| format!("{}: {rule}", run.family)),
        );
    }
    if findings.is_empty() {
        println!("\nno rule fired");
        return false;
    }
    println!(
        "\n{}",
        if enforce {
            "FLAGGED:"
        } else {
            "would flag on the reference host (informational here):"
        }
    );
    for finding in &findings {
        println!("  {finding}");
    }
    enforce
}

fn percent_slower(now: f64, then: f64) -> f64 {
    (now / then - 1.0) * 100.0
}

/// This machine's hostname, from `hostname` (present on macOS, Linux, and
/// Windows).
fn host_name() -> Option<String> {
    let output = Command::new("hostname")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !name.is_empty()).then_some(name)
}

/// Hostnames match on their first label, case-insensitively, so `box` and
/// `box.local` are the same machine.
fn same_host(left: &str, right: &str) -> bool {
    let label = |name: &str| name.split('.').next().unwrap_or(name).to_ascii_lowercase();
    label(left) == label(right)
}

fn write_atomically(path: &Path, text: &str) -> Result<(), String> {
    let staging = path.with_extension("json.tmp");
    std::fs::write(&staging, text)
        .map_err(|error| format!("cannot write {}: {error}", staging.display()))?;
    std::fs::rename(&staging, path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_host_ignores_domain_and_case() {
        assert!(same_host("Build-Linux.local", "build-linux"));
        assert!(!same_host("build-linux2", "build-linux"));
    }

    #[test]
    fn percent_slower_is_relative_to_the_baseline() {
        assert_eq!(percent_slower(12.5, 10.0), 25.0);
        assert_eq!(percent_slower(5.0, 10.0), -50.0);
    }
}
