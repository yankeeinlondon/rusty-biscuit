//! Live, read-only observations of configured Git remotes.

use std::path::Path;

use biscuit_file::{Conditional, FetchError, FetchPolicy, PolicyClient, fetch_blocking};
use gix::bstr::ByteSlice;

use super::{ApiFlavor, resolve_remote_at};
use crate::{Result, SniffError};

/// Checks a branch against the remote's live Git ref advertisement.
///
/// This performs no fetch and does not update local refs, objects, config, the
/// index, or the worktree. HTTP(S) remotes use their configured URL. Supported
/// SSH-only provider URLs use either the provider's branch API or its canonical
/// HTTPS Git endpoint; other transports return an explicit capability error.
pub fn branch_exists_on_remote_at(
    path: &Path,
    branch: Option<&str>,
    remote: Option<&str>,
    policy: &FetchPolicy,
) -> Result<bool> {
    let Some(resolved) = resolve_remote_at(path, remote)? else {
        return Ok(false);
    };
    let branch = match branch.filter(|branch| !branch.is_empty()) {
        Some(branch) => normalize_branch(branch)?,
        None => current_branch(path)?,
    };
    let url = match smart_http_advertisement_url(&resolved) {
        Ok(url) => url,
        Err(SniffError::UnsupportedRemoteCapability { .. }) => {
            return provider_branch_exists(&resolved, &branch, policy);
        }
        Err(error) => return Err(error),
    };
    let response = fetch_git_advertisement(&resolved, &url, policy)?;
    if !(200..=299).contains(&response.0) {
        return Err(status_error(resolved.api_flavor, response.0));
    }
    Ok(advertises_branch(&response.1, &branch))
}

fn normalize_branch(branch: &str) -> Result<String> {
    if branch.starts_with("refs/") && !branch.starts_with("refs/heads/") {
        return Err(SniffError::git(
            "remote_branch_name",
            std::io::Error::other("branch must name a local heads ref"),
        ));
    }
    let short = branch.strip_prefix("refs/heads/").unwrap_or(branch);
    if short.len() == 40 && short.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SniffError::git(
            "remote_branch_name",
            std::io::Error::other("object IDs are not branch names"),
        ));
    }
    let full = format!("refs/heads/{short}");
    let full_name = gix::refs::FullName::try_from(full.as_str())
        .map_err(|error| SniffError::git("remote_branch_name", error))?;
    gix::validate::reference::branch_name(full_name.as_bstr())
        .map_err(|error| SniffError::git("remote_branch_name", error))?;
    Ok(short.to_string())
}

/// Returns the canonical provider token for a configured remote.
///
/// Deterministic URL classifications do not perform network I/O. Ambiguous
/// self-hosted URLs are probed only after the exact host passes `policy`.
pub fn remote_vendor_at(
    path: &Path,
    remote: Option<&str>,
    policy: &FetchPolicy,
) -> Result<String> {
    let Some(resolved) = resolve_remote_at(path, remote)? else {
        return Ok(String::new());
    };
    let flavor = match resolved.api_flavor {
        ApiFlavor::Unknown => probe_self_hosted_flavor(&resolved.fetch_url, policy)?,
        flavor => flavor,
    };
    let token = match flavor {
        ApiFlavor::GitHub => "github",
        ApiFlavor::GitLab => "gitlab",
        ApiFlavor::Gitea => "gitea",
        ApiFlavor::Forgejo => "forgejo",
        ApiFlavor::Bitbucket => "bitbucket",
        ApiFlavor::AzureDevOps => "azure_devops",
        ApiFlavor::AwsCodeCommit => "aws_code_commit",
        ApiFlavor::SourceHut => "source_hut",
        // The probe never returns `Unknown`; it errors instead.
        ApiFlavor::Unknown => {
            return Err(SniffError::UnsupportedProvider { url: resolved.fetch_url });
        }
    };
    Ok(token.to_string())
}

fn current_branch(path: &Path) -> Result<String> {
    let Some(repo) = super::open::trusted_discover(path)? else {
        return Err(SniffError::NotARepository(path.to_path_buf()));
    };
    let head = repo.head().map_err(|error| SniffError::git("head", error))?;
    let name = head
        .referent_name()
        .and_then(|name| name.shorten().to_str().ok())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| SniffError::git("current_branch", std::io::Error::other("HEAD is detached or unborn")))?;
    Ok(name.to_string())
}

fn smart_http_advertisement_url(remote: &super::ResolvedRemote) -> Result<url::Url> {
    let mut url = match url::Url::parse(&remote.fetch_url) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url,
        _ => provider_https_git_url(remote)?,
    };
    if !matches!(url.scheme(), "http" | "https") {
        return Err(SniffError::UnsupportedRemoteCapability {
            capability: "live branch observation",
            target: url.scheme().to_string(),
        });
    }
    let path = format!("{}/info/refs", url.path().trim_end_matches('/'));
    url.set_path(&path);
    url.set_query(Some("service=git-upload-pack"));
    Ok(url)
}

fn provider_https_git_url(remote: &super::ResolvedRemote) -> Result<url::Url> {
    let host = remote.host.as_deref().unwrap_or_default();
    let path = remote_git_path(&remote.fetch_url).unwrap_or_default();
    let endpoint = match remote.api_flavor {
        ApiFlavor::AzureDevOps => {
            let segments = path
                .strip_prefix("v3/")
                .unwrap_or(&path)
                .split('/')
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>();
            if segments.len() != 3 {
                return unsupported_provider_git_endpoint(remote);
            }
            format!(
                "https://dev.azure.com/{}/{}/_git/{}",
                urlencoding::encode(segments[0]),
                urlencoding::encode(segments[1]),
                urlencoding::encode(segments[2].trim_end_matches(".git"))
            )
        }
        ApiFlavor::AwsCodeCommit if !host.is_empty() && path.starts_with("v1/repos/") => {
            format!("https://{host}/{path}")
        }
        ApiFlavor::SourceHut if host.ends_with(".sr.ht") && !path.is_empty() => {
            format!("https://{host}/{path}")
        }
        _ => return unsupported_provider_git_endpoint(remote),
    };
    url::Url::parse(&endpoint).map_err(|error| unreachable_url(&endpoint, error))
}

fn remote_git_path(remote: &str) -> Option<String> {
    if let Ok(url) = url::Url::parse(remote) {
        return Some(url.path().trim_matches('/').to_string());
    }
    let (_, after_at) = remote.split_once('@')?;
    let (_, path) = after_at.split_once(':')?;
    Some(path.trim_matches('/').to_string())
}

fn unsupported_provider_git_endpoint<T>(remote: &super::ResolvedRemote) -> Result<T> {
    Err(SniffError::UnsupportedRemoteCapability {
        capability: "live branch observation",
        target: remote.fetch_url.clone(),
    })
}

fn fetch_git_advertisement(
    remote: &super::ResolvedRemote,
    url: &url::Url,
    policy: &FetchPolicy,
) -> Result<(u16, Vec<u8>)> {
    let remote_host = remote.host.as_deref().unwrap_or_default();
    if !policy.is_allowed(remote_host) {
        return Err(SniffError::RemotePolicyDenied { host: remote_host.to_string() });
    }
    let endpoint_host = url.host_str().unwrap_or_default();
    if !provider_endpoint_allowed(remote_host, endpoint_host, remote.api_flavor) {
        return Err(SniffError::RemotePolicyDenied { host: endpoint_host.to_string() });
    }

    if url::Url::parse(&remote.fetch_url)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
    {
        let effective_policy = policy.clone().allow_host(endpoint_host);
        let client = PolicyClient::new().map_err(|error| unreachable(url, error))?;
        let response = fetch_blocking(&client, url, &effective_policy, &Conditional::default())
            .map_err(|error| map_fetch_error(url, remote.api_flavor, error))?;
        return Ok((response.status, response.body.to_vec()));
    }

    let mut request = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| unreachable(url, error))?
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "sniff/remote-observation");
    request = match remote.api_flavor {
        ApiFlavor::AzureDevOps => optional_basic_token(request, "AZURE_DEVOPS_TOKEN"),
        ApiFlavor::AwsCodeCommit => codecommit_git_credentials(request)?,
        ApiFlavor::SourceHut => optional_bearer(request, &["SOURCEHUT_TOKEN"]),
        _ => request,
    };
    let response = request.send().map_err(|error| unreachable(url, error))?;
    let status = response.status().as_u16();
    let body = response.bytes().map_err(|error| unreachable(url, error))?.to_vec();
    Ok((status, body))
}

fn advertises_branch(body: &[u8], branch: &str) -> bool {
    let target = format!("refs/heads/{branch}");
    let mut cursor = 0;
    while cursor + 4 <= body.len() {
        let Ok(length_text) = std::str::from_utf8(&body[cursor..cursor + 4]) else {
            break;
        };
        let Ok(length) = usize::from_str_radix(length_text, 16) else {
            break;
        };
        cursor += 4;
        if length == 0 {
            continue;
        }
        if length < 4 || cursor + length - 4 > body.len() {
            break;
        }
        let packet = &body[cursor..cursor + length - 4];
        cursor += length - 4;
        let text = String::from_utf8_lossy(packet);
        let ref_name = text
            .split_once(' ')
            .map(|(_, rest)| rest)
            .unwrap_or_default()
            .split(['\0', '\n'])
            .next()
            .unwrap_or_default();
        if ref_name == target {
            return true;
        }
    }
    false
}

/// Detects a self-managed server family behind an ambiguous HTTP(S) host.
///
/// This is the single bounded discovery probe: `remote_vendor_at` projects its
/// result to a vendor token, and the focused provider client reuses it to
/// select a flavor for a neutral-hostname endpoint. The exact host must pass
/// `policy` before any request; the transport is the deny-by-default,
/// redirect-disabled `PolicyClient`. Blocking — callers inside an async
/// runtime must move it to a blocking thread.
pub(crate) fn probe_self_hosted_flavor(remote: &str, policy: &FetchPolicy) -> Result<ApiFlavor> {
    let remote = url::Url::parse(remote)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .ok_or(SniffError::UnsupportedRemoteCapability {
            capability: "provider detection",
            target: "non-HTTP Git transport".to_string(),
        })?;
    let host = remote.host_str().unwrap_or_default();
    if !policy.is_allowed(host) {
        return Err(SniffError::RemotePolicyDenied { host: host.to_string() });
    }
    let client = PolicyClient::new().map_err(|error| unreachable(&remote, error))?;
    for (path, flavor) in [
        ("/api/v4/version", ApiFlavor::GitLab),
        ("/api/v1/version", ApiFlavor::Gitea),
    ] {
        let mut endpoint = remote.clone();
        endpoint.set_path(path);
        endpoint.set_query(None);
        let response = match fetch_blocking(&client, &endpoint, policy, &Conditional::default()) {
            Ok(response) => response,
            Err(FetchError::HttpError { status: 404, .. }) => continue,
            Err(error) => return Err(map_fetch_error(&endpoint, ApiFlavor::Unknown, error)),
        };
        if response.is_success() {
            // Forgejo serves Gitea's API surface; only its version body tells
            // the two apart.
            if flavor == ApiFlavor::Gitea
                && String::from_utf8_lossy(&response.body)
                    .to_ascii_lowercase()
                    .contains("forgejo")
            {
                return Ok(ApiFlavor::Forgejo);
            }
            return Ok(flavor);
        }
    }
    Err(SniffError::UnsupportedProvider { url: remote.to_string() })
}

fn provider_branch_exists(
    remote: &super::ResolvedRemote,
    branch: &str,
    policy: &FetchPolicy,
) -> Result<bool> {
    let endpoint = provider_branch_endpoint(remote, branch)?;
    let remote_host = remote.host.as_deref().unwrap_or_default();
    if !policy.is_allowed(remote_host) {
        return Err(SniffError::RemotePolicyDenied {
            host: remote_host.to_string(),
        });
    }
    let endpoint_host = endpoint.host_str().unwrap_or_default();
    if !provider_endpoint_allowed(remote_host, endpoint_host, remote.api_flavor) {
        return Err(SniffError::RemotePolicyDenied {
            host: endpoint_host.to_string(),
        });
    }

    let mut request = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| unreachable(&endpoint, error))?
        .get(endpoint.clone())
        .header(reqwest::header::USER_AGENT, "sniff/remote-observation");
    request = match remote.api_flavor {
        ApiFlavor::GitHub => optional_bearer(request, &["GH_TOKEN", "GITHUB_TOKEN"]),
        ApiFlavor::GitLab => optional_bearer(request, &["GITLAB_TOKEN"]),
        ApiFlavor::Gitea | ApiFlavor::Forgejo => {
            optional_bearer(request, &["GITEA_TOKEN", "FORGEJO_TOKEN"])
        }
        ApiFlavor::Bitbucket => optional_bearer(request, &["BITBUCKET_TOKEN"]),
        _ => request,
    };
    let response = request.send().map_err(|error| unreachable(&endpoint, error))?;
    let status = response.status().as_u16();
    match status {
        200..=299 => Ok(true),
        404 => Ok(false),
        status => Err(status_error(remote.api_flavor, status)),
    }
}

fn provider_endpoint_allowed(remote_host: &str, endpoint_host: &str, flavor: ApiFlavor) -> bool {
    remote_host.eq_ignore_ascii_case(endpoint_host)
        || (flavor == ApiFlavor::GitHub
            && remote_host.eq_ignore_ascii_case("github.com")
            && endpoint_host.eq_ignore_ascii_case("api.github.com"))
        || (flavor == ApiFlavor::Bitbucket
            && remote_host.eq_ignore_ascii_case("bitbucket.org")
            && endpoint_host.eq_ignore_ascii_case("api.bitbucket.org"))
        || (flavor == ApiFlavor::AzureDevOps
            && (remote_host.eq_ignore_ascii_case("ssh.dev.azure.com")
                || remote_host.eq_ignore_ascii_case("vs-ssh.visualstudio.com"))
            && endpoint_host.eq_ignore_ascii_case("dev.azure.com"))
}

fn provider_branch_endpoint(remote: &super::ResolvedRemote, branch: &str) -> Result<url::Url> {
    let host = remote.host.as_deref().unwrap_or_default();
    let namespace = remote.namespace.as_deref().unwrap_or_default();
    let repository = remote.repository.as_deref().unwrap_or_default();
    if host.is_empty() || namespace.is_empty() || repository.is_empty() {
        return Err(SniffError::UnsupportedRemoteCapability {
            capability: "provider branch lookup",
            target: remote.fetch_url.clone(),
        });
    }
    let branch = urlencoding::encode(branch);
    let namespace = urlencoding::encode(namespace);
    let repository = urlencoding::encode(repository);
    let endpoint = match remote.api_flavor {
        ApiFlavor::GitHub if host.eq_ignore_ascii_case("github.com") => format!(
            "https://api.github.com/repos/{namespace}/{repository}/branches/{branch}"
        ),
        ApiFlavor::GitHub => format!(
            "https://{host}/api/v3/repos/{namespace}/{repository}/branches/{branch}"
        ),
        ApiFlavor::GitLab => format!(
            "https://{host}/api/v4/projects/{namespace}%2F{repository}/repository/branches/{branch}"
        ),
        ApiFlavor::Gitea | ApiFlavor::Forgejo => format!(
            "https://{host}/api/v1/repos/{namespace}/{repository}/branches/{branch}"
        ),
        ApiFlavor::Bitbucket => format!(
            "https://api.bitbucket.org/2.0/repositories/{namespace}/{repository}/refs/branches/{branch}"
        ),
        _ => {
            return Err(SniffError::UnsupportedRemoteCapability {
                capability: "provider branch lookup",
                target: format!("{:?}", remote.api_flavor),
            });
        }
    };
    url::Url::parse(&endpoint).map_err(|error| unreachable_url(&endpoint, error))
}

fn optional_bearer(
    mut request: reqwest::blocking::RequestBuilder,
    variables: &[&str],
) -> reqwest::blocking::RequestBuilder {
    if let Some(token) = variables.iter().find_map(|name| std::env::var(name).ok()) {
        request = request.bearer_auth(token);
    }
    request
}

fn optional_basic_token(
    request: reqwest::blocking::RequestBuilder,
    variable: &str,
) -> reqwest::blocking::RequestBuilder {
    match std::env::var(variable) {
        Ok(token) => request.basic_auth("", Some(token)),
        Err(_) => request,
    }
}

fn codecommit_git_credentials(
    request: reqwest::blocking::RequestBuilder,
) -> Result<reqwest::blocking::RequestBuilder> {
    let username = std::env::var("AWS_CODECOMMIT_GIT_USERNAME").ok();
    let password = std::env::var("AWS_CODECOMMIT_GIT_PASSWORD").ok();
    match (username, password) {
        (Some(username), Some(password)) => Ok(request.basic_auth(username, Some(password))),
        _ => Err(SniffError::InvalidCredentials {
            provider: "AWS CodeCommit".to_string(),
            message: "set AWS_CODECOMMIT_GIT_USERNAME and AWS_CODECOMMIT_GIT_PASSWORD"
                .to_string(),
        }),
    }
}

fn unreachable(url: &url::Url, error: impl std::fmt::Display) -> SniffError {
    let mut sanitized = url.clone();
    let _ = sanitized.set_username("");
    let _ = sanitized.set_password(None);
    SniffError::RemoteUnreachable { url: sanitized.to_string(), message: error.to_string() }
}

fn unreachable_url(url: &str, error: impl std::fmt::Display) -> SniffError {
    SniffError::RemoteUnreachable { url: url.to_string(), message: error.to_string() }
}

fn status_error(provider: ApiFlavor, status: u16) -> SniffError {
    let provider = format!("{provider:?}");
    match status {
        401 => SniffError::InvalidCredentials {
            provider,
            message: "Git ref advertisement requires authentication".to_string(),
        },
        403 => SniffError::RemoteForbidden {
            provider,
            message: "Git ref advertisement was forbidden".to_string(),
        },
        429 => SniffError::RateLimited { provider, retry_after: None },
        status => SniffError::RemoteApi {
            provider,
            status,
            message: "Git ref advertisement failed".to_string(),
        },
    }
}

fn map_fetch_error(url: &url::Url, provider: ApiFlavor, error: FetchError) -> SniffError {
    match error {
        FetchError::HttpError { status, .. } => status_error(provider, status),
        FetchError::PolicyDenied { host } => SniffError::RemotePolicyDenied { host },
        other => unreachable(url, other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::git::ResolvedRemote;

    fn remote(api_flavor: ApiFlavor, host: &str) -> ResolvedRemote {
        ResolvedRemote {
            name: "origin".to_string(),
            fetch_url: format!("git@{host}:group/nested/project.git"),
            push_url: format!("git@{host}:group/nested/project.git"),
            host: Some(host.to_string()),
            namespace: Some("group/nested".to_string()),
            repository: Some("project".to_string()),
            api_flavor,
            endpoint: None,
        }
    }

    #[test]
    fn provider_fallback_endpoints_preserve_nested_identity_and_exact_branch() {
        let cases = [
            (ApiFlavor::GitHub, "github.com", "https://api.github.com/repos/group%2Fnested/project/branches/release%2Fv1"),
            (ApiFlavor::GitLab, "gitlab.com", "https://gitlab.com/api/v4/projects/group%2Fnested%2Fproject/repository/branches/release%2Fv1"),
            (ApiFlavor::Gitea, "gitea.example.com", "https://gitea.example.com/api/v1/repos/group%2Fnested/project/branches/release%2Fv1"),
            (ApiFlavor::Forgejo, "forgejo.example.com", "https://forgejo.example.com/api/v1/repos/group%2Fnested/project/branches/release%2Fv1"),
            (ApiFlavor::Bitbucket, "bitbucket.org", "https://api.bitbucket.org/2.0/repositories/group%2Fnested/project/refs/branches/release%2Fv1"),
        ];
        for (flavor, host, expected) in cases {
            assert_eq!(
                provider_branch_endpoint(&remote(flavor, host), "release/v1").unwrap().as_str(),
                expected
            );
        }
    }

    #[test]
    fn ssh_only_providers_map_to_canonical_https_git_endpoints() {
        let cases = [
            (
                remote(ApiFlavor::AzureDevOps, "ssh.dev.azure.com"),
                "git@ssh.dev.azure.com:v3/acme/widgets/project.git",
                "https://dev.azure.com/acme/widgets/_git/project",
            ),
            (
                remote(ApiFlavor::AwsCodeCommit, "git-codecommit.us-west-2.amazonaws.com"),
                "ssh://git-codecommit.us-west-2.amazonaws.com/v1/repos/project",
                "https://git-codecommit.us-west-2.amazonaws.com/v1/repos/project",
            ),
            (
                remote(ApiFlavor::SourceHut, "git.sr.ht"),
                "git@git.sr.ht:~acme/project",
                "https://git.sr.ht/~acme/project",
            ),
        ];
        for (mut remote, fetch_url, expected) in cases {
            remote.fetch_url = fetch_url.to_string();
            let url = provider_https_git_url(&remote).unwrap();
            assert_eq!(url.as_str().trim_end_matches('/'), expected);
        }
    }

    #[test]
    fn branch_normalization_accepts_only_local_head_names() {
        assert_eq!(normalize_branch("main").unwrap(), "main");
        assert_eq!(normalize_branch("refs/heads/release/v1").unwrap(), "release/v1");
        for invalid in [
            "refs/tags/v1",
            "refs/remotes/origin/main",
            "../main",
            "main..next",
            "0123456789012345678901234567890123456789",
        ] {
            assert!(normalize_branch(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn packet_parser_matches_only_exact_head_refs() {
        let mut body = Vec::new();
        for name in ["refs/heads/main-old", "refs/heads/main"] {
            let packet = format!("0000000000000000000000000000000000000000 {name}\n");
            body.extend_from_slice(format!("{:04x}", packet.len() + 4).as_bytes());
            body.extend_from_slice(packet.as_bytes());
        }
        body.extend_from_slice(b"0000");
        assert!(advertises_branch(&body, "main"));
        assert!(!advertises_branch(&body, "mai"));
    }

    #[test]
    fn provider_endpoint_policy_has_only_narrow_official_host_mappings() {
        assert!(provider_endpoint_allowed(
            "github.com",
            "api.github.com",
            ApiFlavor::GitHub
        ));
        assert!(provider_endpoint_allowed(
            "ssh.dev.azure.com",
            "dev.azure.com",
            ApiFlavor::AzureDevOps
        ));
        assert!(provider_endpoint_allowed(
            "bitbucket.org",
            "api.bitbucket.org",
            ApiFlavor::Bitbucket
        ));
        assert!(!provider_endpoint_allowed(
            "github.com",
            "evil.example",
            ApiFlavor::GitHub
        ));
    }
}
