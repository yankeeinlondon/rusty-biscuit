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

The CLI provides two subcommands: `generate` and `validate`.

```bash
# Generate code for the OpenAI API
schematic-gen generate --api openai --output schematic/schema/src

# Validate an API definition (without generating code)
schematic-gen validate --api openai

# Dry run (print generated code without writing files)
schematic-gen generate --api openai --dry-run

# Verbose output
schematic-gen generate --api openai -v      # Basic info
schematic-gen generate --api openai -vv     # Endpoint details
schematic-gen generate --api openai -vvv    # Full debug output

# Legacy syntax (backwards compatible - runs generate)
schematic-gen --api openai --output schematic/schema/src
```

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `generate` | Generate API client code (runs validation first) |
| `validate` | Validate API definition without generating code |

### Options

| Flag | Description |
|------|-------------|
| `-a, --api <NAME>` | API definition to generate/validate (e.g., `openai`) |
| `-o, --output <DIR>` | Output directory for generated code (default: `schematic/schema/src`) |
| `--dry-run` | Print generated code without writing files |
| `-v, --verbose` | Increase verbosity level |

### Validation

The `validate` subcommand checks for:

- **Naming collisions**: Ensures body type names don't conflict with generated wrapper struct names
- **Request suffix format**: Validates custom `request_suffix` is alphanumeric

```bash
$ schematic-gen validate --api openai
  [PASS] Request suffix format
  [PASS] No naming collisions detected

[OK] All validation checks passed for 'OpenAI'
```

Validation runs automatically before generation. If validation fails, generation is aborted with a descriptive error message.

## Library Usage

### Basic Generation

```rust
use std::path::Path;
use schematic_definitions::openai::define_openai_api;
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

## Generated Output Structure

The generator produces per-API module files:

```
schema/src/
├── lib.rs           # Module declarations
├── prelude.rs       # Convenient re-exports
├── shared.rs        # RequestParts, SchematicError, reqwest re-export
├── anthropic.rs     # Anthropic API client
├── openai.rs        # OpenAI API client
├── elevenlabs.rs    # ElevenLabs API client
├── huggingface.rs   # HuggingFace Hub API client
├── ollama.rs        # Ollama Native API client (generated separately)
└── ollamaopenai.rs  # Ollama OpenAI API client (generated separately)
```

## Generator Conventions & Assumptions

### Available APIs

The CLI supports these API names:

```
anthropic, openai, elevenlabs, huggingface, ollama-native, ollama-openai, emqx-basic, emqx-bearer, all
```

**Note**: `all` excludes Ollama and EMQX APIs (must generate individually due to shared modules).

### Module Naming

The generator assumes **1 API = 1 module** with matching names (lowercased):

```
API Name: "OpenAI"     → Module: openai.rs    → Import: schematic_definitions::openai::*
API Name: "ElevenLabs" → Module: elevenlabs.rs → Import: schematic_definitions::elevenlabs::*
```

#### Module Path Configuration

You can override the default module path derivation using `RestApi::module_path`:

```rust
let api = RestApi {
    name: "OllamaOpenAI".to_string(),
    module_path: Some("ollama".to_string()),  // Use "ollama" instead of "ollamaopenai"
    // ...
};
```

#### Automatic Path Inference

For API names with recognized suffixes, the generator can infer the module path:

| API Name | Inferred Module |
|----------|-----------------|
| `OllamaNative` | `ollama` (strips "Native" suffix) |
| `HTTPClient` | `http` (strips "Client" suffix) |
| `MyService` | `my` (strips "Service" suffix) |

Explicit `module_path` always takes precedence over inference.

**Multi-API modules require explicit configuration**. If you define multiple APIs in a single definitions module, set `module_path` explicitly:

```rust
// definitions/src/ollama/mod.rs defines both:
pub fn define_ollama_native_api() -> RestApi {
    RestApi {
        name: "OllamaNative".to_string(),
        module_path: Some("ollama".to_string()),  // Explicit path
        // ...
    }
}

pub fn define_ollama_openai_api() -> RestApi {
    RestApi {
        name: "OllamaOpenAI".to_string(),
        module_path: Some("ollama".to_string()),  // Same module
        // ...
    }
}
```

### Wrapper Struct Generation

For each endpoint, the generator creates a wrapper struct named `{EndpointId}{Suffix}`:

```
Endpoint ID: "Generate"      → struct GenerateRequest { ... }
Endpoint ID: "ListModels"    → struct ListModelsRequest { ... }
Endpoint ID: "CreateChat"    → struct CreateChatRequest { ... }
```

The suffix defaults to "Request" but can be customized via `RestApi::request_suffix`:

```rust
let api = RestApi {
    name: "MyApi".to_string(),
    request_suffix: Some("Req".to_string()),  // Use "Req" instead of "Request"
    // ...
};
// Generates: struct GenerateReq, struct ListModelsReq, etc.
```

**Important**: Body types in your definitions must use different names (conventionally `*Body` suffix) to avoid recursive struct definitions. See [schematic-define naming conventions](../define/README.md#naming-conventions).

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

/// Request for `ListModels` endpoint.
///
/// ## Example
///
/// ```ignore
/// use schematic_schema::openai::ListModelsRequest;
///
/// let request = ListModelsRequest::default();
/// ```
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
    /// Creates a new request with the required path parameters.
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: model.into() }
    }

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
    env_auth: Vec<String>,
    auth_strategy: schematic_define::AuthStrategy,
    env_username: Option<String>,
    headers: Vec<(String, String)>,
}

impl OpenAI {
    pub const BASE_URL: &'static str = "https://api.openai.com/v1";

    pub fn new() -> Self { ... }
    pub fn with_base_url(base_url: impl Into<String>) -> Self { ... }
    pub fn with_client(client: reqwest::Client) -> Self { ... }
    pub fn with_client_and_base_url(client: reqwest::Client, base_url: impl Into<String>) -> Self { ... }

    /// Create a variant with different configuration (base URL, credentials, auth strategy)
    pub fn variant(&self, base_url: impl Into<String>, env_auth: Vec<String>, strategy: UpdateStrategy) -> Self { ... }

    /// Access the underlying HTTP client for custom requests
    pub fn http_client(&self) -> &reqwest::Client { ... }

    /// Get the base URL for this client
    pub fn api_base_url(&self) -> &str { ... }

    /// Get the API key header name and value (if using ApiKey auth)
    pub fn api_key_header(&self) -> Option<(String, String)> { ... }

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

    // List all models (no path params - use Default)
    let models: ListModelsResponse = client
        .request(ListModelsRequest::default())
        .await?;

    // Retrieve a specific model - type-safe construction with new()
    let model: Model = client
        .request(RetrieveModelRequest::new("gpt-4"))
        .await?;

    // Alternative: struct literal (still works for flexibility)
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

// Generated struct with new() constructor
pub struct GetMessageRequest {
    pub thread_id: String,
    pub message_id: String,
}

impl GetMessageRequest {
    /// Creates a new request with the required path parameters.
    pub fn new(thread_id: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            thread_id: thread_id.into(),
            message_id: message_id.into(),
        }
    }

    pub fn into_parts(self) -> (&'static str, String, Option<String>) {
        let path = format!("/threads/{}/messages/{}", self.thread_id, self.message_id);
        ("GET", path, None)
    }
}

// Usage
let request = GetMessageRequest::new("thread-123", "msg-456");
```

## Request Bodies

Endpoints with request bodies get a `body` field and a `new()` constructor:

```rust
// API definition
Endpoint {
    request: Some(ApiRequest::json_type("CreateCompletionBody")),
    ...
}

// Generated struct with new() constructor
#[derive(Debug, Clone)]
pub struct CreateCompletionRequest {
    pub body: CreateCompletionBody,  // The body type from schema
}

impl CreateCompletionRequest {
    /// Creates a new request with the required body.
    pub fn new(body: CreateCompletionBody) -> Self {
        Self { body }
    }

    pub fn into_parts(self) -> (&'static str, String, Option<String>) {
        let path = "/completions".to_string();
        ("POST", path, serde_json::to_string(&self.body).ok())
    }
}

// Usage - type-safe construction
let request = CreateCompletionRequest::new(CreateCompletionBody {
    model: "gpt-4".to_string(),
    prompt: "Hello".to_string(),
    ..Default::default()
});
```

For endpoints with both path parameters and a body:

```rust
// POST /threads/{thread_id}/messages with JSON body
pub struct CreateMessageRequest {
    pub thread_id: String,
    pub body: CreateMessageBody,
}

impl CreateMessageRequest {
    /// Creates a new request with required path parameters and body.
    pub fn new(thread_id: impl Into<String>, body: CreateMessageBody) -> Self {
        Self {
            thread_id: thread_id.into(),
            body,
        }
    }
}

// Usage
let request = CreateMessageRequest::new("thread-123", CreateMessageBody {
    content: "Hello!".to_string(),
    ..Default::default()
});
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
| `NamingCollision` | Body type name conflicts with generated wrapper struct |
| `InvalidRequestSuffix` | Custom request suffix contains invalid characters |

### `validation`

Pre-generation validation functions:

| Function | Description |
|----------|-------------|
| `validate_api(&api)` | Validates naming collisions and request suffix format |

### `inference`

Module path inference utilities:

| Function | Description |
|----------|-------------|
| `infer_module_path(&name)` | Infers module path from API name (for recognized suffixes) |

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

## Critical Testing Requirements

> **⚠️ WARNING**: The test suite has known gaps! Read this section carefully.

### What Tests Cover

The current test suite verifies:

1. **Syntax validity** - Generated code parses as valid Rust via `syn`
2. **Formatting** - Output is properly formatted via `prettyplease`
3. **Individual generators** - Each generator function has unit tests
4. **Auth strategies** - All authentication patterns generate correct code

### What Tests DO NOT Cover

**Runtime behavior is NOT tested!** Specifically:

1. **Response type handling** - Tests don't verify that `ApiResponse::Binary` endpoints actually call `.bytes()` instead of `.json()`
2. **Module path resolution** - Tests don't verify that multi-API modules work correctly together
3. **End-to-end API calls** - No integration tests with real or mocked HTTP servers

### Known Failure Modes

| Scenario | What Goes Wrong | How to Catch It |
|----------|-----------------|-----------------|
| Binary endpoint with JSON code | Runtime failure: JSON deserialize of binary data | `grep "request_bytes" generated_file.rs` |
| Module path mismatch | Compile failure: unresolved import | `cargo check -p schematic-schema` |
| Multiple APIs same module | Compile failure: duplicate module definitions | `cargo check -p schematic-schema` |

### Required Manual Verification

When modifying response handling or module generation:

```bash
# 1. Generate code
cargo run -p schematic-gen -- --api elevenlabs --output schematic/schema/src

# 2. Verify correct methods generated for non-JSON endpoints
grep -n "pub async fn request" schematic/schema/src/elevenlabs.rs
# Should see: request<T>, request_bytes, request_text, request_empty (as applicable)

# 3. Verify convenience methods for binary endpoints
grep -n "pub async fn create_speech\|pub async fn stream_speech" schematic/schema/src/elevenlabs.rs

# 4. Check it compiles
cargo check -p schematic-schema

# 5. Run tests (necessary but not sufficient!)
cargo test -p schematic-gen
```

### Response Type Method Generation

The generator produces different methods based on `ApiResponse`:

| `ApiResponse` Variant | Method Generated | Runtime Call |
|-----------------------|------------------|--------------|
| `Json(Schema)` | `request<T>()` | `response.json::<T>()` |
| `Binary` | `request_bytes()` | `response.bytes()` |
| `Text` | `request_text()` | `response.text()` |
| `Empty` | `request_empty()` | (discards body) |

Additionally, non-JSON endpoints get **convenience methods** with specific types:

```rust
// For Binary endpoint "CreateSpeech"
pub async fn create_speech(&self, req: CreateSpeechRequest) -> Result<bytes::Bytes, SchematicError>
```

### Future Work

To close the testing gap, we need:

1. **Runtime behavior tests** - Mock HTTP server returning binary data, verify correct handling
2. **Multi-API integration tests** - Generate multiple APIs, verify they compile together
3. **E2E smoke tests** - Optional integration tests against real APIs (gated by feature flag)

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
