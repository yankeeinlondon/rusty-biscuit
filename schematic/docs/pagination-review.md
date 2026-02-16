# Schematic Pagination Review

**Date**: 2026-02-15
**Scope**: `schematic/define`, `schematic/definitions`, `schematic/gen`, `schematic/schema`
**Skills Used**: `rust`, `schematic`, `schematic-define`

---

## Executive Summary

Schematic has a well-designed `PaginationStyle` primitive in `schematic-define` with four pagination strategies and ergonomic builder methods. However, **only 1 of 11 API definitions** (Bitbucket) actually uses it. The remaining APIs either hardcode pagination values in URL path strings or omit pagination entirely. The code generator treats pagination parameters as generic query parameters with no pagination-aware features in generated clients — there is no auto-pagination, no stream/iterator, and no next-page extraction.

---

## Part 1: `schematic-define` Pagination Primitives

### Current Design (`schematic/define/src/params.rs`)

**`PaginationStyle` enum** (line 262-315):

| Variant | Parameters | Factory Method |
|---------|-----------|----------------|
| `PageNumber` | `page_param`, `per_page_param`, `default_per_page`, `max_per_page` | `github()`, `gitlab()` |
| `OffsetLimit` | `offset_param`, `limit_param`, `default_limit`, `max_limit` | `offset_limit()` |
| `Cursor` | `cursor_param`, `limit_param` (optional), `default_limit` | `cursor()` |
| `Bitbucket` | `default_pagelen` | `bitbucket()` |

**`EndpointParams` builder methods**:
- `with_pagination(style)` — appends pagination query params (line 555)
- `has_pagination()` — heuristic check for common param names (line 624)

**Validation** (`schematic/gen/src/validation.rs`):
- `validate_api_with_warnings()` emits `MissingPagination` warnings for endpoints with "List" in their ID or `PaginatedResponse`/`Vec<` in their response type that lack pagination params

### Positive Observations

1. **Clean builder API**: `EndpointParams::default().with_pagination(PaginationStyle::github())` is ergonomic and composable with `.with_query_param()`.
2. **Sensible factory methods**: `github()`, `gitlab()`, `bitbucket()`, `cursor()`, `offset_limit()` cover the most common patterns.
3. **Validation integration**: The generator warns about missing pagination on list endpoints — a thoughtful correctness check.
4. **Non-exhaustive enums**: Both `PaginationStyle` and `QueryParamType` are `#[non_exhaustive]`, allowing future additions without breaking changes.
5. **Well-tested**: 15+ pagination-specific unit tests covering all strategies, builder chaining, and detection.

### Must Fix

#### 1. `PaginationStyle` is discarded after conversion — semantic information is lost

**File**: `schematic/define/src/params.rs:555-557`

```rust
pub fn with_pagination(mut self, style: PaginationStyle) -> Self {
    self.query.extend(style.to_query_params());
    self  // style is dropped here — gone forever
}
```

When `with_pagination()` is called, the `PaginationStyle` is immediately converted to generic `ParamDef` entries and the original style is discarded. This means:
- The generator cannot distinguish between "this endpoint has cursor pagination" vs "this endpoint happens to have an `after` param"
- No generated code can be pagination-aware (e.g., auto-paginate, extract next cursor)
- The `has_pagination()` method relies on string-matching param names instead of checking a stored style

**Suggestion**: Store the pagination style on `EndpointParams`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EndpointParams {
    pub query: Vec<ParamDef>,
    pub header: Vec<ParamDef>,
    pub cookie: Vec<ParamDef>,
    pub pagination: Option<PaginationStyle>,  // NEW: preserve semantic info
}
```

Then `with_pagination()` both appends query params AND stores the style:

```rust
pub fn with_pagination(mut self, style: PaginationStyle) -> Self {
    self.query.extend(style.to_query_params());
    self.pagination = Some(style);
    self
}
```

And `has_pagination()` becomes:

```rust
pub fn has_pagination(&self) -> bool {
    self.pagination.is_some()
}
```

#### 2. `Bitbucket` variant is redundant — it's a special case of `PageNumber`

**File**: `schematic/define/src/params.rs:307-314`

The `Bitbucket` variant is structurally identical to `PageNumber` with `page_param="page"` and `per_page_param="pagelen"`. Maintaining a separate variant adds maintenance cost without adding capability.

**Suggestion**: Remove the `Bitbucket` variant and update the factory:

```rust
pub fn bitbucket() -> Self {
    Self::PageNumber {
        page_param: "page".to_string(),
        per_page_param: "pagelen".to_string(),
        default_per_page: 50,
        max_per_page: 100,
    }
}
```

This simplification reduces match arms in `to_query_params()` and any future consumers.

### Suggested Improvements

#### 3. Add `PaginationStyle` to the prelude

**File**: `schematic/define/src/prelude.rs`

The prelude exports `Endpoint`, `RestApi`, `AuthStrategy`, etc. but not `PaginationStyle`, `EndpointParams`, or `ParamDef`. Any definition author who wants to use pagination must write:

```rust
use schematic_define::{EndpointParams, PaginationStyle};
```

This is unintuitive when the prelude is meant to be a one-stop import. The Bitbucket definition has to import these separately.

**Suggestion**: Add pagination and parameter types to the prelude:

```rust
pub use crate::params::{EndpointParams, PaginationStyle};
```

#### 4. Add a `gitea()` factory method

Gitea uses `page` + `limit` (not `per_page`). Currently there's no factory for this common pattern, forcing callers to use the verbose struct literal:

```rust
PaginationStyle::PageNumber {
    page_param: "page".to_string(),
    per_page_param: "limit".to_string(),
    default_per_page: 50,
    max_per_page: 50,
}
```

**Suggestion**: Add `PaginationStyle::gitea()`:

```rust
pub fn gitea() -> Self {
    Self::PageNumber {
        page_param: "page".to_string(),
        per_page_param: "limit".to_string(),
        default_per_page: 50,
        max_per_page: 50,
    }
}
```

#### 5. `PaginationStyle` should implement `Serialize`/`Deserialize`

The enum derives `Debug, Clone, PartialEq, Eq` but not serde traits. If pagination metadata is ever needed in OpenAPI export, generated docs, or config files, it will require manual conversion.

**Suggestion**: Add `#[derive(Serialize, Deserialize)]` to `PaginationStyle`.

#### 6. `default_per_page` and `max_per_page` should be reflected in generated doc comments

The `to_query_params()` method creates descriptions like `"Items per page (default: 100, max: 100)"` which is good, but the generated Rust doc comment in the schema says just `"Query parameter: Items per page (default: 50, max: 100)"`. The actual max constraint is not enforced — it's informational only.

**Suggestion**: Consider generating a validation check or at minimum a `#[doc]` attribute with the max value on the builder method.

### Nits

- **Line 433**: `format!("Page number (1-indexed, default: 1)")` — `format!` is unnecessary when there are no interpolations; use a string literal.
- The `Cursor` variant's `default_limit` field is unused by `to_query_params()` when `limit_param` is `None`. The field is still stored but never contributes to any output. Consider making `default_limit` part of the `limit_param` option: `limit_param: Option<(String, u32)>`.

---

## Part 2: `schematic-definitions` — API Pagination Audit

### APIs Using Pagination Correctly

| API | Style | Implementation | Endpoints |
|-----|-------|---------------|-----------|
| **Bitbucket** | `PaginationStyle::bitbucket()` | `EndpointParams::default().with_pagination(...)` via helper fn | 8 of 14 endpoints |

Bitbucket is the **only** API using the pagination system as designed. It has:
- A private `bitbucket_pagination()` helper for DRY reuse
- Consistent application across all list endpoints
- Clean URL paths with no hardcoded query params
- `PaginatedResponse<T>` response wrapper with `has_next()` method

### APIs With Hardcoded Pagination (Anti-Pattern)

These APIs bake pagination parameters directly into the endpoint `path` string, bypassing the `PaginationStyle` system entirely. This means:
- Users **cannot override** `per_page` or `page` at runtime
- The generated request structs have **no pagination fields**
- The validation check for missing pagination is **silenced** (the path contains the param but `EndpointParams` is `None`)

#### GitHub (`schematic/definitions/src/github/mod.rs`)

7 list endpoints with hardcoded `per_page=100` in paths:

| Endpoint | Path Fragment |
|----------|--------------|
| `ListPullRequests` | `?state=all&sort=updated&direction=desc&per_page=100` |
| `ListPullRequestFiles` | `?per_page=100` |
| `ListIssues` | `?state=all&per_page=100` |
| `ListIssueComments` | `?per_page=100` |
| `ListIssueTimeline` | `?per_page=100` |
| `ListTags` | `?per_page=100` |
| `ListReleases` | `?per_page=100` |

**Recommendation**: Replace hardcoded params with:
```rust
params: Some(
    EndpointParams::default()
        .with_pagination(PaginationStyle::github())
        .with_query_param("state", QueryParamType::Enum(vec![
            "open".into(), "closed".into(), "all".into()
        ]), false, Some("Filter by state"))
        .with_query_param("sort", QueryParamType::Enum(vec![
            "created".into(), "updated".into(), "popularity".into(), "long-running".into()
        ]), false, Some("Sort field"))
        .with_query_param("direction", QueryParamType::Enum(vec![
            "asc".into(), "desc".into()
        ]), false, Some("Sort direction"))
),
```

And clean the path: `"/repos/{owner}/{repo}/pulls"` (no query string).

**Note**: GitHub also paginates via `Link` response headers with `rel="next"`. This is not modeled anywhere in schematic.

#### GitLab (`schematic/definitions/src/gitlab/mod.rs`)

7 list endpoints with hardcoded `per_page=100`:

| Endpoint | Path Fragment |
|----------|--------------|
| `ListRepositoryTree` | `?per_page=100&recursive=true` |
| `ListMergeRequests` | `?state=all&per_page=100` |
| `ListMergeRequestCommits` | `?per_page=100` |
| `ListIssues` | `?state=all&per_page=100` |
| `ListIssueNotes` | `?per_page=100` |
| `ListTags` | `?per_page=100` |
| `ListReleases` | `?per_page=100` |

**Recommendation**: Same approach as GitHub. Use `PaginationStyle::gitlab()` plus explicit query params for filters like `state` and `recursive`.

#### Gitea (`schematic/definitions/src/gitea/mod.rs`)

7 list endpoints with hardcoded `limit=50`:

| Endpoint | Path Fragment |
|----------|--------------|
| `ListPullRequests` | `?state=all&sort=recentupdate&limit=50` |
| `ListPullRequestFiles` | `?limit=50` |
| `ListIssues` | `?state=all&type=issues&limit=50` |
| `ListIssueComments` | `?limit=50` |
| `ListIssueTimeline` | `?limit=50` |
| `ListTags` | `?limit=50` |
| `ListReleases` | `?draft=true&pre-release=true&limit=50` |

**Recommendation**: Add `PaginationStyle::gitea()` factory (see suggestion #4 above) and migrate.

### APIs Missing Pagination Entirely

These APIs have list endpoints that likely support pagination in the real API but have no pagination configuration:

#### ElevenLabs

List endpoints without pagination: `ListVoices`, `ListSharedVoices`, `ListModels`, `ListServiceAccounts`, `ListServiceAccountApiKeys`, `ListWebhooks`, `GetHistory`

The ElevenLabs API uses offset/limit pagination for some endpoints (e.g., voice library search uses `page_size` + `cursor`).

#### HuggingFace

List endpoints without pagination: `ListModels`, `ListDatasets`, `ListSpaces`, `ListUserRepos`, `ListUserCollections`, plus file/commit listing endpoints.

The HuggingFace Hub API uses `limit` + `cursor` for pagination on search endpoints.

#### EMQX

14+ list endpoints without pagination. Notably, `ListClients` and `ListRetained` even have docstrings mentioning pagination, but `EndpointParams` is `None`.

EMQX uses `page` + `limit` for paginated endpoints.

#### LM Studio

`ListModels` has no pagination. LM Studio's models endpoint is typically unpaginated (returns all loaded models), so this may be correct.

### APIs Where Pagination is Not Needed

| API | Reason |
|-----|--------|
| **Anthropic** | 4 endpoints, all single-item or streaming |
| **OpenAI** | 3 endpoints, all single-item |
| **Ollama (Native + OpenAI)** | 15 endpoints, most single-item; `ListModels` returns all (unpaginated API) |

### Summary Table

| API | Has List Endpoints | Uses `PaginationStyle` | Hardcoded in Path | Status |
|-----|-------------------|----------------------|-------------------|--------|
| Bitbucket | 8 | Yes | No | **Correct** |
| GitHub | 7 | No | Yes (`per_page=100`) | **Needs migration** |
| GitLab | 7 | No | Yes (`per_page=100`) | **Needs migration** |
| Gitea | 7 | No | Yes (`limit=50`) | **Needs migration** |
| ElevenLabs | ~7 | No | No | **Needs addition** |
| HuggingFace | ~8 | No | No | **Needs addition** |
| EMQX | ~14 | No | No | **Needs addition** |
| LM Studio | 1 | No | No | OK (unpaginated API) |
| Anthropic | 0 | N/A | N/A | OK |
| OpenAI | 0 | N/A | N/A | OK |
| Ollama | 1 | No | No | OK (unpaginated API) |

---

## Part 3: `schematic-gen` — Code Generation Review

### How Pagination Parameters Are Generated

The generator (`schematic/gen/src/codegen/request_structs.rs`) processes pagination parameters as **generic query parameters**. There is no special code path for pagination.

**Flow**: `EndpointParams.query` → `extract_query_params()` → `QueryParamInfo` → generated struct fields

**Generated output** (from `schematic/schema/src/bitbucket.rs`):

```rust
pub struct ListPullRequestsRequest {
    pub workspace: String,
    pub repo_slug: String,
    pub page: Option<i64>,      // from PaginationStyle
    pub pagelen: Option<i64>,   // from PaginationStyle
}

impl ListPullRequestsRequest {
    pub fn new(workspace: impl Into<String>, repo_slug: impl Into<String>) -> Self { ... }
    pub fn with_page(mut self, value: i64) -> Self { ... }
    pub fn with_pagelen(mut self, value: i64) -> Self { ... }
    pub fn into_parts(self) -> Result<RequestParts, SchematicError> {
        let mut path = format!("/repositories/{}/{}/pullrequests", ...);
        if let Some(ref value) = self.page {
            // appends ?page=N or &page=N
        }
        if let Some(ref value) = self.pagelen {
            // appends ?pagelen=N or &pagelen=N
        }
        Ok(("GET", path, None, vec![]))
    }
}
```

### Must Fix

#### 7. No auto-pagination or iterator support in generated clients

The generated client requires users to manually loop:

```rust
let mut page = 1;
let mut all_items = Vec::new();
loop {
    let request = ListPullRequestsRequest::new("workspace", "repo")
        .with_page(page);
    let response: PaginatedResponse<PullRequest> = client.request(request).await?;
    all_items.extend(response.values);
    if !response.has_next() { break; }
    page += 1;
}
```

This is repetitive, error-prone (off-by-one, forgetting to check `has_next()`), and should be a first-class generated feature.

**Suggestion**: If `EndpointParams.pagination` is preserved (see suggestion #1), the generator could produce:

```rust
// Generated auto-paginate method on the client
impl Bitbucket {
    pub fn paginate<Req, Resp>(&self, initial_request: Req) -> PaginationStream<Resp>
    where
        Req: Paginated + Into<BitbucketRequest>,
    { ... }
}

// Usage:
let mut stream = client.paginate(
    ListPullRequestsRequest::new("workspace", "repo")
);
while let Some(page) = stream.next().await? {
    for pr in page.values {
        // ...
    }
}
```

This is a significant feature that requires design work and is tracked as a future improvement below.

#### 8. Generator does not use `default_per_page` / `max_per_page` information

The `PaginationStyle` stores `default_per_page` and `max_per_page` but the generator only uses these in description strings. The generated builder method `with_pagelen(value: i64)` accepts any `i64` — including negative values or values exceeding `max_per_page`.

**Suggestion**: At minimum, generate doc comments with defaults and max values. Optionally, generate a clamping or validation check:

```rust
pub fn with_pagelen(mut self, value: i64) -> Self {
    debug_assert!(value > 0 && value <= 100, "pagelen must be 1..=100");
    self.pagelen = Some(value);
    self
}
```

### Suggested Improvements

#### 9. Generate a `Paginated` marker trait for paginated request structs

When `EndpointParams.pagination` is `Some(...)`, the generator could implement a marker trait:

```rust
/// Marker trait for request structs that support pagination.
pub trait Paginated {
    /// Returns the next-page request from this request and a response.
    fn next_page(&self, response: &impl HasNextPage) -> Option<Self>
    where Self: Sized;
}
```

This enables generic pagination utilities without knowing the specific request type.

#### 10. The `into_parts()` method duplicates `?`/`&` logic for every query param

**File**: `schematic/gen/src/codegen/request_structs.rs:765-787`

Each query parameter generates its own `if !path.contains('?')` check:

```rust
if let Some(ref value) = self.page {
    if !path.contains('?') {
        path.push_str(&format!("?{}={}", "page", value));
    } else {
        path.push_str(&format!("&{}={}", "page", value));
    }
}
// repeated for every query param
```

This works but is verbose. For endpoints with 5+ query params, the generated code is bulky.

**Suggestion**: Generate a local helper or use a URL builder pattern:

```rust
let mut query_pairs: Vec<(&str, String)> = Vec::new();
if let Some(ref value) = self.page {
    query_pairs.push(("page", value.to_string()));
}
if let Some(ref value) = self.pagelen {
    query_pairs.push(("pagelen", value.to_string()));
}
if !query_pairs.is_empty() {
    let query_string = query_pairs.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");
    path.push('?');
    path.push_str(&query_string);
}
```

This also fixes a subtle correctness issue: if the path template already contains `?` (e.g., from hardcoded params in legacy definitions), the `contains('?')` check would incorrectly use `&` for the first dynamic param even though it should use `?` if the hardcoded params were removed.

#### 11. Response-side pagination metadata is not modeled

The `PaginationStyle` only models the **request** side (query parameters). The **response** side — how to detect whether more pages exist and how to get the next cursor/page — is not part of the definition system.

Different APIs signal pagination differently:

| API | Next-Page Signal |
|-----|-----------------|
| GitHub/GitLab | `Link` header with `rel="next"` |
| Bitbucket | `next` field in JSON response body |
| HuggingFace | Cursor in response body |
| EMQX | `meta.count` + `meta.page` in response body |

**Suggestion**: Add a `PaginationResponse` type to capture this:

```rust
#[non_exhaustive]
pub enum PaginationResponse {
    /// Next page URL/cursor in JSON response body field
    BodyField { next_field: String },
    /// Link header with rel="next" (RFC 8288)
    LinkHeader,
    /// Total count + current page in response metadata
    TotalCount { total_field: String, page_field: String },
}
```

This could be stored alongside `PaginationStyle` in `EndpointParams` and used by the generator to produce pagination helpers.

---

## Part 4: Feature Roadmap

Based on this review, here is a prioritized list of improvements:

### Priority 1: Fix Foundations (no breaking changes)

1. **Store `PaginationStyle` on `EndpointParams`** (suggestion #1)
2. **Remove `Bitbucket` variant** — collapse into `PageNumber` (suggestion #2)
3. **Add `PaginationStyle` to prelude** (suggestion #3)
4. **Add `gitea()` factory method** (suggestion #4)

### Priority 2: Migrate Definitions

5. **Migrate GitHub** — replace hardcoded `per_page=100` with `PaginationStyle::github()` + explicit filter params
6. **Migrate GitLab** — same approach with `PaginationStyle::gitlab()`
7. **Migrate Gitea** — use new `PaginationStyle::gitea()`
8. **Add pagination to EMQX** — determine correct style per endpoint
9. **Add pagination to ElevenLabs** — cursor-based where applicable
10. **Add pagination to HuggingFace** — cursor-based for search endpoints

### Priority 3: Enhance Code Generation

11. **Improve query param code generation** — deduplicate `?`/`&` logic (suggestion #10)
12. **Add pagination metadata to generated docs** — defaults, max values in `#[doc]`
13. **Model response-side pagination** (suggestion #11)
14. **Generate `Paginated` trait and auto-paginate helpers** (suggestions #7, #9)

### Priority 4: Advanced Features

15. **Generate `paginate()` method on clients** — returns an async stream
16. **Add `PaginationStyle` to OpenAPI export** — round-trip pagination info
17. **Add serde to `PaginationStyle`** (suggestion #5)

---

## Appendix: File Reference

| File | Lines | Role |
|------|-------|------|
| `schematic/define/src/params.rs` | ~1115 | PaginationStyle enum, EndpointParams builder, query param types |
| `schematic/define/src/prelude.rs` | 53 | Re-exports (missing pagination types) |
| `schematic/define/src/types.rs` | 684 | RestApi and Endpoint structs |
| `schematic/gen/src/validation.rs` | 528 | MissingPagination warning |
| `schematic/gen/src/codegen/request_structs.rs` | ~1738 | Query param → struct field generation |
| `schematic/gen/src/codegen/client.rs` | ~1204 | Client request method generation |
| `schematic/gen/src/codegen/api_struct.rs` | ~1052 | API struct with `http_client()` |
| `schematic/definitions/src/bitbucket/mod.rs` | — | Only API using PaginationStyle correctly |
| `schematic/definitions/src/github/mod.rs` | — | 7 endpoints with hardcoded per_page |
| `schematic/definitions/src/gitlab/mod.rs` | — | 7 endpoints with hardcoded per_page |
| `schematic/definitions/src/gitea/mod.rs` | — | 7 endpoints with hardcoded limit |
| `schematic/schema/src/bitbucket.rs` | — | Generated client showing pagination output |
