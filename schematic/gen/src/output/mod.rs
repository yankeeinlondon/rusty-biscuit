//! Output assembly and file writing for generated code.
//!
//! This module handles the final phase of code generation: assembling all generated
//! pieces into a complete Rust file, validating the output, formatting it, and
//! writing it to disk atomically.
//!
//! ## Output Structure
//!
//! The generator produces per-API module files. Small modules are single files;
//! large modules (exceeding [`SPLIT_THRESHOLD_LINES`]) are split into directories:
//!
//! ```text
//! schema/src/
//! ├── lib.rs              # Module declarations and re-exports
//! ├── openai.rs           # Small API module (single file)
//! ├── emqx/               # Large API module (split directory)
//! │   ├── mod.rs          #   Client structs, re-exports, submodule declarations
//! │   ├── requests.rs     #   Request structs and request enums
//! │   ├── responses.rs    #   Definitions re-exports
//! │   └── client.rs       #   HTTP method impl blocks
//! └── prelude.rs          # Common re-exports for consumers
//! ```
//!
//! ## Safety Guarantees
//!
//! - **Syntax validation**: All generated code is parsed with `syn` before
//!   writing, which catches grammar/syntax errors only — it does not
//!   type-check, resolve names, or guarantee the code compiles.
//! - **Formatting**: Output is formatted with `prettyplease` for consistent style
//! - **Atomic writes**: Uses temp file + rename pattern to prevent partial writes
//! - **Ownership-aware cleanup**: Stale-file removal only touches artifacts this
//!   generator produced (identified by the generated-file header), so
//!   hand-written files in the output directory are never deleted.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use schematic_define::RestApi;

use crate::errors::GeneratorError;

pub mod assemble;
pub mod format;
pub mod options;
pub mod write;
pub mod ws_modules;

pub use assemble::{
    SPLIT_THRESHOLD_LINES, SplitApiParts, assemble_api_code, assemble_api_module,
    assemble_api_module_with_options, assemble_combined_api_module, assemble_lib_rs,
    assemble_lib_rs_with_options, assemble_prelude, assemble_prelude_with_options,
    assemble_shared_module, assemble_split_api_module, assemble_split_combined_api_module,
    get_module_path, get_request_suffix,
};
pub use format::{GENERATED_FILE_HEADER, format_code, validate_code};
pub use options::OutputOptions;
pub use write::write_atomic;

/// Generates and writes all API code to the output directory.
///
/// Convenience wrapper around [`generate_and_write_all`] for a single API.
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

/// Represents generated output for a single API module.
///
/// A module is either a single `.rs` file or a directory of submodules.
enum GeneratedModule {
    /// Single-file module: `{module_path}.rs`
    SingleFile { filename: String, content: String },
    /// Directory module: `{module_path}/mod.rs` + submodules
    Directory {
        module_path: String,
        mod_rs: String,
        requests_rs: String,
        responses_rs: String,
        client_rs: String,
    },
}

/// Generates and writes all API code to the output directory.
///
/// This is the main entry point for code generation. It produces:
/// - `lib.rs` - Module declarations and crate documentation
/// - `prelude.rs` - Convenient re-exports
/// - `{api_name}.rs` or `{api_name}/` - Per-API module files
///
/// Modules whose generated code exceeds [`SPLIT_THRESHOLD_LINES`] are split into
/// a directory with `mod.rs`, `requests.rs`, `responses.rs`, and `client.rs`.
/// Smaller modules remain as single files.
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

    let mut generated: Vec<GeneratedModule> = Vec::new();
    for (module_path, group) in &module_groups {
        let tokens = if group.len() == 1 {
            assemble_api_module(group[0])
        } else {
            assemble_combined_api_module(group)
        };
        let file = validate_code(&tokens)?;
        let formatted = format_code(&file);
        let line_count = formatted.lines().count();

        if line_count > SPLIT_THRESHOLD_LINES {
            let parts = if group.len() == 1 {
                assemble_split_api_module(group[0], &OutputOptions::default())
            } else {
                assemble_split_combined_api_module(group)
            };

            let mod_rs_file = validate_code(&parts.mod_rs)?;
            let requests_file = validate_code(&parts.requests_rs)?;
            let responses_file = validate_code(&parts.responses_rs)?;
            let client_file = validate_code(&parts.client_rs)?;

            generated.push(GeneratedModule::Directory {
                module_path: module_path.clone(),
                mod_rs: format_code(&mod_rs_file),
                requests_rs: format_code(&requests_file),
                responses_rs: format_code(&responses_file),
                client_rs: format_code(&client_file),
            });
        } else {
            let filename = format!("{}.rs", module_path);
            generated.push(GeneratedModule::SingleFile {
                filename,
                content: formatted,
            });
        }
    }

    let (ws_shared_filename, ws_shared_content) = ws_modules::generate_ws_shared_module()?;
    let ws_modules_list = ws_modules::generate_ws_definition_modules()?;

    if dry_run {
        println!("=== lib.rs ===\n{}\n", lib_formatted);
        println!("=== shared.rs ===\n{}\n", shared_formatted);
        println!("=== prelude.rs ===\n{}\n", prelude_formatted);
        for module in &generated {
            match module {
                GeneratedModule::SingleFile { filename, content } => {
                    println!("=== {} ===\n{}\n", filename, content);
                }
                GeneratedModule::Directory {
                    module_path,
                    mod_rs,
                    requests_rs,
                    responses_rs,
                    client_rs,
                } => {
                    println!("=== {}/mod.rs ===\n{}\n", module_path, mod_rs);
                    println!("=== {}/requests.rs ===\n{}\n", module_path, requests_rs);
                    println!("=== {}/responses.rs ===\n{}\n", module_path, responses_rs);
                    println!("=== {}/client.rs ===\n{}\n", module_path, client_rs);
                }
            }
        }
        println!("=== {} ===\n{}\n", ws_shared_filename, ws_shared_content);
        for (filename, content) in &ws_modules_list {
            println!("=== {} ===\n{}\n", filename, content);
        }
    } else {
        remove_conflicting_paths(output_dir, &generated)?;

        write_atomic(&output_dir.join("lib.rs"), &lib_formatted)?;

        write_atomic(&output_dir.join("shared.rs"), &shared_formatted)?;

        write_atomic(&output_dir.join("prelude.rs"), &prelude_formatted)?;

        for module in &generated {
            match module {
                GeneratedModule::SingleFile { filename, content } => {
                    write_atomic(&output_dir.join(filename), content)?;
                }
                GeneratedModule::Directory {
                    module_path,
                    mod_rs,
                    requests_rs,
                    responses_rs,
                    client_rs,
                } => {
                    let dir = output_dir.join(module_path);
                    write_atomic(&dir.join("mod.rs"), mod_rs)?;
                    write_atomic(&dir.join("requests.rs"), requests_rs)?;
                    write_atomic(&dir.join("responses.rs"), responses_rs)?;
                    write_atomic(&dir.join("client.rs"), client_rs)?;
                }
            }
        }

        write_atomic(&output_dir.join(&ws_shared_filename), &ws_shared_content)?;
        for (filename, content) in &ws_modules_list {
            write_atomic(&output_dir.join(filename), content)?;
        }

        let stale_names = collect_expected_files(&generated, &ws_shared_filename, &ws_modules_list);
        cleanup_stale_files(output_dir, &stale_names)?;
    }

    Ok(generated
        .into_iter()
        .next()
        .map(|m| match m {
            GeneratedModule::SingleFile { content, .. } => content,
            GeneratedModule::Directory { mod_rs, .. } => mod_rs,
        })
        .unwrap_or_default())
}

/// Collects the set of expected filenames (and directory names) that should
/// exist after generation.
fn collect_expected_files(
    generated: &[GeneratedModule],
    ws_shared_filename: &str,
    ws_modules: &[(String, String)],
) -> HashSet<String> {
    let mut files = HashSet::new();
    files.insert("lib.rs".to_string());
    files.insert("shared.rs".to_string());
    files.insert("prelude.rs".to_string());
    files.insert(ws_shared_filename.to_string());
    for (filename, _) in ws_modules {
        files.insert(filename.clone());
    }
    for module in generated {
        match module {
            GeneratedModule::SingleFile { filename, .. } => {
                files.insert(filename.clone());
            }
            GeneratedModule::Directory { module_path, .. } => {
                files.insert(module_path.clone());
            }
        }
    }
    files
}

/// Returns whether `path` is a `.rs` file this generator wrote.
///
/// Ownership is proven by the [`GENERATED_FILE_HEADER`] on the first line.
/// Files without the header (hand-written, or produced by another tool) are
/// reported as not owned and are therefore never removed by cleanup.
fn is_generator_owned_rs_file(path: &Path) -> bool {
    if path.extension().is_none_or(|ext| ext != "rs") {
        return false;
    }
    match fs::read_to_string(path) {
        Ok(contents) => contents
            .lines()
            .next()
            .is_some_and(|line| line.starts_with(GENERATED_FILE_HEADER)),
        Err(_) => false,
    }
}

/// Returns whether `dir` and its entire subtree were produced by this generator.
///
/// True only when every regular file carries the generated header and every
/// subdirectory is itself generator-owned. An empty directory is owned
/// vacuously. This is the gate for recursively removing a directory: the
/// generator never `remove_dir_all`s a tree it cannot prove it created.
fn is_generator_owned_dir(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let owned = if path.is_dir() {
            is_generator_owned_dir(&path)
        } else {
            is_generator_owned_rs_file(&path)
        };
        if !owned {
            return false;
        }
    }
    true
}

/// Removes a generator-owned file, surfacing any I/O error.
fn remove_owned_file(path: &Path) -> Result<(), GeneratorError> {
    fs::remove_file(path).map_err(|source| GeneratorError::WriteError {
        path: path.display().to_string(),
        source,
    })
}

/// Removes a generator-owned directory tree, surfacing any I/O error.
fn remove_owned_dir(path: &Path) -> Result<(), GeneratorError> {
    fs::remove_dir_all(path).map_err(|source| GeneratorError::WriteError {
        path: path.display().to_string(),
        source,
    })
}

/// Removes files or directories that would conflict with new module output.
///
/// When a module transitions from single-file to directory (or vice versa),
/// the old artifact must be removed before the new one is written, because
/// a file and directory cannot share the same path. Only generator-owned
/// artifacts are removed; a conflicting hand-written file or directory is left
/// in place (the subsequent write will surface the conflict instead of this
/// helper silently destroying user content).
fn remove_conflicting_paths(
    output_dir: &Path,
    generated: &[GeneratedModule],
) -> Result<(), GeneratorError> {
    for module in generated {
        match module {
            GeneratedModule::SingleFile { filename, .. } => {
                let dir_name = filename.strip_suffix(".rs").unwrap_or(filename);
                let dir_path = output_dir.join(dir_name);
                if dir_path.is_dir() && is_generator_owned_dir(&dir_path) {
                    remove_owned_dir(&dir_path)?;
                }
            }
            GeneratedModule::Directory { module_path, .. } => {
                let rs_filename = format!("{}.rs", module_path);
                let rs_path = output_dir.join(&rs_filename);
                if rs_path.is_file() && is_generator_owned_rs_file(&rs_path) {
                    remove_owned_file(&rs_path)?;
                }
            }
        }
    }
    Ok(())
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
        // The import path writes the API as a single `{module}.rs` file. If a
        // prior run left a directory-form `{module}/`, remove it first (when
        // owned) so we don't leave `foo.rs` beside `foo/`, which Rust rejects
        // as an ambiguous module.
        let module = GeneratedModule::SingleFile {
            filename: api_filename.clone(),
            content: api_formatted.clone(),
        };
        remove_conflicting_paths(output_dir, std::slice::from_ref(&module))?;

        write_atomic(&output_dir.join("lib.rs"), &lib_formatted)?;

        write_atomic(&output_dir.join("shared.rs"), &shared_formatted)?;

        write_atomic(&output_dir.join("prelude.rs"), &prelude_formatted)?;

        write_atomic(&output_dir.join(&api_filename), &api_formatted)?;
    }

    Ok(api_formatted)
}

/// Removes stale generator-owned artifacts from the output directory that are
/// not part of the current generation set.
///
/// For single-file modules, removes `{name}.rs` if no longer expected.
/// For directory modules, the directory itself is expected; stale `.rs`
/// files at the top level are also removed.
///
/// Only artifacts this generator provably owns are removed: `.rs` files must
/// carry the generated header, and directories must contain only
/// generator-owned content. Anything else (hand-written files, unrelated
/// subdirectories such as `bin/`) is left untouched. Removal errors are
/// surfaced rather than discarded.
fn cleanup_stale_files(
    output_dir: &Path,
    expected: &HashSet<String>,
) -> Result<(), GeneratorError> {
    let Ok(entries) = fs::read_dir(output_dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if expected.contains(name) {
            continue;
        }
        if path.is_file() {
            if is_generator_owned_rs_file(&path) {
                remove_owned_file(&path)?;
            }
        } else if path.is_dir() && is_generator_owned_dir(&path) {
            remove_owned_dir(&path)?;
        }
    }
    Ok(())
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
                    value_prefix: None,
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
        assert_eq!(
            count, 1,
            "Expected exactly 1 'pub mod foo', found {}",
            count
        );
    }

    #[test]
    fn assemble_combined_module_includes_both_apis() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];
        let tokens = assemble_combined_api_module(&apis);
        let file = validate_code(&tokens).unwrap();
        let code = format_code(&file);
        assert!(
            code.contains("pub struct FooNative"),
            "Missing FooNative struct"
        );
        assert!(
            code.contains("pub struct FooCompat"),
            "Missing FooCompat struct"
        );
        assert!(
            code.contains("pub enum FooNativeRequest"),
            "Missing FooNativeRequest enum"
        );
        assert!(
            code.contains("pub enum FooCompatRequest"),
            "Missing FooCompatRequest enum"
        );
        let import_count = code
            .matches("pub use schematic_definitions::foo::*")
            .count();
        assert_eq!(
            import_count, 1,
            "Expected exactly 1 definitions import, found {}",
            import_count
        );
        let shared_count = code
            .matches("use crate::shared::{RequestParts, SchematicError}")
            .count();
        assert_eq!(
            shared_count, 1,
            "Expected exactly 1 shared import, found {}",
            shared_count
        );
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
        assert!(
            !temp_dir.path().join("foonative.rs").exists(),
            "Unexpected foonative.rs"
        );
        assert!(
            !temp_dir.path().join("foocompat.rs").exists(),
            "Unexpected foocompat.rs"
        );
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
        // Stale files from a prior generation carry the generated header, so
        // they are owned and get removed.
        let stale = format!("{GENERATED_FILE_HEADER}\n\n// stale");
        fs::write(temp_dir.path().join("ollamaopenai.rs"), &stale).unwrap();
        fs::write(temp_dir.path().join("old_api.rs"), &stale).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "# keep").unwrap();
        generate_and_write_all(&apis, temp_dir.path(), false).unwrap();
        assert!(
            !temp_dir.path().join("ollamaopenai.rs").exists(),
            "ollamaopenai.rs should be deleted"
        );
        assert!(
            !temp_dir.path().join("old_api.rs").exists(),
            "old_api.rs should be deleted"
        );
        assert!(
            temp_dir.path().join("Cargo.toml").exists(),
            "Cargo.toml should be preserved"
        );
        assert!(temp_dir.path().join("lib.rs").exists());
        assert!(temp_dir.path().join("shared.rs").exists());
        assert!(temp_dir.path().join("prelude.rs").exists());
        assert!(temp_dir.path().join("test.rs").exists());
    }

    #[test]
    fn cleanup_preserves_hand_written_files_and_foreign_dirs() {
        let api = make_simple_api();
        let apis: Vec<&RestApi> = vec![&api];
        let temp_dir = TempDir::new().unwrap();

        // A hand-written module WITHOUT the generated header must survive.
        let hand_written = temp_dir.path().join("handwritten.rs");
        fs::write(&hand_written, "pub fn keep_me() {}\n").unwrap();

        // An unrelated subdirectory (e.g. a binary crate) must survive intact.
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let bin_main = bin_dir.join("main.rs");
        fs::write(&bin_main, "fn main() {}\n").unwrap();

        // A directory that mixes generator output with a hand-written file must
        // survive: it is not provably generator-owned.
        let mixed_dir = temp_dir.path().join("mixed");
        fs::create_dir_all(&mixed_dir).unwrap();
        fs::write(
            mixed_dir.join("generated.rs"),
            format!("{GENERATED_FILE_HEADER}\n\n// gen"),
        )
        .unwrap();
        let mixed_keep = mixed_dir.join("keep.rs");
        fs::write(&mixed_keep, "pub fn keep() {}\n").unwrap();

        generate_and_write_all(&apis, temp_dir.path(), false).unwrap();

        assert!(hand_written.exists(), "hand-written .rs must survive");
        assert!(bin_dir.is_dir(), "unrelated bin/ dir must survive");
        assert!(bin_main.exists(), "bin/main.rs must survive");
        assert!(
            mixed_dir.is_dir() && mixed_keep.exists(),
            "a dir containing hand-written content must survive"
        );
    }

    #[test]
    fn standalone_import_removes_conflicting_owned_directory() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let module = get_module_path(&api);

        // Simulate a prior directory-form module owned by the generator.
        let dir = temp_dir.path().join(&module);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("mod.rs"),
            format!("{GENERATED_FILE_HEADER}\n\n// old"),
        )
        .unwrap();

        generate_and_write_standalone(&api, temp_dir.path(), false, false).unwrap();

        assert!(
            !dir.is_dir(),
            "prior directory-form module must be removed to avoid foo.rs/foo ambiguity"
        );
        assert!(
            temp_dir.path().join(format!("{module}.rs")).exists(),
            "single-file module should be written"
        );
    }

    #[test]
    fn standalone_import_preserves_hand_written_conflicting_directory() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let module = get_module_path(&api);

        // A conflicting directory that is NOT generator-owned must be left in
        // place rather than destroyed.
        let dir = temp_dir.path().join(&module);
        fs::create_dir_all(&dir).unwrap();
        let keep = dir.join("keep.rs");
        fs::write(&keep, "pub fn keep() {}\n").unwrap();

        generate_and_write_standalone(&api, temp_dir.path(), false, false).unwrap();

        assert!(keep.exists(), "hand-written directory content must survive");
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
        assert!(
            code.contains("FooNativeRequest"),
            "Missing FooNativeRequest in prelude"
        );
        assert!(
            code.contains("FooCompatRequest"),
            "Missing FooCompatRequest in prelude"
        );
    }

    fn make_large_api() -> RestApi {
        let endpoints: Vec<Endpoint> = (0..500)
            .map(|i| Endpoint {
                id: format!("Endpoint{}", i),
                method: if i % 5 == 0 {
                    RestMethod::Post
                } else {
                    RestMethod::Get
                },
                path: if i % 3 == 0 {
                    format!("/endpoint/{}/{{param}}", i)
                } else {
                    format!("/endpoint/{}", i)
                },
                description: format!("Endpoint {} description with enough text to add some length to the generated documentation comments", i),
                request: if i % 5 == 0 {
                    Some(ApiRequest::json_type(format!("Endpoint{}Body", i)))
                } else {
                    None
                },
                response: ApiResponse::json_type("TestResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            })
            .collect();

        RestApi {
            name: "LargeApi".to_string(),
            description: "A large API for testing split output".to_string(),
            base_url: "https://api.large.com/v1".to_string(),
            docs_url: Some("https://docs.large.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["LARGE_API_KEY".to_string()],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints,
            module_path: None,
            request_suffix: None,
            version: None,
        }
    }

    #[test]
    fn large_api_produces_split_directory() {
        let api = make_large_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), false).unwrap();

        let single_file = temp_dir.path().join("large.rs");
        let dir = temp_dir.path().join("large");

        if single_file.exists() && !dir.is_dir() {
            let content = fs::read_to_string(&single_file).unwrap();
            let line_count = content.lines().count();
            panic!(
                "Generated as single file ({} lines, threshold is {}). \
                 Increase endpoint count in make_large_api().",
                line_count, SPLIT_THRESHOLD_LINES
            );
        }

        assert!(dir.is_dir(), "large should be a directory");
        assert!(dir.join("mod.rs").exists(), "large/mod.rs should exist");
        assert!(
            dir.join("requests.rs").exists(),
            "large/requests.rs should exist"
        );
        assert!(
            dir.join("responses.rs").exists(),
            "large/responses.rs should exist"
        );
        assert!(
            dir.join("client.rs").exists(),
            "large/client.rs should exist"
        );

        assert!(
            !temp_dir.path().join("large.rs").exists(),
            "large.rs should NOT exist (directory takes precedence)"
        );

        let mod_rs = fs::read_to_string(dir.join("mod.rs")).unwrap();
        assert!(
            mod_rs.contains("pub struct LargeApi"),
            "mod.rs should contain struct definition"
        );
        assert!(
            mod_rs.contains("pub mod requests;"),
            "mod.rs should declare requests submodule"
        );
        assert!(
            mod_rs.contains("pub use requests::*;"),
            "mod.rs should re-export requests"
        );

        let requests = fs::read_to_string(dir.join("requests.rs")).unwrap();
        assert!(
            requests.contains("pub struct Endpoint0Request"),
            "requests.rs should contain request structs"
        );
        assert!(
            requests.contains("pub enum LargeApiRequest"),
            "requests.rs should contain request enum"
        );

        let client = fs::read_to_string(dir.join("client.rs")).unwrap();
        assert!(
            client.contains("impl LargeApi"),
            "client.rs should contain impl block"
        );
    }

    #[test]
    fn split_module_public_api_is_accessible() {
        let api = make_large_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), false).unwrap();

        let dir = temp_dir.path().join("large");

        let mod_rs = fs::read_to_string(dir.join("mod.rs")).unwrap();
        assert!(
            !mod_rs.contains("pub use client::*;"),
            "mod.rs should not re-export client items (only impl blocks)"
        );
        assert!(
            mod_rs.contains("pub use requests::*;"),
            "mod.rs should re-export request items"
        );
        assert!(
            mod_rs.contains("pub use responses::*;"),
            "mod.rs should re-export response items"
        );

        let client_rs = fs::read_to_string(dir.join("client.rs")).unwrap();
        assert!(
            !client_rs.contains("use super::responses::*;"),
            "client.rs should not import responses (covered by use super::*)"
        );

        let responses = fs::read_to_string(dir.join("responses.rs")).unwrap();
        assert!(
            responses.contains("pub use"),
            "responses.rs should have pub use for definitions"
        );
    }

    #[test]
    fn split_to_single_file_transition_cleans_up() {
        let large_api = make_large_api();
        let mut small_api = make_simple_api();
        small_api.name = "LargeApi".to_string();
        let temp_dir = TempDir::new().unwrap();

        generate_and_write(&large_api, temp_dir.path(), false).unwrap();
        assert!(temp_dir.path().join("large").is_dir());

        generate_and_write(&small_api, temp_dir.path(), false).unwrap();
        assert!(
            temp_dir.path().join("large.rs").exists(),
            "Should now be a single file"
        );
        assert!(
            !temp_dir.path().join("large").is_dir(),
            "Directory should be cleaned up"
        );
    }
}
