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
                // Subdomains only: `*.example.com` matches `a.example.com` but
                // not the bare parent `example.com`. As the shared SSRF
                // boundary, the wildcard must not silently widen to the parent
                // host a delegated-subdomain policy did not intend to allow.
                let host_lower = host.to_ascii_lowercase();
                let suffix_lower = suffix.to_ascii_lowercase();
                host_lower.ends_with(&format!(".{suffix_lower}"))
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

/// A [`reqwest::Client`] that is guaranteed never to follow redirects.
///
/// [`fetch`] and [`post`] accept **only** this type, never a bare
/// `reqwest::Client`. That makes the redirect-policy bypass impossible by
/// construction: there is no way to hand the primitive a redirect-following
/// client, so a 3xx can never be followed to a host the [`FetchPolicy`] never
/// authorized. Redirects always surface as [`FetchError::RedirectBlocked`]
/// (see [`reject_redirect`]) instead of silently re-issuing the request against
/// the `Location` target. This is the single place every consumer of the
/// `fetch` feature obtains its shared client, so the SSRF boundary holds at the
/// only place requests are issued.
#[derive(Debug, Clone)]
pub struct PolicyClient {
    inner: reqwest::Client,
}

impl PolicyClient {
    /// Builds a policy-enforcing client with redirect-following disabled.
    ///
    /// ## Errors
    ///
    /// [`FetchError::ClientBuild`] if the underlying TLS/HTTP backend fails to
    /// initialize. The failure is surfaced rather than swallowed: silently
    /// falling back to a default client would substitute a redirect-following
    /// client and reopen the bypass this type exists to close.
    pub fn new() -> Result<Self, FetchError> {
        let inner = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(FetchError::ClientBuild)?;
        Ok(Self { inner })
    }
}

/// Returns `Err(FetchError::RedirectBlocked)` when `status` is a redirect.
///
/// `304 Not Modified` is a 3xx but not a redirect — it is the expected reply to
/// a conditional revalidation GET, so it is allowed through.
fn reject_redirect(status: u16, headers: &reqwest::header::HeaderMap) -> Result<(), FetchError> {
    if (300..400).contains(&status) && status != 304 {
        let location = headers
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(no Location header)".to_string());
        return Err(FetchError::RedirectBlocked { status, location });
    }
    Ok(())
}

/// Perform an async HTTP GET with policy enforcement and optional
/// conditional headers.
///
/// The caller supplies a shared [`PolicyClient`] so connection pooling and TLS
/// configuration are managed externally. Because the parameter is a
/// [`PolicyClient`] rather than a bare `reqwest::Client`, redirect-following is
/// guaranteed disabled and the allowlist cannot be bypassed mid-request.
///
/// ## Errors
///
/// - [`FetchError::PolicyDenied`] if the host is not in the allowlist.
/// - [`FetchError::UnsupportedScheme`] for non-HTTP(S) URLs.
/// - [`FetchError::RedirectBlocked`] if the server returns a redirect.
/// - [`FetchError::RequestFailed`] if the request itself fails.
/// - [`FetchError::HttpError`] for non-success HTTP status codes.
pub async fn fetch(
    client: &PolicyClient,
    url: &Url,
    policy: &FetchPolicy,
    conditional: &Conditional,
) -> Result<FetchResponse, FetchError> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(FetchError::UnsupportedScheme(scheme.to_string()));
    }

    let host = url.host_str().ok_or_else(|| FetchError::PolicyDenied {
        host: url.to_string(),
    })?;

    if !policy.is_allowed(host) {
        return Err(FetchError::PolicyDenied {
            host: host.to_string(),
        });
    }

    let mut request = client.inner.get(url.as_str());

    if let Some(ref etag) = conditional.if_none_match {
        request = request.header("If-None-Match", etag);
    }
    if let Some(ref date) = conditional.if_modified_since {
        request = request.header("If-Modified-Since", date);
    }

    let response = request.send().await.map_err(FetchError::RequestFailed)?;
    let status = response.status().as_u16();
    reject_redirect(status, response.headers())?;

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
    client: &PolicyClient,
    url: &Url,
    policy: &FetchPolicy,
    body: impl Into<bytes::Bytes>,
) -> Result<FetchResponse, FetchError> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(FetchError::UnsupportedScheme(scheme.to_string()));
    }

    let host = url.host_str().ok_or_else(|| FetchError::PolicyDenied {
        host: url.to_string(),
    })?;

    if !policy.is_allowed(host) {
        return Err(FetchError::PolicyDenied {
            host: host.to_string(),
        });
    }

    let response = client
        .inner
        .post(url.as_str())
        .body(body.into())
        .send()
        .await
        .map_err(FetchError::RequestFailed)?;
    let status = response.status().as_u16();
    reject_redirect(status, response.headers())?;

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
    client: &PolicyClient,
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
    client: &PolicyClient,
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
        // The bare parent host is NOT a subdomain and must not match.
        assert!(!pat.matches("example.com"));
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
        // Wildcard delegates subdomains only; the bare parent stays denied.
        assert!(!policy.is_allowed("example.com"));
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
