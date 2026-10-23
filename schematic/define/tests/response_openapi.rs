//! Response model v2 through OpenAPI: export of success shapes and the
//! resolved error table (R21, R27, D-25, D-31), the `x-schematic` writes
//! (R26, D-24), and import classification (R28, D-20).

#![cfg(feature = "openapi")]

use biscuit_file::serde_yaml_ng;
use openapiv3::{OpenAPI, ReferenceOr};
use schematic_define::openapi::import::RefResolver;
use schematic_define::openapi::{
    DiagnosticSeverity, ExportFormat, ExportOptions, OpenApiImport, OpenApiImportResult, OpenApiSource, classify_responses, export, serialize,
};
use schematic_define::{
    AuthMethod, AuthPolicy, Backoff, CrossOrigin, RateLimit, ResetUnit, RetryGuidance, StatusSelector,
    DefinitionContext, Endpoint, ErrorEntry, ErrorSet, ErrorShape, JsonTag, ResponseShape, RestApi, StatusMatch, StreamEnd,
    StreamFraming, SuccessShape, TypeRef, definition_type,
};
use serde_json::{Value, json};

definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Message { pub text: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Event { pub kind: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Problem { pub message: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Directory { pub entries: Vec<String> });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct File { pub size: u64 });

fn named(name: &str) -> TypeRef { TypeRef::Named(name.into()) }
fn shape(shape: ResponseShape, edit: impl FnOnce(&mut SuccessShape)) -> SuccessShape {
    let mut shape = SuccessShape::new(shape);
    edit(&mut shape);
    shape
}
fn json_shape(name: &str) -> SuccessShape { SuccessShape::new(ResponseShape::Json(named(name))) }
fn sse(end: StreamEnd) -> SuccessShape {
    SuccessShape::new(ResponseShape::Stream { framing: StreamFraming::Sse(named("Event")), end, keep: false })
}
fn fallback_binary() -> SuccessShape { shape(ResponseShape::Binary, |s| s.fallback = true) }

/// An API with `Problem` for 4XX and text for 5XX by default, and one
/// endpoint per entry of `previews`, each `Get{i}` at `/r{i}` carrying the
/// preview shapes (none keeps the builder's lone JSON `Message`).
/// One endpoint per entry of `shapes`; an empty entry keeps the built
/// `Message` JSON shape.
fn fixture(shapes: Vec<Vec<SuccessShape>>, endpoint_errors: impl Fn(usize) -> Option<ErrorSet>) -> RestApi {
    let mut ctx = DefinitionContext::types_in("::models");
    let problem = ctx.register::<Problem>().unwrap();
    for register in [DefinitionContext::register::<Event>, DefinitionContext::register::<Directory>, DefinitionContext::register::<File>] {
        register(&mut ctx).unwrap();
    }
    let endpoints: Vec<Endpoint> = (0..shapes.len()).map(|index| {
        let builder = Endpoint::get(format!("Get{index}"), format!("/r{index}")).describes("Get").returns_json::<Message>(&mut ctx);
        match endpoint_errors(index) { Some(errors) => builder.errors(errors), None => builder }.build().unwrap()
    }).collect();
    let mut api = RestApi::builder(&ctx, "Fixture", "https://api.example.com").describes("Fixture").no_auth()
        .default_errors(ErrorSet::new().on(StatusMatch::class(4), ErrorShape::json(problem)).on(StatusMatch::class(5), ErrorShape::text()))
        .endpoints(endpoints).build().unwrap();
    for (endpoint, shapes) in api.endpoints.iter_mut().zip(shapes) {
        if !shapes.is_empty() { endpoint.response = shapes; }
    }
    api
}

fn exported(api: &RestApi) -> Value {
    serde_json::to_value(export(api, &ExportOptions::new()).unwrap()).unwrap()
}

fn operation(document: &Value, index: usize) -> &Value { &document["paths"][format!("/r{index}")]["get"] }

#[test]
fn every_shape_of_a_status_merges_into_one_response_object_with_its_notes() {
    let shapes = vec![
        shape(ResponseShape::Json(named("Message")), |s| s.note = Some("returned when `stream` is false".into())),
        shape(sse(StreamEnd::EventNamed("message_stop".into())).shape, |s| s.note = Some("returned when `stream` is true".into())),
        fallback_binary(),
    ];
    let document = exported(&fixture(vec![shapes], |_| None));
    let ok = &operation(&document, 0)["responses"]["200"];

    // `serde_json::Value` sorts the keys.
    let media: Vec<&str> = ok["content"].as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(media, ["*/*", "application/json", "text/event-stream"]);
    assert_eq!(ok["content"]["application/json"]["schema"], json!({"$ref": "#/components/schemas/Message"}));
    // An SSE stream exports its item schema; framing and end ride `x-schematic`.
    assert_eq!(ok["content"]["text/event-stream"]["schema"], json!({"$ref": "#/components/schemas/Event"}));
    assert_eq!(ok["content"]["*/*"]["schema"], json!({"type": "string", "format": "binary"}));
    assert_eq!(ok["description"], "Successful response\n\napplication/json: returned when `stream` is false\n\ntext/event-stream: returned when `stream` is true");
    let success = &operation(&document, 0)["x-schematic"]["success"];
    assert_eq!(success[1]["shape"], json!({"Stream": {"framing": {"Sse": {"Named": "Event"}}, "end": {"EventNamed": "message_stop"}}}));
    assert_eq!(success[2], json!({"shape": "Binary", "fallback": true}));
    assert_eq!(success[0]["note"], "returned when `stream` is false");
}

#[test]
fn json_shapes_sharing_status_and_media_become_a_one_of_naming_the_discriminator() {
    let tagged = |name: &str, value: &str| shape(ResponseShape::Json(named(name)), |s| {
        s.json_tag = Some(JsonTag::Field { name: "type".into(), value: json!(value) });
        s.variant = Some(name.into());
    });
    let document = exported(&fixture(vec![vec![tagged("Directory", "dir"), tagged("File", "file")]], |_| None));
    let schema = &operation(&document, 0)["responses"]["200"]["content"]["application/json"]["schema"];

    assert_eq!(schema, &json!({
        "oneOf": [{"$ref": "#/components/schemas/Directory"}, {"$ref": "#/components/schemas/File"}],
        "discriminator": {"propertyName": "type", "mapping": {"dir": "#/components/schemas/Directory", "file": "#/components/schemas/File"}},
    }));
    let success = &operation(&document, 0)["x-schematic"]["success"];
    assert_eq!(success[0]["json_tag"], json!({"Field": {"name": "type", "value": "dir"}}));
    assert_eq!(success[1]["variant"], "File");
}

#[test]
fn array_and_object_json_shapes_become_a_one_of_without_a_discriminator() {
    let listing = shape(ResponseShape::Json(TypeRef::Array(Box::new(named("File")))), |s| s.json_tag = Some(JsonTag::Array));
    let file = shape(ResponseShape::Json(named("File")), |s| s.json_tag = Some(JsonTag::Object));
    let document = exported(&fixture(vec![vec![listing, file]], |_| None));
    let schema = &operation(&document, 0)["responses"]["200"]["content"]["application/json"]["schema"];

    assert_eq!(schema, &json!({"oneOf": [
        {"type": "array", "items": {"$ref": "#/components/schemas/File"}},
        {"$ref": "#/components/schemas/File"},
    ]}));
}

#[test]
fn byte_streams_export_as_binary_strings_under_each_declared_status() {
    let audio = shape(ResponseShape::Stream { framing: StreamFraming::Bytes, end: StreamEnd::ConnectionClose, keep: false }, |s| {
        s.content_type = Some("Audio/MPEG; q=1".into());
        s.statuses = vec![200, 206];
    });
    let accepted = shape(ResponseShape::Empty, |s| s.statuses = vec![202]);
    let document = exported(&fixture(vec![vec![audio, accepted]], |_| None));
    let responses = &operation(&document, 0)["responses"];

    for status in ["200", "206"] {
        assert_eq!(responses[status]["content"], json!({"audio/mpeg": {"schema": {"type": "string", "format": "binary"}}}), "{status}");
        assert_eq!(responses[status]["description"], "Successful byte stream");
    }
    assert_eq!(responses["202"], json!({"description": "No content"}));
}

#[test]
fn a_class_range_stays_4xx_unless_an_exact_entry_supersedes_part_of_it() {
    let api = fixture(vec![vec![], vec![], vec![]], |index| match index {
        1 => Some(ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::empty())),
        2 => Some(ErrorSet::none()),
        _ => None,
    });
    let document = exported(&api);
    let keys = |index: usize| -> Vec<String> { operation(&document, index)["responses"].as_object().unwrap().keys().cloned().collect() };

    assert_eq!(keys(0), ["200", "4XX", "5XX"]);
    assert_eq!(operation(&document, 0)["responses"]["4XX"]["content"]["application/json"]["schema"], json!({"$ref": "#/components/schemas/Problem"}));
    assert_eq!(operation(&document, 0)["responses"]["5XX"]["content"]["text/plain"]["schema"], json!({"type": "string"}));

    let expanded = keys(1);
    assert!(!expanded.contains(&"4XX".to_string()));
    assert_eq!(expanded.len(), 1 + 100 + 1, "200, 400-499 exact, 5XX");
    assert_eq!(expanded.last().map(String::as_str), Some("5XX"));
    let responses = &operation(&document, 1)["responses"];
    assert_eq!(responses["404"], json!({"description": "Declared error 404"}));
    assert_eq!(responses["418"]["content"]["application/json"]["schema"], json!({"$ref": "#/components/schemas/Problem"}));
    assert_eq!(responses["418"]["description"], "Declared error 400-499");

    assert_eq!(keys(2), ["200"]);
    for index in 0..3 {
        let (success, error): (Vec<String>, Vec<String>) = keys(index).into_iter().partition(|key| key.starts_with('2'));
        assert!(success.iter().all(|key| !error.contains(key)), "R21: success and error keys are disjoint");
    }
}

#[test]
fn x_schematic_carries_declared_error_levels_and_no_success_for_a_plain_shape() {
    let api = fixture(vec![vec![], vec![]], |index| match index {
        0 => Some(ErrorSet::new().on(StatusMatch::exact(409), ErrorShape::text().content_type("text/csv"))),
        _ => Some(ErrorSet::none()),
    });
    let document = exported(&api);

    assert_eq!(document["x-schematic"]["errors"], json!([
        {"status": {"Range": {"lo": 400, "hi": 499}}, "shape": {"Json": {"Named": "Problem"}}},
        {"status": {"Range": {"lo": 500, "hi": 599}}, "shape": "Text"},
    ]));
    let own = &operation(&document, 0)["x-schematic"];
    assert_eq!(own["errors"], json!([{"status": {"Exact": 409}, "shape": "Text", "content_type": "text/csv"}]));
    assert!(own.get("success").is_none() && own.get("no_declared_errors").is_none());
    assert_eq!(operation(&document, 0)["responses"]["409"]["content"], json!({"text/csv": {"schema": {"type": "string"}}}));
    assert_eq!(operation(&document, 1)["x-schematic"]["no_declared_errors"], true);
}

#[test]
fn the_exported_document_parses_back_through_openapiv3_as_json_and_yaml() {
    let shapes = vec![
        json_shape("Message"),
        sse(StreamEnd::DataLineEquals("[DONE]".into())),
        shape(ResponseShape::Json(named("Directory")), |s| s.json_tag = Some(JsonTag::Field { name: "type".into(), value: json!(true) })),
        fallback_binary(),
    ];
    let mut shapes_with_tag = shapes.clone();
    shapes_with_tag[0].json_tag = Some(JsonTag::Field { name: "type".into(), value: json!(false) });
    let api = fixture(vec![shapes_with_tag], |_| Some(ErrorSet::new().on(StatusMatch::range(429, 431), ErrorShape::empty())));
    let document = export(&api, &ExportOptions::new()).unwrap();

    let json_text = serialize(&document, ExportFormat::Json).unwrap();
    let yaml_text = serialize(&document, ExportFormat::Yaml).unwrap();
    let responses = |document: &OpenAPI| match &document.paths.paths["/r0"] {
        ReferenceOr::Item(item) => item.get.as_ref().unwrap().responses.clone(),
        _ => unreachable!(),
    };
    for reparsed in [serde_json::from_str::<OpenAPI>(&json_text).unwrap(), serde_yaml_ng::from_str::<OpenAPI>(&yaml_text).unwrap()] {
        assert_eq!(responses(&reparsed), responses(&document));
        assert_eq!(serde_json::to_value(&reparsed).unwrap(), serde_json::to_value(&document).unwrap());
    }
    // A non-string tag value names the field but maps no member.
    let schema = &serde_json::to_value(&document).unwrap()["paths"]["/r0"]["get"]["responses"]["200"]["content"]["application/json"]["schema"];
    assert_eq!(schema["discriminator"], json!({"propertyName": "type"}));
}

// ---------------------------------------------------------------------------
// Import classification (R28)
// ---------------------------------------------------------------------------

fn document(responses: Value) -> OpenAPI {
    serde_json::from_value(json!({
        "openapi": "3.0.3",
        "info": {"title": "Fixture", "version": "1"},
        "paths": {"/r": {"get": {"operationId": "Get", "responses": responses}}},
        "components": {
            "schemas": {
                "Event": {"type": "object"}, "Message": {"type": "object"}, "Problem": {"type": "object"},
                "Directory": {"type": "object"}, "File": {"type": "object"}, "Files": {"type": "array", "items": {"$ref": "#/components/schemas/File"}},
            },
            "responses": {"Missing": {"description": "Not found", "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Problem"}}}}},
        },
    })).unwrap()
}

fn classify(responses: Value) -> Result<schematic_define::openapi::ResponseClassification, String> {
    let document = document(responses);
    let ReferenceOr::Item(item) = &document.paths.paths["/r"] else { unreachable!() };
    let operation = item.get.as_ref().unwrap();
    classify_responses("Fixture", "Get", &operation.responses, &RefResolver::new(&document)).map_err(|error| error.to_string())
}

fn body(media: &str, schema: Value) -> Value { json!({"description": "x", "content": {media: {"schema": schema}}}) }
fn reference(name: &str) -> Value { json!({"$ref": format!("#/components/schemas/{name}")}) }
fn content_typed(shape: ResponseShape, content_type: Option<&str>) -> SuccessShape {
    let mut success = SuccessShape::new(shape);
    success.content_type = content_type.map(str::to_owned);
    success
}

#[test]
fn import_classifies_each_normalized_media_family() {
    let media = json!({
        "Text/Event-Stream; charset=utf-8": {"schema": reference("Event")},
        "application/x-ndjson": {"schema": reference("Event")},
        "application/jsonl": {"schema": reference("Message")},
        "application/json": {"schema": reference("Message")},
        "application/problem+json": {"schema": reference("Problem")},
        "text/plain": {"schema": {"type": "string"}},
        "text/csv": {"schema": {"type": "string"}},
        "audio/mpeg": {"schema": {"type": "string", "format": "binary"}},
        "application/octet-stream": {},
    });
    let classified = classify(json!({"200": {"description": "x", "content": media}})).unwrap();
    let stream = |framing| ResponseShape::Stream { framing, end: StreamEnd::ConnectionClose, keep: false };

    // `json!` sorts the media keys, and uppercase sorts first.
    assert_eq!(classified.successes, vec![
        content_typed(stream(StreamFraming::Sse(named("Event"))), None),
        content_typed(ResponseShape::Json(named("Message")), None),
        content_typed(stream(StreamFraming::Ndjson(named("Message"))), Some("application/jsonl")),
        content_typed(ResponseShape::Binary, None),
        content_typed(ResponseShape::Json(named("Problem")), Some("application/problem+json")),
        content_typed(stream(StreamFraming::Ndjson(named("Event"))), None),
        content_typed(ResponseShape::Binary, Some("audio/mpeg")),
        content_typed(ResponseShape::Text, Some("text/csv")),
        content_typed(ResponseShape::Text, None),
    ]);
    assert!(classified.diagnostics.is_empty(), "{:?}", classified.diagnostics);
}

#[test]
fn a_stream_without_a_named_item_schema_decodes_values_until_close_with_a_diagnostic() {
    let classified = classify(json!({"200": {"description": "x", "content": {"text/event-stream": {}}}})).unwrap();

    assert_eq!(classified.successes, vec![SuccessShape::new(ResponseShape::Stream {
        framing: StreamFraming::Sse(TypeRef::Primitive(schematic_define::PrimitiveType::Json)), end: StreamEnd::ConnectionClose, keep: false,
    })]);
    let [diagnostic] = classified.diagnostics.as_slice() else { panic!("{:?}", classified.diagnostics) };
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Warn);
    assert!(diagnostic.message.starts_with("API Fixture endpoint Get: the stream item has no named schema"), "{}", diagnostic.message);
    assert_eq!(diagnostic.location, "responses/200/content/text/event-stream");
}

#[test]
fn a_one_of_separates_by_discriminator_or_array_versus_object_and_otherwise_imports_as_one_schema() {
    let by_field = classify(json!({"200": body("application/json", json!({
        "oneOf": [reference("Directory"), reference("File")],
        "discriminator": {"propertyName": "type", "mapping": {"dir": "#/components/schemas/Directory"}},
    }))})).unwrap();
    let field = |value: &str| Some(JsonTag::Field { name: "type".into(), value: json!(value) });
    assert_eq!(by_field.successes.iter().map(|s| (s.shape.clone(), s.json_tag.clone())).collect::<Vec<_>>(), vec![
        (ResponseShape::Json(named("Directory")), field("dir")),
        // Unmapped: the component name is the value.
        (ResponseShape::Json(named("File")), field("File")),
    ]);

    let by_kind = classify(json!({"200": body("application/json", json!({"oneOf": [reference("Files"), reference("File")]}))})).unwrap();
    assert_eq!(by_kind.successes.iter().map(|s| (s.shape.clone(), s.json_tag.clone())).collect::<Vec<_>>(), vec![
        (ResponseShape::Json(named("Files")), Some(JsonTag::Array)),
        (ResponseShape::Json(named("File")), Some(JsonTag::Object)),
    ]);
    assert!(by_kind.diagnostics.is_empty());

    let inseparable = classify(json!({"200": body("application/json", json!({"oneOf": [reference("Directory"), reference("File")]}))})).unwrap();
    assert_eq!(inseparable.successes, vec![SuccessShape::new(ResponseShape::Json(TypeRef::Primitive(schematic_define::PrimitiveType::Json)))]);
    assert!(inseparable.diagnostics[0].message.contains("separable neither by a `discriminator` nor as one array and one object"));
}

#[test]
fn errors_stay_endpoint_level_and_unrepresentable_responses_are_reported() {
    let classified = classify(json!({
        "204": {"description": "done"},
        "4XX": {"$ref": "#/components/responses/Missing"},
        "404": {"description": "gone"},
        "5XX": {"description": "x", "content": {"text/plain": {"schema": {"type": "string"}}, "application/json": {"schema": reference("Problem")}}},
        "default": body("application/json", reference("Problem")),
        "101": {"description": "switching"},
        "302": {"description": "moved"},
        "200": {"description": "x", "content": {"*/*": {"schema": {"type": "string"}}, "image/*": {"schema": {"type": "string"}}, "application/json": {"schema": reference("Message")}}},
    })).unwrap();

    let mut fallback = SuccessShape::new(ResponseShape::Text);
    fallback.fallback = true;
    // `json!` sorts the statuses and media keys.
    assert_eq!(classified.successes, vec![fallback, SuccessShape::new(ResponseShape::Json(named("Message"))), SuccessShape::new(ResponseShape::Empty)]);
    assert_eq!(classified.errors, vec![
        ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Empty),
        ErrorEntry::new(StatusMatch::class(4), ResponseShape::Json(named("Problem"))),
        // The first media type is kept; the text alternative is reported.
        ErrorEntry::new(StatusMatch::class(5), ResponseShape::Json(named("Problem"))),
    ]);
    let messages: Vec<&str> = classified.diagnostics.iter().map(|d| d.message.as_str()).collect();
    for needle in [
        "API Fixture endpoint Get: the OpenAPI `default` response is neither a success shape nor an API default error",
        "API Fixture endpoint Get: error 500-599 declares media types needing different shapes",
        "API Fixture endpoint Get: status 101 is neither a 2xx success nor a 400-599 error",
        "API Fixture endpoint Get: status 302 is neither",
        "API Fixture endpoint Get: media range `image/*` cannot select a shape",
    ] {
        assert!(messages.iter().any(|message| message.starts_with(needle) || message.contains(needle)), "missing {needle:?} in {messages:#?}");
    }
}

#[test]
fn success_statuses_are_declared_only_when_statuses_carry_different_shapes() {
    let same = classify(json!({"200": body("application/json", reference("Message")), "201": body("application/json", reference("Message"))})).unwrap();
    assert_eq!(same.successes, vec![json_shape("Message")]);

    let different = classify(json!({
        "200": body("application/json", reference("Message")),
        "202": body("application/json", reference("Event")),
        "204": {"description": "none"},
    })).unwrap();
    let with_statuses = |name: &str, statuses: Vec<u16>| { let mut s = json_shape(name); s.statuses = statuses; s };
    assert_eq!(different.successes, vec![with_statuses("Message", vec![200]), with_statuses("Event", vec![202]), SuccessShape::new(ResponseShape::Empty)]);
}

#[test]
fn an_operation_without_a_representable_success_is_an_import_error() {
    let error = classify(json!({"404": {"description": "gone"}, "default": {"description": "x"}})).unwrap_err();
    assert!(error.contains("API Fixture endpoint Get: no 2xx response yields a representable success shape"), "{error}");
    let error = classify(json!({"200": {"description": "x", "content": {"image/*": {}}}})).unwrap_err();
    assert!(error.contains("no 2xx response yields a representable success shape"), "{error}");
    let error = classify(json!({"200": {"$ref": "#/components/responses/Absent"}})).unwrap_err();
    assert!(error.contains("API Fixture endpoint Get: responses/200 does not resolve"), "{error}");
}

#[test]
fn export_then_import_of_json_sse_fallback_and_errors_yields_the_same_shapes_and_entries() {
    let shapes = vec![json_shape("Message"), sse(StreamEnd::ConnectionClose), fallback_binary()];
    let api = fixture(vec![shapes.clone()], |_| None);
    let document = export(&api, &ExportOptions::new()).unwrap();
    let ReferenceOr::Item(item) = &document.paths.paths["/r0"] else { unreachable!() };

    let classified = classify_responses("Fixture", "Get0", &item.get.as_ref().unwrap().responses, &RefResolver::new(&document)).unwrap();

    assert_eq!(classified.successes, shapes);
    // The API defaults come back as the endpoint's own entries (D-20).
    assert_eq!(classified.errors, api.effective_errors(&api.endpoints[0]).entries());
    assert!(classified.diagnostics.is_empty(), "{:?}", classified.diagnostics);
}

// ---------------------------------------------------------------------------
// Every shape kind, the x-schematic members, and the public import round trip
// ---------------------------------------------------------------------------

fn stream(framing: StreamFraming, end: StreamEnd) -> ResponseShape { ResponseShape::Stream { framing, end, keep: false } }

#[test]
fn every_success_and_error_kind_exports_its_media_key_and_schema() {
    let typed = |shape: ResponseShape, content_type: &str| self::shape(shape, |s| s.content_type = Some(content_type.into()));
    let shapes = vec![
        vec![json_shape("Message")],
        vec![SuccessShape::new(ResponseShape::Text)],
        vec![typed(ResponseShape::Text, "text/markdown")],
        vec![SuccessShape::new(ResponseShape::Binary)],
        vec![typed(ResponseShape::Binary, "audio/mpeg")],
        vec![SuccessShape::new(ResponseShape::Empty)],
        vec![SuccessShape::new(stream(StreamFraming::Sse(named("Event")), StreamEnd::DataLineEquals("[DONE]".into())))],
        vec![SuccessShape::new(stream(StreamFraming::Ndjson(named("Event")), StreamEnd::FieldEquals { field: "done".into(), value: json!(true) }))],
        vec![SuccessShape::new(stream(StreamFraming::Bytes, StreamEnd::ConnectionClose))],
    ];
    let mut api = fixture(shapes, |index| (index == 0).then(|| ErrorSet::new()
        .on(StatusMatch::exact(404), ErrorShape::empty())
        .on(StatusMatch::exact(409), ErrorShape::binary().content_type("application/zip"))
        .on(StatusMatch::exact(410), ErrorShape::text())));
    api.endpoints[0].errors.push(ErrorEntry::new(StatusMatch::exact(422), ResponseShape::Json(named("Directory"))));
    let document = export(&api, &ExportOptions::new()).unwrap();
    let value = serde_json::to_value(&document).unwrap();
    let ok = |index: usize, status: &str| -> Value { operation(&value, index)["responses"][status].clone() };
    let string = json!({"type": "string"});
    let binary = json!({"type": "string", "format": "binary"});

    for (index, media, schema) in [
        (0, "application/json", reference("Message")),
        (1, "text/plain", string.clone()),
        (2, "text/markdown", string.clone()),
        (3, "application/octet-stream", binary.clone()),
        (4, "audio/mpeg", binary.clone()),
        (6, "text/event-stream", reference("Event")),
        (7, "application/x-ndjson", reference("Event")),
        (8, "application/octet-stream", binary.clone()),
    ] {
        assert_eq!(ok(index, "200")["content"], json!({media: {"schema": schema}}), "endpoint {index}");
    }
    assert_eq!(ok(5, "204"), json!({"description": "No content"}));
    assert!(ok(5, "200").is_null());
    assert_eq!(operation(&value, 6)["x-schematic"]["success"], json!([{"shape": {"Stream": {"framing": {"Sse": {"Named": "Event"}}, "end": {"DataLineEquals": "[DONE]"}}}}]));
    assert_eq!(operation(&value, 7)["x-schematic"]["success"][0]["shape"]["Stream"]["end"], json!({"FieldEquals": {"field": "done", "value": true}}));
    assert_eq!(operation(&value, 8)["x-schematic"]["success"][0]["shape"], json!({"Stream": {"framing": "Bytes", "end": "ConnectionClose"}}));

    // Exact errors expand 4XX; 5XX keeps its class key.
    assert_eq!(ok(0, "404"), json!({"description": "Declared error 404"}));
    assert_eq!(ok(0, "409")["content"], json!({"application/zip": {"schema": binary}}));
    assert_eq!(ok(0, "410")["content"], json!({"text/plain": {"schema": string}}));
    assert_eq!(ok(0, "422")["content"], json!({"application/json": {"schema": reference("Directory")}}));
    assert_eq!(ok(0, "400")["content"], json!({"application/json": {"schema": reference("Problem")}}));
    assert_eq!(ok(0, "5XX")["content"], json!({"text/plain": {"schema": string}}));
    assert!(ok(0, "4XX").is_null());
    for index in 1..9 {
        assert!(ok(index, "4XX")["content"]["application/json"].is_object() && ok(index, "5XX").is_object(), "endpoint {index}");
    }

    for index in 0..9 {
        let keys: Vec<&String> = operation(&value, index)["responses"].as_object().unwrap().keys().collect();
        let (success, error): (Vec<&String>, Vec<&String>) = keys.into_iter().partition(|key| key.starts_with('2'));
        assert!(!success.is_empty() && !error.is_empty(), "endpoint {index}");
        assert!(error.iter().all(|key| key.starts_with('4') || key.starts_with('5')), "R21: endpoint {index} {error:?}");
        assert!(success.iter().all(|key| !error.contains(key)), "R21: endpoint {index}");
    }

    let json_text = serialize(&document, ExportFormat::Json).unwrap();
    let yaml_text = serialize(&document, ExportFormat::Yaml).unwrap();
    for reparsed in [serde_json::from_str::<OpenAPI>(&json_text).unwrap(), serde_yaml_ng::from_str::<OpenAPI>(&yaml_text).unwrap()] {
        assert_eq!(serde_json::to_value(&reparsed).unwrap(), value);
    }
}

#[test]
fn x_schematic_carries_cross_origin_request_id_and_guidance_while_deprecation_and_auth_stay_standard() {
    let rate_limit = RateLimit::new("x-limit", "x-remaining", "x-reset", ResetUnit::EpochSeconds).unwrap();
    let retry = RetryGuidance::new([StatusSelector::Code(429)], true, 3, Backoff::None).unwrap();
    let get = |id: &str| Endpoint::get(id, format!("/{}", id.to_ascii_lowercase())).describes(id).returns_empty();
    let ctx = DefinitionContext::types_in("::models");
    let api = RestApi::builder(&ctx, "Policy", "https://api.example.com").describes("Policy").bearer_env(["POLICY_TOKEN"])
        .request_id_header("X-Request-Id")
        .cross_origin(CrossOrigin::allow_origins(["https://app.example.com"]))
        .rate_limit(rate_limit.clone())
        .retry_guidance(retry.clone())
        .endpoints([
            get("Old").deprecated().cross_origin(CrossOrigin::deny())
                .auth(AuthPolicy { explicit: vec![AuthMethod::Basic], env_fallback: None }).build().unwrap(),
            get("Open").public().rate_limit(rate_limit).build().unwrap(),
            get("Plain").build().unwrap(),
        ])
        .build().unwrap();
    let document = exported(&api);
    let op = |path: &str| &document["paths"][path]["get"];

    let doc_ext = &document["x-schematic"];
    assert_eq!(doc_ext["cross_origin"], json!({"AllowOrigins": ["https://app.example.com"]}));
    assert_eq!(doc_ext["request_id_header"], "x-request-id");
    assert_eq!(doc_ext["rate_limit"]["limit_header"], "x-limit");
    assert_eq!(doc_ext["retry_guidance"]["max_attempts"], 3);
    assert_eq!(op("/old")["x-schematic"]["cross_origin"], "Deny");
    assert_eq!(op("/open")["x-schematic"]["rate_limit"]["remaining_header"], "x-remaining");
    for path in ["/open", "/plain"] {
        assert!(op(path)["x-schematic"].get("cross_origin").is_none(), "{path} inherits");
        assert!(op(path).get("deprecated").is_none(), "{path}");
    }
    assert!(op("/plain")["x-schematic"].get("rate_limit").is_none());

    // R26 refresh: deprecation and the auth override are standard members only.
    assert_eq!(op("/old")["deprecated"], true);
    assert_eq!(document["security"], json!([{"bearerAuth": []}]));
    assert_eq!(op("/old")["security"], json!([{"basicAuth": []}]));
    assert_eq!(op("/open")["security"], json!([]));
    assert!(op("/plain").get("security").is_none());
    let schemes: Vec<&String> = document["components"]["securitySchemes"].as_object().unwrap().keys().collect();
    assert_eq!(schemes, ["basicAuth", "bearerAuth"]);
    for extension in [doc_ext, &op("/old")["x-schematic"], &op("/open")["x-schematic"]] {
        for member in ["deprecated", "auth", "security"] {
            assert!(extension.get(member).is_none(), "{member} in {extension}");
        }
    }

    let default_policy = exported(&fixture(vec![vec![]], |_| None));
    for member in ["cross_origin", "request_id_header"] {
        assert!(default_policy["x-schematic"].get(member).is_none(), "{member}");
    }
}

/// Imports `document` through the public importer from `format` text.
fn import_text(document: &OpenAPI, format: ExportFormat) -> OpenApiImportResult {
    let text = serialize(document, format).unwrap();
    let source = match format { ExportFormat::Json => OpenApiSource::json(text), ExportFormat::Yaml => OpenApiSource::yaml(text) };
    OpenApiImport::new(source).build().unwrap()
}

#[test]
fn public_import_of_the_exported_json_and_yaml_keeps_shapes_content_types_and_endpoint_errors() {
    let tagged = |name: &str, value: &str| shape(ResponseShape::Json(named(name)), |s| s.json_tag = Some(JsonTag::Field { name: "type".into(), value: json!(value) }));
    let typed = |shape: ResponseShape, content_type: &str| self::shape(shape, |s| s.content_type = Some(content_type.into()));
    let shapes = vec![
        vec![json_shape("Message"), sse(StreamEnd::ConnectionClose), fallback_binary()],
        vec![tagged("Directory", "dir"), tagged("File", "file")],
        vec![typed(ResponseShape::Binary, "audio/mpeg")],
        vec![typed(ResponseShape::Text, "text/markdown")],
    ];
    let api = fixture(shapes.clone(), |index| (index == 0).then(|| ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::empty())));
    let document = export(&api, &ExportOptions::new()).unwrap();
    let first = serde_json::to_value(&document).unwrap();
    // The exporter names `oneOf` members by `$ref`, so import keeps their types.
    assert_eq!(first["paths"]["/r1"]["get"]["responses"]["200"]["content"]["application/json"]["schema"]["oneOf"],
        json!([reference("Directory"), reference("File")]));

    for format in [ExportFormat::Json, ExportFormat::Yaml] {
        let imported = import_text(&document, format);
        assert!(imported.api.errors.is_empty(), "D-20: no hoisted API errors");
        for (index, expected) in shapes.iter().enumerate() {
            let path = format!("/r{index}");
            let endpoint = imported.api.endpoints.iter().find(|endpoint| endpoint.path == path).unwrap();
            assert_eq!(&endpoint.response, expected, "{path}");
            // The export is the resolved table, so 4XX superseded by 404
            // returns as exact entries; every status looks up the same shape.
            let (original, own) = (api.effective_errors(&api.endpoints[index]), imported.api.effective_errors(endpoint));
            assert_eq!(own.entries(), endpoint.errors.as_slice(), "{path}: D-20");
            for status in 400..=599 {
                let looked_up = |table: &schematic_define::ErrorTable| table.lookup(status).map(|entry| (entry.shape.clone(), entry.content_type.clone()));
                assert_eq!(looked_up(&own), looked_up(&original), "{path} {status}");
            }
        }
        let first_errors = &imported.api.endpoints.iter().find(|endpoint| endpoint.path == "/r0").unwrap().errors;
        assert_eq!((first_errors.len(), first_errors.last().map(|entry| entry.status)), (101, Some(StatusMatch::class(5))));
        let second_errors = &imported.api.endpoints.iter().find(|endpoint| endpoint.path == "/r1").unwrap().errors;
        assert_eq!(second_errors, api.effective_errors(&api.endpoints[1]).entries());

        let second = serde_json::to_value(export(&imported.api, &ExportOptions::new()).unwrap()).unwrap();
        for index in 0..shapes.len() {
            assert_eq!(without_error_descriptions(&operation(&second, index)["responses"]), without_error_descriptions(&operation(&first, index)["responses"]), "/r{index}");
        }
        // An error description names its entry's match, which import made exact.
        assert_eq!(operation(&first, 0)["responses"]["418"]["description"], "Declared error 400-499");
        assert_eq!(operation(&second, 0)["responses"]["418"]["description"], "Declared error 418");
        assert_eq!(operation(&second, 1)["responses"]["4XX"]["description"], "Declared error 400-499");
    }
}

/// A responses map whose error responses drop their `description`.
fn without_error_descriptions(responses: &Value) -> Value {
    let mut responses = responses.clone();
    for (key, response) in responses.as_object_mut().unwrap() {
        if !key.starts_with('2') { response.as_object_mut().unwrap().remove("description"); }
    }
    responses
}

#[test]
fn import_reads_only_the_responses_map_so_stream_ends_and_byte_streams_come_back_plain() {
    // Reading `x-schematic` back is out of scope (R26); content type alone
    // never proves a terminator (R28).
    let shapes = vec![
        vec![sse(StreamEnd::EventNamed("message_stop".into()))],
        vec![SuccessShape::new(stream(StreamFraming::Bytes, StreamEnd::ConnectionClose))],
    ];
    let document = export(&fixture(shapes, |_| None), &ExportOptions::new()).unwrap();
    let first = serde_json::to_value(&document).unwrap();
    let imported = import_text(&document, ExportFormat::Json);
    let response = |path: &str| &imported.api.endpoints.iter().find(|endpoint| endpoint.path == path).unwrap().response;

    assert_eq!(response("/r0"), &vec![sse(StreamEnd::ConnectionClose)]);
    assert_eq!(response("/r1"), &vec![SuccessShape::new(ResponseShape::Binary)]);
    let second = serde_json::to_value(export(&imported.api, &ExportOptions::new()).unwrap()).unwrap();
    for index in 0..2 {
        assert_eq!(operation(&second, index)["responses"]["200"]["content"], operation(&first, index)["responses"]["200"]["content"]);
    }
}
