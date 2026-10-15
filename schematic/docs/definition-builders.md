---
kind: canonical-documentation
---

# Building REST definitions

Use `Endpoint` and `RestApi` builders to state the facts needed to generate a client. Create endpoints before the API builder borrows the module group's `DefinitionContext`:

```rust
use schematic_define::prelude::*;

let mut ctx = DefinitionContext::types_in("::my_models");
let endpoint = Endpoint::get("Read", "/items")
    .describes("Read items")
    .returns_json::<Vec<String>>(&mut ctx)
    .build()?;
let api = RestApi::builder(&ctx, "Items", "https://example.com")
    .describes("Item service")
    .bearer_env(["ITEMS_TOKEN"])
    .endpoints([endpoint])
    .build()?;
```

Description and success are required for endpoint `build()`. Description and an explicit authentication choice are required for API `build()`. Missing facts cause a compile error. Neither value nor builder has `Default`; `EndpointParams::new()` creates a valid empty parameter set.

```mermaid
flowchart LR
    C[Register types in module context] --> E[Build endpoints with description and success]
    E --> A[Declare API description and authentication]
    A --> V[Check destination references and auth coherence]
    V --> R[RestApi and ordered context registry]
```

`Endpoint::new(id, method, path)` covers every `RestMethod`. Named constructors cover `get`, `post`, `put`, `patch`, `delete`, `head`, and `options`. Empty ids/descriptions and paths without a leading `/` fail at build. API names/descriptions must be nonempty and the base URL must have an HTTP(S) scheme and host. Paths containing `?` fail shared validation and endpoint/API build; declare query inputs with `.query(ParamDef::new(...))`. Endpoint identity is `(method, path)`, with placeholder names normalized for duplicate checks. Query values and response shapes do not create separate routes.

Choose one success with `returns(Success::json(desc))`, `Success::text()`, `Success::binary()`, or `Success::empty()`. Shorthands are `returns_json::<T>(&mut ctx)`, `returns_text()`, `returns_binary()`, and `returns_empty()`. A second success declaration fails with “several success shapes arrive with response model v2.” `errors(ErrorSet::none())` and API `default_errors(ErrorSet::none())` reserve the response vocabulary; no other error-set constructor exists yet.

Requests use `request_json::<T>(&mut ctx)`, `request_form(fields)`, `request_urlencoded(fields)`, `request_text()`, or `request_binary()`. `FormField::json_desc("metadata", desc)` authors typed JSON metadata without naming the stored Schema type. Forms retain field names, required flags, descriptions, file MIME patterns, and file-count limits. Text and binary defaults are `text/plain` and `application/octet-stream`; their content type can be stated explicitly with the corresponding `_with_content_type` method. These declarations preserve output-specific transport validation; generated text and binary request fields are sent as raw bodies. A GET endpoint rejects every request-body form at endpoint build. API build also rejects a transferred GET body with destination API, endpoint and method context. HEAD keeps its existing behavior.

Typed `Schema` values retain optional descriptor evidence alongside their Rust path. API build validates this evidence, including component names, wrappers, reserved structure, and agreement with the lowered path. Missing destination registrations, missing descriptor evidence on JSON schemas, forged descriptors, and collected collisions fail. Typed OpenAPI export uses the descriptor's wire schema, so Rust qualification does not erase component references or wrappers. A root typed `Vec<T>` retains automatic GET list recognition; `Option<Vec<T>>` and nominal collection wrappers still require authored collection information. Passing an untyped JSON schema into the API builder fails. REST schemas store a qualified `type_name` plus descriptor evidence; `Schema.module_path` is retained only for WebSocket compatibility. See [typed registration](typed-registration.md).

For authentication, `no_auth()`, `bearer_env(names)`, and `basic_env(username, password_names)` make coherent declarations. `authentication(strategy, names)` supports API keys, parameter keys, OAuth2, and custom bearer headers. The stored auth types are unchanged. A second authentication declaration fails even if it repeats the same strategy. An explicit `env_mapping(mapping)` must agree with each environment value supplied by the auth declaration, in either setter order. For example, declaring `bearer_env(["TOKEN"])` and mapping bearer credentials to `OTHER_TOKEN` fails. Extra mappings support an intentional multi-method `auth_policy(policy)`; that policy remains authoritative through `effective_auth_policy()`.

Headers, query parameters, complete parameter sets, documentation URL, version, module path, request suffix, and API cross-origin policy retain the current stored representation and generator precedence. Endpoint headers override matching API headers through the generator's existing merge. Builders preserve declaration order. Endpoint `cross_origin(policy)` replaces the API policy for that endpoint, including redirects and followed pagination links. `allow_origins` accepts only bare HTTP(S) origins, with an optional port and trailing slash; paths, user information, queries and fragments fail API build. Caller policy settings may only tighten each applicable definition policy. Runtime-only origin and redaction settings are not represented as OpenAPI/Postman transport behavior.

Declare a parameter location with `query(param)`, `header_param(param)`, or `cookie(param)`. Path parameters remain explicit placeholders in the path template, and request methods declare the body location. Constant headers still use `header(name, value)`. Location never changes route identity.

```rust
use schematic_define::{CrossOrigin, Endpoint, ParamDef, QueryParamType};

let param = ParamDef::new("ref", QueryParamType::String)
    .rust_name("git_ref").secret();
let endpoint = Endpoint::get("ReadFile", "/files/{file}")
    .describes("Read a file").returns_empty().query(param)
    .cross_origin(CrossOrigin::allow_origins(["https://downloads.example.com"]))
    .build()?;
```

`ParamDef::new` creates an optional parameter; `required()` and `describes(text)` set existing metadata. `rust_name` changes only the Rust field and setter; encoding and export retain the wire name. For example, `ref` is sent on the wire while callers use `git_ref`. Optional `beta` and `recursive` parameters are ordinary Boolean inputs: omission sends no value, and a supplied false remains false.

API `request_id_header("X-Request-ID")` and `secret_header("X-Pin")` normalize names to lowercase and reject invalid HTTP header names. Parameter `secret()` adds its wire name to the predecessor runtime's redaction metadata. The existing auth strategy markers, redaction behavior and explicit `auth_policy` semantics remain authoritative.

`pager_managed()` records an adoption requirement, preserved in serialized definitions. API build resolves the strategy and requires a matching location and wire name with an exposed caller control. A marker alone cannot hide an input. Required managed inputs additionally need a pagination-authored sent default; the existing `pager-managed-required-without-default` diagnostic identifies a missing default. `default_pagination(strategy)` authors an API strategy; `.pagination(Collection)` assigns an endpoint collection, opt-out, or strategy override. Pagination exception tables and legacy parameter pagination fields are removed.

New optional fields are omitted when unused, preserving historical definition JSON. Absent/null origin overrides and Rust aliases mean no override/alias; absent marker flags mean false, while null/wrong-type flags are Serde errors. These are definition transfer fields rather than a new configuration format. The later response vocabulary remains **planned**.

## Default allow-list

The public defaults below remain meaningful; the audit test checks the shipped source inventory so new defaults require an explicit decision.

| Type | Why its default is valid |
| --- | --- |
| `EndpointParams` | Empty parameters describe a valid endpoint. |
| `AuthStrategy`, `AuthPolicy` | Anonymous auth / no accepted auth or fallback is a valid policy. |
| `CrossOrigin` | Same-origin is the safe complete policy. |
| `EnvMapping`, `Headers` | No environment credentials or extra headers is valid. |
| `SensitiveString` | An empty secret is valid data; it makes no authentication claim. |
| `PkceRequirement`, `OAuth2ClientAuthMethod` | Required PKCE / client-secret Basic are explicit complete protocol defaults. |
| `ConnectionLifecycle` | Empty optional lifecycle messages are valid. |
| `WebSocketEndpoint` | Retained for existing incremental WebSocket construction, which is outside this REST migration. Its empty id still needs authoring before generation. |
| `FrameFormat`, `RequestIdType` | JSON text frames / u64 request ids are complete established defaults. |
| `WebSocketRuntimeHints`, `WebSocketEndpointHints` | No optional hints is valid. |
| `SchematicDocExtension`, `SchematicSchemaExtension` | All optional extension metadata may be absent. |
| `ExportOptions`, `ExportFormat`, `OpenApiImportOptions` | Complete tooling options with established defaults. |

All shipped REST providers and the importer use typed builders. `Endpoint` and `RestApi` are `#[non_exhaustive]`, preventing external struct literals. Generated request types retain their existing defaults. The unused endpoint `oauth_scopes` field is removed; explicit API `auth_policy` remains authoritative.

## Definition transfer input shapes

The JSON transfer matrix below describes the new authoring fields. Invalid input
fails deserialization or public API assembly; it never hides a parameter. The
positive control uses a required, adopted `limit` with a sent default of 25.
YAML is not a separate definition-loading interface.

| JSON shape | Endpoint `cross_origin` | Parameter `rust_name` | Parameter `secret` | Parameter `pager_managed` |
| --- | --- | --- | --- | --- |
| absent | inherit API | derive identifier from wire name | false | false; normal automatic adoption still applies |
| null | inherit API | derive identifier from wire name | error | error |
| wrong whole type | error | error | error | error |
| one wrong element | invalid allow-list member: error | array instead of string: error | array instead of Boolean: error | array instead of Boolean: error |
| every wrong element | error | error | error | error |
| empty | empty allow-list remains an explicit policy | empty identifier: error | empty array: error | empty array: error |
| duplicate key | error | error | error | error |
| trailing garbage | error | error | error | error |

A present `pager_managed: true` without matching strategy input and caller control
is an error, including when no strategy resolves. A matching required input with
no sent default reports `pager-managed-required-without-default`.

Imported definition modules use these builders too. Their public
`module_group()` returns a fallible assembly containing the API and registry
from one ordered context; the catalog factory and registry helper unwrap it.
The importer emits nominal Registration implementations beside generated
models. Aliases reuse their underlying registration; containers and inline
primitive/JSON types do not become extra components. See
[OpenAPI imports](io/import-openapi.md) for normalization and overlay behavior.

Use `.pagination(Collection)` on an endpoint to declare its collection or
pagination override. An explicit collection is resolved before root-array inference
and the API default. OpenAI's overlay assigns this same endpoint field.

An explicit environment mapping may leave credential fields unset. An empty mapping names no environment credentials; it remains distinct from omitting the mapping and deriving one from the authentication fields. When both declarations name the same credential kind, their names must agree, regardless of setter order.
