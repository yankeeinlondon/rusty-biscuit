//! Integration tests for OpenAPI import pipeline.

use biscuit_test_harness::manifest_dir;
use schematic_gen::import_pipeline::{ImportOptions, run_import};

/// Returns path to a test fixture file.
fn fixture_path(name: &str) -> String {
    manifest_dir!()
        .join("tests/fixtures")
        .join(name)
        .display()
        .to_string()
}

#[test]
fn import_petstore_yaml_succeeds() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    std::fs::create_dir_all(&output).unwrap();

    let opts = ImportOptions {
        input: fixture_path("petstore.yaml"),
        api_name: Some("Petstore".to_string()),
        module_path: None,
        output: output.to_string_lossy().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_ok(), "Import failed: {:?}", result.err());

    let res = result.unwrap();
    assert_eq!(res.api_name, "Petstore");
    assert_eq!(res.endpoint_count, 4);
    assert!(res.model_count > 0, "Expected models from petstore spec");
}

#[test]
fn import_petstore_generates_models_to_disk() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    std::fs::create_dir_all(&output).unwrap();

    let opts = ImportOptions {
        input: fixture_path("petstore.yaml"),
        api_name: Some("Petstore".to_string()),
        module_path: None,
        output: output.to_string_lossy().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: false,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_ok(), "Import failed: {:?}", result.err());

    // Verify types.rs was created
    let types_path = output.join("types.rs");
    assert!(types_path.exists(), "types.rs should be generated");

    let types_content = std::fs::read_to_string(&types_path).unwrap();
    assert!(types_content.contains("Pet"), "Should contain Pet struct");
    assert!(
        types_content.contains("PetStatus"),
        "Should contain PetStatus enum"
    );
}

#[test]
fn import_complex_auth_json_succeeds() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    std::fs::create_dir_all(&output).unwrap();

    let opts = ImportOptions {
        input: fixture_path("complex_auth.json"),
        api_name: Some("MultiAuth".to_string()),
        module_path: None,
        output: output.to_string_lossy().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_ok(), "Import failed: {:?}", result.err());

    let res = result.unwrap();
    assert_eq!(res.api_name, "MultiAuth");
    assert_eq!(res.endpoint_count, 3);
    assert!(
        res.model_count > 0,
        "Expected models from complex auth spec"
    );
}

#[test]
fn import_nonexistent_file_returns_error() {
    let opts = ImportOptions {
        input: "/nonexistent/path/spec.yaml".to_string(),
        api_name: None,
        module_path: None,
        output: "/tmp/out".to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_err());
}

#[test]
fn import_dry_run_does_not_write_files() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    std::fs::create_dir_all(&output).unwrap();

    let opts = ImportOptions {
        input: fixture_path("petstore.yaml"),
        api_name: Some("Petstore".to_string()),
        module_path: None,
        output: output.to_string_lossy().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_ok());

    // Dry run should NOT create types.rs on disk
    assert!(!output.join("types.rs").exists());
}

#[test]
fn import_with_verbose_includes_diagnostics() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    std::fs::create_dir_all(&output).unwrap();

    let opts = ImportOptions {
        input: fixture_path("petstore.yaml"),
        api_name: Some("Petstore".to_string()),
        module_path: None,
        output: output.to_string_lossy().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: true,
    };

    let result = run_import(&opts);
    assert!(result.is_ok());
    // Diagnostics are printed to stderr; we just verify the import succeeds
    // with verbose mode enabled
}

#[test]
fn import_with_api_name_override() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    std::fs::create_dir_all(&output).unwrap();

    let opts = ImportOptions {
        input: fixture_path("petstore.yaml"),
        api_name: Some("CustomName".to_string()),
        module_path: None,
        output: output.to_string_lossy().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_ok());

    let res = result.unwrap();
    assert_eq!(res.api_name, "CustomName");
}

#[test]
fn import_with_module_path_override() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    std::fs::create_dir_all(&output).unwrap();

    let opts = ImportOptions {
        input: fixture_path("petstore.yaml"),
        api_name: Some("Petstore".to_string()),
        module_path: Some("custom_pets".to_string()),
        output: output.to_string_lossy().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_ok());
}

#[test]
fn an_imported_definition_records_the_supplied_request_id_header() {
    for (request_id_header, expected) in [
        (Some("x-request-id"), true),
        (None, false),
    ] {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let definitions = temp_dir.path().join("petstore");
        let opts = ImportOptions {
            input: fixture_path("petstore.yaml"),
            api_name: Some("Petstore".to_string()),
            module_path: Some("petstore".to_string()),
            output: String::new(),
            definitions_out: Some(definitions.to_string_lossy().to_string()),
            overlay: None,
            exclude_paths: vec![],
            request_id_header: request_id_header.map(str::to_string),
            dry_run: false,
            strict: false,
            verbose: false,
        };

        run_import(&opts).expect("import succeeds");

        let module = std::fs::read_to_string(definitions.join("mod.rs")).expect("definition module written");
        assert_eq!(module.contains(r#".request_id_header("x-request-id")"#), expected, "{request_id_header:?}: {module}");
    }
}

#[test]
fn a_referenced_separable_union_generates_a_multi_shape_success() {
    // R28 / R1-F2.10: a `oneOf` behind a schema `$ref` reaches the generator
    // as several tagged shapes, so the success is an enum decoded by `Shapes`.
    use schematic_define::openapi::{OpenApiImport, OpenApiSource};

    let document = serde_json::json!({
        "openapi": "3.0.3",
        "info": {"title": "Fixture", "version": "1"},
        "paths": {"/reply": {"get": {"operationId": "GetReply", "responses": {"200": {
            "description": "x",
            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Reply"}}},
        }}}}},
        "components": {"schemas": {
            "Directory": {"type": "object"},
            "File": {"type": "object"},
            "Reply": {
                "oneOf": [{"$ref": "#/components/schemas/Directory"}, {"$ref": "#/components/schemas/File"}],
                "discriminator": {"propertyName": "type", "mapping": {"dir": "#/components/schemas/Directory", "file": "#/components/schemas/File"}},
            },
        }},
    });
    let imported = OpenApiImport::new(OpenApiSource::json(document.to_string())).build().unwrap();
    let api = crate::common::components::with_referenced_components(imported.api);
    let temp_dir = tempfile::TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");

    schematic_gen::output::generate_and_write(&api, &src_dir, false).unwrap();

    let module = std::fs::read_dir(&src_dir).unwrap().map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|ext| ext == "rs") && !["lib.rs", "shared.rs", "prelude.rs"].iter().any(|name| path.ends_with(name)))
        .expect("the API module");
    let flat: String = std::fs::read_to_string(&module).unwrap().split_whitespace().collect();
    assert!(flat.contains("typeSuccess=GetReplyResponse;"), "{flat}");
    assert!(flat.contains("typeDecoder=schematic_runtime::Shapes<GetReplyResponse>;"), "{flat}");
    assert!(flat.contains("Directory(Directory)") && flat.contains("File(File)"), "{flat}");
}
