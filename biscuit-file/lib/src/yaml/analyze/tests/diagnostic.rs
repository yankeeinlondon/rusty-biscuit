//! Tests for the shared diagnostic vocabulary: serde JSON contract, enum
//! spellings, and the classification auto-apply gate.

use super::super::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlRepair};

// ===== YamlCertainty =====

#[test]
fn test_certainty_serializes_snake_case() {
    assert_eq!(
        serde_json::to_string(&YamlCertainty::Deterministic).unwrap(),
        r#""deterministic""#
    );
    assert_eq!(
        serde_json::to_string(&YamlCertainty::DeterministicFindNonDeterministicSolution).unwrap(),
        r#""deterministic_find_non_deterministic_solution""#
    );
    assert_eq!(
        serde_json::to_string(&YamlCertainty::NonDeterministicFind).unwrap(),
        r#""non_deterministic_find""#
    );
}

#[test]
fn test_certainty_deserializes_every_spelling() {
    let parsed: Vec<YamlCertainty> = serde_json::from_str(
        r#"["deterministic","deterministic_find_non_deterministic_solution","non_deterministic_find"]"#,
    )
    .unwrap();
    assert_eq!(
        parsed,
        vec![
            YamlCertainty::Deterministic,
            YamlCertainty::DeterministicFindNonDeterministicSolution,
            YamlCertainty::NonDeterministicFind,
        ]
    );
}

#[test]
fn test_certainty_auto_apply_gate() {
    // The single auto-apply filter: only Deterministic is eligible; both
    // report-only tiers are excluded.
    assert!(YamlCertainty::Deterministic.is_auto_apply_eligible());
    assert!(!YamlCertainty::DeterministicFindNonDeterministicSolution.is_auto_apply_eligible());
    assert!(!YamlCertainty::NonDeterministicFind.is_auto_apply_eligible());
}

#[test]
fn test_certainty_rejects_unknown_spelling() {
    assert!(serde_json::from_str::<YamlCertainty>(r#""maybe""#).is_err());
}

// ===== YamlDiagnosticCode =====

#[test]
fn test_every_code_serializes_to_its_dotted_spelling() {
    for code in YamlDiagnosticCode::ALL {
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, format!("\"{}\"", code.as_str()), "code {code}");
    }
}

#[test]
fn test_every_code_deserializes_from_its_spelling() {
    for code in YamlDiagnosticCode::ALL {
        let parsed: YamlDiagnosticCode =
            serde_json::from_str(&format!("\"{}\"", code.as_str())).unwrap();
        assert_eq!(parsed, code);
    }
}

#[test]
fn test_code_spellings_are_dotted_lowercase_families() {
    // The v1 contract: biscuit-file analyzer codes are `yaml.*`; schema-aware
    // layer codes are `schema.*`. Nothing else may enter the enum.
    for code in YamlDiagnosticCode::ALL {
        let spelling = code.as_str();
        assert!(
            spelling.starts_with("yaml.") || spelling.starts_with("schema."),
            "unexpected code family: {spelling}"
        );
        assert_eq!(spelling, spelling.to_lowercase());
    }
}

#[test]
fn test_code_spellings_pinned() {
    // Pin every v1 spelling verbatim so a rename is a deliberate, reviewed act.
    let spellings: Vec<&str> = YamlDiagnosticCode::ALL
        .iter()
        .map(|code| code.as_str())
        .collect();
    assert_eq!(
        spellings,
        vec![
            "yaml.parse",
            "yaml.bom",
            "yaml.line-ending",
            "yaml.trailing-whitespace",
            "yaml.final-newline",
            "yaml.whitespace",
            "yaml.reserved-indicator",
            "yaml.duplicate-key",
            "yaml.anchor-undeclared",
            "yaml.anchor-forward",
            "yaml.anchor-duplicate",
            "yaml.anchor-unused",
            "yaml.anchor-misspelled",
            "yaml.multi-document",
            "yaml.ambiguous-scalar",
            "yaml.suspicious-empty-value",
            "yaml.block-scalar-smell",
            "yaml.comment-truncation",
            "yaml.style-inconsistency",
            "yaml.similar-key",
            "schema.type-mismatch",
            "schema.key-correction",
            "schema.shape-repair",
        ]
    );
}

#[test]
fn test_code_from_str_round_trip() {
    use std::str::FromStr;
    for code in YamlDiagnosticCode::ALL {
        assert_eq!(YamlDiagnosticCode::from_str(code.as_str()), Ok(code));
        assert_eq!(code.to_string(), code.as_str());
    }
    assert!(YamlDiagnosticCode::from_str("yaml.nope").is_err());
}

// ===== YamlDiagnostic / YamlRepair JSON shape =====

#[test]
fn test_diagnostic_serializes_contract_shape() {
    let diagnostic = YamlDiagnostic {
        code: YamlDiagnosticCode::ReservedIndicator,
        span: 11..24,
        classification: YamlCertainty::Deterministic,
        message: "plain scalar begins with a reserved indicator".to_string(),
        repairs: vec![YamlRepair {
            span: 11..24,
            replacement: "\"@daily-report\"".to_string(),
            explanation: "quote the reserved-indicator scalar".to_string(),
        }],
    };

    let json: serde_json::Value = serde_json::to_value(&diagnostic).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "code": "yaml.reserved-indicator",
            "span": { "start": 11, "end": 24 },
            "classification": "deterministic",
            "message": "plain scalar begins with a reserved indicator",
            "repairs": [
                {
                    "span": { "start": 11, "end": 24 },
                    "replacement": "\"@daily-report\"",
                    "explanation": "quote the reserved-indicator scalar",
                }
            ],
        })
    );
}

#[test]
fn test_diagnostic_with_no_repairs_serializes_empty_array() {
    let diagnostic = YamlDiagnostic {
        code: YamlDiagnosticCode::DuplicateKey,
        span: 0..5,
        classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
        message: "duplicate mapping key".to_string(),
        repairs: vec![],
    };

    let json: serde_json::Value = serde_json::to_value(&diagnostic).unwrap();
    assert_eq!(json["repairs"], serde_json::json!([]));
    assert_eq!(
        json["classification"],
        serde_json::json!("deterministic_find_non_deterministic_solution")
    );
}

#[test]
fn test_diagnostic_round_trip() {
    let diagnostic = YamlDiagnostic {
        code: YamlDiagnosticCode::MultiDocument,
        span: 42..45,
        classification: YamlCertainty::NonDeterministicFind,
        message: "multiple YAML documents".to_string(),
        repairs: vec![YamlRepair {
            span: 42..43,
            replacement: String::new(),
            explanation: "candidate only; never auto-applied".to_string(),
        }],
    };

    let json = serde_json::to_string(&diagnostic).unwrap();
    let back: YamlDiagnostic = serde_json::from_str(&json).unwrap();
    assert_eq!(back, diagnostic);
}

#[test]
fn test_diagnostic_deserializes_with_missing_repairs_as_empty() {
    let diagnostic: YamlDiagnostic = serde_json::from_str(
        r#"{
            "code": "yaml.parse",
            "span": { "start": 0, "end": 1 },
            "classification": "deterministic",
            "message": "parse failure"
        }"#,
    )
    .unwrap();
    assert!(diagnostic.repairs.is_empty());
}

#[test]
fn test_multiple_diagnostics_keep_serialized_order() {
    // Ordering stability: a serialized diagnostics array preserves the
    // source order the analyzer emitted (Phase 6 envelope relies on this).
    let diagnostics = vec![
        YamlDiagnostic {
            code: YamlDiagnosticCode::TrailingWhitespace,
            span: 30..33,
            classification: YamlCertainty::Deterministic,
            message: "trailing whitespace".to_string(),
            repairs: vec![],
        },
        YamlDiagnostic {
            code: YamlDiagnosticCode::ReservedIndicator,
            span: 11..24,
            classification: YamlCertainty::Deterministic,
            message: "reserved indicator".to_string(),
            repairs: vec![],
        },
    ];

    let json: serde_json::Value = serde_json::to_value(&diagnostics).unwrap();
    let array = json.as_array().unwrap();
    assert_eq!(array[0]["code"], "yaml.trailing-whitespace");
    assert_eq!(array[1]["code"], "yaml.reserved-indicator");
}
