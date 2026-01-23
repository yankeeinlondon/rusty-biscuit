//! End-to-end tests: generate code and verify it compiles.
//!
//! These tests exercise the full pipeline from API definition to compiled code.
//! They are slower than unit tests since they invoke cargo check/clippy.
//!
//! ## Test Categories
//!
//! 1. **Compilation tests** (`#[ignore]`): Verify generated code compiles with cargo
//! 2. **Structure tests**: Verify generated files exist and contain expected content
//! 3. **Response type tests**: Verify correct methods generated for Binary/Text/Empty responses
//! 4. **Multi-API tests**: Verify multiple APIs generate correctly together

use std::process::Command;

use tempfile::TempDir;

use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
use schematic_definitions::openai::define_openai_api;
use schematic_gen::cargo_gen::write_cargo_toml;
use schematic_gen::infer_module_path;
use schematic_gen::output::{generate_and_write, generate_and_write_all};

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
    use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

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
            module_path: None,
            request_suffix: None,
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
                    request: Some(ApiRequest::json_type("CreateRequest")),
                    response: ApiResponse::json_type("Response"),
                    headers: vec![],
                },
                Endpoint {
                    id: "Put".to_string(),
                    method: RestMethod::Put,
                    path: "/resource/{id}".to_string(),
                    description: "PUT".to_string(),
                    request: Some(ApiRequest::json_type("UpdateRequest")),
                    response: ApiResponse::json_type("Response"),
                    headers: vec![],
                },
                Endpoint {
                    id: "Patch".to_string(),
                    method: RestMethod::Patch,
                    path: "/resource/{id}".to_string(),
                    description: "PATCH".to_string(),
                    request: Some(ApiRequest::json_type("PatchRequest")),
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
            module_path: None,
            request_suffix: None,
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
            module_path: None,
            request_suffix: None,
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

        // Check the API module file (e.g., simple.rs, allmethods.rs)
        // Use the same inference logic as production code
        let module_name = api
            .module_path
            .clone()
            .or_else(|| infer_module_path(&api.name))
            .unwrap_or_else(|| api.name.to_lowercase());
        let api_module_path = src_dir.join(format!("{}.rs", module_name));
        assert!(
            api_module_path.exists(),
            "{}.rs should exist for API '{}'",
            module_name,
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

// =============================================================================
// Response Type Tests
// =============================================================================
// These tests verify that the correct methods are generated for different
// ApiResponse types. This was a gap that caused runtime failures.

/// Tests that Binary response endpoints generate `request_bytes()` method.
///
/// This test catches the bug where all endpoints used `response.json()` regardless
/// of their declared response type.
#[test]
fn binary_response_generates_request_bytes_method() {
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

    // Note: "BinaryTest" avoids the "Api" suffix which triggers module path inference
    let api = RestApi {
        name: "BinaryTest".to_string(),
        description: "API with binary response".to_string(),
        base_url: "https://api.binary.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "GetAudio".to_string(),
            method: RestMethod::Get,
            path: "/audio".to_string(),
            description: "Get audio file".to_string(),
            request: None,
            response: ApiResponse::Binary,
            headers: vec![],
        }],
        module_path: None,
        request_suffix: None,
    };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content =
        std::fs::read_to_string(src_dir.join("binarytest.rs")).expect("Failed to read binarytest.rs");

    // CRITICAL: Must have request_bytes method, NOT just request<T>
    assert!(
        api_content.contains("pub async fn request_bytes"),
        "Binary API must have request_bytes() method"
    );
    assert!(
        api_content.contains("Result<bytes::Bytes, SchematicError>"),
        "request_bytes must return bytes::Bytes"
    );
    assert!(
        api_content.contains("response.bytes().await"),
        "request_bytes must call response.bytes()"
    );

    // Should NOT have generic request<T> method (no JSON endpoints)
    assert!(
        !api_content.contains("pub async fn request<T"),
        "Binary-only API should not have request<T>() method"
    );

    // Should have convenience method for the binary endpoint
    assert!(
        api_content.contains("pub async fn get_audio"),
        "Should have get_audio convenience method"
    );
}

/// Tests that Text response endpoints generate `request_text()` method.
#[test]
fn text_response_generates_request_text_method() {
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

    let api = RestApi {
        name: "TextTest".to_string(),
        description: "API with text response".to_string(),
        base_url: "https://api.text.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "GetPlainText".to_string(),
            method: RestMethod::Get,
            path: "/text".to_string(),
            description: "Get plain text".to_string(),
            request: None,
            response: ApiResponse::Text,
            headers: vec![],
        }],
        module_path: None,
        request_suffix: None,
    };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content =
        std::fs::read_to_string(src_dir.join("texttest.rs")).expect("Failed to read texttest.rs");

    assert!(
        api_content.contains("pub async fn request_text"),
        "Text API must have request_text() method"
    );
    assert!(
        api_content.contains("Result<String, SchematicError>"),
        "request_text must return String"
    );
    assert!(
        api_content.contains("response.text().await"),
        "request_text must call response.text()"
    );
}

/// Tests that Empty response endpoints generate `request_empty()` method.
#[test]
fn empty_response_generates_request_empty_method() {
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

    let api = RestApi {
        name: "EmptyTest".to_string(),
        description: "API with empty response".to_string(),
        base_url: "https://api.empty.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "DeleteItem".to_string(),
            method: RestMethod::Delete,
            path: "/items/{id}".to_string(),
            description: "Delete an item".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        }],
        module_path: None,
        request_suffix: None,
    };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content =
        std::fs::read_to_string(src_dir.join("emptytest.rs")).expect("Failed to read emptytest.rs");

    assert!(
        api_content.contains("pub async fn request_empty"),
        "Empty API must have request_empty() method"
    );
    assert!(
        api_content.contains("Result<(), SchematicError>"),
        "request_empty must return ()"
    );
}

/// Tests that mixed response types generate all appropriate methods.
#[test]
fn mixed_response_types_generate_all_methods() {
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

    let api = RestApi {
        name: "MixedTest".to_string(),
        description: "API with mixed response types".to_string(),
        base_url: "https://api.mixed.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            Endpoint {
                id: "GetJson".to_string(),
                method: RestMethod::Get,
                path: "/json".to_string(),
                description: "Get JSON".to_string(),
                request: None,
                response: ApiResponse::json_type("JsonResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "GetBinary".to_string(),
                method: RestMethod::Get,
                path: "/binary".to_string(),
                description: "Get binary".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
            },
            Endpoint {
                id: "GetText".to_string(),
                method: RestMethod::Get,
                path: "/text".to_string(),
                description: "Get text".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
            },
            Endpoint {
                id: "DeleteItem".to_string(),
                method: RestMethod::Delete,
                path: "/item".to_string(),
                description: "Delete item".to_string(),
                request: None,
                response: ApiResponse::Empty,
                headers: vec![],
            },
        ],
        module_path: None,
        request_suffix: None,
    };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content =
        std::fs::read_to_string(src_dir.join("mixedtest.rs")).expect("Failed to read mixedtest.rs");

    // Must have ALL four method types
    assert!(
        api_content.contains("pub async fn request<T"),
        "Mixed API must have request<T>() for JSON"
    );
    assert!(
        api_content.contains("pub async fn request_bytes"),
        "Mixed API must have request_bytes() for Binary"
    );
    assert!(
        api_content.contains("pub async fn request_text"),
        "Mixed API must have request_text() for Text"
    );
    assert!(
        api_content.contains("pub async fn request_empty"),
        "Mixed API must have request_empty() for Empty"
    );

    // Convenience methods for non-JSON endpoints
    assert!(
        api_content.contains("pub async fn get_binary"),
        "Should have get_binary convenience method"
    );
    assert!(
        api_content.contains("pub async fn get_text"),
        "Should have get_text convenience method"
    );
    assert!(
        api_content.contains("pub async fn delete_item"),
        "Should have delete_item convenience method"
    );
}

/// Tests ElevenLabs API generates correct binary methods for audio endpoints.
///
/// This is a regression test for the original bug - ElevenLabs has binary audio
/// endpoints that were generating JSON deserialization code.
#[test]
fn elevenlabs_binary_endpoints_generate_correctly() {
    let api = define_elevenlabs_rest_api();

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content = std::fs::read_to_string(src_dir.join("elevenlabs.rs"))
        .expect("Failed to read elevenlabs.rs");

    // Must have request_bytes for binary endpoints
    assert!(
        api_content.contains("pub async fn request_bytes"),
        "ElevenLabs must have request_bytes() method"
    );

    // Must have request<T> for JSON endpoints
    assert!(
        api_content.contains("pub async fn request<T"),
        "ElevenLabs must have request<T>() method for JSON endpoints"
    );

    // Convenience methods for known binary endpoints
    let binary_endpoints = [
        "create_speech",
        "stream_speech",
        "get_voice_sample_audio",
        "create_sound_effect",
        "get_history_item_audio",
        "download_history_items",
    ];

    for endpoint in binary_endpoints {
        assert!(
            api_content.contains(&format!("pub async fn {}", endpoint)),
            "ElevenLabs must have {} convenience method",
            endpoint
        );
    }
}

// =============================================================================
// Multi-API Generation Tests
// =============================================================================

/// Tests that multiple APIs can be generated together.
#[test]
fn multiple_apis_generate_together() {
    let openai = define_openai_api();
    let elevenlabs = define_elevenlabs_rest_api();

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    let apis: Vec<&schematic_define::RestApi> = vec![&openai, &elevenlabs];
    generate_and_write_all(&apis, &src_dir, false).expect("Failed to generate code for all APIs");

    // Verify both API modules exist
    assert!(
        src_dir.join("openai.rs").exists(),
        "openai.rs should exist"
    );
    assert!(
        src_dir.join("elevenlabs.rs").exists(),
        "elevenlabs.rs should exist"
    );

    // Verify lib.rs includes both modules
    let lib_content = std::fs::read_to_string(src_dir.join("lib.rs")).expect("Failed to read lib.rs");
    assert!(
        lib_content.contains("pub mod openai;"),
        "lib.rs should declare openai module"
    );
    assert!(
        lib_content.contains("pub mod elevenlabs;"),
        "lib.rs should declare elevenlabs module"
    );

    // Verify prelude exports both APIs
    let prelude_content =
        std::fs::read_to_string(src_dir.join("prelude.rs")).expect("Failed to read prelude.rs");
    assert!(
        prelude_content.contains("OpenAI"),
        "prelude should export OpenAI"
    );
    assert!(
        prelude_content.contains("ElevenLabs"),
        "prelude should export ElevenLabs"
    );
}

/// Tests that multi-API generation compiles successfully.
#[test]
#[ignore = "slow: compiles generated code"]
fn multiple_apis_compile() {
    let openai = define_openai_api();
    let elevenlabs = define_elevenlabs_rest_api();

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let schema_dir = temp_dir.path().join("schema");
    let src_dir = schema_dir.join("src");

    let apis: Vec<&schematic_define::RestApi> = vec![&openai, &elevenlabs];
    generate_and_write_all(&apis, &src_dir, false).expect("Failed to generate code");
    write_cargo_toml(&schema_dir, false).expect("Failed to write Cargo.toml");

    let output = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(schema_dir.join("Cargo.toml"))
        .output()
        .expect("Failed to run cargo check");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Multi-API generated code failed to compile:\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
            stdout, stderr
        );
    }
}
