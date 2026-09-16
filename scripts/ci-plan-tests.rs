//! Contract tests for the resolved-plan renderer.
//!
//! The renderer is a projection of the canonical plan, so the fixtures assert
//! that every cell and every governed reason survives the projection. A
//! reviewer who cannot see a cell cannot reconcile it with a constraint.

use super::*;

fn plan_json(prohibited: bool) -> String {
    let wsl = if prohibited {
        r#"{"package":"alpha","area":"pkg","environment":"wsl2-ubuntu","gate":"L1",
            "execution":"omit","origin":"none","state":"prohibited",
            "target_kinds":[],"compile_coverage_from":"","selection_reason":"constrained",
            "prohibition":{"owner":"ken","reason":"do not rerun WSL for this branch",
            "expiry":"2099-01-01"}}"#
    } else {
        r#"{"package":"alpha","area":"pkg","environment":"wsl2-ubuntu","gate":"L1",
            "execution":"reuse","origin":"prior-local","state":"reused",
            "target_kinds":[],"compile_coverage_from":"L1","selection_reason":"prior receipt",
            "evidence":{"measurements":"12 test(s), 0 failed, 61s",
            "evidence":{"ref":"refs/notes/ci-local/wsl2-ubuntu"}}}"#
    };
    format!(
        r#"{{
        "schema_version":3,"base":"{a}","head":"{b}","change_class":"package",
        "full_scope":false,"full_scope_gates":[],
        "areas":[{{"area":"pkg","selection_reason":"source change","packages":["alpha"]}}],
        "packages":[],"source_packages":["alpha"],"reverse_dependencies":[],
        "cells":[
          {{"package":"alpha","area":"pkg","environment":"macos-latest","gate":"L1",
            "execution":"reuse","origin":"local","state":"reused","target_kinds":["lib"],
            "compile_coverage_from":"L1","selection_reason":"this host's receipt",
            "evidence":{{"measurements":"7 test(s), 0 failed, 3s",
            "evidence":{{"ref":"refs/notes/ci-local/macos-latest"}}}}}},
          {{"package":"alpha","area":"pkg","environment":"ubuntu-latest","gate":"L1",
            "execution":"execute","origin":"ci","state":"pending","target_kinds":["lib"],
            "compile_coverage_from":"L1","selection_reason":"no evidence here",
            "build":"0123456789abcdef"}},
          {{"package":"alpha","area":"pkg","environment":"windows-latest","gate":"L2",
            "execution":"omit","origin":"none","state":"accepted-gap","target_kinds":[],
            "compile_coverage_from":"","selection_reason":"no backend",
            "gap":{{"owner":"@yankeeinlondon","reason":"no L2 backend on Windows",
            "expiry":"2027-01-31"}}}},
          {wsl}
        ],
        "builds":[
          {{"key":"0123456789abcdef","package":"alpha","producer":"ubuntu-latest",
            "artifact":"build-alpha-ubuntu-latest-0123456789abcdef",
            "compatible_environments":["ubuntu-latest","wsl2-ubuntu"],
            "compatibility_reason":"x86_64-unknown-linux-gnu archive produced on ubuntu-latest (x86_64/gnu/glibc); wsl2-ubuntu declares the same architecture, ABI, and libc",
            "consumers":[{{"environment":"ubuntu-latest","gate":"L1"}}],
            "identity":{{"package":"alpha"}}}}
        ],
        "accepted_evidence":[],"evidence_rejections":["gate-inputs-changed: alpha/macos-latest/L1"],
        "policy_gaps":[],"prohibited_cells":{prohibited_cells},
        "job_estimate":4,"preflight_os":["ubuntu-latest"],
        "preflight_reason":"package-local change","flags":{{"ci_tooling":false}}}}"#,
        a = "a".repeat(40),
        b = "b".repeat(40),
        wsl = wsl,
        prohibited_cells = if prohibited {
            r#"["alpha/wsl2-ubuntu/L1"]"#
        } else {
            "[]"
        },
    )
}

fn parsed(prohibited: bool) -> Plan {
    serde_json::from_str(&plan_json(prohibited)).expect("fixture plan parses")
}

/// One cell's row, found by its key.
///
/// Asserting on rows rather than on rendered text is deliberate: the table
/// wraps and hyphenates to the terminal's width, so a text assertion would be
/// testing the layout engine instead of the projection.
fn row(prohibited: bool, key: &str) -> Vec<String> {
    rows(&parsed(prohibited))
        .into_iter()
        .find(|row| row[0] == key)
        .unwrap_or_else(|| panic!("no row for {key}"))
}

/// The detail line for one cell, from the narrative list under the table.
fn detail_of(prohibited: bool, key: &str) -> String {
    details(&parsed(prohibited))
        .into_iter()
        .find(|line| line.starts_with(&format!("`{key}`")))
        .unwrap_or_else(|| panic!("no detail line for {key}"))
}

#[test]
fn every_cell_appears_in_the_rendered_plan() {
    let output = render(&parsed(false), &Terminal::new());
    for key in [
        "alpha/macos-latest/L1",
        "alpha/ubuntu-latest/L1",
        "alpha/windows-latest/L2",
        "alpha/wsl2-ubuntu/L1",
    ] {
        assert!(output.contains(key), "the plan omits {key}:\n{output}");
    }
}

#[test]
fn every_disposition_is_visible() {
    let projected = rows(&parsed(false));
    let executions = projected.iter().map(|row| row[1].as_str()).collect::<Vec<_>>();
    let states = projected.iter().map(|row| row[3].as_str()).collect::<Vec<_>>();
    assert!(executions.contains(&"reuse") && executions.contains(&"execute"));
    assert!(executions.contains(&"omit"));
    assert!(states.contains(&"reused") && states.contains(&"pending"));
    assert!(states.contains(&"accepted-gap"));
    assert!(projected.iter().any(|row| row[2] == "prior-local"));
}

#[test]
fn a_reused_cell_shows_its_evidence_and_measurements() {
    let detail = detail_of(false, "alpha/macos-latest/L1");
    assert!(detail.contains("refs/notes/ci-local/macos-latest"), "{detail}");
    assert!(detail.contains("7 test(s), 0 failed, 3s"), "{detail}");
}

#[test]
fn an_ordinary_executing_cell_needs_no_detail_line() {
    // "it will run" is the whole story; a reason per executing cell would bury
    // the two kinds a reviewer must actually reconcile.
    assert!(!details(&parsed(false))
        .iter()
        .any(|line| line.starts_with("`alpha/ubuntu-latest/L1`")));
    assert_eq!("pending", row(false, "alpha/ubuntu-latest/L1")[3]);
}

#[test]
fn an_accepted_gap_shows_its_owner_and_expiry() {
    let detail = detail_of(false, "alpha/windows-latest/L2");
    assert!(detail.contains("@yankeeinlondon"), "{detail}");
    assert!(detail.contains("2027-01-31"), "{detail}");
    assert!(detail.contains("no L2 backend on Windows"), "{detail}");
}

#[test]
fn a_prohibited_cell_names_its_constraint_and_the_owning_area() {
    let detail = detail_of(true, "alpha/wsl2-ubuntu/L1");
    assert!(detail.contains("do not rerun WSL for this branch"), "{detail}");
    assert!(detail.contains("ken"), "{detail}");
    assert!(detail.contains("2099-01-01"), "{detail}");

    let banner = prohibition_summary(&parsed(true)).expect("a prohibition banner");
    assert!(banner.contains("1 prohibited cell(s)"), "{banner}");
    assert!(banner.contains("pkg"), "{banner}");
}

#[test]
fn a_plan_with_no_prohibition_says_nothing_about_one() {
    assert!(prohibition_summary(&parsed(false)).is_none());
    assert!(!render(&parsed(false), &Terminal::new()).contains("prohibited"));
}

#[test]
fn rejected_evidence_is_reported_rather_than_dropped() {
    // A refused receipt must be visible, or a reviewer cannot tell a missing
    // receipt from a rejected one.
    let output = render(&parsed(false), &Terminal::new());
    assert!(output.contains("gate-inputs-changed"), "{output}");
}

#[test]
fn every_build_shows_its_key_producer_and_consumers() {
    let projected = build_rows(&parsed(false));
    assert_eq!(1, projected.len(), "{projected:?}");
    assert_eq!("0123456789abcdef", projected[0][0]);
    assert_eq!("alpha", projected[0][1]);
    assert_eq!("ubuntu-latest", projected[0][2]);
    assert_eq!("ubuntu-latest/L1", projected[0][3]);
}

#[test]
fn a_build_states_why_its_consumers_are_compatible() {
    let detail = build_details(&parsed(false))
        .into_iter()
        .find(|line| line.starts_with("`0123456789abcdef`"))
        .expect("a compatibility reason");
    assert!(detail.contains("x86_64-unknown-linux-gnu"), "{detail}");
    assert!(detail.contains("wsl2-ubuntu"), "{detail}");
}

#[test]
fn the_build_table_is_rendered_beside_the_cells_not_inside_them() {
    // Build plumbing is never a result cell: a reviewer must be able to see
    // that the owner produces no gate outcome of its own.
    let output = render(&parsed(false), &Terminal::new());
    assert!(output.contains("Build records"), "{output}");
    assert!(output.contains("0123456789abcdef"), "{output}");
    assert_eq!(
        4,
        rows(&parsed(false)).len(),
        "the cell table must still hold exactly the plan's cells"
    );
}

#[test]
fn a_plan_that_schedules_no_owner_says_nothing_about_builds() {
    // An all-reused plan derives no build. Rendering an empty table would
    // suggest an owner ran and produced nothing.
    let mut plan = parsed(false);
    plan.builds.clear();
    let output = render(&plan, &Terminal::new());
    assert!(!output.contains("Build records"), "{output}");
}

#[test]
fn a_plan_without_the_optional_fields_still_renders() {
    // `evidence_rejections` is optional in the schema; a plan resolved with no
    // evidence at all omits it, and the renderer must not require it.
    let minimal = format!(
        r#"{{"base":"{a}","head":"{b}","change_class":"documentation","areas":[],
        "cells":[],"prohibited_cells":[],"job_estimate":0}}"#,
        a = "a".repeat(40),
        b = "b".repeat(40)
    );
    let plan: Plan = serde_json::from_str(&minimal).expect("a minimal plan parses");
    let output = render(&plan, &Terminal::new());
    assert!(output.contains("documentation"), "{output}");
    assert!(rows(&plan).is_empty());
    assert!(details(&plan).is_empty());
}
