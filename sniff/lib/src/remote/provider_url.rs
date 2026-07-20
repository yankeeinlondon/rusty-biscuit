//! Canonical provider item-reference URL parsing.
//!
//! A canonical provider URL is a *web or API* URL carrying enough host,
//! repository, and item identity to resolve a pull request or CI/CD job without
//! the caller's repository. This module is the single authority for that input
//! half of the contract; [`super::url_parser`] handles the unrelated problem of
//! parsing a Git *clone* URL.
//!
//! Routes are matched per flavor rather than by scanning for a shared marker
//! segment. The marker approach cannot separate GitHub's API `/repos/{o}/{r}/
//! pulls/{n}` from Gitea's web `/{o}/{r}/pulls/{n}` — the two spell the same
//! token at different positions — and it silently accepts one provider's route
//! shape on another provider's host.

use crate::error::SniffError;
use crate::filesystem::git::{ApiFlavor, RemoteEndpoint, ResolvedRemote};

/// Which item family a canonical URL is expected to address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReferenceKind {
    PullRequest,
    CiCdJob,
}

/// One canonical URL resolved to its repository identity and native item ID.
struct Route {
    flavor: ApiFlavor,
    namespace: String,
    repository: String,
    native_id: String,
}

/// Resolves a canonical provider web or API URL into a remote plus native ID.
///
/// The returned [`ResolvedRemote`] carries the URL's own scheme and non-default
/// port, so an enterprise or self-managed endpoint keeps the origin the caller
/// addressed it by. Official API hostnames are mapped back to their web host
/// (`api.github.com` → `github.com`) because the remote host is the policy and
/// repository-identity key, not the API endpoint.
///
/// ## Errors
///
/// Returns [`SniffError::InvalidRemoteQuery`] for a non-HTTP(S) URL, a URL
/// carrying a query or fragment, a route shape no supported provider speaks,
/// and a route shape belonging to a different provider than the host pins.
pub(crate) fn parse_provider_url(
    raw: &str,
    kind: ReferenceKind,
) -> Result<(ResolvedRemote, String), SniffError> {
    let url = url::Url::parse(raw).map_err(|error| {
        invalid(format!(
            "expected a positive native ID or canonical provider URL: {error}"
        ))
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.query().is_some() || url.fragment().is_some()
    {
        return Err(invalid(
            "canonical provider URLs must be HTTP(S) URLs without query or fragment",
        ));
    }
    let url_host = url
        .host_str()
        .ok_or_else(|| invalid("canonical provider URL is missing a host"))?
        .to_ascii_lowercase();
    let segments = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let route = resolve_route(&url_host, &segments, kind)
        .ok_or_else(|| invalid("URL is not a canonical supported-provider reference"))?;
    // No provider numbers an item zero, so a well-formed route carrying `0` is a
    // constructed URL rather than a reference to anything that exists.
    if route.native_id == "0" {
        return Err(invalid("must be a positive provider identifier"));
    }

    let host = repository_host(&url_host);
    let origin = match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    };
    let repository_url = format!(
        "{origin}/{}/{}.git",
        route.namespace, route.repository
    );
    Ok((
        ResolvedRemote {
            name: raw.to_string(),
            fetch_url: repository_url.clone(),
            push_url: repository_url,
            host: Some(host.clone()),
            namespace: Some(route.namespace),
            repository: Some(route.repository),
            api_flavor: route.flavor,
            endpoint: Some(RemoteEndpoint {
                scheme: url.scheme().to_string(),
                host,
                port: url.port(),
            }),
        },
        route.native_id,
    ))
}

/// Selects the flavor whose route grammar the URL must satisfy, then applies it.
///
/// Flavor selection is deliberately narrowed before any route matching: a host
/// that pins a provider accepts only that provider's routes, so a GitLab-shaped
/// path on `github.com` is a rejection rather than a mis-flavored success.
fn resolve_route(host: &str, segments: &[&str], kind: ReferenceKind) -> Option<Route> {
    let (api_flavor, rest) = strip_api_prefix(host, segments);
    let pinned = pinned_flavor(host);
    if let (Some(api), Some(pin)) = (api_flavor, pinned)
        && !same_family(api, pin)
    {
        return None;
    }
    let candidates: &[ApiFlavor] = match (api_flavor, pinned) {
        (Some(flavor), _) | (None, Some(flavor)) => &[flavor],
        (None, None) => &[
            ApiFlavor::GitHub,
            ApiFlavor::GitLab,
            gitea_family(host),
            ApiFlavor::Bitbucket,
        ],
    };
    let is_api = api_flavor.is_some();
    candidates
        .iter()
        .find_map(|flavor| flavor_route(*flavor, rest, kind, is_api))
}

/// Splits a recognized API-version prefix off the path.
///
/// The prefix is the only reliable discriminator between GitHub's and
/// Gitea/Forgejo's API grammars, which are otherwise byte-identical
/// (`/repos/{owner}/{repo}/pulls/{n}`).
fn strip_api_prefix<'a>(host: &str, segments: &'a [&'a str]) -> (Option<ApiFlavor>, &'a [&'a str]) {
    match segments {
        ["api", "v3", rest @ ..] => (Some(ApiFlavor::GitHub), rest),
        ["api", "v4", rest @ ..] => (Some(ApiFlavor::GitLab), rest),
        ["api", "v1", rest @ ..] => (Some(gitea_family(host)), rest),
        ["2.0", rest @ ..] => (Some(ApiFlavor::Bitbucket), rest),
        rest if host == "api.github.com" => (Some(ApiFlavor::GitHub), rest),
        rest => (None, rest),
    }
}

/// The flavor a well-known SaaS hostname guarantees, if any.
fn pinned_flavor(host: &str) -> Option<ApiFlavor> {
    match host {
        "github.com" | "www.github.com" | "api.github.com" => Some(ApiFlavor::GitHub),
        "gitlab.com" | "www.gitlab.com" => Some(ApiFlavor::GitLab),
        "bitbucket.org" | "www.bitbucket.org" | "api.bitbucket.org" => Some(ApiFlavor::Bitbucket),
        "codeberg.org" => Some(ApiFlavor::Forgejo),
        _ => None,
    }
}

/// Forgejo is a Gitea fork with an identical route grammar on both surfaces, so
/// only the hostname can separate them.
fn gitea_family(host: &str) -> ApiFlavor {
    if host.contains("forgejo") || host.contains("codeberg") {
        ApiFlavor::Forgejo
    } else {
        ApiFlavor::Gitea
    }
}

fn same_family(left: ApiFlavor, right: ApiFlavor) -> bool {
    left == right
        || matches!(
            (left, right),
            (ApiFlavor::Gitea, ApiFlavor::Forgejo) | (ApiFlavor::Forgejo, ApiFlavor::Gitea)
        )
}

/// Official API hostnames resolve to the repository's web host.
fn repository_host(host: &str) -> String {
    match host {
        "api.github.com" => "github.com".to_string(),
        "api.bitbucket.org" => "bitbucket.org".to_string(),
        other => other.to_string(),
    }
}

/// Matches one flavor's web or API route grammar for one item kind.
fn flavor_route(
    flavor: ApiFlavor,
    segments: &[&str],
    kind: ReferenceKind,
    is_api: bool,
) -> Option<Route> {
    use ApiFlavor::{Bitbucket, Forgejo, Gitea, GitHub, GitLab};
    use ReferenceKind::{CiCdJob, PullRequest};

    match (flavor, kind, is_api) {
        (GitHub, PullRequest, true) => match segments {
            ["repos", owner, repository, "pulls", id] => flat(flavor, owner, repository, id),
            _ => None,
        },
        (GitHub, PullRequest, false) => match segments {
            [owner, repository, "pull", id] => flat(flavor, owner, repository, id),
            _ => None,
        },
        (GitHub, CiCdJob, true) => match segments {
            ["repos", owner, repository, "actions", "jobs", id] => {
                flat(flavor, owner, repository, id)
            }
            _ => None,
        },
        (GitHub, CiCdJob, false) => match segments {
            [owner, repository, "actions", "runs", _run, "job", id] => {
                flat(flavor, owner, repository, id)
            }
            _ => None,
        },

        (Gitea | Forgejo, PullRequest, true) => match segments {
            ["repos", owner, repository, "pulls", id] => flat(flavor, owner, repository, id),
            _ => None,
        },
        (Gitea | Forgejo, PullRequest, false) => match segments {
            [owner, repository, "pulls", id] => flat(flavor, owner, repository, id),
            _ => None,
        },
        (Gitea | Forgejo, CiCdJob, true) => match segments {
            ["repos", owner, repository, "actions", "jobs", id] => {
                flat(flavor, owner, repository, id)
            }
            _ => None,
        },
        (Gitea | Forgejo, CiCdJob, false) => match segments {
            [owner, repository, "actions", "runs", _run, "jobs", id] => {
                flat(flavor, owner, repository, id)
            }
            _ => None,
        },

        (GitLab, PullRequest, true) => match segments {
            ["projects", project, "merge_requests", id] => encoded_project(project, id),
            _ => None,
        },
        (GitLab, PullRequest, false) => match segments {
            [path @ .., "-", "merge_requests", id] => project_path(path, id),
            _ => None,
        },
        (GitLab, CiCdJob, true) => match segments {
            ["projects", project, "jobs", id] => encoded_project(project, id),
            _ => None,
        },
        (GitLab, CiCdJob, false) => match segments {
            [path @ .., "-", "jobs", id] => project_path(path, id),
            _ => None,
        },

        (Bitbucket, PullRequest, true) => match segments {
            ["repositories", workspace, repository, "pullrequests", id] => {
                flat(flavor, workspace, repository, id)
            }
            _ => None,
        },
        (Bitbucket, PullRequest, false) => match segments {
            [workspace, repository, "pull-requests", id] => {
                flat(flavor, workspace, repository, id)
            }
            _ => None,
        },
        // A Bitbucket step is identified by its pipeline, so the native ID this
        // parser hands back is the composite `pipeline/step` the exact-lookup
        // path expects — not the bare step UUID.
        (Bitbucket, CiCdJob, true) => match segments {
            ["repositories", workspace, repository, "pipelines", parent, "steps", step] => {
                step_route(workspace, repository, parent, step)
            }
            _ => None,
        },
        (Bitbucket, CiCdJob, false) => match segments {
            [workspace, repository, "pipelines", "results", parent, "steps", step] => {
                step_route(workspace, repository, parent, step)
            }
            _ => None,
        },

        _ => None,
    }
}

/// A single-segment owner plus repository, as every non-GitLab flavor spells it.
fn flat(flavor: ApiFlavor, namespace: &str, repository: &str, id: &str) -> Option<Route> {
    Some(Route {
        flavor,
        namespace: identity(namespace)?,
        repository: identity(repository)?,
        native_id: identity(id)?,
    })
}

fn step_route(workspace: &str, repository: &str, parent: &str, step: &str) -> Option<Route> {
    Some(Route {
        flavor: ApiFlavor::Bitbucket,
        namespace: identity(workspace)?,
        repository: identity(repository)?,
        native_id: format!("{}/{}", identity(parent)?, identity(step)?),
    })
}

/// GitLab's API addresses a project by its percent-encoded full path.
///
/// The encoded form is decoded exactly once and then split on its final
/// separator, which is what makes `group%2Fsub%2Fproject` resolve to the same
/// namespace and repository as the equivalent web URL.
fn encoded_project(project: &str, id: &str) -> Option<Route> {
    let decoded = decode(project)?;
    let (namespace, repository) = decoded.rsplit_once('/')?;
    if namespace.is_empty() || repository.is_empty() {
        return None;
    }
    Some(Route {
        flavor: ApiFlavor::GitLab,
        namespace: namespace.to_string(),
        repository: repository.to_string(),
        native_id: identity(id)?,
    })
}

/// GitLab's web URLs spell the same project path as ordinary path segments,
/// so an arbitrarily deep subgroup chain is the namespace.
fn project_path(path: &[&str], id: &str) -> Option<Route> {
    let (repository, namespace) = path.split_last()?;
    if namespace.is_empty() {
        return None;
    }
    let namespace = namespace
        .iter()
        .map(|segment| identity(segment))
        .collect::<Option<Vec<_>>>()?
        .join("/");
    Some(Route {
        flavor: ApiFlavor::GitLab,
        namespace,
        repository: identity(repository)?,
        native_id: identity(id)?,
    })
}

/// Decodes one path segment and rejects anything that is not a usable identity.
///
/// A segment that decodes to an empty string or to something containing a path
/// separator would silently restructure the repository identity, so it is a
/// parse failure rather than a value.
fn identity(segment: &str) -> Option<String> {
    let decoded = decode(segment)?;
    (!decoded.is_empty() && !decoded.contains('/')).then_some(decoded)
}

fn decode(segment: &str) -> Option<String> {
    urlencoding::decode(segment)
        .ok()
        .map(std::borrow::Cow::into_owned)
}

fn invalid(message: impl Into<String>) -> SniffError {
    SniffError::InvalidRemoteQuery {
        field: "id",
        message: message.into(),
    }
}
