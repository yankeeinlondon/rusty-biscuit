//! Ledger row 14 (spec `2026-10-18-schematic-fidelity-polish`, R14): an
//! OpenAPI export whose optional collections are absent or explicitly empty
//! re-imports through the public pipeline with its meaning intact.
//!
//! The importer reads `x-schematic` back, so each test asserts the imported
//! definition keeps every member the export omitted or wrote empty: the read
//! a `skip_serializing_if` field without `serde(default)` would break.

use schematic_define::openapi::{DiagnosticSeverity, ExportOptions, OpenApiImport, OpenApiSource};
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

/// Imports `path` through the public importer and the generator pipeline and
/// asserts the endpoints keep their method, path, request, response kind,
/// errors, and inherited metadata, and that no endpoint or API gained
/// headers. Returns the import diagnostics.
fn assert_reimport_preserves(api: &RestApi, path: &std::path::Path) -> Vec<schematic_define::openapi::OpenApiDiagnostic> {
    let imported = OpenApiImport::new(OpenApiSource::path(path)).build().expect("re-import succeeds");
    assert_eq!(imported.api.endpoints.len(), api.endpoints.len());
    assert!(imported.api.headers.is_empty());
    assert_eq!(imported.api.env_mapping, api.env_mapping);
    assert_eq!(imported.api.errors, api.errors);
    assert_eq!(imported.api.rate_limit, api.rate_limit);
    assert_eq!(imported.api.cross_origin, api.cross_origin);
    assert_eq!(imported.api.request_id_header, api.request_id_header);
    assert_eq!(imported.api.module_path, api.module_path);
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
        assert_eq!(twin.errors, endpoint.errors, "{}", endpoint.id);
        assert_eq!(twin.no_declared_errors, endpoint.no_declared_errors, "{}", endpoint.id);
        assert_eq!(twin.rate_limit, endpoint.rate_limit, "{}", endpoint.id);
        assert_eq!(twin.cross_origin, endpoint.cross_origin, "{}", endpoint.id);
        // A form with no fields has no standard schema to import from.
        match &endpoint.request {
            Some(ApiRequest::FormData { fields }) if fields.is_empty() => assert!(twin.request.is_none(), "{}", endpoint.id),
            Some(ApiRequest::Json(_)) => assert!(matches!(twin.request, Some(ApiRequest::Json(_))), "{}", endpoint.id),
            request => assert_eq!(&twin.request, request, "{}", endpoint.id),
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
    imported.diagnostics
}

/// The only WARN a sparse export re-imports with: its form has no fields.
fn assert_only_the_empty_form_warns(diagnostics: &[schematic_define::openapi::OpenApiDiagnostic]) {
    let warnings: Vec<_> = diagnostics.iter().filter(|diagnostic| diagnostic.severity >= DiagnosticSeverity::Warn).collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].message.contains("no properties"), "{warnings:?}");
}

#[test]
fn absent_optional_collections_reimport() {
    let api = sparse_api(None);
    let (_dir, path, document) = export(&api);

    let doc_extension = document["x-schematic"].as_object().unwrap();
    for absent in ["env_mapping", "headers", "errors", "rate_limit", "retry_guidance", "request_id_header", "cross_origin"] {
        assert!(!doc_extension.contains_key(absent), "{absent} should be skipped, got {doc_extension:?}");
    }
    assert_only_the_empty_form_warns(&assert_reimport_preserves(&api, &path));
}

#[test]
fn explicitly_empty_env_mapping_reimports() {
    let api = sparse_api(Some(EnvMapping::default()));
    let (_dir, path, document) = export(&api);

    assert!(document["x-schematic"]["env_mapping"].is_object(), "an empty mapping is still written");
    assert_only_the_empty_form_warns(&assert_reimport_preserves(&api, &path));
}

/// The writer omits empty `Vec`s, so a document spelling them `[]` is
/// hand-written. `headers` and `errors` mean the same as the omitted form;
/// `success: []` is never valid (an endpoint has at least one success shape),
/// so it is a WARN at its location and the standard responses stand.
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

    let diagnostics = assert_reimport_preserves(&api, &path);
    let mut rejected: Vec<&str> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warn && diagnostic.location.ends_with("/x-schematic/success"))
        .map(|diagnostic| diagnostic.location.as_str())
        .collect();
    rejected.sort_unstable();
    assert_eq!(rejected, [
        "#/paths/~1items/get/x-schematic/success",
        "#/paths/~1items/post/x-schematic/success",
        "#/paths/~1items~1{id}/delete/x-schematic/success",
        "#/paths/~1uploads/post/x-schematic/success",
    ]);
}
