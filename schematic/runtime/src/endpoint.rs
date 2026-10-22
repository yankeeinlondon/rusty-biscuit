//! The contract between a generated request type and the runtime.
//!
//! Each generated request implements [`EndpointSpec`], which fixes at compile
//! time the API it belongs to, its success and failure types, and the codec
//! that decodes its body. A send therefore never takes a caller-chosen
//! response type.

use std::ops::RangeInclusive;

use bytes::Bytes;
use http::Method;

use crate::codec::Decode;
use crate::error::InvalidArgument;

/// The tag of one generated API.
///
/// A generated API struct wraps a [`Client`](crate::Client) of its tag and
/// accepts only requests whose [`EndpointSpec::Api`] is that tag, so a
/// request built for one API cannot be sent through another API's client even
/// when both declare an endpoint of the same name.
pub trait Api: 'static {
    /// Name of the API, used in diagnostics.
    const NAME: &'static str;
}

/// Links a generated request type to everything the runtime needs to send it
/// and decode its reply.
pub trait EndpointSpec: Send + 'static {
    /// The API this endpoint belongs to.
    type Api: Api;
    /// The value a 2xx reply decodes into.
    type Success: Send + 'static;
    /// The declared error body; [`NoDeclaredError`](crate::NoDeclaredError)
    /// when the endpoint declares none.
    type Failure: ErrorBody;
    /// The codec that reads a 2xx body into [`Self::Success`].
    type Decoder: Decode<Output = Self::Success>;

    /// Identifier of the endpoint, unique within its API.
    const ENDPOINT_ID: &'static str;

    /// The endpoint's method. Metadata of a request whose parts could not be
    /// assembled reports it; a sent request uses [`RequestParts::method`].
    const METHOD: Method;

    /// Statuses whose body is decoded as [`Self::Failure`] when the endpoint
    /// declares one. Any other non-2xx status is reported as undeclared.
    const DECLARED_ERROR_STATUSES: RangeInclusive<u16> = 400..=599;

    /// Assembles the application-level request.
    ///
    /// The result carries no credential: the runtime resolves credentials
    /// once per send and attaches them to each hop it is allowed to reach.
    ///
    /// ## Errors
    ///
    /// Returns [`InvalidArgument`] when a parameter cannot be encoded; the
    /// runtime reports it before any network work.
    fn into_parts(self) -> Result<RequestParts, InvalidArgument>;
}

/// A failure body that the runtime decodes when a declared error status
/// arrives, so the caller never performs a decode step.
pub trait ErrorBody: Sized + Send + 'static {
    /// `false` only for [`NoDeclaredError`](crate::NoDeclaredError): every
    /// error reply of such an endpoint is reported as undeclared.
    const DECLARED: bool = true;

    /// Decodes a declared error body, or returns `None` when the bytes do not
    /// form this type; the runtime then reports a malformed error body.
    fn decode_body(body: &[u8]) -> Option<Self>;
}

/// The application-level request an endpoint sends, before credentials.
///
/// It is kept whole for the duration of a send so that a redirect can be
/// replayed from it rather than from an already authenticated request.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestParts {
    /// HTTP method.
    pub method: Method,
    /// Path with path parameters substituted. A query written into the
    /// endpoint's path template stays here; declared query parameters do not.
    pub path: String,
    /// Declared query parameters in declaration order, **not**
    /// percent-encoded. Duplicate names are kept in order.
    pub query: Vec<(String, QueryValue)>,
    /// Request headers as (name, value) pairs, including cookie parameters
    /// already folded into a `Cookie` header.
    pub headers: Vec<(String, String)>,
    /// Request body; held as owned values so it can be replayed.
    pub body: RequestBody,
}

impl RequestParts {
    /// Creates parts for `method` and `path` with no query, headers, or body.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query: Vec::new(),
            headers: Vec::new(),
            body: RequestBody::Empty,
        }
    }
}

/// The value of one declared query pair, before percent-encoding.
///
/// The client encodes every value once, when it builds the URL: every byte
/// outside the RFC 3986 unreserved set becomes `%XX`, so a space is `%20`,
/// never `+`. A [`Delimited`](Self::Delimited) value encodes each element
/// that way and then joins them with its delimiter, so a delimiter inside an
/// element stays escaped:
///
/// ```
/// use schematic_runtime::{QueryDelimiter, QueryValue};
///
/// let tags = QueryValue::delimited(["a,b", "c"], QueryDelimiter::Comma);
/// assert_eq!(tags.encoded(), "a%2Cb,c");
/// assert_eq!(QueryValue::from("a,b c").encoded(), "a%2Cb%20c");
/// assert_eq!(QueryValue::delimited(Vec::<String>::new(), QueryDelimiter::Pipe).encoded(), "");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryValue {
    /// One value, encoded whole.
    Text(String),
    /// A non-exploded array (OpenAPI `form` with `explode: false`,
    /// `spaceDelimited`, or `pipeDelimited`): the elements in order and the
    /// delimiter that joins them. No elements is the empty value.
    Delimited {
        /// The unencoded elements.
        elements: Vec<String>,
        /// What joins the encoded elements.
        delimiter: QueryDelimiter,
    },
}

/// The separator of a [`QueryValue::Delimited`] value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryDelimiter {
    /// A literal `,` (`form`, `explode: false`).
    Comma,
    /// A space, sent as `%20` (`spaceDelimited`).
    Space,
    /// A `|`, sent as `%7C` (`pipeDelimited`).
    Pipe,
}

impl QueryDelimiter {
    /// The delimiter as it appears in the encoded query.
    pub fn encoded(self) -> &'static str {
        match self {
            Self::Comma => ",",
            Self::Space => "%20",
            Self::Pipe => "%7C",
        }
    }
}

impl QueryValue {
    /// A [`Delimited`](Self::Delimited) value of `elements` joined by
    /// `delimiter`.
    pub fn delimited<I>(elements: I, delimiter: QueryDelimiter) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self::Delimited { elements: elements.into_iter().map(Into::into).collect(), delimiter }
    }

    /// The value as it is written into the URL's query.
    pub fn encoded(&self) -> String {
        let mut out = String::new();
        match self {
            Self::Text(text) => encode_query_component(text, &mut out),
            Self::Delimited { elements, delimiter } => {
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        out.push_str(delimiter.encoded());
                    }
                    encode_query_component(element, &mut out);
                }
            }
        }
        out
    }
}

impl From<String> for QueryValue {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for QueryValue {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

/// Percent-encodes `text` into `out`, keeping only RFC 3986 unreserved
/// characters; the one query encoding the runtime writes.
pub(crate) fn encode_query_component(text: &str, out: &mut String) {
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
}

/// A replayable request body. Each variant owns its `Content-Type`.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum RequestBody {
    /// No body is sent.
    #[default]
    Empty,
    /// A serialized JSON body (`application/json`).
    Json(String),
    /// An `application/x-www-form-urlencoded` body of unencoded pairs.
    UrlEncoded(Vec<(String, String)>),
    /// A `multipart/form-data` body; the runtime chooses the boundary.
    Multipart(Vec<FormPart>),
    /// A body sent verbatim under an explicit content type.
    Raw {
        /// Value for the `Content-Type` header.
        content_type: String,
        /// Body bytes, sent unmodified.
        bytes: Bytes,
    },
}

/// One part of a `multipart/form-data` body.
#[derive(Debug, Clone, PartialEq)]
pub enum FormPart {
    /// A plain text part.
    Text {
        /// Part name, as it appears in `Content-Disposition`.
        name: String,
        /// Part value.
        value: String,
    },
    /// A part carrying serialized JSON under `application/json`.
    Json {
        /// Part name, as it appears in `Content-Disposition`.
        name: String,
        /// Serialized JSON value.
        value: String,
    },
    /// A file part.
    File {
        /// Part name, as it appears in `Content-Disposition`.
        name: String,
        /// File name reported to the server.
        file_name: String,
        /// MIME type of the part; the server infers one when absent.
        mime: Option<String>,
        /// File contents.
        bytes: Bytes,
    },
}
