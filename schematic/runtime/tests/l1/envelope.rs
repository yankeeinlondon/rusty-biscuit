//! The response envelope, its failure kinds, the outer transport error, and
//! their safe text (R3, R4, R6, R8, R10–R13 groundwork; network
//! classification follows with managed sends).

use std::time::Duration;

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use schematic_runtime::redact::REDACTED;
use schematic_runtime::{
    ApiError, AuthMethod, BuildError, Credential, CredentialKind, ErrorBody, ErrorKind, FailureCategory,
    NoDeclaredError, Outcome, RedirectRefusal, RequestMeta, Response, ResponseMeta, SecretNames, TransportError,
    TransportErrorKind,
};
use url::Url;

const SENTINEL: &str = "SENTINEL-7f3a9c";

/// A declared error body, as a later definition will declare one.
#[derive(Debug, Clone, PartialEq)]
struct ProviderError {
    message: String,
}

impl ErrorBody for ProviderError {
    fn decode_body(body: &[u8]) -> Option<Self> {
        let message = std::str::from_utf8(body).ok()?.strip_prefix("error: ")?;
        Some(Self { message: message.to_string() })
    }
}

fn secrets() -> SecretNames {
    SecretNames::builtin().with_name("X-Vendor-Secret")
}

/// A request carrying the sentinel in a built-in secret header, a built-in
/// secret query parameter, and an API-marked secret name.
fn request() -> RequestMeta {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", HeaderValue::from_str(&format!("Bearer {SENTINEL}")).expect("value"));
    headers.insert("x-vendor-secret", HeaderValue::from_static(SENTINEL));
    headers.insert("accept", HeaderValue::from_static("application/json"));
    let url = Url::parse(&format!("https://api.example.test/v1/items?token={SENTINEL}&page=2")).expect("url");
    RequestMeta::capture(&secrets(), "list_items", Method::GET, &url, &headers, AuthMethod::PerCall)
}

fn meta(status: StatusCode) -> ResponseMeta {
    let mut headers = HeaderMap::new();
    headers.insert("set-cookie", HeaderValue::from_str(&format!("session={SENTINEL}")).expect("value"));
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    ResponseMeta::capture(&secrets(), request(), status, &headers, None, Duration::from_millis(12))
}

fn failure<E>(status: StatusCode, kind: ErrorKind<E>, body: &str) -> Response<Vec<u32>, E> {
    Response {
        meta: meta(status),
        outcome: Outcome::Failure(ApiError {
            kind,
            body: Bytes::from(body.to_string()),
        }),
    }
}

fn every_failure_kind() -> Vec<ErrorKind<ProviderError>> {
    vec![
        ErrorKind::Declared(ProviderError { message: "quota".to_string() }),
        ErrorKind::Undeclared,
        ErrorKind::MalformedError,
        ErrorKind::MalformedSuccess,
        ErrorKind::BodyInterrupted,
        ErrorKind::RedirectRefused {
            reason: RedirectRefusal::CrossOrigin,
        },
        ErrorKind::RedirectFailed {
            target: secrets().redact_url_reference(&format!("https://b.example.test/x?token={SENTINEL}")),
            category: FailureCategory::Connect,
        },
        ErrorKind::Hook,
    ]
}

#[test]
fn a_success_exposes_its_value_through_every_accessor() {
    let response: Response<Vec<u32>, ProviderError> = Response {
        meta: meta(StatusCode::OK),
        outcome: Outcome::Success(vec![1, 2]),
    };
    assert_eq!(response.data(), Some(&vec![1, 2]));
    assert!(response.error().is_none());
    assert!(!response.is_decode_failure());
    assert!(!response.is_interrupted());

    let (kept, outcome) = response.clone().into_parts();
    assert_eq!(kept.status, StatusCode::OK);
    assert_eq!(outcome, Outcome::Success(vec![1, 2]));
    assert_eq!(response.into_result().expect("success"), vec![1, 2]);
}

#[test]
fn a_failure_exposes_its_error_and_keeps_metadata_through_into_parts() {
    let response = failure(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorKind::Declared(ProviderError { message: "quota".to_string() }),
        "error: quota",
    );
    assert!(response.data().is_none());
    let error = response.error().expect("failure");
    assert_eq!(error.declared(), Some(&ProviderError { message: "quota".to_string() }));
    assert_eq!(error.body, Bytes::from_static(b"error: quota"));

    let (kept, outcome) = response.clone().into_parts();
    assert_eq!(kept.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(matches!(outcome, Outcome::Failure(ApiError { kind: ErrorKind::Declared(_), .. })));

    let error = response.into_result().expect_err("failure");
    assert_eq!(error.body, Bytes::from_static(b"error: quota"));
}

#[test]
fn decode_and_interruption_helpers_are_true_only_for_their_own_kind() {
    for kind in every_failure_kind() {
        let malformed_success = matches!(kind, ErrorKind::MalformedSuccess);
        let interrupted = matches!(kind, ErrorKind::BodyInterrupted);
        let response = failure(StatusCode::OK, kind.clone(), "partial");
        assert_eq!(response.is_decode_failure(), malformed_success, "{kind:?}");
        assert_eq!(response.is_interrupted(), interrupted, "{kind:?}");
        let error = response.error().expect("failure");
        assert_eq!(error.is_decode_failure(), malformed_success, "{kind:?}");
        assert_eq!(error.is_interrupted(), interrupted, "{kind:?}");
        assert_eq!(error.declared().is_some(), matches!(kind, ErrorKind::Declared(_)), "{kind:?}");
    }
}

#[test]
fn metadata_records_every_field_on_success_and_failure() {
    let success: Response<Vec<u32>, ProviderError> = Response {
        meta: meta(StatusCode::OK),
        outcome: Outcome::Success(Vec::new()),
    };
    let failed = failure::<ProviderError>(StatusCode::NOT_FOUND, ErrorKind::Undeclared, "");
    for (response, status) in [(success, StatusCode::OK), (failed, StatusCode::NOT_FOUND)] {
        let meta = &response.meta;
        assert_eq!(meta.status, status);
        assert_eq!(meta.request.method, Method::GET);
        assert_eq!(meta.request.endpoint_id, "list_items");
        assert_eq!(meta.request.url.as_str(), "https://api.example.test/v1/items?token=[redacted]&page=2");
        assert_eq!(
            meta.request.query,
            [("token".to_string(), REDACTED.to_string()), ("page".to_string(), "2".to_string())]
        );
        assert_eq!(meta.request.headers.get("authorization").expect("kept"), REDACTED);
        assert_eq!(meta.request.headers.get("x-vendor-secret").expect("kept"), REDACTED);
        assert_eq!(meta.request.headers.get("accept").expect("kept"), "application/json");
        assert_eq!(meta.headers.get("set-cookie").expect("kept"), REDACTED);
        assert_eq!(meta.headers.get("content-type").expect("kept"), "application/json");
        assert_eq!(meta.request.auth, AuthMethod::PerCall);
        assert_eq!(meta.elapsed, Duration::from_millis(12));
        assert_eq!(meta.request.page, None);
        assert_eq!(meta.request_id, None);
    }
}

#[test]
fn stored_metadata_and_display_never_contain_a_secret_or_a_body() {
    // The body carries the sentinel too, as a provider that echoes a key
    // would; it stays in the evidence and out of the text.
    let body = format!("error: invalid key {SENTINEL}");
    let mut texts = vec![
        Response::<Vec<u32>, ProviderError> {
            meta: meta(StatusCode::OK),
            outcome: Outcome::Success(Vec::new()),
        }
        .to_string(),
    ];
    for kind in every_failure_kind() {
        let response = failure(StatusCode::UNAUTHORIZED, kind, &body);
        let error = response.error().expect("failure");
        assert_eq!(error.body, Bytes::from(body.clone()), "raw evidence is kept unchanged");
        assert!(!format!("{:?}", response.meta).contains(SENTINEL));
        texts.push(response.to_string());
        texts.push(error.to_string());
        texts.push(error.kind.to_string());
    }
    for text in &texts {
        assert!(!text.contains(SENTINEL), "secret in {text:?}");
        assert!(!text.contains("invalid key"), "body in {text:?}");
    }
    assert!(texts[0].contains("200 OK from GET https://api.example.test/v1/items?token=[redacted]&page=2"));
    assert!(texts.iter().any(|text| text.contains("redirect to https://b.example.test/x?token=[redacted] failed")));
}

#[test]
fn the_debug_limitation_is_documented_on_the_error_types() {
    let source = include_str!("../../src/response.rs");
    assert!(source.contains("## Debug can print provider data"));
    assert!(source.contains("echoes a credential in an error body"));
}

#[test]
fn transport_error_text_shows_the_redacted_url_and_no_secret() {
    for kind in [
        TransportErrorKind::Connect,
        TransportErrorKind::Timeout,
        TransportErrorKind::Tls,
        TransportErrorKind::Other,
        TransportErrorKind::InvalidUrl,
        TransportErrorKind::InvalidArgument(schematic_runtime::InvalidArgument::new("per_page exceeds 100")),
        TransportErrorKind::CrossOriginRejected {
            target: secrets().redact_url_reference(&format!("https://b.example.test/?token={SENTINEL}")),
        },
        TransportErrorKind::DeadlineExceeded {
            completed_requests: 2,
            deadline_elapsed: Duration::from_millis(400),
        },
    ] {
        let error = TransportError::new(request(), kind.clone(), Duration::from_millis(30));
        let display = error.to_string();
        let debug = format!("{error:?}");
        for text in [&display, &debug] {
            assert!(!text.contains(SENTINEL), "{kind:?}: {text}");
            assert!(text.contains("token=[redacted]"), "{kind:?}: {text}");
            assert!(text.contains("page=2"), "{kind:?}: {text}");
        }
        assert!(display.contains("for GET https://api.example.test/v1/items?token=[redacted]&page=2"));
    }
    let deadline = TransportError::new(
        request(),
        TransportErrorKind::DeadlineExceeded {
            completed_requests: 2,
            deadline_elapsed: Duration::from_millis(400),
        },
        Duration::ZERO,
    );
    assert!(deadline.to_string().starts_with("deadline exceeded after 2 completed request(s)"));
}

#[test]
fn an_endpoint_without_a_declared_error_reports_undeclared_failures() {
    // Read through a bound, as the runtime classifies replies.
    fn declares<E: ErrorBody>() -> bool {
        E::DECLARED
    }
    assert!(!declares::<NoDeclaredError>());
    assert_eq!(NoDeclaredError::decode_body(b"{\"message\":\"nope\"}"), None);
    assert!(declares::<ProviderError>());
    assert_eq!(
        ProviderError::decode_body(b"error: quota"),
        Some(ProviderError { message: "quota".to_string() })
    );

    let response = failure::<NoDeclaredError>(StatusCode::INTERNAL_SERVER_ERROR, ErrorKind::Undeclared, "oops");
    let error = response.error().expect("failure");
    assert_eq!(error.body, Bytes::from_static(b"oops"));
    // The declared arm needs no handling: the placeholder has no values.
    match &error.kind {
        ErrorKind::Declared(never) => match *never {},
        ErrorKind::Undeclared => {}
        other => panic!("unexpected kind {other:?}"),
    }
}

#[test]
fn build_errors_name_the_option_and_both_policies() {
    let refused = BuildError::OriginPolicyNotPermitted {
        definition: "deny",
        requested: "same_origin",
    };
    assert_eq!(
        refused.to_string(),
        "cross-origin policy `same_origin` is not permitted over the API's `deny` policy"
    );
    assert_eq!(
        BuildError::TransportOptionConflict { option: "proxy" }.to_string(),
        "`proxy` is owned by the attached shared transport"
    );
}

#[test]
fn a_credential_never_prints_its_value() {
    for (credential, kind) in [
        (Credential::access_token(SENTINEL), CredentialKind::AccessToken),
        (Credential::api_key(SENTINEL), CredentialKind::ApiKey),
        (Credential::basic(SENTINEL, SENTINEL), CredentialKind::Basic),
    ] {
        assert_eq!(credential.kind(), kind);
        let text = format!("{credential} {credential:?}");
        assert!(!text.contains(SENTINEL), "{text}");
        assert!(text.contains(REDACTED), "{text}");
    }
}
