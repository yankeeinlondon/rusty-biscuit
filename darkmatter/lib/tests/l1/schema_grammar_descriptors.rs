//! The SimplifiedSchema grammar's tuples, description-only functions, `|`
//! unions with grouping, `category(...)`, and the `categories:` list: what
//! parses, what is rejected (with the offending span), how tuples and unions
//! lower to Draft 2020-12, how documents validate against them, and that a
//! document schema can never reach a function.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
use darkmatter::markdown::schemas::simplified::grammar::{parse_type_definition, parse_type_expr};
use darkmatter::markdown::schemas::simplified::{ExampleOutcome, FunctionParameters, serialize_property_def};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, PropertyDef, SchemaError, TypeExpr, ValidationReport,
    parse_standalone_schema_document, parse_yaml_schema, to_json_schema,
};
use insta::assert_snapshot;
use serde_json::{Value, json};

use crate::request_support::context_at;

// ── Helpers ────────────────────────────────────────────────────────────────

fn definition(text: &str) -> PropertyDef {
    parse_type_definition("p", text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

/// The grammar error for `text`: its message and the authored text its span
/// covers.
fn rejection(text: &str) -> (String, String) {
    match parse_type_definition("p", text) {
        Err(SchemaError::Grammar { message, span, .. }) => (message, text[span].to_string()),
        other => panic!("{text}: expected a grammar error, got {other:?}"),
    }
}

/// The JSON Schema of a `$schema` mapping body.
fn lowered(schema_body: &str) -> Value {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(schema_body).expect("schema YAML");
    to_json_schema(&parse_yaml_schema(&yaml).expect("schema parses")).expect("schema lowers")
}

fn pretty(schema_body: &str) -> String {
    serde_json::to_string_pretty(&lowered(schema_body)).expect("serialize")
}

/// Validates a Markdown document in `dir` read-only.
fn validate_in(dir: &Path, document: &str) -> Result<ValidationReport, SchemaError> {
    let markdown = Markdown::from(document.to_string()).with_source(ComposeSource::File(dir.join("doc.md")));
    DarkmatterSchemas::new(context_at(dir)).validate(&markdown)
}

/// Whether `value` (a YAML flow value) satisfies the one-property schema
/// `v: <type_text>` of a fixture document.
fn accepts(type_text: &str, value: &str) -> bool {
    let dir = tempfile::tempdir().expect("temp dir");
    let document = format!("---\n$schema:\n  v: '{type_text}'\nv: {value}\n---\nBody\n");
    validate_in(dir.path(), &document)
        .unwrap_or_else(|error| panic!("`{type_text}` must load: {error}"))
        .valid
}

// ── Criterion 1: every form parses ─────────────────────────────────────────

#[test]
fn tuple_forms_parse() {
    let PropertyDef::Single(atom) = definition("tuple([string, number, boolean])") else {
        panic!("one atom");
    };
    let TypeExpr::Tuple(fixed) = &atom.ty else { panic!("tuple") };
    assert_eq!((fixed.elements.len(), fixed.required_len(), fixed.spread.is_some()), (3, 3, false));

    let PropertyDef::Single(atom) = definition("tuple([string, number?, boolean?])") else { panic!() };
    let TypeExpr::Tuple(optional) = &atom.ty else { panic!("tuple") };
    assert_eq!(optional.required_len(), 1);
    assert!(optional.elements[1].optional && optional.elements[2].optional);

    let PropertyDef::Single(atom) = definition("tuple([string, ...number[]])") else { panic!() };
    let TypeExpr::Tuple(variadic) = &atom.ty else { panic!("tuple") };
    let spread = variadic.spread.as_deref().expect("spread");
    assert!(spread.is_array, "the spread is an array type");

    let PropertyDef::Single(atom) = definition("tuple([...(string | number)[]])") else { panic!() };
    let TypeExpr::Tuple(grouped) = &atom.ty else { panic!("tuple") };
    assert!(matches!(grouped.spread.as_deref().map(|spread| &spread.ty), Some(TypeExpr::Union(arms)) if arms.len() == 2));

    let PropertyDef::Single(atom) = definition("tuple([string, number | boolean])") else { panic!() };
    let TypeExpr::Tuple(union) = &atom.ty else { panic!("tuple") };
    assert!(matches!(&union.elements[1].def, PropertyDef::Union(arms) if arms.len() == 2));

    let PropertyDef::Single(atom) = definition("tuple([string, (number | boolean)?])") else { panic!() };
    let TypeExpr::Tuple(grouped_optional) = &atom.ty else { panic!("tuple") };
    assert!(grouped_optional.elements[1].optional);

    let PropertyDef::Single(atom) = definition("tuple([])") else { panic!() };
    assert!(matches!(&atom.ty, TypeExpr::Tuple(empty) if empty.elements.is_empty() && empty.spread.is_none()));

    let PropertyDef::Single(atom) = definition("tuple([string, number?]; required)") else { panic!() };
    assert!(atom.is_required());
}

#[test]
fn function_type_with_every_constraint_parses() {
    let text = "function(\n    parameters(name: string, age?: number | null, ...tags: string);\n    \
                returns(boolean | enum(banned, underage));\n    category(Date Arithmetic);\n    fallible;\n    \
                example(check(\"a\") => true);\n    example(check(\"a\", 2) => {ok: [1, 2.5]});\n    \
                example(check(\"b\") => error; display-only: needs network);\n    example(./check.yaml);\n) -> Checks a name.";
    let PropertyDef::Single(atom) = definition(text) else { panic!("one atom") };
    let TypeExpr::Function(function) = &atom.ty else { panic!("function") };
    let FunctionParameters::List(parameters) = &function.parameters else { panic!("parameters") };
    let shape: Vec<(&str, bool, bool)> = parameters
        .iter()
        .map(|parameter| (parameter.name.as_str(), parameter.optional, parameter.rest))
        .collect();
    assert_eq!(shape, [("name", false, false), ("age", true, false), ("tags", false, true)]);
    assert!(matches!(&parameters[1].def, PropertyDef::Union(arms) if arms.len() == 2));
    assert!(matches!(&function.returns, PropertyDef::Union(arms) if arms.len() == 2));
    assert!(function.fallible);
    assert_eq!(function.examples.len(), 3);
    assert_eq!(function.examples[0].call, "check(\"a\")");
    assert_eq!(function.examples[0].expected, ExampleOutcome::Value(json!(true)));
    assert_eq!(function.examples[1].expected, ExampleOutcome::Value(json!({ "ok": [1, 2.5] })));
    assert_eq!(function.examples[2].expected, ExampleOutcome::Error);
    assert_eq!(function.examples[2].display_only.as_deref(), Some("needs network"));
    assert_eq!(&text[function.examples[0].call_span.clone()], "check(\"a\")");
    assert_eq!(atom.description.as_deref(), Some("Checks a name."));
    assert!(atom.constraints.contains(&darkmatter::markdown::schemas::Constraint::Category("Date Arithmetic".into())));
    assert!(atom.constraints.contains(&darkmatter::markdown::schemas::Constraint::Example(vec!["./check.yaml".into()])));

    let PropertyDef::Single(void) = definition("function(parameters(void); returns(string))") else { panic!() };
    assert!(matches!(&void.ty, TypeExpr::Function(function) if function.parameters == FunctionParameters::Void));
}

#[test]
fn union_operator_parses_everywhere_a_type_is_written() {
    assert!(matches!(definition("string | number"), PropertyDef::Union(arms) if arms.len() == 2));
    assert!(matches!(definition("{ a: string | number }"), PropertyDef::Single(atom)
        if matches!(&atom.ty, TypeExpr::InlineObject(shape) if matches!(shape.properties["a"], PropertyDef::Union(_)))));
    let PropertyDef::Single(list) = definition("(string | number)[](min(1))") else { panic!("one atom") };
    assert!(list.is_array && matches!(&list.ty, TypeExpr::Union(arms) if arms.len() == 2));
    let PropertyDef::Single(function) = definition("function(parameters(a: string | number); returns(string[] | null))") else {
        panic!("one atom");
    };
    let TypeExpr::Function(function) = &function.ty else { panic!("function") };
    assert!(matches!(&function.parameters.as_slice()[0].def, PropertyDef::Union(_)));
    assert!(matches!(&function.returns, PropertyDef::Union(_)));
    // A parenthesized group without `[]` joins the enclosing union.
    assert_eq!(definition("(string | number) | boolean"), definition("string | number | boolean"));
}

#[test]
fn categories_list_parses_in_order_with_optional_multi_line_descriptions() {
    let source = "kind: schema\ncategories:\n  - Math -> Arithmetic on numbers.\n  - Devops\n  - |-\n    Filesystem ->\n    Questions about files\n    and paths.\ntypes:\n  size: 'number(category(Filesystem))'\n";
    let document = parse_standalone_schema_document(source, "/w/types.yaml")
        .expect("loads")
        .expect("claimed");
    let categories: Vec<(&str, Option<&str>)> = document
        .categories
        .as_deref()
        .expect("declared")
        .iter()
        .map(|category| (category.name.as_str(), category.description.as_deref()))
        .collect();
    assert_eq!(
        categories,
        [
            ("Math", Some("Arithmetic on numbers.")),
            ("Devops", None),
            ("Filesystem", Some("Questions about files\nand paths.")),
        ]
    );
    let undeclared = parse_standalone_schema_document("kind: schema\ntypes:\n  a: 'string(category(Anything))'\n", "/w/t.yaml")
        .expect("loads")
        .expect("claimed");
    assert!(undeclared.categories.is_none(), "no list means any name is accepted");
}

// ── Criteria 1 and 56: rejections, with the offending span ────────────────

#[test]
fn invalid_tuples_and_repeated_categories_are_rejected_at_the_offending_text() {
    let cases: &[(&str, &str, &str)] = &[
        ("tuple([...number[], ...string[]])", "only one spread", "."),
        ("tuple([...number[], string])", "a spread must be the last", "s"),
        ("tuple([number?, string])", "required tuple element cannot follow an optional", "string"),
        ("tuple([...number[]?])", "`?` cannot follow a spread", "?"),
        ("tuple([name: string])", "cannot be named", "name"),
        ("tuple([name?: string])", "cannot be named", "name"),
        ("tuple([...number])", "must be an array type", "...number"),
        ("tuple([string | number?])", "group a union before `?`", "?"),
        ("tuple([string?, | number])", "unexpected character `|`", "|"),
        ("tuple(string, number)", "bracketed element list", ""),
        ("tuple([string]; min(1))", "constraint `min` is not valid on `tuple`", "tuple"),
        ("string(category(A); category(B))", "at most one `category`", "string(category(A); category(B))"),
        ("enum(a, b; category(A); category(B))", "at most one `category`", "enum(a, b; category(A); category(B))"),
    ];
    for (text, message, covered) in cases {
        match parse_type_definition("p", text) {
            Err(SchemaError::Grammar { message: actual, span, .. }) => {
                assert!(actual.contains(message), "{text}: {actual}");
                if !covered.is_empty() {
                    assert_eq!(&text[span], *covered, "{text}: span of {actual}");
                }
            }
            other => panic!("{text}: expected a grammar error, got {other:?}"),
        }
    }
}

#[test]
fn invalid_function_types_are_rejected_at_the_offending_text() {
    let cases: &[(&str, &str, &str)] = &[
        ("function(returns(string))", "requires `parameters(...)`", "function"),
        ("function(parameters(void))", "requires `returns(...)`", "function"),
        ("function(parameters(void); returns(string); pure)", "unknown constraint `pure`", "pure"),
        ("function(parameters(void); returns(string); required)", "unknown constraint `required`", "required"),
        ("function(parameters(void); parameters(void); returns(string))", "duplicate `parameters`", "parameters"),
        ("function(parameters(void); returns(string); returns(number))", "duplicate `returns`", "returns"),
        ("function(parameters(void); returns(string); category(A); category(B))", "duplicate `category`", "category"),
        ("function(parameters(void); returns(string); fallible; fallible)", "duplicate `fallible`", "fallible"),
        ("function(parameters(a: string, a: number); returns(string))", "duplicate parameter name `a`", "a"),
        ("function(parameters(a?: string, b: number); returns(string))", "required parameter cannot follow", "b: number"),
        ("function(parameters(...a: string, ...b: string); returns(string))", "only one rest parameter", "...b: string"),
        ("function(parameters(...a: string, b: string); returns(string))", "rest parameter must be the last", "b: string"),
        ("function(parameters(...a?: string); returns(string))", "rest parameter cannot be optional", "?"),
        ("function(parameters(void, a: string); returns(string))", "`void` cannot be mixed", "void"),
        ("function(parameters(a: string, void); returns(string))", "`void` cannot be mixed", "void"),
        ("function(parameters(); returns(string))", "`parameters()` is not valid", "parameters()"),
        ("function(parameters(a: string(optional)); returns(string))", "unknown constraint `optional`", "optional"),
        ("function(parameters(void); returns(string); example(3 => 3))", "must be a function call", "3"),
        ("function(parameters(void); returns(string); example(f() => pear))", "must be a literal", "pear"),
        ("function(parameters(void); returns(string); example(f() => 1; slow))", "display-only: REASON", "slow"),
        ("function(parameters(void); returns(string); example(f() => 9007199254740993))", "cannot be represented exactly", "9007199254740993"),
        ("string(example(f() => 1))", "valid only on a `function` type", "example(f() => 1)"),
        (
            "function(parameters(void); returns(string))[](example(f() => 1))",
            "valid only on a `function` type",
            "example(f() => 1)",
        ),
        ("function", "requires a signature", "function"),
    ];
    for (text, message, covered) in cases {
        match parse_type_definition("p", text) {
            Err(SchemaError::Grammar { message: actual, span, .. }) => {
                assert!(actual.contains(message), "{text}: {actual}");
                assert_eq!(&text[span], *covered, "{text}: span of {actual}");
            }
            other => panic!("{text}: expected a grammar error, got {other:?}"),
        }
    }
}

#[test]
fn a_union_inside_a_yaml_list_arm_and_a_question_mark_on_a_property_are_rejected() {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str("id:\n  - string | number\n  - boolean\n").unwrap();
    let error = parse_yaml_schema(&yaml).expect_err("a list option is one type").to_string();
    assert!(error.contains("is itself a `|` union"), "{error}");
    let (message, covered) = rejection("string?");
    assert!(message.contains("optional unless it is `required`"), "{message}");
    assert_eq!(covered, "?");
}

// ── Serialization round trip ──────────────────────────────────────────────

#[test]
fn every_new_form_round_trips_through_serialization() {
    for text in [
        "string | number[]",
        "(string | number)[](min(1); unique)",
        "tuple([string, number?, (boolean | null)?, ...(string | number)[]]; required; category(Pairs))",
        "tuple([])",
        "tuple([string])[](max(3))",
        "{ a: string | number, b: tuple([number, number]) }",
        "function(parameters(void); returns(string))",
        "function(parameters(name: string, age?: number | null, ...rest: any); returns(boolean | null); fallible; \
         example(f(\"a\", 1) => {a: [1, \"x\"]}); example(f() => error; display-only: needs network); \
         category(CI/CD); example(./f.yaml)) -> Describes f.",
        "function(parameters(cb: function(parameters(x: number); returns(number))); returns(tuple([number, string])))",
        "string(category(Text)) | number",
        "number(min(1e-3); max(100000000000000000000))",
    ] {
        let parsed = definition(text);
        let written = serialize_property_def(&parsed);
        let reparsed = parse_type_definition("p", &written).unwrap_or_else(|error| panic!("{text} -> {written}: {error}"));
        assert_eq!(reparsed, parsed, "{text} -> {written}");
        assert_eq!(serialize_property_def(&reparsed), written, "{text}: serialization is a fixed point");
    }
}

// ── Criterion 2: precedence and grouping, through validation ──────────────

#[test]
fn array_suffix_binds_tighter_than_the_union_operator() {
    assert!(accepts("string | number[]", "a"));
    assert!(accepts("string | number[]", "[1]"));
    assert!(!accepts("string | number[]", "[a]"));
    assert!(accepts("(string | number)[]", "[a, 1]"));
    assert!(!accepts("(string | number)[]", "a"));
}

// ── Criterion 3: lowering ─────────────────────────────────────────────────

#[test]
fn snapshot_fixed_tuple() {
    assert_snapshot!(pretty("point: 'tuple([number, number])'\n"));
}

#[test]
fn snapshot_variadic_tuple_with_optional_position() {
    assert_snapshot!(pretty("row: 'tuple([string, number?, ...boolean[]]; required)'\n"));
}

#[test]
fn snapshot_union_operator() {
    assert_snapshot!(pretty("id: 'string | number'\n"));
}

#[test]
fn snapshot_empty_tuple_and_category_annotation() {
    assert_snapshot!(pretty("none: 'tuple([])'\ntagged: 'string(category(Text)) -> A labeled value.'\n"));
}

#[test]
fn union_operator_and_yaml_list_are_the_same_definition() {
    let operator: serde_yaml_ng::Value = serde_yaml_ng::from_str("id: 'string | number'\n").unwrap();
    let list: serde_yaml_ng::Value = serde_yaml_ng::from_str("id: [string, number]\n").unwrap();
    assert_eq!(parse_yaml_schema(&operator).unwrap(), parse_yaml_schema(&list).unwrap());
    assert_eq!(lowered("id: 'string | number'\n"), lowered("id: [string, number]\n"));
}

/// Criterion 47: a grouped union list and a list of a union-typed named type
/// are one representation, lowered identically and validated alike.
///
/// The compiled schema rejects `[true]`; document validation still converts
/// `true` to `"true"` for a `string` option, which is the frontmatter
/// coercion's decision, not the grammar's.
#[test]
fn grouped_union_list_matches_a_named_union_list() {
    let dir = tempfile::tempdir().expect("temp dir");
    let resolve = |body: &str| {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(body).unwrap();
        resolve_yaml_schema(&yaml, dir.path(), &context_at(dir.path())).expect("resolves")
    };
    let grouped = resolve("tags: '(string | number)[]'\n");
    let named = resolve("t: [string, number]\ntags: 't[]'\n");
    let tags = &grouped.json_schema["properties"]["tags"];
    assert_eq!(tags, &named.json_schema["properties"]["tags"]);
    let validator = jsonschema::validator_for(tags).expect("the lowered list compiles");
    assert!(validator.is_valid(&json!(["a", 1])));
    assert!(!validator.is_valid(&json!([true])));
    for body in ["t: [string, number]\n  tags: 't[]'", "tags: '(string | number)[]'"] {
        let accepted = format!("---\n$schema:\n  {body}\ntags: [a, 1]\n---\nBody\n");
        let report = validate_in(dir.path(), &accepted).unwrap();
        assert!(report.valid, "{body}: {:?}", report.problems);
    }
}

#[test]
fn a_category_on_a_data_type_is_an_annotation() {
    let schema = lowered("label: 'string(category(Math))'\nlist: 'number[](category(Math))'\n");
    assert_eq!(schema["properties"]["label"]["x-darkmatter-category"], json!("Math"));
    assert_eq!(schema["properties"]["list"]["x-darkmatter-category"], json!("Math"));
}

// ── Tuple validation (criteria 4 and 56) ──────────────────────────────────

#[test]
fn tuples_validate_positions_omission_and_length() {
    assert!(accepts("tuple([])", "[]"));
    assert!(!accepts("tuple([])", "[1]"));
    assert!(accepts("tuple([string, number?])", "[a]"));
    assert!(accepts("tuple([string, number?])", "[a, 1]"));
    assert!(!accepts("tuple([string, number?])", "[a, null]"), "an optional slot rejects a supplied null");
    assert!(!accepts("tuple([string, number?])", "[a, 1, 2]"), "a fixed tuple has no extra positions");
    assert!(!accepts("tuple([string, number?])", "[]"), "the required position is required");
    assert!(accepts("tuple([string, number | null])", "[a, null]"), "a null-admitting type accepts null");
    assert!(accepts("tuple([string, ...number[]])", "[a, 1, 2, 3]"));
    assert!(!accepts("tuple([string, ...number[]])", "[a, b]"));
    assert!(accepts("tuple([number, number])", "null"), "the property itself stays optional");
}

// ── Criteria 4 and 56: a document schema never reaches a function ─────────

#[test]
fn a_document_schema_that_reaches_a_function_fails_to_load_naming_the_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("types.yaml"),
        "kind: schema\ntypes:\n  runner: 'function(parameters(void); returns(string))'\n  label: string\n",
    )
    .unwrap();
    let function = "function(parameters(void); returns(string))";
    let cases: Vec<(String, &str)> = vec![
        (format!("  run: '{function}'"), "run"),
        ("  run: 'runner@./types.yaml'".to_string(), "run"),
        (format!("  run: 'string | {function}'"), "run"),
        (format!("  run: '{function}[]'"), "run"),
        (format!("  run: 'tuple([string, {function}])'"), "run[1]"),
        ("  run: 'tuple([...(runner@./types.yaml)[]])'".to_string(), "run[...]"),
        (format!("  run:\n    inner: '{function}'"), "run.inner"),
        ("  run:\n    - string\n    - 'runner@./types.yaml'".to_string(), "run"),
    ];
    for (body, path) in cases {
        let document = format!("---\n$schema:\n{body}\n---\nBody\n");
        match validate_in(dir.path(), &document) {
            Err(SchemaError::Convert { property, message }) => {
                assert_eq!(property, path, "{body}");
                assert!(message.contains("description-only"), "{body}: {message}");
            }
            other => panic!("{body}: expected a load error naming `{path}`, got {other:?}"),
        }
    }
    // A data type from the same types file still imports.
    let document = "---\n$schema:\n  name: 'label@./types.yaml'\nname: x\n---\nBody\n";
    assert!(validate_in(dir.path(), document).expect("a data type imports").valid);
}

/// A function carrying a `narrows(...)` clause is still description-only: a
/// document schema reaching it fails at the property path, and the clause
/// does not change that outcome or the message.
#[test]
fn a_document_schema_that_reaches_a_narrowing_function_fails_naming_the_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let function = "function(parameters(x: unknown | null); returns(boolean); narrows(x: string; true-iff))";
    for (body, path) in [
        (format!("  run: '{function}'"), "run"),
        (format!("  run:\n    inner: '{function}[]'"), "run.inner"),
        (format!("  run: 'tuple([string, {function}])'"), "run[1]"),
    ] {
        let document = format!("---\n$schema:\n{body}\n---\nBody\n");
        match validate_in(dir.path(), &document) {
            Err(SchemaError::Convert { property, message }) => {
                assert_eq!(property, path, "{body}");
                assert!(message.contains("description-only"), "{body}: {message}");
            }
            other => panic!("{body}: expected a load error naming `{path}`, got {other:?}"),
        }
    }
    // The same function without the clause fails identically (control).
    let plain = "---\n$schema:\n  run: 'function(parameters(x: unknown | null); returns(boolean))'\n---\nBody\n";
    assert!(matches!(validate_in(dir.path(), plain), Err(SchemaError::Convert { .. })));
}

// ── Criterion 5: categories ───────────────────────────────────────────────

#[test]
fn a_declared_categories_list_limits_category_names() {
    let with_list = "kind: schema\ncategories:\n  - Math -> Arithmetic on numbers.\ntypes:\n  min: 'function(parameters(a: number, b: number); returns(number); category(Text))'\n";
    let error = parse_standalone_schema_document(with_list, "/w/types.yaml").expect_err("undeclared category");
    assert!(error.to_string().contains("`types.min`: category `Text` is not declared"), "{error}");

    let without = "kind: schema\ntypes:\n  min: 'function(parameters(a: number, b: number); returns(number); category(Text))'\n";
    assert!(parse_standalone_schema_document(without, "/w/types.yaml").expect("loads").is_some());

    for (categories, message) in [
        ("[]", "category `Text` is not declared"),
        ("[Text, ' Text ']", "`categories[1]`: duplicate category `Text`"),
        ("[Text, ' -> no name']", "`categories[1]`: a category name cannot be blank"),
        ("null", "`categories` must be a list"),
        ("[Text, 5]", "`categories[1]`: a category entry must be a string"),
        ("[Text ->]", "`categories[0]`: `->` must be followed by a description"),
    ] {
        let source = format!("kind: schema\ncategories: {categories}\ntypes:\n  t: 'string(category(Text))'\n");
        let error = parse_standalone_schema_document(&source, "/w/types.yaml").expect_err(categories);
        assert!(error.to_string().contains(message), "{categories}: {error}");
    }

    // A category nested in a tuple, parameter, or inline object is checked too.
    let nested = "kind: schema\ncategories: [Math]\ntypes:\n  t: 'tuple([string(category(Text))])'\n";
    let error = parse_standalone_schema_document(nested, "/w/types.yaml").expect_err("nested category");
    assert!(error.to_string().contains("category `Text`"), "{error}");
}

// ── Criterion 6: duplicate overloads ──────────────────────────────────────

#[test]
fn overloads_with_one_binding_contract_fail_to_load() {
    let overloads = |first: &str, second: &str| {
        format!("kind: schema\ntypes:\n  pr_list:\n    - '{first}'\n    - '{second}'\n")
    };
    let duplicates = [
        (
            "function(parameters(count: number(integer; min(1))); returns(string[]); category(Devops); fallible) -> Recent.",
            "function(parameters(n: number(min(1); integer)); returns(object)) -> Another description.",
        ),
        (
            "function(parameters(value: any); returns(string))",
            "function(parameters(other: unknown); returns(number))",
        ),
        (
            "function(parameters(value: string | number); returns(string))",
            "function(parameters(value: number | string); returns(string))",
        ),
    ];
    for (first, second) in duplicates {
        let error = parse_standalone_schema_document(&overloads(first, second), "/w/f.yaml")
            .expect_err("duplicate contracts");
        assert!(error.to_string().contains("the same binding contract"), "{error}");
    }
    let operator = "kind: schema\ntypes:\n  pr_list: 'function(parameters(a: any); returns(string)) | function(parameters(b: unknown); returns(string))'\n";
    assert!(parse_standalone_schema_document(operator, "/w/f.yaml").is_err(), "the `|` spelling");

    // Through a local named type: `Count` and its inline spelling are one contract.
    let named = "kind: schema\ntypes:\n  Count: 'number(integer; min(1))'\n  pr_list:\n    - 'function(parameters(count: Count); returns(string[]))'\n    - 'function(parameters(n: number(min(1); integer)); returns(string[]))'\n";
    let error = parse_standalone_schema_document(named, "/w/f.yaml").expect_err("resolved duplicate");
    assert!(error.to_string().contains("`types.pr_list`"), "{error}");

    for (first, second) in [
        (
            "function(parameters(count: number(integer; min(1))); returns(string[]))",
            "function(parameters(query: object); returns(string[]))",
        ),
        ("function(parameters(a: string); returns(string))", "function(parameters(a?: string); returns(string))"),
        ("function(parameters(a: string); returns(string))", "function(parameters(...a: string); returns(string))"),
        ("function(parameters(void); returns(string))", "function(parameters(a: string); returns(string))"),
    ] {
        assert!(
            parse_standalone_schema_document(&overloads(first, second), "/w/f.yaml")
                .expect("distinct contracts load")
                .is_some()
        );
    }
}

// ── Numeric metadata is admitted exactly ──────────────────────────────────

#[test]
fn numeric_metadata_rejects_what_an_f64_cannot_hold() {
    for (text, reason) in [
        ("number(min(inf))", "not finite"),
        ("number(max(-infinity))", "not finite"),
        ("number(min(1e400))", "not finite"),
        ("number(default(9007199254740993))", "cannot be represented exactly"),
        ("number(max(1e-400))", "a non-zero value would become 0"),
        ("enum(1, 9007199254740993)", "cannot be represented exactly"),
        ("literal(9007199254740993)", "cannot be represented exactly"),
    ] {
        let (message, _) = rejection(text);
        assert!(message.contains(reason), "{text}: {message}");
        let dir = tempfile::tempdir().expect("temp dir");
        let document = format!("---\n$schema:\n  n: '{text}'\n---\nBody\n");
        let error = validate_in(dir.path(), &document).expect_err(text).to_string();
        assert!(error.contains(reason), "{text} through document validation: {error}");
    }
    // Admitted whole numbers keep their exact value rather than saturating.
    let PropertyDef::Single(members) = definition("enum(100000000000000000000, 2.50)") else { panic!() };
    assert_eq!(
        members.constraints[0],
        darkmatter::markdown::schemas::Constraint::Members(vec!["100000000000000000000".into(), "2.5".into()])
    );
    let schema = lowered("n: 'number(min(100000000000000000000))'\nl: 'literal(100000000000000000000)'\n");
    assert_eq!(schema["properties"]["n"]["anyOf"][1]["minimum"], json!(1e20));
    assert_eq!(schema["properties"]["l"]["anyOf"][1]["const"], json!(1e20));
    // A quoted default is text, never a number.
    assert!(parse_type_expr("p", "string(default('9007199254740993'))").is_ok());
}

// ── The standalone classifier names a YAML error ──────────────────────────

#[test]
fn a_types_file_that_is_not_yaml_reports_its_yaml_error() {
    let source = "kind: schema\nkind: schema\ntypes:\n  a: string\n";
    match parse_standalone_schema_document(source, "/w/types.yaml") {
        Err(SchemaError::SchemaDocument { message, .. }) => {
            assert!(message.starts_with("not valid YAML") && message.contains("duplicate entry"), "{message}");
        }
        other => panic!("expected the YAML error, got {other:?}"),
    }
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("types.yaml"), source).unwrap();
    let error = validate_in(dir.path(), "---\n$schema: ./types.yaml\n---\nBody\n").expect_err("load fails");
    assert!(error.to_string().contains("not valid YAML"), "{error}");
}
