use schematic_define::{ApiKeyEnv, ApiRequest, ApiResponse, AuthMethod, AuthPolicy, AuthStrategy,
    DefinitionContext, Endpoint, EndpointParams, EnvAuthStrategy, EnvList, EnvMapping, ErrorSet,
    DefinitionError, FormField, FormFieldKind, RestApi, RestMethod, SchemaComponent, Success, TypeRef, definition_type};
use schemars::JsonSchema;
use serde_json::json;

definition_type!(#[derive(JsonSchema)] pub struct Model { pub id: String });
definition_type!(#[derive(JsonSchema)] #[schemars(rename = "Model")] pub struct ClashA { pub a: bool });
definition_type!(#[derive(JsonSchema)] #[schemars(rename = "Model")] pub struct ClashB { pub b: i64 });

fn api(ctx: &DefinitionContext, ep: Endpoint) -> Result<RestApi, schematic_define::DefinitionError> {
    RestApi::builder(ctx, "Fixture", "https://example.com").describes("Fixture API")
        .no_auth().endpoints([ep]).build()
}

#[test]
fn every_verb_preserves_required_facts_and_empty_parameters() {
    let endpoints = [Endpoint::get("Get", "/items"), Endpoint::post("Post", "/items"),
        Endpoint::put("Put", "/items"), Endpoint::patch("Patch", "/items"),
        Endpoint::delete("Delete", "/items"), Endpoint::head("Head", "/items"),
        Endpoint::options("Options", "/items")];
    let methods = [RestMethod::Get, RestMethod::Post, RestMethod::Put, RestMethod::Patch,
        RestMethod::Delete, RestMethod::Head, RestMethod::Options];
    for (builder, method) in endpoints.into_iter().zip(methods) {
        let ep = builder.returns_empty().describes("An operation").errors(ErrorSet::none()).build().unwrap();
        assert_eq!(ep.method, method);
        assert_eq!(ep.path, "/items");
        assert_eq!(ep.description, "An operation");
        assert_eq!(ep.response, ApiResponse::Empty);
        assert!(ep.request.is_none() && ep.params.is_none() && ep.headers.is_empty());
    }
    assert_eq!(EndpointParams::new(), EndpointParams::default());
}

#[test]
fn accepted_successes_and_requests_preserve_stored_wire_shapes() {
    let mut ctx = DefinitionContext::types_in("::models");
    let desc = ctx.register::<Model>().unwrap();
    for (success, expected) in [(Success::json(desc.clone()), ApiResponse::Json(TypeRef::Named("Model".into()))),
        (Success::text(), ApiResponse::Text), (Success::binary(), ApiResponse::Binary),
        (Success::empty(), ApiResponse::Empty)] {
        let ep = Endpoint::post("Operation", "/items").describes("Operation").returns(success).build().unwrap();
        assert_eq!(ep.response, expected);
        assert_eq!(api(&ctx, ep.clone()).unwrap().endpoints, [ep]);
    }
    let json = Endpoint::post("Json", "/items").describes("JSON").request_json::<Model>(&mut ctx)
        .returns_json::<Vec<Model>>(&mut ctx).build().unwrap();
    assert_eq!(json.request, Some(ApiRequest::Json(desc.type_ref().clone())));
    assert_eq!(json.response, ApiResponse::Json(TypeRef::Array(Box::new(desc.type_ref().clone()))));
    assert_eq!(api(&ctx, json.clone()).unwrap().endpoints, [json]);
    for (ep, request, response) in [
        (Endpoint::post("Text", "/").describes("Text").request_text().returns_text().build().unwrap(), ApiRequest::Text { content_type: "text/plain".into() }, ApiResponse::Text),
        (Endpoint::post("Binary", "/").describes("Binary").request_binary().returns_binary().build().unwrap(), ApiRequest::Binary { content_type: "application/octet-stream".into() }, ApiResponse::Binary),
    ] {
        assert_eq!(ep.request, Some(request));
        assert_eq!(ep.response, response);
        api(&ctx, ep).unwrap();
    }
    let fields = vec![FormField::text("caption").optional().with_description("Caption"),
        FormField::file_accept("image", vec!["image/png".into()]),
        FormField { name: "metadata".into(), required: true, description: Some("Metadata".into()),
            kind: FormFieldKind::Json(desc.into_type_ref()) }];
    for ep in [Endpoint::post("Multipart", "/").describes("Upload").request_form(fields.clone()).returns_empty().build().unwrap(),
        Endpoint::post("Encoded", "/").describes("Submit").request_urlencoded(fields.clone()).returns_empty().build().unwrap()] {
        match &ep.request { Some(ApiRequest::FormData { fields: actual } | ApiRequest::UrlEncoded { fields: actual }) => assert_eq!(actual, &fields), _ => panic!("form shape lost") }
        api(&ctx, ep).unwrap();
    }
}

#[test]
fn endpoint_builder_equals_legacy_literal_without_normalization() {
    let built = Endpoint::get("Get", "/items").describes("Get items").header("Accept", "text/plain")
        .params(EndpointParams::new()).returns_text().build().unwrap();
    let literal = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Get".into(); fixture.method = RestMethod::Get; fixture.path = "/items".into(); fixture.description = "Get items".into(); fixture.request = None; fixture.response = ApiResponse::Text; fixture.headers = vec![("Accept".into(), "text/plain".into())]; fixture.params = Some(EndpointParams::default()); fixture.cross_origin = None; fixture.pagination = None; fixture };
    assert_eq!(built, literal);
    assert_eq!(serde_json::to_value(built).unwrap(), serde_json::to_value(literal).unwrap());
}

#[test]
fn repeated_success_and_invalid_required_facts_report_errors() {
    let error = Endpoint::get("Get", "/").describes("Get").returns_empty().returns_text().build().unwrap_err().to_string();
    assert!(error.contains("Get") && error.contains("several success shapes arrive with response model v2"));
    for (id, path, description) in [("", "/", "Get"), ("Get", "items", "Get"), ("Get", "/", "  ")] {
        assert!(Endpoint::get(id, path).describes(description).returns_empty().build().is_err());
    }
    let ctx = DefinitionContext::types_in("::models");
    for (name, url, description) in [("", "https://example.com", "API"), ("API", "bad-url", "API"),
        ("API", "file:///tmp/api", "API"), ("API", "https://example.com", " ")] {
        assert!(RestApi::builder(&ctx, name, url).describes(description).no_auth().build().is_err());
    }
    assert!(RestApi::builder(&ctx, "API", "https://example.com").describes("API").no_auth().bearer_env(["KEY"]).build().unwrap_err().to_string().contains("exactly once"));
    let mut failed_registration = DefinitionContext::types_in("::models");
    let error = Endpoint::get("Unsupported", "/").describes("Unsupported")
        .returns_json::<std::collections::HashMap<bool, Model>>(&mut failed_registration)
        .returns_empty().build().unwrap_err().to_string();
    assert!(error.contains("Unsupported") && error.contains("unsupported map key bool"));
    assert!(failed_registration.registrations().is_empty());
    let error = Endpoint::post("UnsupportedRequest", "/").describes("Unsupported")
        .request_json::<std::collections::HashMap<bool, Model>>(&mut failed_registration)
        .request_text().returns_empty().build().unwrap_err().to_string();
    assert!(error.contains("UnsupportedRequest") && error.contains("unsupported map key bool"));
}

#[test]
fn auth_forms_preserve_fields_and_explicit_mapping_conflicts_in_both_orders() {
    let ctx = DefinitionContext::types_in("::models");
    let basic = RestApi::builder(&ctx, "API", "https://example.com").describes("API").basic_env("USER", ["PASS", "FALLBACK"]).build().unwrap();
    assert_eq!(basic.auth, AuthStrategy::Basic);
    assert_eq!(basic.env_username.as_deref(), Some("USER"));
    assert_eq!(basic.env_auth, ["PASS", "FALLBACK"]);
    let bearer = RestApi::builder(&ctx, "API", "https://example.com").describes("API").bearer_env(["OPENAI_API_KEY"]).build().unwrap();
    assert_eq!(bearer.auth, AuthStrategy::BearerToken { header: None });
    assert_eq!(bearer.env_auth, ["OPENAI_API_KEY"]);
    let key = RestApi::builder(&ctx, "API", "https://example.com").describes("API")
        .authentication(AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: Some("token ".into()) }, ["KEY"]).build().unwrap();
    assert_eq!(key.env_auth, ["KEY"]);
    assert!(matches!(key.auth, AuthStrategy::ApiKey { header, value_prefix: Some(prefix) } if header == "X-Key" && prefix == "token "));
    let none = RestApi::builder(&ctx, "API", "https://example.com").describes("API").no_auth().build().unwrap();
    assert_eq!(none.auth, AuthStrategy::None);
    assert!(none.env_auth.is_empty() && none.env_username.is_none());
    let conflict = EnvMapping { bearer_token: Some(EnvList::single("WRONG_KEY")), ..EnvMapping::default() };
    for result in [RestApi::builder(&ctx, "API", "https://example.com").describes("API").env_mapping(conflict.clone()).bearer_env(["OPENAI_API_KEY"]).build(),
        RestApi::builder(&ctx, "API", "https://example.com").describes("API").bearer_env(["OPENAI_API_KEY"]).env_mapping(conflict).build()] {
        assert!(result.unwrap_err().to_string().contains("conflicts"));
    }
}

#[test]
fn explicit_multi_method_policy_preserves_authoritative_fallback() {
    let ctx = DefinitionContext::types_in("::models");
    let policy = AuthPolicy { explicit: vec![AuthMethod::Basic, AuthMethod::BearerToken { header: None }], env_fallback: Some(EnvAuthStrategy::Basic) };
    let mapping = EnvMapping { basic_user: Some(EnvList::single("USER")), basic_pass: Some(EnvList::single("PASS")),
        bearer_token: Some(EnvList::single("TOKEN")), api_key: Some(ApiKeyEnv { names: EnvList::single("KEY"), header: "X-Key".into() }), ..EnvMapping::default() };
    let built = RestApi::builder(&ctx, "API", "https://example.com").describes("API").no_auth()
        .auth_policy(policy.clone()).env_mapping(mapping.clone()).build().unwrap();
    assert_eq!(built.auth_policy, Some(policy));
    assert_eq!(built.env_mapping, Some(mapping));
}

#[test]
fn references_resolve_through_the_destination_context_and_survive_transfer() {
    let mut source = DefinitionContext::types_in("::models");
    let desc = source.register::<Option<Vec<Model>>>().unwrap();
    let ep = Endpoint::get("GetModels", "/models").describes("Models").returns(Success::json(desc.clone())).build().unwrap();
    let mut destination = DefinitionContext::types_in("::models");
    let error = api(&destination, ep.clone()).unwrap_err();
    assert_eq!(error, DefinitionError::UnresolvedReference { api: "Fixture".into(), endpoint: "GetModels".into(), name: "Model".into() });
    assert_eq!(error.diagnostics("Fixture"), "Fixture endpoint GetModels: unresolved schema reference \"Model\"");
    // The wrapper shape lives in the reference, so only the leaf must be registered.
    destination.register::<Model>().unwrap();
    let built = api(&destination, ep.clone()).unwrap();
    assert_eq!(built.endpoints, std::slice::from_ref(&ep));
    assert_eq!(built.rust_type(desc.type_ref()).as_deref(), Some("::std::option::Option<::std::vec::Vec<::models::Model>>"));
    // The destination, not the authoring context, owns the Rust path.
    let mut other = DefinitionContext::types_in("::other");
    other.register::<Model>().unwrap();
    assert_eq!(api(&other, ep).unwrap().rust_type(desc.type_ref()).as_deref(), Some("::std::option::Option<::std::vec::Vec<::other::Model>>"));
    let mut persisted = built;
    for _ in 0..2 {
        let read: RestApi = serde_json::from_str(&serde_json::to_string(&persisted).unwrap()).unwrap();
        read.validate_references().unwrap();
        assert_eq!(read, persisted);
        persisted = read;
    }
    let request = Endpoint::post("Request", "/").describes("Request").request_json::<Model>(&mut source).returns_empty().build().unwrap();
    assert!(matches!(api(&DefinitionContext::types_in("::models"), request.clone()), Err(DefinitionError::UnresolvedReference { endpoint, .. }) if endpoint == "Request"));
    api(&destination, request).unwrap();
}

#[test]
fn deserialized_definitions_are_checked_by_the_same_resolution() {
    let mut ctx = DefinitionContext::types_in("::models");
    let ep = Endpoint::get("Get", "/").describes("Get").returns_json::<Vec<Model>>(&mut ctx).build().unwrap();
    let value = serde_json::to_value(api(&ctx, ep).unwrap()).unwrap();
    let load = |value: &serde_json::Value| serde_json::from_value::<RestApi>(value.clone()).map_err(|e| e.to_string()).and_then(|api| api.validate_references().map_err(|e| e.to_string()));
    load(&value).unwrap();
    let mut dangling = value.clone();
    dangling["endpoints"][0]["response"] = json!({"Json": {"Array": {"Named": "Missing"}}});
    assert_eq!(load(&dangling).unwrap_err(), "Fixture endpoint Get: unresolved schema reference \"Missing\"");
    let mut nested_only = value.clone();
    nested_only["schemas"][0]["rust_path"] = serde_json::Value::Null;
    assert!(load(&nested_only).unwrap_err().contains("unresolved schema reference \"Model\""));
    let mut duplicate = value.clone();
    let component = duplicate["schemas"][0].clone();
    duplicate["schemas"].as_array_mut().unwrap().push(component);
    assert!(load(&duplicate).unwrap_err().contains("defined more than once"));
    let mut composition = value.clone();
    composition["endpoints"][0]["response"] = json!({"Json": {"OneOf": [{"Named": "Model"}]}});
    assert!(load(&composition).unwrap_err().contains("unsupported schema reference"));
    let mut empty_key = value.clone();
    empty_key["endpoints"][0]["response"] = json!({"Json": {"KeyedMap": {"key": " ", "value": {"Named": "Model"}}}});
    assert!(load(&empty_key).unwrap_err().contains("unsupported schema reference"));
    let mut unknown_field = value;
    unknown_field["schemas"][0]["extra"] = json!(true);
    assert!(load(&unknown_field).unwrap_err().contains("unknown field"));
}

definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Outer { pub inner: Inner });
#[derive(JsonSchema)]
#[allow(dead_code)]
pub struct Inner { pub flag: bool }
definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Unused { pub note: String });

#[test]
fn api_carries_the_whole_context_with_nested_definitions_flattened() {
    let mut ctx = DefinitionContext::types_in("::models");
    ctx.register::<Unused>().unwrap();
    let ep = Endpoint::get("Get", "/").describes("Get").returns_json::<Outer>(&mut ctx).build().unwrap();
    let built = api(&ctx, ep).unwrap();
    let names = |api: &RestApi| api.schemas.iter().map(|c| (c.name.clone(), c.rust_path.clone())).collect::<Vec<_>>();
    assert_eq!(names(&built), [("Unused".into(), Some("::models::Unused".into())), ("Outer".into(), Some("::models::Outer".into())), ("Inner".into(), None)]);
    let outer = built.component("Outer").unwrap();
    assert_eq!(outer.schema["properties"]["inner"]["$ref"], "#/components/schemas/Inner");
    for key in ["$schema", "$defs", "components"] { assert!(outer.schema.get(key).is_none(), "{key} kept"); }
    assert_eq!(built.component("Inner").unwrap().schema["properties"]["flag"]["type"], "boolean");
    let stored: &SchemaComponent = built.component("Unused").unwrap();
    assert_eq!(stored.schema["properties"]["note"]["type"], "string");
}

#[test]
fn api_build_reports_every_registration_collision() {
    let mut ctx = DefinitionContext::types_in("::models");
    ctx.register::<Model>().unwrap();
    assert!(ctx.register::<ClashA>().is_err());
    assert!(ctx.register::<ClashB>().is_err());
    let error = RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").no_auth().build().unwrap_err();
    assert_eq!(error.diagnostics("Fixture").lines().count(), 2);
    assert_eq!(ctx.registrations().len(), 1);
}

#[test]
fn public_default_set_equals_documented_allow_list() {
    use std::collections::BTreeSet;
    let corpus = [
        include_str!("../src/auth.rs"),
        include_str!("../src/builders.rs"),
        include_str!("../src/components.rs"),
        include_str!("../src/cross_origin.rs"),
        include_str!("../src/headers/builder/auth.rs"),
        include_str!("../src/headers/builder/env.rs"),
        include_str!("../src/headers/builder/mod.rs"),
        include_str!("../src/headers/builder/tests.rs"),
        include_str!("../src/headers/builder/validate.rs"),
        include_str!("../src/headers/env.rs"),
        include_str!("../src/headers/error.rs"),
        include_str!("../src/headers/mod.rs"),
        include_str!("../src/headers/sensitive.rs"),
        include_str!("../src/lib.rs"),
        include_str!("../src/models/aliases.rs"),
        include_str!("../src/models/enums.rs"),
        include_str!("../src/models/mod.rs"),
        include_str!("../src/models/structs.rs"),
        include_str!("../src/models/type_ref.rs"),
        include_str!("../src/oauth.rs"),
        include_str!("../src/openapi/error.rs"),
        include_str!("../src/openapi/export/components.rs"),
        include_str!("../src/openapi/export/conversion.rs"),
        include_str!("../src/openapi/export/info.rs"),
        include_str!("../src/openapi/export/mod.rs"),
        include_str!("../src/openapi/export/parameters.rs"),
        include_str!("../src/openapi/export/paths.rs"),
        include_str!("../src/openapi/export/request_body.rs"),
        include_str!("../src/openapi/export/responses.rs"),
        include_str!("../src/openapi/export/security.rs"),
        include_str!("../src/openapi/export/validate.rs"),
        include_str!("../src/openapi/extensions.rs"),
        include_str!("../src/openapi/import/auth.rs"),
        include_str!("../src/openapi/import/builder.rs"),
        include_str!("../src/openapi/import/diagnostics.rs"),
        include_str!("../src/openapi/import/mappings/mod.rs"),
        include_str!("../src/openapi/import/mappings/parameters.rs"),
        include_str!("../src/openapi/import/mappings/responses.rs"),
        include_str!("../src/openapi/import/mappings/schema.rs"),
        include_str!("../src/openapi/import/naming.rs"),
        include_str!("../src/openapi/import/normalize.rs"),
        include_str!("../src/openapi/import/resolver.rs"),
        include_str!("../src/openapi/import.rs"),
        include_str!("../src/openapi/mod.rs"),
        include_str!("../src/openapi/options.rs"),
        include_str!("../src/openapi/source.rs"),
        include_str!("../src/pagination/collection.rs"),
        include_str!("../src/pagination/resolve.rs"),
        include_str!("../src/pagination/selector.rs"),
        include_str!("../src/pagination/strategy.rs"),
        include_str!("../src/pagination/validate.rs"),
        include_str!("../src/pagination.rs"),
        include_str!("../src/params/mod.rs"),
        include_str!("../src/params/param_def.rs"),
        include_str!("../src/prelude.rs"),
        include_str!("../src/registration.rs"),
        include_str!("../src/request.rs"),
        include_str!("../src/response.rs"),
        include_str!("../src/schema.rs"),
        include_str!("../src/types.rs"),
        include_str!("../src/websocket/hints.rs"),
        include_str!("../src/websocket/lifecycle.rs"),
        include_str!("../src/websocket/mod.rs"),
        include_str!("../src/websocket/params.rs"),
    ];
    let mut observed = BTreeSet::new();
    for source in corpus {
        let mut derive = String::new();
        let mut collecting = false;
        let mut has_default = false;
        for line in source.lines() {
            let line = line.trim();
            if line.starts_with("//") { continue; }
            if line.starts_with("#[derive(") { derive.clear(); collecting = true; }
            if collecting {
                derive.push_str(line);
                if line.contains(")]") {
                    collecting = false;
                    has_default = derive.split(|c: char| !c.is_alphanumeric() && c != '_').any(|word| word == "Default");
                }
            }
            if line.starts_with("pub struct ") || line.starts_with("pub enum ") {
                if has_default { observed.insert(line.split_whitespace().nth(2).unwrap().split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap()); }
                has_default = false;
            } else if line.starts_with("struct ") || line.starts_with("enum ") { has_default = false; }
            if let Some(rest) = line.strip_prefix("impl Default for ") {
                let name = rest.split_whitespace().next().unwrap();
                if source.contains(&format!("pub struct {name}")) || source.contains(&format!("pub enum {name}")) { observed.insert(name); }
            }
        }
    }
    let allowed = ["AuthPolicy", "AuthStrategy", "ConnectionLifecycle", "CrossOrigin", "EndpointParams",
        "EnvMapping", "ExportFormat", "ExportOptions", "FrameFormat", "Headers", "OAuth2ClientAuthMethod",
        "OpenApiImportOptions", "PkceRequirement", "RequestIdType", "SchematicDocExtension",
        "SchematicSchemaExtension", "SensitiveString", "WebSocketEndpoint", "WebSocketEndpointHints",
        "WebSocketRuntimeHints"].into_iter().collect::<BTreeSet<_>>();
    assert_eq!(observed, allowed);
    let documentation = include_str!("../../docs/definition-builders.md");
    for name in allowed { assert!(documentation.contains(&format!("`{name}`")), "missing Default rationale for {name}"); }
}

#[test]
fn custom_body_content_types_survive_api_assembly() {
    let ctx = DefinitionContext::types_in("::models");
    for (ep, expected) in [
        (Endpoint::post("Csv", "/").describes("CSV").request_text_with_content_type("text/csv").returns_empty().build().unwrap(), ApiRequest::Text { content_type: "text/csv".into() }),
        (Endpoint::post("Image", "/").describes("Image").request_binary_with_content_type("image/png").returns_empty().build().unwrap(), ApiRequest::Binary { content_type: "image/png".into() }),
    ] {
        assert_eq!(api(&ctx, ep).unwrap().endpoints[0].request, Some(expected));
    }
}

#[test]
fn typed_list_recognition_respects_root_array_and_get_boundaries() {
    use schematic_define::pagination::{Resolution, caller_knobs, resolve};
    let mut ctx = DefinitionContext::types_in("::models");
    let list = Endpoint::get("List", "/").describes("List").returns_json::<Vec<Model>>(&mut ctx).build().unwrap();
    let optional = Endpoint::get("Optional", "/").describes("Optional").returns_json::<Option<Vec<Model>>>(&mut ctx).build().unwrap();
    let nominal = Endpoint::get("One", "/").describes("One").returns_json::<Model>(&mut ctx).build().unwrap();
    let post = Endpoint::post("Post", "/").describes("Post").returns_json::<Vec<Model>>(&mut ctx).build().unwrap();
    let built = RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").no_auth()
        .endpoints([list.clone(), optional.clone(), nominal.clone(), post.clone()]).build().unwrap();
    let resolution = resolve(&built, &list);
    assert!(matches!(&resolution, Resolution::Unpaginated { list, .. } if list.item_type.as_deref() == Some("::models::Model") && list.items.is_root_array()));
    assert!(caller_knobs(&resolution).is_empty());
    for ep in [optional, nominal, post] { assert_eq!(resolve(&built, &ep), Resolution::NotAList); }
}

#[test]
fn missing_destination_references_fail_in_both_form_encodings() {
    let mut source = DefinitionContext::types_in("::models");
    let desc = source.register::<Model>().unwrap();
    let fields = vec![FormField { name: "metadata".into(), required: true, description: None,
        kind: FormFieldKind::Json(desc.into_type_ref()) }];
    let destination = DefinitionContext::types_in("::models");
    for ep in [Endpoint::post("Multipart", "/").describes("Multipart").request_form(fields.clone()).returns_empty().build().unwrap(),
        Endpoint::post("Encoded", "/").describes("Encoded").request_urlencoded(fields).returns_empty().build().unwrap()] {
        let error = api(&destination, ep).unwrap_err().to_string();
        assert!(error.contains("Fixture") && error.contains("Model"));
    }
}

#[test]
fn api_builder_preserves_transfer_metadata_and_authoring_order() {
    let ctx = DefinitionContext::types_in("::models");
    let ep = Endpoint::get("Get", "/").describes("Get").returns_empty().build().unwrap();
    let built = RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API")
        .bearer_env(["TOKEN", "TOKEN_FALLBACK"]).docs_url("https://example.com/docs").version("1")
        .module_path("::models").request_suffix("Request").header("X-First", "one").header("X-Second", "two")
        .endpoints([ep.clone()]).default_errors(ErrorSet::none()).build().unwrap();
    let literal = { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "Fixture".into(); fixture.description = "API".into(); fixture.base_url = "https://example.com".into(); fixture.docs_url = Some("https://example.com/docs".into()); fixture.auth = AuthStrategy::BearerToken { header: None }; fixture.auth_policy = None; fixture.env_auth = vec!["TOKEN".into(), "TOKEN_FALLBACK".into()]; fixture.env_username = None; fixture.headers = vec![("X-First".into(), "one".into()), ("X-Second".into(), "two".into())]; fixture.endpoints = vec![ep]; fixture.module_path = Some("::models".into()); fixture.request_suffix = Some("Request".into()); fixture.version = Some("1".into()); fixture.env_mapping = None; fixture.cross_origin = schematic_define::CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture };
    assert_eq!(built, literal);
    assert_eq!(serde_json::to_value(built).unwrap(), serde_json::to_value(literal).unwrap());
}

#[test]
fn basic_and_key_mapping_conflicts_fail_independent_of_setter_order() {
    let ctx = DefinitionContext::types_in("::models");
    for mapping in [EnvMapping { basic_user: Some(EnvList::single("OTHER_USER")), basic_pass: Some(EnvList::single("PASS")), ..EnvMapping::default() },
        EnvMapping { basic_user: Some(EnvList::single("USER")), basic_pass: Some(EnvList::single("OTHER_PASS")), ..EnvMapping::default() }] {
        for result in [RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").env_mapping(mapping.clone()).basic_env("USER", ["PASS"]).build(),
            RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").basic_env("USER", ["PASS"]).env_mapping(mapping).build()] {
            assert!(result.unwrap_err().to_string().contains("conflicts"));
        }
    }
    for result in [RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").basic_env("USER", ["PASS"]).env_username("OTHER").build(),
        RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").env_username("OTHER").basic_env("USER", ["PASS"]).build()] {
        assert!(result.is_err());
    }
    let auth = AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: None };
    for mapping in [EnvMapping { api_key: Some(ApiKeyEnv { names: EnvList::single("OTHER_KEY"), header: "X-Key".into() }), ..EnvMapping::default() },
        EnvMapping { api_key: Some(ApiKeyEnv { names: EnvList::single("KEY"), header: "X-Other".into() }), ..EnvMapping::default() }] {
        for result in [RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").env_mapping(mapping.clone()).authentication(auth.clone(), ["KEY"]).build(),
            RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").authentication(auth.clone(), ["KEY"]).env_mapping(mapping).build()] {
            assert!(result.unwrap_err().to_string().contains("conflicts"));
        }
    }
}

#[test]
fn primitive_and_free_form_json_successes_remain_component_free() {
    let mut ctx = DefinitionContext::types_in("::models");
    let primitive = Endpoint::get("Primitive", "/").describes("Primitive").returns_json::<String>(&mut ctx).build().unwrap();
    let free_form = Endpoint::get("FreeForm", "/").describes("Free form").returns_json::<serde_json::Value>(&mut ctx).build().unwrap();
    let built = RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API").no_auth().endpoints([primitive, free_form]).build().unwrap();
    assert!(ctx.registrations().is_empty() && built.schemas.is_empty());
    for ep in built.endpoints {
        let ApiResponse::Json(reference) = ep.response else { panic!("JSON shape lost") };
        let wire = reference.wire_schema();
        assert!(wire.get("$ref").is_none());
        assert!(wire == json!({}) || wire == json!({"type":"string"}));
    }
}

#[test]
fn explicit_empty_mapping_preserves_the_absence_of_environment_credentials() {
    let ctx = DefinitionContext::types_in("::models");
    for before_auth in [false, true] {
        let api = if before_auth {
            RestApi::builder(&ctx, "Local", "http://localhost:1234").describes("Local")
                .env_mapping(EnvMapping::default()).bearer_env(["LM_API_TOKEN"]).build()
        } else {
            RestApi::builder(&ctx, "Local", "http://localhost:1234").describes("Local")
                .bearer_env(["LM_API_TOKEN"]).env_mapping(EnvMapping::default()).build()
        }.unwrap();
        assert_eq!(api.env_auth, ["LM_API_TOKEN"]);
        assert_eq!(api.env_mapping, Some(EnvMapping::default()));
        assert_eq!(api.default_env_mapping(), EnvMapping::default());
    }
}

/// One edit per cell of the transfer matrix over the fields API loading reads:
/// each component's `name`, `rust_path` and `schema`, the `schemas` list, and
/// the endpoint reference.
#[test]
fn transferred_components_and_references_input_matrix() {
    let mut ctx = DefinitionContext::types_in("::models");
    let ep = Endpoint::get("Get", "/").describes("Get").returns_json::<Vec<Model>>(&mut ctx).build().unwrap();
    let control = serde_json::to_string(&api(&ctx, ep).unwrap()).unwrap();
    let load = |text: &str| serde_json::from_str::<RestApi>(text).map_err(|e| e.to_string())
        .and_then(|api| api.validate_references().map(|()| api).map_err(|e| e.to_string()));
    let edit = |change: &dyn Fn(&mut serde_json::Value)| { let mut value: serde_json::Value = serde_json::from_str(&control).unwrap(); change(&mut value); value.to_string() };
    let component = |field: &'static str, replacement: serde_json::Value| edit(&move |v| v["schemas"][0][field] = replacement.clone());
    let remove = |field: &'static str| edit(&move |v| { v["schemas"][0].as_object_mut().unwrap().remove(field); });

    // Control: the unedited document loads and the edited fields matter.
    let loaded = load(&control).unwrap();
    assert_eq!(loaded.rust_type(loaded.endpoints[0].json_references()[0]).as_deref(), Some("::std::vec::Vec<::models::Model>"));
    assert_eq!(load(&component("name", json!("Renamed"))).unwrap_err(), "Fixture endpoint Get: unresolved schema reference \"Model\"");
    assert!(load(&component("rust_path", json!("::other::Model"))).is_ok_and(|api| api.rust_type(api.endpoints[0].json_references()[0]).unwrap().contains("::other::Model")));

    // Absent and explicit null: a missing name or schema is a load error; an
    // absent or null Rust path both mean a nested-only component.
    for text in [remove("name"), component("name", serde_json::Value::Null), remove("schema")] { assert!(load(&text).is_err(), "{text}"); }
    assert!(load(&component("schema", serde_json::Value::Null)).unwrap_err().contains("not a JSON Schema"));
    for text in [remove("rust_path"), component("rust_path", serde_json::Value::Null)] {
        assert!(load(&text).unwrap_err().contains("unresolved schema reference"), "{text}");
    }

    // Wrong whole type.
    for (field, value) in [("name", json!(123)), ("rust_path", json!(false)), ("name", json!(["Model"])), ("rust_path", json!({}))] {
        assert!(load(&component(field, value)).is_err());
    }
    for value in [json!(123), json!("object"), json!(["type"])] {
        assert!(load(&component("schema", value)).unwrap_err().contains("not a JSON Schema"));
    }
    assert!(load(&edit(&|v| v["schemas"] = json!({"Model": {}}))).is_err());

    // One wrong element, every wrong element, and empty.
    assert!(load(&edit(&|v| v["schemas"].as_array_mut().unwrap().push(json!(123)))).is_err());
    assert!(load(&edit(&|v| v["schemas"] = json!([123, "Model"]))).is_err());
    assert!(load(&edit(&|v| v["schemas"] = json!([]))).unwrap_err().contains("unresolved schema reference \"Model\""));
    assert!(load(&component("name", json!(" "))).unwrap_err().contains("empty name"));
    assert!(load(&component("rust_path", json!(""))).unwrap_err().contains("empty Rust path"));
    assert!(load(&component("schema", json!({}))).is_ok(), "an unconstrained schema is valid");
    assert!(load(&edit(&|v| v["endpoints"][0]["response"] = json!({"Json": {"Named": ""}}))).unwrap_err().contains("unresolved schema reference \"\""));
    assert!(load(&edit(&|v| v["endpoints"][0]["response"] = json!({"Json": {}}))).is_err());

    // Duplicate keys, preserved textually, and trailing content.
    for (field, first) in [("\"name\":", "\"Model\""), ("\"rust_path\":", "\"::models::Model\""), ("\"schema\":", "{}")] {
        let start = control.find("\"schemas\":[").unwrap();
        let (head, tail) = control.split_at(start);
        let duplicated = format!("{head}{}", tail.replacen(field, &format!("{field}{first},{field}"), 1));
        let error = load(&duplicated).unwrap_err();
        assert!(error.contains("duplicate field"), "{field}: {error}");
    }
    assert!(load(&format!("{control} trailing")).is_err());
}

mod left_status {
    use super::*;
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Status { pub value: String });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Left { pub status: Status });
}
mod right_status {
    use super::*;
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Status { pub value: bool });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Right { pub status: Status });
}

fn status_conflict(error: &DefinitionError) -> Option<(&str, &str)> {
    match error {
        DefinitionError::Collisions(errors) if errors.len() == 1 => match &errors[0] {
            DefinitionError::ComponentConflict { schema_name, first, second } if schema_name == "Status" => Some((first, second)),
            _ => None,
        },
        _ => None,
    }
}

type Register = fn(&mut DefinitionContext);

#[test]
fn conflicting_nested_and_explicit_definitions_of_one_name_fail_build() {
    let nested_then_nested: Register = |ctx| {
        ctx.register::<left_status::Left>().unwrap();
        ctx.register::<right_status::Right>().unwrap();
    };
    let nested_then_explicit: Register = |ctx| {
        ctx.register::<left_status::Left>().unwrap();
        ctx.register::<right_status::Status>().unwrap();
    };
    let explicit_then_nested: Register = |ctx| {
        ctx.register::<left_status::Status>().unwrap();
        ctx.register::<left_status::Left>().unwrap();
        ctx.register::<right_status::Right>().unwrap();
    };
    let shapes: [(&str, Register, &str, &str); 3] = [
        ("nested/nested", nested_then_nested, "a schema nested in builders::left_status::Left", "a schema nested in builders::right_status::Right"),
        ("nested/explicit", nested_then_explicit, "a schema nested in builders::left_status::Left", "builders::right_status::Status"),
        ("explicit/nested", explicit_then_nested, "builders::left_status::Status", "a schema nested in builders::right_status::Right"),
    ];
    for (shape, register, first, second) in shapes {
        let mut ctx = DefinitionContext::types_in("::models");
        register(&mut ctx);
        assert!(ctx.finish().is_ok(), "{shape}: no registration collision");
        assert_eq!(status_conflict(&ctx.components().unwrap_err()), Some((first, second)), "{shape}");
        let ep = Endpoint::get("Get", "/").describes("Get").returns_json::<left_status::Left>(&mut ctx).build().unwrap();
        let error = api(&ctx, ep).unwrap_err();
        assert_eq!(status_conflict(&error), Some((first, second)), "{shape}");
        let message = error.diagnostics("Fixture");
        assert!(message.contains("\"Status\"") && message.contains(first) && message.contains(second), "{shape}: {message}");
    }
}

/// Doc comment carried by both the root and the nested schema.
#[derive(JsonSchema)]
#[allow(dead_code)]
pub struct Shared { pub value: String, pub count: Option<u8>, pub tags: Option<Vec<String>> }
impl schematic_define::Registration for Shared {
    const RUST_IDENTIFIER: Option<&'static str> = Some("Shared");
    fn register(ctx: &mut DefinitionContext) -> Result<schematic_define::TypeDesc, DefinitionError> { ctx.register_nominal::<Self>() }
}
definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct FirstHolder { pub shared: Shared });
definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct SecondHolder { pub shared: Option<Shared> });

#[test]
fn the_same_type_extracted_repeatedly_or_also_registered_merges_in_first_position() {
    let names = |components: &[SchemaComponent]| components.iter().map(|c| (c.name.clone(), c.rust_path.clone())).collect::<Vec<_>>();
    let holder = |name: &str| (name.to_string(), Some(format!("::models::{name}")));

    let mut ctx = DefinitionContext::types_in("::models");
    ctx.register::<FirstHolder>().unwrap();
    ctx.register::<SecondHolder>().unwrap();
    assert_eq!(names(&ctx.components().unwrap()), [holder("FirstHolder"), ("Shared".into(), None), holder("SecondHolder")]);

    ctx.register::<Shared>().unwrap();
    let ep = Endpoint::get("Get", "/").describes("Get").returns_json::<SecondHolder>(&mut ctx).build().unwrap();
    let built = api(&ctx, ep).unwrap();
    assert_eq!(names(&built.schemas), [holder("FirstHolder"), holder("Shared"), holder("SecondHolder")]);
    assert_eq!(built.component("Shared").unwrap().schema["properties"]["count"]["nullable"], true);

    let mut ctx = DefinitionContext::types_in("::models");
    ctx.register::<Shared>().unwrap();
    ctx.register::<FirstHolder>().unwrap();
    ctx.register::<SecondHolder>().unwrap();
    assert_eq!(names(&ctx.components().unwrap()), [holder("Shared"), holder("FirstHolder"), holder("SecondHolder")]);
}

definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Body { pub block: Block });
definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Block { pub result: Option<Contents> });
#[derive(JsonSchema)]
#[allow(dead_code)]
pub struct Contents { pub blocks: Vec<Block> }

#[test]
fn a_recursive_type_nested_elsewhere_matches_its_copy_under_its_own_root() {
    let mut ctx = DefinitionContext::types_in("::models");
    ctx.register::<Body>().unwrap();
    ctx.register::<Block>().unwrap();
    let components = ctx.components().unwrap();
    assert_eq!(components.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Body", "Block", "Contents"]);
    let contents = components.iter().find(|c| c.name == "Contents").unwrap();
    assert_eq!(contents.schema["properties"]["blocks"]["items"]["$ref"], "#/components/schemas/Block");
}
