//! `md schema validate` end to end for the core schema types of
//! `2026-10-16-core-schema-types`: the Phase A presence table (acceptance
//! criterion 1) with the nullable-union and container cases of criteria 3 and
//! 12, and a lifecycle-shaped schema whose `initialize` actions exclude
//! `shell` (criterion 7).
//!
//! Each test validates every document in one `md` process and matches the
//! NDJSON records back to their documents by file name.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

const INCLUSIVE: &str = "$schema:\n  - schema: 'file(required)'\n  - review: 'file(required)'\n";
const XOR: &str = "$schema:\n  - schema: 'file(required)'\n    review: null\n  - review: 'file(required)'\n    schema: null\n";

/// `(document name, frontmatter, expected validity)`.
fn cases() -> Vec<(&'static str, String, bool)> {
    let single = |declaration: &str, value: &str| {
        format!("$schema:\n  p: {declaration}\n{value}")
    };
    let mut cases = vec![
        ("file_present.md", single("'file(required)'", "p: spec.md\n"), true),
        ("file_null.md", single("'file(required)'", "p: null\n"), false),
        ("file_omitted.md", single("'file(required)'", ""), false),
    ];
    for (stem, declaration) in [
        ("null", "null"),
        ("null_tilde", "~"),
        ("null_quoted", "'null'"),
        ("null_required", "'null(required)'"),
    ] {
        cases.push((leak(format!("{stem}_null.md")), single(declaration, "p: null\n"), true));
        cases.push((leak(format!("{stem}_omitted.md")), single(declaration, ""), true));
        cases.push((leak(format!("{stem}_text.md")), single(declaration, "p: text\n"), false));
        cases.push((leak(format!("{stem}_empty.md")), single(declaration, "p: ''\n"), false));
        cases.push((leak(format!("{stem}_list.md")), single(declaration, "p: []\n"), false));
    }
    for (stem, declaration) in [
        ("unknown", "unknown"),
        ("any", "any"),
        ("unknown_required", "'unknown(required)'"),
        ("any_required", "'any(required)'"),
    ] {
        for (value_stem, value) in [
            ("null", "p: null\n"),
            ("omitted", ""),
            ("text", "p: text\n"),
            ("number", "p: 3\n"),
            ("nested", "p:\n    a: [1, null, x]\n"),
        ] {
            cases.push((leak(format!("{stem}_{value_stem}.md")), single(declaration, value), true));
        }
    }
    let both = "schema: spec.md\nreview: review.md\n";
    cases.extend([
        ("inclusive_schema.md", format!("{INCLUSIVE}schema: spec.md\n"), true),
        ("inclusive_review.md", format!("{INCLUSIVE}review: review.md\n"), true),
        ("inclusive_both.md", format!("{INCLUSIVE}{both}"), true),
        ("inclusive_neither.md", INCLUSIVE.to_string(), false),
        ("xor_schema_omitted.md", format!("{XOR}schema: spec.md\n"), true),
        ("xor_schema_null.md", format!("{XOR}schema: spec.md\nreview: null\n"), true),
        ("xor_review_null.md", format!("{XOR}review: review.md\nschema: null\n"), true),
        ("xor_both.md", format!("{XOR}{both}"), false),
        ("xor_neither.md", XOR.to_string(), false),
        ("xor_both_null.md", format!("{XOR}schema: null\nreview: null\n"), false),
    ]);
    // A required named type made optional by a `null` arm; its constraints
    // still apply to non-null values.
    let draft = "$schema:\n  code: 'string(required; min(3))'\n  draft: [code, null]\n  code_value: 'string'\ncode: abc\n";
    cases.extend([
        ("draft_omitted.md", draft.to_string(), true),
        ("draft_null.md", format!("{draft}draft: null\n"), true),
        ("draft_valid.md", format!("{draft}draft: abcd\n"), true),
        ("draft_short.md", format!("{draft}draft: ab\n"), false),
    ]);
    let unknown_arm = "$schema:\n  p: ['string(required)', unknown]\n";
    cases.extend([
        ("unknown_arm_omitted.md", unknown_arm.to_string(), true),
        ("unknown_arm_null.md", format!("{unknown_arm}p: null\n"), true),
    ]);
    // Item and child nullability leave the container required.
    for (stem, declaration, valid_value) in [
        ("null_list", "'null[](required)'", "p: [null]\n"),
        ("unknown_list", "'unknown[](required)'", "p: [1, x, null]\n"),
        ("nullable_child", "'{ c: null }(required)'", "p: {}\n"),
    ] {
        cases.push((leak(format!("{stem}_valid.md")), single(declaration, valid_value), true));
        cases.push((leak(format!("{stem}_omitted.md")), single(declaration, ""), false));
        cases.push((leak(format!("{stem}_null.md")), single(declaration, "p: null\n"), false));
    }
    cases.push(("null_list_text_item.md", single("'null[](required)'", "p: [x]\n"), false));
    // An ordinary optional concrete type can still become required.
    cases.push(("string_required_omitted.md", single("'string(required)'", ""), false));
    cases
}

fn leak(name: String) -> &'static str {
    Box::leak(name.into_boxed_str())
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).expect("write fixture");
}

#[test]
fn null_admitting_presence_matches_the_phase_a_table_end_to_end() {
    let process = CliProcessFixture::new();
    let dir = process.cwd();
    write(dir, "spec.md", "spec\n");
    write(dir, "review.md", "review\n");
    let cases = cases();
    for (name, frontmatter, _) in &cases {
        write(dir, name, &format!("---\n{frontmatter}---\nBody\n"));
    }

    assert_validity(&process, &cases);
}

/// Validates every case's document in one `md` run and asserts each record's
/// validity.
fn assert_validity(process: &CliProcessFixture, cases: &[(&str, String, bool)]) {
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .args(cases.iter().map(|(name, ..)| *name))
        .output()
        .expect("md runs");
    assert_eq!(output.status.code(), Some(1), "some cases are invalid");
    let records: BTreeMap<String, Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line).expect("NDJSON record");
            (record["file"].as_str().expect("file").to_string(), record)
        })
        .collect();

    let mut mismatches = Vec::new();
    for (name, frontmatter, expected) in cases {
        let record = records
            .get(*name)
            .unwrap_or_else(|| panic!("no record for {name}; stderr: {}", String::from_utf8_lossy(&output.stderr)));
        if record["valid"].as_bool() != Some(*expected) {
            mismatches.push(format!("{name}: expected valid={expected}\n{frontmatter}{record}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n\n"));
}

/// Closed action shapes: `initialize` takes a union without the `shell`
/// action, `start` a union with it.
const ACTION_TYPES: &str = "kind: schema\ntypes:\n  \
    say_action: '{ say: string(required) }'\n  \
    set_action: '{ set: object(required) }'\n  \
    shell_action: '{ shell: shell(required) }'\n  \
    initialize_action: [say_action, set_action]\n  \
    start_action: [say_action, set_action, shell_action]\n  \
    initialize_actions: initialize_action[]\n  \
    start_actions: start_action[]\n";

const LIFECYCLE_SCHEMA: &str = "$schema:\n  \
    initialize: initialize_actions@./actions.yaml\n  \
    start: start_actions@./actions.yaml\n";

#[test]
fn a_shell_action_is_rejected_under_initialize_and_accepted_under_start() {
    let process = CliProcessFixture::new();
    let dir = process.cwd();
    write(dir, "actions.yaml", ACTION_TYPES);
    write(dir, "lifecycle.yaml", LIFECYCLE_SCHEMA);
    let cases: Vec<(&str, String, bool)> = [
        ("init_say.md", "initialize:\n  - say: hi\n  - set: { a: 1 }\n", true),
        ("init_shell.md", "initialize:\n  - shell: echo hi\n", false),
        ("init_mixed.md", "initialize:\n  - say: hi\n    shell: echo hi\n", false),
        ("init_empty_action.md", "initialize:\n  - {}\n", false),
        ("start_shell.md", "start:\n  - shell: ls | wc -l; echo done\n", true),
        ("start_shell_boolean.md", "start:\n  - shell: true\n", true),
        ("start_mixed.md", "start:\n  - say: hi\n    shell: echo hi\n", false),
        ("start_blank_shell.md", "start:\n  - shell: '  '\n", false),
        ("start_empty_action.md", "start:\n  - {}\n", false),
    ]
    .into_iter()
    .map(|(name, body, valid)| (name, format!("$schema: ./lifecycle.yaml\n{body}"), valid))
    .collect();
    for (name, frontmatter, _) in &cases {
        write(dir, name, &format!("---\n{frontmatter}---\nBody\n"));
    }
    assert_validity(&process, &cases);
}
