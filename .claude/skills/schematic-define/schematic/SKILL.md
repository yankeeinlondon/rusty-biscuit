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
| `schematic-definitions` | Pre-built APIs: Anthropic, OpenAI, ElevenLabs, HuggingFace, LM Studio, Ollama, EMQX, GitHub, GitLab, Gitea |
| `schematic-gen` | Code generator CLI with `generate`, `validate`, and `import` commands |
| `schematic-runtime` | Hand-written request runtime: managed `Client<A>` (built from an `ApiDefinition` through `ClientBuilder`, optional `SharedTransport`) and its typed `send(request, &CallOptions)`, which follows redirects itself hop by hop under the API's `OriginPolicy` (same_origin default; `allow_origins`, `allow_without_credentials`, `danger_allow_any`, `deny`; `ClientBuilder::cross_origin` may only tighten it; the credential goes only to the base origin or a listed origin; at most `MAX_REDIRECTS` = 10; a refused redirect is `RedirectRefused`, a later hop with no head `RedirectFailed` keeping the last 3xx), plus `Client::check_link::<R>(&Url)` for next-page links and absolute request URLs checked before sending (`CrossOriginRejected`); `Response` envelope, `ApiError`/`ErrorKind`, `TransportError`, redacted `RequestMeta`/`ResponseMeta` (`SecretNames`); `AuthConfig`/`CredentialPlacement`/`EnvFallback` and `Credential` (per call → client → first non-empty env var → anonymous); `Deadline`; codecs `Json`, `Text`, `Binary`, `Empty`; the `EndpointSpec` and `Decode` contracts; `Client::send_raw` (same execution core as `send`, returns `RawResponse { meta, body, completion }` with `RawCompletion::{Complete, BodyInterrupted, RedirectRefused, RedirectFailed}`, any 4xx/5xx is `Complete`, `json::<T>()` refuses an incomplete body, no hooks); response hooks on `ClientBuilder` — `pre_response_json` (client-global) and `mutate_response::<R>` (typed by `R::Success`, needs `R::Decoder: SupportsHooks`, implemented only by `Json`, so text/binary/empty mutators do not compile), run on 2xx JSON only in the order parse → global → decode → mutator, keyed by API + endpoint id; a hook returns `HookError` (text discarded) → `ErrorKind::Hook` with the original bytes. Every generated REST client is a `Client<Api>` of this runtime |
| `schematic-schema` | Generated clients (auto-generated, do not edit): per API a marker type, `{Api}Client` alias, and per endpoint a request struct implementing `EndpointSpec` |

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
#                 ollama-native, ollama-openai, emqx-basic, emqx-bearer,
#                 github, gitlab, gitea, all
```

## Critical Configuration

### Response Types - Choose Correctly

| `ApiResponse` Variant | Codec (`type Decoder`) | Success type | Use For |
|-----------------------|------------------------|--------------|---------|
| `ApiResponse::Json(Schema)` | `Json<T>` | `T` | JSON responses (most common) |
| `ApiResponse::Binary` | `Binary` | `bytes::Bytes` | Audio, images, archives |
| `ApiResponse::Text` | `Text` | `String` | Plain text |
| `ApiResponse::Empty` | `Empty` | `()` | 204 No Content |

**Common Mistake**:
```rust
// WRONG - Audio endpoint returning binary data
Endpoint { response: ApiResponse::json_type("AudioResponse"), ... }  // MalformedSuccess in the envelope

// CORRECT
Endpoint { response: ApiResponse::Binary, ... }  // Success type bytes::Bytes
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

The `request_suffix` prevents naming collisions when APIs have overlapping endpoint IDs (e.g., both have `Embeddings` → `EmbeddingsNativeRequest` vs `EmbeddingsOaiRequest`). The generator automatically combines shared-module APIs into a single output file; generating one member (`generate --api ollama-native`) regenerates the whole shared module and keeps every other module. Full generation (`--api all`) deletes obsolete generator-owned modules.

OpenAPI and Postman artifacts use the same resolved module name as generated
Rust files. Shared-module APIs emit one grouped artifact, e.g.
`ollama.json` / `ollama.postman_collection.json` for `OllamaNative` +
`OllamaOpenAI`, and `emqx.json` / `emqx.postman_collection.json` for
`EmqxBasic` + `EmqxBearer`.

### Body Type Naming Convention

Use `*Body` suffix to avoid collision with generated `*Request` wrappers:

```rust
// WRONG - Collision with generated wrapper
pub struct GenerateRequest { ... }  // Conflicts with generated GenerateRequest

// CORRECT
pub struct GenerateBody { ... }  // Generated: struct GenerateRequest { body: GenerateBody }
```

## Generated Client Features

Each API is an uninhabited marker (`pub enum OpenAI {}`) with `BASE_URL`,
`DOCS_URL`, `definition()`, `builder()`, and `new()`; the client is
`OpenAIClient = schematic_runtime::Client<OpenAI>`. Each request struct
implements `schematic_runtime::EndpointSpec`, so a client accepts only its
own API's requests.

### Constructing and Sending

```rust
use schematic_schema::prelude::*;
use schematic_schema::openai::ListModelsRequest;

let client = OpenAI::new()?;                       // Result<OpenAIClient, BuildError>
let staging = OpenAI::builder()                    // runtime ClientBuilder
    .base_url("https://staging.api.com/v1")
    .timeout(std::time::Duration::from_secs(10))
    .build()?;

// Ok(Response) whenever a response head arrived (any status);
// Err(TransportError) only when none did. `send_raw` skips decoding.
let response = client.send(ListModelsRequest::default(), &CallOptions::new()).await?;
let models = response.into_result()?;
```

### Credentials

The runtime resolves one credential per send: per call → client → first
non-empty env var of the definition's mapping → anonymous. There is no
missing-credential error; a server that needs one answers (typically 401)
inside the envelope.

```rust
let options = CallOptions::new().credential(Credential::access_token(token)); // per call
let client = OpenAI::builder().credentials(Credential::access_token(token)).build()?; // per client
let client = OpenAI::builder().env_order(["STAGING_API_KEY"]).build()?; // replaces env_auth order
client.send(request, &CallOptions::anonymous()).await?; // no credential at all
```

A credential never overrides a same-named query, cookie, or header value the
request sets itself; that send fails as `InvalidArgument` before anything is
sent.

### Response Hooks

`builder().pre_response_json(..)` (client-global) and
`builder().mutate_response::<Request, _>(..)` run on 2xx replies to JSON
endpoints of the same API only; naming a text, binary, or empty endpoint, or
another API's request, does not compile.

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

| Strategy | Configuration | Runtime credential | On the wire |
|----------|---------------|--------------------|-------------|
| `BearerToken { header: None }` | `env_auth: vec!["KEY"]` | `Credential::access_token` | `Authorization: Bearer <token>` |
| `ApiKey { header: "X-Key" }` | `env_auth: vec!["KEY"]` | `Credential::api_key` | `X-Key: <key>` |
| `Basic` | `env_username`, `env_auth[0]` | `Credential::basic` | `Authorization: Basic <base64>` |
| `ApiKeyParam { location, name }` | `env_auth` | `Credential::api_key` | `?api_key=<key>` or appended to the one `Cookie` header |
| `None` | (none) | (none; a per-call credential is `InvalidArgument`) | (none) |

## Testing & Verification

**CRITICAL**: Tests run fixture APIs through the runtime against wiremock, NOT against live providers!

```bash
# 1. Run L1 tests, including the fresh-output guard (compiles freshly generated crates offline)
just -f schematic/justfile test

# 2. Regenerate schemas
just -f schematic/justfile generate

# 3. Verify compilation (schematic-schema is excluded from the workspace)
cargo check --manifest-path schematic/schema/Cargo.toml

# 4. For response type changes, check the codec each endpoint declares:
grep -rn "type Decoder" schematic/schema/src/
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
| Binary endpoint reports `MalformedSuccess` | Wrong `ApiResponse` | Use `ApiResponse::Binary` |
| 401 in the envelope, metadata shows `AuthMethod::Anonymous` | No credential from any source | Pass `Credential` per call/client, or check `env_auth` var names |
| Send fails with `InvalidArgument` before any request | Request sets a same-named query/cookie/header value | Drop the request value or the credential |
| Build fails naming two `schematic-runtime` ABIs | Generated crate older than the runtime | Regenerate (`just generate`) |
