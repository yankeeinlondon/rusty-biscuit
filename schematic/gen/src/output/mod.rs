//! Output assembly and file writing for generated code.
//!
//! This module handles the final phase of code generation: assembling all generated
//! pieces into a complete Rust file, validating the output, formatting it, and
//! writing it to disk atomically.
//!
//! ## Output Structure
//!
//! The generator produces per-API module files:
//! ```text
//! schema/src/
//! ├── lib.rs         # Module declarations and re-exports
//! ├── openai.rs      # OpenAI API client code
//! └── prelude.rs     # Common re-exports for consumers
//! ```
//!
//! ## Safety Guarantees
//!
//! - **Validation**: All generated code is validated with `syn` before writing
//! - **Formatting**: Output is formatted with `prettyplease` for consistent style
//! - **Atomic writes**: Uses temp file + rename pattern to prevent partial writes

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use schematic_define::RestApi;

use crate::errors::GeneratorError;

pub mod assemble;
pub mod format;
pub mod options;
pub mod ws_modules;
pub mod write;

pub use assemble::{
    assemble_api_code, assemble_api_module, assemble_api_module_with_options,
    assemble_combined_api_module, assemble_lib_rs, assemble_lib_rs_with_options,
    assemble_prelude, assemble_prelude_with_options, assemble_shared_module, get_module_path,
    get_request_suffix,
};
pub use format::{format_code, validate_code};
pub use options::OutputOptions;
pub use write::write_atomic;

/// Generates and writes all API code to the output directory.
///
/// This is the main entry point for code generation. It produces:
/// - `lib.rs` - Module declarations and crate documentation
/// - `prelude.rs` - Convenient re-exports
/// - `{api_name}.rs` - Per-API module files
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
///
/// ## Returns
///
/// The formatted API module code (useful for dry-run mode or testing).
///
/// ## Errors
///
/// Returns an error if:
/// - Code generation produces invalid Rust
/// - File writing fails
pub fn generate_and_write(
    api: &RestApi,
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let apis = [api];
    generate_and_write_all(&apis, output_dir, dry_run)
}

/// Generates and writes code for multiple APIs to the output directory.
///
/// This function produces a complete schema crate with:
/// - `lib.rs` - Module declarations for all APIs
/// - `shared.rs` - Shared types (error type, etc.)
/// - `prelude.rs` - Re-exports from all APIs
/// - `{api_name}.rs` - One module file per API
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
///
/// ## Returns
///
/// The formatted code for the first API module (for backwards compatibility).
///
/// ## Errors
///
/// Returns an error if:
/// - Code generation produces invalid Rust
/// - File writing fails
pub fn generate_and_write_all(
    apis: &[&RestApi],
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let lib_tokens = assemble_lib_rs(apis);
    let lib_file = validate_code(&lib_tokens)?;
    let lib_formatted = format_code(&lib_file);

    let shared_tokens = assemble_shared_module();
    let shared_file = validate_code(&shared_tokens)?;
    let shared_formatted = format_code(&shared_file);

    let prelude_tokens = assemble_prelude(apis);
    let prelude_file = validate_code(&prelude_tokens)?;
    let prelude_formatted = format_code(&prelude_file);

    let mut module_groups: BTreeMap<String, Vec<&RestApi>> = BTreeMap::new();
    for api in apis {
        let path = get_module_path(api);
        module_groups.entry(path).or_default().push(api);
    }

    let mut api_modules: Vec<(String, String)> = Vec::new();
    for (module_path, group) in &module_groups {
        let tokens = if group.len() == 1 {
            assemble_api_module(group[0])
        } else {
            assemble_combined_api_module(group)
        };
        let file = validate_code(&tokens)?;
        let formatted = format_code(&file);
        let filename = format!("{}.rs", module_path);
        api_modules.push((filename, formatted));
    }
    let (ws_shared_filename, ws_shared_content) = ws_modules::generate_ws_shared_module()?;
    api_modules.push((ws_shared_filename, ws_shared_content));

    api_modules.extend(ws_modules::generate_ws_definition_modules()?);

    if dry_run {
        println!("=== lib.rs ===\n{}\n", lib_formatted);
        println!("=== shared.rs ===\n{}\n", shared_formatted);
        println!("=== prelude.rs ===\n{}\n", prelude_formatted);
        for (filename, content) in &api_modules {
            println!("=== {} ===\n{}\n", filename, content);
        }
    } else {
        write_atomic(&output_dir.join("lib.rs"), &lib_formatted)?;

        write_atomic(&output_dir.join("shared.rs"), &shared_formatted)?;

        write_atomic(&output_dir.join("prelude.rs"), &prelude_formatted)?;

        for (filename, content) in &api_modules {
            write_atomic(&output_dir.join(filename), content)?;
        }

        let expected_files: HashSet<String> = {
            let mut files = HashSet::new();
            files.insert("lib.rs".to_string());
            files.insert("shared.rs".to_string());
            files.insert("prelude.rs".to_string());
            for (filename, _) in &api_modules {
                files.insert(filename.clone());
            }
            files
        };

        if let Ok(entries) = fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "rs")
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && !expected_files.contains(name)
                {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    Ok(api_modules
        .into_iter()
        .next()
        .map(|(_, content)| content)
        .unwrap_or_default())
}

/// Generates and writes standalone API code (for imported OpenAPI specs).
///
/// Unlike `generate_and_write`, this function:
/// - Does NOT include `pub use schematic_definitions::...` imports
/// - Uses `pub use crate::types::*` to reference locally generated types
/// - Adds `pub mod types;` to lib.rs if types were generated
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
/// * `has_types` - If true, include `pub mod types;` in lib.rs
///
/// ## Returns
///
/// The formatted API module code.
///
/// ## Errors
///
/// Returns an error if:
/// - Code generation produces invalid Rust
/// - File writing fails
pub fn generate_and_write_standalone(
    api: &RestApi,
    output_dir: &Path,
    dry_run: bool,
    has_types: bool,
) -> Result<String, GeneratorError> {
    let apis = [api];
    let options = OutputOptions {
        standalone: true,
        include_types_module: has_types,
    };

    let lib_tokens = assemble_lib_rs_with_options(&apis, &options);
    let lib_file = validate_code(&lib_tokens)?;
    let lib_formatted = format_code(&lib_file);

    let shared_tokens = assemble_shared_module();
    let shared_file = validate_code(&shared_tokens)?;
    let shared_formatted = format_code(&shared_file);

    let prelude_tokens = assemble_prelude_with_options(&apis, false);
    let prelude_file = validate_code(&prelude_tokens)?;
    let prelude_formatted = format_code(&prelude_file);

    let api_tokens = assemble_api_module_with_options(api, &options);
    let api_file = validate_code(&api_tokens)?;
    let api_formatted = format_code(&api_file);
    let api_filename = format!("{}.rs", get_module_path(api));

    if dry_run {
        println!("=== lib.rs ===\n{}\n", lib_formatted);
        println!("=== shared.rs ===\n{}\n", shared_formatted);
        println!("=== prelude.rs ===\n{}\n", prelude_formatted);
        println!("=== {} ===\n{}\n", api_filename, api_formatted);
    } else {
        write_atomic(&output_dir.join("lib.rs"), &lib_formatted)?;

        write_atomic(&output_dir.join("shared.rs"), &shared_formatted)?;

        write_atomic(&output_dir.join("prelude.rs"), &prelude_formatted)?;

        write_atomic(&output_dir.join(&api_filename), &api_formatted)?;
    }

    Ok(api_formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestMethod};
    use std::fs;
    use tempfile::TempDir;

    fn make_simple_api() -> RestApi {
        RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List all items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
        }
    }

    fn make_complex_api() -> RestApi {
        RestApi {
            name: "OpenAI".to_string(),
            description: "OpenAI REST API".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            docs_url: Some("https://platform.openai.com/docs".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["OPENAI_API_KEY".to_string()],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![
                Endpoint {
                    id: "ListModels".to_string(),
                    method: RestMethod::Get,
                    path: "/models".to_string(),
                    description: "Lists available models".to_string(),
                    request: None,
                    response: ApiResponse::json_type("ListModelsResponse"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
                Endpoint {
                    id: "RetrieveModel".to_string(),
                    method: RestMethod::Get,
                    path: "/models/{model}".to_string(),
                    description: "Retrieves a model".to_string(),
                    request: None,
                    response: ApiResponse::json_type("Model"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
                Endpoint {
                    id: "CreateCompletion".to_string(),
                    method: RestMethod::Post,
                    path: "/completions".to_string(),
                    description: "Creates a completion".to_string(),
                    request: Some(ApiRequest::json_type("CreateCompletionRequest")),
                    response: ApiResponse::json_type("Completion"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
            ],
            module_path: None,
            request_suffix: None,
            version: None,
        }
    }

    #[test]
    fn assemble_api_code_produces_valid_tokenstream() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn assemble_api_code_includes_all_components() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();

        assert!(code.contains("SchematicError"));
        assert!(code.contains("ListModelsRequest"));
        assert!(code.contains("RetrieveModelRequest"));
        assert!(code.contains("CreateCompletionRequest"));
        assert!(code.contains("OpenAIRequest"));
        assert!(code.contains("struct OpenAI"));
        assert!(code.contains("async fn request"));
    }

    #[test]
    fn assemble_api_code_includes_imports() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();
        assert!(code.contains("serde"));
    }

    #[test]
    fn assemble_api_code_has_no_unnecessary_lint_allows() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();
        assert!(!code.contains("dead_code"));
        assert!(!code.contains("unused_imports"));
    }

    #[test]
    fn validate_code_accepts_valid_code() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_code_accepts_complex_api() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_code_rejects_invalid_code() {
        use quote::quote;
        let invalid_tokens = quote! {
            let x =
        };
        let result = validate_code(&invalid_tokens);
        assert!(result.is_err());
        match result {
            Err(GeneratorError::CodeGenError(_)) => {}
            Err(other) => panic!("Unexpected error type: {:?}", other),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn format_code_produces_string() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();
        let formatted = format_code(&file);
        assert!(!formatted.is_empty());
    }

    #[test]
    fn format_code_produces_readable_output() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();
        let formatted = format_code(&file);
        assert!(formatted.contains('\n'));
        assert!(formatted.contains("///") || formatted.contains("//!"));
    }

    #[test]
    fn format_code_preserves_structure() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();
        let formatted = format_code(&file);
        assert!(formatted.contains("use crate::shared::{RequestParts, SchematicError}"));
        assert!(formatted.contains("pub struct OpenAI"));
        assert!(formatted.contains("pub enum OpenAIRequest"));
    }

    #[test]
    fn write_atomic_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let content = "// Test content";
        let result = write_atomic(&file_path, content);
        assert!(result.is_ok());
        assert!(file_path.exists());
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn write_atomic_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nested/deep/test.rs");
        let content = "// Nested content";
        let result = write_atomic(&file_path, content);
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn write_atomic_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.rs");
        fs::write(&file_path, "// Old content").unwrap();
        let new_content = "// New content";
        let result = write_atomic(&file_path, new_content);
        assert!(result.is_ok());
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, new_content);
    }

    #[test]
    fn write_atomic_no_temp_file_left_behind() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("clean.rs");
        write_atomic(&file_path, "// Content").unwrap();
        let temp_path = file_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn generate_and_write_dry_run_returns_code() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), true);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("pub struct TestApi"));
    }

    #[test]
    fn generate_and_write_dry_run_no_file_created() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), true).unwrap();
        let output_path = temp_dir.path().join("lib.rs");
        assert!(!output_path.exists());
    }

    #[test]
    fn generate_and_write_creates_lib_rs() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);
        assert!(result.is_ok());
        let output_path = temp_dir.path().join("lib.rs");
        assert!(output_path.exists());
    }

    #[test]
    fn generate_and_write_file_contains_formatted_code() {
        let api = make_complex_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), false).unwrap();
        let api_module_path = temp_dir.path().join("openai.rs");
        let content = fs::read_to_string(api_module_path).unwrap();
        assert!(content.contains("    "));
        assert!(content.contains("pub struct OpenAI"));
        assert!(content.contains("pub enum OpenAIRequest"));
        assert!(content.contains("use crate::shared::{RequestParts, SchematicError}"));
        let shared_path = temp_dir.path().join("shared.rs");
        let shared_content = fs::read_to_string(shared_path).unwrap();
        assert!(shared_content.contains("pub type RequestParts"));
        assert!(shared_content.contains("pub enum SchematicError"));
        let lib_path = temp_dir.path().join("lib.rs");
        let lib_content = fs::read_to_string(lib_path).unwrap();
        assert!(lib_content.contains("pub mod shared;"));
        assert!(lib_content.contains("pub mod openai;"));
        assert!(lib_content.contains("pub mod prelude;"));
    }

    #[test]
    fn generate_and_write_returns_same_as_file_content() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let returned = generate_and_write(&api, temp_dir.path(), false).unwrap();
        let api_module_path = temp_dir.path().join("test.rs");
        let file_content = fs::read_to_string(api_module_path).unwrap();
        assert_eq!(returned, file_content);
    }

    #[test]
    fn generate_and_write_creates_nested_output_dir() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("src/generated");
        let result = generate_and_write(&api, &nested_dir, false);
        assert!(result.is_ok());
        let output_path = nested_dir.join("lib.rs");
        assert!(output_path.exists());
    }

    #[test]
    fn full_pipeline_with_all_auth_strategies() {
        let test_cases: Vec<(AuthStrategy, Vec<String>, Option<String>)> = vec![
            (AuthStrategy::None, vec![], None),
            (
                AuthStrategy::BearerToken { header: None },
                vec!["TOKEN".to_string()],
                None,
            ),
            (
                AuthStrategy::ApiKey {
                    header: "X-API-Key".to_string(),
                },
                vec!["KEY".to_string()],
                None,
            ),
            (
                AuthStrategy::Basic,
                vec!["PASS".to_string()],
                Some("USER".to_string()),
            ),
        ];

        for (auth, env_auth, env_username) in test_cases {
            let api = RestApi {
                name: "TestApi".to_string(),
                description: "Test".to_string(),
                base_url: "https://test.com".to_string(),
                docs_url: None,
                auth: auth.clone(),
                auth_policy: None,
                env_auth,
                env_username,
                env_mapping: None,
                headers: vec![],
                endpoints: vec![Endpoint {
                    id: "Test".to_string(),
                    method: RestMethod::Get,
                    path: "/test".to_string(),
                    description: "Test endpoint".to_string(),
                    request: None,
                    response: ApiResponse::json_type("TestResponse"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                }],
                module_path: None,
                request_suffix: None,
                version: None,
            };

            let temp_dir = TempDir::new().unwrap();
            let result = generate_and_write(&api, temp_dir.path(), false);
            assert!(result.is_ok(), "Failed for auth strategy: {:?}", auth);
        }
    }

    #[test]
    fn full_pipeline_with_all_http_methods() {
        let methods = [
            RestMethod::Get,
            RestMethod::Post,
            RestMethod::Put,
            RestMethod::Patch,
            RestMethod::Delete,
            RestMethod::Head,
            RestMethod::Options,
        ];

        let endpoints: Vec<Endpoint> = methods
            .iter()
            .enumerate()
            .map(|(i, method)| Endpoint {
                id: format!("Endpoint{}", i),
                method: *method,
                path: format!("/path{}", i),
                description: format!("{:?} endpoint", method),
                request: None,
                response: ApiResponse::json_type("Response"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            })
            .collect();

        let api = RestApi {
            name: "AllMethods".to_string(),
            description: "API with all HTTP methods".to_string(),
            base_url: "https://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints,
            module_path: None,
            request_suffix: None,
            version: None,
        };

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);
        assert!(result.is_ok());

        let content = result.unwrap();
        for method in methods {
            let method_str = format!("{:?}", method).to_uppercase();
            assert!(
                content.contains(&format!("\"{}\"", method_str)),
                "Missing method: {}",
                method_str
            );
        }
    }

    #[test]
    fn full_pipeline_empty_api_produces_valid_code() {
        let api = RestApi {
            name: "EmptyApi".to_string(),
            description: "API with no endpoints".to_string(),
            base_url: "https://empty.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        };

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn generated_code_has_module_documentation() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let code = generate_and_write(&api, temp_dir.path(), true).unwrap();
        assert!(code.contains("//!"));
        assert!(code.contains("Generated API client"));
        assert!(code.starts_with("// This code was automatically generated"));
        assert!(code.contains("Do not edit manually"));
    }

    fn make_shared_module_apis() -> (RestApi, RestApi) {
        let api_a = RestApi {
            name: "FooNative".to_string(),
            description: "Foo native API".to_string(),
            base_url: "http://localhost:8080".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/api/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: Some("foo".to_string()),
            request_suffix: Some("NativeRequest".to_string()),
            version: None,
        };

        let api_b = RestApi {
            name: "FooCompat".to_string(),
            description: "Foo compat API".to_string(),
            base_url: "http://localhost:8080".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/v1/items".to_string(),
                description: "List items (compat)".to_string(),
                request: None,
                response: ApiResponse::json_type("CompatListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: Some("foo".to_string()),
            request_suffix: Some("CompatRequest".to_string()),
            version: None,
        };

        (api_a, api_b)
    }

    #[test]
    fn assemble_lib_rs_deduplicates_modules() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];
        let tokens = assemble_lib_rs(&apis);
        let code = tokens.to_string();
        let count = code.matches("pub mod foo").count();
        assert_eq!(count, 1, "Expected exactly 1 'pub mod foo', found {}", count);
    }

    #[test]
    fn assemble_combined_module_includes_both_apis() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];
        let tokens = assemble_combined_api_module(&apis);
        let file = validate_code(&tokens).unwrap();
        let code = format_code(&file);
        assert!(code.contains("pub struct FooNative"), "Missing FooNative struct");
        assert!(code.contains("pub struct FooCompat"), "Missing FooCompat struct");
        assert!(code.contains("pub enum FooNativeRequest"), "Missing FooNativeRequest enum");
        assert!(code.contains("pub enum FooCompatRequest"), "Missing FooCompatRequest enum");
        let import_count = code.matches("pub use schematic_definitions::foo::*").count();
        assert_eq!(import_count, 1, "Expected exactly 1 definitions import, found {}", import_count);
        let shared_count = code.matches("use crate::shared::{RequestParts, SchematicError}").count();
        assert_eq!(shared_count, 1, "Expected exactly 1 shared import, found {}", shared_count);
    }

    #[test]
    fn generate_and_write_all_groups_shared_modules() {
        let (api_a, api_b) = make_shared_module_apis();
        let standalone = make_simple_api();
        let apis: Vec<&RestApi> = vec![&standalone, &api_a, &api_b];
        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write_all(&apis, temp_dir.path(), false);
        assert!(result.is_ok());
        assert!(temp_dir.path().join("foo.rs").exists(), "Missing foo.rs");
        assert!(!temp_dir.path().join("foonative.rs").exists(), "Unexpected foonative.rs");
        assert!(!temp_dir.path().join("foocompat.rs").exists(), "Unexpected foocompat.rs");
        let foo_content = fs::read_to_string(temp_dir.path().join("foo.rs")).unwrap();
        assert!(foo_content.contains("pub struct FooNative"));
        assert!(foo_content.contains("pub struct FooCompat"));
        assert!(temp_dir.path().join("test.rs").exists(), "Missing test.rs");
    }

    #[test]
    fn generate_and_write_all_cleans_stale_files() {
        let api = make_simple_api();
        let apis: Vec<&RestApi> = vec![&api];
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("ollamaopenai.rs"), "// stale").unwrap();
        fs::write(temp_dir.path().join("old_api.rs"), "// stale").unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "# keep").unwrap();
        generate_and_write_all(&apis, temp_dir.path(), false).unwrap();
        assert!(!temp_dir.path().join("ollamaopenai.rs").exists(), "ollamaopenai.rs should be deleted");
        assert!(!temp_dir.path().join("old_api.rs").exists(), "old_api.rs should be deleted");
        assert!(temp_dir.path().join("Cargo.toml").exists(), "Cargo.toml should be preserved");
        assert!(temp_dir.path().join("lib.rs").exists());
        assert!(temp_dir.path().join("shared.rs").exists());
        assert!(temp_dir.path().join("prelude.rs").exists());
        assert!(temp_dir.path().join("test.rs").exists());
    }

    #[test]
    fn prelude_exports_all_apis_in_shared_module() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];
        let tokens = assemble_prelude(&apis);
        let file = validate_code(&tokens).unwrap();
        let code = format_code(&file);
        assert!(code.contains("FooNative"), "Missing FooNative in prelude");
        assert!(code.contains("FooCompat"), "Missing FooCompat in prelude");
        assert!(code.contains("FooNativeRequest"), "Missing FooNativeRequest in prelude");
        assert!(code.contains("FooCompatRequest"), "Missing FooCompatRequest in prelude");
    }
}
