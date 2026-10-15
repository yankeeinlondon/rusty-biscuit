use super::auth::{AuthConfig, EnvFallback, Placement, auth_config};
use super::*;
use crate::codegen::request_structs::format_generated_code;
use schematic_define::{
    ApiKeyEnv, ApiKeyLocation, AuthMethod, AuthPolicy, AuthStrategy, CrossOrigin, EnvAuthStrategy,
    EnvList, EnvMapping, OAuth2ClientAuthMethod, OAuth2Config, OAuth2GrantType, PkceRequirement,
};

fn make_api(name: &str, auth: AuthStrategy, env_auth: &[&str]) -> RestApi {
    {
        schematic_define::RestApi::builder(&schematic_define::DefinitionContext::types_in("fixture"), name.to_string(), "https://api.example.com/v1".to_string())
            .describes(format!("{name} API"))
            .authentication(auth, env_auth.iter().copied())
            .build().expect("valid API fixture")
    }
}

fn code(api: &RestApi) -> String {
    format_generated_code(&generate_api_struct(api)).expect("generated client parses")
}

/// `text` without whitespace and trailing commas, so an assertion does not
/// depend on where the formatter breaks a line.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<String>().replace(",]", "]").replace(",)", ")")
}

fn oauth() -> OAuth2Config {
    OAuth2Config {
        grant_type: OAuth2GrantType::AuthorizationCodePkce,
        authorization_url: Some("https://auth.example.com/authorize".to_string()),
        token_url: "https://auth.example.com/token".to_string(),
        revocation_url: None,
        device_authorization_url: None,
        default_scopes: vec![],
        pkce: PkceRequirement::Required,
        client_auth: OAuth2ClientAuthMethod::None,
    }
}

fn env(placement: Placement, vars: &[&str], username_vars: &[&str]) -> Option<EnvFallback> {
    Some(EnvFallback {
        placement,
        vars: vars.iter().map(|var| var.to_string()).collect(),
        username_vars: username_vars.iter().map(|var| var.to_string()).collect(),
    })
}

#[test]
fn an_api_is_one_client_struct_over_a_tag_with_constants_and_an_infallible_new() {
    let mut api = make_api("OpenAi", AuthStrategy::None, &[]);
    api.docs_url = Some("https://docs.example.com".to_string());
    let code = code(&api);

    for expected in [
        "#[derive(Clone, Debug)]\npub struct OpenAi(schematic_runtime::Client<OpenAiApi>);",
        "pub enum OpenAiApi {}",
        "impl schematic_runtime::Api for OpenAiApi",
        "const NAME: &'static str = \"OpenAi\";",
        "schematic_runtime::api_client!(OpenAi, OpenAiApi);",
        "pub const BASE_URL: &'static str = \"https://api.example.com/v1\";",
        "pub const DOCS_URL: Option<&'static str> = Some(\"https://docs.example.com\");",
        "pub fn definition() -> schematic_runtime::ApiDefinition",
        "schematic_runtime::ApiDefinition::new(Self::BASE_URL)",
        "pub fn new() -> Self",
        "schematic_runtime::Client::new(Self::definition())",
        "impl Default for OpenAi",
    ] {
        assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
    }
    // One user-facing name: no alias, no builder, and no forwarded setting
    // written out per API.
    for removed in ["OpenAiClient", "pub type", "fn builder", "ClientBuilder", "BuildError", "fn base_url", "fn send"] {
        assert!(!code.contains(removed), "`{removed}` is still emitted:\n{code}");
    }
}

#[test]
fn basic_auth_is_emitted_only_for_an_api_that_accepts_basic() {
    let basic = code(&make_api("Basic", AuthStrategy::Basic, &["BASIC_PASS"]));
    assert!(
        squash(&basic).contains(&squash(
            "pub fn basic_auth(self, username: impl Into<String>, password: impl Into<String>) -> Self { \
             self.credentials(schematic_runtime::Credential::basic(username, password)) }"
        )),
        "{basic}"
    );

    let mut mixed = make_api("Mixed", AuthStrategy::BearerToken { header: None }, &[]);
    mixed.auth_policy = Some(AuthPolicy {
        explicit: vec![AuthMethod::BearerToken { header: None }, AuthMethod::Basic],
        env_fallback: None,
    });
    assert!(code(&mixed).contains("pub fn basic_auth("), "an API accepting Basic among others gets it");

    for api in [
        make_api("Bearer", AuthStrategy::BearerToken { header: None }, &["TOKEN"]),
        make_api("Keyed", AuthStrategy::ApiKey { header: "x-api-key".to_string(), value_prefix: None }, &["KEY"]),
        make_api("Open", AuthStrategy::None, &[]),
    ] {
        let code = code(&api);
        assert!(!code.contains("basic_auth"), "{} takes no Basic credential:\n{code}", api.name);
    }
}

#[test]
fn the_client_has_no_own_transport_send_path_or_credential_store() {
    let mut api = make_api("Thin", AuthStrategy::BearerToken { header: None }, &["THIN_TOKEN"]);
    api.auth_policy = Some(AuthPolicy {
        explicit: vec![
            AuthMethod::ApiKeyParam {
                name: "key".to_string(),
                location: ApiKeyLocation::Query,
            },
            AuthMethod::OAuth2(oauth()),
        ],
        env_fallback: None,
    });
    let code = code(&api);

    for removed in [
        "reqwest",
        "with_client",
        "with_base_url",
        "variant",
        "fn request",
        "with_api_key_param",
        "api_key_header",
        "std::env",
        "SchematicError",
        "ResponseKind",
        "schematic_define",
        "bearer_token(",
        "oauth_token(",
    ] {
        assert!(!code.contains(removed), "`{removed}` is still emitted:\n{code}");
    }
}

#[test]
fn default_headers_seed_new() {
    let mut api = make_api("Versioned", AuthStrategy::None, &[]);
    api.headers = vec![
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
        ("Accept".to_string(), "application/json".to_string()),
    ];
    let code = code(&api);

    assert!(code.contains(".default_header(\"anthropic-version\", \"2023-06-01\")"), "{code}");
    assert!(code.contains(".default_header(\"Accept\", \"application/json\")"), "{code}");

    // The runtime refuses a credential-bearing default header, so an API
    // cookie is sent with each request instead.
    api.headers = vec![("Cookie".to_string(), "lang=en".to_string())];
    let code = self::code(&api);
    assert!(!code.contains("default_header"), "{code}");
}

#[test]
fn every_legacy_strategy_translates_to_the_same_placement_and_variables() {
    let bearer = |header: Option<&str>| Placement::Bearer {
        header: header.map(str::to_string),
    };
    let mut basic = make_api("Basic", AuthStrategy::Basic, &["PASS"]);
    basic.env_username = Some("USER".to_string());
    let cases: Vec<(&str, RestApi, AuthConfig)> = vec![
        ("none", make_api("A", AuthStrategy::None, &["IGNORED"]), AuthConfig::default()),
        (
            "bearer",
            make_api("A", AuthStrategy::BearerToken { header: None }, &["T1", "T2"]),
            AuthConfig {
                accepted: vec![bearer(None)],
                env: env(bearer(None), &["T1", "T2"], &[]),
            },
        ),
        (
            "bearer in a custom header",
            make_api(
                "A",
                AuthStrategy::BearerToken {
                    header: Some("X-Token".to_string()),
                },
                &["T"],
            ),
            AuthConfig {
                accepted: vec![bearer(Some("X-Token"))],
                env: env(bearer(Some("X-Token")), &["T"], &[]),
            },
        ),
        (
            "API-key header with a prefix",
            make_api(
                "A",
                AuthStrategy::ApiKey {
                    header: "Authorization".to_string(),
                    value_prefix: Some("token ".to_string()),
                },
                &["KEY"],
            ),
            AuthConfig {
                accepted: vec![Placement::ApiKeyHeader {
                    header: "Authorization".to_string(),
                    prefix: Some("token ".to_string()),
                }],
                env: env(
                    Placement::ApiKeyHeader {
                        header: "Authorization".to_string(),
                        prefix: Some("token ".to_string()),
                    },
                    &["KEY"],
                    &[],
                ),
            },
        ),
        (
            "API-key query parameter",
            make_api(
                "A",
                AuthStrategy::ApiKeyParam {
                    name: "api_key".to_string(),
                    location: ApiKeyLocation::Query,
                },
                &["Q1", "Q2"],
            ),
            AuthConfig {
                accepted: vec![Placement::ApiKeyQuery {
                    name: "api_key".to_string(),
                }],
                env: env(
                    Placement::ApiKeyQuery {
                        name: "api_key".to_string(),
                    },
                    &["Q1", "Q2"],
                    &[],
                ),
            },
        ),
        (
            "API-key cookie",
            make_api(
                "A",
                AuthStrategy::ApiKeyParam {
                    name: "sid".to_string(),
                    location: ApiKeyLocation::Cookie,
                },
                &["C"],
            ),
            AuthConfig {
                accepted: vec![Placement::ApiKeyCookie {
                    name: "sid".to_string(),
                }],
                env: env(
                    Placement::ApiKeyCookie {
                        name: "sid".to_string(),
                    },
                    &["C"],
                    &[],
                ),
            },
        ),
        (
            "Basic, username and password read independently",
            basic,
            AuthConfig {
                accepted: vec![Placement::Basic],
                env: env(Placement::Basic, &["PASS"], &["USER"]),
            },
        ),
        (
            "OAuth2 is an access token supplied out of band, never read from the environment",
            make_api("A", AuthStrategy::OAuth2(oauth()), &["NOT_READ"]),
            AuthConfig {
                accepted: vec![bearer(None)],
                env: None,
            },
        ),
    ];
    for (label, api, expected) in cases {
        assert_eq!(auth_config(&api), expected, "{label}");
    }
}

#[test]
fn an_explicit_policy_wins_over_the_legacy_strategy() {
    let mut api = make_api("A", AuthStrategy::BearerToken { header: None }, &["LEGACY"]);
    api.auth_policy = Some(AuthPolicy {
        explicit: vec![
            AuthMethod::ApiKey {
                header: "X-Api-Key".to_string(),
                value_prefix: None,
            },
            AuthMethod::OAuth2(oauth()),
            AuthMethod::BearerToken { header: None },
        ],
        env_fallback: Some(EnvAuthStrategy::ApiKey {
            header: "X-Api-Key".to_string(),
            value_prefix: None,
        }),
    });
    api.env_mapping = Some(EnvMapping {
        api_key: Some(ApiKeyEnv {
            names: EnvList::new(vec!["FIRST".to_string(), "SECOND".to_string()]),
            header: "X-Api-Key".to_string(),
        }),
        ..EnvMapping::default()
    });

    let header = Placement::ApiKeyHeader {
        header: "X-Api-Key".to_string(),
        prefix: None,
    };
    assert_eq!(
        auth_config(&api),
        AuthConfig {
            // The OAuth2 method and the bearer method place the same
            // credential, so the duplicate is listed once.
            accepted: vec![header.clone(), Placement::Bearer { header: None }],
            env: env(header, &["FIRST", "SECOND"], &[]),
        }
    );
}

#[test]
fn a_mapping_that_names_no_variable_reads_none() {
    // LM Studio and Eversolo set an empty mapping on purpose: their
    // `env_auth` documents a variable that the client never reads.
    let mut api = make_api("A", AuthStrategy::BearerToken { header: None }, &["LM_API_TOKEN"]);
    api.env_mapping = Some(EnvMapping::default());
    assert_eq!(auth_config(&api).env, None);
    assert!(!code(&api).contains("env_fallback"), "{}", code(&api));

    let mut basic = make_api("B", AuthStrategy::Basic, &["PASS"]);
    basic.env_mapping = Some(EnvMapping {
        basic_user: Some(EnvList::single("USER")),
        ..EnvMapping::default()
    });
    assert_eq!(auth_config(&basic).env, None, "Basic without a password variable reads none");
}

#[test]
fn the_translated_policy_is_emitted_in_runtime_terms() {
    let mut basic = make_api("Basic", AuthStrategy::Basic, &["PASS"]);
    basic.env_username = Some("USER".to_string());
    let code = code(&basic);
    for expected in [
        ".auth(",
        "schematic_runtime::AuthConfig::none()",
        ".accept(schematic_runtime::CredentialPlacement::Basic)",
        "schematic_runtime::EnvFallback::new(",
        "[\"PASS\"]",
        ".username_vars([\"USER\"])",
    ] {
        assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
    }

    let none = self::code(&make_api("Open", AuthStrategy::None, &[]));
    assert!(!none.contains(".auth("), "an API without credentials keeps the runtime default:\n{none}");
}

#[test]
fn every_cross_origin_policy_and_the_redaction_settings_reach_the_definition() {
    let cases = [
        (CrossOrigin::same_origin(), None),
        (
            CrossOrigin::allow_origins(["https://uploads.example.com"]),
            Some("schematic_runtime::OriginPolicy::allow_origins([\"https://uploads.example.com\"])"),
        ),
        (
            CrossOrigin::allow_without_credentials(),
            Some("schematic_runtime::OriginPolicy::allow_without_credentials()"),
        ),
        (
            CrossOrigin::danger_allow_any(),
            Some("schematic_runtime::OriginPolicy::danger_allow_any()"),
        ),
        (CrossOrigin::deny(), Some("schematic_runtime::OriginPolicy::deny()")),
    ];
    for (policy, expected) in cases {
        let mut api = make_api("A", AuthStrategy::None, &[]);
        api.cross_origin = policy.clone();
        let code = code(&api);
        match expected {
            Some(expected) => assert!(
                squash(&code).contains(&squash(&format!(".cross_origin({expected})"))),
                "{policy:?}:\n{code}"
            ),
            None => assert!(!code.contains(".cross_origin("), "{policy:?} is the runtime default:\n{code}"),
        }
    }

    let mut api = make_api("A", AuthStrategy::None, &[]);
    assert!(!code(&api).contains(".secret_names("));
    assert!(!code(&api).contains(".request_id_header("));
    api.secret_names = vec!["X-Pin".to_string(), "sig".to_string()];
    api.request_id_header = Some("x-request-id".to_string());
    let code = code(&api);
    assert!(code.contains(".secret_names([\"X-Pin\", \"sig\"])"), "{code}");
    assert!(code.contains(".request_id_header(\"x-request-id\")"), "{code}");
}

#[test]
fn endpoint_origin_replacement_and_parameter_secrets_use_runtime_metadata() {
    let mut api = make_api("Fixture", AuthStrategy::None, &[]);
    let mut endpoint = schematic_define::Endpoint::get("List", "/list")
        .describes("List entries").returns_empty()
        .build().unwrap();
    endpoint.cross_origin = Some(CrossOrigin::same_origin());
    let secret = schematic_define::ParamDef {
        name: "X-Pin".into(), rust_name: None, secret: true, pager_managed: false,
        required: false, description: None, param_type: schematic_define::QueryParamType::String,
        explode: false, style: schematic_define::ParamStyle::Simple,
    };
    endpoint.params = Some(schematic_define::EndpointParams { header: vec![secret], ..Default::default() });
    api.endpoints.push(endpoint);
    let generated = code(&api);
    assert!(squash(&generated).contains(".endpoint_cross_origin(\"List\",schematic_runtime::OriginPolicy::same_origin())"), "{generated}");
    assert!(generated.contains(".secret_names([\"X-Pin\"])"), "{generated}");
    api.endpoints[0].cross_origin = Some(CrossOrigin::allow_origins(Vec::<String>::new()));
    let generated = code(&api);
    assert!(squash(&generated).contains(".endpoint_cross_origin(\"List\",schematic_runtime::OriginPolicy::allow_origins(Vec::<String>::new()))"), "empty origins keep concrete type: {generated}");
}
