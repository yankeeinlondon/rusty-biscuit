//! Shared, persisted execution budgets for `claudine sequence`.
//!
//! A budget ledger caps one platform run's agent invocations and active
//! wall-clock time across every step, lifecycle retry, resume, proxy, group
//! task, restart, and crash recovery. The ledger is a JSON file whose path the
//! caller supplies (`--budget-ledger`); Claudine never picks a default location
//! and has no default limits. Claudine owns every write to it; other tools read
//! it for reporting.
//!
//! Charging rules:
//!
//! - an invocation is debited before the agent is spawned and never refunded;
//! - time is charged by wall-clock segments while a budgeted run holds the
//!   ledger, so orchestration, shell tasks, automatic backoff, kill grace, and
//!   reader joins all count;
//! - only the explicit `stopped`, `suspended`, `interrupted`, and `exhausted`
//!   states pause the clock, and only a recorded operator grant adds
//!   allowance.
//!
//! Design record:
//! `messenger/features/2026-09-17-research-metadata-pipeline/architecture.md`
//! ("Orchestration and budget boundary").

mod error;
mod model;
mod run;
mod store;

#[cfg(test)]
mod tests;

use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::prelude::TerminalRenderable;
use color_eyre::eyre::{Result, eyre};

pub(crate) use error::BudgetError;
pub(crate) use model::{Allowance, CloseOutcome, Ledger, LedgerState};
pub(crate) use run::{
    acquire_recovered, admit_launch, attach_stop_flag, cap_wait, enter_stage, record_child,
    settle_launch, stopped_by_exhaustion,
};
pub(crate) use store::{LedgerFile, read};

use crate::commands::compose::SharedComposeArgs;

/// Exit status of a budgeted sequence stopped because its budget ran out.
pub(crate) const BUDGET_EXHAUSTED_EXIT_CODE: i32 = 76;

/// Exit status when the ledger cannot start a run until an operator acts:
/// it is suspended, interrupted, or held by another run.
pub(crate) const BUDGET_BLOCKED_EXIT_CODE: i32 = 77;

/// Run a sequence body under the ledger at `path`.
///
/// ## Errors
///
/// Refuses `--interactive` (passthrough mode does not isolate the child's
/// process tree) and `--dry-run` (it launches nothing to charge), and
/// propagates ledger I/O and validation failures. A ledger that is exhausted,
/// suspended, interrupted, or locked is a refusal with its own exit status.
pub(crate) fn run_with_ledger(
    path: &Path,
    shared: &SharedComposeArgs,
    body: impl FnOnce() -> Result<i32>,
) -> Result<i32> {
    if shared.interactive {
        return Err(eyre!(
            "--budget-ledger cannot be used with --interactive: interactive passthrough does not \
             isolate the agent's process tree, so the budget could not stop it"
        ));
    }
    if shared.dry_run {
        return Err(eyre!(
            "--budget-ledger cannot be used with --dry-run: a dry run launches no agent"
        ));
    }

    let run = match run::BudgetRun::open(path) {
        Ok(run) => run,
        Err(refusal @ (BudgetError::Exhausted(_)
        | BudgetError::NotRunnable { .. }
        | BudgetError::Locked { .. })) => {
            let code = if matches!(
                refusal,
                BudgetError::Exhausted(_)
                    | BudgetError::NotRunnable {
                        state: LedgerState::Exhausted,
                        ..
                    }
            ) {
                BUDGET_EXHAUSTED_EXIT_CODE
            } else {
                BUDGET_BLOCKED_EXIT_CODE
            };
            emit(&format!(
                "<red><bold>budget:</bold></red> no agent launched — {}",
                escape(&refusal.to_string())
            ));
            return Ok(code);
        }
        Err(other) => return Err(other.into()),
    };

    run::install(&run);
    let result = body();
    let stopped_by_budget = run.exhaustion_noted();
    run::uninstall();

    let outcome = match &result {
        Ok(0) => CloseOutcome::Completed,
        Ok(code)
            if *code == crate::commands::compose::interrupt::USER_INTERRUPT_EXIT_CODE
                && !stopped_by_budget =>
        {
            CloseOutcome::Interrupted
        }
        Ok(code) => CloseOutcome::Failed(*code),
        Err(error) => CloseOutcome::Error(error.to_string()),
    };
    let ledger = run.close(&outcome)?;
    emit_summary(&ledger, shared.silent);

    let code = result?;
    Ok(if ledger.state == LedgerState::Exhausted {
        BUDGET_EXHAUSTED_EXIT_CODE
    } else {
        code
    })
}

/// Render the ledger's state and consumption.
pub(crate) fn summary_lines(ledger: &Ledger) -> Vec<String> {
    let allowed = ledger.allowed();
    let mut lines = vec![
        format!(
            "<bold>budget:</bold> {} run <bold>{}</bold> is <bold>{}</bold>{}",
            escape(&ledger.platform),
            escape(&ledger.run_id),
            ledger.state.label(),
            ledger
                .stop_reason
                .as_deref()
                .map(|reason| format!(" — {}", escape(reason)))
                .unwrap_or_default()
        ),
        format!(
            "  used <bold>{}</bold>/{} invocations and <bold>{:.1}</bold>/{:.1} s of active time over {} run(s)",
            ledger.used.invocations,
            allowed.invocations,
            ledger.used.active_ms as f64 / 1000.0,
            allowed.active_ms as f64 / 1000.0,
            ledger.runs
        ),
    ];
    if let Some(stage) = ledger.stage.as_deref() {
        lines.push(format!("  last stage: <bold>{}</bold>", escape(stage)));
    }
    if ledger.state == LedgerState::Exhausted {
        lines.push(
            "  local agent processes were stopped; remote model work and billing may continue \
             (unverified). Only a recorded grant (<bold>claudine budget grant</bold>) adds allowance."
                .to_string(),
        );
    }
    lines
}

fn emit_summary(ledger: &Ledger, silent: bool) {
    // Exhaustion is a failure the operator must see even under `--silent`.
    if silent && ledger.state != LedgerState::Exhausted {
        return;
    }
    for line in summary_lines(ledger) {
        emit(&line);
    }
}

fn emit(markup: &str) {
    crate::log::message(&Prose::new(markup.to_string()).render(&crate::log::terminal()));
}

/// Keep ledger-supplied text from being read as Prose markup.
pub(crate) fn escape(text: &str) -> String {
    Prose::escape_text(text)
}
