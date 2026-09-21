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

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use biscuit_terminal::prelude::{
    Prose, Table, TableColumn, Terminal, TerminalRenderable, UnorderedList,
};
use serde::Deserialize;

#[path = "ci-change-inventory.rs"]
mod change_inventory;

use change_inventory::{ChangeInventory, NO_PACKAGE_TESTS};

#[derive(Debug, Deserialize)]
struct Plan {
    base: String,
    head: String,
    change_class: String,
    /// What changed, from the plan's own classification. Defaulted so a plan
    /// written before schema 3 still renders the cells it does carry.
    #[serde(default)]
    change_inventory: ChangeInventory,
    #[serde(default)]
    areas: Vec<Area>,
    #[serde(default)]
    cells: Vec<Cell>,
    #[serde(default)]
    builds: Vec<Build>,
    #[serde(default)]
    evidence_rejections: Vec<String>,
    #[serde(default)]
    prohibited_cells: Vec<String>,
    job_estimate: u32,
    /// The dispatch rows the hosted matrices expand, by area. Defaulted so a
    /// plan written before schema 5 still renders the cells it does carry.
    #[serde(default)]
    rows: BTreeMap<String, AreaRows>,
    #[serde(default)]
    skip_policy: Option<SkipPolicy>,
}

/// One area's four disjoint row sets. The scalar `has_*_rows` guards belong to
/// the workflow, not to a reader, so they are not deserialized here.
#[derive(Debug, Default, Deserialize)]
struct AreaRows {
    #[serde(default)]
    test: Vec<Row>,
    #[serde(default)]
    check: Vec<Row>,
    #[serde(default)]
    lint: Vec<Row>,
    #[serde(default)]
    wsl: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct Row {
    package: String,
    gate: String,
    environment: String,
    runner: String,
}

/// The plan's snapshot of the approved exact-skip budget.
#[derive(Debug, Deserialize)]
struct SkipPolicy {
    source: String,
    content_hash: String,
    #[serde(default)]
    entries: Vec<SkipEntry>,
}

#[derive(Debug, Deserialize)]
struct SkipEntry {
    package: String,
    environment: String,
    gate: String,
    owner: String,
    reason: String,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    expiry: Option<String>,
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

/// One immutable compile configuration and the cells that execute it.
///
/// Build plumbing, never a result cell: it is rendered in its own table so a
/// reviewer cannot mistake an owner for something that produces a gate outcome.
#[derive(Debug, Deserialize)]
struct Build {
    key: String,
    package: String,
    producer: String,
    compatibility_reason: String,
    #[serde(default)]
    consumers: Vec<Consumer>,
}

#[derive(Debug, Deserialize)]
struct Consumer {
    environment: String,
    gate: String,
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

const ROW_COLUMNS: [&str; 4] = ["Area", "Set", "Cell", "Runner"];

impl AreaRows {
    /// The four sets in dispatch order, each with the name the workflow's
    /// input carries.
    fn sets(&self) -> [(&'static str, &Vec<Row>); 4] {
        [
            ("test", &self.test),
            ("check", &self.check),
            ("lint", &self.lint),
            ("wsl", &self.wsl),
        ]
    }

    fn total(&self) -> usize {
        self.sets().iter().map(|(_, rows)| rows.len()).sum()
    }
}

/// One line per approved skip: which cell, who owns it, and until when.
///
/// The governance is prose for the same reason a gap's is — an owner and a
/// reason wrapped into a table column are unreadable — and it is separated
/// from [`render`] so a fixture can assert on it without asserting on the
/// layout engine's wrapping.
fn skip_details(policy: &SkipPolicy) -> Vec<String> {
    policy
        .entries
        .iter()
        .map(|entry| {
            format!(
                "`{}/{}/{}`{} — {} (owner {}, {})",
                entry.package,
                entry.environment,
                entry.gate,
                entry
                    .backend
                    .as_deref()
                    .map(|backend| format!(" on {backend}"))
                    .unwrap_or_default(),
                entry.reason,
                entry.owner,
                entry
                    .expiry
                    .as_deref()
                    .map(|expiry| format!("expires {expiry}"))
                    .unwrap_or_else(|| "no expiry".to_owned()),
            )
        })
        .collect()
}

/// The dispatch table's data: one line per matrix row CI will expand.
///
/// A reviewer's question is whether what CI dispatches is what the plan
/// resolved, so the rows are rendered as the plan carries them rather than
/// re-derived from the cells here. A tool that recomputed them could agree
/// with itself while disagreeing with the scheduler.
fn dispatch_rows(plan: &Plan) -> Vec<Vec<String>> {
    plan.rows
        .iter()
        .flat_map(|(area, sets)| {
            sets.sets().into_iter().flat_map(move |(name, rows)| {
                rows.iter().map(move |row| {
                    vec![
                        area.clone(),
                        name.to_owned(),
                        format!("{}/{}/{}", row.package, row.environment, row.gate),
                        row.runner.clone(),
                    ]
                })
            })
        })
        .collect()
}

const BUILD_COLUMNS: [&str; 4] = ["Build", "Package", "Producer", "Consumers"];

impl Build {
    fn consumer_list(&self) -> String {
        self.consumers
            .iter()
            .map(|consumer| format!("{}/{}", consumer.environment, consumer.gate))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The build table's data, before any geometry is applied.
fn build_rows(plan: &Plan) -> Vec<Vec<String>> {
    plan.builds
        .iter()
        .map(|build| {
            vec![
                build.key.clone(),
                build.package.clone(),
                build.producer.clone(),
                build.consumer_list(),
            ]
        })
        .collect()
}

/// Why each build may run where the plan says it may.
///
/// Prose rather than a fifth column for the same reason a cell's evidence is:
/// the reason is a sentence, and a sentence wrapped into fourteen characters is
/// unreadable.
fn build_details(plan: &Plan) -> Vec<String> {
    plan.builds
        .iter()
        .map(|build| format!("`{}` — {}", build.key, build.compatibility_reason))
        .collect()
}

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

    out.push_str(&Prose::new(plan.change_inventory.headline()).render(term));
    out.push('\n');
    let inventory = plan.change_inventory.plain_entries();
    if !inventory.is_empty() {
        out.push_str(&UnorderedList::new(inventory).render(term));
        out.push('\n');
    }

    // Spec section 7: the absence of scheduled work is a decision a reviewer
    // has to see stated, not infer from an empty table.
    if plan.cells.is_empty() {
        out.push_str(&Prose::new(NO_PACKAGE_TESTS).render(term));
        out.push('\n');
    }

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

    if !plan.builds.is_empty() {
        out.push_str(
            &Prose::new(format!(
                "**Build records** — {} immutable compile(s) on {} native owner(s); every \
                 executing test cell above runs one of them and compiles nothing itself.",
                plan.builds.len(),
                plan.builds
                    .iter()
                    .map(|build| build.producer.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
            ))
            .render(term),
        );
        out.push('\n');
        let columns = BUILD_COLUMNS
            .into_iter()
            .map(|header| {
                let column = TableColumn::new(header);
                // The producer is named again in the compatibility reason
                // below, so it is the column that yields on a narrow terminal.
                if header == "Producer" {
                    column.drop_when_space_is_limited(Some("producer named in the reasons below"))
                } else {
                    column
                }
            })
            .collect::<Vec<_>>();
        let data = build_rows(plan)
            .into_iter()
            .map(|row| row.into_iter().map(Into::into).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        out.push_str(&Table::new().with_columns(columns).with_data(data).render(term));
        out.push('\n');
        out.push_str(&UnorderedList::new(build_details(plan)).render(term));
        out.push('\n');
    }

    let dispatched = dispatch_rows(plan);
    if !dispatched.is_empty() {
        out.push_str(
            &Prose::new(format!(
                "**Dispatch rows** — {} matrix row(s) across {} area(s); one per executing \
                 cell, and nothing else is scheduled.",
                dispatched.len(),
                plan.rows
                    .values()
                    .filter(|sets| sets.total() > 0)
                    .count()
            ))
            .render(term),
        );
        out.push('\n');
        let columns = ROW_COLUMNS
            .into_iter()
            .map(|header| {
                let column = TableColumn::new(header);
                // The runner is derivable from the environment for every
                // native row, so it is the column that yields when a cell key
                // has already taken most of the width.
                if header == "Runner" {
                    column.drop_when_space_is_limited(Some("runner comes from the plan's environment table"))
                } else {
                    column
                }
            })
            .collect::<Vec<_>>();
        let data = dispatched
            .into_iter()
            .map(|row| row.into_iter().map(Into::into).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        out.push_str(&Table::new().with_columns(columns).with_data(data).render(term));
        out.push('\n');
    }

    if let Some(policy) = &plan.skip_policy {
        out.push_str(
            &Prose::new(format!(
                "**Approved skips** — {} entr(ies) from `{}` (`{}`).",
                policy.entries.len(),
                policy.source,
                policy.content_hash
            ))
            .render(term),
        );
        out.push('\n');
        let approvals = skip_details(policy);
        if !approvals.is_empty() {
            out.push_str(&UnorderedList::new(approvals).render(term));
            out.push('\n');
        }
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
