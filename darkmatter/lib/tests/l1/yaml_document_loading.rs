//! Document shapes of the one whole-file YAML loader, through
//! `DarkmatterSchemas::validate_yaml_document`: every load failure is a
//! located `SchemaDocument` error (even when no schema could be selected or
//! the file has an alias),
//! document markers, a byte-order mark, and CRLF lines are accepted, and a
//! root that is not a mapping is valid but unvalidated.

use darkmatter::markdown::schemas::{Route, YamlOutcome};
use simplified_schema::{SchemaError, ValidationProblemCode};

use crate::yaml_kind_registry::Fixture;

/// A document that validates against its inline `$schema`.
const CONTROL: &str = "$schema:\n    name: string(required)\n    port: number\nname: api\nport: 8080\n";

fn load_error(text: &str) -> (String, Option<std::ops::Range<usize>>) {
    let fixture = Fixture::new();
    match fixture.validate(&fixture.package().join("doc.yaml"), text) {
        Err(SchemaError::SchemaDocument { message, span, path, .. }) => {
            assert!(path.ends_with("doc.yaml"), "{path:?}");
            (message, span)
        }
        other => panic!("expected a load error for {text:?}, got {other:?}"),
    }
}

fn outcome(text: &str) -> YamlOutcome {
    let fixture = Fixture::new();
    fixture.validate(&fixture.package().join("doc.yaml"), text).expect("loads")
}

#[test]
fn control_a_valid_mapping_validates() {
    let YamlOutcome::Validated(validation) = outcome(CONTROL) else { panic!("validated") };
    assert_eq!(validation.route, Route::RootSchema);
    assert!(validation.report.valid, "{:?}", validation.report.problems);
}

#[test]
fn a_duplicate_key_is_a_located_load_error() {
    // Located where the YAML parser reports it: the mapping holding the key.
    let text = "name: a\nname: b\n";
    let (message, span) = load_error(text);
    assert!(message.contains("duplicate entry with key \"name\""), "{message}");
    assert!(span.is_some(), "{message}");
}

#[test]
fn a_non_string_mapping_key_is_a_load_error_at_the_key() {
    let text = "base: &b roster\nkind: *b\nlimits:\n    42: high\n";
    let (message, span) = load_error(text);
    assert!(message.contains("`/limits`") && message.contains("not a string"), "{message}");
    let span = span.expect("located even after an alias");
    assert_eq!(simplified_schema::line_col_of_offset(text, span.start), (4, 5), "{message}");
}

#[test]
fn an_inadmissible_number_is_a_load_error() {
    // Beyond the exact range of the number model.
    let text = "kind: roster\nsize: 9007199254740993\n";
    let (message, span) = load_error(text);
    assert!(message.contains("quote it"), "{message}");
    let span = span.expect("located");
    assert_eq!(simplified_schema::line_col_of_offset(text, span.start).0, 2, "{message}");
}

#[test]
fn malformed_yaml_is_a_load_error_never_an_unclaimed_success() {
    let (message, span) = load_error("name: [unclosed\n");
    assert!(message.starts_with("not valid YAML"), "{message}");
    assert!(span.is_some());
}

#[test]
fn a_second_document_is_a_load_error_at_its_marker() {
    let text = "name: a\n---\nname: b\n";
    let (message, span) = load_error(text);
    assert!(message.contains("more than one document"), "{message}");
    assert_eq!(span, Some(8..8), "{message}");
}

#[test]
fn document_markers_are_accepted_not_read_as_frontmatter_fences() {
    let text = format!("---\n{CONTROL}...\n");
    let YamlOutcome::Validated(validation) = outcome(&text) else { panic!("validated") };
    assert!(validation.report.valid, "{:?}", validation.report.problems);
}

#[test]
fn a_byte_order_mark_and_crlf_lines_are_accepted_and_spans_point_into_the_original_text() {
    let invalid = CONTROL.replace("port: 8080", "port: high");
    for (label, text) in [
        ("BOM", format!("\u{feff}{invalid}")),
        ("CRLF", invalid.replace('\n', "\r\n")),
        ("BOM + CRLF", format!("\u{feff}{}", invalid.replace('\n', "\r\n"))),
        ("Unicode", invalid.replace("name: api", "name: «Ωμέγα»")),
    ] {
        let YamlOutcome::Validated(validation) = outcome(&text) else { panic!("{label}: validated") };
        assert_eq!(validation.report.problems.len(), 1, "{label}: {:?}", validation.report.problems);
        let problem = &validation.report.problems[0];
        assert_eq!(problem.code, ValidationProblemCode::TypeMismatch, "{label}");
        assert_eq!(problem.line, Some(5), "{label}");
        let range = validation.byte_range(problem).expect("located");
        assert_eq!(&text[range], "high", "{label}");
    }
}

/// A document whose `one` is an alias of `base`.
const ALIASED: &str = "$schema:\n    name: string(required)\n    base: \"{ port: number }\"\n    one: \"{ port: number }\"\nname: api\nbase: &b {port: 8080}\none: *b\n";

#[test]
fn control_an_aliased_document_validates() {
    let YamlOutcome::Validated(validation) = outcome(ALIASED) else { panic!("validated") };
    assert!(validation.report.valid, "{:?}", validation.report.problems);
}

/// The anchored original is located at its failing value and the alias use
/// at its `*name` token, in every encoding variant.
#[test]
fn an_alias_use_is_located_at_its_token() {
    let invalid = ALIASED.replace("port: 8080", "port: x");
    for (label, text) in [
        ("LF", invalid.clone()),
        ("BOM", format!("\u{feff}{invalid}")),
        ("CRLF", invalid.replace('\n', "\r\n")),
        ("BOM + CRLF", format!("\u{feff}{}", invalid.replace('\n', "\r\n"))),
        ("Unicode", invalid.replace("name: api", "name: «Ωμέγα»")),
    ] {
        let YamlOutcome::Validated(validation) = outcome(&text) else { panic!("{label}: validated") };
        let problems = &validation.report.problems;
        assert_eq!(problems.len(), 2, "{label}: {problems:?}");
        let mut found: Vec<_> = problems
            .iter()
            .map(|problem| {
                let range = validation.byte_range(problem).unwrap_or_else(|| panic!("{label}: {problem:?} located"));
                (problem.path.clone(), problem.line, &text[range])
            })
            .collect();
        found.sort();
        assert_eq!(
            found,
            [("/base/port".to_string(), Some(6), "x"), ("/one/port".to_string(), Some(7), "*b")],
            "{label}"
        );
    }
}

#[test]
fn a_scalar_sequence_or_empty_root_is_valid_but_unvalidated() {
    for text in ["just text\n", "- a\n- b\n", "", "# only a comment\n", "~\n"] {
        assert!(matches!(outcome(text), YamlOutcome::NoSchema), "{text:?}");
    }
}

#[test]
fn loading_and_validating_are_separate_steps_so_a_referenced_files_load_error_is_not_the_documents() {
    use darkmatter::markdown::schemas::{DarkmatterSchemas, LoadedYamlDocument};

    let fixture = Fixture::new();
    let document = fixture.package().join("doc.yaml");
    let text = "$schema: ./contract.yaml\nname: api\n";
    // The referenced schema is not admissible YAML, so resolving it fails
    // with the same error type a document load failure has.
    std::fs::write(fixture.package().join("contract.yaml"), "a: [1\n").unwrap();
    let outcome = fixture.validate(&document, text);
    assert!(matches!(outcome, Err(SchemaError::SchemaDocument { .. })), "{outcome:?}");

    // The document itself loads; the failure comes from the validation step.
    let loaded = LoadedYamlDocument::load(text, &document).expect("the document is admissible YAML");
    let validated = DarkmatterSchemas::new(fixture.context(&document, &[])).validate_loaded_yaml_document(&loaded);
    let Err(SchemaError::SchemaDocument { path, .. }) = validated else { panic!("{validated:?}") };
    assert!(path.ends_with("contract.yaml"), "{path:?}");

    // A document that is not admissible YAML fails the load step itself.
    let Err(SchemaError::SchemaDocument { path, .. }) = LoadedYamlDocument::load("a: [1\n", &document) else {
        panic!("a malformed document must fail to load")
    };
    assert_eq!(path, document);
}
