//! An editor's observed values judged against the document's effective
//! schema (`EffectiveSchema::validate_observed_frontmatter`), with supplied
//! candidate values and no evaluation.
//!
//! Each test builds its document's effective schema in its own temporary
//! directory, replaces the authored whole-value templates with concrete
//! values, and reads the per-property verdicts: the same conversion,
//! defaults, and constraints composition applies, a deferral where the
//! editor cannot judge (an undeclared key, a file check, a root union with
//! another template still authored), and never a change to the inputs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, DeferReason, EffectiveSchema, ObservedVerdict};
use serde_json::{Value, json};
use simplified_schema::ValidationProblem;
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create parent");
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// A context anchored in `dir`, with its own empty home.
fn context(dir: &Path) -> biscuit_file::FileResolutionContext {
    let home = dir.join(".home");
    std::fs::create_dir_all(&home).expect("home");
    biscuit_file::FileResolutionContext::from_snapshot(dir, Some(home), std::collections::HashMap::new())
}

/// The document at `dir/doc.md` holding `text`, its effective schema, and
/// its authored frontmatter as JSON.
fn prepared(dir: &Path, text: &str, triggers: bool) -> (EffectiveSchema, Value) {
    let path = write(dir, "doc.md", text);
    let source = Markdown::try_from(path.as_path()).expect("document loads");
    let mut schemas = DarkmatterSchemas::new(context(dir));
    if triggers {
        schemas = schemas.with_trigger_discovery().expect("trigger discovery");
    }
    let effective = schemas.effective_for(&source).expect("schema resolves").expect("an effective schema");
    let seed = darkmatter::markdown::schemas::frontmatter_as_json(&source);
    (effective, seed)
}

fn replaced(entries: &[(&str, Value)]) -> BTreeMap<String, Value> {
    entries.iter().map(|(key, value)| ((*key).to_string(), value.clone())).collect()
}

/// The judged problems of `key`; panics on a deferral.
fn judged<'a>(verdicts: &'a darkmatter::markdown::schemas::ObservedValidation, key: &str) -> &'a [ValidationProblem] {
    match verdicts.properties.get(key) {
        Some(ObservedVerdict::Judged(problems)) => problems,
        other => panic!("{key} was not judged: {other:?}"),
    }
}

fn deferred(verdicts: &darkmatter::markdown::schemas::ObservedValidation, key: &str) -> DeferReason {
    match verdicts.properties.get(key) {
        Some(ObservedVerdict::Deferred(reason)) => *reason,
        other => panic!("{key} was not deferred: {other:?}"),
    }
}

/// The worked example's people, with Grace's age `age`.
fn people(age: Value) -> Value {
    json!([{ "name": "Ada", "age": 36 }, { "name": "Grace", "age": age }])
}

const ROW_DOC: &str = "---\n$schema:\n    direct: \"{ name: string(required), age: number }[]\"\ndirect: \"{{ load_data('data.yaml') }}\"\n---\nBody\n";

#[test]
fn the_worked_example_passes_and_a_bad_age_fails_at_its_element() {
    let dir = TempDir::new().unwrap();
    let (schema, seed) = prepared(dir.path(), ROW_DOC, false);
    let valid = schema.validate_observed_frontmatter(&seed, &replaced(&[("direct", people(json!(85)))]));
    assert!(judged(&valid, "direct").is_empty(), "{valid:?}");
    let invalid = schema.validate_observed_frontmatter(&seed, &replaced(&[("direct", people(json!("old")))]));
    let problems = judged(&invalid, "direct");
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0].path, "/direct/1/age");
}

#[test]
fn the_inputs_are_never_changed() {
    let dir = TempDir::new().unwrap();
    let doc = "---\n$schema:\n    count: number\n    owner: \"{ name: string, role: string(default(viewer)) }\"\ncount: \"{{ x }}\"\nowner: \"{{ y }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let replacements = replaced(&[("count", json!("1000")), ("owner", json!({ "name": "Ada" }))]);
    let (seed_before, replacements_before) = (seed.clone(), replacements.clone());
    let verdicts = schema.validate_observed_frontmatter(&seed, &replacements);
    assert!(judged(&verdicts, "count").is_empty() && judged(&verdicts, "owner").is_empty(), "{verdicts:?}");
    assert_eq!(seed, seed_before);
    assert_eq!(replacements, replacements_before);
    // The seed still holds the authored templates, not the observed values.
    assert_eq!(seed["count"], json!("{{ x }}"));
}

#[test]
fn enum_membership_is_judged_for_each_element() {
    let dir = TempDir::new().unwrap();
    let doc = "---\n$schema:\n    colors: \"enum(red, blue)[]\"\ncolors: \"{{ load_data('c.yaml') }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let pass = schema.validate_observed_frontmatter(&seed, &replaced(&[("colors", json!(["red", "blue"]))]));
    assert!(judged(&pass, "colors").is_empty(), "{pass:?}");
    let fail = schema.validate_observed_frontmatter(&seed, &replaced(&[("colors", json!(["red", "green"]))]));
    let problems = judged(&fail, "colors");
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0].path, "/colors/1");
}

/// A CSV cell is text; a `number` destination converts it, as composition does.
#[test]
fn a_numeric_string_converts_and_a_word_does_not() {
    let dir = TempDir::new().unwrap();
    let doc = "---\n$schema:\n    rows: \"{ total: number }[]\"\nrows: \"{{ load_data('t.csv') }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let pass = schema.validate_observed_frontmatter(&seed, &replaced(&[("rows", json!([{ "total": "1000" }]))]));
    assert!(judged(&pass, "rows").is_empty(), "{pass:?}");
    let fail = schema.validate_observed_frontmatter(&seed, &replaced(&[("rows", json!([{ "total": "lots" }]))]));
    assert_eq!(judged(&fail, "rows").len(), 1, "{fail:?}");
}

#[test]
fn a_default_supplies_a_missing_required_field() {
    let dir = TempDir::new().unwrap();
    let doc = "---\n$schema:\n    rows: \"{ name: string(required), role: string(required; default(viewer)) }[]\"\nrows: \"{{ load_data('r.yaml') }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let pass = schema.validate_observed_frontmatter(&seed, &replaced(&[("rows", json!([{ "name": "Ada" }]))]));
    assert!(judged(&pass, "rows").is_empty(), "{pass:?}");
    // Control: without the default the same row fails.
    let doc = "---\n$schema:\n    rows: \"{ name: string(required), role: string(required) }[]\"\nrows: \"{{ load_data('r.yaml') }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let fail = schema.validate_observed_frontmatter(&seed, &replaced(&[("rows", json!([{ "name": "Ada" }]))]));
    assert_eq!(judged(&fail, "rows").len(), 1, "{fail:?}");
}

#[test]
fn a_closed_object_rejects_an_unexpected_member() {
    let dir = TempDir::new().unwrap();
    let doc = "---\n$schema:\n    owner: \"{ name: string }\"\nowner: \"{{ load_data('o.yaml') }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let pass = schema.validate_observed_frontmatter(&seed, &replaced(&[("owner", json!({ "name": "Ada" }))]));
    assert!(judged(&pass, "owner").is_empty(), "{pass:?}");
    let fail = schema.validate_observed_frontmatter(&seed, &replaced(&[("owner", json!({ "name": "Ada", "color": "blue" }))]));
    let problems = judged(&fail, "owner");
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0].offending_property.as_deref(), Some("color"), "{problems:#?}");
}

#[test]
fn an_imported_type_is_applied() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  Person:\n    name: string(required)\n    age: number\n");
    let doc = "---\n$schema:\n    owner: Person@./types.yaml\nowner: \"{{ load_data('owner.yaml') }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let pass = schema.validate_observed_frontmatter(&seed, &replaced(&[("owner", json!({ "name": "Ada", "age": 36 }))]));
    assert!(judged(&pass, "owner").is_empty(), "{pass:?}");
    let fail = schema.validate_observed_frontmatter(&seed, &replaced(&[("owner", json!({ "name": "Ada", "age": "old" }))]));
    assert_eq!(judged(&fail, "owner")[0].path, "/owner/age", "{fail:?}");
}

#[test]
fn a_trigger_declaration_is_layered_in() {
    let dir = TempDir::new().unwrap();
    // A repository marker without spawning `git`.
    write(dir.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(dir.path(), ".git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
    std::fs::create_dir_all(dir.path().join(".git/refs")).unwrap();
    write(
        dir.path(),
        "schemas/t.yaml",
        "kind: schema-trigger\nmatch:\n  stage: literal(draft; required)\n$schema:\n  people: \"{ name: string(required), age: number }[]\"\n",
    );
    let doc = "---\nstage: draft\npeople: \"{{ load_data('data.yaml') }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, true);
    let fail = schema.validate_observed_frontmatter(&seed, &replaced(&[("people", people(json!("old")))]));
    assert_eq!(judged(&fail, "people")[0].path, "/people/1/age", "{fail:?}");
    let pass = schema.validate_observed_frontmatter(&seed, &replaced(&[("people", people(json!(85)))]));
    assert!(judged(&pass, "people").is_empty(), "{pass:?}");
}

const UNION_DOC: &str = "---\n$schema:\n    - kind: literal(a; required)\n      value: number\n    - kind: literal(b; required)\n      value: string\nkind: a\nvalue: \"{{ load_data('v.yaml') }}\"\n---\n";

#[test]
fn a_root_union_judges_the_fully_known_candidate() {
    let dir = TempDir::new().unwrap();
    let (schema, seed) = prepared(dir.path(), UNION_DOC, false);
    let pass = schema.validate_observed_frontmatter(&seed, &replaced(&[("value", json!(5))]));
    assert!(judged(&pass, "value").is_empty(), "{pass:?}");
    // `kind: a` selects the number arm; no arm takes a list.
    let fail = schema.validate_observed_frontmatter(&seed, &replaced(&[("value", json!(["x"]))]));
    assert!(!judged(&fail, "value").is_empty(), "{fail:?}");
}

#[test]
fn a_root_union_with_another_template_still_authored_is_deferred() {
    let dir = TempDir::new().unwrap();
    let doc = UNION_DOC.replace("kind: a\n", "kind: \"{{ pick() }}\"\n");
    let (schema, seed) = prepared(dir.path(), &doc, false);
    let verdicts = schema.validate_observed_frontmatter(&seed, &replaced(&[("value", json!(["x"]))]));
    assert_eq!(deferred(&verdicts, "value"), DeferReason::RootUnionUnresolved);
    // Once both are observed, the union is judged.
    let verdicts = schema.validate_observed_frontmatter(&seed, &replaced(&[("kind", json!("a")), ("value", json!(5))]));
    assert!(judged(&verdicts, "value").is_empty() && judged(&verdicts, "kind").is_empty(), "{verdicts:?}");
}

/// An eager file check is never run: the property is deferred and the
/// missing file produces no problem, while a sibling without a file check
/// is still judged.
#[test]
fn a_file_dependent_property_is_deferred_without_a_probe() {
    let dir = TempDir::new().unwrap();
    let doc = "---\n$schema:\n    spec: file(eager)\n    count: number\nspec: \"{{ x }}\"\ncount: \"{{ y }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let verdicts =
        schema.validate_observed_frontmatter(&seed, &replaced(&[("spec", json!("./missing.md")), ("count", json!("many"))]));
    assert_eq!(deferred(&verdicts, "spec"), DeferReason::FileDependent);
    assert_eq!(judged(&verdicts, "count").len(), 1, "{verdicts:?}");
    // Control: composition's validation of the same value probes and fails.
    let mut instance = seed.clone();
    instance["spec"] = json!("./missing.md");
    instance["count"] = json!(1);
    let report = schema.validate(&instance);
    assert!(report.problems.iter().any(|problem| problem.path == "/spec"), "{report:?}");
}

/// A returned string that looks like a template is data.
#[test]
fn a_template_like_returned_string_is_judged_as_text() {
    let dir = TempDir::new().unwrap();
    let doc = "---\n$schema:\n    count: number\n    label: \"enum(a, b)\"\ncount: \"{{ x }}\"\nlabel: \"{{ y }}\"\n---\n";
    let (schema, seed) = prepared(dir.path(), doc, false);
    let verdicts = schema.validate_observed_frontmatter(
        &seed,
        &replaced(&[("count", json!("{{ fm.count }}")), ("label", json!("{{ 'a' }}"))]),
    );
    assert_eq!(judged(&verdicts, "count").len(), 1, "{verdicts:?}");
    assert_eq!(judged(&verdicts, "label").len(), 1, "{verdicts:?}");
}

#[test]
fn an_undeclared_property_is_deferred() {
    let dir = TempDir::new().unwrap();
    let (schema, seed) = prepared(dir.path(), ROW_DOC, false);
    let verdicts = schema.validate_observed_frontmatter(&seed, &replaced(&[("other", json!([1])), ("direct", people(json!(1)))]));
    assert_eq!(deferred(&verdicts, "other"), DeferReason::Undeclared);
    assert!(judged(&verdicts, "direct").is_empty(), "{verdicts:?}");
}
