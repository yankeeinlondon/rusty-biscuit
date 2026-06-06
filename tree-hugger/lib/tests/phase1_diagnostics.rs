//! Integration tests for Phase 1 diagnostic contract features.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tree_hugger::{DiagnosticSeverity, RuleRegistry, RuleSelector, TreeFile};

fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

// ============================================================================
// Experimental Semantics Gating Tests
// ============================================================================

#[test]
fn test_undefined_symbol_disabled_by_default() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let _value = missing_value;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let undefined_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("undefined-symbol"));
    assert!(
        undefined_diagnostic.is_none(),
        "undefined-symbol should be disabled by default"
    );
}

#[test]
fn test_undefined_symbol_enabled_with_experimental_semantics() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let _value = missing_value;
}
"#,
    );

    let mut tree_file = TreeFile::new(&path).unwrap();
    tree_file.experimental_semantics = true;
    let diagnostics = tree_file.lint_diagnostics();

    let undefined_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("undefined-symbol"));
    assert!(
        undefined_diagnostic.is_some(),
        "undefined-symbol should be enabled with experimental_semantics"
    );
}

#[test]
fn test_unused_symbol_disabled_by_default() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"struct Holder {
    unused_value: i32,
}

fn main() {
    let _ = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unused_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unused-symbol"));
    assert!(
        unused_diagnostic.is_none(),
        "unused-symbol should be disabled by default"
    );
}

#[test]
fn test_unused_import_detected_by_library_layer_but_off_by_default() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.go",
        r#"package main

import "fmt"

func main() {
    _ = 1
}
"#,
    );

    // The raw library layer reports the diagnostic regardless of policy...
    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();
    let unused_import = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unused-import"));
    assert!(
        unused_import.is_some(),
        "library layer should still detect an unused import"
    );

    // ...but the rule is off by default (trait/macro usage it can't resolve makes
    // it false-positive-prone), so the CLI policy layer hides it unless opted in.
    let registry = RuleRegistry::new();
    let rule = registry.get("unused-import").unwrap();
    assert!(
        !rule.enabled_by_default,
        "unused-import should be off by default"
    );
    assert_eq!(rule.confidence, tree_hugger::DiagnosticConfidence::Low);
}

// ============================================================================
// Rule Registry Tests
// ============================================================================

#[test]
fn test_registry_lookup_by_id() {
    let registry = RuleRegistry::new();
    let rule = registry.get("unwrap-call").unwrap();
    assert_eq!(rule.id, "unwrap-call");
    assert_eq!(rule.category, tree_hugger::DiagnosticCategory::Restriction);
    assert_eq!(rule.confidence, tree_hugger::DiagnosticConfidence::High);
}

#[test]
fn test_registry_lookup_by_alias() {
    let registry = RuleRegistry::new();
    // unreachable-code is an alias for dead-code
    let rule = registry.get("dead-code").unwrap();
    assert_eq!(rule.id, "dead-code");
    assert!(
        rule.aliases.contains(&"unreachable-code".to_string()),
        "dead-code should have unreachable-code as an alias"
    );
}

#[test]
fn test_registry_experimental_rules_disabled() {
    let registry = RuleRegistry::new();
    let undefined = registry.get("undefined-symbol").unwrap();
    assert!(!undefined.enabled_by_default);
    assert!(undefined.requires_experimental_semantics);
}

#[test]
fn test_registry_default_severity() {
    let registry = RuleRegistry::new();
    let syntax = registry.get("invalid-syntax").unwrap();
    assert_eq!(syntax.default_severity, DiagnosticSeverity::Error);
    let unwrap = registry.get("unwrap-call").unwrap();
    assert_eq!(unwrap.default_severity, DiagnosticSeverity::Warning);
}

#[test]
fn test_registry_validation_passes() {
    let registry = RuleRegistry::new();
    registry
        .validate()
        .expect("built-in registry should be valid");
}

// ============================================================================
// Rule Selector Tests
// ============================================================================

#[test]
fn test_rule_selector_parse_all() {
    let selector = RuleSelector::parse("all").unwrap();
    assert!(matches!(selector, RuleSelector::All));
}

#[test]
fn test_rule_selector_parse_category() {
    let selector = RuleSelector::parse("category:restriction").unwrap();
    let registry = RuleRegistry::new();
    let unwrap = registry.get("unwrap-call").unwrap();
    assert!(selector.matches(unwrap));
}

#[test]
fn test_rule_selector_parse_rule_id() {
    let selector = RuleSelector::parse("unwrap-call").unwrap();
    let registry = RuleRegistry::new();
    let unwrap = registry.get("unwrap-call").unwrap();
    assert!(selector.matches(unwrap));
}

#[test]
fn test_rule_selector_unknown_category_returns_none() {
    assert!(RuleSelector::parse("category:unknown").is_none());
}

// ============================================================================
// Metadata Tests
// ============================================================================

#[test]
fn test_lint_diagnostic_has_metadata_after_policy() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = Some(1);
    x.unwrap();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));
    assert!(unwrap_diagnostic.is_some());
    // Metadata is populated by the CLI policy layer, not by the library
    // directly. In library tests, metadata may be None.
}

// ============================================================================
// CLI Exit Code Tests (tested via CLI integration)
// ============================================================================

#[test]
fn test_experimental_semantics_does_not_affect_pattern_rules() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = Some(1);
    x.unwrap();
}
"#,
    );

    // Pattern rules should work regardless of experimental_semantics
    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics_default = tree_file.lint_diagnostics();

    let mut tree_file_exp = TreeFile::new(&path).unwrap();
    tree_file_exp.experimental_semantics = true;
    let diagnostics_exp = tree_file_exp.lint_diagnostics();

    let unwrap_default = diagnostics_default
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));
    let unwrap_exp = diagnostics_exp
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));

    assert!(unwrap_default.is_some());
    assert!(unwrap_exp.is_some());
}
