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
| `Json(TypeRef)` | `schematic_runtime::Json<T>` | `T` |
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

Use API builder policy methods (no `Default` for `RestApi`; endpoint
policies may replace the API policy). Stored policy fields are:

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

### 7) Pagination Defaults and Explicit Endpoint Collections

Use API `.default_pagination(strategy)` and endpoint `.pagination(Collection)`.
There are no API exception tables or parameter-level legacy pagination fields.
Types live in `schematic_define::pagination` and the prelude.
The canonical behavior page is `schematic/docs/pagination.md`.

- `pagination::resolve(api, endpoint)` uses the endpoint collection first;
  otherwise a GET root typed `Vec<T>` response inherits the API default.
  Wrappers, opt-outs, overrides, and non-GET lists need an explicit collection.
  Use `.pagination(Collection::at("data").item_type("Item"))` or
  `.pagination(Collection::root().unpaginated())`. `at("")` is invalid.
- `caller_knobs` is exactly what the generated call exposes; alternates
  (`or_paginate`) are modeled and validated but add no knob.
- `param_adoptions`/`adopted_params`: a strategy input with the same location
  and name as a declared param adopts it (removed from the request struct)
  only when the strategy authors the matching knob (a cursor part needs
  `.resumable()`); otherwise `adoption-without-knob`. A required adopted size
  needs `PageSize::sent_default(n)`; `server_default` is documentation only.
  `ManagedRole::sent_default` is the value sent when the caller sets none,
  and the generator writes it into `into_parts` for a required adopted
  param, so an ordinary send still carries it; the pager replaces it.
- `validate_pagination` feeds `schematic-gen`'s `validate_rust_client` as the
  `pagination-strategy` / `pagination-exception` / `pagination-parameter`
  features; rules are listed in `schematic/docs/generation/validation.md`.
- Definitions keep one API-local strategy helper (`github_page_number()`,
  `bitbucket_next_link()`, ...) and attach collections to endpoint builders.
  Authored: GitHub, GitLab, Gitea, Bitbucket, OpenAI, and ElevenLabs
  `GetHistory`/`ListSharedVoices` (93 pagers). Unverified providers (Anthropic,
  EMQX, Hugging Face, GitLab participants, Gitea trees) stay unpaginated: a
  GET `Vec<T>` under a default needs a `Collection::root().unpaginated()` assignment,
  and a wrapper gets an `.unpaginated()` collection so it reads as a list.
- `schematic/definitions/tests/pagination_coverage/` is exhaustive: every
  endpoint resolving as a list needs a row (origin, items, item type,
  strategy description, exact knobs, adopted fields), so authoring or
  re-importing changes fail there until the row is updated.
- A paginated endpoint generates `impl schematic_runtime::pager::Paginated`
  (`&'static` strategy constants) plus one capability marker per knob, and
  its call type forwards `pager()`, each authored knob (`page`, `offset`,
  `after`, `before`, `page_size`, `keep_alive`), and `pages()` / `fetch_page()` /
  `collect_all()`, or `with_session(|scope| Box::pin(async move { .. }))` for
  a strategy with a `Session` (pages, `partitioned`, and `renew` only inside
  the scope, which closes with the latest handle under its own grace,
  `close_grace(..)`, default 5 s). A
  partition without a session forwards `partitioned(n)`. Knobs return the
  runtime `Pager`, so endpoint setters come first. A remaining setter named
  like a forwarded knob becomes `<name>_param`. Traversal and session logic
  live only in `schematic-runtime`'s `pager` module.
- A session with partitions must use `Session::opened_by(..)` (a separate
  open request). `Session::opened_by_first_page()` is valid only without
  partitions; validation reports `partitioned-first-page-session` on defaults,
  overrides, and alternates. Shared partition sessions retain the latest
  observed handle, including rotating handles, and close once.
- The committed generated crate carries its own pager evidence, run only by
  `just test-schema` (CI does not build `schematic-schema`):
  `schematic/schema/tests/pagination.rs` pages GitHub `ListReleases`, OpenAI
  `ListBatches`, and Bitbucket `ListTags` against wiremock, and
  `compile_fail.rs` runs trybuild cases on real endpoints (`trybuild` is a
  generated dev-dependency in `gen/src/cargo_gen.rs`). Some `.stderr` files
  list rustc's first eight implementors of a capability trait, so adding a
  paginated endpoint can change them: refresh with `TRYBUILD=overwrite cargo
  nextest run --manifest-path schematic/schema/Cargo.toml --test
  compile_fail` and review the diff.

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

Serialized response shape uses enum tagging. A JSON response holds the stored
`TypeRef` (component names plus wrappers), never a Rust path:

```yaml
x-schematic:
  response:
    Json:
      Named: ListModelsResponse   # or Array: { Named: Model }, Primitive: Json
```

`Text`/`Binary`/`Empty` appear as scalar enum values.

## Schematic Definitions Facts That Frequently Drift

Keep these aligned when docs/examples mention built-in APIs:

- OpenAI: 258 endpoints, 1394 types, bearer auth; the imported definition has
  no env mapping, so the client reads **no** variable (pass a `Credential`). **Generated**
  from `schematic/specs/openai/openapi.yaml` — never hand-edit
  `definitions/src/openai/`; re-run `just -f schematic/justfile import-openai`.
  Excludes the deprecated Assistants family (`/assistants`, `/threads`).
- LM Studio: 6 endpoints, bearer auth via `LM_API_TOKEN`
- EMQX Basic: 36 endpoints, basic auth via `EMQX_API_KEY` + `EMQX_API_SECRET`
- EMQX Bearer: 38 endpoints, bearer auth via `EMQX_TOKEN`
- GitHub: 16 endpoints, bearer auth via `GITHUB_TOKEN` or `GH_TOKEN`
- Gitea: 15 endpoints, API key auth via `GITEA_TOKEN` (uses `token ` prefix, not Bearer)

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
writes `mod.rs` (a `define_{api}_api()` plus the fallible `module_group()` it unwraps) and `types.rs`,
so the imported API flows through the normal `generate` pipeline.

- `--exclude-path <PREFIX>` drops endpoints by path prefix (repeatable).
- `--request-id-header <HEADER>` sets the imported `RestApi::request_id_header` (OpenAPI does not declare one); `just import-openai` passes `x-request-id`.
- `--overlay <RUST_PATH>` (requires `--definitions-out`) makes the factory end
  fallible typed builder assembly ending in `Ok(<path>(api))`, so hand-written additions survive a
  re-import. OpenAI's pagination lives in `definitions/src/openai_pagination.rs`
  (`--overlay crate::openai_pagination::apply`); the importer never writes it.
  `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`
  fails when the committed module and a fresh import differ.
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

- `schematic/docs/definition-builders.md` (current definition behavior)
- `schematic/docs/headers/README.md` (credential precedence: REST runtime vs WebSocket `Headers`)
- `schematic/docs/io/openapi-extensions.md` (extension schema and examples)
- `schematic/docs/generation/generated-clients.md` (generated client structs, tag types, and `EndpointSpec` contracts)
- `schematic/docs/generation/request-side.md` (query, header, cookie, body, and credential placement)
- `schematic/docs/runtime/README.md` (sending, response envelope, credentials, redirects, deadlines, hooks, redaction)
- `schematic/docs/runtime/calling-an-api.md` (endpoint methods, setters, per-call overrides, `.full()`, `.build()?`, `CallError`)
- `schematic/schema/README.md` (generated crate)
- `schematic/definitions/README.md` (built-in API catalog)

For the focused definition and generation reference notes, see:

- [`schematic/define.md`](./schematic/define.md) and the other pages in
  [`schematic/`](./schematic/SKILL.md)

## Definition snapshots

REST definitions and parameter types now derive plain Serde, including policy and pagination evidence types. Optional fields define omission/null as None; required collections reject missing/null input. A `RestApi` carries its schema components (`schemas`); this is data transfer, so call `validate_references` on a deserialized API before use. `just -f schematic/justfile migration-check` compares historical JSON values and regenerates all artifacts into temporary directories without mutating fixtures. Baselines use biscuit-hash BLAKE3 on canonical JSON above the 5 MB raw threshold; see `schematic/docs/definition-snapshots.md`.

## Typed registration

`schematic-define` now has additive `DefinitionContext`, `TypeDesc`, `Registration`,
`definition_type!`, and typed `rust_path!` APIs. Declare nominal implementations
beside model declarations; no blanket JsonSchema dispatcher or type_name parsing.
Register exceptional aliases first: first canonical registration fixes name/path/order.
Containers recurse, primitives/Value stay inline, and explicit unused components survive.
`TypeDesc` is an authoring handle (`type_ref()`, `rust_path()`), with no serde.
Endpoints store its `TypeRef`: component names plus wrappers (`Named`, `Array`,
`Optional`, `Map`, `KeyedMap { key, value }` for non-`String` Rust keys,
`Primitive(..)` incl. `I8`..`I128`/`U8`..`U128`/`Isize`/`Usize`/`F32`/`Char`/`Unit`,
and `Primitive(Json)` for free-form JSON). `RestApi::schemas` holds
`SchemaComponent { name, rust_path: Option<String>, schema }` for the whole
context, nested `$defs` flattened (nested-only entries have `rust_path: None` and
cannot be referenced). A name reached again with a different wire schema (nested
or explicit) fails `components()` with `ComponentConflict` inside `Collisions`. `RestApi::{component, rust_type, validate_references}` and
`Endpoint::json_references` are the readers; `validate_references` rejects
dangling names (`DefinitionError::UnresolvedReference { api, endpoint, name }`),
duplicate/empty/malformed components, and empty map keys, for built and
deserialized APIs alike. `schematic_definitions::registry::assemble_module_group`
returns `Result<Vec<RestApi>, DefinitionError>` and gives every member the
finished context's components. `registry_key_for` is only the per-API artifact
key. Explicitly register models in the intended order before building endpoints.
Read `schematic/docs/typed-registration.md`. Ollama progress aliases use explicit
typed alias registration. `just doctest` runs registration compile-fail
examples; the existing fresh-output misuse guard also checks their diagnostic codes.

## Required-fact REST builders

Use `Endpoint::{get,post,put,patch,delete,head,options}` or `new(id, method, path)`,
then `describes` and one success declaration before fallible `build`. JSON methods
receive `&mut DefinitionContext`; build endpoints before borrowing that context
with `RestApi::builder(&ctx, name, base_url)`. API description and one explicit
no_auth/bearer_env/basic_env/authentication choice are required. Repeated success
or auth choices fail; explicit env mappings must agree independent of setter order.
`Success` and `ErrorSet` are non-exhaustive placeholders; only `ErrorSet::none()`
is available. JSON bodies lower to a `TypeRef`; API build stores the context's
components and runs `validate_references`. OpenAPI export writes `Named` as a
`$ref` and takes `components.schemas` from `api.schemas`. Root typed Vec responses
retain GET list inference, while Option/nominal wrappers do not. All shipped REST
providers and the importer use builders; Endpoint/RestApi are non-exhaustive, so
external literals cannot compile. `Schema { type_name, module_path }` is used
only by WebSocket messages. Endpoint oauth_scopes is removed; auth_policy is retained. Read
`schematic/docs/definition-builders.md` for accepted forms and the Default allow-list.

## Imported builder assemblies

`import --definitions-out` emits fallible Endpoint/RestApi builders and a public
`module_group() -> Result<Vec<RestApi>, DefinitionError>`. Its API and schema
components come from one ordered DefinitionContext; catalog factories unwrap that
assembly. The library `OpenApiImport::build()` result already carries one
component per catalog model (source order, source schema, Rust path
`{module_path}::{Name}` or bare) and passes `validate_references`; a dangling
body/response `$ref` fails as `OpenApiError::Definition(UnresolvedReference)`.
A standalone (`--output`) import keeps those components with bare Rust paths.
Assembly's component order and schemas differ (nested definitions flattened
after their parent; schemars output), names and references do not.
Nominal generated models implement Registration; aliases reuse their underlying
implementation, so containers/primitives/Value create no components. The emitter
bounds debug stack use with small endpoint helpers and batches of registrations.
OpenAI is regenerated through this route, never manually rewritten.

The shared importer normalizes query-bearing paths and schema-gates twin merges.
The seven OpenAI beta operations now share their ordinary IDs with optional
`beta` query inputs; beta model types supply the compatible superset. GET bodies
and malformed or conflicting expressions fail with context. JSON/YAML reject
null/malformed declarations, duplicate keys and trailing invalid input. Empty
foundation metadata imports remain possible; run_import rejects no usable endpoints.

`EndpointBuilder::pagination(Collection)` authors the explicit endpoint collection.
OpenAI's untouched-by-import overlay assigns collections and rejects unknown IDs.
`ResolutionOrigin::Exception` remains the serialized origin for explicit collections,
without an API exceptions table. All 93 authored pagers preserve their controls.

## Query-free route identity

Shared validation, endpoint/API build, and import boundaries reject a remaining
`?` in a definition path. Diagnostics name API, endpoint, and path. The importer
normalizes supported source queries before this boundary. Gitea has one
`GetGitTree` endpoint with optional Boolean `recursive`; there is no recursive
request type. GitLab repository-file `ref` is a query input whose Rust name is
`git_ref`. Query inputs never create another endpoint for the same method/path.

## Authoring guidance maintenance

Use `schematic/docs/authoring-examples.md` for the compiling nominal/alias/container
and API example (included directly in schematic-define rustdoc). The generator
README is an overview; its detailed reference lives in
`schematic/docs/generation/generated-clients.md`. The create-schematic-api command
teaches the same fallible context assembly. Keep these entry points aligned.
The L1 `authoring_guidance` corpus rejects obsolete REST constructor examples.
Only four Success forms and ErrorSet::none exist; response/vocabulary extensions
are planned. Reserve path_param/public/auth/deprecated/rate_limit and success
stream/content-type names. No selector API is planned here.
