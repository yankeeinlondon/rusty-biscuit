//! End-to-end tests: generate code and verify it compiles.
//!
//! These tests exercise the full pipeline from API definition to compiled code.
//! They are slower than unit tests since they invoke cargo check/clippy.

use std::process::Command;

use tempfile::TempDir;

use schematic_definitions::openai::define_openai_api;
use schematic_gen::cargo_gen::write_cargo_toml;
use schematic_gen::output::generate_and_write;

/// Tests that generated code compiles successfully.
///
/// This test:
/// 1. Creates a temporary directory structure
/// 2. Generates code from the OpenAI API definition
/// 3. Writes a Cargo.toml with required dependencies
/// 4. Runs `cargo check` to verify the generated code compiles
#[test]
#[ignore = "slow: compiles generated code"]
fn generated_code_compiles() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let schema_dir = temp_dir.path().join("schema");
    let src_dir = schema_dir.join("src");

    // Generate the code from OpenAI API definition
    let api = define_openai_api();
    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");
    write_cargo_toml(&schema_dir, false).expect("Failed to write Cargo.toml");

    // Try to compile with cargo check
    let output = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(schema_dir.join("Cargo.toml"))
        .output()
        .expect("Failed to run cargo check");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Generated code failed to compile:\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
            stdout, stderr
        );
    }
}

/// Tests that generated code has no clippy warnings.
///
/// This test:
/// 1. Creates a temporary directory structure
/// 2. Generates code from the OpenAI API definition
/// 3. Writes a Cargo.toml with required dependencies
/// 4. Runs `cargo clippy -- -D warnings` to check for lints
#[test]
#[ignore = "slow: runs clippy on generated code"]
fn generated_code_passes_clippy() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let schema_dir = temp_dir.path().join("schema");
    let src_dir = schema_dir.join("src");

    // Generate the code
    let api = define_openai_api();
    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");
    write_cargo_toml(&schema_dir, false).expect("Failed to write Cargo.toml");

    // Run cargo clippy with warnings as errors
    let output = Command::new("cargo")
        .args(["clippy", "--manifest-path"])
        .arg(schema_dir.join("Cargo.toml"))
        .args(["--", "-D", "warnings"])
        .output()
        .expect("Failed to run cargo clippy");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Generated code has clippy warnings:\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
            stdout, stderr
        );
    }
}

/// Verifies the generated files exist and have expected content.
#[test]
fn generated_files_exist_and_have_expected_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let schema_dir = temp_dir.path().join("schema");
    let src_dir = schema_dir.join("src");

    let api = define_openai_api();
    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");
    write_cargo_toml(&schema_dir, false).expect("Failed to write Cargo.toml");

    // Verify files exist
    assert!(
        schema_dir.join("Cargo.toml").exists(),
        "Cargo.toml should exist"
    );
    assert!(src_dir.join("lib.rs").exists(), "src/lib.rs should exist");
    assert!(
        src_dir.join("prelude.rs").exists(),
        "src/prelude.rs should exist"
    );
    assert!(
        src_dir.join("openai.rs").exists(),
        "src/openai.rs should exist"
    );

    // Verify Cargo.toml content
    let cargo_content =
        std::fs::read_to_string(schema_dir.join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_content.contains("schematic-schema"));
    assert!(cargo_content.contains("edition = \"2024\""));
    assert!(cargo_content.contains("reqwest"));
    assert!(cargo_content.contains("serde"));
    assert!(cargo_content.contains("tokio"));
    assert!(cargo_content.contains("schematic-define")); // For AuthStrategy and UpdateStrategy

    // Verify lib.rs content (now just module declarations)
    let lib_content =
        std::fs::read_to_string(src_dir.join("lib.rs")).expect("Failed to read lib.rs");
    assert!(lib_content.contains("//!"));
    assert!(lib_content.contains("pub mod shared;"));
    assert!(lib_content.contains("pub mod openai;"));
    assert!(lib_content.contains("pub mod prelude;"));

    // Verify shared.rs content (common error type)
    let shared_content =
        std::fs::read_to_string(src_dir.join("shared.rs")).expect("Failed to read shared.rs");
    assert!(shared_content.contains("//!"));
    assert!(shared_content.contains("pub enum SchematicError"));
    assert!(shared_content.contains("thiserror::Error"));

    // Verify openai.rs content (API module)
    let api_content =
        std::fs::read_to_string(src_dir.join("openai.rs")).expect("Failed to read openai.rs");

    // Should have module-level documentation
    assert!(api_content.contains("//!"));
    assert!(api_content.contains("OpenAI"));

    // Should have all the generated components
    assert!(api_content.contains("use crate::shared::SchematicError"));
    assert!(api_content.contains("pub struct OpenAI"));
    assert!(api_content.contains("pub enum OpenAIRequest"));

    // Should have request structs for all endpoints
    assert!(api_content.contains("pub struct ListModelsRequest"));
    assert!(api_content.contains("pub struct RetrieveModelRequest"));
    assert!(api_content.contains("pub struct DeleteModelRequest"));

    // Should have the async request method
    assert!(api_content.contains("pub async fn request"));

    // Verify prelude.rs content
    let prelude_content =
        std::fs::read_to_string(src_dir.join("prelude.rs")).expect("Failed to read prelude.rs");
    assert!(prelude_content.contains("OpenAI"));
    assert!(prelude_content.contains("OpenAIRequest"));
    assert!(prelude_content.contains("SchematicError"));
}

/// Tests generating code for multiple different API configurations.
#[test]
fn generate_code_for_various_api_configurations() {
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod, Schema};

    let test_cases = [
        // Simple API with no auth
        RestApi {
            name: "SimpleApi".to_string(),
            description: "A simple test API".to_string(),
            base_url: "https://api.simple.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "GetRoot".to_string(),
                method: RestMethod::Get,
                path: "/".to_string(),
                description: "Get root".to_string(),
                request: None,
                response: ApiResponse::json_type("RootResponse"),
                headers: vec![],
            }],
        },
        // API with all HTTP methods
        RestApi {
            name: "AllMethods".to_string(),
            description: "API with all HTTP methods".to_string(),
            base_url: "https://api.methods.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![
                Endpoint {
                    id: "Get".to_string(),
                    method: RestMethod::Get,
                    path: "/resource".to_string(),
                    description: "GET".to_string(),
                    request: None,
                    response: ApiResponse::json_type("Response"),
                    headers: vec![],
                },
                Endpoint {
                    id: "Post".to_string(),
                    method: RestMethod::Post,
                    path: "/resource".to_string(),
                    description: "POST".to_string(),
                    request: Some(Schema::new("CreateRequest")),
                    response: ApiResponse::json_type("Response"),
                    headers: vec![],
                },
                Endpoint {
                    id: "Put".to_string(),
                    method: RestMethod::Put,
                    path: "/resource/{id}".to_string(),
                    description: "PUT".to_string(),
                    request: Some(Schema::new("UpdateRequest")),
                    response: ApiResponse::json_type("Response"),
                    headers: vec![],
                },
                Endpoint {
                    id: "Patch".to_string(),
                    method: RestMethod::Patch,
                    path: "/resource/{id}".to_string(),
                    description: "PATCH".to_string(),
                    request: Some(Schema::new("PatchRequest")),
                    response: ApiResponse::json_type("Response"),
                    headers: vec![],
                },
                Endpoint {
                    id: "Delete".to_string(),
                    method: RestMethod::Delete,
                    path: "/resource/{id}".to_string(),
                    description: "DELETE".to_string(),
                    request: None,
                    response: ApiResponse::json_type("Response"),
                    headers: vec![],
                },
            ],
        },
        // API with multiple path parameters
        RestApi {
            name: "NestedPaths".to_string(),
            description: "API with nested path parameters".to_string(),
            base_url: "https://api.nested.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["NESTED_API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "GetItem".to_string(),
                method: RestMethod::Get,
                path: "/orgs/{org}/repos/{repo}/items/{item}".to_string(),
                description: "Get deeply nested item".to_string(),
                request: None,
                response: ApiResponse::json_type("Item"),
                headers: vec![],
            }],
        },
    ];

    for api in test_cases {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let src_dir = temp_dir.path().join("src");

        let result = generate_and_write(&api, &src_dir, false);
        assert!(
            result.is_ok(),
            "Failed to generate code for API '{}': {:?}",
            api.name,
            result.err()
        );

        let lib_path = src_dir.join("lib.rs");
        assert!(
            lib_path.exists(),
            "lib.rs should exist for API '{}'",
            api.name
        );

        // Check the API module file (e.g., simpleapi.rs, allmethods.rs)
        let api_module_path = src_dir.join(format!("{}.rs", api.name.to_lowercase()));
        assert!(
            api_module_path.exists(),
            "{}.rs should exist for API '{}'",
            api.name.to_lowercase(),
            api.name
        );

        let content =
            std::fs::read_to_string(&api_module_path).expect("Failed to read API module file");
        assert!(
            content.contains(&format!("pub struct {}", api.name)),
            "Should contain API struct for '{}'",
            api.name
        );
    }
}
