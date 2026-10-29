//! Executable contracts established in Phase 1 for the SimplifiedSchema
//! semantic meta-types.

use std::fs;

use biscuit_file::FileReference;
use darkmatter::markdown::{
    Markdown,
    schemas::{
        Constraint, DarkmatterSchemas, PropertyDef, SimplifiedSchema, TypeExpr,
        ValidationReport, darkmatter_base_json_schema, parse_standalone_schema_document,
        parse_yaml_schema, schema_type_descriptors,
        simplified::{grammar::MAX_INLINE_OBJECT_DEPTH, grammar::parse_type_expr,
            parse_yaml_schema_with_source, serialize_property_atom},
    },
};
use serde_json::{Value, json};

fn validate_candidate(semantic_type: &str, candidate_yaml: &str) -> ValidationReport {
    let source = format!(
        "---\n$schema:\n  candidate: {semantic_type}(required)\n{candidate_yaml}---\nBody\n"
    );
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .validate(&Markdown::from(source.as_str()))
        .expect("the semantic schema must compile and validation must run")
}

fn assert_valid_candidate(semantic_type: &str, candidate_yaml: &str) {
    let report = validate_candidate(semantic_type, candidate_yaml);
    assert!(report.valid, "candidate should be valid: {:?}", report.problems);
}

fn assert_invalid_candidate(semantic_type: &str, candidate_yaml: &str) {
    let report = validate_candidate(semantic_type, candidate_yaml);
    assert!(!report.valid, "candidate should be rejected: {candidate_yaml:?}");
}

fn schema_property_contains(value: &Value, property: &str, key: &str) -> bool {
    let Some(property_schema) = value.get("properties").and_then(|v| v.get(property)) else {
        return false;
    };
    fn contains(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(map) => {
                map.contains_key(key) || map.values().any(|value| contains(value, key))
            }
            Value::Array(items) => items.iter().any(|value| contains(value, key)),
            _ => false,
        }
    }
    contains(property_schema, key)
}

#[test]
fn type_definition_keyword_round_trips_and_reaches_the_catalog() {
    for keyword in ["type-definition", "type-definition[]"] {
        let atom = parse_type_expr("candidate", keyword).expect("keyword must parse");
        assert_eq!(serialize_property_atom(&atom), keyword);
    }
    assert!(
        schema_type_descriptors()
            .iter()
            .any(|descriptor| descriptor.keyword == "type-definition")
    );
}

#[test]
fn type_definition_validation_matches_the_property_definition_matrix() {
    for candidate in [
        "candidate: string(required)\n",
        "candidate: 'string(required)'\n",
        "candidate: \"string(required)\"\n",
        "candidate:\n  title: string(required)\n  metadata:\n    count: number\n",
        "candidate:\n  - literal(auto)\n  - number(min(1))\n  - width: number(required)\n",
    ] {
        assert_valid_candidate("type-definition", candidate);
    }

    for candidate in [
        "candidate: true\n",
        "candidate: 2\n",
        "candidate: null\n",
        "candidate: []\n",
        "candidate: [string, true]\n",
        "candidate: string(min(nope))\n",
        "",
    ] {
        assert_invalid_candidate("type-definition", candidate);
    }
}

/// A `narrows(...)` clause is checked by the same grammar behind the
/// `type-definition` meta-type: a valid clause validates, and an invalid one
/// is rejected with the grammar's own located message.
#[test]
fn type_definition_validates_narrows_clauses_with_the_grammar_messages() {
    let function = |clause: &str| {
        format!("candidate: 'function(parameters(x: unknown | null); returns(boolean); {clause})'\n")
    };
    for clause in ["narrows(x: string; true-iff)", "narrows(x: file; true-implies)", "narrows(x; false-implies)"] {
        assert_valid_candidate("type-definition", &function(clause));
    }
    for (clause, expected) in [
        ("narrows(x: string; true-sometimes)", "true-sometimes"),
        ("narrows(y: string; true-iff)", "does not declare"),
        ("narrows(x: string | number; true-implies)", "single type"),
    ] {
        let report = validate_candidate("type-definition", &function(clause));
        assert!(!report.valid, "{clause}");
        let problems = format!("{:?}", report.problems);
        assert!(problems.contains(expected), "{clause}: {problems}");
    }
    let non_boolean = "candidate: 'function(parameters(x: unknown | null); returns(string); narrows(x; false-implies))'\n";
    assert_invalid_candidate("type-definition", non_boolean);
}

/// The overload pairs whose `narrows` facts disagree, each beside a control
/// declaring the same fact on a second binding contract.
const NARROWS_FIRST_OVERLOAD: &str =
    "function(parameters(x: unknown | null); returns(boolean); narrows(x: string; true-iff))";
const NARROWS_AGREEING_OVERLOAD: &str =
    "function(parameters(x: unknown | null, y: string); returns(boolean); narrows(x: string; true-iff))";
const NARROWS_DISAGREEING_OVERLOADS: [(&str, &str); 4] = [
    ("absence", "function(parameters(x: unknown | null, y: string); returns(boolean))"),
    ("target", "function(parameters(x: unknown | null, y: string); returns(boolean); narrows(x: number; true-iff))"),
    ("kind", "function(parameters(x: unknown | null, y: string); returns(boolean); narrows(x: string; true-implies))"),
    ("position", "function(parameters(y: string, x: unknown | null); returns(boolean); narrows(x: string; true-iff))"),
];
const NARROWS_NOT_EXACT_FAMILIES: [&str; 3] = ["file", "string(min(3))", "enum(a,b)"];

fn narrows_function(target: &str, kind: &str) -> String {
    format!("function(parameters(x: unknown | null); returns(boolean); narrows(x: {target}; {kind}))")
}

/// The public type parser applies the declaration rules registration does:
/// a refinement or constrained target is one-way only, with the error at
/// the target.
#[test]
fn public_type_parser_rejects_true_iff_narrows_on_a_refinement() {
    for target in NARROWS_NOT_EXACT_FAMILIES {
        let text = narrows_function(target, "true-iff");
        let error = parse_type_expr("candidate", &text).expect_err(target);
        let darkmatter::markdown::schemas::SchemaError::Grammar { message, span, .. } = &error else {
            panic!("{target}: a grammar error, found {error:?}");
        };
        assert!(message.contains("true-iff"), "{target}: {message}");
        assert_eq!(&text[span.clone()], target, "{target}: located at the target");
        parse_type_expr("candidate", &narrows_function(target, "true-implies")).expect(target);
    }
    parse_type_expr("candidate", &narrows_function("string", "true-iff")).expect("control");
    parse_type_expr("candidate", &narrows_function("(string | number)[]", "true-implies"))
        .expect("a grouped union array is one array type");
}

/// Targets with a named type nested inside a compound type, each beside the
/// name the error must report. A fact is applied against the document's
/// definitions, so no target may depend on one.
const NARROWS_NESTED_NAME_TARGETS: [&str; 9] = [
    "{a: Name}",
    "tuple([Name])",
    "Name[]",
    "(Name | string)[]",
    "{a: string | Name}",
    "{<string>: Name}",
    "tuple([string, ...Name[]])",
    "{f: function(parameters(a: Name); returns(string))}",
    "{a: {b: Name@./other.yaml}}",
];
const NARROWS_SELF_CONTAINED_TARGETS: [&str; 6] =
    ["{a: string}", "tuple([string])", "string(min(3))", "enum(a,b)", "file", "(string|number)[]"];

/// The public type parser rejects a named type anywhere in a target, located
/// at the target and naming the type (not the union rule), and keeps every
/// self-contained target as a one-way fact.
#[test]
fn public_type_parser_rejects_named_types_nested_in_narrows_targets() {
    for target in NARROWS_NESTED_NAME_TARGETS {
        let text = narrows_function(target, "true-implies");
        let error = parse_type_expr("candidate", &text).expect_err(target);
        let darkmatter::markdown::schemas::SchemaError::Grammar { message, span, .. } = &error else {
            panic!("{target}: a grammar error, found {error:?}");
        };
        assert!(message.contains("named type `Name"), "{target}: {message}");
        assert!(!message.contains("union"), "{target}: {message}");
        assert_eq!(&text[span.clone()], target, "{target}: located at the target");
    }
    for target in NARROWS_SELF_CONTAINED_TARGETS {
        parse_type_expr("candidate", &narrows_function(target, "true-implies")).expect(target);
    }
}

/// Standalone schema documents and both meta-types reject the same nested
/// names, and the `schema` meta-type also rejects a refinement declared
/// `true-iff`.
#[test]
fn schema_consumers_reject_named_types_nested_in_narrows_targets() {
    let document = |definition: &str| format!("kind: schema\ntypes:\n  pred: |-\n    {definition}\n");
    for target in ["{a: Name}", "tuple([Name])", "(Name | string)[]"] {
        let definition = narrows_function(target, "true-implies");
        let error = parse_standalone_schema_document(&document(&definition), "narrows.yaml").expect_err(target);
        assert!(error.to_string().contains("named type `Name`"), "{target}: {error}");
        let report = validate_candidate("type-definition", &format!("candidate: '{definition}'\n"));
        assert!(!report.valid, "type-definition, {target}");
        assert!(format!("{:?}", report.problems).contains("named type `Name`"), "{target}");
        assert_invalid_candidate("schema", &format!("candidate:\n  pred: '{definition}'\n"));
    }
    assert_valid_candidate("schema", &format!("candidate:\n  pred: '{}'\n", narrows_function("{a: string}", "true-implies")));
    assert_valid_candidate("schema", &format!("candidate:\n  pred: '{}'\n", narrows_function("file", "true-implies")));
    let report = validate_candidate("schema", &format!("candidate:\n  pred: '{}'\n", narrows_function("file", "true-iff")));
    assert!(!report.valid, "schema meta-type, file; true-iff");
    assert!(format!("{:?}", report.problems).contains("true-iff"), "{:?}", report.problems);
}

/// A standalone schema document rejects the same refinements and every
/// overload disagreement on the fact, and accepts two binding contracts
/// declaring the same fact.
#[test]
fn standalone_schema_parser_rejects_invalid_narrows_declarations() {
    let document = |definition: &str| format!("kind: schema\ntypes:\n  pred: |-\n    {definition}\n");
    let parse = |definition: &str| parse_standalone_schema_document(&document(definition), "narrows.yaml");
    for target in NARROWS_NOT_EXACT_FAMILIES {
        let error = parse(&narrows_function(target, "true-iff")).expect_err(target);
        assert!(error.to_string().contains("true-iff"), "{target}: {error}");
        assert!(parse(&narrows_function(target, "true-implies")).expect(target).is_some());
    }
    let control = parse(&format!("{NARROWS_FIRST_OVERLOAD} | {NARROWS_AGREEING_OVERLOAD}")).expect("control");
    assert!(control.is_some());
    for (case, second) in NARROWS_DISAGREEING_OVERLOADS {
        let error = parse(&format!("{NARROWS_FIRST_OVERLOAD} | {second}")).expect_err(case);
        assert!(error.to_string().contains("overload"), "{case}: {error}");
    }
    // The YAML-list spelling of the same overload set.
    let listed = |second: &str| {
        let source = format!("kind: schema\ntypes:\n  pred:\n    - '{NARROWS_FIRST_OVERLOAD}'\n    - '{second}'\n");
        parse_standalone_schema_document(&source, "narrows.yaml")
    };
    assert!(listed(NARROWS_AGREEING_OVERLOAD).expect("control").is_some());
    for (case, second) in NARROWS_DISAGREEING_OVERLOADS {
        assert!(listed(second).is_err(), "{case}, listed");
    }
}

/// The `type-definition` meta-type accepts what registration accepts and
/// rejects a refinement declared `true-iff` and disagreeing overloads.
#[test]
fn type_definition_rejects_narrows_facts_registration_rejects() {
    let candidate = |definition: &str| format!("candidate: '{definition}'\n");
    assert_valid_candidate("type-definition", &candidate(&narrows_function("string", "true-iff")));
    for target in ["file", "string(min(3))"] {
        let report = validate_candidate("type-definition", &candidate(&narrows_function(target, "true-iff")));
        assert!(!report.valid, "{target}");
        let problems = format!("{:?}", report.problems);
        assert!(problems.contains("true-iff"), "{target}: {problems}");
    }
    assert_valid_candidate(
        "type-definition",
        &candidate(&format!("{NARROWS_FIRST_OVERLOAD} | {NARROWS_AGREEING_OVERLOAD}")),
    );
    for (case, second) in NARROWS_DISAGREEING_OVERLOADS {
        let report = validate_candidate("type-definition", &candidate(&format!("{NARROWS_FIRST_OVERLOAD} | {second}")));
        assert!(!report.valid, "{case}");
    }
}

#[test]
fn schema_validation_is_syntax_only_and_rejects_remote_references() {
    for reference in ["./schemas/does-not-exist.yaml", "does-not-exist.yaml"] {
        let parsed = FileReference::new(reference).expect("local reference syntax");
        assert_eq!(parsed.raw(), reference);
        assert_valid_candidate("schema", &format!("candidate: {reference}\n"));
    }
    for candidate in [
        "candidate:\n  title: string(required)\n",
        "candidate:\n  - ./schemas/does-not-exist.yaml\n  - kind: literal(review)\n",
    ] {
        assert_valid_candidate("schema", candidate);
    }

    for candidate in [
        "candidate: https://example.com/schema.yaml\n",
        "candidate: http://example.com/schema.yaml\n",
        "candidate: true\n",
        "candidate: 2\n",
        "candidate: null\n",
        "candidate: []\n",
        "candidate: [./schema.yaml, true]\n",
    ] {
        assert_invalid_candidate("schema", candidate);
    }
}

#[test]
fn semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips() {
    let source = concat!(
        "---\n$schema:\n",
        "  one_union: type-definition(required)\n",
        "  many_definitions: type-definition[](required)\n",
        "  one_schema_union: schema(required)\n",
        "  many_schemas: schema[](required)\n",
        "one_union: [string, number]\n",
        "many_definitions:\n  - string\n  - number\n  - [literal(auto), number]\n",
        "one_schema_union: [./missing-a.yaml, { title: string }]\n",
        "many_schemas:\n  - { title: string }\n  - [./missing-b.yaml, { kind: literal(review) }]\n",
        "---\nBody\n",
    );
    let original = Markdown::from(source);
    let report = DarkmatterSchemas::new(crate::request_support::cwd_context()).validate(&original).expect("validation must run");
    assert!(report.valid, "array/union matrix must validate: {:?}", report.problems);

    let expected = original.frontmatter().as_map().clone();
    let dir = tempfile::tempdir().expect("tempdir");
    let first_path = dir.path().join("first.md");
    let second_path = dir.path().join("second.md");
    fs::write(&first_path, original.as_string()).expect("first write");
    let first = Markdown::try_from(first_path.as_path()).expect("first read");
    assert_eq!(first.frontmatter().as_map(), &expected);
    fs::write(&second_path, first.as_string()).expect("second write");
    let second = Markdown::try_from(second_path.as_path()).expect("second read");
    assert_eq!(second.frontmatter().as_map(), &expected);
}

#[test]
fn existing_source_projection_covers_union_quote_crlf_and_utf8_variants() {
    let source = concat!(
        "plain: string(suggest(alpha, café))\r\n",
        "single: 'string(suggest(beta, ''café''))'\r\n",
        "double: \"string(suggest(gamma, 'caf\\u00e9'))\"\r\n",
        "union:\r\n",
        "  - string\r\n",
        "  - nested: string(suggest(delta, café))\r\n",
    );
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).expect("YAML");
    let schema = parse_yaml_schema_with_source(&yaml, source, 11).expect("source projection");
    let SimplifiedSchema::Single(shape) = schema else {
        panic!("expected one schema shape");
    };
    let mut atoms = Vec::new();
    for name in ["plain", "single", "double"] {
        let PropertyDef::Single(atom) = &shape.properties[name] else {
            panic!("expected a scalar definition for {name}");
        };
        atoms.push(atom);
    }
    let PropertyDef::Union(arms) = &shape.properties["union"] else {
        panic!("expected a property union");
    };
    let TypeExpr::InlineObject(nested) = &arms[1].ty else {
        panic!("expected a nested mapping union arm");
    };
    let PropertyDef::Single(nested) = &nested.properties["nested"] else {
        panic!("expected a nested scalar definition");
    };
    atoms.push(nested);

    for arm in atoms {
        let Constraint::Suggest(candidates) = arm
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .expect("suggestion constraint")
        else {
            unreachable!()
        };
        let span = candidates.last().expect("candidate").span.clone();
        let raw = &source[span.start - 11..span.end - 11];
        assert!(raw.contains("caf"), "projected authored span: {raw:?}");
    }
}

fn nested_mapping(depth: usize) -> String {
    assert!(depth > 0);
    let mut source = String::from("root:\n");
    for level in 0..depth - 1 {
        source.push_str(&"  ".repeat(level + 1));
        source.push_str(&format!("level_{level}:\n"));
    }
    source.push_str(&"  ".repeat(depth));
    source.push_str("leaf: string\n");
    source
}

#[test]
fn native_mapping_depth_uses_the_shared_structured_limit() {
    let at_limit: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&nested_mapping(MAX_INLINE_OBJECT_DEPTH)).expect("YAML");
    parse_yaml_schema(&at_limit).expect("the documented boundary must remain valid");

    let over_limit: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&nested_mapping(MAX_INLINE_OBJECT_DEPTH + 1)).expect("YAML");
    let error = parse_yaml_schema(&over_limit).expect_err("over-limit mapping must fail");
    let message = error.to_string();
    assert!(message.contains("nest") && message.contains(&MAX_INLINE_OBJECT_DEPTH.to_string()));
}

#[test]
fn shipped_schema_corpus_is_passively_classified_without_resolution() {
    let root = biscuit_test_harness::manifest_dir!().join("../docs/schemas");
    let mut paths = fs::read_dir(&root)
        .expect("schema directory")
        .map(|entry| entry.expect("schema entry").path())
        .filter(|path| matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml")))
        .collect::<Vec<_>>();
    paths.sort();
    // The runtime baseline left this directory for the global catalog's
    // `fm` partial (`darkmatter/schemas/partials/fm.yaml`).
    assert_eq!(paths.len(), 3, "every shipped YAML artifact must remain in the corpus");

    let mut schemas = Vec::new();
    let mut other_artifacts = Vec::new();
    let mut rejected_claims = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).expect("read shipped artifact");
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        match parse_standalone_schema_document(&source, &path) {
            Ok(Some(_)) => schemas.push(name),
            Ok(None) => other_artifacts.push(name),
            Err(_) => rejected_claims.push(name),
        }
    }
    assert_eq!(schemas, ["env.yaml"]);
    assert_eq!(other_artifacts, ["schema-definition.yaml"]);
    assert_eq!(rejected_claims, ["err.yaml"]);
}

#[test]
fn shipped_base_schema_retypes_schema_and_preserves_resolution_acceptance() {
    let compiled = darkmatter_base_json_schema();
    assert!(schema_property_contains(&compiled, "$schema", "x-darkmatter-schema"));

    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("simplified.yaml"), "$schema:\n  title: string(required)\n")
        .expect("write simplified schema");
    fs::write(
        dir.path().join("raw.json"),
        r#"{"type":"object","properties":{"title":{"type":"string"}}}"#,
    )
    .expect("write raw JSON Schema");

    let simplified_ref = FileReference::new("./simplified.yaml").expect("reference syntax");
    let raw_ref = FileReference::new("./raw.json").expect("reference syntax");
    for declaration in [
        json!({ "title": "string(required)" }),
        json!(simplified_ref.raw()),
        json!([simplified_ref.raw(), { "kind": "literal(review)" }]),
        json!(raw_ref.raw()),
    ] {
        darkmatter::markdown::schemas::resolve::resolve_schema_with_roots(
            &declaration,
            dir.path(),
            &[],
            &biscuit_file::FileResolutionContext::new(dir.path()),
        )
        .expect("existing declaration shape must still resolve");
    }

    let api = DarkmatterSchemas::new(crate::request_support::cwd_context())
        .with_darkmatter_baseline_json_schema()
        .expect("shipped base schema");
    let seed = Markdown::from("---\ntitle: Seed\n---\nBody\n");
    let effective = api
        .effective_for(&seed)
        .expect("baseline preparation")
        .expect("baseline schema");
    let malformed = json!({ "$schema": true });
    let report = effective.validate(&malformed);
    assert!(!report.valid, "validation-only use must reject malformed $schema early");
    assert!(
        darkmatter::markdown::schemas::resolve::resolve_schema_with_roots(
            &json!(true),
            dir.path(),
            &[],
            &biscuit_file::FileResolutionContext::new(dir.path()),
        )
        .is_err(),
        "full preparation must continue rejecting the same malformed declaration",
    );
}

/// A duplicate `narrows` clause is located at the second keyword and relates
/// the whole first clause, as structured spans rather than message text.
#[test]
fn public_parser_relates_a_duplicate_narrows_clause_to_the_first() {
    use darkmatter::markdown::schemas::{SchemaError, simplified::grammar::parse_type_definition};
    let first = "narrows(x: string; true-iff)";
    let text = format!("function(parameters(x: unknown | null); returns(boolean); {first}; narrows(x; false-implies))");
    let error = parse_type_definition("pred", &text).expect_err("two clauses");
    let SchemaError::Grammar { message, span, related, .. } = &error else {
        panic!("a grammar error, found {error:?}");
    };
    assert!(message.contains("duplicate `narrows`") && !message.contains("byte"), "{message}");
    assert_eq!(span.start, text.rfind("narrows").unwrap(), "at the second keyword");
    assert_eq!(&text[span.clone()], "narrows");
    let [related] = related.as_slice() else { panic!("one related location: {related:?}") };
    assert_eq!(&text[related.span.clone()], first);
    assert_eq!(related.arm, None);
}

/// Overloads that disagree on the fact name both declarations: different
/// facts are located at the later clause and relate the earlier one; a
/// missing fact is located at the declaring clause and relates the overload
/// that declares none. In one type expression both spans are in its text; in
/// a YAML sequence union each is in its own option, and the source-aware
/// parser projects both into the authored file.
#[test]
fn public_parsers_relate_disagreeing_narrows_overloads_to_both_declarations() {
    use darkmatter::markdown::schemas::{
        SchemaError, parse_property_definition_with_source, simplified::grammar::parse_type_definition,
    };
    let declaring = "function(parameters(x: unknown | null); returns(boolean); narrows(x: string; true-iff))";
    let first_clause = "narrows(x: string; true-iff)";
    let different = "function(parameters(x: unknown | null, y: string); returns(boolean); narrows(x: number; true-iff))";
    let absent = "function(parameters(x: unknown | null, y: string); returns(boolean))";

    let text = format!("{declaring} | {different}");
    let Err(SchemaError::Grammar { span, related, .. }) = parse_type_definition("pred", &text) else {
        panic!("different facts are a grammar error");
    };
    assert_eq!(span.start, text.rfind("narrows").unwrap(), "at the later clause");
    let [related] = related.as_slice() else { panic!("one related location: {related:?}") };
    assert_eq!(&text[related.span.clone()], first_clause);
    assert!(related.message.contains("overload 1"), "{}", related.message);

    let text = format!("{absent} | {declaring}");
    let Err(SchemaError::Grammar { message, span, related, .. }) = parse_type_definition("pred", &text) else {
        panic!("a missing fact is a grammar error");
    };
    assert!(message.contains("overload 2 declares") && message.contains("overload 1 does not"), "{message}");
    assert_eq!(&text[span.clone()], first_clause, "at the declaring clause");
    let [related] = related.as_slice() else { panic!("one related location: {related:?}") };
    assert_eq!(&text[related.span.clone()], absent, "the overload without a clause");

    // A YAML sequence union: the related location is another option.
    for (later, related_text) in [(different, first_clause), (absent, absent)] {
        let source = format!("- '{declaring}'\n- '{later}'\n");
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).unwrap();
        let failure = parse_property_definition_with_source("pred", &value, &source, 0).expect_err(later);
        let SchemaError::Grammar { related: structured, .. } = &*failure.error else {
            panic!("a grammar error, found {:?}", failure.error);
        };
        let [structured] = structured.as_slice() else { panic!("one related location: {structured:?}") };
        let primary = failure.authored_span.clone().expect("the primary token is located");
        let [located] = failure.related.as_slice() else { panic!("one projected location: {:?}", failure.related) };
        if later == different {
            assert_eq!(structured.arm, Some(0), "the earlier option");
            assert_eq!(primary.start, source.rfind("narrows").unwrap(), "at the later option's clause");
            assert_eq!(&source[located.span.clone()], related_text);
        } else {
            assert_eq!(structured.arm, Some(1), "the option without a clause");
            assert_eq!(&source[primary], first_clause, "at the declaring clause");
            assert_eq!(&source[located.span.clone()], related_text);
        }
    }
}
