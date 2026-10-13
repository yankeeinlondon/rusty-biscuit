---
name: schematic-define
description: Expert knowledge for defining REST and WebSocket APIs with `schematic-define`. Use when creating or editing `RestApi`/`Endpoint` models, auth and env mappings, request or response shapes, and OpenAPI extension behavior that drives generated clients.
---

# schematic-define

Use this skill when working in `schematic/define` or when updating docs/tests that describe `schematic-define` behavior.

## Scope

`schematic-define` is the definition layer for Schematic:

- `RestApi`, `Endpoint`, `RestMethod`
- `AuthStrategy`, `ApiKeyLocation`, `UpdateStrategy`
- `ApiRequest`, `ApiResponse`, `FormField`, `Schema`
- `Headers` (credential headers of generated WebSocket clients; REST clients use runtime `Credential`s), `EnvMapping`, `EnvList`, `SensitiveString`
- OpenAPI vendor extension models (`x-schematic`)

Code generation lives in `schematic-gen`, request execution in the hand-written `schematic-runtime`, and the generated clients in `schematic-schema`, but this crate defines the contracts they consume.

## High-Signal Rules

### 1) Response Type Selection Is Runtime-Critical

Pick the right `ApiResponse`, because the generated `EndpointSpec` impl takes its success type and codec from it:

| `ApiResponse` | `type Decoder` | `type Success` |
|---|---|---|
| `Json(Schema)` | `schematic_runtime::Json<T>` | `T` |
| `Binary` | `schematic_runtime::Binary` | `bytes::Bytes` |
| `Text` | `schematic_runtime::Text` | `String` |
| `Empty` | `schematic_runtime::Empty` | `()` |

If an endpoint returns bytes/text but is modeled as JSON, callers get the wrong success type at compile time, and at runtime the `Json` codec reports a `MalformedSuccess` error in the response envelope (body kept).

There is no streaming variant. SSE and NDJSON endpoints are modeled as `Binary`
and framed by the caller — see `definitions/src/ollama/mod.rs:316`.

### 1a) Request Bodies Reach the Client (all variants)

`Json`, `FormData`, and `UrlEncoded` all generate a real body. A form endpoint
gets a synthesized `{EndpointId}Form` struct — file parts typed
`shared::FormFile`, text parts `String`, optional fields `Option<T>` — reached
through the usual `pub body:` field.

`EndpointSpec::into_parts()` returns a `schematic_runtime::RequestParts`
struct (`method`, `path`, `query`, `headers`, `body`) or an `InvalidArgument`.
`body` is a `RequestBody` (`Empty` / `Json` / `Multipart` / `UrlEncoded` /
`Raw`); `query` holds declared query parameters as unencoded pairs kept out of
the path, which the runtime encodes once (`%20` for a space). `Text` bodies are
a `body: String` field and `Binary` bodies a `body: Vec<u8>` field, both sent
as `RequestBody::Raw` under the declared content type. Header and cookie
parameters (`EndpointParams.header`/`cookie`) become `with_*` fields sent in
`headers`; cookies always end up in one `Cookie` header.

### 2) Auth Strategies Include Query/Cookie API Keys

`AuthStrategy` is broader than bearer/header-only patterns:

- `BearerToken { header: Option<String> }`
- `ApiKey { header: String }`
- `ApiKeyParam { location: ApiKeyLocation, name: String }`
- `Basic`
- `None`

`ApiKeyLocation` supports:

- `Query`
- `Cookie`

The generated `definition()` translates the effective auth policy into the
runtime's `AuthConfig` (`ApiKeyParam` becomes `CredentialPlacement::ApiKeyQuery`
or `ApiKeyCookie`). The runtime resolves one credential per send, for every
placement: per call (`.credential(..)` on the call) → client (`api_key(..)`,
`basic_auth(..)`, or `credentials(..)`) → first non-empty env var of the
definition's mapping (`env_order([..])` replaces the order) → anonymous.
`api_key(secret)` goes to the first accepted single-secret placement (bearer,
key header, query, or cookie) in definition order; on a Basic-only or no-auth
API it is refused as `BuildError::IncompatibleCredential`. There is
no missing-credential error, `.anonymous()` on a call sends without one, and
a credential never overrides a same-named request query/cookie/header value
(that send fails as `InvalidArgument`). A client's `Debug` prints no
credential.

### 3) Public Enums Are `#[non_exhaustive]`

Match statements must include a wildcard arm for forward compatibility.

Key enums: `AuthStrategy`, `ApiKeyLocation`, `UpdateStrategy`, `ApiRequest`, `FormFieldKind`, `ApiResponse`, `RestMethod`.

Exception: `CrossOrigin` is exhaustive on purpose, so the generator's translation of a security policy cannot fall through a wildcard arm.

### 4) Env Mapping Field Names

Use current `EnvMapping` names:

- `bearer_token`
- `basic_user`
- `basic_pass`
- `api_key`

Do not document or introduce legacy `basic_auth` object shapes.

### 5) Basic Auth Source of Truth

For `AuthStrategy::Basic`:

- Username: `env_username` (or `env_mapping.basic_user`)
- Password: `env_auth[0]` (or `env_mapping.basic_pass`)

### 6) API-Level Runtime Policy Fields

Every `RestApi` literal sets three fields (no `Default` for `RestApi`, no serde,
no per-endpoint override):

- `cross_origin: CrossOrigin` — redirects and followed next-page links.
  `same_origin()` (default), `allow_origins([..])` (exact origins, credentials
  sent), `allow_without_credentials()`, `danger_allow_any()`, `deny()` (same-origin
  next-page links still pass). `name()` gives the snake_case name. Callers may
  only tighten it; the runtime enforces it.
- `request_id_header: Option<String>` — provider request-id response header,
  case-insensitive; `None` unless the provider is known to send one.
- `secret_names: Vec<String>` — extra header names (case-insensitive) and query
  names (exact) never stored or shown, beyond built-in and auth names.

`definitions/tests/api_policy.rs` pins all three for every shipped API; a new
definition must add its row there.

## Generated-Client Contracts To Keep In Sync

When updating docs or examples that touch generated clients:

- `OpenAI` is the one client struct callers write,
  `pub struct OpenAI(schematic_runtime::Client<OpenAIApi>)`, with `BASE_URL`,
  `DOCS_URL`, `definition()`, an infallible `new() -> Self`, and
  `basic_auth(user, password)` only when the API accepts Basic. There is no
  `OpenAIClient` alias and no `builder()`
- `OpenAIApi` is the tag (`pub enum OpenAIApi {}`, implements
  `schematic_runtime::Api`) that request types name; callers never write it
- `schematic_runtime::api_client!(OpenAI, OpenAIApi);` gives the struct every
  client-wide setting, `build`, `send`, `send_raw`, and `check_link` as
  inherent methods (no trait import)
- Per REST endpoint the struct also gets an inherent endpoint method (the
  endpoint id in snake case, `ListModels` → `list_models`) taking the required
  values in `{Endpoint}Request::new` order and returning a `{Endpoint}Call`
  (the request name with `Request` → `Call`). Name clashes are renamed and
  reported; see [`schematic/gen.md`](./schematic/gen.md) and
  `schematic/docs/generation/validation.md`
- Each request struct implements `schematic_runtime::EndpointSpec` (`type Api`,
  `Success`, `Failure = NoDeclaredError`, `Decoder`; `ENDPOINT_ID`, `METHOD`,
  `into_parts()`); a client accepts only its own API's requests
- A call sends nothing until `.await`, which returns
  `Result<Success, CallError<Failure>>`; chain one setter per optional value,
  then `.anonymous()`/`.credential(..)`/`.deadline(&d)` as needed. `.full()`
  (chained last) yields the `Response` envelope (`Ok` whenever a response head
  arrived, `Err(TransportError)` otherwise); `.build()?` checks the call's
  arguments (encoding, headers, credential kind and placement) without
  sending. `CallError` is `#[non_exhaustive]`, so a `match` needs a
  `_ =>` arm
- `client.send(request)` (one argument) returns the same kind of call for a
  `{Endpoint}Request` object, for generic code; `send_raw(request,
  &CallOptions)` is the only API taking `CallOptions` and skips decoding
- Base URL, timeout, credentials (`api_key`, `basic_auth`, `credentials`),
  env order, and response hooks (`pre_response_json`,
  `mutate_response::<Request, _>` for JSON endpoints of the same API) are
  chained onto the client. They are validated by the optional `build()?`,
  which returns the same type, or on the first call, which reports an invalid
  setting as `TransportErrorKind::InvalidClient(BuildError)`

Canonical pattern:

```rust
use schematic_schema::prelude::*;

let client = OpenAI::new().api_key("token");

let _models = client.list_models().await?; // ListModelsResponse, or a CallError
```

## OpenAPI Extension Reality (Current Exporter)

Schematic defines three extension structs, but exporter emission is currently:

- emitted: document-level `x-schematic`
- emitted: operation-level `x-schematic`
- defined but not emitted: schema-level `x-schematic` (`SchematicSchemaExtension`)

Document-level extension carries API-wide config (`module_path`, `request_suffix`, `env_mapping`, `headers`).
Operation-level extension carries endpoint config (`request`, `response`, `headers`).

Current serialized response shape uses enum tagging, e.g.:

```yaml
x-schematic:
  response:
    Json:
      type_name: ListModelsResponse
      module_path: null
```

`Text`/`Binary`/`Empty` appear as scalar enum values.

## Schematic Definitions Facts That Frequently Drift

Keep these aligned when docs/examples mention built-in APIs:

- OpenAI: 265 endpoints, 1394 types, bearer auth; the imported definition has
  no env mapping, so the client reads **no** variable (pass a `Credential`). **Generated**
  from `schematic/specs/openai/openapi.yaml` — never hand-edit
  `definitions/src/openai/`; re-run `just -f schematic/justfile import-openai`.
  Excludes the deprecated Assistants family (`/assistants`, `/threads`).
- LM Studio: 6 endpoints, bearer auth via `LM_API_TOKEN`
- EMQX Basic: 36 endpoints, basic auth via `EMQX_API_KEY` + `EMQX_API_SECRET`
- EMQX Bearer: 38 endpoints, bearer auth via `EMQX_TOKEN`
- GitHub: 16 endpoints, bearer auth via `GITHUB_TOKEN` or `GH_TOKEN`
- Gitea: 16 endpoints, API key auth via `GITEA_TOKEN` (uses `token ` prefix, not Bearer)

## Update Workflow

1. Edit definition types/docs in `schematic/define`.
2. Verify behavior in generator/runtime contracts when the change affects response/auth/env mapping.
3. Update related docs in:
   - `schematic/README.md`
   - `schematic/define/README.md`
   - `schematic/definitions/README.md`
   - `schematic/docs/headers/README.md`
   - `schematic/docs/runtime/` (when credential, origin, or request-id behavior changes)
   - `schematic/docs/io/openapi-extensions.md`
   - `schematic/gen/README.md`
   - `schematic/schema/README.md`
4. Re-run focused tests before broad checks.

## Verification Commands

Focused response-codec generation coverage:

```bash
cargo nextest run -p schematic-gen --test l1 -E 'test(/^e2e_generation::(binary|text|empty|mixed)_response/)'
```

Definitions/auth sanity checks:

```bash
cargo nextest run -p schematic-definitions -E 'test(api_uses_bearer_token_auth) | test(basic_api_has_expected_endpoint_count) | test(bearer_api_has_more_endpoints)'
```

End-to-end regeneration safety pass (`just test` includes the fresh-output
guard, which compiles freshly generated crates and sends through the runtime
against wiremock):

```bash
just -f schematic/justfile test
just -f schematic/justfile generate
cargo check --manifest-path schematic/schema/Cargo.toml
```

The generator's tmux capture suite is L2 behind its internal
`terminal-tests` feature. Ordinary local L1 omits the target and harness;
`just test-l2` and CI enable them.

## Importing an API From a Published Spec

`schematic-gen import` turns an OpenAPI document into either a standalone client
(`--output`) or a definitions-crate module (`--definitions-out`). The second form
writes `mod.rs` (a `define_{api}_api()` plus `openapi_registry()`) and `types.rs`,
so the imported API flows through the normal `generate` pipeline.

- `--exclude-path <PREFIX>` drops endpoints by path prefix (repeatable).
- `--request-id-header <HEADER>` sets the imported `RestApi::request_id_header` (OpenAPI does not declare one); `just import-openai` passes `x-request-id`.
- Out-of-`i64` schema bounds are clamped with a warning rather than failing the
  parse (`import/normalize.rs`) — OpenAI's `seed` bounds need this.
- Bodies and responses with no named schema become the type path
  `serde_json::Value`. That is a path, not an identifier: render it with
  `syn` parsing, never `format_ident!`.

## Known Gaps

- `oneOf`/`anyOf` over *inline* (unnamed) schemas produce untagged enums whose
  variants are all `serde_json::Value` (e.g. `MessageStreamEvent`); only named
  members get real types.

See `schematic/reviews/2026-07-31-macro-review/spec.md` §10 for the full defect
ledger before touching schematic codegen.

## Remaining Testing Gap

Current tests validate generation logic, emitted `EndpointSpec` impls, compile-time structure,
and the fixture APIs' wire behavior through the runtime against wiremock.
They do not validate live provider HTTP integration behavior.

## Reference Map

Read these as needed:

- `schematic/define/README.md` (authoritative type-level behavior)
- `schematic/docs/headers/README.md` (credential precedence: REST runtime vs WebSocket `Headers`)
- `schematic/docs/io/openapi-extensions.md` (extension schema and examples)
- `schematic/gen/README.md` (generated client structs, tag types, and `EndpointSpec` contracts)
- `schematic/docs/generation/request-side.md` (query, header, cookie, body, and credential placement)
- `schematic/docs/runtime/README.md` (sending, response envelope, credentials, redirects, deadlines, hooks, redaction)
- `schematic/docs/runtime/calling-an-api.md` (endpoint methods, setters, per-call overrides, `.full()`, `.build()?`, `CallError`)
- `schematic/schema/README.md` (generated crate)
- `schematic/definitions/README.md` (built-in API catalog)

For legacy detailed notes that may still be useful, see:

- [`schematic/define.md`](./schematic/define.md) and the other pages in
  [`schematic/`](./schematic/SKILL.md)
