//! Which acyclic named types the resolver stores once, and which uses of a
//! stored type stay inline.
//!
//! A type is stored when its value is structured (an object or dictionary, a
//! tuple, or a list or union of something structured) and two or more
//! distinct authored sites reference it. Each case reads the public result:
//! the lowered JSON Schema's `$defs` and `$ref`s, what it accepts, the typed
//! errors resolution reports, and the definitions an expansion returns.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, expand_named_types, resolve_yaml_schema};
use darkmatter::markdown::schemas::simplified::types::Constraint;
use darkmatter::markdown::schemas::{SchemaConstraintConflictKind, SchemaError, parse_yaml_schema};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use tempfile::TempDir;

fn temp_root() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical tempdir");
    (dir, root)
}

fn resolve(dir: &Path, schema: &str) -> Result<ResolvedSchema, SchemaError> {
    let value: YamlValue = serde_yaml_ng::from_str(schema).expect("fixture YAML parses");
    resolve_yaml_schema(&value, dir, &FileResolutionContext::new(dir))
}

fn resolved(dir: &Path, schema: &str) -> ResolvedSchema {
    resolve(dir, schema).unwrap_or_else(|error| panic!("schema resolves: {error}"))
}

/// The generated `$defs` keys, in order.
fn defs(resolved: &ResolvedSchema) -> Vec<String> {
    resolved.json_schema["$defs"].as_object().map(|defs| defs.keys().cloned().collect()).unwrap_or_default()
}

/// How many `$ref`s anywhere in the lowered schema point at `key`.
fn refs_to(resolved: &ResolvedSchema, key: &str) -> usize {
    fn count(value: &Value, pointer: &str) -> usize {
        match value {
            Value::Object(map) => {
                usize::from(map.get("$ref").and_then(Value::as_str) == Some(pointer))
                    + map.values().map(|child| count(child, pointer)).sum::<usize>()
            }
            Value::Array(items) => items.iter().map(|child| count(child, pointer)).sum(),
            _ => 0,
        }
    }
    count(&resolved.json_schema, &format!("#/$defs/{key}"))
}

fn accepts(resolved: &ResolvedSchema, instance: Value) -> bool {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&resolved.json_schema)
        .expect("the lowered schema compiles")
        .is_valid(&instance)
}

fn property<'a>(resolved: &'a ResolvedSchema, name: &str) -> &'a Value {
    &resolved.json_schema["properties"][name]
}

/// The non-null alternative of an optional property.
fn present(value: &Value) -> &Value {
    match value["anyOf"].as_array() {
        Some(arms) if arms.len() == 2 && arms[0] == json!({ "type": "null" }) => &arms[1],
        _ => value,
    }
}

const ENTRY: &str = "entry: \"{ name: string, size: number(min(0)) }\"\n";

#[test]
fn a_repeated_object_is_stored_once_and_referenced_at_each_use() {
    let (_dir, root) = temp_root();
    let schema = resolved(&root, &format!("{ENTRY}first: entry\nsecond: entry(required)\n"));
    assert_eq!(defs(&schema), ["entry"]);
    assert_eq!(refs_to(&schema, "entry"), 2);
    assert_eq!(schema.definitions.len(), 1);
    assert_eq!(schema.json_schema["required"], json!(["second"]), "presence stays with each use");
    assert!(accepts(&schema, json!({ "first": { "name": "a", "size": 1 }, "second": { "name": "b" } })));
    assert!(!accepts(&schema, json!({ "first": { "size": -1 }, "second": {} })));
    assert!(!accepts(&schema, json!({ "second": { "size": -1 } })), "the body's bound holds at the second use");
    assert!(!accepts(&schema, json!({ "first": {} })), "the second use stays required");
}

#[test]
fn a_type_used_once_is_inlined() {
    let (_dir, root) = temp_root();
    let schema = resolved(&root, &format!("{ENTRY}only: entry\n"));
    assert!(defs(&schema).is_empty());
    assert!(schema.definitions.is_empty());
    assert_eq!(present(property(&schema, "only"))["type"], "object");
}

#[test]
fn repeated_primitive_shapes_stay_inlined() {
    let (_dir, root) = temp_root();
    let schema = resolved(
        &root,
        "slug: alphanumeric(allow-dashes)\ntags: string[]\nid: [string, number]\nids: id[]\n\
         listed: [boolean, \"string[]\"]\n\
         a1: slug(min(3))\na2: slug\nb1: tags(min(1))\nb2: tags\nc1: id\nc2: id\nd1: ids\nd2: ids\n\
         e1: listed\ne2: listed\n",
    );
    assert!(defs(&schema).is_empty(), "{:?}", defs(&schema));
    assert_eq!(present(property(&schema, "a1"))["minLength"], 3, "a narrowed alias stays flat");
    assert!(present(property(&schema, "a2")).get("minLength").is_none());
    assert_eq!(present(property(&schema, "b1"))["minItems"], 1);
}

#[test]
fn repeated_structured_shapes_are_stored() {
    let (_dir, root) = temp_root();
    let schema = resolved(
        &root,
        "dict:\n  \"<string>\": number\npair: \"tuple([string, number])\"\nrows: \"{ n: string }[]\"\n\
         choice: [\"{ n: string }\", number]\n\
         d1: dict\nd2: dict\np1: pair\np2: pair\nr1: rows\nr2: rows\nc1: choice\nc2: choice\n",
    );
    assert_eq!(defs(&schema), ["dict", "pair", "rows", "choice"]);
    for key in ["dict", "pair", "rows", "choice"] {
        assert_eq!(refs_to(&schema, key), 2, "{key}");
    }
    assert!(accepts(&schema, json!({ "d1": { "x": 1 }, "p2": ["a", 1], "r1": [{ "n": "a" }], "c2": 3 })));
    assert!(!accepts(&schema, json!({ "d2": { "x": "one" } })));
    assert!(!accepts(&schema, json!({ "p1": [1, "a"] })));
    assert!(!accepts(&schema, json!({ "r2": [{ "n": 1 }] })));
    assert!(!accepts(&schema, json!({ "c1": "word" })));
}

#[test]
fn two_sites_in_one_body_and_two_aliases_each_count() {
    let (_dir, root) = temp_root();
    // In a library, so the named types are not document properties
    // themselves.
    std::fs::write(
        root.join("types.yaml"),
        format!("kind: schema\ntypes:\n  {ENTRY}  pair: \"{{ left: entry, right: entry }}\"\n  one: entry\n  two: entry\n"),
    )
    .expect("write types");
    let body = resolved(&root, "p: pair@./types.yaml\n");
    assert_eq!(defs(&body), ["entry"], "two sites in one body; the parent itself has one use");
    assert_eq!(refs_to(&body, "entry"), 2);

    let aliases = resolved(&root, "x: one@./types.yaml\ny: two@./types.yaml\n");
    assert_eq!(defs(&aliases), ["entry"], "each alias is one site; each alias has one use");
    assert_eq!(refs_to(&aliases, "entry"), 2);
}

#[test]
fn uses_of_a_parent_do_not_multiply_the_references_in_its_body() {
    let (_dir, root) = temp_root();
    let schema = resolved(&root, &format!("{ENTRY}parent: \"{{ e: entry }}\"\na: parent\nb: parent\n"));
    assert_eq!(defs(&schema), ["parent"], "`entry` is written once, inside `parent`");
    assert_eq!(refs_to(&schema, "parent"), 2);
    assert_eq!(schema.json_schema["$defs"]["parent"]["properties"]["e"]["anyOf"][1]["type"], "object");
}

#[test]
fn a_root_declaration_walked_again_as_a_definition_counts_its_references_once() {
    let (_dir, root) = temp_root();
    // `holder` is both a root property and the body `other` references; the
    // `entry` written inside it is one authored site.
    let schema = resolved(&root, &format!("{ENTRY}holder: \"{{ e: entry }}\"\nother: holder\n"));
    assert!(defs(&schema).is_empty(), "{:?}", defs(&schema));
}

#[test]
fn unused_declarations_and_callable_signatures_do_not_count() {
    let (_dir, root) = temp_root();
    std::fs::write(
        root.join("types.yaml"),
        format!(
            "kind: schema\ntypes:\n  {ENTRY}  unused: \"{{ a: entry, b: entry }}\"\n  \
             call: \"function(parameters(x: entry, y: entry); returns(entry))\"\n"
        ),
    )
    .expect("write types");
    let schema = resolved(&root, "only: entry@./types.yaml\n");
    assert!(defs(&schema).is_empty(), "{:?}", defs(&schema));

    // A callable is expanded passively, without lowering, so its signature
    // is still read; its three references add nothing to `entry`'s one site.
    let authored: YamlValue = serde_yaml_ng::from_str("only: entry@./types.yaml\nrun: call@./types.yaml\n").expect("yaml");
    let expansion =
        expand_named_types(&parse_yaml_schema(&authored).expect("parses"), &root, None, &[], &FileResolutionContext::new(&root))
            .expect("expands");
    assert!(expansion.definitions.is_empty(), "{:?}", expansion.definitions);
}

#[test]
fn a_target_is_its_namespace_and_name() {
    let (_dir, root) = temp_root();
    std::fs::write(root.join("a.yaml"), "kind: schema\ntypes:\n  entry: \"{ x: string }\"\n").expect("write a");
    std::fs::write(root.join("b.yaml"), "kind: schema\ntypes:\n  entry: \"{ y: string }\"\n").expect("write b");
    let separate = resolved(&root, "a1: entry@./a.yaml\nb1: entry@./b.yaml\n");
    assert!(defs(&separate).is_empty(), "same names in two files are two targets");

    // Two spellings of one file are one namespace.
    let pooled = resolved(&root, "a1: entry@./a.yaml\na2: entry@a.yaml\nb1: entry@./b.yaml\n");
    assert_eq!(defs(&pooled), ["entry"]);
    assert_eq!(refs_to(&pooled, "entry"), 2);
    assert_eq!(pooled.json_schema["$defs"]["entry"]["properties"]["x"]["anyOf"][1]["type"], "string");
}

#[test]
fn an_added_key_bound_stays_beside_its_own_reference() {
    let (_dir, root) = temp_root();
    let schema = resolved(&root, &format!("{ENTRY}bounded: entry(min-keys(1))\nplain: entry\n"));
    assert_eq!(defs(&schema), ["entry"]);
    assert_eq!(refs_to(&schema, "entry"), 2);
    assert!(schema.json_schema["$defs"]["entry"].get("minProperties").is_none(), "the body is unchanged");
    assert_eq!(present(property(&schema, "bounded"))["minProperties"], 1);
    assert!(present(property(&schema, "plain")).get("minProperties").is_none());
    assert!(!accepts(&schema, json!({ "bounded": {} })));
    assert!(accepts(&schema, json!({ "plain": {} })));
}

#[test]
fn a_key_exclusion_that_changes_the_body_bound_inlines_that_use_only() {
    let (_dir, root) = temp_root();
    std::fs::write(
        root.join("types.yaml"),
        "kind: schema\ntypes:\n    entry: \"{ value: string, note: string }(max-keys(1))\"\n",
    )
    .expect("write types");
    let schema = resolved(
        &root,
        "ordinary: entry@./types.yaml\nannotated: entry(uncounted-keys(note))@./types.yaml\n",
    );
    assert_eq!(defs(&schema), ["entry"]);
    assert_eq!(refs_to(&schema, "entry"), 1, "only `ordinary` is a reference");
    assert_eq!(present(property(&schema, "annotated"))["type"], "object");
    let both = json!({ "value": "v", "note": "n" });
    assert!(accepts(&schema, json!({ "annotated": both })));
    assert!(!accepts(&schema, json!({ "ordinary": both })));
    assert!(accepts(&schema, json!({ "ordinary": { "value": "v" } })));

    // The same exclusion as the body's keeps the reference.
    std::fs::write(
        root.join("types.yaml"),
        "kind: schema\ntypes:\n    entry: \"{ value: string, note: string }(max-keys(1); uncounted-keys(note))\"\n",
    )
    .expect("rewrite types");
    let same = resolved(&root, "ordinary: entry@./types.yaml\nrepeated: entry(uncounted-keys(note); min-keys(1))@./types.yaml\n");
    assert_eq!(refs_to(&same, "entry"), 2);
    assert!(accepts(&same, json!({ "repeated": both })));
    assert!(!accepts(&same, json!({ "repeated": { "note": "n" } })));
}

#[test]
fn an_alias_used_with_an_unrepresentable_postfix_inlines_through_to_the_body() {
    let (_dir, root) = temp_root();
    std::fs::write(
        root.join("types.yaml"),
        "kind: schema\ntypes:\n    entry: \"{ value: string, note: string }(max-keys(1))\"\n    alias: entry\n",
    )
    .expect("write types");
    let schema = resolved(
        &root,
        "x: alias@./types.yaml\ny: alias(uncounted-keys(note))@./types.yaml\nz: entry@./types.yaml\n\
         w: [\"alias@./types.yaml\", number]\n",
    );
    assert_eq!(defs(&schema), ["entry", "alias"]);
    assert_eq!(present(property(&schema, "y"))["type"], "object", "the whole alias chain is inlined");
    let both = json!({ "value": "v", "note": "n" });
    assert!(accepts(&schema, json!({ "y": both })));
    for name in ["x", "z", "w"] {
        assert!(!accepts(&schema, json!({ name: both })), "{name}");
        assert!(accepts(&schema, json!({ name: { "value": "v" } })), "{name}");
    }
    assert!(accepts(&schema, json!({ "w": 3 })));
}

#[test]
fn when_every_use_inlines_no_definition_is_emitted() {
    let (_dir, root) = temp_root();
    let schema = resolved(
        &root,
        "entry: \"{ value: string, note: string }(max-keys(1))\"\n\
         a: entry(uncounted-keys(note))\nb: entry(uncounted-keys(value))\n",
    );
    assert!(schema.json_schema.get("$defs").is_none());
    assert!(schema.definitions.is_empty());
    assert!(accepts(&schema, json!({ "a": { "value": "v", "note": "n" }, "b": { "value": "v", "note": "n" } })));
}

#[test]
fn a_conflicting_bound_keeps_its_typed_error_and_inherited_first_origins() {
    let (_dir, root) = temp_root();
    let error = resolve(
        &root,
        "entry: \"{ a: string, b: string }(max-keys(1))\"\nwide: entry(min-keys(2))\nplain: entry\n",
    )
    .expect_err("min-keys(2) contradicts the body's max-keys(1)");
    let SchemaError::ConstraintConflict { kind, constraints, .. } = &error else {
        panic!("expected ConstraintConflict, got {error:?}");
    };
    assert_eq!(*kind, SchemaConstraintConflictKind::Bounds);
    let participants: Vec<(Constraint, String)> = constraints
        .iter()
        .map(|participant| (participant.constraint.clone(), participant.origin.path.to_string()))
        .collect();
    assert_eq!(participants[0].0, Constraint::MaxKeys(1), "{participants:?}");
    assert!(participants[0].1.starts_with("entry"), "the inherited declaration leads: {participants:?}");
    assert_eq!(participants[1], (Constraint::MinKeys(2), "wide".to_string()));
    let message = error.to_string();
    for participant in constraints {
        assert!(message.contains(&participant.constraint.diagnostic_excerpt()), "{message}");
    }
}

#[test]
fn a_default_conflict_is_reported_through_a_shared_list() {
    let (_dir, root) = temp_root();
    let error = resolve(
        &root,
        "rows: \"{ n: string }[](default([]))\"\na: \"rows(default([1]))\"\nb: rows\n",
    )
    .expect_err("two different defaults for one use");
    assert!(matches!(&error, SchemaError::ConstraintConflict { .. }), "{error:?}");
}

#[test]
fn context_dependent_union_bodies_stay_inlined() {
    let (_dir, root) = temp_root();
    // A second suggestion reached through a union arm is still counted.
    let error = resolve(
        &root,
        "choice: [\"string(suggest(a, b))\", \"{ n: string }\"]\nx: [choice, \"number(suggest(1))\"]\ny: choice\n",
    )
    .expect_err("two suggestions in one property");
    assert!(error.to_string().contains("at most one `suggest`"), "{error}");

    // An optional `file` alternative tolerates the empty string only where
    // the use is optional.
    let files = resolved(&root, "asset: [file, \"{ path: string }\"]\nloose: asset\nstrict: asset(required)\n");
    assert!(defs(&files).is_empty(), "{:?}", defs(&files));
    let tolerates_empty = |name: &str| property(&files, name).to_string().contains(r#"{"const":""}"#);
    assert!(tolerates_empty("loose"));
    assert!(!tolerates_empty("strict"));

    // A nullable union makes each use optional.
    let nullable = resolved(&root, "maybe: [null, \"{ n: string }\"]\na: maybe(required)\nb: maybe\n");
    assert!(defs(&nullable).is_empty(), "{:?}", defs(&nullable));
    assert!(nullable.json_schema.get("required").is_none());
    assert!(accepts(&nullable, json!({ "a": null, "b": { "n": "x" } })));
}

#[test]
fn a_callable_in_a_shared_body_is_rejected_at_a_use() {
    let (_dir, root) = temp_root();
    std::fs::write(
        root.join("types.yaml"),
        "kind: schema\ntypes:\n  holder: \"{ run: function(parameters(x: string); returns(string)) }\"\n",
    )
    .expect("write types");
    let error = resolve(&root, "a: holder@./types.yaml\nb: holder@./types.yaml\n")
        .expect_err("a function type cannot type a value");
    let SchemaError::Convert { property, message } = &error else {
        panic!("expected a conversion error, got {error:?}");
    };
    assert!(message.contains("function"), "{message}");
    assert!(property.starts_with("a."), "the first use names the place: {property}");
}

/// The library text of a chain `t0 → t1 → … → t{length - 1}` whose last link is an object; with
/// `shared`, each link writes its successor twice, so every link but the
/// first is stored.
fn chain(length: usize, shared: bool) -> String {
    let mut text = String::new();
    for index in 0..length - 1 {
        let next = index + 1;
        if shared {
            text.push_str(&format!("t{index}: \"{{ a: t{next}, b: t{next} }}\"\n"));
        } else {
            text.push_str(&format!("t{index}: \"{{ a: t{next} }}\"\n"));
        }
    }
    text.push_str(&format!("t{}: \"{{ leaf: string }}\"\n", length - 1));
    text
}

#[test]
fn storing_definitions_keeps_the_import_depth_limit() {
    let (_dir, root) = temp_root();
    // 32 frames is the deepest chain expansion admits.
    let library = |length: usize, shared: bool| {
        let types: String = chain(length, shared).lines().map(|line| format!("  {line}\n")).collect();
        std::fs::write(root.join("types.yaml"), format!("kind: schema\ntypes:\n{types}")).expect("write types");
    };
    for shared in [false, true] {
        library(32, shared);
        let accepted = resolved(&root, "root: t0@./types.yaml\n");
        assert_eq!(defs(&accepted).len(), if shared { 31 } else { 0 }, "shared: {shared}");
        library(33, shared);
        let error = resolve(&root, "root: t0@./types.yaml\n").expect_err("one link past the limit");
        assert!(
            matches!(&error, SchemaError::ImportCycle { chain } if chain.contains("exceeded 32 levels")),
            "shared: {shared}: {error:?}"
        );
    }
}

#[test]
fn shared_output_is_deterministic() {
    let (_dir, root) = temp_root();
    let text = format!("{ENTRY}pair: \"{{ left: entry, right: entry }}\"\na: pair\nb: pair\nc: entry\n");
    let first = resolved(&root, &text);
    let second = resolved(&root, &text);
    assert_eq!(first.json_schema.to_string(), second.json_schema.to_string());
    assert_eq!(format!("{:?}", first.definitions), format!("{:?}", second.definitions));
    // First reached first: `pair`'s body reaches `entry` before `pair` is
    // stored.
    assert_eq!(defs(&first), ["entry", "pair"]);
}
