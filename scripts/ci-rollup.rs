//! Roll per-job JUnit artifacts into an area × environment × tier grid, and
//! render a merge verdict against a machine-readable baseline.
//!
//! Two subcommands share one data model:
//!
//! - `rollup` walks the downloaded CI artifacts, parses every staged JUnit
//!   document, crosses the observations with `areas.json` policy, and emits
//!   both a Markdown grid (for `GITHUB_STEP_SUMMARY`) and a machine-readable
//!   result document.
//! - `verdict` diffs that result document against `.github/ci/ci-baseline.toml`,
//!   rules on the `areas.json` policy gaps the rollup recorded, and exits
//!   non-zero when the run must not merge. It is the single required
//!   branch-protection check, so it must run even when every producer job
//!   failed.
//!
//! The split is observation versus judgement. `rollup` never consults the
//! baseline and never excuses anything: its grid shows every cell that is not
//! green, including ones the verdict will go on to accept. All merge policy —
//! baselined failures and declared policy gaps alike — lives in `verdict`.
//!
//! ## Notes
//!
//! Cell identity is `{area, environment, tier}` and record identity is
//! `{area, environment, tier, shard, package}`. Neither is ever derived from a
//! GitHub job display name: when `needs: lint` skips a matrix job, GitHub never
//! evaluates the matrix context and reports the raw, un-interpolated name
//! expression, so `os` and `shard` are not recoverable from it at all.
//!
//! The absence of an artifact is therefore ambiguous on its own. `NOT
//! SCHEDULED` versus `MISSING` is resolved from policy (`areas.json` crossed
//! with the run's affected scope), never from what happens to be on disk.

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
/// baseline. A consumer that reads a higher version must refuse to interpret it.
const SCHEMA_VERSION: u32 = 1;

/// Tiers L2 and browser run only on Linux today. Overridable so Phase 3 can
/// widen them without a code change.
const DEFAULT_TIER_ENVIRONMENTS: &str = "ubuntu-latest";

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
/// NotApplicable → Skip → Pass`, and `Pass` additionally requires at least one
/// *executed* test. `Fail` outranks `PolicyGap` because a real failure is the
/// more actionable signal and both block; the policy-gap observation is still
/// recorded in the cell's `reasons`, so nothing is lost by the ordering.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum CellState {
    Pass,
    Fail,
    Skip,
    #[serde(rename = "N/A")]
    NotApplicable,
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
            Self::NotApplicable => "N/A",
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
    /// first and an owned, unexpired `areas.json` record can excuse the second:
    /// the merge decision belongs to `verdict`, and keeping this predicate
    /// policy-free is what makes the rollup's "Cells failing the summary gate"
    /// table an honest inventory of everything not green.
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

/// One nextest invocation. Shard and package identity are preserved here so a
/// count can be traced back to the invocation that produced it without parsing
/// any display name.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RunRecord {
    area: String,
    environment: String,
    tier: Tier,
    /// nextest `--partition` selector, `"1/1"` when unsharded.
    shard: String,
    package: String,
    /// Downloaded artifact directory this record came from.
    artifact: String,
    /// True when identity was recovered from the artifact directory name
    /// because no manifest record covered this report.
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
            area: self.area.clone(),
            environment: self.environment.clone(),
            tier: self.tier.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
struct CellKey {
    area: String,
    environment: String,
    tier: Tier,
}

impl fmt::Display for CellKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.area, self.environment, self.tier)
    }
}

/// The `areas.json` policy gap that explains a `POLICY GAP` cell, carried into
/// `results.json` verbatim.
///
/// ## Notes
///
/// `verdict` reads only the result document — never `areas.json` — so the
/// accountability fields have to travel with the cell. Rendering the gap into a
/// prose `reasons` line is not enough: the verdict has to decide on `owner`,
/// `reason`, and `expiry` individually, and re-parsing them out of prose would
/// be a second, drift-prone encoding of the same policy.
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
    /// Shards policy expected minus shards that produced a record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    missing_shards: Vec<String>,
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
    /// The `areas.json` gap covering this cell, when one is declared. `None` on
    /// a `POLICY GAP` cell means the gap is *undeclared* — inferred from
    /// backend provisioning — which the verdict must never excuse.
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
    /// Areas policy considered in scope for this run.
    scope: Vec<String>,
    /// True when the scope was inferred from the artifacts on disk instead of
    /// being supplied. An inferred scope cannot see an area that produced
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
/// `_stage_junit`. `area`, `environment`, and `shard` are `""` when unset
/// (local dev) — never invented by the producer, so never inferred here either.
#[derive(Clone, Debug, Deserialize)]
struct ManifestRecord {
    tier: String,
    package: String,
    xml: String,
    #[serde(default)]
    exit_code: i64,
    #[serde(default)]
    area: String,
    #[serde(default)]
    environment: String,
    #[serde(default)]
    shard: String,
    #[serde(default)]
    duration_s: u64,
    #[serde(default)]
    report_present: bool,
}

/// Explicit status for a producer job, so a job that failed before it could
/// stage anything is distinguishable from one that was never scheduled.
///
/// Uploaded as `status-<area>-<job>/status.json`.
#[derive(Clone, Debug, Deserialize)]
struct ProducerStatus {
    area: String,
    job: String,
    /// GitHub `needs.<job>.result`: `success`, `failure`, `cancelled`, `skipped`.
    result: String,
    #[serde(default)]
    environment: Option<String>,
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
// Policy: areas.json
// ---------------------------------------------------------------------------

/// One `areas.json` record. Read-only — `affected_scope.py` owns this schema,
/// and unknown fields are ignored so a concurrent schema addition cannot break
/// the rollup.
#[derive(Clone, Debug, Deserialize)]
struct AreaPolicy {
    area: String,
    #[serde(default = "default_true")]
    ci: bool,
    /// Environments running the full canonical L1 suite.
    ///
    /// `environment` is not `os`: `wsl2-ubuntu` is a distinct Linux environment
    /// hosted by a Windows runner. Retired `full_os` is accepted as an alias so
    /// an artifact produced before the rename still rolls up.
    #[serde(default = "default_environments", alias = "full_os")]
    environments: Vec<String>,
    #[serde(default = "default_shards")]
    shards: Vec<String>,
    #[serde(default)]
    l2: bool,
    #[serde(default)]
    browser: bool,
    #[serde(default)]
    backends: Vec<String>,
    /// Tiers this area owns tests for that a declared environment cannot host.
    #[serde(default)]
    policy_gaps: Vec<PolicyGap>,
}

/// A declared, owned, time-bounded inability to execute a tier on an
/// environment. Its whole purpose is to make the cell render `POLICY GAP`
/// rather than a green `0 run / N skipped`.
#[derive(Clone, Debug, Deserialize)]
struct PolicyGap {
    tier: Tier,
    #[serde(default)]
    environments: Vec<String>,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    owner: String,
    /// `""` when `areas.json` omits it. Absent is *not* "never expires": the
    /// verdict blocks an undated gap, matching `affected_scope.py`, where
    /// `expiry` is in `POLICY_GAP_FIELDS` and therefore mandatory.
    #[serde(default)]
    expiry: String,
}

fn default_true() -> bool {
    true
}

/// Mirrors `scripts/ci/affected_scope.py`'s `AREA_DEFAULTS`.
fn default_environments() -> Vec<String> {
    vec![
        "ubuntu-latest".to_owned(),
        "windows-latest".to_owned(),
        "macos-latest".to_owned(),
    ]
}

fn default_shards() -> Vec<String> {
    vec!["1/1".to_owned()]
}

/// A cell policy says should exist, and the shards it should be built from.
#[derive(Clone, Debug)]
struct ExpectedCell {
    key: CellKey,
    shards: Vec<String>,
    /// L2 backends this area's tests require, empty when the tier is
    /// backend-agnostic.
    backends: Vec<String>,
    /// Set when `areas.json` declares this cell cannot execute here.
    declared_gap: Option<DeclaredGap>,
}

/// Cross `areas.json` with the run's affected scope to derive every cell that
/// should have produced evidence.
///
/// ## Notes
///
/// An L2 tier is expected on **every** environment the area declares, not only
/// the ones with a provisioned backend. Omitting the unprovisioned ones would
/// make them vanish from the grid, which is the failure mode `POLICY GAP`
/// exists to prevent.
fn expected_cells(
    areas: &[AreaPolicy],
    scope: &BTreeSet<String>,
    browser_environments: &[String],
) -> Vec<ExpectedCell> {
    let mut expected = Vec::new();

    for area in areas {
        if !area.ci || !scope.contains(&area.area) {
            continue;
        }

        let gap_for = |tier: &Tier, environment: &str| -> Option<DeclaredGap> {
            area.policy_gaps
                .iter()
                .find(|gap| &gap.tier == tier && gap.environments.iter().any(|e| e == environment))
                .map(|gap| DeclaredGap {
                    owner: gap.owner.clone(),
                    reason: gap.reason.clone(),
                    expiry: gap.expiry.clone(),
                })
        };

        for environment in &area.environments {
            expected.push(ExpectedCell {
                key: CellKey {
                    area: area.area.clone(),
                    environment: environment.clone(),
                    tier: Tier::L1,
                },
                shards: area.shards.clone(),
                backends: Vec::new(),
                declared_gap: gap_for(&Tier::L1, environment),
            });

            if area.l2 {
                expected.push(ExpectedCell {
                    key: CellKey {
                        area: area.area.clone(),
                        environment: environment.clone(),
                        tier: Tier::L2,
                    },
                    shards: default_shards(),
                    backends: area.backends.clone(),
                    declared_gap: gap_for(&Tier::L2, environment),
                });
            }

            if area.browser && browser_environments.iter().any(|e| e == environment) {
                expected.push(ExpectedCell {
                    key: CellKey {
                        area: area.area.clone(),
                        environment: environment.clone(),
                        tier: Tier::Browser,
                    },
                    shards: default_shards(),
                    backends: Vec::new(),
                    declared_gap: gap_for(&Tier::Browser, environment),
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

/// Identity recovered from an artifact directory name, used only when no
/// manifest record covers a staged report.
///
/// Artifact names are `junit-<area>-<tier>-<os>-<index>`. Areas and OS labels
/// both contain hyphens, so the parse anchors on the *tier* token — the only
/// field drawn from a closed vocabulary — and takes everything before it as the
/// area and everything after as `<os>[-<index>]`.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DegradedIdentity {
    area: String,
    tier: Tier,
    environment: String,
}

fn parse_artifact_name(name: &str) -> Option<DegradedIdentity> {
    let rest = name.strip_prefix("junit-")?;
    let parts: Vec<&str> = rest.split('-').collect();

    let tier_at = parts.iter().position(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "l1" | "l2" | "l3" | "browser" | "real" | "sanity"
        )
    })?;
    if tier_at == 0 || tier_at + 1 >= parts.len() {
        return None;
    }

    let area = parts[..tier_at].join("-");
    let tier = Tier::parse(parts[tier_at]);
    // A trailing all-digit token is `strategy.job-index`, not part of the OS
    // label. `ubuntu-latest` and `windows-latest` never end in digits.
    let mut tail = &parts[tier_at + 1..];
    if tail.len() > 1 && tail[tail.len() - 1].chars().all(|ch| ch.is_ascii_digit()) {
        tail = &tail[..tail.len() - 1];
    }
    let environment = tail.join("-");
    if environment.is_empty() {
        return None;
    }

    Some(DegradedIdentity {
        area,
        tier,
        environment,
    })
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

/// Every staged XML under an artifact directory, keyed by its `<tier>/<pkg>.xml`
/// relative path so a manifest record can be matched to it.
fn staged_reports(root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut found = BTreeMap::new();
    let mut stack = vec![(root.to_path_buf(), Vec::<String>::new())];

    while let Some((dir, prefix)) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries {
            let entry = entry.context("failed to read artifact directory entry")?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let mut next = prefix.clone();
            next.push(name.clone());
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                stack.push((entry.path(), next));
            } else if name.to_ascii_lowercase().ends_with(".xml") {
                found.insert(next.join("/"), entry.path());
            }
        }
    }

    Ok(found)
}

/// Turn one artifact directory into run records.
///
/// The manifest is the identity source. A staged report with no manifest record
/// falls back to the artifact directory name and is marked `degraded`.
fn records_from_artifact(dir: &ArtifactDir) -> Result<Vec<RunRecord>> {
    let manifest_path = dir.path.join("manifest.jsonl");
    let manifest = if manifest_path.is_file() {
        read_manifest(&manifest_path)?
    } else {
        Vec::new()
    };
    let reports = staged_reports(&dir.path)?;
    let fallback = parse_artifact_name(&dir.name);

    let mut records = Vec::new();
    let mut claimed: BTreeSet<String> = BTreeSet::new();

    for entry in &manifest {
        let relative = entry.xml.replace('\\', "/");
        claimed.insert(relative.clone());
        let report_path = join_relative(&dir.path, &relative);

        let outcome = if entry.report_present {
            ReportOutcome::read(&report_path)
        } else {
            ReportOutcome::default()
        };

        // The producer writes "" rather than inventing a value; recover from the
        // artifact name only when it is genuinely absent.
        let area = non_empty(&entry.area)
            .or_else(|| fallback.as_ref().map(|id| id.area.clone()))
            .unwrap_or_default();
        let environment = non_empty(&entry.environment)
            .or_else(|| fallback.as_ref().map(|id| id.environment.clone()))
            .unwrap_or_default();
        let shard = non_empty(&entry.shard).unwrap_or_else(|| "1/1".to_owned());
        let identity_degraded = entry.area.trim().is_empty() || entry.environment.trim().is_empty();

        records.push(RunRecord {
            area,
            environment,
            tier: Tier::parse(&entry.tier),
            shard,
            package: entry.package.clone(),
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

    for (relative, path) in &reports {
        if claimed.contains(relative) {
            continue;
        }
        let Some(identity) = fallback.clone() else {
            continue;
        };
        let package = Path::new(relative)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative.clone());

        let outcome = ReportOutcome::read(path);

        records.push(RunRecord {
            area: identity.area,
            environment: identity.environment,
            tier: identity.tier,
            shard: UNKNOWN_SHARD.to_owned(),
            package,
            artifact: dir.name.clone(),
            degraded: true,
            report_present: outcome.parse_error.is_none(),
            exit_code: 0,
            duration_s: 0,
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
    /// environment → provisioned L2 backends. An environment absent from this
    /// map has unknown provisioning, and no policy gap is asserted for it.
    provisioned: &'a BTreeMap<String, BTreeSet<String>>,
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
             areas.json or the affected scope is wrong"
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
    let mut shards_seen = BTreeSet::new();
    let mut reasons = Vec::new();
    let mut has_unusable_record = false;

    for &index in indices {
        let record = &inputs.records[index];
        counts.add(&record.counts);
        failed_tests.extend(record.failed_tests.iter().cloned());
        observed_skips.extend(record.skipped_tests.iter().cloned());
        observed_tests.extend(record.failed_tests.iter().cloned());
        observed_tests.extend(record.skipped_tests.iter().cloned());
        shards_seen.insert(record.shard.clone());

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
                "{}: no manifest record — identity recovered from the artifact name \
                 and shard is unknown",
                record.package
            ));
        }
    }

    let scheduled = expectation.is_some();
    let expected_shards: Vec<String> = expectation
        .map(|cell| cell.shards.clone())
        .unwrap_or_default();
    let missing_shards: Vec<String> = expected_shards
        .iter()
        .filter(|shard| !shards_seen.contains(*shard))
        .cloned()
        .collect();

    let expected_for_cell = inputs
        .expected_tests
        .get(&(key.environment.clone(), key.tier.clone()));
    let mut skip_evidence_degraded = expected_for_cell.is_none();
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

    let declared_gap = expectation.and_then(|cell| cell.declared_gap.clone());

    if let Some(gap) = &declared_gap {
        policy_gap = true;
        reasons.push(format!("declared policy gap: {}", gap.describe()));
    } else if scheduled && !backends.is_empty() {
        // An *undeclared* gap. `affected_scope.py` is supposed to reject this
        // at config time; catching it here too means a config-side regression
        // surfaces as a blocking cell rather than a green one.
        match inputs.provisioned.get(&key.environment) {
            Some(available) => {
                if !backends.iter().any(|backend| available.contains(backend)) {
                    policy_gap = true;
                    reasons.push(format!(
                        "UNDECLARED policy gap: tier requires backend(s) [{}] but {} \
                         provisions [{}], and areas.json declares no gap for it",
                        backends.join(", "),
                        key.environment,
                        available.iter().cloned().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            None => {
                skip_evidence_degraded = true;
                reasons.push(format!(
                    "backend provisioning for {} is unknown; no policy-gap judgement made",
                    key.environment
                ));
            }
        }
    }

    // Matched on area alone. A status's `environment` says where that job ran,
    // not what it gated: `lint` is Linux-only but `needs: lint` deletes the
    // test matrix for *every* environment, which is how five areas lost all
    // their L1 legs in run 30323254931.
    let upstream = inputs
        .statuses
        .iter()
        .find(|status| {
            status.area == key.area
                && Tier::parse(&status.job) != key.tier
                && matches!(status.result.as_str(), "failure" | "cancelled")
        })
        .cloned();

    let state = if !scheduled {
        // Reached only for evidence policy did not schedule; the caller pushes
        // the explanatory reason.
        if counts.bad() > 0 {
            CellState::Fail
        } else {
            CellState::NotScheduled
        }
    } else if counts.bad() == 0 && declared_gap.is_some() {
        // A *declared* gap explains the absence of evidence, so MISSING would
        // be the wrong answer: nobody failed to upload anything. It still
        // blocks — the point of the state is that the cell cannot go green.
        // Actual failures still outrank it, handled below.
        CellState::PolicyGap
    } else if indices.is_empty() || has_unusable_record || !missing_shards.is_empty() {
        if let Some(status) = &upstream {
            reasons.push(format!(
                "upstream job `{}` concluded `{}`, so this leg never ran and \
                 GitHub never evaluated its matrix context",
                status.job, status.result
            ));
        }
        if indices.is_empty() {
            reasons.push("scheduled but produced no report at all".to_owned());
        } else if !missing_shards.is_empty() {
            reasons.push(format!(
                "no report for shard(s) {}",
                missing_shards.join(", ")
            ));
        }
        CellState::Missing
    } else if counts.bad() > 0 {
        CellState::Fail
    } else if policy_gap {
        CellState::PolicyGap
    } else if counts.total == 0 {
        CellState::NotApplicable
    } else if counts.passed == 0 {
        // Evidence exists but nothing executed. Never PASS.
        CellState::Skip
    } else {
        CellState::Pass
    };

    Cell {
        key: key.clone(),
        state,
        counts: Counts {
            skipped: counts.skipped + absent_skips.len() as u32,
            ..counts
        },
        scheduled,
        missing_shards,
        skipped_tests: all_skips.into_iter().collect(),
        failed_tests: failed_tests.into_iter().collect(),
        skip_evidence_degraded,
        declared_gap,
        reasons,
        records: indices.to_vec(),
    }
}

/// Environment assumed for a producer status that does not name one. The lint
/// job is Linux-only today (`_area-ci.yml`).
const DEFAULT_STATUS_ENVIRONMENT: &str = "ubuntu-latest";

/// Shard recorded for a staged report that no manifest record covers.
///
/// The staged path is `<tier>/<package>.xml` and carries no shard component, so
/// shard identity exists *only* in the manifest. Inventing `1/1` here would let
/// one shard of a four-shard area masquerade as the whole leg.
const UNKNOWN_SHARD: &str = "unknown";

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
        if !scope.contains(&status.area) {
            continue;
        }
        let tier = Tier::parse(&status.job);
        if matches!(tier, Tier::L1 | Tier::L2 | Tier::L3 | Tier::Browser | Tier::Real) {
            continue;
        }
        let key = CellKey {
            area: status.area.clone(),
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
            missing_shards: Vec::new(),
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
    area: String,
    environment: String,
    tier: Tier,
    #[serde(default = "default_shard")]
    shard: String,
    owner: String,
    reason: String,
    source_run: String,
    #[serde(default)]
    expiry: Option<String>,
}

fn default_shard() -> String {
    "1/1".to_owned()
}

/// An approved skip budget. `tests` carries exact identities so one removed
/// skip cannot mask one newly-added skip.
#[derive(Clone, Debug, Deserialize)]
struct SkipEntry {
    area: String,
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
    for entry in &baseline.failure {
        validate_date(entry.expiry.as_deref(), "failure", &entry.area)?;
    }
    for entry in &baseline.skip {
        validate_date(entry.expiry.as_deref(), "skip", &entry.area)?;
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

/// Apply every §1.2 and §1.3 rule to a rollup.
///
/// ## Notes
///
/// `scope` is the set of areas this run actually scheduled. An entry naming an
/// area outside it is *ignored* — neither accepted as a pass nor counted as a
/// block — because the run produced no information about it either way.
///
/// Two independent excusal paths feed the blocking-cell loop, and they never
/// overlap: `.github/ci/ci-baseline.toml` excuses a `FAIL`, and an owned,
/// unexpired `areas.json` policy gap excuses a `POLICY GAP` (see
/// [`policy_gap_findings`]). Both leave the cell's state untouched and both
/// report a `note`, so an excused cell is never invisible.
fn verdict(rollup: &Rollup, baseline: &Baseline, today: Option<&str>) -> Vec<Finding> {
    let scope: BTreeSet<&String> = rollup.scope.iter().collect();
    let mut findings = Vec::new();
    let mut excused_cells: BTreeSet<CellKey> = BTreeSet::new();
    let mut out_of_scope: BTreeSet<String> = BTreeSet::new();

    for entry in &baseline.failure {
        let key = CellKey {
            area: entry.area.clone(),
            environment: entry.environment.clone(),
            tier: entry.tier.clone(),
        };
        let subject = format!("{key} shard {}", entry.shard);

        if !scope.contains(&entry.area) {
            out_of_scope.insert(entry.area.clone());
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
            format!("{} area(s)", out_of_scope.len()),
            format!(
                "outside this run's affected scope; ignored, not treated as a pass: {}",
                out_of_scope.iter().cloned().collect::<Vec<_>>().join(", ")
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
/// - **undeclared** — no `areas.json` record at all, so the gap was inferred
///   from backend provisioning. This is the case that catches a tier being
///   quietly switched off; it falls through to the generic blocking cell loop.
/// - **incomplete** — no owner, reason, or expiry. An unattributable or undated
///   gap is a permanent exclusion wearing a temporary label.
/// - **expired** — §1.3's expiry rule, applied to gaps.
///
/// A gap whose cell produced real *failures* never reaches this function:
/// `classify_one` ranks `Fail` above `PolicyGap`, so the cell is `FAIL` and is
/// judged by the baseline. A gap declaration can never suppress evidence.
///
/// There is deliberately **no** `policy-gap-now-executing` rule, tempting as the
/// analogy to `baseline-now-passing` is. A `require_level!` gate that skips
/// because its backend is absent early-returns, and nextest records that as a
/// JUnit **pass** — so on this evidence a passing count is exactly what a
/// correctly-declared gap looks like, and such a rule would block the case it
/// was meant to protect. Detecting a gap that has genuinely closed needs plan
/// §1.1's per-backend execution proof. Until then `expiry` is the only forcing
/// function, which is why an undated gap is rejected outright.
///
/// ## Errors
///
/// The expiry check duplicates `affected_scope.py::validate_expiry` on purpose.
/// That one fails the scope job at config time with the most actionable
/// message, and it is the better place to *learn* about a lapsed gap. But it is
/// not sufficient: `verdict` is the single required branch-protection check and
/// runs `if: always()`, precisely so a failed, skipped, or never-scheduled scope
/// job cannot suppress the verdict. If expiry lived only in the Python, an
/// expired gap would be excused by the one check that actually gates merging
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
            area: entry.area.clone(),
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
        .filter(|entry| !scope.contains(&entry.area))
        .map(|entry| &entry.area)
        .collect();
    if !out_of_scope.is_empty() {
        findings.push(Finding::note(
            "skip-out-of-scope",
            format!("{} area(s)", out_of_scope.len()),
            format!(
                "outside this run's affected scope; ignored: {}",
                out_of_scope
                    .iter()
                    .map(|area| area.as_str())
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

/// Render the area × environment × tier grid.
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
        let areas: BTreeSet<&String> = cells.iter().map(|cell| &cell.key.area).collect();

        out.push_str(&format!("### {tier}\n\n| area |"));
        for environment in &environments {
            out.push_str(&format!(" {} |", cell_text(environment)));
        }
        out.push_str("\n| --- |");
        for _ in &environments {
            out.push_str(" --- |");
        }
        out.push('\n');

        for area in &areas {
            out.push_str(&format!("| `{}` |", cell_text(area)));
            for environment in &environments {
                let found = cells.iter().find(|cell| {
                    cell.key.area == **area && cell.key.environment == **environment
                });
                let text = match found {
                    Some(cell) => match cell.state {
                        CellState::NotScheduled | CellState::NotApplicable => {
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
// CLI
// ---------------------------------------------------------------------------

const USAGE: &str = "\
ci-rollup — package-area × environment × tier CI rollup and merge verdict

USAGE:
  ci-rollup rollup  --artifacts <dir> [options]
  ci-rollup verdict --results <file> --baseline <file> [options]

ROLLUP OPTIONS:
  --artifacts <dir>              root holding the downloaded per-job artifacts
  --areas <file>                 areas.json policy (default .github/ci/areas.json)
  --scope <a,b,c>                areas this run scheduled; repeatable. Omitting it
                                 infers scope from artifacts and marks the run degraded
  --browser-environments <a,b>   environments that run the browser tier (default
                                 ubuntu-latest). L1 and L2 environments come from
                                 each area's `environments` in areas.json
  --provisioned-backends <env>=<b1,b2>
                                 L2 backends provisioned on an environment; repeatable.
                                 Used only to catch an UNDECLARED policy gap — a
                                 declared `policy_gaps` record in areas.json is
                                 authoritative. An environment with no entry is
                                 treated as unknown, not as unprovisioned
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

EXIT CODES:
  0  clear
  1  tool error (bad usage, unreadable input)
  2  blocked
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
        "--help" | "-h" | "help" | "" => {
            println!("{USAGE}");
            Ok(0)
        }
        other => bail!("unknown subcommand `{other}`\n\n{USAGE}"),
    }
}

fn cmd_rollup(args: &Args) -> Result<i32> {
    let artifacts = PathBuf::from(args.required("artifacts")?);
    let areas_path = args
        .one("areas")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".github").join("ci").join("areas.json"));

    let areas_text = fs::read_to_string(&areas_path)
        .with_context(|| format!("failed to read {}", areas_path.display()))?;
    let areas: Vec<AreaPolicy> = serde_json::from_str(&areas_text)
        .with_context(|| format!("invalid areas policy {}", areas_path.display()))?;

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
        records.iter().map(|record| record.area.clone()).collect()
    } else {
        explicit_scope.into_iter().collect()
    };

    let browser_environments = tier_environments(args, "browser-environments");
    let provisioned = parse_provisioned(args)?;
    let expected_tests = load_expected_manifests(args)?;

    let expected = expected_cells(&areas, &scope, &browser_environments);
    let mut cells = classify(&ClassifyInputs {
        expected: &expected,
        records: &records,
        statuses: &statuses,
        provisioned: &provisioned,
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
    if rollup.schema_version > SCHEMA_VERSION {
        bail!(
            "result schema_version {} is newer than this tool understands ({SCHEMA_VERSION})",
            rollup.schema_version
        );
    }

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

fn tier_environments(args: &Args, name: &str) -> Vec<String> {
    let values = args.list(name);
    if values.is_empty() {
        vec![DEFAULT_TIER_ENVIRONMENTS.to_owned()]
    } else {
        values
    }
}

fn parse_provisioned(args: &Args) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut provisioned: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for raw in args.many("provisioned-backends") {
        let (environment, backends) = raw.split_once('=').ok_or_else(|| {
            anyhow!("`--provisioned-backends` expects `<environment>=<backend>[,<backend>]`, got `{raw}`")
        })?;
        let environment = environment.trim();
        if environment.is_empty() {
            bail!("`--provisioned-backends` has an empty environment in `{raw}`");
        }
        let entry = provisioned.entry(environment.to_owned()).or_default();
        for backend in backends.split(',').map(str::trim) {
            if !backend.is_empty() {
                entry.insert(backend.to_owned());
            }
        }
    }
    Ok(provisioned)
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
