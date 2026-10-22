//! Response model v2 vocabulary through the public API: success-shape
//! distinguishability (R1, R3, R3a, R3b, R3c, R23), declared error tables
//! (R16-R18), builder lowering while `Endpoint::response` holds one
//! `ApiResponse` (R24), and serialized forms (R44).

use schematic_define::response::{normalize_media_type, validate_success_shapes};
use schematic_define::{
    ApiResponse, DefinitionContext, Endpoint, ErrorEntry, ErrorSet, ErrorShape, ErrorTable, JsonTag, ResponseShape, RestApi,
    StatusMatch, StreamEnd, StreamFraming, Success, SuccessShape, TypeRef, definition_type,
};
use serde_json::{Value, json};

definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Record { pub id: u64 });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Receipt { pub job: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Problem { pub message: String });

fn json(name: &str) -> SuccessShape { SuccessShape::new(ResponseShape::Json(TypeRef::Named(name.into()))) }
fn with(mut shape: SuccessShape, edit: impl FnOnce(&mut SuccessShape)) -> SuccessShape { edit(&mut shape); shape }
fn tag(name: &str, value: Value) -> Option<JsonTag> { Some(JsonTag::Field { name: name.into(), value }) }
fn stream(framing: StreamFraming, end: StreamEnd, keep: bool) -> SuccessShape {
    SuccessShape::new(ResponseShape::Stream { framing, end, keep })
}
fn sse() -> StreamFraming { StreamFraming::Sse(TypeRef::Named("Event".into())) }
fn ndjson() -> StreamFraming { StreamFraming::Ndjson(TypeRef::Named("Chunk".into())) }
fn rejects(shapes: &[SuccessShape], needle: &str) {
    let error = validate_success_shapes(shapes).expect_err(needle);
    assert!(error.contains(needle), "{error}\n  expected {needle:?}");
}

#[test]
fn media_types_normalize_case_and_parameters_and_reject_wildcards() {
    assert_eq!(normalize_media_type(" Text/Event-Stream ; charset=UTF-8").unwrap(), "text/event-stream");
    assert_eq!(normalize_media_type("application/problem+json").unwrap(), "application/problem+json");
    for invalid in ["*/*", "image/*", "", "json", "application/", "/json", "text/plain extra"] {
        assert!(normalize_media_type(invalid).is_err(), "{invalid}");
    }
    assert_eq!(json("A").media_type().as_deref(), Some("application/json"));
    assert_eq!(stream(sse(), StreamEnd::ConnectionClose, false).media_type().as_deref(), Some("text/event-stream"));
    assert_eq!(stream(ndjson(), StreamEnd::ConnectionClose, false).media_type().as_deref(), Some("application/x-ndjson"));
    assert_eq!(stream(StreamFraming::Bytes, StreamEnd::ConnectionClose, false).media_type().as_deref(), Some("application/octet-stream"));
    assert_eq!(SuccessShape::new(ResponseShape::Empty).media_type(), None);
    assert_eq!(with(SuccessShape::new(ResponseShape::Binary), |s| s.fallback = true).media_type(), None);
}

#[test]
fn a_shape_set_must_be_nonempty_and_each_media_type_concrete() {
    rejects(&[], "declares no success shape");
    rejects(&[with(json("A"), |s| s.content_type = Some("application/*".into()))], "success shape 1 (JSON): media type `application/*`");
    rejects(&[with(SuccessShape::new(ResponseShape::Empty), |s| s.content_type = Some("text/plain".into()))], "an empty shape has no media type");
    // A single shape is never checked against the reply.
    assert!(validate_success_shapes(&[with(SuccessShape::new(ResponseShape::Binary), |s| s.content_type = Some("audio/mpeg".into()))]).is_ok());
}

#[test]
fn shapes_with_distinct_media_types_or_json_plus_empty_are_selectable() {
    assert!(validate_success_shapes(&[json("Message"), stream(sse(), StreamEnd::EventNamed("message_stop".into()), false)]).is_ok());
    assert!(validate_success_shapes(&[json("Record"), SuccessShape::new(ResponseShape::Empty)]).is_ok());
    assert!(validate_success_shapes(&[stream(ndjson(), StreamEnd::ConnectionClose, false), json("Reply")]).is_ok());
}

#[test]
fn shapes_sharing_a_normalized_media_type_are_ambiguous() {
    // A declared type equal to another shape's default collides.
    rejects(&[SuccessShape::new(ResponseShape::Binary), stream(StreamFraming::Bytes, StreamEnd::ConnectionClose, false)],
        "success shape 1 (binary), success shape 2 (byte stream) share media type `application/octet-stream`");
    // Case and parameters are ignored.
    rejects(&[SuccessShape::new(ResponseShape::Text), with(SuccessShape::new(ResponseShape::Text), |s| s.content_type = Some("TEXT/Plain; charset=utf-8".into()))],
        "share media type `text/plain`");
    rejects(&[SuccessShape::new(ResponseShape::Empty), SuccessShape::new(ResponseShape::Empty)], "at most one empty shape");
}

#[test]
fn declared_statuses_separate_json_shapes_and_are_bounded() {
    let record = with(json("Record"), |s| s.statuses = vec![200]);
    let receipt = with(json("Receipt"), |s| s.statuses = vec![202]);
    assert!(validate_success_shapes(&[record.clone(), receipt.clone()]).is_ok());
    // A status-less shape is chosen for any undeclared 2xx, apart from the declared ones.
    assert!(validate_success_shapes(&[record.clone(), json("Other")]).is_ok());
    rejects(&[record.clone(), with(json("Twin"), |s| s.statuses = vec![201, 200])], "share media type `application/json` for status 200");
    rejects(&[with(json("A"), |s| s.statuses = vec![300])], "declared status 300 is outside 200-299");
    rejects(&[with(json("A"), |s| s.statuses = vec![199])], "declared status 199 is outside 200-299");
    rejects(&[with(json("A"), |s| s.statuses = vec![201, 201])], "declares status 201 more than once");
    rejects(&[with(json("A"), |s| s.statuses = vec![204]), SuccessShape::new(ResponseShape::Empty)], "declares status 204, which selects the empty shape");
    rejects(&[json("A"), with(SuccessShape::new(ResponseShape::Empty), |s| s.statuses = vec![200])], "an empty shape is selected by status 204, not 200");
    // On a single shape, statuses are documentation only.
    assert!(validate_success_shapes(&[with(json("A"), |s| s.statuses = vec![201, 202])]).is_ok());
}

#[test]
fn json_tags_separate_shapes_sharing_status_and_media_type() {
    let directory = with(json("Directory"), |s| s.json_tag = tag("type", json!("commit_directory")));
    let file = with(json("File"), |s| s.json_tag = tag("type", json!("commit_file")));
    assert!(validate_success_shapes(&[directory.clone(), file.clone()]).is_ok());
    let array = with(json("List"), |s| s.json_tag = Some(JsonTag::Array));
    let object = with(json("Item"), |s| s.json_tag = Some(JsonTag::Object));
    assert!(validate_success_shapes(&[array.clone(), object.clone()]).is_ok());

    let separable = "no JSON tag separates them";
    rejects(&[directory.clone(), with(file.clone(), |s| s.json_tag = tag("type", json!("commit_directory")))], separable);
    rejects(&[directory.clone(), with(file.clone(), |s| s.json_tag = tag("kind", json!("commit_file")))], separable);
    rejects(&[directory.clone(), object.clone()], separable);
    rejects(&[directory.clone(), json("Untagged")], separable);
    rejects(&[array.clone(), with(json("Other"), |s| s.json_tag = Some(JsonTag::Array))], separable);
    rejects(&[array, object, json("Third")], separable);
    rejects(&[with(SuccessShape::new(ResponseShape::Text), |s| s.json_tag = Some(JsonTag::Array))], "only a JSON shape takes a JSON tag");
    rejects(&[with(json("A"), |s| s.json_tag = tag("a.b", json!(1)))], "must be one nonempty top-level field name");
    rejects(&[with(json("A"), |s| s.json_tag = tag("", json!(1)))], "must be one nonempty top-level field name");
    rejects(&[with(json("A"), |s| s.json_tag = tag("type", json!(["x"])))], "value must be a string, number, or boolean");
    rejects(&[with(json("A"), |s| s.json_tag = tag("type", Value::Null))], "value must be a string, number, or boolean");
    // Distinct statuses need no tag even when media types match.
    assert!(validate_success_shapes(&[with(directory, |s| s.statuses = vec![200]), with(json("Plain"), |s| s.statuses = vec![201])]).is_ok());
}

#[test]
fn one_text_or_binary_fallback_takes_unmatched_media_types() {
    let fallback = with(SuccessShape::new(ResponseShape::Binary), |s| s.fallback = true);
    assert!(validate_success_shapes(&[json("Listing"), fallback.clone()]).is_ok());
    rejects(&[json("Listing"), fallback.clone(), with(SuccessShape::new(ResponseShape::Text), |s| s.fallback = true)], "both the fallback");
    rejects(std::slice::from_ref(&fallback), "a fallback on a single-shape endpoint is redundant");
    rejects(&[json("Listing"), with(fallback.clone(), |s| s.content_type = Some("image/png".into()))], "the fallback declares no content type");
    rejects(&[json("Listing"), with(json("Other"), |s| s.fallback = true)], "only a text or binary shape may be the fallback");
}

#[test]
fn stream_end_conditions_must_suit_their_framing() {
    let ok = |framing, end, keep| validate_success_shapes(&[stream(framing, end, keep)]).is_ok();
    assert!(ok(sse(), StreamEnd::DataLineEquals("[DONE]".into()), false));
    assert!(ok(sse(), StreamEnd::DataLineEquals(String::new()), false));
    assert!(ok(sse(), StreamEnd::EventNamed("chat.end".into()), true));
    assert!(ok(ndjson(), StreamEnd::FieldEquals { field: "done".into(), value: json!(true) }, true));
    assert!(ok(sse(), StreamEnd::FieldEquals { field: "message.status".into(), value: json!("") }, false));
    assert!(ok(StreamFraming::Bytes, StreamEnd::ConnectionClose, false));

    let fails = |framing, end, keep, needle: &str| rejects(&[stream(framing, end, keep)], needle);
    fails(ndjson(), StreamEnd::DataLineEquals("[DONE]".into()), false, "a data-line terminator needs SSE framing");
    fails(StreamFraming::Bytes, StreamEnd::EventNamed("end".into()), false, "an event-name terminator needs SSE framing");
    fails(sse(), StreamEnd::EventNamed(" ".into()), false, "needs a nonempty event name");
    fails(StreamFraming::Bytes, StreamEnd::FieldEquals { field: "done".into(), value: json!(true) }, false, "a field terminator needs SSE or NDJSON framing");
    fails(ndjson(), StreamEnd::ConnectionClose, true, "`keep` needs a terminator");
    fails(ndjson(), StreamEnd::FieldEquals { field: "items[*].done".into(), value: json!(true) }, false, "without array fan-out");
    fails(ndjson(), StreamEnd::FieldEquals { field: "a..b".into(), value: json!(true) }, false, "without array fan-out");
    fails(ndjson(), StreamEnd::FieldEquals { field: "done".into(), value: json!({"x": 1}) }, false, "value must be a string, number, or boolean");
}

#[test]
fn error_tables_overlay_api_defaults_and_resolve_by_specificity() {
    let entry = |status, shape| ErrorEntry::new(status, shape);
    let api = [entry(StatusMatch::exact(404), ResponseShape::Text), entry(StatusMatch::class(5), ResponseShape::Binary),
        entry(StatusMatch::range(400, 449), ResponseShape::Empty)];
    let own = [entry(StatusMatch::class(4), ResponseShape::Json(TypeRef::Named("Problem".into()))),
        entry(StatusMatch::class(5), ResponseShape::Text)];
    let table = ErrorTable::resolve(&api, &own, false);
    let shape = |status| table.lookup(status).map(|entry| entry.shape.clone());
    // An endpoint range does not displace the API's exact entry inside it.
    assert_eq!(shape(404), Some(ResponseShape::Text));
    // The narrower API range beats the wider endpoint class.
    assert_eq!(shape(420), Some(ResponseShape::Empty));
    assert_eq!(shape(450), Some(ResponseShape::Json(TypeRef::Named("Problem".into()))));
    // An identical match replaces the API entry in place.
    assert_eq!(shape(503), Some(ResponseShape::Text));
    assert_eq!(table.entries().len(), 4);
    assert_eq!(shape(399), None);
    assert_eq!(shape(600), None);
    assert!(ErrorTable::resolve(&api, &[], true).lookup(404).is_none());
}

fn api_with(errors: ErrorSet, endpoint_errors: Option<ErrorSet>) -> Result<RestApi, String> {
    let mut ctx = DefinitionContext::types_in("::models");
    let mut builder = Endpoint::get("GetRecord", "/records/{id}").describes("Get a record").returns_json::<Record>(&mut ctx);
    if let Some(set) = endpoint_errors { builder = builder.errors(set); }
    let endpoint = builder.build().map_err(|error| error.to_string())?;
    RestApi::builder(&ctx, "Records", "https://api.example.com").describes("Records").no_auth()
        .default_errors(errors).endpoints([endpoint]).build().map_err(|error| error.to_string())
}

#[test]
fn error_entries_are_bounded_typed_and_unambiguous_with_context() {
    let none = || ErrorSet::new();
    let failing = [
        (none().on(StatusMatch::exact(399), ErrorShape::text()), "Records endpoint <api>: error status match 399 is outside 400-599"),
        (none().on(StatusMatch::exact(600), ErrorShape::text()), "error status match 600 is outside 400-599"),
        (none().on(StatusMatch::range(499, 400), ErrorShape::text()), "error status range 499-400 is reversed"),
        (none().on(StatusMatch::class(3), ErrorShape::text()), "error status match 300-399 is outside 400-599"),
        (none().on(StatusMatch::exact(404), ErrorShape::empty().content_type("text/plain")), "error 404: an empty shape has no media type"),
        (none().on(StatusMatch::exact(404), ErrorShape::binary().content_type("*/*")), "error 404: media type `*/*`"),
        (none().on(StatusMatch::exact(404), ErrorShape::text()).on(StatusMatch::exact(404), ErrorShape::binary()), "error status match 404 is declared more than once"),
        (none().on(StatusMatch::range(400, 449), ErrorShape::text()).on(StatusMatch::range(420, 469), ErrorShape::text()),
            "error ranges 400-449 and 420-469 have equal width and overlap on statuses 420-449"),
    ];
    for (set, needle) in failing {
        let error = api_with(set, None).expect_err(needle);
        assert!(error.contains(needle), "{error}\n  expected {needle:?}");
    }
    // Equal-width ranges across levels overlap in the resolved table.
    let error = api_with(none().on(StatusMatch::range(400, 449), ErrorShape::text()),
        Some(none().on(StatusMatch::range(420, 469), ErrorShape::empty()))).unwrap_err();
    assert!(error.contains("Records endpoint GetRecord: error ranges 400-449 and 420-469 have equal width and overlap on statuses 420-449"), "{error}");
    // Opting out and adding entries contradict each other, at endpoint build.
    let error = Endpoint::get("Get", "/").describes("Get").returns_empty()
        .errors(ErrorSet::none()).errors(none().on(StatusMatch::exact(404), ErrorShape::text())).build().unwrap_err().to_string();
    assert!(error.contains("<endpoint> endpoint Get: opts out of declared errors and also declares error entries"), "{error}");
    // A stream is not an error shape, even through deserialized storage.
    let mut api = api_with(none(), None).unwrap();
    api.errors = vec![ErrorEntry::new(StatusMatch::exact(500), ResponseShape::Stream { framing: StreamFraming::Bytes, end: StreamEnd::ConnectionClose, keep: false })];
    let error = schematic_define::DefinitionBundle::new(api).unwrap_err().to_string();
    assert!(error.contains("error 500: a stream cannot be an error shape"), "{error}");
}

#[test]
fn declared_errors_reach_the_effective_table_and_references_are_checked() {
    let mut ctx = DefinitionContext::types_in("::models");
    let problem = ctx.register::<Problem>().unwrap();
    let inherit = Endpoint::get("Inherit", "/a").describes("Inherits").returns_empty().build().unwrap();
    let overlay = Endpoint::get("Overlay", "/b").describes("Overlays").returns_empty()
        .errors(ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::empty())).build().unwrap();
    let opted_out = Endpoint::get("OptOut", "/c").describes("Opts out").returns_empty().errors(ErrorSet::none()).build().unwrap();
    let api = RestApi::builder(&ctx, "Errors", "https://api.example.com").describes("Errors").no_auth()
        .default_errors(ErrorSet::new().on(StatusMatch::class(4), ErrorShape::json(problem.clone())).on(StatusMatch::exact(404), ErrorShape::text()))
        .endpoints([inherit, overlay, opted_out]).build().unwrap();
    let [inherit, overlay, opted_out] = [0, 1, 2].map(|index| api.effective_errors(&api.endpoints[index]));
    assert_eq!(inherit.lookup(404).unwrap().shape, ResponseShape::Text);
    assert_eq!(inherit.lookup(418).unwrap().shape, ResponseShape::Json(problem.type_ref().clone()));
    assert_eq!(overlay.lookup(404).unwrap().shape, ResponseShape::Empty);
    assert_eq!(overlay.entries().len(), 2);
    assert!(opted_out.entries().is_empty());
    assert!(api.endpoints[2].no_declared_errors && !api.endpoints[0].no_declared_errors);

    // An error type from another context is a dangling reference with context.
    let mut other = DefinitionContext::types_in("::other");
    let foreign = other.register::<Receipt>().unwrap();
    let endpoint = Endpoint::get("Get", "/").describes("Get").returns_empty()
        .errors(ErrorSet::new().on(StatusMatch::exact(409), ErrorShape::json(foreign.clone()))).build().unwrap();
    let error = RestApi::builder(&ctx, "Errors", "https://api.example.com").describes("Errors").no_auth()
        .endpoints([endpoint]).build().unwrap_err().to_string();
    assert_eq!(error, "Errors endpoint Get: unresolved schema reference \"Receipt\"");
    let error = RestApi::builder(&ctx, "Errors", "https://api.example.com").describes("Errors").no_auth()
        .default_errors(ErrorSet::new().on(StatusMatch::exact(409), ErrorShape::json(foreign))).build().unwrap_err().to_string();
    assert_eq!(error, "Errors endpoint <api>: unresolved schema reference \"Receipt\"");
}

#[test]
fn builder_keeps_single_plain_successes_and_refuses_what_storage_cannot_hold() {
    let mut ctx = DefinitionContext::types_in("::models");
    let record = ctx.register::<Record>().unwrap();
    let stored = |success: Success| Endpoint::get("Get", "/").describes("Get").returns(success).build().map(|endpoint| endpoint.response);
    assert_eq!(stored(Success::json(record.clone())).unwrap(), ApiResponse::Json(record.type_ref().clone()));
    assert_eq!(stored(Success::text()).unwrap(), ApiResponse::Text);
    assert_eq!(stored(Success::binary()).unwrap(), ApiResponse::Binary);
    assert_eq!(stored(Success::empty()).unwrap(), ApiResponse::Empty);

    let refused = "are stored once Endpoint::response holds response model v2 shapes";
    for success in [Success::sse(record.clone()).ends_at_event("done"), Success::ndjson(record.clone()), Success::bytes(),
        Success::binary().content_type("audio/mpeg"), Success::json(record.clone()).status(201), Success::json(record.clone()).note("always"),
        Success::json(record.clone()).tagged("type", "record")] {
        let error = stored(success).unwrap_err().to_string();
        assert!(error.contains(refused), "{error}");
    }
    // Validation runs before storage, so an invalid shape reports its own problem.
    let error = stored(Success::ndjson(record.clone()).ends_at_event("done")).unwrap_err().to_string();
    assert!(error.contains("<endpoint> endpoint Get: success shape 1 (NDJSON stream): an event-name terminator needs SSE framing"), "{error}");
    let error = stored(Success::json(record.clone()).ends_at_data_line("[DONE]")).unwrap_err().to_string();
    assert!(error.contains("an `ends_*` terminator applies only to a stream shape"), "{error}");
    let error = stored(Success::text().keep_end()).unwrap_err().to_string();
    assert!(error.contains("`keep_end` applies only to a stream shape"), "{error}");
}

#[test]
fn several_shapes_are_validated_at_build_with_inferred_json_tags() {
    let mut ctx = DefinitionContext::types_in("::models");
    let refused = "are stored once Endpoint::response holds response model v2 shapes";
    // A sequence and a struct infer array and object tags.
    let error = Endpoint::get("Contents", "/contents").describes("A directory or a file")
        .returns_json::<Vec<Record>>(&mut ctx).returns_json::<Record>(&mut ctx).build().unwrap_err().to_string();
    assert!(error.contains(refused), "{error}");
    // Two structs infer nothing and are inseparable.
    let error = Endpoint::get("Either", "/either").describes("Either")
        .returns_json::<Record>(&mut ctx).returns_json::<Receipt>(&mut ctx).build().unwrap_err().to_string();
    assert!(error.contains("<endpoint> endpoint Either: success shape 1 (JSON), success shape 2 (JSON) share media type `application/json`")
        && error.contains("no JSON tag separates them"), "{error}");
    // An explicit tag on every shape separates them.
    let (record, receipt) = (ctx.register::<Record>().unwrap(), ctx.register::<Receipt>().unwrap());
    let error = Endpoint::get("Tagged", "/tagged").describes("Tagged")
        .returns(Success::json(record.clone()).tagged("kind", "record")).returns(Success::json(receipt.clone()).tagged("kind", "receipt"))
        .build().unwrap_err().to_string();
    assert!(error.contains(refused), "{error}");
    // A declared tag is never replaced by an inferred one.
    let error = Endpoint::get("Mixed", "/mixed").describes("Mixed")
        .returns_json::<Vec<Record>>(&mut ctx).returns(Success::json(record).tagged("kind", "record")).build().unwrap_err().to_string();
    assert!(error.contains("no JSON tag separates them"), "{error}");
}

#[test]
fn new_definition_members_are_omitted_until_used_and_keep_their_frozen_forms() {
    let mut ctx = DefinitionContext::types_in("::models");
    let plain = Endpoint::get("Get", "/").describes("Get").returns_json::<Record>(&mut ctx).build().unwrap();
    let value = serde_json::to_value(&plain).unwrap();
    for member in ["errors", "no_declared_errors"] { assert!(value.get(member).is_none(), "{member}"); }
    let api = RestApi::builder(&ctx, "Api", "https://api.example.com").describes("Api").no_auth()
        .default_errors(ErrorSet::none()).endpoints([plain]).build().unwrap();
    let value = serde_json::to_value(&api).unwrap();
    for member in ["errors", "graphql"] { assert!(value.get(member).is_none(), "{member}"); }
    let opted_out = Endpoint::get("Get", "/").describes("Get").returns_empty().errors(ErrorSet::none()).build().unwrap();
    assert_eq!(serde_json::to_value(&opted_out).unwrap()["no_declared_errors"], json!(true));

    let named = || TypeRef::Named("Event".into());
    let forms: [(Value, Value); 12] = [
        (serde_json::to_value(StatusMatch::exact(404)).unwrap(), json!({"Exact": 404})),
        (serde_json::to_value(StatusMatch::class(4)).unwrap(), json!({"Range": {"lo": 400, "hi": 499}})),
        (serde_json::to_value(ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Text)).unwrap(), json!({"status": {"Exact": 404}, "shape": "Text"})),
        (serde_json::to_value(StreamEnd::DataLineEquals("[DONE]".into())).unwrap(), json!({"DataLineEquals": "[DONE]"})),
        (serde_json::to_value(StreamEnd::EventNamed("message_stop".into())).unwrap(), json!({"EventNamed": "message_stop"})),
        (serde_json::to_value(StreamEnd::FieldEquals { field: "done".into(), value: json!(true) }).unwrap(), json!({"FieldEquals": {"field": "done", "value": true}})),
        (serde_json::to_value(StreamEnd::ConnectionClose).unwrap(), json!("ConnectionClose")),
        (serde_json::to_value(ResponseShape::Stream { framing: StreamFraming::Sse(named()), end: StreamEnd::ConnectionClose, keep: false }).unwrap(),
            json!({"Stream": {"framing": {"Sse": {"Named": "Event"}}, "end": "ConnectionClose"}})),
        (serde_json::to_value(ResponseShape::Stream { framing: StreamFraming::Ndjson(named()), end: StreamEnd::FieldEquals { field: "done".into(), value: json!(true) }, keep: true }).unwrap(),
            json!({"Stream": {"framing": {"Ndjson": {"Named": "Event"}}, "end": {"FieldEquals": {"field": "done", "value": true}}, "keep": true}})),
        (serde_json::to_value(StreamFraming::Bytes).unwrap(), json!("Bytes")),
        (serde_json::to_value(with(json("Directory"), |s| { s.json_tag = tag("type", json!("commit_directory")); s.statuses = vec![200]; s.note = Some("a directory".into()); s.content_type = Some("application/json".into()); })).unwrap(),
            json!({"shape": {"Json": {"Named": "Directory"}}, "content_type": "application/json", "statuses": [200], "json_tag": {"Field": {"name": "type", "value": "commit_directory"}}, "note": "a directory"})),
        (serde_json::to_value([with(SuccessShape::new(ResponseShape::Binary), |s| s.fallback = true), with(json("L"), |s| s.json_tag = Some(JsonTag::Array))]).unwrap(),
            json!([{"shape": "Binary", "fallback": true}, {"shape": {"Json": {"Named": "L"}}, "json_tag": "Array"}])),
    ];
    for (actual, expected) in forms { assert_eq!(actual, expected); }

    // Each form reads back equal; explicit null and a missing required end fail.
    let shape: SuccessShape = serde_json::from_value(json!({"shape": {"Stream": {"framing": "Bytes", "end": "ConnectionClose"}}})).unwrap();
    assert_eq!(shape, stream(StreamFraming::Bytes, StreamEnd::ConnectionClose, false));
    assert!(serde_json::from_value::<SuccessShape>(json!({"shape": {"Stream": {"framing": "Bytes"}}})).is_err());
    for member in ["content_type", "json_tag", "note"] {
        assert!(serde_json::from_value::<SuccessShape>(json!({"shape": "Text", member: null})).is_err(), "{member}");
    }
    let entry: ErrorEntry = serde_json::from_value(json!({"status": {"Range": {"lo": 500, "hi": 599}}, "shape": "Empty"})).unwrap();
    assert_eq!(entry, ErrorEntry::new(StatusMatch::class(5), ResponseShape::Empty));
}
