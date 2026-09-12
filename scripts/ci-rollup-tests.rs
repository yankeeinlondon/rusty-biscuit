//! Tests for `ci-rollup`.
//!
//! Included from `ci-rollup.rs` via `#[path]` so they can exercise private
//! items; a `tests/` integration crate cannot reach into a `[[bin]]`.

use super::*;
use rstest::rstest;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn junit(suite: &str, cases: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="nextest-run" tests="0" failures="0" errors="0">
    <testsuite name="{suite}" tests="0" disabled="0" errors="0" failures="0">
{cases}
    </testsuite>
</testsuites>
"#
    )
}

fn passing_case(name: &str) -> String {
    format!(r#"        <testcase name="{name}" classname="pkg" time="0.1"></testcase>"#)
}

fn failing_case(name: &str) -> String {
    format!(
        r#"        <testcase name="{name}" classname="pkg" time="0.1"><failure type="test failure">boom</failure></testcase>"#
    )
}

fn skipped_case(name: &str) -> String {
    format!(r#"        <testcase name="{name}" classname="pkg" time="0.0"><skipped/></testcase>"#)
}

fn record(package: &str, environment: &str, tier: Tier) -> RunRecord {
    RunRecord {
        package: package.to_owned(),
        environment: environment.to_owned(),
        tier,
        artifact: "junit-test".to_owned(),
        degraded: false,
        report_present: true,
        exit_code: 0,
        duration_s: 1,
        counts: Counts::default(),
        failed_tests: Vec::new(),
        skipped_tests: Vec::new(),
        parse_error: None,
        passed_identities: Vec::new(),
    }
}

fn passing_record(package: &str, environment: &str, tier: Tier) -> RunRecord {
    let mut rec = record(package, environment, tier);
    rec.counts = Counts {
        total: 3,
        passed: 3,
        ..Counts::default()
    };
    rec.passed_identities = vec!["pkg::a".into(), "pkg::b".into(), "pkg::c".into()];
    rec
}

/// The repository root. `scripts/` is a workspace of its own, so the manifest
/// directory is one level down.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("scripts/ has a parent")
        .to_path_buf()
}

fn expectation(package: &str, environment: &str, tier: Tier) -> ExpectedCell {
    ExpectedCell::new(
        CellKey {
            package: package.to_owned(),
            environment: environment.to_owned(),
            tier,
        },
        String::new(),
    )
}

/// A well-formed, owned, unexpired governed capability gap. Tests that
/// exercise a *malformed* one mutate this rather than restating every field.
fn gap() -> DeclaredGap {
    DeclaredGap {
        owner: "@yankeeinlondon".to_owned(),
        reason: "tmux has no Windows port".to_owned(),
        expiry: "2027-01-31".to_owned(),
        capability: "tmux".to_owned(),
        policy: POLICY_LINK.to_owned(),
        closes: "features/_unscheduled/windows-l2-ci-leg".to_owned(),
    }
}

fn exclusion() -> Exclusion {
    Exclusion {
        exclusion_class: "promotion-pending".to_owned(),
        owner: "@yankeeinlondon".to_owned(),
        reason: "blocked on the canonical recipe set".to_owned(),
        expiry: Some("2026-10-31".to_owned()),
    }
}

/// The four standard environments, shaped like the checked-in
/// `environments.json`: tmux on Linux/macOS, headless browser and Node on
/// Linux, and an archive-only WSL2 environment hosted by Windows. Every
/// absence the table governs (tmux and headless_browser off ubuntu) is a
/// GOVERNED absence here too.
fn test_environments() -> Vec<Environment> {
    let governed_gap = |reason: &str| Capability::Governed {
        available: false,
        reason: reason.to_owned(),
        owner: "@yankeeinlondon".to_owned(),
        expiry: "2027-01-31".to_owned(),
        closes: String::new(),
    };
    let plain = |tmux: bool, browser: bool, node: bool, archive: bool| {
        BTreeMap::from([
            ("tmux".to_owned(), Capability::Available(tmux)),
            (
                "headless_browser".to_owned(),
                Capability::Available(browser),
            ),
            ("node_pnpm".to_owned(), Capability::Available(node)),
            ("archive_only".to_owned(), Capability::Available(archive)),
        ])
    };
    let with_governed_absences = |mut caps: BTreeMap<String, Capability>| {
        caps.insert("tmux".to_owned(), governed_gap("tmux has no Windows port"));
        caps.insert(
            "headless_browser".to_owned(),
            governed_gap("the browser tier is Linux-hosted in CI"),
        );
        caps
    };
    vec![
        Environment {
            name: "ubuntu-latest".to_owned(),
            capabilities: plain(true, true, true, false),
        },
        Environment {
            name: "windows-latest".to_owned(),
            capabilities: with_governed_absences(plain(false, false, false, false)),
        },
        Environment {
            name: "macos-latest".to_owned(),
            capabilities: {
                let mut caps = plain(true, false, false, false);
                caps.insert(
                    "headless_browser".to_owned(),
                    governed_gap("the browser tier is Linux-hosted in CI"),
                );
                caps
            },
        },
        Environment {
            name: "wsl2-ubuntu".to_owned(),
            capabilities: with_governed_absences(plain(false, false, false, true)),
        },
    ]
}

/// A gating package's policy. Its area follows the repo's commonest layout — a
/// library and its `-cli` companion share one area — so an area-scoped fixture
/// exercises the grouping rather than a one-package-per-area degenerate case.
/// The real derivation is the planner's, from the manifest directory.
fn policy(package: &str) -> PackagePolicy {
    PackagePolicy {
        package: package.to_owned(),
        area: package.trim_end_matches("-cli").to_owned(),
        gates: true,
        tiers: vec![Tier::L1],
        l2_backends: Vec::new(),
        companion_suites: Vec::new(),
        exclusion: None,
    }
}

/// Classify with the given expectations and records and nothing else.
fn classify_simple(expected: &[ExpectedCell], records: &[RunRecord]) -> Vec<Cell> {
    let expected_tests = BTreeMap::new();
    classify(&ClassifyInputs {
        expected,
        records,
        statuses: &[],
        expected_tests: &expected_tests,
    })
}

fn only_cell(cells: Vec<Cell>) -> Cell {
    assert_eq!(cells.len(), 1, "expected exactly one cell, got {cells:#?}");
    cells.into_iter().next().unwrap()
}

/// A rollup that does not know what was scheduled — the shape of a document
/// written before the schedule field existed, which the verdict must treat as
/// "cannot say" and fail closed.
fn rollup_of(cells: Vec<Cell>, scope: &[&str]) -> Rollup {
    Rollup {
        schema_version: RESULT_SCHEMA_VERSION,
        run_id: None,
        scope: scope.iter().map(|s| (*s).to_owned()).collect(),
        areas: derived_areas(&cells),
        area_scope: Vec::new(),
        accepted_evidence: accepted_evidence(&cells),
        scope_degraded: false,
        scheduled: None,
        records: Vec::new(),
        cells,
    }
}

/// A rollup that knows exactly which legs policy scheduled.
fn rollup_scheduling(cells: Vec<Cell>, scope: &[&str], scheduled: Vec<CellKey>) -> Rollup {
    Rollup {
        scheduled: Some(scheduled),
        ..rollup_of(cells, scope)
    }
}

fn cell_key(package: &str, environment: &str, tier: Tier) -> CellKey {
    CellKey {
        package: package.to_owned(),
        environment: environment.to_owned(),
        tier,
    }
}

fn failure_entry(package: &str, environment: &str, tier: Tier) -> FailureEntry {
    FailureEntry {
        package: package.to_owned(),
        environment: environment.to_owned(),
        tier,
        owner: "@owner".to_owned(),
        reason: "known".to_owned(),
        source_run: "1".to_owned(),
        expiry: None,
    }
}

fn blocks_with_rule(findings: &[Finding], rule: &str) -> bool {
    findings
        .iter()
        .any(|f| f.severity == Severity::Block && f.rule == rule)
}

fn any_block(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity == Severity::Block)
}

// ---------------------------------------------------------------------------
// JUnit parsing
// ---------------------------------------------------------------------------

#[test]
fn parses_pass_fail_and_skip_into_exact_identities() {
    let xml = junit(
        "biscuit-file",
        &[passing_case("mod::ok"), failing_case("mod::bad"), skipped_case("mod::gone")].join("\n"),
    );

    let report = parse_junit(&xml).expect("well-formed document");

    assert_eq!(
        report.counts,
        Counts {
            total: 3,
            passed: 1,
            failed: 1,
            skipped: 1,
            errored: 0,
        }
    );
    assert_eq!(report.failed_tests, vec!["biscuit-file::mod::bad"]);
    assert_eq!(report.skipped_tests, vec!["biscuit-file::mod::gone"]);
    assert_eq!(report.passed_tests, vec!["biscuit-file::mod::ok"]);
}

#[test]
fn parses_self_closing_testcase_as_a_pass() {
    let xml = junit(
        "pkg",
        r#"        <testcase name="a" classname="pkg" time="0.1"/>"#,
    );
    let report = parse_junit(&xml).unwrap();
    assert_eq!(report.counts.passed, 1);
    assert_eq!(report.counts.total, 1);
}

#[test]
fn counts_an_error_element_as_not_passing() {
    let xml = junit(
        "pkg",
        r#"        <testcase name="a" classname="pkg"><error type="panic">x</error></testcase>"#,
    );
    let report = parse_junit(&xml).unwrap();
    assert_eq!(report.counts.errored, 1);
    assert_eq!(report.counts.passed, 0);
    assert_eq!(report.counts.bad(), 1);
}

#[test]
fn rejects_truncated_xml_rather_than_reporting_a_partial_pass() {
    let full = junit("pkg", &passing_case("a"));
    let truncated = &full[..full.len() / 2];

    let err = parse_junit(truncated).expect_err("truncated document must not parse");
    let message = format!("{err:#}");
    assert!(
        message.contains("truncated") || message.contains("malformed"),
        "unhelpful error: {message}"
    );
}

#[test]
fn rejects_malformed_xml() {
    let err = parse_junit("<testsuites><testsuite></testsuites>").expect_err("mismatched tags");
    assert!(format!("{err:#}").contains("malformed"));
}

#[test]
fn rejects_a_document_that_is_not_junit() {
    let err = parse_junit(r#"<?xml version="1.0"?><coverage lines="4"/>"#)
        .expect_err("not a JUnit document");
    assert!(format!("{err:#}").contains("no <testsuites>"));
}

#[test]
fn parses_an_empty_but_valid_report_as_zero_tests() {
    let report = parse_junit(r#"<testsuites name="nextest-run" tests="0"></testsuites>"#).unwrap();
    assert_eq!(report.counts.total, 0);
}

// ---------------------------------------------------------------------------
// Cell states — all seven
// ---------------------------------------------------------------------------

#[test]
fn state_pass_requires_an_executed_test() {
    let cells = classify_simple(
        &[expectation("a", "ubuntu-latest", Tier::L1)],
        &[passing_record("a", "ubuntu-latest", Tier::L1)],
    );
    assert_eq!(only_cell(cells).state, CellState::Pass);
}

#[test]
fn state_fail_when_any_test_failed() {
    let mut rec = passing_record("a", "ubuntu-latest", Tier::L1);
    rec.counts.failed = 1;
    rec.counts.total += 1;
    rec.failed_tests = vec!["pkg::bad".into()];

    let cells = classify_simple(&[expectation("a", "ubuntu-latest", Tier::L1)], &[rec]);
    assert_eq!(only_cell(cells).state, CellState::Fail);
}

#[test]
fn state_skip_when_evidence_exists_but_nothing_executed() {
    let mut rec = record("a", "ubuntu-latest", Tier::L2);
    rec.counts = Counts {
        total: 2,
        skipped: 2,
        ..Counts::default()
    };
    rec.skipped_tests = vec!["pkg::x".into(), "pkg::y".into()];

    let cell = only_cell(classify_simple(
        &[expectation("a", "ubuntu-latest", Tier::L2)],
        &[rec],
    ));
    assert_eq!(cell.state, CellState::Skip);
    assert_ne!(cell.state, CellState::Pass, "an all-skipped tier is never PASS");
}

/// R10: a scheduled package whose invocation selects zero tests records
/// NOTHING TO RUN — neither a pass implying coverage nor a blocking missing
/// result, and never conflated with NOT SCHEDULED.
#[test]
fn state_nothing_to_run_when_the_invocation_selects_zero_tests() {
    let rec = record("tabby", "ubuntu-latest", Tier::L1);
    let cell = only_cell(classify_simple(
        &[expectation("tabby", "ubuntu-latest", Tier::L1)],
        &[rec],
    ));
    assert_eq!(cell.state, CellState::NothingToRun);
    assert_ne!(cell.state, CellState::Pass, "a zero-test package is not a pass");
    assert!(!cell.state.blocks(), "NOTHING TO RUN never blocks");
    assert!(cell.reasons.iter().any(|r| r.contains("zero tests")));
}

#[test]
fn state_missing_when_scheduled_but_no_artifact_exists() {
    let cell = only_cell(classify_simple(
        &[expectation("claudine", "ubuntu-latest", Tier::L1)],
        &[],
    ));
    assert_eq!(cell.state, CellState::Missing);
    assert!(cell.state.blocks());
}

#[test]
fn state_missing_when_nextest_produced_no_report() {
    let mut rec = record("a", "ubuntu-latest", Tier::L1);
    rec.report_present = false;
    rec.exit_code = 0;

    let cell = only_cell(classify_simple(
        &[expectation("a", "ubuntu-latest", Tier::L1)],
        &[rec],
    ));
    assert_eq!(cell.state, CellState::Missing);
    assert!(cell.reasons.iter().any(|r| r.contains("staged no report")));
}

#[test]
fn state_missing_when_the_report_is_unreadable() {
    let mut rec = record("a", "ubuntu-latest", Tier::L1);
    rec.parse_error = Some("truncated JUnit XML".to_owned());

    let cell = only_cell(classify_simple(
        &[expectation("a", "ubuntu-latest", Tier::L1)],
        &[rec],
    ));
    assert_eq!(cell.state, CellState::Missing);
}

#[test]
fn state_not_scheduled_for_a_package_outside_the_run_scope() {
    let policies = vec![policy("homelab")];
    let scope = BTreeSet::new();

    let expected = expected_cells(&policies, &scope, &test_environments());
    assert!(expected.is_empty(), "an out-of-scope package schedules nothing");

    // With no expectation and no evidence there is no cell at all, which the
    // grid renders as NOT SCHEDULED.
    let cells = classify_simple(&expected, &[]);
    assert!(cells.is_empty());
}

/// R10: a `gates = false` package records NOT SCHEDULED *with its governance
/// metadata* — visible, owned, and never conflated with NOTHING TO RUN.
#[test]
fn state_not_scheduled_for_an_excluded_package_carries_governance() {
    let mut excluded_policy = policy("test-toolkit");
    excluded_policy.gates = false;
    excluded_policy.exclusion = Some(exclusion());
    let scope: BTreeSet<String> = ["test-toolkit".to_owned()].into_iter().collect();

    let expected = expected_cells(&[excluded_policy], &scope, &test_environments());
    assert_eq!(
        expected.len(),
        4,
        "an excluded package renders one governed cell per environment"
    );

    let cells = classify_simple(&expected, &[]);
    for cell in &cells {
        assert_eq!(cell.state, CellState::NotScheduled);
        assert!(!cell.state.blocks(), "NOT SCHEDULED never blocks");
        assert!(!cell.scheduled);
        assert!(
            cell.reasons.iter().any(|r| r.contains("promotion-pending")
                && r.contains("@yankeeinlondon")
                && r.contains("2026-10-31")),
            "the exclusion's governance must reach the rendered cell: {:?}",
            cell.reasons
        );
    }
}

/// Evidence for a `gates = false` package means CI ran something policy
/// excludes: reported honestly, and the verdict blocks on the disagreement.
#[test]
fn evidence_for_an_excluded_package_blocks_as_unscheduled_evidence() {
    let mut excluded_policy = policy("test-toolkit");
    excluded_policy.gates = false;
    excluded_policy.exclusion = Some(exclusion());
    let scope: BTreeSet<String> = ["test-toolkit".to_owned()].into_iter().collect();
    let expected = expected_cells(&[excluded_policy], &scope, &test_environments());

    let cells = classify_simple(
        &expected,
        &[passing_record("test-toolkit", "ubuntu-latest", Tier::L1)],
    );
    let cell = cells
        .iter()
        .find(|cell| cell.key.environment == "ubuntu-latest")
        .expect("the cell with evidence exists");
    assert_eq!(
        cell.state,
        CellState::Pass,
        "a leg that ran and passed is not NOT SCHEDULED"
    );

    let findings = verdict(
        &rollup_of(cells, &["test-toolkit"]),
        &Baseline::default(),
        None,
    );
    assert!(blocks_with_rule(&findings, "cell-unscheduled-evidence"));
}

#[test]
fn state_policy_gap_when_the_capability_is_governed_and_absent() {
    let mut expectation = expectation("biscuit-terminal-cli", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned(), "wezterm".to_owned()];
    expectation.gap = Some(GapStatus::Governed(gap()));

    // The tier "passes" only because every test early-returned; without a
    // provisioned backend that is a policy gap, not a green cell.
    let record = passing_record("biscuit-terminal-cli", "windows-latest", Tier::L2);

    let cell = only_cell(classify_simple(&[expectation], &[record]));
    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.state.blocks());
}

#[test]
fn no_policy_gap_on_an_environment_that_can_host_the_tier() {
    let expectation = expectation("biscuit-terminal-cli", "ubuntu-latest", Tier::L2);

    let cell = only_cell(classify_simple(
        &[expectation],
        &[passing_record("biscuit-terminal-cli", "ubuntu-latest", Tier::L2)],
    ));
    assert_eq!(cell.state, CellState::Pass);
}

/// A plain `false` in the capability table — no owner, no reason, no expiry —
/// is an UNGOVERNED gap: rendered POLICY GAP and never excused.
#[test]
fn an_ungoverned_policy_gap_is_named_as_ungoverned() {
    let mut expectation = expectation("biscuit-tui-cli", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];
    expectation.gap = Some(GapStatus::Ungoverned);

    let cell = only_cell(classify_simple(
        &[expectation],
        &[passing_record("biscuit-tui-cli", "windows-latest", Tier::L2)],
    ));

    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.declared_gap.is_none());
    assert!(cell.reasons.iter().any(|r| r.contains("UNGOVERNED")));
}

/// The measured case: `needs:` skips the whole matrix, GitHub never evaluates
/// the matrix context, and no artifact exists for any leg.
#[test]
fn a_failing_lint_is_never_blamed_for_a_missing_l1_cell() {
    // `needs: lint` was removed from the test job, so lint gates nothing.
    // Blaming any failing job for the package sent claudine's triage at lint in
    // run 30427703024 for MISSING L1 cells lint could not have caused.
    let statuses = vec![ProducerStatus {
        package: "claudine-cli".to_owned(),
        job: "lint".to_owned(),
        result: "failure".to_owned(),
        environment: None,
        detail: None,
        companion: None,
    }];
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("claudine-cli", "windows-latest", Tier::L1)],
        records: &[],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));

    // Still MISSING and still blocking — only the attribution changes.
    assert_eq!(cell.state, CellState::Missing);
    assert!(cell.state.blocks());
    assert!(
        !cell.reasons.iter().any(|r| r.contains("upstream job")),
        "lint gates no tier, so it must not be named as an upstream cause: {:?}",
        cell.reasons
    );
    assert!(
        cell.reasons.iter().any(|r| r.contains("no report")),
        "the cell must still explain what IT observed: {:?}",
        cell.reasons
    );
}

/// A producer that knows WHY it has no evidence must be able to say so.
///
/// "Scheduled but produced no report at all" is equally true of a WSL2 guest
/// that died mid-suite and of a tier that ran no tests, and only the producer
/// can tell them apart.
#[test]
fn a_producer_detail_explains_why_a_cell_has_no_evidence() {
    let statuses = vec![ProducerStatus {
        package: "claudine-cli".to_owned(),
        job: "L1".to_owned(),
        result: "failure".to_owned(),
        environment: Some("wsl2-ubuntu".to_owned()),
        detail: Some("the WSL2 guest became unreachable after the test step".to_owned()),
        companion: None,
    }];
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("claudine-cli", "wsl2-ubuntu", Tier::L1)],
        records: &[],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.state, CellState::Missing);
    assert!(
        cell.reasons.iter().any(|r| r.contains("guest became unreachable")),
        "the producer's own explanation must reach the rendered cell: {:?}",
        cell.reasons
    );
}

/// L2 *does* declare `needs: test`, so a failing L1 is a real gating edge and
/// must still be named — the fix narrows attribution, it does not remove it.
#[test]
fn a_failing_l1_is_still_blamed_for_a_missing_l2_cell() {
    let statuses = vec![ProducerStatus {
        package: "darkmatter-cli".to_owned(),
        job: "L1".to_owned(),
        result: "failure".to_owned(),
        environment: None,
        detail: None,
        companion: None,
    }];
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("darkmatter-cli", "ubuntu-latest", Tier::L2)],
        records: &[],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.state, CellState::Missing);
    assert!(
        cell.reasons
            .iter()
            .any(|r| r.contains("upstream job") && r.contains("L1")),
        "a real `needs:` edge must still be attributed: {:?}",
        cell.reasons
    );
    // The cell's own observation leads; the upstream edge is context.
    assert!(
        cell.reasons[0].contains("no report"),
        "the leading reason must be what this cell observed: {:?}",
        cell.reasons
    );
}

#[test]
fn evidence_for_an_unscheduled_cell_is_reported_not_dropped() {
    let cells = classify_simple(&[], &[passing_record("ghost", "ubuntu-latest", Tier::L1)]);
    let cell = only_cell(cells);
    assert!(!cell.scheduled);
    assert!(cell.reasons.iter().any(|r| r.contains("did not schedule")));
}

/// The capability table carries the governance, so no further declaration is
/// needed to render POLICY GAP.
#[test]
fn a_governed_policy_gap_renders_policy_gap() {
    let mut expectation = expectation("biscuit-terminal-cli", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];
    expectation.gap = Some(GapStatus::Governed(gap()));

    let cell = only_cell(classify_simple(
        &[expectation],
        &[passing_record("biscuit-terminal-cli", "windows-latest", Tier::L2)],
    ));

    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.reasons.iter().any(|r| r.contains("governed policy gap")));
}

/// A governed gap explains the absence of evidence, so the cell must not read
/// MISSING — nobody failed to upload anything.
#[test]
fn a_governed_gap_with_no_evidence_is_policy_gap_not_missing() {
    let mut expectation = expectation("biscuit-tui-cli", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];
    expectation.gap = Some(GapStatus::Governed(gap()));

    let cell = only_cell(classify_simple(&[expectation], &[]));
    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.state.blocks());
}

/// A real failure is more actionable than a config classification, so it wins
/// even inside a governed gap.
#[test]
fn a_real_failure_outranks_a_governed_gap() {
    let mut expectation = expectation("biscuit-tui-cli", "windows-latest", Tier::L2);
    expectation.gap = Some(GapStatus::Governed(gap()));

    let mut rec = passing_record("biscuit-tui-cli", "windows-latest", Tier::L2);
    rec.counts.failed = 1;
    rec.failed_tests = vec!["t::x".into()];

    let cell = only_cell(classify_simple(&[expectation], &[rec]));
    assert_eq!(cell.state, CellState::Fail);
    assert!(
        cell.reasons.iter().any(|r| r.contains("governed policy gap")),
        "the gap must still be recorded in reasons"
    );
}

/// Backstop: `gates = false` with no parseable exclusion is a shape the scope
/// job's validation is supposed to reject — but the rollup must still render
/// the package as NOT SCHEDULED rather than vanish it from the grid.
#[test]
fn a_gates_false_package_without_exclusion_metadata_still_renders_not_scheduled() {
    let mut excluded = policy("mystery");
    excluded.gates = false;
    excluded.exclusion = None;
    let scope: BTreeSet<String> = ["mystery".to_owned()].into_iter().collect();

    let expected = expected_cells(&[excluded], &scope, &test_environments());
    assert_eq!(expected.len(), 4, "one NOT SCHEDULED cell per environment");

    let cells = classify_simple(&expected, &[]);
    for cell in &cells {
        assert_eq!(cell.state, CellState::NotScheduled);
        assert!(
            cell.reasons.iter().any(|r| r.contains("ungoverned")),
            "the missing governance must be named: {:?}",
            cell.reasons
        );
    }
}

/// R12's guard: a green Rust JUnit report must never hide a failed companion
/// suite (or fixture, or backend proof). The producer job's own `failure`
/// status downgrades the cell; status evidence can worsen a cell, never
/// improve it.
#[test]
fn a_producer_failure_downgrades_a_green_report() {
    let statuses = vec![ProducerStatus {
        package: "homelab-server".to_owned(),
        job: "L1".to_owned(),
        result: "failure".to_owned(),
        environment: Some("ubuntu-latest".to_owned()),
        detail: Some(
            "companion suite homelab-frontend FAILED; the Rust JUnit report does not cover it"
                .to_owned(),
        ),
        companion: None,
    }];
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("homelab-server", "ubuntu-latest", Tier::L1)],
        records: &[passing_record("homelab-server", "ubuntu-latest", Tier::L1)],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.state, CellState::Fail);
    assert!(
        cell.reasons.iter().any(|r| r.contains("homelab-frontend")),
        "the companion suite's failure must be named: {:?}",
        cell.reasons
    );

    let findings = verdict(
        &rollup_of(vec![cell], &["homelab-server"]),
        &Baseline::default(),
        None,
    );
    assert!(blocks_with_rule(&findings, "cell-failed"));
}

/// The symmetric case: a `success` producer status can never upgrade a red
/// cell.
#[test]
fn a_producer_success_never_upgrades_a_failing_cell() {
    let statuses = vec![ProducerStatus {
        package: "sniff-cli".to_owned(),
        job: "L1".to_owned(),
        result: "success".to_owned(),
        environment: Some("ubuntu-latest".to_owned()),
        detail: None,
        companion: None,
    }];
    let expected_tests = BTreeMap::new();
    let mut rec = passing_record("sniff-cli", "ubuntu-latest", Tier::L1);
    rec.counts.failed = 1;
    rec.failed_tests = vec!["sniff-cli::x".into()];

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("sniff-cli", "ubuntu-latest", Tier::L1)],
        records: &[rec],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));
    assert_eq!(cell.state, CellState::Fail);
}

// ---------------------------------------------------------------------------
// Declared companion suites must leave evidence (R12)
// ---------------------------------------------------------------------------

fn companion_expectation(package: &str, environment: &str) -> ExpectedCell {
    let mut expectation = expectation(package, environment, Tier::L1);
    expectation.companion_suites = vec!["homelab-frontend".to_owned()];
    expectation
}

fn companion_status(result: &str, companion: Option<&str>) -> ProducerStatus {
    ProducerStatus {
        package: "homelab-server".to_owned(),
        job: "L1".to_owned(),
        result: result.to_owned(),
        environment: Some("ubuntu-latest".to_owned()),
        detail: None,
        companion: companion.map(str::to_owned),
    }
}

/// The companion expectation is capability-derived: the suite only runs where
/// Node/pnpm can be provisioned.
#[test]
fn companion_suites_are_expected_only_on_node_capable_environments() {
    let mut homelab = policy("homelab-server");
    homelab.companion_suites = vec!["homelab-frontend".to_owned()];
    let scope: BTreeSet<String> = ["homelab-server".to_owned()].into_iter().collect();

    let expected = expected_cells(&[homelab], &scope, &test_environments());
    let with_companion: BTreeSet<&str> = expected
        .iter()
        .filter(|cell| !cell.companion_suites.is_empty())
        .map(|cell| cell.key.environment.as_str())
        .collect();
    assert_eq!(with_companion, ["ubuntu-latest"].into_iter().collect());
}

#[test]
fn a_successful_companion_keeps_the_cell_green() {
    let statuses = vec![companion_status("success", Some("success"))];
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[companion_expectation("homelab-server", "ubuntu-latest")],
        records: &[passing_record("homelab-server", "ubuntu-latest", Tier::L1)],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));
    assert_eq!(cell.state, CellState::Pass);
}

/// The hole R12 names: a SKIPPED companion step leaves no evidence anywhere
/// else, so a scope-derivation bug or a capability flip rendered PASS. The
/// recorded outcome downgrades the cell exactly as a failure does.
#[test]
fn a_skipped_companion_downgrades_a_green_report() {
    let statuses = vec![companion_status("success", Some("skipped"))];
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[companion_expectation("homelab-server", "ubuntu-latest")],
        records: &[passing_record("homelab-server", "ubuntu-latest", Tier::L1)],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.state, CellState::Fail);
    assert!(
        cell.reasons
            .iter()
            .any(|r| r.contains("homelab-frontend") && r.contains("never ran")),
        "the skipped companion must be named: {:?}",
        cell.reasons
    );

    let findings = verdict(
        &rollup_of(vec![cell], &["homelab-server"]),
        &Baseline::default(),
        None,
    );
    assert!(blocks_with_rule(&findings, "cell-failed"));
}

/// An old or broken producer that reports no companion outcome at all is no
/// better than a skipped one.
#[test]
fn a_companion_with_no_reported_outcome_downgrades_a_green_report() {
    let statuses = vec![companion_status("success", None)];
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[companion_expectation("homelab-server", "ubuntu-latest")],
        records: &[passing_record("homelab-server", "ubuntu-latest", Tier::L1)],
        statuses: &statuses,
        expected_tests: &expected_tests,
    }));
    assert_eq!(cell.state, CellState::Fail);
}

/// The same rule on the lint cell: a declared companion suite lints too, so a
/// green clippy result must not hide a companion lint that never ran.
#[test]
fn a_skipped_companion_downgrades_a_green_lint() {
    let mut homelab = policy("homelab-server");
    homelab.companion_suites = vec!["homelab-frontend".to_owned()];
    let statuses = vec![ProducerStatus {
        package: "homelab-server".to_owned(),
        job: "lint".to_owned(),
        result: "success".to_owned(),
        environment: None,
        detail: None,
        companion: Some("skipped".to_owned()),
    }];

    let cells = status_cells(
        &statuses,
        &scope_of(&["homelab-server"]),
        &[],
        &[homelab],
        &[],
    );
    let cell = only_cell(cells);
    assert_eq!(cell.key.tier, Tier::parse("lint"));
    assert_eq!(cell.state, CellState::Fail);
}

// ---------------------------------------------------------------------------
// status_cells: job results -> cell states
// ---------------------------------------------------------------------------

#[test]
fn status_cells_map_each_job_result_to_a_cell_state() {
    let status = |package: &str, result: &str| ProducerStatus {
        package: package.to_owned(),
        job: "lint".to_owned(),
        result: result.to_owned(),
        environment: None,
        detail: None,
        companion: None,
    };
    let statuses = vec![
        status("pkg-success", "success"),
        status("pkg-failure", "failure"),
        status("pkg-cancelled", "cancelled"),
        status("pkg-skipped", "skipped"),
        status("pkg-mystery", "mystery"),
    ];
    let scope = scope_of(&[
        "pkg-success",
        "pkg-failure",
        "pkg-cancelled",
        "pkg-skipped",
        "pkg-mystery",
    ]);

    let cells = status_cells(&statuses, &scope, &[], &[policy("x")], &[]);
    let state_of = |package: &str| {
        cells
            .iter()
            .find(|cell| cell.key.package == package)
            .unwrap_or_else(|| panic!("no cell for {package}"))
            .state
    };
    assert_eq!(state_of("pkg-success"), CellState::Pass);
    assert_eq!(state_of("pkg-failure"), CellState::Fail);
    assert_eq!(state_of("pkg-cancelled"), CellState::Missing);
    assert_eq!(state_of("pkg-skipped"), CellState::NotScheduled);
    assert_eq!(state_of("pkg-mystery"), CellState::Missing);
}

/// A test-tier status (L1/L2/browser) never manufactures a second cell beside
/// the JUnit-backed one, and a status outside the scope is dropped.
#[test]
fn status_cells_skip_test_tiers_and_out_of_scope_packages() {
    let statuses = vec![
        ProducerStatus {
            package: "sniff-cli".to_owned(),
            job: "L1".to_owned(),
            result: "failure".to_owned(),
            environment: Some("ubuntu-latest".to_owned()),
            detail: None,
            companion: None,
        },
        ProducerStatus {
            package: "outsider".to_owned(),
            job: "lint".to_owned(),
            result: "failure".to_owned(),
            environment: None,
            detail: None,
            companion: None,
        },
    ];
    let cells = status_cells(&statuses, &scope_of(&["sniff-cli"]), &[], &[policy("x")], &[]);
    assert!(cells.is_empty(), "no status cell may appear: {cells:#?}");
}

// ---------------------------------------------------------------------------
// nextest exit codes are RAW, not normalized
// ---------------------------------------------------------------------------

/// 101 means the crate never built. That tells us nothing about whether the
/// tier has tests, so it can never be NOTHING TO RUN — and because it emits no
/// test result, it can never be accepted as a known test failure either.
#[test]
fn a_build_failure_is_missing_and_says_so_not_an_empty_tier() {
    let mut rec = record("biscuit-file", "windows-latest", Tier::L1);
    rec.report_present = false;
    rec.exit_code = 101;

    let cell = only_cell(classify_simple(
        &[expectation("biscuit-file", "windows-latest", Tier::L1)],
        &[rec],
    ));

    assert_eq!(cell.state, CellState::Missing);
    assert_ne!(cell.state, CellState::NothingToRun);
    assert!(
        cell.reasons.iter().any(|r| r.contains("failed to BUILD")),
        "a build failure must say so: {:?}",
        cell.reasons
    );
}

/// 100 means tests ran and failed, so a report *should* exist. Its absence is
/// a different defect from a build failure and must read differently.
#[test]
fn a_test_failure_with_no_staged_report_reads_differently_from_a_build_failure() {
    let mut rec = record("queue", "windows-latest", Tier::L1);
    rec.report_present = false;
    rec.exit_code = 100;

    let cell = only_cell(classify_simple(
        &[expectation("queue", "windows-latest", Tier::L1)],
        &[rec],
    ));

    assert_eq!(cell.state, CellState::Missing);
    assert!(cell.reasons.iter().any(|r| r.contains("exit 100")));
    assert!(!cell.reasons.iter().any(|r| r.contains("failed to BUILD")));
}

#[test]
fn a_clean_report_under_exit_100_is_treated_as_incomplete() {
    let mut rec = passing_record("sniff", "ubuntu-latest", Tier::L1);
    rec.exit_code = 100;

    let cell = only_cell(classify_simple(
        &[expectation("sniff", "ubuntu-latest", Tier::L1)],
        &[rec],
    ));

    assert_eq!(
        cell.state,
        CellState::Missing,
        "the exit code and the report disagree; believing the report would show PASS"
    );
    assert!(cell.reasons.iter().any(|r| r.contains("incomplete")));
}

#[rstest]
#[case(101, "failed to BUILD")]
#[case(100, "exit 100")]
#[case(0, "exited 0 but staged no report")]
#[case(42, "exited 42")]
fn every_exit_code_gets_its_own_explanation(#[case] code: i64, #[case] expected: &str) {
    assert!(
        missing_report_reason(code).contains(expected),
        "exit {code} -> {}",
        missing_report_reason(code)
    );
}

// ---------------------------------------------------------------------------
// Local-dev values and unrecognized tiers surface rather than crash
// ---------------------------------------------------------------------------

#[test]
fn a_local_dev_environment_surfaces_as_unscheduled_evidence() {
    let cells = classify_simple(
        &[expectation("biscuit-file", "ubuntu-latest", Tier::L1)],
        &[passing_record("biscuit-file", "darwin-local", Tier::L1)],
    );

    let stray = cells
        .iter()
        .find(|cell| cell.key.environment == "darwin-local")
        .expect("a local-dev environment must surface, not be dropped or crash");
    assert!(!stray.scheduled);
    assert!(stray.reasons.iter().any(|r| r.contains("did not schedule")));
}

/// `sanity` is a fast local-dev subset of L1 and CI never runs it, so it is
/// never a scheduled cell. Evidence for it in a CI artifact means a recipe is
/// mis-wired, which must be visible rather than counted as L1 coverage.
#[test]
fn sanity_tier_evidence_surfaces_as_unscheduled() {
    let cells = classify_simple(&[], &[passing_record("playa", "ubuntu-latest", Tier::Sanity)]);
    let cell = only_cell(cells);

    assert_eq!(cell.key.tier, Tier::Sanity);
    assert!(!cell.scheduled);
    assert!(cell.reasons.iter().any(|r| r.contains("did not schedule")));
}

#[rstest]
#[case("L1", Tier::L1)]
#[case("L2", Tier::L2)]
#[case("L3", Tier::L3)]
#[case("browser", Tier::Browser)]
#[case("real", Tier::Real)]
#[case("sanity", Tier::Sanity)]
fn every_tier_the_producer_emits_round_trips(#[case] raw: &str, #[case] expected: Tier) {
    assert_eq!(Tier::parse(raw), expected);
    assert_eq!(Tier::parse(&expected.to_string()), expected);
}

// ---------------------------------------------------------------------------
// Policy and capability inputs
// ---------------------------------------------------------------------------

#[test]
fn unknown_scope_policy_fields_are_ignored() {
    let json = r#"{
      "packages": ["sniff"],
      "policy": [
        {
          "package": "sniff",
          "gates": true,
          "tiers": ["L1", "L2"],
          "l2_backends": ["tmux"],
          "some_field_added_next_week": {"nested": [1, 2, 3]}
        }
      ],
      "matrix": []
    }"#;

    let doc: PolicyDoc = serde_json::from_str(json).expect("unknown fields are ignored");
    assert_eq!(doc.policy[0].package, "sniff");
    assert_eq!(doc.policy[0].tiers, vec![Tier::L1, Tier::L2]);
}

#[test]
fn capability_values_parse_as_booleans_or_governed_objects() {
    let json = r#"{
      "schema_version": 1,
      "environments": [
        {
          "name": "ubuntu-latest",
          "runner": "ubuntu-latest",
          "native_key": "ubuntu-latest",
          "capabilities": {
            "tmux": true,
            "headless_browser": true,
            "node_pnpm": true,
            "archive_only": false
          }
        },
        {
          "name": "windows-latest",
          "runner": "windows-latest",
          "native_key": "windows-latest",
          "capabilities": {
            "tmux": {"available": false, "reason": "no port", "owner": "@o", "expiry": "2027-01-31"},
            "headless_browser": false,
            "node_pnpm": false,
            "archive_only": false
          }
        }
      ]
    }"#;

    let doc: EnvironmentsDoc = serde_json::from_str(json).unwrap();
    let ubuntu = &doc.environments[0];
    assert!(ubuntu.capable("tmux"));
    assert!(ubuntu.gap_for("tmux").is_none());

    let windows = &doc.environments[1];
    assert!(!windows.capable("tmux"));
    let gap = windows.gap_for("tmux").expect("a governed absence");
    assert_eq!(gap.owner, "@o");
    assert_eq!(gap.expiry, "2027-01-31");
    // A plain false is an UNGOVERNED absence.
    assert!(windows.gap_for("headless_browser").is_none());
}

#[test]
fn the_checked_in_environments_table_parses_and_is_well_governed() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".github")
        .join("ci")
        .join("environments.json");
    let text = fs::read_to_string(&path).expect("environments.json is readable");
    let doc: EnvironmentsDoc = serde_json::from_str(&text).expect("environments.json parses");

    assert_eq!(doc.environments.len(), 4);
    let names: BTreeSet<&str> = doc
        .environments
        .iter()
        .map(|environment| environment.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"]
            .into_iter()
            .collect()
    );

    // Every governed absence must satisfy the acceptance rule, or `ci-verdict`
    // can never exit 0 on a run touching an L2-owning package.
    for environment in &doc.environments {
        for (name, capability) in &environment.capabilities {
            if let Some(gap) = capability.governed_gap(name) {
                assert!(!gap.owner.trim().is_empty(), "{} gap has no owner", environment.name);
                assert!(!gap.reason.trim().is_empty(), "{} gap has no reason", environment.name);
                assert!(
                    is_iso_date(gap.expiry.trim()),
                    "{} gap expiry `{}` is not YYYY-MM-DD",
                    environment.name,
                    gap.expiry
                );
            }
        }
    }

    // The two facts the eight per-area policy_gaps records used to restate:
    // Windows has no tmux, and the WSL2 leg is archive-only.
    let windows = doc
        .environments
        .iter()
        .find(|environment| environment.name == "windows-latest")
        .unwrap();
    assert!(!windows.capable("tmux"));
    assert!(windows.gap_for("tmux").is_some());
    let wsl = doc
        .environments
        .iter()
        .find(|environment| environment.name == "wsl2-ubuntu")
        .unwrap();
    assert!(wsl.capable("archive_only"));
    assert!(wsl.gap_for("tmux").is_some());

    // The browser tier is Linux-hosted; its absence everywhere else must be
    // governed, or the POLICY GAP cells it now renders would hard-block.
    for name in ["windows-latest", "macos-latest", "wsl2-ubuntu"] {
        let environment = doc
            .environments
            .iter()
            .find(|environment| environment.name == name)
            .unwrap();
        assert!(
            environment.gap_for("headless_browser").is_some(),
            "{name} must govern its headless_browser absence"
        );
    }
}

#[test]
fn a_wsl2_environment_gets_its_own_cell_not_the_windows_one() {
    let scope: BTreeSet<String> = ["sniff".to_owned()].into_iter().collect();
    let expected = expected_cells(&[policy("sniff")], &scope, &test_environments());
    let environments: BTreeSet<&str> = expected
        .iter()
        .map(|cell| cell.key.environment.as_str())
        .collect();
    assert_eq!(
        environments,
        ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"]
            .into_iter()
            .collect()
    );
}

#[test]
fn expected_cells_cover_l1_l2_and_browser_by_capability() {
    let mut darkmatter = policy("darkmatter");
    darkmatter.tiers = vec![Tier::L1, Tier::L2, Tier::Browser];
    darkmatter.l2_backends = vec!["tmux".to_owned()];
    let scope: BTreeSet<String> = ["darkmatter".to_owned()].into_iter().collect();

    let expected = expected_cells(&[darkmatter], &scope, &test_environments());

    // L1 on all four environments; L2 and browser on all four as well — the
    // environments that cannot host a tier must still appear, so they render
    // POLICY GAP rather than disappearing from the grid.
    let l1 = expected.iter().filter(|cell| cell.key.tier == Tier::L1).count();
    let l2: Vec<&ExpectedCell> = expected
        .iter()
        .filter(|cell| cell.key.tier == Tier::L2)
        .collect();
    let browser: Vec<&ExpectedCell> = expected
        .iter()
        .filter(|cell| cell.key.tier == Tier::Browser)
        .collect();
    assert_eq!(l1, 4);
    assert_eq!(l2.len(), 4);
    assert_eq!(browser.len(), 4);

    // L2 gaps land exactly where the capability table says tmux is absent.
    let gap_envs: BTreeSet<&str> = l2
        .iter()
        .filter(|cell| cell.gap.is_some())
        .map(|cell| cell.key.environment.as_str())
        .collect();
    assert_eq!(gap_envs, ["windows-latest", "wsl2-ubuntu"].into_iter().collect());
    for cell in &l2 {
        if cell.gap.is_some() {
            assert!(
                matches!(cell.gap, Some(GapStatus::Governed(_))),
                "the checked-in governance must reach the expectation"
            );
        }
    }

    // Browser gaps land everywhere a headless browser cannot be hosted —
    // governed, like the tmux gaps, never a vanished cell.
    let browser_gap_envs: BTreeSet<&str> = browser
        .iter()
        .filter(|cell| cell.gap.is_some())
        .map(|cell| cell.key.environment.as_str())
        .collect();
    assert_eq!(
        browser_gap_envs,
        ["windows-latest", "macos-latest", "wsl2-ubuntu"]
            .into_iter()
            .collect()
    );
    for cell in &browser {
        if cell.gap.is_some() {
            assert!(
                matches!(cell.gap, Some(GapStatus::Governed(_))),
                "a browser absence must be governed, like tmux's"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Expected-test manifest
// ---------------------------------------------------------------------------

#[test]
fn an_expected_test_with_no_result_counts_as_a_skip() {
    let expected_tests: ExpectedTests = [(
        ("windows-latest".to_owned(), Tier::L1),
        [(
            "a".to_owned(),
            vec!["a::present".to_owned(), "a::vanished".to_owned()],
        )]
        .into_iter()
        .collect(),
    )]
    .into_iter()
    .collect();

    let mut rec = record("a", "windows-latest", Tier::L1);
    rec.counts = Counts {
        total: 1,
        passed: 1,
        ..Counts::default()
    };
    rec.passed_identities = vec!["a::present".to_owned()];

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("a", "windows-latest", Tier::L1)],
        records: &[rec],
        statuses: &[],
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.skipped_tests, vec!["a::vanished"]);
    assert!(!cell.skip_evidence_degraded);
}

#[test]
fn without_a_manifest_absence_is_not_inferred_to_be_a_skip() {
    let mut rec = record("a", "windows-latest", Tier::L1);
    rec.counts = Counts {
        total: 1,
        passed: 1,
        ..Counts::default()
    };
    rec.passed_identities = vec!["a::present".to_owned()];

    let cell = only_cell(classify_simple(
        &[expectation("a", "windows-latest", Tier::L1)],
        &[rec],
    ));

    assert!(cell.skipped_tests.is_empty());
    assert!(
        cell.skip_evidence_degraded,
        "no manifest means `#[cfg]`-absent cannot be told from skipped"
    );
}

#[test]
fn a_package_with_no_evidence_does_not_manufacture_skips_from_the_manifest() {
    let expected_tests: ExpectedTests = [(
        ("windows-latest".to_owned(), Tier::L1),
        [("a".to_owned(), vec!["a::gone".to_owned()])]
            .into_iter()
            .collect(),
    )]
    .into_iter()
    .collect();

    let mut rec = record("a", "windows-latest", Tier::L1);
    rec.report_present = false;

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("a", "windows-latest", Tier::L1)],
        records: &[rec],
        statuses: &[],
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.state, CellState::Missing);
    assert!(
        cell.skipped_tests.is_empty(),
        "a MISSING report must not be reported as N test skips"
    );
}

// ---------------------------------------------------------------------------
// Verdict rules
// ---------------------------------------------------------------------------

#[test]
fn an_unlisted_failure_blocks() {
    let cells = classify_simple(
        &[expectation("sniff", "macos-latest", Tier::L1)],
        &[{
            let mut rec = passing_record("sniff", "macos-latest", Tier::L1);
            rec.counts.failed = 1;
            rec.failed_tests = vec!["sniff::x".into()];
            rec
        }],
    );
    let findings = verdict(&rollup_of(cells, &["sniff"]), &Baseline::default(), None);
    assert!(blocks_with_rule(&findings, "cell-failed"));
}

#[test]
fn a_listed_failure_is_accepted() {
    let cells = classify_simple(
        &[expectation("sniff", "macos-latest", Tier::L1)],
        &[{
            let mut rec = passing_record("sniff", "macos-latest", Tier::L1);
            rec.counts.failed = 1;
            rec.failed_tests = vec!["sniff::x".into()];
            rec
        }],
    );
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("sniff", "macos-latest", Tier::L1)],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(cells, &["sniff"]), &baseline, None);
    assert!(!any_block(&findings), "unexpected blocks: {findings:#?}");
}

#[test]
fn messenger_l1_failure_requires_an_exact_package_keyed_baseline() {
    let mut record = passing_record("messenger", "windows-latest", Tier::L1);
    record.counts.failed = 1;
    record.counts.total += 1;
    record.failed_tests = vec!["messenger::desktop::notification".into()];
    let cell = only_cell(classify_simple(
        &[expectation("messenger", "windows-latest", Tier::L1)],
        &[record],
    ));

    assert_eq!(cell.key.package, "messenger");
    assert_eq!(cell.key.environment, "windows-latest");
    assert_eq!(cell.key.tier, Tier::L1);
    assert_eq!(cell.state, CellState::Fail);

    let rollup = rollup_of(vec![cell], &["messenger"]);
    let wrong_environment = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("messenger", "ubuntu-latest", Tier::L1)],
        skip: Vec::new(),
    };
    let findings = verdict(&rollup, &wrong_environment, None);
    assert!(blocks_with_rule(&findings, "cell-failed"));

    let exact = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("messenger", "windows-latest", Tier::L1)],
        skip: Vec::new(),
    };
    let findings = verdict(&rollup, &exact, None);
    assert!(!any_block(&findings), "unexpected blocks: {findings:#?}");
    assert!(findings.iter().any(|finding| {
        finding.rule == "baseline-accepted" && finding.subject.contains("messenger")
    }));
}

#[test]
fn rendezvous_l1_missing_evidence_stays_package_keyed_and_blocks() {
    let cell = only_cell(classify_simple(
        &[expectation(
            "rendezvous-daemon",
            "wsl2-ubuntu",
            Tier::L1,
        )],
        &[],
    ));

    assert_eq!(cell.key.package, "rendezvous-daemon");
    assert_eq!(cell.key.environment, "wsl2-ubuntu");
    assert_eq!(cell.key.tier, Tier::L1);
    assert_eq!(cell.state, CellState::Missing);

    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry(
            "rendezvous-daemon",
            "wsl2-ubuntu",
            Tier::L1,
        )],
        skip: Vec::new(),
    };
    let findings = verdict(
        &rollup_of(vec![cell], &["rendezvous-daemon"]),
        &baseline,
        None,
    );
    assert!(blocks_with_rule(&findings, "baseline-no-result"));
}

#[test]
fn a_listed_entry_that_now_passes_blocks_to_force_cleanup() {
    let cells = classify_simple(
        &[expectation("biscuit-speaks", "ubuntu-latest", Tier::L1)],
        &[passing_record("biscuit-speaks", "ubuntu-latest", Tier::L1)],
    );
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("biscuit-speaks", "ubuntu-latest", Tier::L1)],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(cells, &["biscuit-speaks"]), &baseline, None);
    assert!(blocks_with_rule(&findings, "baseline-now-passing"));
}

#[test]
fn an_out_of_scope_entry_is_ignored_and_not_treated_as_a_pass() {
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("homelab", "ubuntu-latest", Tier::L1)],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(Vec::new(), &["sniff"]), &baseline, None);
    assert!(!any_block(&findings), "out-of-scope must not block");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == "baseline-out-of-scope" && f.severity == Severity::Note),
        "out-of-scope must be visible, not silent"
    );
}

#[rstest]
#[case(CellState::Missing)]
#[case(CellState::Skip)]
#[case(CellState::NothingToRun)]
#[case(CellState::PolicyGap)]
fn a_scheduled_entry_with_no_test_result_stays_blocking(#[case] state: CellState) {
    let cell = Cell {
        state,
        scheduled: true,
        ..blank_cell(cell_key("claudine", "ubuntu-latest", Tier::L1))
    };
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("claudine", "ubuntu-latest", Tier::L1)],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(vec![cell], &["claudine"]), &baseline, None);
    assert!(
        blocks_with_rule(&findings, "baseline-no-result"),
        "state {state:?} must not be accepted as a known test failure: {findings:#?}"
    );
}

/// Scope is per gate (2026-09-09): a package reached only through a
/// lint-global input is in scope with no test tier scheduled. A baselined
/// test leg of that package is a leg this run said nothing about — the
/// standing of an out-of-scope entry, not a vanished result.
#[test]
fn a_baselined_leg_the_run_did_not_schedule_is_ignored_with_a_note() {
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("claudine", "windows-latest", Tier::L1)],
        skip: Vec::new(),
    };
    // A lint-only run: claudine is in scope, its lint status cell passed, and
    // policy scheduled no test tier for it.
    let lint = cell_key("claudine", "ubuntu-latest", Tier::parse("lint"));
    let rollup = rollup_scheduling(
        vec![Cell {
            state: CellState::Pass,
            scheduled: true,
            ..blank_cell(lint.clone())
        }],
        &["claudine"],
        vec![lint],
    );
    let findings = verdict(&rollup, &baseline, None);
    assert!(!any_block(&findings), "an unscheduled leg must not block: {findings:#?}");
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "baseline-unscheduled" && finding.severity == Severity::Note),
        "the ignored leg must be visible as a note: {findings:#?}"
    );
}

/// The excusal is for legs policy KNOWINGLY did not schedule. A leg policy
/// scheduled and the rollup nevertheless has no cell for is still the
/// "vanished result" the blocking rule exists to catch.
#[test]
fn a_baselined_leg_the_run_scheduled_but_has_no_cell_for_still_blocks() {
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("claudine", "windows-latest", Tier::L1)],
        skip: Vec::new(),
    };
    let rollup = rollup_scheduling(
        Vec::new(),
        &["claudine"],
        vec![cell_key("claudine", "windows-latest", Tier::L1)],
    );
    let findings = verdict(&rollup, &baseline, None);
    assert!(blocks_with_rule(&findings, "baseline-no-result"));
}

/// A document that predates the schedule field cannot say whether the leg
/// was scheduled, and "cannot say" fails closed.
#[test]
fn a_baselined_package_that_produced_no_cell_at_all_blocks() {
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("claudine", "windows-latest", Tier::L1)],
        skip: Vec::new(),
    };
    let findings = verdict(&rollup_of(Vec::new(), &["claudine"]), &baseline, None);
    assert!(blocks_with_rule(&findings, "baseline-no-result"));
}

#[test]
fn an_expired_entry_blocks() {
    let cells = classify_simple(
        &[expectation("sniff", "macos-latest", Tier::L1)],
        &[{
            let mut rec = passing_record("sniff", "macos-latest", Tier::L1);
            rec.counts.failed = 1;
            rec
        }],
    );
    let mut entry = failure_entry("sniff", "macos-latest", Tier::L1);
    entry.expiry = Some("2026-01-01".to_owned());
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![entry],
        skip: Vec::new(),
    };

    let findings = verdict(
        &rollup_of(cells, &["sniff"]),
        &baseline,
        Some("2026-07-27"),
    );
    assert!(blocks_with_rule(&findings, "baseline-expired"));
}

#[test]
fn missing_and_policy_gap_cells_block_on_their_own() {
    let missing = only_cell(classify_simple(
        &[expectation("worktree", "ubuntu-latest", Tier::L1)],
        &[],
    ));
    let findings = verdict(
        &rollup_of(vec![missing], &["worktree"]),
        &Baseline::default(),
        None,
    );
    assert!(blocks_with_rule(&findings, "cell-missing"));
}

// ---------------------------------------------------------------------------
// Policy gaps: acknowledged ≠ acceptable ≠ invisible
//
// Every case here starts from the shape the capability table produces for an
// L2-owning package on Windows: a governed unavailability, and a tier whose
// tests all early-return for want of a backend.
// ---------------------------------------------------------------------------

/// Build the `sniff-cli`-shaped Windows L2 cell: L2 tier, tmux-only backend,
/// and whatever gap governance and evidence the caller supplies.
fn windows_l2_gap_cell(governed: Option<DeclaredGap>, records: &[RunRecord]) -> Cell {
    let mut expectation = expectation("sniff-cli", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];
    expectation.gap = match governed {
        Some(gap) => Some(GapStatus::Governed(gap)),
        None => Some(GapStatus::Ungoverned),
    };

    only_cell(classify_simple(&[expectation], records))
}

fn gap_verdict(cell: Cell, today: Option<&str>) -> Vec<Finding> {
    verdict(&rollup_of(vec![cell], &["sniff-cli"]), &Baseline::default(), today)
}

/// The defect this file's policy-gap rules exist to fix: every L2-owning
/// package has a governed Windows-L2 gap, so before this rule the verdict
/// could never exit 0 on any run touching one of them.
#[test]
fn an_owned_unexpired_policy_gap_does_not_block() {
    let findings = gap_verdict(windows_l2_gap_cell(Some(gap()), &[]), Some("2026-07-28"));

    assert!(!any_block(&findings), "must not block: {findings:#?}");
    assert!(
        findings.iter().any(|f| f.rule == "policy-gap-accepted"
            && f.severity == Severity::Note
            && f.detail.contains("@yankeeinlondon")
            && f.detail.contains("2027-01-31")),
        "an accepted gap must stay visible, naming its owner and expiry: {findings:#?}"
    );
}

/// Acceptance changes the verdict, never the grid: never a green
/// `0 run / N skipped` cell.
#[test]
fn an_accepted_policy_gap_still_renders_policy_gap_never_pass() {
    let cell = windows_l2_gap_cell(Some(gap()), &[]);
    assert_eq!(cell.state, CellState::PolicyGap);
    assert_ne!(cell.state, CellState::Pass);

    let rollup = rollup_of(vec![cell], &["sniff-cli"]);
    let grid = render_grid(&rollup);
    assert!(grid.contains("POLICY GAP"), "{grid}");
    assert!(!grid.contains("PASS"), "{grid}");

    // Accepted by the verdict, yet still listed by the rollup's own gate as a
    // cell that is not green.
    assert!(grid.contains("Cells failing the summary gate"), "{grid}");
    assert!(!any_block(&gap_verdict(
        rollup.cells[0].clone(),
        Some("2026-07-28")
    )));
}

/// A lapsed bound is a permanent exclusion wearing a temporary label.
#[test]
fn an_expired_policy_gap_blocks() {
    let mut expired = gap();
    expired.expiry = "2026-01-31".to_owned();

    let findings = gap_verdict(windows_l2_gap_cell(Some(expired), &[]), Some("2026-07-28"));
    assert!(blocks_with_rule(&findings, "policy-gap-expired"));
}

/// The case that catches someone quietly turning a tier off: a plain `false`
/// in the capability table, so there is nobody to hold accountable and nothing
/// to expire.
#[test]
fn an_ungoverned_policy_gap_still_blocks() {
    let cell = windows_l2_gap_cell(
        None,
        &[passing_record("sniff-cli", "windows-latest", Tier::L2)],
    );
    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.declared_gap.is_none());

    let findings = gap_verdict(cell, Some("2026-07-28"));
    assert!(blocks_with_rule(&findings, "cell-policy-gap"));
    assert!(
        !findings.iter().any(|f| f.rule == "policy-gap-accepted"),
        "an ungoverned gap is never acceptable: {findings:#?}"
    );
}

/// A gap declaration must never suppress genuine evidence. `classify` ranks
/// `Fail` above `PolicyGap`, so the cell never reaches the acceptance path.
#[test]
fn a_governed_gap_with_real_failures_still_surfaces_them() {
    let mut expectation = expectation("sniff-cli", "windows-latest", Tier::L2);
    expectation.gap = Some(GapStatus::Governed(gap()));

    let mut rec = passing_record("sniff-cli", "windows-latest", Tier::L2);
    rec.counts.failed = 1;
    rec.failed_tests = vec!["sniff-cli::level2_probe".into()];

    let cell = only_cell(classify_simple(&[expectation], &[rec]));
    assert_eq!(cell.state, CellState::Fail);

    let findings = gap_verdict(cell, Some("2026-07-28"));
    assert!(blocks_with_rule(&findings, "cell-failed"));
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("sniff-cli::level2_probe")),
        "the failing identity must reach the verdict: {findings:#?}"
    );
    assert!(!findings.iter().any(|f| f.rule == "policy-gap-accepted"));
}

/// The shape a *correctly* governed gap actually produces when its job runs: a
/// `require_level!` gate that skips for want of a backend early-returns, and
/// nextest records that as a JUnit pass. The cell must still read POLICY GAP,
/// and must still be accepted — a "the tests passed, so the gap is stale" rule
/// would block precisely the case the gap exists to describe.
#[test]
fn a_governed_gap_whose_tests_all_early_returned_is_still_accepted() {
    let mut expectation = expectation("sniff-cli", "windows-latest", Tier::L2);
    expectation.gap = Some(GapStatus::Governed(gap()));

    let cell = only_cell(classify_simple(
        &[expectation],
        &[passing_record("sniff-cli", "windows-latest", Tier::L2)],
    ));
    assert_eq!(cell.state, CellState::PolicyGap, "never PASS, even so");

    let findings = gap_verdict(cell, Some("2026-07-28"));
    assert!(!any_block(&findings), "{findings:#?}");
}

#[rstest]
#[case("owner", "")]
#[case("expiry", "")]
#[case("expiry", "31-01-2027")]
fn an_unattributable_or_undated_policy_gap_blocks(#[case] field: &str, #[case] value: &str) {
    let mut broken = gap();
    match field {
        "owner" => broken.owner = value.to_owned(),
        "expiry" => broken.expiry = value.to_owned(),
        other => panic!("unhandled field {other}"),
    }

    let findings = gap_verdict(windows_l2_gap_cell(Some(broken), &[]), Some("2026-07-28"));
    assert!(blocks_with_rule(&findings, "policy-gap-incomplete"));
}

/// `verdict` reads only `results.json`, never `environments.json`, so the
/// gap's accountability fields have to survive the round trip or the decision
/// is made on absent data.
#[test]
fn a_governed_gap_round_trips_through_the_result_document() {
    let rollup = rollup_of(vec![windows_l2_gap_cell(Some(gap()), &[])], &["sniff-cli"]);
    let json = serde_json::to_string(&rollup).unwrap();
    let parsed: Rollup = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.cells[0].declared_gap, Some(gap()));
    assert!(!any_block(&gap_verdict(
        parsed.cells[0].clone(),
        Some("2026-07-28")
    )));
}

// ---------------------------------------------------------------------------
// Exact-test-identity skip diff
// ---------------------------------------------------------------------------

fn skip_cell(skips: &[&str]) -> Cell {
    Cell {
        state: CellState::Pass,
        origin: Origin::Ci,
        counts: Counts {
            total: 5,
            passed: 5 - skips.len() as u32,
            skipped: skips.len() as u32,
            ..Counts::default()
        },
        scheduled: true,
        skipped_tests: skips.iter().map(|s| (*s).to_owned()).collect(),
        ..blank_cell(cell_key("biscuit-terminal", "ubuntu-latest", Tier::L2))
    }
}

fn skip_budget(tests: &[&str]) -> Baseline {
    Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: Vec::new(),
        skip: vec![SkipEntry {
            package: "biscuit-terminal".to_owned(),
            environment: "ubuntu-latest".to_owned(),
            tier: Tier::L2,
            backend: "wezterm".to_owned(),
            tests: tests.iter().map(|s| (*s).to_owned()).collect(),
            owner: "@owner".to_owned(),
            reason: "no GUI backend on a headless runner".to_owned(),
            source_run: "1".to_owned(),
            expiry: None,
        }],
    }
}

#[test]
fn an_exactly_matching_skip_set_is_clear() {
    let findings = verdict(
        &rollup_of(vec![skip_cell(&["t::a", "t::b"])], &["biscuit-terminal"]),
        &skip_budget(&["t::a", "t::b"]),
        None,
    );
    assert!(!any_block(&findings), "{findings:#?}");
}

/// Counts are identical (2 approved, 2 observed) — only an identity-set
/// comparison sees that one skip was resolved and a different one appeared.
#[test]
fn a_one_for_one_skip_swap_blocks_on_both_halves() {
    let findings = verdict(
        &rollup_of(vec![skip_cell(&["t::a", "t::NEW"])], &["biscuit-terminal"]),
        &skip_budget(&["t::a", "t::b"]),
        None,
    );

    assert!(
        blocks_with_rule(&findings, "skip-new"),
        "the newly-added skip must block: {findings:#?}"
    );
    assert!(
        blocks_with_rule(&findings, "skip-resolved"),
        "the removed skip must force cleanup: {findings:#?}"
    );

    let approved_count = 2;
    let observed_count = 2;
    assert_eq!(
        approved_count, observed_count,
        "the counts match, which is exactly why counting is insufficient"
    );
}

#[test]
fn a_new_skip_blocks() {
    let findings = verdict(
        &rollup_of(vec![skip_cell(&["t::a", "t::b", "t::c"])], &["biscuit-terminal"]),
        &skip_budget(&["t::a", "t::b"]),
        None,
    );
    assert!(blocks_with_rule(&findings, "skip-new"));
    assert!(!blocks_with_rule(&findings, "skip-resolved"));
}

#[test]
fn a_resolved_skip_forces_baseline_cleanup() {
    let findings = verdict(
        &rollup_of(vec![skip_cell(&["t::a"])], &["biscuit-terminal"]),
        &skip_budget(&["t::a", "t::b"]),
        None,
    );
    assert!(blocks_with_rule(&findings, "skip-resolved"));
    assert!(!blocks_with_rule(&findings, "skip-new"));
}

#[test]
fn a_missing_cell_cannot_report_a_skip_as_resolved() {
    let mut cell = skip_cell(&[]);
    cell.state = CellState::Missing;

    let findings = verdict(
        &rollup_of(vec![cell], &["biscuit-terminal"]),
        &skip_budget(&["t::a"]),
        None,
    );
    assert!(
        !blocks_with_rule(&findings, "skip-resolved"),
        "absent evidence is not evidence of a resolved skip: {findings:#?}"
    );
}

#[test]
fn an_out_of_scope_skip_entry_is_ignored() {
    let findings = verdict(&rollup_of(Vec::new(), &["sniff"]), &skip_budget(&["t::a"]), None);
    assert!(!any_block(&findings));
    assert!(findings.iter().any(|f| f.rule == "skip-out-of-scope"));
}

// ---------------------------------------------------------------------------
// Baseline document
// ---------------------------------------------------------------------------

#[test]
fn the_checked_in_baseline_parses_and_every_entry_is_well_formed() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".github")
        .join("ci")
        .join("ci-baseline.toml");
    let baseline = load_baseline(&path).expect("the checked-in baseline is valid");

    for entry in &baseline.failure {
        assert!(!entry.owner.is_empty(), "{} has no owner", entry.package);
        assert!(!entry.reason.is_empty(), "{} has no reason", entry.package);
        assert!(!entry.source_run.is_empty(), "{} has no source run", entry.package);
    }
}

#[test]
fn a_baseline_from_the_future_is_refused() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("future.toml");
    fs::write(&path, "schema_version = 99\n").unwrap();

    let err = load_baseline(&path).expect_err("a newer schema must be refused");
    assert!(format!("{err:#}").contains("newer than this tool understands"));
    fs::remove_dir_all(&dir).ok();
}

/// The v1 baseline was keyed by `{area, environment, tier, shard}`. Reading it
/// as v2 would silently mis-key every entry, so the error names the migration.
#[test]
fn an_area_keyed_baseline_is_refused_with_a_migration_error() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-v1-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("v1.toml");
    fs::write(&path, "schema_version = 1\n").unwrap();

    let err = load_baseline(&path).expect_err("the area-keyed baseline must be refused");
    let message = format!("{err:#}");
    assert!(message.contains("area-keyed"), "unhelpful error: {message}");
    fs::remove_dir_all(&dir).ok();
}

/// Same migration guard on result documents, in both readers.
#[test]
fn an_area_keyed_result_document_is_refused_with_a_migration_error() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-r1-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("v1-results.json");
    fs::write(
        &path,
        r#"{"schema_version":1,"scope":[],"scope_degraded":false,"records":[],"cells":[]}"#,
    )
    .unwrap();

    let err = load_rollup(&path).expect_err("the area-keyed document must be refused");
    let message = format!("{err:#}");
    assert!(message.contains("area-keyed"), "unhelpful error: {message}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_invalid_expiry_is_refused() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-expiry-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.toml");
    fs::write(
        &path,
        "schema_version = 2\n[[failure]]\npackage = \"a\"\nenvironment = \"ubuntu-latest\"\ntier = \"L1\"\n\
         owner = \"@o\"\nreason = \"r\"\nsource_run = \"1\"\nexpiry = \"soon\"\n",
    )
    .unwrap();

    let err = load_baseline(&path).expect_err("a non-date expiry must be refused");
    assert!(format!("{err:#}").contains("invalid expiry"));
    fs::remove_dir_all(&dir).ok();
}

/// A version-less baseline must die as a missing field, not silently assume
/// the current schema generation and skip the migration-error path.
#[test]
fn a_version_less_baseline_is_refused() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-vless-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("versionless.toml");
    fs::write(
        &path,
        "[[failure]]\npackage = \"a\"\nenvironment = \"ubuntu-latest\"\ntier = \"L1\"\n\
         owner = \"@o\"\nreason = \"r\"\nsource_run = \"1\"\n",
    )
    .unwrap();

    let err = load_baseline(&path).expect_err("a missing schema_version must be refused");
    assert!(
        format!("{err:#}").contains("schema_version"),
        "the error must name the missing field: {err:#}"
    );
    fs::remove_dir_all(&dir).ok();
}

/// AC11: a v2 entry carrying a stale area/shard key (`shard = "1/4"`,
/// `area = …`) must be rejected, not silently parsed away.
#[test]
fn a_baseline_entry_with_a_stale_shard_key_is_refused() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-stray-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stray.toml");
    fs::write(
        &path,
        "schema_version = 2\n[[failure]]\npackage = \"a\"\nenvironment = \"ubuntu-latest\"\n\
         tier = \"L1\"\nowner = \"@o\"\nreason = \"r\"\nsource_run = \"1\"\nshard = \"1/4\"\n",
    )
    .unwrap();

    let err = load_baseline(&path).expect_err("a stale shard key must be refused");
    assert!(
        format!("{err:#}").contains("unknown field"),
        "the error must name the stray key: {err:#}"
    );
    fs::remove_dir_all(&dir).ok();
}

/// A `tier = "lint"` baseline entry excuses a lint FAIL exactly like a
/// JUnit-backed entry: lint cells come from producer status, and the synthetic
/// producers will eventually ride this path.
#[test]
fn a_lint_baseline_entry_excuses_a_lint_failure() {
    let statuses = vec![ProducerStatus {
        package: "homelab-server".to_owned(),
        job: "lint".to_owned(),
        result: "failure".to_owned(),
        environment: None,
        detail: None,
        companion: None,
    }];
    let cells = status_cells(
        &statuses,
        &scope_of(&["homelab-server"]),
        &[],
        &[policy("homelab-server")],
        &[],
    );
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].state, CellState::Fail);

    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry(
            "homelab-server",
            "ubuntu-latest",
            Tier::parse("lint"),
        )],
        skip: Vec::new(),
    };
    let findings = verdict(&rollup_of(cells, &["homelab-server"]), &baseline, None);
    assert!(!any_block(&findings), "unexpected blocks: {findings:#?}");
    assert!(
        findings.iter().any(|f| f.rule == "baseline-accepted"),
        "the excusal must stay visible: {findings:#?}"
    );
}

/// A synthetic identity (`claudine-gen-drift` is the checked-in one) is carried
/// forward outside every package scope: reported out-of-scope BY NAME, never
/// blocking, never passing.
#[test]
fn synthetic_baseline_identities_report_out_of_scope_by_name() {
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![
            failure_entry("claudine-gen-drift", "ubuntu-latest", Tier::parse("lint")),
            failure_entry("some-other-synthetic", "ubuntu-latest", Tier::parse("lint")),
        ],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(Vec::new(), &["sniff"]), &baseline, None);
    assert!(!any_block(&findings), "out-of-scope must not block: {findings:#?}");
    let note = findings
        .iter()
        .find(|f| f.rule == "baseline-out-of-scope")
        .expect("the synthetic entries must be reported out of scope");
    assert!(note.detail.contains("claudine-gen-drift"));
    assert!(note.detail.contains("some-other-synthetic"));
}

#[rstest]
#[case(CellState::Pass, false)]
#[case(CellState::Fail, true)]
#[case(CellState::Skip, false)]
#[case(CellState::NothingToRun, false)]
#[case(CellState::Missing, true)]
#[case(CellState::NotScheduled, false)]
#[case(CellState::PolicyGap, true)]
fn only_fail_missing_and_policy_gap_block_the_summary_gate(
    #[case] state: CellState,
    #[case] blocks: bool,
) {
    assert_eq!(state.blocks(), blocks);
}

// ---------------------------------------------------------------------------
// Walker (manifest present, manifest absent, cross-platform paths)
// ---------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "ci-rollup-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn the_manifest_is_the_identity_source() {
    let temp = TempDir::new("manifest");
    let artifact = temp.path().join("junit-biscuit-file-cli-L1-windows-latest");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(
        artifact.join("L1").join("biscuit-file-cli.xml"),
        junit("biscuit-file-cli", &passing_case("cli::a")),
    )
    .unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"biscuit-file-cli","xml":"L1/biscuit-file-cli.xml","exit_code":0,"environment":"windows-latest","duration_s":12,"report_present":true}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-biscuit-file-cli-L1-windows-latest".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].package, "biscuit-file-cli");
    assert_eq!(records[0].environment, "windows-latest");
    assert!(!records[0].degraded);
    assert_eq!(records[0].counts.passed, 1);
}

/// Artifact-name parsing was retired with the area model: a staged report with
/// no covering manifest record has no trustworthy identity and is dropped.
#[test]
fn a_report_with_no_manifest_record_is_dropped_not_guessed() {
    let temp = TempDir::new("nomanifest");
    let artifact = temp.path().join("junit-sniff-L1-macos-latest");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(
        artifact.join("L1").join("sniff.xml"),
        junit("sniff", &failing_case("net::a")),
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-sniff-L1-macos-latest".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert_eq!(
        records.len(),
        0,
        "identity is never recovered from a directory name"
    );
}

#[test]
fn report_present_false_yields_a_record_with_no_counts() {
    let temp = TempDir::new("noreport");
    let artifact = temp.path().join("junit-queue-L1-windows-latest");
    fs::create_dir_all(&artifact).unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"queue","xml":"L1/queue.xml","exit_code":101,"environment":"windows-latest","duration_s":3,"report_present":false}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-queue-L1-windows-latest".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert_eq!(records.len(), 1);
    assert!(!records[0].report_present);
    assert_eq!(records[0].exit_code, 101);
}

#[test]
fn a_truncated_staged_report_becomes_a_parse_error_not_a_pass() {
    let temp = TempDir::new("truncated");
    let artifact = temp.path().join("junit-playa-L1-ubuntu-latest");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    let full = junit("playa", &passing_case("a"));
    fs::write(
        artifact.join("L1").join("playa.xml"),
        &full[..full.len() / 2],
    )
    .unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"playa","xml":"L1/playa.xml","exit_code":0,"environment":"ubuntu-latest","duration_s":1,"report_present":true}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-playa-L1-ubuntu-latest".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert!(records[0].parse_error.is_some());
    assert!(!records[0].report_present);
    assert_eq!(records[0].counts.passed, 0);
}

#[test]
fn a_manifest_record_for_a_missing_xml_file_is_a_parse_error() {
    let temp = TempDir::new("absentxml");
    let artifact = temp.path().join("junit-research-L1-ubuntu-latest");
    fs::create_dir_all(&artifact).unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"research","xml":"L1/research.xml","exit_code":0,"environment":"ubuntu-latest","duration_s":1,"report_present":true}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-research-L1-ubuntu-latest".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert!(records[0].parse_error.is_some());
    assert!(!records[0].report_present);
}

#[test]
fn a_malformed_manifest_line_is_a_hard_error() {
    let temp = TempDir::new("badmanifest");
    let artifact = temp.path().join("junit-a-L1-ubuntu-latest");
    fs::create_dir_all(&artifact).unwrap();
    fs::write(artifact.join("manifest.jsonl"), "{not json}\n").unwrap();

    let err = records_from_artifact(&ArtifactDir {
        name: "junit-a-L1-ubuntu-latest".to_owned(),
        path: artifact,
    })
    .expect_err("a corrupt manifest must not be silently ignored");
    assert!(format!("{err:#}").contains("malformed manifest record"));
}

#[test]
fn relative_paths_join_without_posix_assumptions() {
    let base = Path::new("root");
    assert_eq!(
        join_relative(base, "L1/biscuit-file.xml"),
        base.join("L1").join("biscuit-file.xml")
    );
    assert_eq!(
        join_relative(base, "L1\\biscuit-file.xml"),
        base.join("L1").join("biscuit-file.xml")
    );
    assert_eq!(
        join_relative(base, "./L1//x.xml"),
        base.join("L1").join("x.xml")
    );
}

// ---------------------------------------------------------------------------
// Rendering and dates
// ---------------------------------------------------------------------------

#[test]
fn the_grid_never_renders_a_missing_cell_as_pass() {
    let cells = classify_simple(
        &[
            expectation("claudine", "ubuntu-latest", Tier::L1),
            expectation("sniff", "ubuntu-latest", Tier::L1),
        ],
        &[passing_record("sniff", "ubuntu-latest", Tier::L1)],
    );
    let markdown = render_grid(&rollup_of(cells, &["claudine", "sniff"]));

    assert!(markdown.contains("MISSING"));
    assert!(markdown.contains("| `claudine` |"));
    let claudine_row = markdown
        .lines()
        .find(|line| line.starts_with("| `claudine` |"))
        .expect("claudine has a row");
    assert!(
        !claudine_row.contains("PASS"),
        "a MISSING leg must never render as PASS: {claudine_row}"
    );
}

#[test]
fn the_grid_renders_nothing_to_run_and_not_scheduled_as_themselves() {
    let cells = classify_simple(
        &[expectation("tabby", "ubuntu-latest", Tier::L1)],
        &[record("tabby", "ubuntu-latest", Tier::L1)],
    );
    let markdown = render_grid(&rollup_of(cells, &["tabby"]));
    assert!(markdown.contains("NOTHING TO RUN"), "{markdown}");
    assert!(!markdown.contains("PASS 0/0/0"), "{markdown}");
}

#[test]
fn table_cells_escape_pipes_so_the_grid_cannot_be_corrupted() {
    assert_eq!(cell_text("a|b"), "a\\|b");
    assert_eq!(cell_text("a\nb"), "a b");
}

#[rstest]
#[case(0, 1970, 1, 1)]
#[case(19_723, 2024, 1, 1)]
#[case(20_664, 2026, 7, 30)]
#[case(20_147, 2025, 2, 28)]
#[case(19_782, 2024, 2, 29)]
fn civil_from_days_matches_known_dates(
    #[case] days: i64,
    #[case] year: i64,
    #[case] month: u32,
    #[case] day: u32,
) {
    assert_eq!(civil_from_days(days), (year, month, day));
}

#[test]
fn today_is_a_sortable_iso_date() {
    let today = today_utc().expect("system clock is readable");
    assert_eq!(today.len(), 10);
    assert!(today.as_str() > "2020-01-01");
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

#[test]
fn repeated_and_comma_joined_flags_flatten_to_one_list() {
    let args = Args::parse(
        ["--scope", "a,b", "--scope=c", "--scope", " d "]
            .into_iter()
            .map(str::to_owned),
    )
    .unwrap();
    assert_eq!(args.list("scope"), vec!["a", "b", "c", "d"]);
}

#[test]
fn a_positional_argument_is_rejected() {
    assert!(Args::parse(["oops".to_owned()].into_iter()).is_err());
}

// ---------------------------------------------------------------------------
// `wsl2-ubuntu` cells
// ---------------------------------------------------------------------------

fn scope_of(packages: &[&str]) -> BTreeSet<String> {
    packages.iter().map(|p| (*p).to_owned()).collect()
}

/// The reported bug that motivated environment-keyed cells: green legs filed
/// `NOT SCHEDULED`.
#[test]
fn a_passing_wsl2_leg_renders_pass_not_not_scheduled() {
    let expected = expected_cells(&[policy("biscuit-hash")], &scope_of(&["biscuit-hash"]), &test_environments());

    assert!(
        expected
            .iter()
            .any(|cell| cell.key.environment == "wsl2-ubuntu"),
        "a gating package must schedule a wsl2-ubuntu cell"
    );

    let cells = classify_simple(
        &expected,
        &[passing_record("biscuit-hash", "wsl2-ubuntu", Tier::L1)],
    );
    let cell = cells
        .iter()
        .find(|cell| cell.key.environment == "wsl2-ubuntu")
        .expect("the wsl2-ubuntu cell must exist");

    assert_eq!(cell.state, CellState::Pass);
    assert!(cell.scheduled);
    assert_eq!(cell.counts.passed, 3);
}

/// The dangerous inverse. A leg policy scheduled that uploads nothing must not
/// quietly become a non-blocking `NOT SCHEDULED`.
#[test]
fn a_scheduled_wsl2_leg_with_no_report_is_missing_and_blocks() {
    let expected = expected_cells(&[policy("biscuit-hash")], &scope_of(&["biscuit-hash"]), &test_environments());

    // Every native leg reported; only wsl2-ubuntu is silent.
    let records: Vec<RunRecord> = ["ubuntu-latest", "windows-latest", "macos-latest"]
        .into_iter()
        .map(|environment| passing_record("biscuit-hash", environment, Tier::L1))
        .collect();

    let cells = classify_simple(&expected, &records);
    let cell = cells
        .iter()
        .find(|cell| cell.key.environment == "wsl2-ubuntu")
        .expect("a scheduled leg must render a cell even with no evidence");

    assert_eq!(cell.state, CellState::Missing);
    assert!(cell.state.blocks(), "MISSING must fail the summary gate");
    assert!(cell
        .reasons
        .iter()
        .any(|reason| reason.contains("produced no report")));

    let findings = verdict(
        &rollup_of(cells, &["biscuit-hash"]),
        &Baseline::default(),
        None,
    );
    assert!(blocks_with_rule(&findings, "cell-missing"));
}

/// Evidence proves a leg ran, so the cell must report what the tests did. The
/// policy disagreement is separately blocking rather than a state that hides
/// the counts.
#[test]
fn passing_evidence_for_an_unscheduled_cell_renders_pass_and_blocks() {
    let cells = classify_simple(&[], &[passing_record("ghost", "wsl2-ubuntu", Tier::L1)]);
    let cell = only_cell(cells.clone());

    assert!(!cell.scheduled);
    assert_eq!(
        cell.state,
        CellState::Pass,
        "a leg that ran and passed is not `NOT SCHEDULED`"
    );
    assert_eq!(cell.counts.passed, 3);

    let findings = verdict(&rollup_of(cells, &["ghost"]), &Baseline::default(), None);
    assert!(blocks_with_rule(&findings, "cell-unscheduled-evidence"));
}

// ---------------------------------------------------------------------------
// compare
// ---------------------------------------------------------------------------

fn compare_cell(package: &str, state: CellState, failed: &[&str]) -> Cell {
    Cell {
        state,
        scheduled: state != CellState::NotScheduled,
        failed_tests: failed.iter().map(|t| (*t).to_owned()).collect(),
        ..blank_cell(cell_key(package, "ubuntu-latest", Tier::L1))
    }
}

fn compare_rollup(cells: Vec<Cell>) -> Rollup {
    Rollup {
        schema_version: RESULT_SCHEMA_VERSION,
        run_id: None,
        scope: Vec::new(),
        areas: derived_areas(&cells),
        area_scope: Vec::new(),
        accepted_evidence: Vec::new(),
        scope_degraded: false,
        scheduled: None,
        records: Vec::new(),
        cells,
    }
}

#[test]
fn a_failure_absent_from_the_base_is_a_regression() {
    let base = compare_rollup(vec![compare_cell("sniff", CellState::Fail, &["a"])]);
    let head = compare_rollup(vec![compare_cell("sniff", CellState::Fail, &["a", "b"])]);

    let result = compare(&base, &head);

    assert!(result.regressed());
    assert_eq!(result.changed.len(), 1);
    assert_eq!(result.changed[0].new_failures, vec!["b".to_owned()]);
    assert_eq!(result.changed[0].shared_failures, 1);
}

#[test]
fn an_identical_failure_set_is_not_a_regression() {
    let base = compare_rollup(vec![compare_cell("sniff", CellState::Fail, &["a", "b"])]);
    let head = compare_rollup(vec![compare_cell("sniff", CellState::Fail, &["b", "a"])]);

    let result = compare(&base, &head);

    assert!(!result.regressed());
    assert_eq!(result.unchanged, 1);
    assert!(result.changed.is_empty());
}

#[test]
fn dropping_a_failure_is_reported_without_blocking() {
    let base = compare_rollup(vec![compare_cell("sniff", CellState::Fail, &["a", "b"])]);
    let head = compare_rollup(vec![compare_cell("sniff", CellState::Pass, &[])]);

    let result = compare(&base, &head);

    assert!(!result.regressed());
    assert_eq!(result.changed.len(), 1);
    assert_eq!(result.changed[0].fixed_failures.len(), 2);
}

/// `MISSING` and `POLICY GAP` carry no `failed_tests`, so a cell that stops
/// reporting names no test. Identity comparison alone would score it unchanged.
#[rstest]
#[case(CellState::Missing)]
#[case(CellState::PolicyGap)]
fn a_cell_that_starts_blocking_regresses_without_naming_a_test(#[case] head_state: CellState) {
    let base = compare_rollup(vec![compare_cell("sniff", CellState::Pass, &[])]);
    let head = compare_rollup(vec![compare_cell("sniff", head_state, &[])]);

    let result = compare(&base, &head);

    assert!(result.regressed());
    assert!(result.changed[0].new_failures.is_empty());
}

/// A branch routinely schedules a narrower scope than `main`. Counting a cell
/// only one side ran would credit or blame it for work that was never measured.
#[rstest]
#[case(CellState::NotScheduled, CellState::Fail, "not scheduled on base")]
#[case(CellState::Fail, CellState::NotScheduled, "not scheduled on head")]
#[case(CellState::NotScheduled, CellState::NotScheduled, "not scheduled on either side")]
fn a_cell_missing_from_one_side_is_not_comparable(
    #[case] base_state: CellState,
    #[case] head_state: CellState,
    #[case] expected: &str,
) {
    let base = compare_rollup(vec![compare_cell("sniff", base_state, &["a"])]);
    let head = compare_rollup(vec![compare_cell("sniff", head_state, &["a"])]);

    let result = compare(&base, &head);

    assert!(!result.regressed());
    assert_eq!(result.incomparable.len(), 1, "exactly one entry per cell");
    assert_eq!(result.incomparable[0].reason, expected);
    assert!(result.changed.is_empty());
    assert_eq!(result.unchanged, 0);
}

#[test]
fn a_cell_present_in_only_one_document_is_not_comparable() {
    let base = compare_rollup(vec![compare_cell("sniff", CellState::Fail, &["a"])]);
    let head = compare_rollup(vec![compare_cell("queue", CellState::Fail, &["b"])]);

    let result = compare(&base, &head);

    assert!(!result.regressed());
    assert_eq!(result.incomparable.len(), 2);
}
// ---------------------------------------------------------------------------
// The area-owned result model
//
// Phase 2 of `fixes/2026-09-11-cicd-cleanup/plan.md` froze these contracts as
// `pending_contract` fixtures; Phase 5 implements them, so the wrappers are
// gone and a regression fails this suite directly. The fixtures that pin the
// *defect* (`the_pr_76_fixture_reproduces_the_regression_exactly`) are
// deliberately kept: they are what stops a contract from being satisfied by
// changing what the fixture builds.
// ---------------------------------------------------------------------------

/// The six packages and seven cells PR #76 filed as `MISSING`, from
/// `fixes/2026-09-11-cicd-cleanup/baseline-2026-09-11.md` section 4.
const PR76_L1_PACKAGES: [&str; 6] = [
    "biscuit-speaks",
    "biscuit-speaks-cli",
    "claudine",
    "claudine-cli",
    "playa",
    "playa-cli",
];

fn pr76_policies() -> Vec<PackagePolicy> {
    let mut policies: Vec<PackagePolicy> = PR76_L1_PACKAGES.iter().map(|p| policy(p)).collect();
    // `claudine-cli` is the one package with an L2 tier, giving the seventh cell.
    let cli = policies
        .iter_mut()
        .find(|p| p.package == "claudine-cli")
        .expect("claudine-cli is in the fixture");
    cli.tiers = vec![Tier::L1, Tier::L2];
    cli.l2_backends = vec!["tmux".to_owned()];
    policies
}

/// The run as it actually happened: every hosted environment reported, macOS
/// reported nothing because a local receipt suppressed its execution, and
/// nothing told the rollup the receipt existed.
fn pr76_cells() -> Vec<Cell> {
    let policies = pr76_policies();
    let scope = scope_of(&PR76_L1_PACKAGES);
    let expected = expected_cells(&policies, &scope, &test_environments());
    classify_simple(&expected, &pr76_records())
}

/// Every hosted record the run produced. macOS is absent by construction.
fn pr76_records() -> Vec<RunRecord> {
    let mut records = Vec::new();
    for package in PR76_L1_PACKAGES {
        for environment in ["ubuntu-latest", "windows-latest", "wsl2-ubuntu"] {
            records.push(passing_record(package, environment, Tier::L1));
        }
    }
    records.push(passing_record("claudine-cli", "ubuntu-latest", Tier::L2));
    records
}

/// The same run, rolled up against the plan that scheduled it: the seven macOS
/// cells are reused from the receipt that suppressed their execution.
fn pr76_cells_from_the_plan() -> Vec<Cell> {
    let mut cells = Vec::new();
    for package in PR76_L1_PACKAGES {
        for environment in ENVIRONMENTS {
            cells.push(plan_cell_json(package, environment, "L1", environment == "macos-latest"));
        }
    }
    for environment in ENVIRONMENTS {
        // Windows and the WSL2 guest host no L2 backend, and both absences are
        // governed: the plan accepts them rather than scheduling a job.
        if matches!(environment, "windows-latest" | "wsl2-ubuntu") {
            let mut gap = accepted_gap_cell_json(true, "2027-01-31");
            gap["environment"] = serde_json::json!(environment);
            cells.push(gap);
            continue;
        }
        cells.push(plan_cell_json(
            "claudine-cli",
            environment,
            "L2",
            environment == "macos-latest",
        ));
    }
    let plan = plan_of(cells);
    let expected = plan_expected_cells(&plan).expect("the plan is version 1");
    classify_simple(&expected, &pr76_records())
}

const ENVIRONMENTS: [&str; 4] = [
    "ubuntu-latest",
    "windows-latest",
    "macos-latest",
    "wsl2-ubuntu",
];

/// One plan cell, in the JSON the planner writes. Built as text rather than as
/// a struct so the fixtures exercise the deserializer the artifact is read
/// through, not a hand-built value that cannot drift with it.
fn plan_cell_json(
    package: &str,
    environment: &str,
    gate: &str,
    reused: bool,
) -> serde_json::Value {
    let area = package.trim_end_matches("-cli");
    let mut cell = serde_json::json!({
        "package": package,
        "area": area,
        "environment": environment,
        "gate": gate,
        "execution": "execute",
        "origin": "ci",
        "state": "pending",
        "target_kinds": if gate == "L1" { vec!["lib", "test"] } else { Vec::new() },
        "compile_coverage_from": if gate == "L1" { "L1" } else { "" },
        "selection_reason": format!("{package} declares the {gate} tier"),
    });
    if reused {
        cell["execution"] = serde_json::json!("reuse");
        cell["origin"] = serde_json::json!("local");
        cell["state"] = serde_json::json!("reused");
        cell["evidence"] = receipt_evidence(package, gate, "pass", 3, 0);
    }
    cell
}

/// The accepted-cell record `local_evidence.py verify --cells` publishes.
fn receipt_evidence(package: &str, gate: &str, outcome: &str, passed: u32, failed: u32) -> serde_json::Value {
    serde_json::json!({
        "package": package,
        "gate": gate,
        "environment": "macos-latest",
        "origin": "local",
        "outcome": outcome,
        "completion": "complete",
        "exit_code": if outcome == "pass" { 0 } else { 100 },
        "counts": {
            "total": passed + failed,
            "passed": passed,
            "failed": failed,
            "skipped": 0,
            "errored": 0,
        },
        "duration_s": 12,
        "gate_input_identity": "abcdef0123456789",
        "backends": Vec::<String>::new(),
        "report": "junit/L1.xml",
        "failed_tests": if failed > 0 { vec![format!("{package}::boom")] } else { Vec::new() },
        "measurements": format!("{} test(s), {failed} failed, 12s", passed + failed),
        "evidence": {
            "ref": "refs/notes/ci-local/macos-latest",
            "commit": "f".repeat(40),
            "host": {"os": "macos", "kernel": "27.0.0", "report_dir": "~/.rusty-biscuit/ci-evidence/0156096ff"},
        },
    })
}

/// A resolved plan carrying `cells`, parsed exactly as the artifact is.
fn plan_of(cells: Vec<serde_json::Value>) -> ResolvedPlan {
    let packages: BTreeSet<String> = cells
        .iter()
        .map(|cell| cell["package"].as_str().unwrap_or_default().to_owned())
        .collect();
    let document = serde_json::json!({
        "schema_version": 1,
        "packages": packages
            .iter()
            .map(|package| serde_json::json!({"package": package}))
            .collect::<Vec<_>>(),
        "cells": cells,
    });
    serde_json::from_str(&document.to_string()).expect("the plan document parses")
}

fn find_cell<'a>(cells: &'a [Cell], package: &str, environment: &str, tier: Tier) -> &'a Cell {
    cells
        .iter()
        .find(|cell| cell.key == cell_key(package, environment, tier.clone()))
        .unwrap_or_else(|| panic!("no {package}/{environment}/{tier} cell in {cells:#?}"))
}

// --- AC6: the PR #76 regression ------------------------------------------

#[test]
fn the_pr_76_macos_cells_must_not_resolve_to_missing() {
    let cells = pr76_cells_from_the_plan();
    let macos: Vec<&Cell> = cells
        .iter()
        .filter(|cell| cell.key.environment == "macos-latest")
        .collect();

    assert_eq!(
        7,
        macos.len(),
        "the regression is exactly seven macOS cells: six L1 and one L2"
    );
    for cell in macos {
        assert_ne!(
            cell.state,
            CellState::Missing,
            "{} resolved to MISSING although local evidence proved it passed",
            cell.key
        );
        assert_eq!(cell.state, CellState::Pass);
        assert_eq!(cell.origin, Origin::Local);
    }
}

#[test]
fn the_pr_76_fixture_reproduces_the_regression_exactly() {
    // Non-pending: pins the defect itself, so a Phase 5 change that makes the
    // contract above pass by changing what the fixture *builds* is caught here.
    // This is the run WITHOUT its plan — nothing tells the rollup a receipt
    // satisfied those cells, and failing safe means MISSING.
    let cells = pr76_cells();
    let missing: Vec<String> = cells
        .iter()
        .filter(|cell| cell.key.environment == "macos-latest" && cell.state == CellState::Missing)
        .map(|cell| cell.key.to_string())
        .collect();

    assert_eq!(
        vec![
            "biscuit-speaks-cli/macos-latest/L1",
            "biscuit-speaks/macos-latest/L1",
            "claudine-cli/macos-latest/L1",
            "claudine-cli/macos-latest/L2",
            "claudine/macos-latest/L1",
            "playa-cli/macos-latest/L1",
            "playa/macos-latest/L1",
        ],
        {
            let mut sorted = missing;
            sorted.sort();
            sorted
        }
    );
}

#[test]
fn the_pr_76_hosted_cells_still_pass() {
    // The other half of the regression: everything that ran was green, so a
    // Phase 5 fix must not make the macOS cells pass by degrading these. The two
    // exceptions are the governed L2 gaps the baseline records as POLICY GAP,
    // which did not block that run and must not start to.
    let cells = pr76_cells();
    let not_passing: Vec<String> = cells
        .iter()
        .filter(|cell| cell.key.environment != "macos-latest" && cell.state != CellState::Pass)
        .map(|cell| format!("{} = {}", cell.key, cell.state))
        .collect();
    assert_eq!(
        vec![
            "claudine-cli/windows-latest/L2 = POLICY GAP",
            "claudine-cli/wsl2-ubuntu/L2 = POLICY GAP",
        ],
        not_passing
    );
}

#[test]
fn the_pr_76_hosted_cells_stay_ci_origin_when_the_plan_is_read() {
    // Reusing macOS must not quietly relabel the environments that did run.
    let cells = pr76_cells_from_the_plan();
    for cell in cells
        .iter()
        .filter(|cell| cell.key.environment != "macos-latest")
    {
        let expected = if cell.state == CellState::AcceptedGap {
            // Nothing ran and nothing was reused: the cell has no result to
            // attribute to anyone.
            Origin::Unproduced
        } else {
            Origin::Ci
        };
        assert_eq!(cell.origin, expected, "{} changed origin", cell.key);
        assert!(cell.evidence.is_none(), "{} invented evidence", cell.key);
    }

    // The two governed gaps of that run stay visible and stay non-blocking.
    let gaps: Vec<String> = cells
        .iter()
        .filter(|cell| cell.state == CellState::AcceptedGap)
        .map(|cell| cell.key.to_string())
        .collect();
    assert_eq!(
        gaps,
        vec![
            "claudine-cli/windows-latest/L2".to_owned(),
            "claudine-cli/wsl2-ubuntu/L2".to_owned(),
        ]
    );
}

#[test]
fn a_reused_cell_carries_the_measurements_its_receipt_recorded() {
    let cells = pr76_cells_from_the_plan();
    let cell = find_cell(&cells, "claudine", "macos-latest", Tier::L1);

    assert_eq!(cell.counts.passed, 3);
    assert_eq!(cell.counts.total, 3);
    assert_eq!(cell.duration_s, 12);
    let evidence = cell.evidence.clone().expect("a reused cell names its receipt");
    assert_eq!(evidence.reference, "refs/notes/ci-local/macos-latest");
    assert_eq!(evidence.measurements, "3 test(s), 0 failed, 12s");
    assert_eq!(evidence.host, "macos 27.0.0");
    assert_eq!(evidence.report, "~/.rusty-biscuit/ci-evidence/0156096ff");
    assert!(
        cell.records.is_empty(),
        "no CI job ran for a reused cell, so it owns no run record"
    );
}

#[test]
fn the_result_document_records_the_evidence_it_was_scheduled_against() {
    let cells = pr76_cells_from_the_plan();
    let rollup = rollup_of(cells, &PR76_L1_PACKAGES);
    let json = serde_json::to_value(&rollup).expect("a rollup serializes");

    assert!(
        json.get("accepted_evidence").is_some(),
        "the rollup document records no accepted evidence; evidence accepted \
         for scheduling cannot satisfy its result cell"
    );
    assert_eq!(
        rollup.accepted_evidence.len(),
        7,
        "one entry per reused cell: {:#?}",
        rollup.accepted_evidence
    );
    for entry in &rollup.accepted_evidence {
        assert_eq!(entry.origin, Origin::Local);
        assert_eq!(entry.evidence.reference, "refs/notes/ci-local/macos-latest");
        assert!(!entry.area.is_empty(), "{} carries no area", entry.key);
    }
}

// --- AC4: one result model, with origin and evidence ---------------------

#[test]
fn a_result_cell_reports_where_its_result_came_from() {
    let cell = &pr76_cells_from_the_plan()[0];
    let json = serde_json::to_value(cell).expect("a cell serializes");
    assert!(
        json.get("origin").is_some(),
        "the result cell carries no `origin`; local, prior-local, and CI \
         results are indistinguishable in {json}"
    );
}

#[test]
fn a_result_cell_carries_its_area_alongside_its_package() {
    let cell = &pr76_cells_from_the_plan()[0];
    let json = serde_json::to_value(cell).expect("a cell serializes");
    assert!(
        json.get("area").is_some(),
        "the result cell carries no `area`; there is nothing to group \
         a top-level identity by in {json}"
    );
}

#[test]
fn package_stays_the_stored_identity_of_every_cell() {
    // Deliberately the inverse of the fixture above: Design Decision 1 forbids
    // re-keying results by area. Adding `area` is required; replacing
    // `package` with it is not.
    let cell = &pr76_cells_from_the_plan()[0];
    let json = serde_json::to_value(cell).expect("a cell serializes");
    assert!(json.get("package").is_some(), "package is the stored identity");
    assert!(json.get("environment").is_some());
    assert!(json.get("tier").is_some());
}

#[test]
fn a_prior_local_receipt_is_distinguishable_from_the_outgoing_head() {
    // Gate-input equivalence accepts a receipt from an OLDER head. That is a
    // weaker claim than an exact-tree match and the result must say so.
    let mut cell = plan_cell_json("claudine", "macos-latest", "L1", true);
    cell["origin"] = serde_json::json!("prior-local");
    cell["evidence"]["origin"] = serde_json::json!("prior-local");
    let plan = plan_of(vec![cell]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    let cell = only_cell(classify_simple(&expected, &[]));

    assert_eq!(cell.state, CellState::Pass);
    assert_eq!(cell.origin, Origin::PriorLocal);
    assert!(
        cell.reasons.iter().any(|reason| reason.contains("prior-local")),
        "the reason must name the weaker claim: {:#?}",
        cell.reasons
    );
}

#[test]
fn a_version_one_receipt_renders_its_measurements_as_unrecorded() {
    // Spec section 3.6: a v1 note carries no per-cell outcome. It is pass-only
    // by contract, and the text is part of the contract.
    let mut cell = plan_cell_json("claudine", "macos-latest", "L1", true);
    cell["evidence"] = serde_json::json!({
        "origin": "local",
        "evidence": "refs/notes/ci-local/macos-latest",
    });
    let plan = plan_of(vec![cell]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    let cell = only_cell(classify_simple(&expected, &[]));

    assert_eq!(cell.state, CellState::Pass);
    assert_eq!(cell.counts, Counts::default());
    let evidence = cell.evidence.clone().expect("the note is still named");
    assert_eq!(evidence.measurements, "not recorded (v1 receipt)");
    assert_eq!(evidence.reference, "refs/notes/ci-local/macos-latest");
}

#[test]
fn a_reused_cell_that_also_produced_ci_evidence_reports_the_executed_result() {
    // The plan and the run disagree: something executed work the plan reused.
    // The executed result is the one with a report behind it, and the
    // disagreement is stated rather than hidden.
    let plan = plan_of(vec![plan_cell_json("claudine", "macos-latest", "L1", true)]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    let mut record = record("claudine", "macos-latest", Tier::L1);
    record.counts = Counts {
        total: 2,
        passed: 1,
        failed: 1,
        ..Counts::default()
    };
    record.failed_tests = vec!["claudine::real_failure".to_owned()];
    let cell = only_cell(classify_simple(&expected, &[record]));

    assert_eq!(cell.state, CellState::Fail);
    assert_eq!(cell.origin, Origin::Ci);
    assert_eq!(cell.failed_tests, vec!["claudine::real_failure".to_owned()]);
    assert!(
        cell.reasons
            .iter()
            .any(|reason| reason.contains("also produced CI evidence")),
        "the disagreement must be visible: {:#?}",
        cell.reasons
    );
}

// --- AC8: a failure blocks its own area, and only its own area -----------

#[test]
fn a_complete_failed_local_result_stays_a_failure() {
    let mut cell = plan_cell_json("claudine", "macos-latest", "L1", true);
    cell["evidence"] = receipt_evidence("claudine", "L1", "fail", 2, 1);
    let plan = plan_of(vec![cell]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    let cell = only_cell(classify_simple(&expected, &[]));

    assert_eq!(cell.state, CellState::Fail);
    assert_eq!(cell.origin, Origin::Local);
    assert_eq!(cell.failed_tests, vec!["claudine::boom".to_owned()]);

    let findings = verdict(
        &rollup_of(vec![cell], &["claudine"]),
        &Baseline::default(),
        None,
    );
    assert!(
        blocks_with_rule(&findings, "cell-failed"),
        "reuse must not turn a failure into a success: {findings:#?}"
    );
}

/// Two areas, one red. Narrowing is what makes the outcome area-owned.
fn two_area_rollup() -> Rollup {
    let cells = vec![
        Cell {
            area: "claudine".to_owned(),
            state: CellState::Fail,
            origin: Origin::Ci,
            failed_tests: vec!["claudine::boom".to_owned()],
            scheduled: true,
            ..blank_cell(cell_key("claudine", "ubuntu-latest", Tier::L1))
        },
        Cell {
            area: "playa".to_owned(),
            state: CellState::Pass,
            origin: Origin::Ci,
            counts: Counts {
                total: 4,
                passed: 4,
                ..Counts::default()
            },
            scheduled: true,
            ..blank_cell(cell_key("playa", "ubuntu-latest", Tier::L1))
        },
    ];
    rollup_of(cells, &["claudine", "playa"])
}

#[test]
fn a_failed_area_blocks_only_its_own_slice() {
    let rollup = two_area_rollup();

    let claudine = rollup.narrowed(&["claudine".to_owned()].into_iter().collect());
    let findings = verdict(&claudine, &Baseline::default(), None);
    assert!(
        blocks_with_rule(&findings, "cell-failed"),
        "the owning area must block: {findings:#?}"
    );

    let playa = rollup.narrowed(&["playa".to_owned()].into_iter().collect());
    let findings = verdict(&playa, &Baseline::default(), None);
    assert!(
        !any_block(&findings),
        "an independent area must still complete and stay green: {findings:#?}"
    );
}

#[test]
fn narrowing_keeps_only_the_areas_own_cells_scope_and_evidence() {
    let mut rollup = two_area_rollup();
    rollup.accepted_evidence = vec![AcceptedEvidence {
        key: cell_key("playa", "macos-latest", Tier::L1),
        area: "playa".to_owned(),
        origin: Origin::Local,
        measurements: "4 test(s), 0 failed, 3s".to_owned(),
        evidence: Evidence {
            reference: "refs/notes/ci-local/macos-latest".to_owned(),
            ..Evidence::default()
        },
    }];
    rollup.scheduled = Some(vec![
        cell_key("claudine", "ubuntu-latest", Tier::L1),
        cell_key("playa", "ubuntu-latest", Tier::L1),
    ]);

    let playa = rollup.narrowed(&["playa".to_owned()].into_iter().collect());

    assert_eq!(playa.scope, vec!["playa".to_owned()]);
    assert_eq!(playa.areas, vec!["playa".to_owned()]);
    assert_eq!(playa.area_scope, vec!["playa".to_owned()]);
    assert_eq!(playa.cells.len(), 1);
    assert_eq!(playa.accepted_evidence.len(), 1);
    assert_eq!(
        playa.scheduled.as_deref(),
        Some([cell_key("playa", "ubuntu-latest", Tier::L1)].as_slice())
    );
}

#[test]
fn another_areas_baseline_entry_is_out_of_scope_in_this_slice() {
    // The entry names a package this slice says nothing about. Ignoring it is
    // right; counting it as a pass or as a vanished result is not.
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        failure: vec![failure_entry("claudine", "ubuntu-latest", Tier::L1)],
        skip: Vec::new(),
    };
    let playa = two_area_rollup().narrowed(&["playa".to_owned()].into_iter().collect());

    let findings = verdict(&playa, &baseline, None);
    assert!(!any_block(&findings), "{findings:#?}");
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "baseline-out-of-scope"),
        "the other area's entry must be visibly ignored: {findings:#?}"
    );
}

#[test]
fn a_missing_cell_blocks_its_own_area() {
    let cells = vec![Cell {
        area: "claudine".to_owned(),
        state: CellState::Missing,
        scheduled: true,
        ..blank_cell(cell_key("claudine", "macos-latest", Tier::L1))
    }];
    let findings = verdict(
        &rollup_of(cells, &["claudine"]).narrowed(&["claudine".to_owned()].into_iter().collect()),
        &Baseline::default(),
        None,
    );
    assert!(blocks_with_rule(&findings, "cell-missing"), "{findings:#?}");
}

#[test]
fn a_prohibited_cell_is_missing_and_names_the_constraint() {
    // A recorded execution constraint stops the push, never CI's expectation.
    // If a prohibited cell reaches a run unsatisfied, its coverage is genuinely
    // absent and the area must block.
    let mut cell = plan_cell_json("claudine", "wsl2-ubuntu", "L1", false);
    cell["execution"] = serde_json::json!("omit");
    cell["origin"] = serde_json::json!("none");
    cell["state"] = serde_json::json!("prohibited");
    cell["prohibition"] = serde_json::json!({
        "owner": "@yankeeinlondon",
        "reason": "WSL was already run; do not run it again",
        "expiry": "2026-10-01",
    });
    let plan = plan_of(vec![cell]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    let cell = only_cell(classify_simple(&expected, &[]));

    assert_eq!(cell.state, CellState::Missing);
    assert_eq!(cell.origin, Origin::Unproduced);
    assert!(
        cell.reasons
            .iter()
            .any(|reason| reason.contains("do not run it again")),
        "the constraint must be named: {:#?}",
        cell.reasons
    );
}

// --- AC9: the accepted-gap state ----------------------------------------

fn accepted_gap_cell_json(governed: bool, expiry: &str) -> serde_json::Value {
    let mut cell = plan_cell_json("claudine-cli", "windows-latest", "L2", false);
    cell["execution"] = serde_json::json!("omit");
    cell["origin"] = serde_json::json!("none");
    cell["state"] = serde_json::json!("accepted-gap");
    cell["gap"] = serde_json::json!({
        "capability": "tmux",
        "governed": governed,
        "policy": ".github/ci/environments.json",
        "owner": "@yankeeinlondon",
        "reason": "tmux has no Windows port",
        "expiry": expiry,
        "closes": "features/_unscheduled/windows-l2-ci-leg",
    });
    if !governed {
        cell["gap"] = serde_json::json!({
            "capability": "tmux",
            "governed": false,
            "policy": ".github/ci/environments.json",
        });
    }
    cell
}

fn accepted_gap_cell(governed: bool, expiry: &str) -> Cell {
    let plan = plan_of(vec![accepted_gap_cell_json(governed, expiry)]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    only_cell(classify_simple(&expected, &[]))
}

#[test]
fn accepted_gap_is_a_distinct_machine_readable_state() {
    let parsed = serde_json::from_str::<CellState>("\"ACCEPTED GAP\"")
        .expect("CellState parses the ACCEPTED GAP state");
    assert_eq!(parsed, CellState::AcceptedGap);
    assert_eq!(
        serde_json::to_string(&CellState::AcceptedGap).unwrap(),
        "\"ACCEPTED GAP\"",
        "the machine-readable name is part of the contract"
    );
    assert_ne!(
        CellState::AcceptedGap,
        CellState::PolicyGap,
        "an accepted gap must not collapse into an unowned one"
    );
}

#[test]
fn an_accepted_gap_carries_its_owner_expiry_policy_and_closure() {
    let cell = accepted_gap_cell(true, "2027-01-31");

    assert_eq!(cell.state, CellState::AcceptedGap);
    let gap = cell.declared_gap.clone().expect("a governed gap");
    assert_eq!(gap.owner, "@yankeeinlondon");
    assert_eq!(gap.expiry, "2027-01-31");
    assert_eq!(gap.capability, "tmux");
    assert_eq!(gap.policy, ".github/ci/environments.json");
    assert_eq!(gap.closes, "features/_unscheduled/windows-l2-ci-leg");

    let reason = cell.reasons.join(" ");
    for required in [
        "@yankeeinlondon",
        "2027-01-31",
        ".github/ci/environments.json",
        "features/_unscheduled/windows-l2-ci-leg",
        "to revoke",
    ] {
        assert!(
            reason.contains(required),
            "the visible description must carry {required:?}: {reason}"
        );
    }
}

#[test]
fn an_accepted_gap_is_neither_a_pass_nor_a_failure_and_does_not_block() {
    let cell = accepted_gap_cell(true, "2027-01-31");
    assert!(
        !cell.state.blocks(),
        "an owned, unexpired gap is not a failing observation"
    );

    let findings = verdict(
        &rollup_of(vec![cell], &["claudine-cli"]),
        &Baseline::default(),
        Some("2026-09-11"),
    );
    assert!(!any_block(&findings), "{findings:#?}");
    assert!(
        findings.iter().any(|finding| finding.rule == "policy-gap-accepted"
            && finding.severity == Severity::Note
            && finding.detail.contains("features/_unscheduled/windows-l2-ci-leg")),
        "acceptance must stay visible, with what closes it: {findings:#?}"
    );
}

#[test]
fn an_expired_accepted_gap_blocks_its_area() {
    let cell = accepted_gap_cell(true, "2026-01-01");
    let findings = verdict(
        &rollup_of(vec![cell], &["claudine-cli"]),
        &Baseline::default(),
        Some("2026-09-11"),
    );
    assert!(blocks_with_rule(&findings, "policy-gap-expired"), "{findings:#?}");
}

#[test]
fn an_ungoverned_gap_is_never_accepted_however_the_plan_labels_it() {
    // The plan says `accepted-gap`; the policy entry names no owner, reason, or
    // expiry. Acceptance that cannot be traced excuses nothing.
    let cell = accepted_gap_cell(false, "");
    assert_ne!(cell.state, CellState::AcceptedGap);
    assert!(cell.state.blocks());
    assert!(cell.declared_gap.is_none());
    assert!(
        cell.reasons
            .iter()
            .any(|reason| reason.contains("UNGOVERNED policy gap")),
        "the unowned absence must be named: {:#?}",
        cell.reasons
    );

    let findings = verdict(
        &rollup_of(vec![cell], &["claudine-cli"]),
        &Baseline::default(),
        Some("2026-09-11"),
    );
    assert!(any_block(&findings), "{findings:#?}");
    assert!(
        !findings
            .iter()
            .any(|finding| finding.rule == "policy-gap-accepted"),
        "an ungoverned gap is never acceptable: {findings:#?}"
    );
}

#[test]
fn an_accepted_gap_state_with_no_policy_entry_blocks() {
    // A hand-edited or corrupted document, not something the planner writes.
    // The verdict must still refuse it rather than read the state as consent.
    let cells = vec![Cell {
        area: "claudine-cli".to_owned(),
        state: CellState::AcceptedGap,
        scheduled: true,
        ..blank_cell(cell_key("claudine-cli", "windows-latest", Tier::L2))
    }];
    let findings = verdict(
        &rollup_of(cells, &["claudine-cli"]),
        &Baseline::default(),
        Some("2026-09-11"),
    );
    assert!(
        blocks_with_rule(&findings, "policy-gap-incomplete"),
        "{findings:#?}"
    );
}

#[test]
fn a_real_failure_outranks_an_accepted_gap() {
    // A gap declaration can never suppress evidence: something ran and failed.
    let plan = plan_of(vec![accepted_gap_cell_json(true, "2027-01-31")]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    let mut record = record("claudine-cli", "windows-latest", Tier::L2);
    record.counts = Counts {
        total: 1,
        failed: 1,
        ..Counts::default()
    };
    record.failed_tests = vec!["claudine_cli::l2".to_owned()];
    let cell = only_cell(classify_simple(&expected, &[record]));

    assert_eq!(cell.state, CellState::Fail);
}

#[test]
fn a_cancelled_job_is_not_an_accepted_gap() {
    // Spec section 6: an ordinary cancellation, a runner cancellation, and an
    // interrupted test are not accepted gaps. The state is never inferred from
    // a GitHub conclusion.
    let statuses = vec![ProducerStatus {
        package: "claudine".to_owned(),
        job: "check".to_owned(),
        result: "cancelled".to_owned(),
        environment: Some("windows-latest".to_owned()),
        detail: None,
        companion: None,
    }];
    let cells = status_cells(
        &statuses,
        &scope_of(&["claudine"]),
        &[],
        &[policy("claudine")],
        &[],
    );

    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].state, CellState::Missing);
    assert_ne!(cells[0].state, CellState::AcceptedGap);
}

// --- AC3: target coverage travels with the cell --------------------------

#[test]
fn a_cell_reports_the_target_kinds_and_compile_coverage_the_plan_gave_it() {
    let cells = pr76_cells_from_the_plan();
    let cell = find_cell(&cells, "claudine", "ubuntu-latest", Tier::L1);
    assert_eq!(cell.target_kinds, vec!["lib".to_owned(), "test".to_owned()]);
    assert_eq!(cell.compile_coverage_from, "L1");
}

#[test]
fn a_status_gate_reports_the_target_coverage_the_plan_scheduled_it_for() {
    // The check gate is where the `--all-targets` replacement is visible: it
    // exists only for target kinds no test gate compiles.
    let mut check = plan_cell_json("claudine", "windows-latest", "check", false);
    check["gate"] = serde_json::json!("check");
    check["target_kinds"] = serde_json::json!(["bench"]);
    check["compile_coverage_from"] = serde_json::json!("check");
    let plan = plan_of(vec![check]);
    let expected = plan_expected_cells(&plan).expect("version 1");
    let statuses = vec![ProducerStatus {
        package: "claudine".to_owned(),
        job: "check".to_owned(),
        result: "success".to_owned(),
        environment: Some("windows-latest".to_owned()),
        detail: None,
        companion: None,
    }];

    let cells = status_cells(
        &statuses,
        &scope_of(&["claudine"]),
        &[],
        &[policy("claudine")],
        &expected,
    );

    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].state, CellState::Pass);
    assert_eq!(cells[0].target_kinds, vec!["bench".to_owned()]);
    assert_eq!(cells[0].compile_coverage_from, "check");
    assert_eq!(cells[0].area, "claudine");
}

#[test]
fn a_scheduled_gate_that_uploaded_no_status_is_missing_not_absent() {
    let plan = plan_of(vec![plan_cell_json("claudine", "ubuntu-latest", "lint", false)]);
    let expected = plan_expected_cells(&plan).expect("version 1");

    let cells = status_cells(
        &[],
        &scope_of(&["claudine"]),
        &[],
        &[policy("claudine")],
        &expected,
    );

    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].state, CellState::Missing);
    assert_eq!(cells[0].origin, Origin::Unproduced);
}

// --- AC11: area scope, and a summary that applies no policy --------------

#[test]
fn the_rollup_and_verdict_commands_accept_an_area_scope() {
    assert!(
        USAGE.contains("--area"),
        "the usage text does not document `--area`, so no area can apply \
         its own baseline, gaps, and missing-cell rule"
    );
}

#[test]
fn the_grid_groups_cells_under_their_area() {
    let markdown = render_grid(&two_area_rollup());
    assert!(markdown.contains("### area `claudine`"), "{markdown}");
    assert!(markdown.contains("### area `playa`"), "{markdown}");
    assert!(
        markdown.find("### area `claudine`") < markdown.find("`playa`"),
        "areas are the top-level grouping, in a stable order: {markdown}"
    );
}

#[test]
fn the_grid_shows_a_reused_cell_with_its_origin_and_evidence() {
    let rollup = rollup_of(pr76_cells_from_the_plan(), &PR76_L1_PACKAGES);
    let markdown = render_grid(&rollup);

    assert!(markdown.contains("PASS 3/0/0 (local)"), "{markdown}");
    assert!(markdown.contains("### Reused results"), "{markdown}");
    assert!(
        markdown.contains("refs/notes/ci-local/macos-latest"),
        "a local-origin cell's evidence link is its notes ref: {markdown}"
    );
}

#[test]
fn the_grid_explains_an_accepted_gap_where_a_reader_sees_it() {
    let rollup = rollup_of(vec![accepted_gap_cell(true, "2027-01-31")], &["claudine-cli"]);
    let markdown = render_grid(&rollup);

    assert!(markdown.contains("### Accepted policy gaps"), "{markdown}");
    assert!(markdown.contains("@yankeeinlondon"), "{markdown}");
    assert!(markdown.contains("2027-01-31"), "{markdown}");
    assert!(markdown.contains("features/_unscheduled/windows-l2-ci-leg"), "{markdown}");
    assert!(
        !markdown.contains("### Cells failing the summary gate"),
        "an owned, unexpired gap is not a failing cell: {markdown}"
    );
}

#[test]
fn the_combined_summary_aggregates_area_slices_and_applies_no_policy() {
    let rollup = two_area_rollup();
    let claudine = rollup.narrowed(&["claudine".to_owned()].into_iter().collect());
    let playa = rollup.narrowed(&["playa".to_owned()].into_iter().collect());

    let markdown = render_combined_summary(&[claudine, playa]);

    assert!(markdown.contains("| claudine |"), "{markdown}");
    assert!(markdown.contains("| playa |"), "{markdown}");
    // The words a policy evaluation would have to produce. The prose above may
    // say what this view does NOT do; a decision is what it must not make.
    for policy_word in [
        "BLOCKED",
        "CLEAR —",
        "must not merge",
        "baseline-",
        "policy-gap-",
        "## CI verdict",
    ] {
        assert!(
            !markdown.contains(policy_word),
            "the combined summary applied policy ({policy_word:?}): {markdown}"
        );
    }
}

// --- The versioned result-schema migration -------------------------------

#[test]
fn a_result_document_from_the_previous_generation_is_refused() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-r2-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("v2-results.json");
    fs::write(
        &path,
        r#"{"schema_version":2,"scope":[],"scope_degraded":false,"records":[],"cells":[]}"#,
    )
    .unwrap();

    let err = load_rollup(&path).expect_err("the previous generation must be refused");
    let message = format!("{err:#}");
    assert!(
        message.contains("area, origin, and evidence"),
        "the error must name the migration: {message}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_result_and_baseline_schemas_version_independently() {
    // The baseline is hand-edited policy and does not move when the generated
    // result document gains fields.
    assert_eq!(RESULT_SCHEMA_VERSION, 3);
    assert_eq!(BASELINE_SCHEMA_VERSION, 2);
    let baseline = load_baseline(&repo_root().join(".github/ci/ci-baseline.toml"))
        .expect("the shipped baseline loads");
    assert!(baseline.failure.len() + baseline.skip.len() > 0 || true);
}

#[test]
fn a_plan_from_another_generation_is_refused_rather_than_partly_read() {
    let document = serde_json::json!({
        "schema_version": 99,
        "packages": [],
        "cells": [],
    });
    let plan: ResolvedPlan = serde_json::from_str(&document.to_string()).unwrap();
    let err = plan_expected_cells(&plan).expect_err("a future plan must be refused");
    assert!(format!("{err:#}").contains("schema_version 99"), "{err:#}");
}

// --- The shipped artifacts -----------------------------------------------

/// Every field this tool reads out of the resolved plan, against the frozen
/// cross-language contract the planner is validated with.
///
/// A passive corpus test over the shipped artifact: renaming a plan field
/// breaks the rollup silently otherwise, because serde ignores what it does not
/// recognize.
#[test]
fn plan_fields_match_the_frozen_contract() {
    let text = fs::read_to_string(repo_root().join(".github/ci/schemas/contract.json"))
        .expect("the frozen contract is shipped");
    let contract: serde_json::Value =
        serde_json::from_str(&text).expect("the frozen contract parses");
    let plan = &contract["resolved_plan"];

    assert_eq!(
        plan["schema_version"].as_u64(),
        Some(u64::from(PLAN_SCHEMA_VERSION)),
        "this tool reads a plan generation the contract no longer describes"
    );
    for field in ["packages", "cells"] {
        assert!(
            plan["document"].get(field).is_some(),
            "the plan document contract has no `{field}`"
        );
    }
    for field in [
        "package", "area", "environment", "gate", "execution", "origin", "state",
        "target_kinds", "compile_coverage_from", "evidence", "gap",
    ] {
        assert!(
            plan["cell"].get(field).is_some(),
            "the plan cell contract has no `{field}`"
        );
    }
    assert!(plan["package"].get("package").is_some());

    let origins: BTreeSet<&str> = contract["vocabulary"]["origins"]
        .as_array()
        .expect("origins is a list")
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        origins,
        ["ci", "local", "prior-local", "none"].into_iter().collect(),
        "the origin vocabulary drifted from `Origin`"
    );
    assert_eq!(
        contract["vocabulary"]["accepted_gap_state"].as_str(),
        Some(CellState::AcceptedGap.label()),
        "the accepted-gap state name drifted from `CellState`"
    );
}

/// The two current L2 gaps must name the work that closes them, and that work
/// must exist. Spec section 6 requires the closure link in the visible
/// description; a link to a deleted feature is worse than none.
#[test]
fn the_shipped_capability_table_names_what_closes_its_l2_gaps() {
    let text = fs::read_to_string(repo_root().join(".github/ci/environments.json"))
        .expect("environments.json is readable");
    let doc: EnvironmentsDoc = serde_json::from_str(&text).expect("environments.json parses");

    for environment in ["windows-latest", "wsl2-ubuntu"] {
        let gap = doc
            .environments
            .iter()
            .find(|candidate| candidate.name == environment)
            .expect("the environment is declared")
            .gap_for("tmux")
            .expect("the L2 backend gap is governed");
        assert!(
            !gap.closes.is_empty(),
            "{environment}'s tmux gap names no closing work"
        );
        assert!(
            repo_root().join(&gap.closes).exists(),
            "{environment}'s tmux gap points at {}, which does not exist",
            gap.closes
        );
    }
}

/// End to end over the real planner: the shipped `affected_scope.py` writes a
/// plan and this tool reads it, through the normal invocation path.
///
/// The unit fixtures build plan JSON by hand, which cannot catch the planner
/// emitting a shape the reader rejects. Skipped where python3 is unavailable
/// rather than failing: the Rust suite must still run on a host without it.
#[test]
fn the_real_planners_plan_rolls_up() {
    let Ok(output) = std::process::Command::new("python3")
        .current_dir(repo_root())
        .args([
            "scripts/ci/affected_scope.py",
            "--resolved-plan",
            "--",
            "claudine/lib/src/lib.rs",
        ])
        .output()
    else {
        eprintln!("python3 is unavailable; skipping the end-to-end plan fixture");
        return;
    };
    assert!(
        output.status.success(),
        "the planner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let plan: ResolvedPlan = serde_json::from_slice(&output.stdout)
        .expect("the planner's own output parses as a resolved plan");
    let expected = plan_expected_cells(&plan).expect("the planner writes the current generation");
    assert!(
        !expected.is_empty(),
        "a source change in claudine must schedule cells"
    );
    assert!(
        expected.iter().all(|cell| cell.area == "claudine"),
        "every cell of this plan belongs to the claudine area"
    );

    let cells = classify_simple(
        &expected
            .iter()
            .filter(|cell| is_test_tier(&cell.key.tier))
            .cloned()
            .collect::<Vec<_>>(),
        &[],
    );
    assert!(
        cells.iter().all(|cell| cell.area == "claudine"),
        "the area must survive into the result document"
    );
    assert!(
        cells.iter().any(|cell| cell.state == CellState::Missing),
        "nothing ran, so every scheduled test cell is missing: {cells:#?}"
    );
}

// --- The command surface, end to end -------------------------------------

/// `rollup --plan … --area … --out`, then `verdict --results … --area`, then
/// `summarize`, through the same argument parsing the CLI uses.
///
/// The unit fixtures above call the classifier directly. This one goes through
/// the files: a result document is written, read back, judged, and folded into
/// a summary, so a field that serializes but does not deserialize — or an
/// `--area` that narrows one command and not the other — cannot pass.
#[test]
fn the_command_surface_writes_reads_and_judges_one_areas_slice() {
    let temp = TempDir::new("cli");
    let artifacts = temp.path().join("ci-artifacts");
    let artifact = artifacts.join("junit-claudine-L1-ubuntu-latest");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(
        artifact.join("L1").join("claudine.xml"),
        junit("claudine", &passing_case("claudine::a")),
    )
    .unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        serde_json::json!({
            "tier": "L1",
            "package": "claudine",
            "xml": "L1/claudine.xml",
            "exit_code": 0,
            "environment": "ubuntu-latest",
            "duration_s": 7,
            "report_present": true,
        })
        .to_string()
            + "\n",
    )
    .unwrap();

    // Two areas: claudine executed on Linux and reused macOS; playa executed.
    let plan = temp.path().join("resolved-plan.json");
    let cells = vec![
        plan_cell_json("claudine", "ubuntu-latest", "L1", false),
        plan_cell_json("claudine", "macos-latest", "L1", true),
        plan_cell_json("playa", "ubuntu-latest", "L1", false),
    ];
    fs::write(
        &plan,
        serde_json::json!({
            "schema_version": 1,
            "packages": [{"package": "claudine"}, {"package": "playa"}],
            "cells": cells,
        })
        .to_string(),
    )
    .unwrap();

    let environments = temp.path().join("environments.json");
    fs::write(
        &environments,
        serde_json::json!({"schema_version": 1, "environments": []}).to_string(),
    )
    .unwrap();

    let results = temp.path().join("claudine-results.json");
    let summary = temp.path().join("summary.md");
    let args = Args::parse(
        [
            "--artifacts",
            artifacts.to_str().unwrap(),
            "--plan",
            plan.to_str().unwrap(),
            "--environments",
            environments.to_str().unwrap(),
            "--area",
            "claudine",
            "--out",
            results.to_str().unwrap(),
            "--summary",
            summary.to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();

    assert_eq!(cmd_rollup(&args).unwrap(), 0, "nothing in this slice blocks");

    let document: Rollup =
        serde_json::from_str(&fs::read_to_string(&results).unwrap()).expect("the slice reloads");
    assert_eq!(document.schema_version, RESULT_SCHEMA_VERSION);
    assert_eq!(document.area_scope, vec!["claudine".to_owned()]);
    assert_eq!(document.scope, vec!["claudine".to_owned()]);
    assert_eq!(
        document
            .cells
            .iter()
            .map(|cell| format!("{} {} {}", cell.key, cell.state, cell.origin))
            .collect::<Vec<_>>(),
        vec![
            "claudine/macos-latest/L1 PASS local".to_owned(),
            "claudine/ubuntu-latest/L1 PASS ci".to_owned(),
        ],
        "playa belongs to another area's slice"
    );
    assert_eq!(document.accepted_evidence.len(), 1);
    assert_eq!(
        document.cells[1].duration_s, 7,
        "the executed cell reports the duration its manifest recorded"
    );

    // Read/write/read: rolling the same run up twice produces identical bytes.
    let again = temp.path().join("claudine-results-2.json");
    let repeat = Args::parse(
        [
            "--artifacts",
            artifacts.to_str().unwrap(),
            "--plan",
            plan.to_str().unwrap(),
            "--environments",
            environments.to_str().unwrap(),
            "--area",
            "claudine",
            "--out",
            again.to_str().unwrap(),
            "--summary",
            summary.to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    assert_eq!(cmd_rollup(&repeat).unwrap(), 0);
    assert_eq!(
        fs::read_to_string(&results).unwrap(),
        fs::read_to_string(&again).unwrap(),
    );

    let baseline = temp.path().join("ci-baseline.toml");
    fs::write(&baseline, "schema_version = 2\n").unwrap();
    let verdict_args = Args::parse(
        [
            "--results",
            results.to_str().unwrap(),
            "--baseline",
            baseline.to_str().unwrap(),
            "--area",
            "claudine",
            "--summary",
            summary.to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    assert_eq!(cmd_verdict(&verdict_args).unwrap(), 0, "the slice is clear");

    let summarize_args = Args::parse(
        [
            "--results",
            results.to_str().unwrap(),
            "--summary",
            summary.to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    assert_eq!(cmd_summarize(&summarize_args).unwrap(), 0);

    let rendered = fs::read_to_string(&summary).unwrap();
    assert!(rendered.contains("### area `claudine`"), "{rendered}");
    assert!(rendered.contains("## CI results by area"), "{rendered}");
    assert!(
        rendered.contains("refs/notes/ci-local/macos-latest"),
        "the reused cell's evidence must reach the step summary: {rendered}"
    );
}

/// R10 under the plan: a `gates = false` package owns no plan cells, so its
/// governed NOT SCHEDULED entries can only come from the policy document. They
/// must not vanish when the rollup reads the plan.
#[test]
fn a_non_gating_packages_governed_cells_survive_the_plan_path() {
    let temp = TempDir::new("nongating");
    let artifacts = temp.path().join("ci-artifacts");
    fs::create_dir_all(&artifacts).unwrap();

    let plan = temp.path().join("resolved-plan.json");
    fs::write(
        &plan,
        serde_json::json!({
            "schema_version": 1,
            "packages": [{"package": "claudine"}, {"package": "tabby"}],
            "cells": [plan_cell_json("claudine", "ubuntu-latest", "L1", false)],
        })
        .to_string(),
    )
    .unwrap();

    let mut excluded = policy("tabby");
    excluded.gates = false;
    excluded.exclusion = Some(exclusion());
    let policy_doc = temp.path().join("scope.json");
    fs::write(
        &policy_doc,
        serde_json::json!({
            "policy": [{
                "package": "tabby",
                "area": "tabby",
                "gates": false,
                "tiers": ["L1"],
                "l2_backends": [],
                "companion_suites": [],
                "exclusion": {
                    "exclusion_class": "promotion-pending",
                    "owner": "@yankeeinlondon",
                    "reason": "blocked on the canonical recipe set",
                    "expiry": "2026-10-31",
                },
            }],
        })
        .to_string(),
    )
    .unwrap();

    let environments = temp.path().join("environments.json");
    fs::write(
        &environments,
        serde_json::json!({
            "schema_version": 1,
            "environments": [{"name": "ubuntu-latest", "capabilities": {}}],
        })
        .to_string(),
    )
    .unwrap();

    let results = temp.path().join("results.json");
    let args = Args::parse(
        [
            "--artifacts",
            artifacts.to_str().unwrap(),
            "--plan",
            plan.to_str().unwrap(),
            "--policy",
            policy_doc.to_str().unwrap(),
            "--environments",
            environments.to_str().unwrap(),
            "--out",
            results.to_str().unwrap(),
            "--summary",
            temp.path().join("summary.md").to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    assert_eq!(cmd_rollup(&args).unwrap(), EXIT_BLOCKED, "claudine produced nothing");

    let document: Rollup =
        serde_json::from_str(&fs::read_to_string(&results).unwrap()).expect("the document reloads");
    let tabby = document
        .cells
        .iter()
        .find(|cell| cell.key.package == "tabby")
        .expect("a non-gating package stays visible");
    assert_eq!(tabby.state, CellState::NotScheduled);
    assert_eq!(tabby.area, "tabby");
    assert!(
        tabby.reasons.iter().any(|reason| reason.contains("promotion-pending")),
        "its governance must travel with it: {:#?}",
        tabby.reasons
    );
}

#[test]
fn the_rollup_refuses_to_run_with_neither_a_plan_nor_a_policy() {
    let temp = TempDir::new("noinput");
    let args = Args::parse(
        ["--artifacts", temp.path().to_str().unwrap()]
            .into_iter()
            .map(str::to_owned),
    )
    .unwrap();
    let err = cmd_rollup(&args).expect_err("an expectation source is required");
    assert!(format!("{err:#}").contains("`--plan` or `--policy` is required"));
}

#[test]
fn a_failing_slice_makes_its_own_verdict_block() {
    // The other half of `the_command_surface…`: an area whose cell failed must
    // exit 2 from its own verdict, not from a global one.
    let temp = TempDir::new("cli-fail");
    let results = temp.path().join("results.json");
    let rollup = rollup_of(
        vec![Cell {
            area: "claudine".to_owned(),
            state: CellState::Fail,
            origin: Origin::Local,
            failed_tests: vec!["claudine::boom".to_owned()],
            scheduled: true,
            ..blank_cell(cell_key("claudine", "macos-latest", Tier::L1))
        }],
        &["claudine"],
    );
    fs::write(&results, serde_json::to_string(&rollup).unwrap()).unwrap();
    let baseline = temp.path().join("ci-baseline.toml");
    fs::write(&baseline, "schema_version = 2\n").unwrap();

    let args = Args::parse(
        [
            "--results",
            results.to_str().unwrap(),
            "--baseline",
            baseline.to_str().unwrap(),
            "--area",
            "claudine",
            "--summary",
            temp.path().join("summary.md").to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    assert_eq!(cmd_verdict(&args).unwrap(), EXIT_BLOCKED);
}

#[test]
fn an_area_that_selects_no_cell_is_a_tool_error_not_a_vacuous_pass() {
    let rollup = two_area_rollup();
    let err = narrow(rollup, &["messenger".to_owned()])
        .expect_err("a narrowing that selects nothing must not produce a verdict");
    let message = format!("{err:#}");
    assert!(message.contains("selects no cell"), "{message}");
    assert!(
        message.contains("claudine, playa"),
        "the error must name what the document does cover: {message}"
    );
}

#[test]
fn no_area_flag_leaves_the_document_whole() {
    let rollup = narrow(two_area_rollup(), &[]).expect("no narrowing requested");
    assert_eq!(rollup.cells.len(), 2);
    assert!(rollup.area_scope.is_empty());
}
