//! Postman collection export.
//!
//! Generates Postman Collection v2.1.0 JSON files from schematic RestApi definitions.

use std::fs;
use std::path::{Path, PathBuf};

use schematic_define::RestApi;

use crate::errors::GeneratorError;
use crate::export::naming::resolve_module_name;

mod auth;
mod collection;
mod examples;
mod item;
mod request;
pub(crate) mod types;
mod variables;

// Re-export all public types.
pub use types::{
    PostmanAuth, PostmanBody, PostmanBodyOptions, PostmanCollection, PostmanFormParam,
    PostmanHeader, PostmanInfo, PostmanItem, PostmanQuery, PostmanRawOptions, PostmanRequest,
    PostmanUrl, PostmanVariable,
};

// Re-export collection builders so external consumers can call them.
pub use collection::{build_postman_collection, build_postman_collection_grouped};

/// Writes a Postman collection to disk.
///
/// File naming: `<module_name>.postman_collection.json`
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use schematic_define::{RestApi, RestMethod, AuthStrategy, ApiResponse};
/// use schematic_gen::export::postman::write_postman;
///
/// let api = RestApi {
///     name: "TestApi".to_string(),
///     description: "Test API".to_string(),
///     base_url: "https://api.test.com".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![],
///     module_path: None,
///     request_suffix: None,
///     version: None,
///     env_mapping: None,
/// };
///
/// let path = write_postman(&api, Path::new("/tmp"), false).unwrap();
/// assert_eq!(path.file_name().unwrap(), "testapi.postman_collection.json");
/// ```
pub fn write_postman(api: &RestApi, dir: &Path, dry_run: bool) -> Result<PathBuf, GeneratorError> {
    let collection = build_postman_collection(api);
    let module_name = resolve_module_name(api);
    let filename = format!("{}.postman_collection.json", module_name);
    let path = dir.join(&filename);

    if dry_run {
        return Ok(path);
    }

    let json = serde_json::to_string_pretty(&collection).map_err(|e| {
        GeneratorError::CodeGenError(format!("Failed to serialize Postman collection: {}", e))
    })?;

    fs::write(&path, json).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(path)
}

/// Writes a grouped Postman collection to disk.
///
/// File naming: `<module_name>.postman_collection.json`
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use schematic_define::{RestApi, AuthStrategy};
/// use schematic_gen::export::postman::write_postman_grouped;
///
/// let api1 = RestApi {
///     name: "OllamaNative".to_string(),
///     description: "Native API".to_string(),
///     base_url: "http://localhost:11434".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![],
///     module_path: Some("ollama".to_string()),
///     request_suffix: None,
///     version: None,
///     env_mapping: None,
/// };
///
/// let path = write_postman_grouped(&[&api1], "ollama", Path::new("/tmp"), false).unwrap();
/// assert_eq!(path.file_name().unwrap(), "ollama.postman_collection.json");
/// ```
pub fn write_postman_grouped(
    apis: &[&RestApi],
    module_name: &str,
    dir: &Path,
    dry_run: bool,
) -> Result<PathBuf, GeneratorError> {
    let collection = build_postman_collection_grouped(apis, module_name);
    let filename = format!("{}.postman_collection.json", module_name);
    let path = dir.join(&filename);

    if dry_run {
        return Ok(path);
    }

    let json = serde_json::to_string_pretty(&collection).map_err(|e| {
        GeneratorError::CodeGenError(format!("Failed to serialize Postman collection: {}", e))
    })?;

    fs::write(&path, json).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(path)
}

#[cfg(test)]
mod tests;
