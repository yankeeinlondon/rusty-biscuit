//! Response model v2 in the generator's exporters: the module-grouped
//! OpenAPI document for several success shapes and declared errors (R27),
//! Postman's first-shape content type (R29), and the GraphQL skip warning of
//! both exporters (R42, D-29).
//!
//! Several shapes and streams reach the exporters through the hidden
//! `Endpoint::shapes_preview` seam until `Endpoint::response` stores them.

use openapiv3::OpenAPI;
use schematic_define::openapi::ExportOptions;
use schematic_define::{
    AddressRule, DefinitionContext, Endpoint, ErrorSet, ErrorShape, GraphQlOperation, JsonTag, ResponseShape, RestApi, StatusMatch,
    StreamEnd, StreamFraming, SuccessShape, TypeRef,
};
use schematic_gen::diagnostics::{Feature, Output, Severity};
use schematic_gen::export::openapi::build_openapi_grouped;
use schematic_gen::export::postman::build_postman_collection;
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

/// An API whose endpoint `Get{i}` at `/r{i}` carries `previews[i]` (none
/// keeps its built lone empty shape), with `Problem` for 4XX by default.
fn api(previews: Vec<Vec<SuccessShape>>, ctx: &mut DefinitionContext, graphql: bool) -> RestApi {
    let endpoints: Vec<Endpoint> = (0..previews.len())
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
    for (endpoint, preview) in api.endpoints.iter_mut().zip(previews) {
        endpoint.shapes_preview = preview;
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
