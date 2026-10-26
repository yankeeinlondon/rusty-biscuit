//! End-to-end tests: generate code and verify it compiles.
//!
//! These tests exercise the full pipeline from API definition to compiled code.
//! They are slower than unit tests since they invoke cargo check/clippy.
//!
//! ## Test Categories
//!
//! 1. **Compilation tests** (`#[ignore]`): Verify generated code compiles with cargo
//! 2. **Structure tests**: Verify generated files exist and contain expected content
//! 3. **Response type tests**: Verify each endpoint's success type and decoder for JSON/Binary/Text/Empty responses
//! 4. **Multi-API tests**: Verify multiple APIs generate correctly together

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use schematic_define::openapi::ExportOptions;
use schematic_definitions::apis_by_module;
use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
use schematic_definitions::openai::define_openai_api;
use schematic_definitions::registry::registrations;
use schematic_gen::cargo_gen::{ModuleFeatures, write_cargo_toml};
use schematic_gen::export::openapi::write_openapi_grouped;
use schematic_definitions::naming::resolve_module_name;
use schematic_gen::output::{generate_and_write, generate_and_write_all};

/// Returns true if a module exists as either a single `.rs` file or a directory
/// with `mod.rs`.
fn module_exists(src_dir: &Path, module_name: &str) -> bool {
    let single_file = src_dir.join(format!("{}.rs", module_name));
    let dir_mod = src_dir.join(module_name).join("mod.rs");
    single_file.exists() || dir_mod.exists()
}

/// Reads module content from either a single file or concatenates all files
/// in a split directory.
fn read_module_content(src_dir: &Path, module_name: &str) -> String {
    let single_file = src_dir.join(format!("{}.rs", module_name));
    if single_file.exists() {
        return std::fs::read_to_string(&single_file)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", single_file.display(), e));
    }

    let dir = src_dir.join(module_name);
    if dir.join("mod.rs").exists() {
        let mut content = String::new();
        for file_name in &["mod.rs", "requests.rs", "responses.rs"] {
            let path = dir.join(file_name);
            if path.exists() {
                content.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
                content.push('\n');
            }
        }
        return content;
    }

    panic!(
        "Module '{}' not found as either file or directory in {}",
        module_name,
        src_dir.display()
    );
}

/// `content` without its comment lines, whose usage examples legitimately
/// mention `send` and `async fn`.
fn without_comments(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The opening of `request`'s `EndpointSpec` impl, whitespace removed: its
/// API's tag, success type, and decoder.
fn endpoint_spec_head(api: &str, request: &str, success: &str, decoder: &str) -> String {
    format!(
        "implschematic_runtime::EndpointSpecfor{request}{{typeApi={api}Api;typeSuccess={success};\
         typeFailure=schematic_runtime::NoDeclaredError;typeDecoder={decoder};"
    )
}

/// Asserts `request` declares `success` and `decoder` in `content`.
fn assert_endpoint_spec(content: &str, api: &str, request: &str, success: &str, decoder: &str) {
    let flat: String = content.split_whitespace().collect();
    assert!(
        flat.contains(&endpoint_spec_head(api, request, success, decoder)),
        "{request} should declare Success = {success} and Decoder = {decoder}\nGenerated code:\n{content}"
    );
}

/// Asserts the module emits no generic request method and no async send of
/// its own, and that each of `endpoint_methods` is an endpoint method
/// returning its call type, which the runtime sends when awaited.
fn assert_endpoint_methods(content: &str, endpoint_methods: &[&str]) {
    let code = without_comments(content);
    // A success enum's `Alternatives::decode_alternative` is the one async fn
    // generated code may define; it decodes a body and sends nothing.
    let sends = code.replace("async fn decode_alternative(", "");
    assert!(!sends.contains("async fn"), "No send method should be emitted:\n{code}");
    assert!(!code.contains("fn request"), "No generic request method should be emitted:\n{code}");
    let flat: String = code.split_whitespace().collect();
    for method in endpoint_methods {
        let call: String = method
            .split('_')
            .map(|word| word[..1].to_uppercase() + &word[1..])
            .collect();
        assert!(
            flat.contains(&format!("pubfn{method}(&self")) && flat.contains(&format!("->{call}Call<'_>{{{call}Call(self.0.send(")),
            "`{method}` should be an endpoint method returning `{call}Call`:\n{code}"
        );
    }
}

/// Returns the absolute path to the schematic workspace directory.
///
/// This is needed for e2e tests that compile generated code, so the Cargo.toml
/// can reference the schematic-define and schematic-definitions packages with
/// absolute paths.
fn schematic_workspace_root() -> &'static str {
    // The gen crate's manifest directory is `schematic/gen`; its parent is the
    // `schematic` package area the generated Cargo.toml points at.
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    let workspace = manifest_dir.parent().unwrap();
    // Leak the string to get a 'static lifetime (acceptable for tests)
    Box::leak(workspace.to_str().unwrap().to_string().into_boxed_str())
}

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
    let features = ModuleFeatures::new(&[&api], registrations());
    write_cargo_toml(&schema_dir, false, Some(schematic_workspace_root()), &features)
        .expect("Failed to write Cargo.toml");

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
    let features = ModuleFeatures::new(&[&api], registrations());
    write_cargo_toml(&schema_dir, false, Some(schematic_workspace_root()), &features)
        .expect("Failed to write Cargo.toml");

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
    let features = ModuleFeatures::new(&[&api], registrations());
    write_cargo_toml(&schema_dir, false, Some(schematic_workspace_root()), &features)
        .expect("Failed to write Cargo.toml");

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
        module_exists(&src_dir, "openai"),
        "openai module should exist"
    );

    // Verify Cargo.toml content
    let cargo_content =
        std::fs::read_to_string(schema_dir.join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_content.contains("schematic-schema"));
    assert!(cargo_content.contains("edition = \"2024\""));
    assert!(cargo_content.contains("schematic-runtime"));
    assert!(!cargo_content.contains("reqwest"));
    assert!(cargo_content.contains("serde"));
    assert!(cargo_content.contains("tokio"));

    // Verify lib.rs content: module declarations and the runtime ABI check
    let lib_content =
        std::fs::read_to_string(src_dir.join("lib.rs")).expect("Failed to read lib.rs");
    assert!(lib_content.contains("//!"));
    assert!(lib_content.contains(&format!("pub const SCHEMATIC_RUNTIME_ABI: u32 = {};", schematic_gen::RUNTIME_ABI)));
    assert!(lib_content.contains("schematic_runtime::ABI == SCHEMATIC_RUNTIME_ABI"));
    assert!(lib_content.contains("pub use schematic_runtime;"));
    assert!(lib_content.contains("pub mod shared;"));
    assert!(lib_content.contains("pub mod openai;"));
    assert!(lib_content.contains("pub mod prelude;"));

    // Verify shared.rs content (request-side helpers only; errors are the runtime's)
    let shared_content =
        std::fs::read_to_string(src_dir.join("shared.rs")).expect("Failed to read shared.rs");
    assert!(shared_content.contains("//!"));
    assert!(shared_content.contains("pub fn encode_cookie_value"));
    assert!(shared_content.contains("pub struct FormFile"));
    assert!(!shared_content.contains("SchematicError"));

    // Verify the openai module's content (split across files once it grew large)
    let api_content = read_module_content(&src_dir, "openai");

    // Should have module-level documentation
    assert!(api_content.contains("//!"));
    assert!(api_content.contains("OpenAI"));

    // Should have the API's one client struct over its tag, and no alias
    assert!(api_content.contains("pub struct OpenAI(schematic_runtime::Client<OpenAIApi>);"));
    assert!(api_content.contains("pub enum OpenAIApi {}"));
    assert!(!api_content.contains("OpenAIClient"));
    assert!(!api_content.contains("pub enum OpenAIRequest"));

    // Should have request structs for all endpoints, scoped to the API
    assert!(api_content.contains("pub struct ListModelsRequest"));
    assert!(api_content.contains("pub struct RetrieveModelRequest"));
    assert!(api_content.contains("pub struct DeleteModelRequest"));
    assert!(api_content.contains("impl schematic_runtime::EndpointSpec for ListModelsRequest {"));
    assert!(api_content.contains("type Api = OpenAIApi;"));

    // Each endpoint has a method; the runtime client sends its call
    assert_endpoint_methods(&api_content, &["list_models"]);

    // Verify prelude.rs content
    let prelude_content =
        std::fs::read_to_string(src_dir.join("prelude.rs")).expect("Failed to read prelude.rs");
    assert!(prelude_content.contains("pub use crate::openai::OpenAI;"));
    assert!(prelude_content.contains("pub use schematic_runtime::{"));
    assert!(prelude_content.contains("CallOptions"));
    assert!(!prelude_content.contains("OpenAIRequest"));
    assert!(!prelude_content.contains("SchematicError"));
}

/// Tests generating code for multiple different API configurations.
#[test]
fn generate_code_for_various_api_configurations() {
    use schematic_define::{
        ApiRequest, SuccessShape, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod,
    };

    let test_cases = [
        // Simple API with no auth
        { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "SimpleApi".to_string(); fixture.description = "A simple test API".to_string(); fixture.base_url = "https://api.simple.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetRoot".to_string(); fixture.method = RestMethod::Get; fixture.path = "/".to_string(); fixture.description = "Get root".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::json_type("RootResponse")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) },
        // API with all HTTP methods
        { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "AllMethods".to_string(); fixture.description = "API with all HTTP methods".to_string(); fixture.base_url = "https://api.methods.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![
                { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Get".to_string(); fixture.method = RestMethod::Get; fixture.path = "/resource".to_string(); fixture.description = "GET".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::json_type("Response")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
                { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Post".to_string(); fixture.method = RestMethod::Post; fixture.path = "/resource".to_string(); fixture.description = "POST".to_string(); fixture.request = Some(ApiRequest::json_type("CreateRequest")); fixture.response = vec![SuccessShape::json_type("Response")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
                { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Put".to_string(); fixture.method = RestMethod::Put; fixture.path = "/resource/{id}".to_string(); fixture.description = "PUT".to_string(); fixture.request = Some(ApiRequest::json_type("UpdateRequest")); fixture.response = vec![SuccessShape::json_type("Response")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
                { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Patch".to_string(); fixture.method = RestMethod::Patch; fixture.path = "/resource/{id}".to_string(); fixture.description = "PATCH".to_string(); fixture.request = Some(ApiRequest::json_type("PatchBody")); fixture.response = vec![SuccessShape::json_type("Response")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
                { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Delete".to_string(); fixture.method = RestMethod::Delete; fixture.path = "/resource/{id}".to_string(); fixture.description = "DELETE".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::json_type("Response")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
            ]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) },
        // API with multiple path parameters
        { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "NestedPaths".to_string(); fixture.description = "API with nested path parameters".to_string(); fixture.base_url = "https://api.nested.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::BearerToken { header: None }; fixture.auth_policy = None; fixture.env_auth = vec!["NESTED_API_KEY".to_string()]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetItem".to_string(); fixture.method = RestMethod::Get; fixture.path = "/orgs/{org}/repos/{repo}/items/{item}".to_string(); fixture.description = "Get deeply nested item".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::json_type("Item")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) },
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
        let module_name = resolve_module_name(&api);
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
            content.contains(&format!("pub struct {}(schematic_runtime::Client<{}Api>);", api.name, api.name)),
            "Should contain the client struct for '{}'",
            api.name
        );
        assert_eq!(
            content.matches(&format!("type Api = {}Api;", api.name)).count(),
            api.endpoints.len(),
            "Every request of '{}' should name its API tag",
            api.name
        );
    }
}

// =============================================================================
// Response Type Tests
// =============================================================================
// These tests verify that each endpoint declares the success type and decoder
// of its success shape. Decoding every response as JSON was a gap that caused
// runtime failures.

/// Tests that Binary response endpoints decode to `bytes::Bytes`.
///
/// This test catches the bug where all endpoints used `response.json()` regardless
/// of their declared response type.
#[test]
fn binary_response_declares_bytes_success_and_binary_decoder() {
    use schematic_define::{SuccessShape, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod};

    // Note: "BinaryTest" avoids the "Api" suffix which triggers module path inference
    let api = { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "BinaryTest".to_string(); fixture.description = "API with binary response".to_string(); fixture.base_url = "https://api.binary.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetAudio".to_string(); fixture.method = RestMethod::Get; fixture.path = "/audio".to_string(); fixture.description = "Get audio file".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::binary()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content = std::fs::read_to_string(src_dir.join("binarytest.rs"))
        .expect("Failed to read binarytest.rs");

    // CRITICAL: must decode as bytes, NOT as JSON
    assert_endpoint_spec(
        &api_content,
        "BinaryTest",
        "GetAudioRequest",
        "bytes::Bytes",
        "schematic_runtime::Binary",
    );
    assert!(
        !api_content.contains("schematic_runtime::Json<"),
        "Binary-only API should not decode JSON"
    );
    assert_endpoint_methods(&api_content, &["get_audio"]);
}

/// Tests that Text response endpoints decode to `String`.
#[test]
fn text_response_declares_string_success_and_text_decoder() {
    use schematic_define::{SuccessShape, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod};

    let api = { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "TextTest".to_string(); fixture.description = "API with text response".to_string(); fixture.base_url = "https://api.text.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetPlainText".to_string(); fixture.method = RestMethod::Get; fixture.path = "/text".to_string(); fixture.description = "Get plain text".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::text()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content =
        std::fs::read_to_string(src_dir.join("texttest.rs")).expect("Failed to read texttest.rs");

    assert_endpoint_spec(
        &api_content,
        "TextTest",
        "GetPlainTextRequest",
        "String",
        "schematic_runtime::Text",
    );
    assert_endpoint_methods(&api_content, &["get_plain_text"]);
}

/// Tests that Empty response endpoints decode to `()`.
#[test]
fn empty_response_declares_unit_success_and_empty_decoder() {
    use schematic_define::{SuccessShape, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod};

    let api = { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "EmptyTest".to_string(); fixture.description = "API with empty response".to_string(); fixture.base_url = "https://api.empty.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "DeleteItem".to_string(); fixture.method = RestMethod::Delete; fixture.path = "/items/{id}".to_string(); fixture.description = "Delete an item".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::empty()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content =
        std::fs::read_to_string(src_dir.join("emptytest.rs")).expect("Failed to read emptytest.rs");

    assert_endpoint_spec(
        &api_content,
        "EmptyTest",
        "DeleteItemRequest",
        "()",
        "schematic_runtime::Empty",
    );
    assert_endpoint_methods(&api_content, &["delete_item"]);
}

/// Tests that each endpoint of a mixed API declares its own success type and
/// decoder.
#[test]
fn mixed_response_types_declare_each_endpoint_decoder() {
    use schematic_define::{SuccessShape, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod};

    let api = { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "MixedTest".to_string(); fixture.description = "API with mixed response types".to_string(); fixture.base_url = "https://api.mixed.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![
            { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetJson".to_string(); fixture.method = RestMethod::Get; fixture.path = "/json".to_string(); fixture.description = "Get JSON".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::json_type("JsonResponse")]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
            { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetBinary".to_string(); fixture.method = RestMethod::Get; fixture.path = "/binary".to_string(); fixture.description = "Get binary".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::binary()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
            { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "GetText".to_string(); fixture.method = RestMethod::Get; fixture.path = "/text".to_string(); fixture.description = "Get text".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::text()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
            { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "DeleteItem".to_string(); fixture.method = RestMethod::Delete; fixture.path = "/item".to_string(); fixture.description = "Delete item".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::empty()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
        ]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) };

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let api_content =
        std::fs::read_to_string(src_dir.join("mixedtest.rs")).expect("Failed to read mixedtest.rs");

    // Each of the four response kinds gets its own decoder
    for (request, success, decoder) in [
        ("GetJsonRequest", "JsonResponse", "schematic_runtime::Json<JsonResponse>"),
        ("GetBinaryRequest", "bytes::Bytes", "schematic_runtime::Binary"),
        ("GetTextRequest", "String", "schematic_runtime::Text"),
        ("DeleteItemRequest", "()", "schematic_runtime::Empty"),
    ] {
        assert_endpoint_spec(&api_content, "MixedTest", request, success, decoder);
    }

    assert_endpoint_methods(&api_content, &["get_json", "get_binary", "get_text", "delete_item"]);
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

    let api_content = read_module_content(&src_dir, "elevenlabs");

    // Known binary endpoints must decode as bytes, not JSON
    let binary_endpoints = [
        "CreateSpeech",
        "GetVoiceSampleAudio",
        "CreateSoundEffect",
        "GetHistoryItemAudio",
        "DownloadHistoryItems",
    ];

    for endpoint in binary_endpoints {
        assert_endpoint_spec(
            &api_content,
            "ElevenLabs",
            &format!("{endpoint}Request"),
            "bytes::Bytes",
            "schematic_runtime::Binary",
        );
    }

    // The streaming endpoints decode incrementally (R33): audio as raw byte
    // chunks, timestamps one JSON object per line.
    assert_endpoint_spec(&api_content, "ElevenLabs", "StreamSpeechRequest",
        "schematic_runtime::ByteStream", "schematic_runtime::ByteStream");
    assert_endpoint_spec(&api_content, "ElevenLabs", "StreamSpeechWithTimestampsRequest",
        "schematic_runtime::ItemStream<::schematic_definitions::elevenlabs::SpeechWithTimestampsResponse,>",
        "schematic_runtime::Ndjson<::schematic_definitions::elevenlabs::SpeechWithTimestampsResponse,schematic_runtime::UntilClose,>");

    // JSON endpoints keep the JSON decoder
    assert!(
        api_content.contains("type Decoder = schematic_runtime::Json<"),
        "ElevenLabs JSON endpoints must decode JSON"
    );

    assert_endpoint_methods(&api_content, &["create_speech", "stream_speech"]);
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
        module_exists(&src_dir, "openai"),
        "openai module should exist"
    );
    assert!(
        module_exists(&src_dir, "elevenlabs"),
        "elevenlabs module should exist"
    );

    // Verify lib.rs includes both modules
    let lib_content =
        std::fs::read_to_string(src_dir.join("lib.rs")).expect("Failed to read lib.rs");
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
    let features = ModuleFeatures::new(&apis, registrations());
    write_cargo_toml(&schema_dir, false, Some(schematic_workspace_root()), &features)
        .expect("Failed to write Cargo.toml");

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

// =============================================================================
// Legacy Pagination Metadata
// =============================================================================



/// shared.rs holds request helpers only; paging is the runtime's.
#[test]
fn shared_module_defines_no_pagination_trait() {
    let api = define_openai_api();
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");

    generate_and_write(&api, &src_dir, false).expect("Failed to generate code");

    let shared_content =
        std::fs::read_to_string(src_dir.join("shared.rs")).expect("Failed to read shared.rs");

    assert!(
        !shared_content.contains("Paginated"),
        "shared.rs must not define a pagination trait:\n{shared_content}"
    );
}

// =============================================================================
// Module-Grouped OpenAPI Export Tests (Phase 2)
// =============================================================================

/// Verifies that running grouped OpenAPI export across every module produces
/// module-named files (e.g. `ollama.json`, `emqx.json`) and never legacy
/// per-API filenames (e.g. `ollamanative.json`, `emqxbasic.json`).
#[test]
fn generated_openapi_filenames_use_module_names() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let openapi_dir = temp_dir.path();

    let grouped = apis_by_module();
    assert!(!grouped.is_empty(), "apis_by_module() should not be empty");

    for (module_name, module_apis) in grouped.iter() {
        let api_refs: Vec<&schematic_define::RestApi> = module_apis.iter().collect();
        write_openapi_grouped(
            &api_refs,
            module_name,
            &ExportOptions::new(),
            openapi_dir,
        )
        .unwrap_or_else(|e| panic!("write_openapi_grouped failed for '{}': {}", module_name, e));
    }

    // Required module-grouped files MUST exist.
    let expected = [
        "ollama.json",
        "emqx.json",
        "huggingface.json",
        "samsung_smart_tv.json",
        "unfolded_circle_core_rest.json",
    ];
    for name in expected {
        assert!(
            openapi_dir.join(name).exists(),
            "expected grouped file {} to exist",
            name,
        );
    }

    // Legacy per-API filenames MUST NOT appear.
    let legacy = [
        "ollamanative.json",
        "ollamaopenai.json",
        "emqxbasic.json",
        "emqxbearer.json",
        "huggingfacehub.json",
        "samsungsmarttv.json",
        "unfoldedcirclecorerest.json",
    ];
    for name in legacy {
        assert!(
            !openapi_dir.join(name).exists(),
            "legacy per-API filename '{}' must not be produced",
            name,
        );
    }

    // Parse ollama.json and assert paths from BOTH variants are present.
    let ollama_content = std::fs::read_to_string(openapi_dir.join("ollama.json"))
        .expect("ollama.json should be readable");
    let ollama_doc: openapiv3::OpenAPI =
        serde_json::from_str(&ollama_content).expect("ollama.json should parse as OpenAPI");
    let path_keys: std::collections::BTreeSet<&str> =
        ollama_doc.paths.paths.keys().map(String::as_str).collect();

    let ollama_module_apis = grouped.get("ollama").expect("ollama module present");
    assert!(
        ollama_module_apis.len() >= 2,
        "ollama module should contain at least 2 APIs, got {}",
        ollama_module_apis.len(),
    );
    for api in ollama_module_apis {
        let owned = api
            .endpoints
            .iter()
            .any(|e| path_keys.contains(e.path.as_str()));
        assert!(
            owned,
            "grouped ollama.json missing any endpoint owned by '{}'",
            api.name,
        );
    }
}

/// Verifies that the grouped writer refuses a module whose API references a
/// component it does not carry, naming the API, endpoint, and name, and that
/// an API without JSON bodies still exports under the module's file name
/// (no `api.name` anywhere in OpenAPI filename derivation).
#[test]
fn grouped_export_rejects_a_dangling_reference_and_names_files_by_module() {
    use schematic_define::{SuccessShape, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod};
    use schematic_gen::diagnostics::Feature;
    use schematic_gen::errors::GeneratorError;

    fn synth(name: &str) -> RestApi {
        { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = name.to_string(); fixture.description = format!("{} synthetic", name); fixture.base_url = "https://synthetic.example.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = format!("{}Endpoint", name); fixture.method = RestMethod::Get; fixture.path = format!("/{}", name.to_lowercase()); fixture.description = String::new(); fixture.request = None; fixture.response = vec![SuccessShape::empty()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = Some("synthetic_unknown_module".to_string()); fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) }
    }

    let module_name = "synthetic_unknown_module";
    let temp = TempDir::new().unwrap();

    let mut dangling = synth("FooBar");
    dangling.endpoints[0].response = vec![SuccessShape::json_vec_type("Missing")];
    let err = write_openapi_grouped(&[&dangling], module_name, &ExportOptions::new(), temp.path())
        .expect_err("a dangling reference must block the export");
    let GeneratorError::Validation { diagnostics, .. } = err else {
        panic!("expected a validation error, got {err}");
    };
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.feature, Feature::InvalidSchemaReference);
    assert_eq!(diagnostic.api, ["FooBar"]);
    assert_eq!(diagnostic.endpoint.as_deref(), Some("FooBarEndpoint"));
    assert!(diagnostic.message.contains("`Missing`"), "{}", diagnostic.message);
    assert!(
        std::fs::read_dir(temp.path()).unwrap().next().is_none(),
        "nothing is written while an error stands"
    );

    let api = synth("FooBar");
    let path = write_openapi_grouped(&[&api], module_name, &ExportOptions::new(), temp.path())
        .expect("write_openapi_grouped should succeed");
    assert_eq!(
        path.file_name().unwrap().to_str().unwrap(),
        format!("{}.json", module_name),
    );
}
