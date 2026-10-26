//! The boundary between the runtime and the codecs that read response bodies.
//!
//! The runtime classifies a response and records its metadata before a codec
//! runs, then hands the codec a [`LiveBody`]: the body as it arrives, not a
//! pre-buffered copy. A buffering codec collects it; a streaming codec can
//! return a value that owns the body and yields items as they arrive.
//!
//! The built-in buffering codecs are [`Json`], [`Text`], [`Binary`], and
//! [`Empty`]; the streaming codecs live in [`crate::stream`], the multi-shape
//! codec in [`crate::shapes`], and the GraphQL codec in [`crate::graphql`].

use std::any::Any;
use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use http::{HeaderMap, StatusCode};
use tokio::time::Instant;

use crate::hooks::{HookStage, SupportsHooks};

/// Decodes a live response body into an endpoint's success type.
///
/// A codec must keep evidence: on a read failure it returns the bytes that
/// arrived ([`DecodeFailureKind::Interrupted`]), and on a parse failure the
/// original bytes ([`DecodeFailureKind::Malformed`]). A naked library error
/// cannot satisfy this, which is why decoders return [`DecodeFailure`].
///
/// `Output` may own the [`LiveBody`] and read it after `decode` returns, as a
/// stream does. Such an output is `Send` but need not be `Clone` or `Sync`,
/// and the envelope's `elapsed`, which stops when decoding returns, then
/// measures about the time to the response head, not to the end of the body.
pub trait Decode: 'static {
    /// The decoded value.
    type Output: Send + 'static;

    /// Reads `body` into the output.
    fn decode(body: LiveBody) -> impl Future<Output = Result<Self::Output, DecodeFailure>> + Send;

    /// Reads a 2xx `body` into the output, applying the client's response
    /// `hooks`; the runtime calls this rather than [`decode`](Self::decode).
    ///
    /// The default ignores `hooks`. Only a codec that implements
    /// [`SupportsHooks`] overrides it, and it must then
    /// report a hook's refusal as [`DecodeFailureKind::Hook`] with the
    /// original bytes.
    fn decode_hooked(
        body: LiveBody,
        hooks: HookStage<'_>,
    ) -> impl Future<Output = Result<Self::Output, DecodeFailure>> + Send {
        let _ = hooks;
        Self::decode(body)
    }
}

/// A response body as it arrives, with the head it belongs to.
///
/// Reads are bounded by the send's expiration: a read still waiting when it
/// passes fails with [`BodyReadError::TimedOut`]. A streaming codec switches
/// to [stream bounds](Self::into_streaming) instead.
pub struct LiveBody {
    status: StatusCode,
    headers: HeaderMap,
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, BodyReadError>> + Send>>,
    read_until: Option<Instant>,
    /// The bounds that apply after [`into_streaming`](Self::into_streaming).
    stream_bounds: StreamBounds,
    streaming: bool,
    /// When the idle bound was last restarted: the head's arrival, then each
    /// item a streaming codec yields.
    idle_anchor: Instant,
}

/// The default per-frame byte limit of the SSE and NDJSON codecs: 16 MiB.
pub const DEFAULT_MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// The per-request timeout, as an idle bound between items, the shared
/// deadline over the whole stream, and the per-frame byte limit.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StreamBounds {
    pub(crate) idle: Option<Duration>,
    pub(crate) deadline: Option<Instant>,
    pub(crate) max_frame_bytes: usize,
}

impl Default for StreamBounds {
    fn default() -> Self {
        Self {
            idle: None,
            deadline: None,
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
        }
    }
}

impl LiveBody {
    /// A body read from `stream`, belonging to a response with `status` and
    /// `headers`, whose reads must finish by `read_until`.
    ///
    /// The runtime builds one from each response; a codec's own tests can
    /// build one from any stream.
    pub fn new<S>(status: StatusCode, headers: HeaderMap, stream: S, read_until: Option<Instant>) -> Self
    where
        S: Stream<Item = Result<Bytes, BodyReadError>> + Send + 'static,
    {
        Self {
            status,
            headers,
            stream: Box::pin(stream),
            read_until,
            stream_bounds: StreamBounds::default(),
            streaming: false,
            idle_anchor: Instant::now(),
        }
    }

    /// A body whose bytes have already been read, for a codec that decides
    /// how to decode only after reading them.
    pub(crate) fn buffered(status: StatusCode, headers: HeaderMap, bytes: Bytes) -> Self {
        let stream = futures_util::stream::iter((!bytes.is_empty()).then_some(Ok(bytes)));
        Self::new(status, headers, stream, None)
    }

    /// Applies the client's stream bounds; the idle bound starts now, when
    /// the head has arrived.
    pub(crate) fn with_stream_bounds(mut self, bounds: StreamBounds) -> Self {
        self.stream_bounds = bounds;
        self.idle_anchor = Instant::now();
        self
    }

    /// Sets the per-frame byte limit the SSE and NDJSON codecs enforce
    /// (default [`DEFAULT_MAX_FRAME_BYTES`]); a client applies its
    /// [`max_frame_bytes`](crate::Client::max_frame_bytes) setting.
    #[must_use]
    pub fn with_max_frame_bytes(mut self, limit: usize) -> Self {
        self.stream_bounds.max_frame_bytes = limit;
        self
    }

    /// The per-frame byte limit a framing codec enforces.
    pub fn max_frame_bytes(&self) -> usize {
        self.stream_bounds.max_frame_bytes
    }

    /// Switches to the bounds of a streamed body, for a codec whose output
    /// keeps reading after `decode` returns.
    ///
    /// From then on a read fails with [`BodyReadError::TimedOut`] once the
    /// client's per-request timeout has passed since the head arrived or
    /// since the codec last called [`mark_item`](Self::mark_item) (an idle
    /// bound between items, not between reads), and with
    /// [`BodyReadError::DeadlineExceeded`] once the send's shared deadline
    /// passes. The send's single expiration no longer applies, so a long
    /// stream that keeps delivering is not cut off.
    #[must_use]
    pub fn into_streaming(mut self) -> Self {
        self.streaming = true;
        self
    }

    /// Restarts the idle bound of a streamed body: a streaming codec calls it
    /// each time it yields an item. Bytes that arrive without completing an
    /// item do not restart it.
    pub fn mark_item(&mut self) {
        self.idle_anchor = Instant::now();
    }

    /// The response status.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// The response headers as received, **not redacted**; read them to
    /// decode, never store them.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The next piece of the body, or `None` at its end.
    ///
    /// Bytes the server has already sent are returned even when the caller
    /// polls after a bound has passed: the read is attempted before its
    /// timer.
    ///
    /// ## Errors
    ///
    /// Returns the stream's [`BodyReadError`], or the bound that passed
    /// first: [`BodyReadError::TimedOut`] for the expiration or idle bound,
    /// [`BodyReadError::DeadlineExceeded`] for a streamed body's shared
    /// deadline.
    pub async fn chunk(&mut self) -> Result<Option<Bytes>, BodyReadError> {
        let (read_until, expired) = if self.streaming {
            let idle = self.stream_bounds.idle.and_then(|idle| self.idle_anchor.checked_add(idle));
            match (idle, self.stream_bounds.deadline) {
                (Some(idle), Some(deadline)) if deadline <= idle => (Some(deadline), BodyReadError::DeadlineExceeded),
                (Some(idle), _) => (Some(idle), BodyReadError::TimedOut),
                (None, Some(deadline)) => (Some(deadline), BodyReadError::DeadlineExceeded),
                (None, None) => (None, BodyReadError::TimedOut),
            }
        } else {
            (self.read_until, BodyReadError::TimedOut)
        };
        let stream = &mut self.stream;
        let next = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx));
        let item = match read_until {
            Some(read_until) => tokio::time::timeout_at(read_until, next).await.map_err(|_| expired)?,
            None => next.await,
        };
        item.transpose()
    }

    /// Reads the whole body.
    ///
    /// ## Errors
    ///
    /// Returns [`DecodeFailureKind::Interrupted`] with every byte that
    /// arrived when a read fails or times out.
    pub async fn collect(mut self) -> Result<Bytes, DecodeFailure> {
        let mut received = BytesMut::new();
        loop {
            match self.chunk().await {
                Ok(Some(piece)) => received.extend_from_slice(&piece),
                Ok(None) => return Ok(received.freeze()),
                Err(_) => return Err(DecodeFailure::new(DecodeFailureKind::Interrupted, received.freeze())),
            }
        }
    }
}

impl fmt::Debug for LiveBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveBody")
            .field("status", &self.status)
            .field("read_until", &self.read_until)
            .field("streaming", &self.streaming)
            .finish_non_exhaustive()
    }
}

/// Why reading a body failed. It carries no library error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BodyReadError {
    /// The connection failed or closed before the body was complete.
    Interrupted,
    /// The send's expiration, or a streamed body's idle bound, passed while
    /// waiting for the body.
    TimedOut,
    /// A streamed body's shared deadline passed while waiting for it.
    DeadlineExceeded,
}

impl fmt::Display for BodyReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Interrupted => "response body was interrupted",
            Self::TimedOut => "response body read timed out",
            Self::DeadlineExceeded => "shared deadline expired while reading the response body",
        })
    }
}

impl std::error::Error for BodyReadError {}

/// A codec failure with the bytes that are kept as evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeFailure {
    /// What failed.
    pub kind: DecodeFailureKind,
    /// The bytes received before an interruption, or the original bytes of a
    /// body that did not parse.
    pub retained: Bytes,
    /// The typed failure of a [`DecodeFailureKind::Declared`] failure.
    declared: Option<DeclaredPayload>,
}

impl DecodeFailure {
    /// A failure of `kind` keeping `retained`.
    pub fn new(kind: DecodeFailureKind, retained: Bytes) -> Self {
        Self { kind, retained, declared: None }
    }

    /// A 2xx body that carries a failure the endpoint declares, such as a
    /// GraphQL `errors` array, keeping `retained`.
    ///
    /// The runtime hands `failure` to the endpoint's
    /// [`ErrorBody::from_declared`](crate::ErrorBody::from_declared); a
    /// failure type that does not accept it makes the reply
    /// [`ErrorKind::MalformedSuccess`](crate::ErrorKind::MalformedSuccess).
    pub fn declared<F: Any + Send>(failure: F, retained: Bytes) -> Self {
        Self {
            kind: DecodeFailureKind::Declared,
            retained,
            declared: Some(DeclaredPayload(Arc::new(Mutex::new(Some(Box::new(failure)))))),
        }
    }

    /// Takes the typed failure of a [`DecodeFailureKind::Declared`] failure;
    /// `None` for every other kind and once taken.
    pub(crate) fn take_declared(&mut self) -> Option<Box<dyn Any + Send>> {
        let payload = self.declared.take()?;
        let mut slot = payload.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        slot.take()
    }
}

/// The type-erased failure of a [`DecodeFailure::declared`] failure. Clones
/// share it, so it is taken once; equality is identity.
#[derive(Clone)]
struct DeclaredPayload(Arc<Mutex<Option<Box<dyn Any + Send>>>>);

impl PartialEq for DeclaredPayload {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for DeclaredPayload {}

impl fmt::Debug for DeclaredPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeclaredPayload(..)")
    }
}

/// What a codec failed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DecodeFailureKind {
    /// The body could not be read to its end.
    Interrupted,
    /// The body was read but did not decode.
    Malformed,
    /// A response hook rejected the body.
    Hook,
    /// No declared shape of a multi-shape endpoint matches the reply's
    /// status, media type, or JSON tag.
    UnexpectedContentType,
    /// The 2xx body carries a failure the endpoint declares; built by
    /// [`DecodeFailure::declared`].
    Declared,
}

/// Decodes a JSON body into `T`.
///
/// A successful parse is the only validity check: a body that parses is
/// accepted whatever its `Content-Type`, and one that does not is
/// [`DecodeFailureKind::Malformed`] with the original bytes.
///
/// It is the only built-in codec that runs the response hooks
/// ([`SupportsHooks`]).
pub struct Json<T>(PhantomData<fn() -> T>);

impl<T> SupportsHooks for Json<T> where T: serde::de::DeserializeOwned + Send + 'static {}

impl<T> Decode for Json<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Output = T;

    async fn decode(body: LiveBody) -> Result<T, DecodeFailure> {
        let bytes = body.collect().await?;
        serde_json::from_slice(&bytes).map_err(|_| DecodeFailure::new(DecodeFailureKind::Malformed, bytes))
    }

    async fn decode_hooked(body: LiveBody, hooks: HookStage<'_>) -> Result<T, DecodeFailure> {
        if hooks.is_empty() {
            return Self::decode(body).await;
        }
        let bytes = body.collect().await?;
        let malformed = || DecodeFailure::new(DecodeFailureKind::Malformed, bytes.clone());
        let hook_failed = || DecodeFailure::new(DecodeFailureKind::Hook, bytes.clone());
        let mut value: T = if hooks.has_pre_response_json() {
            // The global hook sees only a body that parses as JSON.
            let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| malformed())?;
            let json = hooks.pre_response_json(json).map_err(|_| hook_failed())?;
            serde_json::from_value(json).map_err(|_| malformed())?
        } else {
            serde_json::from_slice(&bytes).map_err(|_| malformed())?
        };
        hooks.mutate(&mut value).map_err(|_| hook_failed())?;
        Ok(value)
    }
}

/// Decodes any body as text, replacing invalid UTF-8 sequences; it accepts
/// every content type.
pub struct Text;

impl Decode for Text {
    type Output = String;

    async fn decode(body: LiveBody) -> Result<String, DecodeFailure> {
        let bytes = body.collect().await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// Returns any body as bytes; it accepts every content type.
pub struct Binary;

impl Decode for Binary {
    type Output = Bytes;

    async fn decode(body: LiveBody) -> Result<Bytes, DecodeFailure> {
        body.collect().await
    }
}

/// Reads the body to its end and ignores its bytes, for an endpoint that
/// declares no response body.
pub struct Empty;

impl Decode for Empty {
    type Output = ();

    async fn decode(body: LiveBody) -> Result<(), DecodeFailure> {
        body.collect().await.map(drop)
    }
}
