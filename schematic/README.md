# Schematic

<table>
  <tr>
    <td><img src="../assets/schematic-2.png" style="max-width='25%'" width=200px /></td>
    <td>
      <p>This package area includes the sub-packages that together produce strongly typed, ergonomic API clients:</p>
      <ul>
        <li><code>define</code> - <i>provides primitives for defining an API, Request and Response schemas, and REST, Websocket, or Multi-part Form Endpoints</i></li>
        <li>
            <code>definitions</code> - <i>uses the primitives from <code>define</code> to define an API surface</i>
        </li>
        <li>
            <code>gen</code> - <i>takes the definitions found in the <code>definitions</code> package and generates the request and response types for each API, plus WebSocket clients</i>
        </li>
        <li>
            <code>runtime</code> - <i>the hand-written runtime every generated REST client sends through: credentials, redirects, timeouts, and the response envelope</i>
        </li>
        <li>
            <code>schema</code> - <i>this is where the finalized API and schema definition go for use by external libraries</i>
        </li>
        <li>
            <code>oauth</code> - <i>obtains and refreshes OAuth2 tokens to pass to a client as credentials</i>
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
├── runtime/      # Request runtime for generated REST clients (managed client, typed send, envelope, credentials)
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
               ┌─────────────────────────────┐     ┌─────────────────────────────┐
               │      schematic-schema       │────►│     schematic-runtime       │
               │   (generated API clients)   │     │ (sends every REST request)  │
               └─────────────────────────────┘     └─────────────────────────────┘

                              ┌─────────────────────────────┐
                              │      schematic-oauth        │
                              │  (OAuth2 runtime library)   │
                              └─────────────────────────────┘
```

Generated REST code only describes an API: a marker type such as `OpenAI`,
an `OpenAIClient` alias, and one request struct per endpoint. Sending,
credentials, redirects, timeouts, and the response envelope live once in
`schematic-runtime`.

## Quick Start

```rust
use schematic_schema::openai::{ListModelsRequest, RetrieveModelRequest};
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAI::builder()
        .credentials(Credential::access_token(std::env::var("OPENAI_API_KEY")?))
        .build()?;

    // List all models (no required params - use Default)
    let models = client
        .send(ListModelsRequest::default(), &CallOptions::new())
        .await? // Err only when no HTTP response arrived
        .into_result()?; // Err(ApiError) for any non-2xx or undecodable reply
    println!("Found {} models", models.data.len());

    // Required path parameters go through new(), or From<&str> for one parameter
    let response = client
        .send(RetrieveModelRequest::from("gpt-4"), &CallOptions::new())
        .await?;
    println!("{} (request id {:?})", response.meta.status, response.meta.request_id);
    if let Some(model) = response.data() {
        println!("Model: {}", model.id);
    }

    println!("API docs: {:?}", OpenAI::DOCS_URL);
    Ok(())
}
```

A send returns `Ok` with a `Response` envelope whenever the server
answered, whatever the status; the envelope carries the decoded value or the
failure with its raw body, plus the status, headers, and request id.
`Err(TransportError)` means no response arrived. See
[the runtime docs](./docs/runtime/README.md).

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
| [schematic-runtime](./runtime/)         | Request runtime library        | [README](./runtime/README.md)     |
| [schematic-schema](./schema/)           | Generated API clients          | [README](./schema/README.md)      |

Topic documentation lives in [`docs/`](./docs/):

- [`docs/runtime/`](./docs/runtime/README.md) - sending requests and reading responses with a generated client
- [`docs/generation/`](./docs/generation/request-side.md) - how request structs become HTTP requests, generation validation, and partial generation
- [`docs/headers/`](./docs/headers/README.md) - credential precedence and the `Headers` builder
- [`docs/io/`](./docs/io/) - OpenAPI and Postman import and export

## Available APIs

| API | Module | Endpoints | Auth | Description |
|-----|--------|-----------|------|-------------|
| Anthropic | `anthropic` | 4 | API Key (`X-Api-Key`) | Claude Messages API with tool use |
| Bitbucket | `bitbucket` | 15 | Basic | Bitbucket Cloud API for repos, PRs, issues, tags |
| OpenAI | `openai` | 3 | Bearer | Models API (list, retrieve, delete) |
| HuggingFace Hub | `huggingface` | 28+ | Bearer | Models, datasets, spaces, repos |
| ElevenLabs | `elevenlabs` | 45+ REST, 2 WebSocket | API Key (`xi-api-key`) | TTS, voices, audio generation |
| LM Studio | `lmstudio` | 6 | Bearer (`LM_API_TOKEN`) | Local inference (OpenAI-compatible) |
| Ollama Native | `ollama` | 11 | None | Local inference (generate, chat, embed) |
| Ollama OpenAI | `ollama` | 4 | None | OpenAI-compatible subset |
| EMQX Basic | `emqx` | 36 | Basic | MQTT broker REST API |
| EMQX Bearer | `emqx` | 38 | Bearer | MQTT broker with token auth |
| GitHub | `github` | 16 | Bearer | GitHub REST API for repos, PRs, issues, releases |
| GitLab | `gitlab` | 19 | API Key (`PRIVATE-TOKEN`) | GitLab REST API for repos, MRs, issues, releases |
| Gitea | `gitea` | 16 | API Key (`token`) | Gitea REST API for self-hosted Git forges |
| Eversolo | `eversolo` | 24 | None | DMP-A8 local HTTP control (device, playback, I/O) |
| Samsung Smart TV | `samsung_smart_tv` | 4 REST, 1 WebSocket | None | S95C-focused LAN control (Smart View + remote WS) |
| Unfolded Circle | `unfolded_circle` | 11 REST, 4+1+1 WebSocket | API Key / Bearer | Core REST + Core/Dock/Integration WebSocket APIs |

APIs sharing a module (`ollama`, `emqx`) are combined into a single generated file with distinct request suffixes.

## Key Features

- **Type-safe requests**: Each endpoint gets a strongly-typed request struct with `new()` constructors
- **Compile-time enforcement**: Required path parameters and bodies are enforced via `new()` constructors, and a client accepts only its own API's requests
- **Ergonomic conversions**: `From<&str>`/`From<String>` for single-param requests, `From<Body>` for body-only requests
- **One response envelope**: every reply that arrived, whatever its status, is an `Ok(Response)` with the decoded value or the failure (raw body kept), plus status, headers, and request id; `Err(TransportError)` means no reply arrived
- **Credentials**: Bearer, API Key (header, query, or cookie), Basic, and OAuth2 tokens, supplied per call, per client, or from env var fallback chains; a missing credential sends anonymously
- **Safe redirects**: the runtime follows redirects itself under a per-API cross-origin policy and sends the credential only where that policy allows
- **Deadlines**: a per-request timeout plus a `Deadline` shared across many sends
- **Response hooks**: client-global JSON transformation (`pre_response_json`) and type-safe per-endpoint mutation (`mutate_response`)
- **Redacted metadata**: request and response metadata never stores a credential, so it is safe to log
- **Multiple response types**: JSON, Text, Binary, and Empty responses, each with its own success type (`T`, `String`, `bytes::Bytes`, `()`)
- **Per-API modules**: Each API gets its own module file with configurable paths
- **Prelude exports**: Convenient imports via `use schematic_*::prelude::*`
- **Validation**: Pre-generation checks for naming collisions and configuration errors
- **Doc examples**: Generated request structs include usage examples in doc comments
- **Future-proof enums**: Public enums that may grow are `#[non_exhaustive]`
- **OpenAPI import**: Import any OpenAPI 3.x spec to generate a typed Rust client
- **OpenAPI export**: Export existing API definitions to OpenAPI 3.0.3 specs (JSON or YAML)

## Using a Generated Client

Every REST API module exposes `{Api}::new()` for a client with every
default and `{Api}::builder()` for one you adjust. Both return
`Result<{Api}Client, BuildError>`.

```rust
use std::time::Duration;

use schematic_schema::openai::ListModelsRequest;
use schematic_schema::prelude::*;

let client = OpenAI::builder()
    .base_url("https://staging.api.com/v1")
    .timeout(Duration::from_secs(10))
    .credentials(Credential::access_token(token)) // used when a call supplies none
    // Unwrap { "data": { ... } } before decoding, for every JSON reply
    .pre_response_json(|_meta, json| Ok(json.get("data").cloned().unwrap_or(json)))
    // Adjust one endpoint's decoded value
    .mutate_response::<ListModelsRequest, _>(|_meta, models| {
        models.data.retain(|model| !model.id.contains("deprecated"));
        Ok(())
    })
    .build()?;

// A different credential for one call, or none at all
let options = CallOptions::new().credential(Credential::access_token(user_token));
let public = CallOptions::anonymous();
```

For OAuth2 APIs, obtain a token with `schematic-oauth` and pass it as an
access token; the runtime does not run OAuth flows itself:

```rust
use schematic_oauth::{MemoryTokenStore, OAuth2Manager};

let manager = OAuth2Manager::new(runtime_config, Box::new(MemoryTokenStore::new()))?;
let token = manager.get_valid_token().await?; // refreshes when expired
let client = GitHub::builder().credentials(Credential::access_token(token)).build()?;
```

The runtime never retries; a second attempt is an explicit second call. The
[runtime docs](./docs/runtime/README.md) cover the envelope and its failure
kinds, credentials, redirects and origins, deadlines, hooks, raw sends, and
redaction.

## Artifact Generation

Schematic generates three artifact types from API definitions:

1. **Rust clients** — Strongly-typed API clients in `schema/src/`
2. **OpenAPI specs** — OpenAPI 3.0.3 documents in `openapi/`
3. **Postman collections** — Postman v2.1.0 collections in `postman/`

When the output path is `schema/src`, OpenAPI and Postman artifacts are generated automatically to sibling directories. Use `--no-openapi` or `--no-postman` to suppress.

```bash
# Generate all three artifact types (default behavior)
just -f schematic/justfile generate

# Regenerate one API's module group in all artifacts; every other module is kept
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

Each request struct's `EndpointSpec` impl takes its codec and success type from the endpoint's `ApiResponse`:

| Response Type | Codec (`type Decoder`) | Success type |
|---------------|------------------------|--------------|
| `ApiResponse::Json(schema)` | `schematic_runtime::Json<T>` | `T` |
| `ApiResponse::Binary` | `schematic_runtime::Binary` | `bytes::Bytes` |
| `ApiResponse::Text` | `schematic_runtime::Text` | `String` |
| `ApiResponse::Empty` | `schematic_runtime::Empty` | `()` |

**When adding endpoints with non-JSON responses:**

1. **Verify the response type is correct** - Binary audio endpoints must use `ApiResponse::Binary`, not `ApiResponse::Json`; a JSON codec on a binary endpoint gives callers the wrong success type and reports every reply as `MalformedSuccess`
2. **Test the generated code compiles** - Run `cargo check --manifest-path schematic/schema/Cargo.toml` (the generated crate is outside the workspace)
3. **Run generation tests** - `schematic-gen` e2e tests verify the codec each endpoint declares, but they do not exercise live provider HTTP behavior

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
- ✅ Freshly generated code compiles: `just test` ends with the fresh-output guard (`just test-fresh-output`), which generates all 18 APIs and a set of fixture APIs into temporary crates outside the checkout and runs `cargo check --offline --locked` on them
- ✅ The fresh 18-API Rust output matches the committed `schema/src`, and the exported OpenAPI and Postman documents match `openapi/` and `postman/` (`just check-drift`)
- ✅ The query strings the generated Gitea, GitLab, and OpenAI clients put on the wire match recorded request lines, captured by a local listener
- ✅ Unit test coverage for individual generators

- ✅ Freshly generated fixture APIs send through the runtime against local mocks: wire bytes, credentials, codecs, hooks, and definition settings
- ✅ The runtime's own behavior (envelope, credentials, redirects, deadlines, raw sends, hooks, redaction) against loopback mocks, plus compile-fail cases for misuse

**Current tests DO NOT verify:**
- ❌ Integration with real APIs
- ❌ Runtime behavior of combined shared-module APIs

The guard needs the dependencies of `schema/Cargo.lock` in the local Cargo cache, because it never goes online. If it fails with an offline-resolution error (the failure message then starts with a `hint:` line saying so), run `cargo fetch --manifest-path schematic/schema/Cargo.toml` once from the repository root.

**Before submitting changes:**

```bash
# 1. Run the tests, including the fresh-output guard
just -f schematic/justfile test

# 2. Regenerate all schemas
just -f schematic/justfile generate

# 3. Verify generated code compiles (schematic-schema is outside the workspace)
cargo check --manifest-path schematic/schema/Cargo.toml

# 4. For response type changes, check the codec each endpoint declares:
grep -rn "type Decoder" schematic/schema/src/
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
