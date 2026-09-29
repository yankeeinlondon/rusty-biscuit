//! `FileChanged` through the `policy` binary with the bundled file adapter:
//! capture, match, edit the watched file, triggered, renew, fresh, and the
//! boundary as the starting directory decides it.
//!
//! The workspace is a temporary directory outside any repository and is the
//! process's working directory, so it is the document tree root.

mod common;

use std::fs;

use common::Workspace;

const GUIDE: &str = "---
title: Guide
content_policy:
  - FileChanged(../src/config.rs, @config_fingerprint)
---

Body.
";

fn workspace() -> Workspace {
    let workspace = Workspace::new();
    fs::create_dir_all(workspace.path().join("docs")).unwrap();
    fs::create_dir_all(workspace.path().join("src")).unwrap();
    workspace.write("src/config.rs", "pub fn config() {}\n");
    workspace.write("docs/guide.md", GUIDE);
    workspace
}

fn check(workspace: &Workspace) -> serde_json::Value {
    workspace
        .run(&["check", "--json", "--at", "2026-09-29", "docs/guide.md"])
        .success()
        .json()
}

fn renew(workspace: &Workspace, write: bool) -> common::Run {
    let mut args = vec!["renew", "--today", "2026-09-29", "docs/guide.md"];
    if write {
        args.insert(1, "--write");
    }
    workspace.run(&args)
}

#[test]
fn the_file_changed_lifecycle_through_the_cli() {
    let workspace = workspace();

    // Nothing captured yet.
    let report = check(&workspace);
    assert_eq!(report["status"], "unknown");
    assert_eq!(report["results"][0]["unknown_reason"], "missing_baseline");
    assert_eq!(report["results"][0]["file"]["path"], "../src/config.rs");

    // The preview labels a first capture and writes nothing.
    let preview = workspace
        .run(&["renew", "--plain", "--today", "2026-09-29", "docs/guide.md"])
        .success();
    assert!(preview.stdout.contains("config_fingerprint"), "{}", preview.stdout);
    assert!(preview.stdout.contains("new baseline"), "{}", preview.stdout);
    assert!(preview.stdout.contains("blake3-lf:"), "{}", preview.stdout);
    assert_eq!(workspace.read("docs/guide.md"), GUIDE);

    // Capture: the full fingerprint is written.
    let plan = renew(&workspace, true).success();
    assert!(plan.stdout.contains("written"), "{}", plan.stdout);
    let captured = workspace.read("docs/guide.md");
    let line = captured
        .lines()
        .find(|line| line.starts_with("config_fingerprint: blake3-lf:"))
        .unwrap_or_else(|| panic!("no fingerprint line:\n{captured}"));
    assert_eq!(line.len(), "config_fingerprint: blake3-lf:".len() + 64);
    assert_eq!(captured, GUIDE.replace("---\n\nBody", &format!("{line}\n---\n\nBody")));

    // Match.
    let report = check(&workspace);
    assert_eq!(report["status"], "fresh");
    assert_eq!(report["results"][0]["result"], "not_triggered");

    // Edit the watched file: triggered.
    workspace.write("src/config.rs", "pub fn config() { changed() }\n");
    let report = check(&workspace);
    assert_eq!((report["status"].as_str(), report["action"].as_str()), (Some("stale"), Some("refresh")));
    assert_eq!(report["results"][0]["reason"], "Watched file changed");
    let needs = workspace
        .run(&["check", "--needs-action", "--at", "2026-09-29", "docs/guide.md"])
        .success();
    assert_eq!(needs.stdout, "true\n");

    // Renew: fresh again, with a new value in place.
    let json = workspace
        .run(&["renew", "--json", "--write", "--today", "2026-09-29", "docs/guide.md"])
        .success()
        .json();
    assert_eq!(json["changes"][0]["kind"], "renewed");
    assert_eq!(json["changes"][0]["previous"], line["config_fingerprint: ".len()..]);
    assert_eq!(check(&workspace)["status"], "fresh");

    // Delete it: triggered, and renewal refuses without writing.
    fs::remove_file(workspace.path().join("src/config.rs")).unwrap();
    let report = check(&workspace);
    assert_eq!(report["results"][0]["reason"], "Source removed");
    let before = workspace.read("docs/guide.md");
    let refused = renew(&workspace, true);
    assert_eq!(refused.code, 1, "{}", refused.stdout);
    assert!(refused.stderr.contains("missing evidence"), "{}", refused.stderr);
    assert!(refused.stderr.contains("`../src/config.rs` is missing"), "{}", refused.stderr);
    assert_eq!(workspace.read("docs/guide.md"), before);
}

/// A `FileChanged` rule and a fingerprint are long unbroken words; the
/// tables still fit the 80 columns a piped `policy` renders at, breaking at
/// `/` and `_` only when they must.
#[test]
fn file_rules_and_fingerprints_fit_an_80_column_table() {
    let workspace = workspace();
    renew(&workspace, true).success();
    workspace.write("src/config.rs", "pub fn config() { changed() }\n");
    let check = workspace
        .run(&["check", "--plain", "--at", "2026-09-29", "docs/guide.md"])
        .success();
    let preview = workspace
        .run(&["renew", "--plain", "--today", "2026-09-29", "docs/guide.md"])
        .success();
    for output in [&check.stdout, &preview.stdout] {
        assert!(!output.contains("could not be rendered"), "{output}");
        let table = output.lines().filter(|line| line.starts_with(['┌', '│', '├', '└']));
        assert!(table.clone().count() > 3 && table.clone().all(|line| line.chars().count() <= 80), "{output}");
        assert!(output.contains("blake3-"), "{output}");
        assert!(output.contains('…'), "{output}");
    }
    assert!(check.stdout.contains("FileChanged(../src/"), "{}", check.stdout);
    assert!(check.stdout.contains("triggered"), "{}", check.stdout);
    assert!(preview.stdout.contains("renewed"), "{}", preview.stdout);
    // The abbreviation keeps 8 hex digits; `--json` keeps all 64.
    let json = workspace
        .run(&["renew", "--json", "--today", "2026-09-29", "docs/guide.md"])
        .success()
        .json();
    let full = json["changes"][0]["value"].as_str().unwrap();
    assert_eq!(full.len(), "blake3-lf:".len() + 64);
    let digest = &full["blake3-lf:".len()..];
    assert!(preview.stdout.contains(&format!("{}…", &digest[..8])), "{}", preview.stdout);
}

/// The spec's two-directory example: outside a repository the boundary is
/// the directory the command starts in.
#[test]
fn the_boundary_depends_on_the_starting_directory_outside_a_repository() {
    let workspace = Workspace::new();
    fs::create_dir_all(workspace.path().join("notes")).unwrap();
    fs::create_dir_all(workspace.path().join("drafts")).unwrap();
    workspace.write("drafts/x.md", "draft\n");
    workspace.write(
        "notes/doc.md",
        "---\ncontent_policy:\n  - FileChanged(../drafts/x.md, @draft_fingerprint)\n---\n",
    );

    // From the workspace: `policy check notes/doc.md` is valid.
    let valid = workspace.run(&["check", "--json", "--at", "2026-09-29", "notes/doc.md"]).success();
    assert_eq!(valid.json()["results"][0]["unknown_reason"], "missing_baseline");

    // From `notes`: `policy check doc.md` escapes the boundary.
    let output = workspace
        .command()
        .current_dir(workspace.path().join("notes"))
        .args(["check", "--json", "--at", "2026-09-29", "doc.md"])
        .output()
        .unwrap();
    let invalid = common::Run::from(output);
    assert_eq!(invalid.code, 1, "{}", invalid.stdout);
    assert!(invalid.stdout.is_empty());
    assert!(invalid.stderr.contains("resolves outside the boundary"), "{}", invalid.stderr);
}

#[test]
fn invalid_file_rules_exit_one_with_the_reason() {
    let workspace = Workspace::new();
    for (rule, reason) in [
        ("FileChanged(src/config.rs)", "one-argument form is not supported"),
        ("FileChanged(/etc/hosts, @fp)", "is absolute"),
        ("FileChanged(src\\\\config.rs, @fp)", "write `/`"),
        ("\"FileChanged(a,b.md, @fp)\"", "`,` or `)`"),
    ] {
        workspace.write("doc.md", format!("---\ncontent_policy:\n  - {rule}\n---\n"));
        let run = workspace.run(&["check", "--at", "2026-09-29", "doc.md"]);
        assert_eq!(run.code, 1, "{rule}: {}", run.stdout);
        assert!(run.stderr.contains(reason), "{rule}: {}", run.stderr);
    }
}
