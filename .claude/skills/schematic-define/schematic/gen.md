# schematic-gen

Code generator that transforms REST API definitions into strongly-typed Rust client code.

## CLI Reference

```bash
# Generate client code
schematic-gen generate --api <NAME> [--output <DIR>] [--dry-run] [-v|-vv|-vvv]

# Generate with OpenAPI export
schematic-gen generate --api <NAME> --openapi-out <DIR> [--openapi-format json|yaml]

# Validate definition (no generation)
schematic-gen validate --api <NAME>

# Import from OpenAPI spec (feature-gated)
schematic-gen import --input <FILE> [--api-name <NAME>] [--module-path <PATH>] \
                     --output <DIR> [--dry-run] [--strict]

# Legacy syntax (backwards compatible)
schematic-gen --api <NAME> --output <DIR>
```

**Available APIs**: `anthropic`, `openai`, `elevenlabs`, `huggingface`, `lmstudio`, `ollama-native`, `ollama-openai`, `emqx-basic`, `emqx-bearer`, `all`

## OpenAPI Import

Import any OpenAPI 3.x specification and generate Rust client code:

```bash
# Basic import
schematic-gen import --input petstore.yaml --output generated/src

# With custom API name
schematic-gen import --input api.json --api-name MyApi --output generated/src

# Strict mode (fail on warnings)
schematic-gen import --input api.yaml --output generated/src --strict
```

### Import Options

| Flag | Description |
|------|-------------|
| `--input` | Path to OpenAPI spec (JSON or YAML) |
| `--api-name` | Override API name (derived from spec title by default) |
| `--module-path` | Override module path for generated code |
| `--output` | Output directory for generated code |
| `--dry-run` | Print generated code without writing files |
| `--strict` | Fail on any warning-level diagnostic |

### What Gets Generated

- `{api_name}.rs` - Client struct with request methods
- `types.rs` - Model structs/enums from OpenAPI schemas
- Proper Rust types from JSON Schema mappings
- Serde derives for serialization

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
├── shared.rs        # RequestParts, SchematicError, auth resolution, cookie/query helpers, reqwest re-export
├── ws_shared.rs     # WebSocket support shared by the *_ws modules
├── anthropic.rs     # A module under 2000 lines stays one file
├── lmstudio.rs
├── openai/          # A larger module is split into a directory:
│   ├── mod.rs       #   module docs and re-exports
│   ├── client.rs    #   client struct and request methods
│   ├── requests.rs  #   per-endpoint request structs
│   └── responses.rs #   (when the module has response types)
├── ollama/          # OllamaNative + OllamaOpenAI (combined, shared module)
├── emqx/            # EmqxBasic + EmqxBearer (combined, shared module)
├── elevenlabs_ws.rs # WebSocket clients
└── ...              # one entry per remaining module group
```

APIs that share a `module_path` (e.g., Ollama Native + OpenAI, EMQX Basic + Bearer)
are combined into a single module file. Each API gets its own client struct, request
enum, and request suffix to avoid naming collisions. Stale `.rs` files from previous
generations are automatically cleaned up.

## Generated Code Components

### Shared Module

```rust
// Re-export reqwest for custom requests
pub use reqwest;

// Request components tuple:
// (HTTP method, path, body, headers, unencoded declared query pairs)
pub type RequestParts = (
    &'static str,
    String,
    RequestBody,
    Vec<(String, String)>,
    Vec<(String, String)>,
);

// Encodes each query pair once and appends it; `?` or `&` follows `path`
pub fn append_query_pairs(url: &mut String, path: &str, pairs: &[(String, String)]);

// Error enum
pub enum SchematicError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    ApiError { status: u16, body: String },
    UnsupportedMethod(String),
    SerializationError(String),
    Io(String),
    MissingCredential { env_vars: Vec<String> },
    AuthenticationRequired { .. },
    Header(schematic_define::HeaderError),
    WrongResponseDecoder { .. },
    InternalError(String),
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

    /// Converts to (method, path, body, headers, query) tuple.
    pub fn into_parts(self) -> Result<RequestParts, SchematicError> {
        let path = format!("/users/{}", urlencoding::encode(&self.user_id.to_string()));
        let query: Vec<(String, String)> = Vec::new();
        Ok(("GET", path, crate::shared::RequestBody::Empty, vec![], query))
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
    auth_policy: schematic_define::AuthPolicy,
    env_username: Option<String>,
    headers: schematic_define::Headers,
    api_key_param: Option<schematic_define::SensitiveString>,
    // ...
}

// `Debug` prints only `base_url` and `auth_policy` (`shared::debug_client`).

impl MyApi {
    pub const BASE_URL: &'static str = "https://api.example.com/v1";

    // Constructors
    pub fn new() -> Self;
    pub fn with_base_url(base_url: impl Into<String>) -> Self;
    pub fn with_client(client: reqwest::Client) -> Self;
    pub fn with_client_and_base_url(client: reqwest::Client, base_url: impl Into<String>) -> Self;

    // Variant with different config
    pub fn variant(&self) -> MyApiVariantBuilder<'_>;
    pub fn variant_with(
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

`generate` validates the whole batch once per output (Rust, OpenAPI, Postman)
and writes only outputs with no error; one output's failure never blocks the
others, and any failure exits 1. Diagnostics print to stderr as
`severity=… api=… endpoint=… feature=… output=…: message`; warnings never fail.
A same-API duplicate method + normalized path (`/a/{x}` = `/a/{y}`) is an
error in every output, with no exception list (`endpoint_uniqueness_key`).
Grouped OpenAPI documents are built in memory for every module before any is
written (`export::openapi::build_openapi_grouped`); a later group member's
clashing `operationId` gets its API-name prefix, and the final
`operation_id_diagnostics` scan rejects any duplicate that remains.
Full rules: `schematic/docs/generation/validation.md`.

```bash
$ schematic-gen validate --api gitea
[WARN] severity=warning api=Gitea endpoint=GetGitTreeRecursive feature=query-in-path output=rust: ...
  [PASS] rust
  [PASS] openapi
  [PASS] postman
```

## Generator Modules

| Module | Function | Purpose |
|--------|----------|---------|
| `api_struct/` | `generate_api_struct()` | Client struct with constructors/accessors |
| `client/` | `generate_request_method()` | Async `request()` method with auth |
| `client/shared_auth.rs` | `generate_request_auth_helpers()` | `shared::resolve_request_auth` and the other auth helpers emitted once into `shared.rs` |
| `error.rs` | `generate_error_type()` | SchematicError enum, `append_query_pairs`, cookie helpers |
| `request_enum.rs` | `generate_request_enum()` | Unified request enum |
| `request_structs/` | `generate_request_struct()` | Per-endpoint structs; `named_params.rs` holds header/cookie parameter fields |
| `module_docs.rs` | `ModuleDocBuilder` | Module documentation |
| `import_pipeline.rs` | `run_import()` | OpenAPI import orchestration |
| `model_gen.rs` | `generate_models()` | Generate types.rs from ModelCatalog |

## Library API

```rust
use schematic_gen::output::{generate_and_write, generate_and_write_all};
use schematic_gen::validate_rust_client;
use schematic_definitions::openai::define_openai_api;

// Single API (generate_and_write also refuses an invalid batch on its own)
let api = define_openai_api();
for diagnostic in validate_rust_client(&[&api]) {
    eprintln!("{diagnostic}");
}
generate_and_write(&api, Path::new("schema/src"), false)?;

// Full generation: `apis` is the whole crate; obsolete generated modules go
let apis = vec![&api1, &api2, &api3];
generate_and_write_all(&apis, output_dir, dry_run)?;

// Partial generation: regenerate the selected APIs' complete module groups,
// keep every other module of the existing output byte-for-byte
use schematic_gen::output::{RustOutputRequest, Selection, prepare_rust_output};
let prepared = prepare_rust_output(&RustOutputRequest {
    registry: &all_apis,                 // every API, registry order
    selection: Selection::Apis(&[&api]), // or Selection::All
    output_dir,
    manifest_dir: Some(schema_dir),      // writes Cargo.toml too
})?;                                     // nothing written yet
prepared.plan.publish()?;                // reports completed/pending on I/O failure
```

Preparation checks ownership of every path it would write or delete (the
generated `.rs` header, the `Cargo.toml` marker line, export artifact names;
symlinks never followed) and, for a partial run, the existing inventory
(`lib.rs`/`prelude.rs` parse and match current definitions; `shared.rs` and
`ws_shared.rs` unchanged while modules are retained). See
`schematic/docs/generation/partial-generation.md`.

## Error Types

```rust
pub enum GeneratorError {
    ParseError(String),
    CodeGenError(String),
    WriteError { path: String, source: std::io::Error },
    OutputDirNotFound(String),
    ConfigError(String),
    // An output's validator reported errors; nothing of it was written.
    Validation { output: Output, diagnostics: Vec<ValidationDiagnostic> },
    // An existing output cannot be updated safely; nothing was changed.
    UnsafeOutput { root: String, problems: Vec<String>, remedy: String },
    // A partial run would change shared runtime under retained modules.
    RequiresFullGeneration { root: String, changed: Vec<String>, retained: Vec<String> },
    // A write or deletion failed after publication started.
    PublicationFailed { root: String, failed: String, completed: Vec<String>, pending: Vec<String>, source: Box<GeneratorError> },
    // A generate run where at least one requested output failed.
    OutputsFailed { succeeded: Vec<Output>, failed: Vec<Output> },
}
```

## OpenAPI Export

Generate OpenAPI 3.0.3 specification from existing API definitions:

```bash
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml
```

This exports the API definition for external tools (Swagger UI, code generators for other languages, etc.).

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
| Shared module missing `request_suffix` | Compile error: duplicate struct names | `cargo check` |
