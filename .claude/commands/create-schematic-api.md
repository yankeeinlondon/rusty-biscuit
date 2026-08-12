---
name: create-schematic-api
argument-hint: <api-name> <brief description, API docs URL, or @doc reference>
description: Create a new REST API definition in schematic-definitions using schematic-define primitives
---

**IMPORTANT:** You must use the `schematic-define` skill for this command.

Use additional skills only when directly relevant. Typically that means `rust`, `thiserror`, and `serde`.

The user's requested action is: `$ARGUMENTS`

If the above is empty or still says `$ARGUMENTS`, stop immediately and reply with:

> You need to provide:
>
> 1. the API name (snake_case module name)
> 2. a brief description, docs URL, or `@path` to research/design doc
>
> Example:
>
> - `/create-schematic-api stripe "Stripe Payments API" https://docs.stripe.com/api`
> - `/create-schematic-api cloudflare use @schematic/docs/cloudflare-research.md`

Do not continue past that point if the arguments are missing.

---

## Intent

Create a production-quality REST API definition under `schematic/definitions/src/{api_name}/` using primitives from `schematic-define`. The definition will be consumed by `schematic-gen` to produce a type-safe async Rust HTTP client.

Treat the first whitespace-delimited token in `$ARGUMENTS` as the API module name (snake_case). Treat the rest as the design brief.

---

## Reference Implementations

Before writing code, read these existing definitions to understand the house patterns. Choose the reference closest to your target API's complexity:

**Simple API (few endpoints, bearer auth):**
- `schematic/definitions/src/anthropic/mod.rs` — 4 endpoints, API-key header, required constant header
- `schematic/definitions/src/lmstudio/mod.rs` — 6 endpoints, bearer token

**Generated from a published spec (do not hand-edit):**
- `schematic/definitions/src/openai/` — 265 endpoints, 1394 types, produced by
  `schematic-gen import` from `schematic/specs/openai/openapi.yaml`. If the API
  you are adding publishes an OpenAPI document, prefer importing it over hand-authoring:
  see `just -f schematic/justfile import-openai` for the invocation shape.

**Medium API (10-20 endpoints, path params, pagination):**
- `schematic/definitions/src/github/mod.rs` — 16 endpoints, rich query params, pagination
- `schematic/definitions/src/github/types.rs` — nested types, serde rename, Option patterns

**Complex API (many endpoints, multiple auth variants):**
- `schematic/definitions/src/emqx/mod.rs` — 2 auth variants sharing endpoints
- `schematic/definitions/src/elevenlabs/mod.rs` — 35+ REST + WebSocket, binary responses

**No-auth local API:**
- `schematic/definitions/src/eversolo/mod.rs` — local device control, no auth
- `schematic/definitions/src/ollama/mod.rs` — dual protocol variants

---

## Available Primitives

### Core Types

```rust
use schematic_define::{
    // API structure
    RestApi, Endpoint, RestMethod,
    // Auth
    AuthStrategy, ApiKeyLocation, UpdateStrategy,
    // Request/Response
    ApiRequest, ApiResponse, Schema,
    // Form data
    FormField, FormFieldKind,
    // Parameters
    EndpointParams, ParamDef, QueryParamType, ParamStyle, PaginationStyle, PaginationResponse,
    // Headers & env
    Headers, EnvMapping, EnvList, ApiKeyEnv, SensitiveString,
};
```

### Auth Strategies

| Strategy | Constructor | Use When |
|---|---|---|
| Bearer Token | `AuthStrategy::BearerToken { header: None }` | Most cloud APIs (OpenAI, GitHub) |
| Bearer (custom header) | `AuthStrategy::BearerToken { header: Some("X-Custom".into()) }` | Non-standard bearer header |
| API Key (header) | `AuthStrategy::ApiKey { header: "X-API-Key".into() }` | Key in custom header (Anthropic, GitLab) |
| API Key (query/cookie) | `AuthStrategy::ApiKeyParam { name, location }` | Key in query string or cookie |
| Basic Auth | `AuthStrategy::Basic` | Username + password (EMQX) |
| None | `AuthStrategy::None` | Local/unauthenticated APIs (Ollama) |

### Response Types — CRITICAL

Pick the right `ApiResponse` because the generated client method depends on it:

| `ApiResponse` | Generated Method | Return Type | Use For |
|---|---|---|---|
| `Json(Schema)` | `request<T>()` | Deserialized `T` | Most endpoints |
| `Binary` | `request_bytes()` | `bytes::Bytes` | Audio, images, archives, NDJSON streams |
| `Text` | `request_text()` | `String` | Plain text, CSV, XML |
| `Empty` | `request_empty()` | `()` | DELETE 204, fire-and-forget |

**Common mistake:** Using `Json` for binary endpoints causes runtime JSON parse failures. If the response is audio, a file download, or streaming NDJSON, use `Binary`.

### Request Types

| `ApiRequest` | Use For |
|---|---|
| `ApiRequest::json_type("BodyType")` | JSON POST/PUT/PATCH bodies |
| `ApiRequest::form_data(fields)` | Multipart file uploads |
| `ApiRequest::url_encoded(fields)` | URL-encoded form submissions |
| `ApiRequest::binary(content_type)` | Raw binary upload |
| `ApiRequest::text(content_type)` | Raw text upload |
| `None` | GET, DELETE with no body |

### Convenience Constructors

```rust
// JSON response referencing a type name
ApiResponse::json_type("ListModelsResponse")

// JSON array response: Vec<Model>
ApiResponse::json_vec_type("Model")

// JSON request body
ApiRequest::json_type("CreateUserBody")

// Schema with explicit module path (for cross-module types)
Schema::with_path("User", "crate::models")
```

---

## Critical Rules

### 1. Body Type Naming — Use `*Body` Suffix

The generator creates `{EndpointId}Request` wrapper structs. If your body type is also named `*Request`, you get a naming collision.

```rust
// WRONG — collides with generated CreateUserRequest wrapper
pub struct CreateUserRequest { pub name: String }

// CORRECT — no collision
pub struct CreateUserBody { pub name: String }
```

### 2. All Body Types Must Derive `Default`

The generator calls `Default::default()` on body types. Missing derive = compile error.

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateUserBody {
    pub name: String,
    pub email: String,
}
```

### 3. Endpoint IDs Are PascalCase

Endpoint `id` values become enum variants and struct name prefixes. They must be valid Rust identifiers in PascalCase.

```rust
Endpoint {
    id: "ListModels".to_string(),     // → ListModelsRequest, ListModels variant
    id: "GetUserById".to_string(),    // → GetUserByIdRequest
    id: "DeleteRepo".to_string(),     // → DeleteRepoRequest
}
```

### 4. Path Parameters Auto-Extract to Struct Fields

Parameters in `{braces}` in the path become `String` fields on the generated request struct with a `new()` constructor.

```rust
// Path: "/repos/{owner}/{repo}/pulls/{pull_number}"
// Generated:
pub struct ListPullFilesRequest {
    pub owner: String,
    pub repo: String,
    pub pull_number: String,
}
impl ListPullFilesRequest {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, pull_number: impl Into<String>) -> Self { ... }
}
```

### 5. Module Path for Multi-Variant APIs

When defining multiple API variants in one module (e.g., native + OpenAI-compatible), set explicit `module_path` and unique `request_suffix`:

```rust
RestApi {
    name: "OllamaNative".to_string(),
    module_path: Some("ollama".to_string()),
    request_suffix: Some("NativeRequest".to_string()),
    ...
}
```

### 6. `#[non_exhaustive]` Awareness

All `schematic-define` enums are `#[non_exhaustive]`. Match statements in tests and consumer code must include a wildcard arm.

---

## Best Practices for API Design Quality

### Idiomatic Rust & Typing

- **Strict nullability:** Don't default all fields to `Option<T>`. If the API guarantees a field is present, type it as required. This prevents downstream `unwrap()` churn.
- **Strongly typed discriminants:** Avoid stringly-typed fields for enumerations, flags, or `kind`/`type` discriminants. Define proper Rust enums. Document exhaustive lists of valid values in endpoint descriptions.
- **Standardized derives:** Derive `Eq` and `Hash` on response/request models unless floating-point or `serde_json::Value` fields prevent it. This enables use in `HashSet` and `HashMap` keys.

### Documentation & Developer Experience

- **Contextualize types:** Document what enum-like integers mean (e.g., `state: 0 = Stopped, 1 = Playing`). Explain *why* optional fields might be absent.
- **Rich endpoint descriptions:** Go beyond one-liners. Include typical parameter values, usage scenarios, and links to official docs.
- **Module-level examples:** Provide code examples showing client instantiation, especially for runtime base URL overrides and programmatic auth.
- **Document payload handling:** For envelope structures, document how payloads should be deserialized based on message type.

### Performance

- **Defer payload parsing:** For generic message wrappers, use `Option<Box<serde_json::value::RawValue>>` instead of `Option<serde_json::Value>` to avoid premature allocation.
- **Pre-allocate vectors:** When endpoint count is known, use `Vec::with_capacity(n)`.
- **Minimize allocations in helpers:** Use `Cow<'static, str>` or array slices where appropriate in builder functions.
- **Document polling overhead:** If an endpoint is meant for high-frequency polling, note the memory churn from owned `String` fields so consumers can optimize.

---

## Required Output Structure

### Files to Create

```
schematic/definitions/src/{api_name}/
├── mod.rs      # API definition function + tests
└── types.rs    # Request/response types
```

### `mod.rs` Structure

```rust
//! {API Name} REST API definition.
//!
//! ## Authentication
//! Brief description of auth strategy and env vars.
//!
//! ## Endpoint Coverage
//! | Category | Endpoints |
//! |----------|-----------|
//! | Models   | List, Get, Delete |
//!
//! ## Environment Variables
//! - `ENV_VAR_NAME` - description

mod types;
pub use types::*;

use schematic_define::{
    AuthStrategy, ApiResponse, Endpoint, EnvList, EnvMapping, RestApi, RestMethod,
    // ... other needed imports
};

/// Build the {API Name} REST API definition.
pub fn define_{api_name}_api() -> RestApi {
    RestApi {
        name: "{ApiName}".to_string(),
        description: "...".to_string(),
        base_url: "https://api.example.com/v1".to_string(),
        docs_url: Some("https://docs.example.com".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        env_auth: vec!["API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            // ... endpoints
        ],
        module_path: None,
        request_suffix: None,
        env_mapping: Some(EnvMapping {
            bearer_token: Some(EnvList::single("API_KEY".to_string())),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::RestMethod;

    #[test]
    fn api_has_correct_metadata() {
        let api = define_{api_name}_api();
        assert_eq!(api.name, "{ApiName}");
        assert_eq!(api.base_url, "https://api.example.com/v1");
    }

    #[test]
    fn api_uses_expected_auth() {
        let api = define_{api_name}_api();
        assert!(matches!(api.auth, AuthStrategy::BearerToken { .. }));
    }

    #[test]
    fn api_has_expected_endpoint_count() {
        let api = define_{api_name}_api();
        assert_eq!(api.endpoints.len(), N);
    }

    #[test]
    fn endpoints_have_valid_methods_and_paths() {
        let api = define_{api_name}_api();
        for ep in &api.endpoints {
            assert!(!ep.id.is_empty(), "endpoint id must not be empty");
            assert!(!ep.path.is_empty(), "endpoint path must not be empty");
            assert!(!ep.description.is_empty(), "endpoint {} needs description", ep.id);
        }
    }

    // Add specific endpoint tests for critical paths
}
```

### `types.rs` Structure

```rust
use serde::{Deserialize, Serialize};

/// Response from the list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItemsResponse {
    pub items: Vec<Item>,
    #[serde(default)]
    pub total: Option<u64>,
}

/// A single item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Request body for creating an item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateItemBody {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
```

### Registration in `lib.rs`

Add the new module and re-export:

1. Add `pub mod {api_name};` in alphabetical order
2. Add `pub use {api_name}::define_{api_name}_api;` to the re-exports
3. Add a doc example in the module-level docs
4. Update `definitions/README.md` with the new API

---

## Post-Creation Verification

Run these commands in order:

```bash
# 1. Unit tests on the definition
cargo test -p schematic-definitions {api_name}

# 2. Generate the client code
cargo run -p schematic-gen -- generate --api {api_name}

# 3. Verify generated code compiles
cargo check -p schematic-schema

# 4. Verify correct response methods were generated
grep -n "request_bytes\|request_text\|request_empty" schematic/schema/src/{api_name}.rs

# 5. Full lint pass
cargo clippy -p schematic-definitions -p schematic-schema -- -D warnings
```

If any step fails, fix the issue before proceeding.

---

## Common Patterns for Endpoints

### GET list with pagination

```rust
Endpoint {
    id: "ListItems".to_string(),
    method: RestMethod::Get,
    path: "/items".to_string(),
    description: "List all items with pagination support".to_string(),
    request: None,
    response: ApiResponse::json_type("ListItemsResponse"),
    headers: vec![],
    params: Some(EndpointParams {
        query: vec![
            ParamDef { name: "page".into(), required: false, param_type: QueryParamType::Integer, ..Default::default() },
            ParamDef { name: "per_page".into(), required: false, param_type: QueryParamType::Integer, ..Default::default() },
        ],
        pagination: Some(PaginationStyle::PageNumber {
            page_param: "page".into(),
            per_page_param: "per_page".into(),
            default_per_page: Some(20),
            max_per_page: Some(100),
        }),
        ..Default::default()
    }),
}
```

### GET single resource by ID

```rust
Endpoint {
    id: "GetItem".to_string(),
    method: RestMethod::Get,
    path: "/items/{item_id}".to_string(),
    description: "Retrieve a single item by ID".to_string(),
    request: None,
    response: ApiResponse::json_type("Item"),
    headers: vec![],
    params: None,
}
```

### POST with JSON body

```rust
Endpoint {
    id: "CreateItem".to_string(),
    method: RestMethod::Post,
    path: "/items".to_string(),
    description: "Create a new item".to_string(),
    request: Some(ApiRequest::json_type("CreateItemBody")),
    response: ApiResponse::json_type("Item"),
    headers: vec![],
    params: None,
}
```

### DELETE returning empty

```rust
Endpoint {
    id: "DeleteItem".to_string(),
    method: RestMethod::Delete,
    path: "/items/{item_id}".to_string(),
    description: "Delete an item by ID. Returns 204 No Content on success.".to_string(),
    request: None,
    response: ApiResponse::Empty,
    headers: vec![],
    params: None,
}
```

### Binary download

```rust
Endpoint {
    id: "DownloadFile".to_string(),
    method: RestMethod::Get,
    path: "/files/{file_id}/content".to_string(),
    description: "Download file content as binary data".to_string(),
    request: None,
    response: ApiResponse::Binary,
    headers: vec![],
    params: None,
}
```

### Multipart file upload

```rust
Endpoint {
    id: "UploadFile".to_string(),
    method: RestMethod::Post,
    path: "/files".to_string(),
    description: "Upload a file with metadata".to_string(),
    request: Some(ApiRequest::form_data(vec![
        FormField::file("file"),
        FormField::text("title").optional(),
    ])),
    response: ApiResponse::json_type("FileMetadata"),
    headers: vec![],
    params: None,
}
```

### Custom accept header (raw content)

```rust
Endpoint {
    id: "GetRawContent".to_string(),
    method: RestMethod::Get,
    path: "/repos/{owner}/{repo}/contents/{path}".to_string(),
    description: "Get raw file content from a repository".to_string(),
    request: None,
    response: ApiResponse::Text,
    headers: vec![("Accept".to_string(), "application/vnd.github.raw+json".to_string())],
    params: None,
}
```

---

## Quality Bar

The definition should be:

- **Complete:** All endpoints the user specified are present with accurate methods, paths, and types
- **Accurate:** Response types match what the API actually returns (JSON vs Binary vs Text vs Empty)
- **Typed:** Request/response structs use appropriate Rust types, not stringly-typed catch-alls
- **Documented:** Module docs, endpoint descriptions, and type field docs are useful
- **Tested:** Tests cover metadata, auth strategy, endpoint count, and critical endpoint details
- **Registered:** Added to `lib.rs` with module declaration, re-export, and doc example

Production code must not use `unwrap()` or `expect()`.

---

## Final Response Format

When you finish, report:

1. Files created or modified
2. The endpoint catalog (table of id, method, path, response type)
3. Auth strategy and environment variables
4. Verification commands run and their results
5. Any assumptions made or gaps to fill later
