use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use super::*;

/// [`load_catalog`] of an expression catalog at `path`.
fn load(source: &str, path: impl AsRef<Path>, example_files: &[(&str, &str)]) -> Result<FunctionCatalog, CatalogError> {
    load_catalog(source, path.as_ref(), StandaloneSchemaEnvelope::ExpressionSchema, example_files)
}
use crate::markdown::compose::expression::{EvaluationLookup, ResolutionContext, evaluate, parse};

const FIXTURE: &str = r#"kind: expression-schema
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
    load(FIXTURE, "fixture.yaml", EXAMPLE_FILES).expect("the fixture loads")
}

fn by_signature<'a>(catalog: &'a FunctionCatalog, signature: &str) -> &'a ExpressionFunctionDescriptor {
    catalog
        .descriptors
        .iter()
        .find(|descriptor| &*descriptor.signature == signature)
        .unwrap_or_else(|| panic!("`{signature}` is cataloged"))
}

#[test]
fn a_document_without_categories_does_not_load() {
    let source = FIXTURE.replace("categories:\n    - Math -> Numbers.\n    - Logical\n", "");
    let error = load(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
    // The kind's definition requires the list; the shared reader refuses it.
    assert!(matches!(error, CatalogError::Schema(_)), "{error:?}");
    assert!(error.to_string().contains("categories"), "{error}");
}

#[test]
fn an_unlisted_category_is_the_standalone_parsers_error() {
    let source = FIXTURE.replacen("category(Math);", "category(Geometry);", 1);
    let error = load(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
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
    let error = load(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
    assert!(matches!(error, CatalogError::Schema(_)), "{error:?}");
    assert!(error.to_string().contains("the same binding contract"), "{error}");
}

/// Ruling R5: the catalog reader requires the expected callable kind; a
/// general schema, another kind, or the other callable kind is refused.
#[test]
fn a_missing_null_or_wrong_kind_does_not_load() {
    for kind in ["", "kind: null\n", "kind: ~\n", "kind: \"\"\n", "kind: trigger\n", "kind: 5\n", "kind: schema\n", "kind: mutation-schema\n"] {
        let source = FIXTURE.replacen("kind: expression-schema\n", kind, 1);
        let error = load(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
        assert!(error.to_string().contains("kind: expression-schema") || matches!(error, CatalogError::Schema(_)), "{kind:?}: {error}");
    }
    let duplicate = FIXTURE.replacen("kind: expression-schema\n", "kind: expression-schema\nkind: expression-schema\n", 1);
    let trailing = format!("{FIXTURE}---\nkind: expression-schema\n");
    let garbage = format!("{FIXTURE}  ] not yaml\n");
    for source in [duplicate, trailing, garbage] {
        assert!(load(&source, "fixture.yaml", EXAMPLE_FILES).is_err(), "{source}");
    }
}

/// A bare null `types` entry is refused by the shared reader, located at
/// the entry; a quoted `"null"` helper is the null type.
#[test]
fn an_entry_written_as_null_does_not_load() {
    for value in [" null", " ~", ""] {
        let source = FIXTURE.replacen("    ip-like: string\n", &format!("    ip-like: string\n    round_down:{value}\n"), 1);
        let error = load(&source, "fixture.yaml", EXAMPLE_FILES).unwrap_err();
        assert!(matches!(error, CatalogError::Schema(_)), "{value:?}: {error:?}");
        assert!(error.to_string().contains("round_down"), "{value:?}: {error}");
    }
    let quoted = FIXTURE.replacen("    ip-like: string\n", "    ip-like: string\n    nothing: \"null\"\n", 1);
    let catalog = load(&quoted, "fixture.yaml", EXAMPLE_FILES).expect("a quoted null helper is the null type");
    assert!(catalog.named_type("nothing").is_some());
}

#[test]
fn a_non_function_entry_is_a_named_type_not_a_function() {
    let catalog = fixture();
    assert!(catalog.descriptors.iter().all(|descriptor| &*descriptor.name != "ip-like"));
    assert_eq!(catalog.named_type_names().collect::<Vec<_>>(), ["ip-like"]);
    assert!(catalog.named_type("ip-like").is_some());
}

#[test]
fn descriptors_follow_category_order_then_name_then_authored_overloads() {
    let catalog = fixture();
    let order: Vec<(&str, &str, usize, usize)> = catalog
        .descriptors
        .iter()
        .map(|descriptor| (&*descriptor.category, &*descriptor.name, descriptor.order, descriptor.overload))
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
    assert_eq!(&*by_signature(&catalog, "is_string(x)").description, "One.");
    assert_eq!(&*by_signature(&catalog, "is_string(x, y)").description, "Two.");
    assert_eq!(
        *catalog.categories,
        [
            CatalogCategory {
                name: "Math".into(),
                description: Some("Numbers.".into())
            },
            CatalogCategory {
                name: "Logical".into(),
                description: None
            },
        ]
    );
}

#[test]
fn registered_aliases_attach_and_lookups_resolve_alias_and_case() {
    let catalog = fixture();
    assert_eq!(*by_signature(&catalog, "is_string(x)").aliases, ["isstring".into()]);
    assert_eq!(catalog.descriptors_for("isString").len(), 2);
    assert_eq!(catalog.descriptors_for("IS_STRING").len(), 2);
    assert!(catalog.descriptors_for("nope").is_empty());

    let overloads = function_descriptors("isString");
    assert!(!overloads.is_empty());
    assert!(overloads.iter().all(|descriptor| &*descriptor.name == "is_string"));
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
        *min.examples,
        [
            FunctionExample::Call {
                call: "min(3, 5)".into(),
                expected: ExampleOutcome::Value(json!(3)),
                result: "3".into(),
                display_only: None,
            },
            FunctionExample::Call {
                call: "min(\"a\", 5)".into(),
                expected: ExampleOutcome::Error,
                result: "error".into(),
                display_only: Some("shows the type error".into()),
            },
        ]
    );
    let example = min.example().expect("the first inline example");
    assert_eq!((example.invocation, example.result), ("min(3, 5)", "3"));
    assert_eq!(example.verification, ExampleVerification::Executable);

    let upper = by_signature(&catalog, "upper(text)");
    assert_eq!(upper.example().map(|example| example.result), Some("\"A\""));
    let FunctionExample::File {
        path,
        invocation,
        returns,
        ..
    } = &upper.examples[1]
    else {
        panic!("a file-form example: {:?}", upper.examples);
    };
    assert_eq!((&**path, &**invocation, &**returns), ("function-examples/as_tsv.yaml", r#"as_tsv(["a", "b", "c"])"#, "a\tb\tc"));

    let error = load(FIXTURE, "fixture.yaml", &[]).unwrap_err();
    assert!(error.to_string().contains("is not one of this catalog's embedded examples"), "{error}");
}

/// `Described::example` is a view of the first inline call example: a
/// display-only reason and an `=> error` outcome carry through, and a function
/// whose only example is file-form, or that has none, has no view. Static
/// descriptor tables lend the same view of their stored example.
#[test]
fn the_described_example_is_a_view_of_the_first_inline_call() {
    use crate::effects::EFFECT_DESCRIPTORS;
    use crate::markdown::compose::expression::semantics::OPERATOR_DESCRIPTORS;

    const SOURCE: &str = r#"kind: expression-schema
categories:
    - Text
types:
    shown: |-
        function(
            parameters(text: string);
            returns(string);
            category(Text);
            example(shown("a") => "a"; display-only: depends on the host);
            example(shown("b") => "b");
        ) -> Shown.
    failing: |-
        function(
            parameters(text: string);
            returns(string);
            category(Text);
            fallible;
            example(failing("") => error);
            example(failing("a") => "a");
        ) -> Failing.
    filed: |-
        function(
            parameters(list: string[]);
            returns(string);
            category(Text);
            example(./function-examples/as_tsv.yaml);
        ) -> Filed.
    bare: |-
        function(parameters(x: string); returns(string); category(Text)) -> Bare.
"#;
    let catalog = load(SOURCE, "fixture.yaml", EXAMPLE_FILES).expect("the fixture loads");

    let shown = by_signature(&catalog, "shown(text)").example().expect("an inline example");
    assert_eq!((shown.invocation, shown.result), (r#"shown("a")"#, r#""a""#));
    assert_eq!(shown.verification, ExampleVerification::DisplayOnly("depends on the host"));
    assert_eq!(
        by_signature(&catalog, "failing(text)").example(),
        Some(Example {
            invocation: r#"failing("")"#,
            result: "error",
            verification: ExampleVerification::Executable,
        }),
        "the first of two inline calls, with its `=> error` outcome"
    );
    let filed = by_signature(&catalog, "filed(list)");
    assert_eq!(filed.examples.len(), 1, "the file-form example still loads");
    assert_eq!(filed.example(), None);
    assert_eq!(by_signature(&catalog, "bare(x)").example(), None);

    let set = &EFFECT_DESCRIPTORS[0];
    let effect: Option<Example<'static>> = set.example();
    let effect = effect.expect("`set` has an example");
    assert_eq!((set.signature, effect.invocation, effect.result), ("set(key, value)", r#"set("ready", true)"#, "null"));
    assert!(
        matches!(effect.verification, ExampleVerification::DisplayOnly(reason) if !reason.is_empty()),
        "{effect:?}"
    );
    let operator: Option<Example<'static>> = OPERATOR_DESCRIPTORS[0].example();
    assert_eq!(
        operator,
        Some(Example {
            invocation: "1 + 2 * 3",
            result: "7",
            verification: ExampleVerification::Executable,
        })
    );
}

/// A loaded catalog owns its projection: dropping the source text and path
/// it was loaded from leaves every name, type, example, and reason intact.
#[test]
fn a_loaded_catalog_outlives_the_text_it_was_loaded_from() {
    let source = FIXTURE.to_string();
    let path = "fixture.yaml".to_string();
    let catalog = load(&source, &path, EXAMPLE_FILES).expect("the fixture loads");
    drop((source, path));

    let min = by_signature(&catalog, "min(a, b)");
    assert_eq!(min.typed_signature(), "min(a: number, b: number) -> number | error");
    assert_eq!((&*min.category, &*min.description), ("Math", "The smaller number."));
    let FunctionExample::Call { display_only, .. } = &min.examples[1] else {
        panic!("an inline example: {:?}", min.examples);
    };
    assert_eq!(display_only.as_deref(), Some("shows the type error"));
    assert_eq!(
        min.example(),
        Some(Example {
            invocation: "min(3, 5)",
            result: "3",
            verification: ExampleVerification::Executable,
        })
    );
    assert_eq!(catalog.descriptors_for("is_string").len(), 2);
    assert_eq!(catalog.categories[0].description.as_deref(), Some("Numbers."));
}

#[test]
fn the_embedded_catalog_loads_with_unique_display_keys() {
    let catalog = expression_function_catalog();
    assert!(!catalog.descriptors.is_empty());
    let mut seen = HashSet::new();
    for descriptor in &catalog.descriptors {
        assert!(seen.insert(&descriptor.signature), "duplicate display key `{}`", descriptor.signature);
        assert!(!descriptor.category.is_empty());
    }
    for category in &catalog.categories {
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
        simplified_schema::example::validate_example_bytes(path, text.as_bytes())
            .unwrap_or_else(|error| panic!("{path}: {error}"));
    }
}

/// The `agentic-cli` named type is the generated name table, in order.
#[test]
fn agentic_cli_named_type_is_the_generated_table() {
    use super::super::functions::agentic_cli_generated::AGENTIC_CLI_NAMES;
    use simplified_schema::simplified::SimplifiedType;

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
    for (file, content) in EXAMPLE_DATA_FILES {
        std::fs::write(dir.path().join(file), content).unwrap();
    }
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
        for example in &descriptor.examples {
            if let FunctionExample::Call {
                call,
                expected,
                display_only,
                ..
            } = example
            {
                match display_only {
                    None => examples.executable.push((&**call, expected)),
                    Some(reason) => examples.display_only.push((&**call, &**reason)),
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

/// The committed `load_data` reference page must equal the generator's
/// output; regenerate it with `just regen-expr-doc`.
#[test]
fn load_data_function_page_matches_catalog() {
    let page_path = biscuit_test_harness::manifest_dir!().join("../../darkmatter/docs/functions/load-data.md");
    let content = std::fs::read_to_string(&page_path).expect("docs/functions/load-data.md should be readable");
    let generated = generate_expression_function_page("load_data").expect("load_data is cataloged");
    assert_eq!(content, generated, "docs/functions/load-data.md does not match generated output");
    assert!(
        EXPRESSION_FUNCTION_PAGES
            .iter()
            .any(|page| page.function == "load_data" && page.path == "docs/functions/load-data.md"),
        "the generator writes the page this test reads"
    );
}

/// The committed `try` reference page must equal the generator's output;
/// regenerate it with `just regen-expr-doc`.
#[test]
fn try_function_page_matches_catalog() {
    let page_path = biscuit_test_harness::manifest_dir!().join("../../darkmatter/docs/functions/try.md");
    let content = std::fs::read_to_string(&page_path).expect("docs/functions/try.md should be readable");
    let generated = generate_expression_function_page("try").expect("try is cataloged");
    assert_eq!(content, generated, "docs/functions/try.md does not match generated output");
    assert!(
        EXPRESSION_FUNCTION_PAGES.iter().any(|page| page.function == "try" && page.path == "docs/functions/try.md"),
        "the generator writes the page this test reads"
    );
}

/// The generated `try` page states the lazy contract the evaluator
/// implements: the fallback runs only after a recoverable read failure, every
/// other failure propagates, falsy values are successes, and the page links
/// the topic sections that list the recoverable failures.
#[test]
fn try_page_states_lazy_semantics() {
    let page = generate_expression_function_page("try").expect("try is cataloged");
    assert!(page.contains("try(expr: unknown | null, fallback: unknown | null)"), "the page shows the two-operand signature");
    assert!(page.contains("Only when expr fails with a recoverable read failure"), "the fallback is lazy");
    assert!(page.contains("every other failure propagates"), "non-recoverable failures propagate");
    assert!(page.contains("including a falsy one"), "falsy values are successes");
    for example in ["try(null, 7)", "try(0, 7)"] {
        assert!(page.contains(example), "the page shows the falsy-success example `{example}`");
    }
    for link in ["../topics/darkmatter-expressions.md#recovering-from-read-failures", "../topics/loading-data.md"] {
        assert!(page.contains(&format!("]({link})")), "the page links {link}");
    }
}

/// R12: the generated `load_data` page shows an executable example, and its
/// input file, for every format family.
#[test]
fn load_data_function_page_shows_every_format_family() {
    let page = generate_expression_function_page("load_data").expect("load_data is cataloged");
    let executable = partition_call_examples().executable;
    for extension in ["yaml", "json", "jsonl", "csv", "tsv", "toml", "json5"] {
        let file = format!("fixture.{extension}");
        let call = executable
            .iter()
            .map(|(call, _)| *call)
            .find(|call| call.starts_with("load_data(") && call.contains(&format!("\"{file}\"")))
            .unwrap_or_else(|| panic!("no executable load_data example reads a .{extension} file"));
        assert!(page.contains(call), "the page shows `{call}`");
        assert!(page.contains(&format!("`{file}`:")), "the page shows the input file `{file}`");
    }
    assert!(page.contains("Ada,yes,1_000"), "the CSV example shows the boolean and number cell vocabulary");
    for link in ["../topics/loading-data.md", "../topics/schemas/definition.md#container-types", "../cli/data-contracts.md"] {
        assert!(page.contains(&format!("]({link})")), "the page links {link}");
    }
}

/// The generated `load_data` page shows the three-argument examples (a
/// headerless CSV, a custom delimiter on a `.txt` file, an explicit format on
/// an extensionless file), each one's input file, and the `options` parameter.
#[test]
fn load_data_function_page_shows_option_examples() {
    let page = generate_expression_function_page("load_data").expect("load_data is cataloged");
    let executable = partition_call_examples().executable;
    for (file, option, language) in [
        ("fixture-headerless.csv", "{header: false}", "csv"),
        ("fixture-semicolon.txt", "{format: \"csv\", delimiter: \";\"}", "text"),
        ("fixture-export", "{format: \"json\"}", "text"),
    ] {
        let call = executable
            .iter()
            .map(|(call, _)| *call)
            .find(|call| call.starts_with("load_data(") && call.contains(&format!("\"{file}\"")))
            .unwrap_or_else(|| panic!("no executable load_data example reads `{file}`"));
        assert!(call.contains(option), "`{call}` passes `{option}`");
        assert!(page.contains(call), "the page shows `{call}`");
        assert!(page.contains(&format!("`{file}`:\n\n```{language}\n")), "the page shows `{file}` fenced as {language}");
    }
    assert!(page.contains("web,8080\ndb,5432"), "the headerless input has no header row");
    assert!(page.contains("name;port"), "the semicolon input is shown");
    assert!(page.contains("options"), "the page names the third parameter");
}

#[test]
fn function_pages_render_only_cataloged_functions() {
    assert!(generate_expression_function_page("no_such_function").is_none());
    let page = generate_expression_function_page("loaddata").expect("an alias names the function");
    assert!(page.starts_with("<!-- Generated by `just regen-expr-doc`"));
    let overloaded = generate_expression_function_page("link").expect("link is cataloged");
    assert!(overloaded.contains("## Overloads"), "an overload set lists each overload");
    assert!(!overloaded.contains("## See also"), "only a registered page has related links");
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
        .map(|descriptor| &*descriptor.signature)
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

/// A `narrows` clause on each overload of `is_string`; `second` replaces the
/// second overload's clause (`None` leaves it absent).
fn with_narrows(first: &str, second: Option<&str>) -> String {
    let add = |signature: &str, clause: &str| signature.replacen("returns(boolean);", &format!("returns(boolean); {clause};"), 1);
    let one = "function(parameters(x: string); returns(boolean); category(Logical)) -> One.";
    let two = "function(parameters(x: number, y: number); returns(boolean); category(Logical)) -> Two.";
    let second = second.map_or_else(|| two.to_string(), |clause| add(two, clause));
    FIXTURE.replacen(one, &add(one, first), 1).replacen(two, &second, 1)
}

/// Overloads share one declared fact: the control loads and every
/// disagreement fails the whole file as a schema error naming `is_string`,
/// since the schema grammar applies the declaration rules.
#[test]
fn overloads_must_agree_on_the_declared_fact() {
    let load = |source: &str| load(source, "fixture.yaml", EXAMPLE_FILES);
    load(&with_narrows("narrows(x: string; true-iff)", Some("narrows(x: string; true-iff)"))).expect("control: same fact");
    for (cell, source) in [
        ("absent on the second", with_narrows("narrows(x: string; true-iff)", None)),
        ("different kind", with_narrows("narrows(x: string; true-iff)", Some("narrows(x: string; true-implies)"))),
        ("different target", with_narrows("narrows(x: string; true-iff)", Some("narrows(x: number; true-iff)"))),
        ("presence versus type", with_narrows("narrows(x; false-implies)", Some("narrows(x: string; false-implies)"))),
    ] {
        let error = load(&source).expect_err(cell);
        assert!(matches!(error, CatalogError::Schema(_)), "{cell}: {error:?}");
        assert!(error.to_string().contains("`is_string`"), "{cell}: {error}");
    }
}

/// Targets compare by meaning: description and constraint order do not make
/// two overloads disagree.
#[test]
fn equivalent_target_spellings_agree() {
    let source = with_narrows(
        "narrows(x: string(min(1); max(5)); true-implies)",
        Some("narrows(x: string(max(5); min(1)); true-implies)"),
    );
    load(&source, "fixture.yaml", EXAMPLE_FILES).expect("equivalent spellings agree");
}

/// A clause naming a parameter only the later overload declares is not
/// skipped on the overload that omits it: the grammar rejects it per overload.
#[test]
fn a_clause_naming_a_parameter_the_overload_omits_fails_the_load() {
    let source = with_narrows("narrows(x: string; true-iff)", Some("narrows(y: string; true-implies)"));
    let error = load(&source, "fixture.yaml", EXAMPLE_FILES).expect_err("y is not declared");
    assert!(error.to_string().contains("does not declare") || error.to_string().contains("different"), "{error}");
    let only_later = FIXTURE.replacen(
        "function(parameters(x: number, y: number); returns(boolean); category(Logical)) -> Two.",
        "function(parameters(x: number, y: number); returns(boolean); narrows(y: number; true-implies); category(Logical)) -> Two.",
        1,
    );
    let error = load(&only_later, "fixture.yaml", EXAMPLE_FILES).expect_err("only the later overload declares");
    assert!(error.to_string().contains("`is_string`"), "{error}");
}

/// Overloads with one binding contract stay duplicates whatever their clauses say.
#[test]
fn duplicate_binding_contracts_stay_duplicates_despite_differing_clauses() {
    let source = with_narrows("narrows(x: string; true-iff)", Some("narrows(x; false-implies)"))
        .replacen("parameters(x: number, y: number)", "parameters(x: string)", 1);
    let error = load(&source, "fixture.yaml", EXAMPLE_FILES).expect_err("same contract");
    assert!(error.to_string().contains("the same binding contract"), "{error}");
}

#[test]
fn callback_example_arrows_preserve_nested_calls_patterns_and_quoted_text() {
    for (call, expected) in [
        ("filter([1, 2, 3], n => n > 1)", json!([2, 3])),
        ("filter([1, 2, 3], n -> n > 1)", json!([2, 3])),
        (r#"map({z: "=>", a: "->"}, {key, value} => value)"#, json!(["=>", "->"])),
        ("map([[1, 2]], xs => filter(xs, n -> n > 1))", json!([[2]])),
    ] {
        let source = format!("kind: expression-schema\ncategories: [Collection]\ntypes:\n  demo: |-\n    function(parameters(void); returns(unknown); category(Collection); example({call} => {expected}))\n");
        let catalog = load(&source, "callback-example.yaml", &[]).unwrap();
        let FunctionExample::Call { call: parsed, expected: parsed_expected, result, .. } = &catalog.descriptors[0].examples[0] else {
            panic!("an inline example");
        };
        assert_eq!(parsed.as_ref(), call);
        assert_eq!(parsed_expected, &ExampleOutcome::Value(expected.clone()));
        assert_eq!(result.as_ref(), expected.to_string());
    }
    for (example, reason) in [
        ("filter([1, 2, 3], n =>) => [2, 3]", "not a valid expression"),
        ("filter([1, 2, 3], n => n > 1) => n", "must be a literal"),
        ("filter([1, 2, 3], n => n > 1) =>", "must be a literal"),
    ] {
        let source = format!("kind: expression-schema\ncategories: [Collection]\ntypes:\n  demo: |-\n    function(parameters(void); returns(unknown); category(Collection); example({example}))\n");
        let error = load(&source, "callback-example.yaml", &[]).unwrap_err().to_string();
        assert!(error.contains("callback-example.yaml"), "{error}");
        assert!(error.contains(reason), "{example}: {error}");
    }
}

#[test]
fn shipped_callback_examples_passively_parse_through_the_catalog() {
    let source = include_str!("../../../../../../schemas/partials/functions.yaml");
    let catalog = load(source, CATALOG_PATH, EXAMPLE_FILES).unwrap();
    for name in ["map", "filter"] {
        let descriptors = catalog.descriptors_for(name);
        assert_eq!(descriptors.len(), 1, "{name}");
        assert!(!descriptors[0].accepts_value_arguments());
        assert!(!descriptors[0].examples.is_empty());
        for example in &descriptors[0].examples {
            let FunctionExample::Call { call, display_only, .. } = example else {
                panic!("{name}: expected inline call corpus");
            };
            assert!(display_only.is_none(), "{call}");
            assert!(parse(call).is_ok(), "{call}");
        }
    }
}

/// `load_data`'s third parameter is an optional, nullable, closed inline
/// object whose keys and `format` members come from the catalog.
#[test]
fn load_data_options_parameter_is_a_closed_inline_object() {
    use simplified_schema::simplified::{FunctionParameters, SimplifiedType};

    let descriptors = function_descriptors("load_data");
    assert_eq!(descriptors.len(), 1, "one overload");
    let descriptor = &descriptors[0];
    assert_eq!(&*descriptor.signature, "load_data(file, [as], [options])");
    let FunctionParameters::List(parameters) = &descriptor.function_type.parameters else {
        panic!("load_data declares parameters");
    };
    assert_eq!(parameters.len(), 3);
    let options = &parameters[2];
    assert_eq!(options.name, "options");
    assert!(options.optional && !options.rest);
    let PropertyDef::Union(arms) = &options.def else {
        panic!("options is `{{…}} | null`: {:?}", options.def);
    };
    assert!(arms.iter().any(|arm| arm.ty == TypeExpr::Primitive(SimplifiedType::Null)), "admits null: {arms:?}");
    let shape = arms
        .iter()
        .find_map(|arm| match &arm.ty {
            TypeExpr::InlineObject(shape) => Some(shape),
            _ => None,
        })
        .expect("an inline-object arm");
    assert_eq!(shape.properties.keys().map(String::as_str).collect::<Vec<_>>(), ["format", "delimiter", "header"]);
    assert!(shape.pattern_keys.is_empty(), "closed: no pattern keys");
    let PropertyDef::Single(format) = &shape.properties["format"] else {
        panic!("format is one enum");
    };
    assert_eq!(format.ty, TypeExpr::Primitive(SimplifiedType::Enum));
    let members: Vec<&str> = format
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::Members(values) => Some(values.iter().map(String::as_str).collect()),
            _ => None,
        })
        .unwrap_or_default();
    assert_eq!(members, ["yaml", "yml", "json", "jsonl", "ndjson", "csv", "tsv", "toml", "json5"]);
}

/// The shared binder decides the options object's shape: an unknown key, a
/// wrongly typed field, and a format name outside the enum fail binding at
/// argument 2, while `null`, `{}`, and null fields bind (and the unchanged
/// handler then reads the file).
#[test]
fn load_data_options_bind_through_the_shared_binder() {
    use crate::markdown::compose::expression::ExpressionError;
    use crate::markdown::compose::expression::functions::call_function;

    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("rows.csv"), "name,port\nweb,8080\n").unwrap();
    let context = ResolutionContext::at(dir.path().to_path_buf());
    let call = |options: Value| {
        call_function("load_data", &[json!("rows.csv"), json!("tuple([string, number])[]"), options], Some(&context))
            .expect("load_data is registered")
    };
    for options in [
        json!({"quote": ","}),
        // An object converts to neither a boolean nor a name; a number does
        // (`[1]` binds as `["1"]`).
        json!({"header": [{}]}),
        json!({"header": ["web", {}]}),
        json!({"header": 5}),
        json!({"format": "CSV"}),
        json!({"format": " csv"}),
        json!({"format": "text"}),
        json!({"delimiter": [","]}),
        json!(5),
        json!("csv"),
        json!([]),
    ] {
        let error = call(options.clone()).expect_err(&options.to_string());
        let ExpressionError::Call(binding) = &error else {
            panic!("{options}: not a binding error: {error:?}");
        };
        let indices: Vec<usize> = binding.argument_errors().map(|failure| failure.index).collect();
        assert!(!indices.is_empty() && indices.iter().all(|index| *index == 2), "{options}: {indices:?}");
    }
    for options in [json!(null), json!({}), json!({"header": null}), json!({"format": null, "delimiter": null})] {
        assert_eq!(call(options.clone()).unwrap_or_else(|error| panic!("{options}: {error}")), json!([["web", 8080]]));
    }
    // The coercion engine's normal conversions apply to option fields, and
    // the handler reads the converted values: `5` splits on the character
    // `5` (so each row is one cell), `"false"` reads the header as a row, and
    // `[1]` is the name list `["1"]`, whose object rows this list contract
    // cannot describe.
    for (options, code) in [
        (json!({"delimiter": 5}), "dm.data.contract_mismatch"),
        (json!({"header": "false"}), "dm.data.contract_mismatch"),
        (json!({"header": [1]}), "dm.data.row_shape_conflict"),
    ] {
        match call(options.clone()) {
            Err(ExpressionError::Data(data)) => assert_eq!(data.code(), Some(code), "{options}: {data}"),
            other => panic!("{options} binds and is applied: {other:?}"),
        }
    }
}

// ── callable checks and their locations ─────────────────────────────────────

/// A small authored expression catalog: one helper, two callables.
const AUTHORED: &str = r#"kind: expression-schema
categories:
    - Text -> Transform text.
types:
    label: string(not-empty)
    shout: |-
        function(
            parameters(x: label);
            returns(string);
            category(Text);
        ) -> Shouts.
    whisper:
        - |-
            function(parameters(x: string); returns(string); category(Text)) -> One.
        - |-
            function(parameters(x: string, y: number); returns(string)) -> Two.
"#;

fn authored(source: &str) -> Result<FunctionCatalog, CatalogError> {
    load_authored_catalog(source, Path::new("host/text.yaml"), StandaloneSchemaEnvelope::ExpressionSchema)
}

/// The failure's message, primary text, and related texts, as written.
fn located(source: &str, error: &CatalogError) -> (String, Option<String>, Vec<Option<String>>) {
    let CatalogError::Invalid { message, span, related, .. } = error else {
        panic!("a callable failure: {error:?}");
    };
    let text = |span: &Option<std::ops::Range<usize>>| span.clone().map(|span| source[span].to_string());
    (message.clone(), text(span), related.iter().map(|related| text(&related.span)).collect())
}

#[test]
fn the_authored_control_loads_with_an_inherited_category_and_no_aliases() {
    let catalog = authored(AUTHORED).expect("the control loads");
    let names: Vec<&str> = catalog.descriptors.iter().map(|descriptor| &*descriptor.name).collect();
    assert_eq!(names, ["shout", "whisper", "whisper"]);
    assert!(catalog.descriptors.iter().all(|descriptor| &*descriptor.category == "Text" && descriptor.aliases.is_empty()));
    assert_eq!(catalog.named_type_names().collect::<Vec<_>>(), ["label"]);
}

#[test]
fn a_types_table_of_helpers_only_is_not_a_catalog() {
    let source = "kind: expression-schema\ncategories: [Text]\ntypes:\n    label: string\n    other: number\n";
    let error = authored(source).unwrap_err();
    let (message, span, _) = located(source, &error);
    assert!(message.contains("declares no callable"), "{message}");
    assert!(span.is_some_and(|text| text.contains("label: string")), "{error:?}");
}

#[test]
fn a_callable_mixed_with_an_ordinary_type_is_located_at_the_ordinary_arm() {
    let source = AUTHORED.replace(
        "            function(parameters(x: string, y: number); returns(string)) -> Two.",
        "            string",
    );
    let error = authored(&source).unwrap_err();
    let (message, span, _) = located(&source, &error);
    assert!(message.contains("mixes `function(...)` options"), "{message}");
    // A sequence item's range includes its block-scalar indicator.
    assert!(span.as_deref().is_some_and(|text| text.trim_end().ends_with("string") && !text.contains("function")), "{error:?}");
}

#[test]
fn a_callable_without_any_category_is_located_at_its_key() {
    let source = AUTHORED.replace("category(Text) -> One.", "category(Text)) -> One.").replace(
        "            function(parameters(x: string); returns(string); category(Text)) -> One.",
        "            function(parameters(x: string); returns(string)) -> One.",
    );
    let error = authored(&source).unwrap_err();
    let (message, span, _) = located(&source, &error);
    assert!(message.contains("`types.whisper` has no `category(...)`"), "{message}");
    assert_eq!(span.as_deref(), Some("whisper"), "{error:?}");
}

#[test]
fn overloads_naming_different_categories_point_at_both() {
    let source = AUTHORED
        .replace("    - Text -> Transform text.\n", "    - Text -> Transform text.\n    - Other\n")
        .replace("returns(string)) -> Two.", "returns(string); category(Other)) -> Two.");
    let error = authored(&source).unwrap_err();
    let (message, span, related) = located(&source, &error);
    assert!(message.contains("different categories (`Text`, `Other`)"), "{message}");
    assert!(span.is_some_and(|text| text.contains("category(Other)")), "{error:?}");
    assert!(matches!(related.as_slice(), [Some(text)] if text.contains("category(Text)) -> One.")), "{related:?}");
}

#[test]
fn names_matching_without_ascii_case_are_one_callable() {
    let source = AUTHORED.replace("    whisper:\n", "    Shout:\n");
    let error = authored(&source).unwrap_err();
    let (message, span, related) = located(&source, &error);
    assert!(message.contains("`types.Shout` and `types.shout` name the same callable"), "{message}");
    assert_eq!(span.as_deref(), Some("Shout"));
    assert_eq!(related, [Some("shout".to_string())]);
}

#[test]
fn helper_references_stay_inside_the_catalog_and_resolve_no_slot() {
    let cases = [
        ("parameters(x: label)", "parameters(x: label@./other.yaml)", "references `label@./other.yaml`", Some("function(")),
        ("parameters(x: label)", "parameters(x: missing)", "references `missing`, which this catalog does not declare", Some("function(")),
        ("parameters(x: label)", "parameters(x: whisper)", "uses the callable `whisper` as a type", Some("function(")),
        ("    label: string(not-empty)\n", "    label: string(not-empty)\n    unused: nowhere@./x.yaml\n", "`types.unused` references", None),
        ("    label: string(not-empty)\n", "    label: \"{{ fm.kind }}\"\n", "template slot", Some("label")),
    ];
    for (from, to, expected, at) in cases {
        let source = AUTHORED.replacen(from, to, 1);
        let error = authored(&source).unwrap_err();
        let (message, span, _) = located(&source, &error);
        assert!(message.contains(expected), "{to}: {message}");
        match at {
            Some("function(") => assert!(span.as_deref().is_some_and(|span| span.starts_with("function(")), "{to}: {error:?}"),
            Some(key) => assert_eq!(span.as_deref(), Some(key), "{to}: {error:?}"),
            None => {}
        }
    }
    // A helper may refer to itself through a container: grammar-valid local
    // recursion is admitted.
    let recursive = AUTHORED.replacen("    label: string(not-empty)\n", "    label: string(not-empty)\n    tree: \"{ name: string, kids: tree[] }\"\n", 1);
    authored(&recursive).expect("a recursive helper loads");
}

#[test]
fn reference_diagnostics_use_the_signature_and_its_overload_arm() {
    let base = "kind: expression-schema\ncategories: [Text]\ntypes:\n    Label: string\n    show_label:";
    let first = "function(parameters(Missing: string); returns(string); category(Text))";
    let parameter = "function(parameters(Missing: Missing@this); returns(string); category(Text))";
    let returns = "function(parameters(Missing: string); returns(Missing@this); category(Text))";
    let second = "function(parameters(value: number); returns(Missing@this); category(Text))";
    let cases = [
        (format!(" |- # Missing\n        {parameter}\n"), parameter),
        (format!(" |- # Missing\n        {returns}\n"), returns),
        (format!("\n        - |-\n            {first} -> Mentions Missing.\n        - |-\n            {second}\n"), second),
        (format!(" [\"{first} -> Mentions Missing.\", \"{second}\"]\n"), second),
        (format!(" >-\n        {first} |\n        {second}\n"), second),
        (format!(" |-\n        {first} |\n        {second}\n"), second),
    ];
    for (carrier, signature) in cases {
        let source = format!("{base}{carrier}");
        let control = source.replace("Missing@this", "Label@this");
        authored(&control).unwrap_or_else(|error| panic!("{control}: {error}"));
        let error = authored(&source).unwrap_err();
        let (message, span, related) = located(&source, &error);
        assert!(message.contains("references `Missing`"), "{error:?}");
        assert_eq!(span.as_deref(), Some(signature), "{source}");
        assert!(related.is_empty(), "{source}");
    }
}

#[test]
fn reference_diagnostics_use_the_helper_member_or_owning_atom() {
    let cases = [
        ("    Helper:\n        Missing: Missing@this # Missing\n", "Missing"),
        ("    Helper: '{ Missing: Missing@this }'\n", "Missing"),
        ("    Helper:\n        '<starting::Missing>': Missing@this\n", "Missing"),
        ("    Helper:\n        item: string -> Check Missing.\n        $extends: Missing@this\n", "Missing"),
        ("    Helper:\n        item: string -> Check Missing.\n        $extends: [Object@this, Missing@this]\n", "Missing"),
        ("    Helper: '{ item: string -> Check Missing., ...Missing@this }'\n", "Missing"),
        ("    Helper: 'tuple([string, Missing@this])'\n", "tuple([string, Missing@this])"),
        ("    Helper: 'tuple([string, ...(number | Missing@this)[]])'\n", "tuple([string, ...(number | Missing@this)[]])"),
        ("    Helper: '(string | Missing@this)[]'\n", "(string | Missing@this)[]"),
    ];
    for (declaration, marked) in cases {
        let source = format!("{AUTHORED}    Object:\n        key: string\n{declaration}");
        let control = source.replace("Missing@this", "Object@this");
        authored(&control).unwrap_or_else(|error| panic!("{control}: {error}"));
        let error = authored(&source).unwrap_err();
        assert!(error.to_string().contains("references `Missing`"), "{error:?}");
        let start = if marked == "Missing" { source.find("Missing@this") } else { source.find(marked) }.unwrap();
        let CatalogError::Invalid { span, related, .. } = &error else { panic!("{error:?}") };
        assert_eq!(*span, Some(start..start + marked.len()), "{source}\n{error:?}");
        assert!(related.is_empty(), "{error:?}");
    }
}

#[test]
fn a_mutation_declares_no_narrowing_even_in_a_nested_signature() {
    let mutation = |signature: &str| {
        format!("kind: mutation-schema\ncategories: [State]\ntypes:\n    put: |-\n        {signature}\n")
    };
    let control = mutation("function(parameters(key: string); returns(boolean); category(State))");
    load_catalog(&control, Path::new("m.yaml"), StandaloneSchemaEnvelope::MutationSchema, &[]).expect("the control loads");
    for signature in [
        "function(parameters(key: unknown | null); returns(boolean); narrows(key: string; true-implies); category(State))",
        "function(parameters(check: function(parameters(x: unknown | null); returns(boolean); narrows(x: string; true-implies))); returns(boolean); category(State))",
    ] {
        let source = mutation(signature);
        let error = load_catalog(&source, Path::new("m.yaml"), StandaloneSchemaEnvelope::MutationSchema, &[]).unwrap_err();
        let (message, span, _) = located(&source, &error);
        assert!(message.contains("a mutation declares no `narrows(...)`"), "{message}");
        assert!(span.is_some_and(|text| text.contains("narrows(")), "{error:?}");
    }
}

#[test]
fn an_authored_expression_catalog_reuses_no_built_in_name_or_alias() {
    for (name, expected) in [("min", "the built-in function `min`"), ("IsString", "an alias of the built-in function `is_string`")] {
        let name = if name == "IsString" { "IsNull" } else { name };
        let source = AUTHORED.replacen("    shout:", &format!("    {name}:"), 1);
        let error = authored(&source).unwrap_err();
        let CatalogError::Invalid { message, span, related, .. } = &error else { panic!("{error:?}") };
        let expected = if name == "IsNull" { "an alias of the built-in function `is_null`" } else { expected };
        assert!(message.contains(expected), "{message}");
        assert_eq!(span.clone().map(|span| &source[span]), Some(name));
        let [related] = related.as_slice() else { panic!("{error:?}") };
        assert_eq!(related.file, Path::new(CATALOG_PATH));
        let canonical = if name == "IsNull" { "is_null" } else { "min" };
        assert_eq!(related.span.clone().map(|span| &CATALOG_SOURCE[span]), Some(canonical));
    }
    // The built-in reader admits its own names.
    load_catalog(&AUTHORED.replacen("    shout:", "    min:", 1), Path::new("x.yaml"), StandaloneSchemaEnvelope::ExpressionSchema, &[])
        .expect("a built-in read does not check reserved names");
}

#[test]
fn an_authored_callable_compiles_for_eager_evaluation() {
    let source = AUTHORED.replacen(
        "parameters(x: label);",
        "parameters(x: label, each: function(parameters(v: string); returns(string)));",
        1,
    );
    let error = authored(&source).unwrap_err();
    let (message, span, _) = located(&source, &error);
    assert!(message.contains("takes or returns a function value"), "{message}");
    assert!(span.is_some_and(|text| text.contains("each: function(")), "{error:?}");
}

#[test]
fn an_authored_catalog_has_no_file_form_examples() {
    let source = AUTHORED.replacen("category(Text);\n        ) -> Shouts.", "category(Text);\n            example(./function-examples/as_tsv.yaml);\n        ) -> Shouts.", 1);
    let error = authored(&source).unwrap_err();
    let (message, span, _) = located(&source, &error);
    assert!(message.contains("is not one of this catalog's embedded examples"), "{message}");
    assert!(span.is_some_and(|text| text.contains("example(./function-examples/as_tsv.yaml)")), "{error:?}");
}

#[test]
fn a_duplicate_overload_pair_is_refused_by_the_parser_naming_both_overloads() {
    let source = AUTHORED.replacen("parameters(x: string, y: number)", "parameters(z: string)", 1);
    let error = authored(&source).unwrap_err();
    // The standalone parser refuses one binding contract twice before the
    // reader runs; its message names both overloads by position.
    assert!(matches!(error, CatalogError::Schema(_)), "{error:?}");
    assert!(error.to_string().contains("overloads 1 and 2 have the same binding contract"), "{error}");
}

#[test]
fn only_the_built_in_expression_catalog_projects_registered_aliases() {
    let builtin = expression_function_catalog_from_sources();
    assert_eq!(&*builtin.descriptors_for("is_null")[0].aliases, [Box::from("isnull")]);
    let mutations = load_catalog(
        crate::effects::types::MUTATIONS_SOURCE,
        Path::new(crate::effects::types::MUTATIONS_PATH),
        StandaloneSchemaEnvelope::MutationSchema,
        &[],
    )
    .expect("the mutation catalog loads");
    assert!(mutations.descriptors.iter().all(|descriptor| descriptor.aliases.is_empty()));
}

/// The reader's spans are byte ranges of the whole document, so the editor
/// and the CLI place them without adjustment (a non-ASCII character before
/// the failure included).
#[test]
fn spans_are_document_byte_ranges_after_non_ascii_text() {
    let source = AUTHORED.replace("    - Text -> Transform text.", "    - Text -> Transform téxt — ok.").replace("    whisper:\n", "    SHOUT:\n");
    let error = authored(&source).unwrap_err();
    let (_, span, related) = located(&source, &error);
    assert_eq!((span.as_deref(), related), (Some("SHOUT"), vec![Some("shout".to_string())]));
}
