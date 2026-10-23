//! Recursive named types through the public resolver and validator: which
//! cycles are admitted, how a cyclic definition is stored once and used
//! through `$ref`, and what each use site inherits.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, DefNamespace, PropertyDef, SchemaConstraintConflictKind, SchemaError,
    SchemaSourceId, SimplifiedSchema, TypeExpr, ValidationProblemKind, ValidationReport,
};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use tempfile::TempDir;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fixture");
    path
}

fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).expect("fixture path canonicalizes")
}

fn resolve_inline(dir: &Path, schema: &str) -> Result<ResolvedSchema, SchemaError> {
    let value: YamlValue = serde_yaml_ng::from_str(schema).expect("fixture YAML parses");
    resolve_yaml_schema(&value, dir, &FileResolutionContext::new(dir))
}

fn validator(resolved: &ResolvedSchema) -> jsonschema::Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&resolved.json_schema)
        .expect("the lowered schema compiles")
}

/// Validates a whole Markdown document written in `dir`, as `md schema
/// validate` does.
fn validate_document(dir: &Path, document: &str) -> ValidationReport {
    let md: Markdown = document.to_string().into();
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .validate(&md)
        .expect("the document's schema resolves")
}

fn non_descending_cycles(error: &SchemaError) -> Vec<Vec<(String, String)>> {
    let SchemaError::NonDescendingCycle { cycles } = error else {
        panic!("expected NonDescendingCycle, got {error:?}");
    };
    cycles
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|reference| (reference.site.path.to_string(), reference.target.clone()))
                .collect()
        })
        .collect()
}

fn pair(site: &str, target: &str) -> (String, String) {
    (site.to_string(), target.to_string())
}

/// A YAML block for `tree:` nested `depth` levels deep through `children`,
/// whose deepest `value` is `leaf`.
fn nested_tree(depth: usize, leaf: &str) -> String {
    let mut out = String::from("tree:\n");
    for level in 0..depth {
        let indent = "  ".repeat(2 * level + 1);
        let value = if level + 1 == depth { leaf.to_string() } else { format!("n{level}") };
        if level > 0 {
            out.push_str(&format!("{}- value: {value}\n", "  ".repeat(2 * level)));
        } else {
            out.push_str(&format!("{indent}value: {value}\n"));
        }
        let child_indent = if level > 0 { "  ".repeat(2 * level + 1) } else { indent.clone() };
        if level + 1 == depth {
            out.push_str(&format!("{child_indent}children: []\n"));
        } else {
            out.push_str(&format!("{child_indent}children:\n"));
        }
    }
    out
}

const TREE_SCHEMA: &str = "$schema:\n  item: \"{ value: string(required), children: item[](required) }\"\n  tree: item(required)\n";

// ── Recursive validation at depth ─────────────────────────────────────────

#[test]
fn a_self_recursive_type_validates_depth_eight_and_positions_a_bad_leaf() {
    let dir = TempDir::new().expect("tempdir");
    let good = format!("---\n{}{TREE_SCHEMA}---\nbody\n", nested_tree(8, "deep"));
    let report = validate_document(dir.path(), &good);
    assert!(report.valid, "a finite depth-8 tree validates: {:#?}", report.problems);

    let bad = format!("---\n{}{TREE_SCHEMA}---\nbody\n", nested_tree(8, "42"));
    let report = validate_document(dir.path(), &bad);
    assert!(!report.valid);
    let pointer = format!("/tree{}/value", "/children/0".repeat(7));
    let problem = report
        .problems
        .iter()
        .find(|problem| problem.path == pointer)
        .unwrap_or_else(|| panic!("no problem at {pointer}: {:#?}", report.problems));
    assert!(matches!(problem.kind, ValidationProblemKind::Type), "{problem:#?}");
    let line = bad.lines().position(|line| line.ends_with("value: 42")).expect("bad leaf line") + 1;
    let column = bad.lines().nth(line - 1).expect("line").find("42").expect("value") + 1;
    assert_eq!((problem.line, problem.column), (Some(line as u32), Some(column as u32)), "{problem:#?}");
}

#[test]
fn a_two_file_cycle_validates_depth_six_and_records_both_files() {
    let dir = TempDir::new().expect("tempdir");
    let a = write(
        dir.path(),
        "a.yaml",
        "kind: schema\ntypes:\n  node: \"{ name: string(required), next: leaf(required)@./b.yaml }\"\n",
    );
    let b = write(dir.path(), "b.yaml", "kind: schema\ntypes:\n  leaf: \"{ back: node[](required)@./a.yaml }\"\n");
    let resolved = resolve_inline(dir.path(), "root: node(required)@./a.yaml\n").expect("two-file cycle resolves");
    assert_eq!(resolved.imports, vec![canonical(&a), canonical(&b)], "the closing file is a dependency");
    assert_eq!(resolved.definitions.len(), 2);

    // root → next → back[0] → next → back[0] → next → back[0]: depth six.
    let level4 = json!({"name": "4", "next": {"back": []}});
    let level3 = json!({"name": "3", "next": {"back": [level4]}});
    let level2 = json!({"name": "2", "next": {"back": [level3]}});
    let deep = json!({"root": {"name": "1", "next": {"back": [level2]}}});
    let validator = validator(&resolved);
    assert!(validator.is_valid(&deep), "depth 6 validates");
    let mut bad = deep.clone();
    bad["root"]["next"]["back"][0]["next"]["back"][0]["next"]["back"][0]["name"] = json!(4);
    let errors: Vec<String> =
        validator.iter_errors(&bad).map(|error| error.instance_path().to_string()).collect();
    assert!(
        errors.iter().any(|path| path == "/root/next/back/0/next/back/0/next/back/0/name"),
        "{errors:?}"
    );
}

#[test]
fn mutual_recursion_in_one_file_stores_each_definition_once() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "group: \"{ members: member[] }\"\nmember: \"{ name: string, group: group }\"\nroot: group\n",
    )
    .expect("mutual recursion resolves");
    let defs = resolved.json_schema["$defs"].as_object().expect("$defs");
    // `member` is the first definition reached, inside the root `group`.
    assert_eq!(defs.keys().collect::<Vec<_>>(), ["member", "group"]);
    assert_eq!(resolved.json_schema["properties"]["root"]["anyOf"][1], json!({"$ref": "#/$defs/group"}));
    let instance = json!({"root": {"members": [{"name": "a", "group": {"members": [{"name": "b", "group": {"members": []}}]}}]}});
    assert!(validator(&resolved).is_valid(&instance));
    let bad = json!({"root": {"members": [{"name": "a", "group": {"members": [{"name": 1, "group": {"members": []}}]}}]}});
    assert!(!validator(&resolved).is_valid(&bad));
}

// ── Which cycles are admitted ─────────────────────────────────────────────

#[test]
fn every_descending_position_closes_a_cycle() {
    let dir = TempDir::new().expect("tempdir");
    for (position, schema, instance) in [
        ("array item", "item: \"{ children: item[] }\"\n", json!({"item": {"children": [{"children": []}]}})),
        ("object property", "item: \"{ next: item }\"\n", json!({"item": {"next": {"next": null}}})),
        ("dictionary value", "item:\n  <string>: item\n", json!({"item": {"a": {"b": {}}}})),
        (
            "tuple slot",
            "item: \"tuple([string, item?])\"\n",
            json!({"item": ["a", ["b", ["c"]]]}),
        ),
    ] {
        let resolved = resolve_inline(dir.path(), schema).unwrap_or_else(|error| panic!("{position}: {error:?}"));
        assert_eq!(resolved.definitions.len(), 1, "{position}");
        assert!(validator(&resolved).is_valid(&instance), "{position}: {}", resolved.json_schema);
    }
}

#[test]
fn an_alias_cycle_is_rejected_naming_each_reference() {
    let dir = TempDir::new().expect("tempdir");
    let error = resolve_inline(dir.path(), "a: b\nb: a\n").expect_err("alias cycle");
    assert_eq!(non_descending_cycles(&error), [vec![pair("b", "a"), pair("a", "b")]]);
}

#[test]
fn a_self_union_is_rejected_even_though_another_arm_accepts_values() {
    let dir = TempDir::new().expect("tempdir");
    let error = resolve_inline(dir.path(), "a: [a, string]\n").expect_err("self-union");
    assert_eq!(non_descending_cycles(&error), [vec![pair("a[0]", "a")]]);
}

#[test]
fn a_cycle_through_a_function_signature_is_rejected() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  f: \"function(parameters(x: f); returns(string))\"\n",
    );
    let error = resolve_inline(dir.path(), "p: f@./types.yaml\n").expect_err("callable cycle");
    assert_eq!(non_descending_cycles(&error), [vec![pair("f.x", "f")]]);
}

#[test]
fn a_mixed_group_reports_only_its_non_descending_cycle() {
    let dir = TempDir::new().expect("tempdir");
    let error = resolve_inline(dir.path(), "a: [b, \"{ x: a }\"]\nb: a\nroot: a\n").expect_err("mixed");
    // Edges are reported in discovery order.
    assert_eq!(non_descending_cycles(&error), [vec![pair("b", "a"), pair("a[0]", "b")]]);

    let resolved = resolve_inline(dir.path(), "a: [b, \"{ x: a }\"]\nb: \"{ y: a }\"\nroot: a\n")
        .expect("without the alias cycle the same group resolves");
    assert_eq!(resolved.definitions.len(), 2);
}

#[test]
fn a_two_file_non_descending_cycle_reports_spans_in_both_files() {
    let dir = TempDir::new().expect("tempdir");
    let a_text = "kind: schema\ntypes:\n  a: b@./b.yaml\n";
    let b_text = "kind: schema\ntypes:\n  b: a@./a.yaml\n";
    let a = write(dir.path(), "a.yaml", a_text);
    let b = write(dir.path(), "b.yaml", b_text);
    let error = resolve_inline(dir.path(), "p: a@./a.yaml\n").expect_err("two-file alias cycle");
    let SchemaError::NonDescendingCycle { cycles } = &error else {
        panic!("expected NonDescendingCycle, got {error:?}");
    };
    let mut spans: Vec<(SchemaSourceId, String)> = cycles[0]
        .iter()
        .map(|reference| {
            let text = if reference.site.source == SchemaSourceId::File(canonical(&a)) { a_text } else { b_text };
            let span = reference.span.clone().expect("a file reference carries its span");
            (reference.site.source.clone(), text[span].to_string())
        })
        .collect();
    spans.sort_by_key(|(source, _)| source.to_string());
    assert_eq!(
        spans,
        [
            (SchemaSourceId::File(canonical(&a)), "b".to_string()),
            (SchemaSourceId::File(canonical(&b)), "a".to_string()),
        ]
    );
}

#[test]
fn unknown_names_and_long_acyclic_chains_keep_their_errors() {
    let dir = TempDir::new().expect("tempdir");
    let error = resolve_inline(dir.path(), "item: \"{ children: item[], other: missing }\"\n").expect_err("unknown");
    assert!(matches!(error, SchemaError::UnknownNamedType { ref name, .. } if name == "missing"), "{error:?}");

    let mut chain = String::new();
    for index in 0..40 {
        chain.push_str(&format!("t{index}: t{}\n", index + 1));
    }
    chain.push_str("t40: string\n");
    let error = resolve_inline(dir.path(), &chain).expect_err("depth");
    assert!(matches!(&error, SchemaError::ImportCycle { chain } if chain.contains("exceeded")), "{error:?}");
}

// ── Storage, keys, and namespaces ─────────────────────────────────────────

#[test]
fn an_acyclic_alias_into_a_cycle_inlines_to_a_reference() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "node: \"{ kids: node[] }\"\ntree: node -> A tree\nroot: tree(required)\n",
    )
    .expect("alias into a cycle resolves");
    assert_eq!(resolved.definitions.len(), 1, "the alias is not cyclic");
    assert_eq!(
        resolved.json_schema["properties"]["root"],
        json!({"$ref": "#/$defs/node", "description": "A tree"})
    );
    assert!(resolved.json_schema["required"].as_array().unwrap().contains(&json!("root")));
}

#[test]
fn repeated_imports_share_one_definition_and_resolution_is_deterministic() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  node: \"{ kids: node[] }\"\n");
    let schema = "a: node@./types.yaml\nb: node[]@./types.yaml\n";
    let first = resolve_inline(dir.path(), schema).expect("resolves");
    let second = resolve_inline(dir.path(), schema).expect("resolves again");
    assert_eq!(first.definitions.len(), 1);
    let text = serde_json::to_string(&first.json_schema).unwrap();
    assert_eq!(text, serde_json::to_string(&second.json_schema).unwrap());
    let root = dir.path().to_string_lossy().to_string();
    assert!(!text.contains(&root), "generated keys name no path: {text}");
    assert_eq!(first.json_schema["properties"]["b"]["anyOf"][1]["items"], json!({"$ref": "#/$defs/node"}));
}

#[test]
fn same_named_types_from_different_namespaces_get_distinct_keys() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  item: \"{ file_child: item[] }\"\n");
    let resolved = resolve_inline(dir.path(), "item: \"{ local_child: item[] }\"\nother: item@./types.yaml\n")
        .expect("resolves");
    let keys: Vec<(DefNamespace, String)> = resolved
        .definitions
        .iter()
        .map(|(key, definition)| (key.namespace.clone(), definition.json_key.clone()))
        .collect();
    assert_eq!(
        keys,
        [
            (DefNamespace::InlineRoot, "item".to_string()),
            (DefNamespace::File(canonical(&dir.path().join("types.yaml"))), "item-2".to_string()),
        ]
    );
    let defs = &resolved.json_schema["$defs"];
    assert!(defs["item"]["properties"].get("local_child").is_some(), "{defs}");
    assert!(defs["item-2"]["properties"].get("file_child").is_some(), "{defs}");
}

#[test]
fn root_union_arms_are_separate_namespaces_with_definitions_at_the_union_root() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "- kind: literal(a)\n  item: \"{ a_child: item[] }\"\n- kind: literal(b)\n  item: \"{ b_child: item[] }\"\n",
    )
    .expect("root union resolves");
    let defs = resolved.json_schema["$defs"].as_object().expect("$defs at the union root");
    assert_eq!(defs.keys().collect::<Vec<_>>(), ["item", "item-2"]);
    assert!(resolved.json_schema["anyOf"][0].get("$defs").is_none());
    let validator = validator(&resolved);
    assert!(validator.is_valid(&json!({"kind": "b", "item": {"b_child": [{"b_child": []}]}})));
    assert!(!validator.is_valid(&json!({"kind": "b", "item": {"b_child": [{"a_child": []}]}})));
}

#[test]
fn an_acyclic_schema_lowers_with_no_definitions() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(dir.path(), "base: \"{ x: string }\"\np: base[]\n").expect("resolves");
    assert!(resolved.definitions.is_empty());
    assert!(resolved.json_schema.get("$defs").is_none());
}

// ── Use-site semantics ────────────────────────────────────────────────────

/// The lowered schema of top-level property `name`.
fn property<'a>(resolved: &'a ResolvedSchema, name: &str) -> &'a Value {
    &resolved.json_schema["properties"][name]
}

fn definition_body<'a>(resolved: &'a ResolvedSchema, key: &str) -> &'a Value {
    &resolved.json_schema["$defs"][key]
}

#[test]
fn definition_presence_and_annotations_move_to_each_use() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  node: \"{ kids: node[] }(required; default({})) -> A node\"\n  \
         flat: \"{ kids: string[] }(required; default({})) -> A node\"\n  \
         made: \"{ more: made[] }(generated)\"\n  timed: \"{ more: timed[] }(eager)\"\n",
    );
    let resolved = resolve_inline(
        dir.path(),
        "a: node@./types.yaml\nb: node@./types.yaml -> Use-site text\nc: made@./types.yaml\nd: timed@./types.yaml\n\
         inlined: flat@./types.yaml\n",
    )
    .expect("resolves");
    // The use carries the default and description an inlined copy carries.
    let inlined = property(&resolved, "inlined");
    assert_eq!(
        property(&resolved, "a"),
        &json!({"$ref": "#/$defs/node", "default": inlined["default"], "description": "A node"})
    );
    assert_eq!(property(&resolved, "b")["description"], json!("Use-site text"));
    let required: Vec<&Value> = resolved.json_schema["required"].as_array().unwrap().iter().collect();
    assert_eq!(
        required,
        [&json!("a"), &json!("b"), &json!("inlined")],
        "required is inherited, generated is not static"
    );
    assert_eq!(property(&resolved, "c")["x-darkmatter-generated"], json!(true));
    let body = definition_body(&resolved, "node");
    assert!(body.get("description").is_none() && body.get("default").is_none(), "{body}");
    // Inside the body `kids: node[]` inherits `required` like an inlined use.
    assert_eq!(body["required"], json!(["kids"]));

    let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified else { panic!("single shape") };
    let PropertyDef::Single(timed) = &shape.properties["d"] else { panic!("single atom") };
    assert!(matches!(timed.ty, TypeExpr::Ref(_)));
    assert!(timed.constraints.contains(&Constraint::Eager), "{timed:?}");
}

#[test]
fn a_nullable_union_definition_is_never_required() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "link: [null, \"{ next: link(required) }\"]\nroot: link(required)\n",
    )
    .expect("resolves");
    assert!(resolved.json_schema.get("required").is_none(), "{}", resolved.json_schema);
    let validator = validator(&resolved);
    for instance in [json!({}), json!({"root": null}), json!({"root": {"next": {"next": null}}})] {
        assert!(validator.is_valid(&instance), "{instance}");
    }
    assert!(!validator.is_valid(&json!({"root": {"next": 3}})));
}

#[test]
fn two_differently_constrained_uses_share_one_unchanged_body() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "node: \"{ kids: node[] }\"\nwide: node(min-keys(1))\nnarrow: node(max-keys(0))\nfew: node[](min(1); max(2))\n",
    )
    .expect("resolves");
    assert_eq!(property(&resolved, "wide")["anyOf"][1], json!({"$ref": "#/$defs/node", "minProperties": 1}));
    assert_eq!(property(&resolved, "narrow")["anyOf"][1], json!({"$ref": "#/$defs/node", "maxProperties": 0}));
    assert_eq!(
        property(&resolved, "few")["anyOf"][1],
        json!({"type": "array", "items": {"$ref": "#/$defs/node"}, "minItems": 1, "maxItems": 2})
    );
    let body = definition_body(&resolved, "node");
    assert!(body.get("minProperties").is_none() && body.get("maxProperties").is_none(), "{body}");
    let validator = validator(&resolved);
    assert!(!validator.is_valid(&json!({"wide": {}})), "the sibling applies beside the reference");
    assert!(validator.is_valid(&json!({"wide": {"kids": []}, "narrow": {}})));
    assert!(!validator.is_valid(&json!({"few": []})));
}

#[test]
fn a_list_definition_counts_use_site_bounds_as_items() {
    let dir = TempDir::new().expect("tempdir");
    let resolved =
        resolve_inline(dir.path(), "chain: \"{ next: chain }[]\"\nroot: chain(min(2))\n").expect("resolves");
    assert_eq!(property(&resolved, "root")["anyOf"][1], json!({"$ref": "#/$defs/chain", "minItems": 2}));
    let error = resolve_inline(dir.path(), "chain: \"{ next: chain }[]\"\nroot: chain[]\n").expect_err("[] on a list");
    assert!(
        matches!(&error, SchemaError::Convert { message, .. } if message.contains("already-array")),
        "{error:?}"
    );
}

#[test]
fn contradictory_use_site_bounds_name_the_definition_and_the_use() {
    let dir = TempDir::new().expect("tempdir");
    let error = resolve_inline(dir.path(), "node: \"{ kids: node[] }(max-keys(1))\"\nroot: node(min-keys(3))\n")
        .expect_err("contradiction");
    let SchemaError::ConstraintConflict { kind, constraints, .. } = &error else {
        panic!("expected ConstraintConflict, got {error:?}");
    };
    assert_eq!(*kind, SchemaConstraintConflictKind::Bounds);
    let paths: Vec<String> = constraints.iter().map(|origin| origin.path.to_string()).collect();
    assert_eq!(paths, ["node", "root"], "{error:?}");
}

#[test]
fn a_suggestion_on_a_recursive_reference_is_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let error = resolve_inline(dir.path(), "node: \"{ kids: node[] }\"\nroot: node(suggest(a))\n")
        .expect_err("suggest");
    assert!(matches!(&error, SchemaError::Grammar { message, .. } if message.contains("suggest")), "{error:?}");
}

#[test]
fn a_function_reached_only_through_a_definition_fails_at_the_use_site() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  node: \"{ f: function(parameters(void); returns(string)), kids: node[] }\"\n",
    );
    let error = resolve_inline(dir.path(), "root: node@./types.yaml\n").expect_err("function type");
    assert!(
        matches!(&error, SchemaError::Convert { property, message } if property == "root.f" && message.contains("function")),
        "{error:?}"
    );
}

// ── Required children with a finite leaf ──────────────────────────────────

#[test]
fn required_children_accept_an_empty_leaf_and_reject_missing_or_null_at_depth_four() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "item: \"{ value: string(required), children: item[](required) }\"\nroot: item(required)\n\
         optional: item\nlist: item[]\n",
    )
    .expect("resolves");
    let validator = validator(&resolved);
    let leaf = |children: Value| json!({"value": "d", "children": children});
    let depth_four = |inner: Value| {
        json!({"root": {"value": "a", "children": [{"value": "b", "children": [{"value": "c", "children": [inner]}]}]}})
    };
    assert!(validator.is_valid(&depth_four(leaf(json!([])))), "an empty leaf list passes");
    for (case, inner) in [("missing", json!({"value": "d"})), ("null", leaf(Value::Null))] {
        let instance = depth_four(inner);
        let paths: Vec<String> = validator.iter_errors(&instance).map(|e| e.instance_path().to_string()).collect();
        assert!(
            paths.iter().any(|path| path.starts_with("/root/children/0/children/0/children/0")),
            "{case}: {paths:?}"
        );
    }
    assert!(validator.is_valid(&json!({"root": leaf(json!([])), "optional": null})), "optional accepts null");
    assert!(validator.is_valid(&json!({"root": leaf(json!([]))})), "optional accepts absence");
    assert!(!validator.is_valid(&json!({"root": leaf(json!([])), "list": [null]})), "list items are not nullable");
    assert!(!validator.is_valid(&json!({"root": leaf(json!([null]))})), "nested items are not nullable");
}

// ── Input matrix: a referenced `types.item` with a recursive control ──────

/// One edit of the recursive control per row; `Ok` rows name an instance the
/// prepared schema must accept and one it must reject.
#[test]
fn a_referenced_definition_takes_every_declaration_shape_from_a_recursive_control() {
    let dir = TempDir::new().expect("tempdir");
    let types = dir.path().join("types.yaml");
    let schema = "p: item@./types.yaml\n";
    let control = "kind: schema\ntypes:\n  item: \"{ children: item[] }\"\n";

    std::fs::write(&types, control).unwrap();
    let resolved = resolve_inline(dir.path(), schema).expect("control resolves");
    assert_eq!(resolved.definitions.len(), 1, "the control recurses");
    let control_validator = validator(&resolved);
    assert!(control_validator.is_valid(&json!({"p": {"children": [{"children": []}]}})));
    assert!(!control_validator.is_valid(&json!({"p": {"children": [{"children": [1]}]}})));

    for (cell, text, accepts, rejects) in [
        ("explicit null is the null type", "kind: schema\ntypes:\n  item: null\n", json!({"p": null}), json!({"p": {}})),
        ("an empty mapping is an empty object", "kind: schema\ntypes:\n  item: {}\n", json!({"p": {}}), json!({"p": {"x": 1}})),
    ] {
        std::fs::write(&types, text).unwrap();
        let resolved = resolve_inline(dir.path(), schema).unwrap_or_else(|error| panic!("{cell}: {error:?}"));
        assert!(resolved.definitions.is_empty(), "{cell}");
        let cell_validator = validator(&resolved);
        assert!(cell_validator.is_valid(&accepts), "{cell}");
        assert!(!cell_validator.is_valid(&rejects), "{cell}");
    }

    for (cell, text) in [
        ("absent", "kind: schema\ntypes:\n  other: string\n"),
        ("wrong whole type: number", "kind: schema\ntypes:\n  item: 42\n"),
        ("wrong whole type: boolean", "kind: schema\ntypes:\n  item: true\n"),
        ("one wrong union element", "kind: schema\ntypes:\n  item: [\"{ children: item[] }\", 42]\n"),
        ("every union element wrong", "kind: schema\ntypes:\n  item: [42]\n"),
        ("empty union", "kind: schema\ntypes:\n  item: []\n"),
        ("empty expression", "kind: schema\ntypes:\n  item: \"\"\n"),
        ("duplicate definition", "kind: schema\ntypes:\n  item: \"{ children: item[] }\"\n  item: string\n"),
        ("trailing document", "kind: schema\ntypes:\n  item: \"{ children: item[] }\"\n---\nextra: 1\n"),
    ] {
        std::fs::write(&types, text).unwrap();
        let error = resolve_inline(dir.path(), schema).expect_err(cell);
        if cell == "absent" {
            assert!(matches!(error, SchemaError::UnknownNamedType { .. }), "{cell}: {error:?}");
        }
    }
}
