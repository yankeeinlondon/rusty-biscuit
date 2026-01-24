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

use std::fs;
use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

use crate::codegen::{
    ModuleDocBuilder, generate_api_struct, generate_error_type, generate_request_enum_with_suffix,
    generate_request_method_with_suffix, generate_request_parts_type, generate_request_struct_with_options,
};
use crate::errors::GeneratorError;
use crate::inference::infer_module_path;

/// Returns the module path for the given API.
///
/// Uses `api.module_path` if set, otherwise attempts to infer from the API name.
/// Falls back to `api.name.to_lowercase()` if inference returns None.
///
/// ## Resolution Order
///
/// 1. Explicit `module_path` (highest priority)
/// 2. Inferred from CamelCase name (e.g., "OllamaNative" -> "ollama")
/// 3. Lowercase API name (fallback)
///
/// ## Examples
///
/// ```ignore
/// // Explicit module_path takes precedence
/// let api = RestApi { name: "HuggingFaceHub".to_string(), module_path: Some("huggingface".to_string()), ... };
/// assert_eq!(get_module_path(&api), "huggingface");
///
/// // Inferred from CamelCase
/// let api = RestApi { name: "OllamaNative".to_string(), module_path: None, ... };
/// assert_eq!(get_module_path(&api), "ollama");
///
/// // Fallback to lowercase
/// let api = RestApi { name: "OpenAI".to_string(), module_path: None, ... };
/// // Inference returns "open", fallback returns "openai"
/// ```
fn get_module_path(api: &RestApi) -> String {
    api.module_path
        .clone()
        .unwrap_or_else(|| infer_module_path(&api.name).unwrap_or_else(|| api.name.to_lowercase()))
}

/// Returns the request suffix for the given API.
///
/// Uses `api.request_suffix` if set, otherwise defaults to `"Request"`.
///
/// ## Examples
///
/// ```ignore
/// let api = RestApi { request_suffix: None, ... };
/// assert_eq!(get_request_suffix(&api), "Request");
///
/// let api = RestApi { request_suffix: Some("Params".to_string()), ... };
/// assert_eq!(get_request_suffix(&api), "Params");
/// ```
fn get_request_suffix(api: &RestApi) -> String {
    api.request_suffix
        .clone()
        .unwrap_or_else(|| "Request".to_string())
}

/// Assembles the shared module code (shared.rs).
///
/// This function generates code for the shared module, containing:
/// - Module documentation
/// - Common error type used by all API clients
/// - Common type aliases (e.g., `RequestParts`)
///
/// ## Returns
///
/// A TokenStream containing the shared module code.
pub fn assemble_shared_module() -> TokenStream {
    // Generate shared types
    let request_parts_type = generate_request_parts_type();
    let error_type = generate_error_type();

    quote! {
        //! Shared types and utilities for generated API clients.

        #request_parts_type

        #error_type
    }
}

/// Assembles the API module code (e.g., openai.rs).
///
/// This function generates code for a single API module, containing:
/// - Rich module documentation (intro, auth, features, example)
/// - Import statements
/// - Re-exports from definitions
/// - Per-endpoint request structs
/// - Request enum (unifying all endpoints)
/// - API client struct
/// - Request method implementation
///
/// The error type (`SchematicError`) is imported from the shared module
/// rather than being duplicated in each API module.
///
/// ## Arguments
///
/// * `api` - The REST API definition to generate code for
///
/// ## Returns
///
/// A TokenStream containing the API module code.
pub fn assemble_api_module(api: &RestApi) -> TokenStream {
    let api_name_lower = get_module_path(api);
    let suffix = get_request_suffix(api);

    // Generate request structs for each endpoint
    let request_structs: TokenStream = api
        .endpoints
        .iter()
        .map(|ep| generate_request_struct_with_options(ep, &suffix, Some(&api_name_lower)))
        .collect();

    // Generate request enum
    let request_enum = generate_request_enum_with_suffix(api, &suffix);

    // Generate API struct
    let api_struct = generate_api_struct(api);

    // Generate request method with the appropriate suffix
    let request_method = generate_request_method_with_suffix(api, &suffix);

    // Generate rich module documentation
    let module_docs = ModuleDocBuilder::new(api).build();

    // Build the re-export path dynamically
    let definitions_module = format_ident!("{}", api_name_lower);

    // Combine all pieces with necessary imports
    quote! {
        #module_docs

        use serde::{Deserialize, Serialize};

        // Re-export response types from definitions so consumers can import from one place
        pub use schematic_definitions::#definitions_module::*;

        // Import shared types
        use crate::shared::{RequestParts, SchematicError};

        #request_structs

        #request_enum

        #api_struct

        #request_method
    }
}

/// Assembles the lib.rs content for the schema crate.
///
/// This generates the main library file that:
/// - Declares the shared module (containing common types like `SchematicError`)
/// - Declares all API modules
/// - Re-exports modules at crate root
/// - Provides a prelude module
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to include
///
/// ## Returns
///
/// A TokenStream containing the lib.rs code.
pub fn assemble_lib_rs(apis: &[&RestApi]) -> TokenStream {
    // Generate module declarations and re-exports
    let module_decls: Vec<_> = apis
        .iter()
        .map(|api| {
            let module_name = format_ident!("{}", get_module_path(api));
            quote! {
                pub mod #module_name;
            }
        })
        .collect();

    quote! {
        //! Generated REST API clients.
        //!
        //! ## Available APIs
        //!
        //! Each API is available as a separate module with its client struct,
        //! request types, and response types re-exported from definitions.
        //!
        //! ## Quick Start
        //!
        //! Use the prelude for convenient imports:
        //!
        //! ```ignore
        //! use schematic_schema::prelude::*;
        //! ```

        // Shared types and utilities
        pub mod shared;

        pub mod prelude;

        #(#module_decls)*
    }
}

/// Assembles the prelude.rs content for the schema crate.
///
/// The prelude provides convenient re-exports for consumers:
/// - All API client structs
/// - All request enums
/// - Common error type (from shared module)
/// - Response types from definitions
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to include
///
/// ## Returns
///
/// A TokenStream containing the prelude.rs code.
pub fn assemble_prelude(apis: &[&RestApi]) -> TokenStream {
    // Generate re-exports for each API (client and request enum only, not error)
    let api_reexports: Vec<_> = apis
        .iter()
        .map(|api| {
            let module_name = format_ident!("{}", get_module_path(api));
            let client_name = format_ident!("{}", api.name);
            let request_enum = format_ident!("{}Request", api.name);

            quote! {
                pub use crate::#module_name::{#client_name, #request_enum};
            }
        })
        .collect();

    // Re-export response types from definitions
    let definitions_reexports: Vec<_> = apis
        .iter()
        .map(|api| {
            let module_name = format_ident!("{}", get_module_path(api));
            quote! {
                pub use schematic_definitions::#module_name::*;
            }
        })
        .collect();

    quote! {
        //! Convenient re-exports for working with generated API clients.
        //!
        //! ## Examples
        //!
        //! ```ignore
        //! use schematic_schema::prelude::*;
        //!
        //! #[tokio::main]
        //! async fn main() -> Result<(), SchematicError> {
        //!     let client = OpenAI::new();
        //!     // Use client...
        //!     Ok(())
        //! }
        //! ```

        // Shared types
        pub use crate::shared::{RequestParts, SchematicError};

        // API clients and request types
        #(#api_reexports)*

        // Response types from definitions
        #(#definitions_reexports)*
    }
}

// Keep the old function for backwards compatibility in tests
#[doc(hidden)]
pub fn assemble_api_code(api: &RestApi) -> TokenStream {
    assemble_api_module(api)
}

/// Validates generated code using syn.
///
/// Parses the token stream as a complete Rust file to ensure it's syntactically
/// valid before writing to disk.
///
/// ## Arguments
///
/// * `tokens` - The generated code to validate
///
/// ## Returns
///
/// The parsed `syn::File` on success, or an error if the code is invalid.
///
/// ## Errors
///
/// Returns `GeneratorError::CodeGenError` if the code fails to parse.
pub fn validate_code(tokens: &TokenStream) -> Result<syn::File, GeneratorError> {
    syn::parse2(tokens.clone())
        .map_err(|e| GeneratorError::CodeGenError(format!("Generated code is invalid: {}", e)))
}

/// Formats generated code using prettyplease.
///
/// Converts a parsed syn::File back to a nicely formatted string,
/// prepending an auto-generated notice as a regular comment.
///
/// ## Arguments
///
/// * `file` - The parsed Rust file to format
///
/// ## Returns
///
/// A formatted string representation of the code with auto-generated notice.
pub fn format_code(file: &syn::File) -> String {
    let formatted = prettyplease::unparse(file);
    // Prepend auto-generated notice as regular comment
    format!(
        "// This code was automatically generated by schematic-gen. Do not edit manually.\n\n{}",
        formatted
    )
}

/// Writes content to a file atomically using temp file + rename.
///
/// This pattern ensures that:
/// - The file is never left in a partially-written state
/// - Other processes see either the old or new content, never a mix
/// - Power failures or crashes don't corrupt the file
///
/// ## Arguments
///
/// * `path` - The target file path
/// * `content` - The content to write
///
/// ## Returns
///
/// `Ok(())` on success.
///
/// ## Errors
///
/// Returns `GeneratorError::WriteError` if:
/// - Parent directories cannot be created
/// - The temp file cannot be written
/// - The rename operation fails
pub fn write_atomic(path: &Path, content: &str) -> Result<(), GeneratorError> {
    // Create parent directories if needed
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| GeneratorError::WriteError {
            path: parent.display().to_string(),
            source: e,
        })?;
    }

    // Write to temp file first
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, content).map_err(|e| GeneratorError::WriteError {
        path: temp_path.display().to_string(),
        source: e,
    })?;

    // Atomically rename to final path
    fs::rename(&temp_path, path).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

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
    // Generate and validate lib.rs
    let lib_tokens = assemble_lib_rs(apis);
    let lib_file = validate_code(&lib_tokens)?;
    let lib_formatted = format_code(&lib_file);

    // Generate and validate shared.rs
    let shared_tokens = assemble_shared_module();
    let shared_file = validate_code(&shared_tokens)?;
    let shared_formatted = format_code(&shared_file);

    // Generate and validate prelude.rs
    let prelude_tokens = assemble_prelude(apis);
    let prelude_file = validate_code(&prelude_tokens)?;
    let prelude_formatted = format_code(&prelude_file);

    // Generate and validate each API module
    let mut api_modules: Vec<(String, String)> = Vec::new();
    for api in apis {
        let tokens = assemble_api_module(api);
        let file = validate_code(&tokens)?;
        let formatted = format_code(&file);
        let filename = format!("{}.rs", get_module_path(api));
        api_modules.push((filename, formatted));
    }

    if dry_run {
        println!("=== lib.rs ===\n{}\n", lib_formatted);
        println!("=== shared.rs ===\n{}\n", shared_formatted);
        println!("=== prelude.rs ===\n{}\n", prelude_formatted);
        for (filename, content) in &api_modules {
            println!("=== {} ===\n{}\n", filename, content);
        }
    } else {
        // Write lib.rs
        write_atomic(&output_dir.join("lib.rs"), &lib_formatted)?;

        // Write shared.rs
        write_atomic(&output_dir.join("shared.rs"), &shared_formatted)?;

        // Write prelude.rs
        write_atomic(&output_dir.join("prelude.rs"), &prelude_formatted)?;

        // Write each API module
        for (filename, content) in &api_modules {
            write_atomic(&output_dir.join(filename), content)?;
        }
    }

    // Return the first API module content for backwards compatibility
    Ok(api_modules
        .into_iter()
        .next()
        .map(|(_, content)| content)
        .unwrap_or_default())
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
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List all items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
            }],
            module_path: None,
            request_suffix: None,
        }
    }

    fn make_complex_api() -> RestApi {
        RestApi {
            name: "OpenAI".to_string(),
            description: "OpenAI REST API".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            docs_url: Some("https://platform.openai.com/docs".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["OPENAI_API_KEY".to_string()],
            env_username: None,
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
                },
                Endpoint {
                    id: "RetrieveModel".to_string(),
                    method: RestMethod::Get,
                    path: "/models/{model}".to_string(),
                    description: "Retrieves a model".to_string(),
                    request: None,
                    response: ApiResponse::json_type("Model"),
                    headers: vec![],
                },
                Endpoint {
                    id: "CreateCompletion".to_string(),
                    method: RestMethod::Post,
                    path: "/completions".to_string(),
                    description: "Creates a completion".to_string(),
                    request: Some(ApiRequest::json_type("CreateCompletionRequest")),
                    response: ApiResponse::json_type("Completion"),
                    headers: vec![],
                },
            ],
            module_path: None,
            request_suffix: None,
        }
    }

    // === assemble_api_code tests ===

    #[test]
    fn assemble_api_code_produces_valid_tokenstream() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);

        // Should produce non-empty output
        assert!(!tokens.is_empty());
    }

    #[test]
    fn assemble_api_code_includes_all_components() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();

        // Should include error type
        assert!(code.contains("SchematicError"));

        // Should include request structs
        assert!(code.contains("ListModelsRequest"));
        assert!(code.contains("RetrieveModelRequest"));
        assert!(code.contains("CreateCompletionRequest"));

        // Should include request enum
        assert!(code.contains("OpenAIRequest"));

        // Should include API struct
        assert!(code.contains("struct OpenAI"));

        // Should include request method
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

        // Generated code should not need blanket lint suppressions
        // All public items are used, all imports are used
        assert!(!code.contains("dead_code"));
        assert!(!code.contains("unused_imports"));
    }

    // === validate_code tests ===

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
        // Create a token stream that is valid tokens but not a valid Rust file
        // "let x =" is an incomplete statement that won't parse as a file
        let invalid_tokens = quote! {
            let x =
        };

        let result = validate_code(&invalid_tokens);
        assert!(result.is_err());

        match result {
            Err(GeneratorError::CodeGenError(_)) => {} // Expected
            Err(other) => panic!("Unexpected error type: {:?}", other),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    // === format_code tests ===

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

        // Should have proper indentation and newlines
        assert!(formatted.contains('\n'));
        // Should have doc comments
        assert!(formatted.contains("///") || formatted.contains("//!"));
    }

    #[test]
    fn format_code_preserves_structure() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();

        let formatted = format_code(&file);

        // All major elements should be present
        assert!(formatted.contains("use crate::shared::{RequestParts, SchematicError}"));
        assert!(formatted.contains("pub struct OpenAI"));
        assert!(formatted.contains("pub enum OpenAIRequest"));
    }

    // === write_atomic tests ===

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

        // Write initial content
        fs::write(&file_path, "// Old content").unwrap();

        // Overwrite with atomic write
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

        // Check no .tmp file exists
        let temp_path = file_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    // === generate_and_write tests ===

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

        // No file should be created in dry run
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

        // Check the API module file (openai.rs)
        let api_module_path = temp_dir.path().join("openai.rs");
        let content = fs::read_to_string(api_module_path).unwrap();

        // Should be properly formatted (has indentation)
        assert!(content.contains("    ")); // 4-space indent
        // Should have all components
        assert!(content.contains("pub struct OpenAI"));
        assert!(content.contains("pub enum OpenAIRequest"));
        assert!(content.contains("use crate::shared::{RequestParts, SchematicError}"));

        // Check shared.rs exists and contains SchematicError and RequestParts
        let shared_path = temp_dir.path().join("shared.rs");
        let shared_content = fs::read_to_string(shared_path).unwrap();
        assert!(shared_content.contains("pub type RequestParts"));
        assert!(shared_content.contains("pub enum SchematicError"));

        // Check lib.rs exists and has module declarations
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

        // The returned content should match the API module file, not lib.rs
        // Note: "TestApi" has "Api" suffix, so it infers to "test"
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

    // === Integration tests ===

    #[test]
    fn full_pipeline_with_all_auth_strategies() {
        // Test configurations: (auth, env_auth, env_username)
        // For BasicAuth, password comes from env_auth[0]
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
                vec!["PASS".to_string()], // Password from env_auth[0]
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
                env_auth,
                env_username,
                headers: vec![],
                endpoints: vec![Endpoint {
                    id: "Test".to_string(),
                    method: RestMethod::Get,
                    path: "/test".to_string(),
                    description: "Test endpoint".to_string(),
                    request: None,
                    response: ApiResponse::json_type("TestResponse"),
                    headers: vec![],
                }],
                module_path: None,
                request_suffix: None,
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
            })
            .collect();

        let api = RestApi {
            name: "AllMethods".to_string(),
            description: "API with all HTTP methods".to_string(),
            base_url: "https://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints,
            module_path: None,
            request_suffix: None,
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
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
        };

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);

        // Even empty API should produce valid code
        assert!(result.is_ok());
    }

    #[test]
    fn generated_code_has_module_documentation() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();

        let code = generate_and_write(&api, temp_dir.path(), true).unwrap();

        // Should have module-level doc comments
        assert!(code.contains("//!"));
        assert!(code.contains("Generated API client"));
        // Auto-generated notice should be a regular comment (not doc comment)
        assert!(code.starts_with("// This code was automatically generated"));
        assert!(code.contains("Do not edit manually"));
    }
}
