//! Metadata recorded for every send, redacted when it is captured.
//!
//! The only public constructors are [`RequestMeta::capture`] and
//! [`ResponseMeta::capture`]; they, and the runtime's own record of a request
//! its invalid client never assembled, apply a [`SecretNames`] set, so a
//! stored metadata field never holds a secret value.

use std::fmt;
use std::time::Duration;

use http::{HeaderMap, Method, StatusCode};
use url::Url;

use crate::redact::{RedactedUrl, SecretNames};

/// Where the credential sent with a request came from.
///
/// It names the source, never the value. It is distinct from the definition
/// enum in `schematic-define` that selects *which* authentication method an
/// API accepts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AuthMethod {
    /// A credential passed for this call.
    PerCall,
    /// A credential set on the client.
    ClientLevel,
    /// A credential read from the named environment variable.
    EnvVar(String),
    /// No credential was sent.
    Anonymous,
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PerCall => f.write_str("per-call credential"),
            Self::ClientLevel => f.write_str("client credential"),
            Self::EnvVar(name) => write!(f, "credential from environment variable {name}"),
            Self::Anonymous => f.write_str("anonymous"),
        }
    }
}

/// The request after redaction, including any credential the runtime
/// attached.
///
/// It describes the last request sent in a logical send: after a redirect,
/// the hop that received the final response.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RequestMeta {
    /// Identifier of the endpoint, unique within its API.
    pub endpoint_id: &'static str,
    /// HTTP method.
    pub method: Method,
    /// Full URL with user information and secret query values redacted.
    pub url: RedactedUrl,
    /// Decoded query parameters in URL order, secret values redacted.
    pub query: Vec<(String, String)>,
    /// Request headers, secret values redacted.
    pub headers: HeaderMap,
    /// Source of the credential that was sent.
    pub auth: AuthMethod,
    /// Page number when the send is part of a paginated operation. Reserved
    /// for pagers, which are planned; the runtime always leaves it `None`.
    pub page: Option<u32>,
}

impl RequestMeta {
    /// Records a request, redacting `url` and `headers` with `secrets`.
    pub fn capture(
        secrets: &SecretNames,
        endpoint_id: &'static str,
        method: Method,
        url: &Url,
        headers: &HeaderMap,
        auth: AuthMethod,
    ) -> Self {
        Self {
            endpoint_id,
            method,
            url: secrets.redact_url(url),
            query: secrets.redact_query(url.query_pairs().map(|(name, value)| (name.into_owned(), value.into_owned()))),
            headers: secrets.redact_headers(headers),
            auth,
            page: None,
        }
    }
}

impl RequestMeta {
    /// Records a request that was never assembled because its client's
    /// settings are invalid: the endpoint, its declared method, and the
    /// configured base URL, redacted with `secrets`.
    pub(crate) fn unassembled(secrets: &SecretNames, endpoint_id: &'static str, method: Method, base_url: &str) -> Self {
        Self {
            endpoint_id,
            method,
            url: secrets.redact_url_reference(base_url),
            query: Vec::new(),
            headers: HeaderMap::new(),
            auth: AuthMethod::Anonymous,
            page: None,
        }
    }
}

impl fmt::Display for RequestMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.url)
    }
}

/// Metadata of a received response head and the request that produced it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ResponseMeta {
    /// The request that received this response.
    pub request: RequestMeta,
    /// Response status.
    pub status: StatusCode,
    /// Response headers, secret values redacted and location headers
    /// sanitized. Repeated headers stay repeated.
    pub headers: HeaderMap,
    /// The provider's request id, when the API names a request-id header and
    /// the response carries it; redacted when that header name is secret.
    pub request_id: Option<String>,
    /// Time from the start of the logical send until its outcome was
    /// produced, including every redirect hop, the body read, decoding, and
    /// response hooks. For a codec output that keeps reading the body after
    /// decoding returns, it stops when decoding returned. A response hook,
    /// which runs before the outcome exists, sees the time to the head.
    pub elapsed: Duration,
}

impl ResponseMeta {
    /// Records a response head, redacting `headers` with `secrets`.
    ///
    /// `request_id_header` is the API's request-id header name, matched
    /// case-insensitively; the first value of that header becomes
    /// [`request_id`](Self::request_id).
    pub fn capture(
        secrets: &SecretNames,
        request: RequestMeta,
        status: StatusCode,
        headers: &HeaderMap,
        request_id_header: Option<&str>,
        elapsed: Duration,
    ) -> Self {
        let request_id = request_id_header.and_then(|wanted| {
            headers
                .iter()
                .find(|(name, _)| name.as_str().eq_ignore_ascii_case(wanted))
                .map(|(name, value)| {
                    String::from_utf8_lossy(secrets.redact_header_value(name, value).as_bytes()).into_owned()
                })
        });
        Self {
            request,
            status,
            headers: secrets.redact_headers(headers),
            request_id,
            elapsed,
        }
    }
}

impl fmt::Display for ResponseMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} from {}", self.status, self.request)
    }
}
