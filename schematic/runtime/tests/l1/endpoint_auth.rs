//! Per-endpoint authentication against a local mock: a public endpoint sends
//! no credential from any source and strips every known auth location, an
//! endpoint policy replaces the API's with its own placements and sources,
//! and the refusals a replacement adds (R2 of the definition vocabulary).

use http::Method;
use schematic_runtime::{
    ApiDefinition, AuthConfig, AuthMethod, BuildError, Client, Credential, CredentialPlacement, EndpointSpec,
    EnvFallback, InvalidArgument, Json, NoDeclaredError, OriginPolicy, RequestParts, TransportError,
    TransportErrorKind,
};
use test_toolkit::EnvGuard;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Item, SENTINEL, TestApi, builder};

const UNUSED: &str = "https://unused.invalid";

/// Declares an endpoint of [`TestApi`] with its own id, sending whatever
/// parts it was built with.
macro_rules! endpoint {
    ($name:ident, $id:literal) => {
        struct $name(RequestParts);

        impl EndpointSpec for $name {
            type Api = TestApi;
            type Success = Item;
            type Failure = NoDeclaredError;
            type Decoder = Json<Item>;

            const ENDPOINT_ID: &'static str = $id;
            const METHOD: Method = Method::GET;

            fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
                Ok(self.0)
            }
        }
    };
}

endpoint!(Inherited, "Inherited");
endpoint!(Public, "Public");
endpoint!(Replaced, "Replaced");
endpoint!(QueryReplaced, "QueryReplaced");
endpoint!(CookieReplaced, "CookieReplaced");

fn bearer() -> CredentialPlacement {
    CredentialPlacement::Bearer { header: None }
}

fn get(path: &str) -> RequestParts {
    RequestParts::new(Method::GET, path)
}

/// A bearer API reading `token_var`, whose `Public` endpoint is public and
/// whose `Replaced` endpoint takes Basic credentials from its own variables.
fn mixed_api(token_var: &str, user_var: &str, pass_var: &str) -> ApiDefinition {
    ApiDefinition::new(UNUSED)
        .auth(AuthConfig::none().accept(bearer()).env_fallback(EnvFallback::new(bearer(), [token_var])))
        .endpoint_public("Public")
        .endpoint_auth(
            "Replaced",
            AuthConfig::none()
                .accept(CredentialPlacement::Basic)
                .env_fallback(EnvFallback::new(CredentialPlacement::Basic, [pass_var]).username_vars([user_var])),
        )
}

async fn ok_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"id":1}"#, "application/json"))
        .mount(&server)
        .await;
    server
}

async fn received(server: &MockServer) -> Vec<wiremock::Request> {
    server.received_requests().await.expect("recording is on")
}

fn authorization(request: &wiremock::Request) -> Vec<String> {
    request
        .headers
        .get_all("authorization")
        .iter()
        .map(|value| value.to_str().expect("ascii").to_string())
        .collect()
}

fn basic(username: &str, password: &str) -> String {
    use base64::Engine;
    format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}")))
}

fn invalid_argument(error: &TransportError) -> &InvalidArgument {
    match &error.kind {
        TransportErrorKind::InvalidArgument(argument) => argument,
        other => panic!("expected an invalid argument, got {other:?}"),
    }
}

#[tokio::test]
async fn a_public_endpoint_sends_no_credential_from_any_source_and_strips_every_known_auth_location() {
    let var = "SCHEMATIC_RT_TEST_PUBLIC_ENV_KEY";
    let _env = EnvGuard::set_safe(var, SENTINEL);
    let api = ApiDefinition::new(UNUSED)
        .auth(
            AuthConfig::none()
                .accept(bearer())
                .accept(CredentialPlacement::ApiKeyQuery { name: "key".into() })
                .accept(CredentialPlacement::ApiKeyCookie { name: "sid".into() })
                .env_fallback(EnvFallback::new(
                    CredentialPlacement::ApiKeyHeader {
                        header: "X-Vendor-Key".into(),
                        prefix: None,
                    },
                    [var],
                )),
        )
        .endpoint_public("Public")
        // Locations only an endpoint replacement uses are stripped too.
        .endpoint_auth(
            "Replaced",
            AuthConfig::none()
                .accept(CredentialPlacement::ApiKeyQuery { name: "endpoint_key".into() })
                .accept(CredentialPlacement::ApiKeyHeader {
                    header: "X-Endpoint-Key".into(),
                    prefix: None,
                }),
        );
    let server = ok_server().await;
    let with_credential = builder(api.clone(), &server.uri())
        .credentials(Credential::access_token(SENTINEL))
        .build()
        .expect("client");
    let with_key = builder(api.clone(), &server.uri()).api_key(SENTINEL).build().expect("client");
    let mut parts = get("/public");
    parts.query = vec![
        ("key".into(), SENTINEL.into()),
        ("endpoint_key".into(), SENTINEL.into()),
        ("page".into(), "2".into()),
    ];
    parts.headers = vec![
        ("Authorization".into(), format!("Bearer {SENTINEL}")),
        ("X-Vendor-Key".into(), SENTINEL.into()),
        ("X-Endpoint-Key".into(), SENTINEL.into()),
        ("Cookie".into(), format!("a=1; sid={SENTINEL}; b=2")),
        ("X-Trace".into(), "t1".into()),
    ];

    for client in [&with_credential, &with_key] {
        let plain = client.send(Public(parts.clone())).full().await.expect("head");
        assert_eq!(plain.meta.request.auth, AuthMethod::Anonymous);
        let per_call = client
            .send(Public(parts.clone()))
            .credential(Credential::access_token(SENTINEL))
            .full()
            .await
            .expect("a per-call credential is dropped, not refused");
        assert_eq!(per_call.meta.request.auth, AuthMethod::Anonymous);
    }

    let requests = received(&server).await;
    assert_eq!(requests.len(), 4);
    for request in &requests {
        assert!(authorization(request).is_empty());
        assert!(request.headers.get("x-vendor-key").is_none());
        assert!(request.headers.get("x-endpoint-key").is_none());
        assert_eq!(request.headers["cookie"], "a=1; b=2", "ordinary cookies are kept");
        assert_eq!(request.headers["x-trace"], "t1", "ordinary headers are kept");
        assert_eq!(request.url.query(), Some("page=2"));
    }
}

#[tokio::test]
async fn an_authenticated_call_after_a_public_call_still_sends_its_credential() {
    let token_var = "SCHEMATIC_RT_TEST_AFTER_PUBLIC_TOKEN";
    let _token = EnvGuard::set_safe(token_var, "env-token");
    let server = ok_server().await;
    let client = builder(mixed_api(token_var, "SCHEMATIC_RT_TEST_AFTER_PUBLIC_USER", "SCHEMATIC_RT_TEST_AFTER_PUBLIC_PASS"), &server.uri())
        .build()
        .expect("client");
    let with_credential = client.clone().credentials(Credential::access_token("client-token"));

    client.send(Public(get("/public"))).full().await.expect("head");
    let from_env = client.send(Inherited(get("/inherited"))).full().await.expect("head");
    with_credential.send(Public(get("/public"))).full().await.expect("head");
    let from_client = with_credential.send(Inherited(get("/inherited"))).full().await.expect("head");

    assert_eq!(from_env.meta.request.auth, AuthMethod::EnvVar(token_var.into()));
    assert_eq!(from_client.meta.request.auth, AuthMethod::ClientLevel);
    let sent: Vec<_> = received(&server).await.iter().map(authorization).collect();
    assert_eq!(
        sent,
        [vec![], vec!["Bearer env-token".to_string()], vec![], vec!["Bearer client-token".to_string()]]
    );
}

#[tokio::test]
async fn a_replacement_reads_its_own_environment_source_instead_of_the_apis() {
    let token_var = "SCHEMATIC_RT_TEST_DISTINCT_TOKEN";
    let user_var = "SCHEMATIC_RT_TEST_DISTINCT_USER";
    let pass_var = "SCHEMATIC_RT_TEST_DISTINCT_PASS";
    let _token = EnvGuard::set_safe(token_var, "api-token");
    let _user = EnvGuard::set_safe(user_var, "alice");
    let _pass = EnvGuard::set_safe(pass_var, "endpoint-pass");
    let server = ok_server().await;
    let client = builder(mixed_api(token_var, user_var, pass_var), &server.uri()).build().expect("client");

    let replaced = client.send(Replaced(get("/replaced"))).full().await.expect("head");
    let inherited = client.send(Inherited(get("/inherited"))).full().await.expect("head");

    assert_eq!(replaced.meta.request.auth, AuthMethod::EnvVar(pass_var.into()));
    assert_eq!(inherited.meta.request.auth, AuthMethod::EnvVar(token_var.into()));
    let requests = received(&server).await;
    assert_eq!(authorization(&requests[0]), [basic("alice", "endpoint-pass")]);
    assert_eq!(authorization(&requests[1]), ["Bearer api-token"]);
}

#[tokio::test]
async fn query_and_cookie_replacements_place_the_endpoint_credential_where_the_endpoint_expects_it() {
    let var = "SCHEMATIC_RT_TEST_COOKIE_REPLACEMENT_KEY";
    let _env = EnvGuard::set_safe(var, "cookie-key");
    let query = CredentialPlacement::ApiKeyQuery { name: "endpoint_key".into() };
    let cookie = CredentialPlacement::ApiKeyCookie { name: "session".into() };
    let api = ApiDefinition::new(UNUSED)
        .auth(AuthConfig::none().accept(bearer()))
        .endpoint_auth("QueryReplaced", AuthConfig::none().accept(query))
        .endpoint_auth("CookieReplaced", AuthConfig::none().env_fallback(EnvFallback::new(cookie, [var])));
    let server = ok_server().await;
    let client = builder(api, &server.uri()).build().expect("client");

    let per_call = client
        .send(QueryReplaced(get("/query")))
        .credential(Credential::api_key("call-key"))
        .full()
        .await
        .expect("head");
    let mut parts = get("/cookie");
    parts.headers = vec![("Cookie".into(), "a=1".into())];
    let from_env = client.send(CookieReplaced(parts)).full().await.expect("head");

    assert_eq!(per_call.meta.request.auth, AuthMethod::PerCall);
    assert_eq!(from_env.meta.request.auth, AuthMethod::EnvVar(var.into()));
    let requests = received(&server).await;
    assert_eq!(requests[0].url.query(), Some("endpoint_key=call-key"));
    assert!(authorization(&requests[0]).is_empty());
    assert_eq!(requests[1].headers["cookie"], "a=1; session=cookie-key");
    assert!(authorization(&requests[1]).is_empty());
}

#[tokio::test]
async fn a_per_call_credential_the_replacement_does_not_accept_is_refused_before_any_request() {
    let server = ok_server().await;
    let client = builder(
        mixed_api("SCHEMATIC_RT_TEST_PER_CALL_TOKEN", "SCHEMATIC_RT_TEST_PER_CALL_USER", "SCHEMATIC_RT_TEST_PER_CALL_PASS"),
        &server.uri(),
    )
    .build()
    .expect("client");

    let error = client
        .send(Replaced(get("/replaced")))
        .credential(Credential::access_token(SENTINEL))
        .full()
        .await
        .expect_err("a bearer token on a Basic endpoint");

    invalid_argument(&error);
    assert!(!format!("{error:?}").contains(SENTINEL));
    assert!(!error.to_string().contains(SENTINEL));
    assert_eq!(received(&server).await.len(), 0);
}

#[tokio::test]
async fn a_replacement_with_nothing_resolving_is_refused_while_an_inherited_endpoint_goes_anonymous() {
    let token_var = "SCHEMATIC_RT_TEST_MISSING_TOKEN";
    let user_var = "SCHEMATIC_RT_TEST_MISSING_USER";
    let pass_var = "SCHEMATIC_RT_TEST_MISSING_PASS";
    let _unset = [token_var, user_var, pass_var].map(EnvGuard::remove_safe);
    let server = ok_server().await;
    let client = builder(mixed_api(token_var, user_var, pass_var), &server.uri()).build().expect("client");

    let error = client.send(Replaced(get("/replaced"))).full().await.expect_err("missing source");
    let message = invalid_argument(&error).to_string();
    assert!(message.contains("`Replaced`"), "{message}");
    assert!(message.contains("Basic"), "{message}");
    assert_eq!(received(&server).await.len(), 0, "refused before sending");

    let inherited = client.send(Inherited(get("/inherited"))).full().await.expect("head");
    assert_eq!(inherited.meta.request.auth, AuthMethod::Anonymous);
    let anonymous = client.send(Replaced(get("/replaced"))).anonymous().full().await.expect("asked for none");
    assert_eq!(anonymous.meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(received(&server).await.len(), 2);
}

#[tokio::test]
async fn a_client_bearer_token_is_never_reinterpreted_as_a_replacements_basic_credential() {
    let user_var = "SCHEMATIC_RT_TEST_NO_REINTERPRET_USER";
    let pass_var = "SCHEMATIC_RT_TEST_NO_REINTERPRET_PASS";
    let _user = EnvGuard::remove_safe(user_var);
    let _pass = EnvGuard::remove_safe(pass_var);
    let server = ok_server().await;
    let client = builder(mixed_api("SCHEMATIC_RT_TEST_NO_REINTERPRET_TOKEN", user_var, pass_var), &server.uri())
        .credentials(Credential::access_token(SENTINEL))
        .build()
        .expect("client");

    let error = client.send(Replaced(get("/replaced"))).full().await.expect_err("the token is not Basic");
    invalid_argument(&error);
    assert_eq!(received(&server).await.len(), 0);

    let _user = EnvGuard::set_safe(user_var, "alice");
    let _pass = EnvGuard::set_safe(pass_var, "env-pass");
    let response = client.send(Replaced(get("/replaced"))).full().await.expect("head");
    assert_eq!(response.meta.request.auth, AuthMethod::EnvVar(pass_var.into()));
    assert_eq!(authorization(&received(&server).await[0]), [basic("alice", "env-pass")]);
}

#[tokio::test]
async fn a_client_credential_only_a_replacement_accepts_builds_and_is_sent_only_there() {
    let token_var = "SCHEMATIC_RT_TEST_ONLY_REPLACEMENT_TOKEN";
    let _unset = EnvGuard::remove_safe(token_var);
    let server = ok_server().await;
    let client = builder(
        mixed_api(token_var, "SCHEMATIC_RT_TEST_ONLY_REPLACEMENT_USER", "SCHEMATIC_RT_TEST_ONLY_REPLACEMENT_PASS"),
        &server.uri(),
    )
    .credentials(Credential::basic("alice", "client-pass"))
    .build()
    .expect("a replacement accepts Basic");

    let replaced = client.send(Replaced(get("/replaced"))).full().await.expect("head");
    let inherited = client.send(Inherited(get("/inherited"))).full().await.expect("head");

    assert_eq!(replaced.meta.request.auth, AuthMethod::ClientLevel);
    assert_eq!(inherited.meta.request.auth, AuthMethod::Anonymous);
    let requests = received(&server).await;
    assert_eq!(authorization(&requests[0]), [basic("alice", "client-pass")]);
    assert!(authorization(&requests[1]).is_empty());

    let error = builder(ApiDefinition::new(UNUSED).endpoint_public("Public"), UNUSED)
        .credentials(Credential::basic("alice", SENTINEL))
        .build()
        .expect_err("no policy accepts Basic");
    assert_eq!(error, BuildError::IncompatibleCredential { supplied: "Basic" });
}

#[tokio::test]
async fn an_api_key_takes_a_replacements_placement_when_the_api_has_none() {
    let api = ApiDefinition::new(UNUSED)
        .auth(AuthConfig::none().accept(CredentialPlacement::Basic))
        .endpoint_auth("QueryReplaced", AuthConfig::none().accept(CredentialPlacement::ApiKeyQuery { name: "endpoint_key".into() }));
    let server = ok_server().await;
    let client: Client<TestApi> = builder(api, &server.uri()).api_key("client-key").build().expect("client");

    client.send(QueryReplaced(get("/query"))).full().await.expect("head");
    client.send(Inherited(get("/inherited"))).full().await.expect("head");

    let requests = received(&server).await;
    assert_eq!(requests[0].url.query(), Some("endpoint_key=client-key"));
    assert!(authorization(&requests[1]).is_empty(), "a key is never sent as Basic");
}

#[tokio::test]
async fn replacement_locations_are_secret_in_stored_metadata_and_refused_as_default_headers() {
    let api = ApiDefinition::new(UNUSED).endpoint_auth(
        "QueryReplaced",
        AuthConfig::none()
            .accept(CredentialPlacement::ApiKeyQuery { name: "endpoint_key".into() })
            .accept(CredentialPlacement::ApiKeyHeader {
                header: "X-Endpoint-Key".into(),
                prefix: None,
            }),
    );
    let server = ok_server().await;
    let client = builder(api.clone(), &server.uri()).build().expect("client");

    let response = client
        .send(QueryReplaced(get("/query")))
        .credential(Credential::api_key(SENTINEL))
        .full()
        .await
        .expect("head");

    assert!(received(&server).await[0].url.query().is_some_and(|query| query.contains(SENTINEL)));
    for text in [format!("{:?}", response.meta), response.to_string()] {
        assert!(!text.contains(SENTINEL), "{text}");
    }
    let error = builder(api, UNUSED).default_header("X-Endpoint-Key", SENTINEL).build().expect_err("auth-bearing");
    assert_eq!(error, BuildError::AuthBearingDefaultHeader { name: "X-Endpoint-Key".into() });
}

#[tokio::test]
async fn a_replacement_credential_follows_only_the_redirects_the_origin_policy_allows_credentials_on() {
    let origin_a = MockServer::start().await;
    let origin_b = MockServer::start().await;
    Mock::given(path("/same"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", "/landing"))
        .mount(&origin_a)
        .await;
    Mock::given(path("/cross"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", format!("{}/landing", origin_b.uri())))
        .mount(&origin_a)
        .await;
    for server in [&origin_a, &origin_b] {
        Mock::given(path("/landing"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"id":1}"#, "application/json"))
            .mount(server)
            .await;
    }
    let api = mixed_api("SCHEMATIC_RT_TEST_REDIRECT_TOKEN", "SCHEMATIC_RT_TEST_REDIRECT_USER", "SCHEMATIC_RT_TEST_REDIRECT_PASS")
        .cross_origin(OriginPolicy::allow_without_credentials());
    let client = builder(api, &origin_a.uri())
        .credentials(Credential::basic("alice", "client-pass"))
        .build()
        .expect("client");

    client.send(Replaced(get("/same"))).full().await.expect("head");
    client.send(Replaced(get("/cross"))).full().await.expect("head");

    let at_a = received(&origin_a).await;
    let landed_a = at_a.iter().find(|request| request.url.path() == "/landing").expect("same-origin hop");
    assert_eq!(authorization(landed_a), [basic("alice", "client-pass")]);
    let at_b = received(&origin_b).await;
    assert_eq!(at_b.len(), 1);
    assert!(authorization(&at_b[0]).is_empty(), "no credential crosses origins");
}
