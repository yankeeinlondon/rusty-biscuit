//! Roll per-job JUnit artifacts into a package × environment × tier grid, and
//! render a merge verdict against a machine-readable baseline.
//!
//! Three subcommands share one data model:
//!
//! - `rollup` walks the downloaded CI artifacts, parses every staged JUnit
//!   document, crosses the observations with the run's resolved execution plan
//!   (or, transitionally, the legacy package policy) and the environment
//!   capability table, and emits both a Markdown grid (for
//!   `GITHUB_STEP_SUMMARY`) and a machine-readable result document.
//! - `verdict` diffs that result document against `.github/ci/ci-baseline.toml`,
//!   rules on the capability-derived policy gaps the rollup recorded, and exits
//!   non-zero when the run must not merge. It must run even when every producer
//!   job failed.
//! - `summarize` folds several area result documents into one reader-facing
//!   view. It applies no policy at all.
//!
//! The split is observation versus judgement. `rollup` never consults the
//! baseline and never excuses anything: its grid shows every cell that is not
//! green, including ones the verdict will go on to accept. All merge policy —
//! baselined failures and governed policy gaps alike — lives in `verdict`.
//!
//! ## Notes
//!
//! Cell identity is `{package, environment, tier}`, and stays so:
//! `fixes/2026-09-11-cicd-cleanup/spec.md` Design Decision 1 makes the package
//! the stored identity and the area a derived grouping field. It is never
//! derived from a GitHub job display name: when `needs:` skips a matrix job,
//! GitHub never evaluates the matrix context and reports the raw,
//! un-interpolated name expression, so nothing is recoverable from it at all.
//!
//! The absence of an artifact is therefore ambiguous on its own. `NOT
//! SCHEDULED` versus `MISSING` is resolved from the plan (the scope artifact
//! crossed with the run's affected scope), never from what happens to be on
//! disk. A cell the plan satisfied from a local receipt carries its result and
//! its origin rather than producing nothing: omitting an *execution* must never
//! omit the *cell*, which is the PR #76 seven-cell `MISSING` regression.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

/// Version of the emitted result document (`results.json`).
///
/// Version 1 was area-keyed. Version 2 keyed every identity on the package.
/// Version 3 keeps that identity and adds the area-owned result model: each
/// cell carries its derived `area`, its `origin`, the evidence behind a reused
/// result, its measured duration, and its target coverage, and the document
/// carries the accepted evidence the run was scheduled against. A consumer that
/// reads a higher version must refuse to interpret it.
const RESULT_SCHEMA_VERSION: u32 = 3;

/// Version of `.github/ci/ci-baseline.toml`. Deliberately its own constant: the
/// baseline is hand-edited policy keyed on `{package, environment, tier}` and
/// does not move when the machine-generated result document gains fields.
const BASELINE_SCHEMA_VERSION: u32 = 2;

/// Version of an expected-test manifest (`just/devops.just::_expected_manifest`).
const EXPECTED_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Version of the resolved execution plan this tool reads
/// (`scripts/ci/schema.py::RESOLVED_PLAN_SCHEMA_VERSION`).
const PLAN_SCHEMA_VERSION: u32 = 1;

/// Process exit codes. A verdict gates merging — today through the
/// transitional whole-run `ci-verdict` job, and per area through each
/// `_area-ci.yml` rollup — so a blocked run and a broken tool must be
/// distinguishable but both non-zero.
const EXIT_TOOL_ERROR: i32 = 1;
const EXIT_BLOCKED: i32 = 2;

// ---------------------------------------------------------------------------
// Domain vocabulary
// ---------------------------------------------------------------------------

/// Test tier. `Other` preserves an unrecognized tier verbatim rather than
/// discarding evidence the rollup does not yet understand.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Tier {
    L1,
    L2,
    L3,
    Browser,
    Real,
    Sanity,
    Other(String),
}

impl Tier {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "l1" => Self::L1,
            "l2" => Self::L2,
            "l3" => Self::L3,
            "browser" => Self::Browser,
            "real" => Self::Real,
            "sanity" => Self::Sanity,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L3 => "L3",
            Self::Browser => "browser",
            Self::Real => "real",
            Self::Sanity => "sanity",
            Self::Other(value) => value.as_str(),
        };
        f.write_str(text)
    }
}

impl Serialize for Tier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Tier {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::parse(&String::deserialize(deserializer)?))
    }
}

/// The closed set of cell states. A cell is exactly one of these; the counts
/// and reasons that accompany it are additive detail, never a substitute.
///
/// ## Notes
///
/// Resolution order is `NotScheduled → Missing → Fail → PolicyGap →
/// NothingToRun → Skip → Pass`, and `Pass` additionally requires at least one
/// *executed* test. `Fail` outranks `PolicyGap` because a real failure is the
/// more actionable signal and both block; the policy-gap observation is still
/// recorded in the cell's `reasons`, so nothing is lost by the ordering.
///
/// `NothingToRun` and `NotScheduled` are never conflated (R10): the first is a
/// scheduled package whose invocation selected zero tests — neither a pass
/// implying coverage nor a blocking missing result; the second is a
/// `gates = false` package carrying its governance metadata.
///
/// `AcceptedGap` and `PolicyGap` are likewise never conflated. `AcceptedGap` is
/// the plan's own decision, taken before the run from a governed, unexpired
/// `environments.json` record, and is neither a pass nor a failure. `PolicyGap`
/// is an unowned or undated absence the verdict must never excuse. Neither is
/// ever inferred from a GitHub cancellation conclusion (Design Decision 10), so
/// retry and rollup logic cannot mistake an accepted gap for an interruption.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum CellState {
    Pass,
    Fail,
    Skip,
    #[serde(rename = "NOTHING TO RUN")]
    NothingToRun,
    Missing,
    #[serde(rename = "NOT SCHEDULED")]
    NotScheduled,
    #[serde(rename = "POLICY GAP")]
    PolicyGap,
    #[serde(rename = "ACCEPTED GAP")]
    AcceptedGap,
}

impl CellState {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
            Self::NothingToRun => "NOTHING TO RUN",
            Self::Missing => "MISSING",
            Self::NotScheduled => "NOT SCHEDULED",
            Self::PolicyGap => "POLICY GAP",
            Self::AcceptedGap => "ACCEPTED GAP",
        }
    }

    /// Whether this state describes an environment that cannot host the cell's
    /// tier, governed or not. Both kinds are ruled on by the same verdict rule.
    fn is_gap(self) -> bool {
        matches!(self, Self::PolicyGap | Self::AcceptedGap)
    }

    /// Whether this state alone fails the summary gate.
    ///
    /// `Skip` is deliberately absent: a skipped tier is judged by the
    /// exact-identity skip diff, not by its cell state, so an approved skip is
    /// not double-jeopardized.
    ///
    /// ## Notes
    ///
    /// This is the *rollup's* gate, which observes rather than judges. `Fail`
    /// and `PolicyGap` both appear here even though the baseline can excuse the
    /// first and a governed, unexpired `environments.json` capability gap can
    /// excuse the second: the merge decision belongs to `verdict`, and keeping
    /// this predicate policy-free is what makes the rollup's "Cells failing the
    /// summary gate" table an honest inventory of everything not green.
    ///
    /// `AcceptedGap` is absent because it is not an observation of something
    /// missing: the plan decided, from an owned and unexpired policy entry,
    /// that the cell will not execute, and the grid renders it in its own
    /// section with that governance. The verdict still re-checks its expiry —
    /// `policy_gap_findings` blocks an expired or incomplete one without
    /// consulting this predicate — because the verdict runs when the scope job
    /// did not.
    fn blocks(self) -> bool {
        matches!(self, Self::Fail | Self::Missing | Self::PolicyGap)
    }
}

impl fmt::Display for CellState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Per-record and per-cell test tallies.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct Counts {
    total: u32,
    passed: u32,
    failed: u32,
    skipped: u32,
    errored: u32,
}

impl Counts {
    fn add(&mut self, other: &Counts) {
        self.total += other.total;
        self.passed += other.passed;
        self.failed += other.failed;
        self.skipped += other.skipped;
        self.errored += other.errored;
    }

    /// Failures and errors both mean "a test ran and did not pass".
    fn bad(&self) -> u32 {
        self.failed + self.errored
    }
}

/// Where a cell's result came from.
///
/// ## Notes
///
/// The vocabulary is the resolved plan's (`scripts/ci/schema.py::ORIGINS`), so
/// the document that scheduled the run and the document that reports it name a
/// reused result the same way. `PriorLocal` is a receipt from an older head
/// accepted through gate-input identity; `Local` is the outgoing head's own.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Origin {
    #[default]
    Ci,
    Local,
    PriorLocal,
    #[serde(rename = "none")]
    Unproduced,
}

impl Origin {
    fn label(self) -> &'static str {
        match self {
            Self::Ci => "ci",
            Self::Local => "local",
            Self::PriorLocal => "prior-local",
            Self::Unproduced => "none",
        }
    }

    /// Whether a result of this origin was produced off this run, so no CI job
    /// is expected to have uploaded anything for it.
    fn is_reused(self) -> bool {
        matches!(self, Self::Local | Self::PriorLocal)
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// What a reused cell's result can be traced back to.
///
/// CI cannot link to a file on a developer host, so the link is the Git notes
/// ref that carried the receipt (Design Decision 6). The full report stays on
/// the producing host, and `report` records where, so a reviewer can ask for it.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct Evidence {
    /// `refs/notes/ci-local/<environment>`.
    #[serde(rename = "ref")]
    reference: String,
    /// The note commit the receipt was read from, when the verifier recorded it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    commit: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    host: String,
    /// Counts and duration as the receipt recorded them, or
    /// `not recorded (v1 receipt)` for a legacy note.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    measurements: String,
    /// The producing host's report directory, when the receipt named one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    report: String,
}

// ---------------------------------------------------------------------------
// Machine-readable result schema
// ---------------------------------------------------------------------------

/// One nextest invocation. Package identity is preserved here so a count can
/// be traced back to the invocation that produced it without parsing any
/// display name.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RunRecord {
    package: String,
    environment: String,
    tier: Tier,
    /// Downloaded artifact directory this record came from.
    artifact: String,
    /// True when identity was incomplete in the manifest record (a local-dev
    /// run stages no environment, for instance).
    degraded: bool,
    report_present: bool,
    exit_code: i64,
    duration_s: u64,
    counts: Counts,
    failed_tests: Vec<String>,
    skipped_tests: Vec<String>,
    /// Why this record carries no usable counts, when it does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parse_error: Option<String>,
    /// Identities that passed. Deliberately not serialized: the repo has
    /// thousands of tests and no consumer needs them individually, but the
    /// expected-test diff must be able to ask "did this test produce *any*
    /// result", and a pass is a result.
    #[serde(skip)]
    passed_identities: Vec<String>,
}

impl RunRecord {
    fn cell_key(&self) -> CellKey {
        CellKey {
            package: self.package.clone(),
            environment: self.environment.clone(),
            tier: self.tier.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
struct CellKey {
    package: String,
    environment: String,
    tier: Tier,
}

impl fmt::Display for CellKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.package, self.environment, self.tier)
    }
}

/// The capability-table gap that explains a `POLICY GAP` cell, carried into
/// `results.json` verbatim.
///
/// ## Notes
///
/// `verdict` reads only the result document — never `environments.json` — so
/// the accountability fields have to travel with the cell. Rendering the gap
/// into a prose `reasons` line is not enough: the verdict has to decide on
/// `owner`, `reason`, and `expiry` individually, and re-parsing them out of
/// prose would be a second, drift-prone encoding of the same policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct DeclaredGap {
    owner: String,
    reason: String,
    /// `""` when undeclared. See [`policy_gap_findings`].
    #[serde(default)]
    expiry: String,
    /// The capability whose governed absence produced the gap (`tmux`,
    /// `headless_browser`, …). Empty on a document written before the field.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    capability: String,
    /// Where the acceptance is declared, so a reader can reach it without being
    /// told the path.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    policy: String,
    /// The tracked work that closes the gap, when the policy entry names it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    closes: String,
}

/// How acceptance is changed or revoked. Spec section 6 requires the
/// instructions to travel with the cell; the path is the one authority
/// `verdict` and the grid both point at.
const POLICY_LINK: &str = ".github/ci/environments.json";
const REVOCATION_INSTRUCTIONS: &str = "to revoke, set the capability to `true` \
    (or delete the governed record) in .github/ci/environments.json; the cell \
    then blocks its area until the coverage exists";

impl DeclaredGap {
    fn owner_or_unassigned(&self) -> &str {
        if self.owner.trim().is_empty() {
            "unassigned"
        } else {
            self.owner.as_str()
        }
    }

    fn policy_link(&self) -> &str {
        if self.policy.trim().is_empty() {
            POLICY_LINK
        } else {
            self.policy.as_str()
        }
    }

    /// The `reasons` line rendered into the grid's "why" column.
    fn describe(&self) -> String {
        let expiry = if self.expiry.trim().is_empty() {
            "no expiry".to_owned()
        } else {
            format!("expires {}", self.expiry)
        };
        format!(
            "{} (owner {}, {expiry})",
            self.reason,
            self.owner_or_unassigned()
        )
    }

    /// Everything spec section 6 requires an accepted gap to show: who owns it,
    /// why, until when, where it is declared, what closes it, and how to revoke
    /// it.
    fn describe_acceptance(&self) -> String {
        let mut out = self.describe();
        if !self.capability.is_empty() {
            out.push_str(&format!("; capability `{}`", self.capability));
        }
        out.push_str(&format!("; policy {}", self.policy_link()));
        if !self.closes.is_empty() {
            out.push_str(&format!("; closed by {}", self.closes));
        }
        out.push_str(&format!("; {REVOCATION_INSTRUCTIONS}"));
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Cell {
    #[serde(flatten)]
    key: CellKey,
    /// The package's package area, derived at plan time and carried here for
    /// grouping only. Package stays the identity (Design Decision 1); this is
    /// never a key. `""` when the producing document predates area derivation.
    #[serde(default)]
    area: String,
    state: CellState,
    /// Whether this result was produced by this run or reused from a receipt.
    #[serde(default)]
    origin: Origin,
    counts: Counts,
    /// Wall time of the executions behind this cell, summed across its records,
    /// or the receipt's recorded duration for a reused cell.
    #[serde(default)]
    duration_s: u64,
    /// Cargo target kinds the cell's gate covers, from the plan. Empty when the
    /// rollup ran without a plan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    target_kinds: Vec<String>,
    /// Which gate produced this cell's compile coverage, from the plan. An
    /// archive-only environment names the runner that built its archive rather
    /// than claiming a compile it never performed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    compile_coverage_from: String,
    /// What satisfied a reused cell. Absent on a CI-origin cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evidence: Option<Evidence>,
    /// Whether policy scheduled this cell for this run.
    scheduled: bool,
    /// Exact identities of every test observed as skipped, plus (when an
    /// expected-test manifest was supplied) every expected test absent from the
    /// report.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skipped_tests: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    failed_tests: Vec<String>,
    /// True when the skip set could not be completed because no expected-test
    /// manifest was supplied for this environment and tier.
    skip_evidence_degraded: bool,
    /// The capability-table gap covering this cell, when one is governed.
    /// `None` on a `POLICY GAP` cell means the gap is *ungoverned* — the
    /// capability is recorded as plain absent — which the verdict must never
    /// excuse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    declared_gap: Option<DeclaredGap>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    reasons: Vec<String>,
    /// Indices into [`Rollup::records`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    records: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Rollup {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    /// Packages policy considered in scope for this run.
    scope: Vec<String>,
    /// The package areas this document covers, derived from its cells. A
    /// grouping field, not an identity.
    #[serde(default)]
    areas: Vec<String>,
    /// The `--area` narrowing applied to this document, empty when it covers
    /// the whole run. A narrowed document is one area's own slice: it applies
    /// that area's baseline, gaps, and missing-cell rule and nobody else's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    area_scope: Vec<String>,
    /// The verified evidence this run was scheduled against, one entry per
    /// reused cell. Written so the reporting path cannot expect a CI result for
    /// a cell a receipt already satisfied — the PR #76 regression — and so a
    /// reviewer can reach the receipt behind every non-CI result.
    #[serde(default)]
    accepted_evidence: Vec<AcceptedEvidence>,
    /// True when the scope was inferred from the artifacts on disk instead of
    /// being supplied. An inferred scope cannot see a package that produced
    /// nothing at all, which is exactly the case `MISSING` exists to catch.
    scope_degraded: bool,
    /// Every `{package, environment, tier}` policy scheduled for this run —
    /// the expected cells, before any evidence. Scope is per gate, so an
    /// in-scope package may legitimately have no test tier here (it was
    /// reached only through a lint-global input); the verdict consults this
    /// to tell "not scheduled" from "scheduled and vanished". `None` on a
    /// document written before the field existed: unknown, so fail closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scheduled: Option<Vec<CellKey>>,
    records: Vec<RunRecord>,
    cells: Vec<Cell>,
}

impl Rollup {
    /// Whether policy scheduled `key` for this run. `None` when the document
    /// predates the schedule field and cannot say.
    fn scheduled(&self, key: &CellKey) -> Option<bool> {
        self.scheduled.as_ref().map(|keys| keys.contains(key))
    }

    fn cell(&self, key: &CellKey) -> Option<&Cell> {
        self.cells.iter().find(|cell| &cell.key == key)
    }

    /// This document narrowed to one area's own slice.
    ///
    /// ## Notes
    ///
    /// `scope` is narrowed with the cells. A baseline entry naming another
    /// area's package then has the standing of an out-of-scope entry — ignored
    /// with a note — rather than a vanished result, which is what keeps one
    /// area's failure from blocking another's outcome (spec section 5).
    fn narrowed(&self, areas: &BTreeSet<String>) -> Self {
        let cells: Vec<Cell> = self
            .cells
            .iter()
            .filter(|cell| areas.contains(&cell.area))
            .cloned()
            .collect();
        let packages: BTreeSet<&String> = cells.iter().map(|cell| &cell.key.package).collect();
        Self {
            schema_version: self.schema_version,
            run_id: self.run_id.clone(),
            scope: self
                .scope
                .iter()
                .filter(|package| packages.contains(package))
                .cloned()
                .collect(),
            areas: derived_areas(&cells),
            area_scope: areas.iter().cloned().collect(),
            accepted_evidence: self
                .accepted_evidence
                .iter()
                .filter(|entry| areas.contains(&entry.area))
                .cloned()
                .collect(),
            scope_degraded: self.scope_degraded,
            scheduled: self.scheduled.as_ref().map(|keys| {
                keys.iter()
                    .filter(|key| packages.contains(&key.package))
                    .cloned()
                    .collect()
            }),
            records: self.records.clone(),
            cells,
        }
    }
}

/// The areas a set of cells covers, deduplicated and ordered.
fn derived_areas(cells: &[Cell]) -> Vec<String> {
    cells
        .iter()
        .filter(|cell| !cell.area.is_empty())
        .map(|cell| cell.area.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// One cell the plan satisfied from a receipt instead of scheduling.
///
/// Carried in the result document so the evidence accepted for *scheduling* is
/// the same evidence that satisfies the *result* cell. Recalculating a
/// conflicting expectation in the reporting path is what produced the PR #76
/// seven-cell regression (spec section 3.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AcceptedEvidence {
    #[serde(flatten)]
    key: CellKey,
    area: String,
    origin: Origin,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    measurements: String,
    evidence: Evidence,
}

// ---------------------------------------------------------------------------
// Producer input: manifest.jsonl, staged JUnit, producer status
// ---------------------------------------------------------------------------

/// One record of `$STAGE/manifest.jsonl`, written by `just/devops.just`'s
/// `_stage_junit`. `environment` is `""` when unset (local dev) — never
/// invented by the producer, so never inferred here either.
#[derive(Clone, Debug, Deserialize)]
struct ManifestRecord {
    tier: String,
    package: String,
    xml: String,
    #[serde(default)]
    exit_code: i64,
    #[serde(default)]
    environment: String,
    #[serde(default)]
    duration_s: u64,
    #[serde(default)]
    report_present: bool,
}

/// Explicit status for a producer job, so a job that failed before it could
/// stage anything is distinguishable from one that was never scheduled.
///
/// Uploaded as `status-<package>-<job>[-<environment>]/status.json`.
#[derive(Clone, Debug, Deserialize)]
struct ProducerStatus {
    package: String,
    job: String,
    /// GitHub `needs.<job>.result`: `success`, `failure`, `cancelled`, `skipped`.
    result: String,
    #[serde(default)]
    environment: Option<String>,
    /// Why this producer has no evidence, when it knows. A dead WSL2 guest and a
    /// tier that genuinely ran no tests both upload nothing; only the producer
    /// can tell them apart, so it says so here rather than leaving the rollup to
    /// render one indistinguishable blank cell for both.
    #[serde(default)]
    detail: Option<String>,
    /// The companion-suite step's outcome (`success`, `failure`, `skipped`),
    /// recorded by every job of a package that DECLARES a companion suite —
    /// not only on failure, because a skipped companion leaves no other
    /// evidence and must downgrade the cell just as a failed one does.
    #[serde(default)]
    companion: Option<String>,
}

/// Tests the target environment actually compiled, generated *on* that
/// environment (`cargo nextest list`). Without it, a test absent from a JUnit
/// report is ambiguous between `#[cfg]`-absent (N/A) and skipped-at-runtime.
#[derive(Clone, Debug, Deserialize)]
struct ExpectedManifest {
    #[serde(default)]
    schema_version: u32,
    environment: String,
    tier: Tier,
    /// package name → fully-qualified test identities (`<suite>::<test>`).
    packages: BTreeMap<String, Vec<String>>,
}

// ---------------------------------------------------------------------------
// Policy: the scope artifact's resolved package policy + environments.json
// ---------------------------------------------------------------------------

/// One impacted package's resolved CI policy, as emitted by the scope job in
/// its `ci-scope` artifact. Derived from the package's own
/// `[package.metadata.ci]` manifest block; `affected_scope.py` owns and
/// validates that schema.
#[derive(Clone, Debug, Deserialize)]
struct PackagePolicy {
    package: String,
    /// The package's area, derived by the planner. Grouping only.
    #[serde(default)]
    area: String,
    #[serde(default = "default_true")]
    gates: bool,
    /// CI-gating tiers this package owns. Always contains L1 for a gating
    /// package; L2 and browser are declared opt-ins.
    #[serde(default = "default_tiers")]
    tiers: Vec<Tier>,
    /// L2 terminal backends the package's tests require.
    #[serde(default)]
    l2_backends: Vec<String>,
    /// Non-Cargo companion suites the package owns (e.g. `homelab-frontend`).
    /// The rollup needs the declaration itself: a green Rust JUnit report must
    /// not hide a companion suite that never ran (R12).
    #[serde(default)]
    companion_suites: Vec<String>,
    /// Governance for a `gates = false` package. Non-gating is never inferred
    /// from zero observed tests; it is always this explicit, owned, dated
    /// record.
    #[serde(default)]
    exclusion: Option<Exclusion>,
}

/// Why a package does not gate, who owns closing that, and when the exclusion
/// runs out.
#[derive(Clone, Debug, Deserialize)]
struct Exclusion {
    exclusion_class: String,
    owner: String,
    reason: String,
    #[serde(default)]
    expiry: Option<String>,
}

impl Exclusion {
    fn describe(&self) -> String {
        let expiry = match &self.expiry {
            Some(expiry) => format!(", expires {expiry}"),
            None => String::new(),
        };
        format!(
            "{} ({} — owner {}{})",
            self.reason, self.exclusion_class, self.owner, expiry
        )
    }
}

/// The scope artifact (`ci-scope/scope.json`). Only `policy` is read here.
#[derive(Clone, Debug, Deserialize)]
struct PolicyDoc {
    policy: Vec<PackagePolicy>,
}

/// `.github/ci/environments.json` — the one versioned capability table.
#[derive(Clone, Debug, Deserialize)]
struct EnvironmentsDoc {
    schema_version: u32,
    environments: Vec<Environment>,
}

#[derive(Clone, Debug, Deserialize)]
struct Environment {
    name: String,
    capabilities: BTreeMap<String, Capability>,
}

/// A capability is either a plain boolean or, for a governed unavailability,
/// an object carrying the accountability fields the verdict rules on.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum Capability {
    Available(bool),
    Governed {
        available: bool,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        owner: String,
        #[serde(default)]
        expiry: String,
        /// The tracked work that closes the gap, when the record names it.
        #[serde(default)]
        closes: String,
    },
}

impl Capability {
    fn available(&self) -> bool {
        match self {
            Self::Available(value) => *value,
            Self::Governed { available, .. } => *available,
        }
    }

    /// The governance record, when this unavailability is governed. A plain
    /// `false` is an UNGOVERNED absence — the gap it produces is never
    /// excused, exactly like an undeclared gap before.
    fn governed_gap(&self, capability: &str) -> Option<DeclaredGap> {
        match self {
            Self::Governed {
                available: false,
                reason,
                owner,
                expiry,
                closes,
            } => Some(DeclaredGap {
                owner: owner.clone(),
                reason: reason.clone(),
                expiry: expiry.clone(),
                capability: capability.to_owned(),
                policy: POLICY_LINK.to_owned(),
                closes: closes.clone(),
            }),
            _ => None,
        }
    }
}

impl Environment {
    fn capable(&self, capability: &str) -> bool {
        self.capabilities
            .get(capability)
            .map(Capability::available)
            .unwrap_or(false)
    }

    fn gap_for(&self, capability: &str) -> Option<DeclaredGap> {
        self.capabilities.get(capability)?.governed_gap(capability)
    }
}

fn default_true() -> bool {
    true
}

fn default_tiers() -> Vec<Tier> {
    vec![Tier::L1]
}

/// Whether the environment can host a cell's tier, and if not, whether the
/// absence is governed.
#[derive(Clone, Debug)]
enum GapStatus {
    /// The capability table carries owner, reason, and expiry for this
    /// absence. The verdict may accept it.
    Governed(DeclaredGap),
    /// A plain `false` in the capability table: nobody owns this absence, so
    /// the verdict must never excuse it.
    Ungoverned,
}

/// A cell policy says should exist.
#[derive(Clone, Debug)]
struct ExpectedCell {
    key: CellKey,
    /// The package's area. Grouping only; `""` without a plan.
    area: String,
    /// L2 backends this package's tests require, empty when the tier is
    /// backend-agnostic.
    backends: Vec<String>,
    /// Set when the environment cannot host this cell's tier.
    gap: Option<GapStatus>,
    /// Set when the package is `gates = false`: the cell renders NOT SCHEDULED
    /// with this governance metadata rather than running.
    exclusion: Option<Exclusion>,
    /// Companion suites expected to run alongside this cell (L1 only, and only
    /// on a Node-capable environment). Non-empty means the cell's producer
    /// status must report the companion step's success; a skipped companion
    /// downgrades an otherwise-green cell (R12).
    companion_suites: Vec<String>,
    /// The result a receipt already established for this cell. Present only on
    /// the plan path; a cell carrying one expects no CI job to have run.
    reused: Option<ReusedResult>,
    /// True when the plan accepted this cell's gap from a governed, unexpired
    /// policy entry. The gap itself is in `gap`.
    accepted_gap: bool,
    /// The recorded execution constraint that stopped this cell, when one did.
    prohibition: Option<String>,
    /// Cargo target kinds the gate covers, from the plan.
    target_kinds: Vec<String>,
    /// Where the cell's compile coverage came from, from the plan.
    compile_coverage_from: String,
}

impl ExpectedCell {
    /// A cell with no plan-side detail, for the legacy policy path.
    fn new(key: CellKey, area: String) -> Self {
        Self {
            key,
            area,
            backends: Vec::new(),
            gap: None,
            exclusion: None,
            companion_suites: Vec::new(),
            reused: None,
            accepted_gap: false,
            prohibition: None,
            target_kinds: Vec::new(),
            compile_coverage_from: String::new(),
        }
    }
}

/// A result a validation receipt established off this run.
#[derive(Clone, Debug)]
struct ReusedResult {
    origin: Origin,
    /// `false` when the receipt recorded a complete failure for the cell. A
    /// complete failed local result stays a failure (spec section 3.5).
    passed: bool,
    counts: Counts,
    duration_s: u64,
    failed_tests: Vec<String>,
    evidence: Evidence,
}

/// Cross the resolved package policy with the run's affected scope to derive
/// every cell that should have produced evidence.
///
/// ## Notes
///
/// An L2 or browser tier is expected on **every** environment, not only the
/// ones that can host it. Omitting the unhostable ones would make them vanish
/// from the grid, which is the failure mode `POLICY GAP` exists to prevent.
/// L2 is hostable where ANY declared backend is (each backend is looked up as
/// its own capability, so `kitty`/`apple-terminal` get an environment axis
/// rather than the tier hardcoding `tmux`); a gap arises only when no
/// declared backend can be hosted, governed by that backend's own capability
/// record.
/// The visible, governed `NOT SCHEDULED` cells of a `gates = false` package —
/// never a pass, never a silent absence.
///
/// ## Notes
///
/// A policy whose exclusion block is absent still renders (backstop):
/// `affected_scope.py` validation is supposed to reject that shape, and the
/// rollup must not let a config-side regression vanish a package from the grid.
/// The resolved plan gives a non-gating package no cells at all, so this stays
/// keyed off the package policy document under both input paths.
fn exclusion_cells(policy: &PackagePolicy, environments: &[Environment]) -> Vec<ExpectedCell> {
    let exclusion = policy.exclusion.clone().unwrap_or_else(|| Exclusion {
        exclusion_class: "ungoverned".to_owned(),
        owner: "unassigned".to_owned(),
        reason: "`gates = false` without exclusion metadata; the scope \
                 job's validation is supposed to reject this"
            .to_owned(),
        expiry: None,
    });
    environments
        .iter()
        .map(|environment| ExpectedCell {
            exclusion: Some(exclusion.clone()),
            ..ExpectedCell::new(
                CellKey {
                    package: policy.package.clone(),
                    environment: environment.name.clone(),
                    tier: Tier::L1,
                },
                policy.area.clone(),
            )
        })
        .collect()
}

fn expected_cells(
    policies: &[PackagePolicy],
    scope: &BTreeSet<String>,
    environments: &[Environment],
) -> Vec<ExpectedCell> {
    let mut expected = Vec::new();

    for policy in policies {
        if !scope.contains(&policy.package) {
            continue;
        }

        if !policy.gates {
            expected.extend(exclusion_cells(policy, environments));
            continue;
        }

        for environment in environments {
            for tier in &policy.tiers {
                let (backends, gap) = match tier {
                    Tier::L2 => {
                        let gap = if policy
                            .l2_backends
                            .iter()
                            .any(|backend| environment.capable(backend))
                        {
                            None
                        } else {
                            Some(
                                match policy
                                    .l2_backends
                                    .iter()
                                    .find_map(|backend| environment.gap_for(backend))
                                {
                                    Some(governed) => GapStatus::Governed(governed),
                                    None => GapStatus::Ungoverned,
                                },
                            )
                        };
                        (policy.l2_backends.clone(), gap)
                    }
                    Tier::Browser => {
                        // Same rule as L2: an environment that cannot host the
                        // tier renders an explicit POLICY GAP (governed by the
                        // `headless_browser` capability record), never a cell
                        // that silently disappeared from the grid.
                        let gap = if environment.capable("headless_browser") {
                            None
                        } else {
                            Some(match environment.gap_for("headless_browser") {
                                Some(governed) => GapStatus::Governed(governed),
                                None => GapStatus::Ungoverned,
                            })
                        };
                        (Vec::new(), gap)
                    }
                    _ => (Vec::new(), None),
                };
                let companion_suites = if *tier == Tier::L1
                    && !policy.companion_suites.is_empty()
                    && environment.capable("node_pnpm")
                {
                    policy.companion_suites.clone()
                } else {
                    Vec::new()
                };
                expected.push(ExpectedCell {
                    backends,
                    gap,
                    companion_suites,
                    ..ExpectedCell::new(
                        CellKey {
                            package: policy.package.clone(),
                            environment: environment.name.clone(),
                            tier: tier.clone(),
                        },
                        policy.area.clone(),
                    )
                });
            }
        }
    }

    expected.sort_by(|left, right| left.key.cmp(&right.key));
    expected
}

// ---------------------------------------------------------------------------
// Policy: the canonical resolved execution plan
// ---------------------------------------------------------------------------

/// The resolved execution plan (`scripts/ci/affected_scope.py --plan-out`).
///
/// ## Notes
///
/// The planner already crossed the package policy with the capability table and
/// with the verified evidence, so reading the plan is a *translation*, not a
/// second calculation: this file derives no expectation the plan does not state.
/// That is what spec section 3.1 requires — evidence accepted for scheduling
/// must satisfy the corresponding result cell, and a reporting path that
/// recomputes the expectation can disagree with the scheduler, which is exactly
/// how PR #76 filed seven passing cells as `MISSING`.
///
/// Only the fields this tool consumes are declared; the planner owns the rest.
/// `plan_fields_match_the_frozen_contract` pins these names against
/// `.github/ci/schemas/contract.json` so a rename cannot land silently.
#[derive(Clone, Debug, Deserialize)]
struct ResolvedPlan {
    schema_version: u32,
    cells: Vec<PlanCell>,
    packages: Vec<PlanPackage>,
}

/// A package the plan selected. Only the name is read here: the area travels on
/// every cell, and a `gates = false` package's governance stays in the policy
/// document, which is the only place that carries its owner and expiry.
#[derive(Clone, Debug, Deserialize)]
struct PlanPackage {
    package: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PlanCell {
    package: String,
    area: String,
    environment: String,
    /// `lint`, `check`, `L1`, `L2`, or `browser`.
    gate: String,
    /// `execute`, `reuse`, or `omit`.
    execution: String,
    origin: Origin,
    /// `pending`, `reused`, `accepted-gap`, or `prohibited`.
    state: String,
    #[serde(default)]
    target_kinds: Vec<String>,
    #[serde(default)]
    compile_coverage_from: String,
    #[serde(default)]
    evidence: Option<PlanEvidence>,
    #[serde(default)]
    gap: Option<PlanGap>,
    #[serde(default)]
    prohibition: Option<PlanProhibition>,
}

/// A plan cell's accepted evidence.
///
/// Two shapes reach this field. A per-cell acceptance carries the whole receipt
/// cell (outcome, counts, duration, report) plus the note it came from; the
/// transitional whole-environment acceptance carries only an origin and a ref
/// string. Every measurement is therefore optional, and an acceptance with no
/// recorded outcome is a version-1 receipt — pass-only by contract (spec
/// section 3.6), rendered with its measurements marked unrecorded.
#[derive(Clone, Debug, Deserialize)]
struct PlanEvidence {
    #[serde(default)]
    origin: Option<Origin>,
    #[serde(default)]
    outcome: Option<String>,
    #[serde(default)]
    counts: Option<Counts>,
    #[serde(default)]
    duration_s: Option<f64>,
    #[serde(default)]
    measurements: Option<String>,
    #[serde(default)]
    failed_tests: Vec<String>,
    #[serde(default)]
    report: Option<String>,
    #[serde(default)]
    evidence: Option<EvidenceLink>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum EvidenceLink {
    /// The whole-environment shape: just the notes ref.
    Reference(String),
    Detail {
        #[serde(default, rename = "ref")]
        reference: String,
        #[serde(default)]
        commit: String,
        #[serde(default)]
        host: Option<ReceiptHost>,
    },
}

#[derive(Clone, Debug, Deserialize)]
struct ReceiptHost {
    #[serde(default)]
    os: String,
    #[serde(default)]
    kernel: String,
    #[serde(default)]
    report_dir: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PlanGap {
    #[serde(default)]
    capability: String,
    governed: bool,
    #[serde(default)]
    policy: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    expiry: String,
    #[serde(default)]
    closes: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PlanProhibition {
    #[serde(default)]
    owner: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    expiry: String,
}

impl PlanEvidence {
    fn link(&self) -> Evidence {
        match &self.evidence {
            Some(EvidenceLink::Reference(reference)) => Evidence {
                reference: reference.clone(),
                measurements: self.measurements.clone().unwrap_or_default(),
                report: self.report.clone().unwrap_or_default(),
                ..Evidence::default()
            },
            Some(EvidenceLink::Detail {
                reference,
                commit,
                host,
            }) => Evidence {
                reference: reference.clone(),
                commit: commit.clone(),
                host: host
                    .as_ref()
                    .map(|host| {
                        let kernel = if host.kernel.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", host.kernel)
                        };
                        format!("{}{kernel}", host.os).trim().to_owned()
                    })
                    .unwrap_or_default(),
                measurements: self.measurements.clone().unwrap_or_default(),
                report: host
                    .as_ref()
                    .map(|host| host.report_dir.clone())
                    .filter(|dir| !dir.is_empty())
                    .or_else(|| self.report.clone())
                    .unwrap_or_default(),
            },
            None => Evidence {
                measurements: self.measurements.clone().unwrap_or_default(),
                report: self.report.clone().unwrap_or_default(),
                ..Evidence::default()
            },
        }
    }
}

/// Turn the plan's cells into the expectations the rollup classifies against.
///
/// ## Errors
///
/// A plan from another schema generation is refused rather than partially
/// interpreted: an unknown vocabulary would silently reclassify cells.
fn plan_expected_cells(plan: &ResolvedPlan) -> Result<Vec<ExpectedCell>> {
    if plan.schema_version != PLAN_SCHEMA_VERSION {
        bail!(
            "resolved plan schema_version {} is not the {PLAN_SCHEMA_VERSION} this tool \
             reads; regenerate it with scripts/ci/affected_scope.py",
            plan.schema_version
        );
    }

    let mut expected = Vec::new();
    for cell in &plan.cells {
        let key = CellKey {
            package: cell.package.clone(),
            environment: cell.environment.clone(),
            tier: Tier::parse(&cell.gate),
        };
        let mut expectation = ExpectedCell::new(key, cell.area.clone());
        expectation.target_kinds = cell.target_kinds.clone();
        expectation.compile_coverage_from = cell.compile_coverage_from.clone();

        if let Some(gap) = &cell.gap {
            let declared = DeclaredGap {
                owner: gap.owner.clone(),
                reason: gap.reason.clone(),
                expiry: gap.expiry.clone(),
                capability: gap.capability.clone(),
                policy: gap.policy.clone(),
                closes: gap.closes.clone(),
            };
            // A gap the plan did not govern is never accepted here either. The
            // `accepted-gap` state without a governed record is treated as
            // ungoverned rather than trusted: acceptance that cannot name an
            // owner and an expiry is a permanent exclusion wearing a temporary
            // label.
            if gap.governed {
                expectation.gap = Some(GapStatus::Governed(declared));
                expectation.accepted_gap = cell.state == "accepted-gap";
            } else {
                expectation.gap = Some(GapStatus::Ungoverned);
            }
        } else if cell.state == "accepted-gap" {
            expectation.gap = Some(GapStatus::Ungoverned);
        }

        if let Some(prohibition) = &cell.prohibition {
            expectation.prohibition = Some(format!(
                "a recorded execution constraint forbids {}: {} (owner {}{})",
                cell.environment,
                prohibition.reason,
                if prohibition.owner.is_empty() {
                    "unassigned"
                } else {
                    prohibition.owner.as_str()
                },
                if prohibition.expiry.is_empty() {
                    String::new()
                } else {
                    format!(", expires {}", prohibition.expiry)
                }
            ));
        }

        if cell.execution == "reuse" {
            let evidence = cell.evidence.as_ref();
            let counts = evidence.and_then(|record| record.counts).unwrap_or_default();
            let measured = evidence.is_some_and(|record| record.outcome.is_some());
            let mut link = evidence.map(PlanEvidence::link).unwrap_or_default();
            if link.measurements.is_empty() {
                link.measurements = if measured {
                    format!(
                        "{} test(s), {} failed, {}s",
                        counts.total,
                        counts.bad(),
                        evidence.and_then(|record| record.duration_s).unwrap_or(0.0) as u64
                    )
                } else {
                    // Spec section 3.6's verbatim text for a version-1 receipt.
                    "not recorded (v1 receipt)".to_owned()
                };
            }
            if link.reference.is_empty() {
                link.reference = format!("refs/notes/ci-local/{}", cell.environment);
            }
            expectation.reused = Some(ReusedResult {
                origin: evidence
                    .and_then(|record| record.origin)
                    .unwrap_or(cell.origin),
                passed: evidence.and_then(|record| record.outcome.as_deref()) != Some("fail"),
                counts,
                duration_s: evidence
                    .and_then(|record| record.duration_s)
                    .unwrap_or(0.0)
                    .max(0.0) as u64,
                failed_tests: evidence
                    .map(|record| record.failed_tests.clone())
                    .unwrap_or_default(),
                evidence: link,
            });
        }

        expected.push(expectation);
    }

    expected.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(expected)
}


// ---------------------------------------------------------------------------
// JUnit parsing
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct JunitReport {
    counts: Counts,
    failed_tests: Vec<String>,
    skipped_tests: Vec<String>,
    passed_tests: Vec<String>,
}

/// Parse a nextest JUnit document into per-test identities.
///
/// Counts are derived from the `<testcase>` elements actually present, not from
/// the `<testsuite>` summary attributes: the identities are what the
/// exact-identity skip diff needs, and a document whose attributes disagree
/// with its contents should be believed on its contents.
///
/// ## Errors
///
/// Returns an error for malformed or truncated XML. A truncated document is a
/// hard error rather than a partial result — silently accepting the prefix of a
/// report would let a killed job look like a small passing one.
fn parse_junit(xml: &str) -> Result<JunitReport> {
    let mut reader = Reader::from_str(xml);
    let mut report = JunitReport::default();
    let mut buf = Vec::new();
    // Depth-tracked so `<failure>` inside `<system-out>` text cannot be
    // mistaken for a real failure element.
    let mut suite_name = String::new();
    let mut open_case: Option<OpenCase> = None;
    let mut depth: usize = 0;
    let mut saw_root = false;

    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event,
            Err(err) => {
                let position = reader.buffer_position();
                bail!("malformed JUnit XML at byte {position}: {err}");
            }
        };

        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let empty = matches!(event, Event::Empty(_));
                let name = element.local_name();
                match name.as_ref() {
                    b"testsuites" => saw_root = true,
                    b"testsuite" => {
                        suite_name = attribute(element, b"name")?.unwrap_or_default();
                    }
                    b"testcase" => {
                        let case = attribute(element, b"name")?.unwrap_or_default();
                        let classname = attribute(element, b"classname")?.unwrap_or_default();
                        let owner = if suite_name.is_empty() {
                            classname
                        } else {
                            suite_name.clone()
                        };
                        let identity = if owner.is_empty() {
                            case
                        } else {
                            format!("{owner}::{case}")
                        };
                        let open = OpenCase {
                            identity,
                            outcome: CaseOutcome::Passed,
                        };
                        if empty {
                            record_case(&mut report, open);
                        } else {
                            open_case = Some(open);
                        }
                    }
                    b"failure" => mark(&mut open_case, CaseOutcome::Failed),
                    b"error" => mark(&mut open_case, CaseOutcome::Errored),
                    b"skipped" => mark(&mut open_case, CaseOutcome::Skipped),
                    _ => {}
                }
                if !empty {
                    depth += 1;
                }
            }
            Event::End(ref element) => {
                depth = depth.saturating_sub(1);
                if element.local_name().as_ref() == b"testcase" {
                    if let Some(case) = open_case.take() {
                        record_case(&mut report, case);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if !saw_root {
        bail!("not a JUnit document: no <testsuites> element");
    }
    if depth != 0 {
        bail!("truncated JUnit XML: {depth} unclosed element(s)");
    }

    Ok(report)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaseOutcome {
    Passed,
    Failed,
    Errored,
    Skipped,
}

#[derive(Clone, Debug)]
struct OpenCase {
    identity: String,
    outcome: CaseOutcome,
}

fn mark(open_case: &mut Option<OpenCase>, outcome: CaseOutcome) {
    if let Some(case) = open_case.as_mut() {
        // A case carrying both <failure> and <error> keeps the first verdict.
        if case.outcome == CaseOutcome::Passed {
            case.outcome = outcome;
        }
    }
}

fn record_case(report: &mut JunitReport, case: OpenCase) {
    report.counts.total += 1;
    match case.outcome {
        CaseOutcome::Passed => {
            report.counts.passed += 1;
            report.passed_tests.push(case.identity);
        }
        CaseOutcome::Failed => {
            report.counts.failed += 1;
            report.failed_tests.push(case.identity);
        }
        CaseOutcome::Errored => {
            report.counts.errored += 1;
            report.failed_tests.push(case.identity);
        }
        CaseOutcome::Skipped => {
            report.counts.skipped += 1;
            report.skipped_tests.push(case.identity);
        }
    }
}

fn attribute(element: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Result<Option<String>> {
    for attr in element.attributes() {
        let attr = attr.context("malformed XML attribute")?;
        if attr.key.local_name().as_ref() == key {
            let value = attr.unescape_value().context("malformed XML attribute")?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Artifact walking
// ---------------------------------------------------------------------------

/// One downloaded CI artifact directory.
#[derive(Clone, Debug)]
struct ArtifactDir {
    name: String,
    path: PathBuf,
}

fn list_artifact_dirs(root: &Path) -> Result<Vec<ArtifactDir>> {
    let mut dirs = Vec::new();
    let entries = fs::read_dir(root)
        .with_context(|| format!("failed to read artifacts root {}", root.display()))?;

    for entry in entries {
        let entry = entry.context("failed to read artifacts root entry")?;
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        dirs.push(ArtifactDir {
            name,
            path: entry.path(),
        });
    }

    dirs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(dirs)
}

/// Join a manifest's forward-slash-relative `xml` path onto a base directory
/// without ever constructing a `/`-joined literal, so the walker works on
/// Windows runners.
fn join_relative(base: &Path, relative: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for component in relative.split(['/', '\\']).filter(|part| !part.is_empty()) {
        if component == "." {
            continue;
        }
        path.push(component);
    }
    path
}

fn read_manifest(path: &Path) -> Result<Vec<ManifestRecord>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    let mut records = Vec::new();

    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: ManifestRecord = serde_json::from_str(line).with_context(|| {
            format!("malformed manifest record at {}:{}", path.display(), index + 1)
        })?;
        records.push(record);
    }

    Ok(records)
}

/// Turn one artifact directory into run records.
///
/// The manifest is the ONLY identity source. A staged report with no covering
/// manifest record has no trustworthy package/environment identity at all —
/// artifact-name parsing was retired with the area model — so it is dropped;
/// `_stage_junit` writes a record for every invocation, including one whose
/// report is absent.
fn records_from_artifact(dir: &ArtifactDir) -> Result<Vec<RunRecord>> {
    let manifest_path = dir.path.join("manifest.jsonl");
    let manifest = if manifest_path.is_file() {
        read_manifest(&manifest_path)?
    } else {
        Vec::new()
    };

    let mut records = Vec::new();

    for entry in &manifest {
        let relative = entry.xml.replace('\\', "/");
        let report_path = join_relative(&dir.path, &relative);

        let outcome = if entry.report_present {
            ReportOutcome::read(&report_path)
        } else {
            ReportOutcome::default()
        };

        // The producer writes "" rather than inventing a value; an empty
        // environment marks the record as local-dev evidence, which surfaces
        // as unscheduled rather than being guessed at.
        let environment = non_empty(&entry.environment).unwrap_or_default();
        let identity_degraded = entry.environment.trim().is_empty();

        records.push(RunRecord {
            package: entry.package.clone(),
            environment,
            tier: Tier::parse(&entry.tier),
            artifact: dir.name.clone(),
            degraded: identity_degraded,
            report_present: entry.report_present && outcome.parse_error.is_none(),
            exit_code: entry.exit_code,
            duration_s: entry.duration_s,
            counts: outcome.report.counts,
            failed_tests: outcome.report.failed_tests,
            skipped_tests: outcome.report.skipped_tests,
            parse_error: outcome.parse_error,
            passed_identities: outcome.report.passed_tests,
        });
    }

    Ok(records)
}

/// A staged report, or the reason it could not be read.
///
/// An unreadable report is never an empty passing one: `parse_error` propagates
/// to the cell and makes it `MISSING`.
#[derive(Debug, Default)]
struct ReportOutcome {
    report: JunitReport,
    parse_error: Option<String>,
}

impl ReportOutcome {
    fn read(path: &Path) -> Self {
        let outcome = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))
            .and_then(|text| {
                parse_junit(&text).with_context(|| format!("failed to parse {}", path.display()))
            });
        match outcome {
            Ok(report) => Self {
                report,
                parse_error: None,
            },
            Err(err) => Self {
                report: JunitReport::default(),
                parse_error: Some(format!("{err:#}")),
            },
        }
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn read_producer_statuses(root: &Path) -> Result<Vec<ProducerStatus>> {
    let mut statuses = Vec::new();
    for dir in list_artifact_dirs(root)? {
        if !dir.name.starts_with("status-") {
            continue;
        }
        let path = dir.path.join("status.json");
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let status: ProducerStatus = serde_json::from_str(&text)
            .with_context(|| format!("malformed producer status {}", path.display()))?;
        statuses.push(status);
    }
    Ok(statuses)
}

// ---------------------------------------------------------------------------
// Cell classification
// ---------------------------------------------------------------------------

struct ClassifyInputs<'a> {
    expected: &'a [ExpectedCell],
    records: &'a [RunRecord],
    statuses: &'a [ProducerStatus],
    /// (environment, tier) → package → expected test identities.
    expected_tests: &'a BTreeMap<(String, Tier), BTreeMap<String, Vec<String>>>,
}

fn classify(inputs: &ClassifyInputs<'_>) -> Vec<Cell> {
    let mut by_key: BTreeMap<CellKey, Vec<usize>> = BTreeMap::new();
    for (index, record) in inputs.records.iter().enumerate() {
        by_key.entry(record.cell_key()).or_default().push(index);
    }

    let mut cells: BTreeMap<CellKey, Cell> = BTreeMap::new();

    for expectation in inputs.expected {
        let indices = by_key.remove(&expectation.key).unwrap_or_default();
        cells.insert(
            expectation.key.clone(),
            classify_one(inputs, &expectation.key, Some(expectation), &indices),
        );
    }

    // Evidence for a cell policy did not schedule. Reported rather than
    // dropped: an unexpected report is itself a policy/scope defect.
    for (key, indices) in by_key {
        let mut cell = classify_one(inputs, &key, None, &indices);
        cell.reasons.push(
            "produced evidence but policy did not schedule it for this run; \
             the resolved package policy or the affected scope is wrong"
                .to_owned(),
        );
        cells.insert(key, cell);
    }

    cells.into_values().collect()
}

fn classify_one(
    inputs: &ClassifyInputs<'_>,
    key: &CellKey,
    expectation: Option<&ExpectedCell>,
    indices: &[usize],
) -> Cell {
    let mut counts = Counts::default();
    let mut failed_tests = BTreeSet::new();
    let mut observed_skips = BTreeSet::new();
    let mut observed_tests = BTreeSet::new();
    let mut packages_with_evidence = BTreeSet::new();
    let mut reasons = Vec::new();
    let mut has_unusable_record = false;
    let mut duration_s = 0u64;

    for &index in indices {
        let record = &inputs.records[index];
        counts.add(&record.counts);
        duration_s += record.duration_s;
        failed_tests.extend(record.failed_tests.iter().cloned());
        observed_skips.extend(record.skipped_tests.iter().cloned());
        observed_tests.extend(record.failed_tests.iter().cloned());
        observed_tests.extend(record.skipped_tests.iter().cloned());

        if let Some(err) = &record.parse_error {
            has_unusable_record = true;
            reasons.push(format!("{}: unreadable report — {err}", record.package));
        } else if !record.report_present {
            has_unusable_record = true;
            reasons.push(format!(
                "{}: {}",
                record.package,
                missing_report_reason(record.exit_code)
            ));
        } else {
            packages_with_evidence.insert(record.package.clone());
            // nextest exits 100 only when a test failed. A report that parses
            // clean under exit 100 means the report and the exit code disagree,
            // and the report is the less trustworthy of the two.
            if record.exit_code == 100 && record.counts.bad() == 0 {
                has_unusable_record = true;
                reasons.push(format!(
                    "{}: nextest exited 100 (test failure) but its report shows no \
                     failure; the report is incomplete",
                    record.package
                ));
            }
        }
        if record.degraded {
            reasons.push(format!(
                "{}: the manifest record carried no environment — a local-dev \
                 staging tree, not CI evidence",
                record.package
            ));
        }
    }

    let exclusion = expectation.and_then(|cell| cell.exclusion.clone());
    let scheduled = expectation.is_some() && exclusion.is_none();

    let expected_for_cell = inputs
        .expected_tests
        .get(&(key.environment.clone(), key.tier.clone()));
    let skip_evidence_degraded = expected_for_cell.is_none();
    let mut absent_skips = BTreeSet::new();

    if let Some(by_package) = expected_for_cell {
        for (package, tests) in by_package {
            // Only judge packages this cell actually produced evidence for;
            // otherwise a MISSING report would masquerade as N test skips.
            if !packages_with_evidence.contains(package) {
                continue;
            }
            for test in tests {
                if !produced_a_result(inputs, indices, test) {
                    absent_skips.insert(test.clone());
                }
            }
        }
    }

    let mut all_skips: BTreeSet<String> = observed_skips.clone();
    all_skips.extend(absent_skips.iter().cloned());
    if !absent_skips.is_empty() {
        reasons.push(format!(
            "{} expected test(s) compiled on this environment produced no result",
            absent_skips.len()
        ));
    }

    let backends: Vec<String> = expectation
        .map(|cell| cell.backends.clone())
        .unwrap_or_default();
    let mut policy_gap = false;

    let declared_gap = expectation.and_then(|cell| match &cell.gap {
        Some(GapStatus::Governed(gap)) => Some(gap.clone()),
        _ => None,
    });

    if let Some(gap) = &declared_gap {
        policy_gap = true;
        if expectation.is_some_and(|cell| cell.accepted_gap) {
            // Spec section 6: the visible description must carry the owner,
            // expiry, policy link, what closes the gap, and how to revoke the
            // acceptance. A reader must not have to be told where to look.
            reasons.push(format!("accepted policy gap: {}", gap.describe_acceptance()));
        } else {
            reasons.push(format!("governed policy gap: {}", gap.describe()));
        }
    } else if matches!(expectation.and_then(|cell| cell.gap.as_ref()), Some(GapStatus::Ungoverned)) {
        // An *ungoverned* gap: the package declares this tier on an environment
        // the capability table records as plainly unable to host it, with no
        // owner, reason, or expiry. `affected_scope.py` is supposed to keep
        // such a table from landing; catching it here too means a config-side
        // regression surfaces as a blocking cell rather than a green one.
        policy_gap = true;
        let requirement = match key.tier {
            Tier::L2 => format!(
                "tier requires backend(s) [{}] but {} cannot host any of them",
                backends.join(", "),
                key.environment
            ),
            Tier::Browser => format!(
                "the browser tier but {} cannot host a headless browser",
                key.environment
            ),
            _ => format!("the {} tier but {} cannot host it", key.tier, key.environment),
        };
        reasons.push(format!(
            "UNGOVERNED policy gap: {requirement}, and environments.json records \
             no governed unavailability for it"
        ));
    }

    // A cell may only be blamed on a job that actually gates it. Only the
    // `needs:` edges `_package-ci.yml` actually declares are encoded here.
    let gating_jobs: &[&str] = match key.tier {
        Tier::L2 | Tier::Browser => &["L1"],
        _ => &[],
    };
    let upstream = inputs
        .statuses
        .iter()
        .find(|status| {
            status.package == key.package
                && gating_jobs.contains(&status.job.as_str())
                && matches!(status.result.as_str(), "failure" | "cancelled")
        })
        .cloned();

    // The producer's own word for this cell, when it has one: a producer
    // status naming THIS package, tier, and environment. A `failure` here with
    // a clean JUnit report is the companion-suite case — a green Rust report
    // must never hide a failed companion suite (or fixture, or backend proof),
    // so the status DOWNGRADES the cell below. It can never upgrade one.
    let own_status = find_status(inputs.statuses, key);

    // The plan's own decisions, taken before the run. A reused cell's result
    // came off a receipt the scheduler already accepted; an accepted gap is an
    // owned, unexpired entry in the capability table. Both are *states of the
    // cell*, not reasons to drop it, which is what makes the PR #76 shape —
    // omitting an execution and losing the cell — unrepresentable here.
    let reused = expectation.and_then(|cell| cell.reused.as_ref());
    let reuse_contested = reused.is_some() && !indices.is_empty();
    if reuse_contested {
        reasons.push(
            "a cell satisfied by a receipt also produced CI evidence; the run \
             executed work the plan reused, and the executed result is the one \
             reported"
                .to_owned(),
        );
    }
    if let Some(result) = reused.filter(|_| !reuse_contested) {
        reasons.push(format!(
            "reused {} evidence from {}: {}",
            result.origin, result.evidence.reference, result.evidence.measurements
        ));
        counts = result.counts;
        duration_s = result.duration_s;
        failed_tests.extend(result.failed_tests.iter().cloned());
    }
    if let Some(prohibition) = expectation.and_then(|cell| cell.prohibition.as_deref()) {
        reasons.push(prohibition.to_owned());
    }

    let mut state = if let Some(result) = reused.filter(|_| !reuse_contested) {
        // A complete failed local result stays a failure (spec section 3.5):
        // reuse must not turn a failure into a success or conceal it.
        if result.passed {
            CellState::Pass
        } else {
            CellState::Fail
        }
    } else if expectation.is_some_and(|cell| cell.accepted_gap) && counts.bad() == 0 {
        // Governed and unexpired at plan time, so the cell is neither a pass
        // nor a failure and no test or archive build was started for it. Real
        // failures still outrank it, as they do for POLICY GAP.
        CellState::AcceptedGap
    } else if let Some(exclusion) = &exclusion {
        if indices.is_empty() {
            // `gates = false`: NOT SCHEDULED with its governance metadata —
            // never conflated with NOTHING TO RUN, never a pass.
            reasons.push(format!("not gating: {}", exclusion.describe()));
            CellState::NotScheduled
        } else {
            // Evidence for a package policy excludes: someone ran it anyway.
            // The counts are reported honestly and the verdict blocks on the
            // disagreement (`cell-unscheduled-evidence`).
            reasons.push(format!(
                "produced evidence but the package is `gates = false` ({})",
                exclusion.describe()
            ));
            classify_state_from_evidence(
                &counts,
                declared_gap.is_some(),
                policy_gap,
                indices,
                has_unusable_record,
                &mut reasons,
                own_status,
                upstream.as_ref(),
            )
        }
    } else if !scheduled && indices.is_empty() {
        // Policy did not schedule it and nothing was uploaded for it, so the
        // cell genuinely does not exist.
        CellState::NotScheduled
    } else {
        classify_state_from_evidence(
            &counts,
            declared_gap.is_some(),
            policy_gap,
            indices,
            has_unusable_record,
            &mut reasons,
            own_status,
            upstream.as_ref(),
        )
    };

    // R12: a DECLARED companion suite must leave success evidence. A skipped
    // companion step — or no reported outcome at all — leaves the Rust JUnit
    // report green while the suite never ran, so it downgrades the cell
    // exactly as a failed companion does. Only green-reading states are
    // downgraded; MISSING and FAIL already block.
    let expected_companions = expectation
        .map(|cell| cell.companion_suites.as_slice())
        .unwrap_or(&[]);
    if !expected_companions.is_empty()
        && matches!(state, CellState::Pass | CellState::NothingToRun | CellState::Skip)
    {
        let outcome = own_status.and_then(|status| status.companion.as_deref());
        if outcome != Some("success") {
            reasons.push(format!(
                "declared companion suite(s) [{}] produced no success evidence \
                 (the producer reports `{}`); a green Rust JUnit report must not \
                 hide a companion suite that never ran",
                expected_companions.join(", "),
                outcome.unwrap_or("no companion outcome"),
            ));
            state = CellState::Fail;
        }
    }

    Cell {
        key: key.clone(),
        area: expectation.map(|cell| cell.area.clone()).unwrap_or_default(),
        state,
        // A cell nothing produced is `none`, not `ci`: reporting a CI origin
        // for a result that does not exist would make a MISSING cell look like
        // something CI answered for.
        origin: match reused {
            Some(result) if !reuse_contested => result.origin,
            _ if indices.is_empty() => Origin::Unproduced,
            _ => Origin::Ci,
        },
        counts: Counts {
            skipped: counts.skipped + absent_skips.len() as u32,
            ..counts
        },
        duration_s,
        target_kinds: expectation
            .map(|cell| cell.target_kinds.clone())
            .unwrap_or_default(),
        compile_coverage_from: expectation
            .map(|cell| cell.compile_coverage_from.clone())
            .unwrap_or_default(),
        evidence: reused
            .filter(|_| !reuse_contested)
            .map(|result| result.evidence.clone()),
        scheduled,
        skipped_tests: all_skips.into_iter().collect(),
        failed_tests: failed_tests.into_iter().collect(),
        skip_evidence_degraded,
        declared_gap,
        reasons,
        records: indices.to_vec(),
    }
}

/// Resolve a cell's state from the evidence it produced. Handles the shared
/// tail of [`classify_one`] for scheduled, unscheduled-with-evidence, and
/// exclusion-violating cells alike.
#[allow(clippy::too_many_arguments)]
fn classify_state_from_evidence(
    counts: &Counts,
    has_declared_gap: bool,
    policy_gap: bool,
    indices: &[usize],
    has_unusable_record: bool,
    reasons: &mut Vec<String>,
    own_status: Option<&ProducerStatus>,
    upstream: Option<&ProducerStatus>,
) -> CellState {
    let mut state = if counts.bad() == 0 && has_declared_gap {
        // A *governed* gap explains the absence of evidence, so MISSING would
        // be the wrong answer: nobody failed to upload anything. It still
        // blocks — the point of the state is that the cell cannot go green.
        // Actual failures still outrank it, handled below.
        CellState::PolicyGap
    } else if indices.is_empty() || has_unusable_record {
        // The cell's own observation comes FIRST. The upstream edge is
        // context, not cause, and a renderer showing only the leading reason
        // must show what this cell actually saw.
        if indices.is_empty() {
            reasons.push("scheduled but produced no report at all".to_owned());
        }
        // The producer's own explanation, when it has one. "No report at all"
        // is equally true of a dead guest and of a tier that ran no tests, and
        // only the producer can tell them apart, so it says which.
        if let Some(detail) = own_status.and_then(|status| status.detail.as_deref()) {
            reasons.push(detail.to_owned());
        }
        if let Some(status) = upstream {
            reasons.push(format!(
                "upstream job `{}` concluded `{}`, so this leg never ran and \
                 GitHub never evaluated its matrix context",
                status.job, status.result
            ));
        }
        CellState::Missing
    } else if counts.bad() > 0 {
        CellState::Fail
    } else if policy_gap {
        CellState::PolicyGap
    } else if counts.total == 0 {
        // A scheduled package whose invocation selected ZERO tests. Not a pass
        // (nothing executed), not a blocking missing result (the invocation
        // succeeded and said so), and never conflated with NOT SCHEDULED.
        reasons.push(
            "the invocation selected zero tests; a package with no tests is not a pass".to_owned(),
        );
        CellState::NothingToRun
    } else if counts.passed == 0 {
        // Evidence exists but nothing executed. Never PASS.
        CellState::Skip
    } else {
        CellState::Pass
    };

    // A producer status of `failure` for THIS cell downgrades any clean
    // reading: the job failed although its JUnit shows no failing test, which
    // is exactly the companion-suite / fixture / backend-proof case. Status
    // evidence can worsen a cell, never improve it.
    if matches!(state, CellState::Pass | CellState::NothingToRun | CellState::Skip)
        && own_status.is_some_and(|status| status.result == "failure")
    {
        let detail = own_status
            .and_then(|status| status.detail.as_deref())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                "the producer job concluded `failure` although its report shows no \
                 failing test"
                    .to_owned()
            });
        reasons.push(format!("producer reported failure: {detail}"));
        state = CellState::Fail;
    }

    state
}

/// Environment assumed for a producer status that does not name one. The lint
/// job is Linux-only today (`_package-ci.yml`).
const DEFAULT_STATUS_ENVIRONMENT: &str = "ubuntu-latest";

/// Why a scheduled invocation staged no report.
///
/// `exit_code` is nextest's raw code, not a normalized 0/1, and the distinction
/// matters: `101` means the crate never built, so the tier's test set is
/// unknown — emphatically *not* an empty tier, which would be `N/A`.
fn missing_report_reason(exit_code: i64) -> String {
    match exit_code {
        101 => "the crate failed to BUILD (nextest exit 101); no test ran, so this \
                tier's test set is unknown and cannot be treated as empty"
            .to_owned(),
        100 => "nextest reported test failures (exit 100) but staged no report; the \
                failing tests cannot be identified"
            .to_owned(),
        0 => "nextest exited 0 but staged no report; nothing was recorded".to_owned(),
        other => format!("nextest exited {other} and staged no report"),
    }
}

/// Turn producer statuses for non-test jobs (today: `lint`, `check`) into
/// cells, so a job that emits no JUnit can still be baselined and can still
/// block, under the same stable `{package, environment, tier}` identity as a
/// test tier.
///
/// ## Notes
///
/// This is why a verdict can gate merging at all. GitHub's own job name for a
/// skipped matrix leg is an un-interpolated expression, so the run's job list
/// cannot supply this; an explicit status artifact can. It is also why these
/// gates are always CI-origin: with no JUnit report to attribute, a local
/// receipt has nothing to contribute for them.
fn status_cells(
    statuses: &[ProducerStatus],
    scope: &BTreeSet<String>,
    existing: &[Cell],
    policies: &[PackagePolicy],
    planned_gates: &[ExpectedCell],
) -> Vec<Cell> {
    let mut cells: Vec<Cell> = Vec::new();

    // A gate the plan scheduled that uploaded no status at all is MISSING, not
    // absent. Without the plan there is nothing to notice its absence against,
    // which is why this half only exists on the plan path.
    for expectation in planned_gates {
        if existing.iter().any(|cell| cell.key == expectation.key)
            || cells.iter().any(|cell| cell.key == expectation.key)
        {
            continue;
        }
        let status = find_status(statuses, &expectation.key);
        let (state, reason) = match status {
            Some(status) => state_from_status(status),
            None => (
                CellState::Missing,
                Some(
                    "scheduled but uploaded no producer status; the job never \
                     reported a result"
                        .to_owned(),
                ),
            ),
        };
        let (state, reason) = companion_lint_downgrade(status, policies, state, reason);
        cells.push(Cell {
            area: expectation.area.clone(),
            state,
            origin: if status.is_some() {
                Origin::Ci
            } else {
                Origin::Unproduced
            },
            target_kinds: expectation.target_kinds.clone(),
            compile_coverage_from: expectation.compile_coverage_from.clone(),
            scheduled: true,
            reasons: reason.into_iter().collect(),
            ..blank_cell(expectation.key.clone())
        });
    }

    for status in statuses {
        if !scope.contains(&status.package) {
            continue;
        }
        let tier = Tier::parse(&status.job);
        if is_test_tier(&tier) {
            continue;
        }
        let key = CellKey {
            package: status.package.clone(),
            environment: status
                .environment
                .clone()
                .unwrap_or_else(|| DEFAULT_STATUS_ENVIRONMENT.to_owned()),
            tier,
        };
        if existing.iter().any(|cell| cell.key == key)
            || cells.iter().any(|cell: &Cell| cell.key == key)
        {
            continue;
        }

        let (state, reason) = state_from_status(status);

        let (state, reason) = companion_lint_downgrade(Some(status), policies, state, reason);
        let area = policies
            .iter()
            .find(|policy| policy.package == status.package)
            .map(|policy| policy.area.clone())
            .unwrap_or_default();
        cells.push(Cell {
            area,
            state,
            origin: Origin::Ci,
            scheduled: true,
            reasons: reason.into_iter().collect(),
            ..blank_cell(key)
        });
    }

    cells
}

/// Tiers whose evidence is a JUnit report. Everything else (`lint`, `check`)
/// reports through an explicit producer status.
fn is_test_tier(tier: &Tier) -> bool {
    matches!(
        tier,
        Tier::L1 | Tier::L2 | Tier::L3 | Tier::Browser | Tier::Real
    )
}

/// The producer status covering a cell, if one was uploaded.
///
/// A status that names no environment is the lint job's, which is Linux-only
/// (`_package-ci.yml`); it covers the default environment rather than nothing.
fn find_status<'a>(statuses: &'a [ProducerStatus], key: &CellKey) -> Option<&'a ProducerStatus> {
    statuses.iter().find(|status| {
        status.package == key.package
            && Tier::parse(&status.job) == key.tier
            && status
                .environment
                .as_deref()
                .unwrap_or(DEFAULT_STATUS_ENVIRONMENT)
                == key.environment
    })
}

/// A producer's own account of a gate that emits no JUnit.
fn state_from_status(status: &ProducerStatus) -> (CellState, Option<String>) {
    match status.result.as_str() {
        "success" => (CellState::Pass, None),
        "failure" => (CellState::Fail, status.detail.clone()),
        "cancelled" => (
            CellState::Missing,
            Some("job was cancelled, so it emitted no result".to_owned()),
        ),
        "skipped" => (
            CellState::NotScheduled,
            Some("job was skipped by its `if:` condition".to_owned()),
        ),
        other => (
            CellState::Missing,
            Some(format!("unrecognized job result `{other}`")),
        ),
    }
}

/// A declared companion suite lints too (the lint job runs the frontend lint on
/// its Node-capable leg), so its success must be evidenced exactly as on the L1
/// cell: a skipped companion downgrades a green lint rather than hiding behind
/// it (R12).
fn companion_lint_downgrade(
    status: Option<&ProducerStatus>,
    policies: &[PackagePolicy],
    state: CellState,
    reason: Option<String>,
) -> (CellState, Option<String>) {
    let Some(status) = status else {
        return (state, reason);
    };
    if status.job == "lint"
        && state == CellState::Pass
        && policies
            .iter()
            .any(|policy| policy.package == status.package && !policy.companion_suites.is_empty())
        && status.companion.as_deref() != Some("success")
    {
        return (
            CellState::Fail,
            Some(format!(
                "declared companion suite produced no success evidence (the \
                 producer reports `{}`); a green lint must not hide a \
                 companion suite that never ran",
                status.companion.as_deref().unwrap_or("no companion outcome"),
            )),
        );
    }
    (state, reason)
}

/// A cell with no observations yet, for the status-derived gates.
fn blank_cell(key: CellKey) -> Cell {
    Cell {
        key,
        area: String::new(),
        state: CellState::NotScheduled,
        origin: Origin::Unproduced,
        counts: Counts::default(),
        duration_s: 0,
        target_kinds: Vec::new(),
        compile_coverage_from: String::new(),
        evidence: None,
        scheduled: false,
        skipped_tests: Vec::new(),
        failed_tests: Vec::new(),
        skip_evidence_degraded: false,
        declared_gap: None,
        reasons: Vec::new(),
        records: Vec::new(),
    }
}

/// Whether an expected test identity produced any result — pass, fail, or an
/// explicit skip — anywhere in this cell's reports.
fn produced_a_result(inputs: &ClassifyInputs<'_>, indices: &[usize], test: &str) -> bool {
    indices.iter().any(|&index| {
        let record = &inputs.records[index];
        record.passed_identities.iter().any(|id| id == test)
            || record.failed_tests.iter().any(|id| id == test)
            || record.skipped_tests.iter().any(|id| id == test)
    })
}

// ---------------------------------------------------------------------------
// Baseline
// ---------------------------------------------------------------------------

/// `.github/ci/ci-baseline.toml`.
///
/// TOML rather than JSON because this file is *hand-edited policy*: owners add
/// and remove entries and must be able to leave comments explaining why an
/// entry exists and when it expires. JSON has no comments and no multi-line
/// strings, and the repo's config idiom is TOML. The machine-generated rollup
/// (`results.json`) goes the other way for the same reason — nothing hand-edits
/// it.
#[derive(Clone, Debug, Default, Deserialize)]
struct Baseline {
    // Required, not defaulted: a version-less file must die here as a missing
    // field rather than silently assume the current schema generation and
    // skip the migration-error path on the next bump.
    schema_version: u32,
    #[serde(default)]
    failure: Vec<FailureEntry>,
    #[serde(default)]
    skip: Vec<SkipEntry>,
}

/// A known-red leg, keyed by stable identity — never by a GitHub display name.
/// `deny_unknown_fields`: a stale area/shard key (`shard = "1/4"`, `area = …`)
/// must be rejected, not silently parsed away (AC11).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureEntry {
    package: String,
    environment: String,
    tier: Tier,
    owner: String,
    reason: String,
    source_run: String,
    #[serde(default)]
    expiry: Option<String>,
}

/// An approved skip budget. `tests` carries exact identities so one removed
/// skip cannot mask one newly-added skip.
#[derive(Clone, Debug, Deserialize)]
struct SkipEntry {
    package: String,
    environment: String,
    tier: Tier,
    /// The backend whose absence justifies these skips. Recorded for triage and
    /// to correlate with policy gaps; matching is on `tests`, because JUnit
    /// carries no backend attribution.
    #[serde(default)]
    backend: String,
    tests: Vec<String>,
    owner: String,
    reason: String,
    source_run: String,
    #[serde(default)]
    expiry: Option<String>,
}

fn load_baseline(path: &Path) -> Result<Baseline> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read baseline {}", path.display()))?;
    let baseline: Baseline =
        toml::from_str(&text).with_context(|| format!("invalid baseline {}", path.display()))?;

    if baseline.schema_version > BASELINE_SCHEMA_VERSION {
        bail!(
            "baseline schema_version {} is newer than this tool understands \
             ({BASELINE_SCHEMA_VERSION})",
            baseline.schema_version
        );
    }
    if baseline.schema_version < BASELINE_SCHEMA_VERSION {
        bail!(
            "baseline {} is schema_version {} (area-keyed); this tool reads \
             schema_version {BASELINE_SCHEMA_VERSION} (package-keyed). Re-key the entries \
             to packages — see fixes/2026-08-06-cicd/plan.md (Phase 4)",
            path.display(),
            baseline.schema_version
        );
    }
    for entry in &baseline.failure {
        validate_date(entry.expiry.as_deref(), "failure", &entry.package)?;
    }
    for entry in &baseline.skip {
        validate_date(entry.expiry.as_deref(), "skip", &entry.package)?;
    }

    Ok(baseline)
}

/// Whether `value` is a syntactically valid ISO `YYYY-MM-DD`.
///
/// ## Notes
///
/// Zero-padded ISO dates compare chronologically under plain lexicographic
/// ordering, which is what lets every expiry check in this tool run without a
/// date dependency. That equivalence holds only for well-formed input, so a
/// caller that compares must validate first.
fn is_iso_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

fn validate_date(value: Option<&str>, kind: &str, package: &str) -> Result<()> {
    let Some(value) = value else { return Ok(()) };
    if !is_iso_date(value) {
        bail!("{kind} entry for `{package}` has an invalid expiry `{value}`; expected YYYY-MM-DD");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Verdict
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Block,
    Note,
}

#[derive(Clone, Debug, Serialize)]
struct Finding {
    severity: Severity,
    rule: &'static str,
    subject: String,
    detail: String,
}

impl Finding {
    fn block(rule: &'static str, subject: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            severity: Severity::Block,
            rule,
            subject: subject.into(),
            detail: detail.into(),
        }
    }

    fn note(rule: &'static str, subject: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            severity: Severity::Note,
            rule,
            subject: subject.into(),
            detail: detail.into(),
        }
    }
}

/// Apply every merge-gate rule to a rollup.
///
/// ## Notes
///
/// `scope` is the set of packages this run actually scheduled. An entry naming
/// a package outside it is *ignored* — neither accepted as a pass nor counted
/// as a block — because the run produced no information about it either way.
///
/// Two independent excusal paths feed the blocking-cell loop, and they never
/// overlap: `.github/ci/ci-baseline.toml` excuses a `FAIL`, and a governed,
/// unexpired capability-table gap excuses a `POLICY GAP` (see
/// [`policy_gap_findings`]). Both leave the cell's state untouched and both
/// report a `note`, so an excused cell is never invisible.
fn verdict(rollup: &Rollup, baseline: &Baseline, today: Option<&str>) -> Vec<Finding> {
    let scope: BTreeSet<&String> = rollup.scope.iter().collect();
    let mut findings = Vec::new();
    let mut excused_cells: BTreeSet<CellKey> = BTreeSet::new();
    let mut out_of_scope: BTreeSet<String> = BTreeSet::new();
    let mut unscheduled: BTreeSet<String> = BTreeSet::new();

    for entry in &baseline.failure {
        let key = CellKey {
            package: entry.package.clone(),
            environment: entry.environment.clone(),
            tier: entry.tier.clone(),
        };
        let subject = key.to_string();

        if !scope.contains(&entry.package) {
            out_of_scope.insert(entry.package.clone());
            continue;
        }

        if let Some(expiry) = &entry.expiry {
            if let Some(today) = today {
                if expiry.as_str() < today {
                    findings.push(Finding::block(
                        "baseline-expired",
                        subject.clone(),
                        format!(
                            "entry expired on {expiry} (owner {}); re-justify it or fix the failure",
                            entry.owner
                        ),
                    ));
                }
            }
        }

        let Some(cell) = rollup.cell(&key) else {
            // Scope is per gate: a package reached only through a lint-global
            // input is in scope with no test tier scheduled, and the run
            // produced no information about this leg either way — the same
            // standing as an out-of-scope entry. Only a KNOWN unscheduled
            // leg is excused; a document that cannot say fails closed.
            if rollup.scheduled(&key) == Some(false) {
                unscheduled.insert(subject);
                continue;
            }
            findings.push(Finding::block(
                "baseline-no-result",
                subject,
                "in scope but the rollup has no cell for it; a baselined entry \
                 that emits no result stays blocking",
            ));
            continue;
        };

        match cell.state {
            CellState::Fail => {
                excused_cells.insert(key);
                findings.push(Finding::note(
                    "baseline-accepted",
                    subject,
                    format!(
                        "known failure ({}), owner {}, first recorded in run {}",
                        entry.reason, entry.owner, entry.source_run
                    ),
                ));
            }
            CellState::Pass => findings.push(Finding::block(
                "baseline-now-passing",
                subject,
                format!(
                    "baselined leg now PASSES; remove the entry (owner {})",
                    entry.owner
                ),
            )),
            other => findings.push(Finding::block(
                "baseline-no-result",
                subject,
                format!(
                    "scheduled but rendered {other}; a baselined entry that is \
                     cancelled, missing, or emits no result stays blocking"
                ),
            )),
        }
    }

    if !unscheduled.is_empty() {
        findings.push(Finding::note(
            "baseline-unscheduled",
            format!("{} leg(s)", unscheduled.len()),
            format!(
                "in scope, but policy scheduled no such leg for this run (scope is \
                 per gate); ignored, not treated as a pass: {}",
                unscheduled.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        ));
    }

    // Collapsed to one line. Most of the baseline is out of scope on any
    // narrow run, and one note per entry buries the blocks that matter.
    if !out_of_scope.is_empty() {
        findings.push(Finding::note(
            "baseline-out-of-scope",
            format!("{} package(s)", out_of_scope.len()),
            format!(
                "outside this run's affected scope; ignored, not treated as a pass: {}",
                out_of_scope.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        ));
    }

    // A cell that produced evidence policy never scheduled means the resolved
    // package policy disagrees with what CI actually ran. Blocking is what
    // makes that self-correcting: the cell's own state is now honest about the
    // tests, so if this did not block, a green leg the policy layer cannot see
    // would report as an ordinary PASS and the divergence would stay
    // invisible.
    for cell in &rollup.cells {
        if cell.scheduled || cell.records.is_empty() {
            continue;
        }
        findings.push(Finding::block(
            "cell-unscheduled-evidence",
            cell.key.to_string(),
            format!(
                "rendered {} from a leg that ran, but policy scheduled no such cell; \
                 the resolved package policy or the affected scope is wrong",
                cell.state
            ),
        ));
    }

    let (gap_findings, ruled_on_gaps) = policy_gap_findings(rollup, today);
    findings.extend(gap_findings);

    for cell in &rollup.cells {
        if !cell.state.blocks() {
            continue;
        }
        if cell.state == CellState::Fail && excused_cells.contains(&cell.key) {
            continue;
        }
        if cell.state.is_gap() && ruled_on_gaps.contains(&cell.key) {
            continue;
        }
        let detail = why(cell);
        findings.push(Finding::block(
            match cell.state {
                CellState::Missing => "cell-missing",
                state if state.is_gap() => "cell-policy-gap",
                _ => "cell-failed",
            },
            cell.key.to_string(),
            format!("{} — {detail}", cell.state),
        ));
    }

    findings.extend(skip_findings(rollup, baseline, &scope));
    findings
}

/// Decide, for every gap cell — `POLICY GAP` and `ACCEPTED GAP` alike — whether
/// it is acknowledged work or an unexplained hole. This is the entire
/// accept/block decision for both states.
///
/// Returns the findings to report and the cells this function has already ruled
/// on — accepted *or* rejected — which the caller must therefore not also emit a
/// generic `cell-policy-gap` block for. One cell, one row, whichever way it
/// went.
///
/// ## Notes
///
/// Acknowledged is not acceptable, and neither is invisible. A gap cell renders
/// in the grid with its governance — `classify` decided its state and nothing
/// here can change it — and appears in the verdict table as a `note` naming its
/// owner, expiry, policy entry, and what closes it, exactly as an accepted
/// baseline failure does. Acceptance buys one thing: the run may merge. This is
/// deliberately *not* `soft_os`, which removed the leg from the verdict
/// altogether (plan §1.4).
///
/// The state distinction is *where the decision was taken*, not whether the
/// verdict re-checks it. `ACCEPTED GAP` means the plan already matched the cell
/// to a governed, unexpired entry before the run; `POLICY GAP` means the rollup
/// derived the gap from the capability table itself. Both are ruled on here,
/// because the verdict runs `if: always()` and must catch an expiry that lapsed
/// after the plan was written.
///
/// Three things forfeit acceptance, and each maps to a rule the plan already
/// states for baselined failures:
///
/// - **ungoverned** — the capability table records the absence as a plain
///   `false`, with no owner, reason, or expiry. This is the case that catches a
///   tier being quietly switched off; it falls through to the generic blocking
///   cell loop.
/// - **incomplete** — no owner, reason, or expiry. An unattributable or undated
///   gap is a permanent exclusion wearing a temporary label.
/// - **expired** — the expiry rule, applied to gaps.
///
/// A gap whose cell produced real *failures* never reaches this function:
/// `classify` ranks `Fail` above `PolicyGap`, so the cell is `FAIL` and is
/// judged by the baseline. A gap declaration can never suppress evidence.
///
/// There is deliberately **no** `policy-gap-now-executing` rule, tempting as
/// the analogy to `baseline-now-passing` is. A `require_level!` gate that skips
/// because its backend is absent early-returns, and nextest records that as a
/// JUnit **pass** — so on this evidence a passing count is exactly what a
/// correctly-governed gap looks like, and such a rule would block the case it
/// was meant to protect. Detecting a gap that has genuinely closed needs
/// per-backend execution proof. Until then `expiry` is the only forcing
/// function, which is why an undated gap is rejected outright.
///
/// ## Errors
///
/// The expiry check duplicates `affected_scope.py::validate_expiry` on purpose.
/// That one fails the scope job at config time with the most actionable
/// message, and it is the better place to *learn* about a lapsed gap. But it is
/// not sufficient: every `verdict` invocation — the transitional whole-run
/// `ci-verdict` job and each area's own rollup — runs `if: always()`, precisely
/// so a failed, skipped, or never-scheduled scope job cannot suppress it. If
/// expiry lived only in the Python, an expired gap would be excused by the
/// checks that actually gate merging whenever the check that catches it did not
/// run.
fn policy_gap_findings(rollup: &Rollup, today: Option<&str>) -> (Vec<Finding>, BTreeSet<CellKey>) {
    let mut findings = Vec::new();
    let mut ruled_on: BTreeSet<CellKey> = BTreeSet::new();

    for cell in &rollup.cells {
        if !cell.state.is_gap() {
            continue;
        }
        let subject = cell.key.to_string();
        let Some(gap) = &cell.declared_gap else {
            if cell.state == CellState::AcceptedGap {
                // An acceptance that names no policy entry is not an
                // acceptance. It cannot be traced to an owner or an expiry, so
                // it excuses nothing.
                findings.push(Finding::block(
                    "policy-gap-incomplete",
                    subject,
                    "carries the ACCEPTED GAP state with no governing policy entry; \
                     an ungoverned gap can never excuse missing coverage",
                ));
                ruled_on.insert(cell.key.clone());
            }
            continue;
        };
        ruled_on.insert(cell.key.clone());

        let mut incomplete = Vec::new();
        if gap.owner.trim().is_empty() {
            incomplete.push("an owner");
        }
        if gap.reason.trim().is_empty() {
            incomplete.push("a reason");
        }
        if gap.expiry.trim().is_empty() {
            incomplete.push("an expiry");
        }
        if !incomplete.is_empty() {
            findings.push(Finding::block(
                "policy-gap-incomplete",
                subject,
                format!(
                    "declared policy gap lacks {}; an unattributable or undated gap is a \
                     permanent exclusion wearing a temporary label",
                    incomplete.join(" and ")
                ),
            ));
            continue;
        }

        let expiry = gap.expiry.trim();
        if !is_iso_date(expiry) {
            findings.push(Finding::block(
                "policy-gap-incomplete",
                subject,
                format!(
                    "declared policy gap has an invalid expiry `{expiry}`; expected \
                     YYYY-MM-DD (owner {})",
                    gap.owner
                ),
            ));
            continue;
        }

        if today.is_some_and(|today| expiry < today) {
            findings.push(Finding::block(
                "policy-gap-expired",
                subject,
                format!(
                    "declared policy gap expired on {expiry} (owner {}); close it, or move \
                     the date out with a fresh justification. Declared reason: {}",
                    gap.owner, gap.reason
                ),
            ));
            continue;
        }

        findings.push(Finding::note(
            "policy-gap-accepted",
            subject,
            format!(
                "owned, unexpired policy gap: {} (owner {}, expires {expiry}); policy {}{}; \
                 {REVOCATION_INSTRUCTIONS}",
                gap.reason,
                gap.owner,
                gap.policy_link(),
                if gap.closes.is_empty() {
                    String::new()
                } else {
                    format!("; closed by {}", gap.closes)
                },
            ),
        ));
    }

    (findings, ruled_on)
}

/// Exact-test-identity skip diff.
///
/// Counts alone cannot see a swap: remove one approved skip, add one new one,
/// and the total is unchanged. Comparing identity *sets* catches both halves
/// independently — the new skip blocks, and the resolved one forces cleanup.
fn skip_findings(
    rollup: &Rollup,
    baseline: &Baseline,
    scope: &BTreeSet<&String>,
) -> Vec<Finding> {
    let mut approved: BTreeMap<CellKey, BTreeMap<String, &SkipEntry>> = BTreeMap::new();
    for entry in &baseline.skip {
        let key = CellKey {
            package: entry.package.clone(),
            environment: entry.environment.clone(),
            tier: entry.tier.clone(),
        };
        let bucket = approved.entry(key).or_default();
        for test in &entry.tests {
            bucket.insert(test.clone(), entry);
        }
    }

    let mut findings = Vec::new();

    for cell in &rollup.cells {
        if !cell.scheduled {
            continue;
        }
        let empty = BTreeMap::new();
        let bucket = approved.get(&cell.key).unwrap_or(&empty);
        let observed: BTreeSet<&String> = cell.skipped_tests.iter().collect();

        for test in &observed {
            match bucket.get(*test) {
                None => findings.push(Finding::block(
                    "skip-new",
                    format!("{} :: {test}", cell.key),
                    "newly skipped and not in the approved skip budget",
                )),
                Some(entry) => findings.push(Finding::note(
                    "skip-approved",
                    format!("{} :: {test}", cell.key),
                    format!(
                        "approved skip: {} (owner {}, recorded in run {})",
                        entry.reason, entry.owner, entry.source_run
                    ),
                )),
            }
        }

        // A cell with no evidence cannot say an approved skip was resolved.
        if matches!(cell.state, CellState::Missing | CellState::NotScheduled) {
            continue;
        }
        for (test, entry) in bucket {
            if !observed.contains(test) {
                findings.push(Finding::block(
                    "skip-resolved",
                    format!("{} :: {test}", cell.key),
                    format!(
                        "approved skip (backend `{}`) no longer skipped; remove it from \
                         the budget (owner {})",
                        if entry.backend.is_empty() {
                            "any"
                        } else {
                            entry.backend.as_str()
                        },
                        entry.owner
                    ),
                ));
            }
        }

        if cell.skip_evidence_degraded && !bucket.is_empty() {
            findings.push(Finding::note(
                "skip-evidence-degraded",
                cell.key.to_string(),
                "no expected-test manifest for this environment/tier, so a test \
                 absent from the report cannot be told apart from a `#[cfg]`-absent one",
            ));
        }
    }

    let out_of_scope: BTreeSet<&String> = baseline
        .skip
        .iter()
        .filter(|entry| !scope.contains(&entry.package))
        .map(|entry| &entry.package)
        .collect();
    if !out_of_scope.is_empty() {
        findings.push(Finding::note(
            "skip-out-of-scope",
            format!("{} package(s)", out_of_scope.len()),
            format!(
                "outside this run's affected scope; ignored: {}",
                out_of_scope
                    .iter()
                    .map(|package| package.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    findings
}

// ---------------------------------------------------------------------------
// Markdown rendering
// ---------------------------------------------------------------------------

/// Escape a value for a GFM pipe-table cell.
fn cell_text(value: &str) -> String {
    value.replace('|', "\\|").replace(['\n', '\r'], " ")
}

/// Render the package × environment × tier grid.
///
/// Plain GFM rather than the `renderable` Markdown tree renderer. `renderable`
/// would work — its tree renderer emits GFM pipe tables with correct cell
/// escaping — but reaching it means linking `renderable` (and, in this package,
/// `biscuit-terminal`) into the one CI job whose entire purpose is to be the
/// fast, always-runs required check, and whose only output target is GitHub's
/// own GFM renderer. Neither the terminal nor the browser fold is ever used
/// here, so the render tree buys nothing for the compile time it costs.
fn render_grid(rollup: &Rollup) -> String {
    let mut out = if rollup.area_scope.is_empty() {
        String::from("## CI rollup\n\n")
    } else {
        format!("## CI rollup — {}\n\n", rollup.area_scope.join(", "))
    };

    if rollup.scope_degraded {
        out.push_str(
            "> **Scope was inferred from the artifacts on disk.** A package that produced \
             nothing at all is invisible to an inferred scope, which is precisely the case \
             `MISSING` exists to catch. Pass `--scope`.\n\n",
        );
    }

    // Area is the reader's entry point and package the identity underneath it;
    // a document with no areas (the legacy policy path) renders one unnamed
    // group, which is the old layout exactly.
    let areas: BTreeSet<&str> = rollup
        .cells
        .iter()
        .map(|cell| cell.area.as_str())
        .collect();

    for area in &areas {
        let area_cells: Vec<&Cell> = rollup
            .cells
            .iter()
            .filter(|cell| cell.area == *area)
            .collect();
        if !area.is_empty() {
            out.push_str(&format!("### area `{}`\n\n", cell_text(area)));
        }
        let tiers: BTreeSet<Tier> = area_cells.iter().map(|cell| cell.key.tier.clone()).collect();

        for tier in &tiers {
            let cells: Vec<&&Cell> = area_cells
                .iter()
                .filter(|cell| &cell.key.tier == tier)
                .collect();
            let environments: BTreeSet<&String> =
                cells.iter().map(|cell| &cell.key.environment).collect();
            let packages: BTreeSet<&String> = cells.iter().map(|cell| &cell.key.package).collect();

            let heading = if area.is_empty() { "###" } else { "####" };
            out.push_str(&format!("{heading} {tier}\n\n| package |"));
            for environment in &environments {
                out.push_str(&format!(" {} |", cell_text(environment)));
            }
            out.push_str("\n| --- |");
            for _ in &environments {
                out.push_str(" --- |");
            }
            out.push('\n');

            for package in &packages {
                out.push_str(&format!("| `{}` |", cell_text(package)));
                for environment in &environments {
                    let found = cells.iter().find(|cell| {
                        cell.key.package == **package && cell.key.environment == **environment
                    });
                    let text = match found {
                        Some(cell) => grid_text(cell),
                        None => CellState::NotScheduled.label().to_owned(),
                    };
                    out.push_str(&format!(" {} |", cell_text(&text)));
                }
                out.push('\n');
            }
            out.push('\n');
        }
    }

    out.push_str(
        "Cells read `STATE pass/fail/skip`; a result produced off this run names its \
         origin.\n\n",
    );

    let reused: Vec<&Cell> = rollup
        .cells
        .iter()
        .filter(|cell| cell.origin.is_reused())
        .collect();
    if !reused.is_empty() {
        out.push_str(
            "### Reused results\n\nNo test runner started for these cells. Their evidence \
             is the Git note that carried the receipt; the full report stays on the \
             producing host.\n\n\
             | cell | origin | measurements | evidence |\n| --- | --- | --- | --- |\n",
        );
        for cell in reused {
            let evidence = cell.evidence.clone().unwrap_or_default();
            out.push_str(&format!(
                "| `{}` | {} | {} | `{}`{} |\n",
                cell_text(&cell.key.to_string()),
                cell.origin,
                cell_text(&evidence.measurements),
                cell_text(&evidence.reference),
                if evidence.host.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", cell_text(&evidence.host))
                },
            ));
        }
        out.push('\n');
    }

    let accepted: Vec<&Cell> = rollup
        .cells
        .iter()
        .filter(|cell| cell.state == CellState::AcceptedGap)
        .collect();
    if !accepted.is_empty() {
        out.push_str(
            "### Accepted policy gaps\n\nNeither a pass nor a test failure: the \
             environment cannot host the tier, and the absence is owned and dated.\n\n\
             | cell | owner | expires | reason | policy | closed by |\n\
             | --- | --- | --- | --- | --- | --- |\n",
        );
        for cell in accepted {
            let gap = cell.declared_gap.clone().unwrap_or_else(|| DeclaredGap {
                owner: String::new(),
                reason: "no governing policy entry".to_owned(),
                expiry: String::new(),
                capability: String::new(),
                policy: String::new(),
                closes: String::new(),
            });
            out.push_str(&format!(
                "| `{}` | {} | {} | {} | `{}` | {} |\n",
                cell_text(&cell.key.to_string()),
                cell_text(gap.owner_or_unassigned()),
                cell_text(if gap.expiry.is_empty() {
                    "no expiry"
                } else {
                    gap.expiry.as_str()
                }),
                cell_text(&gap.reason),
                cell_text(gap.policy_link()),
                cell_text(if gap.closes.is_empty() {
                    "unscheduled"
                } else {
                    gap.closes.as_str()
                }),
            ));
        }
        out.push_str(&format!("\n{REVOCATION_INSTRUCTIONS}.\n\n"));
    }

    let blocking: Vec<&Cell> = rollup
        .cells
        .iter()
        .filter(|cell| cell.state.blocks())
        .collect();
    if blocking.is_empty() {
        out.push_str("No cell fails the summary gate.\n");
    } else {
        out.push_str("### Cells failing the summary gate\n\n| cell | state | why |\n| --- | --- | --- |\n");
        for cell in blocking {
            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                cell_text(&cell.key.to_string()),
                cell.state.label(),
                cell_text(&why(cell))
            ));
        }
        out.push('\n');
    }

    out
}

/// One grid cell: its state, its counts, and — when the result did not come
/// from this run — where it did come from.
fn grid_text(cell: &Cell) -> String {
    match cell.state {
        CellState::NotScheduled | CellState::NothingToRun | CellState::AcceptedGap => {
            cell.state.label().to_owned()
        }
        _ => {
            let origin = if cell.origin.is_reused() {
                format!(" ({})", cell.origin)
            } else {
                String::new()
            };
            format!(
                "{} {}/{}/{}{origin}",
                cell.state.label(),
                cell.counts.passed,
                cell.counts.bad(),
                cell.counts.skipped
            )
        }
    }
}

/// One line explaining why a cell is in the state it is in. Failing test
/// identities are the answer when there are any; the classifier's reasons are
/// the answer when there are not.
fn why(cell: &Cell) -> String {
    const SHOWN: usize = 5;
    let mut parts = cell.reasons.clone();

    if !cell.failed_tests.is_empty() {
        let mut listed = cell
            .failed_tests
            .iter()
            .take(SHOWN)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if cell.failed_tests.len() > SHOWN {
            listed.push_str(&format!(" (+{} more)", cell.failed_tests.len() - SHOWN));
        }
        parts.push(format!("failing: {listed}"));
    }

    if parts.is_empty() {
        format!(
            "{} pass / {} fail / {} skip",
            cell.counts.passed,
            cell.counts.bad(),
            cell.counts.skipped
        )
    } else {
        parts.join("; ")
    }
}

fn render_verdict(findings: &[Finding], blocked: bool) -> String {
    let mut out = String::from("## CI verdict\n\n");
    out.push_str(if blocked {
        "**BLOCKED** — this run must not merge.\n\n"
    } else {
        "**CLEAR** — every non-green cell is accepted by the baseline, covered by an \
         owned and unexpired policy gap, or out of scope.\n\n"
    });

    if findings.is_empty() {
        out.push_str("No findings.\n");
        return out;
    }

    out.push_str("| severity | rule | subject | detail |\n| --- | --- | --- | --- |\n");
    for finding in findings {
        out.push_str(&format!(
            "| {} | `{}` | `{}` | {} |\n",
            match finding.severity {
                Severity::Block => "BLOCK",
                Severity::Note => "note",
            },
            cell_text(finding.rule),
            cell_text(&finding.subject),
            cell_text(&finding.detail),
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Dates (expiry evaluation without a date dependency)
// ---------------------------------------------------------------------------

/// Today in UTC as `YYYY-MM-DD`.
fn today_utc() -> Option<String> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// Days-since-epoch to a proleptic Gregorian date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

// ---------------------------------------------------------------------------
// Compare: is this branch worse than its base?
// ---------------------------------------------------------------------------

/// A cell observed in both runs, with the failure identities that moved.
#[derive(Clone, Debug)]
struct CellDelta {
    key: CellKey,
    base_state: CellState,
    head_state: CellState,
    /// Failing on head, not on base.
    new_failures: Vec<String>,
    /// Failing on base, not on head.
    fixed_failures: Vec<String>,
    /// Failing on both.
    shared_failures: usize,
}

impl CellDelta {
    /// Whether this cell got worse.
    ///
    /// A state that starts blocking is a regression even with no named test:
    /// `MISSING` and `POLICY GAP` carry no `failed_tests`, so identity-set
    /// comparison alone would score a cell that stopped reporting as unchanged.
    fn regressed(&self) -> bool {
        !self.new_failures.is_empty() || (self.head_state.blocks() && !self.base_state.blocks())
    }

    fn changed(&self) -> bool {
        self.regressed() || !self.fixed_failures.is_empty() || self.base_state != self.head_state
    }
}

/// A cell that carries no comparable evidence, and why.
///
/// Kept as data rather than dropped: a PR that schedules a narrower scope than
/// its base is the normal case, and silently omitting those cells would read as
/// "no difference" when the truth is "not measured".
#[derive(Clone, Debug)]
struct Incomparable {
    key: CellKey,
    reason: String,
}

#[derive(Debug, Default)]
struct Comparison {
    changed: Vec<CellDelta>,
    unchanged: usize,
    incomparable: Vec<Incomparable>,
}

impl Comparison {
    fn regressed(&self) -> bool {
        self.changed.iter().any(CellDelta::regressed)
    }
}

/// Compare two result documents cell by cell.
///
/// Comparison is keyed on `{package, environment, tier}` and on test
/// *identity*, aggregated to the cell first so the result is stable across
/// changes in how a suite's tests are distributed.
fn compare(base: &Rollup, head: &Rollup) -> Comparison {
    let base_cells: BTreeMap<&CellKey, &Cell> =
        base.cells.iter().map(|cell| (&cell.key, cell)).collect();
    let head_cells: BTreeMap<&CellKey, &Cell> =
        head.cells.iter().map(|cell| (&cell.key, cell)).collect();

    let mut result = Comparison::default();
    let keys: BTreeSet<&CellKey> = base_cells.keys().chain(head_cells.keys()).copied().collect();

    for key in keys {
        let (base_cell, head_cell) = match (base_cells.get(key), head_cells.get(key)) {
            (Some(b), Some(h)) => (*b, *h),
            (None, Some(_)) => {
                result.incomparable.push(Incomparable {
                    key: key.clone(),
                    reason: "absent from the base run".into(),
                });
                continue;
            }
            (Some(_), None) => {
                result.incomparable.push(Incomparable {
                    key: key.clone(),
                    reason: "absent from the head run".into(),
                });
                continue;
            }
            (None, None) => continue,
        };

        let unscheduled = match (
            base_cell.state == CellState::NotScheduled,
            head_cell.state == CellState::NotScheduled,
        ) {
            (true, true) => Some("not scheduled on either side"),
            (true, false) => Some("not scheduled on base"),
            (false, true) => Some("not scheduled on head"),
            (false, false) => None,
        };
        if let Some(reason) = unscheduled {
            result.incomparable.push(Incomparable {
                key: key.clone(),
                reason: reason.to_owned(),
            });
            continue;
        }

        let base_failures: BTreeSet<&str> =
            base_cell.failed_tests.iter().map(String::as_str).collect();
        let head_failures: BTreeSet<&str> =
            head_cell.failed_tests.iter().map(String::as_str).collect();

        let delta = CellDelta {
            key: key.clone(),
            base_state: base_cell.state,
            head_state: head_cell.state,
            new_failures: head_failures
                .difference(&base_failures)
                .map(|t| (*t).to_owned())
                .collect(),
            fixed_failures: base_failures
                .difference(&head_failures)
                .map(|t| (*t).to_owned())
                .collect(),
            shared_failures: base_failures.intersection(&head_failures).count(),
        };

        if delta.changed() {
            result.changed.push(delta);
        } else {
            result.unchanged += 1;
        }
    }

    result
}

fn render_comparison(comparison: &Comparison, base: &Rollup, head: &Rollup) -> String {
    let mut out = String::new();
    let describe = |rollup: &Rollup| match &rollup.run_id {
        Some(id) => format!("run {id}"),
        None => "unidentified run".to_owned(),
    };

    out.push_str("\n## CI comparison\n\n");
    out.push_str(&format!(
        "base: {} · head: {}\n\n",
        describe(base),
        describe(head)
    ));

    if comparison.regressed() {
        out.push_str("**REGRESSED** — the head run fails something its base does not.\n\n");
    } else {
        out.push_str(
            "**CLEAR** — the head run introduces no failure absent from its base.\n\n",
        );
    }

    let new_failures: Vec<_> = comparison
        .changed
        .iter()
        .flat_map(|d| d.new_failures.iter().map(move |t| (&d.key, t)))
        .collect();
    if !new_failures.is_empty() {
        out.push_str(&format!("### New failures ({})\n\n", new_failures.len()));
        out.push_str("| cell | test |\n|---|---|\n");
        for (key, test) in &new_failures {
            out.push_str(&format!("| `{key}` | `{test}` |\n"));
        }
        out.push('\n');
    }

    let state_regressions: Vec<_> = comparison
        .changed
        .iter()
        .filter(|d| d.head_state.blocks() && !d.base_state.blocks())
        .collect();
    if !state_regressions.is_empty() {
        out.push_str(&format!(
            "### Cell state regressions ({})\n\n",
            state_regressions.len()
        ));
        out.push_str("| cell | base | head |\n|---|---|---|\n");
        for delta in &state_regressions {
            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                delta.key, delta.base_state, delta.head_state
            ));
        }
        out.push('\n');
    }

    // Per-cell counts, not identities. The fixed set is routinely in the
    // hundreds when the base is even slightly stale, and every row of it is
    // something the reader is NOT being asked to act on.
    let fixed_total: usize = comparison.changed.iter().map(|d| d.fixed_failures.len()).sum();
    if fixed_total > 0 {
        out.push_str(&format!("### No longer failing ({fixed_total})\n\n"));
        out.push_str("| cell | count |\n|---|---|\n");
        for delta in comparison.changed.iter().filter(|d| !d.fixed_failures.is_empty()) {
            out.push_str(&format!(
                "| `{}` | {} |\n",
                delta.key,
                delta.fixed_failures.len()
            ));
        }
        out.push_str(
            "\nAttributable to the head branch only if the base is its merge-base. A base older than the branch also credits it with everything `main` fixed in between.\n\n",
        );
    }

    let carried: usize = comparison.changed.iter().map(|d| d.shared_failures).sum();
    if carried > 0 {
        out.push_str(&format!(
            "Pre-existing failures carried by changed cells: {carried} (unchanged, not this branch's).\n\n"
        ));
    }

    if !comparison.incomparable.is_empty() {
        out.push_str(&format!(
            "### Not comparable ({})\n\n",
            comparison.incomparable.len()
        ));
        out.push_str("These cells produced evidence on only one side, usually because the two runs scheduled different scopes. They are neither new nor fixed.\n\n");
        out.push_str("| cell | reason |\n|---|---|\n");
        for entry in &comparison.incomparable {
            out.push_str(&format!("| `{}` | {} |\n", entry.key, entry.reason));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "{} cell(s) identical between the two runs.\n",
        comparison.unchanged
    ));

    out
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const USAGE: &str = "\
ci-rollup — package × environment × tier CI rollup and merge verdict

USAGE:
  ci-rollup rollup    --artifacts <dir> [options]
  ci-rollup verdict   --results <file> --baseline <file> [options]
  ci-rollup compare   --base <file> --head <file> [options]
  ci-rollup summarize --results <file> [--results <file>…] [options]

ROLLUP OPTIONS:
  --artifacts <dir>              root holding the downloaded per-job artifacts
  --plan <file>                  the scope job's resolved execution plan
                                 (resolved-plan.json): every cell it scheduled,
                                 reused, or accepted as a policy gap
  --policy <file>                the scope job's policy artifact (scope.json), carrying
                                 every impacted package's resolved CI policy. Required
                                 without --plan; with it, supplies `gates = false`
                                 governance
  --environments <file>          environment capability table
                                 (default .github/ci/environments.json)
  --scope <a,b,c>                packages this run scheduled; repeatable. Omitting it
                                 takes scope from the plan, or infers it from the
                                 artifacts and marks the run degraded
  --area <a,b>                   emit only these areas' slice; repeatable. Each area
                                 then applies its own baseline, gaps, and missing-cell
                                 rule and nobody else's
  --expected-manifest <file>     expected-test manifest generated ON the target
                                 environment; repeatable
  --out <file>                   write the machine-readable result document
  --summary <file>               append Markdown (default $GITHUB_STEP_SUMMARY)
  --run-id <id>                  record the producing CI run

VERDICT OPTIONS:
  --results <file>               a result document written by `rollup --out`
  --baseline <file>              .github/ci/ci-baseline.toml
  --area <a,b>                   judge only these areas; repeatable
  --summary <file>               append Markdown (default $GITHUB_STEP_SUMMARY)
  --today <YYYY-MM-DD>           override today's date for expiry evaluation

SUMMARIZE OPTIONS:
  --results <file>               an area's result slice; repeatable
  --summary <file>               append Markdown (default $GITHUB_STEP_SUMMARY)

  Folds area slices into one view and applies NO policy: no baseline, no gap
  acceptance, no missing-cell rule, no merge decision. Each area's own rollup
  already made those decisions, and a second evaluation could contradict it.
  Always exits 0.

COMPARE OPTIONS:
  --base <file>                  result document to compare against (usually main)
  --head <file>                  result document under judgement (the branch)
  --summary <file>               append Markdown (default $GITHUB_STEP_SUMMARY)

  Answers `is this branch worse than its base`, which is the only question a
  repo with known-red cells can ask of a run. Cells scheduled on one side only
  are reported as not comparable rather than counted either way.

EXIT CODES:
  0  clear
  1  tool error (bad usage, unreadable input)
  2  blocked (verdict), or the head run regressed against its base (compare)
";

/// Subcommands, for the dispatch table and the unknown-command error.
const COMMANDS: [&str; 4] = ["rollup", "verdict", "compare", "summarize"];

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("ci-rollup: {err:#}");
            std::process::exit(EXIT_TOOL_ERROR);
        }
    }
}

#[derive(Debug, Default)]
struct Args {
    flags: BTreeMap<String, Vec<String>>,
}

impl Args {
    fn parse(raw: impl Iterator<Item = String>) -> Result<Self> {
        let mut flags: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut iter = raw.peekable();

        while let Some(token) = iter.next() {
            let Some(name) = token.strip_prefix("--") else {
                bail!("unexpected positional argument `{token}`");
            };
            let (name, inline) = match name.split_once('=') {
                Some((name, value)) => (name.to_owned(), Some(value.to_owned())),
                None => (name.to_owned(), None),
            };
            let value = match inline {
                Some(value) => value,
                None => iter
                    .next()
                    .ok_or_else(|| anyhow!("missing value for `--{name}`"))?,
            };
            flags.entry(name).or_default().push(value);
        }

        Ok(Self { flags })
    }

    fn one(&self, name: &str) -> Option<&str> {
        self.flags.get(name).and_then(|v| v.last()).map(String::as_str)
    }

    fn required(&self, name: &str) -> Result<&str> {
        self.one(name)
            .ok_or_else(|| anyhow!("`--{name}` is required\n\n{USAGE}"))
    }

    fn many(&self, name: &str) -> Vec<String> {
        self.flags.get(name).cloned().unwrap_or_default()
    }

    /// Comma-separated values, flattened across repeated occurrences.
    fn list(&self, name: &str) -> Vec<String> {
        self.many(name)
            .iter()
            .flat_map(|raw| raw.split(','))
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

fn run() -> Result<i32> {
    let mut argv = std::env::args().skip(1);
    let command = argv.next().unwrap_or_default();
    let args = Args::parse(argv)?;

    match command.as_str() {
        "rollup" => cmd_rollup(&args),
        "verdict" => cmd_verdict(&args),
        "compare" => cmd_compare(&args),
        "summarize" => cmd_summarize(&args),
        "--help" | "-h" | "help" | "" => {
            println!("{USAGE}");
            Ok(0)
        }
        other => bail!(
            "unknown subcommand `{other}`; expected one of {}\n\n{USAGE}",
            COMMANDS.join(", ")
        ),
    }
}

fn cmd_rollup(args: &Args) -> Result<i32> {
    let artifacts = PathBuf::from(args.required("artifacts")?);
    let plan_path = args.one("plan").map(PathBuf::from);
    let policy_path = args.one("policy").map(PathBuf::from);
    if plan_path.is_none() && policy_path.is_none() {
        bail!("`--plan` or `--policy` is required\n\n{USAGE}");
    }
    let environments_path = args
        .one("environments")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".github").join("ci").join("environments.json"));

    let policy_doc = match &policy_path {
        Some(path) => {
            let text = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            serde_json::from_str::<PolicyDoc>(&text)
                .with_context(|| format!("invalid package policy {}", path.display()))?
        }
        None => PolicyDoc { policy: Vec::new() },
    };
    let plan = match &plan_path {
        Some(path) => {
            let text = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            Some(
                serde_json::from_str::<ResolvedPlan>(&text)
                    .with_context(|| format!("invalid resolved plan {}", path.display()))?,
            )
        }
        None => None,
    };

    let environments_text = fs::read_to_string(&environments_path)
        .with_context(|| format!("failed to read {}", environments_path.display()))?;
    let environments_doc: EnvironmentsDoc = serde_json::from_str(&environments_text)
        .with_context(|| format!("invalid environments {}", environments_path.display()))?;
    if environments_doc.schema_version != 1 {
        bail!(
            "environments {} has schema_version {}; expected 1",
            environments_path.display(),
            environments_doc.schema_version
        );
    }

    let mut records = Vec::new();
    for dir in list_artifact_dirs(&artifacts)? {
        if dir.name.starts_with("status-") {
            continue;
        }
        records.extend(records_from_artifact(&dir)?);
    }
    let statuses = read_producer_statuses(&artifacts)?;

    let explicit_scope = args.list("scope");
    // The plan names every package it selected, so a run with a plan is never
    // scope-degraded: a package that produced nothing at all is still visible.
    let plan_scope: Vec<String> = plan
        .as_ref()
        .map(|plan| {
            plan.packages
                .iter()
                .map(|entry| entry.package.clone())
                .collect()
        })
        .unwrap_or_default();
    let scope_degraded = explicit_scope.is_empty() && plan_scope.is_empty();
    let scope: BTreeSet<String> = if !explicit_scope.is_empty() {
        explicit_scope.into_iter().collect()
    } else if !plan_scope.is_empty() {
        plan_scope.into_iter().collect()
    } else {
        records.iter().map(|record| record.package.clone()).collect()
    };

    let expected_tests = load_expected_manifests(args)?;

    // One expectation set, from the plan when there is one. The planner already
    // crossed policy, capabilities, and evidence; deriving a second expectation
    // here is how the scheduler and the report come to disagree.
    let all_expected = match &plan {
        Some(plan) => {
            let mut expected = plan_expected_cells(plan)?;
            for policy in &policy_doc.policy {
                if !policy.gates && scope.contains(&policy.package) {
                    expected.extend(exclusion_cells(policy, &environments_doc.environments));
                }
            }
            expected.sort_by(|left, right| left.key.cmp(&right.key));
            expected
        }
        None => expected_cells(&policy_doc.policy, &scope, &environments_doc.environments),
    };
    // `lint` and `check` stage no JUnit; their evidence is an explicit producer
    // status, so they are classified from `status_cells` rather than from the
    // reports on disk.
    let (expected, planned_gates): (Vec<ExpectedCell>, Vec<ExpectedCell>) = all_expected
        .into_iter()
        .partition(|cell| is_test_tier(&cell.key.tier));

    let mut cells = classify(&ClassifyInputs {
        expected: &expected,
        records: &records,
        statuses: &statuses,
        expected_tests: &expected_tests,
    });
    cells.extend(status_cells(
        &statuses,
        &scope,
        &cells,
        &policy_doc.policy,
        &planned_gates,
    ));
    cells.sort_by(|left, right| left.key.cmp(&right.key));

    let accepted_evidence = accepted_evidence(&cells);
    let mut scheduled: Vec<CellKey> = expected.iter().map(|cell| cell.key.clone()).collect();
    scheduled.extend(planned_gates.iter().map(|cell| cell.key.clone()));

    let rollup = Rollup {
        schema_version: RESULT_SCHEMA_VERSION,
        run_id: args.one("run-id").map(str::to_owned),
        scope: scope.into_iter().collect(),
        areas: derived_areas(&cells),
        area_scope: Vec::new(),
        accepted_evidence,
        scope_degraded,
        scheduled: Some(scheduled),
        records,
        cells,
    };

    let rollup = narrow(rollup, &args.list("area"))?;

    if let Some(out) = args.one("out") {
        let json = serde_json::to_string_pretty(&rollup)?;
        write_out(Path::new(out), &json)?;
    }

    let markdown = render_grid(&rollup);
    append_summary(args, &markdown)?;
    print!("{markdown}");

    let blocked = rollup.cells.iter().any(|cell| cell.state.blocks());
    Ok(if blocked { EXIT_BLOCKED } else { 0 })
}

/// Apply the `--area` narrowing, refusing a narrowing that selects nothing.
///
/// ## Errors
///
/// An area naming no cell in a document that has cells. That is a broken
/// invocation — a misspelled area, or a slice asked of a document that predates
/// area derivation — and the honest answer is a tool error. Left unchecked it
/// would produce a green verdict over an empty grid, which is precisely the
/// vacuous pass the area-owned model exists to prevent.
fn narrow(rollup: Rollup, areas: &[String]) -> Result<Rollup> {
    if areas.is_empty() {
        return Ok(rollup);
    }
    let wanted: BTreeSet<String> = areas.iter().cloned().collect();
    let narrowed = rollup.narrowed(&wanted);
    if narrowed.cells.is_empty() && !rollup.cells.is_empty() {
        bail!(
            "`--area {}` selects no cell; the document covers {}",
            areas.join(","),
            if rollup.areas.is_empty() {
                "no area at all".to_owned()
            } else {
                rollup.areas.join(", ")
            }
        );
    }
    Ok(narrowed)
}

/// The evidence behind every reused cell, in cell order.
///
/// Derived from the cells rather than copied from the plan so the document
/// cannot claim evidence for a cell it did not report, or report a reused cell
/// whose evidence it does not carry.
fn accepted_evidence(cells: &[Cell]) -> Vec<AcceptedEvidence> {
    cells
        .iter()
        .filter(|cell| cell.origin.is_reused())
        .map(|cell| {
            let evidence = cell.evidence.clone().unwrap_or_default();
            AcceptedEvidence {
                key: cell.key.clone(),
                area: cell.area.clone(),
                origin: cell.origin,
                measurements: evidence.measurements.clone(),
                evidence,
            }
        })
        .collect()
}

fn cmd_verdict(args: &Args) -> Result<i32> {
    let results_path = PathBuf::from(args.required("results")?);
    let baseline_path = PathBuf::from(args.required("baseline")?);

    let results_text = fs::read_to_string(&results_path)
        .with_context(|| format!("failed to read {}", results_path.display()))?;
    let rollup: Rollup = serde_json::from_str(&results_text)
        .with_context(|| format!("invalid result document {}", results_path.display()))?;
    reject_old_schema(rollup.schema_version, &results_path)?;

    // Narrowing to an area is what makes the outcome area-owned: this verdict
    // sees that area's cells, that area's scope, and that area's accepted
    // evidence, so another area's failure can neither block it nor be excused
    // by it (spec section 5).
    let rollup = narrow(rollup, &args.list("area"))?;

    let baseline = load_baseline(&baseline_path)?;
    let today = args.one("today").map(str::to_owned).or_else(today_utc);
    let findings = verdict(&rollup, &baseline, today.as_deref());
    let blocked = findings
        .iter()
        .any(|finding| finding.severity == Severity::Block);

    let markdown = render_verdict(&findings, blocked);
    append_summary(args, &markdown)?;
    print!("{markdown}");

    Ok(if blocked { EXIT_BLOCKED } else { 0 })
}

fn cmd_compare(args: &Args) -> Result<i32> {
    let base = load_rollup(Path::new(args.required("base")?))?;
    let head = load_rollup(Path::new(args.required("head")?))?;

    let comparison = compare(&base, &head);
    let markdown = render_comparison(&comparison, &base, &head);
    append_summary(args, &markdown)?;
    print!("{markdown}");

    Ok(if comparison.regressed() {
        EXIT_BLOCKED
    } else {
        0
    })
}

/// Fold several area result slices into one reader-facing view.
///
/// ## Notes
///
/// This applies no policy: no baseline, no gap acceptance, no missing-cell
/// rule, no merge decision. Each area's own rollup already took those decisions
/// and owns its outcome; a combined job that evaluated them again could
/// contradict a green area, which is the global verdict this specification
/// removes (spec section 5). It therefore always exits 0 — its failure mode is
/// being unreadable, not being wrong.
fn cmd_summarize(args: &Args) -> Result<i32> {
    let paths = args.many("results");
    if paths.is_empty() {
        bail!("`--results` is required\n\n{USAGE}");
    }
    let slices = paths
        .iter()
        .map(|raw| load_rollup(Path::new(raw)))
        .collect::<Result<Vec<Rollup>>>()?;

    let markdown = render_combined_summary(&slices);
    append_summary(args, &markdown)?;
    print!("{markdown}");
    Ok(0)
}

/// Render the combined view: one row per area, counted from the states its own
/// rollup already recorded.
fn render_combined_summary(slices: &[Rollup]) -> String {
    let mut by_area: BTreeMap<String, Vec<&Cell>> = BTreeMap::new();
    for slice in slices {
        for cell in &slice.cells {
            by_area.entry(cell.area.clone()).or_default().push(cell);
        }
    }

    let mut out = String::from("## CI results by area\n\n");
    out.push_str(
        "A reporting view of the slices each area's own rollup produced. It applies no \
         baseline, gap, missing-cell, or merge policy: each area owns its outcome.\n\n",
    );
    out.push_str(
        "| area | cells | passed | failed | missing | accepted gaps | reused | tests |\n\
         | --- | --- | --- | --- | --- | --- | --- | --- |\n",
    );

    for (area, cells) in &by_area {
        let count = |state: CellState| cells.iter().filter(|cell| cell.state == state).count();
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            cell_text(if area.is_empty() { "(ungrouped)" } else { area }),
            cells.len(),
            count(CellState::Pass),
            count(CellState::Fail),
            count(CellState::Missing),
            count(CellState::AcceptedGap),
            cells.iter().filter(|cell| cell.origin.is_reused()).count(),
            cells.iter().map(|cell| cell.counts.total).sum::<u32>(),
        ));
    }

    let evidence: Vec<&AcceptedEvidence> = slices
        .iter()
        .flat_map(|slice| slice.accepted_evidence.iter())
        .collect();
    if !evidence.is_empty() {
        out.push_str(&format!(
            "\n{} cell(s) were satisfied by verified evidence rather than executed; each \
             area's slice links the note that carried it.\n",
            evidence.len()
        ));
    }

    out
}

fn load_rollup(path: &Path) -> Result<Rollup> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let rollup: Rollup = serde_json::from_str(&text)
        .with_context(|| format!("invalid result document {}", path.display()))?;
    reject_old_schema(rollup.schema_version, path)?;
    Ok(rollup)
}

/// Refuse a result document from another schema generation, in both
/// directions. Version 1 was area-keyed; version 2 keys every identity on the
/// package. Reading one as the other would silently mis-key every cell, so the
/// error names the migration rather than just the mismatch.
fn reject_old_schema(version: u32, path: &Path) -> Result<()> {
    if version > RESULT_SCHEMA_VERSION {
        bail!(
            "result schema_version {} in {} is newer than this tool understands \
             ({RESULT_SCHEMA_VERSION})",
            version,
            path.display()
        );
    }
    if version == 1 {
        bail!(
            "result document {} is schema_version 1 (area-keyed); this tool reads \
             schema_version {RESULT_SCHEMA_VERSION} (package-keyed, area-grouped). Re-run \
             the rollup that produced it — see fixes/2026-08-06-cicd/plan.md (Phase 4)",
            path.display()
        );
    }
    if version < RESULT_SCHEMA_VERSION {
        bail!(
            "result document {} is schema_version {version}; this tool reads \
             schema_version {RESULT_SCHEMA_VERSION}, which adds each cell's area, origin, \
             and evidence. Re-run the rollup that produced it — see \
             fixes/2026-09-11-cicd-cleanup/plan.md (Phase 5)",
            path.display()
        );
    }
    Ok(())
}

type ExpectedTests = BTreeMap<(String, Tier), BTreeMap<String, Vec<String>>>;

fn load_expected_manifests(args: &Args) -> Result<ExpectedTests> {
    let mut expected: ExpectedTests = BTreeMap::new();
    for raw in args.many("expected-manifest") {
        let path = PathBuf::from(raw);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let manifest: ExpectedManifest = serde_json::from_str(&text)
            .with_context(|| format!("invalid expected-test manifest {}", path.display()))?;
        if manifest.schema_version > EXPECTED_MANIFEST_SCHEMA_VERSION {
            bail!(
                "expected-test manifest {} has schema_version {} (max \
                 {EXPECTED_MANIFEST_SCHEMA_VERSION})",
                path.display(),
                manifest.schema_version
            );
        }
        let bucket = expected
            .entry((manifest.environment.clone(), manifest.tier.clone()))
            .or_default();
        for (package, tests) in manifest.packages {
            bucket.entry(package).or_default().extend(tests);
        }
    }
    Ok(expected)
}

fn write_out(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn append_summary(args: &Args, markdown: &str) -> Result<()> {
    let target = args
        .one("summary")
        .map(str::to_owned)
        .or_else(|| std::env::var("GITHUB_STEP_SUMMARY").ok());
    let Some(target) = target else { return Ok(()) };
    if target.trim().is_empty() {
        return Ok(());
    }

    use std::io::Write;
    let path = Path::new(&target);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).ok();
        }
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open summary {}", path.display()))?;
    file.write_all(markdown.as_bytes())
        .with_context(|| format!("failed to write summary {}", path.display()))
}

#[cfg(test)]
#[path = "ci-rollup-tests.rs"]
mod tests;
