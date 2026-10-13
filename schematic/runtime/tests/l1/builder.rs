//! Client and shared-transport construction against local mocks: base URL
//! validation, timeouts, user agent and default headers, transport ownership
//! of connection options, connection reuse, proxies, and TLS roots
//! (R17, R18). That no constructor lets the HTTP library follow a redirect
//! (R19) is proved in `redirects.rs`.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use http::Method;
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use schematic_runtime::{
    ApiDefinition, BuildError, CallOptions, Client, DEFAULT_TIMEOUT, Json, RequestParts, SharedTransport,
    TransportErrorKind,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::{JoinHandle, JoinSet};
use tokio_rustls::TlsAcceptor;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, TestApi, builder, listener, plain_client, read_head, write_all};

const ITEM_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 8\r\n\r\n{\"id\":1}";

fn definition() -> ApiDefinition {
    ApiDefinition::new("https://api.example.invalid")
}

fn build_error(base_url: &str) -> BuildError {
    builder(definition(), base_url).build().expect_err("the base URL is refused")
}

async fn item_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/item"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"id":1}"#, "application/json"))
        .mount(&server)
        .await;
    server
}

/// Aborts the wrapped server task when the test ends, pass or fail.
pub(crate) struct ServerTask(JoinHandle<()>);

impl Drop for ServerTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// A keep-alive HTTP/1.1 mock that answers every request on a connection with
/// an `Item` and counts accepted connections.
pub(crate) async fn counting_server() -> (String, Arc<AtomicUsize>, ServerTask) {
    let (listener, url) = listener().await;
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&accepted);
    let task = tokio::spawn(async move {
        // Dropping the set when this task is aborted aborts every connection.
        let mut connections = JoinSet::new();
        while let Ok((mut stream, _)) = listener.accept().await {
            counter.fetch_add(1, Ordering::SeqCst);
            connections.spawn(async move {
                while !read_head(&mut stream).await.is_empty() {
                    write_all(&mut stream, ITEM_RESPONSE).await;
                }
            });
        }
    });
    (url, accepted, ServerTask(task))
}

/// A mock that accepts one connection, reads its head, and reports it,
/// answering with an `Item` and closing.
async fn recording_server() -> (String, oneshot::Receiver<String>, ServerTask) {
    let (listener, url) = listener().await;
    let (sender, receiver) = oneshot::channel();
    let task = tokio::spawn(async move {
        let Ok((mut stream, _)) = listener.accept().await else { return };
        let head = read_head(&mut stream).await;
        let _ = sender.send(head);
        write_all(
            &mut stream,
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 8\r\nConnection: close\r\n\r\n{\"id\":1}",
        )
        .await;
    });
    (url, receiver, ServerTask(task))
}

#[tokio::test]
async fn a_base_url_that_is_not_an_absolute_http_url_without_user_information_is_refused_at_build() {
    for base_url in ["not a url", "/relative/path", "api.example.com", "http://"] {
        assert!(
            matches!(build_error(base_url), BuildError::InvalidBaseUrl { .. }),
            "{base_url} is refused"
        );
    }
    for base_url in ["ftp://files.example.com", "file:///etc/hosts", "ws://socket.example.com"] {
        assert_eq!(
            build_error(base_url),
            BuildError::InvalidBaseUrl {
                reason: "scheme is not http or https"
            },
            "{base_url} has a non-HTTP scheme"
        );
    }
    for base_url in ["http://user:pass@api.example.com", "https://user@api.example.com"] {
        assert_eq!(
            build_error(base_url),
            BuildError::InvalidBaseUrl {
                reason: "carries user information"
            },
            "{base_url} carries user information"
        );
    }
}

#[tokio::test]
async fn a_valid_base_url_redirects_every_call_to_the_mock() {
    let server = item_server().await;
    let client = builder(definition(), &format!("{}/", server.uri())).build().expect("client");

    let response = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head arrived");

    assert_eq!(response.meta.status, 200);
    assert_eq!(response.data(), Some(&Item { id: 1 }));
    assert_eq!(response.meta.request.url.as_str(), format!("{}/item", server.uri()));
    assert_eq!(server.received_requests().await.expect("recording").len(), 1);
}

#[test]
fn the_default_per_request_timeout_is_thirty_seconds() {
    assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(30));
}

#[tokio::test]
async fn a_server_that_never_answers_times_out_after_the_configured_timeout() {
    let (listener, url) = listener().await;
    let (release, released) = oneshot::channel::<()>();
    let _server = ServerTask(tokio::spawn(async move {
        let Ok((mut stream, _)) = listener.accept().await else { return };
        read_head(&mut stream).await;
        // Holding the socket open keeps the failure a timeout, not a reset.
        let _ = released.await;
        drop(stream);
    }));
    let timeout = Duration::from_millis(200);
    let client = builder(definition(), &url).timeout(timeout).build().expect("client");

    let started = std::time::Instant::now();
    let error = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect_err("no head arrives");
    let waited = started.elapsed();
    let _ = release.send(());

    assert_eq!(error.kind, TransportErrorKind::Timeout);
    assert!(error.elapsed >= timeout, "elapsed {:?} covers the timeout", error.elapsed);
    assert!(waited < Duration::from_secs(5), "the send ended after {waited:?}");
}

#[tokio::test]
async fn the_user_agent_and_default_headers_reach_the_server_and_a_request_header_replaces_a_default() {
    let server = item_server().await;
    let client = builder(definition(), &server.uri())
        .user_agent("schematic-test/1.0")
        .default_header("x-default", "from-builder")
        .default_header("x-kept", "kept")
        .build()
        .expect("client");

    client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head arrived");
    let mut parts = RequestParts::new(Method::GET, "/item");
    parts.headers = vec![("X-Default".into(), "from-request".into())];
    client.send(Call::<Json<Item>>::new(parts), &CallOptions::new()).await.expect("head arrived");

    let requests = server.received_requests().await.expect("recording");
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(request.headers["user-agent"], "schematic-test/1.0");
        assert_eq!(request.headers["x-kept"], "kept");
    }
    let first: Vec<_> = requests[0].headers.get_all("x-default").iter().collect();
    assert_eq!(first, ["from-builder"]);
    let second: Vec<_> = requests[1].headers.get_all("x-default").iter().collect();
    assert_eq!(second, ["from-request"], "the request header replaces the default rather than adding to it");
}

#[tokio::test]
async fn an_invalid_default_header_or_user_agent_is_refused_at_build() {
    let error = builder(definition(), "http://127.0.0.1:1")
        .default_header("bad name", "value")
        .build()
        .expect_err("invalid header name");
    assert_eq!(error, BuildError::InvalidHeader { name: "bad name".into() });

    let error = builder(definition(), "http://127.0.0.1:1")
        .default_header("x-ok", "line\nbreak")
        .build()
        .expect_err("invalid header value");
    assert_eq!(error, BuildError::InvalidHeader { name: "x-ok".into() });

    let error = builder(definition(), "http://127.0.0.1:1")
        .user_agent("agent\r\n")
        .build()
        .expect_err("invalid user agent");
    assert_eq!(error, BuildError::InvalidHeader { name: "user-agent".into() });
}

#[tokio::test]
async fn a_client_attached_to_a_shared_transport_refuses_every_connection_option() {
    let transport = SharedTransport::builder().build().expect("transport");
    let attached = || builder(definition(), "http://127.0.0.1:1").transport(&transport);
    let cases = [
        ("proxy", attached().proxy("http://127.0.0.1:2")),
        ("add_root_certificate", attached().add_root_certificate(b"-----BEGIN CERTIFICATE-----".to_vec())),
        ("danger_accept_invalid_certificates", attached().danger_accept_invalid_certificates()),
        // Only ownership and delivery (`transport::tests`) of the connect
        // timeout are tested; a hanging connect cannot be produced portably on
        // loopback, and enforcing the bound is the HTTP library's job.
        ("connect_timeout", attached().connect_timeout(Duration::from_secs(1))),
    ];
    for (option, conflicting) in cases {
        assert_eq!(
            conflicting.build().expect_err("the transport owns the option"),
            BuildError::TransportOptionConflict { option },
        );
    }
    attached().build().expect("an attached client without connection options builds");
}

#[tokio::test]
async fn an_unparseable_proxy_url_is_refused_at_build() {
    assert_eq!(
        SharedTransport::builder().proxy("not a url").build().expect_err("invalid proxy"),
        BuildError::InvalidProxy
    );
    assert_eq!(
        builder(definition(), "http://127.0.0.1:1").proxy("::bad").build().expect_err("invalid proxy"),
        BuildError::InvalidProxy
    );
}

#[tokio::test]
async fn a_malformed_pem_root_certificate_is_refused_at_build() {
    let garbage = b"-----BEGIN CERTIFICATE-----\nnot base64 at all!\n-----END CERTIFICATE-----\n".to_vec();
    assert_eq!(
        builder(definition(), "http://127.0.0.1:1").add_root_certificate(garbage.clone()).build().expect_err("invalid root"),
        BuildError::InvalidCertificate
    );
    assert_eq!(
        SharedTransport::builder().add_root_certificate(garbage).build().expect_err("invalid root"),
        BuildError::InvalidCertificate
    );
}

#[tokio::test]
async fn clients_on_one_shared_transport_reuse_a_single_connection() {
    let (url, accepted, _server) = counting_server().await;
    let transport = SharedTransport::builder().build().expect("transport");
    let first = builder(definition(), &url).transport(&transport).build().expect("client");
    let second = builder(definition(), &url).transport(&transport).build().expect("client");

    for client in [&first, &second, &first] {
        let response = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head");
        assert_eq!(response.data(), Some(&Item { id: 1 }), "the body is fully drained");
    }

    assert_eq!(accepted.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn standalone_clients_each_open_their_own_connection() {
    let (url, accepted, _server) = counting_server().await;
    let first = plain_client(&url);
    let second = plain_client(&url);

    for client in [&first, &second, &first] {
        let response = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head");
        assert_eq!(response.data(), Some(&Item { id: 1 }));
    }

    assert_eq!(accepted.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_standalone_client_sends_through_its_proxy_in_absolute_form() {
    let (proxy_url, head, _proxy) = recording_server().await;
    let client = builder(definition(), "http://target.invalid").proxy(proxy_url).build().expect("client");

    let response = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head arrived");

    assert_eq!(response.data(), Some(&Item { id: 1 }));
    let head = head.await.expect("the proxy received the request");
    assert_eq!(head.lines().next(), Some("GET http://target.invalid/item HTTP/1.1"));
}

#[tokio::test]
async fn a_client_on_a_shared_transport_sends_through_the_transport_proxy() {
    let (proxy_url, head, _proxy) = recording_server().await;
    let transport = SharedTransport::builder().proxy(proxy_url).build().expect("transport");
    let client = builder(definition(), "http://target.invalid").transport(&transport).build().expect("client");

    let response = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head arrived");

    assert_eq!(response.data(), Some(&Item { id: 1 }));
    let head = head.await.expect("the proxy received the request");
    assert_eq!(head.lines().next(), Some("GET http://target.invalid/item HTTP/1.1"));
}

/// A CA and a `localhost`/`127.0.0.1` leaf it signed.
struct Pki {
    ca_pem: String,
    ca_der: Vec<u8>,
    leaf: CertificateDer<'static>,
    leaf_key: Vec<u8>,
}

fn short_validity(params: &mut CertificateParams) {
    // macOS refuses leaves valid for more than 825 days, so stay well inside.
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock after epoch");
    let epoch = rcgen::date_time_ymd(1970, 1, 1);
    params.not_before = epoch + (now - Duration::from_secs(86_400));
    params.not_after = epoch + (now + Duration::from_secs(30 * 86_400));
}

fn leaf_params() -> CertificateParams {
    let mut params = CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()]).expect("SANs");
    params.distinguished_name.push(DnType::CommonName, "localhost");
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    short_validity(&mut params);
    params
}

fn ca_signed_pki() -> Pki {
    let ca_key = KeyPair::generate().expect("CA key");
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).expect("CA params");
    ca_params.distinguished_name.push(DnType::CommonName, "schematic-runtime test CA");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    short_validity(&mut ca_params);
    let ca = ca_params.self_signed(&ca_key).expect("CA certificate");

    let leaf_key = KeyPair::generate().expect("leaf key");
    let leaf = leaf_params().signed_by(&leaf_key, &ca, &ca_key).expect("leaf certificate");
    Pki {
        ca_pem: ca.pem(),
        ca_der: ca.der().to_vec(),
        leaf: leaf.der().clone(),
        leaf_key: leaf_key.serialize_der(),
    }
}

fn self_signed_leaf() -> (CertificateDer<'static>, Vec<u8>) {
    let key = KeyPair::generate().expect("leaf key");
    let leaf = leaf_params().self_signed(&key).expect("self-signed leaf");
    (leaf.der().clone(), key.serialize_der())
}

async fn read_tls_head<S: AsyncRead + Unpin>(stream: &mut S) -> Vec<u8> {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte).await {
            Ok(0) | Err(_) => break,
            Ok(_) => head.push(byte[0]),
        }
    }
    head
}

/// An HTTPS mock presenting `leaf` that answers each connection with one
/// `Item`; returns its `https://127.0.0.1:<port>` base URL.
async fn tls_server(leaf: CertificateDer<'static>, key: Vec<u8>) -> (String, ServerTask) {
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_no_client_auth()
        .with_single_cert(vec![leaf], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)))
        .expect("server certificate");
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind loopback");
    let port = listener.local_addr().expect("local address").port();
    let task = tokio::spawn(async move {
        let mut connections = JoinSet::new();
        while let Ok((stream, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            connections.spawn(async move {
                // A client that rejects the certificate fails the handshake here.
                let Ok(mut stream) = acceptor.accept(stream).await else { return };
                if read_tls_head(&mut stream).await.is_empty() {
                    return;
                }
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 8\r\nConnection: close\r\n\r\n{\"id\":1}")
                    .await;
                let _ = stream.shutdown().await;
            });
        }
    });
    (format!("https://127.0.0.1:{port}"), ServerTask(task))
}

async fn send_item(client: &Client<TestApi>) -> Result<u16, TransportErrorKind> {
    client
        .send(Call::<Json<Item>>::get("/item"), &CallOptions::new())
        .await
        .map(|response| response.meta.status.as_u16())
        .map_err(|error| error.kind)
}

#[cfg_attr(
    windows,
    ignore = "native Windows: aws-lc's CPU-jitter entropy setup aborts the process (0xc0000409) under parallel load; tracked by fix 2026-10-12-windows-aws-lc-entropy-abort"
)]
#[tokio::test]
async fn an_added_pem_root_lets_the_client_reach_a_server_it_signed() {
    let pki = ca_signed_pki();
    let (url, _server) = tls_server(pki.leaf.clone(), pki.leaf_key.clone()).await;

    let client = builder(definition(), &url).add_root_certificate(pki.ca_pem.into_bytes()).build().expect("client");
    assert_eq!(send_item(&client).await, Ok(200));

    let transport = SharedTransport::builder().add_root_certificate(pki.ca_der.clone()).build().expect("transport");
    let attached = builder(definition(), &url).transport(&transport).build().expect("client");
    assert_eq!(send_item(&attached).await, Ok(200));
}

#[cfg_attr(
    windows,
    ignore = "native Windows: aws-lc's CPU-jitter entropy setup aborts the process (0xc0000409) under parallel load; tracked by fix 2026-10-12-windows-aws-lc-entropy-abort"
)]
#[tokio::test]
async fn an_added_der_root_lets_the_client_reach_a_server_it_signed() {
    let pki = ca_signed_pki();
    let (url, _server) = tls_server(pki.leaf, pki.leaf_key).await;

    let client = builder(definition(), &url).add_root_certificate(pki.ca_der).build().expect("client");

    assert_eq!(send_item(&client).await, Ok(200));
}

#[cfg_attr(
    windows,
    ignore = "native Windows: aws-lc's CPU-jitter entropy setup aborts the process (0xc0000409) under parallel load; tracked by fix 2026-10-12-windows-aws-lc-entropy-abort"
)]
#[tokio::test]
async fn without_the_added_root_the_handshake_fails_as_a_tls_error() {
    let pki = ca_signed_pki();
    let (url, _server) = tls_server(pki.leaf, pki.leaf_key).await;

    let client = plain_client(&url);

    assert_eq!(send_item(&client).await, Err(TransportErrorKind::Tls));
}

#[cfg_attr(
    windows,
    ignore = "native Windows: aws-lc's CPU-jitter entropy setup aborts the process (0xc0000409) under parallel load; tracked by fix 2026-10-12-windows-aws-lc-entropy-abort"
)]
#[tokio::test]
async fn accepting_invalid_certificates_reaches_a_self_signed_server_that_the_default_rejects() {
    let (leaf, key) = self_signed_leaf();
    let (url, _server) = tls_server(leaf, key).await;

    let default = plain_client(&url);
    assert_eq!(send_item(&default).await, Err(TransportErrorKind::Tls));

    let accepting = builder(definition(), &url).danger_accept_invalid_certificates().build().expect("client");
    assert_eq!(send_item(&accepting).await, Ok(200));

    let transport = SharedTransport::builder().danger_accept_invalid_certificates().build().expect("transport");
    let attached = builder(definition(), &url).transport(&transport).build().expect("client");
    assert_eq!(send_item(&attached).await, Ok(200));
}
