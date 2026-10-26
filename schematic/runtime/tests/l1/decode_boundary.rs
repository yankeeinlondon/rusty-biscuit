//! The `Decode` boundary and the endpoint contract (R30 groundwork, R11 and
//! R26 groundwork for body reads under an expiration).
//!
//! Bodies here are in-memory streams; managed sends feed the same
//! [`LiveBody`] from the network.

use std::time::Duration;

use bytes::Bytes;
use futures_util::{StreamExt, stream};
use http::{HeaderMap, Method, StatusCode};
use schematic_runtime::{
    Api, BodyReadError, CallOptions, Deadline, Decode, DecodeFailure, DecodeFailureKind, EndpointSpec, ErrorRoute,
    InvalidArgument, LiveBody, NoDeclaredError, RequestBody, RequestParts, StatusMatch,
};
use tokio::sync::mpsc;
use tokio::time::Instant;

fn body<S>(pieces: S, read_until: Option<Instant>) -> LiveBody
where
    S: futures_core::Stream<Item = Result<Bytes, BodyReadError>> + Send + 'static,
{
    LiveBody::new(StatusCode::OK, HeaderMap::new(), pieces, read_until)
}

fn pieces(items: &[&'static str]) -> Vec<Result<Bytes, BodyReadError>> {
    items.iter().map(|item| Ok(Bytes::from_static(item.as_bytes()))).collect()
}

/// A test-only decoder whose success value has two wire-distinguishable
/// shapes, as a generated multi-shape enum would.
struct TwoShape;

#[derive(Debug, PartialEq)]
enum Shape {
    Number(u64),
    Word(String),
}

impl Decode for TwoShape {
    type Output = Shape;

    async fn decode(body: LiveBody) -> Result<Shape, DecodeFailure> {
        let bytes = body.collect().await?;
        let text = std::str::from_utf8(&bytes).map_err(|_| DecodeFailure::new(DecodeFailureKind::Malformed, bytes.clone()))?;
        if let Ok(number) = text.parse() {
            Ok(Shape::Number(number))
        } else if !text.is_empty() && text.chars().all(char::is_alphabetic) {
            Ok(Shape::Word(text.to_string()))
        } else {
            Err(DecodeFailure::new(DecodeFailureKind::Malformed, bytes))
        }
    }
}

/// A test-only incremental line decoder: its output owns the live body and
/// yields each line as soon as it arrives.
struct Lines;

struct LineStream {
    body: LiveBody,
    pending: Vec<u8>,
}

impl LineStream {
    async fn next_line(&mut self) -> Option<Result<String, BodyReadError>> {
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
        Ok(LineStream { body, pending: Vec::new() })
    }
}

struct FixtureApi;

impl Api for FixtureApi {
    const NAME: &'static str = "Fixture";
}

struct GetItem {
    id: String,
    tags: Vec<String>,
}

impl EndpointSpec for GetItem {
    type Api = FixtureApi;
    type Success = Shape;
    type Failure = NoDeclaredError;
    type Decoder = TwoShape;

    const ENDPOINT_ID: &'static str = "get_item";
    const METHOD: Method = Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        if self.id.is_empty() {
            return Err(InvalidArgument::new("id must not be empty"));
        }
        let mut parts = RequestParts::new(Method::GET, format!("/items/{}", self.id));
        parts.query = self.tags.into_iter().map(|tag| ("tag".to_string(), tag.into())).collect();
        Ok(parts)
    }
}

/// Decodes through the trait bound, as the runtime does.
async fn decode_with<D: Decode>(body: LiveBody) -> Result<D::Output, DecodeFailure> {
    D::decode(body).await
}

#[tokio::test]
async fn collect_reads_every_piece_in_order() {
    let collected = body(stream::iter(pieces(&["ab", "", "cd", "e"])), None).collect().await;
    assert_eq!(collected, Ok(Bytes::from_static(b"abcde")));
}

#[tokio::test]
async fn an_interrupted_body_keeps_the_bytes_that_arrived() {
    let mut items = pieces(&["abc", "de"]);
    items.push(Err(BodyReadError::Interrupted));
    items.extend(pieces(&["never read"]));
    let failure = body(stream::iter(items), None).collect().await.expect_err("interrupted");
    assert_eq!(failure, DecodeFailure::new(DecodeFailureKind::Interrupted, Bytes::from_static(b"abcde")));
}

#[tokio::test(start_paused = true)]
async fn a_read_still_waiting_at_the_expiration_times_out_and_keeps_partial_bytes() {
    let started = Instant::now();
    let read_until = started + Duration::from_millis(200);
    let stalled = || stream::iter(pieces(&["abc"])).chain(stream::pending());

    let mut live = body(stalled(), Some(read_until));
    assert_eq!(live.chunk().await, Ok(Some(Bytes::from_static(b"abc"))));
    assert_eq!(live.chunk().await, Err(BodyReadError::TimedOut));
    assert_eq!(started.elapsed(), Duration::from_millis(200));

    let failure = body(stalled(), Some(Instant::now() + Duration::from_millis(200)))
        .collect()
        .await
        .expect_err("timed out");
    assert_eq!(failure, DecodeFailure::new(DecodeFailureKind::Interrupted, Bytes::from_static(b"abc")));
}

#[tokio::test]
async fn a_streaming_decoder_delivers_its_first_item_before_the_body_ends() {
    let (sender, receiver) = mpsc::unbounded_channel::<Result<Bytes, BodyReadError>>();
    let live = body(
        stream::unfold(receiver, |mut receiver| async move { receiver.recv().await.map(|item| (item, receiver)) }),
        None,
    );
    let mut lines = decode_with::<Lines>(live).await.expect("decoder returns before reading");

    sender.send(Ok(Bytes::from_static(b"first\nsec"))).expect("send");
    // The sender is still open, so the body has not ended.
    assert_eq!(lines.next_line().await, Some(Ok("first".to_string())));
    assert!(!sender.is_closed());

    sender.send(Ok(Bytes::from_static(b"ond\n"))).expect("send");
    drop(sender);
    assert_eq!(lines.next_line().await, Some(Ok("second".to_string())));
    assert_eq!(lines.next_line().await, None);
}

#[tokio::test]
async fn a_streaming_decoder_reports_a_later_read_failure_as_an_item() {
    let mut items = pieces(&["one\n"]);
    items.push(Err(BodyReadError::Interrupted));
    let mut lines = decode_with::<Lines>(body(stream::iter(items), None)).await.expect("decoder");
    assert_eq!(lines.next_line().await, Some(Ok("one".to_string())));
    assert_eq!(lines.next_line().await, Some(Err(BodyReadError::Interrupted)));
}

#[tokio::test]
async fn a_two_shape_decoder_returns_either_shape_or_keeps_the_original_bytes() {
    let number = decode_with::<TwoShape>(body(stream::iter(pieces(&["4", "2"])), None)).await;
    assert_eq!(number, Ok(Shape::Number(42)));
    let word = decode_with::<TwoShape>(body(stream::iter(pieces(&["ok"])), None)).await;
    assert_eq!(word, Ok(Shape::Word("ok".to_string())));
    let malformed = decode_with::<TwoShape>(body(stream::iter(pieces(&["<html>"])), None)).await;
    assert_eq!(
        malformed,
        Err(DecodeFailure::new(DecodeFailureKind::Malformed, Bytes::from_static(b"<html>")))
    );
}

#[test]
fn a_live_body_exposes_its_head_to_the_decoder() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/plain; charset=utf-8".parse().expect("value"));
    let live = LiveBody::new(StatusCode::CREATED, headers, stream::iter(pieces(&[])), None);
    assert_eq!(live.status(), StatusCode::CREATED);
    assert_eq!(live.headers().get("content-type").expect("header"), "text/plain; charset=utf-8");
}

#[test]
fn an_endpoint_declares_its_identity_codec_and_request_parts() {
    assert_eq!(<GetItem as EndpointSpec>::ENDPOINT_ID, "get_item");
    assert_eq!(<<GetItem as EndpointSpec>::Api as Api>::NAME, "Fixture");
    assert_eq!(
        <GetItem as EndpointSpec>::ERRORS,
        [ErrorRoute::new(StatusMatch::Range { lo: 400, hi: 599 }, 0)],
        "the default table sends every 4xx and 5xx status to the one failure shape"
    );

    let parts = GetItem {
        id: "7".to_string(),
        tags: vec!["b".to_string(), "a".to_string(), "b".to_string()],
    }
    .into_parts()
    .expect("parts");
    assert_eq!(parts.method, Method::GET);
    assert_eq!(parts.path, "/items/7");
    assert_eq!(
        parts.query,
        [("tag", "b"), ("tag", "a"), ("tag", "b")].map(|(name, value)| (name.to_string(), value.into()))
    );
    assert!(parts.headers.is_empty());
    assert_eq!(parts.body, RequestBody::Empty);

    let refused = GetItem { id: String::new(), tags: Vec::new() }.into_parts().expect_err("invalid");
    assert_eq!(refused.to_string(), "invalid argument: id must not be empty");
}

#[tokio::test(start_paused = true)]
async fn deadline_clones_share_one_expiration() {
    let deadline = Deadline::after(Duration::from_millis(300));
    let options = CallOptions::new().deadline(&deadline);
    let clone = deadline.clone();
    assert_eq!(clone.expires_at(), deadline.expires_at());
    assert_eq!(deadline.remaining(), Duration::from_millis(300));
    assert_eq!(deadline.completed_requests(), 0);
    assert!(!deadline.is_expired());

    tokio::time::advance(Duration::from_millis(100)).await;
    assert_eq!(clone.remaining(), Duration::from_millis(200));
    assert_eq!(clone.elapsed(), Duration::from_millis(100));

    tokio::time::advance(Duration::from_millis(200)).await;
    assert!(deadline.is_expired());
    assert_eq!(clone.remaining(), Duration::ZERO);
    assert!(format!("{options:?}").contains("Deadline"));
}

#[test]
fn an_unrepresentable_deadline_budget_does_not_panic() {
    let deadline = Deadline::after(Duration::MAX);
    assert!(!deadline.is_expired());
    assert!(deadline.remaining() > Duration::from_secs(365 * 24 * 60 * 60));
}
