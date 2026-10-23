//! Definition bundles through the public construction, JSON, and export
//! paths. This crate cannot reach `schematic-definitions`, so every load here
//! runs without the Rust registry; the committed fixture is the bundle tool's
//! output for the typed fixture API in `bundle_support`.

mod bundle_support;

use bundle_support::{fixture, fixture_bundle};
use schematic_define::{
    BundleError, DefinitionBundle, DefinitionError, ModelCatalog, ModelDef, RestApi, SchemaComponent, TypeAlias,
    TypeRef,
};
use serde_json::{Value, json};

fn loaded_fixture() -> DefinitionBundle {
    DefinitionBundle::from_json(include_str!("fixtures/bundles/fixture.json")).expect("committed fixture loads")
}

/// The OpenAPI export of `api`. The `openapi` feature provides it, and CI's
/// L1 enables every feature of this crate; without it this is empty.
#[cfg(feature = "openapi")]
fn openapi(api: &RestApi) -> Result<String, String> {
    use schematic_define::openapi::{ExportOptions, export};
    export(api, &ExportOptions::new()).map(|document| serde_json::to_string(&document).unwrap()).map_err(|e| e.to_string())
}

#[cfg(not(feature = "openapi"))]
fn openapi(_: &RestApi) -> Result<String, String> { Ok(String::new()) }

fn refs(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get("$ref") { found.push(reference.clone()); }
            object.values().for_each(|value| refs(value, found));
        }
        Value::Array(values) => values.iter().for_each(|value| refs(value, found)),
        _ => {}
    }
}

#[test]
fn committed_fixture_is_the_bundle_tool_output() {
    let expected = include_str!("fixtures/bundles/fixture.json");
    let actual = fixture_bundle().to_json();
    if actual != expected {
        let path = std::env::temp_dir().join("schematic-bundle-fixture.json");
        std::fs::write(&path, &actual).unwrap();
        panic!("tests/fixtures/bundles/fixture.json is stale; review the literals in {} and copy it over", path.display());
    }
}

#[test]
fn committed_fixture_loads_and_exports_like_the_built_definition() {
    let loaded = loaded_fixture();
    let (api, models) = fixture();
    assert_eq!(loaded.api(), &api);
    assert_eq!(loaded.models(), Some(&models));
    assert_eq!(openapi(loaded.api()).unwrap(), openapi(&api).unwrap());

    let names: Vec<_> = loaded.api().schemas.iter().map(|c| (c.name.as_str(), c.rust_path.is_some())).collect();
    assert_eq!(names, [("Node", true), ("Parent", true), ("Child", false), ("Item", true), ("Cat", true), ("Dog", true),
        ("Pet", true), ("Thing", true), ("Problem", true), ("IssuesVars", true), ("IssuesData", true), ("Reply", true), ("Event", true)]);
    // The alias keeps its schema identity apart from its Rust path.
    assert_eq!(loaded.api().component("Thing").unwrap().rust_path.as_deref(), Some("crate::bundle_support::models::Widget"));
    let mut found = Vec::new();
    for component in &loaded.api().schemas { refs(&component.schema, &mut found); }
    assert!(found.iter().all(|r| r.starts_with("#/components/schemas/")), "{found:?}");
    // The nested Child's recursion into its registered root names that root.
    assert!(found.contains(&"#/components/schemas/Parent".to_owned()));
    // Schema data keeps its nulls.
    assert!(loaded.api().component("Parent").unwrap().schema.to_string().contains("[null]"));
}

#[test]
fn bundle_files_round_trip_byte_equal_through_repeated_writes_and_reads() {
    let dir = std::env::temp_dir().join(format!("schematic-bundle-round-trip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let first = dir.join("first.json");
    std::fs::write(&first, fixture_bundle().to_json()).unwrap();
    let read = DefinitionBundle::from_json(&std::fs::read_to_string(&first).unwrap()).unwrap();
    let second = dir.join("second.json");
    std::fs::write(&second, read.to_json()).unwrap();
    let reread = DefinitionBundle::from_json(&std::fs::read_to_string(&second).unwrap()).unwrap();
    assert_eq!(reread, read);
    assert_eq!(std::fs::read(&first).unwrap(), std::fs::read(&second).unwrap());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn removed_endpoint_component_is_a_load_error_naming_api_endpoint_and_name() {
    let mut value: Value = serde_json::from_str(&fixture_bundle().to_json()).unwrap();
    value["api"]["schemas"].as_array_mut().unwrap().retain(|c| c["name"] != "Item");
    let error = DefinitionBundle::from_json(&value.to_string()).unwrap_err();
    assert_eq!(error, BundleError::Definition(DefinitionError::UnresolvedReference {
        api: "Fixture".into(), endpoint: "Lookup".into(), name: "Item".into() }));
}

#[test]
fn removed_nested_component_is_a_load_error_naming_its_referrer() {
    let mut value: Value = serde_json::from_str(&fixture_bundle().to_json()).unwrap();
    value["api"]["schemas"].as_array_mut().unwrap().retain(|c| c["name"] != "Child");
    let error = DefinitionBundle::from_json(&value.to_string()).unwrap_err();
    assert_eq!(error.to_string(), "Fixture schema component \"Parent\": reference \"#/components/schemas/Child\" names no component in the bundle");
}

#[test]
fn canonical_json_does_not_depend_on_map_insertion_order() {
    let nested = |reverse: bool| {
        let mut pairs = vec![("alpha", json!({"b": 1, "a": {"y": [3, 1], "x": null}})), ("omega", json!(true))];
        if reverse { pairs.reverse(); }
        Value::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    };
    let bundle = |reverse: bool| {
        let (mut api, _) = fixture();
        api.schemas[3].schema["properties"]["id"]["example"] = nested(reverse);
        DefinitionBundle::new(api).unwrap().to_json()
    };
    assert_eq!(bundle(false), bundle(true));
    // Arrays keep their order; data nulls stay.
    assert!(bundle(false).contains("\"y\": [\n"));
    let loaded = DefinitionBundle::from_json(&bundle(true)).unwrap();
    assert_eq!(loaded.api().schemas[3].schema["properties"]["id"]["example"]["alpha"]["a"], json!({"x": null, "y": [3, 1]}));
}

fn api_with_component(schema: Value, rust_path: Option<&str>) -> RestApi {
    let (mut api, _) = fixture();
    api.schemas.push(SchemaComponent { name: "Extra".into(), rust_path: rust_path.map(str::to_owned), schema });
    api
}

#[test]
fn construction_normalizes_local_references_into_the_component_namespace() {
    let schema = json!({"properties": {
        "a": {"$ref": "#/$defs/Item"}, "b": {"$ref": "#/definitions/Cat"}, "c": {"$ref": "#"},
        "d": {"default": {"$ref": "kept as data"}, "examples": [{"$ref": "https://data.example"}]}}});
    let bundle = DefinitionBundle::new(api_with_component(schema, Some("::fixture::models::Extra"))).unwrap();
    let extra = &bundle.api().component("Extra").unwrap().schema["properties"];
    assert_eq!((&extra["a"]["$ref"], &extra["b"]["$ref"], &extra["c"]["$ref"]),
        (&json!("#/components/schemas/Item"), &json!("#/components/schemas/Cat"), &json!("#/components/schemas/Extra")));
    assert_eq!(extra["d"], json!({"default": {"$ref": "kept as data"}, "examples": [{"$ref": "https://data.example"}]}));
    assert!(DefinitionBundle::from_json(&bundle.to_json()).is_ok());
}

#[test]
fn construction_refuses_external_unresolved_and_ambiguous_references() {
    let cases = [
        (json!({"$ref": "https://example.com/node.json"}), Some("::m::Extra"), "\"https://example.com/node.json\" is not a local component reference"),
        (json!({"$ref": "other.json#/components/schemas/Node"}), Some("::m::Extra"), "is not a local component reference"),
        (json!({"$ref": "#/paths/~1x"}), Some("::m::Extra"), "is not a local component reference"),
        (json!({"$ref": "#/components/schemas/Missing"}), Some("::m::Extra"), "\"#/components/schemas/Missing\" names no component"),
        // A nested-only component no longer knows which root `#` meant.
        (json!({"$ref": "#"}), None, "\"#\" is not a local component reference"),
    ];
    for (schema, rust_path, expected) in cases {
        let error = DefinitionBundle::new(api_with_component(schema.clone(), rust_path)).unwrap_err().to_string();
        assert!(error.starts_with("Fixture schema component \"Extra\": reference ") && error.contains(expected), "{schema}: {error}");
    }
}

#[test]
fn construction_runs_the_builder_validation() {
    let (mut api, _) = fixture();
    api.endpoints[0].path = "/nodes/{id}?x=1".into();
    let error = DefinitionBundle::new(api).unwrap_err();
    assert!(matches!(error, BundleError::Definition(DefinitionError::InvalidDefinition { ref endpoint, .. }) if endpoint == "GetNode"), "{error}");
}

#[test]
fn catalog_must_agree_with_the_bundled_components() {
    let alias = |name: &str, target: TypeRef| ModelDef::Alias(TypeAlias { name: name.into(), description: None, target });
    let pet = || alias("Pet", TypeRef::OneOf(vec![TypeRef::Named("Cat".into()), TypeRef::Named("Dog".into())]));
    let catalog = |types: Vec<ModelDef>| ModelCatalog { module_path: None, types };
    let bundle = || DefinitionBundle::new(fixture().0).unwrap();
    assert!(bundle().with_models(catalog(vec![])).unwrap().models().is_some_and(|m| m.types.is_empty()));
    let cases = [
        (catalog(vec![pet(), pet()]), "model \"Pet\" is listed more than once"),
        (catalog(vec![alias("Ghost", TypeRef::Named("Cat".into()))]), "model \"Ghost\" is not a registered component of the bundle"),
        (catalog(vec![alias("Child", TypeRef::Named("Cat".into()))]), "model \"Child\" is not a registered component of the bundle"),
        (catalog(vec![alias("Pet", TypeRef::OneOf(vec![TypeRef::Named("Child".into())]))]), "model \"Pet\" names \"Child\", which is not a registered component"),
        (catalog(vec![alias("Pet", TypeRef::OneOf(vec![TypeRef::Named("Cat".into())]))]), "composition \"Pet\" disagrees with its component schema"),
        (catalog(vec![alias("Pet", TypeRef::OneOf(vec![]))]), "composition \"Pet\" needs one or more component members"),
        (ModelCatalog { module_path: Some(" ".into()), types: vec![pet()] }, "module_path is empty"),
    ];
    for (models, expected) in cases {
        let error = bundle().with_models(models).unwrap_err().to_string();
        assert_eq!(error, format!("Fixture model catalog: {expected}"));
    }
}

/// One edit to the canonical fixture text or its JSON value.
enum Edit {
    Set(&'static str, Value),
    /// Append to the array at the pointer.
    Push(&'static str, Value),
    Remove(&'static str),
    Text(fn(&str) -> String),
}

/// `Loads` holds the observable consequence; `Fails` a substring that
/// locates the problem in the rendered error.
enum Expect {
    Loads(fn(&DefinitionBundle) -> bool),
    Fails(&'static str),
}

use Edit::{Push, Remove, Set, Text};
use Expect::{Fails, Loads};

fn parent_of(pointer: &str) -> (&str, &str) { pointer.rsplit_once('/').unwrap() }

fn apply(edit: &Edit, canonical: &str) -> String {
    let mut value: Value = serde_json::from_str(canonical).unwrap();
    match edit {
        Set(pointer, new) => {
            let (parent, key) = parent_of(pointer);
            match value.pointer_mut(parent).unwrap_or_else(|| panic!("no {parent}")) {
                Value::Object(object) => { object.insert(key.into(), new.clone()); }
                Value::Array(values) => values[key.parse::<usize>().unwrap()] = new.clone(),
                _ => panic!("{parent} is not a container"),
            }
            serde_json::to_string_pretty(&value).unwrap()
        }
        Push(pointer, new) => {
            value.pointer_mut(pointer).and_then(Value::as_array_mut).unwrap_or_else(|| panic!("no array {pointer}")).push(new.clone());
            serde_json::to_string_pretty(&value).unwrap()
        }
        Remove(pointer) => {
            let (parent, key) = parent_of(pointer);
            let removed = value.pointer_mut(parent).and_then(Value::as_object_mut).and_then(|o| o.remove(key));
            assert!(removed.is_some(), "{pointer} is absent from the control");
            serde_json::to_string_pretty(&value).unwrap()
        }
        Text(edit) => {
            let edited = edit(canonical);
            assert_ne!(edited, canonical, "text edit had no effect");
            edited
        }
    }
}

fn duplicate(text: &str, line: &str) -> String {
    assert!(text.contains(line), "{line}");
    text.replacen(line, &format!("{line}\n{line}"), 1)
}

const API_RATE: &str = "/api/rate_limit";
const API_RETRY: &str = "/api/retry_guidance";
const API_ERRORS: &str = "/api/errors";
const GRAPHQL: &str = "/api/graphql";
const OPERATIONS: &str = "/api/graphql/operations";

fn json_shape(name: &str) -> schematic_define::ResponseShape {
    schematic_define::ResponseShape::Json(TypeRef::Named(name.into()))
}

fn cells() -> Vec<(&'static str, Edit, Expect)> {
    vec![
        ("control", Text(|t| format!("\n {t}\n\n")), Loads(|b| b.api().name == "Fixture" && b.models().is_some())),
        // Bundle root.
        ("format_version absent", Remove("/format_version"), Fails("bundle field /format_version: missing")),
        ("format_version null", Set("/format_version", Value::Null), Fails("/format_version: expected an integer, found null")),
        ("format_version string", Set("/format_version", json!("1")), Fails("/format_version: expected an integer, found a string")),
        ("format_version empty", Set("/format_version", json!("")), Fails("/format_version: expected an integer, found a string")),
        ("format_version 0", Set("/format_version", json!(0)), Fails("unsupported bundle format_version 0")),
        ("format_version 2", Set("/format_version", json!(2)), Fails("unsupported bundle format_version 2")),
        ("format_version 1.5", Set("/format_version", json!(1.5)), Fails("unsupported bundle format_version 1.5")),
        ("root unknown member", Set("/extra", json!(1)), Fails("unknown field `extra`")),
        ("api absent", Remove("/api"), Fails("missing field `api`")),
        ("api null", Set("/api", Value::Null), Fails("bundle field /api: explicit null")),
        ("api wrong type", Set("/api", json!([])), Fails("expected struct RestApi")),
        ("api empty", Set("/api", json!({})), Fails("missing field")),
        ("api unknown member", Set("/api/extra_field", json!(true)), Fails("bundle field /api/extra_field: unknown member")),
        ("endpoint misspelled member", Set("/api/endpoints/0/deprecatd", json!(true)), Fails("/api/endpoints/0/deprecatd: unknown member")),
        ("endpoint explicit default false", Set("/api/endpoints/0/deprecated", json!(false)), Loads(|b| !b.api().endpoints[0].deprecated)),
        ("models absent", Remove("/models"), Loads(|b| b.models().is_none())),
        ("models null", Set("/models", Value::Null), Fails("bundle field /models: explicit null")),
        ("models wrong type", Set("/models", json!("x")), Fails("invalid type: string")),
        ("models empty object", Set("/models", json!({})), Fails("missing field `types`")),
        ("models empty catalog", Set("/models/types", json!([])), Loads(|b| b.models().is_some_and(|m| m.types.is_empty()))),
        ("models one wrong element", Push("/models/types", json!(5)), Fails("line")),
        ("models every element wrong", Set("/models/types", json!([5])), Fails("bundle JSON line 803 column 7")),
        ("models unregistered", Set("/models/types/0/Alias/name", json!("Child")), Fails("model catalog: model \"Child\" is not a registered component")),
        ("models disagree", Set("/models/types/0/Alias/target/OneOf/1/Named", json!("Cat")), Fails("composition \"Pet\" disagrees")),
        // Components: schema identities, bindings, and schemas.
        ("schemas absent", Remove("/api/schemas"), Fails("missing field `schemas`")),
        ("schemas null", Set("/api/schemas", Value::Null), Fails("bundle field /api/schemas: explicit null")),
        ("schemas wrong type", Set("/api/schemas", json!({})), Fails("invalid type: map")),
        ("schemas empty", Set("/api/schemas", json!([])), Fails("Fixture endpoint <api>: unresolved schema reference \"Problem\"")),
        ("schemas one wrong element", Set("/api/schemas/2", json!(5)), Fails("invalid type: integer")),
        ("schemas every element wrong", Set("/api/schemas", json!([5])), Fails("invalid type: integer")),
        ("schema identity absent", Remove("/api/schemas/0/name"), Fails("missing field `name`")),
        ("schema identity null", Set("/api/schemas/0/name", Value::Null), Fails("/api/schemas/0/name: explicit null")),
        ("schema identity wrong type", Set("/api/schemas/0/name", json!(5)), Fails("invalid type: integer")),
        ("schema identity empty", Set("/api/schemas/0/name", json!("")), Fails("schema component 0 has an empty name")),
        ("schema identity duplicate", Set("/api/schemas/7/name", json!("Node")), Fails("schema component \"Node\" is defined more than once")),
        ("binding absent", Remove("/api/schemas/0/rust_path"), Fails("Fixture endpoint GetNode: unresolved schema reference \"Node\"")),
        ("binding null", Set("/api/schemas/0/rust_path", Value::Null), Fails("/api/schemas/0/rust_path: explicit null")),
        ("binding wrong type", Set("/api/schemas/0/rust_path", json!(5)), Fails("invalid type: integer")),
        ("binding empty", Set("/api/schemas/0/rust_path", json!("")), Fails("schema component \"Node\" has an empty Rust path")),
        ("schema absent", Remove("/api/schemas/0/schema"), Fails("missing field `schema`")),
        ("schema null", Set("/api/schemas/0/schema", Value::Null), Fails("schema component \"Node\" is not a JSON Schema object or boolean")),
        ("schema wrong type", Set("/api/schemas/0/schema", json!("x")), Fails("schema component \"Node\" is not a JSON Schema object or boolean")),
        ("schema empty is arbitrary JSON", Set("/api/schemas/3/schema", json!({})), Loads(|b| b.api().schemas[3].schema == json!({}))),
        ("schema external reference", Set("/api/schemas/0/schema/properties/children/items/$ref", json!("https://example.com/node.json")),
            Fails("\"Node\": reference \"https://example.com/node.json\" is not a local component reference")),
        ("schema dangling reference", Set("/api/schemas/0/schema/properties/children/items/$ref", json!("#/components/schemas/Gone")),
            Fails("\"Node\": reference \"#/components/schemas/Gone\" names no component")),
        // API metadata.
        ("api rate_limit absent", Remove(API_RATE), Loads(|b| b.api().rate_limit.is_none() && b.api().effective_rate_limit(&b.api().endpoints[1]).is_none())),
        ("api rate_limit null", Set(API_RATE, Value::Null), Fails("/api/rate_limit: explicit null")),
        ("api rate_limit wrong type", Set(API_RATE, json!("x")), Fails("invalid type: string")),
        ("api rate_limit empty", Set(API_RATE, json!({})), Fails("missing field `limit_header`")),
        ("api rate_limit nested wrong", Set("/api/rate_limit/limit_header", json!(5)), Fails("invalid type: integer")),
        ("api retry_guidance absent", Remove(API_RETRY), Loads(|b| b.api().retry_guidance.is_none())),
        ("api retry_guidance null", Set(API_RETRY, Value::Null), Fails("/api/retry_guidance: explicit null")),
        ("api retry_guidance wrong type", Set(API_RETRY, json!(1)), Fails("invalid type: integer")),
        ("api retry_guidance empty", Set(API_RETRY, json!({})), Fails("missing field `retry_on`")),
        ("api retry_guidance nested wrong", Set("/api/retry_guidance/retry_on/1", json!(5)), Fails("bundle JSON line 531 column 9")),
        // Endpoint vocabulary.
        ("endpoint rate_limit absent inherits", Remove("/api/endpoints/0/rate_limit"),
            Loads(|b| b.api().effective_rate_limit(&b.api().endpoints[0]) == b.api().rate_limit.as_ref())),
        ("endpoint rate_limit null", Set("/api/endpoints/0/rate_limit", Value::Null), Fails("/api/endpoints/0/rate_limit: explicit null")),
        ("endpoint rate_limit wrong type", Set("/api/endpoints/0/rate_limit", json!([])), Fails("expected struct RawRateLimit")),
        ("endpoint rate_limit empty", Set("/api/endpoints/0/rate_limit", json!({})), Fails("missing field `limit_header`")),
        ("endpoint rate_limit nested wrong", Set("/api/endpoints/0/rate_limit/reset_unit", json!("minutes")), Fails("unknown variant `minutes`")),
        ("endpoint retry absent inherits", Remove("/api/endpoints/2/retry_guidance"),
            Loads(|b| b.api().effective_retry_guidance(&b.api().endpoints[2]) == b.api().retry_guidance.as_ref())),
        ("endpoint retry null", Set("/api/endpoints/2/retry_guidance", Value::Null), Fails("/api/endpoints/2/retry_guidance: explicit null")),
        ("endpoint retry wrong type", Set("/api/endpoints/2/retry_guidance", json!("x")), Fails("invalid type: string")),
        ("endpoint retry empty", Set("/api/endpoints/2/retry_guidance", json!({})), Fails("missing field `retry_on`")),
        ("endpoint retry nested wrong", Set("/api/endpoints/2/retry_guidance/retry_on/0/code", json!("503")), Fails("invalid type: string")),
        ("endpoint auth absent inherits", Remove("/api/endpoints/2/auth"), Loads(|b| b.api().endpoints[2].auth.is_none())),
        ("endpoint auth public", Set("/api/endpoints/2/auth", json!("Public")), Loads(|b| b.api().endpoints[2].auth == Some(schematic_define::EndpointAuth::Public))),
        ("endpoint auth null", Set("/api/endpoints/2/auth", Value::Null), Fails("/api/endpoints/2/auth: explicit null")),
        ("endpoint auth wrong type", Set("/api/endpoints/2/auth", json!(5)), Fails("bundle JSON line 156 column 17")),
        ("endpoint auth empty", Set("/api/endpoints/2/auth", json!({})), Fails("line")),
        ("endpoint auth nested wrong", Set("/api/endpoints/2/auth/Policy/policy/explicit/0", json!(5)), Fails("line")),
        ("endpoint auth accepts nothing", Set("/api/endpoints/2/auth/Policy/policy/explicit", json!([])),
            Fails("Fixture endpoint Lookup: endpoint auth policy accepts no authentication method")),
        ("endpoint deprecated absent", Remove("/api/endpoints/1/deprecated"), Loads(|b| !b.api().endpoints[1].deprecated)),
        ("endpoint deprecated true", Set("/api/endpoints/0/deprecated", json!(true)), Loads(|b| b.api().endpoints[0].deprecated)),
        ("endpoint deprecated null", Set("/api/endpoints/1/deprecated", Value::Null), Fails("/api/endpoints/1/deprecated: explicit null")),
        ("endpoint deprecated string", Set("/api/endpoints/1/deprecated", json!("true")), Fails("invalid type: string")),
        ("endpoint deprecated empty", Set("/api/endpoints/1/deprecated", json!("")), Fails("invalid type: string")),
        ("param deprecated absent", Remove("/api/endpoints/1/params/query/0/deprecated"), Loads(|b| !b.api().endpoints[1].params.as_ref().unwrap().query[0].deprecated)),
        ("param deprecated null", Set("/api/endpoints/1/params/query/0/deprecated", Value::Null), Fails("/api/endpoints/1/params/query/0/deprecated: explicit null")),
        ("param deprecated string", Set("/api/endpoints/1/params/query/0/deprecated", json!("yes")), Fails("invalid type: string")),
        ("params path absent is legacy", Remove("/api/endpoints/0/params/path"), Loads(|b| b.api().endpoints[0].params.as_ref().unwrap().path.is_empty())),
        ("params path empty is legacy", Set("/api/endpoints/0/params/path", json!([])), Loads(|b| b.api().endpoints[0].params.as_ref().unwrap().path.is_empty())),
        ("params path null", Set("/api/endpoints/0/params/path", Value::Null), Fails("/api/endpoints/0/params/path: explicit null")),
        ("params path wrong type", Set("/api/endpoints/0/params/path", json!("id")), Fails("invalid type: string")),
        ("params path one wrong element", Push("/api/endpoints/0/params/path", json!(5)), Fails("invalid type: integer")),
        ("params path every element wrong", Set("/api/endpoints/0/params/path", json!([5])), Fails("invalid type: integer")),
        ("params path undeclared placeholder", Set("/api/endpoints/0/params/path/0/name", json!("other")), Fails("Fixture endpoint GetNode:")),
        // Rate-limit fields.
        ("limit_header absent", Remove("/api/rate_limit/limit_header"), Fails("missing field `limit_header`")),
        ("limit_header null", Set("/api/rate_limit/limit_header", Value::Null), Fails("/api/rate_limit/limit_header: explicit null")),
        ("limit_header empty", Set("/api/rate_limit/limit_header", json!("")), Fails("`limit_header` is not a valid HTTP header name")),
        ("remaining_header absent", Remove("/api/rate_limit/remaining_header"), Fails("missing field `remaining_header`")),
        ("remaining_header null", Set("/api/rate_limit/remaining_header", Value::Null), Fails("/api/rate_limit/remaining_header: explicit null")),
        ("remaining_header wrong type", Set("/api/rate_limit/remaining_header", json!(true)), Fails("invalid type: boolean")),
        ("remaining_header empty", Set("/api/rate_limit/remaining_header", json!("")), Fails("`remaining_header` is not a valid HTTP header name")),
        ("reset_header absent", Remove("/api/rate_limit/reset_header"), Fails("missing field `reset_header`")),
        ("reset_header null", Set("/api/rate_limit/reset_header", Value::Null), Fails("/api/rate_limit/reset_header: explicit null")),
        ("reset_header wrong type", Set("/api/rate_limit/reset_header", json!([])), Fails("invalid type: sequence")),
        ("reset_header empty", Set("/api/rate_limit/reset_header", json!("")), Fails("`reset_header` is not a valid HTTP header name")),
        ("retry_after_header absent", Remove("/api/rate_limit/retry_after_header"), Loads(|b| b.api().rate_limit.as_ref().unwrap().retry_after_header.is_none())),
        ("retry_after_header null", Set("/api/rate_limit/retry_after_header", Value::Null), Fails("/api/rate_limit/retry_after_header: explicit null")),
        ("retry_after_header wrong type", Set("/api/rate_limit/retry_after_header", json!(1)), Fails("invalid type: integer")),
        ("retry_after_header empty", Set("/api/rate_limit/retry_after_header", json!("")), Fails("`retry_after_header` is not a valid HTTP header name")),
        ("reset_unit absent", Remove("/api/rate_limit/reset_unit"), Fails("missing field `reset_unit`")),
        ("reset_unit null", Set("/api/rate_limit/reset_unit", Value::Null), Fails("/api/rate_limit/reset_unit: explicit null")),
        ("reset_unit wrong type", Set("/api/rate_limit/reset_unit", json!(1)), Fails("bundle JSON line 515 column 21")),
        ("reset_unit empty", Set("/api/rate_limit/reset_unit", json!("")), Fails("unknown variant ``")),
        // Retry-guidance fields.
        ("retry_on absent", Remove("/api/retry_guidance/retry_on"), Fails("missing field `retry_on`")),
        ("retry_on null", Set("/api/retry_guidance/retry_on", Value::Null), Fails("/api/retry_guidance/retry_on: explicit null")),
        ("retry_on wrong type", Set("/api/retry_guidance/retry_on", json!({"code": 429})), Fails("invalid type: map")),
        ("retry_on every element wrong", Set("/api/retry_guidance/retry_on", json!([429])), Fails("line")),
        ("retry_on empty selects nothing", Set("/api/retry_guidance/retry_on", json!([])), Loads(|b| b.api().retry_guidance.as_ref().unwrap().retry_on.is_empty())),
        ("honor_retry_after absent", Remove("/api/retry_guidance/honor_retry_after"), Fails("missing field `honor_retry_after`")),
        ("honor_retry_after null", Set("/api/retry_guidance/honor_retry_after", Value::Null), Fails("/api/retry_guidance/honor_retry_after: explicit null")),
        ("honor_retry_after string", Set("/api/retry_guidance/honor_retry_after", json!("true")), Fails("invalid type: string")),
        ("max_attempts absent", Remove("/api/retry_guidance/max_attempts"), Fails("missing field `max_attempts`")),
        ("max_attempts null", Set("/api/retry_guidance/max_attempts", Value::Null), Fails("/api/retry_guidance/max_attempts: explicit null")),
        ("max_attempts string", Set("/api/retry_guidance/max_attempts", json!("3")), Fails("invalid type: string")),
        ("max_attempts zero", Set("/api/retry_guidance/max_attempts", json!(0)), Fails("`max_attempts` must be at least 1")),
        ("backoff absent", Remove("/api/retry_guidance/backoff"), Fails("missing field `backoff`")),
        ("backoff null", Set("/api/retry_guidance/backoff", Value::Null), Fails("/api/retry_guidance/backoff: explicit null")),
        ("backoff wrong type", Set("/api/retry_guidance/backoff", json!(5)), Fails("line")),
        ("backoff empty", Set("/api/retry_guidance/backoff", json!({})), Fails("line")),
        ("backoff none tag", Set("/api/retry_guidance/backoff", json!("none")), Loads(|b| b.api().retry_guidance.as_ref().unwrap().backoff == schematic_define::Backoff::None)),
        ("selector code null", Set("/api/retry_guidance/retry_on/0/code", Value::Null), Fails("/api/retry_guidance/retry_on/0/code: explicit null")),
        ("selector code string", Set("/api/retry_guidance/retry_on/0/code", json!("429")), Fails("invalid type: string")),
        ("selector code out of range", Set("/api/retry_guidance/retry_on/0/code", json!(99)), Fails("`code` status code 99 is outside 100..=599")),
        ("range start absent", Remove("/api/retry_guidance/retry_on/1/range/start"), Fails("missing field `start`")),
        ("range start null", Set("/api/retry_guidance/retry_on/1/range/start", Value::Null), Fails("/api/retry_guidance/retry_on/1/range/start: explicit null")),
        ("range end wrong type", Set("/api/retry_guidance/retry_on/1/range/end", json!("599")), Fails("invalid type: string")),
        ("range reversed", Set("/api/retry_guidance/retry_on/1/range/end", json!(400)), Fails("`start` 500 is greater than `end` 400")),
        ("fixed delay absent", Remove("/api/endpoints/2/retry_guidance/backoff/fixed/delay_ms"), Fails("missing field `delay_ms`")),
        ("fixed delay null", Set("/api/endpoints/2/retry_guidance/backoff/fixed/delay_ms", Value::Null), Fails("backoff/fixed/delay_ms: explicit null")),
        ("fixed delay zero", Set("/api/endpoints/2/retry_guidance/backoff/fixed/delay_ms", json!(0)), Fails("`delay_ms` must be greater than 0 ms")),
        ("exponential base absent", Remove("/api/retry_guidance/backoff/exponential/base_ms"), Fails("missing field `base_ms`")),
        ("exponential base null", Set("/api/retry_guidance/backoff/exponential/base_ms", Value::Null), Fails("exponential/base_ms: explicit null")),
        ("exponential cap wrong type", Set("/api/retry_guidance/backoff/exponential/cap_ms", json!("8000")), Fails("invalid type: string")),
        ("exponential cap below base", Set("/api/retry_guidance/backoff/exponential/cap_ms", json!(10)), Fails("`cap_ms` 10 is less than `base_ms` 250")),
        // Basic environment lists.
        ("username_env absent", Remove("/api/auth/Basic/username_env"), Fails("missing field `username_env`")),
        ("username_env null", Set("/api/auth/Basic/username_env", Value::Null), Fails("/api/auth/Basic/username_env: explicit null")),
        ("username_env wrong type", Set("/api/auth/Basic/username_env", json!("FIXTURE_USER")), Fails("invalid type: string")),
        ("username names one wrong element", Set("/api/auth/Basic/username_env/names", json!(["FIXTURE_USER", 5])), Fails("invalid type: integer")),
        ("username names every element wrong", Set("/api/auth/Basic/username_env/names", json!([5])), Fails("invalid type: integer")),
        ("username names empty disables", Set("/api/auth/Basic/username_env/names", json!([])), Loads(|b| b.api().default_env_mapping().basic_user.is_none())),
        ("username wrapper empty", Set("/api/auth/Basic/username_env", json!({})), Fails("missing field `names`")),
        ("password_env absent", Remove("/api/auth/Basic/password_env"), Fails("missing field `password_env`")),
        ("password_env null", Set("/api/auth/Basic/password_env", Value::Null), Fails("/api/auth/Basic/password_env: explicit null")),
        ("password names one wrong element", Set("/api/auth/Basic/password_env/names/1", json!(false)), Fails("invalid type: boolean")),
        ("password names every element wrong", Set("/api/auth/Basic/password_env/names", json!([1, 2])), Fails("invalid type: integer")),
        ("password names empty disables", Set("/api/auth/Basic/password_env/names", json!([])), Loads(|b| b.api().default_env_mapping().basic_pass.is_none())),
        ("password order is kept", Set("/api/auth/Basic/password_env/names", json!(["B", "A"])),
            Loads(|b| b.api().default_env_mapping().basic_pass.unwrap().names() == ["B", "A"])),
        // OAuth2: authorization code with PKCE.
        ("code grant_type absent", Remove("/api/auth_policy/explicit/1/OAuth2/grant_type"), Fails("missing field `grant_type`")),
        ("code grant_type null", Set("/api/auth_policy/explicit/1/OAuth2/grant_type", Value::Null), Fails("explicit/1/OAuth2/grant_type: explicit null")),
        ("code grant_type empty", Set("/api/auth_policy/explicit/1/OAuth2/grant_type", json!("")), Fails("unknown variant ``")),
        ("code token_url absent", Remove("/api/auth_policy/explicit/1/OAuth2/token_url"), Fails("missing field `token_url`")),
        ("code token_url null", Set("/api/auth_policy/explicit/1/OAuth2/token_url", Value::Null), Fails("explicit/1/OAuth2/token_url: explicit null")),
        ("code token_url wrong type", Set("/api/auth_policy/explicit/1/OAuth2/token_url", json!(5)), Fails("invalid type: integer")),
        ("code token_url empty", Set("/api/auth_policy/explicit/1/OAuth2/token_url", json!("")), Fails("`token_url` must be an absolute http or https URL with a host, got \"\": empty at line")),
        ("code authorization_url absent", Remove("/api/auth_policy/explicit/1/OAuth2/authorization_url"), Fails("authorization_url")),
        ("code authorization_url null", Set("/api/auth_policy/explicit/1/OAuth2/authorization_url", Value::Null), Fails("authorization_url")),
        ("code authorization_url empty", Set("/api/auth_policy/explicit/1/OAuth2/authorization_url", json!("")), Fails("`authorization_url` must be an absolute http or https URL")),
        ("code device url null is inapplicable", Set("/api/auth_policy/explicit/1/OAuth2/device_authorization_url", Value::Null), Loads(|b| oauth(b, 1).device_authorization_url().is_none())),
        ("code device url contradicts", Set("/api/auth_policy/explicit/1/OAuth2/device_authorization_url", json!("https://auth.example.com/device")), Fails("device_authorization_url")),
        ("code revocation_url absent", Remove("/api/auth_policy/explicit/1/OAuth2/revocation_url"), Loads(|b| oauth(b, 1).revocation_url().is_none())),
        ("code revocation_url null", Set("/api/auth_policy/explicit/1/OAuth2/revocation_url", Value::Null), Fails("explicit/1/OAuth2/revocation_url: explicit null")),
        ("code revocation_url empty", Set("/api/auth_policy/explicit/1/OAuth2/revocation_url", json!("")), Fails("`revocation_url` must be an absolute http or https URL")),
        ("code scopes absent", Remove("/api/auth_policy/explicit/1/OAuth2/default_scopes"), Fails("missing field `default_scopes`")),
        ("code scopes null", Set("/api/auth_policy/explicit/1/OAuth2/default_scopes", Value::Null), Fails("explicit/1/OAuth2/default_scopes: explicit null")),
        ("code scopes wrong type", Set("/api/auth_policy/explicit/1/OAuth2/default_scopes", json!("read")), Fails("invalid type: string")),
        ("code scopes one wrong element", Set("/api/auth_policy/explicit/1/OAuth2/default_scopes/1", json!(5)), Fails("invalid type: integer")),
        ("code scopes every element wrong", Set("/api/auth_policy/explicit/1/OAuth2/default_scopes", json!([5])), Fails("invalid type: integer")),
        ("code scopes empty", Set("/api/auth_policy/explicit/1/OAuth2/default_scopes", json!([])), Loads(|b| oauth(b, 1).default_scopes().is_empty())),
        ("code pkce absent", Remove("/api/auth_policy/explicit/1/OAuth2/pkce"), Fails("pkce")),
        ("code pkce null", Set("/api/auth_policy/explicit/1/OAuth2/pkce", Value::Null), Fails("pkce")),
        ("code pkce required", Set("/api/auth_policy/explicit/1/OAuth2/pkce", json!("Required")), Loads(|b| oauth(b, 1).pkce() == Some(schematic_define::PkceRequirement::Required))),
        ("code pkce unknown", Set("/api/auth_policy/explicit/1/OAuth2/pkce", json!("Maybe")), Fails("unknown variant `Maybe`")),
        ("code client_auth absent", Remove("/api/auth_policy/explicit/1/OAuth2/client_auth"), Fails("missing field `client_auth`")),
        ("code client_auth null", Set("/api/auth_policy/explicit/1/OAuth2/client_auth", Value::Null), Fails("explicit/1/OAuth2/client_auth: explicit null")),
        ("code client_auth empty", Set("/api/auth_policy/explicit/1/OAuth2/client_auth", json!("")), Fails("unknown variant ``")),
        ("code unknown member", Set("/api/auth_policy/explicit/1/OAuth2/scope", json!("x")), Fails("unknown field `scope`")),
        // OAuth2: client credentials.
        ("client authorization_url null", Set("/api/auth_policy/explicit/2/OAuth2/authorization_url", Value::Null), Loads(|b| oauth(b, 2).authorization_url().is_none())),
        ("client authorization_url contradicts", Set("/api/auth_policy/explicit/2/OAuth2/authorization_url", json!("https://auth.example.com/a")), Fails("authorization_url")),
        ("client authorization_url empty contradicts", Set("/api/auth_policy/explicit/2/OAuth2/authorization_url", json!("")), Fails("authorization_url")),
        ("client pkce null", Set("/api/auth_policy/explicit/2/OAuth2/pkce", Value::Null), Loads(|b| oauth(b, 2).pkce().is_none())),
        ("client pkce contradicts", Set("/api/auth_policy/explicit/2/OAuth2/pkce", json!("Required")), Fails("pkce")),
        ("client device url null", Set("/api/auth_policy/explicit/2/OAuth2/device_authorization_url", Value::Null), Loads(|b| oauth(b, 2).device_authorization_url().is_none())),
        ("client token_url empty", Set("/api/auth_policy/explicit/2/OAuth2/token_url", json!("")), Fails("`token_url` must be an absolute http or https URL")),
        ("client scopes absent", Remove("/api/auth_policy/explicit/2/OAuth2/default_scopes"), Fails("missing field `default_scopes`")),
        ("client client_auth wrong type", Set("/api/auth_policy/explicit/2/OAuth2/client_auth", json!(1)), Fails("line")),
        // OAuth2: device code.
        ("device url absent", Remove("/api/auth_policy/explicit/3/OAuth2/device_authorization_url"), Fails("device_authorization_url")),
        ("device url null", Set("/api/auth_policy/explicit/3/OAuth2/device_authorization_url", Value::Null), Fails("device_authorization_url")),
        ("device url wrong type", Set("/api/auth_policy/explicit/3/OAuth2/device_authorization_url", json!(5)), Fails("invalid type: integer")),
        ("device url empty", Set("/api/auth_policy/explicit/3/OAuth2/device_authorization_url", json!("")), Fails("`device_authorization_url` must be an absolute http or https URL")),
        ("device authorization_url null", Set("/api/auth_policy/explicit/3/OAuth2/authorization_url", Value::Null), Loads(|b| oauth(b, 3).authorization_url().is_none())),
        ("device pkce contradicts", Set("/api/auth_policy/explicit/3/OAuth2/pkce", json!("Supported")), Fails("pkce")),
        ("device token_url wrong type", Set("/api/auth_policy/explicit/3/OAuth2/token_url", json!(["x"])), Fails("invalid type: sequence")),
        // Declared errors: API defaults, endpoint overlay, and opt-out.
        ("api errors absent", Remove(API_ERRORS), Loads(|b| b.api().errors.is_empty() && b.api().effective_errors(&b.api().endpoints[1]).entries().is_empty())),
        ("api errors null", Set(API_ERRORS, Value::Null), Fails("/api/errors: explicit null")),
        ("api errors wrong type", Set(API_ERRORS, json!({})), Fails("invalid type: map")),
        ("api errors one wrong element", Push(API_ERRORS, json!(5)), Fails("invalid type: integer")),
        ("api errors every element wrong", Set(API_ERRORS, json!([5])), Fails("invalid type: integer")),
        ("api errors empty", Set(API_ERRORS, json!([])), Loads(|b| b.api().effective_errors(&b.api().endpoints[1]).entries().is_empty())),
        ("api errors inherited", Text(|t| format!(" {t}")), Loads(|b| b.api().effective_errors(&b.api().endpoints[1]).lookup(503).is_some_and(|e| e.shape == json_shape("Problem")))),
        ("error status absent", Remove("/api/errors/1/status"), Fails("missing field `status`")),
        ("error status null", Set("/api/errors/1/status", Value::Null), Fails("/api/errors/1/status: explicit null")),
        ("error status wrong type", Set("/api/errors/1/status", json!(404)), Fails("expected value")),
        ("error status unknown variant", Set("/api/errors/1/status", json!({"Class": 4})), Fails("unknown variant `Class`")),
        ("error exact out of domain", Set("/api/errors/1/status/Exact", json!(302)), Fails("Fixture endpoint <api>: error status match 302 is outside 400-599")),
        ("error exact wrong type", Set("/api/errors/1/status/Exact", json!("404")), Fails("invalid type: string")),
        ("error range lo absent", Remove("/api/errors/0/status/Range/lo"), Fails("missing field `lo`")),
        ("error range hi null", Set("/api/errors/0/status/Range/hi", Value::Null), Fails("/api/errors/0/status/Range/hi: explicit null")),
        ("error range lo out of domain", Set("/api/errors/0/status/Range/lo", json!(600)), Fails("error status match 600-599 is outside 400-599")),
        ("error range reversed", Set("/api/errors/0/status/Range/hi", json!(450)), Fails("Fixture endpoint <api>: error status range 500-450 is reversed")),
        ("error range equal-width overlap", Push(API_ERRORS, json!({"status": {"Range": {"lo": 450, "hi": 549}}, "shape": "Text"})),
            Fails("Fixture endpoint <api>: error ranges 500-599 and 450-549 have equal width and overlap on statuses 500-549")),
        ("error duplicate match", Push(API_ERRORS, json!({"status": {"Exact": 404}, "shape": "Empty"})), Fails("Fixture endpoint <api>: error status match 404 is declared more than once")),
        ("error shape absent", Remove("/api/errors/1/shape"), Fails("missing field `shape`")),
        ("error shape stream", Set("/api/errors/1/shape", json!({"Stream": {"framing": "Bytes", "end": "ConnectionClose"}})), Fails("error 404: a stream cannot be an error shape")),
        ("error shape dangling", Set("/api/errors/0/shape/Json/Named", json!("Gone")), Fails("Fixture endpoint <api>: unresolved schema reference \"Gone\"")),
        ("error shape empty object", Set("/api/errors/0/shape", json!({})), Fails("expected value")),
        ("error content_type null", Set("/api/errors/1/content_type", Value::Null), Fails("/api/errors/1/content_type: explicit null")),
        ("error content_type wildcard", Set("/api/errors/1/content_type", json!("text/*")), Fails("error 404: media type `text/*`")),
        ("error content_type empty", Set("/api/errors/1/content_type", json!("")), Fails("error 404: media type ``")),
        ("error content_type on empty", Set("/api/endpoints/0/errors/0/content_type", json!("text/plain")), Fails("Fixture endpoint GetNode: error 404: an empty shape has no media type")),
        ("endpoint errors absent", Remove("/api/endpoints/0/errors"), Loads(|b| b.api().effective_errors(&b.api().endpoints[0]).lookup(404).is_some_and(|e| e.shape == schematic_define::ResponseShape::Text))),
        ("endpoint errors empty still inherits", Set("/api/endpoints/0/errors", json!([])), Loads(|b| b.api().effective_errors(&b.api().endpoints[0]).entries().len() == 2)),
        ("endpoint errors overlay", Text(|t| format!(" {t}")), Loads(|b| { let table = b.api().effective_errors(&b.api().endpoints[0]);
            table.lookup(404).is_some_and(|e| e.shape == schematic_define::ResponseShape::Empty) && table.lookup(425).is_some_and(|e| e.content_type.as_deref() == Some("text/html")) })),
        ("endpoint errors null", Set("/api/endpoints/0/errors", Value::Null), Fails("/api/endpoints/0/errors: explicit null")),
        ("endpoint errors one wrong element", Push("/api/endpoints/0/errors", json!("x")), Fails("invalid type: string")),
        ("endpoint errors equal-width overlap", Push("/api/endpoints/0/errors", json!({"status": {"Range": {"lo": 425, "hi": 434}}, "shape": "Text"})),
            Fails("Fixture endpoint GetNode: error ranges 420-429 and 425-434 have equal width and overlap on statuses 425-429")),
        ("opt-out absent", Remove("/api/endpoints/3/no_declared_errors"), Loads(|b| b.api().effective_errors(&b.api().endpoints[3]).entries().len() == 2)),
        ("opt-out explicit false", Set("/api/endpoints/3/no_declared_errors", json!(false)), Loads(|b| !b.api().endpoints[3].no_declared_errors)),
        ("opt-out empties the table", Text(|t| format!(" {t}")), Loads(|b| b.api().effective_errors(&b.api().endpoints[3]).entries().is_empty())),
        ("opt-out null", Set("/api/endpoints/3/no_declared_errors", Value::Null), Fails("/api/endpoints/3/no_declared_errors: explicit null")),
        ("opt-out wrong type", Set("/api/endpoints/3/no_declared_errors", json!("yes")), Fails("invalid type: string")),
        ("opt-out with entries", Set("/api/endpoints/0/no_declared_errors", json!(true)), Fails("Fixture endpoint GetNode: opts out of declared errors and also declares error entries")),
        // Success shapes (Converse: JSON + SSE + fallback; Submit: tagged JSON,
        // byte stream, 204 empty; Follow: NDJSON with a status).
        ("response control", Text(|t| format!("{t}\n")), Loads(|b| b.api().endpoints[5].response.len() == 3 && b.api().endpoints[6].response.len() == 4
            && b.api().endpoints[7].response[0].statuses == [200])),
        ("response absent", Remove("/api/endpoints/0/response"), Fails("missing field `response`")),
        ("response null", Set("/api/endpoints/0/response", Value::Null), Fails("/api/endpoints/0/response: explicit null")),
        ("response pre-v2 form", Set("/api/endpoints/0/response", json!({"Json": {"Named": "Node"}})), Fails("invalid type: map, expected a sequence")),
        ("response one wrong element", Push("/api/endpoints/5/response", json!(5)), Fails("invalid type: integer")),
        ("response every element wrong", Set("/api/endpoints/0/response", json!([5])), Fails("invalid type: integer")),
        ("response empty", Set("/api/endpoints/0/response", json!([])), Fails("Fixture endpoint GetNode: declares no success shape")),
        ("shape absent", Remove("/api/endpoints/5/response/0/shape"), Fails("missing field `shape`")),
        ("shape null", Set("/api/endpoints/5/response/0/shape", Value::Null), Fails("/api/endpoints/5/response/0/shape: explicit null")),
        ("shape wrong type", Set("/api/endpoints/5/response/0/shape", json!(5)), Fails("bundle JSON line 246 column 22")),
        ("shape empty", Set("/api/endpoints/5/response/0/shape", json!({})), Fails("line")),
        ("shape unknown variant", Set("/api/endpoints/5/response/0/shape", json!("Xml")), Fails("unknown variant `Xml`")),
        ("success unknown member", Set("/api/endpoints/5/response/0/extra", json!(1)), Fails("/api/endpoints/5/response/0/extra: unknown member")),
        ("content_type absent", Remove("/api/endpoints/6/response/2/content_type"), Loads(|b| b.api().endpoints[6].response[2].content_type.is_none())),
        ("content_type null", Set("/api/endpoints/6/response/2/content_type", Value::Null), Fails("/api/endpoints/6/response/2/content_type: explicit null")),
        ("content_type wrong type", Set("/api/endpoints/6/response/2/content_type", json!(5)), Fails("invalid type: integer")),
        ("content_type empty", Set("/api/endpoints/6/response/2/content_type", json!("")), Fails("Fixture endpoint Submit: success shape 3 (byte stream): media type `` is not a concrete")),
        ("content_type wildcard", Set("/api/endpoints/6/response/2/content_type", json!("audio/*")), Fails("wildcard ranges are not supported")),
        ("content_type shared", Set("/api/endpoints/6/response/2/content_type", json!("Application/JSON")), Fails("share media type `application/json`")),
        ("content_type on empty", Set("/api/endpoints/6/response/3/content_type", json!("text/plain")), Fails("success shape 4 (empty): an empty shape has no media type")),
        ("content_type on fallback", Set("/api/endpoints/5/response/2/content_type", json!("image/png")), Fails("the fallback declares no content type")),
        ("statuses absent", Remove("/api/endpoints/7/response/0/statuses"), Loads(|b| b.api().endpoints[7].response[0].statuses.is_empty())),
        ("statuses empty", Set("/api/endpoints/7/response/0/statuses", json!([])), Loads(|b| b.api().endpoints[7].response[0].statuses.is_empty())),
        ("statuses null", Set("/api/endpoints/7/response/0/statuses", Value::Null), Fails("/api/endpoints/7/response/0/statuses: explicit null")),
        ("statuses wrong type", Set("/api/endpoints/7/response/0/statuses", json!("200")), Fails("invalid type: string")),
        ("statuses one wrong element", Push("/api/endpoints/7/response/0/statuses", json!("201")), Fails("invalid type: string")),
        ("statuses every element wrong", Set("/api/endpoints/7/response/0/statuses", json!(["x"])), Fails("invalid type: string")),
        ("statuses not u16", Set("/api/endpoints/7/response/0/statuses", json!([70000])), Fails("u16")),
        ("statuses outside 2xx", Set("/api/endpoints/7/response/0/statuses", json!([300])), Fails("Fixture endpoint Follow: success shape 1 (NDJSON stream): declared status 300 is outside 200-299")),
        ("statuses duplicate", Set("/api/endpoints/7/response/0/statuses", json!([200, 200])), Fails("declares status 200 more than once")),
        ("statuses 204 on a body", Set("/api/endpoints/6/response/0/statuses", json!([204])), Fails("declares status 204, which selects the empty shape")),
        ("json_tag absent", Remove("/api/endpoints/6/response/1/json_tag"), Fails("no JSON tag separates them")),
        ("json_tag null", Set("/api/endpoints/6/response/1/json_tag", Value::Null), Fails("/api/endpoints/6/response/1/json_tag: explicit null")),
        ("json_tag wrong type", Set("/api/endpoints/6/response/1/json_tag", json!(5)), Fails("bundle JSON line 297 column 25")),
        ("json_tag empty", Set("/api/endpoints/6/response/1/json_tag", json!({})), Fails("line")),
        ("json_tag on a stream", Set("/api/endpoints/6/response/2/json_tag", json!("Array")), Fails("only a JSON shape takes a JSON tag")),
        ("tag name absent", Remove("/api/endpoints/6/response/0/json_tag/Field/name"), Fails("missing field `name`")),
        ("tag name wrong type", Set("/api/endpoints/6/response/0/json_tag/Field/name", json!(5)), Fails("invalid type: integer")),
        ("tag name empty", Set("/api/endpoints/6/response/0/json_tag/Field/name", json!("")), Fails("JSON tag field `` must be one nonempty top-level field name")),
        ("tag name a path", Set("/api/endpoints/6/response/0/json_tag/Field/name", json!("a.b")), Fails("must be one nonempty top-level field name")),
        ("tag value absent", Remove("/api/endpoints/6/response/0/json_tag/Field/value"), Fails("missing field `value`")),
        ("tag value null", Set("/api/endpoints/6/response/0/json_tag/Field/value", Value::Null), Fails("/api/endpoints/6/response/0/json_tag/Field/value: explicit null")),
        ("tag value object", Set("/api/endpoints/6/response/0/json_tag/Field/value", json!({})), Fails("value must be a string, number, or boolean")),
        ("tag value repeated", Set("/api/endpoints/6/response/0/json_tag/Field/value", json!("event")), Fails("no JSON tag separates them")),
        ("tag value empty string", Set("/api/endpoints/6/response/0/json_tag/Field/value", json!("")), Loads(|b| b.api().endpoints[6].response[0].json_tag
            == Some(schematic_define::JsonTag::Field { name: "kind".into(), value: json!("") }))),
        ("fallback absent", Remove("/api/endpoints/5/response/2/fallback"), Loads(|b| !b.api().endpoints[5].response[2].fallback)),
        ("fallback explicit false", Set("/api/endpoints/5/response/2/fallback", json!(false)), Loads(|b| !b.api().endpoints[5].response[2].fallback)),
        ("fallback null", Set("/api/endpoints/5/response/2/fallback", Value::Null), Fails("/api/endpoints/5/response/2/fallback: explicit null")),
        ("fallback wrong type", Set("/api/endpoints/5/response/2/fallback", json!("yes")), Fails("invalid type: string")),
        ("fallback on JSON", Set("/api/endpoints/5/response/0/fallback", json!(true)), Fails("only a text or binary shape may be the fallback")),
        ("note absent", Remove("/api/endpoints/5/response/0/note"), Loads(|b| b.api().endpoints[5].response[0].note.is_none())),
        ("note null", Set("/api/endpoints/5/response/0/note", Value::Null), Fails("/api/endpoints/5/response/0/note: explicit null")),
        ("note wrong type", Set("/api/endpoints/5/response/0/note", json!(5)), Fails("invalid type: integer")),
        ("note empty is data", Set("/api/endpoints/5/response/0/note", json!("")), Loads(|b| b.api().endpoints[5].response[0].note.as_deref() == Some(""))),
        ("variant absent", Remove("/api/endpoints/5/response/0/variant"), Loads(|b| b.api().endpoints[5].response[0].variant.is_none())),
        ("variant null", Set("/api/endpoints/5/response/0/variant", Value::Null), Fails("/api/endpoints/5/response/0/variant: explicit null")),
        ("variant wrong type", Set("/api/endpoints/5/response/0/variant", json!(5)), Fails("invalid type: integer")),
        ("variant empty", Set("/api/endpoints/5/response/0/variant", json!("")), Fails("variant name `` must be an UpperCamelCase Rust identifier")),
        ("variant repeated", Set("/api/endpoints/5/response/2/variant", json!("Whole")), Fails("both name the variant Whole")),
        ("variant on a single shape", Set("/api/endpoints/7/response/0/variant", json!("Lines")), Fails("names the variant Lines, but a single shape generates no enum")),
        ("framing absent", Remove("/api/endpoints/5/response/1/shape/Stream/framing"), Fails("missing field `framing`")),
        ("framing null", Set("/api/endpoints/5/response/1/shape/Stream/framing", Value::Null), Fails("/api/endpoints/5/response/1/shape/Stream/framing: explicit null")),
        ("framing wrong type", Set("/api/endpoints/5/response/1/shape/Stream/framing", json!(5)), Fails("bundle JSON line 260 column 28")),
        ("framing unknown variant", Set("/api/endpoints/5/response/1/shape/Stream/framing", json!("Xml")), Fails("unknown variant `Xml`")),
        ("sse item dangling", Set("/api/endpoints/5/response/1/shape/Stream/framing/Sse/Named", json!("Gone")), Fails("Fixture endpoint Converse: unresolved schema reference \"Gone\"")),
        ("ndjson item dangling", Set("/api/endpoints/7/response/0/shape/Stream/framing/Ndjson/Named", json!("Gone")), Fails("Fixture endpoint Follow: unresolved schema reference \"Gone\"")),
        ("sse item wrong type", Set("/api/endpoints/5/response/1/shape/Stream/framing/Sse", json!(5)), Fails("bundle JSON line 261 column 26")),
        ("end absent", Remove("/api/endpoints/5/response/1/shape/Stream/end"), Fails("missing field `end`")),
        ("end null", Set("/api/endpoints/5/response/1/shape/Stream/end", Value::Null), Fails("/api/endpoints/5/response/1/shape/Stream/end: explicit null")),
        ("end wrong type", Set("/api/endpoints/5/response/1/shape/Stream/end", json!(5)), Fails("bundle JSON line 257 column 24")),
        ("end unknown variant", Set("/api/endpoints/5/response/1/shape/Stream/end", json!("Never")), Fails("unknown variant `Never`")),
        ("event name empty", Set("/api/endpoints/5/response/1/shape/Stream/end/EventNamed", json!("")), Fails("an event-name terminator needs a nonempty event name")),
        ("event name wrong type", Set("/api/endpoints/5/response/1/shape/Stream/end/EventNamed", json!(5)), Fails("invalid type: integer")),
        ("data line empty is a terminator", Set("/api/endpoints/5/response/1/shape/Stream/end", json!({"DataLineEquals": ""})),
            Loads(|b| matches!(&b.api().endpoints[5].response[1].shape, schematic_define::ResponseShape::Stream { end: schematic_define::StreamEnd::DataLineEquals(data), .. } if data.is_empty()))),
        ("data line on ndjson", Set("/api/endpoints/7/response/0/shape/Stream/end", json!({"DataLineEquals": "[DONE]"})), Fails("a data-line terminator needs SSE framing")),
        ("end field absent", Remove("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/field"), Fails("missing field `field`")),
        ("end field empty", Set("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/field", json!("")), Fails("terminator field `` must be a dotted object path without array fan-out")),
        ("end field fan-out", Set("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/field", json!("items[0].kind")), Fails("must be a dotted object path without array fan-out")),
        ("end value absent", Remove("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/value"), Fails("missing field `value`")),
        ("end value null", Set("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/value", Value::Null), Fails("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/value: explicit null")),
        ("end value object", Set("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/value", json!({})), Fails("terminator field `kind` value must be a string, number, or boolean")),
        ("end value empty string", Set("/api/endpoints/7/response/0/shape/Stream/end/FieldEquals/value", json!("")),
            Loads(|b| matches!(&b.api().endpoints[7].response[0].shape, schematic_define::ResponseShape::Stream { end: schematic_define::StreamEnd::FieldEquals { value, .. }, .. } if *value == json!("")))),
        ("end field on bytes", Set("/api/endpoints/6/response/2/shape/Stream/end", json!({"FieldEquals": {"field": "kind", "value": "end"}})), Fails("a field terminator needs SSE or NDJSON framing")),
        ("keep absent", Remove("/api/endpoints/5/response/1/shape/Stream/keep"), Loads(|b| matches!(&b.api().endpoints[5].response[1].shape, schematic_define::ResponseShape::Stream { keep: false, .. }))),
        ("keep explicit false", Set("/api/endpoints/5/response/1/shape/Stream/keep", json!(false)), Loads(|b| matches!(&b.api().endpoints[5].response[1].shape, schematic_define::ResponseShape::Stream { keep: false, .. }))),
        ("keep null", Set("/api/endpoints/5/response/1/shape/Stream/keep", Value::Null), Fails("/api/endpoints/5/response/1/shape/Stream/keep: explicit null")),
        ("keep wrong type", Set("/api/endpoints/5/response/1/shape/Stream/keep", json!("yes")), Fails("invalid type: string")),
        ("keep at connection close", Set("/api/endpoints/6/response/2/shape/Stream/keep", json!(true)), Fails("`keep` needs a terminator")),
        ("duplicate success key", Text(|t| duplicate(t, "            \"note\": \"when stream is false\",")), Fails("duplicate key \"note\" at line")),
        // GraphQL surface and operations.
        ("graphql absent", Remove(GRAPHQL), Loads(|b| b.api().graphql.is_none())),
        ("graphql null", Set(GRAPHQL, Value::Null), Fails("/api/graphql: explicit null")),
        ("graphql wrong type", Set(GRAPHQL, json!([])), Fails("expected struct GraphQlSurface")),
        ("graphql empty", Set(GRAPHQL, json!({})), Fails("missing field `address`")),
        ("graphql unknown member", Set("/api/graphql/batching", json!(true)), Fails("/api/graphql/batching: unknown member")),
        ("address empty", Set("/api/graphql/address", json!([])), Fails("Fixture endpoint <api>: graphql: address needs at least one rule")),
        ("address null", Set("/api/graphql/address", Value::Null), Fails("/api/graphql/address: explicit null")),
        ("address one wrong element", Push("/api/graphql/address", json!(5)), Fails("bundle JSON line 389 column 9")),
        ("address every element wrong", Set("/api/graphql/address", json!(["/graphql"])), Fails("unknown variant `/graphql`")),
        ("address unknown rule", Set("/api/graphql/address/1", json!({"Prepend": "/x"})), Fails("unknown variant `Prepend`")),
        ("address suffix absent", Remove("/api/graphql/address/0/ReplaceSuffix/suffix"), Fails("missing field `suffix`")),
        ("address suffix empty", Set("/api/graphql/address/0/ReplaceSuffix/suffix", json!("")), Fails("address rule path `` must start with `/`")),
        ("address suffix wrong type", Set("/api/graphql/address/0/ReplaceSuffix/suffix", json!(3)), Fails("invalid type: integer")),
        ("address with relative", Set("/api/graphql/address/0/ReplaceSuffix/with", json!("api/graphql")), Fails("address rule path `api/graphql` must start with `/`")),
        ("address append query", Set("/api/graphql/address/1/Append", json!("/graphql?x=1")), Fails("address rule path `/graphql?x=1`")),
        ("address append empty", Set("/api/graphql/address/1/Append", json!("")), Fails("address rule path ``")),
        ("operations absent", Remove(OPERATIONS), Loads(|b| b.api().graphql.as_ref().is_some_and(|g| g.operations.is_empty()))),
        ("operations empty", Set(OPERATIONS, json!([])), Loads(|b| b.api().graphql.as_ref().is_some_and(|g| g.operations.is_empty()))),
        ("operations null", Set(OPERATIONS, Value::Null), Fails("/api/graphql/operations: explicit null")),
        ("operations wrong type", Set(OPERATIONS, json!({})), Fails("invalid type: map")),
        ("operations one wrong element", Push(OPERATIONS, json!(5)), Fails("invalid type: integer")),
        ("operations every element wrong", Set(OPERATIONS, json!([5])), Fails("invalid type: integer")),
        ("operation id absent", Remove("/api/graphql/operations/0/id"), Fails("missing field `id`")),
        ("operation id empty", Set("/api/graphql/operations/0/id", json!("")), Fails("Fixture operation : id and description must be nonempty")),
        ("operation id duplicate", Set("/api/graphql/operations/1/id", json!("Issues")), Fails("Fixture operation Issues: operation id is declared more than once")),
        ("operation id equals endpoint", Set("/api/graphql/operations/1/id", json!("Raw")), Fails("Fixture operation Raw: operation id equals an endpoint id")),
        ("operation kind unknown", Set("/api/graphql/operations/0/kind", json!("Subscription")), Fails("unknown variant `Subscription`")),
        ("operation kind null", Set("/api/graphql/operations/0/kind", Value::Null), Fails("/api/graphql/operations/0/kind: explicit null")),
        ("document empty is sent unchanged", Text(|t| format!(" {t}")), Loads(|b| b.api().graphql.as_ref().unwrap().operations[1].document.is_empty())),
        ("document unparsable", Set("/api/graphql/operations/0/document", json!("query {")), Loads(|b| b.api().graphql.as_ref().unwrap().operations[0].document == "query {")),
        ("document wrong type", Set("/api/graphql/operations/0/document", json!(["q"])), Fails("invalid type: sequence")),
        ("document absent", Remove("/api/graphql/operations/0/document"), Fails("missing field `document`")),
        ("variables unit", Set("/api/graphql/operations/0/variables", json!("Unit")), Fails("Fixture operation Issues: a paginated operation needs typed variables")),
        ("variables absent", Remove("/api/graphql/operations/1/variables"), Fails("missing field `variables`")),
        ("variables null", Set("/api/graphql/operations/1/variables", Value::Null), Fails("/api/graphql/operations/1/variables: explicit null")),
        ("variables empty descriptor", Set("/api/graphql/operations/1/variables", json!({})), Fails("expected value")),
        ("variables dangling", Set("/api/graphql/operations/0/variables/Typed/Named", json!("Gone")), Fails("Fixture operation Issues: unresolved schema reference \"Gone\"")),
        ("variables require a pager input", Set("/api/schemas/9/schema/required", json!(["owner", "after"])),
            Fails("Fixture operation Issues: pager-managed variable `after` must be optional in the variables type")),
        ("variables not an object", Set("/api/graphql/operations/0/variables/Typed", json!({"Array": {"Named": "IssuesVars"}})), Fails("variables must be a named type or map")),
        ("data absent", Remove("/api/graphql/operations/0/data"), Fails("missing field `data`")),
        ("data null", Set("/api/graphql/operations/0/data", Value::Null), Fails("/api/graphql/operations/0/data: explicit null")),
        ("data dangling", Set("/api/graphql/operations/0/data/Named", json!("Gone")), Fails("Fixture operation Issues: unresolved schema reference \"Gone\"")),
        ("data unregistered nested component", Set("/api/graphql/operations/0/data/Named", json!("Child")), Fails("Fixture operation Issues: unresolved schema reference \"Child\"")),
        ("operation_name absent", Remove("/api/graphql/operations/0/operation_name"), Loads(|b| b.api().graphql.as_ref().unwrap().operations[0].operation_name.is_none())),
        ("operation_name empty", Set("/api/graphql/operations/0/operation_name", json!("")), Loads(|b| b.api().graphql.as_ref().unwrap().operations[0].operation_name.as_deref() == Some(""))),
        ("operation_name null", Set("/api/graphql/operations/0/operation_name", Value::Null), Fails("/api/graphql/operations/0/operation_name: explicit null")),
        ("operation_name wrong type", Set("/api/graphql/operations/0/operation_name", json!(1)), Fails("invalid type: integer")),
        ("operation errors overlay", Text(|t| format!(" {t}")), Loads(|b| { let api = b.api(); let op = &api.graphql.as_ref().unwrap().operations[0];
            api.effective_operation_errors(op).lookup(403).is_some() && api.effective_operation_errors(op).lookup(503).is_some() })),
        ("operation errors null", Set("/api/graphql/operations/0/errors", Value::Null), Fails("/api/graphql/operations/0/errors: explicit null")),
        ("operation errors dangling", Set("/api/graphql/operations/0/errors/0/shape/Json/Named", json!("Gone")), Fails("Fixture operation Issues: unresolved schema reference \"Gone\"")),
        ("operation opt-out empties", Text(|t| format!(" {t}")), Loads(|b| { let api = b.api(); api.effective_operation_errors(&api.graphql.as_ref().unwrap().operations[1]).entries().is_empty() })),
        ("operation opt-out with entries", Set("/api/graphql/operations/0/no_declared_errors", json!(true)), Fails("Fixture operation Issues: opts out of declared errors and also declares error entries")),
        ("operation opt-out wrong type", Set("/api/graphql/operations/1/no_declared_errors", json!(1)), Fails("invalid type: integer")),
        ("operation pagination absent", Remove("/api/graphql/operations/0/pagination"), Loads(|b| b.api().graphql.as_ref().unwrap().operations[0].pagination.is_none())),
        ("operation pagination null", Set("/api/graphql/operations/0/pagination", Value::Null), Fails("/api/graphql/operations/0/pagination: explicit null")),
        ("operation pagination wrong type", Set("/api/graphql/operations/0/pagination", json!("relay")), Fails("invalid type: string")),
        ("operation pagination inherits", Set("/api/graphql/operations/0/pagination/pagination", json!({"policy": "inherit"})), Fails("Fixture operation Issues: pagination must declare its strategy")),
        ("operation pager input outside variables", Set("/api/graphql/operations/0/pagination/pagination/strategy/kind/forward/parts/0/param/name", json!(["after"])),
            Fails("Fixture operation Issues: pager input `after` must be a request-body path under `variables`")),
        ("operation pager input in query", Set("/api/graphql/operations/0/pagination/pagination/strategy/kind/size/param", json!({"in": "query", "name": "first"})),
            Fails("pager input `first` must be a request-body path under `variables`")),
        ("operation pagination item type absent", Remove("/api/graphql/operations/0/pagination/item_type"), Fails("Fixture operation Issues: pagination needs an item type")),
        ("duplicate graphql key", Text(|t| duplicate(t, "            \"suffix\": \"/api/v3\",")), Fails("duplicate key \"suffix\" at line")),
        ("duplicate operation key", Text(|t| duplicate(t, "          \"kind\": \"Query\",")), Fails("duplicate key \"kind\" at line")),
        ("duplicate opt-out key", Text(|t| duplicate(t, "        \"no_declared_errors\": true,")), Fails("duplicate key \"no_declared_errors\" at line")),
        // Document shapes.
        ("duplicate root key", Text(|t| duplicate(t, "  \"format_version\": 1,")), Fails("duplicate key \"format_version\" at line")),
        ("duplicate configuration key", Text(|t| duplicate(t, "    \"base_url\": \"https://api.example.com/v1\",")), Fails("duplicate key \"base_url\" at line")),
        ("duplicate OAuth2 key", Text(|t| duplicate(t, "            \"pkce\": \"Supported\",")), Fails("duplicate key \"pkce\" at line")),
        ("duplicate schema key", Text(|t| duplicate(t, "          \"title\": \"Node\",")), Fails("duplicate key \"title\" at line")),
        ("duplicate example key", Text(|t| duplicate(t, "              \"example\": \"item-1\",")), Fails("duplicate key \"example\" at line")),
        ("trailing garbage", Text(|t| format!("{t}x")), Fails("trailing characters at line")),
        ("second document", Text(|t| format!("{t}{{}}")), Fails("trailing characters at line")),
        ("comment", Text(|t| format!("// bundle\n{t}")), Fails("line 1 column 1")),
        ("trailing comma", Text(|t| t.replacen("\n}\n", ",\n}\n", 1)), Fails("trailing comma at line")),
        ("truncated", Text(|t| t[..t.len() / 2].to_owned()), Fails("EOF while parsing")),
        ("not an object", Text(|_| "[]".into()), Fails("bundle field /: a bundle is a JSON object")),
        ("empty document", Text(|_| " ".into()), Fails("EOF while parsing")),
    ]
}

fn oauth(bundle: &DefinitionBundle, index: usize) -> &schematic_define::OAuth2Config {
    match &bundle.api().auth_policy.as_ref().unwrap().explicit[index] {
        schematic_define::AuthMethod::OAuth2(config) => config,
        other => panic!("{other:?}"),
    }
}

/// One walk of the input matrix: each cell makes one edit to the canonical
/// fixture and observes the public load-and-export result. A loaded cell
/// must also export and round-trip; a failed cell returns no bundle.
#[test]
fn bundle_json_matrix() {
    let canonical = fixture_bundle().to_json();
    let mut failures = Vec::new();
    for (name, edit, expect) in cells() {
        let text = apply(&edit, &canonical);
        match (DefinitionBundle::from_json(&text), expect) {
            (Ok(bundle), Loads(check)) => {
                if !check(&bundle) { failures.push(format!("{name}: loaded without the expected consequence")); }
                if let Err(error) = openapi(bundle.api()) { failures.push(format!("{name}: export failed: {error}")); }
                if DefinitionBundle::from_json(&bundle.to_json()).as_ref() != Ok(&bundle) { failures.push(format!("{name}: does not round-trip")); }
            }
            (Ok(_), Fails(expected)) => failures.push(format!("{name}: loaded, expected an error containing {expected:?}")),
            (Err(error), Loads(_)) => failures.push(format!("{name}: failed: {error}")),
            (Err(error), Fails(expected)) => {
                if !error.to_string().contains(expected) { failures.push(format!("{name}: {error}\n    expected it to contain {expected:?}")); }
            }
        }
    }
    assert!(failures.is_empty(), "{} cell(s) failed:\n{}", failures.len(), failures.join("\n"));
}
