//! Turns replies from the generated provider clients into [`SniffError`]s.
//!
//! Every awaited endpoint call returns its data or a [`CallError`]: a
//! [`TransportError`] when no response head arrived, or an [`ApiError`] with
//! the [`ResponseMeta`] of the reply that carried it. A fetched list page
//! fails with a [`PagerError`] carrying the same two cases. [`Replies`]
//! classifies them the same way for every provider; only the meaning of a 401
//! differs.
//!
//! ## What is safe to report
//!
//! Errors are composed from the reply's status, the `Retry-After` and
//! `x-ratelimit-remaining` header values, the provider's body text, and the
//! runtime's redacted `Display` forms. A header map, a raw HTTP library error,
//! or an unredacted request URL never reaches a message.

use reqwest::header::HeaderMap;
use schematic_schema::schematic_runtime::pager::{Page, PagerError};
use schematic_schema::schematic_runtime::{
    ApiError, BuildError, CallError, ErrorKind, RawCompletion, RawResponse, ResponseMeta,
    TransportError,
};

use crate::error::SniffError;

/// What a provider means by a 401.
#[derive(Debug, Clone, Copy)]
pub(super) enum Unauthorized {
    /// The credentials were rejected: `InvalidCredentials`, with the JSON
    /// body's `message` when it has one.
    Rejected,
    /// Credentials are needed: `MissingCredentials`, naming these variables
    /// and quoting the body.
    Missing(&'static str),
}

/// The classifier for one provider's replies.
#[derive(Debug, Clone, Copy)]
pub(super) struct Replies {
    provider: &'static str,
    unauthorized: Unauthorized,
}

impl Replies {
    pub(super) const fn new(provider: &'static str, unauthorized: Unauthorized) -> Self {
        Self {
            provider,
            unauthorized,
        }
    }

    /// The data of an awaited call, or the error it classifies as.
    pub(super) fn value<T, E>(&self, sent: Result<T, CallError<E>>) -> Result<T, SniffError> {
        sent.map_err(|error| self.error(&error))
    }

    /// A raw send's 2xx reply with its complete body, or the error it
    /// classifies as: an error status as for a call, and a cut-off body or an
    /// unfollowed redirect without a status.
    pub(super) fn raw(&self, sent: Result<RawResponse, TransportError>) -> Result<RawResponse, SniffError> {
        let raw = sent.map_err(|error| self.transport(&error))?;
        let status = raw.meta.status;
        if !status.is_success() && !status.is_redirection() {
            return Err(self.status(status.as_u16(), &raw.meta.headers, &raw.body));
        }
        match raw.completion {
            RawCompletion::Complete if status.is_success() => Ok(raw),
            RawCompletion::BodyInterrupted => Err(self.remote_api(0, "the response body was cut off".to_string())),
            _ => Err(self.remote_api(status.as_u16(), "the response was a redirect that was not followed".to_string())),
        }
    }

    /// A fetched list page, or the error it classifies as.
    ///
    /// Transport and response failures classify as they do for a call. Any
    /// other pager failure (an unusable continuation, an expired deadline) has
    /// no status and is reported by its redacted `Display`.
    pub(super) fn page<T, E>(
        &self,
        fetched: Result<Page<T>, PagerError<E>>,
    ) -> Result<Page<T>, SniffError> {
        fetched.map_err(|error| match &error {
            PagerError::Transport(error) => self.transport(error),
            PagerError::Api { error, meta } => self.failure(meta, error),
            other => self.remote_api(0, other.to_string()),
        })
    }

    /// The error a failed call classifies as.
    fn error<E>(&self, error: &CallError<E>) -> SniffError {
        match error {
            CallError::Transport(error) => self.transport(error),
            CallError::Api { error, meta } => self.failure(meta, error),
            // `CallError` is non-exhaustive; a future variant still carries a
            // redacted `Display`, reported without a status.
            other => self.remote_api(0, other.to_string()),
        }
    }

    /// A call that received no response head.
    fn transport(&self, error: &TransportError) -> SniffError {
        self.remote_api(0, error.to_string())
    }

    /// A reply whose outcome is a failure.
    ///
    /// An error status is classified by [`status`](Self::status), even when its
    /// body was cut off. A 2xx body that did not decode, a cut-off 2xx body, a
    /// refused or failed redirect, and a hook rejection are status 0, told
    /// apart by the runtime's description of the failure.
    fn failure<E>(&self, meta: &ResponseMeta, error: &ApiError<E>) -> SniffError {
        let has_error_status = !meta.status.is_success() && !meta.status.is_redirection();
        let classify_by_status = match &error.kind {
            ErrorKind::Declared(_) | ErrorKind::Undeclared | ErrorKind::MalformedError => true,
            ErrorKind::BodyInterrupted => has_error_status,
            _ => false,
        };
        if classify_by_status {
            self.status(meta.status.as_u16(), &meta.headers, &error.body)
        } else {
            self.remote_api(0, error.to_string())
        }
    }

    /// Classifies an error status from its headers and body.
    pub(super) fn status(&self, status: u16, headers: &HeaderMap, body: &[u8]) -> SniffError {
        let text = String::from_utf8_lossy(body);
        match status {
            401 => match self.unauthorized {
                Unauthorized::Rejected => SniffError::InvalidCredentials {
                    provider: self.provider.to_string(),
                    message: serde_json::from_slice::<serde_json::Value>(body)
                        .ok()
                        .and_then(|value| value.get("message")?.as_str().map(String::from))
                        .unwrap_or_else(|| "bad credentials".to_string()),
                },
                Unauthorized::Missing(variables) => SniffError::MissingCredentials {
                    provider: self.provider.to_string(),
                    env_var: format!("{variables} (API returned 401: {text})"),
                },
            },
            403 if quota_spent(headers) || text.to_lowercase().contains("rate limit") => {
                self.rate_limited(headers)
            }
            429 => self.rate_limited(headers),
            404 => self.remote_api(404, "Not found".to_string()),
            status => self.remote_api(status, text.into_owned()),
        }
    }

    fn rate_limited(&self, headers: &HeaderMap) -> SniffError {
        SniffError::RateLimited {
            provider: self.provider.to_string(),
            retry_after: retry_after(headers),
        }
    }

    fn remote_api(&self, status: u16, message: String) -> SniffError {
        SniffError::RemoteApi {
            provider: self.provider.to_string(),
            status,
            message,
        }
    }

    /// A client that could not be built.
    pub(super) fn init(&self, error: &BuildError) -> SniffError {
        SniffError::RemoteInit {
            provider: self.provider.to_string(),
            message: error.to_string(),
        }
    }
}

/// Whether `x-ratelimit-remaining` says the quota is spent.
fn quota_spent(headers: &HeaderMap) -> bool {
    headers
        .get("x-ratelimit-remaining")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.trim() == "0")
}

/// `Retry-After` in delta-seconds; the HTTP-date form is not read.
fn retry_after(headers: &HeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    const GITHUB: Replies = Replies::new("GitHub", Unauthorized::Rejected);
    const GITEA: Replies = Replies::new("Gitea", Unauthorized::Missing("GITEA_TOKEN"));

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, HeaderValue::from_static(value));
        }
        map
    }

    #[test]
    fn a_401_is_rejected_credentials_with_the_body_message() {
        let error = GITHUB.status(401, &HeaderMap::new(), br#"{"message":"Bad credentials"}"#);

        match error {
            SniffError::InvalidCredentials { provider, message } => {
                assert_eq!(provider, "GitHub");
                assert_eq!(message, "Bad credentials");
            }
            other => panic!("expected InvalidCredentials, got {other:?}"),
        }
    }

    #[test]
    fn a_401_is_missing_credentials_naming_the_variables_for_other_providers() {
        let error = GITEA.status(401, &HeaderMap::new(), b"Unauthorized");

        match error {
            SniffError::MissingCredentials { provider, env_var } => {
                assert_eq!(provider, "Gitea");
                assert_eq!(env_var, "GITEA_TOKEN (API returned 401: Unauthorized)");
            }
            other => panic!("expected MissingCredentials, got {other:?}"),
        }
    }

    #[test]
    fn a_429_reads_retry_after_delta_seconds() {
        let error = GITHUB.status(429, &headers(&[("retry-after", "30")]), b"");

        assert!(matches!(
            error,
            SniffError::RateLimited {
                retry_after: Some(30),
                ..
            }
        ));
    }

    #[test]
    fn a_retry_after_that_is_absent_or_not_seconds_is_none() {
        for map in [
            HeaderMap::new(),
            headers(&[("retry-after", "Wed, 21 Oct 2015 07:28:00 GMT")]),
        ] {
            let error = GITHUB.status(429, &map, b"");
            assert!(
                matches!(
                    error,
                    SniffError::RateLimited {
                        retry_after: None,
                        ..
                    }
                ),
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_403_is_rate_limited_by_a_spent_quota_or_a_rate_limit_body() {
        let spent = GITHUB.status(403, &headers(&[("x-ratelimit-remaining", "0")]), b"denied");
        let body = GITHUB.status(403, &HeaderMap::new(), b"API Rate Limit exceeded");

        assert!(matches!(spent, SniffError::RateLimited { .. }), "{spent:?}");
        assert!(matches!(body, SniffError::RateLimited { .. }), "{body:?}");
    }

    #[test]
    fn any_other_403_is_a_remote_api_error_with_the_body() {
        let error = GITHUB.status(403, &headers(&[("x-ratelimit-remaining", "12")]), b"denied");

        match error {
            SniffError::RemoteApi {
                status, message, ..
            } => {
                assert_eq!(status, 403);
                assert_eq!(message, "denied");
            }
            other => panic!("expected RemoteApi, got {other:?}"),
        }
    }

    #[test]
    fn a_404_is_not_found_and_other_statuses_keep_their_body() {
        let missing = GITHUB.status(404, &HeaderMap::new(), b"{\"message\":\"Not Found\"}");
        let failed = GITHUB.status(502, &HeaderMap::new(), b"bad gateway");

        assert!(matches!(
            missing,
            SniffError::RemoteApi { status: 404, ref message, .. } if message == "Not found"
        ));
        assert!(matches!(
            failed,
            SniffError::RemoteApi { status: 502, ref message, .. } if message == "bad gateway"
        ));
    }
}
