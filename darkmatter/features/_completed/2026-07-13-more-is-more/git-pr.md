---
hash: ef46db3751d8e999-f9aa64b1719a03d3
---
# Pull request lookup and query capability assessment

## Executive summary

Sniff currently supports four provider families:

- GitHub
- GitLab
- Gitea and Forgejo
- Bitbucket Cloud

The existing provider abstraction is a useful starting point, but it does not yet satisfy either proposed query type well.

| Goal                                                    | Current support                        | Assessment          |
|---------------------------------------------------------|----------------------------------------|---------------------|
| Retrieve one uniquely identified pull request           | No public provider or CLI operation    | Unsupported         |
| Query for a collection of pull requests                 | Repository-scoped state filtering only | Partially supported |
| Provider-native query syntax                            | None                                   | Unsupported         |
| Complete result traversal                               | One provider-default page              | Unsupported         |
| Queries spanning repositories, groups, or organizations | None                                   | Unsupported         |

The main limitations are:

- `RemoteRepoProvider` only exposes `list_pull_requests(owner, repo, state)`.
- There is no exact `get_pull_request` operation.
- There is no provider-native or structured query representation.
- The CLI only accepts `--status`.
- Pagination is not traversed or exposed.
- Results do not retain their provider, host, namespace, or repository identity.
- Several useful parameters already modeled by the generated clients are not exposed.
- Bitbucket behavior has drifted from the current Bitbucket Cloud API, particularly around draft pull requests.
- GitHub Enterprise and Bitbucket Data Center are not reliably recognized from repository URLs.

A sustainable design should treat exact identification and collection queries as separate operations. It should also distinguish portable filters from provider-native query capabilities instead of attempting to force every provider into one universal query language.

## Current Sniff implementation

### Public provider interface

`sniff/lib/src/remote/provider.rs` defines one pull request operation:

```rust
async fn list_pull_requests(
    &self,
    owner: &str,
    repo: &str,
    state: PullRequestState,
) -> Result<Vec<PullRequestInfo>, SniffError>;
```

This interface requires a repository and one lifecycle state. It cannot express:

- A pull request number or URL
- Multiple pull request numbers
- Free-text search
- Branch filters
- Author, assignee, or reviewer filters
- Labels or milestones
- Date ranges
- Review or CI status
- Sort order
- Pagination
- Organization, group, workspace, or global scope
- A provider-native query expression

`Vec<PullRequestInfo>` also prevents the provider from returning pagination metadata, continuation links, total counts, warnings, or information about filters it could not apply.

### CLI behavior

`sniff repo pr` discovers the preferred remote for the current repository and accepts:

```text
--status open|closed|merged|draft|all
```

It cannot accept:

- A repository URL
- A pull request URL
- A pull request number
- A provider query
- An explicit remote
- A page size or continuation token

`sniff repo remote <URL>` can inspect an arbitrary repository, but its report fetches only open pull requests. It is not an alternative query interface.

### Result identity

`PullRequestInfo` contains the pull request number, title, state, author, branches, labels, timestamps, body, and web URL. It does not carry:

- Provider
- Server host
- Repository namespace
- Repository name
- GitLab global ID versus project-scoped IID
- Provider API URL

A number such as `123` is only meaningful within a repository. Consequently, the current model is unsuitable for multi-repository results and cannot always be used as input to a later exact lookup.

### State handling

| Provider  | Current behavior                                                                                                                                              |
|-----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub    | Open and closed are sent to the API. Merged and draft retrieve `state=all` and filter the returned page locally.                                              |
| GitLab    | Open, closed, and merged are sent to the API. Draft retrieves all states and filters locally.                                                                 |
| Gitea     | Open and closed are sent to the API. Merged and draft retrieve all states and filter locally.                                                                 |
| Bitbucket | Open and merged are sent to the API. Closed and all omit `state`, even though Bitbucket defaults the collection to open pull requests. Draft is always empty. |

Local filtering after a single page is not complete. For example, the first GitHub page containing all states may contain no draft pull requests even though matching drafts exist on later pages.

Bitbucket’s current behavior is more serious:

- A closed query can retrieve the default open page and then filter everything out.
- An all-state query can return only open pull requests.
- Draft queries always return no results.

Bitbucket Cloud has supported draft pull requests since April 2025, and its current response schema includes a `draft` property. Sniff’s hard-coded `draft: false` behavior is therefore stale. See [Bitbucket draft pull requests](https://support.atlassian.com/bitbucket-cloud/docs/draft-pull-requests/) and the [Bitbucket Cloud pull request API](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pullrequests/).

### Pagination and ordering

Every adapter currently makes one generated-client request. The page, page-size, limit, sort, and direction fields remain unset.

This means Sniff returns only the provider’s first default page:

- GitHub exposes `page` and `per_page`.
- GitLab exposes `page` and `per_page`.
- Gitea exposes `page` and `limit`.
- Bitbucket exposes `page`, `pagelen`, and a `next` link.

The limitation affects every provider, even though it is explicitly documented only for Bitbucket.

### Generated-client coverage

Some capabilities exist in the generated Schematic clients but are not exposed by Sniff:

- GitHub list requests already model `sort`, `direction`, `page`, and `per_page`.
- GitLab already has a generated exact merge-request operation.
- Gitea list requests already model `sort`, `page`, and `limit`.
- Bitbucket already has a generated exact pull-request operation.

Conversely, the generated GitHub and Gitea definitions do not currently include their exact pull-request endpoints. Supporting exact lookup consistently will therefore require additions to those API definitions as well as changes to the Sniff provider trait.

## Exact pull request identification

An exact reference must include both repository identity and the provider’s repository-scoped pull request identifier.

Canonical web URLs are good user-facing identifiers:

| Provider        | Example                                                         |
|-----------------|-----------------------------------------------------------------|
| GitHub          | `https://github.com/acme/widgets/pull/123`                      |
| GitLab          | `https://gitlab.example.com/group/widgets/-/merge_requests/123` |
| Gitea           | `https://codeberg.org/acme/widgets/pulls/123`                   |
| Bitbucket Cloud | `https://bitbucket.org/acme/widgets/pull-requests/123`          |

Sniff’s existing repository URL parser should not be used unchanged for these URLs. In particular, a nested GitLab merge-request URL can cause its trailing merge-request path to be mistaken for part of the project path.

A structured reference should preserve:

- Provider family
- Host
- Namespace, workspace, or project key
- Repository or project
- Repository-scoped pull request number
- The original URL when one was supplied

GitLab requires particular care: its `iid` is project-scoped, while its `id` is globally assigned. The normal web reference `!123` identifies the project-scoped IID.

### Provider APIs

| Provider        | Exact lookup API                                                            |
|-----------------|-----------------------------------------------------------------------------|
| GitHub          | `GET /repos/{owner}/{repo}/pulls/{pull_number}`                             |
| GitLab          | `GET /projects/{id-or-url-encoded-path}/merge_requests/{merge_request_iid}` |
| Gitea           | `GET /repos/{owner}/{repo}/pulls/{index}`                                   |
| Bitbucket Cloud | `GET /repositories/{workspace}/{repo_slug}/pullrequests/{pull_request_id}`  |

These operations are documented by the [GitHub pull request API](https://docs.github.com/en/rest/pulls/pulls?apiVersion=2022-11-28), [GitLab merge request API](https://docs.gitlab.com/api/merge_requests/), [Gitea API](https://docs.gitea.com/api/1.25/), and [Bitbucket Cloud pull request API](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pullrequests/).

A missing pull request should normally map an HTTP `404` to `Ok(None)`. Authentication, authorization, rate-limit, transport, and malformed-reference failures should remain distinguishable errors.

### Batch lookup

GitLab supports a native set lookup through:

```text
GET /projects/:id/merge_requests?iids[]=42&iids[]=43
```

The other providers generally require either:

- Several exact requests with bounded concurrency, or
- A provider query where the query language can express a set of identifiers

Bitbucket’s `q` language can filter on fields and supports `IN`, making an expression such as `id IN (...)` possible. Exact endpoint calls remain preferable when stable ordering and one-result-per-input semantics matter.

## Provider-native search and query expressions

The providers do not share one query model:

- GitHub and Bitbucket expose expression languages.
- GitLab and Gitea primarily expose collections of query parameters.
- GitLab additionally has a general search API.
- Gitea represents pull requests as issues in its cross-repository issue search.

Sniff should therefore distinguish a portable structured filter from an opaque provider-native query. A single unqualified string would be ambiguous.

### GitHub

GitHub’s issue and pull request search endpoint is:

```text
GET /search/issues?q=repo:acme/widgets+is:pr+is:open+review:required
```

Useful qualifiers include:

- `is:pr`
- `repo:`, `org:`, and `user:`
- `is:open`, `is:closed`, `is:merged`, and `is:draft`
- `author:`, `assignee:`, `reviewed-by:`, and `review-requested:`
- `involves:`
- `label:` and `milestone:`
- `head:` and `base:`
- `review:none|required|approved|changes_requested`
- `status:success|failure|pending`
- Creation, update, merge, and closure date ranges
- Boolean `AND`, `OR`, `NOT`, and grouped expressions
- Sort directives

The API returns pull requests through the issue search representation. Callers needing the complete pull request representation may need to hydrate each match through the exact pull request endpoint.

See GitHub’s [search API](https://docs.github.com/en/rest/search/search?apiVersion=2022-11-28#search-issues-and-pull-requests) and [pull request search syntax](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests).

### GitLab

GitLab’s project collection provides extensive query parameters:

```text
GET /projects/:id/merge_requests?state=opened&search=parser&in=title,description
```

The same model is available at wider scopes:

```text
GET /groups/:id/merge_requests
GET /merge_requests
```

The group endpoint includes merge requests from projects in the group and its subgroups. The top-level endpoint searches merge requests visible to the authenticated user.

The separate search API can also search merge requests:

```text
GET /search?scope=merge_requests&search=parser
```

The merge-request collection is usually preferable because it offers richer filtering. See the [GitLab merge request API](https://docs.gitlab.com/api/merge_requests/) and [GitLab search API](https://docs.gitlab.com/api/search/).

### Gitea and Forgejo

Gitea supports repository pull-request listing:

```text
GET /repos/{owner}/{repo}/pulls
```

It also provides issue search with pull-request filtering:

```text
GET /repos/issues/search?type=pulls&q=parser
```

This search surface can add owner, team, state, labels, milestones, date bounds, and authenticated-user relationship filters. A repository issue collection can similarly use `type=pulls`.

Exact parameters vary by Gitea and Forgejo server version. Sniff should discover or record the server version instead of assuming the newest public Gitea schema. See the [Gitea API](https://docs.gitea.com/api/1.25/) and [Gitea API usage guidance](https://docs.gitea.com/1.25/development/api-usage).

### Bitbucket Cloud

Bitbucket Cloud adds its common `q` and `sort` parameters to the repository pull-request collection:

```text
GET /repositories/{workspace}/{repo_slug}/pullrequests?q=...&sort=-updated_on
```

Its query language supports:

- Nested fields
- `AND`, `OR`, and parentheses
- `=`, `!=`, `~`, and `!~`
- Comparison operators
- `IN` and `NOT IN`
- String, numeric, Boolean, null, datetime, and list values
- Ascending and descending sort fields

An example query can combine source repository, state, reviewer, and destination branch:

```text
source.repository.full_name != "acme/widgets"
AND state = "OPEN"
AND reviewers.nickname = "reviewer"
AND destination.branch.name = "main"
```

See the [Bitbucket pull request API](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pullrequests/) and [Bitbucket filtering and sorting syntax](https://developer.atlassian.com/cloud/bitbucket/rest/intro/).

## Common structured capabilities missing from Sniff

### Lifecycle and draft state

| Provider  | API access                                                                                                                          |
|-----------|-------------------------------------------------------------------------------------------------------------------------------------|
| GitHub    | Repository listing accepts \`state=open                                                                                              |
| GitLab    | \`state=opened                                                                                                                       |
| Gitea     | Repository listing accepts \`state=open                                                                                              |
| Bitbucket | Repeat the `state` parameter to select `OPEN`, `MERGED`, `DECLINED`, or `SUPERSEDED`; draft can be filtered through `q=draft=true`. |

A portable query should allow a set of lifecycle states rather than one enum value. Draft should be modeled independently because it is not a terminal lifecycle state.

### Source and destination branches

| Provider  | API access                                                                                                                                                       |
|-----------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub    | Repository list parameters `head={owner}:{branch}` and `base={branch}`; search also supports `head:` and `base:`.                                                |
| GitLab    | `source_branch` and `target_branch` parameters.                                                                                                                  |
| Gitea     | Pull request listing supports branch-oriented filters in newer API versions; `GET /repos/{owner}/{repo}/pulls/{base}/{head}` can resolve a branch pair directly. |
| Bitbucket | Use `q` against fields such as `source.branch.name`, `source.repository.full_name`, and `destination.branch.name`.                                               |

These filters are especially useful for finding a pull request associated with the current local branch, including pull requests originating in forks.

### Authors, assignees, reviewers, and work queues

| Provider  | API access                                                                                                                                                                          |
|-----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub    | Search qualifiers including `author:`, `assignee:`, `involves:`, `reviewed-by:`, `review-requested:`, `user-review-requested:`, and `team-review-requested:`.                       |
| GitLab    | `author_id`, `author_username`, `assignee_id`, `assignee_username`, `reviewer_id`, `reviewer_username`, and scopes such as `created_by_me`, `assigned_to_me`, and `reviews_for_me`. |
| Gitea     | Issue search supports authenticated-user flags such as assigned, created, mentioned, review requested, and reviewed, plus creator and team filters.                                 |
| Bitbucket | Use `q` over author, reviewers, participants, approval state, and account identifiers.                                                                                              |

Sniff currently retains only a normalized author name. It loses reviewer, assignee, participant, approval, and review-request information.

### Review, approval, CI, and merge readiness

| Provider  | API access                                                                                                                                                             |
|-----------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub    | Search qualifiers \`review:none                                                                                                                                         |
| GitLab    | Reviewer and approval filters, including approved-by and eligible-approver filters where supported by the GitLab tier; exact results expose merge and pipeline status. |
| Gitea     | Search flags cover requested and completed reviews; exact pull request and review endpoints provide further detail.                                                    |
| Bitbucket | `q` can inspect reviewers, participants, approvals, draft state, and other returned fields; build and commit status resources provide CI information.                  |

These features are important for questions such as “which pull requests are waiting for me?” and “which approved pull requests are blocked only by CI?”

### Labels, milestones, reactions, and project metadata

| Provider  | API access                                                                                                   |
|-----------|--------------------------------------------------------------------------------------------------------------|
| GitHub    | Search qualifiers such as `label:`, `milestone:`, `project:`, `linked:issue`, and reaction-count qualifiers. |
| GitLab    | `labels`, `milestone`, `my_reaction_emoji`, label-priority ordering, and related exclusion filters.          |
| Gitea     | Label IDs or names and milestone IDs or names, depending on the collection endpoint.                         |
| Bitbucket | Use `q` on fields present in the pull request representation or related resources.                           |

GitLab additionally supports negated filters through its `not` parameter.

### Date and activity ranges

| Provider  | API access                                                                                                     |
|-----------|----------------------------------------------------------------------------------------------------------------|
| GitHub    | Search qualifiers for created, updated, closed, and merged timestamps.                                         |
| GitLab    | `created_after`, `created_before`, `updated_after`, `updated_before`, `deployed_after`, and `deployed_before`. |
| Gitea     | Issue search supports `since` and `before` timestamps.                                                         |
| Bitbucket | Use datetime comparisons in `q`, such as `updated_on >= 2026-01-01T00:00:00Z`.                                 |

Sniff currently returns timestamps but cannot filter by them.

### Repository, organization, group, and account scope

| Provider        | API access                                                                                                                                              |
|-----------------|---------------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub          | Search can be scoped with `repo:`, `org:`, or `user:` and can span multiple repositories.                                                               |
| GitLab          | Project, group, and all-visible merge request collection endpoints.                                                                                     |
| Gitea           | Cross-repository issue search with `type=pulls`, optionally narrowed by owner or team.                                                                  |
| Bitbucket Cloud | The documented pull request query collection is repository-scoped; wider queries generally require repository enumeration or a separate search surface. |

A query scope should be explicit. It cannot always be represented by `owner` and `repo`.

### Commit association

All four providers expose useful ways to find the pull request associated with a commit:

| Provider        | API access                                                               |
|-----------------|--------------------------------------------------------------------------|
| GitHub          | `GET /repos/{owner}/{repo}/commits/{commit_sha}/pulls`                   |
| GitLab          | `GET /projects/:id/repository/commits/:sha/merge_requests`               |
| Gitea           | `GET /repos/{owner}/{repo}/commits/{sha}/pull`                           |
| Bitbucket Cloud | `GET /repositories/{workspace}/{repo_slug}/commit/{commit}/pullrequests` |

GitHub documents this in its [commit API](https://docs.github.com/en/rest/commits/commits?apiVersion=2022-11-28#list-pull-requests-associated-with-a-commit), GitLab in its [commit API](https://docs.gitlab.com/api/commits/#list-merge-requests-associated-with-a-commit), and Bitbucket in its [pull request API](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pullrequests/).

This capability would allow Sniff to resolve pull requests from the current `HEAD`, an arbitrary commit, or a detached checkout without guessing from branch names.

### Sorting and pagination

| Provider  | Sorting                                                                                                                | Pagination                                       |
|-----------|------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------|
| GitHub    | Repository lists support created, updated, popularity, and long-running ordering; search supports its own sort fields. | `page`, `per_page`, and response links           |
| GitLab    | Created, updated, merged, priority, label priority, milestone due date, popularity, and title, depending on endpoint   | `page`, `per_page`, and pagination headers       |
| Gitea     | Oldest, recent update, least update, comment count, and priority, depending on server version                          | `page`, `limit`, `Link`, and total-count headers |
| Bitbucket | `sort=field` or `sort=-field`                                                                                          | `page`, `pagelen`, `next`, and `previous`        |

Sniff should either traverse all pages deliberately or return a page object. Silently returning the first page as though it were complete is unsafe for both exact post-filtering and user-facing search.

### Richer response fields and related resources

Exact endpoints and related subresources expose information that Sniff currently discards:

- Reviewers, assignees, and participants
- Approvals and requested changes
- Mergeability and conflict status
- Commit and file counts
- Diff statistics
- Comments and unresolved tasks
- Checks, pipelines, and commit statuses
- Queue state
- Provider-specific links
- Repository and project identity

Bitbucket’s generated model deserves particular attention. Its current API representation includes `description`, rendered content, summary, reviewers, participants, task and comment counts, draft state, and queue state. Sniff currently sets the body to `None` and draft to `false`, losing information that is now available.

## Server families and capability discovery

Provider identity alone is not enough to determine the API contract.

### GitHub Enterprise Server

The GitHub adapter accepts a configurable API base URL, but automatic URL detection recognizes only `github.com`. An enterprise host can therefore be classified as Gitea.

Sniff should preserve explicit configuration and recognize known enterprise hosts rather than assuming every unknown forge is Gitea.

### GitLab self-managed

GitLab self-managed instances vary by GitLab version and license tier. Some approval and deployment filters may not be available everywhere. Capability errors should remain visible rather than being converted into empty results.

### Gitea and Forgejo

Gitea and Forgejo APIs evolve independently. Query parameters available on the current public Gitea service may not exist on an older self-hosted server.

Sniff should record the server version when practical and allow provider adapters to report supported filters.

### Bitbucket Cloud versus Data Center

Bitbucket Cloud and Bitbucket Data Center are separate API families.

Bitbucket Cloud uses paths such as:

```text
/2.0/repositories/{workspace}/{repo_slug}/pullrequests/{id}
```

Bitbucket Data Center uses project and repository coordinates, with paths such as:

```text
/rest/api/latest/projects/{projectKey}/repos/{repositorySlug}/pull-requests/{id}
```

The current Sniff implementation uses the Cloud API and recognizes `bitbucket.org`. An unknown Bitbucket Data Center host is likely to be classified as Gitea, despite existing documentation suggesting custom Bitbucket base URLs are supported.

Data Center should therefore be represented as a distinct provider variant or API flavor. See the [Bitbucket Data Center pull request API](https://developer.atlassian.com/server/bitbucket/rest/v1003/api-group-pull-requests/).

## Recommended Sniff API shape

The exact and collection operations should remain separate:

```rust
async fn get_pull_request(
    &self,
    reference: &PullRequestReference,
) -> Result<Option<PullRequestInfo>, SniffError>;

async fn query_pull_requests(
    &self,
    query: &PullRequestQuery,
) -> Result<PullRequestPage, SniffError>;
```

A reference should support canonical URLs and structured coordinates:

```rust
enum PullRequestReference {
    Url(Url),
    Coordinates {
        provider: GitProvider,
        host: String,
        namespace: String,
        repository: String,
        number: u64,
    },
}
```

A collection query should separate portable filters from provider-native syntax:

```rust
enum PullRequestQuery {
    Structured(StructuredPullRequestQuery),
    Native(ProviderNativePullRequestQuery),
}
```

The structured query should include:

- Scope
- One or more lifecycle states
- Draft status
- Source and destination branches
- Authors, assignees, and reviewers
- Labels and milestones
- Date ranges
- Sort order
- Page size or result limit

A native query needs provider-specific representation:

- GitHub: complete `q` expression
- GitLab: merge-request query parameters or general search term and scope
- Gitea: issue-search parameters
- Bitbucket: `q` expression plus `sort`

A page result should retain completeness and provenance:

```rust
struct PullRequestPage {
    items: Vec<PullRequestInfo>,
    next: Option<PullRequestContinuation>,
    total: Option<u64>,
    warnings: Vec<PullRequestQueryWarning>,
}
```

Each returned pull request should include its provider, host, repository coordinates, repository-scoped identifier, canonical web URL, and API URL.

Unsupported portable filters should produce a clear error or warning. They should not be silently ignored. Provider-native queries should be passed through without Sniff attempting to reinterpret their semantics.

## Overall assessment

Sniff has the authentication, provider selection, normalized result type, and generated-client infrastructure needed to build this feature. Its current pull request surface, however, is a status listing convenience rather than a general lookup or query API.

The highest-priority gaps are:

1. Add exact reference parsing and exact provider lookups.
2. Add pagination before relying on any client-side filtering.
3. Correct Bitbucket state and draft handling.
4. Preserve provider and repository identity in every result.
5. Introduce explicit query scope and portable structured filters.
6. Add provider-native query support without pretending the providers share one grammar.
7. Distinguish Bitbucket Cloud from Data Center and improve enterprise-host detection.
8. Expose commit and branch association, which are often more ergonomic than manually supplying a pull request number.

With those changes, Sniff could provide a predictable common surface while still allowing callers to use the substantially richer query features offered by each forge.
