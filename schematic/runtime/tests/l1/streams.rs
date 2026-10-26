//! Streaming codecs (R6–R11c): SSE and NDJSON framing at awkward byte
//! boundaries, the four end conditions with consumed and kept terminators,
//! exactly one terminal error, malformed-item recovery, the frame limit, and
//! provider error events as typed items, read through `Decode::decode` on a
//! built `LiveBody`; then incremental delivery, the idle bound and shared
//! deadline, ownership, and drop over real sockets through the managed send.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;
use futures_util::stream;
use http::{HeaderMap, StatusCode};
use schematic_runtime::{
    ApiDefinition, BodyReadError, BuildError, ByteStream, Client, Deadline, Decode, EndCondition, ErrorKind,
    ItemStream, LiveBody, Ndjson, NoDeclaredError, Outcome, Response, Scalar, Sse, StreamEnd, StreamError,
    TransportErrorKind, UntilClose,
};
use serde::Deserialize;
use tokio::io::AsyncReadExt;
use tokio::sync::oneshot;
use tokio::time::Instant;

use crate::support::{Call, TestApi, builder, read_head, serve, write_all};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event {
    ContentBlockDelta { text: String },
    MessageStop,
    Error { error: Fault },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Fault {
    message: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Chunk {
    response: String,
    done: bool,
    eval_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Progress {
    status: String,
}

/// OpenAI's `[DONE]` data line, consumed.
struct Done;
impl StreamEnd for Done {
    const CONDITION: EndCondition = EndCondition::DataLineEquals("[DONE]");
}

/// Anthropic's `message_stop` event, consumed.
struct MessageStop;
impl StreamEnd for MessageStop {
    const CONDITION: EndCondition = EndCondition::EventNamed("message_stop");
}

/// The `message_stop` event, delivered as the last item.
struct MessageStopKept;
impl StreamEnd for MessageStopKept {
    const CONDITION: EndCondition = EndCondition::EventNamed("message_stop");
    const KEEP: bool = true;
}

/// Ollama's `done: true` line, delivered as the last item with its
/// statistics.
struct OllamaDone;
impl StreamEnd for OllamaDone {
    const CONDITION: EndCondition = EndCondition::FieldEquals {
        path: &["done"],
        value: Scalar::Bool(true),
    };
    const KEEP: bool = true;
}

/// A nested field terminator, consumed.
struct NestedSuccess;
impl StreamEnd for NestedSuccess {
    const CONDITION: EndCondition = EndCondition::FieldEquals {
        path: &["meta", "status"],
        value: Scalar::String("success"),
    };
}

fn live(chunks: Vec<Result<Bytes, BodyReadError>>) -> LiveBody {
    LiveBody::new(StatusCode::OK, HeaderMap::new(), stream::iter(chunks), None)
}

fn pieces(parts: &[&[u8]]) -> Vec<Result<Bytes, BodyReadError>> {
    parts.iter().map(|part| Ok(Bytes::copy_from_slice(part))).collect()
}

async fn drain<T>(mut items: ItemStream<T>) -> Vec<Result<T, StreamError>> {
    let mut all = Vec::new();
    while let Some(item) = items.next().await {
        all.push(item);
    }
    assert!(items.next().await.is_none(), "an ended stream stays ended");
    all
}

async fn sse<T, E>(chunks: Vec<Result<Bytes, BodyReadError>>) -> Vec<Result<T, StreamError>>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    E: StreamEnd,
{
    drain(<Sse<T, E> as Decode>::decode(live(chunks)).await.expect("a stream never fails to start")).await
}

async fn ndjson<T, E>(chunks: Vec<Result<Bytes, BodyReadError>>) -> Vec<Result<T, StreamError>>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    E: StreamEnd,
{
    drain(<Ndjson<T, E> as Decode>::decode(live(chunks)).await.expect("a stream never fails to start")).await
}

fn delta(text: &str) -> Result<Event, StreamError> {
    Ok(Event::ContentBlockDelta { text: text.to_string() })
}

/// A byte order mark, a comment, CRLF and CR line endings, an `id:` field,
/// a two-line `data:` event, multi-byte UTF-8, and the consumed terminator
/// followed by bytes that must never be read as items.
const ANTHROPIC_STYLE: &[u8] = "\u{FEFF}: keep-alive\r\n\
event: content_block_delta\r\nid: 1\r\ndata: {\"type\":\"content_block_delta\",\r\ndata: \"text\":\"h\u{e9}llo\"}\r\n\r\n\
event: content_block_delta\rdata:{\"type\":\"content_block_delta\",\"text\":\"w\u{f6}rld \u{1F600}\"}\r\r\
event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n\
data: not json after the end\n\n"
    .as_bytes();

#[tokio::test]
async fn sse_events_decode_the_same_whatever_the_chunk_boundaries() {
    let expected = vec![delta("h\u{e9}llo"), delta("w\u{f6}rld \u{1F600}")];
    assert_eq!(sse::<Event, MessageStop>(pieces(&[ANTHROPIC_STYLE])).await, expected, "one chunk");
    for split in 1..ANTHROPIC_STYLE.len() {
        let (head, tail) = ANTHROPIC_STYLE.split_at(split);
        assert_eq!(sse::<Event, MessageStop>(pieces(&[head, tail])).await, expected, "split at byte {split}");
    }
    let bytes: Vec<&[u8]> = ANTHROPIC_STYLE.chunks(1).collect();
    assert_eq!(sse::<Event, MessageStop>(pieces(&bytes)).await, expected, "one byte per chunk");
}

#[tokio::test]
async fn ndjson_lines_decode_the_same_whatever_the_chunk_boundaries() {
    let body: &[u8] = b"{\"response\":\"a\",\"done\":false}\r\n\n  \r\n{\"response\":\"\xC3\xA9\",\"done\":false}\n{\"response\":\"\",\"done\":true,\"eval_count\":42}";
    let expected = vec![
        Ok(Chunk { response: "a".into(), done: false, eval_count: None }),
        Ok(Chunk { response: "\u{e9}".into(), done: false, eval_count: None }),
        Ok(Chunk { response: String::new(), done: true, eval_count: Some(42) }),
    ];
    for split in 1..body.len() {
        let (head, tail) = body.split_at(split);
        assert_eq!(
            ndjson::<Chunk, UntilClose>(pieces(&[head, tail])).await,
            expected,
            "split at byte {split}: blank lines skipped, CRLF accepted, final line without newline decoded"
        );
    }
}

#[tokio::test]
async fn each_end_condition_finishes_cleanly_consuming_or_keeping_its_terminator() {
    // Data line, consumed: the last item is the one before `[DONE]`.
    let done = b"data: {\"type\":\"content_block_delta\",\"text\":\"a\"}\n\ndata: [DONE]\n\ndata: {\"type\":\"message_stop\"}\n\n";
    assert_eq!(sse::<Event, Done>(pieces(&[done])).await, vec![delta("a")]);

    // Event name, consumed and kept.
    let stop = b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"text\":\"a\"}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    assert_eq!(sse::<Event, MessageStop>(pieces(&[stop])).await, vec![delta("a")]);
    assert_eq!(sse::<Event, MessageStopKept>(pieces(&[stop])).await, vec![delta("a"), Ok(Event::MessageStop)]);

    // Field, kept: Ollama's final line carries its statistics.
    let ollama = b"{\"response\":\"Hi\",\"done\":false}\n{\"response\":\"\",\"done\":true,\"eval_count\":7}\n{\"response\":\"late\",\"done\":false}\n";
    let items = ndjson::<Chunk, OllamaDone>(pieces(&[ollama])).await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items.last(), Some(&Ok(Chunk { response: String::new(), done: true, eval_count: Some(7) })));

    // Nested field, consumed; a missing field never matches.
    let nested = b"{\"status\":\"pulling\"}\n{\"status\":\"x\",\"meta\":{}}\n{\"status\":\"done\",\"meta\":{\"status\":\"success\"}}\n";
    assert_eq!(
        ndjson::<Progress, NestedSuccess>(pieces(&[nested])).await,
        vec![Ok(Progress { status: "pulling".into() }), Ok(Progress { status: "x".into() })]
    );

    // Connection close: the body's end is the clean end.
    assert_eq!(ndjson::<Progress, UntilClose>(pieces(&[b"{\"status\":\"a\"}\n"])).await, vec![Ok(Progress { status: "a".into() })]);
}

#[tokio::test]
async fn a_body_that_ends_before_its_terminator_reports_ended_early_once() {
    let early = b"data: {\"type\":\"content_block_delta\",\"text\":\"a\"}\n\n";
    assert_eq!(sse::<Event, Done>(pieces(&[early])).await, vec![delta("a"), Err(StreamError::EndedEarly)]);
    assert_eq!(sse::<Event, UntilClose>(pieces(&[early])).await, vec![delta("a")], "connection close never ends early");

    // A truncated last line is malformed, then the missing terminator is reported.
    let truncated = b"{\"response\":\"a\",\"done\":false}\n{\"response\":\"b\",\"do";
    assert_eq!(
        ndjson::<Chunk, OllamaDone>(pieces(&[truncated])).await,
        vec![
            Ok(Chunk { response: "a".into(), done: false, eval_count: None }),
            Err(StreamError::Malformed { raw: Bytes::from_static(b"{\"response\":\"b\",\"do") }),
            Err(StreamError::EndedEarly),
        ]
    );

    // An SSE event the body cuts off before its blank line is discarded.
    let cut = b"data: {\"type\":\"content_block_delta\",\"text\":\"a\"}\n\ndata: [DONE]\n";
    assert_eq!(sse::<Event, Done>(pieces(&[cut])).await, vec![delta("a"), Err(StreamError::EndedEarly)]);
}

#[tokio::test]
async fn a_malformed_item_keeps_its_bytes_and_the_stream_continues() {
    let body = b"data: {\"type\":\"unknown_event\"}\n\ndata: {nope\ndata: still}\n\ndata: {\"type\":\"content_block_delta\",\"text\":\"b\"}\n\ndata: [DONE]\n\n";
    assert_eq!(
        sse::<Event, Done>(pieces(&[body])).await,
        vec![
            Err(StreamError::Malformed { raw: Bytes::from_static(b"{\"type\":\"unknown_event\"}") }),
            Err(StreamError::Malformed { raw: Bytes::from_static(b"{nope\nstill}") }),
            delta("b"),
        ],
        "`[DONE]` is matched before decoding, so it is not malformed"
    );

    // A malformed kept terminator is the one error, then the end.
    let kept = b"{\"response\":\"a\",\"done\":false}\n{\"response\":1,\"done\":true}\n{\"response\":\"late\",\"done\":false}\n";
    let items = ndjson::<Chunk, OllamaDone>(pieces(&[kept])).await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[1], Err(StreamError::Malformed { raw: Bytes::from_static(b"{\"response\":1,\"done\":true}") }));
    assert!(!items[1].as_ref().expect_err("malformed").is_terminal());
}

#[tokio::test]
async fn a_provider_error_event_is_a_typed_item_not_a_stream_error() {
    let body = b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"text\":\"a\"}\n\nevent: error\ndata: {\"type\":\"error\",\"error\":{\"message\":\"Overloaded\"}}\n\n";
    assert_eq!(
        sse::<Event, MessageStop>(pieces(&[body])).await,
        vec![
            delta("a"),
            Ok(Event::Error { error: Fault { message: "Overloaded".into() } }),
            Err(StreamError::EndedEarly),
        ],
        "the error event decodes as the item type; the close that follows takes the end rule"
    );
}

#[tokio::test]
async fn a_transport_break_is_interrupted_with_the_pending_frame_and_never_ended_early() {
    let chunks = vec![
        Ok(Bytes::from_static(b"data: {\"type\":\"content_block_delta\",\"text\":\"a\"}\n\ndata: {\"type\":")),
        Err(BodyReadError::Interrupted),
    ];
    assert_eq!(
        sse::<Event, Done>(chunks).await,
        vec![delta("a"), Err(StreamError::Interrupted { pending: Bytes::from_static(b"data: {\"type\":") })]
    );

    let chunks = vec![Ok(Bytes::from_static(b"{\"status\":\"a\"}\n{\"sta")), Err(BodyReadError::Interrupted)];
    assert_eq!(
        ndjson::<Progress, UntilClose>(chunks).await,
        vec![Ok(Progress { status: "a".into() }), Err(StreamError::Interrupted { pending: Bytes::from_static(b"{\"sta") })]
    );
}

#[tokio::test]
async fn a_frame_over_the_limit_ends_the_stream_with_a_bounded_prefix() {
    let long = "x".repeat(4096);
    let unterminated = format!("data: {{\"type\":\"content_block_delta\",\"text\":\"a\"}}\n\ndata: {long}");
    let items = drain(
        <Sse<Event, Done> as Decode>::decode(live(pieces(&[unterminated.as_bytes()])).with_max_frame_bytes(2048))
            .await
            .expect("stream"),
    )
    .await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0], delta("a"), "frames before the oversized one are delivered");
    let Err(StreamError::FrameLimitExceeded { limit, prefix }) = &items[1] else {
        panic!("expected a limit error, got {:?}", items[1]);
    };
    assert_eq!(*limit, 2048);
    assert_eq!(prefix.len(), schematic_runtime::stream::FRAME_LIMIT_PREFIX_BYTES);
    assert!(prefix.starts_with(b"data: xxx"), "the pending frame's first bytes");

    // A complete NDJSON line over the limit, and a pending one without its newline.
    for body in [format!("{{\"status\":\"{long}\"}}\n"), format!("{{\"status\":\"{long}")] {
        let items = drain(
            <Ndjson<Progress, UntilClose> as Decode>::decode(live(pieces(&[body.as_bytes()])).with_max_frame_bytes(1024))
                .await
                .expect("stream"),
        )
        .await;
        assert!(matches!(items.as_slice(), [Err(StreamError::FrameLimitExceeded { limit: 1024, .. })]), "{items:?}");
    }

    // A byte stream assembles no frame, so the limit does not apply.
    let mut bytes = <ByteStream as Decode>::decode(live(pieces(&[long.as_bytes()])).with_max_frame_bytes(1))
        .await
        .expect("stream");
    assert_eq!(bytes.next().await, Some(Ok(Bytes::from(long.clone()))));
    assert_eq!(bytes.next().await, None);
}

#[tokio::test]
async fn a_byte_stream_yields_chunks_in_order_and_reports_a_break() {
    let chunks = vec![Ok(Bytes::from_static(b"ID3")), Ok(Bytes::new()), Ok(Bytes::from_static(b"\x00\xff")), Err(BodyReadError::Interrupted)];
    let mut bytes = <ByteStream as Decode>::decode(live(chunks)).await.expect("stream");
    assert_eq!(bytes.next().await, Some(Ok(Bytes::from_static(b"ID3"))));
    assert_eq!(bytes.next().await, Some(Ok(Bytes::from_static(b"\x00\xff"))), "empty chunks are skipped");
    assert_eq!(bytes.next().await, Some(Err(StreamError::Interrupted { pending: Bytes::new() })));
    assert_eq!(bytes.next().await, None);
}

#[test]
fn streams_are_owned_and_send() {
    fn owned_send<T: Send + 'static>() {}
    owned_send::<ItemStream<Event>>();
    owned_send::<ByteStream>();
    owned_send::<StreamError>();
}

// Over real sockets, through the managed send.

const SSE_HEAD: &str =
    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";

fn chunk(data: &str) -> String {
    format!("{:x}\r\n{data}\r\n", data.len())
}

fn event(text: &str) -> String {
    chunk(&format!("data: {{\"type\":\"content_block_delta\",\"text\":\"{text}\"}}\n\n"))
}

fn client(base_url: &str, timeout: Duration) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), base_url)
        .timeout(timeout)
        .build()
        .expect("client builds")
}

fn events(response: Response<ItemStream<Event>, NoDeclaredError>) -> ItemStream<Event> {
    match response.outcome {
        Outcome::Success(items) => items,
        Outcome::Failure(error) => panic!("expected a stream, got {error}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_envelope_and_first_item_arrive_before_the_server_finishes() {
    let (release, released) = oneshot::channel::<()>();
    let released = std::sync::Mutex::new(Some(released));
    let (url, _server) = serve(move |mut stream| {
        let released = released.lock().expect("lock").take();
        async move {
            read_head(&mut stream).await;
            write_all(&mut stream, format!("{SSE_HEAD}{}", event("first")).as_bytes()).await;
            if let Some(released) = released {
                let _ = released.await;
            }
            write_all(&mut stream, format!("{}{}0\r\n\r\n", event("second"), chunk("data: [DONE]\n\n")).as_bytes()).await;
        }
    })
    .await;

    let response = client(&url, Duration::from_secs(5))
        .send(Call::<Sse<Event, Done>>::get("/stream"))
        .full()
        .await
        .expect("head");
    assert_eq!(response.meta.status, 200, "the envelope exists while the server holds the body open");
    assert!(response.meta.elapsed < Duration::from_secs(5), "elapsed is the time to the head: {:?}", response.meta.elapsed);
    let mut items = events(response);
    assert_eq!(items.next().await, Some(delta("first")), "the first item arrives before the rest is sent");
    release.send(()).expect("the server is waiting");
    assert_eq!(items.next().await, Some(delta("second")));
    assert_eq!(items.next().await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stream_that_delivers_every_second_outlives_a_two_second_timeout() {
    let (url, _server) = serve(|mut stream| async move {
        read_head(&mut stream).await;
        write_all(&mut stream, format!("{SSE_HEAD}{}", event("0")).as_bytes()).await;
        for second in 1..=3 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            write_all(&mut stream, event(&second.to_string()).as_bytes()).await;
        }
        write_all(&mut stream, format!("{}0\r\n\r\n", chunk("data: [DONE]\n\n")).as_bytes()).await;
    })
    .await;

    let started = Instant::now();
    let response = client(&url, Duration::from_secs(2))
        .send(Call::<Sse<Event, Done>>::get("/stream"))
        .full()
        .await
        .expect("head");
    let items = drain(events(response)).await;
    assert_eq!(items, vec![delta("0"), delta("1"), delta("2"), delta("3")]);
    assert!(started.elapsed() > Duration::from_secs(2), "the stream ran past the per-request timeout: {:?}", started.elapsed());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_silent_stream_and_one_that_only_drips_partial_bytes_both_time_out_idle() {
    let timeout = Duration::from_millis(300);
    for drip in [false, true] {
        let (url, _server) = serve(move |mut stream| async move {
            read_head(&mut stream).await;
            write_all(&mut stream, format!("{SSE_HEAD}{}", event("a")).as_bytes()).await;
            if drip {
                // Bytes keep arriving, but never a complete event.
                for _ in 0..40 {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    write_all(&mut stream, chunk("data: x").as_bytes()).await;
                }
            }
            std::future::pending::<()>().await;
        })
        .await;

        let response = client(&url, timeout)
            .send(Call::<Sse<Event, Done>>::get("/stream"))
            .full()
            .await
            .expect("head");
        let mut items = events(response);
        assert_eq!(items.next().await, Some(delta("a")));
        let idle = Instant::now();
        assert_eq!(items.next().await, Some(Err(StreamError::IdleTimeout)), "drip {drip}");
        assert!(idle.elapsed() >= timeout - Duration::from_millis(20), "drip {drip}: {:?}", idle.elapsed());
        assert!(idle.elapsed() < Duration::from_millis(1500), "partial bytes do not extend the idle bound: {:?}", idle.elapsed());
        assert_eq!(items.next().await, None, "exactly one terminal error");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_shared_deadline_ends_a_healthy_stream_with_its_own_error() {
    let (url, _server) = serve(|mut stream| async move {
        read_head(&mut stream).await;
        write_all(&mut stream, SSE_HEAD.as_bytes()).await;
        for n in 0..100 {
            write_all(&mut stream, event(&n.to_string()).as_bytes()).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    let deadline = Deadline::after(Duration::from_millis(400));
    let response = client(&url, Duration::from_secs(10))
        .send(Call::<Sse<Event, Done>>::get("/stream"))
        .deadline(&deadline)
        .full()
        .await
        .expect("head");
    let items = drain(events(response)).await;
    assert!(items.len() >= 2, "items arrived before the deadline: {items:?}");
    assert_eq!(items.last(), Some(&Err(StreamError::DeadlineExceeded)));
    assert!(items[..items.len() - 1].iter().all(Result::is_ok), "{items:?}");
    assert!(deadline.elapsed() >= Duration::from_millis(400), "{:?}", deadline.elapsed());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stream_outlives_its_client_and_dropping_it_releases_the_connection() {
    let (closed, closed_seen) = oneshot::channel::<()>();
    let closed = std::sync::Mutex::new(Some(closed));
    let (url, _server) = serve(move |mut stream| {
        let closed = closed.lock().expect("lock").take();
        async move {
            read_head(&mut stream).await;
            write_all(&mut stream, format!("{SSE_HEAD}{}", event("a")).as_bytes()).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            write_all(&mut stream, event("b").as_bytes()).await;
            // Wait for the client side to go away.
            let mut buffer = [0_u8; 64];
            while matches!(stream.read(&mut buffer).await, Ok(read) if read > 0) {}
            if let Some(closed) = closed {
                let _ = closed.send(());
            }
        }
    })
    .await;

    let client = client(&url, Duration::from_secs(5));
    let response = client.send(Call::<Sse<Event, Done>>::get("/stream")).full().await.expect("head");
    drop(client);
    let mut items = events(response);
    assert_eq!(items.next().await, Some(delta("a")));
    assert_eq!(items.next().await, Some(delta("b")), "the stream reads on after its client is dropped");
    drop(items);
    tokio::time::timeout(Duration::from_secs(2), closed_seen)
        .await
        .expect("dropping the stream closed its connection")
        .expect("server task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_frame_limit_bounds_an_unterminated_event_and_zero_is_refused() {
    let (url, _server) = serve(|mut stream| async move {
        read_head(&mut stream).await;
        let long = "y".repeat(300);
        write_all(&mut stream, format!("{SSE_HEAD}{}{}", event("a"), chunk(&format!("data: {long}"))).as_bytes()).await;
        std::future::pending::<()>().await;
    })
    .await;
    let response = builder(ApiDefinition::new("https://api.example.invalid"), &url)
        .max_frame_bytes(64)
        .send(Call::<Sse<Event, Done>>::get("/stream"))
        .full()
        .await
        .expect("head");
    let items = drain(events(response)).await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert!(matches!(&items[1], Err(StreamError::FrameLimitExceeded { limit: 64, prefix }) if prefix.len() <= 1024));

    let refused = builder(ApiDefinition::new("https://api.example.invalid"), &url).max_frame_bytes(0);
    assert_eq!(refused.clone().build().map(drop), Err(BuildError::InvalidMaxFrameBytes));
    let error = refused.send(Call::<Sse<Event, Done>>::get("/stream")).full().await.expect_err("no send");
    assert_eq!(error.kind, TransportErrorKind::InvalidClient(BuildError::InvalidMaxFrameBytes));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_error_status_on_a_stream_endpoint_takes_the_error_path_and_skips_the_global_hook() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (url, _server) = serve(|mut stream| async move {
        let head = read_head(&mut stream).await;
        let reply = if head.starts_with("GET /fail") {
            let body = "data: {\"busy\":1}\n\n\n";
            format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
        } else {
            format!("{SSE_HEAD}{}{}0\r\n\r\n", event("a"), chunk("data: [DONE]\n\n"))
        };
        write_all(&mut stream, reply.as_bytes()).await;
    })
    .await;
    let counted = Arc::clone(&calls);
    let client = client(&url, Duration::from_secs(5)).pre_response_json(move |_, value| {
        counted.fetch_add(1, Ordering::SeqCst);
        Ok(value)
    });

    let failed = client.send(Call::<Sse<Event, Done>>::get("/fail")).full().await.expect("head");
    assert_eq!(failed.meta.status, 503);
    let error = failed.error().expect("a non-2xx reply is never a stream");
    assert_eq!(error.kind, ErrorKind::Undeclared);
    assert_eq!(error.body, Bytes::from_static(b"data: {\"busy\":1}\n\n\n"));

    let streamed = client.send(Call::<Sse<Event, Done>>::get("/ok")).full().await.expect("head");
    assert_eq!(drain(events(streamed)).await, vec![delta("a")]);
    assert_eq!(calls.load(Ordering::SeqCst), 0, "the global JSON hook never runs on a stream or a non-2xx reply");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_delayed_first_item_and_a_real_connection_break_mid_frame() {
    let (release, released) = oneshot::channel::<()>();
    let released = std::sync::Mutex::new(Some(released));
    let (url, _server) = serve(move |mut stream| {
        let released = released.lock().expect("lock").take();
        async move {
            read_head(&mut stream).await;
            write_all(&mut stream, SSE_HEAD.as_bytes()).await;
            if let Some(released) = released {
                let _ = released.await;
            }
            // One event, then half of the next chunk, then the socket drops.
            let partial = "data: {\"type\":";
            write_all(&mut stream, format!("{}{:x}\r\n{partial}", event("a"), partial.len() + 10).as_bytes()).await;
        }
    })
    .await;

    let response = client(&url, Duration::from_secs(5))
        .send(Call::<Sse<Event, Done>>::get("/stream"))
        .full()
        .await
        .expect("the envelope arrives with the head, before any item");
    let mut items = events(response);
    release.send(()).expect("the server is waiting");
    assert_eq!(items.next().await, Some(delta("a")));
    assert_eq!(items.next().await, Some(Err(StreamError::Interrupted { pending: Bytes::from_static(b"data: {\"type\":") })));
    assert_eq!(items.next().await, None, "a break is never followed by ended-early");
}
