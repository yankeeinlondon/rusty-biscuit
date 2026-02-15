---
title: GitHub REST API Research
description: Endpoint coverage, common pitfalls, and Rust examples for practical GitHub REST API usage.
prompt: |-
    Do research into the Github REST API. Document all of it's API endpoints. Discuss common gotcha's developers encounter when working with this API and how they are able to get around them. Create a Markdown table (with Markdown links) of all the key resources you've found for this API. All code examples should be written in Rust. Demonstrate the how to use the API for the following use cases:

    1. Get a list of all `README.md` filepaths (case insensitive) in the repo and then provides a means to get the content of any of these readme's that the caller wants.
    2. Get a list of all PR's for the repo with associated metadata.
    3. Get a list of all Issues for the repo along with base metadata and a means to dig into further information (if this further information requires a separate call).
    4. Get a list of all Tags from the repos including all metadata; make sure you can distinguish between a normal tag and one that is deemed a "release"

    Write all your findings to @sniff/docs/github-api.md ; replace the body of this file if it already has content but retain the frontmatter. Update the the `last_updated` frontmatter property to today's date.
last_updated: 2026-02-15
---

# GitHub REST API Research

## Scope and source of truth

- Research date: **2026-02-15**.
- Current documented REST API version: **`2022-11-28`**.
- Operation-level endpoint source of truth: GitHub's OpenAPI description (`github/rest-api-description`).
- Human-readable endpoint catalog source of truth: GitHub REST API docs navigation (`/en/rest`).

For exact, machine-readable operation coverage (method + path + operationId), use:

- OpenAPI JSON (api.github.com): `https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.json`
- OpenAPI YAML (api.github.com): `https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.yaml`

## Full endpoint surface (documentation map)

This is the full endpoint family and sub-resource map currently published in GitHub's REST docs navigation.

| Endpoint family | Sub-resources |
|---|---|
| GitHub Actions | Artifacts, Cache, GitHub-hosted runners, OIDC, Permissions, Secrets, Self-hosted runner groups, Self-hosted runners, Variables, Workflow jobs, Workflow runs, Workflows |
| Activity | Events, Feeds, Notifications, Starring, Watching |
| Apps | GitHub Apps, GitHub App installations, GitHub Marketplace, OAuth authorizations, GitHub App webhooks |
| Billing | Budgets, Billing usage |
| Branches and settings | Branches, Protected branches |
| Security campaigns | Security campaigns |
| Checks | Check runs, Check suites |
| GitHub Classroom | Classroom |
| Code scanning | Code scanning |
| Code security settings | Configurations |
| Codes of conduct | Codes of conduct |
| Codespaces | Codespaces, Organizations, Organization secrets, Machines, Repository secrets, User secrets |
| Collaborators | Collaborators, Repository invitations |
| Commits | Commits, Commit comments, Commit statuses |
| Copilot | Copilot metrics, Copilot user management |
| Credentials | Revocation |
| Dependabot | Alerts, Repository access, Secrets |
| Dependency graph | Dependency review, Dependency submission, Software bill of materials (SBOM) |
| Deploy keys | Deploy keys |
| Deployments | Deployment branch policies, Deployments, Deployment environments, Protection rules, Deployment statuses |
| Emojis | Emojis |
| Enterprise teams | Enterprise team memberships, Enterprise team organizations, Enterprise teams |
| Gists | Gists, Gist comments |
| Git database | Blobs, Commits, References, Tags, Trees |
| Gitignore | Gitignore |
| Interactions | Organization interactions, Repository interactions, User interactions |
| Issues | Issue assignees, Issue comments, Issue events, Issues, Issue dependencies, Labels, Milestones, Sub-issues, Timeline events |
| Licenses | Licenses |
| Markdown | Markdown |
| Meta data | Meta data |
| Metrics | Community metrics, Repository statistics, Repository traffic |
| Migrations | Organization migrations, Source imports, User migrations |
| Models | Models catalog, Model embeddings, Models inference |
| Organizations | API Insights, Artifact metadata, Artifact attestations, Blocking users, Custom properties, Issue types, Organization members, Network configurations, Organization roles, Organizations, Outside collaborators, Personal access tokens, Rule suites, Rules, Security managers, Organization webhooks |
| Packages | Packages |
| GitHub Pages | GitHub Pages |
| Private registries | Organization configurations |
| Projects | Draft Project items, Project fields, Project items, Projects, Project views |
| Pull requests | Pull requests, Pull request review comments, Review requests, Pull request reviews |
| Rate limits | Rate limits |
| Reactions | Reactions |
| Releases | Releases, Release assets |
| Repositories | Repository attestations, Repository autolinks, Repository contents, Custom properties, Forks, Repositories, Rule suites, Rules, Repository tags, Repository webhooks |
| Search | Search |
| Secret scanning | Push protection, Secret scanning |
| Security advisories | Global security advisories, Repository security advisories |
| Teams | Team members, Teams |
| Users | Artifact attestations, Blocking users, Emails, Followers, GPG keys, Git SSH keys, Social accounts, SSH signing keys, Users |

## Key resources (with links)

| Resource | Why it matters | Link |
|---|---|---|
| REST API home | Primary index and endpoint navigation | [GitHub REST API docs](https://docs.github.com/en/rest?apiVersion=2022-11-28) |
| API versioning | Required version header behavior | [API Versions](https://docs.github.com/en/rest/about-the-rest-api/api-versions) |
| OpenAPI overview | Canonical spec and generation guidance | [About the OpenAPI description](https://docs.github.com/en/rest/about-the-rest-api/about-the-openapi-description-for-the-rest-api) |
| OpenAPI repo | Official source for JSON/YAML specs | [github/rest-api-description](https://github.com/github/rest-api-description) |
| OpenAPI JSON (api.github.com) | Full operation-level list | [api.github.com.json](https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.json) |
| Authentication | Token models and auth patterns | [Authenticating to the REST API](https://docs.github.com/en/rest/authentication/authenticating-to-the-rest-api) |
| Rate limits | Primary and secondary limits | [Rate limits for the REST API](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api) |
| Pagination | Correct handling of `Link` headers | [Using pagination in the REST API](https://docs.github.com/en/rest/using-the-rest-api/using-pagination-in-the-rest-api) |
| Best practices | Concurrency, retry, conditional requests | [Best practices for using the REST API](https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api) |
| Troubleshooting | Common 404/422/version issues | [Troubleshooting the REST API](https://docs.github.com/en/rest/using-the-rest-api/troubleshooting-the-rest-api) |
| Repository contents | README and file content retrieval | [Repository contents endpoints](https://docs.github.com/en/rest/repos/contents) |
| Git trees | Recursive file discovery and truncation rules | [Git trees endpoints](https://docs.github.com/en/rest/git/trees) |
| Pull requests | PR list + metadata and linked resources | [Pull requests endpoints](https://docs.github.com/en/rest/pulls/pulls) |
| Issues | Issue listing and base metadata | [Issues endpoints](https://docs.github.com/en/rest/issues/issues) |
| Timeline events | Additional issue/PR lifecycle detail | [Timeline endpoints](https://docs.github.com/en/rest/issues/timeline) |
| Repository tags | Raw tag list (`/repos/{owner}/{repo}/tags`) | [Repositories endpoints](https://docs.github.com/en/rest/repos/repos?apiVersion=2022-11-28) |
| Releases | Release metadata and tag linkage | [Releases endpoints](https://docs.github.com/en/rest/releases/releases) |
| Git tags | Annotated tag object details | [Git tags endpoints](https://docs.github.com/en/rest/git/tags) |
| Git refs | Ref type checks (`commit` vs `tag`) | [Git references endpoints](https://docs.github.com/en/rest/git/refs) |
| Search | Separate limits and 1,000-result cap | [Search endpoints](https://docs.github.com/en/rest/search/search) |

## Common gotchas and practical workarounds

| Gotcha | Why it happens | How to get around it |
|---|---|---|
| Missing `X-GitHub-Api-Version` | Versioned API; unsupported versions return `400` | Send `X-GitHub-Api-Version: 2022-11-28` on every request. |
| Token has wrong permissions | Fine-grained PATs and GitHub App tokens are endpoint-permission specific | Check endpoint permission requirements and inspect `X-Accepted-GitHub-Permissions` when you get "Resource not accessible" errors. |
| `404` for real private resources | GitHub intentionally returns `404` instead of `403` for unauthorized private resources | Treat unexpected `404` as possible auth/permission failure, not always "not found." |
| Incomplete lists | Most list endpoints return only 30 items by default | Always paginate via `Link` header (`rel="next"`) until exhausted. |
| Secondary rate limits | Too much concurrency or too many mutating requests in a short period | Serialize requests where possible, add backoff, and obey `retry-after`/`x-ratelimit-reset`. |
| Polling-heavy integrations | Polling burns rate budget and can trigger secondary limits | Prefer webhooks for change-driven workflows. |
| `Contents` API misses files in large directories | Directory listing via contents API is capped at 1,000 entries | Use Git Trees API for recursive file discovery. |
| Recursive tree truncation | `GET /git/trees/{sha}?recursive=1` can truncate at 100,000 entries / 7 MB | If `truncated=true`, walk trees non-recursively subtree-by-subtree. |
| `Issues` endpoint includes PRs | PRs are a subtype of issue in GitHub's model | Filter out items with `pull_request` key when you want only issues. |
| Search missing expected results | Search has custom limits (including 1,000 max results) and strict query limits | Narrow queries, paginate within caps, and expect search-specific rate limits. |
| Tag confusion (`tag` vs `release`) | Git tags and GitHub Releases are related but distinct resources | Join `/repos/{owner}/{repo}/tags` with `/repos/{owner}/{repo}/releases` by `tag_name`. |
| Lightweight vs annotated tag confusion | Git tag objects endpoint handles annotated tag objects | Inspect `GET /git/ref/tags/{name}` object type (`commit` vs `tag`), then call `/git/tags/{sha}` only for annotated tags. |

## Rust examples

### Dependencies

```toml
[dependencies]
anyhow = "1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
urlencoding = "2"
```

### Shared client and pagination helpers

```rust
use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, LINK, USER_AGENT},
    Client,
};
use serde::de::DeserializeOwned;

const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";

#[derive(Clone)]
struct GitHubClient {
    http: Client,
}

impl GitHubClient {
    fn new(token: Option<&str>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static(API_VERSION),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("rusty-biscuit-github-api"));

        if let Some(token) = token {
            let value = format!("Bearer {token}");
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&value)?);
        }

        let http = Client::builder().default_headers(headers).build()?;
        Ok(Self { http })
    }

    async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json::<T>().await?)
    }

    async fn get_text_with_accept(&self, url: &str, accept: &'static str) -> Result<String> {
        let resp = self
            .http
            .get(url)
            .header(ACCEPT, accept)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.text().await?)
    }

    async fn get_paginated<T: DeserializeOwned>(&self, url: &str) -> Result<Vec<T>> {
        let mut all = Vec::new();
        let mut next = Some(url.to_string());

        while let Some(current) = next.take() {
            let resp = self.http.get(&current).send().await?.error_for_status()?;
            next = next_link(resp.headers());
            let mut page: Vec<T> = resp.json().await?;
            all.append(&mut page);
        }

        Ok(all)
    }
}

fn next_link(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(LINK)?.to_str().ok()?;

    for entry in raw.split(',') {
        let mut parts = entry.trim().split(';').map(str::trim);
        let url_part = parts.next()?;
        let rel_part = parts.next().unwrap_or_default();

        if rel_part == r#"rel="next""# {
            return Some(
                url_part
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string(),
            );
        }
    }

    None
}
```

### 1) List all `README.md` paths (case-insensitive), then fetch selected README content

```rust
use anyhow::Result;
use serde::Deserialize;
use std::collections::VecDeque;

#[derive(Debug, Deserialize)]
struct RepoInfo {
    default_branch: String,
}

#[derive(Debug, Deserialize)]
struct GitTreeResponse {
    tree: Vec<GitTreeEntry>,
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct GitTreeEntry {
    path: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    sha: String,
}

async fn default_branch(client: &GitHubClient, owner: &str, repo: &str) -> Result<String> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}");
    let info: RepoInfo = client.get_json(&url).await?;
    Ok(info.default_branch)
}

async fn get_tree(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    tree_sha_or_ref: &str,
    recursive: bool,
) -> Result<GitTreeResponse> {
    let mut url = format!("{API_BASE}/repos/{owner}/{repo}/git/trees/{tree_sha_or_ref}");
    if recursive {
        url.push_str("?recursive=1");
    }
    client.get_json(&url).await
}

async fn list_all_repo_files(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    default_branch: &str,
) -> Result<Vec<String>> {
    let recursive = get_tree(client, owner, repo, default_branch, true).await?;
    if !recursive.truncated {
        return Ok(recursive
            .tree
            .into_iter()
            .filter(|e| e.node_type == "blob")
            .map(|e| e.path)
            .collect());
    }

    // Fallback path when recursive tree is truncated.
    let mut files = Vec::new();
    let mut queue = VecDeque::from([(String::new(), default_branch.to_string())]);

    while let Some((prefix, tree_sha_or_ref)) = queue.pop_front() {
        let chunk = get_tree(client, owner, repo, &tree_sha_or_ref, false).await?;

        for entry in chunk.tree {
            let full_path = if prefix.is_empty() {
                entry.path.clone()
            } else {
                format!("{prefix}/{}", entry.path)
            };

            match entry.node_type.as_str() {
                "blob" => files.push(full_path),
                "tree" => queue.push_back((full_path, entry.sha)),
                _ => {}
            }
        }
    }

    Ok(files)
}

fn is_case_insensitive_readme_md(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .map(|name| name.eq_ignore_ascii_case("readme.md"))
        .unwrap_or(false)
}

async fn list_readme_paths(client: &GitHubClient, owner: &str, repo: &str) -> Result<Vec<String>> {
    let branch = default_branch(client, owner, repo).await?;
    let mut readmes: Vec<String> = list_all_repo_files(client, owner, repo, &branch)
        .await?
        .into_iter()
        .filter(|p| is_case_insensitive_readme_md(p))
        .collect();

    readmes.sort();
    readmes.dedup();
    Ok(readmes)
}

async fn get_readme_content(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: Option<&str>,
) -> Result<String> {
    // Use raw media type so caller gets plain text directly.
    let encoded_path = path
        .split('/')
        .map(urlencoding::encode)
        .collect::<Vec<_>>()
        .join("/");

    let mut url = format!("{API_BASE}/repos/{owner}/{repo}/contents/{encoded_path}");
    if let Some(git_ref) = git_ref {
        url.push_str(&format!("?ref={}", urlencoding::encode(git_ref)));
    }

    client
        .get_text_with_accept(&url, "application/vnd.github.raw+json")
        .await
}
```

### 2) List all PRs with associated metadata

```rust
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct UserSummary {
    login: String,
}

#[derive(Debug, Deserialize)]
struct BranchRef {
    #[serde(rename = "ref")]
    ref_name: String,
    sha: String,
}

#[derive(Debug, Deserialize)]
struct PullRequestSummary {
    number: u64,
    state: String,
    title: String,
    draft: Option<bool>,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
    html_url: String,
    url: String,
    issue_url: String,
    comments_url: String,
    review_comments_url: String,
    commits_url: String,
    statuses_url: String,
    user: UserSummary,
    base: BranchRef,
    head: BranchRef,
}

async fn list_pull_requests(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<PullRequestSummary>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/pulls?state=all&sort=updated&direction=desc&per_page=100"
    );
    client.get_paginated(&url).await
}

// Optional drill-down calls if you need deeper metadata.
#[derive(Debug, Deserialize)]
struct PullFile {
    filename: String,
    status: String,
    additions: i64,
    deletions: i64,
    changes: i64,
}

async fn list_pull_request_files(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pull_number: u64,
) -> Result<Vec<PullFile>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/pulls/{pull_number}/files?per_page=100"
    );
    client.get_paginated(&url).await
}
```

### 3) List all issues with base metadata and follow-up endpoints for deeper detail

```rust
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct IssueSummary {
    id: u64,
    number: u64,
    state: String,
    title: String,
    user: UserSummary,
    comments: u64,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    url: String,
    html_url: String,
    comments_url: String,
    events_url: String,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct IssueComment {
    id: u64,
    user: UserSummary,
    body: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct TimelineEvent {
    id: Option<u64>,
    event: Option<String>,
    created_at: Option<String>,
    actor: Option<UserSummary>,
}

async fn list_issues(client: &GitHubClient, owner: &str, repo: &str) -> Result<Vec<IssueSummary>> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/issues?state=all&per_page=100");
    client.get_paginated(&url).await
}

async fn get_issue(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<IssueSummary> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/issues/{issue_number}");
    client.get_json(&url).await
}

async fn list_issue_comments(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<Vec<IssueComment>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/issues/{issue_number}/comments?per_page=100"
    );
    client.get_paginated(&url).await
}

async fn list_issue_timeline(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<Vec<TimelineEvent>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/issues/{issue_number}/timeline?per_page=100"
    );
    client.get_paginated(&url).await
}

// Notes:
// 1) List issues may include PRs; check `pull_request.is_some()`.
// 2) Use comments/events/timeline endpoints for deeper detail.
```

### 4) List all tags and distinguish plain tags from release tags

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct RepoTag {
    name: String,
    commit: TagCommit,
    tarball_url: String,
    zipball_url: String,
    node_id: String,
}

#[derive(Debug, Deserialize)]
struct TagCommit {
    sha: String,
    url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Release {
    id: u64,
    tag_name: String,
    name: Option<String>,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    immutable: Option<bool>,
    created_at: String,
    published_at: Option<String>,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GitRef {
    #[serde(rename = "ref")]
    ref_name: String,
    object: GitRefObject,
}

#[derive(Debug, Deserialize)]
struct GitRefObject {
    #[serde(rename = "type")]
    object_type: String, // "commit" => lightweight, "tag" => annotated tag object
    sha: String,
    url: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnnotatedTagObject {
    sha: String,
    tag: String,
    message: String,
    tagger: Option<AnnotatedTagger>,
    verification: Option<TagVerification>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnnotatedTagger {
    name: String,
    email: String,
    date: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TagVerification {
    verified: bool,
    reason: String,
    verified_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum TagKind {
    Lightweight,
    Annotated,
}

#[derive(Debug, Serialize)]
struct ReleaseSummary {
    id: u64,
    name: Option<String>,
    draft: bool,
    prerelease: bool,
    immutable: Option<bool>,
    created_at: String,
    published_at: Option<String>,
    html_url: String,
}

#[derive(Debug, Serialize)]
struct TagReport {
    name: String,
    commit_sha: String,
    tarball_url: String,
    zipball_url: String,
    node_id: String,
    tag_kind: TagKind,
    git_ref: String,
    git_ref_object_url: String,
    is_release_tag: bool,
    release: Option<ReleaseSummary>,
    annotated_tag: Option<AnnotatedTagObject>,
}

async fn list_tags_with_release_classification(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<TagReport>> {
    let tags_url = format!("{API_BASE}/repos/{owner}/{repo}/tags?per_page=100");
    let releases_url = format!("{API_BASE}/repos/{owner}/{repo}/releases?per_page=100");

    let tags: Vec<RepoTag> = client.get_paginated(&tags_url).await?;
    let releases: Vec<Release> = client.get_paginated(&releases_url).await?;

    let by_tag_name: HashMap<String, Release> = releases
        .into_iter()
        .map(|r| (r.tag_name.clone(), r))
        .collect();

    let mut out = Vec::with_capacity(tags.len());

    for tag in tags {
        let encoded_name = urlencoding::encode(&tag.name);
        let ref_url = format!("{API_BASE}/repos/{owner}/{repo}/git/ref/tags/{encoded_name}");
        let git_ref: GitRef = client.get_json(&ref_url).await?;

        let (tag_kind, annotated_tag) = if git_ref.object.object_type == "tag" {
            let tag_url = format!("{API_BASE}/repos/{owner}/{repo}/git/tags/{}", git_ref.object.sha);
            let detail: AnnotatedTagObject = client.get_json(&tag_url).await?;
            (TagKind::Annotated, Some(detail))
        } else {
            (TagKind::Lightweight, None)
        };

        let release = by_tag_name.get(&tag.name).cloned();
        let release_summary = release.map(|r| ReleaseSummary {
            id: r.id,
            name: r.name,
            draft: r.draft,
            prerelease: r.prerelease,
            immutable: r.immutable,
            created_at: r.created_at,
            published_at: r.published_at,
            html_url: r.html_url,
        });

        out.push(TagReport {
            name: tag.name,
            commit_sha: tag.commit.sha,
            tarball_url: tag.tarball_url,
            zipball_url: tag.zipball_url,
            node_id: tag.node_id,
            tag_kind,
            git_ref: git_ref.ref_name,
            git_ref_object_url: git_ref.object.url,
            is_release_tag: release_summary.is_some(),
            release: release_summary,
            annotated_tag,
        });
    }

    Ok(out)
}
```

### Optional: generate a complete operation list directly from OpenAPI (Rust)

This is the most reliable way to enumerate every REST operation path currently published.

```rust
use anyhow::{Context, Result};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    let openapi_url = "https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.json";

    let spec: Value = reqwest::get(openapi_url)
        .await?
        .error_for_status()?
        .json()
        .await?;

    let paths = spec
        .get("paths")
        .and_then(Value::as_object)
        .context("OpenAPI spec missing `paths`")?;

    let methods = ["get", "post", "put", "patch", "delete", "head", "options"];
    let mut rows = Vec::new();

    for (path, item) in paths {
        let ops = item.as_object().context("path item is not an object")?;
        for method in methods {
            if let Some(op) = ops.get(method) {
                let operation_id = op
                    .get("operationId")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                rows.push((method.to_uppercase(), path.to_string(), operation_id.to_string()));
            }
        }
    }

    rows.sort();
    println!("total operations: {}", rows.len());

    for (method, path, operation_id) in rows {
        println!("{method:6} {path}  # {operation_id}");
    }

    Ok(())
}
```

## Practical implementation notes

- Prefer authenticated requests, even for public data, to avoid low unauthenticated limits.
- Use `per_page=100` plus `Link` pagination.
- Add retry/backoff behavior for `403`/`429`, respecting `retry-after` and `x-ratelimit-reset`.
- Treat `404` as potentially authorization-related for private resources.
- For large repositories, assume recursive tree truncation can happen and support subtree traversal.
- For tag/release reporting, merge data from `tags`, `releases`, and Git ref/tag-object endpoints.
