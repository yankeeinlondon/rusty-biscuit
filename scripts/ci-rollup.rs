//! Roll per-job JUnit artifacts into a package × environment × tier grid, and
//! render a merge verdict against a machine-readable baseline.
//!
//! Two subcommands share one data model:
//!
//! - `rollup` walks the downloaded CI artifacts, parses every staged JUnit
//!   document, crosses the observations with the run's resolved package policy
//!   (the scope job's artifact) and the environment capability table, and
//!   emits both a Markdown grid (for `GITHUB_STEP_SUMMARY`) and a
//!   machine-readable result document.
//! - `verdict` diffs that result document against `.github/ci/ci-baseline.toml`,
//!   rules on the capability-derived policy gaps the rollup recorded, and exits
//!   non-zero when the run must not merge. It is the single required
//!   branch-protection check, so it must run even when every producer job
//!   failed.
//!
//! The split is observation versus judgement. `rollup` never consults the
//! baseline and never excuses anything: its grid shows every cell that is not
//! green, including ones the verdict will go on to accept. All merge policy —
//! baselined failures and governed policy gaps alike — lives in `verdict`.
//!
//! ## Notes
//!
//! Cell identity is `{package, environment, tier}`. It is never derived from a
//! GitHub job display name: when `needs:` skips a matrix job, GitHub never
//! evaluates the matrix context and reports the raw, un-interpolated name
//! expression, so nothing is recoverable from it at all.
//!
//! The absence of an artifact is therefore ambiguous on its own. `NOT
//! SCHEDULED` versus `MISSING` is resolved from policy (the scope artifact
//! crossed with the run's affected scope), never from what happens to be on
//! disk.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

/// Version of both emitted documents (`results.json`) and the accepted
/// baseline. Version 2 keys every identity on the package; version 1 was
/// area-keyed and is rejected with an explicit migration error. A consumer
/// that reads a higher version must refuse to interpret it.
const SCHEMA_VERSION: u32 = 2;

/// Process exit codes. The verdict is destined to be the single required
/// branch-protection check, so a blocked run and a broken tool must be
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
        }
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
    /// `""` when undeclared. See [`PolicyGap::expiry`].
    #[serde(default)]
    expiry: String,
}

impl DeclaredGap {
    fn owner_or_unassigned(&self) -> &str {
        if self.owner.trim().is_empty() {
            "unassigned"
        } else {
            self.owner.as_str()
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Cell {
    #[serde(flatten)]
    key: CellKey,
    state: CellState,
    counts: Counts,
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
    /// True when the scope was inferred from the artifacts on disk instead of
    /// being supplied. An inferred scope cannot see a package that produced
    /// nothing at all, which is exactly the case `MISSING` exists to catch.
    scope_degraded: bool,
    records: Vec<RunRecord>,
    cells: Vec<Cell>,
}

impl Rollup {
    fn cell(&self, key: &CellKey) -> Option<&Cell> {
        self.cells.iter().find(|cell| &cell.key == key)
    }
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
    #[serde(default = "default_true")]
    gates: bool,
    /// CI-gating tiers this package owns. Always contains L1 for a gating
    /// package; L2 and browser are declared opt-ins.
    #[serde(default = "default_tiers")]
    tiers: Vec<Tier>,
    /// L2 terminal backends the package's tests require.
    #[serde(default)]
    l2_backends: Vec<String>,
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
    fn governed_gap(&self) -> Option<DeclaredGap> {
        match self {
            Self::Governed {
                available: false,
                reason,
                owner,
                expiry,
            } => Some(DeclaredGap {
                owner: owner.clone(),
                reason: reason.clone(),
                expiry: expiry.clone(),
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
        self.capabilities.get(capability)?.governed_gap()
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
    /// L2 backends this package's tests require, empty when the tier is
    /// backend-agnostic.
    backends: Vec<String>,
    /// Set when the environment cannot host this cell's tier.
    gap: Option<GapStatus>,
    /// Set when the package is `gates = false`: the cell renders NOT SCHEDULED
    /// with this governance metadata rather than running.
    exclusion: Option<Exclusion>,
}

/// Cross the resolved package policy with the run's affected scope to derive
/// every cell that should have produced evidence.
///
/// ## Notes
///
/// An L2 tier is expected on **every** environment, not only the ones with a
/// provisionable backend. Omitting the unprovisioned ones would make them
/// vanish from the grid, which is the failure mode `POLICY GAP` exists to
/// prevent. The browser tier is expected only where the capability table says
/// a headless browser can be hosted, matching the tier's Linux-only policy.
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

        if let Some(exclusion) = &policy.exclusion {
            // `gates = false`: visible, governed NOT SCHEDULED cells — never a
            // pass, never a silent absence.
            for environment in environments {
                expected.push(ExpectedCell {
                    key: CellKey {
                        package: policy.package.clone(),
                        environment: environment.name.clone(),
                        tier: Tier::L1,
                    },
                    backends: Vec::new(),
                    gap: None,
                    exclusion: Some(exclusion.clone()),
                });
            }
            continue;
        }
        if !policy.gates {
            continue;
        }

        for environment in environments {
            for tier in &policy.tiers {
                let (backends, gap) = match tier {
                    Tier::L2 => {
                        let gap = if environment.capable("tmux") {
                            None
                        } else {
                            Some(match environment.gap_for("tmux") {
                                Some(governed) => GapStatus::Governed(governed),
                                None => GapStatus::Ungoverned,
                            })
                        };
                        (policy.l2_backends.clone(), gap)
                    }
                    Tier::Browser => {
                        // Browser cells exist only where a headless browser can
                        // be hosted at all; elsewhere the tier is not
                        // scheduled, which is capability policy, not a gap.
                        if !environment.capable("headless_browser") {
                            continue;
                        }
                        (Vec::new(), None)
                    }
                    _ => (Vec::new(), None),
                };
                expected.push(ExpectedCell {
                    key: CellKey {
                        package: policy.package.clone(),
                        environment: environment.name.clone(),
                        tier: tier.clone(),
                    },
                    backends,
                    gap,
                    exclusion: None,
                });
            }
        }
    }

    expected.sort_by(|left, right| left.key.cmp(&right.key));
    expected
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

    for &index in indices {
        let record = &inputs.records[index];
        counts.add(&record.counts);
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
        reasons.push(format!("governed policy gap: {}", gap.describe()));
    } else if matches!(expectation.and_then(|cell| cell.gap.as_ref()), Some(GapStatus::Ungoverned)) {
        // An *ungoverned* gap: the package declares this tier on an environment
        // the capability table records as plainly unable to host it, with no
        // owner, reason, or expiry. `affected_scope.py` is supposed to keep
        // such a table from landing; catching it here too means a config-side
        // regression surfaces as a blocking cell rather than a green one.
        policy_gap = true;
        reasons.push(format!(
            "UNGOVERNED policy gap: tier requires backend(s) [{}] but {} cannot \
             host an L2 backend, and environments.json records no governed \
             unavailability for it",
            backends.join(", "),
            key.environment
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
    let own_status = inputs.statuses.iter().find(|status| {
        status.package == key.package
            && Tier::parse(&status.job) == key.tier
            && status.environment.as_deref() == Some(key.environment.as_str())
    });

    let state = if let Some(exclusion) = &exclusion {
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

    Cell {
        key: key.clone(),
        state,
        counts: Counts {
            skipped: counts.skipped + absent_skips.len() as u32,
            ..counts
        },
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
/// block, under the same stable `{area, environment, tier}` identity as a test
/// tier.
///
/// ## Notes
///
/// This is why the verdict can be the single required check. GitHub's own job
/// name for a skipped matrix leg is an un-interpolated expression, so the run's
/// job list cannot supply this; an explicit status artifact can.
fn status_cells(
    statuses: &[ProducerStatus],
    scope: &BTreeSet<String>,
    existing: &[Cell],
) -> Vec<Cell> {
    let mut cells = Vec::new();

    for status in statuses {
        if !scope.contains(&status.package) {
            continue;
        }
        let tier = Tier::parse(&status.job);
        if matches!(tier, Tier::L1 | Tier::L2 | Tier::L3 | Tier::Browser | Tier::Real) {
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

        let (state, reason) = match status.result.as_str() {
            "success" => (CellState::Pass, None),
            "failure" => (CellState::Fail, None),
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
        };

        cells.push(Cell {
            key,
            state,
            counts: Counts::default(),
            scheduled: true,
            skipped_tests: Vec::new(),
            failed_tests: Vec::new(),
            skip_evidence_degraded: false,
            declared_gap: None,
            reasons: reason.into_iter().collect(),
            records: Vec::new(),
        });
    }

    cells
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
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    failure: Vec<FailureEntry>,
    #[serde(default)]
    skip: Vec<SkipEntry>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

/// A known-red leg, keyed by stable identity — never by a GitHub display name.
#[derive(Clone, Debug, Deserialize)]
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

    if baseline.schema_version > SCHEMA_VERSION {
        bail!(
            "baseline schema_version {} is newer than this tool understands ({SCHEMA_VERSION})",
            baseline.schema_version
        );
    }
    if baseline.schema_version < SCHEMA_VERSION {
        bail!(
            "baseline {} is schema_version {} (area-keyed); this tool reads \
             schema_version {SCHEMA_VERSION} (package-keyed). Re-key the entries to \
             packages — see fixes/2026-08-06-cicd/plan.md (Phase 4)",
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

fn validate_date(value: Option<&str>, kind: &str, area: &str) -> Result<()> {
    let Some(value) = value else { return Ok(()) };
    if !is_iso_date(value) {
        bail!("{kind} entry for `{area}` has an invalid expiry `{value}`; expected YYYY-MM-DD");
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
        if cell.state == CellState::PolicyGap && ruled_on_gaps.contains(&cell.key) {
            continue;
        }
        let detail = why(cell);
        findings.push(Finding::block(
            match cell.state {
                CellState::Missing => "cell-missing",
                CellState::PolicyGap => "cell-policy-gap",
                _ => "cell-failed",
            },
            cell.key.to_string(),
            format!("{} — {detail}", cell.state),
        ));
    }

    findings.extend(skip_findings(rollup, baseline, &scope));
    findings
}

/// Decide, for every `POLICY GAP` cell, whether it is acknowledged work or an
/// unexplained hole. This is the entire accept/block decision for the state.
///
/// Returns the findings to report and the cells this function has already ruled
/// on — accepted *or* rejected — which the caller must therefore not also emit a
/// generic `cell-policy-gap` block for. One cell, one row, whichever way it
/// went.
///
/// ## Notes
///
/// Acknowledged is not acceptable, and neither is invisible. An accepted gap
/// still renders `POLICY GAP` in the grid — `classify` decided that and nothing
/// here can change it — still appears in the rollup's failing-cells table, and
/// still appears in the verdict table as a `note` naming its owner and expiry,
/// exactly as an accepted baseline failure does. Acceptance buys one thing: the
/// run may merge. This is deliberately *not* `soft_os`, which removed the leg
/// from the verdict altogether (plan §1.4).
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
/// not sufficient: `verdict` is the single required branch-protection check and
/// runs `if: always()`, precisely so a failed, skipped, or never-scheduled
/// scope job cannot suppress the verdict. If expiry lived only in the Python,
/// an expired gap would be excused by the one check that actually gates merging
/// whenever the check that catches it did not run.
fn policy_gap_findings(rollup: &Rollup, today: Option<&str>) -> (Vec<Finding>, BTreeSet<CellKey>) {
    let mut findings = Vec::new();
    let mut ruled_on: BTreeSet<CellKey> = BTreeSet::new();

    for cell in &rollup.cells {
        if cell.state != CellState::PolicyGap {
            continue;
        }
        let subject = cell.key.to_string();
        let Some(gap) = &cell.declared_gap else {
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
                "owned, unexpired policy gap: {} (owner {}, expires {expiry})",
                gap.reason, gap.owner
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
    let mut out = String::from("## CI rollup\n\n");

    if rollup.scope_degraded {
        out.push_str(
            "> **Scope was inferred from the artifacts on disk.** An area that produced \
             nothing at all is invisible to an inferred scope, which is precisely the case \
             `MISSING` exists to catch. Pass `--scope`.\n\n",
        );
    }

    let tiers: BTreeSet<Tier> = rollup.cells.iter().map(|cell| cell.key.tier.clone()).collect();

    for tier in &tiers {
        let cells: Vec<&Cell> = rollup
            .cells
            .iter()
            .filter(|cell| &cell.key.tier == tier)
            .collect();
        let environments: BTreeSet<&String> =
            cells.iter().map(|cell| &cell.key.environment).collect();
        let packages: BTreeSet<&String> = cells.iter().map(|cell| &cell.key.package).collect();

        out.push_str(&format!("### {tier}\n\n| package |"));
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
                    Some(cell) => match cell.state {
                        CellState::NotScheduled | CellState::NothingToRun => {
                            cell.state.label().to_owned()
                        }
                        _ => format!(
                            "{} {}/{}/{}",
                            cell.state.label(),
                            cell.counts.passed,
                            cell.counts.bad(),
                            cell.counts.skipped
                        ),
                    },
                    None => CellState::NotScheduled.label().to_owned(),
                };
                out.push_str(&format!(" {} |", cell_text(&text)));
            }
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("Cells read `STATE pass/fail/skip`.\n\n");

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
  ci-rollup rollup  --artifacts <dir> [options]
  ci-rollup verdict --results <file> --baseline <file> [options]
  ci-rollup compare --base <file> --head <file> [options]

ROLLUP OPTIONS:
  --artifacts <dir>              root holding the downloaded per-job artifacts
  --policy <file>                the scope job's policy artifact (scope.json), carrying
                                 every impacted package's resolved CI policy
  --environments <file>          environment capability table
                                 (default .github/ci/environments.json)
  --scope <a,b,c>                packages this run scheduled; repeatable. Omitting it
                                 infers scope from artifacts and marks the run degraded
  --expected-manifest <file>     expected-test manifest generated ON the target
                                 environment; repeatable
  --out <file>                   write the machine-readable result document
  --summary <file>               append Markdown (default $GITHUB_STEP_SUMMARY)
  --run-id <id>                  record the producing CI run

VERDICT OPTIONS:
  --results <file>               a result document written by `rollup --out`
  --baseline <file>              .github/ci/ci-baseline.toml
  --summary <file>               append Markdown (default $GITHUB_STEP_SUMMARY)
  --today <YYYY-MM-DD>           override today's date for expiry evaluation

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
        "--help" | "-h" | "help" | "" => {
            println!("{USAGE}");
            Ok(0)
        }
        other => bail!("unknown subcommand `{other}`\n\n{USAGE}"),
    }
}

fn cmd_rollup(args: &Args) -> Result<i32> {
    let artifacts = PathBuf::from(args.required("artifacts")?);
    let policy_path = PathBuf::from(args.required("policy")?);
    let environments_path = args
        .one("environments")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".github").join("ci").join("environments.json"));

    let policy_text = fs::read_to_string(&policy_path)
        .with_context(|| format!("failed to read {}", policy_path.display()))?;
    let policy_doc: PolicyDoc = serde_json::from_str(&policy_text)
        .with_context(|| format!("invalid package policy {}", policy_path.display()))?;

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
    let scope_degraded = explicit_scope.is_empty();
    let scope: BTreeSet<String> = if scope_degraded {
        records.iter().map(|record| record.package.clone()).collect()
    } else {
        explicit_scope.into_iter().collect()
    };

    let expected_tests = load_expected_manifests(args)?;

    let expected = expected_cells(&policy_doc.policy, &scope, &environments_doc.environments);
    let mut cells = classify(&ClassifyInputs {
        expected: &expected,
        records: &records,
        statuses: &statuses,
        expected_tests: &expected_tests,
    });
    cells.extend(status_cells(&statuses, &scope, &cells));
    cells.sort_by(|left, right| left.key.cmp(&right.key));

    let rollup = Rollup {
        schema_version: SCHEMA_VERSION,
        run_id: args.one("run-id").map(str::to_owned),
        scope: scope.into_iter().collect(),
        scope_degraded,
        records,
        cells,
    };

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

fn cmd_verdict(args: &Args) -> Result<i32> {
    let results_path = PathBuf::from(args.required("results")?);
    let baseline_path = PathBuf::from(args.required("baseline")?);

    let results_text = fs::read_to_string(&results_path)
        .with_context(|| format!("failed to read {}", results_path.display()))?;
    let rollup: Rollup = serde_json::from_str(&results_text)
        .with_context(|| format!("invalid result document {}", results_path.display()))?;
    reject_old_schema(rollup.schema_version, &results_path)?;

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
    if version > SCHEMA_VERSION {
        bail!(
            "result schema_version {} in {} is newer than this tool understands ({SCHEMA_VERSION})",
            version,
            path.display()
        );
    }
    if version < SCHEMA_VERSION {
        bail!(
            "result document {} is schema_version {version} (area-keyed); this tool reads \
             schema_version {SCHEMA_VERSION} (package-keyed). Re-run the rollup that \
             produced it — see fixes/2026-08-06-cicd/plan.md (Phase 4)",
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
        if manifest.schema_version > SCHEMA_VERSION {
            bail!(
                "expected-test manifest {} has schema_version {} (max {SCHEMA_VERSION})",
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
