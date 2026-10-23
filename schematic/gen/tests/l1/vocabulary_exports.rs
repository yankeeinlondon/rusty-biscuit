//! Definition vocabulary in OpenAPI and Postman exports.
//!
//! Fixture APIs built with the public builders go through the real exporters
//! (`build_openapi_grouped`, `write_openapi`, the Postman builders and
//! writer) and validators; each test asserts the emitted JSON. The committed
//! artifacts of shipped APIs are held byte-identical to fresh output by
//! `artifact_drift`; GitHub, the one shipped API adopting rate-limit metadata,
//! is checked here in every committed output.

use schematic_define::openapi::ExportOptions;
use schematic_define::{
    ApiKeyLocation, ApiRequest, SuccessShape, AuthMethod, AuthPolicy, AuthStrategy, Backoff, DefinitionContext,
    DeviceCodeGrant, Endpoint, EndpointBuilder, EnvList, EnvMapping, OAuth2ClientAuthMethod, OAuth2Config, ParamDef,
    QueryParamType, RateLimit, ResetUnit, RestApi, RestApiBuilder, RetryGuidance, SchemaComponent, StatusSelector,
};
use schematic_define::builders::Present;
use schematic_gen::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use schematic_gen::errors::GeneratorError;
use schematic_gen::export::openapi::{build_openapi_grouped, write_openapi};
use schematic_gen::export::postman::{build_postman_collection, write_postman};
use schematic_gen::validation::{validate_openapi, validate_postman};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn get(id: &str, path: &str) -> EndpointBuilder<Present, Present> {
    Endpoint::get(id, path).describes(format!("{id} operation")).returns_empty()
}

fn delete(id: &str, path: &str) -> EndpointBuilder<Present, Present> {
    Endpoint::delete(id, path).describes(format!("{id} operation")).returns_empty()
}

fn builder<'a>(ctx: &'a DefinitionContext, name: &str) -> RestApiBuilder<'a, Present, Present> {
    RestApi::builder(ctx, name, "https://fixture.example").describes(format!("{name} fixture")).bearer_env(["FIXTURE_TOKEN"])
}

/// A bearer-authenticated API named `name` holding `endpoints`.
fn api(name: &str, endpoints: Vec<Endpoint>) -> RestApi {
    let ctx = DefinitionContext::types_in("::fixture");
    builder(&ctx, name).endpoints(endpoints).build().unwrap()
}

fn openapi(api: &RestApi) -> Value {
    serde_json::to_value(build_openapi_grouped(&[api], "fixture", &ExportOptions::new()).unwrap()).unwrap()
}

fn postman(api: &RestApi) -> Value {
    serde_json::to_value(build_postman_collection(api)).unwrap()
}

/// The request item named `name`, wherever its folder puts it.
fn request<'a>(collection: &'a Value, name: &str) -> &'a Value {
    fn find<'a>(items: &'a Value, name: &str) -> Option<&'a Value> {
        items.as_array()?.iter().find_map(|item| {
            if item["name"] == name && item.get("request").is_some() {
                Some(&item["request"])
            } else {
                find(&item["item"], name)
            }
        })
    }
    find(&collection["item"], name).unwrap_or_else(|| panic!("no request {name}"))
}

fn variable_keys(collection: &Value) -> Vec<&str> {
    collection["variable"].as_array().unwrap().iter().map(|var| var["key"].as_str().unwrap()).collect()
}

fn item_id(description: &str) -> ParamDef {
    ParamDef::path("item_id", QueryParamType::Integer).describes(description)
}

fn query_key() -> AuthMethod {
    AuthMethod::ApiKeyParam { name: "key".into(), location: ApiKeyLocation::Query }
}

fn explicit(methods: Vec<AuthMethod>) -> AuthPolicy {
    AuthPolicy { explicit: methods, env_fallback: None }
}

fn vocabulary_diagnostics(diagnostics: Vec<ValidationDiagnostic>) -> Vec<ValidationDiagnostic> {
    diagnostics
        .into_iter()
        .filter(|d| matches!(d.feature, Feature::EndpointAuth | Feature::SchemaComposition | Feature::OAuth2Grant | Feature::PathParameter))
        .collect()
}

fn requirement(name: &str) -> Value {
    json!({ name: [] })
}

// ---------------------------------------------------------------------------
// Typed path parameters
// ---------------------------------------------------------------------------

#[test]
fn shared_get_delete_path_declares_the_get_parameter_and_overrides_delete() {
    // DELETE is visited first; the common declaration still comes from GET.
    let fixture = api("Paths", vec![
        delete("DeleteItem", "/items/{item_id}").path_param(item_id("Item to delete").deprecated()).build().unwrap(),
        get("GetItem", "/items/{item_id}").path_param(item_id("Item to read")).build().unwrap(),
    ]);
    let item = &openapi(&fixture)["paths"]["/items/{item_id}"];

    assert_eq!(item["parameters"], json!([{
        "in": "path", "name": "item_id", "description": "Item to read", "required": true,
        "schema": {"type": "integer"}, "style": "simple",
    }]));
    assert!(item["get"].get("parameters").is_none());
    assert_eq!(item["delete"]["parameters"], json!([{
        "in": "path", "name": "item_id", "description": "Item to delete", "required": true, "deprecated": true,
        "schema": {"type": "integer"}, "style": "simple",
    }]));
}

#[test]
fn agreeing_path_declarations_live_on_the_path_item_only() {
    let fixture = api("Paths", vec![
        get("GetItem", "/items/{item_id}").path_param(item_id("Item")).build().unwrap(),
        delete("DeleteItem", "/items/{item_id}").path_param(item_id("Item")).build().unwrap(),
    ]);
    let item = &openapi(&fixture)["paths"]["/items/{item_id}"];
    assert_eq!(item["parameters"].as_array().unwrap().len(), 1);
    assert_eq!(item["parameters"][0]["schema"], json!({"type": "integer"}));
    assert!(item["get"].get("parameters").is_none() && item["delete"].get("parameters").is_none());
}

#[test]
fn repeated_and_reserved_placeholders_declare_each_name_once() {
    let fixture = api("Paths", vec![
        get("GetFile", "/repos/{owner}/{+path}/by/{owner}")
            .path_param(ParamDef::path("path", QueryParamType::String).describes("File path"))
            .path_param(ParamDef::path("owner", QueryParamType::String))
            .build().unwrap(),
    ]);
    let doc = openapi(&fixture);
    let params = &doc["paths"]["/repos/{owner}/{path}/by/{owner}"]["parameters"];
    let names: Vec<&str> = params.as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["owner", "path"]);
    assert!(!doc.to_string().contains("{+"));

    let collection = postman(&fixture);
    let url = &request(&collection, "GetFile")["url"];
    assert_eq!(url["raw"], "{{baseUrl}}/repos/:owner/:path/by/:owner");
    assert_eq!(url["variable"][1]["description"], "File path (string)");
}

#[test]
fn enum_path_parameter_lists_its_values_and_postman_describes_the_type() {
    let fixture = api("Paths", vec![
        get("GetPet", "/pets/{kind}/{id}")
            .path_param(ParamDef::path("kind", QueryParamType::Enum(vec!["cat".into(), "dog".into()])))
            .path_param(ParamDef::path("id", QueryParamType::Integer).describes("Pet id"))
            .build().unwrap(),
    ]);
    let params = &openapi(&fixture)["paths"]["/pets/{kind}/{id}"]["parameters"];
    assert_eq!(params[0]["schema"], json!({"type": "string", "enum": ["cat", "dog"]}));
    assert_eq!(params[1]["schema"], json!({"type": "integer"}));

    let collection = postman(&fixture);
    let variables = &request(&collection, "GetPet")["url"]["variable"];
    assert_eq!(variables[0], json!({"key": "kind", "value": "<kind>", "description": "one of: cat, dog"}));
    assert_eq!(variables[1], json!({"key": "id", "value": "<id>", "description": "Pet id (integer)"}));
}

// ---------------------------------------------------------------------------
// Deprecation
// ---------------------------------------------------------------------------

#[test]
fn deprecation_is_emitted_when_true_and_absent_when_false() {
    let deprecated = |name: &str| ParamDef::new(name, QueryParamType::String).deprecated();
    let fixture = api("Deprecation", vec![
        get("Old", "/old/{id}")
            .deprecated()
            .path_param(ParamDef::path("id", QueryParamType::String).deprecated())
            .query(deprecated("q"))
            .query(ParamDef::new("fresh", QueryParamType::String))
            .header_param(deprecated("X-Old"))
            .cookie(deprecated("session"))
            .build().unwrap(),
        get("New", "/new").query(ParamDef::new("fresh", QueryParamType::String)).build().unwrap(),
    ]);
    let doc = openapi(&fixture);
    let old = &doc["paths"]["/old/{id}"];
    assert_eq!(old["get"]["deprecated"], true);
    assert_eq!(old["parameters"][0]["deprecated"], true);
    let flags: Vec<(&str, &Value)> = old["get"]["parameters"].as_array().unwrap().iter()
        .map(|p| (p["name"].as_str().unwrap(), &p["deprecated"])).collect();
    assert_eq!(flags, [("q", &json!(true)), ("fresh", &Value::Null), ("X-Old", &json!(true)), ("session", &json!(true))]);
    let new = &doc["paths"]["/new"]["get"];
    assert!(new.get("deprecated").is_none() && new["parameters"][0].get("deprecated").is_none());

    let collection = postman(&fixture);
    assert_eq!(request(&collection, "Old")["description"],
        "Old operation\n\nDeprecated.\nDeprecated parameters: id, q, X-Old, session.");
    assert_eq!(request(&collection, "New")["description"], "New operation");
}

// ---------------------------------------------------------------------------
// Endpoint authentication
// ---------------------------------------------------------------------------

#[test]
fn public_endpoint_has_empty_security_and_postman_noauth() {
    let fixture = api("Public", vec![
        get("Health", "/health").public().build().unwrap(),
        get("Private", "/private").build().unwrap(),
    ]);
    let doc = openapi(&fixture);
    assert_eq!(doc["paths"]["/health"]["get"]["security"], json!([]));
    assert!(doc["paths"]["/private"]["get"].get("security").is_none());
    assert_eq!(doc["security"], json!([requirement("bearerAuth")]));

    let collection = postman(&fixture);
    assert_eq!(request(&collection, "Health")["auth"], json!({"type": "noauth"}));
    assert!(request(&collection, "Private").get("auth").is_none());
    assert_eq!(collection["auth"]["type"], "bearer");
}

#[test]
fn replacement_scheme_used_only_by_the_endpoint_is_registered_and_referenced() {
    let fixture = api("Replace", vec![get("Search", "/search").auth(explicit(vec![query_key()])).build().unwrap()]);
    let typed = build_openapi_grouped(&[&fixture], "fixture", &ExportOptions::new()).unwrap();
    let names: Vec<&String> = typed.components.as_ref().unwrap().security_schemes.keys().collect();
    assert_eq!(names, ["bearerAuth", "apiKeyAuth"], "the API's scheme first, then override-only ones");
    let doc = serde_json::to_value(&typed).unwrap();
    let schemes = &doc["components"]["securitySchemes"];
    assert_eq!(schemes["apiKeyAuth"]["in"], "query");
    assert_eq!(schemes["apiKeyAuth"]["name"], "key");
    assert_eq!(doc["paths"]["/search"]["get"]["security"], json!([requirement("apiKeyAuth")]));

    let collection = postman(&fixture);
    assert_eq!(request(&collection, "Search")["auth"], json!({"type": "apikey", "apikey": [
        {"key": "key", "value": "key"}, {"key": "value", "value": "{{apiKey}}"}, {"key": "in", "value": "query"},
    ]}));
    assert_eq!(variable_keys(&collection), ["baseUrl", "bearerToken", "apiKey"]);
}

#[test]
fn replacement_scheme_colliding_with_the_api_scheme_name_gets_a_suffix() {
    let ctx = DefinitionContext::types_in("::fixture");
    let fixture = RestApi::builder(&ctx, "Collide", "https://fixture.example").describes("Collide")
        .authentication(AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: None }, ["FIXTURE_KEY"])
        .endpoints([
            get("Search", "/search").auth(explicit(vec![query_key()])).build().unwrap(),
            // The same replacement again reuses the registered name.
            get("Find", "/find").auth(explicit(vec![query_key()])).build().unwrap(),
        ])
        .build().unwrap();
    let doc = openapi(&fixture);
    let schemes = &doc["components"]["securitySchemes"];
    assert_eq!(schemes["apiKeyAuth"]["in"], "header", "the API's scheme is never overwritten");
    assert_eq!(schemes["apiKeyAuth2"]["in"], "query");
    assert_eq!(schemes.as_object().unwrap().len(), 2);
    assert_eq!(doc["paths"]["/search"]["get"]["security"], json!([requirement("apiKeyAuth2")]));
    assert_eq!(doc["paths"]["/find"]["get"]["security"], json!([requirement("apiKeyAuth2")]));
}

#[test]
fn replacement_alternatives_are_separate_requirements_and_postman_warns() {
    let policy = explicit(vec![AuthMethod::Basic, query_key()]);
    let fixture = api("Alternatives", vec![get("Either", "/either").auth(policy).build().unwrap()]);
    let doc = openapi(&fixture);
    assert_eq!(doc["paths"]["/either"]["get"]["security"], json!([requirement("basicAuth"), requirement("apiKeyAuth")]));

    let collection = postman(&fixture);
    assert_eq!(request(&collection, "Either")["auth"]["type"], "basic");
    let warnings = vocabulary_diagnostics(validate_postman(&[&fixture]));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(
        (warnings[0].severity, warnings[0].feature, warnings[0].output, warnings[0].api.as_slice(), warnings[0].endpoint.as_deref()),
        (Severity::Warning, Feature::EndpointAuth, Output::Postman, &["Alternatives".to_string()][..], Some("Either"))
    );
    assert!(vocabulary_diagnostics(validate_openapi(&[&fixture])).is_empty());

    let dir = tempfile::TempDir::new().unwrap();
    assert!(write_postman(&fixture, dir.path(), false).unwrap().exists(), "a warning never blocks the output");
}

#[test]
fn environment_variable_names_never_become_exported_values() {
    let own = EnvMapping { bearer_token: Some(EnvList::single("ENDPOINT_ONLY_TOKEN")), ..Default::default() };
    let bearer = AuthPolicy::from_auth_strategy(&AuthStrategy::BearerToken { header: None });
    let fixture = api("Secrets", vec![get("Admin", "/admin").auth_with_env_mapping(bearer, own).build().unwrap()]);

    let doc = openapi(&fixture);
    assert!(!doc.to_string().contains("ENDPOINT_ONLY_TOKEN"));
    assert_eq!(doc["paths"]["/admin"]["get"]["security"], json!([requirement("bearerAuth")]));

    let collection = postman(&fixture);
    assert!(!collection.to_string().contains("ENDPOINT_ONLY_TOKEN"));
    assert!(!collection.to_string().contains("FIXTURE_TOKEN"));
    assert_eq!(request(&collection, "Admin")["auth"]["bearer"][0]["value"], "{{bearerToken_Admin}}");
    let declared = collection["variable"].as_array().unwrap().iter().find(|v| v["key"] == "bearerToken_Admin").unwrap();
    assert_eq!(declared["value"], "");
}

// ---------------------------------------------------------------------------
// Group merge
// ---------------------------------------------------------------------------

fn grouped(apis: &[&RestApi]) -> Result<Value, GeneratorError> {
    build_openapi_grouped(apis, "grouped", &ExportOptions::new()).map(|doc| serde_json::to_value(doc).unwrap())
}

#[test]
fn group_merge_keeps_a_public_side_as_an_anonymous_alternative() {
    let ctx = DefinitionContext::types_in("::fixture");
    let public = builder(&ctx, "Open").module_path("grouped")
        .endpoints([get("List", "/items").public().build().unwrap()]).build().unwrap();
    let basic = RestApi::builder(&ctx, "Closed", "https://fixture.example").describes("Closed fixture")
        .basic_env("FIXTURE_USER", ["FIXTURE_PASS"]).module_path("grouped")
        .endpoints([get("List", "/items").build().unwrap()]).build().unwrap();
    let both_public = builder(&ctx, "AlsoOpen").module_path("grouped")
        .endpoints([get("List", "/items").public().build().unwrap()]).build().unwrap();

    let merged = grouped(&[&public, &basic]).unwrap();
    assert_eq!(merged["paths"]["/items"]["get"]["security"], json!([{}, requirement("basicAuth")]));
    let reversed = grouped(&[&basic, &public]).unwrap();
    assert_eq!(reversed["paths"]["/items"]["get"]["security"], json!([requirement("basicAuth"), {}]));
    let open = grouped(&[&public, &both_public]).unwrap();
    assert_eq!(open["paths"]["/items"]["get"]["security"], json!([]));
}

#[test]
fn group_merge_renames_schemes_without_overwriting_an_earlier_rename() {
    let ctx = DefinitionContext::types_in("::fixture");
    let header_bearer = |name: &str, header: &str| {
        RestApi::builder(&ctx, name, "https://fixture.example").describes("fixture")
            .authentication(AuthStrategy::BearerToken { header: Some(header.into()) }, ["FIXTURE_TOKEN"])
            .module_path("grouped").endpoints([get(&format!("Get{}", name.len()), &format!("/{header}")).build().unwrap()])
            .build().unwrap()
    };
    let first = builder(&ctx, "First").module_path("grouped").endpoints([get("Get", "/a").build().unwrap()]).build().unwrap();
    // `Second` and `SECOND` both rename to `bearerAuth_second`.
    let second = header_bearer("Second", "X-B");
    let third = header_bearer("SECOND", "X-C");

    let merged = grouped(&[&first, &second, &third]).unwrap();
    let schemes = &merged["components"]["securitySchemes"];
    assert_eq!(schemes["bearerAuth_second"]["name"], "X-B");
    assert_eq!(schemes["bearerAuth_second2"]["name"], "X-C");
    assert_eq!(merged["security"], json!([requirement("bearerAuth"), requirement("bearerAuth_second"), requirement("bearerAuth_second2")]));
}

#[test]
fn group_merge_overrides_a_later_differing_path_declaration() {
    let ctx = DefinitionContext::types_in("::fixture");
    let first = builder(&ctx, "First").module_path("grouped")
        .endpoints([get("GetItem", "/items/{item_id}").path_param(item_id("First")).build().unwrap()]).build().unwrap();
    let second = builder(&ctx, "Second").module_path("grouped")
        .endpoints([delete("DeleteItem", "/items/{item_id}").path_param(item_id("Second")).build().unwrap()]).build().unwrap();
    let merged = grouped(&[&first, &second]).unwrap();
    let item = &merged["paths"]["/items/{item_id}"];
    assert_eq!(item["parameters"].as_array().unwrap().len(), 1);
    assert_eq!(item["parameters"][0]["description"], "First");
    assert_eq!(item["delete"]["parameters"][0]["description"], "Second");
}

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

fn component(name: &str, schema: Value) -> SchemaComponent {
    SchemaComponent { name: name.into(), rust_path: Some(format!("::fixture::{name}")), schema }
}

fn reference(name: &str) -> Value {
    json!({"$ref": format!("#/components/schemas/{name}")})
}

/// An API whose `Adopt` request is the composed `Pet` and response the
/// composed `Both`, stored in the form `DefinitionContext::one_of`/`all_of`
/// produce (this crate's tests have no `schemars` to register types).
fn composed_api(name: &str, pet: Value) -> RestApi {
    let mut fixture = api(name, vec![
        Endpoint::post("Adopt", "/pets").describes("Adopt").returns_empty().build().unwrap(),
    ]);
    fixture.module_path = Some("grouped".into());
    fixture.endpoints[0].request = Some(ApiRequest::json_type("Pet"));
    fixture.endpoints[0].response = vec![SuccessShape::json_type("Both")];
    fixture.schemas = vec![
        component("Cat", json!({"type": "object", "properties": {"meows": {"type": "boolean"}}})),
        component("Dog", json!({"type": "object", "properties": {"barks": {"type": "integer"}}})),
        component("Pet", pet),
        component("Both", json!({"allOf": [reference("Cat"), reference("Dog")]})),
    ];
    fixture
}

#[test]
fn composed_components_export_their_references() {
    let one_of = json!({"oneOf": [reference("Cat"), reference("Dog")]});
    let any_of = json!({"anyOf": [reference("Cat"), reference("Dog")]});
    let doc = openapi(&composed_api("Pets", one_of.clone()));
    assert_eq!(doc["components"]["schemas"]["Pet"], one_of);
    assert_eq!(doc["components"]["schemas"]["Both"], json!({"allOf": [reference("Cat"), reference("Dog")]}));
    let doc = openapi(&composed_api("Pets", any_of.clone()));
    assert_eq!(doc["components"]["schemas"]["Pet"], any_of);
}

#[test]
fn group_merge_reports_a_conflicting_same_name_composed_component() {
    let composed = composed_api("Composed", json!({"oneOf": [reference("Cat"), reference("Dog")]}));
    let mut plain = composed_api("Plain", json!({"type": "object", "properties": {"name": {"type": "string"}}}));
    plain.endpoints[0].path = "/other".into();
    let Err(GeneratorError::Validation { diagnostics, .. }) = grouped(&[&composed, &plain]) else {
        panic!("a conflicting component must not merge");
    };
    assert_eq!(diagnostics[0].feature, Feature::InvalidSchemaReference);
    assert_eq!(diagnostics[0].api, ["Composed", "Plain"]);
    assert!(diagnostics[0].message.contains("\"Pet\""), "{}", diagnostics[0].message);
}

#[test]
fn postman_warns_about_each_composed_component_and_still_writes_the_collection() {
    let fixture = composed_api("Pets", json!({"oneOf": [reference("Cat"), reference("Dog")]}));
    let warnings = vocabulary_diagnostics(validate_postman(&[&fixture]));
    let summary: Vec<(Severity, Feature, Output, Option<&str>, bool)> = warnings.iter()
        .map(|d| (d.severity, d.feature, d.output, d.endpoint.as_deref(), d.api == ["Pets"])).collect();
    let expected = (Severity::Warning, Feature::SchemaComposition, Output::Postman, Some("Adopt"), true);
    assert_eq!(summary, [expected, expected]);
    assert!(warnings[0].message.contains("request body uses component \"Pet\", a oneOf composition"), "{}", warnings[0].message);
    assert!(warnings[1].message.contains("response uses component \"Both\", a allOf composition"), "{}", warnings[1].message);
    assert!(vocabulary_diagnostics(validate_openapi(&[&fixture])).is_empty());

    // The existing placeholder example is kept; nothing is invented.
    let collection = postman(&fixture);
    assert_eq!(request(&collection, "Adopt")["body"]["raw"], "{}");
    let dir = tempfile::TempDir::new().unwrap();
    assert!(write_postman(&fixture, dir.path(), false).unwrap().exists());
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

#[test]
fn metadata_keeps_every_field_at_api_and_endpoint_level() {
    let api_limit = RateLimit::new("x-ratelimit-limit", "x-ratelimit-remaining", "x-ratelimit-reset", ResetUnit::EpochSeconds)
        .unwrap().with_retry_after_header("retry-after").unwrap();
    let api_retry = RetryGuidance::new([StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 599 }], true, 3,
        Backoff::Exponential { base_ms: 250, cap_ms: 8000 }).unwrap();
    let own_limit = RateLimit::new("x-search-limit", "x-search-remaining", "x-search-reset", ResetUnit::Seconds).unwrap();
    let own_retry = RetryGuidance::new([], false, 1, Backoff::Fixed { delay_ms: 500 }).unwrap();
    let ctx = DefinitionContext::types_in("::fixture");
    let fixture = builder(&ctx, "Meta").rate_limit(api_limit).retry_guidance(api_retry)
        .endpoints([
            get("Inherit", "/inherit").build().unwrap(),
            get("Search", "/search").rate_limit(own_limit).retry_guidance(own_retry).build().unwrap(),
        ])
        .build().unwrap();

    let doc = openapi(&fixture);
    assert_eq!(doc["x-schematic"]["rate_limit"], json!({
        "limit_header": "x-ratelimit-limit", "remaining_header": "x-ratelimit-remaining",
        "reset_header": "x-ratelimit-reset", "retry_after_header": "retry-after", "reset_unit": "epoch_seconds",
    }));
    assert_eq!(doc["x-schematic"]["retry_guidance"], json!({
        "retry_on": [{"code": 429}, {"range": {"start": 500, "end": 599}}], "honor_retry_after": true,
        "max_attempts": 3, "backoff": {"exponential": {"base_ms": 250, "cap_ms": 8000}},
    }));
    let inherit = &doc["paths"]["/inherit"]["get"]["x-schematic"];
    assert!(inherit.get("rate_limit").is_none() && inherit.get("retry_guidance").is_none());
    let search = &doc["paths"]["/search"]["get"]["x-schematic"];
    assert_eq!(search["rate_limit"], json!({
        "limit_header": "x-search-limit", "remaining_header": "x-search-remaining",
        "reset_header": "x-search-reset", "reset_unit": "seconds",
    }));
    assert_eq!(search["retry_guidance"], json!({
        "retry_on": [], "honor_retry_after": false, "max_attempts": 1, "backoff": {"fixed": {"delay_ms": 500}},
    }));
    let bare = serde_json::to_value(build_openapi_grouped(&[&fixture], "fixture", &ExportOptions::new().skip_extensions()).unwrap()).unwrap();
    assert!(!bare.to_string().contains("x-ratelimit"));

    let collection = postman(&fixture);
    assert_eq!(request(&collection, "Inherit")["description"], "Inherit operation\n\n\
        Rate limit headers: limit `x-ratelimit-limit`, remaining `x-ratelimit-remaining`, reset `x-ratelimit-reset` (epoch seconds), retry after `retry-after`.\n\
        Retry guidance: retry on 429, 500-599; honor Retry-After: yes; max attempts 3 (including the first); backoff exponential from 250 ms up to 8000 ms.");
    assert_eq!(request(&collection, "Search")["description"], "Search operation\n\n\
        Rate limit headers: limit `x-search-limit`, remaining `x-search-remaining`, reset `x-search-reset` (seconds).\n\
        Retry guidance: retry on none; honor Retry-After: no; max attempts 1 (including the first); backoff fixed 500 ms.");
}

// ---------------------------------------------------------------------------
// OAuth2 device code
// ---------------------------------------------------------------------------

fn device_code() -> OAuth2Config {
    OAuth2Config::DeviceCode(DeviceCodeGrant {
        device_authorization_url: "https://auth.fixture.example/device".into(),
        token_url: "https://auth.fixture.example/token".into(),
        revocation_url: None,
        default_scopes: vec!["read".into()],
        client_auth: OAuth2ClientAuthMethod::None,
    })
}

#[test]
fn openapi_warns_about_device_code_and_still_writes_the_document() {
    let ctx = DefinitionContext::types_in("::fixture");
    let fixture = RestApi::builder(&ctx, "Device", "https://fixture.example").describes("Device fixture")
        .authentication(AuthStrategy::OAuth2(device_code()), Vec::<String>::new())
        .endpoints([
            get("Poll", "/poll").build().unwrap(),
            get("Mixed", "/mixed").auth(explicit(vec![AuthMethod::OAuth2(device_code()), query_key()])).build().unwrap(),
        ])
        .build().unwrap();

    let warnings = vocabulary_diagnostics(validate_openapi(&[&fixture]));
    let scopes: Vec<(Severity, Feature, Output, Option<&str>)> =
        warnings.iter().map(|d| (d.severity, d.feature, d.output, d.endpoint.as_deref())).collect();
    assert_eq!(scopes, [
        (Severity::Warning, Feature::OAuth2Grant, Output::OpenApi, None),
        (Severity::Warning, Feature::OAuth2Grant, Output::OpenApi, Some("Mixed")),
    ]);

    let dir = tempfile::TempDir::new().unwrap();
    let path = write_openapi(&fixture, &ExportOptions::new(), dir.path()).unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(doc.get("security").is_none(), "no requirement names a scheme that does not exist");
    let schemes: Vec<&String> = doc["components"]["securitySchemes"].as_object().unwrap().keys().collect();
    assert_eq!(schemes, ["apiKeyAuth"], "only the endpoint's expressible alternative");
    assert_eq!(doc["paths"]["/mixed"]["get"]["security"], json!([requirement("apiKeyAuth")]));
}

// ---------------------------------------------------------------------------
// Control
// ---------------------------------------------------------------------------

#[test]
fn an_api_adopting_no_vocabulary_exports_as_before() {
    let fixture = api("Plain", vec![
        get("GetItem", "/items/{item_id}").query(ParamDef::new("q", QueryParamType::String)).build().unwrap(),
        delete("DeleteItem", "/items/{item_id}").build().unwrap(),
    ]);
    let doc = openapi(&fixture);
    let item = &doc["paths"]["/items/{item_id}"];
    assert_eq!(item["parameters"], json!([{
        "in": "path", "name": "item_id", "required": true, "schema": {"type": "string"}, "style": "simple",
    }]));
    for method in ["get", "delete"] {
        let op = &item[method];
        assert!(op.get("security").is_none() && op.get("deprecated").is_none(), "{method}");
        assert_eq!(op["x-schematic"], json!({"response": "Empty"}));
    }
    assert!(item["get"]["parameters"][0].get("deprecated").is_none());
    assert_eq!(doc["components"]["securitySchemes"].as_object().unwrap().len(), 1);
    assert!(doc["x-schematic"].get("rate_limit").is_none());

    let collection = postman(&fixture);
    let get_item = request(&collection, "GetItem");
    assert!(get_item.get("auth").is_none());
    assert_eq!(get_item["description"], "GetItem operation");
    assert_eq!(get_item["url"]["variable"], json!([{"key": "item_id", "value": "<item_id>"}]));
    assert_eq!(variable_keys(&collection), ["baseUrl", "bearerToken"]);

    assert!(vocabulary_diagnostics(validate_openapi(&[&fixture])).is_empty());
    assert!(vocabulary_diagnostics(validate_postman(&[&fixture])).is_empty());
}

/// GitHub's documented rate-limit headers reach the OpenAPI extension, every
/// Postman request, and the generated module and request docs. The committed
/// artifacts are read because `artifact_drift` proves them equal to fresh
/// output.
#[test]
fn github_rate_limit_adoption_reaches_every_output() {
    let openapi: Value = serde_json::from_str(include_str!("../../../openapi/github.json")).unwrap();
    assert_eq!(
        openapi["x-schematic"]["rate_limit"],
        json!({
            "limit_header": "x-ratelimit-limit",
            "remaining_header": "x-ratelimit-remaining",
            "reset_header": "x-ratelimit-reset",
            "reset_unit": "epoch_seconds",
            "retry_after_header": "retry-after"
        })
    );
    assert!(openapi["x-schematic"].get("retry_guidance").is_none(), "GitHub declares no retry guidance");

    let line = "Rate limit headers: limit `x-ratelimit-limit`, remaining `x-ratelimit-remaining`, \
                reset `x-ratelimit-reset` (epoch seconds), retry after `retry-after`.";
    let postman: Value = serde_json::from_str(include_str!("../../../postman/github.postman_collection.json")).unwrap();
    let mut requests = 0;
    let mut stack = vec![&postman["item"]];
    while let Some(items) = stack.pop() {
        for item in items.as_array().unwrap() {
            if let Some(request) = item.get("request") {
                requests += 1;
                let description = request["description"].as_str().unwrap();
                assert!(description.ends_with(line), "{}: {description}", item["name"]);
            } else {
                stack.push(&item["item"]);
            }
        }
    }
    assert_eq!(requests, 16);

    let module = include_str!("../../../schema/src/github/mod.rs");
    let requests_rs = include_str!("../../../schema/src/github/requests.rs");
    for header in ["`x-ratelimit-limit`", "`x-ratelimit-remaining`", "`x-ratelimit-reset`, as Unix time in seconds", "`retry-after`"] {
        assert!(module.contains(header), "module docs miss {header}");
    }
    assert_eq!(requests_rs.matches("/// ## Rate limit").count(), 16);
    assert!(!module.contains("## Retry guidance") && !requests_rs.contains("## Retry guidance"));
}
