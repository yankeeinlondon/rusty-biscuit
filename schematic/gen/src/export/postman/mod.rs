//! Postman collection export.
//!
//! Generates Postman Collection v2.1.0 JSON files from schematic RestApi
//! definitions: requests with example bodies, saved example responses,
//! GraphQL operations, and every variable they reference declared. An
//! optional Postman environment file lists the same variables with values.

use std::fs;
use std::path::{Path, PathBuf};

use schematic_define::RestApi;

use crate::diagnostics::Output;
use crate::errors::GeneratorError;
use schematic_definitions::naming::resolve_module_name;
use crate::validation::{ensure_no_errors, validate_postman};

mod auth;
mod collection;
mod environment;
mod graphql;
mod item;
mod request;
mod responses;
pub(crate) mod types;
mod variables;

// Re-export all public types.
pub use types::{
    PostmanAuth, PostmanBody, PostmanBodyOptions, PostmanCollection, PostmanEnvironment,
    PostmanEnvironmentValue, PostmanFormParam, PostmanHeader, PostmanInfo, PostmanItem,
    PostmanQuery, PostmanRawOptions, PostmanRequest, PostmanResponse, PostmanUrl,
    PostmanVariable,
};

// Re-export collection builders so external consumers can call them.
pub use collection::{build_postman_collection, build_postman_collection_grouped};

/// Writes a Postman collection to disk.
///
/// File naming: `<module_name>.postman_collection.json`
///
/// ## Errors
///
/// [`GeneratorError::Validation`] when [`validate_postman`] reports an error
/// (nothing is written), or a serialization or write error.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use schematic_define::{DefinitionContext, RestApi};
/// use schematic_gen::export::postman::write_postman;
///
/// let ctx = DefinitionContext::types_in("crate::types");
/// let api = RestApi::builder(&ctx, "TestApi", "https://api.test.com")
///     .describes("Test API")
///     .no_auth()
///     .build().unwrap();
///
/// let path = write_postman(&api, Path::new("/tmp"), false).unwrap();
/// assert_eq!(path.file_name().unwrap(), "test.postman_collection.json");
/// ```
pub fn write_postman(api: &RestApi, dir: &Path, dry_run: bool) -> Result<PathBuf, GeneratorError> {
    ensure_no_errors(Output::Postman, validate_postman(&[api]))?;
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
/// ## Errors
///
/// [`GeneratorError::Validation`] when [`validate_postman`] reports an error
/// for any member (nothing is written), or a serialization or write error.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use schematic_define::{DefinitionContext, RestApi};
/// use schematic_gen::export::postman::write_postman_grouped;
///
/// let ctx = DefinitionContext::types_in("crate::types");
/// let api1 = RestApi::builder(&ctx, "OllamaNative", "http://localhost:11434")
///     .describes("Native API")
///     .no_auth()
///     .module_path("ollama")
///     .build().unwrap();
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
    ensure_no_errors(Output::Postman, validate_postman(apis))?;
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

/// The collection of one export module: a module with one API gets that
/// API's collection, a shared module the grouped collection.
fn build_module(apis: &[&RestApi], module_name: &str) -> collection::Built {
    match apis {
        [api] => collection::build_single(api),
        _ => collection::build_grouped(apis, module_name),
    }
}

/// The warnings building `api`'s collection reports: each request body,
/// saved response, or GraphQL `variables` exported without an example
/// because none validating against its schema could be built, and GraphQL
/// operations no address rule can place.
pub(crate) fn export_diagnostics(api: &RestApi) -> Vec<crate::diagnostics::ValidationDiagnostic> {
    collection::build_single(api).collected.diagnostics
}

/// Builds the Postman environment of one export module: every variable its
/// collection declares, with base URLs and GraphQL addresses valued and
/// credentials and parameters empty (credentials typed `secret`).
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionContext, RestApi};
/// use schematic_gen::export::postman::build_postman_environment;
///
/// let ctx = DefinitionContext::types_in("crate::types");
/// let api = RestApi::builder(&ctx, "TestApi", "https://api.test.com")
///     .describes("Test API")
///     .bearer_env(["TEST_TOKEN"])
///     .build().unwrap();
///
/// let environment = build_postman_environment(&[&api], "test");
/// let keys: Vec<&str> = environment.values.iter().map(|value| value.key.as_str()).collect();
/// assert_eq!(keys, ["baseUrl", "bearerToken"]);
/// assert_eq!(environment.values[1].type_field, "secret");
/// ```
pub fn build_postman_environment(apis: &[&RestApi], module_name: &str) -> PostmanEnvironment {
    environment::environment_of(&build_module(apis, module_name))
}

/// Serializes the Postman environment of one export module
/// ([`build_postman_environment`]).
///
/// ## Errors
///
/// [`GeneratorError::CodeGenError`] when serialization fails.
pub fn render_postman_environment(apis: &[&RestApi], module_name: &str) -> Result<String, GeneratorError> {
    serde_json::to_string_pretty(&build_postman_environment(apis, module_name)).map_err(|e| {
        GeneratorError::CodeGenError(format!("Failed to serialize Postman environment: {}", e))
    })
}

/// Writes the Postman environment of one export module to disk.
///
/// File naming: `<module_name>.postman_environment.json`, beside the
/// collection.
///
/// ## Errors
///
/// [`GeneratorError::Validation`] when [`validate_postman`] reports an error
/// (nothing is written), or a serialization or write error.
pub fn write_postman_environment(apis: &[&RestApi], module_name: &str, dir: &Path, dry_run: bool) -> Result<PathBuf, GeneratorError> {
    ensure_no_errors(Output::Postman, validate_postman(apis))?;
    let path = dir.join(format!("{module_name}.postman_environment.json"));
    if dry_run {
        return Ok(path);
    }
    let json = render_postman_environment(apis, module_name)?;
    fs::write(&path, json).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(path)
}

/// Serializes the Postman collection of one export module.
///
/// A module with one API gets that API's collection
/// ([`build_postman_collection`]); a shared module gets the grouped
/// collection ([`build_postman_collection_grouped`]). The text is what
/// [`write_postman`] and [`write_postman_grouped`] write for the same input.
///
/// ## Errors
///
/// [`GeneratorError::CodeGenError`] when serialization fails.
pub fn render_postman_module(apis: &[&RestApi], module_name: &str) -> Result<String, GeneratorError> {
    serde_json::to_string_pretty(&build_module(apis, module_name).collection).map_err(|e| {
        GeneratorError::CodeGenError(format!("Failed to serialize Postman collection: {}", e))
    })
}

#[cfg(test)]
mod tests;
