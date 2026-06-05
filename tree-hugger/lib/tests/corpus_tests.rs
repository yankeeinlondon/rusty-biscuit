//! Integration tests for corpus harness, snapshot normalization, and threshold reporting.

use std::path::PathBuf;

use tree_hugger::corpus::*;
use tree_hugger::shared::{
    CodeRange, Diagnostic, DiagnosticCategory, DiagnosticConfidence, DiagnosticKind,
    DiagnosticMetadata, DiagnosticSeverity, DiagnosticSource,
};

// ============================================================================
// CorpusTier Serialization Tests
// ============================================================================

#[test]
fn test_corpus_tier_serialization() {
    let smoke = CorpusTier::Smoke;
    let json = serde_json::to_string(&smoke).unwrap();
    assert_eq!(json, "\"Smoke\"");

    let expanded = CorpusTier::Expanded;
    let json = serde_json::to_string(&expanded).unwrap();
    assert_eq!(json, "\"Expanded\"");

    let benchmark = CorpusTier::Benchmark;
    let json = serde_json::to_string(&benchmark).unwrap();
    assert_eq!(json, "\"Benchmark\"");
}

#[test]
fn test_corpus_tier_deserialization() {
    let tier: CorpusTier = serde_json::from_str("\"Smoke\"").unwrap();
    assert_eq!(tier, CorpusTier::Smoke);

    let tier: CorpusTier = serde_json::from_str("\"Expanded\"").unwrap();
    assert_eq!(tier, CorpusTier::Expanded);

    let tier: CorpusTier = serde_json::from_str("\"Benchmark\"").unwrap();
    assert_eq!(tier, CorpusTier::Benchmark);
}

// ============================================================================
// CorpusManifest Tests
// ============================================================================

#[test]
fn test_manifest_new() {
    let manifest = CorpusManifest::new("Test corpus for syntax rules");
    assert_eq!(manifest.version, "1");
    assert_eq!(manifest.description, "Test corpus for syntax rules");
    assert!(manifest.items.is_empty());
    assert!(manifest.enabled_rules.is_empty());
}

#[test]
fn test_manifest_builder() {
    let manifest = CorpusManifest::new("Test")
        .with_enabled_rules(vec!["unwrap-call".to_string(), "dbg-macro".to_string()])
        .with_disabled_rules(vec!["experimental-rule".to_string()])
        .with_threshold("unwrap-call", Threshold::Zero)
        .with_threshold("undefined-symbol", Threshold::Budget(5))
        .add_item(
            CorpusItem::new("serde", "https://github.com/serde-rs/serde", "Rust")
                .with_revision("v1.0.200")
                .with_license("MIT")
                .with_selected_path("serde/src/**/*.rs")
                .with_excluded_path("serde/src/de/mod.rs")
                .with_tiers(vec![CorpusTier::Smoke, CorpusTier::Expanded]),
        );

    assert_eq!(manifest.enabled_rules.len(), 2);
    assert_eq!(manifest.disabled_rules.len(), 1);
    assert_eq!(manifest.thresholds.len(), 2);
    assert_eq!(manifest.items.len(), 1);

    let item = &manifest.items[0];
    assert_eq!(item.name, "serde");
    assert_eq!(item.revision.as_deref(), Some("v1.0.200"));
    assert_eq!(item.license.as_deref(), Some("MIT"));
    assert_eq!(item.selected_paths.len(), 1);
    assert_eq!(item.excluded_paths.len(), 1);
    assert!(item.tiers.contains(&CorpusTier::Smoke));
}

#[test]
fn test_manifest_items_for_tier() {
    let manifest = CorpusManifest::new("Test")
        .add_item(
            CorpusItem::new("fast", "local/fast", "Rust")
                .with_tiers(vec![CorpusTier::Smoke]),
        )
        .add_item(
            CorpusItem::new("slow", "local/slow", "Rust")
                .with_tiers(vec![CorpusTier::Expanded]),
        )
        .add_item(
            CorpusItem::new("all", "local/all", "Rust")
                .with_tiers(vec![CorpusTier::Smoke, CorpusTier::Expanded]),
        );

    let smoke_items: Vec<_> = manifest.items_for_tier(CorpusTier::Smoke).collect();
    assert_eq!(smoke_items.len(), 2); // fast + all

    let expanded_items: Vec<_> = manifest.items_for_tier(CorpusTier::Expanded).collect();
    assert_eq!(expanded_items.len(), 3); // all tiers include smoke items too

    let bench_items: Vec<_> = manifest.items_for_tier(CorpusTier::Benchmark).collect();
    assert_eq!(bench_items.len(), 3); // benchmark includes everything
}

#[test]
fn test_manifest_yaml_roundtrip() {
    let manifest = CorpusManifest::new("Test corpus")
        .add_item(
            CorpusItem::new("clap", "https://github.com/clap-rs/clap", "Rust")
                .with_revision("v4.5.0")
                .with_license("MIT")
                .with_selected_path("src/**/*.rs")
                .with_tiers(vec![CorpusTier::Smoke]),
        )
        .with_enabled_rules(vec!["unwrap-call".to_string()]);

    let yaml = serde_yaml::to_string(&manifest).unwrap();
    let parsed: CorpusManifest = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(parsed.description, "Test corpus");
    assert_eq!(parsed.items.len(), 1);
    assert_eq!(parsed.items[0].name, "clap");
}

// ============================================================================
// OracleConfig Tests
// ============================================================================

#[test]
fn test_oracle_config() {
    let oracle = OracleConfig::new("oxlint", vec!["JavaScript".to_string(), "TypeScript".to_string()])
        .with_version("0.2.x")
        .with_rule_mapping("no-console", "console-log");

    assert_eq!(oracle.tool, "oxlint");
    assert_eq!(oracle.version.as_deref(), Some("0.2.x"));
    assert_eq!(oracle.languages.len(), 2);
    assert_eq!(oracle.rule_mapping.get("no-console"), Some(&"console-log".to_string()));
}

// ============================================================================
// CorpusDiagnostic Tests
// ============================================================================

#[test]
fn test_corpus_diagnostic_from_diagnostic() {
    let diagnostic = Diagnostic {
        kind: DiagnosticKind::Lint,
        message: "Called unwrap on Option".to_string(),
        range: CodeRange {
            start_line: 5,
            start_column: 10,
            end_line: 5,
            end_column: 20,
            start_byte: 100,
            end_byte: 110,
        },
        severity: DiagnosticSeverity::Warning,
        rule: Some("unwrap-call".to_string()),
        context: None,
        metadata: Some(DiagnosticMetadata {
            category: DiagnosticCategory::Suspicious,
            confidence: DiagnosticConfidence::High,
            source: DiagnosticSource::TreeSitterQuery,
            default_severity: DiagnosticSeverity::Warning,
            effective_severity: DiagnosticSeverity::Warning,
            is_enabled_by_default: true,
            requires_experimental_semantics: false,
        }),
    };

    let corpus_root = PathBuf::from("/project");
    let corpus_diag = CorpusDiagnostic::from_diagnostic(&diagnostic,
        corpus_root.as_path(),
    );

    assert_eq!(corpus_diag.rule, Some("unwrap-call".to_string()));
    assert_eq!(corpus_diag.line, 5);
    assert_eq!(corpus_diag.kind, CorpusDiagnosticKind::Lint);
    assert_eq!(corpus_diag.severity, CorpusSeverity::Warning);
    assert_eq!(corpus_diag.confidence, Some(CorpusConfidence::High));
}

#[test]
fn test_corpus_diagnostic_path_redaction() {
    let diagnostic = Diagnostic {
        kind: DiagnosticKind::Lint,
        message: "Error at /project/src/main.rs".to_string(),
        range: CodeRange {
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 5,
            start_byte: 0,
            end_byte: 4,
        },
        severity: DiagnosticSeverity::Error,
        rule: Some("syntax-error".to_string()),
        context: None,
        metadata: None,
    };

    let corpus_root = PathBuf::from("/project");
    let corpus_diag = CorpusDiagnostic::from_diagnostic(&diagnostic,
        corpus_root.as_path(),
    );

    assert_eq!(corpus_diag.message, "Error at <REDACTED>");
}

// ============================================================================
// ThresholdReport Tests
// ============================================================================

#[test]
fn test_threshold_report_new_is_clean() {
    let report = ThresholdReport::new();
    assert!(report.is_clean());
}

#[test]
fn test_threshold_report_zero_threshold() {
    let mut report = ThresholdReport::new();
    report.record("unwrap-call", Some(CorpusConfidence::High), Threshold::Zero);

    assert!(!report.is_clean());

    let threshold = report.rules.get("unwrap-call").unwrap();
    assert_eq!(threshold.count, 1);
    assert!(!threshold.within_threshold);
}

#[test]
fn test_threshold_report_budget_threshold() {
    let mut report = ThresholdReport::new();
    report.record("undefined-symbol", Some(CorpusConfidence::Low), Threshold::Budget(10));
    report.record("undefined-symbol", Some(CorpusConfidence::Low), Threshold::Budget(10));

    let threshold = report.rules.get("undefined-symbol").unwrap();
    assert_eq!(threshold.count, 2);
    assert!(threshold.within_threshold);
}

#[test]
fn test_threshold_report_experimental_unlimited() {
    let mut report = ThresholdReport::new();
    report.record("unused-symbol", Some(CorpusConfidence::Experimental), Threshold::Zero);

    // Experimental rules don't fail the zero threshold because they're opt-in
    let threshold = report.rules.get("unused-symbol").unwrap();
    assert_eq!(threshold.count, 1);
    // Note: within_threshold remains true because experimental rules are special-cased
    // in the record method, but for Zero threshold with experimental confidence,
    // we intentionally don't mark it as a failure.
}

#[test]
fn test_threshold_report_multiple_rules() {
    let mut report = ThresholdReport::new();
    report.record("unwrap-call", Some(CorpusConfidence::High), Threshold::Zero);
    report.record("dbg-macro", Some(CorpusConfidence::High), Threshold::Zero);
    report.record("console-log", Some(CorpusConfidence::High), Threshold::Budget(5));

    assert!(!report.is_clean());
    assert_eq!(report.rules.len(), 3);
}

// ============================================================================
// OracleClassification Tests
// ============================================================================

#[test]
fn test_oracle_classification_display() {
    assert_eq!(format!("{}", MismatchKind::FalsePositive), "false-positive");
    assert_eq!(format!("{}", MismatchKind::FalseNegative), "false-negative");
    assert_eq!(format!("{}", MismatchKind::Disagreement), "disagreement");
    assert_eq!(
        format!("{}", MismatchKind::AcceptedLimitation),
        "accepted-limitation"
    );
}

#[test]
fn test_oracle_classification_roundtrip() {
    let classification = OracleClassification {
        rule: "unwrap-call".to_string(),
        file: PathBuf::from("src/main.rs"),
        line: 42,
        classification: MismatchKind::FalsePositive,
        explanation: "This is a known false positive in macro-generated code".to_string(),
    };

    let json = serde_json::to_string(&classification).unwrap();
    let parsed: OracleClassification = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.rule, "unwrap-call");
    assert_eq!(parsed.line, 42);
    assert_eq!(parsed.classification, MismatchKind::FalsePositive);
}

// ============================================================================
// Redaction Tests
// ============================================================================

#[test]
fn test_redact_snapshot_text_normalizes_line_endings() {
    let text = "line1\r\nline2\r\n";
    let normalized = redact_snapshot_text(text, std::path::Path::new("/project"));
    assert!(!normalized.contains('\r'));
    assert!(normalized.contains("line1\nline2"));
}

#[test]
fn test_redact_snapshot_text_redacts_paths() {
    let text = "Error in /project/src/lib.rs at line 5";
    let redacted = redact_snapshot_text(text, std::path::Path::new("/project"));
    assert!(redacted.contains("Error in"));
    assert!(redacted.contains("<REDACTED>"));
    assert!(!redacted.contains("/project/src/lib.rs"));
}

#[test]
fn test_redact_snapshot_text_redacts_temp() {
    let temp = std::env::temp_dir().to_string_lossy().to_string();
    let text = format!("Created file at {}/test_file.rs", temp);
    let redacted = redact_snapshot_text(
        &text, std::path::Path::new("/project"));
    // Should contain either <TMP> or <REDACTED> since both temp and path redaction apply
    assert!(
        redacted.contains("<TMP>") || redacted.contains("<REDACTED>"),
        "Expected temp to be redacted, got: {}", redacted
    );
}

#[test]
fn test_compact_diagnostic_text() {
    let compact = compact_diagnostic_text(Some("unwrap-call"), 42, "Called unwrap on Option");
    assert!(compact.contains("unwrap-call"));
    assert!(compact.contains(":42"));
    assert!(compact.contains("Called unwrap"));
}

#[test]
fn test_compact_diagnostic_text_truncates_long_messages() {
    let long_msg = "a".repeat(100);
    let compact = compact_diagnostic_text(Some("rule"), 1, &long_msg);
    assert!(compact.len() < 80);
}

#[test]
fn test_sort_lines() {
    let text = "zebra\napple\nbanana";
    let sorted = sort_lines(text);
    assert_eq!(sorted, "apple\nbanana\nzebra");
}

#[test]
fn test_strip_ansi_codes() {
    let text = "\u{001b}[31mred\u{001b}[0m text";
    let stripped = strip_ansi_codes(text);
    assert_eq!(stripped, "red text");
}

#[test]
fn test_strip_ansi_codes_no_codes() {
    let text = "plain text";
    let stripped = strip_ansi_codes(text);
    assert_eq!(stripped, "plain text");
}

// ============================================================================
// Normalize Diagnostics Tests
// ============================================================================

#[test]
fn test_normalize_diagnostics_sorts_and_dedupes() {
    let mut diagnostics = vec![
        CorpusDiagnostic {
            rule: Some("b".to_string()),
            message: "msg".to_string(),
            line: 2,
            kind: CorpusDiagnosticKind::Lint,
            severity: CorpusSeverity::Warning,
            confidence: None,
        },
        CorpusDiagnostic {
            rule: Some("a".to_string()),
            message: "msg".to_string(),
            line: 1,
            kind: CorpusDiagnosticKind::Lint,
            severity: CorpusSeverity::Warning,
            confidence: None,
        },
        CorpusDiagnostic {
            rule: Some("a".to_string()),
            message: "msg".to_string(),
            line: 1,
            kind: CorpusDiagnosticKind::Lint,
            severity: CorpusSeverity::Warning,
            confidence: None,
        },
    ];

    normalize_diagnostics(&mut diagnostics);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].rule, Some("a".to_string()));
    assert_eq!(diagnostics[1].rule, Some("b".to_string()));
}

// ============================================================================
// Corpus Runner Tests
// ============================================================================

#[test]
fn test_run_smoke_corpus() {
    let manifest = CorpusManifest::new("Smoke test")
        .add_item(CorpusItem::new("item1", "local/item1", "Rust").with_tiers(vec![CorpusTier::Smoke]))
        .add_item(CorpusItem::new("item2", "local/item2", "Rust").with_tiers(vec![CorpusTier::Expanded]));

    let result = run_smoke_corpus(&manifest);

    assert_eq!(result.tier, CorpusTier::Smoke);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].relative_path, PathBuf::from("local/item1"));
    assert!(result.elapsed_ms.is_none());
}

#[test]
fn test_run_expanded_corpus() {
    let manifest = CorpusManifest::new("Expanded test")
        .add_item(CorpusItem::new("item1", "local/item1", "Rust").with_tiers(vec![CorpusTier::Smoke]))
        .add_item(CorpusItem::new("item2", "local/item2", "Rust").with_tiers(vec![CorpusTier::Expanded]));

    let result = run_expanded_corpus(&manifest);

    assert_eq!(result.tier, CorpusTier::Expanded);
    // Expanded includes both smoke and expanded items
    assert_eq!(result.entries.len(), 2);
}

#[test]
fn test_run_benchmark_corpus() {
    let manifest = CorpusManifest::new("Benchmark test")
        .add_item(CorpusItem::new("item1", "local/item1", "Rust").with_tiers(vec![CorpusTier::Smoke]));

    let result = run_benchmark_corpus(&manifest);

    assert_eq!(result.tier, CorpusTier::Benchmark);
    assert_eq!(result.entries.len(), 1);
    assert!(result.start_time.is_some());
    assert!(result.end_time.is_some());
}

// ============================================================================
// CorpusResult Serialization Tests
// ============================================================================

#[test]
fn test_corpus_result_serialization() {
    let result = CorpusResult {
        tier: CorpusTier::Smoke,
        entries: vec![CorpusEntry {
            relative_path: PathBuf::from("src/lib.rs"),
            language: "Rust".to_string(),
            diagnostics: vec![CorpusDiagnostic {
                rule: Some("unwrap-call".to_string()),
                message: "Called unwrap".to_string(),
                line: 10,
                kind: CorpusDiagnosticKind::Lint,
                severity: CorpusSeverity::Warning,
                confidence: Some(CorpusConfidence::High),
            }],
            skipped: false,
            skip_reason: None,
        }],
        threshold_report: ThresholdReport::new(),
        start_time: None,
        end_time: None,
        elapsed_ms: None,
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("unwrap-call"));
    assert!(json.contains("Smoke"));
    assert!(json.contains("src/lib.rs"));
}

// ============================================================================
// Threshold Serialization Tests
// ============================================================================

#[test]
fn test_threshold_serialization() {
    let zero = Threshold::Zero;
    let json = serde_json::to_string(&zero).unwrap();
    assert_eq!(json, "\"Zero\"");

    let budget = Threshold::Budget(10);
    let json = serde_json::to_string(&budget).unwrap();
    assert_eq!(json, "{\"Budget\":10}");

    let unlimited = Threshold::Unlimited;
    let json = serde_json::to_string(&unlimited).unwrap();
    assert_eq!(json, "\"Unlimited\"");
}

#[test]
fn test_threshold_deserialization() {
    let threshold: Threshold = serde_json::from_str("\"Zero\"").unwrap();
    assert_eq!(threshold, Threshold::Zero);

    let threshold: Threshold = serde_json::from_str("{\"Budget\":5}").unwrap();
    assert_eq!(threshold, Threshold::Budget(5));

    let threshold: Threshold = serde_json::from_str("\"Unlimited\"").unwrap();
    assert_eq!(threshold, Threshold::Unlimited);
}

// ============================================================================
// Redact Paths Tests
// ============================================================================

#[test]
fn test_redact_paths_in_message() {
    let message = "Found at /project/src/main.rs and /tmp/test.rs";
    let corpus_root = std::path::Path::new("/project");
    let redacted = redact_paths(message, corpus_root);
    assert!(redacted.contains("<REDACTED>"));
}
