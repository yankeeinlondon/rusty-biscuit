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
path. The API's authentication strategy (`auth`) determines the document's
security scheme and requirement; an endpoint's own authentication adds
operation-level `security` (see [Endpoint security](#endpoint-security)).
Runtime-only origin/redaction policies do not become OpenAPI transport rules.
The supported extension fields, including rate-limit and retry metadata, are
described in [OpenAPI extensions](openapi-extensions.md).

## Responses

Each operation's `responses` map holds the endpoint's success shapes and its
resolved error table, one response object per status (see
[Responses and errors](../responses-and-errors.md#what-is-stored)); the rules
below apply to every shape it declares.

**Success statuses.** A shape exports under the statuses it declares, else
`204` when it is empty and `200` otherwise. Every shape under one status
shares that status's response object, one `content` entry per media type:

| Shape | Media key | Schema |
| --- | --- | --- |
| JSON | declared, else `application/json` | the referenced component |
| Text | declared, else `text/plain` | `type: string` |
| Binary, byte stream | declared, else `application/octet-stream` | `type: string, format: binary` |
| SSE stream | declared, else `text/event-stream` | the item component |
| NDJSON stream | declared, else `application/x-ndjson` | the item component |
| The fallback shape | `*/*` | a (binary) string |
| Empty | none | none |

A stream's framing and end condition, and anything else the map cannot say,
are written to the operation's [`x-schematic`](openapi-extensions.md#success-shapes-and-error-entries).
Two JSON shapes under one status and media type become a `oneOf`; when each
declares a field tag on the same field, a `discriminator` names it and maps
each string value to its component. Each shape's note joins the response
description as `<media type>: <note>`:

```yaml
"200":
  description: |-
    Successful response

    application/json: returned when `stream` is false

    text/event-stream: returned when `stream` is true
  content:
    application/json: { schema: { $ref: "#/components/schemas/Message" } }
    text/event-stream: { schema: { $ref: "#/components/schemas/Event" } }
```

**Error statuses.** Export writes the endpoint's *resolved* table (API
defaults, overlaid or opted out by the endpoint), as the client looks it up:

- an exact entry keys its status (`"404"`);
- a class range (`400-499`, `500-599`) keys `4XX`/`5XX` when it wins every
  status of its class;
- a class range that a narrower entry supersedes in part, and every other
  range, expands into exact statuses, each holding the entry that wins there.

```text
API defaults 4XX -> Problem, 5XX -> text; endpoint adds 404 -> empty
exported:    "400" ... "403" -> Problem, "404" -> no content,
             "405" ... "499" -> Problem, "5XX" -> text
```

Success statuses are 2xx and error statuses 400-599, so the two never share a
key. GraphQL operations are not exported: each is reported as the warning
`<API> operation <id>: GraphQL operation skipped in OpenAPI output` and the
document is still written.

## Path parameters, deprecation, and composition

A path parameter declared with `path_param` exports with its type, its
description, `required: true`, and `deprecated: true` when it is deprecated.
An endpoint that declares none keeps one untyped `string` parameter per
placeholder. A placeholder repeated in the template, or written `{+name}`,
is declared once:

```yaml
/items/{item_id}:
  parameters:
    - in: path
      name: item_id
      description: Item to read
      required: true
      schema: { type: integer }
      style: simple
  get: { ... }
  delete:
    parameters:            # this operation declares item_id differently
      - in: path
        name: item_id
        description: Item to delete
        required: true
        deprecated: true
        schema: { type: integer }
        style: simple
```

Path parameters live on the path item, shared by its operations. When the
operations of one path declare a parameter differently, the path item holds
the declaration of the first operation in the order `get`, `put`, `post`,
`delete`, `options`, `head`, `patch`, and each operation that differs carries
its own declaration as an operation-level override. The result never depends
on the order the endpoints were defined in. An `Enum` type exports as
`{type: string, enum: [..]}`.

A deprecated endpoint's operation has `deprecated: true`, as does a
deprecated query, header, or cookie parameter; nothing is written for one
that is not deprecated.

A composed component (`DefinitionContext::one_of`, `any_of`, or `all_of`;
see [typed registration](../typed-registration.md)) exports as that
composition of references, for example
`Pet: {oneOf: [{$ref: '#/components/schemas/Cat'}, {$ref: '#/components/schemas/Dog'}]}`.

## Endpoint security

An endpoint that declares its own authentication gets an operation-level
`security`; one that inherits gets none and uses the document's:

| Endpoint declaration | Operation `security` |
| --- | --- |
| none (inherits) | absent |
| `.public()` | `[]`, no authentication |
| `.auth(policy)`, `.auth_with_env_mapping(..)` | one requirement object per method the policy accepts (its explicit methods, then its environment fallback), any one of which satisfies the operation |

A replacement's schemes are registered in `components.securitySchemes`
after the API's own. A scheme identical to one already registered reuses its
name; a different scheme takes its base name (`bearerAuth`, `apiKeyAuth`,
`basicAuth`, `oauth2Auth`) when free, else `bearerAuth2`, `bearerAuth3`, and
so on. A name is never reassigned to a different scheme, and endpoints are
visited in definition order, so the names are deterministic.

OpenAPI 3.0 has no OAuth2 device-code flow. A device-code grant gets no
scheme and no requirement, and the generator reports an `oauth2-grant`
warning (see [Generation Validation](../generation/validation.md#what-each-output-checks)).
An endpoint replacement whose only method is a device-code grant has no
`security` of its own.

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

- **Components** — Union keyed by component name, in member order. Members
  of one module group carry the same `schemas`, so they collapse into one; a
  name that two members define differently fails the OpenAPI output with
  `invalid-schema-reference`, naming both APIs
- **Paths** — Union of all endpoints from all APIs. An operation that two
  members declare identically apart from authentication appears once and
  accepts what either side accepts: each side's own `security`, or else its
  document's requirements, are unioned, and a side that needs no
  authentication (a public `security: []`, or an API without auth)
  contributes the empty requirement `{}`, so its public alternative is kept.
  Two different operations on one method and path fail the OpenAPI output.
- **Path parameters** — Declared once per name and location on the path
  item. A later member that declares one differently gets an
  operation-level override on its own operations; the earlier member's
  declaration stays common.
- **operationId** — Unique across the document. The first member in module
  group order keeps an id; a later clash is prefixed with its API name
  (`OllamaOpenAIListModels`), with a numeric suffix if that is taken too.
- **Security schemes** — Union keyed by scheme name; a same-named scheme with
  different content is renamed `<scheme>_<api_snake>` for the later API, then
  `<scheme>_<api_snake>2`, `3`, ... while that name holds a different scheme.
  A rename never overwrites an existing scheme, and that API's operation and
  document requirements reference the new name.
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
