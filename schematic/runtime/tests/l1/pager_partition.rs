//! Partitioned traversals (R23) through the runtime pager against a local
//! mock server.
//!
//! The point-in-time cases use [`PitSearch`]: one open request and one close
//! per scope however many partitions page it, slice inputs on every page
//! request, independent continuations and failures per partition, and the
//! handle each partition sends being the one most recently observed, in
//! response-arrival order. Ordering-sensitive cases run against a scripted
//! raw server that holds a response until another request has arrived, so
//! the order is driven by server events rather than by sleeps.

use std::sync::Arc;
use std::time::Duration;

use futures_util::future::join_all;
use schematic_runtime::pager::spec::{
    Boundary, CursorLeg, CursorPart, CursorSource, Input, Kind, Partition, Source, Step,
};
use schematic_runtime::pager::{CursorKind, ItemSelector, Paginated, PagerError, Pages, Partitioned, Strategy};
use schematic_runtime::{EndpointSpec, InvalidArgument};
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::watch;
use wiremock::matchers::{body_partial_json, path, query_param};
use wiremock::{MockServer, Request, ResponseTemplate};

use crate::pager_support::{
    Item, PitSearch, ProviderError, TestApi, client, endpoint, ids, json, named_hits, names, on, otherwise,
    query, received,
};
use crate::support::{AbortOnDrop, listener, plain_client, read_head, write_all};

type Error = PagerError<ProviderError>;

// A sessionless list sliced by `slice`/`slices` query inputs, at most three
// slices; the cursor is `after` from the body's `next`, and a page without
// `next` ends the slice.
endpoint!(
    SlicedList,
    CursorKind,
    Item,
    "/sliced",
    Strategy {
        kind: Kind::Cursor {
            forward: CursorLeg {
                parts: &[CursorPart {
                    param: Input::Query("after"),
                    source: CursorSource::Field(Source::Body(&["next"])),
                }],
                boundary: Boundary::Exclusive,
            },
            backward: None,
            more: None,
            resumable: false,
        },
        size: None,
        constants: &[],
        total: None,
        terminal_values: &[],
        follow_up: None,
        session: None,
        partition: Some(Partition {
            index: Input::Query("slice"),
            count: Input::Query("slices"),
            max: Some(3),
        }),
    },
    ItemSelector {
        record: &[Step::Key("data"), Step::Each],
        projection: &[],
    }
);
impl Partitioned for SlicedList {}

/// Pages `pages` to its end or its first error: the items delivered, and
/// the error that stopped it, if any.
async fn drain<R>(pages: &mut Pages<'_, TestApi, R>) -> (Vec<R::Item>, Option<Error>)
where
    R: Paginated<Api = TestApi> + EndpointSpec<Failure = ProviderError>,
{
    let mut items = Vec::new();
    loop {
        match pages.next().await {
            Ok(Some(page)) => items.extend(page.items),
            Ok(None) => return (items, None),
            Err(error) => return (items, Some(error)),
        }
    }
}

fn body(request: &Request) -> Value {
    serde_json::from_slice(&request.body).unwrap_or(Value::Null)
}

fn slice_id(body: &Value) -> Option<u64> {
    body.pointer("/slice/id").and_then(Value::as_u64)
}

/// The page requests a wiremock server received, as `(slice id, body)`.
async fn page_bodies(server: &MockServer) -> Vec<(u64, Value)> {
    received(server)
        .await
        .iter()
        .filter(|request| request.url.path() == "/_search")
        .map(|request| {
            let body = body(request);
            (slice_id(&body).expect("every page request names its slice"), body)
        })
        .collect()
}

fn reason(result: Result<Vec<impl std::fmt::Debug>, InvalidArgument>) -> String {
    result.expect_err("the count is refused").reason().to_owned()
}

// ---------------------------------------------------------------------------
// Point in time over wiremock
// ---------------------------------------------------------------------------

#[tokio::test]
async fn four_partitions_share_one_open_and_one_close_and_each_slice_pages_its_own_items() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "id": "p0" }))).await;
    for slice in 0..4 {
        let first = format!("s{slice}-1");
        on(
            &server,
            body_partial_json(json!({ "search_after": [first] })),
            json(named_hits(&[], json!({ "pit_id": "p1" }))),
        )
        .await;
    }
    for slice in 0..4 {
        let first = format!("s{slice}-1");
        on(
            &server,
            body_partial_json(json!({ "slice": { "id": slice } })),
            json(named_hits(&[first.as_str()], json!({ "pit_id": "p1" }))),
        )
        .await;
    }
    otherwise(&server, json(json!({}))).await;
    let client = client(&server);

    let outcomes = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| {
            Box::pin(async move {
                let mut parts = scope.partitioned(4).expect("four is within the maximum");
                let outcomes = join_all(parts.iter_mut().map(drain)).await;
                let finished: Vec<bool> = parts.iter().map(Pages::is_finished).collect();
                let fetched: Vec<u32> = parts.iter().map(Pages::pages_fetched).collect();
                (outcomes, finished, fetched)
            })
        })
        .await
        .expect("closed");

    let (outcomes, finished, fetched) = outcomes;
    for (slice, (items, error)) in outcomes.iter().enumerate() {
        assert!(error.is_none(), "slice {slice}: {error:?}");
        assert_eq!(names(items), [format!("s{slice}-1")], "slice {slice}");
    }
    assert_eq!(finished, [true; 4]);
    assert_eq!(fetched, [2; 4]);

    let requests = received(&server).await;
    let opens: Vec<&Request> = requests.iter().filter(|request| request.url.path() == "/logs/_pit").collect();
    let closes: Vec<&Request> = requests.iter().filter(|request| request.method.as_str() == "DELETE").collect();
    assert_eq!(opens.len(), 1);
    assert_eq!(closes.len(), 1);
    assert_eq!(requests.first().map(|request| request.url.path()), Some("/logs/_pit"), "the open precedes every page");
    assert_eq!(requests.last().map(|request| request.method.as_str()), Some("DELETE"), "the close follows every page");
    assert_eq!(body(closes[0]), json!({ "id": "p1" }));

    let pages = page_bodies(&server).await;
    assert_eq!(pages.len(), 8);
    for slice in 0..4 {
        let bodies: Vec<&Value> = pages.iter().filter(|(id, _)| *id == slice).map(|(_, body)| body).collect();
        assert_eq!(bodies.len(), 2, "slice {slice}");
        for body in &bodies {
            assert_eq!(body["slice"], json!({ "id": slice, "max": 4 }), "slice {slice}");
        }
        assert_eq!(bodies[0].get("search_after"), None, "slice {slice}");
        assert_eq!(bodies[1]["search_after"], json!([format!("s{slice}-1")]), "slice {slice}");
    }
}

#[tokio::test]
async fn each_partition_follows_its_own_cursor_and_a_stalled_or_failed_partition_leaves_the_others_complete() {
    let server = MockServer::start().await;
    on(&server, path("/logs/_pit"), json(json!({ "id": "p0" }))).await;
    let pit = || json!({ "pit_id": "p1" });
    on(&server, body_partial_json(json!({ "search_after": ["a1"] })), json(named_hits(&["a2"], pit()))).await;
    on(&server, body_partial_json(json!({ "search_after": ["a2"] })), json(named_hits(&[], pit()))).await;
    on(&server, body_partial_json(json!({ "search_after": ["b1"] })), json(named_hits(&[], pit()))).await;
    on(&server, body_partial_json(json!({ "slice": { "id": 0 } })), json(named_hits(&["a1"], pit()))).await;
    on(&server, body_partial_json(json!({ "slice": { "id": 1 } })), json(named_hits(&["b1"], pit()))).await;
    // Slice 2 answers the same hit, and so the same cursor, on every page.
    on(&server, body_partial_json(json!({ "slice": { "id": 2 } })), json(named_hits(&["s"], pit()))).await;
    on(
        &server,
        body_partial_json(json!({ "slice": { "id": 3 } })),
        ResponseTemplate::new(500).set_body_json(json!({ "message": "shard failure" })),
    )
    .await;
    otherwise(&server, json(json!({}))).await;
    let client = client(&server);

    let outcomes = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| {
            Box::pin(async move {
                let mut parts = scope.partitioned(4).expect("four is within the maximum");
                join_all(parts.iter_mut().map(drain)).await
            })
        })
        .await
        .expect("closed");

    let [(a, a_error), (b, b_error), (stalled, stalled_error), (failed, failed_error)] =
        <[_; 4]>::try_from(outcomes).expect("four outcomes");
    assert_eq!((names(&a), a_error), (vec!["a1", "a2"], None));
    assert_eq!((names(&b), b_error), (vec!["b1"], None));
    assert_eq!(names(&stalled), ["s", "s"]);
    assert!(matches!(stalled_error, Some(PagerError::RepeatedContinuation { .. })), "{stalled_error:?}");
    assert!(failed.is_empty());
    let Some(PagerError::Api { meta, .. }) = &failed_error else { panic!("{failed_error:?}") };
    assert_eq!(meta.status.as_u16(), 500);

    let cursors = |slice: u64, pages: &[(u64, Value)]| -> Vec<Option<Value>> {
        pages.iter().filter(|(id, _)| *id == slice).map(|(_, body)| body.get("search_after").cloned()).collect()
    };
    let pages = page_bodies(&server).await;
    assert_eq!(cursors(0, &pages), [None, Some(json!(["a1"])), Some(json!(["a2"]))]);
    assert_eq!(cursors(1, &pages), [None, Some(json!(["b1"]))]);
    assert_eq!(cursors(2, &pages), [None, Some(json!(["s"]))], "the repeated cursor is never sent again");
    assert_eq!(cursors(3, &pages), [None], "a failed partition stays failed");

    let requests = received(&server).await;
    assert_eq!(requests.iter().filter(|request| request.url.path() == "/logs/_pit").count(), 1);
    let closes: Vec<&Request> = requests.iter().filter(|request| request.method.as_str() == "DELETE").collect();
    assert_eq!(closes.len(), 1);
    assert_eq!(body(closes[0]), json!({ "id": "p1" }));
}

#[tokio::test]
async fn a_partition_count_of_zero_or_above_the_maximum_is_refused_before_anything_is_sent() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({}))).await;
    let client = client(&server);

    let (zero, above) = client
        .send(PitSearch::get())
        .pager()
        .with_session(|scope| Box::pin(async move { (reason(scope.partitioned(0)), reason(scope.partitioned(5))) }))
        .await
        .expect("nothing to close");
    assert!(zero.contains("at least 1"), "{zero}");
    assert!(above.contains("maximum, 4"), "{above}");

    let zero = reason(client.send(SlicedList::get()).pager().partitioned(0));
    let above = reason(client.send(SlicedList::get()).pager().partitioned(4));
    assert!(zero.contains("at least 1"), "{zero}");
    assert!(above.contains("maximum, 3"), "{above}");

    assert!(received(&server).await.is_empty(), "no open, page, or close was sent");
}

// ---------------------------------------------------------------------------
// Sessionless partitions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_sessionless_partition_sends_its_index_and_count_on_every_request_and_keeps_its_own_history() {
    let server = MockServer::start().await;
    on(&server, query_param("after", "p0-2"), json(json!({ "data": [{ "id": 3 }] }))).await;
    on(&server, query_param("after", "p1-2"), json(json!({ "data": [{ "id": 11 }], "next": "p1-3" }))).await;
    on(&server, query_param("after", "p1-3"), json(json!({ "data": [] }))).await;
    on(&server, query_param("slice", "0"), json(json!({ "data": [{ "id": 1 }, { "id": 2 }], "next": "p0-2" }))).await;
    on(&server, query_param("slice", "1"), json(json!({ "data": [{ "id": 10 }], "next": "p1-2" }))).await;
    otherwise(&server, ResponseTemplate::new(404)).await;
    let client = client(&server);

    let mut parts = client.send(SlicedList::get()).pager().partitioned(2).expect("two is within the maximum");
    let outcomes = join_all(parts.iter_mut().map(drain)).await;

    let [(first, first_error), (second, second_error)] = <[_; 2]>::try_from(outcomes).expect("two outcomes");
    assert_eq!((ids(&first), first_error), (vec![1, 2, 3], None));
    assert_eq!((ids(&second), second_error), (vec![10, 11], None));

    let requests = received(&server).await;
    assert_eq!(requests.len(), 5);
    let value = |pairs: &[(String, String)], name: &str| {
        pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone())
    };
    for slice in ["0", "1"] {
        let sent: Vec<Vec<(String, String)>> = requests
            .iter()
            .map(query)
            .filter(|pairs| value(pairs, "slice").as_deref() == Some(slice))
            .collect();
        for pairs in &sent {
            assert_eq!(value(pairs, "slices").as_deref(), Some("2"), "slice {slice}: {pairs:?}");
        }
        let afters: Vec<Option<String>> = sent.iter().map(|pairs| value(pairs, "after")).collect();
        let expected: Vec<Option<String>> = match slice {
            "0" => vec![None, Some("p0-2".into())],
            _ => vec![None, Some("p1-2".into()), Some("p1-3".into())],
        };
        assert_eq!(afters, expected, "slice {slice}");
    }
}

// ---------------------------------------------------------------------------
// Point in time over a scripted raw server
// ---------------------------------------------------------------------------

/// One request the scripted server received.
#[derive(Debug, Clone)]
struct Arrival {
    /// The script's name for it, such as `p0.1` (slice 0, page 1).
    label: String,
    /// `METHOD /path?query`.
    target: String,
    body: Value,
}

/// The scripted server's answer to one request: hold it until a request
/// labelled `after` has arrived, then reply 200 with `body`.
struct Reply {
    label: String,
    after: Option<&'static str>,
    body: Value,
}

/// A loopback HTTP server that answers each request (one per connection)
/// as `script` says, recording arrivals in order.
struct ScriptedServer {
    url: String,
    arrivals: watch::Receiver<Vec<Arrival>>,
    _accept: AbortOnDrop,
}

impl ScriptedServer {
    async fn start(script: impl Fn(&str, &str, &Value) -> Reply + Send + Sync + 'static) -> Self {
        let (listener, url) = listener().await;
        let (log, arrivals) = watch::channel(Vec::<Arrival>::new());
        let log = Arc::new(log);
        let script = Arc::new(script);
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let (log, script) = (Arc::clone(&log), Arc::clone(&script));
                tokio::spawn(async move {
                    let (method, target, body) = read_request(&mut stream).await;
                    let reply = script(&method, target.split('?').next().unwrap_or_default(), &body);
                    log.send_modify(|arrivals| {
                        arrivals.push(Arrival {
                            label: reply.label.clone(),
                            target: format!("{method} {target}"),
                            body,
                        });
                    });
                    if let Some(after) = reply.after {
                        let mut seen = log.subscribe();
                        let _ = seen.wait_for(|arrivals| arrivals.iter().any(|arrival| arrival.label == after)).await;
                    }
                    let text = reply.body.to_string();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                         Connection: close\r\n\r\n{text}",
                        text.len()
                    );
                    write_all(&mut stream, response.as_bytes()).await;
                });
            }
        });
        Self {
            url,
            arrivals,
            _accept: AbortOnDrop(task),
        }
    }

    /// A receiver that waits for arrivals.
    fn watch(&self) -> watch::Receiver<Vec<Arrival>> {
        self.arrivals.clone()
    }

    fn arrivals(&self) -> Vec<Arrival> {
        self.arrivals.borrow().clone()
    }
}

/// Reads one request: its method, its target, and its JSON body (`Null`
/// when it has none).
async fn read_request(stream: &mut TcpStream) -> (String, String, Value) {
    let head = read_head(stream).await;
    let mut words = head.split_whitespace();
    let method = words.next().unwrap_or_default().to_owned();
    let target = words.next().unwrap_or_default().to_owned();
    let length = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut bytes = vec![0_u8; length];
    let _ = stream.read_exact(&mut bytes).await;
    (method, target, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// `p{slice}.{page}` for a page request, numbered by the cursor it sends,
/// `open` and `close` for the session steps.
fn pit_label(method: &str, path: &str, body: &Value, page_of: impl Fn(&Value) -> u32) -> String {
    match (method, path) {
        ("POST", "/logs/_pit") => "open".to_owned(),
        ("DELETE", _) => "close".to_owned(),
        _ => format!("p{}.{}", slice_id(body).unwrap_or(u64::MAX), page_of(body)),
    }
}

/// Generous bound for a scripted exchange: a pager that serialized the
/// partitions would wait on a held response forever.
const EXCHANGE_LIMIT: Duration = Duration::from_secs(3);

#[tokio::test]
async fn every_request_after_a_late_arriving_response_carries_that_latest_observed_handle_and_so_does_the_close() {
    // Partition 0's first page is requested first but answered last: its
    // response (handle "late") is held until partition 1 has received its
    // first page (handle "early") and requested its second. Partition 1's
    // second page is in turn held until partition 0 has requested its own
    // second page, so partition 1's third request follows "late".
    let server = ScriptedServer::start(|method, path, body| {
        let page = |body: &Value| match body.get("search_after").and_then(|after| after[0].as_str()) {
            None => 1,
            Some("a" | "b") => 2,
            Some(_) => 3,
        };
        let label = pit_label(method, path, body, page);
        let (after, body) = match label.as_str() {
            "open" => (None, json!({ "id": "p0" })),
            "p0.1" => (Some("p1.2"), named_hits(&["a"], json!({ "pit_id": "late" }))),
            "p1.1" => (None, named_hits(&["b"], json!({ "pit_id": "early" }))),
            "p1.2" => (Some("p0.2"), named_hits(&["c"], json!({}))),
            _ => (None, named_hits(&[], json!({}))),
        };
        Reply { label, after, body }
    })
    .await;
    let client = plain_client(&server.url);
    let mut seen = server.watch();

    let traversal = client.send(PitSearch::get()).pager().with_session(move |scope| {
        Box::pin(async move {
            let mut parts = scope.partitioned(2).expect("two is within the maximum");
            let [first, second] = &mut parts[..] else { unreachable!("two partitions") };
            let started_second = async {
                // Partition 1 starts only once partition 0's first request
                // has reached the server.
                let _ = seen.wait_for(|arrivals| arrivals.iter().any(|arrival| arrival.label == "p0.1")).await;
                drain(second).await
            };
            tokio::join!(drain(first), started_second)
        })
    });
    let ((first, first_error), (second, second_error)) =
        tokio::time::timeout(EXCHANGE_LIMIT, traversal).await.expect("the exchange completes").expect("closed");

    assert_eq!((names(&first), first_error), (vec!["a"], None));
    assert_eq!((names(&second), second_error), (vec!["b", "c"], None));
    let pit = |id: &str| json!({ "id": id, "keep_alive": "1m" });
    let slice = |id: u32| json!({ "id": id, "max": 2 });
    let sent: Vec<(String, String, Value)> =
        server.arrivals().into_iter().map(|arrival| (arrival.label, arrival.target, arrival.body)).collect();
    assert_eq!(
        sent,
        [
            ("open".into(), "POST /logs/_pit?keep_alive=1m".into(), Value::Null),
            ("p0.1".into(), "POST /_search".into(), json!({ "pit": pit("p0"), "slice": slice(0) })),
            ("p1.1".into(), "POST /_search".into(), json!({ "pit": pit("p0"), "slice": slice(1) })),
            (
                "p1.2".into(),
                "POST /_search".into(),
                json!({ "pit": pit("early"), "slice": slice(1), "search_after": ["b"] })
            ),
            (
                "p0.2".into(),
                "POST /_search".into(),
                json!({ "pit": pit("late"), "slice": slice(0), "search_after": ["a"] })
            ),
            (
                "p1.3".into(),
                "POST /_search".into(),
                json!({ "pit": pit("late"), "slice": slice(1), "search_after": ["c"] })
            ),
            ("close".into(), "DELETE /_pit".into(), json!({ "id": "late" })),
        ]
    );
}

#[tokio::test]
async fn two_partitions_have_their_first_page_requests_in_flight_at_the_same_time() {
    // Each first page is answered only after the other has arrived, so the
    // traversal completes only if neither request waits for the other's
    // response.
    let server = ScriptedServer::start(|method, path, body| {
        let label = pit_label(method, path, body, |body| if body.get("search_after").is_some() { 2 } else { 1 });
        let (after, body) = match label.as_str() {
            "open" => (None, json!({ "id": "p0" })),
            "p0.1" => (Some("p1.1"), named_hits(&[], json!({ "pit_id": "p1" }))),
            "p1.1" => (Some("p0.1"), named_hits(&[], json!({ "pit_id": "p1" }))),
            _ => (None, json!({})),
        };
        Reply { label, after, body }
    })
    .await;
    let client = plain_client(&server.url);

    let traversal = client.send(PitSearch::get()).pager().with_session(|scope| {
        Box::pin(async move {
            let mut parts = scope.partitioned(2).expect("two is within the maximum");
            join_all(parts.iter_mut().map(drain)).await
        })
    });
    let outcomes =
        tokio::time::timeout(EXCHANGE_LIMIT, traversal).await.expect("both requests were in flight").expect("closed");

    for (items, error) in &outcomes {
        assert!(items.is_empty() && error.is_none(), "{items:?} {error:?}");
    }
    let labels: Vec<String> = server.arrivals().into_iter().map(|arrival| arrival.label).collect();
    assert_eq!(labels.first().map(String::as_str), Some("open"));
    assert_eq!(labels.last().map(String::as_str), Some("close"));
    assert_eq!(labels.len(), 4, "{labels:?}");
}
