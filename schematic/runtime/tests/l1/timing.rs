//! Timing over real sockets: a cut-off body stays in the envelope (R11), a
//! stalled head is a redacted timeout (R7b), the shared deadline bounds and
//! counts the sends of an operation and dropping a send cancels its work
//! (R26), and a streaming decoder outlives the single-send expiration under
//! an idle bound and the shared deadline (R30).

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;
use http::Method;
use schematic_runtime::redact::REDACTED;
use schematic_runtime::{
    ApiDefinition, BodyReadError, Client, Deadline, Decode, DecodeFailure, ErrorKind, Json, LiveBody,
    NoDeclaredError, Outcome, RequestParts, Response, TransportErrorKind,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::support::{AbortOnDrop, Call, Item, SENTINEL, TestApi, builder, closed_port_url, listener, read_head, write_all};

/// Serves each accepted connection with `handle`, one at a time, in accept
/// order.
async fn serve<F, Fut>(handle: F) -> (String, AbortOnDrop)
where
    F: Fn(TcpStream) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (listener, url) = listener().await;
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            handle(stream).await;
        }
    });
    (url, AbortOnDrop(task))
}

/// Reads the request head, then never answers.
async fn hang(mut stream: TcpStream) {
    read_head(&mut stream).await;
    std::future::pending::<()>().await;
}

fn reply(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Ten bytes of a body whose head promises a hundred.
const PARTIAL: &[u8] = br#"{"id":1234"#;

fn partial_head(status: &str) -> Vec<u8> {
    let mut bytes = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n").into_bytes();
    bytes.extend_from_slice(PARTIAL);
    bytes
}

/// Reads until the peer closes or resets the connection.
async fn read_until_closed(stream: &mut TcpStream) {
    let mut buffer = [0_u8; 256];
    while matches!(stream.read(&mut buffer).await, Ok(read) if read > 0) {}
}

fn client(base_url: &str, timeout: Duration) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), base_url)
        .timeout(timeout)
        .build()
        .expect("client builds")
}

fn item() -> Call<Json<Item>> {
    Call::get("/item")
}

/// A test-only streaming decoder: its output owns the live body under stream
/// bounds and yields each line as it arrives.
pub(crate) struct Lines;

pub(crate) struct LineStream {
    body: LiveBody,
    pending: Vec<u8>,
}

impl LineStream {
    pub(crate) async fn next_line(&mut self) -> Option<Result<String, BodyReadError>> {
        loop {
            if let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
                let line: Vec<u8> = self.pending.drain(..=end).collect();
                return Some(Ok(String::from_utf8_lossy(&line[..end]).into_owned()));
            }
            match self.body.chunk().await {
                Ok(Some(piece)) => self.pending.extend_from_slice(&piece),
                Ok(None) => return None,
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

impl Decode for Lines {
    type Output = LineStream;

    async fn decode(body: LiveBody) -> Result<LineStream, DecodeFailure> {
        Ok(LineStream {
            body: body.into_streaming(),
            pending: Vec::new(),
        })
    }
}

const STREAM_HEAD: &str =
    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";

fn chunk(data: &str) -> String {
    format!("{:x}\r\n{data}\r\n", data.len())
}

fn into_lines(response: Response<LineStream, NoDeclaredError>) -> LineStream {
    match response.outcome {
        Outcome::Success(lines) => lines,
        Outcome::Failure(error) => panic!("expected a stream, got {error}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_body_cut_off_after_the_head_is_interrupted_inside_the_envelope_with_its_bytes() {
    for status in ["200 OK", "500 Internal Server Error"] {
        let head = partial_head(status);
        let (url, _server) = serve(move |mut stream| {
            let head = head.clone();
            async move {
                read_head(&mut stream).await;
                write_all(&mut stream, &head).await;
            }
        })
        .await;

        let response = client(&url, Duration::from_secs(10))
            .send(item()).full()
            .await
            .expect("a head arrived, so an envelope");

        let code: u16 = status[..3].parse().expect("status code");
        assert_eq!(response.meta.status, code);
        assert_eq!(response.meta.headers["content-length"], "100");
        let error = response.error().expect("failure");
        assert_eq!(error.kind, ErrorKind::BodyInterrupted, "{status}");
        assert_eq!(error.body, Bytes::from_static(PARTIAL), "{status}");
        assert!(response.is_interrupted(), "{status}");
        assert!(!response.is_decode_failure(), "{status}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stalled_head_is_a_timeout_whose_text_redacts_the_query() {
    let (url, _server) = serve(hang).await;
    let mut parts = RequestParts::new(Method::GET, "/item");
    parts.query = vec![("api_key".into(), SENTINEL.into()), ("page".into(), "2".into())];

    let started = Instant::now();
    let error = client(&url, Duration::from_millis(200))
        .send(Call::<Json<Item>>::new(parts)).full()
        .await
        .expect_err("no head arrives");

    assert_eq!(error.kind, TransportErrorKind::Timeout);
    assert!(error.elapsed >= Duration::from_millis(200), "{:?}", error.elapsed);
    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
    for text in [error.to_string(), format!("{error:?}")] {
        assert!(!text.contains(SENTINEL), "{text}");
        assert!(text.contains(&format!("api_key={REDACTED}")), "{text}");
        assert!(text.contains("page=2"), "{text}");
    }
}

/// A mock that counts the connections it accepted and answers each request
/// after `delay`.
async fn counting_server(delay: Duration) -> (String, Arc<AtomicUsize>, AbortOnDrop) {
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&accepted);
    let (url, server) = serve(move |mut stream| {
        counter.fetch_add(1, Ordering::SeqCst);
        async move {
            read_head(&mut stream).await;
            tokio::time::sleep(delay).await;
            write_all(&mut stream, reply("200 OK", r#"{"id":1}"#).as_bytes()).await;
        }
    })
    .await;
    (url, accepted, server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_exhausted_deadline_sends_nothing() {
    let (url, accepted, _server) = counting_server(Duration::ZERO).await;
    let client = client(&url, Duration::from_secs(10));
    let deadline = Deadline::after(Duration::ZERO);

    let error = client
        .send(item()).deadline(&deadline).full()
        .await
        .expect_err("refused before sending");
    assert!(
        matches!(error.kind, TransportErrorKind::DeadlineExceeded { completed_requests: 0, .. }),
        "{:?}",
        error.kind
    );

    // The mock serves in accept order, so a control request answered first
    // proves the refused send never connected.
    let control = client.send(item()).full().await.expect("control request");
    assert_eq!(control.data(), Some(&Item { id: 1 }));
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    assert_eq!(deadline.completed_requests(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_per_request_timeout_under_a_longer_deadline_leaves_the_rest_of_the_budget() {
    let (hanging, _hanging_server) = serve(hang).await;
    let (fast, _accepted, _fast_server) = counting_server(Duration::ZERO).await;
    let deadline = Deadline::after(Duration::from_secs(2));

    let error = client(&hanging, Duration::from_millis(200))
        .send(item()).deadline(&deadline).full()
        .await
        .expect_err("no head arrives");
    assert_eq!(error.kind, TransportErrorKind::Timeout, "the request timeout fired, not the deadline");
    assert!(error.elapsed >= Duration::from_millis(200), "{:?}", error.elapsed);
    assert!(deadline.remaining() > Duration::from_secs(1), "{:?}", deadline.remaining());
    assert_eq!(deadline.completed_requests(), 0);

    let response = client(&fast, Duration::from_millis(200)).send(item()).deadline(&deadline).full().await.expect("head");
    assert_eq!(response.data(), Some(&Item { id: 1 }));
    assert_eq!(deadline.completed_requests(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deadline_shorter_than_the_timeout_bounds_the_request() {
    let (url, _server) = serve(hang).await;
    let deadline = Deadline::after(Duration::from_millis(300));

    let error = client(&url, Duration::from_secs(10))
        .send(item()).deadline(&deadline).full()
        .await
        .expect_err("no head arrives");

    match error.kind {
        TransportErrorKind::DeadlineExceeded {
            completed_requests,
            deadline_elapsed,
        } => {
            assert_eq!(completed_requests, 0);
            assert!(deadline_elapsed >= Duration::from_millis(300), "{deadline_elapsed:?}");
            assert!(deadline_elapsed < Duration::from_secs(5), "{deadline_elapsed:?}");
        }
        other => panic!("expected DeadlineExceeded, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deadline_stops_an_operation_of_several_sends_and_sends_nothing_after_it_expires() {
    let (url, accepted, _server) = counting_server(Duration::from_millis(150)).await;
    let client = client(&url, Duration::from_secs(10));
    let deadline = Deadline::after(Duration::from_millis(400));

    let mut successes = 0_u64;
    let mut refusals = 0;
    for _ in 0..5 {
        match client.send(item()).deadline(&deadline).full().await {
            Ok(response) => {
                assert_eq!(refusals, 0, "no send succeeds after the deadline stopped one");
                assert_eq!(response.data(), Some(&Item { id: 1 }));
                successes += 1;
            }
            Err(error) => {
                refusals += 1;
                match error.kind {
                    TransportErrorKind::DeadlineExceeded {
                        completed_requests,
                        deadline_elapsed,
                    } => {
                        assert_eq!(completed_requests, successes);
                        assert!(deadline_elapsed >= Duration::from_millis(400), "{deadline_elapsed:?}");
                    }
                    other => panic!("expected DeadlineExceeded, got {other:?}"),
                }
            }
        }
    }
    assert!((1..5).contains(&successes), "{successes} of 5 fit in the budget");
    assert_eq!(deadline.completed_requests(), successes);

    // Served in accept order: everything counted before the control request
    // was sent under the deadline.
    client.send(item()).full().await.expect("control request");
    let under_deadline = accepted.load(Ordering::SeqCst) as u64 - 1;
    assert!(
        (successes..=successes + 1).contains(&under_deadline),
        "{under_deadline} requests reached the mock for {successes} successes and at most one in flight"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_sends_that_received_a_head_count_as_completed() {
    let (failing, _failing_server) = serve(|mut stream| async move {
        read_head(&mut stream).await;
        write_all(&mut stream, reply("500 Internal Server Error", "{}").as_bytes()).await;
    })
    .await;
    let (heads_seen, mut head_seen) = mpsc::unbounded_channel();
    let (hanging, _hanging_server) = serve(move |mut stream| {
        let heads_seen = heads_seen.clone();
        async move {
            read_head(&mut stream).await;
            let _ = heads_seen.send(());
            std::future::pending::<()>().await;
        }
    })
    .await;
    let deadline = Deadline::after(Duration::from_secs(10));

    let response = client(&failing, Duration::from_secs(5)).send(item()).deadline(&deadline).full().await.expect("head");
    assert_eq!(response.error().expect("failure").kind, ErrorKind::Undeclared);
    assert_eq!(deadline.completed_requests(), 1, "an HTTP failure is a completed send");

    let error = client(&closed_port_url().await, Duration::from_secs(5))
        .send(item()).deadline(&deadline).full()
        .await
        .expect_err("refused");
    assert_eq!(error.kind, TransportErrorKind::Connect);
    assert_eq!(deadline.completed_requests(), 1, "a transport failure is not");

    let hanging_client = client(&hanging, Duration::from_secs(5));
    tokio::select! {
        result = hanging_client.send(item()).deadline(&deadline).full() => panic!("the send cannot finish: {:?}", result.map(|r| r.meta.status)),
        _ = head_seen.recv() => {}
    }
    assert_eq!(deadline.completed_requests(), 1, "a dropped send is not");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_expiration_after_the_head_is_a_body_interruption_with_the_partial_bytes() {
    let (url, _server) = serve(|mut stream| async move {
        read_head(&mut stream).await;
        write_all(&mut stream, &partial_head("200 OK")).await;
        std::future::pending::<()>().await;
    })
    .await;

    let started = Instant::now();
    let response = client(&url, Duration::from_millis(300))
        .send(item()).full()
        .await
        .expect("the head arrived, so an envelope, not a timeout");

    assert_eq!(response.meta.status, 200);
    let error = response.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::BodyInterrupted);
    assert_eq!(error.body, Bytes::from_static(PARTIAL));
    assert!(started.elapsed() >= Duration::from_millis(300), "{:?}", started.elapsed());
    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
}

/// Accepts one connection, sends a head and part of the body, reports that,
/// then reports when the client closes the connection.
fn partial_then_wait_for_close(
    listener: tokio::net::TcpListener,
) -> (oneshot::Receiver<()>, JoinHandle<()>) {
    let (written, written_seen) = oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_head(&mut stream).await;
        write_all(&mut stream, &partial_head("200 OK")).await;
        let _ = written.send(());
        read_until_closed(&mut stream).await;
    });
    (written_seen, task)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_a_send_closes_its_connection_and_does_not_count_it() {
    let (listener, url) = listener().await;
    let (written_seen, server) = partial_then_wait_for_close(listener);
    let mut server = AbortOnDrop(server);
    let deadline = Deadline::after(Duration::from_secs(10));
    let client = client(&url, Duration::from_secs(10));

    tokio::select! {
        result = client.send(item()).deadline(&deadline).full() => {
            panic!("the body never completes: {:?}", result.map(|r| r.meta.status));
        }
        _ = written_seen => {}
    }

    tokio::time::timeout(Duration::from_secs(2), &mut server.0)
        .await
        .expect("the mock saw the connection close")
        .expect("mock task");
    assert_eq!(deadline.completed_requests(), 0);
}

#[test]
fn a_dropped_send_leaves_no_task_alive_and_the_runtime_shuts_down_promptly() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let metrics = tokio::runtime::Handle::current().metrics();
        let baseline = metrics.num_alive_tasks();
        let (listener, url) = listener().await;
        let (written_seen, server) = partial_then_wait_for_close(listener);
        let client = client(&url, Duration::from_secs(10));

        tokio::select! {
            result = client.send(item()).full() => {
                panic!("the body never completes: {:?}", result.map(|r| r.meta.status));
            }
            _ = written_seen => {}
        }
        drop(client);
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .expect("the mock saw the connection close")
            .expect("mock task");

        // Connection tasks end asynchronously after the close; poll with a bound.
        let settling = Instant::now();
        while metrics.num_alive_tasks() > baseline {
            assert!(
                settling.elapsed() < Duration::from_secs(2),
                "{} tasks still alive over a baseline of {baseline}",
                metrics.num_alive_tasks()
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });

    let shutting_down = std::time::Instant::now();
    runtime.shutdown_timeout(Duration::from_secs(2));
    assert!(shutting_down.elapsed() < Duration::from_secs(1), "{:?}", shutting_down.elapsed());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_streaming_decoder_yields_its_first_line_before_the_body_ends() {
    let (release, released) = oneshot::channel::<()>();
    let released = std::sync::Mutex::new(Some(released));
    let (url, _server) = serve(move |mut stream| {
        let released = released.lock().expect("lock").take();
        async move {
            read_head(&mut stream).await;
            write_all(&mut stream, format!("{STREAM_HEAD}{}", chunk("first\n")).as_bytes()).await;
            if let Some(released) = released {
                let _ = released.await;
            }
            write_all(&mut stream, format!("{}{}0\r\n\r\n", chunk("second\nthi"), chunk("rd\n")).as_bytes()).await;
        }
    })
    .await;

    let response = client(&url, Duration::from_secs(5))
        .send(Call::<Lines>::get("/stream")).full()
        .await
        .expect("head");
    assert_eq!(response.meta.status, 200);
    let mut lines = into_lines(response);

    assert_eq!(lines.next_line().await, Some(Ok("first".to_string())));
    release.send(()).expect("mock is waiting");
    assert_eq!(lines.next_line().await, Some(Ok("second".to_string())));
    assert_eq!(lines.next_line().await, Some(Ok("third".to_string())));
    assert_eq!(lines.next_line().await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stream_outlives_the_send_expiration_while_it_keeps_delivering_then_times_out_when_idle() {
    let timeout = Duration::from_millis(400);
    let gap = Duration::from_millis(150);
    let (url, _server) = serve(move |mut stream| async move {
        read_head(&mut stream).await;
        write_all(&mut stream, format!("{STREAM_HEAD}{}", chunk("line 1\n")).as_bytes()).await;
        for line in 2..=4 {
            tokio::time::sleep(gap).await;
            write_all(&mut stream, chunk(&format!("line {line}\n")).as_bytes()).await;
        }
        std::future::pending::<()>().await;
    })
    .await;

    let started = Instant::now();
    let response = client(&url, timeout)
        .send(Call::<Lines>::get("/stream")).full()
        .await
        .expect("head");
    let mut lines = into_lines(response);
    for line in 1..=4 {
        assert_eq!(lines.next_line().await, Some(Ok(format!("line {line}"))));
    }
    assert!(started.elapsed() > timeout, "the stream ran past the single-send expiration: {:?}", started.elapsed());

    let idle = Instant::now();
    assert_eq!(lines.next_line().await, Some(Err(BodyReadError::TimedOut)));
    assert!(idle.elapsed() >= timeout - Duration::from_millis(10), "{:?}", idle.elapsed());
    assert!(idle.elapsed() < Duration::from_secs(5), "{:?}", idle.elapsed());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_shared_deadline_bounds_a_stream_that_never_goes_idle() {
    let (url, _server) = serve(|mut stream| async move {
        read_head(&mut stream).await;
        if stream.write_all(STREAM_HEAD.as_bytes()).await.is_err() {
            return;
        }
        for line in 1.. {
            if stream.write_all(chunk(&format!("line {line}\n")).as_bytes()).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;
    let deadline = Deadline::after(Duration::from_millis(300));

    let response = client(&url, Duration::from_secs(10))
        .send(Call::<Lines>::get("/stream")).deadline(&deadline).full()
        .await
        .expect("head");
    assert_eq!(deadline.completed_requests(), 1, "the send completed when the decoder returned");
    let mut lines = into_lines(response);

    let mut received = 0;
    let ended = loop {
        match lines.next_line().await {
            Some(Ok(_)) => received += 1,
            other => break other,
        }
    };
    assert_eq!(ended, Some(Err(BodyReadError::TimedOut)));
    assert!(received >= 1, "lines arrived before the deadline");
    assert!(deadline.elapsed() >= Duration::from_millis(300), "{:?}", deadline.elapsed());
    assert!(deadline.elapsed() < Duration::from_secs(5), "{:?}", deadline.elapsed());
}
