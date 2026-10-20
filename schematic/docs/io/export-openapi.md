---
kind: canonical-documentation
---

# Exporting OpenAPI from Schematic

Generate OpenAPI 3.0 documents alongside Rust clients from the same checked
module-group assembly. APIs sharing a generated module produce one document.

```sh
schematic-gen generate --api all --output schematic/schema/src \
    --openapi-out schematic/openapi
schematic-gen generate --api github --output schematic/schema/src \
    --openapi-out schematic/openapi --openapi-format yaml
```

From the area, `just generate-openapi` emits Rust and OpenAPI artifacts while
suppressing Postman output. `just generate` emits all three forms. Generated
files are never hand-edited. See [generation workflow](../generation/workflow.md).

## Schema source and validation

Author nominal types with `definition_type!` and register them in a
`DefinitionContext`. Build endpoints, then build the API against that context.
API build stores that context's components in `RestApi::schemas`, and
`assemble_module_group` gives every API of a module group the finished
context's components. Export reads nothing else: `openapi::export(api, options)`
writes `RestApi::schemas` as `components.schemas`, in the same order, with
types that appear only nested inside a registered type flattened into
components of their own. There is no separate registry to maintain.

An endpoint's JSON body is a `TypeRef` naming components, and export turns it
into a wire schema: `Named("Item")` becomes `$ref: '#/components/schemas/Item'`,
`Array(Named("Item"))` an inline array of that reference, `Optional(..)` a
nullable schema, and primitives and free-form JSON (`Primitive(Json)`) stay
inline without inventing components. Public aliases use typed `register_as`
before ordinary registration. Schema names and generated Rust paths remain
distinct.

API assembly rejects a reference naming no registered component, and the
generator reports it as `invalid-schema-reference` before writing anything.
Strict export completeness also rejects a `$ref` with no matching component
rather than emitting an incomplete document. Explicit unused registrations and
their order are retained.
See [typed registration](../typed-registration.md) and
[the compiling example](../authoring-examples.md).

Query/header/cookie parameters export in their declared locations. Query strings
do not belong in path templates; endpoint identity is method plus normalized
path. API authentication and explicit `auth_policy` determine security schemes.
Runtime-only origin/redaction policies do not become OpenAPI transport rules.
The supported extension fields are described in
[OpenAPI extensions](openapi-extensions.md).

## Version Resolution

OpenAPI spec version is resolved with this priority chain:

1. `--openapi-version <VERSION>` CLI flag (highest priority)
2. `RestApi.version` field on the API definition
3. Fallback: `"0.1.0"`

## Shared Modules

OpenAPI artifacts are named from the resolved generated module, not from the
individual API definition name. APIs that share a module are exported into one
combined document:

- `OllamaNative` + `OllamaOpenAI` -> `ollama.json`
- `EmqxBasic` + `EmqxBearer` -> `emqx.json`

The same naming convention applies to single-API modules such as
`HuggingFaceHub` -> `huggingface.json`, `SamsungSmartTv` ->
`samsung_smart_tv.json`, and `UnfoldedCircleCoreRest` ->
`unfolded_circle_core_rest.json`.

Component coverage is intentionally strict. A reference that does not resolve
in its API's `schemas` fails generation for both Rust and OpenAPI output, so
neither a broken client nor an incomplete OpenAPI artifact is written.

## Grouped Export

APIs sharing a module (`ollama`, `emqx`) produce grouped OpenAPI documents that merge:

- **Components** — Every member carries the same `schemas`, so the
  documents' `components.schemas` collapse into one
- **Paths** — Union of all endpoints from all APIs. An operation that two
  members declare identically apart from authentication appears once; two
  different operations on one method and path fail the OpenAPI output.
- **operationId** — Unique across the document. The first member in module
  group order keeps an id; a later clash is prefixed with its API name
  (`OllamaOpenAIListModels`), with a numeric suffix if that is taken too.
- **Security schemes** — Union keyed by scheme name; a same-named scheme with
  different content is renamed `<scheme>_<api_snake>` for the later API
- **Security** — Union of every member's document-level requirements
- **Servers, external docs, info** — From the first member, with the module
  name as `info.title`
- **`x-schematic-grouped-from`** — Every contributing API name

The full rule, including link updates and the final duplicate scan, is in
[Generation Validation](../generation/validation.md#openapi-documents-of-a-module-group).

## Schema Components

All 18 shipped REST APIs carry their components in `RestApi::schemas`; nominal models implement `Registration` with `JsonSchema`. A new API gets its components by building through a `DefinitionContext`, so there is nothing extra to register for export.

## Default Artifact Generation

When the output path ends with `schema/src`, OpenAPI specs are generated automatically to `openapi/` without requiring explicit `--openapi-out`. Use `--no-openapi` to suppress.

## Open Questions / Future Enhancements

- Consider an AsyncAPI export path for WebSocket definitions.
- Provide a `--bundle` flag to emit a single combined spec for `--api all`.
