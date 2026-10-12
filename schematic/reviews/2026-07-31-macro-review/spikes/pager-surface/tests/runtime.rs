mod mock;

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use futures::{FutureExt, StreamExt, TryStreamExt};
use mock::{MockServer, Reply};
use pager_surface::generated::{Hit, Item};
use pager_surface::{Client, PageSizeError, PagerError};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(5);

fn ids(items: &[Item]) -> Vec<u64> {
    items.iter().map(|i| i.id).collect()
}

fn alive_tasks() -> usize {
    tokio::runtime::Handle::current().metrics().num_alive_tasks()
}

/// Polls the runtime's task count until it reaches `n` or `WAIT` passes.
async fn settle_to(n: usize) -> usize {
    let start = Instant::now();
    loop {
        let alive = alive_tasks();
        if alive == n || start.elapsed() > WAIT {
            return alive;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

#[tokio::test]
async fn page_number_sends_size_and_jumps_to_page() {
    let server = MockServer::start(|_, req| {
        let page: u64 = req.query("page").unwrap().parse().unwrap();
        let mut reply = Reply::json(json!([{ "id": page * 10 }]));
        if page < 3 {
            reply = reply.header("link", &format!("</repos/o/r/issues?page={}>; rel=\"next\"", page + 1));
        }
        reply
    });
    let client = Client::new(&server.url());
    let mut pages = client.list_issues().page_size(50).unwrap().page(2).pages();
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [20]);
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [30]);
    assert!(pages.next().await.unwrap().is_none());
    let reqs = server.requests();
    assert_eq!(reqs.len(), 2);
    assert_eq!(reqs[0].query("page").as_deref(), Some("2"));
    assert_eq!(reqs[1].query("page").as_deref(), Some("3"));
    assert!(reqs.iter().all(|r| r.query("per_page").as_deref() == Some("50")));
}

#[tokio::test]
async fn page_size_is_never_sent_unless_the_caller_set_it() {
    let server = MockServer::start(|_, _| Reply::json(json!([])));
    let client = Client::new(&server.url());
    let mut pages = client.list_issues().pages();
    assert!(pages.next().await.unwrap().is_none());
    assert_eq!(server.requests()[0].query("per_page"), None);
}

#[tokio::test]
async fn opaque_cursor_streams_items_across_pages() {
    let server = MockServer::start(|i, _| {
        let id = i as u64 + 1;
        Reply::json(json!({ "data": [{ "id": id }], "last_id": format!("r{id}"), "has_more": id < 3 }))
    });
    let client = Client::new(&server.url());
    let items: Vec<Item> = client.list_roles().pages().items().try_collect().await.unwrap();
    assert_eq!(ids(&items), [1, 2, 3]);
    let afters: Vec<_> = server.requests().iter().map(|r| r.query("after")).collect();
    assert_eq!(afters, [None, Some("r1".into()), Some("r2".into())]);
}

#[tokio::test]
async fn resumable_cursor_starts_after_and_goes_back() {
    let server = MockServer::start(|_, req| {
        let (data, first, last) = match (req.query("after_id"), req.query("before_id")) {
            (Some(a), None) if a == "x5" => (json!([{ "id": 6 }, { "id": 7 }]), "x6", "x7"),
            (None, Some(b)) if b == "x6" => (json!([{ "id": 4 }, { "id": 5 }]), "x4", "x5"),
            (None, Some(b)) if b == "x9" => (json!([{ "id": 8 }]), "x8", "x8"),
            other => panic!("unexpected {other:?}"),
        };
        Reply::json(json!({ "data": data, "first_id": first, "last_id": last, "has_more": true }))
    });
    let client = Client::new(&server.url());

    let mut pages = client.list_batches().page_size(2).unwrap().after("x5").pages();
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [6, 7]);
    assert_eq!(ids(&pages.prev().await.unwrap().unwrap().items), [4, 5]);

    let mut back = client.list_batches().before("x9").pages();
    assert_eq!(ids(&back.next().await.unwrap().unwrap().items), [8]);
    assert_eq!(server.requests()[2].query("before_id").as_deref(), Some("x9"));
}

#[tokio::test]
async fn next_link_in_body_with_total() {
    let server = MockServer::start(|_, req| {
        match req.query("page").as_deref() {
            None => Reply::json(json!({ "values": [{ "id": 1 }], "size": 2, "next": "/2.0/repositories/w?page=2&pagelen=1" })),
            Some("2") => Reply::json(json!({ "values": [{ "id": 2 }], "size": 2 })),
            other => panic!("unexpected {other:?}"),
        }
    });
    let client = Client::new(&server.url());
    let mut pages = client.list_repos().page_size(1).unwrap().pages();
    let first = pages.next().await.unwrap().unwrap();
    assert_eq!((ids(&first.items), first.total), (vec![1], Some(2)));
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [2]);
    assert!(pages.next().await.unwrap().is_none());
    // The link is followed verbatim: the client does not re-add pagelen.
    assert_eq!(server.requests()[1].target, "/2.0/repositories/w?page=2&pagelen=1");
}

#[tokio::test]
async fn next_link_in_link_header_with_prev() {
    let server = MockServer::start(|_, req| {
        let page: u64 = req.query("p").map_or(1, |p| p.parse().unwrap());
        let mut links = Vec::new();
        if page < 3 {
            links.push(format!("</commits?p={}>; rel=\"next\"", page + 1));
        }
        if page > 1 {
            links.push(format!("</commits?p={}>; rel=\"prev\"", page - 1));
        }
        let mut reply = Reply::json(json!([{ "id": page }]));
        if !links.is_empty() {
            reply = reply.header("link", &links.join(", "));
        }
        reply
    });
    let client = Client::new(&server.url());
    let mut pages = client.list_commits().pages();
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [1]);
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [2]);
    assert_eq!(ids(&pages.prev().await.unwrap().unwrap().items), [1]);
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [2]);
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [3]);
    assert!(pages.next().await.unwrap().is_none());
}

fn scroll_server(pages_with_hits: usize, fail_at: Option<usize>) -> MockServer {
    MockServer::start(move |i, req| {
        if req.method == "DELETE" {
            return Reply::json(json!({ "succeeded": true }));
        }
        if fail_at == Some(i) {
            return Reply::json(json!({ "error": "boom" })).status(500);
        }
        let hits: Vec<_> = if i < pages_with_hits { vec![json!({ "_source": { "id": i } })] } else { vec![] };
        // The handle changes per page; the latest must win.
        Reply::json(json!({ "_scroll_id": format!("h{i}"), "hits": { "total": { "value": pages_with_hits }, "hits": hits } }))
    })
}

#[tokio::test]
async fn scroll_session_opens_pages_and_closes_at_the_end() {
    let server = scroll_server(2, None);
    let client = Client::new(&server.url());
    let hits: Vec<Hit> = client.search_scroll().page_size(500).unwrap().keep_alive("2m").pages().items().try_collect().await.unwrap();
    assert_eq!(hits.iter().map(|h| h.source.id).collect::<Vec<_>>(), [0, 1]);
    let reqs = server.requests();
    let shape: Vec<_> = reqs.iter().map(|r| (r.method.as_str(), r.path())).collect();
    assert_eq!(
        shape,
        [("POST", "/idx/_search"), ("POST", "/_search/scroll"), ("POST", "/_search/scroll"), ("DELETE", "/_search/scroll")]
    );
    assert_eq!(reqs[0].query("scroll").as_deref(), Some("2m"));
    assert_eq!(reqs[0].body, Some(json!({ "size": 500 })));
    assert_eq!(reqs[1].body, Some(json!({ "scroll": "2m", "scroll_id": "h0" })));
    assert_eq!(reqs[2].body, Some(json!({ "scroll": "2m", "scroll_id": "h1" })));
    assert_eq!(reqs[3].body, Some(json!({ "scroll_id": "h2" })));
    assert!(client.leak_warnings().is_empty());
}

#[tokio::test]
async fn scroll_session_closes_on_error() {
    let server = scroll_server(5, Some(1));
    let client = Client::new(&server.url());
    let mut pages = client.search_scroll().pages();
    pages.next().await.unwrap().unwrap();
    assert!(matches!(pages.next().await, Err(PagerError::Http { status: 500, .. })));
    let last = server.requests().last().cloned().unwrap();
    assert_eq!((last.method.as_str(), last.body), ("DELETE", Some(json!({ "scroll_id": "h0" }))));
}

// ---------------------------------------------------------------------------
// (b) Shared deadline
// ---------------------------------------------------------------------------

fn slow_cursor_server(delay: Duration) -> MockServer {
    MockServer::start(move |i, _| {
        Reply::json(json!({ "data": [{ "id": i }], "last_id": format!("c{i}"), "has_more": true })).delay(delay)
    })
}

#[tokio::test]
async fn deadline_spans_pages_and_stops_dispatch() {
    let server = slow_cursor_server(Duration::from_millis(150));
    let client = Client::new(&server.url());
    let budget = Duration::from_millis(400);
    let mut pages = client.list_roles().deadline(budget).pages();
    let started = Instant::now();

    pages.next().await.unwrap().unwrap();
    pages.next().await.unwrap().unwrap();
    let err = pages.next().await.unwrap_err();
    let PagerError::DeadlineExceeded { completed_pages, elapsed } = err else {
        panic!("expected DeadlineExceeded, got {err:?}");
    };
    assert_eq!(completed_pages, 2);
    assert!(elapsed >= budget && elapsed < budget + Duration::from_millis(150), "{elapsed:?}");
    assert!(started.elapsed() < budget + Duration::from_millis(150));

    // Each request got the remaining budget, not the 30 s default.
    let timeouts: Vec<_> = client.sent().iter().map(|s| s.timeout).collect();
    assert_eq!(timeouts.len(), 3);
    assert!(timeouts[0] <= budget && timeouts[0] > Duration::from_millis(390), "{timeouts:?}");
    assert!(timeouts[1] < Duration::from_millis(260) && timeouts[1] > Duration::from_millis(200), "{timeouts:?}");
    assert!(timeouts[2] < Duration::from_millis(110), "{timeouts:?}");

    // The third request was cut off: the server saw its connection close.
    assert!(server.wait_until(WAIT, |s| s.aborted.iter().any(|(i, _)| *i == 2)).await);

    // Spent: no further request is dispatched.
    assert!(matches!(pages.next().await, Err(PagerError::DeadlineExceeded { completed_pages: 2, .. })));
    assert_eq!(server.requests().len(), 3);
    assert_eq!(client.sent().len(), 3);
}

#[tokio::test]
async fn deadline_spent_between_pages_sends_nothing() {
    let server = slow_cursor_server(Duration::ZERO);
    let client = Client::new(&server.url());
    let mut pages = client.list_roles().deadline(Duration::from_millis(100)).pages();
    pages.next().await.unwrap().unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await; // the caller dawdles
    assert!(matches!(pages.next().await, Err(PagerError::DeadlineExceeded { completed_pages: 1, .. })));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn without_a_deadline_each_request_gets_the_default_timeout() {
    let server = slow_cursor_server(Duration::ZERO);
    let client = Client::new(&server.url());
    client.list_roles().pages().next().await.unwrap();
    assert_eq!(client.sent()[0].timeout, Duration::from_secs(30));
}

// ---------------------------------------------------------------------------
// (c) Drop-cancel
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dropping_next_mid_page_closes_the_connection_and_leaves_no_task() {
    let server = MockServer::start(|_, _| Reply::hang());
    let client = Client::new(&server.url());
    assert_eq!(alive_tasks(), 0);
    let mut pages = client.list_roles().pages();

    tokio::select! {
        result = pages.next() => panic!("a hung page returned: {result:?}"),
        arrived = server.wait_until(WAIT, |s| s.requests.len() == 1) => {
            assert!(arrived);
            // Non-vacuity: hyper's connection task is alive while in flight.
            assert!(alive_tasks() >= 1);
        }
    }
    // The `next()` future is gone; the server must see the connection close.
    assert!(server.wait_until(WAIT, |s| s.aborted.iter().any(|(i, _)| *i == 0)).await);
    drop(pages);
    drop(client);
    assert_eq!(settle_to(0).await, 0, "a task outlived the dropped pager");
}

#[test]
fn runtime_shutdown_after_drop_cancel_is_prompt() {
    let server = MockServer::start(|_, _| Reply::hang());
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    runtime.block_on(async {
        let client = Client::new(&server.url());
        let mut pages = client.list_roles().pages();
        let _ = tokio::time::timeout(Duration::from_millis(100), pages.next()).await;
    });
    let start = Instant::now();
    runtime.shutdown_timeout(Duration::from_secs(5));
    assert!(start.elapsed() < Duration::from_millis(500), "{:?}", start.elapsed());
}

// ---------------------------------------------------------------------------
// (d) Host policy (D12) on followed links
// ---------------------------------------------------------------------------

fn link_server(next: impl Fn() -> String + Send + Sync + 'static) -> MockServer {
    MockServer::start(move |_, _| Reply::json(json!({ "values": [{ "id": 1 }], "next": next() })))
}

#[tokio::test]
async fn link_to_another_port_is_rejected_unsent_after_a_good_page() {
    let other = MockServer::start(|_, _| Reply::json(json!({ "values": [] })));
    let other_url = other.url();
    let base = link_server(move || format!("{other_url}/steal"));
    let client = Client::with_host_policy(&base.url(), &[]);
    let mut pages = client.list_repos().pages();
    pages.next().await.unwrap().unwrap();
    let err = pages.next().await.unwrap_err();
    assert!(matches!(err, PagerError::HostPolicy { .. }), "{err:?}");
    assert_eq!(other.requests().len(), 0);
    assert_eq!(client.sent().len(), 1);
}

#[tokio::test]
async fn link_with_other_scheme_or_host_is_rejected() {
    for variant in ["https", "localhost"] {
        let addr_cell = Arc::new(std::sync::OnceLock::<String>::new());
        let cell = addr_cell.clone();
        let base = link_server(move || {
            let addr = cell.get().unwrap();
            match variant {
                "https" => format!("https://{addr}/x"),
                _ => format!("http://localhost:{}/x", addr.rsplit(':').next().unwrap()),
            }
        });
        addr_cell.set(base.addr.to_string()).unwrap();
        let client = Client::with_host_policy(&base.url(), &[]);
        let mut pages = client.list_repos().pages();
        pages.next().await.unwrap().unwrap();
        assert!(matches!(pages.next().await, Err(PagerError::HostPolicy { .. })), "{variant}");
        assert_eq!(base.requests().len(), 1, "{variant}");
    }
}

#[tokio::test]
async fn link_to_a_declared_extra_origin_is_followed() {
    let other = MockServer::start(|_, _| Reply::json(json!({ "values": [{ "id": 2 }] })));
    let other_url = other.url();
    let base = link_server(move || format!("{other_url}/page2"));
    let client = Client::with_host_policy(&base.url(), &[&other.url()]);
    let items: Vec<Item> = client.list_repos().pages().items().try_collect().await.unwrap();
    assert_eq!(ids(&items), [1, 2]);
    assert_eq!(other.requests().len(), 1);
}

#[tokio::test]
async fn without_the_policy_a_cross_origin_link_is_followed() {
    // D12 "policy off = today's behavior". Recorded as a finding: see
    // findings.md on whether link following should always be origin-checked.
    let other = MockServer::start(|_, _| Reply::json(json!({ "values": [] })));
    let other_url = other.url();
    let base = link_server(move || format!("{other_url}/page2"));
    let client = Client::new(&base.url());
    let _: Vec<Item> = client.list_repos().pages().items().try_collect().await.unwrap();
    assert_eq!(other.requests().len(), 1);
}

// ---------------------------------------------------------------------------
// (e) Page size above the declared max
// ---------------------------------------------------------------------------

#[tokio::test]
async fn page_size_outside_declared_range_errors_before_any_request() {
    let server = MockServer::start(|_, _| Reply::json(json!([])));
    let client = Client::new(&server.url());
    let err = client.list_issues().page_size(101).err().unwrap();
    assert_eq!(err, PageSizeError { requested: 101, min: 1, max: Some(100) });
    assert!(client.list_issues().page_size(0).is_err());
    assert!(client.list_issues().page_size(100).is_ok());
    // The error converts into PagerError for `?` in an async fn.
    let as_pager: PagerError = err.into();
    assert!(matches!(as_pager, PagerError::PageSize(_)));
    assert_eq!(server.requests().len(), 0);
    assert!(client.sent().is_empty());
}

// ---------------------------------------------------------------------------
// (f) Sessioned pager dropped early: #[must_use] + warning vs with_session
// ---------------------------------------------------------------------------

fn deletes(server: &MockServer) -> usize {
    server.requests().iter().filter(|r| r.method == "DELETE").count()
}

#[tokio::test]
async fn option_a_drop_without_close_leaks_and_warns() {
    let server = scroll_server(5, None);
    let client = Client::new(&server.url());
    {
        let mut pages = client.search_scroll().pages();
        pages.next().await.unwrap().unwrap();
    }
    // Drop is synchronous and spawns nothing, so nothing can send later.
    assert_eq!(deletes(&server), 0);
    assert_eq!(client.leak_warnings().len(), 1, "{:?}", client.leak_warnings());
}

#[tokio::test]
async fn option_a_explicit_close_on_early_exit() {
    let server = scroll_server(5, None);
    let client = Client::new(&server.url());
    let mut pages = client.search_scroll().pages();
    pages.next().await.unwrap().unwrap();
    pages.close().await.unwrap();
    drop(pages);
    assert_eq!(deletes(&server), 1);
    assert!(client.leak_warnings().is_empty());
}

#[tokio::test]
async fn option_c_with_session_closes_on_early_return() {
    let server = scroll_server(5, None);
    let client = Client::new(&server.url());
    let first = client
        .search_scroll()
        .with_session(async |pages| Ok(pages.next().await?.map(|p| p.items.len())))
        .await
        .unwrap();
    assert_eq!(first, Some(1));
    assert_eq!(deletes(&server), 1);
    assert!(client.leak_warnings().is_empty());
}

#[tokio::test]
async fn option_c_with_session_closes_on_error_in_body() {
    let server = scroll_server(5, None);
    let client = Client::new(&server.url());
    let result: Result<(), _> = client
        .search_scroll()
        .with_session(async |pages| {
            pages.next().await?;
            Err(PagerError::Decode("caller gave up".into()))
        })
        .await;
    assert!(matches!(result, Err(PagerError::Decode(_))));
    assert_eq!(deletes(&server), 1);
}

#[tokio::test]
async fn option_c_with_session_closes_on_panic_in_body() {
    let server = scroll_server(5, None);
    let client = Client::new(&server.url());
    let outcome = AssertUnwindSafe(client.search_scroll().with_session(async |pages| -> Result<(), PagerError> {
        pages.next().await?;
        panic!("caller bug");
    }))
    .catch_unwind()
    .await;
    assert!(outcome.is_err(), "the panic propagates");
    assert_eq!(deletes(&server), 1);
}

#[tokio::test]
async fn option_c_with_session_does_not_close_when_cancelled() {
    let server = MockServer::start(|i, req| {
        if req.method == "DELETE" {
            return Reply::json(json!({}));
        }
        let reply = Reply::json(json!({ "_scroll_id": "h", "hits": { "hits": [{ "_source": { "id": i } }] } }));
        if i == 0 { reply } else { Reply::hang() }
    });
    let client = Client::new(&server.url());
    let pages_done = Arc::new(AtomicUsize::new(0));
    let counter = pages_done.clone();
    let scoped = client.search_scroll().with_session(async move |pages| {
        while pages.next().await?.is_some() {
            counter.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    });
    tokio::select! {
        _ = scoped => panic!("the second page hangs"),
        arrived = server.wait_until(WAIT, |s| s.requests.len() == 2) => assert!(arrived),
    }
    // Cancellation drops the scope's future: its close code never runs.
    assert_eq!(pages_done.load(Ordering::SeqCst), 1);
    assert_eq!(deletes(&server), 0);
    assert_eq!(client.leak_warnings().len(), 1);
}

#[tokio::test]
async fn option_b_close_on_drop_needs_a_detached_task() {
    let server = scroll_server(5, None);
    let client = Client::with_close_on_drop(&server.url());
    {
        let mut pages = client.search_scroll().pages();
        pages.next().await.unwrap().unwrap();
    }
    // The close happens, but on a task nobody owns or awaits (D13 forbids
    // detached work), and its error is unobservable.
    assert!(alive_tasks() >= 1);
    assert!(server.wait_until(WAIT, |s| s.requests.iter().any(|r| r.method == "DELETE")).await);
    assert!(client.leak_warnings().is_empty());
}

// ---------------------------------------------------------------------------
// (g) Repeated or invalid continuation
// ---------------------------------------------------------------------------

fn cursor_sequence(cursors: &'static [serde_json::Value]) -> MockServer {
    MockServer::start(move |i, _| {
        let cursor = cursors.get(i).cloned().unwrap_or(serde_json::Value::Null);
        Reply::json(json!({ "data": [{ "id": i }], "last_id": cursor, "has_more": true }))
    })
}

#[tokio::test]
async fn same_cursor_twice_is_a_stall() {
    static CURSORS: std::sync::LazyLock<Vec<serde_json::Value>> = std::sync::LazyLock::new(|| vec![json!("a"), json!("a")]);
    let server = cursor_sequence(&CURSORS);
    let client = Client::new(&server.url());
    let mut pages = client.list_roles().pages();
    pages.next().await.unwrap().unwrap();
    let err = pages.next().await.unwrap_err();
    assert!(matches!(&err, PagerError::Stalled { continuation, completed_pages: 1 } if continuation == "a"), "{err:?}");
    // A failed pager stays failed and sends nothing.
    assert!(matches!(pages.next().await, Err(PagerError::Failed { .. })));
    assert_eq!(server.requests().len(), 2);
}

#[tokio::test]
async fn cursor_cycle_is_a_stall() {
    static CURSORS: std::sync::LazyLock<Vec<serde_json::Value>> =
        std::sync::LazyLock::new(|| vec![json!("a"), json!("b"), json!("a")]);
    let server = cursor_sequence(&CURSORS);
    let client = Client::new(&server.url());
    let result: Result<Vec<Item>, _> = client.list_roles().pages().items().try_collect().await;
    assert!(matches!(result, Err(PagerError::Stalled { completed_pages: 2, .. })), "{result:?}");
    assert_eq!(server.requests().len(), 3);
}

#[tokio::test]
async fn more_without_cursor_is_missing_continuation() {
    static CURSORS: std::sync::LazyLock<Vec<serde_json::Value>> = std::sync::LazyLock::new(|| vec![json!(null)]);
    let server = cursor_sequence(&CURSORS);
    let client = Client::new(&server.url());
    assert!(matches!(client.list_roles().pages().next().await, Err(PagerError::MissingContinuation)));
}

#[tokio::test]
async fn non_scalar_cursor_is_invalid() {
    static CURSORS: std::sync::LazyLock<Vec<serde_json::Value>> = std::sync::LazyLock::new(|| vec![json!({ "k": 1 })]);
    let server = cursor_sequence(&CURSORS);
    let client = Client::new(&server.url());
    assert!(matches!(client.list_roles().pages().next().await, Err(PagerError::InvalidContinuation(_))));
}

#[tokio::test]
async fn link_cycle_is_a_stall_and_bad_link_is_invalid() {
    let server = MockServer::start(|i, _| {
        Reply::json(json!({ "values": [{ "id": i }], "next": "/p2" }))
    });
    let client = Client::new(&server.url());
    let mut pages = client.list_repos().pages();
    pages.next().await.unwrap().unwrap();
    assert!(matches!(pages.next().await, Err(PagerError::Stalled { .. })));
    assert_eq!(server.requests().len(), 2);

    let bad = MockServer::start(|_, _| Reply::json(json!({ "values": [], "next": "http://[::1" })));
    let client = Client::new(&bad.url());
    let mut pages = client.list_repos().pages();
    assert!(matches!(pages.next().await, Err(PagerError::InvalidContinuation(_))));
    assert_eq!(bad.requests().len(), 1);
}

#[tokio::test]
async fn items_stream_ends_after_first_error() {
    let server = MockServer::start(|i, _| {
        if i == 0 {
            Reply::json(json!({ "data": [{ "id": 1 }, { "id": 2 }], "last_id": "a", "has_more": true }))
        } else {
            Reply::json(json!({})).status(503)
        }
    });
    let client = Client::new(&server.url());
    let mut items = std::pin::pin!(client.list_roles().pages().items());
    assert_eq!(items.next().await.unwrap().unwrap().id, 1);
    assert_eq!(items.next().await.unwrap().unwrap().id, 2);
    assert!(matches!(items.next().await, Some(Err(PagerError::Http { status: 503, .. }))));
    assert!(items.next().await.is_none());
}
