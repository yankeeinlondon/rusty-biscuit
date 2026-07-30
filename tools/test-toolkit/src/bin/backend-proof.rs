//! `backend-proof` — assert that every required L2 backend actually ran a test.
//!
//! `BISCUIT_TEST_REQUIRED_BACKENDS` makes a *missing* backend a hard failure,
//! but a backend that is installed and simply never exercised still produces a
//! green tier that verified nothing. This binary closes that gap: it reads the
//! JSON Lines evidence appended by `require_level!` and fails when a required
//! backend has no `run` record.
//!
//! ## Examples
//!
//! ```text
//! backend-proof reset                       # before the nextest run
//! just test-l2 <area>                       # tests append evidence
//! backend-proof verify                      # after the nextest run
//! backend-proof verify --required tmux,wezterm --stage-dir /tmp/reports
//! ```
//!
//! ## Notes
//!
//! `reset` must run *before* the tier, or a previous run's evidence satisfies
//! the check and the whole mechanism silently degrades to a no-op.
//!
//! Exit codes: `0` proved (or nothing required), `1` a required backend is
//! unproven, `2` bad configuration or unreadable evidence.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use test_toolkit::{
    BACKEND_EXECUTIONS_FILE, BISCUIT_TEST_REQUIRED_BACKENDS, Backend, decision_counts,
    parse_required_backends, read_backend_executions, required_backends, stage_dir,
    unproven_backends,
};

/// Exit code for "a required backend produced no executed test".
const UNPROVEN_EXIT_CODE: u8 = 1;
/// Exit code for a configuration or I/O problem, distinct from a real verdict.
const CONFIG_EXIT_CODE: u8 = 2;

#[derive(Parser)]
#[command(
    name = "backend-proof",
    about = "Verify that every required L2 backend executed at least one test"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Delete stale execution evidence. Run before the tier.
    Reset(CommonArgs),
    /// Fail unless every required backend has at least one executed test.
    Verify(CommonArgs),
}

#[derive(clap::Args)]
struct CommonArgs {
    /// Staging directory holding the evidence file. Defaults to
    /// `$BISCUIT_JUNIT_STAGE_DIR`, else `target/nextest/ci-reports` under the
    /// workspace root.
    #[arg(long)]
    stage_dir: Option<PathBuf>,

    /// Comma-separated required backends. Defaults to
    /// `$BISCUIT_TEST_REQUIRED_BACKENDS`.
    #[arg(long)]
    required: Option<String>,
}

impl CommonArgs {
    fn evidence_path(&self) -> PathBuf {
        self.stage_dir
            .clone()
            .unwrap_or_else(stage_dir)
            .join(BACKEND_EXECUTIONS_FILE)
    }

    fn required(&self) -> Result<BTreeSet<Backend>, String> {
        match &self.required {
            Some(raw) => parse_required_backends(raw).map_err(|err| err.to_string()),
            None => required_backends().map_err(|err| err.to_string()),
        }
    }
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Reset(args) => reset(&args),
        Command::Verify(args) => verify(&args),
    }
}

fn reset(args: &CommonArgs) -> ExitCode {
    let path = args.evidence_path();
    match std::fs::remove_file(&path) {
        Ok(()) => println!("backend-proof: cleared {}", path.display()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!("backend-proof: no evidence at {}", path.display());
        }
        Err(err) => {
            eprintln!("backend-proof: could not clear {}: {err}", path.display());
            return ExitCode::from(CONFIG_EXIT_CODE);
        }
    }
    ExitCode::SUCCESS
}

fn verify(args: &CommonArgs) -> ExitCode {
    let required = match args.required() {
        Ok(required) => required,
        Err(message) => {
            eprintln!("backend-proof: {message}");
            return ExitCode::from(CONFIG_EXIT_CODE);
        }
    };

    if required.is_empty() {
        println!(
            "backend-proof: {BISCUIT_TEST_REQUIRED_BACKENDS} is empty; nothing to verify"
        );
        return ExitCode::SUCCESS;
    }

    let path = args.evidence_path();
    let records = match read_backend_executions(&path) {
        Ok(records) => records,
        Err(err) => {
            eprintln!("backend-proof: could not read {}: {err}", path.display());
            return ExitCode::from(CONFIG_EXIT_CODE);
        }
    };

    for backend in &required {
        let (run, skip, panicked) = decision_counts(*backend, &records);
        println!(
            "backend-proof: {:<15} run={run} skip={skip} panic={panicked}",
            backend.as_str()
        );
    }

    let unproven = unproven_backends(&required, &records);
    if unproven.is_empty() {
        return ExitCode::SUCCESS;
    }

    let names: Vec<&str> = unproven.iter().map(|backend| backend.as_str()).collect();
    eprintln!(
        "backend-proof: no test executed for required backend(s): {}",
        names.join(", ")
    );
    eprintln!("backend-proof: evidence file {}", path.display());
    if records.is_empty() {
        eprintln!(
            "backend-proof: the evidence file is empty or absent — either the tier selected no \
             backend-gated tests, or {BISCUIT_TEST_REQUIRED_BACKENDS} was not set for the test \
             processes, or the staging directory was not writable."
        );
    }
    ExitCode::from(UNPROVEN_EXIT_CODE)
}
