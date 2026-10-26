//! The pager over the client's transport: link origin checks under each
//! cross-origin policy and across redirects (R18), `Link` header parsing
//! through a traversal, failed pages kept inside the envelope (R20), the
//! shared deadline and cancellation (R19), the privacy of pager error text
//! with a live credential (R36), and the response hooks on pages.
//!
//! Origins A and B are loopback servers that differ only in port, which
//! makes them distinct origins.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use schematic_runtime::pager::{ContinuationProblem, Incomplete, IncompleteReason, PagerError};
use schematic_runtime::{
    ApiDefinition, AuthConfig, Client, Credential, CredentialPlacement, Deadline, ErrorKind, FailureCategory,
    HookError, OriginPolicy, RedirectRefusal, TransportErrorKind,
};
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use crate::pager_support::{
    BodyNextPrevious, Item, LinkCursor, NextField, ProviderError, client, ids, json, numbered, on_query, otherwise,
    received,
};
use crate::support::{AbortOnDrop, SENTINEL, TestApi, builder, closed_port_url, listener, read_head, write_all};

type Error = PagerError<ProviderError>;
type Unfinished = Incomplete<Item, ProviderError>;

/// The credential the authenticated clients send.
const SECRET: &str = "bearer-5e1f0a";

/// A client for `base` that sends [`SECRET`] as a bearer token under the
/// API policy `policy`.
fn bearer_client(base: &str, policy: OriginPolicy, token: &str) -> Client<TestApi> {
    let definition = ApiDefinition::new("https://api.example.invalid")
        .auth(AuthConfig::none().accept(CredentialPlacement::Bearer { header: None }))
        .cross_origin(policy);
    builder(definition, base).credentials(Credential::access_token(token)).build().expect("client builds")
}

fn authorization(request: &Request) -> Option<String> {
    request.headers.get("authorization").map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
}

/// Serves page one of [`LinkCursor`] (item 1) with `link` as its `Link`
/// header on every request without `cursor=2`.
async fn first_page_links_to(server: &MockServer, link: &str) {
    otherwise(server, json(numbered(1..=1)).insert_header("link", link)).await;
}

/// Page one, then the refused page two, of a [`LinkCursor`] traversal.
async fn second_page_error(client: &Client<TestApi>) -> Error {
    let mut pages = client.send(LinkCursor::get()).pager().pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    assert_eq!(ids(&first.items), [1]);
    pages.next().await.expect_err("page 2 fails")
}

fn assert_cross_origin_refusal(error: &Error) {
    let PagerError::Transport(transport) = error else { panic!("{error:?}") };
    assert!(matches!(transport.kind, TransportErrorKind::CrossOriginRejected { .. }), "{transport:?}");
    assert_eq!(transport.request.page, Some(2));
}

// ---------------------------------------------------------------------------
// Link origins (R18)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_link_to_a_second_origin_follows_the_policy_and_its_credential_rule() {
    // (policy, followed, credential at B), per the table in `origin.rs`.
    let cases = [
        ("same_origin", false, None),
        ("allow_origins", true, Some(true)),
        ("allow_without_credentials", true, Some(false)),
    ];
    for (label, followed, credential_at_b) in cases {
        let (a, b) = (MockServer::start().await, MockServer::start().await);
        let policy = match label {
            "same_origin" => OriginPolicy::same_origin(),
            "allow_origins" => OriginPolicy::allow_origins([b.uri()]),
            _ => OriginPolicy::allow_without_credentials(),
        };
        first_page_links_to(&a, &format!(r#"<{}/link-cursor?cursor=2>; rel="next""#, b.uri())).await;
        otherwise(&b, json(numbered(2..=2))).await;
        let client = bearer_client(&a.uri(), policy, SECRET);

        let result = client.send(LinkCursor::get()).pager().collect_all().await;

        let bearer = format!("Bearer {SECRET}");
        assert_eq!(authorization(&received(&a).await[0]).as_deref(), Some(bearer.as_str()), "{label}");
        let at_b = received(&b).await;
        if followed {
            assert_eq!(ids(&result.expect("complete")), [1, 2], "{label}");
            assert_eq!(at_b.len(), 1, "{label}");
            let expected = credential_at_b.expect("followed").then_some(bearer.as_str());
            assert_eq!(authorization(&at_b[0]).as_deref(), expected, "{label}");
            assert!(!at_b[0].url.as_str().contains(SECRET), "{label}");
        } else {
            let incomplete = result.expect_err("refused");
            assert_eq!(incomplete.reason, IncompleteReason::CrossOriginRejected, "{label}");
            assert_eq!(ids(&incomplete.items), [1], "{label}");
            assert_cross_origin_refusal(incomplete.error.as_ref().expect("cause"));
            assert!(at_b.is_empty(), "{label}: no request, so no credential, reached B");
        }
    }
}

#[tokio::test]
async fn under_the_default_policy_every_other_scheme_host_port_and_userinfo_link_is_refused_unsent() {
    let other_port = MockServer::start().await;
    otherwise(&other_port, json(numbered(2..=2))).await;
    let b_port = other_port.address().port();
    for target in [
        "https://127.0.0.1:{A}/link-cursor?cursor=2".to_owned(),
        format!("http://127.0.0.1:{b_port}/link-cursor?cursor=2"),
        "http://localhost:{A}/link-cursor?cursor=2".to_owned(),
        "http://user:pass@127.0.0.1:{A}/link-cursor?cursor=2".to_owned(),
        "//other.invalid/link-cursor?cursor=2".to_owned(),
    ] {
        let a = MockServer::start().await;
        let target = target.replace("{A}", &a.address().port().to_string());
        first_page_links_to(&a, &format!(r#"<{target}>; rel="next""#)).await;
        let client = client(&a);

        let error = second_page_error(&client).await;

        // A send attempt would have failed as Connect or Tls instead.
        assert_cross_origin_refusal(&error);
        assert_eq!(received(&a).await.len(), 1, "{target}");
    }
    assert!(received(&other_port).await.is_empty());
}

#[tokio::test]
async fn a_relative_link_resolves_against_the_url_page_one_was_redirected_to() {
    for (link, followed) in [("?page=2", "/v3/repos/list"), ("../next", "/v3/next")] {
        let server = MockServer::start().await;
        Mock::given(path("/v2/repos/list"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/v3/repos/list"))
            .mount(&server)
            .await;
        let last = json(json!({ "values": numbered(2..=2) }));
        Mock::given(path("/v3/repos/list"))
            .and(query_param("page", "2"))
            .respond_with(last.clone())
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(path("/v3/next")).respond_with(last).mount(&server).await;
        Mock::given(path("/v3/repos/list"))
            .respond_with(json(json!({ "values": numbered(1..=1), "next": link })))
            .with_priority(5)
            .mount(&server)
            .await;
        let client = client(&server);

        let items = client.send(BodyNextPrevious::get()).pager().collect_all().await.expect("complete");

        assert_eq!(ids(&items), [1, 2], "{link}");
        let paths: Vec<String> = received(&server).await.iter().map(|request| request.url.path().to_owned()).collect();
        assert_eq!(paths, ["/v2/repos/list", "/v3/repos/list", followed], "{link}");
    }
}

#[tokio::test]
async fn a_followed_link_that_redirects_to_another_origin_obeys_the_redirect_policy_and_credential_rule() {
    for policy in [OriginPolicy::same_origin(), OriginPolicy::allow_without_credentials()] {
        let label = policy.name();
        let (a, b) = (MockServer::start().await, MockServer::start().await);
        first_page_links_to(&a, r#"</hop?cursor=2>; rel="next""#).await;
        Mock::given(path("/hop"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", format!("{}/landing", b.uri())))
            .with_priority(1)
            .mount(&a)
            .await;
        otherwise(&b, json(numbered(2..=2))).await;
        let client = bearer_client(&a.uri(), policy.clone(), SECRET);

        let result = client.send(LinkCursor::get()).pager().collect_all().await;

        let hop = received(&a).await.into_iter().find(|request| request.url.path() == "/hop").expect("the link was sent");
        assert_eq!(authorization(&hop), Some(format!("Bearer {SECRET}")), "{label}: the link is same-origin");
        let at_b = received(&b).await;
        if policy == OriginPolicy::same_origin() {
            let incomplete = result.expect_err("redirect refused");
            assert_eq!(incomplete.reason, IncompleteReason::HttpError, "{label}");
            let Some(PagerError::Api { error, meta }) = &incomplete.error else { panic!("{incomplete:?}") };
            assert_eq!(error.kind, ErrorKind::RedirectRefused { reason: RedirectRefusal::CrossOrigin });
            assert_eq!((meta.status.as_u16(), meta.request.page), (302, Some(2)));
            assert!(at_b.is_empty(), "{label}");
        } else {
            assert_eq!(ids(&result.expect("complete")), [1, 2], "{label}");
            assert_eq!(at_b.len(), 1, "{label}");
            assert_eq!(authorization(&at_b[0]), None, "{label}: the credential is stripped on the cross-origin hop");
        }
    }
}

// ---------------------------------------------------------------------------
// `Link` header parsing through a traversal (R18)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_next_link_is_found_across_fields_quoted_commas_and_multiple_relations() {
    let cases: [&[&str]; 3] = [
        &[r#"</link-cursor?cursor=9>; rel="last""#, r#"</link-cursor?cursor=2>; rel="next""#],
        &[r#"</link-cursor?cursor=2>; title="a, b"; rel="next", </link-cursor?cursor=9>; rel="last""#],
        &[r#"</link-cursor?cursor=2>; rel="last next""#],
    ];
    for fields in cases {
        let server = MockServer::start().await;
        let first = fields.iter().fold(json(numbered(1..=1)), |response, field| response.append_header("link", *field));
        otherwise(&server, first).await;
        on_query(&server, "cursor", "2", json(numbered(2..=2))).await;
        let client = client(&server);

        let items = client.send(LinkCursor::get()).pager().collect_all().await.expect("complete");

        assert_eq!(ids(&items), [1, 2], "{fields:?}");
        let requests = received(&server).await;
        assert_eq!(requests.len(), 2, "{fields:?}");
        assert_eq!(requests[1].url.query(), Some("cursor=2"), "{fields:?}");
    }
}

#[tokio::test]
async fn a_malformed_link_header_is_an_invalid_link_after_its_page_is_delivered() {
    let server = MockServer::start().await;
    first_page_links_to(&server, r#"</link-cursor?cursor=2; rel="next""#).await;
    let client = client(&server);

    let incomplete = client.send(LinkCursor::get()).pager().collect_all().await.expect_err("malformed");

    assert_eq!(incomplete.reason, IncompleteReason::InvalidContinuation);
    assert_eq!(ids(&incomplete.items), [1]);
    assert!(
        matches!(incomplete.error, Some(PagerError::InvalidContinuation { problem: ContinuationProblem::Link, .. })),
        "{incomplete:?}"
    );
    assert_eq!(received(&server).await.len(), 1);
}

#[tokio::test]
async fn a_link_header_without_a_next_relation_ends_the_list() {
    let server = MockServer::start().await;
    first_page_links_to(&server, r#"</link-cursor?cursor=0>; rel="prev", </link-cursor?cursor=0>; rel="first""#).await;
    let client = client(&server);

    let items = client.send(LinkCursor::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1]);
    assert_eq!(received(&server).await.len(), 1);
}

// ---------------------------------------------------------------------------
// Failed pages keep the envelope (R20)
// ---------------------------------------------------------------------------

/// The error of page two from `pages()` and from `collect_all`.
async fn both_ways(client: &Client<TestApi>) -> (Error, Unfinished) {
    let error = second_page_error(client).await;
    let incomplete = client.send(LinkCursor::get()).pager().collect_all().await.expect_err("page 2 fails");
    assert_eq!(ids(&incomplete.items), [1]);
    assert_eq!(incomplete.pages, 2);
    (error, incomplete)
}

/// A server whose page one links to page two, answered by `second`.
async fn second_page_answers(second: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    first_page_links_to(&server, r#"</link-cursor?cursor=2>; rel="next""#).await;
    on_query(&server, "cursor", "2", second).await;
    server
}

async fn page_two_requests(server: &MockServer) -> usize {
    received(server).await.iter().filter(|request| request.url.query() == Some("cursor=2")).count()
}

#[tokio::test]
async fn pages_are_ordinary_sends_and_an_error_status_keeps_its_status_headers_and_body_without_a_retry() {
    let body = r#"{"message":"boom"}"#;
    let server = second_page_answers(
        ResponseTemplate::new(500).set_body_raw(body, "application/json").insert_header("x-trace", "t-1"),
    )
    .await;
    let client = client(&server);

    let mut pages = client.send(LinkCursor::get()).pager().pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    assert_eq!(first.meta.status, 200);
    assert_eq!(first.meta.request.page, Some(1));
    assert_eq!(first.meta.request.url.as_str(), format!("{}/link-cursor", server.uri()));
    assert!(first.meta.headers.contains_key("link"));
    assert!(first.meta.elapsed > Duration::ZERO);
    let error = pages.next().await.expect_err("page 2 fails");
    assert_eq!(page_two_requests(&server).await, 1, "no retry");

    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    assert_eq!(meta.status, 500);
    assert_eq!(meta.headers.get("x-trace").map(|value| value.as_bytes()), Some(&b"t-1"[..]));
    assert_eq!(meta.request.page, Some(2));
    assert_eq!(meta.request.url.as_str(), format!("{}/link-cursor?cursor=2", server.uri()));
    assert_eq!(api.body, Bytes::from_static(body.as_bytes()));
    assert_eq!(api.declared().map(|declared| declared.message.as_str()), Some("boom"));

    let (_, incomplete) = both_ways(&client).await;
    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
}

#[tokio::test]
async fn a_page_that_is_not_json_is_a_malformed_success_with_its_bytes() {
    let server = second_page_answers(ResponseTemplate::new(200).set_body_raw("<html>", "text/html")).await;
    let client = client(&server);

    let (error, incomplete) = both_ways(&client).await;

    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    assert_eq!(api.kind, ErrorKind::MalformedSuccess);
    assert_eq!(api.body, Bytes::from_static(b"<html>"));
    assert_eq!((meta.status.as_u16(), meta.request.page), (200, Some(2)));
    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
}

#[tokio::test]
async fn a_page_redirected_to_a_refused_origin_is_a_redirect_refusal_with_the_redirect_body() {
    let elsewhere = MockServer::start().await;
    let server = second_page_answers(
        ResponseTemplate::new(302)
            .insert_header("location", format!("{}/landing", elsewhere.uri()))
            .set_body_raw("moved", "text/plain"),
    )
    .await;
    let client = client(&server);

    let (error, incomplete) = both_ways(&client).await;

    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    assert_eq!(api.kind, ErrorKind::RedirectRefused { reason: RedirectRefusal::CrossOrigin });
    assert_eq!(api.body, Bytes::from_static(b"moved"));
    assert_eq!((meta.status.as_u16(), meta.request.page), (302, Some(2)));
    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert!(received(&elsewhere).await.is_empty());
}

#[tokio::test]
async fn a_page_whose_redirect_hop_fails_keeps_the_redirect_as_evidence() {
    let closed = closed_port_url().await;
    let server = second_page_answers(
        ResponseTemplate::new(302)
            .insert_header("location", format!("{closed}/landing"))
            .set_body_raw("moved", "text/plain"),
    )
    .await;
    // The hop must be allowed for it to fail rather than be refused.
    let client = bearer_client(&server.uri(), OriginPolicy::allow_without_credentials(), SECRET);

    let (error, incomplete) = both_ways(&client).await;

    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    let ErrorKind::RedirectFailed { category, .. } = &api.kind else { panic!("{api:?}") };
    assert_eq!(*category, FailureCategory::Connect);
    assert_eq!(api.body, Bytes::from_static(b"moved"));
    assert_eq!((meta.status.as_u16(), meta.request.page), (302, Some(2)));
    assert_eq!(incomplete.reason, IncompleteReason::TransportError);
}

// ---------------------------------------------------------------------------
// Raw-socket server for cut-off bodies, stalls, and cancellation
// ---------------------------------------------------------------------------

/// What the raw server does with one request.
enum Answer {
    /// Writes these bytes, then closes the connection.
    Reply(Vec<u8>),
    /// Never answers, and reports when the client closes the connection.
    Hang,
}

struct RawServer {
    url: String,
    /// Each request head, in arrival order.
    heads: mpsc::UnboundedReceiver<String>,
    /// One message per hung connection the client closed.
    closed: mpsc::UnboundedReceiver<()>,
    _accept: AbortOnDrop,
}

impl RawServer {
    /// Every head received so far, after a control request proves that
    /// anything sent earlier has arrived.
    async fn heads_through_control(&mut self) -> Vec<String> {
        client_for(&self.url).send(LinkCursor::get()).pager().fetch_page().await.expect("control page");
        let mut heads = Vec::new();
        while let Ok(head) = self.heads.try_recv() {
            heads.push(head);
        }
        assert!(heads.last().is_some_and(|head| is_page_one(head)), "{heads:?}");
        heads.pop();
        heads
    }
}

fn client_for(url: &str) -> Client<TestApi> {
    crate::support::plain_client(url)
}

async fn raw_server(answer: impl Fn(&str) -> Answer + Send + Sync + 'static) -> RawServer {
    let (listener, url) = listener().await;
    let (head_sent, heads) = mpsc::unbounded_channel();
    let (closed_sent, closed) = mpsc::unbounded_channel();
    let answer = Arc::new(answer);
    let task = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let (answer, head_sent, closed_sent) = (Arc::clone(&answer), head_sent.clone(), closed_sent.clone());
            tokio::spawn(async move {
                let head = read_head(&mut stream).await;
                let action = answer(&head);
                let _ = head_sent.send(head);
                match action {
                    Answer::Reply(bytes) => write_all(&mut stream, &bytes).await,
                    Answer::Hang => {
                        read_until_closed(&mut stream).await;
                        let _ = closed_sent.send(());
                    }
                }
            });
        }
    });
    RawServer {
        url,
        heads,
        closed,
        _accept: AbortOnDrop(task),
    }
}

async fn read_until_closed(stream: &mut TcpStream) {
    let mut buffer = [0_u8; 256];
    while matches!(stream.read(&mut buffer).await, Ok(read) if read > 0) {}
}

fn is_page_one(head: &str) -> bool {
    head.starts_with("GET /link-cursor HTTP/1.1")
}

fn is_page_two(head: &str) -> bool {
    head.starts_with("GET /link-cursor?cursor=2 HTTP/1.1")
}

/// Page one of [`LinkCursor`]: item 1, linking to `cursor=2`.
fn page_one() -> Answer {
    let body = r#"[{"id":1}]"#;
    Answer::Reply(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nLink: </link-cursor?cursor=2>; rel=\"next\"\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes(),
    )
}

/// Page one answered at once and page two by `second`.
async fn two_page_server(second: fn() -> Answer) -> RawServer {
    raw_server(move |head| if is_page_two(head) { second() } else { page_one() }).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_page_body_cut_off_after_its_head_is_a_body_interruption() {
    const PARTIAL: &[u8] = br#"[{"id":2"#;
    let mut server = two_page_server(|| {
        let mut bytes = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n".to_vec();
        bytes.extend_from_slice(PARTIAL);
        Answer::Reply(bytes)
    })
    .await;
    let client = client_for(&server.url);

    let (error, incomplete) = both_ways(&client).await;

    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    assert_eq!(api.kind, ErrorKind::BodyInterrupted);
    assert_eq!(api.body, Bytes::from_static(PARTIAL));
    assert_eq!((meta.status.as_u16(), meta.request.page), (200, Some(2)));
    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(server.heads_through_control().await.iter().filter(|head| is_page_two(head)).count(), 2, "no retry");
}

// ---------------------------------------------------------------------------
// Shared deadline and cancellation (R19)
// ---------------------------------------------------------------------------

const BUDGET: Duration = Duration::from_millis(300);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deadline_that_expires_while_page_two_waits_reports_the_completed_pages_and_stays_failed() {
    let mut server = two_page_server(|| Answer::Hang).await;
    let client = client_for(&server.url);
    let deadline = Deadline::after(BUDGET);

    let mut pages = client.send(LinkCursor::get()).pager().deadline(&deadline).pages();
    pages.next().await.expect("page 1").expect("a page");
    let error = pages.next().await.expect_err("the deadline expires");

    let PagerError::DeadlineExceeded { completed_pages, error: transport } = &error else { panic!("{error:?}") };
    assert_eq!(*completed_pages, 1);
    assert_eq!(transport.request.page, Some(2));
    let TransportErrorKind::DeadlineExceeded { completed_requests, deadline_elapsed } = transport.kind else {
        panic!("{transport:?}")
    };
    assert_eq!(completed_requests, 1);
    assert!(deadline_elapsed >= BUDGET, "{deadline_elapsed:?}");
    assert!(deadline_elapsed < Duration::from_secs(1), "{deadline_elapsed:?}");

    let again = pages.next().await.expect_err("already failed");
    assert_eq!(again, error);
    let heads = server.heads_through_control().await;
    assert_eq!(heads.len(), 2, "{heads:?}");
    assert!(is_page_two(&heads[1]), "{heads:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deadline_that_expires_between_pages_sends_nothing_more() {
    let mut server = two_page_server(page_one).await;
    let client = client_for(&server.url);
    let deadline = Deadline::after(Duration::from_millis(150));

    let mut pages = client.send(LinkCursor::get()).deadline(&deadline).pager().pages();
    pages.next().await.expect("page 1").expect("a page");
    tokio::time::sleep_until(deadline.expires_at()).await;
    let error = pages.next().await.expect_err("expired");

    let PagerError::DeadlineExceeded { completed_pages: 1, error: transport } = &error else { panic!("{error:?}") };
    assert!(
        matches!(transport.kind, TransportErrorKind::DeadlineExceeded { completed_requests: 1, .. }),
        "{transport:?}"
    );
    let heads = server.heads_through_control().await;
    assert_eq!(heads.len(), 1, "page two was never sent: {heads:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_a_page_in_flight_closes_its_connection_and_sends_nothing_later() {
    let mut server = two_page_server(|| Answer::Hang).await;
    let client = client_for(&server.url);
    let deadline = Deadline::after(Duration::from_secs(10));

    let mut pages = client.send(LinkCursor::get()).pager().deadline(&deadline).pages();
    pages.next().await.expect("page 1").expect("a page");
    assert!(is_page_one(&server.heads.recv().await.expect("page 1 head")));
    tokio::select! {
        result = pages.next() => panic!("page two never answers: {:?}", result.map(|page| page.map(|page| page.number))),
        head = server.heads.recv() => assert!(is_page_two(&head.expect("page 2 head"))),
    }
    drop(pages);

    tokio::time::timeout(Duration::from_secs(2), server.closed.recv())
        .await
        .expect("the server saw page two's connection close")
        .expect("close reported");
    assert_eq!(deadline.completed_requests(), 1, "the dropped page is not counted");
    assert!(server.heads_through_control().await.is_empty(), "nothing was sent after the drop");
}

// ---------------------------------------------------------------------------
// Privacy with a live credential (R36)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_refused_link_carrying_a_secret_cursor_keeps_it_out_of_the_error_text_and_summaries() {
    let (a, b) = (MockServer::start().await, MockServer::start().await);
    first_page_links_to(&a, &format!(r#"<{}/link-cursor?cursor={SENTINEL}>; rel="next""#, b.uri())).await;
    let client = client(&a);

    let mut pages = client.send(LinkCursor::get()).pager().pages();
    pages.next().await.expect("page 1").expect("a page");
    let error = pages.next().await.expect_err("refused");
    assert_cross_origin_refusal(&error);
    let incomplete = client.send(LinkCursor::get()).pager().collect_all().await.expect_err("refused");

    for text in [error.to_string(), incomplete.to_string(), format!("{pages:?}")] {
        assert!(!text.contains(SENTINEL), "{text}");
        assert!(!text.contains("127.0.0.1"), "no URL in {text}");
    }
    assert!(received(&b).await.is_empty());
}

#[tokio::test]
async fn the_client_credential_is_in_no_page_or_pager_error_text_or_debug_output() {
    let (a, b) = (MockServer::start().await, MockServer::start().await);
    first_page_links_to(&a, r#"</link-cursor?cursor=2>; rel="next""#).await;
    on_query(&a, "cursor", "2", ResponseTemplate::new(500).set_body_raw(r#"{"message":"no"}"#, "application/json"))
        .await;
    let failing = bearer_client(&a.uri(), OriginPolicy::same_origin(), SENTINEL);

    let mut pages = failing.send(LinkCursor::get()).pager().pages();
    let page = pages.next().await.expect("page 1").expect("a page");
    let api_error = pages.next().await.expect_err("500");
    let api_incomplete = failing.send(LinkCursor::get()).pager().collect_all().await.expect_err("500");
    assert!(received(&a).await.iter().all(|request| authorization(request) == Some(format!("Bearer {SENTINEL}"))));

    let refusing_origin = MockServer::start().await;
    first_page_links_to(&refusing_origin, &format!(r#"<{}/x>; rel="next""#, b.uri())).await;
    let refusing = bearer_client(&refusing_origin.uri(), OriginPolicy::same_origin(), SENTINEL);
    let refused = refusing.send(LinkCursor::get()).pager().collect_all().await.expect_err("refused");

    // Raw bodies are kept as evidence and are not redacted; none of these
    // bodies carries the credential, so a leak can only come from metadata.
    let texts = [
        format!("{page:?}"),
        format!("{api_error}"),
        format!("{api_error:?}"),
        format!("{api_incomplete}"),
        format!("{api_incomplete:?}"),
        format!("{refused}"),
        format!("{refused:?}"),
        format!("{pages:?}"),
    ];
    for text in texts {
        assert!(!text.contains(SENTINEL), "{text}");
    }
}

// ---------------------------------------------------------------------------
// Response hooks on pages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_global_json_hook_reshapes_every_page_before_items_and_continuations_are_read() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "results": [{ "id": 1 }], "cursor": "c2" }))).await;
    on_query(&server, "after", "c2", json(json!({ "results": [{ "id": 2 }], "cursor": null }))).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let client = builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
        .pre_response_json(move |_, body| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(json!({ "data": body["results"], "next": body["cursor"] }))
        })
        .build()
        .expect("client builds");

    let items = client.send(NextField::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    assert_eq!(calls.load(Ordering::SeqCst), 2, "once per page");
}

#[tokio::test]
async fn a_global_hook_refusal_is_a_hook_failure_with_the_original_bytes() {
    let body = r#"{"data":[{"id":1}],"next":null}"#;
    let server = MockServer::start().await;
    otherwise(&server, ResponseTemplate::new(200).set_body_raw(body, "application/json")).await;
    let client = builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
        .pre_response_json(|_, _| Err(HookError::new()))
        .build()
        .expect("client builds");

    let error = client.send(NextField::get()).pager().fetch_page().await.expect_err("refused by the hook");

    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    assert_eq!(api.kind, ErrorKind::Hook);
    assert_eq!(api.body, Bytes::from_static(body.as_bytes()));
    assert_eq!(meta.request.page, Some(1));
}

#[tokio::test]
async fn an_endpoint_mutator_does_not_run_on_pages() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [{ "id": 1 }], "next": null }))).await;
    let calls = Arc::new(Mutex::new(0_u32));
    let counter = Arc::clone(&calls);
    let client = builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
        .mutate_response::<NextField, _>(move |_, _| {
            *counter.lock().expect("count") += 1;
            Err(HookError::new())
        })
        .build()
        .expect("client builds");

    let items = client.send(NextField::get()).pager().collect_all().await.expect("the mutator does not fail pages");
    assert_eq!(ids(&items), [1]);
    assert_eq!(*calls.lock().expect("count"), 0);

    // The same endpoint sent ordinarily runs it, so it is registered.
    let response = client.send(NextField::get()).full().await.expect("head");
    assert_eq!(response.error().map(|error| &error.kind), Some(&ErrorKind::Hook));
    assert_eq!(*calls.lock().expect("count"), 1);
}
