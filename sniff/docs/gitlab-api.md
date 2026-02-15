---
title: Gitlab REST API Research
description: Endpoint coverage, common pitfalls, and Rust examples for practical gitlab REST API usage.
prompt: |-
    Do research into the Gitlab REST API. Document all of it's API endpoints. Discuss common gotcha's developers encounter when working with this API and how they are able to get around them. Create a Markdown table (with Markdown links) of all the key resources you've found for this API. All code examples should be written in Rust. Demonstrate the how to use the API for the following use cases:

    1. Get a list of all `README.md` filepaths (case insensitive) in the repo and then provides a means to get the content of any of these readme's that the caller wants.
    2. Get a list of all PR's for the repo with associated metadata.
    3. Get a list of all Issues for the repo along with base metadata and a means to dig into further information (if this further information requires a separate call).
    4. Get a list of all Tags from the repos including all metadata; make sure you can distinguish between a normal tag and one that is deemed a "release"

    Finally near the end, add a section which compares the Github and gitlab API's in capability and approach. You can refer to the Github API's design in the [Github API](@sniff/docs/github.md).

    Write all your findings to @sniff/docs/gitlab-api.md ; replace the body of this file if it already has content but retain the frontmatter. Update the the `last_updated` frontmatter property to today's date.
last_updated: 2026-02-15
---


# GitLab REST API: A Comprehensive Developer Guide

## Table of Contents

1. [Introduction](#introduction)
2. [API Endpoints Overview](#api-endpoints-overview)
   - [Project Resources](#project-resources)
   - [Group Resources](#group-resources)
   - [Standalone Resources](#standalone-resources)
   - [Template Resources](#template-resources)
3. [Authentication](#authentication)
4. [Common Gotchas and Workarounds](#common-gotchas-and-workarounds)
5. [Key Resources](#key-resources)
6. [Rust Code Examples](#rust-code-examples)
   - [Use Case 1: Finding README.md Files](#use-case-1-finding-readmemd-files)
   - [Use Case 2: Listing Merge Requests](#use-case-2-listing-merge-requests)
   - [Use Case 3: Listing Issues](#use-case-3-listing-issues)
   - [Use Case 4: Listing Tags and Releases](#use-case-4-listing-tags-and-releases)
7. [GitLab vs GitHub API Comparison](#gitlab-vs-github-api-comparison)
8. [Conclusion](#conclusion)

---

## Introduction

The GitLab REST API provides programmatic access to GitLab resources, enabling developers to automate workflows, build integrations, and extract data for custom reporting. The API follows RESTful principles and returns JSON responses.

**Base URL:** `https://gitlab.example.com/api/v4`

**Current Version:** API v4 (all endpoints are prefixed with `/api/v4`)

---

## API Endpoints Overview

The GitLab REST API is organized into four main categories:

### Project Resources

These endpoints operate within a project context (`/projects/:id/...`):

| Resource                | Endpoint Pattern                                             |
| ----------------------- | ------------------------------------------------------------ |
| Access Requests         | `/projects/:id/access_requests`                              |
| Access Tokens           | `/projects/:id/access_tokens`                                |
| Agents (Kubernetes)     | `/projects/:id/cluster_agents`                               |
| Branches                | `/projects/:id/repository/branches/`, `/projects/:id/repository/merged_branches` |
| Commits                 | `/projects/:id/repository/commits`, `/projects/:id/statuses` |
| Container Registry      | `/projects/:id/registry/repositories`                        |
| Custom Attributes       | `/projects/:id/custom_attributes`                            |
| Deploy Keys             | `/projects/:id/deploy_keys`                                  |
| Deploy Tokens           | `/projects/:id/deploy_tokens`                                |
| Deployments             | `/projects/:id/deployments`                                  |
| Discussions             | `/projects/:id/issues/.../discussions`, `/projects/:id/merge_requests/.../discussions` |
| Environments            | `/projects/:id/environments`                                 |
| Events                  | `/projects/:id/events`                                       |
| Feature Flags           | `/projects/:id/feature_flags`                                |
| Issues                  | `/projects/:id/issues`                                       |
| Issue Boards            | `/projects/:id/boards`                                       |
| Issue Links             | `/projects/:id/issues/.../links`                             |
| Jobs                    | `/projects/:id/jobs`, `/projects/:id/pipelines/.../jobs`     |
| Job Artifacts           | `/projects/:id/jobs/:job_id/artifacts`                       |
| Labels                  | `/projects/:id/labels`                                       |
| Members                 | `/projects/:id/members`                                      |
| Merge Requests          | `/projects/:id/merge_requests`                               |
| Merge Request Approvals | `/projects/:id/approvals`, `/projects/:id/merge_requests/.../approvals` |
| Milestones              | `/projects/:id/milestones`                                   |
| Notes (Comments)        | `/projects/:id/issues/.../notes`, `/projects/:id/merge_requests/.../notes` |
| Packages                | `/projects/:id/packages`                                     |
| Pipelines               | `/projects/:id/pipelines`                                    |
| Pipeline Schedules      | `/projects/:id/pipeline_schedules`                           |
| Pipeline Triggers       | `/projects/:id/triggers`                                     |
| Projects                | `/projects`, `/projects/:id`                                 |
| Project Hooks           | `/projects/:id/hooks`                                        |
| Protected Branches      | `/projects/:id/protected_branches`                           |
| Protected Tags          | `/projects/:id/protected_tags`                               |
| Releases                | `/projects/:id/releases`                                     |
| Release Links           | `/projects/:id/releases/.../assets/links`                    |
| Repositories            | `/projects/:id/repository`                                   |
| Repository Files        | `/projects/:id/repository/files`                             |
| Repository Submodules   | `/projects/:id/repository/submodules`                        |
| Repository Tree         | `/projects/:id/repository/tree`                              |
| Runners                 | `/projects/:id/runners`                                      |
| Search                  | `/projects/:id/search`                                       |
| Snippets                | `/projects/:id/snippets`                                     |
| Tags                    | `/projects/:id/repository/tags`                              |
| Variables (CI/CD)       | `/projects/:id/variables`                                    |
| Wikis                   | `/projects/:id/wikis`                                        |

### Group Resources

These endpoints operate within a group context (`/groups/:id/...`):

| Resource          | Endpoint Pattern                      |
| ----------------- | ------------------------------------- |
| Access Requests   | `/groups/:id/access_requests`         |
| Access Tokens     | `/groups/:id/access_tokens`           |
| Custom Attributes | `/groups/:id/custom_attributes`       |
| Epics             | `/groups/:id/epics`                   |
| Epic Issues       | `/groups/:id/epics/.../issues`        |
| Epic Links        | `/groups/:id/epics/.../related_epics` |
| Groups            | `/groups`, `/groups/:id`              |
| Issue Boards      | `/groups/:id/boards`                  |
| Issues            | `/groups/:id/issues`                  |
| Issues Statistics | `/groups/:id/issues_statistics`       |
| Iterations        | `/groups/:id/iterations`              |
| Labels            | `/groups/:id/labels`                  |
| Members           | `/groups/:id/members`                 |
| Member Roles      | `/groups/:id/member_roles`            |
| Merge Requests    | `/groups/:id/merge_requests`          |
| Milestones        | `/groups/:id/milestones`              |
| Packages          | `/groups/:id/packages`                |
| Search            | `/groups/:id/search`                  |
| Shared Projects   | `/groups/:id/projects`                |
| Variables (CI/CD) | `/groups/:id/variables`               |
| Wikis             | `/groups/:id/wikis`                   |

### Standalone Resources

These endpoints operate outside of project and group contexts:

| Resource                   | Endpoint                       |
| -------------------------- | ------------------------------ |
| Appearance                 | `/application/appearance`      |
| Applications               | `/applications`                |
| Audit Events               | `/audit_events`                |
| Avatar                     | `/avatar`                      |
| Broadcast Messages         | `/broadcast_messages`          |
| Code Snippets              | `/snippets`                    |
| Deploy Keys                | `/deploy_keys`                 |
| Deploy Tokens              | `/deploy_tokens`               |
| Events                     | `/events`, `/users/:id/events` |
| Feature Flags (Admin)      | `/features`                    |
| Geo Nodes                  | `/geo_nodes`                   |
| Issues (Global)            | `/issues`                      |
| Issues Statistics (Global) | `/issues_statistics`           |
| Keys                       | `/keys`                        |
| License                    | `/license`                     |
| Markdown                   | `/markdown`                    |
| Merge Requests (Global)    | `/merge_requests`              |
| Metadata                   | `/metadata`                    |
| Namespaces                 | `/namespaces`                  |
| Notification Settings      | `/notification_settings`       |
| Personal Access Tokens     | `/personal_access_tokens`      |
| Plan Limits                | `/application/plan_limits`     |
| Projects (Global)          | `/projects`                    |
| Runners (Global)           | `/runners`                     |
| Search (Global)            | `/search`                      |
| Settings                   | `/application/settings`        |
| Sidekiq Metrics            | `/sidekiq`                     |
| Statistics                 | `/application/statistics`      |
| Suggestions                | `/suggestions`                 |
| System Hooks               | `/hooks`                       |
| To-Dos                     | `/todos`                       |
| Token Information          | `/admin/token`                 |
| Topics                     | `/topics`                      |
| Users                      | `/users`                       |
| Version                    | `/version`                     |
| Web Commits                | `/web_commits/public_key`      |

### Template Resources

These endpoints provide access to various templates:

| Resource                    | Endpoint                    |
| --------------------------- | --------------------------- |
| Dockerfile Templates        | `/templates/dockerfiles`    |
| .gitignore Templates        | `/templates/gitignores`     |
| GitLab CI/CD YAML Templates | `/templates/gitlab_ci_ymls` |
| License Templates           | `/templates/licenses`       |

---

## Authentication

GitLab supports multiple authentication methods:

### 1. Personal Access Tokens (Recommended)

Create tokens via **User Settings > Personal Access Tokens**.

```bash
curl --header "PRIVATE-TOKEN: <your_access_token>" \
  --url "https://gitlab.example.com/api/v4/projects"
```

**Available Scopes:**

| Scope              | Access                                   |
| ------------------ | ---------------------------------------- |
| `api`              | Complete read/write access to the API    |
| `read_api`         | Read-only access to the API              |
| `read_user`        | Read access to user profile              |
| `read_repository`  | Read access to repositories              |
| `write_repository` | Read-write access to repositories        |
| `read_registry`    | Read access to container registry        |
| `write_registry`   | Write access to container registry       |
| `sudo`             | Perform actions as any user (admin only) |

### 2. OAuth 2.0 Tokens

```bash
# Using Authorization header
curl --header "Authorization: Bearer <oauth_token>" \
  --url "https://gitlab.example.com/api/v4/projects"

# Using query parameter
curl --url "https://gitlab.example.com/api/v4/projects?access_token=<oauth_token>"
```

### 3. Project/Group Access Tokens

Similar to personal access tokens but scoped to a specific project or group.

### 4. CI/CD Job Tokens

Available within CI/CD jobs via the `CI_JOB_TOKEN` variable:

```bash
curl --header "JOB-TOKEN: $CI_JOB_TOKEN" \
  --url "https://gitlab.example.com/api/v4/projects/1/releases"
```

---

## Common Gotchas and Workarounds

### 1. URL Encoding of Project Paths

**Gotcha:** Project paths containing `/` must be URL-encoded (`%2F`).

```
# Wrong
GET /api/v4/projects/group/project

# Correct
GET /api/v4/projects/group%2Fproject
```

**Workaround:** Always URL-encode the namespace and project name:

```rust
fn encode_path(path: &str) -> String {
    path.replace("/", "%2F")
}

let project_path = encode_path("my-group/my-project");
// Results: "my-group%2Fmy-project"
```

### 2. Reverse Proxy Decoding Issues

**Gotcha:** Some reverse proxies (Nginx, Apache) automatically decode `%2F` back to `/`, causing 404 errors.

**Workaround:** Configure your reverse proxy to preserve encoded URLs:

```nginx
# Nginx configuration
location ~ ^/api/v4 {
    proxy_pass http://gitlab-workhorse;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    # Ensure URL encoding is preserved
    proxy_intercept_errors off;
}
```

### 3. Pagination Headers Missing for Large Result Sets

**Gotcha:** For queries returning more than 10,000 records, GitLab omits `X-Total`, `X-Total-Pages`, and `rel="last"` Link headers.

**Workaround:** Use Link headers for navigation instead of relying on total counts:

```rust
// Parse Link header for pagination
fn get_next_page_url(link_header: &str) -> Option<String> {
    // Link: <url>; rel="next", <url>; rel="last"
    for part in link_header.split(',') {
        if part.contains(r#"rel="next""#) {
            let url = part.split(';').next()?.trim();
            return Some(url.trim_matches(|c| c == '<' || c == '>').to_string());
        }
    }
    None
}
```

### 4. Base64 Encoded File Content

**Gotcha:** File contents from the Repository Files API are Base64 encoded.

**Workaround:** Always decode the content:

```rust
use base64::Engine;

fn decode_file_content(encoded: &str) -> Result<String, Box<dyn std::error::Error>> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    Ok(String::from_utf8(decoded)?)
}
```

### 5. Case Sensitivity in Search

**Gotcha:** Some search operations are case-sensitive by default.

**Workaround:** Normalize search terms or use case-insensitive filtering in your application code.

### 6. Rate Limiting

**Gotcha:** GitLab.com has rate limits:

- Unauthenticated: 400 requests per 10 minutes
- Authenticated: 2,000 requests per 10 minutes

**Workaround:** Implement exponential backoff:

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn with_retry<F, Fut, T>(f: F, max_retries: u32) -> Result<T, reqwest::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if retries >= max_retries {
                    return Err(e);
                }
                let delay = Duration::from_secs(2_u64.pow(retries));
                sleep(delay).await;
                retries += 1;
            }
        }
    }
}
```

### 7. Empty vs Missing Paths in Tree API

**Gotcha:** As of GitLab 17.7, requesting a non-existent path returns 404 instead of an empty array.

**Workaround:** Handle 404 responses gracefully:

```rust
match response.status() {
    404 => Ok(vec![]), // Return empty list for missing paths
    200 => response.json().await,
    status => Err(format!("Unexpected status: {}", status).into()),
}
```

### 8. Token Prefixes

**Gotcha:** GitLab tokens have specific prefixes that help identify their type:

| Token Type               | Prefix   |
| ------------------------ | -------- |
| Personal Access Token    | `glpat-` |
| OAuth Application Secret | `gloas-` |
| Deploy Token             | `gldt-`  |
| CI/CD Job Token          | `glcbt-` |
| Runner Auth Token        | `glrt-`  |
| Trigger Token            | `glptt-` |

---

## Key Resources

| Resource                        | Description                         | Link                                                         |
| ------------------------------- | ----------------------------------- | ------------------------------------------------------------ |
| Official REST API Documentation | Complete API reference              | [docs.gitlab.com/api/rest](https://docs.gitlab.com/api/rest/) |
| API Resources List              | All available endpoints by category | [docs.gitlab.com/api/api_resources](https://docs.gitlab.com/api/api_resources/) |
| Authentication Guide            | Detailed authentication options     | [docs.gitlab.com/api/rest/authentication](https://docs.gitlab.com/api/rest/authentication/) |
| Personal Access Tokens          | Creating and managing PATs          | [docs.gitlab.com/user/profile/personal_access_tokens](https://docs.gitlab.com/user/profile/personal_access_tokens/) |
| OAuth 2.0 Provider              | OAuth integration guide             | [docs.gitlab.com/api/oauth2](https://docs.gitlab.com/api/oauth2/) |
| Pagination Documentation        | Offset and keyset pagination        | [docs.gitlab.com/api/rest/#pagination](https://docs.gitlab.com/api/rest/#pagination) |
| Rate Limits                     | Understanding rate limiting         | [docs.gitlab.com/security/rate_limits](https://docs.gitlab.com/security/rate_limits/) |
| API Deprecations                | Upcoming API changes                | [docs.gitlab.com/api/rest/deprecations](https://docs.gitlab.com/api/rest/deprecations/) |
| OpenAPI Specification           | Machine-readable API spec           | [docs.gitlab.com/api/openapi](https://docs.gitlab.com/api/openapi/) |
| GraphQL API                     | Alternative to REST API             | [docs.gitlab.com/api/graphql](https://docs.gitlab.com/api/graphql/) |
| python-gitlab Library           | Official Python client              | [python-gitlab.readthedocs.io](https://python-gitlab.readthedocs.io/) |

---

## Rust Code Examples

### Prerequisites

Add these dependencies to your `Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"
urlencoding = "2.1"
```

### Client Setup

```rust
use reqwest::{Client, header::HeaderMap};
use std::error::Error;

pub struct GitLabClient {
    client: Client,
    base_url: String,
    token: String,
}

impl GitLabClient {
    pub fn new(base_url: &str, token: &str) -> Result<Self, Box<dyn Error>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "PRIVATE-TOKEN",
            token.parse()?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
        })
    }

    fn encode_path(&self, path: &str) -> String {
        path.replace("/", "%2F")
    }
}
```

---

### Use Case 1: Finding README.md Files

This example demonstrates how to recursively find all `README.md` files (case-insensitive) in a repository and provide a method to fetch their content.

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TreeItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String, // "blob" for files, "tree" for directories
    pub path: String,
    pub mode: String,
}

#[derive(Debug, Deserialize)]
pub struct FileContent {
    pub file_name: String,
    pub file_path: String,
    pub size: u64,
    pub encoding: String,
    pub content: String,
    pub ref: String,
    pub blob_id: String,
    pub commit_id: String,
}

impl GitLabClient {
    /// Recursively list all files in a repository tree
    pub async fn list_repository_tree(
        &self,
        project_id: &str,
        path: Option<&str>,
        ref_name: Option<&str>,
        recursive: bool,
    ) -> Result<Vec<TreeItem>, Box<dyn Error>> {
        let mut all_items = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let mut url = format!(
                "{}/api/v4/projects/{}/repository/tree?per_page={}&page={}",
                self.base_url,
                self.encode_path(project_id),
                per_page,
                page
            );

            if let Some(p) = path {
                url.push_str(&format!("&path={}", urlencoding::encode(p)));
            }

            if let Some(r) = ref_name {
                url.push_str(&format!("&ref={}", r));
            }

            if recursive {
                url.push_str("&recursive=true");
            }

            let response = self.client.get(&url).send().await?;

            if response.status() == 404 {
                break; // Path not found or empty
            }

            let items: Vec<TreeItem> = response.json().await?;

            if items.is_empty() {
                break;
            }

            all_items.extend(items);
            page += 1;
        }

        Ok(all_items)
    }

    /// Find all README.md files (case-insensitive) in a repository
    pub async fn find_readme_files(
        &self,
        project_id: &str,
        ref_name: Option<&str>,
    ) -> Result<Vec<TreeItem>, Box<dyn Error>> {
        let all_files = self.list_repository_tree(project_id, None, ref_name, true).await?;

        let readme_files: Vec<TreeItem> = all_files
            .into_iter()
            .filter(|item| {
                item.item_type == "blob" &&
                item.name.to_lowercase() == "readme.md"
            })
            .collect();

        Ok(readme_files)
    }

    /// Get the content of a file from the repository
    pub async fn get_file_content(
        &self,
        project_id: &str,
        file_path: &str,
        ref_name: &str,
    ) -> Result<String, Box<dyn Error>> {
        let encoded_path = self.encode_path(file_path);
        let url = format!(
            "{}/api/v4/projects/{}/repository/files/{}?ref={}",
            self.base_url,
            self.encode_path(project_id),
            encoded_path,
            ref_name
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch file: {}", response.status()).into());
        }

        let file_data: FileContent = response.json().await?;

        // Decode base64 content
        let decoded = base64::engine::general_purpose::STANDARD.decode(&file_data.content)?;
        let content = String::from_utf8(decoded)?;

        Ok(content)
    }
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = GitLabClient::new(
        "https://gitlab.com",
        "your-personal-access-token"
    )?;

    // Find all README.md files
    let readme_files = client.find_readme_files("my-group/my-project", Some("main")).await?;

    println!("Found {} README.md files:", readme_files.len());
    for file in &readme_files {
        println!("  - {}", file.path);
    }

    // Get content of a specific README
    if let Some(first_readme) = readme_files.first() {
        let content = client.get_file_content(
            "my-group/my-project",
            &first_readme.path,
            "main"
        ).await?;
        println!("\nFirst README content (first 200 chars):");
        println!("{}", &content[..content.len().min(200)]);
    }

    Ok(())
}
```

---

### Use Case 2: Listing Merge Requests

This example shows how to fetch all merge requests (GitLab's equivalent of pull requests) with associated metadata.

```rust
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub state: String,
    pub avatar_url: Option<String>,
    pub web_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    pub id: u64,
    pub iid: u64, // Internal ID within the project
    pub project_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String, // "opened", "closed", "merged", "locked"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub target_branch: String,
    pub source_branch: String,
    pub author: User,
    pub assignee: Option<User>,
    pub assignees: Vec<User>,
    pub reviewers: Vec<User>,
    pub source_project_id: Option<u64>,
    pub target_project_id: u64,
    pub labels: Vec<String>,
    pub draft: bool,
    pub work_in_progress: bool,
    pub milestone: Option<Milestone>,
    pub merge_when_pipeline_succeeds: bool,
    pub merge_status: String,
    pub sha: String, // HEAD commit SHA
    pub merge_commit_sha: Option<String>,
    pub squash_commit_sha: Option<String>,
    pub user_notes_count: u64,
    pub upvotes: u64,
    pub downvotes: u64,
    pub web_url: String,
    pub references: References,
    pub time_stats: TimeStats,
    pub squash: bool,
    pub subscribed: bool,
    pub changes_count: Option<String>,
    pub merged_by: Option<User>,
    pub closed_by: Option<User>,
}

#[derive(Debug, Deserialize)]
pub struct Milestone {
    pub id: u64,
    pub iid: u64,
    pub project_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_date: Option<String>,
    pub start_date: Option<String>,
    pub expired: Option<bool>,
    pub web_url: String,
}

#[derive(Debug, Deserialize)]
pub struct References {
    pub short: String,
    pub relative: String,
    pub full: String,
}

#[derive(Debug, Deserialize)]
pub struct TimeStats {
    pub time_estimate: u64,
    pub total_time_spent: u64,
    pub human_time_estimate: Option<String>,
    pub human_total_time_spent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MergeRequestFilter {
    pub state: Option<String>, // "opened", "closed", "locked", "merged", "all"
    pub target_branch: Option<String>,
    pub source_branch: Option<String>,
    pub author_id: Option<u64>,
    pub assignee_id: Option<u64>,
    pub labels: Vec<String>,
    pub milestone: Option<String>,
    pub search: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub scope: Option<String>, // "created_by_me", "assigned_to_me", "reviews_for_me", "all"
    pub order_by: Option<String>, // "created_at", "updated_at", "title"
    pub sort: Option<String>, // "asc", "desc"
}

impl Default for MergeRequestFilter {
    fn default() -> Self {
        Self {
            state: Some("all".to_string()),
            target_branch: None,
            source_branch: None,
            author_id: None,
            assignee_id: None,
            labels: Vec::new(),
            milestone: None,
            search: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            scope: Some("all".to_string()),
            order_by: Some("created_at".to_string()),
            sort: Some("desc".to_string()),
        }
    }
}

impl GitLabClient {
    /// List merge requests for a project
    pub async fn list_merge_requests(
        &self,
        project_id: &str,
        filter: &MergeRequestFilter,
    ) -> Result<Vec<MergeRequest>, Box<dyn Error>> {
        let mut all_mrs = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let mut url = format!(
                "{}/api/v4/projects/{}/merge_requests?per_page={}&page={}",
                self.base_url,
                self.encode_path(project_id),
                per_page,
                page
            );

            // Apply filters
            if let Some(state) = &filter.state {
                url.push_str(&format!("&state={}", state));
            }
            if let Some(target) = &filter.target_branch {
                url.push_str(&format!("&target_branch={}", urlencoding::encode(target)));
            }
            if let Some(source) = &filter.source_branch {
                url.push_str(&format!("&source_branch={}", urlencoding::encode(source)));
            }
            if let Some(author) = filter.author_id {
                url.push_str(&format!("&author_id={}", author));
            }
            if let Some(assignee) = filter.assignee_id {
                url.push_str(&format!("&assignee_id={}", assignee));
            }
            if !filter.labels.is_empty() {
                url.push_str(&format!("&labels={}", filter.labels.join(",")));
            }
            if let Some(milestone) = &filter.milestone {
                url.push_str(&format!("&milestone={}", urlencoding::encode(milestone)));
            }
            if let Some(search) = &filter.search {
                url.push_str(&format!("&search={}", urlencoding::encode(search)));
            }
            if let Some(scope) = &filter.scope {
                url.push_str(&format!("&scope={}", scope));
            }
            if let Some(order) = &filter.order_by {
                url.push_str(&format!("&order_by={}", order));
            }
            if let Some(sort) = &filter.sort {
                url.push_str(&format!("&sort={}", sort));
            }

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(format!("Failed to fetch MRs: {}", response.status()).into());
            }

            let mrs: Vec<MergeRequest> = response.json().await?;

            if mrs.is_empty() {
                break;
            }

            all_mrs.extend(mrs);
            page += 1;
        }

        Ok(all_mrs)
    }

    /// Get detailed information about a single merge request
    pub async fn get_merge_request(
        &self,
        project_id: &str,
        mr_iid: u64,
    ) -> Result<MergeRequest, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/merge_requests/{}",
            self.base_url,
            self.encode_path(project_id),
            mr_iid
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch MR: {}", response.status()).into());
        }

        let mr: MergeRequest = response.json().await?;
        Ok(mr)
    }

    /// Get commits for a merge request
    pub async fn get_merge_request_commits(
        &self,
        project_id: &str,
        mr_iid: u64,
    ) -> Result<Vec<Commit>, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/merge_requests/{}/commits",
            self.base_url,
            self.encode_path(project_id),
            mr_iid
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch MR commits: {}", response.status()).into());
        }

        let commits: Vec<Commit> = response.json().await?;
        Ok(commits)
    }

    /// Get changes (diffs) for a merge request
    pub async fn get_merge_request_changes(
        &self,
        project_id: &str,
        mr_iid: u64,
    ) -> Result<MergeRequestChanges, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/merge_requests/{}/changes",
            self.base_url,
            self.encode_path(project_id),
            mr_iid
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch MR changes: {}", response.status()).into());
        }

        let changes: MergeRequestChanges = response.json().await?;
        Ok(changes)
    }
}

#[derive(Debug, Deserialize)]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub created_at: DateTime<Utc>,
    pub parent_ids: Vec<String>,
    pub title: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_date: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committed_date: DateTime<Utc>,
    pub web_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequestChanges {
    pub id: u64,
    pub iid: u64,
    pub project_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub target_branch: String,
    pub source_branch: String,
    pub changes: Vec<Diff>,
}

#[derive(Debug, Deserialize)]
pub struct Diff {
    pub old_path: String,
    pub new_path: String,
    pub a_mode: String,
    pub b_mode: String,
    pub diff: String,
    pub new_file: bool,
    pub renamed_file: bool,
    pub deleted_file: bool,
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = GitLabClient::new(
        "https://gitlab.com",
        "your-personal-access-token"
    )?;

    let filter = MergeRequestFilter {
        state: Some("opened".to_string()),
        scope: Some("all".to_string()),
        ..Default::default()
    };

    // List open merge requests
    let merge_requests = client.list_merge_requests("my-group/my-project", &filter).await?;

    println!("Found {} open merge requests:", merge_requests.len());
    for mr in &merge_requests {
        println!("  !{} - {} ({})", mr.iid, mr.title, mr.author.name);
        println!("     Branch: {} -> {}", mr.source_branch, mr.target_branch);
        println!("     Created: {}", mr.created_at);
        println!("     Upvotes: {}, Downvotes: {}", mr.upvotes, mr.downvotes);
        println!("     Labels: {:?}", mr.labels);
        println!();
    }

    // Get detailed info for first MR
    if let Some(first_mr) = merge_requests.first() {
        let commits = client.get_merge_request_commits("my-group/my-project", first_mr.iid).await?;
        println!("MR !{} has {} commits", first_mr.iid, commits.len());

        let changes = client.get_merge_request_changes("my-group/my-project", first_mr.iid).await?;
        println!("MR !{} has {} changed files", first_mr.iid, changes.changes.len());
    }

    Ok(())
}
```

---

### Use Case 3: Listing Issues

This example demonstrates fetching issues with base metadata and methods to retrieve additional details.

```rust
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Clone)]
pub struct Issue {
    pub id: u64,
    pub iid: u64,
    pub project_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String, // "opened", "closed"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by: Option<User>,
    pub labels: Vec<String>,
    pub milestone: Option<Milestone>,
    pub assignees: Vec<User>,
    pub author: User,
    pub assignee: Option<User>,
    pub user_notes_count: u64,
    pub upvotes: u64,
    pub downvotes: u64,
    pub due_date: Option<String>,
    pub confidential: bool,
    pub discussion_locked: Option<bool>,
    pub issue_type: String, // "issue", "incident", "test_case"
    pub web_url: String,
    pub time_stats: TimeStats,
    pub task_completion_status: TaskCompletionStatus,
    pub weight: Option<u64>,
    pub has_tasks: bool,
    pub task_status: String,
    pub _links: IssueLinks,
    pub references: References,
    pub severity: Option<String>, // For incidents
    pub moved_to_id: Option<u64>,
    pub service_desk_reply_to: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TaskCompletionStatus {
    pub count: u64,
    pub completed_count: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IssueLinks {
    #[serde(rename = "self")]
    pub self_url: String,
    pub notes: String,
    pub award_emoji: String,
    pub project: String,
    pub closed_as_duplicate_of: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IssueFilter {
    pub state: Option<String>, // "opened", "closed", "all"
    pub labels: Vec<String>,
    pub milestone: Option<String>,
    pub author_id: Option<u64>,
    pub author_username: Option<String>,
    pub assignee_id: Option<u64>,
    pub assignee_username: Option<String>,
    pub search: Option<String>,
    pub in_field: Option<String>, // "title", "description", "title,description"
    pub confidential: Option<bool>,
    pub issue_type: Option<String>, // "issue", "incident", "test_case"
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub scope: Option<String>, // "created_by_me", "assigned_to_me", "all"
    pub order_by: Option<String>, // "created_at", "updated_at", "priority", "due_date", "relative_position", "label_priority", "popularity", "weight"
    pub sort: Option<String>, // "asc", "desc"
    pub weight: Option<u64>,
}

impl Default for IssueFilter {
    fn default() -> Self {
        Self {
            state: Some("all".to_string()),
            labels: Vec::new(),
            milestone: None,
            author_id: None,
            author_username: None,
            assignee_id: None,
            assignee_username: None,
            search: None,
            in_field: None,
            confidential: None,
            issue_type: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            scope: Some("all".to_string()),
            order_by: Some("created_at".to_string()),
            sort: Some("desc".to_string()),
            weight: None,
        }
    }
}

impl GitLabClient {
    /// List issues for a project
    pub async fn list_issues(
        &self,
        project_id: &str,
        filter: &IssueFilter,
    ) -> Result<Vec<Issue>, Box<dyn Error>> {
        let mut all_issues = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let mut url = format!(
                "{}/api/v4/projects/{}/issues?per_page={}&page={}",
                self.base_url,
                self.encode_path(project_id),
                per_page,
                page
            );

            // Apply filters
            if let Some(state) = &filter.state {
                url.push_str(&format!("&state={}", state));
            }
            if !filter.labels.is_empty() {
                url.push_str(&format!("&labels={}", filter.labels.join(",")));
            }
            if let Some(milestone) = &filter.milestone {
                url.push_str(&format!("&milestone={}", urlencoding::encode(milestone)));
            }
            if let Some(author) = filter.author_id {
                url.push_str(&format!("&author_id={}", author));
            }
            if let Some(author) = &filter.author_username {
                url.push_str(&format!("&author_username={}", author));
            }
            if let Some(assignee) = filter.assignee_id {
                url.push_str(&format!("&assignee_id={}", assignee));
            }
            if let Some(assignee) = &filter.assignee_username {
                url.push_str(&format!("&assignee_username={}", assignee));
            }
            if let Some(search) = &filter.search {
                url.push_str(&format!("&search={}", urlencoding::encode(search)));
            }
            if let Some(in_field) = &filter.in_field {
                url.push_str(&format!("&in={}", in_field));
            }
            if let Some(confidential) = filter.confidential {
                url.push_str(&format!("&confidential={}", confidential));
            }
            if let Some(issue_type) = &filter.issue_type {
                url.push_str(&format!("&issue_type={}", issue_type));
            }
            if let Some(scope) = &filter.scope {
                url.push_str(&format!("&scope={}", scope));
            }
            if let Some(order) = &filter.order_by {
                url.push_str(&format!("&order_by={}", order));
            }
            if let Some(sort) = &filter.sort {
                url.push_str(&format!("&sort={}", sort));
            }
            if let Some(weight) = filter.weight {
                url.push_str(&format!("&weight={}", weight));
            }

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(format!("Failed to fetch issues: {}", response.status()).into());
            }

            let issues: Vec<Issue> = response.json().await?;

            if issues.is_empty() {
                break;
            }

            all_issues.extend(issues);
            page += 1;
        }

        Ok(all_issues)
    }

    /// Get a single issue with full details
    pub async fn get_issue(
        &self,
        project_id: &str,
        issue_iid: u64,
    ) -> Result<Issue, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues/{}",
            self.base_url,
            self.encode_path(project_id),
            issue_iid
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch issue: {}", response.status()).into());
        }

        let issue: Issue = response.json().await?;
        Ok(issue)
    }

    /// Get notes (comments) for an issue
    pub async fn get_issue_notes(
        &self,
        project_id: &str,
        issue_iid: u64,
    ) -> Result<Vec<Note>, Box<dyn Error>> {
        let mut all_notes = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let url = format!(
                "{}/api/v4/projects/{}/issues/{}/notes?per_page={}&page={}",
                self.base_url,
                self.encode_path(project_id),
                issue_iid,
                per_page,
                page
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(format!("Failed to fetch notes: {}", response.status()).into());
            }

            let notes: Vec<Note> = response.json().await?;

            if notes.is_empty() {
                break;
            }

            all_notes.extend(notes);
            page += 1;
        }

        Ok(all_notes)
    }

    /// Get related merge requests for an issue
    pub async fn get_issue_related_merge_requests(
        &self,
        project_id: &str,
        issue_iid: u64,
    ) -> Result<Vec<MergeRequest>, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues/{}/related_merge_requests",
            self.base_url,
            self.encode_path(project_id),
            issue_iid
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch related MRs: {}", response.status()).into());
        }

        let mrs: Vec<MergeRequest> = response.json().await?;
        Ok(mrs)
    }

    /// Get participants for an issue
    pub async fn get_issue_participants(
        &self,
        project_id: &str,
        issue_iid: u64,
    ) -> Result<Vec<User>, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues/{}/participants",
            self.base_url,
            self.encode_path(project_id),
            issue_iid
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch participants: {}", response.status()).into());
        }

        let participants: Vec<User> = response.json().await?;
        Ok(participants)
    }

    /// Get time tracking statistics for an issue
    pub async fn get_issue_time_stats(
        &self,
        project_id: &str,
        issue_iid: u64,
    ) -> Result<TimeStats, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues/{}/time_stats",
            self.base_url,
            self.encode_path(project_id),
            issue_iid
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch time stats: {}", response.status()).into());
        }

        let stats: TimeStats = response.json().await?;
        Ok(stats)
    }
}

#[derive(Debug, Deserialize)]
pub struct Note {
    pub id: u64,
    #[serde(rename = "type")]
    pub note_type: Option<String>,
    pub body: String,
    pub attachment: Option<String>,
    pub author: User,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub system: bool,
    pub noteable_id: u64,
    pub noteable_type: String,
    pub resolvable: bool,
    pub resolved: Option<bool>,
    pub resolved_by: Option<User>,
    pub confidential: bool,
    pub internal: bool,
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = GitLabClient::new(
        "https://gitlab.com",
        "your-personal-access-token"
    )?;

    let filter = IssueFilter {
        state: Some("opened".to_string()),
        scope: Some("all".to_string()),
        ..Default::default()
    };

    // List open issues
    let issues = client.list_issues("my-group/my-project", &filter).await?;

    println!("Found {} open issues:", issues.len());
    for issue in &issues {
        println!("  #{} - {} ({})", issue.iid, issue.title, issue.author.name);
        println!("     State: {}, Created: {}", issue.state, issue.created_at);
        println!("     Labels: {:?}", issue.labels);
        println!("     Comments: {}, Upvotes: {}", issue.user_notes_count, issue.upvotes);

        if let Some(weight) = issue.weight {
            println!("     Weight: {}", weight);
        }
        if let Some(due) = &issue.due_date {
            println!("     Due: {}", due);
        }
        println!();
    }

    // Get detailed info for first issue
    if let Some(first_issue) = issues.first() {
        // Get comments
        let notes = client.get_issue_notes("my-group/my-project", first_issue.iid).await?;
        println!("Issue #{} has {} comments", first_issue.iid, notes.len());

        // Get related MRs
        let related_mrs = client.get_issue_related_merge_requests("my-group/my-project", first_issue.iid).await?;
        println!("Issue #{} has {} related merge requests", first_issue.iid, related_mrs.len());

        // Get participants
        let participants = client.get_issue_participants("my-group/my-project", first_issue.iid).await?;
        println!("Issue #{} has {} participants", first_issue.iid, participants.len());
    }

    Ok(())
}
```

---

### Use Case 4: Listing Tags and Releases

This example shows how to fetch tags and distinguish between regular tags and release tags.

```rust
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Clone)]
pub struct Tag {
    pub name: String,
    pub message: Option<String>,
    pub target: String, // SHA that the tag points to
    pub commit: TagCommit,
    pub release: Option<TagRelease>, // None for regular tags, Some for release tags
    pub protected: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TagCommit {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub created_at: String,
    pub parent_ids: Vec<String>,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_date: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committed_date: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TagRelease {
    pub tag_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Release {
    pub name: String,
    pub tag_name: String,
    pub description: Option<String>,
    pub description_html: Option<String>,
    pub created_at: DateTime<Utc>,
    pub released_at: DateTime<Utc>,
    pub author: User,
    pub commit: ReleaseCommit,
    pub milestones: Vec<ReleaseMilestone>,
    pub commit_path: String,
    pub tag_path: String,
    pub assets: ReleaseAssets,
    pub evidences: Vec<ReleaseEvidence>,
    pub _links: ReleaseLinks,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseCommit {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub parent_ids: Vec<String>,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_date: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committed_date: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseMilestone {
    pub id: u64,
    pub iid: u64,
    pub project_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_date: Option<String>,
    pub start_date: Option<String>,
    pub web_url: String,
    pub issue_stats: Option<MilestoneIssueStats>,
    pub merge_request_stats: Option<MilestoneMergeRequestStats>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MilestoneIssueStats {
    pub total: u64,
    pub closed: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MilestoneMergeRequestStats {
    pub total: u64,
    pub closed: u64,
    pub merged: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseAssets {
    pub count: u64,
    pub sources: Vec<ReleaseSource>,
    pub links: Vec<ReleaseLink>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseSource {
    pub format: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseLink {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub external: bool,
    pub link_type: Option<String>, // "other", "runbook", "image", "package"
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseEvidence {
    pub sha: String,
    pub filepath: String,
    pub collected_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseLinks {
    #[serde(rename = "self")]
    pub self_url: String,
    pub edit_url: Option<String>,
    pub closed_issues_url: String,
    pub closed_merge_requests_url: String,
    pub merged_merge_requests_url: String,
    pub opened_issues_url: String,
    pub opened_merge_requests_url: String,
}

#[derive(Debug, Clone)]
pub struct TagFilter {
    pub search: Option<String>,
    pub order_by: Option<String>, // "name", "updated", "version"
    pub sort: Option<String>, // "asc", "desc"
}

impl Default for TagFilter {
    fn default() -> Self {
        Self {
            search: None,
            order_by: Some("updated".to_string()),
            sort: Some("desc".to_string()),
        }
    }
}

/// Enum to distinguish between regular tags and release tags
#[derive(Debug, Clone)]
pub enum TagType {
    Regular(Tag),      // A simple git tag without an associated release
    Release(Tag, Release), // A tag with an associated release
}

impl GitLabClient {
    /// List all tags for a project
    pub async fn list_tags(
        &self,
        project_id: &str,
        filter: &TagFilter,
    ) -> Result<Vec<Tag>, Box<dyn Error>> {
        let mut all_tags = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let mut url = format!(
                "{}/api/v4/projects/{}/repository/tags?per_page={}&page={}",
                self.base_url,
                self.encode_path(project_id),
                per_page,
                page
            );

            if let Some(search) = &filter.search {
                url.push_str(&format!("&search={}", urlencoding::encode(search)));
            }
            if let Some(order) = &filter.order_by {
                url.push_str(&format!("&order_by={}", order));
            }
            if let Some(sort) = &filter.sort {
                url.push_str(&format!("&sort={}", sort));
            }

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(format!("Failed to fetch tags: {}", response.status()).into());
            }

            let tags: Vec<Tag> = response.json().await?;

            if tags.is_empty() {
                break;
            }

            all_tags.extend(tags);
            page += 1;
        }

        Ok(all_tags)
    }

    /// Get a single tag
    pub async fn get_tag(
        &self,
        project_id: &str,
        tag_name: &str,
    ) -> Result<Tag, Box<dyn Error>> {
        let encoded_tag = urlencoding::encode(tag_name);
        let url = format!(
            "{}/api/v4/projects/{}/repository/tags/{}",
            self.base_url,
            self.encode_path(project_id),
            encoded_tag
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch tag: {}", response.status()).into());
        }

        let tag: Tag = response.json().await?;
        Ok(tag)
    }

    /// List all releases for a project
    pub async fn list_releases(
        &self,
        project_id: &str,
    ) -> Result<Vec<Release>, Box<dyn Error>> {
        let mut all_releases = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let url = format!(
                "{}/api/v4/projects/{}/releases?per_page={}&page={}",
                self.base_url,
                self.encode_path(project_id),
                per_page,
                page
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(format!("Failed to fetch releases: {}", response.status()).into());
            }

            let releases: Vec<Release> = response.json().await?;

            if releases.is_empty() {
                break;
            }

            all_releases.extend(releases);
            page += 1;
        }

        Ok(all_releases)
    }

    /// Get a single release by tag name
    pub async fn get_release(
        &self,
        project_id: &str,
        tag_name: &str,
    ) -> Result<Release, Box<dyn Error>> {
        let encoded_tag = urlencoding::encode(tag_name);
        let url = format!(
            "{}/api/v4/projects/{}/releases/{}",
            self.base_url,
            self.encode_path(project_id),
            encoded_tag
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch release: {}", response.status()).into());
        }

        let release: Release = response.json().await?;
        Ok(release)
    }

    /// Get the latest release
    pub async fn get_latest_release(
        &self,
        project_id: &str,
    ) -> Result<Release, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/releases/permalink/latest",
            self.base_url,
            self.encode_path(project_id)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch latest release: {}", response.status()).into());
        }

        let release: Release = response.json().await?;
        Ok(release)
    }

    /// Get all tags with their release information (if any)
    /// This method categorizes tags as either regular or release tags
    pub async fn get_tags_with_release_info(
        &self,
        project_id: &str,
        filter: &TagFilter,
    ) -> Result<Vec<TagType>, Box<dyn Error>> {
        let tags = self.list_tags(project_id, filter).await?;
        let releases = self.list_releases(project_id).await?;

        // Create a map of tag names to releases for quick lookup
        let release_map: std::collections::HashMap<String, Release> = releases
            .into_iter()
            .map(|r| (r.tag_name.clone(), r))
            .collect();

        let mut result = Vec::new();

        for tag in tags {
            if let Some(release) = release_map.get(&tag.name) {
                result.push(TagType::Release(tag, release.clone()));
            } else {
                result.push(TagType::Regular(tag));
            }
        }

        Ok(result)
    }

    /// Get upcoming releases (releases with release date in the future)
    pub async fn get_upcoming_releases(
        &self,
        project_id: &str,
    ) -> Result<Vec<Release>, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/projects/{}/releases?upcoming=true",
            self.base_url,
            self.encode_path(project_id)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch upcoming releases: {}", response.status()).into());
        }

        let releases: Vec<Release> = response.json().await?;
        Ok(releases)
    }
}

// Helper methods for working with TagType
impl TagType {
    pub fn is_release(&self) -> bool {
        matches!(self, TagType::Release(_, _))
    }

    pub fn is_regular(&self) -> bool {
        matches!(self, TagType::Regular(_))
    }

    pub fn tag_name(&self) -> &str {
        match self {
            TagType::Regular(tag) => &tag.name,
            TagType::Release(tag, _) => &tag.name,
        }
    }

    pub fn tag(&self) -> &Tag {
        match self {
            TagType::Regular(tag) => tag,
            TagType::Release(tag, _) => tag,
        }
    }

    pub fn release(&self) -> Option<&Release> {
        match self {
            TagType::Regular(_) => None,
            TagType::Release(_, release) => Some(release),
        }
    }
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = GitLabClient::new(
        "https://gitlab.com",
        "your-personal-access-token"
    )?;

    let filter = TagFilter::default();

    // Get all tags with release information
    let tags = client.get_tags_with_release_info("my-group/my-project", &filter).await?;

    let mut regular_tags = Vec::new();
    let mut release_tags = Vec::new();

    for tag in &tags {
        if tag.is_release() {
            release_tags.push(tag);
        } else {
            regular_tags.push(tag);
        }
    }

    println!("Total tags: {}", tags.len());
    println!("Regular tags: {}", regular_tags.len());
    println!("Release tags: {}", release_tags.len());
    println!();

    // Display release tags with details
    println!("=== Release Tags ===");
    for tag_type in &release_tags {
        if let TagType::Release(tag, release) = tag_type {
            println!("Tag: {}", tag.name);
            println!("  Commit: {}", &tag.commit.short_id);
            println!("  Commit Title: {}", tag.commit.title);
            println!("  Created: {}", tag.created_at);
            println!("  Protected: {}", tag.protected);
            println!("  Release Name: {}", release.name);
            println!("  Released At: {}", release.released_at);
            println!("  Author: {}", release.author.name);

            if let Some(desc) = &release.description {
                let preview = if desc.len() > 100 {
                    format!("{}...", &desc[..100])
                } else {
                    desc.clone()
                };
                println!("  Description: {}", preview);
            }

            println!("  Assets: {} sources, {} links",
                release.assets.sources.len(),
                release.assets.links.len()
            );

            for source in &release.assets.sources {
                println!("    Source: {} - {}", source.format, source.url);
            }

            for link in &release.assets.links {
                println!("    Link: {} - {}", link.name, link.url);
            }

            println!();
        }
    }

    // Display regular tags
    println!("=== Regular Tags ===");
    for tag_type in &regular_tags[..regular_tags.len().min(5)] {
        if let TagType::Regular(tag) = tag_type {
            println!("Tag: {}", tag.name);
            println!("  Commit: {}", &tag.commit.short_id);
            println!("  Created: {}", tag.created_at);
            println!("  Protected: {}", tag.protected);
            println!();
        }
    }

    if regular_tags.len() > 5 {
        println!("... and {} more regular tags", regular_tags.len() - 5);
    }

    // Get latest release
    match client.get_latest_release("my-group/my-project").await {
        Ok(latest) => {
            println!("\n=== Latest Release ===");
            println!("Name: {}", latest.name);
            println!("Tag: {}", latest.tag_name);
            println!("Released: {}", latest.released_at);
        }
        Err(e) => println!("No latest release found: {}", e),
    }

    // Get upcoming releases
    let upcoming = client.get_upcoming_releases("my-group/my-project").await?;
    if !upcoming.is_empty() {
        println!("\n=== Upcoming Releases ===");
        for release in &upcoming {
            println!("{} - {}", release.tag_name, release.released_at);
        }
    }

    Ok(())
}
```

---

## GitLab vs GitHub API Comparison

| Aspect                           | GitLab API                                        | GitHub API                        |
| -------------------------------- | ------------------------------------------------- | --------------------------------- |
| **Terminology**                  | Merge Requests                                    | Pull Requests                     |
| **Base URL Pattern**             | `/api/v4/...`                                     | `/api/v3/` or `/api/graphql`      |
| **Authentication Header**        | `PRIVATE-TOKEN`                                   | `Authorization: token ...`        |
| **OAuth Header**                 | `Authorization: Bearer ...`                       | `Authorization: Bearer ...`       |
| **Pagination**                   | Offset (page/per_page) and Keyset                 | Offset (page/per_page) and Cursor |
| **Pagination Headers**           | `X-Total`, `X-Total-Pages`, `X-Next-Page`, `Link` | `Link`, `X-RateLimit-*`           |
| **Max Items Per Page**           | 100                                               | 100                               |
| **Rate Limit (Authenticated)**   | 2,000/10 min (GitLab.com)                         | 5,000/hour                        |
| **Rate Limit (Unauthenticated)** | 400/10 min                                        | 60/hour                           |
| **Project Path Format**          | URL-encoded (`group%2Fproject`)                   | URL-encoded (`repos/owner/repo`)  |
| **File Content Encoding**        | Base64 in JSON                                    | Base64 in JSON                    |
| **Tree API**                     | `/repository/tree`                                | `/git/trees`                      |
| **Raw File Access**              | `/repository/files/:path/raw`                     | `/contents/:path`                 |
| **Release Association**          | Tags have optional `release` object               | Releases are separate from tags   |
| **Issue/IID**                    | Uses `iid` (project-scoped)                       | Uses `number` (repo-scoped)       |
| **GraphQL Support**              | Yes (growing)                                     | Yes (mature)                      |
| **Webhooks**                     | Project/Group/System hooks                        | Repository/Organization hooks     |
| **CI/CD Integration**            | Native, deeply integrated                         | GitHub Actions (separate)         |
| **Package Registry**             | Built-in (multiple formats)                       | GitHub Packages                   |
| **Container Registry**           | Built-in                                          | GitHub Container Registry         |
| **Self-Hosting**                 | Full support (CE/EE)                              | Enterprise Server only            |
| **Open Source**                  | Core is open source                               | Proprietary                       |

### Key Differences in Detail

#### 1. **Merge Requests vs Pull Requests**

GitLab uses "Merge Requests" while GitHub uses "Pull Requests". Functionally identical, but terminology differs throughout the API.

```rust
// GitLab
GET /api/v4/projects/:id/merge_requests

// GitHub
GET /repos/:owner/:repo/pulls
```

#### 2. **Project Identification**

GitLab allows both numeric IDs and URL-encoded paths. GitHub uses `owner/repo` format.

```rust
// GitLab - both work
GET /api/v4/projects/12345
GET /api/v4/projects/group%2Fproject

// GitHub
GET /repos/owner/project
```

#### 3. **Release Handling**

GitLab treats releases as optional metadata attached to tags. GitHub treats releases as separate entities.

```rust
// GitLab - check tag.release field
if tag.release.is_some() {
    // This is a release tag
}

// GitHub - separate API calls
GET /repos/:owner/:repo/releases
GET /repos/:owner/:repo/tags
```

#### 4. **Pagination Strategy**

Both support offset pagination, but GitLab also offers keyset pagination for performance on large datasets.

```rust
// GitLab offset pagination
GET /api/v4/projects/:id/issues?page=2&per_page=50

// GitLab keyset pagination (more efficient)
GET /api/v4/projects/:id/issues?pagination=keyset&per_page=50

// GitHub offset pagination
GET /repos/:owner/:repo/issues?page=2&per_page=50

// GitHub cursor pagination (GraphQL)
```

#### 5. **Internal ID (IID) vs Number**

GitLab uses `iid` (internal ID) which is scoped to the project. GitHub uses `number` which is also repo-scoped.

```rust
// GitLab issue reference
issue.iid  // e.g., #42 within the project

// GitHub issue reference
issue.number  // e.g., #42 within the repo
```

#### 6. **File Content API**

GitLab requires URL-encoding of the entire file path. GitHub uses the path as a URL segment.

```rust
// GitLab
GET /api/v4/projects/:id/repository/files/path%2Fto%2Ffile%2Emd?ref=main

// GitHub
GET /repos/:owner/:repo/contents/path/to/file.md?ref=main
```

#### 7. **Webhook Payloads**

Both have similar event types but different payload structures. GitLab includes more metadata by default.

#### 8. **CI/CD API**

GitLab's CI/CD API is more extensive since CI/CD is natively integrated. GitHub Actions API is separate and newer.

#### 9. **Error Responses**

GitLab typically returns:

```json
{ "message": "404 Not Found" }
```

GitHub typically returns:

```json
{ "message": "Not Found", "documentation_url": "..." }
```

#### 10. **Search Capabilities**

GitHub has more powerful global search APIs. GitLab search is primarily project/group scoped.

```rust
// GitLab - project scoped
GET /api/v4/projects/:id/search?search=query&scope=blobs

// GitHub - global search
GET /search/code?q=query+repo:owner/repo
```

### When to Choose Which

**Choose GitLab API when:**

- You need self-hosted/on-premises deployment
- You want integrated CI/CD pipeline management
- You're working with container registries extensively
- You need built-in security scanning APIs
- You prefer open-source solutions

**Choose GitHub API when:**

- You're working with the largest open-source community
- You need advanced code search capabilities
- You want mature GraphQL support
- You're building GitHub Apps/Actions
- You need extensive third-party integrations

---

## Conclusion

The GitLab REST API provides comprehensive access to GitLab's features, from repository management to CI/CD automation. Key takeaways:

1. **Always URL-encode project paths** to avoid 404 errors
2. **Use keyset pagination** for large datasets when available
3. **Handle missing pagination headers** for queries returning >10,000 records
4. **Decode Base64 content** from the repository files API
5. **Implement exponential backoff** for rate limit handling
6. **Check the `release` field** on tags to distinguish regular tags from releases

The GitLab API's tight integration with CI/CD, security scanning, and container registries makes it particularly powerful for DevOps workflows, while its open-core nature provides flexibility for self-hosted deployments.

---

*Document generated based on GitLab API v4 documentation as of February 2026.*
