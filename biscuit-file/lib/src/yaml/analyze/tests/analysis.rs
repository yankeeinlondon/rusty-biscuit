//! Tests for the top-level analysis result: parse-outcome retention,
//! diagnostics access, and the classification-gated `apply()` path.

use super::super::{
    YamlAnalysis, YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlParseOutcome, YamlRepair,
};

fn parse(source: &str) -> YamlParseOutcome {
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source) {
        Ok(value) => YamlParseOutcome::Parsed(value),
        Err(error) => YamlParseOutcome::Failed(super::super::YamlParseFailure {
            message: error.to_string(),
            location: error.location().map(Into::into),
        }),
    }
}

fn diagnostic(
    code: YamlDiagnosticCode,
    classification: YamlCertainty,
    repairs: Vec<YamlRepair>,
) -> YamlDiagnostic {
    YamlDiagnostic {
        code,
        span: 0..1,
        classification,
        message: "test finding".to_string(),
        repairs,
    }
}

#[test]
fn test_parseable_outcome_retains_value() {
    let analysis = YamlAnalysis::new("key: value", parse("key: value"), vec![]);
    assert!(analysis.is_parseable());
    assert_eq!(
        analysis.parsed_value().unwrap()["key"],
        serde_yaml_ng::Value::String("value".to_string())
    );
    assert!(analysis.parse_failure().is_none());
    assert!(analysis.is_clean());
}

#[test]
fn test_unparseable_outcome_retains_failure() {
    let analysis = YamlAnalysis::new("title: @daily-report", parse("title: @daily-report"), vec![]);
    assert!(!analysis.is_parseable());
    assert!(analysis.parsed_value().is_none());
    let failure = analysis.parse_failure().unwrap();
    assert!(failure.message.contains("cannot start any token"));
    assert_eq!(failure.location.unwrap().byte, 7);
}

#[test]
fn test_diagnostics_are_returned_in_order() {
    let diagnostics = vec![
        diagnostic(YamlDiagnosticCode::Bom, YamlCertainty::Deterministic, vec![]),
        diagnostic(YamlDiagnosticCode::DuplicateKey, YamlCertainty::NonDeterministicFind, vec![]),
    ];
    let analysis = YamlAnalysis::new("x", parse("key: value"), diagnostics);
    assert_eq!(analysis.diagnostics().len(), 2);
    assert_eq!(analysis.diagnostics()[0].code, YamlDiagnosticCode::Bom);
    assert!(!analysis.is_clean());
}

#[test]
fn test_apply_gathers_only_deterministic_repairs() {
    // The classification gate: a report-only diagnostic carrying a candidate
    // must NOT contribute it to application.
    let source = "title: @daily-report";
    let quote = YamlRepair {
        span: 7..20,
        replacement: "\"@daily-report\"".to_string(),
        explanation: "quote the scalar".to_string(),
    };
    let diagnostics = vec![
        diagnostic(
            YamlDiagnosticCode::ReservedIndicator,
            YamlCertainty::Deterministic,
            vec![quote.clone()],
        ),
        diagnostic(
            YamlDiagnosticCode::SimilarKey,
            YamlCertainty::DeterministicFindNonDeterministicSolution,
            vec![YamlRepair {
                span: 0..5,
                replacement: "name".to_string(),
                explanation: "report-only guess".to_string(),
            }],
        ),
    ];
    let analysis = YamlAnalysis::new(source, parse(source), diagnostics);

    let outcome = analysis.apply();
    assert_eq!(outcome.source, "title: \"@daily-report\"");
    assert_eq!(outcome.audit.applied, vec![quote]);
    assert!(outcome.audit.rejected.is_empty());
}

#[test]
fn test_apply_with_no_repairs_is_byte_identical() {
    let source = "key: value\n";
    let analysis = YamlAnalysis::new(source, parse(source), vec![]);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source);
    assert!(!outcome.changed());
}

#[test]
fn test_apply_non_deterministic_only_changes_nothing() {
    let source = "key: value\n";
    let diagnostics = vec![diagnostic(
        YamlDiagnosticCode::AmbiguousScalar,
        YamlCertainty::NonDeterministicFind,
        vec![YamlRepair {
            span: 5..10,
            replacement: "\"value\"".to_string(),
            explanation: "never auto-applied".to_string(),
        }],
    )];
    let analysis = YamlAnalysis::new(source, parse(source), diagnostics);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source);
    assert!(outcome.audit.applied.is_empty());
    assert!(outcome.audit.rejected.is_empty());
}
