//! Object composition: the authored `...Name` spread and YAML `$extends`
//! directive, as the passive parsers read them.
//!
//! The first half covers syntax, the authored model
//! (`SchemaShape::composition`), the reserved `$extends` key in every
//! schema-object position, the located errors of the directive's field
//! shapes, the document shapes of the two loaders that carry it, and the
//! source spans of each operand. The second half covers resolution: which
//! targets compose, how members replace one another, recursion and cycles,
//! the origin each winning member keeps, and that a composed object lowers,
//! validates, and compares like its handwritten equivalent.
//!
//! Boundary: the crate's public API in-process (`SimplifiedSchema`, the
//! resolver and lowering functions, trigger registries and bundle
//! preparation). F1 for content and embedded cases; the cases gated on
//! `feature = "file"` read real files in a temporary directory.

use std::sync::Arc;

use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::{
    CompositionOperand, CompositionTargetKind, Extends, InlineSpread, ObjectComposition, PropertyDef, SchemaArm,
    SchemaError, SchemaRoot, SchemaShape, SchemaSourceId, SchemaSourcePath, SchemaSpanKind, SimplifiedSchema,
    SimplifiedSchemaError, TypeExpr, ValidationFailure, locate_standalone_definition_failure,
    parse_schema_declaration_with_source, parse_property_definition, parse_standalone_schema_document,
    parse_yaml_schema,
};

/// A definition string, parsed as a schema declaration parses it.
fn parse_type_definition(property: &str, text: &str) -> Result<PropertyDef, SchemaError> {
    parse_property_definition(property, &YamlValue::String(text.into()))
}

fn yaml(text: &str) -> YamlValue {
    serde_yaml_ng::from_str(text).expect("the fixture is YAML")
}

fn operand(name: &str, reference: Option<&str>) -> CompositionOperand {
    CompositionOperand { name: name.into(), reference: reference.map(str::to_string) }
}

fn spread(literals_before: usize, patterns_before: usize, operand: CompositionOperand) -> InlineSpread {
    InlineSpread { literals_before, patterns_before, operand }
}

/// The inline object a definition string parses to.
#[track_caller]
fn inline(text: &str) -> SchemaShape {
    match parse_type_definition("p", text).unwrap_or_else(|error| panic!("`{text}`: {error}")) {
        PropertyDef::Single(atom) => match atom.ty {
            TypeExpr::InlineObject(shape) => shape,
            other => panic!("`{text}` is not an inline object: {other:?}"),
        },
        other => panic!("`{text}` is a union: {other:?}"),
    }
}

/// The grammar error message and the text its span covers.
#[track_caller]
fn grammar_error(text: &str) -> (String, String) {
    match parse_type_definition("p", text) {
        Err(SchemaError::Grammar { message, span, .. }) => (message, text[span].to_string()),
        other => panic!("`{text}` should be a grammar error, got {other:?}"),
    }
}

fn spreads(shape: &SchemaShape) -> &[InlineSpread] {
    match shape.composition.as_deref() {
        Some(ObjectComposition::Inline(spreads)) => spreads,
        other => panic!("expected inline spreads, got {other:?}"),
    }
}

fn root_shape(text: &str) -> SchemaShape {
    match parse_yaml_schema(&yaml(text)).unwrap_or_else(|error| panic!("{error}")) {
        SchemaRoot::Single(shape) => shape,
        other => panic!("expected one object, got {other:?}"),
    }
}

// ── Inline spread syntax ────────────────────────────────────────────────

#[test]
fn composition_spread_records_its_position_among_literal_and_pattern_members() {
    let shape = inline("{ ...a, x: string, <string>: number, ...b@./b.yaml, y: number, ...c }");
    assert_eq!(
        spreads(&shape),
        [
            spread(0, 0, operand("a", None)),
            spread(1, 1, operand("b", Some("./b.yaml"))),
            spread(2, 1, operand("c", None)),
        ]
    );
    assert_eq!(shape.properties.keys().collect::<Vec<_>>(), ["x", "y"]);
    assert_eq!(shape.pattern_keys.len(), 1);
}

#[test]
fn composition_spread_may_be_the_whole_body_repeat_and_end_with_a_comma() {
    assert_eq!(spreads(&inline("{ ...a }")), [spread(0, 0, operand("a", None))]);
    assert_eq!(
        spreads(&inline("{ ...a, ...a, }")),
        [spread(0, 0, operand("a", None)), spread(0, 0, operand("a", None))]
    );
    let shape = inline("{ x: string, ...a }");
    assert_eq!(spreads(&shape), [spread(1, 0, operand("a", None))]);
}

#[test]
fn composition_spread_whitespace_does_not_change_the_shape() {
    assert_eq!(inline("{...a,x:string}"), inline("{ ...  a ,\n  x: string }"));
}

#[test]
fn composition_absent_is_omitted_from_the_shape_and_its_record() {
    let shape = inline("{ a: string }");
    assert_eq!(shape.composition, None);
    assert_eq!(inline("{}").composition, None);
    let record = serde_json::to_value(&shape).expect("serializes");
    assert!(record.get("composition").is_none(), "{record}");
    let composed = serde_json::to_value(inline("{ ...a }")).expect("serializes");
    let back: SchemaShape = serde_json::from_value(composed).expect("round-trips");
    assert_eq!(back, inline("{ ...a }"));
}

#[test]
fn composition_keyword_collision_needs_an_explicit_local_reference() {
    assert_eq!(spreads(&inline("{ ...string@this }")), [spread(0, 0, operand("string", Some("this")))]);
    let (message, at) = grammar_error("{ ...string }");
    assert_eq!(at, "string");
    assert!(message.contains("built-in `string`") && message.contains("`...string@this`"), "{message}");
}

#[test]
fn composition_spread_rejects_every_postfix_and_an_empty_operand() {
    for (text, at, needle) in [
        ("{ ...a(required) }", "(", "no constraint list"),
        ("{ ...a[] }", "[", "no constraint list or `[]`"),
        ("{ ...a -> label }", "->", "no description"),
        ("{ ...{{ shape }} }", "{{ shape }}", "template cannot be composed"),
        ("{ ...a{{ x }} }", "a{{ x }}", "slot"),
        ("{ ..., a: string }", ",", "expected a named object type after `...`"),
        ("{ ... }", "}", "expected a named object type after `...`"),
        ("{ ...@./a.yaml }", "@", "expected a named object type"),
        ("{ ...a@ }", "", "expected a file reference after `@`"),
        ("{ ...a | b }", "|", "expected `,` or `}`"),
    ] {
        let (message, span) = grammar_error(text);
        assert!(message.contains(needle), "`{text}`: {message}");
        assert_eq!(span, at, "`{text}`: {message}");
    }
}

#[test]
fn composition_receiving_postfix_stays_on_the_containing_atom() {
    let PropertyDef::Single(listed) = parse_type_definition("p", "{ ...a }[](min(1))").expect("parses") else {
        panic!("one atom");
    };
    assert!(listed.is_array);
    assert_eq!(listed.array_constraints.len(), 1);
    let PropertyDef::Single(atom) = parse_type_definition("p", "{ ...a }(required) -> rows").expect("parses") else {
        panic!("one atom");
    };
    assert_eq!(atom.description.as_deref(), Some("rows"));
    assert!(!atom.constraints.is_empty(), "`required` applies to the receiving property");
    let TypeExpr::InlineObject(shape) = atom.ty else { panic!("inline object") };
    assert_eq!(spreads(&shape), [spread(0, 0, operand("a", None))]);
}

#[test]
fn composition_keeps_the_duplicate_literal_error_across_a_spread() {
    let (message, at) = grammar_error("{ id: string, ...events, id: number }");
    assert!(message.contains("duplicate property `id`"), "{message}");
    assert_eq!(at, "id");
}

#[test]
fn composition_import_path_punctuation_follows_the_reference_delimiters() {
    assert_eq!(
        spreads(&inline("{ ...a@./dir (x)/a[1].yaml, b: string }")),
        [spread(0, 0, operand("a", Some("./dir (x)/a[1].yaml")))]
    );
    // Inside a group a reference also ends at `)`.
    let PropertyDef::Union(arms) = parse_type_definition("p", "({ ...a@./x.yaml } | string)").expect("parses") else {
        panic!("a union");
    };
    let TypeExpr::InlineObject(shape) = &arms[0].ty else { panic!("inline object") };
    assert_eq!(spreads(shape), [spread(0, 0, operand("a", Some("./x.yaml")))]);
}

#[test]
fn composition_spreads_parse_in_arrays_unions_and_nested_objects() {
    let PropertyDef::Union(arms) = parse_type_definition("p", "{ ...a }[] | { b: { ...c, d: string } }").expect("parses")
    else {
        panic!("a union");
    };
    let TypeExpr::InlineObject(first) = &arms[0].ty else { panic!("inline object") };
    assert!(arms[0].is_array);
    assert_eq!(spreads(first), [spread(0, 0, operand("a", None))]);
    let TypeExpr::InlineObject(second) = &arms[1].ty else { panic!("inline object") };
    assert_eq!(second.composition, None);
    let PropertyDef::Single(b) = &second.properties["b"] else { panic!("one atom") };
    let TypeExpr::InlineObject(nested) = &b.ty else { panic!("inline object") };
    assert_eq!(spreads(nested), [spread(0, 0, operand("c", None))]);
}

// ── YAML `$extends` ─────────────────────────────────────────────────────

fn extends(shape: &SchemaShape) -> &Extends {
    match shape.composition.as_deref() {
        Some(ObjectComposition::Extends(extends)) => extends,
        other => panic!("expected `$extends`, got {other:?}"),
    }
}

#[test]
fn composition_extends_takes_a_scalar_a_flow_list_or_a_block_list() {
    let shape = root_shape("$extends: events\nsource: string\n");
    assert_eq!(extends(&shape), &Extends::Scalar(operand("events", None)));
    assert_eq!(shape.properties.keys().collect::<Vec<_>>(), ["source"]);

    let flow = root_shape("$extends: [events, 'audit@./audit.yaml', events]\n");
    assert_eq!(
        extends(&flow),
        &Extends::Sequence(vec![operand("events", None), operand("audit", Some("./audit.yaml")), operand("events", None)])
    );
    let block = root_shape("$extends:\n  - events\n  - audit@./audit.yaml\n  - events\n");
    assert_eq!(block, flow);
}

#[test]
fn composition_extends_placement_and_key_quoting_do_not_matter() {
    let first = root_shape("$extends: events\na: string\nb: number\n");
    assert_eq!(root_shape("a: string\nb: number\n$extends: events\n"), first);
    assert_eq!(root_shape("a: string\n\"$extends\": events\nb: number\n"), first);
    assert_eq!(root_shape("a: string\n'$extends': events\nb: number\n"), first);
}

#[test]
fn composition_extends_is_reserved_in_every_schema_object_position() {
    let source = "\
kind: schema
$schema:
  $extends: base
  nested:
    $extends: [base]
    x: string
  choice:
    - string
    - $extends: base
types:
  base: '{ id: string }'
  order:
    $extends: base
    total: number
";
    let document = parse_standalone_schema_document(source, "composition.yaml").expect("parses").expect("a schema");
    let simplified_schema::SchemaDeclaration::Schema(SchemaRoot::Single(root)) = &document.declaration else {
        panic!("one object");
    };
    assert_eq!(extends(root), &Extends::Scalar(operand("base", None)));
    let nested = nested_shape(&root.properties["nested"]);
    assert_eq!(extends(nested), &Extends::Sequence(vec![operand("base", None)]));
    assert!(!nested.properties.contains_key("$extends"));
    let PropertyDef::Union(arms) = &root.properties["choice"] else { panic!("a union") };
    let TypeExpr::InlineObject(arm) = &arms[1].ty else { panic!("a mapping arm") };
    assert_eq!(extends(arm), &Extends::Scalar(operand("base", None)));
    let types = document.local_types.as_ref().expect("a types table");
    assert_eq!(types.composition, None, "a `types` table is not an object");
    assert_eq!(extends(nested_shape(&types.properties["order"])), &Extends::Scalar(operand("base", None)));

    let union = parse_yaml_schema(&yaml("- $extends: base\n  a: string\n- b: string\n")).expect("parses");
    let SchemaRoot::Union(arms) = union else { panic!("a root union") };
    let SchemaArm::Inline(first) = &arms[0] else { panic!("inline arm") };
    assert_eq!(extends(first), &Extends::Scalar(operand("base", None)));
}

fn nested_shape(definition: &PropertyDef) -> &SchemaShape {
    match definition {
        PropertyDef::Single(atom) => match &atom.ty {
            TypeExpr::InlineObject(shape) => shape,
            other => panic!("not an object: {other:?}"),
        },
        other => panic!("a union: {other:?}"),
    }
}

#[test]
fn composition_extends_in_a_types_table_is_an_ordinary_type_name() {
    for source in [
        "kind: schema\ntypes:\n  $extends: other\n  other: number\n",
        "kind: schema\n$schema:\n  a: string\ntypes:\n  $extends: other\n  other: number\n",
    ] {
        let document = parse_standalone_schema_document(source, "types.yaml").expect("parses").expect("a schema");
        let types = document.named_types().expect("a types table");
        assert_eq!(types.composition, None, "{source}");
        let PropertyDef::Single(atom) = &types.properties["$extends"] else { panic!("{source}: one atom") };
        assert_eq!(atom.ty, TypeExpr::local("other"), "{source}");
    }
}

/// The `$extends` declaration of a schema built through the public factory,
/// after checking it lowers.
#[track_caller]
fn extends_declaration(source: &str) -> PropertyDef {
    let schema = SimplifiedSchema::try_from(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    schema.to_json_schema().unwrap_or_else(|error| panic!("{source} lowers: {error}"));
    schema.named_type("$extends").unwrap_or_else(|| panic!("{source}: no `$extends` type")).def().clone()
}

/// Envelope admission judges a `types` table as the table parser reads it,
/// so a type named `$extends` loads whatever its definition is, in every
/// envelope that carries a table.
#[test]
fn composition_extends_in_a_types_table_loads_through_every_envelope() {
    let string = parse_type_definition("$extends", "string").unwrap();
    for source in [
        "kind: schema\ntypes:\n  $extends: string\n",
        "kind: schema\n$schema:\n  a: string\ntypes:\n  $extends: string\n",
        "$schema:\n  a: string\ntypes:\n  $extends: string\n",
        "kind: yaml-schema\n$schema:\n  kind: literal(roster; required)\ntypes:\n  $extends: string\n",
    ] {
        assert_eq!(extends_declaration(source), string, "{source}");
    }

    let object = extends_declaration("kind: schema\ntypes:\n  $extends:\n    a: string\n");
    let shape = nested_shape(&object);
    assert_eq!(shape.composition, None);
    assert_eq!(shape.properties.keys().collect::<Vec<_>>(), ["a"]);

    let union = extends_declaration("kind: schema\ntypes:\n  $extends: [string, number]\n");
    assert_eq!(union, parse_type_definition("$extends", "string | number").unwrap());

    let alias = extends_declaration("kind: schema\ntypes:\n  $extends: other\n  other: string\n");
    assert_eq!(alias, parse_type_definition("$extends", "other").unwrap(), "control: an alias of `other`");
}

/// Only the table itself is read as a table: `$extends` stays a directive in
/// the exported root, a nested object, a mapping union arm, and a mapping
/// entry of the table.
#[test]
fn composition_extends_stays_a_directive_beside_a_types_table() {
    for source in [
        "kind: schema\n$schema:\n  $extends: string\ntypes:\n  a: string\n",
        "kind: schema\n$schema:\n  nested:\n    $extends: string\ntypes:\n  a: string\n",
        "kind: schema\n$schema:\n  choice:\n    - number\n    - $extends: string\ntypes:\n  a: string\n",
        "kind: schema\ntypes:\n  order:\n    $extends: string\n",
    ] {
        let error = SimplifiedSchema::try_from(source).err().unwrap_or_else(|| panic!("{source} must be refused"));
        assert!(error.to_string().contains("`$extends` is reserved for object composition"), "{source}: {error}");
    }
}

/// The envelopes that carry a `types` table, with `ENTRY` standing for the
/// table's first entry.
const TABLE_ENVELOPES: [&str; 4] = [
    "kind: schema\ntypes:\n  ENTRY\n  b: string\n",
    "kind: schema\n$schema:\n  a: string\ntypes:\n  ENTRY\n  b: string\n",
    "$schema:\n  a: string\ntypes:\n  ENTRY\n  b: string\n",
    "kind: yaml-schema\n$schema:\n  kind: literal(roster; required)\ntypes:\n  ENTRY\n  b: string\n",
];

/// A `types` table holds named declarations only: object metadata and
/// pattern keys written directly in it are refused with a located error in
/// every envelope, never applied to a root or silently dropped.
#[test]
fn composition_types_table_refuses_object_only_keys_in_every_envelope() {
    for (entry, key, value, message) in [
        ("$constraints:\n    min-keys: 5", "$constraints", "min-keys: 5", "`$constraints` belongs inside a declaration's object"),
        ("'<string>': number", "<string>", "number", "pattern key `<string>` belongs inside a declaration's object"),
    ] {
        for envelope in TABLE_ENVELOPES {
            let source = envelope.replace("ENTRY", entry);
            let error = SimplifiedSchema::try_from(source.as_str())
                .err()
                .unwrap_or_else(|| panic!("{source} must be refused"));
            assert!(error.to_string().contains(message), "{source}: {error}");
            if source.starts_with("kind:") {
                let (located, path, text) = located(&source);
                assert!(located.contains("a `types` table holds named declarations"), "{source}: {located}");
                assert_eq!(path, SchemaSourcePath::root().property("types").property(key), "{source}");
                assert_eq!(text, value, "{source}");
            }
        }
    }
}

/// Inside a declaration's object the same keys keep their object meaning.
#[test]
fn composition_types_table_entries_keep_object_only_keys() {
    let source = "kind: schema\n$schema:\n  m: map\ntypes:\n  map:\n    '<string>': number\n    $constraints:\n      max-keys: 1\n";
    let schema = SimplifiedSchema::try_from(source).unwrap_or_else(|error| panic!("{error}"));
    assert!(schema.validate_strict(&json!({ "m": { "x": 1 } })).is_ok());
    assert!(schema.validate_strict(&json!({ "m": { "x": 1, "y": 2 } })).is_err(), "max-keys applies");
    assert!(schema.validate_strict(&json!({ "m": { "x": "a" } })).is_err(), "the pattern applies");
}

/// A `schema(mapping)` value is judged as a `types` table is.
#[test]
fn composition_schema_mapping_value_refuses_object_only_keys() {
    let schema = SimplifiedSchema::try_from("$schema:\n  x: schema(mapping)\n").unwrap();
    for table in [json!({ "$constraints": { "min-keys": 1 }, "b": "string" }), json!({ "<string>": "number" })] {
        let refused = schema.validate_strict(&json!({ "x": table })).expect_err("refused");
        assert!(format!("{refused:?}").contains("a `types` table holds named declarations"), "{refused:?}");
    }
    let entry = json!({ "x": { "map": { "<string>": "number", "$constraints": { "max-keys": 1 } } } });
    assert!(schema.validate_strict(&entry).is_ok(), "an entry's object keeps them");
    let object = SimplifiedSchema::try_from("$schema:\n  x: schema\n").unwrap();
    assert!(object.validate_strict(&json!({ "x": { "<string>": "number" } })).is_ok(), "control: a schema object");
}

/// A valid standalone schema: one edit per robustness cell replaces the
/// `$extends` value.
const FIELD: &str = "kind: schema\n$schema:\n  $extends: base\n  total: number\ntypes:\n  base: '{ id: string }'\n";

fn with_extends(value: &str) -> String {
    let edited = FIELD.replacen("$extends: base\n", &format!("$extends:{value}\n"), 1);
    assert_ne!(edited, FIELD, "the edit applies");
    edited
}

/// The located failure of a standalone document: the message, the failure
/// path, and the authored text the error points at.
#[track_caller]
fn located(source: &str) -> (String, SchemaSourcePath, String) {
    let error = parse_standalone_schema_document(source, "field.yaml").expect_err("the document is rejected");
    let located = locate_standalone_definition_failure(source).expect("a located definition failure");
    let span = located.authored_span.clone().or(located.value_span.clone()).expect("a source range");
    assert!(error.to_string().contains(&located.error.to_string()), "{error} / {}", located.error);
    (located.error.to_string(), located.path, source[span].to_string())
}

fn at_extends() -> SchemaSourcePath {
    SchemaSourcePath::root().property("$schema").property("$extends")
}

#[test]
fn composition_extends_field_control_parses() {
    let document = parse_standalone_schema_document(FIELD, "field.yaml").expect("parses").expect("a schema");
    let simplified_schema::SchemaDeclaration::Schema(SchemaRoot::Single(root)) = &document.declaration else {
        panic!("one object");
    };
    assert_eq!(extends(root), &Extends::Scalar(operand("base", None)));
    assert!(locate_standalone_definition_failure(FIELD).is_none());
}

#[test]
fn composition_extends_rejects_a_wrong_carrier_at_the_value() {
    for (value, needle, at) in [
        (" null", "got null", "null"),
        (" ~", "got null", "~"),
        (" 42", "got number", "42"),
        (" true", "got boolean", "true"),
        (" { a: b }", "got mapping", "{ a: b }"),
        (" []", "at least one named object type", "[]"),
        (" ''", "expected a named object type after `$extends`", "''"),
    ] {
        let (message, path, text) = located(&with_extends(value));
        assert!(message.contains(needle), "`{value}`: {message}");
        assert_eq!(path, at_extends(), "`{value}`");
        assert_eq!(text, at, "`{value}`");
    }
}

#[test]
fn composition_extends_implicit_null_is_located_where_its_value_would_be() {
    let source = with_extends("");
    let (message, path, text) = located(&source);
    assert!(message.contains("got null"), "{message}");
    assert_eq!(path, at_extends());
    // An implicit null has no text: its range is empty, just past the key,
    // unlike the explicit `null` and `~` spellings.
    assert_eq!(text, "");
    let located = locate_standalone_definition_failure(&source).expect("located");
    let span = located.authored_span.or(located.value_span).expect("a range");
    let key_end = source.find("$extends:").expect("the key") + "$extends:".len();
    assert!((key_end..=key_end + 1).contains(&span.start), "{span:?} after {key_end}");
}

#[test]
fn composition_extends_rejects_a_wrong_item_at_the_item() {
    for (value, index, needle, at) in [
        (" [base, 3]", 1, "item [1] must be a named object type string, got number", "3"),
        (" [1, 2]", 0, "item [0] must be a named object type string, got number", "1"),
        (" [base, null]", 1, "got null", "null"),
        (" [base, [a]]", 1, "got sequence", "[a]"),
    ] {
        let (message, path, text) = located(&with_extends(value));
        assert!(message.contains(needle), "`{value}`: {message}");
        assert_eq!(path, at_extends().union_arm(index), "`{value}`");
        assert_eq!(text, at, "`{value}`");
    }
}

#[test]
fn composition_extends_rejects_an_invalid_operand_at_the_operand() {
    for (value, path, needle, at) in [
        (" base(required)", at_extends(), "no constraint list", "("),
        (" 'a, b'", at_extends(), "write several operands as a YAML sequence", ", b"),
        (" [base, 'b[]']", at_extends().union_arm(1), "no constraint list or `[]`", "["),
        (" string", at_extends(), "`$extends` is reserved for object composition", "string"),
    ] {
        let (message, actual, text) = located(&with_extends(value));
        assert!(message.contains(needle), "`{value}`: {message}");
        assert_eq!(actual, path, "`{value}`");
        assert_eq!(text, at, "`{value}`");
    }
    let (message, ..) = located(&with_extends(" string"));
    assert!(message.contains("named object type") && message.contains("`string@this`"), "{message}");
}

#[test]
fn composition_extends_rejects_invalid_fields_in_every_carrier_route() {
    for (source, path) in [
        (
            "kind: schema\n$schema:\n  nested:\n    $extends: 42\n",
            SchemaSourcePath::root().property("$schema").property("nested").property("$extends"),
        ),
        (
            "kind: schema\ntypes:\n  order:\n    $extends: 42\n",
            SchemaSourcePath::root().property("types").property("order").property("$extends"),
        ),
        (
            "kind: schema\n$schema:\n  choice:\n    - string\n    - $extends: 42\n",
            SchemaSourcePath::root().property("$schema").property("choice").union_arm(1).property("$extends"),
        ),
        (
            "kind: schema\n$schema:\n  - a: string\n  - $extends: 42\n",
            SchemaSourcePath::root().property("$schema").union_arm(1).property("$extends"),
        ),
    ] {
        let (message, actual, text) = located(source);
        assert!(message.contains("got number"), "{source}: {message}");
        assert_eq!(actual, path, "{source}");
        assert_eq!(text, "42", "{source}");
    }
}

#[cfg(feature = "file")]
#[test]
fn composition_extends_in_markdown_frontmatter_keeps_the_grammar_error() {
    use simplified_schema::{SchemaSource, SimplifiedSchema};
    let text = "---\n$schema:\n  $extends: [base, 3]\n  a: string\n---\n# Body\n";
    let error = SimplifiedSchema::try_from_markdown(SchemaSource::from("doc.md").with_text(text))
        .expect_err("the frontmatter schema is rejected");
    let message = format!("{error}");
    assert!(message.contains("`$extends` item [1] must be a named object type string, got number"), "{message}");
}

// ── Loader document shapes ──────────────────────────────────────────────

#[test]
fn composition_standalone_loader_rejects_duplicate_trailing_and_multiple_documents() {
    // `located` says whether the YAML loader reports a position; it reports
    // none for a second document.
    for (source, needle, located) in [
        ("kind: schema\n$schema:\n  $extends: a\n  $extends: b\n", "duplicate", true),
        ("kind: schema\n$schema:\n  $extends: a\n  '$extends': b\n", "duplicate", true),
        ("kind: schema\n$schema:\n  $extends: a\n  x: string\n: : bad\n", "", true),
        ("kind: schema\n$schema:\n  $extends: a\n---\nkind: schema\n", "more than one document", false),
    ] {
        let error = parse_standalone_schema_document(source, "doc.yaml").expect_err("rejected");
        assert!(error.to_string().contains(needle), "{source}: {error}");
        let SchemaError::SchemaDocument { span, .. } = &error else {
            panic!("{source}: a document error, got {error:?}");
        };
        // The YAML loader's position survives as a span or in the message.
        let has_location = span.is_some() || error.to_string().contains(" line ");
        assert_eq!(has_location, located, "{source}: {error}");
    }
}

#[test]
fn composition_trigger_envelope_loader_rejects_duplicate_trailing_and_multiple_documents() {
    use simplified_schema::triggers::parse_trigger_envelope_from_str;
    const HEAD: &str = "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n";
    let control = format!("{HEAD}  $extends: common@./shared.yaml\n  total: number\n");
    let envelope = parse_trigger_envelope_from_str(&control).expect("parses").expect("claimed");
    let payload = envelope.payload.expect("a payload");
    let SchemaRoot::Single(shape) = parse_yaml_schema(&payload).expect("the payload parses") else {
        panic!("one object");
    };
    assert_eq!(extends(&shape), &Extends::Scalar(operand("common", Some("./shared.yaml"))));

    for (tail, needle, located) in [
        ("  $extends: a\n  $extends: b\n", "duplicate", true),
        ("  $extends: a\n: : bad\n", "", true),
        ("  $extends: a\n---\nkind: schema\n", "more than one document", false),
    ] {
        let source = format!("{HEAD}{tail}");
        let error = parse_trigger_envelope_from_str(&source).expect_err("rejected");
        assert!(matches!(error, SchemaError::TriggerEnvelopeYaml { .. }), "{source}: {error:?}");
        assert!(error.to_string().contains(needle), "{source}: {error}");
        assert_eq!(error.to_string().contains(" line "), located, "{source}: {error}");
    }
}

// ── Operand spans ───────────────────────────────────────────────────────

/// The source map of a `$schema` value, and its text.
fn source_map(source: &str) -> simplified_schema::SchemaSourceMap {
    parse_schema_declaration_with_source(&yaml(source), source, 0)
        .unwrap_or_else(|failure| panic!("{}", failure.error))
        .source_map
}

#[track_caller]
fn texts<'a>(
    source: &'a str,
    map: &simplified_schema::SchemaSourceMap,
    path: &SchemaSourcePath,
    kind: SchemaSpanKind,
) -> Vec<&'a str> {
    map.spans(path, kind).iter().map(|span| &source[span.clone()]).collect()
}

#[test]
fn composition_spread_spans_distinguish_repeated_operands() {
    let source = "snapshot: \"{ ...events, price: number, ...audit@./a b.yaml, ...events }\"\n";
    let map = source_map(source);
    let snapshot = SchemaSourcePath::root().property("snapshot");
    let first = snapshot.spread(0);
    let second = snapshot.spread(1);
    let third = snapshot.spread(2);
    assert_eq!(texts(source, &map, &first, SchemaSpanKind::CompositionOperand), ["events"]);
    assert_eq!(texts(source, &map, &first, SchemaSpanKind::ImportName), ["events"]);
    assert!(texts(source, &map, &first, SchemaSpanKind::ImportReference).is_empty());
    assert_eq!(texts(source, &map, &second, SchemaSpanKind::CompositionOperand), ["audit@./a b.yaml"]);
    assert_eq!(texts(source, &map, &second, SchemaSpanKind::ImportName), ["audit"]);
    assert_eq!(texts(source, &map, &second, SchemaSpanKind::ImportReference), ["./a b.yaml"]);
    assert_eq!(texts(source, &map, &third, SchemaSpanKind::CompositionOperand), ["events"]);
    let (one, three) = (map.spans(&first, SchemaSpanKind::CompositionOperand), map.spans(&third, SchemaSpanKind::CompositionOperand));
    assert!(one[0].start < three[0].start, "each repeat has its own span");
    assert_eq!(texts(source, &map, &snapshot.property("price"), SchemaSpanKind::MappingKey), ["price"]);
}

#[test]
fn composition_spread_spans_cross_yaml_folding_and_escapes() {
    let source = "snapshot: >-\n  { ...events,\n    price: number,\n    ...audit@this }\nquoted: '{ ...it''s@./x.yaml }'\n";
    // `it's` is not an identifier: the folded case is the positive control,
    // the quoted one a rejected operand located through the quoting.
    let value = yaml(source);
    let failure = parse_schema_declaration_with_source(&value, source, 0).expect_err("`it's` is not a name");
    let span = failure.authored_span.expect("located");
    assert_eq!(&source[span], "''", "the decoded quote is located at its escape: {}", failure.error);

    let source = "snapshot: >-\n  { ...events,\n    price: number,\n    ...audit@this }\nescaped: \"{ ...a@./\\u0078.yaml }\"\n";
    let map = source_map(source);
    let snapshot = SchemaSourcePath::root().property("snapshot");
    assert_eq!(texts(source, &map, &snapshot.spread(0), SchemaSpanKind::CompositionOperand), ["events"]);
    assert_eq!(texts(source, &map, &snapshot.spread(1), SchemaSpanKind::CompositionOperand), ["audit@this"]);
    assert_eq!(texts(source, &map, &snapshot.spread(1), SchemaSpanKind::ImportReference), ["this"]);
    let escaped = SchemaSourcePath::root().property("escaped").spread(0);
    assert_eq!(texts(source, &map, &escaped, SchemaSpanKind::ImportReference), ["./\\u0078.yaml"]);
}

#[test]
fn composition_extends_spans_cover_scalars_and_each_list_item() {
    let source = "\
\"$extends\": 'events@./e.yaml'
nested:
  $extends: [base, \"audit@./a.yaml\", base]
block:
  $extends:
    - base
    - other@this
";
    let map = source_map(source);
    let root = SchemaSourcePath::root().property("$extends");
    assert_eq!(texts(source, &map, &root, SchemaSpanKind::MappingKey), ["\"$extends\""]);
    assert_eq!(texts(source, &map, &root, SchemaSpanKind::CompositionOperand), ["events@./e.yaml"]);
    assert_eq!(texts(source, &map, &root, SchemaSpanKind::ImportName), ["events"]);
    assert_eq!(texts(source, &map, &root, SchemaSpanKind::ImportReference), ["./e.yaml"]);

    let nested = SchemaSourcePath::root().property("nested").property("$extends");
    assert_eq!(texts(source, &map, &nested, SchemaSpanKind::Definition), ["[base, \"audit@./a.yaml\", base]"]);
    assert_eq!(texts(source, &map, &nested.union_arm(0), SchemaSpanKind::CompositionOperand), ["base"]);
    assert_eq!(texts(source, &map, &nested.union_arm(1), SchemaSpanKind::CompositionOperand), ["audit@./a.yaml"]);
    assert_eq!(texts(source, &map, &nested.union_arm(1), SchemaSpanKind::UnionArm), ["\"audit@./a.yaml\""]);
    assert_eq!(texts(source, &map, &nested.union_arm(2), SchemaSpanKind::CompositionOperand), ["base"]);
    let (first, last) = (
        map.spans(&nested.union_arm(0), SchemaSpanKind::CompositionOperand)[0].clone(),
        map.spans(&nested.union_arm(2), SchemaSpanKind::CompositionOperand)[0].clone(),
    );
    assert!(first.start < last.start, "each repeated item has its own span");

    let block = SchemaSourcePath::root().property("block").property("$extends");
    assert_eq!(texts(source, &map, &block.union_arm(0), SchemaSpanKind::CompositionOperand), ["base"]);
    assert_eq!(texts(source, &map, &block.union_arm(1), SchemaSpanKind::ImportName), ["other"]);
    assert_eq!(texts(source, &map, &block.union_arm(1), SchemaSpanKind::ImportReference), ["this"]);
}

#[test]
fn composition_extends_spans_in_root_union_arms_follow_the_arm() {
    let source = "- $extends: base\n  a: string\n- $extends: [base, other]\n";
    let map = source_map(source);
    let first = SchemaSourcePath::root().union_arm(0).property("$extends");
    let second = SchemaSourcePath::root().union_arm(1).property("$extends");
    assert_eq!(texts(source, &map, &first, SchemaSpanKind::CompositionOperand), ["base"]);
    assert_eq!(texts(source, &map, &second.union_arm(1), SchemaSpanKind::CompositionOperand), ["other"]);
    assert!(map.spans(&second, SchemaSpanKind::CompositionOperand).is_empty(), "a list records its items");
}

#[test]
fn composition_malformed_operands_are_located_in_the_authored_text() {
    for (source, at, needle) in [
        ("snapshot: \"{ a: string, ...b[] }\"\n", "[", "no constraint list"),
        ("nested:\n  $extends: [base, 'b(required)']\n", "(", "no constraint list"),
        ("nested:\n  $extends: [base, 7]\n", "7", "got number"),
        ("nested:\n  $extends: {}\n", "{}", "got mapping"),
    ] {
        let failure = parse_schema_declaration_with_source(&yaml(source), source, 0).expect_err("rejected");
        let span = failure.authored_span.clone().or(failure.value_span.clone()).expect("located");
        assert_eq!(&source[span], at, "{source}: {}", failure.error);
        assert!(failure.error.to_string().contains(needle), "{source}: {}", failure.error);
    }
}

#[test]
fn composition_path_renders_spreads_distinctly() {
    let path = SchemaSourcePath::root().property("snapshot").spread(1);
    assert_eq!(path.to_string(), "snapshot[...1]");
    assert_ne!(path, SchemaSourcePath::root().property("snapshot").union_arm(1));
}

// ── Resolution ────────────────────────────────────────────────────────────

/// Content that constructs.
#[track_caller]
fn content(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("`{text}` constructs: {error}"))
}

/// The JSON Schema `text` lowers to.
#[track_caller]
fn lowered(text: &str) -> Value {
    let schema = content(text);
    let json = schema.to_json_schema().unwrap_or_else(|error| panic!("`{text}` lowers: {error}"));
    (*json).clone()
}

/// The resolution error constructing `text` fails with.
#[track_caller]
fn resolution_error(text: &str) -> SchemaError {
    match SimplifiedSchema::try_from(text) {
        Err(SimplifiedSchemaError::Schema(error)) => error,
        Err(other) => panic!("`{text}`: expected a schema error, got {other:?}"),
        Ok(_) => panic!("`{text}` was expected to fail"),
    }
}

/// The lowering error `text` constructs with.
#[track_caller]
fn lowering_error(text: &str) -> Arc<SchemaError> {
    match content(text).to_json_schema() {
        Err(SimplifiedSchemaError::Unprepared(error)) => error,
        other => panic!("`{text}`: expected an unpreparable schema, got {other:?}"),
    }
}

/// The instance paths and properties strict validation reports for `value`.
#[track_caller]
fn strict_problems(schema: &SimplifiedSchema, value: &Value) -> Vec<String> {
    match schema.validate_strict(value) {
        Err(ValidationFailure::InvalidData { problems }) => problems
            .into_iter()
            .map(|problem| format!("{}/{}", problem.path, problem.property.unwrap_or_default()))
            .collect(),
        other => panic!("expected invalid data for {value}, got {other:?}"),
    }
}

fn path(segments: &[&str]) -> SchemaSourcePath {
    segments.iter().fold(SchemaSourcePath::root(), |path, segment| path.property(*segment))
}

/// A composed schema and the handwritten schema it must equal.
#[track_caller]
fn assert_same_lowering(composed: &str, handwritten: &str) {
    let (composed_json, handwritten_json) = (lowered(composed), lowered(handwritten));
    assert_eq!(composed_json, handwritten_json, "composed:\n{composed}\nhandwritten:\n{handwritten}");
    // Equal maps can still differ in property order, which output keeps.
    let order = |json: &Value| json["properties"].as_object().map(|map| map.keys().cloned().collect::<Vec<_>>());
    assert_eq!(order(&composed_json), order(&handwritten_json), "property order");
}

const EVENTS: &str = "types:\n  events: \"{ id: string(required), price: number }\"\n";

fn with_events(schema: &str) -> String {
    format!("kind: schema\n$schema:\n{schema}{EVENTS}")
}

#[test]
fn composition_the_spec_example_behaves_like_its_handwritten_equivalent() {
    let composed = with_events(
        "  $extends: events\n  source: string\n  snapshot: \"{ ...events, price: number(min(0)) }\"\n",
    );
    let handwritten = with_events(
        "  id: string(required)\n  price: number\n  source: string\n  snapshot: \"{ id: string(required), price: number(min(0)) }\"\n",
    );
    assert_same_lowering(&composed, &handwritten);

    let (composed, handwritten) = (content(&composed), content(&handwritten));
    for value in [json!({}), json!({ "id": 1 }), json!({ "id": "a", "snapshot": { "price": -1 } })] {
        assert_eq!(strict_problems(&composed, &value), strict_problems(&handwritten, &value), "{value}");
    }
    assert!(!strict_problems(&composed, &json!({})).is_empty(), "the inherited `id` is required");
    let valid = json!({ "id": "a", "price": 3, "snapshot": { "id": "b", "price": 0 } });
    assert!(composed.validate_strict(&valid).is_ok());
    assert!(composed.validate_soft(&valid).is_ok());

    let id = composed.property("id").expect("the resolved root has the copied `id`");
    assert!(id.is_required());
    assert_eq!(id.type_text(), handwritten.property("id").expect("handwritten id").type_text());
    assert!(composed.property("events").is_none(), "the `types` table is not the root");
}

#[test]
fn composition_spread_applies_in_place_and_later_entries_replace_earlier_ones() {
    let cases = [
        // A spread after a literal replaces it, at the literal's position.
        ("{ id: number, ...events }", "{ id: string(required), price: number }"),
        // A literal after a spread replaces the spread's member.
        ("{ ...events, id: number }", "{ id: number, price: number }"),
        ("{ note: string, ...events, extra: boolean }", "{ note: string, id: string(required), price: number, extra: boolean }"),
        ("{ ...events }", "{ id: string(required), price: number }"),
        ("{ ...events, }", "{ id: string(required), price: number }"),
        ("{ ...events, ...events }", "{ id: string(required), price: number }"),
        // A replaced key keeps its first position.
        ("{ price: string, ...empty, ...events }", "{ price: number, id: string(required) }"),
    ];
    for (composed, handwritten) in cases {
        let schema = |object: &str| format!("kind: schema\n$schema:\n  value: \"{object}\"\n{EVENTS}  empty: \"{{}}\"\n");
        assert_same_lowering(&schema(composed), &schema(handwritten));
    }
}

#[test]
fn composition_replacing_a_member_replaces_its_whole_declaration() {
    let schema = content(
        "kind: schema\n$schema:\n  value: \"{ ...base, id: number }\"\ntypes:\n  base: \"{ id: string(required; default('x')) -> the base id }\"\n",
    );
    let value = schema.query("value.id").expect("path").expect("the replaced member");
    assert!(!value.is_required(), "the replacement drops the inherited `required`");
    assert_eq!(value.description(), None, "and the inherited description");
    assert_eq!(value.type_text(), "number");
    let input = json!({ "value": {} });
    let soft = schema.validate_soft(&input).expect("an optional member");
    assert!(soft.defaults.is_empty(), "and the inherited default: {:?}", soft.defaults);
}

#[test]
fn composition_extends_applies_its_operands_in_order_before_the_mapping() {
    let types = "types:\n  a: \"{ x: string, y: string }\"\n  b: \"{ y: number, z: number }\"\n";
    let handwritten = format!("kind: schema\n$schema:\n  x: string\n  y: number\n  z: number\n  w: boolean\n{types}");
    for placement in [
        "  $extends: [a, b]\n  w: boolean\n",
        "  w: boolean\n  $extends: [a, b]\n",
        "  w: boolean\n  \"$extends\":\n    - a\n    - b\n",
    ] {
        assert_same_lowering(&format!("kind: schema\n$schema:\n{placement}{types}"), &handwritten);
    }
    // The mapping's own members replace what an operand supplied.
    let own = format!("kind: schema\n$schema:\n  $extends: [a, b]\n  y: boolean\n{types}");
    let expected = format!("kind: schema\n$schema:\n  x: string\n  y: boolean\n  z: number\n{types}");
    assert_same_lowering(&own, &expected);
}

#[test]
fn composition_extends_resolves_in_every_schema_object_position() {
    let types = "types:\n  base: \"{ id: string(required) }\"\n";
    let cases = [
        ("  nested:\n    $extends: base\n    n: number\n", "  nested:\n    id: string(required)\n    n: number\n"),
        (
            "  choice:\n    - string\n    - $extends: base\n      n: number\n",
            "  choice:\n    - string\n    - id: string(required)\n      n: number\n",
        ),
        ("  value: framed\n", "  value: framed\n"),
    ];
    for (composed, handwritten) in cases {
        let framed_composed = "  framed:\n    $extends: base\n    n: number\n";
        let framed_handwritten = "  framed:\n    id: string(required)\n    n: number\n";
        assert_same_lowering(
            &format!("kind: schema\n$schema:\n{composed}{types}{framed_composed}"),
            &format!("kind: schema\n$schema:\n{handwritten}{types}{framed_handwritten}"),
        );
    }
    // A root-union mapping arm, with the explicit `types` table.
    let union = |arm: &str| format!("kind: schema\n$schema:\n  - {arm}\n  - other: number\n{types}");
    assert_eq!(lowered(&union("$extends: base")), lowered(&union("id: string(required)")));
}

#[test]
fn composition_root_union_arms_keep_their_own_local_names() {
    // Without a `types` table each inline arm's members are its local names.
    let composed = lowered(
        "$schema:\n  - $extends: shape\n    shape: \"{ a: string }\"\n  - shape: \"{ b: number }\"\n    $extends: shape\n",
    );
    let handwritten = lowered(
        "$schema:\n  - a: string\n    shape: \"{ a: string }\"\n  - b: number\n    shape: \"{ b: number }\"\n",
    );
    assert_eq!(composed, handwritten);
}

#[test]
fn composition_targets_local_aliased_keyword_named_and_empty_objects() {
    let types = "types:\n  base: \"{ id: string }\"\n  alias: \"base(required) -> an alias's qualifiers stay with it\"\n  \
                 twice: alias\n  string: \"{ s: number }\"\n  empty: \"{}\"\n";
    let cases = [
        ("{ ...alias }", "{ id: string }"),
        ("{ ...twice, n: number }", "{ id: string, n: number }"),
        ("{ ...string@this }", "{ s: number }"),
        ("{ ...empty, n: number }", "{ n: number }"),
        ("{ ...empty }", "{}"),
    ];
    for (composed, handwritten) in cases {
        let schema = |object: &str| format!("kind: schema\n$schema:\n  value: \"{object}\"\n{types}");
        assert_same_lowering(&schema(composed), &schema(handwritten));
    }
}

#[test]
fn composition_copies_members_and_never_the_target_objects_own_qualifiers() {
    let types = "types:\n  bounded:\n    $constraints:\n      max-keys: 1\n    a: string\n    b: string\n  \
                 closed: \"{ a: string }(required) -> the target's description\"\n";
    let schema = content(&format!("kind: schema\n$schema:\n  copy: \"{{ ...bounded }}\"\n  use: bounded\n  \
                                   other: \"{{ ...closed }}\"\n{types}"));
    let two = json!({ "a": "x", "b": "y" });
    assert!(schema.validate_strict(&json!({ "copy": two })).is_ok(), "the copy drops the target's key bound");
    assert!(!strict_problems(&schema, &json!({ "use": two })).is_empty(), "an ordinary use keeps it");
    let other = schema.property("other").expect("other");
    assert!(!other.is_required() && other.description().is_none(), "the target's postfix stays with it");

    // The receiver's own object constraints apply to the merged members.
    let bounded = content(&format!("kind: schema\n$schema:\n  value:\n    $extends: bounded\n    $constraints:\n      \
                                    max-keys: 2\n      closed: true\n    c: string\n{types}"));
    assert!(bounded.validate_strict(&json!({ "value": { "a": "x", "c": "z" } })).is_ok());
    assert!(!strict_problems(&bounded, &json!({ "value": { "a": "x", "b": "y", "c": "z" } })).is_empty());
}

#[test]
fn composition_patterns_replace_by_emitted_key_and_distinct_patterns_overlap() {
    let schema = |object: &str, types: &str| format!("kind: schema\n$schema:\n  value: \"{object}\"\ntypes:\n{types}");
    // `<starting::a>` and `<pattern::^a>` lower to the same key: the later wins.
    let same = "  base: \"{ <starting::a>: number }\"\n";
    assert_same_lowering(
        &schema("{ ...base, <pattern::^a>: string }", same),
        &schema("{ <pattern::^a>: string }", same),
    );
    // Equal only once the literal exclusion wraps both: `x$` and `^.*x$`.
    let wrapped = "  base: \"{ id: string, <ending::x>: number }\"\n";
    let composed = lowered(&schema("{ ...base, <pattern::^.*x$>: string }", wrapped));
    let patterns = composed["properties"]["value"]["anyOf"][1]["patternProperties"].as_object().expect("patterns");
    assert_eq!(patterns.len(), 1, "{patterns:?}");
    assert_eq!(patterns.values().next(), Some(&json!({ "type": "string" })));
    // Distinct patterns both apply.
    let overlap = content(&schema("{ ...base, <ending::z>: string }", same));
    assert!(overlap.validate_strict(&json!({ "value": { "az": 1 } })).is_err(), "both patterns judge `az`");
    assert!(overlap.validate_strict(&json!({ "value": { "ab": 1, "qz": "s" } })).is_ok());
}

#[test]
fn composition_catch_alls_replace_and_literals_keep_precedence() {
    let types = "types:\n  dict: \"{ <string>: number }\"\n  prefixed: \"{ <starting::x_>: number }\"\n";
    let schema = content(&format!("kind: schema\n$schema:\n  replaced: \"{{ ...dict, <string>: string }}\"\n  \
                                   named: \"{{ ...dict, name: string }}\"\n{types}"));
    assert!(schema.validate_strict(&json!({ "replaced": { "k": "v" } })).is_ok());
    assert!(schema.validate_strict(&json!({ "replaced": { "k": 1 } })).is_err(), "the later catch-all won");
    assert!(schema.validate_strict(&json!({ "named": { "name": "n", "k": 1 } })).is_ok(), "literal precedence");
    assert!(schema.validate_strict(&json!({ "named": { "k": "v" } })).is_err());
    // An inherited pattern without a catch-all closes an open root.
    let closed = content(&format!("kind: schema\n$schema:\n  $extends: prefixed\n{types}"));
    assert!(closed.validate_strict(&json!({ "x_a": 1 })).is_ok());
    assert!(closed.validate_strict(&json!({ "other": 1 })).is_err(), "pattern-only root is closed");
}

#[test]
fn composition_rejects_every_ineligible_target_at_its_operand() {
    let types = "types:\n  label: string\n  meta: object\n  shape:\n    - \"{ a: string }\"\n    - \"{ b: number }\"\n  \
                 grouped: \"string | number\"\n  rows: \"{ a: string }[]\"\n  pair: \"tuple([string, number])\"\n  \
                 greet: 'function(parameters(name: string); returns(string))'\n  dynamic: \"{{ shape_of() }}\"\n  \
                 via: rows\n";
    let cases = [
        ("label", CompositionTargetKind::Primitive, "label"),
        ("meta", CompositionTargetKind::OpaqueObject, "meta"),
        ("shape", CompositionTargetKind::Union, "shape"),
        ("grouped", CompositionTargetKind::Union, "grouped"),
        ("rows", CompositionTargetKind::Array, "rows"),
        ("pair", CompositionTargetKind::Array, "pair"),
        ("greet", CompositionTargetKind::Callable, "greet"),
        ("dynamic", CompositionTargetKind::Template, "dynamic"),
        // Through an alias, the related location is the type it reaches.
        ("via", CompositionTargetKind::Array, "rows"),
    ];
    for (name, kind, declared) in cases {
        for receiver in [format!("  value: \"{{ n: number, ...{name} }}\"\n"), format!("  $extends: {name}\n")] {
            let text = format!("kind: schema\n$schema:\n{receiver}{types}");
            let error = resolution_error(&text);
            let SchemaError::InvalidCompositionTarget { name: written, site, definition, reason } = &error else {
                panic!("`{text}`: {error:?}");
            };
            assert_eq!((written.as_str(), *reason), (name, kind), "{text}");
            let operand = if receiver.contains("$extends") {
                path(&["$schema", "$extends"])
            } else {
                path(&["$schema", "value"]).spread(0)
            };
            assert_eq!(site.path, operand, "{text}");
            assert_eq!(definition.path, path(&["types", declared]), "{text}");
            assert_eq!(definition.source, SchemaSourceId::InlineDocument);
            assert!(error.to_string().contains(name), "{error}");
        }
    }
}

#[test]
fn composition_operands_resolve_even_when_every_member_is_replaced() {
    let types = "types:\n  base: \"{ a: string }\"\n  label: string\n";
    // `a` is replaced, but the operand still names a missing type.
    let missing = resolution_error(&format!("kind: schema\n$schema:\n  value: \"{{ ...nope, a: number }}\"\n{types}"));
    match &missing {
        SchemaError::UnknownNamedType { name, site, .. } => {
            assert_eq!(name, "nope");
            assert_eq!(site.path, path(&["$schema", "value"]).spread(0));
        }
        other => panic!("{other:?}"),
    }
    // A later spread replaces every member of an ineligible earlier one.
    let ineligible = resolution_error(&format!("kind: schema\n$schema:\n  $extends: [label, base]\n  a: number\n{types}"));
    assert!(
        matches!(&ineligible, SchemaError::InvalidCompositionTarget { site, .. } if site.path == path(&["$schema", "$extends"]).union_arm(0)),
        "{ineligible:?}"
    );
}

#[test]
fn composition_a_loop_of_composition_and_alias_hops_reports_every_reference() {
    let direct = resolution_error("kind: schema\n$schema:\n  value: a\ntypes:\n  a: \"{ ...b }\"\n  b: \"{ ...a }\"\n");
    let SchemaError::NonDescendingCycle { cycles } = &direct else { panic!("{direct:?}") };
    let chain: Vec<(SchemaSourcePath, &str)> =
        cycles[0].iter().map(|reference| (reference.site.path.clone(), reference.target.as_str())).collect();
    assert_eq!(chain, [(path(&["types", "b"]).spread(0), "a"), (path(&["types", "a"]).spread(0), "b")]);

    // An alias hop is part of the chain, written where the alias is declared.
    let aliased = resolution_error(
        "kind: schema\n$schema:\n  $extends: a\ntypes:\n  a: \"{ x: string, ...c }\"\n  c: b\n  b:\n    $extends: a\n",
    );
    let SchemaError::NonDescendingCycle { cycles } = &aliased else { panic!("{aliased:?}") };
    let chain: Vec<(SchemaSourcePath, &str)> =
        cycles[0].iter().map(|reference| (reference.site.path.clone(), reference.target.as_str())).collect();
    assert_eq!(
        chain,
        [
            (path(&["types", "a"]).spread(0), "c"),
            (path(&["types", "c"]), "b"),
            (path(&["types", "b", "$extends"]), "a"),
        ]
    );
    assert!(aliased.to_string().contains("types.c -> `b`"), "{aliased}");
    // A self-spread is the shortest such loop.
    assert!(matches!(
        resolution_error("kind: schema\n$schema:\n  value: s\ntypes:\n  s: \"{ ...s }\"\n"),
        SchemaError::NonDescendingCycle { .. }
    ));
}

/// A chain of `hops` composition hops below the root, alternating spreads
/// with aliases when `mixed`.
fn chain(hops: usize, mixed: bool) -> String {
    let mut text = String::from("kind: schema\n$schema:\n  $extends: t1\ntypes:\n");
    for hop in 1..hops {
        let next = hop + 1;
        if mixed && hop % 2 == 0 {
            text.push_str(&format!("  t{hop}: t{next}\n"));
        } else {
            text.push_str(&format!("  t{hop}: \"{{ ...t{next}, m{hop}: number }}\"\n"));
        }
    }
    text.push_str(&format!("  t{hops}: \"{{ leaf: string }}\"\n"));
    text
}

#[test]
fn composition_hops_count_toward_the_named_type_depth_limit() {
    for mixed in [false, true] {
        let at_limit = content(&chain(32, mixed));
        assert!(at_limit.property("leaf").is_some(), "32 hops resolve (mixed: {mixed})");
        let error = resolution_error(&chain(33, mixed));
        let SchemaError::CompositionReference { operand, source, .. } = &error else { panic!("{error:?}") };
        assert!(matches!(source.as_ref(), SchemaError::ImportCycle { .. }), "{source:?}");
        assert!(operand.starts_with('t'), "{operand}");
        assert!(error.to_string().contains("32 levels"), "{error}");
    }
}

const BRANCHES: &str = "types:\n  base: \"{ children: branch[] }\"\n  branch: \"{ ...base, label: string }\"\n";

#[test]
fn composition_a_cycle_through_a_child_value_is_ordinary_recursion() {
    let schema = content(&format!("kind: schema\n$schema:\n  tree: branch\n  also: base\n{BRANCHES}"));
    let json = schema.to_json_schema().expect("lowers");
    assert!(json["$defs"]["branch"]["properties"]["label"].is_object(), "{json}");
    let nested = json!({ "tree": { "label": "a", "children": [{ "label": "b", "children": [{ "label": "c" }] }] } });
    assert!(schema.validate_strict(&nested).is_ok());
    let bad = json!({ "tree": { "children": [{ "children": [{ "label": 3 }] }] } });
    assert_eq!(strict_problems(&schema, &bad), ["/tree/children/0/children/0/label/"]);
    // `base` used as an ordinary value keeps its own members; its children
    // are branches.
    assert!(schema.validate_strict(&json!({ "also": { "children": [{ "label": "x" }] } })).is_ok());
    assert!(!strict_problems(&schema, &json!({ "also": { "label": "x" } })).is_empty());
}

#[test]
fn composition_a_recursive_target_outside_its_cycle_keeps_its_own_type() {
    let schema = content(
        "kind: schema\n$schema:\n  value: labeled\ntypes:\n  node: \"{ children: node[] }\"\n  \
         labeled: \"{ ...node, label: string }\"\n",
    );
    assert!(schema.validate_strict(&json!({ "value": { "label": "l", "children": [{ "children": [] }] } })).is_ok());
    // The children are `node`s, which have no label.
    assert!(!strict_problems(&schema, &json!({ "value": { "children": [{ "label": "l" }] } })).is_empty());
}

#[test]
fn composition_copied_names_are_not_local_names() {
    // Without a `types` table the root's members are its names; one copied
    // in by `$extends` is not among them.
    let error = resolution_error("$schema:\n  $extends: base\n  base: \"{ helper: { q: string } }\"\n  x: \"{ ...helper }\"\n");
    assert!(matches!(&error, SchemaError::UnknownNamedType { name, .. } if name == "helper"), "{error:?}");
    // Selection reads the authored table.
    let schema = content(&with_events("  $extends: events\n  priced: \"{ ...events, price: number(min(0)) }\"\n"));
    assert!(matches!(schema.select("id"), Err(SimplifiedSchemaError::Selection { .. })));
    let priced = content(&format!(
        "kind: schema\n$schema:\n  other: string\n{EVENTS}  priced: \"{{ ...events, price: number(min(0)) }}\"\n"
    ))
    .select("priced")
    .expect("a composed named object is selectable");
    assert!(priced.validate_strict(&json!({ "id": "a", "price": 1 })).is_ok());
    assert!(!strict_problems(&priced, &json!({ "price": -1 })).is_empty());
}

#[test]
fn composition_with_host_supplied_local_types() {
    let root: YamlValue = yaml("value: \"{ ...events, extra: boolean }\"\n");
    let schema = parse_yaml_schema(&root).expect("parses");
    let locals = match parse_yaml_schema(&yaml("events: \"{ id: string(required) }\"\n")).expect("locals") {
        SchemaRoot::Single(shape) => shape,
        SchemaRoot::Union(_) => unreachable!(),
    };
    let context = biscuit_file::FileResolutionContext::from_snapshot(std::path::PathBuf::new(), None, Default::default());
    let expansion = simplified_schema::resolve::expand_named_types_with_locals(
        &schema,
        Some(&locals),
        std::path::Path::new(""),
        None,
        &[],
        &context,
        simplified_schema::embedded::ShippedSources::NONE,
    )
    .expect("expands");
    let SchemaRoot::Single(shape) = &expansion.schema else { unreachable!() };
    let PropertyDef::Single(atom) = &shape.properties["value"] else { panic!() };
    let TypeExpr::InlineObject(value) = &atom.ty else { panic!("{atom:?}") };
    assert_eq!(value.properties.keys().collect::<Vec<_>>(), ["id", "extra"]);
    assert_eq!(value.composition, None, "a resolved shape never carries its composition");
    assert!(expansion.imports.is_empty());
}

#[test]
fn composition_subtype_answers_like_the_handwritten_object() {
    let atom = |text: &str| {
        let schema = content(&format!("kind: schema\n$schema:\n  value: \"{text}\"\n{EVENTS}"));
        let value = schema.property("value").expect("value");
        let PropertyDef::Single(atom) = value.def().clone() else { panic!() };
        atom
    };
    let composed = atom("{ ...events, note: string }");
    let handwritten = atom("{ id: string(required), price: number, note: string }");
    let narrower = atom("{ id: string(required), price: number, note: string, extra: number }");
    for (a, b) in [(&composed, &handwritten), (&handwritten, &composed)] {
        assert!(simplified_schema::is_subtype(a, b));
    }
    assert_eq!(
        simplified_schema::is_subtype(&narrower, &composed),
        simplified_schema::is_subtype(&narrower, &handwritten)
    );
    assert_eq!(
        simplified_schema::is_subtype(&composed, &narrower),
        simplified_schema::is_subtype(&handwritten, &narrower)
    );
}

#[test]
fn composition_the_recursive_comparison_uses_the_definition_table() {
    let schema = content(&format!(
        "kind: schema\n$schema:\n  tree: branch\n  manual: manual\n  wrong: other\n{BRANCHES}  \
         manual: \"{{ children: manual[], label: string }}\"\n  other: \"{{ children: other[], label: number }}\"\n"
    ));
    let atom = |name: &str| {
        let PropertyDef::Single(atom) = schema.property(name).expect(name).def().clone() else { panic!() };
        assert!(matches!(atom.ty, TypeExpr::Ref(_)), "a stored recursive definition: {atom:?}");
        atom
    };
    let definitions = schema.property("tree").expect("tree").definitions();
    let (composed, manual, wrong) = (atom("tree"), atom("manual"), atom("wrong"));
    assert!(simplified_schema::is_subtype_with_definitions(&composed, &manual, definitions));
    assert!(simplified_schema::is_subtype_with_definitions(&manual, &composed, definitions));
    assert!(!simplified_schema::is_subtype_with_definitions(&composed, &wrong, definitions));
    assert!(!simplified_schema::is_subtype_with_definitions(&wrong, &composed, definitions));
}

#[test]
fn composition_soft_validation_repairs_and_defaults_like_the_handwritten_object() {
    let types = "types:\n  base: \"{ count: number(default(3)), flag: boolean }\"\n";
    let composed = content(&format!("kind: schema\n$schema:\n  value: \"{{ ...base, name: string }}\"\n{types}"));
    let handwritten = content(&format!(
        "kind: schema\n$schema:\n  value: \"{{ count: number(default(3)), flag: boolean, name: string }}\"\n{types}"
    ));
    for input in [json!({ "value": {} }), json!({ "value": { "count": "4", "flag": "true" } }), json!({ "value": { "count": null } })] {
        let soft = |schema: &SimplifiedSchema| match schema.validate_soft(&input) {
            Ok(success) => Ok((success.value.into_owned(), success.defaults)),
            Err(ValidationFailure::InvalidData { problems }) => Err(problems.len()),
            Err(other) => panic!("{other:?}"),
        };
        assert_eq!(soft(&composed), soft(&handwritten), "{input}");
    }
    let input = json!({ "value": {} });
    let defaults = composed.validate_soft(&input).expect("valid").defaults;
    assert_eq!(defaults.iter().map(|applied| applied.path.to_string()).collect::<Vec<_>>(), ["value.count"]);
}

#[test]
fn composition_lowering_refuses_an_unresolved_operand() {
    for (text, at) in [
        ("value: \"{ n: number, ...events }\"\n", "value[...0]"),
        ("$extends: [events]\nn: number\n", "<root>.$extends[0]"),
        ("$extends: events\n", "<root>.$extends"),
    ] {
        let schema = parse_yaml_schema(&yaml(text)).expect("parses");
        let error = simplified_schema::simplified::to_json_schema(&schema).expect_err("unresolved");
        let SchemaError::Convert { property, message } = &error else { panic!("{error:?}") };
        assert_eq!(property, at, "{text}");
        assert!(message.contains("events") && message.contains("not resolved"), "{message}");
    }
}

#[test]
fn composition_a_copied_members_lowering_failure_names_its_declaration() {
    let error = lowering_error(
        "kind: schema\n$schema:\n  value: \"{ ...base }\"\ntypes:\n  base:\n    ok: string\n    f: 'function(parameters(void); returns(string))'\n",
    );
    let SchemaError::ComposedMember { site, source } = error.as_ref() else { panic!("{error:?}") };
    assert_eq!(site.path, path(&["types", "base", "f"]));
    assert!(matches!(source.as_ref(), SchemaError::Convert { property, .. } if property == "value.f"), "{source:?}");
    // A member written in the receiver keeps the ordinary error.
    let own = lowering_error("kind: schema\n$schema:\n  value:\n    $extends: base\n    f: 'function(parameters(void); returns(string))'\ntypes:\n  base: \"{ ok: string }\"\n");
    assert!(matches!(own.as_ref(), SchemaError::Convert { .. }), "{own:?}");
    // A supplied value is still reported at its instance location.
    let schema = content(&with_events("  value: \"{ ...events }\"\n"));
    assert_eq!(strict_problems(&schema, &json!({ "value": { "id": 1 } })), ["/value/id/"]);
}

#[test]
fn composition_content_reads_no_file_for_an_imported_operand() {
    for text in [
        "$schema:\n  $extends: base@./types.yaml\n",
        "$schema:\n  value: \"{ ...base@./types.yaml }\"\n",
        "kind: schema\n$schema:\n  value: local\ntypes:\n  local: \"{ ...base@./types.yaml }\"\n",
    ] {
        let error = lowering_error(text);
        let SchemaError::Unresolved { reference, source, .. } = error.as_ref() else { panic!("`{text}`: {error:?}") };
        assert_eq!(reference, "base@./types.yaml");
        assert!(matches!(source, biscuit_file::FileReferenceError::ResolutionUnavailable { .. }), "{source:?}");
    }
}

#[cfg(feature = "file")]
#[test]
fn composition_markdown_frontmatter_resolves() {
    let schema = SimplifiedSchema::try_from_markdown(
        simplified_schema::SchemaSource::from(std::path::PathBuf::from("/virtual/doc.md"))
            .with_text("---\n$schema:\n  $extends: base\n  base: \"{ id: string(required) }\"\n---\nbody\n")
            .bound(biscuit_file::FileResolutionContext::from_snapshot(std::path::PathBuf::new(), None, Default::default())),
    )
    .expect("the frontmatter schema resolves");
    assert!(schema.property("id").expect("copied").is_required());
}

/// Composition read from real schema files.
#[cfg(feature = "file")]
mod disk {
    use std::path::{Path, PathBuf};

    use biscuit_file::FileResolutionContext;
    use serde_json::json;
    use simplified_schema::embedded::ShippedSources;
    use simplified_schema::resolve::resolve_yaml_schema;
    use simplified_schema::{SchemaError, SchemaSource, SimplifiedSchema, SimplifiedSchemaError};

    use super::{path, strict_problems, yaml};

    const EXAMPLE: &str = "kind: example\ninvocation: \"x\"\nreturns: \"val\"\ndescription: d\n";

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let file = dir.join(name);
        std::fs::create_dir_all(file.parent().expect("a parent")).unwrap();
        std::fs::write(&file, text).unwrap();
        file
    }

    fn canonical(path: &Path) -> PathBuf {
        biscuit_file::canonicalize_simplified(path).unwrap()
    }

    fn load(dir: &Path, file: &str) -> Result<SimplifiedSchema, SimplifiedSchemaError> {
        SimplifiedSchema::try_from_yaml(SchemaSource::from(dir.join(file)).bound(FileResolutionContext::new(dir)))
    }

    #[track_caller]
    fn loaded(dir: &Path, file: &str) -> SimplifiedSchema {
        load(dir, file).unwrap_or_else(|error| panic!("`{file}` loads: {error}"))
    }

    #[track_caller]
    fn load_error(dir: &Path, file: &str) -> SchemaError {
        match load(dir, file) {
            Err(SimplifiedSchemaError::Schema(error)) => error,
            other => panic!("`{file}`: expected a schema error, got {other:?}"),
        }
    }

    #[test]
    fn composition_imports_record_the_target_and_its_transitive_files() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        let a = write(dir, "a.yaml", "kind: schema\ntypes:\n  a: \"{ ...b@./nested/b.yaml, x: string }\"\n");
        let b = write(dir, "nested/b.yaml", "kind: schema\ntypes:\n  b: \"{ q: string(required) }\"\n");
        write(dir, "main.yaml", "$schema:\n  $extends: a@./a.yaml\n  own: number\n");
        let schema = loaded(dir, "main.yaml");
        assert!(schema.validate_strict(&json!({ "q": "v", "x": "y", "own": 1 })).is_ok());
        assert!(!strict_problems(&schema, &json!({ "x": "y" })).is_empty(), "the transitive `q` is required");
        let imports: Vec<PathBuf> = schema.provenance().imports().iter().map(|path| canonical(path)).collect();
        assert_eq!(imports, [canonical(&a), canonical(&b)]);
        assert!(schema.provenance().searched().iter().any(|probe| probe.ends_with("nested/b.yaml")));
    }

    #[test]
    fn composition_a_target_file_that_fails_keeps_its_cause() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        write(dir, "broken.yaml", "kind: schema\ntypes:\n  base: \"{ a: \"\n");
        write(dir, "missing.yaml", "$schema:\n  value: \"{ ...base@./absent.yaml, a: string }\"\n");
        write(dir, "invalid.yaml", "$schema:\n  $extends: [base@./broken.yaml]\n");
        let missing = load_error(dir, "missing.yaml");
        let SchemaError::CompositionReference { operand, site, source } = &missing else { panic!("{missing:?}") };
        assert_eq!(operand, "base@./absent.yaml");
        // A file without `types` addresses its `$schema` members from the root.
        assert_eq!(site.path, path(&["value"]).spread(0));
        assert!(matches!(source.as_ref(), SchemaError::NoMatch { .. }), "{source:?}");

        let invalid = load_error(dir, "invalid.yaml");
        let SchemaError::CompositionReference { site, source, .. } = &invalid else { panic!("{invalid:?}") };
        assert_eq!(site.path, path(&["$extends"]).union_arm(0));
        assert!(matches!(source.as_ref(), SchemaError::SchemaDocument { path, .. } if path.ends_with("broken.yaml")), "{source:?}");
        assert!(std::error::Error::source(&invalid).is_some(), "the cause stays in the chain");
    }

    #[test]
    fn composition_recursion_through_a_spread_works_across_two_files() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        write(dir, "base.yaml", "kind: schema\ntypes:\n  base: \"{ children: branch[]@./branch.yaml }\"\n");
        write(dir, "branch.yaml", "kind: schema\ntypes:\n  branch: \"{ ...base@./base.yaml, label: string }\"\n");
        write(dir, "main.yaml", "$schema:\n  tree: branch@./branch.yaml\n");
        let schema = loaded(dir, "main.yaml");
        let nested = json!({ "tree": { "label": "a", "children": [{ "label": "b", "children": [] }] } });
        assert!(schema.validate_strict(&nested).is_ok());
        let bad = json!({ "tree": { "children": [{ "label": 1 }] } });
        assert_eq!(strict_problems(&schema, &bad), ["/tree/children/0/label/"]);
    }

    #[test]
    fn composition_a_copied_member_resolves_names_where_it_was_declared() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        write(dir, "types.yaml", "kind: schema\ntypes:\n  child: \"{ a: string }\"\n  base: \"{ c: child }\"\n");
        write(
            dir,
            "main.yaml",
            "kind: schema\n$schema:\n  $extends: base@./types.yaml\n  mine: child\ntypes:\n  child: \"{ z: number }\"\n",
        );
        let schema = loaded(dir, "main.yaml");
        assert!(schema.validate_strict(&json!({ "c": { "a": "x" }, "mine": { "z": 1 } })).is_ok());
        assert!(schema.validate_strict(&json!({ "c": { "z": 1 } })).is_err(), "`c` is the declaring file's child");
    }

    #[test]
    fn composition_winning_examples_resolve_from_their_declaring_file() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        let near = write(dir, "ex.yaml", EXAMPLE);
        let far = write(dir, "sub/ex.yaml", EXAMPLE);
        write(
            dir,
            "sub/types.yaml",
            "kind: schema\ntypes:\n  base: \"{ id: string(example(./ex.yaml)), gone: string(example(./missing.yaml)) }\"\n",
        );
        write(
            dir,
            "main.yaml",
            "$schema:\n  $extends: base@./sub/types.yaml\n  own: string(example(./ex.yaml))\n  gone: string\n",
        );
        let schema = loaded(dir, "main.yaml");
        let examples: Vec<PathBuf> = schema.provenance().examples().iter().map(|path| canonical(path)).collect();
        assert_eq!(examples.len(), 2, "{examples:?}");
        assert!(examples.contains(&canonical(&near)) && examples.contains(&canonical(&far)), "{examples:?}");
        // The replaced declaration's example is never resolved.
        assert!(!schema.provenance().searched().iter().any(|probe| probe.ends_with("missing.yaml")));
    }

    #[test]
    fn composition_default_files_name_the_winning_declaration() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        let types = write(
            dir,
            "sub/types.yaml",
            "kind: schema\ntypes:\n  base: \"{ n: number(default(1)), m: number(default(2)) }\"\n",
        );
        let declaration = yaml(
            "$extends: base@./sub/types.yaml\nm: number(default(5))\nsnapshot: \"{ ...base@./sub/types.yaml }\"\n",
        );
        let resolved =
            resolve_yaml_schema(&declaration, dir, &FileResolutionContext::new(dir), ShippedSources::NONE).expect("resolves");
        let files: Vec<(Vec<String>, PathBuf)> =
            resolved.default_files.iter().map(|(path, file)| (path.clone(), canonical(file))).collect();
        let key = |segments: &[&str]| segments.iter().map(|segment| segment.to_string()).collect::<Vec<_>>();
        assert_eq!(
            files,
            [
                (key(&["n"]), canonical(&types)),
                (key(&["snapshot", "m"]), canonical(&types)),
                (key(&["snapshot", "n"]), canonical(&types)),
            ],
            "the root's own `m` replaced the imported one"
        );
    }

    #[test]
    fn composition_a_winning_slot_evaluates_in_its_declaring_file() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        let types = write(
            dir,
            "types.yaml",
            "kind: schema\ntypes:\n  base:\n    kind: \"enum(a, {{ more_kinds }})\"\n    replaced: \"enum(b, {{ never_called }})\"\n",
        );
        write(dir, "main.yaml", "$schema:\n  $extends: base@./types.yaml\n  replaced: string\n");
        let schema = loaded(dir, "main.yaml");
        let slots = schema.pending_slots();
        assert_eq!(slots.len(), 1, "only the winning declaration's slot is pending: {slots:?}");
        assert_eq!(slots[0].expression, "more_kinds");
        assert_eq!(slots[0].origin.as_deref().map(canonical), Some(canonical(&types)));

        let host = Recording::default();
        let concrete = schema.resolve_slots(&host).expect("the slot resolves");
        assert_eq!(*host.calls.borrow(), [("more_kinds".to_string(), Some(canonical(&types)))]);
        assert!(concrete.validate_strict(&json!({ "kind": "c", "replaced": "free text" })).is_ok());
    }

    /// A slot host that records each evaluation and its defining file.
    #[derive(Default)]
    struct Recording {
        calls: std::cell::RefCell<Vec<(String, Option<PathBuf>)>>,
    }

    impl simplified_schema::slots::SlotHost for Recording {
        fn admit(&self, _: &simplified_schema::Slot) -> Result<(), simplified_schema::slots::SlotAdmissionError> {
            Ok(())
        }

        fn resolve(
            &self,
            request: &simplified_schema::slots::SlotRequest<'_>,
        ) -> Result<serde_json::Value, simplified_schema::slots::SlotFailure> {
            self.calls.borrow_mut().push((request.slot.expression.clone(), request.defining_file.map(canonical)));
            Ok(json!(["c"]))
        }

        fn template_reads(&self) -> std::rc::Rc<dyn simplified_schema::slots::TemplateReads> {
            std::rc::Rc::new(simplified_schema::slots::RefuseTemplateReads)
        }
    }
}

/// Composition inside an embedded schema bundle: a trigger payload extends a
/// shared type of the bundle, which extends another bundle document.
mod embedded {
    use biscuit_file::FileResolutionContext;
    use serde_json::{Value, json};
    use simplified_schema::embedded::{EmbeddedSchemaBundle, ShippedSources};
    use simplified_schema::shipped::PreparationMode;
    use simplified_schema::triggers::{TriggerOutcomeRecord, TriggerRegistry, prepare_trigger_bundle};
    use simplified_schema::{SchemaError, SchemaSourcePath};

    const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  \
                           $extends: common-attributes@./shared.yaml\n  note: string\n";
    const SHARED: &str = "kind: schema\ntypes:\n  common-attributes:\n    $extends: base@./base.yaml\n    \
                          id: string(required)\n    note: number\n";
    const BASE: &str = "kind: schema\ntypes:\n  base: \"{ origin: string }\"\n";
    const FILES: &[(&str, &str)] = &[("base.yaml", BASE), ("shared.yaml", SHARED), ("trigger.yaml", TRIGGER)];

    fn context() -> FileResolutionContext {
        FileResolutionContext::from_snapshot(std::path::PathBuf::new(), None, Default::default())
    }

    fn registry(files: &'static [(&'static str, &'static str)]) -> TriggerRegistry {
        let mut registry = TriggerRegistry::new(Vec::new(), context(), ShippedSources::NONE);
        registry.append_embedded_bundle(EmbeddedSchemaBundle::new("composed", files)).expect("a valid bundle");
        registry
    }

    #[track_caller]
    fn assert_composed(json: &Value) {
        let properties = json["properties"].as_object().expect("properties");
        assert_eq!(properties.keys().collect::<Vec<_>>(), ["origin", "id", "note"], "{json}");
        assert_eq!(json["required"], json!(["id"]), "{json}");
        assert_eq!(properties["note"]["anyOf"][1]["type"], "string", "the payload's own `note` wins: {json}");
    }

    #[test]
    fn composition_resolves_in_an_embedded_trigger_payload_at_runtime_and_prepared() {
        let registry = registry(FILES);
        assert!(registry.findings().is_empty(), "{:?}", registry.findings());
        let [payload] = registry.payloads() else { panic!("one payload: {:?}", registry.payloads().len()) };
        assert_composed(&payload.json_schema);
        assert!(payload.dependencies.is_empty() && payload.searched.is_empty(), "embedded reads are not files");
        assert_eq!(registry.dependencies().count(), 0);

        let record = prepare_trigger_bundle(EmbeddedSchemaBundle::new("composed", FILES), PreparationMode::Runtime, ShippedSources::NONE)
            .expect("the bundle prepares");
        let trigger = record.documents.iter().find(|document| document.name == "trigger.yaml").expect("trigger");
        let TriggerOutcomeRecord::Payload(prepared) = &trigger.outcome else { panic!("{:?}", trigger.outcome) };
        assert_eq!(prepared.json_schema, *payload.json_schema, "runtime and prepared agree");
    }

    #[test]
    fn composition_failures_in_an_embedded_trigger_keep_their_cause() {
        const MISSING: &[(&str, &str)] = &[(
            "trigger.yaml",
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  $extends: common@./absent.yaml\n",
        )];
        const INVALID: &[(&str, &str)] = &[(
            "trigger.yaml",
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  $extends: 3\n",
        )];
        let missing = registry(MISSING);
        let [finding] = missing.findings() else { panic!("{:?}", missing.findings()) };
        let mut cause: &dyn std::error::Error = finding.cause();
        let composition = loop {
            if let Some(SchemaError::CompositionReference { operand, site, .. }) = cause.downcast_ref::<SchemaError>() {
                break (operand.clone(), site.path.clone());
            }
            cause = cause.source().unwrap_or_else(|| panic!("no composition cause in {:?}", finding.cause()));
        };
        assert_eq!(composition, ("common@./absent.yaml".to_string(), SchemaSourcePath::root().property("$extends")));

        let invalid = registry(INVALID);
        let [finding] = invalid.findings() else { panic!("{:?}", invalid.findings()) };
        assert!(finding.message().contains("$extends"), "{}", finding.message());
        let error = prepare_trigger_bundle(EmbeddedSchemaBundle::new("composed", INVALID), PreparationMode::Runtime, ShippedSources::NONE)
            .expect_err("an invalid directive fails preparation");
        assert!(error.to_string().contains("$extends"), "{error}");
    }
}

// ── Presentations that cross line breaks ─────────────────────────────────

/// Every YAML presentation of one object definition loads through the public
/// factory and lowers exactly as the one-line, single-quoted spelling does:
/// quoting styles fold their line breaks before the grammar reads the text,
/// so wrapping a value never changes or rejects it.
#[test]
fn composition_objects_wrapped_across_lines_load_and_lower_like_one_line() {
    let schema = |payload: &str| format!("kind: schema\n$schema:\n{payload}types:\n  base: '{{ id: string }}'\n");
    for (one_line, wrapped) in [
        ("  p: '{ a: string, n: number }'\n", [
            "  p: '{ a: string,\n    n: number }'\n",
            "  p: \"{ a: string,\n    n: number }\"\n",
            "  p: >-\n    { a: string,\n      n: number }\n",
        ]),
        ("  p: '{ ...base, n: number }'\n", [
            "  p: '{ ...base,\n    n: number }'\n",
            "  p: \"{ ...base,\n    n: number }\"\n",
            "  p: >-\n    { ...base,\n      n: number }\n",
        ]),
    ] {
        for wrapped in wrapped {
            assert_same_lowering(&schema(wrapped), &schema(one_line));
        }
    }
}

/// A quoted `$extends` operand wrapped after `@` folds to `base@ this`, which
/// the operand grammar reads as the local reference `base@this`.
#[test]
fn composition_extends_operand_wrapped_inside_its_reference_loads() {
    let wrapped = "kind: schema\n$schema:\n  $extends: 'base@\n    this'\n  n: number\ntypes:\n  base: '{ id: string }'\n";
    assert_eq!(yaml(wrapped)["$schema"]["$extends"], YamlValue::String("base@ this".into()));
    let one_line = "kind: schema\n$schema:\n  $extends: base@this\n  n: number\ntypes:\n  base: '{ id: string }'\n";
    assert_same_lowering(wrapped, one_line);
    let map = source_map("$extends: 'base@\n    this'\nn: number\n");
    let operand = SchemaSourcePath::root().property("$extends");
    let source = "$extends: 'base@\n    this'\nn: number\n";
    assert_eq!(texts(source, &map, &operand, SchemaSpanKind::ImportName), ["base"]);
    assert_eq!(texts(source, &map, &operand, SchemaSpanKind::ImportReference), ["this"]);
}
