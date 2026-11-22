//! Object composition: the authored `...Name` spread and YAML `$extends`
//! directive, as the passive parsers read them.
//!
//! These cases cover syntax, the authored model (`SchemaShape::composition`),
//! the reserved `$extends` key in every schema-object position, the located
//! errors of the directive's field shapes, the document shapes of the two
//! loaders that carry it, and the source spans of each operand. Resolution,
//! merging, and lowering are covered separately.

use serde_yaml_ng::Value as YamlValue;
use simplified_schema::{
    CompositionOperand, Extends, InlineSpread, ObjectComposition, PropertyDef, SchemaArm, SchemaError,
    SchemaRoot, SchemaShape, SchemaSourcePath, SchemaSpanKind, TypeExpr, locate_standalone_definition_failure,
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
    // Envelope admission still checks the table with the schema-object
    // grammar, so the entry's definition is one an operand could also be.
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
