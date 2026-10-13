//! The built `md` checks `content_policy` with nothing but the test document:
//! a launch directory outside the checkout, fixture-owned home, config,
//! cache, and temp, and no schema file or schema variable reaching the
//! child. The embedded baseline is the only source of the policy's typing.
//!
//! `schema_validate_baseline_selection` holds `md schema validate` to each
//! [`ACCEPTANCE_REJECTIONS`] row; this file holds `md compose` to the same
//! rows and proves host schema configuration cannot stand in for the
//! embedded schemas. Each compose row spawns `md` once, so each is its own
//! `#[test]`; the whole declaration table goes through one validate run.

use crate::common;
use crate::content_policy_fixtures::{
    self as policy, ACCEPTANCE_REJECTIONS, DECLARATIONS, LOAD_FAILURES, MIXED_POLICY_FRONTMATTER,
    declaration,
};

use common::CliProcessFixture;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Writes `text` as the launch directory's only file.
fn only_document(process: &CliProcessFixture, text: &str) -> PathBuf {
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, text).unwrap();
    let entries: Vec<_> = std::fs::read_dir(process.cwd())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, ["doc.md"], "the launch directory holds only the test document");
    document
}

/// Runs `md compose` on `document`, returning the exit code, stdout, and
/// stderr with the error block's wrapped lines joined into one.
fn compose(process: &CliProcessFixture, document: &Path) -> (Option<i32>, String, String) {
    let output = process
        .command_builder()
        .plain_terminal(400, 50)
        .build()
        .arg("compose")
        .arg(document)
        .output()
        .expect("run md compose");
    (
        output.status.code(),
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        plain(&String::from_utf8(output.stderr).expect("utf-8 stderr"))
            .lines()
            .map(|line| line.trim_start_matches('┃').trim())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// `text` without SGR (`ESC [ … m`) and OSC (`ESC ] … ESC \`) sequences;
/// the error block styles stderr even when `NO_COLOR` is set.
fn plain(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            plain.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                chars.by_ref().find(|c| *c == 'm');
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\u{7}' || (c == '\u{1b}' && chars.next() == Some('\\')) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    plain
}

fn validate_json(process: &CliProcessFixture, declared: &[(&str, &str)], document: &Path) -> (Option<i32>, Value) {
    let mut builder = process.command_builder();
    for (key, value) in declared {
        builder = builder.application_input(key, value);
    }
    let output = builder
        .build()
        .args(["schema", "validate", "--format", "json"])
        .arg(document)
        .output()
        .expect("run md schema validate");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let record = serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!(
            "one JSON record expected ({error}); stdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code(), record)
}

fn problem_paths(record: &Value) -> Vec<&str> {
    record["problems"]
        .as_array()
        .expect("a validated record carries `problems`")
        .iter()
        .map(|problem| problem["path"].as_str().expect("a problem path"))
        .collect()
}

#[test]
fn compose_accepts_the_authoring_example_unchanged() {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let document = only_document(&process, &format!("---\n{MIXED_POLICY_FRONTMATTER}---\nBody\n"));

    let (code, stdout, stderr) = compose(&process, &document);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout.trim(), "Body", "{stdout}");
}

/// Composes one [`ACCEPTANCE_REJECTIONS`] row: compose fails before
/// producing output, naming each expected problem's instance path (a missing
/// field after its parent's path) at its authored line and column.
fn assert_compose_rejects(case_name: &str) {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let case = declaration(case_name);
    assert_eq!(case.schema, policy::Verdict::Reject, "`{case_name}` is a rejection case");
    let rendered = policy::render(case.declaration).remove(0);
    let process = CliProcessFixture::new();
    let document = only_document(&process, &rendered.document());

    let (code, stdout, stderr) = compose(&process, &document);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("schema validation failed"), "{stderr}");
    for expected in case.problems {
        let (line, column) = rendered.start(expected.at);
        let instance = expected.path.trim_start_matches('/');
        let site = match expected.kind {
            "missing" => format!("missing {instance}/"),
            kind => format!("{kind} {instance}: "),
        };
        assert!(stderr.contains(&site), "no `{site}` for {}: {stderr}", expected.path);
        assert!(stderr.contains(&format!(" at {line}:{column}")), "{} at {line}:{column}: {stderr}", expected.path);
    }
}

macro_rules! compose_rejections {
    ($($test:ident => $case:literal,)+) => {
        $(
            #[test]
            fn $test() {
                assert_compose_rejects($case);
            }
        )+

        const COMPOSE_REJECTION_ROWS: &[&str] = &[$($case),+];
    };
}

compose_rejections! {
    compose_rejects_an_unknown_rule => "duration_rule_name",
    compose_rejects_an_unknown_action => "unknown_action",
    compose_rejects_an_empty_policy_list => "empty_list",
    compose_rejects_a_policy_that_is_not_a_list => "whole_string",
    compose_rejects_a_long_form_entry_without_rule => "missing_rule",
    compose_rejects_a_long_form_entry_with_null_rule => "null_rule",
}

#[test]
fn compose_rejection_tests_cover_every_acceptance_row_once() {
    assert_eq!(COMPOSE_REJECTION_ROWS, ACCEPTANCE_REJECTIONS);
}

/// A host whose shell exports a permissive `BASELINE_SCHEMA`, a truthy
/// `DARKMATTER_NO_BASELINE_SCHEMA`, and a `SCHEMAS_DIR` still gets the
/// embedded baseline's verdict from both commands. Declared to the child,
/// each of the first two would accept the invalid policy, so the scrub is
/// what the verdict rests on; `md_process_fixture` records that
/// `SCHEMAS_DIR` itself never reaches the child.
#[test]
fn host_schema_configuration_cannot_stand_in_for_the_embedded_schemas() {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let host = process.workspace_path().join("host");
    std::fs::create_dir_all(host.join("schemas")).unwrap();
    let permissive = host.join("baseline.yaml");
    std::fs::write(&permissive, "$schema:\n  content_policy: unknown\n").unwrap();
    let permissive = permissive.to_string_lossy().into_owned();
    let schemas_dir = host.join("schemas").to_string_lossy().into_owned();

    let case = declaration("duration_rule_name");
    let rendered = policy::render(case.declaration).remove(0);
    let document = only_document(&process, &rendered.document());

    for declared in [("BASELINE_SCHEMA", permissive.as_str()), ("DARKMATTER_NO_BASELINE_SCHEMA", "1")] {
        let (code, record) = validate_json(&process, &[declared], &document);
        assert_eq!(code, Some(0), "declared {declared:?} accepts the policy: {record}");
    }

    // Each test runs in its own nextest process, so these exports reach no
    // other test.
    unsafe {
        std::env::set_var("BASELINE_SCHEMA", &permissive);
        std::env::set_var("DARKMATTER_NO_BASELINE_SCHEMA", "1");
        std::env::set_var("SCHEMAS_DIR", &schemas_dir);
    }

    let (code, record) = validate_json(&process, &[], &document);
    assert_eq!(code, Some(1), "{record}");
    assert_eq!(problem_paths(&record), ["/content_policy/0"], "{record}");
    let (code, _, stderr) = compose(&process, &document);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("invalid content_policy/0: "), "{stderr}");

    let valid = only_document(&process, &format!("---\n{MIXED_POLICY_FRONTMATTER}---\nBody\n"));
    let (code, record) = validate_json(&process, &[], &valid);
    assert_eq!(code, Some(0), "{record}");
    let (code, _, stderr) = compose(&process, &valid);
    assert_eq!(code, Some(0), "{stderr}");
}

/// Every other shared declaration (`half` 0 or 1) written in
/// `presentation`, through one `md schema validate` run: each record carries
/// the row's verdict and exactly its expected problems, by path, kind, and
/// authored position.
fn assert_declarations_validate(presentation: policy::Presentation, half: usize) {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let mut expected = std::collections::BTreeMap::new();
    for case in DECLARATIONS.iter().skip(half).step_by(2) {
        for rendered in policy::render(case.declaration) {
            if rendered.presentation != presentation {
                continue;
            }
            let file = format!("{}-{:?}.md", case.name, rendered.presentation);
            std::fs::write(process.cwd().join(&file), rendered.document()).unwrap();
            let problems: std::collections::BTreeSet<_> = case
                .problems
                .iter()
                .map(|problem| {
                    let (line, column) = rendered.start(problem.at);
                    (problem.path.to_string(), problem.kind.to_string(), u64::from(line), u64::from(column))
                })
                .collect();
            expected.insert(file, (case.schema == policy::Verdict::Accept, problems));
        }
    }

    let output = process
        .command_builder()
        .build()
        .args(["schema", "validate", "--format", "json"])
        .args(expected.keys())
        .output()
        .expect("run md schema validate");
    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let mut failures = Vec::new();
    let mut seen = 0;
    for line in stdout.lines() {
        let record: Value = serde_json::from_str(line).unwrap_or_else(|error| panic!("{error}: {line}"));
        let file = record["file"].as_str().expect("a record names its file");
        let (valid, problems) = &expected[file];
        let actual: std::collections::BTreeSet<_> = record["problems"]
            .as_array()
            .expect("problems")
            .iter()
            .map(|problem| {
                (
                    problem["path"].as_str().unwrap_or_default().to_string(),
                    problem["kind"].as_str().unwrap_or_default().to_string(),
                    problem["line"].as_u64().unwrap_or_default(),
                    problem["column"].as_u64().unwrap_or_default(),
                )
            })
            .collect();
        if record["valid"] != *valid || actual != *problems {
            failures.push(format!("{file}: expected {valid} {problems:?}, got {record}"));
        }
        seen += 1;
    }
    assert_eq!(seen, expected.len(), "one record per document:\n{stdout}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The table is split four ways to keep each run's document count, and so
/// its per-document baseline cost, within the L1 budget. Rows that are not
/// lists render only in flow form.
macro_rules! declaration_runs {
    ($($test:ident => ($presentation:ident, $half:literal),)+) => {
        $(
            #[test]
            fn $test() {
                assert_declarations_validate(policy::Presentation::$presentation, $half);
            }
        )+

        const DECLARATION_RUNS: &[(policy::Presentation, usize)] = &[$((policy::Presentation::$presentation, $half)),+];
    };
}

declaration_runs! {
    schema_validate_meets_the_even_shared_declarations_in_block_form => (Block, 0),
    schema_validate_meets_the_odd_shared_declarations_in_block_form => (Block, 1),
    schema_validate_meets_the_even_shared_declarations_in_flow_form => (Flow, 0),
    schema_validate_meets_the_odd_shared_declarations_in_flow_form => (Flow, 1),
}

#[test]
fn declaration_runs_cover_both_presentations_and_halves_once() {
    let mut runs = DECLARATION_RUNS.to_vec();
    runs.sort_by_key(|(presentation, half)| (*presentation == policy::Presentation::Flow, *half));
    runs.dedup();
    assert_eq!(
        runs,
        [
            (policy::Presentation::Block, 0),
            (policy::Presentation::Block, 1),
            (policy::Presentation::Flow, 0),
            (policy::Presentation::Flow, 1),
        ]
    );
    assert_eq!(DECLARATION_RUNS.len(), 4);
}
/// Duplicate keys and malformed YAML carrying `content_policy`, `rule`, or
/// `action` fail frontmatter parsing (exit 3) before any schema applies.
#[test]
fn schema_validate_reports_damaged_policy_frontmatter_as_a_parse_error() {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let files: Vec<String> = LOAD_FAILURES
        .iter()
        .map(|(name, frontmatter)| {
            let file = format!("{name}.md");
            std::fs::write(process.cwd().join(&file), format!("---\n{frontmatter}---\nBody\n")).unwrap();
            file
        })
        .collect();

    let output = process
        .command_builder()
        .build()
        .args(["schema", "validate", "--format", "json"])
        .args(&files)
        .output()
        .expect("run md schema validate");
    assert_eq!(output.status.code(), Some(3), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let records: Vec<Value> = stdout.lines().map(|line| serde_json::from_str(line).expect("a JSON record")).collect();
    assert_eq!(records.len(), files.len(), "{stdout}");
    for record in &records {
        assert_eq!(record["error"], "frontmatter_parse", "{record}");
        assert_eq!(record["valid"], false, "{record}");
    }
}

/// Commands the shell-resolution tests run, approved by the launch
/// directory's own whitelist.
const SHELL_WHITELIST: &str =
    "exact echo words\nexact cat policy.txt\nexact echo TimeSensitive\nexact echo Duration\n";

/// Runs `md compose --frontmatter [extra…]` on a `doc.md` holding
/// `frontmatter`, beside a fixture-owned shell whitelist and a `policy.txt`
/// whose text is a JSON list of one unknown rule.
fn compose_shell_resolved(frontmatter: &str, extra: &[&str]) -> (Option<i32>, String, String) {
    let process = CliProcessFixture::new();
    let document = process.write_file("cwd/doc.md", &format!("---\n{frontmatter}---\nBody\n"));
    process.write_file("cwd/.darkmatter-shell-whitelist", SHELL_WHITELIST);
    process.write_file("cwd/policy.txt", "[\"Duration(3mo)\"]\n");
    // `echo` and `cat` run as programs; on Windows they exist only as Git's
    // executables outside System32, so the host PATH is declared.
    let output = process
        .command_builder()
        .host_path()
        .plain_terminal(400, 50)
        .build()
        .args(["compose", "--frontmatter"])
        .args(extra)
        .arg(&document)
        .output()
        .expect("run md compose");
    (
        output.status.code(),
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        plain(&String::from_utf8(output.stderr).expect("utf-8 stderr")),
    )
}

/// Shell output that leaves `content_policy` invalid is rejected after
/// resolution under the embedded baseline, whether or not trigger discovery
/// is enabled: disabling triggers removes trigger schemas, not the
/// baseline's verdict on resolved values.
fn assert_shell_resolved_policies_rejected(extra: &[&str]) {
    for frontmatter in [
        "content_policy: \"$(echo words)\"\n",
        // Shell stdout is text: a JSON list printed by a command is a string.
        "content_policy: \"$(cat policy.txt)\"\n",
        "content_policy: \"$(echo words)::ok\"\n",
        "value: \"$(echo words)\"\ncontent_policy: \"{{ value }}\"\n",
        // A rule a template reads from shell output is judged once resolved.
        "rule: \"$(echo Duration)\"\ncontent_policy: [\"{{ rule }}\"]\n",
        // `$(...)` inside a list is never run: it is an invalid rule as written.
        "content_policy: [\"$(echo X)\", TimeSensitive]\n",
    ] {
        let (code, stdout, stderr) = compose_shell_resolved(frontmatter, extra);
        assert_eq!(code, Some(1), "{extra:?} {frontmatter:?} composed:\n{stdout}\n{stderr}");
        assert!(stdout.is_empty(), "{extra:?} {frontmatter:?}: {stdout}");
        assert!(stderr.contains("schema validation failed"), "{extra:?} {frontmatter:?}: {stderr}");
        assert!(stderr.contains("content_policy"), "{extra:?} {frontmatter:?}: {stderr}");
    }
}

#[test]
fn compose_rejects_an_invalid_shell_resolved_policy() {
    assert_shell_resolved_policies_rejected(&[]);
}

#[test]
fn compose_without_trigger_schemas_rejects_an_invalid_shell_resolved_policy() {
    assert_shell_resolved_policies_rejected(&["--no-trigger-schemas"]);
}

/// The positive control: shell output that resolves to a value the
/// baseline accepts composes beside a valid policy in both trigger modes, so
/// the post-resolution check judges values rather than refusing shell output.
#[test]
fn compose_accepts_valid_shell_resolved_values_in_both_trigger_modes() {
    let frontmatter = "title: \"$(echo words)\"\ncontent_policy: [TimeSensitive]\n";
    for extra in [&[][..], &["--no-trigger-schemas"][..]] {
        let (code, stdout, stderr) = compose_shell_resolved(frontmatter, extra);
        assert_eq!(code, Some(0), "{extra:?}: {stderr}");
        assert!(stdout.contains("title: words"), "{extra:?}: {stdout}");
    }
}

/// A policy that a template builds from shell output waits for that output:
/// the first schema pass, and the shell-discovery pass before it, see only the
/// command's unexpanded text, so they defer the value rather than reject it.
#[test]
fn compose_accepts_a_policy_template_over_shell_output_in_both_trigger_modes() {
    for content_policy in ["[\"{{ rule }}\"]", "\"{{ [rule] }}\""] {
        let frontmatter = format!("rule: \"$(echo TimeSensitive)\"\ncontent_policy: {content_policy}\n");
        for extra in [&[][..], &["--no-trigger-schemas"][..]] {
            let (code, stdout, stderr) = compose_shell_resolved(&frontmatter, extra);
            assert_eq!(code, Some(0), "{extra:?} {content_policy}: {stderr}");
            assert!(
                stdout.replace("\r\n", "\n").contains("content_policy:\n- TimeSensitive\n"),
                "{extra:?} {content_policy}: {stdout}"
            );
        }
    }
}

/// With the baseline disabled and no document or trigger schema, there is no
/// effective schema, so the post-resolution check has nothing to apply.
#[test]
fn compose_without_any_schema_leaves_shell_resolved_values_unjudged() {
    for extra in [&["--no-baseline-schema"][..], &["--no-baseline-schema", "--no-trigger-schemas"][..]] {
        let (code, stdout, stderr) = compose_shell_resolved("content_policy: \"$(echo words)\"\n", extra);
        assert_eq!(code, Some(0), "{extra:?}: {stderr}");
        assert!(stdout.contains("content_policy: words"), "{extra:?}: {stdout}");
    }
}
