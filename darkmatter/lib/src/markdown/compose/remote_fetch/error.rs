//! The typed failure of one remote fetch.
//!
//! The remote runtime used to keep a failed fetch as its message alone. The
//! message is unchanged (it is still what [`RemoteFetchRuntime::get_content`]
//! returns), and [`RemoteFetchFailure::kind`] adds what the message could only
//! imply: whether the source itself failed (`Transport`) or the request or
//! host did. Expression recovery (`try`) reads that kind, never the message.
//!
//! [`RemoteFetchRuntime::get_content`]: super::RemoteFetchRuntime::get_content

use biscuit_file::file_reference::FetchError;

use crate::markdown::compose::expression::RemoteReadKind;
use crate::markdown::compose::remote::RemoteReadError;

/// A failed remote fetch: how it failed and the runtime's message for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteFetchFailure {
    pub(crate) kind: RemoteReadKind,
    pub(crate) message: String,
}

impl RemoteFetchFailure {
    pub(crate) fn new(kind: RemoteReadKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }

    /// A response body that is not UTF-8 text: the source answered, but not
    /// with text, so it is a transport failure.
    pub(crate) fn invalid_utf8(error: std::string::FromUtf8Error) -> Self {
        Self::new(RemoteReadKind::Transport, format!("invalid UTF-8 in response body: {error}"))
    }
}

impl std::fmt::Display for RemoteFetchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<FetchError> for RemoteFetchFailure {
    fn from(error: FetchError) -> Self {
        let kind = match &error {
            FetchError::RequestFailed(_)
            | FetchError::HttpError { .. }
            | FetchError::BodyReadFailed(_)
            | FetchError::InvalidHeader { .. } => RemoteReadKind::Transport,
            FetchError::PolicyDenied { .. } | FetchError::RedirectBlocked { .. } => RemoteReadKind::Policy,
            FetchError::UnsupportedScheme(_) => RemoteReadKind::InvalidReference,
            FetchError::ClientBuild(_) => RemoteReadKind::Infrastructure,
        };
        Self::new(kind, error.to_string())
    }
}

impl From<RemoteReadError> for RemoteFetchFailure {
    fn from(error: RemoteReadError) -> Self {
        let kind = match &error {
            RemoteReadError::DeniedByPolicy { .. } => RemoteReadKind::Policy,
            RemoteReadError::InvalidUrl { .. } => RemoteReadKind::InvalidReference,
        };
        Self::new(kind, error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_fetch_error_maps_to_its_kind_and_keeps_its_message() {
        let cases = [
            (FetchError::HttpError { status: 500, url: "https://a.test/x".into() }, RemoteReadKind::Transport),
            (
                FetchError::InvalidHeader { header: "etag".into(), value: "\u{7f}".into() },
                RemoteReadKind::Transport,
            ),
            (FetchError::PolicyDenied { host: "a.test".into() }, RemoteReadKind::Policy),
            (
                FetchError::RedirectBlocked { status: 302, location: "https://b.test/".into() },
                RemoteReadKind::Policy,
            ),
            (FetchError::UnsupportedScheme("ftp".into()), RemoteReadKind::InvalidReference),
        ];
        for (error, kind) in cases {
            let message = error.to_string();
            let failure = RemoteFetchFailure::from(error);
            assert_eq!(failure.kind, kind, "{message}");
            assert_eq!(failure.to_string(), message);
        }
    }

    /// A `reqwest::Error` built without any network: an unparsable request
    /// URL fails when the request is built.
    fn reqwest_error() -> reqwest::Error {
        reqwest::Client::new().get("not a url").build().expect_err("the URL does not parse")
    }

    #[test]
    fn request_and_body_failures_are_transport_and_client_build_is_infrastructure() {
        assert_eq!(RemoteFetchFailure::from(FetchError::RequestFailed(reqwest_error())).kind, RemoteReadKind::Transport);
        assert_eq!(RemoteFetchFailure::from(FetchError::BodyReadFailed(reqwest_error())).kind, RemoteReadKind::Transport);
        assert_eq!(
            RemoteFetchFailure::from(FetchError::ClientBuild(reqwest_error())).kind,
            RemoteReadKind::Infrastructure
        );
    }

    #[test]
    fn invalid_utf8_is_transport() {
        let error = String::from_utf8(vec![0xff, 0xfe]).unwrap_err();
        let failure = RemoteFetchFailure::invalid_utf8(error);
        assert_eq!(failure.kind, RemoteReadKind::Transport);
        assert!(failure.message.starts_with("invalid UTF-8 in response body: "), "{failure}");
    }

    #[test]
    fn a_policy_refusal_is_never_transport() {
        let denied = RemoteFetchFailure::from(RemoteReadError::DeniedByPolicy { host: "a.test".into() });
        assert_eq!(denied.kind, RemoteReadKind::Policy);
        assert_eq!(denied.message, "remote read denied for host 'a.test'");
        let invalid = RemoteFetchFailure::from(RemoteReadError::InvalidUrl {
            url: "data:x".into(),
            reason: "missing host".into(),
        });
        assert_eq!(invalid.kind, RemoteReadKind::InvalidReference);
    }
}
