use schematic_define::{ApiKeyEnv, ApiRequest, ApiResponse, AuthMethod, AuthPolicy, AuthStrategy,
    DefinitionContext, Endpoint, EndpointParams, EnvAuthStrategy, EnvList, EnvMapping, ErrorSet,
    FormField, FormFieldKind, RestApi, RestMethod, Schema, Success, TypeDesc, definition_type};
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
    for (success, expected) in [(Success::json(desc.clone()), ApiResponse::Json(Schema {
        type_name: desc.rust_path(), module_path: None, descriptor: Some(desc.clone()),
    })), (Success::text(), ApiResponse::Text), (Success::binary(), ApiResponse::Binary),
        (Success::empty(), ApiResponse::Empty)] {
        let ep = Endpoint::post("Operation", "/items").describes("Operation").returns(success).build().unwrap();
        assert_eq!(ep.response, expected);
        assert_eq!(api(&ctx, ep.clone()).unwrap().endpoints, [ep]);
    }
    let json = Endpoint::post("Json", "/items").describes("JSON").request_json::<Model>(&mut ctx)
        .returns_json::<Vec<Model>>(&mut ctx).build().unwrap();
    assert!(matches!(&json.request, Some(ApiRequest::Json(s)) if s.type_name == desc.rust_path()));
    assert!(matches!(&json.response, ApiResponse::Json(s) if s.type_name.contains("Vec<")));
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
            kind: FormFieldKind::Json(Schema { type_name: desc.rust_path(), module_path: None, descriptor: Some(desc) }) }];
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
    let literal = Endpoint { id: "Get".into(), method: RestMethod::Get, path: "/items".into(),
        description: "Get items".into(), request: None, response: ApiResponse::Text,
        headers: vec![("Accept".into(), "text/plain".into())], params: Some(EndpointParams::default()), oauth_scopes: None, cross_origin: None, pagination: None };
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
fn destination_context_checks_transfers_roundtrips_and_forged_wrappers() {
    let mut source = DefinitionContext::types_in("::models");
    let mut desc = source.register::<Option<Vec<Model>>>().unwrap();
    for _ in 0..2 { desc = serde_json::from_str(&serde_json::to_string(&desc).unwrap()).unwrap(); }
    let ep = Endpoint::get("GetModels", "/models").describes("Models").returns(Success::json(desc.clone())).build().unwrap();
    let mut destination = DefinitionContext::types_in("::models");
    let error = api(&destination, ep.clone()).unwrap_err().to_string();
    assert!(error.contains("Fixture") && error.contains("GetModels") && error.contains("Model"));
    destination.register::<Option<Vec<Model>>>().unwrap();
    assert_eq!(api(&destination, ep.clone()).unwrap().endpoints, [ep]);
    let mut stripped = Endpoint::get("Stripped", "/").describes("Stripped")
        .returns(Success::json(desc.clone())).build().unwrap();
    let ApiResponse::Json(schema) = &mut stripped.response else { panic!("JSON response") };
    schema.descriptor = None;
    let error = api(&destination, stripped).unwrap_err().to_string();
    assert!(error.contains("Fixture") && error.contains("Stripped") && error.contains("Model"));
    let original = serde_json::to_value(desc).unwrap();
    let mut wrong_path = original.clone();
    wrong_path["shape"]["Option"]["Vec"]["Named"]["rust_path"] = json!("::other::Model");
    let mut wrong_name = original.clone();
    wrong_name["shape"]["Option"]["Vec"]["Named"]["schema_name"] = json!("Missing");
    let mut wrong_shape = original.clone();
    wrong_shape["shape"] = json!({"Vec": original["shape"].clone()});
    let mut reserved = original;
    reserved["structure"] = json!({"Named":"unsupported"});
    for data in [wrong_path, wrong_name, wrong_shape, reserved] {
        let forged: TypeDesc = serde_json::from_value(data).unwrap();
        let ep = Endpoint::get("Forged", "/").describes("Forged").returns(Success::json(forged)).build().unwrap();
        let error = api(&destination, ep).unwrap_err().to_string();
        assert!(error.contains("Fixture") && error.contains("Forged"));
    }
    let valid = Endpoint::get("Roundtrip", "/").describes("Roundtrip")
        .returns_json::<Option<Vec<Model>>>(&mut source).build().unwrap();
    let mut persisted = api(&destination, valid).unwrap();
    for _ in 0..2 {
        persisted = serde_json::from_str(&serde_json::to_string(&persisted).unwrap()).unwrap();
        assert_eq!(api(&destination, persisted.endpoints[0].clone()).unwrap(), persisted);
    }
    let request = Endpoint::post("Request", "/").describes("Request").request_json::<Model>(&mut source).returns_empty().build().unwrap();
    assert!(api(&DefinitionContext::types_in("::models"), request.clone()).is_err());
    api(&destination, request).unwrap();
    let mut changed_lowering = Endpoint::get("ChangedLowering", "/").describes("Changed lowering")
        .returns_json::<Model>(&mut source).build().unwrap();
    let ApiResponse::Json(schema) = &mut changed_lowering.response else { panic!("JSON response") };
    schema.type_name = "::other::Model".into();
    let error = api(&destination, changed_lowering).unwrap_err().to_string();
    assert!(error.contains("Fixture") && error.contains("ChangedLowering") && error.contains("disagrees"));
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
        kind: FormFieldKind::Json(Schema { type_name: desc.rust_path(), module_path: None, descriptor: Some(desc) }) }];
    let destination = DefinitionContext::types_in("::models");
    for ep in [Endpoint::post("Multipart", "/").describes("Multipart").request_form(fields.clone()).returns_empty().build().unwrap(),
        Endpoint::post("Encoded", "/").describes("Encoded").request_urlencoded(fields).returns_empty().build().unwrap()] {
        let error = api(&destination, ep).unwrap_err().to_string();
        assert!(error.contains("Fixture") && error.contains("Model"));
    }
}

#[test]
fn api_builder_preserves_legacy_literal_metadata_and_authoring_order() {
    let ctx = DefinitionContext::types_in("::models");
    let ep = Endpoint::get("Get", "/").describes("Get").returns_empty().build().unwrap();
    let built = RestApi::builder(&ctx, "Fixture", "https://example.com").describes("API")
        .bearer_env(["TOKEN", "TOKEN_FALLBACK"]).docs_url("https://example.com/docs").version("1")
        .module_path("::models").request_suffix("Request").header("X-First", "one").header("X-Second", "two")
        .endpoints([ep.clone()]).default_errors(ErrorSet::none()).build().unwrap();
    let literal = RestApi {
        name: "Fixture".into(), description: "API".into(), base_url: "https://example.com".into(),
        docs_url: Some("https://example.com/docs".into()), auth: AuthStrategy::BearerToken { header: None },
        auth_policy: None, env_auth: vec!["TOKEN".into(), "TOKEN_FALLBACK".into()], env_username: None,
        headers: vec![("X-First".into(), "one".into()), ("X-Second".into(), "two".into())], endpoints: vec![ep],
        module_path: Some("::models".into()), request_suffix: Some("Request".into()), version: Some("1".into()),
        env_mapping: None, cross_origin: schematic_define::CrossOrigin::default(), request_id_header: None,
        secret_names: vec![], default_pagination: None, pagination_exceptions: vec![],
    };
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
    assert!(ctx.registrations().is_empty());
    for ep in built.endpoints {
        let ApiResponse::Json(schema) = ep.response else { panic!("JSON shape lost") };
        let wire = schema.descriptor.unwrap().wire_schema();
        assert!(wire.get("$ref").is_none());
        assert!(wire == json!({}) || wire == json!({"type":"string"}));
    }
}
