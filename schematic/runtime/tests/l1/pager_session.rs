//! Scoped sessions (R22, R22a) through the runtime pager against a local
//! mock server: the open, page, keep-alive, renew, and close requests a
//! scope sends, and what `with_session` returns on every exit path.
//!
//! Cancelling the `with_session` future sends nothing more: Rust has no
//! asynchronous drop and the runtime spawns no task, so a cancelled scope
//! leaves its session to the server's lease.

use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant};

use futures_util::FutureExt;
use schematic_runtime::pager::{
    DEFAULT_CLOSE_GRACE, Incomplete, IncompleteReason, PagerError, PagerErrorKind, SessionError,
};
use schematic_runtime::{Deadline, ErrorKind, Json, TransportErrorKind};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{MockServer, ResponseTemplate};

use crate::pager_support::{
    Named, PitSearch, ProviderError, ScrollSearch, client, hits, ids, json, line, named_hits, names, on, otherwise,
    plain_client, received,
};
use crate::support::{AbortOnDrop, Call, listener, read_head};

type Error = PagerError<ProviderError>;
type Unfinished = Incomplete<Named, ProviderError>;

/// A session handle that must never appear in error text or summaries.
const SENTINEL: &str = "SECRET-HANDLE-123";

/// The close grace the timing tests give, against a close that hangs for
/// [`HANG`].
const GRACE: Duration = Duration::from_millis(200);
const HANG: Duration = Duration::from_secs(10);

/// Every request the server received, one [`line`] each.
async fn lines(server: &MockServer) -> Vec<String> {
    received(server).await.iter().map(line).collect()
}

/// A 500 with a declared error body.
fn failure(message: &str) -> ResponseTemplate {
    ResponseTemplate::new(500).set_body_json(json!({ "message": message }))
}

/// A [`PitSearch`] server: the open returns `p0`, page one returns `a` with
/// handle `p1`, page two (after `a`) is answered by `second`, and the close
/// by `close`.
async fn pit_server(second: ResponseTemplate, close: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "id": "p0" }))).await;
    on(&server, path("/_pit"), close).await;
    on(&server, body_partial_json(json!({ "search_after": ["a"] })), second).await;
    on(&server, path("/_search"), json(named_hits(&["a"], json!({ "pit_id": "p1" })))).await;
    server
}

/// The page two that ends a [`pit_server`] list.
fn last_page() -> ResponseTemplate {
    json(named_hits(&[], json!({ "pit_id": "p2" })))
}

fn closed() -> ResponseTemplate {
    json(json!({ "succeeded": true }))
}

/// The requests of a [`pit_server`] scope that reads both pages and closes
/// the latest handle.
const BOTH_PAGES_CLOSED: [&str; 4] = [
    "POST /logs/_pit?keep_alive=1m",
    r#"POST /_search {"pit":{"id":"p0","keep_alive":"1m"}}"#,
    r#"POST /_search {"pit":{"id":"p1","keep_alive":"1m"},"search_after":["a"]}"#,
    r#"DELETE /_pit {"id":"p2"}"#,
];

/// The panic message of a caught panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<not a string>")
}

/// Asserts `error` is the close grace running out.
fn assert_grace_ran_out(error: &Error) {
    let PagerError::Transport(transport) = error else { panic!("{error:?}") };
    let TransportErrorKind::DeadlineExceeded { deadline_elapsed, .. } = transport.kind else {
        panic!("{transport:?}")
    };
    assert!(deadline_elapsed >= GRACE, "{deadline_elapsed:?}");
}

#[tokio::test]
async fn a_scroll_opened_by_its_first_page_pages_with_the_scroll_id_and_closes_the_latest_one() {
    let server = MockServer::start().await;
    on(&server, method("GET"), json(hits([1, 2], json!({ "_scroll_id": "s1" })))).await;
    on(
        &server,
        body_partial_json(json!({ "scroll_id": "s1" })),
        json(hits([3], json!({ "_scroll_id": "s2" }))),
    )
    .await;
    on(
        &server,
        body_partial_json(json!({ "scroll_id": "s2" })),
        json(hits([], json!({ "_scroll_id": "s3" }))),
    )
    .await;
    otherwise(&server, json(json!({}))).await;
    let client = client(&server);

    let items = client
        .send(ScrollSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect("closed")
        .expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    let lines: Vec<String> = received(&server).await.iter().map(line).collect();
    assert_eq!(
        lines,
        [
            "GET /logs/_search?scroll=1m",
            r#"POST /_search/scroll {"scroll":"1m","scroll_id":"s1"}"#,
            r#"POST /_search/scroll {"scroll":"1m","scroll_id":"s2"}"#,
            r#"DELETE /_search/scroll {"scroll_id":"s3"}"#,
        ]
    );
}

#[tokio::test]
async fn a_point_in_time_opens_once_pages_at_its_target_and_closes_with_the_latest_handle() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "id": "p0" }))).await;
    on(
        &server,
        body_partial_json(json!({ "search_after": ["a"] })),
        json(named_hits(&[], json!({ "pit_id": "p2" }))),
    )
    .await;
    on(&server, path("/_search"), json(named_hits(&["a"], json!({ "pit_id": "p1" })))).await;
    otherwise(&server, json(json!({}))).await;
    let client = client(&server);

    let items = client
        .send(PitSearch::get())
        .pager()
        .page_size(5)
        .keep_alive("2m")
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect("closed")
        .expect("complete");

    assert_eq!(names(&items), ["a"]);
    let lines: Vec<String> = received(&server).await.iter().map(line).collect();
    assert_eq!(
        lines,
        [
            "POST /logs/_pit?keep_alive=2m",
            r#"POST /_search {"pit":{"id":"p0","keep_alive":"2m"},"size":5}"#,
            r#"POST /_search {"pit":{"id":"p1","keep_alive":"2m"},"search_after":["a"],"size":5}"#,
            r#"DELETE /_pit {"id":"p2"}"#,
        ]
    );
}

#[tokio::test]
async fn without_a_caller_keep_alive_the_authored_default_goes_on_the_open_and_every_page() {
    let server = pit_server(last_page(), closed()).await;
    let client = client(&server);

    let items = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect("closed")
        .expect("complete");

    assert_eq!(names(&items), ["a"]);
    assert_eq!(lines(&server).await, BOTH_PAGES_CLOSED);
}

#[test]
fn the_close_grace_defaults_to_five_seconds_and_takes_the_callers_value() {
    let client = plain_client("http://127.0.0.1:9");

    assert_eq!(DEFAULT_CLOSE_GRACE, Duration::from_secs(5));
    assert_eq!(client.send(PitSearch::get()).pager().configured_close_grace(), DEFAULT_CLOSE_GRACE);
    assert_eq!(client.send(PitSearch::get()).pager().close_grace(GRACE).configured_close_grace(), GRACE);
}

/// A [`ScrollSearch`] server: page one holds 1 with `s1`, page two holds 2
/// with `s2`, page three is empty with `s3`; renew and close succeed.
async fn scroll_server() -> MockServer {
    let server = MockServer::start().await;
    on(&server, path("/_search/scroll/renew"), json(json!({}))).await;
    on(&server, method("DELETE"), json(json!({}))).await;
    on(&server, method("GET"), json(hits([1], json!({ "_scroll_id": "s1" })))).await;
    on(&server, body_partial_json(json!({ "scroll_id": "s1" })), json(hits([2], json!({ "_scroll_id": "s2" })))).await;
    on(&server, body_partial_json(json!({ "scroll_id": "s2" })), json(hits([], json!({ "_scroll_id": "s3" })))).await;
    server
}

#[tokio::test]
async fn renewing_inside_the_scope_sends_the_renew_step_with_the_latest_handle_and_its_constant() {
    let server = scroll_server().await;
    let client = client(&server);

    let (first, second, renewed_by_pages, renewed_by_scope) = client
        .send(ScrollSearch::get())
        .pager()
        .with_session(|scope| {
            Box::pin(async move {
                let (first, second, renewed_by_pages) = {
                    let mut pages = scope.pages();
                    let first = pages.next().await.expect("page 1").expect("a page");
                    let renewed = pages.renew().await.expect("renewed");
                    let second = pages.next().await.expect("page 2").expect("a page");
                    (first.items, second.items, renewed)
                };
                let renewed_by_scope = scope.renew().await.expect("renewed");
                (first, second, renewed_by_pages, renewed_by_scope)
            })
        })
        .await
        .expect("closed");

    assert_eq!((ids(&first), ids(&second)), (vec![1], vec![2]));
    assert!(renewed_by_pages && renewed_by_scope);
    assert_eq!(
        lines(&server).await,
        [
            "GET /logs/_search?scroll=1m",
            r#"POST /_search/scroll/renew {"scroll":"5m","scroll_id":"s1"}"#,
            r#"POST /_search/scroll {"scroll":"1m","scroll_id":"s1"}"#,
            r#"POST /_search/scroll/renew {"scroll":"5m","scroll_id":"s2"}"#,
            r#"DELETE /_search/scroll {"scroll_id":"s2"}"#,
        ]
    );
}

#[tokio::test]
async fn renewing_before_any_handle_is_known_returns_false_and_sends_nothing() {
    let server = scroll_server().await;
    let client = client(&server);

    let renewed = client
        .send(ScrollSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { (scope.renew().await, scope.pages().renew().await) }))
        .await
        .expect("nothing to close");

    assert!(matches!(renewed, (Ok(false), Ok(false))), "{renewed:?}");
    assert!(lines(&server).await.is_empty());
}

#[tokio::test]
async fn a_scope_that_returns_after_its_first_page_still_closes_the_latest_handle() {
    let server = pit_server(last_page(), closed()).await;
    let client = client(&server);

    let page = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.fetch_page().await }))
        .await
        .expect("closed")
        .expect("page 1");

    assert_eq!(names(&page.items), ["a"]);
    assert_eq!(lines(&server).await, [BOTH_PAGES_CLOSED[0], BOTH_PAGES_CLOSED[1], r#"DELETE /_pit {"id":"p1"}"#]);
}

#[tokio::test]
async fn a_scope_whose_closure_returns_an_error_closes_and_returns_that_error_unchanged() {
    let server = pit_server(failure("boom"), closed()).await;
    let client = client(&server);

    let result: Result<Result<Vec<Named>, Unfinished>, _> = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await;

    let incomplete = result.expect("closed").expect_err("page 2 failed");
    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(names(&incomplete.items), ["a"]);
    assert_eq!(incomplete.pages, 2);
    let Some(PagerError::Api { meta, .. }) = &incomplete.error else { panic!("{incomplete:?}") };
    assert_eq!((meta.status.as_u16(), meta.request.page), (500, Some(2)));
    assert_eq!(
        lines(&server).await,
        [BOTH_PAGES_CLOSED[0], BOTH_PAGES_CLOSED[1], BOTH_PAGES_CLOSED[2], r#"DELETE /_pit {"id":"p1"}"#]
    );
}

#[tokio::test]
async fn a_closure_that_panics_before_returning_its_future_sends_nothing_and_the_panic_resumes() {
    let server = pit_server(last_page(), closed()).await;
    let client = client(&server);

    let session = client.send(PitSearch::get()).pager().with_session::<_, ()>(|_scope| panic!("before the future"));
    let payload = AssertUnwindSafe(session).catch_unwind().await.expect_err("the panic resumes");

    assert_eq!(panic_message(payload.as_ref()), "before the future");
    assert!(lines(&server).await.is_empty(), "no handle, so no close");
}

#[tokio::test]
async fn a_closure_that_panics_after_a_page_closes_the_latest_handle_then_resumes_the_panic() {
    let server = pit_server(last_page(), closed()).await;
    let client = client(&server);

    let session = client.send(PitSearch::get()).pager().with_session::<_, ()>(|scope| {
        Box::pin(async move {
            scope.fetch_page().await.expect("page 1");
            panic!("after a page");
        })
    });
    let payload = AssertUnwindSafe(session).catch_unwind().await.expect_err("the panic resumes");

    assert_eq!(panic_message(payload.as_ref()), "after a page");
    assert_eq!(lines(&server).await, [BOTH_PAGES_CLOSED[0], BOTH_PAGES_CLOSED[1], r#"DELETE /_pit {"id":"p1"}"#]);
}

#[tokio::test]
async fn a_failed_close_returns_the_complete_list_with_the_close_error() {
    let server = pit_server(last_page(), failure("close refused")).await;
    let client = client(&server);

    let error = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect_err("the close failed");

    let SessionError { outcome, close, .. } = error;
    assert_eq!(names(&outcome.expect("every item")), ["a"]);
    let PagerError::Api { error, meta } = &close else { panic!("{close:?}") };
    assert_eq!(error.declared().map(|declared| declared.message.as_str()), Some("close refused"));
    assert_eq!(meta.status, 500);
    assert_eq!(lines(&server).await, BOTH_PAGES_CLOSED);
}

#[tokio::test]
async fn a_failed_close_after_a_closure_error_keeps_the_closure_error_as_the_outcome() {
    let server = pit_server(failure("boom"), failure("close refused")).await;
    let client = client(&server);

    let error = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect_err("the close failed");

    let incomplete = error.outcome.expect_err("the closure's error");
    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(names(&incomplete.items), ["a"]);
    assert_eq!(error.close.kind(), PagerErrorKind::Api);
    assert_eq!(lines(&server).await.last().map(String::as_str), Some(r#"DELETE /_pit {"id":"p1"}"#));
}

#[tokio::test]
async fn a_close_slower_than_its_grace_is_abandoned_with_a_deadline_error_and_the_list_kept() {
    let server = pit_server(last_page(), closed().set_delay(HANG)).await;
    let client = client(&server);

    let started = Instant::now();
    let error = client
        .send(PitSearch::get())
        .pager()
        .close_grace(GRACE)
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect_err("the close ran out of grace");
    let elapsed = started.elapsed();

    assert_eq!(names(&error.outcome.expect("every item")), ["a"]);
    assert_grace_ran_out(&error.close);
    assert!(elapsed < HANG / 4, "bounded by the grace, not the close: {elapsed:?}");
    assert_eq!(lines(&server).await, BOTH_PAGES_CLOSED);
}

#[tokio::test]
async fn an_early_return_under_a_long_page_deadline_bounds_the_close_by_the_grace() {
    let server = pit_server(last_page(), closed().set_delay(HANG)).await;
    let client = client(&server);
    let deadline = Deadline::after(Duration::from_secs(60));

    let started = Instant::now();
    let error = client
        .send(PitSearch::get())
        .pager()
        .deadline(&deadline)
        .close_grace(GRACE)
        .with_session(|scope| Box::pin(async move { scope.fetch_page().await }))
        .await
        .expect_err("the close ran out of grace");
    let elapsed = started.elapsed();

    assert_eq!(names(&error.outcome.expect("page 1").items), ["a"]);
    assert_grace_ran_out(&error.close);
    assert!(elapsed < HANG / 4, "{elapsed:?}");
    assert!(!deadline.is_expired());
    assert_eq!(lines(&server).await, [BOTH_PAGES_CLOSED[0], BOTH_PAGES_CLOSED[1], r#"DELETE /_pit {"id":"p1"}"#]);
}

#[tokio::test]
async fn the_close_is_still_sent_and_succeeds_after_the_pages_deadline_expired() {
    let budget = Duration::from_millis(300);
    let server = pit_server(last_page().set_delay(HANG), closed()).await;
    let client = client(&server);
    let deadline = Deadline::after(budget);

    let result = client
        .send(PitSearch::get())
        .pager()
        .deadline(&deadline)
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await;

    assert!(deadline.is_expired());
    let incomplete = result.expect("the close succeeded").expect_err("page 2 ran out of time");
    assert_eq!(incomplete.reason, IncompleteReason::DeadlineExceeded);
    assert_eq!(names(&incomplete.items), ["a"]);
    assert!(
        matches!(incomplete.error, Some(PagerError::DeadlineExceeded { completed_pages: 1, .. })),
        "{incomplete:?}"
    );
    assert_eq!(
        lines(&server).await,
        [BOTH_PAGES_CLOSED[0], BOTH_PAGES_CLOSED[1], BOTH_PAGES_CLOSED[2], r#"DELETE /_pit {"id":"p1"}"#]
    );
}

#[tokio::test]
async fn a_malformed_page_still_hands_its_new_handle_to_the_close() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "id": "p0" }))).await;
    on(&server, path("/_search"), json(json!({ "hits": { "hits": "not a list" }, "pit_id": "p-new" }))).await;
    otherwise(&server, closed()).await;
    let client = client(&server);

    let error = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.fetch_page().await }))
        .await
        .expect("closed")
        .expect_err("page 1 is malformed");

    let PagerError::Api { error, meta } = &error else { panic!("{error:?}") };
    assert_eq!((&error.kind, meta.request.page), (&ErrorKind::MalformedSuccess, Some(1)));
    assert_eq!(
        lines(&server).await,
        [BOTH_PAGES_CLOSED[0], BOTH_PAGES_CLOSED[1], r#"DELETE /_pit {"id":"p-new"}"#]
    );
}

#[tokio::test]
async fn a_malformed_first_page_that_opened_a_scroll_still_closes_it() {
    let server = MockServer::start().await;
    on(&server, method("GET"), json(json!({ "hits": { "hits": 7 }, "_scroll_id": "s1" }))).await;
    otherwise(&server, json(json!({}))).await;
    let client = client(&server);

    let error = client
        .send(ScrollSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect("closed")
        .expect_err("page 1 is malformed");

    assert_eq!(error.reason, IncompleteReason::HttpError);
    assert_eq!(
        lines(&server).await,
        ["GET /logs/_search?scroll=1m", r#"DELETE /_search/scroll {"scroll_id":"s1"}"#]
    );
}

#[tokio::test]
async fn a_failed_open_sends_no_page_and_no_close_and_a_second_traversal_does_not_reopen() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), failure("no index")).await;
    otherwise(&server, json(named_hits(&["a"], json!({ "pit_id": "p1" })))).await;
    let client = client(&server);

    let (first, second) = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| {
            Box::pin(async move {
                let first = scope.fetch_page().await.map(|page| page.items);
                (first, scope.collect_all().await)
            })
        })
        .await
        .expect("nothing to close");

    let error = first.expect_err("the open failed");
    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    assert_eq!(api.declared().map(|declared| declared.message.as_str()), Some("no index"));
    assert_eq!(meta.status, 500);
    let incomplete = second.expect_err("already failed");
    assert!(
        matches!(incomplete.error, Some(PagerError::AlreadyFailed { first: PagerErrorKind::Api })),
        "{incomplete:?}"
    );
    assert!(incomplete.items.is_empty());
    assert_eq!(lines(&server).await, ["POST /logs/_pit?keep_alive=1m"]);
}

#[tokio::test]
async fn an_open_reply_without_a_handle_is_a_missing_handle_with_no_page_and_no_close() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "acknowledged": true }))).await;
    otherwise(&server, json(named_hits(&["a"], json!({ "pit_id": "p1" })))).await;
    let client = client(&server);

    let error = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.fetch_page().await }))
        .await
        .expect("nothing to close")
        .expect_err("no handle");

    assert!(matches!(error, PagerError::MissingSessionHandle { .. }), "{error:?}");
    assert_eq!(lines(&server).await, ["POST /logs/_pit?keep_alive=1m"]);
}

#[tokio::test]
async fn no_handle_or_scope_summary_appears_in_session_error_text() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "id": SENTINEL }))).await;
    on(&server, path("/_pit"), failure("close refused")).await;
    on(&server, body_partial_json(json!({ "search_after": ["a"] })), failure("boom")).await;
    on(&server, path("/_search"), json(named_hits(&["a"], json!({ "pit_id": SENTINEL })))).await;
    let client = client(&server);

    let error = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| {
            Box::pin(async move {
                let unopened = format!("{scope:?}");
                let incomplete = scope.collect_all().await.expect_err("page 2 failed");
                let opened = format!("{scope:?}");
                let pages = format!("{:?}", scope.pages());
                (incomplete, [unopened, opened, pages])
            })
        })
        .await
        .expect_err("the close failed");

    let (incomplete, summaries) = &error.outcome;
    assert_eq!(names(&incomplete.items), ["a"]);
    assert!(summaries[1].contains("open: true"), "{}", summaries[1]);
    let texts = [
        error.to_string(),
        format!("{error:?}"),
        error.close.to_string(),
        format!("{:?}", error.close),
        incomplete.to_string(),
        format!("{incomplete:?}"),
    ];
    for text in texts.iter().chain(summaries) {
        assert!(!text.contains(SENTINEL), "{text}");
    }
    let sent = lines(&server).await;
    assert_eq!(sent.last().map(String::as_str), Some(format!(r#"DELETE /_pit {{"id":"{SENTINEL}"}}"#).as_str()));
}

#[tokio::test]
async fn a_page_error_whose_raw_body_carries_the_handle_keeps_it_out_of_its_display_text() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "id": "p0" }))).await;
    on(&server, path("/_search"), json(json!({ "hits": { "hits": 7 }, "pit_id": SENTINEL }))).await;
    otherwise(&server, json(json!({}))).await;
    let client = client(&server);

    let incomplete = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect("closed")
        .expect_err("page 1 is malformed");

    let page_error = incomplete.error.as_ref().expect("cause");
    for text in [incomplete.to_string(), page_error.to_string()] {
        assert!(!text.contains(SENTINEL), "{text}");
    }
    // The raw body is kept as evidence, unredacted, so its `Debug` shows it.
    let PagerError::Api { error, .. } = page_error else { panic!("{page_error:?}") };
    assert!(String::from_utf8_lossy(&error.body).contains(SENTINEL));
}

/// A raw [`PitSearch`] server that opens `p0`, never answers a page, and
/// answers anything else with `{}`.
struct HangingPages {
    url: String,
    /// `METHOD /path` of each request, in arrival order.
    requests: mpsc::UnboundedReceiver<String>,
    /// One message per hung page connection the client closed.
    closed: mpsc::UnboundedReceiver<()>,
    _accept: AbortOnDrop,
}

impl HangingPages {
    async fn start() -> Self {
        let (listener, url) = listener().await;
        let (request_sent, requests) = mpsc::unbounded_channel();
        let (closed_sent, closed) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let (request_sent, closed_sent) = (request_sent.clone(), closed_sent.clone());
                tokio::spawn(async move {
                    let request = read_request(&mut stream).await;
                    let _ = request_sent.send(request.clone());
                    match request.as_str() {
                        "POST /_search" => {
                            let mut buffer = [0_u8; 256];
                            while matches!(stream.read(&mut buffer).await, Ok(read) if read > 0) {}
                            let _ = closed_sent.send(());
                        }
                        "POST /logs/_pit?keep_alive=1m" => reply(&mut stream, r#"{"id":"p0"}"#).await,
                        _ => reply(&mut stream, "{}").await,
                    }
                });
            }
        });
        Self {
            url,
            requests,
            closed,
            _accept: AbortOnDrop(task),
        }
    }

    /// Every request received so far, after a control request proves that
    /// anything sent earlier has arrived.
    async fn requests_through_control(&mut self) -> Vec<String> {
        plain_client(&self.url).send(Call::<Json<Value>>::get("/control")).await.expect("control reply");
        let mut requests = Vec::new();
        while let Ok(request) = self.requests.try_recv() {
            requests.push(request);
        }
        assert_eq!(requests.pop().as_deref(), Some("GET /control"), "{requests:?}");
        requests
    }
}

/// Reads one request, head and body, as `METHOD /path?query`.
async fn read_request(stream: &mut TcpStream) -> String {
    let head = read_head(stream).await;
    let length = head
        .lines()
        .find_map(|header| header.to_ascii_lowercase().strip_prefix("content-length:")?.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0_u8; length];
    let _ = stream.read_exact(&mut body).await;
    let request_line = head.lines().next().unwrap_or_default();
    request_line.rsplit_once(' ').map_or(request_line, |(request, _)| request).to_owned()
}

async fn reply(stream: &mut TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_a_scope_while_a_page_is_in_flight_closes_its_connection_and_sends_no_close() {
    let mut server = HangingPages::start().await;
    let client = plain_client(&server.url);

    let mut session = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }));
    tokio::select! {
        result = &mut session => panic!("the page never answers: {result:?}"),
        () = async {
            assert_eq!(server.requests.recv().await.as_deref(), Some("POST /logs/_pit?keep_alive=1m"));
            assert_eq!(server.requests.recv().await.as_deref(), Some("POST /_search"));
        } => {}
    }
    drop(session);

    tokio::time::timeout(Duration::from_secs(2), server.closed.recv())
        .await
        .expect("the server saw the page connection close")
        .expect("close reported");
    assert!(server.requests_through_control().await.is_empty(), "the session is left to its lease");
}
