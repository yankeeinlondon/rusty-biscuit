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
`assemble_module_group` returns APIs and their ordered `SchemaRegistry` together;
`SchemaRegistry::from_context` is the checked adapter, including nested-schema
extraction. There is no separate string registry to maintain.

REST schemas carry a qualified Rust path and typed descriptor evidence. Export
uses the descriptor's wire shape: `Vec<Item>` is an inline array referencing the
Item component, `Option` preserves nullability, and primitives/free-form JSON do
not invent components. Public aliases use typed `register_as` before ordinary
registration. Schema names and generated Rust paths remain distinct.

API assembly rejects mismatched destination references and collisions. Strict
export completeness rejects missing component references rather than emitting
incomplete documents. Explicit unused registrations and their order are retained.
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

Registry coverage is intentionally strict. Missing a schema registry for a
module fails generation so incomplete OpenAPI artifacts are not written. Use
`--no-openapi` when generating Rust clients during a registry migration.

## Grouped Export

APIs sharing a module (`ollama`, `emqx`) produce grouped OpenAPI documents that merge:

- **Paths** — Union of all endpoints from all APIs. An operation that two
  members declare identically apart from authentication appears once; two
  different operations on one method and path fail the OpenAPI output.
- **operationId** — Unique across the document. The first member in registry
  order keeps an id; a later clash is prefixed with its API name
  (`OllamaOpenAIListModels`), with a numeric suffix if that is taken too.
- **Security schemes** — Union keyed by scheme name; a same-named scheme with
  different content is renamed `<scheme>_<api_snake>` for the later API
- **Security** — Union of every member's document-level requirements
- **Servers, external docs, info** — From the first member, with the module
  name as `info.title`
- **`x-schematic-grouped-from`** — Every contributing API name

The full rule, including link updates and the final duplicate scan, is in
[Generation Validation](../generation/validation.md#openapi-documents-of-a-module-group).

## Schema Registries

All 18 shipped REST API views assemble typed schema registries; nominal models implement `Registration` with `JsonSchema`. Missing a registry for a new API produces an error (use `--no-openapi` to skip).

## Default Artifact Generation

When the output path ends with `schema/src`, OpenAPI specs are generated automatically to `openapi/` without requiring explicit `--openapi-out`. Use `--no-openapi` to suppress.

## Open Questions / Future Enhancements

- Consider an AsyncAPI export path for WebSocket definitions.
- Provide a `--bundle` flag to emit a single combined spec for `--api all`.
