//! HTTP fetch primitive with policy-enforcing host allowlists.
//!
//! All remote read/write consumers share this primitive so host policy is
//! enforced in a single place. The feature is off by default and must be
//! enabled via the `fetch` cargo feature.

use url::Url;

use crate::file_reference::error::FetchError;

/// A pattern that matches one or more hosts in a [`FetchPolicy`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostPattern {
    /// Match a hostname exactly (case-insensitive).
    Exact(String),
    /// Match any subdomain of the given parent domain.
    ///
    /// The wildcard `*.example.com` matches `a.example.com` and
    /// `b.c.example.com` but **not** `example.com` itself.
    Wildcard(String),
}

impl HostPattern {
    /// Returns `true` if `host` matches this pattern.
    pub fn matches(&self, host: &str) -> bool {
        match self {
            HostPattern::Exact(e) => e.eq_ignore_ascii_case(host),
            HostPattern::Wildcard(suffix) => {
                let host_lower = host.to_ascii_lowercase();
                let suffix_lower = suffix.to_ascii_lowercase();
                host_lower.ends_with(&format!(".{suffix_lower}"))
                    || host_lower.eq_ignore_ascii_case(&suffix_lower)
            }
        }
    }
}

/// Policy controlling which hosts the fetch primitive is allowed to contact.
///
/// The default is deny-all: no host is allowed until explicitly added.
#[derive(Debug, Clone, Default)]
pub struct FetchPolicy {
    allowed: Vec<HostPattern>,
}

impl FetchPolicy {
    /// Create a new deny-all policy.
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// Allow a host pattern.
    pub fn allow(mut self, pattern: HostPattern) -> Self {
        self.allowed.push(pattern);
        self
    }

    /// Allow an exact host.
    pub fn allow_host(self, host: &str) -> Self {
        self.allow(HostPattern::Exact(host.to_string()))
    }

    /// Check whether a host is permitted by this policy.
    pub fn is_allowed(&self, host: &str) -> bool {
        self.allowed.iter().any(|p| p.matches(host))
    }
}

/// Conditional request metadata for cache revalidation.
#[derive(Debug, Clone, Default)]
pub struct Conditional {
    /// `If-None-Match` header value (e.g. `"abc123"`).
    pub if_none_match: Option<String>,
    /// `If-Modified-Since` header value (HTTP-date).
    pub if_modified_since: Option<String>,
}

/// A structured HTTP fetch response.
#[derive(Debug, Clone)]
pub struct FetchResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response body as bytes.
    pub body: bytes::Bytes,
    /// `ETag` header, if present.
    pub etag: Option<String>,
    /// `Last-Modified` header, if present.
    pub last_modified: Option<String>,
    /// `Cache-Control` header, if present.
    pub cache_control: Option<String>,
}

impl FetchResponse {
    /// Returns `true` if the response is a success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Returns `true` if the response is `304 Not Modified`.
    pub fn is_not_modified(&self) -> bool {
        self.status == 304
    }
}

/// Perform an async HTTP GET with policy enforcement and optional
/// conditional headers.
///
/// The caller supplies a shared [`reqwest::Client`] so connection pooling
/// and TLS configuration are managed externally.
///
/// ## Errors
///
/// - [`FetchError::PolicyDenied`] if the host is not in the allowlist.
/// - [`FetchError::UnsupportedScheme`] for non-HTTP(S) URLs.
/// - [`FetchError::RequestFailed`] if the request itself fails.
/// - [`FetchError::HttpError`] for non-success HTTP status codes.
pub async fn fetch(
    client: &reqwest::Client,
    url: &Url,
    policy: &FetchPolicy,
    conditional: &Conditional,
) -> Result<FetchResponse, FetchError> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(FetchError::UnsupportedScheme(scheme.to_string()));
    }

    let host = url
        .host_str()
        .ok_or_else(|| FetchError::PolicyDenied {
            host: url.to_string(),
        })?;

    if !policy.is_allowed(host) {
        return Err(FetchError::PolicyDenied {
            host: host.to_string(),
        });
    }

    let mut request = client.get(url.as_str());

    if let Some(ref etag) = conditional.if_none_match {
        request = request.header("If-None-Match", etag);
    }
    if let Some(ref date) = conditional.if_modified_since {
        request = request.header("If-Modified-Since", date);
    }

    let response = request.send().await.map_err(FetchError::RequestFailed)?;
    let status = response.status().as_u16();

    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let cache_control = response
        .headers()
        .get("cache-control")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body = response.bytes().await.map_err(FetchError::BodyReadFailed)?;

    if status >= 400 {
        return Err(FetchError::HttpError {
            status,
            url: url.to_string(),
        });
    }

    Ok(FetchResponse {
        status,
        body,
        etag,
        last_modified,
        cache_control,
    })
}

/// Perform an async HTTP POST with the same scheme and host policy enforcement
/// as [`fetch()`].
pub async fn post(
    client: &reqwest::Client,
    url: &Url,
    policy: &FetchPolicy,
    body: impl Into<bytes::Bytes>,
) -> Result<FetchResponse, FetchError> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(FetchError::UnsupportedScheme(scheme.to_string()));
    }

    let host = url
        .host_str()
        .ok_or_else(|| FetchError::PolicyDenied {
            host: url.to_string(),
        })?;

    if !policy.is_allowed(host) {
        return Err(FetchError::PolicyDenied {
            host: host.to_string(),
        });
    }

    let response = client
        .post(url.as_str())
        .body(body.into())
        .send()
        .await
        .map_err(FetchError::RequestFailed)?;
    let status = response.status().as_u16();

    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let cache_control = response
        .headers()
        .get("cache-control")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body = response.bytes().await.map_err(FetchError::BodyReadFailed)?;

    if status >= 400 {
        return Err(FetchError::HttpError {
            status,
            url: url.to_string(),
        });
    }

    Ok(FetchResponse {
        status,
        body,
        etag,
        last_modified,
        cache_control,
    })
}

/// Synchronous convenience wrapper around [`fetch()`].
///
/// Spawns a minimal Tokio runtime for the duration of the request.
/// Callers that already have a runtime should prefer [`fetch()`] directly.
pub fn fetch_blocking(
    client: &reqwest::Client,
    url: &Url,
    policy: &FetchPolicy,
    conditional: &Conditional,
) -> Result<FetchResponse, FetchError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build Tokio runtime for fetch_blocking");

    rt.block_on(fetch(client, url, policy, conditional))
}

/// Synchronous convenience wrapper around [`post()`].
pub fn post_blocking(
    client: &reqwest::Client,
    url: &Url,
    policy: &FetchPolicy,
    body: impl Into<bytes::Bytes>,
) -> Result<FetchResponse, FetchError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build Tokio runtime for post_blocking");

    rt.block_on(post(client, url, policy, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_pattern_exact_matches_case_insensitive() {
        let pat = HostPattern::Exact("Example.com".to_string());
        assert!(pat.matches("example.com"));
        assert!(pat.matches("EXAMPLE.COM"));
        assert!(!pat.matches("www.example.com"));
    }

    #[test]
    fn host_pattern_wildcard_matches_subdomains() {
        let pat = HostPattern::Wildcard("example.com".to_string());
        assert!(pat.matches("a.example.com"));
        assert!(pat.matches("b.c.example.com"));
        assert!(pat.matches("example.com"));
        assert!(!pat.matches("notexample.com"));
    }

    #[test]
    fn deny_all_rejects_everything() {
        let policy = FetchPolicy::deny_all();
        assert!(!policy.is_allowed("example.com"));
        assert!(!policy.is_allowed("anything.org"));
    }

    #[test]
    fn default_is_deny_all() {
        let policy = FetchPolicy::default();
        assert!(!policy.is_allowed("example.com"));
    }

    #[test]
    fn allow_host_permits_exact_match() {
        let policy = FetchPolicy::deny_all().allow_host("example.com");
        assert!(policy.is_allowed("example.com"));
        assert!(!policy.is_allowed("other.com"));
    }

    #[test]
    fn allow_wildcard_permits_subdomain() {
        let policy =
            FetchPolicy::deny_all().allow(HostPattern::Wildcard("example.com".to_string()));
        assert!(policy.is_allowed("api.example.com"));
        assert!(policy.is_allowed("example.com"));
        assert!(!policy.is_allowed("other.com"));
    }

    #[test]
    fn multiple_patterns_stack() {
        let policy = FetchPolicy::deny_all()
            .allow_host("a.com")
            .allow_host("b.com");
        assert!(policy.is_allowed("a.com"));
        assert!(policy.is_allowed("b.com"));
        assert!(!policy.is_allowed("c.com"));
    }

    #[test]
    fn fetch_response_is_success() {
        let resp = FetchResponse {
            status: 200,
            body: bytes::Bytes::new(),
            etag: None,
            last_modified: None,
            cache_control: None,
        };
        assert!(resp.is_success());
        assert!(!resp.is_not_modified());
    }

    #[test]
    fn fetch_response_is_not_modified() {
        let resp = FetchResponse {
            status: 304,
            body: bytes::Bytes::new(),
            etag: Some("abc".to_string()),
            last_modified: None,
            cache_control: None,
        };
        assert!(resp.is_not_modified());
        assert!(!resp.is_success());
    }
}
