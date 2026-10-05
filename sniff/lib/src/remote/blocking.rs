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
use std::sync::Arc;
use std::time::{Duration, Instant};

use biscuit_file::FetchPolicy;

use crate::SniffError;
use crate::filesystem::git::commit_links::parse_remote_identity;
use crate::filesystem::git::{ApiFlavor, GitHostingProvider, ResolvedRemote};

use super::focused::{INSUFFICIENT_CREDENTIALS_MESSAGE, SentLog, SentWith};
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

/// Every open PR against a repository, and the credentials the lookup's
/// requests were sent with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPullRequests {
    /// In the provider's order.
    pub pull_requests: Vec<PrSummary>,
    /// Covers every page and auxiliary request of the lookup.
    pub credentials: RequestCredentials,
}

/// The commit a provider reports at the tip of one branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchHead {
    /// Full object ID: 40 (SHA-1) or 64 (SHA-256) lowercase hex digits.
    pub sha: String,
    /// The credentials the request was sent with.
    pub credentials: RequestCredentials,
}

/// Which credentials a successful lookup's requests were sent with.
///
/// Built from the selection the client made for each request as it sent it,
/// so it describes what was sent, including a host-bound `SNIFF_*_TOKEN`
/// override, rather than what a later look at the environment would find.
/// It holds variable *names*, never token values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RequestCredentials {
    /// Every request was sent without a token.
    Anonymous,
    /// At least one request carried a token. `variables` names each variable
    /// a request read its token from, in first-use order, without repeats;
    /// it is never empty.
    Keyed { variables: Vec<String> },
    /// No request is known to have been sent, so nothing can be claimed.
    Unknown,
}

impl RequestCredentials {
    /// Folds per-request selections: anonymous only when every request was.
    fn from_sent(sent: &[SentWith]) -> Self {
        let mut variables: Vec<String> = Vec::new();
        for request in sent {
            if let SentWith::Key(name) = request
                && !variables.contains(name)
            {
                variables.push(name.clone());
            }
        }
        match (sent.is_empty(), variables.is_empty()) {
            (true, _) => Self::Unknown,
            (false, true) => Self::Anonymous,
            (false, false) => Self::Keyed { variables },
        }
    }
}

/// The environment variables a provider's blocking lookups read a token from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialEnv {
    /// Provider display name, such as `GitHub`.
    pub provider: String,
    /// Variable names in the order lookups consult them; the first one set
    /// to a nonempty value is sent.
    pub variables: Vec<String>,
}

/// Why no answer, positive or negative, could be obtained.
///
/// None of these is ever reported as `Ok(None)`: only a provider that answered
/// the query and listed no matching PR produces that.
///
/// A `key` is always the *name* of the environment variable whose token was
/// sent, never its value, and no variant's `Display` includes a token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PrUnavailable {
    /// The deadline passed before the lookup finished.
    #[error("provider lookup did not finish within {deadline:?}")]
    Timeout { deadline: Duration },
    /// The provider could not be reached or the connection failed.
    #[error("provider unreachable: {message}")]
    Network { message: String },
    /// The provider answered 401 to a request sent without a token.
    ///
    /// `key` is `None` from every lookup here, since no variable was set; it
    /// has the other credential variants' field so a caller can read `key`
    /// uniformly.
    #[error("provider requires credentials, and no token variable is set")]
    CredentialsRequired { key: Option<String> },
    /// The provider answered 401 to the token in `key`: it may be invalid,
    /// expired, or revoked.
    #[error("provider rejected the token in {key}")]
    CredentialsRejected { key: String },
    /// The provider answered 403 to the token in `key`, and its response
    /// named a missing permission or scope.
    #[error("the token in {key} lacks permission for this query")]
    CredentialsInsufficient { key: String },
    /// A 404, or a 403 whose response establishes neither rate limiting nor
    /// a credentials denial. Providers answer a private repository the caller
    /// may not see this way, so it is a permission failure as much as an
    /// absence, whether or not a token was sent. `key` names the variable
    /// whose token the failing request sent, and is `None` when it was sent
    /// without one.
    #[error("repository not found or not permitted: {message}")]
    NotFoundOrNotPermitted {
        message: String,
        key: Option<String>,
    },
    /// A 429, or a 403 carrying a spent `x-ratelimit-remaining` or a
    /// rate-limit body. `authenticated` is whether the token in `key` was
    /// sent.
    #[error("provider rate limit reached ({})", key.as_deref().map_or_else(|| "anonymous request".to_string(), |key| format!("token in {key}")))]
    RateLimited {
        authenticated: bool,
        key: Option<String>,
    },
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
    let (candidates, _) = run_with_deadline(client, deadline, |client| async move {
        client.branch_pull_requests(source_repo, branch).await
    })?;
    Ok(select_evidence(candidates, source_repo))
}

/// Lists every open PR against the repository at `remote_url`, in the
/// provider's order, with the credentials its requests were sent with
/// ([`RequestCredentials`]: anonymous only when every page was).
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
/// list: in particular a 401 is [`PrUnavailable::CredentialsRequired`] or
/// [`PrUnavailable::CredentialsRejected`], and a list 404 is
/// [`PrUnavailable::NotFoundOrNotPermitted`]. More open PRs than the page
/// bound allows is [`PrUnavailable::Other`].
pub fn open_pull_requests(
    remote_url: &str,
    deadline: Duration,
) -> Result<OpenPullRequests, PrUnavailable> {
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
) -> Result<OpenPullRequests, PrUnavailable> {
    let (records, credentials) = run_with_deadline(client, deadline, |client| async move {
        client.open_pull_requests().await
    })?;
    let pull_requests = records
        .into_iter()
        .map(|record| PrSummary {
            number: record.number,
            html_url: Some(record.html_url).filter(|url| !url.is_empty()),
            source_repo: record.source_repo,
            source_branch: record.source_branch,
            target_branch: record.target_branch,
        })
        .collect();
    Ok(OpenPullRequests {
        pull_requests,
        credentials,
    })
}

/// The head commit of `branch` in the repository at `remote_url`, with the
/// credentials the request was sent with.
///
/// `remote_url` should be the remote's *fetch* URL, since a separate push URL
/// may name another repository. `branch` is sent as one percent-encoded path
/// segment, so no branch name can change the repository path or the query.
///
/// A request is authenticated with the first set, nonempty variable of
/// [`credential_env`], and anonymous when none is set. A rejected or
/// insufficient token is reported, not retried anonymously, as for
/// [`open_pull_requests`].
///
/// Supported hosts are those of [`pull_request_for_branch`]; for a
/// self-hosted server that needs consented discovery, call
/// [`branch_head_with`].
///
/// Must not be called from inside a Tokio runtime (see the module docs).
///
/// ## Errors
///
/// A 404 is [`PrUnavailable::NotFoundOrNotPermitted`], never proof that the
/// branch is absent: providers answer a private repository the same way. A
/// head that is not 40 or 64 lowercase hex digits is [`PrUnavailable::Other`].
pub fn branch_head(
    remote_url: &str,
    branch: &str,
    deadline: Duration,
) -> Result<BranchHead, PrUnavailable> {
    let client = client_for_url(remote_url)?;
    branch_head_with(&client, branch, deadline)
}

/// [`branch_head`] through an already-built client, whose API base, fetch
/// policy, and credential scope are used as they are.
///
/// Must not be called from inside a Tokio runtime (see the module docs).
///
/// ## Errors
///
/// As for [`branch_head`].
pub fn branch_head_with(
    client: &FocusedProviderClient,
    branch: &str,
    deadline: Duration,
) -> Result<BranchHead, PrUnavailable> {
    let (sha, credentials) = run_with_deadline(client, deadline, |client| async move {
        client.branch_head(branch).await
    })?;
    let object_id = matches!(sha.len(), 40 | 64)
        && sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !object_id {
        return Err(PrUnavailable::Other {
            message: "provider sent a branch head that is not a full object ID".to_string(),
        });
    }
    Ok(BranchHead { sha, credentials })
}

/// The provider display name and token variables for the repository at
/// `remote_url`, or `None` when the blocking lookups do not support it.
///
/// The variables are the ones [`branch_head`], [`open_pull_requests`], and
/// [`pull_request_for_branch`] read, in the order they read them. No variable
/// is read and no request is made.
pub fn credential_env(remote_url: &str) -> Option<CredentialEnv> {
    let client = client_for_url(remote_url).ok()?;
    Some(CredentialEnv {
        provider: GitHostingProvider::from_url(remote_url)
            .metadata()
            .display_name
            .to_string(),
        variables: crate::credentials::provider_token_variables(client.remote().api_flavor)
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
    })
}

/// Runs one focused-client operation on a fresh current-thread runtime, with
/// every request bounded by `deadline` from now, and classifies its failure.
///
/// A success comes with the credentials of every request the operation sent.
/// A failure's `key` is the variable the last request sent, never a fresh
/// look at the environment.
fn run_with_deadline<T, F, Fut>(
    client: &FocusedProviderClient,
    deadline: Duration,
    operation: F,
) -> Result<(T, RequestCredentials), PrUnavailable>
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
    let log = SentLog::default();
    let result = runtime.block_on(operation(
        client.with_deadline(expires).with_sent_log(Arc::clone(&log)),
    ));
    // The log is pushed to only between two awaits, so it cannot be poisoned
    // in practice; if it were, nothing would be known about what was sent.
    let sent = match log.lock() {
        Ok(sent) => sent.clone(),
        Err(_) => Vec::new(),
    };
    match result {
        Ok(value) => Ok((value, RequestCredentials::from_sent(&sent))),
        Err(error) => {
            let key = match sent.last() {
                Some(SentWith::Key(name)) => Some(name.clone()),
                Some(SentWith::Anonymous) | None => None,
            };
            Err(classify(error, Some((expires, deadline)), key))
        }
    }
}

fn client_for_url(remote_url: &str) -> Result<FocusedProviderClient, PrUnavailable> {
    let unsupported = |message: String| PrUnavailable::Unsupported { message };
    let (endpoint, namespace, repository) = parse_remote_identity(remote_url);
    // The URL is never echoed: its userinfo may hold a token.
    let Some(host) = endpoint.as_ref().map(|endpoint| endpoint.host.clone()) else {
        return Err(unsupported("the remote URL names no host".to_string()));
    };
    if namespace.is_none() || repository.is_none() {
        return Err(unsupported(format!(
            "the remote URL on `{host}` names no owner/repository path"
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
        .map_err(|error| classify(error, None, None))
}

/// `timing` is the lookup's expiry and its original duration; a transport
/// failure at or after the expiry is the deadline's doing. `key` names the
/// variable whose token the lookup sent.
fn classify(
    error: SniffError,
    timing: Option<(Instant, Duration)>,
    key: Option<String>,
) -> PrUnavailable {
    let message = error.to_string();
    match error {
        SniffError::InvalidCredentials { .. } | SniffError::RemoteApi { status: 401, .. } => {
            match key {
                Some(key) => PrUnavailable::CredentialsRejected { key },
                None => PrUnavailable::CredentialsRequired { key: None },
            }
        }
        SniffError::MissingCredentials { .. } => PrUnavailable::CredentialsRequired { key: None },
        SniffError::RemoteForbidden {
            message: ref denial,
            ..
        } if denial == INSUFFICIENT_CREDENTIALS_MESSAGE => match key {
            Some(key) => PrUnavailable::CredentialsInsufficient { key },
            None => PrUnavailable::NotFoundOrNotPermitted { message, key: None },
        },
        SniffError::RemoteForbidden { .. }
        | SniffError::RemoteApi {
            status: 403 | 404, ..
        } => PrUnavailable::NotFoundOrNotPermitted { message, key },
        SniffError::RateLimited { .. } | SniffError::RemoteApi { status: 429, .. } => {
            PrUnavailable::RateLimited {
                authenticated: key.is_some(),
                key,
            }
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
            message: INSUFFICIENT_CREDENTIALS_MESSAGE.to_string(),
        };
        let list_404 = SniffError::RemoteApi {
            provider: "GitHub".to_string(),
            status: 404,
            message: "not found or not permitted".to_string(),
        };

        assert_eq!(
            classify(forbidden, None, Some("GH_TOKEN".to_string())),
            PrUnavailable::CredentialsInsufficient {
                key: "GH_TOKEN".to_string()
            }
        );
        assert!(matches!(
            classify(list_404, None, Some("GH_TOKEN".to_string())),
            PrUnavailable::NotFoundOrNotPermitted { key: Some(key), .. } if key == "GH_TOKEN"
        ));
    }

    #[test]
    fn request_credentials_are_anonymous_only_when_every_request_was() {
        let key = |name: &str| SentWith::Key(name.to_string());
        let keyed = |names: &[&str]| RequestCredentials::Keyed {
            variables: names.iter().map(|name| (*name).to_string()).collect(),
        };
        let cases = [
            (vec![], RequestCredentials::Unknown),
            (vec![SentWith::Anonymous], RequestCredentials::Anonymous),
            (
                vec![SentWith::Anonymous, SentWith::Anonymous],
                RequestCredentials::Anonymous,
            ),
            (
                vec![SentWith::Anonymous, key("GH_TOKEN")],
                keyed(&["GH_TOKEN"]),
            ),
            (
                vec![key("GH_TOKEN"), SentWith::Anonymous],
                keyed(&["GH_TOKEN"]),
            ),
            (
                vec![key("GH_TOKEN"), key("GITHUB_TOKEN"), key("GH_TOKEN")],
                keyed(&["GH_TOKEN", "GITHUB_TOKEN"]),
            ),
        ];
        for (sent, expected) in cases {
            assert_eq!(RequestCredentials::from_sent(&sent), expected, "{sent:?}");
        }
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
