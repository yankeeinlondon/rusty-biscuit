//! The response envelope: metadata plus a success value or an API failure.
//!
//! Every send that received an HTTP response head returns a [`Response`],
//! whatever its status and whether or not its body could be read or decoded.
//! The raw body of a failure is always kept in [`ApiError::body`].
//!
//! ## Debug can print provider data
//!
//! Stored metadata and every `Display` implementation here are free of
//! credentials and never print a body. `Debug` is different: it prints
//! [`ApiError::body`] and a decoded declared error exactly as the provider
//! sent them. A provider that echoes a credential in an error body (for
//! example "invalid key sk-…") exposes it through `Debug` logging. The bytes
//! are kept unchanged on purpose, as evidence; log the `Display` form, or
//! the [`ResponseMeta`], when a log line may be shared.

use std::fmt;

use bytes::Bytes;

use crate::endpoint::ErrorBody;
use crate::meta::ResponseMeta;
use crate::redact::RedactedUrl;

/// The envelope returned for every send that received a response head.
#[derive(Debug, Clone)]
pub struct Response<T, E> {
    /// Metadata of the response and of the request that produced it.
    pub meta: ResponseMeta,
    /// The decoded success value or the failure.
    pub outcome: Outcome<T, E>,
}

/// The result inside a [`Response`].
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome<T, E> {
    /// A 2xx reply whose body decoded into the endpoint's success type.
    Success(T),
    /// Any other reply, with its raw body.
    Failure(ApiError<E>),
}

impl<T, E> Response<T, E> {
    /// The success value, or `None` for a failure.
    pub fn data(&self) -> Option<&T> {
        match &self.outcome {
            Outcome::Success(value) => Some(value),
            Outcome::Failure(_) => None,
        }
    }

    /// The failure, or `None` for a success.
    pub fn error(&self) -> Option<&ApiError<E>> {
        match &self.outcome {
            Outcome::Success(_) => None,
            Outcome::Failure(error) => Some(error),
        }
    }

    /// The outcome as a `Result`, **discarding the metadata**; use
    /// [`into_parts`](Self::into_parts) to keep it.
    pub fn into_result(self) -> Result<T, ApiError<E>> {
        match self.outcome {
            Outcome::Success(value) => Ok(value),
            Outcome::Failure(error) => Err(error),
        }
    }

    /// Splits the envelope into its metadata and outcome.
    pub fn into_parts(self) -> (ResponseMeta, Outcome<T, E>) {
        (self.meta, self.outcome)
    }

    /// Whether this is a reply whose 2xx body could not be decoded
    /// ([`ErrorKind::MalformedSuccess`]).
    pub fn is_decode_failure(&self) -> bool {
        self.error().is_some_and(ApiError::is_decode_failure)
    }

    /// Whether the body was cut off after the head arrived
    /// ([`ErrorKind::BodyInterrupted`]).
    pub fn is_interrupted(&self) -> bool {
        self.error().is_some_and(ApiError::is_interrupted)
    }
}

impl<T, E> fmt::Display for Response<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.outcome {
            Outcome::Success(_) => write!(f, "{}: success", self.meta),
            Outcome::Failure(error) => write!(f, "{}: {error}", self.meta),
        }
    }
}

/// A failure inside the envelope, with the raw body kept as evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiError<E> {
    /// What went wrong.
    pub kind: ErrorKind<E>,
    /// The body bytes exactly as received: the whole body, or the bytes that
    /// arrived before an interruption. Never redacted.
    pub body: Bytes,
}

impl<E> ApiError<E> {
    /// The decoded declared error body, if this is
    /// [`ErrorKind::Declared`].
    pub fn declared(&self) -> Option<&E> {
        match &self.kind {
            ErrorKind::Declared(error) => Some(error),
            _ => None,
        }
    }

    /// Whether a 2xx body could not be decoded
    /// ([`ErrorKind::MalformedSuccess`]); `false` for every other kind.
    pub fn is_decode_failure(&self) -> bool {
        matches!(self.kind, ErrorKind::MalformedSuccess)
    }

    /// Whether the body was cut off after the head arrived
    /// ([`ErrorKind::BodyInterrupted`]); `false` for every other kind.
    pub fn is_interrupted(&self) -> bool {
        matches!(self.kind, ErrorKind::BodyInterrupted)
    }
}

impl<E> fmt::Display for ApiError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({} body bytes)", self.kind, self.body.len())
    }
}

impl<E: fmt::Debug> std::error::Error for ApiError<E> {}

/// Why a reply is a failure.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind<E> {
    /// A declared error status whose body decoded into the endpoint's
    /// declared failure type.
    Declared(E),
    /// A non-2xx reply the endpoint declares no body for.
    Undeclared,
    /// A declared error status whose body did not decode.
    MalformedError,
    /// A 2xx reply whose body did not decode, including a body whose content
    /// does not match the endpoint's codec.
    MalformedSuccess,
    /// The body was cut off, or its read timed out, after the head arrived.
    BodyInterrupted,
    /// A redirect was received and not followed.
    RedirectRefused {
        /// Why the redirect was not followed.
        reason: RedirectRefusal,
    },
    /// A followed redirect hop failed before its response head arrived; the
    /// envelope describes the last redirect that was received.
    RedirectFailed {
        /// The redacted URL of the hop that failed.
        target: RedactedUrl,
        /// Why that hop failed.
        category: FailureCategory,
    },
    /// A response hook rejected the body.
    Hook,
}

impl<E> fmt::Display for ErrorKind<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declared(_) => f.write_str("declared error response"),
            Self::Undeclared => f.write_str("undeclared error response"),
            Self::MalformedError => f.write_str("error response body could not be decoded"),
            Self::MalformedSuccess => f.write_str("success response body could not be decoded"),
            Self::BodyInterrupted => f.write_str("response body was interrupted"),
            Self::RedirectRefused { reason } => write!(f, "redirect refused: {reason}"),
            Self::RedirectFailed { target, category } => {
                write!(f, "redirect to {target} failed: {category}")
            }
            Self::Hook => f.write_str("response hook rejected the body"),
        }
    }
}

/// Why a received redirect was not followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RedirectRefusal {
    /// The cross-origin policy does not allow the target origin.
    CrossOrigin,
    /// The hop limit was reached.
    HopLimit,
    /// The redirect carried no `Location` header.
    MissingLocation,
    /// The `Location` header could not be parsed as a URL.
    InvalidLocation,
    /// The target uses a scheme other than HTTP(S) or carries user
    /// information.
    UnsafeLocation,
}

impl fmt::Display for RedirectRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CrossOrigin => "target origin is not allowed",
            Self::HopLimit => "too many redirects",
            Self::MissingLocation => "no Location header",
            Self::InvalidLocation => "Location is not a valid URL",
            Self::UnsafeLocation => "Location is not a plain HTTP(S) URL",
        })
    }
}

/// A sanitized category for a failure before a response head arrived.
///
/// It carries no library error text, which can include an unredacted URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FailureCategory {
    /// DNS failure, or the connection was refused or reset.
    Connect,
    /// The TLS handshake failed.
    Tls,
    /// The per-request timeout expired.
    Timeout,
    /// The shared deadline expired.
    DeadlineExceeded,
    /// Any other failure.
    Other,
}

impl fmt::Display for FailureCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Connect => "connection failed",
            Self::Tls => "TLS handshake failed",
            Self::Timeout => "timed out",
            Self::DeadlineExceeded => "deadline exceeded",
            Self::Other => "request failed",
        })
    }
}

/// The failure type of an endpoint that declares no error body.
///
/// It has no values, so [`ErrorKind::Declared`] cannot occur for such an
/// endpoint; its error replies arrive as [`ErrorKind::Undeclared`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoDeclaredError {}

impl fmt::Display for NoDeclaredError {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for NoDeclaredError {}

impl ErrorBody for NoDeclaredError {
    const DECLARED: bool = false;

    fn decode_body(_: &[u8]) -> Option<Self> {
        None
    }
}
