# schematic-define

Definition types and typed builders for Schematic REST and WebSocket clients.
This crate describes APIs; `schematic-gen` generates clients and
`schematic-runtime` executes their requests.

```toml
[dependencies]
schematic-define = { path = "../schematic/define" }
```

Build a REST endpoint and its API in one type-registration context:

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

Endpoints require a description and success shape. APIs require a description
and an explicit authentication choice. Both builds are fallible; REST structs
are non-exhaustive and external callers use builders. WebSocket definitions
retain their existing construction API.

Optional vocabulary adds facts a definition can state when the provider has
them:

```rust
let endpoint = Endpoint::get("GetIssue", "/repos/{owner}/{repo}/issues/{number}")
    .describes("Get an issue")
    .returns_json::<Issue>(&mut context)
    .path_param(ParamDef::path("owner", QueryParamType::String))
    .path_param(ParamDef::path("repo", QueryParamType::String))
    .path_param(ParamDef::path("number", QueryParamType::Integer))
    .query(ParamDef::new("fields", QueryParamType::String).required())
    .deprecated()
    .public()
    .build()?;
// Generated: GetIssueRequest::new(owner, repo, number, fields), with no Default
```

- **Typed path parameters** (`path_param`) must match the template's
  placeholders. **Required query parameters** become constructor arguments,
  after the path parameters. A parameter a pager adopts is the exception.
- **Endpoint authentication**: `.public()` sends no credential, and strips any
  raw header placed where a credential goes. `.auth(policy)` replaces the
  API policy.
- **`deprecated`** reaches docs and OpenAPI. **`rate_limit`** and
  **`retry_guidance`** are metadata only; nothing retries.
- **Basic** keeps its username and password variable lists inside
  `AuthStrategy::Basic`. **OAuth2** is one `OAuth2Config` variant per grant.
- **Several success shapes and streams**: `.returns(..)` may repeat, as in
  `.returns(Success::json(message)).returns(Success::sse(event).ends_at_event("message_stop"))`;
  the client picks one by status, `Content-Type`, or JSON tag.
  `Success::ndjson` and `Success::bytes` declare the other streams.
- **Declared errors**: `.errors(ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::text()))`
  on an endpoint overlays the API's `default_errors(..)`; `ErrorSet::none()`
  opts out.
- **GraphQL operations**: `.graphql(AddressRule::github()).operations([..])` on
  the API adds typed queries and mutations, which can page a Relay connection.
- **Composed bodies**: `context.one_of::<T>([..])`, `any_of`, and `all_of` bind
  a hand-written type to an OpenAPI composition.
- **`DefinitionBundle`** serializes an API with its schemas. It loads without
  the Rust types, for comparison and export; it cannot generate a client.

Read the current behavior and examples in:

- [Definition builders](../docs/definition-builders.md): required facts, auth coherence, parameter locations, route identity, and validation.
- [Typed registration](../docs/typed-registration.md): nominal types, aliases, containers, ordered module groups, and destination validation.
- [Success shapes and declared errors](../docs/responses-and-errors.md): several shapes at one address, streams, error tables, and the generated success and failure types.
- [GraphQL operations](../docs/graphql.md): address rules, reply classification, and Relay pagination.
- [Pagination](../docs/pagination.md): API defaults, endpoint collections, caller controls, and parameter adoption.
- [Definition snapshots](../docs/definition-snapshots.md) and [definition bundles](../docs/io/definition-bundles.md): serialized transfer contracts.
- [Request inputs](../docs/generation/request-side.md) and [credentials](../docs/runtime/credentials.md#endpoint-authentication): query fidelity and endpoint authentication on the wire.
- [Headers](../docs/headers/README.md) and [runtime](../docs/runtime/README.md): credentials and execution policy.
- [OpenAPI extensions](../docs/io/openapi-extensions.md) and [imports](../docs/io/import-openapi.md).
- [Package dependencies](../docs/dependencies.md).

The Rust API documentation covers REST models, `Headers`, form fields,
WebSocket messages and lifecycle, `ModelCatalog`, and `TypeRef`. Enable the
`openapi` feature for OpenAPI import/export. The
[definitions crate](../definitions/README.md) provides shipped APIs.

See [the compiling authoring example](../docs/authoring-examples.md) for nominal models, public
aliases, containers, and complete context-based API assembly.
