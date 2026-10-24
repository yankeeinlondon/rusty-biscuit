//! `md schema validate` and `md schema triggers` against Claudine's shipped
//! schema bundle (`claudine/schemas/`, `2026-10-21-claudine-schemas`
//! criteria 1 and 5, and `2026-10-26-task-inputs-and-providers` criterion 9):
//! the seven files are copied into a fixture repository's `schemas/` folder,
//! and a representative subset of Claudine's AC5 and sequence-schema fixture
//! corpora, which carry no `$schema:` pointer, is discovered and validated
//! through the CLI. The full corpora are judged in-process by Claudine's
//! `claudine_schemas` and `sequence_schema` suites.

use crate::common;

use common::{CliProcessFixture, write};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Makes `root` a repository without spawning Git, so its `schemas/` folder
/// is every document's file-tree-root schema location.
fn initialize_repository(root: &Path) {
    let git_dir = root.join(".git");
    std::fs::create_dir_all(git_dir.join("objects")).unwrap();
    std::fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(git_dir.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").unwrap();
}

/// A fixture repository holding `bundle` in `schemas/` and `documents` in
/// `docs/`; returns the document paths.
fn repository(process: &CliProcessFixture, bundle: &[(&str, &str)], documents: &[(&str, &str)]) -> Vec<PathBuf> {
    let root = process.cwd();
    initialize_repository(root);
    for (name, text) in bundle {
        write(&root.join("schemas").join(name), text);
    }
    documents
        .iter()
        .map(|(name, text)| {
            let path = root.join("docs").join(format!("{name}.md"));
            write(&path, text);
            path
        })
        .collect()
}

/// One `md schema validate --format json` run, keyed by document name. Each
/// document compiles its own effective schema, so the baseline is left out to
/// keep the run inside the L1 budget; no row depends on it.
fn validate(process: &CliProcessFixture, documents: &[PathBuf]) -> (Option<i32>, BTreeMap<String, Value>) {
    let output = process
        .command()
        .args(["schema", "validate", "--no-baseline-schema", "--format", "json"])
        .args(documents)
        .output()
        .expect("md runs");
    let records = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line).expect("NDJSON record");
            let file = record["file"].as_str().expect("file").replace('\\', "/");
            let name = file.rsplit('/').next().expect("file name").trim_end_matches(".md").to_string();
            (name, record)
        })
        .collect();
    (output.status.code(), records)
}

/// Asserts a record's validity and that every listed `(instance path, text)`
/// is one of its problems.
fn assert_record(records: &BTreeMap<String, Value>, name: &str, valid: bool, expected: &[(&str, &str)]) {
    let record = records.get(name).unwrap_or_else(|| panic!("{name}: no record in {records:?}"));
    assert!(record.get("error").is_none(), "{name} loads its schemas: {record}");
    assert_eq!(record["valid"].as_bool(), Some(valid), "{name}: {record}");
    let problems: Vec<(&str, &str)> = record["problems"]
        .as_array()
        .expect("problems")
        .iter()
        .map(|problem| (problem["path"].as_str().unwrap_or_default(), problem["message"].as_str().unwrap_or_default()))
        .collect();
    for (pointer, needle) in expected {
        assert!(
            problems.iter().any(|(path, message)| path == pointer && message.contains(needle)),
            "{name}: expected a problem at `{pointer}` naming `{needle}`, got {problems:?}"
        );
    }
}

#[test]
fn claudine_bundle_loads_and_its_triggers_are_discovered() {
    let process = CliProcessFixture::new();
    // Read here rather than in a helper, so an edit to one of these files
    // schedules this test alone.
    let bundle = [
        ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
        ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
        ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
        ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
        ("sequence-types.yaml", include_str!("../../../../claudine/schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../../claudine/schemas/sequence.yaml")),
    ];
    let documents = repository(
        &process,
        &bundle,
        &[
            ("controls-beside-ticket", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/controls-beside-ticket.md")),
            ("cancelled-container", "---\ncancelled: [{stderr: cancelled}]\nsequence: [{name: child, sequence: []}]\n---\n"),
        ],
    );
    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(0), "{records:?}");
    assert_record(&records, "controls-beside-ticket", true, &[]);
    assert_record(&records, "cancelled-container", true, &[]);

    let output = process
        .command()
        .args(["schema", "triggers"])
        .arg(&documents[0])
        .output()
        .expect("md runs");
    assert!(output.status.success(), "{output:?}");
    let trace = String::from_utf8_lossy(&output.stdout);
    for trigger in ["claudine.yaml", "inline-compose.yaml"] {
        let line = trace
            .lines()
            .find(|line| line.contains(trigger))
            .unwrap_or_else(|| panic!("`{trigger}` is listed:\n{trace}"));
        assert!(line.contains("matched"), "`{trigger}` matched:\n{trace}");
        assert!(!line.contains("not matched"), "`{trigger}` matched:\n{trace}");
    }
}

#[test]
fn lifecycle_defects_are_reported_at_their_event_and_key() {
    let process = CliProcessFixture::new();
    // Read here rather than in a helper, so an edit to one of these files
    // schedules this test alone.
    let bundle = [
        ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
        ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
        ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
        ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
        ("sequence-types.yaml", include_str!("../../../../claudine/schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../../claudine/schemas/sequence.yaml")),
    ];
    let documents = repository(
        &process,
        &bundle,
        &[
            ("shell-in-initialize-then", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/shell-in-initialize-then.md")),
            ("misspelled-parameter", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/misspelled-parameter.md")),
            ("long-form-in-root-dictionary", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/long-form-in-root-dictionary.md")),
        ],
    );
    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(1), "{records:?}");
    assert_record(&records, "shell-in-initialize-then", false, &[("/initialize/0/then/0", "\"shell\"")]);
    assert_record(&records, "misspelled-parameter", false, &[("/start/0/say", "txet")]);
    assert_record(&records, "long-form-in-root-dictionary", false, &[("/success/say", "text")]);
}

#[test]
fn malformed_identifying_values_select_the_trigger_and_unrelated_data_does_not() {
    let process = CliProcessFixture::new();
    // Read here rather than in a helper, so an edit to one of these files
    // schedules this test alone.
    let bundle = [
        ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
        ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
        ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
        ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
        ("sequence-types.yaml", include_str!("../../../../claudine/schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../../claudine/schemas/sequence.yaml")),
    ];
    let documents = repository(
        &process,
        &bundle,
        &[
            ("start-number", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/start-number.md")),
            ("prompt-with-malformed-sequence", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/prompt-with-malformed-sequence.md")),
            ("unrelated-ticket", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/unrelated-ticket.md")),
        ],
    );
    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(1), "{records:?}");
    assert_record(&records, "start-number", false, &[("/start", "42")]);
    assert_record(&records, "prompt-with-malformed-sequence", false, &[("/sequence", "tasks")]);
    assert_record(&records, "unrelated-ticket", true, &[]);
}

#[test]
fn parser_only_defects_pass_schema_validation() {
    let process = CliProcessFixture::new();
    // Read here rather than in a helper, so an edit to one of these files
    // schedules this test alone.
    let bundle = [
        ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
        ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
        ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
        ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
        ("sequence-types.yaml", include_str!("../../../../claudine/schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../../claudine/schemas/sequence.yaml")),
    ];
    let documents = repository(
        &process,
        &bundle,
        &[
            ("depth6", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth6.md")),
            ("depth5-guarded-bundle", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth5-guarded-bundle.md")),
            ("shell-expansion-in-initialize-set", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/shell-expansion-in-initialize-set.md")),
        ],
    );
    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(0), "{records:?}");
    for name in ["depth6", "depth5-guarded-bundle", "shell-expansion-in-initialize-set"] {
        assert_record(&records, name, true, &[]);
    }
}

#[test]
fn sequence_documents_select_the_sequence_trigger_and_validate_through_the_cli() {
    let process = CliProcessFixture::new();
    // Read here rather than in a helper, so an edit to one of these files
    // schedules this test alone.
    let bundle = [
        ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
        ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
        ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
        ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
        ("sequence-types.yaml", include_str!("../../../../claudine/schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../../claudine/schemas/sequence.yaml")),
    ];
    let documents = repository(
        &process,
        &bundle,
        &[
            ("example-containers-kinds", include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/example-containers-kinds.md")),
            ("level3-valid", include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/level3-valid.md")),
            ("group-execution", include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/group-execution.md")),
            ("root-fail-fast", include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/root-fail-fast.md")),
            ("sequence-mapping", include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/sequence-mapping.md")),
        ],
    );

    // A document whose only control is `sequence` selects sequence.yaml alone.
    let output = process
        .command()
        .args(["schema", "triggers"])
        .arg(&documents[0])
        .output()
        .expect("md runs");
    assert!(output.status.success(), "{output:?}");
    let trace = String::from_utf8_lossy(&output.stdout);
    let line = |trigger: &str| {
        trace
            .lines()
            .find(|line| line.contains(&format!("{trigger} ")))
            .unwrap_or_else(|| panic!("`{trigger}` is listed:\n{trace}"))
            .to_string()
    };
    assert!(!line("sequence.yaml").contains("not matched") && line("sequence.yaml").contains("matched"), "{trace}");
    assert!(line("claudine.yaml").contains("not matched"), "{trace}");

    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(1), "{records:?}");
    assert_record(&records, "example-containers-kinds", true, &[]);
    assert_record(&records, "level3-valid", true, &[]);
    assert_record(&records, "group-execution", false, &[("/sequence/0/group", "\"execution\"")]);
    assert_record(&records, "root-fail-fast", false, &[("/fail_fast", "null")]);
    assert_record(&records, "sequence-mapping", false, &[("/sequence", "tasks")]);
}
