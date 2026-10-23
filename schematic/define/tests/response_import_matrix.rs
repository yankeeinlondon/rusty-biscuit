//! The OpenAPI input robustness matrix for the response members the v2
//! classification reads (R28): operation `responses`, response `content`,
//! the media entry `schema`, a response `$ref`, `oneOf` members, and
//! `discriminator.propertyName`.
//!
//! The fixture is the real exporter's document for an API with JSON + SSE +
//! `*/*` fallback, a discriminator `oneOf`, a 204, exact and class errors,
//! and one response moved to `components.responses`. Every cell applies one
//! edit to that unedited fixture, as JSON and re-serialized as YAML, and goes
//! through the public importer: a rejection is an import error naming the
//! member. Cells the importer accepts are classified by
//! `classify_responses`, which production import runs per operation. Exact
//! textual duplicate keys exist only in text, so those cells run on JSON;
//! duplicates by spelling (`4XX`/`4xx`, media case) run in both formats.

#![cfg(feature = "openapi")]

use biscuit_file::serde_yaml_ng;
use openapiv3::{OpenAPI, ReferenceOr};
use schematic_define::openapi::import::RefResolver;
use schematic_define::openapi::{ExportOptions, OpenApiImport, OpenApiSource, ResponseClassification, classify_responses, export};
use schematic_define::{
    DefinitionContext, Endpoint, ErrorSet, ErrorShape, JsonTag, PrimitiveType, ResponseShape, RestApi, StatusMatch, StreamEnd,
    StreamFraming, SuccessShape, TypeRef, definition_type,
};
use serde_json::{Value, json};

definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Message { pub text: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Event { pub kind: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Problem { pub message: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Directory { pub entries: Vec<String> });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct File { pub size: u64 });

const RESPONSES: &str = "/paths/~1r0/get/responses";
const CONTENT: &str = "/paths/~1r0/get/responses/200/content";
const SCHEMA: &str = "/paths/~1r0/get/responses/200/content/application~1json/schema";
const REFERENCED: &str = "/paths/~1r0/get/responses/404";
const ONE_OF: &str = "/paths/~1r1/get/responses/200/content/application~1json/schema/oneOf";
const DISCRIMINATOR: &str = "/paths/~1r1/get/responses/200/content/application~1json/schema/discriminator";

fn named(name: &str) -> TypeRef { TypeRef::Named(name.into()) }

/// The exported fixture document, with `Get0`'s 404 moved to
/// `components.responses.NotFound`.
fn fixture() -> Value {
    let mut ctx = DefinitionContext::types_in("::models");
    let problem = ctx.register::<Problem>().unwrap();
    for register in [DefinitionContext::register::<Event>, DefinitionContext::register::<Directory>, DefinitionContext::register::<File>] {
        register(&mut ctx).unwrap();
    }
    let get = |index: usize, ctx: &mut DefinitionContext| Endpoint::get(format!("Get{index}"), format!("/r{index}")).describes("Get").returns_json::<Message>(ctx);
    let endpoints = vec![
        get(0, &mut ctx).errors(ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::empty())).build().unwrap(),
        get(1, &mut ctx).build().unwrap(),
    ];
    let mut api: RestApi = RestApi::builder(&ctx, "Fixture", "https://api.example.com").describes("Fixture").no_auth()
        .default_errors(ErrorSet::new().on(StatusMatch::class(4), ErrorShape::json(problem)).on(StatusMatch::class(5), ErrorShape::text()))
        .endpoints(endpoints).build().unwrap();
    let mut fallback = SuccessShape::new(ResponseShape::Binary);
    fallback.fallback = true;
    api.endpoints[0].response = vec![
        SuccessShape::new(ResponseShape::Json(named("Message"))),
        SuccessShape::new(ResponseShape::Stream { framing: StreamFraming::Sse(named("Event")), end: StreamEnd::ConnectionClose, keep: false }),
        fallback,
        SuccessShape::new(ResponseShape::Empty),
    ];
    let tagged = |name: &str, value: &str| {
        let mut shape = SuccessShape::new(ResponseShape::Json(named(name)));
        shape.json_tag = Some(JsonTag::Field { name: "type".into(), value: json!(value) });
        shape
    };
    api.endpoints[1].response = vec![tagged("Directory", "dir"), tagged("File", "file")];

    let mut document = serde_json::to_value(export(&api, &ExportOptions::new()).unwrap()).unwrap();
    let not_found = document.pointer(REFERENCED).unwrap().clone();
    document["components"]["responses"] = json!({"NotFound": not_found});
    *document.pointer_mut(REFERENCED).unwrap() = json!({"$ref": "#/components/responses/NotFound"});
    document
}

#[derive(Clone, Copy, PartialEq)]
enum Format { Json, Yaml }

fn text(document: &Value, format: Format) -> String {
    match format {
        Format::Json => serde_json::to_string(document).unwrap(),
        Format::Yaml => serde_yaml_ng::to_string(document).unwrap(),
    }
}

fn import(text: &str, format: Format) -> Result<schematic_define::openapi::OpenApiImportResult, String> {
    let source = match format { Format::Json => OpenApiSource::json(text), Format::Yaml => OpenApiSource::yaml(text) };
    OpenApiImport::new(source).build().map_err(|error| error.to_string())
}

fn classify(document: &Value, path: &str) -> Result<ResponseClassification, String> {
    let document: OpenAPI = serde_json::from_value(document.clone()).unwrap();
    let ReferenceOr::Item(item) = &document.paths.paths[path] else { unreachable!() };
    let id = if path == "/r0" { "Get0" } else { "Get1" };
    classify_responses("Fixture", id, &item.get.as_ref().unwrap().responses, &RefResolver::new(&document)).map_err(|error| error.to_string())
}

fn set(document: &mut Value, pointer: &str, value: Value) {
    *document.pointer_mut(pointer).unwrap_or_else(|| panic!("{pointer}")) = value;
}

fn insert(document: &mut Value, pointer: &str, key: &str, value: Value) {
    document.pointer_mut(pointer).unwrap().as_object_mut().unwrap().insert(key.into(), value);
}

/// A named edit of the fixture and the text its rejection must contain.
type Cell = (&'static str, Box<dyn Fn(&mut Value)>, &'static str);

/// The member edits every format runs, each rejected with `needle`.
fn rejected_cells() -> Vec<Cell> {
    let problem = || json!({"description": "x"});
    vec![
        ("responses null", Box::new(|d| set(d, RESPONSES, Value::Null)), "Expected responses object; null is not omission"),
        ("responses wrong type", Box::new(|d| set(d, RESPONSES, json!(["200"]))), "Expected responses object"),
        ("responses one invalid status", Box::new(move |d| insert(d, RESPONSES, "abc", problem())), "Response key `abc` is not `default`"),
        ("responses status out of range", Box::new(move |d| insert(d, RESPONSES, "600", problem())), "Response key `600`"),
        ("responses all invalid", Box::new(move |d| set(d, RESPONSES, json!({"abc": problem(), "6XX": problem()}))), "is not `default`, a status 100-599"),
        ("responses duplicate spelling", Box::new(|d| {
            let class = d.pointer("/paths/~1r1/get/responses/4XX").unwrap().clone();
            insert(d, "/paths/~1r1/get/responses", "4xx", class);
        }), "duplicates another spelling of the same status"),
        ("response member wrong type", Box::new(|d| set(d, &format!("{RESPONSES}/200"), json!("ok"))), "Expected response object"),
        ("content null", Box::new(|d| set(d, CONTENT, Value::Null)), "Expected content object; null is not omission"),
        ("content wrong type", Box::new(|d| set(d, CONTENT, json!([]))), "Expected content object"),
        ("content one invalid media", Box::new(|d| insert(d, CONTENT, "not a media type", json!({}))), "media type `not a media type` is not a concrete"),
        ("content all invalid", Box::new(|d| set(d, CONTENT, json!({"json": {}, "/x": {}}))), "is not a concrete `type/subtype`"),
        ("content media value wrong type", Box::new(|d| set(d, &format!("{CONTENT}/text~1event-stream"), json!(true))), "Expected media type object"),
        ("content duplicate spelling", Box::new(|d| insert(d, CONTENT, "Application/JSON; charset=utf-8", json!({}))), "duplicates another spelling of the same media type"),
        ("schema null", Box::new(|d| set(d, SCHEMA, Value::Null)), "Expected schema object; null is not omission"),
        ("schema wrong type", Box::new(|d| set(d, SCHEMA, json!("Message"))), "Expected schema object"),
        ("schema invalid reference", Box::new(|d| set(d, SCHEMA, json!({"$ref": 5}))), "`$ref` must be a nonempty string"),
        ("schema all invalid", Box::new(|d| set(d, SCHEMA, json!({"$ref": "", "oneOf": 7}))), "`$ref` must be a nonempty string"),
        ("response $ref null", Box::new(|d| set(d, REFERENCED, json!({"$ref": null}))), "`$ref` must be a nonempty string"),
        ("response $ref wrong type", Box::new(|d| set(d, REFERENCED, json!({"$ref": ["#/components/responses/NotFound"]}))), "`$ref` must be a nonempty string"),
        ("response $ref empty", Box::new(|d| set(d, REFERENCED, json!({"$ref": ""}))), "`$ref` must be a nonempty string"),
        ("oneOf null", Box::new(|d| set(d, ONE_OF, Value::Null)), "`oneOf` must be a nonempty array"),
        ("oneOf wrong type", Box::new(|d| set(d, ONE_OF, json!({}))), "`oneOf` must be a nonempty array"),
        ("oneOf one invalid member", Box::new(|d| d.pointer_mut(ONE_OF).unwrap().as_array_mut().unwrap().push(json!(5))), "`oneOf` member must be a schema or reference object"),
        ("oneOf all invalid", Box::new(|d| set(d, ONE_OF, json!([5, "File"]))), "`oneOf` member must be a schema or reference object"),
        ("oneOf invalid member reference", Box::new(|d| set(d, &format!("{ONE_OF}/0"), json!({"$ref": null}))), "`$ref` must be a nonempty string"),
        ("oneOf empty", Box::new(|d| set(d, ONE_OF, json!([]))), "`oneOf` must be a nonempty array"),
        ("discriminator null", Box::new(|d| set(d, DISCRIMINATOR, Value::Null)), "Expected discriminator object; null is not omission"),
        ("propertyName null", Box::new(|d| set(d, &format!("{DISCRIMINATOR}/propertyName"), Value::Null)), "`discriminator.propertyName` must be a nonempty string"),
        ("propertyName wrong type", Box::new(|d| set(d, &format!("{DISCRIMINATOR}/propertyName"), json!(5))), "`discriminator.propertyName` must be a nonempty string"),
        ("propertyName empty", Box::new(|d| set(d, &format!("{DISCRIMINATOR}/propertyName"), json!(""))), "`discriminator.propertyName` must be a nonempty string"),
        ("discriminator all invalid", Box::new(|d| set(d, DISCRIMINATOR, json!({"propertyName": [], "mapping": 5}))), "`discriminator.propertyName` must be a nonempty string"),
        ("discriminator mapping invalid", Box::new(|d| set(d, &format!("{DISCRIMINATOR}/mapping/dir"), json!(7))), "`discriminator.mapping` must map values"),
    ]
}

fn controls(document: &Value) {
    let first = classify(document, "/r0").unwrap();
    let kinds: Vec<&ResponseShape> = first.successes.iter().map(|shape| &shape.shape).collect();
    // `serde_json::Value` sorts media keys, so `*/*` comes first.
    assert!(matches!(kinds.as_slice(), [ResponseShape::Binary, ResponseShape::Json(_), ResponseShape::Stream { .. }, ResponseShape::Empty]), "{kinds:?}");
    assert!(first.successes[0].fallback);
    // 4XX is expanded because 404 supersedes part of it; the $ref'd 404 resolves to its empty entry.
    assert_eq!(first.errors.iter().find(|entry| entry.status == StatusMatch::exact(404)).unwrap().shape, ResponseShape::Empty);
    assert_eq!(first.errors.last().unwrap().status, StatusMatch::class(5));
    let second = classify(document, "/r1").unwrap();
    let tags: Vec<_> = second.successes.iter().map(|shape| shape.json_tag.clone()).collect();
    assert_eq!(tags, [Some(JsonTag::Field { name: "type".into(), value: json!("dir") }), Some(JsonTag::Field { name: "type".into(), value: json!("file") })]);
    assert!(second.errors.iter().any(|entry| entry.status == StatusMatch::class(4)));
}

fn run_format(format: Format) {
    let fixture = fixture();
    import(&text(&fixture, format), format).unwrap_or_else(|error| panic!("control import: {error}"));
    controls(&fixture);

    for (cell, edit, needle) in rejected_cells() {
        let mut document = fixture.clone();
        edit(&mut document);
        let error = import(&text(&document, format), format).err().unwrap_or_else(|| panic!("{cell}: imported"));
        assert!(error.contains(needle), "{cell}: {error}\n  expected {needle:?}");
        assert!(error.contains("API Fixture endpoint Get"), "{cell}: no API and endpoint context in {error}");
    }

    let trailing = format!("{}\n}}", text(&fixture, format));
    assert!(import(&trailing, format).is_err(), "trailing content");

    // Accepted edits: the public import succeeds and the classification
    // applies the matrix's absent and empty outcomes.
    let accepted = |edit: &dyn Fn(&mut Value)| {
        let mut document = fixture.clone();
        edit(&mut document);
        import(&text(&document, format), format).unwrap_or_else(|error| panic!("accepted edit rejected: {error}"));
        document
    };
    let no_content = accepted(&|d| set(d, &format!("{RESPONSES}/200"), json!({"description": "none"})));
    assert!(classify(&no_content, "/r0").unwrap().successes.iter().any(|shape| shape.shape == ResponseShape::Empty && shape.statuses.is_empty()));
    let empty_content = accepted(&|d| set(d, CONTENT, json!({})));
    assert_eq!(classify(&empty_content, "/r0").unwrap().successes, [SuccessShape::new(ResponseShape::Empty)]);
    let absent_schema = accepted(&|d| { d.pointer_mut(&format!("{CONTENT}/application~1json")).unwrap().as_object_mut().unwrap().remove("schema"); });
    let classified = classify(&absent_schema, "/r0").unwrap();
    assert_eq!(classified.successes[1].shape, ResponseShape::Json(TypeRef::Primitive(PrimitiveType::Json)));
    assert!(classified.diagnostics.iter().any(|d| d.message.contains("the JSON body has no named schema")));
    let empty_schema = accepted(&|d| set(d, SCHEMA, json!({})));
    assert_eq!(classify(&empty_schema, "/r0").unwrap().successes[1].shape, ResponseShape::Json(TypeRef::Primitive(PrimitiveType::Json)));
    let inline_response = accepted(&|d| set(d, REFERENCED, json!({"description": "gone"})));
    assert!(classify(&inline_response, "/r0").unwrap().errors.contains(&schematic_define::ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Empty)));
    let dangling = accepted(&|d| set(d, REFERENCED, json!({"$ref": "#/components/responses/Missing"})));
    assert!(classify(&dangling, "/r0").unwrap_err().contains("responses/404 does not resolve"));
    let no_discriminator = accepted(&|d| { d.pointer_mut("/paths/~1r1/get/responses/200/content/application~1json/schema").unwrap().as_object_mut().unwrap().remove("discriminator"); });
    assert!(classify(&no_discriminator, "/r1").unwrap().diagnostics[0].message.contains("separable neither"));
    let empty_responses = accepted(&|d| set(d, RESPONSES, json!({})));
    assert!(classify(&empty_responses, "/r0").unwrap_err().contains("no 2xx response yields a representable success shape"));
    let wildcard = accepted(&|d| insert(d, CONTENT, "image/*", json!({})));
    assert!(classify(&wildcard, "/r0").unwrap().diagnostics.iter().any(|d| d.message.contains("media range `image/*` cannot select a shape")));
}

#[test]
fn json_response_matrix() {
    run_format(Format::Json);
}

#[test]
fn yaml_response_matrix() {
    run_format(Format::Yaml);
}

/// Exact duplicates of a member's first key, injected into the JSON text
/// where `serde_json::Value` cannot hold them.
#[test]
fn json_exact_duplicate_keys_are_rejected_for_every_column() {
    let fixture = fixture();
    for pointer in [RESPONSES, CONTENT, SCHEMA, REFERENCED, "/paths/~1r1/get/responses/200/content/application~1json/schema", DISCRIMINATOR, &format!("{DISCRIMINATOR}/mapping")] {
        let member = serde_json::to_string(fixture.pointer(pointer).unwrap()).unwrap();
        let document = serde_json::to_string(&fixture).unwrap();
        assert_eq!(document.matches(&member).count(), 1, "{pointer} is not unique in the text");
        let first = serde_json::to_string(fixture.pointer(pointer).unwrap().as_object().unwrap().iter().next().map(|(key, value)| json!({key: value})).as_ref().unwrap()).unwrap();
        let duplicated = format!("{{{},{}", &first[1..first.len() - 1], &member[1..]);
        let error = import(&document.replacen(&member, &duplicated, 1), Format::Json).err().unwrap_or_else(|| panic!("{pointer}: imported"));
        assert!(error.contains("Duplicate key"), "{pointer}: {error}");
    }
}
