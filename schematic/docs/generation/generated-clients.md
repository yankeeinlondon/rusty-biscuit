---
kind: canonical-documentation
---

# Generator and generated client reference

Code generator that transforms REST and WebSocket API definitions into strongly-typed Rust client code.

## Overview

`schematic-gen` takes API definitions created with `schematic-define` and generates strongly-typed Rust HTTP and WebSocket client code. Generated REST clients are thin: they describe each API and each endpoint, and the hand-written [`schematic-runtime`](../../runtime/) crate sends the requests. The generated code includes:

- Per API, one client struct named after the API (`OpenAI`), the only name a caller writes, wrapping `schematic_runtime::Client<OpenAIApi>`; an infallible `new()` that hands the runtime the API's base URL, credential placements, and policies; and a tag type (`OpenAIApi`) that the API's requests name, so the client sends only its own API's requests. The runtime's `api_client!` macro gives the struct every client-wide setting and send
- Per endpoint, a request struct (with path, query, header, and cookie parameters as fields) implementing `schematic_runtime::EndpointSpec`, which names its API, its success type, and the codec that decodes the response
- Per REST endpoint, an inherent method on the API struct (`list_models`, `retrieve_model`) that takes the required values and returns a `{Name}Call` (`RetrieveModelCall`): one setter per optional value plus the shared per-call forwards (`anonymous`, `credential`, `deadline`, `full`, `build`), sent when it is awaited
- A compile-time check that the generated crate and the runtime it builds against agree on the runtime ABI

Sending (`Call`, `FullCall`, the generic `send`, `send_raw`), the response envelope, `CallError` and transport errors, credential resolution, redirects, deadlines, response hooks, and redaction all live in the runtime; see its crate documentation in [`runtime/src/lib.rs`](../../runtime/src/lib.rs).

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

The CLI provides three subcommands: `generate`, `validate`, and `import`.

```bash
# Regenerate the OpenAI API's module group; every other module is kept
schematic-gen generate --api openai --output schematic/schema/src

# Regenerate every API (full generation)
schematic-gen generate --api all --output schematic/schema/src

# Generate with OpenAPI spec export
schematic-gen generate --api openai --output schematic/schema/src --openapi-out specs/
schematic-gen generate --api openai --output schematic/schema/src --openapi-out specs/ --openapi-format yaml

# Validate an API definition (without generating code)
schematic-gen validate --api openai

# Import from an OpenAPI 3.x spec
schematic-gen import --input petstore.yaml --output generated/src
schematic-gen import --input api.json --api-name MyApi --output src --strict

# Import into schematic-definitions, keeping a hand-written overlay
# (see ../docs/io/import-openapi.md)
schematic-gen import --input api.yaml --api-name MyApi --module-path my_api \
    --definitions-out definitions/src/my_api --overlay crate::my_api_overlay::apply

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
| `generate` | Generate the Rust client, OpenAPI, and Postman outputs (each validated first) |
| `validate` | Validate API definition without generating code |
| `import` | Import an OpenAPI 3.x spec and generate Rust client code |

### Generate Options

| Flag | Description |
|------|-------------|
| `-a, --api <NAME>` | API definition to generate (e.g., `openai`) |
| `-o, --output <DIR>` | Output directory for generated code (default: `schematic/schema/src`) |
| `--dry-run` | Print generated code without writing files |
| `--openapi-out <DIR>` | Output directory for OpenAPI spec export (auto: `openapi/`) |
| `--openapi-format <FORMAT>` | OpenAPI output format: `json` (default) or `yaml` |
| `--openapi-version <VER>` | Override version in OpenAPI spec |
| `--postman-out <DIR>` | Output directory for Postman collections (auto: `postman/`) |
| `--no-openapi` | Skip OpenAPI specification generation |
| `--no-postman` | Skip Postman collection generation |
| `-v, --verbose` | Increase verbosity level |

When the output path ends with `schema/src`, OpenAPI and Postman output directories default to `openapi/` and `postman/` (sibling to `schema/`). Use `--no-openapi` or `--no-postman` to suppress.

### Partial Generation

`--api <name>` regenerates that API's whole module group (selecting
`ollama-native` also regenerates `OllamaOpenAI`, which shares the `ollama`
module) and keeps every other module, OpenAPI document, and Postman
collection byte-for-byte. `lib.rs` and `prelude.rs` are rebuilt from the kept
and regenerated groups. `--api all` regenerates everything and deletes
generated modules no API produces.

Before writing, each output checks the paths it would change: it never
overwrites or deletes a file the generator did not write, and a partial run
refuses an existing crate it cannot account for (a corrupt or hand-edited
`lib.rs`, a missing or unknown module, exports that no longer match the
definitions, a changed `shared.rs` or `ws_shared.rs`, or a `lib.rs` whose recorded
`SCHEMATIC_RUNTIME_ABI` is missing or differs from the generator's) without
changing anything. If a write
fails midway, the error lists completed and pending writes and deletions.
See [`docs/generation/partial-generation.md`](../generation/partial-generation.md).

### Import Options

| Flag | Description |
|------|-------------|
| `-i, --input <FILE>` | Path to OpenAPI spec file (JSON or YAML) |
| `--api-name <NAME>` | Override API name (default: derived from spec title) |
| `--module-path <PATH>` | Override module path for generated code |
| `-o, --output <DIR>` | Output directory for generated code |
| `--dry-run` | Print generated code without writing files |
| `--strict` | Fail on any WARN or ERROR diagnostic; INFO never fails |
| `-v, --verbose` | Increase verbosity level |

### Validation

`generate` validates the whole batch separately for each output (Rust client,
OpenAPI, Postman) and writes only the outputs whose validator reports no
error; one output's failure never stops the others, and any failed output
makes the command exit 1. `validate` runs the same three validators without
writing anything:

```bash
$ schematic-gen validate --api gitea
[WARN] severity=warning api=Gitea endpoint=GetGitTreeRecursive feature=query-in-path output=rust: path template `/repos/{owner}/{repo}/git/trees/{sha}?recursive=true` embeds a query string; declare it as query parameters so it can be separated from the path
[WARN] ... output=openapi: ...
[WARN] ... output=postman: ...
  [PASS] rust
  [PASS] openapi
  [PASS] postman
```

Warnings never fail a run. Two endpoints of one API on the same method and
path (placeholder names ignored) are an error in every output; merge them
into one endpoint. The diagnostic format, every check, and which
output applies it are described in
[`docs/generation/validation.md`](../generation/validation.md).

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
    generate_request_struct,
    generate_request_struct_with_options,
};
use schematic_gen::output::{validate_code, format_code, assemble_api_code};

// Generate just the API's client struct, tag type, and constructor
let api_tokens = generate_api_struct(&api);

// Generate a single request struct; its `EndpointSpec::Api` names the
// placeholder marker `Api` (`PLACEHOLDER_API_MARKER`)
let request_tokens = generate_request_struct(&endpoint);

// The same, scoped to a real API's tag type, as generated modules do
let request_tokens =
    generate_request_struct_with_options(&endpoint, "Request", Some("openai"), "OpenAIApi");

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
│  │ Request       │ │ EndpointSpec  │ │ API Client Struct,    │  │
│  │ Structs       │ │ Impls         │ │ Tag, Constructor      │  │
│  └───────────────┘ └───────────────┘ └───────────────────────┘  │
│  ┌───────────────┐ ┌───────────────┐ ┌───────────────────────┐  │
│  │ Shared Request│ │ Pager         │ │ Module Docs           │  │
│  │ Helpers       │ │ Impls         │ │ Generation            │  │
│  └───────────────┘ └───────────────┘ └───────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 2: Assembly                             │
│  - Combine all TokenStreams                                      │
│  - Add module documentation                                      │
│  - Add imports and lint attributes                               │
│  - Generate lib.rs (with the runtime ABI check), shared.rs,      │
│    prelude.rs                                                    │
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
│  - Check every planned path's ownership before the first write   │
│  - Atomic per-file writes; lib.rs last                           │
│  - Generate Cargo.toml for schema package                        │
│  - Delete obsolete generator-owned paths                         │
└─────────────────────────────────────────────────────────────────┘
```

## Generated Output Structure

The generator produces per-API module files:

```
schema/src/
├── lib.rs           # Module declarations, SCHEMATIC_RUNTIME_ABI and its compile-time check, `pub use schematic_runtime`
├── prelude.rs       # Each API's client struct, plus the runtime's Call, CallError, Credential, FullCall, Response, ...
├── shared.rs        # encode_cookie_value, combine_cookie_headers, FormFile
├── ws_shared.rs     # WebSocket support shared by the *_ws modules
├── anthropic.rs     # A module under 2000 formatted lines stays one file
├── lmstudio.rs
├── openai/          # A larger module is split into a directory:
│   ├── mod.rs       #   module docs, submodules, re-exports, each API's client struct and tag
│   ├── requests.rs  #   per-endpoint request structs, EndpointSpec impls, {Name}Call types, endpoint methods
│   └── responses.rs #   (when the module has response types)
├── ollama/          # OllamaNative + OllamaOpenAI (combined, shared module)
├── emqx/            # EmqxBasic + EmqxBearer (combined, shared module)
├── elevenlabs_ws.rs # WebSocket clients
└── ...              # one entry per remaining module group
```

APIs sharing a `module_path` are automatically combined into a single output file.
Each API in the combined file gets its own client struct, tag type, and request
suffix to avoid naming collisions. Full generation deletes generated modules
that no API produces any more; files without the generated header are never
deleted or overwritten.

## Generator Conventions & Assumptions

### Available APIs

The CLI accepts the selection key of every REST entry in the registration
inventory, in catalog order, then `all` (see
[Register an API](workflow.md#register-an-api)):

```
anthropic, artificial-analysis-data, artificial-analysis-critpt, bitbucket, openai, elevenlabs, gitea, github, gitlab, huggingface, lmstudio, ollama-native, ollama-openai, emqx-basic, emqx-bearer, eversolo, samsung-smart-tv, unfolded-circle-core-rest, all
```

### Module Naming

The generator uses **1 module path = 1 output file**. APIs sharing a `module_path` are combined:

```
API Name: "OpenAI"       → Module: openai.rs    → Import: schematic_definitions::openai::*
API Name: "ElevenLabs"   → Module: elevenlabs.rs → Import: schematic_definitions::elevenlabs::*
API Name: "OllamaNative" ┐
API Name: "OllamaOpenAI" ┘→ Module: ollama.rs   → Import: schematic_definitions::ollama::*
```

#### Module Path Configuration

You can override the default module path derivation using `RestApi::module_path`:

```rust
let api = RestApi::builder(&context, "OllamaOpenAI", "http://localhost:11434/v1")
    .describes("OpenAI-compatible Ollama API").no_auth()
    .module_path("ollama").build()?;
```

#### Automatic Path Inference

Without an explicit `module_path`, one resolver,
`schematic_definitions::naming::resolve_module_name`, names the module. The
generated Rust module, the OpenAPI document, the Postman collection, the
module grouping, and the definitions module written by `import` all use it,
so an API never lands under two names. For API names with recognized
suffixes it drops the suffix:

| API Name | Inferred Module |
|----------|-----------------|
| `OllamaNative` | `ollama` (strips "Native" suffix) |
| `HTTPClient` | `http` (strips "Client" suffix) |
| `MyService` | `my` (strips "Service" suffix) |
| `HuggingFaceHub` | `huggingface` (strips "Hub" suffix) |
| `TestApi` | `test` (strips "Api" suffix) |
| `AwsSdk` | `aws` (strips "Sdk" suffix) |

The supported suffixes for inference are: `Native`, `Client`, `Service`, `Hub`, `Api`, `Sdk`.

Explicit `module_path` always takes precedence over inference. Any other name
is lowercased (`OpenAI` becomes `openai`). Module names that differ only by
letter case are refused, because they would collide on case-insensitive file
systems (macOS, Windows).

**Multi-API modules require explicit configuration**. When multiple APIs share a definitions module, set both `module_path` and `request_suffix` to avoid naming collisions:

```rust
let native = RestApi::builder(&context, "OllamaNative", "http://localhost:11434")
    .describes("Native Ollama API").no_auth()
    .module_path("ollama").request_suffix("NativeRequest").build()?;
let openai = RestApi::builder(&context, "OllamaOpenAI", "http://localhost:11434/v1")
    .describes("OpenAI-compatible API").no_auth()
    .module_path("ollama").request_suffix("OaiRequest").build()?;
```

The generator groups APIs by `module_path` and combines them into a single output file with shared imports. Stale `.rs` files from previous generations are automatically cleaned up.

### Wrapper Struct Generation

For each endpoint, the generator creates a wrapper struct named `{EndpointId}{Suffix}`:

```
Endpoint ID: "Generate"      → struct GenerateRequest { ... }
Endpoint ID: "ListModels"    → struct ListModelsRequest { ... }
Endpoint ID: "CreateChat"    → struct CreateChatRequest { ... }
```

The suffix defaults to "Request" but can be customized via `RestApi::request_suffix`:

```rust
let api = RestApi::builder(&context, "MyApi", "https://example.com")
    .describes("Example API").no_auth().request_suffix("Req").build()?;
```

**Important**: Body types in your definitions must use different names (conventionally `*Body` suffix) to avoid recursive struct definitions. See [schematic-define naming conventions](../definition-builders.md).

## Generated Code Structure

For an API named "OpenAI", the generator produces (abridged):

```rust
//! Generated API client for OpenAI.
//!
//! This code was automatically generated by schematic-gen. Do not edit manually.

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// API client struct and tag type (one per API)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The OpenAI API: ...
#[derive(Clone, Debug)]
pub struct OpenAI(schematic_runtime::Client<OpenAIApi>);

/// The tag of the [`OpenAI`] API: ... Callers never write it.
pub enum OpenAIApi {}

impl schematic_runtime::Api for OpenAIApi {
    const NAME: &'static str = "OpenAI";
}

// Every client-wide setting (`base_url`, `api_key`, `timeout`, …), `build`,
// the generic `send` (returns a `Call`), `send_raw`, and `check_link`, as
// one-line forwards.
schematic_runtime::api_client!(OpenAI, OpenAIApi);

impl OpenAI {
    pub const BASE_URL: &'static str = "https://api.openai.com/v1";
    pub const DOCS_URL: Option<&'static str> = None;

    /// What the API tells the runtime: its base URL, how it accepts
    /// credentials, its cross-origin policy, its secret names, its
    /// request-id header, and any endpoint that authenticates differently.
    pub fn definition() -> schematic_runtime::ApiDefinition {
        schematic_runtime::ApiDefinition::new(Self::BASE_URL)
            .auth(
                schematic_runtime::AuthConfig::none()
                    .accept(schematic_runtime::CredentialPlacement::Bearer { header: None }),
            )
            .request_id_header("x-request-id")
    }

    /// A client with the API's defaults and default headers; never fails.
    pub fn new() -> Self {
        Self(schematic_runtime::Client::new(Self::definition()))
    }

    // Only when the API, or an endpoint policy replacing the API's, accepts HTTP Basic:
    // pub fn basic_auth(self, username: impl Into<String>, password: impl Into<String>) -> Self
}

impl Default for OpenAI { ... }

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Request structs (one per endpoint)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Request for `RetrieveModel` endpoint.
// `{model}` is an untyped placeholder; a declared (typed) path parameter
// changes the field type and drops `Default` and the `From` impls (see "Path Parameters").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrieveModelRequest {
    /// Path parameter: model
    pub model: String,
}

impl RetrieveModelRequest {
    /// Creates a new request with the required path parameters.
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: model.into() }
    }
}

// From<&str> and From<String> for single-param, no-body requests
impl From<&str> for RetrieveModelRequest { ... }
impl From<String> for RetrieveModelRequest { ... }

impl schematic_runtime::EndpointSpec for RetrieveModelRequest {
    type Api = OpenAIApi;
    type Success = Model;
    type Failure = schematic_runtime::NoDeclaredError;
    type Decoder = schematic_runtime::Json<Model>;

    const ENDPOINT_ID: &'static str = "RetrieveModel";
    const METHOD: schematic_runtime::Method = schematic_runtime::Method::GET;

    fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument> {
        let path = format!("/models/{}", urlencoding::encode(&self.model.to_string()));
        let query: Vec<(String, schematic_runtime::QueryValue)> = Vec::new();
        let headers: Vec<(String, String)> = vec![];
        Ok(schematic_runtime::RequestParts {
            method: <Self as schematic_runtime::EndpointSpec>::METHOD,
            path,
            query,
            headers,
            body: schematic_runtime::RequestBody::Empty,
        })
    }
}
```

What each part of the `EndpointSpec` impl means:

- **`type Api`** scopes the request to its API. An `OpenAI` client accepts
  only requests whose `Api` is `OpenAIApi`, so passing another API's request
  to `send`, `send_raw`, `check_link`, or `mutate_response` does not compile.
- **`type Success` and `type Decoder`** come from the endpoint's success
  shapes. With one shape, the codec, not the response's `Content-Type`,
  judges the body: a JSON endpoint that receives HTML reports a
  `MalformedSuccess` error in the envelope with the body kept, and valid JSON
  under a misleading `Content-Type` still decodes.

  | One shape | `Success` | `Decoder` |
  |-----------|-----------|-----------|
  | JSON | the schema's type | `schematic_runtime::Json<T>` |
  | text | `String` | `schematic_runtime::Text` |
  | binary | `bytes::Bytes` | `schematic_runtime::Binary` |
  | empty | `()` | `schematic_runtime::Empty` |
  | SSE / NDJSON stream | `schematic_runtime::ItemStream<T>` | `Sse<T, End>` / `Ndjson<T, End>` |
  | byte stream | `schematic_runtime::ByteStream` | the same |

  Several shapes generate an `{Id}Response` enum chosen by status, media
  type, and JSON tag; see
  [Generated success and failure types](../responses-and-errors.md#generated-success-and-failure-types).
- **`type Failure`** comes from the effective error table: `NoDeclaredError`
  (every non-2xx reply is an undeclared API error) when the endpoint declares
  and inherits nothing or opts out, as in the example above; otherwise the
  one error shape's type or an `{Id}Error` enum, with a `const ERRORS` table
  routing each status to it.
- **`into_parts`** describes the request as a `RequestParts` struct, or
  returns an `InvalidArgument` (a JSON body that fails to serialize, a static
  `Cookie` header that cannot be merged) that the call reports, at `.build()`
  or `.await`, before any network work. See [Request-Side Inputs](../generation/request-side.md).

`definition()` leaves out every setting equal to the runtime's default: a
`same_origin` cross-origin policy, no extra secret names, no request-id
header. A definition's `CrossOrigin` value maps onto the `OriginPolicy`
constructor of the same name. Only an endpoint that declares its own
authentication adds a call: `.endpoint_public("Health")` for `.public()`,
and `.endpoint_auth("Upload", AuthConfig::none()...)` for `.auth(..)` or
`.auth_with_env_mapping(..)`, spelling out the replacement policy the way
`.auth(..)` spells out the API's (see
[Authentication Strategies](#authentication-strategies)). An API without
such endpoints generates exactly the `definition()` it always did. The API's static headers other than `Cookie`
become `.default_header(..)` calls in `new()`; an API-level `Cookie` is
folded into every endpoint's static headers instead, because the runtime
refuses a default header that can carry a credential.

Each GraphQL operation of the API gets a request struct of the same kind,
sent to the API's GraphQL address; see
[Generated operation requests](../graphql.md#generated-operation-requests).

Each REST endpoint also gets a method on the API struct and a call type that
wraps the runtime's `Call` (abridged; `ListPullRequests` from GitHub, a
paginated list with optional values):

```rust
/// A call to the `ListPullRequests` endpoint, returned by [`GitHub::list_pull_requests`].
#[derive(Clone, Debug)]
#[must_use = "a call sends nothing until it is awaited"]
pub struct ListPullRequestsCall<'a>(schematic_runtime::Call<'a, GitHubApi, ListPullRequestsRequest>);

impl<'a> ListPullRequestsCall<'a> {
    // One setter per optional value, named after the request field without `with_`;
    // `page` and `per_page` are adopted by the pagination strategy, so they have none
    pub fn state(self, value: String) -> Self { ... }
    pub fn sort(self, value: String) -> Self { ... }
    pub fn direction(self, value: String) -> Self { ... }
    // Shared per-call forwards to `schematic_runtime::Call`
    pub fn anonymous(self) -> Self { ... }
    pub fn credential(self, credential: schematic_runtime::Credential) -> Self { ... }
    pub fn deadline(self, deadline: &schematic_runtime::Deadline) -> Self { ... }
    // `full` ends the endpoint's setters: `FullCall` keeps only the per-call settings
    pub fn full(self) -> schematic_runtime::FullCall<'a, GitHubApi, ListPullRequestsRequest> { ... }
    pub fn build(self) -> Result<Self, schematic_runtime::InvalidArgument> { ... }
    // Paginated lists only: `pager`, each authored control, and the iteration
    // entry points (see "Pagination Support")
    pub fn pager(self) -> schematic_runtime::pager::Pager<'a, GitHubApi, ListPullRequestsRequest> { ... }
    pub fn page(self, number: u64) -> schematic_runtime::pager::Pager<'a, GitHubApi, ListPullRequestsRequest> { ... }
    pub fn page_size(self, size: u32) -> schematic_runtime::pager::Pager<'a, GitHubApi, ListPullRequestsRequest> { ... }
    pub fn pages(self) -> schematic_runtime::pager::Pages<'a, GitHubApi, ListPullRequestsRequest> { ... }
    pub fn fetch_page(self) -> schematic_runtime::CallFuture<'a, schematic_runtime::pager::PageResult<ListPullRequestsRequest>> { ... }
    pub fn collect_all(self) -> schematic_runtime::CallFuture<'a, schematic_runtime::pager::CollectResult<ListPullRequestsRequest>> { ... }
}

// `.await` resolves to Result<Success, CallError<Failure>>
impl<'a> IntoFuture for ListPullRequestsCall<'a> { ... }

impl GitHub {
    /// List pull requests with metadata
    ///
    /// `GET /repos/{owner}/{repo}/pulls`
    /// ... `## Arguments` and `## Optional values` (links to the setters)
    pub fn list_pull_requests(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> ListPullRequestsCall<'_> {
        ListPullRequestsCall(self.0.send(ListPullRequestsRequest::new(owner, repo)))
    }
}
```

The method is the endpoint id in snake case, and its arguments are the
required values in the order `{Name}Request::new` takes them. A name that
would collide is renamed rather than shadowed: an endpoint method that
matches an API-struct method (`new`, `send`, `base_url`, …) gets an
`_endpoint` suffix, and a setter that matches a call-type method (`full`,
`build`, `credential`, …) gets a `_param` suffix, each reported as an
`endpoint-method-rename` warning. See
[Endpoint method names](../generation/validation.md#endpoint-method-names).

### Deprecation, rate limits, and endpoint authentication in docs

Some definition vocabulary only changes the generated documentation, so your
editor's hover text tells you about it while the code stays the same. An
endpoint or API that declares none of it generates exactly the docs it did
before.

| Declaration | Where it appears |
| --- | --- |
| `.deprecated()` on an endpoint | a "Deprecated." line on the request type, the endpoint method, and the call type |
| `.deprecated()` on a parameter | a "Deprecated." line on its request field and its setter, and after its entry in the method's `## Optional values` list |
| `.public()` | "This endpoint is public: it sends no credential." on the request type (under `## Authentication`) and the method |
| `.auth(..)`, `.auth_with_env_mapping(..)` | the same places: the methods the replacement accepts and the environment variables it reads, in order |
| API `.rate_limit(..)`, `.retry_guidance(..)` | `## Rate limit` and `## Retry guidance` sections in the module docs, including a module shared by several APIs |
| the endpoint's effective rate limit or retry guidance (its own, else the API's) | the same sections on its request type |

Deprecation never generates a `#[deprecated]` attribute: that would fail
every `-D warnings` build of the generated crate and of its dependents.
Rate-limit and retry sections are descriptive only; the client neither reads
the headers nor retries. For example, every GitHub request type carries:

```rust
/// ## Rate limit
///
/// Response headers reporting the rate limit (documentation only; the client does not act on them):
///
/// - limit: `x-ratelimit-limit`
/// - remaining: `x-ratelimit-remaining`
/// - reset: `x-ratelimit-reset`, as Unix time in seconds
/// - retry after: `retry-after`
```

When any endpoint authenticates differently, the API struct's `new()` docs
also say that a public endpoint sends no credential and that a replacement
refuses a send which resolves none.

### Examples in the generated docs

Every code example the generator writes into the crate is a `no_run`
doctest: it is compiled, never run, so it cannot drift from the generated API
without failing `just doctest`. No example names a method the client lacks.

| Where | What the example shows |
| --- | --- |
| each request type | building the request: `default()` when it takes no values, otherwise `new(..)` with the path values, the required query values, and a `Default` body, then one `with_*` call per optional parameter with a sample value of its type |
| each API's module | calling the first GET endpoint (or the first endpoint) through its endpoint method, then as the typed request `client.send({Name}Request::new(..))`, then `.full()` for the whole response; and overriding the base URL |
| crate root `## Quick Start`, prelude `## Examples` | the same module example for the first API with a GET endpoint, shown only when that API's module feature is enabled |

For example, the GitHub module's docs carry this example; the generated
doctest hides the `async fn` wrapper lines with `#`:

```rust
use schematic_schema::prelude::*;
use schematic_schema::github::GetRepositoryRequest;

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let client = GitHub::new();
    let data = client.get_repository("owner", "repo").await?;

    // the same request, built as a typed request and sent
    let data = client.send(GetRepositoryRequest::new("owner", "repo")).await?;
    Ok(())
}
```

A module shared by several APIs (`emqx`, `ollama`, `artificial_analysis`)
opens with a sentence naming each API, then gives each one a `## {Api}`
section holding the docs its own module would have (authentication,
endpoint list, rate limit, examples), with every heading one level deeper.

`just doctest` compiles the committed crate's examples twice through
`just doctest-schema`: with the default features, and with only the shared
`emqx` module enabled, which proves no example outside a module's own docs
imports a disabled module.

## Using Generated Code

```rust
use schematic_schema::prelude::*;
use schematic_schema::openai::RetrieveModelRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAI::new();

    // An endpoint method: the data, or a CallError (no reply, or a failed one)
    let models = client.list_models().await?; // ListModelsResponse
    let model = client.retrieve_model("gpt-4").await?;

    // `.full()` returns Ok(Response) whenever an HTTP response head arrived,
    // whatever its status; Err(TransportError) only when none did.
    let response = client.list_models().full().await?;

    // The generic `send` takes a request object, for code written over many
    // endpoints or a request built elsewhere; it returns the same kind of call
    let model = client.send(RetrieveModelRequest::from("gpt-4")).await?;

    // Chain settings onto the client; `build()?` reports a bad one now
    // instead of on the first send, and returns the same `OpenAI`
    let test_client = OpenAI::new()
        .base_url("http://localhost:8080/v1")
        .timeout(std::time::Duration::from_secs(10))
        .api_key("sk-...")
        .build()?;

    Ok(())
}
```

`send_raw(request, &CallOptions)` returns the reply without decoding it.
[Calling an API](../runtime/calling-an-api.md) covers endpoint methods,
setters, per-call overrides, and `CallError`. The envelope, its metadata,
transport errors, deadlines, and redirects are described in the runtime's
crate documentation ([`runtime/src/lib.rs`](../../runtime/src/lib.rs)).

## Authentication Strategies

Generation translates the definition's **effective** auth policy
(`RestApi::effective_auth_policy()`, so an explicit `auth_policy` wins over
the legacy `auth` strategy) into the runtime's `AuthConfig` inside
`definition()`. The generated code reads no environment variable itself and
adds no credential setters: the runtime resolves a credential for every
send.

| Definition method | Accepted runtime placement |
|-------------------|----------------------------|
| `BearerToken { header }` | `CredentialPlacement::Bearer { header }` |
| `ApiKey { header, value_prefix }` | `CredentialPlacement::ApiKeyHeader { header, prefix }` |
| `ApiKeyParam { name, location: Query }` | `CredentialPlacement::ApiKeyQuery { name }` |
| `ApiKeyParam { name, location: Cookie }` | `CredentialPlacement::ApiKeyCookie { name }` |
| `Basic` | `CredentialPlacement::Basic` |
| `OAuth2(..)` | `CredentialPlacement::Bearer { header: None }`: an access token the caller supplies, never read from the environment |

The policy's env fallback becomes an `EnvFallback` over the variables the
definition's environment mapping names (`RestApi::default_env_mapping()`;
for `ApiKeyParam`, the definition's `env_auth`; for `Basic`, the password
variables plus `username_vars`). A fallback with no variable to read is
dropped, so the API reads nothing from the environment. LM Studio and
Eversolo, for example, set an empty mapping, so their `env_auth` variable is
never read.

Runtime precedence, for an endpoint that uses the API's policy:

1. a per-call credential (`.credential(..)` on the call)
2. the client credential (`api_key(..)`, `basic_auth(..)`, or `credentials(..)` on the client), when the policy accepts its kind
3. the first non-empty configured environment variable (the client's `env_order([..])` replaces the definition's order)
4. none: the request is sent anonymously and its metadata records `AuthMethod::Anonymous`

There is no missing-credential error for such an endpoint; a server that
requires one answers (typically with 401) as a `CallError::Api`, or inside
the `.full()` response envelope. `.anonymous()` on a call removes every
credential source for that call; `.anonymous()` and `.credential(..)`
replace each other, and the last one wins. A credential is never
injected over a request value of the same name: a request that already sets
the credential header, the API-key query parameter, or the API-key cookie
fails with `TransportErrorKind::InvalidArgument` before anything is sent.

An endpoint that declares its own authentication changes this for its sends
only:

- **`.public()`**: no credential from any source is sent (a per-call one is
  silently dropped), and every header, query parameter, and cookie in a
  credential location of the API or of any endpoint replacement is removed
  from the request, even a value the request set itself.
- **`.auth(..)` or `.auth_with_env_mapping(..)`**: the replacement policy is
  used instead of the API's, in the same order (per call, client, then the
  endpoint's own environment variables, which `env_order` does not change).
  A credential counts only when the replacement accepts its kind; a per-call
  credential of another kind fails with `InvalidArgument`. When nothing
  resolves the send fails with `InvalidArgument`, naming the endpoint and
  the kinds it accepts, unless the call is `.anonymous()`.

A client credential is accepted at `build()` when the API's policy or any
replacement accepts its kind, and `basic_auth(..)` is generated when either
accepts Basic. [Endpoint authentication](../runtime/credentials.md#endpoint-authentication)
has the full rules and a diagram of the decision for one send.

```rust
use schematic_schema::prelude::*;

// Per call
let mrs = GitLab::new()
    .list_merge_requests("group/project")
    .credential(Credential::api_key("key"))
    .await?;

// Per client
let client = GitLab::new().api_key("token");

// Environment, in a custom order
let client = GitLab::new().env_order(["MY_GITLAB_TOKEN", "GITLAB_TOKEN"]);
```

### Bearer Token

```rust
let api = RestApi::builder(&context, "OpenAI", "https://api.openai.com/v1")
    .describes("OpenAI API").bearer_env(["OPENAI_API_KEY"]).build()?;
```

Accepts `Credential::access_token(..)`, sent as `Authorization: Bearer <token>`.

### API Key

```rust
let api = RestApi::builder(&context, "Example", "https://example.com")
    .describes("Example API")
    .authentication(AuthStrategy::ApiKey { header: "X-API-Key".into() }, ["X_API_KEY"])
    .build()?;
```

Accepts `Credential::api_key(..)`, sent as `X-API-Key: <key>`, preceded by
the method's `value_prefix` when it declares one.

### API Key in a Query Parameter or Cookie

```rust
let api = RestApi::builder(&context, "Example", "https://example.com")
    .describes("Example API")
    .authentication(AuthStrategy::ApiKeyParam {
        name: "api_key".into(), location: ApiKeyLocation::Query,
    }, ["EXAMPLE_KEY"]).build()?;
```

Accepts `Credential::api_key(..)`. The key, from any source above, is
appended as the `api_key` query pair or to the one `Cookie` header. See
[Request-Side Inputs](../generation/request-side.md#api-keys-in-a-query-parameter-or-cookie).

### Basic Auth

```rust
let api = RestApi::builder(&context, "Example", "https://example.com")
    .describes("Example API").basic_env("API_USER", ["API_PASS"]).build()?;
```

Accepts `Credential::basic(username, password)`. The env fallback needs a
non-empty username variable and a non-empty password variable.

### No Authentication

```rust
let api = RestApi::builder(&context, "Local", "http://localhost:11434")
    .describes("Local API").no_auth().build()?;
```

`definition()` configures no authentication; every request is anonymous,
and a per-call credential is refused as `InvalidArgument`.

### OAuth2

```rust
let api = RestApi::builder(&context, "Example", "https://example.com")
    .describes("Example API")
    .authentication(AuthStrategy::OAuth2(oauth_config), Vec::<String>::new())
    .build()?;
```

The client accepts a bearer access token. Obtain it with `schematic-oauth`
and supply it per client or per call:

```rust
let client = Api::new().api_key(token);
```

### Multi-Method Auth

When `RestApi::auth_policy` is set, generated clients can accept more than one explicit auth method. For example, a client can accept either an explicit API key or an explicit OAuth token while only falling back to env-based API keys:

```rust
let policy = AuthPolicy {
    explicit: vec![
        AuthMethod::ApiKey { header: "PRIVATE-TOKEN".into() },
        AuthMethod::OAuth2(oauth_config),
    ],
    env_fallback: Some(EnvAuthStrategy::ApiKey { header: "PRIVATE-TOKEN".into() }),
};
let api = RestApi::builder(&context, "GitLab", "https://gitlab.com/api/v4")
    .describes("GitLab API")
    .authentication(AuthStrategy::ApiKey { header: "PRIVATE-TOKEN".into() }, ["GITLAB_TOKEN"])
    .auth_policy(policy).build()?;
```

`definition()` then accepts both placements, so the same client takes
`Credential::api_key(..)` (sent as `PRIVATE-TOKEN`) or
`Credential::access_token(..)` (sent as `Authorization: Bearer`), and reads
only `GITLAB_TOKEN` from the environment.

## HTTP Methods

All standard HTTP methods are supported. Each request's `EndpointSpec::METHOD`
names the runtime's method constant:

| Method | `EndpointSpec::METHOD` |
|--------|------------------------|
| GET | `schematic_runtime::Method::GET` |
| POST | `schematic_runtime::Method::POST` |
| PUT | `schematic_runtime::Method::PUT` |
| PATCH | `schematic_runtime::Method::PATCH` |
| DELETE | `schematic_runtime::Method::DELETE` |
| HEAD | `schematic_runtime::Method::HEAD` |
| OPTIONS | `schematic_runtime::Method::OPTIONS` |

## Path Parameters

Path parameters use `{param}` syntax. An endpoint that declares none of them
with `path_param(..)` keeps every placeholder an untyped `String` field,
extracted from the template:

```rust
// API definition
let endpoint = Endpoint::get("GetMessage", "/threads/{thread_id}/messages/{message_id}")
    .describes("Read a message").returns_json::<Message>(&mut context).build()?;

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
}

impl schematic_runtime::EndpointSpec for GetMessageRequest {
    // type Api, Success, Failure, Decoder; ENDPOINT_ID; METHOD ...

    fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument> {
        let path = format!(
            "/threads/{}/messages/{}",
            urlencoding::encode(&self.thread_id.to_string()),
            urlencoding::encode(&self.message_id.to_string()),
        );
        let query: Vec<(String, schematic_runtime::QueryValue)> = Vec::new();
        let headers: Vec<(String, String)> = vec![];
        Ok(schematic_runtime::RequestParts {
            method: <Self as schematic_runtime::EndpointSpec>::METHOD,
            path,
            query,
            headers,
            body: schematic_runtime::RequestBody::Empty,
        })
    }
}

// Usage
let request = GetMessageRequest::new("thread-123", "msg-456");
```

### Typed path parameters

An endpoint that declares its path parameters with
`.path_param(ParamDef::path(name, type))` gets one typed field per distinct
placeholder:

| Declared type | Field type | `new(..)` argument |
| --- | --- | --- |
| `String` | `String` | `impl Into<String>` |
| `Integer` | `i64` | `i64` |
| `Number` | `f64` | `f64` |
| `Boolean` | `bool` | `bool` |
| `Enum(values)` | generated enum `{RequestType}{FieldPascal}` | the enum |

```rust
// API definition
let endpoint = Endpoint::get("GetIssue", "/repos/{owner}/{number}/labels/{kind}")
    .describes("Read an issue's labels").returns_empty()
    .path_param(ParamDef::path("kind", QueryParamType::Enum(vec!["bug".into(), "help-wanted".into()])))
    .path_param(ParamDef::path("number", QueryParamType::Integer).describes("The issue number."))
    .path_param(ParamDef::path("owner", QueryParamType::String))
    .build()?;

// Generated (abridged): no `Default`, no `From<&str>`/`From<String>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIssueRequest {
    pub owner: String,
    /// The issue number.
    pub number: i64,
    pub kind: GetIssueRequestKind,
}

#[non_exhaustive]
pub enum GetIssueRequestKind {
    #[serde(rename = "bug")]
    Bug,
    #[serde(rename = "help-wanted")]
    HelpWanted,
}
// plus `as_str()` and `Display`, both giving the wire value ("help-wanted")

// Usage: arguments follow the template, not the declaration order
let request = GetIssueRequest::new("octocat", 42, GetIssueRequestKind::HelpWanted);
// sends /repos/octocat/42/labels/help-wanted
```

- **Argument order.** `new(..)` takes the path arguments in the order their
  placeholders first appear in the template, then the body. The endpoint
  method on the API struct takes the same arguments.
- **Repeated placeholders.** A placeholder that appears twice
  (`/items/{id}/copies/{id}`) is one field, sent at every occurrence.
- **Wire spelling.** Each value is converted to its wire form before
  encoding: the decimal number, `true` or `false`, or the enum's `as_str()`.
  It is then percent-encoded, except at a `{+name}` occurrence, which keeps
  the raw value (`dir/sub/file.txt` stays unescaped). A non-finite `f64`
  (`NaN`, infinity) fails with `InvalidArgument` naming the parameter before
  anything is sent. See [Request-Side Inputs](./request-side.md).
- **No shortcut constructors.** A request with a typed path field derives no
  `Default` and gets no `From<&str>` or `From<String>`, because a defaulted
  or string-converted value would send a placeholder the caller never chose.
  `new(..)` is the only way to build it.
- **Docs.** A field's doc is its declared description, else
  `Path parameter: {name}`, with a "Deprecated." line when the parameter is
  deprecated. The enum is `#[non_exhaustive]`, so a value added later is not
  a breaking change.

Endpoints that declare no path parameters are unchanged. The rules a
declaration must satisfy are in
[Building REST definitions](../definition-builders.md#paths-deprecation-rate-limits-and-endpoint-authentication),
and the generator's `path-parameter` and `generated-name-collision` checks are
in [Generation Validation](./validation.md#what-each-output-checks).

## Query Parameters

Declared query parameters become fields with `with_*` builders: a required
one is a `T` field taken by `new()` after the path parameters and before the
body, an optional one an `Option<T>` field. An enum parameter gets a
generated `{RequestType}{FieldPascalCase}` enum. A request with a required
query parameter, like one with a typed path parameter, has no `Default` and
no `From` impls; its endpoint method takes the value as an argument rather
than offering a setter:

```rust
// `start_time` is required; `bucket_width` is an optional enum.
let costs = client
    .usage_costs(1_700_000_000)
    .bucket_width(UsageCostsRequestBucketWidth::V1d)
    .await?;
```

`into_parts()` returns the parameters in `RequestParts::query` as unencoded
`(name, QueryValue)` pairs, apart from the path, and the runtime encodes each
pair once when it builds the URL (a space is `%20`, never `+`), appending
them with `&` after any query already in the path template. Arrays are
repeated or delimited as their `style` and `explode` declare. See
[`docs/generation/request-side.md`](../generation/request-side.md#query-parameters).

## Request Bodies

Endpoints with request bodies get a `body` field and a `new()` constructor:

```rust
// API definition
let endpoint = Endpoint::post("CreateCompletion", "/completions")
    .describes("Create a completion")
    .request_json::<CreateCompletionBody>(&mut context)
    .returns_json::<Completion>(&mut context).build()?;

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
}

impl schematic_runtime::EndpointSpec for CreateCompletionRequest {
    // type Api, Success, Failure, Decoder; ENDPOINT_ID; METHOD ...

    fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument> {
        let path = "/completions".to_string();
        let body = serde_json::to_string(&self.body).map_err(|_| {
            schematic_runtime::InvalidArgument::new("the request body cannot be serialized as JSON")
        })?;
        // ...
        Ok(schematic_runtime::RequestParts {
            method: <Self as schematic_runtime::EndpointSpec>::METHOD,
            path,
            query,
            headers,
            body: schematic_runtime::RequestBody::Json(body),
        })
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

A text body (`ApiRequest::Text { content_type }`) is a `body: String` field and
a binary body (`ApiRequest::Binary { content_type }`) a `body: Vec<u8>` field.
Both become `RequestBody::Raw`: the bytes are sent unchanged with the declared
`Content-Type`. A multipart form's file fields use the generated
`shared::FormFile`, which `into_part(name)` turns into a
`schematic_runtime::FormPart`.

## Header and Cookie Parameters

`EndpointParams.header` and `EndpointParams.cookie` entries become optional
fields with `with_*` builders, sent under their exact declared names in
`RequestParts::headers`. A set header parameter replaces a static endpoint
header of the same name (case-insensitive); arrays are joined with `,`.
Cookie values are percent-encoded where RFC 6265 requires
(`shared::encode_cookie_value`) and merged, with the API's and the
endpoint's static cookies, into one `Cookie` header
(`shared::combine_cookie_headers`). A cookie API key is appended to that
header by the runtime. The runtime parses every header through the HTTP
types before sending, so an invalid value fails with
`TransportErrorKind::InvalidArgument` without a request and without echoing
the value. See
[`docs/generation/request-side.md`](../generation/request-side.md).

## Module Reference

### `codegen`

Code generation for individual components:

| Generator | Description |
|-----------|-------------|
| `generate_api_struct(&api)` | Creates an API's client struct, its `{Api}Api` tag type and `Api` impl, the `api_client!` invocation, constants, `definition()`, `new()`, and `basic_auth()` when the API accepts Basic |
| `generate_request_struct(&endpoint)` | Creates the request struct and its `EndpointSpec` impl for an endpoint, naming the placeholder API marker |
| `generate_request_struct_with_suffix(&endpoint, suffix)` | The same, with a custom struct-name suffix |
| `generate_request_struct_with_options(&endpoint, suffix, module_path, api_marker)` | The same, with the module path used in doc examples and the type `EndpointSpec::Api` names; generated modules pass the API's tag (`{Api}Api`) |
| `PLACEHOLDER_API_MARKER` | The marker (`Api`) named by request structs generated without an API |
| `generate_request_helpers()` | Creates the request helpers of `shared.rs`: `encode_cookie_value`, `combine_cookie_headers`, `FormFile` |
| `generate_pagination_impls(&api, suffix)` | Creates the `schematic_runtime::pager::Paginated` impl and the capability marker impls of every paginated list endpoint |
| `ModuleDocBuilder` | Builds a generated module's `//!` documentation |

### `output`

Assembly, validation, and file writing:

| Function | Description |
|----------|-------------|
| `assemble_api_code(&api)` | Combines all generators into one `TokenStream` |
| `assemble_api_module(&api)` | Assembles a single API module |
| `assemble_combined_api_module(&apis)` | Combines multiple APIs sharing a module path into one `TokenStream` |
| `assemble_lib_rs(&apis)` | Generates lib.rs with module declarations and the runtime ABI marker and check |
| `assemble_shared_module()` | Generates shared.rs with the request helpers |
| `assemble_prelude(&apis)` | Generates prelude.rs with convenient re-exports |
| `generate_and_write(&api, &dir, dry_run)` | Full generation of one API (the API is the whole crate) |
| `generate_and_write_all(&apis, &dir, dry_run)` | Full generation: `apis` is the whole crate; obsolete generated modules are deleted |
| `prepare_rust_output(&RustOutputRequest)` | Validates, assembles, and checks a full (`Selection::All`) or partial (`Selection::Apis`) run; returns a `RustOutputPlan` without writing |
| `expand_selection(&registry, selection)` | The registry APIs a selection regenerates (complete module groups) |
| `MutationPlan::publish()` | Applies a prepared plan; on I/O failure reports completed and pending mutations |
| `generate_and_write_standalone(&api, &dir, dry_run, has_types)` | Generates standalone code for imported OpenAPI specs |
| `validate_code(&tokens)` | Validates generated code with `syn` |
| `format_code(&file)` | Formats code with `prettyplease` |
| `write_atomic(&path, &content)` | Atomic file write (temp + rename) |

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
| `ModuleFeatures::new(&registry, registrations)` | The module groups the crate declares, one Cargo feature each |
| `generate_cargo_toml(workspace_root, &features)` | Returns Cargo.toml content string |
| `generate_cargo_toml_with_paths(define, definitions, runtime, &features)` | The same, with explicit dependency paths |
| `write_cargo_toml(&dir, dry_run, workspace_root, &features)` | Writes Cargo.toml to directory |

### `errors`

Generator error types (used by the generator itself):

| Variant | Description |
|---------|-------------|
| `ParseError` | Failed to parse API definition |
| `CodeGenError` | Code generation produced invalid Rust |
| `WriteError` | Failed to write output file |
| `OutputDirNotFound` | Output directory doesn't exist |
| `ConfigError` | Invalid configuration |
| `Validation` | An output's validator reported errors; nothing of that output was written |
| `UnsafeOutput` | The output holds paths the generator cannot account for or does not own; nothing was changed |
| `RequiresFullGeneration` | A partial run would keep modules while the runtime ABI recorded in `lib.rs` or a shared file changes; nothing was changed |
| `PublicationFailed` | A write or deletion failed after publication started; lists completed and pending mutations |
| `OutputsFailed` | At least one requested output failed; names failed and succeeded outputs |

### `validation`

Per-output validators; each takes the whole batch and returns
`Vec<ValidationDiagnostic>` (see the `diagnostics` module):

| Function | Description |
|----------|-------------|
| `validate_rust_client(&apis)` | Routes, schema references, plus Rust-only rules: identifiers, suffixes, module paths, type syntax, generated-name collisions, request-input names and content types |
| `validate_openapi(&apis)` | Routes, embedded queries, schema references, document file names |
| `validate_postman(&apis)` | Routes, embedded queries, collection file names |
| `endpoint_uniqueness_key(&endpoint)` | The `(method, normalized path)` identity two endpoints of one API may not share |

### Module names

The resolver lives in `schematic_definitions::naming`; the generator has no
copy of its own:

| Function | Description |
|----------|-------------|
| `resolve_module_name(&api)` | The module name and artifact file stem of an API |
| `module_name(&name, explicit)` | The same, from a name and an optional explicit module path |
| `infer_module_path(&name)` | The suffix rule alone (Native, Client, Service, Hub, Api, Sdk) |

### Runtime compatibility

`schematic_gen::RUNTIME_ABI` is the `schematic_runtime::ABI` the generator
emits code for; a unit test keeps the two equal. Generated `lib.rs` declares
it and checks it at compile time:

```rust
pub const SCHEMATIC_RUNTIME_ABI: u32 = 1;
const _: () = assert!(
    schematic_runtime::ABI == SCHEMATIC_RUNTIME_ABI,
    "schematic-schema was generated for schematic-runtime ABI 1, but ...; regenerate the crate ..."
);
pub use schematic_runtime;
```

Building generated code against a runtime of another ABI fails to compile
with that message. Partial generation reads the same marker before keeping
any module; see
[shared runtime compatibility](../generation/partial-generation.md#shared-runtime-compatibility).

Generated code defines no error type of its own. Sends report
`schematic_runtime::TransportError` when no response head arrived, and
everything else (any status, decode failures, hook failures) inside the
`schematic_runtime::Response` envelope. An invalid client setting is a
`schematic_runtime::BuildError`, returned by the optional `build()` or
carried by `TransportErrorKind::InvalidClient` on the first send.

## Ergonomic Conversions

The generator produces convenience `From` implementations for request structs:

### Single-Parameter Requests

For endpoints with exactly one untyped path parameter and no body, `From<&str>` and `From<String>` are generated. An endpoint whose path parameters are [typed](#typed-path-parameters) gets neither:

```rust
// Instead of:
let req = RetrieveModelRequest::new("gpt-4");
// You can write:
let req = RetrieveModelRequest::from("gpt-4");
let req: RetrieveModelRequest = "gpt-4".into();
```

### Body-Only Requests

For endpoints with a body and no path parameters, `From<BodyType>` is generated:

```rust
// Instead of:
let req = CreateMessageRequest::new(body);
// You can write:
let req = CreateMessageRequest::from(body);
let req: CreateMessageRequest = body.into();
```

## Pagination Support

A list endpoint is paginated when its definition authors a strategy, as the
API's `default_pagination` with an inferred root array or an endpoint
`.pagination(Collection)` declaration. For each one the
generator emits `impl schematic_runtime::pager::Paginated` on the request
type, with the strategy as a constant, plus one capability marker impl per
control the strategy authors; the paging itself is the runtime's. The
endpoint's call type forwards to the pager, so only authored controls exist:

```rust
let releases = gh.list_releases("owner", "repo").page_size(50).collect_all().await?;

let mut pages = gh.list_releases("owner", "repo").pages();
while let Some(page) = pages.next().await? {
    for release in page.items { /* ... */ }
}
```

- A parameter the strategy adopts (for example `page` or `per_page`) is not
  a request field or setter; the pager controls it. An ordinary send of the
  request (`.await`) still sends an adopted parameter the endpoint declares
  required, at the value the pager would send: the first page or offset, or
  the size's authored `sent_default`. An optional one is omitted. To send
  one customized page, use `.page(3).page_size(10).fetch_page()`, which
  returns a `Page<T>` rather than the endpoint's own response.
- An endpoint whose strategy has a server-side session (a scroll or point
  in time) forwards `with_session` instead of `pages`, `fetch_page`, and
  `collect_all`, plus `close_grace(..)` for the close request's time limit
  (5 s by default); its pages, partitions, and `renew()` exist only inside
  that scope, which closes the session when it ends:

  ```rust
  let hits = es.search(index).keep_alive("5m")
      .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
      .await?;
  ```

  A partition without a session forwards `partitioned(n)`, which returns
  `n` independent page iterators.
- A setter named like a pager control the call type forwards gains a
  `_param` suffix, reported as an `endpoint-method-rename` warning.
- Alternate strategies are validated but add no caller controls. Parameters
  cannot carry pagination metadata; collections belong to endpoints.

A paginated GraphQL operation gets the same `Paginated` impl beside its
request struct, with `PATH = ""` and a `page_root` that calls
`schematic_runtime::graphql::page_data`, so every path reads under the reply's
`data` and a page carrying GraphQL errors ends paging. It has no endpoint
method; page it with `client.send(IssuesRequest::new(vars)).pager()` (see
[Paging a generated operation](../graphql.md#paging-a-generated-operation)).

How resolution, adoption, completeness, sessions, and partitions behave for
callers is described in [Pagination](../pagination.md).

## Safety Guarantees

- **Validation**: All generated code is parsed with `syn` before writing
- **Formatting**: Output is consistently formatted with `prettyplease`
- **Atomic writes**: Uses temp file + rename to prevent partial writes
- **No panics**: Production code paths use `Result` types, no `unwrap()`/`expect()`

## Response Hooks

Hooks are client settings, implemented once in the runtime:

```rust
use schematic_schema::prelude::*;
use schematic_schema::openai::ListModelsRequest;

let client = OpenAI::new()
    .base_url("https://staging.example.com/v1")
    .pre_response_json(|meta, json| {
        // Reshape a successful JSON body before it is decoded
        Ok(json)
    })
    .mutate_response::<ListModelsRequest, _>(|meta, models| {
        // Adjust the decoded success value of this one endpoint
        Ok(())
    });
```

Both run only for a 2xx reply to a JSON endpoint. `mutate_response` accepts
only a request type of the same API whose codec supports hooks, so naming
another API's request or a text, binary, or empty endpoint does not compile.
The hook order and failure handling are described in the runtime's `hooks`
module ([`runtime/src/hooks.rs`](../../runtime/src/hooks.rs)).

## OpenAPI Import Pipeline

The `import` subcommand provides a full pipeline for converting OpenAPI 3.x specifications into type-safe Rust clients:

```
┌─────────────────────────────────────────────────────────────────┐
│                    OpenAPI Spec (YAML/JSON)                       │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 1: Parse & Import                        │
│  - Parse OpenAPI 3.x spec (openapiv3 crate)                      │
│  - Map OpenAPI types → Schematic types                           │
│  - Extract endpoints, models, auth strategies                    │
│  - Generate diagnostics for unsupported features                 │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 2: Model Generation                     │
│  - Generate Rust structs/enums from OpenAPI schemas              │
│  - Validate model names for collisions                           │
│  - Write types module                                            │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 3: Client Generation                    │
│  - Generate client code (standalone, no schematic-definitions)   │
│  - Validate, format, and write output                            │
└─────────────────────────────────────────────────────────────────┘
```

### Type Mappings

| OpenAPI Type | Rust Type |
|--------------|-----------|
| `string` | `String` |
| `string` + `format: date-time` | `String` |
| `integer` | `i64` |
| `integer` + `format: int32` | `i32` |
| `number` | `f64` |
| `boolean` | `bool` |
| `array` | `Vec<T>` |
| `object` | Named struct |
| `object` + `additionalProperties` | `HashMap<String, T>` |
| `oneOf`/`anyOf` | Enum; decoded by its `discriminator` when every member has a tag value, else untagged |
| `$ref` | Named type reference |

### Diagnostics

Import produces diagnostics for potential issues:

```rust
pub enum DiagnosticSeverity {
    Info,   // Informational messages
    Warn,   // Approximated or ignored input (fails strict mode)
    Error,  // Input that could not be imported (fails strict mode)
}
```

Use `--strict` to fail on any WARN or ERROR, or `-v` to print all diagnostics.

## OpenAPI Export

The `generate` subcommand can optionally export OpenAPI 3.0.3 specs alongside Rust client generation:

```bash
# JSON export (default)
schematic-gen generate --api openai --openapi-out specs/

# YAML export
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml
```

Exported specs include `x-schematic` extensions carrying the Schematic metadata standard OpenAPI cannot (module path, request suffix, env mapping, auth details, success shapes, declared errors, and more). `import` reads them back, so an exported document re-imports as its definition apart from a few listed facts; see [OpenAPI extensions](../io/openapi-extensions.md#what-round-trips).

APIs that share a generated Rust module are exported as one OpenAPI document
using the module filename. For example, `OllamaNative` and `OllamaOpenAI`
produce `ollama.json`, while `EmqxBasic` and `EmqxBearer` produce `emqx.json`.

> **Note**: OpenAPI `components.schemas` come from each API's own `RestApi::schemas`, so there is nothing separate to register for export. A JSON reference that names no component fails both the Rust and OpenAPI outputs with `invalid-schema-reference`.

### Module Reference: `import_pipeline`

| Function | Description |
|----------|-------------|
| `run_import(&options)` | Full pipeline: parse → import → generate models → generate client |

### Module Reference: `openapi_output`

| Function | Description |
|----------|-------------|
| `write_openapi(&api, &options, &dir)` | Export a single API definition to an OpenAPI spec file named from its resolved module |
| `write_openapi_grouped(&apis, &module_name, &options, &dir)` | Export all APIs in a shared module to one module-named OpenAPI spec file |
| `build_openapi_grouped(&apis, &module_name, &options)` | Build a module's merged OpenAPI document (unique `operationId`s) without writing it |
| `write_openapi_document(&doc, &file_stem, format, &dir)` | Write a built OpenAPI document |
| `operation_id_diagnostics(&doc, &apis)` | Report duplicate `operationId`s and links to undeclared operations in a document |

### Module Reference: `postman_output`

| Function | Description |
|----------|-------------|
| `build_postman_collection(&api)` | Build Postman collection from a single API |
| `build_postman_collection_grouped(&apis, &module_name)` | Build grouped collection from multiple APIs |
| `write_postman(&api, &dir, dry_run)` | Write single-API Postman collection to disk |
| `write_postman_grouped(&apis, &module_name, &dir, dry_run)` | Write grouped Postman collection to disk |

### Module Reference: `export`

Shared normalized types consumed by both OpenAPI and Postman writers:

| Function | Description |
|----------|-------------|
| `export::extract_folder_key(&path)` | Folder key from URL path for Postman grouping |
| `export::auth::map_auth(&strategy)` | Map `AuthStrategy` to `ExportAuth` |
| `export::body::map_body(&request)` | Map `ApiRequest` to `ExportBody` |

## Critical Testing Requirements

> **⚠️ WARNING**: The test suite has known gaps! Read this section carefully.

### What Tests Cover

The current test suite verifies:

1. **Syntax validity** - Generated code parses as valid Rust via `syn`
2. **Formatting** - Output is properly formatted via `prettyplease`
3. **Individual generators** - Each generator function has unit tests
4. **Auth translation** - Every authentication pattern produces the expected runtime `AuthConfig` in `definition()`
5. **Runtime ABI** - A unit test keeps `RUNTIME_ABI` equal to `schematic_runtime::ABI`, and partial-generation tests cover matching, different, missing, and malformed `lib.rs` markers
6. **Fresh-output guard** - The `slow_*` tests in `tests/l1/fresh_output_compile.rs` (`just test-fresh-output`, part of `just test`) generate fresh crates into temporary directories and:
    - compile the fixture APIs and run wiremock tests that send through the runtime and assert the bytes each request puts on the wire, credential sourcing and refusal, codec selection, hooks, and definition settings (request-id header, secret names, cross-origin policy); the `FixtureShapes` API covers several success shapes, SSE, NDJSON, and byte streams, and declared error tables, and the `FixtureGraphQl` API covers GraphQL addresses, reply classification, and a Relay pager
    - compile all 18 real APIs, including their WebSocket modules, with `cargo check --offline --locked`
    - compile the host module generated from a Bearer-authenticated WebSocket fixture (no shipped definition uses Bearer), then start that host on a loopback port and check that a missing or wrong bearer token gets HTTP 401 and a matching one upgrades and echoes a frame
    - compile typed WebSocket fixtures (payloads without `Clone`, empty message directions, JSON text and binary framing) and a multi-endpoint host that names no protocol, then run them on loopback ports: each inbound frame selects the variant its selectors name (the most specific of overlapping ones, else the fallback), an unmatched, non-JSON, or undecodable frame is an error that keeps the frame and the connection, `send` writes the payload alone, typed and value receives share one queue, and the host's post-authentication hook runs once before any reply, never after a rejected upgrade, and closes the connection when its write fails; sending a message of the wrong type to `send` does not compile
    - compile the client module generated from a WebSocket fixture whose handshake uses a path parameter and every supported query form, then check the request target a loopback server receives: the path value encoded as one segment, unset optional values left out, exploded arrays repeated, and delimited arrays joined
    - compile WebSocket client fixtures for every handshake credential placement (API-key header, query, and cookie; Basic; Bearer) and a correlated endpoint with string request ids, then check on loopback ports that the upgrade carries exactly one credential from the first source that has one (per-connect, client, first non-empty environment variable), merges a cookie key without duplicating its name, connects anonymously when nothing resolves, is not retried after a 401, does not follow a redirect (the other listener receives nothing), and that a typed request sends its id as a JSON string, returns only its declared response, leaves a same-id frame of another message as an event, and frees its pending slot on timeout and cancellation
    - compile a WebSocket client fixture with an opening message bound to the handshake credential and a challenge-flow setup callback, then check on loopback ports that the opening message is the first frame of every connection with its key field filled from the credential, a conflicting key fails before any dial, setup runs before `Ready` on the first connection and on each reconnect, a setup rejection fails the connect after one connection, the reconnect budget counts only consecutive failed redials, and a silent server trips the receive timeout and is redialed
    - check, in one `cargo check --tests --keep-going` run, that every listed misuse fails to compile, among them: a wrong success type, a request impl missing its codec, a non-request type, another API's request passed to `send`, `send_raw`, `check_link`, or `mutate_response`, a mutator on a text, binary, or stream endpoint or on several shapes that are not all JSON, GraphQL variables of another type, a pager knob a GraphQL operation does not author, an endpoint setter after `.full()`, the data instead of the envelope expected from `.full()`, and the removed client alias, per-API builder, `request` method, request enum, and `with_client_and_base_url` constructor
    - build against a doctored runtime copy with another ABI and assert the build fails with the readable assertion message
    - compare the query request lines with a recorded baseline

Sending itself (envelope, transport errors, timeouts and deadlines,
redirects, credential resolution, redaction, hooks) is tested in the
runtime crate's own suite (`runtime/tests/`).

### What Tests DO NOT Cover

**Live HTTP behavior against real providers is NOT tested!** Specifically:

1. **Provider integration** - No tests exercise real API endpoints
2. **Generated request encoding for real APIs** - Wire bytes are checked against wiremock for the fixture APIs and for the query baseline only
3. **Auth at runtime** - No end-to-end test validates real credential sourcing against external services

### Known Failure Modes

| Scenario | What Goes Wrong | How to Catch It |
|----------|-----------------|-----------------|
| Wrong success shape in a definition (e.g. `Json` for an audio endpoint) | The `Json` codec reports a `MalformedSuccess` error in the envelope, with the body kept | Check the endpoint's `type Decoder` in the generated `requests.rs` (or module file) |
| Module path mismatch | Compile failure: unresolved import | `just test-fresh-output` |
| Shared module missing `request_suffix` | Compile failure: duplicate struct names | `just test-fresh-output` |
| Generated crate built against another runtime | Compile failure naming both ABIs and `just generate` | Regenerate with the matching `schematic-gen` |

### Required Manual Verification

When modifying response handling or module generation:

```bash
# 1. Generate code
cargo run -p schematic-gen -- generate --api elevenlabs --output schematic/schema/src

# 2. Verify the codec chosen for non-JSON endpoints
grep -n "type Decoder" schematic/schema/src/elevenlabs/requests.rs
# Should see Json<T>, Binary, ByteStream, and Ndjson<T, End> as the
# definition declares (Text, Empty, Sse<T, End>, and Shapes<..> elsewhere)

# 3. Check it compiles (schematic-schema is excluded from the workspace)
cargo check --manifest-path schematic/schema/Cargo.toml

# 4. Run tests, including the fresh-output guard (necessary but not sufficient!)
just -f schematic/justfile test
```

### Future Work

To close the testing gap, we need:

1. **E2E smoke tests** - Optional integration tests against real APIs (gated by feature flag)

## Selecting modules with Cargo features

Each generated module is behind a Cargo feature named for the module, and
the default features enable every module. A consumer that needs only some
APIs turns the defaults off and lists the modules it uses:

```toml
[dependencies]
schematic-schema = { path = "../schematic/schema", default-features = false, features = ["anthropic", "elevenlabs_ws"] }
```

- **One feature per module, not per API.** APIs that share a module share
  its feature: `ollama` enables both `OllamaNative` and `OllamaOpenAI`.
- **Features are additive.** A REST module and a WebSocket module can be
  enabled together; with no feature at all the crate still builds and holds
  only `shared`, `prelude`, and the runtime re-export.
- **Everything about a module follows its feature:** the `pub mod`
  declaration, its `prelude` re-exports, and its rows in the crate and
  prelude docs. `ws_shared` compiles when any WebSocket module is enabled.
  A disabled module is simply absent; naming it fails with rustc's
  "the item is gated behind the `<module>` feature" note.
- **WebSocket transport is optional.** `futures-util`, `thiserror`, and
  `tokio-tungstenite` are used only by WebSocket modules, so they are
  optional dependencies enabled by each WebSocket module's feature. A
  REST-only selection does not build `tokio-tungstenite`.
- **The manifest comes from the whole registry.** Every run, full or
  partial, writes the same `[features]` table, so selecting one API for
  regeneration never drops another module's feature.

A module named `default` is refused by validation (`invalid-module-path`),
since it would collide with the `default` feature.

## Dependencies

The generated code requires these runtime dependencies (automatically included in generated `Cargo.toml`):

```toml
[dependencies]
bytes = "1"
futures-util = { version = "0.3", optional = true }
schematic-define = { version = "0.1.0", path = "../define" }
schematic-definitions = { version = "0.1.0", path = "../definitions" }
schematic-runtime = { version = "0.1.0", path = "../runtime" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = { version = "2.0", optional = true }
tokio = { version = "1.43", features = ["rt", "macros", "sync", "time", "net"] }
tokio-stream = "0.1"
tokio-tungstenite = { version = "0.26", default-features = false, features = ["connect", "rustls-tls-native-roots"], optional = true }
urlencoding = "2.1"

[features]
default = ["anthropic", "elevenlabs_ws", "ollama"] # every module, abridged here
anthropic = []
elevenlabs_ws = ["dep:futures-util", "dep:thiserror", "dep:tokio-tungstenite"]
ollama = []

[dev-dependencies]
serial_test = "3"
trybuild = "1.0"
wiremock = "0.6"
```

REST requests are sent by `schematic-runtime`, so the generated crate does
not depend on an HTTP client directly; the WebSocket modules use
`tokio-tungstenite`. The template lives in `src/cargo_gen.rs`. When the
registry has no WebSocket module, the three WebSocket-only dependencies are
left out of the manifest.

## License

AGPL-3.0-only
