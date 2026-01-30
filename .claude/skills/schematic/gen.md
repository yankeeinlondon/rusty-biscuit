# schematic-gen

Code generator that transforms REST API definitions into strongly-typed Rust client code.

## CLI Reference

```bash
# Generate client code
schematic-gen generate --api <NAME> [--output <DIR>] [--dry-run] [-v|-vv|-vvv]

# Validate definition (no generation)
schematic-gen validate --api <NAME>

# Legacy syntax (backwards compatible)
schematic-gen --api <NAME> --output <DIR>
```

**Available APIs**: `anthropic`, `openai`, `elevenlabs`, `huggingface`, `ollama-native`, `ollama-openai`, `emqx-basic`, `emqx-bearer`, `all`

**Note**: `all` excludes Ollama and EMQX (shared module dependencies require individual generation).

## Generation Pipeline

```
API Definition (RestApi)
        │
        ▼
┌──────────────────┐
│ Phase 1: Codegen │  generate_error_type(), generate_request_struct(),
│                  │  generate_request_enum(), generate_api_struct(),
│                  │  generate_request_method()
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 2: Assembly│  Combine TokenStreams, add imports, lint attrs
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 3: Validate│  Parse with syn to verify syntax
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 4: Format  │  prettyplease for consistent style
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 5: Output  │  Atomic write (temp + rename), Cargo.toml
└──────────────────┘
```

## Generated Output Structure

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

## Generated Code Components

### Shared Module

```rust
// Re-export reqwest for custom requests
pub use reqwest;

// Request components tuple
pub type RequestParts = (&'static str, String, Option<String>, Vec<(String, String)>);
// (HTTP method, path, optional JSON body, headers)

// Error enum
pub enum SchematicError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    ApiError { status: u16, body: String },
    UnsupportedMethod(String),
    SerializationError(String),
    MissingCredential { env_vars: Vec<String> },
}
```

### Per-Endpoint Request Struct

```rust
/// Request for `GetUser` endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetUserRequest {
    /// Path parameter: user_id
    pub user_id: String,
}

impl GetUserRequest {
    /// Creates a new request with required path parameters.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self { user_id: user_id.into() }
    }

    /// Converts to (method, path, body, headers) tuple.
    pub fn into_parts(self) -> Result<RequestParts, SchematicError> {
        let path = format!("/users/{}", self.user_id);
        Ok(("GET", path, None, vec![]))
    }
}
```

### API Client Struct

```rust
pub struct MyApi {
    client: reqwest::Client,
    base_url: String,
    env_auth: Vec<String>,
    auth_strategy: schematic_define::AuthStrategy,
    env_username: Option<String>,
    headers: Vec<(String, String)>,
}

impl MyApi {
    pub const BASE_URL: &'static str = "https://api.example.com/v1";

    // Constructors
    pub fn new() -> Self;
    pub fn with_base_url(base_url: impl Into<String>) -> Self;
    pub fn with_client(client: reqwest::Client) -> Self;
    pub fn with_client_and_base_url(client: reqwest::Client, base_url: impl Into<String>) -> Self;

    // Variant with different config
    pub fn variant(
        &self,
        base_url: impl Into<String>,
        env_auth: Vec<String>,
        strategy: UpdateStrategy,
    ) -> Self;

    // Accessors
    pub fn http_client(&self) -> &reqwest::Client;
    pub fn api_base_url(&self) -> &str;
    pub fn api_key_header(&self) -> Option<(String, String)>;

    // Request methods
    pub async fn request<T: DeserializeOwned>(&self, req: impl Into<MyApiRequest>) -> Result<T, SchematicError>;
    pub async fn request_bytes(&self, req: impl Into<MyApiRequest>) -> Result<bytes::Bytes, SchematicError>;
    pub async fn request_text(&self, req: impl Into<MyApiRequest>) -> Result<String, SchematicError>;
    pub async fn request_empty(&self, req: impl Into<MyApiRequest>) -> Result<(), SchematicError>;
}

impl Default for MyApi {
    fn default() -> Self { Self::new() }
}
```

## Validation

Pre-generation checks:

1. **Request suffix format**: Custom `request_suffix` must be alphanumeric
2. **Naming collisions**: Body type names must not match generated `{EndpointId}Request`

```bash
$ schematic-gen validate --api openai
  [PASS] Request suffix format
  [PASS] No naming collisions detected

[OK] All validation checks passed for 'OpenAI'
```

## Generator Modules

| Module | Function | Purpose |
|--------|----------|---------|
| `api_struct.rs` | `generate_api_struct()` | Client struct with constructors/accessors |
| `client.rs` | `generate_request_method()` | Async `request()` method with auth |
| `error.rs` | `generate_error_type()` | SchematicError enum |
| `request_enum.rs` | `generate_request_enum()` | Unified request enum |
| `request_structs.rs` | `generate_request_struct()` | Per-endpoint structs |
| `module_docs.rs` | `ModuleDocBuilder` | Module documentation |

## Library API

```rust
use schematic_gen::output::{generate_and_write, generate_and_write_all};
use schematic_gen::validate_api;
use schematic_definitions::openai::define_openai_api;

// Single API
let api = define_openai_api();
validate_api(&api)?;
generate_and_write(&api, Path::new("schema/src"), false)?;

// Multiple APIs
let apis = vec![&api1, &api2, &api3];
generate_and_write_all(&apis, output_dir, dry_run)?;
```

## Error Types

```rust
pub enum GeneratorError {
    ParseError(String),
    CodeGenError(String),
    WriteError { path: PathBuf, source: std::io::Error },
    OutputDirNotFound(PathBuf),
    ConfigError(String),
    NamingCollision {
        endpoint_id: String,
        body_type: String,
        suggestion: String,
    },
    InvalidRequestSuffix {
        suffix: String,
        reason: String,
    },
}
```

## Testing Gap Warning

**Unit tests verify syntax, NOT runtime behavior!**

After modifying response handling:

```bash
# 1. Tests pass (necessary but not sufficient)
cargo test -p schematic-gen

# 2. Generate code
just -f schematic/justfile generate

# 3. Verify compilation
cargo check -p schematic-schema

# 4. MANUALLY verify correct methods for non-JSON endpoints:
grep -n "request_bytes\|request_text\|request_empty" schematic/schema/src/*.rs
```

| Scenario | Failure Mode | Detection |
|----------|--------------|-----------|
| Binary endpoint with JSON code | Runtime JSON parse error | Manual grep |
| Module path mismatch | Compile error: unresolved import | `cargo check` |
| Multiple APIs same module | Compile error: duplicate module | `cargo check` |
