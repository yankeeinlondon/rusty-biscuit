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

// ── A checkout copy whose root closure is removed ──────────────────────────
//
// A checkout copy at a shipped definition's repository-relative path is the
// selected definition (it replaces the compiled-in copy). With its root
// `$constraints: { closed: true }` removed it is open, so an added `note`
// must pass: the kind checks verify meaning only, never the compiled shape
// again. The semantic checks themselves still run under the open copy.

/// `shipped` with every root-closure line removed.
fn opened(shipped: &str) -> String {
    let opened: String =
        shipped.lines().filter(|line| line.trim() != "$constraints: { closed: true }").map(|line| format!("{line}\n")).collect();
    assert_ne!(opened, shipped, "the definition had a root closure to remove");
    opened
}

/// Validates documents beside an opened checkout copy of a shipped
/// definition: the copy is written to `relative` in the fixture repository,
/// whose folder `SCHEMAS_DIR` names as a schema root.
struct OpenedCopy {
    fixture: Fixture,
    roots: String,
}

impl OpenedCopy {
    fn new(relative: &str, shipped: &str) -> Self {
        let fixture = Fixture::new();
        let copy = fixture.repo().join(relative);
        crate::yaml_kind_registry::write(&copy, &opened(shipped));
        let roots = copy.parent().expect("copy folder").display().to_string();
        Self { fixture, roots }
    }

    fn validate(&self, text: &str) -> YamlValidation {
        let document = self.fixture.repo().join("docs/doc.yaml");
        crate::yaml_kind_registry::write(&document, text);
        let context = self.fixture.context(&document, &[("SCHEMAS_DIR", self.roots.as_str())]);
        match darkmatter::markdown::schemas::DarkmatterSchemas::new(context).validate_yaml_document(text, &document) {
            Ok(YamlOutcome::Validated(validation)) => validation,
            other => panic!("expected a validated document, got {other:?}"),
        }
    }

    /// `text` and `text` with an added `note: hi` both pass.
    fn assert_open(&self, text: &str) {
        let control = self.validate(text);
        assert!(
            control.definition.ends_with("schemas/schema.yaml")
                || control.definition.ends_with("schemas/yaml-schema.yaml")
                || control.definition.ends_with("schemas/schema-trigger.yaml"),
            "{}",
            control.definition
        );
        assert!(std::path::Path::new(&control.definition).is_absolute(), "the checkout copy is selected: {}", control.definition);
        assert!(control.report.valid, "control: {:?}", control.report.problems);
        let added = self.validate(&format!("{text}note: hi\n"));
        assert!(added.report.valid, "added `note`: {:?}", added.report.problems);
    }

    /// The single problem of `text`'s report, which must be a kind check.
    fn kind_check(&self, text: &str) -> simplified_schema::ValidationProblem {
        let validation = self.validate(text);
        assert_eq!(validation.report.problems.len(), 1, "{:?}", validation.report.problems);
        let problem = validation.report.problems[0].clone();
        assert_eq!(problem.code, ValidationProblemCode::KindCheck, "{problem:?}");
        problem
    }
}

#[test]
fn an_opened_schema_definition_admits_an_added_property_and_keeps_its_checks() {
    let copy = OpenedCopy::new("simplified-schema/schemas/schema.yaml", include_str!("../../../../simplified-schema/schemas/schema.yaml"));
    copy.assert_open("kind: schema\n$schema:\n    title: string\n");
    copy.assert_open("kind: schema\ncategories: [text]\ntypes:\n    title: string(category(text))\n");

    let problem = copy.kind_check("kind: schema\ncategories: [text]\ntypes:\n    title: string(category(media))\nnote: hi\n");
    assert!(problem.message.contains("category `media` is not declared"), "{}", problem.message);
    assert_eq!(problem.line, Some(4));
}

/// A `Name@file` import is a direct library load: it is admitted against
/// the compiled `schema.yaml`, never the checkout copy, so an imported file
/// with an unknown root key is still refused under the opened copy.
#[test]
fn an_imported_file_is_still_admitted_against_the_compiled_definition() {
    let copy = OpenedCopy::new("simplified-schema/schemas/schema.yaml", include_str!("../../../../simplified-schema/schemas/schema.yaml"));
    crate::yaml_kind_registry::write(&copy.fixture.repo().join("docs/lib.yaml"), "kind: schema\ntypes:\n    Host: string\n");
    copy.assert_open("kind: schema\n$schema:\n    host: Host@./lib.yaml\n");

    crate::yaml_kind_registry::write(&copy.fixture.repo().join("docs/lib.yaml"), "kind: schema\ntypes:\n    Host: string\nnote: hi\n");
    let problem = copy.kind_check("kind: schema\n$schema:\n    host: Host@./lib.yaml\n");
    assert!(problem.message.contains("'note' was unexpected"), "{}", problem.message);
}

#[test]
fn an_opened_yaml_schema_definition_admits_an_added_property_and_keeps_its_checks() {
    let copy = OpenedCopy::new(
        "simplified-schema/schemas/yaml-schema.yaml",
        include_str!("../../../../simplified-schema/schemas/yaml-schema.yaml"),
    );
    copy.assert_open("kind: yaml-schema\n$schema:\n    kind: literal(inventory; required)\n    count: number\n");

    let problem = copy.kind_check("kind: yaml-schema\n$schema:\n    kind: literal(inventory; required)\n    host: missing@./absent.yaml\nnote: hi\n");
    assert!(problem.message.contains("absent.yaml"), "{}", problem.message);
}

#[test]
fn an_opened_trigger_definition_admits_an_added_property_and_keeps_its_checks() {
    let copy = OpenedCopy::new("darkmatter/schemas/schema-trigger.yaml", include_str!("../../../schemas/schema-trigger.yaml"));
    copy.assert_open(VALID_TRIGGER);

    let problem = copy.kind_check(&format!("{}note: hi\n", VALID_TRIGGER.replace("\"docs/**\"", "\"@/docs/**\"")));
    assert!(problem.message.contains("cannot use `@`"), "{}", problem.message);
    assert_eq!((problem.line, problem.column), (Some(4), Some(18)));
}

/// The unmodified shipped `schema` definition is closed and its metadata
/// non-null: the selected definition, not a kind check, rejects both.
#[test]
fn the_shipped_schema_definition_rejects_a_null_description_and_an_unknown_key() {
    for (text, code) in [
        ("kind: schema\ndescription: null\n$schema:\n    title: string\n", ValidationProblemCode::TypeMismatch),
        ("kind: schema\n$schema:\n    title: string\nnote: hi\n", ValidationProblemCode::UnknownKey),
    ] {
        let validation = validation(text);
        assert!(!validation.report.valid, "{text}");
        assert!(
            validation.report.problems.iter().all(|problem| problem.code != ValidationProblemCode::KindCheck),
            "{text}: {:?}",
            validation.report.problems
        );
        assert!(validation.report.problems.iter().any(|problem| problem.code == code), "{text}: {:?}", validation.report.problems);
    }
}

/// The kind checks read the copy the definition accepted: a category the
/// definition's `string[]` converts from a number is read as that string.
#[test]
fn the_schema_checks_read_the_softly_converted_categories() {
    let validation = validation("kind: schema\ncategories: [2024]\ntypes:\n    title: string(category(2024))\n");
    assert!(validation.report.valid, "{:?}", validation.report.problems);
}
