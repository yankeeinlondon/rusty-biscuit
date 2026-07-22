//! Shared configured-remote resolution.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{GitHostingProvider, open};
use crate::{Result, SniffError};

/// Provider API shape inferred from a configured remote URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ApiFlavor {
    GitHub,
    GitLab,
    Gitea,
    Forgejo,
    Bitbucket,
    BitbucketDataCenter,
    AzureDevOps,
    AwsCodeCommit,
    SourceHut,
    Unknown,
}

impl From<GitHostingProvider> for ApiFlavor {
    fn from(provider: GitHostingProvider) -> Self {
        match provider {
            GitHostingProvider::GitHub => Self::GitHub,
            GitHostingProvider::GitLab => Self::GitLab,
            GitHostingProvider::Gitea => Self::Gitea,
            GitHostingProvider::Forgejo => Self::Forgejo,
            GitHostingProvider::Bitbucket => Self::Bitbucket,
            GitHostingProvider::AzureDevOps => Self::AzureDevOps,
            GitHostingProvider::AwsCodeCommit => Self::AwsCodeCommit,
            GitHostingProvider::SourceHut => Self::SourceHut,
            GitHostingProvider::SelfHosted | GitHostingProvider::Unknown => Self::Unknown,
        }
    }
}

/// Normalized transport origin of a configured remote URL.
///
/// Captures what the configured URL actually said — scheme, host, and any
/// explicitly configured non-default port (`url::Url::port()` semantics, so a
/// default port normalizes to `None`). Self-managed servers routinely live on
/// `http://` or a non-default port, and provider API bases must be derived
/// from this origin rather than from the bare hostname.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEndpoint {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
}

impl RemoteEndpoint {
    /// The configured HTTP(S) origin (`scheme://host[:port]`), when there is one.
    ///
    /// `None` for non-HTTP transports: an `ssh://` port is an SSH port, not an
    /// API port, so those remotes keep the canonical `https://{host}` API
    /// assumption instead of inheriting a transport port.
    pub fn http_origin(&self) -> Option<String> {
        if !matches!(self.scheme.as_str(), "http" | "https") {
            return None;
        }
        Some(match self.port {
            Some(port) => format!("{}://{}:{port}", self.scheme, self.host),
            None => format!("{}://{}", self.scheme, self.host),
        })
    }
}

/// Fully resolved identity of one configured Git remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRemote {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
    pub host: Option<String>,
    pub namespace: Option<String>,
    pub repository: Option<String>,
    pub api_flavor: ApiFlavor,
    /// Transport origin of `fetch_url`; `None` when no host could be parsed.
    #[serde(default)]
    pub endpoint: Option<RemoteEndpoint>,
}

impl ResolvedRemote {
    /// The configured HTTP(S) origin of the fetch URL, if any.
    pub fn http_origin(&self) -> Option<String> {
        self.endpoint.as_ref().and_then(RemoteEndpoint::http_origin)
    }
}

/// Resolves either an exact configured remote or the repository's preferred remote.
///
/// Preferred selection considers only remotes with usable URLs and orders them
/// as `origin`, alphabetically first non-`upstream`, then `upstream`.
pub fn resolve_remote_at(path: &Path, requested: Option<&str>) -> Result<Option<ResolvedRemote>> {
    let Some(repo) = open::trusted_discover(path)? else {
        return Ok(None);
    };
    let mut names = repo
        .remote_names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    names.sort();

    let selected = if let Some(name) = requested {
        if !names.iter().any(|candidate| candidate == name) {
            return Err(SniffError::RemoteNotConfigured { name: name.to_string() });
        }
        if configured_url(&repo, name).is_none() {
            return Err(SniffError::RemoteUrlMissing { name: name.to_string() });
        }
        Some(name.to_string())
    } else {
        let usable = names
            .iter()
            .filter(|name| configured_url(&repo, name).is_some())
            .map(String::as_str)
            .collect::<Vec<_>>();
        select_preferred_remote(usable).map(str::to_string)
    };
    selected.map(|name| resolve_named(&repo, name)).transpose()
}

/// Applies the preferred-remote order to names already known to have URLs.
///
/// This is the single ordering authority behind both [`resolve_remote_at`] and
/// the aggregate `GitRepo` projections. Callers must pre-filter out remotes
/// without a usable URL; a URL-less remote is never preferred over one that
/// can actually be contacted.
///
/// ## Notes
///
/// Order is `origin`, then the alphabetically first non-`upstream` remote,
/// then `upstream`.
pub(super) fn select_preferred_remote<'a>(
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let mut names = candidates.into_iter().collect::<Vec<_>>();
    names.sort_unstable();
    names
        .iter()
        .find(|name| **name == "origin")
        .or_else(|| names.iter().find(|name| **name != "upstream"))
        .or_else(|| names.first())
        .copied()
}

fn configured_url(repo: &gix::Repository, name: &str) -> Option<String> {
    repo.config_snapshot()
        .string(format!("remote.{name}.url").as_str())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
}

fn resolve_named(repo: &gix::Repository, name: String) -> Result<ResolvedRemote> {
    let fetch_url = configured_url(repo, &name)
        .ok_or_else(|| SniffError::RemoteUrlMissing { name: name.clone() })?;
    let push_url = repo.config_snapshot()
        .string(format!("remote.{name}.pushurl").as_str())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fetch_url.clone());
    let (endpoint, namespace, repository) = parse_identity(&fetch_url);
    let api_flavor = GitHostingProvider::from_url(&fetch_url).into();
    Ok(ResolvedRemote {
        name,
        fetch_url,
        push_url,
        host: endpoint.as_ref().map(|endpoint| endpoint.host.clone()),
        namespace,
        repository,
        api_flavor,
        endpoint,
    })
}

fn parse_identity(remote: &str) -> (Option<RemoteEndpoint>, Option<String>, Option<String>) {
    let (endpoint, path) = if let Ok(url) = url::Url::parse(remote) {
        (
            url.host_str().map(|host| RemoteEndpoint {
                scheme: url.scheme().to_string(),
                host: host.to_string(),
                port: url.port(),
            }),
            url.path().trim_matches('/').to_string(),
        )
    } else if let Some((_, after_at)) = remote.split_once('@') {
        let Some((host, path)) = after_at.split_once(':') else {
            return (None, None, None);
        };
        (
            Some(RemoteEndpoint {
                scheme: "ssh".to_string(),
                host: host.to_string(),
                port: None,
            }),
            path.trim_matches('/').to_string(),
        )
    } else {
        return (None, None, None);
    };
    let mut segments = path.split('/').filter(|segment| !segment.is_empty()).collect::<Vec<_>>();
    let repository = segments.pop().map(|segment| segment.trim_end_matches(".git").to_string());
    let namespace = (!segments.is_empty()).then(|| segments.join("/"));
    (endpoint, namespace, repository)
}
