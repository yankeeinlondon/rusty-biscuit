//! Recursive definitions across merge boundaries and validation phases,
//! through the public schema API: a root union's arms, a baseline, and
//! trigger payloads each bring their own `$defs`, which join the document's
//! root without a key collision binding a reference to another layer's body;
//! sub-schemas judged alone keep their root's definitions; and a phase
//! projection reaches the bodies below the recursion.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, DefKey, DefNamespace, EffectiveSchema, SchemaError, SchemaPhase, ValidationReport,
};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use tempfile::TempDir;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture folder");
    std::fs::write(&path, body).expect("write fixture");
    path
}

fn markdown(document: &str) -> Markdown {
    document.to_string().into()
}

fn effective(schemas: &DarkmatterSchemas, document: &str) -> EffectiveSchema {
    schemas.effective_for(&markdown(document)).expect("schema resolves").expect("an effective schema")
}

fn problem_paths(report: &ValidationReport) -> Vec<String> {
    report.problems.iter().map(|problem| problem.path.clone()).collect()
}

/// A `{ value, children }` tree `depth` levels deep, `leaf` at the bottom.
fn tree(depth: usize, leaf: Value) -> Value {
    (1..depth).fold(json!({"value": leaf, "children": []}), |child, level| {
        json!({"value": format!("n{level}"), "children": [child]})
    })
}

fn deep_value_path(root: &str, depth: usize) -> String {
    format!("/{root}{}/value", "/children/0".repeat(depth - 1))
}

const ITEM: &str = "\"{ value: string(required), children: item[](required) }\"";

// ── Root unions ───────────────────────────────────────────────────────────

#[test]
fn a_recursive_root_union_arm_validates_with_the_union_definitions() {
    let dir = TempDir::new().expect("tempdir");
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()));
    let schema = format!(
        "$schema:\n  - kind: literal(tree)\n    item: {ITEM}\n    tree: item(required)\n  - kind: literal(flat)\n    name: string(required)\n"
    );
    let document = format!("---\n{schema}---\nbody\n");
    let effective = effective(&schemas, &document);
    assert!(effective.json_schema["anyOf"][0].get("$defs").is_none(), "definitions live at the union root");

    let good = json!({"kind": "tree", "tree": tree(6, json!("deep"))});
    assert!(effective.validate(&good).valid, "{:#?}", effective.validate(&good).problems);
    let bad = json!({"kind": "tree", "tree": tree(6, json!({"not": "text"}))});
    let report = effective.validate(&bad);
    assert!(!report.valid);
    assert!(problem_paths(&report).contains(&deep_value_path("tree", 6)), "{:#?}", report.problems);
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        let report = effective.validate_for_phase(&bad, phase).expect("phase schema builds");
        assert!(problem_paths(&report).contains(&deep_value_path("tree", 6)), "{phase:?}: {:#?}", report.problems);
    }
}

#[test]
fn file_arms_with_same_named_definitions_are_rekeyed_into_the_union_root() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "a.yaml", &format!("$schema:\n  kind: literal(a)\n  item: {ITEM}\n  atree: item(required)\n"));
    write(
        dir.path(),
        "b.yaml",
        "$schema:\n  kind: literal(b)\n  item: \"{ count: number(required), children: item[](required) }\"\n  btree: item(required)\n",
    );
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()));
    let effective = effective(&schemas, "---\n$schema: [./a.yaml, ./b.yaml]\n---\nbody\n");
    let defs = effective.json_schema["$defs"].as_object().expect("union root $defs");
    assert_eq!(defs.keys().collect::<Vec<_>>(), ["item", "item-2"]);
    assert_eq!(defs["item-2"]["properties"]["count"]["type"], "number");
    assert_eq!(effective.json_schema["anyOf"][1]["properties"]["btree"]["$ref"], "#/$defs/item-2");
    assert!(effective.json_schema["anyOf"].as_array().expect("arms").iter().all(|arm| arm.get("$defs").is_none()));

    let counted = |leaf: Value| {
        (1..5).fold(json!({"count": leaf, "children": []}), |child, _| json!({"count": 1, "children": [child]}))
    };
    let good = json!({"kind": "b", "btree": counted(json!(4))});
    let bad = json!({"kind": "b", "btree": counted(json!("four"))});
    let bad_path = format!("/btree{}/count", "/children/0".repeat(4));
    assert!(effective.validate(&good).valid, "{:#?}", effective.validate(&good).problems);
    assert!(effective.validate(&json!({"kind": "a", "atree": tree(5, json!("x"))})).valid);
    assert!(problem_paths(&effective.validate(&bad)).contains(&bad_path));
    // The phase projection lowers the arms against the union's table; its keys
    // must be the ones the arms were re-keyed to.
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        assert!(effective.validate_for_phase(&good, phase).expect("phase").valid, "{phase:?}");
        let report = effective.validate_for_phase(&bad, phase).expect("phase");
        assert!(problem_paths(&report).contains(&bad_path), "{phase:?}: {:#?}", report.problems);
    }
}

#[test]
fn a_raw_arm_keeps_its_definitions_and_an_inline_arm_keeps_its_keys() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "raw.json",
        &json!({
            "type": "object",
            "properties": {"node": {"$ref": "#/$defs/item"}},
            "required": ["node"],
            "$defs": {"item": {"type": "object", "properties": {"n": {"type": "integer"}, "next": {"$ref": "#/$defs/item"}}}},
        })
        .to_string(),
    );
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()));
    let schema = format!("$schema:\n  - item: {ITEM}\n    tree: item(required)\n  - ./raw.json\n");
    let effective = effective(&schemas, &format!("---\n{schema}---\nbody\n"));
    let defs = effective.json_schema["$defs"].as_object().expect("union root $defs");
    assert_eq!(defs.keys().collect::<Vec<_>>(), ["item", "item-2"], "the generated key is kept; the raw one moves");
    assert_eq!(effective.json_schema["anyOf"][1]["properties"]["node"]["$ref"], "#/$defs/item-2");
    assert_eq!(defs["item-2"]["properties"]["next"]["$ref"], "#/$defs/item-2");

    let raw_good = json!({"node": {"n": 1, "next": {"n": 2, "next": {"n": 3}}}});
    let raw_bad = json!({"node": {"n": 1, "next": {"n": 2, "next": {"n": "three"}}}});
    assert!(effective.validate(&raw_good).valid, "{:#?}", effective.validate(&raw_good).problems);
    assert!(!effective.validate(&raw_bad).valid);
    assert!(effective.validate(&json!({"tree": tree(4, json!("x"))})).valid);
}

#[test]
fn an_arm_reference_that_would_bind_to_another_arm_is_refused() {
    let dir = TempDir::new().expect("tempdir");
    // `#/$defs/item` names nothing in the raw arm; under the union root it
    // would silently bind to the inline arm's `item`.
    write(
        dir.path(),
        "orphan.json",
        &json!({"type": "object", "properties": {"node": {"$ref": "#/$defs/item"}}}).to_string(),
    );
    write(
        dir.path(),
        "rooted.json",
        &json!({"type": "object", "properties": {"node": {"$ref": "#"}}, "$defs": {}}).to_string(),
    );
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()));
    let inline = format!("  - item: {ITEM}\n    tree: item\n");
    let orphan = schemas.effective_for(&markdown(&format!("---\n$schema:\n{inline}  - ./orphan.json\n---\nb\n")));
    match orphan {
        Err(SchemaError::UnresolvedReference { reference, location, .. }) => {
            assert_eq!((reference.as_str(), location.as_str()), ("#/$defs/item", "/properties/node"));
        }
        other => panic!("expected UnresolvedReference, got {:?}", other.map(|_| ())),
    }
    let rooted = schemas.effective_for(&markdown(&format!("---\n$schema:\n{inline}  - ./rooted.json\n---\nb\n")));
    assert!(
        matches!(rooted, Err(SchemaError::UnsupportedMergeReference { ref location, .. }) if location == "/properties/node"),
        "{:?}",
        rooted.map(|_| ())
    );
}

// ── Baselines ─────────────────────────────────────────────────────────────

/// A raw baseline with a recursive `$defs.item` used by `tree`.
fn raw_baseline() -> Value {
    json!({
        "type": "object",
        "properties": {
            "tree": {"$ref": "#/$defs/item"},
            "note": {
                "type": "string",
                "default": "#/$defs/item",
                "const": "kept",
                "examples": [{"$ref": "#/$defs/item"}],
                "x-darkmatter-suggest": [{"$defs": {"item": 1}}],
            },
        },
        "$defs": {
            "item": {
                "type": "object",
                "properties": {"label": {"type": "string"}, "kids": {"type": "array", "items": {"$ref": "#/$defs/item"}}},
                "required": ["label", "kids"],
            },
        },
    })
}

fn labeled(depth: usize, leaf: Value) -> Value {
    (1..depth).fold(json!({"label": leaf, "kids": []}), |child, _| json!({"label": "l", "kids": [child]}))
}

#[test]
fn a_baseline_with_definitions_joins_the_document_root_without_rebinding_either() {
    let dir = TempDir::new().expect("tempdir");
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .with_baseline_json_schema(raw_baseline())
        .expect("a recursive baseline is merge-compatible");
    let document = format!("---\n$schema:\n  item: {ITEM}\n  doc: item(required)\n---\nbody\n");
    let effective = effective(&schemas, &document);
    let schema = &effective.json_schema;
    let defs = schema["$defs"].as_object().expect("$defs");
    assert_eq!(defs.keys().collect::<Vec<_>>(), ["item", "item-2"]);
    assert_eq!(schema["properties"]["doc"]["$ref"], "#/$defs/item", "the document's key never moves");
    assert_eq!(schema["properties"]["tree"]["$ref"], "#/$defs/item-2");
    assert_eq!(defs["item-2"]["properties"]["kids"]["items"]["$ref"], "#/$defs/item-2");
    // Literal data that merely looks like a reference is byte-for-byte kept.
    assert_eq!(schema["properties"]["note"], raw_baseline()["properties"]["note"]);

    let doc_tree = tree(5, json!("x"));
    let good = json!({"doc": doc_tree, "tree": labeled(6, json!("deep"))});
    assert!(effective.validate(&good).valid, "{:#?}", effective.validate(&good).problems);
    let bad = json!({"doc": tree(5, json!("x")), "tree": labeled(6, json!({"not": "text"}))});
    let report = effective.validate(&bad);
    assert!(problem_paths(&report).contains(&format!("/tree{}/label", "/kids/0".repeat(5))), "{:#?}", report.problems);
    let crossed = json!({"doc": labeled(2, json!("x")), "tree": labeled(2, json!("x"))});
    assert!(!effective.validate(&crossed).valid, "the document's `doc` keeps its own body");
}

#[test]
fn a_baseline_with_definitions_joins_every_arm_of_a_root_union_once() {
    let dir = TempDir::new().expect("tempdir");
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .with_baseline_json_schema(raw_baseline())
        .expect("baseline");
    let document = "---\n$schema:\n  - kind: literal(a)\n  - kind: literal(b)\n---\nbody\n";
    let effective = effective(&schemas, document);
    let schema = &effective.json_schema;
    assert_eq!(schema["$defs"].as_object().expect("$defs").keys().collect::<Vec<_>>(), ["item"]);
    for arm in schema["anyOf"].as_array().expect("arms") {
        assert_eq!(arm["properties"]["tree"]["$ref"], "#/$defs/item");
        assert!(arm.get("$defs").is_none());
    }
    assert!(effective.validate(&json!({"kind": "b", "tree": labeled(5, json!("x"))})).valid);
    assert!(!effective.validate(&json!({"kind": "b", "tree": labeled(5, json!({"not": "text"}))})).valid);
}

#[test]
fn a_baseline_cannot_move_its_definitions_into_an_arm_with_its_own_id() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "scoped.json", &json!({"$id": "urn:example:scoped", "type": "object"}).to_string());
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .with_baseline_json_schema(raw_baseline())
        .expect("baseline");
    let outcome = schemas.effective_for(&markdown("---\n$schema:\n  - kind: literal(a)\n  - ./scoped.json\n---\nb\n"));
    assert!(
        matches!(outcome, Err(SchemaError::UnsupportedMergeReference { ref location, .. }) if location == "/anyOf/1"),
        "{:?}",
        outcome.map(|_| ())
    );
    // Without definitions to move, the same arm merges as before.
    let plain = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .with_baseline_json_schema(json!({"type": "object", "properties": {"x": {"type": "string"}}}))
        .expect("baseline");
    assert!(plain.effective_for(&markdown("---\n$schema:\n  - kind: literal(a)\n  - ./scoped.json\n---\nb\n")).is_ok());
}

// ── Raw merge-ingress matrix ──────────────────────────────────────────────

/// What loading an edited baseline does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ingress {
    /// Loads; the control's deep negative instance is still rejected.
    Enforced,
    /// Loads; the edit removed the assertion the negative instance breaks.
    Loosened,
    /// `SchemaError::Baseline` (a shape or meta-schema error).
    Invalid,
    /// `SchemaError::UnresolvedReference`.
    Unresolved,
    /// `SchemaError::UnsupportedMergeReference`.
    Unsupported,
}

type Edit = fn(&mut Value);

/// One mutation of the control per cell, with its outcome.
fn ingress_cells() -> Vec<(&'static str, Edit, Ingress)> {
    use Ingress::*;
    fn item(schema: &mut Value) -> &mut Value {
        &mut schema["$defs"]["item"]
    }
    vec![
        ("control", |_| {}, Enforced),
        // `$defs`
        ("$defs absent", |s| drop(s.as_object_mut().unwrap().remove("$defs")), Unresolved),
        ("$defs null", |s| s["$defs"] = Value::Null, Invalid),
        ("$defs scalar", |s| s["$defs"] = json!(3), Invalid),
        ("$defs list", |s| s["$defs"] = json!([]), Invalid),
        ("$defs one invalid body", |s| s["$defs"]["other"] = json!(4), Invalid),
        ("$defs every body invalid", |s| s["$defs"] = json!({"item": 4, "other": "x"}), Invalid),
        ("$defs empty", |s| s["$defs"] = json!({}), Unresolved),
        // `$defs.item`
        ("item null", |s| *item(s) = Value::Null, Invalid),
        ("item number", |s| *item(s) = json!(1), Invalid),
        ("item string", |s| *item(s) = json!("object"), Invalid),
        ("item boolean", |s| *item(s) = json!(true), Loosened),
        ("item one invalid keyword", |s| item(s)["minProperties"] = json!("one"), Invalid),
        ("item every keyword invalid", |s| *item(s) = json!({"type": 5, "required": 6}), Invalid),
        ("item empty", |s| *item(s) = json!({}), Loosened),
        // property `$ref`
        ("$ref absent", |s| drop(s["properties"]["tree"].as_object_mut().unwrap().remove("$ref")), Loosened),
        ("$ref null", |s| s["properties"]["tree"]["$ref"] = Value::Null, Invalid),
        ("$ref number", |s| s["properties"]["tree"]["$ref"] = json!(1), Invalid),
        ("$ref list", |s| s["properties"]["tree"]["$ref"] = json!(["#/$defs/item"]), Invalid),
        ("$ref empty", |s| s["properties"]["tree"]["$ref"] = json!(""), Unsupported),
        ("$ref root", |s| s["properties"]["tree"]["$ref"] = json!("#"), Unsupported),
        ("$ref external", |s| s["properties"]["tree"]["$ref"] = json!("https://example.com/item.json"), Unsupported),
        ("$ref escaped pointer", |s| {
            let body = s["$defs"].as_object_mut().unwrap().remove("item").unwrap();
            let body = serde_json::to_string(&body).unwrap().replace("#/$defs/item", "#/$defs/a~1b");
            s["$defs"]["a/b"] = serde_json::from_str(&body).unwrap();
            s["properties"]["tree"]["$ref"] = json!("#/$defs/a~1b");
        }, Enforced),
        // `$id`
        ("$id in a definition", |s| item(s)["$id"] = json!("urn:example:item"), Unsupported),
        ("$id null", |s| item(s)["$id"] = Value::Null, Invalid),
        ("$id number", |s| item(s)["$id"] = json!(1), Invalid),
        ("$id list", |s| item(s)["$id"] = json!(["urn:a"]), Invalid),
        ("$id empty", |s| item(s)["$id"] = json!(""), Unsupported),
        // structural keywords of the definition
        ("type absent", |s| drop(item(s).as_object_mut().unwrap().remove("type")), Enforced),
        ("type null", |s| item(s)["type"] = Value::Null, Invalid),
        ("type wrong", |s| item(s)["type"] = json!({}), Invalid),
        ("type one wrong member", |s| item(s)["type"] = json!(["object", "thing"]), Invalid),
        ("type every member wrong", |s| item(s)["type"] = json!(["thing"]), Invalid),
        ("type empty list", |s| item(s)["type"] = json!([]), Invalid),
        ("type repeated member", |s| item(s)["type"] = json!(["object", "object"]), Invalid),
        ("properties absent", |s| drop(item(s).as_object_mut().unwrap().remove("properties")), Loosened),
        ("properties null", |s| item(s)["properties"] = Value::Null, Invalid),
        ("properties list", |s| item(s)["properties"] = json!([]), Invalid),
        ("properties one invalid", |s| item(s)["properties"]["extra"] = json!(3), Invalid),
        ("properties every invalid", |s| item(s)["properties"] = json!({"label": 1, "kids": 2}), Invalid),
        ("properties empty", |s| item(s)["properties"] = json!({}), Loosened),
        ("required absent", |s| drop(item(s).as_object_mut().unwrap().remove("required")), Enforced),
        ("required null", |s| item(s)["required"] = Value::Null, Invalid),
        ("required object", |s| item(s)["required"] = json!({}), Invalid),
        ("required one wrong member", |s| item(s)["required"] = json!(["label", 3]), Invalid),
        ("required every member wrong", |s| item(s)["required"] = json!([3]), Invalid),
        ("required empty", |s| item(s)["required"] = json!([]), Enforced),
        ("required repeated member", |s| item(s)["required"] = json!(["label", "label"]), Invalid),
        ("items absent", |s| drop(item(s)["properties"]["kids"].as_object_mut().unwrap().remove("items")), Loosened),
        ("items null", |s| item(s)["properties"]["kids"]["items"] = Value::Null, Invalid),
        ("items number", |s| item(s)["properties"]["kids"]["items"] = json!(2), Invalid),
        ("items boolean", |s| item(s)["properties"]["kids"]["items"] = json!(true), Loosened),
        ("items one invalid keyword", |s| item(s)["properties"]["kids"]["items"]["minItems"] = json!("x"), Invalid),
        ("items every keyword invalid", |s| item(s)["properties"]["kids"]["items"] = json!({"type": 1, "minItems": "x"}), Invalid),
        ("items empty", |s| item(s)["properties"]["kids"]["items"] = json!({}), Loosened),
        ("anyOf control", |s| item(s)["properties"]["label"] = json!({"anyOf": [{"type": "string"}, {"type": "null"}]}), Enforced),
        ("anyOf null", |s| item(s)["properties"]["label"] = json!({"anyOf": null}), Invalid),
        ("anyOf object", |s| item(s)["properties"]["label"] = json!({"anyOf": {}}), Invalid),
        ("anyOf one invalid arm", |s| item(s)["properties"]["label"] = json!({"anyOf": [{"type": "string"}, 3]}), Invalid),
        ("anyOf every arm invalid", |s| item(s)["properties"]["label"] = json!({"anyOf": [3]}), Invalid),
        ("anyOf empty", |s| item(s)["properties"]["label"] = json!({"anyOf": []}), Invalid),
        ("oneOf empty", |s| item(s)["properties"]["label"] = json!({"oneOf": []}), Invalid),
        ("root anyOf", |s| s["anyOf"] = json!([{"type": "object"}]), Invalid),
        ("root $ref", |s| s["$ref"] = json!("#/$defs/item"), Invalid),
    ]
}

/// A tree property whose deepest label breaks the control's item body.
fn negative_instance() -> Value {
    json!({"tree": labeled(5, json!({"not": "text"}))})
}

/// Loads `baseline` through `load` and judges the outcome against `expected`.
fn judge_ingress(cell: &str, outcome: Result<DarkmatterSchemas, SchemaError>, expected: Ingress) {
    let actual = match outcome {
        Ok(schemas) => {
            let effective = effective(&schemas, "---\ntitle: t\n---\nbody\n");
            assert!(effective.validate(&json!({"tree": labeled(5, json!("x"))})).valid, "{cell}: positive instance");
            match effective.validate(&negative_instance()).valid {
                false => Ingress::Enforced,
                true => Ingress::Loosened,
            }
        }
        Err(SchemaError::Baseline { .. }) => Ingress::Invalid,
        Err(SchemaError::UnresolvedReference { .. }) => Ingress::Unresolved,
        Err(SchemaError::UnsupportedMergeReference { .. }) => Ingress::Unsupported,
        Err(other) => panic!("{cell}: unexpected error {other:?}"),
    };
    assert_eq!(actual, expected, "{cell}");
}

fn raw_ingress_cells(extension: &str, serialize: fn(&Value) -> String) {
    let dir = TempDir::new().expect("tempdir");
    for (index, (cell, edit, expected)) in ingress_cells().into_iter().enumerate() {
        let mut baseline = raw_baseline();
        edit(&mut baseline);
        let path = write(dir.path(), &format!("cell-{index}.{extension}"), &serialize(&baseline));
        let outcome = DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).with_baseline_from_file(&path);
        // A file loader wraps a schema error in the baseline error that names
        // the file; the cell's own outcome is its source.
        let outcome = outcome.map_err(|error| match error {
            SchemaError::Baseline { source: Some(source), .. } => *source,
            other => other,
        });
        judge_ingress(&format!("{extension} {cell}"), outcome, expected);
    }
}

#[test]
fn raw_json_baseline_ingress_takes_every_cell_from_a_recursive_control() {
    raw_ingress_cells("json", |value| serde_json::to_string_pretty(value).expect("json"));

    let dir = TempDir::new().expect("tempdir");
    let control = serde_json::to_string_pretty(&raw_baseline()).expect("json");
    for (cell, text) in [
        ("duplicate $defs", control.replacen("\"$defs\": {", "\"$defs\": {}, \"$defs\": {", 1)),
        ("duplicate definition name", control.replacen("\"item\": {", "\"item\": {}, \"item\": {", 1)),
        ("trailing content", format!("{control}\n{{}}")),
    ] {
        let path = write(dir.path(), "edited.json", &text);
        let outcome = DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).with_baseline_from_file(&path);
        assert!(
            matches!(&outcome, Err(SchemaError::Baseline { source: Some(source), .. }) if matches!(**source, SchemaError::SchemaDocument { .. })),
            "{cell}: {:?}",
            outcome.map(|_| ())
        );
    }
}

#[test]
fn raw_yaml_baseline_ingress_takes_every_cell_from_a_recursive_control() {
    raw_ingress_cells("yaml", |value| serde_yaml_ng::to_string(value).expect("yaml"));

    let dir = TempDir::new().expect("tempdir");
    let control = serde_yaml_ng::to_string(&raw_baseline()).expect("yaml");
    for (cell, text) in [
        ("duplicate $defs", format!("{control}$defs: {{}}\n")),
        ("duplicate definition name", control.replacen("  item:\n", "  item: {}\n  item:\n", 1)),
        ("second document", format!("{control}---\ntype: object\n")),
        ("trailing content", format!("{control}]\n")),
    ] {
        let path = write(dir.path(), "edited.yaml", &text);
        let outcome = DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).with_baseline_from_file(&path);
        assert!(
            matches!(&outcome, Err(SchemaError::Baseline { source: Some(source), .. }) if matches!(**source, SchemaError::SchemaDocument { .. })),
            "{cell}: {:?}",
            outcome.map(|_| ())
        );
    }
}

#[test]
fn an_in_memory_baseline_shares_the_ingress_validator() {
    let dir = TempDir::new().expect("tempdir");
    let load = |edit: Edit| {
        let mut baseline = raw_baseline();
        edit(&mut baseline);
        DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).with_baseline_json_schema(baseline)
    };
    judge_ingress("in-memory control", load(|_| {}), Ingress::Enforced);
    judge_ingress("in-memory $defs empty", load(|s| s["$defs"] = json!({})), Ingress::Unresolved);
    judge_ingress("in-memory $id", load(|s| s["$defs"]["item"]["$id"] = json!("urn:x")), Ingress::Unsupported);
}

#[test]
fn a_baseline_without_definitions_keeps_its_other_reference_forms() {
    let dir = TempDir::new().expect("tempdir");
    // Nothing moves, so a reference into the merged root keeps the meaning
    // it had before definitions were admitted.
    let baseline = json!({"type": "object", "properties": {"a": {"type": "integer"}, "b": {"$ref": "#/properties/a"}}});
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .with_baseline_json_schema(baseline)
        .expect("accepted as before");
    let effective = effective(&schemas, "---\ntitle: t\n---\nbody\n");
    assert!(effective.validate(&json!({"b": 1})).valid);
    assert!(!effective.validate(&json!({"b": {"not": "an integer"}})).valid);
}

// ── Trigger payloads ──────────────────────────────────────────────────────

/// A temporary repository whose `schemas/` root holds the given files and a
/// document `doc.md` with `text`.
struct Repository {
    _dir: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Repository {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("repo");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("objects")).expect("git objects");
        std::fs::create_dir_all(git.join("refs/heads")).expect("git refs");
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("git HEAD");
        std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").expect("git config");
        for (name, text) in files {
            write(&root.join("schemas"), name, text);
        }
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).expect("home dir");
        Self { _dir: dir, root, home }
    }

    fn effective(&self, document: &str) -> Result<Option<EffectiveSchema>, SchemaError> {
        let path = write(&self.root, "docs/doc.md", document);
        let snapshot = RequestSnapshot::new(path.parent().expect("folder")).with_home(Some(self.home.clone()));
        let context = build_resolution_context(&snapshot).expect("fixture context");
        let markdown = Markdown::try_from(path.as_path()).expect("document").with_source(ComposeSource::File(path));
        DarkmatterSchemas::new(context).with_trigger_discovery()?.effective_for(&markdown)
    }
}

const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  - kind: enum(tree; required)\n$schema: payload.yaml\n";

#[test]
fn a_recursive_trigger_payload_merges_beside_a_recursive_document() {
    let payload = "$schema:\n  item: \"{ label: string(required), kids: item[](required) }\"\n  tree: item\n";
    let repo = Repository::new(&[("tree.trigger.yaml", TRIGGER), ("payload.yaml", payload)]);
    let document = format!("---\nkind: tree\n$schema:\n  item: {ITEM}\n  doc: item\n---\nbody\n");
    let effective = repo.effective(&document).expect("a recursive payload is merge-compatible").expect("effective");
    let defs = effective.json_schema["$defs"].as_object().expect("$defs");
    assert_eq!(defs.keys().collect::<Vec<_>>(), ["item", "item-2"]);
    assert_eq!(effective.json_schema["properties"]["doc"]["anyOf"][1]["$ref"], "#/$defs/item");
    assert_eq!(effective.json_schema["properties"]["tree"]["anyOf"][1]["$ref"], "#/$defs/item-2");

    let good = json!({"kind": "tree", "doc": tree(5, json!("x")), "tree": labeled(6, json!("x"))});
    assert!(effective.validate(&good).valid, "{:#?}", effective.validate(&good).problems);
    let bad = json!({"kind": "tree", "doc": tree(5, json!("x")), "tree": labeled(6, json!({"not": "text"}))});
    let bad_path = format!("/tree{}/label", "/kids/0".repeat(5));
    assert!(problem_paths(&effective.validate(&bad)).contains(&bad_path));
    // The phase merge keeps the payload's definitions beside the projection.
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        let report = effective.validate_for_phase(&bad, phase).expect("phase");
        assert!(problem_paths(&report).contains(&bad_path), "{phase:?}: {:#?}", report.problems);
    }

    // Each layer's references are looked up by identity, in one place.
    let payload_path = biscuit_file::canonicalize_simplified(&repo.root.join("schemas/payload.yaml")).expect("payload");
    let key = DefKey::new(DefNamespace::File(payload_path), "item");
    let definition = effective.definition(&key).expect("the payload's definition");
    assert_eq!(definition.json_key, "item", "keys are those of the payload's own lowering");
    assert!(effective.definition(&DefKey::new(DefNamespace::InlineRoot, "item")).is_some());
}

#[test]
fn a_raw_trigger_payload_with_an_unresolved_reference_is_not_mergeable() {
    let control = json!({"type": "object", "properties": {"tree": {"$ref": "#/$defs/item"}}, "$defs": {"item": {"type": "object"}}});
    let trigger = TRIGGER.replace("payload.yaml", "payload.json");
    let repo = Repository::new(&[("tree.trigger.yaml", &trigger), ("payload.json", &control.to_string())]);
    assert!(repo.effective("---\nkind: tree\n---\nbody\n").expect("control merges").is_some());

    let mut broken = control.clone();
    broken["$defs"] = json!({});
    let repo = Repository::new(&[("tree.trigger.yaml", &trigger), ("payload.json", &broken.to_string())]);
    match repo.effective("---\nkind: tree\n---\nbody\n") {
        Err(SchemaError::TriggerLoad { source, .. }) => match *source {
            SchemaError::TriggerPayloadNotMergeable { reason, .. } => {
                assert!(reason.contains("#/$defs/item") && reason.contains("/properties/tree"), "{reason}");
            }
            other => panic!("expected TriggerPayloadNotMergeable, got {other:?}"),
        },
        other => panic!("expected a trigger load error, got {:?}", other.map(|_| ())),
    }
}

/// An embedded trigger whose inline payload declares its own `item` and
/// imports a library's `node`, each recursive and each named `item` in its
/// own namespace.
const EMBEDDED_TRIGGER: &str = "kind: schema-trigger\n\
    match:\n  kind: enum(tree; required)\n\
    $schema:\n  item: \"{ label: string(required), kids: item[](required) }\"\n  tree: item\n  graph: item@./lib.yaml\n";

const EMBEDDED_LIBRARY: &str = "kind: schema\ntypes:\n  item: \"{ id: number(required), edges: item[](required) }\"\n";

#[test]
fn embedded_payload_namespaces_join_a_recursive_document_without_collision() {
    let bundle = darkmatter::markdown::schemas::EmbeddedSchemaBundle::new(
        "recursive-bundle",
        &[("host.yaml", EMBEDDED_TRIGGER), ("lib.yaml", EMBEDDED_LIBRARY)],
    );
    let repo = Repository::new(&[]);
    let snapshot = RequestSnapshot::new(&repo.root).with_home(Some(repo.home.clone()));
    let context = build_resolution_context(&snapshot).expect("fixture context");
    let schemas = DarkmatterSchemas::new(context)
        .with_trigger_discovery()
        .expect("discovery")
        .with_embedded_trigger_bundle(bundle)
        .expect("a recursive embedded payload is merge-compatible");
    let document = format!("---\nkind: tree\n$schema:\n  item: {ITEM}\n  doc: item\n---\nbody\n");
    let effective = effective(&schemas, &document);
    let defs = effective.json_schema["$defs"].as_object().expect("$defs");
    assert_eq!(defs.len(), 3, "three namespaces, three bodies: {:?}", defs.keys().collect::<Vec<_>>());
    assert_eq!(effective.json_schema["properties"]["doc"]["anyOf"][1]["$ref"], "#/$defs/item");

    let graph = |leaf: Value| {
        (1..6).fold(json!({"id": leaf, "edges": []}), |child, level| json!({"id": level, "edges": [child]}))
    };
    let good = json!({"kind": "tree", "doc": tree(5, json!("x")), "tree": labeled(5, json!("x")), "graph": graph(json!(9))});
    assert!(effective.validate(&good).valid, "{:#?}", effective.validate(&good).problems);
    for (property, bad) in [
        ("doc", json!({"kind": "tree", "doc": tree(5, json!({"not": "text"}))})),
        ("tree", json!({"kind": "tree", "tree": labeled(5, json!({"not": "text"}))})),
        ("graph", json!({"kind": "tree", "graph": graph(json!({"not": "a number"}))})),
    ] {
        let report = effective.validate(&bad);
        assert!(
            report.problems.iter().any(|problem| problem.path.starts_with(&format!("/{property}/"))),
            "{property}: {:#?}",
            report.problems
        );
    }
    let embedded_item = DefKey::new(DefNamespace::Embedded("recursive-bundle/host.yaml".into()), "item");
    let library_item = DefKey::new(DefNamespace::Embedded("recursive-bundle/lib.yaml".into()), "item");
    assert!(effective.definition(&embedded_item).is_some() && effective.definition(&library_item).is_some());
}

// ── Phases ────────────────────────────────────────────────────────────────

/// `name` is required only at completion; `stamp` is eager, so it is required
/// at launch too.
const TIMED_ITEM: &str = "\"{ name: string(required), stamp: string(required; eager), kids: item[](required) }\"";

fn timed(depth: usize, leaf: Value) -> Value {
    (1..depth).fold(leaf, |child, _| json!({"name": "n", "stamp": "s", "kids": [child]}))
}

#[test]
fn a_phase_projects_presence_below_the_recursion() {
    let dir = TempDir::new().expect("tempdir");
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()));
    let effective = effective(&schemas, &format!("---\n$schema:\n  item: {TIMED_ITEM}\n  root: item\n---\nb\n"));
    let without_name = json!({"root": timed(5, json!({"stamp": "s", "kids": []}))});
    let without_stamp = json!({"root": timed(5, json!({"name": "n", "kids": []}))});
    let deep = format!("/root{}", "/kids/0".repeat(4));
    let judge = |instance: &Value, phase| effective.validate_for_phase(instance, phase).expect("phase schema");

    assert!(judge(&without_name, SchemaPhase::Launch).valid, "{:#?}", judge(&without_name, SchemaPhase::Launch).problems);
    let completion = judge(&without_name, SchemaPhase::Completion);
    assert!(problem_paths(&completion).contains(&deep), "{:#?}", completion.problems);
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        let report = judge(&without_stamp, phase);
        assert!(problem_paths(&report).contains(&deep), "{phase:?}: {:#?}", report.problems);
    }
}

#[test]
fn a_definition_shared_by_eager_and_deferred_properties_projects_once() {
    let dir = TempDir::new().expect("tempdir");
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()));
    let schema = format!("$schema:\n  item: {ITEM}\n  now: item(required; eager)\n  later: item(required)\n");
    let effective = effective(&schemas, &format!("---\n{schema}---\nb\n"));
    let launch = |instance: Value| effective.validate_for_phase(&instance, SchemaPhase::Launch).expect("phase");
    let completion = |instance: Value| effective.validate_for_phase(&instance, SchemaPhase::Completion).expect("phase");

    assert!(launch(json!({"now": tree(4, json!("x"))})).valid, "a deferred property may be absent at launch");
    assert!(!launch(json!({"later": tree(4, json!("x"))})).valid, "an eager property may not");
    assert!(!completion(json!({"now": tree(4, json!("x"))})).valid);
    let report = launch(json!({"now": tree(4, json!("x")), "later": tree(4, json!({"not": "text"}))}));
    assert!(problem_paths(&report).contains(&deep_value_path("later", 4)), "{:#?}", report.problems);
    let both = json!({"now": tree(4, json!("x")), "later": tree(4, json!("y"))});
    assert!(completion(both.clone()).valid && launch(both).valid);
}

// ── Example targets ───────────────────────────────────────────────────────

#[test]
fn an_example_on_a_recursive_property_is_judged_with_the_root_definitions() {
    let dir = TempDir::new().expect("tempdir");
    let example = |returns: &str| format!("kind: example\ninvocation: tree\nreturns: {returns}\ndescription: d\n");
    write(dir.path(), "good.yaml", &example("{ value: v, children: [{ value: w, children: [] }] }"));
    write(dir.path(), "bad.yaml", &example("{ value: v, children: [{ value: { not: text }, children: [] }] }"));
    let schema = |file: &str| format!("item: {ITEM}\ntree: item(example(./{file}))\n");
    let resolve = |file: &str| {
        let value: YamlValue = serde_yaml_ng::from_str(&schema(file)).expect("yaml");
        resolve_yaml_schema(&value, dir.path(), &FileResolutionContext::new(dir.path()))
    };
    let resolved = resolve("good.yaml").expect("a matching example resolves");
    assert!(resolved.json_schema["$defs"].get("item").is_some());
    match resolve("bad.yaml") {
        Err(SchemaError::InvalidExample { message, .. }) => assert!(message.contains("returns") || message.contains('3'), "{message}"),
        other => panic!("expected InvalidExample, got {:?}", other.map(|_| ())),
    }
}
