//! One test (or a few) per fixture listed in the spike brief. Local mocks only.

use std::time::Duration;

use futures_util::StreamExt;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use typed_send_spike::api::*;
use typed_send_spike::client::{AuthPlacement, CallOptions, DemoClient};
use typed_send_spike::runtime::*;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const SENTINEL: &str = "SENTINEL-sk-7f3a9e";

fn client(server: &MockServer) -> DemoClient {
    DemoClient::builder(server.uri()).build()
}

// --- plain JSON success + full metadata -------------------------------------

#[tokio::test]
async fn json_success_with_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "req-123")
                .insert_header("x-ratelimit-remaining", "59")
                .set_body_json(json!({"id": 1, "full_name": "o/r", "default_branch": "main"})),
        )
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri())
        .request_id_header("x-request-id")
        .build();

    let resp = c.send(GetRepoRequest::new("o", "r")).await.expect("transport ok");

    assert_eq!(resp.data().map(|r| r.default_branch.as_str()), Some("main"));
    assert!(resp.error().is_none());
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.meta.request.method, "GET");
    assert_eq!(resp.meta.request.url, format!("{}/repos/o/r", server.uri()));
    assert_eq!(resp.meta.request.header("user-agent"), Some("typed-send-spike"));
    assert_eq!(resp.header("x-ratelimit-remaining"), Some("59"));
    assert_eq!(resp.meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(resp.meta.request.page, None);
    assert_eq!(resp.meta.request_id.as_deref(), Some("req-123"));
    assert!(resp.meta.elapsed > Duration::ZERO);
    assert!(resp.into_result().is_ok());
}

#[tokio::test]
async fn request_id_absent_when_not_named() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "req-123")
                .set_body_json(json!({"id": 1, "full_name": "o/r", "default_branch": "main"})),
        )
        .mount(&server)
        .await;
    let resp = client(&server).send(GetRepoRequest::new("o", "r")).await.unwrap();
    assert_eq!(resp.meta.request_id, None);
}

// --- text / bytes / empty ---------------------------------------------------

#[tokio::test]
async fn text_success() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r/readme"))
        .and(header("accept", "application/vnd.github.raw"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# Hello"))
        .mount(&server)
        .await;
    let resp = client(&server).send(GetReadmeRawRequest::new("o", "r")).await.unwrap();
    assert_eq!(resp.data().map(String::as_str), Some("# Hello"));
}

#[tokio::test]
async fn bytes_success() {
    let server = MockServer::start().await;
    let payload = vec![0u8, 159, 146, 150, 255];
    Mock::given(path("/repos/o/r/tarball"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.clone()))
        .mount(&server)
        .await;
    let req = DownloadArchiveRequest {
        owner: "o".into(),
        repo: "r".into(),
    };
    let resp = client(&server).send(req).await.unwrap();
    assert_eq!(resp.data().map(|b| b.to_vec()), Some(payload));
}

#[tokio::test]
async fn empty_success_204() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/repos/o/r/git/refs/heads/feat"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let req = DeleteBranchRequest {
        owner: "o".into(),
        repo: "r".into(),
        branch: "feat".into(),
    };
    let resp = client(&server).send(req).await.unwrap();
    assert_eq!(resp.status(), 204);
    assert_eq!(resp.data(), Some(&()));
}

// --- multi-shape success ----------------------------------------------------

#[tokio::test]
async fn multi_shape_success_enum() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/computing/stats/commit_activity"))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;
    Mock::given(path("/repos/o/ready/stats/commit_activity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"week": 1, "total": 3}])))
        .mount(&server)
        .await;
    let c = client(&server);
    let computing = c
        .send(GetCommitActivityRequest {
            owner: "o".into(),
            repo: "computing".into(),
        })
        .await
        .unwrap();
    assert_eq!(computing.data(), Some(&CommitActivitySuccess::Computing));
    let ready = c
        .send(GetCommitActivityRequest {
            owner: "o".into(),
            repo: "ready".into(),
        })
        .await
        .unwrap();
    assert_eq!(
        ready.data(),
        Some(&CommitActivitySuccess::Ready(vec![WeekActivity { week: 1, total: 3 }]))
    );
}

// --- streaming --------------------------------------------------------------

/// A raw HTTP/1.1 server that sends chunks one at a time, waiting on a gate
/// before each chunk after the first. The client releasing the gate only
/// after it has decoded item 1 proves item 1 arrived before the rest was sent.
async fn gated_stream_server(
    content_type: &'static str,
    chunks: Vec<&'static str>,
    terminate: bool,
) -> (String, tokio::sync::mpsc::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (gate_tx, mut gate_rx) = tokio::sync::mpsc::channel::<()>(16);
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let mut seen = Vec::new();
        // Read headers + body (Content-Length) of the request.
        loop {
            let n = sock.read(&mut buf).await.unwrap();
            seen.extend_from_slice(&buf[..n]);
            if let Some(pos) = seen.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&seen[..pos]).to_ascii_lowercase();
                let len = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .map(|v| v.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                if seen.len() >= pos + 4 + len {
                    break;
                }
            }
        }
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ntransfer-encoding: chunked\r\n\r\n"
        );
        sock.write_all(head.as_bytes()).await.unwrap();
        for (i, chunk) in chunks.iter().enumerate() {
            if i > 0 && gate_rx.recv().await.is_none() {
                return;
            }
            let frame = format!("{:x}\r\n{}\r\n", chunk.len(), chunk);
            sock.write_all(frame.as_bytes()).await.unwrap();
            sock.flush().await.unwrap();
        }
        if terminate {
            sock.write_all(b"0\r\n\r\n").await.unwrap();
        }
        // Dropping `sock` without the terminator = premature close.
    });
    (format!("http://{addr}"), gate_tx)
}

#[tokio::test]
async fn sse_stream_items_arrive_incrementally() {
    let (base, gate) = gated_stream_server(
        "text/event-stream",
        vec![
            "data: {\"delta\":\"Hel\"}\n\n",
            "data: {\"delta\":\"lo\"}\n\n",
            "data: [DONE]\n\n",
        ],
        true,
    )
    .await;
    let c = DemoClient::builder(base).build();
    let resp = c
        .send(StreamChatRequest {
            prompt: "hi".into(),
        })
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.header("content-type"), Some("text/event-stream"));
    let mut stream = resp.into_result().expect("framing ok");
    // Server has sent ONLY chunk 1 at this point.
    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.delta, "Hel");
    gate.send(()).await.unwrap();
    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(second.delta, "lo");
    gate.send(()).await.unwrap();
    assert!(stream.next().await.is_none(), "[DONE] ends the stream");
}

#[tokio::test]
async fn sse_partial_event_is_premature_close() {
    let (base, gate) = gated_stream_server(
        "text/event-stream",
        vec!["data: {\"delta\":\"a\"}\n\n", "data: {\"delta\":\"b\"}"],
        true,
    )
    .await;
    let c = DemoClient::builder(base).build();
    let mut stream = c
        .send(StreamChatRequest::default())
        .await
        .unwrap()
        .into_result()
        .unwrap();
    assert!(stream.next().await.unwrap().is_ok());
    gate.send(()).await.unwrap();
    assert_eq!(stream.next().await.unwrap(), Err(StreamError::PrematureClose));
}

#[tokio::test]
async fn sse_dropped_socket_is_transport_stream_error() {
    let (base, gate) = gated_stream_server(
        "text/event-stream",
        vec!["data: {\"delta\":\"a\"}\n\n", "data: {\"delta\":\"b\"}\n\n"],
        false,
    )
    .await;
    let c = DemoClient::builder(base).build();
    let mut stream = c
        .send(StreamChatRequest::default())
        .await
        .unwrap()
        .into_result()
        .unwrap();
    assert!(stream.next().await.unwrap().is_ok());
    gate.send(()).await.unwrap();
    assert!(stream.next().await.unwrap().is_ok());
    drop(gate);
    assert!(matches!(stream.next().await, Some(Err(StreamError::Transport(_)))));
}

#[tokio::test]
async fn sse_wrong_content_type_keeps_status_headers_and_body() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw("{\"delta\":\"not a stream\"}", "application/json"),
        )
        .mount(&server)
        .await;
    let resp = client(&server).send(StreamChatRequest::default()).await.unwrap();
    let err = resp.error().expect("failure side");
    assert!(matches!(
        &err.kind,
        ErrorKind::UnexpectedContentType { expected: "text/event-stream", actual: Some(a) } if a == "application/json"
    ));
    assert_eq!(err.body_text(), "{\"delta\":\"not a stream\"}");
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn ndjson_stream_with_malformed_line() {
    let (base, gate) = gated_stream_server(
        "application/x-ndjson",
        vec![
            "{\"status\":\"pulling\"}\n",
            "not json\n{\"status\":\"verifying\",\"completed\":10}\n",
            "{\"status\":\"success\"}",
        ],
        true,
    )
    .await;
    let c = DemoClient::builder(base).build();
    let mut stream = c
        .send(PullModelRequest {
            model: "llama".into(),
        })
        .await
        .unwrap()
        .into_result()
        .unwrap();
    assert_eq!(stream.next().await.unwrap().unwrap().status, "pulling");
    gate.send(()).await.unwrap();
    assert!(matches!(
        stream.next().await.unwrap(),
        Err(StreamError::MalformedItem { raw, .. }) if raw == "not json"
    ));
    assert_eq!(stream.next().await.unwrap().unwrap().completed, Some(10));
    gate.send(()).await.unwrap();
    // Final line without '\n' that parses is accepted.
    assert_eq!(stream.next().await.unwrap().unwrap().status, "success");
    assert!(stream.next().await.is_none());
}

// --- GraphQL ----------------------------------------------------------------

#[tokio::test]
async fn graphql_typed_variables_and_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_json(json!({
            "query": ViewerRepos::DOCUMENT,
            "variables": {"login": "ken", "first": 5}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"user": {"login": "ken", "repositories": {"totalCount": 42}}}
        })))
        .mount(&server)
        .await;
    let q = GraphQlRequest::<ViewerRepos>::new(ViewerReposVars {
        login: "ken".into(),
        first: 5,
    });
    let data = client(&server).send(q).await.unwrap().into_result().unwrap();
    assert_eq!(data.user.unwrap().repositories.total_count, 42);
}

#[tokio::test]
async fn graphql_200_with_errors_is_failure_side_with_partial_data() {
    let server = MockServer::start().await;
    let body = json!({
        "data": {"user": null},
        "errors": [{"message": "Could not resolve to a User with the login of 'nobody'.",
                    "path": ["user"], "extensions": {"type": "NOT_FOUND"}}]
    });
    Mock::given(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
        .mount(&server)
        .await;
    let q = GraphQlRequest::<ViewerRepos>::new(ViewerReposVars {
        login: "nobody".into(),
        first: 1,
    });
    let resp = client(&server).send(q).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.data().is_none(), "errors present => not a success");
    let err = resp.error().unwrap();
    let gql = err.declared().expect("GraphQL errors are the declared failure");
    assert_eq!(gql.errors[0].extensions, Some(json!({"type": "NOT_FOUND"})));
    assert_eq!(gql.partial_data, Some(ViewerReposData { user: None }));
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&err.body).unwrap(), body);
}

#[tokio::test]
async fn graphql_invalid_document_is_sent_unchanged() {
    // D17: no parsing. A bad document reaches the server; its error comes back.
    struct Broken;
    impl GraphQlOperation for Broken {
        const ID: &'static str = "Broken";
        const DOCUMENT: &'static str = "query { this is not graphql";
        type Variables = serde_json::Value;
        type Data = serde_json::Value;
    }
    let server = MockServer::start().await;
    Mock::given(body_json(json!({"query": "query { this is not graphql", "variables": {}})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"errors": [{"message": "Parse error on \"is\""}]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let resp = client(&server)
        .send(GraphQlRequest::<Broken>::new(json!({})))
        .await
        .unwrap();
    assert!(resp.error().and_then(|e| e.declared()).is_some());
}

// --- error side -------------------------------------------------------------

#[tokio::test]
async fn declared_error_404_typed_with_headers_and_raw_body() {
    let server = MockServer::start().await;
    let body = json!({"message": "Not Found", "documentation_url": "https://docs"});
    Mock::given(path("/repos/o/missing"))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("x-github-request-id", "abc")
                .set_body_json(body.clone()),
        )
        .mount(&server)
        .await;
    let resp = client(&server).send(GetRepoRequest::new("o", "missing")).await.unwrap();
    assert!(resp.data().is_none());
    assert_eq!(resp.status(), 404);
    assert_eq!(resp.header("x-github-request-id"), Some("abc"));
    let err = resp.error().unwrap();
    assert_eq!(err.declared().map(|e| e.message.as_str()), Some("Not Found"));
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&err.body).unwrap(), body);
    let failure = resp.into_result().unwrap_err();
    assert_eq!(failure.status(), 404);
}

#[tokio::test]
async fn undeclared_error_status_keeps_raw_body() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(500).set_body_string("{\"message\":\"boom\"}"))
        .mount(&server)
        .await;
    let req = DeleteBranchRequest {
        owner: "o".into(),
        repo: "r".into(),
        branch: "b".into(),
    };
    let resp = client(&server).send(req).await.unwrap();
    let err = resp.error().unwrap();
    // 500 is outside DeleteBranch's declared (404, 422), so it is NOT decoded.
    assert!(matches!(err.kind, ErrorKind::Undeclared));
    assert_eq!(err.body_text(), "{\"message\":\"boom\"}");
    assert_eq!(resp.status(), 500);
}

#[tokio::test]
async fn endpoint_without_error_type_reports_undeclared() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r/readme"))
        .respond_with(ResponseTemplate::new(403).set_body_string("nope"))
        .mount(&server)
        .await;
    let resp = client(&server).send(GetReadmeRawRequest::new("o", "r")).await.unwrap();
    let err: &ApiError<NoDeclaredError> = resp.error().unwrap();
    assert!(matches!(err.kind, ErrorKind::Undeclared));
    assert_eq!(err.body_text(), "nope");
}

#[tokio::test]
async fn malformed_error_body_survives_raw() {
    let server = MockServer::start().await;
    let html = "<html><body>502 Bad Gateway</body></html>";
    Mock::given(path("/repos/o/r"))
        .respond_with(
            ResponseTemplate::new(502).set_body_raw(html, "text/html"),
        )
        .mount(&server)
        .await;
    let resp = client(&server).send(GetRepoRequest::new("o", "r")).await.unwrap();
    let err = resp.error().unwrap();
    assert!(matches!(err.kind, ErrorKind::MalformedError(_)));
    assert_eq!(err.body_text(), html);
    assert_eq!(resp.header("content-type"), Some("text/html"));
}

#[tokio::test]
async fn malformed_success_body_is_failure_with_raw_body() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"id\": \"not-a-number\"}"))
        .mount(&server)
        .await;
    let resp = client(&server).send(GetRepoRequest::new("o", "r")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let err = resp.error().unwrap();
    assert!(matches!(err.kind, ErrorKind::MalformedSuccess(_)));
    assert_eq!(err.body_text(), "{\"id\": \"not-a-number\"}");
}

// --- transport --------------------------------------------------------------

async fn refused_base() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{addr}")
}

#[tokio::test]
async fn connection_refused_carries_request_metadata() {
    let base = refused_base().await;
    let c = DemoClient::builder(base.clone())
        .credential(Secret::new(SENTINEL))
        .build();
    let err = c
        .send(ListPullsRequest::new("o", "r").with_page(3))
        .await
        .expect_err("nothing listens");
    assert!(matches!(err.kind, TransportErrorKind::Connect(_)), "{:?}", err.kind);
    assert_eq!(err.request.method, "GET");
    assert_eq!(err.request.url, format!("{base}/repos/o/r/pulls?page=3"));
    assert_eq!(err.request.query, vec![("page".to_string(), "3".to_string())]);
    assert_eq!(err.request.header("authorization"), Some(REDACTED));
    assert_eq!(err.request.auth, AuthMethod::ClientLevel);
    assert!(err.elapsed > Duration::ZERO);
    assert!(!format!("{err:?}{err}").contains(SENTINEL));
}

#[tokio::test]
async fn timeout_is_transport_error() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri())
        .timeout(Duration::from_millis(100))
        .build();
    let err = c.send(GetRepoRequest::new("o", "r")).await.unwrap_err();
    assert_eq!(err.kind, TransportErrorKind::Timeout);
    assert_eq!(err.request.endpoint_id, "GetRepo");
}

#[tokio::test]
async fn redirect_is_denied_and_target_sees_nothing() {
    let target = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&target)
        .await;
    let first = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", format!("{}/elsewhere", target.uri())),
        )
        .mount(&first)
        .await;
    let c = DemoClient::builder(first.uri())
        .credential(Secret::new(SENTINEL))
        .build();
    let err = c.send(GetRepoRequest::new("o", "r")).await.unwrap_err();
    assert!(matches!(err.kind, TransportErrorKind::RedirectDenied { status: 302, .. }));
    assert_eq!(err.request.auth, AuthMethod::ClientLevel);
}

#[tokio::test]
async fn auth_required_without_credential_sends_nothing() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri())
        .env_vars(&["SPIKE_UNSET_VAR_A1"])
        .auth_required(true)
        .build();
    let err = c.send(GetRepoRequest::new("o", "r")).await.unwrap_err();
    assert!(matches!(err.kind, TransportErrorKind::AuthenticationRequired { .. }));
    assert_eq!(err.request.auth, AuthMethod::Anonymous);
}

// --- redaction --------------------------------------------------------------

fn all_text<T: std::fmt::Debug, E: std::fmt::Debug>(r: &Response<T, E>) -> String {
    let m = &r.meta;
    format!(
        "{r:?}|{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        m.request.url, m.request.headers, m.request.query, m.headers, m.request.auth, m.request_id
    )
}

#[tokio::test]
async fn sentinel_never_appears_in_any_output() {
    let server = MockServer::start().await;
    // Success with a Set-Cookie carrying the sentinel.
    Mock::given(path("/repos/o/ok"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", format!("session={SENTINEL}"))
                .set_body_json(json!({"id": 1, "full_name": "o/ok", "default_branch": "main"})),
        )
        .mount(&server)
        .await;
    Mock::given(path("/repos/o/bad"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({"message": "Bad credentials"})),
        )
        .mount(&server)
        .await;
    // Error whose body ECHOES the sentinel (a provider may do this).
    Mock::given(path("/repos/o/echo"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({"message": format!("bad credentials {SENTINEL}")})),
        )
        .mount(&server)
        .await;

    let bearer = DemoClient::builder(server.uri())
        .credential(Secret::new(SENTINEL))
        .secret_header("x-custom-secret")
        .build();
    let ok = bearer.send(GetRepoRequest::new("o", "ok")).await.unwrap();
    assert!(!all_text(&ok).contains(SENTINEL), "{}", all_text(&ok));
    assert_eq!(ok.header("set-cookie"), Some(REDACTED));
    assert_eq!(ok.meta.request.header("authorization"), Some(REDACTED));
    // The token really was sent: it is just not stored.
    let received = server.received_requests().await.unwrap();
    assert_eq!(
        received[0].headers.get("authorization").unwrap(),
        &format!("Bearer {SENTINEL}")
    );

    let per_call = || CallOptions {
        credential: Some(Secret::new(SENTINEL)),
        ..CallOptions::default()
    };
    let bad = bearer.send_with(GetRepoRequest::new("o", "bad"), per_call()).await.unwrap();
    assert!(!all_text(&bad).contains(SENTINEL));
    let failure = bad.into_result().unwrap_err();
    assert!(!format!("{failure} {failure:?}").contains(SENTINEL));
    let as_call: CallError<GhError> = failure.into();
    assert!(!format!("{as_call} {as_call:?}").contains(SENTINEL));

    // LIMIT (recorded in findings): server-echoed body content is evidence,
    // not request metadata, and cannot be redacted without knowing which
    // body bytes are secret. Metadata and Display stay clean; the raw body
    // and the decoded `E` (and therefore Debug) carry the echo.
    let echo = bearer.send_with(GetRepoRequest::new("o", "echo"), per_call()).await.unwrap();
    let m = &echo.meta;
    let meta_text = format!("{m:?}");
    assert!(!meta_text.contains(SENTINEL));
    let failure = echo.into_result().unwrap_err();
    assert!(!failure.to_string().contains(SENTINEL));
    assert!(failure.error.body_text().contains(SENTINEL));
    assert!(failure.declared().unwrap().message.contains(SENTINEL));
    assert!(format!("{failure:?}").contains(SENTINEL), "Debug prints the decoded E");
}

#[tokio::test]
async fn query_param_credential_redacted_in_url_and_query() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r/pulls"))
        .and(query_param("api_key", SENTINEL))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri())
        .placement(AuthPlacement::QueryParam("api_key"))
        .credential(Secret::new(SENTINEL))
        .build();
    let resp = c.send(ListPullsRequest::new("o", "r").with_page(1)).await.unwrap();
    assert!(resp.is_success(), "server saw the key in the query");
    assert_eq!(
        resp.meta.request.url,
        format!("{}/repos/o/r/pulls?page=1&api_key=[redacted]", server.uri())
    );
    assert!(resp.meta.request.query.contains(&("api_key".into(), REDACTED.into())));
    assert!(!all_text(&resp).contains(SENTINEL));

    // Same client, nothing listening: transport Display must not leak the URL.
    let dead = DemoClient::builder(refused_base().await)
        .placement(AuthPlacement::QueryParam("api_key"))
        .credential(Secret::new(SENTINEL))
        .build();
    let err = dead.send(ListPullsRequest::new("o", "r")).await.unwrap_err();
    let text = format!("{err} {err:?}");
    assert!(!text.contains(SENTINEL), "{text}");
}

// --- credential reporting (D11) ---------------------------------------------

#[tokio::test]
async fn auth_method_per_call_client_env_anonymous() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"id": 1, "full_name": "o/r", "default_branch": "main"})),
        )
        .mount(&server)
        .await;
    // SAFETY (test): unique variable names; no other test reads them.
    unsafe {
        std::env::set_var("SPIKE_EMPTY_TOKEN_B2", "");
        std::env::set_var("SPIKE_TOKEN_B2", "env-token-value");
    }
    let env_client = DemoClient::builder(server.uri())
        .env_vars(&["SPIKE_UNSET_TOKEN_B2", "SPIKE_EMPTY_TOKEN_B2", "SPIKE_TOKEN_B2"])
        .build();
    let both = DemoClient::builder(server.uri())
        .env_vars(&["SPIKE_TOKEN_B2"])
        .credential(Secret::new("client-token"))
        .build();
    let anon = client(&server);

    let req = || GetRepoRequest::new("o", "r");
    let per_call = CallOptions {
        credential: Some(Secret::new("call-token")),
        ..CallOptions::default()
    };
    let r1 = both.send_with(req(), per_call).await.unwrap();
    let r2 = both.send(req()).await.unwrap();
    let r3 = env_client.send(req()).await.unwrap();
    let r4 = anon.send(req()).await.unwrap();
    assert_eq!(r1.meta.request.auth, AuthMethod::PerCall);
    assert_eq!(r2.meta.request.auth, AuthMethod::ClientLevel);
    assert_eq!(r3.meta.request.auth, AuthMethod::EnvVar("SPIKE_TOKEN_B2".into()));
    assert_eq!(r4.meta.request.auth, AuthMethod::Anonymous);

    let sent: Vec<Option<String>> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r: &Request| r.headers.get("authorization").map(|v| v.to_str().unwrap().to_string()))
        .collect();
    assert_eq!(
        sent,
        vec![
            Some("Bearer call-token".into()),
            Some("Bearer client-token".into()),
            Some("Bearer env-token-value".into()),
            None
        ]
    );
    for r in [&r1, &r2, &r3, &r4] {
        let t = all_text(r);
        assert!(!t.contains("call-token") && !t.contains("client-token") && !t.contains("env-token-value"));
    }
}

// --- hooks ------------------------------------------------------------------

#[tokio::test]
async fn pre_response_json_and_mutate_response_still_run() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"id": 1, "full_name": "o/r", "default_branch": "main"}
        })))
        .mount(&server)
        .await;
    Mock::given(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"data": {"user": {"login": "ken", "repositories": {"totalCount": 1}}}}
        })))
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri())
        // Unwrap a `{"data": ...}` envelope (the documented use case).
        .pre_response_json(|ctx, json| {
            assert!(!ctx.url.is_empty());
            Ok(json.get("data").cloned().unwrap_or(json))
        })
        .mutate_response::<GetRepoRequest>(|ctx, repo| {
            assert_eq!(ctx.endpoint_id, "GetRepo");
            repo.default_branch.push_str("-mutated");
            Ok(())
        })
        .mutate_response::<GraphQlRequest<ViewerRepos>>(|_ctx, data| {
            if let Some(u) = data.user.as_mut() {
                u.login = u.login.to_uppercase();
            }
            Ok(())
        })
        .build();
    let repo = c.send(GetRepoRequest::new("o", "r")).await.unwrap().into_result().unwrap();
    assert_eq!(repo.default_branch, "main-mutated");
    let gql = c
        .send(GraphQlRequest::<ViewerRepos>::new(ViewerReposVars {
            login: "ken".into(),
            first: 1,
        }))
        .await
        .unwrap()
        .into_result()
        .unwrap();
    assert_eq!(gql.user.unwrap().login, "KEN");
}

#[tokio::test]
async fn hook_error_is_failure_side_with_raw_body() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"id": 1, "full_name": "o/r", "default_branch": "main"})),
        )
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri())
        .mutate_response::<GetRepoRequest>(|_, _| Err("refused by hook".into()))
        .build();
    let resp = c.send(GetRepoRequest::new("o", "r")).await.unwrap();
    let err = resp.error().unwrap();
    assert!(matches!(&err.kind, ErrorKind::Hook(m) if m == "refused by hook"));
    assert!(err.body_text().contains("\"default_branch\""));
}
