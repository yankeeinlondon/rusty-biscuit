//! The report foundation through the injected runner: fake checks whose
//! state is a marker file in a temporary home, so a repair is a real write
//! and every re-read is a real read.

use std::fs;
use std::path::{Path, PathBuf};

use super::report::{Finding, Scope};
use super::{CheckDescriptor, CheckOutcome, ContextUnavailable, DoctorRow, Environment, Pass, RepairOutcome, Severity};

fn environment(home: &Path) -> Environment {
    Environment::for_test(Some(home.to_path_buf()), Err(ContextUnavailable))
}

fn marker(pass: &Pass<'_>, name: &str) -> Option<String> {
    let home = pass.environment().home()?;
    fs::read_to_string(home.join(name)).ok().map(|text| text.trim().to_string())
}

fn write_marker(pass: &Pass<'_>, name: &str, text: &str) {
    fs::write(pass.environment().home().expect("home").join(name), text).expect("write marker");
}

/// `alpha` reads its marker: `broken` is a fixable error, anything else ok.
fn check_alpha(pass: &Pass<'_>) -> CheckOutcome {
    let mut outcome = CheckOutcome::new("alpha is healthy");
    if marker(pass, "alpha").as_deref() == Some("broken") {
        outcome.push(Finding::new(Severity::Error, Scope::User, "alpha is broken").fixable().remedy("run the fix"));
    }
    outcome
}

/// Repairs `alpha` unless `alpha-fails` exists, in which case it makes one
/// change, then fails (and, with `alpha-fails` holding `heal`, leaves the
/// disk healthy anyway).
fn repair_alpha(pass: &Pass<'_>) -> RepairOutcome {
    match marker(pass, "alpha-fails").as_deref() {
        Some(mode) => {
            write_marker(pass, "alpha-partial", "done");
            if mode == "heal" {
                write_marker(pass, "alpha", "ok");
            }
            RepairOutcome { changes: vec!["alpha-partial written".into()], failure: Some("the second step failed".into()) }
        }
        None => {
            write_marker(pass, "alpha", "ok");
            RepairOutcome { changes: vec!["alpha replaced".into()], failure: None }
        }
    }
}

/// `beta` depends on `alpha`'s marker and on its own: a warning while
/// alpha is broken, info when its own marker says it is blocked.
fn check_beta(pass: &Pass<'_>) -> CheckOutcome {
    let mut outcome = CheckOutcome::new("beta is healthy");
    if marker(pass, "beta").as_deref() == Some("blocked") {
        outcome.push(Finding::new(Severity::Info, Scope::General, "beta could not run: alpha's file is unavailable"));
    } else if marker(pass, "alpha").as_deref() == Some("broken") {
        outcome.push(Finding::new(Severity::Warn, Scope::Repository, "beta sees a broken alpha"));
    }
    outcome
}

/// `gamma` repairs by reading what `alpha`'s repair wrote: fixable while
/// `gamma` is broken, and its repair only succeeds when alpha is healthy.
fn check_gamma(pass: &Pass<'_>) -> CheckOutcome {
    let mut outcome = CheckOutcome::new("gamma is healthy");
    if marker(pass, "gamma").as_deref() == Some("broken") {
        outcome.push(Finding::new(Severity::Error, Scope::User, "gamma is broken").fixable());
    }
    outcome
}

fn repair_gamma(pass: &Pass<'_>) -> RepairOutcome {
    if marker(pass, "alpha").as_deref() == Some("broken") {
        return RepairOutcome { changes: Vec::new(), failure: Some("alpha is still broken".into()) };
    }
    write_marker(pass, "gamma", "ok");
    RepairOutcome { changes: vec!["gamma replaced".into()], failure: None }
}

/// Several findings at mixed severities and scopes, out of order.
fn check_mixed(_: &Pass<'_>) -> CheckOutcome {
    CheckOutcome::from(vec![
        Finding::new(Severity::Warn, Scope::Repository, "repository warning").at("/b/repo.yaml"),
        Finding::new(Severity::Error, Scope::Repository, "repository error").at("/b/repo.yaml"),
        Finding::new(Severity::Info, Scope::General, "a general note"),
        Finding::new(Severity::Error, Scope::User, "second user error").at("/a/z.yaml"),
        Finding::new(Severity::Ok, Scope::User, "first user file is fine").at("/a/a.yaml"),
    ])
}

fn check_hostile(_: &Pass<'_>) -> CheckOutcome {
    CheckOutcome::from(vec![
        Finding::new(Severity::Warn, Scope::User, "key `a\u{1b}[31mred`\nsecond line")
            .at("/tmp/evil\u{7}name")
            .note("bidi \u{202e}reversed")
            .remedy("tab\there"),
    ])
}

const fn descriptor(
    id: &'static str,
    prerequisites: &'static [&'static str],
    check: fn(&Pass<'_>) -> CheckOutcome,
    repair: Option<fn(&Pass<'_>) -> RepairOutcome>,
) -> CheckDescriptor {
    CheckDescriptor { id, title: id, prerequisites, check, repair }
}

fn registry() -> [CheckDescriptor; 3] {
    [
        descriptor("alpha", &[], check_alpha, Some(repair_alpha)),
        descriptor("beta", &[], check_beta, None),
        descriptor("gamma", &["beta"], check_gamma, Some(repair_gamma)),
    ]
}

fn home() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().to_path_buf();
    (dir, path)
}

fn row<'a>(rows: &'a [DoctorRow], id: &str) -> &'a DoctorRow {
    rows.iter().find(|row| row.id == id).expect("row")
}

#[test]
fn healthy_checks_are_ok_rows_in_registry_order() {
    let (_dir, home) = home();
    let rows = super::run_with(&registry(), &environment(&home), false);
    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), ["alpha", "beta", "gamma"]);
    for row in &rows {
        assert_eq!(row.severity, Severity::Ok, "{row:?}");
        assert!(!row.fixable);
        assert_eq!(row.detail, "");
    }
    assert_eq!(rows[0].summary, "alpha is healthy");
}

#[test]
fn a_row_takes_its_worst_finding_and_lists_findings_by_scope_then_path() {
    let (_dir, home) = home();
    let rows = super::run_with(&[descriptor("mixed", &[], check_mixed, None)], &environment(&home), false);
    let row = &rows[0];
    assert_eq!(row.severity, Severity::Error);
    // Both errors count; the first one in the check's order leads.
    assert_eq!(row.summary, "repository error (+1 more)");
    assert_eq!(
        row.detail,
        [
            "a general note",
            "/a/a.yaml: first user file is fine",
            "/a/z.yaml: second user error",
            "/b/repo.yaml: repository warning",
            "/b/repo.yaml: repository error",
        ]
        .join("\n")
    );
    assert!(!row.fixable);
}

#[test]
fn a_blocked_check_is_an_info_row_naming_what_blocked_it_and_the_others_still_run() {
    let (_dir, home) = home();
    fs::write(home.join("beta"), "blocked").unwrap();
    fs::write(home.join("gamma"), "broken").unwrap();
    let rows = super::run_with(&registry(), &environment(&home), false);
    let beta = row(&rows, "beta");
    assert_eq!(beta.severity, Severity::Info);
    assert_eq!(beta.summary, "beta could not run: alpha's file is unavailable");
    assert_eq!(row(&rows, "gamma").severity, Severity::Error);
    assert_eq!(row(&rows, "alpha").severity, Severity::Ok);
}

#[test]
fn fixable_is_reported_without_the_flag_and_an_inspection_writes_nothing() {
    let (_dir, home) = home();
    fs::write(home.join("alpha"), "broken").unwrap();
    let rows = super::run_with(&registry(), &environment(&home), false);
    let alpha = row(&rows, "alpha");
    assert_eq!((alpha.severity, alpha.fixable), (Severity::Error, true));
    assert_eq!(alpha.summary, "alpha is broken");
    assert_eq!(alpha.detail, "alpha is broken\nRemedy: run the fix");
    assert_eq!(row(&rows, "beta").severity, Severity::Warn);
    assert_eq!(fs::read_to_string(home.join("alpha")).unwrap(), "broken");
    assert_eq!(fs::read_dir(&home).unwrap().count(), 1);
}

#[test]
fn a_successful_repair_reports_the_verified_state_and_clears_fixable() {
    let (_dir, home) = home();
    fs::write(home.join("alpha"), "broken").unwrap();
    let rows = super::run_with(&registry(), &environment(&home), true);
    let alpha = row(&rows, "alpha");
    assert_eq!((alpha.severity, alpha.fixable), (Severity::Ok, false));
    assert_eq!(alpha.summary, "alpha is healthy");
    assert_eq!(alpha.detail, "Repaired: alpha replaced");
    assert_eq!(fs::read_to_string(home.join("alpha")).unwrap(), "ok");
    // beta has no repair and was not attempted, but its first reading
    // stands; its prerequisites were not touched either.
    assert_eq!(row(&rows, "beta").severity, Severity::Warn);
}

#[test]
fn a_second_successful_run_is_a_no_op() {
    let (_dir, home) = home();
    fs::write(home.join("alpha"), "broken").unwrap();
    let environment = environment(&home);
    super::run_with(&registry(), &environment, true);
    let modified = fs::metadata(home.join("alpha")).unwrap().modified().unwrap();
    let rows = super::run_with(&registry(), &environment, true);
    assert!(rows.iter().all(|row| row.severity <= Severity::Warn && !row.fixable), "{rows:?}");
    assert_eq!(row(&rows, "alpha").detail, "", "nothing was eligible, so nothing was attempted");
    assert_eq!(fs::metadata(home.join("alpha")).unwrap().modified().unwrap(), modified);
}

#[test]
fn repairs_run_in_order_from_fresh_evidence_and_rows_they_affect_are_read_again() {
    let (_dir, home) = home();
    fs::write(home.join("alpha"), "broken").unwrap();
    fs::write(home.join("gamma"), "broken").unwrap();
    let rows = super::run_with(&registry(), &environment(&home), true);
    // gamma's repair saw alpha's edit, made after the first inspection.
    let gamma = row(&rows, "gamma");
    assert_eq!((gamma.severity, gamma.fixable), (Severity::Ok, false), "{gamma:?}");
    assert_eq!(gamma.detail, "Repaired: gamma replaced");
    // beta is gamma's prerequisite, so it was read again after the repairs.
    assert_eq!(row(&rows, "beta").severity, Severity::Ok);
}

#[test]
fn a_failed_repair_is_an_error_with_its_reason_and_the_changes_already_made() {
    let (_dir, home) = home();
    fs::write(home.join("alpha"), "broken").unwrap();
    fs::write(home.join("alpha-fails"), "stop").unwrap();
    let rows = super::run_with(&registry(), &environment(&home), true);
    let alpha = row(&rows, "alpha");
    assert_eq!(alpha.severity, Severity::Error);
    assert_eq!(alpha.summary, "the repair failed: the second step failed");
    assert_eq!(
        alpha.detail,
        "alpha is broken\nRemedy: run the fix\nRepair failed: the second step failed\nAlready changed: alpha-partial written"
    );
    // A fresh read still finds the finding eligible, so a retry can apply.
    assert!(alpha.fixable);
    assert!(home.join("alpha-partial").is_file());
}

#[test]
fn a_failed_repair_stays_an_error_even_when_the_disk_now_reads_healthy() {
    let (_dir, home) = home();
    fs::write(home.join("alpha"), "broken").unwrap();
    fs::write(home.join("alpha-fails"), "heal").unwrap();
    let rows = super::run_with(&registry(), &environment(&home), true);
    let alpha = row(&rows, "alpha");
    assert_eq!((alpha.severity, alpha.fixable), (Severity::Error, false));
    assert!(alpha.detail.starts_with("Repair failed: the second step failed"), "{alpha:?}");
}

#[test]
fn a_failed_repair_with_no_change_says_so_and_unrelated_repairs_still_run() {
    let (_dir, home) = home();
    fs::write(home.join("alpha"), "broken").unwrap();
    fs::write(home.join("alpha-fails"), "stop").unwrap();
    fs::write(home.join("gamma"), "broken").unwrap();
    let rows = super::run_with(&registry(), &environment(&home), true);
    let gamma = row(&rows, "gamma");
    assert_eq!(gamma.severity, Severity::Error);
    assert!(gamma.detail.ends_with("Repair failed: alpha is still broken\nNo change was made before the failure"), "{gamma:?}");
    assert!(gamma.fixable);
}

#[test]
fn dynamic_text_cannot_carry_control_or_bidirectional_characters() {
    let (_dir, home) = home();
    let rows = super::run_with(&[descriptor("hostile", &[], check_hostile, None)], &environment(&home), false);
    let row = &rows[0];
    assert_eq!(row.summary, "key `a\\u{1b}[31mred`\\u{a}second line");
    assert_eq!(
        row.detail,
        "/tmp/evil\\u{7}name: key `a\\u{1b}[31mred`\\u{a}second line\nbidi \\u{202e}reversed\nRemedy: tab\\u{9}here"
    );
    assert!(!row.summary.chars().chain(row.detail.chars().filter(|c| *c != '\n')).any(char::is_control));
}

#[test]
fn a_row_serializes_to_the_stable_json_fields() {
    let row = DoctorRow { id: "tts", severity: Severity::Warn, summary: "s".into(), detail: String::new(), fixable: false };
    assert_eq!(
        serde_json::to_value(&row).unwrap(),
        serde_json::json!({"id": "tts", "severity": "warn", "summary": "s", "detail": "", "fixable": false})
    );
    let names: Vec<&str> = [Severity::Ok, Severity::Info, Severity::Warn, Severity::Error].map(Severity::as_str).to_vec();
    assert_eq!(names, ["ok", "info", "warn", "error"]);
}

#[test]
fn the_built_in_registry_lists_the_seven_checks_in_report_order() {
    let ids: Vec<&str> = super::REGISTRY.iter().map(|check| check.id).collect();
    assert_eq!(ids, ["config", "registration", "darkmatter-permissions", "repo-config", "stray-config-keys", "tts", "path"]);
    assert_eq!(super::title_of("tts"), Some("Text-to-speech"));
    assert_eq!(super::title_of("nope"), None);
}
