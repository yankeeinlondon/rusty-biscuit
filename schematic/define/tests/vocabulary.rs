//! Definition vocabulary: typed paths, deprecation, rate-limit and retry
//! metadata, endpoint authentication, and pager-managed required defaults,
//! through the public builders and serde.

use schematic_define::pagination::{CursorSource, MoreSignal, PageSize, Strategy};
use schematic_define::{
    ApiKeyLocation, AuthMethod, AuthPolicy, AuthStrategy, Backoff, ClientCredentialsGrant, DefinitionContext,
    DefinitionError, Endpoint, EndpointAuth, EndpointBuilder, EndpointParams, EnvAuthStrategy, EnvList, EnvMapping,
    OAuth2ClientAuthMethod, OAuth2Config, ParamDef, ParamStyle, QueryParamType, RateLimit, ResetUnit,
    ResolvedEndpointAuth, RestApi, RetryGuidance, StatusSelector, definition_type,
};
use schematic_define::builders::Present;
use schemars::JsonSchema;
use serde_json::{Value, json};

definition_type!(#[derive(JsonSchema)] pub struct Item { pub id: u64 });

fn get(id: &str, path: &str) -> EndpointBuilder<Present, Present> {
    Endpoint::get(id, path).describes("An operation").returns_empty()
}

fn api_with(endpoints: Vec<Endpoint>) -> Result<RestApi, DefinitionError> {
    let ctx = DefinitionContext::types_in("::models");
    RestApi::builder(&ctx, "Vocab", "https://example.com").describes("Vocabulary").no_auth()
        .endpoints(endpoints).build()
}

fn message(result: Result<impl std::fmt::Debug, DefinitionError>) -> String {
    result.unwrap_err().to_string()
}

fn rate_limit(prefix: &str) -> RateLimit {
    RateLimit::new(format!("{prefix}-limit"), format!("{prefix}-remaining"), format!("{prefix}-reset"), ResetUnit::EpochSeconds).unwrap()
}

fn retry(attempts: u8) -> RetryGuidance {
    RetryGuidance::new([StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 599 }], true, attempts,
        Backoff::Exponential { base_ms: 250, cap_ms: 8000 }).unwrap()
}

fn bearer_policy() -> AuthPolicy {
    AuthPolicy::from_auth_strategy(&AuthStrategy::BearerToken { header: None })
}

fn to_value(endpoint: &Endpoint) -> Value { serde_json::to_value(endpoint).unwrap() }

fn endpoint_with(field: &str, value: Value) -> Result<Endpoint, serde_json::Error> {
    let mut json = to_value(&get("Get", "/items").build().unwrap());
    json.as_object_mut().unwrap().insert(field.into(), value);
    serde_json::from_value(json)
}

// ---------------------------------------------------------------------------
// Typed path parameters
// ---------------------------------------------------------------------------

#[test]
fn every_scalar_and_enum_path_type_is_accepted_and_forced_required_simple() {
    let endpoint = get("Get", "/{id}/{ratio}/{flag}/{name}/{kind}")
        .path_param(ParamDef::path("id", QueryParamType::Integer).describes("Numeric id"))
        .path_param(ParamDef::path("ratio", QueryParamType::Number))
        .path_param(ParamDef::path("flag", QueryParamType::Boolean))
        // `path_param` forces the path rules even onto a query-shaped ParamDef.
        .path_param(ParamDef::new("name", QueryParamType::String))
        .path_param(ParamDef::path("kind", QueryParamType::Enum(vec!["a".into(), "b".into()])))
        .build().unwrap();
    let path = &endpoint.params.as_ref().unwrap().path;
    assert_eq!(path.len(), 5);
    assert!(path.iter().all(|param| param.required && param.style == ParamStyle::Simple && !param.explode));
    assert_eq!(path[0].description.as_deref(), Some("Numeric id"));
    api_with(vec![endpoint]).unwrap();
}

#[test]
fn reserved_and_repeated_placeholders_need_one_declaration_in_first_occurrence_order() {
    let endpoint = get("Get", "/{owner}/{+rest}/{owner}/{repo}")
        .path_param(ParamDef::path("repo", QueryParamType::String))
        .path_param(ParamDef::path("rest", QueryParamType::String))
        .path_param(ParamDef::path("owner", QueryParamType::String))
        .build().unwrap();
    assert_eq!(endpoint.path_placeholders(), ["owner", "rest", "repo"]);
    api_with(vec![endpoint]).unwrap();
}

#[test]
fn no_path_declarations_keep_legacy_untyped_paths() {
    let endpoint = get("Get", "/{owner}/{repo}").query(ParamDef::new("owner", QueryParamType::String)).build().unwrap();
    assert!(endpoint.params.as_ref().unwrap().path.is_empty());
    let api = api_with(vec![endpoint]).unwrap();
    assert!(!serde_json::to_string(&api).unwrap().contains("\"path\":["));
}

#[test]
fn malformed_path_declarations_fail_naming_the_parameter() {
    let mut required_false = ParamDef::path("id", QueryParamType::String);
    required_false.required = false;
    let mut form_style = ParamDef::path("id", QueryParamType::String);
    form_style.style = ParamStyle::Form;
    let params = |path: Vec<ParamDef>, query: Vec<ParamDef>| EndpointParams { path, query, ..EndpointParams::default() };
    let string = |name: &str| ParamDef::path(name, QueryParamType::String);
    let cases: Vec<(&str, &str, EndpointParams, &str)> = vec![
        ("Missing", "/{id}/{other}", params(vec![string("id")], vec![]), "path placeholder `{other}` has no declared path parameter"),
        ("Extra", "/{id}", params(vec![string("id"), string("ghost")], vec![]), "path parameter `ghost` has no `{ghost}` placeholder in `/{id}`"),
        ("Duplicate", "/{id}", params(vec![string("id"), string("id")], vec![]), "path parameter `id` is declared more than once"),
        ("Optional", "/{id}", params(vec![required_false], vec![]), "path parameter `id` must be required"),
        ("Form", "/{id}", params(vec![form_style], vec![]), "path parameter `id` must use the Simple style, not Form"),
        ("Array", "/{id}", params(vec![ParamDef::path("id", QueryParamType::Array(Box::new(QueryParamType::String)))], vec![]),
            "path parameter `id` must be a scalar or enum"),
        ("Json", "/{id}", params(vec![ParamDef::path("id", QueryParamType::Json)], vec![]), "path parameter `id` must be a scalar or enum"),
        ("EmptyEnum", "/{id}", params(vec![ParamDef::path("id", QueryParamType::Enum(vec![]))], vec![]),
            "path parameter `id` declares an enum with no values"),
        ("DupEnum", "/{id}", params(vec![ParamDef::path("id", QueryParamType::Enum(vec!["a".into(), "b".into(), "a".into()]))], vec![]),
            "path parameter `id` declares enum value `a` more than once"),
        ("Collision", "/{user-id}", params(vec![string("user-id")], vec![ParamDef::new("user_id", QueryParamType::String)]),
            "path parameter `user-id` and query parameter `user_id` both generate the Rust field `user_id`"),
        ("RustName", "/{id}", params(vec![string("id").rust_name("type")], vec![]), "path parameter `id` has invalid Rust name `type`"),
    ];
    for (id, path, endpoint_params, expected) in cases {
        let error = message(get(id, path).params(endpoint_params.clone()).build());
        assert!(error.starts_with(&format!("<endpoint> endpoint {id}: {expected}")), "{id}: {error}");

        // A value deserialized or mutated after endpoint build is checked again by API build.
        let mut endpoint = get(id, path).build().unwrap();
        endpoint.params = Some(endpoint_params);
        let error = message(api_with(vec![endpoint]));
        assert!(error.starts_with(&format!("Vocab endpoint {id}: {expected}")), "{id}: {error}");
    }
}

#[test]
fn deserialized_path_declarations_with_required_false_are_rejected_not_normalized() {
    let mut json = to_value(&get("Get", "/{id}").path_param(ParamDef::path("id", QueryParamType::Integer)).build().unwrap());
    json["params"]["path"][0]["required"] = json!(false);
    let endpoint: Endpoint = serde_json::from_value(json).unwrap();
    assert!(!endpoint.params.as_ref().unwrap().path[0].required);
    assert!(message(api_with(vec![endpoint])).contains("path parameter `id` must be required"));
}

#[test]
fn params_path_serde_absent_is_empty_and_null_or_wrong_shapes_fail() {
    let base = json!({"query": [], "header": [], "cookie": []});
    assert!(serde_json::from_value::<EndpointParams>(base.clone()).unwrap().path.is_empty());
    let with = |value: Value| { let mut json = base.clone(); json["path"] = value; serde_json::from_value::<EndpointParams>(json) };
    assert!(with(json!([])).unwrap().path.is_empty());
    for bad in [json!(null), json!({}), json!("id"), json!([1]), json!([{"name": "id"}])] {
        assert!(with(bad.clone()).is_err(), "{bad}");
    }
    let declared = serde_json::to_value(ParamDef::path("id", QueryParamType::Integer)).unwrap();
    assert_eq!(with(json!([declared])).unwrap().path, vec![ParamDef::path("id", QueryParamType::Integer)]);
}

// ---------------------------------------------------------------------------
// Deprecation
// ---------------------------------------------------------------------------

#[test]
fn endpoint_and_parameter_deprecation_set_the_flag_and_false_is_not_serialized() {
    let plain = get("Get", "/items").query(ParamDef::new("q", QueryParamType::String)).build().unwrap();
    assert!(!plain.deprecated && !plain.params.as_ref().unwrap().query[0].deprecated);
    assert!(!serde_json::to_string(&plain).unwrap().contains("deprecated"));

    let marked = get("Get", "/items").deprecated().query(ParamDef::new("q", QueryParamType::String).deprecated()).build().unwrap();
    assert!(marked.deprecated && marked.params.as_ref().unwrap().query[0].deprecated);
    let json = to_value(&marked);
    assert_eq!((&json["deprecated"], &json["params"]["query"][0]["deprecated"]), (&json!(true), &json!(true)));
    assert_eq!(serde_json::from_value::<Endpoint>(json).unwrap(), marked);
}

#[test]
fn deprecation_serde_absent_is_false_and_null_or_strings_fail() {
    assert!(!get("Get", "/items").build().unwrap().deprecated);
    assert!(endpoint_with("deprecated", json!(true)).unwrap().deprecated);
    assert!(!endpoint_with("deprecated", json!(false)).unwrap().deprecated);
    for bad in [json!(null), json!("true"), json!(""), json!({}), json!(1)] {
        assert!(endpoint_with("deprecated", bad.clone()).is_err(), "endpoint {bad}");
    }
    let param = serde_json::to_value(ParamDef::new("q", QueryParamType::String)).unwrap();
    assert!(!serde_json::from_value::<ParamDef>(param.clone()).unwrap().deprecated);
    for (value, expected) in [(json!(true), Some(true)), (json!(null), None), (json!("true"), None), (json!({}), None)] {
        let mut json = param.clone();
        json["deprecated"] = value.clone();
        assert_eq!(serde_json::from_value::<ParamDef>(json).ok().map(|param| param.deprecated), expected, "parameter {value}");
    }
}

// ---------------------------------------------------------------------------
// Rate-limit and retry metadata
// ---------------------------------------------------------------------------

#[test]
fn endpoint_metadata_replaces_the_api_value_whole_and_absence_inherits() {
    let ctx = DefinitionContext::types_in("::models");
    let inherits = get("Inherits", "/a").build().unwrap();
    let overrides = get("Overrides", "/b").rate_limit(rate_limit("ep")).retry_guidance(retry(1)).build().unwrap();
    let api = RestApi::builder(&ctx, "Vocab", "https://example.com").describes("Vocabulary").no_auth()
        .rate_limit(rate_limit("api")).retry_guidance(retry(3)).endpoints([inherits, overrides]).build().unwrap();
    assert_eq!(api.rate_limit, Some(rate_limit("api")));
    assert_eq!(api.retry_guidance, Some(retry(3)));
    assert_eq!(api.effective_rate_limit(&api.endpoints[0]), Some(&rate_limit("api")));
    assert_eq!(api.effective_retry_guidance(&api.endpoints[0]), Some(&retry(3)));
    // The endpoint value has no retry-after header although the API's
    // declaration might; nothing is merged.
    assert_eq!(api.effective_rate_limit(&api.endpoints[1]), Some(&rate_limit("ep")));
    assert_eq!(api.effective_retry_guidance(&api.endpoints[1]), Some(&retry(1)));

    let bare = api_with(vec![get("Get", "/c").build().unwrap()]).unwrap();
    assert_eq!((bare.effective_rate_limit(&bare.endpoints[0]), bare.effective_retry_guidance(&bare.endpoints[0])), (None, None));
}

#[test]
fn metadata_mutated_invalid_after_construction_is_rejected_by_build() {
    let ctx = DefinitionContext::types_in("::models");
    let mut bad_limit = rate_limit("api");
    bad_limit.limit_header = "bad header".into();
    let error = message(RestApi::builder(&ctx, "Vocab", "https://example.com").describes("Vocabulary").no_auth()
        .rate_limit(bad_limit.clone()).build());
    assert_eq!(error, "Vocab endpoint <api>: rate_limit: `limit_header` is not a valid HTTP header name: \"bad header\"");

    let mut bad_retry = retry(3);
    bad_retry.max_attempts = 0;
    let error = message(RestApi::builder(&ctx, "Vocab", "https://example.com").describes("Vocabulary").no_auth()
        .retry_guidance(bad_retry.clone()).build());
    assert_eq!(error, "Vocab endpoint <api>: retry_guidance: `max_attempts` must be at least 1, got 0");

    assert!(message(get("Get", "/a").rate_limit(bad_limit.clone()).build()).starts_with("<endpoint> endpoint Get: rate_limit:"));
    let mut endpoint = get("Get", "/a").build().unwrap();
    endpoint.retry_guidance = Some(bad_retry);
    assert!(message(api_with(vec![endpoint])).starts_with("Vocab endpoint Get: retry_guidance: `max_attempts`"));
}

#[test]
fn metadata_serde_for_each_api_and_endpoint_field() {
    let api = api_with(vec![get("Get", "/a").build().unwrap()]).unwrap();
    let api_json = serde_json::to_value(&api).unwrap();
    let values = [("rate_limit", serde_json::to_value(rate_limit("x")).unwrap()), ("retry_guidance", serde_json::to_value(retry(2)).unwrap())];
    for (field, valid) in values {
        assert!(api_json.get(field).is_none() && api_json["endpoints"][0].get(field).is_none(), "{field} serialized while absent");
        let api_with_field = |value: Value| {
            let mut json = api_json.clone();
            json[field] = value;
            serde_json::from_value::<RestApi>(json)
        };
        let endpoint_with_field = |value: Value| endpoint_with(field, value);
        let read_api = api_with_field(valid.clone()).unwrap();
        let read_endpoint = endpoint_with_field(valid.clone()).unwrap();
        assert_eq!(serde_json::to_value(&read_api).unwrap()[field], valid);
        assert_eq!(to_value(&read_endpoint)[field], valid);
        for bad in [json!(null), json!("x"), json!([]), json!({})] {
            assert!(api_with_field(bad.clone()).is_err(), "API {field} {bad}");
            assert!(endpoint_with_field(bad.clone()).is_err(), "endpoint {field} {bad}");
        }
    }
    let absent: RestApi = serde_json::from_value(api_json).unwrap();
    assert_eq!((absent.rate_limit, absent.retry_guidance), (None, None));
    let absent = get("Get", "/a").build().unwrap();
    assert_eq!((absent.rate_limit, absent.retry_guidance), (None, None));
}

// ---------------------------------------------------------------------------
// Endpoint authentication
// ---------------------------------------------------------------------------

fn bearer_api(endpoint: Endpoint) -> Result<RestApi, DefinitionError> {
    let ctx = DefinitionContext::types_in("::models");
    RestApi::builder(&ctx, "Bearer", "https://example.com").describes("Bearer API").bearer_env(["TOKEN"])
        .endpoints([endpoint]).build()
}

fn basic_api(endpoint: Endpoint) -> Result<RestApi, DefinitionError> {
    let ctx = DefinitionContext::types_in("::models");
    RestApi::builder(&ctx, "Basic", "https://example.com").describes("Basic API").basic_env("USER", ["PASS"])
        .endpoints([endpoint]).build()
}

fn api_key_api(header: &str, endpoint: Endpoint) -> Result<RestApi, DefinitionError> {
    let ctx = DefinitionContext::types_in("::models");
    RestApi::builder(&ctx, "Keyed", "https://example.com").describes("Keyed API")
        .authentication(AuthStrategy::ApiKey { header: header.into(), value_prefix: None }, ["KEY"])
        .endpoints([endpoint]).build()
}

fn query_key() -> AuthStrategy { AuthStrategy::ApiKeyParam { name: "key".into(), location: ApiKeyLocation::Query } }

#[test]
fn public_and_policy_overrides_are_stored_and_resolved() {
    let public = get("Public", "/a").public().build().unwrap();
    assert_eq!(public.auth, Some(EndpointAuth::Public));
    let api = bearer_api(public).unwrap();
    assert_eq!(api.endpoint_auth(&api.endpoints[0]), ResolvedEndpointAuth::Public);

    let inherits = bearer_api(get("Inherits", "/a").build().unwrap()).unwrap();
    assert_eq!(inherits.endpoints[0].auth, None);
    assert_eq!(inherits.endpoint_auth(&inherits.endpoints[0]), ResolvedEndpointAuth::Inherited {
        policy: inherits.effective_auth_policy(), env_mapping: inherits.default_env_mapping() });

    // A compatible API mapping is reused.
    let reuse = bearer_api(get("Reuse", "/a").auth(bearer_policy()).build().unwrap()).unwrap();
    assert_eq!(reuse.endpoints[0].auth, Some(EndpointAuth::Policy { policy: bearer_policy(), env_mapping: None }));
    assert_eq!(reuse.endpoint_auth(&reuse.endpoints[0]), ResolvedEndpointAuth::Replaced {
        policy: bearer_policy(), env_mapping: reuse.default_env_mapping() });

    // A bearer endpoint on a Basic API names its own source, replacing the API mapping whole.
    let own = EnvMapping { bearer_token: Some(EnvList::single("ENDPOINT_TOKEN")), ..Default::default() };
    let distinct = basic_api(get("Distinct", "/a").auth_with_env_mapping(bearer_policy(), own.clone()).build().unwrap()).unwrap();
    assert_eq!(distinct.endpoint_auth(&distinct.endpoints[0]), ResolvedEndpointAuth::Replaced { policy: bearer_policy(), env_mapping: own });

    // Header names compare without regard to case; explicit-only policies need no source.
    let key_policy = AuthPolicy::from_auth_strategy(&AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: None });
    api_key_api("x-key", get("Key", "/a").auth(key_policy).build().unwrap()).unwrap();
    let explicit_only = AuthPolicy { explicit: vec![AuthMethod::Basic], env_fallback: None };
    bearer_api(get("Explicit", "/a").auth(explicit_only).build().unwrap()).unwrap();

    // A parameter key reuses only the API's identical fallback and its env_auth.
    let ctx = DefinitionContext::types_in("::models");
    RestApi::builder(&ctx, "Param", "https://example.com").describes("Param API").authentication(query_key(), ["KEY"])
        .endpoints([get("Param", "/a").auth(AuthPolicy::from_auth_strategy(&query_key())).build().unwrap()]).build().unwrap();
}

#[test]
fn incompatible_or_missing_credential_sources_fail_naming_api_endpoint_and_method() {
    let basic = AuthPolicy::from_auth_strategy(&AuthStrategy::Basic { username_env: EnvList::new(vec![]), password_env: EnvList::new(vec![]) });
    let key_policy = AuthPolicy::from_auth_strategy(&AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: None });
    let param = AuthPolicy::from_auth_strategy(&query_key());
    let cases = [
        (basic_api(get("Bearer", "/a").auth(bearer_policy()).build().unwrap()),
            "Basic endpoint Bearer: endpoint auth policy falls back to bearer token environment credentials, but the API's effective env mapping has no `bearer_token` variables"),
        (api_key_api("X-Other", get("Key", "/a").auth(key_policy.clone()).build().unwrap()),
            "Keyed endpoint Key: endpoint auth policy falls back to API key environment credentials, but the API's effective env mapping has `api_key` for header `X-Other`, not `X-Key`"),
        (bearer_api(get("Key", "/a").auth(key_policy).build().unwrap()),
            "Bearer endpoint Key: endpoint auth policy falls back to API key environment credentials, but the API's effective env mapping has no `api_key` variables"),
        (bearer_api(get("Basic", "/a").auth(basic.clone()).build().unwrap()),
            "Bearer endpoint Basic: endpoint auth policy falls back to Basic environment credentials, but the API's effective env mapping has no `basic_user` or `basic_pass` variables"),
        (bearer_api(get("Basic", "/a").auth_with_env_mapping(basic, EnvMapping { basic_user: Some(EnvList::single("U")), ..Default::default() }).build().unwrap()),
            "Bearer endpoint Basic: endpoint auth policy falls back to Basic environment credentials, but the endpoint env_mapping has no `basic_pass` variables"),
        (bearer_api(get("Param", "/a").auth(param.clone()).build().unwrap()),
            "Bearer endpoint Param: endpoint auth policy falls back to a parameter API key from the environment"),
    ];
    for (result, expected) in cases {
        let error = message(result);
        assert!(error.starts_with(expected), "{error}");
    }
    let ctx = DefinitionContext::types_in("::models");
    let with_mapping = RestApi::builder(&ctx, "Param", "https://example.com").describes("Param API").authentication(query_key(), ["KEY"])
        .endpoints([get("Param", "/a").auth_with_env_mapping(param, EnvMapping::default()).build().unwrap()]).build();
    assert!(message(with_mapping).contains("an env_mapping cannot"));
}

#[test]
fn malformed_endpoint_auth_declarations_fail() {
    assert_eq!(message(get("Twice", "/a").public().auth(bearer_policy()).build()),
        "<endpoint> endpoint Twice: endpoint authentication must be declared at most once (public, auth, or auth_with_env_mapping)");
    assert_eq!(message(get("Empty", "/a").auth(AuthPolicy::default()).build()),
        "<endpoint> endpoint Empty: endpoint auth policy accepts no authentication method; use public() for an unauthenticated endpoint");

    let bad_grant = OAuth2Config::ClientCredentials(ClientCredentialsGrant {
        token_url: "not a url".into(), revocation_url: None, default_scopes: vec![], client_auth: OAuth2ClientAuthMethod::ClientSecretPost });
    let oauth = AuthPolicy { explicit: vec![AuthMethod::OAuth2(bad_grant)], env_fallback: None };
    assert!(message(get("OAuth", "/a").auth(oauth.clone()).build()).starts_with("<endpoint> endpoint OAuth: endpoint auth policy: "));
    let mut endpoint = get("OAuth", "/a").build().unwrap();
    endpoint.auth = Some(EndpointAuth::Policy { policy: oauth.clone(), env_mapping: None });
    assert!(message(bearer_api(endpoint)).starts_with("Bearer endpoint OAuth: endpoint auth policy: "));

    // The API's own auth policy is checked too.
    let ctx = DefinitionContext::types_in("::models");
    let error = message(RestApi::builder(&ctx, "Api", "https://example.com").describes("Api").no_auth().auth_policy(oauth).build());
    assert!(error.starts_with("Api endpoint <api>: auth_policy: "), "{error}");
}

#[test]
fn endpoint_auth_serde_contract() {
    assert_eq!(get("Get", "/a").build().unwrap().auth, None);
    assert_eq!(endpoint_with("auth", json!("Public")).unwrap().auth, Some(EndpointAuth::Public));
    let mapping = EnvMapping { bearer_token: Some(EnvList::single("T")), ..Default::default() };
    for auth in [EndpointAuth::Public, EndpointAuth::Policy { policy: bearer_policy(), env_mapping: None },
        EndpointAuth::Policy { policy: bearer_policy(), env_mapping: Some(mapping) }] {
        let json = serde_json::to_value(&auth).unwrap();
        assert_eq!(serde_json::from_value::<EndpointAuth>(json.clone()).unwrap(), auth);
        assert_eq!(endpoint_with("auth", json).unwrap().auth, Some(auth));
    }
    assert_eq!(serde_json::to_value(EndpointAuth::Policy { policy: bearer_policy(), env_mapping: None }).unwrap(),
        json!({"Policy": {"policy": serde_json::to_value(bearer_policy()).unwrap()}}));
    for bad in [json!(null), json!({}), json!("Private"), json!([]), json!({"Policy": {}}),
        json!({"Policy": {"policy": {}, "extra": 1}}), json!({"Policy": {"policy": {}, "env_mapping": null}}),
        json!({"Public": {}, "Policy": {"policy": {}}})] {
        assert!(endpoint_with("auth", bad.clone()).is_err(), "{bad}");
    }
}

// ---------------------------------------------------------------------------
// Pager-managed required parameters (R14)
// ---------------------------------------------------------------------------

fn listed(param: ParamDef, size: PageSize) -> Result<RestApi, DefinitionError> {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::get("ListItems", "/items").describes("List").returns_json::<Vec<Item>>(&mut ctx).query(param).build().unwrap();
    let strategy = Strategy::cursor("after", CursorSource::body("next")).page_size(size).build();
    RestApi::builder(&ctx, "Pager", "https://example.com").describes("Pager").no_auth()
        .default_pagination(strategy).endpoints([endpoint]).build()
}

#[test]
fn required_pager_managed_parameters_need_a_type_valid_sent_default() {
    let limit = |param_type| ParamDef::new("limit", param_type).required();
    assert_eq!(message(listed(limit(QueryParamType::Integer), PageSize::new("limit").max(100))),
        "Pager endpoint ListItems: pager-managed-required-without-default: required parameter `limit` is pager-managed by the \
         `cursor` strategy, so the strategy must author a value to send when the caller sets none");
    assert_eq!(message(listed(limit(QueryParamType::Enum(vec!["10".into(), "20".into()])), PageSize::new("limit").max(100).sent_default(50))),
        "Pager endpoint ListItems: pager-managed-default-invalid: required parameter `limit` is pager-managed by the `cursor` \
         strategy, whose default `50` is not a valid value of the declared enum");
    assert!(message(listed(limit(QueryParamType::Boolean), PageSize::new("limit").sent_default(50)))
        .ends_with("whose default `50` is not a valid boolean (`true` or `false`)"));

    listed(limit(QueryParamType::Integer), PageSize::new("limit").max(100).sent_default(50)).unwrap();
    listed(limit(QueryParamType::Number), PageSize::new("limit").sent_default(50)).unwrap();
    listed(limit(QueryParamType::String), PageSize::new("limit").sent_default(50)).unwrap();
    listed(limit(QueryParamType::Enum(vec!["50".into()])), PageSize::new("limit").sent_default(50)).unwrap();
    // An optional adopted parameter is not subject to the rule.
    listed(ParamDef::new("limit", QueryParamType::Boolean), PageSize::new("limit").sent_default(50)).unwrap();
}

#[test]
fn page_number_bases_are_checked_against_a_required_page_type() {
    let mut ctx = DefinitionContext::types_in("::models");
    let build = |ctx: &mut DefinitionContext, page_type| {
        let endpoint = Endpoint::get("ListItems", "/items").describes("List").returns_json::<Vec<Item>>(ctx)
            .query(ParamDef::new("page", page_type).required()).build().unwrap();
        RestApi::builder(ctx, "Pager", "https://example.com").describes("Pager").no_auth()
            .default_pagination(Strategy::page_number("page", MoreSignal::NonEmptyPage).build()).endpoints([endpoint]).build()
    };
    build(&mut ctx, QueryParamType::Integer).unwrap();
    assert!(message(build(&mut ctx, QueryParamType::Boolean)).ends_with("by the `page_number` strategy, whose default `1` is not a valid boolean (`true` or `false`)"));
}

// ---------------------------------------------------------------------------
// Byte stability
// ---------------------------------------------------------------------------

#[test]
fn every_new_field_populated_serializes_identically_after_a_read() {
    let endpoint = get("Get", "/{id}")
        .path_param(ParamDef::path("id", QueryParamType::Enum(vec!["a".into(), "b".into()])).describes("Id").deprecated())
        .query(ParamDef::new("q", QueryParamType::String).deprecated())
        .deprecated().rate_limit(rate_limit("ep").with_retry_after_header("retry-after").unwrap()).retry_guidance(retry(4))
        .auth_with_env_mapping(bearer_policy(), EnvMapping { bearer_token: Some(EnvList::single("T")), ..Default::default() })
        .build().unwrap();
    let ctx = DefinitionContext::types_in("::models");
    let api = RestApi::builder(&ctx, "Vocab", "https://example.com").describes("Vocabulary").bearer_env(["TOKEN"])
        .rate_limit(rate_limit("api")).retry_guidance(retry(2)).endpoints([endpoint]).build().unwrap();
    let first = serde_json::to_string(&api).unwrap();
    let read: RestApi = serde_json::from_str(&first).unwrap();
    assert_eq!(read, api);
    assert_eq!(serde_json::to_string(&read).unwrap(), first);
    let auth: EnvAuthStrategy = EnvAuthStrategy::BearerToken { header: None };
    assert_eq!(read.endpoints[0].auth.as_ref().and_then(|auth| match auth {
        EndpointAuth::Policy { policy, .. } => policy.env_fallback.clone(),
        _ => None,
    }), Some(auth));
}
