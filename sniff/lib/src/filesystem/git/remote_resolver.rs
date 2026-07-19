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
        names
            .iter()
            .find(|name| name.as_str() == "origin" && configured_url(&repo, name).is_some())
            .or_else(|| {
                names.iter().find(|name| {
                    name.as_str() != "upstream" && configured_url(&repo, name).is_some()
                })
            })
            .or_else(|| {
                names.iter().find(|name| {
                    name.as_str() == "upstream" && configured_url(&repo, name).is_some()
                })
            })
            .cloned()
    };
    selected.map(|name| resolve_named(&repo, name)).transpose()
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
    let (host, namespace, repository) = parse_identity(&fetch_url);
    let api_flavor = GitHostingProvider::from_url(&fetch_url).into();
    Ok(ResolvedRemote {
        name,
        fetch_url,
        push_url,
        host,
        namespace,
        repository,
        api_flavor,
    })
}

fn parse_identity(remote: &str) -> (Option<String>, Option<String>, Option<String>) {
    let (host, path) = if let Ok(url) = url::Url::parse(remote) {
        (
            url.host_str().map(str::to_string),
            url.path().trim_matches('/').to_string(),
        )
    } else if let Some((_, after_at)) = remote.split_once('@') {
        let Some((host, path)) = after_at.split_once(':') else {
            return (None, None, None);
        };
        (Some(host.to_string()), path.trim_matches('/').to_string())
    } else {
        return (None, None, None);
    };
    let mut segments = path.split('/').filter(|segment| !segment.is_empty()).collect::<Vec<_>>();
    let repository = segments.pop().map(|segment| segment.trim_end_matches(".git").to_string());
    let namespace = (!segments.is_empty()).then(|| segments.join("/"));
    (host, namespace, repository)
}
