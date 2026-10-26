//! Mocked HTTP tests for the OAuth2 token flows using `wiremock`.
//!
//! These exercise the happy paths of `exchange_code` (both PKCE modes),
//! `acquire_client_credentials_token` under each client-authentication
//! method, and token refresh via `get_valid_token`; the refusal of a flow for
//! a provider of another grant; and the refresh-token-preservation and
//! single-flight behaviors called out in review.

use std::sync::Arc;

use schematic_define::{
    AuthorizationCodeGrant, ClientCredentialsGrant, OAuth2ClientAuthMethod, OAuth2Config,
    PkceRequirement,
};
use schematic_oauth::{
    FileTokenStore, MemoryTokenStore, OAuth2Manager, OAuth2RuntimeConfig, StoredTokens, TokenStore,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn auth_code_config(token_url: String) -> OAuth2RuntimeConfig {
    OAuth2RuntimeConfig {
        provider: OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant {
            authorization_url: "https://example.com/authorize".into(),
            token_url,
            revocation_url: None,
            default_scopes: vec!["read".into()],
            pkce: PkceRequirement::Required,
            client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
        }),
        client_id: "test_client".into(),
        client_secret: Some("test_secret".into()),
        redirect_uri: Some("http://localhost:8080/callback".into()),
        scopes: vec!["read".into()],
    }
}

fn client_credentials_config(token_url: String) -> OAuth2RuntimeConfig {
    OAuth2RuntimeConfig {
        provider: OAuth2Config::ClientCredentials(ClientCredentialsGrant {
            token_url,
            revocation_url: None,
            default_scopes: vec!["api".into()],
            client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
        }),
        client_id: "service_client".into(),
        client_secret: Some("service_secret".into()),
        redirect_uri: None,
        scopes: vec!["api".into()],
    }
}

fn token_body_with_refresh() -> serde_json::Value {
    serde_json::json!({
        "access_token": "new_access",
        "token_type": "Bearer",
        "expires_in": 3600,
        "refresh_token": "new_refresh",
    })
}

#[tokio::test]
async fn exchange_code_stores_tokens_from_token_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_body_with_refresh()))
        .mount(&server)
        .await;

    let manager = OAuth2Manager::new(
        auth_code_config(format!("{}/token", server.uri())),
        Box::new(MemoryTokenStore::new()),
    )
    .unwrap();

    let session = manager.begin_authorization().unwrap();
    let tokens = manager
        .exchange_code("auth_code", &session.csrf_state, &session)
        .await
        .unwrap();

    assert_eq!(tokens.access_token, "new_access");
    assert_eq!(tokens.refresh_token.as_deref(), Some("new_refresh"));

    // Stored for later reuse.
    let cached = manager.get_valid_token().await.unwrap();
    assert_eq!(cached, "new_access");
}

#[tokio::test]
async fn acquire_client_credentials_token_stores_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "cc_access",
            "token_type": "Bearer",
            "expires_in": 3600,
        })))
        .mount(&server)
        .await;

    let manager = OAuth2Manager::new(
        client_credentials_config(format!("{}/token", server.uri())),
        Box::new(MemoryTokenStore::new()),
    )
    .unwrap();

    let tokens = manager.acquire_client_credentials_token().await.unwrap();
    assert_eq!(tokens.access_token, "cc_access");
}

#[tokio::test]
async fn refresh_updates_access_token_via_get_valid_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_body_with_refresh()))
        .mount(&server)
        .await;

    let store = MemoryTokenStore::new();
    store
        .save(&StoredTokens {
            access_token: "old_access".into(),
            refresh_token: Some("old_refresh".into()),
            expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            scopes: vec!["read".into()],
        })
        .unwrap();

    let manager =
        OAuth2Manager::new(auth_code_config(format!("{}/token", server.uri())), Box::new(store))
            .unwrap();

    let token = manager.get_valid_token().await.unwrap();
    assert_eq!(token, "new_access");
}

#[tokio::test]
async fn refresh_preserves_existing_refresh_token_when_response_omits_it() {
    let server = MockServer::start().await;
    // RFC 6749 §6: response without `refresh_token`.
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "rotated_access",
            "token_type": "Bearer",
            "expires_in": 3600,
        })))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tokens.json");

    // Seed an expired token that still carries a refresh token.
    FileTokenStore::new(&path)
        .save(&StoredTokens {
            access_token: "old_access".into(),
            refresh_token: Some("keep_this_refresh".into()),
            expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            scopes: vec!["read".into()],
        })
        .unwrap();

    let manager = OAuth2Manager::new(
        auth_code_config(format!("{}/token", server.uri())),
        Box::new(FileTokenStore::new(&path)),
    )
    .unwrap();

    let token = manager.get_valid_token().await.unwrap();
    assert_eq!(token, "rotated_access");

    // The refresh token from before the refresh must survive the omission.
    let persisted = FileTokenStore::new(&path).load().unwrap().unwrap();
    assert_eq!(persisted.refresh_token.as_deref(), Some("keep_this_refresh"));
}

// NOTE: `revoke_token`'s happy path is not covered here because the `oauth2`
// crate requires the revocation endpoint to use HTTPS, which the plain-HTTP
// `wiremock` server cannot serve. Revocation error paths are covered by unit
// tests in `manager::tests`.

#[tokio::test]
async fn concurrent_get_valid_token_refreshes_exactly_once() {
    let server = MockServer::start().await;
    // `.expect(1)` fails on server drop if the token endpoint is hit more than once.
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_body_with_refresh()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryTokenStore::new();
    store
        .save(&StoredTokens {
            access_token: "old_access".into(),
            refresh_token: Some("old_refresh".into()),
            expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            scopes: vec!["read".into()],
        })
        .unwrap();

    let manager = Arc::new(
        OAuth2Manager::new(auth_code_config(format!("{}/token", server.uri())), Box::new(store))
            .unwrap(),
    );

    let mut handles = Vec::new();
    for _ in 0..8 {
        let manager = Arc::clone(&manager);
        handles.push(tokio::spawn(async move { manager.get_valid_token().await }));
    }

    for handle in handles {
        let token = handle.await.unwrap().unwrap();
        assert_eq!(token, "new_access");
    }

    // Explicit verification of the single-flight expectation.
    server.verify().await;
}

/// The client-credentials provider of [`client_credentials_config`] with
/// `client_auth` set to `auth`.
fn client_credentials_with(token_url: String, auth: OAuth2ClientAuthMethod) -> OAuth2RuntimeConfig {
    OAuth2RuntimeConfig {
        provider: OAuth2Config::ClientCredentials(ClientCredentialsGrant {
            token_url,
            revocation_url: None,
            default_scopes: vec!["provider-default".into()],
            client_auth: auth,
        }),
        ..client_credentials_config(String::new())
    }
}

/// Mounts a token endpoint and returns the one request it received after
/// `acquire_client_credentials_token` ran with `auth`.
async fn client_credentials_request(auth: OAuth2ClientAuthMethod) -> wiremock::Request {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "cc_access",
            "token_type": "Bearer",
        })))
        .mount(&server)
        .await;
    let manager = OAuth2Manager::new(
        client_credentials_with(format!("{}/token", server.uri()), auth),
        Box::new(MemoryTokenStore::new()),
    )
    .unwrap();
    let tokens = manager.acquire_client_credentials_token().await.unwrap();
    // The response names no scope, so the requested scopes are stored.
    assert_eq!(tokens.scopes, ["api"]);
    let mut received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    received.remove(0)
}

fn form_body(request: &wiremock::Request) -> Vec<(String, String)> {
    url::form_urlencoded::parse(&request.body).into_owned().collect()
}

fn body_value<'a>(body: &'a [(String, String)], name: &str) -> Option<&'a str> {
    body.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
}

#[tokio::test]
async fn client_secret_basic_sends_credentials_in_the_authorization_header() {
    let request = client_credentials_request(OAuth2ClientAuthMethod::ClientSecretBasic).await;
    let authorization = request.headers.get("authorization").expect("Authorization header");
    // base64("service_client:service_secret")
    assert_eq!(authorization.to_str().unwrap(), "Basic c2VydmljZV9jbGllbnQ6c2VydmljZV9zZWNyZXQ=");
    let body = form_body(&request);
    assert_eq!(body_value(&body, "grant_type"), Some("client_credentials"));
    assert_eq!(body_value(&body, "client_secret"), None);
    // The runtime scopes are requested; the provider's default scopes are not.
    assert_eq!(body_value(&body, "scope"), Some("api"));
}

#[tokio::test]
async fn client_secret_post_sends_credentials_in_the_body() {
    let request = client_credentials_request(OAuth2ClientAuthMethod::ClientSecretPost).await;
    assert!(request.headers.get("authorization").is_none());
    let body = form_body(&request);
    assert_eq!(body_value(&body, "client_id"), Some("service_client"));
    assert_eq!(body_value(&body, "client_secret"), Some("service_secret"));
}

#[tokio::test]
async fn a_public_client_sends_its_id_and_never_the_configured_secret() {
    let request = client_credentials_request(OAuth2ClientAuthMethod::None).await;
    assert!(request.headers.get("authorization").is_none());
    let body = form_body(&request);
    assert_eq!(body_value(&body, "client_id"), Some("service_client"));
    assert_eq!(body_value(&body, "client_secret"), None);
    assert!(!String::from_utf8_lossy(&request.body).contains("service_secret"));
}

#[tokio::test]
async fn each_flow_refuses_a_provider_of_another_grant_before_sending() {
    let server = MockServer::start().await;
    let token_url = format!("{}/token", server.uri());

    let auth_code = OAuth2Manager::new(auth_code_config(token_url.clone()), Box::new(MemoryTokenStore::new())).unwrap();
    let error = auth_code.acquire_client_credentials_token().await.unwrap_err();
    assert!(error.to_string().contains("ClientCredentials grant"), "got: {error}");

    let client_credentials = OAuth2Manager::new(
        client_credentials_config(token_url.clone()),
        Box::new(MemoryTokenStore::new()),
    )
    .unwrap();
    let error = client_credentials.begin_authorization().unwrap_err();
    assert!(error.to_string().contains("AuthorizationCodePkce grant"), "got: {error}");

    let device = OAuth2Manager::new(
        OAuth2RuntimeConfig {
            provider: OAuth2Config::DeviceCode(schematic_define::DeviceCodeGrant {
                device_authorization_url: "https://example.com/device".into(),
                token_url,
                revocation_url: None,
                default_scopes: vec![],
                client_auth: OAuth2ClientAuthMethod::None,
            }),
            ..client_credentials_config(String::new())
        },
        Box::new(MemoryTokenStore::new()),
    )
    .unwrap();
    assert!(device.begin_authorization().is_err());
    assert!(device.acquire_client_credentials_token().await.is_err());

    assert!(server.received_requests().await.unwrap().is_empty(), "nothing may be sent");
}

#[tokio::test]
async fn both_pkce_modes_send_the_verifier_with_the_code() {
    for pkce in [PkceRequirement::Required, PkceRequirement::Supported] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body_with_refresh()))
            .mount(&server)
            .await;
        let mut config = auth_code_config(format!("{}/token", server.uri()));
        if let OAuth2Config::AuthorizationCodePkce(grant) = &mut config.provider {
            grant.pkce = pkce;
        }
        let manager = OAuth2Manager::new(config, Box::new(MemoryTokenStore::new())).unwrap();
        let session = manager.begin_authorization().unwrap();
        let verifier = session.pkce_verifier.clone().expect("a verifier for {pkce:?}");
        manager.exchange_code("auth_code", &session.csrf_state, &session).await.unwrap();

        let received = server.received_requests().await.unwrap();
        let body = form_body(&received[0]);
        assert_eq!(body_value(&body, "grant_type"), Some("authorization_code"), "{pkce:?}");
        assert_eq!(body_value(&body, "code_verifier"), Some(verifier.as_str()), "{pkce:?}");
    }
}
