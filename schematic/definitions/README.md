# schematic-definitions

Pre-built REST API definitions using `schematic-define` primitives.

## Overview

`schematic-definitions` contains actual API definitions that use the primitives from `schematic-define`. Each API is organized in its own module with:

- A `define_*_api()` function that returns a `RestApi` definition
- Response types (structs) for the API endpoints

These definitions are consumed by `schematic-gen` to generate strongly-typed Rust clients.

## Available APIs

| API | Module | Definition Function | Endpoints | Description |
|-----|--------|---------------------|-----------|-------------|
| Anthropic | `anthropic` | `define_anthropic_api()` | 4 | Anthropic Messages API for Claude AI and agent tool use |
| Artificial Analysis Data | `artificial_analysis` | `define_artificial_analysis_data_api()` | 6 | Artificial Analysis free data API for LLM and media-model benchmarks |
| Artificial Analysis CritPt | `artificial_analysis` | `define_artificial_analysis_critpt_api()` | 1 | Artificial Analysis CritPt benchmark evaluation API |
| Bitbucket | `bitbucket` | `define_bitbucket_api()` | 15 | Bitbucket Cloud REST API for repos, PRs, issues, tags |
| OpenAI | `openai` | `define_openai_api()` | 265 | Full OpenAI platform surface, imported from the vendored spec (see `specs/openai/`) |
| HuggingFace Hub | `huggingface` | `define_huggingface_hub_api()` | 28+ | Hugging Face Hub API (models, datasets, spaces, repos) |
| LM Studio | `lmstudio` | `define_lmstudio_api()` | 6 | LM Studio local inference API |
| Ollama Native | `ollama` | `define_ollama_native_api()` | 11 | Ollama local inference API (generate, chat, embeddings) |
| Ollama OpenAI | `ollama` | `define_ollama_openai_api()` | 4 | Ollama OpenAI-compatible API |
| ElevenLabs REST | `elevenlabs` | `define_elevenlabs_rest_api()` | 35+ | ElevenLabs TTS REST API (voices, text-to-speech, audio) |
| ElevenLabs WebSocket | `elevenlabs` | `define_elevenlabs_websocket_api()` | 2 | ElevenLabs TTS WebSocket streaming API |
| EMQX Basic | `emqx` | `define_emqx_basic_api()` | 30+ | EMQX MQTT Broker REST API with Basic auth |
| EMQX Bearer | `emqx` | `define_emqx_bearer_api()` | 30+ | EMQX MQTT Broker REST API with Bearer token auth |
| GitHub | `github` | `define_github_api()` | 16 | GitHub REST API for repos, PRs, issues, releases |
| GitLab | `gitlab` | `define_gitlab_api()` | 18 | GitLab REST API for repos, MRs, issues, releases |
| Gitea | `gitea` | `define_gitea_api()` | 15 | Gitea REST API for self-hosted Git forge instances |
| Eversolo | `eversolo` | `define_eversolo_api()` | 24 | Eversolo DMP-A8 local HTTP control API |
| Samsung Smart TV REST | `samsung_smart_tv` | `define_samsung_smart_tv_api()` | 4 | Samsung S95C-focused LAN API (Smart View) |
| Samsung Smart TV Remote WS | `samsung_smart_tv::remote_ws` | `define_samsung_smart_tv_remote_ws_api()` | 1 | Samsung remote control WebSocket channel |
| Unfolded Circle Core REST | `unfolded_circle::core_rest` | `define_unfolded_circle_core_rest_api()` | 11 | Unfolded Circle Core REST API |
| Unfolded Circle Core WS | `unfolded_circle::core_ws` | `define_unfolded_circle_core_ws_api()` | 4 | Unfolded Circle Core WebSocket API |
| Unfolded Circle Dock WS | `unfolded_circle::dock_ws` | `define_unfolded_circle_dock_ws_api()` | 1 | Unfolded Circle Dock WebSocket API |
| Unfolded Circle Integration WS | `unfolded_circle::integration_ws` | `define_unfolded_circle_integration_ws_api()` | 1 | Unfolded Circle Integration WS API |

## Usage

### Using the Prelude

```rust
use schematic_definitions::prelude::*;

// Get the OpenAI API definition
let api = define_openai_api();
println!("API: {} with {} endpoints", api.name, api.endpoints.len());

// Response types are also available
let model = Model {
    id: "gpt-4".to_string(),
    object: "model".to_string(),
    created: 1687882411,
    owned_by: "openai".to_string(),
};
```

### Direct Module Access

```rust
use schematic_definitions::anthropic::define_anthropic_api;

let api = define_anthropic_api();
assert_eq!(api.name, "Anthropic");
assert_eq!(api.endpoints.len(), 4);
```

```rust
use schematic_definitions::openai::{define_openai_api, Model, ListModelsResponse};

let api = define_openai_api();
assert_eq!(api.name, "OpenAI");
assert_eq!(api.base_url, "https://api.openai.com/v1");
```

```rust
use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};

let native_api = define_ollama_native_api();
assert_eq!(native_api.name, "OllamaNative");
assert_eq!(native_api.endpoints.len(), 11);

let openai_api = define_ollama_openai_api();
assert_eq!(openai_api.name, "OllamaOpenAI");
```

```rust
use schematic_definitions::elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};

let rest_api = define_elevenlabs_rest_api();
assert_eq!(rest_api.name, "ElevenLabs");
assert!(rest_api.endpoints.len() >= 45);

let ws_api = define_elevenlabs_websocket_api();
assert_eq!(ws_api.name, "ElevenLabsTTS");
```

```rust
use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};

let basic_api = define_emqx_basic_api();
assert_eq!(basic_api.name, "EmqxBasic");

let bearer_api = define_emqx_bearer_api();
assert_eq!(bearer_api.name, "EmqxBearer");
```

```rust
use schematic_definitions::github::define_github_api;

let api = define_github_api();
assert_eq!(api.name, "GitHub");
assert_eq!(api.endpoints.len(), 16);
```

```rust
use schematic_definitions::gitea::define_gitea_api;

let api = define_gitea_api();
assert_eq!(api.name, "Gitea");
assert_eq!(api.endpoints.len(), 15);
```

```rust
use schematic_definitions::gitlab::define_gitlab_api;

let api = define_gitlab_api();
assert_eq!(api.name, "GitLab");
assert_eq!(api.endpoints.len(), 18);
```

```rust
use schematic_definitions::bitbucket::define_bitbucket_api;

let api = define_bitbucket_api();
assert_eq!(api.name, "Bitbucket");
assert_eq!(api.endpoints.len(), 15);
```

## Bitbucket API

The Bitbucket module provides a definition for the Bitbucket Cloud REST API v2.0, optimized for common developer workflows.

### Authentication

Uses HTTP Basic Authentication with App Passwords:

```bash
export BITBUCKET_USERNAME="your_atlassian_account_username"
export BITBUCKET_APP_PASSWORD="your_app_password"
```

Create App Passwords via Bitbucket Settings: **Personal settings > App passwords** with required scopes.

**Important**: Bitbucket scopes are NOT hierarchical — `repository:write` does NOT imply `repository:read`. Request both explicitly.

### Pagination

Bitbucket uses **cursor-based pagination** (not page-based). Responses include a `next` URL field. The generated `PaginatedResponse<T>` type provides:

```rust
if paginated.has_next() {
    // Follow the next URL for more results
    let next_url = paginated.next.unwrap();
}
```

### Endpoints

| Endpoint | Method | Path | Response Type |
|----------|--------|------|---------------|
| GetRepository | GET | `/repositories/{workspace}/{repo_slug}` | `Repository` |
| ListDirectoryContents | GET | `/repositories/{workspace}/{repo_slug}/src/{commit}/{path}` | `PaginatedResponse<SourceEntry>` |
| GetFileContentRaw | GET | `/repositories/{workspace}/{repo_slug}/src/{commit}/{path}` | `String` (text) |
| ListPullRequests | GET | `/repositories/{workspace}/{repo_slug}/pullrequests` | `PaginatedResponse<PullRequest>` |
| GetPullRequest | GET | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}` | `PullRequest` |
| ListPullRequestComments | GET | `/repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments` | `PaginatedResponse<PullRequestComment>` |
| ListIssues | GET | `/repositories/{workspace}/{repo_slug}/issues` | `PaginatedResponse<Issue>` |
| GetIssue | GET | `/repositories/{workspace}/{repo_slug}/issues/{id}` | `Issue` |
| ListIssueComments | GET | `/repositories/{workspace}/{repo_slug}/issues/{id}/comments` | `PaginatedResponse<IssueComment>` |
| ListIssueChanges | GET | `/repositories/{workspace}/{repo_slug}/issues/{id}/changes` | `PaginatedResponse<IssueChange>` |
| ListTags | GET | `/repositories/{workspace}/{repo_slug}/refs/tags` | `PaginatedResponse<Tag>` |
| GetTag | GET | `/repositories/{workspace}/{repo_slug}/refs/tags/{name}` | `Tag` |
| ListDownloads | GET | `/repositories/{workspace}/{repo_slug}/downloads` | `PaginatedResponse<Download>` |
| GetDownload | GET | `/repositories/{workspace}/{repo_slug}/downloads/{filename}` | `bytes::Bytes` (binary) |

### Releases in Bitbucket

Bitbucket does NOT have a first-class release concept like GitHub. Instead:

- **Tags** are git refs via `/refs/tags`
- **Downloads** are release artifacts via `/downloads`
- A "release" is a tag with associated download artifacts

To list releases, query both ListTags and ListDownloads, then correlate by naming convention.

### Key Differences from GitHub

| Aspect | Bitbucket | GitHub |
|--------|-----------|--------|
| Auth | Basic Auth (App Password) | Bearer Token |
| Pagination | Cursor-based (`next` URL) | Link headers + `per_page` |
| Releases | Tags + Downloads combined | First-class Release objects |
| Scopes | NOT hierarchical | Hierarchical (write implies read) |
| IDs | UUIDs encouraged | Numeric IDs |

## GitLab API

The GitLab module provides a definition for the GitLab REST API v4, optimized for common developer workflows.

### Authentication

**Important**: GitLab uses `PRIVATE-TOKEN` header (not Bearer token):

```bash
export GITLAB_TOKEN="your_personal_access_token"
# or
export GITLAB_PRIVATE_TOKEN="your_personal_access_token"
```

### GitLab-Specific Patterns

- Project paths must be URL-encoded (`group%2Fproject` for `group/project`)
- File content is Base64 encoded
- Uses `iid` (internal ID) scoped to project, not global `id`
- Releases are optional metadata on tags (check `tag.release` field)

### Endpoints

| Endpoint | Method | Path | Response Type |
|----------|--------|------|---------------|
| ListRepositoryTree | GET | `/projects/{id}/repository/tree` | `Vec<TreeItem>` |
| GetRepositoryFile | GET | `/projects/{id}/repository/files/{file_path}` | `FileContent` |
| ListMergeRequests | GET | `/projects/{id}/merge_requests` | `Vec<MergeRequest>` |
| GetMergeRequest | GET | `/projects/{id}/merge_requests/{merge_request_iid}` | `MergeRequest` |
| ListMergeRequestCommits | GET | `/projects/{id}/merge_requests/{merge_request_iid}/commits` | `Vec<Commit>` |
| ListMergeRequestChanges | GET | `/projects/{id}/merge_requests/{merge_request_iid}/changes` | `MergeRequestChanges` |
| ListIssues | GET | `/projects/{id}/issues` | `Vec<Issue>` |
| GetIssue | GET | `/projects/{id}/issues/{issue_iid}` | `Issue` |
| ListIssueNotes | GET | `/projects/{id}/issues/{issue_iid}/notes` | `Vec<Note>` |
| ListIssueParticipants | GET | `/projects/{id}/issues/{issue_iid}/participants` | `Vec<User>` |
| ListTags | GET | `/projects/{id}/repository/tags` | `Vec<Tag>` |
| GetTag | GET | `/projects/{id}/repository/tags/{tag_name}` | `Tag` |
| ListReleases | GET | `/projects/{id}/releases` | `Vec<Release>` |
| GetRelease | GET | `/projects/{id}/releases/{tag_name}` | `Release` |
| GetLatestRelease | GET | `/projects/{id}/releases/permalink/latest` | `Release` |

### Key Differences from GitHub

| Aspect | GitLab | GitHub |
|--------|--------|--------|
| Auth header | `PRIVATE-TOKEN: <token>` | `Authorization: Bearer <token>` |
| Merge/Pull Requests | "Merge Requests" (MRs) | "Pull Requests" (PRs) |
| ID scope | `iid` (project-scoped) | `number` (repo-scoped) |
| Releases | Optional metadata on tags | Separate entities |
| File content | Base64 encoded in JSON | Base64 encoded in JSON |

## Gitea API

The Gitea module provides a definition for the Gitea REST API v1.25+, optimized for self-hosted Git forge instances.

### Authentication

**Important**: Gitea uses `Authorization: token <pat>` (not Bearer). When setting `GITEA_TOKEN`, include the `token ` prefix:

```bash
export GITEA_TOKEN="token your_personal_access_token"
```

### Base URL

The default base URL is a placeholder (`https://gitea.example.com/api/v1`). Configure this to your Gitea instance URL when creating a variant.

### Endpoints

| Endpoint | Method | Path | Response Type |
|----------|--------|------|---------------|
| GetRepository | GET | `/repos/{owner}/{repo}` | `RepositoryInfo` |
| GetGitTree | GET | `/repos/{owner}/{repo}/git/trees/{sha}` | `GitTreeResponse` |
| GetGitTreeRecursive | GET | `/repos/{owner}/{repo}/git/trees/{sha}?recursive=true` | `GitTreeResponse` |
| GetRepositoryContentRaw | GET | `/repos/{owner}/{repo}/raw/{filepath}` | `String` (text) |
| ListPullRequests | GET | `/repos/{owner}/{repo}/pulls` | `Vec<PullRequestSummary>` |
| ListPullRequestFiles | GET | `/repos/{owner}/{repo}/pulls/{index}/files` | `Vec<PullRequestFile>` |
| ListIssues | GET | `/repos/{owner}/{repo}/issues` | `Vec<IssueSummary>` |
| GetIssue | GET | `/repos/{owner}/{repo}/issues/{index}` | `IssueSummary` |
| ListIssueComments | GET | `/repos/{owner}/{repo}/issues/{index}/comments` | `Vec<IssueComment>` |
| ListIssueTimeline | GET | `/repos/{owner}/{repo}/issues/{index}/timeline` | `Vec<TimelineEvent>` |
| ListTags | GET | `/repos/{owner}/{repo}/tags` | `Vec<RepoTag>` |
| ListReleases | GET | `/repos/{owner}/{repo}/releases` | `Vec<Release>` |
| GetTagReference | GET | `/repos/{owner}/{repo}/git/refs/{git_ref}` | `Vec<GitRef>` (array!) |
| GetAnnotatedTag | GET | `/repos/{owner}/{repo}/git/tags/{sha}` | `AnnotatedTagObject` |

### Key Differences from GitHub

| Aspect | Gitea | GitHub |
|--------|-------|--------|
| Auth header | `Authorization: token <pat>` | `Authorization: Bearer <token>` |
| Base URL | Instance-specific | `https://api.github.com` |
| Pagination | `limit` (default 50) | `per_page` (default 30) |
| Issues list | `type=issues` excludes PRs | `pull_request` field filtering |
| Tag refs | Returns **array** | Returns single object |

## OpenAI API

The OpenAI module provides a definition for the OpenAI Models API.

### Endpoints

| Endpoint | Method | Path | Response Type |
|----------|--------|------|---------------|
| ListModels | GET | `/models` | `ListModelsResponse` |
| RetrieveModel | GET | `/models/{model}` | `Model` |
| DeleteModel | DELETE | `/models/{model}` | `DeleteModelResponse` |

### Response Types

```rust
use schematic_definitions::openai::{Model, ListModelsResponse, DeleteModelResponse};

/// A model available through the OpenAI API
pub struct Model {
    pub id: String,        // e.g., "gpt-4"
    pub object: String,    // always "model"
    pub created: i64,      // Unix timestamp
    pub owned_by: String,  // e.g., "openai"
}

/// Response from ListModels endpoint
pub struct ListModelsResponse {
    pub object: String,    // always "list"
    pub data: Vec<Model>,
}

/// Response from DeleteModel endpoint
pub struct DeleteModelResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

### Authentication

The OpenAI API uses Bearer token authentication:

```rust
use schematic_definitions::openai::define_openai_api;
use schematic_define::AuthStrategy;

let api = define_openai_api();

// Uses Bearer token auth
assert!(matches!(api.auth, AuthStrategy::BearerToken { .. }));

// Reads token from OPENAI_API_KEY environment variable
assert_eq!(api.env_auth, vec!["OPENAI_API_KEY"]);
```

## Critical Configuration Requirements

> **⚠️ WARNING**: Incorrect configuration here causes runtime failures or compile errors!

### Response Types

Choose the correct `ApiResponse` variant for each endpoint:

| Response Type | When to Use | Generated Method |
|---------------|-------------|------------------|
| `ApiResponse::Json(Schema)` | JSON responses (most common) | `request<T>()` |
| `ApiResponse::Binary` | Audio files, images, ZIP archives | `request_bytes()` |
| `ApiResponse::Text` | Plain text responses | `request_text()` |
| `ApiResponse::Empty` | 204 No Content, fire-and-forget | `request_empty()` |

**Common Mistakes:**

```rust
// ❌ WRONG - Audio endpoints returning binary data
Endpoint {
    id: "CreateSpeech".to_string(),
    response: ApiResponse::json_type("AudioResponse"),  // Will fail at runtime!
    ...
}

// ✅ CORRECT
Endpoint {
    id: "CreateSpeech".to_string(),
    response: ApiResponse::Binary,  // Returns bytes::Bytes
    ...
}
```

### Module Path Configuration

The `module_path` field controls where the generator imports types from:

| Scenario | Configuration |
|----------|---------------|
| API name matches module name | `module_path: None` (auto-inferred) |
| API name differs from module | **MUST set `module_path`** |
| Multiple APIs in one module | **MUST set `module_path` for each** |

**Example - Ollama has two APIs in one module:**

```rust
// definitions/src/ollama/mod.rs exports both APIs

pub fn define_ollama_native_api() -> RestApi {
    RestApi {
        name: "OllamaNative".to_string(),
        module_path: Some("ollama".to_string()),  // ← REQUIRED
        ...
    }
}

pub fn define_ollama_openai_api() -> RestApi {
    RestApi {
        name: "OllamaOpenAI".to_string(),
        module_path: Some("ollama".to_string()),  // ← REQUIRED
        ...
    }
}
```

**What happens without explicit `module_path`:**

| API Name | Inferred Path | Actual Module | Result |
|----------|---------------|---------------|--------|
| `OllamaNative` | `ollamanative` | `ollama` | ❌ Compile error: `schematic_definitions::ollamanative` not found |
| `ElevenLabs` | `elevenlabs` | `elevenlabs` | ✅ Works (names match) |

### Verification Checklist

After adding or modifying an API definition:

```bash
# 1. Generate the code
cargo run -p schematic-gen -- --api YOUR_API --output schematic/schema/src

# 2. Check for correct response methods
grep -n "request_bytes\|request_text\|request_empty" schematic/schema/src/YOUR_API.rs

# 3. Verify it compiles
cargo check -p schematic-schema

# 4. For binary endpoints, verify convenience methods exist
grep -n "pub async fn create_speech\|pub async fn download" schematic/schema/src/YOUR_API.rs
```

## Adding New APIs

To add a new API definition:

1. Create a new module directory: `src/{api_name}/`
2. Add `mod.rs` with the `define_{api_name}_api()` function
3. Add `types.rs` with response types
4. **Choose correct `ApiResponse` for each endpoint** (see above)
5. **Set `module_path` if API name differs from module name**
5b. **Set `request_suffix` if sharing a module with another API** (to avoid naming collisions)
6. Export from `src/lib.rs`
7. Add to the prelude in `src/prelude.rs`
8. **Run verification checklist above**

### Example Structure

```
src/
├── lib.rs
├── prelude.rs
├── openai/
│   ├── mod.rs      # define_openai_api()
│   └── types.rs    # Model, ListModelsResponse, etc.
└── anthropic/      # Future API
    ├── mod.rs
    └── types.rs
```

## Dependencies

- `schematic-define` - Provides the `RestApi`, `Endpoint`, `AuthStrategy` primitives
- `serde` - Serialization for response types
- `schemars` - JSON Schema generation for OpenAPI export
- `indexmap` - Ordered map for deterministic schema output

## Schema Registry

The `registry` module provides OpenAPI schema generation capabilities:

```rust
use schematic_definitions::registry::{SchemaRegistry, get_registry};
use schematic_definitions::openai::Model;

// Create a registry and register types
let registry = SchemaRegistry::new()
    .register::<Model>("Model");

// Get schemas in OpenAPI format
let openapi_schemas = registry.to_openapi_schemas();

// Get a specific schema by name
let model_schema = registry.get("Model");

// Validate that all response types for an API are registered
let api = schematic_definitions::openai::define_openai_api();
registry.validate_completeness(&api).expect("All schemas registered");
```

### get_registry()

A convenience function to get the pre-built schema registry for supported APIs:

```rust
use schematic_definitions::registry::get_registry;

// Only OpenAI currently has complete schema registry
let registry = get_registry("openai");
```

The registry supports:
- Schema registration from types implementing `JsonSchema`
- Conversion to OpenAPI 3.0 schema format
- Validation that all API response types are registered

## License

AGPL-3.0-only
