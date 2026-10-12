//! The spec's lifecycle example, step by step, through the `policy` binary
//! (AC 7). `--at` fixes the evaluation time and the hidden `--today` fixes
//! renewal's current date.

mod common;

use common::{LIFECYCLE, Workspace};

fn check(workspace: &Workspace, at: &str) -> serde_json::Value {
    workspace
        .run(&["check", "--json", "--at", at, "doc.md"])
        .success()
        .json()
}

fn results(report: &serde_json::Value) -> Vec<&str> {
    report["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["result"].as_str().unwrap())
        .collect()
}

#[test]
fn the_lifecycle_example_through_the_cli() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);

    // 1. Before the computed date.
    let report = check(&workspace, "2026-12-27");
    assert_eq!(report["status"], "fresh");
    assert_eq!(report["action"], serde_json::Value::Null);

    // 2. On the computed date, 3mo after 2026-09-28.
    let report = check(&workspace, "2026-12-28");
    assert_eq!(report["status"], "stale");
    assert_eq!(report["action"], "refresh");
    assert_eq!(results(&report), ["triggered", "not_triggered"]);

    // 3. Renew; only `last_updated` changes.
    workspace
        .run(&["renew", "--on", "2026-12-29", "--write", "--today", "2026-12-29", "doc.md"])
        .success();
    assert_eq!(
        workspace.read("doc.md"),
        LIFECYCLE.replace("last_updated: 2026-09-28", "last_updated: 2026-12-29")
    );

    // 4. Fresh again; the new due date is 2027-03-29.
    let report = check(&workspace, "2026-12-29");
    assert_eq!(report["status"], "fresh");
    assert_eq!(report["results"][0]["due"], "2027-03-29");

    // 5. The baseline is now in that date's future.
    let report = check(&workspace, "2026-12-28");
    assert_eq!(report["status"], "unknown");
    assert_eq!(report["results"][0]["unknown_reason"], "inconsistent_baseline");

    // 6. The fixed deadline passes.
    let report = check(&workspace, "2027-01-01");
    assert_eq!(report["status"], "expired");
    assert_eq!(report["action"], "archive");
    assert_eq!(results(&report), ["not_triggered", "triggered"]);

    // 7. Renewal never moves a deadline.
    let before = workspace.read("doc.md");
    workspace
        .run(&["renew", "--on", "2027-01-01", "--write", "--today", "2027-01-01", "doc.md"])
        .success();
    assert_eq!(
        workspace.read("doc.md"),
        before.replace("last_updated: 2026-12-29", "last_updated: 2027-01-01")
    );
    let report = check(&workspace, "2027-01-01");
    assert_eq!(report["status"], "expired");
    assert_eq!(report["action"], "archive");

    // 8. A stronger action on the deadline entry.
    let edited = workspace.read("doc.md").replace("action: archive", "action: remove");
    workspace.write("doc.md", edited);
    let report = check(&workspace, "2027-01-01");
    assert_eq!(report["status"], "expired");
    assert_eq!(report["action"], "remove");
    assert_eq!(report["results"].as_array().unwrap().len(), 2);
    assert_eq!(results(&report), ["not_triggered", "triggered"]);
}
