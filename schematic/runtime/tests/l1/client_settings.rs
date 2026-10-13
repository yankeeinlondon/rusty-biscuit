//! Client setup in one step: `Client::new` never fails, chained settings are
//! validated once (by `build` or by the first call, which reports an invalid
//! setting as `TransportErrorKind::InvalidClient` before any network work),
//! clones share that validation and its connection pool, `api_key` follows
//! the definition's placement, and `api_client!` gives a generated struct
//! the whole surface without a trait import.

use std::sync::atomic::Ordering;
use std::time::Duration;

use schematic_runtime::{
    ApiDefinition, AuthConfig, AuthMethod, BuildError, CallOptions, Client, Credential, CredentialPlacement, Json,
    SharedTransport, TransportError, TransportErrorKind, Url,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::builder::counting_server;
use crate::support::{Call, Item, SENTINEL, TestApi, builder};

type GetItem = Call<Json<Item>>;

fn bare() -> ApiDefinition {
    ApiDefinition::new("https://api.example.invalid")
}

fn accepting(placements: impl IntoIterator<Item = CredentialPlacement>) -> ApiDefinition {
    let auth = placements.into_iter().fold(AuthConfig::none(), AuthConfig::accept);
    bare().auth(auth)
}

async fn item_server() -> MockServer {
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

/// One client per kind of invalid setting, pointed at `base_url` unless the
/// base URL is the invalid setting, with the error each must report.
fn invalid_settings(base_url: &str, transport: &SharedTransport) -> Vec<(Client<TestApi>, BuildError)> {
    vec![
        (builder(bare(), "not a url"), BuildError::InvalidBaseUrl { reason: "not an absolute URL" }),
        (
            builder(bare(), base_url).default_header("bad header", "x"),
            BuildError::InvalidHeader { name: "bad header".into() },
        ),
        (
            builder(bare(), base_url).default_header("Authorization", SENTINEL),
            BuildError::AuthBearingDefaultHeader { name: "Authorization".into() },
        ),
        (
            builder(bare(), base_url).credentials(Credential::basic("user", SENTINEL)),
            BuildError::IncompatibleCredential { supplied: "Basic" },
        ),
        (
            builder(bare(), base_url).transport(transport).proxy("http://127.0.0.1:2"),
            BuildError::TransportOptionConflict { option: "proxy" },
        ),
    ]
}

fn assert_invalid_client(error: &TransportError, expected: &BuildError) {
    assert_eq!(error.kind, TransportErrorKind::InvalidClient(expected.clone()));
    assert_eq!(error.request.endpoint_id, "call");
    let shown = format!("{error} {error:?}");
    assert!(shown.contains("invalid client"), "{shown}");
    assert!(!shown.contains(SENTINEL), "{shown}");
}

#[tokio::test]
async fn new_never_fails_and_the_first_call_reports_each_invalid_setting_without_network_work() {
    let server = item_server().await;
    let transport = SharedTransport::builder().build().expect("transport");
    let link = Url::parse(&server.uri()).expect("server URL");

    for (client, expected) in invalid_settings(&server.uri(), &transport) {
        let sent = client.send(GetItem::get("/item")).full().await.expect_err("refused");
        assert_invalid_client(&sent, &expected);
        let raw = client.send_raw(GetItem::get("/item"), &CallOptions::new()).await.expect_err("refused");
        assert_invalid_client(&raw, &expected);
        let checked = client.check_link::<GetItem>(&link).expect_err("refused");
        assert_invalid_client(&checked, &expected);
    }

    assert!(received(&server).await.is_empty(), "an invalid client sends nothing");
}

#[tokio::test]
async fn build_reports_the_same_errors_and_returns_the_same_client_type() {
    let server = item_server().await;
    let transport = SharedTransport::builder().build().expect("transport");
    for (client, expected) in invalid_settings(&server.uri(), &transport) {
        assert_eq!(client.build().expect_err("refused at build"), expected);
    }

    let built: Client<TestApi> = builder(bare(), &server.uri()).build().expect("valid settings build");
    let response = built.send(GetItem::get("/item")).full().await.expect("head");
    assert_eq!(response.data(), Some(&Item { id: 1 }));
}

#[tokio::test]
async fn a_client_and_its_clones_share_one_validation_and_connection_pool() {
    let (url, accepted, _server) = counting_server().await;
    // Cloned before validation: whichever clone sends first validates for
    // both.
    let unbuilt = builder(bare(), &url);
    let clone = unbuilt.clone();
    // Validated by `build`, then cloned.
    let (built_url, built_accepted, _built_server) = counting_server().await;
    let built = builder(bare(), &built_url).build().expect("client");
    let built_clone = built.clone();

    for client in [&clone, &unbuilt, &clone, &built, &built_clone] {
        let response = client.send(GetItem::get("/item")).full().await.expect("head");
        assert_eq!(response.data(), Some(&Item { id: 1 }));
    }

    assert_eq!(accepted.load(Ordering::SeqCst), 1, "the lazily validated clones share one pool");
    assert_eq!(built_accepted.load(Ordering::SeqCst), 1, "the built clones share one pool");
}

#[tokio::test]
async fn a_setting_applied_after_build_takes_effect_and_leaves_earlier_clones_alone() {
    let server = item_server().await;
    let built = builder(bare(), &server.uri()).build().expect("client");
    let later = built.clone().default_header("x-after", "1");
    let broken = built.clone().base_url("not a url");

    built.send(GetItem::get("/item")).full().await.expect("head");
    later.send(GetItem::get("/item")).full().await.expect("head");
    let error = broken.send(GetItem::get("/item")).full().await.expect_err("refused");
    assert!(matches!(error.kind, TransportErrorKind::InvalidClient(BuildError::InvalidBaseUrl { .. })), "{error:?}");

    let requests = received(&server).await;
    assert_eq!(requests.len(), 2);
    assert!(requests[0].headers.get("x-after").is_none(), "the earlier client is unchanged");
    assert_eq!(requests[1].headers.get("x-after").map(|value| value.as_bytes()), Some(&b"1"[..]));
}

/// Reads where a credential landed in a received request.
type SentValue = fn(&wiremock::Request) -> Option<String>;

#[tokio::test]
async fn api_key_goes_to_the_first_placement_that_takes_a_single_secret() {
    let header_key = CredentialPlacement::ApiKeyHeader {
        header: "X-Api-Key".to_string(),
        prefix: Some("token ".to_string()),
    };
    let query_key = || CredentialPlacement::ApiKeyQuery { name: "key".to_string() };
    let cases: [(&str, ApiDefinition, SentValue, &str); 4] = [
        (
            "bearer",
            accepting([CredentialPlacement::Bearer { header: None }]),
            |request| request.headers.get("authorization").map(|value| value.to_str().unwrap().to_string()),
            "Bearer k-1",
        ),
        (
            "header key",
            accepting([header_key]),
            |request| request.headers.get("x-api-key").map(|value| value.to_str().unwrap().to_string()),
            "token k-1",
        ),
        ("query key", accepting([query_key()]), |request| request.url.query().map(str::to_string), "key=k-1"),
        (
            "Basic, then a query key, then bearer",
            accepting([CredentialPlacement::Basic, query_key(), CredentialPlacement::Bearer { header: None }]),
            |request| {
                assert!(request.headers.get("authorization").is_none(), "only one placement is used");
                request.url.query().map(str::to_string)
            },
            "key=k-1",
        ),
    ];

    for (label, definition, sent, expected) in cases {
        let server = item_server().await;
        let client = builder(definition, &server.uri()).api_key("k-1");
        let response = client.send(GetItem::get("/item")).full().await.expect("head");
        assert_eq!(response.meta.request.auth, AuthMethod::ClientLevel, "{label}");
        let requests = received(&server).await;
        assert_eq!(sent(&requests[0]).as_deref(), Some(expected), "{label}");
    }
}

#[tokio::test]
async fn api_key_on_an_api_without_a_single_secret_placement_is_refused() {
    let server = item_server().await;
    for (label, definition) in [
        ("Basic only", accepting([CredentialPlacement::Basic])),
        ("no authentication", bare()),
    ] {
        let expected = BuildError::IncompatibleCredential { supplied: "API key" };
        let client = builder(definition, &server.uri()).api_key(SENTINEL);
        assert_eq!(client.clone().build().expect_err(label), expected, "{label}");
        let error = client.send(GetItem::get("/item")).full().await.expect_err(label);
        assert_invalid_client(&error, &expected);
    }
    assert!(received(&server).await.is_empty(), "a refused key sends nothing");
}

#[tokio::test]
async fn the_last_client_credential_set_wins() {
    let server = item_server().await;
    let definition = accepting([CredentialPlacement::Bearer { header: None }, CredentialPlacement::Basic]);
    let client = builder(definition, &server.uri()).api_key("first").credentials(Credential::basic("u", "p"));
    client.send(GetItem::get("/item")).full().await.expect("head");

    let requests = received(&server).await;
    assert_eq!(
        requests[0].headers.get("authorization").map(|value| value.to_str().unwrap().to_string()),
        Some("Basic dTpw".to_string())
    );
}

// The macro adds every setting; this test calls only some of them.
#[allow(dead_code)]
mod generated {
    use schematic_runtime::{Client, CredentialPlacement};

    use super::accepting;
    use crate::support::TestApi;

    /// A generated API's client struct, as `schematic-gen` emits it.
    #[derive(Clone, Debug)]
    pub struct Example(Client<TestApi>);

    schematic_runtime::api_client!(Example, TestApi);

    impl Example {
        pub fn new(base_url: &str) -> Self {
            Self(Client::new(accepting([CredentialPlacement::Bearer { header: None }])).base_url(base_url))
        }
    }
}

#[tokio::test]
async fn the_api_client_macro_gives_the_struct_every_setting_and_send_without_a_trait_import() {
    let server = item_server().await;
    let link = Url::parse(&server.uri()).expect("server URL");
    let client: generated::Example = generated::Example::new(&server.uri())
        .timeout(Duration::from_secs(5))
        .user_agent("example-agent")
        .default_header("x-example", "1")
        .api_key(SENTINEL)
        .pre_response_json(|_, value| Ok(value))
        .mutate_response::<GetItem, _>(|_, item| {
            item.id += 1;
            Ok(())
        })
        .build()
        .expect("valid settings build");

    let response = client.send(GetItem::get("/item")).full().await.expect("head");
    assert_eq!(response.data(), Some(&Item { id: 2 }), "the registered mutator ran");
    let raw = client.send_raw(GetItem::get("/item"), &CallOptions::new()).await.expect("head");
    assert_eq!(&raw.body[..], br#"{"id":1}"#);
    client.check_link::<GetItem>(&link).expect("same origin");
    assert!(!format!("{client:?}").contains(SENTINEL), "{client:?}");

    let requests = received(&server).await;
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(request.headers.get("x-example").map(|value| value.as_bytes()), Some(&b"1"[..]));
        assert_eq!(request.headers.get("user-agent").map(|value| value.as_bytes()), Some(&b"example-agent"[..]));
        assert_eq!(
            request.headers.get("authorization").map(|value| value.to_str().unwrap().to_string()),
            Some(format!("Bearer {SENTINEL}"))
        );
    }

    let invalid = generated::Example::new("not a url");
    let error = invalid.send(GetItem::get("/item")).full().await.expect_err("refused");
    assert!(matches!(error.kind, TransportErrorKind::InvalidClient(_)), "{error:?}");
    assert!(invalid.build().is_err());
}
