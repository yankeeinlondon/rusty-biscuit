//! Render a resolved CI plan for a human about to trigger a workflow.
//!
//! `just ci-local --plan` writes the canonical plan JSON — the machine
//! interface — and hands it here to be read. Every question the pre-trigger
//! review of `fixes/2026-09-11-cicd-cleanup/spec.md` section 4 asks is a column
//! of one table: which cell, what will happen to it, where its result comes
//! from, and why.
//!
//! ## Notes
//!
//! Behind the `local-tools` feature because it links `biscuit-terminal`;
//! `ci-rollup`, the always-runs merge-gate binary, links none of the monorepo's
//! crates and must stay that way. `--plan` uses this renderer only when it is
//! already built, because a pre-trigger review must not start a build.

use std::env;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use biscuit_terminal::prelude::{
    Prose, Table, TableColumn, Terminal, TerminalRenderable, UnorderedList,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Plan {
    base: String,
    head: String,
    change_class: String,
    #[serde(default)]
    areas: Vec<Area>,
    #[serde(default)]
    cells: Vec<Cell>,
    #[serde(default)]
    evidence_rejections: Vec<String>,
    #[serde(default)]
    prohibited_cells: Vec<String>,
    job_estimate: u32,
}

#[derive(Debug, Deserialize)]
struct Area {
    area: String,
    selection_reason: String,
    #[serde(default)]
    packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Cell {
    package: String,
    area: String,
    environment: String,
    gate: String,
    execution: String,
    origin: String,
    state: String,
    selection_reason: String,
    #[serde(default)]
    evidence: Option<serde_json::Value>,
    #[serde(default)]
    gap: Option<Governance>,
    #[serde(default)]
    prohibition: Option<Governance>,
}

#[derive(Debug, Deserialize)]
struct Governance {
    #[serde(default)]
    owner: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    expiry: String,
}

impl Cell {
    fn key(&self) -> String {
        format!("{}/{}/{}", self.package, self.environment, self.gate)
    }

    /// What a reviewer needs beyond the state: where a reused cell's result
    /// comes from, or who owns a cell that will not run.
    fn detail(&self) -> String {
        if let Some(governance) = self.prohibition.as_ref().or(self.gap.as_ref()) {
            return format!(
                "{} (owner {}, expires {})",
                governance.reason, governance.owner, governance.expiry
            );
        }
        if let Some(evidence) = &self.evidence {
            let source = evidence
                .get("evidence")
                .and_then(|inner| inner.get("ref"))
                .or_else(|| evidence.get("ref"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("published evidence");
            let measured = evidence
                .get("measurements")
                .and_then(serde_json::Value::as_str);
            return match measured {
                Some(measurements) => format!("{source} — {measurements}"),
                None => source.to_owned(),
            };
        }
        self.selection_reason.clone()
    }
}

const COLUMNS: [&str; 4] = ["Cell", "Execution", "Origin", "State"];

/// The table's data, before any geometry is applied.
///
/// Separated from [`render`] because a rendered table wraps and hyphenates to
/// the terminal's width: asserting on its text would test the layout engine,
/// not the projection. This is what the fixtures assert on.
fn rows(plan: &Plan) -> Vec<Vec<String>> {
    plan.cells
        .iter()
        .map(|cell| {
            vec![
                cell.key(),
                cell.execution.clone(),
                cell.origin.clone(),
                cell.state.clone(),
            ]
        })
        .collect()
}

/// The cells whose disposition a reviewer cannot act on from the table alone.
///
/// A reused cell's evidence and a governed cell's owner and expiry are prose,
/// not a column: a cell key already consumes most of an 80-column table, and a
/// reason wrapped into fourteen characters is unreadable. An ordinary executing
/// cell needs no entry — "it will run" is the whole story.
fn details(plan: &Plan) -> Vec<String> {
    plan.cells
        .iter()
        .filter(|cell| {
            cell.evidence.is_some() || cell.gap.is_some() || cell.prohibition.is_some()
        })
        .map(|cell| format!("`{}` — {}", cell.key(), cell.detail()))
        .collect()
}

/// The refusal banner, or `None` when no cell is prohibited.
///
/// Areas are a grouping, never an identity, so the banner reports them while
/// the prohibited list itself stays package-keyed.
fn prohibition_summary(plan: &Plan) -> Option<String> {
    if plan.prohibited_cells.is_empty() {
        return None;
    }
    let areas = plan
        .cells
        .iter()
        .filter(|cell| plan.prohibited_cells.contains(&cell.key()))
        .map(|cell| cell.area.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    Some(format!(
        "<red>**{} prohibited cell(s)** across {}: triggering CI now would violate a \
         recorded execution constraint.</red>",
        plan.prohibited_cells.len(),
        areas.into_iter().collect::<Vec<_>>().join(", ")
    ))
}

fn plan_path() -> Result<String> {
    env::args()
        .nth(1)
        .context("usage: ci-plan <resolved-plan.json>")
}

fn render(plan: &Plan, term: &Terminal) -> String {
    let mut out = String::new();

    out.push_str(
        &Prose::new(format!(
            "**Resolved CI plan** — `{}..{}`, change class _{}_, {} job(s) estimated.",
            &plan.base[..plan.base.len().min(9)],
            &plan.head[..plan.head.len().min(9)],
            plan.change_class,
            plan.job_estimate
        ))
        .render(term),
    );
    out.push('\n');

    if !plan.areas.is_empty() {
        out.push_str(
            &UnorderedList::new(
                plan.areas
                    .iter()
                    .map(|area| {
                        format!(
                            "**{}** ({}) — {}",
                            area.area,
                            area.packages.join(", "),
                            area.selection_reason
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .render(term),
        );
        out.push('\n');
    }

    let columns = COLUMNS
        .into_iter()
        .map(|header| {
            let column = TableColumn::new(header);
            // A cell key is one unbreakable token that can reach forty columns,
            // so on a narrow terminal the origin — which the detail list
            // repeats — is the column that yields.
            if header == "Origin" {
                column.drop_when_space_is_limited(Some("origin shown in the details below"))
            } else {
                column
            }
        })
        .collect::<Vec<_>>();
    let data = rows(plan)
        .into_iter()
        .map(|row| row.into_iter().map(Into::into).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    out.push_str(&Table::new().with_columns(columns).with_data(data).render(term));
    out.push('\n');

    let details = details(plan);
    if !details.is_empty() {
        out.push_str(&UnorderedList::new(details).render(term));
        out.push('\n');
    }

    if !plan.evidence_rejections.is_empty() {
        out.push_str(&Prose::new("**Rejected evidence**").render(term));
        out.push('\n');
        out.push_str(&UnorderedList::new(plan.evidence_rejections.clone()).render(term));
        out.push('\n');
    }

    if let Some(banner) = prohibition_summary(plan) {
        out.push_str(&Prose::new(banner).render(term));
        out.push('\n');
    }
    out
}

fn main() -> ExitCode {
    let term = Terminal::new();
    let plan = match plan_path()
        .and_then(|path| {
            let text = fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
            serde_json::from_str::<Plan>(&text).with_context(|| format!("parsing {path}"))
        }) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("ci-plan: {error:#}");
            return ExitCode::from(2);
        }
    };
    print!("{}", render(&plan, &term));
    // The refusal belongs to the caller: `just ci-local --plan` owns the exit
    // status so the rule holds whether or not this renderer is installed.
    ExitCode::SUCCESS
}

#[cfg(test)]
#[path = "ci-plan-tests.rs"]
mod tests;
