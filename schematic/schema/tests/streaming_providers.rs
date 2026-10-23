//! Streaming and multi-shape replies of the migrated providers, received
//! through the generated clients from a local mock server.
//!
//! Incremental delivery cannot be shown with wiremock, which sends a whole
//! body at once, so those tests use [`HeldServer`]: a raw TCP server that
//! sends the response head and a first part of the body, then waits until the
//! test has received the first item before sending the rest. A client that
//! buffered the body would never yield that item, and the test times out
//! instead of passing. Other tests use wiremock. Calls are anonymous so a host
//! credential never reaches the mock.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bytes::Bytes;
use schematic_definitions::anthropic::{CreateMessageBody, Message, MessageStreamEvent};
use schematic_definitions::elevenlabs::CreateSpeechBody;
use schematic_definitions::ollama::{GenerateBody, GenerateStreamItem, OpenAIChatCompletionRequest, OpenAIMessage};
use schematic_definitions::openai::BetaResponseStreamEvent;
use schematic_schema::anthropic::CreateMessageResponse;
use schematic_schema::bitbucket::GetSourceResponse;
use schematic_schema::ollama::{ChatCompletionsResponse, GenerateResponse};
use schematic_schema::openai::GetResponseResponse;
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::StreamError;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The bound on every await that waits for the mock.
const WAIT: Duration = Duration::from_secs(5);

/// Awaits `future`, failing the test instead of hanging past [`WAIT`].
async fn bounded<F: std::future::IntoFuture>(what: &str, future: F) -> F::Output {
    timeout(WAIT, future.into_future())
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
}

/// How a [`HeldServer`] frames its body and ends it.
#[derive(Clone, Copy)]
enum Framing {
    /// `Transfer-Encoding: chunked`; the body ends with the last chunk.
    Chunked,
    /// No length and `Connection: close`; the body ends when the server closes.
    CloseDelimited,
}

/// What a [`HeldServer`] does after sending the rest of the body.
#[derive(Clone, Copy, PartialEq)]
enum AfterRest {
    /// Ends the body.
    End,
    /// Keeps the connection open without ending the body until the test ends,
    /// so a stream that finishes must have finished at its terminator.
    Hold,
}

/// One request as the server received it.
struct Captured {
    head: String,
    body: Vec<u8>,
}

impl Captured {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("the request body is JSON")
    }
}

/// A single-connection server that sends `first`, waits for [`proceed`],
/// then sends `rest`.
///
/// [`proceed`]: HeldServer::proceed
struct HeldServer {
    url: String,
    request: Option<oneshot::Receiver<Captured>>,
    proceed: Option<oneshot::Sender<()>>,
    rest_sent: Arc<AtomicBool>,
    // Dropping it releases a held connection.
    _release: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl HeldServer {
    async fn start(content_type: &str, framing: Framing, first: &[u8], rest: &[u8], after: AfterRest) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind loopback");
        let url = format!("http://{}", listener.local_addr().expect("local address"));
        let (request_tx, request_rx) = oneshot::channel();
        let (proceed_tx, proceed_rx) = oneshot::channel::<()>();
        let (release_tx, release_rx) = oneshot::channel::<()>();
        let rest_sent = Arc::new(AtomicBool::new(false));
        let server_rest_sent = Arc::clone(&rest_sent);
        let head = match framing {
            Framing::Chunked => format!("HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ntransfer-encoding: chunked\r\n\r\n"),
            Framing::CloseDelimited => format!("HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\nconnection: close\r\n\r\n"),
        };
        let (first, rest) = (first.to_vec(), rest.to_vec());
        let task = tokio::spawn(async move {
            let Ok((mut stream, _)) = listener.accept().await else { return };
            let _ = request_tx.send(read_request(&mut stream).await);
            write(&mut stream, head.as_bytes()).await;
            write(&mut stream, &frame(framing, &first)).await;
            if proceed_rx.await.is_err() {
                return;
            }
            server_rest_sent.store(true, Ordering::SeqCst);
            write(&mut stream, &frame(framing, &rest)).await;
            if after == AfterRest::Hold {
                let _ = release_rx.await;
                return;
            }
            if let Framing::Chunked = framing {
                write(&mut stream, b"0\r\n\r\n").await;
            }
            let _ = stream.shutdown().await;
        });
        Self {
            url,
            request: Some(request_rx),
            proceed: Some(proceed_tx),
            rest_sent,
            _release: release_tx,
            task,
        }
    }

    /// Asserts the rest of the body has not been sent, then lets the server
    /// send it.
    fn proceed(&mut self) {
        assert!(!self.rest_sent.load(Ordering::SeqCst), "the rest was sent before the test asked for it");
        self.proceed.take().expect("proceed once").send(()).expect("the server is waiting");
    }

    async fn request(&mut self) -> Captured {
        bounded("the captured request", self.request.take().expect("request once"))
            .await
            .expect("the server read a request")
    }
}

impl Drop for HeldServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn frame(framing: Framing, bytes: &[u8]) -> Vec<u8> {
    match framing {
        Framing::Chunked if bytes.is_empty() => Vec::new(),
        Framing::Chunked => [format!("{:x}\r\n", bytes.len()).as_bytes(), bytes, b"\r\n"].concat(),
        Framing::CloseDelimited => bytes.to_vec(),
    }
}

async fn write(stream: &mut TcpStream, bytes: &[u8]) {
    let _ = stream.write_all(bytes).await;
    let _ = stream.flush().await;
}

/// Reads one request head and its `content-length` body.
async fn read_request(stream: &mut TcpStream) -> Captured {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte).await {
            Ok(0) | Err(_) => break,
            Ok(_) => head.push(byte[0]),
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let length = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; length];
    let _ = stream.read_exact(&mut body).await;
    Captured { head, body }
}

fn generate_body(stream: Option<bool>) -> GenerateBody {
    GenerateBody {
        model: "llama3".into(),
        prompt: "Why is the sky blue?".into(),
        stream,
        ..Default::default()
    }
}

fn message_body(stream: Option<bool>) -> CreateMessageBody {
    CreateMessageBody {
        model: "claude-opus-5-5".into(),
        messages: vec![Message::user("Hello")],
        max_tokens: 64,
        stream,
        ..Default::default()
    }
}

fn stream_chunk(item: &GenerateStreamItem) -> &schematic_definitions::ollama::GenerateStreamChunk {
    match item {
        GenerateStreamItem::Chunk(chunk) => chunk,
        GenerateStreamItem::Error(error) => panic!("expected a chunk, got the error line {error:?}"),
    }
}

/// Ollama `Generate` with `stream: true` yields its first NDJSON line before
/// the server sends the rest, delivers the kept `done: true` line with its
/// statistics last, and then ends while the connection is still open.
#[tokio::test]
async fn ollama_generate_ndjson_is_delivered_incrementally_and_keeps_its_final_line() {
    let first = br#"{"model":"llama3","created_at":"2026-10-22T00:00:00Z","response":"The","done":false}"#.to_vec();
    let rest = concat!(
        r#"{"model":"llama3","created_at":"2026-10-22T00:00:01Z","response":" sky","done":false}"#,
        "\n",
        r#"{"model":"llama3","created_at":"2026-10-22T00:00:02Z","response":"","done":true,"#,
        r#""total_duration":5000,"prompt_eval_count":7,"eval_count":42}"#,
        "\n",
    );
    let mut server = HeldServer::start(
        "application/x-ndjson",
        Framing::Chunked,
        &[first, b"\n".to_vec()].concat(),
        rest.as_bytes(),
        AfterRest::Hold,
    )
    .await;
    let client = OllamaNative::new().base_url(&server.url).build().expect("client builds");

    let reply = bounded("the response head", client.generate(generate_body(Some(true))).anonymous())
        .await
        .expect("success");
    let GenerateResponse::Stream(mut items) = reply else { panic!("an NDJSON reply is the stream: {reply:?}") };

    let item = bounded("the first item", items.next()).await.expect("an item").expect("it decodes");
    assert_eq!(stream_chunk(&item).response.as_deref(), Some("The"));
    server.proceed();

    let item = bounded("the second item", items.next()).await.expect("an item").expect("it decodes");
    assert_eq!(stream_chunk(&item).response.as_deref(), Some(" sky"));
    let last = bounded("the final item", items.next()).await.expect("the kept final line").expect("it decodes");
    let last = stream_chunk(&last);
    assert!(last.done);
    assert_eq!(last.eval_count, Some(42));
    assert_eq!(last.prompt_eval_count, Some(7));
    assert_eq!(last.total_duration, Some(5000));
    assert!(bounded("the end of the stream", items.next()).await.is_none(), "the stream ends at done=true");
    assert!(bounded("the end of the stream", items.next()).await.is_none(), "an ended stream stays ended");

    let request = server.request().await;
    assert!(request.head.starts_with("POST /api/generate "), "{}", request.head);
    assert_eq!(request.json()["stream"], json!(true));
}

/// A `{"error": ...}` line inside a 200 NDJSON stream decodes as the typed
/// `Error` item, not a `StreamError`, and the stream continues; with `stream`
/// omitted the body carries no `stream` field.
#[tokio::test]
async fn ollama_generate_mid_stream_error_line_is_a_typed_item() {
    let body = concat!(
        r#"{"model":"llama3","created_at":"2026-10-22T00:00:00Z","response":"Hi","done":false}"#,
        "\n",
        r#"{"error":"model runner has unexpectedly stopped"}"#,
        "\n",
        r#"{"model":"llama3","created_at":"2026-10-22T00:00:01Z","done":true,"eval_count":1}"#,
        "\n",
    );
    let mut server = HeldServer::start("application/x-ndjson", Framing::Chunked, body.as_bytes(), b"", AfterRest::End).await;
    server.proceed();
    let client = OllamaNative::new().base_url(&server.url).build().expect("client builds");

    let reply = bounded("the response head", client.generate(generate_body(None)).anonymous())
        .await
        .expect("success");
    let GenerateResponse::Stream(mut items) = reply else { panic!("an NDJSON reply is the stream: {reply:?}") };

    let mut received = Vec::new();
    while let Some(item) = bounded("the next item", items.next()).await {
        received.push(item.expect("every line decodes"));
    }

    assert_eq!(received.len(), 3, "{received:?}");
    let GenerateStreamItem::Error(error) = &received[1] else { panic!("expected the error line: {:?}", received[1]) };
    assert_eq!(error.error, "model runner has unexpectedly stopped");
    assert_eq!(stream_chunk(&received[2]).eval_count, Some(1));
    let request = server.request().await;
    assert!(request.json().get("stream").is_none(), "an omitted `stream` is not sent: {:?}", request.json());
}

/// Ollama `Generate` replying `application/json` selects the `Json` variant;
/// the body carries `stream: false` as the caller set it.
#[tokio::test]
async fn ollama_generate_json_reply_selects_json_variant() {
    let mock_server = MockServer::start().await;
    let completion = json!({
        "model": "llama3",
        "created_at": "2026-10-22T00:00:00Z",
        "response": "Because of Rayleigh scattering.",
        "done": true,
        "eval_count": 9
    });
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(completion.to_string(), "application/json"))
        .expect(1)
        .mount(&mock_server)
        .await;
    let client = OllamaNative::new().base_url(mock_server.uri()).build().expect("client builds");

    let reply = client.generate(generate_body(Some(false))).anonymous().await.expect("success");

    let GenerateResponse::Json(completion) = reply else { panic!("a JSON reply is the completion: {reply:?}") };
    assert_eq!(completion.response, "Because of Rayleigh scattering.");
    assert_eq!(completion.eval_count, Some(9));
    let requests = mock_server.received_requests().await.expect("recording is on");
    let sent: Value = serde_json::from_slice(&requests[0].body).expect("JSON body");
    assert_eq!(sent["stream"], json!(false));
}

fn openai_chunk(content: &str, finish_reason: Option<&str>) -> String {
    json!({
        "id": "chatcmpl-1",
        "object": "chat.completion.chunk",
        "created": 1_760_000_000,
        "model": "llama3",
        "choices": [{ "index": 0, "delta": { "content": content }, "finish_reason": finish_reason }]
    })
    .to_string()
}

/// Ollama's OpenAI-compatible `ChatCompletions` yields its first SSE event
/// before the server sends the rest, and `data: [DONE]` ends the stream
/// without being yielded, while the connection is still open.
#[tokio::test]
async fn ollama_chat_completions_sse_is_delivered_incrementally_and_done_is_consumed() {
    let first = format!("data: {}\n\n", openai_chunk("Hel", None));
    let rest = format!("data: {}\n\ndata: [DONE]\n\n", openai_chunk("lo", Some("stop")));
    let mut server =
        HeldServer::start("text/event-stream", Framing::Chunked, first.as_bytes(), rest.as_bytes(), AfterRest::Hold).await;
    let client = OllamaOpenAI::new().base_url(&server.url).build().expect("client builds");
    let body = OpenAIChatCompletionRequest {
        model: "llama3".into(),
        messages: vec![OpenAIMessage { role: "user".into(), content: "Hello".into() }],
        stream: Some(true),
        ..Default::default()
    };

    let reply = bounded("the response head", client.chat_completions(body).anonymous()).await.expect("success");
    let ChatCompletionsResponse::Stream(mut chunks) = reply else { panic!("an SSE reply is the stream: {reply:?}") };

    let chunk = bounded("the first chunk", chunks.next()).await.expect("a chunk").expect("it decodes");
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hel"));
    server.proceed();
    let chunk = bounded("the second chunk", chunks.next()).await.expect("a chunk").expect("it decodes");
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("lo"));
    assert_eq!(chunk.choices[0].finish_reason.as_deref(), Some("stop"));
    assert!(bounded("the end of the stream", chunks.next()).await.is_none(), "[DONE] ends the stream unyielded");

    let request = server.request().await;
    assert!(request.head.starts_with("POST /v1/chat/completions "), "{}", request.head);
    assert_eq!(request.json()["stream"], json!(true));
}

/// Anthropic `CreateMessage` replying `application/json` selects the `Json`
/// variant whether `stream` was false or omitted, and the client sends
/// `stream` exactly as the caller set it.
#[tokio::test]
async fn anthropic_json_reply_selects_json_variant_and_sends_stream_as_set() {
    let mock_server = MockServer::start().await;
    let message = json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-opus-5-5",
        "content": [{ "type": "text", "text": "Hi there" }],
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 5, "output_tokens": 3 }
    });
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(message.to_string(), "application/json"))
        .expect(2)
        .mount(&mock_server)
        .await;
    let client = Anthropic::new().base_url(mock_server.uri()).build().expect("client builds");

    for stream in [Some(false), None] {
        let reply = client.create_message(message_body(stream)).anonymous().await.expect("success");
        let CreateMessageResponse::Json(message) = reply else { panic!("a JSON reply is the message: {reply:?}") };
        assert_eq!(message.id, "msg_01");
        assert_eq!(message.text_content(), "Hi there");
    }

    let requests = mock_server.received_requests().await.expect("recording is on");
    let sent: Vec<Value> = requests.iter().map(|r| serde_json::from_slice(&r.body).expect("JSON body")).collect();
    assert_eq!(sent[0]["stream"], json!(false));
    assert!(sent[1].get("stream").is_none(), "an omitted `stream` is not sent: {:?}", sent[1]);
}

/// Anthropic `CreateMessage` replying `text/event-stream` selects the
/// `Stream` variant, which delivers every event in order, including `ping` and
/// an in-stream `error` as typed items, and ends at `message_stop` without
/// yielding it or anything after it; the body carries `stream: true`.
#[tokio::test]
async fn anthropic_sse_reply_delivers_events_in_order_and_consumes_message_stop() {
    let mock_server = MockServer::start().await;
    let events = concat!(
        "event: message_start\n",
        r#"data: {"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","content":[],"model":"claude-opus-5-5","stop_reason":null,"stop_sequence":null}}"#,
        "\n\n",
        "event: ping\n",
        r#"data: {"type":"ping"}"#,
        "\n\n",
        "event: content_block_delta\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
        "\n\n",
        "event: error\n",
        r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
        "\n\n",
        "event: message_stop\n",
        r#"data: {"type":"message_stop"}"#,
        "\n\n",
        "event: ping\n",
        r#"data: {"type":"ping"}"#,
        "\n\n",
    );
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(events, "text/event-stream"))
        .expect(1)
        .mount(&mock_server)
        .await;
    let client = Anthropic::new().base_url(mock_server.uri()).build().expect("client builds");

    let reply = client.create_message(message_body(Some(true))).anonymous().await.expect("success");
    let CreateMessageResponse::Stream(mut stream) = reply else { panic!("an SSE reply is the stream: {reply:?}") };
    let mut received = Vec::new();
    while let Some(event) = bounded("the next event", stream.next()).await {
        received.push(event.expect("every event decodes"));
    }

    assert_eq!(received.len(), 4, "message_stop and what follows it are not yielded: {received:?}");
    assert!(matches!(&received[0], MessageStreamEvent::MessageStart { message } if message.id == "msg_01"));
    assert_eq!(received[1], MessageStreamEvent::Ping);
    assert!(matches!(&received[2], MessageStreamEvent::ContentBlockDelta { index: 0, .. }));
    let MessageStreamEvent::Error { error } = &received[3] else { panic!("expected the error event: {:?}", received[3]) };
    assert_eq!(error.error_type, "overloaded_error");
    assert_eq!(error.message, "Overloaded");
    let requests = mock_server.received_requests().await.expect("recording is on");
    let sent: Value = serde_json::from_slice(&requests[0].body).expect("JSON body");
    assert_eq!(sent["stream"], json!(true));
}

/// Bitbucket `GetSource` selects `Directory` for a JSON listing and `File`,
/// with the exact bytes, for a `text/x-rust` body.
#[tokio::test]
async fn bitbucket_get_source_selects_directory_or_file_by_media_type() {
    let mock_server = MockServer::start().await;
    let listing = json!({
        "values": [
            { "path": "src/lib.rs", "type": "commit_file", "size": 120 },
            { "path": "src/bin", "type": "commit_directory" }
        ],
        "pagelen": 10
    });
    let source: &[u8] = b"fn main() {\n    println!(\"hi\");\n}\n";
    Mock::given(method("GET"))
        .and(path("/repositories/team/app/src/abc123/src"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(listing.to_string(), "application/json"))
        .expect(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repositories/team/app/src/abc123/main.rs"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(source, "text/x-rust"))
        .expect(1)
        .mount(&mock_server)
        .await;
    let client = Bitbucket::new().base_url(mock_server.uri()).build().expect("client builds");

    let directory = client.get_source("team", "app", "abc123", "src").anonymous().await.expect("success");
    let file = client.get_source("team", "app", "abc123", "main.rs").anonymous().await.expect("success");

    let GetSourceResponse::Directory(listing) = directory else { panic!("a JSON listing is the directory: {directory:?}") };
    assert_eq!(listing.pagelen, Some(10));
    let entries: Vec<_> = listing.values.iter().map(|entry| (entry.path.as_deref(), entry.entry_type.as_deref())).collect();
    assert_eq!(entries, [(Some("src/lib.rs"), Some("commit_file")), (Some("src/bin"), Some("commit_directory"))]);
    assert_eq!(listing.values[0].size, Some(120));
    let GetSourceResponse::File(bytes) = file else { panic!("a text/x-rust body is the file: {file:?}") };
    let bytes: Bytes = bytes;
    assert_eq!(bytes.as_ref(), source);
}

/// OpenAI `GetResponse` replying `application/json` selects the `Json`
/// variant.
#[tokio::test]
async fn openai_get_response_json_reply_selects_json_variant() {
    let mock_server = MockServer::start().await;
    let response = json!({ "id": "resp_1", "object": "response", "status": "completed" });
    Mock::given(method("GET"))
        .and(path("/responses/resp_1"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(response.to_string(), "application/json"))
        .expect(1)
        .mount(&mock_server)
        .await;
    let client = OpenAI::new().base_url(mock_server.uri()).build().expect("client builds");

    let reply = client.get_response("resp_1").anonymous().await.expect("success");

    let GetResponseResponse::Json(response) = reply else { panic!("a JSON reply is the response: {reply:?}") };
    assert_eq!(response.json["id"], json!("resp_1"));
    assert_eq!(response.json["status"], json!("completed"));
}

/// OpenAI `GetResponse` with `stream=true` replying `text/event-stream`
/// selects the `Stream` variant, which yields each event as the alternative
/// its `type` names, with its payload, and ends cleanly, with no `EndedEarly`,
/// when the server closes the connection.
#[tokio::test]
async fn openai_get_response_sse_ends_cleanly_at_connection_close() {
    // A text delta, the completed response, and an error event each carry the
    // fields of the audio events listed before them, so a first-match decoder
    // returns them as audio events.
    let events = concat!(
        "event: response.audio.delta\n",
        r#"data: {"type":"response.audio.delta","sequence_number":1,"delta":"AAEC"}"#,
        "\n\n",
        "event: response.audio.done\n",
        r#"data: {"type":"response.audio.done","sequence_number":2}"#,
        "\n\n",
        "event: response.output_text.delta\n",
        r#"data: {"type":"response.output_text.delta","item_id":"msg_1","output_index":0,"content_index":0,"delta":"hi","sequence_number":3,"logprobs":[]}"#,
        "\n\n",
        "event: response.completed\n",
        r#"data: {"type":"response.completed","sequence_number":4,"response":{"id":"resp_1","object":"response","status":"completed","output":[{"type":"message","id":"msg_1","role":"assistant","status":"completed","content":[{"type":"output_text","text":"hi","annotations":[]}]}]}}"#,
        "\n\n",
        "event: error\n",
        r#"data: {"type":"error","code":"server_error","message":"The server had an error.","param":null,"sequence_number":5}"#,
        "\n\n",
    );
    let mut server =
        HeldServer::start("text/event-stream", Framing::CloseDelimited, events.as_bytes(), b"", AfterRest::End).await;
    server.proceed();
    let client = OpenAI::new().base_url(&server.url).build().expect("client builds");

    let reply = bounded("the response head", client.get_response("resp_1").stream(true).anonymous())
        .await
        .expect("success");
    let GetResponseResponse::Stream(mut stream) = reply else { panic!("an SSE reply is the stream: {reply:?}") };
    let mut received = Vec::new();
    while let Some(event) = bounded("the next event", stream.next()).await {
        received.push(event);
    }

    assert_eq!(received.len(), 5, "{received:?}");
    let Ok(BetaResponseStreamEvent::BetaResponseAudioDeltaEvent(delta)) = &received[0] else {
        panic!("expected the audio delta: {:?}", received[0])
    };
    assert_eq!((delta.sequence_number, delta.delta.as_str()), (1, "AAEC"));
    let Ok(BetaResponseStreamEvent::BetaResponseAudioDoneEvent(done)) = &received[1] else {
        panic!("expected the audio done event: {:?}", received[1])
    };
    assert_eq!(done.sequence_number, 2);
    let Ok(BetaResponseStreamEvent::BetaResponseTextDeltaEvent(text)) = &received[2] else {
        panic!("expected the text delta: {:?}", received[2])
    };
    assert_eq!(
        (text.item_id.as_str(), text.output_index, text.content_index, text.delta.as_str(), text.sequence_number),
        ("msg_1", 0, 0, "hi", 3)
    );
    assert!(text.logprobs.is_empty());
    let Ok(BetaResponseStreamEvent::BetaResponseCompletedEvent(completed)) = &received[3] else {
        panic!("expected the completed response: {:?}", received[3])
    };
    let response = serde_json::to_value(&completed.response).expect("the response serializes");
    assert_eq!((response["id"].as_str(), response["status"].as_str()), (Some("resp_1"), Some("completed")));
    assert_eq!(response["output"][0]["content"][0]["text"], json!("hi"));
    let Ok(BetaResponseStreamEvent::BetaResponseErrorEvent(error)) = &received[4] else {
        panic!("expected the error event: {:?}", received[4])
    };
    assert_eq!((error.code.clone(), error.message.as_str(), error.param.clone()), (json!("server_error"), "The server had an error.", Value::Null));
    assert_eq!(error.sequence_number, 5);
    assert!(!received.iter().any(|item| matches!(item, Err(StreamError::EndedEarly))));
    let request = server.request().await;
    assert!(request.head.starts_with("GET /responses/resp_1?stream=true "), "{}", request.head);
}

/// ElevenLabs `StreamSpeech` yields the first audio chunk before the server
/// sends the rest, and the chunks concatenate to exactly the bytes sent.
#[tokio::test]
async fn elevenlabs_stream_speech_yields_audio_before_the_body_completes() {
    let first: &[u8] = b"ID3\x04\x00\x00\x00\x00\x00\x00\xff\xfb\x90";
    let rest: &[u8] = b"\xff\xfb\x90\x44\x00\x00\x00\x00\x01\x02\x03";
    let mut server = HeldServer::start("audio/mpeg", Framing::Chunked, first, rest, AfterRest::End).await;
    let client = ElevenLabs::new().base_url(&server.url).build().expect("client builds");
    let body = CreateSpeechBody { text: "Hello".into(), ..Default::default() };

    let mut audio = bounded("the response head", client.stream_speech("voice-1", body).anonymous())
        .await
        .expect("success");

    let mut received = Vec::new();
    while received.len() < first.len() {
        let chunk = bounded("the first audio bytes", audio.next()).await.expect("a chunk").expect("no error");
        received.extend_from_slice(&chunk);
    }
    assert_eq!(received, first, "only the first part has been sent");
    server.proceed();
    while let Some(chunk) = bounded("the next audio chunk", audio.next()).await {
        received.extend_from_slice(&chunk.expect("no error"));
    }

    assert_eq!(received, [first, rest].concat());
    let request = server.request().await;
    assert!(request.head.starts_with("POST /v1/text-to-speech/voice-1/stream "), "{}", request.head);
    assert_eq!(request.json()["text"], json!("Hello"));
}
