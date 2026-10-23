//! Conversion through a union item (`U[]`, lowered to
//! `items: {anyOf: [arm…]}`): each item takes the conversion engine's union
//! rule on its own (an exact match is kept, else the one reachable option
//! converts), `boolish`/`numberlike` arms keep an accepted item exactly as
//! written, and a value that fails anywhere commits nothing.
//!
//! Every conversion test asserts at least one item that must change, so a
//! converter that skips `items.anyOf` (or applies one arm to every item)
//! fails it.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeSource};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::coerce::coerce_frontmatter;
use serde_json::{Value, json};

use crate::request_support::{context_at, request};
use crate::union_fixtures::{NESTED_UNION_FILE, NESTED_UNION_TYPES, write_files};

/// Unions whose arms coerce: a `boolish` arm beside an object arm, a
/// `numberlike` arm beside an object arm, and an ambiguous `numberlike | string`.
const COERCE_TYPES: &str = "kind: schema\ntypes:\n  toggle:\n    - boolish\n    - \"{ name: string(required), enabled: boolean }\"\n  amount:\n    - numberlike\n    - \"{ unit: string(required), size: number }\"\n  either: [numberlike, string]\n";

/// The literal-tagged union with a coercible field in each arm.
const STEP_TYPES: &str = "kind: schema\ntypes:\n  step:\n    - \"{ kind: literal(wait; required), seconds: number(required) }\"\n    - \"{ kind: literal(say; required), loud: boolean }\"\n";

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    write_files(&root, &[("coerce.yaml", COERCE_TYPES), ("step.yaml", STEP_TYPES)]);
    (dir, root)
}

/// Writes `doc.md` declaring `key: <type_expr>` with `key: <value_yaml>` and
/// returns the loaded document.
fn document(root: &Path, key: &str, type_expr: &str, value_yaml: &str) -> (PathBuf, Markdown) {
    let path = root.join("doc.md");
    std::fs::write(
        &path,
        format!("---\n$schema:\n  {key}: \"{type_expr}\"\n{key}: {value_yaml}\n---\nBody\n"),
    )
    .expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    (path, markdown)
}

/// The value of `key` after coercion against the document's effective schema.
fn coerced(root: &Path, key: &str, type_expr: &str, value_yaml: &str) -> Value {
    let (_, markdown) = document(root, key, type_expr, value_yaml);
    let effective = DarkmatterSchemas::new(context_at(root))
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("document declares a schema");
    let value = markdown
        .frontmatter()
        .get::<Value>(key)
        .expect("frontmatter reads")
        .expect("key present");
    let instance = json!({ key: value });
    coerce_frontmatter(&effective.json_schema, &instance).value[key].clone()
}

#[test]
fn boolish_and_numberlike_arms_keep_items_as_written() {
    let (_dir, root) = fixture();

    assert_eq!(
        coerced(&root, "toggles", "toggle[]@./coerce.yaml", "[\"TRUE\", \"false\"]"),
        json!(["TRUE", "false"]),
    );
    assert_eq!(
        coerced(&root, "amounts", "amount[]@./coerce.yaml", "[\"42\", \"-0.5\"]"),
        json!(["42", "-0.5"]),
    );
    // The object arm beside them still converts its own fields.
    assert_eq!(
        coerced(&root, "toggles", "toggle[](required)@./coerce.yaml", "[\"True\", {name: x, enabled: \"yes\"}]"),
        json!(["True", { "name": "x", "enabled": true }]),
    );
}

#[test]
fn composition_writes_back_a_converted_union_item() {
    let (_dir, root) = fixture();
    let (path, markdown) =
        document(&root, "toggles", "toggle[]@./coerce.yaml", "[\"TRUE\", {name: x, enabled: \"false\"}]");

    let (composed, _) = markdown
        .compose_with(&request(ComposeOptions::new().with_source_file(&path)))
        .expect("composition succeeds");

    let toggles = composed
        .frontmatter()
        .get::<Value>("toggles")
        .expect("frontmatter reads")
        .expect("key present");
    assert_eq!(toggles, json!(["TRUE", { "name": "x", "enabled": false }]));
}

#[test]
fn an_item_no_arm_accepts_leaves_the_whole_list_unchanged() {
    let (_dir, root) = fixture();

    // `maybe` is not boolish and not an object; `[1]` is neither too.
    assert_eq!(
        coerced(&root, "toggles", "toggle[]@./coerce.yaml", "[maybe, {name: x, enabled: \"true\"}, [1]]"),
        json!(["maybe", { "name": "x", "enabled": "true" }, [1]]),
    );
    // An object item missing the required `name` fails the object arm even
    // after its `enabled` would convert, so it is not partially rewritten.
    assert_eq!(
        coerced(&root, "toggles", "toggle[]@./coerce.yaml", "[{enabled: \"true\"}, \"false\"]"),
        json!([{ "enabled": "true" }, "false"]),
    );
}

#[test]
fn an_item_several_arms_accept_is_left_unchanged_beside_one_that_coerces() {
    let (_dir, root) = fixture();

    // `"42"` validates as a string and, coerced, as a number: two arms, so it
    // stays. `true` only fits the string arm once stringified.
    assert_eq!(
        coerced(&root, "values", "either[]@./coerce.yaml", "[\"42\", true]"),
        json!(["42", "true"]),
    );
}

#[test]
fn object_arm_candidates_coerce_their_nested_fields() {
    let (_dir, root) = fixture();

    assert_eq!(
        coerced(
            &root,
            "amounts",
            "amount[]@./coerce.yaml",
            "[{unit: kg, size: \"3\"}, {unit: g, size: 7}]",
        ),
        json!([{ "unit": "kg", "size": 3 }, { "unit": "g", "size": 7 }]),
    );
}

#[test]
fn independent_items_select_different_arms() {
    let (_dir, root) = fixture();

    assert_eq!(
        coerced(
            &root,
            "steps",
            "step[]@./step.yaml",
            "[{kind: wait, seconds: \"5\"}, {kind: say, loud: \"true\"}, {kind: wait, seconds: \"1.5\"}]",
        ),
        json!([
            { "kind": "wait", "seconds": 5 },
            { "kind": "say", "loud": true },
            { "kind": "wait", "seconds": 1.5 },
        ]),
    );
    // Mixed scalar and object arms of one union in one array: the
    // `numberlike` item is kept as written, the object item converts.
    assert_eq!(
        coerced(&root, "amounts", "amount[]@./coerce.yaml", "[\"8\", {unit: m, size: \"2\"}]"),
        json!(["8", { "unit": "m", "size": 2 }]),
    );
}

#[test]
fn a_union_item_converts_exactly_as_the_same_union_property() {
    let (_dir, root) = fixture();
    let cases: &[(&str, &str, Value)] = &[
        ("toggle", "\"TRUE\"", json!("TRUE")),
        ("toggle", "maybe", json!("maybe")),
        ("toggle", "{name: x, enabled: \"false\"}", json!({ "name": "x", "enabled": false })),
        ("amount", "\"42\"", json!("42")),
        ("amount", "{unit: kg, size: \"3\"}", json!({ "unit": "kg", "size": 3 })),
        ("either", "\"42\"", json!("42")),
        ("either", "true", json!("true")),
        ("step", "{kind: say, loud: \"false\"}", json!({ "kind": "say", "loud": false })),
    ];

    for (name, value, expected) in cases {
        let file = if *name == "step" { "step.yaml" } else { "coerce.yaml" };
        let as_property = coerced(&root, "v", &format!("{name}@./{file}"), value);
        let as_item = coerced(&root, "v", &format!("{name}[]@./{file}"), &format!("[{value}]"));
        assert_eq!(&as_property, expected, "property `{name}` with {value}");
        assert_eq!(as_item, json!([as_property]), "item of `{name}[]` with {value}");
    }
}

#[test]
fn a_postfixed_nested_union_converts_like_its_direct_control_as_property_and_item() {
    let (_dir, root) = fixture();
    write_files(&root, &[(NESTED_UNION_FILE, NESTED_UNION_TYPES)]);

    // `toggle` is `boolish | number`: `"TRUE"` matches `boolish` exactly and
    // is kept as written; `"7"` matches `boolish` nowhere and converts to `7`.
    for (key, type_expr, value, expected) in [
        ("p", "direct@./types.yaml", "\"TRUE\"", json!("TRUE")),
        ("p", "nested@./types.yaml", "\"TRUE\"", json!("TRUE")),
        ("p", "direct[]@./types.yaml", "[\"TRUE\"]", json!(["TRUE"])),
        ("p", "nested[]@./types.yaml", "[\"TRUE\"]", json!(["TRUE"])),
        ("p", "direct@./types.yaml", "\"7\"", json!(7)),
        ("p", "nested[]@./types.yaml", "[\"7\"]", json!([7])),
    ] {
        assert_eq!(coerced(&root, key, type_expr, value), expected, "{type_expr}");
    }
}
