---
name: schematic
description: Expert knowledge for Schematic REST and WebSocket API client code generation. Use when defining APIs, generating typed Rust clients, importing OpenAPI specs, adding endpoints, configuring authentication, building headers programmatically, or troubleshooting code generation issues.
---

# Schematic

Type-safe REST and WebSocket API client code generation for Rust. Define APIs declaratively, generate strongly-typed clients automatically.

## Quick Reference

| Package | Purpose |
|---------|---------|
| `schematic-define` | Primitives: `RestApi`, `Endpoint`, `AuthStrategy`, `CrossOrigin`, `Headers` (WebSocket clients), `ApiRequest`, `ApiResponse` |
| `schematic-definitions` | Pre-built APIs: Anthropic, OpenAI, ElevenLabs, HuggingFace, LM Studio, Ollama, EMQX, GitHub, GitLab, Gitea |
| `schematic-gen` | Code generator CLI with `generate`, `validate`, and `import` commands |
| `schematic-runtime` | Hand-written runtime every generated REST client wraps a `Client<Tag>` of: infallible `Client::new` with chained settings validated by `build()?` or on the first call (`TransportErrorKind::InvalidClient`), `api_client!` forwarding them onto each API struct, `SharedTransport`, tighten-only `OriginPolicy`, awaitable `Call` (`.await` → `Result<T, CallError<E>>`, `.full()` → `Response` envelope that is `Ok` iff a response head arrived, else `TransportError`), `send_raw(request, &CallOptions)` → `RawResponse`, credentials (per call → client → env → anonymous), redirects hop by hop (`MAX_REDIRECTS`, `check_link`), shared `Deadline`, codecs `Json`/`Text`/`Binary`/`Empty`, JSON hooks, redacted metadata. Never retries; call futures are boxed. Docs: `schematic/docs/runtime/` |
| `schematic-schema` | Generated clients (auto-generated, do not edit): per API one client struct `{Api}` and its tag `{Api}Api`, and per endpoint a request struct implementing `EndpointSpec`, an endpoint method on `{Api}`, and a `{Endpoint}Call` type |

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

The `request_suffix` prevents naming collisions when APIs have overlapping endpoint IDs (e.g., both have `Embeddings` → `EmbeddingsNativeRequest` vs `EmbeddingsOaiRequest`); call types follow, since `Request` becomes `Call` (`EmbeddingsNativeCall`). The generator automatically combines shared-module APIs into a single output file; generating one member (`generate --api ollama-native`) regenerates the whole shared module and keeps every other module. Full generation (`--api all`) deletes obsolete generator-owned modules.

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

Each API is one client struct, `pub struct OpenAI(schematic_runtime::Client<OpenAIApi>)`,
with `BASE_URL`, `DOCS_URL`, `definition()`, an infallible `new()`, and
`basic_auth()` only when the API accepts Basic; `schematic_runtime::api_client!`
gives it every client-wide setting and send. Each request struct implements
`schematic_runtime::EndpointSpec` with `type Api = OpenAIApi` (a tag callers
never write), so a client accepts only its own API's requests. Each REST
endpoint is also an inherent method on the client struct (`list_models`)
returning a `{Endpoint}Call` (`ListModelsCall`); naming and rename rules are in
[gen.md](./gen.md#endpoint-methods-and-call-types).

### Constructing and Sending

```rust
use schematic_schema::prelude::*;

let client = OpenAI::new();                        // never fails
let github = GitHub::new();
let staging = OpenAI::new()
    .base_url("https://staging.api.com/v1")
    .timeout(std::time::Duration::from_secs(10))
    .build()?; // optional: BuildError now; otherwise the first send reports it

// Nothing is sent until `.await`: Ok(ListModelsResponse) or Err(CallError)
let models = client.list_models().await?;

// Optional values are setters; `.full()` (chain it last) returns the
// Response envelope: Ok whenever a response head arrived (any status),
// Err(TransportError) only when none did.
let response = github.list_releases("owner", "repo").per_page(10).full().await?;

match github.get_repository("owner", "repo").await {
    Ok(repository) => { /* ... */ }
    Err(CallError::Transport(error)) => { /* no response head arrived */ }
    Err(CallError::Api { error, meta }) => { /* error status, bad body, hook refusal */ }
    Err(_) => { /* CallError is #[non_exhaustive]: a wildcard arm is required */ }
}
```

`.build()?` checks a call's arguments without sending it, running the send's
own first-hop preparation (encoding, headers, credential kind and placement)
with the call's per-call credential; the same checks run again at `.await`. For generic code over request objects, `client.send(request)` takes
one `{Endpoint}Request` and returns the same kind of call; `send_raw(request,
&CallOptions)` skips decoding and is the only API that takes `CallOptions`.
Details: `schematic/docs/runtime/calling-an-api.md`.

### Credentials

The runtime resolves one credential per send: per call → client → first
non-empty env var of the definition's mapping → anonymous. There is no
missing-credential error; a server that needs one answers (typically 401),
which `.await` reports as `CallError::Api` (inside the envelope with `.full()`).

```rust
client.list_models().credential(Credential::access_token(token)).await?; // per call
let client = OpenAI::new().api_key(token); // per client, wherever the API takes its key
let client = OpenAI::new().env_order(["STAGING_API_KEY"]); // replaces env_auth order
client.list_models().anonymous().await?; // no credential at all
```

A credential never overrides a same-named query, cookie, or header value the
request sets itself; that send fails as `InvalidArgument` before anything is
sent.

### Response Hooks

`pre_response_json(..)` (client-global) and
`mutate_response::<Request, _>(..)` run on 2xx replies to JSON
endpoints of the same API only; naming a text, binary, or empty endpoint, or
another API's request, does not compile. Paging (`pages`, `fetch_page`,
`collect_all`, including session pages) runs only the global JSON hook, before
items and continuations are extracted; it skips the typed endpoint mutator.
Ordinary endpoint sends still run both hooks.

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
        auth_policy: None,
        env_auth: vec!["MY_API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![/* ... */],
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: None,
        cross_origin: CrossOrigin::same_origin(), // redirects and next-page links
        request_id_header: None,                  // e.g. Some("x-request-id".to_string())
        secret_names: vec![],                     // extra names never stored in metadata
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
    oauth_scopes: None,
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
    oauth_scopes: None,
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
    oauth_scopes: None,
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

## Pagination recovery

`Pages::retry().await` explicitly retries every failure category and returns
`Result<(), PagerError<_>>`; the next iteration yields a recovered page once.
`Items::retry()` forwards recovery. Other iteration calls retain the specific
latest error with complete evidence and send nothing. An unusable continuation
on an already delivered page is repaired by refreshing that source page without
redelivering its items. Loop history, direction, partition, and deadline remain;
retry never promises success. `PagerError::Api` shares `Arc<ApiError<E>>`, so
provider errors need no `Clone`; `Paginated` requires `Failure: Sync`.
Collection and single-page fetch consume their traversal; retain `pages()` for
manual recovery. Full rules: `schematic/docs/pagination.md`.

## Pagination sessions

Session partitions require a shared session opened by a separate request
(`Session::opened_by(..)`). Definition validation rejects partitions combined
with `Session::opened_by_first_page()` as `partitioned-first-page-session`.
First-page sessions without partitions remain supported; shared partition
sessions keep the latest observed rotating handle and close once. See
`schematic/docs/pagination.md` for lifecycle and cleanup guarantees.

## Detailed Documentation

- [Define Package](./define.md) - API definition primitives, Headers builder
- [Definitions Package](./definitions.md) - Pre-built API catalog
- [Generator Package](./gen.md) - Code generation, import command
- [OpenAPI Support](./openapi.md) - Import/export OpenAPI specs
- [Headers Builder](./headers.md) - `Headers` for WebSocket clients, env resolution
- Runtime topic pages: `schematic/docs/runtime/` - envelope, credentials, redirects, deadlines, hooks, redaction

## Troubleshooting

| Issue | Cause | Fix |
|-------|-------|-----|
| `schematic_definitions::xyz not found` | Module path mismatch | Set `module_path` explicitly |
| Duplicate struct definitions | Shared module missing `request_suffix` | Set distinct `request_suffix` on each API |
| Recursive struct definition | Body type name collision | Rename to `*Body` suffix |
| Binary endpoint reports `MalformedSuccess` | Wrong `ApiResponse` | Use `ApiResponse::Binary` |
| 401 as `CallError::Api` (or in the envelope), metadata shows `AuthMethod::Anonymous` | No credential from any source | Pass `Credential` per call/client, or check `env_auth` var names |
| Send fails with `InvalidArgument` before any request | Request sets a same-named query/cookie/header value | Drop the request value or the credential |
| `endpoint-method-rename` warning / `generated-name-collision` error | Endpoint id or parameter clashes with a generated method name | See [gen.md](./gen.md#endpoint-methods-and-call-types); rename the id or parameter |
| Build fails naming two `schematic-runtime` ABIs | Generated crate older than the runtime | Regenerate (`just generate`) |

## Definition migration safety net

`just -f schematic/justfile migration-check` checks historical JSON definition/pagination/ordered-registry snapshots and current Rust/OpenAPI/Postman artifacts without updating them. The normal L1 artifact suite includes a complete Rust source file-set/byte and Cargo.toml comparison. REST definition and parameter types implement plain Serde; optional omission/null means None and required collection omissions are errors. Read `schematic/docs/definition-snapshots.md` before changing the format or capture tooling.

## Typed registration

`schematic-define` now has additive `DefinitionContext`, `TypeDesc`, `Registration`,
`definition_type!`, and typed `rust_path!` APIs. Declare nominal implementations
beside model declarations; no blanket JsonSchema dispatcher or type_name parsing.
Register exceptional aliases first: first canonical registration fixes name/path/order.
Containers recurse, primitives/Value stay inline, and explicit unused components survive.
Call `validate_reference(api, endpoint, &desc)` on transfer data: the destination must
have registered the whole matching wrapper shape. Reserved Option<TypeRef> structure
is always None for authored registrations; non-null data fails validation.
`schematic-definitions::registry::{assemble_module_group, SchemaRegistry::from_context}`
produce definitions and ordered schemas from one context; current literal lookups
remain until migration. Read `schematic/docs/typed-registration.md`. Shipped Ollama
PullModelProgress/PushModelProgress currently lack JsonSchema; alias coverage uses a
small fixture pending their migration. `just doctest` runs descriptor compile-fail
examples; the existing fresh-output misuse guard also checks their diagnostic codes.

## Required-fact REST builders

Use `Endpoint::{get,post,put,patch,delete,head,options}` or `new(id, method, path)`,
then `describes` and one success declaration before fallible `build`. JSON methods
receive `&mut DefinitionContext`; build endpoints before borrowing that context
with `RestApi::builder(&ctx, name, base_url)`. API description and one explicit
no_auth/bearer_env/basic_env/authentication choice are required. Repeated success
or auth choices fail; explicit env mappings must agree independent of setter order.
`Success` and `ErrorSet` are non-exhaustive placeholders; only `ErrorSet::none()`
is available. Typed Schema descriptor evidence survives lowering and is validated
at API build; JSON schemas without evidence fail the builder too. OpenAPI export consumes its wire shape. Root typed Vec responses
retain GET list inference, while Option/nominal wrappers do not. Legacy providers,
import emitter and external literals remain until migration. Read
`schematic/docs/definition-builders.md` for accepted forms and the Default allow-list.
