---
name: schematic
description: Expert knowledge for Schematic REST and WebSocket API client code generation. Use when defining APIs, generating typed Rust clients, importing OpenAPI specs, adding endpoints, configuring authentication, building headers programmatically, or troubleshooting code generation issues.
---

# Schematic

Type-safe REST and WebSocket API client code generation for Rust. Define APIs declaratively, generate strongly-typed clients automatically.

## Quick Reference

| Package | Purpose |
|---------|---------|
| `schematic-define` | Primitives: `RestApi`, `Endpoint`, `AuthStrategy`, `Headers`, `ApiRequest`, `ApiResponse` |
| `schematic-definitions` | Pre-built APIs: Anthropic, OpenAI, ElevenLabs, HuggingFace, LM Studio, Ollama, EMQX |
| `schematic-gen` | Code generator CLI with `generate`, `validate`, and `import` commands |
| `schematic-schema` | Generated clients (auto-generated, do not edit) |

## CLI Commands

```bash
# Generate client code
schematic-gen generate --api anthropic --output schematic/schema/src

# Validate without generating
schematic-gen validate --api openai

# Import from OpenAPI spec (feature-gated)
schematic-gen import --input api.yaml --output schematic/schema/src

# Generate with OpenAPI export
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml

# Available APIs: anthropic, openai, elevenlabs, huggingface, lmstudio,
#                 ollama-native, ollama-openai, emqx-basic, emqx-bearer, all
```

## Critical Configuration

### Response Types - Choose Correctly

| `ApiResponse` Variant | Generated Method | Use For |
|-----------------------|------------------|---------|
| `ApiResponse::Json(Schema)` | `request<T>()` | JSON responses (most common) |
| `ApiResponse::Binary` | `request_bytes()` | Audio, images, archives |
| `ApiResponse::Text` | `request_text()` | Plain text |
| `ApiResponse::Empty` | `request_empty()` | 204 No Content |

**Common Mistake**:
```rust
// WRONG - Audio endpoint returning binary data
Endpoint { response: ApiResponse::json_type("AudioResponse"), ... }  // Runtime failure!

// CORRECT
Endpoint { response: ApiResponse::Binary, ... }  // Returns bytes::Bytes
```

### Module Path for Multi-API Modules

When multiple APIs share one definitions module, you MUST set `module_path` and `request_suffix`:

```rust
// Both APIs in definitions/src/ollama/mod.rs → generates single ollama.rs
RestApi { name: "OllamaNative".to_string(), module_path: Some("ollama".to_string()),
          request_suffix: Some("NativeRequest".to_string()), ... }
RestApi { name: "OllamaOpenAI".to_string(), module_path: Some("ollama".to_string()),
          request_suffix: Some("OaiRequest".to_string()), ... }
```

The `request_suffix` prevents naming collisions when APIs have overlapping endpoint IDs (e.g., both have `Embeddings` → `EmbeddingsNativeRequest` vs `EmbeddingsOaiRequest`). The generator automatically combines shared-module APIs into a single output file and cleans up stale files.

### Body Type Naming Convention

Use `*Body` suffix to avoid collision with generated `*Request` wrappers:

```rust
// WRONG - Collision with generated wrapper
pub struct GenerateRequest { ... }  // Conflicts with generated GenerateRequest

// CORRECT
pub struct GenerateBody { ... }  // Generated: struct GenerateRequest { body: GenerateBody }
```

## Generated Client Features

### Constructors

```rust
let client = OpenAI::new()?;                                 // Default (reads env vars)
let client = OpenAI::with_base_url("http://localhost:8080"); // Custom URL
let client = OpenAI::with_client(custom_reqwest_client);     // Custom HTTP client
```

### Programmatic Authentication

Inject tokens without environment variables using the `Headers` builder:

```rust
use schematic_define::Headers;

let token = get_token_from_somewhere();

let client = OpenAI::new()?
    .variant()
    .headers_builder(Headers::default().use_bearer_token(token))
    .build();
```

When `Headers` has authorization set via `use_bearer_token()` or `use_basic_auth()`, env-based auth is skipped.

### Variants for Environment Switching

```rust
use schematic_define::UpdateStrategy;

let staging = client.variant()
    .base_url("https://staging.api.com/v1")
    .env_auth(vec!["STAGING_API_KEY".to_string()])
    .auth_update(UpdateStrategy::NoChange)
    .build();
```

## Defining New APIs

### Basic Structure

```rust
use schematic_define::prelude::*;

pub fn define_my_api() -> RestApi {
    RestApi {
        name: "MyApi".to_string(),
        description: "My REST API".to_string(),
        base_url: "https://api.example.com/v1".to_string(),
        docs_url: Some("https://docs.example.com".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        env_auth: vec!["MY_API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![/* ... */],
        module_path: None,
        request_suffix: None,
        env_mapping: None,
        params: None,
    }
}
```

### Endpoint Patterns

```rust
// GET with path parameter
Endpoint {
    id: "GetUser".to_string(),
    method: RestMethod::Get,
    path: "/users/{user_id}".to_string(),
    description: "Retrieve a user by ID".to_string(),
    request: None,
    response: ApiResponse::json_type("User"),
    headers: vec![],
    params: None,
}

// POST with JSON body
Endpoint {
    id: "CreateUser".to_string(),
    method: RestMethod::Post,
    path: "/users".to_string(),
    description: "Create a new user".to_string(),
    request: Some(ApiRequest::json_type("CreateUserBody")),
    response: ApiResponse::json_type("User"),
    headers: vec![],
    params: None,
}

// File upload with multipart form
Endpoint {
    id: "UploadFile".to_string(),
    method: RestMethod::Post,
    path: "/files".to_string(),
    description: "Upload a file".to_string(),
    request: Some(ApiRequest::form_data(vec![
        FormField::file("document"),
        FormField::text("title").optional(),
    ])),
    response: ApiResponse::json_type("FileMetadata"),
    headers: vec![],
    params: None,
}
```

### Authentication Strategies

| Strategy | Configuration | Generated Header |
|----------|---------------|------------------|
| `BearerToken { header: None }` | `env_auth: vec!["KEY"]` | `Authorization: Bearer <token>` |
| `ApiKey { header: "X-Key" }` | `env_auth: vec!["KEY"]` | `X-Key: <key>` |
| `Basic` | `env_username`, `env_auth[0]` | `Authorization: Basic <base64>` |
| `ApiKeyParam { location, name }` | Query or cookie | `?api_key=<key>` |
| `None` | (none) | (none) |

## Testing & Verification

**CRITICAL**: Unit tests verify syntax only, NOT runtime behavior!

```bash
# 1. Run unit tests
cargo test -p schematic-define -p schematic-definitions -p schematic-gen

# 2. Regenerate schemas
just -f schematic/justfile generate

# 3. Verify compilation
cargo check -p schematic-schema

# 4. For response type changes, manually verify:
grep -n "request_bytes\|request_text\|request_empty" schematic/schema/src/*.rs
```

## Detailed Documentation

- [Define Package](./define.md) - API definition primitives, Headers builder
- [Definitions Package](./definitions.md) - Pre-built API catalog
- [Generator Package](./gen.md) - Code generation, import command
- [OpenAPI Support](./openapi.md) - Import/export OpenAPI specs
- [Headers Builder](./headers.md) - Programmatic auth, env resolution

## Troubleshooting

| Issue | Cause | Fix |
|-------|-------|-----|
| `schematic_definitions::xyz not found` | Module path mismatch | Set `module_path` explicitly |
| Duplicate struct definitions | Shared module missing `request_suffix` | Set distinct `request_suffix` on each API |
| Recursive struct definition | Body type name collision | Rename to `*Body` suffix |
| Binary endpoint returns JSON error | Wrong `ApiResponse` | Use `ApiResponse::Binary` |
| Missing credentials error | Env var not set | Check `env_auth` var names |
| `MissingCredential` with token | `Headers` auth not set | Use `use_bearer_token()` |
