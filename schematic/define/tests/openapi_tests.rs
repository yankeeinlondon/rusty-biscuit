//! Tests for OpenAPI module (Phase 1: Foundation Types).
//!
//! These tests verify:
//! - `OpenApiSource` enum construction from file path, JSON string, YAML string
//! - `OpenApiError` has required variants: Parse, Reference, UnsupportedFeature, Validation

#![cfg(feature = "openapi")]

use schematic_define::openapi::{OpenApiError, OpenApiSource};
use std::path::PathBuf;

// ========================================
// OpenApiSource Tests
// ========================================

#[test]
fn source_from_path() {
    let source = OpenApiSource::Path(PathBuf::from("/path/to/openapi.yaml"));
    assert!(matches!(source, OpenApiSource::Path(_)));
}

#[test]
fn source_from_json_string() {
    let json = r#"{"openapi": "3.0.0", "info": {"title": "Test", "version": "1.0.0"}}"#;
    let source = OpenApiSource::Json(json.to_string());
    assert!(matches!(source, OpenApiSource::Json(_)));
}

#[test]
fn source_from_yaml_string() {
    let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
"#;
    let source = OpenApiSource::Yaml(yaml.to_string());
    assert!(matches!(source, OpenApiSource::Yaml(_)));
}

#[test]
fn source_from_bytes() {
    let bytes = b"openapi: \"3.0.0\"\ninfo:\n  title: Test\n  version: \"1.0.0\"";
    let source = OpenApiSource::Bytes(bytes.to_vec());
    assert!(matches!(source, OpenApiSource::Bytes(_)));
}

#[test]
fn source_from_document() {
    // This variant holds a pre-parsed openapiv3::OpenAPI document
    // We can't easily construct one without parsing, so we test
    // that the variant exists and can be constructed with a document
    // This will be tested more thoroughly in Phase 2 when we add parsing
    let yaml = r#"
openapi: "3.0.0"
info:
  title: Test API
  version: "1.0.0"
paths: {}
"#;
    let doc: openapiv3::OpenAPI = serde_yaml::from_str(yaml).expect("valid yaml");
    let source = OpenApiSource::Document(Box::new(doc));
    assert!(matches!(source, OpenApiSource::Document(_)));
}

#[test]
fn source_debug_impl() {
    let source = OpenApiSource::Json("{}".to_string());
    let debug = format!("{:?}", source);
    assert!(debug.contains("Json"));
}

#[test]
fn source_clone_impl() {
    let source = OpenApiSource::Yaml("openapi: 3.0.0".to_string());
    let cloned = source.clone();
    assert!(matches!(cloned, OpenApiSource::Yaml(_)));
}

// ========================================
// OpenApiError Tests
// ========================================

#[test]
fn error_parse_variant() {
    let error = OpenApiError::Parse {
        message: "Invalid JSON syntax".to_string(),
        location: None,
    };
    let msg = error.to_string();
    assert!(msg.contains("Parse error"));
    assert!(msg.contains("Invalid JSON syntax"));
}

#[test]
fn error_parse_with_location() {
    let error = OpenApiError::Parse {
        message: "Unexpected token".to_string(),
        location: Some("line 42, column 10".to_string()),
    };
    let msg = error.to_string();
    assert!(msg.contains("Parse error"));
    assert!(msg.contains("line 42"));
}

#[test]
fn error_reference_variant() {
    let error = OpenApiError::Reference {
        ref_path: "#/components/schemas/MissingType".to_string(),
        message: "Referenced schema not found".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("Reference error"));
    assert!(msg.contains("#/components/schemas/MissingType"));
}

#[test]
fn error_unsupported_feature_variant() {
    let error = OpenApiError::UnsupportedFeature {
        feature: "callbacks".to_string(),
        context: Some("POST /webhooks endpoint".to_string()),
    };
    let msg = error.to_string();
    assert!(msg.contains("Unsupported feature"));
    assert!(msg.contains("callbacks"));
}

#[test]
fn error_unsupported_feature_no_context() {
    let error = OpenApiError::UnsupportedFeature {
        feature: "links".to_string(),
        context: None,
    };
    let msg = error.to_string();
    assert!(msg.contains("Unsupported feature"));
    assert!(msg.contains("links"));
}

#[test]
fn error_validation_variant() {
    let error = OpenApiError::Validation {
        path: "/paths/~1users/get".to_string(),
        message: "Missing required response definition".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("Validation error"));
    assert!(msg.contains("/paths/~1users/get"));
}

#[test]
fn error_io_variant() {
    let error = OpenApiError::Io {
        path: PathBuf::from("/nonexistent/openapi.yaml"),
        message: "No such file or directory".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("I/O error"));
    assert!(msg.contains("/nonexistent/openapi.yaml"));
}

#[test]
fn error_debug_impl() {
    let error = OpenApiError::Parse {
        message: "test".to_string(),
        location: None,
    };
    let debug = format!("{:?}", error);
    assert!(debug.contains("Parse"));
}

#[test]
fn error_clone_impl() {
    let error = OpenApiError::Validation {
        path: "/test".to_string(),
        message: "error".to_string(),
    };
    let cloned = error.clone();
    assert!(matches!(cloned, OpenApiError::Validation { .. }));
}

#[test]
fn error_is_std_error() {
    // Verify OpenApiError implements std::error::Error
    fn assert_error<T: std::error::Error>() {}
    assert_error::<OpenApiError>();
}

// ========================================
// Convenience Constructor Tests
// ========================================

#[test]
fn source_path_convenience() {
    let source = OpenApiSource::path("/api/openapi.yaml");
    assert!(matches!(source, OpenApiSource::Path(p) if p == PathBuf::from("/api/openapi.yaml")));
}

#[test]
fn source_json_convenience() {
    let source = OpenApiSource::json(r#"{"openapi": "3.0.0"}"#);
    assert!(matches!(source, OpenApiSource::Json(s) if s.contains("openapi")));
}

#[test]
fn source_yaml_convenience() {
    let source = OpenApiSource::yaml("openapi: 3.0.0");
    assert!(matches!(source, OpenApiSource::Yaml(s) if s.contains("openapi")));
}

#[test]
fn source_bytes_convenience() {
    let source = OpenApiSource::bytes(b"openapi: 3.0.0".to_vec());
    assert!(matches!(source, OpenApiSource::Bytes(_)));
}
