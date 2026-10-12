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

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use openapiv3::{LinkOperation, OpenAPI, ReferenceOr};
use schematic_define::RestApi;
use schematic_define::openapi::{
    ExportFormat, ExportOptions, SchemaRegistryLike, export, serialize,
};

use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::errors::GeneratorError;
use crate::export::resolve_module_name;
use crate::validation::{ensure_no_errors, validate_openapi};

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
/// Returns `GeneratorError::Validation` if [`validate_openapi`] reports an
/// error or the exported document fails [`operation_id_diagnostics`];
/// nothing is written.
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
    ensure_no_errors(Output::OpenApi, validate_openapi(&[api]))?;

    if !dir.exists() {
        return Err(GeneratorError::OutputDirNotFound(dir.display().to_string()));
    }

    let doc = export(api, registry, options)
        .map_err(|e| GeneratorError::ConfigError(format!("OpenAPI export failed: {}", e)))?;
    ensure_no_errors(
        Output::OpenApi,
        operation_id_diagnostics(&doc, &[api.name.as_str()]),
    )?;

    // Filename derived from resolved module name, NOT api.name — keeps
    // OpenAPI artifacts aligned with the generated Rust module layout.
    write_openapi_document(&doc, &resolve_module_name(api), options.format, dir)
}

/// Writes a module-grouped OpenAPI specification to disk.
///
/// Builds the document with [`build_openapi_grouped`] and writes it as
/// `<module_name>.<ext>` (where `<ext>` is `json` or `yaml` per
/// `options.format`).
///
/// ## Returns
///
/// The path of the written file.
///
/// ## Errors
///
/// - Every error of [`build_openapi_grouped`]; nothing is written.
/// - [`GeneratorError::OutputDirNotFound`] when the directory does not exist.
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
    let doc = build_openapi_grouped(apis, module_name, registry, options)?;
    write_openapi_document(&doc, module_name, options.format, dir)
}

/// Builds the OpenAPI document of one module without writing it.
///
/// Merges every `RestApi` in `apis` into a single OpenAPI 3.0.3 document. The
/// merge semantics are:
///
/// - **info**: seeded from the first API; `info.title` is overridden with
///   `module_name` (informational — it is not a contract).
/// - **servers / external_docs**: taken from the first API.
/// - **paths**: unioned across all members. Two members may declare the same
///   `path` + `method` only when the operations are equal apart from their
///   `security` (the EMQX Basic + Bearer pair); the merged operation then
///   lists both security alternatives. Any other overlap is an OpenAPI-only
///   `duplicate-method-path` error naming both APIs.
/// - **operationId**: unique across the document. The first member in
///   registry order keeps an id; a later member's clashing id is prefixed
///   with its API name in PascalCase (`OllamaOpenAIListModels`), plus a
///   numeric suffix if that name is taken or is any operation's original id.
///   Links that name a renamed operation are updated for the API that wrote
///   them.
/// - **components.schemas**: unioned via the merged `registry` (callers
///   should pass the registry returned by
///   [`schematic_definitions::registry::get_registries_for_module`]).
/// - **components.security_schemes**: unioned across members. If two members
///   define the same scheme name with different security-scheme content, the
///   second is renamed to `<scheme>_<api_snake>` and that API's operation and
///   top-level `security` requirements reference the renamed scheme.
/// - **security**: top-level requirements unioned across members,
///   deduplicated while preserving insertion order.
/// - **tags**: unioned across members (currently always empty in practice).
/// - **extensions**: the first API's `x-schematic` value is preserved, and a
///   sibling `x-schematic-grouped-from` extension is added that lists every
///   contributing API name so the artifact is self-describing.
///
/// The finished document passes [`operation_id_diagnostics`].
///
/// ## Errors
///
/// - [`GeneratorError::Validation`] when [`validate_openapi`] reports an
///   error for any member, when two members declare incompatible operations
///   on one `path` + `method`, or when the finished document fails
///   [`operation_id_diagnostics`].
/// - [`GeneratorError::ConfigError`] when per-API export fails.
///
/// ## Panics
///
/// Panics if `apis` is empty — callers must supply at least one API.
pub fn build_openapi_grouped<R: SchemaRegistryLike>(
    apis: &[&RestApi],
    module_name: &str,
    registry: &R,
    options: &ExportOptions,
) -> Result<OpenAPI, GeneratorError> {
    assert!(
        !apis.is_empty(),
        "build_openapi_grouped requires at least one API"
    );
    ensure_no_errors(Output::OpenApi, validate_openapi(apis))?;

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

    let merged = merge_documents(module_name, docs, !options.skip_extensions)?;
    let api_names: Vec<&str> = apis.iter().map(|api| api.name.as_str()).collect();
    ensure_no_errors(
        Output::OpenApi,
        operation_id_diagnostics(&merged, &api_names),
    )?;
    Ok(merged)
}

/// Serializes `doc` exactly as [`write_openapi_document`] writes it.
///
/// ## Errors
///
/// [`GeneratorError::ConfigError`] when serialization fails.
pub fn render_openapi_document(doc: &OpenAPI, format: ExportFormat) -> Result<String, GeneratorError> {
    serialize(doc, format)
        .map_err(|e| GeneratorError::ConfigError(format!("OpenAPI serialization failed: {}", e)))
}

/// Serializes `doc` and writes it to `<dir>/<file_stem>.<ext>`.
///
/// ## Errors
///
/// - [`GeneratorError::OutputDirNotFound`] when `dir` does not exist.
/// - [`GeneratorError::ConfigError`] when serialization fails.
/// - [`GeneratorError::WriteError`] when the file cannot be written.
pub fn write_openapi_document(
    doc: &OpenAPI,
    file_stem: &str,
    format: ExportFormat,
    dir: &Path,
) -> Result<PathBuf, GeneratorError> {
    if !dir.exists() {
        return Err(GeneratorError::OutputDirNotFound(dir.display().to_string()));
    }
    let content = render_openapi_document(doc, format)?;
    let extension = match format {
        ExportFormat::Json => "json",
        ExportFormat::Yaml => "yaml",
    };
    let file_path = dir.join(format!("{}.{}", file_stem, extension));
    fs::write(&file_path, content).map_err(|source| GeneratorError::WriteError {
        path: file_path.display().to_string(),
        source,
    })?;
    Ok(file_path)
}

/// Reports every `operationId` integrity defect of one finished document.
///
/// Two operations sharing an `operationId` are a `duplicate-operation-id`
/// error; a link naming an `operationId` that no operation declares is an
/// `unresolved-link-operation` error. `apis` names the APIs the document was
/// built from and scopes every diagnostic. Grouped and per-API documents are
/// checked by this one scan before they are written.
pub fn operation_id_diagnostics(doc: &OpenAPI, apis: &[&str]) -> Vec<ValidationDiagnostic> {
    let mut declared: IndexMap<&str, Vec<String>> = IndexMap::new();
    let mut links: Vec<(String, &str)> = Vec::new();
    for (path, item_ref) in &doc.paths.paths {
        let ReferenceOr::Item(item) = item_ref else {
            continue;
        };
        for (method, op) in path_methods_present(item) {
            let route = format!("{method} {path}");
            if let Some(id) = op.operation_id.as_deref() {
                declared.entry(id).or_default().push(route.clone());
            }
            for (name, target) in response_link_targets(&op.responses) {
                links.push((format!("'{name}' of {route}"), target));
            }
        }
    }
    if let Some(components) = &doc.components {
        for (name, link) in &components.links {
            if let ReferenceOr::Item(link) = link
                && let LinkOperation::OperationId(target) = &link.operation
            {
                links.push((format!("'{name}' of components.links"), target));
            }
        }
        for (response_name, response) in &components.responses {
            if let ReferenceOr::Item(response) = response {
                for (name, target) in link_targets(&response.links) {
                    links.push((
                        format!("'{name}' of components.responses.{response_name}"),
                        target,
                    ));
                }
            }
        }
    }

    let mut diagnostics = Vec::new();
    for (id, routes) in &declared {
        if routes.len() > 1 {
            diagnostics.push(ValidationDiagnostic::for_group(
                Severity::Error,
                apis.iter().copied(),
                Feature::DuplicateOperationId,
                Output::OpenApi,
                format!(
                    "operationId '{id}' is declared by {}; every operationId in a document \
                     must be unique",
                    routes.join(" and ")
                ),
            ));
        }
    }
    for (location, target) in links {
        if !declared.contains_key(target) {
            diagnostics.push(ValidationDiagnostic::for_group(
                Severity::Error,
                apis.iter().copied(),
                Feature::UnresolvedLinkOperation,
                Output::OpenApi,
                format!(
                    "link {location} names operationId '{target}', which no operation \
                     in the document declares"
                ),
            ));
        }
    }
    diagnostics
}

/// Merges per-API documents into one module document.
///
/// `docs` is in registry order; the first document seeds the result. See
/// [`build_openapi_grouped`] for the merge semantics.
fn merge_documents(
    module_name: &str,
    docs: Vec<(String, OpenAPI)>,
    record_contributors: bool,
) -> Result<OpenAPI, GeneratorError> {
    let mut docs = docs.into_iter();
    let Some((seed_name, mut merged)) = docs.next() else {
        return Err(GeneratorError::ConfigError(format!(
            "no OpenAPI document to merge for module '{module_name}'"
        )));
    };
    merged.info.title = module_name.to_string();

    // Every operation of the merged document in first-owner order (API order,
    // then the owner's path order, then the fixed method order), with the
    // indexes of the APIs that contributed it. The first index owns the
    // operation: its document supplied the operation and its links.
    let mut operations: IndexMap<(String, &'static str), Vec<usize>> = IndexMap::new();
    for (path, item_ref) in &merged.paths.paths {
        if let ReferenceOr::Item(item) = item_ref {
            for (method, _op) in path_methods_present(item) {
                operations.insert((path.clone(), method), vec![0]);
            }
        }
    }
    let mut contributors: Vec<String> = vec![seed_name];

    for (api_name, mut doc) in docs {
        let api_index = contributors.len();
        contributors.push(api_name.clone());

        // ---- Merge components ----
        // Security schemes merge before paths so this document's renamed
        // scheme references are rewritten while its operations are still its
        // own; afterwards a compatible operation shares a slot with another
        // API's.
        let merged_components = merged.components.get_or_insert_with(Default::default);
        if let Some(incoming_components) = doc.components.take() {
            // Schemas: IndexMap insert overwrites; identical schemas collapse.
            for (name, schema) in incoming_components.schemas {
                merged_components.schemas.insert(name, schema);
            }

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

            if !renames.is_empty() {
                for item_ref in doc.paths.paths.values_mut() {
                    if let ReferenceOr::Item(item) = item_ref {
                        rewrite_path_item_security(item, &renames);
                    }
                }
                if let Some(security) = doc.security.as_mut() {
                    rename_security_requirements(security, &renames);
                }
            }
        }

        // ---- Merge paths ----
        for (path, item_ref) in std::mem::take(&mut doc.paths.paths) {
            match merged.paths.paths.get_mut(&path) {
                Some(ReferenceOr::Item(existing)) => {
                    if let ReferenceOr::Item(incoming) = item_ref {
                        for (method, incoming_op) in path_methods_present(&incoming) {
                            if let Some(existing_op) = method_slot(existing, method)
                                && !operations_compatible(existing_op, incoming_op)
                            {
                                let owner = operations
                                    .get(&(path.clone(), method))
                                    .and_then(|apis| apis.first())
                                    .map_or(&contributors[0], |&index| &contributors[index]);
                                return Err(GeneratorError::Validation {
                                    output: Output::OpenApi,
                                    diagnostics: vec![ValidationDiagnostic::for_group(
                                        Severity::Error,
                                        [owner.as_str(), api_name.as_str()],
                                        Feature::DuplicateMethodPath,
                                        Output::OpenApi,
                                        format!(
                                            "module '{module_name}': {method} {path} is \
                                             declared by both APIs with different operations; \
                                             an OpenAPI document holds one operation per \
                                             method and path"
                                        ),
                                    )],
                                });
                            }
                        }
                        for (method, _op) in path_methods_present(&incoming) {
                            operations
                                .entry((path.clone(), method))
                                .or_default()
                                .push(api_index);
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
                    }
                }
                Some(ReferenceOr::Reference { .. }) | None => {
                    if let ReferenceOr::Item(item) = &item_ref {
                        for (method, _op) in path_methods_present(item) {
                            operations.insert((path.clone(), method), vec![api_index]);
                        }
                    }
                    merged.paths.paths.insert(path, item_ref);
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

    assign_operation_ids(&mut merged, &operations, &contributors);

    // Record contributors as a sibling extension for self-description.
    if record_contributors {
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

    Ok(merged)
}

/// Makes every `operationId` of a merged document unique.
///
/// `operations` is the merge's first-owner order (see [`merge_documents`]).
/// Walking it, the first operation to declare an id keeps it. A later
/// operation whose id is taken is renamed to its owning API's name in
/// PascalCase followed by the original id (`OllamaOpenAIListModels`); if that
/// name is any operation's original id or is already assigned, a numeric
/// suffix from 2 upward is appended. Every original id is reserved before the
/// walk, so a rename never takes the id a later operation already declares.
///
/// A link inside an operation was written by that operation's owner, so it is
/// mapped through the owner's original-to-final ids; the same raw id in
/// another API's link is left alone. Links in `components` come from the seed
/// API, whose ids are never renamed because it claims first.
fn assign_operation_ids(
    doc: &mut OpenAPI,
    operations: &IndexMap<(String, &'static str), Vec<usize>>,
    api_names: &[String],
) {
    let reserved: HashSet<String> = operations
        .keys()
        .filter_map(|(path, method)| operation_mut(doc, path, method)?.operation_id.clone())
        .collect();
    let mut claimed: HashSet<String> = HashSet::new();
    let mut final_ids: Vec<HashMap<String, String>> = vec![HashMap::new(); api_names.len()];

    for ((path, method), apis) in operations {
        let Some(op) = operation_mut(doc, path, method) else {
            continue;
        };
        let Some(original) = op.operation_id.clone() else {
            continue;
        };
        let assigned = if claimed.contains(&original) {
            let base = format!("{}{}", pascal_case(&api_names[apis[0]]), original);
            let mut candidate = base.clone();
            let mut suffix = 2;
            while reserved.contains(&candidate) || claimed.contains(&candidate) {
                candidate = format!("{base}{suffix}");
                suffix += 1;
            }
            candidate
        } else {
            original.clone()
        };
        claimed.insert(assigned.clone());
        op.operation_id = Some(assigned.clone());
        for &api in apis {
            final_ids[api].insert(original.clone(), assigned.clone());
        }
    }

    for ((path, method), apis) in operations {
        if let Some(op) = operation_mut(doc, path, method) {
            rewrite_response_links(&mut op.responses, &final_ids[apis[0]]);
        }
    }
}

/// Returns the operation at `path` + `method` of `doc`, if present.
fn operation_mut<'a>(
    doc: &'a mut OpenAPI,
    path: &str,
    method: &str,
) -> Option<&'a mut openapiv3::Operation> {
    let ReferenceOr::Item(item) = doc.paths.paths.get_mut(path)? else {
        return None;
    };
    match method {
        "GET" => item.get.as_mut(),
        "POST" => item.post.as_mut(),
        "PUT" => item.put.as_mut(),
        "PATCH" => item.patch.as_mut(),
        "DELETE" => item.delete.as_mut(),
        "HEAD" => item.head.as_mut(),
        "OPTIONS" => item.options.as_mut(),
        "TRACE" => item.trace.as_mut(),
        _ => None,
    }
}

/// Maps the `operationId` of every link in an operation's responses.
fn rewrite_response_links(responses: &mut openapiv3::Responses, ids: &HashMap<String, String>) {
    for response in responses
        .default
        .iter_mut()
        .chain(responses.responses.values_mut())
    {
        if let ReferenceOr::Item(response) = response {
            rewrite_links(&mut response.links, ids);
        }
    }
}

/// Maps the `operationId` of every inline link through `ids`.
fn rewrite_links(
    links: &mut IndexMap<String, ReferenceOr<openapiv3::Link>>,
    ids: &HashMap<String, String>,
) {
    for link in links.values_mut() {
        if let ReferenceOr::Item(link) = link
            && let LinkOperation::OperationId(target) = &mut link.operation
            && let Some(assigned) = ids.get(target.as_str())
        {
            *target = assigned.clone();
        }
    }
}

/// Lists `(link name, operationId)` for every inline link of an operation's
/// responses.
fn response_link_targets(responses: &openapiv3::Responses) -> Vec<(&str, &str)> {
    responses
        .default
        .iter()
        .chain(responses.responses.values())
        .filter_map(|response| match response {
            ReferenceOr::Item(response) => Some(link_targets(&response.links)),
            ReferenceOr::Reference { .. } => None,
        })
        .flatten()
        .collect()
}

/// Lists `(link name, operationId)` for every inline `operationId` link.
fn link_targets(links: &IndexMap<String, ReferenceOr<openapiv3::Link>>) -> Vec<(&str, &str)> {
    links
        .iter()
        .filter_map(|(name, link)| match link {
            ReferenceOr::Item(openapiv3::Link {
                operation: LinkOperation::OperationId(target),
                ..
            }) => Some((name.as_str(), target.as_str())),
            _ => None,
        })
        .collect()
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
    let ops: [&mut Option<openapiv3::Operation>; 8] = [
        &mut item.get,
        &mut item.post,
        &mut item.put,
        &mut item.patch,
        &mut item.delete,
        &mut item.head,
        &mut item.options,
        &mut item.trace,
    ];
    for op_slot in ops {
        if let Some(op) = op_slot.as_mut()
            && let Some(security) = op.security.as_mut()
        {
            rename_security_requirements(security, renames);
        }
    }
}

/// Renames the schemes referenced by security requirements.
fn rename_security_requirements(
    requirements: &mut [openapiv3::SecurityRequirement],
    renames: &IndexMap<String, String>,
) {
    for req in requirements.iter_mut() {
        let mut renamed_req = openapiv3::SecurityRequirement::new();
        for (name, scopes) in req.iter() {
            let new_name = renames.get(name).cloned().unwrap_or_else(|| name.clone());
            renamed_req.insert(new_name, scopes.clone());
        }
        *req = renamed_req;
    }
}

/// Converts an API name to the PascalCase prefix of a renamed `operationId`.
///
/// Names that are already PascalCase are returned unchanged
/// (`OllamaOpenAI`); `_`-separated words are capitalized and joined
/// (`my_api` → `MyApi`).
fn pascal_case(name: &str) -> String {
    name.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
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
    use schematic_define::CrossOrigin;
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
        let id = format!("{}Endpoint", name);
        api_with_endpoints(name, module, auth, &[(id.as_str(), method, path)])
    }

    /// Synthetic REST API with the given `(id, method, path)` endpoints.
    fn api_with_endpoints(
        name: &str,
        module: &str,
        auth: schematic_define::AuthStrategy,
        endpoints: &[(&str, schematic_define::RestMethod, &str)],
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
            endpoints: endpoints
                .iter()
                .map(|(id, method, path)| Endpoint {
                    id: id.to_string(),
                    method: *method,
                    path: path.to_string(),
                    description: format!("Endpoint {}", id),
                    request: None,
                    response: ApiResponse::Empty,
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                })
                .collect(),
            module_path: Some(module.to_string()),
            request_suffix: None,
            version: None,
            env_mapping: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
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

        let GeneratorError::Validation {
            output,
            diagnostics,
        } = err
        else {
            panic!("expected a validation error, got {err:?}");
        };
        assert_eq!(output, Output::OpenApi);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        let rendered = diagnostics[0].to_string();
        assert!(
            rendered.starts_with(
                "severity=error api=FooA,FooB endpoint=<none> \
                 feature=duplicate-method-path output=openapi: module 'foo': GET /dup"
            ),
            "{rendered}"
        );
        assert_eq!(
            fs::read_dir(temp.path()).unwrap().count(),
            0,
            "a rejected module writes no file"
        );
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
    // operationId allocation (R13, R14)
    // =============================================

    /// `(path, method) -> operationId` for every operation, in document order.
    fn operation_ids(doc: &OpenAPI) -> Vec<(String, String)> {
        let mut ids = Vec::new();
        for (path, item_ref) in &doc.paths.paths {
            if let ReferenceOr::Item(item) = item_ref {
                for (method, op) in path_methods_present(item) {
                    ids.push((
                        format!("{method} {path}"),
                        op.operation_id.clone().unwrap_or_default(),
                    ));
                }
            }
        }
        ids
    }

    fn build(apis: &[&RestApi]) -> Result<OpenAPI, GeneratorError> {
        build_openapi_grouped(apis, "foo", &EmptyRegistry, &ExportOptions::new())
    }

    #[test]
    fn compatible_operations_merge_security_without_renaming() {
        use schematic_define::{AuthStrategy, RestMethod};

        // The EMQX shape: one route, one id, two auth strategies.
        let basic = api_with_endpoints(
            "FooBasic",
            "foo",
            AuthStrategy::Basic,
            &[("ListThings", RestMethod::Get, "/things")],
        );
        let bearer = api_with_endpoints(
            "FooBearer",
            "foo",
            AuthStrategy::BearerToken { header: None },
            &[
                ("ListThings", RestMethod::Get, "/things"),
                ("GetThing", RestMethod::Get, "/things/{id}"),
            ],
        );

        let doc = build(&[&basic, &bearer]).unwrap();

        assert_eq!(
            operation_ids(&doc),
            [
                ("GET /things".to_string(), "ListThings".to_string()),
                ("GET /things/{id}".to_string(), "GetThing".to_string()),
            ]
        );
        // The exporter declares security at document level; both
        // alternatives survive the merge.
        let schemes: Vec<&String> = doc
            .security
            .as_ref()
            .expect("merged security")
            .iter()
            .flat_map(|req| req.keys())
            .collect();
        assert_eq!(schemes, ["basicAuth", "bearerAuth"]);
    }

    #[test]
    fn later_clashing_ids_take_the_owning_api_prefix() {
        use schematic_define::{AuthStrategy, RestMethod};

        let native = api_with_endpoints(
            "OllamaNative",
            "foo",
            AuthStrategy::None,
            &[
                ("ListModels", RestMethod::Get, "/api/tags"),
                ("Embeddings", RestMethod::Post, "/api/embeddings"),
            ],
        );
        let openai = api_with_endpoints(
            "OllamaOpenAI",
            "foo",
            AuthStrategy::None,
            &[
                ("Embeddings", RestMethod::Post, "/v1/embeddings"),
                ("ListModels", RestMethod::Get, "/v1/models"),
                ("Completions", RestMethod::Post, "/v1/completions"),
            ],
        );

        let doc = build(&[&native, &openai]).unwrap();

        assert_eq!(
            operation_ids(&doc),
            [
                ("GET /api/tags".to_string(), "ListModels".to_string()),
                ("POST /api/embeddings".to_string(), "Embeddings".to_string()),
                (
                    "POST /v1/embeddings".to_string(),
                    "OllamaOpenAIEmbeddings".to_string()
                ),
                (
                    "GET /v1/models".to_string(),
                    "OllamaOpenAIListModels".to_string()
                ),
                ("POST /v1/completions".to_string(), "Completions".to_string()),
            ]
        );
        assert!(operation_id_diagnostics(&doc, &["OllamaNative", "OllamaOpenAI"]).is_empty());
    }

    #[test]
    fn renames_never_take_an_original_id_declared_later() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = api_with_endpoints(
            "FooA",
            "foo",
            AuthStrategy::None,
            &[("List", RestMethod::Get, "/a")],
        );
        let b = api_with_endpoints(
            "FooB",
            "foo",
            AuthStrategy::None,
            &[("List", RestMethod::Get, "/b")],
        );
        // The adversary: C declares the name B's rename would propose, and C
        // is merged after B.
        let c = api_with_endpoints(
            "FooC",
            "foo",
            AuthStrategy::None,
            &[
                ("FooBList", RestMethod::Get, "/c"),
                ("List", RestMethod::Get, "/c/list"),
            ],
        );
        let d = api_with_endpoints(
            "FooB2",
            "foo",
            AuthStrategy::None,
            &[("List", RestMethod::Get, "/d")],
        );

        let doc = build(&[&a, &b, &c, &d]).unwrap();

        assert_eq!(
            operation_ids(&doc),
            [
                ("GET /a".to_string(), "List".to_string()),
                ("GET /b".to_string(), "FooBList2".to_string()),
                ("GET /c".to_string(), "FooBList".to_string()),
                ("GET /c/list".to_string(), "FooCList".to_string()),
                // `FooB2List` is free: suffixes are only appended on a clash.
                ("GET /d".to_string(), "FooB2List".to_string()),
            ]
        );
        assert!(operation_id_diagnostics(&doc, &["FooA"]).is_empty());
    }

    #[test]
    fn allocation_is_deterministic_and_follows_registry_order() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = api_with_endpoints(
            "FooA",
            "foo",
            AuthStrategy::None,
            &[("List", RestMethod::Get, "/a")],
        );
        let b = api_with_endpoints(
            "FooB",
            "foo",
            AuthStrategy::None,
            &[("List", RestMethod::Get, "/b")],
        );

        let first = serde_json::to_string(&build(&[&a, &b]).unwrap()).unwrap();
        let second = serde_json::to_string(&build(&[&a, &b]).unwrap()).unwrap();
        assert_eq!(first, second);

        // The first API in registry order keeps the id.
        let reversed = build(&[&b, &a]).unwrap();
        assert_eq!(
            operation_ids(&reversed),
            [
                ("GET /b".to_string(), "List".to_string()),
                ("GET /a".to_string(), "FooAList".to_string()),
            ]
        );
    }

    #[test]
    fn unique_ids_are_unchanged_for_every_live_module() {
        use schematic_definitions::apis_by_module;
        use schematic_definitions::registry::get_registries_for_module;

        for (module_name, apis) in apis_by_module() {
            let Some(registry) = get_registries_for_module(&module_name) else {
                continue;
            };
            let refs: Vec<&RestApi> = apis.iter().collect();
            let doc =
                build_openapi_grouped(&refs, &module_name, &registry, &ExportOptions::new())
                    .unwrap_or_else(|e| panic!("module {module_name}: {e}"));

            let declared: HashSet<&str> = apis
                .iter()
                .flat_map(|api| api.endpoints.iter().map(|e| e.id.as_str()))
                .collect();
            let renamed: Vec<String> = operation_ids(&doc)
                .into_iter()
                .filter(|(_, id)| !declared.contains(id.as_str()))
                .map(|(route, id)| format!("{route} {id}"))
                .collect();
            let expected: &[&str] = if module_name == "ollama" {
                &[
                    "POST /v1/embeddings OllamaOpenAIEmbeddings",
                    "GET /v1/models OllamaOpenAIListModels",
                ]
            } else {
                &[]
            };
            assert_eq!(renamed, expected, "module {module_name}");
        }
    }

    /// Adds a response link named `name` targeting `target` to the operation
    /// at `path` + `method` of `doc`.
    fn add_link(doc: &mut OpenAPI, path: &str, method: &str, name: &str, target: &str) {
        let op = operation_mut(doc, path, method).expect("operation");
        let mut response = openapiv3::Response {
            description: "ok".to_string(),
            ..Default::default()
        };
        response.links.insert(
            name.to_string(),
            ReferenceOr::Item(openapiv3::Link {
                description: None,
                operation: LinkOperation::OperationId(target.to_string()),
                request_body: None,
                parameters: IndexMap::new(),
                server: None,
                extensions: IndexMap::new(),
            }),
        );
        op.responses
            .responses
            .insert(openapiv3::StatusCode::Code(299), ReferenceOr::Item(response));
    }

    /// The `operationId` of the link `name` on the operation at `path` + `method`.
    fn link_target(doc: &mut OpenAPI, path: &str, method: &str, name: &str) -> String {
        let op = operation_mut(doc, path, method).expect("operation");
        response_link_targets(&op.responses)
            .into_iter()
            .find(|(link, _)| *link == name)
            .map(|(_, target)| target.to_string())
            .expect("link present")
    }

    #[test]
    fn links_follow_the_rename_of_their_own_document_only() {
        use schematic_define::{AuthStrategy, RestMethod};

        let a = api_with_endpoints(
            "FooA",
            "foo",
            AuthStrategy::None,
            &[
                ("List", RestMethod::Get, "/a"),
                ("Create", RestMethod::Post, "/a"),
            ],
        );
        let b = api_with_endpoints(
            "FooB",
            "foo",
            AuthStrategy::None,
            &[
                ("List", RestMethod::Get, "/b"),
                ("Delete", RestMethod::Delete, "/b"),
            ],
        );
        let options = ExportOptions::new();
        let mut doc_a = export(&a, &EmptyRegistry, &options).unwrap();
        let mut doc_b = export(&b, &EmptyRegistry, &options).unwrap();
        // Both documents link to their own `List`; the raw id is ambiguous
        // across them.
        add_link(&mut doc_a, "/a", "POST", "list", "List");
        add_link(&mut doc_b, "/b", "DELETE", "list", "List");
        add_link(&mut doc_b, "/b", "GET", "self", "List");

        let mut merged = merge_documents(
            "foo",
            vec![("FooA".to_string(), doc_a), ("FooB".to_string(), doc_b)],
            true,
        )
        .unwrap();

        assert_eq!(link_target(&mut merged, "/a", "POST", "list"), "List");
        assert_eq!(link_target(&mut merged, "/b", "DELETE", "list"), "FooBList");
        assert_eq!(link_target(&mut merged, "/b", "GET", "self"), "FooBList");
        assert!(operation_id_diagnostics(&merged, &["FooA", "FooB"]).is_empty());
    }

    #[test]
    fn scan_reports_duplicate_ids_and_unresolved_links() {
        use schematic_define::{AuthStrategy, RestMethod};

        let api = api_with_endpoints(
            "FooA",
            "foo",
            AuthStrategy::None,
            &[
                ("List", RestMethod::Get, "/a"),
                ("Other", RestMethod::Get, "/b"),
            ],
        );
        let mut doc = export(&api, &EmptyRegistry, &ExportOptions::new()).unwrap();
        assert!(operation_id_diagnostics(&doc, &["FooA"]).is_empty());

        operation_mut(&mut doc, "/b", "GET").unwrap().operation_id = Some("List".to_string());
        add_link(&mut doc, "/a", "GET", "gone", "Other");

        let rendered: Vec<String> = operation_id_diagnostics(&doc, &["FooA"])
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            rendered,
            [
                "severity=error api=FooA endpoint=<none> feature=duplicate-operation-id \
                 output=openapi: operationId 'List' is declared by GET /a and GET /b; every \
                 operationId in a document must be unique",
                "severity=error api=FooA endpoint=<none> feature=unresolved-link-operation \
                 output=openapi: link 'gone' of GET /a names operationId 'Other', which no \
                 operation in the document declares",
            ]
        );
    }

    #[test]
    fn scan_checks_component_links_too() {
        let mut doc = OpenAPI::default();
        doc.components
            .get_or_insert_with(Default::default)
            .links
            .insert(
                "orphan".to_string(),
                ReferenceOr::Item(openapiv3::Link {
                    description: None,
                    operation: LinkOperation::OperationId("Missing".to_string()),
                    request_body: None,
                    parameters: IndexMap::new(),
                    server: None,
                    extensions: IndexMap::new(),
                }),
            );

        let diagnostics = operation_id_diagnostics(&doc, &["FooA"]);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].feature, Feature::UnresolvedLinkOperation);
        assert!(
            diagnostics[0]
                .message
                .contains("'orphan' of components.links"),
            "{}",
            diagnostics[0].message
        );
    }

    #[test]
    fn pascal_case_keeps_pascal_names_and_joins_words() {
        assert_eq!(pascal_case("OllamaOpenAI"), "OllamaOpenAI");
        assert_eq!(pascal_case("my_api"), "MyApi");
        assert_eq!(pascal_case("emqx"), "Emqx");
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
