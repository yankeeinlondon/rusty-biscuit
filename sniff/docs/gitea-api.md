---
title: Gitea REST API Research
description: Endpoint coverage, common pitfalls, and Rust examples for practical Gitea REST API usage.
prompt: |-
    Do research into the Gitea REST API. Document all of it's API endpoints. Discuss common gotcha's developers encounter when working with this API and how they are able to get around them. Create a Markdown table (with Markdown links) of all the key resources you've found for this API. All code examples should be written in Rust. Demonstrate the how to use the API for the following use cases:

    1. Get a list of all `README.md` filepaths (case insensitive) in the repo and then provides a means to get the content of any of these readme's that the caller wants.
    2. Get a list of all PR's for the repo with associated metadata.
    3. Get a list of all Issues for the repo along with base metadata and a means to dig into further information (if this further information requires a separate call).
    4. Get a list of all Tags from the repos including all metadata; make sure you can distinguish between a normal tag and one that is deemed a "release"

    Finally near the end, add a section which compares the Github and Gitea API's in capability and approach. You can refer to the Github API's design in the [Github API](@sniff/docs/github.md).

    Write all your findings to @sniff/docs/gitea-api.md ; replace the body of this file if it already has content but retain the frontmatter. Update the the `last_updated` frontmatter property to today's date.
last_updated: 2026-02-15
---
# Gitea REST API Research

## Scope and source of truth

- Research date: **2026-02-15**.
- Gitea docs version observed: **1.25.4**.
- Canonical OpenAPI source used for endpoint inventory: [`plugin-redoc-2.yaml`](https://docs.gitea.com/redocusaurus/plugin-redoc-2.yaml) (OpenAPI/Swagger 2.0, `info.version = 1.25.4`).
- OpenAPI `basePath` in that spec is `https://gitea.com/api/v1`; for self-hosted Gitea, replace host with your own (`https://<your-gitea-host>/api/v1`).

## Key resources (with links)

| Resource | Why it matters | Link |
|---|---|---|
| API usage guide | Auth models, pagination, sudo, SDK pointers | [docs.gitea.com/development/api-usage](https://docs.gitea.com/development/api-usage) |
| API reference (1.25) | Interactive endpoint documentation | [docs.gitea.com/api/1.25](https://docs.gitea.com/api/1.25/) |
| OpenAPI source used in this document | Machine-readable endpoint list (used for exhaustive inventory) | [plugin-redoc-2.yaml](https://docs.gitea.com/redocusaurus/plugin-redoc-2.yaml) |
| FAQ: Swagger location | Confirms where `/api/swagger` and `swagger.v1.json` are exposed | [FAQ: What is Swagger?](https://docs.gitea.com/help/faq#what-is-swagger) |
| API router package (v1.25.4) | Upstream route organization by API domain | [routers/api/v1](https://pkg.go.dev/code.gitea.io/gitea/routers/api/v1) |
| Repository API package | Upstream handler coverage for repo/pulls/tags/releases/contents trees | [routers/api/v1/repo](https://pkg.go.dev/code.gitea.io/gitea/routers/api/v1/repo) |
| Issues API package | Upstream handler coverage for issue resources | [routers/api/v1/issue](https://pkg.go.dev/code.gitea.io/gitea/routers/api/v1/issue) |
| Org API package | Upstream handler coverage for org/team resources | [routers/api/v1/org](https://pkg.go.dev/code.gitea.io/gitea/routers/api/v1/org) |
| User API package | Upstream handler coverage for user-scoped resources | [routers/api/v1/user](https://pkg.go.dev/code.gitea.io/gitea/routers/api/v1/user) |
| Known large-file contents edge case | Real-world issue where `contents` payload can return `content: null` | [go-gitea/gitea#18055](https://github.com/go-gitea/gitea/issues/18055) |

## Full endpoint surface summary

The OpenAPI file for Gitea **1.25.4** contains:

- **299** unique paths
- **467** operations (method + path)
- **10** endpoint tags

| Tag | Operations |
|---|---:|
| `repository` | 191 |
| `user` | 75 |
| `issue` | 69 |
| `organization` | 66 |
| `admin` | 32 |
| `miscellaneous` | 13 |
| `package` | 8 |
| `notification` | 7 |
| `settings` | 4 |
| `activitypub` | 2 |

A full operation-by-operation inventory (all 467 endpoints) is included at the end of this document.

## Common gotchas and workarounds

| Gotcha | Why it happens | Workaround |
|---|---|---|
| Using `Bearer` with a personal access token fails on some instances | Gitea documents PAT auth as `Authorization: token <token>` | For PATs, always send `Authorization: token <token>`; reserve bearer for OAuth2 flows. |
| Query-string auth (`token=`, `access_token=`) lingers in old clients | OpenAPI still lists these as deprecated auth schemes | Avoid query auth entirely; use header auth only. |
| `users/{name}/tokens` calls fail unexpectedly | Token management endpoint expects `BasicAuth`; if 2FA is enabled it also needs OTP | Send Basic auth and `X-Gitea-OTP` when required. |
| Pagination is missed, causing incomplete results | Many list endpoints default to server-limited pages; docs call out `MAX_RESPONSE_ITEMS` default 50 | Iterate with `page`/`limit` and parse `Link`; optionally read `x-total-count`. |
| `/api/swagger` not available on an instance | Swagger UI depends on `ENABLE_SWAGGER` | Fall back to instance `swagger.v1.json` endpoint if available, or use upstream docs spec. |
| `contents` API can return `content: null` for large files | Known behavior discussed in upstream issue tracker | Fall back to `/repos/{owner}/{repo}/raw/{filepath}` or `download_url`. |
| Issues list includes pull requests when unfiltered | Gitea models PRs with issue-like metadata, and `issueListIssues` supports a `type` filter | For issue-only workflows, call `/issues` with `type=issues`. |
| Git tags vs Releases confusion | Tags (`/tags`) and Releases (`/releases`) are separate resources | Join by `tag_name`: every release points to a tag, but many tags have no release. |
| Annotated vs lightweight tag ambiguity | `/tags` list alone doesn’t fully describe tag object kind | Resolve ref via `/git/refs/{ref}` (e.g. `tags/v1.2.3`); if object type is `tag`, fetch `/git/tags/{sha}`. |
| “Missing releases” after migration despite tags existing | FAQ notes release records and git tags can be out of sync post-migration | Ensure tags are pushed and run release/tag sync on the server side (`repo-sync-releases`). |

## Rust examples

The examples below are intentionally API-first and compile-friendly, with resilient `serde` models (optional fields where instances differ).

### Shared client and pagination helper

```rust
use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, LINK},
    Client,
};
use serde::de::DeserializeOwned;

const API_BASE: &str = "https://gitea.example.com/api/v1";

#[derive(Clone)]
pub struct GiteaClient {
    http: Client,
}

impl GiteaClient {
    pub fn new_personal_token(token: Option<&str>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(token) = token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("token {token}"))?,
            );
        }

        let http = Client::builder().default_headers(headers).build()?;
        Ok(Self { http })
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json::<T>().await?)
    }

    pub async fn get_text(&self, url: &str) -> Result<String> {
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.text().await?)
    }

    pub async fn get_paginated<T: DeserializeOwned>(&self, url: &str) -> Result<Vec<T>> {
        let mut out = Vec::new();
        let mut next = Some(url.to_string());

        while let Some(current) = next.take() {
            let resp = self.http.get(&current).send().await?.error_for_status()?;
            next = parse_next_link(resp.headers());
            let mut page: Vec<T> = resp.json().await?;
            out.append(&mut page);
        }

        Ok(out)
    }
}

fn parse_next_link(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(LINK)?.to_str().ok()?;

    for entry in raw.split(',') {
        let mut parts = entry.trim().split(';').map(str::trim);
        let url_part = parts.next()?;
        let rel_part = parts.next().unwrap_or_default();

        if rel_part == r#"rel=\"next\""# {
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

### 1) List all `README.md` paths (case-insensitive) and fetch selected README content

```rust
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use std::collections::VecDeque;

#[derive(Debug, Deserialize)]
struct RepoInfo {
    default_branch: String,
}

#[derive(Debug, Deserialize)]
struct GitTreeResponse {
    tree: Vec<GitTreeEntry>,
    #[serde(default)]
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

#[derive(Debug, Deserialize)]
struct RepoContent {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    encoding: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
}

async fn default_branch(client: &GiteaClient, owner: &str, repo: &str) -> Result<String> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}");
    Ok(client.get_json::<RepoInfo>(&url).await?.default_branch)
}

async fn get_tree(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    sha_or_ref: &str,
    recursive: bool,
) -> Result<GitTreeResponse> {
    let mut url = format!("{API_BASE}/repos/{owner}/{repo}/git/trees/{sha_or_ref}");
    if recursive {
        url.push_str("?recursive=true");
    }
    client.get_json(&url).await
}

async fn list_all_repo_files(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Vec<String>> {
    let first = get_tree(client, owner, repo, branch, true).await?;

    if !first.truncated {
        return Ok(first
            .tree
            .into_iter()
            .filter(|e| e.node_type == "blob")
            .map(|e| e.path)
            .collect());
    }

    // Fallback when recursive tree is truncated.
    let mut files = Vec::new();
    let mut queue = VecDeque::from([(String::new(), branch.to_string())]);

    while let Some((prefix, sha_or_ref)) = queue.pop_front() {
        let chunk = get_tree(client, owner, repo, &sha_or_ref, false).await?;
        for entry in chunk.tree {
            let full = if prefix.is_empty() {
                entry.path.clone()
            } else {
                format!("{prefix}/{}", entry.path)
            };

            match entry.node_type.as_str() {
                "blob" => files.push(full),
                "tree" => queue.push_back((full, entry.sha)),
                _ => {}
            }
        }
    }

    Ok(files)
}

fn is_readme_md_case_insensitive(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .map(|name| name.eq_ignore_ascii_case("readme.md"))
        .unwrap_or(false)
}

pub async fn list_readme_paths(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<String>> {
    let branch = default_branch(client, owner, repo).await?;
    let mut paths: Vec<String> = list_all_repo_files(client, owner, repo, &branch)
        .await?
        .into_iter()
        .filter(|p| is_readme_md_case_insensitive(p))
        .collect();

    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub async fn get_readme_content(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: Option<&str>,
) -> Result<String> {
    let encoded_path = path
        .split('/')
        .map(urlencoding::encode)
        .collect::<Vec<_>>()
        .join("/");

    let mut contents_url = format!(
        "{API_BASE}/repos/{owner}/{repo}/contents/{encoded_path}"
    );
    if let Some(git_ref) = git_ref {
        contents_url.push_str(&format!("?ref={}", urlencoding::encode(git_ref)));
    }

    let content: RepoContent = client.get_json(&contents_url).await?;

    if let (Some(enc), Some(raw)) = (content.encoding.as_deref(), content.content.as_deref()) {
        if enc.eq_ignore_ascii_case("base64") {
            let bytes = STANDARD.decode(raw.replace('\n', ""))?;
            return String::from_utf8(bytes).map_err(Into::into);
        }
    }

    if let Some(download_url) = content.download_url {
        return client.get_text(&download_url).await;
    }

    // Final fallback for large files on some instances.
    let mut raw_url = format!("{API_BASE}/repos/{owner}/{repo}/raw/{encoded_path}");
    if let Some(git_ref) = git_ref {
        raw_url.push_str(&format!("?ref={}", urlencoding::encode(git_ref)));
    }

    client.get_text(&raw_url).await
}
```

### 2) List all PRs with associated metadata

```rust
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct UserSummary {
    id: Option<i64>,
    login: Option<String>,
    username: Option<String>,
    full_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PullRef {
    #[serde(rename = "ref")]
    ref_name: Option<String>,
    sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PullRequestSummary {
    id: Option<i64>,
    number: Option<i64>,
    state: Option<String>,
    title: Option<String>,
    body: Option<String>,
    draft: Option<bool>,
    mergeable: Option<bool>,
    has_merged: Option<bool>,
    created_at: Option<String>,
    updated_at: Option<String>,
    merged_at: Option<String>,
    html_url: Option<String>,
    url: Option<String>,
    user: Option<UserSummary>,
    head: Option<PullRef>,
    base: Option<PullRef>,
}

pub async fn list_pull_requests(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<PullRequestSummary>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/pulls?state=all&sort=recentupdate&limit=50"
    );
    client.get_paginated(&url).await
}

pub async fn get_pull_request(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    index: i64,
) -> Result<PullRequestSummary> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{index}");
    client.get_json(&url).await
}

#[derive(Debug, Deserialize)]
struct PullFile {
    filename: Option<String>,
    status: Option<String>,
    additions: Option<i64>,
    deletions: Option<i64>,
    changes: Option<i64>,
}

pub async fn list_pull_files(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    index: i64,
) -> Result<Vec<PullFile>> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{index}/files?page=1&limit=50");
    client.get_paginated(&url).await
}
```

### 3) List all Issues with base metadata and follow-up detail calls

```rust
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct IssueSummary {
    id: Option<i64>,
    index: Option<i64>,
    number: Option<i64>,
    state: Option<String>,
    title: Option<String>,
    body: Option<String>,
    comments: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
    closed_at: Option<String>,
    user: Option<UserSummary>,
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct IssueComment {
    id: Option<i64>,
    body: Option<String>,
    user: Option<UserSummary>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TimelineEntry {
    id: Option<i64>,
    #[serde(rename = "type")]
    event_type: Option<String>,
    created: Option<String>,
}

pub async fn list_issues(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<IssueSummary>> {
    // type=issues avoids PR rows in most cases.
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/issues?state=all&type=issues&limit=50"
    );
    client.get_paginated(&url).await
}

pub async fn get_issue(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    index: i64,
) -> Result<IssueSummary> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/issues/{index}");
    client.get_json(&url).await
}

pub async fn list_issue_comments(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    index: i64,
) -> Result<Vec<IssueComment>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/issues/{index}/comments?page=1&limit=50"
    );
    client.get_paginated(&url).await
}

pub async fn list_issue_timeline(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    index: i64,
) -> Result<Vec<TimelineEntry>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/issues/{index}/timeline?page=1&limit=50"
    );
    client.get_paginated(&url).await
}
```

### 4) List tags and distinguish plain tags vs release tags

```rust
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
struct TagCommit {
    sha: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct RepoTag {
    name: String,
    id: Option<String>,
    message: Option<String>,
    commit: Option<TagCommit>,
}

#[derive(Debug, Deserialize, Clone)]
struct Release {
    id: i64,
    tag_name: String,
    name: Option<String>,
    draft: Option<bool>,
    prerelease: Option<bool>,
    created_at: Option<String>,
    published_at: Option<String>,
    html_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitRefObject {
    #[serde(rename = "type")]
    object_type: String,
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GitRef {
    #[serde(rename = "ref")]
    ref_name: Option<String>,
    object: GitRefObject,
}

#[derive(Debug, Deserialize)]
struct AnnotatedTag {
    tag: Option<String>,
    message: Option<String>,
}

#[derive(Debug)]
enum TagKind {
    PlainTag,
    ReleaseTag(Release),
}

#[derive(Debug)]
struct ClassifiedTag {
    tag: RepoTag,
    kind: TagKind,
    is_annotated_tag_object: Option<bool>,
    annotated_message: Option<String>,
}

pub async fn list_tags(client: &GiteaClient, owner: &str, repo: &str) -> Result<Vec<RepoTag>> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/tags?page=1&limit=50");
    client.get_paginated(&url).await
}

pub async fn list_releases(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<Release>> {
    let url = format!(
        "{API_BASE}/repos/{owner}/{repo}/releases?draft=true&pre-release=true&page=1&limit=50"
    );
    client.get_paginated(&url).await
}

async fn tag_ref_kind(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    tag_name: &str,
) -> Result<GitRefObject> {
    let ref_name = format!("tags/{tag_name}");
    let encoded = urlencoding::encode(&ref_name);
    let url = format!("{API_BASE}/repos/{owner}/{repo}/git/refs/{encoded}");
    let refs: Vec<GitRef> = client.get_json(&url).await?;

    refs.into_iter()
        .find(|r| {
            r.ref_name
                .as_deref()
                .map(|name| name == format!("refs/{ref_name}") || name.ends_with(&ref_name))
                .unwrap_or(true)
        })
        .map(|r| r.object)
        .ok_or_else(|| anyhow!("tag ref not found: {tag_name}"))
}

async fn annotated_tag(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
    sha: &str,
) -> Result<AnnotatedTag> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/git/tags/{sha}");
    client.get_json(&url).await
}

pub async fn classify_tags(
    client: &GiteaClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<ClassifiedTag>> {
    let tags = list_tags(client, owner, repo).await?;
    let releases = list_releases(client, owner, repo).await?;

    let releases_by_tag: HashMap<String, Release> = releases
        .into_iter()
        .map(|r| (r.tag_name.clone(), r))
        .collect();

    let mut out = Vec::with_capacity(tags.len());

    for tag in tags {
        let (is_annotated_tag_object, annotated_message) =
            match tag_ref_kind(client, owner, repo, &tag.name).await {
                Ok(obj) if obj.object_type == "tag" => {
                    let details = annotated_tag(client, owner, repo, &obj.sha).await.ok();
                    (Some(true), details.and_then(|d| d.message))
                }
                Ok(_) => (Some(false), None),
                Err(_) => (None, None),
            };

        let kind = if let Some(release) = releases_by_tag.get(&tag.name) {
            TagKind::ReleaseTag(release.clone())
        } else {
            TagKind::PlainTag
        };

        out.push(ClassifiedTag {
            tag,
            kind,
            is_annotated_tag_object,
            annotated_message,
        });
    }

    Ok(out)
}
```

## GitHub vs Gitea API comparison

Reference for GitHub side: [`@sniff/docs/github-api.md`](@sniff/docs/github-api.md).

| Dimension | Gitea | GitHub |
|---|---|---|
| Primary contract source | Versioned docs + OpenAPI (`plugin-redoc-*.yaml`) | Versioned REST docs + official OpenAPI repo |
| API base URL model | Instance-specific (`https://<host>/api/v1`) | Fixed SaaS endpoint (`https://api.github.com`) |
| Version negotiation | No required request version header in normal usage | Explicit version header (`X-GitHub-Api-Version`) is expected |
| PAT auth convention | `Authorization: token <pat>` | `Authorization: Bearer <token>` |
| Query-token auth | Present but deprecated in spec | Not the standard pattern |
| Pagination | `page` + `limit`, with `Link` and `x-total-count` | `page` + `per_page`, with `Link`; different rate-limit semantics |
| Admin APIs | Rich server-admin endpoints when authenticated as admin | GitHub.com user integrations have no equivalent server-admin API |
| Tags vs releases | Separate resources; explicit server-side sync concerns are documented in FAQ | Separate resources; less migration/sync friction in hosted SaaS context |
| API ecosystem | Strongly GitHub-like shape for repo/issue/pr workflows, plus self-hosting controls | Broad ecosystem + stronger hosted platform guarantees and quotas |

Inference: for repo-centric automation (contents, PRs, issues, tags/releases), the mental model ports cleanly from GitHub to Gitea. The main design difference is operational: Gitea exposes self-hosting/admin surfaces and instance-specific behavior you must handle in clients.


## Full endpoint inventory (OpenAPI generated)

Generated from `https://docs.gitea.com/redocusaurus/plugin-redoc-2.yaml` (version `1.25.4`) on 2026-02-15.

Total operations: **467** across **10** tags.

### activitypub (2)

| Method | Path | operationId |
|---|---|---|
| GET | `/activitypub/user-id/{user-id}` | `activitypubPerson` |
| POST | `/activitypub/user-id/{user-id}/inbox` | `activitypubPersonInbox` |

### admin (32)

| Method | Path | operationId |
|---|---|---|
| DELETE | `/admin/actions/runners/{runner_id}` | `deleteAdminRunner` |
| DELETE | `/admin/hooks/{id}` | `adminDeleteHook` |
| DELETE | `/admin/unadopted/{owner}/{repo}` | `adminDeleteUnadoptedRepository` |
| DELETE | `/admin/users/{username}/badges` | `adminDeleteUserBadges` |
| DELETE | `/admin/users/{username}/keys/{id}` | `adminDeleteUserPublicKey` |
| DELETE | `/admin/users/{username}` | `adminDeleteUser` |
| GET | `/admin/actions/jobs` | `listAdminWorkflowJobs` |
| GET | `/admin/actions/runners/{runner_id}` | `getAdminRunner` |
| GET | `/admin/actions/runners` | `getAdminRunners` |
| GET | `/admin/actions/runs` | `listAdminWorkflowRuns` |
| GET | `/admin/cron` | `adminCronList` |
| GET | `/admin/emails/search` | `adminSearchEmails` |
| GET | `/admin/emails` | `adminGetAllEmails` |
| GET | `/admin/hooks/{id}` | `adminGetHook` |
| GET | `/admin/hooks` | `adminListHooks` |
| GET | `/admin/orgs` | `adminGetAllOrgs` |
| GET | `/admin/runners/registration-token` | `adminGetRunnerRegistrationToken` |
| GET | `/admin/unadopted` | `adminUnadoptedList` |
| GET | `/admin/users/{username}/badges` | `adminListUserBadges` |
| GET | `/admin/users` | `adminSearchUsers` |
| PATCH | `/admin/hooks/{id}` | `adminEditHook` |
| PATCH | `/admin/users/{username}` | `adminEditUser` |
| POST | `/admin/actions/runners/registration-token` | `adminCreateRunnerRegistrationToken` |
| POST | `/admin/cron/{task}` | `adminCronRun` |
| POST | `/admin/hooks` | `adminCreateHook` |
| POST | `/admin/unadopted/{owner}/{repo}` | `adminAdoptRepository` |
| POST | `/admin/users/{username}/badges` | `adminAddUserBadges` |
| POST | `/admin/users/{username}/keys` | `adminCreatePublicKey` |
| POST | `/admin/users/{username}/orgs` | `adminCreateOrg` |
| POST | `/admin/users/{username}/rename` | `adminRenameUser` |
| POST | `/admin/users/{username}/repos` | `adminCreateRepo` |
| POST | `/admin/users` | `adminCreateUser` |

### issue (69)

| Method | Path | operationId |
|---|---|---|
| DELETE | `/repos/{owner}/{repo}/issues/comments/{id}/assets/{attachment_id}` | `issueDeleteIssueCommentAttachment` |
| DELETE | `/repos/{owner}/{repo}/issues/comments/{id}/reactions` | `issueDeleteCommentReaction` |
| DELETE | `/repos/{owner}/{repo}/issues/comments/{id}` | `issueDeleteComment` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/assets/{attachment_id}` | `issueDeleteIssueAttachment` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/blocks` | `issueRemoveIssueBlocking` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/comments/{id}` | `issueDeleteCommentDeprecated` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/dependencies` | `issueRemoveIssueDependencies` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/labels/{id}` | `issueRemoveLabel` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/labels` | `issueClearLabels` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/lock` | `issueUnlockIssue` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/pin` | `unpinIssue` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/reactions` | `issueDeleteIssueReaction` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/stopwatch/delete` | `issueDeleteStopWatch` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/subscriptions/{user}` | `issueDeleteSubscription` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/times/{id}` | `issueDeleteTime` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}/times` | `issueResetTime` |
| DELETE | `/repos/{owner}/{repo}/issues/{index}` | `issueDelete` |
| DELETE | `/repos/{owner}/{repo}/labels/{id}` | `issueDeleteLabel` |
| DELETE | `/repos/{owner}/{repo}/milestones/{id}` | `issueDeleteMilestone` |
| GET | `/repos/issues/search` | `issueSearchIssues` |
| GET | `/repos/{owner}/{repo}/issues/comments/{id}/assets/{attachment_id}` | `issueGetIssueCommentAttachment` |
| GET | `/repos/{owner}/{repo}/issues/comments/{id}/assets` | `issueListIssueCommentAttachments` |
| GET | `/repos/{owner}/{repo}/issues/comments/{id}/reactions` | `issueGetCommentReactions` |
| GET | `/repos/{owner}/{repo}/issues/comments/{id}` | `issueGetComment` |
| GET | `/repos/{owner}/{repo}/issues/comments` | `issueGetRepoComments` |
| GET | `/repos/{owner}/{repo}/issues/{index}/assets/{attachment_id}` | `issueGetIssueAttachment` |
| GET | `/repos/{owner}/{repo}/issues/{index}/assets` | `issueListIssueAttachments` |
| GET | `/repos/{owner}/{repo}/issues/{index}/blocks` | `issueListBlocks` |
| GET | `/repos/{owner}/{repo}/issues/{index}/comments` | `issueGetComments` |
| GET | `/repos/{owner}/{repo}/issues/{index}/dependencies` | `issueListIssueDependencies` |
| GET | `/repos/{owner}/{repo}/issues/{index}/labels` | `issueGetLabels` |
| GET | `/repos/{owner}/{repo}/issues/{index}/reactions` | `issueGetIssueReactions` |
| GET | `/repos/{owner}/{repo}/issues/{index}/subscriptions/check` | `issueCheckSubscription` |
| GET | `/repos/{owner}/{repo}/issues/{index}/subscriptions` | `issueSubscriptions` |
| GET | `/repos/{owner}/{repo}/issues/{index}/timeline` | `issueGetCommentsAndTimeline` |
| GET | `/repos/{owner}/{repo}/issues/{index}/times` | `issueTrackedTimes` |
| GET | `/repos/{owner}/{repo}/issues/{index}` | `issueGetIssue` |
| GET | `/repos/{owner}/{repo}/issues` | `issueListIssues` |
| GET | `/repos/{owner}/{repo}/labels/{id}` | `issueGetLabel` |
| GET | `/repos/{owner}/{repo}/labels` | `issueListLabels` |
| GET | `/repos/{owner}/{repo}/milestones/{id}` | `issueGetMilestone` |
| GET | `/repos/{owner}/{repo}/milestones` | `issueGetMilestonesList` |
| PATCH | `/repos/{owner}/{repo}/issues/comments/{id}/assets/{attachment_id}` | `issueEditIssueCommentAttachment` |
| PATCH | `/repos/{owner}/{repo}/issues/comments/{id}` | `issueEditComment` |
| PATCH | `/repos/{owner}/{repo}/issues/{index}/assets/{attachment_id}` | `issueEditIssueAttachment` |
| PATCH | `/repos/{owner}/{repo}/issues/{index}/comments/{id}` | `issueEditCommentDeprecated` |
| PATCH | `/repos/{owner}/{repo}/issues/{index}/pin/{position}` | `moveIssuePin` |
| PATCH | `/repos/{owner}/{repo}/issues/{index}` | `issueEditIssue` |
| PATCH | `/repos/{owner}/{repo}/labels/{id}` | `issueEditLabel` |
| PATCH | `/repos/{owner}/{repo}/milestones/{id}` | `issueEditMilestone` |
| POST | `/repos/{owner}/{repo}/issues/comments/{id}/assets` | `issueCreateIssueCommentAttachment` |
| POST | `/repos/{owner}/{repo}/issues/comments/{id}/reactions` | `issuePostCommentReaction` |
| POST | `/repos/{owner}/{repo}/issues/{index}/assets` | `issueCreateIssueAttachment` |
| POST | `/repos/{owner}/{repo}/issues/{index}/blocks` | `issueCreateIssueBlocking` |
| POST | `/repos/{owner}/{repo}/issues/{index}/comments` | `issueCreateComment` |
| POST | `/repos/{owner}/{repo}/issues/{index}/deadline` | `issueEditIssueDeadline` |
| POST | `/repos/{owner}/{repo}/issues/{index}/dependencies` | `issueCreateIssueDependencies` |
| POST | `/repos/{owner}/{repo}/issues/{index}/labels` | `issueAddLabel` |
| POST | `/repos/{owner}/{repo}/issues/{index}/pin` | `pinIssue` |
| POST | `/repos/{owner}/{repo}/issues/{index}/reactions` | `issuePostIssueReaction` |
| POST | `/repos/{owner}/{repo}/issues/{index}/stopwatch/start` | `issueStartStopWatch` |
| POST | `/repos/{owner}/{repo}/issues/{index}/stopwatch/stop` | `issueStopStopWatch` |
| POST | `/repos/{owner}/{repo}/issues/{index}/times` | `issueAddTime` |
| POST | `/repos/{owner}/{repo}/issues` | `issueCreateIssue` |
| POST | `/repos/{owner}/{repo}/labels` | `issueCreateLabel` |
| POST | `/repos/{owner}/{repo}/milestones` | `issueCreateMilestone` |
| PUT | `/repos/{owner}/{repo}/issues/{index}/labels` | `issueReplaceLabels` |
| PUT | `/repos/{owner}/{repo}/issues/{index}/lock` | `issueLockIssue` |
| PUT | `/repos/{owner}/{repo}/issues/{index}/subscriptions/{user}` | `issueAddSubscription` |

### miscellaneous (13)

| Method | Path | operationId |
|---|---|---|
| GET | `/gitignore/templates/{name}` | `getGitignoreTemplateInfo` |
| GET | `/gitignore/templates` | `listGitignoresTemplates` |
| GET | `/label/templates/{name}` | `getLabelTemplateInfo` |
| GET | `/label/templates` | `listLabelTemplates` |
| GET | `/licenses/{name}` | `getLicenseTemplateInfo` |
| GET | `/licenses` | `listLicenseTemplates` |
| GET | `/nodeinfo` | `getNodeInfo` |
| GET | `/signing-key.gpg` | `getSigningKey` |
| GET | `/signing-key.pub` | `getSigningKeySSH` |
| GET | `/version` | `getVersion` |
| POST | `/markdown/raw` | `renderMarkdownRaw` |
| POST | `/markdown` | `renderMarkdown` |
| POST | `/markup` | `renderMarkup` |

### notification (7)

| Method | Path | operationId |
|---|---|---|
| GET | `/notifications/new` | `notifyNewAvailable` |
| GET | `/notifications/threads/{id}` | `notifyGetThread` |
| GET | `/notifications` | `notifyGetList` |
| GET | `/repos/{owner}/{repo}/notifications` | `notifyGetRepoList` |
| PATCH | `/notifications/threads/{id}` | `notifyReadThread` |
| PUT | `/notifications` | `notifyReadList` |
| PUT | `/repos/{owner}/{repo}/notifications` | `notifyReadRepoList` |

### organization (66)

| Method | Path | operationId |
|---|---|---|
| DELETE | `/orgs/{org}/actions/runners/{runner_id}` | `deleteOrgRunner` |
| DELETE | `/orgs/{org}/actions/secrets/{secretname}` | `deleteOrgSecret` |
| DELETE | `/orgs/{org}/actions/variables/{variablename}` | `deleteOrgVariable` |
| DELETE | `/orgs/{org}/avatar` | `orgDeleteAvatar` |
| DELETE | `/orgs/{org}/blocks/{username}` | `organizationUnblockUser` |
| DELETE | `/orgs/{org}/hooks/{id}` | `orgDeleteHook` |
| DELETE | `/orgs/{org}/labels/{id}` | `orgDeleteLabel` |
| DELETE | `/orgs/{org}/members/{username}` | `orgDeleteMember` |
| DELETE | `/orgs/{org}/public_members/{username}` | `orgConcealMember` |
| DELETE | `/orgs/{org}` | `orgDelete` |
| DELETE | `/teams/{id}/members/{username}` | `orgRemoveTeamMember` |
| DELETE | `/teams/{id}/repos/{org}/{repo}` | `orgRemoveTeamRepository` |
| DELETE | `/teams/{id}` | `orgDeleteTeam` |
| GET | `/orgs/{org}/actions/jobs` | `getOrgWorkflowJobs` |
| GET | `/orgs/{org}/actions/runners/registration-token` | `orgGetRunnerRegistrationToken` |
| GET | `/orgs/{org}/actions/runners/{runner_id}` | `getOrgRunner` |
| GET | `/orgs/{org}/actions/runners` | `getOrgRunners` |
| GET | `/orgs/{org}/actions/runs` | `getOrgWorkflowRuns` |
| GET | `/orgs/{org}/actions/secrets` | `orgListActionsSecrets` |
| GET | `/orgs/{org}/actions/variables/{variablename}` | `getOrgVariable` |
| GET | `/orgs/{org}/actions/variables` | `getOrgVariablesList` |
| GET | `/orgs/{org}/activities/feeds` | `orgListActivityFeeds` |
| GET | `/orgs/{org}/blocks/{username}` | `organizationCheckUserBlock` |
| GET | `/orgs/{org}/blocks` | `organizationListBlocks` |
| GET | `/orgs/{org}/hooks/{id}` | `orgGetHook` |
| GET | `/orgs/{org}/hooks` | `orgListHooks` |
| GET | `/orgs/{org}/labels/{id}` | `orgGetLabel` |
| GET | `/orgs/{org}/labels` | `orgListLabels` |
| GET | `/orgs/{org}/members/{username}` | `orgIsMember` |
| GET | `/orgs/{org}/members` | `orgListMembers` |
| GET | `/orgs/{org}/public_members/{username}` | `orgIsPublicMember` |
| GET | `/orgs/{org}/public_members` | `orgListPublicMembers` |
| GET | `/orgs/{org}/repos` | `orgListRepos` |
| GET | `/orgs/{org}/teams/search` | `teamSearch` |
| GET | `/orgs/{org}/teams` | `orgListTeams` |
| GET | `/orgs/{org}` | `orgGet` |
| GET | `/orgs` | `orgGetAll` |
| GET | `/teams/{id}/activities/feeds` | `orgListTeamActivityFeeds` |
| GET | `/teams/{id}/members/{username}` | `orgListTeamMember` |
| GET | `/teams/{id}/members` | `orgListTeamMembers` |
| GET | `/teams/{id}/repos/{org}/{repo}` | `orgListTeamRepo` |
| GET | `/teams/{id}/repos` | `orgListTeamRepos` |
| GET | `/teams/{id}` | `orgGetTeam` |
| GET | `/user/orgs` | `orgListCurrentUserOrgs` |
| GET | `/users/{username}/orgs/{org}/permissions` | `orgGetUserPermissions` |
| GET | `/users/{username}/orgs` | `orgListUserOrgs` |
| PATCH | `/orgs/{org}/hooks/{id}` | `orgEditHook` |
| PATCH | `/orgs/{org}/labels/{id}` | `orgEditLabel` |
| PATCH | `/orgs/{org}` | `orgEdit` |
| PATCH | `/teams/{id}` | `orgEditTeam` |
| POST | `/org/{org}/repos` | `createOrgRepoDeprecated` |
| POST | `/orgs/{org}/actions/runners/registration-token` | `orgCreateRunnerRegistrationToken` |
| POST | `/orgs/{org}/actions/variables/{variablename}` | `createOrgVariable` |
| POST | `/orgs/{org}/avatar` | `orgUpdateAvatar` |
| POST | `/orgs/{org}/hooks` | `orgCreateHook` |
| POST | `/orgs/{org}/labels` | `orgCreateLabel` |
| POST | `/orgs/{org}/rename` | `renameOrg` |
| POST | `/orgs/{org}/repos` | `createOrgRepo` |
| POST | `/orgs/{org}/teams` | `orgCreateTeam` |
| POST | `/orgs` | `orgCreate` |
| PUT | `/orgs/{org}/actions/secrets/{secretname}` | `updateOrgSecret` |
| PUT | `/orgs/{org}/actions/variables/{variablename}` | `updateOrgVariable` |
| PUT | `/orgs/{org}/blocks/{username}` | `organizationBlockUser` |
| PUT | `/orgs/{org}/public_members/{username}` | `orgPublicizeMember` |
| PUT | `/teams/{id}/members/{username}` | `orgAddTeamMember` |
| PUT | `/teams/{id}/repos/{org}/{repo}` | `orgAddTeamRepository` |

### package (8)

| Method | Path | operationId |
|---|---|---|
| DELETE | `/packages/{owner}/{type}/{name}/{version}` | `deletePackage` |
| GET | `/packages/{owner}/{type}/{name}/-/latest` | `getLatestPackageVersion` |
| GET | `/packages/{owner}/{type}/{name}/{version}/files` | `listPackageFiles` |
| GET | `/packages/{owner}/{type}/{name}/{version}` | `getPackage` |
| GET | `/packages/{owner}/{type}/{name}` | `listPackageVersions` |
| GET | `/packages/{owner}` | `listPackages` |
| POST | `/packages/{owner}/{type}/{name}/-/link/{repo_name}` | `linkPackage` |
| POST | `/packages/{owner}/{type}/{name}/-/unlink` | `unlinkPackage` |

### repository (191)

| Method | Path | operationId |
|---|---|---|
| DELETE | `/repos/{owner}/{repo}/actions/artifacts/{artifact_id}` | `deleteArtifact` |
| DELETE | `/repos/{owner}/{repo}/actions/runners/{runner_id}` | `deleteRepoRunner` |
| DELETE | `/repos/{owner}/{repo}/actions/runs/{run}` | `deleteActionRun` |
| DELETE | `/repos/{owner}/{repo}/actions/secrets/{secretname}` | `deleteRepoSecret` |
| DELETE | `/repos/{owner}/{repo}/actions/variables/{variablename}` | `deleteRepoVariable` |
| DELETE | `/repos/{owner}/{repo}/avatar` | `repoDeleteAvatar` |
| DELETE | `/repos/{owner}/{repo}/branch_protections/{name}` | `repoDeleteBranchProtection` |
| DELETE | `/repos/{owner}/{repo}/branches/{branch}` | `repoDeleteBranch` |
| DELETE | `/repos/{owner}/{repo}/collaborators/{collaborator}` | `repoDeleteCollaborator` |
| DELETE | `/repos/{owner}/{repo}/contents/{filepath}` | `repoDeleteFile` |
| DELETE | `/repos/{owner}/{repo}/hooks/git/{id}` | `repoDeleteGitHook` |
| DELETE | `/repos/{owner}/{repo}/hooks/{id}` | `repoDeleteHook` |
| DELETE | `/repos/{owner}/{repo}/keys/{id}` | `repoDeleteKey` |
| DELETE | `/repos/{owner}/{repo}/pulls/{index}/merge` | `repoCancelScheduledAutoMerge` |
| DELETE | `/repos/{owner}/{repo}/pulls/{index}/requested_reviewers` | `repoDeletePullReviewRequests` |
| DELETE | `/repos/{owner}/{repo}/pulls/{index}/reviews/{id}` | `repoDeletePullReview` |
| DELETE | `/repos/{owner}/{repo}/push_mirrors/{name}` | `repoDeletePushMirror` |
| DELETE | `/repos/{owner}/{repo}/releases/tags/{tag}` | `repoDeleteReleaseByTag` |
| DELETE | `/repos/{owner}/{repo}/releases/{id}/assets/{attachment_id}` | `repoDeleteReleaseAttachment` |
| DELETE | `/repos/{owner}/{repo}/releases/{id}` | `repoDeleteRelease` |
| DELETE | `/repos/{owner}/{repo}/subscription` | `userCurrentDeleteSubscription` |
| DELETE | `/repos/{owner}/{repo}/tag_protections/{id}` | `repoDeleteTagProtection` |
| DELETE | `/repos/{owner}/{repo}/tags/{tag}` | `repoDeleteTag` |
| DELETE | `/repos/{owner}/{repo}/teams/{team}` | `repoDeleteTeam` |
| DELETE | `/repos/{owner}/{repo}/topics/{topic}` | `repoDeleteTopic` |
| DELETE | `/repos/{owner}/{repo}/wiki/page/{pageName}` | `repoDeleteWikiPage` |
| DELETE | `/repos/{owner}/{repo}` | `repoDelete` |
| GET | `/repos/search` | `repoSearch` |
| GET | `/repos/{owner}/{repo}/actions/artifacts/{artifact_id}/zip` | `downloadArtifact` |
| GET | `/repos/{owner}/{repo}/actions/artifacts/{artifact_id}` | `getArtifact` |
| GET | `/repos/{owner}/{repo}/actions/artifacts` | `getArtifacts` |
| GET | `/repos/{owner}/{repo}/actions/jobs/{job_id}/logs` | `downloadActionsRunJobLogs` |
| GET | `/repos/{owner}/{repo}/actions/jobs/{job_id}` | `getWorkflowJob` |
| GET | `/repos/{owner}/{repo}/actions/jobs` | `listWorkflowJobs` |
| GET | `/repos/{owner}/{repo}/actions/runners/registration-token` | `repoGetRunnerRegistrationToken` |
| GET | `/repos/{owner}/{repo}/actions/runners/{runner_id}` | `getRepoRunner` |
| GET | `/repos/{owner}/{repo}/actions/runners` | `getRepoRunners` |
| GET | `/repos/{owner}/{repo}/actions/runs/{run}/artifacts` | `getArtifactsOfRun` |
| GET | `/repos/{owner}/{repo}/actions/runs/{run}/jobs` | `listWorkflowRunJobs` |
| GET | `/repos/{owner}/{repo}/actions/runs/{run}` | `GetWorkflowRun` |
| GET | `/repos/{owner}/{repo}/actions/runs` | `getWorkflowRuns` |
| GET | `/repos/{owner}/{repo}/actions/secrets` | `repoListActionsSecrets` |
| GET | `/repos/{owner}/{repo}/actions/tasks` | `ListActionTasks` |
| GET | `/repos/{owner}/{repo}/actions/variables/{variablename}` | `getRepoVariable` |
| GET | `/repos/{owner}/{repo}/actions/variables` | `getRepoVariablesList` |
| GET | `/repos/{owner}/{repo}/actions/workflows/{workflow_id}` | `ActionsGetWorkflow` |
| GET | `/repos/{owner}/{repo}/actions/workflows` | `ActionsListRepositoryWorkflows` |
| GET | `/repos/{owner}/{repo}/activities/feeds` | `repoListActivityFeeds` |
| GET | `/repos/{owner}/{repo}/archive/{archive}` | `repoGetArchive` |
| GET | `/repos/{owner}/{repo}/assignees` | `repoGetAssignees` |
| GET | `/repos/{owner}/{repo}/branch_protections/{name}` | `repoGetBranchProtection` |
| GET | `/repos/{owner}/{repo}/branch_protections` | `repoListBranchProtection` |
| GET | `/repos/{owner}/{repo}/branches/{branch}` | `repoGetBranch` |
| GET | `/repos/{owner}/{repo}/branches` | `repoListBranches` |
| GET | `/repos/{owner}/{repo}/collaborators/{collaborator}/permission` | `repoGetRepoPermissions` |
| GET | `/repos/{owner}/{repo}/collaborators/{collaborator}` | `repoCheckCollaborator` |
| GET | `/repos/{owner}/{repo}/collaborators` | `repoListCollaborators` |
| GET | `/repos/{owner}/{repo}/commits/{ref}/status` | `repoGetCombinedStatusByRef` |
| GET | `/repos/{owner}/{repo}/commits/{ref}/statuses` | `repoListStatusesByRef` |
| GET | `/repos/{owner}/{repo}/commits/{sha}/pull` | `repoGetCommitPullRequest` |
| GET | `/repos/{owner}/{repo}/commits` | `repoGetAllCommits` |
| GET | `/repos/{owner}/{repo}/compare/{basehead}` | `repoCompareDiff` |
| GET | `/repos/{owner}/{repo}/contents-ext/{filepath}` | `repoGetContentsExt` |
| GET | `/repos/{owner}/{repo}/contents/{filepath}` | `repoGetContents` |
| GET | `/repos/{owner}/{repo}/contents` | `repoGetContentsList` |
| GET | `/repos/{owner}/{repo}/editorconfig/{filepath}` | `repoGetEditorConfig` |
| GET | `/repos/{owner}/{repo}/file-contents` | `repoGetFileContents` |
| GET | `/repos/{owner}/{repo}/forks` | `listForks` |
| GET | `/repos/{owner}/{repo}/git/blobs/{sha}` | `GetBlob` |
| GET | `/repos/{owner}/{repo}/git/commits/{sha}.{diffType}` | `repoDownloadCommitDiffOrPatch` |
| GET | `/repos/{owner}/{repo}/git/commits/{sha}` | `repoGetSingleCommit` |
| GET | `/repos/{owner}/{repo}/git/notes/{sha}` | `repoGetNote` |
| GET | `/repos/{owner}/{repo}/git/refs/{ref}` | `repoListGitRefs` |
| GET | `/repos/{owner}/{repo}/git/refs` | `repoListAllGitRefs` |
| GET | `/repos/{owner}/{repo}/git/tags/{sha}` | `GetAnnotatedTag` |
| GET | `/repos/{owner}/{repo}/git/trees/{sha}` | `GetTree` |
| GET | `/repos/{owner}/{repo}/hooks/git/{id}` | `repoGetGitHook` |
| GET | `/repos/{owner}/{repo}/hooks/git` | `repoListGitHooks` |
| GET | `/repos/{owner}/{repo}/hooks/{id}` | `repoGetHook` |
| GET | `/repos/{owner}/{repo}/hooks` | `repoListHooks` |
| GET | `/repos/{owner}/{repo}/issue_config/validate` | `repoValidateIssueConfig` |
| GET | `/repos/{owner}/{repo}/issue_config` | `repoGetIssueConfig` |
| GET | `/repos/{owner}/{repo}/issue_templates` | `repoGetIssueTemplates` |
| GET | `/repos/{owner}/{repo}/issues/pinned` | `repoListPinnedIssues` |
| GET | `/repos/{owner}/{repo}/keys/{id}` | `repoGetKey` |
| GET | `/repos/{owner}/{repo}/keys` | `repoListKeys` |
| GET | `/repos/{owner}/{repo}/languages` | `repoGetLanguages` |
| GET | `/repos/{owner}/{repo}/licenses` | `repoGetLicenses` |
| GET | `/repos/{owner}/{repo}/media/{filepath}` | `repoGetRawFileOrLFS` |
| GET | `/repos/{owner}/{repo}/new_pin_allowed` | `repoNewPinAllowed` |
| GET | `/repos/{owner}/{repo}/pulls/pinned` | `repoListPinnedPullRequests` |
| GET | `/repos/{owner}/{repo}/pulls/{base}/{head}` | `repoGetPullRequestByBaseHead` |
| GET | `/repos/{owner}/{repo}/pulls/{index}.{diffType}` | `repoDownloadPullDiffOrPatch` |
| GET | `/repos/{owner}/{repo}/pulls/{index}/commits` | `repoGetPullRequestCommits` |
| GET | `/repos/{owner}/{repo}/pulls/{index}/files` | `repoGetPullRequestFiles` |
| GET | `/repos/{owner}/{repo}/pulls/{index}/merge` | `repoPullRequestIsMerged` |
| GET | `/repos/{owner}/{repo}/pulls/{index}/reviews/{id}/comments` | `repoGetPullReviewComments` |
| GET | `/repos/{owner}/{repo}/pulls/{index}/reviews/{id}` | `repoGetPullReview` |
| GET | `/repos/{owner}/{repo}/pulls/{index}/reviews` | `repoListPullReviews` |
| GET | `/repos/{owner}/{repo}/pulls/{index}` | `repoGetPullRequest` |
| GET | `/repos/{owner}/{repo}/pulls` | `repoListPullRequests` |
| GET | `/repos/{owner}/{repo}/push_mirrors/{name}` | `repoGetPushMirrorByRemoteName` |
| GET | `/repos/{owner}/{repo}/push_mirrors` | `repoListPushMirrors` |
| GET | `/repos/{owner}/{repo}/raw/{filepath}` | `repoGetRawFile` |
| GET | `/repos/{owner}/{repo}/releases/latest` | `repoGetLatestRelease` |
| GET | `/repos/{owner}/{repo}/releases/tags/{tag}` | `repoGetReleaseByTag` |
| GET | `/repos/{owner}/{repo}/releases/{id}/assets/{attachment_id}` | `repoGetReleaseAttachment` |
| GET | `/repos/{owner}/{repo}/releases/{id}/assets` | `repoListReleaseAttachments` |
| GET | `/repos/{owner}/{repo}/releases/{id}` | `repoGetRelease` |
| GET | `/repos/{owner}/{repo}/releases` | `repoListReleases` |
| GET | `/repos/{owner}/{repo}/reviewers` | `repoGetReviewers` |
| GET | `/repos/{owner}/{repo}/signing-key.gpg` | `repoSigningKey` |
| GET | `/repos/{owner}/{repo}/signing-key.pub` | `repoSigningKeySSH` |
| GET | `/repos/{owner}/{repo}/stargazers` | `repoListStargazers` |
| GET | `/repos/{owner}/{repo}/statuses/{sha}` | `repoListStatuses` |
| GET | `/repos/{owner}/{repo}/subscribers` | `repoListSubscribers` |
| GET | `/repos/{owner}/{repo}/subscription` | `userCurrentCheckSubscription` |
| GET | `/repos/{owner}/{repo}/tag_protections/{id}` | `repoGetTagProtection` |
| GET | `/repos/{owner}/{repo}/tag_protections` | `repoListTagProtection` |
| GET | `/repos/{owner}/{repo}/tags/{tag}` | `repoGetTag` |
| GET | `/repos/{owner}/{repo}/tags` | `repoListTags` |
| GET | `/repos/{owner}/{repo}/teams/{team}` | `repoCheckTeam` |
| GET | `/repos/{owner}/{repo}/teams` | `repoListTeams` |
| GET | `/repos/{owner}/{repo}/times/{user}` | `userTrackedTimes` |
| GET | `/repos/{owner}/{repo}/times` | `repoTrackedTimes` |
| GET | `/repos/{owner}/{repo}/topics` | `repoListTopics` |
| GET | `/repos/{owner}/{repo}/wiki/page/{pageName}` | `repoGetWikiPage` |
| GET | `/repos/{owner}/{repo}/wiki/pages` | `repoGetWikiPages` |
| GET | `/repos/{owner}/{repo}/wiki/revisions/{pageName}` | `repoGetWikiPageRevisions` |
| GET | `/repos/{owner}/{repo}` | `repoGet` |
| GET | `/repositories/{id}` | `repoGetByID` |
| GET | `/topics/search` | `topicSearch` |
| PATCH | `/repos/{owner}/{repo}/branch_protections/{name}` | `repoEditBranchProtection` |
| PATCH | `/repos/{owner}/{repo}/branches/{branch}` | `repoRenameBranch` |
| PATCH | `/repos/{owner}/{repo}/hooks/git/{id}` | `repoEditGitHook` |
| PATCH | `/repos/{owner}/{repo}/hooks/{id}` | `repoEditHook` |
| PATCH | `/repos/{owner}/{repo}/pulls/{index}` | `repoEditPullRequest` |
| PATCH | `/repos/{owner}/{repo}/releases/{id}/assets/{attachment_id}` | `repoEditReleaseAttachment` |
| PATCH | `/repos/{owner}/{repo}/releases/{id}` | `repoEditRelease` |
| PATCH | `/repos/{owner}/{repo}/tag_protections/{id}` | `repoEditTagProtection` |
| PATCH | `/repos/{owner}/{repo}/wiki/page/{pageName}` | `repoEditWikiPage` |
| PATCH | `/repos/{owner}/{repo}` | `repoEdit` |
| POST | `/repos/migrate` | `repoMigrate` |
| POST | `/repos/{owner}/{repo}/actions/runners/registration-token` | `repoCreateRunnerRegistrationToken` |
| POST | `/repos/{owner}/{repo}/actions/variables/{variablename}` | `createRepoVariable` |
| POST | `/repos/{owner}/{repo}/actions/workflows/{workflow_id}/dispatches` | `ActionsDispatchWorkflow` |
| POST | `/repos/{owner}/{repo}/avatar` | `repoUpdateAvatar` |
| POST | `/repos/{owner}/{repo}/branch_protections/priority` | `repoUpdateBranchProtectionPriories` |
| POST | `/repos/{owner}/{repo}/branch_protections` | `repoCreateBranchProtection` |
| POST | `/repos/{owner}/{repo}/branches` | `repoCreateBranch` |
| POST | `/repos/{owner}/{repo}/contents/{filepath}` | `repoCreateFile` |
| POST | `/repos/{owner}/{repo}/contents` | `repoChangeFiles` |
| POST | `/repos/{owner}/{repo}/diffpatch` | `repoApplyDiffPatch` |
| POST | `/repos/{owner}/{repo}/file-contents` | `repoGetFileContentsPost` |
| POST | `/repos/{owner}/{repo}/forks` | `createFork` |
| POST | `/repos/{owner}/{repo}/hooks/{id}/tests` | `repoTestHook` |
| POST | `/repos/{owner}/{repo}/hooks` | `repoCreateHook` |
| POST | `/repos/{owner}/{repo}/keys` | `repoCreateKey` |
| POST | `/repos/{owner}/{repo}/merge-upstream` | `repoMergeUpstream` |
| POST | `/repos/{owner}/{repo}/mirror-sync` | `repoMirrorSync` |
| POST | `/repos/{owner}/{repo}/pulls/{index}/merge` | `repoMergePullRequest` |
| POST | `/repos/{owner}/{repo}/pulls/{index}/requested_reviewers` | `repoCreatePullReviewRequests` |
| POST | `/repos/{owner}/{repo}/pulls/{index}/reviews/{id}/dismissals` | `repoDismissPullReview` |
| POST | `/repos/{owner}/{repo}/pulls/{index}/reviews/{id}/undismissals` | `repoUnDismissPullReview` |
| POST | `/repos/{owner}/{repo}/pulls/{index}/reviews/{id}` | `repoSubmitPullReview` |
| POST | `/repos/{owner}/{repo}/pulls/{index}/reviews` | `repoCreatePullReview` |
| POST | `/repos/{owner}/{repo}/pulls/{index}/update` | `repoUpdatePullRequest` |
| POST | `/repos/{owner}/{repo}/pulls` | `repoCreatePullRequest` |
| POST | `/repos/{owner}/{repo}/push_mirrors-sync` | `repoPushMirrorSync` |
| POST | `/repos/{owner}/{repo}/push_mirrors` | `repoAddPushMirror` |
| POST | `/repos/{owner}/{repo}/releases/{id}/assets` | `repoCreateReleaseAttachment` |
| POST | `/repos/{owner}/{repo}/releases` | `repoCreateRelease` |
| POST | `/repos/{owner}/{repo}/statuses/{sha}` | `repoCreateStatus` |
| POST | `/repos/{owner}/{repo}/tag_protections` | `repoCreateTagProtection` |
| POST | `/repos/{owner}/{repo}/tags` | `repoCreateTag` |
| POST | `/repos/{owner}/{repo}/transfer/accept` | `acceptRepoTransfer` |
| POST | `/repos/{owner}/{repo}/transfer/reject` | `rejectRepoTransfer` |
| POST | `/repos/{owner}/{repo}/transfer` | `repoTransfer` |
| POST | `/repos/{owner}/{repo}/wiki/new` | `repoCreateWikiPage` |
| POST | `/repos/{template_owner}/{template_repo}/generate` | `generateRepo` |
| POST | `/user/repos` | `createCurrentUserRepo` |
| PUT | `/repos/{owner}/{repo}/actions/secrets/{secretname}` | `updateRepoSecret` |
| PUT | `/repos/{owner}/{repo}/actions/variables/{variablename}` | `updateRepoVariable` |
| PUT | `/repos/{owner}/{repo}/actions/workflows/{workflow_id}/disable` | `ActionsDisableWorkflow` |
| PUT | `/repos/{owner}/{repo}/actions/workflows/{workflow_id}/enable` | `ActionsEnableWorkflow` |
| PUT | `/repos/{owner}/{repo}/collaborators/{collaborator}` | `repoAddCollaborator` |
| PUT | `/repos/{owner}/{repo}/contents/{filepath}` | `repoUpdateFile` |
| PUT | `/repos/{owner}/{repo}/subscription` | `userCurrentPutSubscription` |
| PUT | `/repos/{owner}/{repo}/teams/{team}` | `repoAddTeam` |
| PUT | `/repos/{owner}/{repo}/topics/{topic}` | `repoAddTopic` |
| PUT | `/repos/{owner}/{repo}/topics` | `repoUpdateTopics` |

### settings (4)

| Method | Path | operationId |
|---|---|---|
| GET | `/settings/api` | `getGeneralAPISettings` |
| GET | `/settings/attachment` | `getGeneralAttachmentSettings` |
| GET | `/settings/repository` | `getGeneralRepositorySettings` |
| GET | `/settings/ui` | `getGeneralUISettings` |

### user (75)

| Method | Path | operationId |
|---|---|---|
| DELETE | `/user/actions/runners/{runner_id}` | `deleteUserRunner` |
| DELETE | `/user/actions/secrets/{secretname}` | `deleteUserSecret` |
| DELETE | `/user/actions/variables/{variablename}` | `deleteUserVariable` |
| DELETE | `/user/applications/oauth2/{id}` | `userDeleteOAuth2Application` |
| DELETE | `/user/avatar` | `userDeleteAvatar` |
| DELETE | `/user/blocks/{username}` | `userUnblockUser` |
| DELETE | `/user/emails` | `userDeleteEmail` |
| DELETE | `/user/following/{username}` | `userCurrentDeleteFollow` |
| DELETE | `/user/gpg_keys/{id}` | `userCurrentDeleteGPGKey` |
| DELETE | `/user/hooks/{id}` | `userDeleteHook` |
| DELETE | `/user/keys/{id}` | `userCurrentDeleteKey` |
| DELETE | `/user/starred/{owner}/{repo}` | `userCurrentDeleteStar` |
| DELETE | `/users/{username}/tokens/{token}` | `userDeleteAccessToken` |
| GET | `/user/actions/jobs` | `getUserWorkflowJobs` |
| GET | `/user/actions/runners/registration-token` | `userGetRunnerRegistrationToken` |
| GET | `/user/actions/runners/{runner_id}` | `getUserRunner` |
| GET | `/user/actions/runners` | `getUserRunners` |
| GET | `/user/actions/runs` | `getUserWorkflowRuns` |
| GET | `/user/actions/variables/{variablename}` | `getUserVariable` |
| GET | `/user/actions/variables` | `getUserVariablesList` |
| GET | `/user/applications/oauth2/{id}` | `userGetOAuth2Application` |
| GET | `/user/applications/oauth2` | `userGetOauth2Application` |
| GET | `/user/blocks/{username}` | `userCheckUserBlock` |
| GET | `/user/blocks` | `userListBlocks` |
| GET | `/user/emails` | `userListEmails` |
| GET | `/user/followers` | `userCurrentListFollowers` |
| GET | `/user/following/{username}` | `userCurrentCheckFollowing` |
| GET | `/user/following` | `userCurrentListFollowing` |
| GET | `/user/gpg_key_token` | `getVerificationToken` |
| GET | `/user/gpg_keys/{id}` | `userCurrentGetGPGKey` |
| GET | `/user/gpg_keys` | `userCurrentListGPGKeys` |
| GET | `/user/hooks/{id}` | `userGetHook` |
| GET | `/user/hooks` | `userListHooks` |
| GET | `/user/keys/{id}` | `userCurrentGetKey` |
| GET | `/user/keys` | `userCurrentListKeys` |
| GET | `/user/repos` | `userCurrentListRepos` |
| GET | `/user/settings` | `getUserSettings` |
| GET | `/user/starred/{owner}/{repo}` | `userCurrentCheckStarring` |
| GET | `/user/starred` | `userCurrentListStarred` |
| GET | `/user/stopwatches` | `userGetStopWatches` |
| GET | `/user/subscriptions` | `userCurrentListSubscriptions` |
| GET | `/user/teams` | `userListTeams` |
| GET | `/user/times` | `userCurrentTrackedTimes` |
| GET | `/user` | `userGetCurrent` |
| GET | `/users/search` | `userSearch` |
| GET | `/users/{username}/activities/feeds` | `userListActivityFeeds` |
| GET | `/users/{username}/followers` | `userListFollowers` |
| GET | `/users/{username}/following/{target}` | `userCheckFollowing` |
| GET | `/users/{username}/following` | `userListFollowing` |
| GET | `/users/{username}/gpg_keys` | `userListGPGKeys` |
| GET | `/users/{username}/heatmap` | `userGetHeatmapData` |
| GET | `/users/{username}/keys` | `userListKeys` |
| GET | `/users/{username}/repos` | `userListRepos` |
| GET | `/users/{username}/starred` | `userListStarred` |
| GET | `/users/{username}/subscriptions` | `userListSubscriptions` |
| GET | `/users/{username}/tokens` | `userGetTokens` |
| GET | `/users/{username}` | `userGet` |
| PATCH | `/user/applications/oauth2/{id}` | `userUpdateOAuth2Application` |
| PATCH | `/user/hooks/{id}` | `userEditHook` |
| PATCH | `/user/settings` | `updateUserSettings` |
| POST | `/user/actions/runners/registration-token` | `userCreateRunnerRegistrationToken` |
| POST | `/user/actions/variables/{variablename}` | `createUserVariable` |
| POST | `/user/applications/oauth2` | `userCreateOAuth2Application` |
| POST | `/user/avatar` | `userUpdateAvatar` |
| POST | `/user/emails` | `userAddEmail` |
| POST | `/user/gpg_key_verify` | `userVerifyGPGKey` |
| POST | `/user/gpg_keys` | `userCurrentPostGPGKey` |
| POST | `/user/hooks` | `userCreateHook` |
| POST | `/user/keys` | `userCurrentPostKey` |
| POST | `/users/{username}/tokens` | `userCreateToken` |
| PUT | `/user/actions/secrets/{secretname}` | `updateUserSecret` |
| PUT | `/user/actions/variables/{variablename}` | `updateUserVariable` |
| PUT | `/user/blocks/{username}` | `userBlockUser` |
| PUT | `/user/following/{username}` | `userCurrentPutFollow` |
| PUT | `/user/starred/{owner}/{repo}` | `userCurrentPutStar` |
