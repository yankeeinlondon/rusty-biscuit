//! Integration tests for semantic resolver interfaces.

use std::path::PathBuf;

use tree_hugger::resolver::{
    ProjectContext, ResolverError, ResolverMetadata, ResolverOutput, ResolverScope,
};
use tree_hugger::shared::{DiagnosticConfidence, ProgrammingLanguage};

// ============================================================================
// ProjectContext Tests
// ============================================================================

#[test]
fn test_project_context_new() {
    let ctx = ProjectContext::new("/project");
    assert_eq!(ctx.root_path, PathBuf::from("/project"));
    assert!(ctx.manifests.is_empty());
    assert!(ctx.language_configs.is_empty());
    assert!(ctx.dependency_hints.is_empty());
    assert!(ctx.generated_file_markers.is_empty());
    assert!(ctx.target_environment.is_none());
}

#[test]
fn test_project_context_with_fields() {
    let mut ctx = ProjectContext::new("/project");
    ctx.manifests = vec![PathBuf::from("/project/Cargo.toml")];
    ctx.language_configs = vec![PathBuf::from("/project/tsconfig.json")];
    ctx.dependency_hints = vec!["serde".to_string(), "tokio".to_string()];
    ctx.generated_file_markers = vec!["*.gen.rs".to_string()];
    ctx.target_environment = Some("node".to_string());

    assert_eq!(ctx.manifests.len(), 1);
    assert_eq!(ctx.language_configs.len(), 1);
    assert_eq!(ctx.dependency_hints.len(), 2);
    assert_eq!(ctx.generated_file_markers.len(), 1);
    assert_eq!(ctx.target_environment, Some("node".to_string()));
}

// ============================================================================
// ResolverMetadata Tests
// ============================================================================

#[test]
fn test_resolver_metadata_defaults() {
    let meta = ResolverMetadata {
        id: "test-resolver".to_string(),
        name: "Test Resolver".to_string(),
        version: "1.0.0".to_string(),
        supported_languages: vec![ProgrammingLanguage::Rust],
        max_confidence: DiagnosticConfidence::High,
        scope: ResolverScope::Project,
        needs_project_context: true,
    };

    assert_eq!(meta.id, "test-resolver");
    assert_eq!(meta.name, "Test Resolver");
    assert_eq!(meta.version, "1.0.0");
    assert_eq!(meta.supported_languages.len(), 1);
    assert_eq!(meta.max_confidence, DiagnosticConfidence::High);
    assert_eq!(meta.scope, ResolverScope::Project);
    assert!(meta.needs_project_context);
}

#[test]
fn test_resolver_scope_variants() {
    let scopes = [
        ResolverScope::FileLocal,
        ResolverScope::Module,
        ResolverScope::Project,
        ResolverScope::Workspace,
    ];

    // Verify each variant can be created and compared
    assert_ne!(scopes[0], scopes[1]);
    assert_ne!(scopes[1], scopes[2]);
    assert_ne!(scopes[2], scopes[3]);
    assert_eq!(scopes[0], ResolverScope::FileLocal);
}

// ============================================================================
// ResolverOutput Tests
// ============================================================================

#[test]
fn test_resolver_output_default() {
    let output = ResolverOutput::default();
    assert!(output.diagnostics.is_empty());
    assert!(output.resolved_imports.is_empty());
    assert!(output.unresolved_symbols.is_empty());
}

// ============================================================================
// ResolverError Tests
// ============================================================================

#[test]
fn test_resolver_error_not_available() {
    let err = ResolverError::NotAvailable {
        resolver_id: "rust-cargo".to_string(),
        language: ProgrammingLanguage::Rust,
    };
    assert!(err.to_string().contains("rust-cargo"));
    assert!(err.to_string().contains("Rust"));
}

#[test]
fn test_resolver_error_missing_project_context() {
    let err = ResolverError::MissingProjectContext;
    assert!(err.to_string().contains("Project context required"));
}

#[test]
fn test_resolver_error_resolution_failed() {
    let err = ResolverError::ResolutionFailed("module not found".to_string());
    assert!(err.to_string().contains("module not found"));
}

#[test]
fn test_resolver_error_version_incompatible() {
    let err = ResolverError::VersionIncompatible {
        expected: "1.0.0".to_string(),
        found: "0.5.0".to_string(),
    };
    assert!(err.to_string().contains("1.0.0"));
    assert!(err.to_string().contains("0.5.0"));
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_project_context_serialization() {
    let mut ctx = ProjectContext::new("/project");
    ctx.manifests = vec![PathBuf::from("/project/package.json")];
    ctx.dependency_hints = vec!["react".to_string()];
    ctx.target_environment = Some("browser".to_string());

    let json = serde_json::to_string(&ctx).expect("serialize");
    assert!(json.contains("/project"));
    assert!(json.contains("package.json"));
    assert!(json.contains("react"));
    assert!(json.contains("browser"));
}

#[test]
fn test_resolver_metadata_serialization() {
    let meta = ResolverMetadata {
        id: "js-resolver".to_string(),
        name: "JS Resolver".to_string(),
        version: "2.0.0".to_string(),
        supported_languages: vec![ProgrammingLanguage::JavaScript],
        max_confidence: DiagnosticConfidence::Medium,
        scope: ResolverScope::Module,
        needs_project_context: false,
    };

    let json = serde_json::to_string(&meta).expect("serialize");
    assert!(json.contains("js-resolver"));
    assert!(json.contains("JS Resolver"));
    assert!(json.contains("2.0.0"));
    assert!(json.contains("Module"));
    assert!(json.contains("Medium"));
}

#[test]
fn test_resolver_output_serialization() {
    let output = ResolverOutput {
        diagnostics: Vec::new(),
        resolved_imports: Vec::new(),
        unresolved_symbols: Vec::new(),
    };

    let json = serde_json::to_string(&output).expect("serialize");
    assert!(json.contains("diagnostics"));
    assert!(json.contains("resolved_imports"));
    assert!(json.contains("unresolved_symbols"));
}

// ============================================================================
// Confidence Level Tests
// ============================================================================

#[test]
fn test_diagnostic_confidence_ordering() {
    // Verify confidence levels exist and can be used
    let confidences = vec![
        DiagnosticConfidence::High,
        DiagnosticConfidence::Medium,
        DiagnosticConfidence::Low,
        DiagnosticConfidence::Experimental,
    ];

    // Each should serialize/deserialize correctly
    for confidence in confidences {
        let json = serde_json::to_string(&confidence).expect("serialize");
        let deserialized: DiagnosticConfidence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(confidence, deserialized);
    }
}
