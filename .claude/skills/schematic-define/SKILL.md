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
- `Headers`, `EnvMapping`, `EnvList`, `SensitiveString`
- OpenAPI vendor extension models (`x-schematic`)

Code generation and runtime clients live in `schematic-gen` and `schematic-schema`, but this crate defines the contracts they consume.

## High-Signal Rules

### 1) Response Type Selection Is Runtime-Critical

Pick the right `ApiResponse`, because generated method shape depends on it:

| `ApiResponse` | Generated method | Return type |
|---|---|---|
| `Json(Schema)` | `request<T>()` | `T` |
| `Binary` | `request_bytes()` | `bytes::Bytes` |
| `Text` | `request_text()` | `String` |
| `Empty` | `request_empty()` | `()` |

If an endpoint returns bytes/text but is modeled as JSON, generated clients call the wrong response parser.

There is no streaming variant. SSE and NDJSON endpoints are modeled as `Binary`
and framed by the caller — see `definitions/src/ollama/mod.rs:316`.

### 1a) Request Bodies Reach the Client (all variants)

`Json`, `FormData`, and `UrlEncoded` all generate a real body. A form endpoint
gets a synthesized `{EndpointId}Form` struct — file parts typed
`shared::FormFile`, text parts `String`, optional fields `Option<T>` — reached
through the usual `pub body:` field.

`into_parts` returns `shared::RequestBody` (`Empty` / `Json` / `Multipart` /
`UrlEncoded` / `Raw`), **not** `Option<String>`. `Text` and `Binary` bodies are
still not carried on the request struct.

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

### 3) Public Enums Are `#[non_exhaustive]`

Match statements must include a wildcard arm for forward compatibility.

Key enums: `AuthStrategy`, `ApiKeyLocation`, `UpdateStrategy`, `ApiRequest`, `FormFieldKind`, `ApiResponse`, `RestMethod`.

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

## Generated-Client Contracts To Keep In Sync

When updating docs or examples that touch generated clients:

- `OpenAI::new()` returns `Self` (no `?`)
- Variant APIs are:
  - `variant()` builder
  - `variant_with(base_url, env_auth, strategy)`
  - `variant_with_headers(headers)`
- `Headers::use_bearer_token()` / `use_basic_auth()` bypass env credential lookup when auth is already set programmatically.

Canonical pattern:

```rust
use schematic_define::Headers;
use schematic_schema::openai::{ListModelsRequest, ListModelsResponse, OpenAI};

let client = OpenAI::new()
    .variant()
    .headers_builder(Headers::default().use_bearer_token("token"))
    .build();

let _models: ListModelsResponse = client
    .request(ListModelsRequest::default())
    .await?;
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

- OpenAI: 265 endpoints, 1394 types, bearer auth via `OPENAI_API_KEY`. **Generated**
  from `schematic/specs/openai/openapi.yaml` — never hand-edit
  `definitions/src/openai/`; re-run `just -f schematic/justfile import-openai`.
  Excludes the deprecated Assistants family (`/assistants`, `/threads`).
- LM Studio: 6 endpoints, bearer auth via `LM_API_TOKEN`
- EMQX Basic: 36 endpoints, basic auth via `EMQX_API_KEY` + `EMQX_API_SECRET`
- EMQX Bearer: 38 endpoints, bearer auth via `EMQX_TOKEN`
- GitHub: 14 endpoints, bearer auth via `GITHUB_TOKEN` or `GH_TOKEN`
- Gitea: 14 endpoints, API key auth via `GITEA_TOKEN` (uses `token ` prefix, not Bearer)

## Update Workflow

1. Edit definition types/docs in `schematic/define`.
2. Verify behavior in generator/runtime contracts when the change affects response/auth/env mapping.
3. Update related docs in:
   - `schematic/README.md`
   - `schematic/define/README.md`
   - `schematic/definitions/README.md`
   - `schematic/docs/headers/README.md`
   - `schematic/docs/io/openapi-extensions.md`
   - `schematic/gen/README.md`
   - `schematic/schema/README.md`
4. Re-run focused tests before broad checks.

## Verification Commands

Focused response-method generation coverage:

```bash
cargo test -p schematic-gen --test e2e_generation binary_response_generates_request_bytes_method
cargo test -p schematic-gen --test e2e_generation text_response_generates_request_text_method
cargo test -p schematic-gen --test e2e_generation empty_response_generates_request_empty_method
```

Definitions/auth sanity checks:

```bash
cargo test -p schematic-definitions api_uses_bearer_token_auth
cargo test -p schematic-definitions basic_api_has_expected_endpoint_count
cargo test -p schematic-definitions bearer_api_has_more_endpoints
```

End-to-end regeneration safety pass:

```bash
cargo test -p schematic-gen
just -f schematic/justfile generate
cargo check -p schematic-schema
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
- Out-of-`i64` schema bounds are clamped with a warning rather than failing the
  parse (`import/normalize.rs`) — OpenAI's `seed` bounds need this.
- Bodies and responses with no named schema become the type path
  `serde_json::Value`. That is a path, not an identifier: render it with
  `syn` parsing, never `format_ident!`.

## Known Gaps

- `Endpoint.params` is still dropped by the OpenAPI exporter (`export/paths.rs`),
  so exported specs carry no query parameters.
- `oneOf`/`anyOf` over *inline* (unnamed) schemas produce untagged enums whose
  variants are all `serde_json::Value` (e.g. `MessageStreamEvent`); only named
  members get real types.

See `schematic/reviews/2026-07-31-macro-review/spec.md` §10 for the full defect
ledger before touching schematic codegen.

## Remaining Testing Gap

Current tests validate generation logic, emitted methods, and compile-time structure.
They do not validate live provider HTTP integration behavior.

## Reference Map

Read these as needed:

- `schematic/define/README.md` (authoritative type-level behavior)
- `schematic/docs/headers/README.md` (programmatic auth patterns)
- `schematic/docs/io/openapi-extensions.md` (extension schema and examples)
- `schematic/gen/README.md` (generated method/contracts)
- `schematic/schema/README.md` (runtime client usage examples)
- `schematic/definitions/README.md` (built-in API catalog)

For legacy detailed notes that may still be useful, see:

- `.claude/skills/schematic/define.md`
