//! The library's strict and soft validation
//! ([`simplified_schema::Prepared`]) agree with Darkmatter's existing exact
//! and coercing operations on the same effective schema: strict with
//! [`EffectiveSchema::validate_raw`], soft with [`EffectiveSchema::validate`]
//! for the verdict and problem paths and with
//! [`EffectiveSchema::coerce_frontmatter_with_pending`] for the accepted
//! value. The cells are the R6 rows and the nested, union, and precision
//! cases of `schema_extraction_characterization`.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, ValidationReport};
use serde_json::{Value, json};
use simplified_schema::{Prepared, ValidationFailure, ValidationSuccess};

/// The effective schema of a document at `dir/doc.md` declaring `schema`
/// (the lines under `$schema:`), and the same lowered schema prepared by the
/// library.
fn schemas(dir: &Path, schema: &str) -> (EffectiveSchema, Prepared) {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    let source = source.with_source(ComposeSource::File(dir.join("doc.md")));
    let effective = DarkmatterSchemas::new(crate::request_support::context_at(dir))
        .effective_for(&source)
        .expect("schema resolves")
        .expect("schema is present");
    let prepared = Prepared::new(
        Arc::clone(&effective.json_schema),
        Some(dir.to_path_buf()),
        biscuit_file::FileResolutionContext::new(dir),
    )
    .expect("schema compiles");
    (effective, prepared)
}

fn problem_paths(report: &ValidationReport) -> Vec<String> {
    report.problems.iter().map(|problem| problem.path.clone()).collect()
}

fn failure_paths(failure: &ValidationFailure) -> Vec<String> {
    failure.problems().iter().map(|problem| problem.path.clone()).collect()
}

/// Asserts both methods agree with the existing operations on `input`.
#[track_caller]
fn assert_parity(schema: &str, input: Value) {
    let workspace = tempfile::tempdir().expect("workspace");
    let (effective, prepared) = schemas(workspace.path(), schema);

    let raw = effective.validate_raw(&input);
    match prepared.validate_strict(&input) {
        Ok(ValidationSuccess { value, .. }) => {
            assert!(raw.valid, "strict accepted what validate_raw rejects: {:?}", raw.problems);
            assert!(std::ptr::eq(value.as_ref(), &input), "strict success borrows the input");
        }
        Err(failure) => assert_eq!(failure_paths(&failure), problem_paths(&raw), "strict vs validate_raw"),
    }

    let coercing = effective.validate(&input);
    let coerced = effective.coerce_frontmatter_with_pending(&input, &HashSet::new());
    match prepared.validate_soft(&input) {
        Ok(success) => {
            assert!(coercing.valid, "soft accepted what validate rejects: {:?}", coercing.problems);
            assert_eq!(success.is_converted(), coerced.changed, "soft converts exactly when coercion does");
            assert_eq!(success.value.as_ref(), &coerced.value, "soft accepts the coerced value");
        }
        Err(failure) => {
            assert!(!coercing.valid);
            assert_eq!(failure_paths(&failure), problem_paths(&coercing), "soft vs validate");
        }
    }
}

#[test]
fn number_with_number_4() {
    assert_parity("  v: number\n", json!({ "v": 4 }));
}

#[test]
fn number_with_string_4() {
    assert_parity("  v: number\n", json!({ "v": "4" }));
}

#[test]
fn integer_with_string_2_5() {
    assert_parity("  v: number(integer)\n", json!({ "v": "2.5" }));
}

#[test]
fn string_or_number_with_string_4() {
    assert_parity("  v: string | number\n", json!({ "v": "4" }));
}

#[test]
fn boolean_or_number_with_string_4() {
    assert_parity("  v: boolean | number\n", json!({ "v": "4" }));
}

#[test]
fn inexact_number_text() {
    assert_parity("  n: number\n", json!({ "n": "9007199254740993" }));
}

#[test]
fn nested_values_convert() {
    assert_parity(
        "  inner: \"{ flag: boolean }\"\n  tags: \"number[]\"\n",
        json!({ "inner": { "flag": "true" }, "tags": ["1", 2] }),
    );
}

#[test]
fn mixed_convertible_and_failing_values() {
    assert_parity(
        "  n: number\n  i: number(integer)\n  tags: \"string[]\"\n  inner: \"{ flag: boolean }\"\n",
        json!({ "n": "4", "i": "2.5", "tags": [1, "two"], "inner": { "flag": "true" } }),
    );
}
