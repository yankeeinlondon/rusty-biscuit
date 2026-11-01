//! The checks of Darkmatter's own document kinds that their definitions
//! cannot state, through `DarkmatterSchemas::validate_yaml_document`: each
//! is a located `KindCheck` problem in the same report as the definition's
//! problems, and runs only once the definition passes.

use darkmatter::markdown::schemas::{Route, YamlOutcome, YamlValidation};
use simplified_schema::ValidationProblemCode;

use crate::yaml_kind_registry::Fixture;

fn validation(text: &str) -> YamlValidation {
    let fixture = Fixture::new();
    match fixture.validate(&fixture.package().join("doc.yaml"), text) {
        Ok(YamlOutcome::Validated(validation)) => validation,
        other => panic!("expected a validated document, got {other:?}"),
    }
}

/// The single problem of `text`'s report, which must be a kind check.
fn kind_check(text: &str) -> (simplified_schema::ValidationProblem, String) {
    let validation = validation(text);
    assert!(!validation.report.valid, "{text}");
    assert_eq!(validation.report.problems.len(), 1, "{:?}", validation.report.problems);
    let problem = validation.report.problems[0].clone();
    assert_eq!(problem.code, ValidationProblemCode::KindCheck, "{problem:?}");
    let token = validation.byte_range(&problem).map(|range| text[range].to_string()).unwrap_or_default();
    (problem, token)
}

const VALID_TRIGGER: &str = "kind: schema-trigger\nmatch:\n    all:\n        - $path: \"docs/**\"\n        - title: string(required)\n";

#[test]
fn control_a_valid_trigger_passes_its_definition_and_its_checks() {
    let validation = validation(VALID_TRIGGER);
    assert_eq!(validation.route, Route::Kind("schema-trigger".into()));
    assert!(validation.report.valid, "{:?}", validation.report.problems);
}

#[test]
fn an_invalid_path_glob_is_located_on_the_glob() {
    let text = VALID_TRIGGER.replace("\"docs/**\"", "\"@/docs/**\"");
    let (problem, token) = kind_check(&text);
    assert!(problem.message.contains("cannot use `@`"), "{}", problem.message);
    assert_eq!((problem.line, problem.column), (Some(4), Some(18)));
    assert_eq!(token, "\"@/docs/**\"");
}

#[test]
fn an_empty_match_list_is_located_on_match() {
    let (problem, _) = kind_check("kind: schema-trigger\nmatch: []\n");
    assert!(problem.message.contains("at least one condition"), "{}", problem.message);
    assert_eq!(problem.line, Some(2));
}

#[test]
fn a_vacuous_match_is_located_on_the_match_expression() {
    let (problem, token) = kind_check("kind: schema-trigger\nmatch:\n    title: string\n");
    assert!(problem.message.contains("vacuous"), "{}", problem.message);
    assert_eq!(problem.line, Some(3), "{problem:?}");
    assert!(token.starts_with("title: string"), "{token:?}");
}

#[test]
fn min_match_count_bounds_are_located_on_the_count() {
    for count in ["0", "3"] {
        let text = format!(
            "kind: schema-trigger\nmatch:\n    min-match:\n        count: {count}\n        of:\n            - a: string(required)\n            - b: string(required)\n"
        );
        let (problem, token) = kind_check(&text);
        assert!(problem.message.contains("min-match.count"), "{}", problem.message);
        assert_eq!(problem.line, Some(4), "{count}");
        assert_eq!(token, count);
    }
}

#[test]
fn an_undeclared_category_is_located_on_its_type() {
    let text = "kind: schema\ncategories: [text]\ntypes:\n    title: string(category(media))\n";
    let (problem, _) = kind_check(text);
    assert!(problem.message.contains("category `media` is not declared"), "{}", problem.message);
    assert_eq!(problem.line, Some(4));
    let declared = text.replace("[text]", "[text, media]");
    assert!(validation(&declared).report.valid);
}

#[test]
fn an_unresolvable_import_in_a_schema_document_is_a_kind_check() {
    let (problem, _) = kind_check("kind: schema\ntypes:\n    title: missing@./absent.yaml\n");
    assert!(problem.message.contains("absent.yaml"), "{}", problem.message);
}

#[test]
fn a_definition_failure_stops_the_kind_checks() {
    // Both an undeclared key (the definition's problem) and an invalid glob
    // (a kind check): only the definition's problem is reported.
    let text = format!("{}extra: 1\n", VALID_TRIGGER.replace("\"docs/**\"", "\"@/docs/**\""));
    let validation = validation(&text);
    let codes: Vec<_> = validation.report.problems.iter().map(|problem| problem.code).collect();
    assert_eq!(codes, [ValidationProblemCode::UnknownKey], "{:?}", validation.report.problems);
}

#[test]
fn a_user_kind_has_no_kind_checks() {
    let fixture = Fixture::new();
    crate::yaml_kind_registry::write(
        &fixture.repo().join("schemas/note.yaml"),
        "kind: yaml-schema\n$schema:\n    kind: literal(note; required)\n    match: unknown\n",
    );
    let document = fixture.package().join("n.yaml");
    // `match: []` would fail a trigger's checks; a note has none.
    let Ok(YamlOutcome::Validated(validation)) = fixture.validate(&document, "kind: note\nmatch: []\n") else {
        panic!("validated");
    };
    assert!(validation.report.valid, "{:?}", validation.report.problems);
}
