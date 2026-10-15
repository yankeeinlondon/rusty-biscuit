//! Redaction of stored request and response metadata (R5 groundwork, R7
//! text inputs). Every case is observed through the public capture
//! constructors and the stored fields, not only through printed text.

use std::time::Duration;

use http::header::{HeaderName, HeaderValue};
use http::{HeaderMap, Method, StatusCode};
use schematic_runtime::redact::{BUILTIN_SECRET_HEADERS, BUILTIN_SECRET_QUERY_NAMES, REDACTED};
use schematic_runtime::{AuthMethod, RequestMeta, ResponseMeta, SecretNames};
use url::Url;

const SENTINEL: &str = "SENTINEL-7f3a9c";

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.append(
            HeaderName::from_bytes(name.as_bytes()).expect("header name"),
            HeaderValue::from_str(value).expect("header value"),
        );
    }
    map
}

fn capture_request(secrets: &SecretNames, url: &str, request_headers: &HeaderMap) -> RequestMeta {
    RequestMeta::capture(
        secrets,
        "list_items",
        Method::GET,
        &Url::parse(url).expect("url"),
        request_headers,
        AuthMethod::Anonymous,
    )
}

fn capture_response(secrets: &SecretNames, response_headers: &HeaderMap, request_id_header: Option<&str>) -> ResponseMeta {
    let request = capture_request(secrets, "https://api.example.test/v1/items", &HeaderMap::new());
    ResponseMeta::capture(
        secrets,
        request,
        StatusCode::OK,
        response_headers,
        request_id_header,
        Duration::from_millis(5),
    )
}

fn values<'a>(map: &'a HeaderMap, name: &str) -> Vec<&'a str> {
    map.get_all(name).iter().map(|value| value.to_str().expect("text")).collect()
}

#[test]
fn the_builtin_names_are_exactly_the_documented_lists() {
    assert_eq!(
        BUILTIN_SECRET_HEADERS,
        ["Authorization", "Proxy-Authorization", "Cookie", "Set-Cookie", "X-Api-Key", "X-Auth-Token", "Api-Key"]
    );
    assert_eq!(
        BUILTIN_SECRET_QUERY_NAMES,
        ["api_key", "apikey", "access_token", "token", "key", "signature"]
    );
    assert_eq!(REDACTED, "[redacted]");
}

#[test]
fn every_builtin_secret_header_is_redacted_in_request_and_response_metadata() {
    let secrets = SecretNames::builtin();
    for name in BUILTIN_SECRET_HEADERS {
        // Header names match case-insensitively: try the documented spelling
        // and a lowercase one.
        for spelling in [name.to_string(), name.to_ascii_lowercase()] {
            let map = headers(&[(&spelling, SENTINEL), ("X-Trace", "visible-trace")]);

            let request = capture_request(&secrets, "https://api.example.test/v1/items", &map);
            assert_eq!(values(&request.headers, name), [REDACTED], "request header {spelling}");
            assert_eq!(values(&request.headers, "x-trace"), ["visible-trace"]);
            assert!(!format!("{request:?} {request}").contains(SENTINEL), "request {spelling}");

            let response = capture_response(&secrets, &map, None);
            assert_eq!(values(&response.headers, name), [REDACTED], "response header {spelling}");
            assert_eq!(values(&response.headers, "x-trace"), ["visible-trace"]);
            assert!(!format!("{response:?} {response}").contains(SENTINEL), "response {spelling}");
        }
    }
}

#[test]
fn every_builtin_secret_query_name_is_redacted_in_the_url_and_query_fields() {
    let secrets = SecretNames::builtin();
    for name in BUILTIN_SECRET_QUERY_NAMES {
        let request = capture_request(
            &secrets,
            &format!("https://api.example.test/v1/items?{name}={SENTINEL}&page=2"),
            &HeaderMap::new(),
        );
        assert_eq!(
            request.url.as_str(),
            format!("https://api.example.test/v1/items?{name}=[redacted]&page=2"),
            "{name}"
        );
        assert_eq!(
            request.query,
            [(name.to_string(), REDACTED.to_string()), ("page".to_string(), "2".to_string())],
            "{name}"
        );
        assert!(!format!("{request:?} {request}").contains(SENTINEL), "{name}");
    }
}

#[test]
fn query_names_match_exactly_after_decoding() {
    let secrets = SecretNames::builtin();
    let request = capture_request(
        &secrets,
        &format!("https://api.example.test/v1?TOKEN=upper&access%5Ftoken={SENTINEL}&tokens=plural"),
        &HeaderMap::new(),
    );
    // `TOKEN` and `tokens` are different names; the encoded `access_token` is
    // the built-in one and keeps its original spelling.
    assert_eq!(
        request.url.as_str(),
        "https://api.example.test/v1?TOKEN=upper&access%5Ftoken=[redacted]&tokens=plural"
    );
    assert_eq!(
        request.query,
        [
            ("TOKEN".to_string(), "upper".to_string()),
            ("access_token".to_string(), REDACTED.to_string()),
            ("tokens".to_string(), "plural".to_string()),
        ]
    );
}

#[test]
fn api_and_authentication_names_join_the_builtin_names() {
    // An API's own secret names and its authentication names (a custom key
    // header, an `ApiKeyParam` query name) are added the same way.
    let secrets = SecretNames::builtin().with_names(["X-Custom-Secret", "appid"]);
    let map = headers(&[("x-custom-secret", SENTINEL), ("Authorization", SENTINEL), ("Accept", "application/json")]);
    let request = capture_request(&secrets, &format!("https://api.example.test/v1?appid={SENTINEL}&key={SENTINEL}&q=rust"), &map);

    assert_eq!(values(&request.headers, "x-custom-secret"), [REDACTED]);
    assert_eq!(values(&request.headers, "authorization"), [REDACTED]);
    assert_eq!(values(&request.headers, "accept"), ["application/json"]);
    assert_eq!(request.url.as_str(), "https://api.example.test/v1?appid=[redacted]&key=[redacted]&q=rust");
    assert!(!format!("{request:?}").contains(SENTINEL));

    // Header names compare case-insensitively whichever side is mixed case;
    // query names do not.
    assert!(secrets.is_secret_header("x-CUSTOM-secret"));
    assert!(secrets.is_secret_header("AUTHORIZATION"));
    assert!(secrets.is_secret_query("appid"));
    assert!(!secrets.is_secret_query("APPID"));

    // Without the extra names the same values are not secret.
    let builtin = capture_request(&SecretNames::builtin(), "https://api.example.test/v1?appid=visible", &map);
    assert_eq!(builtin.url.as_str(), "https://api.example.test/v1?appid=visible");
    assert_eq!(values(&builtin.headers, "x-custom-secret"), [SENTINEL]);
}

#[test]
fn ordinary_duplicate_query_values_keep_their_order() {
    let request = capture_request(
        &SecretNames::builtin(),
        &format!("https://api.example.test/v1?tag=a&token={SENTINEL}&tag=b&tag=a"),
        &HeaderMap::new(),
    );
    assert_eq!(request.url.as_str(), "https://api.example.test/v1?tag=a&token=[redacted]&tag=b&tag=a");
    let expected: Vec<(String, String)> = [("tag", "a"), ("token", REDACTED), ("tag", "b"), ("tag", "a")]
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    assert_eq!(request.query, expected);
}

#[test]
fn repeated_headers_stay_repeated_in_order() {
    let map = headers(&[
        ("Set-Cookie", &format!("session={SENTINEL}")),
        ("X-Rate", "first"),
        ("Set-Cookie", "theme=dark"),
        ("X-Rate", "second"),
    ]);
    let response = capture_response(&SecretNames::builtin(), &map, None);
    assert_eq!(values(&response.headers, "set-cookie"), [REDACTED, REDACTED]);
    assert_eq!(values(&response.headers, "x-rate"), ["first", "second"]);
}

#[test]
fn url_user_information_is_always_redacted() {
    let secrets = SecretNames::builtin();
    for (url, expected) in [
        (format!("https://user:{SENTINEL}@api.example.test/p?page=1"), "https://[redacted]@api.example.test/p?page=1"),
        (format!("https://{SENTINEL}@api.example.test/p"), "https://[redacted]@api.example.test/p"),
        ("https://api.example.test:8443/p".to_string(), "https://api.example.test:8443/p"),
    ] {
        let request = capture_request(&secrets, &url, &HeaderMap::new());
        assert_eq!(request.url.as_str(), expected);
    }
}

#[test]
fn location_headers_keep_their_destination_and_lose_their_secrets() {
    let secrets = SecretNames::builtin();
    for (location, expected) in [
        (
            format!("https://other.example.test/next?page=3&access_token={SENTINEL}"),
            "https://other.example.test/next?page=3&access_token=[redacted]".to_string(),
        ),
        (
            format!("/next?page=3&token={SENTINEL}#section"),
            "/next?page=3&token=[redacted]#section".to_string(),
        ),
        (
            format!("//user:{SENTINEL}@other.example.test/x?key={SENTINEL}"),
            "//[redacted]@other.example.test/x?key=[redacted]".to_string(),
        ),
        (
            format!("https://app.example.test/callback#access_token={SENTINEL}&state=ok"),
            "https://app.example.test/callback#access_token=[redacted]&state=ok".to_string(),
        ),
        ("relative/path".to_string(), "relative/path".to_string()),
    ] {
        for header in ["Location", "Content-Location"] {
            let response = capture_response(&secrets, &headers(&[(header, &location)]), None);
            assert_eq!(values(&response.headers, header), [expected.as_str()], "{header}: {location}");
        }
    }
}

#[test]
fn an_unreadable_location_is_redacted_whole() {
    let mut map = HeaderMap::new();
    map.insert("location", HeaderValue::from_bytes(b"/next?token=\xff").expect("opaque header bytes"));
    let response = capture_response(&SecretNames::builtin(), &map, None);
    assert_eq!(values(&response.headers, "location"), [REDACTED]);
}

#[test]
fn the_request_id_comes_from_the_named_header_only() {
    let secrets = SecretNames::builtin();
    let map = headers(&[("x-github-request-id", "ABCD:1234"), ("x-request-id", "other")]);

    let named = capture_response(&secrets, &map, Some("X-GitHub-Request-Id"));
    assert_eq!(named.request_id.as_deref(), Some("ABCD:1234"));

    let absent = capture_response(&secrets, &map, Some("x-amzn-requestid"));
    assert_eq!(absent.request_id, None);

    let unnamed = capture_response(&secrets, &map, None);
    assert_eq!(unnamed.request_id, None);
}

#[test]
fn a_request_id_from_a_secret_header_is_redacted() {
    let secrets = SecretNames::builtin().with_name("X-Request-Id");
    let response = capture_response(&secrets, &headers(&[("x-request-id", SENTINEL)]), Some("x-request-id"));
    assert_eq!(response.request_id.as_deref(), Some(REDACTED));
    assert_eq!(values(&response.headers, "x-request-id"), [REDACTED]);
    assert!(!format!("{response:?} {response}").contains(SENTINEL));
}
