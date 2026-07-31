//! Integration tests for OpenAPI import pipeline.

use schematic_gen::import_pipeline::{ImportOptions, run_import};

/// Returns path to a test fixture file.
fn fixture_path(name: &str) -> String {
    format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
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
        exclude_paths: vec![],
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
        exclude_paths: vec![],
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
        exclude_paths: vec![],
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
        exclude_paths: vec![],
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
        exclude_paths: vec![],
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
        exclude_paths: vec![],
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
        exclude_paths: vec![],
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
        exclude_paths: vec![],
        dry_run: true,
        strict: false,
        verbose: false,
    };

    let result = run_import(&opts);
    assert!(result.is_ok());
}
