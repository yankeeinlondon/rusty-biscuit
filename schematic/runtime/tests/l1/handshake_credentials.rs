//! Handshake credential selection for transports the runtime does not send
//! itself (generated WebSocket clients): per-call, client, first non-empty
//! environment variable, anonymous; one placement per selection; refusal of
//! a second explicit source; and a value that never reaches `Debug` or the
//! redacted names.

use http::HeaderMap;
use http::header::COOKIE;
use schematic_runtime::{AuthConfig, AuthMethod, Credential, CredentialPlacement, EnvFallback, Url};
use test_toolkit::EnvGuard;

const SENTINEL: &str = "hs-sentinel-9c41";

fn api_key_header() -> CredentialPlacement {
    CredentialPlacement::ApiKeyHeader {
        header: "API-KEY".to_string(),
        prefix: None,
    }
}

/// Applies the selection to a fresh URL and header map.
fn placed(config: &AuthConfig, per_call: Option<&Credential>, client: Option<&Credential>) -> (Url, HeaderMap, AuthMethod) {
    let selected = config.select_handshake(per_call, client).expect("selects");
    let mut url = Url::parse("ws://127.0.0.1/ws?mode=live").unwrap();
    let mut headers = HeaderMap::new();
    selected.apply(&mut url, &mut headers).expect("applies");
    (url, headers, selected.auth_method().clone())
}

#[test]
fn per_call_wins_over_client_and_environment_and_client_over_environment() {
    let var = "SCHEMATIC_HS_TEST_PRECEDENCE";
    let _env = EnvGuard::set_safe(var, "from-env");
    let config = AuthConfig::none()
        .accept(api_key_header())
        .env_fallback(EnvFallback::new(api_key_header(), [var]));
    let per_call = Credential::api_key("per-call");
    let client = Credential::api_key("client");

    let (_, headers, auth) = placed(&config, Some(&per_call), Some(&client));
    assert_eq!(headers.get_all("api-key").iter().collect::<Vec<_>>(), ["per-call"]);
    assert_eq!(auth, AuthMethod::PerCall);

    let (_, headers, auth) = placed(&config, None, Some(&client));
    assert_eq!(headers.get_all("api-key").iter().collect::<Vec<_>>(), ["client"]);
    assert_eq!(auth, AuthMethod::ClientLevel);

    let (_, headers, auth) = placed(&config, None, None);
    assert_eq!(headers.get_all("api-key").iter().collect::<Vec<_>>(), ["from-env"]);
    assert_eq!(auth, AuthMethod::EnvVar(var.to_string()));
}

#[test]
fn the_first_non_empty_variable_wins_and_none_set_is_anonymous() {
    let (empty, second) = ("SCHEMATIC_HS_TEST_EMPTY", "SCHEMATIC_HS_TEST_SECOND");
    let _empty = EnvGuard::set_safe(empty, "");
    let _second = EnvGuard::set_safe(second, "second");
    let config = AuthConfig::none()
        .accept(api_key_header())
        .env_fallback(EnvFallback::new(api_key_header(), [empty, second]));
    let (_, headers, auth) = placed(&config, None, None);
    assert_eq!(headers.get("api-key").unwrap(), "second");
    assert_eq!(auth, AuthMethod::EnvVar(second.to_string()));

    let _unset = EnvGuard::remove_safe(second);
    let (url, headers, auth) = placed(&config, None, None);
    assert!(headers.is_empty(), "{headers:?}");
    assert_eq!(url.query(), Some("mode=live"));
    assert_eq!(auth, AuthMethod::Anonymous);
}

#[test]
fn an_api_accepting_no_credential_ignores_the_environment() {
    let var = "SCHEMATIC_HS_TEST_NONE";
    let _env = EnvGuard::set_safe(var, SENTINEL);
    let (_, headers, auth) = placed(&AuthConfig::none(), None, None);
    assert!(headers.is_empty());
    assert_eq!(auth, AuthMethod::Anonymous);
}

#[test]
fn each_placement_writes_exactly_one_credential() {
    let key = Credential::api_key("k1");
    let bearer = AuthConfig::none().accept(CredentialPlacement::Bearer { header: None });
    let (_, headers, _) = placed(&bearer, None, Some(&Credential::access_token("t1")));
    assert_eq!(headers.get_all("authorization").iter().collect::<Vec<_>>(), ["Bearer t1"]);

    let query = AuthConfig::none().accept(CredentialPlacement::ApiKeyQuery { name: "token".to_string() });
    let (url, headers, _) = placed(&query, None, Some(&key));
    assert_eq!(url.query(), Some("mode=live&token=k1"));
    assert!(headers.is_empty());

    let basic = AuthConfig::none().accept(CredentialPlacement::Basic);
    let (_, headers, _) = placed(&basic, None, Some(&Credential::basic("user", "pass")));
    // base64("user:pass")
    assert_eq!(headers.get_all("authorization").iter().collect::<Vec<_>>(), ["Basic dXNlcjpwYXNz"]);
}

#[test]
fn a_cookie_key_is_merged_with_existing_cookies() {
    let config = AuthConfig::none().accept(CredentialPlacement::ApiKeyCookie { name: "session".to_string() });
    let selected = config.select_handshake(None, Some(&Credential::api_key("c1"))).unwrap();
    let mut url = Url::parse("ws://127.0.0.1/ws").unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, "theme=dark".parse().unwrap());
    selected.apply(&mut url, &mut headers).unwrap();
    assert_eq!(headers.get_all(COOKIE).iter().collect::<Vec<_>>(), ["theme=dark; session=c1"]);

    // The same cookie set by the caller is a second source: refused.
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, "session=mine".parse().unwrap());
    let error = selected.apply(&mut url, &mut headers).unwrap_err();
    assert!(error.to_string().contains("cookie `session`"), "{error}");
}

#[test]
fn a_second_explicit_source_in_the_header_or_query_is_refused() {
    let selected = AuthConfig::none()
        .accept(api_key_header())
        .select_handshake(None, Some(&Credential::api_key(SENTINEL)))
        .unwrap();
    let mut url = Url::parse("ws://127.0.0.1/ws").unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("api-key", "caller".parse().unwrap());
    let error = selected.apply(&mut url, &mut headers).unwrap_err();
    assert!(error.to_string().contains("header `API-KEY`"), "{error}");
    assert_eq!(headers.get_all("api-key").iter().collect::<Vec<_>>(), ["caller"]);

    let selected = AuthConfig::none()
        .accept(CredentialPlacement::ApiKeyQuery { name: "token".to_string() })
        .select_handshake(None, Some(&Credential::api_key(SENTINEL)))
        .unwrap();
    let mut url = Url::parse("ws://127.0.0.1/ws?token=caller").unwrap();
    let error = selected.apply(&mut url, &mut HeaderMap::new()).unwrap_err();
    assert!(error.to_string().contains("query parameter `token`"), "{error}");
    assert!(!error.to_string().contains(SENTINEL));
}

#[test]
fn a_credential_kind_the_api_does_not_accept_is_refused_before_reading_the_environment() {
    let var = "SCHEMATIC_HS_TEST_KIND";
    let _env = EnvGuard::set_safe(var, "from-env");
    let config = AuthConfig::none()
        .accept(api_key_header())
        .env_fallback(EnvFallback::new(api_key_header(), [var]));
    let basic = Credential::basic("user", SENTINEL);
    let per_call = config.select_handshake(Some(&basic), None).unwrap_err();
    assert!(per_call.to_string().contains("per-call Basic"), "{per_call}");
    let client = config.select_handshake(None, Some(&basic)).unwrap_err();
    assert!(client.to_string().contains("client Basic"), "{client}");
    assert!(!client.to_string().contains(SENTINEL));
}

#[test]
fn the_selection_debug_and_secret_names_never_show_the_value() {
    let config = AuthConfig::none().accept(CredentialPlacement::ApiKeyQuery { name: "sig_v2".to_string() });
    let selected = config.select_handshake(Some(&Credential::api_key(SENTINEL)), None).unwrap();
    let debug = format!("{selected:?}");
    assert!(!debug.contains(SENTINEL), "{debug}");
    assert!(debug.contains("PerCall"), "{debug}");

    let secrets = config.secret_names();
    assert!(secrets.is_secret_query("sig_v2"));
    assert!(secrets.is_secret_header("set-cookie"));
    let url = Url::parse(&format!("ws://127.0.0.1/ws?sig_v2={SENTINEL}&mode=live")).unwrap();
    let redacted = secrets.redact_url(&url);
    assert!(!redacted.as_str().contains(SENTINEL), "{}", redacted.as_str());
    assert!(redacted.as_str().contains("mode=live"));
}
