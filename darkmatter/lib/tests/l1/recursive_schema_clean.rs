//! Both clean walks through recursive references.
//!
//! A recursive named type lowers to one body under `$defs` with a `$ref` at
//! each use, so every node below the first level is reached only through a
//! reference. Schema-proven quoting and did-you-mean key suggestions must
//! follow those references to any depth, through a bare reference, a
//! nullable reference with a sibling constraint, and a list item reference,
//! while a genuine union reached through a reference is still never repaired.

use std::path::Path;

use biscuit_file::{FileResolutionContext, YamlCertainty, YamlDiagnosticCode, YamlRepair, apply_edit_set};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, SchemaCleanAnalysis, analyze_frontmatter};
use serde_json::{Value, json};
use tempfile::TempDir;

/// `tree` is a bare reference, `maybe` a nullable reference with a sibling
/// constraint (`anyOf: [null, {$ref, minProperties}]`), and every
/// `children` item a reference; `tag` is a genuine literal-tagged union of
/// two closed objects that both accept a string `name`, so the validator
/// narrows a tagged value's problems to one arm.
const SCHEMA: &str = "$schema:\n  \
    first: \"{ kind: literal(first; required), name: string(required) }\"\n  \
    second: \"{ kind: literal(second; required), name: string(required), size: number }\"\n  \
    tag: \"first | second\"\n  \
    item: \"{ value: string(required), children: item[](required), tag: tag }\"\n  \
    tree: item(required)\n  \
    maybe: item(min-keys(1))\n";

const DEPTH: usize = 6;

/// A `{ value, children }` tree `depth` levels deep whose bottom node is
/// `leaf` merged over `{ value: "leaf", children: [] }`.
fn tree(depth: usize, leaf: Value) -> Value {
    let mut bottom = json!({"value": "leaf", "children": []});
    for (key, value) in leaf.as_object().expect("leaf is a mapping") {
        bottom[key] = value.clone();
    }
    (1..depth).fold(bottom, |child, level| json!({"value": format!("n{level}"), "children": [child]}))
}

/// The frontmatter body: the schema, then `instance` as block YAML.
fn body(instance: &Value) -> String {
    format!("{SCHEMA}{}", serde_yaml_ng::to_string(instance).expect("instance serializes"))
}

fn effective(dir: &Path, body: &str) -> EffectiveSchema {
    let path = dir.join("doc.md");
    std::fs::write(&path, format!("---\n{body}---\nBody\n")).expect("write document");
    let markdown = Markdown::try_from(path.as_path()).expect("document loads");
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("the document declares a schema")
}

fn analyze(body: &str) -> (TempDir, EffectiveSchema, SchemaCleanAnalysis) {
    let dir = TempDir::new().expect("tempdir");
    let effective = effective(dir.path(), body);
    let analysis = analyze_frontmatter(&effective, body);
    (dir, effective, analysis)
}

fn repairs(analysis: &SchemaCleanAnalysis, auto_only: bool) -> Vec<YamlRepair> {
    analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| !auto_only || diagnostic.classification.is_auto_apply_eligible())
        .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
        .collect()
}

fn instance(body: &str) -> Value {
    let mut value: Value = serde_yaml_ng::from_str(body).expect("frontmatter parses");
    value.as_object_mut().expect("mapping").remove("$schema");
    value
}

fn key_suggestions(analysis: &SchemaCleanAnalysis) -> Vec<String> {
    analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::SchemaKeyCorrection)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[test]
fn recursive_schema_clean_quotes_a_number_deep_below_each_reference_form() {
    for property in ["tree", "maybe"] {
        let mut document = json!({"tree": tree(1, json!({}))});
        document[property] = tree(DEPTH, json!({"value": 42}));
        let body = body(&document);
        let (_dir, effective, analysis) = analyze(&body);

        assert_eq!(analysis.diagnostics().len(), 1, "{property}: {:#?}", analysis.diagnostics());
        let diagnostic = &analysis.diagnostics()[0];
        assert_eq!(diagnostic.code, YamlDiagnosticCode::SchemaTypeMismatch, "{property}");
        assert_eq!(diagnostic.classification, YamlCertainty::Deterministic, "{property}: {diagnostic:?}");

        let repaired = apply_edit_set(&body, &repairs(&analysis, true)).source;
        let mut expected = document.clone();
        expected[property] = tree(DEPTH, json!({"value": "42"}));
        assert_eq!(instance(&repaired), expected, "{property}: only the deep scalar is quoted");
        assert!(effective.validate_raw(&instance(&repaired)).valid, "{property}: {repaired}");
    }
}

#[test]
fn recursive_schema_clean_suggests_a_declared_key_at_every_depth() {
    for property in ["tree", "maybe"] {
        let mut shallow = tree(2, json!({"childrne": []}));
        shallow["children"][0]["children"] = json!([tree(DEPTH - 2, json!({"vaule": "x"}))]);
        let mut document = json!({"tree": tree(1, json!({}))});
        document[property] = shallow;
        let (_dir, _effective, analysis) = analyze(&body(&document));

        let suggestions = key_suggestions(&analysis);
        assert!(
            suggestions.iter().any(|message| message.contains("`childrne`") && message.contains("did you mean `children`?")),
            "{property}: the shallow node is suggested: {suggestions:#?}"
        );
        assert!(
            suggestions.iter().any(|message| message.contains("`vaule`") && message.contains("did you mean `value`?")),
            "{property}: the deeper node of the same definition is suggested too: {suggestions:#?}"
        );
    }
}

/// Asserts `document` fails the raw schema below the union, yet cleanup
/// neither edits it, whichever repairs a caller applies, nor suggests any
/// one arm's key; returns the analysis.
fn assert_union_left_unrepaired(document: &Value) -> SchemaCleanAnalysis {
    let body = body(document);
    let (_dir, effective, analysis) = analyze(&body);

    let raw = effective.validate_raw(&instance(&body));
    assert!(
        raw.problems.iter().any(|problem| problem.path.contains("/tag")),
        "the value fails inside the union: {:#?}",
        raw.problems
    );
    assert_eq!(apply_edit_set(&body, &repairs(&analysis, false)).source, body, "no union repair is offered");
    assert!(
        analysis.diagnostics().iter().all(|diagnostic| !diagnostic.message.contains("did you mean")),
        "no arm's keys are suggested: {:#?}",
        analysis.diagnostics()
    );
    analysis
}

#[test]
fn recursive_schema_clean_never_quotes_inside_a_union_reached_through_a_reference() {
    // The tagged arm's problem is narrowed to `tag/name`, and quoting `42`
    // would satisfy that arm; closest-arm reporting still authorizes no
    // repair. Compose coercion accepts the number, so nothing is reported.
    assert_union_left_unrepaired(&json!({"tree": tree(DEPTH, json!({"tag": {"kind": "first", "name": 42}}))}));
}

#[test]
fn recursive_schema_clean_never_suggests_an_arm_key_through_a_reference() {
    let analysis =
        assert_union_left_unrepaired(&json!({"tree": tree(DEPTH, json!({"tag": {"kind": "first", "name": "x", "nmae": "x"}}))}));

    assert!(
        analysis.diagnostics().iter().any(|diagnostic| diagnostic.message.contains("`nmae`")),
        "the undeclared key is still reported: {:#?}",
        analysis.diagnostics()
    );
}
