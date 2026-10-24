//! `validate` checks: each rule against matching and other JSON kinds,
//! absent/empty/null, exact numbers, uniqueness, `enum`/`const`, recursion,
//! and the typed parameter helpers.

use schematic_runtime::validate::{
    self, Keyword, Literal, PathSegment, Pattern, Plan, Property, Violations, check_array_len, check_number, check_str,
    check_value,
};
use serde_json::json;

const BODY: &[PathSegment] = &[PathSegment::Name("body")];
const NONE: [Keyword; 0] = [];
const NO_REPORT: [(String, Keyword); 0] = [];

fn report(value: serde_json::Value, plan: &Plan) -> Vec<(String, Keyword)> {
    let mut violations = Violations::new();
    check_value(&value, plan, BODY, &mut violations);
    violations.iter().map(|violation| (violation.path_text(), violation.keyword())).collect()
}

fn keywords(value: serde_json::Value, plan: &Plan) -> Vec<Keyword> {
    report(value, plan).into_iter().map(|(_, keyword)| keyword).collect()
}

#[test]
fn string_length_counts_code_points_not_bytes_or_utf16_units() {
    const THREE: Plan = Plan::new().min_length(3).max_length(3);
    // "😀" is 4 UTF-8 bytes and 2 UTF-16 units but one code point; "é" here
    // is the single precomposed code point.
    for accepted in ["abc", "😀😀😀", "𝄞a😀", "été"] {
        assert_eq!(keywords(json!(accepted), &THREE), NONE, "{accepted:?}");
    }
    assert_eq!(keywords(json!("😀😀"), &THREE), [Keyword::MinLength]);
    assert_eq!(keywords(json!("😀😀😀😀"), &THREE), [Keyword::MaxLength]);
    // A combining accent is its own code point: "e" + U+0301 is two.
    assert_eq!(keywords(json!("ae\u{301}"), &THREE), NONE);
}

#[test]
fn an_empty_string_or_array_is_checked_and_an_absent_property_is_not() {
    static PLAN: Plan = Plan::new().properties(&[
        Property::new("name", &Plan::new().min_length(1)),
        Property::new("tags", &Plan::new().min_items(1)),
    ]);
    assert_eq!(report(json!({}), &PLAN), NO_REPORT);
    assert_eq!(
        report(json!({"name": "", "tags": []}), &PLAN),
        [("body.name".to_owned(), Keyword::MinLength), ("body.tags".to_owned(), Keyword::MinItems)]
    );
    assert_eq!(report(json!({"name": "a", "tags": ["x"]}), &PLAN), NO_REPORT);
}

#[test]
fn null_is_data_for_an_enum_or_const_with_or_without_a_nullable_type() {
    // `{"type": ["string", "null"], "enum": ["ab"]}`: the type admits null,
    // the enum does not.
    const NULLABLE_ENUM: Plan = Plan::new().min_length(2).allowed_values(&[Literal::String("ab")]).nullable();
    const NULLABLE_CONST: Plan = Plan::new().constant(&Literal::String("ab")).nullable();
    const NULLABLE_ENUM_WITH_NULL: Plan = Plan::new().allowed_values(&[Literal::String("ab"), Literal::Null]).nullable();
    const NULLABLE_LENGTH_ONLY: Plan = Plan::new().min_length(2).nullable();
    const WITH_NULL: Plan = Plan::new().allowed_values(&[Literal::String("ab"), Literal::Null]);
    const WITHOUT_NULL: Plan = Plan::new().allowed_values(&[Literal::String("ab")]);
    const LENGTH_ONLY: Plan = Plan::new().min_length(2);

    assert_eq!(keywords(json!(null), &NULLABLE_ENUM), [Keyword::Enum], "nullability does not add null to the enum");
    assert_eq!(keywords(json!(null), &NULLABLE_CONST), [Keyword::Const]);
    assert_eq!(keywords(json!("a"), &NULLABLE_ENUM), [Keyword::MinLength, Keyword::Enum], "a non-null value is checked");
    assert_eq!(keywords(json!("ab"), &NULLABLE_CONST), NONE);
    assert_eq!(keywords(json!(null), &NULLABLE_ENUM_WITH_NULL), NONE);
    assert_eq!(keywords(json!(null), &NULLABLE_LENGTH_ONLY), NONE);
    assert_eq!(keywords(json!(null), &WITH_NULL), NONE);
    assert_eq!(keywords(json!(null), &WITHOUT_NULL), [Keyword::Enum]);
    assert_eq!(keywords(json!(null), &LENGTH_ONLY), NONE, "null is not a string");
    assert_eq!(keywords(json!(null), &Plan::new().constant(&Literal::Null)), NONE);
}

#[test]
fn a_null_alternative_admits_null_past_its_constrained_member() {
    // `{"anyOf": [{"type": "string", "minLength": 2, "enum": ["ab"]}, {"type": "null"}]}`,
    // lowered as the generator lowers a Schemars `Option`.
    static MEMBER: Plan = Plan::new().min_length(2).allowed_values(&[Literal::String("ab")]);
    static OPTION: Plan = Plan::reference(&MEMBER).nullable();
    static CONST_MEMBER: Plan = Plan::new().constant(&Literal::String("ab"));
    static CONST_OPTION: Plan = Plan::reference(&CONST_MEMBER).nullable();

    assert_eq!(keywords(json!(null), &OPTION), NONE);
    assert_eq!(keywords(json!(null), &CONST_OPTION), NONE);
    assert_eq!(keywords(json!("a"), &OPTION), [Keyword::MinLength, Keyword::Enum]);
    assert_eq!(keywords(json!("x"), &CONST_OPTION), [Keyword::Const]);
    // Without the alternative, the same member refuses null.
    assert_eq!(keywords(json!(null), &Plan::reference(&MEMBER)), [Keyword::Enum]);
}

#[test]
fn a_rule_applies_only_to_values_of_its_own_json_kind() {
    const EVERY_KIND: Plan = Plan::new().min_length(5).minimum("10").min_items(3);
    for other_kind in [json!(true), json!({}), json!(null)] {
        assert_eq!(keywords(other_kind.clone(), &EVERY_KIND), NONE, "{other_kind}");
    }
    assert_eq!(keywords(json!("abc"), &EVERY_KIND), [Keyword::MinLength]);
    assert_eq!(keywords(json!(3), &EVERY_KIND), [Keyword::Minimum]);
    assert_eq!(keywords(json!([1]), &EVERY_KIND), [Keyword::MinItems]);
    assert_eq!(keywords(json!("12"), &Plan::new().maximum("1")), NONE, "a numeric string is a string");
}

#[test]
fn inclusive_and_exclusive_endpoints() {
    const INCLUSIVE: Plan = Plan::new().minimum("1").maximum("10");
    const EXCLUSIVE: Plan = Plan::new().exclusive_minimum("1").exclusive_maximum("10");
    let cases = [
        (json!(0), vec![Keyword::Minimum], vec![Keyword::ExclusiveMinimum]),
        (json!(1), vec![], vec![Keyword::ExclusiveMinimum]),
        (json!(1.0), vec![], vec![Keyword::ExclusiveMinimum]),
        (json!(1.000001), vec![], vec![]),
        (json!(10), vec![], vec![Keyword::ExclusiveMaximum]),
        (json!(10.5), vec![Keyword::Maximum], vec![Keyword::ExclusiveMaximum]),
    ];
    for (value, inclusive, exclusive) in cases {
        assert_eq!(keywords(value.clone(), &INCLUSIVE), inclusive, "inclusive {value}");
        assert_eq!(keywords(value.clone(), &EXCLUSIVE), exclusive, "exclusive {value}");
    }
}

#[test]
fn bounds_beyond_u64_and_decimal_divisors_are_exact() {
    const HUGE: Plan = Plan::new().maximum("18446744073709551616");
    assert_eq!(keywords(json!(u64::MAX), &HUGE), NONE);
    const BELOW_U64_MAX: Plan = Plan::new().exclusive_maximum("18446744073709551615");
    assert_eq!(keywords(json!(u64::MAX), &BELOW_U64_MAX), [Keyword::ExclusiveMaximum], "no f64 rounding");

    const TENTHS: Plan = Plan::new().multiple_of("0.1");
    assert_eq!(keywords(json!(0.3), &TENTHS), NONE, "0.3 is a multiple although 0.3 / 0.1 is not 3.0 in f64");
    assert_eq!(keywords(json!(0.35), &TENTHS), [Keyword::MultipleOf]);
    assert_eq!(keywords(json!(-2), &TENTHS), NONE);
}

#[test]
fn reasons_name_the_declared_limit_in_canonical_form() {
    let mut violations = Violations::new();
    check_value(&json!(500), &Plan::new().maximum("100.0").multiple_of("7"), BODY, &mut violations);
    let reasons: Vec<_> = violations.iter().map(|violation| violation.reason().to_owned()).collect();
    assert_eq!(reasons, ["must be at most 100", "must be a multiple of 7"]);
}

#[test]
fn unique_items_compares_objects_regardless_of_member_order_and_numbers_exactly() {
    const UNIQUE: Plan = Plan::new().unique_items();
    let reordered: serde_json::Value = serde_json::from_str(r#"[{"a": 1, "b": [2, 3]}, {"b": [2, 3], "a": 1}]"#).unwrap();
    assert_eq!(keywords(reordered, &UNIQUE), [Keyword::UniqueItems]);
    assert_eq!(keywords(json!([1, 1.0]), &UNIQUE), [Keyword::UniqueItems], "1 equals 1.0");
    assert_eq!(keywords(json!([{"a": 1}, {"a": 1, "b": null}]), &UNIQUE), NONE, "a null member is a member");
    assert_eq!(keywords(json!([[1, 2], [2, 1]]), &UNIQUE), NONE, "array order matters");
    assert_eq!(keywords(json!(["1", 1]), &UNIQUE), NONE, "kinds differ");
    assert_eq!(keywords(json!([]), &UNIQUE), NONE);

    let mut violations = Violations::new();
    check_value(&json!(["x", "y", "z", "y"]), &UNIQUE, BODY, &mut violations);
    assert_eq!(
        violations.iter().next().unwrap().reason(),
        "must not repeat an item; items 1 and 3 are equal",
        "positions are reported, values are not"
    );
}

#[test]
fn enum_and_const_compare_whole_json_values() {
    static ALLOWED: [Literal; 3] = [
        Literal::Object(&[("kind", Literal::String("tag")), ("weight", Literal::Number("1"))]),
        Literal::Array(&[Literal::Bool(true), Literal::Null]),
        Literal::Number("0.5"),
    ];
    const ONE_OF: Plan = Plan::new().allowed_values(&ALLOWED);
    for accepted in [json!({"weight": 1.0, "kind": "tag"}), json!([true, null]), json!(0.50)] {
        assert_eq!(keywords(accepted.clone(), &ONE_OF), NONE, "{accepted}");
    }
    for refused in [json!({"kind": "tag"}), json!({"kind": "tag", "weight": 1, "x": 0}), json!([null, true]), json!("0.5")] {
        assert_eq!(keywords(refused.clone(), &ONE_OF), [Keyword::Enum], "{refused}");
    }

    const CONSTANT: Plan = Plan::new().constant(&Literal::Object(&[]));
    assert_eq!(keywords(json!({}), &CONSTANT), NONE);
    assert_eq!(keywords(json!({"a": 1}), &CONSTANT), [Keyword::Const]);
    assert_eq!(keywords(json!([]), &CONSTANT), [Keyword::Const]);
}

#[test]
fn every_violation_is_reported_in_key_index_and_keyword_order() {
    static ITEM: Plan = Plan::new().properties(&[Property::new("displayName", &Plan::new().max_length(3).pattern(&UPPER))]);
    static UPPER: Pattern = Pattern::new("^[A-Z]");
    // Declared out of key order, to show that keys, not declarations, order a body.
    static PLAN: Plan = Plan::new().properties(&[
        Property::new("zeta", &Plan::new().minimum("0")),
        Property::new("items", &Plan::new().max_items(2).items(&ITEM)),
        Property::new("alpha", &Plan::new().min_length(2).allowed_values(&[Literal::String("ok")])),
    ]);
    let body = json!({
        "zeta": -1,
        "items": [{"displayName": "Fine"}, {"displayName": "ok"}, {"displayName": "lowercase"}],
        "alpha": "x",
    });
    let expected = [
        ("body.alpha", Keyword::MinLength),
        ("body.alpha", Keyword::Enum),
        ("body.items", Keyword::MaxItems),
        ("body.items[0].displayName", Keyword::MaxLength),
        ("body.items[1].displayName", Keyword::Pattern),
        ("body.items[2].displayName", Keyword::MaxLength),
        ("body.items[2].displayName", Keyword::Pattern),
        ("body.zeta", Keyword::Minimum),
    ];
    let expected: Vec<_> = expected.iter().map(|(path, keyword)| ((*path).to_owned(), *keyword)).collect();
    assert_eq!(report(body.clone(), &PLAN), expected);
    assert_eq!(report(body, &PLAN), expected, "the same input gives the same report");
}

#[test]
fn map_values_are_checked_under_a_redacted_key_and_declared_properties_by_name() {
    static PLAN: Plan = Plan::new()
        .properties(&[Property::new("id", &Plan::new().max_length(2))])
        .values(&Plan::new().max_length(4));
    let body = json!({"id": "abc", "Authorization: Bearer sk-live-1": "longer than four", "short": "ok"});
    assert_eq!(
        report(body, &PLAN),
        [("body[[redacted]]".to_owned(), Keyword::MaxLength), ("body.id".to_owned(), Keyword::MaxLength)]
    );
    assert_eq!(report(json!({"undeclared": "too long"}), &Plan::new().properties(&[])), NO_REPORT, "no plan, no checks");
}

#[test]
fn recursion_follows_the_finite_value_through_named_plans() {
    static TREE: Plan = Plan::new().properties(&[
        Property::new("children", &Plan::new().items(&Plan::reference(&TREE)).max_items(2)),
        Property::new("name", &Plan::new().min_length(1)),
    ]);
    let tree = json!({
        "name": "root",
        "children": [
            {"name": "a", "children": [{"name": "", "children": []}]},
            {"name": "b", "children": [{"name": "c"}, {"name": "d"}, {"name": "e"}]},
        ],
    });
    assert_eq!(
        report(tree, &TREE),
        [
            ("body.children[0].children[0].name".to_owned(), Keyword::MinLength),
            ("body.children[1].children".to_owned(), Keyword::MaxItems),
        ]
    );
}

#[test]
fn a_reference_cycle_that_never_descends_terminates() {
    static LOOP_A: Plan = Plan::reference(&LOOP_B).min_length(1);
    static LOOP_B: Plan = Plan::reference(&LOOP_A);
    let found = keywords(json!(""), &LOOP_A);
    assert!(!found.is_empty() && found.iter().all(|keyword| *keyword == Keyword::MinLength));
}

#[test]
fn a_nullable_reference_admits_null_and_checks_the_target_otherwise() {
    static TARGET: Plan = Plan::new().min_length(2);
    static OPTIONAL: Plan = Plan::reference(&TARGET).nullable();
    assert_eq!(keywords(json!(null), &OPTIONAL), NONE);
    assert_eq!(keywords(json!("a"), &OPTIONAL), [Keyword::MinLength]);
}

#[test]
fn patterns_search_unless_anchored_and_handle_unicode_input() {
    static CONTAINS_DIGIT: Pattern = Pattern::new("[0-9]");
    static WHOLE_WORD: Pattern = Pattern::new("^[a-z]+$");
    static ACCENT: Pattern = Pattern::new("é");
    static NOT_LOWER: Pattern = Pattern::new("^[^a-z]+$");
    assert_eq!(CONTAINS_DIGIT.is_match("abc1def"), Some(true), "unanchored search");
    assert_eq!(WHOLE_WORD.is_match("abc1def"), Some(false));
    assert_eq!(WHOLE_WORD.is_match("héllo"), Some(false));
    assert_eq!(ACCENT.is_match("café"), Some(true));
    assert_eq!(NOT_LOWER.is_match("ÉÀ😀"), Some(true), "negated classes accept non-ASCII text");

    static PLAN: Plan = Plan::new().pattern(&WHOLE_WORD);
    assert_eq!(keywords(json!("héllo"), &PLAN), [Keyword::Pattern]);
    static BROKEN: Pattern = Pattern::new("(unclosed");
    assert_eq!(keywords(json!("anything"), &Plan::new().pattern(&BROKEN)), NONE, "unenforced");
}

#[test]
fn check_str_applies_string_enum_and_const_rules() {
    const PATH: &[PathSegment] = &[PathSegment::Name("path"), PathSegment::Name("owner")];
    const PLAN: Plan = Plan::new().max_length(5).allowed_values(&[Literal::String("alice"), Literal::String("bob")]);
    let mut violations = Violations::new();
    check_str("bob", &PLAN, PATH, &mut violations);
    assert!(violations.is_empty());
    check_str("mallory", &PLAN, PATH, &mut violations);
    let found: Vec<_> = violations.iter().map(|v| (v.path_text(), v.keyword())).collect();
    assert_eq!(found, [("path.owner".to_owned(), Keyword::MaxLength), ("path.owner".to_owned(), Keyword::Enum)]);

    let mut violations = Violations::new();
    check_str("", &Plan::new().min_length(1), PATH, &mut violations);
    assert_eq!(violations.len(), 1, "an empty string is present and checked");
}

#[test]
fn check_number_is_exact_for_every_primitive() {
    const PATH: &[PathSegment] = &[PathSegment::Name("query"), PathSegment::Name("n")];
    let count = |check: &dyn Fn(&mut Violations)| {
        let mut violations = Violations::new();
        check(&mut violations);
        violations.iter().map(|v| v.keyword()).collect::<Vec<_>>()
    };
    const TENTHS: Plan = Plan::new().multiple_of("0.1");
    assert_eq!(count(&|out| check_number(0.3_f64, &TENTHS, PATH, out)), NONE);
    assert_eq!(count(&|out| check_number(0.1_f32, &TENTHS, PATH, out)), NONE, "an f32 reads as its own shortest text");
    assert_eq!(count(&|out| check_number(0.35_f64, &TENTHS, PATH, out)), [Keyword::MultipleOf]);

    const ABOVE_U64: Plan = Plan::new().minimum("18446744073709551616");
    assert_eq!(count(&|out| check_number(u64::MAX, &ABOVE_U64, PATH, out)), [Keyword::Minimum]);
    assert_eq!(count(&|out| check_number(u128::from(u64::MAX) + 1, &ABOVE_U64, PATH, out)), NONE);
    assert_eq!(count(&|out| check_number(i8::MIN, &Plan::new().minimum("-128"), PATH, out)), NONE);

    const ALL: Plan = Plan::new().minimum("0").maximum("1").multiple_of("1");
    assert_eq!(
        count(&|out| check_number(f64::NAN, &ALL, PATH, out)),
        [Keyword::Minimum, Keyword::Maximum, Keyword::MultipleOf],
        "a NaN satisfies no numeric rule"
    );
    assert_eq!(count(&|out| check_number(f64::INFINITY, &Plan::new().maximum("1"), PATH, out)), [Keyword::Maximum]);

    const ONE_OF: Plan = Plan::new().allowed_values(&[Literal::Number("1"), Literal::Number("2.5")]);
    assert_eq!(count(&|out| check_number(2.5_f64, &ONE_OF, PATH, out)), NONE);
    assert_eq!(count(&|out| check_number(1_i32, &ONE_OF, PATH, out)), NONE);
    assert_eq!(count(&|out| check_number(3_i32, &ONE_OF, PATH, out)), [Keyword::Enum]);
}

#[test]
fn check_array_len_checks_counts_before_joining() {
    const PATH: &[PathSegment] = &[PathSegment::Name("query"), PathSegment::Name("ids")];
    const PLAN: Plan = Plan::new().min_items(1).max_items(3);
    let check = |len| {
        let mut violations = Violations::new();
        check_array_len(len, &PLAN, PATH, &mut violations);
        violations.iter().map(|v| v.keyword()).collect::<Vec<_>>()
    };
    assert_eq!(check(0), [Keyword::MinItems], "an empty array is present and checked");
    assert_eq!(check(1), NONE);
    assert_eq!(check(3), NONE);
    assert_eq!(check(4), [Keyword::MaxItems]);
}

#[test]
fn limit_text_that_does_not_parse_leaves_its_rule_unenforced() {
    assert_eq!(keywords(json!(5), &Plan::new().maximum("not a number").minimum("10")), [Keyword::Minimum]);
    assert_eq!(keywords(json!(5), &Plan::new().multiple_of("0")), NONE, "a nonpositive divisor");
    assert_eq!(keywords(json!(5), &Plan::new().allowed_values(&[Literal::Number("5.")])), [Keyword::Enum]);
}

#[test]
fn the_module_functions_are_reachable_by_path() {
    let mut violations = Violations::new();
    validate::check_str("x", &Plan::new(), &[], &mut violations);
    assert!(violations.into_result().is_ok());
}
