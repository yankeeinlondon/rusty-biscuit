//! Response model v2 in the generator's exporters: the module-grouped
//! OpenAPI document for several success shapes and declared errors (R27),
//! Postman's first-shape content type (R29), and the GraphQL skip warning of
//! both exporters (R42, D-29), first over a synthetic API and then over the
//! shipped definitions migrated to it (R5, R33).

use openapiv3::OpenAPI;
use schematic_define::openapi::ExportOptions;
use schematic_define::{
    AddressRule, DefinitionContext, Endpoint, ErrorSet, ErrorShape, GraphQlOperation, JsonTag, ResponseShape, RestApi, StatusMatch,
    StreamEnd, StreamFraming, SuccessShape, TypeRef,
};
use schematic_gen::diagnostics::{Feature, Output, Severity};
use schematic_gen::export::openapi::build_openapi_grouped;
use schematic_gen::export::postman::{build_postman_collection, build_postman_collection_grouped};
use schematic_gen::validation::{validate_openapi, validate_postman};
use serde_json::{Value, json};

use crate::common::components::with_referenced_components;

fn named(name: &str) -> TypeRef { TypeRef::Named(name.into()) }

fn shape(shape: ResponseShape, edit: impl FnOnce(&mut SuccessShape)) -> SuccessShape {
    let mut shape = SuccessShape::new(shape);
    edit(&mut shape);
    shape
}

fn sse(item: &str) -> ResponseShape {
    ResponseShape::Stream { framing: StreamFraming::Sse(named(item)), end: StreamEnd::EventNamed("message_stop".into()), keep: false }
}

/// An API whose endpoint `Get{i}` at `/r{i}` carries `shapes[i]` (an empty
/// entry keeps its built lone empty shape), with text for 4XX by default.
fn api(shapes: Vec<Vec<SuccessShape>>, ctx: &mut DefinitionContext, graphql: bool) -> RestApi {
    let endpoints: Vec<Endpoint> = (0..shapes.len())
        .map(|index| Endpoint::get(format!("Get{index}"), format!("/r{index}")).describes("Get").returns_empty().build().unwrap())
        .collect();
    let viewer = graphql.then(|| GraphQlOperation::query("Viewer", "query Viewer { viewer { login } }").describes("Viewer")
        .no_variables().data::<serde_json::Value>(ctx).build().unwrap());
    let mut builder = RestApi::builder(ctx, "Fixture", "https://fixture.example/api").describes("Fixture").no_auth()
        .default_errors(ErrorSet::new().on(StatusMatch::class(4), ErrorShape::text()))
        .endpoints(endpoints);
    if let Some(viewer) = viewer {
        builder = builder.graphql([AddressRule::append("/graphql")]).operations([viewer]);
    }
    let mut api = builder.build().unwrap();
    for (endpoint, shapes) in api.endpoints.iter_mut().zip(shapes) {
        if !shapes.is_empty() { endpoint.response = shapes; }
    }
    with_referenced_components(api)
}

fn request<'a>(collection: &'a Value, name: &str) -> &'a Value {
    fn find<'a>(items: &'a Value, name: &str) -> Option<&'a Value> {
        items.as_array()?.iter().find_map(|item| {
            if item["name"] == name && item.get("request").is_some() { Some(&item["request"]) } else { find(&item["item"], name) }
        })
    }
    find(&collection["item"], name).unwrap_or_else(|| panic!("no request {name}"))
}

#[test]
fn the_grouped_openapi_document_merges_shapes_and_parses_back() {
    let mut ctx = DefinitionContext::types_in("::fixture");
    let tagged = |name: &str, value: &str| shape(ResponseShape::Json(named(name)), |s| {
        s.json_tag = Some(JsonTag::Field { name: "type".into(), value: json!(value) });
    });
    let api = api(vec![vec![
        shape(ResponseShape::Json(named("Message")), |s| s.note = Some("returned when `stream` is false".into())),
        shape(sse("Event"), |s| s.note = Some("returned when `stream` is true".into())),
        shape(ResponseShape::Binary, |s| s.fallback = true),
    ], vec![tagged("Directory", "dir"), tagged("File", "file")]], &mut ctx, false);

    let document = build_openapi_grouped(&[&api], "fixture", &ExportOptions::new()).unwrap();
    let text = serde_json::to_string(&document).unwrap();
    assert_eq!(serde_json::from_str::<OpenAPI>(&text).unwrap().paths, document.paths);
    let document = serde_json::to_value(&document).unwrap();

    let ok = &document["paths"]["/r0"]["get"]["responses"]["200"];
    let media: Vec<&str> = ok["content"].as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(media, ["*/*", "application/json", "text/event-stream"]);
    assert!(ok["description"].as_str().unwrap().contains("text/event-stream: returned when `stream` is true"));
    assert_eq!(document["paths"]["/r0"]["get"]["responses"]["4XX"]["content"]["text/plain"]["schema"], json!({"type": "string"}));
    assert_eq!(document["paths"]["/r0"]["get"]["x-schematic"]["success"][1]["shape"]["Stream"]["end"], json!({"EventNamed": "message_stop"}));
    assert_eq!(document["paths"]["/r1"]["get"]["responses"]["200"]["content"]["application/json"]["schema"]["discriminator"]["propertyName"], "type");
}

#[test]
fn postman_records_the_first_success_shapes_telling_content_type() {
    let mut ctx = DefinitionContext::types_in("::fixture");
    let api = api(vec![
        vec![shape(sse("Event"), |_| {}), shape(ResponseShape::Json(named("Message")), |_| {})],
        vec![shape(ResponseShape::Binary, |s| s.content_type = Some("Audio/MPEG".into()))],
        vec![],
    ], &mut ctx, false);

    let collection = serde_json::to_value(build_postman_collection(&api)).unwrap();

    assert_eq!(request(&collection, "Get0")["description"], "Get\n\nResponse content type: `text/event-stream`.");
    assert_eq!(request(&collection, "Get1")["description"], "Get\n\nResponse content type: `audio/mpeg`.");
    // A plain shape says nothing beyond its kind, so shipped requests are unchanged.
    assert_eq!(request(&collection, "Get2")["description"], "Get");
}

#[test]
fn both_exporters_skip_graphql_operations_with_a_warning() {
    let mut ctx = DefinitionContext::types_in("::fixture");
    let api = api(vec![vec![]], &mut ctx, true);

    for (diagnostics, output, target) in [
        (validate_openapi(&[&api]), Output::OpenApi, "OpenAPI"),
        (validate_postman(&[&api]), Output::Postman, "Postman"),
    ] {
        let skips: Vec<_> = diagnostics.iter().filter(|d| d.feature == Feature::GraphQlOperation).collect();
        let [skip] = skips.as_slice() else { panic!("{diagnostics:?}") };
        assert_eq!(skip.severity, Severity::Warning);
        assert_eq!((skip.output, skip.endpoint.as_deref(), skip.api.as_slice()), (output, Some("Viewer"), &["Fixture".to_string()][..]));
        assert_eq!(skip.message, format!("Fixture operation Viewer: GraphQL operation skipped in {target} output"));
        assert!(!diagnostics.iter().any(|d| d.is_error()), "{diagnostics:?}");
    }
    // The skip is nonfatal: both outputs build, without the operation.
    let document = serde_json::to_string(&build_openapi_grouped(&[&api], "fixture", &ExportOptions::new()).unwrap()).unwrap();
    assert!(!document.contains("Viewer"));
    assert!(!serde_json::to_string(&build_postman_collection(&api)).unwrap().contains("Viewer"));
}

// Shipped definitions: one module group per test.

/// The grouped document for `apis`, after asserting it parses back through
/// `openapiv3` from both JSON and YAML.
fn shipped_document(apis: &[RestApi], module: &str) -> Value {
    let refs: Vec<&RestApi> = apis.iter().collect();
    let document = build_openapi_grouped(&refs, module, &ExportOptions::new()).unwrap();
    let value = serde_json::to_value(&document).unwrap();
    let json_text = serde_json::to_string(&document).unwrap();
    let json_back = serde_json::to_value(serde_json::from_str::<OpenAPI>(&json_text).unwrap()).unwrap();
    assert!(json_back == value, "{module}: JSON round trip differs");
    let yaml_text = biscuit_file::serde_yaml_ng::to_string(&document).unwrap();
    let yaml_back = serde_json::to_value(biscuit_file::serde_yaml_ng::from_str::<OpenAPI>(&yaml_text).unwrap()).unwrap();
    assert!(yaml_back == value, "{module}: YAML round trip differs");
    value
}

/// The path item and operation whose `operationId` is `id`.
fn operation<'a>(document: &'a Value, id: &str) -> (&'a Value, &'a Value) {
    document["paths"].as_object().unwrap().values()
        .find_map(|item| item.as_object()?.values().find(|op| op["operationId"] == id).map(|op| (item, op)))
        .unwrap_or_else(|| panic!("no operation {id}"))
}

fn ok_media(op: &Value) -> Vec<&str> {
    op["responses"]["200"]["content"].as_object().unwrap().keys().map(String::as_str).collect()
}

fn endpoint<'a>(apis: &'a [RestApi], id: &str) -> &'a Endpoint {
    apis.iter().flat_map(|api| &api.endpoints).find(|e| e.id == id).unwrap_or_else(|| panic!("no endpoint {id}"))
}

#[test]
fn shipped_elevenlabs_audio_exports_octet_stream_and_keeps_query_parameters() {
    let apis = schematic_definitions::elevenlabs::module_group().unwrap();
    let document = shipped_document(&apis, "elevenlabs");
    let binary = json!({"application/octet-stream": {"schema": {"type": "string", "format": "binary"}}});

    // ElevenLabs documents no audio media type, so none is declared (R5).
    for id in ["CreateSpeech", "CreateSoundEffect", "GetHistoryItemAudio", "DownloadHistoryItems", "GetVoiceSampleAudio", "StreamSpeech"] {
        let (_, op) = operation(&document, id);
        assert_eq!(op["responses"].as_object().unwrap().keys().filter(|s| s.starts_with('2')).count(), 1, "{id}");
        assert_eq!(op["responses"]["200"]["content"], binary, "{id}");
    }
    let (_, op) = operation(&document, "StreamSpeech");
    assert_eq!(op["x-schematic"]["success"], json!([{"shape": {"Stream": {"end": "ConnectionClose", "framing": "Bytes"}}}]));

    let (_, op) = operation(&document, "StreamSpeechWithTimestamps");
    assert_eq!(op["responses"]["200"]["content"], json!({"application/x-ndjson": {"schema": {"$ref": "#/components/schemas/SpeechWithTimestampsResponse"}}}));
    assert_eq!(op["x-schematic"]["success"][0]["shape"]["Stream"],
        json!({"end": "ConnectionClose", "framing": {"Ndjson": {"Named": "SpeechWithTimestampsResponse"}}}));

    // Every declared query parameter survives export (R5: format-changing ones included).
    for id in ["CreateSpeech", "StreamSpeech", "StreamSpeechWithTimestamps", "CreateSoundEffect", "GetHistoryItemAudio", "DownloadHistoryItems", "GetVoiceSampleAudio"] {
        let declared: Vec<&str> = endpoint(&apis, id).params.iter().flat_map(|p| &p.query).map(|p| p.name.as_str()).collect();
        let (item, op) = operation(&document, id);
        let exported: Vec<&str> = [&item["parameters"], &op["parameters"]].into_iter()
            .filter_map(Value::as_array).flatten()
            .filter(|p| p["in"] == "query").map(|p| p["name"].as_str().unwrap()).collect();
        assert_eq!(exported, declared, "{id}");
    }
}

#[test]
fn shipped_anthropic_create_message_exports_json_and_sse_with_both_notes() {
    let document = shipped_document(&schematic_definitions::anthropic::module_group().unwrap(), "anthropic");
    let (_, op) = operation(&document, "CreateMessage");
    assert_eq!(ok_media(op), ["application/json", "text/event-stream"]);
    let description = op["responses"]["200"]["description"].as_str().unwrap();
    assert!(description.contains("application/json: returned when `stream` is false or omitted"), "{description}");
    assert!(description.contains("text/event-stream: returned when `stream` is true"), "{description}");
    assert_eq!(op["x-schematic"]["success"][1]["shape"]["Stream"]["end"], json!({"EventNamed": "message_stop"}));
}

#[test]
fn shipped_bitbucket_get_source_exports_json_and_a_fallback() {
    let document = shipped_document(&schematic_definitions::bitbucket::module_group().unwrap(), "bitbucket");
    let (_, op) = operation(&document, "GetSource");
    assert_eq!(ok_media(op), ["*/*", "application/json"]);
    assert_eq!(op["responses"]["200"]["content"]["*/*"]["schema"], json!({"type": "string", "format": "binary"}));
}

#[test]
fn shipped_ollama_streams_export_beside_their_json_shapes() {
    let document = shipped_document(&schematic_definitions::ollama::module_group().unwrap(), "ollama");
    for id in ["Generate", "Chat"] {
        assert_eq!(ok_media(operation(&document, id).1), ["application/json", "application/x-ndjson"], "{id}");
    }
    assert_eq!(ok_media(operation(&document, "ChatCompletions").1), ["application/json", "text/event-stream"]);
}

#[test]
fn shipped_openai_get_response_exports_json_and_sse() {
    // Assembling OpenAI's 258 endpoints takes most of a second; export one.
    let mut apis = schematic_definitions::openai::module_group().unwrap();
    apis[0].endpoints.retain(|e| e.id == "GetResponse");
    let document = shipped_document(&apis, "openai");
    assert_eq!(ok_media(operation(&document, "GetResponse").1), ["application/json", "text/event-stream"]);
}

#[test]
fn shipped_postman_collections_record_the_first_success_content_type() {
    let ollama = schematic_definitions::ollama::module_group().unwrap();
    let refs: Vec<&RestApi> = ollama.iter().collect();
    let collection = serde_json::to_value(build_postman_collection_grouped(&refs, "ollama")).unwrap();
    // NDJSON comes first, so the stream's media type is recorded, not JSON's.
    assert!(request(&collection, "Generate")["description"].as_str().unwrap().ends_with("\n\nResponse content type: `application/x-ndjson`."));
    // A plain JSON first shape records nothing.
    assert!(!request(&collection, "ChatCompletions")["description"].as_str().unwrap().contains("Response content type"));

    let anthropic = schematic_definitions::anthropic::module_group().unwrap();
    let collection = serde_json::to_value(build_postman_collection(&anthropic[0])).unwrap();
    assert!(!request(&collection, "CreateMessage")["description"].as_str().unwrap().contains("Response content type"));
}
