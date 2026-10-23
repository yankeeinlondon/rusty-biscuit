//! Multipart and URL-encoded request bodies through OpenAPI: export writes
//! `encoding` and array bounds, import reads them back into the same
//! `FormField`s, and what the model cannot carry is reported (R18).

#![cfg(feature = "openapi")]

use schematic_define::openapi::{
    DiagnosticSeverity, ExportFormat, ExportOptions, OpenApiDiagnostic, OpenApiImport, OpenApiImportResult, OpenApiSource,
    export, serialize,
};
use schematic_define::{
    ApiRequest, DefinitionContext, Endpoint, FormField, PrimitiveType, RestApi, RestMethod, TypeRef, definition_type,
};
use serde_json::{Value, json};

definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct AddSampleResponse { pub sample_id: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Metadata { pub title: String });

const SAMPLES: &str = "/v1/voices/{voice_id}/samples";
const SAMPLES_OP: &str = "#/paths/~1v1~1voices~1{voice_id}~1samples/post";
const MULTIPART: &str = "#/paths/~1v1~1voices~1{voice_id}~1samples/post/requestBody/content/multipart~1form-data";
const URL_ENCODED: &str =
    "#/paths/~1v1~1voices~1{voice_id}~1samples/post/requestBody/content/application~1x-www-form-urlencoded";

/// The ElevenLabs `AddVoiceSample` form, field for field.
fn add_voice_sample_fields() -> Vec<FormField> {
    vec![
        FormField::file_accept("audio", vec!["audio/*".into()]).with_description("Audio file (mp3, wav, ogg, m4a)"),
        FormField::text("name").optional().with_description("Name for the sample"),
    ]
}

/// One `AddVoiceSample`-shaped endpoint whose request is `request`.
fn api_with(request: ApiRequest) -> RestApi {
    let mut ctx = DefinitionContext::types_in("::models");
    ctx.register::<Metadata>().unwrap();
    let endpoint = Endpoint::new("AddVoiceSample", RestMethod::Post, SAMPLES)
        .describes("Upload audio sample for voice cloning")
        .request_form(add_voice_sample_fields())
        .returns_json::<AddSampleResponse>(&mut ctx)
        .build()
        .unwrap();
    let mut api = RestApi::builder(&ctx, "ElevenLabs", "https://api.elevenlabs.io")
        .describes("ElevenLabs")
        .no_auth()
        .endpoints(vec![endpoint])
        .build()
        .unwrap();
    api.endpoints[0].request = Some(request);
    api
}

fn export_text(api: &RestApi, format: ExportFormat) -> String {
    serialize(&export(api, &ExportOptions::new()).unwrap(), format).unwrap()
}

fn import(source: OpenApiSource) -> OpenApiImportResult {
    OpenApiImport::new(source).build().unwrap()
}

fn request_body(api: &RestApi) -> Value {
    let document = serde_json::to_value(export(api, &ExportOptions::new()).unwrap()).unwrap();
    document["paths"][SAMPLES]["post"]["requestBody"].clone()
}

/// Exports `request`, imports the JSON (and YAML when asked), and checks
/// both the request and its re-exported body survive unchanged.
fn assert_round_trip(request: ApiRequest, formats: &[ExportFormat]) {
    let api = api_with(request.clone());
    let exported_body = request_body(&api);
    for format in formats {
        let text = export_text(&api, *format);
        let source = match format {
            ExportFormat::Json => OpenApiSource::json(text),
            ExportFormat::Yaml => OpenApiSource::yaml(text),
        };
        let imported = import(source);
        let form_diagnostics: Vec<_> =
            imported.diagnostics.iter().filter(|diagnostic| diagnostic.location.starts_with(SAMPLES_OP)).collect();

        assert_eq!(imported.api.endpoints[0].request.as_ref(), Some(&request), "{format:?}");
        assert!(form_diagnostics.is_empty(), "{format:?}: {form_diagnostics:?}");
        assert_eq!(request_body(&imported.api), exported_body, "{format:?}");
    }
}

/// Imports one `AddVoiceSample` operation whose request body content is
/// `content`, with `schemas` as its components.
fn import_body(content: Value, schemas: Value) -> OpenApiImportResult {
    let document = json!({
        "openapi": "3.0.3",
        "info": {"title": "ElevenLabs", "version": "1"},
        "paths": {SAMPLES: {"post": {
            "operationId": "AddVoiceSample",
            "parameters": [{"name": "voice_id", "in": "path", "required": true, "schema": {"type": "string"}}],
            "requestBody": {"required": true, "content": content},
            "responses": {"204": {"description": "Added"}},
        }}},
        "components": {"schemas": schemas},
    });
    import(OpenApiSource::json(document.to_string()))
}

fn multipart(schema: Value, encoding: Value) -> Value {
    json!({"multipart/form-data": {"schema": schema, "encoding": encoding}})
}

fn form_fields(result: &OpenApiImportResult) -> &[FormField] {
    match &result.api.endpoints[0].request {
        Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) => fields,
        other => panic!("expected a form request, got {other:?}"),
    }
}

/// The diagnostics located at or under `location`, as (severity, location).
fn diagnostics_at(result: &OpenApiImportResult, prefix: &str) -> Vec<(DiagnosticSeverity, String)> {
    result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.location.starts_with(prefix))
        .map(|OpenApiDiagnostic { severity, location, .. }| (*severity, location.clone()))
        .collect()
}

#[test]
fn add_voice_sample_multipart_round_trips_through_json_and_yaml() {
    let request = ApiRequest::form_data(add_voice_sample_fields());

    let body = request_body(&api_with(request.clone()));

    assert_eq!(
        body,
        json!({
            "required": true,
            "content": {"multipart/form-data": {
                "schema": {
                    "type": "object",
                    "properties": {
                        "audio": {"description": "Audio file (mp3, wav, ogg, m4a)", "type": "string", "format": "binary"},
                        "name": {"description": "Name for the sample", "type": "string"},
                    },
                    "required": ["audio"],
                },
                "encoding": {"audio": {"contentType": "audio/*"}},
            }},
        })
    );
    assert_round_trip(request, &[ExportFormat::Json, ExportFormat::Yaml]);
}

#[test]
fn files_with_bounds_and_accept_list_round_trip() {
    let request = ApiRequest::form_data(vec![
        FormField::files_with_constraints("samples", vec!["audio/mpeg".into(), "audio/wav".into()], Some(1), Some(25))
            .with_description("Voice samples"),
        FormField::files("attachments").optional(),
        FormField::file("cover").optional(),
    ]);

    let body = request_body(&api_with(request.clone()));

    let media = &body["content"]["multipart/form-data"];
    assert_eq!(
        media["schema"]["properties"]["samples"],
        json!({"description": "Voice samples", "type": "array", "items": {"type": "string", "format": "binary"}, "minItems": 1, "maxItems": 25})
    );
    assert_eq!(media["encoding"], json!({"samples": {"contentType": "audio/mpeg, audio/wav"}}));
    assert_round_trip(request, &[ExportFormat::Json]);
}

#[test]
fn json_parts_round_trip_named_and_untyped() {
    let request = ApiRequest::form_data(vec![
        FormField::json("metadata", TypeRef::Named("Metadata".into())).with_description("Sample metadata"),
        FormField::json("extra", TypeRef::Primitive(PrimitiveType::Json)).optional(),
        FormField::text("name"),
    ]);

    let body = request_body(&api_with(request.clone()));

    let media = &body["content"]["multipart/form-data"];
    assert_eq!(
        media["schema"]["properties"]["metadata"],
        json!({"description": "Sample metadata", "allOf": [{"$ref": "#/components/schemas/Metadata"}]})
    );
    assert_eq!(
        media["encoding"],
        json!({"metadata": {"contentType": "application/json"}, "extra": {"contentType": "application/json"}})
    );
    assert_round_trip(request, &[ExportFormat::Json]);
}

#[test]
fn url_encoded_form_exports_no_encoding_and_round_trips() {
    let request = ApiRequest::url_encoded(vec![FormField::text("username"), FormField::text("password").optional()]);

    let body = request_body(&api_with(request.clone()));

    assert_eq!(body["content"]["application/x-www-form-urlencoded"].get("encoding"), None);
    assert_round_trip(request, &[ExportFormat::Json]);
}

#[test]
fn json_encoded_component_and_inline_parts_import_as_json() {
    let result = import_body(
        multipart(
            json!({"type": "object", "properties": {
                "metadata": {"$ref": "#/components/schemas/Metadata"},
                "options": {"type": "object", "properties": {"speed": {"type": "number"}}},
                "tags": {"type": "array", "items": {"type": "string"}},
            }}),
            json!({
                "metadata": {"contentType": "application/json"},
                "options": {"contentType": "application/json"},
                "tags": {"contentType": "application/vnd.tags+json"},
            }),
        ),
        json!({"Metadata": {"type": "object", "properties": {"title": {"type": "string"}}}}),
    );

    assert_eq!(
        form_fields(&result),
        [
            FormField::json("metadata", TypeRef::Named("Metadata".into())).optional(),
            FormField::json("options", TypeRef::Primitive(PrimitiveType::Json)).optional(),
            FormField::json("tags", TypeRef::Primitive(PrimitiveType::Json)).optional(),
        ]
    );
    assert_eq!(diagnostics_at(&result, SAMPLES_OP), []);
}

/// OpenAPI's default part encoding: a primitive, a component naming one, and
/// a composition of primitives are text parts (OpenAI's `response_format`
/// and `model` parts).
#[test]
fn primitive_parts_without_an_encoding_import_as_text_silently() {
    let result = import_body(
        multipart(
            json!({"type": "object", "properties": {
                "format": {"$ref": "#/components/schemas/Format"},
                "model": {"anyOf": [{"type": "string"}, {"type": "string", "enum": ["a", "b"]}]},
                "stream": {"type": "boolean"},
            }}),
            json!({}),
        ),
        json!({"Format": {"type": "string", "enum": ["json", "text"]}}),
    );

    assert_eq!(
        form_fields(&result),
        [FormField::text("format").optional(), FormField::text("model").optional(), FormField::text("stream").optional()]
    );
    assert_eq!(diagnostics_at(&result, SAMPLES_OP), []);
}

#[test]
fn object_parts_without_json_encoding_warn_and_stay_text() {
    let result = import_body(
        multipart(
            json!({"type": "object", "properties": {
                "options": {"type": "object", "properties": {"speed": {"type": "number"}}},
                "metadata": {"$ref": "#/components/schemas/Metadata"},
                "strategy": {"anyOf": [{"type": "string"}, {"$ref": "#/components/schemas/Metadata"}]},
            }}),
            json!({}),
        ),
        json!({"Metadata": {"type": "object", "properties": {"title": {"type": "string"}}}}),
    );

    assert_eq!(
        form_fields(&result),
        [FormField::text("metadata").optional(), FormField::text("options").optional(), FormField::text("strategy").optional()]
    );
    assert_eq!(diagnostics_at(&result, SAMPLES_OP), [
        (DiagnosticSeverity::Warn, format!("{MULTIPART}/schema/properties/metadata")),
        (DiagnosticSeverity::Warn, format!("{MULTIPART}/schema/properties/options")),
        (DiagnosticSeverity::Warn, format!("{MULTIPART}/schema/properties/strategy")),
    ]);
}

#[test]
fn array_of_non_binary_items_warns_and_imports_as_text() {
    let result = import_body(
        multipart(json!({"type": "object", "properties": {"labels": {"type": "array", "items": {"type": "string"}}}}), json!({})),
        json!({}),
    );

    assert_eq!(form_fields(&result), [FormField::text("labels").optional()]);
    assert_eq!(diagnostics_at(&result, SAMPLES_OP), [(DiagnosticSeverity::Warn, format!("{MULTIPART}/schema/properties/labels"))]);
}

#[test]
fn unresolved_part_ref_is_an_error_and_the_part_is_skipped() {
    let result = import_body(
        multipart(
            json!({"type": "object", "required": ["audio"], "properties": {
                "audio": {"type": "string", "format": "binary"},
                "metadata": {"$ref": "#/components/schemas/Missing"},
            }}),
            json!({}),
        ),
        json!({}),
    );

    assert_eq!(form_fields(&result), [FormField::file("audio")]);
    assert_eq!(diagnostics_at(&result, SAMPLES_OP), [(DiagnosticSeverity::Error, format!("{MULTIPART}/schema/properties/metadata"))]);
}

#[test]
fn unresolved_items_ref_is_an_error_and_the_part_is_skipped() {
    let result = import_body(
        multipart(
            json!({"type": "object", "properties": {
                "name": {"type": "string"},
                "samples": {"type": "array", "items": {"$ref": "#/components/schemas/Missing"}},
            }}),
            json!({}),
        ),
        json!({}),
    );

    assert_eq!(form_fields(&result), [FormField::text("name").optional()]);
    assert_eq!(
        diagnostics_at(&result, SAMPLES_OP),
        [(DiagnosticSeverity::Error, format!("{MULTIPART}/schema/properties/samples/items"))]
    );
}

#[test]
fn encoding_for_an_unknown_part_warns() {
    let result = import_body(
        multipart(json!({"type": "object", "properties": {"name": {"type": "string"}}}), json!({"ghost": {"contentType": "text/plain"}})),
        json!({}),
    );

    assert_eq!(form_fields(&result), [FormField::text("name").optional()]);
    assert_eq!(diagnostics_at(&result, SAMPLES_OP), [(DiagnosticSeverity::Warn, format!("{MULTIPART}/encoding/ghost"))]);
}

#[test]
fn encoding_headers_style_explode_and_allow_reserved_each_warn() {
    let result = import_body(
        multipart(
            json!({"type": "object", "properties": {"audio": {"type": "string", "format": "binary"}}}),
            json!({"audio": {
                "contentType": "audio/*",
                "headers": {"X-Rate": {"schema": {"type": "integer"}}},
                "style": "form",
                "explode": true,
                "allowReserved": true,
            }}),
        ),
        json!({}),
    );

    assert_eq!(form_fields(&result), [FormField::file_accept("audio", vec!["audio/*".into()]).optional()]);
    let encoding = format!("{MULTIPART}/encoding/audio");
    assert_eq!(
        diagnostics_at(&result, SAMPLES_OP),
        ["headers", "style", "explode", "allowReserved"].map(|key| (DiagnosticSeverity::Warn, format!("{encoding}/{key}")))
    );
}

#[test]
fn empty_content_type_is_an_error_and_accepts_any_file() {
    let result = import_body(
        multipart(json!({"type": "object", "properties": {"audio": {"type": "string", "format": "binary"}}}), json!({"audio": {"contentType": ""}})),
        json!({}),
    );

    assert_eq!(form_fields(&result), [FormField::file("audio").optional()]);
    assert_eq!(
        diagnostics_at(&result, SAMPLES_OP),
        [(DiagnosticSeverity::Error, format!("{MULTIPART}/encoding/audio/contentType"))]
    );
}

#[test]
fn binary_property_on_a_url_encoded_form_warns_and_imports_as_text() {
    let result = import_body(
        json!({"application/x-www-form-urlencoded": {"schema": {"type": "object", "properties": {
            "audio": {"type": "string", "format": "binary"},
            "name": {"type": "string"},
        }}}}),
        json!({}),
    );

    assert_eq!(form_fields(&result), [FormField::text("audio").optional(), FormField::text("name").optional()]);
    assert_eq!(diagnostics_at(&result, SAMPLES_OP), [(DiagnosticSeverity::Warn, format!("{URL_ENCODED}/schema/properties/audio"))]);
}
