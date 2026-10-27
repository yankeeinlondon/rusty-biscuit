use darkmatter::markdown::schemas::{
    PropertyDef, SchemaDeclaration, SchemaError, SchemaReferenceKind, SchemaSourcePath,
    SchemaSourcePathSegment, SchemaSpanKind, SimplifiedSchema, parse_property_definition,
    parse_property_definition_with_source, parse_schema_declaration,
    parse_schema_declaration_with_source, parse_yaml_schema,
    classify_schema_reference, resolve::resolve_schema_with_roots,
    simplified::grammar::MAX_INLINE_OBJECT_DEPTH,
};
use serde_yaml_ng::Value as YamlValue;

fn yaml(source: &str) -> YamlValue {
    serde_yaml_ng::from_str(source).expect("valid YAML fixture")
}

fn property_from_schema(value: &YamlValue) -> PropertyDef {
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(YamlValue::String("candidate".into()), value.clone());
    let SimplifiedSchema::Single(mut shape) =
        parse_yaml_schema(&YamlValue::Mapping(mapping)).expect("schema parser must accept value")
    else {
        panic!("one property must produce one schema shape");
    };
    shape.properties.shift_remove("candidate").expect("candidate property")
}

fn authored<'a>(source: &'a str, offset: usize, span: &std::ops::Range<usize>) -> &'a str {
    &source[span.start - offset..span.end - offset]
}

#[test]
fn property_definition_parser_matches_schema_property_matrix() {
    for source in [
        "string(required)",
        "'string(required)'",
        "\"string(required)\"",
        "title: string(required)\nmetadata:\n  count: number\n",
        "- literal(auto)\n- number(min(1))\n- width: number(required)\n",
        // The YAML null scalar is the `null` type at a property position.
        "null",
        "~",
        "- string\n- null\n",
    ] {
        let value = yaml(source);
        assert_eq!(
            parse_property_definition("candidate", &value).expect("public property parser"),
            property_from_schema(&value),
            "public and whole-schema parsers diverged for {source:?}",
        );
    }

    for source in [
        "true",
        "2",
        "[]",
        "[string, true]",
        "string(min(nope))",
    ] {
        let value = yaml(source);
        assert!(
            matches!(
                parse_property_definition("candidate", &value),
                Err(SchemaError::Grammar { .. })
            ),
            "invalid property definition was accepted: {source:?}",
        );
    }
}

#[test]
fn source_aware_property_parser_preserves_semantics_and_authored_spans() {
    const OFFSET: usize = 19;
    let unquoted_source = "string(required)\r\n";
    let unquoted_value = yaml(unquoted_source);
    let unquoted = parse_property_definition_with_source(
        "candidate",
        &unquoted_value,
        unquoted_source,
        OFFSET,
    )
    .expect("source-aware unquoted scalar parse");
    let root = SchemaSourcePath::root();
    assert_eq!(
        authored(
            unquoted_source,
            OFFSET,
            &unquoted.source_map.spans(&root, SchemaSpanKind::TypeKeyword)[0],
        ),
        "string",
    );

    let plain_source = "\"string(min(2); pattern('café'))\"\r\n";
    let plain_value = yaml(plain_source);
    let plain = parse_property_definition_with_source(
        "candidate",
        &plain_value,
        plain_source,
        OFFSET,
    )
    .expect("source-aware scalar parse");
    assert_eq!(
        plain.value,
        parse_property_definition("candidate", &plain_value).expect("semantic parse")
    );
    assert_eq!(
        authored(
            plain_source,
            OFFSET,
            &plain.source_map.spans(&root, SchemaSpanKind::Definition)[0],
        ),
        "\"string(min(2); pattern('café'))\"",
    );
    assert_eq!(
        authored(
            plain_source,
            OFFSET,
            &plain.source_map.spans(&root, SchemaSpanKind::Atom)[0],
        ),
        "string(min(2); pattern('café'))",
    );
    assert_eq!(
        authored(
            plain_source,
            OFFSET,
            &plain.source_map.spans(&root, SchemaSpanKind::TypeKeyword)[0],
        ),
        "string",
    );
    assert_eq!(
        plain
            .source_map
            .spans(&root, SchemaSpanKind::Constraint)
            .iter()
            .map(|span| authored(plain_source, OFFSET, span))
            .collect::<Vec<_>>(),
        ["min(2)", "pattern('café')"],
    );
    assert_eq!(
        plain
            .source_map
            .spans(&root, SchemaSpanKind::Argument)
            .iter()
            .map(|span| authored(plain_source, OFFSET, span))
            .collect::<Vec<_>>(),
        ["2", "'café'"],
    );

    let import_source = "'Widget(required)@./types/café.yaml'\r\n";
    let import_value = yaml(import_source);
    let import = parse_property_definition_with_source(
        "candidate",
        &import_value,
        import_source,
        OFFSET,
    )
    .expect("source-aware import parse");
    assert_eq!(
        authored(
            import_source,
            OFFSET,
            &import.source_map.spans(&root, SchemaSpanKind::ImportName)[0],
        ),
        "Widget",
    );
    assert_eq!(
        authored(
            import_source,
            OFFSET,
            &import.source_map.spans(&root, SchemaSpanKind::ImportReference)[0],
        ),
        "./types/café.yaml",
    );

    let mapping_source = concat!(
        "title: string(required)\r\n",
        "choice:\r\n",
        "  - number(max(9))\r\n",
        "  - nested:\r\n",
        "      value: 'boolean'\r\n",
    );
    let mapping_value = yaml(mapping_source);
    let mapping = parse_property_definition_with_source(
        "candidate",
        &mapping_value,
        mapping_source,
        OFFSET,
    )
    .expect("source-aware mapping parse");
    assert_eq!(
        mapping.value,
        parse_property_definition("candidate", &mapping_value).expect("semantic mapping parse")
    );
    let title = root.property("title");
    assert_eq!(
        authored(
            mapping_source,
            OFFSET,
            &mapping.source_map.spans(&title, SchemaSpanKind::MappingKey)[0],
        ),
        "title",
    );
    let second_arm = root.property("choice").union_arm(1);
    assert_eq!(
        authored(
            mapping_source,
            OFFSET,
            &mapping.source_map.spans(&second_arm, SchemaSpanKind::UnionArm)[0],
        ),
        "nested:\r\n      value: 'boolean'",
    );
    let nested = second_arm.property("nested").property("value");
    assert_eq!(
        authored(
            mapping_source,
            OFFSET,
            &mapping.source_map.spans(&nested, SchemaSpanKind::TypeKeyword)[0],
        ),
        "boolean",
    );

    let inline_source = "'{ child: string(min(1)), nested: { leaf: boolean } }'\r\n";
    let inline_value = yaml(inline_source);
    let inline = parse_property_definition_with_source(
        "candidate",
        &inline_value,
        inline_source,
        OFFSET,
    )
    .expect("source-aware string-form inline object");
    assert_eq!(
        inline.value,
        parse_property_definition("candidate", &inline_value).expect("semantic inline parse")
    );
    let child = root.property("child");
    assert_eq!(
        authored(
            inline_source,
            OFFSET,
            &inline.source_map.spans(&child, SchemaSpanKind::MappingKey)[0],
        ),
        "child",
    );
    let leaf = root.property("nested").property("leaf");
    assert_eq!(
        authored(
            inline_source,
            OFFSET,
            &inline.source_map.spans(&leaf, SchemaSpanKind::TypeKeyword)[0],
        ),
        "boolean",
    );

    let flow_source = "{title: string(required), choice: [number, {nested: boolean}]}\r\n";
    let flow_value = yaml(flow_source);
    let flow = parse_property_definition_with_source(
        "candidate",
        &flow_value,
        flow_source,
        OFFSET,
    )
    .expect("source-aware flow collections");
    assert_eq!(
        flow.value,
        parse_property_definition("candidate", &flow_value).expect("semantic flow parse")
    );
    assert_eq!(
        authored(
            flow_source,
            OFFSET,
            &flow
                .source_map
                .spans(&root.property("title"), SchemaSpanKind::MappingKey)[0],
        ),
        "title",
    );
    assert_eq!(
        authored(
            flow_source,
            OFFSET,
            &flow
                .source_map
                .spans(
                    &root.property("choice").union_arm(1).property("nested"),
                    SchemaSpanKind::TypeKeyword,
                )[0],
        ),
        "boolean",
    );

    let constraints_source = concat!(
        "$constraints:\r\n",
        "  min-keys: 1\r\n",
        "title: string\r\n",
    );
    let constraints_value = yaml(constraints_source);
    let constraints = parse_property_definition_with_source(
        "candidate",
        &constraints_value,
        constraints_source,
        OFFSET,
    )
    .expect("source-aware native constraints");
    let min_keys = root.property("$constraints").property("min-keys");
    assert_eq!(
        authored(
            constraints_source,
            OFFSET,
            &constraints.source_map.spans(&min_keys, SchemaSpanKind::Constraint)[0],
        ),
        "min-keys: 1",
    );
    assert_eq!(
        authored(
            constraints_source,
            OFFSET,
            &constraints.source_map.spans(&min_keys, SchemaSpanKind::Argument)[0],
        ),
        "1",
    );
}

#[test]
fn schema_declaration_parser_classifies_syntax_without_io() {
    for source in ["./schemas/does-not-exist.yaml", "does-not-exist.yaml"] {
        let declaration = parse_schema_declaration(&yaml(source)).expect("local reference syntax");
        let SchemaDeclaration::Reference(reference) = declaration else {
            panic!("string declaration must classify as a reference");
        };
        assert_eq!(reference.file_reference().raw(), source);
        assert_eq!(
            reference.kind(),
            if source.contains('/') {
                SchemaReferenceKind::PathQualified
            } else {
                SchemaReferenceKind::BareName
            }
        );
    }

    for source in [
        "title: string(required)\n",
        "- ./schemas/does-not-exist.yaml\n- kind: literal(review)\n",
    ] {
        let declaration = parse_schema_declaration(&yaml(source)).expect("valid declaration");
        assert!(matches!(declaration, SchemaDeclaration::Schema(_)));
    }

    for source in [
        "https://example.com/schema.yaml",
        "http://example.com/schema.yaml",
        "'{{}}'",
        "true",
        "2",
        "null",
        "[]",
        "[./schema.yaml, true]",
    ] {
        assert!(
            parse_schema_declaration(&yaml(source)).is_err(),
            "invalid declaration was accepted: {source:?}",
        );
    }
}

/// Padded local references must be syntax-checked and resolved as the same
/// string. Before this contract a quoted `' ./schemas/post.yaml '` classified
/// as path-qualified on the trimmed value but resolved the padded one.
#[test]
fn padded_schema_references_are_classified_and_resolved_as_one_trimmed_string() {
    let padded_path = [
        " ./schemas/post.yaml",
        "./schemas/post.yaml ",
        " ./schemas/post.yaml ",
        "\t\n./schemas/post.yaml\n\t",
    ];
    for source in padded_path {
        let reference = classify_schema_reference(source).expect("padded local reference");
        assert_eq!(reference.file_reference().raw(), "./schemas/post.yaml");
        assert_eq!(reference.kind(), SchemaReferenceKind::PathQualified);
    }
    for source in [" post.yaml", "post.yaml ", " post.yaml ", "\tpost.yaml\n"] {
        let reference = classify_schema_reference(source).expect("padded bare name");
        assert_eq!(reference.file_reference().raw(), "post.yaml");
        assert_eq!(reference.kind(), SchemaReferenceKind::BareName);
    }

    // Classification stays passive: none of the above exists on disk.
    for source in ["  ./no/such/dir/missing.yaml  ", " missing.yaml "] {
        classify_schema_reference(source).expect("a missing file is still valid syntax");
    }

    // The declaration parser (parser/validator path) reports the same product.
    let declaration = parse_schema_declaration(&yaml("' ./schemas/does-not-exist.yaml '"))
        .expect("quoted padded declaration");
    let SchemaDeclaration::Reference(reference) = declaration else {
        panic!("string declaration must classify as a reference");
    };
    assert_eq!(reference.file_reference().raw(), "./schemas/does-not-exist.yaml");
    assert_eq!(reference.kind(), SchemaReferenceKind::PathQualified);

    // A whitespace-only value is an empty reference, not a path of spaces.
    for source in ["", " ", "   ", "\t", "\n", " \t\n "] {
        let error = classify_schema_reference(source).expect_err("empty reference must be rejected");
        let SchemaError::Unresolved { reference, source: cause } = &error else {
            panic!("empty reference must report a structured resolution error: {error:?}");
        };
        assert_eq!(reference, "");
        assert!(cause.to_string().contains("empty reference string"), "{cause}");
    }
    assert!(parse_schema_declaration(&yaml("'   '")).is_err());

    // Resolver parity: the same padded strings load the same schema file.
    let dir = tempfile::tempdir().expect("tempdir");
    let schemas = dir.path().join("schemas");
    std::fs::create_dir(&schemas).expect("schemas dir");
    std::fs::write(schemas.join("post.yaml"), "$schema:\n  title: string(required)\n")
        .expect("write referenced schema");

    let context = biscuit_file::FileResolutionContext::new(dir.path());
    let expected =
        resolve_schema_with_roots(&serde_json::json!("./schemas/post.yaml"), dir.path(), &[], &context)
            .expect("unpadded reference resolves")
            .json_schema;
    for source in padded_path {
        let resolved = resolve_schema_with_roots(&serde_json::json!(source), dir.path(), &[], &context)
            .unwrap_or_else(|error| panic!("padded reference {source:?} must resolve: {error}"))
            .json_schema;
        assert_eq!(resolved, expected, "padded reference {source:?} resolved differently");
    }

    // Bare-name resolution against schema roots agrees on the same trimming.
    let roots = [schemas.clone()];
    for source in [" post.yaml", "post.yaml ", " post.yaml ", "\tpost.yaml\n"] {
        let resolved = resolve_schema_with_roots(&serde_json::json!(source), dir.path(), &roots, &context)
            .unwrap_or_else(|error| panic!("padded bare name {source:?} must resolve: {error}"))
            .json_schema;
        assert_eq!(resolved, expected, "padded bare name {source:?} resolved differently");
    }

    for source in ["", "   ", "\t\n"] {
        assert!(
            resolve_schema_with_roots(&serde_json::json!(source), dir.path(), &[], &context).is_err(),
            "the resolver must reject the empty reference {source:?}",
        );
    }
}

#[test]
fn source_aware_schema_declaration_maps_outer_and_reference_spans() {
    const OFFSET: usize = 31;
    let reference_source = "'./schemas/does-not-exist.yaml'\r\n";
    let reference_value = yaml(reference_source);
    let reference = parse_schema_declaration_with_source(
        &reference_value,
        reference_source,
        OFFSET,
    )
    .expect("source-aware reference declaration");
    let root = SchemaSourcePath::root();
    assert_eq!(
        authored(
            reference_source,
            OFFSET,
            &reference.source_map.spans(&root, SchemaSpanKind::Declaration)[0],
        ),
        "'./schemas/does-not-exist.yaml'",
    );
    assert_eq!(
        authored(
            reference_source,
            OFFSET,
            &reference.source_map.spans(&root, SchemaSpanKind::FileReference)[0],
        ),
        "./schemas/does-not-exist.yaml",
    );

    let union_source = "- './missing.yaml'\r\n- kind: literal(review)\r\n";
    let union_value = yaml(union_source);
    let union = parse_schema_declaration_with_source(&union_value, union_source, OFFSET)
        .expect("source-aware union declaration");
    let SchemaDeclaration::Schema(expected) =
        parse_schema_declaration(&union_value).expect("semantic union declaration")
    else {
        panic!("union declaration must parse as a schema");
    };
    let SchemaDeclaration::Schema(actual) = union.value else {
        panic!("source-aware union declaration must parse as a schema");
    };
    assert_eq!(actual, expected);
    let first_arm = root.union_arm(0);
    assert_eq!(
        authored(
            union_source,
            OFFSET,
            &union.source_map.spans(&first_arm, SchemaSpanKind::UnionArm)[0],
        ),
        "'./missing.yaml'",
    );
    assert_eq!(
        authored(
            union_source,
            OFFSET,
            &union.source_map.spans(&first_arm, SchemaSpanKind::FileReference)[0],
        ),
        "./missing.yaml",
    );
}

fn string_object(depth: usize) -> String {
    let mut value = String::from("string");
    for level in 0..depth {
        value = format!("{{ level_{level}: {value} }}");
    }
    value
}

#[test]
fn string_and_native_objects_share_the_depth_boundary() {
    parse_property_definition("candidate", &YamlValue::String(string_object(MAX_INLINE_OBJECT_DEPTH)))
        .expect("string-form object at the shared limit");
    assert!(matches!(
        parse_property_definition(
            "candidate",
            &YamlValue::String(string_object(MAX_INLINE_OBJECT_DEPTH + 1)),
        ),
        Err(SchemaError::Grammar { .. })
    ));

    let mut source = String::from("- string\n- nested:\n");
    for level in 0..MAX_INLINE_OBJECT_DEPTH {
        source.push_str(&"  ".repeat(level + 2));
        source.push_str(&format!("level_{level}:\n"));
    }
    source.push_str(&"  ".repeat(MAX_INLINE_OBJECT_DEPTH + 2));
    source.push_str("leaf: string\n");
    assert!(matches!(
        parse_property_definition("candidate", &yaml(&source)),
        Err(SchemaError::Grammar { .. })
    ));
}

/// One expected source-map entry: a dotted property path (`#n` for a union
/// arm, empty for the root), a role, and the decoded text of each span.
type Expected = (&'static str, SchemaSpanKind, &'static [&'static str]);

fn render_path(path: &SchemaSourcePath) -> String {
    path.segments()
        .iter()
        .map(|segment| match segment {
            SchemaSourcePathSegment::Property(name) => name.clone(),
            SchemaSourcePathSegment::UnionArm(index) => format!("#{index}"),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// The authored spelling of decoded scalar text inside `quote`.
fn quoted_spelling(text: &str, quote: char) -> String {
    match quote {
        '\'' => text.replace('\'', "''"),
        _ => text.replace('\\', "\\\\").replace('"', "\\\""),
    }
}

/// A property whose definition is one primitive atom: key, definition, atom,
/// type keyword, then its constraints and arguments.
fn primitive(
    path: &'static str,
    key: &'static [&'static str],
    definition: &'static [&'static str],
    atom: &'static [&'static str],
    keyword: &'static [&'static str],
    constraints: &'static [&'static str],
    arguments: &'static [&'static str],
) -> Vec<Expected> {
    let mut expected = vec![
        (path, SchemaSpanKind::MappingKey, key),
        (path, SchemaSpanKind::Definition, definition),
        (path, SchemaSpanKind::Atom, atom),
        (path, SchemaSpanKind::TypeKeyword, keyword),
    ];
    if !constraints.is_empty() {
        expected.push((path, SchemaSpanKind::Constraint, constraints));
    }
    if !arguments.is_empty() {
        expected.push((path, SchemaSpanKind::Argument, arguments));
    }
    expected
}

/// The source map follows the grammar's lexical modes: description prose and
/// imported file references never nest or quote, while argument lists keep
/// their quoting. Every expression is accepted by the semantic parser first,
/// and the whole map is compared so a property that silently merges into its
/// neighbor fails.
#[test]
fn expression_source_maps_follow_description_and_file_reference_modes() {
    const OFFSET: usize = 23;
    let b_suggest = || primitive("b", &["b"], &["string(suggest(x, y))"], &["string(suggest(x, y))"], &["string"], &["suggest(x, y)"], &["x", "y"]);
    let rows: Vec<(&str, Vec<Expected>)> = vec![
        // Description prose: quotes and brackets are not structure.
        (
            "{ a: string -> (it's fine), b: string(suggest(x, y)) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ a: string -> (it's fine), b: string(suggest(x, y)) }"][..])],
                primitive("a", &["a"], &["string -> (it's fine)"], &["string"], &["string"], &[], &[]),
                b_suggest(),
            ]
            .concat(),
        ),
        (
            "{ a: string -> (say \"hi), b: string(suggest(x, y)) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ a: string -> (say \"hi), b: string(suggest(x, y)) }"][..])],
                primitive("a", &["a"], &["string -> (say \"hi)"], &["string"], &["string"], &[], &[]),
                b_suggest(),
            ]
            .concat(),
        ),
        (
            "{ a: string -> ({x} it's fine), b: string(suggest(x, y)) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ a: string -> ({x} it's fine), b: string(suggest(x, y)) }"][..])],
                primitive("a", &["a"], &["string -> ({x} it's fine)"], &["string"], &["string"], &[], &[]),
                b_suggest(),
            ]
            .concat(),
        ),
        (
            "{ a: string -> (it's fine), b: string -> (it's clear), c: string(suggest(x, y)) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ a: string -> (it's fine), b: string -> (it's clear), c: string(suggest(x, y)) }"][..])],
                primitive("a", &["a"], &["string -> (it's fine)"], &["string"], &["string"], &[], &[]),
                primitive("b", &["b"], &["string -> (it's clear)"], &["string"], &["string"], &[], &[]),
                primitive("c", &["c"], &["string(suggest(x, y))"], &["string(suggest(x, y))"], &["string"], &["suggest(x, y)"], &["x", "y"]),
            ]
            .concat(),
        ),
        (
            "{ a: string -> plain [x, b: number(min(2)) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ a: string -> plain [x, b: number(min(2)) }"][..])],
                primitive("a", &["a"], &["string -> plain [x"], &["string"], &["string"], &[], &[]),
                primitive("b", &["b"], &["number(min(2))"], &["number(min(2))"], &["number"], &["min(2)"], &["2"]),
            ]
            .concat(),
        ),
        (
            "{ a: string -> plain ] here, b: number(min(2)) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ a: string -> plain ] here, b: number(min(2)) }"][..])],
                primitive("a", &["a"], &["string -> plain ] here"], &["string"], &["string"], &[], &[]),
                primitive("b", &["b"], &["number(min(2))"], &["number(min(2))"], &["number"], &["min(2)"], &["2"]),
            ]
            .concat(),
        ),
        // The same descriptions one object deeper, and on the nested object.
        (
            "{ o: { a: string -> (it's fine), b: string(suggest(x, y)) } -> (outer's [note), c: number(min(2)) }",
            [
                vec![
                    ("", SchemaSpanKind::Atom, &["{ o: { a: string -> (it's fine), b: string(suggest(x, y)) } -> (outer's [note), c: number(min(2)) }"][..]),
                    ("o", SchemaSpanKind::MappingKey, &["o"][..]),
                    ("o", SchemaSpanKind::Definition, &["{ a: string -> (it's fine), b: string(suggest(x, y)) } -> (outer's [note)"][..]),
                    ("o", SchemaSpanKind::Atom, &["{ a: string -> (it's fine), b: string(suggest(x, y)) }"][..]),
                ],
                primitive("o.a", &["a"], &["string -> (it's fine)"], &["string"], &["string"], &[], &[]),
                primitive("o.b", &["b"], &["string(suggest(x, y))"], &["string(suggest(x, y))"], &["string"], &["suggest(x, y)"], &["x", "y"]),
                primitive("c", &["c"], &["number(min(2))"], &["number(min(2))"], &["number"], &["min(2)"], &["2"]),
            ]
            .concat(),
        ),
        // Imported file references are opaque up to `,`, `}`, or `->`.
        (
            "Name@./a(b.yaml",
            vec![
                ("", SchemaSpanKind::Atom, &["Name@./a(b.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a(b.yaml"][..]),
            ],
        ),
        (
            "Name@./a[b.yaml",
            vec![
                ("", SchemaSpanKind::Atom, &["Name@./a[b.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a[b.yaml"][..]),
            ],
        ),
        (
            "Name@./a{b.yaml -> description",
            vec![
                ("", SchemaSpanKind::Atom, &["Name@./a{b.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a{b.yaml"][..]),
            ],
        ),
        (
            "Name@./a(b.yaml -> it's (the) [note",
            vec![
                ("", SchemaSpanKind::Atom, &["Name@./a(b.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a(b.yaml"][..]),
            ],
        ),
        (
            "Name(required)@./a('b.yaml",
            vec![
                ("", SchemaSpanKind::Atom, &["Name(required)@./a('b.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a('b.yaml"][..]),
                ("", SchemaSpanKind::Constraint, &["required"][..]),
            ],
        ),
        (
            "Name@./a(\"b.yaml",
            vec![
                ("", SchemaSpanKind::Atom, &["Name@./a(\"b.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a(\"b.yaml"][..]),
            ],
        ),
        (
            "{ r: Name@./a(b.yaml, s: string(min(1)) }",
            vec![
                ("", SchemaSpanKind::Atom, &["{ r: Name@./a(b.yaml, s: string(min(1)) }"][..]),
                ("r", SchemaSpanKind::MappingKey, &["r"][..]),
                ("r", SchemaSpanKind::Definition, &["Name@./a(b.yaml"][..]),
                ("r", SchemaSpanKind::Atom, &["Name@./a(b.yaml"][..]),
                ("r", SchemaSpanKind::ImportName, &["Name"][..]),
                ("r", SchemaSpanKind::ImportReference, &["./a(b.yaml"][..]),
                ("s", SchemaSpanKind::MappingKey, &["s"][..]),
                ("s", SchemaSpanKind::Definition, &["string(min(1))"][..]),
                ("s", SchemaSpanKind::Atom, &["string(min(1))"][..]),
                ("s", SchemaSpanKind::TypeKeyword, &["string"][..]),
                ("s", SchemaSpanKind::Constraint, &["min(1)"][..]),
                ("s", SchemaSpanKind::Argument, &["1"][..]),
            ],
        ),
        (
            "{ r: Name@./a[b.yaml, s: string(min(1)) }",
            vec![
                ("", SchemaSpanKind::Atom, &["{ r: Name@./a[b.yaml, s: string(min(1)) }"][..]),
                ("r", SchemaSpanKind::MappingKey, &["r"][..]),
                ("r", SchemaSpanKind::Definition, &["Name@./a[b.yaml"][..]),
                ("r", SchemaSpanKind::Atom, &["Name@./a[b.yaml"][..]),
                ("r", SchemaSpanKind::ImportName, &["Name"][..]),
                ("r", SchemaSpanKind::ImportReference, &["./a[b.yaml"][..]),
                ("s", SchemaSpanKind::MappingKey, &["s"][..]),
                ("s", SchemaSpanKind::Definition, &["string(min(1))"][..]),
                ("s", SchemaSpanKind::Atom, &["string(min(1))"][..]),
                ("s", SchemaSpanKind::TypeKeyword, &["string"][..]),
                ("s", SchemaSpanKind::Constraint, &["min(1)"][..]),
                ("s", SchemaSpanKind::Argument, &["1"][..]),
            ],
        ),
        (
            "{ r: Name@./a{b.yaml -> (its file), s: string(min(1)) }",
            vec![
                ("", SchemaSpanKind::Atom, &["{ r: Name@./a{b.yaml -> (its file), s: string(min(1)) }"][..]),
                ("r", SchemaSpanKind::MappingKey, &["r"][..]),
                ("r", SchemaSpanKind::Definition, &["Name@./a{b.yaml -> (its file)"][..]),
                ("r", SchemaSpanKind::Atom, &["Name@./a{b.yaml"][..]),
                ("r", SchemaSpanKind::ImportName, &["Name"][..]),
                ("r", SchemaSpanKind::ImportReference, &["./a{b.yaml"][..]),
                ("s", SchemaSpanKind::MappingKey, &["s"][..]),
                ("s", SchemaSpanKind::Definition, &["string(min(1))"][..]),
                ("s", SchemaSpanKind::Atom, &["string(min(1))"][..]),
                ("s", SchemaSpanKind::TypeKeyword, &["string"][..]),
                ("s", SchemaSpanKind::Constraint, &["min(1)"][..]),
                ("s", SchemaSpanKind::Argument, &["1"][..]),
            ],
        ),
        (
            "{ o: { r: Name@./a('b.yaml }, s: string(suggest(\"p\", q)) }",
            vec![
                ("", SchemaSpanKind::Atom, &["{ o: { r: Name@./a('b.yaml }, s: string(suggest(\"p\", q)) }"][..]),
                ("o", SchemaSpanKind::MappingKey, &["o"][..]),
                ("o", SchemaSpanKind::Definition, &["{ r: Name@./a('b.yaml }"][..]),
                ("o", SchemaSpanKind::Atom, &["{ r: Name@./a('b.yaml }"][..]),
                ("o.r", SchemaSpanKind::MappingKey, &["r"][..]),
                ("o.r", SchemaSpanKind::Definition, &["Name@./a('b.yaml"][..]),
                ("o.r", SchemaSpanKind::Atom, &["Name@./a('b.yaml"][..]),
                ("o.r", SchemaSpanKind::ImportName, &["Name"][..]),
                ("o.r", SchemaSpanKind::ImportReference, &["./a('b.yaml"][..]),
                ("s", SchemaSpanKind::MappingKey, &["s"][..]),
                ("s", SchemaSpanKind::Definition, &["string(suggest(\"p\", q))"][..]),
                ("s", SchemaSpanKind::Atom, &["string(suggest(\"p\", q))"][..]),
                ("s", SchemaSpanKind::TypeKeyword, &["string"][..]),
                ("s", SchemaSpanKind::Constraint, &["suggest(\"p\", q)"][..]),
                ("s", SchemaSpanKind::Argument, &["\"p\"", "q"][..]),
            ],
        ),
        // A pattern key is opaque up to its `>`.
        (
            "{ <pattern::^[a,b]:(c|d)$>: string, e: number(min(1)) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ <pattern::^[a,b]:(c|d)$>: string, e: number(min(1)) }"][..])],
                primitive("<pattern::^[a,b]:(c|d)$>", &["<pattern::^[a,b]:(c|d)$>"], &["string"], &["string"], &["string"], &[], &[]),
                primitive("e", &["e"], &["number(min(1))"], &["number(min(1))"], &["number"], &["min(1)"], &["1"]),
            ]
            .concat(),
        ),
        // Controls: argument quoting, balanced and closing filename punctuation,
        // and quote-free or unparenthesized prose.
        (
            "string(suggest('a)b', c))",
            vec![
                ("", SchemaSpanKind::Atom, &["string(suggest('a)b', c))"][..]),
                ("", SchemaSpanKind::TypeKeyword, &["string"][..]),
                ("", SchemaSpanKind::Constraint, &["suggest('a)b', c)"][..]),
                ("", SchemaSpanKind::Argument, &["'a)b'", "c"][..]),
            ],
        ),
        (
            "enum('a(b', c)",
            vec![
                ("", SchemaSpanKind::Atom, &["enum('a(b', c)"][..]),
                ("", SchemaSpanKind::TypeKeyword, &["enum"][..]),
                ("", SchemaSpanKind::Constraint, &["'a(b', c"][..]),
            ],
        ),
        (
            "Name@./a(b)c.yaml",
            vec![
                ("", SchemaSpanKind::Atom, &["Name@./a(b)c.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a(b)c.yaml"][..]),
            ],
        ),
        (
            "Name@./a)b.yaml",
            vec![
                ("", SchemaSpanKind::Atom, &["Name@./a)b.yaml"][..]),
                ("", SchemaSpanKind::ImportName, &["Name"][..]),
                ("", SchemaSpanKind::ImportReference, &["./a)b.yaml"][..]),
            ],
        ),
        (
            "{ a: string -> plain, b: string -> it's fine, c: string -> (plain) }",
            [
                vec![("", SchemaSpanKind::Atom, &["{ a: string -> plain, b: string -> it's fine, c: string -> (plain) }"][..])],
                primitive("a", &["a"], &["string -> plain"], &["string"], &["string"], &[], &[]),
                primitive("b", &["b"], &["string -> it's fine"], &["string"], &["string"], &[], &[]),
                primitive("c", &["c"], &["string -> (plain)"], &["string"], &["string"], &[], &[]),
            ]
            .concat(),
        ),
    ];

    for (expression, expected) in rows {
        let mut expected: Vec<(String, SchemaSpanKind, Vec<String>)> = expected
            .into_iter()
            .map(|(path, kind, texts)| {
                (path.to_string(), kind, texts.iter().map(ToString::to_string).collect())
            })
            .collect();
        expected.sort();
        for quote in ['\'', '"'] {
            for line_ending in ["\n", "\r\n"] {
                let scalar = format!("{quote}{}{quote}", quoted_spelling(expression, quote));
                let source = format!("{scalar}{line_ending}");
                let value = yaml(&source);
                let semantic = parse_property_definition("candidate", &value)
                    .unwrap_or_else(|error| panic!("semantic parse of {source:?}: {error:?}"));
                let projected =
                    parse_property_definition_with_source("candidate", &value, &source, OFFSET)
                        .unwrap_or_else(|error| panic!("projection of {source:?}: {error:?}"));
                assert_eq!(projected.value, semantic, "{source:?}");

                let mut actual = Vec::new();
                for (path, kind, spans) in projected.source_map.entries() {
                    let texts: Vec<String> =
                        spans.iter().map(|span| authored(&source, OFFSET, span).to_string()).collect();
                    if path.segments().is_empty() && kind == SchemaSpanKind::Definition {
                        assert_eq!(texts, std::slice::from_ref(&scalar), "{source:?}");
                        continue;
                    }
                    actual.push((render_path(path), kind, texts));
                }
                actual.sort();
                let expected_authored: Vec<_> = expected
                    .iter()
                    .map(|(path, kind, texts)| {
                        let texts: Vec<String> =
                            texts.iter().map(|text| quoted_spelling(text, quote)).collect();
                        (path.clone(), *kind, texts)
                    })
                    .collect();
                assert_eq!(actual, expected_authored, "{source:?}");
            }
        }
    }
}

/// Every authored span of `kind` at `path`, as authored text.
fn span_texts<'a>(
    source: &'a str,
    map: &darkmatter::markdown::schemas::SchemaSourceMap,
    path: &SchemaSourcePath,
    kind: SchemaSpanKind,
) -> Vec<&'a str> {
    map.spans(path, kind).iter().map(|span| &source[span.clone()]).collect()
}

/// A bare non-keyword name is a local reference: its identifier is recorded
/// as an `ImportName` span (never `TypeKeyword`) with no `ImportReference`, in
/// every YAML presentation and in documents that contain no `@` at all.
#[test]
fn bare_local_reference_projects_its_name_as_an_import_name_span() {
    let root = SchemaSourcePath::root();
    for (source, path, name) in [
        ("person[](min(1)) -> People\n", root.clone(), "person"),
        ("'person(required)'\n", root.clone(), "person"),
        ("\"post_code -> The code\"\n", root.clone(), "post_code"),
        ("{author: person, title: string}\n", root.property("author"), "person"),
        ("'{ author: x-person(required), title: string }'\n", root.property("author"), "x-person"),
        ("- string\n- person\n", root.union_arm(1), "person"),
        ("[string, 'person[]']\n", root.union_arm(1), "person"),
    ] {
        assert!(!source.contains('@'), "fixture must not contain `@`: {source:?}");
        let value = yaml(source);
        let parsed = parse_property_definition_with_source("candidate", &value, source, 0)
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let map = &parsed.source_map;
        assert_eq!(span_texts(source, map, &path, SchemaSpanKind::ImportName), [name], "{source:?}");
        assert!(map.spans(&path, SchemaSpanKind::ImportReference).is_empty(), "{source:?}");
        assert!(map.spans(&path, SchemaSpanKind::TypeKeyword).is_empty(), "{source:?}");
    }
}

#[test]
fn bare_keyword_stays_a_type_keyword_span_and_explicit_this_records_both_spans() {
    let root = SchemaSourcePath::root();
    let source = "{plain: string, local: string@this, bare: person}\n";
    let value = yaml(source);
    let parsed = parse_property_definition_with_source("candidate", &value, source, 0)
        .expect("source-aware parse");
    let map = &parsed.source_map;

    let plain = root.property("plain");
    assert_eq!(span_texts(source, map, &plain, SchemaSpanKind::TypeKeyword), ["string"]);
    assert!(map.spans(&plain, SchemaSpanKind::ImportName).is_empty());

    let local = root.property("local");
    assert_eq!(span_texts(source, map, &local, SchemaSpanKind::ImportName), ["string"]);
    assert_eq!(span_texts(source, map, &local, SchemaSpanKind::ImportReference), ["this"]);

    let bare = root.property("bare");
    assert_eq!(span_texts(source, map, &bare, SchemaSpanKind::ImportName), ["person"]);
    assert!(map.spans(&bare, SchemaSpanKind::ImportReference).is_empty());
}

#[test]
fn bare_local_reference_in_a_schema_declaration_records_import_name_spans() {
    const OFFSET: usize = 11;
    let source = concat!(
        "author: person(required)\n",
        "reviewers: 'person[] -> Reviewers'\n",
        "person:\n",
        "  name: string(required)\n",
    );
    assert!(!source.contains('@'));
    let value = yaml(source);
    let declared = parse_schema_declaration_with_source(&value, source, OFFSET)
        .expect("a bare reference parses passively without a definition lookup");
    let root = SchemaSourcePath::root();
    for (property, name) in [("author", "person"), ("reviewers", "person")] {
        let path = root.property(property);
        let spans = declared.source_map.spans(&path, SchemaSpanKind::ImportName);
        assert_eq!(spans.len(), 1, "{property}");
        assert_eq!(authored(source, OFFSET, &spans[0]), name, "{property}");
        assert!(declared.source_map.spans(&path, SchemaSpanKind::ImportReference).is_empty());
    }
    let name = root.property("person").property("name");
    assert_eq!(
        authored(source, OFFSET, &declared.source_map.spans(&name, SchemaSpanKind::TypeKeyword)[0]),
        "string",
    );
}

/// Editing one schema property splices only that leaf: an untouched
/// `person@this` elsewhere keeps its explicit spelling even though canonical
/// serialization would write it bare.
#[test]
fn a_leaf_edit_does_not_respell_an_untouched_explicit_local_reference() {
    use darkmatter::markdown::hash::{FrontmatterPathSegment, locate_frontmatter_leaves};
    use darkmatter::markdown::schemas::simplified::{
        grammar::parse_type_expr, serialize_property_atom,
    };

    let document = concat!(
        "---\n",
        "$schema:\n",
        "  editor: person@this\n",
        "  title: string\n",
        "  person:\n",
        "    name: string\n",
        "---\n",
        "Body\n",
    );
    let title = vec![
        FrontmatterPathSegment::Key("$schema".into()),
        FrontmatterPathSegment::Key("title".into()),
    ];
    let leaves = locate_frontmatter_leaves(document, &[title]).expect("locate the title leaf");
    let replacement =
        serialize_property_atom(&parse_type_expr("title", "string(required)").expect("parse"));
    let mut edited = document.to_string();
    edited.replace_range(leaves[0].range.clone(), &replacement);

    assert!(edited.contains("  editor: person@this\n"), "{edited}");
    assert!(edited.contains("  title: string(required)\n"), "{edited}");
    assert_eq!(
        serialize_property_atom(&parse_type_expr("editor", "person@this").expect("parse")),
        "person",
        "canonical serialization is bare, so the splice above preserved authored text",
    );
}

// ── Located definition failures ───────────────────────────────────────────

use darkmatter::markdown::schemas::{
    LocatedDefinitionFailure, locate_standalone_definition_failure,
    parse_standalone_schema_document,
};

fn root() -> SchemaSourcePath {
    SchemaSourcePath::root()
}

/// The source-free error for the same declaration, which every located
/// failure must carry unchanged.
fn declaration_error(source: &str) -> String {
    format!("{:?}", parse_schema_declaration(&yaml(source)).expect_err(source))
}

fn located_declaration_failure(source: &str, offset: usize) -> LocatedDefinitionFailure {
    let failure = parse_schema_declaration_with_source(&yaml(source), source, offset)
        .expect_err(source);
    assert_eq!(format!("{:?}", failure.error), declaration_error(source), "{source:?}");
    failure
}

/// The authored text a located range covers in a document whose first byte
/// sits at `offset`.
fn covered(source: &str, offset: usize, range: Option<std::ops::Range<usize>>) -> Option<&str> {
    range.map(|range| authored(source, offset, &range))
}

#[test]
fn a_located_property_failure_projects_its_token_through_each_scalar_presentation() {
    const OFFSET: usize = 100;
    for (source, value_text) in [
        ("string(min(nope))", "string(min(nope))"),
        ("'string(min(nope))'", "'string(min(nope))'"),
        ("\"string(\\x6Din(nope))\"", "\"string(\\x6Din(nope))\""),
        ("string(min(1);\n  min(nope))\n", "string(min(1);\n  min(nope))"),
        ("string(min(1);\r\n  min(nope))\r\n", "string(min(1);\r\n  min(nope))"),
        ("|-\n  string(min(nope))\n", "|-\n  string(min(nope))"),
    ] {
        let value = yaml(source);
        let expected = parse_property_definition("candidate", &value).expect_err(source);
        let failure = parse_property_definition_with_source("candidate", &value, source, OFFSET)
            .expect_err(source);
        assert_eq!(format!("{:?}", failure.error), format!("{expected:?}"), "{source:?}");
        assert_eq!(failure.path, root(), "{source:?}");
        assert_eq!(covered(source, OFFSET, failure.authored_span), Some("nope"), "{source:?}");
        assert_eq!(covered(source, OFFSET, failure.value_span), Some(value_text), "{source:?}");
    }
}

#[test]
fn a_located_declaration_failure_follows_literal_keys_and_sequence_members() {
    const OFFSET: usize = 7;
    let nested = "outer:\n  \"a.b\":\n    \"c[0]/ü\": 'string(min(nope))'\n";
    let failure = located_declaration_failure(nested, OFFSET);
    assert_eq!(failure.path, root().property("outer").property("a.b").property("c[0]/ü"));
    assert_eq!(covered(nested, OFFSET, failure.authored_span), Some("nope"));
    assert_eq!(covered(nested, OFFSET, failure.value_span), Some("'string(min(nope))'"));

    // The second member shares the first member's prefix; only its own token is named.
    let members = "tags:\n  - string(min(1))\n  - string(min(nope))\n";
    let failure = located_declaration_failure(members, OFFSET);
    assert_eq!(failure.path, root().property("tags").union_arm(1));
    let token = failure.authored_span.clone().expect("token");
    assert_eq!(authored(members, OFFSET, &token), "nope");
    assert_eq!(token.start - OFFSET, members.rfind("nope").expect("second member"));

    let mapping_arm = "tags:\n  - string\n  - width: number(min(nope))\n";
    let failure = located_declaration_failure(mapping_arm, OFFSET);
    assert_eq!(failure.path, root().property("tags").union_arm(1).property("width"));
    assert_eq!(covered(mapping_arm, OFFSET, failure.authored_span), Some("nope"));

    // Root-union arms are followed by index too.
    let root_union = "- a: string\n- b: string(min(nope))\n";
    let failure = located_declaration_failure(root_union, OFFSET);
    assert_eq!(failure.path, root().union_arm(1).property("b"));
    assert_eq!(covered(root_union, OFFSET, failure.authored_span), Some("nope"));
}

/// Each member is valid alone; the second suggestion violates the
/// property's one-suggestion rule, so only the cross-arm check can name it.
#[test]
fn a_second_suggestion_across_valid_arms_is_located_at_the_second_candidate() {
    for source in [
        "color: [string(suggest(red)), string(suggest(blue))]\n",
        "color:\n  - string(suggest(red))\n  - string(suggest(blue))\n",
    ] {
        let failure = located_declaration_failure(source, 0);
        assert_eq!(failure.path, root().property("color").union_arm(1), "{source:?}");
        assert_eq!(covered(source, 0, failure.authored_span), Some("blue"), "{source:?}");
        assert_eq!(
            covered(source, 0, failure.value_span),
            Some("string(suggest(blue))"),
            "{source:?}"
        );
    }
}

#[test]
fn structural_failures_have_a_value_range_but_no_token() {
    for (source, path, value) in [
        ("tags: []\n", root().property("tags"), "[]"),
        ("tags: 7\n", root().property("tags"), "7"),
        ("tags: [string, 7]\n", root().property("tags").union_arm(1), "7"),
        ("tags: [7]\n", root().property("tags").union_arm(0), "7"),
        (
            "box:\n  $constraints:\n    min-keys: -1\n",
            root().property("box").property("$constraints").property("min-keys"),
            "-1",
        ),
    ] {
        let failure = located_declaration_failure(source, 0);
        assert_eq!(failure.path, path, "{source:?}");
        assert_eq!(failure.authored_span, None, "{source:?}");
        assert_eq!(covered(source, 0, failure.value_span), Some(value), "{source:?}");
    }
    // The declaration's own shape is located at the whole declaration.
    let failure = located_declaration_failure("[]", 0);
    assert_eq!((failure.path, failure.value_span), (root(), Some(0..2)));
}

#[test]
fn an_unprojectable_failure_keeps_its_semantic_error() {
    // A quoted scalar continued across lines is outside the strict locator,
    // so the document cannot be located at all.
    let sibling = "a: \"string(min(1);\n  max(2))\"\nb: string(min(nope))\n";
    let failure = located_declaration_failure(sibling, 0);
    assert_eq!(failure.path, root().property("b"));
    assert_eq!((failure.authored_span, failure.value_span), (None, None));

    // An alias has no scalar of its own to project the token into; its
    // value range is still the alias where the definition is used.
    let alias = "a: &s string(suggest(red))\nb: [*s, *s]\n";
    let failure = located_declaration_failure(alias, 0);
    assert_eq!(failure.path, root().property("b").union_arm(1));
    assert_eq!(failure.authored_span, None);
    assert_eq!(covered(alias, 0, failure.value_span), Some("*s"));

    // Source text that authors different text from the parsed value.
    let value = yaml("b: string(min(nope))");
    let failure = parse_schema_declaration_with_source(&value, "b: string(min(nada))\n", 0)
        .expect_err("semantic failure");
    assert_eq!(failure.path, root().property("b"));
    assert_eq!((failure.authored_span, failure.value_span), (None, None));
    assert!(matches!(*failure.error, SchemaError::Grammar { .. }), "{:?}", failure.error);
}

#[test]
fn a_projection_only_failure_is_wrapped_at_the_root_without_ranges() {
    let value = yaml("b: string(min(1))");
    let failure = parse_schema_declaration_with_source(&value, "b: string(min(2))\n", 0)
        .expect_err("source does not author the value");
    assert_eq!(
        (failure.path.clone(), failure.authored_span.clone(), failure.value_span.clone()),
        (root(), None, None)
    );
    assert!(failure.to_string().contains("could not project"), "{failure}");
    let error: SchemaError = failure.into();
    assert!(error.to_string().contains("could not project"), "{error}");
}

#[test]
fn a_located_failure_ranges_cover_non_ascii_and_crlf_documents() {
    const OFFSET: usize = 40;
    let source = "titre:\r\n  \"é.ü\": 'string(min(1); max(nöpe))'\r\n";
    let failure = located_declaration_failure(source, OFFSET);
    assert_eq!(failure.path, root().property("titre").property("é.ü"));
    assert_eq!(covered(source, OFFSET, failure.authored_span), Some("nöpe"));
}

// ── Standalone documents ──────────────────────────────────────────────────

/// The classifier rejects the document, and its outer message still carries
/// the located failure's semantic error.
fn standalone_failure(source: &str) -> LocatedDefinitionFailure {
    let failure = locate_standalone_definition_failure(source).expect(source);
    let outer = parse_standalone_schema_document(source, "/w/types.yaml").expect_err(source);
    assert!(matches!(outer, SchemaError::SchemaDocument { .. }), "{source:?}: {outer:?}");
    assert!(outer.to_string().contains(&failure.error.to_string()), "{source:?}: {outer}");
    failure
}

#[test]
fn standalone_definition_failures_are_located_in_both_envelopes() {
    for (source, path) in [
        (
            "kind: schema\ntypes:\n  slug: string(min(nope))\n",
            root().property("types").property("slug"),
        ),
        (
            "kind: schema\ntypes:\n  slug: 'string(min(nope))'\n",
            root().property("types").property("slug"),
        ),
        (
            "kind: schema\ntypes:\n  slug: string(min(1);\n    min(nope))\n",
            root().property("types").property("slug"),
        ),
        (
            "kind: schema\ntypes:\n  slug: |-\n    string(min(nope))\n",
            root().property("types").property("slug"),
        ),
        (
            "$schema:\n  slug: \"string(min(nope))\"\n",
            root().property("$schema").property("slug"),
        ),
        (
            "$schema:\n  page:\n    \"a.b\": string(min(nope))\n",
            root().property("$schema").property("page").property("a.b"),
        ),
        (
            "$schema:\n  - a: string\n  - b: string(min(nope))\n",
            root().property("$schema").union_arm(1).property("b"),
        ),
    ] {
        let failure = standalone_failure(source);
        assert_eq!(failure.path, path, "{source:?}");
        assert_eq!(covered(source, 0, failure.authored_span), Some("nope"), "{source:?}");
    }
}

#[test]
fn a_standalone_failure_is_returned_even_when_it_cannot_be_located() {
    let source = "kind: schema\ntypes:\n  note: \"string(min(1);\n    max(2))\"\n  slug: string(min(nope))\n";
    let failure = standalone_failure(source);
    assert_eq!(failure.path, root().property("types").property("slug"));
    assert_eq!((failure.authored_span, failure.value_span), (None, None));
}

#[test]
fn standalone_documents_without_a_definition_failure_return_none() {
    for source in [
        // Valid payloads, including the YAML null type and an empty object.
        "kind: schema\ntypes:\n  slug: string(min(1))\n",
        "kind: schema\ntypes:\n  nothing:\n  empty: {}\n",
        "kind: schema\ntypes: {}\n",
        "$schema:\n  - a: string\n",
        "$schema: ./types.yaml\n",
        // Unrecognized documents.
        "title: string(min(nope))\n",
        "$schema:\n  slug: string(min(nope))\nother: 1\n",
        "kind: note\ntypes:\n  slug: string(min(nope))\n",
        "- string(min(nope))\n",
    ] {
        assert!(locate_standalone_definition_failure(source).is_none(), "{source:?}");
        assert!(
            parse_standalone_schema_document(source, "/w/types.yaml").is_ok(),
            "{source:?}"
        );
    }
}

/// Outer failures stay the classifier's to report; the helper declines them
/// rather than fabricating a definition range.
#[test]
fn outer_envelope_and_declaration_shape_failures_return_none() {
    for source in [
        // Loader: invalid YAML, a duplicate key, a second document, trailing garbage.
        "kind: schema\ntypes:\n  slug: [\n",
        "kind: schema\ntypes:\n  slug: string(min(nope))\n  slug: string\n",
        "kind: schema\ntypes:\n  slug: string(min(nope))\n---\nkind: note\n",
        "kind: schema\ntypes:\n  slug: string(min(nope))\n]\n",
        // Envelopes: a global catalog, malformed `types`, extra tagged keys.
        "kind: schema\n$schema:\n  slug: string(min(nope))\n",
        "kind: schema\ntypes: 7\n",
        "kind: schema\ntypes:\n",
        "kind: schema\n",
        "kind: schema\ntypes:\n  slug: string(min(nope))\nextra: 1\n",
        // A non-string extra key is still an unsupported tagged key.
        "kind: schema\ntypes:\n  slug: string(min(nope))\n7: x\n",
        // Declaration shapes: payload and root-union arms of the wrong type,
        // an empty root union, and a remote reference.
        "$schema: 7\n",
        "$schema:\n",
        "$schema: []\n",
        "$schema: [7]\n",
        "$schema:\n  - https://example.com/types.yaml\n",
    ] {
        assert!(locate_standalone_definition_failure(source).is_none(), "{source:?}");
        assert!(
            parse_standalone_schema_document(source, "/w/types.yaml").is_err(),
            "{source:?} must stay an outer failure"
        );
    }
}

#[test]
fn standalone_definition_field_shapes_are_located_through_one_validator() {
    // Control: the unedited fixture is valid, so each edit below is load-bearing.
    let fixture = |definition: &str| format!("kind: schema\ntypes:\n  tags: {definition}\n");
    assert!(locate_standalone_definition_failure(&fixture("[string, number]")).is_none());
    for (definition, path, value) in [
        ("7", root().property("types").property("tags"), "7"),
        ("true", root().property("types").property("tags"), "true"),
        ("[]", root().property("types").property("tags"), "[]"),
        ("[string, 7]", root().property("types").property("tags").union_arm(1), "7"),
        ("[7, true]", root().property("types").property("tags").union_arm(0), "7"),
        ("[[string]]", root().property("types").property("tags").union_arm(0), "[string]"),
    ] {
        let source = fixture(definition);
        let failure = standalone_failure(&source);
        assert_eq!(failure.path, path, "{definition}");
        assert_eq!(covered(&source, 0, failure.value_span), Some(value), "{definition}");
    }
}

/// An inline frontmatter `$schema` declaration is located in whole-document
/// bytes: the caller passes the declaration's text and its document offset
/// once, and the ranges land on the document's own token.
#[test]
fn an_inline_frontmatter_declaration_failure_is_located_in_document_bytes() {
    for newline in ["\n", "\r\n"] {
        let document = format!(
            "---{newline}title: café{newline}$schema:{newline}  page:{newline}    tags:{newline}      - string{newline}      - string(min(nope)){newline}---{newline}Body{newline}"
        );
        let start = document.find("  page:").expect("declaration");
        let end = document.rfind("---").expect("closing fence");
        let declaration = &document[start..end];
        let failure = located_declaration_failure(declaration, start);
        assert_eq!(failure.path, root().property("page").property("tags").union_arm(1));
        let token = failure.authored_span.expect("token");
        assert_eq!(&document[token.clone()], "nope", "{newline:?}");
        assert_eq!(token.start, document.find("nope").expect("token"), "{newline:?}");
        let value = failure.value_span.expect("value");
        assert_eq!(&document[value], "string(min(nope))", "{newline:?}");
    }
}
