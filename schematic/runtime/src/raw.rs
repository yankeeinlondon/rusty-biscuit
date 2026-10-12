//! The result of [`Client::send_raw`](crate::Client::send_raw): a response
//! head's metadata and body bytes, without any decoding.
//!
//! A raw send goes through the same credential resolution, cross-origin
//! policy, redirect following, expiration, and redaction as a typed send. It
//! differs in two ways: no status is treated as a failure, so a 4xx or 5xx
//! with its whole body is [`RawCompletion::Complete`]; and neither response
//! hook runs.
//!
//! ## Debug can print provider data
//!
//! As with [`ApiError`](crate::ApiError), `Debug` prints the body bytes as
//! received, which can include a credential a provider echoed. `Display`
//! never prints the body.

use std::fmt;

use bytes::Bytes;
use http::{HeaderMap, StatusCode};

use crate::meta::ResponseMeta;
use crate::redact::RedactedUrl;
use crate::response::{FailureCategory, RedirectRefusal};

/// A received response with its body bytes and how far the send got.
#[derive(Debug, Clone, PartialEq)]
pub struct RawResponse {
    /// Metadata of the last response received and the request that produced
    /// it, as for a typed send.
    pub meta: ResponseMeta,
    /// The body bytes as received, never redacted: the whole body, the bytes
    /// that arrived before an interruption, or the body of the redirect that
    /// stopped the send.
    pub body: Bytes,
    /// Whether `body` is the complete body of a final response.
    pub completion: RawCompletion,
}

/// How a raw send ended after a response head arrived.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RawCompletion {
    /// The final response's body was read to its end, whatever its status.
    Complete,
    /// The body was cut off, or its read timed out; `body` holds the bytes
    /// that arrived.
    BodyInterrupted,
    /// A redirect was received and not followed; `meta` and `body` are the
    /// redirect's.
    RedirectRefused {
        /// Why the redirect was not followed.
        reason: RedirectRefusal,
    },
    /// A followed redirect hop failed before its response head arrived;
    /// `meta` and `body` are the last redirect's.
    RedirectFailed {
        /// The redacted URL of the hop that failed.
        target: RedactedUrl,
        /// Why that hop failed.
        category: FailureCategory,
    },
}

impl fmt::Display for RawCompletion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Complete => f.write_str("complete"),
            Self::BodyInterrupted => f.write_str("response body was interrupted"),
            Self::RedirectRefused { reason } => write!(f, "redirect refused: {reason}"),
            Self::RedirectFailed { target, category } => write!(f, "redirect to {target} failed: {category}"),
        }
    }
}

impl RawResponse {
    /// The status of the last response received.
    pub fn status(&self) -> StatusCode {
        self.meta.status
    }

    /// The headers of the last response received, redacted as in
    /// [`ResponseMeta::headers`].
    pub fn headers(&self) -> &HeaderMap {
        &self.meta.headers
    }

    /// Whether the body is the complete body of a final response.
    pub fn is_complete(&self) -> bool {
        matches!(self.completion, RawCompletion::Complete)
    }

    /// Parses the body as JSON into `T`, whatever the status.
    ///
    /// ## Errors
    ///
    /// Returns [`RawJsonError::Incomplete`] without parsing when the body is
    /// not complete, even if the bytes that arrived happen to form valid
    /// JSON; [`body`](Self::body) still holds them. Returns
    /// [`RawJsonError::Malformed`] when a complete body does not parse.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, RawJsonError> {
        if !self.is_complete() {
            return Err(RawJsonError::Incomplete);
        }
        serde_json::from_slice(&self.body).map_err(|_| RawJsonError::Malformed)
    }
}

impl fmt::Display for RawResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ({} body bytes)", self.meta, self.completion, self.body.len())
    }
}

/// Why [`RawResponse::json`] returned no value. It carries no parser text,
/// which can quote the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RawJsonError {
    /// The response is not [`RawCompletion::Complete`].
    Incomplete,
    /// The complete body is not valid JSON for the requested type.
    Malformed,
}

impl fmt::Display for RawJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Incomplete => "the raw response body is not complete",
            Self::Malformed => "the raw response body is not valid JSON for the requested type",
        })
    }
}

impl std::error::Error for RawJsonError {}
