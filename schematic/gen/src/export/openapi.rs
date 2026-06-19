//! OpenAPI export file writing.
//!
//! This module provides utilities for writing OpenAPI specifications to files
//! after export from schematic API definitions.
//!
//! [`write_openapi_grouped`] is the preferred entry point for module-grouped
//! export (one OpenAPI document per resolved module path, even when multiple
//! `RestApi` definitions share that module). [`write_openapi`] remains for
//! the single-API case and now also derives its filename from the resolved
//! module name rather than from `api.name`.

use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use openapiv3::{OpenAPI, ReferenceOr};
use schematic_define::RestApi;
use schematic_define::openapi::{
    ExportFormat, ExportOptions, SchemaRegistryLike, export, serialize,
};

use crate::errors::GeneratorError;
use crate::export::resolve_module_name;

/// Writes an OpenAPI specification file to the specified directory.
///
/// This function exports the API definition to OpenAPI 3.0.3 format and writes
/// it to a file in the specified directory. The filename is derived from the
/// resolved module name of the API ([`resolve_module_name`]) so artifacts stay
/// aligned with the generated Rust module layout.
///
/// Prefer [`write_openapi_grouped`] when emitting artifacts for an entire
/// module: it merges every member API into one document and applies the same
/// module-name filename rule.
///
/// ## Parameters
///
/// - `api` - The REST API definition to export
/// - `registry` - Schema registry containing JSON schemas for response types
/// - `options` - Export configuration options (format, version, extensions)
/// - `dir` - Output directory for the generated file
///
/// ## Returns
///
/// The path to the written file.
///
/// ## Errors
///
/// Returns `GeneratorError::WriteError` if file writing fails.
/// Returns `GeneratorError::ConfigError` if the directory doesn't exist or export fails.
///
/// ## Examples
///
/// ```text
/// use std::path::Path;
/// use schematic_define::openapi::ExportOptions;
/// use schematic_definitions::openai::{define_openai_api, openapi_registry};
/// use schematic_gen::export::openapi::write_openapi;
///
/// let api = define_openai_api();
/// let registry = openapi_registry();
/// let options = ExportOptions::new().with_version("1.0.0");
///
/// let path = write_openapi(&api, &registry, &options, Path::new("./openapi"))?;
/// assert_eq!(path.file_name().unwrap(), "openai.json");
/// ```
pub fn write_openapi<R: SchemaRegistryLike>(
    api: &RestApi,
    registry: &R,
    options: &ExportOptions,
    dir: &Path,
) -> Result<std::path::PathBuf, GeneratorError> {
    // Validate directory exists
    if !dir.exists() {
        return Err(GeneratorError::OutputDirNotFound(dir.display().to_string()));
    }

    // Export to OpenAPI document
    let doc = export(api, registry, options)
        .map_err(|e| GeneratorError::ConfigError(format!("OpenAPI export failed: {}", e)))?;

    // Serialize to string
    let content = serialize(&doc, options.format)
        .map_err(|e| GeneratorError::ConfigError(format!("OpenAPI serialization failed: {}", e)))?;

    // Determine file extension
    let extension = match options.format {
        ExportFormat::Json => "json",
        ExportFormat::Yaml => "yaml",
    };

    // Build file path: {module_name}.{extension}
    // Filename derived from resolved module name, NOT api.name — keeps
    // OpenAPI artifacts aligned with the generated Rust module layout.
    let module_name = resolve_module_name(api);
    let file_name = format!("{}.{}", module_name, extension);
    let file_path = dir.join(&file_name);

    // Write the file
    fs::write(&file_path, content).map_err(|source| GeneratorError::WriteError {
        path: file_path.display().to_string(),
        source,
    })?;

    Ok(file_path)
}

/// Writes a module-grouped OpenAPI specification to disk.
///
/// Merges every `RestApi` in `apis` into a single OpenAPI 3.0.3 document
/// named `<module_name>.<ext>` (where `<ext>` is `json` or `yaml` per
/// `options.format`). The merge semantics are:
///
/// - **info**: seeded from the first API; `info.title` is overridden with
///   `module_name` (informational — it is not a contract).
/// - **servers / external_docs**: taken from the first API.
/// - **paths**: unioned across all members. If two members emit the same
///   `path` + `method`, the writer fails with [`GeneratorError::ConfigError`]
///   identifying both contributing API names.
/// - **components.schemas**: unioned via the merged `registry` (callers
///   should pass the registry returned by
///   [`schematic_definitions::registry::get_registries_for_module`]).
/// - **components.security_schemes**: unioned across members. If two members
///   define the same scheme name with different security-scheme content, the
///   second is renamed to `<scheme>_<api_snake>` and that API's per-operation
///   `security` requirements are rewritten to reference the renamed scheme.
/// - **security**: top-level requirements unioned across members,
///   deduplicated while preserving insertion order.
/// - **tags**: unioned across members (currently always empty in practice).
/// - **extensions**: the first API's `x-schematic` value is preserved, and a
///   sibling `x-schematic-grouped-from` extension is added that lists every
///   contributing API name so the artifact is self-describing.
///
/// ## Returns
///
/// The path of the written file.
///
/// ## Errors
///
/// - [`GeneratorError::ConfigError`] when the directory does not exist, when
///   per-API export or serialization fails, or when two member APIs collide
///   on the same `path` + `method`.
/// - [`GeneratorError::WriteError`] when the file cannot be written.
///
/// ## Panics
///
/// Panics if `apis` is empty — callers must supply at least one API.
pub fn write_openapi_grouped<R: SchemaRegistryLike>(
    apis: &[&RestApi],
    module_name: &str,
    registry: &R,
    options: &ExportOptions,
    dir: &Path,
) -> Result<PathBuf, GeneratorError> {
    assert!(
        !apis.is_empty(),
        "write_openapi_grouped requires at least one API"
    );

    if !dir.exists() {
        return Err(GeneratorError::OutputDirNotFound(dir.display().to_string()));
    }

    // Export each member API independently. The first document seeds the
    // merged result; subsequent documents contribute their paths, schemes,
    // and security requirements.
    let mut docs: Vec<(String, OpenAPI)> = Vec::with_capacity(apis.len());
    for api in apis {
        let doc = export(api, registry, options).map_err(|e| {
            GeneratorError::ConfigError(format!(
                "OpenAPI export failed for API '{}': {}",
                api.name, e
            ))
        })?;
        docs.push((api.name.clone(), doc));
    }

    // Seed the merged document from the first API, then override the title
    // with the module name and union the rest of the fields below.
    let (seed_name, mut merged) = docs.remove(0);
    merged.info.title = module_name.to_string();

    // Track which API contributed each (path, method) so we can produce
    // helpful error messages on collision and rewrite security schemes per
    // owner during merging.
    let mut path_method_owners: IndexMap<(String, &'static str), String> = IndexMap::new();
    for (path, item_ref) in &merged.paths.paths {
        if let ReferenceOr::Item(item) = item_ref {
            for (method, _op) in path_methods_present(item) {
                path_method_owners.insert((path.clone(), method), seed_name.clone());
            }
        }
    }

    // Track contributors for the grouped extension — start with the seed.
    let mut contributors: Vec<String> = vec![seed_name];

    for (api_name, mut doc) in docs {
        contributors.push(api_name.clone());

        // ---- Merge paths ----
        for (path, item_ref) in std::mem::take(&mut doc.paths.paths) {
            // Union path-level path-parameters when an entry already exists,
            // otherwise insert wholesale.
            match merged.paths.paths.get_mut(&path) {
                Some(ReferenceOr::Item(existing)) => {
                    if let ReferenceOr::Item(incoming) = item_ref {
                        // Detect method-level collisions. Two member APIs may
                        // legitimately expose the same `METHOD path` when only
                        // their auth differs (e.g. EMQX Basic + Bearer cover
                        // the same management surface). If the operations are
                        // structurally equivalent under JSON, union their
                        // security requirements; otherwise treat as a hard
                        // collision.
                        for (method, present) in path_methods_iter(&incoming) {
                            if !present {
                                continue;
                            }
                            let existing_op = method_slot(existing, method);
                            let incoming_op = method_slot(&incoming, method);
                            if let (Some(eo), Some(io)) = (existing_op, incoming_op) {
                                if operations_compatible(eo, io) {
                                    // Compatible — security requirements will
                                    // be merged below.
                                    continue;
                                }
                                let prev_owner = path_method_owners
                                    .get(&(path.clone(), method))
                                    .cloned()
                                    .unwrap_or_else(|| seed_name_for_collision(&contributors));
                                return Err(GeneratorError::ConfigError(format!(
                                    "OpenAPI path/method collision while merging module \
                                     '{}': {} {} is defined by both '{}' and '{}'",
                                    module_name, method, path, prev_owner, api_name,
                                )));
                            }
                        }

                        // Move method operations from incoming into existing,
                        // or union security requirements when the existing
                        // operation is structurally equivalent.
                        merge_method(&mut existing.get, incoming.get);
                        merge_method(&mut existing.post, incoming.post);
                        merge_method(&mut existing.put, incoming.put);
                        merge_method(&mut existing.patch, incoming.patch);
                        merge_method(&mut existing.delete, incoming.delete);
                        merge_method(&mut existing.head, incoming.head);
                        merge_method(&mut existing.options, incoming.options);
                        merge_method(&mut existing.trace, incoming.trace);

                        // Append any new path-level parameters.
                        existing.parameters.extend(incoming.parameters);

                        for (method, present) in path_methods_iter(existing) {
                            if present {
                                path_method_owners
                                    .entry((path.clone(), method))
                                    .or_insert_with(|| api_name.clone());
                            }
                        }
                    }
                }
                Some(ReferenceOr::Reference { .. }) | None => {
                    if let ReferenceOr::Item(ref item) = item_ref {
                        for (method, present) in path_methods_iter(item) {
                            if present {
                                path_method_owners.insert((path.clone(), method), api_name.clone());
                            }
                        }
                    }
                    merged.paths.paths.insert(path, item_ref);
                }
            }
        }

        // ---- Merge components ----
        let merged_components = merged.components.get_or_insert_with(Default::default);
        if let Some(incoming_components) = doc.components {
            // Schemas: IndexMap insert overwrites; identical schemas collapse.
            for (name, schema) in incoming_components.schemas {
                merged_components.schemas.insert(name, schema);
            }

            // Security schemes: rename on conflict, rewrite operation
            // security requirements that previously referenced the original.
            let mut renames: IndexMap<String, String> = IndexMap::new();
            for (name, scheme) in incoming_components.security_schemes {
                match merged_components.security_schemes.get(&name) {
                    Some(existing) => {
                        if security_schemes_equal(existing, &scheme) {
                            // Same scheme by content — drop the duplicate.
                            continue;
                        }
                        let renamed = format!("{}_{}", name, snake_case(&api_name));
                        renames.insert(name.clone(), renamed.clone());
                        merged_components.security_schemes.insert(renamed, scheme);
                    }
                    None => {
                        merged_components.security_schemes.insert(name, scheme);
                    }
                }
            }

            // Apply renames to the operations of the incoming document — but
            // its paths are already merged into `merged`, so rewrite there.
            // Since path collisions failed earlier, only operations belonging
            // to this `api_name` need rewriting; identifying them requires
            // walking every operation owned by `api_name`. The simplest
            // correct approach is to rewrite all operations on paths that
            // path_method_owners attributes to `api_name`.
            if !renames.is_empty() {
                for ((path, _method), owner) in &path_method_owners {
                    if owner != &api_name {
                        continue;
                    }
                    if let Some(ReferenceOr::Item(item)) = merged.paths.paths.get_mut(path) {
                        rewrite_path_item_security(item, &renames);
                    }
                }
            }
        }

        // ---- Merge top-level security requirements (deduplicated) ----
        if let Some(incoming_sec) = doc.security {
            let bucket = merged.security.get_or_insert_with(Vec::new);
            for req in incoming_sec {
                if !bucket.iter().any(|existing| existing == &req) {
                    bucket.push(req);
                }
            }
        }

        // ---- Merge tags (preserve insertion order, dedupe by name) ----
        for tag in doc.tags {
            if !merged.tags.iter().any(|t| t.name == tag.name) {
                merged.tags.push(tag);
            }
        }
    }

    // Record contributors as a sibling extension for self-description.
    if !options.skip_extensions {
        let value = serde_json::Value::Array(
            contributors
                .iter()
                .map(|name| serde_json::Value::String(name.clone()))
                .collect(),
        );
        merged
            .extensions
            .insert("x-schematic-grouped-from".to_string(), value);
    }

    // Serialize and write.
    let content = serialize(&merged, options.format)
        .map_err(|e| GeneratorError::ConfigError(format!("OpenAPI serialization failed: {}", e)))?;
    let extension = match options.format {
        ExportFormat::Json => "json",
        ExportFormat::Yaml => "yaml",
    };
    let file_path = dir.join(format!("{}.{}", module_name, extension));
    fs::write(&file_path, content).map_err(|source| GeneratorError::WriteError {
        path: file_path.display().to_string(),
        source,
    })?;

    Ok(file_path)
}

/// Returns the operation slot for a given HTTP method label.
fn method_slot<'a>(
    item: &'a openapiv3::PathItem,
    method: &str,
) -> Option<&'a openapiv3::Operation> {
    match method {
        "GET" => item.get.as_ref(),
        "POST" => item.post.as_ref(),
        "PUT" => item.put.as_ref(),
        "PATCH" => item.patch.as_ref(),
        "DELETE" => item.delete.as_ref(),
        "HEAD" => item.head.as_ref(),
        "OPTIONS" => item.options.as_ref(),
        "TRACE" => item.trace.as_ref(),
        _ => None,
    }
}

/// Returns `true` when two operations are structurally equivalent ignoring
/// per-operation `security` requirements.
///
/// Two member APIs in the same module may legitimately expose the same
/// endpoint differing only in their auth strategy (the EMQX Basic + Bearer
/// pair is the canonical example). When that happens, the merged operation
/// keeps the seed operation and the security requirements from both
/// contributors are unioned by [`merge_method`].
fn operations_compatible(a: &openapiv3::Operation, b: &openapiv3::Operation) -> bool {
    let mut a_clone = a.clone();
    let mut b_clone = b.clone();
    a_clone.security = None;
    b_clone.security = None;
    match (
        serde_json::to_value(&a_clone).ok(),
        serde_json::to_value(&b_clone).ok(),
    ) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

/// Merges an incoming operation into an existing operation slot.
///
/// - If the slot is empty, the incoming operation is moved in wholesale.
/// - If both are present and structurally equivalent
///   (see [`operations_compatible`]), the incoming security requirements are
///   unioned into the existing operation, deduplicated while preserving order.
/// - Otherwise the incoming operation is dropped (the caller has already
///   verified compatibility before calling this function).
fn merge_method(
    existing: &mut Option<openapiv3::Operation>,
    incoming: Option<openapiv3::Operation>,
) {
    let Some(incoming_op) = incoming else {
        return;
    };
    match existing.as_mut() {
        None => {
            *existing = Some(incoming_op);
        }
        Some(existing_op) => {
            if let Some(incoming_security) = incoming_op.security {
                let bucket = existing_op.security.get_or_insert_with(Vec::new);
                for req in incoming_security {
                    if !bucket.iter().any(|present| present == &req) {
                        bucket.push(req);
                    }
                }
            }
        }
    }
}

/// Picks a reasonable previous-owner label when none has been recorded for the
/// colliding (path, method). Falls back to the seed name when contributors is
/// non-empty.
fn seed_name_for_collision(contributors: &[String]) -> String {
    contributors
        .first()
        .cloned()
        .unwrap_or_else(|| "<unknown>".to_string())
}

/// Returns presence flags for each HTTP method on a path item.
fn path_methods_iter(
    item: &openapiv3::PathItem,
) -> impl IntoIterator<Item = (&'static str, bool)> + '_ {
    [
        ("GET", item.get.is_some()),
        ("POST", item.post.is_some()),
        ("PUT", item.put.is_some()),
        ("PATCH", item.patch.is_some()),
        ("DELETE", item.delete.is_some()),
        ("HEAD", item.head.is_some()),
        ("OPTIONS", item.options.is_some()),
        ("TRACE", item.trace.is_some()),
    ]
}

/// Returns each method label that has an attached operation.
fn path_methods_present(
    item: &openapiv3::PathItem,
) -> impl IntoIterator<Item = (&'static str, &openapiv3::Operation)> + '_ {
    let mut out: Vec<(&'static str, &openapiv3::Operation)> = Vec::new();
    if let Some(op) = item.get.as_ref() {
        out.push(("GET", op));
    }
    if let Some(op) = item.post.as_ref() {
        out.push(("POST", op));
    }
    if let Some(op) = item.put.as_ref() {
        out.push(("PUT", op));
    }
    if let Some(op) = item.patch.as_ref() {
        out.push(("PATCH", op));
    }
    if let Some(op) = item.delete.as_ref() {
        out.push(("DELETE", op));
    }
    if let Some(op) = item.head.as_ref() {
        out.push(("HEAD", op));
    }
    if let Some(op) = item.options.as_ref() {
        out.push(("OPTIONS", op));
    }
    if let Some(op) = item.trace.as_ref() {
        out.push(("TRACE", op));
    }
    out
}

/// Compares two security schemes for content equality via JSON serialization.
///
/// `openapiv3::SecurityScheme` does not implement `PartialEq`, so we compare
/// the JSON form. Equal serialization means identical scheme definitions for
/// merge purposes.
fn security_schemes_equal(
    a: &ReferenceOr<openapiv3::SecurityScheme>,
    b: &ReferenceOr<openapiv3::SecurityScheme>,
) -> bool {
    match (serde_json::to_value(a).ok(), serde_json::to_value(b).ok()) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

/// Rewrites operation-level security references on every operation of a path
/// item using the supplied rename map.
fn rewrite_path_item_security(item: &mut openapiv3::PathItem, renames: &IndexMap<String, String>) {
    let mut ops: [&mut Option<openapiv3::Operation>; 8] = [
        &mut item.get,
        &mut item.post,
        &mut item.put,
        &mut item.patch,
        &mut item.delete,
        &mut item.head,
        &mut item.options,
        &mut item.trace,
    ];
    for op_slot in ops.iter_mut() {
        if let Some(op) = op_slot.as_mut()
            && let Some(security) = op.security.as_mut()
        {
            for req in security.iter_mut() {
                let mut renamed_req = openapiv3::SecurityRequirement::new();
                for (name, scopes) in req.iter() {
                    let new_name = renames.get(name).cloned().unwrap_or_else(|| name.clone());
                    renamed_req.insert(new_name, scopes.clone());
                }
                *req = renamed_req;
            }
        }
    }
}

/// Converts an arbitrary identifier-like string to `snake_case`.
///
/// Used to produce stable disambiguation suffixes for security-scheme renames
/// (e.g. `OllamaNative` → `ollama_native`).
fn snake_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    let mut prev_is_lower_or_digit = false;
    for ch in input.chars() {
        if ch.is_ascii_uppercase() {
            if prev_is_lower_or_digit {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_is_lower_or_digit = false;
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            out.push('_');
            prev_is_lower_or_digit = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_file::serde_yaml_ng;
    use schematic_define::openapi::ExportFormat;
    use schematic_definitions::openai::{define_openai_api, openapi_registry};
    use tempfile::TempDir;

    // =============================================
    // write_openapi() tests
    // =============================================

    #[test]
    fn write_openapi_creates_json_file() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("1.0.0");
        let temp_dir = TempDir::new().unwrap();

        let result = write_openapi(&api, &registry, &options, temp_dir.path());
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "openai.json");
    }

    #[test]
    fn write_openapi_creates_yaml_file() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new()
            .with_version("1.0.0")
            .with_format(ExportFormat::Yaml);
        let temp_dir = TempDir::new().unwrap();

        let result = write_openapi(&api, &registry, &options, temp_dir.path());
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "openai.yaml");
    }

    #[test]
    fn write_openapi_json_produces_valid_json() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("1.0.0");
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Should be parseable as JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
        assert!(parsed.is_ok(), "Should produce valid JSON");

        let doc = parsed.unwrap();
        assert_eq!(doc["openapi"], "3.0.3");
        assert_eq!(doc["info"]["title"], "OpenAI");
    }

    #[test]
    fn write_openapi_yaml_produces_valid_yaml() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new()
            .with_version("1.0.0")
            .with_format(ExportFormat::Yaml);
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Should be parseable as YAML
        let parsed: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(&content);
        assert!(parsed.is_ok(), "Should produce valid YAML");
    }

    #[test]
    fn write_openapi_file_name_lowercase() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();

        // API name is "OpenAI" with module_path = None, so the filename falls
        // back to the lowercased name and produces "openai.json".
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "openai.json");
    }

    #[test]
    fn write_openapi_filename_uses_module_path_when_set() {
        use schematic_define::{AuthStrategy, RestMethod};

        // module_path = "foo_bar" must produce "foo_bar.json", NOT
        // "<api.name>.json" or anything derived from api.name.
        let api = synthetic_api(
            "FooBarApi",
            "foo_bar",
            RestMethod::Get,
            "/x",
            AuthStrategy::None,
        );
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &EmptyRegistry, &ExportOptions::new(), temp_dir.path())
            .expect("write_openapi should succeed");

        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "foo_bar.json",
            "filename must derive from module_path, not api.name"
        );
    }

    #[test]
    fn write_openapi_error_on_missing_directory() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();

        let result = write_openapi(&api, &registry, &options, Path::new("/nonexistent/path"));
        assert!(result.is_err());

        match result {
            Err(GeneratorError::OutputDirNotFound(path)) => {
                assert!(path.contains("nonexistent"));
            }
            _ => panic!("Expected OutputDirNotFound error"),
        }
    }

    #[test]
    fn write_openapi_respects_version_option() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("2.5.0");
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(doc["info"]["version"], "2.5.0");
    }

    #[test]
    fn write_openapi_includes_schemas() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify schemas are included
        assert!(doc["components"]["schemas"]["Model"].is_object());
        assert!(doc["components"]["schemas"]["ListModelsResponse"].is_object());
    }

    #[test]
    fn write_openapi_can_skip_extensions() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().skip_extensions();
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();

        // x-schematic should not be present
        assert!(doc.get("x-schematic").is_none());
    }

    // =============================================
    // write_openapi_grouped() tests
    // =============================================

    /// Synthetic single-endpoint REST API used by grouped tests.
    fn synthetic_api(
        name: &str,
        module: &str,
        method: schematic_define::RestMethod,
        path: &str,
        auth: schematic_define::AuthStrategy,
    ) -> schematic_define::RestApi {
        use schematic_define::{ApiResponse, Endpoint, RestApi};

        RestApi {
            name: name.to_string(),
            description: format!("{} synthetic API", name),
            base_url: format!("https://{}.example.com", name.to_lowercase()),
            docs_url: None,
            auth,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: format!("{}Endpoint", name),
                method,
                path: path.to_string(),
                description: format!("Endpoint for {}", name),
                request: None,
                response: ApiResponse::Empty,
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: Some(module.to_string()),
            request_suffix: None,
            version: None,
            env_mapping: None,
        }
    }

    /// Empty registry: synthetic APIs use `ApiResponse::Empty` so no schemas
    /// need to be registered for these tests.
    struct EmptyRegistry;
    impl SchemaRegistryLike for EmptyRegistry {
        fn to_openapi_schemas(&self) -> indexmap::IndexMap<String, openapiv3::Schema> {
            indexmap::IndexMap::new()
        }
    }

    #[test]
    fn write_openapi_grouped_creates_module_named_file() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/a", AuthStrategy::None);
        let b = synthetic_api("FooB", "foo", RestMethod::Get, "/b", AuthStrategy::None);
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&a, &b],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new(),
            temp.path(),
        )
        .unwrap();

        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "foo.json");
        assert!(path.exists());
    }

    #[test]
    fn write_openapi_grouped_unions_paths() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/a", AuthStrategy::None);
        let b = synthetic_api("FooB", "foo", RestMethod::Get, "/b", AuthStrategy::None);
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&a, &b],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new(),
            temp.path(),
        )
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let doc: openapiv3::OpenAPI = serde_json::from_str(&content).unwrap();
        let path_keys: Vec<&str> = doc.paths.paths.keys().map(String::as_str).collect();
        assert!(
            path_keys.contains(&"/a"),
            "expected /a, got {:?}",
            path_keys
        );
        assert!(
            path_keys.contains(&"/b"),
            "expected /b, got {:?}",
            path_keys
        );
    }

    #[test]
    fn write_openapi_grouped_collides_on_duplicate_method_path() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/dup", AuthStrategy::None);
        let b = synthetic_api("FooB", "foo", RestMethod::Get, "/dup", AuthStrategy::None);
        let temp = TempDir::new().unwrap();

        let err = write_openapi_grouped(
            &[&a, &b],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new(),
            temp.path(),
        )
        .expect_err("should fail on path/method collision");

        match err {
            GeneratorError::ConfigError(msg) => {
                assert!(msg.contains("FooA"), "msg should name FooA: {msg}");
                assert!(msg.contains("FooB"), "msg should name FooB: {msg}");
                assert!(msg.contains("/dup"));
                assert!(msg.contains("GET"));
            }
            other => panic!("expected ConfigError, got {other:?}"),
        }
    }

    #[test]
    fn write_openapi_grouped_unions_security_schemes_distinct() {
        use schematic_define::{AuthStrategy, RestMethod};

        // Bearer (HTTP) vs Basic (HTTP) — distinct scheme names.
        let bearer_api = synthetic_api(
            "FooBearer",
            "foo",
            RestMethod::Get,
            "/bear",
            AuthStrategy::BearerToken { header: None },
        );
        let basic_api = synthetic_api(
            "FooBasic",
            "foo",
            RestMethod::Get,
            "/basic",
            AuthStrategy::Basic,
        );
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&bearer_api, &basic_api],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new(),
            temp.path(),
        )
        .unwrap();
        let doc: openapiv3::OpenAPI =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let schemes = doc.components.expect("components").security_schemes;
        assert!(schemes.contains_key("bearerAuth"));
        assert!(schemes.contains_key("basicAuth"));
    }

    #[test]
    fn write_openapi_grouped_uses_module_name_in_info_title() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/a", AuthStrategy::None);
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&a],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new(),
            temp.path(),
        )
        .unwrap();

        let doc: openapiv3::OpenAPI =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(doc.info.title, "foo");
    }

    #[test]
    fn write_openapi_grouped_emits_grouped_extension() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/a", AuthStrategy::None);
        let b = synthetic_api("FooB", "foo", RestMethod::Get, "/b", AuthStrategy::None);
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&a, &b],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new(),
            temp.path(),
        )
        .unwrap();

        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let contributors = value
            .get("x-schematic-grouped-from")
            .and_then(|v| v.as_array())
            .expect("x-schematic-grouped-from array");
        let names: Vec<&str> = contributors.iter().filter_map(|v| v.as_str()).collect();
        assert!(names.contains(&"FooA"));
        assert!(names.contains(&"FooB"));
    }

    #[test]
    fn write_openapi_grouped_skip_extensions_omits_grouped_marker() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/a", AuthStrategy::None);
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&a],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new().skip_extensions(),
            temp.path(),
        )
        .unwrap();

        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(value.get("x-schematic-grouped-from").is_none());
        assert!(value.get("x-schematic").is_none());
    }

    #[test]
    fn write_openapi_grouped_yaml_round_trips() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/a", AuthStrategy::None);
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&a],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new().with_format(ExportFormat::Yaml),
            temp.path(),
        )
        .unwrap();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "foo.yaml");

        let content = fs::read_to_string(&path).unwrap();
        let doc: openapiv3::OpenAPI = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(doc.info.title, "foo");
    }

    #[test]
    fn write_openapi_grouped_real_ollama_module() {
        use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};
        use schematic_definitions::registry::get_registries_for_module;

        let native = define_ollama_native_api();
        let openai = define_ollama_openai_api();
        let registry = get_registries_for_module("ollama").expect("ollama registry");
        let temp = TempDir::new().unwrap();

        let path = write_openapi_grouped(
            &[&native, &openai],
            "ollama",
            &registry,
            &ExportOptions::new(),
            temp.path(),
        )
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let doc: openapiv3::OpenAPI = serde_json::from_str(&content).unwrap();
        assert!(
            doc.info.title.to_lowercase().contains("ollama"),
            "info.title should mention ollama: {}",
            doc.info.title,
        );

        // Each member API contributes at least one path.
        let path_keys: std::collections::BTreeSet<&str> =
            doc.paths.paths.keys().map(String::as_str).collect();
        let native_paths: Vec<&str> = native.endpoints.iter().map(|e| e.path.as_str()).collect();
        let openai_paths: Vec<&str> = openai.endpoints.iter().map(|e| e.path.as_str()).collect();
        assert!(
            native_paths.iter().any(|p| path_keys.contains(p)),
            "expected at least one native path",
        );
        assert!(
            openai_paths.iter().any(|p| path_keys.contains(p)),
            "expected at least one openai-compatible path",
        );

        // Schemas must be non-empty (registries provided).
        let schemas = doc.components.expect("components present").schemas;
        assert!(!schemas.is_empty(), "components.schemas must not be empty");
    }

    #[test]
    fn write_openapi_grouped_error_on_missing_directory() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = synthetic_api("FooA", "foo", RestMethod::Get, "/a", AuthStrategy::None);
        let err = write_openapi_grouped(
            &[&a],
            "foo",
            &EmptyRegistry,
            &ExportOptions::new(),
            Path::new("/nonexistent/never/here"),
        )
        .expect_err("missing dir should fail");
        assert!(matches!(err, GeneratorError::OutputDirNotFound(_)));
    }

    // =============================================
    // snake_case() unit tests
    // =============================================

    #[test]
    fn snake_case_handles_camel_and_pascal_case() {
        // Word-boundary detection keys off lower-or-digit -> upper.
        // Runs of uppercase letters collapse together (e.g. "HTTP" stays).
        assert_eq!(snake_case("OllamaNative"), "ollama_native");
        assert_eq!(snake_case("FooBar"), "foo_bar");
        assert_eq!(snake_case("foo"), "foo");
        assert_eq!(snake_case("HTTPServer"), "httpserver");
        assert_eq!(snake_case("foo-bar"), "foo_bar");
    }
}
