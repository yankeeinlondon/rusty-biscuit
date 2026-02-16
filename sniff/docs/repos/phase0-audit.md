# Phase 0: Schematic Endpoint Audit

**Date:** 2026-02-15
**Status:** Complete
**Skills Used:** schematic, rust

## Executive Summary

All four git host providers (GitHub, GitLab, Gitea, Bitbucket) have generated schematic-schema clients with comprehensive endpoint coverage. **GitLab is missing a GetProject endpoint** but can use existing endpoints as a mitigation. Bitbucket uses cursor-based pagination via `PaginatedResponse<T>` which is properly handled.

**Decision:** PROCEED with all 4 providers. No blocking issues.

---

## Provider Audit Results

### GitHub

**File:** `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/github.rs`
**Definition:** `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/github/mod.rs`
**Endpoints:** 14

| Required | Endpoint | Status | Notes |
|----------|----------|--------|-------|
| Get repository metadata | `GetRepository` | PRESENT | Returns `RepositoryInfo` |
| Get repository topics/tags | - | PARTIAL | Topics available via `RepositoryInfo` response |
| List repository contents | `GetGitTree`, `GetGitTreeRecursive` | PRESENT | Tree API for file discovery |
| Get file content | `GetRepositoryContentRaw` | PRESENT | Returns raw text (Accept: `application/vnd.github.raw+json`) |
| List pull requests | `ListPullRequests` | PRESENT | Returns `Vec<PullRequestSummary>` |
| List issues | `ListIssues` | PRESENT | Returns `Vec<IssueSummary>` (includes PRs) |
| List releases | `ListReleases` | PRESENT | Returns `Vec<Release>` |
| List tags | `ListTags` | PRESENT | Returns `Vec<RepoTag>` |
| Get organization info | - | MISSING | Not required for repo workflows |

**Additional Endpoints:**
- `GetGitTree` / `GetGitTreeRecursive` - Git tree access
- `ListPullRequestFiles` - PR changed files
- `GetIssue` - Single issue by number
- `ListIssueComments` - Issue comments
- `ListIssueTimeline` - Issue timeline events
- `GetTagReference` - Tag reference (commit vs annotated)
- `GetAnnotatedTag` - Annotated tag details

**Verdict:** PROCEED - All critical endpoints present

---

### GitLab

**File:** `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/gitlab.rs`
**Definition:** `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/gitlab/mod.rs`
**Endpoints:** 15

| Required | Endpoint | Status | Notes |
|----------|----------|--------|-------|
| Get project (repository) | - | **MISSING** | No `GetProject` endpoint |
| Get project topics | - | MISSING | Would be in project response |
| List repository tree | `ListRepositoryTree` | PRESENT | Returns `Vec<TreeItem>`, recursive by default |
| Get file content | `GetRepositoryFile` | PRESENT | Returns `FileContent` (Base64 encoded) |
| List merge requests | `ListMergeRequests` | PRESENT | Returns `Vec<MergeRequest>` |
| List issues | `ListIssues` | PRESENT | Returns `Vec<Issue>` |
| List releases | `ListReleases` | PRESENT | Returns `Vec<Release>` |
| List tags | `ListTags` | PRESENT | Returns `Vec<Tag>` |
| Get group info | - | MISSING | Not required for repo workflows |

**Additional Endpoints:**
- `GetMergeRequest` - Single MR by IID
- `ListMergeRequestCommits` - MR commits
- `ListMergeRequestChanges` - MR file changes/diffs
- `GetIssue` - Single issue by IID
- `ListIssueNotes` - Issue comments/notes
- `ListIssueParticipants` - Issue participants
- `GetTag` - Single tag by name
- `GetRelease` - Single release by tag name
- `GetLatestRelease` - Latest release permalink

**Mitigation for Missing GetProject:**
The sniff implementation can work without a dedicated `GetProject` endpoint:
1. Use `ListRepositoryTree` to verify project exists (returns 404 if not)
2. Extract project metadata from MR/Issue responses which include project references
3. For full project metadata, add `GetProject` endpoint to schematic-definitions in a future iteration

**Verdict:** PROCEED with mitigation - Non-blocking

---

### Gitea

**File:** `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/gitea.rs`
**Definition:** `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/gitea/mod.rs`
**Endpoints:** 14

| Required | Endpoint | Status | Notes |
|----------|----------|--------|-------|
| Get repository | `GetRepository` | PRESENT | Returns `RepositoryInfo` |
| Get repository topics | - | PARTIAL | Topics available via `RepositoryInfo` |
| List repository contents | `GetGitTree`, `GetGitTreeRecursive` | PRESENT | Tree API |
| Get file content | `GetRepositoryContentRaw` | PRESENT | Returns raw text via `/raw/{filepath}` |
| List pull requests | `ListPullRequests` | PRESENT | Returns `Vec<PullRequestSummary>` |
| List issues | `ListIssues` | PRESENT | Returns `Vec<IssueSummary>` (excludes PRs via `type=issues`) |
| List releases | `ListReleases` | PRESENT | Returns `Vec<Release>` |
| List tags | `ListTags` | PRESENT | Returns `Vec<RepoTag>` |
| Get organization info | - | MISSING | Not required for repo workflows |

**Additional Endpoints:**
- `ListPullRequestFiles` - PR changed files
- `GetIssue` - Single issue by index
- `ListIssueComments` - Issue comments
- `ListIssueTimeline` - Issue timeline events
- `GetTagReference` - Tag reference (**returns array unlike GitHub**)
- `GetAnnotatedTag` - Annotated tag details

**Gitea-Specific Notes:**
- Uses `limit=50` instead of `per_page=100`
- Issues endpoint uses `type=issues` filter to exclude PRs
- Tag reference returns `Vec<GitRef>` (array), not single object
- Releases include `draft=true&pre-release=true` by default
- Base URL is placeholder `https://gitea.example.com/api/v1` - requires configuration
- Auth uses `Authorization: token <pat>` format (include `token ` prefix)

**Verdict:** PROCEED - All critical endpoints present

---

### Bitbucket

**File:** `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/bitbucket.rs`
**Definition:** `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/bitbucket/mod.rs`
**Endpoints:** 14

| Required | Endpoint | Status | Notes |
|----------|----------|--------|-------|
| Get repository | `GetRepository` | PRESENT | Returns `Repository` |
| List repository contents | `ListDirectoryContents` | PRESENT | Returns `PaginatedResponse<SourceEntry>` |
| Get file content | `GetFileContentRaw` | PRESENT | Returns raw text (Accept: `text/plain`) |
| List pull requests | `ListPullRequests` | PRESENT | Returns `PaginatedResponse<PullRequest>` |
| List issues | `ListIssues` | PRESENT | Returns `PaginatedResponse<Issue>` |
| List tags | `ListTags` | PRESENT | Returns `PaginatedResponse<Tag>` |

**Additional Endpoints:**
- `GetPullRequest` - Single PR by ID
- `ListPullRequestComments` - PR comments
- `GetIssue` - Single issue by ID
- `ListIssueComments` - Issue comments
- `ListIssueChanges` - Issue change history
- `GetTag` - Single tag by name
- `ListDownloads` - Release artifacts
- `GetDownload` - Download file (returns `bytes::Bytes`)

**Bitbucket-Specific Notes:**
- **No releases API** - Bitbucket uses Downloads for release artifacts
- Uses `workspace` + `repo_slug` path pattern (not `owner/repo`)
- All list endpoints return `PaginatedResponse<T>` with cursor-based pagination
- Uses `pagelen=50` parameter
- Auth is Basic (username + app password)
- Issue tracker is optional and may be disabled on some repos

**Pagination Strategy:**
Bitbucket's `PaginatedResponse<T>` structure:
```rust
struct PaginatedResponse<T> {
    values: Vec<T>,
    page: Option<u32>,
    pagelen: Option<u32>,
    size: Option<u32>,
    next: Option<String>,  // URL for next page
    previous: Option<String>,
}
```

The sniff implementation should:
1. Use initial request with `pagelen=50`
2. Follow `next` URL if present
3. Handle optional `size` field for total count

**Verdict:** PROCEED - No releases but Downloads covers artifacts

---

## Missing Endpoints Summary

### Critical (None)
All providers have sufficient endpoint coverage for sniff remote workflows.

### Nice-to-Have (Defer)

| Provider | Missing Endpoint | Priority | Notes |
|----------|------------------|----------|-------|
| GitHub | GetOrganization | LOW | Not needed for repo-level workflows |
| GitLab | GetProject | MEDIUM | Mitigated via other endpoints |
| GitLab | GetGroup | LOW | Not needed for repo-level workflows |
| Gitea | GetOrganization | LOW | Not needed for repo-level workflows |
| Bitbucket | ListReleases | LOW | Use ListDownloads instead |

---

## Verification Checklist

- [x] GitHub: 14 endpoints audited
- [x] GitLab: 15 endpoints audited, GetProject missing documented
- [x] Gitea: 14 endpoints audited
- [x] Bitbucket: 14 endpoints audited, pagination strategy documented

---

## Response Type Patterns

### GitHub, Gitea
- List endpoints return `Vec<T>`
- Single-item endpoints return `T`

### GitLab
- List endpoints return `Vec<T>`
- Single-item endpoints return `T`
- File content returns Base64-encoded `FileContent` struct

### Bitbucket
- List endpoints return `PaginatedResponse<T>` (cursor-based)
- Single-item endpoints return `T`
- Pagination via `next` URL field

---

## Authentication Summary

| Provider | Strategy | Env Vars |
|----------|----------|----------|
| GitHub | Bearer Token | `GITHUB_TOKEN`, `GH_TOKEN` |
| GitLab | API Key (`PRIVATE-TOKEN` header) | `GITLAB_TOKEN`, `GITLAB_PRIVATE_TOKEN` |
| Gitea | API Key (`Authorization: token <pat>`) | `GITEA_TOKEN` |
| Bitbucket | Basic Auth | `BITBUCKET_USERNAME`, `BITBUCKET_APP_PASSWORD` |

---

## Decision Log

| Provider | Decision | Rationale |
|----------|----------|-----------|
| GitHub | PROCEED | Full endpoint coverage |
| GitLab | PROCEED | GetProject missing but mitigated via tree/MR responses |
| Gitea | PROCEED | Full endpoint coverage |
| Bitbucket | PROCEED | No releases but Downloads API sufficient |

---

## Next Steps

1. **Phase 1:** Define unified trait for remote repository operations
2. **Phase 2:** Implement GitHub provider (most complete API)
3. **Phase 3:** Implement GitLab provider with GetProject mitigation
4. **Phase 4:** Implement Gitea provider
5. **Phase 5:** Implement Bitbucket provider with pagination handling

---

## Files Examined

- `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/github.rs` (1625 lines)
- `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/gitlab.rs` (1635 lines)
- `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/gitea.rs` (1597 lines)
- `/Volumes/coding/personal/rusty-biscuit/schematic/schema/src/bitbucket.rs` (1665 lines)
- `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/github/mod.rs` (536 lines)
- `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/gitlab/mod.rs` (550 lines)
- `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/gitea/mod.rs` (557 lines)
- `/Volumes/coding/personal/rusty-biscuit/schematic/definitions/src/bitbucket/mod.rs` (601 lines)
