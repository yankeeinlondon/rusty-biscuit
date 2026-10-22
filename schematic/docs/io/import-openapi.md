---
kind: canonical-documentation
---

# Importing OpenAPI into Schematic

Import OpenAPI 3.0.x JSON or YAML into typed clients or a reusable definitions
module. The vendored OpenAI document uses this same import path; never edit its
emitted models or definition module manually.

```sh
schematic-gen import --input api.yaml --output generated/src
schematic-gen import --input api.yaml --api-name Items \
    --module-path items --definitions-out definitions/src/items
```

The library entry point is feature-gated by `schematic-define/openapi`:

```rust
use schematic_define::openapi::{OpenApiImport, OpenApiSource};

let result = OpenApiImport::new(OpenApiSource::path("api.yaml"))
    .api_name("Items")
    .module_path("items")
    .build()?;
let api = result.api;
let models = result.models;
```

Import reads the OpenAPI AST, resolves local references, extracts a `ModelCatalog`,
and lowers operations into definitions. Inspect the result's diagnostics for lossy
mappings. The generator emits models before clients. Definitions-module output
uses typed registration and fallible builders.

The imported `api` is ready to validate and export. It carries one schema
component per catalog model, in source order: the model's final name, its Rust
path, and the source schema. Endpoint bodies and responses name those
components, so `api.validate_references()` passes and
`schematic_define::openapi::export` accepts the API as it is.

```rust
let pet = result.api.component("Pet").unwrap();
assert_eq!(pet.rust_path.as_deref(), Some("items::Pet")); // `Pet` without module_path
```

- **Rust paths** are `{module_path}::{Name}`, or the bare name when no module
  path is set; the models are generated into that module.
- **Renamed models** keep their references consistent. A schema whose name an
  endpoint's request struct claims (`CreatePetRequest` for the `CreatePet`
  endpoint) becomes `CreatePetRequest2`; the component, endpoint references,
  and `$ref`s inside other components all use the new name.
- **A dangling reference fails the import.** If an operation's JSON request or
  response names a schema the document does not define, `build()` returns
  `OpenApiError::Definition(DefinitionError::UnresolvedReference { api,
  endpoint, name })` instead of an API that cannot be generated or exported.
- **A document without components** imports with no schema components; inline
  bodies are free-form JSON (`Primitive(Json)`), which needs none.

## Mapping rules

| OpenAPI input | Definition or generated model |
| --- | --- |
| info, servers, externalDocs | API name, description, base URL, docs URL; import options may override metadata |
| operationId or operation fallback | PascalCase endpoint id, de-conflicted as needed |
| path parameter | Path template placeholder |
| query, header, cookie | Parameter in the declared transport location, with its `required`, `style`, and `explode`; an absent `explode` takes OpenAPI's default, `true` for `form` style (query and cookie) and `false` otherwise |
| JSON body | Typed model or free-form JSON |
| multipart / URL-encoded body | Form fields, with file constraints where supported |
| text / binary body | Raw body with content type |
| responses | One selected success; other responses remain diagnostic information (see [Several success shapes and declared errors](#several-success-shapes-and-declared-errors)) |
| component / inline schemas | Named models or synthesized model names |
| oneOf / anyOf | Untagged enum; unnamed members may remain free-form JSON |
| allOf | Merged schema/model according to import support |
| nullable | Optional Rust representation |
| bearer, basic, header/query/cookie API key | Existing auth strategy and environment mapping |

The importer does not implement OpenAPI 3.1. Full provider HTTP fidelity is
not established by a successful import; compile the generated crate and test
its wire behavior.

### Response checks

Before mapping, the importer rejects response members that the OpenAPI parser
would otherwise read loosely, naming the API, operation, and member:

- `responses`, `content`, a media entry's `schema`, and a `discriminator`
  that is `null` or not an object;
- a status key other than `default`, `100`-`599`, `1XX`-`5XX`, or an `x-`
  extension (`"600"`, `"abc"`), and two spellings of one status (`4XX`, `4xx`);
- a media key that is not `type/subtype` or a media range (`json`), and two
  spellings of one media type (`application/json`, `Application/JSON`);
- an empty or non-string `$ref`, an empty `oneOf` or one with a member that
  is not an object, and an empty or non-string `discriminator.propertyName`;
- exact duplicate keys anywhere, and trailing content after the document.

```text
Validation error at 'API Pets endpoint ListPets GET /pets responses':
Response key `abc` is not `default`, a status 100-599, or a class 1XX-5XX
```

### Several success shapes and declared errors

`openapi::classify_responses(api, endpoint, &operation.responses, &resolver)`
maps one operation's responses to success shapes and endpoint error entries.
`build()` does not use it yet, since an endpoint stores one response today;
switching the import to it is **planned**.

| Response | Imports as |
| --- | --- |
| `text/event-stream` | SSE stream of the schema's component |
| `application/x-ndjson`, `application/jsonl` | NDJSON stream of the schema's component |
| `application/json`, `application/*+json` | JSON shape |
| other `text/*` | text shape with that content type |
| any other media type | binary shape with that content type |
| `*/*` | the fallback shape |
| several media types under one status | one shape each |
| JSON `oneOf` with a `discriminator`, or of one array and one object | one tagged JSON shape per member |
| 204 or no `content` | empty shape |
| exact `400`-`599`, `4XX`, `5XX` | the endpoint's own error entries (never hoisted to the API) |

Media types are compared normalized (`Text/Event-Stream; charset=utf-8` is an
SSE stream). Identical shapes under several statuses import once; when the
statuses carry different shapes (apart from a body-less 204), each shape
declares the statuses it appears under. A stream's end condition cannot be
inferred from a media type, so it ends at connection close.

Each of these imports with a warning naming the API, operation, and what was
lost, rather than being guessed:

- a `default`, 1xx, or 3xx response (not imported);
- a media range other than `*/*` (not imported);
- a `oneOf` neither a discriminator nor array/object separates (one
  `serde_json::Value` JSON shape);
- a JSON body or stream without a named schema (`serde_json::Value`);
- an error status whose media types need different shapes (the first is
  kept).

An operation with no representable success is an import error.

## Importing into `schematic-definitions`

`schematic-gen import --definitions-out <dir>` writes a definitions-crate
module instead of a standalone client: `<dir>/types.rs` holds the models and
`<dir>/mod.rs` holds `define_{module}_api()`, which rebuilds the imported
`RestApi` as a value, plus the fallible `module_group()` it unwraps. The
imported API then joins the catalog and flows through `schematic-gen generate`
like a hand-written one.

```bash
schematic-gen import --input specs/openai/openapi.yaml --api-name OpenAI \
    --module-path openai --definitions-out definitions/src/openai
```

### Overlays: authored additions the import keeps

An OpenAPI document cannot express everything a definition carries;
pagination is the main example. Writing such additions into the imported
`mod.rs` does not work, because the next import overwrites it. Put them in a
hand-written overlay function instead and name it with `--overlay`:

```bash
schematic-gen import ... --definitions-out definitions/src/openai \
    --overlay crate::openai_pagination::apply
```

The overlay is a `fn(RestApi) -> RestApi` anywhere the module can reach by
that path. The importer never reads or writes it; it only emits the call, so
the factory every client and exporter uses returns the augmented definition:

```rust
fn imported_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = Vec::from([/* typed endpoint builders, each ending in build()? */]);
    let api = RestApi::builder(ctx, "OpenAI", "https://api.openai.com/v1")
        .describes("OpenAI API")
        .bearer_env(Vec::<String>::new())
        .endpoints(endpoints)
        .build()?;
    Ok(crate::openai_pagination::apply(api))
}
```

Generated `module_group()` registers models in source order and returns
`Result<Vec<RestApi>, DefinitionError>`. The API it builds carries every
registered model in `RestApi::schemas`, and its endpoints name those components.
The infallible catalog factory unwraps this checked assembly. Nominal models
implement `Registration` beside their declarations; aliases reuse their
underlying type, and containers, primitives, and free-form JSON remain inline
(free-form JSON is the reference `Primitive(Json)`).

Assembly orders components by registration, with each model's nested
definitions flattened directly after it, so its order can differ from the
library import's source order (Petstore: `Pet`, `PetStatus`,
`CreatePetRequest2` against `Pet`, `CreatePetRequest2`, `PetStatus`). Its
schemas are the generated models' JSON Schema rather than the source schemas.
Names and endpoint references are the same.

A standalone import (`--output`) keeps the library import's components but
spells each Rust path as the bare model name, because the standalone API module
glob-imports the generated `types` module.


```mermaid
flowchart LR
    spec[OpenAPI document] -->|schematic-gen import| modrs["openai/mod.rs (regenerated)"]
    overlay["openai_pagination.rs (hand-written)"] -.->|called by| modrs
    modrs --> factory["define_openai_api()"]
```

Rules:

- The value must be a plain Rust path: `crate::openai_pagination::apply`,
  `super::apply`, and `apply` are accepted; an empty value, generic arguments
  (`crate::x::apply::<T>`), or anything that is not a path (`apply()`) is
  rejected before anything is written.
- `--overlay` requires `--definitions-out`; a standalone client has no
  definitions factory to call it from, so the combination is rejected.
- Without `--overlay` the fallible assembly returns the built API directly.
- The module docs' regeneration hint includes the `--overlay` argument, so the
  emitted file records how to reproduce it. `just import-openai` passes
  `--overlay crate::openai_pagination::apply`.

### Query expressions in imported paths

The shared importer normalizes query-bearing paths before source emission. A
path such as `/items?beta=true` becomes `/items` plus an optional Boolean `beta`
query parameter. Callers select it with `.beta(true)`; omitting it preserves the
ordinary URL. A supported placeholder such as `/files?ref={git_ref}` becomes a
caller query input with wire name `ref` and Rust name `git_ref`.

If an operation already exists on the same method and base path, normalization
keeps the ordinary operation ID and order and uses the suffixed operation's
request and response only after both flattened schemas pass the compatibility
gate. Malformed expressions, duplicate query keys, conflicting declarations,
unsupported placeholders, and imported GET request bodies produce contextual
errors. The seven OpenAI beta operations are exposed through their ordinary
twins; the vendored source remains unchanged.

The changed input boundary rejects explicit null or malformed paths and
parameter declarations. Omitted parameters mean no declarations; an empty
array explicitly declares none. JSON and YAML both reject duplicate authored
keys and trailing invalid content. Metadata-only foundation imports can be empty. The normal `schematic-gen import`
invocation rejects an API with no usable endpoints before writing output.
