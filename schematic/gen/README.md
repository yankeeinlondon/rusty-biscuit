# schematic-gen

Code generator that transforms REST API definitions into strongly-typed Rust client code.

## Overview

`schematic-gen` takes API definitions created with `schematic-define` and generates complete, production-ready Rust HTTP client code. The generated code includes:

- A client struct with configurable base URL
- Request structs for each endpoint (with path parameters as fields)
- A unified request enum for type-safe request handling
- An async `request()` method powered by `reqwest`
- Comprehensive error handling with `thiserror`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
schematic-gen = { path = "../gen" }
```

Or build the CLI binary:

```bash
cargo build -p schematic-gen --release
```

## CLI Usage

```bash
# Generate code for the OpenAI API
schematic-gen --api openai --output schematic/schema/src

# Dry run (print generated code without writing files)
schematic-gen --api openai --dry-run

# Verbose output
schematic-gen --api openai -v      # Basic info
schematic-gen --api openai -vv     # Endpoint details
schematic-gen --api openai -vvv    # Full debug output
```

### Options

| Flag | Description |
|------|-------------|
| `-a, --api <NAME>` | API definition to generate (e.g., `openai`) |
| `-o, --output <DIR>` | Output directory for generated code (default: `schematic/schema/src`) |
| `--dry-run` | Print generated code without writing files |
| `-v, --verbose` | Increase verbosity level |

## Library Usage

### Basic Generation

```rust
use std::path::Path;
use schematic_define::apis::define_openai_api;
use schematic_gen::output::generate_and_write;

fn main() -> Result<(), schematic_gen::errors::GeneratorError> {
    let api = define_openai_api();
    let output_dir = Path::new("generated/src");

    // Generate and write code to disk
    let code = generate_and_write(&api, output_dir, false)?;
    println!("Generated {} bytes of code", code.len());
    Ok(())
}
```

### Dry Run Mode

```rust
use schematic_gen::output::generate_and_write;

// Dry run mode prints code without writing files
let code = generate_and_write(&api, output_dir, true)?;
```

### Using Individual Generators

```rust
use schematic_gen::codegen::{
    generate_api_struct,
    generate_error_type,
    generate_request_enum,
    generate_request_method,
    generate_request_struct,
};
use schematic_gen::output::{validate_code, format_code, assemble_api_code};

// Generate just the API struct
let api_tokens = generate_api_struct(&api);

// Generate a single request struct
let request_tokens = generate_request_struct(&endpoint);

// Assemble everything and validate
let tokens = assemble_api_code(&api);
let file = validate_code(&tokens)?;
let formatted = format_code(&file);
```

## Generation Pipeline

The generator follows a multi-phase pipeline:

```
┌─────────────────────────────────────────────────────────────────┐
│                      API Definition                              │
│  (RestApi struct from schematic-define)                          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 1: Code Generation                      │
│  ┌───────────────┐ ┌───────────────┐ ┌───────────────────────┐  │
│  │ Error Type    │ │ Request       │ │ Request Enum          │  │
│  │ Generation    │ │ Structs       │ │ Generation            │  │
│  └───────────────┘ └───────────────┘ └───────────────────────┘  │
│  ┌───────────────┐ ┌───────────────┐                            │
│  │ API Struct    │ │ Request       │                            │
│  │ Generation    │ │ Method        │                            │
│  └───────────────┘ └───────────────┘                            │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 2: Assembly                             │
│  - Combine all TokenStreams                                      │
│  - Add module documentation                                      │
│  - Add imports and lint attributes                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 3: Validation                           │
│  - Parse with syn to verify syntax                               │
│  - Catch generation bugs before writing                          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 4: Formatting                           │
│  - Format with prettyplease                                      │
│  - Consistent style and indentation                              │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 5: Output                               │
│  - Atomic file writes (temp + rename)                            │
│  - Create parent directories                                     │
│  - Generate Cargo.toml for schema package                        │
└─────────────────────────────────────────────────────────────────┘
```

## Generated Code Structure

For an API named "OpenAI" with three endpoints, the generator produces:

```rust
//! Generated API client for OpenAI.
//!
//! This code was automatically generated by schematic-gen. Do not edit manually.

use serde::{Deserialize, Serialize};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Error Type
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, thiserror::Error)]
pub enum SchematicError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API error (status {status}): {body}")]
    ApiError { status: u16, body: String },

    #[error("Unsupported HTTP method: {0}")]
    UnsupportedMethod(String),

    #[error("Failed to serialize request body: {0}")]
    SerializationError(String),

    #[error("Missing credentials: none of the following environment variables are set: {env_vars:?}")]
    MissingCredential { env_vars: Vec<String> },
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Request Structs (one per endpoint)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Request for ListModels endpoint.
#[derive(Debug, Clone, Default)]
pub struct ListModelsRequest {}

impl ListModelsRequest {
    pub fn into_parts(self) -> (&'static str, String, Option<String>) {
        let path = "/models".to_string();
        ("GET", path, None)
    }
}

/// Request for RetrieveModel endpoint.
#[derive(Debug, Clone, Default)]
pub struct RetrieveModelRequest {
    /// Path parameter: model
    pub model: String,
}

impl RetrieveModelRequest {
    pub fn into_parts(self) -> (&'static str, String, Option<String>) {
        let path = format!("/models/{}", self.model);
        ("GET", path, None)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Request Enum (unifies all endpoints)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub enum OpenAIRequest {
    ListModels(ListModelsRequest),
    RetrieveModel(RetrieveModelRequest),
    DeleteModel(DeleteModelRequest),
}

impl OpenAIRequest {
    pub fn into_parts(self) -> (&'static str, String, Option<String>) {
        match self {
            Self::ListModels(req) => req.into_parts(),
            Self::RetrieveModel(req) => req.into_parts(),
            Self::DeleteModel(req) => req.into_parts(),
        }
    }
}

// From impls for ergonomic conversion
impl From<ListModelsRequest> for OpenAIRequest { ... }
impl From<RetrieveModelRequest> for OpenAIRequest { ... }

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// API Client Struct
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct OpenAI {
    client: reqwest::Client,
    base_url: String,
}

impl OpenAI {
    pub const BASE_URL: &'static str = "https://api.openai.com/v1";

    pub fn new() -> Self { ... }
    pub fn with_base_url(base_url: impl Into<String>) -> Self { ... }

    pub async fn request<T: serde::de::DeserializeOwned>(
        &self,
        request: impl Into<OpenAIRequest>,
    ) -> Result<T, SchematicError> { ... }
}
```

## Using Generated Code

```rust
use schematic_schema::{OpenAI, RetrieveModelRequest, ListModelsResponse, Model};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAI::new();

    // List all models
    let models: ListModelsResponse = client
        .request(ListModelsRequest::default())
        .await?;

    // Retrieve a specific model
    let model: Model = client
        .request(RetrieveModelRequest {
            model: "gpt-4".to_string(),
        })
        .await?;

    // Use custom base URL (for testing/proxies)
    let test_client = OpenAI::with_base_url("http://localhost:8080/v1");

    Ok(())
}
```

## Authentication Strategies

The generator supports multiple authentication strategies defined in `schematic-define`. Authentication is configured in two parts:

1. **`RestApi::auth`** - Defines *how* authentication is applied (Bearer, API Key, Basic)
2. **`RestApi::env_auth`** / **`env_username`** / **`env_password`** - Defines *where* credentials come from

All strategies return `SchematicError::MissingCredential` if required credentials are not found.

### Bearer Token

```rust
RestApi {
    auth: AuthStrategy::BearerToken { header: None },  // Uses "Authorization" header
    env_auth: vec!["OPENAI_API_KEY".to_string()],      // Fallback chain of env vars
    // ...
}
```

Generated code:

```rust
let token = ["OPENAI_API_KEY"]
    .iter()
    .find_map(|var| std::env::var(var).ok())
    .ok_or_else(|| SchematicError::MissingCredential {
        env_vars: vec!["OPENAI_API_KEY".to_string()],
    })?;
req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
```

### API Key

```rust
RestApi {
    auth: AuthStrategy::ApiKey { header: "X-API-Key".to_string() },
    env_auth: vec!["X_API_KEY".to_string()],
    // ...
}
```

Generated code:

```rust
let key = ["X_API_KEY"]
    .iter()
    .find_map(|var| std::env::var(var).ok())
    .ok_or_else(|| SchematicError::MissingCredential {
        env_vars: vec!["X_API_KEY".to_string()],
    })?;
req_builder = req_builder.header("X-API-Key", key);
```

### Basic Auth

```rust
RestApi {
    auth: AuthStrategy::Basic,
    env_username: Some("API_USER".to_string()),
    env_password: Some("API_PASS".to_string()),
    // ...
}
```

Generated code:

```rust
let username = std::env::var("API_USER")
    .map_err(|_| SchematicError::MissingCredential {
        env_vars: vec!["API_USER".to_string()],
    })?;
let password = std::env::var("API_PASS")
    .map_err(|_| SchematicError::MissingCredential {
        env_vars: vec!["API_PASS".to_string()],
    })?;
req_builder = req_builder.basic_auth(username, Some(password));
```

### No Authentication

```rust
RestApi {
    auth: AuthStrategy::None,
    env_auth: vec![],
    // ...
}
```

No authentication headers are added.

## HTTP Methods

All standard HTTP methods are supported:

| Method | Generated String | reqwest Method |
|--------|------------------|----------------|
| GET | `"GET"` | `client.get()` |
| POST | `"POST"` | `client.post()` |
| PUT | `"PUT"` | `client.put()` |
| PATCH | `"PATCH"` | `client.patch()` |
| DELETE | `"DELETE"` | `client.delete()` |
| HEAD | `"HEAD"` | `client.head()` |
| OPTIONS | `"OPTIONS"` | `client.request(Method::OPTIONS)` |

## Path Parameters

Path parameters use `{param}` syntax and are automatically extracted:

```rust
// API definition
Endpoint {
    path: "/threads/{thread_id}/messages/{message_id}",
    ...
}

// Generated struct
pub struct GetMessageRequest {
    pub thread_id: String,
    pub message_id: String,
}

impl GetMessageRequest {
    pub fn into_parts(self) -> (&'static str, String, Option<String>) {
        let path = format!("/threads/{}/messages/{}", self.thread_id, self.message_id);
        ("GET", path, None)
    }
}
```

## Request Bodies

Endpoints with request bodies get a `body` field:

```rust
// API definition
Endpoint {
    request: Some(Schema::new("CreateCompletionRequest")),
    ...
}

// Generated struct
#[derive(Debug, Clone)]
pub struct CreateCompletionRequest {
    pub body: CreateCompletionRequest,  // The body type from schema
}

impl CreateCompletionRequest {
    pub fn into_parts(self) -> (&'static str, String, Option<String>) {
        let path = "/completions".to_string();
        ("POST", path, serde_json::to_string(&self.body).ok())
    }
}
```

## Module Reference

### `codegen`

Code generation for individual components:

| Generator | Description |
|-----------|-------------|
| `generate_error_type()` | Creates `SchematicError` enum |
| `generate_request_struct(&endpoint)` | Creates request struct for an endpoint |
| `generate_request_enum(&api)` | Creates unified request enum |
| `generate_api_struct(&api)` | Creates API client struct |
| `generate_request_method(&api)` | Creates async `request()` method |

### `output`

Assembly, validation, and file writing:

| Function | Description |
|----------|-------------|
| `assemble_api_code(&api)` | Combines all generators into one `TokenStream` |
| `validate_code(&tokens)` | Validates generated code with `syn` |
| `format_code(&file)` | Formats code with `prettyplease` |
| `write_atomic(&path, &content)` | Atomic file write (temp + rename) |
| `generate_and_write(&api, &dir, dry_run)` | Full pipeline: generate, validate, format, write |

### `parser`

Path parameter utilities:

| Function | Description |
|----------|-------------|
| `extract_path_params(&path)` | Extracts parameter names from path template |
| `substitute_path_params(&path, &params)` | Replaces `{param}` with values |

### `cargo_gen`

Cargo.toml generation:

| Function | Description |
|----------|-------------|
| `generate_cargo_toml()` | Returns Cargo.toml content string |
| `write_cargo_toml(&dir, dry_run)` | Writes Cargo.toml to directory |

### `errors`

Generator error types (used by the generator itself):

| Variant | Description |
|---------|-------------|
| `ParseError` | Failed to parse API definition |
| `CodeGenError` | Code generation produced invalid Rust |
| `WriteError` | Failed to write output file |
| `OutputDirNotFound` | Output directory doesn't exist |
| `ConfigError` | Invalid configuration |

Generated runtime error types (`SchematicError`):

| Variant | Description |
|---------|-------------|
| `Http` | HTTP request failed (network, timeout) |
| `Json` | JSON deserialization failed |
| `ApiError` | Non-2xx status code from API |
| `UnsupportedMethod` | Unknown HTTP method (should never occur) |
| `SerializationError` | Request body serialization failed |
| `MissingCredential` | Required auth env vars not found |

## Safety Guarantees

- **Validation**: All generated code is parsed with `syn` before writing
- **Formatting**: Output is consistently formatted with `prettyplease`
- **Atomic writes**: Uses temp file + rename to prevent partial writes
- **No panics**: Production code paths use `Result` types, no `unwrap()`/`expect()`

## Dependencies

The generated code requires these runtime dependencies (automatically included in generated `Cargo.toml`):

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tokio = { version = "1.43", features = ["rt", "macros"] }
```

## License

AGPL-3.0-only
