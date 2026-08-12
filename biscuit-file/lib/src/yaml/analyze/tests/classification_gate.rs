//! Tests for the single auto-apply filter keyed on classification
//! (acceptance B-6). The gate is exhaustive: only
//! `YamlCertainty::Deterministic` findings may contribute repairs to
//! application, so neither report-only classification can ever reach edit
//! application — even when a diagnostic carries candidate repairs.

use super::super::{
    YamlAnalysis, YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlParseOutcome, YamlRepair,
    analyze_yaml,
};

/// Every classification variant, exhaustively. A new variant fails to
/// compile here until its auto-apply posture is decided.
const ALL_CERTAINTIES: [YamlCertainty; 3] = [
    YamlCertainty::Deterministic,
    YamlCertainty::DeterministicFindNonDeterministicSolution,
    YamlCertainty::NonDeterministicFind,
];

fn parse_outcome(source: &str) -> YamlParseOutcome {
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source) {
        Ok(value) => YamlParseOutcome::Parsed(value),
        Err(error) => YamlParseOutcome::Failed(super::super::YamlParseFailure {
            message: error.to_string(),
            location: error.location().map(Into::into),
        }),
    }
}

#[test]
fn test_gate_is_exhaustive_over_classification() {
    // Compile-time exhaustiveness: matching without a wildcard arm forces
    // every present and future variant through an explicit decision.
    for classification in ALL_CERTAINTIES {
        let eligible = match classification {
            YamlCertainty::Deterministic => true,
            YamlCertainty::DeterministicFindNonDeterministicSolution => false,
            YamlCertainty::NonDeterministicFind => false,
        };
        assert_eq!(classification.is_auto_apply_eligible(), eligible);
    }
}

#[test]
fn test_report_only_classifications_never_apply_even_with_repairs() {
    // The gate is keyed on classification alone: a report-only diagnostic
    // carrying a fully valid candidate repair must contribute nothing.
    let source = "title: @daily-report";
    let repair = YamlRepair {
        span: 7..20,
        replacement: "\"@daily-report\"".to_string(),
        explanation: "candidate that must never apply".to_string(),
    };
    for classification in ALL_CERTAINTIES {
        let diagnostics = vec![YamlDiagnostic {
            code: YamlDiagnosticCode::ReservedIndicator,
            span: 7..20,
            classification,
            message: "finding".to_string(),
            repairs: vec![repair.clone()],
        }];
        let analysis = YamlAnalysis::new(source, parse_outcome(source), diagnostics);
        let outcome = analysis.apply();
        if classification.is_auto_apply_eligible() {
            assert_eq!(outcome.source, "title: \"@daily-report\"");
            assert_eq!(outcome.audit.applied, vec![repair.clone()]);
        } else {
            assert_eq!(
                outcome.source, source,
                "{classification:?} must never reach edit application"
            );
            assert!(outcome.audit.applied.is_empty());
            assert!(outcome.audit.rejected.is_empty());
        }
    }
}

#[test]
fn test_engine_report_only_findings_never_mutate_source() {
    // End-to-end gate: every input below produces at least one report-only
    // diagnostic from the engine, and application is byte-identical.
    let inputs: &[&str] = &[
        // Duplicate keys.
        "a: 1\na: 2\n",
        "{a: 1, a: 2}\n",
        // Anchor/alias conditions (flow-position aliases: the S3
        // reserved-indicator repair is block-only, so nothing auto-applies).
        "data: {service: *defaults}\n",
        "data: {production: *defaults}\ndefaults: &defaults\n  retries: 3\n",
        "data: {service: *defualts}\ndefaults: &defaults\n  retries: 3\n",
        "a: &x 1\nb: &x 2\nc: *x\n",
        "a: &x 1\nb: 2\n",
        // Multiple documents.
        "---\nname: first\n---\nname: second\n",
        "name: first\n...\nname: second\n",
        // Schema-free lints.
        "release: 1.20\n",
        "timeout:\n",
        "script: >\n  echo first\n  echo second\n",
        "color: #fff\n",
        "token: abc #123\n",
        "a:\n  x: 1\nb:\n    y: 2\n",
        "a: true\nb: True\n",
        "development:\n  timeout: 10\nproduction:\n  timeuot: 30\n",
    ];
    for source in inputs {
        let analysis = analyze_yaml(source);
        assert!(
            !analysis.is_clean(),
            "expected report-only findings for {source:?}"
        );
        assert!(
            analysis
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.classification != YamlCertainty::Deterministic),
            "no deterministic diagnostic expected for {source:?}"
        );
        let outcome = analysis.apply();
        assert_eq!(
            outcome.source, *source,
            "report-only findings must never mutate {source:?}"
        );
        assert!(!outcome.changed());
    }
}

#[test]
fn test_mixed_document_applies_only_deterministic_repairs() {
    // The S3 reserved-indicator repair is deterministic; the empty-value
    // lint on the same document is report-only. Application must quote the
    // lexeme and leave the lint's span untouched.
    let source = "title: @x\nempty:\n";
    let analysis = analyze_yaml(source);
    let deterministic: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.classification == YamlCertainty::Deterministic)
        .collect();
    assert_eq!(deterministic.len(), 1);
    assert_eq!(deterministic[0].code, YamlDiagnosticCode::ReservedIndicator);
    assert!(analysis
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == YamlDiagnosticCode::SuspiciousEmptyValue));

    let outcome = analysis.apply();
    assert_eq!(outcome.source, "title: \"@x\"\nempty:\n");
    // The report-only lint's key span is untouched in the output.
    assert!(outcome.source.contains("\nempty:\n"));
}

#[test]
fn test_report_only_ordering_is_deterministic_across_runs() {
    let source = "a: &unused 1\nrelease: 1.20\nb: 2\nb: 3\ntimeout:\n";
    let first: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    for _ in 0..5 {
        let repeated: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
        assert_eq!(first, repeated, "findings must be deterministically ordered");
    }
}
