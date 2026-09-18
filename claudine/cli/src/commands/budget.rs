//! `claudine budget` — create and operate the shared budget ledger that
//! `claudine sequence --budget-ledger` enforces.
//!
//! Every mutation takes the ledger lock, so none can run while a budgeted
//! sequence holds it, and each first recovers a crashed runner.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use chrono::Utc;
use clap::{Args, Subcommand};
use color_eyre::eyre::{Result, WrapErr};

use crate::budget::{self, Allowance, Ledger, LedgerFile};
use crate::log;

/// Arguments for `claudine budget`.
#[derive(Debug, Args)]
pub struct BudgetArgs {
    #[command(subcommand)]
    pub command: BudgetCommand,
}

#[derive(Debug, Subcommand)]
pub enum BudgetCommand {
    /// Create a ledger. Both limits are required; there are no defaults.
    Init {
        /// Path of the ledger file to create.
        ledger: PathBuf,
        /// Stable identity of the run this ledger budgets.
        #[arg(long)]
        run_id: String,
        /// Platform (or other unit) the run covers.
        #[arg(long)]
        platform: String,
        /// Active elapsed-time limit, in whole seconds.
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
        max_seconds: u64,
        /// Agent-invocation limit.
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
        max_invocations: u64,
        /// Lock file shared by runs that must execute one at a time; a
        /// relative path resolves against the ledger's directory.
        #[arg(long, value_name = "PATH")]
        exclusive_lock: Option<String>,
        /// How often a running sequence persists charged time (e.g. `5s`,
        /// `0.5s`). A crash is charged one interval past the last heartbeat.
        #[arg(long, default_value = "5s", value_name = "DURATION")]
        heartbeat: String,
    },
    /// Show a ledger's state, consumption, and history.
    Show {
        ledger: PathBuf,
        /// Print the ledger document as JSON on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Pause a stopped ledger for human approval.
    Suspend {
        ledger: PathBuf,
        #[arg(long)]
        reason: String,
    },
    /// Return a suspended or interrupted ledger to `stopped` so a run may start.
    Resume {
        ledger: PathBuf,
        /// Operator authorizing the resumption.
        #[arg(long)]
        operator: String,
    },
    /// Record extra allowance decided by an operator.
    Grant {
        ledger: PathBuf,
        #[arg(long)]
        operator: String,
        #[arg(long)]
        reason: String,
        /// Extra agent invocations.
        #[arg(long, default_value_t = 0)]
        invocations: u64,
        /// Extra active seconds.
        #[arg(long, default_value_t = 0)]
        seconds: u64,
    },
}

/// Entry point for `claudine budget`.
pub fn run(args: BudgetArgs) -> Result<()> {
    match args.command {
        BudgetCommand::Init {
            ledger,
            run_id,
            platform,
            max_seconds,
            max_invocations,
            exclusive_lock,
            heartbeat,
        } => {
            let heartbeat = claudine::harness::parse_timeout(&heartbeat, Path::new("<--heartbeat>"))
                .wrap_err("invalid --heartbeat value")?;
            let record = Ledger::new(
                &run_id,
                &platform,
                Allowance {
                    invocations: max_invocations,
                    active_ms: max_seconds.saturating_mul(1000),
                },
                u64::try_from(heartbeat.as_millis()).unwrap_or(u64::MAX).max(1),
                exclusive_lock,
                Utc::now(),
            )?;
            LedgerFile::create(&ledger, &record)?;
            report(&record);
        }
        BudgetCommand::Show { ledger, json } => {
            let record = budget::read(&ledger)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record)?);
            } else {
                report(&record);
            }
        }
        BudgetCommand::Suspend { ledger, reason } => {
            mutate(&ledger, |record| record.suspend(&reason, Utc::now()))?;
        }
        BudgetCommand::Resume { ledger, operator } => {
            mutate(&ledger, |record| record.resume(&operator, Utc::now()))?;
        }
        BudgetCommand::Grant {
            ledger,
            operator,
            reason,
            invocations,
            seconds,
        } => {
            let extra = Allowance {
                invocations,
                active_ms: seconds.saturating_mul(1000),
            };
            mutate(&ledger, |record| record.grant(&operator, &reason, extra, Utc::now()))?;
        }
    }
    Ok(())
}

fn mutate(
    path: &Path,
    change: impl FnOnce(&mut Ledger) -> Result<(), budget::BudgetError>,
) -> Result<()> {
    let (file, mut record, recovered) = budget::acquire_recovered(path)?;
    if recovered.is_some() {
        emit("<orange><bold>budget:</bold></orange> recovered a crashed run before applying the change");
    }
    change(&mut record)?;
    file.write(&record)?;
    report(&record);
    Ok(())
}

fn report(record: &Ledger) {
    for line in budget::summary_lines(record) {
        emit(&line);
    }
}

fn emit(markup: &str) {
    log::message(&Prose::new(markup.to_string()).render(&log::terminal()));
}
