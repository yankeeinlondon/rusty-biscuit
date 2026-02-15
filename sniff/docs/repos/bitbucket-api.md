---
title: bitbucket REST API Research
description: Endpoint coverage, common pitfalls, and Rust examples for practical bitbucket REST API usage.
prompt: |-
    Do research into the Bitbucket REST API. Document all of it's API endpoints. Discuss common gotcha's developers encounter when working with this API and how they are able to get around them. Create a Markdown table (with Markdown links) of all the key resources you've found for this API. All code examples should be written in Rust. Demonstrate the how to use the API for the following use cases:

    1. Get a list of all `README.md` filepaths (case insensitive) in the repo and then provides a means to get the content of any of these readme's that the caller wants.
    2. Get a list of all PR's for the repo with associated metadata.
    3. Get a list of all Issues for the repo along with base metadata and a means to dig into further information (if this further information requires a separate call).
    4. Get a list of all Tags from the repos including all metadata; make sure you can distinguish between a normal tag and one that is deemed a "release"

    Finally near the end, add a section which compares the Github and bitbucket API's in capability and approach. You can refer to the Github API's design in the [Github API](@sniff/docs/github.md).

    Write all your findings to @sniff/docs/bitbucket-api.md ; replace the body of this file if it already has content but retain the frontmatter. Update the the `last_updated` frontmatter property to today's date.
last_updated: 2026-02-15
model: "kimi-k2.5"
---

# Bitbucket REST API Documentation

A comprehensive guide to the Bitbucket Cloud REST API, including endpoints, authentication, common gotchas, and practical Rust code examples.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [API Endpoints Reference](#2-api-endpoints-reference)
3. [Common Gotchas and Workarounds](#3-common-gotchas-and-workarounds)
4. [Key Resources](#4-key-resources)
5. [Rust Code Examples](#5-rust-code-examples)
6. [GitHub vs Bitbucket API Comparison](#6-github-vs-bitbucket-api-comparison)

---

## 1. Introduction

The Bitbucket REST API provides programmatic access to Bitbucket Cloud resources, enabling developers to automate workflows, integrate with other tools, and build custom applications. This documentation covers the Bitbucket Cloud 2.0 API, which is the current version recommended for new integrations.

### 1.1 Base URL

```
https://api.bitbucket.org/2.0
```

All API requests must use HTTPS. The 2.0 API prefix indicates the API version.

### 1.2 Authentication Methods

Bitbucket supports several authentication mechanisms:

- **App Passwords**: Recommended for personal scripts and automation. Created in Bitbucket settings with specific scopes.
- **OAuth 2.0**: The most secure approach for third-party applications. Supports authorization code and client credentials flows.
- **Access Tokens**: Repository, project, or workspace-scoped tokens for CI/CD and server-to-server integrations.
- **Basic Authentication**: Username + App Password for development and testing only.

---

## 2. API Endpoints Reference

The Bitbucket Cloud REST API is organized into logical resource groups. Below is a comprehensive list of all major endpoints.

### 2.1 Repositories

| Method | Endpoint                                                    | Description                          |
| ------ | ----------------------------------------------------------- | ------------------------------------ |
| GET    | `/repositories/{workspace}`                                 | List all repositories in a workspace |
| GET    | `/repositories/{workspace}/{repo_slug}`                     | Get a specific repository            |
| POST   | `/repositories/{workspace}/{repo_slug}`                     | Create a new repository              |
| PUT    | `/repositories/{workspace}/{repo_slug}`                     | Update a repository                  |
| DELETE | `/repositories/{workspace}/{repo_slug}`                     | Delete a repository                  |
| GET    | `/repositories/{workspace}/{repo_slug}/forks`               | List repository forks                |
| POST   | `/repositories/{workspace}/{repo_slug}/forks`               | Fork a repository                    |
| GET    | `/repositories/{workspace}/{repo_slug}/commits`             | List commits                         |
| GET    | `/repositories/{workspace}/{repo_slug}/commit/{commit}`     | Get a specific commit                |
| GET    | `/repositories/{workspace}/{repo_slug}/src/{commit}/{path}` | Get file/directory contents          |

### 2.2 Pull Requests

| Method | Endpoint                                                     | Description            |
| ------ | ------------------------------------------------------------ | ---------------------- |
| GET    | `/repositories/{workspace}/{repo_slug}/pullrequests`         | List pull requests     |
| POST   | `/repositories/{workspace}/{repo_slug}/pullrequests`         | Create a pull request  |
| GET    | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}`    | Get a pull request     |
| PUT    | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}`    | Update a pull request  |
| POST   | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/approve` | Approve a PR           |
| DELETE | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/approve` | Unapprove a PR         |
| POST   | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/merge` | Merge a pull request   |
| POST   | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/decline` | Decline a pull request |
| GET    | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/diff` | Get PR diff            |
| GET    | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/commits` | List PR commits        |
| GET    | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments` | List PR comments       |
| POST   | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments` | Add PR comment         |
| GET    | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/statuses` | List commit statuses   |

### 2.3 Issue Tracker

| Method | Endpoint                                                     | Description              |
| ------ | ------------------------------------------------------------ | ------------------------ |
| GET    | `/repositories/{workspace}/{repo_slug}/issues`               | List issues              |
| POST   | `/repositories/{workspace}/{repo_slug}/issues`               | Create an issue          |
| GET    | `/repositories/{workspace}/{repo_slug}/issues/{id}`          | Get an issue             |
| PUT    | `/repositories/{workspace}/{repo_slug}/issues/{id}`          | Update an issue          |
| DELETE | `/repositories/{workspace}/{repo_slug}/issues/{id}`          | Delete an issue          |
| GET    | `/repositories/{workspace}/{repo_slug}/issues/{id}/comments` | List issue comments      |
| POST   | `/repositories/{workspace}/{repo_slug}/issues/{id}/comments` | Add issue comment        |
| GET    | `/repositories/{workspace}/{repo_slug}/issues/{id}/changes`  | Get issue change history |

### 2.4 Branches and Tags (Refs)

| Method | Endpoint                                                     | Description            |
| ------ | ------------------------------------------------------------ | ---------------------- |
| GET    | `/repositories/{workspace}/{repo_slug}/refs`                 | List branches and tags |
| GET    | `/repositories/{workspace}/{repo_slug}/refs/branches`        | List branches          |
| POST   | `/repositories/{workspace}/{repo_slug}/refs/branches`        | Create a branch        |
| GET    | `/repositories/{workspace}/{repo_slug}/refs/branches/{name}` | Get a branch           |
| DELETE | `/repositories/{workspace}/{repo_slug}/refs/branches/{name}` | Delete a branch        |
| GET    | `/repositories/{workspace}/{repo_slug}/refs/tags`            | List tags              |
| POST   | `/repositories/{workspace}/{repo_slug}/refs/tags`            | Create a tag           |
| GET    | `/repositories/{workspace}/{repo_slug}/refs/tags/{name}`     | Get a tag              |
| DELETE | `/repositories/{workspace}/{repo_slug}/refs/tags/{name}`     | Delete a tag           |

### 2.5 Workspaces and Projects

| Method | Endpoint                                         | Description      |
| ------ | ------------------------------------------------ | ---------------- |
| GET    | `/workspaces`                                    | List workspaces  |
| GET    | `/workspaces/{workspace}`                        | Get a workspace  |
| GET    | `/workspaces/{workspace}/projects`               | List projects    |
| POST   | `/workspaces/{workspace}/projects`               | Create a project |
| GET    | `/workspaces/{workspace}/projects/{project_key}` | Get a project    |
| PUT    | `/workspaces/{workspace}/projects/{project_key}` | Update a project |
| DELETE | `/workspaces/{workspace}/projects/{project_key}` | Delete a project |

### 2.6 Pipelines

| Method | Endpoint                                                     | Description         |
| ------ | ------------------------------------------------------------ | ------------------- |
| GET    | `/repositories/{workspace}/{repo_slug}/pipelines`            | List pipelines      |
| POST   | `/repositories/{workspace}/{repo_slug}/pipelines`            | Trigger a pipeline  |
| GET    | `/repositories/{workspace}/{repo_slug}/pipelines/{uuid}`     | Get a pipeline      |
| POST   | `/repositories/{workspace}/{repo_slug}/pipelines/{uuid}/stopPipeline` | Stop a pipeline     |
| GET    | `/repositories/{workspace}/{repo_slug}/pipelines/{uuid}/steps` | List pipeline steps |
| GET    | `/repositories/{workspace}/{repo_slug}/pipelines/{uuid}/steps/{step_uuid}/log` | Get step logs       |

### 2.7 Webhooks

| Method | Endpoint                                            | Description      |
| ------ | --------------------------------------------------- | ---------------- |
| GET    | `/repositories/{workspace}/{repo_slug}/hooks`       | List webhooks    |
| POST   | `/repositories/{workspace}/{repo_slug}/hooks`       | Create a webhook |
| GET    | `/repositories/{workspace}/{repo_slug}/hooks/{uid}` | Get a webhook    |
| PUT    | `/repositories/{workspace}/{repo_slug}/hooks/{uid}` | Update a webhook |
| DELETE | `/repositories/{workspace}/{repo_slug}/hooks/{uid}` | Delete a webhook |

### 2.8 Downloads

| Method | Endpoint                                                     | Description                |
| ------ | ------------------------------------------------------------ | -------------------------- |
| GET    | `/repositories/{workspace}/{repo_slug}/downloads`            | List download artifacts    |
| POST   | `/repositories/{workspace}/{repo_slug}/downloads`            | Upload a download artifact |
| GET    | `/repositories/{workspace}/{repo_slug}/downloads/{filename}` | Download a file            |
| DELETE | `/repositories/{workspace}/{repo_slug}/downloads/{filename}` | Delete a download          |

---

## 3. Common Gotchas and Workarounds

### 3.1 Pagination Handling

Bitbucket uses **cursor-based pagination**, not offset-based. The response includes a `next` URL that should be followed instead of calculating page offsets.

**WRONG:** Don't try to calculate pages

```rust
for page in 1..10 {
    fetch(format!("{}?page={}", url, page));
}
```

**CORRECT:** Follow the `next` URL

```rust
let mut next_url = Some(initial_url);
while let Some(url) = next_url {
    let response = fetch(&url).await?;
    next_url = response.next;
}
```

### 3.2 Rate Limiting (429 Errors)

Bitbucket imposes rate limits based on authentication method and account type. When hitting limits:

- **Implement exponential backoff**: Start with 1 second, double on each retry
- **Use different access tokens**: Rotate between multiple app passwords
- **Cache responses**: Avoid redundant API calls

```rust
async fn with_retry<T, F, Fut>(f: F) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let mut delay = Duration::from_secs(1);
    for attempt in 0..5 {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if is_rate_limit(&e) && attempt < 4 => {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### 3.3 Concurrent Request Limitations

Bitbucket has undocumented limits on concurrent requests. Making multiple simultaneous API calls can result in generic "Something went wrong" errors.

**WRONG:** Concurrent requests may fail

```rust
let futures = vec![fetch_pr(1), fetch_pr(2), fetch_pr(3)];
let results = join_all(futures).await;
```

**CORRECT:** Sequential processing

```rust
let mut results = vec![];
for id in [1, 2, 3] {
    results.push(fetch_pr(id).await?);
}
```

### 3.4 Scope Requirements

Unlike GitHub, Bitbucket scopes are granular and don't implicitly grant related permissions:

- `repository:write` does **NOT** imply `repository:read` — Must request both explicitly
- `pullrequest:read` does **NOT** give repository access — Need separate repository scope
- `repository:admin` is admin-only — Doesn't include read/write access to content

### 3.5 UUID vs Username

Bitbucket has deprecated username-based lookups for privacy. Always use UUIDs in API calls:

```rust
// DEPRECATED: Username-based lookup
/repositories/myworkspace/myrepo

// RECOMMENDED: Use workspace UUID
/repositories/{7a8b9c0d-1e2f-3a4b-5c6d-7e8f9a0b1c2d}/myrepo
```

### 3.6 Tag vs Release Distinction

Bitbucket handles releases differently than GitHub. In Bitbucket:

- **Tags are git refs** — Use `/refs/tags` endpoint
- **Releases are download artifacts** — Use `/downloads` endpoint with release metadata
- **A tag becomes a release** — When you attach files via the Downloads API

---

## 4. Key Resources

The following table provides links to essential Bitbucket API documentation and resources:

| Resource               | URL                                                          | Description                     |
| ---------------------- | ------------------------------------------------------------ | ------------------------------- |
| Official Documentation | [developer.atlassian.com/cloud/bitbucket/rest](https://developer.atlassian.com/cloud/bitbucket/rest/) | Complete API reference          |
| Authentication Guide   | [Authentication Methods](https://developer.atlassian.com/cloud/bitbucket/rest/intro/#authentication) | Auth methods and scopes         |
| Filtering & Sorting    | [Filter and Sort](https://developer.atlassian.com/cloud/bitbucket/rest/intro/#filtering) | Query language syntax           |
| Pagination Guide       | [Pagination](https://developer.atlassian.com/cloud/bitbucket/rest/intro/#pagination) | Cursor-based pagination         |
| Rate Limits            | [Rate Limit Troubleshooting](https://support.atlassian.com/bitbucket-cloud/kb/bitbucket-cloud-rate-limit-troubleshooting/) | Rate limit handling             |
| OpenAPI Spec           | [swagger.v3.json](https://dac-static.atlassian.com/cloud/bitbucket/swagger.v3.json) | Machine-readable API spec       |
| Postman Collection     | [bitbucketcloud.postman.json](https://developer.atlassian.com/cloud/bitbucket/bitbucketcloud.postman.json) | Ready-to-use Postman collection |
| Community Forum        | [Atlassian Community](https://community.atlassian.com/t5/Bitbucket-questions/bd-p/Bitbucket) | Developer community support     |
| Changelog              | [API Changelog](https://developer.atlassian.com/cloud/bitbucket/changelog/) | API changes and deprecations    |

---

## 5. Rust Code Examples

All examples use the `reqwest` crate for HTTP requests and `serde` for JSON deserialization.

### 5.1 Listing README.md Files

This example demonstrates how to find all README.md files (case-insensitive) in a repository and retrieve their contents.

```rust
use reqwest::header::{HeaderMap, AUTHORIZATION};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct SourceEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
    links: HashMap<String, Link>,
}

#[derive(Debug, Deserialize)]
struct Link {
    href: String,
}

#[derive(Debug, Deserialize)]
struct PaginatedResponse<T> {
    values: Vec<T>,
    next: Option<String>,
}

pub struct BitbucketClient {
    client: reqwest::Client,
    base_url: String,
    auth_header: String,
}

impl BitbucketClient {
    pub fn new(workspace: &str, repo_slug: &str, app_password: &str) -> Self {
        let auth = format!(
            "Basic {}",
            base64::encode(format!("{}:{}", workspace, app_password))
        );

        Self {
            client: reqwest::Client::new(),
            base_url: format!(
                "https://api.bitbucket.org/2.0/repositories/{}/{}",
                workspace, repo_slug
            ),
            auth_header: auth,
        }
    }

    /// Recursively find all README.md files (case-insensitive)
    pub async fn find_readme_files(&self, commit: &str) -> Result<Vec<SourceEntry>, reqwest::Error> {
        let mut readme_files = Vec::new();
        self.scan_directory("", commit, &mut readme_files).await?;
        Ok(readme_files)
    }

    async fn scan_directory(
        &self,
        path: &str,
        commit: &str,
        results: &mut Vec<SourceEntry>,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/src/{}/{}", self.base_url, commit, path);

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .json::<PaginatedResponse<SourceEntry>>()
            .await?;

        for entry in response.values {
            if entry.entry_type == "commit_directory" {
                // Recurse into subdirectory
                Box::pin(self.scan_directory(&entry.path, commit, results)).await?;
            } else if entry.entry_type == "commit_file" {
                // Check if filename matches README.md (case-insensitive)
                if let Some(filename) = entry.path.split('/').last() {
                    if filename.to_lowercase() == "readme.md" {
                        results.push(entry);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the raw content of a file
    pub async fn get_file_content(&self, path: &str, commit: &str) -> Result<String, reqwest::Error> {
        let url = format!("{}/src/{}/{}", self.base_url, commit, path);

        let content = self
            .client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .text()
            .await?;

        Ok(content)
    }
}

// Usage example
async fn list_and_read_readmes() -> Result<(), Box<dyn std::error::Error>> {
    let client = BitbucketClient::new(
        "myworkspace",
        "myrepo",
        "my-app-password"
    );

    // Find all README.md files
    let readmes = client.find_readme_files("main").await?;

    println!("Found {} README.md files:", readmes.len());
    for readme in &readmes {
        println!("  - {}", readme.path);
    }

    // Read a specific README
    if let Some(first_readme) = readmes.first() {
        let content = client.get_file_content(&first_readme.path, "main").await?;
        println!("\nContent of {}:\n{}", first_readme.path, content);
    }

    Ok(())
}
```

### 5.2 Listing Pull Requests

This example shows how to list all pull requests with comprehensive metadata.

```rust
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
struct PullRequest {
    id: u64,
    title: String,
    description: Option<String>,
    state: String,  // OPEN, MERGED, DECLINED, SUPERSEDED
    #[serde(rename = "created_on")]
    created_on: DateTime<Utc>,
    #[serde(rename = "updated_on")]
    updated_on: DateTime<Utc>,
    author: User,
    source: BranchRef,
    destination: BranchRef,
    #[serde(rename = "comment_count")]
    comment_count: u32,
    #[serde(rename = "task_count")]
    task_count: u32,
    #[serde(rename = "close_source_branch")]
    close_source_branch: bool,
    reviewers: Vec<Reviewer>,
    participants: Vec<Participant>,
    links: std::collections::HashMap<String, Link>,
}

#[derive(Debug, Deserialize)]
struct User {
    uuid: String,
    #[serde(rename = "display_name")]
    display_name: String,
    nickname: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BranchRef {
    branch: Branch,
    commit: Commit,
    repository: Repository,
}

#[derive(Debug, Deserialize)]
struct Branch {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Commit {
    hash: String,
    #[serde(rename = "type")]
    commit_type: String,
}

#[derive(Debug, Deserialize)]
struct Repository {
    name: String,
    #[serde(rename = "full_name")]
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct Reviewer {
    user: User,
    #[serde(rename = "approved")]
    is_approved: bool,
}

#[derive(Debug, Deserialize)]
struct Participant {
    user: User,
    role: String,  // REVIEWER, PARTICIPANT
    #[serde(rename = "approved")]
    is_approved: bool,
    #[serde(rename = "changes_requested")]
    changes_requested: bool,
}

#[derive(Debug, Deserialize)]
struct PaginatedPRs {
    values: Vec<PullRequest>,
    next: Option<String>,
}

impl BitbucketClient {
    /// List all pull requests with optional filtering
    pub async fn list_pull_requests(
        &self,
        state: Option<&str>,  // OPEN, MERGED, DECLINED, or None for all
        author: Option<&str>,
    ) -> Result<Vec<PullRequest>, reqwest::Error> {
        let mut all_prs = Vec::new();
        let mut url = format!("{}/pullrequests", self.base_url);

        // Build query parameters
        let mut params = vec![];
        if let Some(s) = state {
            params.push(format!("state={}", s));
        }
        if let Some(a) = author {
            params.push(format!("author={}", a));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        // Paginate through all results
        let mut next_url = Some(url);
        while let Some(current_url) = next_url {
            let response = self
                .client
                .get(&current_url)
                .header(AUTHORIZATION, &self.auth_header)
                .send()
                .await?
                .json::<PaginatedPRs>()
                .await?;

            all_prs.extend(response.values);
            next_url = response.next;
        }

        Ok(all_prs)
    }

    /// Get detailed information about a specific PR
    pub async fn get_pull_request(&self, pr_id: u64) -> Result<PullRequest, reqwest::Error> {
        let url = format!("{}/pullrequests/{}", self.base_url, pr_id);

        self.client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .json::<PullRequest>()
            .await
    }

    /// Get PR diff statistics
    pub async fn get_pr_diffstat(&self, pr_id: u64) -> Result<DiffStat, reqwest::Error> {
        let url = format!("{}/pullrequests/{}/diffstat", self.base_url, pr_id);

        self.client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .json::<DiffStat>()
            .await
    }
}

#[derive(Debug, Deserialize)]
struct DiffStat {
    #[serde(rename = "lines_added")]
    lines_added: u32,
    #[serde(rename = "lines_removed")]
    lines_removed: u32,
    status: String,  // added, removed, modified, renamed
}

// Usage
async fn list_all_prs() -> Result<(), Box<dyn std::error::Error>> {
    let client = BitbucketClient::new(
        "myworkspace",
        "myrepo",
        "my-app-password"
    );

    // List open PRs
    let open_prs = client.list_pull_requests(Some("OPEN"), None).await?;
    println!("Open PRs: {}", open_prs.len());

    for pr in &open_prs {
        println!("  #{}: {} by {}", pr.id, pr.title, pr.author.display_name);
        println!("     Branch: {} -> {}", pr.source.branch.name, pr.destination.branch.name);
        println!("     Comments: {}, Tasks: {}", pr.comment_count, pr.task_count);

        // Show approval status
        let approvals: Vec<_> = pr.reviewers.iter()
            .filter(|r| r.is_approved)
            .map(|r| &r.user.display_name)
            .collect();
        if !approvals.is_empty() {
            println!("     Approved by: {:?}", approvals);
        }
    }

    Ok(())
}
```

### 5.3 Listing Issues

This example demonstrates listing issues with filtering and retrieving detailed information.

```rust
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
struct Issue {
    id: u64,
    title: String,
    #[serde(rename = "type")]
    issue_type: String,  // issue, enhancement, proposal, task, bug
    priority: String,    // trivial, minor, major, critical, blocker
    state: String,       // new, open, resolved, on hold, invalid, duplicate, wontfix, closed
    #[serde(rename = "created_on")]
    created_on: DateTime<Utc>,
    #[serde(rename = "updated_on")]
    updated_on: DateTime<Utc>,
    reporter: User,
    assignee: Option<User>,
    content: Option<Content>,
    #[serde(rename = "comment_count")]
    comment_count: u32,
    votes: u32,
    watchers: u32,
    #[serde(rename = "kind")]
    kind: String,
    component: Option<String>,
    version: Option<String>,
    milestone: Option<String>,
    links: std::collections::HashMap<String, Link>,
}

#[derive(Debug, Deserialize)]
struct Content {
    raw: String,
    markup: String,
    html: String,
}

#[derive(Debug, Deserialize)]
struct PaginatedIssues {
    values: Vec<Issue>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IssueComment {
    id: u64,
    content: Content,
    user: User,
    #[serde(rename = "created_on")]
    created_on: DateTime<Utc>,
    #[serde(rename = "updated_on")]
    updated_on: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct PaginatedComments {
    values: Vec<IssueComment>,
    next: Option<String>,
}

impl BitbucketClient {
    /// List issues with optional filtering using Bitbucket's query language
    pub async fn list_issues(
        &self,
        state: Option<&str>,
        assignee: Option<&str>,
        priority: Option<&str>,
        query: Option<&str>,  // Custom Bitbucket query
    ) -> Result<Vec<Issue>, reqwest::Error> {
        let mut all_issues = Vec::new();
        let mut url = format!("{}/issues", self.base_url);

        // Build query string using Bitbucket's query language
        let mut filters = vec![];
        if let Some(s) = state {
            filters.push(format!("state=\"{}\"", s));
        }
        if let Some(a) = assignee {
            filters.push(format!("assignee.nickname=\"{}\"", a));
        }
        if let Some(p) = priority {
            filters.push(format!("priority=\"{}\"", p));
        }
        if let Some(q) = query {
            filters.push(q.to_string());
        }

        if !filters.is_empty() {
            // URL-encode the query
            let query_str = filters.join(" AND ");
            url = format!("{}?q={}", url, urlencoding::encode(&query_str));
        }

        // Paginate through results
        let mut next_url = Some(url);
        while let Some(current_url) = next_url {
            let response = self
                .client
                .get(&current_url)
                .header(AUTHORIZATION, &self.auth_header)
                .send()
                .await?
                .json::<PaginatedIssues>()
                .await?;

            all_issues.extend(response.values);
            next_url = response.next;
        }

        Ok(all_issues)
    }

    /// Get detailed information about a specific issue
    pub async fn get_issue(&self, issue_id: u64) -> Result<Issue, reqwest::Error> {
        let url = format!("{}/issues/{}", self.base_url, issue_id);

        self.client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .json::<Issue>()
            .await
    }

    /// Get comments for an issue (requires separate call)
    pub async fn get_issue_comments(&self, issue_id: u64) -> Result<Vec<IssueComment>, reqwest::Error> {
        let mut all_comments = Vec::new();
        let url = format!("{}/issues/{}/comments", self.base_url, issue_id);

        let mut next_url = Some(url);
        while let Some(current_url) = next_url {
            let response = self
                .client
                .get(&current_url)
                .header(AUTHORIZATION, &self.auth_header)
                .send()
                .await?
                .json::<PaginatedComments>()
                .await?;

            all_comments.extend(response.values);
            next_url = response.next;
        }

        Ok(all_comments)
    }

    /// Get issue change history
    pub async fn get_issue_changes(&self, issue_id: u64) -> Result<Vec<IssueChange>, reqwest::Error> {
        let url = format!("{}/issues/{}/changes", self.base_url, issue_id);

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .json::<PaginatedChanges>()
            .await?;

        Ok(response.values)
    }
}

#[derive(Debug, Deserialize)]
struct IssueChange {
    #[serde(rename = "created_on")]
    created_on: DateTime<Utc>,
    user: User,
    changes: std::collections::HashMap<String, ChangeDetail>,
}

#[derive(Debug, Deserialize)]
struct ChangeDetail {
    old: Option<String>,
    new: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaginatedChanges {
    values: Vec<IssueChange>,
}

// Usage
async fn list_and_inspect_issues() -> Result<(), Box<dyn std::error::Error>> {
    let client = BitbucketClient::new(
        "myworkspace",
        "myrepo",
        "my-app-password"
    );

    // List open high-priority issues
    let issues = client.list_issues(
        Some("open"),
        None,
        Some("major"),
        None
    ).await?;

    println!("Found {} open major issues", issues.len());

    for issue in &issues {
        println!("\n#{}: {} [{}]", issue.id, issue.title, issue.priority);
        println!("  State: {}, Type: {}", issue.state, issue.issue_type);
        println!("  Reporter: {}", issue.reporter.display_name);

        if let Some(ref assignee) = issue.assignee {
            println!("  Assignee: {}", assignee.display_name);
        }

        if issue.comment_count > 0 {
            println!("  Comments: {} (use get_issue_comments to retrieve)", issue.comment_count);
        }

        // Get full content if truncated
        if issue.content.is_none() {
            let full_issue = client.get_issue(issue.id).await?;
            if let Some(content) = full_issue.content {
                println!("  Content preview: {:.100}...", content.raw);
            }
        }
    }

    Ok(())
}
```

### 5.4 Listing Tags

This example shows how to list all tags and distinguish between regular tags and release tags.

```rust
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
struct Tag {
    name: String,
    #[serde(rename = "type")]
    tag_type: String,  // tag
    target: TagTarget,
    #[serde(rename = "created_on")]
    created_on: Option<DateTime<Utc>>,
    #[serde(rename = "updated_on")]
    updated_on: Option<DateTime<Utc>>,
    #[serde(rename = "tagger")]
    tagger: Option<Tagger>,
    message: Option<String>,
    links: std::collections::HashMap<String, Link>,
}

#[derive(Debug, Deserialize)]
struct TagTarget {
    hash: String,
    #[serde(rename = "type")]
    target_type: String,  // commit
    date: DateTime<Utc>,
    message: String,
    author: User,
}

#[derive(Debug, Deserialize)]
struct Tagger {
    raw: String,
    #[serde(rename = "display_name")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct PaginatedTags {
    values: Vec<Tag>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Download {
    name: String,
    #[serde(rename = "created_on")]
    created_on: DateTime<Utc>,
    #[serde(rename = "user")]
    uploader: User,
    size: u64,
    downloads: u64,
    links: std::collections::HashMap<String, Link>,
}

#[derive(Debug, Deserialize)]
struct PaginatedDownloads {
    values: Vec<Download>,
}

/// Represents a tag with release information
#[derive(Debug)]
struct TagWithReleaseInfo {
    tag: Tag,
    is_release: bool,
    downloads: Vec<Download>,
}

impl BitbucketClient {
    /// List all tags with release information
    pub async fn list_tags_with_releases(&self) -> Result<Vec<TagWithReleaseInfo>, reqwest::Error> {
        // First, get all downloads (releases)
        let downloads = self.list_downloads().await?;

        // Create a map of tag names to downloads
        let mut release_map: std::collections::HashMap<String, Vec<Download>> =
            std::collections::HashMap::new();

        for download in downloads {
            // Downloads associated with tags typically have the tag name
            // or are uploaded with a specific naming convention
            let tag_name = self.extract_tag_from_download(&download.name);
            release_map.entry(tag_name).or_default().push(download);
        }

        // Get all tags
        let mut all_tags = Vec::new();
        let url = format!("{}/refs/tags", self.base_url);

        let mut next_url = Some(url);
        while let Some(current_url) = next_url {
            let response = self
                .client
                .get(&current_url)
                .header(AUTHORIZATION, &self.auth_header)
                .send()
                .await?
                .json::<PaginatedTags>()
                .await?;

            for tag in response.values {
                let tag_name = tag.name.clone();
                let downloads = release_map.get(&tag_name).cloned().unwrap_or_default();
                let is_release = !downloads.is_empty();

                all_tags.push(TagWithReleaseInfo {
                    tag,
                    is_release,
                    downloads,
                });
            }

            next_url = response.next;
        }

        Ok(all_tags)
    }

    /// List only release tags (tags with downloads)
    pub async fn list_releases(&self) -> Result<Vec<TagWithReleaseInfo>, reqwest::Error> {
        let all_tags = self.list_tags_with_releases().await?;
        Ok(all_tags.into_iter().filter(|t| t.is_release).collect())
    }

    /// Get a specific tag with full metadata
    pub async fn get_tag(&self, tag_name: &str) -> Result<Tag, reqwest::Error> {
        let url = format!("{}/refs/tags/{}", self.base_url, tag_name);

        self.client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .json::<Tag>()
            .await
    }

    /// List all downloads (potential release artifacts)
    async fn list_downloads(&self) -> Result<Vec<Download>, reqwest::Error> {
        let url = format!("{}/downloads", self.base_url);

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?
            .json::<PaginatedDownloads>()
            .await?;

        Ok(response.values)
    }

    /// Extract tag name from download filename
    /// This is a heuristic - adjust based on your naming convention
    fn extract_tag_from_download(&self, filename: &str) -> String {
        // Common patterns: myapp-v1.0.0.tar.gz, release-1.0.0.zip, etc.
        if let Some(version_start) = filename.find("v") {
            let rest = &filename[version_start..];
            if let Some(end) = rest.find(|c: char| !c.is_alphanumeric() && c != '.') {
                return rest[..end].to_string();
            }
            return rest.to_string();
        }
        filename.to_string()
    }

    /// Get annotated tag message (for annotated tags)
    pub async fn get_tag_message(&self, tag_name: &str) -> Result<Option<String>, reqwest::Error> {
        let tag = self.get_tag(tag_name).await?;
        Ok(tag.message)
    }
}

// Usage
async fn list_and_categorize_tags() -> Result<(), Box<dyn std::error::Error>> {
    let client = BitbucketClient::new(
        "myworkspace",
        "myrepo",
        "my-app-password"
    );

    let tags = client.list_tags_with_releases().await?;

    let (releases, regular_tags): (Vec<_>, Vec<_>) =
        tags.into_iter().partition(|t| t.is_release);

    println!("=== Release Tags ({} total) ===", releases.len());
    for release in &releases {
        let tag = &release.tag;
        println!("\n{}:", tag.name);
        println!("  Commit: {}", &tag.target.hash[..12]);
        println!("  Author: {}", tag.target.author.display_name);
        println!("  Date: {}", tag.target.date.format("%Y-%m-%d %H:%M"));

        if let Some(ref message) = tag.message {
            println!("  Annotation: {}", message.lines().next().unwrap_or(""));
        }

        println!("  Downloads:");
        for download in &release.downloads {
            println!("    - {} ({} bytes, {} downloads)",
                download.name, download.size, download.downloads);
        }
    }

    println!("\n=== Regular Tags ({} total) ===", regular_tags.len());
    for tag_info in &regular_tags {
        let tag = &tag_info.tag;
        println!("  {} -> {}", tag.name, &tag.target.hash[..12]);
    }

    Ok(())
}
```

---

## 6. GitHub vs Bitbucket API Comparison

While both platforms provide REST APIs for repository management, there are significant differences in design philosophy, capabilities, and approach.

### 6.1 API Design Philosophy

| Aspect          | GitHub API                         | Bitbucket API                    |
| --------------- | ---------------------------------- | -------------------------------- |
| API Versions    | REST v3 (stable), GraphQL v4       | REST 2.0 (current), 1.0 (legacy) |
| Query Language  | GraphQL for flexible queries       | Custom SQL-like filtering        |
| Pagination      | Link headers, page-based           | JSON `next` field, cursor-based  |
| Rate Limit Info | Clear X-RateLimit headers          | Limited visibility               |
| Scope Hierarchy | Hierarchical (write implies read)  | Explicit (no implicit grants)    |
| Releases        | First-class release objects        | Tags + Downloads combined        |
| Issues          | Rich, organization-scoped possible | Simpler, repository-scoped only  |
| Draft PRs       | Native support                     | No native equivalent             |
| Webhook Events  | Extensive, well-documented         | Good coverage                    |

### 6.2 Key Differences

#### Authentication

- **GitHub**: Personal Access Tokens (classic and fine-grained) with repository-scoped permissions. OAuth apps and GitHub Apps with installation tokens.
- **Bitbucket**: App Passwords with explicit scopes, OAuth 2.0, and resource-scoped tokens. Scopes are **NOT** hierarchical — write does not imply read.

#### Pagination

- **GitHub**: Uses Link headers with `rel=next/prev/last/first`. Supports `per_page` and `page` parameters.
- **Bitbucket**: Uses JSON response body with `next` URL field. Cursor-based, page numbers not exposed.

#### Rate Limiting

- **GitHub**: Clear rate limit headers (`X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`). Different limits for different authentication types.
- **Bitbucket**: Undocumented limits vary by auth method. Returns 429 with limited retry guidance. Concurrent request limits exist but are not documented.

#### Releases

- **GitHub**: First-class release objects with associated tags, release notes, and assets. `/repos/{owner}/{repo}/releases` endpoint.
- **Bitbucket**: Downloads are separate from tags. A "release" is a tag with associated download artifacts. No unified release metadata endpoint.

#### Issues

- **GitHub**: Rich issue tracking with labels, milestones, assignees, projects. Issues can exist without a repository (organization-level).
- **Bitbucket**: Simpler issue tracker. Issues are repository-scoped. Labels are called "kinds" and "components" with limited customization.

#### Pull Requests

- **GitHub**: PRs are issues with code. Unified model with reviews, comments, and status checks. Draft PRs supported.
- **Bitbucket**: PRs are separate from issues. Different endpoints for comments, approvals, and tasks. No native draft PR concept.

#### GraphQL

- **GitHub**: Full GraphQL API v4 with comprehensive schema and query flexibility.
- **Bitbucket**: No GraphQL API. REST API 2.0 is the primary interface.

### 6.3 When to Choose Which

**Choose GitHub API when:**

- You need GraphQL for complex queries
- Rich release management is critical
- You need advanced issue/project tracking
- You want better API documentation and tooling

**Choose Bitbucket API when:**

- You're already in the Atlassian ecosystem (Jira, Confluence)
- You need built-in CI/CD (Bitbucket Pipelines) integration
- You want more granular permission control
- You need workspace-level access tokens

---

*Document generated: 2026-02-16*
