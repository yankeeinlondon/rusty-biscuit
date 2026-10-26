//! Runnable Postman collections (spec `2026-10-18-schematic-fidelity-polish`,
//! R19 to R24 and R55).
//!
//! Every JSON request body and saved JSON response of every shipped API is
//! validated against the API's original registered schemas
//! ([`RestApi::schemas`]) with `jsonschema` (Draft 2020-12, offline, local
//! references only), never against the OpenAPI conversion; stream examples
//! are validated frame by frame. Fixture APIs pin the request mapping:
//! text and binary content types, header and cookie parameters with their
//! precedence rules, saved-response statuses, GraphQL operations, and the
//! optional environment file.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use schematic_define::response::{JsonTag, ResponseShape, StreamEnd, StreamFraming, SuccessShape};
use schematic_define::{
    AddressRule, ApiRequest, AuthStrategy, DefinitionContext, Endpoint, EndpointParams, ErrorEntry, GraphQlSurface,
    ParamDef, ParamStyle, QueryParamType, RestApi, SchemaComponent, StatusMatch, TypeRef,
};
use schematic_define::auth::ApiKeyLocation;
use schematic_define::graphql::{GraphQlOperation, GraphQlVariables};
use schematic_gen::diagnostics::{Feature, Output, Severity};
use schematic_define::openapi::ExportOptions;
use schematic_gen::export::openapi::build_openapi_grouped;
use schematic_gen::export::postman::{build_postman_collection, build_postman_collection_grouped, build_postman_environment};
use schematic_gen::pipeline::{BatchScope, GenerateOpts, OpenApiFormat, generate_batch};
use schematic_gen::validate_openapi;
use serde_json::{Value, json};

use crate::postman_schema::validate_postman_json;

// ---------------------------------------------------------------------------
// Validation against the registered schemas
// ---------------------------------------------------------------------------

/// `schema` with each `{"type": T, "nullable": true}` as `{"type": [T, "null"]}`,
/// the OpenAPI 3.0 spelling no JSON Schema draft reads.
fn without_nullable(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            if object.remove("nullable") == Some(Value::Bool(true))
                && let Some(Value::String(kind)) = object.get("type").cloned()
            {
                object.insert("type".into(), json!([kind, "null"]));
            }
            for (key, child) in object.iter_mut() {
                if !["const", "default", "enum", "example", "examples"].contains(&key.as_str()) {
                    without_nullable(child);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(without_nullable),
        _ => {}
    }
}

/// Where the API's components are registered for the test's validators.
const COMPONENTS_URI: &str = "https://components.test/api";

/// `#…` references made absolute against [`COMPONENTS_URI`], each JSON
/// pointer percent-encoded (component names hold `<` and `>`).
fn absolute_references(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref")
                && let Some(pointer) = reference.strip_prefix('#')
            {
                let encoded: String = pointer
                    .bytes()
                    .map(|byte| if byte.is_ascii_alphanumeric() || b"-._~/%".contains(&byte) { (byte as char).to_string() } else { format!("%{byte:02X}") })
                    .collect();
                *reference = format!("{COMPONENTS_URI}#{encoded}");
            }
            for (key, child) in object.iter_mut() {
                if !["const", "default", "enum", "example", "examples"].contains(&key.as_str()) {
                    absolute_references(child);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(absolute_references),
        _ => {}
    }
}

/// Validators for one API's references against its registered components,
/// which are registered once; each reference's own schema compiles once.
struct Checker {
    registry: jsonschema::Registry<'static>,
    validators: HashMap<String, jsonschema::Validator>,
}

impl Checker {
    fn new(api: &RestApi) -> Self {
        let components: serde_json::Map<String, Value> =
            api.schemas.iter().map(|component| (component.name.clone(), component.schema.clone())).collect();
        let mut document = json!({"components": {"schemas": components}});
        without_nullable(&mut document);
        absolute_references(&mut document);
        let registry = jsonschema::Registry::new()
            .draft(jsonschema::Draft::Draft202012)
            .add(COMPONENTS_URI, document)
            .and_then(|builder| builder.prepare())
            .unwrap_or_else(|error| panic!("{}: components do not register: {error}", api.name));
        Self { registry, validators: HashMap::new() }
    }

    fn check(&mut self, reference: &TypeRef, instance: &Value) -> Result<(), String> {
        let mut schema = reference.wire_schema();
        without_nullable(&mut schema);
        absolute_references(&mut schema);
        let key = schema.to_string();
        if !self.validators.contains_key(&key) {
            let validator = jsonschema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .with_registry(&self.registry)
                .offline()
                .build(&schema)
                .map_err(|error| format!("{reference:?} does not compile: {error}"))?;
            self.validators.insert(key.clone(), validator);
        }
        match self.validators[&key].iter_errors(instance).next() {
            None => Ok(()),
            Some(error) => Err(format!("{instance} fails {reference:?}: {error} at {}", error.instance_path())),
        }
    }
}

/// Every request item of a collection, folders flattened.
fn requests(collection: &Value) -> Vec<&Value> {
    fn walk<'v>(items: &'v Value, out: &mut Vec<&'v Value>) {
        for item in items.as_array().into_iter().flatten() {
            if item.get("request").is_some() {
                out.push(item);
            } else {
                walk(&item["item"], out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&collection["item"], &mut out);
    out
}

fn request<'v>(collection: &'v Value, name: &str) -> &'v Value {
    requests(collection).into_iter().find(|item| item["name"] == name).unwrap_or_else(|| panic!("no request {name} in {collection:#}"))
}

/// Checks each frame of a stream example: every SSE `data:` frame or NDJSON
/// line is an item, except a terminator the runtime consumes.
fn check_stream(checker: &mut Checker, framing: &StreamFraming, end: &StreamEnd, keep: bool, body: &str) -> Result<usize, String> {
    let mut frames = 0;
    match framing {
        StreamFraming::Sse(item) => {
            for event in body.split("\n\n").filter(|event| !event.is_empty()) {
                let name = event.lines().find_map(|line| line.strip_prefix("event: "));
                let data = event.lines().find_map(|line| line.strip_prefix("data: ")).ok_or_else(|| format!("no data line in {event:?}"))?;
                let terminator = match end {
                    StreamEnd::DataLineEquals(marker) => data == marker,
                    StreamEnd::EventNamed(event_name) => name == Some(event_name.as_str()) && !keep,
                    _ => false,
                };
                if !terminator {
                    checker.check(item, &serde_json::from_str(data).map_err(|error| format!("{data}: {error}"))?)?;
                    frames += 1;
                }
            }
        }
        StreamFraming::Ndjson(item) => {
            for line in body.lines() {
                checker.check(item, &serde_json::from_str(line).map_err(|error| format!("{line}: {error}"))?)?;
                frames += 1;
            }
        }
        _ => {}
    }
    Ok(frames)
}

/// Validates every JSON request body, saved JSON response, and stream frame
/// of `api`'s collection, and returns how many values were checked.
fn check_api(api: &RestApi) -> usize {
    let collection = serde_json::to_value(build_postman_collection(api)).unwrap();
    validate_postman_json(&collection).unwrap_or_else(|errors| panic!("{}: {errors:#?}", api.name));
    let mut checker = Checker::new(api);
    let mut checked = 0;
    let mut failures = Vec::new();
    for endpoint in &api.endpoints {
        let item = request(&collection, &endpoint.id);
        if let Some(ApiRequest::Json(reference)) = &endpoint.request {
            let raw = item["request"]["body"]["raw"].as_str().unwrap_or_default();
            // No accepted gap exists: every shipped JSON request has an example.
            match serde_json::from_str::<Value>(raw).map_err(|error| format!("{raw:?}: {error}")).and_then(|body| checker.check(reference, &body)) {
                Ok(()) => checked += 1,
                Err(error) => failures.push(format!("{} request body: {error}", endpoint.id)),
            }
        }
        let responses = item["response"].as_array().cloned().unwrap_or_default();
        let shapes = endpoint.success_shapes();
        let table = api.effective_errors(endpoint);
        for (index, response) in responses.iter().enumerate() {
            let code = response["code"].as_u64().unwrap() as u16;
            let (shape, content_type) = match shapes.get(index) {
                Some(success) => (success.shape.clone(), None),
                None => {
                    let entry = table.lookup(code).unwrap_or_else(|| panic!("{} saves {code}, which no error entry matches", endpoint.id));
                    (entry.shape.clone(), entry.content_type.clone())
                }
            };
            let _ = content_type;
            let body = response["body"].as_str();
            let result = match (&shape, body) {
                (ResponseShape::Json(reference), Some(body)) => {
                    serde_json::from_str::<Value>(body).map_err(|error| error.to_string()).and_then(|body| checker.check(reference, &body)).map(|()| 1)
                }
                (ResponseShape::Stream { framing, end, keep }, Some(body)) => check_stream(&mut checker, framing, end, *keep, body),
                (ResponseShape::Empty, None) => Ok(0),
                (ResponseShape::Empty, Some(body)) => Err(format!("an empty shape saved the body {body:?}")),
                (ResponseShape::Json(_), None) => Err("a JSON shape saved no body".to_string()),
                _ => Ok(0),
            };
            match result {
                Ok(count) => checked += count,
                Err(error) => failures.push(format!("{} response {code}: {error}", endpoint.id)),
            }
        }
    }
    assert!(failures.is_empty(), "{}:\n{}", api.name, failures.join("\n"));
    checked
}

fn check_module(module: &str) {
    let modules = schematic_definitions::apis_by_module();
    let apis = modules.get(module).unwrap_or_else(|| panic!("no shipped module {module}"));
    let checked: usize = apis.iter().map(check_api).sum();
    assert!(checked > 0 || apis.iter().all(|api| api.endpoints.iter().all(|e| e.json_references().is_empty())), "{module}");
}

macro_rules! shipped_modules {
    ($($module:ident),* $(,)?) => {
        /// One test per shipped module, so the modules validate in parallel.
        mod every_shipped_example_validates {
            $(#[test] fn $module() { super::check_module(stringify!($module)); })*
        }
        const SHIPPED_MODULES: &[&str] = &[$(stringify!($module)),*];
    };
}

shipped_modules!(
    anthropic, artificial_analysis, bitbucket, elevenlabs, emqx, eversolo, gitea, github, gitlab, huggingface,
    lmstudio, ollama, openai, samsung_smart_tv, unfolded_circle_core_rest,
);

#[test]
fn the_validated_module_list_is_every_shipped_module() {
    let shipped: BTreeSet<String> = schematic_definitions::apis_by_module().keys().cloned().collect();
    let listed: BTreeSet<String> = SHIPPED_MODULES.iter().map(|module| module.to_string()).collect();
    assert_eq!(listed, shipped, "add the new module to `shipped_modules!`");
}

#[test]
fn shipped_collections_report_no_example_gaps_and_build_the_same_bytes_twice() {
    for api in schematic_definitions::all_apis() {
        let gaps: Vec<_> = schematic_gen::validate_postman(&[&api]).into_iter().filter(|d| d.feature == Feature::ExampleValue).collect();
        assert!(gaps.is_empty(), "{gaps:#?}");
    }
    let api = schematic_definitions::all_apis().into_iter().find(|api| api.name == "Anthropic").unwrap();
    let first = serde_json::to_string(&build_postman_collection(&api)).unwrap();
    assert_eq!(first, serde_json::to_string(&build_postman_collection(&api)).unwrap());
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn component(name: &str, schema: Value) -> SchemaComponent {
    SchemaComponent { name: name.into(), rust_path: Some(format!("crate::{name}")), schema }
}

fn api(name: &str, auth: AuthStrategy, endpoints: Vec<Endpoint>, schemas: Vec<SchemaComponent>) -> RestApi {
    let context = DefinitionContext::types_in("::fixture");
    let mut api = RestApi::builder(&context, name, "https://fixture.example/v1").describes(format!("{name} fixture")).no_auth().build().unwrap();
    api.auth = auth;
    api.endpoints = endpoints;
    api.schemas = schemas;
    api
}

fn endpoint(id: &str, path: &str) -> Endpoint {
    let mut endpoint = Endpoint::get(id, path).describes(id).returns_empty().build().unwrap();
    endpoint.response = vec![SuccessShape::empty()];
    endpoint
}

fn param(name: &str, required: bool) -> ParamDef {
    ParamDef { constraints: None,
        name: name.into(), rust_name: None, secret: false, pager_managed: false, deprecated: false, required,
        description: None, param_type: QueryParamType::String, explode: false, style: ParamStyle::Form,
    }
}

fn collection_of(api: &RestApi) -> Value {
    let collection = serde_json::to_value(build_postman_collection(api)).unwrap();
    validate_postman_json(&collection).unwrap_or_else(|errors| panic!("{errors:#?}"));
    collection
}

fn headers(item: &Value) -> Vec<(String, String, bool)> {
    item["request"]["header"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|h| (h["key"].as_str().unwrap().into(), h["value"].as_str().unwrap().into(), h["disabled"].as_bool().unwrap_or(false)))
        .collect()
}

fn variable_keys(collection: &Value) -> Vec<String> {
    collection["variable"].as_array().unwrap().iter().map(|v| v["key"].as_str().unwrap().to_string()).collect()
}

// ---------------------------------------------------------------------------
// R23: text and binary bodies keep their content types
// ---------------------------------------------------------------------------

#[test]
fn text_and_binary_bodies_keep_their_declared_content_type() {
    let mut text = endpoint("UploadCsv", "/csv");
    text.request = Some(ApiRequest::Text { content_type: "text/csv".into() });
    let mut binary = endpoint("UploadAudio", "/audio");
    binary.request = Some(ApiRequest::Binary { content_type: "audio/mpeg".into() });
    // A fixed Content-Type header wins over the body's.
    let mut fixed = endpoint("UploadFixed", "/fixed");
    fixed.request = Some(ApiRequest::Binary { content_type: "audio/mpeg".into() });
    fixed.headers = vec![("content-type".into(), "audio/wav".into())];
    let collection = collection_of(&api("Bodies", AuthStrategy::None, vec![text, binary, fixed], vec![]));

    let text = request(&collection, "UploadCsv");
    assert_eq!(text["request"]["body"], json!({"mode": "raw", "raw": "", "options": {"raw": {"language": "text"}}}));
    assert_eq!(headers(text), [("Content-Type".to_string(), "text/csv".to_string(), false)]);

    let binary = request(&collection, "UploadAudio");
    assert_eq!(binary["request"]["body"], json!({"mode": "file"}));
    assert_eq!(headers(binary), [("Content-Type".to_string(), "audio/mpeg".to_string(), false)]);

    assert_eq!(headers(request(&collection, "UploadFixed")), [("content-type".to_string(), "audio/wav".to_string(), false)]);
}

// ---------------------------------------------------------------------------
// R24: header, query, and cookie parameters
// ---------------------------------------------------------------------------

#[test]
fn parameters_become_declared_variables_with_constant_and_credential_precedence() {
    let mut search = endpoint("Search", "/search/{scope}");
    search.headers = vec![("X-Fixed".into(), "constant".into())];
    search.params = Some(EndpointParams {
        query: vec![param("q", true), param("page", false), param("api_key", false)],
        header: vec![param("X-Trace", true), param("X-Debug", false), param("x-fixed", true), param("X-Key", true), param("Cookie", true)],
        cookie: vec![param("session", true), param("theme", false), param("prefs", true), param("token", true)],
        ..EndpointParams::default()
    });
    // The API key travels in `X-Key`: a header parameter of that name is the
    // credential's, not the user's.
    let header_key = AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: None };
    let collection = collection_of(&api("Params", header_key, vec![search.clone()], vec![]));
    let item = request(&collection, "Search");

    assert_eq!(item["request"]["url"]["variable"], json!([{"key": "scope", "value": "{{path_scope}}"}]));
    assert_eq!(
        item["request"]["url"]["query"],
        json!([
            {"key": "q", "value": "{{query_q}}"},
            {"key": "page", "value": "{{query_page}}", "disabled": true},
            {"key": "api_key", "value": "{{query_api_key}}", "disabled": true},
        ])
    );
    assert_eq!(
        headers(item),
        [
            ("X-Fixed".to_string(), "constant".to_string(), false),
            ("X-Trace".to_string(), "{{header_X-Trace}}".to_string(), false),
            ("X-Debug".to_string(), "{{header_X-Debug}}".to_string(), true),
            ("Cookie".to_string(), "session={{cookie_session}}; prefs={{cookie_prefs}}; token={{cookie_token}}".to_string(), false),
        ]
    );
    assert_eq!(item["request"]["header"][3]["description"], "Optional cookies, sent only when added: theme.");
    assert_eq!(
        variable_keys(&collection),
        ["baseUrl", "apiKey", "path_scope", "query_q", "query_page", "query_api_key", "header_X-Trace", "header_X-Debug", "cookie_session", "cookie_prefs", "cookie_token"]
    );

    // A query API key and a cookie API key take their parameter's place.
    let query_key = AuthStrategy::ApiKeyParam { name: "api_key".into(), location: ApiKeyLocation::Query };
    let collection = collection_of(&api("Params", query_key, vec![search.clone()], vec![]));
    let query: Vec<&str> = request(&collection, "Search")["request"]["url"]["query"].as_array().unwrap().iter().map(|q| q["key"].as_str().unwrap()).collect();
    assert_eq!(query, ["q", "page"]);
    let cookie_key = AuthStrategy::ApiKeyParam { name: "token".into(), location: ApiKeyLocation::Cookie };
    let collection = collection_of(&api("Params", cookie_key, vec![search.clone()], vec![]));
    let cookie = headers(request(&collection, "Search")).into_iter().find(|(key, ..)| key == "Cookie").unwrap();
    assert_eq!(cookie.1, "session={{cookie_session}}; prefs={{cookie_prefs}}");

    // Bearer auth owns `Authorization`; every cookie optional → one disabled header.
    let mut open = endpoint("Open", "/open");
    open.params = Some(EndpointParams { header: vec![param("Authorization", true)], cookie: vec![param("a", false), param("b", false)], ..EndpointParams::default() });
    let collection = collection_of(&api("Params", AuthStrategy::BearerToken { header: None }, vec![open], vec![]));
    assert_eq!(headers(request(&collection, "Open")), [("Cookie".to_string(), "a={{cookie_a}}; b={{cookie_b}}".to_string(), true)]);
}

#[test]
fn parameter_variables_never_collide_with_server_or_credential_variables() {
    let mut login = endpoint("Login", "/login/{username}");
    login.params = Some(EndpointParams { query: vec![param("password", true), param("odd name[]", true)], ..EndpointParams::default() });
    let basic = AuthStrategy::Basic { username_env: schematic_define::EnvList::single("U"), password_env: schematic_define::EnvList::single("P") };
    let collection = collection_of(&api("Clash", basic, vec![login], vec![]));
    assert_eq!(variable_keys(&collection), ["baseUrl", "username", "password", "path_username", "query_password", "query_odd_name__"]);
}

// ---------------------------------------------------------------------------
// R21: saved responses
// ---------------------------------------------------------------------------

fn widget_schemas() -> Vec<SchemaComponent> {
    vec![
        component("Widget", json!({"type": "object", "required": ["id", "kind"], "properties": {"id": {"type": "integer", "minimum": 1}, "kind": {"type": "string"}, "done": {"type": "boolean"}}})),
        component("Problem", json!({"type": "object", "required": ["message"], "properties": {"message": {"type": "string", "examples": ["not found"]}}})),
    ]
}

#[test]
fn saved_responses_cover_each_success_media_type_and_declared_error() {
    let mut get = endpoint("GetWidget", "/widgets/{id}");
    let mut tagged = SuccessShape::json_type("Widget");
    tagged.statuses = vec![200];
    tagged.json_tag = Some(JsonTag::Field { name: "kind".into(), value: json!("gear") });
    tagged.variant = Some("Gear".into());
    let mut created = SuccessShape::json_type("Widget");
    created.statuses = vec![201];
    let mut audio = SuccessShape::binary();
    audio.content_type = Some("audio/mpeg".into());
    get.response = vec![tagged, created, SuccessShape::text(), audio, SuccessShape::empty()];
    let problem = || ResponseShape::Json(TypeRef::Named("Problem".into()));
    // A 4xx range with a narrower 400 entry: the range is saved under 401,
    // the first status it still resolves.
    get.errors = vec![
        ErrorEntry::new(StatusMatch::class(4), problem()),
        ErrorEntry::new(StatusMatch::exact(400), ResponseShape::Text),
        ErrorEntry::new(StatusMatch::range(500, 503), ResponseShape::Empty),
    ];
    let mut fixture = api("Widgets", AuthStrategy::None, vec![get], widget_schemas());
    // An API-level 429 is overlaid by nothing and saved too.
    fixture.errors = vec![ErrorEntry::new(StatusMatch::exact(429), problem())];
    let collection = collection_of(&fixture);
    let responses = request(&collection, "GetWidget")["response"].as_array().unwrap().clone();

    let summary: Vec<(u64, &str, Option<&str>)> = responses
        .iter()
        .map(|r| (r["code"].as_u64().unwrap(), r["name"].as_str().unwrap(), r["header"][0]["value"].as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            (200, "200 OK (application/json) — Gear", Some("application/json")),
            (201, "201 Created (application/json)", Some("application/json")),
            (200, "200 OK (text/plain)", Some("text/plain")),
            (200, "200 OK (audio/mpeg)", Some("audio/mpeg")),
            (204, "204 No Content", None),
            (429, "429 Too Many Requests (application/json)", Some("application/json")),
            (401, "401 Unauthorized (application/json)", Some("application/json")),
            (400, "400 Bad Request (text/plain)", Some("text/plain")),
            (500, "500 Internal Server Error", None),
        ]
    );
    let body = |index: usize| responses[index]["body"].as_str().map(|body| body.to_string());
    assert_eq!(serde_json::from_str::<Value>(&body(0).unwrap()).unwrap(), json!({"id": 1, "kind": "gear"}));
    assert_eq!(body(2).as_deref(), Some("string"));
    assert_eq!(body(3).as_deref(), Some("<binary audio/mpeg body; not representable as an example>"));
    assert_eq!(body(4), None);
    assert_eq!(serde_json::from_str::<Value>(&body(6).unwrap()).unwrap(), json!({"message": "not found"}));
    assert_eq!(check_api(&fixture), 4);
}

#[test]
fn stream_examples_are_finite_and_end_with_their_terminator() {
    let item = TypeRef::Named("Widget".into());
    let stream = |framing, end, keep| SuccessShape::new(ResponseShape::Stream { framing, end, keep });
    let mut sse = endpoint("StreamDone", "/sse");
    sse.response = vec![stream(StreamFraming::Sse(item.clone()), StreamEnd::DataLineEquals("[DONE]".into()), false)];
    let mut named = endpoint("StreamNamed", "/named");
    named.response = vec![stream(StreamFraming::Sse(item.clone()), StreamEnd::EventNamed("stop".into()), false)];
    let mut ndjson = endpoint("StreamLines", "/lines");
    ndjson.response = vec![stream(StreamFraming::Ndjson(item.clone()), StreamEnd::FieldEquals { field: "done".into(), value: json!(true) }, true)];
    let mut bytes = endpoint("StreamBytes", "/bytes");
    bytes.response = vec![stream(StreamFraming::Bytes, StreamEnd::ConnectionClose, false)];
    let fixture = api("Streams", AuthStrategy::None, vec![sse, named, ndjson, bytes], widget_schemas());
    let collection = collection_of(&fixture);
    let body = |name: &str| request(&collection, name)["response"][0]["body"].as_str().unwrap().to_string();

    assert_eq!(body("StreamDone"), "data: {\"id\":1,\"kind\":\"string\"}\n\ndata: [DONE]\n\n");
    assert_eq!(body("StreamNamed"), "data: {\"id\":1,\"kind\":\"string\"}\n\nevent: stop\ndata: {}\n\n");
    assert_eq!(body("StreamLines"), "{\"id\":1,\"kind\":\"string\"}\n{\"done\":true,\"id\":1,\"kind\":\"string\"}\n");
    assert_eq!(body("StreamBytes"), "<binary application/octet-stream body; not representable as an example>");
    // Five item frames validate on their own; the consumed terminators are
    // not items.
    assert_eq!(check_api(&fixture), 4);

    // A terminator the item schema rejects is reported, never emitted.
    let mut broken = endpoint("StreamBroken", "/broken");
    broken.response = vec![stream(StreamFraming::Ndjson(item), StreamEnd::FieldEquals { field: "id".into(), value: json!(0) }, false)];
    let fixture = api("Streams", AuthStrategy::None, vec![broken], widget_schemas());
    let warnings = schematic_gen::validate_postman(&[&fixture]);
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
    assert_eq!((warnings[0].severity, warnings[0].feature, warnings[0].output), (Severity::Warning, Feature::ExampleValue, Output::Postman));
    assert!(warnings[0].message.contains("200 OK (application/x-ndjson) response"), "{}", warnings[0].message);
    assert!(request(&collection_of(&fixture), "StreamBroken")["response"][0].get("body").is_none());
}

// ---------------------------------------------------------------------------
// R19: request bodies
// ---------------------------------------------------------------------------

#[test]
fn json_request_bodies_hold_a_validated_example_or_nothing() {
    let mut create = endpoint("CreateWidget", "/widgets");
    create.request = Some(ApiRequest::json_type("Widget"));
    let mut broken = endpoint("CreateSha", "/sha");
    broken.request = Some(ApiRequest::json_type("Sha"));
    let mut schemas = widget_schemas();
    schemas.push(component("Sha", json!({"type": "object", "required": ["sha"], "properties": {"sha": {"type": "string", "pattern": "^[0-9a-f]{40}$"}}})));
    let fixture = api("Bodies", AuthStrategy::None, vec![create, broken], schemas);
    let collection = collection_of(&fixture);

    assert_eq!(request(&collection, "CreateWidget")["request"]["body"]["raw"], "{\n  \"id\": 1,\n  \"kind\": \"string\"\n}");
    assert_eq!(request(&collection, "CreateSha")["request"]["body"]["raw"], "");
    let warnings = schematic_gen::validate_postman(&[&fixture]);
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
    assert_eq!(warnings[0].endpoint.as_deref(), Some("CreateSha"));
    assert!(warnings[0].message.contains("request body") && warnings[0].message.contains("/sha"), "{}", warnings[0].message);
}

// ---------------------------------------------------------------------------
// R55: GraphQL operations
// ---------------------------------------------------------------------------

fn graphql_api(base_url: &str) -> RestApi {
    let mut ctx = DefinitionContext::types_in("::fixture");
    let mut viewer = GraphQlOperation::query("Viewer", "query { viewer { login } }")
        .describes("The signed-in user").no_variables().data::<Value>(&mut ctx).build().unwrap();
    viewer.data = TypeRef::Named("ViewerData".into());
    let mut issues = GraphQlOperation::query("Issues", "query Issues($owner: String!, $first: Int) { repository(owner: $owner) { issues(first: $first) { totalCount } } }")
        .describes("Issues").no_variables().data::<Value>(&mut ctx).operation_name("Issues").build().unwrap();
    issues.variables = GraphQlVariables::Typed(TypeRef::Named("IssuesVars".into()));
    issues.data = TypeRef::Named("ViewerData".into());
    let mut fixture = api("Hub", AuthStrategy::BearerToken { header: None }, vec![endpoint("GetUser", "/user")], vec![
        component("ViewerData", json!({"type": "object", "required": ["viewer"], "properties": {"viewer": {"type": "object", "required": ["login"], "properties": {"login": {"type": "string"}}}}})),
        component("IssuesVars", json!({"type": "object", "required": ["owner"], "properties": {"owner": {"type": "string", "examples": ["octocat"]}, "first": {"type": "integer", "minimum": 1}}})),
    ]);
    fixture.base_url = base_url.to_string();
    fixture.headers = vec![("X-Protocol-Version".into(), "2".into())];
    let mut surface = GraphQlSurface::new(AddressRule::github());
    surface.operations = vec![viewer, issues];
    fixture.graphql = Some(surface);
    fixture
}

#[test]
fn graphql_operations_are_runnable_posts_and_openapi_skips_each_by_name() {
    let fixture = graphql_api("https://ghe.example/prefix/api/v3/");
    let collection = collection_of(&fixture);
    let folder = collection["item"].as_array().unwrap().iter().find(|item| item["name"] == "GraphQL").expect("GraphQL folder");
    let names: Vec<&str> = folder["item"].as_array().unwrap().iter().map(|item| item["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Viewer", "Issues"]);
    assert!(collection["variable"].as_array().unwrap().contains(&json!({
        "key": "graphqlUrl", "value": "https://ghe.example/prefix/api/graphql", "description": "GraphQL endpoint of Hub",
    })));

    let issues = request(&collection, "Issues");
    assert_eq!(issues["request"]["method"], "POST");
    assert_eq!(issues["request"]["url"]["raw"], "{{graphqlUrl}}");
    assert_eq!(issues["request"]["body"]["options"]["raw"]["language"], "json");
    let body: Value = serde_json::from_str(issues["request"]["body"]["raw"].as_str().unwrap()).unwrap();
    assert_eq!(
        body,
        json!({
            "query": "query Issues($owner: String!, $first: Int) { repository(owner: $owner) { issues(first: $first) { totalCount } } }",
            "operationName": "Issues",
            "variables": {"owner": "octocat"},
        })
    );
    let viewer: Value = serde_json::from_str(request(&collection, "Viewer")["request"]["body"]["raw"].as_str().unwrap()).unwrap();
    assert_eq!(viewer, json!({"query": "query { viewer { login } }", "variables": {}}));
    assert_eq!(
        serde_json::from_str::<Value>(request(&collection, "Viewer")["response"][0]["body"].as_str().unwrap()).unwrap(),
        json!({"data": {"viewer": {"login": "string"}}})
    );

    // OpenAPI: no path for any operation, one named diagnostic each.
    let openapi = serde_json::to_value(build_openapi_grouped(&[&fixture], "hub", &ExportOptions::new()).unwrap()).unwrap();
    let paths: Vec<&String> = openapi["paths"].as_object().unwrap().keys().collect();
    assert_eq!(paths, ["/user"]);
    let skips: Vec<(Severity, Option<String>)> = validate_openapi(&[&fixture])
        .into_iter()
        .filter(|d| d.feature == Feature::GraphQlOperation)
        .map(|d| (d.severity, d.endpoint))
        .collect();
    assert_eq!(skips, [(Severity::Warning, Some("Viewer".to_string())), (Severity::Warning, Some("Issues".to_string()))]);
    // Postman carries them, so it reports none.
    assert!(schematic_gen::validate_postman(&[&fixture]).iter().all(|d| d.feature != Feature::GraphQlOperation));
}

/// Each request as `(name, URL, X-Protocol-Version value)`; the URL names
/// the member's base-URL or GraphQL-address variable.
fn protocol_versions(collection: &Value) -> Vec<(String, String, Option<String>)> {
    requests(collection)
        .into_iter()
        .map(|item| {
            let version = headers(item).into_iter().find(|(key, _, _)| key.eq_ignore_ascii_case("X-Protocol-Version")).map(|(_, value, _)| value);
            (item["name"].as_str().unwrap().into(), item["request"]["url"]["raw"].as_str().unwrap().into(), version)
        })
        .collect()
}

#[test]
fn every_rest_and_graphql_request_carries_the_api_fixed_headers() {
    let fixture = graphql_api("https://hub.example/api/v3");
    let collection = collection_of(&fixture);
    let version = Some("2".to_string());
    assert_eq!(
        protocol_versions(&collection),
        [
            ("GetUser".to_string(), "{{baseUrl}}/user".to_string(), version.clone()),
            ("Viewer".to_string(), "{{graphqlUrl}}".to_string(), version.clone()),
            ("Issues".to_string(), "{{graphqlUrl}}".to_string(), version),
        ]
    );
    // A fixed header joins the body's media type, not replaces it.
    assert_eq!(request(&collection, "Issues")["request"]["body"]["options"]["raw"]["language"], "json");
}

#[test]
fn every_shared_module_member_request_carries_its_own_fixed_headers() {
    let mut first = graphql_api("https://hub.example/api/v3");
    first.module_path = Some("hub".into());
    let mut second = graphql_api("https://ghe.example/api/v3");
    second.name = "Enterprise".into();
    second.module_path = Some("hub".into());
    second.headers = vec![("x-protocol-version".into(), "3".into())];
    let collection = serde_json::to_value(build_postman_collection_grouped(&[&first, &second], "hub")).unwrap();
    validate_postman_json(&collection).unwrap_or_else(|errors| panic!("{errors:#?}"));

    let mut versions = protocol_versions(&collection);
    versions.sort();
    let row = |name: &str, url: &str, version: &str| (name.to_string(), url.to_string(), Some(version.to_string()));
    assert_eq!(
        versions,
        [
            row("GetUser (Enterprise)", "{{baseUrl2}}/user", "3"),
            row("GetUser (Hub)", "{{baseUrl}}/user", "2"),
            row("Issues", "{{graphqlUrl2}}", "3"),
            row("Issues", "{{graphqlUrl}}", "2"),
            row("Viewer", "{{graphqlUrl2}}", "3"),
            row("Viewer", "{{graphqlUrl}}", "2"),
        ]
    );
}

#[test]
fn a_graphql_address_no_rule_derives_is_reported_and_its_operations_left_out() {
    let mut fixture = graphql_api("https://hub.example/api/v3");
    fixture.graphql.as_mut().unwrap().address = vec![AddressRule::replace_suffix("/v4", "/graphql")];
    let warnings: Vec<_> = schematic_gen::validate_postman(&[&fixture]).into_iter().filter(|d| d.feature == Feature::GraphQlOperation).collect();
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
    assert!(warnings[0].message.contains("2 operation(s) are not exported"), "{}", warnings[0].message);
    let collection = collection_of(&fixture);
    assert!(collection["item"].as_array().unwrap().iter().all(|item| item["name"] != "GraphQL"));
    assert!(!variable_keys(&collection).contains(&"graphqlUrl".to_string()));
}

// ---------------------------------------------------------------------------
// R20: environments
// ---------------------------------------------------------------------------

fn generate(api: &RestApi, root: &Path, environment: bool) {
    let postman = root.join("postman");
    let rust = root.join("schema/src");
    let report = generate_batch(
        std::slice::from_ref(api),
        &BatchScope::Complete,
        &GenerateOpts {
            output: rust.to_str().unwrap(),
            dry_run: false,
            verbose: 0,
            openapi_out: None,
            openapi_format: OpenApiFormat::Json,
            openapi_version: None,
            postman_out: Some(postman.to_str().unwrap()),
            no_openapi: true,
            no_postman: false,
            postman_environment: environment,
        },
    );
    assert!(report.failed().is_empty(), "{:#?}", report.outputs);
}

fn file_names(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect()
}

#[test]
fn the_environment_is_optional_and_declares_every_collection_variable_with_empty_credentials() {
    let mut search = endpoint("Search", "/search/{scope}");
    let mut secret = param("X-Secret", true);
    secret.secret = true;
    search.params = Some(EndpointParams { query: vec![param("q", true)], header: vec![secret], ..EndpointParams::default() });
    let mut fixture = api("Searcher", AuthStrategy::BearerToken { header: None }, vec![search], vec![]);
    fixture.module_path = Some("searcher".into());

    let without = tempfile::tempdir().unwrap();
    generate(&fixture, without.path(), false);
    assert_eq!(file_names(&without.path().join("postman")), BTreeSet::from(["searcher.postman_collection.json".to_string()]));
    let collection: Value = serde_json::from_str(&std::fs::read_to_string(without.path().join("postman/searcher.postman_collection.json")).unwrap()).unwrap();

    let with = tempfile::tempdir().unwrap();
    generate(&fixture, with.path(), true);
    let postman = with.path().join("postman");
    assert_eq!(
        file_names(&postman),
        BTreeSet::from(["searcher.postman_collection.json".to_string(), "searcher.postman_environment.json".to_string()])
    );
    // The switch adds the environment; the collection is the same either way.
    assert_eq!(
        std::fs::read_to_string(postman.join("searcher.postman_collection.json")).unwrap(),
        std::fs::read_to_string(without.path().join("postman/searcher.postman_collection.json")).unwrap()
    );
    let environment: Value = serde_json::from_str(&std::fs::read_to_string(postman.join("searcher.postman_environment.json")).unwrap()).unwrap();
    assert_eq!(
        environment,
        json!({
            "name": "Searcher",
            "values": [
                {"key": "baseUrl", "value": "https://fixture.example/v1", "type": "default", "enabled": true},
                {"key": "bearerToken", "value": "", "type": "secret", "enabled": true},
                {"key": "path_scope", "value": "", "type": "default", "enabled": true},
                {"key": "query_q", "value": "", "type": "default", "enabled": true},
                {"key": "header_X-Secret", "value": "", "type": "secret", "enabled": true},
            ],
            "_postman_variable_scope": "environment",
        })
    );

    // Every reference resolves with or without the environment, because the
    // collection declares them all.
    let declared: BTreeSet<String> = variable_keys(&collection).into_iter().collect();
    let text = collection.to_string();
    let referenced: BTreeSet<String> = text.split("{{").skip(1).map(|rest| rest.split("}}").next().unwrap().to_string()).collect();
    assert!(referenced.is_subset(&declared), "{referenced:?} ⊄ {declared:?}");
}

#[test]
fn shipped_environments_mirror_their_collections_with_separate_shared_module_members() {
    for (module, apis) in schematic_definitions::apis_by_module() {
        let refs: Vec<&RestApi> = apis.iter().collect();
        let collection: Value = serde_json::from_str(&schematic_gen::export::postman::render_postman_module(&refs, &module).unwrap()).unwrap();
        let environment = build_postman_environment(&refs, &module);
        let keys: Vec<String> = environment.values.iter().map(|value| value.key.clone()).collect();
        assert_eq!(keys, variable_keys(&collection), "{module}");
        for value in &environment.values {
            if value.type_field == "secret" {
                assert_eq!(value.value, "", "{module} {}", value.key);
            }
        }
    }
    let emqx = schematic_definitions::apis_by_module().get("emqx").unwrap().clone();
    let refs: Vec<&RestApi> = emqx.iter().collect();
    let environment = build_postman_environment(&refs, "emqx");
    let summary: Vec<(&str, &str)> = environment.values.iter().take(5).map(|v| (v.key.as_str(), v.type_field.as_str())).collect();
    assert_eq!(
        summary,
        [
            ("baseUrl", "default"),
            ("baseUrl2", "default"),
            ("username_EmqxBasic", "secret"),
            ("password_EmqxBasic", "secret"),
            ("bearerToken_EmqxBearer", "secret"),
        ]
    );
}
