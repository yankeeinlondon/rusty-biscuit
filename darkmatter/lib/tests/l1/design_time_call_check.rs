//! The design-time call check at the library's public API: the variable-type
//! projection, the five argument-type categories, whole-call classification,
//! and the passive walk over every branch. Nothing here evaluates an
//! expression; the handler-entry counters prove no handler runs.

use std::sync::Arc;

use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, ArgumentType, AuthoredMode, CallCheck, CallProblem, DeclaredTypes, Shape, TypeScope,
    check_expression_calls, check_prepared_calls, parse_spanned, prepare_value,
};
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use serde_json::json;

use ArgumentCategory::{Compatible, Incompatible, NullableMatch, Uncertain, Unknown};

/// The worked example's schema.
const WORKED: &str = "
count: string
maybe: number
size: number(required)
flag: boolean(required)
foo:
    - string(required)
    - number(required)
";

fn schema(yaml: &str) -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).expect("YAML parses")).expect("schema parses")
}

/// Every call check in `expression` under `schema`.
fn checks(schema: Option<&SimplifiedSchema>, expression: &str) -> Vec<CallCheck> {
    let declared = DeclaredTypes::new(schema);
    let expr = parse_spanned(expression).expect("the expression parses");
    check_expression_calls(&expr, &TypeScope::unguarded(&declared))
}

/// The outermost call's category for `expression`.
fn call_category(yaml: &str, expression: &str) -> ArgumentCategory {
    let schema = schema(yaml);
    checks(Some(&schema), expression).first().expect("one call is checked").category()
}

fn register(name: &str, signature: &str) -> TestFunctionGuard {
    let registration = FunctionRegistration::builder(name, signature).build().expect("the test signature compiles");
    hooks::register(registration, Arc::new(|_| Ok(json!(true))))
}

/// The category of passing `source` (declared `src: <source>`) to a
/// one-parameter function of type `target`.
fn pair(source: &str, target: &str) -> ArgumentCategory {
    let _probe = register("probe", &format!("function(parameters(x: {target}); returns(boolean))"));
    call_category(&format!("src: {source}"), "probe(src)")
}

/// The category of passing the literal `literal` to `target`.
fn literal(literal: &str, target: &str) -> ArgumentCategory {
    let _probe = register("probe", &format!("function(parameters(x: {target}); returns(boolean))"));
    call_category("{}", &format!("probe({literal})"))
}

fn shape_text(shape: &Shape) -> String {
    ArgumentType::Typed(shape.clone()).to_string()
}

mod projection {
    use super::*;

    #[test]
    fn the_worked_example_projects_its_declared_types() {
        let schema = schema(WORKED);
        let types = DeclaredTypes::new(Some(&schema)).top_level_types();
        let text: Vec<(String, String)> = types.iter().map(|(name, shape)| (name.clone(), shape_text(shape))).collect();
        assert_eq!(
            text,
            [
                ("count", "string | null"),
                ("flag", "boolean"),
                ("foo", "string | number"),
                ("maybe", "number | null"),
                ("size", "number"),
            ]
            .map(|(name, ty)| (name.to_string(), ty.to_string()))
        );
        assert!(!types.contains_key("untyped_var"), "an undeclared name is unknown");
    }

    #[test]
    fn no_typed_projection_makes_every_variable_unknown() {
        assert!(DeclaredTypes::new(None).top_level_types().is_empty());
        assert_eq!(checks(None, "min(size, 5)")[0].arguments[0].category, Unknown);
    }

    #[test]
    fn a_mixed_root_union_projects_nothing() {
        let schema = schema("- size: number(required)\n- ./other.schema.json\n");
        assert!(DeclaredTypes::new(Some(&schema)).top_level_types().is_empty());
        assert_eq!(checks(Some(&schema), "min(size, 5)")[0].arguments[0].category, Unknown);
    }

    #[test]
    fn presence_follows_the_shared_rule() {
        let types = DeclaredTypes::new(Some(&schema(
            "
union_required:
    - string(required)
    - number
null_arm:
    - string(required)
    - 'null'
anything: unknown(required)
",
        )))
        .top_level_types();
        assert_eq!(shape_text(&types["union_required"]), "string | number");
        assert_eq!(shape_text(&types["null_arm"]), "string | null");
        assert!(matches!(types["anything"], Shape::Unknown { .. }));
    }

    #[test]
    fn projection_is_deterministic_and_reads_only_the_schema() {
        let first = schema(WORKED);
        let reordered = schema(
            "
foo:
    - number(required)
    - string(required)
size: number(required)
flag: boolean(required)
maybe: number
count: string
",
        );
        let left = DeclaredTypes::new(Some(&first)).top_level_types();
        let right = DeclaredTypes::new(Some(&reordered)).top_level_types();
        assert_eq!(left.keys().collect::<Vec<_>>(), right.keys().collect::<Vec<_>>());
        for (name, shape) in &left {
            assert_eq!(shape.admits_null(), right[name].admits_null(), "{name}");
        }
    }
}

mod categories {
    use super::*;

    /// Rows a–e of the worked example, and rows f and i (not rejected in
    /// this phase: the outer argument is an expression, so unknown).
    #[test]
    fn the_worked_example_rows() {
        for (expression, expected) in [
            ("min(size, 5)", Compatible),
            ("min(maybe, 5)", NullableMatch),
            ("min(count, 5)", Uncertain),
            ("min(flag, 5)", Incompatible),
            ("min(untyped_var, 5)", Unknown),
            ("min(length(count), 5)", Unknown),
            ("min(maybe || 0, 5)", Unknown),
        ] {
            assert_eq!(call_category(WORKED, expression), expected, "{expression}");
        }
        // Row f's inner call: `count` may be null, which `length` rejects.
        let row_f = checks(Some(&schema(WORKED)), "min(length(count), 5)");
        assert_eq!(row_f[1].function, "length");
        assert_eq!(row_f[1].category(), NullableMatch);
    }

    /// Rows g and h: the guard narrows `foo` in the branch that calls `min`.
    #[test]
    fn guards_narrow_the_argument() {
        let schema = schema(WORKED);
        for expression in ["is_number(foo) ? min(foo, 5) : 0", "is_string(foo) ? 0 : min(foo, 5)"] {
            let found = checks(Some(&schema), expression);
            let min = found.iter().find(|check| check.function == "min").expect("min is checked");
            assert_eq!(min.category(), Compatible, "{expression}");
        }
        let unguarded = checks(Some(&schema), "min(foo, 5)");
        assert_eq!(unguarded[0].category(), Uncertain);
    }

    #[test]
    fn criterion_36_pairs() {
        assert_eq!(pair("number(required)", "string"), Compatible);
        assert_eq!(pair("boolean(required)", "number"), Incompatible);
        assert_eq!(pair("object(required)", "number"), Incompatible);
        assert_eq!(pair("numberlike(required)", "number"), Compatible);
        assert_eq!(pair("string(required)", "numberlike"), Uncertain);
        assert_eq!(pair("number(required)", "numberlike"), Compatible);
        assert_eq!(pair("numberlike(required)", "string"), Compatible);
        assert_eq!(pair("string(required)", "file"), Compatible);
        assert_eq!(pair("string(required)", "string(min(3); max(3))"), Compatible);
        assert_eq!(pair("number(required)", "string(min(3); max(3))"), Compatible);
        assert_eq!(pair("number(required)", "number(integer)"), Compatible);
        assert_eq!(pair("number(required)", "enum(red, green)"), Compatible);
        assert_eq!(pair("boolish(required)", "boolean"), Compatible);
        assert_eq!(pair("string(required)", "boolean"), Uncertain);
        assert_eq!(literal("'4'", "number"), Compatible);
        assert_eq!(literal("2026", "string"), Compatible);
        assert_eq!(literal("'pear'", "number"), Incompatible);
        assert_eq!(literal("'purple'", "enum(red, green)"), Incompatible);
        assert_eq!(literal("0", "number(min(1))"), Incompatible);
        assert_eq!(call_category("{}", "pr_list(0)"), Incompatible);
        assert_eq!(call_category("{}", "min('pear', 5)"), Incompatible);
    }

    #[test]
    fn three_letter_acronym_takes_a_typed_number() {
        let _tla = register("three_letter_acronym", "function(parameters(tla: string(min(3); max(3))); returns(string))");
        assert_eq!(call_category("lookup: number(required)", "three_letter_acronym(lookup)"), Compatible);
        assert_eq!(call_category("lookup: string", "three_letter_acronym(lookup)"), NullableMatch);
        assert_eq!(call_category("{}", "three_letter_acronym(lookup)"), Unknown);
    }

    #[test]
    fn unions_quantify_over_their_options() {
        assert_eq!(pair("[number(required), boolean(required)]", "number"), Uncertain);
        assert_eq!(pair("[boolean(required), object(required)]", "number"), Incompatible);
        assert_eq!(pair("[number(required), numberlike(required)]", "number"), Compatible);
        assert_eq!(pair("number", "number | null"), Compatible);
        assert_eq!(pair("boolean", "number | null"), Uncertain);
    }

    #[test]
    fn null_is_accepted_only_by_an_explicit_null_arm() {
        assert_eq!(pair("number", "number | null"), Compatible);
        assert_eq!(pair("number", "unknown"), NullableMatch);
        assert_eq!(pair("number(required)", "unknown"), Compatible);
        assert_eq!(pair("number", "unknown | null"), Compatible);
        // Any source, even an unknown one, fits `unknown | null`.
        assert_eq!(call_category("{}", "is_number(untyped)"), Compatible);
        assert_eq!(call_category("{}", "is_number(upper('a'))"), Compatible);
        assert_eq!(call_category("{}", "and(false, upper('a'))"), Compatible);
        assert_eq!(pair("unknown", "unknown"), Unknown);
        assert_eq!(literal("null", "number"), Incompatible);
        assert_eq!(literal("null", "number | null"), Compatible);
        // `file_exists(value: unknown | null)` inspects anything.
        assert_eq!(call_category("n: number(required)", "file_exists(n)"), Compatible);
        assert_eq!(call_category("{}", "file_exists(42)"), Compatible);
    }

    /// Criterion 52: content formats classify as binding does.
    #[test]
    fn content_formats() {
        assert_eq!(pair("string(required)", "json"), Uncertain);
        assert_eq!(literal(r#"'{"a": 1}'"#, "json"), Compatible);
        assert_eq!(literal("'{a'", "json"), Incompatible);
        assert_eq!(pair("object(required)", "json"), Compatible);
        assert_eq!(pair("number(required)", "yaml"), Compatible);
        assert_eq!(literal("null", "json"), Incompatible);
        assert_eq!(pair("number", "json"), NullableMatch);
    }

    #[test]
    fn containers_reason_about_their_elements() {
        assert_eq!(pair("number[](required)", "number[]"), Compatible);
        assert_eq!(pair("string[](required)", "number[]"), Uncertain);
        // An empty list binds whatever its element type.
        assert_eq!(pair("object[](required)", "number[]"), Uncertain);
        assert_eq!(pair("number[](required)", "number"), Incompatible);
        assert_eq!(literal("[1, '2']", "number[]"), Compatible);
        assert_eq!(literal("[true]", "number[]"), Incompatible);
    }

    #[test]
    fn source_exclusions_are_part_of_classification() {
        assert_eq!(call_category("{}", "length(true)"), Incompatible);
        assert_eq!(call_category("b: boolean(required)", "length(b)"), Incompatible);
        assert_eq!(call_category("{}", "length('true')"), Compatible);
        assert_eq!(call_category("{}", "length(2.5)"), Compatible);
        assert_eq!(call_category("x: [boolean(required), string(required)]", "length(x)"), Uncertain);
    }
}

mod whole_calls {
    use super::*;

    #[test]
    fn wrong_argument_counts_range_the_call() {
        let found = checks(None, "min(1)");
        assert_eq!(found[0].category(), Incompatible);
        assert!(matches!(found[0].problem, Some(CallProblem::Arity { supplied: 1, .. })));
        let findings = found[0].findings();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].span, 0..6);
        assert_eq!(findings[0].argument, None);
    }

    #[test]
    fn an_argument_finding_ranges_the_argument_and_names_both_types() {
        let found = checks(Some(&schema(WORKED)), "min(flag, 5)");
        let findings = found[0].findings();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].span, 4..8);
        assert_eq!(findings[0].argument, Some(0));
        let message = &findings[0].message;
        for part in ["min", "`a`", "boolean", "number", "category 5"] {
            assert!(message.contains(part), "{message} names {part}");
        }
    }

    /// Criterion 51: recovery agrees with binding.
    #[test]
    fn numeric_recovery() {
        for expression in [
            "number('pear', 7)",
            "number(true, 7)",
            "number(null, 7)",
            "round('pear', 2.7)",
            "round(true, 7)",
        ] {
            assert_eq!(call_category("{}", expression), Compatible, "{expression}");
        }
        for expression in ["number('4', 'x')", "number('pear')", "number()", "round()"] {
            assert_eq!(call_category("{}", expression), Incompatible, "{expression}");
        }
        assert_eq!(call_category("flag: boolean(required)", "number(flag, 7)"), Compatible);
        assert_eq!(call_category("flag: boolean(required)", "number(flag)"), Incompatible);
        assert_eq!(call_category("flag: boolean(required)", "number(flag, 'x')"), Incompatible);
        assert_eq!(
            call_category("flag: boolean(required)\nmaybe: number", "number(flag, maybe)"),
            NullableMatch
        );
    }

    #[test]
    fn a_literal_call_reports_the_binders_own_error() {
        let found = checks(None, "min('pear', 5)");
        let error = found[0].binding_error.as_ref().expect("the binder's error is kept");
        assert!(error.to_string().contains("min"), "{error}");
    }
}

mod walk {
    use super::*;

    #[test]
    fn branches_that_do_not_run_are_checked() {
        let schema = schema(WORKED);
        for expression in [
            "true ? 'ok' : min(flag, 5)",
            "size || min(flag, 5)",
            "false && min(flag, 5)",
            "and(false, min(flag, 5))",
            "or(true, min(flag, 5))",
        ] {
            let found = checks(Some(&schema), expression);
            assert!(found.iter().any(CallCheck::is_rejected), "{expression}");
        }
    }

    #[test]
    fn checking_never_enters_a_handler() {
        let _effect = register("effect", "function(parameters(void); returns(boolean))");
        hooks::reset_handler_entries();
        let found = checks(Some(&schema(WORKED)), "and(false, effect()) || or(true, effect(), min(flag, 5))");
        assert!(found.iter().any(CallCheck::is_rejected));
        assert_eq!(hooks::handler_entries("effect"), 0);
        assert_eq!(hooks::handler_entries("min"), 0);
    }

    #[test]
    fn fm_spellings_reject_like_the_bare_name() {
        let schema = schema("flag: boolean(required)\nenv: boolean(required)");
        for expression in ["min(flag, 5)", "min(fm.flag, 5)", "min(fm['flag'], 5)", "min(fm.env, 5)"] {
            assert_eq!(checks(Some(&schema), expression)[0].category(), Incompatible, "{expression}");
        }
        // Bare `env` is the reserved root, not the document property.
        assert_eq!(checks(Some(&schema), "min(env, 5)")[0].category(), Unknown);
        assert_eq!(checks(Some(&schema), "min(flag.inner, 5)")[0].category(), Unknown);
    }

    #[test]
    fn unknown_functions_are_left_to_unknown_function_validation() {
        assert!(checks(None, "no_such_function(1)").is_empty());
    }

    #[test]
    fn prepared_values_locate_findings_in_their_leaf() {
        let schema = schema(WORKED);
        let declared = DeclaredTypes::new(Some(&schema));
        let prepared = prepare_value(
            &json!({"a": "ok", "b": ["x {{ min(flag, 5) }}"], "c": "{{ min(size, 5) }}"}),
            AuthoredMode::Subtree,
        )
        .expect("prepares");
        let found = check_prepared_calls(&prepared, &TypeScope::unguarded(&declared));
        assert_eq!(found.len(), 1, "only the non-category-1 call");
        assert_eq!(found[0].pointer, "/b/0");
        assert_eq!(found[0].expression, "min(flag, 5)");
        assert_eq!(found[0].check.argument_spans[0], 9..13);
    }
}
