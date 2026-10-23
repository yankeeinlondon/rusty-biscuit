---
name: schematic
description: Expert knowledge for Schematic REST and WebSocket API client code generation. Use when defining APIs, generating typed Rust clients, importing OpenAPI specs, adding endpoints, configuring authentication, building headers programmatically, or troubleshooting code generation issues.
---

# Schematic

Type-safe REST and WebSocket API client code generation for Rust. Define APIs declaratively, generate strongly-typed clients automatically.

## Quick Reference

| Package | Purpose |
|---------|---------|
| `schematic-define` | Primitives: `RestApi`, `Endpoint`, `AuthStrategy`, `CrossOrigin`, `Headers` (WebSocket clients), `ApiRequest`, `SuccessShape`/`Success`, `ErrorSet`/`StatusMatch`, `GraphQlOperation`/`AddressRule`, `pagination::Relay` |
| `schematic-definitions` | Pre-built APIs: Anthropic, OpenAI, ElevenLabs, HuggingFace, LM Studio, Ollama, EMQX, GitHub, GitLab, Gitea, Bitbucket, Eversolo, Samsung Smart TV, Unfolded Circle, Artificial Analysis |
| `schematic-gen` | Code generator CLI with `generate`, `validate`, and `import` commands |
| `schematic-runtime` | Hand-written runtime every generated REST client wraps a `Client<Tag>` of: infallible `Client::new` with chained settings validated by `build()?` or on the first call (`TransportErrorKind::InvalidClient`), `api_client!` forwarding them onto each API struct, `SharedTransport`, tighten-only `OriginPolicy`, awaitable `Call` (`.await` → `Result<T, CallError<E>>`, `.full()` → `Response` envelope that is `Ok` iff a response head arrived, else `TransportError`), `send_raw(request, &CallOptions)` → `RawResponse`, credentials (per call → client → env → anonymous), redirects hop by hop (`MAX_REDIRECTS`, `check_link`), shared `Deadline`, codecs `Json`/`Text`/`Binary`/`Empty`, streams `Sse`/`Ndjson`/`ByteStream` (`ItemStream`, `StreamError`), multi-shape `Shapes`, `GraphQl<D>`, the `ERRORS` table, the `pager`, JSON hooks, redacted metadata, and `validate` (request-side constraint checks over `&'static` `Plan`s: exact `Decimal` numbers, lazily compiled `Pattern`s, value-free `Violations` → `InvalidArgument`; generated `validate()` and `into_parts` call it). Never retries; call futures are boxed. Docs: `schematic/docs/runtime/` |
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

## REST authoring

Use the parent `schematic-define/SKILL.md` for detailed runtime and generation
contracts, and [`define.md`](./define.md), [`responses.md`](./responses.md),
and [`pagination.md`](./pagination.md) for authoring detail. Current behavior lives in `schematic/docs/definition-builders.md`,
`schematic/docs/typed-registration.md`, and `schematic/docs/pagination.md`.

Every shipped REST provider uses a fallible `module_group()` assembly with one
ordered `DefinitionContext` for its APIs; every API carries that context's
components in `RestApi::schemas`. Explicitly register types
in component order, build endpoints, then build APIs against the same context.
`Endpoint` and `RestApi` are non-exhaustive; external literals do not compile.

```rust
use schematic_define::prelude::*;

let mut context = DefinitionContext::types_in("::my_models");
let endpoint = Endpoint::get("ListItems", "/items")
    .describes("List items")
    .returns_json::<Vec<String>>(&mut context)
    .build()?;
let api = RestApi::builder(&context, "Items", "https://example.com")
    .describes("Item service")
    .no_auth()
    .endpoints([endpoint])
    .build()?;
```

- Description and one success shape are required for endpoints; description
  and one explicit auth choice are required for APIs. Builds validate values,
  auth/env coherence, that every schema reference resolves
  (`RestApi::validate_references`), and that every `$ref` inside a component
  names a component (`RestApi::validate_schema_closure`). GET bodies fail.
- Use typed JSON methods, `Success`, request body builders, and `ParamDef`
  locations. Do not author response/request/Schema constructors or string types.
- `definition_type!` supplies nominal registration. Aliases/re-exports use
  `register_as::<T>(rust_path!(Alias))` before the first canonical registration;
  add `schema_name` only when the component name differs. Containers recurse
  into leaves; primitive/Value responses remain inline, with no fake component.
- REST JSON bodies store a `TypeRef` (component names plus wrappers:
  `Named`, `Array`, `Optional`, `Map`, `KeyedMap`, `Primitive`), never a Rust
  path. Names resolve through `RestApi::schemas`
  (`SchemaComponent { name, rust_path, schema }`); `api.rust_type(&reference)`
  spells the Rust type. A dangling name fails API build with
  `DefinitionError::UnresolvedReference` and generation with
  `invalid-schema-reference`. `Schema { type_name, module_path }` is WebSocket-only.
- Paths cannot contain `?`: endpoint/API build, shared validators, and import
  reject it with API, endpoint, and path context. Query/header/cookie/body
  locations never distinguish two endpoints with the same method/path.
- Gitea has 15 endpoints: `GetGitTree` takes optional Boolean `recursive`.
  GitLab's wire `ref` is a query parameter with Rust spelling `git_ref`.
- Authored pagination is API `.default_pagination(strategy)` plus endpoint
  `.pagination(Collection)`; keep strategies provider-local. No API exception
  table, legacy pagination enums/helpers, or parameter pagination fields remain.
  Explicit endpoint collections keep serialized origin `Exception`. There are
  93 authored pagers; adoption must preserve the exposed controls.
- Endpoint OAuth scopes are removed; explicit API auth_policy remains effective.
- Basic carries `username_env`/`password_env` inside `AuthStrategy::Basic`;
  OAuth2 is one `OAuth2Config` variant per grant.
- WebSocket factories and stored forms retain their established construction API.

## Import and generated artifacts

OpenAI is imported from `schematic/specs/openai/openapi.yaml`; never edit its
emitted definition module or types manually. Use `just import-openai`. It has
258 endpoints after seven beta-route merges, with optional Boolean `beta` inputs.
The handwritten `openai_pagination` overlay assigns collections and rejects
unknown IDs. Imports emit typed module-group assemblies and nominal model
registrations in source order. Supported source query expressions normalize
before building; malformed, conflicting, or GET-body inputs fail.

Generated Rust, OpenAPI, and Postman artifacts are never hand-edited. Use the
area's generate recipes and review complete file sets. Preserve each
endpoint's declared success shapes and error table; streaming SSE/NDJSON
endpoints declare stream shapes and generate `Sse`/`Ndjson` codecs yielding
`ItemStream<T>`, never `Binary` with caller framing. See
`schematic/docs/generation/` and `schematic/docs/runtime/`.

## Verification

Run `just test`, `just lint`, and `just doctest` in schematic. The L1 generation
guard compiles fresh output and sends through the runtime against wiremock.
`just test-schema` separately checks committed generated clients, pager calls,
and compile-fail fixtures, and `just doctest-schema` (part of `just doctest`)
compiles their doc examples; CI does not build schematic-schema. L2 terminal tests
use `just test-l2` with the internal terminal-tests feature and no-focus harness.
The generator build script reserves an 8 MiB Windows main-thread stack for
catalog assembly (MSVC and GNU); retain it when changing CLI build settings.
Never run cargo fmt unless explicitly requested. No live-provider integration
coverage is implied by the mock-server checks.

Nominal generic registrations must retain concrete public type arguments.
Bitbucket declares its shipped `PaginatedResponse<Item>` implementations beside
the model; registering one in a fresh context works independently of catalog
assembly. Never stringify a generic parameter `T` into an emitted Rust path.

## Authoring guidance maintenance

Use `schematic/docs/authoring-examples.md` for the compiling nominal/alias/container
and API example (included directly in schematic-define rustdoc). The generator
README is an overview; its detailed reference lives in
`schematic/docs/generation/generated-clients.md`. The create-schematic-api command
teaches the same fallible context assembly. Keep these entry points aligned.
The L1 `authoring_guidance` corpus rejects obsolete REST constructor examples.
`Success` covers JSON, text, binary, empty, SSE, NDJSON, and byte streams with
selection modifiers; `ErrorSet::new().on(..)`/`ErrorSet::none()` declare errors
(endpoint `.errors`, API `.default_errors`); `GraphQlOperation` and `Relay`
declare GraphQL operations and their paging.
The definition vocabulary (`path_param`, `public`, `auth`,
`deprecated`, `rate_limit`, `retry_guidance`, `one_of`) exists; see the parent
skill. No selector API is planned here.
