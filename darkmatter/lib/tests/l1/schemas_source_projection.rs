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
        "null",
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
