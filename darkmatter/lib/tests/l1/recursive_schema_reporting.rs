//! Recursive references in validation reports and literal-discriminant selection.
//!
//! A recursive union lowers to one `$defs` body that every level of a tree
//! reaches through `$ref`. These tests validate documents whose bad value sits
//! six or more levels down and check that the report still names one problem,
//! at the right JSON path and source position, explained by the arm the
//! instance's literal tag selects.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, ValidationProblem, ValidationProblemKind, ValidationReport,
};
use serde_json::Value;
use tempfile::TempDir;

/// `node` is a discriminated union written inline, so both arms sit in the
/// `$defs/node` body and only `children` refers back to it. The arms disagree
/// on `value`'s type and description, so an undiscriminated report or
/// explanation differs from the selected arm's.
const INLINE_ARM_TYPES: &str = "kind: schema\ntypes:\n  node:\n    - \"{ kind: literal(leaf; required), value: string(required) -> the leaf text }\"\n    - \"{ kind: literal(branch; required), value: number(required) -> the branch weight, children: node[](required) }\"\n";

/// `node`'s arms are named recursive types, so each arm lowers to a bare
/// `{"$ref": "#/$defs/<arm>"}` and the `kind` literals live only in the
/// referenced bodies.
const REFERENCE_ARM_TYPES: &str = "kind: schema\ntypes:\n  node:\n    - branch\n    - leaf\n  branch: \"{ kind: literal(branch; required), value: number(required) -> the branch weight, children: node[](required) }\"\n  leaf: \"{ kind: literal(leaf; required), value: string(required) -> the leaf text, note: node }\"\n";

fn validate_document(dir: &Path, document: &str) -> ValidationReport {
    let md: Markdown = document.to_string().into();
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .validate(&md)
        .expect("the document's schema resolves")
}

/// A block-YAML `root:` chain of `depth` nodes: `depth - 1` branches, each
/// with one child, ending in a leaf whose `value` is written as `leaf_value`.
/// Fields of level `k` are indented `4k + 2` spaces, each child list item
/// opening with `- ` at `4k`.
fn branch_chain(depth: usize, leaf_value: &str) -> String {
    let mut out = String::from("root:\n");
    for level in 0..depth {
        let fields = " ".repeat(4 * level + 2);
        let first = if level == 0 { fields.clone() } else { format!("{}- ", " ".repeat(4 * level)) };
        if level + 1 == depth {
            out.push_str(&format!("{first}kind: leaf\n{fields}value: {leaf_value}\n"));
        } else {
            out.push_str(&format!("{first}kind: branch\n{fields}value: {level}\n{fields}children:\n"));
        }
    }
    out
}

fn leaf_value_path(depth: usize) -> String {
    format!("/root{}/value", "/children/0".repeat(depth - 1))
}

/// The 1-based line and column of the first occurrence of `needle` in
/// `document`.
fn position_of(document: &str, needle: &str) -> (u32, u32) {
    document
        .lines()
        .enumerate()
        .find_map(|(index, line)| line.find(needle).map(|column| (index as u32 + 1, column as u32 + 1)))
        .unwrap_or_else(|| panic!("{needle:?} is in the document"))
}

fn only_problem(report: &ValidationReport) -> &ValidationProblem {
    assert!(!report.valid);
    assert_eq!(report.problems.len(), 1, "exactly one problem: {:#?}", report.problems);
    &report.problems[0]
}

#[test]
fn a_depth_seven_failure_is_reported_once_at_its_path_and_position_with_the_selected_arm_description() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("types.yaml"), INLINE_ARM_TYPES).expect("write types");
    let document = format!(
        "---\n$schema:\n  root: \"node(required)@./types.yaml\"\n{}---\nbody\n",
        branch_chain(7, "{not: text}")
    );

    let report = validate_document(dir.path(), &document);

    // Each level is a separate reach of `$defs/node`; without the discriminant
    // narrowing every level, the branch arm's `kind` and `value` mismatches at
    // the leaf would be reported too, and the leaf's description would be
    // ambiguous between the two arms.
    let problem = only_problem(&report);
    assert_eq!(problem.path, leaf_value_path(7));
    assert!(matches!(problem.kind, ValidationProblemKind::Type), "{problem:#?}");
    assert!(problem.message.contains("\"string\""), "{problem:#?}");
    assert_eq!((problem.line, problem.column), {
        let (line, column) = position_of(&document, "{not: text}");
        (Some(line), Some(column))
    });
    assert_eq!(problem.description.as_deref(), Some("the leaf text"));
}

#[test]
fn a_union_of_reference_arms_is_narrowed_by_the_literals_in_their_definitions() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("types.yaml"), REFERENCE_ARM_TYPES).expect("write types");
    let schema = "$schema:\n  root: \"node(required)@./types.yaml\"\n";
    let effective = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .effective_for(&format!("---\n{schema}---\n").into())
        .expect("schema resolves")
        .expect("an effective schema");
    let arms = effective.json_schema["$defs"]["node"]["anyOf"].as_array().expect("node is a union");
    assert!(
        arms.iter().all(|arm| arm.get("$ref").is_some() && arm.get("properties").is_none()),
        "both arms are bare references: {arms:#?}"
    );

    let valid_document = format!("---\n{schema}{}---\nbody\n", branch_chain(6, "deep"));
    let valid = validate_document(dir.path(), &valid_document);
    assert!(valid.valid, "a depth-6 chain ending in a leaf validates: {:#?}", valid.problems);

    let document = format!("---\n{schema}{}---\nbody\n", branch_chain(6, "{not: text}"));
    let report = validate_document(dir.path(), &document);
    let problem = only_problem(&report);
    assert_eq!(problem.path, leaf_value_path(6));
    assert!(problem.message.contains("\"string\""), "{problem:#?}");
    let (line, column) = position_of(&document, "{not: text}");
    assert_eq!((problem.line, problem.column), (Some(line), Some(column)));
    assert_eq!(problem.description.as_deref(), Some("the leaf text"));
}

#[test]
fn a_reference_arm_selected_at_the_root_of_the_tree_explains_a_missing_field_below_it() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("types.yaml"), REFERENCE_ARM_TYPES).expect("write types");
    let chain = branch_chain(6, "deep").replace("kind: leaf\n", "kind: branch\n");
    let document = format!("---\n$schema:\n  root: \"node(required)@./types.yaml\"\n{chain}---\nbody\n");

    let report = validate_document(dir.path(), &document);

    // The deepest node is tagged `branch` but has a string `value` and no
    // `children`; only the branch arm's two problems surface.
    let deepest = format!("/root{}", "/children/0".repeat(5));
    let mut found: Vec<(String, Option<String>, Option<String>)> = report
        .problems
        .iter()
        .map(|problem| (problem.path.clone(), problem.property.clone(), problem.description.clone()))
        .collect();
    found.sort();
    assert_eq!(
        found,
        [
            (deepest.clone(), Some("children".to_string()), None),
            (format!("{deepest}/value"), None, Some("the branch weight".to_string())),
        ],
        "{:#?}",
        report.problems
    );
}

#[test]
fn a_root_union_arm_explains_a_deep_failure_through_the_root_definitions() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("types.yaml"), INLINE_ARM_TYPES).expect("write types");
    let document = format!(
        "---\n$schema:\n  - mode: literal(tree)\n    root: \"node(required)@./types.yaml\"\n  - mode: literal(flat)\n    name: string(required)\nmode: tree\n{}---\nbody\n",
        branch_chain(6, "{not: text}")
    );

    let report = validate_document(dir.path(), &document);

    // The root arm is compiled on its own with the root's `$defs`; its report
    // keeps the arm index, and the description walk starts at that arm and
    // follows each `$ref` against the document root.
    assert!(!report.valid);
    let path = leaf_value_path(6);
    let problem = report
        .problems
        .iter()
        .find(|problem| problem.path == path && problem.message.contains("\"string\""))
        .unwrap_or_else(|| panic!("no string mismatch at {path}: {:#?}", report.problems));
    assert_eq!(problem.arm_index, Some(0), "{problem:#?}");
    let (line, column) = position_of(&document, "{not: text}");
    assert_eq!((problem.line, problem.column), (Some(line), Some(column)));
    assert_eq!(problem.description.as_deref(), Some("the leaf text"));
}

#[test]
fn branch_chain_writes_the_documented_tree() {
    let yaml: Value = serde_yaml_ng::from_str(&branch_chain(3, "x")).expect("chain parses");
    assert_eq!(
        yaml,
        serde_json::json!({ "root": { "kind": "branch", "value": 0, "children": [
            { "kind": "branch", "value": 1, "children": [ { "kind": "leaf", "value": "x" } ] }
        ] } })
    );
}

/// A pending top-level value defers only its own key's problems. A nested
/// missing property is attributed to the top-level key that holds it, never
/// to a pending top-level key that shares the missing property's name; the
/// root-level missing key is still deferred when it is the pending one.
#[test]
fn a_nested_missing_property_is_not_hidden_by_a_pending_key_of_the_same_name() {
    use darkmatter::markdown::schemas::{PendingPolicy, PositionMap, ValidationOptions};
    use serde_json::json;

    let dir = TempDir::new().expect("tempdir");
    let source: Markdown = "---\n$schema:\n  children: string(required)\n  item: \"{ value: string(required), children: item[](required) }\"\n  tree: item(required)\n---\nbody\n"
        .to_string()
        .into();
    let effective = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema");
    let deep = (1..6).fold(json!({"value": "leaf"}), |child, level| {
        json!({"value": format!("n{level}"), "children": [child]})
    });
    let defer = ValidationOptions { pending_policy: PendingPolicy::Defer, ..ValidationOptions::default() };

    let report = effective.validate_with_options(
        &json!({"children": "$(echo pending)", "tree": deep}),
        &PositionMap::new(),
        &defer,
    );
    let missing: Vec<(String, Option<String>)> =
        report.problems.iter().map(|problem| (problem.path.clone(), problem.property.clone())).collect();
    let deepest = format!("/tree{}", "/children/0".repeat(5));
    assert_eq!(missing, [(deepest, Some("children".to_string()))], "{:#?}", report.problems);

    // Control: the pending key's own root-level absence is still deferred.
    let report = effective.validate_with_options(
        &json!({"tree": {"value": "leaf", "children": []}, "other": "$(echo pending)"}),
        &PositionMap::new(),
        &defer,
    );
    assert_eq!(report.problems.len(), 1, "an absent required key is reported: {:#?}", report.problems);
    let report = effective.validate_with_options(
        &json!({"children": "$(echo pending)", "tree": {"value": "leaf", "children": []}}),
        &PositionMap::new(),
        &defer,
    );
    assert!(report.valid, "{:#?}", report.problems);
}
