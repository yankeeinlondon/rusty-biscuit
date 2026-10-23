//! Ledger row 14 (spec `2026-10-18-schematic-fidelity-polish`, R14): an
//! OpenAPI export whose optional collections are absent or explicitly empty
//! re-imports through the public pipeline with its meaning intact.
//!
//! The importer does not read `x-schematic` yet, so each test also decodes the
//! exported extension values with the public extension types: that is the
//! read a `skip_serializing_if` field without `serde(default)` would break.

use schematic_define::openapi::{
    ExportOptions, OpenApiImport, OpenApiSource, SchematicDocExtension, SchematicOpExtension,
};
use schematic_define::{
    ApiRequest, DefinitionContext, Endpoint, EnvMapping, ErrorSet, ResponseShape, RestApi,
    SuccessShape,
};
use schematic_gen::export::openapi::write_openapi;
use schematic_gen::import_pipeline::{ImportOptions, run_import};
use serde_json::Value;

/// An API with no fixed headers, no default errors, no rate limit, and the
/// given environment mapping, whose endpoints have no headers and (except
/// `CreateItem`) no request body.
fn sparse_api(env_mapping: Option<EnvMapping>) -> RestApi {
    let mut list = Endpoint::get("ListItems", "/items").describes("List items").returns_empty().build().unwrap();
    list.response = vec![SuccessShape::json_type("Item")];
    let mut create = Endpoint::post("CreateItem", "/items").describes("Create an item").returns_empty().build().unwrap();
    create.request = Some(ApiRequest::json_type("NewItem"));
    create.response = vec![SuccessShape::json_type("Item")];
    let delete = Endpoint::delete("DeleteItem", "/items/{id}")
        .describes("Delete an item")
        .returns_empty()
        .errors(ErrorSet::none())
        .build()
        .unwrap();
    let upload = Endpoint::post("UploadNothing", "/uploads").describes("Upload with no fields").request_form(vec![]).returns_empty().build().unwrap();
    let context = DefinitionContext::types_in("::fixture");
    let mut builder =
        RestApi::builder(&context, "Sparse", "https://sparse.test").describes("Sparse fixture API").no_auth().module_path("sparse");
    if let Some(mapping) = env_mapping {
        builder = builder.env_mapping(mapping);
    }
    // Endpoints join after build: the fixture's components are bare names
    // that `with_referenced_components` supplies, not registered Rust types.
    let mut api = builder.build().expect("valid sparse API");
    api.endpoints = vec![list, create, delete, upload];
    crate::common::components::with_referenced_components(api)
}

/// Writes `api`'s OpenAPI document into a fresh directory and returns the
/// directory guard, the file path, and the parsed document.
fn export(api: &RestApi) -> (tempfile::TempDir, std::path::PathBuf, Value) {
    let dir = tempfile::TempDir::new().unwrap();
    let path = write_openapi(api, &ExportOptions::new(), dir.path()).expect("export succeeds");
    let document = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    (dir, path, document)
}

/// Every operation object of `document`, keyed by `operationId`.
fn operations(document: &Value) -> Vec<(String, &Value)> {
    let mut found = Vec::new();
    for item in document["paths"].as_object().expect("paths").values() {
        for operation in item.as_object().unwrap().values().filter(|value| value.get("operationId").is_some()) {
            found.push((operation["operationId"].as_str().unwrap().to_string(), operation));
        }
    }
    found
}

/// The document extension `api` exports, as the exporter builds it.
fn expected_doc_extension(api: &RestApi) -> SchematicDocExtension {
    SchematicDocExtension {
        module_path: api.module_path.clone(),
        request_suffix: api.request_suffix.clone(),
        env_mapping: api.env_mapping.clone(),
        headers: api.headers.clone(),
        rate_limit: api.rate_limit.clone(),
        retry_guidance: api.retry_guidance.clone(),
        errors: api.errors.clone(),
        cross_origin: api.cross_origin.clone(),
        request_id_header: api.request_id_header.clone(),
    }
}

/// Decodes every `x-schematic` value of `document` with the public types and
/// asserts each equals what `api` declared.
fn assert_extensions_decode_to(api: &RestApi, document: &Value) {
    let doc_extension: SchematicDocExtension =
        serde_json::from_value(document["x-schematic"].clone()).expect("document x-schematic decodes");
    assert_eq!(doc_extension, expected_doc_extension(api));

    let operations = operations(document);
    assert_eq!(operations.len(), api.endpoints.len());
    for endpoint in &api.endpoints {
        let (_, operation) = operations
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(&endpoint.id) || id.replace('_', "").eq_ignore_ascii_case(&endpoint.id))
            .unwrap_or_else(|| panic!("no operation for {}", endpoint.id));
        let extension: SchematicOpExtension =
            serde_json::from_value(operation["x-schematic"].clone()).expect("operation x-schematic decodes");
        assert_eq!(extension.request, endpoint.request, "{}", endpoint.id);
        assert_eq!(extension.headers, endpoint.headers, "{}", endpoint.id);
        assert_eq!(extension.errors, endpoint.errors, "{}", endpoint.id);
        assert_eq!(extension.no_declared_errors, endpoint.no_declared_errors, "{}", endpoint.id);
        assert_eq!(extension.rate_limit, endpoint.rate_limit, "{}", endpoint.id);
        assert_eq!(extension.cross_origin, endpoint.cross_origin, "{}", endpoint.id);
        assert_eq!(extension.response, endpoint.response[0].shape, "{}", endpoint.id);
    }
}

/// Imports `path` through the public importer and the generator pipeline and
/// asserts the endpoints keep their method, path, body presence, and response
/// kind, and that no endpoint or API gained headers.
fn assert_reimport_preserves(api: &RestApi, path: &std::path::Path) {
    let imported = OpenApiImport::new(OpenApiSource::path(path)).build().expect("re-import succeeds");
    assert_eq!(imported.api.endpoints.len(), api.endpoints.len());
    assert!(imported.api.headers.is_empty());
    for endpoint in &api.endpoints {
        let twin = imported
            .api
            .endpoints
            .iter()
            .find(|candidate| candidate.method == endpoint.method && candidate.path == endpoint.path)
            .unwrap_or_else(|| panic!("{} {:?} {} lost on re-import", endpoint.id, endpoint.method, endpoint.path));
        assert!(twin.headers.is_empty(), "{}", endpoint.id);
        let kind = |shape: &ResponseShape| std::mem::discriminant(shape);
        assert_eq!(kind(&twin.response[0].shape), kind(&endpoint.response[0].shape), "{}", endpoint.id);
        // A multipart body imports as `None` today, a separate import gap, so
        // body presence is compared for JSON bodies and bodiless endpoints.
        match &endpoint.request {
            Some(ApiRequest::Json(_)) => assert!(matches!(twin.request, Some(ApiRequest::Json(_))), "{}", endpoint.id),
            None => assert!(twin.request.is_none(), "{}", endpoint.id),
            Some(_) => {}
        }
    }

    let output = tempfile::TempDir::new().unwrap();
    let result = run_import(&ImportOptions {
        input: path.display().to_string(),
        api_name: None,
        module_path: None,
        output: output.path().display().to_string(),
        definitions_out: None,
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: true,
        strict: false,
        verbose: false,
    })
    .expect("pipeline re-import succeeds");
    assert_eq!(result.endpoint_count, api.endpoints.len());
}

#[test]
fn absent_optional_collections_reimport() {
    let api = sparse_api(None);
    let (_dir, path, document) = export(&api);

    let doc_extension = document["x-schematic"].as_object().unwrap();
    for absent in ["env_mapping", "headers", "errors", "rate_limit", "retry_guidance", "request_id_header", "cross_origin"] {
        assert!(!doc_extension.contains_key(absent), "{absent} should be skipped, got {doc_extension:?}");
    }
    assert_extensions_decode_to(&api, &document);
    assert_reimport_preserves(&api, &path);
}

#[test]
fn explicitly_empty_env_mapping_reimports() {
    let api = sparse_api(Some(EnvMapping::default()));
    let (_dir, path, document) = export(&api);

    assert!(document["x-schematic"]["env_mapping"].is_object(), "an empty mapping is still written");
    assert_extensions_decode_to(&api, &document);
    assert_reimport_preserves(&api, &path);
}

/// The writer omits empty `Vec`s, so a document spelling them `[]` is
/// hand-written; it must decode to the same meaning as the omitted form.
#[test]
fn explicitly_empty_extension_arrays_decode_like_absent_ones() {
    let api = sparse_api(None);
    let (dir, _, mut document) = export(&api);
    let doc_extension = document["x-schematic"].as_object_mut().unwrap();
    doc_extension.insert("headers".into(), Value::Array(vec![]));
    doc_extension.insert("errors".into(), Value::Array(vec![]));
    for item in document["paths"].as_object_mut().unwrap().values_mut() {
        for operation in item.as_object_mut().unwrap().values_mut() {
            if let Some(extension) = operation.get_mut("x-schematic").and_then(Value::as_object_mut) {
                for field in ["headers", "success", "errors"] {
                    extension.entry(field).or_insert_with(|| Value::Array(vec![]));
                }
            }
        }
    }
    let path = dir.path().join("explicit_empty.json");
    std::fs::write(&path, serde_json::to_string_pretty(&document).unwrap()).unwrap();

    assert_extensions_decode_to(&api, &document);
    assert_reimport_preserves(&api, &path);
}
