//! Which baseline `md schema validate` checks a document against, and what
//! the embedded Darkmatter baseline's `content_policy` typing reports.
//!
//! The selection order, first match wins: `--no-baseline-schema` (none) →
//! `--schema <path>` (custom) → truthy `DARKMATTER_NO_BASELINE_SCHEMA` (none)
//! → `BASELINE_SCHEMA` (custom) → the embedded baseline. Each row spawns `md`
//! once or twice, so every row is its own `#[test]`.

use crate::common;
use crate::content_policy_fixtures::{self as policy, DECLARATIONS, MIXED_POLICY_FRONTMATTER};

use common::CliProcessFixture;
use predicates::prelude::*;
use serde_json::Value;
use std::path::PathBuf;

const NO_BASELINE_ENV: &str = "DARKMATTER_NO_BASELINE_SCHEMA";
const BASELINE_ENV: &str = "BASELINE_SCHEMA";

/// Valid with no baseline; rejected by the embedded baseline on
/// `/content_policy` (a string, not a list) and `/last_updated` (not a date);
/// rejected by either custom baseline only for its own required property.
const PROBE: &str = "---\ncontent_policy: plain words\nlast_updated: not-a-date\n---\nBody\n";

/// A custom baseline selected by `--schema`. It replaces the embedded one, so
/// its `content_policy: string` accepts the probe's policy.
const FLAG_BASELINE: &str = "$schema:\n  content_policy: string\n  flag_owner: string(required)\n";
/// A custom baseline selected by `BASELINE_SCHEMA`.
const ENV_BASELINE: &str = "$schema:\n  content_policy: string\n  env_owner: string(required)\n";

/// The baseline a probe run was checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Observed {
    None,
    Embedded,
    Flag,
    Env,
}

/// One `md schema validate --format json` run's exit code and record.
struct JsonRun {
    code: Option<i32>,
    record: Value,
    stdout: String,
}

impl JsonRun {
    fn problem_paths(&self) -> Vec<String> {
        self.record["problems"]
            .as_array()
            .expect("a validated record carries `problems`")
            .iter()
            .map(|problem| problem["path"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    fn mentions(&self, needle: &str) -> bool {
        self.stdout.contains(needle)
    }
}

/// The inputs of one selection row.
#[derive(Default, Clone, Copy)]
struct Selection {
    no_baseline_flag: bool,
    schema_flag: bool,
    no_baseline_env: Option<&'static str>,
    baseline_env: bool,
}

/// Lays out both custom baselines at the launch directory and the probe one
/// directory below it, so the relative custom paths resolve only in the
/// launch context.
fn probe_layout(process: &CliProcessFixture) -> PathBuf {
    let cwd = process.cwd();
    std::fs::write(cwd.join("flag.yaml"), FLAG_BASELINE).unwrap();
    std::fs::write(cwd.join("env.yaml"), ENV_BASELINE).unwrap();
    std::fs::create_dir_all(cwd.join("docs")).unwrap();
    let probe = cwd.join("docs/probe.md");
    std::fs::write(&probe, PROBE).unwrap();
    probe
}

fn run_json(
    process: &CliProcessFixture,
    selection: Selection,
    extra: &[&str],
    document: &std::path::Path,
) -> JsonRun {
    let mut builder = process.command_builder();
    if let Some(value) = selection.no_baseline_env {
        builder = builder.application_input(NO_BASELINE_ENV, value);
    }
    if selection.baseline_env {
        builder = builder.application_input(BASELINE_ENV, "./env.yaml");
    }
    let mut command = builder.build();
    command.args(["schema", "validate", "--format", "json"]);
    if selection.no_baseline_flag {
        command.arg("--no-baseline-schema");
    }
    if selection.schema_flag {
        command.args(["--schema", "./flag.yaml"]);
    }
    let output = command.args(extra).arg(document).output().expect("run md schema validate");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let record = serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!(
            "one JSON record expected ({error}); stdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    JsonRun {
        code: output.status.code(),
        record,
        stdout,
    }
}

/// Runs the probe under `selection` and names the baseline its problems show.
fn observe(selection: Selection) -> Observed {
    let process = CliProcessFixture::new();
    let probe = probe_layout(&process);
    let run = run_json(&process, selection, &[], &probe);
    assert!(run.record["schema"].is_null(), "the probe has no `$schema`: {}", run.stdout);
    let paths = run.problem_paths();
    match (run.code, run.mentions("flag_owner"), run.mentions("env_owner")) {
        (Some(0), false, false) => Observed::None,
        (Some(1), true, false) => Observed::Flag,
        (Some(1), false, true) => Observed::Env,
        (Some(1), false, false)
            if paths.iter().any(|path| path == "/content_policy")
                && paths.iter().any(|path| path == "/last_updated") =>
        {
            Observed::Embedded
        }
        _ => panic!("unrecognized outcome {:?}: {}", run.code, run.stdout),
    }
}

// ── selection order ─────────────────────────────────────────────────────────

macro_rules! selection_rows {
    ($($name:ident: { $($field:ident: $value:expr),* $(,)? } => $expected:ident;)+) => {
        $(
            #[test]
            fn $name() {
                #[allow(clippy::needless_update)]
                let selection = Selection { $($field: $value,)* ..Selection::default() };
                assert_eq!(observe(selection), Observed::$expected);
            }
        )+
    };
}

selection_rows! {
    selection_nothing_uses_the_embedded_baseline: {} => Embedded;
    selection_flag_opt_out_uses_none: { no_baseline_flag: true } => None;
    selection_flag_opt_out_ignores_baseline_schema_env: { no_baseline_flag: true, baseline_env: true } => None;
    selection_env_opt_out_without_flag_uses_none: { no_baseline_env: Some("1") } => None;
    selection_schema_flag_wins_over_truthy_env_opt_out: { schema_flag: true, no_baseline_env: Some("true") } => Flag;
    selection_schema_flag_wins_over_baseline_schema_env: { schema_flag: true, baseline_env: true } => Flag;
    selection_schema_flag_wins_over_both_variables: {
        schema_flag: true, no_baseline_env: Some("yes"), baseline_env: true
    } => Flag;
    selection_truthy_env_opt_out_ignores_baseline_schema_env: {
        no_baseline_env: Some("YES"), baseline_env: true
    } => None;
    selection_baseline_schema_env_alone_is_the_custom_baseline: { baseline_env: true } => Env;
    selection_non_disabling_env_keeps_baseline_schema_env: {
        no_baseline_env: Some("0"), baseline_env: true
    } => Env;
}

// ── `DARKMATTER_NO_BASELINE_SCHEMA` spellings ───────────────────────────────

/// Every documented disabling spelling, and representative values that leave
/// the embedded baseline on.
const SPELLINGS: &[(&str, Observed)] = &[
    ("1", Observed::None),
    ("true", Observed::None),
    ("TRUE", Observed::None),
    ("yes", Observed::None),
    ("YES", Observed::None),
    ("0", Observed::Embedded),
    ("false", Observed::Embedded),
    ("no", Observed::Embedded),
    ("", Observed::Embedded),
    ("True", Observed::Embedded),
    (" yes", Observed::Embedded),
];

macro_rules! spelling_rows {
    ($($name:ident => $value:literal;)+) => {
        /// The values the generated tests below spawn, in order.
        const SPELLING_TESTS: &[&str] = &[$($value),+];
        $(
            #[test]
            fn $name() {
                let (_, expected) = SPELLINGS
                    .iter()
                    .find(|(value, _)| *value == $value)
                    .unwrap_or_else(|| panic!("{:?} has no row in SPELLINGS", $value));
                let selection = Selection { no_baseline_env: Some($value), ..Selection::default() };
                assert_eq!(observe(selection), *expected, "DARKMATTER_NO_BASELINE_SCHEMA={:?}", $value);
            }
        )+
    };
}

spelling_rows! {
    spelling_one_disables => "1";
    spelling_true_disables => "true";
    spelling_upper_true_disables => "TRUE";
    spelling_yes_disables => "yes";
    spelling_upper_yes_disables => "YES";
    spelling_zero_keeps_the_baseline => "0";
    spelling_false_keeps_the_baseline => "false";
    spelling_no_keeps_the_baseline => "no";
    spelling_empty_keeps_the_baseline => "";
    spelling_title_case_true_keeps_the_baseline => "True";
    spelling_padded_yes_keeps_the_baseline => " yes";
}

#[test]
fn spelling_tests_cover_every_row_once() {
    let mut tested: Vec<&str> = SPELLING_TESTS.to_vec();
    let mut table: Vec<&str> = SPELLINGS.iter().map(|(value, _)| *value).collect();
    tested.sort_unstable();
    table.sort_unstable();
    assert_eq!(tested, table, "one generated test per SPELLINGS row");
    let disabling: Vec<&str> = SPELLINGS
        .iter()
        .filter(|(_, observed)| *observed == Observed::None)
        .map(|(value, _)| *value)
        .collect();
    assert_eq!(disabling, ["1", "true", "TRUE", "yes", "YES"], "the documented disabling spellings");
}

// ── custom baselines that fail to load ──────────────────────────────────────

/// Runs `md` against a broken custom baseline and asserts the schema-load
/// exit with no per-file record, so nothing fell back to another baseline.
fn assert_broken_baseline(schema_flag: bool, no_baseline_env: Option<&str>, baseline_env: bool) {
    let process = CliProcessFixture::new();
    let cwd = process.cwd();
    std::fs::write(cwd.join("broken.yaml"), "not: a-schema\n").unwrap();
    let document = cwd.join("doc.md");
    std::fs::write(&document, "---\ntitle: hi\n---\nBody\n").unwrap();

    let mut builder = process.command_builder();
    if let Some(value) = no_baseline_env {
        builder = builder.application_input(NO_BASELINE_ENV, value);
    }
    if baseline_env {
        builder = builder.application_input(BASELINE_ENV, "broken.yaml");
    }
    let mut command = builder.build();
    command.args(["schema", "validate", "--format", "json"]);
    if schema_flag {
        command.args(["--schema", "broken.yaml"]);
    }
    command.arg(&document).assert().code(2).stdout(predicate::str::is_empty());
}

#[test]
fn broken_schema_flag_baseline_exits_2_without_fallback() {
    assert_broken_baseline(true, None, false);
}

#[test]
fn broken_schema_flag_baseline_exits_2_even_with_env_opt_out() {
    assert_broken_baseline(true, Some("1"), false);
}

#[test]
fn broken_baseline_schema_env_exits_2_without_fallback() {
    assert_broken_baseline(false, None, true);
}

#[test]
fn missing_baseline_schema_env_file_exits_2_without_fallback() {
    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, "---\ntitle: hi\n---\nBody\n").unwrap();
    process
        .command_builder()
        .application_input(BASELINE_ENV, "missing.yaml")
        .build()
        .args(["schema", "validate"])
        .arg(&document)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty());
}

#[test]
fn no_baseline_schema_conflicts_with_schema_flag() {
    let process = CliProcessFixture::new();
    // Clap rejects the pair before `md` reads either path; clap's usage exit
    // is `2` throughout this CLI.
    process
        .command()
        .args(["schema", "validate", "--no-baseline-schema", "--schema", "x.yaml", "doc.md"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("cannot be used with"));
}

// ── document and trigger schemas after an opt-out ───────────────────────────

const OWNER_SCHEMA: &str = "$schema:\n  owner: string(required)\n";

fn assert_document_schema_survives(selection: Selection) {
    let process = CliProcessFixture::new();
    let cwd = process.cwd();
    let missing = cwd.join("missing.md");
    std::fs::write(&missing, format!("---\n{OWNER_SCHEMA}last_updated: not-a-date\n---\nBody\n")).unwrap();
    let present = cwd.join("present.md");
    std::fs::write(&present, format!("---\n{OWNER_SCHEMA}owner: me\nlast_updated: not-a-date\n---\nBody\n")).unwrap();

    let rejected = run_json(&process, selection, &[], &missing);
    assert_eq!(rejected.code, Some(1), "{}", rejected.stdout);
    assert!(rejected.mentions("owner"), "the document schema still applies: {}", rejected.stdout);
    assert!(!rejected.problem_paths().contains(&"/last_updated".to_string()), "{}", rejected.stdout);

    let accepted = run_json(&process, selection, &[], &present);
    assert_eq!(accepted.code, Some(0), "no baseline checks `last_updated`: {}", accepted.stdout);
}

#[test]
fn document_schema_still_validates_after_flag_opt_out() {
    assert_document_schema_survives(Selection { no_baseline_flag: true, ..Selection::default() });
}

#[test]
fn document_schema_still_validates_after_env_opt_out() {
    assert_document_schema_survives(Selection { no_baseline_env: Some("1"), ..Selection::default() });
}

/// A repository whose `kind: prompt` trigger requires `owner`.
fn trigger_layout(process: &CliProcessFixture) {
    let cwd = process.cwd();
    assert!(process.initialize_repository(), "`git` is required to build the trigger repository");
    std::fs::create_dir_all(cwd.join("schemas")).unwrap();
    std::fs::create_dir_all(cwd.join("docs")).unwrap();
    std::fs::write(
        cwd.join("schemas/prompt.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  kind: enum(prompt; required)\n$schema: prompt.yaml\n",
    )
    .unwrap();
    std::fs::write(cwd.join("schemas/prompt.yaml"), OWNER_SCHEMA).unwrap();
}

fn assert_trigger_schema_survives(selection: Selection) {
    let process = CliProcessFixture::new();
    trigger_layout(&process);
    let cwd = process.cwd();
    let missing = cwd.join("docs/missing.md");
    std::fs::write(&missing, "---\nkind: prompt\nlast_updated: not-a-date\n---\nBody\n").unwrap();
    let present = cwd.join("docs/present.md");
    std::fs::write(&present, "---\nkind: prompt\nowner: me\nlast_updated: not-a-date\n---\nBody\n").unwrap();

    let rejected = run_json(&process, selection, &[], &missing);
    assert_eq!(rejected.code, Some(1), "{}", rejected.stdout);
    assert!(rejected.mentions("owner"), "the trigger schema still applies: {}", rejected.stdout);

    let accepted = run_json(&process, selection, &[], &present);
    assert_eq!(accepted.code, Some(0), "no baseline checks `last_updated`: {}", accepted.stdout);
}

#[test]
fn trigger_schema_still_validates_after_flag_opt_out() {
    assert_trigger_schema_survives(Selection { no_baseline_flag: true, ..Selection::default() });
}

#[test]
fn trigger_schema_still_validates_after_env_opt_out() {
    assert_trigger_schema_survives(Selection { no_baseline_env: Some("true"), ..Selection::default() });
}

/// `--no-schema-triggers` turns triggers off and leaves the embedded baseline
/// on.
#[test]
fn no_schema_triggers_keeps_the_embedded_baseline() {
    let process = CliProcessFixture::new();
    trigger_layout(&process);
    let document = process.cwd().join("docs/prompt.md");
    std::fs::write(&document, "---\nkind: prompt\nlast_updated: not-a-date\n---\nBody\n").unwrap();

    let run = run_json(&process, Selection::default(), &["--no-schema-triggers"], &document);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert_eq!(run.problem_paths(), ["/last_updated"], "baseline only, no trigger: {}", run.stdout);
}

/// A document `$schema` that redeclares `content_policy` replaces the
/// baseline's policy typing for that property.
#[test]
fn document_schema_replaces_the_baseline_content_policy() {
    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(
        &document,
        "---\n$schema:\n  content_policy: string\ncontent_policy: plain words\n---\nBody\n",
    )
    .unwrap();
    let run = run_json(&process, Selection::default(), &[], &document);
    assert_eq!(run.code, Some(0), "{}", run.stdout);
}

// ── the embedded baseline by default ────────────────────────────────────────

fn pretty(
    process: &CliProcessFixture,
    extra: &[&str],
    no_baseline_env: Option<&str>,
    document: &std::path::Path,
) -> (Option<i32>, String) {
    let mut builder = process.command_builder().plain_terminal(400, 50);
    if let Some(value) = no_baseline_env {
        builder = builder.application_input(NO_BASELINE_ENV, value);
    }
    let output = builder
        .build()
        .args(["schema", "validate"])
        .args(extra)
        .arg(document)
        .output()
        .expect("run md schema validate");
    (output.status.code(), String::from_utf8(output.stdout).expect("utf-8 stdout"))
}

const NO_SCHEMA_CLAIM: &str = "doesn't have a schema definition";

#[test]
fn default_accepts_the_authoring_example() {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, format!("---\n{MIXED_POLICY_FRONTMATTER}---\nBody\n")).unwrap();

    let run = run_json(&process, Selection::default(), &[], &document);
    assert_eq!(run.code, Some(0), "{}", run.stdout);
    assert!(run.record["schema"].is_null(), "{}", run.stdout);

    let (code, stdout) = pretty(&process, &[], None, &document);
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.contains("has a schema definition and is valid"), "{stdout}");
}

#[test]
fn default_accepts_a_policy_free_document_quietly() {
    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, "---\ntitle: Plain\nlast_updated: 2026-10-04\n---\nBody\n").unwrap();

    let run = run_json(&process, Selection::default(), &[], &document);
    assert_eq!(run.code, Some(0), "{}", run.stdout);

    let (code, stdout) = pretty(&process, &["--quiet"], None, &document);
    assert_eq!(code, Some(0));
    assert!(stdout.is_empty(), "--quiet prints no success line: {stdout}");
}

/// Validates one shared declaration case with the default baseline and checks
/// every expected problem's path, kind, and authored position, then the
/// pretty rendering's schema claim.
fn assert_policy_rejected(case_name: &str) {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let case = DECLARATIONS
        .iter()
        .find(|case| case.name == case_name)
        .unwrap_or_else(|| panic!("no declaration case `{case_name}`"));
    assert_eq!(case.schema, policy::Verdict::Reject, "`{case_name}` is a rejection case");
    let rendered = policy::render(case.declaration).remove(0);

    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, rendered.document()).unwrap();

    let run = run_json(&process, Selection::default(), &[], &document);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.record["schema"].is_null(), "{}", run.stdout);
    let problems = run.record["problems"].as_array().expect("problems");
    assert_eq!(problems.len(), case.problems.len(), "{}", run.stdout);
    for expected in case.problems {
        let (line, column) = rendered.start(expected.at);
        let found = problems.iter().find(|problem| problem["path"] == expected.path);
        let found = found.unwrap_or_else(|| panic!("no problem at {}: {}", expected.path, run.stdout));
        assert_eq!(found["kind"], expected.kind, "{}", run.stdout);
        assert_eq!(found["line"], line, "{}", run.stdout);
        assert_eq!(found["column"], column, "{}", run.stdout);
    }

    let (code, stdout) = pretty(&process, &[], None, &document);
    assert_eq!(code, Some(1), "{stdout}");
    assert!(!stdout.contains(NO_SCHEMA_CLAIM), "{stdout}");
}

#[test]
fn default_rejects_an_unknown_rule() {
    assert_policy_rejected("duration_rule_name");
}

#[test]
fn default_rejects_an_unknown_action() {
    assert_policy_rejected("unknown_action");
}

#[test]
fn default_rejects_an_empty_policy_list() {
    assert_policy_rejected("empty_list");
}

#[test]
fn default_rejects_a_policy_that_is_not_a_list() {
    assert_policy_rejected("whole_string");
}

#[test]
fn default_rejects_a_long_form_entry_without_rule() {
    assert_policy_rejected("missing_rule");
}

#[test]
fn default_rejects_a_long_form_entry_with_null_rule() {
    assert_policy_rejected("null_rule");
}

// ── the intentional compatibility change ────────────────────────────────────

const NOT_A_DATE: &str = "---\nlast_updated: not-a-date\n---\nBody\n";

#[test]
fn default_rejects_an_invalid_last_updated() {
    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, NOT_A_DATE).unwrap();
    let run = run_json(&process, Selection::default(), &[], &document);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert_eq!(run.problem_paths(), ["/last_updated"], "{}", run.stdout);
}

#[test]
fn flag_opt_out_accepts_an_invalid_last_updated_without_a_schema() {
    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, NOT_A_DATE).unwrap();
    let (code, stdout) = pretty(&process, &["--no-baseline-schema"], None, &document);
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.contains(NO_SCHEMA_CLAIM), "{stdout}");
}

#[test]
fn env_opt_out_accepts_an_invalid_last_updated_without_a_schema() {
    let process = CliProcessFixture::new();
    let document = process.cwd().join("doc.md");
    std::fs::write(&document, NOT_A_DATE).unwrap();
    let (code, stdout) = pretty(&process, &[], Some("1"), &document);
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.contains(NO_SCHEMA_CLAIM), "{stdout}");
}
