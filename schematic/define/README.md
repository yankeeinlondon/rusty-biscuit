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

Read the current behavior and examples in:

- [Definition builders](../docs/definition-builders.md): required facts, auth coherence, parameter locations, route identity, and validation.
- [Typed registration](../docs/typed-registration.md): nominal types, aliases, containers, ordered module groups, and destination validation.
- [Pagination](../docs/pagination.md): API defaults, endpoint collections, caller controls, and parameter adoption.
- [Definition snapshots](../docs/definition-snapshots.md): serialized transfer contracts.
- [Headers](../docs/headers/README.md) and [runtime](../docs/runtime/README.md): credentials and execution policy.
- [OpenAPI extensions](../docs/io/openapi-extensions.md) and [imports](../docs/io/import-openapi.md).
- [Package dependencies](../docs/dependencies.md).

The Rust API documentation covers REST models, `Headers`, form fields,
WebSocket messages and lifecycle, `ModelCatalog`, and `TypeRef`. Enable the
`openapi` feature for OpenAPI import/export. The
[definitions crate](../definitions/README.md) provides shipped APIs.
