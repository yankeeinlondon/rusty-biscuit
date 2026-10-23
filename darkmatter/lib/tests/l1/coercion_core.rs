//! The coercion rule core through its public API: every conversion and
//! rejected conversion, numeric and boolean text boundaries, constraints,
//! `numberlike`/`boolish`, content formats, containers, the union rule in
//! every option order, overlapping object declarations, exact checks,
//! deferral, the depth bound, and error rendering.

use darkmatter::markdown::coercion::{
    Additional, ArrayShape, Assertion, BindOptions, Bound, Constraint, ContentFormat, ConversionError, FailureReason,
    IntegerRule, KeyMatcher, NumberShape, NumericBounds, ObjectShape, PathSegment, PatternScope, PatternSlot,
    PropertySlot, Shape, TextPattern, TextShape, TupleShape, TupleSlot, ValueKind, accepts_exactly, bind, bind_with,
    check_exact, render_path,
};
use darkmatter::markdown::numeric::NumericAdmissionError;
use serde_json::{Value, json};

// ── shape builders ──────────────────────────────────────────────────────────

fn number() -> Shape {
    Shape::number()
}

fn number_with(bounds: NumericBounds) -> Shape {
    Shape::Number(NumberShape { as_sent: false, bounds })
}

fn integer() -> Shape {
    number_with(NumericBounds {
        integer: Some(IntegerRule::WholeBelow2Pow64),
        ..NumericBounds::default()
    })
}

fn numberlike() -> Shape {
    Shape::Number(NumberShape {
        as_sent: true,
        bounds: NumericBounds::default(),
    })
}

fn boolean() -> Shape {
    Shape::boolean()
}

fn boolish() -> Shape {
    Shape::Boolean { as_sent: true }
}

fn text() -> Shape {
    Shape::text()
}

fn text_length(min: usize, max: usize) -> Shape {
    Shape::Text(TextShape {
        min_length: Some(min),
        max_length: Some(max),
        ..TextShape::default()
    })
}

fn members(values: &[&str]) -> Shape {
    Shape::Text(TextShape {
        members: Some(values.iter().map(|value| value.to_string()).collect()),
        ..TextShape::default()
    })
}

fn json_content() -> Shape {
    Shape::Content(ContentFormat::Json)
}

fn yaml_content() -> Shape {
    Shape::Content(ContentFormat::Yaml)
}

fn tuple(slots: Vec<(Shape, bool)>, rest: Option<Shape>) -> Shape {
    Shape::Tuple(TupleShape {
        prefix: slots
            .into_iter()
            .map(|(shape, optional)| TupleSlot { shape, optional })
            .collect(),
        rest: rest.map(Box::new),
    })
}

fn object(properties: Vec<(&str, Shape)>) -> Shape {
    Shape::Object(ObjectShape {
        properties: properties
            .into_iter()
            .map(|(name, shape)| PropertySlot {
                name: name.to_string(),
                shape,
                required: true,
            })
            .collect(),
        ..ObjectShape::opaque()
    })
}

fn union(options: Vec<Shape>) -> Shape {
    Shape::Union(options)
}

// ── outcome helpers ─────────────────────────────────────────────────────────

/// The bound value, or the failure reasons, as one comparable outcome.
#[derive(Debug, Clone, PartialEq)]
enum Outcome {
    Kept,
    Converted(Value),
    Failed(Vec<FailureReason>),
}

fn outcome(shape: &Shape, value: &Value) -> Outcome {
    let before = value.clone();
    let result = bind(shape, value);
    assert_eq!(value, &before, "binding never mutates the original");
    match result {
        Ok(Bound::Kept(kept)) => {
            assert!(std::ptr::eq(kept, value), "a kept value is the original, not a copy");
            Outcome::Kept
        }
        Ok(Bound::Converted(converted)) => Outcome::Converted(converted),
        Err(error) => Outcome::Failed(error.failures.into_iter().map(|failure| failure.reason).collect()),
    }
}

fn converted(shape: &Shape, value: Value) -> Value {
    match outcome(shape, &value) {
        Outcome::Converted(converted) => converted,
        other => panic!("{shape} given {value}: expected a conversion, got {other:?}"),
    }
}

fn assert_kept(shape: &Shape, value: Value) {
    assert_eq!(outcome(shape, &value), Outcome::Kept, "{shape} given {value}");
}

fn error(shape: &Shape, value: Value) -> ConversionError {
    match bind(shape, &value) {
        Err(error) => error,
        Ok(bound) => panic!("{shape} given {value}: expected an error, got {bound:?}"),
    }
}

fn only_reason(shape: &Shape, value: Value) -> FailureReason {
    let error = error(shape, value);
    assert_eq!(error.failures.len(), 1, "{error}");
    error.failures.into_iter().next().map(|failure| failure.reason).expect("one failure")
}

fn malformed(reason: &FailureReason) -> bool {
    matches!(reason, FailureReason::MalformedText { .. })
}

/// Every ordering of `items`.
fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut all = Vec::new();
    for index in 0..items.len() {
        let mut rest = items.to_vec();
        let first = rest.remove(index);
        for mut tail in permutations(&rest) {
            tail.insert(0, first.clone());
            all.push(tail);
        }
    }
    all
}

/// The union outcome, asserted identical for every option order. Ambiguity
/// readings are compared as a set, since they are listed in option order.
fn union_outcome(options: Vec<Shape>, value: Value) -> Outcome {
    let mut seen: Option<Outcome> = None;
    for ordering in permutations(&options) {
        let current = normalize(outcome(&union(ordering.clone()), &value));
        match &seen {
            None => seen = Some(current),
            Some(first) => assert_eq!(&current, first, "option order changed the result for {value}"),
        }
    }
    seen.expect("at least one ordering")
}

fn normalize(outcome: Outcome) -> Outcome {
    match outcome {
        Outcome::Failed(reasons) => Outcome::Failed(
            reasons
                .into_iter()
                .map(|reason| match reason {
                    FailureReason::AmbiguousUnion { mut readings } => {
                        readings.sort_by_key(|reading| reading.to_string());
                        FailureReason::AmbiguousUnion { readings }
                    }
                    FailureReason::ConflictingDeclarations { mut readings } => {
                        readings.sort_by_key(|reading| reading.to_string());
                        FailureReason::ConflictingDeclarations { readings }
                    }
                    FailureReason::NoOptionAccepts { .. } => FailureReason::NoOptionAccepts { options: Vec::new() },
                    other => other,
                })
                .collect(),
        ),
        other => other,
    }
}

// ── conversions ─────────────────────────────────────────────────────────────

#[test]
fn text_converts_to_number_under_the_lenient_spellings() {
    for (text, expected) in [
        (" 4 ", json!(4)),
        ("+4", json!(4)),
        ("1e3", json!(1000)),
        (".5", json!(0.5)),
        ("1_000", json!(1000)),
        ("-3.5", json!(-3.5)),
        ("04", json!(4)),
        ("4.0", json!(4)),
        ("0.1", json!(0.1)),
        ("9007199254740992", json!(9007199254740992_u64)),
        ("-9007199254740992", json!(-9007199254740992_i64)),
    ] {
        assert_eq!(converted(&number(), json!(text)), expected, "{text:?}");
    }
}

#[test]
fn text_that_is_not_a_number_spelling_is_a_failed_conversion() {
    for text in ["5.", "1,000", "NaN", "0x10", "inf", "pear", "", "1__0", "_1"] {
        let reason = only_reason(&number(), json!(text));
        assert!(malformed(&reason), "{text:?}: {reason:?}");
    }
}

#[test]
fn number_text_must_be_exact_finite_and_not_underflow() {
    for (text, expected) in [
        ("9007199254740993", NumericAdmissionError::Inexact),
        ("-9007199254740993", NumericAdmissionError::Inexact),
        ("18446744073709551615", NumericAdmissionError::Inexact),
        ("99999999999999999999", NumericAdmissionError::Inexact),
        ("9007199254740993.0", NumericAdmissionError::Inexact),
        ("9.007199254740993e15", NumericAdmissionError::Inexact),
        ("1e400", NumericAdmissionError::NotFinite),
        ("1e-400", NumericAdmissionError::Underflow),
    ] {
        assert_eq!(
            only_reason(&number(), json!(text)),
            FailureReason::NumberNotAdmitted(expected),
            "{text}"
        );
    }
}

#[test]
fn a_native_integer_no_f64_holds_is_rejected() {
    let inexact = Value::from(9007199254740993_u64);
    assert_eq!(
        only_reason(&number(), inexact.clone()),
        FailureReason::NumberNotAdmitted(NumericAdmissionError::Inexact)
    );
    assert_eq!(
        only_reason(&text(), inexact),
        FailureReason::NumberNotAdmitted(NumericAdmissionError::Inexact),
        "number → text never formats a value Darkmatter cannot hold"
    );
    assert_kept(&number(), Value::from(9007199254740992_u64));
}

#[test]
fn text_converts_to_boolean_words_in_any_case() {
    for (text, expected) in [("Yes", true), ("OFF", false), (" on ", true), ("1", true), ("0", false), ("tRuE", true)] {
        assert_eq!(converted(&boolean(), json!(text)), json!(expected), "{text:?}");
    }
    for text in ["maybe", "y", "o n", "2", ""] {
        assert!(malformed(&only_reason(&boolean(), json!(text))), "{text:?}");
    }
}

#[test]
fn numbers_and_booleans_convert_to_their_one_text_form() {
    assert_eq!(converted(&text(), json!(2026)), json!("2026"));
    assert_eq!(converted(&text(), json!(1.2)), json!("1.2"));
    assert_eq!(converted(&text(), json!(1.0)), json!("1"));
    assert_eq!(converted(&text(), json!(true)), json!("true"));
    assert_eq!(converted(&text(), json!(false)), json!("false"));
    assert_kept(&text(), json!("as written"));
}

#[test]
fn rejected_conversions_are_type_errors() {
    let cases = [
        (boolean(), json!(1)),
        (number(), json!(true)),
        (number(), json!(null)),
        (text(), json!(null)),
        (boolean(), json!(null)),
        (json_content(), json!(null)),
        (yaml_content(), json!(null)),
        (Shape::array(text()), json!("a")),
        (Shape::array(number()), json!("[1, 2]")),
        (object(vec![("a", number())]), json!("{\"a\": 1}")),
        (text(), json!(["a"])),
        (text(), json!({"a": 1})),
        (boolish(), json!(1)),
    ];
    for (shape, value) in cases {
        assert_eq!(only_reason(&shape, value.clone()), FailureReason::NoConversion, "{shape} given {value}");
    }
}

#[test]
fn alphanumeric_text_refuses_native_scalars() {
    let alphanumeric = Shape::Text(TextShape {
        scalar_text: false,
        ..TextShape::default()
    });
    assert_eq!(only_reason(&alphanumeric, json!(5)), FailureReason::NoConversion);
    assert_eq!(only_reason(&alphanumeric, json!(true)), FailureReason::NoConversion);
    assert_kept(&alphanumeric, json!("abc1"));
}

// ── constraints ─────────────────────────────────────────────────────────────

#[test]
fn constraints_are_enforced_never_repaired() {
    assert_eq!(
        only_reason(&integer(), json!("2.5")),
        FailureReason::ConstraintNotMet(Constraint::Integer(IntegerRule::WholeBelow2Pow64))
    );
    let three = text_length(3, 3);
    assert_eq!(only_reason(&three, json!("ABCD")), FailureReason::ConstraintNotMet(Constraint::MaxLength(3)));
    assert_eq!(converted(&three, json!(123)), json!("123"));
    let failure = error(&three, json!(1234)).failures.remove(0);
    assert_eq!(failure.converted, Some(json!("1234")));
    assert_eq!(failure.reason, FailureReason::ConstraintNotMet(Constraint::MaxLength(3)));
    assert_eq!(converted(&integer(), json!("4.0")), json!(4));
}

#[test]
fn lengths_count_unicode_characters() {
    assert_kept(&text_length(3, 3), json!("äöü"));
}

#[test]
fn the_simplified_integer_rule_has_a_2_pow_64_ceiling_and_the_json_schema_rule_does_not() {
    let two_pow_64 = json!("18446744073709551616");
    assert_eq!(
        only_reason(&integer(), two_pow_64.clone()),
        FailureReason::ConstraintNotMet(Constraint::Integer(IntegerRule::WholeBelow2Pow64))
    );
    let whole = number_with(NumericBounds {
        integer: Some(IntegerRule::Whole),
        ..NumericBounds::default()
    });
    assert_eq!(converted(&whole, two_pow_64), json!(18446744073709551616.0_f64));
    assert_eq!(
        only_reason(&whole, json!(2.5)),
        FailureReason::ConstraintNotMet(Constraint::Integer(IntegerRule::Whole))
    );
}

#[test]
fn numeric_bounds_filter_the_converted_value() {
    let bounded = number_with(NumericBounds {
        minimum: Some(1.0),
        exclusive_maximum: Some(10.0),
        multiple_of: Some(2.0),
        ..NumericBounds::default()
    });
    assert_eq!(converted(&bounded, json!("4")), json!(4));
    assert_eq!(only_reason(&bounded, json!("3")), FailureReason::ConstraintNotMet(Constraint::MultipleOf(2.0)));
    assert_eq!(only_reason(&bounded, json!("0")), FailureReason::ConstraintNotMet(Constraint::Minimum(1.0)));
    assert_eq!(
        only_reason(&bounded, json!(10)),
        FailureReason::ConstraintNotMet(Constraint::ExclusiveMaximum(10.0))
    );
    let positive = number_with(NumericBounds {
        exclusive_minimum: Some(0.0),
        maximum: Some(5.0),
        ..NumericBounds::default()
    });
    assert_eq!(only_reason(&positive, json!(0)), FailureReason::ConstraintNotMet(Constraint::ExclusiveMinimum(0.0)));
    assert_eq!(only_reason(&positive, json!("6")), FailureReason::ConstraintNotMet(Constraint::Maximum(5.0)));
}

#[test]
fn enum_and_pattern_text_accept_converted_scalars_then_check() {
    let one_or_two = members(&["1", "2"]);
    assert_eq!(converted(&one_or_two, json!(2)), json!("2"));
    assert_eq!(
        only_reason(&one_or_two, json!(3)),
        FailureReason::ConstraintNotMet(Constraint::Members(vec!["1".into(), "2".into()]))
    );
    let digits = Shape::Text(TextShape {
        pattern: Some(TextPattern::new("^[0-9]+$").expect("pattern")),
        ..TextShape::default()
    });
    assert_eq!(converted(&digits, json!(42)), json!("42"));
    assert_eq!(
        only_reason(&digits, json!(" 42")),
        FailureReason::ConstraintNotMet(Constraint::Pattern("^[0-9]+$".into()))
    );
}

#[test]
fn literals_accept_the_one_direct_conversion_then_compare() {
    assert_eq!(converted(&Shape::Literal(json!("2")), json!(2)), json!("2"));
    assert_eq!(converted(&Shape::Literal(json!(5)), json!("5")), json!(5));
    assert_eq!(converted(&Shape::Literal(json!(true)), json!("yes")), json!(true));
    assert_kept(&Shape::Literal(json!(5)), json!(5.0));
    assert_eq!(
        only_reason(&Shape::Literal(json!(5)), json!("6")),
        FailureReason::ConstraintNotMet(Constraint::Literal(json!(5)))
    );
    assert_eq!(only_reason(&Shape::Literal(json!(1)), json!(true)), FailureReason::NoConversion);
    assert!(malformed(&only_reason(&Shape::Literal(json!(true)), json!("maybe"))));
}

#[test]
fn assertions_filter_without_converting() {
    let dated = text().assert(Assertion::new("date", |value| {
        value.as_str().is_some_and(|text| text.len() == 10 && text.as_bytes()[4] == b'-')
    }));
    assert_kept(&dated, json!("2026-01-01"));
    let failure = error(&dated, json!(2026)).failures.remove(0);
    assert_eq!(failure.converted, Some(json!("2026")));
    assert_eq!(failure.reason, FailureReason::AssertionFailed { description: "date".into() });
    assert_eq!(failure.expected, "string & date");
    let even = number().assert(Assertion::new("even", |value| value.as_f64().is_some_and(|n| n % 2.0 == 0.0)));
    assert_eq!(converted(&even, json!("4")), json!(4));
    assert!(bind(&even, &json!("3")).is_err());
}

// ── numberlike and boolish ──────────────────────────────────────────────────

#[test]
fn numberlike_and_boolish_keep_what_they_accept_as_sent() {
    for value in [json!(5), json!("5"), json!("02134"), json!(" 4 "), json!("1e3"), json!("1_000")] {
        assert_kept(&numberlike(), value);
    }
    assert!(malformed(&only_reason(&numberlike(), json!("pear"))));
    assert_eq!(only_reason(&numberlike(), json!(true)), FailureReason::NoConversion);
    assert_eq!(only_reason(&numberlike(), json!(null)), FailureReason::NoConversion);
    assert_eq!(
        only_reason(&numberlike(), json!("9007199254740993")),
        FailureReason::NumberNotAdmitted(NumericAdmissionError::Inexact)
    );
    for value in [json!(true), json!("Yes"), json!(" off ")] {
        assert_kept(&boolish(), value);
    }
    assert_eq!(only_reason(&boolish(), json!(1)), FailureReason::NoConversion);
    assert!(malformed(&only_reason(&boolish(), json!("maybe"))));
}

#[test]
fn numberlike_bounds_apply_to_text_as_sent() {
    let small = Shape::Number(NumberShape {
        as_sent: true,
        bounds: NumericBounds {
            maximum: Some(10.0),
            ..NumericBounds::default()
        },
    });
    assert_kept(&small, json!("7"));
    let failure = error(&small, json!("70")).failures.remove(0);
    assert_eq!(failure.reason, FailureReason::ConstraintNotMet(Constraint::Maximum(10.0)));
    assert_eq!(failure.converted, None, "numberlike never converts");
}

// ── content formats ─────────────────────────────────────────────────────────

#[test]
fn native_values_serialize_into_json_and_yaml_in_written_order() {
    let value: Value = serde_json::from_str(r#"{"port": 8080, "debug": true, "a": [1, 2]}"#).expect("json");
    assert_eq!(
        converted(&json_content(), value.clone()),
        json!(r#"{"port":8080,"debug":true,"a":[1,2]}"#)
    );
    assert_eq!(converted(&yaml_content(), value), json!("port: 8080\ndebug: true\na:\n- 1\n- 2"));
    assert_eq!(converted(&json_content(), json!(5)), json!("5"));
    assert_eq!(converted(&yaml_content(), json!(true)), json!("true"));
}

#[test]
fn text_given_to_a_content_format_is_validated_and_kept() {
    assert_kept(&json_content(), json!(r#"{"a":1}"#));
    assert_kept(&json_content(), json!("true"));
    assert_kept(&yaml_content(), json!("port: 8080"));
    for text in ["{a", r#"{"a":1,"a":2}"#, "1e400", r#"{"a":1} trailing"#, "title: Foo"] {
        assert!(malformed(&only_reason(&json_content(), json!(text))), "{text}");
    }
    for text in ["a: 1\na: 2", "a: [1"] {
        assert!(malformed(&only_reason(&yaml_content(), json!(text))), "{text}");
    }
}

// ── containers ──────────────────────────────────────────────────────────────

fn person() -> Shape {
    tuple(vec![(text(), false), (number(), false), (boolean(), true)], None)
}

#[test]
fn the_person_tuple_table() {
    assert_eq!(converted(&person(), json!(["Ada", "36", "yes"])), json!(["Ada", 36, true]));
    assert_eq!(converted(&person(), json!(["Ada", "36"])), json!(["Ada", 36]));
    let failure = error(&person(), json!(["Ada"])).failures.remove(0);
    assert_eq!(failure.path_text(), "[1]");
    assert_eq!(render_path("p", &failure.path), "p[1]");
    assert_eq!(failure.reason, FailureReason::MissingRequired);
    assert_eq!(failure.received, None);
    assert_eq!(failure.expected, "number");
    assert_eq!(only_reason(&person(), json!(["Ada", 36, true, "extra"])), FailureReason::UnexpectedElement);
}

#[test]
fn tuple_rest_elements_bind_to_the_rest_shape() {
    let shape = tuple(vec![(text(), false)], Some(number()));
    assert_eq!(converted(&shape, json!(["a", "1", "2"])), json!(["a", 1, 2]));
    assert_kept(&shape, json!(["a"]));
    let failure = error(&shape, json!(["a", "x"])).failures.remove(0);
    assert_eq!(failure.path, vec![PathSegment::Index(1)]);
}

#[test]
fn the_sum_list_table() {
    let sum = Shape::array(number());
    let failure = error(&sum, json!(["1", "2", "x"])).failures.remove(0);
    assert_eq!(render_path("values", &failure.path), "values[2]");
    assert_eq!(converted(&sum, json!(["1", "2", "3"])), json!([1, 2, 3]));
    assert_eq!(only_reason(&sum, json!("1")), FailureReason::NoConversion);
}

#[test]
fn every_failing_path_is_reported() {
    let failures = error(&Shape::array(number()), json!(["x", "1", "y", true])).failures;
    let paths: Vec<String> = failures.iter().map(|failure| failure.path_text()).collect();
    assert_eq!(paths, ["[0]", "[2]", "[3]"]);
    let shape = object(vec![("a", number()), ("b", boolean()), ("c", text())]);
    let failures = error(&shape, json!({"a": "pear", "b": "maybe"})).failures;
    let paths: Vec<String> = failures.iter().map(|failure| failure.path_text()).collect();
    assert_eq!(paths, ["a", "b", "c"]);
}

#[test]
fn list_constraints_apply_to_the_final_list() {
    let shape = Shape::Array(ArrayShape {
        items: Box::new(number()),
        min_items: Some(2),
        max_items: None,
        unique_items: true,
    });
    assert_eq!(only_reason(&shape, json!(["1"])), FailureReason::ConstraintNotMet(Constraint::MinItems(2)));
    assert_eq!(only_reason(&shape, json!(["1", 1])), FailureReason::ConstraintNotMet(Constraint::UniqueItems));
    assert_eq!(converted(&shape, json!(["1", 2])), json!([1, 2]));
}

#[test]
fn nested_objects_convert_at_every_declared_depth() {
    let shape = object(vec![("meta", object(vec![("inner", object(vec![("count", number())]))]))]);
    assert_eq!(
        converted(&shape, json!({"meta": {"inner": {"count": "3"}, "extra": "kept"}})),
        json!({"meta": {"inner": {"count": 3}, "extra": "kept"}})
    );
    let failure = error(&shape, json!({"meta": {"inner": {"count": "pear"}}})).failures.remove(0);
    assert_eq!(failure.path_text(), "meta.inner.count");
}

#[test]
fn missing_optional_properties_stay_missing_and_required_ones_fail() {
    let shape = Shape::Object(ObjectShape {
        properties: vec![
            PropertySlot {
                name: "a".into(),
                shape: number(),
                required: true,
            },
            PropertySlot {
                name: "b".into(),
                shape: number(),
                required: false,
            },
        ],
        ..ObjectShape::opaque()
    });
    assert_eq!(converted(&shape, json!({"a": "1"})), json!({"a": 1}));
    let failure = error(&shape, json!({"b": 1})).failures.remove(0);
    assert_eq!((failure.path_text(), failure.reason), ("a".to_string(), FailureReason::MissingRequired));
}

fn dictionary(properties: Vec<(&str, Shape)>, patterns: Vec<(KeyMatcher, Shape)>, scope: PatternScope) -> Shape {
    let Shape::Object(mut shape) = object(properties) else {
        unreachable!("object() builds an object")
    };
    shape.patterns = patterns
        .into_iter()
        .map(|(matcher, shape)| PatternSlot { matcher, shape })
        .collect();
    shape.pattern_scope = scope;
    Shape::Object(shape)
}

#[test]
fn pattern_key_dictionary_values_convert() {
    let shape = dictionary(vec![], vec![(KeyMatcher::Any, number())], PatternScope::UndeclaredKeys);
    assert_eq!(converted(&shape, json!({"x": "3", "y": 4})), json!({"x": 3, "y": 4}));
    let prefixed = dictionary(
        vec![],
        vec![(KeyMatcher::Prefix("n_".into()), number()), (KeyMatcher::Suffix("_b".into()), boolean())],
        PatternScope::UndeclaredKeys,
    );
    assert_eq!(
        converted(&prefixed, json!({"n_a": "1", "x_b": "yes", "other": "kept"})),
        json!({"n_a": 1, "x_b": true, "other": "kept"})
    );
}

#[test]
fn undeclared_key_patterns_skip_declared_properties() {
    let shape = dictionary(vec![("name", text())], vec![(KeyMatcher::Any, number())], PatternScope::UndeclaredKeys);
    assert_eq!(converted(&shape, json!({"name": "x", "n": "3"})), json!({"name": "x", "n": 3}));
}

#[test]
fn overlapping_declarations_must_all_hold() {
    let count_pattern = || KeyMatcher::Pattern(TextPattern::new("^c").expect("pattern"));
    let agree = dictionary(vec![("count", number())], vec![(count_pattern(), numberlike())], PatternScope::AllKeys);
    assert_eq!(converted(&agree, json!({"count": "4"})), json!({"count": 4}));
    let conflict = dictionary(vec![("count", number())], vec![(count_pattern(), boolean())], PatternScope::AllKeys);
    let reason = only_reason(&conflict, json!({"count": "1"}));
    let FailureReason::ConflictingDeclarations { readings } = reason else {
        panic!("expected conflicting declarations, got {reason:?}");
    };
    assert_eq!(readings.len(), 2);
    let bounded = dictionary(
        vec![("count", number())],
        vec![(
            count_pattern(),
            number_with(NumericBounds {
                minimum: Some(10.0),
                ..NumericBounds::default()
            }),
        )],
        PatternScope::AllKeys,
    );
    assert!(bind(&bounded, &json!({"count": "4"})).is_err());
    assert_eq!(converted(&bounded, json!({"count": "12"})), json!({"count": 12}));
}

#[test]
fn overlapping_pattern_order_never_changes_the_result() {
    let starts_c = || KeyMatcher::Pattern(TextPattern::new("^c").expect("pattern"));
    let ends_t = || KeyMatcher::Suffix("t".into());
    for value in [json!({"count": "1"}), json!({"count": "4"}), json!({"cat": "yes"})] {
        let forward = dictionary(
            vec![],
            vec![(starts_c(), number()), (ends_t(), boolean())],
            PatternScope::AllKeys,
        );
        let backward = dictionary(
            vec![],
            vec![(ends_t(), boolean()), (starts_c(), number())],
            PatternScope::AllKeys,
        );
        assert_eq!(normalize(outcome(&forward, &value)), normalize(outcome(&backward, &value)), "{value}");
    }
}

#[test]
fn additional_keys_follow_their_rule() {
    let Shape::Object(mut closed) = object(vec![("a", number())]) else {
        unreachable!("object() builds an object")
    };
    closed.additional = Additional::Forbidden;
    let closed = Shape::Object(closed);
    let failure = error(&closed, json!({"a": 1, "b": 2})).failures.remove(0);
    assert_eq!((failure.path_text(), failure.reason), ("b".to_string(), FailureReason::UnexpectedProperty));
    let Shape::Object(mut typed) = object(vec![]) else {
        unreachable!("object() builds an object")
    };
    typed.additional = Additional::Shape(Box::new(number()));
    assert_eq!(converted(&Shape::Object(typed), json!({"z": "2"})), json!({"z": 2}));
}

#[test]
fn any_and_opaque_objects_keep_contents_including_null() {
    assert_kept(&Shape::Any, json!(null));
    assert_kept(&Shape::array(Shape::Any), json!([null, "1", {"a": null}]));
    assert_kept(&Shape::Object(ObjectShape::opaque()), json!({"a": "1", "b": null}));
    assert!(!Shape::Any.admits_null_explicitly());
    assert!(union(vec![number(), Shape::Null]).admits_null_explicitly());
    assert!(union(vec![number(), Shape::Null]).assert(Assertion::new("x", |_| true)).admits_null_explicitly());
    assert!(!Shape::array(Shape::Null).admits_null_explicitly());
}

#[test]
fn lists_of_unions_bind_element_by_element() {
    assert_eq!(
        converted(&Shape::array(union(vec![boolean(), number()])), json!(["2", "yes"])),
        json!([2, true])
    );
    let text_or_number = Shape::array(union(vec![text(), number()]));
    assert_kept(&text_or_number, json!(["a", 1]));
    assert_eq!(converted(&text_or_number, json!([true])), json!(["true"]));
    assert!(check_exact(&text_or_number, &json!([true])).is_err());
}

// ── unions ──────────────────────────────────────────────────────────────────

fn object_options() -> Vec<Shape> {
    vec![object(vec![("id", number())]), object(vec![("id", boolean())])]
}

#[test]
fn the_union_table_holds_in_every_option_order() {
    assert_eq!(union_outcome(vec![text(), number()], json!("4")), Outcome::Kept);
    assert_eq!(
        union_outcome(vec![text(), Shape::array(text())], json!(2026)),
        Outcome::Converted(json!("2026"))
    );
    let at_least_five = number_with(NumericBounds {
        minimum: Some(5.0),
        ..NumericBounds::default()
    });
    assert_eq!(
        union_outcome(vec![at_least_five, boolean()], json!("1")),
        Outcome::Converted(json!(true))
    );
    assert_eq!(union_outcome(vec![number(), boolean()], json!("1")), Outcome::Converted(json!(1)));
    assert_eq!(union_outcome(vec![number(), boolean()], json!("0")), Outcome::Converted(json!(0)));
    assert_eq!(union_outcome(object_options(), json!({"id": "1"})), Outcome::Converted(json!({"id": 1})));
    let config: Value = serde_json::from_str(r#"{"port": 8080, "debug": true}"#).expect("json");
    assert_eq!(
        union_outcome(vec![json_content(), yaml_content()], config),
        Outcome::Converted(json!(r#"{"port":8080,"debug":true}"#))
    );
    assert_eq!(union_outcome(vec![json_content(), yaml_content()], json!("port: 8080")), Outcome::Kept);
    assert_eq!(union_outcome(vec![number(), integer()], json!("4")), Outcome::Converted(json!(4)));
    assert_eq!(union_outcome(vec![boolean(), json_content()], json!("true")), Outcome::Kept);
}

#[test]
fn fields_pointing_to_different_options_are_ambiguous() {
    let options = vec![
        object(vec![("a", number()), ("b", boolean())]),
        object(vec![("a", boolean()), ("b", number())]),
    ];
    let outcome = union_outcome(options.clone(), json!({"a": "1", "b": "1"}));
    let mut expected = vec![json!({"a": 1, "b": true}), json!({"a": true, "b": 1})];
    expected.sort_by_key(|reading| reading.to_string());
    assert_eq!(outcome, Outcome::Failed(vec![FailureReason::AmbiguousUnion { readings: expected }]));
    let rendered = error(&union(options), json!({"a": "1", "b": "1"})).to_string();
    assert!(rendered.contains(r#"{"a":1,"b":true}"#) && rendered.contains(r#"{"a":true,"b":1}"#), "{rendered}");
}

#[test]
fn a_difference_without_a_tie_breaker_is_ambiguous() {
    let options = vec![
        tuple(vec![(number(), false), (text(), false)], None),
        tuple(vec![(boolean(), false), (number(), false)], None),
    ];
    let outcome = union_outcome(options, json!(["1", 2]));
    assert!(
        matches!(&outcome, Outcome::Failed(reasons) if matches!(reasons.as_slice(), [FailureReason::AmbiguousUnion { .. }])),
        "{outcome:?}"
    );
}

#[test]
fn tie_breakers_apply_field_by_field_when_fields_agree() {
    let options = vec![
        object(vec![("a", number()), ("b", number())]),
        object(vec![("a", boolean()), ("b", boolean())]),
    ];
    assert_eq!(
        union_outcome(options, json!({"a": "1", "b": "0"})),
        Outcome::Converted(json!({"a": 1, "b": 0}))
    );
}

#[test]
fn no_reachable_option_lists_every_option() {
    let error = error(&union(vec![number(), boolean()]), json!("pear"));
    let FailureReason::NoOptionAccepts { options } = &error.failures[0].reason else {
        panic!("{error}");
    };
    let expected: Vec<&str> = options.iter().map(|option| option.expected.as_str()).collect();
    assert_eq!(expected, ["number", "boolean"]);
    assert!(options.iter().all(|option| !option.failures.is_empty()));
}

// ── immutability, exact checks, deferral, depth ─────────────────────────────

#[test]
fn a_failed_bind_leaves_the_input_unchanged() {
    let value = json!({"meta": {"inner": {"count": "3", "bad": "x"}}});
    let snapshot = value.clone();
    let shape = object(vec![("meta", object(vec![("inner", object(vec![("count", number()), ("bad", number())]))]))]);
    assert!(bind(&shape, &value).is_err());
    assert_eq!(value, snapshot);
}

#[test]
fn exact_checks_never_convert() {
    let reason = error_exact(&number(), json!("4"));
    assert_eq!(reason, FailureReason::ConversionNotAllowed);
    assert!(!accepts_exactly(&number(), &json!("4")));
    assert!(check_exact(&number(), &json!(4)).is_ok());
    assert!(accepts_exactly(&numberlike(), &json!("4")));
    let FailureReason::NoOptionAccepts { .. } = error_exact(&union(vec![number(), boolean()]), json!("1")) else {
        panic!("a union exact check names every option");
    };
}

fn error_exact(shape: &Shape, value: Value) -> FailureReason {
    let mut error = check_exact(shape, &value).expect_err("exact check fails");
    error.failures.remove(0).reason
}

#[test]
fn exactness_agrees_across_the_three_entry_points() {
    let shapes = [
        number(),
        integer(),
        numberlike(),
        boolean(),
        boolish(),
        text(),
        text_length(3, 3),
        members(&["1", "2"]),
        json_content(),
        yaml_content(),
        Shape::Literal(json!(5)),
        Shape::array(number()),
        person(),
        union(vec![number(), boolean()]),
        union(object_options()),
        object(vec![("id", number())]),
        Shape::Null,
        Shape::Any,
    ];
    let values = [
        json!(4),
        json!("4"),
        json!("1"),
        json!(true),
        json!("yes"),
        json!(null),
        json!("abc"),
        json!(["Ada", 36]),
        json!(["Ada", "36"]),
        json!({"id": 1}),
        json!({"id": "1"}),
        json!(2.5),
        json!("{}"),
    ];
    for shape in &shapes {
        for value in &values {
            let exact = accepts_exactly(shape, value);
            assert_eq!(exact, check_exact(shape, value).is_ok(), "{shape} given {value}");
            let kept = matches!(bind(shape, value), Ok(Bound::Kept(_)));
            assert_eq!(exact, kept, "{shape} given {value}");
        }
    }
}

#[test]
fn deferred_values_are_kept_at_their_path() {
    let shape = object(vec![("when", number()), ("count", number())]);
    let pending = |path: &[PathSegment], value: &Value| {
        path == [PathSegment::Key("when".into())] && value.as_str().is_some_and(|text| text.starts_with("$("))
    };
    let options = BindOptions {
        defer: Some(&pending),
        ..BindOptions::default()
    };
    let value = json!({"when": "$(date +%s)", "count": "2"});
    let bound = bind_with(&shape, &value, &options).expect("deferred value binds");
    assert_eq!(bound.value(), &json!({"when": "$(date +%s)", "count": 2}));
    assert!(bind(&shape, &value).is_err());
}

#[test]
fn nesting_beyond_the_depth_bound_is_a_resource_error() {
    let mut shape = number();
    let mut value = json!("1");
    for _ in 0..8 {
        shape = Shape::array(shape);
        value = json!([value]);
    }
    let options = BindOptions {
        max_depth: 4,
        ..BindOptions::default()
    };
    let error = bind_with(&shape, &value, &options).expect_err("too deep");
    assert_eq!(error.failures[0].reason, FailureReason::ResourceLimit { max_depth: 4 });
    assert_eq!(bind(&shape, &value).map(Bound::into_value).ok(), Some({
        let mut expected = json!(1);
        for _ in 0..8 {
            expected = json!([expected]);
        }
        expected
    }));
}

// ── rendering ───────────────────────────────────────────────────────────────

#[test]
fn shapes_describe_themselves_with_constraints() {
    let pr_count = number_with(NumericBounds {
        integer: Some(IntegerRule::WholeBelow2Pow64),
        minimum: Some(1.0),
        ..NumericBounds::default()
    });
    assert_eq!(pr_count.describe(), "number(integer; min(1))");
    assert_eq!(text_length(3, 3).describe(), "string(min(3); max(3))");
    assert_eq!(person().describe(), "tuple([string, number, boolean?])");
    assert_eq!(union(object_options()).describe(), "{ id: number } | { id: boolean }");
    assert_eq!(Shape::array(union(vec![boolean(), number()])).describe(), "(boolean | number)[]");
    assert_eq!(members(&["1", "2"]).describe(), r#"enum("1", "2")"#);
    assert_eq!(numberlike().describe(), "numberlike");
    assert_eq!(Shape::Object(ObjectShape::opaque()).describe(), "object");
}

#[test]
fn errors_render_path_expectation_received_value_and_reason() {
    let rendered = error(&Shape::array(number()), json!(["1", "2", "x"])).to_string();
    assert_eq!(rendered, r#"[2]: expected number; received text "x"; conversion failed: not a number"#);
    let pr_count = number_with(NumericBounds {
        integer: Some(IntegerRule::WholeBelow2Pow64),
        minimum: Some(1.0),
        ..NumericBounds::default()
    });
    let rendered = error(&pr_count, json!("0")).to_string();
    assert_eq!(
        rendered,
        r#"expected number(integer; min(1)); received text "0", converted to 0; fails min(1)"#
    );
    let failure = &error(&number(), json!("9007199254740993")).failures[0];
    assert_eq!(failure.received_kind(), Some(ValueKind::Text));
    assert!(failure.to_string().ends_with("cannot be represented exactly"), "{failure}");
    let many = error(&object(vec![("a", number()), ("b", boolean())]), json!({"a": null, "b": 2})).to_string();
    assert!(many.starts_with("2 values failed:"), "{many}");
    assert!(many.contains("a: expected number; received null; no conversion from that type"), "{many}");
}

#[test]
fn paths_render_keys_indexes_and_quoted_keys() {
    let path = [
        PathSegment::Key("meta".into()),
        PathSegment::Index(2),
        PathSegment::Key("a b".into()),
    ];
    assert_eq!(render_path("", &path), r#"meta[2]["a b"]"#);
    assert_eq!(render_path("values", &[PathSegment::Index(0)]), "values[0]");
}
