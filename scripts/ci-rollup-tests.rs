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

fn record(area: &str, environment: &str, tier: Tier, shard: &str, package: &str) -> RunRecord {
    RunRecord {
        area: area.to_owned(),
        environment: environment.to_owned(),
        tier,
        shard: shard.to_owned(),
        package: package.to_owned(),
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

fn passing_record(area: &str, environment: &str, tier: Tier) -> RunRecord {
    let mut rec = record(area, environment, tier, "1/1", "pkg");
    rec.counts = Counts {
        total: 3,
        passed: 3,
        ..Counts::default()
    };
    rec.passed_identities = vec!["pkg::a".into(), "pkg::b".into(), "pkg::c".into()];
    rec
}

fn expectation(area: &str, environment: &str, tier: Tier) -> ExpectedCell {
    ExpectedCell {
        key: CellKey {
            area: area.to_owned(),
            environment: environment.to_owned(),
            tier,
        },
        shards: vec!["1/1".to_owned()],
        backends: Vec::new(),
        declared_gap: None,
    }
}

/// A well-formed, owned, unexpired `areas.json` policy gap. Tests that exercise
/// a *malformed* one mutate this rather than restating every field.
fn gap() -> DeclaredGap {
    DeclaredGap {
        owner: "@yankeeinlondon".to_owned(),
        reason: "tmux has no Windows port".to_owned(),
        expiry: "2027-01-31".to_owned(),
    }
}

/// Classify with the given expectations and records and nothing else.
fn classify_simple(expected: &[ExpectedCell], records: &[RunRecord]) -> Vec<Cell> {
    let provisioned = BTreeMap::new();
    let expected_tests = BTreeMap::new();
    classify(&ClassifyInputs {
        expected,
        records,
        statuses: &[],
        provisioned: &provisioned,
        expected_tests: &expected_tests,
    })
}

fn only_cell(cells: Vec<Cell>) -> Cell {
    assert_eq!(cells.len(), 1, "expected exactly one cell, got {cells:#?}");
    cells.into_iter().next().unwrap()
}

fn rollup_of(cells: Vec<Cell>, scope: &[&str]) -> Rollup {
    Rollup {
        schema_version: SCHEMA_VERSION,
        run_id: None,
        scope: scope.iter().map(|s| (*s).to_owned()).collect(),
        scope_degraded: false,
        records: Vec::new(),
        cells,
    }
}

fn failure_entry(area: &str, environment: &str, tier: Tier) -> FailureEntry {
    FailureEntry {
        area: area.to_owned(),
        environment: environment.to_owned(),
        tier,
        shard: "1/1".to_owned(),
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
// Artifact name fallback
// ---------------------------------------------------------------------------

#[rstest]
#[case("junit-biscuit-file-L1-ubuntu-latest-0", "biscuit-file", Tier::L1, "ubuntu-latest")]
#[case("junit-sniff-L1-windows-latest-3", "sniff", Tier::L1, "windows-latest")]
#[case("junit-biscuit-terminal-L2-ubuntu", "biscuit-terminal", Tier::L2, "ubuntu")]
#[case("junit-darkmatter-browser-ubuntu", "darkmatter", Tier::Browser, "ubuntu")]
fn recovers_identity_from_an_artifact_name(
    #[case] name: &str,
    #[case] area: &str,
    #[case] tier: Tier,
    #[case] environment: &str,
) {
    let parsed = parse_artifact_name(name).expect("parsable");
    assert_eq!(
        parsed,
        DegradedIdentity {
            area: area.to_owned(),
            tier,
            environment: environment.to_owned(),
        }
    );
}

#[rstest]
#[case("results-biscuit-file-L1-ubuntu-latest-0")]
#[case("junit-biscuit-file-ubuntu-latest-0")]
#[case("junit-L1-ubuntu-latest")]
fn refuses_to_guess_an_unparsable_artifact_name(#[case] name: &str) {
    assert_eq!(parse_artifact_name(name), None);
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
    let mut rec = record("a", "ubuntu-latest", Tier::L2, "1/1", "pkg");
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

#[test]
fn state_not_applicable_when_the_tier_has_no_tests() {
    let rec = record("a", "ubuntu-latest", Tier::L2, "1/1", "pkg");
    let cell = only_cell(classify_simple(
        &[expectation("a", "ubuntu-latest", Tier::L2)],
        &[rec],
    ));
    assert_eq!(cell.state, CellState::NotApplicable);
    assert_ne!(cell.state, CellState::Pass, "a zero-test tier is N/A, not PASS");
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
    let mut rec = record("a", "ubuntu-latest", Tier::L1, "1/1", "pkg");
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
    let mut rec = record("a", "ubuntu-latest", Tier::L1, "1/1", "pkg");
    rec.parse_error = Some("truncated JUnit XML".to_owned());

    let cell = only_cell(classify_simple(
        &[expectation("a", "ubuntu-latest", Tier::L1)],
        &[rec],
    ));
    assert_eq!(cell.state, CellState::Missing);
}

#[test]
fn state_missing_when_a_shard_produced_nothing() {
    let mut expectation = expectation("darkmatter", "ubuntu-latest", Tier::L1);
    expectation.shards = vec!["1/2".to_owned(), "2/2".to_owned()];

    let mut rec = passing_record("darkmatter", "ubuntu-latest", Tier::L1);
    rec.shard = "1/2".to_owned();

    let cell = only_cell(classify_simple(&[expectation], &[rec]));
    assert_eq!(cell.state, CellState::Missing);
    assert_eq!(cell.missing_shards, vec!["2/2"]);
}

#[test]
fn state_not_scheduled_for_an_area_outside_the_run_scope() {
    let areas = vec![AreaPolicy {
        area: "homelab".to_owned(),
        ci: true,
        environments: default_environments(),
        shards: default_shards(),
        l2: false,
        browser: false,
        backends: Vec::new(),
        policy_gaps: Vec::new(),
    }];
    let scope = BTreeSet::new();

    let expected = expected_cells(&areas, &scope, &[]);
    assert!(expected.is_empty(), "an out-of-scope area schedules nothing");

    // With no expectation and no evidence there is no cell at all, which the
    // grid renders as NOT SCHEDULED.
    let cells = classify_simple(&expected, &[]);
    assert!(cells.is_empty());
}

#[test]
fn state_not_scheduled_for_a_ci_false_area() {
    let areas = vec![AreaPolicy {
        area: "tabby".to_owned(),
        ci: false,
        environments: default_environments(),
        shards: default_shards(),
        l2: false,
        browser: false,
        backends: Vec::new(),
        policy_gaps: Vec::new(),
    }];
    let scope: BTreeSet<String> = ["tabby".to_owned()].into_iter().collect();
    assert!(expected_cells(&areas, &scope, &[]).is_empty());
}

#[test]
fn state_policy_gap_when_no_compatible_backend_is_provisioned() {
    let mut expectation = expectation("biscuit-terminal", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned(), "wezterm".to_owned()];

    // The tier "passes" only because every test early-returned; without a
    // provisioned backend that is a policy gap, not a green cell.
    let record = passing_record("biscuit-terminal", "windows-latest", Tier::L2);
    let provisioned: BTreeMap<String, BTreeSet<String>> =
        [("windows-latest".to_owned(), BTreeSet::new())]
            .into_iter()
            .collect();
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation],
        records: &[record],
        statuses: &[],
        provisioned: &provisioned,
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.state.blocks());
}

#[test]
fn no_policy_gap_when_a_compatible_backend_is_provisioned() {
    let mut expectation = expectation("biscuit-terminal", "ubuntu-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned(), "wezterm".to_owned()];

    let provisioned: BTreeMap<String, BTreeSet<String>> = [(
        "ubuntu-latest".to_owned(),
        ["tmux".to_owned()].into_iter().collect(),
    )]
    .into_iter()
    .collect();
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation],
        records: &[passing_record("biscuit-terminal", "ubuntu-latest", Tier::L2)],
        statuses: &[],
        provisioned: &provisioned,
        expected_tests: &expected_tests,
    }));
    assert_eq!(cell.state, CellState::Pass);
}

#[test]
fn unknown_backend_provisioning_asserts_no_policy_gap() {
    let mut expectation = expectation("biscuit-terminal", "macos-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];

    let cell = only_cell(classify_simple(
        &[expectation],
        &[passing_record("biscuit-terminal", "macos-latest", Tier::L2)],
    ));
    assert_eq!(cell.state, CellState::Pass);
    assert!(cell.skip_evidence_degraded);
    assert!(cell.reasons.iter().any(|r| r.contains("unknown")));
}

/// The measured case: `needs: lint` skipped the whole matrix, GitHub never
/// evaluated the matrix context, and no artifact exists for any leg.
#[test]
fn a_failing_lint_is_never_blamed_for_a_missing_l1_cell() {
    // `needs: lint` was removed from the test job, so lint gates nothing. This
    // test previously asserted the opposite, which was correct while the edge
    // existed. Blaming any failing job in the area sent claudine's triage at
    // lint in run 30427703024 for MISSING L1 cells lint could not have caused.
    let statuses = vec![ProducerStatus {
        area: "claudine".to_owned(),
        job: "lint".to_owned(),
        result: "failure".to_owned(),
        environment: None,
        detail: None,
    }];
    let provisioned = BTreeMap::new();
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("claudine", "windows-latest", Tier::L1)],
        records: &[],
        statuses: &statuses,
        provisioned: &provisioned,
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
/// can tell them apart. In run 30595280027 that single wording hid two
/// different faults: claudine's guest was killed by SIGBUS after extracting 153
/// binaries, and darkmatter's shard 3/4 never provisioned (`wsl.exe` exit
/// 4294967295). Both rendered as the same blank cell.
#[test]
fn a_producer_detail_explains_why_a_cell_has_no_evidence() {
    let statuses = vec![ProducerStatus {
        area: "claudine".to_owned(),
        job: "L1".to_owned(),
        result: "failure".to_owned(),
        environment: Some("wsl2-ubuntu".to_owned()),
        detail: Some(
            "the WSL2 guest became unreachable after the test step".to_owned(),
        ),
    }];
    let provisioned = BTreeMap::new();
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("claudine", "wsl2-ubuntu", Tier::L1)],
        records: &[],
        statuses: &statuses,
        provisioned: &provisioned,
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
        area: "darkmatter".to_owned(),
        job: "L1".to_owned(),
        result: "failure".to_owned(),
        environment: None,
        detail: None,
    }];
    let provisioned = BTreeMap::new();
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("darkmatter", "ubuntu-latest", Tier::L2)],
        records: &[],
        statuses: &statuses,
        provisioned: &provisioned,
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

/// `areas.json` declares the gap, so it is authoritative — no provisioning
/// information is needed to render POLICY GAP.
#[test]
fn a_declared_policy_gap_renders_policy_gap_without_provisioning_data() {
    let mut expectation = expectation("biscuit-terminal", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];
    expectation.declared_gap = Some(gap());

    let cell = only_cell(classify_simple(
        &[expectation],
        &[passing_record("biscuit-terminal", "windows-latest", Tier::L2)],
    ));

    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.reasons.iter().any(|r| r.contains("declared policy gap")));
}

/// A declared gap explains the absence of evidence, so the cell must not read
/// MISSING — nobody failed to upload anything.
#[test]
fn a_declared_gap_with_no_evidence_is_policy_gap_not_missing() {
    let mut expectation = expectation("biscuit-tui", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];
    expectation.declared_gap = Some(gap());

    let cell = only_cell(classify_simple(&[expectation], &[]));
    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.state.blocks());
}

/// A real failure is more actionable than a config classification, so it wins
/// even inside a declared gap.
#[test]
fn a_real_failure_outranks_a_declared_gap() {
    let mut expectation = expectation("biscuit-tui", "windows-latest", Tier::L2);
    expectation.declared_gap = Some(gap());

    let mut rec = passing_record("biscuit-tui", "windows-latest", Tier::L2);
    rec.counts.failed = 1;
    rec.failed_tests = vec!["t::x".into()];

    let cell = only_cell(classify_simple(&[expectation], &[rec]));
    assert_eq!(cell.state, CellState::Fail);
    assert!(
        cell.reasons.iter().any(|r| r.contains("declared policy gap")),
        "the gap must still be recorded in reasons"
    );
}

#[test]
fn an_undeclared_policy_gap_is_named_as_undeclared() {
    let mut expectation = expectation("biscuit-tui", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];

    let provisioned: BTreeMap<String, BTreeSet<String>> =
        [("windows-latest".to_owned(), BTreeSet::new())]
            .into_iter()
            .collect();
    let expected_tests = BTreeMap::new();

    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation],
        records: &[passing_record("biscuit-tui", "windows-latest", Tier::L2)],
        statuses: &[],
        provisioned: &provisioned,
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.reasons.iter().any(|r| r.contains("UNDECLARED")));
}

// ---------------------------------------------------------------------------
// nextest exit codes are RAW, not normalized
// ---------------------------------------------------------------------------

/// 101 means the crate never built. That tells us nothing about whether the
/// tier has tests, so it can never be N/A — and because it emits no test
/// result, it can never be accepted as a known test failure either.
#[test]
fn a_build_failure_is_missing_and_says_so_not_an_empty_tier() {
    let mut rec = record("biscuit-file", "windows-latest", Tier::L1, "1/1", "biscuit-file");
    rec.report_present = false;
    rec.exit_code = 101;

    let cell = only_cell(classify_simple(
        &[expectation("biscuit-file", "windows-latest", Tier::L1)],
        &[rec],
    ));

    assert_eq!(cell.state, CellState::Missing);
    assert_ne!(cell.state, CellState::NotApplicable);
    assert!(
        cell.reasons.iter().any(|r| r.contains("failed to BUILD")),
        "a build failure must say so: {:?}",
        cell.reasons
    );
}

/// 100 means tests ran and failed, so a report *should* exist. Its absence is a
/// different defect from a build failure and must read differently.
#[test]
fn a_test_failure_with_no_staged_report_reads_differently_from_a_build_failure() {
    let mut rec = record("queue", "windows-latest", Tier::L1, "1/1", "queue");
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
// Shard identity lives only in the manifest
// ---------------------------------------------------------------------------

#[test]
fn a_report_with_no_manifest_records_shard_as_unknown_not_one_of_one() {
    let temp = TempDir::new("shardless");
    let artifact = temp.path().join("junit-darkmatter-L1-ubuntu-latest-2");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(
        artifact.join("L1").join("darkmatter.xml"),
        junit("darkmatter", &passing_case("a")),
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-darkmatter-L1-ubuntu-latest-2".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert_eq!(
        records[0].shard, "unknown",
        "the staged path carries no shard, so it must not be invented"
    );
    assert!(records[0].degraded);
}

/// Two shards of the same area staging into one directory collide on
/// `<tier>/<package>.xml` — but both manifest records survive, so keying on the
/// manifest keeps both identities.
#[test]
fn two_shard_records_sharing_one_xml_keep_distinct_identities() {
    let temp = TempDir::new("collide");
    let artifact = temp.path().join("junit-darkmatter-L1-ubuntu-latest-0");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(
        artifact.join("L1").join("darkmatter.xml"),
        junit("darkmatter", &passing_case("a")),
    )
    .unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        "{\"tier\":\"L1\",\"package\":\"darkmatter\",\"xml\":\"L1/darkmatter.xml\",\"exit_code\":0,\
         \"area\":\"darkmatter\",\"environment\":\"ubuntu-latest\",\"shard\":\"1/4\",\
         \"duration_s\":1,\"report_present\":true}\n\
         {\"tier\":\"L1\",\"package\":\"darkmatter\",\"xml\":\"L1/darkmatter.xml\",\"exit_code\":0,\
         \"area\":\"darkmatter\",\"environment\":\"ubuntu-latest\",\"shard\":\"2/4\",\
         \"duration_s\":1,\"report_present\":true}\n",
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-darkmatter-L1-ubuntu-latest-0".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert_eq!(records.len(), 2);
    let shards: BTreeSet<&str> = records.iter().map(|r| r.shard.as_str()).collect();
    assert_eq!(shards, ["1/4", "2/4"].into_iter().collect());
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
// areas.json tolerance
// ---------------------------------------------------------------------------

#[test]
fn unknown_areas_json_fields_are_ignored_and_new_environments_are_accepted() {
    let json = r#"[
      {
        "area": "sniff",
        "check_args": "-p sniff",
        "environments": ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"],
        "exclusion_class": "capability",
        "some_field_added_next_week": {"nested": [1, 2, 3]}
      }
    ]"#;

    let areas: Vec<AreaPolicy> = serde_json::from_str(json).expect("unknown fields are ignored");
    assert_eq!(areas[0].environments.len(), 4);
    assert!(areas[0]
        .environments
        .contains(&"wsl2-ubuntu".to_owned()));
}

#[test]
fn the_retired_full_os_field_is_still_read_as_environments() {
    let json = r#"[{"area": "a", "full_os": ["ubuntu-latest"]}]"#;
    let areas: Vec<AreaPolicy> = serde_json::from_str(json).unwrap();
    assert_eq!(areas[0].environments, vec!["ubuntu-latest"]);
}

#[test]
fn a_wsl2_environment_gets_its_own_cell_not_the_windows_one() {
    let areas = vec![AreaPolicy {
        area: "sniff".to_owned(),
        ci: true,
        environments: vec!["windows-latest".to_owned(), "wsl2-ubuntu".to_owned()],
        shards: default_shards(),
        l2: false,
        browser: false,
        backends: Vec::new(),
        policy_gaps: Vec::new(),
    }];
    let scope: BTreeSet<String> = ["sniff".to_owned()].into_iter().collect();

    let expected = expected_cells(&areas, &scope, &[]);
    let environments: BTreeSet<&str> = expected
        .iter()
        .map(|cell| cell.key.environment.as_str())
        .collect();
    assert_eq!(
        environments,
        ["windows-latest", "wsl2-ubuntu"].into_iter().collect()
    );
}

// ---------------------------------------------------------------------------
// Expected-test manifest
// ---------------------------------------------------------------------------

#[test]
fn an_expected_test_with_no_result_counts_as_a_skip() {
    let expected_tests: ExpectedTests = [(
        ("windows-latest".to_owned(), Tier::L1),
        [(
            "pkg".to_owned(),
            vec!["pkg::a".to_owned(), "pkg::vanished".to_owned()],
        )]
        .into_iter()
        .collect(),
    )]
    .into_iter()
    .collect();

    let mut rec = record("a", "windows-latest", Tier::L1, "1/1", "pkg");
    rec.counts = Counts {
        total: 1,
        passed: 1,
        ..Counts::default()
    };
    rec.passed_identities = vec!["pkg::a".to_owned()];

    let provisioned = BTreeMap::new();
    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("a", "windows-latest", Tier::L1)],
        records: &[rec],
        statuses: &[],
        provisioned: &provisioned,
        expected_tests: &expected_tests,
    }));

    assert_eq!(cell.skipped_tests, vec!["pkg::vanished"]);
    assert!(!cell.skip_evidence_degraded);
}

#[test]
fn without_a_manifest_absence_is_not_inferred_to_be_a_skip() {
    let mut rec = record("a", "windows-latest", Tier::L1, "1/1", "pkg");
    rec.counts = Counts {
        total: 1,
        passed: 1,
        ..Counts::default()
    };
    rec.passed_identities = vec!["pkg::a".to_owned()];

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
        [("pkg".to_owned(), vec!["pkg::a".to_owned()])]
            .into_iter()
            .collect(),
    )]
    .into_iter()
    .collect();

    let mut rec = record("a", "windows-latest", Tier::L1, "1/1", "pkg");
    rec.report_present = false;

    let provisioned = BTreeMap::new();
    let cell = only_cell(classify(&ClassifyInputs {
        expected: &[expectation("a", "windows-latest", Tier::L1)],
        records: &[rec],
        statuses: &[],
        provisioned: &provisioned,
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
        schema_version: SCHEMA_VERSION,
        failure: vec![failure_entry("sniff", "macos-latest", Tier::L1)],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(cells, &["sniff"]), &baseline, None);
    assert!(!any_block(&findings), "unexpected blocks: {findings:#?}");
}

#[test]
fn a_listed_entry_that_now_passes_blocks_to_force_cleanup() {
    let cells = classify_simple(
        &[expectation("biscuit-speaks", "ubuntu-latest", Tier::L1)],
        &[passing_record("biscuit-speaks", "ubuntu-latest", Tier::L1)],
    );
    let baseline = Baseline {
        schema_version: SCHEMA_VERSION,
        failure: vec![failure_entry("biscuit-speaks", "ubuntu-latest", Tier::L1)],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(cells, &["biscuit-speaks"]), &baseline, None);
    assert!(blocks_with_rule(&findings, "baseline-now-passing"));
}

#[test]
fn an_out_of_scope_entry_is_ignored_and_not_treated_as_a_pass() {
    let baseline = Baseline {
        schema_version: SCHEMA_VERSION,
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
#[case(CellState::NotApplicable)]
#[case(CellState::PolicyGap)]
fn a_scheduled_entry_with_no_test_result_stays_blocking(#[case] state: CellState) {
    let cell = Cell {
        key: CellKey {
            area: "claudine".to_owned(),
            environment: "ubuntu-latest".to_owned(),
            tier: Tier::L1,
        },
        state,
        counts: Counts::default(),
        scheduled: true,
        missing_shards: Vec::new(),
        skipped_tests: Vec::new(),
        failed_tests: Vec::new(),
        skip_evidence_degraded: false,
        declared_gap: None,
        reasons: Vec::new(),
        records: Vec::new(),
    };
    let baseline = Baseline {
        schema_version: SCHEMA_VERSION,
        failure: vec![failure_entry("claudine", "ubuntu-latest", Tier::L1)],
        skip: Vec::new(),
    };

    let findings = verdict(&rollup_of(vec![cell], &["claudine"]), &baseline, None);
    assert!(
        blocks_with_rule(&findings, "baseline-no-result"),
        "state {state:?} must not be accepted as a known test failure: {findings:#?}"
    );
}

#[test]
fn a_baselined_area_that_produced_no_cell_at_all_blocks() {
    let baseline = Baseline {
        schema_version: SCHEMA_VERSION,
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
        schema_version: SCHEMA_VERSION,
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
// Every case here starts from a real `areas.json` shape — an area that owns L2
// tests, declares an L2 gap on `windows-latest`, and would otherwise render a
// green `0 run / N skipped` cell there.
// ---------------------------------------------------------------------------

/// Build the `sniff`-shaped Windows L2 cell: L2 tier, tmux-only backend, a
/// Windows runner that provisions nothing, and whatever gap declaration and
/// evidence the caller supplies.
///
/// ## Notes
///
/// The two realistic evidence shapes differ, and the difference is load-bearing.
/// A *declared* gap means the tier was never dispatched, so there are no records
/// at all — and only the declaration stops that reading as `MISSING`. An
/// *undeclared* gap means the tier ran and every test early-returned from
/// `require_level!`, which JUnit records as passes; that is the green cell the
/// state exists to catch.
fn windows_l2_gap_cell(declared: Option<DeclaredGap>, records: &[RunRecord]) -> Cell {
    let mut expectation = expectation("sniff", "windows-latest", Tier::L2);
    expectation.backends = vec!["tmux".to_owned()];
    expectation.declared_gap = declared;

    let provisioned: BTreeMap<String, BTreeSet<String>> =
        [("windows-latest".to_owned(), BTreeSet::new())]
            .into_iter()
            .collect();
    let expected_tests = BTreeMap::new();

    only_cell(classify(&ClassifyInputs {
        expected: &[expectation],
        records,
        statuses: &[],
        provisioned: &provisioned,
        expected_tests: &expected_tests,
    }))
}

fn gap_verdict(cell: Cell, today: Option<&str>) -> Vec<Finding> {
    verdict(&rollup_of(vec![cell], &["sniff"]), &Baseline::default(), today)
}

/// The defect this file's policy-gap rules exist to fix: eight areas declare an
/// owned Windows-L2 gap, so before this rule the verdict could never exit 0 on
/// any run touching one of them.
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

/// Acceptance changes the verdict, never the grid. Success criterion 2 and plan
/// §2.3: never a green `0 run / N skipped` cell.
#[test]
fn an_accepted_policy_gap_still_renders_policy_gap_never_pass() {
    let cell = windows_l2_gap_cell(Some(gap()), &[]);
    assert_eq!(cell.state, CellState::PolicyGap);
    assert_ne!(cell.state, CellState::Pass);

    let rollup = rollup_of(vec![cell], &["sniff"]);
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

/// Same rule §1.3 applies to a baselined failure, applied to a gap: a lapsed
/// bound is a permanent exclusion wearing a temporary label.
#[test]
fn an_expired_policy_gap_blocks() {
    let mut expired = gap();
    expired.expiry = "2026-01-31".to_owned();

    let findings = gap_verdict(windows_l2_gap_cell(Some(expired), &[]), Some("2026-07-28"));
    assert!(blocks_with_rule(&findings, "policy-gap-expired"));
}

/// The case that catches someone quietly turning a tier off. `areas.json` says
/// nothing, so there is nobody to hold accountable and nothing to expire.
#[test]
fn an_undeclared_policy_gap_still_blocks() {
    let cell = windows_l2_gap_cell(
        None,
        &[passing_record("sniff", "windows-latest", Tier::L2)],
    );
    assert_eq!(cell.state, CellState::PolicyGap);
    assert!(cell.declared_gap.is_none());

    let findings = gap_verdict(cell, Some("2026-07-28"));
    assert!(blocks_with_rule(&findings, "cell-policy-gap"));
    assert!(
        !findings.iter().any(|f| f.rule == "policy-gap-accepted"),
        "an undeclared gap is never acceptable: {findings:#?}"
    );
}

/// A gap declaration must never suppress genuine evidence. `classify_one` ranks
/// `Fail` above `PolicyGap`, so the cell never reaches the acceptance path.
#[test]
fn a_declared_gap_with_real_failures_still_surfaces_them() {
    let mut expectation = expectation("sniff", "windows-latest", Tier::L2);
    expectation.declared_gap = Some(gap());

    let mut rec = passing_record("sniff", "windows-latest", Tier::L2);
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

/// The shape a *correctly* declared gap actually produces when its job runs: a
/// `require_level!` gate that skips for want of a backend early-returns, and
/// nextest records that as a JUnit pass. The cell must still read POLICY GAP,
/// and must still be accepted — a "the tests passed, so the gap is stale" rule
/// would block precisely the case the gap exists to describe.
#[test]
fn a_declared_gap_whose_tests_all_early_returned_is_still_accepted() {
    let mut expectation = expectation("sniff", "windows-latest", Tier::L2);
    expectation.declared_gap = Some(gap());

    let cell = only_cell(classify_simple(
        &[expectation],
        &[passing_record("sniff", "windows-latest", Tier::L2)],
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

/// `verdict` reads only `results.json`, never `areas.json`, so the gap's
/// accountability fields have to survive the round trip or the decision is made
/// on absent data.
#[test]
fn a_declared_gap_round_trips_through_the_result_document() {
    let rollup = rollup_of(vec![windows_l2_gap_cell(Some(gap()), &[])], &["sniff"]);
    let json = serde_json::to_string(&rollup).unwrap();
    let parsed: Rollup = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.cells[0].declared_gap, Some(gap()));
    assert!(!any_block(&gap_verdict(
        parsed.cells[0].clone(),
        Some("2026-07-28")
    )));
}

/// `areas.json` is the source of the gap, so the deserializer has to keep
/// `expiry` — which it did not before this rule existed.
#[test]
fn areas_json_policy_gap_expiry_reaches_the_cell() {
    let areas: Vec<AreaPolicy> = serde_json::from_str(
        r#"[{
          "area": "sniff",
          "l2": true,
          "backends": ["tmux"],
          "policy_gaps": [{
            "tier": "L2",
            "environments": ["windows-latest"],
            "reason": "tmux has no Windows port",
            "owner": "@yankeeinlondon",
            "expiry": "2027-01-31"
          }]
        }]"#,
    )
    .unwrap();
    let scope: BTreeSet<String> = ["sniff".to_owned()].into_iter().collect();

    let gap = expected_cells(&areas, &scope, &[])
        .into_iter()
        .find(|cell| cell.key.environment == "windows-latest" && cell.key.tier == Tier::L2)
        .expect("windows L2 must be expected, so it can render POLICY GAP")
        .declared_gap
        .expect("the declared gap must reach the expectation");

    assert_eq!(gap.owner, "@yankeeinlondon");
    assert_eq!(gap.expiry, "2027-01-31");
}

// ---------------------------------------------------------------------------
// Exact-test-identity skip diff
// ---------------------------------------------------------------------------

fn skip_cell(skips: &[&str]) -> Cell {
    Cell {
        key: CellKey {
            area: "biscuit-terminal".to_owned(),
            environment: "ubuntu-latest".to_owned(),
            tier: Tier::L2,
        },
        state: CellState::Pass,
        counts: Counts {
            total: 5,
            passed: 5 - skips.len() as u32,
            skipped: skips.len() as u32,
            ..Counts::default()
        },
        scheduled: true,
        missing_shards: Vec::new(),
        skipped_tests: skips.iter().map(|s| (*s).to_owned()).collect(),
        failed_tests: Vec::new(),
        skip_evidence_degraded: false,
        declared_gap: None,
        reasons: Vec::new(),
        records: Vec::new(),
    }
}

fn skip_budget(tests: &[&str]) -> Baseline {
    Baseline {
        schema_version: SCHEMA_VERSION,
        failure: Vec::new(),
        skip: vec![SkipEntry {
            area: "biscuit-terminal".to_owned(),
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

/// The crux of §1.2. Counts are identical (2 approved, 2 observed) — only an
/// identity-set comparison sees that one skip was resolved and a different one
/// appeared.
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
// Policy derivation
// ---------------------------------------------------------------------------

#[test]
fn expected_cells_cover_l1_shards_l2_and_browser() {
    let areas = vec![AreaPolicy {
        area: "darkmatter".to_owned(),
        ci: true,
        environments: vec!["ubuntu-latest".to_owned(), "windows-latest".to_owned()],
        shards: vec!["1/4".into(), "2/4".into(), "3/4".into(), "4/4".into()],
        l2: true,
        browser: true,
        backends: vec!["tmux".to_owned()],
        policy_gaps: Vec::new(),
    }];
    let scope: BTreeSet<String> = ["darkmatter".to_owned()].into_iter().collect();

    let expected = expected_cells(&areas, &scope, &["ubuntu-latest".to_owned()]);

    // L1 on both environments, L2 on both (the unprovisioned one must still
    // appear, so it can render POLICY GAP), browser on Linux only.
    assert_eq!(expected.len(), 5);
    let l1: Vec<&ExpectedCell> = expected
        .iter()
        .filter(|cell| cell.key.tier == Tier::L1)
        .collect();
    assert_eq!(l1.len(), 2);
    assert_eq!(l1[0].shards.len(), 4);
    assert_eq!(
        expected
            .iter()
            .filter(|cell| cell.key.tier == Tier::L2)
            .count(),
        2
    );
    assert_eq!(
        expected
            .iter()
            .filter(|cell| cell.key.tier == Tier::Browser)
            .count(),
        1
    );
}

#[test]
fn the_real_areas_json_parses() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".github")
        .join("ci")
        .join("areas.json");
    let text = fs::read_to_string(&path).expect("areas.json is readable");
    let areas: Vec<AreaPolicy> = serde_json::from_str(&text).expect("areas.json parses");

    assert!(areas.len() >= 31);
    let terminal = areas
        .iter()
        .find(|area| area.area == "biscuit-terminal")
        .expect("biscuit-terminal is declared");
    assert!(terminal.l2);
    assert!(terminal.backends.contains(&"tmux".to_owned()));

    // Every checked-in gap must satisfy the acceptance rule, or `ci-verdict`
    // can never exit 0 on a run touching that area — the defect these rules
    // exist to fix. Eight areas declare a Windows-L2 gap today.
    let gaps: Vec<(&String, &PolicyGap)> = areas
        .iter()
        .flat_map(|area| area.policy_gaps.iter().map(move |gap| (&area.area, gap)))
        .collect();
    assert!(gaps.len() >= 8, "expected the declared Windows-L2 gaps");
    for (area, gap) in gaps {
        assert!(!gap.owner.trim().is_empty(), "{area} gap has no owner");
        assert!(!gap.reason.trim().is_empty(), "{area} gap has no reason");
        assert!(
            is_iso_date(gap.expiry.trim()),
            "{area} gap expiry `{}` is not YYYY-MM-DD",
            gap.expiry
        );
    }
}

#[test]
fn the_checked_in_baseline_parses_and_every_entry_is_well_formed() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".github")
        .join("ci")
        .join("ci-baseline.toml");
    let baseline = load_baseline(&path).expect("the checked-in baseline is valid");

    assert!(!baseline.failure.is_empty());
    for entry in &baseline.failure {
        assert!(!entry.owner.is_empty(), "{} has no owner", entry.area);
        assert!(!entry.reason.is_empty(), "{} has no reason", entry.area);
        assert!(!entry.source_run.is_empty(), "{} has no source run", entry.area);
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

#[test]
fn an_invalid_expiry_is_refused() {
    let dir = std::env::temp_dir().join(format!("ci-rollup-expiry-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.toml");
    fs::write(
        &path,
        "[[failure]]\narea = \"a\"\nenvironment = \"ubuntu-latest\"\ntier = \"L1\"\n\
         owner = \"@o\"\nreason = \"r\"\nsource_run = \"1\"\nexpiry = \"soon\"\n",
    )
    .unwrap();

    let err = load_baseline(&path).expect_err("a non-date expiry must be refused");
    assert!(format!("{err:#}").contains("invalid expiry"));
    fs::remove_dir_all(&dir).ok();
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
    let artifact = temp.path().join("junit-biscuit-file-L1-ubuntu-latest-0");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(
        artifact.join("L1").join("biscuit-file-cli.xml"),
        junit("biscuit-file-cli", &passing_case("cli::a")),
    )
    .unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"biscuit-file-cli","xml":"L1/biscuit-file-cli.xml","exit_code":0,"area":"biscuit-file","environment":"windows-latest","shard":"2/4","duration_s":12,"report_present":true}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-biscuit-file-L1-ubuntu-latest-0".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].package, "biscuit-file-cli");
    assert_eq!(records[0].shard, "2/4");
    assert_eq!(
        records[0].environment, "windows-latest",
        "the manifest wins over the artifact directory name"
    );
    assert!(!records[0].degraded);
    assert_eq!(records[0].counts.passed, 1);
}

#[test]
fn a_report_with_no_manifest_falls_back_to_the_artifact_name_and_is_degraded() {
    let temp = TempDir::new("nomanifest");
    let artifact = temp.path().join("junit-sniff-L1-macos-latest-1");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(
        artifact.join("L1").join("sniff.xml"),
        junit("sniff", &failing_case("net::a")),
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-sniff-L1-macos-latest-1".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].area, "sniff");
    assert_eq!(records[0].environment, "macos-latest");
    assert_eq!(records[0].package, "sniff");
    assert!(records[0].degraded, "no manifest means degraded identity");
    assert_eq!(records[0].counts.failed, 1);
}

#[test]
fn report_present_false_yields_a_record_with_no_counts() {
    let temp = TempDir::new("noreport");
    let artifact = temp.path().join("junit-queue-L1-windows-latest-0");
    fs::create_dir_all(&artifact).unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"queue","xml":"L1/queue.xml","exit_code":101,"area":"queue","environment":"windows-latest","shard":"1/1","duration_s":3,"report_present":false}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-queue-L1-windows-latest-0".to_owned(),
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
    let artifact = temp.path().join("junit-playa-L1-ubuntu-latest-0");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    let full = junit("playa", &passing_case("a"));
    fs::write(
        artifact.join("L1").join("playa.xml"),
        &full[..full.len() / 2],
    )
    .unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"playa","xml":"L1/playa.xml","exit_code":0,"area":"playa","environment":"ubuntu-latest","shard":"1/1","duration_s":1,"report_present":true}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-playa-L1-ubuntu-latest-0".to_owned(),
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
    let artifact = temp.path().join("junit-research-L1-ubuntu-latest-0");
    fs::create_dir_all(&artifact).unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        r#"{"tier":"L1","package":"research","xml":"L1/research.xml","exit_code":0,"area":"research","environment":"ubuntu-latest","shard":"1/1","duration_s":1,"report_present":true}
"#,
    )
    .unwrap();

    let records = records_from_artifact(&ArtifactDir {
        name: "junit-research-L1-ubuntu-latest-0".to_owned(),
        path: artifact,
    })
    .unwrap();

    assert!(records[0].parse_error.is_some());
    assert!(!records[0].report_present);
}

#[test]
fn a_malformed_manifest_line_is_a_hard_error() {
    let temp = TempDir::new("badmanifest");
    let artifact = temp.path().join("junit-a-L1-ubuntu-latest-0");
    fs::create_dir_all(&artifact).unwrap();
    fs::write(artifact.join("manifest.jsonl"), "{not json}\n").unwrap();

    let err = records_from_artifact(&ArtifactDir {
        name: "junit-a-L1-ubuntu-latest-0".to_owned(),
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
fn provisioned_backends_parse_per_environment() {
    let args = Args::parse(
        [
            "--provisioned-backends",
            "ubuntu-latest=tmux",
            "--provisioned-backends",
            "windows-latest=",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();

    let provisioned = parse_provisioned(&args).unwrap();
    assert_eq!(
        provisioned["ubuntu-latest"],
        ["tmux".to_owned()].into_iter().collect::<BTreeSet<_>>()
    );
    assert!(
        provisioned["windows-latest"].is_empty(),
        "an explicit empty set means `provisioned nothing`, not `unknown`"
    );
}

#[test]
fn a_malformed_provisioned_backends_flag_is_rejected() {
    let args = Args::parse(
        ["--provisioned-backends", "tmux"]
            .into_iter()
            .map(str::to_owned),
    )
    .unwrap();
    assert!(parse_provisioned(&args).is_err());
}

#[test]
fn a_positional_argument_is_rejected() {
    assert!(Args::parse(["oops".to_owned()].into_iter()).is_err());
}

// ---------------------------------------------------------------------------
// `wsl2-ubuntu` and the areas.json default-environment contract
// ---------------------------------------------------------------------------

/// An area record that omits `environments`, exactly as serde builds it.
fn defaulted_area(area: &str) -> AreaPolicy {
    AreaPolicy {
        area: area.to_owned(),
        ci: true,
        environments: default_environments(),
        shards: default_shards(),
        l2: false,
        browser: false,
        backends: Vec::new(),
        policy_gaps: Vec::new(),
    }
}

fn scope_of(areas: &[&str]) -> BTreeSet<String> {
    areas.iter().map(|a| (*a).to_owned()).collect()
}

/// `AREA_DEFAULTS["environments"]` as written in `affected_scope.py`.
///
/// Panics rather than returning an `Option` on a shape it cannot read: a
/// silently-skipped drift guard is the failure mode this whole test exists to
/// prevent.
fn python_default_environments() -> Vec<String> {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ci/affected_scope.py"
    ))
    .expect("scripts/ci/affected_scope.py must be readable");

    let defaults = source
        .split_once("AREA_DEFAULTS")
        .expect("affected_scope.py must define AREA_DEFAULTS")
        .1;
    let line = defaults
        .lines()
        .find(|line| line.trim_start().starts_with("\"environments\":"))
        .expect("AREA_DEFAULTS must set \"environments\"");
    let list = line
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .expect("\"environments\" must be a single-line list literal")
        .0;

    let parsed: Vec<String> = list
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_owned())
        .filter(|item| !item.is_empty())
        .collect();
    assert!(
        !parsed.is_empty(),
        "parsed an empty environment list out of `{line}`, so this guard would pass vacuously"
    );
    parsed
}

/// Most areas omit `environments`, so this Rust list — not `areas.json` —
/// decides which cells exist for them. It silently fell behind the Python side
/// once already; that cost a full run of WSL2 evidence.
#[test]
fn default_environments_match_affected_scope_py() {
    assert_eq!(
        default_environments(),
        python_default_environments(),
        "ci-rollup's default environments have drifted from AREA_DEFAULTS in \
         scripts/ci/affected_scope.py; an area omitting `environments` will be \
         judged against a different cell set than CI actually scheduled"
    );
}

/// The reported bug: five WSL2 legs ran green and were filed `NOT SCHEDULED`.
#[test]
fn a_passing_wsl2_leg_renders_pass_not_not_scheduled() {
    let areas = vec![defaulted_area("biscuit-hash")];
    let expected = expected_cells(&areas, &scope_of(&["biscuit-hash"]), &[]);

    assert!(
        expected
            .iter()
            .any(|cell| cell.key.environment == "wsl2-ubuntu"),
        "an area that omits `environments` must still schedule a wsl2-ubuntu cell"
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
    let areas = vec![defaulted_area("biscuit-hash")];
    let expected = expected_cells(&areas, &scope_of(&["biscuit-hash"]), &[]);

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

/// The legitimate case must keep working: opting out is a policy statement, and
/// it is the one thing `NOT SCHEDULED` is for.
#[test]
fn an_area_excluding_wsl2_from_environments_still_renders_not_scheduled() {
    let mut area = defaulted_area("homelab");
    area.environments = vec!["ubuntu-latest".to_owned(), "macos-latest".to_owned()];

    let expected = expected_cells(&[area], &scope_of(&["homelab"]), &[]);
    assert!(
        !expected
            .iter()
            .any(|cell| cell.key.environment == "wsl2-ubuntu"),
        "a declared opt-out must schedule no wsl2-ubuntu cell"
    );

    // No expectation and no evidence means no cell at all, which the grid
    // renders as NOT SCHEDULED.
    let cells = classify_simple(&expected, &[]);
    assert!(!cells
        .iter()
        .any(|cell| cell.key.environment == "wsl2-ubuntu"));
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
