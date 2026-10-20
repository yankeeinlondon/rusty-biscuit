//! `md schema validate` end to end against the real shipped artifacts migrated
//! by `2026-10-16-core-schema-types`: the Claudine side-effect partial and the
//! three dialect trigger fixtures, plus a test-owned schema for the presence of
//! a `generated; required` object.
//!
//! Each test copies the artifacts into an isolated fixture directory, validates
//! every document in one `md` run, and matches the NDJSON records back to their
//! documents by file name. Validation is passive, so a second run must report
//! the same records and leave every document byte for byte unchanged.

use crate::common;

use common::{CliProcessFixture, write};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `(document name, frontmatter, expected validity, text a problem message
/// must contain)`.
type Case = (&'static str, String, bool, Option<&'static str>);

/// Writes each case under `dir` and returns the written paths.
fn write_cases(dir: &Path, cases: &[Case]) -> Vec<PathBuf> {
    cases
        .iter()
        .map(|(name, frontmatter, ..)| {
            let path = dir.join(name);
            write(&path, &format!("---\n{frontmatter}---\nBody\n"));
            path
        })
        .collect()
}

/// One `md schema validate --format json` run over `documents`, keyed by the
/// record's `file`.
fn validate(process: &CliProcessFixture, extra: &[&str], documents: &[PathBuf]) -> BTreeMap<String, Value> {
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .args(extra)
        .args(documents)
        .output()
        .expect("md runs");
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "md schema validate must validate, not fail to load: {output:?}"
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line).expect("NDJSON record");
            let file = record["file"].as_str().expect("file").replace('\\', "/");
            let name = file.rsplit('/').next().expect("file name").to_string();
            (name, record)
        })
        .collect()
}

/// Asserts each case's validity and expected problem text, twice, and that
/// validation left every document unchanged.
fn assert_cases(process: &CliProcessFixture, extra: &[&str], cases: &[Case], documents: &[PathBuf]) {
    let before: Vec<Vec<u8>> = documents.iter().map(|path| std::fs::read(path).expect("readable")).collect();
    let records = validate(process, extra, documents);

    let mut mismatches = Vec::new();
    for (name, frontmatter, valid, problem) in cases {
        let Some(record) = records.get(*name) else {
            mismatches.push(format!("{name}: no record"));
            continue;
        };
        if record["valid"].as_bool() != Some(*valid) {
            mismatches.push(format!("{name}: expected valid={valid}\n{frontmatter}{record}"));
        }
        let messages: Vec<&str> = record["problems"]
            .as_array()
            .map(|problems| problems.iter().filter_map(|problem| problem["message"].as_str()).collect())
            .unwrap_or_default();
        if let Some(problem) = problem
            && !messages.iter().any(|message| message.contains(problem))
        {
            mismatches.push(format!("{name}: no problem names `{problem}`\n{record}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n\n"));

    assert_eq!(validate(process, extra, documents), records, "a repeated validation must agree");
    let after: Vec<Vec<u8>> = documents.iter().map(|path| std::fs::read(path).expect("readable")).collect();
    assert!(before == after, "md schema validate must not modify a document");
}

/// The migrated side-effect partial: `set_frontmatter.value` admits null and
/// every native value, `merge_frontmatter` takes an object `obj` and no
/// `prop`, and a leftover `prop` is an unexpected key.
#[test]
fn side_effect_partial_validates_set_and_merge_actions() {
    let process = CliProcessFixture::new();
    let dir = process.cwd();
    write(
        &dir.join("side-effect.yaml"),
        include_str!("../../../../claudine/docs/schemas/partials/side-effect.yaml"),
    );
    write(&dir.join("target.md"), "---\ntitle: Target\n---\nBody\n");
    let effect = |action: &str, body: &str| {
        format!("$schema:\n  effect: side_effect@./side-effect.yaml\neffect:\n  {action}:\n    file: ./target.md\n{body}")
    };
    let set = |value: &str| effect("set_frontmatter", &format!("    prop: title\n    value: {value}\n"));
    let merge = |body: &str| effect("merge_frontmatter", body);

    let cases: Vec<Case> = vec![
        ("set_string.md", set("hello"), true, None),
        ("set_null.md", set("null"), true, None),
        ("set_value_omitted.md", effect("set_frontmatter", "    prop: title\n"), true, None),
        ("set_object.md", set("{ a: 1, b: { c: null } }"), true, None),
        ("set_array.md", set("[1, a, true, null]"), true, None),
        ("set_prop_omitted.md", effect("set_frontmatter", "    value: hello\n"), false, Some("prop")),
        ("merge_obj.md", merge("    obj: { title: New, tags: [a, null] }\n"), true, None),
        ("merge_empty_obj.md", merge("    obj: {}\n"), true, None),
        ("merge_obj_omitted.md", merge(""), false, Some("obj")),
        ("merge_obj_null.md", merge("    obj: null\n"), false, Some("null")),
        ("merge_obj_scalar.md", merge("    obj: title\n"), false, Some("object")),
        ("merge_leftover_prop.md", merge("    obj: { a: 1 }\n    prop: title\n"), false, Some("'prop' was unexpected")),
    ];
    let documents = write_cases(dir, &cases);

    assert_cases(&process, &[], &cases, &documents);
}

/// Each shipped dialect trigger selects its document and the document is then
/// validated against that trigger's target: a valid document passes, and a
/// document missing the target's required key fails naming it. Without
/// trigger discovery the same documents validate against nothing.
#[test]
fn each_migrated_trigger_selects_and_validates_its_target() {
    let process = CliProcessFixture::new();
    assert!(process.initialize_repository(), "git init");
    let schemas = process.cwd().join("schemas");
    for (name, content) in [
        (
            "claudine.trigger.yaml",
            include_str!("../../../tests/fixtures/schema-triggers/schemas/claudine.trigger.yaml"),
        ),
        (
            "claudine.yaml",
            include_str!("../../../tests/fixtures/schema-triggers/schemas/claudine.yaml"),
        ),
        (
            "inline-compose.trigger.yaml",
            include_str!("../../../tests/fixtures/schema-triggers/schemas/inline-compose.trigger.yaml"),
        ),
        (
            "inline-compose.yaml",
            include_str!("../../../tests/fixtures/schema-triggers/schemas/inline-compose.yaml"),
        ),
        (
            "sequence.trigger.yaml",
            include_str!("../../../tests/fixtures/schema-triggers/schemas/sequence.trigger.yaml"),
        ),
        (
            "sequence.yaml",
            include_str!("../../../tests/fixtures/schema-triggers/schemas/sequence.yaml"),
        ),
    ] {
        write(&schemas.join(name), content);
    }
    let docs = process.cwd().join("docs");

    let cases: Vec<Case> = vec![
        ("claudine_valid.md", "kind: claudine\nprovider: claude\n".into(), true, None),
        ("claudine_invalid.md", "kind: claudine\n".into(), false, Some("\"provider\" is a required property")),
        ("inline_valid.md", "prompt: Explain.\noutput: out.md\n".into(), true, None),
        ("inline_invalid.md", "prompt: Explain.\n".into(), false, Some("\"output\" is a required property")),
        ("sequence_valid.md", "steps: []\nsequence_name: nightly\n".into(), true, None),
        ("sequence_items_valid.md", "steps: [1, a, null]\nsequence_name: nightly\n".into(), true, None),
        ("sequence_invalid.md", "steps: [build]\n".into(), false, Some("\"sequence_name\" is a required property")),
        // A null `steps` is absent, so no dialect is selected.
        ("steps_null.md", "steps: null\n".into(), true, None),
    ];
    let documents = write_cases(&docs, &cases);
    assert_cases(&process, &[], &cases, &documents);

    let untriggered: Vec<Case> = cases.iter().map(|(name, frontmatter, ..)| (*name, frontmatter.clone(), true, None)).collect();
    assert_cases(&process, &["--no-trigger-schemas"], &untriggered, &documents);
}

/// A `state: object(generated; required)` property: authored documents may
/// omit it, but a present value must be an object and never null, while the
/// optional `previous_state`/`next_state` (`any`) admit null.
#[test]
fn generated_required_object_state_stays_an_object() {
    let process = CliProcessFixture::new();
    let dir = process.cwd();
    write(
        &dir.join("sequence-state.yaml"),
        "$schema:\n  state: object(generated; required)\n  previous_state: any\n  next_state: any\n",
    );
    let schema = |values: &str| format!("$schema: ./sequence-state.yaml\n{values}");

    let cases: Vec<Case> = vec![
        ("state_omitted.md", schema(""), true, None),
        ("state_object.md", schema("state:\n  phase: draft\n"), true, None),
        ("state_null.md", schema("state: null\n"), false, Some("null is not of type \"object\"")),
        ("state_number.md", schema("state: 5\n"), false, Some("5 is not of type \"object\"")),
        ("state_list.md", schema("state: [1]\n"), false, Some("is not of type \"object\"")),
        ("neighbor_states.md", schema("previous_state: null\nnext_state: { step: 2 }\n"), true, None),
    ];
    let documents = write_cases(dir, &cases);

    assert_cases(&process, &["--no-trigger-schemas"], &cases, &documents);
}
