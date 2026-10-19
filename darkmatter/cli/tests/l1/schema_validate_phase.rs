//! The runtime phase of the `md schema validate` named-type fixture.
//!
//! `md schema validate` judges the authored document (`DarkmatterSchemas::validate`)
//! and has no flag selecting a runtime `SchemaPhase`, so it can show that an
//! absent inherited `generated` property passes but not that the runtime
//! phase then requires it. This module validates the same document through
//! the library at both phases.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, SchemaPhase, ValidationProblemCode, ValidationReport,
};
use std::path::Path;

/// `code: string(required; generated)` reused through a bare name, `@this`,
/// and a cross-file import.
fn validate_both_phases(dir: &Path, values: &str) -> (ValidationReport, ValidationReport) {
    std::fs::write(
        dir.join("types.yaml"),
        "kind: schema\ntypes:\n  code: string(required; generated)\n",
    )
    .unwrap();
    let document: Markdown = format!(
        "---\n$schema:\n  code: string(required; generated)\n  ga: code\n  gb: code@this\n  gc: code@./types.yaml\n{values}---\nBody\n"
    )
    .into();
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir));
    let authored = schemas.validate(&document).expect("schema resolves");
    let completion = schemas
        .validate_for_phase(&document, SchemaPhase::Completion)
        .expect("phase schema builds");
    (authored, completion)
}

fn missing_required(report: &ValidationReport) -> Vec<String> {
    let mut names: Vec<String> = report
        .problems
        .iter()
        .filter(|problem| problem.code == ValidationProblemCode::MissingRequired)
        .filter_map(|problem| problem.property.clone())
        .collect();
    names.sort();
    names
}

#[test]
fn schema_validate_phase_absent_generated_passes_authored_and_fails_completion() {
    let dir = tempfile::tempdir().unwrap();

    let (authored, completion) = validate_both_phases(dir.path(), "");

    assert!(authored.valid, "{:?}", authored.problems);
    assert_eq!(missing_required(&completion), ["code", "ga", "gb", "gc"]);
}

#[test]
fn schema_validate_phase_wrong_typed_generated_value_fails_both_phases() {
    let dir = tempfile::tempdir().unwrap();

    let (authored, completion) =
        validate_both_phases(dir.path(), "code: [1]\nga: [1]\ngb: [1]\ngc: [1]\n");

    for report in [&authored, &completion] {
        let mut paths: Vec<&str> = report
            .problems
            .iter()
            .map(|problem| problem.path.as_str())
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            ["/code", "/ga", "/gb", "/gc"],
            "{:?}",
            report.problems
        );
    }
}
