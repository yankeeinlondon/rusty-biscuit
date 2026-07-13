//! Mocked HTTP tests for the OAuth2 token flows using `wiremock`.
//!
//! These exercise the happy paths of `exchange_code`,
//! `acquire_client_credentials_token`, token refresh via `get_valid_token`, and
//! `revoke_token`, plus the refresh-token-preservation and single-flight
//! behaviors called out in review.

use std::sync::Arc;

use schematic_define::{OAuth2ClientAuthMethod, OAuth2Config, OAuth2GrantType, PkceRequirement};
use schematic_oauth::{
    FileTokenStore, MemoryTokenStore, OAuth2Manager, OAuth2RuntimeConfig, StoredTokens, TokenStore,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn auth_code_config(token_url: String) -> OAuth2RuntimeConfig {
    OAuth2RuntimeConfig {
        provider: OAuth2Config {
            grant_type: OAuth2GrantType::AuthorizationCodePkce,
            authorization_url: Some("https://example.com/authorize".into()),
            token_url,
            revocation_url: None,
            device_authorization_url: None,
            default_scopes: vec!["read".into()],
            pkce: PkceRequirement::Required,
            client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
        },
        client_id: "test_client".into(),
        client_secret: Some("test_secret".into()),
        redirect_uri: Some("http://localhost:8080/callback".into()),
        scopes: vec!["read".into()],
    }
}

fn client_credentials_config(token_url: String) -> OAuth2RuntimeConfig {
    OAuth2RuntimeConfig {
        provider: OAuth2Config {
            grant_type: OAuth2GrantType::ClientCredentials,
            authorization_url: None,
            token_url,
            revocation_url: None,
            device_authorization_url: None,
            default_scopes: vec!["api".into()],
            pkce: PkceRequirement::NotUsed,
            client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
        },
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
