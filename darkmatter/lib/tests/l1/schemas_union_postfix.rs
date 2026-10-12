//! Public resolution and lowering evidence for postfixes on references to
//! union-typed named types: `U[]`, `U[](…)`, `U(required)`, `U -> text`.
//!
//! An array of a union lowers to a container whose `items.anyOf` holds the
//! flattened arms; markers inherited from arms hoist to the property; value
//! constraints stay on their arms; unsupported operations fail with
//! [`SchemaError::UnionReference`].

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, PropertyDef, SchemaConstraintConflictKind, SchemaError,
    SchemaSourceId, SchemaSourcePath, SchemaUnionRejection, SimplifiedSchema, TypeExpr,
};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

use crate::request_support::context_at;
use crate::union_fixtures::{
    CONTENT_POLICY_CASES, CONTENT_POLICY_FILE, CONTENT_POLICY_LIST, FILE_TYPES, MULTI_HOP_FILES,
    content_policy_union_schema, write_files,
};

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// Resolves an inline `$schema` mapping body from a document in `dir`.
fn resolve_inline(dir: &Path, schema: &str) -> Result<ResolvedSchema, SchemaError> {
    let value: YamlValue = serde_yaml_ng::from_str(schema).expect("fixture YAML parses");
    resolve_yaml_schema(&value, dir, &context_at(dir))
}

fn property<'a>(resolved: &'a ResolvedSchema, name: &str) -> &'a Value {
    &resolved.json_schema["properties"][name]
}

fn required(resolved: &ResolvedSchema) -> Vec<&str> {
    resolved.json_schema["required"]
        .as_array()
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// The non-null member of an optional property's `anyOf: [null, T]` wrapper.
fn unwrap_optional(schema: &Value) -> &Value {
    let arms = schema["anyOf"].as_array().expect("optional wrapper");
    assert_eq!(arms[0], json!({ "type": "null" }), "optional wrapper: {schema}");
    assert_eq!(arms.len(), 2, "optional wrapper: {schema}");
    &arms[1]
}

fn simplified_property<'a>(resolved: &'a ResolvedSchema, name: &str) -> &'a PropertyDef {
    let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified else {
        panic!("expected a single-shape schema");
    };
    shape.properties.get(name).expect("property is declared")
}

fn union_error(err: SchemaError) -> (String, SchemaUnionRejection, SchemaSourcePath, SchemaSourcePath) {
    let SchemaError::UnionReference {
        name,
        rejection,
        site,
        definition,
    } = err
    else {
        panic!("expected UnionReference, got {err:?}");
    };
    (name, *rejection, site.path, definition.path)
}

/// Writes a document whose `$schema` mapping body is `schema` and validates
/// it, loading it from disk as `md schema validate` does. Asserts validation
/// leaves the file unchanged and returns the problem paths.
fn validate(dir: &Path, schema: &str, frontmatter: &str) -> Result<Vec<String>, SchemaError> {
    let path = dir.join("doc.md");
    let text = format!("---\n$schema:\n{schema}{frontmatter}---\nBody\n");
    std::fs::write(&path, &text).expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    let report = DarkmatterSchemas::new(context_at(dir)).validate(&markdown);
    assert_eq!(std::fs::read_to_string(&path).expect("read document"), text);
    report.map(|report| {
        assert_eq!(report.valid, report.problems.is_empty());
        report.problems.into_iter().map(|problem| problem.path).collect()
    })
}

// ── AC 1 and AC 3: the Content Policy shapes ──────────────────────────────

#[test]
fn content_policy_list_lowers_to_an_array_of_the_flattened_arms() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_files(&root, &[(CONTENT_POLICY_FILE, &content_policy_union_schema(shipped))]);
    let resolved = resolve_inline(&root, &format!("content_policy: \"{CONTENT_POLICY_LIST}\"\n"))
        .expect("the acceptance fixture resolves");

    assert!(required(&resolved).is_empty(), "content_policy stays optional");
    let array = unwrap_optional(property(&resolved, "content_policy"));
    assert_eq!(array["type"], "array");
    assert_eq!(array["minItems"], 1);
    let arms = array["items"]["anyOf"].as_array().expect("items.anyOf");
    assert_eq!(arms.len(), 9, "short_form's eight arms flatten beside long_form");
    assert!(arms.iter().all(|arm| arm["type"] != "null"), "items get no null arm");
    assert_eq!(arms[0]["pattern"], "^Evergreen$");
    assert_eq!(arms[1]["description"], "Always triggers", "arm descriptions stay per arm");
    let long_form = &arms[8];
    assert_eq!(long_form["type"], "object");
    assert_eq!(long_form["required"], json!(["rule", "action"]));
    assert_eq!(long_form["additionalProperties"], false);
    let rule = long_form["properties"]["rule"]["anyOf"].as_array().expect("rule is a union");
    assert_eq!(rule.len(), 8, "a required union has no null arm");

    let PropertyDef::Single(atom) = simplified_property(&resolved, "content_policy") else {
        panic!("an array of a union is one atom");
    };
    assert!(atom.is_array);
    assert!(matches!(&atom.ty, TypeExpr::Union(arms) if arms.len() == 9));
    assert_eq!(atom.array_constraints, vec![Constraint::MinItems(1)]);
    assert!(atom.constraints.is_empty());
}

#[test]
fn content_policy_list_validates_the_acceptance_cases() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_files(&root, &[(CONTENT_POLICY_FILE, &content_policy_union_schema(shipped))]);
    let schema = format!("  content_policy: \"{CONTENT_POLICY_LIST}\"\n");
    for (frontmatter, expected) in CONTENT_POLICY_CASES {
        let problems = validate(&root, &schema, frontmatter).expect("schema resolves");
        assert_eq!(problems.is_empty(), *expected, "{frontmatter:?}: {problems:?}");
        if !expected {
            assert!(
                problems.iter().all(|path| path.starts_with("/content_policy")),
                "{frontmatter:?}: {problems:?}"
            );
        }
    }
}

// ── AC 2: presence on a union reference ───────────────────────────────────

#[test]
fn required_union_reference_requires_the_property_without_a_null_arm() {
    let (_dir, root) = temp_root();
    let resolved =
        resolve_inline(&root, "choice: [string, number]\np: choice(required)\n").expect("resolves");
    assert_eq!(required(&resolved), ["p"]);
    assert_eq!(
        property(&resolved, "p"),
        &json!({ "anyOf": [{ "type": "string" }, { "type": "number" }] })
    );

    let schema = "  choice: [string, number]\n  p: choice(required)\n";
    assert_eq!(validate(&root, schema, "p: x\n").expect("resolves"), Vec::<String>::new());
    assert_eq!(validate(&root, schema, "p: 3\n").expect("resolves"), Vec::<String>::new());
    assert!(!validate(&root, schema, "").expect("resolves").is_empty(), "missing");
    assert!(!validate(&root, schema, "p: null\n").expect("resolves").is_empty(), "null");
}

#[test]
fn union_reference_without_a_postfix_still_inlines_the_union() {
    let (_dir, root) = temp_root();
    let resolved = resolve_inline(&root, "choice: [string, number]\np: choice\n").expect("resolves");
    assert!(matches!(simplified_property(&resolved, "p"), PropertyDef::Union(arms) if arms.len() == 2));
    assert_eq!(
        property(&resolved, "p"),
        &json!({ "anyOf": [{ "type": "null" }, { "type": "string" }, { "type": "number" }] })
    );
}

#[test]
fn a_described_union_reference_keeps_the_arm_descriptions() {
    let (_dir, root) = temp_root();
    let resolved = resolve_inline(
        &root,
        "choice: [string -> a word, number -> a count]\np: choice -> either one\n",
    )
    .expect("resolves");
    assert_eq!(
        property(&resolved, "p"),
        &json!({
            "anyOf": [
                { "type": "null" },
                { "type": "string", "description": "a word" },
                { "type": "number", "description": "a count" }
            ],
            "description": "either one"
        })
    );

    let list = resolve_inline(
        &root,
        "choice: [string -> a word, number -> a count]\np: choice[] -> some of either\n",
    )
    .expect("resolves");
    let wrapper = property(&list, "p");
    assert_eq!(wrapper["description"], "some of either");
    let array = unwrap_optional(wrapper);
    assert!(array.get("description").is_none(), "the description moves to the wrapper");
    assert_eq!(array["items"]["anyOf"][0]["description"], "a word");
    assert_eq!(array["items"]["anyOf"][1]["description"], "a count");
}

#[test]
fn inherited_markers_hoist_to_the_property_and_arms_lower_as_in_a_property_union() {
    let (_dir, root) = temp_root();
    let resolved = resolve_inline(
        &root,
        "choice: [\"string(required; min(3))\", number]\nlist: choice[](max(2))\nsingle: choice -> one\n",
    )
    .expect("resolves");
    assert_eq!(required(&resolved), ["choice", "list", "single"]);
    let list = property(&resolved, "list");
    assert_eq!(list["type"], "array", "a required array has no null wrapper");
    assert_eq!(list["maxItems"], 2);
    assert!(list.get("minItems").is_none(), "the arm's string min stays on the arm");
    assert_eq!(
        list["items"]["anyOf"],
        property(&resolved, "choice")["anyOf"],
        "an item lowers exactly as the same arm of a property union"
    );
    assert_eq!(list["items"]["anyOf"][0]["minLength"], 3);

    let generated = resolve_inline(
        &root,
        "choice: [\"string(generated)\", number]\nlist: choice[]\nmarked: \"choice(generated; required)\"\n",
    )
    .expect("resolves");
    assert_eq!(unwrap_optional(property(&generated, "list"))["type"], "array");
    assert_eq!(property(&generated, "list")["x-darkmatter-generated"], true);
    assert_eq!(property(&generated, "marked")["x-darkmatter-generated"], true);
    assert!(
        required(&generated).is_empty(),
        "`generated` suppresses the static required entry: {:?}",
        required(&generated)
    );
    assert!(
        property(&generated, "marked")["anyOf"]
            .as_array()
            .expect("union")
            .iter()
            .all(|arm| arm["type"] != "null"),
        "`required` still removes the null arm"
    );
}

#[test]
fn eager_on_the_union_does_not_make_lazy_file_arms_eager() {
    let (_dir, root) = temp_root();
    write_files(&root, &[("files.yaml", FILE_TYPES)]);
    let resolved = resolve_inline(
        &root,
        "lazy: \"lazy_asset[](eager)@./files.yaml\"\nmixed: \"asset[]@./files.yaml\"\n\
         timed: \"lazy_asset(eager)@./files.yaml\"\nproperty: \"lazy_asset(required)@./files.yaml\"\n",
    )
    .expect("resolves");
    let lazy = unwrap_optional(property(&resolved, "lazy"));
    assert_eq!(lazy["items"]["anyOf"], property(&resolved, "property")["anyOf"]);
    assert_eq!(lazy["items"]["anyOf"][0]["format"], "darkmatter-file-reference");
    let mixed = unwrap_optional(property(&resolved, "mixed"));
    assert_eq!(mixed["items"]["anyOf"][0]["format"], "darkmatter-file", "file(eager) keeps its check");
    let timed = property(&resolved, "timed")["anyOf"].as_array().expect("union");
    assert!(timed.iter().all(|arm| arm.get("format") != Some(&json!("darkmatter-file"))));

    let PropertyDef::Single(atom) = simplified_property(&resolved, "lazy") else {
        panic!("one atom");
    };
    assert_eq!(atom.array_constraints, vec![Constraint::Eager], "eager is property timing");
    let arms = atom.union_arms().expect("union");
    assert!(arms[0].constraints.is_empty(), "the lazy arm is untouched");
}

// ── AC 7: reference forms, imports, and nesting ───────────────────────────

#[test]
fn bare_this_and_cross_file_references_lower_identically() {
    let (_dir, root) = temp_root();
    write_files(&root, &[("types.yaml", "kind: schema\ntypes:\n  choice: [string, number]\n")]);
    let lowered = |schema: &str| {
        let resolved = resolve_inline(&root, schema).expect(schema);
        (property(&resolved, "p").clone(), property(&resolved, "q").clone(), required(&resolved).len())
    };
    let bare = lowered("choice: [string, number]\np: choice[](min(1))\nq: choice(required)\n");
    let this = lowered("choice: [string, number]\np: choice[](min(1))@this\nq: choice(required)@this\n");
    let file = lowered(
        "p: \"choice[](min(1))@./types.yaml\"\nq: \"choice(required)@./types.yaml\"\n",
    );
    assert_eq!(bare.0, this.0);
    assert_eq!(bare.0, file.0);
    assert_eq!(bare.1, file.1);
    assert_eq!(bare.2, 1, "only q is required");
    assert_eq!(file.2, 1, "only q is required");
}

#[test]
fn a_forward_reference_resolves_regardless_of_declaration_order() {
    let (_dir, root) = temp_root();
    let forward = resolve_inline(&root, "p: choice[]\nchoice: [string, number]\n").expect("resolves");
    let backward = resolve_inline(&root, "choice: [string, number]\np: choice[]\n").expect("resolves");
    assert_eq!(property(&forward, "p"), property(&backward, "p"));
}

#[test]
fn a_multi_hop_union_keeps_its_defining_scope() {
    let (_dir, root) = temp_root();
    write_files(&root, MULTI_HOP_FILES);
    let resolved = resolve_inline(&root, "p: \"choice[](max(2))@./outer.yaml\"\n").expect("resolves");
    let array = unwrap_optional(property(&resolved, "p"));
    assert_eq!(
        array["items"]["anyOf"],
        json!([{ "type": "string", "pattern": "^[a-z]+$" }, { "type": "number" }])
    );
    assert_eq!(resolved.imports, vec![root.join("middle.yaml"), root.join("outer.yaml")]);

    let schema = "  p: \"choice[](max(2))@./outer.yaml\"\n";
    assert!(validate(&root, schema, "p: [abc, 3]\n").expect("resolves").is_empty());
    assert_eq!(validate(&root, schema, "p: [abc, ABC]\n").expect("resolves"), ["/p/1"]);
    assert_eq!(validate(&root, schema, "p: [a, b, c]\n").expect("resolves"), ["/p"]);
}

#[test]
fn a_bare_nested_union_flattens_and_a_postfixed_one_stays_one_arm() {
    let (_dir, root) = temp_root();
    let resolved = resolve_inline(
        &root,
        "pair: [string, number]\nflat: [pair, boolean]\nnested: [\"pair(required)\", boolean]\n\
         a: flat[]\nb: nested[]\n",
    )
    .expect("resolves");
    assert_eq!(
        unwrap_optional(property(&resolved, "a"))["items"]["anyOf"],
        json!([{ "type": "string" }, { "type": "number" }, { "type": "boolean" }])
    );
    let b = property(&resolved, "b");
    assert_eq!(b["type"], "array", "the nested arm's required hoists through both unions");
    assert_eq!(
        b["items"]["anyOf"],
        json!([{ "anyOf": [{ "type": "string" }, { "type": "number" }] }, { "type": "boolean" }])
    );
}

#[test]
fn an_object_arm_may_contain_an_array() {
    let (_dir, root) = temp_root();
    let schema = "  entry: [\"{ tags: string[] }\", number]\n  p: entry[]\n";
    assert!(validate(&root, schema, "p: [{tags: [a, b]}, 3]\n").expect("resolves").is_empty());
    let problems = validate(&root, schema, "p: [{tags: a}]\n").expect("resolves");
    assert!(!problems.is_empty() && problems.iter().all(|path| path.starts_with("/p/0")), "{problems:?}");
}

#[test]
fn independent_items_select_different_arms_and_null_items_fail() {
    let (_dir, root) = temp_root();
    let schema = "  choice: [string, \"{ n: number(required) }\"]\n  p: choice[]\n";
    assert!(validate(&root, schema, "p: [a, {n: 1}, b]\n").expect("resolves").is_empty());
    assert_eq!(validate(&root, schema, "p: [a, null]\n").expect("resolves"), ["/p/1"]);
    assert_eq!(validate(&root, schema, "p: a\n").expect("resolves"), ["/p"]);
    assert!(validate(&root, schema, "p: null\n").expect("resolves").is_empty());
}

// ── AC 7: container constraints, bounds, uniqueness, defaults ─────────────

#[test]
fn container_constraints_apply_to_the_array_and_value_constraints_stay_on_arms() {
    let (_dir, root) = temp_root();
    let resolved = resolve_inline(
        &root,
        "choice: [\"string(min(3))\", number]\np: \"choice[](min(1); max(2); unique)\"\n",
    )
    .expect("resolves");
    let array = unwrap_optional(property(&resolved, "p"));
    assert_eq!(array["minItems"], 1);
    assert_eq!(array["maxItems"], 2);
    assert_eq!(array["uniqueItems"], true);
    assert_eq!(array["items"]["anyOf"][0], json!({ "type": "string", "minLength": 3 }));

    let schema = "  choice: [\"string(min(3))\", number]\n  p: \"choice[](min(1); max(2); unique)\"\n";
    for (frontmatter, expected) in [
        ("p: [abc]\n", Vec::<&str>::new()),
        ("p: [abc, 4]\n", vec![]),
        ("p: []\n", vec!["/p"]),
        ("p: [abc, 4, 5]\n", vec!["/p"]),
        ("p: [abc, abc]\n", vec!["/p"]),
        ("p: [ab]\n", vec!["/p/0"]),
    ] {
        assert_eq!(validate(&root, schema, frontmatter).expect("resolves"), expected, "{frontmatter}");
    }
}

#[test]
fn contradictory_container_bounds_fail_with_their_origins() {
    let (_dir, root) = temp_root();
    let err = resolve_inline(&root, "choice: [string, number]\np: \"choice[](min(3); max(1))\"\n")
        .expect_err("contradictory bounds");
    let SchemaError::ConstraintConflict {
        name,
        kind,
        site,
        constraints,
    } = err
    else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(name, "choice");
    assert_eq!(kind, SchemaConstraintConflictKind::Bounds);
    assert_eq!(site.path, SchemaSourcePath::root().property("p"));
    assert_eq!(
        constraints.iter().map(|origin| origin.keyword.as_str()).collect::<Vec<_>>(),
        ["min", "max"]
    );
}

#[test]
fn container_defaults_match_plain_arrays_and_arm_defaults_are_rejected() {
    let (_dir, root) = temp_root();
    // The grammar reads a bracketed default as a string, so a use-site
    // default on `U[]` lowers exactly as one on any other array.
    let resolved = resolve_inline(
        &root,
        "choice: [string, number]\np: \"choice[](default([a]))\"\nplain: \"string[](default([a]))\"\n",
    )
    .expect("a use-site default applies to the container");
    assert_eq!(property(&resolved, "p")["default"], property(&resolved, "plain")["default"]);
    let array = unwrap_optional(property(&resolved, "p"));
    assert!(array["items"]["anyOf"].as_array().expect("items").iter().all(|arm| arm.get("default").is_none()));

    for (choice, arm, value) in [
        ("[string, \"number(default(3))\"]", 1, json!(3)),
        ("[\"string(default([a]))\", number]", 0, json!("[a]")),
    ] {
        let schema = format!("choice: {choice}\np: choice[]\n");
        let err = resolve_inline(&root, &schema).expect_err("an arm default cannot default an array");
        let message = err.to_string();
        let (name, rejection, site, definition) = union_error(err);
        assert_eq!(name, "choice");
        assert_eq!(rejection, SchemaUnionRejection::ScalarDefault { arm, value }, "{schema}");
        assert_eq!(site, SchemaSourcePath::root().property("p"));
        assert_eq!(definition, SchemaSourcePath::root().property("choice"));
        assert!(message.starts_with("p: cannot apply `[]`"), "{message}");
    }

    // Without `[]`, arm defaults keep the property-union hoisting rule.
    let single = resolve_inline(&root, "choice: [string, \"number(default(3))\"]\np: choice(eager)\n")
        .expect("resolves");
    assert_eq!(property(&single, "p")["default"], 3);
    assert_eq!(property(&single, "p")["anyOf"][2], json!({ "type": "number" }));
}

// ── AC 2 and AC 7: rejected operations and their provenance ──────────────

#[test]
fn value_constraints_on_a_union_reference_are_rejected_by_name() {
    let (_dir, root) = temp_root();
    for (definition, keyword, array_level) in [
        ("choice(max(10))", "max", false),
        ("choice(min(1))", "min", false),
        ("\"choice(pattern(^a))\"", "pattern", false),
        ("choice(default(x))", "default", false),
        ("\"choice(required; unique)\"", "unique", false),
        ("\"choice[](pattern(^a))\"", "pattern", true),
        ("\"choice[](integer)\"", "integer", true),
    ] {
        let schema = format!("choice: [string, number]\np: {definition}\n");
        let err = resolve_inline(&root, &schema).expect_err(&schema);
        let message = err.to_string();
        let (name, rejection, site, definition_path) = union_error(err);
        assert_eq!(name, "choice");
        assert_eq!(
            rejection,
            SchemaUnionRejection::Constraint {
                keyword: keyword.to_string(),
                array_level
            },
            "{schema}"
        );
        assert_eq!(site, SchemaSourcePath::root().property("p"));
        assert_eq!(definition_path, SchemaSourcePath::root().property("choice"));
        assert!(message.contains(&format!("`{keyword}`")), "{message}");
    }
}

#[test]
fn an_array_arm_rejects_the_array_suffix_even_behind_an_import() {
    let (_dir, root) = temp_root();
    write_files(
        &root,
        &[
            ("lists.yaml", "kind: schema\ntypes:\n  names: string[]\n"),
            ("choices.yaml", "kind: schema\ntypes:\n  choice: [number, \"names@./lists.yaml\"]\n"),
        ],
    );
    for (schema, arm) in [
        ("choice: [\"string[]\", number]\np: choice[]\n", 0),
        ("p: \"choice[]@./choices.yaml\"\n", 1),
        ("pair: [string, number]\nchoice: [boolean, \"pair[]\"]\np: choice[]\n", 1),
        ("pair: [\"string[]\", number]\nchoice: [boolean, \"pair(required)\"]\np: choice[]\n", 1),
    ] {
        let err = resolve_inline(&root, schema).expect_err(schema);
        let (_, rejection, _, _) = union_error(err);
        assert_eq!(rejection, SchemaUnionRejection::ArrayArm { arm }, "{schema}");
    }
    // The same unions stay usable without `[]`.
    resolve_inline(&root, "p: \"choice(required)@./choices.yaml\"\n").expect("no `[]`");
}

#[test]
fn a_cross_file_rejection_names_both_sources() {
    let (_dir, root) = temp_root();
    write_files(&root, &[("types.yaml", "kind: schema\ntypes:\n  choice: [string, number]\n")]);
    let err = resolve_inline(&root, "p: \"choice(max(3))@./types.yaml\"\n").expect_err("rejected");
    let SchemaError::UnionReference { site, definition, .. } = err else {
        panic!("expected UnionReference, got {err:?}");
    };
    assert_eq!(site.source, SchemaSourceId::InlineDocument);
    assert_eq!(definition.source, SchemaSourceId::File(root.join("types.yaml")));
    assert_eq!(definition.path, SchemaSourcePath::root().property("choice"));
}

#[test]
fn cycles_and_depth_limits_still_apply_through_union_postfixes() {
    let (_dir, root) = temp_root();
    let err = resolve_inline(&root, "a: [\"b(required)\", number]\nb: a[]\n").expect_err("cycle");
    assert!(matches!(err, SchemaError::ImportCycle { .. }), "{err:?}");

    let mut chain = String::new();
    for index in 0..40 {
        chain.push_str(&format!("t{index}: [\"t{}(required)\", number]\n", index + 1));
    }
    chain.push_str("t40: [string, number]\n");
    let err = resolve_inline(&root, &chain).expect_err("depth");
    let SchemaError::ImportCycle { chain } = err else {
        panic!("expected ImportCycle, got {err:?}");
    };
    assert!(chain.contains("exceeded"), "{chain}");
}

