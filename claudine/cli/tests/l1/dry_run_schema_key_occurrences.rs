//! Schema-key findings of a sequence's passive check belong to task
//! occurrences: each loaded document is judged once, and each verdict is
//! reported under every occurrence that uses the object it names, with that
//! occurrence's trail, located at the authored key. The parser's refusal of
//! the same key, and a formal file's aggregate refusal made of such keys, are
//! not second rows; a catalog entry no step selects reports nothing. The
//! parity case runs the schema-key fixture shared with DMLS and the embedded
//! validator through the shipped CLI.

use std::fs;

use serde_json::Value;

use crate::common::CliProcessFixture;

const PARITY_DOCUMENT: &str = include_str!("../../../lib/tests/fixtures/schema_keys/sequence.md");
const PARITY_EXPECTED: &str = include_str!("../../../lib/tests/fixtures/schema_keys/expected.json");

/// A fixture holding `files`, with a `goose` the check only looks up.
fn fixture(name: &str, files: &[(&str, &str)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    for binary in ["goose", "claude"] {
        let path = fixture.bin_dir().join(format!("{binary}{}", std::env::consts::EXE_SUFFIX));
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    for (path, text) in files {
        fs::write(fixture.cwd().join(path), text).unwrap();
    }
    fixture
}

/// The exit code and JSON envelope of `claudine sequence <root> --dry-run --json`.
fn check_json(fixture: &CliProcessFixture, root: &str) -> (Option<i32>, Value) {
    let output = fixture.command().args(["sequence", root, "--dry-run", "--json"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    (output.status.code(), envelope)
}

/// One row, reduced to what an occurrence check compares.
#[derive(Debug, PartialEq)]
struct Row {
    code: String,
    severity: String,
    message: String,
    hint: Option<String>,
    /// The file name, line, and column of the row's source.
    at: Option<(String, Option<u64>, Option<u64>)>,
    /// Each trail frame as `label@property`.
    trail: Vec<String>,
}

fn rows(envelope: &Value) -> Vec<Row> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| Row {
            code: row["code"].as_str().unwrap().to_string(),
            severity: row["severity"].as_str().unwrap().to_string(),
            message: row["message"].as_str().unwrap().to_string(),
            hint: row["hint"].as_str().map(ToString::to_string),
            at: row["source"]["file"].as_str().map(|file| {
                let name = file.replace('\\', "/").rsplit('/').next().unwrap().to_string();
                (name, row["source"]["line"].as_u64(), row["source"]["column"].as_u64())
            }),
            trail: row["trail"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|frame| format!("{}@{}", frame["label"].as_str().unwrap(), frame["property"].as_str().unwrap()))
                .collect(),
        })
        .collect()
}

/// The rows of `envelope` that are errors or warnings.
fn findings(envelope: &Value) -> Vec<Row> {
    rows(envelope).into_iter().filter(|row| row.severity != "info").collect()
}

fn at(file: &str, line: u64, column: u64) -> Option<(String, Option<u64>, Option<u64>)> {
    Some((file.to_string(), Some(line), Some(column)))
}

const WHEM_MESSAGE: &str = "Additional properties are not allowed ('whem' was unexpected)";
const WHEM_HINT: &str = "did you mean `when`?";
const PROMT_MESSAGE: &str = "key `promt` is not a declared field, so it is kept as data";
const PROMT_HINT: &str = "did you mean `prompt`?";

fn whem(at: Option<(String, Option<u64>, Option<u64>)>, trail: &[&str]) -> Row {
    Row {
        code: "dm.schema.unknown_key".into(),
        severity: "error".into(),
        message: WHEM_MESSAGE.into(),
        hint: Some(WHEM_HINT.into()),
        at,
        trail: trail.iter().map(ToString::to_string).collect(),
    }
}

fn promt(at: Option<(String, Option<u64>, Option<u64>)>, trail: &[&str]) -> Row {
    Row {
        code: "dm.schema.near_miss_key".into(),
        severity: "warning".into(),
        message: PROMT_MESSAGE.into(),
        hint: Some(PROMT_HINT.into()),
        at,
        trail: trail.iter().map(ToString::to_string).collect(),
    }
}

/// R1-F7.1: a standalone `kind: task` file's typo is one located row at the
/// task occurrence, not a schema row plus the parser's.
#[test]
fn a_task_files_typo_is_one_row_at_the_task() {
    let fixture = fixture(
        "keys-task-file",
        &[
            ("seq.md", "---\nagent: goose\nsequence:\n  - name: t\n    task: ./t.yaml\n---\n"),
            ("t.yaml", "kind: task\nprompt: ./x.md\nwhem: \"true\"\n"),
            ("x.md", "x\n"),
        ],
    );
    let (code, envelope) = check_json(&fixture, "seq.md");
    assert_eq!(code, Some(2), "{envelope:#}");
    assert_eq!(findings(&envelope), vec![whem(at("t.yaml", 3, 1), &["t@sequence[0]"])]);
}

/// R1-F7.2: an inline group leg in a formal YAML sequence: one verdict under
/// the leg, with no aggregate `kind: sequence` refusal or parser row.
#[test]
fn a_formal_files_inline_leg_typo_is_one_row_under_the_leg() {
    let fixture = fixture(
        "keys-formal-leg",
        &[
            (
                "seq.yaml",
                "kind: sequence\nagent: goose\nsequence:\n  - name: g\n    group:\n      tasks:\n        - name: leg\n          prompt: ./x.md\n          whem: \"true\"\n",
            ),
            ("x.md", "x\n"),
        ],
    );
    let (code, envelope) = check_json(&fixture, "seq.yaml");
    assert_eq!(code, Some(2), "{envelope:#}");
    assert_eq!(
        findings(&envelope),
        vec![whem(at("seq.yaml", 9, 11), &["g@sequence[0]", "leg@sequence[0].group.tasks[0].whem"])]
    );
}

/// R1-F7.3: a referenced group file's leg: one verdict under the leg.
#[test]
fn a_group_files_leg_typo_is_one_row_under_the_leg() {
    let fixture = fixture(
        "keys-group-file",
        &[
            ("seq.md", "---\nagent: goose\nsequence:\n  - name: g\n    group: ./g.yaml\n---\n"),
            ("g.yaml", "kind: group\ntasks:\n  - name: leg\n    prompt: ./x.md\n    whem: \"true\"\n"),
            ("x.md", "x\n"),
        ],
    );
    let (code, envelope) = check_json(&fixture, "seq.md");
    assert_eq!(code, Some(2), "{envelope:#}");
    assert_eq!(findings(&envelope), vec![whem(at("g.yaml", 5, 5), &["g@sequence[0]", "leg@tasks[0].whem"])]);
}

/// The advice of a root that runs the step `s` (with `promt` beside its
/// `prompt`) twice, the child list written by `child`: one row per
/// occurrence, each with its trail.
fn twice(name: &str, child: Option<(&str, &str)>) -> Vec<Row> {
    let step = "  - name: s\n    prompt: ./x.md\n    promt: ./y.md\n";
    let root = match child {
        Some((file, _)) => format!("---\nagent: goose\nsequence:\n  - name: a\n    sequence: ./{file}\n  - name: b\n    sequence: ./{file}\n---\n"),
        None => {
            let nested = step.lines().map(|line| format!("    {line}\n")).collect::<String>();
            format!("---\nagent: goose\nsequence:\n  - name: a\n    sequence:\n{nested}  - name: b\n    sequence:\n{nested}---\n")
        }
    };
    let mut files = vec![("seq.md", root.as_str()), ("x.md", "x\n")];
    let child_text = child.map(|(file, wrap)| (file, wrap.replace("STEP", step)));
    if let Some((file, text)) = &child_text {
        files.push((file, text.as_str()));
    }
    let fixture = fixture(name, &files);
    let (code, envelope) = check_json(&fixture, "seq.md");
    assert_eq!(code, Some(2), "{envelope:#}");
    findings(&envelope)
}

/// R1-F7.4: a referenced formal YAML sequence used twice: one advisory per
/// occurrence, located at the key, each with its occurrence's trail.
#[test]
fn a_yaml_sequence_used_twice_advises_each_occurrence() {
    let rows = twice("keys-yaml-twice", Some(("child.yaml", "kind: sequence\nsequence:\nSTEP")));
    assert_eq!(
        rows,
        vec![
            promt(at("child.yaml", 5, 5), &["a@sequence[0]", "s@sequence[0].promt"]),
            promt(at("child.yaml", 5, 5), &["b@sequence[1]", "s@sequence[0].promt"]),
        ]
    );
}

/// R1-F7.5: a referenced formal Markdown sequence gives the same advice as
/// its YAML and inline equivalents, per occurrence.
#[test]
fn a_markdown_sequence_advises_as_its_yaml_and_inline_equivalents() {
    let markdown = twice("keys-md-twice", Some(("child.md", "---\nkind: sequence\nsequence:\nSTEP---\n")));
    assert_eq!(
        markdown,
        vec![
            promt(at("child.md", 6, 5), &["a@sequence[0]", "s@sequence[0].promt"]),
            promt(at("child.md", 6, 5), &["b@sequence[1]", "s@sequence[0].promt"]),
        ]
    );
    let inline = twice("keys-inline-twice", None);
    let advice = |rows: &[Row]| rows.iter().map(|row| (row.code.clone(), row.message.clone(), row.hint.clone())).collect::<Vec<_>>();
    assert_eq!(advice(&inline), advice(&markdown));
    assert_eq!(
        inline.iter().map(|row| row.trail.clone()).collect::<Vec<_>>(),
        vec![
            vec!["a@sequence[0]".to_string(), "s@sequence[0].sequence[0].promt".to_string()],
            vec!["b@sequence[1]".to_string(), "s@sequence[1].sequence[0].promt".to_string()],
        ]
    );
}

/// R1-F7.6: a root open step's near miss is one advisory under the
/// authored step; `when` on a step, a closed leg, and a task file is legal
/// and reported by nothing.
#[test]
fn a_root_steps_near_miss_is_advised_and_legal_when_is_not() {
    let fixture = fixture(
        "keys-root-when",
        &[
            (
                "seq.md",
                "---\nagent: goose\nsequence:\n  - name: s\n    prompt: ./x.md\n    promt: ./y.md\n    when: \"true\"\n  - name: g\n    group:\n      tasks:\n        - name: leg\n          prompt: ./x.md\n          when: \"true\"\n  - name: t\n    task: ./t.yaml\n---\n",
            ),
            ("t.yaml", "kind: task\nprompt: ./x.md\nwhen: \"true\"\n"),
            ("x.md", "x\n"),
        ],
    );
    let (_, envelope) = check_json(&fixture, "seq.md");
    assert_eq!(findings(&envelope), vec![promt(at("seq.md", 6, 5), &["s@sequence[0].promt"])]);
}

const CATALOG_ROOT: &str = "---\nagent: goose\nsequence:\n  - name: g\n    group: second@./cat.yaml\n---\n";

/// R1-F7.7: a typo in a catalog entry no step selects is not reported, and
/// the run's exit reflects the selected entry only.
#[test]
fn an_unselected_catalog_entry_reports_nothing() {
    let fixture = fixture(
        "keys-catalog-unselected",
        &[
            ("seq.md", CATALOG_ROOT),
            (
                "cat.yaml",
                "kind: group-catalog\ngroups:\n  - name: first\n    tasks:\n      - name: a\n        prompt: ./x.md\n        whem: \"true\"\n  - name: second\n    tasks:\n      - name: b\n        prompt: ./x.md\n",
            ),
            ("x.md", "x\n"),
        ],
    );
    let (code, envelope) = check_json(&fixture, "seq.md");
    assert_eq!(code, Some(0), "{envelope:#}");
    assert_eq!(findings(&envelope), Vec::new());
}

/// The control of [`an_unselected_catalog_entry_reports_nothing`]: the
/// selected entry's own keys are reported, a leg's under the leg, and the
/// entry's under the task that selects it.
#[test]
fn a_selected_catalog_entrys_keys_are_reported_at_their_occurrence() {
    let catalog = |second: &str| {
        format!(
            "kind: group-catalog\ngroups:\n  - name: first\n    tasks:\n      - name: a\n        prompt: ./x.md\n  - name: second\n{second}    tasks:\n      - name: b\n        prompt: ./x.md\n        whem: \"true\"\n"
        )
    };
    let leg = fixture("keys-catalog-leg", &[("seq.md", CATALOG_ROOT), ("cat.yaml", &catalog("")), ("x.md", "x\n")]);
    let (code, envelope) = check_json(&leg, "seq.md");
    assert_eq!(code, Some(2), "{envelope:#}");
    assert_eq!(findings(&envelope), vec![whem(at("cat.yaml", 11, 9), &["g@sequence[0]", "b@groups[1].tasks[0].whem"])]);

    // The parser stops at the entry's removed key before reading its legs,
    // so both verdicts are the selecting task's.
    let entry = fixture(
        "keys-catalog-entry",
        &[("seq.md", CATALOG_ROOT), ("cat.yaml", &catalog("    execution: parallel\n")), ("x.md", "x\n")],
    );
    let (code, envelope) = check_json(&entry, "seq.md");
    assert_eq!(code, Some(2), "{envelope:#}");
    let rows = findings(&envelope);
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert_eq!(rows[0].code, "dm.schema.removed_key");
    assert_eq!(rows[0].at, at("cat.yaml", 8, 5));
    assert_eq!(rows[0].trail, vec!["g@sequence[0]".to_string()]);
    assert_eq!(rows[1], whem(at("cat.yaml", 12, 9), &["g@sequence[0]"]));
}

/// R1-F7.8 and R1-F7.9: two occurrences of one child file whose prompt is
/// missing keep their missing-document rows apart, and every `not checked`
/// row carries its occurrence's trail.
#[test]
fn repeated_occurrences_keep_their_trails_on_unchecked_rows() {
    let fixture = fixture(
        "keys-unchecked-twice",
        &[
            ("seq.md", "---\nagent: goose\nsequence:\n  - name: a\n    sequence: ./child.yaml\n  - name: b\n    sequence: ./child.yaml\n---\n"),
            ("child.yaml", "kind: sequence\nsequence:\n  - name: s\n    prompt: ./missing.md\n"),
        ],
    );
    let (code, envelope) = check_json(&fixture, "seq.md");
    assert_eq!(code, Some(2), "{envelope:#}");
    let rows = rows(&envelope);
    let missing: Vec<_> = rows.iter().filter(|row| row.code == "dm.reference.missing_document").map(|row| row.trail.clone()).collect();
    assert_eq!(
        missing,
        vec![
            vec!["a@sequence[0]".to_string(), "s@sequence[0].prompt".to_string()],
            vec!["b@sequence[1]".to_string(), "s@sequence[0].prompt".to_string()],
        ]
    );
    let unchecked: Vec<_> = rows.iter().filter(|row| row.code == "dm.check.not_checked").map(|row| (row.message.clone(), row.trail.clone())).collect();
    let trail = |outer: &str| vec![outer.to_string(), "s@sequence[0]".to_string()];
    assert_eq!(
        unchecked,
        vec![
            ("not checked: provider (the prompt file could not be read)".to_string(), trail("a@sequence[0]")),
            ("not checked: inputs (the prompt file could not be read)".to_string(), trail("a@sequence[0]")),
            ("not checked: provider (the prompt file could not be read)".to_string(), trail("b@sequence[1]")),
            ("not checked: inputs (the prompt file could not be read)".to_string(), trail("b@sequence[1]")),
        ]
    );
}

/// A child file whose first occurrence fails to load is still read by the
/// second, not refused as a cycle: each occurrence reports the removed key,
/// and the real run still stops at the parser's own typed error.
#[test]
fn a_failed_occurrence_does_not_make_the_next_one_a_cycle() {
    let fixture = fixture(
        "keys-failed-twice",
        &[
            ("seq.md", "---\nagent: goose\nsequence:\n  - name: a\n    sequence: ./child.yaml\n  - name: b\n    sequence: ./child.yaml\n---\n"),
            ("child.yaml", "kind: sequence\nfail_fast: true\nsequence:\n  - name: s\n    prompt: ./x.md\n"),
            ("x.md", "x\n"),
        ],
    );
    let (code, envelope) = check_json(&fixture, "seq.md");
    assert_eq!(code, Some(2), "{envelope:#}");
    let rows = findings(&envelope);
    assert_eq!(
        rows.iter().map(|row| (row.code.as_str(), row.at.clone(), row.trail.clone())).collect::<Vec<_>>(),
        vec![
            ("dm.schema.removed_key", at("child.yaml", 2, 1), vec!["a@sequence[0]".to_string()]),
            ("dm.schema.removed_key", at("child.yaml", 2, 1), vec!["b@sequence[1]".to_string()]),
        ]
    );

    let real = fixture.command().args(["sequence", "seq.md"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&real.stderr);
    assert_eq!(real.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("The run stopped at: `fail_fast` uses the removed `fail_fast` grammar"), "{stderr}");
}

/// The schema-key parity fixture DMLS and the embedded validator share
/// (`claudine/lib/tests/fixtures/schema_keys/`), through the shipped CLI, run
/// directly and as a Markdown sequence another sequence references: in each,
/// every expected finding is one row with the same code, message, hint,
/// line, and column, and the CLI adds no other schema-key row. The
/// referenced run is what fails when the check skips a referenced Markdown
/// file.
#[test]
fn the_cli_reports_the_parity_fixtures_findings() {
    let fixture = fixture(
        "keys-parity",
        &[
            ("sequence.md", PARITY_DOCUMENT),
            ("ask.md", "ask\n"),
            ("outer.yaml", "sequence:\n  - name: parity\n    sequence: ./sequence.md\n"),
        ],
    );
    for root in ["sequence.md", "outer.yaml"] {
        assert_parity_rows(&fixture, root);
    }
}

fn assert_parity_rows(fixture: &CliProcessFixture, root: &str) {
    let (_, envelope) = check_json(fixture, root);
    let expected: Vec<Value> = serde_json::from_str(PARITY_EXPECTED).unwrap();
    let key_rows: Vec<Row> = rows(&envelope).into_iter().filter(|row| row.code.starts_with("dm.schema.")).collect();
    let mut expected_rows: Vec<_> = expected
        .iter()
        .map(|finding| {
            (
                finding["code"].as_str().unwrap().to_string(),
                finding["severity"].as_str().unwrap().to_string(),
                finding["message"].as_str().unwrap().to_string(),
                finding["hint"].as_str().map(ToString::to_string),
                at("sequence.md", finding["line"].as_u64().unwrap(), finding["column"].as_u64().unwrap()),
            )
        })
        .collect();
    let mut published: Vec<_> = key_rows
        .into_iter()
        .map(|row| (row.code, row.severity, row.message, row.hint, row.at))
        .collect();
    expected_rows.sort();
    published.sort();
    assert_eq!(published, expected_rows, "{root}: {envelope:#}");
}
