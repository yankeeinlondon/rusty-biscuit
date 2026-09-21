//! Runs the archive-path guard against the real repository.
//!
//! The matcher, the scanner, the plan reader, and the exemption rules live in
//! [`test_toolkit::archive_guard`], where they are unit-tested against fixture
//! corpora and temp trees. This file is only the driver: it resolves the scan
//! scope from the CI planner's resolved plan, scans the checkout, and turns the
//! result into two failures a developer can act on.

use std::collections::BTreeSet;
use std::path::Path;

use test_toolkit::archive_guard::{
    ALLOWED, GuardPlan, ScanReport, allowlist_problems, raw_live_form_files, repo_root, scan,
};

/// Scan the checkout in the mode the resolved plan asked for, announcing the
/// mode and the file count first.
///
/// The announcement is the whole reason this is not inlined: a changed-file
/// scan that checked nothing and a full-tree scan that checked everything both
/// report zero violations, and only the printed summary tells them apart.
fn planned_scan(root: &Path) -> ScanReport {
    let plan = GuardPlan::from_env().unwrap_or_else(|err| panic!("{err}"));

    if !plan.selected {
        println!(
            "archive-path guard: not selected by the resolved plan ({})",
            plan.reason
        );
    }

    let report = scan(root, &plan.mode).unwrap_or_else(|err| panic!("{err}"));
    println!("{}", report.summary());
    println!("archive-path guard: scope reason — {}", plan.reason);
    report
}

#[test]
fn no_archive_executed_target_bakes_in_a_producer_path() {
    let root = repo_root();
    let report = planned_scan(&root);
    let allowed: BTreeSet<&str> = ALLOWED.iter().map(|entry| entry.file).collect();

    let offenders: Vec<String> = report
        .violations
        .iter()
        .filter(|violation| !allowed.contains(violation.file.as_str()))
        .map(ToString::to_string)
        .collect();

    assert!(
        offenders.is_empty(),
        "compile-time paths do not survive an archived run; {} site(s):\n{}\n\n\
         Prefer `biscuit_test_harness::manifest_dir!()` / `bin_exe!(\"<bin>\")`. \
         If a site is genuinely never archive-executed, add it to ALLOWED in \
         tools/test-toolkit/src/archive_guard.rs with a one-line reason.",
        offenders.len(),
        offenders.join("\n"),
    );
}

#[test]
fn every_allowlist_entry_still_names_a_live_site() {
    let root = repo_root();
    // Deliberately the raw full-tree form scan rather than the (possibly
    // changed-file, possibly fallback-filtered) violation list: a valid
    // runtime-first fallback produces no violation, and validating exemptions
    // against violations would declare the harness's own entry stale.
    let live = raw_live_form_files(&root).unwrap_or_else(|err| panic!("{err}"));
    println!(
        "archive-path guard: exemption maintenance checked {} live form site(s) across the full tree",
        live.len()
    );

    let problems = allowlist_problems(ALLOWED, &live, &root);

    assert!(
        problems.is_empty(),
        "the ALLOWED exemption list is out of date; {} problem(s):\n{}",
        problems.len(),
        problems.join("\n"),
    );
}
