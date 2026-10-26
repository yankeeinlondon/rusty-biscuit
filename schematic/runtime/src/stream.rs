//! Streaming codecs: [`Sse`] and [`Ndjson`] decode one JSON item per frame,
//! and [`ByteStream`] yields the body's bytes as they arrive.
//!
//! A stream owns the response body and reads it only when the caller polls:
//! there is no background task and no copy of the whole body. Dropping it
//! releases the body. It is `Send` and stays usable after the client that
//! sent it is dropped.
//!
//! Every stream ends after at most one terminal [`StreamError`]:
//! [`EndedEarly`](StreamError::EndedEarly),
//! [`Interrupted`](StreamError::Interrupted),
//! [`IdleTimeout`](StreamError::IdleTimeout),
//! [`DeadlineExceeded`](StreamError::DeadlineExceeded), or
//! [`FrameLimitExceeded`](StreamError::FrameLimitExceeded). A
//! [`Malformed`](StreamError::Malformed) item is not terminal: the stream
//! continues at the next frame, unless the malformed frame was a kept
//! terminator.
//!
//! ## Framing
//!
//! HTTP chunks are not item boundaries. SSE follows the
//! [event-stream interpretation](https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation):
//! LF, CRLF, and CR line endings, comments, joined `data:` lines, and a
//! leading byte order mark; an event cut off by the end of the body is
//! discarded, and there is no reconnection. NDJSON accepts LF and CRLF, skips
//! blank lines, and decodes a final line that has no newline.
//!
//! ## End conditions
//!
//! The [`StreamEnd`] marker fixes the end condition at compile time. A
//! terminator is matched before its frame is decoded, so OpenAI's `[DONE]`
//! is never a malformed item. By default the terminator is consumed; a
//! marker with [`KEEP`](StreamEnd::KEEP) delivers it as the last item.

use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Buf, Bytes, BytesMut};
use futures_core::Stream;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::codec::{BodyReadError, Decode, DecodeFailure, LiveBody};

/// How many bytes of an oversized frame
/// [`StreamError::FrameLimitExceeded`] keeps.
pub const FRAME_LIMIT_PREFIX_BYTES: usize = 1024;

/// A stream item that is not a decoded value.
///
/// Its `Debug` prints raw provider bytes; its `Display` never does.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StreamError {
    /// A frame that did not decode as the item type; the stream continues.
    Malformed {
        /// The frame's bytes: an SSE event's joined data, or an NDJSON line.
        raw: Bytes,
    },
    /// The body ended cleanly before the declared terminator arrived.
    EndedEarly,
    /// The connection broke mid-stream.
    Interrupted {
        /// The bytes of the frame that was being assembled.
        pending: Bytes,
    },
    /// No item arrived within the per-request timeout of the previous item,
    /// or of the response head for the first.
    IdleTimeout,
    /// The call's shared deadline passed.
    DeadlineExceeded,
    /// A frame grew past the client's frame limit.
    FrameLimitExceeded {
        /// The limit in bytes.
        limit: usize,
        /// The frame's first bytes, at most [`FRAME_LIMIT_PREFIX_BYTES`].
        prefix: Bytes,
    },
}

impl StreamError {
    /// Whether the stream ends after this item; only
    /// [`Malformed`](Self::Malformed) lets it continue.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Malformed { .. })
    }
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { raw } => write!(f, "stream item could not be decoded ({} bytes)", raw.len()),
            Self::EndedEarly => f.write_str("stream ended before its terminator"),
            Self::Interrupted { pending } => write!(f, "stream was interrupted ({} pending bytes)", pending.len()),
            Self::IdleTimeout => f.write_str("stream timed out waiting for the next item"),
            Self::DeadlineExceeded => f.write_str("shared deadline expired during the stream"),
            Self::FrameLimitExceeded { limit, .. } => write!(f, "stream frame exceeded the {limit}-byte limit"),
        }
    }
}

impl std::error::Error for StreamError {}

impl From<BodyReadError> for StreamError {
    fn from(error: BodyReadError) -> Self {
        match error {
            BodyReadError::TimedOut => Self::IdleTimeout,
            BodyReadError::DeadlineExceeded => Self::DeadlineExceeded,
            _ => Self::Interrupted { pending: Bytes::new() },
        }
    }
}

/// A JSON scalar compared by a stream terminator or a JSON tag.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Scalar {
    /// A string.
    String(&'static str),
    /// A boolean.
    Bool(bool),
    /// A signed integer.
    Integer(i64),
    /// An unsigned integer.
    Unsigned(u64),
    /// A floating-point number.
    Float(f64),
}

impl Scalar {
    /// Whether `value` is this scalar; numbers compare by value.
    pub fn matches(self, value: &Value) -> bool {
        match self {
            Self::String(expected) => value.as_str() == Some(expected),
            Self::Bool(expected) => value.as_bool() == Some(expected),
            Self::Integer(expected) => value.as_i64() == Some(expected),
            Self::Unsigned(expected) => value.as_u64() == Some(expected),
            Self::Float(expected) => value.as_f64() == Some(expected),
        }
    }
}

/// When a stream ends; mirrors the definition's stream end.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum EndCondition {
    /// An SSE event whose joined data equals this text (`[DONE]`).
    DataLineEquals(&'static str),
    /// An SSE event of this name (`message_stop`).
    EventNamed(&'static str),
    /// An item whose field at `path` (object keys, no array fan-out) equals
    /// `value`; a missing field does not match.
    FieldEquals {
        /// The object keys leading to the field.
        path: &'static [&'static str],
        /// The terminating value.
        value: Scalar,
    },
    /// The server closing the body; never [`StreamError::EndedEarly`].
    ConnectionClose,
}

/// A marker type fixing a stream's end condition; generated code declares one
/// per stream that has a terminator.
pub trait StreamEnd: Send + 'static {
    /// The end condition.
    const CONDITION: EndCondition;
    /// Whether the terminator is delivered as the last item rather than
    /// consumed.
    const KEEP: bool = false;
}

/// The end condition of a stream that ends when the server closes the body.
#[derive(Debug, Clone, Copy)]
pub struct UntilClose;

impl StreamEnd for UntilClose {
    const CONDITION: EndCondition = EndCondition::ConnectionClose;
}

/// Decodes a `text/event-stream` body into an [`ItemStream`] of `T`, one item
/// per event's joined data, ending as `E` declares.
///
/// It does not implement [`SupportsHooks`](crate::SupportsHooks): response
/// hooks never run on a stream.
pub struct Sse<T, E = UntilClose>(PhantomData<fn() -> (T, E)>);

/// Decodes a newline-delimited JSON body into an [`ItemStream`] of `T`, one
/// item per line, ending as `E` declares.
///
/// It does not implement [`SupportsHooks`](crate::SupportsHooks).
pub struct Ndjson<T, E = UntilClose>(PhantomData<fn() -> (T, E)>);

impl<T, E> Decode for Sse<T, E>
where
    T: DeserializeOwned + Send + 'static,
    E: StreamEnd,
{
    type Output = ItemStream<T>;

    async fn decode(body: LiveBody) -> Result<ItemStream<T>, DecodeFailure> {
        Ok(ItemStream::new::<SseFramer, E>(body))
    }
}

impl<T, E> Decode for Ndjson<T, E>
where
    T: DeserializeOwned + Send + 'static,
    E: StreamEnd,
{
    type Output = ItemStream<T>;

    async fn decode(body: LiveBody) -> Result<ItemStream<T>, DecodeFailure> {
        Ok(ItemStream::new::<NdjsonFramer, E>(body))
    }
}

/// A stream of decoded items, each `Ok(item)` or a [`StreamError`]. Once it
/// returns `None` it keeps returning `None`.
///
/// A provider's error event that decodes as `T` is `Ok`: the item decoded,
/// which does not mean the provider's operation succeeded.
pub struct ItemStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>,
}

impl<T: DeserializeOwned + Send + 'static> ItemStream<T> {
    fn new<F: Framer, E: StreamEnd>(body: LiveBody) -> Self {
        let state = Framed::<T, F, E> {
            body: body.into_streaming(),
            framer: F::default(),
            eof: false,
            flushed: false,
            finished: false,
            item: PhantomData,
        };
        let inner = futures_util::stream::unfold(state, |mut state| async move {
            let item = state.next().await?;
            Some((item, state))
        });
        Self { inner: Box::pin(inner.fuse()) }
    }
}

impl<T> ItemStream<T> {
    /// The next item, or `None` once the stream has ended.
    pub async fn next(&mut self) -> Option<Result<T, StreamError>> {
        std::future::poll_fn(|cx| self.inner.as_mut().poll_next(cx)).await
    }
}

impl<T> Stream for ItemStream<T> {
    type Item = Result<T, StreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl<T> fmt::Debug for ItemStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemStream").finish_non_exhaustive()
    }
}

/// A raw byte stream: both the codec and its output. Chunks keep the body's
/// byte order but have no stable size; it assembles no frame, so the frame
/// limit does not apply.
pub struct ByteStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>,
}

impl Decode for ByteStream {
    type Output = ByteStream;

    async fn decode(body: LiveBody) -> Result<ByteStream, DecodeFailure> {
        let inner = futures_util::stream::unfold(Some(body.into_streaming()), |body| async move {
            let mut body = body?;
            loop {
                match body.chunk().await {
                    Ok(Some(chunk)) if chunk.is_empty() => {}
                    Ok(Some(chunk)) => {
                        body.mark_item();
                        return Some((Ok(chunk), Some(body)));
                    }
                    Ok(None) => return None,
                    Err(error) => return Some((Err(error.into()), None)),
                }
            }
        });
        Ok(ByteStream { inner: Box::pin(inner.fuse()) })
    }
}

impl ByteStream {
    /// The next chunk, or `None` once the body has ended.
    pub async fn next(&mut self) -> Option<Result<Bytes, StreamError>> {
        std::future::poll_fn(|cx| self.inner.as_mut().poll_next(cx)).await
    }
}

impl Stream for ByteStream {
    type Item = Result<Bytes, StreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl fmt::Debug for ByteStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ByteStream").finish_non_exhaustive()
    }
}

/// One assembled frame.
struct Frame {
    /// An SSE event's name; `None` for NDJSON and unnamed events.
    name: Option<Bytes>,
    /// The frame's data: an SSE event's joined data or an NDJSON line.
    data: Bytes,
    /// Whether an SSE event had any `data:` line; always true for NDJSON.
    has_data: bool,
}

/// Splits a byte stream into frames.
trait Framer: Default + Send + 'static {
    fn push(&mut self, bytes: &[u8]);

    /// The next complete frame, a frame limit error, or `None` until more
    /// bytes arrive. `eof` says no more bytes will.
    fn next_frame(&mut self, limit: usize, eof: bool) -> Option<Result<Frame, StreamError>>;

    /// The last frame at a clean end of the body, if the framing completes
    /// one there.
    fn finish(&mut self) -> Option<Frame>;

    /// The bytes of the frame being assembled.
    fn pending(&self) -> Bytes;
}

/// The position and length of the first line ending in `buffer` from `from`,
/// or `None` while it may still be incomplete. A CR at the end of a buffer
/// waits for a possible LF unless `eof`.
fn line_end(buffer: &[u8], from: usize, eof: bool) -> Option<(usize, usize)> {
    let at = from + buffer[from..].iter().position(|byte| matches!(byte, b'\r' | b'\n'))?;
    match (buffer[at], buffer.get(at + 1)) {
        (b'\n', _) => Some((at, 1)),
        (_, Some(b'\n')) => Some((at, 2)),
        (_, Some(_)) => Some((at, 1)),
        (_, None) if eof => Some((at, 1)),
        (_, None) => None,
    }
}

fn limit_exceeded(limit: usize, parts: &[&[u8]]) -> StreamError {
    let mut prefix = BytesMut::new();
    for part in parts {
        let room = FRAME_LIMIT_PREFIX_BYTES - prefix.len();
        prefix.extend_from_slice(&part[..part.len().min(room)]);
    }
    StreamError::FrameLimitExceeded {
        limit,
        prefix: prefix.freeze(),
    }
}

const BOM: &[u8] = b"\xEF\xBB\xBF";

#[derive(Default)]
struct SseFramer {
    buffer: BytesMut,
    /// How far `buffer` is known to hold no complete line.
    scanned: usize,
    /// The current event's joined data, each line followed by LF.
    data: BytesMut,
    name: Option<Bytes>,
    has_data: bool,
    bom_checked: bool,
}

impl Framer for SseFramer {
    fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    fn next_frame(&mut self, limit: usize, eof: bool) -> Option<Result<Frame, StreamError>> {
        if !self.bom_checked {
            if self.buffer.len() < BOM.len() && BOM.starts_with(&self.buffer) && !eof {
                return None;
            }
            if self.buffer.starts_with(BOM) {
                self.buffer.advance(BOM.len());
            }
            self.bom_checked = true;
        }
        loop {
            let Some((at, ending)) = line_end(&self.buffer, self.scanned, eof) else {
                self.scanned = self.buffer.len().saturating_sub(1).max(self.scanned);
                if self.data.len() + self.buffer.len() > limit {
                    return Some(Err(limit_exceeded(limit, &[&self.data, &self.buffer])));
                }
                return None;
            };
            let line = self.buffer.split_to(at).freeze();
            self.buffer.advance(ending);
            self.scanned = 0;
            if line.is_empty() {
                if !self.has_data && self.name.is_none() {
                    continue;
                }
                let mut data = std::mem::take(&mut self.data);
                if data.last() == Some(&b'\n') {
                    data.truncate(data.len() - 1);
                }
                let frame = Frame {
                    name: self.name.take(),
                    data: data.freeze(),
                    has_data: std::mem::take(&mut self.has_data),
                };
                return Some(Ok(frame));
            }
            if line[0] == b':' {
                continue;
            }
            let (field, value) = match line.iter().position(|byte| *byte == b':') {
                Some(colon) => {
                    let value = &line[colon + 1..];
                    (&line[..colon], value.strip_prefix(b" ").unwrap_or(value))
                }
                None => (&line[..], &[][..]),
            };
            match field {
                b"data" => {
                    self.data.extend_from_slice(value);
                    self.data.extend_from_slice(b"\n");
                    self.has_data = true;
                    if self.data.len() > limit {
                        return Some(Err(limit_exceeded(limit, &[&self.data])));
                    }
                }
                b"event" => self.name = Some(Bytes::copy_from_slice(value)),
                // `id` and `retry` serve reconnection, which is not offered.
                _ => {}
            }
        }
    }

    fn finish(&mut self) -> Option<Frame> {
        // An event the body cut off before its blank line is discarded.
        None
    }

    fn pending(&self) -> Bytes {
        let mut pending = BytesMut::from(&self.data[..]);
        pending.extend_from_slice(&self.buffer);
        pending.freeze()
    }
}

#[derive(Default)]
struct NdjsonFramer {
    buffer: BytesMut,
    scanned: usize,
}

impl NdjsonFramer {
    fn line(mut line: Bytes) -> Option<Frame> {
        if line.last() == Some(&b'\r') {
            line.truncate(line.len() - 1);
        }
        (!line.trim_ascii().is_empty()).then_some(Frame {
            name: None,
            data: line,
            has_data: true,
        })
    }
}

impl Framer for NdjsonFramer {
    fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    fn next_frame(&mut self, limit: usize, _eof: bool) -> Option<Result<Frame, StreamError>> {
        loop {
            let Some(at) = self.buffer[self.scanned..].iter().position(|byte| *byte == b'\n').map(|at| at + self.scanned) else {
                self.scanned = self.buffer.len();
                if self.buffer.len() > limit {
                    return Some(Err(limit_exceeded(limit, &[&self.buffer])));
                }
                return None;
            };
            let line = self.buffer.split_to(at).freeze();
            self.buffer.advance(1);
            self.scanned = 0;
            if line.len() > limit {
                return Some(Err(limit_exceeded(limit, &[&line])));
            }
            if let Some(frame) = Self::line(line) {
                return Some(Ok(frame));
            }
        }
    }

    fn finish(&mut self) -> Option<Frame> {
        self.scanned = 0;
        Self::line(self.buffer.split().freeze())
    }

    fn pending(&self) -> Bytes {
        Bytes::copy_from_slice(&self.buffer)
    }
}

/// The state behind an [`ItemStream`].
struct Framed<T, F, E> {
    body: LiveBody,
    framer: F,
    /// The body has ended cleanly.
    eof: bool,
    /// The framer's last frame has been taken at the end of the body.
    flushed: bool,
    /// No further item will be produced.
    finished: bool,
    item: PhantomData<fn() -> (T, E)>,
}

/// What one frame produces.
enum Step<T> {
    Item(Result<T, StreamError>),
    /// The terminator, with the item a kept terminator delivers.
    End(Option<Result<T, StreamError>>),
    Skip,
}

impl<T: DeserializeOwned, F: Framer, E: StreamEnd> Framed<T, F, E> {
    async fn next(&mut self) -> Option<Result<T, StreamError>> {
        if self.finished {
            return None;
        }
        loop {
            let limit = self.body.max_frame_bytes();
            let frame = match self.framer.next_frame(limit, self.eof) {
                Some(Ok(frame)) => Some(frame),
                Some(Err(error)) => return self.end(Some(Err(error))),
                None if self.eof && !self.flushed => {
                    self.flushed = true;
                    self.framer.finish()
                }
                None if self.eof => {
                    let ended_early = !matches!(E::CONDITION, EndCondition::ConnectionClose);
                    return self.end(ended_early.then_some(Err(StreamError::EndedEarly)));
                }
                None => None,
            };
            if let Some(frame) = frame {
                match Self::step(frame) {
                    Step::Item(item) => {
                        self.body.mark_item();
                        return Some(item);
                    }
                    Step::End(item) => return self.end(item),
                    Step::Skip => continue,
                }
            }
            if self.eof {
                continue;
            }
            match self.body.chunk().await {
                Ok(Some(bytes)) => self.framer.push(&bytes),
                Ok(None) => self.eof = true,
                Err(BodyReadError::TimedOut) => return self.end(Some(Err(StreamError::IdleTimeout))),
                Err(BodyReadError::DeadlineExceeded) => return self.end(Some(Err(StreamError::DeadlineExceeded))),
                Err(_) => {
                    let pending = self.framer.pending();
                    return self.end(Some(Err(StreamError::Interrupted { pending })));
                }
            }
        }
    }

    fn end(&mut self, item: Option<Result<T, StreamError>>) -> Option<Result<T, StreamError>> {
        self.finished = true;
        item
    }

    fn step(frame: Frame) -> Step<T> {
        let terminator = match E::CONDITION {
            EndCondition::DataLineEquals(text) => frame.has_data && frame.data == text.as_bytes(),
            EndCondition::EventNamed(name) => frame.name.as_deref() == Some(name.as_bytes()),
            _ => false,
        };
        if terminator {
            return Step::End(E::KEEP.then(|| decode(&frame.data)));
        }
        if !frame.has_data {
            return Step::Skip;
        }
        let EndCondition::FieldEquals { path, value } = E::CONDITION else {
            return Step::Item(decode(&frame.data));
        };
        let Ok(json) = serde_json::from_slice::<Value>(&frame.data) else {
            return Step::Item(Err(StreamError::Malformed { raw: frame.data }));
        };
        let ends = lookup(&json, path).is_some_and(|field| value.matches(field));
        let item = || serde_json::from_value(json).map_err(|_| StreamError::Malformed { raw: frame.data.clone() });
        match (ends, E::KEEP) {
            (false, _) => Step::Item(item()),
            (true, true) => Step::End(Some(item())),
            (true, false) => Step::End(None),
        }
    }
}

fn decode<T: DeserializeOwned>(data: &Bytes) -> Result<T, StreamError> {
    serde_json::from_slice(data).map_err(|_| StreamError::Malformed { raw: data.clone() })
}

fn lookup<'v>(value: &'v Value, path: &[&str]) -> Option<&'v Value> {
    path.iter().try_fold(value, |node, key| node.as_object()?.get(*key))
}
