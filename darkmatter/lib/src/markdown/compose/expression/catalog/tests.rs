use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use super::*;
use crate::markdown::compose::expression::{EvaluationLookup, ResolutionContext, evaluate, parse};

const FIXTURE: &str = r#"kind: schema
categories:
    - Math -> Numbers.
    - Logical
types:
    ip-like: string
    round: |-
        function(
            parameters(x: numberlike, fallback?: number);
            returns(number);
            category(Math);
            example(round("2.5") => 3);
        ) -> Rounds.
    min: |-
        function(
            parameters(a: number, b: number);
            returns(number);
            category(Math);
            fallible;
            example(min(3, 5) => 3);
            example(min("a", 5) => error; display-only: shows the type error);
        ) -> The smaller number.
    upper: |-
        function(
            parameters(text: string);
            returns(string);
            category(Math);
            example(upper("a") => "A");
            example(./function-examples/as_tsv.yaml);
        ) -> Uppercases.
    and: |-
        function(
            parameters(...values: unknown | null);
            returns(boolean);
            category(Logical);
        ) -> Whether every value is truthy.
    is_string:
        - |-
            function(parameters(x: string); returns(boolean); category(Logical)) -> One.
        - |-
            function(parameters(x: number, y: number); returns(boolean); category(Logical)) -> Two.
"#;

fn fixture() -> FunctionCatalog {
    load_catalog(FIXTURE, "fixture.yaml", EXAMPLE_FILES).expect("the fixture loads")
}

fn by_signature<'a>(catalog: &'a FunctionCatalog, signature: &str) -> &'a ExpressionFunctionDescriptor {
    catalog
        .descriptors
        .iter()
        .find(|descriptor| descriptor.signature == signature)
        .unwrap_or_else(|| panic!("`{signature}` is cataloged"))
}

#[test]
fn a_document_without_categories_does_not_load() {
    let source = FIXTURE.replace("categories:\n    - Math -> Numbers.\n    - Logical\n", "");
    let error = load_catalog(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
    assert!(matches!(error, CatalogError::Invalid { .. }), "{error:?}");
    assert!(error.to_string().contains("categories"), "{error}");
}

#[test]
fn an_unlisted_category_is_the_standalone_parsers_error() {
    let source = FIXTURE.replacen("category(Math);", "category(Geometry);", 1);
    let error = load_catalog(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
    assert!(matches!(error, CatalogError::Schema(_)), "{error:?}");
    assert!(error.to_string().contains("Geometry"), "{error}");
}

/// Criterion 57: two overloads that differ only in parameter names, and in
/// spelling a named type out, have one binding contract, so the catalog does
/// not load. (Criterion 6 covers the standalone-document check this path
/// reaches: `schema_grammar_descriptors.rs`.)
#[test]
fn semantically_duplicate_overloads_fail_catalog_load() {
    let source = FIXTURE.replacen(
        "            function(parameters(x: number, y: number); returns(boolean); category(Logical)) -> Two.",
        "            function(parameters(y: ip-like); returns(string); category(Logical)) -> Two.",
        1,
    );
    let error = load_catalog(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
    assert!(matches!(error, CatalogError::Schema(_)), "{error:?}");
    assert!(error.to_string().contains("the same binding contract"), "{error}");
}

/// Ruling R5: the catalog loader, unlike a general schema file, requires
/// `kind: schema` and never reads a null entry as the `null` type.
#[test]
fn a_missing_null_or_wrong_kind_does_not_load() {
    for kind in ["", "kind: null\n", "kind: ~\n", "kind: \"\"\n", "kind: trigger\n", "kind: 5\n"] {
        let source = FIXTURE.replacen("kind: schema\n", kind, 1);
        let error = load_catalog(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
        assert!(error.to_string().contains("kind: schema"), "{kind:?}: {error}");
    }
    let duplicate = FIXTURE.replacen("kind: schema\n", "kind: schema\nkind: schema\n", 1);
    let trailing = format!("{FIXTURE}---\nkind: schema\n");
    let garbage = format!("{FIXTURE}  ] not yaml\n");
    for source in [duplicate, trailing, garbage] {
        assert!(load_catalog(&source, "fixture.yaml", EXAMPLE_FILES).is_err(), "{source}");
    }
}

#[test]
fn an_entry_written_as_null_does_not_load() {
    for value in [" null", " ~", ""] {
        let source = FIXTURE.replacen("    ip-like: string\n", &format!("    ip-like: string\n    round_down:{value}\n"), 1);
        let error = load_catalog(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
        assert!(matches!(error, CatalogError::Invalid { .. }), "{value:?}: {error:?}");
        assert!(error.to_string().contains("`types.round_down` is null, not a function signature"), "{value:?}: {error}");
    }
}

#[test]
fn a_non_function_entry_is_a_named_type_not_a_function() {
    let catalog = fixture();
    assert!(catalog.descriptors.iter().all(|descriptor| descriptor.name != "ip-like"));
    assert_eq!(catalog.named_type_names().collect::<Vec<_>>(), ["ip-like"]);
    assert!(catalog.named_type("ip-like").is_some());
}

#[test]
fn descriptors_follow_category_order_then_name_then_authored_overloads() {
    let catalog = fixture();
    let order: Vec<(&str, &str, usize, usize)> = catalog
        .descriptors
        .iter()
        .map(|descriptor| (descriptor.category, descriptor.name, descriptor.order, descriptor.overload))
        .collect();
    assert_eq!(
        order,
        [
            ("Math", "min", 1, 0),
            ("Math", "round", 2, 0),
            ("Math", "upper", 3, 0),
            ("Logical", "and", 1, 0),
            ("Logical", "is_string", 2, 0),
            ("Logical", "is_string", 3, 1),
        ]
    );
    assert_eq!(by_signature(&catalog, "is_string(x)").description, "One.");
    assert_eq!(by_signature(&catalog, "is_string(x, y)").description, "Two.");
    assert_eq!(
        catalog.categories,
        [
            CatalogCategory {
                name: "Math",
                description: Some("Numbers.")
            },
            CatalogCategory {
                name: "Logical",
                description: None
            },
        ]
    );
}

#[test]
fn registered_aliases_attach_and_lookups_resolve_alias_and_case() {
    let catalog = fixture();
    assert_eq!(by_signature(&catalog, "is_string(x)").aliases, ["isstring"]);
    assert_eq!(catalog.descriptors_for("isString").len(), 2);
    assert_eq!(catalog.descriptors_for("IS_STRING").len(), 2);
    assert!(catalog.descriptors_for("nope").is_empty());

    let overloads = function_descriptors("isString");
    assert!(!overloads.is_empty());
    assert!(overloads.iter().all(|descriptor| descriptor.name == "is_string"));
}

#[test]
fn typed_signatures_render_parameters_returns_and_fallibility() {
    let catalog = fixture();
    assert_eq!(by_signature(&catalog, "min(a, b)").typed_signature(), "min(a: number, b: number) -> number | error");
    assert_eq!(
        by_signature(&catalog, "round(x, [fallback])").typed_signature(),
        "round(x: numberlike, [fallback: number]) -> number"
    );
    assert_eq!(by_signature(&catalog, "and(...)").typed_signature(), "and(...values: unknown | null) -> boolean");
}

#[test]
fn examples_keep_their_authored_text_and_files_are_embedded() {
    let catalog = fixture();
    let min = by_signature(&catalog, "min(a, b)");
    assert_eq!(
        min.examples,
        [
            FunctionExample::Call {
                call: "min(3, 5)",
                expected: ExampleOutcome::Value(json!(3)),
                result: "3",
                display_only: None,
            },
            FunctionExample::Call {
                call: "min(\"a\", 5)",
                expected: ExampleOutcome::Error,
                result: "error",
                display_only: Some("shows the type error"),
            },
        ]
    );
    let example = min.example.expect("the first inline example");
    assert_eq!((example.invocation, example.result), ("min(3, 5)", "3"));
    assert_eq!(example.verification, ExampleVerification::Executable);

    let upper = by_signature(&catalog, "upper(text)");
    assert_eq!(upper.example.map(|example| example.result), Some("\"A\""));
    let FunctionExample::File {
        path,
        invocation,
        returns,
        ..
    } = &upper.examples[1]
    else {
        panic!("a file-form example: {:?}", upper.examples);
    };
    assert_eq!((*path, *invocation, *returns), ("function-examples/as_tsv.yaml", r#"as_tsv(["a", "b", "c"])"#, "a\tb\tc"));

    let error = load_catalog(FIXTURE, "fixture.yaml", &[]).unwrap_err();
    assert!(error.to_string().contains("is not embedded"), "{error}");
}

#[test]
fn the_embedded_catalog_loads_with_unique_display_keys() {
    let catalog = expression_function_catalog();
    assert!(!catalog.descriptors.is_empty());
    let mut seen = HashSet::new();
    for descriptor in catalog.descriptors {
        assert!(seen.insert(descriptor.signature), "duplicate display key `{}`", descriptor.signature);
        assert!(!descriptor.category.is_empty());
    }
    for category in catalog.categories {
        assert!(
            catalog.descriptors.iter().any(|descriptor| descriptor.category == category.name),
            "category `{}` lists no function",
            category.name
        );
    }
}

/// Every embedded example file is a valid example envelope.
#[test]
fn every_embedded_example_file_is_a_valid_example() {
    for (path, text) in EXAMPLE_FILES {
        crate::markdown::schemas::example::validate_example_bytes(path, text.as_bytes())
            .unwrap_or_else(|error| panic!("{path}: {error}"));
    }
}

/// The `agentic-cli` named type is the generated name table, in order.
#[test]
fn agentic_cli_named_type_is_the_generated_table() {
    use super::super::functions::agentic_cli_generated::AGENTIC_CLI_NAMES;
    use crate::markdown::schemas::simplified::SimplifiedType;

    let def = expression_function_catalog().named_type("agentic-cli").expect("`agentic-cli` is a named type");
    let PropertyDef::Single(atom) = def else {
        panic!("`agentic-cli` is one `enum(...)`: {def:?}");
    };
    assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Enum), "{def:?}");
    let members: Vec<&str> = atom
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::Members(values) => Some(values.iter().map(String::as_str).collect()),
            _ => None,
        })
        .unwrap_or_default();
    assert_eq!(members, AGENTIC_CLI_NAMES.iter().map(|(name, _)| *name).collect::<Vec<_>>());
}

struct FixtureLookup {
    ctx: ResolutionContext,
    data: HashMap<String, Value>,
}

impl EvaluationLookup for FixtureLookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.data.get(path).cloned()
    }
    fn resolution_context(&self) -> Option<ResolutionContext> {
        Some(self.ctx.clone())
    }
}

/// Every executable inline example evaluates to its declared result, or fails
/// when it declares `error`.
#[test]
fn every_executable_example_evaluates_to_its_declared_result() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("fixture.md"), "---\ntitle: Fixture Title\n---\n# Fixture\n\nBody\n").unwrap();
    std::fs::write(dir.path().join("note.md"), "plain\n").unwrap();
    std::fs::write(dir.path().join("review-1.md"), "").unwrap();
    std::fs::write(dir.path().join("review-2.md"), "").unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/note.md"), "").unwrap();
    let lookup = FixtureLookup {
        ctx: ResolutionContext::at(dir.path().to_path_buf()),
        data: HashMap::from([("items".to_string(), json!([1, 2, 3])), ("obj".to_string(), json!({"a": 1}))]),
    };
    let mut failures = Vec::new();
    for (call, expected) in partition_call_examples().executable {
        let outcome = parse(call).map_err(|error| error.message).and_then(|expression| {
            evaluate(&expression, &lookup).map_err(|error| error.to_string())
        });
        if !example_holds(expected, &outcome) {
            failures.push(format!("{call}: expected {expected:?}, got {outcome:?}"));
        }
    }
    assert!(failures.is_empty(), "examples that disagree with their result:\n{}", failures.join("\n"));
}

/// The embedded catalog's inline call examples, split into the ones the
/// runner evaluates and the display-only ones it skips by design.
struct CallExamples {
    executable: Vec<(&'static str, &'static ExampleOutcome)>,
    display_only: Vec<(&'static str, &'static str)>,
}

fn partition_call_examples() -> CallExamples {
    let mut examples = CallExamples {
        executable: Vec::new(),
        display_only: Vec::new(),
    };
    for descriptor in expression_function_descriptors() {
        for example in descriptor.examples {
            if let FunctionExample::Call {
                call,
                expected,
                display_only,
                ..
            } = example
            {
                match display_only {
                    None => examples.executable.push((*call, expected)),
                    Some(reason) => examples.display_only.push((*call, *reason)),
                }
            }
        }
    }
    examples
}

/// Whether an evaluation outcome satisfies an example's declared result:
/// a value compares structurally, and `=> error` holds only when the call
/// fails.
fn example_holds(expected: &ExampleOutcome, outcome: &Result<Value, String>) -> bool {
    match (expected, outcome) {
        (ExampleOutcome::Value(want), Ok(got)) => numeric_eq(want, got),
        (ExampleOutcome::Error, Err(_)) => true,
        _ => false,
    }
}

/// Criteria 8 and 9: at least 72 examples run, and every display-only one is
/// reported as skipped by design with its reason.
#[test]
fn executable_examples_number_at_least_72_and_display_only_ones_are_skipped_by_design() {
    let examples = partition_call_examples();
    assert!(
        examples.executable.len() >= 72,
        "only {} executable examples",
        examples.executable.len()
    );
    assert!(!examples.display_only.is_empty(), "the catalog carries display-only examples");
    for (call, reason) in &examples.display_only {
        assert!(!reason.trim().is_empty(), "{call}: a display-only example names its reason");
        assert!(
            !examples.executable.iter().any(|(executable, _)| executable == call),
            "{call}: a display-only example is never executed"
        );
    }
    eprintln!(
        "{} executable examples; {} display-only examples skipped by design",
        examples.executable.len(),
        examples.display_only.len()
    );
}

/// Criterion 8: results compare structurally, never by spelling.
#[test]
fn example_results_compare_structurally() {
    let three = ExampleOutcome::Value(json!(3));
    assert!(example_holds(&three, &Ok(json!(3))));
    assert!(example_holds(&three, &Ok(json!(3.0))), "numbers compare by value");
    assert!(!example_holds(&three, &Ok(json!("3"))), "`=> 3` fails against the text \"3\"");

    let object = ExampleOutcome::Value(json!({"a": 1, "b": 2}));
    assert!(example_holds(&object, &Ok(json!({"b": 2, "a": 1}))), "key order is irrelevant");
    assert!(!example_holds(&object, &Ok(json!({"a": 1}))));
    assert!(!example_holds(&object, &Ok(json!({"a": 1, "b": 2, "c": 3}))));

    assert!(example_holds(&ExampleOutcome::Error, &Err("type error".to_string())));
    assert!(!example_holds(&ExampleOutcome::Error, &Ok(Value::Null)), "`=> error` requires a failure");
    assert!(!example_holds(&three, &Err("type error".to_string())));
}

/// Structural equality that compares numbers by value (`3` and `3.0`).
fn numeric_eq(want: &Value, got: &Value) -> bool {
    match (want, got) {
        (Value::Number(want), Value::Number(got)) => want.as_f64() == got.as_f64(),
        (Value::Array(want), Value::Array(got)) => {
            want.len() == got.len() && want.iter().zip(got).all(|(want, got)| numeric_eq(want, got))
        }
        (Value::Object(want), Value::Object(got)) => {
            want.len() == got.len()
                && want.iter().all(|(key, want)| got.get(key).is_some_and(|got| numeric_eq(want, got)))
        }
        _ => want == got,
    }
}

/// Every embedded example file's `invocation`, with its `parameters` bound as
/// values, evaluates to its `returns`.
#[test]
fn every_embedded_example_file_evaluates_to_its_returns() {
    for (path, text) in EXAMPLE_FILES {
        let document: Value = serde_yaml_ng::from_str(text).unwrap_or_else(|error| panic!("{path}: {error}"));
        let mut data = HashMap::new();
        for parameter in document["parameters"].as_array().into_iter().flatten() {
            for (key, value) in parameter.as_object().into_iter().flatten() {
                data.insert(key.clone(), value.clone());
            }
        }
        let lookup = FixtureLookup {
            ctx: ResolutionContext::at(std::env::temp_dir()),
            data,
        };
        let invocation = document["invocation"].as_str().unwrap_or_else(|| panic!("{path}: a call invocation"));
        let got = evaluate(&parse(invocation).unwrap(), &lookup).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(got, document["returns"], "{path}");
    }
}

#[test]
fn the_generated_table_shows_inline_display_only_and_file_examples() {
    let table = generate_expression_function_table();
    assert!(table.starts_with("| Category | Function | Description | Example |\n"));
    let rows = table.lines().count() - 2;
    assert_eq!(rows, expression_function_descriptors().len());
    assert!(table.contains("(display only: "), "display-only examples are shown and marked");
}

/// The generated function table in `darkmatter-expressions.md` must match
/// the catalog output exactly.
#[test]
fn narrative_doc_function_table_matches_catalog() {
    let doc_path = biscuit_test_harness::manifest_dir!().join("../../darkmatter/docs/topics/darkmatter-expressions.md");
    let content = std::fs::read_to_string(&doc_path).expect("darkmatter-expressions.md should be readable");
    const START: &str = "<!-- BEGIN GENERATED FUNCTION TABLE -->";
    const END: &str = "<!-- END GENERATED FUNCTION TABLE -->";
    let start = content.find(START).expect("start marker should exist") + START.len();
    let end = content.find(END).expect("end marker should exist");
    assert_eq!(
        content[start..end].trim(),
        generate_expression_function_table().trim(),
        "function table in darkmatter-expressions.md does not match generated output"
    );
}

/// The `pr_list`/`cicd_list` query overloads' descriptions carry the
/// query-vocabulary link that DMLS hover and completion surface (the `count`
/// overloads take no query); the anchor must exist.
#[test]
fn query_vocabulary_link_resolves_to_an_existing_doc_anchor() {
    const LINK: &str = "(darkmatter-expressions.md#provider-query-vocabulary)";
    const ANCHOR: &str = "provider-query-vocabulary";

    let linking: Vec<&str> = expression_function_descriptors()
        .iter()
        .filter(|descriptor| descriptor.description.contains(LINK))
        .map(|descriptor| descriptor.signature)
        .collect();
    for signature in ["pr_list(query)", "cicd_list(query)"] {
        assert!(linking.contains(&signature), "`{signature}` must link to the query vocabulary; linking: {linking:?}");
    }
    let doc_path = biscuit_test_harness::manifest_dir!().join("../../darkmatter/docs/topics/darkmatter-expressions.md");
    let content = std::fs::read_to_string(&doc_path).expect("darkmatter-expressions.md should be readable");
    let has_anchor = content.lines().filter_map(|line| line.strip_prefix("## ")).any(|heading| {
        let slug: String = heading
            .trim()
            .to_ascii_lowercase()
            .chars()
            .filter_map(|c| match c {
                ' ' => Some('-'),
                c if c.is_ascii_alphanumeric() || c == '-' => Some(c),
                _ => None,
            })
            .collect();
        slug == ANCHOR
    });
    assert!(has_anchor, "`#{ANCHOR}` must resolve to a heading in darkmatter-expressions.md");
}

/// AC12: both names are cataloged and dispatch to the same probe.
#[test]
fn has_binary_and_has_command_share_one_probe() {
    use crate::markdown::compose::expression::functions::call_function;

    assert_eq!(function_descriptors("has_command").len(), 1);
    assert_eq!(function_descriptors("has_binary").len(), 1);
    let context = ResolutionContext::at(std::env::temp_dir());
    for probe in ["sh", "definitely-not-a-real-bin-zzz", "", "./relative"] {
        let args = [json!(probe)];
        assert_eq!(
            call_function("has_binary", &args, Some(&context)).unwrap().unwrap(),
            call_function("has_command", &args, Some(&context)).unwrap().unwrap(),
            "`{probe}`"
        );
    }
}
