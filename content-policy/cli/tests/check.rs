//! `policy check` through the binary: output modes, exit codes,
//! `--needs-action`, and configuration precedence.

mod common;

// The library's list of the 20 migrated repository documents. Declared here,
// not in `common`, so CI's test-input index schedules only this binary when
// one of them changes.
#[path = "../../lib/tests/common/mod.rs"]
mod migrated;

use common::{LIFECYCLE, Run, Workspace};

fn check_json(workspace: &Workspace, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["check", "--json"];
    all.extend_from_slice(args);
    workspace.run(&all).success().json()
}

#[test]
fn json_output_is_the_report_alone() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    let run = workspace.run(&["check", "--json", "--at", "2026-12-28", "doc.md"]).success();
    let report = run.json();
    assert_eq!(report["document"], "doc.md", "the path as given, not canonicalized");
    assert_eq!(report["evaluated_at"], "2026-12-28T00:00:00Z");
    assert_eq!(report["status"], "stale");
    assert_eq!(report["action"], "refresh");
    assert_eq!(report["policy"]["source"], "declared");
    assert_eq!(report["results"][0]["result"], "triggered");
    assert_eq!(report["results"][0]["baseline"]["property"], "last_updated");
    assert_eq!(report["results"][1]["result"], "not_triggered");
    assert!(run.stderr.is_empty(), "stderr: {}", run.stderr);
}

#[test]
fn plain_output_has_a_summary_and_an_entry_table_without_escapes() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    let run = workspace
        .run(&["check", "--plain", "--at", "2026-12-28", "doc.md"])
        .success();
    assert!(!run.stdout.contains('\x1b'), "plain output is unstyled:\n{}", run.stdout);
    let first = run.stdout.lines().next().unwrap();
    assert!(first.starts_with("doc.md: stale, action refresh"), "{first}");
    assert!(first.contains("declared policy"), "{first}");
    for cell in ["Rule", "Result", "ValidFor(3mo,", "@last_updated)", "triggered", "2026-12-28", "archive"] {
        assert!(run.stdout.contains(cell), "missing {cell:?} in:\n{}", run.stdout);
    }
}

#[test]
fn default_output_is_styled_when_color_is_available() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    let output = workspace
        .command()
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1")
        .args(["check", "--at", "2026-12-28", "doc.md"])
        .output()
        .unwrap();
    let run = Run::from(output).success();
    assert!(run.stdout.contains("\x1b["), "styled output:\n{}", run.stdout);
    assert!(run.stdout.contains("stale"));
}

#[test]
fn a_stale_expired_or_unknown_report_still_exits_zero() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    workspace.write("missing.md", "---\nlast_updated:\n---\n");
    for (document, at, status) in [
        ("doc.md", "2026-12-27", "fresh"),
        ("doc.md", "2026-12-28", "stale"),
        ("doc.md", "2027-01-01", "expired"),
        ("missing.md", "2027-01-01", "unknown"),
    ] {
        let report = check_json(&workspace, &["--at", at, document]);
        assert_eq!(report["status"], status, "{document} at {at}");
    }
}

#[test]
fn needs_action_prints_true_for_a_confirmed_trigger() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    for at in ["2026-12-28", "2027-01-01"] {
        let run = workspace
            .run(&["check", "--needs-action", "--at", at, "doc.md"])
            .success();
        assert_eq!(run.stdout, "true\n", "at {at}");
    }
}

#[test]
fn needs_action_prints_false_for_a_fresh_document() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    let run = workspace
        .run(&["check", "--needs-action", "--at", "2026-12-27", "doc.md"])
        .success();
    assert_eq!(run.stdout, "false\n");
}

#[test]
fn needs_action_prints_unknown_when_nothing_is_confirmed_and_evidence_is_missing() {
    let workspace = Workspace::new();
    workspace.write("missing.md", "---\ntitle: No baseline\n---\n");
    let run = workspace
        .run(&["check", "--needs-action", "--at", "2026-12-27", "missing.md"])
        .success();
    assert_eq!(run.stdout, "unknown\n");
}

#[test]
fn needs_action_prints_true_when_a_trigger_is_known_but_another_entry_is_unknown() {
    let workspace = Workspace::new();
    workspace.write(
        "mixed.md",
        "---\ncontent_policy:\n  - ValidFor(3mo, @reviewed)\n  - TimeSensitive\n---\n",
    );
    let report = check_json(&workspace, &["mixed.md"]);
    assert_eq!(report["action_resolution_complete"], true);
    assert_eq!(report["evaluation_complete"], false);
    let run = workspace.run(&["check", "--needs-action", "mixed.md"]).success();
    assert_eq!(run.stdout, "true\n");
}

#[test]
fn needs_action_exits_one_only_for_errors() {
    let workspace = Workspace::new();
    workspace.write("bad.md", "---\ncontent_policy: []\n---\n");
    let run = workspace.run(&["check", "--needs-action", "bad.md"]);
    assert_eq!(run.code, 1);
    assert!(run.stdout.is_empty(), "no answer is printed: {}", run.stdout);
    assert!(run.stderr.contains("Evergreen"), "{}", run.stderr);
}

#[test]
fn invalid_declarations_exit_one_with_diagnostics_on_stderr_even_in_json_mode() {
    let workspace = Workspace::new();
    workspace.write(
        "bad.md",
        "---\ncontent_policy:\n  - Duration(3mo)\n  - rule: ValidFor(3mo)\n    action: delete\n---\n",
    );
    for mode in ["--json", "--plain"] {
        let run = workspace.run(&["check", mode, "bad.md"]);
        assert_eq!(run.code, 1, "{mode}");
        assert!(run.stdout.is_empty(), "{mode} stdout: {}", run.stdout);
        assert!(run.stderr.contains("bad.md: invalid content policy"), "{}", run.stderr);
        assert!(run.stderr.contains("entry 1: "), "every entry is listed: {}", run.stderr);
        assert!(run.stderr.contains("entry 2 action: "), "{}", run.stderr);
    }
}

#[test]
fn unreadable_files_and_malformed_frontmatter_exit_one() {
    let workspace = Workspace::new();
    workspace.write("malformed.md", "---\ntitle: [unclosed\n---\n");
    workspace.write("template.md", "---\ntitle: {{ page.title }}\n---\n");
    workspace.write("duplicate.md", "---\nlast_updated: 2026-01-01\nlast_updated: 2026-02-01\n---\n");

    let missing = workspace.run(&["check", "--json", "absent.md"]);
    assert_eq!(missing.code, 1);
    assert!(missing.stderr.starts_with("error: absent.md: "), "{}", missing.stderr);
    assert!(missing.stdout.is_empty());

    let malformed = workspace.run(&["check", "--json", "malformed.md"]);
    assert_eq!(malformed.code, 1);
    assert!(malformed.stderr.contains("malformed frontmatter"), "{}", malformed.stderr);

    let template = workspace.run(&["check", "template.md"]);
    assert_eq!(template.code, 1);
    assert!(template.stderr.contains("expression protection"), "{}", template.stderr);

    let duplicate = workspace.run(&["check", "duplicate.md"]);
    assert_eq!(duplicate.code, 1);
    assert!(duplicate.stderr.contains("last_updated"), "{}", duplicate.stderr);
}

#[test]
fn usage_errors_exit_two() {
    let workspace = Workspace::new();
    workspace.write("doc.md", LIFECYCLE);
    for args in [
        &["check"][..],
        &["check", "--at", "2026-9-1", "doc.md"],
        &["check", "--at", "2026-02-30", "doc.md"],
        &["check", "--json", "--plain", "doc.md"],
        &["check", "--json", "--needs-action", "doc.md"],
        &["check", "--default-policy", "", "doc.md"],
        &["check", "--default-policy", "Duration(3mo)", "doc.md"],
        &["check", "--key", "", "doc.md"],
        &["frobnicate"],
    ] {
        let run = workspace.run(args);
        assert_eq!(run.code, 2, "{args:?}: {}", run.stderr);
    }
}

#[test]
fn tab_indented_frontmatter_reports_a_status_and_a_warning() {
    let workspace = Workspace::new();
    workspace.write(
        "tabs.md",
        "---\nlast_updated: 2026-09-01\ncontent_policy:\n\t- ValidFor(3mo)\n---\nBody\n",
    );
    let report = check_json(&workspace, &["--at", "2026-10-01", "tabs.md"]);
    assert_eq!(report["status"], "fresh");
    assert_eq!(report["warnings"][0]["code"], "tab_indentation_repaired");

    let run = workspace
        .run(&["check", "--plain", "--at", "2026-10-01", "tabs.md"])
        .success();
    assert!(run.stdout.contains("warning: the frontmatter is indented with tabs"), "{}", run.stdout);
    assert!(run.stderr.is_empty(), "the warning is part of the report, not stderr");
}

// --- Configuration precedence: flag, then environment, then built-in ------

const TWO_KEYS: &str = "---
last_updated: 2026-09-28
content_policy:
  - TimeSensitive
alt_policy:
  - Evergreen
---
";

#[test]
fn key_built_in_value_is_content_policy() {
    let workspace = Workspace::new();
    workspace.write("doc.md", TWO_KEYS);
    let report = check_json(&workspace, &["doc.md"]);
    assert_eq!(report["results"][0]["rule"], "TimeSensitive");
}

#[test]
fn key_environment_variable_overrides_the_built_in_value() {
    let workspace = Workspace::new().env("CONTENT_POLICY_KEY", "alt_policy");
    workspace.write("doc.md", TWO_KEYS);
    let report = check_json(&workspace, &["doc.md"]);
    assert_eq!(report["results"][0]["rule"], "Evergreen");
}

#[test]
fn key_flag_overrides_the_environment_variable() {
    let workspace = Workspace::new().env("CONTENT_POLICY_KEY", "alt_policy");
    workspace.write("doc.md", TWO_KEYS);
    let report = check_json(&workspace, &["--key", "missing_key", "doc.md"]);
    assert_eq!(report["policy"]["source"], "defaulted", "the flag's key is absent");
}

const UNDECLARED: &str = "---\nlast_updated: 2026-09-28\nreviewed: 2025-01-01\n---\n";

#[test]
fn default_policy_built_in_value_is_valid_for_six_months() {
    let workspace = Workspace::new();
    workspace.write("doc.md", UNDECLARED);
    let report = check_json(&workspace, &["--at", "2026-12-28", "doc.md"]);
    assert_eq!(report["policy"]["source"], "defaulted");
    assert_eq!(report["results"][0]["rule"], "ValidFor(6mo)");
    assert_eq!(report["status"], "fresh");
}

#[test]
fn default_policy_environment_variable_overrides_the_built_in_value() {
    let workspace = Workspace::new().env("CONTENT_POLICY_DEFAULT", "TimeSensitive");
    workspace.write("doc.md", UNDECLARED);
    let report = check_json(&workspace, &["doc.md"]);
    assert_eq!(report["results"][0]["rule"], "TimeSensitive");
    assert_eq!(report["status"], "stale", "fail-closed default");
}

#[test]
fn default_policy_flag_overrides_the_environment_variable() {
    let workspace = Workspace::new().env("CONTENT_POLICY_DEFAULT", "TimeSensitive");
    workspace.write("doc.md", UNDECLARED);
    let report = check_json(
        &workspace,
        &[
            "--default-policy",
            r#"["ValidFor(1yr)", {rule: "ValidUntil(2027-01-01)", action: archive}]"#,
            "--at",
            "2026-12-28",
            "doc.md",
        ],
    );
    assert_eq!(report["results"][0]["rule"], "ValidFor(1yr)");
    assert_eq!(report["results"][1]["action"], "archive");
    assert_eq!(report["status"], "fresh");
}

#[test]
fn an_invalid_default_policy_environment_variable_is_a_usage_error() {
    for value in ["", "Duration(3mo)", "[ValidFor(3mo, 2026-01-01)]"] {
        let workspace = Workspace::new().env("CONTENT_POLICY_DEFAULT", value);
        workspace.write("doc.md", UNDECLARED);
        let run = workspace.run(&["check", "doc.md"]);
        assert_eq!(run.code, 2, "{value:?}: {}", run.stderr);
        assert!(run.stderr.contains("--default-policy"), "{}", run.stderr);
    }
}

const TWO_DATES: &str = "---
last_updated: 2026-09-28
reviewed: 2026-01-01
content_policy:
  - ValidFor(3mo)
---
";

#[test]
fn date_property_built_in_value_is_last_updated() {
    let workspace = Workspace::new();
    workspace.write("doc.md", TWO_DATES);
    let report = check_json(&workspace, &["--at", "2026-12-01", "doc.md"]);
    assert_eq!(report["results"][0]["baseline"]["property"], "last_updated");
    assert_eq!(report["status"], "fresh");
}

#[test]
fn date_property_environment_variable_overrides_the_built_in_value() {
    let workspace = Workspace::new().env("CONTENT_POLICY_DATE_PROPERTY", "reviewed");
    workspace.write("doc.md", TWO_DATES);
    let report = check_json(&workspace, &["--at", "2026-12-01", "doc.md"]);
    assert_eq!(report["results"][0]["baseline"]["property"], "reviewed");
    assert_eq!(report["status"], "stale");
}

#[test]
fn date_property_flag_overrides_the_environment_variable() {
    let workspace = Workspace::new().env("CONTENT_POLICY_DATE_PROPERTY", "reviewed");
    workspace.write("doc.md", TWO_DATES);
    let report = check_json(
        &workspace,
        &["--date-property", "checked", "--at", "2026-12-01", "doc.md"],
    );
    assert_eq!(report["results"][0]["baseline"]["property"], "checked");
    assert_eq!(report["status"], "unknown");
}

#[test]
fn the_migrated_repository_documents_check_without_diagnostics() {
    let workspace = Workspace::new();
    for (name, bytes) in migrated::MIGRATED_DOCUMENTS {
        let file = format!("{}.md", name.replace('/', "-"));
        workspace.write(&file, bytes);
        let run = workspace.run(&["check", "--json", "--at", "2026-09-29", &file]);
        assert_eq!(run.code, 0, "{name}: {}", run.stderr);
        assert!(run.stderr.is_empty(), "{name}: {}", run.stderr);
        let report = run.json();
        assert_eq!(report["policy"]["source"], "declared", "{name}");
        assert!(
            report["results"][0]["rule"].as_str().unwrap().starts_with("ValidFor("),
            "{name}"
        );
    }
}
