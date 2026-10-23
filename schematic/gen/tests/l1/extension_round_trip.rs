//! Spec `2026-10-18-schematic-fidelity-polish`, R10 and R54: the
//! `x-schematic` subset survives export, import, and export again.
//!
//! Every committed OpenAPI artifact is imported through the public importer
//! and compared twice, as parsed values and never as bytes:
//!
//! - its re-export with the committed document, and
//! - the imported `RestApi` with the current definition
//!   (`canonical(serde_json::to_value(api))`, contract D9).
//!
//! A difference is tolerated only when a named category below covers it,
//! and each artifact lists exactly the categories it shows: a new loss fails
//! as untolerated, and a fixed one fails until its category leaves the list.
//! A purpose-built fixture makes every v1 member load-bearing, and the R54
//! tests pin the two auth details the extension may carry.

use std::collections::BTreeSet;

use schematic_define::openapi::{
    DiagnosticSeverity, ExportFormat, ExportOptions, OpenApiDiagnostic, OpenApiImport, OpenApiImportResult, OpenApiSource,
    export, serialize,
};
use schematic_define::{
    ApiKeyEnv, AuthMethod, AuthPolicy, AuthStrategy, Backoff, CrossOrigin, DefinitionContext, Endpoint, EnvAuthStrategy,
    EnvList, EnvMapping, ErrorEntry, ErrorSet, FormField, ParamDef, ParamStyle, QueryParamType, RateLimit, ResetUnit, ResponseShape,
    RestApi, RetryGuidance, SchemaComponent, StatusMatch, StatusSelector, StreamEnd, StreamFraming, SuccessShape, TypeRef,
};
use serde_json::{Map, Value, json};

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// Object keys sorted at every depth; arrays keep their order.
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted: std::collections::BTreeMap<_, _> = object.into_iter().map(|(key, value)| (key, canonical(value))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        value => value,
    }
}

fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// One leaf where two values disagree; a side is `None` when the member is
/// absent there.
#[derive(Debug)]
struct Difference {
    pointer: String,
    expected: Option<Value>,
    actual: Option<Value>,
}

fn differences(expected: &Value, actual: &Value, pointer: &str, out: &mut Vec<Difference>) {
    let push = |out: &mut Vec<Difference>, pointer: String, expected: Option<&Value>, actual: Option<&Value>| {
        out.push(Difference { pointer, expected: expected.cloned(), actual: actual.cloned() });
    };
    match (expected, actual) {
        (Value::Object(left), Value::Object(right)) => {
            let keys: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
            for key in keys {
                let child = format!("{pointer}/{}", escape(key));
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => differences(left, right, &child, out),
                    (left, right) => push(out, child, left, right),
                }
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            for index in 0..left.len().max(right.len()) {
                let child = format!("{pointer}/{index}");
                match (left.get(index), right.get(index)) {
                    (Some(left), Some(right)) => differences(left, right, &child, out),
                    (left, right) => push(out, child, left, right),
                }
            }
        }
        (left, right) if left != right => push(out, pointer.to_string(), Some(left), Some(right)),
        _ => {}
    }
}

/// Matches an escaped JSON pointer against `pattern`: `*` inside a token
/// matches any run of characters, and a `**` token matches every remaining
/// token.
fn pointer_matches(pattern: &str, pointer: &str) -> bool {
    fn token_matches(pattern: &str, token: &str) -> bool {
        match pattern.split_once('*') {
            None => pattern == token,
            Some((head, rest)) => {
                let Some(tail) = token.strip_prefix(head) else { return false };
                (0..=tail.len()).filter(|cut| tail.is_char_boundary(*cut)).any(|cut| token_matches(rest, &tail[cut..]))
            }
        }
    }
    let pattern: Vec<&str> = pattern.split('/').collect();
    let pointer: Vec<&str> = pointer.split('/').collect();
    let mut index = 0;
    for (position, token) in pattern.iter().enumerate() {
        if *token == "**" {
            return position + 1 == pattern.len() && index < pointer.len();
        }
        if index >= pointer.len() || !token_matches(token, pointer[index]) {
            return false;
        }
        index += 1;
    }
    index == pointer.len()
}

/// What a tolerance may compare a difference against.
struct Context<'a> {
    module: &'a str,
    version: &'a str,
    /// The first member's name: a grouped document keeps its metadata (D8).
    member: &'a str,
    imported: &'a str,
    /// The effective module path: the document's `x-schematic.module_path`
    /// (no import option is set).
    module_path: Option<&'a str>,
}

/// One tolerated difference: the category it belongs to, the pointers it
/// covers, and what both sides must hold there.
struct Tolerance {
    category: &'static str,
    pattern: &'static str,
    accepts: fn(&Context, &Difference) -> bool,
}

fn string(value: &Option<Value>) -> Option<&str> {
    value.as_ref().and_then(Value::as_str)
}

/// The PascalCase name the importer derives from a grouped title (the module
/// name).
fn pascal(module: &str) -> String {
    module.split('_').map(|word| {
        let mut chars = word.chars();
        chars.next().map(|first| first.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
    }).collect()
}

/// Differences between the committed document and the re-export of its
/// import.
///
/// - `grouped-title`: the grouped writer retitles `info` to the module name
///   (D8); the importer reads the title as the API name, which it spells as
///   a Rust identifier, so the re-export's title is that identifier.
/// - `grouped-server-description`: the server description is derived from
///   the API name (`{name} API Server`), so it follows the name above.
/// - `grouped-from-sibling`: `x-schematic-grouped-from` is written by the
///   generator's grouped writer, not by `export`; it is outside the subset
///   and read only as INFO.
const DOCUMENT_TOLERANCES: &[Tolerance] = &[
    Tolerance { category: "grouped-title", pattern: "/info/title", accepts: |context, difference| {
        string(&difference.expected) == Some(context.module) && string(&difference.actual) == Some(context.imported)
    } },
    Tolerance { category: "grouped-server-description", pattern: "/servers/0/description", accepts: |context, difference| {
        string(&difference.expected) == Some(&format!("{} API Server", context.member))
            && string(&difference.actual) == Some(&format!("{} API Server", context.imported))
    } },
    Tolerance { category: "grouped-from-sibling", pattern: "/x-schematic-grouped-from", accepts: |_, difference| {
        difference.actual.is_none()
    } },
];

/// Differences between the current definition and its re-import.
///
/// - `version-default`: shipped definitions leave `version` unset and the
///   exporter writes the version option; the importer reads `info.version`.
/// - `grouped-name`: as `grouped-title` above, from the definition's side.
/// - `schema-rust-path`: no extension carries a component's Rust path, so a
///   definition's own path (`::schematic_definitions::…`, or none) cannot
///   survive. The importer rebuilds it as `{module_path}::{Name}` from the
///   effective module path, or the bare name when there is none.
/// - `schema-json-normalized`: a definition stores each component as JSON
///   Schema and the importer stores the OpenAPI 3.0 form it read. The
///   document comparison holds `components.schemas` equal, so the schemas
///   mean the same.
/// - `pagination`: pagination is outside v1 (D7).
/// - `page-size-bounds`: export writes an adopted page-size parameter's
///   strategy bounds into its schema, and the re-import, which has no
///   pagination, keeps them as the parameter's own `minimum`/`maximum`. A
///   page-size parameter that declares exactly its strategy's bounds is the
///   same definition as one that declares none.
/// - `grouped-operation-ids`: the grouped writer prefixes a later member's
///   clashing `operationId`; one document imports as one API (D8).
/// - `grouped-auth-union`: the grouped writer merges two members' operation
///   on one method and path into one accepting either member's requirements,
///   so the imported API accepts both members' methods (D8).
const DEFINITION_TOLERANCES: &[Tolerance] = &[
    Tolerance { category: "version-default", pattern: "/version", accepts: |context, difference| {
        difference.expected == Some(Value::Null) && string(&difference.actual) == Some(context.version)
    } },
    Tolerance { category: "grouped-name", pattern: "/name", accepts: |context, difference| {
        string(&difference.expected) == Some(context.member) && string(&difference.actual) == Some(&pascal(context.module))
    } },
    Tolerance { category: "schema-rust-path", pattern: "/schemas/*/rust_path", accepts: rebuilt_rust_path },
    Tolerance { category: "schema-json-normalized", pattern: "/schemas/*/schema/**", accepts: |_, _| true },
    Tolerance { category: "pagination", pattern: "/default_pagination", accepts: |_, difference| {
        difference.actual == Some(Value::Null)
    } },
    Tolerance { category: "pagination", pattern: "/endpoints/*/pagination", accepts: |_, difference| {
        difference.expected.is_some() && difference.actual.is_none()
    } },
    Tolerance { category: "page-size-bounds", pattern: "/endpoints/*/params/*/*/constraints", accepts: |_, difference| {
        difference.expected.is_none()
            && difference.actual.as_ref().and_then(Value::as_object).is_some_and(|rules| {
                rules.contains_key("minimum") && rules.keys().all(|keyword| keyword == "minimum" || keyword == "maximum")
            })
    } },
    Tolerance { category: "grouped-operation-ids", pattern: "/endpoints/*/id", accepts: |_, difference| {
        matches!((string(&difference.expected), string(&difference.actual)),
            (Some(expected), Some(actual)) if actual.len() > expected.len() && actual.ends_with(expected))
    } },
    Tolerance { category: "grouped-auth-union", pattern: "/auth_policy", accepts: |_, difference| {
        difference.expected == Some(Value::Null)
            && difference.actual.as_ref().and_then(|policy| policy["explicit"].as_array()).is_some_and(|methods| methods.len() > 1)
    } },
];

/// Whether the imported `rust_path` at `/schemas/{Name}/rust_path` is the
/// one the importer must rebuild from the effective module path.
fn rebuilt_rust_path(context: &Context, difference: &Difference) -> bool {
    let name = difference.pointer.split('/').nth(2).unwrap_or_default().replace("~1", "/").replace("~0", "~");
    let rebuilt = match context.module_path {
        Some(module_path) => format!("{module_path}::{name}"),
        None => name,
    };
    string(&difference.actual) == Some(rebuilt.as_str())
}

/// The categories `found` falls into; panics listing every difference no
/// tolerance covers.
fn categorize(label: &str, found: &[Difference], tolerances: &[Tolerance], context: &Context) -> BTreeSet<&'static str> {
    let mut categories = BTreeSet::new();
    let mut untolerated = Vec::new();
    for difference in found {
        match tolerances.iter().find(|tolerance| pointer_matches(tolerance.pattern, &difference.pointer) && (tolerance.accepts)(context, difference)) {
            Some(tolerance) => {
                categories.insert(tolerance.category);
            }
            None => untolerated.push(format!("{} expected {:?} actual {:?}", difference.pointer, difference.expected, difference.actual)),
        }
    }
    assert!(untolerated.is_empty(), "{label}: {} untolerated differences:\n{}", untolerated.len(), untolerated.join("\n"));
    categories
}

/// Endpoints keyed by `METHOD path` and schemas keyed by name, so grouped
/// members compare by operation rather than by position (D8). A later
/// member's endpoint on a method and path already taken is the merged
/// operation, which the first member's endpoint describes.
fn definition_view(metadata: &RestApi, members: &[RestApi]) -> Value {
    let mut view = canonical(serde_json::to_value(metadata).unwrap());
    let object = view.as_object_mut().unwrap();
    let mut endpoints = Map::new();
    let mut schemas = Map::new();
    for member in members {
        for endpoint in &member.endpoints {
            let key = format!("{} {}", endpoint.method.to_string().to_uppercase(), endpoint.path);
            endpoints.entry(key).or_insert_with(|| canonical(serde_json::to_value(endpoint).unwrap()));
        }
        for schema in &member.schemas {
            schemas.entry(schema.name.clone()).or_insert_with(|| canonical(serde_json::to_value(schema).unwrap()));
        }
    }
    object.insert("endpoints".into(), Value::Object(endpoints));
    object.insert("schemas".into(), Value::Object(schemas));
    view
}

/// Renames every string equal to a component name in `renames` (bare, or as
/// a component `$ref`) and every object key equal to one.
fn rename_components(value: &mut Value, renames: &[(String, String)]) {
    let rename = |text: &str| {
        renames.iter().find_map(|(from, to)| {
            if text == from {
                Some(to.clone())
            } else {
                (text.strip_prefix("#/components/schemas/") == Some(from)).then(|| format!("#/components/schemas/{to}"))
            }
        })
    };
    match value {
        Value::String(text) => {
            if let Some(renamed) = rename(text) {
                *text = renamed;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| rename_components(item, renames)),
        Value::Object(object) => {
            let entries: Vec<(String, Value)> = std::mem::take(object).into_iter().map(|(key, mut member)| {
                rename_components(&mut member, renames);
                (rename(&key).unwrap_or(key), member)
            }).collect();
            object.extend(entries);
        }
        _ => {}
    }
}

/// Every WARN or ERROR diagnostic, each as `location: message`.
fn failing(diagnostics: &[OpenApiDiagnostic]) -> Vec<String> {
    diagnostics.iter().filter(|diagnostic| diagnostic.severity >= DiagnosticSeverity::Warn)
        .map(|diagnostic| format!("{} {}: {}", diagnostic.severity, diagnostic.location, diagnostic.message)).collect()
}

// ---------------------------------------------------------------------------
// Committed artifacts
// ---------------------------------------------------------------------------

/// What one committed artifact's round trip shows.
struct Expected {
    /// The repair applied before comparison, and document tolerances.
    document: &'static [&'static str],
    definition: &'static [&'static str],
}

const TITLE: &[&str] = &["grouped-from-sibling", "grouped-title"];
const RENAMED: &[&str] = &["grouped-from-sibling", "grouped-server-description", "grouped-title"];

/// The expected round trip of `module`.
///
/// One repair runs before the comparison, a category of its own:
///
/// - `generic-component-names`: a definition may name a component
///   `PaginatedResponse<Tag>`, which the importer spells as the Rust
///   identifier `PaginatedResponse_Tag_`. The committed document and the
///   definition are compared with that spelling.
fn expected(module: &str) -> Expected {
    let plain = &["schema-json-normalized", "schema-rust-path", "version-default"];
    let paginated = &["pagination", "schema-json-normalized", "schema-rust-path", "version-default"];
    let renamed = &["grouped-name", "pagination", "schema-json-normalized", "schema-rust-path", "version-default"];
    let sized = &["grouped-name", "page-size-bounds", "pagination", "schema-json-normalized", "schema-rust-path", "version-default"];
    match module {
        "anthropic" => Expected { document: TITLE, definition: paginated },
        "artificial_analysis" => Expected { document: RENAMED, definition: &["grouped-name", "schema-rust-path", "version-default"] },
        "bitbucket" => Expected {
            document: &["generic-component-names", "grouped-from-sibling", "grouped-title"],
            definition: &["generic-component-names", "page-size-bounds", "pagination", "schema-json-normalized", "schema-rust-path", "version-default"],
        },
        "elevenlabs" | "huggingface" => Expected { document: RENAMED, definition: renamed },
        "github" | "gitlab" => Expected { document: RENAMED, definition: sized },
        "emqx" => Expected {
            document: RENAMED,
            definition: &["grouped-auth-union", "grouped-name", "pagination", "schema-json-normalized", "schema-rust-path", "version-default"],
        },
        "eversolo" | "samsung_smart_tv" | "unfolded_circle_core_rest" => Expected { document: TITLE, definition: plain },
        "gitea" => Expected { document: TITLE, definition: &["page-size-bounds", "pagination", "schema-json-normalized", "schema-rust-path", "version-default"] },
        "lmstudio" => Expected { document: RENAMED, definition: &["grouped-name", "schema-json-normalized", "schema-rust-path", "version-default"] },
        "ollama" => Expected {
            document: RENAMED,
            definition: &["grouped-name", "grouped-operation-ids", "schema-json-normalized", "schema-rust-path", "version-default"],
        },
        "openai" => Expected { document: RENAMED, definition: sized },
        other => panic!("no round-trip expectation for module `{other}`"),
    }
}

fn round_trip_artifact(module: &str, text: &str) {
    let grouped = schematic_definitions::apis_by_module();
    let members = grouped.get(module).unwrap_or_else(|| panic!("`{module}.json` names no definition module"));
    let version = members[0].version.as_deref().unwrap_or("0.1.0");
    let options = ExportOptions::new().with_version(version);
    let expectation = expected(module);
    let mut document_categories = BTreeSet::new();
    let mut definition_categories = BTreeSet::new();

    let mut committed: Value = serde_json::from_str(text).unwrap();
    let problems = document_r54_problems(&committed, &format!("{module}.json"));
    assert!(problems.is_empty(), "R54 violations:\n{}", problems.join("\n"));
    let imported = OpenApiImport::new(OpenApiSource::json(text)).build()
        .unwrap_or_else(|error| panic!("{module}.json does not import: {error}"));
    let warnings = failing(&imported.diagnostics);
    assert!(warnings.is_empty(), "{module}: unexpected import diagnostics:\n{}", warnings.join("\n"));
    if let Err(error) = OpenApiImport::new(OpenApiSource::json(text)).strict().build() {
        panic!("{module}.json does not import strictly: {error}");
    }

    let renames: Vec<(String, String)> = members.iter().flat_map(|member| &member.schemas)
        .filter(|schema| schema.name.contains('<'))
        .map(|schema| (schema.name.clone(), schema.name.replace(['<', '>'], "_")))
        .collect::<BTreeSet<_>>().into_iter().collect();
    if !renames.is_empty() {
        document_categories.insert("generic-component-names");
        definition_categories.insert("generic-component-names");
        rename_components(&mut committed, &renames);
    }

    let module_path = committed["x-schematic"]["module_path"].as_str().map(str::to_string);
    let context = Context { module, version, member: &members[0].name, imported: &imported.api.name, module_path: module_path.as_deref() };
    let reexported = serde_json::to_value(export(&imported.api, &options).expect("the imported API exports")).unwrap();
    let mut found = Vec::new();
    differences(&committed, &reexported, "", &mut found);
    document_categories.extend(categorize(&format!("{module}.json re-export"), &found, DOCUMENT_TOLERANCES, &context));
    assert_eq!(document_categories.into_iter().collect::<Vec<_>>(), expectation.document, "{module}: document categories");

    let mut definition = definition_view(&members[0], members);
    rename_components(&mut definition, &renames);
    let mut found = Vec::new();
    differences(&definition, &definition_view(&imported.api, std::slice::from_ref(&imported.api)), "", &mut found);
    definition_categories.extend(categorize(&format!("{module} definition"), &found, DEFINITION_TOLERANCES, &context));
    assert_eq!(definition_categories.into_iter().collect::<Vec<_>>(), expectation.definition, "{module}: definition categories");
}

/// One `#[test]` per committed artifact. Each reads its file with an
/// `include_str!` literal so CI's test-input scan sees the dependency.
macro_rules! committed_artifacts {
    ($($module:ident => $path:literal),* $(,)?) => {
        const ARTIFACTS: &[&str] = &[$(stringify!($module)),*];

        mod artifact {
            $(
                #[test]
                fn $module() {
                    super::round_trip_artifact(stringify!($module), include_str!($path));
                }
            )*
        }
    };
}

committed_artifacts! {
    anthropic => "../../../openapi/anthropic.json",
    artificial_analysis => "../../../openapi/artificial_analysis.json",
    bitbucket => "../../../openapi/bitbucket.json",
    elevenlabs => "../../../openapi/elevenlabs.json",
    emqx => "../../../openapi/emqx.json",
    eversolo => "../../../openapi/eversolo.json",
    gitea => "../../../openapi/gitea.json",
    github => "../../../openapi/github.json",
    gitlab => "../../../openapi/gitlab.json",
    huggingface => "../../../openapi/huggingface.json",
    lmstudio => "../../../openapi/lmstudio.json",
    ollama => "../../../openapi/ollama.json",
    openai => "../../../openapi/openai.json",
    samsung_smart_tv => "../../../openapi/samsung_smart_tv.json",
    unfolded_circle_core_rest => "../../../openapi/unfolded_circle_core_rest.json",
}

#[test]
fn every_committed_artifact_and_module_has_a_round_trip_test() {
    let listed: BTreeSet<String> = ARTIFACTS.iter().map(|module| module.to_string()).collect();
    let openapi = biscuit_test_harness::manifest_dir!().parent().expect("schematic/gen has a parent").join("openapi");
    let on_disk: BTreeSet<String> = std::fs::read_dir(openapi).unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter_map(|name| name.strip_suffix(".json").map(str::to_string))
        .collect();
    let modules: BTreeSet<String> = schematic_definitions::apis_by_module().keys().cloned().collect();
    assert_eq!(listed, on_disk, "the macro's artifacts must equal `schematic/openapi/*.json`");
    assert_eq!(listed, modules, "the macro's artifacts must equal the definition modules");
}

// ---------------------------------------------------------------------------
// Purpose-built fixture
// ---------------------------------------------------------------------------

fn named(name: &str) -> TypeRef {
    TypeRef::Named(name.into())
}

fn sse(end: StreamEnd, keep: bool) -> ResponseShape {
    ResponseShape::Stream { framing: StreamFraming::Sse(named("Event")), end, keep }
}

/// A header parameter. OpenAPI fixes a header's style as `simple`, so a
/// definition declaring another one could not round-trip.
fn header(name: &str) -> ParamDef {
    ParamDef { style: ParamStyle::Simple, ..ParamDef::new(name, QueryParamType::String) }
}

/// An API on which every member of the v1 subset changes the definition.
fn full_fixture() -> RestApi {
    let rate_limit = |prefix: &str, unit| RateLimit::new(format!("{prefix}-limit"), format!("{prefix}-remaining"), format!("{prefix}-reset"), unit).unwrap();
    let admin_key = AuthMethod::ApiKey { header: "X-Admin-Key".into(), value_prefix: None };
    // The importer rebuilds a policy's fallback as its first method's
    // environment form, so the fixture spells it that way.
    let admin_policy = AuthPolicy {
        explicit: vec![admin_key],
        env_fallback: Some(EnvAuthStrategy::ApiKey { header: "X-Admin-Key".into(), value_prefix: None }),
    };
    let admin_mapping = EnvMapping {
        api_key: Some(ApiKeyEnv { names: EnvList::single("FIXTURE_ADMIN_ONLY_KEY"), header: "X-Admin-Key".into() }),
        ..EnvMapping::default()
    };

    let mut list = Endpoint::get("ListItems", "/items").describes("List items")
        .query(ParamDef::new("cursor", QueryParamType::String).describes("Opaque cursor"))
        .query(ParamDef::new("api-token", QueryParamType::String).secret().rust_name("token"))
        .query(ParamDef::new("legacy", QueryParamType::Boolean).deprecated())
        .header_param(header("X-Session").secret())
        .header_param(header("X-Trace-Id").rust_name("trace"))
        .header("X-Feature", "beta")
        .rate_limit(rate_limit("x-items", ResetUnit::Seconds))
        .retry_guidance(RetryGuidance::new([], false, 1, Backoff::Fixed { delay_ms: 500 }).unwrap())
        .cross_origin(CrossOrigin::deny())
        .returns_empty().build().unwrap();
    list.response = vec![SuccessShape::json_type("ItemList")];
    list.errors = vec![ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Empty)];

    let mut get = Endpoint::get("GetItem", "/items/{item_id}").describes("Get an item")
        .path_param(ParamDef::path("item_id", QueryParamType::Integer).describes("Item id").deprecated())
        .deprecated()
        .errors(ErrorSet::none())
        .returns_empty().build().unwrap();
    get.response = vec![SuccessShape::json_type("Item")];

    let file = Endpoint::get("GetFile", "/files/{+path}").describes("Read a file").returns_binary().build().unwrap();
    let health = Endpoint::get("Health", "/health").describes("Health check").public().returns_text().build().unwrap();
    let mut stats = Endpoint::get("AdminStats", "/admin/stats").describes("Admin statistics")
        .auth_with_env_mapping(admin_policy, admin_mapping)
        .returns_empty().build().unwrap();
    stats.response = vec![SuccessShape::json_type("Item")];

    let mut upload = Endpoint::post("Upload", "/uploads").describes("Upload files")
        .request_form(vec![
            FormField::text("caption").optional().with_description("Caption"),
            FormField::file_accept("image", vec!["image/png".into()]),
            FormField::files_with_constraints("attachments", vec!["application/pdf".into()], Some(1), Some(3)),
            FormField::json("metadata", named("Meta")),
        ])
        .returns_empty().build().unwrap();
    upload.response = vec![SuccessShape::json_type("Item")];

    let mut chat = Endpoint::post("Chat", "/chat").describes("Chat").returns_empty().build().unwrap();
    chat.request = Some(schematic_define::ApiRequest::json_type("ChatInput"));
    let mut reply = SuccessShape::json_type("Reply");
    reply.note = Some("returned when `stream` is false".into());
    let mut events = SuccessShape::new(sse(StreamEnd::EventNamed("message_stop".into()), false));
    events.note = Some("returned when `stream` is true".into());
    chat.response = vec![reply, events];

    let mut complete = Endpoint::post("Complete", "/complete").describes("Complete").returns_empty().build().unwrap();
    complete.response = vec![SuccessShape::new(sse(StreamEnd::DataLineEquals("[DONE]".into()), true))];

    let mut speech = Endpoint::get("Speech", "/speech").describes("Speech audio").returns_empty().build().unwrap();
    let mut audio = SuccessShape::new(ResponseShape::Stream { framing: StreamFraming::Bytes, end: StreamEnd::ConnectionClose, keep: false });
    audio.content_type = Some("audio/mpeg".into());
    speech.response = vec![audio];

    let context = DefinitionContext::types_in("::fixture");
    let mut api = RestApi::builder(&context, "Fixture", "https://fixture.example")
        .describes("Round-trip fixture")
        .bearer_env(["FIXTURE_TOKEN"])
        .env_mapping(EnvMapping {
            bearer_token: Some(EnvList::single("FIXTURE_TOKEN")),
            api_key: Some(ApiKeyEnv { names: EnvList::single("FIXTURE_ADMIN_KEY"), header: "X-Admin-Key".into() }),
            ..EnvMapping::default()
        })
        .header("X-Client", "fixture")
        .module_path("fixture")
        .request_suffix("Body")
        .version("2.0.0")
        .cross_origin(CrossOrigin::allow_origins(["https://cdn.fixture.example"]))
        .request_id_header("X-Request-Id")
        .secret_header("X-Session")
        .rate_limit(rate_limit("x-ratelimit", ResetUnit::EpochSeconds).with_retry_after_header("retry-after").unwrap())
        .retry_guidance(RetryGuidance::new([StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 599 }], true, 3,
            Backoff::Exponential { base_ms: 250, cap_ms: 8000 }).unwrap())
        .build().expect("valid fixture API");
    api.errors = vec![
        ErrorEntry::new(StatusMatch::class(4), ResponseShape::Json(named("Problem"))),
        ErrorEntry::new(StatusMatch::class(5), ResponseShape::Text),
    ];
    // Endpoints join after build: their components are bare names added
    // below, not registered Rust types.
    api.endpoints = vec![list, get, file, health, stats, upload, chat, complete, speech];
    for name in ["ChatInput", "Event", "Item", "ItemList", "Meta", "Problem", "Reply"] {
        api.schemas.push(SchemaComponent {
            name: name.into(),
            rust_path: Some(name.into()),
            schema: json!({"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}),
        });
    }
    api
}

fn import(source: OpenApiSource) -> OpenApiImportResult {
    OpenApiImport::new(source).build().expect("the export imports")
}

fn exported_value(api: &RestApi, options: &ExportOptions) -> Value {
    serde_json::to_value(export(api, options).expect("export succeeds")).unwrap()
}

/// The fixture is chosen so each of these holds before the round trip is
/// measured; a member the fixture leaves at its default would prove nothing.
#[test]
fn the_fixture_writes_every_v1_member() {
    let document = exported_value(&full_fixture(), &ExportOptions::new().with_version("2.0.0"));
    let root = document["x-schematic"].as_object().unwrap();
    for member in ["module_path", "request_suffix", "env_mapping", "env_auth", "headers", "rate_limit", "retry_guidance",
        "errors", "cross_origin", "request_id_header", "secret_names"] {
        assert!(root.contains_key(member), "document `x-schematic` lacks `{member}`: {root:?}");
    }
    let operation = |path: &str, method: &str| document["paths"][path][method]["x-schematic"].clone();
    let list = operation("/items", "get");
    for member in ["headers", "rate_limit", "retry_guidance", "errors", "cross_origin", "params"] {
        assert!(list.get(member).is_some(), "ListItems lacks `{member}`: {list}");
    }
    assert_eq!(operation("/items/{item_id}", "get")["no_declared_errors"], true);
    assert_eq!(operation("/files/{path}", "get")["path_template"], "/files/{+path}");
    assert!(operation("/admin/stats", "get")["env_mapping"].is_object());
    assert_eq!(document["paths"]["/health"]["get"]["security"], json!([]));
    assert!(operation("/uploads", "post")["request"]["FormData"].is_object());
    assert_eq!(operation("/chat", "post")["success"][1]["shape"]["Stream"]["end"], json!({"EventNamed": "message_stop"}));
    assert_eq!(operation("/complete", "post")["success"][0]["shape"]["Stream"]["keep"], true);
    assert_eq!(operation("/speech", "get")["success"][0]["shape"]["Stream"]["framing"], "Bytes");
    assert_eq!(document["paths"]["/items/{item_id}"]["get"]["deprecated"], true);
}

/// Export to JSON and YAML text, import each, and require the definition
/// and the re-exported document to equal the originals exactly, except that
/// each component's `rust_path` is rebuilt from the module path (see
/// `schema-rust-path`).
#[test]
fn the_fixture_round_trips_through_json_and_yaml() {
    let api = full_fixture();
    let context = Context { module: "fixture", version: "2.0.0", member: &api.name, imported: &api.name, module_path: api.module_path.as_deref() };
    let options = ExportOptions::new().with_version("2.0.0");
    let document = export(&api, &options).expect("export succeeds");
    let first = serde_json::to_value(&document).unwrap();
    for format in [ExportFormat::Json, ExportFormat::Yaml] {
        let text = serialize(&document, format).unwrap();
        let source = match format {
            ExportFormat::Json => OpenApiSource::json(text),
            ExportFormat::Yaml => OpenApiSource::yaml(text),
        };
        let imported = import(source);
        assert_eq!(failing(&imported.diagnostics), Vec::<String>::new(), "{format:?}");

        let mut found = Vec::new();
        differences(&definition_view(&api, std::slice::from_ref(&api)), &definition_view(&imported.api, std::slice::from_ref(&imported.api)), "", &mut found);
        found.retain(|difference| !(pointer_matches("/schemas/*/rust_path", &difference.pointer) && rebuilt_rust_path(&context, difference)));
        assert!(found.is_empty(), "{format:?} definition differs:\n{found:#?}");

        let mut found = Vec::new();
        differences(&first, &exported_value(&imported.api, &options), "", &mut found);
        assert!(found.is_empty(), "{format:?} re-export differs:\n{found:#?}");
    }
}

// ---------------------------------------------------------------------------
// R54: the two auth details
// ---------------------------------------------------------------------------

fn auth_fixture(auth: AuthStrategy, env: &str) -> RestApi {
    let context = DefinitionContext::types_in("::fixture");
    RestApi::builder(&context, "AuthFixture", "https://auth.fixture.example")
        .describes("Auth fixture")
        .authentication(auth, [env])
        .endpoints([Endpoint::get("Ping", "/ping").describes("Ping").returns_text().build().unwrap()])
        .build().expect("valid auth fixture")
}

fn gitea_like() -> RestApi {
    auth_fixture(AuthStrategy::ApiKey { header: "Authorization".into(), value_prefix: Some("token ".into()) }, "FIXTURE_TOKEN")
}

fn bearer_in_custom_header() -> RestApi {
    auth_fixture(AuthStrategy::BearerToken { header: Some("X-Auth-Token".into()) }, "FIXTURE_TOKEN")
}

#[test]
fn an_api_key_value_prefix_and_a_bearer_header_override_survive_the_round_trip() {
    for api in [gitea_like(), bearer_in_custom_header()] {
        let first = exported_value(&api, &ExportOptions::new());
        let imported = import(OpenApiSource::json(first.to_string()));
        assert_eq!(failing(&imported.diagnostics), Vec::<String>::new());
        assert_eq!(imported.api.auth, api.auth);
        assert_eq!(imported.api.auth_policy, api.auth_policy);
        assert_eq!(imported.api.env_auth, api.env_auth);
        assert_eq!(exported_value(&imported.api, &ExportOptions::new()), first);
    }
}

/// An auth detail naming an `http bearer` scheme cannot apply to it: WARN at
/// the detail, and the standard scheme decides the API's auth.
#[test]
fn an_auth_detail_on_an_http_bearer_scheme_warns_and_the_standard_scheme_wins() {
    let mut document = exported_value(&gitea_like(), &ExportOptions::new());
    let scheme = document["x-schematic"]["auth"].as_object().unwrap().keys().next().unwrap().clone();
    document["components"]["securitySchemes"][&scheme] = json!({"type": "http", "scheme": "bearer"});

    let imported = import(OpenApiSource::json(document.to_string()));
    let warnings: Vec<&OpenApiDiagnostic> = imported.diagnostics.iter().filter(|diagnostic| diagnostic.severity >= DiagnosticSeverity::Warn).collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].severity, DiagnosticSeverity::Warn);
    assert_eq!(warnings[0].location, format!("#/x-schematic/auth/{scheme}"));
    assert_eq!(imported.api.auth, AuthStrategy::BearerToken { header: None });
}

/// Keys no `x-schematic` object may hold: auth fields standard OpenAPI
/// already carries (R54).
const FORBIDDEN_KEYS: [&str; 3] = ["strategy", "scopes", "oauth_scopes"];

/// A string that looks like a credential rather than a name: a well-known
/// token prefix, or a long opaque run of letters and digits.
fn credential_like(text: &str) -> bool {
    const PREFIXES: [&str; 9] = ["sk-", "sk_", "ghp_", "gho_", "github_pat_", "glpat-", "xoxb-", "Bearer ", "Basic "];
    let opaque = text.len() >= 32
        && text.chars().all(|c| c.is_ascii_alphanumeric() || "-_+=".contains(c))
        && text.chars().any(|c| c.is_ascii_digit())
        && text.chars().any(|c| c.is_ascii_lowercase());
    opaque || PREFIXES.iter().any(|prefix| text.starts_with(prefix))
}

/// Every problem R54 forbids in `value`, an `x-schematic` object at
/// `pointer`.
fn r54_problems(value: &Value, pointer: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, member) in object {
                let child = format!("{pointer}/{}", escape(key));
                if FORBIDDEN_KEYS.contains(&key.as_str()) {
                    out.push(format!("{child}: forbidden key"));
                }
                r54_problems(member, &child, out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                r54_problems(item, &format!("{pointer}/{index}"), out);
            }
        }
        Value::String(text) if credential_like(text) => out.push(format!("{pointer}: credential-like value")),
        _ => {}
    }
}

/// The R54 problems of one document's extensions, document and operation
/// level; an `auth` entry holds only `bearer` or `value_prefix`.
fn document_r54_problems(document: &Value, label: &str) -> Vec<String> {
    let mut extensions = vec![("#/x-schematic".to_string(), &document["x-schematic"])];
    for (path, item) in document["paths"].as_object().into_iter().flatten() {
        for (method, operation) in item.as_object().into_iter().flatten() {
            if let Some(extension) = operation.get("x-schematic") {
                extensions.push((format!("#/paths/{}/{method}/x-schematic", escape(path)), extension));
            }
        }
    }
    let mut problems = Vec::new();
    for (pointer, extension) in extensions {
        r54_problems(extension, &pointer, &mut problems);
        for (scheme, entry) in extension["auth"].as_object().into_iter().flatten() {
            for key in entry.as_object().into_iter().flatten().map(|(key, _)| key) {
                if key != "bearer" && key != "value_prefix" {
                    problems.push(format!("{pointer}/auth/{}: auth entry key `{key}`", escape(scheme)));
                }
            }
        }
    }
    problems.into_iter().map(|problem| format!("{label} {problem}")).collect()
}

/// The committed artifacts are checked by their own round-trip tests.
#[test]
fn no_fixture_extension_carries_an_auth_field_beyond_the_two_details() {
    for (label, api, details) in [("fixture", full_fixture(), 0), ("gitea-like", gitea_like(), 1), ("bearer-header", bearer_in_custom_header(), 1)] {
        let document = exported_value(&api, &ExportOptions::new());
        assert_eq!(document["x-schematic"]["auth"].as_object().map_or(0, Map::len), details, "{label}");
        let problems = document_r54_problems(&document, label);
        assert!(problems.is_empty(), "R54 violations:\n{}", problems.join("\n"));
    }
}
