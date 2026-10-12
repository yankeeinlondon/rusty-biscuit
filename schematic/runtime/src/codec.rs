//! The boundary between the runtime and the codecs that read response bodies.
//!
//! The runtime classifies a response and records its metadata before a codec
//! runs, then hands the codec a [`LiveBody`]: the body as it arrives, not a
//! pre-buffered copy. A buffering codec collects it; a streaming codec can
//! return a value that owns the body and yields items as they arrive.
//!
//! The built-in codecs are [`Json`], [`Text`], [`Binary`], and [`Empty`].

use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
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
/// and the envelope's `elapsed` then measures the time to the response head,
/// not to the end of the body.
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
}

/// The per-request timeout, as an idle bound between reads, and the shared
/// deadline over the whole stream.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StreamBounds {
    pub(crate) idle: Option<Duration>,
    pub(crate) deadline: Option<Instant>,
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
        }
    }

    pub(crate) fn with_stream_bounds(mut self, bounds: StreamBounds) -> Self {
        self.stream_bounds = bounds;
        self
    }

    /// Switches to the bounds of a streamed body, for a codec whose output
    /// keeps reading after `decode` returns.
    ///
    /// From then on each read must arrive within the client's per-request
    /// timeout of the previous one (an idle bound), and the shared deadline,
    /// when the send has one, bounds the whole stream. The send's single
    /// expiration no longer applies, so a long stream that keeps delivering
    /// is not cut off.
    #[must_use]
    pub fn into_streaming(mut self) -> Self {
        self.streaming = true;
        self
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
    /// ## Errors
    ///
    /// Returns the stream's [`BodyReadError`], or
    /// [`BodyReadError::TimedOut`] when the expiration passes first.
    pub async fn chunk(&mut self) -> Result<Option<Bytes>, BodyReadError> {
        let read_until = if self.streaming {
            let idle = self.stream_bounds.idle.and_then(|idle| Instant::now().checked_add(idle));
            match (idle, self.stream_bounds.deadline) {
                (Some(idle), Some(deadline)) => Some(idle.min(deadline)),
                (idle, deadline) => idle.or(deadline),
            }
        } else {
            self.read_until
        };
        let stream = &mut self.stream;
        let next = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx));
        let item = match read_until {
            Some(read_until) => tokio::time::timeout_at(read_until, next)
                .await
                .map_err(|_| BodyReadError::TimedOut)?,
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
    /// The send's expiration passed while waiting for the body.
    TimedOut,
}

impl fmt::Display for BodyReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Interrupted => "response body was interrupted",
            Self::TimedOut => "response body read timed out",
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
}

impl DecodeFailure {
    /// A failure of `kind` keeping `retained`.
    pub fn new(kind: DecodeFailureKind, retained: Bytes) -> Self {
        Self { kind, retained }
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
