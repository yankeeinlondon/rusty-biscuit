//! The typed grammar of the core schema types `unknown` (alias `any`),
//! `null`, `alphanumeric`, and `shell`: keyword and option parsing, native
//! YAML null at every type position with its authored span, named-reference
//! option identity, and serializer round trips.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
use darkmatter::markdown::schemas::simplified::serialize_property_atom;
use darkmatter::markdown::schemas::{
    AlphanumericOption, AlphanumericOptions, Constraint, DarkmatterSchemas, PropertyAtom, PropertyDef,
    SchemaConstraintConflictKind, SchemaError, SchemaSourceId, SchemaSourcePath, SchemaSpanKind,
    SimplifiedSchema, SimplifiedType, TypeExpr, parse_property_definition,
    parse_property_definition_with_source, parse_yaml_schema,
};
use serde_json::json;
use serde_yaml_ng::Value as YamlValue;
use tempfile::TempDir;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fixture");
    path
}

fn yaml(text: &str) -> YamlValue {
    serde_yaml_ng::from_str(text).expect("fixture YAML parses")
}

fn resolve_inline(dir: &Path, schema: &str) -> Result<ResolvedSchema, SchemaError> {
    resolve_yaml_schema(&yaml(schema), dir, &FileResolutionContext::new(dir))
}

fn atom<'a>(resolved: &'a ResolvedSchema, property: &str) -> &'a PropertyAtom {
    let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified else {
        panic!("expected a single simplified shape");
    };
    match &shape.properties[property] {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(_) => panic!("`{property}` resolved to a union"),
    }
}

fn path(segments: &[&str]) -> SchemaSourcePath {
    segments
        .iter()
        .fold(SchemaSourcePath::root(), |path, segment| path.property(*segment))
}

/// Whether a document in `dir` whose frontmatter is `$schema` plus `data`
/// validates.
fn validates(dir: &Path, schema_reference: &str, data: &str) -> bool {
    let source: Markdown = format!("---\n$schema: {schema_reference}\n{data}\n---\nbody\n").into();
    let source = source.with_source(darkmatter::markdown::compose::ComposeSource::File(
        dir.join("document.md"),
    ));
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .validate(&source)
        .expect("schema prepares")
        .valid
}

fn options(list: &[AlphanumericOption]) -> AlphanumericOptions {
    list.iter().copied().collect()
}

// ── keywords ─────────────────────────────────────────────────────────────

#[test]
fn any_is_unknown_and_serializes_canonically() {
    let any = parse_type_expr("p", "any(required)").expect("any parses");
    assert_eq!(any.ty, TypeExpr::Primitive(SimplifiedType::Unknown));
    assert_eq!(any, parse_type_expr("p", "unknown(required)").expect("unknown parses"));
    assert_eq!(serialize_property_atom(&any), "unknown(required)");
    assert_eq!(
        serialize_property_atom(&parse_type_expr("p", "any[]").expect("parses")),
        "unknown[]"
    );
}

#[test]
fn shipped_qualified_shell_import_stays_an_import() {
    // `claudine/docs/schemas/partials/stack.yaml` writes `shell@./shell.yaml`;
    // `shell` becoming a keyword must not change that reference.
    let atom = parse_type_expr("p", "shell@./shell.yaml[]").expect("parses");
    assert!(matches!(&atom.ty, TypeExpr::Imported { name, .. } if name == "shell"));
}

// ── native null at type positions ────────────────────────────────────────

#[test]
fn every_native_null_spelling_is_the_null_type_at_a_property() {
    for spelling in ["null", "Null", "NULL", "~", "", "'null'", "\"null\""] {
        let schema = parse_yaml_schema(&yaml(&format!("review: {spelling}\n")))
            .unwrap_or_else(|error| panic!("`review: {spelling}` parses: {error}"));
        let SimplifiedSchema::Single(shape) = schema else {
            panic!("single shape expected");
        };
        assert_eq!(
            shape.properties["review"],
            PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Null)),
            "{spelling:?}"
        );
    }
}

#[test]
fn native_null_is_a_union_arm_and_a_nested_property() {
    let schema = parse_yaml_schema(&yaml(
        "draft: [string, null]\nblock:\n  - number\n  - ~\nmeta:\n  gone:\n",
    ))
    .expect("parses");
    let SimplifiedSchema::Single(shape) = schema else {
        panic!("single shape expected");
    };
    for property in ["draft", "block"] {
        let PropertyDef::Union(arms) = &shape.properties[property] else {
            panic!("`{property}` is a union");
        };
        assert_eq!(arms[1], PropertyAtom::bare(SimplifiedType::Null), "{property}");
    }
    let PropertyDef::Single(meta) = &shape.properties["meta"] else {
        panic!("meta is a single mapping atom");
    };
    let TypeExpr::InlineObject(nested) = &meta.ty else {
        panic!("meta is an inline object");
    };
    assert_eq!(
        nested.properties["gone"],
        PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Null))
    );
}

#[test]
fn root_null_null_root_arms_and_literal_null_stay_errors() {
    for root in ["null", "~", "[null]", "[{a: string}, ~]"] {
        assert!(parse_yaml_schema(&yaml(root)).is_err(), "root {root:?} must fail");
    }
    assert!(parse_type_expr("p", "literal(null)").is_err());
}

#[test]
fn a_native_null_definition_keeps_its_authored_span() {
    const OFFSET: usize = 40;
    for (source, keyword) in [("~\n", "~"), ("Null\n", "Null"), ("null\n", "null")] {
        let value = yaml(source);
        let projected = parse_property_definition_with_source("review", &value, source, OFFSET)
            .unwrap_or_else(|error| panic!("{source:?} projects: {error}"));
        assert_eq!(projected.value, PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Null)));
        let spans = projected
            .source_map
            .spans(&SchemaSourcePath::root(), SchemaSpanKind::TypeKeyword);
        assert_eq!(spans.len(), 1, "{source:?}");
        assert_eq!(&source[spans[0].start - OFFSET..spans[0].end - OFFSET], keyword);
    }

    let source = "- string\n- ~\n";
    let projected = parse_property_definition_with_source("draft", &yaml(source), source, 0)
        .expect("union with a null arm projects");
    let arm = SchemaSourcePath::root().union_arm(1);
    let spans = projected.source_map.spans(&arm, SchemaSpanKind::TypeKeyword);
    assert_eq!(&source[spans[0].clone()], "~");
}

#[test]
fn an_empty_value_and_an_empty_union_item_project_as_null() {
    let source = "title:\nstatus:\n  - number\n  -\n";
    let value = yaml(source);
    let mapping = value.as_mapping().expect("mapping");
    for (key, expected) in [
        ("title", PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Null))),
        (
            "status",
            PropertyDef::Union(vec![
                PropertyAtom::bare(SimplifiedType::Number),
                PropertyAtom::bare(SimplifiedType::Null),
            ]),
        ),
    ] {
        assert_eq!(
            parse_property_definition(key, &mapping[&YamlValue::from(key)]).expect("parses"),
            expected
        );
    }
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "types.yaml", &format!("kind: schema\ntypes:\n  {}", source.replace('\n', "\n  ")));
    let reference = "./types.yaml";
    assert!(validates(dir.path(), reference, "title: null\nstatus: null"));
    assert!(validates(dir.path(), reference, "status: 4"));
    assert!(!validates(dir.path(), reference, "title: x"));
    assert!(!validates(dir.path(), reference, "status: x"));
}

#[test]
fn a_null_named_type_resolves_through_local_and_imported_references() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  nothing: ~\n  quoted: \"null\"\n");
    write(
        dir.path(),
        "local.yaml",
        "kind: schema\ntypes:\n  nothing: null\n  a: nothing\n  b: [string, nothing@this]\n",
    );
    let resolved = resolve_inline(
        dir.path(),
        "a: nothing@./types.yaml\nb: quoted@./types.yaml\nc: nothing[]@./types.yaml\n",
    )
    .expect("imported null types resolve");
    for property in ["a", "b"] {
        assert_eq!(atom(&resolved, property).ty, TypeExpr::Primitive(SimplifiedType::Null));
    }
    assert!(atom(&resolved, "c").is_array);

    for (data, valid) in [
        ("a: null", true),
        ("a: x", false),
        ("b: x", true),
        ("b: null", true),
        // `5` coerces to the string arm; a structured value matches no arm.
        ("b: [1]", false),
    ] {
        assert_eq!(validates(dir.path(), "./local.yaml", data), valid, "{data}");
    }
}

#[test]
fn a_null_default_must_be_null() {
    let dir = TempDir::new().expect("tempdir");
    resolve_inline(dir.path(), "gone: null(default(null))\n").expect("a null default loads");
    for (definition, argument) in [("null(default(1))", "1"), ("null(required; default(x))", "x")] {
        let error = resolve_inline(dir.path(), &format!("gone: \"{definition}\"\n"))
            .expect_err("a non-null default fails to load");
        let SchemaError::Grammar { property, message, span, .. } = &error else {
            panic!("{definition}: a located grammar error expected, got {error:?}");
        };
        assert_eq!(property, "gone");
        assert!(message.contains("must be null"), "{message}");
        assert_eq!(&definition[span.clone()], argument, "{definition}: the span covers the default");
    }
    // A default written at a use site of a `null` named type is judged after
    // resolution.
    let error = resolve_inline(dir.path(), "nothing: null\ngone: nothing(default(1))\n")
        .expect_err("a non-null default through a reference fails to load");
    assert!(error.to_string().contains("must be null"), "{error}");
}

// ── alphanumeric options ─────────────────────────────────────────────────

#[test]
fn option_order_and_redundant_options_do_not_change_identity() {
    let read = |text: &str| {
        parse_type_expr("p", text)
            .expect("parses")
            .alphanumeric_options()
            .expect("alphanumeric")
    };
    assert_eq!(
        read("alphanumeric(leading-alpha, allow-dashes)"),
        read("alphanumeric(allow-dashes, leading-alpha, block-leading-dash)")
    );
    assert_eq!(read("alphanumeric(block-leading-dash)"), read("alphanumeric"));
    assert_eq!(read("alphanumeric(block-double-periods; min(1))"), AlphanumericOptions::base());
    assert_eq!(
        read("alphanumeric(allow-periods, block-double-periods)"),
        options(&[AlphanumericOption::AllowPeriods, AlphanumericOption::BlockDoublePeriods])
    );
}

#[test]
fn serialization_writes_canonical_options_and_round_trips_meaning() {
    for (authored, canonical) in [
        (
            "alphanumeric(leading-alpha, allow-dashes, block-leading-dash; min(1))",
            "alphanumeric(allow-dashes,leading-alpha;min(1))",
        ),
        ("alphanumeric(block-leading-slash)", "alphanumeric"),
        ("alphanumeric(block-leading-slash; required)", "alphanumeric(required)"),
        ("alphanumeric(allow-slashes)[](min(1))", "alphanumeric(allow-slashes)[](min(1))"),
        ("null(required)", "null(required)"),
        ("any", "unknown"),
        ("shell(required)", "shell(required)"),
    ] {
        let first = parse_type_expr("p", authored).expect("authored parses");
        let written = serialize_property_atom(&first);
        assert_eq!(written, canonical, "{authored}");
        // Parse → serialize → parse → serialize is stable and keeps meaning.
        let again = parse_type_expr("p", &written).expect("canonical form parses");
        assert_eq!(serialize_property_atom(&again), written, "{authored}");
        assert_eq!(again.alphanumeric_options(), first.alphanumeric_options(), "{authored}");
        assert_eq!(again.ty, first.ty);
        assert_eq!(again.is_array, first.is_array);
    }
}

#[test]
fn serialization_preserves_option_assertions_on_unresolved_references() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  slug: alphanumeric(allow-dashes)\n");
    for restriction in ["block-leading-dash", "block-leading-slash", "block-double-periods"] {
        for definition in [
            format!("slug({restriction})"),
            format!("slug({restriction})[]"),
            format!("slug({restriction})@./types.yaml"),
            format!("slug({restriction}; max(10))[]@./types.yaml"),
            format!("{{ child: slug({restriction})@./types.yaml }}[]"),
        ] {
            let original = parse_type_expr("value", &definition).expect("reference parses");
            let serialized = serialize_property_atom(&original);
            for expression in [&definition, &serialized] {
                let error = resolve_inline(
                    dir.path(),
                    &format!("slug: alphanumeric(allow-dashes)\nvalue: '{expression}'\n"),
                )
                .expect_err("a round trip must preserve the mismatched option assertion");
                assert!(
                    matches!(error, SchemaError::ConstraintConflict { kind: SchemaConstraintConflictKind::Options, .. }),
                    "{definition} -> {serialized}: {error:?}"
                );
            }
        }
    }
}

#[test]
fn equal_option_sets_at_a_use_site_resolve() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  slug: alphanumeric(allow-dashes, block-leading-dash; max(40))\n",
    );
    let resolved = resolve_inline(
        dir.path(),
        "a: slug(max(10))@./types.yaml\n\
         b: slug(allow-dashes, block-leading-dash; max(10))@./types.yaml\n\
         c: slug(block-leading-dash, allow-dashes)@./types.yaml\n\
         d: slug[]@./types.yaml\n",
    )
    .expect("equivalent spellings resolve");
    let expected = options(&[AlphanumericOption::AllowDashes, AlphanumericOption::BlockLeadingDash]);
    for property in ["a", "b", "c", "d"] {
        assert_eq!(atom(&resolved, property).alphanumeric_options(), Some(expected), "{property}");
    }
    // Constraints after `;` merge as constraints; the stricter `max` wins.
    let b = atom(&resolved, "b");
    assert_eq!(
        b.constraints
            .iter()
            .filter(|c| matches!(c, darkmatter::markdown::schemas::Constraint::Options(_)))
            .count(),
        1,
        "the repeated option list is not duplicated: {b:?}"
    );
    assert!(b.constraints.contains(&darkmatter::markdown::schemas::Constraint::MaxLen(10)));
}

#[test]
fn a_different_option_set_at_a_use_site_is_a_conflict_with_both_locations() {
    let dir = TempDir::new().expect("tempdir");
    let types = write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  slug: alphanumeric(allow-dashes, allow-periods; max(40))\n",
    );
    let types_text = std::fs::read_to_string(&types).expect("read fixture");
    let err = resolve_inline(dir.path(), "c: slug(allow-periods)@./types.yaml\n")
        .expect_err("a different option set conflicts");
    let SchemaError::ConstraintConflict { name, kind, site, constraints } = err else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(name, "slug");
    assert_eq!(kind, SchemaConstraintConflictKind::Options);
    assert_eq!(site.path, path(&["c"]));
    assert_eq!(constraints.len(), 2);
    let definition = &constraints[0];
    assert_eq!(
        definition.origin.source,
        SchemaSourceId::File(biscuit_file::canonicalize_simplified(&types).expect("canonical"))
    );
    assert_eq!(definition.origin.path, path(&["slug"]));
    let span = definition.origin.span.clone().expect("the definition's options are located");
    assert_eq!(&types_text[span], "allow-dashes, allow-periods");
    assert_eq!(constraints[1].origin.source, SchemaSourceId::InlineDocument);
    assert_eq!(constraints[1].origin.path, path(&["c"]));
    assert_eq!(constraints[1].origin.keyword, "allow-periods");
}

#[test]
fn a_conflict_against_a_base_type_without_options_names_its_declaration() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  code: alphanumeric(min(1))\n");
    let err = resolve_inline(dir.path(), "c: code(allow-dashes)@./types.yaml\n")
        .expect_err("widening the base set conflicts");
    let SchemaError::ConstraintConflict { kind, constraints, .. } = err else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(kind, SchemaConstraintConflictKind::Options);
    assert_eq!(constraints[0].origin.path, path(&["code"]));
    assert_eq!(constraints[0].origin.keyword, "alphanumeric");
    assert_eq!(constraints[0].origin.span, None, "bare `alphanumeric` has no option token");
    assert_eq!(constraints[0].constraint, Constraint::Options(vec![]));
    assert_eq!(
        constraints[1].constraint,
        Constraint::Options(vec![AlphanumericOption::AllowDashes])
    );
}

#[test]
fn every_route_to_a_different_option_set_conflicts() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "local.yaml",
        "kind: schema\ntypes:\n  slug: alphanumeric(allow-dashes)\n  wide: slug(allow-periods)\n  w: wide\n",
    );
    write(
        dir.path(),
        "union.yaml",
        "kind: schema\ntypes:\n  slug: alphanumeric(allow-dashes)\n  u: [slug(allow-slashes), null]\n  w: u\n",
    );
    for file in ["./local.yaml", "./union.yaml"] {
        let err = resolve_yaml_schema(
            &YamlValue::String(file.into()),
            dir.path(),
            &FileResolutionContext::new(dir.path()),
        )
        .expect_err("a named definition cannot widen another's options");
        let mut source: &SchemaError = &err;
        while let SchemaError::ReferencedSchema { source: inner, .. } = source {
            source = inner;
        }
        assert!(
            matches!(
                source,
                SchemaError::ConstraintConflict { kind: SchemaConstraintConflictKind::Options, .. }
            ),
            "{file}: {err:?}"
        );
    }
}

#[test]
fn options_on_a_target_that_is_not_alphanumeric_fail() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  title: string(min(1))\n  either: [alphanumeric(allow-dashes), number]\n",
    );
    let err = resolve_inline(dir.path(), "t: title(allow-dashes)@./types.yaml\n")
        .expect_err("a string target has no options");
    assert!(err.to_string().contains("not `alphanumeric`"), "{err}");
    let err = resolve_inline(dir.path(), "e: either(allow-dashes)@./types.yaml\n")
        .expect_err("a union target takes no options");
    assert!(matches!(err, SchemaError::UnionReference { .. }), "{err:?}");
}

#[test]
fn option_grammar_errors_surface_through_schema_loading() {
    let dir = TempDir::new().expect("tempdir");
    for (definition, expected) in [
        ("alphanumeric(allow-dash)", "unknown `alphanumeric` option"),
        ("alphanumeric(allow-dashes, allow-dashes)", "repeated `alphanumeric` option"),
        ("alphanumeric(min(1); allow-dashes)", "before the first `;`"),
        ("alphanumeric(allow-dashes, min(1))", "alphanumeric(allow-dashes; min(1))"),
    ] {
        let err = resolve_inline(dir.path(), &format!("p: \"{definition}\"\n"))
            .expect_err("option grammar errors fail to load");
        assert!(err.to_string().contains(expected), "{definition}: {err}");
    }
}

#[test]
fn lowering_records_the_canonical_options() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "a: alphanumeric(required)\nb: alphanumeric(leading-alpha, block-leading-dash; required)\n",
    )
    .expect("resolves");
    let props = resolved.json_schema["properties"].as_object().expect("properties");
    assert_eq!(props["a"]["x-darkmatter-type"], json!("alphanumeric"));
    assert_eq!(props["a"]["x-darkmatter-options"], json!([]));
    assert_eq!(props["b"]["x-darkmatter-options"], json!(["leading-alpha"]));
}

// ── schema meta-types ────────────────────────────────────────────────────

#[test]
fn type_definition_values_follow_the_same_null_rules() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "meta.yaml", "kind: schema\ntypes:\n  def: type-definition(required)\n");
    for (data, valid) in [
        ("def: [string, null]", true),
        ("def: [string, ~]", true),
        ("def: \"null(required)\"", true),
        ("def: \"alphanumeric(allow-dashes; min(1))\"", true),
        ("def: \"alphanumeric(allow-dashes, min(1))\"", false),
        ("def: [string, 5]", false),
    ] {
        assert_eq!(validates(dir.path(), "./meta.yaml", data), valid, "{data}");
    }
}
