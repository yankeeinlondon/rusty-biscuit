//! Port of sniff's `FocusedProviderClient::fetch_json` status handling
//! (sniff/lib/src/remote/focused.rs ~L1022-1069) and its `Link` paging onto
//! the envelope. `SniffError` is a local stand-in with the same variants.

use std::time::Duration;

use crate::api::{GetPullRequest, GhError, ListPullsRequest, Pull};
use crate::client::{CallOptions, DemoClient};
use crate::runtime::{AuthMethod, ErrorKind, HttpFailure, TransportError, TransportErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SniffError {
    MissingCredentials { env_var: String },
    InvalidCredentials,
    RateLimited { retry_after: Option<Duration> },
    RemoteForbidden { message: String },
    RemoteApi { status: u16, message: String },
    RemoteUnreachable { url: String, message: String },
    Incomplete { pages: u32 },
}

impl From<TransportError> for SniffError {
    fn from(e: TransportError) -> Self {
        match e.kind {
            TransportErrorKind::AuthenticationRequired { env_vars } => Self::MissingCredentials {
                env_var: env_vars.join(" or "),
            },
            kind => Self::RemoteUnreachable {
                url: e.request.url,
                message: kind.to_string(),
            },
        }
    }
}

/// The whole status policy, in one match over the failure.
impl From<HttpFailure<GhError>> for SniffError {
    fn from(f: HttpFailure<GhError>) -> Self {
        if let ErrorKind::MalformedSuccess(e) = &f.error.kind {
            return Self::RemoteApi {
                status: f.status(),
                message: format!("malformed provider answer: {e}"),
            };
        }
        let anonymous = f.meta.request.auth == AuthMethod::Anonymous;
        match f.status() {
            401 if anonymous => Self::MissingCredentials {
                env_var: "GITHUB_TOKEN or GH_TOKEN".into(),
            },
            401 => Self::InvalidCredentials,
            403 | 429 if is_rate_limited(&f) => Self::RateLimited {
                retry_after: retry_after(&f),
            },
            403 => Self::RemoteForbidden {
                message: "provider denied the query".into(),
            },
            status => Self::RemoteApi {
                status,
                // Declared body when it decoded; raw text otherwise.
                message: f
                    .declared()
                    .map_or_else(|| f.error.body_text().into_owned(), |e| e.message.clone()),
            },
        }
    }
}

fn is_rate_limited(f: &HttpFailure<GhError>) -> bool {
    f.status() == 429
        || f.header("x-ratelimit-remaining").is_some_and(|v| v.trim() == "0")
        || f.error.body_text().to_ascii_lowercase().contains("rate limit")
}

fn retry_after(f: &HttpFailure<GhError>) -> Option<Duration> {
    f.header("retry-after")?.trim().parse().ok().map(Duration::from_secs)
}

/// One exact PR. Authoritative 404 is `Ok(None)`.
pub async fn get_pull(
    client: &DemoClient,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<Option<Pull>, SniffError> {
    match client.send(GetPullRequest::new(owner, repo, number)).await?.into_result() {
        Ok(pull) => Ok(Some(pull)),
        Err(f) if f.status() == 404 => Ok(None),
        Err(f) => Err(f.into()),
    }
}

/// All open PRs following `Link: rel="next"`; never a silent partial list.
pub async fn list_pulls(
    client: &DemoClient,
    owner: &str,
    repo: &str,
    max_pages: u32,
) -> Result<Vec<Pull>, SniffError> {
    let mut all = Vec::new();
    for page in 1..=max_pages {
        let request = ListPullsRequest::new(owner, repo).with_page(page);
        let options = CallOptions {
            page: Some(page),
            ..CallOptions::default()
        };
        let response = client.send_with(request, options).await?;
        let has_next = link_has_next(response.header("link"));
        all.extend(response.into_result()?);
        if !has_next {
            return Ok(all);
        }
    }
    Err(SniffError::Incomplete { pages: max_pages })
}

fn link_has_next(link: Option<&str>) -> bool {
    link.is_some_and(|value| {
        value.split(',').any(|l| {
            l.split(';')
                .skip(1)
                .any(|p| matches!(p.trim(), "rel=\"next\"" | "rel=next"))
        })
    })
}
