//! Blocking, deadline-bound provider lookups for callers without an async
//! runtime.
//!
//! Each entry point builds its own current-thread Tokio runtime and blocks on
//! it, so none of them may be called from inside an existing Tokio runtime.
//! Such a call returns [`PrUnavailable::Other`] without contacting the
//! provider.
//!
//! The deadline covers the whole lookup, including every page and any
//! auxiliary request, and replaces the focused client's 5 s per-request
//! timeout.

use std::future::Future;
use std::time::{Duration, Instant};

use biscuit_file::FetchPolicy;

use crate::SniffError;
use crate::filesystem::git::commit_links::parse_remote_identity;
use crate::filesystem::git::{ApiFlavor, GitHostingProvider, ResolvedRemote};

use super::types::optional_timestamp_order;
use super::{FocusedProviderClient, PullRequestInfo};

/// Lifecycle states that can count as PR evidence.
///
/// Closed-unmerged, declined, and superseded PRs never count, so they have no
/// variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrState {
    Open,
    Merged,
}

/// The PR chosen as evidence for one branch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PrEvidence {
    pub number: u64,
    /// `None` when the provider sent no link, or one outside the provider's
    /// own host.
    pub html_url: Option<String>,
    pub state: PrState,
    /// The source repository the lookup matched, as the caller spelled it.
    pub source_repo: String,
    /// The PR's head commit exactly as the provider returned it.
    ///
    /// `None` when the provider sent none; such a PR is still reported but
    /// carries no commit evidence. A value shorter than the local object ID
    /// is a prefix (Bitbucket Cloud sends 12 characters), never a full ID.
    /// Gitea and Forgejo values come from the list endpoint, which freezes
    /// the head at merge.
    pub source_head_sha: Option<String>,
    pub target_branch: Option<String>,
}

/// One open PR against a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PrSummary {
    pub number: u64,
    /// `None` when the provider sent no link, or one outside the provider's
    /// own host.
    pub html_url: Option<String>,
    /// `owner/repo` (GitLab: full project path) of the repository the head
    /// branch lives in, as the provider spells it; a fork's differs from the
    /// target's.
    ///
    /// `None` when the provider cannot name it (a deleted fork, or a GitLab
    /// fork project the caller may not see). Such a PR matches no local
    /// branch.
    pub source_repo: Option<String>,
    pub source_branch: Option<String>,
    pub target_branch: Option<String>,
}

/// Why no answer, positive or negative, could be obtained.
///
/// None of these is ever reported as `Ok(None)`: only a provider that answered
/// the query and listed no matching PR produces that.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PrUnavailable {
    /// The deadline passed before the lookup finished.
    #[error("provider lookup did not finish within {deadline:?}")]
    Timeout { deadline: Duration },
    /// The provider could not be reached or the connection failed.
    #[error("provider unreachable: {message}")]
    Network { message: String },
    /// Missing or rejected credentials (401), or a denied query (403).
    #[error("provider denied the query: {message}")]
    Auth { message: String },
    /// A list endpoint answered 404. Providers answer a private repository
    /// the caller may not see this way, so it is a permission failure as
    /// much as an absence.
    #[error("repository not found or not permitted: {message}")]
    NotFoundOrNotPermitted { message: String },
    #[error("provider rate limit reached: {message}")]
    RateLimited { message: String },
    /// The remote is not a provider this module can query, or policy
    /// forbids contacting its host.
    #[error("unsupported provider or host: {message}")]
    Unsupported { message: String },
    #[error("provider lookup failed: {message}")]
    Other { message: String },
}

/// Finds the PR opened from `branch` in `source_repo` against the repository
/// at `remote_url`.
///
/// `source_repo` is the `owner/repo` (GitLab: full project path) of the
/// repository that holds the branch; it differs from `remote_url`'s repository
/// when the branch lives in a fork. A PR from a same-named branch in another
/// repository never matches.
///
/// Only hosts `remote_url` identifies without a network probe are supported:
/// github.com, gitlab.com, bitbucket.org, and Gitea or Forgejo hosts named as
/// such (`gitea.*`, `forgejo.*`, codeberg.org). A self-hosted server that needs
/// consented discovery is [`PrUnavailable::Unsupported`] here; build its
/// client after discovery and call [`pull_request_for_branch_with`].
///
/// Must not be called from inside a Tokio runtime (see the module docs).
///
/// ## Returns
///
/// An open PR outranks every merged one. Among several of the same state the
/// most recent wins (merged time, else last update, else creation, then
/// number). No local tip is consulted, so a caller matching a local commit
/// must compare [`PrEvidence::source_head_sha`] itself.
///
/// ## Errors
///
/// Every failure to get an answer is a [`PrUnavailable`], never `Ok(None)`.
pub fn pull_request_for_branch(
    remote_url: &str,
    source_repo: &str,
    branch: &str,
    deadline: Duration,
) -> Result<Option<PrEvidence>, PrUnavailable> {
    let client = client_for_url(remote_url)?;
    pull_request_for_branch_with(&client, source_repo, branch, deadline)
}

/// [`pull_request_for_branch`] through an already-built client, whose API
/// base, fetch policy, and credential scope are used as they are.
///
/// Must not be called from inside a Tokio runtime (see the module docs).
///
/// ## Errors
///
/// As for [`pull_request_for_branch`].
pub fn pull_request_for_branch_with(
    client: &FocusedProviderClient,
    source_repo: &str,
    branch: &str,
    deadline: Duration,
) -> Result<Option<PrEvidence>, PrUnavailable> {
    let candidates = run_with_deadline(client, deadline, |client| async move {
        client.branch_pull_requests(source_repo, branch).await
    })?;
    Ok(select_evidence(candidates, source_repo))
}

/// Lists every open PR against the repository at `remote_url`, in the
/// provider's order.
///
/// Pages are followed up to the focused client's page bound. A match against
/// a local branch must compare both [`PrSummary::source_repo`] and
/// [`PrSummary::source_branch`], since a fork can open a PR from a branch with
/// the same name.
///
/// Supported hosts are those of [`pull_request_for_branch`]; for a
/// self-hosted server that needs consented discovery, call
/// [`open_pull_requests_with`].
///
/// Must not be called from inside a Tokio runtime (see the module docs).
///
/// ## Errors
///
/// Every failure to get an answer is a [`PrUnavailable`], never an empty
/// list: in particular a 401 or 403 is [`PrUnavailable::Auth`] and a list 404
/// is [`PrUnavailable::NotFoundOrNotPermitted`]. More open PRs than the page
/// bound allows is [`PrUnavailable::Other`].
pub fn open_pull_requests(
    remote_url: &str,
    deadline: Duration,
) -> Result<Vec<PrSummary>, PrUnavailable> {
    let client = client_for_url(remote_url)?;
    open_pull_requests_with(&client, deadline)
}

/// [`open_pull_requests`] through an already-built client, whose API base,
/// fetch policy, and credential scope are used as they are.
///
/// Must not be called from inside a Tokio runtime (see the module docs).
///
/// ## Errors
///
/// As for [`open_pull_requests`].
pub fn open_pull_requests_with(
    client: &FocusedProviderClient,
    deadline: Duration,
) -> Result<Vec<PrSummary>, PrUnavailable> {
    let records = run_with_deadline(client, deadline, |client| async move {
        client.open_pull_requests().await
    })?;
    Ok(records
        .into_iter()
        .map(|record| PrSummary {
            number: record.number,
            html_url: Some(record.html_url).filter(|url| !url.is_empty()),
            source_repo: record.source_repo,
            source_branch: record.source_branch,
            target_branch: record.target_branch,
        })
        .collect())
}

/// Runs one focused-client operation on a fresh current-thread runtime, with
/// every request bounded by `deadline` from now, and classifies its failure.
///
/// Shared by every blocking entry point.
fn run_with_deadline<T, F, Fut>(
    client: &FocusedProviderClient,
    deadline: Duration,
    operation: F,
) -> Result<T, PrUnavailable>
where
    F: FnOnce(FocusedProviderClient) -> Fut,
    Fut: Future<Output = Result<T, SniffError>>,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(PrUnavailable::Other {
            message: "blocking provider lookup called from inside a Tokio runtime".to_string(),
        });
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| PrUnavailable::Other {
            message: format!("could not start the lookup runtime: {error}"),
        })?;
    let expires = Instant::now()
        .checked_add(deadline)
        .ok_or_else(|| PrUnavailable::Other {
            message: format!("deadline {deadline:?} is out of range"),
        })?;
    runtime
        .block_on(operation(client.with_deadline(expires)))
        .map_err(|error| classify(error, Some((expires, deadline))))
}

fn client_for_url(remote_url: &str) -> Result<FocusedProviderClient, PrUnavailable> {
    let unsupported = |message: String| PrUnavailable::Unsupported { message };
    let (endpoint, namespace, repository) = parse_remote_identity(remote_url);
    let Some(host) = endpoint.as_ref().map(|endpoint| endpoint.host.clone()) else {
        return Err(unsupported(format!("no host in remote URL `{remote_url}`")));
    };
    if namespace.is_none() || repository.is_none() {
        return Err(unsupported(format!(
            "no owner/repository path in remote URL `{remote_url}`"
        )));
    }
    let api_flavor: ApiFlavor = GitHostingProvider::from_url(remote_url).into();
    if !matches!(
        api_flavor,
        ApiFlavor::GitHub
            | ApiFlavor::GitLab
            | ApiFlavor::Gitea
            | ApiFlavor::Forgejo
            | ApiFlavor::Bitbucket
    ) {
        return Err(unsupported(format!(
            "`{host}` is not a recognized pull-request provider host"
        )));
    }
    let remote = ResolvedRemote {
        name: remote_url.to_string(),
        fetch_url: remote_url.to_string(),
        push_url: remote_url.to_string(),
        host: Some(host.clone()),
        namespace,
        repository,
        api_flavor,
        endpoint,
    };
    // The caller's configured remote is the consent to contact its own host.
    FocusedProviderClient::new(remote, FetchPolicy::deny_all().allow_host(&host))
        .map_err(|error| classify(error, None))
}

/// `timing` is the lookup's expiry and its original duration; a transport
/// failure at or after the expiry is the deadline's doing.
fn classify(error: SniffError, timing: Option<(Instant, Duration)>) -> PrUnavailable {
    let message = error.to_string();
    match error {
        SniffError::MissingCredentials { .. }
        | SniffError::InvalidCredentials { .. }
        | SniffError::RemoteForbidden { .. }
        | SniffError::RemoteApi {
            status: 401 | 403, ..
        } => PrUnavailable::Auth { message },
        SniffError::RemoteApi { status: 404, .. } => {
            PrUnavailable::NotFoundOrNotPermitted { message }
        }
        SniffError::RateLimited { .. } | SniffError::RemoteApi { status: 429, .. } => {
            PrUnavailable::RateLimited { message }
        }
        SniffError::RemoteUnreachable { .. } => match timing {
            Some((expires, deadline)) if Instant::now() >= expires => {
                PrUnavailable::Timeout { deadline }
            }
            _ => PrUnavailable::Network { message },
        },
        SniffError::RemotePolicyDenied { .. }
        | SniffError::UnsupportedProvider { .. }
        | SniffError::UnsupportedRemoteCapability { .. }
        | SniffError::UnsupportedServerVersion { .. } => PrUnavailable::Unsupported { message },
        _ => PrUnavailable::Other { message },
    }
}

fn select_evidence(
    candidates: Vec<(PullRequestInfo, PrState)>,
    source_repo: &str,
) -> Option<PrEvidence> {
    let recency = |record: &PullRequestInfo| {
        record
            .merged_at
            .clone()
            .or_else(|| record.updated_at.clone())
            .unwrap_or_else(|| record.created_at.clone())
    };
    let (record, state) =
        candidates
            .into_iter()
            .max_by(|(left, left_state), (right, right_state)| {
                (*left_state == PrState::Open)
                    .cmp(&(*right_state == PrState::Open))
                    .then_with(|| {
                        optional_timestamp_order(Some(&recency(left)), Some(&recency(right)))
                    })
                    .then_with(|| left.number.cmp(&right.number))
            })?;
    Some(PrEvidence {
        number: record.number,
        html_url: Some(record.html_url).filter(|url| !url.is_empty()),
        state,
        source_repo: record
            .source_repo
            .unwrap_or_else(|| source_repo.to_string()),
        source_head_sha: record.source_head_sha,
        target_branch: record.target_branch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(number: u64, merged_at: Option<&str>, updated_at: &str) -> PullRequestInfo {
        PullRequestInfo {
            number,
            title: String::new(),
            state: String::new(),
            author: String::new(),
            draft: false,
            source_branch: Some("feature".to_string()),
            target_branch: Some("main".to_string()),
            labels: Vec::new(),
            body: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: Some(updated_at.to_string()),
            merged_at: merged_at.map(str::to_string),
            html_url: String::new(),
            source_repo: Some("acme/project".to_string()),
            source_repo_is_target: Some(true),
            source_head_sha: None,
        }
    }

    #[test]
    fn open_outranks_a_more_recent_merged_pr() {
        let chosen = select_evidence(
            vec![
                (
                    record(1, Some("2024-06-01T00:00:00Z"), "2024-06-01T00:00:00Z"),
                    PrState::Merged,
                ),
                (record(2, None, "2024-02-01T00:00:00Z"), PrState::Open),
            ],
            "acme/project",
        )
        .unwrap();

        assert_eq!((chosen.number, chosen.state), (2, PrState::Open));
    }

    #[test]
    fn most_recently_merged_pr_wins_regardless_of_number() {
        let chosen = select_evidence(
            vec![
                (
                    record(9, Some("2024-02-01T00:00:00Z"), "2024-07-01T00:00:00Z"),
                    PrState::Merged,
                ),
                (
                    record(3, Some("2024-05-01T00:00:00Z"), "2024-05-01T00:00:00Z"),
                    PrState::Merged,
                ),
            ],
            "acme/project",
        )
        .unwrap();

        assert_eq!(chosen.number, 3);
    }

    #[test]
    fn a_denial_is_never_classified_as_an_answer() {
        let forbidden = SniffError::RemoteForbidden {
            provider: "GitHub".to_string(),
            message: "denied".to_string(),
        };
        let list_404 = SniffError::RemoteApi {
            provider: "GitHub".to_string(),
            status: 404,
            message: "not found or not permitted".to_string(),
        };

        assert!(matches!(
            classify(forbidden, None),
            PrUnavailable::Auth { .. }
        ));
        assert!(matches!(
            classify(list_404, None),
            PrUnavailable::NotFoundOrNotPermitted { .. }
        ));
    }

    #[test]
    fn unrecognized_hosts_are_unsupported_before_any_request() {
        for url in [
            "https://git.internal.example/acme/project.git",
            "https://dev.azure.com/acme/project/_git/project",
            "not a url",
        ] {
            assert!(
                matches!(client_for_url(url), Err(PrUnavailable::Unsupported { .. })),
                "{url}"
            );
        }
    }

    #[test]
    fn a_call_from_inside_a_runtime_is_refused_without_io() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            pull_request_for_branch(
                "https://github.com/acme/project.git",
                "acme/project",
                "feature",
                Duration::from_secs(1),
            )
        });

        assert!(matches!(result, Err(PrUnavailable::Other { .. })));
    }
}
