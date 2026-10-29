//! Recursive named types through the normal `md` invocation paths:
//! `md schema validate` (positioned failures, presence, passivity,
//! assignments), `md compose` (conversion below the former expansion cutoff,
//! atomic failure, idempotent normalization, the design-time call check), and
//! `md schema about`. Every fixture is written into a `CliProcessFixture`
//! workspace; nothing reads the checkout.

use std::path::Path;

use crate::common;

use biscuit_file::FileResolutionContext;
use common::CliProcessFixture;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::Value;

/// `item` requires `children`, so `children: []` is the finite leaf.
const TREE_SCHEMA: &str =
    "$schema:\n  item: \"{ value: string(required), children: item[](required) }\"\n  tree: item(required)\n";

/// Block YAML for `key:` holding a `{ value, children }` chain `depth` levels
/// deep; `n<level>` fills every `value` but the deepest, which is `leaf`.
fn nested_tree(key: &str, depth: usize, leaf: &str) -> String {
    let mut out = format!("{key}:\n");
    let mut indent = String::from("  ");
    for level in 0..depth {
        let value = if level + 1 == depth { leaf.to_string() } else { format!("n{level}") };
        if level == 0 {
            out.push_str(&format!("{indent}value: {value}\n"));
        } else {
            out.push_str(&format!("{indent}- value: {value}\n"));
            indent.push_str("  ");
        }
        let children = if level + 1 == depth { " []" } else { "" };
        out.push_str(&format!("{indent}children:{children}\n"));
    }
    out
}

/// The one-based `(line, column)` of the first occurrence of `needle` in
/// `text`.
fn position_of(text: &str, needle: &str) -> (u64, u64) {
    let (index, line) = text
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} is not in the fixture"));
    let column = line.find(needle).expect("needle on its line") + 1;
    ((index + 1) as u64, column as u64)
}

/// Runs `md schema validate --format json` on `document` (plus any
/// assignments) and returns the exit code and the report's `problems`.
fn validate_json(fixture: &CliProcessFixture, document: &Path, extra: &[&str]) -> (Option<i32>, Vec<Value>) {
    let output = fixture
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(document)
        .args(extra)
        .output()
        .expect("run md schema validate");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!("not one JSON report ({error}):\n{stdout}\nstderr: {}", String::from_utf8_lossy(&output.stderr))
    });
    let problems = report["problems"].as_array().cloned().unwrap_or_default();
    (output.status.code(), problems)
}

/// `(path, line, column, message)` of each reported problem.
fn positioned(problems: &[Value]) -> Vec<(String, u64, u64, String)> {
    problems
        .iter()
        .map(|problem| {
            (
                problem["path"].as_str().unwrap_or_default().to_string(),
                problem["line"].as_u64().unwrap_or_default(),
                problem["column"].as_u64().unwrap_or_default(),
                problem["message"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

/// The same tuple from the library validator DMLS publishes from, run in
/// process over the same text and directory.
fn library_positioned(dir: &Path, text: &str) -> Vec<(String, u64, u64, String)> {
    let markdown: Markdown = text.to_string().into();
    let report = DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .validate(&markdown)
        .expect("the document's schema resolves");
    report
        .problems
        .iter()
        .map(|problem| {
            (
                problem.path.clone(),
                u64::from(problem.line.unwrap_or_default()),
                u64::from(problem.column.unwrap_or_default()),
                problem.message.clone(),
            )
        })
        .collect()
}

// ── Recursive validation at depth ≥ 6 ─────────────────────────────────────

#[test]
fn a_self_recursive_document_seven_levels_deep_validates() {
    let fixture = CliProcessFixture::named("schema_recursive_direct_good");
    let document =
        fixture.write_file("cwd/good.md", &format!("---\n{TREE_SCHEMA}{}---\nbody\n", nested_tree("tree", 7, "deep")));

    let (code, problems) = validate_json(&fixture, &document, &[]);

    assert_eq!(code, Some(0), "{problems:#?}");
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_self_recursive_failure_seven_levels_deep_reports_its_path_and_position() {
    let fixture = CliProcessFixture::named("schema_recursive_direct_bad");
    // A list has no conversion to text; a deep number would convert.
    let text = format!("---\n{TREE_SCHEMA}{}---\nbody\n", nested_tree("tree", 7, "[42]"));
    let document = fixture.write_file("cwd/bad.md", &text);
    let pointer = format!("/tree{}/value", "/children/0".repeat(6));
    let (line, column) = position_of(&text, "[42]");

    let (code, problems) = validate_json(&fixture, &document, &[]);

    assert_eq!(code, Some(1), "{problems:#?}");
    let reported = positioned(&problems);
    assert_eq!(reported, [(pointer.clone(), line, column, "[42] is not of type \"string\"".to_string())]);
    assert_eq!(reported, library_positioned(fixture.cwd(), &text), "md and the editor's validator agree");

    // The pretty report carries the same path and position.
    let output = fixture.command().args(["schema", "validate"]).arg(&document).output().expect("run md");
    let stdout = biscuit_test_harness::strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains(pointer.trim_start_matches('/')), "{stdout}");
    assert!(stdout.contains(&format!("(at line {line}, column {column})")), "{stdout}");
}

/// `node` in `a.yaml` and `leaf` in `b.yaml` reference each other; the document
/// walks root → next → back[0] three times, seven levels deep.
fn two_file_document(name: &str) -> String {
    format!(
        "---\n$schema:\n  root: node(required)@./a.yaml\nroot:\n  name: l1\n  next:\n    back:\n      - name: l2\n        \
         next:\n          back:\n            - name: l3\n              next:\n                back:\n                  - name: {name}\n                    \
         next:\n                      back: []\n---\nbody\n"
    )
}

fn write_two_file_schema(fixture: &CliProcessFixture) {
    fixture.write_file(
        "cwd/a.yaml",
        "kind: schema\ntypes:\n  node: \"{ name: string(required), next: leaf(required)@./b.yaml }\"\n",
    );
    fixture.write_file("cwd/b.yaml", "kind: schema\ntypes:\n  leaf: \"{ back: node[](required)@./a.yaml }\"\n");
}

#[test]
fn a_two_file_recursive_document_seven_levels_deep_validates() {
    let fixture = CliProcessFixture::named("schema_recursive_two_file_good");
    write_two_file_schema(&fixture);
    let document = fixture.write_file("cwd/good.md", &two_file_document("l4"));

    let (code, problems) = validate_json(&fixture, &document, &[]);

    assert_eq!(code, Some(0), "{problems:#?}");
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_two_file_recursive_failure_seven_levels_deep_reports_its_path_and_position() {
    let fixture = CliProcessFixture::named("schema_recursive_two_file_bad");
    write_two_file_schema(&fixture);
    let text = two_file_document("[42]");
    let document = fixture.write_file("cwd/bad.md", &text);
    let (line, column) = position_of(&text, "[42]");

    let (code, problems) = validate_json(&fixture, &document, &[]);

    assert_eq!(code, Some(1), "{problems:#?}");
    let reported = positioned(&problems);
    assert_eq!(
        reported,
        [(
            "/root/next/back/0/next/back/0/next/back/0/name".to_string(),
            line,
            column,
            "[42] is not of type \"string\"".to_string()
        )]
    );
    assert_eq!(reported, library_positioned(fixture.cwd(), &text), "md and the editor's validator agree");
}

// ── Presence: a finite leaf and optional references ───────────────────────

#[test]
fn required_children_accept_an_empty_leaf_and_reject_null_or_missing_at_depth_four() {
    let fixture = CliProcessFixture::named("schema_recursive_required_children");
    // `d` and `e` are both at depth four: `d` supplies null, `e` omits
    // `children`; `c`'s sibling `leaf` closes with the valid `[]`.
    let text = format!(
        "---\n{TREE_SCHEMA}tree:\n  value: a\n  children:\n    - value: b\n      children:\n        - value: c\n          \
         children:\n            - value: d\n              children: null\n            - value: e\n        - value: leaf\n          \
         children: []\n---\n"
    );
    let document = fixture.write_file("cwd/doc.md", &text);
    let (null_line, null_column) = position_of(&text, "null");
    let (missing_line, missing_column) = position_of(&text, "value: e");

    let (code, problems) = validate_json(&fixture, &document, &[]);

    assert_eq!(code, Some(1), "{problems:#?}");
    let reported = positioned(&problems);
    assert_eq!(
        reported,
        [
            (
                "/tree/children/0/children/0/children/0/children".to_string(),
                null_line,
                null_column,
                "null is not of type \"array\"".to_string()
            ),
            (
                "/tree/children/0/children/0/children/1".to_string(),
                missing_line,
                missing_column,
                "\"children\" is a required property".to_string()
            ),
        ]
    );
    assert_eq!(reported, library_positioned(fixture.cwd(), &text), "md and the editor's validator agree");
}

#[test]
fn an_optional_recursive_reference_accepts_missing_and_null_but_not_a_null_list_item() {
    let fixture = CliProcessFixture::named("schema_recursive_optional_reference");
    let schema = "$schema:\n  node: \"{ value: string(required), next: node, kids: node[] }\"\n  head: node\n  absent: node\n  nil: node\n";
    let accepted = fixture.write_file(
        "cwd/accepted.md",
        &format!("---\n{schema}head:\n  value: a\n  next: null\n  kids:\n    - value: b\n      next:\n        value: c\nnil: null\n---\n"),
    );
    let rejected_text = format!("---\n{schema}head:\n  value: a\n  kids:\n    - value: b\n    - null\n---\n");
    let rejected = fixture.write_file("cwd/rejected.md", &rejected_text);

    let (code, problems) = validate_json(&fixture, &accepted, &[]);
    assert_eq!(code, Some(0), "missing `absent`, null `nil` and null `next` are accepted: {problems:#?}");

    let (code, problems) = validate_json(&fixture, &rejected, &[]);
    assert_eq!(code, Some(1), "{problems:#?}");
    let (line, column) = position_of(&rejected_text, "- null");
    assert_eq!(
        positioned(&problems),
        [("/head/kids/1".to_string(), line, column + 2, "null is not of type \"object\"".to_string())]
    );
}

// ── Passive validation ────────────────────────────────────────────────────

#[test]
fn validation_runs_no_expression_inside_a_recursive_item_and_leaves_the_source_unchanged() {
    let fixture = CliProcessFixture::named("schema_recursive_passive");
    let top = fixture.workspace_path().join("top-ran");
    let nested = fixture.workspace_path().join("nested-ran");
    // `git init` creates a directory on every OS without a shell builtin or
    // an interpreter (both of which policy or Windows would refuse), and
    // single-quoted YAML keeps a Windows path's backslashes literal.
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix git\n");
    let text = format!(
        "---\n{TREE_SCHEMA}stamp: '$(git init -q {})'\ntree:\n  value: '$(git init -q {})'\n  children:\n    - value: \"{{{{ upper('x') }}}}\"\n      \
         children: []\n---\nbody\n",
        top.display(),
        nested.display()
    );
    let document = fixture.write_file("cwd/doc.md", &text);

    // Both spawns expose the host `PATH` so `git` resolves on native
    // Windows, whose minimal system set carries none.
    let output = fixture
        .command_builder()
        .host_path()
        .build()
        .args(["schema", "validate", "--format", "json"])
        .arg(&document)
        .output()
        .expect("run md schema validate");

    assert_eq!(output.status.code(), Some(0), "stdout: {}", String::from_utf8_lossy(&output.stdout));
    assert!(!top.exists() && !nested.exists(), "validation ran a command");
    assert_eq!(std::fs::read_to_string(&document).expect("read source"), text, "validation changed the source");

    // The same document composes its approved top-level command, so the
    // fixture's commands are live and only validation left them alone.
    let output = fixture.command_builder().host_path().build().arg("compose").arg(&document).output().expect("run md compose");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(top.exists(), "composition runs the approved command");
    assert_eq!(std::fs::read_to_string(&document).expect("read source"), text, "composition changed the source");
}

#[test]
fn a_recursive_assignment_keeps_its_text_when_the_definition_has_a_string_arm() {
    let fixture = CliProcessFixture::named("schema_recursive_assignment");
    // `node` is cyclic, so `label` is a reference whose body decides how
    // `label=1.20` is read: as the text `1.20` (four characters), never the
    // number 1.2, whose text would fail `min(4)`.
    let document = fixture.write_file(
        "cwd/doc.md",
        "---\n$schema:\n  node:\n    - \"string(min(4))\"\n    - \"{ kids: node[] }\"\n  label: node(required)\n---\nbody\n",
    );

    let (code, problems) = validate_json(&fixture, &document, &["label=1.20"]);
    assert_eq!(code, Some(0), "{problems:#?}");

    // The arm's constraint still applies to the text it now receives.
    let (code, problems) = validate_json(&fixture, &document, &["label=1.2"]);
    assert_eq!(code, Some(1), "{problems:#?}");
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0]["path"], "/label", "{problems:#?}");
}

// ── Schema-load errors ─────────────────────────────────────────────────────

#[test]
fn a_function_type_reachable_only_through_a_recursive_definition_is_a_schema_load_error() {
    let fixture = CliProcessFixture::named("schema_recursive_function_type");
    fixture.write_file(
        "cwd/types.yaml",
        "kind: schema\ntypes:\n  node: \"{ f: function(parameters(void); returns(string)), kids: node[] }\"\n",
    );
    let document = fixture.write_file("cwd/doc.md", "---\n$schema:\n  root: node@./types.yaml\nroot:\n  kids: []\n---\nbody\n");

    let output = fixture.command().args(["schema", "validate"]).arg(&document).output().expect("run md");
    let stdout = common::unwrapped(&String::from_utf8_lossy(&output.stdout));
    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(2), "a schema definition error: {stdout}{stderr}");
    let report = format!("{stdout}{stderr}");
    assert!(report.contains("root.f"), "{report}");
    assert!(report.contains("cannot type a value in a document schema"), "{report}");

    let output = fixture.command_builder().host_path().build().arg("compose").arg(&document).output().expect("run md compose");
    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty(), "no document is composed");
    assert!(stderr.contains("root.f") && stderr.contains("cannot type a value in a document schema"), "{stderr}");
}

// ── Composition ───────────────────────────────────────────────────────────

/// Nine levels: deeper than the former four expansions of one target.
const DEEP: usize = 9;

/// The body compares the deepest `value` with the text `'42'`, so it reads
/// `true` only when the number was converted.
fn deep_body() -> String {
    format!("{{{{ tree{}.value == '42' }}}}\n", ".children[0]".repeat(DEEP - 1))
}

#[test]
fn composition_converts_a_value_below_the_former_expansion_cutoff() {
    let fixture = CliProcessFixture::named("schema_recursive_deep_conversion");
    let document = fixture
        .write_file("cwd/doc.md", &format!("---\n{TREE_SCHEMA}{}---\n{}", nested_tree("tree", DEEP, "42"), deep_body()));

    let output = fixture.command().args(["compose", "--frontmatter"]).arg(&document).output().expect("run md");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let deepest_value = format!("{}- value: '42'\n", "  ".repeat(DEEP - 1));
    assert!(stdout.contains(&deepest_value), "the deep number is written as text:\n{stdout}");
    assert!(stdout.ends_with("---\ntrue\n"), "the body sees text:\n{stdout}");
}

#[test]
fn a_failed_deep_conversion_fails_composition_and_leaves_the_source_unchanged() {
    let fixture = CliProcessFixture::named("schema_recursive_failed_conversion");
    // A convertible deep `42` beside an unconvertible mapping four levels down.
    let tree = nested_tree("tree", DEEP, "42").replace("value: n3", "value: {not: text}");
    let text = format!("---\n{TREE_SCHEMA}{tree}---\nbody\n");
    let document = fixture.write_file("cwd/doc.md", &text);
    let (line, column) = position_of(&text, "{not: text}");

    let output = fixture.command_builder().host_path().build().arg("compose").arg(&document).output().expect("run md compose");

    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty(), "no document is composed: {}", String::from_utf8_lossy(&output.stdout));
    assert!(stderr.contains("tree/children/0/children/0/children/0/value"), "{stderr}");
    assert!(stderr.contains(&format!("at {line}:{column}")), "{stderr}");
    assert_eq!(std::fs::read_to_string(&document).expect("read source"), text, "the source is unchanged");
}

#[test]
fn repeated_composition_of_a_recursive_document_is_idempotent() {
    let fixture = CliProcessFixture::named("schema_recursive_idempotent");
    let text = format!("---\n{TREE_SCHEMA}{}---\nbody\n", nested_tree("tree", DEEP, "42"));
    let document = fixture.write_file("cwd/doc.md", &text);
    let compose = |path: &Path| {
        let output = fixture.command().args(["compose", "--frontmatter"]).arg(path).output().expect("run md compose");
        assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).expect("UTF-8 document")
    };

    let first = compose(&document);
    let normalized = fixture.write_file("cwd/normalized.md", &first);
    let second = compose(&normalized);

    assert_ne!(first, text, "the first composition normalizes the deep number");
    assert_eq!(second, first, "composing the normalized document changes nothing");
    assert_eq!(std::fs::read_to_string(&document).expect("read source"), text, "the source is unchanged");
}

#[test]
fn the_call_check_names_a_recursive_argument_by_its_type_name() {
    let fixture = CliProcessFixture::named("schema_recursive_call_check");
    let document = fixture.write_file(
        "cwd/doc.md",
        &format!("---\n{TREE_SCHEMA}tree:\n  value: a\n  children: []\n---\n{{{{ min(tree, 5) }}}}\n"),
    );

    let output = fixture.command_builder().host_path().build().arg("compose").arg(&document).output().expect("run md compose");

    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("min: argument 1 (a) is item, expected number"), "{stderr}");
}

// ── md schema about ───────────────────────────────────────────────────────

#[test]
fn schema_about_terminates_beside_recursive_schemas() {
    let fixture = CliProcessFixture::named("schema_recursive_about");
    // Recursive schemas in the launch directory and the home schema root:
    // `md schema about` is documentation-only, so neither may change it.
    write_two_file_schema(&fixture);
    fixture.write_file("home/schemas/tree.yaml", "kind: schema\ntypes:\n  item: \"{ children: item[] }\"\n");
    let empty = CliProcessFixture::named("schema_recursive_about_empty");
    let about = |process: &CliProcessFixture| {
        let output = process.command().args(["--verbose", "schema", "about"]).output().expect("run md schema about");
        assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).expect("UTF-8 report")
    };

    assert_eq!(about(&fixture), about(&empty));
}
