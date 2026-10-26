# schematic-define

Definition types and typed builders for Schematic REST and WebSocket clients.
This crate describes APIs; `schematic-gen` generates clients and
`schematic-runtime` executes their requests.

```toml
[dependencies]
schematic-define = { path = "../schematic/define" }
```

Every Rust example on this page compiles and runs as a `schematic-define`
doctest (`just doctest`). None of them sends a request.

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
assert_eq!(api.endpoints[0].id, "ListItems");
# Ok::<(), schematic_define::DefinitionError>(())
```

Endpoints require a description and success shape. APIs require a description
and an explicit authentication choice. Both builds are fallible; REST structs
are non-exhaustive and external callers use builders. WebSocket definitions
retain their existing construction API.

## Endpoint facts

Optional vocabulary adds facts a definition can state when the provider has
them:

```rust
use schematic_define::prelude::*;
use schematic_define::{definition_type, ParamDef, QueryParamType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

definition_type! {
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct Issue { pub title: String }
}

let mut context = DefinitionContext::types_in("::my_models");
let endpoint = Endpoint::get("GetIssue", "/repos/{owner}/{repo}/issues/{number}")
    .describes("Get an issue")
    .returns_json::<Issue>(&mut context)
    .path_param(ParamDef::path("owner", QueryParamType::String))
    .path_param(ParamDef::path("repo", QueryParamType::String))
    .path_param(ParamDef::path("number", QueryParamType::Integer))
    .query(ParamDef::new("fields", QueryParamType::String).required())
    .deprecated()
    .build()?;
// Generated: GetIssueRequest::new(owner, repo, number, fields), with no Default
assert!(endpoint.deprecated);
// A query string in the path template is a build error.
assert!(Endpoint::get("Search", "/items?limit=10").describes("Search").returns_text().build().is_err());
# Ok::<(), schematic_define::DefinitionError>(())
```

- **Typed path parameters** (`path_param`) must match the template's
  placeholders. **Required query parameters** become constructor arguments,
  after the path parameters. A parameter a pager adopts is the exception.
- **A path template has no query string.** `/items?limit=10` fails the build;
  declare `limit` with `.query(..)` instead.
- **`deprecated`** reaches docs and OpenAPI. **`rate_limit`** and
  **`retry_guidance`** are metadata only; nothing retries.

## Authentication policies

The API declares one authentication choice: `no_auth()`, `bearer_env(names)`,
`basic_env(username, password_names)`, or `authentication(strategy, names)`
for API keys, parameter keys, OAuth2, and custom bearer headers. Basic keeps
its username and password variable lists inside `AuthStrategy::Basic`; OAuth2
is one `OAuth2Config` variant per grant. An endpoint inherits that policy
unless it declares its own:

```rust
use schematic_define::prelude::*;

let mut context = DefinitionContext::types_in("::my_models");
let health = Endpoint::get("Health", "/health")
    .describes("Service health")
    .returns_text()
    .public()
    .build()?;
let upload = Endpoint::post("Upload", "/uploads")
    .describes("Upload a file")
    .request_binary()
    .returns_empty()
    .auth(AuthPolicy {
        explicit: vec![AuthMethod::BearerToken { header: Some("X-Upload-Token".into()) }],
        env_fallback: None,
    })
    .build()?;
let api = RestApi::builder(&context, "Items", "https://example.com")
    .describes("Item service")
    .bearer_env(["ITEMS_TOKEN"])
    .endpoints([health, upload])
    .build()?;
assert!(matches!(api.endpoint_auth(&api.endpoints[0]), ResolvedEndpointAuth::Public));
# Ok::<(), schematic_define::DefinitionError>(())
```

- `.public()` sends no credential, and strips any raw header placed where a
  credential goes.
- `.auth(policy)` replaces the API policy and reads credentials through the
  API's environment mapping; `.auth_with_env_mapping(policy, mapping)`
  replaces the mapping too. A policy with neither an explicit method nor an
  environment fallback fails the build; use `.public()` instead.
- `RestApiBuilder::auth_policy(policy)` declares a multi-method API policy,
  which stays authoritative over the strategy.

## Success shapes and declared errors

`.returns(..)` may repeat; the client picks one shape by status,
`Content-Type`, or JSON tag. `Success::ndjson` and `Success::bytes` declare the
other streams. `.errors(..)` on an endpoint overlays the API's
`default_errors(..)`, and `ErrorSet::none()` opts out.

```rust
use schematic_define::prelude::*;
use schematic_define::{ErrorSet, ErrorShape, StatusMatch, Success};

let mut context = DefinitionContext::types_in("::my_models");
let message = context.register::<serde_json::Value>()?;
let event = context.register::<serde_json::Value>()?;
let endpoint = Endpoint::post("CreateMessage", "/messages")
    .describes("Create a message, optionally streamed")
    .request_json::<serde_json::Value>(&mut context)
    .returns(Success::json(message))
    .returns(Success::sse(event).ends_at_event("message_stop"))
    .errors(ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::text()))
    .build()?;
let api = RestApi::builder(&context, "Messages", "https://example.com")
    .describes("Message service")
    .bearer_env(["MESSAGES_TOKEN"])
    .default_errors(ErrorSet::new().on(StatusMatch::class(5), ErrorShape::text()))
    .endpoints([endpoint])
    .build()?;
assert_eq!(api.endpoints[0].response.len(), 2);
# Ok::<(), schematic_define::DefinitionError>(())
```

## Pagination

An API declares a default strategy, and an endpoint's `Collection` says where
its items are and how it pages. Each provider writes its own strategy
function; this crate ships no vendor presets.

```rust
use schematic_define::prelude::*;

fn page_number() -> Strategy {
    Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("per_page").server_default(30).max(100))
        .build()
}

let mut context = DefinitionContext::types_in("::my_models");
let tags = Endpoint::get("ListTags", "/tags")
    .describes("List tags, page by page")
    .returns_json::<Vec<String>>(&mut context)
    .build()?;
let owners = Endpoint::get("ListOwners", "/owners")
    .describes("List owners in one reply")
    .returns_json::<Vec<String>>(&mut context)
    .pagination(Collection::root().unpaginated())
    .build()?;
let api = RestApi::builder(&context, "Tags", "https://example.com")
    .describes("Tag service")
    .no_auth()
    .default_pagination(page_number())
    .endpoints([tags, owners])
    .build()?;
assert!(api.default_pagination.is_some());
# Ok::<(), schematic_define::DefinitionError>(())
```

`ListTags` inherits the default; `ListOwners` stays a single request. For a
wrapper response, use `Collection::at("data").item_type("Item")`.

## More vocabulary

- **GraphQL operations**: `.graphql(rules).operations([..])` on the API adds
  typed queries and mutations, which can page a Relay connection.
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
