---
name: schematic-define
description: Expert knowledge for defining REST and WebSocket APIs with `schematic-define`. Use when creating or editing `RestApi`/`Endpoint` models, auth and env mappings, request bodies, success shapes (JSON, text, binary, empty, SSE/NDJSON/byte streams, multi-shape replies), declared error tables, GraphQL operations and Relay pagination, REST pagination, and OpenAPI extension behavior that drives generated clients.
---

# schematic-define

Use this skill when working in `schematic/define` or when updating docs/tests
that describe `schematic-define` behavior. Code generation lives in
`schematic-gen`, request execution in the hand-written `schematic-runtime`
(which does not depend on this crate), and generated clients in
`schematic-schema`; this crate defines the contracts they consume.

## Detail pages

| Page | Covers |
|---|---|
| [`schematic/define.md`](./schematic/define.md) | Required-fact builders, typed registration, auth/env, policy fields, optional vocabulary, query fidelity, bundles and snapshots |
| [`schematic/responses.md`](./schematic/responses.md) | Success shapes, streams, multi-shape enums, declared errors, GraphQL operations, `Relay` |
| [`schematic/pagination.md`](./schematic/pagination.md) | REST strategies, collections, adoption, generated pager surface |
| [`schematic/gen.md`](./schematic/gen.md) | Generator CLI, output layout, generated code, name-collision renames, fresh-output guard |
| [`schematic/openapi.md`](./schematic/openapi.md) | OpenAPI import/export, `x-schematic` writer and reader (what round-trips, strict rule), imported builder assemblies |
| [`schematic/definitions.md`](./schematic/definitions.md) | Shipped APIs and facts that drift |
| [`schematic/headers.md`](./schematic/headers.md) | `Headers` (standalone builder; no generated client uses it), `SensitiveString`, `EnvList`, `EnvMapping` |
| [`schematic/websocket.md`](./schematic/websocket.md) | `WebSocketApi` roles, client-relative directions, selectors/fallback, `responds_with` typed requests, handshake params and credential, hints, JSON form |

## High-Signal Rules

### 1) Success shapes decide the generated type

`Endpoint::response: Vec<SuccessShape>` (read with `success_shapes()`) is
authored with `.returns(Success::…)` (repeatable) or `returns_json::<T>(ctx)` /
`returns_text()` / `returns_binary()` / `returns_empty()`:

| Shapes | `type Success` | `type Decoder` |
|---|---|---|
| one `Json(T)` / `Text` / `Binary` / `Empty` | `T` / `String` / `Bytes` / `()` | `Json<T>` / `Text` / `Binary` / `Empty` |
| one `Success::sse(item)` / `ndjson(item)` | `ItemStream<T>` | `Sse<T, End>` / `Ndjson<T, End>` |
| one `Success::bytes()` | `ByteStream` | `ByteStream` |
| several | `{Id}Response` enum | `Shapes<{Id}Response>` |

Bytes or text modeled as JSON gives the wrong success type and a runtime
`MalformedSuccess`. Model streaming endpoints as stream shapes (with
`ends_at_data_line`/`ends_at_event`/`ends_when_field`/`keep_end`), never as
`Binary` framed by the caller. Several shapes must be selectable by status,
media type, and JSON tag (`validate_success_shapes`); an unmatched multi-shape
reply is `ErrorKind::UnexpectedContentType`. A registered type named
`{Id}Response` or `{Id}Error` collides with the generated enum.

### 2) Declared errors are a table

`ErrorSet::new().on(StatusMatch::exact(404) | range(lo, hi) | class(4),
ErrorShape::json(desc) | text() | binary() | empty())`; endpoint and operation
`.errors(..)` overlay API `.default_errors(..)`; `.errors(ErrorSet::none())`
opts out of all. `RestApi::effective_errors(&ep)` resolves the `ErrorTable`
(exact, then narrowest range). Generated `Failure` is `NoDeclaredError`, the
one shape's type, or an `{Id}Error` enum, routed by `EndpointSpec::ERRORS`;
a matched reply is `ErrorKind::Declared(E)`.

### 3) GraphQL operations ride on a REST API

`GraphQlOperation::query|mutation(id, document)` (typestate: `describes`,
`variables::<V>`/`no_variables`, `data::<D>`) plus
`RestApiBuilder::graphql(AddressRule::github()).operations([..])`. Generated
requests use `GraphQl<D>` and `GraphQlOperationError<D, H>` and are sent with
`client.send(Request::new(vars))` (no endpoint method). Paging uses
`schematic_define::pagination::Relay::{forward, backward}` (one direction per
pager; pager-managed variables must be optional) and generates
`Paginated::page_root = graphql::page_data`. Runtime `ABI` is 8. Detail:
[`responses.md`](./schematic/responses.md#graphql-operations).

### 4) Request bodies reach the client (all variants)

`Json`, `FormData`, and `UrlEncoded` generate a real body; a form endpoint
gets a `{EndpointId}Form` struct (`shared::FormFile` file parts). `Text` and
`Binary` bodies are `body: String` / `body: Vec<u8>` fields sent as
`RequestBody::Raw`. `EndpointSpec::into_parts()` returns `RequestParts`
(`method`, `path`, `query` as unencoded `(String, QueryValue)` pairs the
runtime encodes once, `headers`, `body`) or `InvalidArgument`. Header and
cookie params become `with_*` fields; cookies fold into one `Cookie` header.

### 5) Auth, env mapping, and policy

`AuthStrategy`: `BearerToken`, `ApiKey`, `ApiKeyParam { location: Query |
Cookie, name }`, `Basic { username_env, password_env }`, `OAuth2(OAuth2Config)`
(one variant per grant), `None`. `EnvMapping` fields are `bearer_token`,
`basic_user`, `basic_pass`, `api_key`. The runtime resolves one credential per
send: call → client → env → anonymous (no missing-credential error).
API policy fields `cross_origin`, `request_id_header`, and `secret_names` are
pinned per shipped API by `definitions/tests/api_policy.rs`. Detail:
[`define.md`](./schematic/define.md#auth-and-environment).

### 6) Public enums are `#[non_exhaustive]`

Match with a wildcard arm: `AuthStrategy`, `ApiKeyLocation`, `UpdateStrategy`,
`ApiRequest`, `FormFieldKind`, `ResponseShape`, `StreamFraming`, `StreamEnd`,
`JsonTag`, `AddressRule`, `OperationKind`, `GraphQlVariables`, `RestMethod`;
`Endpoint`, `RestApi`, `SuccessShape`, and `GraphQlOperation` are
non-exhaustive structs built only through builders. Exception: `CrossOrigin` is
exhaustive so the generator's policy translation cannot fall through.

### 7) Pagination

API `.default_pagination(strategy)` plus endpoint `.pagination(Collection)`;
inference applies only to a GET whose sole shape is JSON `Vec<T>`. Every list
needs a row in `definitions/tests/pagination_coverage/`. Detail:
[`pagination.md`](./schematic/pagination.md).

### 8) Value constraints have one source each

`schematic_define::constraints::Constraints` (fluent: `min_length`,
`max_length`, `pattern`, `unenforced_pattern`, `format`, `minimum`, `maximum`,
`exclusive_minimum`, `exclusive_maximum`, `multiple_of`, `min_items`,
`max_items`, `unique_items`, `allowed_values`, `constant`, `items`) attaches via
`ParamDef::constraints`, `FormField::constraints` (text/JSON fields), and
`EndpointBuilder::request_constraints` (text body or a non-component JSON
root). Registered models carry rules as Schemars attributes only; a rule on a
component body, file field, binary/form body, or WebSocket handshake param is
an error. Numeric limits are decimal text (`"18446744073709551616"`, `"0.1"`;
serialized as JSON strings). Endpoint/API build, bundle load, and the
generator's `type-constraint` check all run `constraint_problems`
(`DefinitionError::InvalidConstraint { api, problem: Box<ConstraintProblem> }`).
`admit_pattern` decides the enforced ECMAScript subset (table and test:
`schematic/docs/value-constraints.md`); `format` is annotation-only. A pager-
adopted page-size param takes `minimum`/`maximum` from the strategy
(`page_size_constraints`, `adopted_page_size`); its own differing bounds or
other rules are `page-size-constraint-conflict`. `unenforceable_rules` finds
model rules beneath unsupported composition or unresolved refs. Generated
requests do not enforce constraints yet.

## Generated-Client Contracts To Keep In Sync

- `OpenAI` is the one client struct callers write,
  `pub struct OpenAI(schematic_runtime::Client<OpenAIApi>)`, with `BASE_URL`,
  `DOCS_URL`, `definition()`, infallible `new()`, and `basic_auth(..)` only
  for Basic APIs; `OpenAIApi` is the tag request types name.
  `schematic_runtime::api_client!(OpenAI, OpenAIApi)` gives it every client
  setting, `build`, `send`, `send_raw`, and `check_link` as inherent methods.
- Per REST endpoint: an inherent snake-case method (`ListModels` →
  `list_models`) taking required values in `{Endpoint}Request::new` order and
  returning `{Endpoint}Call`; clashes are renamed and reported (see
  [`gen.md`](./schematic/gen.md)).
- Each request implements `EndpointSpec` (`type Api`, `Success`, `Failure`,
  `Decoder`; `ENDPOINT_ID`, `METHOD`, `ERRORS`, `ADDRESS`, `into_parts()`); a
  client accepts only its own API's requests.
- A call sends on `.await` → `Result<Success, CallError<Failure>>`
  (`CallError` is `#[non_exhaustive]`); `.full()` (last) yields the `Response`
  envelope (`Ok` iff a head arrived); `.build()?` checks arguments without
  sending; `send_raw(request, &CallOptions)` skips decoding.
- Client settings (base URL, timeout, credentials, env order,
  `max_frame_bytes`, `graphql_url`, hooks: `pre_response_json` and
  `mutate_response::<Request, _>` for JSON endpoints) are validated by
  `build()?` or on the first call (`TransportErrorKind::InvalidClient`).

```rust
use schematic_schema::prelude::*;

let client = OpenAI::new().api_key("token");
let _models = client.list_models().await?; // ListModelsResponse, or a CallError
```

## Update Workflow

1. Edit definition types/docs in `schematic/define`; new defaulted members
   join `OMITTED_DEFAULTS` in `define/src/bundle.rs`.
2. Verify generator/runtime contracts when the change affects responses,
   errors, auth, env mapping, or pagination.
3. Update `schematic/docs/` topic pages (canonical), the READMEs (overview
   only), `schematic/docs/io/openapi-extensions.md`, and this skill.
   `define/README.md`'s Rust examples are doctests (`ReadmeDoc` in
   `define/src/lib.rs`), so a builder rename fails `just doctest` until the
   README follows.
4. After a generator change that alters signatures, build dependents with
   `--all-features --all-targets` (sniff's remote providers are feature-gated)
   and run `just test-schema` (no other gate compiles
   `schematic/schema/tests/`).

## Verification Commands

```bash
cargo nextest run -p schematic-gen --test l1 -E 'test(/^e2e_generation::(binary|text|empty|mixed)_response/)'
cargo nextest run -p schematic-gen --test l1 -E 'test(/^response_adapters::/)'
just -f schematic/justfile test        # includes the fresh-output guard (compiles and sends against wiremock)
just -f schematic/justfile generate    # or generate-one <key>; generate-{openapi,postman,asyncapi} for one export
just -f schematic/justfile doctest     # includes doctest-schema (generated crate)
just -f schematic/justfile test-schema
just -f schematic/justfile lint-schema
just -f schematic/justfile migration-check
```

The generator's tmux capture suite is L2 behind its internal `terminal-tests`
feature; `just test-l2` and CI enable it.

## Known Gaps

- `oneOf`/`anyOf` over inline schemas produce untagged enums whose variants
  are all `serde_json::Value` (e.g. `MessageStreamEvent`).
- An imported union decodes by its `discriminator` only when every member
  resolves a tag value (`mapping`, the property's `enum`, or the component
  name); otherwise, and for every union without a `discriminator` keyword
  (e.g. `TextResponseFormatConfiguration`), it is first-match untagged. A
  defaulted discriminated enum has an empty tag and does not decode again.
- OpenAPI skips GraphQL operations (`graphql-operation` warning); Postman
  exports them as runnable POSTs. OpenAPI cannot express stream end conditions.
- Tests cover generation, compile-time structure, and wire behavior against
  wiremock, not live providers. See
  `schematic/reviews/2026-07-31-macro-review/spec.md` §10 before touching
  schematic codegen.

## Reference Map

- `schematic/docs/definition-builders.md`, `typed-registration.md`,
  `pagination.md`, `responses-and-errors.md`, `graphql.md`
- `schematic/docs/generation/` (`generated-clients.md`, `request-side.md`,
  `validation.md`, `partial-generation.md`)
- `schematic/docs/runtime/` (`calling-an-api.md`, `codecs.md`,
  `sending-and-responses.md`, `credentials.md`, `deadlines-and-timeouts.md`)
- `schematic/docs/io/` (`openapi-extensions.md`, `import-openapi.md`,
  `definition-bundles.md`), `schematic/docs/headers/README.md`
