//! Remote URL identity, browser links, and bounded commit containment.
//!
//! This module is the single authority for turning a configured remote URL
//! into a repository identity and browser URLs, and for deciding whether the
//! locally recorded remote-tracking refs contain a commit. It never fetches or
//! contacts a provider: containment reflects the last fetch, not live remote
//! state.
//!
//! Containment follows `sniff/features/2026-09-15-recent-commits/spec.md`
//! Decision 9: remote-tracking tips are walked in preferred-remote order under
//! one total commit-visit budget, and a commit still undetermined when the
//! budget runs out is reported as unknown rather than absent.

use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;

use super::GitHostingProvider;
use super::remote_refresh::{AncestryWalkEnd, RefSnapshot, walk_ancestry};
use super::remote_resolver::{RemoteEndpoint, configured_url, preferred_remote_order};
use crate::Result;

/// Total ancestry visits one linking pass may spend across every
/// remote-tracking tip.
///
/// About 10× the largest normal-state cost measured in
/// `sniff/features/2026-09-15-recent-commits/spike-linking-cost.md` (521,005
/// visits); see `contract.md` § Containment Visit Budget.
pub(crate) const COMMIT_VISIT_BUDGET: usize = 5_000_000;

/// Browser-facing identity of a remote repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryLink {
    /// The repository path on its host, such as `owner/repo` or
    /// `group/subgroup/repo`, without a `.git` suffix.
    pub owner_repo: String,
    /// The repository's web page, when the provider has a known browser host.
    pub browser_url: Option<String>,
}

/// Remote containment and browser link for one commit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitLink {
    /// `Some(true)` when a local remote-tracking ref contains the commit,
    /// `Some(false)` when every remote-tracking tip was walked without finding
    /// it, and `None` when that could not be determined (the visit budget ran
    /// out, an ancestor was unreadable, or the commit id was invalid).
    pub remote: Option<bool>,
    /// The commit's page on the preferred containing remote. Present only when
    /// `remote` is `Some(true)` and that remote's provider has a browser URL.
    pub commit_url: Option<String>,
}

/// Parse a remote URL into its transport endpoint, namespace, and repository
/// name.
///
/// Accepts URL forms (`https://`, `ssh://`, `git://`, `file://`) and SCP-style
/// `user@host:path`. The repository is the last path segment without `.git`;
/// the namespace is every earlier segment joined with `/`.
pub(crate) fn parse_remote_identity(
    remote: &str,
) -> (Option<RemoteEndpoint>, Option<String>, Option<String>) {
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
    let mut segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let repository = segments
        .pop()
        .map(|segment| segment.trim_end_matches(".git").to_string());
    let namespace = (!segments.is_empty()).then(|| segments.join("/"));
    (endpoint, namespace, repository)
}

/// The canonical HTTPS URL of the repository a configured remote names.
///
/// Providers whose Git transport form differs from the form they serve over
/// HTTPS are normalized here: a SourceHut remote keeps the `git.sr.ht` service
/// host rather than the site root, and the Azure DevOps SSH form
/// `v3/{organization}/{project}/{repository}` becomes
/// `{organization}/{project}/_git/{repository}` on `dev.azure.com`, its
/// segments carrying exactly one percent-encoding layer whether the remote is
/// URL-form or SCP-style. This module's browser links and the HTTPS Git endpoint
/// `remote_observation::branch_exists_on_remote_at` advertises against both
/// read this one mapping, so the two cannot normalize a provider differently.
///
/// ## Returns
///
/// `None` when the URL names no repository, when the provider publishes no
/// canonical HTTPS location (self-managed servers, unknown hosts), or when the
/// provider's required path shape is absent — an Azure DevOps SSH path that is
/// not `{organization}/{project}/{repository}`, or an AWS CodeCommit path
/// outside `v1/repos/`.
pub(crate) fn canonical_repository_url(
    provider: GitHostingProvider,
    remote_url: &str,
) -> Option<String> {
    let (endpoint, namespace, repository) = parse_remote_identity(remote_url);
    let repository = repository.filter(|name| !name.is_empty())?;
    let endpoint = endpoint?;
    let host = endpoint.host.to_ascii_lowercase();
    let over_http = matches!(endpoint.scheme.as_str(), "http" | "https");
    let path = match namespace {
        Some(namespace) => format!("{namespace}/{repository}"),
        None => repository,
    };
    match provider {
        GitHostingProvider::AzureDevOps => {
            let ssh_path = match path.strip_prefix("v3/") {
                Some(ssh_path) => ssh_path,
                // An HTTP(S) remote already spells the browsed path, including
                // the legacy form that carries the organization in the host.
                None if over_http => return Some(format!("https://{host}/{path}")),
                // The SSH form always names the collection as `v3/…`; anything
                // else locates no repository.
                None => return None,
            };
            let segments = ssh_path
                .split('/')
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>();
            let [organization, project, repository] = segments.as_slice() else {
                return None;
            };
            // A URL-form remote arrives with its path still percent-escaped
            // while an SCP-style one carries names raw, so decode before the
            // one encode. Azure DevOps forbids `%` in organization, project,
            // and repository names, so decoding cannot corrupt a raw segment;
            // an escape that is not valid UTF-8 is treated as a raw name.
            let encode_once = |segment: &str| match urlencoding::decode(segment) {
                Ok(decoded) => urlencoding::encode(&decoded).into_owned(),
                Err(_) => urlencoding::encode(segment).into_owned(),
            };
            Some(format!(
                "https://dev.azure.com/{}/{}/_git/{}",
                encode_once(organization),
                encode_once(project),
                encode_once(repository)
            ))
        }
        // CodeCommit's console page is region- and account-scoped and is not
        // derivable from the remote; this is its HTTPS Git endpoint, which
        // `browser_base_url` correctly refuses to present as a web page.
        GitHostingProvider::AwsCodeCommit => path
            .starts_with("v1/repos/")
            .then(|| format!("https://{host}/{path}")),
        GitHostingProvider::SourceHut => {
            // A `*.sr.ht` remote already names the Git service host; the bare
            // site root does not serve repositories.
            if host.ends_with(".sr.ht") {
                Some(format!("https://{host}/{path}"))
            } else {
                Some(format!("{}/{path}", provider.browser_base_url()?))
            }
        }
        _ => Some(format!("{}/{path}", provider.browser_base_url()?)),
    }
}

/// The repository identity and web page for a configured remote URL.
///
/// ## Examples
///
/// ```
/// use sniff::filesystem::git::repository_link;
///
/// let link = repository_link("git@github.com:rust-lang/cargo.git").unwrap();
/// assert_eq!(link.owner_repo, "rust-lang/cargo");
/// assert_eq!(link.browser_url.as_deref(), Some("https://github.com/rust-lang/cargo"));
///
/// // A self-hosted server has an identity but no known browser host.
/// let link = repository_link("https://git.example.com/team/app").unwrap();
/// assert_eq!(link.browser_url, None);
/// ```
///
/// ## Returns
///
/// `None` when the URL has no repository path segment.
pub fn repository_link(remote_url: &str) -> Option<RepositoryLink> {
    let (_, namespace, repository) = parse_remote_identity(remote_url);
    let repository = repository.filter(|name| !name.is_empty())?;
    let owner_repo = match namespace {
        Some(namespace) => format!("{namespace}/{repository}"),
        None => repository,
    };
    let provider = GitHostingProvider::from_url(remote_url);
    // A provider without a browser base has no web page even when it has a
    // canonical HTTPS Git endpoint, as AWS CodeCommit does.
    let browser_url = provider
        .browser_base_url()
        .and_then(|_| canonical_repository_url(provider, remote_url));
    Some(RepositoryLink {
        owner_repo,
        browser_url,
    })
}

/// The browser URL for `sha` on the repository behind `remote_url`, or `None`
/// when the provider has no known browser host.
///
/// This builds the URL only; it does not check that the remote contains the
/// commit. See [`commit_links_at`](super::commit_links_at) for that.
pub fn commit_url(remote_url: &str, sha: &str) -> Option<String> {
    let provider = GitHostingProvider::from_url(remote_url);
    let base = repository_link(remote_url)?.browser_url?;
    Some(format!("{base}/{}/{sha}", provider.commit_path_segment()))
}

/// Link `targets` with [`COMMIT_VISIT_BUDGET`]. Results are in `targets` order.
///
/// ## Errors
///
/// A ref store that cannot be read surfaces as [`crate::SniffError::Git`].
pub(crate) fn link_commits(
    repo: &gix::Repository,
    targets: &[gix::ObjectId],
) -> Result<Vec<CommitLink>> {
    link_commits_with_budget(repo, targets, COMMIT_VISIT_BUDGET)
}

/// [`link_commits`] with an explicit total visit budget.
pub(crate) fn link_commits_with_budget(
    repo: &gix::Repository,
    targets: &[gix::ObjectId],
    visit_budget: usize,
) -> Result<Vec<CommitLink>> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let refs = RefSnapshot::observe(repo, false, true, false)?;
    Ok(link_commits_from_snapshot(repo, &refs, targets, visit_budget))
}

/// [`link_commits_with_budget`] over remote-tracking refs the caller already
/// observed, so a request that snapshots refs for other facts reads them once.
///
/// Tips are walked remote by remote in preferred-remote order, and within a
/// remote its default branch first, then its other branches alphabetically.
/// The first containing remote in that order wins, so a commit is determined
/// as soon as any walk reaches it, and linking stops once every target is
/// determined.
///
/// `refs` must include remote branches; a snapshot without them links every
/// target `remote: Some(false)`.
pub(crate) fn link_commits_from_snapshot(
    repo: &gix::Repository,
    refs: &RefSnapshot,
    targets: &[gix::ObjectId],
    visit_budget: usize,
) -> Vec<CommitLink> {
    if targets.is_empty() {
        return Vec::new();
    }
    let remotes = preferred_remote_order(refs.remote_branch_tips().map(|(remote, _, _)| remote));
    let mut tips: Vec<(usize, gix::ObjectId)> = Vec::new();
    for (remote_index, remote) in remotes.iter().enumerate() {
        let default = refs.remote_default_branch(remote);
        let mut branches: Vec<(&str, gix::ObjectId)> = refs
            .remote_branch_tips()
            .filter(|(name, _, _)| name == remote)
            .map(|(_, branch, tip)| (branch, tip))
            .collect();
        // Already alphabetical; the stable sort only lifts the default branch.
        branches.sort_by_key(|(branch, _)| Some(*branch) != default);
        tips.extend(branches.into_iter().map(|(_, tip)| (remote_index, tip)));
    }

    let mut undetermined: HashSet<gix::ObjectId> = targets.iter().copied().collect();
    let mut winners: HashMap<gix::ObjectId, usize> = HashMap::new();
    let mut remaining = visit_budget;
    let mut incomplete = false;

    for (remote_index, tip) in tips {
        if undetermined.is_empty() {
            break;
        }
        let end = walk_ancestry(repo, tip, &mut remaining, |id| {
            if undetermined.remove(&id) {
                winners.insert(id, remote_index);
            }
            if undetermined.is_empty() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        match end {
            AncestryWalkEnd::Stopped | AncestryWalkEnd::Completed => {}
            AncestryWalkEnd::Failed => incomplete = true,
            AncestryWalkEnd::OverBudget => {
                incomplete = true;
                break;
            }
        }
    }

    let mut urls: HashMap<usize, Option<String>> = HashMap::new();
    targets
        .iter()
        .map(|target| match winners.get(target) {
            Some(remote_index) => {
                let base = urls.entry(*remote_index).or_insert_with(|| {
                    configured_url(repo, remotes[*remote_index])
                });
                CommitLink {
                    remote: Some(true),
                    commit_url: base
                        .as_deref()
                        .and_then(|url| commit_url(url, &target.to_string())),
                }
            }
            // Unfound after walks that all finished means absent; any
            // unfinished walk could still have reached it.
            None => CommitLink {
                remote: (!incomplete).then_some(false),
                commit_url: None,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::{counters, testing};
    use git2::{Repository, Signature, Time};
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (dir, repo)
    }

    /// Commit an empty tree on top of `parents` without moving any ref.
    fn commit(repo: &Repository, parents: &[git2::Oid], message: &str, seconds: i64) -> git2::Oid {
        let signature = Signature::new("Test", "test@example.com", &Time::new(seconds, 0)).unwrap();
        let tree_id = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parents: Vec<git2::Commit<'_>> =
            parents.iter().map(|id| repo.find_commit(*id).unwrap()).collect();
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        repo.commit(None, &signature, &signature, message, &tree, &parent_refs)
            .unwrap()
    }

    fn chain(repo: &Repository, base: Option<git2::Oid>, len: usize, label: &str) -> Vec<git2::Oid> {
        let mut ids = Vec::new();
        let mut parent = base;
        for i in 0..len {
            let parents: Vec<git2::Oid> = parent.into_iter().collect();
            let id = commit(repo, &parents, &format!("{label} {i}"), 1_700_000_000 + i as i64);
            ids.push(id);
            parent = Some(id);
        }
        ids
    }

    fn add_remote(repo: &Repository, remote: &str, url: &str) {
        repo.remote(remote, url).unwrap();
    }

    fn set_tip(repo: &Repository, remote: &str, branch: &str, target: git2::Oid) {
        repo.reference(
            &format!("refs/remotes/{remote}/{branch}"),
            target,
            true,
            "test remote tip",
        )
        .unwrap();
    }

    fn gix_repo(dir: &TempDir) -> gix::Repository {
        gix::open(dir.path()).unwrap()
    }

    fn oid(id: git2::Oid) -> gix::ObjectId {
        gix::ObjectId::from_hex(id.to_string().as_bytes()).unwrap()
    }

    mod urls {
        use super::*;

        #[test]
        fn repository_link_parses_every_url_form() {
            for (url, owner_repo, browser) in [
                ("https://github.com/o/r.git", "o/r", Some("https://github.com/o/r")),
                ("git@github.com:o/r.git", "o/r", Some("https://github.com/o/r")),
                ("ssh://git@github.com/o/r", "o/r", Some("https://github.com/o/r")),
                (
                    "https://gitlab.com/group/sub/r",
                    "group/sub/r",
                    Some("https://gitlab.com/group/sub/r"),
                ),
                ("https://git.example.com/team/app.git", "team/app", None),
            ] {
                assert_eq!(
                    repository_link(url),
                    Some(RepositoryLink {
                        owner_repo: owner_repo.to_string(),
                        browser_url: browser.map(str::to_string),
                    }),
                    "{url}"
                );
            }
        }

        #[test]
        fn repository_link_is_none_without_a_repository_segment() {
            assert_eq!(repository_link("https://github.com/"), None);
            assert_eq!(repository_link("not a url"), None);
        }

        /// Every recognized provider in every transport form it publishes,
        /// through the repository page and the commit route built on it.
        ///
        /// The SSH-only providers name the repository differently over their
        /// transport than on the web; `remote_observation`'s
        /// `ssh_only_providers_map_to_canonical_https_git_endpoints` pins the
        /// same normalization from the other consumer.
        #[test]
        fn every_provider_transport_form_builds_its_repository_and_commit_url() {
            const AZURE: &str = "https://dev.azure.com/acme/widgets/_git/project";
            const SOURCE_HUT: &str = "https://git.sr.ht/~acme/project";
            for (url, browser, commit) in [
                (
                    "https://github.com/o/r.git",
                    Some("https://github.com/o/r"),
                    Some("https://github.com/o/r/commit/abc"),
                ),
                (
                    "git@github.com:o/r.git",
                    Some("https://github.com/o/r"),
                    Some("https://github.com/o/r/commit/abc"),
                ),
                (
                    "ssh://git@github.com/o/r.git",
                    Some("https://github.com/o/r"),
                    Some("https://github.com/o/r/commit/abc"),
                ),
                (
                    "git://github.com/o/r.git",
                    Some("https://github.com/o/r"),
                    Some("https://github.com/o/r/commit/abc"),
                ),
                (
                    "https://gitlab.com/group/sub/r.git",
                    Some("https://gitlab.com/group/sub/r"),
                    Some("https://gitlab.com/group/sub/r/-/commit/abc"),
                ),
                (
                    "git@gitlab.com:group/sub/r.git",
                    Some("https://gitlab.com/group/sub/r"),
                    Some("https://gitlab.com/group/sub/r/-/commit/abc"),
                ),
                (
                    "https://bitbucket.org/w/r",
                    Some("https://bitbucket.org/w/r"),
                    Some("https://bitbucket.org/w/r/commits/abc"),
                ),
                (
                    "git@bitbucket.org:w/r.git",
                    Some("https://bitbucket.org/w/r"),
                    Some("https://bitbucket.org/w/r/commits/abc"),
                ),
                (
                    "https://dev.azure.com/acme/widgets/_git/project",
                    Some(AZURE),
                    Some("https://dev.azure.com/acme/widgets/_git/project/commit/abc"),
                ),
                (
                    "git@ssh.dev.azure.com:v3/acme/widgets/project",
                    Some(AZURE),
                    Some("https://dev.azure.com/acme/widgets/_git/project/commit/abc"),
                ),
                (
                    "ssh://acme@vs-ssh.visualstudio.com:22/v3/acme/widgets/project",
                    Some(AZURE),
                    Some("https://dev.azure.com/acme/widgets/_git/project/commit/abc"),
                ),
                // Azure permits spaces and non-ASCII in project and repository
                // names; a URL-form remote escapes them, an SCP-style remote
                // carries them raw, and the page carries one encoding layer.
                (
                    "ssh://git@ssh.dev.azure.com/v3/acme/My%20Project/My%20Repo",
                    Some("https://dev.azure.com/acme/My%20Project/_git/My%20Repo"),
                    Some("https://dev.azure.com/acme/My%20Project/_git/My%20Repo/commit/abc"),
                ),
                (
                    "git@ssh.dev.azure.com:v3/acme/My Project/My Repo",
                    Some("https://dev.azure.com/acme/My%20Project/_git/My%20Repo"),
                    Some("https://dev.azure.com/acme/My%20Project/_git/My%20Repo/commit/abc"),
                ),
                (
                    "ssh://acme@vs-ssh.visualstudio.com:22/v3/acme/My%20Project/My%20Repo",
                    Some("https://dev.azure.com/acme/My%20Project/_git/My%20Repo"),
                    Some("https://dev.azure.com/acme/My%20Project/_git/My%20Repo/commit/abc"),
                ),
                (
                    "ssh://git@ssh.dev.azure.com/v3/acme/Proj%C3%A9/Repo",
                    Some("https://dev.azure.com/acme/Proj%C3%A9/_git/Repo"),
                    Some("https://dev.azure.com/acme/Proj%C3%A9/_git/Repo/commit/abc"),
                ),
                (
                    "git@ssh.dev.azure.com:v3/acme/Projé/Repo",
                    Some("https://dev.azure.com/acme/Proj%C3%A9/_git/Repo"),
                    Some("https://dev.azure.com/acme/Proj%C3%A9/_git/Repo/commit/abc"),
                ),
                // The legacy HTTPS form carries the organization in the host
                // and is already the browsed path.
                (
                    "https://acme.visualstudio.com/widgets/_git/project",
                    Some("https://acme.visualstudio.com/widgets/_git/project"),
                    Some("https://acme.visualstudio.com/widgets/_git/project/commit/abc"),
                ),
                (
                    "git@git.sr.ht:~acme/project",
                    Some(SOURCE_HUT),
                    Some("https://git.sr.ht/~acme/project/commit/abc"),
                ),
                (
                    "https://git.sr.ht/~acme/project",
                    Some(SOURCE_HUT),
                    Some("https://git.sr.ht/~acme/project/commit/abc"),
                ),
                (
                    "git@sr.ht:~acme/project",
                    Some(SOURCE_HUT),
                    Some("https://git.sr.ht/~acme/project/commit/abc"),
                ),
                // CodeCommit has an HTTPS Git endpoint but no derivable page.
                (
                    "ssh://git-codecommit.us-west-2.amazonaws.com/v1/repos/project",
                    None,
                    None,
                ),
                (
                    "https://git-codecommit.us-west-2.amazonaws.com/v1/repos/project",
                    None,
                    None,
                ),
                // Self-managed families have no provider-wide browser host.
                ("https://codeberg.org/o/r.git", None, None),
                ("git@gitea.example.com:o/r.git", None, None),
                ("https://git.example.com/team/app.git", None, None),
                ("not a url", None, None),
            ] {
                assert_eq!(
                    repository_link(url)
                        .and_then(|link| link.browser_url)
                        .as_deref(),
                    browser,
                    "{url}"
                );
                assert_eq!(commit_url(url, "abc").as_deref(), commit, "{url}");
            }
        }

        #[test]
        fn azure_ssh_paths_outside_the_v3_collection_form_have_no_url() {
            for url in [
                "git@ssh.dev.azure.com:v3/acme/project",
                "git@ssh.dev.azure.com:v3/acme/widgets/nested/project",
                "git@ssh.dev.azure.com:acme/widgets/project",
            ] {
                assert_eq!(commit_url(url, "abc"), None, "{url}");
            }
        }
    }

    mod containment {
        use super::*;

        #[test]
        fn no_remote_refs_determines_every_target_absent_without_walking() {
            let (dir, repo) = setup_repo();
            let ids = chain(&repo, None, 2, "local");

            let (links, snapshot) = testing::measure(|| {
                link_commits(&gix_repo(&dir), &[oid(ids[0]), oid(ids[1])]).unwrap()
            });

            assert_eq!(links, vec![CommitLink { remote: Some(false), commit_url: None }; 2]);
            assert_eq!(snapshot.get(counters::GIT_COMMIT_VISITS), 0);
            assert_eq!(snapshot.get(counters::GIT_REF_WALKS), 1);
            assert_eq!(snapshot.get(counters::PROC_SPAWNS), 0);
            assert_eq!(snapshot.get(counters::REMOTE_REQUESTS), 0);
        }

        #[test]
        fn pushed_and_unpushed_commits_are_true_and_false() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "origin", "git@github.com:o/r.git");
            let pushed = chain(&repo, None, 3, "pushed");
            set_tip(&repo, "origin", "main", pushed[2]);
            let unpushed = chain(&repo, Some(pushed[2]), 2, "unpushed");

            let links = link_commits(
                &gix_repo(&dir),
                &[oid(unpushed[1]), oid(pushed[2]), oid(pushed[0])],
            )
            .unwrap();

            assert_eq!(links[0], CommitLink { remote: Some(false), commit_url: None });
            for (link, id) in links[1..].iter().zip([pushed[2], pushed[0]]) {
                assert_eq!(link.remote, Some(true));
                assert_eq!(
                    link.commit_url.as_deref(),
                    Some(format!("https://github.com/o/r/commit/{id}").as_str())
                );
            }
        }

        #[test]
        fn origin_wins_over_other_remotes_that_also_contain_the_commit() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "upstream", "https://github.com/up/r.git");
            add_remote(&repo, "fork", "https://github.com/fork/r.git");
            add_remote(&repo, "origin", "https://github.com/me/r.git");
            let ids = chain(&repo, None, 2, "shared");
            for remote in ["upstream", "fork", "origin"] {
                set_tip(&repo, remote, "main", ids[1]);
            }

            let links = link_commits(&gix_repo(&dir), &[oid(ids[0])]).unwrap();

            assert_eq!(
                links[0].commit_url.as_deref(),
                Some(format!("https://github.com/me/r/commit/{}", ids[0]).as_str())
            );
        }

        #[test]
        fn alphabetical_non_upstream_wins_without_origin_and_upstream_is_last() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "upstream", "https://github.com/up/r.git");
            add_remote(&repo, "zeta", "https://github.com/zeta/r.git");
            add_remote(&repo, "alpha", "https://github.com/alpha/r.git");
            let ids = chain(&repo, None, 1, "shared");
            for remote in ["upstream", "zeta", "alpha"] {
                set_tip(&repo, remote, "main", ids[0]);
            }
            let only_upstream = commit(&repo, &[ids[0]], "upstream only", 1_800_000_000);
            set_tip(&repo, "upstream", "next", only_upstream);

            let links =
                link_commits(&gix_repo(&dir), &[oid(ids[0]), oid(only_upstream)]).unwrap();

            assert_eq!(
                links[0].commit_url.as_deref(),
                Some(format!("https://github.com/alpha/r/commit/{}", ids[0]).as_str())
            );
            assert_eq!(
                links[1].commit_url.as_deref(),
                Some(format!("https://github.com/up/r/commit/{only_upstream}").as_str())
            );
        }

        #[test]
        fn self_hosted_or_url_less_remote_is_contained_without_a_url() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "origin", "https://git.example.com/team/app.git");
            let ids = chain(&repo, None, 1, "c");
            set_tip(&repo, "origin", "main", ids[0]);
            // A remote-tracking ref with no configured remote at all.
            set_tip(&repo, "orphan", "main", ids[0]);

            let links = link_commits(&gix_repo(&dir), &[oid(ids[0])]).unwrap();

            assert_eq!(links[0], CommitLink { remote: Some(true), commit_url: None });
        }

        #[test]
        fn skewed_timestamps_do_not_hide_an_ancestor() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "origin", "git@github.com:o/r.git");
            // The ancestor is dated long after its descendant tip.
            let ancestor = commit(&repo, &[], "future-dated ancestor", 2_000_000_000);
            let tip = commit(&repo, &[ancestor], "old-dated tip", 1_000_000_000);
            set_tip(&repo, "origin", "main", tip);

            let links = link_commits(&gix_repo(&dir), &[oid(ancestor)]).unwrap();

            assert_eq!(links[0].remote, Some(true));
        }

        #[test]
        fn stale_tips_are_walked_after_the_default_branch_finds_the_targets() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "origin", "git@github.com:o/r.git");
            let main = chain(&repo, None, 3, "main");
            // An unrelated stale branch sorting before `main` alphabetically.
            let stale = chain(&repo, None, 50, "stale");
            set_tip(&repo, "origin", "aaa-stale", stale[49]);
            set_tip(&repo, "origin", "main", main[2]);
            repo.reference_symbolic(
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
                true,
                "remote default",
            )
            .unwrap();

            let (links, snapshot) = testing::measure(|| {
                link_commits(&gix_repo(&dir), &[oid(main[2]), oid(main[1])]).unwrap()
            });

            assert!(links.iter().all(|link| link.remote == Some(true)));
            // The default branch is walked first and stops at the second
            // target, so the 50-commit stale branch is never visited.
            assert_eq!(snapshot.get(counters::GIT_COMMIT_VISITS), 2);
        }

        #[test]
        fn budget_exhaustion_leaves_undetermined_targets_null_not_false() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "origin", "git@github.com:o/r.git");
            let history = chain(&repo, None, 20, "history");
            set_tip(&repo, "origin", "main", history[19]);
            let unpushed = commit(&repo, &[history[19]], "unpushed", 1_900_000_000);

            let gix = gix_repo(&dir);
            let targets = [oid(history[19]), oid(unpushed), oid(history[0])];

            let (links, snapshot) = testing::measure(|| {
                link_commits_with_budget(&gix, &targets, 8).unwrap()
            });

            assert_eq!(links[0].remote, Some(true), "found on the first visit");
            assert!(links[0].commit_url.is_some());
            assert_eq!(links[1], CommitLink::default(), "never reachable, but unproven");
            assert_eq!(links[2], CommitLink::default(), "reachable beyond the budget");
            assert_eq!(snapshot.get(counters::GIT_COMMIT_VISITS), 8);

            let unbounded = link_commits(&gix, &targets).unwrap();
            assert_eq!(
                unbounded.iter().map(|link| link.remote).collect::<Vec<_>>(),
                [Some(true), Some(false), Some(true)]
            );
        }

        /// The aggregate links from the ref snapshot it already took for branch
        /// facts (local and remote branches). That snapshot must decide
        /// containment exactly as a self-observed one, including the unknown
        /// state after budget exhaustion, without walking refs again.
        #[test]
        fn a_shared_branch_snapshot_links_like_a_fresh_one_without_a_ref_walk() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "origin", "git@github.com:o/r.git");
            let history = chain(&repo, None, 20, "history");
            set_tip(&repo, "origin", "main", history[19]);
            let unpushed = commit(&repo, &[history[19]], "unpushed", 1_900_000_000);

            let gix = gix_repo(&dir);
            let targets = [oid(history[19]), oid(unpushed), oid(history[0])];
            let shared = RefSnapshot::observe(&gix, true, true, false).unwrap();

            for budget in [8, COMMIT_VISIT_BUDGET] {
                let (links, counts) = testing::measure(|| {
                    link_commits_from_snapshot(&gix, &shared, &targets, budget)
                });
                assert_eq!(
                    links,
                    link_commits_with_budget(&gix, &targets, budget).unwrap(),
                    "budget {budget}"
                );
                assert_eq!(counts.get(counters::GIT_REF_WALKS), 0, "budget {budget}");
            }
            let exhausted = link_commits_from_snapshot(&gix, &shared, &targets, 8);
            assert_eq!(
                exhausted.iter().map(|link| link.remote).collect::<Vec<_>>(),
                [Some(true), None, None]
            );
        }

        #[test]
        fn budget_spent_on_one_remote_leaves_later_remotes_unwalked() {
            let (dir, repo) = setup_repo();
            add_remote(&repo, "origin", "git@github.com:o/r.git");
            add_remote(&repo, "upstream", "git@github.com:u/r.git");
            let origin_history = chain(&repo, None, 10, "origin");
            set_tip(&repo, "origin", "main", origin_history[9]);
            let upstream_only = chain(&repo, None, 1, "upstream");
            set_tip(&repo, "upstream", "main", upstream_only[0]);

            let gix = gix_repo(&dir);
            let partial =
                link_commits_with_budget(&gix, &[oid(upstream_only[0])], 10).unwrap();
            let complete =
                link_commits_with_budget(&gix, &[oid(upstream_only[0])], 11).unwrap();

            assert_eq!(partial[0].remote, None);
            assert_eq!(complete[0].remote, Some(true));
            assert_eq!(
                complete[0].commit_url.as_deref(),
                Some(format!("https://github.com/u/r/commit/{}", upstream_only[0]).as_str())
            );
        }

        #[test]
        fn empty_target_list_reads_no_refs() {
            let (dir, _repo) = setup_repo();
            let (links, snapshot) =
                testing::measure(|| link_commits(&gix_repo(&dir), &[]).unwrap());
            assert!(links.is_empty());
            assert_eq!(snapshot.get(counters::GIT_REF_WALKS), 0);
        }
    }
}
