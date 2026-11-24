//! Object composition in the authored SimplifiedSchema YAML export.
//!
//! The export writes composition as authored, never as its merged members:
//! `$extends` keeps its scalar or sequence form, an inline object keeps each
//! `...Name` spread at its position among the literal and pattern members,
//! and a selected inline-composed named object exports a root that extends
//! the selection. Each case reparses the export and checks it describes the
//! same composition (equal authored model) and prepares the same JSON Schema,
//! and that a second export is byte-identical.
//!
//! Boundary: the crate's public API in-process (`SimplifiedSchema`,
//! `parse_standalone_schema_document`). F1 for content cases; the `disk`
//! module (`feature = "file"`) reparses imports from real files in a
//! temporary directory with the original resolution context.

use std::path::Path;

use serde_json::{Value, json};
use simplified_schema::{
    SchemaDeclaration, SchemaShape, SimplifiedSchema, SimplifiedSchemaError, parse_standalone_schema_document,
};

/// The authored declaration and `types` table of a standalone document.
fn authored(text: &str) -> (SchemaDeclaration, Option<SchemaShape>) {
    let document = parse_standalone_schema_document(text, Path::new("schema.yaml"))
        .unwrap_or_else(|error| panic!("parses: {error}\n{text}"))
        .expect("a standalone schema document");
    (document.declaration, document.local_types)
}

#[track_caller]
fn content(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("loads: {error}\n{text}"))
}

#[track_caller]
fn export(schema: &SimplifiedSchema) -> String {
    schema.to_simplified_yaml().unwrap_or_else(|error| panic!("exports: {error}"))
}

#[track_caller]
fn json_schema(schema: &SimplifiedSchema) -> Value {
    schema.to_json_schema().unwrap_or_else(|error| panic!("lowers: {error}")).as_ref().clone()
}

/// Exports `text`, reparses the export as content, and checks the authored
/// model, the prepared JSON Schema, and a second export. Returns the export.
#[track_caller]
fn round_trip(text: &str) -> String {
    let schema = content(text);
    let exported = export(&schema);
    assert_eq!(authored(&exported), authored(text), "the export describes the same composition:\n{exported}");
    let reparsed = content(&exported);
    assert_eq!(json_schema(&reparsed), json_schema(&schema), "the export prepares the same schema:\n{exported}");
    assert_eq!(export(&reparsed), exported, "a second export is identical");
    exported
}

/// The keys of a prepared object's `properties`, in order. An optional
/// nested object lowers as `anyOf: [null, object]`; its object arm is read.
fn property_order(schema: &Value) -> Vec<String> {
    let object = match schema.get("anyOf").and_then(Value::as_array) {
        Some(arms) => arms.iter().find(|arm| arm.get("properties").is_some()).expect("an object arm"),
        None => schema,
    };
    object["properties"].as_object().expect("properties").keys().cloned().collect()
}

/// The prepared definition of `name` inside a prepared object.
fn member<'a>(schema: &'a Value, name: &str) -> &'a Value {
    let object = match schema.get("anyOf").and_then(Value::as_array) {
        Some(arms) => arms.iter().find(|arm| arm.get("properties").is_some()).expect("an object arm"),
        None => schema,
    };
    &object["properties"][name]
}

fn unsupported(result: Result<String, SimplifiedSchemaError>) -> String {
    match result {
        Err(SimplifiedSchemaError::ExportUnsupported { reason }) => reason,
        other => panic!("expected ExportUnsupported, got {other:?}"),
    }
}

const INLINE: &str = r#"$schema:
    first: "{ ...base, id: number }"
    middle: "{ a: string, ...base, b: number }"
    last: "{ a: string, ...base }"
    only: "{ ...base, ...extra, ...base, }"
types:
    base:
        id: string(required)
        name: string
    extra: "{ flag: boolean }"
"#;

#[test]
fn composition_export_writes_each_spread_at_its_position() {
    let exported = round_trip(INLINE);
    // Spreads stay inline, at their positions; a repeat stays repeated.
    for line in [
        "first: '{...base,id: number}'",
        "middle: '{a: string,...base,b: number}'",
        "last: '{a: string,...base}'",
        "only: '{...base,...extra,...base}'",
    ] {
        assert!(exported.contains(line), "`{line}` in:\n{exported}");
    }
    // The merged object keeps the target's insertion order, and a later
    // receiver entry replaces the target's `id` in its first position.
    let lowered = json_schema(&content(&exported));
    let first = member(&lowered, "first");
    assert_eq!(property_order(first), ["id", "name"]);
    assert_eq!(member(first, "id")["anyOf"][1]["type"], json!("number"), "{first}");
    assert_eq!(property_order(member(&lowered, "middle")), ["a", "id", "name", "b"]);
    assert_eq!(property_order(&lowered), ["first", "middle", "last", "only"]);
}

#[test]
fn composition_export_keeps_pattern_members_around_spreads() {
    // Literals and patterns between two spreads come out literals first;
    // the reparsed spread counts are those of the original.
    let text = r#"$schema:
    tags: "{ <starting::x->: string, a: string, ...base, <string>: number, b: boolean, ...extra, c: string }"
types:
    base:
        id: string
        <ending::-at>: string
    extra: "{ <pattern::^n[0-9]+$>: number }"
"#;
    let exported = round_trip(text);
    assert!(
        exported.contains("tags: '{a: string,<starting::x->: string,...base,b: boolean,<string>: number,...extra,c: string}'"),
        "{exported}"
    );
    // A pattern-only inline object without a spread still has no inline
    // form where it cannot be a block, as before composition.
    let reason = unsupported(content("$schema:\n    p: \"{ <string>: number }[]\"\n").to_simplified_yaml());
    assert!(reason.contains("pattern keys"), "{reason}");
}

#[test]
fn composition_export_keeps_spreads_in_arrays_unions_and_nested_objects() {
    let text = r#"$schema:
    rows: "{ ...base, n: number }[]"
    either: "string | { ...base }"
    arms:
        - "{ ...base }[]"
        - number
    nested: "{ inner: { ...base, deep: { ...extra } } }"
    block:
        inner: "{ ...extra, z: string }"
        <string>: "{ ...base }"
    marked: "{ ...base }(min-keys(1)) -> a described object"
types:
    base: "{ id: string }"
    extra: "{ flag: boolean }"
"#;
    let exported = round_trip(text);
    for line in [
        "rows: '{...base,n: number}[]'",
        "either:\n        - string\n        - '{...base}'\n",
        "- '{...base}[]'",
        "nested:\n        inner: '{...base,deep: {...extra}}'\n",
        "inner: '{...extra,z: string}'",
        "<string>: '{...base}'",
        "marked: '{...base}(min-keys(1)) -> a described object'",
    ] {
        assert!(exported.contains(line), "`{line}` in:\n{exported}");
    }
}

#[test]
fn composition_export_keeps_explicit_local_references() {
    // `string@this` names a local type called `string`; a bare `string`
    // would be the built-in keyword, so the suffix must survive.
    let text = r#"$schema:
    inline: "{ ...string@this, ...base@this, ...base }"
    $extends: [string@this, base]
types:
    string: "{ text: string }"
    base: "{ id: number }"
"#;
    let exported = round_trip(text);
    assert!(exported.contains("inline: '{...string@this,...base@this,...base}'"), "{exported}");
    assert!(exported.starts_with("$schema:\n    $extends:\n        - string@this\n        - base\n"), "{exported}");
}

#[test]
fn composition_export_writes_extends_before_the_mapping_members() {
    // The directive is placed last in the source; the export writes it first,
    // and placement does not change the composition.
    let text = r#"$schema:
    total: number
    nested:
        own: string
        $extends: base
    union:
        - string
        - $extends: [base, extra]
          tag: string
    $extends: base
types:
    base:
        id: string(required)
    extra: "{ flag: boolean }"
"#;
    let exported = round_trip(text);
    assert!(exported.starts_with("$schema:\n    $extends: base\n    total: number\n"), "{exported}");
    assert!(exported.contains("nested:\n        $extends: base\n        own: string\n"), "{exported}");
    // A mapping union arm stays a mapping, so its `$extends` survives.
    assert!(
        exported.contains("        - $extends:\n              - base\n              - extra\n          tag: string\n"),
        "{exported}"
    );
    let schema = content(&exported);
    assert!(schema.validate_strict(&json!({ "id": "a", "total": 1, "union": { "id": "b", "tag": "t" } })).is_ok());
    assert!(schema.validate_strict(&json!({ "total": 1 })).is_err(), "the inherited `id` stays required");
}

#[test]
fn composition_export_keeps_root_union_arm_directives() {
    let text = r#"$schema:
    - $extends: base
      kind: literal(a; required)
    - kind: literal(b; required)
      $extends: [extra]
types:
    base: "{ id: string }"
    extra: "{ flag: boolean }"
"#;
    let exported = round_trip(text);
    assert!(exported.starts_with("$schema:\n    - $extends: base\n      kind: literal(a;required)\n"), "{exported}");
    assert!(exported.contains("    - $extends:\n          - extra\n      kind: literal(b;required)\n"), "{exported}");
}

#[test]
fn composition_export_writes_decoded_operands_for_anchors_aliases_and_folds() {
    let text = "$schema:\n    one:\n        $extends: &operand base\n    two:\n        $extends: *operand\n    three:\n        $extends: >-\n            base\n    five: &spread \"{ ...base, n: number }\"\n    six: *spread\ntypes:\n    base: \"{ id: string }\"\n";
    let exported = round_trip(text);
    assert_eq!(exported.matches("$extends: base\n").count(), 3, "{exported}");
    assert!(!exported.contains('&') && !exported.contains('*'), "{exported}");
    assert_eq!(exported.matches("'{...base,n: number}'").count(), 2, "{exported}");
}

/// A `Layer::select` of an inline-composed named object validates as the
/// composed object and exports a root that extends the selection, beside
/// the authored table that holds its inline expression.
#[test]
fn composition_export_of_a_selected_inline_composition_extends_the_selection() {
    let text = r#"$schema:
    item: branch
types:
    base: "{ id: string(required), children: branch[] }"
    branch: "{ ...base, label: string(required) }"
    via-extends:
        $extends: base
        extra: number
"#;
    let catalog = content(text);
    let branch = catalog.select("branch").expect("an object type");
    let valid = json!({ "id": "a", "label": "root", "children": [{ "id": "b", "label": "leaf" }] });
    assert!(branch.validate_strict(&valid).is_ok());
    assert!(branch.validate_strict(&json!({ "id": "a" })).is_err(), "the receiver's `label` is required");
    assert!(
        branch.validate_strict(&json!({ "id": "a", "label": "x", "children": [{ "label": "leaf" }] })).is_err(),
        "the recursive child keeps the inherited required `id`"
    );
    let exported = export(&branch);
    assert!(exported.starts_with("$schema:\n    $extends: branch@this\ntypes:\n"), "{exported}");
    assert!(exported.contains("branch: '{...base,label: string(required)}'"), "{exported}");
    let reparsed = content(&exported);
    assert_eq!(json_schema(&reparsed), json_schema(&branch), "{exported}");
    assert!(reparsed.validate_strict(&valid).is_ok());
    assert!(reparsed.validate_strict(&json!({ "id": "a" })).is_err());
    assert_eq!(export(&reparsed), exported);

    // A mapping-composed selection exports its own `$extends` mapping.
    let selected = catalog.select("via-extends").expect("an object type");
    let exported = export(&selected);
    assert!(exported.starts_with("$schema:\n    $extends: base\n    extra: number\ntypes:\n"), "{exported}");
    assert_eq!(json_schema(&content(&exported)), json_schema(&selected));

    // The source schema still exports its named declaration as authored.
    let exported = round_trip(text);
    assert!(exported.contains("branch: '{...base,label: string(required)}'"), "{exported}");
}

#[test]
fn composition_export_of_a_selection_from_a_composed_root_omits_the_roots_directive() {
    // Without a `types` table the root's literals are the named types; the
    // root's own `$extends` is not a declaration and is not written into the
    // exported `types` table.
    let text = r#"$schema:
    $extends: base
    base: "{ a: string }"
    item: "{ ...base, n: number }"
"#;
    let item = content(text).select("item").expect("an object type");
    let exported = export(&item);
    assert!(!exported.contains("types:\n    $extends"), "{exported}");
    assert!(exported.starts_with("$schema:\n    $extends: item@this\ntypes:\n"), "{exported}");
    let reparsed = content(&exported);
    assert_eq!(json_schema(&reparsed), json_schema(&item));
    assert!(reparsed.validate_strict(&json!({ "a": "x", "n": 1 })).is_ok());
    assert!(reparsed.validate_strict(&json!({ "n": "not a number" })).is_err());
}

#[test]
fn composition_export_of_a_selection_whose_name_is_not_an_operand_is_unsupported() {
    // `type.v2` is a valid `types` key but not a named reference, so no
    // `$extends` operand can name it.
    let text = "$schema:\n    x: string\ntypes:\n    base: \"{ id: string }\"\n    type.v2: \"{ ...base, n: number }\"\n";
    let selected = content(text).select("type.v2").expect("an object type");
    assert!(selected.validate_strict(&json!({ "id": "a", "n": 1 })).is_ok());
    let reason = unsupported(selected.to_simplified_yaml());
    assert!(reason.contains("`type.v2`"), "{reason}");
}

#[test]
fn composition_export_refuses_layers_it_cannot_flatten() {
    let composed_root = "$schema:\n    $extends: base\n    n: number\ntypes:\n    base: \"{ id: string }\"\n";
    let plain = "$schema:\n    owner: string\n";
    // A composed root in either layer.
    for (top, lower) in [(composed_root, plain), (plain, composed_root)] {
        let mut schema = content(top);
        schema.layer_under(&[&content(lower)]).expect("layers");
        let reason = unsupported(schema.to_simplified_yaml());
        assert!(reason.contains("root composes"), "{reason}");
        // The layered schema still validates as one.
        assert!(schema.validate_strict(&json!({ "id": "a", "n": 1, "owner": "b" })).is_ok());
    }
    // A lower layer's property that composes is not copied into the top
    // layer's document.
    let lower = "$schema:\n    copied: \"{ ...base }\"\ntypes:\n    base: \"{ id: string }\"\n";
    let mut schema = content(plain);
    schema.layer_under(&[&content(lower)]).expect("layers");
    let reason = unsupported(schema.to_simplified_yaml());
    assert!(reason.contains("`copied`"), "{reason}");
    // A nested composition in the top layer is retained.
    let top = "$schema:\n    kept: \"{ ...base, n: number }\"\ntypes:\n    base: \"{ id: string }\"\n";
    let mut schema = content(top);
    schema.layer_under(&[&content(plain)]).expect("layers");
    let exported = export(&schema);
    assert!(exported.contains("kept: '{...base,n: number}'") && exported.contains("owner: string"), "{exported}");
    assert_eq!(json_schema(&content(&exported)), json_schema(&schema));
}

#[test]
fn composition_export_leaves_uncomposed_objects_unchanged() {
    // A spread-free inline object is still written as a block mapping, and a
    // pattern member in a block keeps its key.
    let text = "$schema:\n    p: \"{ a: string, b: { c: number } }\"\n    q:\n        <string>: number\n";
    let exported = export(&content(text));
    assert_eq!(
        exported,
        "$schema:\n    p:\n        a: string\n        b:\n            c: number\n    q:\n        <string>: number\n"
    );
    assert_eq!(json_schema(&content(&exported)), json_schema(&content(text)));
}

#[cfg(feature = "file")]
mod disk {
    use std::path::{Path, PathBuf};

    use biscuit_file::FileResolutionContext;
    use serde_json::json;
    use simplified_schema::{SchemaSource, SimplifiedSchema};

    use super::{authored, export, json_schema};

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let file = dir.join(name);
        std::fs::create_dir_all(file.parent().expect("a parent")).unwrap();
        std::fs::write(&file, text).unwrap();
        file
    }

    fn source(dir: &Path, file: &Path) -> SchemaSource {
        SchemaSource::from(file).bound(FileResolutionContext::new(dir))
    }

    /// Imported operands, including path punctuation the reference
    /// delimiters keep, reparse from the receiver's own path and context and
    /// resolve to the same files.
    #[test]
    fn composition_export_reparses_imported_operands_in_the_same_context() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "events.yaml", "kind: schema\ntypes:\n    events:\n        at: string(required)\n");
        write(dir.path(), "dir (x)/a[1].yaml", "kind: schema\ntypes:\n    audit:\n        by: string\n");
        let receiver = write(
            dir.path(),
            "receiver.yaml",
            "$schema:\n    total: number\n    $extends:\n        - events@./events.yaml\n        - audit@./dir (x)/a[1].yaml\n    inline: \"{ ...audit@./dir (x)/a[1].yaml, n: number }\"\n",
        );
        let schema = SimplifiedSchema::try_from_yaml(source(dir.path(), &receiver)).expect("loads");
        let exported = export(&schema);
        assert!(
            exported.starts_with("$schema:\n    $extends:\n        - events@./events.yaml\n        - audit@./dir (x)/a[1].yaml\n"),
            "{exported}"
        );
        assert!(exported.contains("inline: '{...audit@./dir (x)/a[1].yaml,n: number}'"), "{exported}");
        let original = std::fs::read_to_string(&receiver).unwrap();
        assert_eq!(authored(&exported), authored(&original));
        let reparsed =
            SimplifiedSchema::try_from_yaml(source(dir.path(), &receiver).with_text(exported.clone())).expect("reparses");
        assert_eq!(json_schema(&reparsed), json_schema(&schema));
        let value = json!({ "at": "now", "by": "me", "total": 1, "inline": { "by": "x", "n": 2 } });
        assert!(reparsed.validate_strict(&value).is_ok());
        assert!(reparsed.validate_strict(&json!({ "total": 1 })).is_err(), "the imported `at` stays required");
        let imports = |schema: &SimplifiedSchema| {
            let mut imports: Vec<PathBuf> =
                schema.layer_provenance().flat_map(|provenance| provenance.imports().to_vec()).collect();
            imports.sort();
            imports
        };
        assert_eq!(imports(&reparsed), imports(&schema));
        assert_eq!(imports(&schema).len(), 2, "{:?}", imports(&schema));
        assert_eq!(export(&reparsed), exported);
    }
}
