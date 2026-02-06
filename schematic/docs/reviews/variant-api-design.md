## Summary
This document proposes replacing the current `variant(base_url, env_auth, strategy)` method
with a builder-style `variant()` API that preserves all existing behavior and adds
response hooks. The new hooks support pre-response JSON remapping (to fit existing
response types) and post-deserialization mutation of response objects, with an
explicit path for additive response properties.

## Goals

- Preserve everything the current `variant()` call can do (base URL, env auth names,
  auth update strategy).
- Provide a fluent builder for partial overrides instead of mandatory args.
- Add a pre-response JSON hook to remap incoming payloads to existing response
  structs.
- Add a post-response hook to mutate response structs after deserialization.
- Keep the default request/response fast path when no hooks are configured.

## Non-goals

- Changing the API definition model in `schematic/define`.
- Adding runtime introspection of unknown response shapes.
- Supporting full response type substitution for convenience methods
  (still returns the defined response type unless a separate method is used).

## Current behavior (baseline)

- `variant(base_url, env_auth, UpdateStrategy)` clones the reqwest client and
  replaces base URL + auth envs, optionally changing auth strategy.
- Response handling is fixed: JSON responses deserialize directly into the
  declared response type with no hooks.

## Proposed API surface

### `variant()` returns a builder
The existing `variant(...) -> Self` signature is replaced with a builder entry
point. The builder defaults to the base client settings and allows overrides.

```rust
use schematic_define::{AuthStrategy, UpdateStrategy};

let api = OpenAI::new();

let variant = api
    .variant()
    .base_url("https://staging.api.com/v1")
    .env_auth(["STAGING_API_KEY"])
    .auth_update(UpdateStrategy::NoChange)
    .pre_response_json(|ctx, json| {
        // Example: unwrap `{ data: ... }` payloads to match existing response types
        Ok(json.get("data").cloned().unwrap_or(json))
    })
    .mutate_response::<ListModelsRequest>(|_ctx, response| {
        response.data.retain(|model| !model.id.contains("deprecated"));
        Ok(())
    })
    .build();
```

### Back-compat convenience
Keep a helper for the legacy signature to minimize disruption:

```rust
pub fn variant_with(
    &self,
    base_url: impl Into<String>,
    env_auth: Vec<String>,
    strategy: UpdateStrategy,
) -> Self {
    self.variant()
        .base_url(base_url)
        .env_auth(env_auth)
        .auth_update(strategy)
        .build()
}
```

## Response hook model

### Shared context
Introduce a `ResponseContext` (in `schematic/schema/src/shared.rs`) to carry
metadata to hooks:

```rust
pub struct ResponseContext {
    pub endpoint_id: &'static str,
    pub method: &'static str,
    pub path: String,
    pub url: String,
    pub status: u16,
    pub headers: reqwest::header::HeaderMap,
}
```

### Pre-response JSON remap (required)
This hook runs after a successful HTTP response but before deserialization.
It allows a variant to reshape a payload so the existing response type can
deserialize cleanly.

```rust
pub type PreResponseJsonHook =
    dyn Fn(&ResponseContext, serde_json::Value) -> Result<serde_json::Value, SchematicError>
        + Send
        + Sync;
```

Implementation detail: in `request<T>()`, if a pre-response hook is configured,
the response body is read into bytes, parsed as JSON `Value`, passed through the
hook, and then deserialized into `T` via `serde_json::from_value`.

### Post-response mutation (required)
This hook runs after deserialization and can mutate the response object.

```rust
pub type ResponseMutator<T> =
    dyn Fn(&ResponseContext, &mut T) -> Result<(), SchematicError> + Send + Sync;
```

Registration is keyed by endpoint request type to keep it type-safe:

```rust
variant
    .mutate_response::<ListModelsRequest>(|ctx, response| {
        response.object = format!("{}-patched", response.object);
        Ok(())
    })
    .mutate_response::<RetrieveModelRequest>(|ctx, response| {
        response.owned_by = "staging".to_string();
        Ok(())
    });
```

### Additive properties
Pre-response JSON hooks can insert new fields, but they will only be visible
to callers if the response type has a field to receive them (for example,
`#[serde(flatten)] extra: HashMap<String, Value>`). For responses that need
additive properties, there are two viable paths:

1. Add a `#[serde(flatten)] extra: HashMap<String, Value>` field in the
   response type(s) that need extension.
2. Provide an optional `request_with_extensions()` API that returns a wrapper
   (e.g. `VariantResponse<T> { value: T, extensions: HashMap<String, Value> }`)
   populated by the pre-response hook.

Either option preserves existing response types while allowing additive data
to be surfaced intentionally.

## Builder and hook storage

### Builder shape
Each generated API module gets a builder tailored to its client:

```rust
pub struct OpenAIVariantBuilder<'a> {
    base: &'a OpenAI,
    base_url: Option<String>,
    env_auth: Option<Vec<String>>,
    auth_update: UpdateStrategy,
    auth_override: Option<AuthStrategy>,
    env_username: Option<String>,
    headers: Vec<(String, String)>,
    pre_response_json: Option<Arc<PreResponseJsonHook>>,
    response_mutators: HashMap<&'static str, Arc<dyn AnyResponseMutator>>,
}
```

`build()` produces a new API client instance with a `variant_hooks` field
containing the configured hooks and overrides applied.

### Endpoint identity
To keep hook registration type-safe, generate a small trait that each request
type implements:

```rust
pub trait EndpointSpec {
    type Response;
    const ENDPOINT_ID: &'static str;
}
```

Generated request structs implement this trait, allowing:

```rust
pub fn mutate_response<R, F>(self, hook: F) -> Self
where
    R: EndpointSpec,
    F: Fn(&ResponseContext, &mut R::Response) -> Result<(), SchematicError>
        + Send
        + Sync
        + 'static,
```

## Request pipeline updates (generated clients)

- Capture `endpoint_id` before consuming the request enum.
- Build `ResponseContext` once a response is received and successful.
- If no hooks are configured, keep the current fast path:
  `response.json::<T>().await`.
- If hooks exist, use the hook pipeline:
  `bytes -> json Value -> pre_response_json -> deserialize -> post_response`.

This keeps overhead near-zero for the common case while enabling powerful
variants when explicitly configured.

## Impact on `schematic/define`
No changes required. The builder and hook types live in generated clients and
`schematic/schema/src/shared.rs`. If we later decide to expose a shared
endpoint trait or response hook type across crates, we can add it to
`schematic/schema` without touching `schematic/define`.

## Migration notes

- Introduce `variant()` builder and `variant_with(...)` helper in generated
  clients.
- Deprecate the existing `variant(base_url, env_auth, strategy)` signature
  in a minor release once `variant_with(...)` is available.
- Update codegen tests to assert the builder methods and hook wiring.
