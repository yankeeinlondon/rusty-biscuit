//! `backend-proof` — assert that every required L2 backend actually ran a test.
//!
//! `BISCUIT_TEST_REQUIRED_BACKENDS` makes a *missing* backend a hard failure,
//! but a backend that is installed and simply never exercised still produces a
//! green tier that verified nothing. This binary closes that gap: it reads the
//! JSON Lines evidence appended by `require_level!`, fails when a required
//! backend has no `run` record, and writes the per-backend verdict to
//! `backend-proofs.json` beside the evidence so `scripts/ci/completion.py
//! --backend-proofs` can certify the cell from the same facts.
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
//! the check and the whole mechanism silently degrades to a no-op. It clears
//! the verdict document too, so a stale `proven: true` cannot outlive the
//! evidence it was derived from.
//!
//! `verify` writes the verdict document on the proven and the unproven outcome
//! alike, and not at all when nothing is required: an unproven backend is
//! recorded as `proven: false`, which is a different fact from silence.
//!
//! Exit codes: `0` proved (or nothing required), `1` a required backend is
//! unproven, `2` bad configuration, unreadable evidence, or an unwritable
//! verdict document.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use test_toolkit::{
    BACKEND_EXECUTIONS_FILE, BACKEND_PROOFS_FILE, BISCUIT_TEST_REQUIRED_BACKENDS, Backend,
    clear_backend_evidence, decision_counts, parse_required_backends, read_backend_executions,
    required_backends, stage_dir, unproven_backends, write_backend_proofs,
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
    /// Delete stale execution evidence and the previous verdict. Run before the tier.
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
    fn stage(&self) -> PathBuf {
        self.stage_dir.clone().unwrap_or_else(stage_dir)
    }

    fn evidence_path(&self) -> PathBuf {
        self.stage().join(BACKEND_EXECUTIONS_FILE)
    }

    fn proofs_path(&self) -> PathBuf {
        self.stage().join(BACKEND_PROOFS_FILE)
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
    let stage = args.stage();
    match clear_backend_evidence(&stage) {
        Ok(()) => println!(
            "backend-proof: cleared {} and {} under {}",
            BACKEND_EXECUTIONS_FILE,
            BACKEND_PROOFS_FILE,
            stage.display()
        ),
        Err(err) => {
            eprintln!("backend-proof: could not clear {}: {err}", stage.display());
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

    // Before the verdict, so an unproven backend leaves `proven: false` behind
    // rather than nothing; `completion.py` treats both as unproven, but only the
    // document says the check ran.
    let proofs = args.proofs_path();
    let written = write_backend_proofs(&proofs, &required, &records);
    match &written {
        Ok(()) => println!("backend-proof: wrote {}", proofs.display()),
        Err(err) => eprintln!("backend-proof: could not write {}: {err}", proofs.display()),
    }

    let unproven = unproven_backends(&required, &records);
    if unproven.is_empty() {
        return if written.is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(CONFIG_EXIT_CODE)
        };
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
