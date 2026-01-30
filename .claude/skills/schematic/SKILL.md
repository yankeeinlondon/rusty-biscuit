---
name: schematic
description: Expert knowledge for Schematic REST and WebSocket API client code generation. Use when defining APIs, generating typed Rust clients, adding new endpoints, configuring authentication, or troubleshooting code generation issues.
---

# Schematic

Type-safe REST and WebSocket API client code generation for Rust. Define APIs declaratively, generate strongly-typed clients automatically.

## Quick Reference

| Package | Purpose |
|---------|---------|
| `schematic-define` | Primitives: `RestApi`, `Endpoint`, `AuthStrategy`, `ApiRequest`, `ApiResponse`, `WebSocketApi` |
| `schematic-definitions` | Pre-built APIs: Anthropic, OpenAI, ElevenLabs (REST + WebSocket), HuggingFace, Ollama, EMQX |
| `schematic-gen` | Code generator CLI with `validate` and `generate` subcommands |
| `schematic-schema` | Generated clients (auto-generated, do not edit) |

## CLI Usage

```bash
# Generate a specific API
schematic-gen generate --api anthropic --output schematic/schema/src

# Validate without generating
schematic-gen validate --api openai

# Generate all standard APIs (excludes Ollama/EMQX shared modules)
schematic-gen generate --api all

# Available: anthropic, openai, elevenlabs, huggingface, ollama-native, ollama-openai, emqx-basic, emqx-bearer, all
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

When multiple APIs share one definitions module, you MUST set `module_path`:

```rust
// Both APIs in definitions/src/ollama/mod.rs
RestApi { name: "OllamaNative".to_string(), module_path: Some("ollama".to_string()), ... }
RestApi { name: "OllamaOpenAI".to_string(), module_path: Some("ollama".to_string()), ... }
```

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
let client = OpenAI::new();                                  // Default
let client = OpenAI::with_base_url("http://localhost:8080"); // Custom URL
let client = OpenAI::with_client(custom_reqwest_client);     // Custom HTTP client
let client = OpenAI::with_client_and_base_url(client, url);  // Both
```

### Accessors

```rust
client.http_client()    // &reqwest::Client - for custom requests
client.api_base_url()   // &str - current base URL
client.api_key_header() // Option<(String, String)> - auth header if ApiKey
```

### Variants

```rust
use schematic_define::UpdateStrategy;

let staging = client.variant(
    "https://staging.api.com/v1",
    vec!["STAGING_API_KEY".to_string()],
    UpdateStrategy::NoChange,  // Or UpdateStrategy::ChangeTo(new_auth)
);
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
        headers: vec![],  // Default headers for all requests
        endpoints: vec![...],
        module_path: None,      // Auto-derived from name
        request_suffix: None,   // Defaults to "Request"
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
}
```

### Authentication Strategies

| Strategy | Configuration | Generated Header |
|----------|---------------|------------------|
| `AuthStrategy::BearerToken { header: None }` | `env_auth: vec!["API_KEY"]` | `Authorization: Bearer <token>` |
| `AuthStrategy::ApiKey { header: "X-Api-Key" }` | `env_auth: vec!["API_KEY"]` | `X-Api-Key: <key>` |
| `AuthStrategy::Basic` | `env_username`, `env_auth[0]` | `Authorization: Basic <base64>` |
| `AuthStrategy::None` | (none) | (none) |

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

- [Define Package](./define.md) - API definition primitives
- [Definitions Package](./definitions.md) - Pre-built API catalog
- [Generator Package](./gen.md) - Code generation internals

## Troubleshooting

| Issue | Cause | Fix |
|-------|-------|-----|
| `schematic_definitions::xyz not found` | Module path mismatch | Set `module_path` explicitly |
| Recursive struct definition | Body type name collision | Rename to `*Body` suffix |
| Binary endpoint returns JSON error | Wrong `ApiResponse` | Use `ApiResponse::Binary` |
| Missing credentials error | Env var not set | Check `env_auth` var names |
