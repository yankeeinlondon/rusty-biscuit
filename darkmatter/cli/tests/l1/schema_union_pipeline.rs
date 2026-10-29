//! References to union-typed named types (`U[]`, `U(required)`) through the
//! real `md` binary: `md schema validate` on the Content Policy acceptance
//! fixture, `md schema about`'s shape catalog, schema-aware assignments, and
//! the coercion and eager-file rewriting `md compose` applies to union items.
//!
//! The document-data robustness matrix lives in `schema_union_matrix.rs` and
//! shares this file's report helpers.

use std::fs;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::testing::strip_ansi_codes;
use serde_json::{Value, json};

use crate::common::CliProcessFixture;
use crate::union_fixtures::{
    CONTENT_POLICY_CASES, CONTENT_POLICY_FILE, CONTENT_POLICY_LIST, FILE_TYPES, NESTED_UNION_FILE,
    NESTED_UNION_TYPES, content_policy_union_schema, write_files,
};

/// The public outcome of `md schema validate --format json` for one document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Expected {
    /// Exit 0 and a report with no problems.
    Valid,
    /// Exit 1 and exactly these problem paths, in any order.
    Invalid(&'static [&'static str]),
    /// Exit 3: the frontmatter failed to load (`error: frontmatter_parse`).
    LoadError,
}

/// The Content Policy acceptance document: `frontmatter` follows the
/// `$schema` declaration, and the body is ordinary Markdown.
pub(crate) fn content_policy_document(frontmatter: &str) -> String {
    format!(
        "---\n$schema:\n  content_policy: \"{CONTENT_POLICY_LIST}\"\n{frontmatter}---\nBody with a [link](./notes.md).\n"
    )
}

/// A fixture whose cwd holds the acceptance schema (built from `shipped`) and
/// `document` as `doc.md`.
pub(crate) fn content_policy_workspace(
    shipped: &str,
    document: &str,
) -> (CliProcessFixture, PathBuf) {
    let process = CliProcessFixture::new();
    let schema = content_policy_union_schema(shipped);
    write_files(
        process.cwd(),
        &[(CONTENT_POLICY_FILE, &schema), ("doc.md", document)],
    );
    let doc = process.cwd().join("doc.md");
    (process, doc)
}

/// Runs `md schema validate --format json` on `doc`, asserts the file's bytes
/// did not change, and asserts the exit code and report match `expected`.
pub(crate) fn assert_validates_unchanged(
    process: &CliProcessFixture,
    doc: &Path,
    expected: Expected,
) {
    let before = fs::read(doc).expect("read document before validation");
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(doc)
        .output()
        .expect("run md schema validate");
    let after = fs::read(doc).expect("read document after validation");
    assert_eq!(
        after,
        before,
        "md schema validate modified {}",
        doc.display()
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let context = format!(
        "document:\n{}\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&before),
        String::from_utf8_lossy(&output.stderr),
    );
    let report: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("expected one JSON report ({err})\n{context}"));
    let mut paths: Vec<&str> = report["problems"]
        .as_array()
        .unwrap_or_else(|| panic!("report has no problems array\n{context}"))
        .iter()
        .map(|problem| problem["path"].as_str().expect("problem path"))
        .collect();
    paths.sort_unstable();
    let code = output.status.code();
    match expected {
        Expected::Valid => {
            assert_eq!(code, Some(0), "{context}");
            assert_eq!(report["valid"], json!(true), "{context}");
            assert!(paths.is_empty(), "{context}");
        }
        Expected::Invalid(expected_paths) => {
            let mut expected_paths = expected_paths.to_vec();
            expected_paths.sort_unstable();
            assert_eq!(code, Some(1), "{context}");
            assert_eq!(report["valid"], json!(false), "{context}");
            assert!(report.get("error").is_none(), "{context}");
            assert_eq!(paths, expected_paths, "{context}");
        }
        Expected::LoadError => {
            assert_eq!(code, Some(3), "{context}");
            assert_eq!(report["error"], json!("frontmatter_parse"), "{context}");
            assert!(paths.is_empty(), "{context}");
        }
    }
}

// ── Acceptance criterion 1: one test per `CONTENT_POLICY_CASES` entry ──────

/// The public outcome of each `CONTENT_POLICY_CASES` entry, by index. A
/// rejected item is reported at its index; a missing `rule` fails every arm
/// and is reported at the item, while a present but invalid field narrows to
/// the field.
const ACCEPTANCE_EXPECTED: &[Expected] = &[
    Expected::Valid,
    Expected::Valid,
    Expected::Valid,
    Expected::Invalid(&["/content_policy/0"]),
    Expected::Invalid(&["/content_policy"]),
    Expected::Invalid(&["/content_policy"]),
    Expected::Invalid(&["/content_policy/0"]),
    Expected::Invalid(&["/content_policy/0"]),
    Expected::Invalid(&["/content_policy/0/rule"]),
    Expected::Invalid(&["/content_policy/0/action"]),
];

fn acceptance_case(shipped: &str, index: usize) {
    let (frontmatter, valid) = CONTENT_POLICY_CASES[index];
    let expected = ACCEPTANCE_EXPECTED[index];
    assert_eq!(
        valid,
        expected == Expected::Valid,
        "the shared fixture and this table disagree about case {index}",
    );
    let (process, doc) = content_policy_workspace(shipped, &content_policy_document(frontmatter));
    assert_validates_unchanged(&process, &doc, expected);
}

#[test]
fn acceptance_cases_each_have_an_expected_report_and_a_test() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    // Whitespace is dropped so a call the formatter wraps still matches.
    let source: String = include_str!("schema_union_pipeline.rs")
        .split_whitespace()
        .collect();
    assert_eq!(ACCEPTANCE_EXPECTED.len(), CONTENT_POLICY_CASES.len());
    for index in 0..CONTENT_POLICY_CASES.len() {
        let call = format!("acceptance_case(shipped,{index});");
        assert_eq!(
            source.matches(&call).count(),
            1,
            "`{call}` must appear exactly once"
        );
    }
}

#[test]
fn acceptance_mixed_scalar_and_object_list_validates() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 0);
}

#[test]
fn acceptance_missing_property_validates() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 1);
}

#[test]
fn acceptance_null_property_validates() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 2);
}

#[test]
fn acceptance_null_item_is_rejected_at_its_index() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 3);
}

#[test]
fn acceptance_scalar_in_place_of_the_list_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 4);
}

#[test]
fn acceptance_empty_list_fails_min_items() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 5);
}

#[test]
fn acceptance_item_matching_no_arm_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 6);
}

#[test]
fn acceptance_object_item_without_rule_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 7);
}

#[test]
fn acceptance_object_item_with_null_rule_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 8);
}

#[test]
fn acceptance_object_item_with_unknown_action_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    acceptance_case(shipped, 9);
}

// ── Union items under validation and composition ─────────────────────────

/// `count` items coerce only when exactly one arm's candidate validates;
/// every `label` item is accepted by both arms, so none is coerced.
const ITEM_TYPES: &str = "kind: schema\ntypes:\n  count: [number, \"{ name: string(required) }\"]\n  label: [number, string]\n";

const UNION_ITEMS_SCHEMA: &str = "$schema:\n  counts: \"count[]@./items.yaml\"\n  labels: \"label[]@./items.yaml\"\n  assets: \"asset[]@./files.yaml\"\n  lazy: \"lazy_asset[]@./files.yaml\"\n";

/// A repository-shaped cwd (`.git` marker, so the eager rewrite projects
/// repository-relative paths) holding both type files, `area/a.md`, and
/// `doc.md` with `items` after the union-items `$schema`.
fn union_items_workspace(items: &str, body: &str) -> (CliProcessFixture, PathBuf) {
    let process = CliProcessFixture::new();
    let root = process.cwd();
    fs::create_dir_all(root.join(".git")).expect("create .git marker");
    fs::create_dir_all(root.join("area")).expect("create area");
    let document = format!("---\n{UNION_ITEMS_SCHEMA}{items}---\n{body}");
    write_files(
        root,
        &[
            ("items.yaml", ITEM_TYPES),
            ("files.yaml", FILE_TYPES),
            ("area/a.md", "# A\n"),
            ("doc.md", &document),
        ],
    );
    let doc = root.join("doc.md");
    (process, doc)
}

const NORMALIZABLE_ITEMS: &str = "counts: [\"42\", {name: x}]\nlabels: [\"42\", 7]\nassets: [\"./area/a.md\", 3]\nlazy: [\"./area/a.md\"]\n";

const NORMALIZATION_PROBE: &str = "next={{counts[0] + 1}} asset={{assets[0]}} lazy={{lazy[0]}}\n";

/// `md compose --frontmatter` stdout and its reparsed frontmatter.
fn compose(process: &CliProcessFixture, doc: &Path) -> (String, serde_json::Map<String, Value>) {
    let output = process
        .command()
        .args(["compose", "--frontmatter"])
        .arg(doc)
        .output()
        .expect("run md compose");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "md compose failed\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let reparsed: Markdown = stdout.as_str().into();
    let frontmatter = reparsed
        .frontmatter()
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    (stdout, frontmatter)
}

#[test]
fn validate_leaves_coercible_and_eager_file_union_items_unchanged() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let (process, doc) = union_items_workspace(NORMALIZABLE_ITEMS, "Body\n");
    assert_validates_unchanged(&process, &doc, Expected::Valid);
}

#[test]
fn validate_leaves_a_document_with_invalid_union_items_unchanged() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let items = "counts: [\"42\", {}]\nlabels: [\"42\"]\nassets: [\"./area/a.md\", \"./area/missing.md\"]\nlazy: [\"./area/a.md\"]\n";
    let (process, doc) = union_items_workspace(items, "Body\n");
    assert_validates_unchanged(
        &process,
        &doc,
        Expected::Invalid(&["/assets/1", "/counts/1"]),
    );
}

#[test]
fn compose_coerces_an_unambiguous_union_item_and_keeps_ambiguous_items() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let (process, doc) = union_items_workspace(NORMALIZABLE_ITEMS, NORMALIZATION_PROBE);
    let source = fs::read(&doc).expect("read source");

    let (stdout, frontmatter) = compose(&process, &doc);

    assert_eq!(
        frontmatter["counts"],
        json!([42, {"name": "x"}]),
        "{stdout}"
    );
    // Both `label` arms accept `"42"` and `7`, so neither is coerced.
    assert_eq!(frontmatter["labels"], json!(["42", 7]), "{stdout}");
    // Normalization precedes body expressions: the body sees the number.
    assert!(stdout.contains("next=43 "), "{stdout}");
    assert_eq!(
        fs::read(&doc).expect("reread source"),
        source,
        "compose wrote the source"
    );
}

#[test]
fn compose_converts_items_of_a_postfixed_nested_union_like_its_direct_control() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let root = process.cwd();
    // `toggle` is `boolish | number`: `"TRUE"` matches `boolish` exactly and
    // is kept as written, `"7"` is no boolean word and converts to `7`.
    let document = "---\n$schema:\n  direct: \"direct[]@./types.yaml\"\n  nested: \"nested[]@./types.yaml\"\n\
                    direct: [\"TRUE\", \"7\"]\nnested: [\"TRUE\", \"7\"]\n---\nBody\n";
    write_files(root, &[(NESTED_UNION_FILE, NESTED_UNION_TYPES), ("doc.md", document)]);

    let (stdout, frontmatter) = compose(&process, &root.join("doc.md"));

    assert_eq!(frontmatter["direct"], json!(["TRUE", 7]), "{stdout}");
    assert_eq!(frontmatter["nested"], json!(["TRUE", 7]), "{stdout}");
}

#[test]
fn compose_rewrites_an_eager_file_union_item_and_keeps_lazy_items() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let (process, doc) = union_items_workspace(NORMALIZABLE_ITEMS, NORMALIZATION_PROBE);

    let (stdout, frontmatter) = compose(&process, &doc);

    assert_eq!(frontmatter["assets"], json!(["area/a.md", 3]), "{stdout}");
    assert_eq!(frontmatter["lazy"], json!(["./area/a.md"]), "{stdout}");
    assert!(
        stdout.contains(" asset=area/a.md lazy=./area/a.md"),
        "{stdout}"
    );
}

#[test]
fn recomposing_normalized_union_items_is_idempotent_and_inserts_nothing() {
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    let (process, doc) = union_items_workspace(NORMALIZABLE_ITEMS, NORMALIZATION_PROBE);

    let (first, first_frontmatter) = compose(&process, &doc);
    let persisted = process.cwd().join("persisted.md");
    fs::write(&persisted, &first).expect("persist composed output");
    let (second, second_frontmatter) = compose(&process, &persisted);

    assert_eq!(second, first);
    assert_eq!(second_frontmatter, first_frontmatter);
    let mut keys: Vec<&str> = first_frontmatter.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["$schema", "assets", "counts", "labels", "lazy"]);
    assert_eq!(first_frontmatter["counts"][1], json!({"name": "x"}));
}

// ── Rejected operations through a postfixed alias ────────────────────────

/// A union alias that already carries a postfix is still a union: the
/// forbidden value constraint and the `[]` over an array arm fail schema
/// resolution (exit 2) naming the alias, while an allowed container postfix
/// through the same alias validates.
#[test]
fn schema_validate_rejects_union_operations_through_a_postfixed_alias() {
    let process = CliProcessFixture::new();
    let types = "kind: schema\ntypes:\n  choice: [string, number]\n  required_choice: choice(required)\n  list_choice: [\"string[]\", number]\n  required_list_choice: list_choice(required)\n";
    write_files(
        process.cwd(),
        &[
            ("types.yaml", types),
            (
                "value.md",
                "---\n$schema:\n  p: required_choice(max(2))@./types.yaml\np: abcd\n---\nBody\n",
            ),
            (
                "array_arm.md",
                "---\n$schema:\n  p: required_list_choice[]@./types.yaml\np: [[a], 3]\n---\nBody\n",
            ),
            (
                "allowed.md",
                "---\n$schema:\n  p: \"required_choice[](min(1); max(3); unique)@./types.yaml\"\np: [a, 3]\n---\nBody\n",
            ),
        ],
    );
    let validate = |doc: &str| {
        let output = process
            .command()
            .args(["schema", "validate", "--format", "json", doc])
            .output()
            .expect("run md schema validate");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let report: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|err| panic!("expected one JSON report ({err}): {stdout}"));
        (output.status.code(), report)
    };

    for (doc, needle) in [
        ("value.md", "constraint `max` cannot apply"),
        ("array_arm.md", "union arm 0 is already an array"),
    ] {
        let (code, report) = validate(doc);
        assert_eq!(code, Some(2), "{doc}: {report}");
        assert_eq!(report["error"], json!("schema"), "{doc}: {report}");
        let message = report["message"].as_str().expect("schema error message");
        assert!(message.contains(needle), "{doc}: {message}");
        assert!(message.contains("`required_"), "{doc} names the alias: {message}");
    }
    let (code, report) = validate("allowed.md");
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["valid"], json!(true), "{report}");
}

// ── A list before `[]` on a union reference ──────────────────────────────

/// The review's cross-file reproduction: `eager` before `[]` validates as
/// `[]` followed by `eager` does, while a value constraint there, or any list
/// before `[]` on a single named type, fails schema resolution (exit 2).
#[test]
fn schema_validate_accepts_eager_before_the_array_suffix_and_rejects_value_constraints() {
    let process = CliProcessFixture::new();
    let types = "kind: schema\ntypes:\n  choice: [string, number]\n  name: string\n";
    let document = |reference: &str| format!("---\n$schema:\n  p: \"{reference}\"\np: [a, 3]\n---\nBody\n");
    write_files(
        process.cwd(),
        &[
            ("types.yaml", types),
            ("before.md", &document("choice(eager)[]@./types.yaml")),
            ("after.md", &document("choice[](eager)@./types.yaml")),
            ("value.md", &document("choice(max(2))[]@./types.yaml")),
            ("single.md", &document("name(eager)[]@./types.yaml")),
        ],
    );
    let validate = |doc: &str| {
        let output = process
            .command()
            .args(["schema", "validate", "--format", "json", doc])
            .output()
            .expect("run md schema validate");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let report: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|err| panic!("expected one JSON report ({err}): {stdout}"));
        (output.status.code(), report)
    };

    for doc in ["before.md", "after.md"] {
        let (code, report) = validate(doc);
        assert_eq!(code, Some(0), "{doc}: {report}");
        assert_eq!(report["valid"], json!(true), "{doc}: {report}");
    }
    for (doc, needle) in [
        ("value.md", "constraint `max` cannot apply to a union-typed type"),
        ("single.md", "constraint `eager` before `[]`"),
    ] {
        let (code, report) = validate(doc);
        assert_eq!(code, Some(2), "{doc}: {report}");
        assert_eq!(report["error"], json!("schema"), "{doc}: {report}");
        let message = report["message"].as_str().expect("schema error message");
        assert!(message.contains(needle), "{doc}: {message}");
    }
}

// ── Schema-aware assignments and the shape catalog ───────────────────────

/// `version=1.10` parses as the YAML float `1.1`; a string-preferring
/// property keeps the raw `1.10`, which only the string arm's pattern accepts.
/// A union written inline, referenced bare, and referenced with a marker must
/// all keep it.
#[test]
fn schema_validate_assignment_keeps_raw_text_for_every_union_reference_form() {
    let process = CliProcessFixture::new();
    let root = process.cwd();
    let ver = "kind: schema\ntypes:\n  ver: [\"string(pattern(^1\\\\.10$))\", boolean]\n";
    write_files(
        root,
        &[
            ("ver.yaml", ver),
            (
                "inline.md",
                "---\n$schema:\n  version: ['string(pattern(^1\\.10$))', boolean]\n---\nB\n",
            ),
            (
                "bare.md",
                "---\n$schema:\n  version: \"ver@./ver.yaml\"\n---\nB\n",
            ),
            (
                "marked.md",
                "---\n$schema:\n  version: \"ver(required)@./ver.yaml\"\n---\nB\n",
            ),
        ],
    );

    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .args(["inline.md", "bare.md", "marked.md", "version=1.10"])
        .output()
        .expect("run md schema validate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let reports: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("NDJSON report"))
        .collect();
    assert_eq!(reports.len(), 3, "{stdout}");
    for report in &reports {
        assert_eq!(report["valid"], json!(true), "{stdout}");
    }
    assert_eq!(output.status.code(), Some(0), "{stdout}");
}

#[test]
fn schema_about_renders_the_named_union_reference_shapes() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success(), "md schema about failed");
    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    // Wrapping depends on the render width; compare whitespace-collapsed text.
    let text = plain.split_whitespace().collect::<Vec<_>>().join(" ");

    let shapes = text.find("Schema Shapes").expect("Schema Shapes section");
    let types = text.find("Type System").expect("Type System section");
    for needle in [
        "Array of a named union.",
        "steps: step[](min(1); unique) -> The steps to run",
        "Marked union reference.",
        "answer: choice(required) -> The chosen value",
    ] {
        let at = text
            .find(needle)
            .unwrap_or_else(|| panic!("missing `{needle}`:\n{plain}"));
        assert!(
            shapes < at && at < types,
            "`{needle}` is outside Schema Shapes"
        );
    }
}
