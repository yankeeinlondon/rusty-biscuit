//! Object spread (`{ ...base, key: value }`) at the library's public API:
//! evaluation order and single evaluation, the typed failure of a
//! non-object operand and where it is reported, the passive check that
//! proves one before anything runs, ordered type inference, and the literal
//! option folding `load_data`'s passive check reads.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, ArgumentType, CallCheck, CallProblem, DeclaredTypes, EvaluationLookup, ExpressionError,
    INVALID_SPREAD_OPERAND_CODE, ResolutionContext, SPREAD_CHECK_NAME, TypeScope, check_expression_calls, evaluate,
    expression_member_findings, expression_type, parse, parse_spanned, spread_operand_span,
};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use serde_json::{Value, json};
use simplified_schema::{SchemaRoot, parse_yaml_schema};

struct Lookup {
    values: serde_json::Map<String, Value>,
    context: ResolutionContext,
    _dir: tempfile::TempDir,
}

impl Lookup {
    fn new(values: Value) -> Self {
        let dir = tempfile::tempdir().unwrap();
        Self {
            values: values.as_object().cloned().unwrap_or_default(),
            context: ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir.path())),
            _dir: dir,
        }
    }
}

impl EvaluationLookup for Lookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.values.get(path).cloned()
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        Some(self.context.clone())
    }
}

fn eval(source: &str, lookup: &Lookup) -> Result<Value, ExpressionError> {
    evaluate(&parse(source).unwrap(), lookup)
}

/// The schema the passive cases type their variables with.
const SCHEMA: &str = "
size: number(required)
maybe: number
foo:
    - string(required)
    - number(required)
obj:
    a: string(required)
    b: number
named:
    n: string
";

fn schema() -> SchemaRoot {
    parse_yaml_schema(&serde_yaml_ng::from_str(SCHEMA).unwrap()).unwrap()
}

fn checks(source: &str) -> Vec<CallCheck> {
    let schema = schema();
    let declared = DeclaredTypes::new(Some(&schema));
    check_expression_calls(&parse_spanned(source).unwrap(), &TypeScope::unguarded(&declared))
}

/// `(operand text, found)` of every spread check of `source`.
fn spread_problems(source: &str) -> Vec<(String, String)> {
    checks(source)
        .into_iter()
        .filter(|check| check.function == SPREAD_CHECK_NAME)
        .map(|check| match check.problem {
            Some(CallProblem::InvalidSpreadOperand { span, found }) => {
                assert_eq!(check.span, span);
                (source[span].to_string(), found)
            }
            other => panic!("{source}: {other:?}"),
        })
        .collect()
}

fn type_text(source: &str) -> String {
    let schema = schema();
    let declared = DeclaredTypes::new(Some(&schema));
    ArgumentType::Typed(expression_type(&parse_spanned(source).unwrap(), &TypeScope::unguarded(&declared))).to_string()
}

/// The category of the one `min` call in `source`.
fn min_category(source: &str) -> ArgumentCategory {
    let found = checks(source);
    let [check] = found.iter().filter(|check| check.function == "min").collect::<Vec<_>>()[..] else {
        panic!("{source}: {found:#?}");
    };
    check.category()
}

mod evaluation {
    use super::*;

    #[test]
    fn object_spread_entries_merge_in_written_order() {
        let lookup = Lookup::new(json!({"base": {"a": 1, "b": 1}, "over": {"b": 2, "c": 2}, "none": null}));
        // The last entry to supply a key wins; key order is first insertion.
        assert_eq!(eval("{ a: 0, ...base, c: 0, ...over }", &lookup).unwrap(), json!({"a": 1, "b": 2, "c": 2}));
        assert_eq!(eval("{ ...base, ...over, a: 9 }", &lookup).unwrap(), json!({"a": 9, "b": 2, "c": 2}));
        // A null operand, literal or computed, supplies nothing.
        assert_eq!(eval("{ ...null, ...none, ...{}, k: 1 }", &lookup).unwrap(), json!({"k": 1}));
        // Spreads nest, and a nested object is copied whole, not merged.
        assert_eq!(
            eval("{ ...{ n: { x: 1 } }, ...{ n: { y: 2 } }, m: { ...base } }", &lookup).unwrap(),
            json!({"n": {"y": 2}, "m": {"a": 1, "b": 1}})
        );
    }

    /// Needs a counting test function, which only the instrumented build
    /// registers (`just test` and CI enable it).
    #[cfg(feature = "effects-instrumentation")]
    #[test]
    fn object_spread_operands_are_evaluated_once_each_in_order() {
        use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
        use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let registration = FunctionRegistration::builder("tick", "function(parameters(void); returns(unknown))")
            .build()
            .unwrap();
        let _tick: TestFunctionGuard = hooks::register(
            registration,
            Arc::new(move |_| Ok(json!({ "n": counter.fetch_add(1, Ordering::SeqCst) }))),
        );
        let lookup = Lookup::new(json!({}));
        // Each operand runs once, left to right, even when a later entry
        // overwrites every key it supplied.
        assert_eq!(eval("{ ...tick(), ...tick(), n: 'kept' }", &lookup).unwrap(), json!({"n": "kept"}));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(eval("{ ...tick(), ...tick() }", &lookup).unwrap(), json!({"n": 3}));
    }

    #[test]
    fn object_spread_of_a_non_object_is_a_typed_fatal_error_at_its_operand() {
        let lookup = Lookup::new(json!({"count": 5, "items": [{"a": 1}, "x"]}));
        for (source, found, operand) in [
            ("{ ...count }", "number", "count"),
            ("{ a: 1, ...{ b: 2 }, ...'s' }", "string", "'s'"),
            ("{ ...[1] }", "array", "[1]"),
            ("{ ...true }", "boolean", "true"),
            ("{ k: { ...(count) } }", "number", "(count)"),
        ] {
            let error = eval(source, &lookup).unwrap_err();
            assert_eq!(error.to_string(), format!("object spread requires an object or null, found {found}"));
            assert!(error.is_authoring_fatal(), "{source}");
            assert!(!error.is_recoverable_read_failure(), "{source}");
            let ExpressionError::InvalidSpreadOperand { found: typed, site } = &error else {
                panic!("{source}: {error:?}");
            };
            assert_eq!(*typed, found);
            let span = spread_operand_span(&parse_spanned(source).unwrap(), site.ordinal().unwrap()).unwrap();
            assert_eq!(&source[span], operand, "{source}");
        }
        // Inside a callback: every item runs the same authored operand.
        let source = "map(items, item => { ...item, seen: true })";
        let error = eval(source, &lookup).unwrap_err();
        let ExpressionError::InvalidSpreadOperand { site, .. } = error.without_callbacks() else {
            panic!("{error:?}");
        };
        let span = spread_operand_span(&parse_spanned(source).unwrap(), site.ordinal().unwrap()).unwrap();
        assert_eq!(&source[span], "item");
        assert!(error.is_authoring_fatal());
        // `try` does not recover it: it is not a read failure.
        assert!(eval("try({ ...count }, {})", &lookup).is_err());
    }
}

mod passive {
    use super::*;

    #[test]
    fn object_spread_of_a_proven_non_object_is_rejected_at_its_operand() {
        assert_eq!(spread_problems("{ ...5 }"), [("5".to_string(), "number".to_string())]);
        assert_eq!(spread_problems("{ a: 1, ...'s' }"), [("'s'".to_string(), "string".to_string())]);
        assert_eq!(spread_problems("{ ...[1, 2] }"), [("[1, 2]".to_string(), "array".to_string())]);
        assert_eq!(spread_problems("{ ...size }"), [("size".to_string(), "number".to_string())]);
        assert_eq!(spread_problems("{ ...foo }"), [("foo".to_string(), "number | string".to_string())]);
        // Every branch is checked, selected or not, and callback bodies too.
        assert_eq!(spread_problems("size > 1 ? {} : { ...size }").len(), 1);
        assert_eq!(spread_problems("map([1, 2], n => { ...n })"), [("n".to_string(), "number".to_string())]);
        // A nested spread is checked inside its operand.
        assert_eq!(spread_problems("{ ...{ k: { ...size } } }"), [("size".to_string(), "number".to_string())]);
        let check = checks("{ ...size }").remove(0);
        assert!(check.is_rejected());
        let [diagnostic] = check.diagnostics(false).try_into().unwrap();
        assert_eq!(diagnostic.code, INVALID_SPREAD_OPERAND_CODE);
        assert_eq!(diagnostic.span, 5..9);
        assert_eq!(diagnostic.message, "object spread requires an object or null, found number");
    }

    #[test]
    fn object_spread_operands_that_may_be_objects_or_null_are_left_to_evaluation() {
        for source in [
            "{ ...null }",
            "{ ...{} }",
            "{ ...obj }",
            // may be null, or of an unknown type
            "{ ...maybe }",
            "{ ...undeclared }",
            "{ ...obj.a }",
            "{ ...(size > 1 ? obj : null) }",
            "{ ...(size > 1 ? obj : size) }",
        ] {
            assert_eq!(spread_problems(source), [] as [(String, String); 0], "{source}");
        }
        // Calls inside an operand are still checked as calls.
        let found = checks("{ ...{ k: min('pear', 1) } }");
        assert!(found.iter().any(|check| check.function == "min" && check.is_rejected()), "{found:#?}");
    }
}

mod typing {
    use super::*;

    #[test]
    fn object_spread_types_follow_written_order() {
        // A later field replaces an earlier one; a literal spread is a literal.
        assert_eq!(type_text("{ a: size, ...{ a: 'x' } }"), type_text("{ a: 'x' }"));
        assert_eq!(type_text("{ ...{ a: 'x' }, a: size }"), type_text("{ a: size }"));
        // A spread of a closed object may write each key it declares (this
        // one may also be null, so even its required key only widens); a
        // key it does not declare keeps its type.
        assert_eq!(
            type_text("{ a: size, b: 'x', c: true, ...obj }"),
            "{a: number(required) | string(required),b: literal(x;required) | number(required),c: literal(true;required)}"
        );
        // An object every alternative of which writes the key replaces it.
        assert_eq!(type_text("{ a: size, ...(size > 1 ? { a: 'x' } : { a: 'y', b: 1 }) }"), type_text("{ a: size > 1 ? 'x' : 'y', b: size > 1 ? null : 1 }"));
        // An unknown operand may write any key: earlier keys widen to
        // unknown and later ones stay known. (The result is also open, which
        // the type text does not show; the member checks below prove it.)
        assert_eq!(type_text("{ a: size, ...undeclared }"), "{a: unknown}");
        assert_eq!(type_text("{ ...undeclared, a: size }"), "{a: number(required)}");
    }

    #[test]
    fn object_spread_member_types_reach_callbacks_and_member_checks() {
        // A later explicit field sets the type, whatever the spread wrote.
        assert_eq!(min_category("map([{ ...undeclared, n: 'x' }], x => min(x.n, 1))"), ArgumentCategory::Incompatible);
        assert_eq!(min_category("map([{ ...named, n: 'x' }], x => min(x.n, 1))"), ArgumentCategory::Incompatible);
        // An earlier field an unknown spread may overwrite is unknown.
        assert_eq!(min_category("map([{ n: 'x', ...undeclared }], x => min(x.n, 1))"), ArgumentCategory::Unknown);
        // A closed spread that does not declare the key leaves it alone.
        assert_eq!(min_category("map([{ n: 'x', ...obj }], x => min(x.n, 1))"), ArgumentCategory::Incompatible);
        // One that may write it widens it with the incoming type.
        assert_eq!(min_category("map([{ n: 1, ...named }], x => min(x.n, 1))"), ArgumentCategory::Uncertain);
        assert_eq!(min_category("map([{ n: 1, ...obj }], x => min(x.n, 1))"), ArgumentCategory::Compatible);

        // Undeclared members: a closed result reports one, an open one never.
        let members = |source: &str| {
            let schema = schema();
            let declared = DeclaredTypes::new(Some(&schema));
            let expr = parse_spanned(source).unwrap();
            expression_member_findings(&expr, source, &TypeScope::unguarded(&declared))
                .into_iter()
                .map(|finding| finding.member)
                .collect::<Vec<_>>()
        };
        assert_eq!(members("map([{ ...obj, k: 1 }], x => x.z)"), ["z"]);
        assert_eq!(members("map([{ ...obj, k: 1 }], x => x.a)"), [] as [String; 0]);
        assert_eq!(members("map([{ k: 1, ...undeclared }], x => x.z)"), [] as [String; 0]);
        assert_eq!(members("map([{ ...undeclared, k: 1 }], x => x.z)"), [] as [String; 0]);
        assert_eq!(members("map([{ ...{ a: 1 }, k: 1 }], x => x.z)"), ["z"]);
    }

    #[test]
    fn object_spread_literal_options_fold_in_written_order() {
        let problem = |source: &str| {
            checks(source)
                .into_iter()
                .find(|check| check.function == "load_data")
                .and_then(|check| check.data_problem)
                .map(|problem| (problem.code, problem.argument))
        };
        const INVALID: Option<(&str, usize)> = Some(("dm.data.invalid_options", 2));
        // A literal spread is read as if written; null adds nothing.
        assert_eq!(problem("load_data('rows.csv', 'tuple([string])[]', { ...{ delimiter: ';;' } })"), INVALID);
        assert_eq!(problem("load_data('rows.csv', 'tuple([string])[]', { ...null, delimiter: ';;' })"), INVALID);
        assert_eq!(problem("load_data('rows.csv', 'tuple([string])[]', { delimiter: ';;', ...{ delimiter: ';' } })"), None);
        // A known bad option written after an opaque spread is still caught;
        // one written before it may be overwritten.
        assert_eq!(problem("load_data('rows.csv', 'tuple([string])[]', { ...opts, delimiter: ';;' })"), INVALID);
        assert_eq!(problem("load_data('rows.csv', 'tuple([string])[]', { delimiter: ';;', ...opts })"), None);
        // An opaque spread may choose the format, so applicability waits.
        assert_eq!(problem("load_data('rows.json', 'string[]', { delimiter: ';' })"), INVALID);
        assert_eq!(problem("load_data('rows.json', 'string[]', { ...opts, delimiter: ';' })"), None);
        // A literal spread of the format is read.
        assert_eq!(problem("load_data(f, 'string[]', { ...{ format: 'json' }, delimiter: ';' })"), INVALID);
    }
}

mod composition {
    use super::*;

    fn compose(dir: &std::path::Path, content: &str) -> Box<darkmatter::markdown::compose::ComposeFailure> {
        let doc = dir.join("doc.md");
        std::fs::write(&doc, content).unwrap();
        let options = ComposeOptions::new().with_source_file(&doc);
        let request = crate::request_support::request(options);
        Markdown::try_from(doc.as_path()).unwrap().compose_with_warnings(&request).expect_err("rejected")
    }

    /// `(code, line, column)` of the failure's one record.
    fn record(failure: &darkmatter::markdown::compose::ComposeFailure) -> (String, Option<usize>, Option<usize>) {
        let [diagnostic] = error_diagnostics(&failure.error).try_into().expect("one record");
        let source = diagnostic.source.expect("a located record");
        (diagnostic.code, source.line, source.column)
    }

    #[test]
    fn object_spread_runtime_failure_is_located_at_its_operand_in_body_and_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        // `count` is undeclared, so only evaluation can judge it.
        let body = compose(dir.path(), "---\ncount: 5\n---\nLine one.\n\nValue: {{ { a: 1, ...count } }}\n");
        assert_eq!(record(&body), (INVALID_SPREAD_OPERAND_CODE.to_string(), Some(6), Some(22)));
        let frontmatter =
            compose(dir.path(), "---\ncount: 5\nmerged: \"{{ { ...{}, ...count } }}\"\n---\nBody.\n");
        assert_eq!(record(&frontmatter), (INVALID_SPREAD_OPERAND_CODE.to_string(), Some(3), Some(25)));
        let callback = compose(
            dir.path(),
            "---\nitems:\n  - a: 1\n  - 5\n---\n{{ map(items, item => { ...item }) }}\n",
        );
        assert_eq!(record(&callback), (INVALID_SPREAD_OPERAND_CODE.to_string(), Some(6), Some(28)));
    }

    #[test]
    fn object_spread_proven_wrong_rejects_the_document_before_anything_runs() {
        let dir = tempfile::tempdir().unwrap();
        let failure = compose(dir.path(), "---\ntitle: x\n---\n{{ upper(title) }} and {{ { ...[1] } }}\n");
        let call = failure.error.rejected_call().expect("the passive check rejected it");
        assert_eq!(call.function, SPREAD_CHECK_NAME);
        assert_eq!(record(&failure), (INVALID_SPREAD_OPERAND_CODE.to_string(), Some(4), Some(32)));
    }
}

mod scans {
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::compose::remote::discover_remote_urls_from_expressions;

    /// Pre-fetch discovery finds a URL read in a spread operand, even when a
    /// later field overwrites every key it supplies, so the read is subject
    /// to the same host policy as any other.
    #[test]
    fn object_spread_operand_urls_are_discovered_even_when_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let content = "{{ { ...{ k: load_data('https://example.com/a.yaml', 'string[]') }, k: 1 } }}\n\n\
                       {{ { ...frontmatter('https://example.org/b.md'), title: 'x' } }}\n";
        let source = ComposeSource::File(dir.path().join("doc.md"));
        let found: Vec<String> =
            discover_remote_urls_from_expressions(content, &source).into_iter().map(|url| url.normalized_key).collect();
        assert_eq!(found, ["https://example.com/a.yaml", "https://example.org/b.md"]);
    }
}
