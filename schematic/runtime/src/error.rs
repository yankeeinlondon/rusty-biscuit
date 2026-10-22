//! Errors outside the envelope: sends that received no response head, and
//! client construction that was refused.
//!
//! Their `Display` and `Debug` output is composed by the runtime from
//! redacted metadata and fixed category text. The HTTP library's own error
//! text is never stored, because it can include the unredacted request URL.

use std::borrow::Cow;
use std::fmt;
use std::time::Duration;

use crate::meta::RequestMeta;
use crate::redact::RedactedUrl;

/// A send that ended before any HTTP response head arrived.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TransportError {
    /// The request as assembled, redacted. Boxed so that a `Result` carrying
    /// this error stays small.
    pub request: Box<RequestMeta>,
    /// What prevented a response head from arriving.
    pub kind: TransportErrorKind,
    /// Time from the start of the logical send to the failure.
    pub elapsed: Duration,
}

impl TransportError {
    /// Creates a transport error for `request`.
    pub fn new(request: RequestMeta, kind: TransportErrorKind, elapsed: Duration) -> Self {
        Self {
            request: Box::new(request),
            kind,
            elapsed,
        }
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} for {}", self.kind, self.request)
    }
}

impl std::error::Error for TransportError {}

/// Why no response head arrived.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TransportErrorKind {
    /// The cross-origin policy refused the target before any request to it
    /// started.
    CrossOriginRejected {
        /// The redacted URL that was refused.
        target: RedactedUrl,
    },
    /// An argument was refused before sending.
    InvalidArgument(InvalidArgument),
    /// The request URL could not be built.
    InvalidUrl,
    /// DNS failure, or the connection was refused or reset before a head.
    Connect,
    /// The TLS handshake failed.
    Tls,
    /// The per-request timeout expired before a head arrived.
    Timeout,
    /// The shared deadline expired before a head arrived, or before the send
    /// started.
    DeadlineExceeded {
        /// Logical sends the deadline completed before this one.
        completed_requests: u64,
        /// Time since the deadline was created.
        deadline_elapsed: Duration,
    },
    /// The client's settings are invalid, so no request could start. A
    /// client validates its settings on its first send, or earlier through
    /// its `build`.
    InvalidClient(BuildError),
    /// Any other failure before a head; the library's text is not kept.
    Other,
}

impl fmt::Display for TransportErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CrossOriginRejected { target } => write!(f, "cross-origin target {target} rejected"),
            Self::InvalidArgument(argument) => write!(f, "{argument}"),
            Self::InvalidUrl => f.write_str("request URL is invalid"),
            Self::Connect => f.write_str("connection failed"),
            Self::Tls => f.write_str("TLS handshake failed"),
            Self::Timeout => f.write_str("timed out before a response arrived"),
            Self::DeadlineExceeded {
                completed_requests,
                deadline_elapsed,
            } => write!(
                f,
                "deadline exceeded after {completed_requests} completed request(s) in {deadline_elapsed:?}"
            ),
            Self::InvalidClient(error) => write!(f, "invalid client: {error}"),
            Self::Other => f.write_str("request failed"),
        }
    }
}

/// An argument refused before any network work.
///
/// The reason is written by the runtime or generated code and must name the
/// argument without quoting its value, which may be secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidArgument {
    reason: Cow<'static, str>,
}

impl InvalidArgument {
    /// Creates an invalid-argument error with `reason`.
    pub fn new(reason: impl Into<Cow<'static, str>>) -> Self {
        Self { reason: reason.into() }
    }

    /// Why the argument was refused.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for InvalidArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid argument: {}", self.reason)
    }
}

impl std::error::Error for InvalidArgument {}

/// A client or shared transport that could not be constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    /// The base URL is not an HTTP(S) URL with a host and no user
    /// information.
    InvalidBaseUrl {
        /// What is wrong with it.
        reason: &'static str,
    },
    /// An allowed-origin entry is not a bare HTTP(S) origin.
    InvalidAllowedOrigin {
        /// What is wrong with it.
        reason: &'static str,
    },
    /// The requested cross-origin policy is not a tightening of the API's
    /// policy.
    OriginPolicyNotPermitted {
        /// The API definition's policy.
        definition: &'static str,
        /// The policy the caller requested.
        requested: &'static str,
    },
    /// A client attached to a shared transport also set an option that the
    /// shared transport owns.
    TransportOptionConflict {
        /// The conflicting option.
        option: &'static str,
    },
    /// A default header would carry authentication; credentials are
    /// configured as credentials instead.
    AuthBearingDefaultHeader {
        /// The header name.
        name: String,
    },
    /// A default header name or value is not valid HTTP.
    InvalidHeader {
        /// The header name, as given.
        name: String,
    },
    /// A credential of a method the API does not accept was supplied.
    IncompatibleCredential {
        /// The method that was supplied.
        supplied: &'static str,
    },
    /// The proxy URL is not valid.
    InvalidProxy,
    /// A root certificate could not be parsed.
    InvalidCertificate,
    /// The HTTP client could not be initialized.
    Transport,
    /// The stream frame limit is zero.
    InvalidMaxFrameBytes,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseUrl { reason } => write!(f, "invalid base URL: {reason}"),
            Self::InvalidAllowedOrigin { reason } => write!(f, "invalid allowed origin: {reason}"),
            Self::OriginPolicyNotPermitted { definition, requested } => write!(
                f,
                "cross-origin policy `{requested}` is not permitted over the API's `{definition}` policy"
            ),
            Self::TransportOptionConflict { option } => {
                write!(f, "`{option}` is owned by the attached shared transport")
            }
            Self::AuthBearingDefaultHeader { name } => {
                write!(f, "default header `{name}` carries authentication; configure a credential instead")
            }
            Self::InvalidHeader { name } => write!(f, "default header `{name}` is not valid"),
            Self::IncompatibleCredential { supplied } => {
                write!(f, "the API does not accept a {supplied} credential")
            }
            Self::InvalidProxy => f.write_str("proxy URL is not valid"),
            Self::InvalidCertificate => f.write_str("root certificate could not be parsed"),
            Self::Transport => f.write_str("HTTP client could not be initialized"),
            Self::InvalidMaxFrameBytes => f.write_str("the stream frame limit must be at least one byte"),
        }
    }
}

impl std::error::Error for BuildError {}
