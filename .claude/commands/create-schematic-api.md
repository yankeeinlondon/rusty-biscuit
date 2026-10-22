---
name: create-schematic-api
argument-hint: <api-name> <brief description, API docs URL, or @doc reference>
description: Create a REST API definition with typed registration and checked builders
---

Use the `schematic-define` skill. The requested action is `$ARGUMENTS`.
The first token is the snake_case provider module; the remainder is its brief.
If arguments are missing, report that the API name and brief are required and stop.

## Read before authoring

Read `schematic/docs/authoring-examples.md`, `definition-builders.md`,
`typed-registration.md`, and `pagination.md`. These are the current contracts.
Use `schematic/definitions/src/anthropic/` for a small provider, `github/` for
pagination, and `emqx/` or `ollama/` for shared-module API variants.

If the provider publishes OpenAPI, prefer the generic import workflow in
`schematic/docs/io/import-openapi.md`. OpenAI is an imported example with 258
endpoints; `just -f schematic/justfile import-openai` preserves its handwritten
pagination overlay. Never hand-edit imported or generated artifacts.

## Required implementation

Create `schematic/definitions/src/{api_name}/{mod,types}.rs`. Keep models typed,
with accurate Serde names and nullability. Use `*Body` for body models to avoid
colliding with generated `*Request` wrappers. Body models need meaningful
`Default` implementations where generated request defaults require them.
Declare nominal `Registration` implementations beside models with
`definition_type!` (or an explicit implementation for concrete generic models).

Expose `module_group() -> Result<Vec<RestApi>, DefinitionError>` through
`schematic_definitions::registry::assemble_module_group`. Its closure receives
one mutable `DefinitionContext` and returns all APIs sharing that module.
Register explicit models in the intended component order before endpoints.
Use `register_as::<T>(rust_path!(public::Alias))` before T's first registration
for aliases/re-exports; add `.schema_name(...)` only for a different component
name. Containers recurse into leaves, and primitives/free-form JSON stay inline.
The public module path must be reachable from the generated crate.

Build endpoints before borrowing the context for the API:

```rust
let endpoint = Endpoint::get("ListItems", "/items")
    .describes("List items")
    .returns_json::<Vec<Item>>(context)
    .build()?;
let api = RestApi::builder(context, "Items", "https://example.com")
    .describes("Item service")
    .bearer_env(["ITEMS_TOKEN"])
    .endpoints([endpoint])
    .build()?;
```

Supply description and exactly one success before endpoint build. Choose typed
JSON, `returns_text`, `returns_binary`, or `returns_empty` to match the wire.
SSE/NDJSON use binary success with caller framing. JSON body methods receive
`&mut DefinitionContext`; forms use `request_form` / `request_urlencoded`, typed
form metadata uses `FormField::json_desc`. Text/binary body methods support
explicit request content types.

Supply description and exactly one auth choice before API build: `no_auth`,
`bearer_env`, `basic_env(username, password_names)`, or
`authentication(strategy, env_names)`. Basic keeps both variable lists inside
`AuthStrategy::Basic` and never reads a password from `env_names`. OAuth2 is
one `OAuth2Config` variant per grant (`AuthorizationCodePkce`,
`ClientCredentials`, `DeviceCode`), each carrying only its own URLs. Explicit
`env_mapping` credential names must agree with auth declarations in either setter
order. Keep intentional multi-method `auth_policy`; an empty mapping remains
an explicit choice. Return assembly errors with `?`; catalog compatibility
factories may unwrap the checked assembly with the existing contextual message.

Use path placeholders, `.query(ParamDef::new(...))`, `.header_param(...)`,
`.cookie(...)`, or a request body to declare location. `.header(name, value)`
is a constant header. GET rejects every body form; HEAD behavior is unchanged.
A path cannot contain `?`. Endpoint identity is `(method, normalized_path)`;
query values and body shapes cannot define separate endpoints on one route.
Wire keywords use `rust_name`, for example `ref` → `git_ref`.

Query flags shape the generated client, so state them as the provider documents
them. A `.required()` query parameter becomes a `new()` and endpoint-method
argument after the path parameters. The request then has no `Default`.
`QueryParamType::Enum` generates an enum. `style`/`explode` pick the array form;
`ParamDef::new` is form-style and comma-delimited, so set the public `explode` field to
`true` when the provider expects repeated keys. A query shape the client cannot send
(JSON, nested arrays, deepObject) is a `query-parameter` warning when optional,
an error when required.

Optional vocabulary, used only when the provider documents the fact:

```rust
let endpoint = Endpoint::get("GetItem", "/items/{id}")
    .describes("Get an item")
    .returns_json::<Item>(context)
    .path_param(ParamDef::path("id", QueryParamType::Integer))
    .deprecated()
    .public()
    .build()?;
```

`path_param` declarations must equal the template's placeholders. `.public()`
or `.auth(policy)` overrides the API policy. `.rate_limit(..)` and
`.retry_guidance(..)` are metadata only; nothing retries. Use
`context.one_of::<T>([..])` (or `any_of`/`all_of`) for a composed body.

Keep pagination strategy helpers provider-local. Use API `default_pagination`
and endpoint `pagination(Collection)`. `pager_managed()` alone cannot hide a
parameter: strategy location/name, exposed caller control, and a sent default
for required inputs must agree. Do not add a shared preset layer.

Set `module_path` and distinct `request_suffix` values for shared API variants.
Use `request_id_header`, `secret_header`, and parameter `secret()` for metadata.
Endpoint `cross_origin` replaces the API policy; runtime callers may only tighten
it. Allowed origins must be bare HTTP(S) origins.

Register the module, catalog lookup/group assembly, and documented provider row.
Update the appropriate shipped-policy and ordered endpoint-id inventories.
READMEs remain overviews; current behavior belongs in the area's docs tree.
Future response methods remain planned; only `ErrorSet::none()` is
available. Match non-exhaustive types with a wildcard where needed; CrossOrigin
is intentionally exhaustive.

## Verification and report

Map behaviors to public-result tests before implementation. Cover valid and
invalid declarations, component order, export shapes, and generated wire behavior
at the appropriate fidelity. Do not rely on tests that merely repeat literals.

Run in `schematic/`:

```sh
just generate
just test
just lint
just doctest
just test-schema
just lint-schema
just migration-check
```

These are nextest-based; do not substitute `cargo test`. When a generated
signature changes, build the dependents that call it with
`cargo build -p <crate> --all-features --all-targets`, because some call sites
sit behind features.
Review full Rust/OpenAPI/Postman file sets; never edit generated files manually.
Do not run formatting commands, stage, or commit unless explicitly instructed.
Report changed files, endpoint catalog, auth/env inputs, verification results,
and remaining fidelity gaps. No live-provider fidelity is implied by mock tests.
