//! Stand-in for the generated `shared.rs`: the extended endpoint trait, the
//! `Response<T, E>` envelope, `TransportError`, decoders, redaction, and the
//! two existing hook mechanisms.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::de::DeserializeOwned;

pub const REDACTED: &str = "[redacted]";

// ---------------------------------------------------------------------------
// The endpoint trait (today: `EndpointSpec { type Response; ENDPOINT_ID }`)
// ---------------------------------------------------------------------------

/// Every generated request type implements this. Associated type defaults are
/// unstable, so every impl states all three types.
///
/// `Decode` is the *how*: a codec marker (`Json<T>`, `Text`, `Sse<T>`, ...)
/// or a generated multi-shape enum. Its `Output` must equal `Success`, and it
/// must be able to produce the endpoint's `Failure` (only `GraphQl<D>` ever
/// produces a declared failure from a 2xx), so a mismatched triple does not
/// compile.
pub trait Endpoint: Send + Sized + 'static {
    type Success: Send + 'static;
    type Failure: ErrorBody;
    type Decode: Decode<Self::Failure, Output = Self::Success>;
    const ENDPOINT_ID: &'static str;
    /// Inclusive status ranges whose body decodes as `Failure`. Associated
    /// *const* defaults are stable (only associated *type* defaults are not).
    const DECLARED_ERROR_STATUSES: &'static [(u16, u16)] = &[(400, 599)];
}

/// A declared error body. Which statuses are declared is the endpoint's
/// business (`Endpoint::DECLARED_ERROR_STATUSES`); this only decodes.
pub trait ErrorBody: Sized + Send + 'static {
    /// `false` only for [`NoDeclaredError`].
    const DECLARED: bool = true;
    fn decode(body: &[u8]) -> Result<Self, DecodeError>;
}

/// The `Failure` type of an endpoint that declares no error body.
/// Uninhabited: `ErrorKind::Declared(NoDeclaredError)` cannot exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoDeclaredError {}

impl ErrorBody for NoDeclaredError {
    const DECLARED: bool = false;
    fn decode(_body: &[u8]) -> Result<Self, DecodeError> {
        Err(DecodeError {
            message: "no error body is declared".into(),
        })
    }
}

/// Helper the generator would call from each generated error type's impl.
pub fn decode_json_error<T: DeserializeOwned>(body: &[u8]) -> Result<T, DecodeError> {
    serde_json::from_slice(body).map_err(DecodeError::from)
}

#[derive(Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct DecodeError {
    pub message: String,
}

impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DecodeError({:?})", self.message)
    }
}

impl From<serde_json::Error> for DecodeError {
    fn from(e: serde_json::Error) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Credentials and redaction
// ---------------------------------------------------------------------------

/// A credential value. Never printed; only `expose` (crate-private) reads it.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret({REDACTED})")
    }
}

/// Which credential source was used. Never the value (D11, D21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    PerCall,
    ClientLevel,
    EnvVar(String),
    Anonymous,
}

/// Built-in redaction list (D21); the definition can add more names.
const SECRET_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "api-key",
    "private-token",
    "x-auth-token",
];

pub(crate) fn redact_headers<'a>(
    pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
    extra_secret: &[&str],
) -> Vec<(String, String)> {
    pairs
        .into_iter()
        .map(|(k, v)| {
            let secret = SECRET_HEADERS.iter().any(|s| k.eq_ignore_ascii_case(s))
                || extra_secret.iter().any(|s| k.eq_ignore_ascii_case(s));
            let value = if secret { REDACTED } else { v };
            (k.to_ascii_lowercase(), value.to_string())
        })
        .collect()
}

pub(crate) fn redact_query(
    query: &[(String, String)],
    secret_params: &[&str],
) -> Vec<(String, String)> {
    query
        .iter()
        .map(|(k, v)| {
            if secret_params.iter().any(|s| s.eq_ignore_ascii_case(k)) {
                (k.clone(), REDACTED.to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Metadata, envelope, transport error
// ---------------------------------------------------------------------------

/// Request-side evidence. Present on the envelope AND on `TransportError`.
/// Every string here is already redacted: real secret values are never stored.
#[derive(Debug, Clone)]
pub struct RequestMeta {
    pub endpoint_id: &'static str,
    pub method: &'static str,
    /// Full URL; secret query values replaced by `[redacted]`.
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub query: Vec<(String, String)>,
    pub auth: AuthMethod,
    /// Present only when the call is part of a paginated operation.
    pub page: Option<u32>,
}

impl RequestMeta {
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }
}

/// Response-side evidence (plus the request side).
#[derive(Debug, Clone)]
pub struct ResponseMeta {
    pub request: RequestMeta,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub elapsed: Duration,
    /// Provider request id, when the definition names its header.
    pub request_id: Option<String>,
}

impl ResponseMeta {
    /// First response header with this name (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }
}

fn find_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// The D7 envelope: metadata plus exactly one of success or failure.
pub struct Response<T, E> {
    pub meta: ResponseMeta,
    pub outcome: Outcome<T, E>,
}

pub enum Outcome<T, E> {
    Success(T),
    Failure(ApiError<E>),
}

/// Any HTTP response that is not a decoded success. The raw body is always
/// retained, whatever `kind` says.
pub struct ApiError<E> {
    pub kind: ErrorKind<E>,
    /// Raw response body, exactly as received. Server-controlled: it is NOT
    /// printed by `Debug`/`Display` because a provider may echo a credential.
    pub body: Bytes,
}

pub enum ErrorKind<E> {
    /// Status declared by the endpoint; body decoded into `E`.
    /// For GraphQL this is also how errors inside an HTTP 200 arrive.
    Declared(E),
    /// Status the endpoint does not declare (or no error type at all).
    Undeclared,
    /// Declared error status, but the body did not decode as `E`.
    MalformedError(DecodeError),
    /// 2xx, but the body did not decode as the declared success type.
    MalformedSuccess(DecodeError),
    /// 2xx with a content type that does not match the declared framing.
    UnexpectedContentType {
        expected: &'static str,
        actual: Option<String>,
    },
    /// The body could not be read to the end after headers arrived.
    BodyInterrupted(String),
    /// A `pre_response_json` or `mutate_response` hook returned an error.
    Hook(String),
}

impl<E> ApiError<E> {
    pub fn declared(&self) -> Option<&E> {
        match &self.kind {
            ErrorKind::Declared(e) => Some(e),
            _ => None,
        }
    }
    /// Lossy UTF-8 view of the raw body.
    pub fn body_text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }
}

impl<T, E> Response<T, E> {
    pub fn status(&self) -> u16 {
        self.meta.status
    }
    pub fn header(&self, name: &str) -> Option<&str> {
        self.meta.header(name)
    }
    pub fn is_success(&self) -> bool {
        matches!(self.outcome, Outcome::Success(_))
    }
    pub fn data(&self) -> Option<&T> {
        match &self.outcome {
            Outcome::Success(t) => Some(t),
            Outcome::Failure(_) => None,
        }
    }
    pub fn error(&self) -> Option<&ApiError<E>> {
        match &self.outcome {
            Outcome::Success(_) => None,
            Outcome::Failure(e) => Some(e),
        }
    }
    /// Drops success metadata; keeps failure metadata inside `HttpFailure`.
    pub fn into_result(self) -> Result<T, HttpFailure<E>> {
        match self.outcome {
            Outcome::Success(t) => Ok(t),
            Outcome::Failure(error) => Err(HttpFailure {
                meta: self.meta,
                error,
            }),
        }
    }
    pub fn into_parts(self) -> (ResponseMeta, Outcome<T, E>) {
        (self.meta, self.outcome)
    }
}

/// `into_result`'s error: an HTTP response that is not a decoded success.
pub struct HttpFailure<E> {
    pub meta: ResponseMeta,
    pub error: ApiError<E>,
}

impl<E> HttpFailure<E> {
    pub fn status(&self) -> u16 {
        self.meta.status
    }
    pub fn header(&self, name: &str) -> Option<&str> {
        self.meta.header(name)
    }
    pub fn declared(&self) -> Option<&E> {
        self.error.declared()
    }
}

/// No HTTP response exists. Still carries request-side metadata.
#[derive(Debug, thiserror::Error)]
#[error("{kind} ({} {}, auth={:?})", .request.method, .request.url, .request.auth)]
pub struct TransportError {
    pub request: RequestMeta,
    pub kind: TransportErrorKind,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportErrorKind {
    #[error("connection failed: {0}")]
    Connect(String),
    #[error("request timed out")]
    Timeout,
    #[error("redirect denied (status {status})")]
    RedirectDenied { status: u16, location: Option<String> },
    #[error("credential host policy rejected {origin}")]
    HostPolicy { origin: String },
    #[error("authentication required; checked env vars {env_vars:?}")]
    AuthenticationRequired { env_vars: Vec<String> },
    #[error("could not build request: {0}")]
    Build(String),
    #[error("transport error: {0}")]
    Other(String),
}

/// `?`-friendly union of both failure layers.
#[derive(Debug, thiserror::Error)]
pub enum CallError<E: fmt::Debug> {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Http(#[from] HttpFailure<E>),
}

/// `client.send(r).await.into_data()?` in one step.
pub trait SendResultExt<T, E: fmt::Debug> {
    fn into_data(self) -> Result<T, CallError<E>>;
}

impl<T, E: fmt::Debug> SendResultExt<T, E> for Result<Response<T, E>, TransportError> {
    fn into_data(self) -> Result<T, CallError<E>> {
        Ok(self?.into_result()?)
    }
}

// --- Debug / Display (never prints the raw body) ---------------------------

impl<E: fmt::Debug> fmt::Debug for ErrorKind<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declared(e) => f.debug_tuple("Declared").field(e).finish(),
            Self::Undeclared => f.write_str("Undeclared"),
            Self::MalformedError(e) => f.debug_tuple("MalformedError").field(e).finish(),
            Self::MalformedSuccess(e) => f.debug_tuple("MalformedSuccess").field(e).finish(),
            Self::UnexpectedContentType { expected, actual } => f
                .debug_struct("UnexpectedContentType")
                .field("expected", expected)
                .field("actual", actual)
                .finish(),
            Self::BodyInterrupted(m) => f.debug_tuple("BodyInterrupted").field(m).finish(),
            Self::Hook(m) => f.debug_tuple("Hook").field(m).finish(),
        }
    }
}

impl<E: fmt::Debug> fmt::Debug for ApiError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiError")
            .field("kind", &self.kind)
            .field("body_len", &self.body.len())
            .finish()
    }
}

impl<E: fmt::Debug> fmt::Debug for HttpFailure<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpFailure")
            .field("meta", &self.meta)
            .field("error", &self.error)
            .finish()
    }
}

impl<E: fmt::Debug> fmt::Display for HttpFailure<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Kind name only: a decoded `E` or the body may echo request secrets.
        let kind = match &self.error.kind {
            ErrorKind::Declared(_) => "declared error body",
            ErrorKind::Undeclared => "undeclared status",
            ErrorKind::MalformedError(_) => "undecodable error body",
            ErrorKind::MalformedSuccess(_) => "undecodable success body",
            ErrorKind::UnexpectedContentType { .. } => "unexpected content type",
            ErrorKind::BodyInterrupted(_) => "body interrupted",
            ErrorKind::Hook(_) => "response hook failed",
        };
        write!(
            f,
            "HTTP {} from {} {}: {kind}",
            self.meta.status, self.meta.request.method, self.meta.request.url
        )
    }
}

impl<E: fmt::Debug> std::error::Error for HttpFailure<E> {}

impl<T: fmt::Debug, E: fmt::Debug> fmt::Debug for Outcome<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success(t) => f.debug_tuple("Success").field(t).finish(),
            Self::Failure(e) => f.debug_tuple("Failure").field(e).finish(),
        }
    }
}

impl<T: fmt::Debug, E: fmt::Debug> fmt::Debug for Response<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("meta", &self.meta)
            .field("outcome", &self.outcome)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Hooks (unchanged mechanism: endpoint-id keyed, type-erased via Any)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ResponseContext {
    pub endpoint_id: &'static str,
    pub method: &'static str,
    /// Redacted URL.
    pub url: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

pub type PreResponseJsonHook = dyn Fn(&ResponseContext, serde_json::Value) -> Result<serde_json::Value, String>
    + Send
    + Sync;

pub trait AnyResponseMutator: Send + Sync {
    fn mutate(&self, ctx: &ResponseContext, response: &mut dyn Any) -> Result<(), String>;
}

pub(crate) struct TypedMutator<T, F> {
    hook: F,
    // fn pointer marker: the mutator is Send + Sync regardless of T.
    _marker: PhantomData<fn(&mut T)>,
}

impl<T, F> TypedMutator<T, F> {
    pub(crate) fn new(hook: F) -> Self {
        Self {
            hook,
            _marker: PhantomData,
        }
    }
}

impl<T, F> AnyResponseMutator for TypedMutator<T, F>
where
    T: 'static,
    F: Fn(&ResponseContext, &mut T) -> Result<(), String> + Send + Sync + 'static,
{
    fn mutate(&self, ctx: &ResponseContext, response: &mut dyn Any) -> Result<(), String> {
        let typed = response.downcast_mut::<T>().ok_or_else(|| {
            format!("type mismatch in response mutator for '{}'", ctx.endpoint_id)
        })?;
        (self.hook)(ctx, typed)
    }
}

#[derive(Default, Clone)]
pub struct VariantHooks {
    pub pre_response_json: Option<Arc<PreResponseJsonHook>>,
    pub response_mutators: HashMap<&'static str, Arc<dyn AnyResponseMutator>>,
}

/// Marker: codecs on which `mutate_response` hooks run (JSON-decoded bodies,
/// matching today's `request<T>` which is the only path that runs hooks).
/// Registering a mutator for any other endpoint is a compile error.
pub trait SupportsHooks {}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

/// Everything a codec needs. Built once per response by `send`.
pub struct DecodeCtx<'a> {
    pub response: &'a ResponseContext,
    pub content_type: Option<String>,
    pub hooks: &'a VariantHooks,
}

impl DecodeCtx<'_> {
    fn mutator(&self) -> Option<&Arc<dyn AnyResponseMutator>> {
        self.hooks.response_mutators.get(self.response.endpoint_id)
    }
}

/// How a 2xx body becomes `Output`. Async and handed the live response, so
/// buffered and streaming codecs share one signature. `E` is the endpoint's
/// failure type; only GraphQL produces `ErrorKind::Declared` from a 2xx.
pub trait Decode<E>: Send + 'static {
    type Output: Send + 'static;
    fn decode(
        response: reqwest::Response,
        ctx: DecodeCtx<'_>,
    ) -> impl Future<Output = Result<Self::Output, ApiError<E>>> + Send;
}

pub(crate) fn fail<E>(kind: ErrorKind<E>, body: Bytes) -> ApiError<E> {
    ApiError { kind, body }
}

pub(crate) async fn read_body<E>(response: reqwest::Response) -> Result<Bytes, ApiError<E>> {
    response
        .bytes()
        .await
        .map_err(|e| fail(ErrorKind::BodyInterrupted(e.without_url().to_string()), Bytes::new()))
}

/// JSON body decoded into `T`. Also usable as a declared error body.
#[derive(Debug, Clone, PartialEq)]
pub struct Json<T>(pub T);
impl<T> SupportsHooks for Json<T> {}

/// `pre_response_json` then decode. Shared by `Json<T>` and `GraphQl<D>`.
fn decode_json_pre_hook<T, E>(bytes: &Bytes, ctx: &DecodeCtx<'_>) -> Result<T, ApiError<E>>
where
    T: DeserializeOwned,
{
    let malformed = |e: serde_json::Error| fail(ErrorKind::MalformedSuccess(e.into()), bytes.clone());
    match ctx.hooks.pre_response_json.as_ref() {
        Some(hook) => {
            let json: serde_json::Value = serde_json::from_slice(bytes).map_err(malformed)?;
            let json =
                hook(ctx.response, json).map_err(|m| fail(ErrorKind::Hook(m), bytes.clone()))?;
            serde_json::from_value(json).map_err(malformed)
        }
        None => serde_json::from_slice(bytes).map_err(malformed),
    }
}

/// The endpoint's `mutate_response` hook, run on the endpoint's `Success`.
fn run_mutator<T: 'static, E>(
    value: &mut T,
    bytes: &Bytes,
    ctx: &DecodeCtx<'_>,
) -> Result<(), ApiError<E>> {
    match ctx.mutator() {
        Some(m) => m
            .mutate(ctx.response, value)
            .map_err(|msg| fail(ErrorKind::Hook(msg), bytes.clone())),
        None => Ok(()),
    }
}

impl<T, E> Decode<E> for Json<T>
where
    T: DeserializeOwned + Send + 'static,
    E: Send + 'static,
{
    type Output = T;
    async fn decode(response: reqwest::Response, ctx: DecodeCtx<'_>) -> Result<T, ApiError<E>> {
        let bytes = read_body(response).await?;
        let mut value: T = decode_json_pre_hook(&bytes, &ctx)?;
        run_mutator(&mut value, &bytes, &ctx)?;
        Ok(value)
    }
}

/// Plain-text body.
pub struct Text;
impl<E: Send + 'static> Decode<E> for Text {
    type Output = String;
    async fn decode(response: reqwest::Response, _ctx: DecodeCtx<'_>) -> Result<String, ApiError<E>> {
        let bytes = read_body(response).await?;
        String::from_utf8(bytes.to_vec()).map_err(|e| {
            fail(
                ErrorKind::MalformedSuccess(DecodeError {
                    message: e.to_string(),
                }),
                bytes,
            )
        })
    }
}

/// Raw bytes.
pub struct Binary;
impl<E: Send + 'static> Decode<E> for Binary {
    type Output = Bytes;
    async fn decode(response: reqwest::Response, _ctx: DecodeCtx<'_>) -> Result<Bytes, ApiError<E>> {
        read_body(response).await
    }
}

/// No body expected. A non-empty body is still accepted (and discarded);
/// tightening that is a policy choice, not a type-system one.
pub struct Empty;
impl<E: Send + 'static> Decode<E> for Empty {
    type Output = ();
    async fn decode(response: reqwest::Response, _ctx: DecodeCtx<'_>) -> Result<(), ApiError<E>> {
        read_body(response).await.map(|_| ())
    }
}

// --- GraphQL ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct GraphQlError {
    pub message: String,
    #[serde(default)]
    pub path: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// GraphQL's declared failure: the `errors` list plus whatever `data` the
/// server returned alongside it (partial data is kept, never discarded).
#[derive(Debug, Clone, PartialEq)]
pub struct GraphQlFailure<D> {
    pub errors: Vec<GraphQlError>,
    pub partial_data: Option<D>,
}

#[derive(serde::Deserialize)]
#[serde(bound = "D: DeserializeOwned")]
struct GraphQlBody<D> {
    data: Option<D>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

impl<D: DeserializeOwned + Send + 'static> ErrorBody for GraphQlFailure<D> {
    fn decode(body: &[u8]) -> Result<Self, DecodeError> {
        let parsed: GraphQlBody<D> = serde_json::from_slice(body)?;
        if parsed.errors.is_empty() {
            return Err(DecodeError {
                message: "GraphQL error response without `errors`".into(),
            });
        }
        Ok(GraphQlFailure {
            errors: parsed.errors,
            partial_data: parsed.data,
        })
    }
}

/// GraphQL response: `{data, errors}` inside an HTTP 200.
pub struct GraphQl<D>(PhantomData<fn() -> D>);
impl<D> SupportsHooks for GraphQl<D> {}

impl<D> Decode<GraphQlFailure<D>> for GraphQl<D>
where
    D: DeserializeOwned + Send + 'static,
{
    type Output = D;
    async fn decode(
        response: reqwest::Response,
        ctx: DecodeCtx<'_>,
    ) -> Result<D, ApiError<GraphQlFailure<D>>> {
        let bytes = read_body(response).await?;
        let body: GraphQlBody<D> = decode_json_pre_hook(&bytes, &ctx)?;
        if !body.errors.is_empty() {
            return Err(fail(
                ErrorKind::Declared(GraphQlFailure {
                    errors: body.errors,
                    partial_data: body.data,
                }),
                bytes,
            ));
        }
        let mut data = body.data.ok_or_else(|| {
            fail(
                ErrorKind::MalformedSuccess(DecodeError {
                    message: "GraphQL response had neither data nor errors".into(),
                }),
                bytes.clone(),
            )
        })?;
        run_mutator(&mut data, &bytes, &ctx)?;
        Ok(data)
    }
}

// --- Streaming -------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StreamError {
    #[error("malformed stream item: {error}")]
    MalformedItem { raw: String, error: DecodeError },
    #[error("stream closed before its terminal marker or mid-item")]
    PrematureClose,
    #[error("stream transport error: {0}")]
    Transport(String),
}

/// A decoded item stream. Not `DeserializeOwned`, not `Sync`, not `Clone`.
pub struct ItemStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>,
}

impl<T> fmt::Debug for ItemStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ItemStream { .. }")
    }
}

impl<T> Stream for ItemStream<T> {
    type Item = Result<T, StreamError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

fn content_type_matches(actual: Option<&str>, expected: &[&str]) -> bool {
    actual
        .and_then(|ct| ct.split(';').next())
        .is_some_and(|ct| expected.iter().any(|e| ct.trim().eq_ignore_ascii_case(e)))
}

async fn wrong_framing<E>(
    response: reqwest::Response,
    expected: &'static str,
    actual: Option<String>,
) -> ApiError<E> {
    let body = response.bytes().await.unwrap_or_default();
    fail(ErrorKind::UnexpectedContentType { expected, actual }, body)
}

/// Line-oriented incremental framing shared by SSE and NDJSON.
struct LineFramer {
    buf: Vec<u8>,
}

impl LineFramer {
    fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut lines = Vec::new();
        while let Some(pos) = self.buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line[..line.len() - 1])
                .trim_end_matches('\r')
                .to_string();
            lines.push(line);
        }
        lines
    }
}

fn parse_item<T: DeserializeOwned>(raw: &str) -> Result<T, StreamError> {
    serde_json::from_str(raw).map_err(|e| StreamError::MalformedItem {
        raw: raw.to_string(),
        error: e.into(),
    })
}

/// Server-sent events; each `data:` payload is JSON `T`. `data: [DONE]`
/// ends the stream (the terminal marker is an OPEN D6 detail).
pub struct Sse<T>(PhantomData<fn() -> T>);

impl<T, E> Decode<E> for Sse<T>
where
    T: DeserializeOwned + Send + 'static,
    E: Send + 'static,
{
    type Output = ItemStream<T>;
    async fn decode(
        response: reqwest::Response,
        ctx: DecodeCtx<'_>,
    ) -> Result<ItemStream<T>, ApiError<E>> {
        if !content_type_matches(ctx.content_type.as_deref(), &["text/event-stream"]) {
            return Err(wrong_framing(response, "text/event-stream", ctx.content_type).await);
        }
        let mut framer = LineFramer { buf: Vec::new() };
        let mut data: Option<String> = None;
        let mut done = false;
        let bytes = response.bytes_stream();
        let stream = async_stream_items(bytes, move |chunk| {
            let mut out = Vec::new();
            match chunk {
                Some(Ok(chunk)) => {
                    for line in framer.push(&chunk) {
                        if done {
                            break;
                        }
                        if line.is_empty() {
                            if let Some(payload) = data.take() {
                                if payload == "[DONE]" {
                                    done = true;
                                } else {
                                    out.push(parse_item::<T>(&payload));
                                }
                            }
                        } else if let Some(rest) = line.strip_prefix("data:") {
                            let rest = rest.strip_prefix(' ').unwrap_or(rest);
                            match &mut data {
                                Some(d) => {
                                    d.push('\n');
                                    d.push_str(rest);
                                }
                                None => data = Some(rest.to_string()),
                            }
                        }
                    }
                    (out, done)
                }
                Some(Err(e)) => (vec![Err(StreamError::Transport(e.without_url().to_string()))], true),
                None => {
                    // Clean end only if nothing is half-received.
                    if !framer.buf.is_empty() || data.is_some() {
                        out.push(Err(StreamError::PrematureClose));
                    }
                    (out, true)
                }
            }
        });
        Ok(ItemStream {
            inner: Box::pin(stream),
        })
    }
}

/// Newline-delimited JSON; each line is `T`.
pub struct Ndjson<T>(PhantomData<fn() -> T>);

impl<T, E> Decode<E> for Ndjson<T>
where
    T: DeserializeOwned + Send + 'static,
    E: Send + 'static,
{
    type Output = ItemStream<T>;
    async fn decode(
        response: reqwest::Response,
        ctx: DecodeCtx<'_>,
    ) -> Result<ItemStream<T>, ApiError<E>> {
        const ACCEPTED: &[&str] = &["application/x-ndjson", "application/jsonl", "application/ndjson"];
        if !content_type_matches(ctx.content_type.as_deref(), ACCEPTED) {
            return Err(wrong_framing(response, "application/x-ndjson", ctx.content_type).await);
        }
        let mut framer = LineFramer { buf: Vec::new() };
        let stream = async_stream_items(response.bytes_stream(), move |chunk| match chunk {
            Some(Ok(chunk)) => (
                framer
                    .push(&chunk)
                    .into_iter()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| parse_item::<T>(&l))
                    .collect(),
                false,
            ),
            Some(Err(e)) => (vec![Err(StreamError::Transport(e.without_url().to_string()))], true),
            None => {
                let tail = String::from_utf8_lossy(&framer.buf).trim().to_string();
                if tail.is_empty() {
                    (vec![], true)
                } else {
                    // A final line without '\n' is accepted only if it parses.
                    match parse_item::<T>(&tail) {
                        Ok(v) => (vec![Ok(v)], true),
                        Err(_) => (vec![Err(StreamError::PrematureClose)], true),
                    }
                }
            }
        });
        Ok(ItemStream {
            inner: Box::pin(stream),
        })
    }
}

/// Drives a byte stream through a stateful `step`, flattening its output.
/// `step(None)` is called once at end of input. `step` returning `true`
/// finishes the stream.
fn async_stream_items<S, T, F>(bytes: S, step: F) -> impl Stream<Item = Result<T, StreamError>> + Send
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    T: Send + 'static,
    F: FnMut(Option<Result<Bytes, reqwest::Error>>) -> (Vec<Result<T, StreamError>>, bool) + Send + 'static,
{
    struct State<S, T, F> {
        bytes: Pin<Box<S>>,
        step: F,
        pending: std::collections::VecDeque<Result<T, StreamError>>,
        finished: bool,
    }
    let state = State {
        bytes: Box::pin(bytes),
        step,
        pending: Default::default(),
        finished: false,
    };
    futures_util::stream::unfold(state, |mut st| async move {
        loop {
            if let Some(item) = st.pending.pop_front() {
                return Some((item, st));
            }
            if st.finished {
                return None;
            }
            let next = st.bytes.next().await;
            let (items, finished) = (st.step)(next);
            st.pending.extend(items);
            st.finished = finished;
        }
    })
}
