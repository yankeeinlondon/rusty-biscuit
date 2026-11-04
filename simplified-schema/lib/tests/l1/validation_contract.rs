//! The strict/soft validation contract of [`Prepared`]: strict accepts only a
//! value that already matches; soft applies the conversion engine's existing
//! rules first. Both borrow the input on an unconverted success, own the
//! value only after a conversion, never change the input, and fail with a
//! typed [`ValidationFailure`] that keeps every structured problem and no
//! value.
//!
//! The four R6 rows match what Darkmatter's exact (`validate_raw`) and
//! coercing (`validate`, `coerce_frontmatter_with_pending`) operations
//! produce for the same cells, pinned before the extraction in Darkmatter's
//! `schema_extraction_characterization::r6_table`.

use std::borrow::Cow;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use simplified_schema::coerce::coerce_frontmatter;
use simplified_schema::{
    Prepared, SchemaAdvisory, ValidationFailure, ValidationProblemCode, ValidationSuccess, parse_yaml_schema,
    to_json_schema,
};

/// `schema` (SimplifiedSchema YAML, the body of a `$schema:` key) lowered
/// and prepared, with file values resolving from `dir`.
fn prepared_in(dir: &std::path::Path, schema: &str) -> Prepared {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(schema).expect("schema YAML");
    let json_schema = to_json_schema(&parse_yaml_schema(&yaml).expect("schema parses")).expect("schema lowers");
    Prepared::new(Arc::new(json_schema), None, FileResolutionContext::new(dir)).expect("schema compiles")
}

/// [`prepared_in`] for a schema that judges no file value.
fn prepared(schema: &str) -> Prepared {
    prepared_in(&std::env::temp_dir(), schema)
}

#[track_caller]
fn borrowed<'a>(result: &'a Result<ValidationSuccess<'_>, ValidationFailure>, input: &Value) -> &'a Value {
    match result {
        Ok(ValidationSuccess { value: Cow::Borrowed(value), .. }) => {
            assert!(std::ptr::eq(*value, input), "the accepted value is the caller's own input");
            value
        }
        other => panic!("expected the input accepted as written, got {other:?}"),
    }
}

#[track_caller]
fn owned(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Value {
    match result {
        Ok(success @ ValidationSuccess { value: Cow::Owned(_), .. }) => {
            assert!(success.is_converted());
            success.value.into_owned()
        }
        other => panic!("expected an owned converted value, got {other:?}"),
    }
}

#[track_caller]
fn invalid(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Vec<simplified_schema::ValidationProblem> {
    match result {
        Err(ValidationFailure::InvalidData { problems }) => {
            assert!(!problems.is_empty(), "a failure keeps its problems");
            problems
        }
        other => panic!("expected invalid data, got {other:?}"),
    }
}

fn paths(problems: &[simplified_schema::ValidationProblem]) -> Vec<&str> {
    problems.iter().map(|problem| problem.path.as_str()).collect()
}

/// The spec's R6 table, one test per row, through both methods.
mod r6_table {
    use super::*;

    #[test]
    fn number_with_number_4_is_borrowed_by_both() {
        let schema = prepared("v: number\n");
        let input = json!({ "v": 4 });
        assert_eq!(borrowed(&schema.validate_strict(&input), &input), &json!({ "v": 4 }));
        assert_eq!(borrowed(&schema.validate_soft(&input), &input), &json!({ "v": 4 }));
    }

    #[test]
    fn number_with_string_4_is_rejected_strictly_and_owned_as_4_softly() {
        let schema = prepared("v: number\n");
        let input = json!({ "v": "4" });
        let problems = invalid(schema.validate_strict(&input));
        assert_eq!(paths(&problems), ["/v"]);
        assert_eq!(problems[0].code, ValidationProblemCode::TypeMismatch);
        assert_eq!(owned(schema.validate_soft(&input)), json!({ "v": 4 }));
        assert_eq!(input, json!({ "v": "4" }), "conversion left the input as written");
        // Parity with the existing coercing operation's effective value.
        let coerced = coerce_frontmatter(schema.json_schema(), &input);
        assert!(coerced.changed);
        assert_eq!(coerced.value, json!({ "v": 4 }));
    }

    #[test]
    fn integer_with_string_2_5_is_rejected_by_both_and_never_repaired() {
        let schema = prepared("v: number(integer)\n");
        let input = json!({ "v": "2.5" });
        assert_eq!(paths(&invalid(schema.validate_strict(&input))), ["/v"]);
        assert_eq!(paths(&invalid(schema.validate_soft(&input))), ["/v"]);
        assert_eq!(input, json!({ "v": "2.5" }));
        assert!(!coerce_frontmatter(schema.json_schema(), &input).changed, "the coercing operation keeps it too");
    }

    #[test]
    fn string_or_number_with_string_4_is_borrowed_as_the_string_by_both() {
        let schema = prepared("v: string | number\n");
        let input = json!({ "v": "4" });
        assert_eq!(borrowed(&schema.validate_strict(&input), &input), &json!({ "v": "4" }));
        assert_eq!(borrowed(&schema.validate_soft(&input), &input), &json!({ "v": "4" }));
    }
}

/// Conversion follows the existing engine's rules below the top level and
/// across unions, and never repairs a constraint.
mod conversion_rules {
    use super::*;

    #[test]
    fn nested_values_convert_inside_mappings_and_lists() {
        let schema = prepared("inner: \"{ flag: boolean }\"\ntags: \"number[]\"\n");
        let input = json!({ "inner": { "flag": "true" }, "tags": ["1", 2] });
        assert_eq!(
            paths(&invalid(schema.validate_strict(&input))),
            ["/inner/flag", "/tags/0"],
            "strict judges every nested value as written"
        );
        let accepted = owned(schema.validate_soft(&input));
        assert_eq!(accepted, json!({ "inner": { "flag": true }, "tags": [1, 2] }));
        assert_eq!(accepted, coerce_frontmatter(schema.json_schema(), &input).value);
    }

    #[test]
    fn a_union_keeps_the_arm_that_accepts_as_written_before_converting() {
        let schema = prepared("v: boolean | number\n");
        let as_written = json!({ "v": 4 });
        borrowed(&schema.validate_soft(&as_written), &as_written);
        let text = json!({ "v": "4" });
        assert_eq!(paths(&invalid(schema.validate_strict(&text))), ["/v"]);
        let accepted = owned(schema.validate_soft(&text));
        assert_eq!(accepted, coerce_frontmatter(schema.json_schema(), &text).value);
        assert_eq!(accepted, json!({ "v": 4 }));
    }

    #[test]
    fn a_number_the_model_cannot_hold_exactly_fails_with_the_reason() {
        let schema = prepared("n: number\n");
        let input = json!({ "n": "9007199254740993" });
        let problems = invalid(schema.validate_soft(&input));
        assert_eq!(paths(&problems), ["/n"]);
        assert!(problems[0].message.contains("cannot be represented exactly"), "{}", problems[0].message);
        assert_eq!(input, json!({ "n": "9007199254740993" }));
    }

    #[test]
    fn one_failing_value_fails_the_whole_soft_validation_without_a_value() {
        let schema = prepared("n: number\ni: number(integer)\n");
        let input = json!({ "n": "4", "i": "2.5" });
        // `n` converts on its own, but the failure carries only problems.
        let problems = invalid(schema.validate_soft(&input));
        assert_eq!(paths(&problems), ["/i"]);
    }
}

/// A failure keeps structured problems (codes, instance and schema paths)
/// rather than strings.
mod diagnostics {
    use super::*;

    #[test]
    fn every_problem_is_kept_with_its_paths_and_code() {
        let schema = prepared("name: string(required)\nage: number\ninner: \"{ flag: boolean }\"\n");
        let input = json!({ "age": "old", "inner": { "flag": true, "extra": 1 } });
        let problems = invalid(schema.validate_soft(&input));
        let mut codes: Vec<_> = problems.iter().map(|problem| problem.code).collect();
        codes.sort();
        assert_eq!(
            codes,
            [
                ValidationProblemCode::MissingRequired,
                ValidationProblemCode::TypeMismatch,
                ValidationProblemCode::UnknownKey,
            ],
            "{problems:#?}"
        );
        let mismatch = problems.iter().find(|p| p.code == ValidationProblemCode::TypeMismatch).unwrap();
        assert_eq!(mismatch.instance_path.segments(), ["age"]);
        assert!(mismatch.schema_path.is_some(), "the failing keyword is located in the schema");
        let missing = problems.iter().find(|p| p.code == ValidationProblemCode::MissingRequired).unwrap();
        assert_eq!(missing.property.as_deref(), Some("name"));
        let unknown = problems.iter().find(|p| p.code == ValidationProblemCode::UnknownKey).unwrap();
        assert_eq!(unknown.offending_property.as_deref(), Some("extra"));
        assert_eq!(unknown.path, "/inner", "an unknown key is reported on its parent");
        assert!(problems.iter().all(|p| !p.is_unavailable_check()));
    }

    #[test]
    fn a_declared_description_rides_on_its_problem() {
        let schema = prepared("age: number -> Age in years\n");
        let input = json!({ "age": "old" });
        let problems = invalid(schema.validate_strict(&input));
        assert_eq!(problems[0].description.as_deref(), Some("Age in years"));
    }

    #[test]
    fn success_reports_the_schema_advisories() {
        let advisory = SchemaAdvisory::missing_simplified_envelope("types.yaml");
        let schema = prepared("v: number\n").with_advisories(vec![advisory.clone()]);
        let input = json!({ "v": "4" });
        assert_eq!(schema.validate_strict(&json!({ "v": 4 })).unwrap().advisories, std::slice::from_ref(&advisory));
        assert_eq!(schema.validate_soft(&input).unwrap().advisories, [advisory]);
    }
}

/// Neither method changes its input or its schema, and one prepared schema
/// judges many values.
mod immutability {
    use super::*;

    #[test]
    fn both_methods_leave_input_and_schema_unchanged() {
        let schema = prepared("n: number\ni: number(integer)\ntags: \"string[]\"\ninner: \"{ flag: boolean }\"\n");
        let schema_before = schema.json_schema().as_ref().clone();
        let inputs = [
            json!({ "n": "4", "i": "2.5", "tags": [1, "two"], "inner": { "flag": "true" } }),
            json!({ "n": "4", "i": 2, "tags": ["one"], "inner": { "flag": "true" } }),
            json!({ "n": 4, "i": 2, "tags": [], "inner": { "flag": false } }),
        ];
        for input in &inputs {
            let before = input.clone();
            let _ = schema.validate_strict(input);
            let _ = schema.validate_soft(input);
            assert_eq!(input, &before, "validation changed its input");
        }
        assert!(schema.validate_soft(&inputs[1]).unwrap().is_converted(), "the probe must exercise a conversion");
        borrowed(&schema.validate_strict(&inputs[2]), &inputs[2]);
        assert_eq!(schema.json_schema().as_ref(), &schema_before, "validation changed the schema");
    }
}

/// A root union is judged arm by arm, by both methods.
mod root_union {
    use super::*;

    #[test]
    fn a_root_union_borrows_an_arm_match_and_converts_into_an_arm() {
        let schema = prepared("- kind: literal(a)\n  n: number\n- kind: literal(b)\n  s: string\n");
        let as_written = json!({ "kind": "a", "n": 1 });
        borrowed(&schema.validate_strict(&as_written), &as_written);
        let text = json!({ "kind": "a", "n": "1" });
        let problems = invalid(schema.validate_strict(&text));
        assert_eq!(problems[0].arm_index, Some(0), "the discriminant selects the reporting arm");
        assert_eq!(owned(schema.validate_soft(&text)), json!({ "kind": "a", "n": 1 }));
    }
}

/// A check this build cannot run fails as "could not complete", never as a
/// pass; with the `file` feature the same check runs (the control cases).
mod unavailable_checks {
    use super::*;

    /// A folder holding `note.md`.
    fn workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "note").unwrap();
        dir
    }

    #[test]
    fn an_eager_file_value_is_accepted_only_where_it_can_be_resolved() {
        let dir = workspace();
        let schema = prepared_in(dir.path(), "doc: file(eager)\n");
        let input = json!({ "doc": "./note.md" });
        for result in [schema.validate_strict(&input), schema.validate_soft(&input)] {
            if cfg!(feature = "file") {
                borrowed(&result, &input);
            } else {
                match result {
                    Err(ValidationFailure::UnavailableCheck { problems }) => {
                        assert_eq!(paths(&problems), ["/doc"]);
                        assert_eq!(problems[0].code, ValidationProblemCode::InvalidFileReference);
                        assert!(problems[0].is_unavailable_check());
                    }
                    other => panic!("expected an unavailable check, got {other:?}"),
                }
            }
        }
    }

    #[test]
    fn a_missing_file_is_invalid_data_with_resolution_and_unavailable_without() {
        let dir = workspace();
        let schema = prepared_in(dir.path(), "doc: file(eager)\n");
        let input = json!({ "doc": "./missing.md" });
        let failure = schema.validate_strict(&input).unwrap_err();
        assert_eq!(failure.problems().len(), 1);
        if cfg!(feature = "file") {
            assert!(matches!(failure, ValidationFailure::InvalidData { .. }), "{failure:?}");
        } else {
            assert!(matches!(failure, ValidationFailure::UnavailableCheck { .. }), "{failure:?}");
        }
    }

    #[test]
    fn a_decided_violation_beside_an_unavailable_check_is_invalid_data() {
        let dir = workspace();
        let schema = prepared_in(dir.path(), "doc: file(eager)\nn: number\n");
        let input = json!({ "doc": "./note.md", "n": "four" });
        let problems = invalid(schema.validate_soft(&input));
        let unavailable: Vec<&str> =
            problems.iter().filter(|p| p.is_unavailable_check()).map(|p| p.path.as_str()).collect();
        let expected: &[&str] = if cfg!(feature = "file") { &[] } else { &["/doc"] };
        assert_eq!(unavailable, expected);
        assert!(paths(&problems).contains(&"/n"));
    }

    #[test]
    fn malformed_file_syntax_is_invalid_data_in_every_build() {
        let dir = workspace();
        let schema = prepared_in(dir.path(), "doc: file(eager)\n");
        let input = json!({ "doc": "~alice/notes.md" });
        let problems = invalid(schema.validate_strict(&input));
        assert!(
            matches!(problems[0].file_reference, Some(simplified_schema::FileReferenceDiagnostic::InvalidSyntax { .. })),
            "{problems:#?}"
        );
        assert!(!problems[0].is_unavailable_check());
    }
}
