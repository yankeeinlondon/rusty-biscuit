//! `validate::Violations`: aggregate text, paths, keywords, the
//! `InvalidArgument` conversion, and that no output carries a submitted
//! value.

use std::error::Error as _;

use schematic_runtime::validate::{Keyword, Literal, PathSegment, Pattern, Plan, Property, Violations, check_str, check_value};
use schematic_runtime::{InvalidArgument, TransportErrorKind};
use serde_json::json;

const BODY: &[PathSegment] = &[PathSegment::Name("body")];

#[test]
fn no_rendering_of_a_report_contains_submitted_values_or_declared_sets() {
    static TOKEN_SHAPE: Pattern = Pattern::new("^tok_[a-z]{4}$");
    static PLAN: Plan = Plan::new().properties(&[
        Property::new("labels", &Plan::new().values(&Plan::new().max_length(3))),
        Property::new("mode", &Plan::new().allowed_values(&[Literal::String("allowed-mode-alpha"), Literal::String("allowed-mode-beta")])),
        Property::new("password", &Plan::new().min_length(64).pattern(&TOKEN_SHAPE)),
        Property::new("pinned", &Plan::new().constant(&Literal::String("constant-value-gamma"))),
        Property::new("secrets", &Plan::new().unique_items()),
        Property::new("token", &Plan::new().maximum("10")),
    ]);
    let body = json!({
        "labels": {"key-hunter2-secret": "value-hunter2-secret"},
        "mode": "mode-hunter2-secret",
        "password": "password-hunter2-secret",
        "pinned": "pinned-hunter2-secret",
        "secrets": ["dup-hunter2-secret", "dup-hunter2-secret"],
        "token": 987654321,
    });
    let mut violations = Violations::new();
    check_value(&body, &PLAN, BODY, &mut violations);
    assert_eq!(violations.len(), 7, "{violations}");

    let argument = InvalidArgument::from(violations.clone());
    let mut renderings = vec![
        violations.to_string(),
        format!("{violations:?}"),
        argument.to_string(),
        format!("{argument:?}"),
        TransportErrorKind::InvalidArgument(argument.clone()).to_string(),
    ];
    for violation in &violations {
        renderings.push(violation.to_string());
        renderings.push(format!("{violation:?}"));
        renderings.push(violation.path_text());
    }
    for rendering in &renderings {
        for forbidden in ["hunter2", "987654321", "allowed-mode", "constant-value", "tok_", "[a-z]{4}"] {
            assert!(!rendering.contains(forbidden), "{forbidden:?} leaked into {rendering}");
        }
    }
    assert!(violations.source().is_none());
    assert!(argument.source().is_none());
}

#[test]
fn one_violation_reads_as_a_sentence_and_several_are_counted() {
    const NAME: Plan = Plan::new().min_length(2).max_length(1);
    let path = [PathSegment::Name("query"), PathSegment::Name("name")];

    let mut one = Violations::new();
    check_str("", &Plan::new().min_length(2), &path, &mut one);
    assert_eq!(one.to_string(), "query.name violates minLength: must be at least 2 characters long");
    assert_eq!(
        InvalidArgument::from(one).to_string(),
        "invalid argument: query.name violates minLength: must be at least 2 characters long"
    );

    let mut two = Violations::new();
    check_str("", &NAME, &path, &mut two);
    check_str("abc", &NAME, &[PathSegment::Name("header"), PathSegment::Name("name")], &mut two);
    assert_eq!(
        two.to_string(),
        "2 constraint violations: query.name violates minLength: must be at least 2 characters long; \
         header.name violates maxLength: must be at most 1 character long"
    );

    assert_eq!(Violations::new().to_string(), "no constraint violations");
    assert!(Violations::new().into_result().is_ok());
    assert_eq!(two.clone().into_result().unwrap_err(), two);
}

#[test]
fn parameters_report_in_the_order_they_are_checked() {
    // Generated code checks parameters in declaration order; the report keeps it.
    const RULE: Plan = Plan::new().min_length(1);
    let mut violations = Violations::new();
    for name in ["zulu", "alpha", "mike"] {
        check_str("", &RULE, &[PathSegment::Name("query"), PathSegment::Name(name)], &mut violations);
    }
    let paths: Vec<_> = violations.iter().map(|violation| violation.path_text()).collect();
    assert_eq!(paths, ["query.zulu", "query.alpha", "query.mike"]);
}

#[test]
fn paths_render_names_indices_and_redacted_keys() {
    static PLAN: Plan = Plan::new().properties(&[Property::new(
        "items",
        &Plan::new().items(&Plan::new().values(&Plan::new().properties(&[Property::new("displayName", &Plan::new().min_length(1))]))),
    )]);
    let mut violations = Violations::new();
    check_value(&json!({"items": [{}, {}, {"private": {"displayName": ""}}]}), &PLAN, BODY, &mut violations);
    let violation = violations.iter().next().expect("one violation");
    assert_eq!(violation.path_text(), "body.items[2][[redacted]].displayName");
    assert_eq!(
        violation.path(),
        [
            PathSegment::Name("body"),
            PathSegment::Name("items"),
            PathSegment::Index(2),
            PathSegment::RedactedKey,
            PathSegment::Name("displayName"),
        ]
    );

    let mut root = Violations::new();
    check_value(&json!(""), &Plan::new().min_length(1), &[], &mut root);
    assert_eq!(root.iter().next().unwrap().path_text(), "value", "an empty path");
}

#[test]
fn keywords_use_json_schema_spellings_in_report_order() {
    let ordered = [
        (Keyword::MinLength, "minLength"),
        (Keyword::MaxLength, "maxLength"),
        (Keyword::Pattern, "pattern"),
        (Keyword::Minimum, "minimum"),
        (Keyword::ExclusiveMinimum, "exclusiveMinimum"),
        (Keyword::Maximum, "maximum"),
        (Keyword::ExclusiveMaximum, "exclusiveMaximum"),
        (Keyword::MultipleOf, "multipleOf"),
        (Keyword::MinItems, "minItems"),
        (Keyword::MaxItems, "maxItems"),
        (Keyword::UniqueItems, "uniqueItems"),
        (Keyword::Enum, "enum"),
        (Keyword::Const, "const"),
    ];
    for (keyword, spelling) in ordered {
        assert_eq!(keyword.as_str(), spelling);
        assert_eq!(keyword.to_string(), spelling);
    }
    for pair in ordered.windows(2) {
        assert!(pair[0].0 < pair[1].0, "{} before {}", pair[0].1, pair[1].1);
    }
}
