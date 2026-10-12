//! Redaction of credentials from stored metadata and diagnostic text.
//!
//! A [`SecretNames`] set is the union of the built-in names below, the names
//! an API's authentication uses, and the names its definition marks secret.
//! Header names match case-insensitively; query names match exactly, after
//! percent-decoding. A secret value is replaced by [`REDACTED`]; every other
//! value, including a repeated non-secret query value, is kept as it is.
//!
//! URL user information is always redacted, and URL-valued `Location` and
//! `Content-Location` headers have their query and fragment redacted by the
//! same query-name rules, so a redirect target keeps its destination visible
//! without exposing a secret parameter.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt;

use http::header::{CONTENT_LOCATION, LOCATION};
use http::{HeaderMap, HeaderName, HeaderValue};
use url::{Position, Url};

/// The text that replaces every redacted value.
pub const REDACTED: &str = "[redacted]";

/// Header names that are always secret.
pub const BUILTIN_SECRET_HEADERS: [&str; 7] = [
    "Authorization",
    "Proxy-Authorization",
    "Cookie",
    "Set-Cookie",
    "X-Api-Key",
    "X-Auth-Token",
    "Api-Key",
];

/// Query parameter names that are always secret.
pub const BUILTIN_SECRET_QUERY_NAMES: [&str; 6] =
    ["api_key", "apikey", "access_token", "token", "key", "signature"];

/// The set of header and query names whose values are never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretNames {
    /// Lowercase header names.
    headers: BTreeSet<String>,
    /// Exact query parameter names.
    query: BTreeSet<String>,
}

impl Default for SecretNames {
    fn default() -> Self {
        Self::builtin()
    }
}

impl SecretNames {
    /// The built-in names only.
    pub fn builtin() -> Self {
        Self {
            headers: BUILTIN_SECRET_HEADERS.iter().map(|name| name.to_ascii_lowercase()).collect(),
            query: BUILTIN_SECRET_QUERY_NAMES.iter().map(|name| (*name).to_string()).collect(),
        }
    }

    /// Adds `name` as secret both as a header name and as a query name.
    ///
    /// Authentication header and parameter names and an API's own secret
    /// names are added this way.
    #[must_use]
    pub fn with_name(mut self, name: impl AsRef<str>) -> Self {
        let name = name.as_ref();
        self.headers.insert(name.to_ascii_lowercase());
        self.query.insert(name.to_string());
        self
    }

    /// Adds every name in `names`; see [`with_name`](Self::with_name).
    #[must_use]
    pub fn with_names<I, S>(self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names.into_iter().fold(self, Self::with_name)
    }

    /// Whether a header of this name has its value redacted.
    pub fn is_secret_header(&self, name: &str) -> bool {
        self.headers.contains(&name.to_ascii_lowercase())
    }

    /// Whether a query parameter of this (decoded) name has its value redacted.
    pub fn is_secret_query(&self, name: &str) -> bool {
        self.query.contains(name)
    }

    /// Copies `headers` with every secret value replaced and URL-valued
    /// location headers sanitized. Repeated headers stay repeated, in order.
    pub fn redact_headers(&self, headers: &HeaderMap) -> HeaderMap {
        let mut redacted = HeaderMap::with_capacity(headers.len());
        for (name, value) in headers {
            redacted.append(name.clone(), self.redact_header_value(name, value));
        }
        redacted
    }

    /// Copies decoded query pairs with every secret value replaced.
    pub fn redact_query<I, K, V>(&self, pairs: I) -> Vec<(String, String)>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        pairs
            .into_iter()
            .map(|(name, value)| {
                let name = name.into();
                let value = if self.is_secret_query(&name) { REDACTED.to_string() } else { value.into() };
                (name, value)
            })
            .collect()
    }

    /// Renders `url` with its user information, secret query values, and
    /// secret fragment values replaced.
    pub fn redact_url(&self, url: &Url) -> RedactedUrl {
        let mut text = String::with_capacity(url.as_str().len());
        text.push_str(&url[..Position::BeforeUsername]);
        if !url.username().is_empty() || url.password().is_some() {
            text.push_str(REDACTED);
            text.push('@');
        }
        text.push_str(&url[Position::BeforeHost..Position::AfterPath]);
        if let Some(query) = url.query() {
            text.push('?');
            text.push_str(&self.redact_pairs(query));
        }
        if let Some(fragment) = url.fragment() {
            text.push('#');
            text.push_str(&self.redact_pairs(fragment));
        }
        RedactedUrl(text)
    }

    /// Renders an absolute or relative URL reference, such as a `Location`
    /// value, with the same rules as [`redact_url`](Self::redact_url).
    pub fn redact_url_reference(&self, reference: &str) -> RedactedUrl {
        if let Ok(url) = Url::parse(reference) {
            return self.redact_url(&url);
        }
        // A scheme-relative reference can carry user information; borrow a
        // scheme to parse it and drop that scheme again.
        if reference.starts_with("//")
            && let Ok(url) = Url::parse(&format!("http:{reference}"))
        {
            let redacted = self.redact_url(&url);
            return RedactedUrl(redacted.0["http:".len()..].to_string());
        }
        let (before_fragment, fragment) = match reference.split_once('#') {
            Some((head, fragment)) => (head, Some(fragment)),
            None => (reference, None),
        };
        let mut text = match before_fragment.split_once('?') {
            Some((path, query)) => format!("{path}?{}", self.redact_pairs(query)),
            None => before_fragment.to_string(),
        };
        if let Some(fragment) = fragment {
            text.push('#');
            text.push_str(&self.redact_pairs(fragment));
        }
        RedactedUrl(text)
    }

    /// The stored form of one header value.
    pub(crate) fn redact_header_value(&self, name: &HeaderName, value: &HeaderValue) -> HeaderValue {
        if self.is_secret_header(name.as_str()) {
            return HeaderValue::from_static(REDACTED);
        }
        if name == LOCATION || name == CONTENT_LOCATION {
            // An unreadable location cannot be checked for a secret query.
            return value
                .to_str()
                .ok()
                .and_then(|reference| HeaderValue::from_str(self.redact_url_reference(reference).as_str()).ok())
                .unwrap_or_else(|| HeaderValue::from_static(REDACTED));
        }
        value.clone()
    }

    /// Redacts `name=value` pairs of a raw query or fragment, keeping every
    /// non-secret piece byte for byte.
    fn redact_pairs(&self, raw: &str) -> String {
        raw.split('&')
            .map(|piece| match piece.split_once('=') {
                Some((raw_name, _)) if self.is_secret_query(&decode_component(raw_name)) => {
                    Cow::Owned(format!("{raw_name}={REDACTED}"))
                }
                _ => Cow::Borrowed(piece),
            })
            .collect::<Vec<_>>()
            .join("&")
    }
}

/// Percent- and plus-decodes one form-encoded name.
fn decode_component(raw: &str) -> String {
    url::form_urlencoded::parse(raw.as_bytes())
        .next()
        .map(|(name, _)| name.into_owned())
        .unwrap_or_default()
}

/// A URL whose secret parts have been replaced by [`REDACTED`].
///
/// It is only produced by [`SecretNames`], so holding one means the
/// redaction has been applied.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RedactedUrl(String);

impl RedactedUrl {
    /// The redacted URL text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RedactedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for RedactedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
