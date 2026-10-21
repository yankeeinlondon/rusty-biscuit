//! Tests for OpenAPI module (Phase 1: Foundation Types).
//!
//! These tests verify:
//! - `OpenApiSource` enum construction from file path, JSON string, YAML string
//! - `OpenApiError` has required variants: Parse, Reference, UnsupportedFeature, Validation

#![cfg(feature = "openapi")]

use biscuit_file::serde_yaml_ng;
use schematic_define::openapi::{OpenApiError, OpenApiSource};
use std::path::{Path, PathBuf};

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
    let doc: openapiv3::OpenAPI = serde_yaml_ng::from_str(yaml).expect("valid yaml");
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
    assert!(matches!(source, OpenApiSource::Path(p) if p == Path::new("/api/openapi.yaml")));
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

// ========================================
// Import Result Reference Assembly
// ========================================

/// The Petstore fixture the generator's import tests also use.
const PETSTORE: &str = include_str!("../../gen/tests/fixtures/petstore.yaml");

fn import(source: OpenApiSource) -> Result<schematic_define::openapi::OpenApiImportResult, OpenApiError> {
    schematic_define::openapi::OpenApiImport::new(source).build()
}

/// Petstore with one `$ref` of the create operation renamed to `MissingPet`.
fn petstore_with(original: &str, replacement: &str) -> String {
    assert_eq!(PETSTORE.matches(original).count(), 1, "fixture anchor {original:?}");
    PETSTORE.replace(original, replacement)
}

#[test]
fn every_source_form_imports_petstore_with_resolvable_exportable_components() {
    let document: openapiv3::OpenAPI = serde_yaml_ng::from_str(PETSTORE).unwrap();
    let json = serde_json::to_string(&document).unwrap();
    let path = std::env::temp_dir().join(format!("schematic-define-petstore-{}.yaml", std::process::id()));
    std::fs::write(&path, PETSTORE).unwrap();
    let sources = [
        ("yaml", OpenApiSource::yaml(PETSTORE)),
        ("json", OpenApiSource::json(json)),
        ("path", OpenApiSource::path(&path)),
        ("bytes", OpenApiSource::bytes(PETSTORE.as_bytes().to_vec())),
        ("document", OpenApiSource::Document(Box::new(document))),
    ];

    let results: Vec<_> = sources.into_iter().map(|(form, source)| (form, import(source))).collect();
    std::fs::remove_file(&path).unwrap();

    for (form, result) in results {
        let api = result.unwrap_or_else(|error| panic!("{form}: {error}")).api;
        let components: Vec<(&str, Option<&str>)> =
            api.schemas.iter().map(|c| (c.name.as_str(), c.rust_path.as_deref())).collect();
        assert_eq!(api.endpoints.len(), 4, "{form}");
        // Source order; `CreatePetRequest` yields its name to the CreatePet request struct.
        assert_eq!(
            components,
            [("Pet", Some("Pet")), ("CreatePetRequest2", Some("CreatePetRequest2")), ("PetStatus", Some("PetStatus"))],
            "{form}"
        );
        api.validate_references().unwrap_or_else(|error| panic!("{form}: {error}"));
        let document = schematic_define::openapi::export(&api, &schematic_define::openapi::ExportOptions::new())
            .unwrap_or_else(|error| panic!("{form}: {error}"));
        let pet = serde_json::to_value(&document.components.unwrap().schemas["Pet"]).unwrap();
        assert_eq!(pet.pointer("/properties/status/allOf/0/$ref"), Some(&"#/components/schemas/PetStatus".into()), "{form}");
    }
}

#[test]
fn an_import_module_path_qualifies_component_rust_paths() {
    let imported = schematic_define::openapi::OpenApiImport::new(OpenApiSource::yaml(PETSTORE))
        .module_path("petstore")
        .build()
        .unwrap();

    let pet = imported.api.component("Pet").unwrap();

    assert_eq!(pet.rust_path.as_deref(), Some("petstore::Pet"));
    assert_eq!(imported.api.rust_type(&imported.api.endpoints[1].json_references()[0].clone()).as_deref(), Some("petstore::Pet"));
}

#[test]
fn a_dangling_create_reference_fails_the_import_naming_api_endpoint_and_type() {
    let response = petstore_with(
        "The created pet\n          content:\n            application/json:\n              schema:\n                $ref: '#/components/schemas/Pet'",
        "The created pet\n          content:\n            application/json:\n              schema:\n                $ref: '#/components/schemas/MissingPet'",
    );
    let request = petstore_with(
        "$ref: '#/components/schemas/CreatePetRequest'",
        "$ref: '#/components/schemas/MissingPet'",
    );

    for (site, text) in [("response", response), ("request", request)] {
        let error = import(OpenApiSource::yaml(text)).expect_err(site);

        assert_eq!(
            error,
            OpenApiError::Definition(schematic_define::DefinitionError::UnresolvedReference {
                api: "Petstore".into(),
                endpoint: "CreatePet".into(),
                name: "MissingPet".into(),
            }),
            "{site}"
        );
        assert!(error.to_string().contains("Petstore endpoint CreatePet: unresolved schema reference \"MissingPet\""), "{site}: {error}");
    }
}

#[test]
fn a_component_free_import_still_validates_and_exports() {
    let yaml = "openapi: '3.0.3'\ninfo: {title: Plain, version: '1'}\npaths:\n  /hello:\n    get:\n      operationId: hello\n      responses:\n        '200':\n          description: Greeting\n          content:\n            application/json:\n              schema: {type: string}\n";

    let api = import(OpenApiSource::yaml(yaml)).unwrap().api;

    assert!(api.schemas.is_empty());
    assert_eq!(api.endpoints.len(), 1);
    api.validate_references().unwrap();
    schematic_define::openapi::export(&api, &schematic_define::openapi::ExportOptions::new()).unwrap();
}
