# Schematic

<table>
  <tr>
    <td><img src="../assets/schematic-2.png" style="max-width='25%'" width=200px /></td>
    <td>
      <p>This package includes four sub-packages which are all aligned to create strongly typed, ergonomic API clients:</p>
      <ul>
        <li><code>define</code> - <i>provides primitives for defining an API, Request and Response schemas, and REST, Websocket, or Multi-part Form Endpoints</i></li>
        <li>
            <code>definitions</code> - <i>uses the primitives from <code>define</code> to define an API surface</i>
        </li>
        <li>
            <code>gen</code> - <i>takes the definitions found in the <code>definitions</code> package and generates structs and enums to represent these API definitions including a fully functioning network client</i>
        </li>
        <li>
            <code>schema</code> - <i>this is where the finalized API and schema definition go for use by external libraries</i>
        </li>
      </ul>
      <p></p>
    </td>
  </tr>
</table>

## Architecture

```sh
schematic/
├── define/       # Primitives for describing REST APIs (types, auth, endpoints)
├── definitions/  # Actual API definitions using those primitives (OpenAI, etc.)
├── gen/          # Code generator binary and library
├── oauth/        # OAuth2 runtime (token lifecycle, storage, manager)
└── schema/       # Generated API clients ready for consumption
```

## Workflow

```txt
┌─────────────────────────────┐     ┌─────────────────────────────┐
│      schematic-define       │     │   schematic-definitions     │
│  (primitives: RestApi,      │◄────│  (actual APIs: Anthropic,   │
│   Endpoint, AuthStrategy)   │     │   OpenAI, ElevenLabs, etc.) │
└──────────────┬──────────────┘     └──────────────┬──────────────┘
               │                                   │
               └───────────────┬───────────────────┘
                               │
                               ▼
               ┌─────────────────────────────┐
               │       schematic-gen         │
               │    (code generator CLI)     │
               └──────────────┬──────────────┘
                              │
                              ▼
               ┌─────────────────────────────┐
               │      schematic-schema       │
               │   (generated API clients)   │
               └─────────────────────────────┘

                              ┌─────────────────────────────┐
                              │      schematic-oauth        │
                              │  (OAuth2 runtime library)   │
                              └─────────────────────────────┘
```

## Quick Start

```rust
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), SchematicError> {
    let client = OpenAI::new();

    // List all models (no required params - use Default)
    let models: ListModelsResponse = client
        .request(ListModelsRequest::default())
        .await?;

    println!("Found {} models", models.data.len());

    // Retrieve a specific model - type-safe construction with new()
    let model: Model = client
        .request(RetrieveModelRequest::new("gpt-4"))
        .await?;

    // Or use From<&str> for single-param requests
    let model: Model = client
        .request(RetrieveModelRequest::from("gpt-4"))
        .await?;

    // Access documentation URL
    println!("API docs: {:?}", OpenAI::DOCS_URL);
    println!("Model: {}", model.id);
    Ok(())
}
```

## WebSocket Quick Start

```rust
use std::time::Duration;

use schematic_schema::elevenlabs_ws::{
    ElevenLabsTTSWs, TextToSpeechConnectionParams,
};
use schematic_schema::ws_shared::WsClientOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut api = ElevenLabsTTSWs::new();
    api.headers = api.headers.clone().use_api_key("my-key", "xi-api-key");

    let params = TextToSpeechConnectionParams {
        model_id: Some("eleven_turbo_v2_5".to_string()),
        ..Default::default()
    };

    let options = WsClientOptions::builder()
        .request_timeout(Duration::from_secs(10))
        .disable_nagle(true)
        .build();

    let ws = api
        .connect_text_to_speech("voice_id_here", params, options)
        .await?;

    ws.send(serde_json::json!({"text": "Hello from Schematic"})).await?;
    ws.close().await?;
    Ok(())
}
```

## Packages

| Package                                 | Description                    | Details                           |
|-----------------------------------------|--------------------------------|-----------------------------------|
| [schematic-define](./define/)           | REST API definition primitives | [README](./define/README.md)      |
| [schematic-definitions](./definitions/) | Pre-built API definitions      | [README](./definitions/README.md) |
| [schematic-gen](./gen/)                 | Code generator CLI/library     | [README](./gen/README.md)         |
| [schematic-oauth](./oauth/)             | OAuth2 runtime library         | Token lifecycle, storage          |
| [schematic-schema](./schema/)           | Generated API clients          | [README](./schema/README.md)      |

## Available APIs

| API | Module | Endpoints | Auth | Description |
|-----|--------|-----------|------|-------------|
| Anthropic | `anthropic` | 4 | API Key (`X-Api-Key`) | Claude Messages API with tool use |
| Bitbucket | `bitbucket` | 14 | Basic | Bitbucket Cloud API for repos, PRs, issues, tags |
| OpenAI | `openai` | 3 | Bearer | Models API (list, retrieve, delete) |
| HuggingFace Hub | `huggingface` | 28+ | Bearer | Models, datasets, spaces, repos |
| ElevenLabs | `elevenlabs` | 45+ REST, 2 WebSocket | API Key (`xi-api-key`) | TTS, voices, audio generation |
| LM Studio | `lmstudio` | 6 | Bearer (`LM_API_TOKEN`) | Local inference (OpenAI-compatible) |
| Ollama Native | `ollama` | 11 | None | Local inference (generate, chat, embed) |
| Ollama OpenAI | `ollama` | 4 | None | OpenAI-compatible subset |
| EMQX Basic | `emqx` | 36 | Basic | MQTT broker REST API |
| EMQX Bearer | `emqx` | 38 | Bearer | MQTT broker with token auth |
| GitHub | `github` | 14 | Bearer | GitHub REST API for repos, PRs, issues, releases |
| GitLab | `gitlab` | 15 | API Key (`PRIVATE-TOKEN`) | GitLab REST API for repos, MRs, issues, releases |
| Gitea | `gitea` | 14 | API Key (`token`) | Gitea REST API for self-hosted Git forges |
| Eversolo | `eversolo` | 24 | None | DMP-A8 local HTTP control (device, playback, I/O) |
| Samsung Smart TV | `samsung_smart_tv` | 4 REST, 1 WebSocket | None | S95C-focused LAN control (Smart View + remote WS) |
| Unfolded Circle | `unfolded_circle` | 11 REST, 4+1+1 WebSocket | API Key / Bearer | Core REST + Core/Dock/Integration WebSocket APIs |

APIs sharing a module (`ollama`, `emqx`) are combined into a single generated file with distinct request suffixes.

## Key Features

- **Type-safe requests**: Each endpoint gets a strongly-typed request struct with `new()` constructors
- **Compile-time enforcement**: Required path parameters and bodies are enforced via `new()` constructors
- **Ergonomic conversions**: `From<&str>`/`From<String>` for single-param requests, `From<Body>` for body-only requests
- **Automatic authentication**: Bearer, API Key, Basic, and OAuth2 auth with env var fallback chains
- **Runtime configuration**: `DOCS_URL` constant on API structs, `variant()` builder for alternate environments
- **Response hooks**: Pre-response JSON transformation and type-safe post-response mutation via `VariantBuilder`
- **Proper error handling**: `MissingCredential` errors with documented error handling patterns
- **Path parameters**: `{param}` syntax in paths become struct fields with `impl Into<String>` for ergonomic usage
- **Multiple response types**: JSON, Text, Binary, and Empty responses with type-specific methods and `#[must_use]` attributes
- **Per-API modules**: Each API gets its own module file with configurable paths
- **Prelude exports**: Convenient imports via `use schematic_*::prelude::*`
- **Validation**: Pre-generation checks for naming collisions and configuration errors
- **Doc examples**: Generated request structs include usage examples in doc comments
- **Future-proof enums**: All public enums use `#[non_exhaustive]` for backward-compatible extension
- **OpenAPI import**: Import any OpenAPI 3.x spec to generate a typed Rust client
- **OpenAPI export**: Export existing API definitions to OpenAPI 3.0.3 specs (JSON or YAML)

## Variant Builder & Response Hooks

The `variant()` builder creates alternate API client configurations with optional response hooks. This enables environment switching, staging/production setups, and response transformation.

### Basic Variant

```rust
use schematic_define::UpdateStrategy;

let client = OpenAI::new();

// Simple environment switch
let staging = client.variant_with(
    "https://staging.api.com/v1",
    vec!["STAGING_API_KEY".to_string()],
    UpdateStrategy::NoChange,
);
```

### Programmatic Authentication

Inject tokens programmatically without requiring environment variables:

```rust
use schematic_define::Headers;

// Token from runtime source (Vault, OAuth, config file, etc.)
let token = get_token_from_somewhere();

// Create client with programmatic token - no env vars needed!
let client = OpenAI::new()
    .variant()
    .headers_builder(Headers::default().use_bearer_token(token))
    .build();
```

When `Headers` has an authorization set via `use_bearer_token()` or `use_basic_auth()`, the env-based auth check is automatically skipped. No need to also set `AuthStrategy::None`.

### OAuth2 Authentication

For APIs using OAuth2, obtain a token via `schematic-oauth` and inject it programmatically:

```rust
use schematic_oauth::{OAuth2Manager, OAuth2RuntimeConfig, MemoryTokenStore};
use schematic_define::Headers;

// Configure OAuth2 manager from API's OAuth2Config
let manager = OAuth2Manager::new(runtime_config, Box::new(MemoryTokenStore::new()));

// Get a valid token (refreshes automatically if expired)
let token = manager.get_valid_token().await?;

// Inject into any API client
let client = GitHub::new()
    .variant_with_headers(Headers::default().use_bearer_token(token));
```

### Builder Pattern with Hooks

```rust
let staging = client.variant()
    .base_url("https://staging.api.com/v1")
    .env_auth(vec!["STAGING_API_KEY".to_string()])
    .auth_update(UpdateStrategy::NoChange)
    // Pre-response: transform raw JSON before deserialization
    .pre_response_json(|ctx, json| {
        // Unwrap envelope: { "data": { ... } } → { ... }
        if let Some(inner) = json.get("data").cloned() {
            Ok(inner)
        } else {
            Ok(json)
        }
    })
    // Post-response: mutate typed response after deserialization
    .mutate_response::<ListModelsRequest>(|ctx, response| {
        response.data.retain(|m| !m.id.contains("deprecated"));
        Ok(())
    })
    .build();
```

### Hook Types

| Hook | Signature | Runs |
|------|-----------|------|
| `pre_response_json` | `Fn(&ResponseContext, Value) → Result<Value>` | Before deserialization, on raw JSON |
| `mutate_response::<R>` | `Fn(&ResponseContext, &mut R::Response) → Result<()>` | After deserialization, per-endpoint |

- **`ResponseContext`** provides: `endpoint_id`, `method`, `path`, `url`, `status`, `headers`
- **`EndpointSpec`** trait on request structs enables type-safe `mutate_response` registration
- Hooks are stored in `Arc` and the variant is `Clone`-able

## Artifact Generation

Schematic generates three artifact types from API definitions:

1. **Rust clients** — Strongly-typed API clients in `schema/src/`
2. **OpenAPI specs** — OpenAPI 3.0.3 documents in `openapi/`
3. **Postman collections** — Postman v2.1.0 collections in `postman/`

When the output path is `schema/src`, OpenAPI and Postman artifacts are generated automatically to sibling directories. Use `--no-openapi` or `--no-postman` to suppress.

```bash
# Generate all three artifact types (default behavior)
just -f schematic/justfile generate

# Generate a single API with all artifacts
just -f schematic/justfile generate-one openai

# Only OpenAPI specs
just -f schematic/justfile generate-openapi

# Only Postman collections
just -f schematic/justfile generate-postman

# Check for artifact drift
just -f schematic/justfile check-drift
```

### Output Layout

```
schematic/
├── schema/src/       # Generated Rust API clients
│   ├── lib.rs
│   ├── openai.rs
│   ├── ollama.rs     # Grouped: OllamaNative + OllamaOpenAI
│   └── ...
├── openapi/          # OpenAPI 3.0.3 specs (JSON)
│   ├── openai.json
│   ├── ollama.json   # Grouped
│   ├── emqx.json     # Grouped
│   ├── huggingface.json
│   ├── samsung_smart_tv.json
│   └── ...
└── postman/          # Postman v2.1.0 collections
    ├── openai.postman_collection.json
    ├── ollama.postman_collection.json  # Grouped
    └── ...
```

APIs sharing a module (`ollama`, `emqx`) produce grouped artifacts that merge all endpoints.

## OpenAPI Support

Schematic supports bidirectional OpenAPI 3.x integration: import external specs to generate clients, or export existing definitions to OpenAPI format.

### Importing OpenAPI Specs

Transform any OpenAPI 3.x specification into a type-safe Rust client:

```bash
# Basic import
schematic-gen import --input petstore.yaml --output generated/src

# With custom API name and strict diagnostics
schematic-gen import --input api.json --api-name MyPetStore --output src --strict

# Dry run (preview without writing)
schematic-gen import --input api.yaml --output src --dry-run
```

| Option | Description |
|--------|-------------|
| `--input` | Path to OpenAPI spec file (JSON or YAML) |
| `--api-name` | Override API name (default: derived from spec title) |
| `--module-path` | Override module path for generated code |
| `--output` | Output directory for generated code |
| `--dry-run` | Preview generated code without writing files |
| `--strict` | Fail on any warning-level diagnostic |

### Exporting to OpenAPI

Generate OpenAPI 3.0.3 specs from existing Schematic API definitions:

```bash
# Export as JSON (default)
schematic-gen generate --api openai --openapi-out specs/

# Export as YAML
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml

# Override version
schematic-gen generate --api openai --openapi-version 2.0.0

# Generate all with OpenAPI export
schematic-gen generate --api all --openapi-out specs/
```

Version resolution: `--openapi-version` > `RestApi.version` > `"0.1.0"` fallback.

Exported specs include `x-schematic` extensions for round-trip fidelity (module path, request suffix, env mapping, per-endpoint type names).

> **Feature Gate**: OpenAPI functionality requires the `openapi` feature in `schematic-define`. The `schematic-gen` and `schematic-definitions` crates enable this feature by default.

## Critical Development Requirements

> **⚠️ IMPORTANT**: Read this section before modifying schematic packages!

### 1. Response Type Verification

The generator produces different methods based on `ApiResponse` types:

| Response Type | Generated Method | Return Type |
|---------------|------------------|-------------|
| `ApiResponse::Json(schema)` | `request<T>()` | `Result<T, SchematicError>` |
| `ApiResponse::Binary` | `request_bytes()` | `Result<bytes::Bytes, SchematicError>` |
| `ApiResponse::Text` | `request_text()` | `Result<String, SchematicError>` |
| `ApiResponse::Empty` | `request_empty()` | `Result<(), SchematicError>` |

**When adding endpoints with non-JSON responses:**

1. **Verify the response type is correct** - Binary audio endpoints must use `ApiResponse::Binary`, not `ApiResponse::Json`
2. **Test the generated code compiles** - Run `cargo check -p schematic-schema`
3. **Run generation tests** - `schematic-gen` e2e tests verify response-method generation (`response.bytes()`, `response.text()`, etc.), but they do not exercise live provider HTTP behavior

### 2. Module Path Configuration

The generator groups APIs by `module_path` — APIs sharing a path are combined into a single output file.

| Scenario | Configuration Required |
|----------|------------------------|
| Single API per module | `module_path: None` (auto-inferred) |
| Multiple APIs sharing one definitions module | **REQUIRES explicit `module_path` and `request_suffix`** |
| API name differs from definitions module | **REQUIRES explicit `module_path`** |

**Example - Ollama has two APIs sharing one definitions module:**

```rust
// ✅ CORRECT - Both use explicit path and distinct suffixes
RestApi { name: "OllamaNative".to_string(), module_path: Some("ollama".to_string()),
          request_suffix: Some("NativeRequest".to_string()), ... }
RestApi { name: "OllamaOpenAI".to_string(), module_path: Some("ollama".to_string()),
          request_suffix: Some("OaiRequest".to_string()), ... }
// → Generates single ollama.rs with both OllamaNative and OllamaOpenAI clients
```

### 3. Testing Requirements

**Current tests verify:**
- ✅ Generated code is syntactically valid Rust
- ✅ Code compiles (`cargo check`)
- ✅ Unit test coverage for individual generators

**Current tests DO NOT verify:**
- ❌ Runtime behavior (binary responses actually call `.bytes()`)
- ❌ Integration with real APIs
- ❌ Runtime behavior of combined shared-module APIs

**Before submitting changes:**

```bash
# 1. Run unit tests
cargo test -p schematic-define -p schematic-definitions -p schematic-gen

# 2. Regenerate all schemas
just -f schematic/justfile generate

# 3. Verify generated code compiles
cargo check -p schematic-schema

# 4. For response type changes, manually verify correct method is generated:
grep -n "request_bytes\|request_text\|request_empty" schematic/schema/src/*.rs
```

## Building

All operations are done using the _justfile_ and the `just` runner:

```bash
# Build all schematic packages
just -f schematic/justfile build
# Run tests
just -f schematic/justfile test
# Run linter
just -f schematic/justfile lint
# Generate API clients
just -f schematic/justfile generate
# Validate API definitions (without generating)
schematic-gen validate --api openai
# Full workflow: generate and verify
just -f schematic/justfile full
```

### CLI Subcommands

The `schematic-gen` CLI supports three subcommands:

```bash
# Validate an API definition
schematic-gen validate --api openai

# Generate client code (validates first)
schematic-gen generate --api openai --output ./output

# Generate with OpenAPI spec export
schematic-gen generate --api openai --output ./output --openapi-out specs/ --openapi-format yaml

# Import from an OpenAPI 3.x spec
schematic-gen import --input petstore.yaml --output ./output

# Import with overrides
schematic-gen import --input api.json --api-name MyApi --output ./output --strict
```

## License

AGPL-3.0-only
