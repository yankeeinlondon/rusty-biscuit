//! `policy renew` through the binary: preview, `--write`, output modes, and
//! errors that write nothing.

mod common;

use common::{LIFECYCLE, Workspace};

#[test]
fn the_preview_labels_a_first_capture_new_baseline_and_writes_nothing() {
    let workspace = Workspace::new();
    let original = "---\ntitle: Note\ncontent_policy:\n  - ValidFor(3mo)\n---\nBody\n";
    workspace.write("doc.md", original);
    let run = workspace
        .run(&["renew", "--plain", "--today", "2026-09-29", "doc.md"])
        .success();
    assert!(run.stdout.starts_with("doc.md: renewal on 2026-09-29 (preview; pass --write to apply)"), "{}", run.stdout);
    assert!(run.stdout.contains("new baseline"), "{}", run.stdout);
    assert!(run.stdout.contains("last_updated"), "{}", run.stdout);
    assert!(!run.stdout.contains('\x1b'), "{}", run.stdout);
    assert_eq!(workspace.read("doc.md"), original, "a preview never writes");
}

#[test]
fn write_applies_the_plan() {
    let workspace = Workspace::new();
    workspace.write("doc.md", "---\ntitle: Note\ncontent_policy:\n  - ValidFor(3mo)\n---\nBody\n");
    let run = workspace
        .run(&["renew", "--plain", "--write", "--today", "2026-09-29", "doc.md"])
        .success();
    assert!(run.stdout.contains("(written)"), "{}", run.stdout);
    assert_eq!(
        workspace.read("doc.md"),
        "---\ntitle: Note\ncontent_policy:\n  - ValidFor(3mo)\nlast_updated: 2026-09-29\n---\nBody\n"
    );
}

#[test]
fn json_output_is_the_plan_alone() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    let run = workspace
        .run(&["renew", "--json", "--today", "2026-12-29", "doc.md"])
        .success();
    let plan = run.json();
    assert_eq!(plan["document"], "doc.md");
    assert_eq!(plan["update_date"], "2026-12-29");
    assert_eq!(plan["changes"][0]["target"]["kind"], "property");
    assert_eq!(plan["changes"][0]["target"]["name"], "last_updated");
    assert_eq!(plan["changes"][0]["kind"], "renewed");
    assert_eq!(plan["changes"][0]["previous"], "2026-09-28");
    assert_eq!(plan["tab_repair"], serde_json::json!([]));
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert_eq!(workspace.read("doc.md"), LIFECYCLE);
}

#[test]
fn nothing_to_renew_is_not_an_error() {
    let workspace = Workspace::new();
    for (name, policy) in [
        ("evergreen.md", "  - Evergreen\n"),
        ("sensitive.md", "  - TimeSensitive\n"),
        ("deadline.md", "  - ValidUntil(2027-01-01)\n"),
    ] {
        let original = format!("---\ncontent_policy:\n{policy}---\n");
        workspace.write(name, &original);
        let run = workspace.run(&["renew", "--plain", "--write", name]).success();
        assert_eq!(run.stdout, format!("{name}: nothing to renew; the policy has no rule with a baseline to advance\n"));
        assert_eq!(workspace.read(name), original);

        let plan = workspace.run(&["renew", "--json", name]).success().json();
        assert_eq!(plan["changes"], serde_json::json!([]), "{name}");
    }
}

#[test]
fn the_tab_repair_is_listed_separately_and_applied_with_write() {
    let workspace = Workspace::new();
    workspace.write(
        "tabs.md",
        "---\nlast_updated: 2026-09-01\ncontent_policy:\n\t- ValidFor(3mo)\n---\nBody\n",
    );
    let preview = workspace
        .run(&["renew", "--plain", "--today", "2026-10-01", "tabs.md"])
        .success();
    let table_end = preview.stdout.rfind('┘').expect("a change table");
    assert!(
        preview.stdout[table_end..].contains("tab repair: 1 frontmatter line(s)"),
        "the repair is its own item after the date changes:\n{}",
        preview.stdout
    );

    let plan = workspace
        .run(&["renew", "--json", "--today", "2026-10-01", "tabs.md"])
        .success()
        .json();
    assert_eq!(plan["tab_repair"].as_array().unwrap().len(), 1);

    workspace
        .run(&["renew", "--write", "--today", "2026-10-01", "tabs.md"])
        .success();
    assert_eq!(
        workspace.read("tabs.md"),
        "---\nlast_updated: 2026-10-01\ncontent_policy:\n  - ValidFor(3mo)\n---\nBody\n"
    );
    let report = workspace
        .run(&["check", "--json", "--at", "2026-10-01", "tabs.md"])
        .success()
        .json();
    assert_eq!(report["warnings"], serde_json::json!([]), "repaired on disk");
}

#[test]
fn refusals_conflicts_and_bad_evidence_exit_one_and_write_nothing() {
    let workspace = Workspace::new();
    let cases = [
        (
            "flow.md",
            "---\ncontent_policy: [\"ValidFor(3mo, 2026-01-01)\"]\n---\n",
            "  - ValidFor(3mo, 2026-01-01)",
        ),
        (
            "deadline.md",
            "---\nreviewed: 2026-01-01\ncontent_policy:\n  - ValidFor(3mo, @reviewed)\n  - ValidUntil(@reviewed)\n---\n",
            "reviewed",
        ),
        (
            "malformed-baseline.md",
            "---\nlast_updated: soon\ncontent_policy:\n  - ValidFor(3mo)\n---\n",
            "last_updated",
        ),
        ("invalid.md", "---\ncontent_policy: []\n---\n", "Evergreen"),
    ];
    for (name, original, needle) in cases {
        workspace.write(name, original);
        for mode in ["--plain", "--json"] {
            let run = workspace.run(&["renew", mode, "--write", "--today", "2026-09-29", name]);
            assert_eq!(run.code, 1, "{name} {mode}: {}", run.stdout);
            assert!(run.stdout.is_empty(), "{name} {mode}: {}", run.stdout);
            assert!(run.stderr.contains(needle), "{name} {mode}: {}", run.stderr);
            assert_eq!(workspace.read(name), original, "{name}: nothing written");
        }
    }
}

#[test]
fn a_future_update_date_is_rejected() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    let run = workspace.run(&["renew", "--write", "--today", "2026-12-28", "--on", "2026-12-29", "doc.md"]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("after today's UTC date 2026-12-28"), "{}", run.stderr);
    assert_eq!(workspace.read("doc.md"), LIFECYCLE);
}

#[test]
fn renew_reads_the_shared_configuration() {
    let workspace = Workspace::new().env("CONTENT_POLICY_DATE_PROPERTY", "reviewed");
    workspace.write("doc.md", "---\nreviewed: 2026-01-01\n---\n");
    let plan = workspace
        .run(&["renew", "--json", "--today", "2026-09-29", "doc.md"])
        .success()
        .json();
    assert_eq!(plan["changes"][0]["target"]["name"], "reviewed");

    let plan = workspace
        .run(&["renew", "--json", "--today", "2026-09-29", "--default-policy", "Evergreen", "doc.md"])
        .success()
        .json();
    assert_eq!(plan["changes"], serde_json::json!([]));
}

#[test]
fn the_today_option_is_hidden_from_help() {
    let workspace = Workspace::new();
    let run = workspace.run(&["renew", "--help"]).success();
    assert!(run.stdout.contains("--write"));
    assert!(!run.stdout.contains("--today"), "{}", run.stdout);
}
