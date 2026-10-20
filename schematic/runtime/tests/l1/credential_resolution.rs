//! Credential resolution against a local mock: precedence of per-call,
//! client, environment, and anonymous sources, the environment scan, one
//! resolution and one attempt per send, placements, refusals, anonymous
//! mode, and the sentinel's absence from what is stored (R14–R16).

use base64::Engine;
use bytes::Bytes;
use http::Method;
use schematic_runtime::{
    ApiDefinition, AuthConfig, AuthMethod, BuildError, Client, Credential, CredentialPlacement, EnvFallback, FullCall,
    ErrorKind, Json, NoDeclaredError, RequestParts, Response, TransportError, TransportErrorKind,
};
use test_toolkit::EnvGuard;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, ProviderError, SENTINEL, TestApi, builder, listener, read_head, write_all};

const UNUSED: &str = "https://unused.invalid";

fn bearer() -> CredentialPlacement {
    CredentialPlacement::Bearer { header: None }
}

/// An API that accepts a bearer token and reads one from `vars`.
fn bearer_api(vars: &[&str]) -> ApiDefinition {
    ApiDefinition::new(UNUSED).auth(AuthConfig::none().accept(bearer()).env_fallback(EnvFallback::new(bearer(), vars.to_vec())))
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

/// A send of `parts` whose await returns the envelope; per-call settings
/// chain on the result.
fn send(client: &Client<TestApi>, parts: RequestParts) -> FullCall<'_, TestApi, Call<Json<Item>>> {
    client.send(Call::<Json<Item>>::new(parts)).full()
}

fn get() -> RequestParts {
    RequestParts::new(Method::GET, "/item")
}

fn authorization_values(request: &wiremock::Request) -> Vec<String> {
    request
        .headers
        .get_all("authorization")
        .iter()
        .map(|value| value.to_str().expect("ascii").to_string())
        .collect()
}

fn assert_invalid_argument(error: &TransportError) {
    assert!(matches!(error.kind, TransportErrorKind::InvalidArgument(_)), "{:?}", error.kind);
}

fn assert_sentinel_not_stored(response: &Response<Item, NoDeclaredError>) {
    let stored = format!("{:?}", response.meta);
    let shown = response.to_string();
    for text in [stored, shown] {
        assert!(!text.contains(SENTINEL), "{text}");
    }
}

#[tokio::test]
async fn per_call_beats_client_beats_environment_beats_anonymous_with_one_credential_each() {
    let var = "SCHEMATIC_RT_TEST_PRECEDENCE_TOKEN";
    let per_call = format!("{SENTINEL}-per-call");
    let client_level = format!("{SENTINEL}-client");
    let from_env = format!("{SENTINEL}-env");
    let server = ok_server().await;
    let env = EnvGuard::set_safe(var, &from_env);
    let with_client = builder(bearer_api(&[var]), &server.uri())
        .credentials(Credential::access_token(&client_level))
        .build()
        .expect("client");
    let without_client = builder(bearer_api(&[var]), &server.uri()).build().expect("client");

    let mut reported = Vec::new();
    for (client, per_call) in [
        (&with_client, Some(Credential::access_token(&per_call))),
        (&with_client, None),
        (&without_client, None),
    ] {
        let call = send(client, get());
        let call = match per_call {
            Some(credential) => call.credential(credential),
            None => call,
        };
        let response = call.await.expect("head");
        assert_sentinel_not_stored(&response);
        reported.push(response.meta.request.auth.clone());
    }
    drop(env);
    let response = send(&without_client, get()).await.expect("head");
    assert_sentinel_not_stored(&response);
    reported.push(response.meta.request.auth.clone());

    assert_eq!(
        reported,
        [AuthMethod::PerCall, AuthMethod::ClientLevel, AuthMethod::EnvVar(var.into()), AuthMethod::Anonymous]
    );
    let requests = received(&server).await;
    let sent: Vec<_> = requests.iter().map(authorization_values).collect();
    assert_eq!(
        sent,
        [
            vec![format!("Bearer {per_call}")],
            vec![format!("Bearer {client_level}")],
            vec![format!("Bearer {from_env}")],
            vec![],
        ]
    );
}

#[tokio::test]
async fn nothing_resolving_sends_once_anonymously_and_a_401_is_an_envelope_failure() {
    let var = "SCHEMATIC_RT_TEST_NOTHING_RESOLVES_TOKEN";
    let _unset = EnvGuard::remove_safe(var);
    let body: &[u8] = br#"{"message":"authentication required"}"#;
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_raw(body, "application/json")
                .append_header("www-authenticate", "Bearer realm=\"api\""),
        )
        .mount(&server)
        .await;
    let client = builder(bearer_api(&[var]), &server.uri()).build().expect("client");

    let response = client
        .send(Call::<Json<Item>, ProviderError>::get("/item")).full()
        .await
        .expect("a 401 head is an envelope, not a transport error");

    assert_eq!(response.meta.status, 401);
    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(response.meta.headers["www-authenticate"], "Bearer realm=\"api\"");
    let error = response.error().expect("failure");
    assert_eq!(
        error.kind,
        ErrorKind::Declared(ProviderError {
            message: "authentication required".into()
        })
    );
    assert_eq!(error.body, Bytes::from_static(body));
    let requests = received(&server).await;
    assert_eq!(requests.len(), 1, "no retry with another credential or none");
    assert!(authorization_values(&requests[0]).is_empty());
}

#[tokio::test]
async fn the_first_non_empty_variable_wins_and_a_client_order_replaces_the_definitions() {
    let empty = "SCHEMATIC_RT_TEST_SCAN_EMPTY";
    let second = "SCHEMATIC_RT_TEST_SCAN_SECOND";
    let third = "SCHEMATIC_RT_TEST_SCAN_THIRD";
    let _empty = EnvGuard::set_safe(empty, "");
    let _second = EnvGuard::set_safe(second, "second-token");
    let _third = EnvGuard::set_safe(third, "third-token");
    let server = ok_server().await;

    let scanned = builder(bearer_api(&[empty, second, third]), &server.uri()).build().expect("client");
    let response = send(&scanned, get()).await.expect("head");
    assert_eq!(response.meta.request.auth, AuthMethod::EnvVar(second.into()), "the empty variable is skipped");

    let reordered = builder(bearer_api(&[empty, second, third]), &server.uri())
        .env_order([third, second])
        .build()
        .expect("client");
    let response = send(&reordered, get()).await.expect("head");
    assert_eq!(response.meta.request.auth, AuthMethod::EnvVar(third.into()));

    let requests = received(&server).await;
    assert_eq!(authorization_values(&requests[0]), ["Bearer second-token"]);
    assert_eq!(authorization_values(&requests[1]), ["Bearer third-token"]);
}

#[tokio::test]
async fn each_send_reads_the_environment_again() {
    let var = "SCHEMATIC_RT_TEST_EACH_SEND_TOKEN";
    let server = ok_server().await;
    let client = builder(bearer_api(&[var]), &server.uri()).build().expect("client");

    let first = EnvGuard::set_safe(var, "first-token");
    let response = send(&client, get()).await.expect("head");
    assert_eq!(response.meta.request.auth, AuthMethod::EnvVar(var.into()));
    drop(first);
    let _second = EnvGuard::set_safe(var, "second-token");
    send(&client, get()).await.expect("head");

    let requests = received(&server).await;
    assert_eq!(authorization_values(&requests[0]), ["Bearer first-token"]);
    assert_eq!(authorization_values(&requests[1]), ["Bearer second-token"]);
}

#[tokio::test]
async fn an_environment_change_after_the_request_left_does_not_change_the_reported_source() {
    let first = "SCHEMATIC_RT_TEST_MID_FLIGHT_FIRST";
    let second = "SCHEMATIC_RT_TEST_MID_FLIGHT_SECOND";
    let (socket, base_url) = listener().await;
    let client = builder(bearer_api(&[first, second]), &base_url).build().expect("client");
    let first_guard = EnvGuard::set_safe(first, SENTINEL);
    let _second_unset = EnvGuard::remove_safe(second);

    // The server holds the response until it has read the request head, then
    // moves the credential to the other variable before answering.
    let server = async move {
        let (mut stream, _) = socket.accept().await.expect("accept");
        let head = read_head(&mut stream).await;
        drop(first_guard);
        let replacement = EnvGuard::set_safe(second, "replacement-token");
        write_all(
            &mut stream,
            b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 8\r\nconnection: close\r\n\r\n{\"id\":1}",
        )
        .await;
        (head, replacement)
    };
    let (response, (head, _replacement)) = tokio::join!(send(&client, get()), server);

    let response = response.expect("head");
    assert_eq!(response.meta.request.auth, AuthMethod::EnvVar(first.into()));
    assert!(head.to_ascii_lowercase().contains(&format!("authorization: bearer {}", SENTINEL.to_ascii_lowercase())), "{head}");
    assert_sentinel_not_stored(&response);
}

#[tokio::test]
async fn each_placement_puts_the_credential_where_the_api_expects_it() {
    let server = ok_server().await;
    let token = Credential::access_token(SENTINEL);
    let key = Credential::api_key(SENTINEL);
    let cases = [
        (bearer(), token.clone()),
        (CredentialPlacement::Bearer { header: Some("X-Auth".into()) }, token),
        (
            CredentialPlacement::ApiKeyHeader {
                header: "X-Vendor-Key".into(),
                prefix: Some("token ".into()),
            },
            key.clone(),
        ),
        (CredentialPlacement::ApiKeyQuery { name: "sig".into() }, key),
        (CredentialPlacement::Basic, Credential::basic("alice", SENTINEL)),
    ];
    for (placement, credential) in cases {
        let client = builder(ApiDefinition::new(UNUSED).auth(AuthConfig::none().accept(placement)), &server.uri())
            .build()
            .expect("client");
        let mut parts = get();
        parts.query = vec![("page".into(), "2".into())];
        let response = send(&client, parts).credential(credential).await.expect("head");
        assert_eq!(response.meta.request.auth, AuthMethod::PerCall);
        assert_sentinel_not_stored(&response);
    }

    let requests = received(&server).await;
    assert_eq!(authorization_values(&requests[0]), [format!("Bearer {SENTINEL}")]);
    assert_eq!(requests[1].headers["x-auth"], format!("Bearer {SENTINEL}").as_str());
    assert!(authorization_values(&requests[1]).is_empty(), "a named header replaces Authorization");
    assert_eq!(requests[2].headers["x-vendor-key"], format!("token {SENTINEL}").as_str());
    assert_eq!(requests[3].url.query(), Some(format!("page=2&sig={SENTINEL}").as_str()));
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("alice:{SENTINEL}"));
    assert_eq!(authorization_values(&requests[4]), [format!("Basic {basic}")]);
    for request in &requests[1..4] {
        assert!(authorization_values(request).is_empty());
    }
}

#[tokio::test]
async fn a_cookie_credential_joins_existing_cookies_with_its_value_percent_encoded() {
    let server = ok_server().await;
    let api = ApiDefinition::new(UNUSED).auth(AuthConfig::none().accept(CredentialPlacement::ApiKeyCookie { name: "sid".into() }));
    let client = builder(api, &server.uri()).build().expect("client");
    let mut parts = get();
    parts.headers = vec![("Cookie".into(), "a=1".into())];

    let response = send(&client, parts).credential(Credential::api_key("k ey;\"1\"")).await.expect("head");

    assert_eq!(response.meta.request.auth, AuthMethod::PerCall);
    let requests = received(&server).await;
    let cookies: Vec<_> = requests[0].headers.get_all("cookie").iter().collect();
    assert_eq!(cookies, ["a=1; sid=k%20ey%3B%221%22"]);
}

#[tokio::test]
async fn basic_environment_credentials_need_both_variables_and_report_the_password_variable() {
    let user_var = "SCHEMATIC_RT_TEST_BASIC_ENV_USER";
    let password_var = "SCHEMATIC_RT_TEST_BASIC_ENV_PASSWORD";
    let backup_password_var = "SCHEMATIC_RT_TEST_BASIC_ENV_PASSWORD_BACKUP";
    let api = ApiDefinition::new(UNUSED).auth(
        AuthConfig::none().accept(CredentialPlacement::Basic).env_fallback(
            EnvFallback::new(CredentialPlacement::Basic, [backup_password_var, password_var]).username_vars([user_var]),
        ),
    );
    let server = ok_server().await;
    let client = builder(api, &server.uri()).build().expect("client");
    let _backup_unset = EnvGuard::remove_safe(backup_password_var);

    let user = EnvGuard::set_safe(user_var, "alice");
    let password = EnvGuard::set_safe(password_var, SENTINEL);
    let both = send(&client, get()).await.expect("head");
    assert_eq!(both.meta.request.auth, AuthMethod::EnvVar(password_var.into()));
    assert_sentinel_not_stored(&both);

    drop(user);
    let no_user = send(&client, get()).await.expect("head");
    assert_eq!(no_user.meta.request.auth, AuthMethod::Anonymous);

    drop(password);
    let _user = EnvGuard::set_safe(user_var, "alice");
    let no_password = send(&client, get()).await.expect("head");
    assert_eq!(no_password.meta.request.auth, AuthMethod::Anonymous);

    let requests = received(&server).await;
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("alice:{SENTINEL}"));
    assert_eq!(authorization_values(&requests[0]), [format!("Basic {basic}")]);
    assert!(authorization_values(&requests[1]).is_empty());
    assert!(authorization_values(&requests[2]).is_empty());
}

#[tokio::test]
async fn an_explicit_policy_and_its_environment_fallback_use_their_own_placements() {
    let var = "SCHEMATIC_RT_TEST_POLICY_SPLIT_KEY";
    let _env = EnvGuard::set_safe(var, "env-key");
    let api = ApiDefinition::new(UNUSED).auth(
        AuthConfig::none()
            .accept(bearer())
            .env_fallback(EnvFallback::new(CredentialPlacement::ApiKeyQuery { name: "key".into() }, [var])),
    );
    let server = ok_server().await;
    let client = builder(api, &server.uri()).build().expect("client");

    let per_call = send(&client, get()).credential(Credential::access_token("call-token"))
        .await
        .expect("head");
    assert_eq!(per_call.meta.request.auth, AuthMethod::PerCall);
    let from_env = send(&client, get()).await.expect("head");
    assert_eq!(from_env.meta.request.auth, AuthMethod::EnvVar(var.into()));

    let requests = received(&server).await;
    assert_eq!(authorization_values(&requests[0]), ["Bearer call-token"]);
    assert_eq!(requests[0].url.query(), None);
    assert!(authorization_values(&requests[1]).is_empty());
    assert_eq!(requests[1].url.query(), Some("key=env-key"));

    // The fallback's placement is not an accepted placement for a caller's key.
    let refused = send(&client, get()).credential(Credential::api_key("call-key"))
        .await
        .expect_err("refused before sending");
    assert_invalid_argument(&refused);
    assert_eq!(received(&server).await.len(), 2);
}

#[tokio::test]
async fn a_client_credential_no_placement_accepts_fails_the_build() {
    let error = builder(bearer_api(&[]), UNUSED)
        .credentials(Credential::api_key(SENTINEL))
        .build()
        .expect_err("an API key on a bearer-only API");
    assert_eq!(error, BuildError::IncompatibleCredential { supplied: "API key" });
    assert!(!error.to_string().contains(SENTINEL));

    let error = builder(ApiDefinition::new(UNUSED), UNUSED)
        .credentials(Credential::basic("alice", SENTINEL))
        .build()
        .expect_err("an API without authentication accepts no credential");
    assert_eq!(error, BuildError::IncompatibleCredential { supplied: "Basic" });
}

#[tokio::test]
async fn a_per_call_credential_no_placement_accepts_is_refused_before_any_request() {
    let server = ok_server().await;
    let bearer_only = builder(bearer_api(&[]), &server.uri()).build().expect("client");
    let no_auth = builder(ApiDefinition::new(UNUSED), &server.uri()).build().expect("client");

    for (client, credential) in [
        (&bearer_only, Credential::api_key(SENTINEL)),
        (&bearer_only, Credential::basic("alice", SENTINEL)),
        (&no_auth, Credential::access_token(SENTINEL)),
    ] {
        let error = send(client, get()).credential(credential).await.expect_err("refused");
        assert_invalid_argument(&error);
        assert!(!format!("{error:?}").contains(SENTINEL));
        assert!(!error.to_string().contains(SENTINEL));
    }
    assert_eq!(received(&server).await.len(), 0);
}

#[tokio::test]
async fn a_request_already_carrying_the_credentials_slot_is_refused_and_sent_unchanged_without_one() {
    let server = ok_server().await;
    let cases: [(CredentialPlacement, Credential, RequestParts); 4] = [
        (bearer(), Credential::access_token(SENTINEL), {
            let mut parts = get();
            parts.headers = vec![("authorization".into(), "Bearer from-parts".into())];
            parts
        }),
        (CredentialPlacement::Bearer { header: Some("X-Auth".into()) }, Credential::access_token(SENTINEL), {
            let mut parts = get();
            parts.headers = vec![("X-Auth".into(), "from-parts".into())];
            parts
        }),
        (CredentialPlacement::ApiKeyQuery { name: "key".into() }, Credential::api_key(SENTINEL), {
            let mut parts = get();
            parts.query = vec![("key".into(), "from-parts".into())];
            parts
        }),
        (CredentialPlacement::ApiKeyCookie { name: "sid".into() }, Credential::api_key(SENTINEL), {
            let mut parts = get();
            parts.headers = vec![("Cookie".into(), "a=1; sid=from-parts".into())];
            parts
        }),
    ];
    for (sent_before, (placement, credential, parts)) in cases.into_iter().enumerate() {
        let client = builder(ApiDefinition::new(UNUSED).auth(AuthConfig::none().accept(placement.clone())), &server.uri())
            .build()
            .expect("client");
        let error = send(&client, parts.clone()).credential(credential)
            .await
            .expect_err("collision");
        assert_invalid_argument(&error);
        assert_eq!(received(&server).await.len(), sent_before, "{placement:?}");

        send(&client, parts).await.expect("no credential resolved, so no collision");
    }

    let requests = received(&server).await;
    assert_eq!(authorization_values(&requests[0]), ["Bearer from-parts"]);
    assert_eq!(requests[1].headers["x-auth"], "from-parts");
    assert_eq!(requests[2].url.query(), Some("key=from-parts"));
    assert_eq!(requests[3].headers["cookie"], "a=1; sid=from-parts");
}

#[tokio::test]
async fn ordinary_duplicate_query_values_stay_beside_a_query_credential() {
    let server = ok_server().await;
    let api = ApiDefinition::new(UNUSED).auth(AuthConfig::none().accept(CredentialPlacement::ApiKeyQuery { name: "key".into() }));
    let client = builder(api, &server.uri()).build().expect("client");
    let mut parts = get();
    parts.query = vec![("tag".into(), "a".into()), ("tag".into(), "b".into())];

    send(&client, parts).credential(Credential::api_key("k1")).await.expect("head");

    assert_eq!(received(&server).await[0].url.query(), Some("tag=a&tag=b&key=k1"));
}

#[tokio::test]
async fn anonymous_mode_bypasses_every_source_and_strips_every_placement_from_the_request() {
    let var = "SCHEMATIC_RT_TEST_ANONYMOUS_MODE_KEY";
    let _env = EnvGuard::set_safe(var, SENTINEL);
    let api = ApiDefinition::new(UNUSED).auth(
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
    );
    let server = ok_server().await;
    let client = builder(api, &server.uri())
        .credentials(Credential::access_token(SENTINEL))
        .build()
        .expect("client");
    let mut parts = get();
    parts.query = vec![("key".into(), SENTINEL.into()), ("page".into(), "2".into())];
    parts.headers = vec![
        ("Authorization".into(), format!("Bearer {SENTINEL}")),
        ("X-Vendor-Key".into(), SENTINEL.into()),
        ("Cookie".into(), format!("a=1; sid={SENTINEL}; b=2")),
        ("X-Trace".into(), "t1".into()),
    ];

    let response = send(&client, parts).anonymous().await.expect("head");

    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    let requests = received(&server).await;
    let request = &requests[0];
    assert!(authorization_values(request).is_empty());
    assert!(request.headers.get("x-vendor-key").is_none());
    assert_eq!(request.headers["cookie"], "a=1; b=2", "other cookies are kept");
    assert_eq!(request.headers["x-trace"], "t1");
    assert_eq!(request.url.query(), Some("page=2"));
}

#[tokio::test]
async fn a_default_header_that_would_carry_authentication_fails_the_build() {
    let custom_header_api = ApiDefinition::new(UNUSED).auth(AuthConfig::none().accept(CredentialPlacement::ApiKeyHeader {
        header: "X-Vendor-Key".into(),
        prefix: None,
    }));
    let marked_secret_api = ApiDefinition::new(UNUSED).secret_names(["X-Signature"]);
    for (definition, name) in [
        (ApiDefinition::new(UNUSED), "Authorization"),
        (custom_header_api, "x-vendor-key"),
        (marked_secret_api, "X-Signature"),
    ] {
        let error = builder(definition, UNUSED)
            .default_header(name, SENTINEL)
            .build()
            .expect_err("an auth-bearing default header");
        assert_eq!(error, BuildError::AuthBearingDefaultHeader { name: name.into() });
        assert!(!error.to_string().contains(SENTINEL));
    }

    builder(ApiDefinition::new(UNUSED), UNUSED)
        .default_header("X-Trace", "t1")
        .build()
        .expect("an ordinary default header is accepted");
}
