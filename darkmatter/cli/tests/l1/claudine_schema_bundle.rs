//! `md schema validate` and `md schema triggers` against Claudine's shipped
//! schema bundle (`claudine/schemas/`, `2026-10-21-claudine-schemas`
//! criteria 1 and 5, `2026-10-26-task-inputs-and-providers` criterion 9, and
//! `2026-10-30-claudine-recursive-schemas` criteria 4 and 8):
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

/// The problem `name` reports at `pointer`.
fn problem_at<'a>(records: &'a BTreeMap<String, Value>, name: &str, pointer: &str) -> &'a Value {
    records[name]["problems"]
        .as_array()
        .expect("problems")
        .iter()
        .find(|problem| problem["path"] == pointer)
        .unwrap_or_else(|| panic!("{name}: no problem at `{pointer}` in {}", records[name]))
}

/// Asserts `name` reports nothing at `pointer`: a deferred operand stays
/// unjudged beside the sibling error it must not hide.
fn assert_no_problem_at(records: &BTreeMap<String, Value>, name: &str, pointer: &str) {
    let problems = records[name]["problems"].as_array().expect("problems");
    assert!(problems.iter().all(|problem| problem["path"] != pointer), "{name}: `{pointer}` is unjudged: {problems:?}");
}

/// The 1-based document line (the opening `---` is line 1) and column of
/// the first occurrence of `needle` in `text`.
fn source_position(text: &str, needle: &str) -> (u64, u64) {
    text.lines()
        .enumerate()
        .find_map(|(index, line)| line.find(needle).map(|column| (index as u64 + 1, column as u64 + 1)))
        .unwrap_or_else(|| panic!("`{needle}` occurs in the fixture"))
}

/// Asserts the trigger trace lists `trigger` as matched (`true`) or not.
fn assert_trigger(trace: &str, trigger: &str, matched: bool) {
    let line = trace
        .lines()
        .find(|line| line.contains(&format!("{trigger} ")))
        .unwrap_or_else(|| panic!("`{trigger}` is listed:\n{trace}"));
    assert_eq!(!line.contains("not matched") && line.contains("matched"), matched, "`{trigger}` matched = {matched}:\n{trace}");
}

/// The `md schema triggers` trace for `document`.
fn triggers(process: &CliProcessFixture, document: &Path) -> String {
    let output = process.command().args(["schema", "triggers"]).arg(document).output().expect("md runs");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
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

/// A single-action dictionary takes its verb's long form at the root of
/// `start` (the review's reproduction), in either branch, and as a loop's one
/// loose verb; a loose long form beside another loose verb is still refused,
/// and so is a conditional written as a stack's root dictionary.
#[test]
fn single_action_dictionaries_validate_through_the_cli() {
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
            ("single-action-root-dictionary", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/single-action-root-dictionary.md")),
            ("single-action-branch-dictionaries", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/single-action-branch-dictionaries.md")),
            ("single-action-loop-loose", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/single-action-loop-loose.md")),
            ("long-forms-in-loop-bundle", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/long-forms-in-loop-bundle.md")),
            ("conditional-root-dictionary", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/conditional-root-dictionary.md")),
        ],
    );
    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(1), "{records:?}");
    for name in ["single-action-root-dictionary", "single-action-branch-dictionaries", "single-action-loop-loose"] {
        assert_record(&records, name, true, &[]);
    }
    assert_record(&records, "long-forms-in-loop-bundle", false, &[("/loop/ensure_file", "file")]);
    assert_record(&records, "conditional-root-dictionary", false, &[("/start", "not valid under any of the schemas")]);
}

/// A malformed `{{ … }}` span in a string operand is a positioned syntax
/// problem, though the parameter's type accepts the text: whole value, named
/// operand with mixed text, and positional operand.
#[test]
fn malformed_templates_in_string_operands_are_positioned_problems() {
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
    let malformed =
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-malformed-string-template.md");
    let documents = repository(
        &process,
        &bundle,
        &[
            ("generated-verb-malformed-string-template", malformed),
            ("generated-verb-deferred", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred.md")),
        ],
    );

    let (code, records) = validate(&process, &documents);

    assert_eq!(code, Some(1), "{records:?}");
    assert_record(&records, "generated-verb-deferred", true, &[]);
    let name = "generated-verb-malformed-string-template";
    for (pointer, needle, located) in [
        ("/start/0/ensure_file", "malformed template `{{ 1 + }}`", "\"{{ 1 + }}\""),
        ("/start/1/trim/text", "malformed template `hello {{ 1 + }}`", "'hello"),
        ("/start/2/ensure_file/1", "malformed template `out/{{ 1 + }}.md`", "'out/"),
    ] {
        assert_record(&records, name, false, &[(pointer, needle)]);
        let problem = problem_at(&records, name, pointer);
        let (line, column) = source_position(malformed, located);
        assert_eq!((problem["line"].as_u64(), problem["column"].as_u64()), (Some(line), Some(column)), "{problem}");
    }
    assert_eq!(records[name]["problems"].as_array().map(Vec::len), Some(3), "{}", records[name]);
}

/// A malformed `{{ … }}` span in a hand-written verb's value (communication
/// text, `set` value, `shell` command, directive string, `with` overlay) is a
/// positioned syntax problem, as the lifecycle parser rejects it; the `set`
/// value the parser reads as a command is not one.
#[test]
fn malformed_templates_in_hand_written_actions_are_positioned_problems() {
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
    let malformed =
        include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/handwritten-action-malformed-template.md");
    let documents = repository(&process, &bundle, &[("handwritten-action-malformed-template", malformed)]);

    let (code, records) = validate(&process, &documents);

    assert_eq!(code, Some(1), "{records:?}");
    let name = "handwritten-action-malformed-template";
    let expected = [
        ("/start/0/say", "malformed template `{{ 1 + }}`", "\"{{ 1 + }}\""),
        ("/start/1/info/text", "malformed template `hello {{ 1 + }}`", "'hello"),
        ("/start/2/set/round", "malformed template `{{ fm.round + }}`", "'{{ fm.round"),
        ("/start/3/shell", "malformed template `echo {{ 2 * }}`", "'echo"),
        ("/start/4/then/0/error/reason", "malformed template `failed {{ 3 - }}`", "'failed"),
        ("/start/4/else/0/proxy/with/note", "malformed template `next {{ 4 / }}`", "'next"),
    ];
    for (pointer, needle, located) in expected {
        assert_record(&records, name, false, &[(pointer, needle)]);
        let problem = problem_at(&records, name, pointer);
        let (line, column) = source_position(malformed, located);
        assert_eq!((problem["line"].as_u64(), problem["column"].as_u64()), (Some(line), Some(column)), "{problem}");
    }
    assert_eq!(records[name]["problems"].as_array().map(Vec::len), Some(expected.len()), "{}", records[name]);
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

/// `2026-10-30-claudine-recursive-schemas` criteria 4 and 8 through the CLI:
/// every recursive-depth, generated-verb, and overload fixture of the shared
/// corpus gets its agreed schema verdict, problems carry their pointer and
/// document source position, and the trigger trace confirms `claudine.yaml`
/// was the schema that judged them.
#[test]
fn recursive_lifecycle_corpus_gets_the_agreed_verdicts_and_positions() {
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
    let wrong_verb = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth6-wrong-verb.md");
    let wrong_type = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-wrong-type.md");
    // A stored literal token holding `{{ fm.count }}` is data, judged as text.
    let literal_token = "---\nstart:\n    - abs: \"{{!data:v1:e3sgZm0uY291bnQgfX0}}\"\n---\n# literal token\n";
    let documents = repository(
        &process,
        &bundle,
        &[
            ("depth6", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth6.md")),
            ("depth6-wrong-verb", wrong_verb),
            ("depth1", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth1.md")),
            ("depth3", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth3.md")),
            ("depth5", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth5.md")),
            ("depth4-completion", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth4-completion.md")),
            ("depth5-guarded-bundle", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/depth5-guarded-bundle.md")),
            ("generated-verb-typed-value", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-typed-value.md")),
            ("generated-verb-wrong-type", wrong_type),
            ("generated-verb-convertible-text", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-convertible-text.md")),
            ("generated-verb-deferred", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred.md")),
            (
                "generated-verb-deferred-beside-unknown-parameter",
                include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-unknown-parameter.md"),
            ),
            (
                "generated-verb-deferred-beside-extra-argument",
                include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-extra-argument.md"),
            ),
            (
                "generated-verb-deferred-beside-bad-sibling",
                include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-bad-sibling.md"),
            ),
            ("generated-verb-malformed-template", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-malformed-template.md")),
            ("optional-nonnullable-null", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/optional-nonnullable-null.md")),
            ("ensure-file-overloads", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/ensure-file-overloads.md")),
            ("variadic-and-or", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/variadic-and-or.md")),
            ("generated-verb-literal-token", literal_token),
        ],
    );

    // The listing is evidence of which schema judged depth 6, never a verdict.
    let trace = triggers(&process, &documents[0]);
    assert_trigger(&trace, "claudine.yaml", true);
    assert_trigger(&trace, "sequence.yaml", false);

    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(1), "{records:?}");
    for name in [
        "depth6",
        "depth1",
        "depth3",
        "depth5",
        "depth4-completion",
        "depth5-guarded-bundle",
        "generated-verb-typed-value",
        "generated-verb-convertible-text",
        "generated-verb-deferred",
        "ensure-file-overloads",
        "variadic-and-or",
    ] {
        assert_record(&records, name, true, &[]);
    }

    let verb_pointer = "/success/0/then/0/then/0/then/0/then/0/then/0/then/0";
    assert_record(&records, "depth6-wrong-verb", false, &[(verb_pointer, "sya")]);
    let problem = problem_at(&records, "depth6-wrong-verb", verb_pointer);
    let (line, column) = source_position(wrong_verb, "sya:");
    assert_eq!((problem["line"].as_u64(), problem["column"].as_u64()), (Some(line), Some(column)), "{problem}");

    assert_record(&records, "generated-verb-wrong-type", false, &[("/start/0/pr_list", "three")]);
    let problem = problem_at(&records, "generated-verb-wrong-type", "/start/0/pr_list");
    let (line, column) = source_position(wrong_type, "three");
    assert_eq!((problem["line"].as_u64(), problem["column"].as_u64()), (Some(line), Some(column)), "{problem}");

    assert_record(&records, "generated-verb-deferred-beside-unknown-parameter", false, &[("/start/0/pr_list", "limit")]);
    assert_record(&records, "generated-verb-deferred-beside-extra-argument", false, &[("/start/0/min/2", "does not allow 3")]);
    assert_no_problem_at(&records, "generated-verb-deferred-beside-extra-argument", "/start/0/min/0");
    assert_record(&records, "generated-verb-deferred-beside-bad-sibling", false, &[("/start/0/min/0", "three")]);
    assert_no_problem_at(&records, "generated-verb-deferred-beside-bad-sibling", "/start/0/min/1");
    assert_record(&records, "generated-verb-malformed-template", false, &[("/start/0/pr_list", "{{ 1 + }}")]);
    assert_record(&records, "optional-nonnullable-null", false, &[("/start/0/ping/1", "null"), ("/start/1/ping/timeout", "null")]);
    assert_record(&records, "generated-verb-literal-token", false, &[("/start/0/abs", "")]);
    let problem = problem_at(&records, "generated-verb-literal-token", "/start/0/abs");
    let (line, column) = source_position(literal_token, "\"{{!data:");
    assert_eq!((problem["line"].as_u64(), problem["column"].as_u64()), (Some(line), Some(column)), "{problem}");
}

/// The four depth-four container controls select `sequence.yaml` alone and
/// validate through the CLI; a defect in the innermost container's events is
/// reported at its own item.
#[test]
fn depth_four_container_controls_validate_and_their_innermost_defects_are_located() {
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
    let seq_seq = include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/containers-depth4-seq-seq.md");
    let group_group = include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/containers-depth4-group-group.md");
    let edit = |text: &str, from: &str, to: &str| {
        assert!(text.contains(from), "the control still holds `{from}`");
        text.replacen(from, to, 1)
    };
    let misspelled_verb = edit(seq_seq, "        start:\n        - info: start", "        start:\n        - sya: start");
    let cancelled_retry = edit(seq_seq, "        cancelled:\n        - stop", "        cancelled:\n        - retry: 3");
    let initialize_shell = edit(group_group, "                initialize:\n                - info: initialize", "                initialize:\n                - shell: ls");
    let documents = repository(
        &process,
        &bundle,
        &[
            ("containers-depth4-seq-seq", seq_seq),
            ("containers-depth4-seq-group", include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/containers-depth4-seq-group.md")),
            ("containers-depth4-group-seq", include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/containers-depth4-group-seq.md")),
            ("containers-depth4-group-group", group_group),
            ("seq-seq-misspelled-verb", &misspelled_verb),
            ("seq-seq-cancelled-retry", &cancelled_retry),
            ("group-group-initialize-shell", &initialize_shell),
        ],
    );

    let trace = triggers(&process, &documents[0]);
    assert_trigger(&trace, "sequence.yaml", true);
    assert_trigger(&trace, "claudine.yaml", false);

    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(1), "{records:?}");
    for name in ["containers-depth4-seq-seq", "containers-depth4-seq-group", "containers-depth4-group-seq", "containers-depth4-group-group"] {
        assert_record(&records, name, true, &[]);
    }
    let seq_leaf = "/sequence/0/sequence/0/sequence/0/sequence/0";
    let group_leaf = "/sequence/0/group/tasks/0/group/tasks/0/group/tasks/0/group";
    assert_record(&records, "seq-seq-misspelled-verb", false, &[(&format!("{seq_leaf}/start/0"), "sya")]);
    assert_record(&records, "seq-seq-cancelled-retry", false, &[(&format!("{seq_leaf}/cancelled/0"), "retry")]);
    assert_record(&records, "group-group-initialize-shell", false, &[(&format!("{group_leaf}/initialize/0"), "shell")]);
}

/// A generated verb's runtime registration decides its literal operands at
/// every depth `md schema validate` reaches: a numeric-recovery fallback
/// passes although the projected subject type rejects the text, and
/// `length` refuses a boolean its projected `string` converts, positioned at
/// the operand, in an event stack, a conditional branch, and the innermost
/// container of a depth-four sequence.
#[test]
fn generated_verb_registration_rules_decide_through_the_cli() {
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
    let excluded = include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-excluded-source.md");
    let seq_seq = include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/containers-depth4-seq-seq.md");
    let innermost = "        start:\n        - info: start";
    assert!(seq_seq.contains(innermost), "the control still holds its innermost `start`");
    let nested_recovery = seq_seq.replacen(innermost, "        start:\n        - number: [pear, 7]", 1);
    let nested_excluded = seq_seq.replacen(innermost, "        start:\n        - length: [true]", 1);
    let documents = repository(
        &process,
        &bundle,
        &[
            ("generated-verb-numeric-recovery", include_str!("../../../../claudine/lib/tests/fixtures/claudine-schemas/generated-verb-numeric-recovery.md")),
            ("generated-verb-excluded-source", excluded),
            ("seq-seq-numeric-recovery", &nested_recovery),
            ("seq-seq-excluded-source", &nested_excluded),
        ],
    );

    let (code, records) = validate(&process, &documents);
    assert_eq!(code, Some(1), "{records:?}");
    assert_record(&records, "generated-verb-numeric-recovery", true, &[]);
    assert_record(&records, "seq-seq-numeric-recovery", true, &[]);
    assert_record(
        &records,
        "generated-verb-excluded-source",
        false,
        &[("/start/0/length/0", "does not accept a boolean"), ("/start/1/then/0/length/val", "does not accept a boolean")],
    );
    let problem = problem_at(&records, "generated-verb-excluded-source", "/start/0/length/0");
    let (line, column) = source_position(excluded, "true");
    assert_eq!((problem["line"].as_u64(), problem["column"].as_u64()), (Some(line), Some(column)), "{problem}");
    let seq_leaf = "/sequence/0/sequence/0/sequence/0/sequence/0";
    assert_record(&records, "seq-seq-excluded-source", false, &[(&format!("{seq_leaf}/start/0/length/0"), "boolean")]);
}
